use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::process::Child;
use tracing::{debug, error, info, warn};

use toru_plugin_api::{HttpMessageResponse, HttpRequest, Message, PluginMetadata};

use super::logging::{LogLevel, PluginLogger, SupervisorLogger};
use crate::db::DbPool;

/// Represents a running plugin process
#[derive(Debug)]
pub struct PluginProcess {
    pub id: String,
    pub process: Option<Child>,
    pub socket_path: String,
    pub enabled: bool,
    pub metadata: Option<PluginMetadata>,
    pub pid: Option<u32>,
}

/// Manages plugin lifecycle, including spawning, monitoring, and restarting plugins
#[derive(Debug)]
pub struct PluginSupervisor {
    plugins: HashMap<String, PluginProcess>,
    // Used for tracking crash recovery and exponential backoff
    restart_counts: HashMap<String, u32>,
    plugins_dir: PathBuf,
    metadata_dir: PathBuf,
    sockets_dir: PathBuf,
    // Used to determine when to disable plugins after repeated crashes
    max_restarts: u32,
    instance_id: String,
    plugin_logger: Arc<PluginLogger>,
    supervisor_logger: Arc<SupervisorLogger>,
    db_pool: DbPool,
}

impl PluginSupervisor {
    /// Create a new PluginSupervisor
    ///
    /// # Arguments
    /// * `plugins_dir` - Directory containing plugin .binary files
    /// * `max_restarts` - Maximum restart attempts before disabling a plugin
    /// * `instance_id` - Unique instance ID to pass to plugins
    /// * `log_dir` - Directory for plugin logs (defaults to ./logs if not provided)
    /// * `db_pool` - Database pool for writing plugin events
    pub fn new<P: AsRef<Path>, L: AsRef<Path>>(
        plugins_dir: P,
        max_restarts: u32,
        instance_id: String,
        log_dir: L,
        db_pool: DbPool,
    ) -> Result<Self> {
        let plugins_dir = plugins_dir.as_ref().to_path_buf();
        let metadata_dir = plugins_dir.join(".metadata");
        let sockets_dir = PathBuf::from("/tmp/toru-plugins");
        let log_dir = log_dir.as_ref().to_path_buf();

        // Create directories if they don't exist
        fs::create_dir_all(&plugins_dir).context("Failed to create plugins directory")?;
        fs::create_dir_all(&metadata_dir).context("Failed to create metadata directory")?;
        fs::create_dir_all(&sockets_dir).context("Failed to create sockets directory")?;

        // Initialize loggers
        let plugin_logger = Arc::new(PluginLogger::new(super::logging::LogConfig {
            log_dir: log_dir.clone(),
            ..Default::default()
        })?);

        let supervisor_logger = Arc::new(SupervisorLogger::new(&log_dir)?);

        Ok(Self {
            plugins: HashMap::new(),
            restart_counts: HashMap::new(),
            plugins_dir,
            metadata_dir,
            sockets_dir,
            max_restarts,
            instance_id,
            plugin_logger,
            supervisor_logger,
            db_pool,
        })
    }

    /// Get a reference to the plugin logger
    pub fn plugin_logger(&self) -> Arc<PluginLogger> {
        Arc::clone(&self.plugin_logger)
    }

