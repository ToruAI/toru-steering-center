use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

pub type DbPool = Arc<Mutex<Connection>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistory {
    pub id: String,
    pub script_name: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub exit_code: Option<i32>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickAction {
    pub id: String,
    pub name: String,
    pub script_path: String,
    pub icon: Option<String>,
    pub display_order: i32,
}

// Auth types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Client,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::Client => write!(f, "client"),
        }
    }
}

impl std::str::FromStr for UserRole {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(UserRole::Admin),
            "client" => Ok(UserRole::Client),
            _ => Err(anyhow::anyhow!("Invalid role: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub display_name: Option<String>,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub user_id: Option<String>, // None for admin (from env)
    pub user_role: UserRole,
    pub username: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginAttempt {
    pub id: String,
    pub username: String,
    pub ip_address: Option<String>,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub attempted_at: String,
}

pub fn init_db() -> Result<DbPool> {
    let conn = Connection::open("steering.db")?;

    // Create tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS task_history (
            id TEXT PRIMARY KEY,
            script_name TEXT NOT NULL,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            exit_code INTEGER,
            output TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS quick_actions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            script_path TEXT NOT NULL,
            icon TEXT,
            display_order INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )?;

    // Users table (for client users, admin is from env)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            display_name TEXT,
            role TEXT NOT NULL DEFAULT 'client',
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        )",
        [],
    )?;

    // Sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT,
            user_role TEXT NOT NULL,
            username TEXT NOT NULL,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL
        )",
        [],
    )?;

    // Login attempts table for security audit and rate limiting
    conn.execute(
        "CREATE TABLE IF NOT EXISTS login_attempts (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL,
            ip_address TEXT,
            success INTEGER NOT NULL,
            failure_reason TEXT,
            attempted_at TEXT NOT NULL
        )",
        [],
    )?;

    // Index for efficient rate limiting queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_login_attempts_username_time
         ON login_attempts(username, attempted_at)",
        [],
    )?;

    // Plugin KV storage (per-plugin namespace for settings/state)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS plugin_kv (
            plugin_id TEXT NOT NULL,
            key TEXT NOT NULL,
            value TEXT,
            PRIMARY KEY (plugin_id, key)
        )",
        [],
    )?;

    // Plugin events (for observability)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS plugin_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            plugin_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            details TEXT
        )",
        [],
    )?;

    // Index for efficient plugin event queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_plugin_events_plugin_timestamp
         ON plugin_events(plugin_id, timestamp)",
        [],
    )?;

    // Insert default settings
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('scripts_dir', './scripts')",
        [],
    )?;

    Ok(Arc::new(Mutex::new(conn)))
}

pub async fn get_setting(pool: &DbPool, key: &str) -> Result<Option<String>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let value: Option<String> = stmt.query_row(params![key], |row| row.get(0)).ok();
    Ok(value)
}

pub async fn set_setting(pool: &DbPool, key: &str, value: &str) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub async fn get_all_settings(pool: &DbPool) -> Result<Vec<Setting>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        Ok(Setting {
            key: row.get(0)?,
            value: row.get(1)?,
        })
    })?;

    let mut settings = Vec::new();
    for row in rows {
        settings.push(row?);
    }
    Ok(settings)
}

pub async fn insert_task_history(pool: &DbPool, task: &TaskHistory) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "INSERT INTO task_history (id, script_name, started_at, finished_at, exit_code, output) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            task.id,
            task.script_name,
            task.started_at,
            task.finished_at,
            task.exit_code,
            task.output
        ],
    )?;
    Ok(())
}

pub async fn update_task_history(
    pool: &DbPool,
    id: &str,
    finished_at: &str,
    exit_code: i32,
    output: Option<&str>,
) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "UPDATE task_history SET finished_at = ?1, exit_code = ?2, output = ?3 WHERE id = ?4",
        params![finished_at, exit_code, output, id],
    )?;
    Ok(())
}

pub async fn get_task_history(pool: &DbPool, limit: i32) -> Result<Vec<TaskHistory>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, script_name, started_at, finished_at, exit_code, output 
         FROM task_history 
         ORDER BY started_at DESC 
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(TaskHistory {
            id: row.get(0)?,
            script_name: row.get(1)?,
            started_at: row.get(2)?,
            finished_at: row.get(3)?,
            exit_code: row.get(4)?,
            output: row.get(5)?,
        })
    })?;

    let mut history = Vec::new();
    for row in rows {
        history.push(row?);
    }
    Ok(history)
}

