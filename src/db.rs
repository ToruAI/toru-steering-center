use anyhow::Result;
use rusqlite::{Connection, params};
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
    let value: Option<String> = stmt.query_row(params![key], |row| Ok(row.get(0)?)).ok();
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
         LIMIT ?1"
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
         ORDER BY display_order ASC"
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