    /// Scan the plugins directory for .binary files and load metadata
    ///
    /// # Returns
    /// HashMap mapping plugin_id to (binary_path, metadata)
    pub async fn scan_plugins_directory(
        &self,
    ) -> Result<HashMap<String, (PathBuf, PluginMetadata)>> {
        let mut discovered = HashMap::new();

        let entries = match fs::read_dir(&self.plugins_dir) {
            Ok(entries) => entries,
            Err(e) => {
                error!("Failed to read plugins directory: {}", e);
                return Ok(discovered);
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read directory entry: {}", e);
                    continue;
                }
            };

            let path = entry.path();

            // Skip directories and non-.binary files
            if path.is_dir() {
                continue;
            }

            if path.extension().and_then(|ext| ext.to_str()) != Some("binary") {
                continue;
            }

            // Skip metadata directory
            if path.starts_with(&self.metadata_dir) {
                continue;
            }

            // Security: Resolve symlinks and verify path is still within plugins_dir
            let canonical_path = match path.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    warn!("Failed to canonicalize plugin path {:?}: {}", path, e);
                    continue;
                }
            };

            let canonical_plugins_dir = match self.plugins_dir.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to canonicalize plugins directory: {}", e);
                    return Ok(discovered);
                }
            };

            if !canonical_path.starts_with(&canonical_plugins_dir) {
                warn!(
                    "Plugin {:?} resolves outside plugins directory (symlink attack?), skipping",
                    path
                );
                continue;
            }

            // Read plugin metadata
            match self.read_plugin_metadata(&path).await {
                Ok(metadata) => {
                    debug!("Discovered plugin: {} v{}", metadata.name, metadata.version);
                    discovered.insert(metadata.id.clone(), (path, metadata));
                }
                Err(e) => {
                    error!("Failed to read metadata for {:?}: {}", path, e);
                    // Continue loading other plugins
                }
            }
        }

        info!("Discovered {} plugins", discovered.len());
        Ok(discovered)
    }

    /// Read plugin metadata by running the binary with --metadata flag
    ///
    /// # Arguments
    /// * `binary_path` - Path to the plugin binary
    ///
    /// # Returns
    /// PluginMetadata parsed from JSON output
    async fn read_plugin_metadata(&self, binary_path: &Path) -> Result<PluginMetadata> {
        use tokio::process::Command;

        let output = Command::new(binary_path)
            .arg("--metadata")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute plugin binary")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Plugin --metadata command failed: {}",
                stderr
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .context("Plugin metadata output is not valid UTF-8")?;

        let metadata: PluginMetadata =
            serde_json::from_str(&stdout).context("Failed to parse plugin metadata JSON")?;

        // Security: Validate metadata fields to prevent injection attacks
        if !metadata
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(anyhow::anyhow!("Invalid plugin ID format"));
        }
        if !metadata.route.starts_with('/') || metadata.route.contains("..") {
            return Err(anyhow::anyhow!("Invalid plugin route"));
        }
        if metadata.name.len() > 100 || metadata.author.as_ref().is_some_and(|a| a.len() > 100) {
            return Err(anyhow::anyhow!("Metadata field too long"));
        }

        Ok(metadata)
    }

    /// Spawn a plugin process
    ///
    /// # Arguments
    /// * `plugin_id` - Unique identifier for the plugin
    /// * `binary_path` - Path to the plugin binary
    /// * `metadata` - Plugin metadata
    ///
    /// # Returns
    /// Ok(()) if successful, Err otherwise
    pub async fn spawn_plugin(
        &mut self,
        plugin_id: &str,
        binary_path: &Path,
        metadata: PluginMetadata,
    ) -> Result<()> {
        let socket_path = self.sockets_dir.join(format!("{}.sock", plugin_id));
        let socket_path_str = socket_path.to_string_lossy().to_string();

        // Clean up existing socket if present
        if socket_path.exists() {
            fs::remove_file(&socket_path).ok();
        }

        let mut child = tokio::process::Command::new(binary_path)
            .env("TORU_PLUGIN_SOCKET", &socket_path_str)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn plugin process")?;

        let pid = child.id();

        // Capture stderr to plugin log file
        if let Some(mut stderr) = child.stderr.take() {
            let plugin_logger = Arc::clone(&self.plugin_logger);
            let plugin_id_clone = plugin_id.to_string();

            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut buffer = [0u8; 4096];

                loop {
                    match stderr.read(&mut buffer).await {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            let output = String::from_utf8_lossy(&buffer[..n]).to_string();
                            // Parse structured JSON logs or write as plain text
                            if let Ok(log_entry) =
                                serde_json::from_str::<crate::services::logging::LogEntry>(&output)
                            {
                                let _ = plugin_logger.log_plugin(log_entry).await;
                            } else {
                                // Write plain text as Info log
                                let log_entry = crate::services::logging::LogEntry::new(
                                    crate::services::logging::LogLevel::Info,
                                    output.trim(),
                                )
                                .with_plugin(&plugin_id_clone);
                                let _ = plugin_logger.log_plugin(log_entry).await;
                            }
                        }
                        Err(_) => break, // Error
                    }
                }
            });
        }

        let process = PluginProcess {
            id: plugin_id.to_string(),
            process: Some(child),
            socket_path: socket_path_str,
            enabled: true,
            metadata: Some(metadata),
            pid,
        };

        self.plugins.insert(plugin_id.to_string(), process);
        info!("Spawned plugin: {} (PID: {:?})", plugin_id, pid);

        // Notify plugin event via notification hooks
        self.notify_plugin_event(
            plugin_id,
            "started",
            LogLevel::Info,
            Some(
                &serde_json::json!({
                    "pid": pid,
                })
                .to_string(),
            ),
        )
        .await;

        Ok(())
    }

    /// Kill a plugin process gracefully (with shutdown message)
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier to kill
    pub async fn kill_plugin(&mut self, plugin_id: &str) -> Result<()> {
        let process = self
            .plugins
            .get_mut(plugin_id)
            .context("Plugin not found")?;

        if let Some(mut child) = process.process.take() {
            // Try graceful shutdown first
            match child.start_kill() {
                Ok(_) => {
                    info!("Sent kill signal to plugin: {}", plugin_id);
                }
                Err(e) => {
                    warn!("Failed to kill plugin {}: {}", plugin_id, e);
                }
            }

            // Wait for process to exit (with timeout)
            tokio::select! {
                _ = child.wait() => {
                    debug!("Plugin {} exited gracefully", plugin_id);
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                    warn!("Plugin {} did not exit within 5s, forcing", plugin_id);
                }
            }
        }

        // Remove socket if it exists
        if let Ok(socket_path) = std::path::PathBuf::from(&process.socket_path).canonicalize() {
            if socket_path.exists() {
                fs::remove_file(&socket_path).ok();
            }
        }

        process.enabled = false;
        info!("Plugin {} killed and disabled", plugin_id);

        // Notify plugin event via notification hooks
        self.notify_plugin_event(plugin_id, "killed", LogLevel::Info, None)
            .await;

        Ok(())
    }

    /// Check if a plugin is healthy (socket exists and process is running)
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier to check
    ///
    /// # Returns
    /// true if healthy, false otherwise
    // TODO: Integrate in health check endpoints and monitoring
    #[allow(dead_code)]
    pub fn check_plugin_health(&self, plugin_id: &str) -> bool {
        let process = match self.plugins.get(plugin_id) {
            Some(p) => p,
            None => return false,
        };

        if !process.enabled {
            return false;
        }

        // Check if socket file exists
        let socket_path = std::path::Path::new(&process.socket_path);
        if !socket_path.exists() {
            debug!(
                "Plugin {} socket not found: {:?}",
                plugin_id, process.socket_path
            );
            return false;
        }

        // Check if process is still running using PID (Unix only)
        #[cfg(unix)]
        {
            if let Some(pid) = process.pid {
                // Use libc to send signal 0 (no-op) to check if process exists
                unsafe {
                    let result = libc::kill(pid as i32, 0);
                    result == 0 // 0 = success, -1 = error (process not found or permission denied)
                }
            } else {
                false
            }
        }

        #[cfg(not(unix))]
        {
            // Fallback for non-Unix: assume healthy if socket exists
            true
        }
    }

    /// Get plugin status information
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    ///
    /// # Returns
    /// Plugin process info if found
    pub fn get_plugin_status(&self, plugin_id: &str) -> Option<&PluginProcess> {
        self.plugins.get(plugin_id)
    }

    /// Get all managed plugins
    pub fn get_all_plugins(&self) -> &HashMap<String, PluginProcess> {
        &self.plugins
    }

    /// Get the plugins directory path
    pub fn get_plugins_dir(&self) -> PathBuf {
        self.plugins_dir.clone()
    }

    /// Notify plugin event through all configured notification hooks
    ///
    /// This is the unified entry point for plugin event notifications.
    /// Currently writes to:
    /// 1. Log files (via supervisor_logger)
    /// 2. Database (plugin_events table)
    ///
    /// Future extensibility: Email, webhooks, Slack, etc.
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    /// * `event_type` - Type of event (e.g., "started", "crash", "disabled")
    /// * `log_level` - Log level for file logging
    /// * `details` - Optional JSON string with event details
    pub async fn notify_plugin_event(
        &self,
        plugin_id: &str,
        event_type: &str,
        log_level: LogLevel,
        details: Option<&str>,
    ) {
        // Hook 1: Log to file
        let _ = self
            .supervisor_logger
            .log_plugin_event(log_level, plugin_id, event_type, details)
            .await;

        // Hook 2: Log to database
        let _ = crate::db::plugin_event_log(&self.db_pool, plugin_id, event_type, details).await;

        // Future: Hook 3 - Email notifications
        // Future: Hook 4 - Webhook calls
        // Future: Hook 5 - Plugin-specific callbacks
    }

    /// Increment restart counter for a plugin
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    ///
    /// # Returns
    /// Current restart count
    // Used in restart_plugin_with_backoff
    #[allow(dead_code)]
    pub fn increment_restart_count(&mut self, plugin_id: &str) -> u32 {
        let count = self
            .restart_counts
            .entry(plugin_id.to_string())
            .or_insert(0);
        *count += 1;
        *count
    }

    /// Get restart count for a plugin
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    // Used in tests and should_disable_plugin
    #[allow(dead_code)]
    pub fn get_restart_count(&self, plugin_id: &str) -> u32 {
        *self.restart_counts.get(plugin_id).unwrap_or(&0)
    }

    /// Check if plugin should be disabled due to too many restarts
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    ///
    /// # Returns
    /// true if should be disabled
    // Used in restart_plugin_with_backoff
    #[allow(dead_code)]
    pub fn should_disable_plugin(&self, plugin_id: &str) -> bool {
        self.get_restart_count(plugin_id) >= self.max_restarts
    }

    /// Reset restart counter for a plugin (e.g., after successful startup)
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    // Used in tests and will be needed for crash recovery reset
    #[allow(dead_code)]
    pub fn reset_restart_count(&mut self, plugin_id: &str) {
        self.restart_counts.remove(plugin_id);
    }

    /// Get enabled state for a plugin from metadata storage
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    ///
    /// # Returns
    /// true if plugin is enabled, false if disabled
    pub fn is_plugin_enabled(&self, plugin_id: &str) -> bool {
        let config_path = self.metadata_dir.join("config.json");

        if !config_path.exists() {
            // Default to enabled if no config exists
            return true;
        }

        match fs::read_to_string(&config_path) {
            Ok(content) => {
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(plugins) = config.get("plugins") {
                        if let Some(enabled) = plugins.get(plugin_id).and_then(|v| v.as_bool()) {
                            return enabled;
                        }
                    }
                }
                // Default to enabled if we can't determine from config
                true
            }
            Err(_) => true,
        }
    }

    /// Set enabled state for a plugin in metadata storage
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    /// * `enabled` - Whether plugin should be enabled
    pub async fn set_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
        let config_path = self.metadata_dir.join("config.json");

        let mut config: serde_json::Value = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        if !config.is_object() {
            config = serde_json::json!({});
        }

        if !config["plugins"].is_object() {
            config["plugins"] = serde_json::json!({});
        }

        config["plugins"][plugin_id] = serde_json::json!(enabled);

        fs::write(&config_path, serde_json::to_string_pretty(&config)? + "\n")
            .context("Failed to write plugin config")?;

        debug!("Plugin {} enabled state set to: {}", plugin_id, enabled);
        Ok(())
    }

    /// Enable a plugin (spawn process and set enabled flag)
    pub async fn enable_plugin(&mut self, plugin_id: &str) -> Result<()> {
        self.set_plugin_enabled(plugin_id, true).await?;

        if let Some(process) = self.plugins.get_mut(plugin_id) {
            // If plugin is disabled or not running, spawn it
            if !process.enabled || process.process.is_none() {
                // Get binary path from plugins directory
                let binary_path = self.plugins_dir.join(format!("{}.binary", plugin_id));
                if let Some(metadata) = process.metadata.clone() {
                    // Spawn the plugin (process reference is dropped automatically at end of scope)
                    let _ = process; // Explicitly indicate we're done with the mutable borrow
                    self.spawn_plugin(plugin_id, &binary_path, metadata).await?;
                } else {
                    process.enabled = true;
                }
            } else {
                process.enabled = true;
            }
        } else {
            // Plugin not in memory, need to discover and spawn it
            let discovered = self.scan_plugins_directory().await?;
            if let Some((binary_path, metadata)) = discovered.get(plugin_id) {
                self.spawn_plugin(plugin_id, binary_path, metadata.clone())
                    .await?;
            }
        }

        // Wait for socket to be ready after spawning (similar to send_init_message retry logic)
        let socket_path = self.sockets_dir.join(format!("{}.sock", plugin_id));
        for _ in 0..20 {
            // 20 retries * 100ms = 2 seconds max
            if socket_path.exists() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        info!("Plugin {} enabled", plugin_id);

        // Notify plugin event via notification hooks
        self.notify_plugin_event(plugin_id, "enabled", LogLevel::Info, None)
            .await;

        Ok(())
    }

    /// Disable a plugin (kill process and set disabled flag)
    pub async fn disable_plugin(&mut self, plugin_id: &str) -> Result<()> {
        self.set_plugin_enabled(plugin_id, false).await?;
        self.kill_plugin(plugin_id).await?;

        info!("Plugin {} disabled", plugin_id);

        // Notify plugin event via notification hooks
        self.notify_plugin_event(plugin_id, "disabled", LogLevel::Info, None)
            .await;

        Ok(())
    }

    /// Disable a plugin (kill process and set disabled flag)
    /// This should be called on server startup.
    ///
    /// # Returns
    /// Number of plugins that were successfully spawned
    pub async fn initialize(&mut self) -> Result<usize> {
        let discovered = self.scan_plugins_directory().await?;
        let total_plugins = discovered.len();

        let mut spawned_count = 0;

        for (plugin_id, (binary_path, metadata)) in discovered {
            // Check if plugin is enabled
            if self.is_plugin_enabled(&plugin_id) {
                match self.spawn_plugin(&plugin_id, &binary_path, metadata).await {
                    Ok(_) => {
                        spawned_count += 1;
                        // Send init message to plugin
                        if let Err(e) = self.send_init_message(&plugin_id).await {
                            error!("Failed to send init message to {}: {}", plugin_id, e);
                            // Continue anyway - plugin may still work
                        }
                    }
                    Err(e) => {
                        error!("Failed to spawn plugin {}: {}", plugin_id, e);
                    }
                }
            } else {
                info!("Plugin {} is disabled, skipping", plugin_id);
            }
        }

        info!(
            "Initialized {} plugins (spawned {} enabled plugins)",
            total_plugins, spawned_count
        );
        Ok(spawned_count)
    }

    /// Send lifecycle init message to a plugin via Unix socket
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    async fn send_init_message(&self, plugin_id: &str) -> Result<()> {
        use toru_plugin_api::LifecycleInitPayload;

        let process = self
            .get_plugin_status(plugin_id)
            .context("Plugin not found")?;

        // Wait for socket to be available (with timeout)
        let socket_path = std::path::Path::new(&process.socket_path);
        let mut retries = 10;

        while !socket_path.exists() && retries > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            retries -= 1;
        }

        if !socket_path.exists() {
            return Err(anyhow::anyhow!("Plugin socket not available after waiting"));
        }

        // Connect to plugin socket
        let mut stream = UnixStream::connect(&process.socket_path)
            .await
            .context("Failed to connect to plugin socket")?;

        // Create init message with instance_id
        let init_payload = LifecycleInitPayload {
            instance_id: self.instance_id.clone(),
            plugin_socket: process.socket_path.clone(),
            log_path: format!("/var/log/toru/plugins/{}.log", plugin_id),
        };

        let message = Message::new_lifecycle("init", Some(init_payload));

        // Serialize and send message
        let json = serde_json::to_string(&message).context("Failed to serialize init message")?;

        stream
            .write_all(json.as_bytes())
            .await
            .context("Failed to send init message")?;

        debug!("Sent init message to plugin {}", plugin_id);
        Ok(())
    }

    /// Send lifecycle shutdown message to a plugin via Unix socket
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    // TODO: Integrate in graceful shutdown flow
    #[allow(dead_code)]
    async fn send_shutdown_message(&self, plugin_id: &str) -> Result<()> {
        let process = self
            .get_plugin_status(plugin_id)
            .context("Plugin not found")?;

        let socket_path = std::path::Path::new(&process.socket_path);

        if !socket_path.exists() {
            debug!(
                "Plugin {} socket not found, skipping shutdown message",
                plugin_id
            );
            return Ok(());
        }

        // Connect to plugin socket
        let mut stream = UnixStream::connect(&process.socket_path)
            .await
            .context("Failed to connect to plugin socket")?;

        // Create shutdown message
        let message = Message::new_lifecycle("shutdown", None);

        // Serialize and send message
        let json =
            serde_json::to_string(&message).context("Failed to serialize shutdown message")?;

        stream
            .write_all(json.as_bytes())
            .await
            .context("Failed to send shutdown message")?;

        debug!("Sent shutdown message to plugin {}", plugin_id);
        Ok(())
    }

    /// Forward an HTTP request to a plugin
    ///
    /// This method is used by the HTTP router to forward requests to plugins.
    /// The plugin route path is resolved to the plugin ID, and the request
    /// is forwarded via the plugin's Unix socket.
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    /// * `request` - HTTP request to forward
    ///
    /// # Returns
    /// The plugin's HTTP response
    pub async fn forward_http_request(
        &self,
        plugin_id: &str,
        request: &HttpRequest,
    ) -> Result<HttpMessageResponse> {
        let process = self
            .get_plugin_status(plugin_id)
            .context("Plugin not found")?;

        // Check if plugin is enabled and has a socket
        if !process.enabled {
            return Err(anyhow::anyhow!("Plugin {} is not enabled", plugin_id));
        }

        let socket_path = std::path::Path::new(&process.socket_path);
        if !socket_path.exists() {
            return Err(anyhow::anyhow!("Plugin {} socket not found", plugin_id));
        }

        // Connect to plugin socket
        let mut stream = UnixStream::connect(&process.socket_path)
            .await
            .context("Failed to connect to plugin socket")?;

        // Generate a unique request ID
        let request_id = uuid::Uuid::new_v4().to_string();

        // Create HTTP request message
        let message = Message::new_http(request_id.clone(), request.clone());

        // Use the protocol to send the message
        use toru_plugin_api::PluginProtocol;
        let mut protocol = PluginProtocol::new();
        protocol
            .write_message(&mut stream, &message)
            .await
            .context("Failed to send HTTP request to plugin")?;

        // Read the response with timeout to prevent hanging on unresponsive plugins
        let response_msg = tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            protocol.read_message(&mut stream),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Plugin response timeout after 30s"))?
        .context("Failed to read HTTP response from plugin")?;

        // Extract the HTTP response - parse JSON to get status/headers/body
        let response_value =
            serde_json::to_value(&response_msg).context("Failed to serialize response message")?;

        // Extract HTTP response fields from nested payload
        let http_response = toru_plugin_api::HttpMessageResponse {
            status: response_value
                .get("payload")
                .and_then(|p| {
                    // Check if payload has "http" field (nested response)
                    if p.get("http").is_some() {
                        p.get("http").and_then(|h| h.get("status"))
                    } else {
                        // Direct payload without nesting
                        p.get("status")
                    }
                })
                .and_then(|s| s.as_u64())
                .unwrap_or(500) as u16,
            headers: response_value
                .get("payload")
                .and_then(|p| {
                    if p.get("http").is_some() {
                        p.get("http").and_then(|h| h.get("headers"))
                    } else {
                        p.get("headers")
                    }
                })
                .and_then(|h| serde_json::from_value(h.clone()).ok())
                .unwrap_or_default(),
            body: response_value
                .get("payload")
                .and_then(|p| {
                    if p.get("http").is_some() {
                        p.get("http").and_then(|h| h.get("body"))
                    } else {
                        p.get("body")
                    }
                })
                .and_then(|b| b.as_str())
                .map(|s| s.to_string()),
        };

        Ok(http_response)
    }

    /// Get the plugin ID that owns a given route path
    ///
    /// Each plugin declares a route in its metadata (e.g., "/my-plugin").
    /// This method looks up which plugin owns a given route path.
    ///
    /// # Arguments
    /// * `route_path` - The route path (e.g., "/my-plugin")
    ///
    /// # Returns
    /// The plugin ID that owns the route, if found
    pub fn get_plugin_for_route(&self, route_path: &str) -> Option<String> {
        for (plugin_id, process) in self.plugins.iter() {
            if let Some(metadata) = &process.metadata {
                if metadata.route == route_path {
                    return Some(plugin_id.clone());
                }
            }
        }
        None
    }

    /// Restart a crashed plugin with exponential backoff
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier to restart
    /// * `binary_path` - Path to plugin binary
    /// * `metadata` - Plugin metadata
    // TODO: Integrate in crash monitoring and auto-recovery system
    #[allow(dead_code)]
    pub async fn restart_plugin_with_backoff(
        &mut self,
        plugin_id: &str,
        binary_path: &Path,
        metadata: PluginMetadata,
    ) -> Result<()> {
        let restart_count = self.increment_restart_count(plugin_id);

        // Check if we've reached max restarts
        if self.should_disable_plugin(plugin_id) {
            error!(
                "Plugin {} has reached max restarts ({}), disabling",
                plugin_id, self.max_restarts
            );

            // Notify plugin event via notification hooks
            self.notify_plugin_event(
                plugin_id,
                "disabled_after_max_restarts",
                LogLevel::Error,
                Some(
                    &serde_json::json!({
                        "reason": "max_restarts_exceeded",
                        "restart_count": restart_count,
                    })
                    .to_string(),
                ),
            )
            .await;

            warn!(
                "Plugin {} disabled after {} crashes",
                plugin_id, restart_count
            );

            self.disable_plugin(plugin_id).await?;

            return Err(anyhow::anyhow!(
                "Plugin disabled after {} consecutive failures",
                restart_count
            ));
        }

        // Calculate exponential backoff delay (1s, 2s, 4s, 8s, 16s)
        let backoff_exponent = restart_count.min(4);
        let delay_ms = 2u64.pow(backoff_exponent) * 1000; // 1000ms, 2000ms, 4000ms, 8000ms, 16000ms

        info!(
            "Restarting plugin {} (attempt #{}, waiting {}ms)",
            plugin_id, restart_count, delay_ms
        );

        // Notify plugin event via notification hooks
        self.notify_plugin_event(
            plugin_id,
            "restarting_with_backoff",
            LogLevel::Warn,
            Some(
                &serde_json::json!({
                    "reason": "plugin_crashed",
                    "restart_attempt": restart_count,
                    "backoff_delay_ms": delay_ms,
                })
                .to_string(),
            ),
        )
        .await;

        warn!(
            "Plugin {} crashed, restarting in {}ms (attempt #{})",
            plugin_id, delay_ms, restart_count
        );

        // Wait with exponential backoff
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

        // Spawn plugin
        self.spawn_plugin(plugin_id, binary_path, metadata).await?;

        // Send init message
        if let Err(e) = self.send_init_message(plugin_id).await {
            error!("Failed to send init message after restart: {}", e);
        }

        info!("Plugin {} restarted successfully", plugin_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[tokio::test]
    async fn test_supervisor_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_pool = db::init_db().unwrap();
        let supervisor = PluginSupervisor::new(
            temp_dir.path(),
            10,
            "test-instance-id".to_string(),
            temp_dir.path(),
            db_pool,
        )
        .unwrap();

        assert_eq!(supervisor.max_restarts, 10);
        assert_eq!(supervisor.instance_id, "test-instance-id");
        assert!(supervisor.plugins_dir.exists());
    }

    #[test]
    fn test_restart_counter() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_pool = db::init_db().unwrap();
        let mut supervisor = PluginSupervisor::new(
            temp_dir.path(),
            10,
            "test-instance-id".to_string(),
            temp_dir.path(),
            db_pool,
        )
        .unwrap();

        assert_eq!(supervisor.get_restart_count("test"), 0);
        assert_eq!(supervisor.increment_restart_count("test"), 1);
        assert_eq!(supervisor.get_restart_count("test"), 1);
        assert_eq!(supervisor.increment_restart_count("test"), 2);

        supervisor.reset_restart_count("test");
        assert_eq!(supervisor.get_restart_count("test"), 0);
    }

    #[test]
    fn test_should_disable() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_pool = db::init_db().unwrap();
        let mut supervisor = PluginSupervisor::new(
            temp_dir.path(),
            3,
            "test-instance-id".to_string(),
            temp_dir.path(),
            db_pool,
        )
        .unwrap();

        assert!(!supervisor.should_disable_plugin("test"));

        // Simulate 3 restarts
        supervisor.increment_restart_count("test");
        supervisor.increment_restart_count("test");
        supervisor.increment_restart_count("test");

        assert!(supervisor.should_disable_plugin("test"));
    }
}
