use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;
use crate::db::{self, DbPool, TaskHistory};
use chrono::Utc;

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

/// Stores the child process handle for cancellation
pub type TaskRegistry = Arc<Mutex<HashMap<String, Arc<Mutex<Option<tokio::process::Child>>>>>>;

pub fn create_task_registry() -> TaskRegistry {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Spawns a script and returns stdout/stderr handles separately.
/// The Child is wrapped for safe cancellation while streaming.
pub async fn execute_script(
    script_path: &str,
) -> Result<tokio::process::Child> {
    let child = TokioCommand::new("sh")
        .arg(script_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    Ok(child)
}

/// Stores task handle in registry for cancellation support
pub async fn store_task(task_id: String, child: tokio::process::Child, registry: &TaskRegistry) {
    let mut reg = registry.lock().await;
    reg.insert(task_id, Arc::new(Mutex::new(Some(child))));
}

/// Gets the task handle from registry (does not remove it)
pub async fn get_task(task_id: &str, registry: &TaskRegistry) -> Option<Arc<Mutex<Option<tokio::process::Child>>>> {
    let reg = registry.lock().await;
    reg.get(task_id).cloned()
}

/// Removes task from registry (called after task completes)
pub async fn remove_task(task_id: &str, registry: &TaskRegistry) {
    let mut reg = registry.lock().await;
    reg.remove(task_id);
}

/// Cancels a running task by killing the child process
pub async fn cancel_task(task_id: &str, registry: &TaskRegistry) -> Result<bool> {
    let task_handle = {
        let reg = registry.lock().await;
        reg.get(task_id).cloned()
    };
    
    if let Some(handle) = task_handle {
        let mut child_opt = handle.lock().await;
        if let Some(ref mut child) = *child_opt {
            child.kill().await?;
            *child_opt = None; // Mark as killed
            return Ok(true);
        }
    }
    Ok(false)
}

/// Runs a script, monitors output, updates DB, and optionally streams events to a channel
pub async fn run_script_task(
    script_path: String,
    task_id: String,
    script_name: String,
    db: DbPool,
    registry: TaskRegistry,
    event_sender: Option<tokio::sync::mpsc::UnboundedSender<TaskMessage>>,
) -> Result<()> {
    // 1. Create task history entry
    let task_history = TaskHistory {
        id: task_id.clone(),
        script_name: script_name.clone(),
        started_at: Utc::now().to_rfc3339(),
        finished_at: None,
        exit_code: None,
        output: None,
    };
    
    if let Err(e) = db::insert_task_history(&db, &task_history).await {
        tracing::error!("Failed to insert task history: {}", e);
        // We continue anyway
    }

    // 2. Notify started
    if let Some(ref tx) = event_sender {
        let _ = tx.send(TaskMessage {
            r#type: "started".to_string(),
            task_id: Some(task_id.clone()),
            data: None,
            code: None,
        });
    }

    // 3. Execute script
    let mut child = match execute_script(&script_path).await {
        Ok(c) => c,
        Err(e) => {
            let err_msg = format!("Failed to start script: {}", e);
            if let Some(ref tx) = event_sender {
                let _ = tx.send(TaskMessage {
                    r#type: "error".to_string(),
                    task_id: Some(task_id.clone()),
                    data: Some(err_msg.clone()),
                    code: None,
                });
            }
            // Update DB with failure
            let finished_at = Utc::now().to_rfc3339();
            let _ = db::update_task_history(&db, &task_id, &finished_at, -1, Some(&err_msg)).await;
            return Err(e);
        }
    };

    // 4. Capture output handles
    let stdout = child.stdout.take().expect("stdout not captured");
    let stderr = child.stderr.take().expect("stderr not captured");

    // 5. Store in registry
    store_task(task_id.clone(), child, &registry).await;

    // 6. Spawn monitoring task
    tokio::spawn(async move {
        let mut stdout_reader = BufReader::new(stdout);
        let mut stderr_reader = BufReader::new(stderr);
        let mut output_buffer = String::new();
        let mut stdout_line = String::new();
        let mut stderr_line = String::new();
        let mut stdout_done = false;
        let mut stderr_done = false;

        // Stream output
        while !stdout_done || !stderr_done {
            tokio::select! {
                result = stdout_reader.read_line(&mut stdout_line), if !stdout_done => {
                    match result {
                        Ok(0) => stdout_done = true,
                        Ok(_) => {
                            let line = stdout_line.clone();
                            output_buffer.push_str(&line);
                            if let Some(ref tx) = event_sender {
                                let _ = tx.send(TaskMessage {
                                    r#type: "stdout".to_string(),
                                    task_id: Some(task_id.clone()),
                                    data: Some(line.trim_end().to_string()),
                                    code: None,
                                });
                            }
                            stdout_line.clear();
                        }
                        Err(_) => stdout_done = true,
                    }
                }
                result = stderr_reader.read_line(&mut stderr_line), if !stderr_done => {
                    match result {
                        Ok(0) => stderr_done = true,
                        Ok(_) => {
                            let line = stderr_line.clone();
                            output_buffer.push_str(&line);
                            if let Some(ref tx) = event_sender {
                                let _ = tx.send(TaskMessage {
                                    r#type: "stderr".to_string(),
                                    task_id: Some(task_id.clone()),
                                    data: Some(line.trim_end().to_string()),
                                    code: None,
                                });
                            }
                            stderr_line.clear();
                        }
                        Err(_) => stderr_done = true,
                    }
                }
            }
        }

        // Wait for exit
        let exit_code = if let Some(handle) = get_task(&task_id, &registry).await {
            let mut child_opt = handle.lock().await;
            if let Some(ref mut child) = *child_opt {
                let status = child.wait().await;
                status.ok().and_then(|s| s.code()).unwrap_or(-1)
            } else {
                -1
            }
        } else {
            -1
        };

        // Remove from registry
        remove_task(&task_id, &registry).await;

        // Update DB
        let finished_at = Utc::now().to_rfc3339();
        let output_str = if output_buffer.is_empty() { None } else { Some(output_buffer.as_str()) };
        let _ = db::update_task_history(&db, &task_id, &finished_at, exit_code, output_str).await;

        // Notify exit
        if let Some(ref tx) = event_sender {
            let _ = tx.send(TaskMessage {
                r#type: "exit".to_string(),
                task_id: Some(task_id),
                data: None,
                code: Some(exit_code),
            });
        }
    });

    Ok(())
}