pub async fn get_quick_actions(pool: &DbPool) -> Result<Vec<QuickAction>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, name, script_path, icon, display_order 
         FROM quick_actions 
         ORDER BY display_order ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(QuickAction {
            id: row.get(0)?,
            name: row.get(1)?,
            script_path: row.get(2)?,
            icon: row.get(3)?,
            display_order: row.get(4)?,
        })
    })?;

    let mut actions = Vec::new();
    for row in rows {
        actions.push(row?);
    }
    Ok(actions)
}

pub async fn create_quick_action(pool: &DbPool, action: &QuickAction) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "INSERT INTO quick_actions (id, name, script_path, icon, display_order) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            action.id,
            action.name,
            action.script_path,
            action.icon,
            action.display_order
        ],
    )?;
    Ok(())
}

pub async fn delete_quick_action(pool: &DbPool, id: &str) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute("DELETE FROM quick_actions WHERE id = ?1", params![id])?;
    Ok(())
}

// ============ User functions ============

pub async fn create_user(pool: &DbPool, user: &User) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "INSERT INTO users (id, username, password_hash, display_name, role, is_active, created_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            user.id,
            user.username,
            user.password_hash,
            user.display_name,
            user.role.to_string(),
            user.is_active as i32,
            user.created_at
        ],
    )?;
    Ok(())
}

pub async fn get_user_by_username(pool: &DbPool, username: &str) -> Result<Option<User>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, username, password_hash, display_name, role, is_active, created_at 
         FROM users WHERE username = ?1",
    )?;

    let user = stmt
        .query_row(params![username], |row| {
            let role_str: String = row.get(4)?;
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                display_name: row.get(3)?,
                role: role_str.parse().unwrap_or(UserRole::Client),
                is_active: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
            })
        })
        .ok();

    Ok(user)
}

pub async fn get_user_by_id(pool: &DbPool, id: &str) -> Result<Option<User>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, username, password_hash, display_name, role, is_active, created_at 
         FROM users WHERE id = ?1",
    )?;

    let user = stmt
        .query_row(params![id], |row| {
            let role_str: String = row.get(4)?;
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                display_name: row.get(3)?,
                role: role_str.parse().unwrap_or(UserRole::Client),
                is_active: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
            })
        })
        .ok();

    Ok(user)
}

pub async fn get_all_users(pool: &DbPool) -> Result<Vec<User>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, username, password_hash, display_name, role, is_active, created_at 
         FROM users ORDER BY created_at DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        let role_str: String = row.get(4)?;
        Ok(User {
            id: row.get(0)?,
            username: row.get(1)?,
            password_hash: row.get(2)?,
            display_name: row.get(3)?,
            role: role_str.parse().unwrap_or(UserRole::Client),
            is_active: row.get::<_, i32>(5)? != 0,
            created_at: row.get(6)?,
        })
    })?;

    let mut users = Vec::new();
    for row in rows {
        users.push(row?);
    }
    Ok(users)
}

pub async fn update_user(
    pool: &DbPool,
    id: &str,
    display_name: Option<&str>,
    is_active: bool,
) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "UPDATE users SET display_name = ?1, is_active = ?2 WHERE id = ?3",
        params![display_name, is_active as i32, id],
    )?;
    Ok(())
}

pub async fn update_user_password(pool: &DbPool, id: &str, password_hash: &str) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "UPDATE users SET password_hash = ?1 WHERE id = ?2",
        params![password_hash, id],
    )?;
    // Invalidate existing sessions for security
    conn.execute("DELETE FROM sessions WHERE user_id = ?1", params![id])?;
    Ok(())
}

pub async fn delete_user(pool: &DbPool, id: &str) -> Result<()> {
    let conn = pool.lock().await;
    // Also delete user's sessions
    conn.execute("DELETE FROM sessions WHERE user_id = ?1", params![id])?;
    conn.execute("DELETE FROM users WHERE id = ?1", params![id])?;
    Ok(())
}

// ============ Session functions ============

pub async fn create_session(pool: &DbPool, session: &Session) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "INSERT INTO sessions (id, user_id, user_role, username, created_at, expires_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            session.id,
            session.user_id,
            session.user_role.to_string(),
            session.username,
            session.created_at,
            session.expires_at
        ],
    )?;
    Ok(())
}

