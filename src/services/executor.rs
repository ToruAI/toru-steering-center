use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMessage {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
}

pub type TaskRegistry = Arc<Mutex<HashMap<String, tokio::process::Child>>>;

pub fn create_task_registry() -> TaskRegistry {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn execute_script(
    script_path: &str,
    _task_id: String,
    _registry: TaskRegistry,
) -> Result<tokio::process::Child> {
    let child = TokioCommand::new("sh")
        .arg(script_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    Ok(child)
}

pub async fn store_task(task_id: String, child: tokio::process::Child, registry: TaskRegistry) {
    let mut reg = registry.lock().await;
    reg.insert(task_id, child);
}

pub async fn cancel_task(task_id: &str, registry: TaskRegistry) -> Result<bool> {
    let mut reg = registry.lock().await;
    if let Some(mut child) = reg.remove(task_id) {
        child.kill().await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn stream_output(
    mut child: tokio::process::Child,
) -> Result<(i32, String)> {
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    
    let mut output = String::new();
    let mut stdout_reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);
    
    let mut stdout_line = String::new();
    let mut stderr_line = String::new();
    
    loop {
        tokio::select! {
            result = stdout_reader.read_line(&mut stdout_line) => {
                match result {
                    Ok(0) => break,
                    Ok(_) => {
                        output.push_str(&stdout_line);
                        stdout_line.clear();
                    }
                    Err(_) => break,
                }
            }
            result = stderr_reader.read_line(&mut stderr_line) => {
                match result {
                    Ok(0) => {}
                    Ok(_) => {
                        output.push_str(&stderr_line);
                        stderr_line.clear();
                    }
                    Err(_) => {}
                }
            }
        }
    }
    
    let status = child.wait().await?;
    let exit_code = status.code().unwrap_or(-1);
    
    Ok((exit_code, output))
}


