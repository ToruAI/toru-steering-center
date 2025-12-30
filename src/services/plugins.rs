use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::process::Child;
use tracing::{debug, error, info, warn};

use toru_plugin_api::{Message, PluginMetadata};

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
    restart_counts: HashMap<String, u32>,
    plugins_dir: PathBuf,
    metadata_dir: PathBuf,
    sockets_dir: PathBuf,
    max_restarts: u32,
}

impl PluginSupervisor {
    /// Create a new PluginSupervisor
    ///
    /// # Arguments
    /// * `plugins_dir` - Directory containing plugin .binary files
    /// * `max_restarts` - Maximum restart attempts before disabling a plugin
    pub fn new<P: AsRef<Path>>(
        plugins_dir: P,
        max_restarts: u32,
    ) -> Result<Self> {
        let plugins_dir = plugins_dir.as_ref().to_path_buf();
        let metadata_dir = plugins_dir.join(".metadata");
        let sockets_dir = PathBuf::from("/tmp/toru-plugins");

        // Create directories if they don't exist
        fs::create_dir_all(&plugins_dir).context("Failed to create plugins directory")?;
        fs::create_dir_all(&metadata_dir).context("Failed to create metadata directory")?;
        fs::create_dir_all(&sockets_dir).context("Failed to create sockets directory")?;

        Ok(Self {
            plugins: HashMap::new(),
            restart_counts: HashMap::new(),
            plugins_dir,
            metadata_dir,
            sockets_dir,
            max_restarts,
        })
    }

    /// Scan the plugins directory for .binary files and load metadata
    ///
    /// # Returns
    /// HashMap mapping plugin_id to (binary_path, metadata)
    pub async fn scan_plugins_directory(&self) -> Result<HashMap<String, (PathBuf, PluginMetadata)>> {
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

        let metadata: PluginMetadata = serde_json::from_str(&stdout)
            .context("Failed to parse plugin metadata JSON")?;

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

        let child = tokio::process::Command::new(binary_path)
            .env("TORU_PLUGIN_SOCKET", &socket_path_str)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn plugin process")?;

        let pid = child.id();

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

        Ok(())
    }

    /// Kill a plugin process gracefully (with shutdown message)
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier to kill
    pub async fn kill_plugin(&mut self, plugin_id: &str) -> Result<()> {
        let process = self.plugins.get_mut(plugin_id)
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

        Ok(())
    }

    /// Check if a plugin is healthy (socket exists and process is running)
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier to check
    ///
    /// # Returns
    /// true if healthy, false otherwise
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
            debug!("Plugin {} socket not found: {:?}", plugin_id, process.socket_path);
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

    /// Increment restart counter for a plugin
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    ///
    /// # Returns
    /// Current restart count
    pub fn increment_restart_count(&mut self, plugin_id: &str) -> u32 {
        let count = self.restart_counts.entry(plugin_id.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    /// Get restart count for a plugin
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
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
    pub fn should_disable_plugin(&self, plugin_id: &str) -> bool {
        self.get_restart_count(plugin_id) >= self.max_restarts
    }

    /// Reset restart counter for a plugin (e.g., after successful startup)
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
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

        fs::write(
            &config_path,
            serde_json::to_string_pretty(&config)? + "\n",
        ).context("Failed to write plugin config")?;

        debug!("Plugin {} enabled state set to: {}", plugin_id, enabled);
        Ok(())
    }

    /// Enable a plugin (spawn process and set enabled flag)
    pub async fn enable_plugin(&mut self, plugin_id: &str) -> Result<()> {
        self.set_plugin_enabled(plugin_id, true).await?;

        if let Some(process) = self.plugins.get_mut(plugin_id) {
            process.enabled = true;
        }

        info!("Plugin {} enabled", plugin_id);
        Ok(())
    }

    /// Disable a plugin (kill process and set disabled flag)
    pub async fn disable_plugin(&mut self, plugin_id: &str) -> Result<()> {
        self.set_plugin_enabled(plugin_id, false).await?;
        self.kill_plugin(plugin_id).await?;

        info!("Plugin {} disabled", plugin_id);
        Ok(())
    }

    /// Initialize the plugin supervisor by loading all plugins and spawning enabled ones
    ///
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

        info!("Initialized {} plugins (spawned {} enabled plugins)", total_plugins, spawned_count);
        Ok(spawned_count)
    }

