use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Log levels for plugin and supervisor logging
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Convert string to LogLevel
    pub fn parse_level(level: &str) -> Option<Self> {
        match level.to_lowercase().as_str() {
            "trace" => Some(LogLevel::Trace),
            "debug" => Some(LogLevel::Debug),
            "info" => Some(LogLevel::Info),
            "warn" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            _ => None,
        }
    }

    /// Get level for filtering (higher values = more severe)
    pub fn severity(&self) -> u8 {
        match self {
            LogLevel::Trace => 0,
            LogLevel::Debug => 1,
            LogLevel::Info => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
        }
    }
}

/// Structured log entry (JSON format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(level: LogLevel, message: &str) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            level: format!("{:?}", level),
            message: message.to_string(),
            plugin: None,
            error: None,
            pid: None,
        }
    }

    /// Set plugin ID for the log entry
    pub fn with_plugin(mut self, plugin_id: &str) -> Self {
        self.plugin = Some(plugin_id.to_string());
        self
    }

    /// Set error details for the log entry
    // TODO: Integrate in crash recovery logging
    #[allow(dead_code)]
    pub fn with_error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }

    /// Set PID for the log entry
    // TODO: Integrate in process monitoring
    #[allow(dead_code)]
    pub fn with_pid(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize log entry")
    }
}

/// Log configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Maximum size of a log file before rotation (in bytes)
    pub max_file_size: u64,
    /// Maximum number of rotated log files to keep
    pub max_rotated_files: usize,
    /// Base directory for logs
    pub log_dir: PathBuf,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10 MB
            max_rotated_files: 5,
            log_dir: PathBuf::from("/var/log/toru"),
        }
    }
}

/// Plugin logger for writing structured JSON logs
#[derive(Debug)]
pub struct PluginLogger {
    config: LogConfig,
    // Per-plugin log files: HashMap<plugin_id, log_file_path>
    // TODO: Integrate file handle caching for improved performance
    #[allow(dead_code)]
    log_files: Arc<Mutex<std::collections::HashMap<String, PathBuf>>>,
}

impl PluginLogger {
    /// Create a new plugin logger
    pub fn new(config: LogConfig) -> Result<Self> {
        // Create base log directory
        fs::create_dir_all(&config.log_dir).context("Failed to create log directory")?;

        // Create plugins subdirectory
        let plugins_log_dir = config.log_dir.join("plugins");
        fs::create_dir_all(&plugins_log_dir).context("Failed to create plugins log directory")?;

        Ok(Self {
            config,
            log_files: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }

    /// Initialize logger with default config
    // TODO: Use in alternative initialization paths
    #[allow(dead_code)]
    pub fn with_default_config() -> Result<Self> {
        Self::new(LogConfig::default())
    }

    /// Create logger from custom log directory
    // TODO: Use in alternative initialization paths
    #[allow(dead_code)]
    pub fn from_directory<P: AsRef<Path>>(log_dir: P) -> Result<Self> {
        let config = LogConfig {
            log_dir: log_dir.as_ref().to_path_buf(),
            ..Default::default()
        };
        Self::new(config)
    }

    /// Get log file path for a plugin
    pub fn get_plugin_log_path(&self, plugin_id: &str) -> PathBuf {
        self.config
            .log_dir
            .join("plugins")
            .join(format!("{}.log", plugin_id))
    }

    /// Write a log entry to a plugin's log file
    pub async fn log_plugin(&self, entry: LogEntry) -> Result<()> {
        let plugin_id = entry
            .plugin
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Log entry must have plugin_id"))?;

        let log_path = self.get_plugin_log_path(&plugin_id);

        // Check if rotation is needed
        self.check_and_rotate(&log_path).await?;

        // Open file in append mode
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .context("Failed to open log file")?;

        // Write JSON log entry
        let json = entry.to_json()?;
        writeln!(file, "{}", json).context("Failed to write log entry")?;

        Ok(())
    }

    /// Read logs for a plugin with optional filtering and pagination
    pub async fn read_plugin_logs(
        &self,
        plugin_id: &str,
        filter_level: Option<LogLevel>,
        page: usize,
        page_size: usize,
    ) -> Result<Vec<LogEntry>> {
        let log_path = self.get_plugin_log_path(plugin_id);

        if !log_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&log_path).context("Failed to read log file")?;

        // Parse all log entries
        let mut logs: Vec<LogEntry> = content
            .lines()
            .filter_map(|line| serde_json::from_str::<LogEntry>(line).ok())
            .collect();

        // Filter by log level if specified
        if let Some(level) = filter_level {
            let min_severity = level.severity();
            logs.retain(|entry| {
                if let Some(entry_level) = LogLevel::parse_level(&entry.level) {
                    entry_level.severity() >= min_severity
                } else {
                    false
                }
            });
        }

        // Reverse to show newest first
        logs.reverse();

        // Apply pagination
        let start = page * page_size;
        let end = start + page_size;
        if start < logs.len() {
            logs.truncate(end);
            Ok(logs[start..].to_vec())
        } else {
            Ok(Vec::new())
        }
    }

    /// Check if log file needs rotation and rotate if necessary
    async fn check_and_rotate(&self, log_path: &Path) -> Result<()> {
        if !log_path.exists() {
            return Ok(());
        }

        let metadata = fs::metadata(log_path)?;
        if metadata.len() >= self.config.max_file_size {
            self.rotate_log(log_path)?;
        }

        Ok(())
    }

    /// Rotate a log file (rename with timestamp, create new)
    fn rotate_log(&self, log_path: &Path) -> Result<()> {
        // Get current timestamp for rotation
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");

        // Construct rotated filename
        let stem = log_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("log");
        let ext = log_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("log");
        let parent = log_path.parent().unwrap_or_else(|| Path::new("."));

        // Rotate existing files (archive -> archive.1 -> archive.2 -> ...)
        let rotated_path = parent.join(format!("{}-{}.{}", stem, timestamp, ext));

        // Rename current log file to rotated name
        fs::rename(log_path, &rotated_path).context("Failed to rename log file for rotation")?;

        // Clean up old rotated files
        self.cleanup_old_rotated_logs(parent, stem, ext)?;

        Ok(())
    }

    /// Remove old rotated log files beyond max_rotated_files limit
    fn cleanup_old_rotated_logs(&self, dir: &Path, stem: &str, ext: &str) -> Result<()> {
        // Read directory and find all rotated log files
        let mut rotated_files: Vec<PathBuf> = fs::read_dir(dir)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with(stem) && name.ends_with(ext))
                    .unwrap_or(false)
            })
            .collect();

        // Sort by modification time (oldest first)
        rotated_files.sort_by(|a, b| {
            let a_time = a
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let b_time = b
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            a_time.cmp(&b_time)
        });

        // Remove files beyond max_rotated_files limit
        let keep_count = self.config.max_rotated_files;
        if rotated_files.len() > keep_count {
            for old_file in rotated_files.iter().take(rotated_files.len() - keep_count) {
                fs::remove_file(old_file).ok(); // Ignore errors
            }
        }

        Ok(())
    }
}