pub async fn get_session(pool: &DbPool, id: &str) -> Result<Option<Session>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, user_id, user_role, username, created_at, expires_at 
         FROM sessions WHERE id = ?1",
    )?;

    let session = stmt
        .query_row(params![id], |row| {
            let role_str: String = row.get(2)?;
            Ok(Session {
                id: row.get(0)?,
                user_id: row.get(1)?,
                user_role: role_str.parse().unwrap_or(UserRole::Client),
                username: row.get(3)?,
                created_at: row.get(4)?,
                expires_at: row.get(5)?,
            })
        })
        .ok();

    Ok(session)
}

pub async fn delete_session(pool: &DbPool, id: &str) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
    Ok(())
}

pub async fn cleanup_expired_sessions(pool: &DbPool) -> Result<()> {
    let conn = pool.lock().await;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute("DELETE FROM sessions WHERE expires_at < ?1", params![now])?;
    Ok(())
}

// ============ Plugin Types ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // Used by plugins, not yet integrated (Phase 5+)
pub struct PluginKvEntry {
    pub plugin_id: String,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // Used by plugins, not yet integrated (Phase 5+)
pub struct PluginEvent {
    pub id: i64,
    pub plugin_id: String,
    pub event_type: String, // started, stopped, crashed, restarted, disabled
    pub timestamp: String,
    pub details: Option<String>, // JSON
}

// ============ Login Attempts functions ============

pub async fn record_login_attempt(pool: &DbPool, attempt: &LoginAttempt) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "INSERT INTO login_attempts (id, username, ip_address, success, failure_reason, attempted_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            attempt.id,
            attempt.username,
            attempt.ip_address,
            attempt.success as i32,
            attempt.failure_reason,
            attempt.attempted_at
        ],
    )?;
    Ok(())
}

/// Get recent failed login attempts for rate limiting (by username)
pub async fn get_recent_failed_attempts(pool: &DbPool, username: &str, since: &str) -> Result<i32> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM login_attempts 
         WHERE username = ?1 AND success = 0 AND attempted_at > ?2",
    )?;
    let count: i32 = stmt.query_row(params![username, since], |row| row.get(0))?;
    Ok(count)
}

/// Get recent failed login attempts for rate limiting (by IP)
pub async fn get_recent_failed_attempts_by_ip(pool: &DbPool, ip: &str, since: &str) -> Result<i32> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM login_attempts 
         WHERE ip_address = ?1 AND success = 0 AND attempted_at > ?2",
    )?;
    let count: i32 = stmt.query_row(params![ip, since], |row| row.get(0))?;
    Ok(count)
}

/// Get the most recent failed attempt time for a username
pub async fn get_last_failed_attempt(pool: &DbPool, username: &str) -> Result<Option<String>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT attempted_at FROM login_attempts 
         WHERE username = ?1 AND success = 0 
         ORDER BY attempted_at DESC LIMIT 1",
    )?;
    let result = stmt.query_row(params![username], |row| row.get(0)).ok();
    Ok(result)
}

/// Get the most recent failed attempt time for an IP
pub async fn get_last_failed_attempt_by_ip(pool: &DbPool, ip: &str) -> Result<Option<String>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT attempted_at FROM login_attempts 
         WHERE ip_address = ?1 AND success = 0 
         ORDER BY attempted_at DESC LIMIT 1",
    )?;
    let result = stmt.query_row(params![ip], |row| row.get(0)).ok();
    Ok(result)
}

/// Get login attempt history (for admin view)
pub async fn get_login_attempts(pool: &DbPool, limit: i32) -> Result<Vec<LoginAttempt>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, username, ip_address, success, failure_reason, attempted_at 
         FROM login_attempts 
         ORDER BY attempted_at DESC 
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(LoginAttempt {
            id: row.get(0)?,
            username: row.get(1)?,
            ip_address: row.get(2)?,
            success: row.get::<_, i32>(3)? != 0,
            failure_reason: row.get(4)?,
            attempted_at: row.get(5)?,
        })
    })?;

    let mut attempts = Vec::new();
    for row in rows {
        attempts.push(row?);
    }
    Ok(attempts)
}

/// Clean up old login attempts (keep last 30 days)
pub async fn cleanup_old_login_attempts(pool: &DbPool) -> Result<()> {
    let conn = pool.lock().await;
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(30)).to_rfc3339();
    conn.execute(
        "DELETE FROM login_attempts WHERE attempted_at < ?1",
        params![cutoff],
    )?;
    Ok(())
}

// ============ Plugin KV functions ============