    /// Send lifecycle init message to a plugin via Unix socket
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    async fn send_init_message(&self, plugin_id: &str) -> Result<()> {
        use toru_plugin_api::LifecycleInitPayload;

        let process = self.get_plugin_status(plugin_id)
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
        let mut stream = UnixStream::connect(&process.socket_path).await
            .context("Failed to connect to plugin socket")?;

        // Create init message with instance_id (will be added in Phase 3)
        let init_payload = LifecycleInitPayload {
            instance_id: "placeholder".to_string(), // Will be replaced with actual instance_id in Phase 3
            plugin_socket: process.socket_path.clone(),
            log_path: format!("/var/log/toru/plugins/{}.log", plugin_id),
        };

        let message = Message::new_lifecycle("init", Some(init_payload));

        // Serialize and send message
        let json = serde_json::to_string(&message)
            .context("Failed to serialize init message")?;

        stream.write_all(json.as_bytes()).await
            .context("Failed to send init message")?;

        debug!("Sent init message to plugin {}", plugin_id);
        Ok(())
    }

    /// Send lifecycle shutdown message to a plugin via Unix socket
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier
    async fn send_shutdown_message(&self, plugin_id: &str) -> Result<()> {
        let process = self.get_plugin_status(plugin_id)
            .context("Plugin not found")?;

        let socket_path = std::path::Path::new(&process.socket_path);

        if !socket_path.exists() {
            debug!("Plugin {} socket not found, skipping shutdown message", plugin_id);
            return Ok(());
        }

        // Connect to plugin socket
        let mut stream = UnixStream::connect(&process.socket_path).await
            .context("Failed to connect to plugin socket")?;

        // Create shutdown message
        let message = Message::new_lifecycle("shutdown", None);

        // Serialize and send message
        let json = serde_json::to_string(&message)
            .context("Failed to serialize shutdown message")?;

        stream.write_all(json.as_bytes()).await
            .context("Failed to send shutdown message")?;

        debug!("Sent shutdown message to plugin {}", plugin_id);
        Ok(())
    }

    /// Restart a crashed plugin with exponential backoff
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin identifier to restart
    /// * `binary_path` - Path to plugin binary
    /// * `metadata` - Plugin metadata
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
                plugin_id,
                self.max_restarts
            );

            // Log crash event
            // Note: Database access will be added when supervisor is integrated with AppState
            warn!("Plugin {} disabled after {} crashes", plugin_id, restart_count);

            self.disable_plugin(plugin_id).await?;

            return Err(anyhow::anyhow!(
                "Plugin disabled after {} consecutive failures",
                restart_count
            ));
        }

        // Calculate exponential backoff delay (1s, 2s, 4s, 8s, 16s)
        let backoff_exponent = (restart_count as u32).min(4);
        let delay_ms = 2u64.pow(backoff_exponent) * 1000; // 1000ms, 2000ms, 4000ms, 8000ms, 16000ms

        info!(
            "Restarting plugin {} (attempt #{}, waiting {}ms)",
            plugin_id,
            restart_count,
            delay_ms
        );

        // Log crash event
        // Note: Database access will be added when supervisor is integrated with AppState
        warn!("Plugin {} crashed, restarting in {}ms (attempt #{})", plugin_id, delay_ms, restart_count);

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

    #[tokio::test]
    async fn test_supervisor_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let supervisor = PluginSupervisor::new(temp_dir.path(), 10).unwrap();

        assert_eq!(supervisor.max_restarts, 10);
        assert!(supervisor.plugins_dir.exists());
    }

    #[test]
    fn test_restart_counter() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut supervisor = PluginSupervisor::new(temp_dir.path(), 10).unwrap();

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
        let supervisor = PluginSupervisor::new(temp_dir.path(), 3).unwrap();

        assert!(!supervisor.should_disable_plugin("test"));

        // Simulate 3 restarts
        supervisor.increment_restart_count("test");
        supervisor.increment_restart_count("test");
        supervisor.increment_restart_count("test");

        assert!(supervisor.should_disable_plugin("test"));
    }
}