/// Supervisor logger for core plugin system logs
#[derive(Debug)]
pub struct SupervisorLogger {
    log_file: Arc<Mutex<File>>,
}

impl SupervisorLogger {
    /// Create a new supervisor logger
    pub fn new(log_dir: &Path) -> Result<Self> {
        fs::create_dir_all(log_dir).context("Failed to create log directory")?;

        let log_path = log_dir.join("plugin-supervisor.log");

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .context("Failed to open supervisor log file")?;

        Ok(Self {
            log_file: Arc::new(Mutex::new(file)),
        })
    }

    /// Log a message
    // TODO: Integrate in general supervisor logging
    #[allow(dead_code)]
    pub async fn log(&self, level: LogLevel, message: &str) -> Result<()> {
        let entry = LogEntry::new(level, message);
        let json = entry.to_json()?;

        let mut file = self.log_file.lock().await;
        writeln!(file, "{}", json).context("Failed to write supervisor log")?;

        Ok(())
    }

    /// Log error with details
    // TODO: Integrate in error handling paths
    #[allow(dead_code)]
    pub async fn log_error(&self, message: &str, error: &str) -> Result<()> {
        let entry = LogEntry::new(LogLevel::Error, message).with_error(error);
        let json = entry.to_json()?;

        let mut file = self.log_file.lock().await;
        writeln!(file, "{}", json).context("Failed to write supervisor log")?;

        Ok(())
    }

    /// Log plugin event (spawn, kill, crash, restart, etc.)
    pub async fn log_plugin_event(
        &self,
        level: LogLevel,
        plugin_id: &str,
        event: &str,
        details: Option<&str>,
    ) -> Result<()> {
        let message = if let Some(details) = details {
            format!("Plugin {}: {} - {}", plugin_id, event, details)
        } else {
            format!("Plugin {}: {}", plugin_id, event)
        };

        let entry = LogEntry::new(level, &message).with_plugin(plugin_id);
        let json = entry.to_json()?;

        let mut file = self.log_file.lock().await;
        writeln!(file, "{}", json).context("Failed to write supervisor log")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new(LogLevel::Info, "Test message")
            .with_plugin("test-plugin")
            .with_error("Test error");

        assert_eq!(entry.level, "Info");
        assert_eq!(entry.message, "Test message");
        assert_eq!(entry.plugin, Some("test-plugin".to_string()));
        assert_eq!(entry.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_log_level_severity() {
        assert!(LogLevel::Error.severity() > LogLevel::Info.severity());
        assert!(LogLevel::Info.severity() > LogLevel::Debug.severity());
        assert!(LogLevel::Trace.severity() == 0);
    }

    #[test]
    fn test_log_level_parse_level() {
        assert_eq!(LogLevel::parse_level("info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::parse_level("ERROR"), Some(LogLevel::Error));
        assert_eq!(LogLevel::parse_level("invalid"), None);
    }
}
