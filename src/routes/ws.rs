use axum::{
    extract::{ws::Message, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::db::{self, UserRole};
use crate::routes::api::AppState;
use crate::routes::auth::SESSION_COOKIE_NAME;
use crate::services::auth::validate_session;
use crate::services::executor::{self, TaskMessage};

#[derive(Deserialize)]
struct ClientMessage {
    r#type: String,
    script: Option<String>,
    task_id: Option<String>,
}

pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    jar: CookieJar,
) -> Response {
    // Validate session cookie before upgrading to WebSocket
    let session_id = match jar.get(SESSION_COOKIE_NAME) {
        Some(cookie) => cookie.value().to_string(),
        None => {
            return (StatusCode::UNAUTHORIZED, "Not authenticated").into_response();
        }
    };
    
    let session = match validate_session(&state.db, &session_id).await {
        Some(s) => s,
        None => {
            return (StatusCode::UNAUTHORIZED, "Invalid or expired session").into_response();
        }
    };
    
    let is_admin = session.user_role == UserRole::Admin;
    let session_id = session.id.clone();
    
    ws.on_upgrade(move |socket| handle_socket(socket, state, session_id, is_admin))
}

async fn handle_socket(socket: axum::extract::ws::WebSocket, state: AppState, session_id: String, is_admin: bool) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));
    let registry = executor::create_task_registry();
    let mut session_check_interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
    
    loop {
        tokio::select! {
             _ = session_check_interval.tick() => {
                 // Re-validate session
                 if validate_session(&state.db, &session_id).await.is_none() {
                     tracing::warn!("Session expired or invalid during WebSocket connection, closing.");
                     let error_msg = TaskMessage {
                        r#type: "error".to_string(),
                        task_id: None,
                        data: Some("Session expired".to_string()),
                        code: None,
                     };
                     let mut s = sender.lock().await;
                     let _ = s.send(Message::Text(
                         serde_json::to_string(&error_msg).unwrap(),
                     )).await;
                     break;
                 }
             }

             msg = receiver.next() => {
                let msg = match msg {
                    Some(Ok(msg)) => msg,
                    Some(Err(_)) => break,
                    None => break,
                };
                
                let text = match msg.to_text() {
                    Ok(text) => text,
                    Err(_) => continue,
                };
                
                let client_msg: ClientMessage = match serde_json::from_str(text) {
                    Ok(msg) => msg,
                    Err(_) => continue,
                };
                
                match client_msg.r#type.as_str() {
                    "run" => {
                        if let Some(script_name) = client_msg.script {
                            // Check permissions
                            let mut allowed = is_admin;
                            if !allowed {
                                // Check if it's a quick action
                                let actions = db::get_quick_actions(&state.db).await.unwrap_or_default();
                                if actions.iter().any(|a| a.script_path == script_name) {
                                    allowed = true; // Allowed if it matches a registered quick action
                                }
                            }

                            if !allowed {
                                let error_msg = TaskMessage {
                                    r#type: "error".to_string(),
                                    task_id: None,
                                    data: Some("Admin access required to run this script".to_string()),
                                    code: None,
                                };
                                let mut s = sender.lock().await;
                                let _ = s.send(Message::Text(
                                    serde_json::to_string(&error_msg).unwrap(),
                                )).await;
                                continue;
                            }

                            let scripts_dir = db::get_setting(&state.db, "scripts_dir")
                                .await
                                .unwrap_or_else(|_| Some("./scripts".to_string()))
                                .unwrap_or_else(|| "./scripts".to_string());
                            
                            let script_path = format!("{}/{}", scripts_dir, script_name);
                            let task_id = Uuid::new_v4().to_string();

                            // Create channel for streaming output back to WS
                            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                            let sender_clone = sender.clone();
                            
                            // Bridge task: MPSC -> WebSocket
                            tokio::spawn(async move {
                                while let Some(msg) = rx.recv().await {
                                    let text = serde_json::to_string(&msg).unwrap();
                                    let mut s = sender_clone.lock().await;
                                    if s.send(Message::Text(text)).await.is_err() {
                                        break;
                                    }
                                }
                            });
                            
                            // Run the task (detached)
                            let _ = executor::run_script_task(
                                script_path,
                                task_id,
                                script_name,
                                state.db.clone(),
                                registry.clone(),
                                Some(tx) // Pass the sender to stream output
                            ).await;
                        }
                    }
                    "cancel" => {
                        if let Some(task_id) = client_msg.task_id {
                            if executor::cancel_task(&task_id, &registry).await.unwrap_or(false) {
                                let cancelled_msg = TaskMessage {
                                    r#type: "cancelled".to_string(),
                                    task_id: Some(task_id.clone()),
                                    data: None,
                                    code: None,
                                };
                                let mut s = sender.lock().await;
                                let _ = s.send(Message::Text(
                                    serde_json::to_string(&cancelled_msg).unwrap(),
                                )).await;
                                
                                // Clean up registry
                                executor::remove_task(&task_id, &registry).await;
                            }
                        }
                    }
                    _ => {}
                }
             }
        }
    }
}