/// Get a value from plugin KV storage
#[allow(dead_code)] // Used by plugins, not yet integrated (Phase 5+)
pub async fn plugin_kv_get(pool: &DbPool, plugin_id: &str, key: &str) -> Result<Option<String>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare("SELECT value FROM plugin_kv WHERE plugin_id = ?1 AND key = ?2")?;
    let value: Option<String> = stmt
        .query_row(params![plugin_id, key], |row| row.get(0))
        .ok();
    Ok(value)
}

/// Set a value in plugin KV storage
#[allow(dead_code)] // Used by plugins, not yet integrated (Phase 5+)
pub async fn plugin_kv_set(pool: &DbPool, plugin_id: &str, key: &str, value: &str) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "INSERT OR REPLACE INTO plugin_kv (plugin_id, key, value) VALUES (?1, ?2, ?3)",
        params![plugin_id, key, value],
    )?;
    Ok(())
}

/// Delete a value from plugin KV storage
#[allow(dead_code)] // Used by plugins, not yet integrated (Phase 5+)
pub async fn plugin_kv_delete(pool: &DbPool, plugin_id: &str, key: &str) -> Result<()> {
    let conn = pool.lock().await;
    conn.execute(
        "DELETE FROM plugin_kv WHERE plugin_id = ?1 AND key = ?2",
        params![plugin_id, key],
    )?;
    Ok(())
}

/// Get all KV entries for a plugin
#[allow(dead_code)] // Used by plugins, not yet integrated (Phase 5+)
pub async fn plugin_kv_get_all(pool: &DbPool, plugin_id: &str) -> Result<Vec<PluginKvEntry>> {
    let conn = pool.lock().await;
    let mut stmt = conn
        .prepare("SELECT plugin_id, key, value FROM plugin_kv WHERE plugin_id = ?1 ORDER BY key")?;
    let rows = stmt.query_map(params![plugin_id], |row| {
        Ok(PluginKvEntry {
            plugin_id: row.get(0)?,
            key: row.get(1)?,
            value: row.get(2)?,
        })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

// ============ Plugin Event functions ============

/// Log a plugin event
#[allow(dead_code)] // Used by plugins, not yet integrated (Phase 5+)
pub async fn plugin_event_log(
    pool: &DbPool,
    plugin_id: &str,
    event_type: &str,
    details: Option<&str>,
) -> Result<i64> {
    let conn = pool.lock().await;
    conn.execute(
        "INSERT INTO plugin_events (plugin_id, event_type, timestamp, details) VALUES (?1, ?2, ?3, ?4)",
        params![
            plugin_id,
            event_type,
            chrono::Utc::now().to_rfc3339(),
            details,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Get recent events for a plugin
#[allow(dead_code)] // Used by plugins, not yet integrated (Phase 5+)
pub async fn plugin_event_get_recent(
    pool: &DbPool,
    plugin_id: &str,
    limit: i32,
) -> Result<Vec<PluginEvent>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, plugin_id, event_type, timestamp, details
         FROM plugin_events
         WHERE plugin_id = ?1
         ORDER BY timestamp DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![plugin_id, limit], |row| {
        Ok(PluginEvent {
            id: row.get(0)?,
            plugin_id: row.get(1)?,
            event_type: row.get(2)?,
            timestamp: row.get(3)?,
            details: row.get(4)?,
        })
    })?;

    let mut events = Vec::new();
    for row in rows {
        events.push(row?);
    }
    Ok(events)
}

/// Get all recent plugin events (for dashboard)
#[allow(dead_code)] // Used by plugins, not yet integrated (Phase 5+)
pub async fn plugin_event_get_all_recent(pool: &DbPool, limit: i32) -> Result<Vec<PluginEvent>> {
    let conn = pool.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id, plugin_id, event_type, timestamp, details
         FROM plugin_events
         ORDER BY timestamp DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(PluginEvent {
            id: row.get(0)?,
            plugin_id: row.get(1)?,
            event_type: row.get(2)?,
            timestamp: row.get(3)?,
            details: row.get(4)?,
        })
    })?;

    let mut events = Vec::new();
    for row in rows {
        events.push(row?);
    }
    Ok(events)
}

/// Clean up old plugin events (keep last 7 days)
#[allow(dead_code)] // Used by plugins, not yet integrated (Phase 5+)
pub async fn cleanup_old_plugin_events(pool: &DbPool) -> Result<()> {
    let conn = pool.lock().await;
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(7)).to_rfc3339();
    conn.execute(
        "DELETE FROM plugin_events WHERE timestamp < ?1",
        params![cutoff],
    )?;
    Ok(())
}
