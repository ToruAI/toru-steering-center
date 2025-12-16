use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::{self, DbPool, QuickAction, TaskHistory, User, UserRole};
use crate::routes::auth::{AdminUser, AuthUser};
use crate::services::auth::{hash_password, validate_password};
use crate::services::system::{get_system_resources, SystemResources};
use sysinfo::System;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub sys: Arc<Mutex<System>>,
}

pub fn create_api_router() -> Router<AppState> {
    Router::new()
        // Public routes (still need auth)
        .route("/health", get(health))
        .route("/resources", get(resources))
        .route("/history", get(get_history))
        .route("/quick-actions", get(get_quick_actions))
        // Admin-only routes
        .route("/scripts", get(list_scripts))
        .route("/settings", get(get_settings))
        .route("/settings/:key", put(update_setting))
        .route("/quick-actions", post(create_quick_action))
        .route("/quick-actions/:id", delete(delete_quick_action))
        .route("/quick-actions/:id/execute", post(execute_quick_action))
        // User management (admin-only)
        .route("/users", get(list_users))
        .route("/users", post(create_user))
        .route("/users/:id", get(get_user))
        .route("/users/:id", put(update_user))
        .route("/users/:id", delete(delete_user))
        .route("/users/:id/password", put(reset_user_password))
        // Self-service password change (any authenticated user)
        .route("/me/password", put(change_own_password))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn resources(
    _auth: AuthUser,  // Require any authenticated user
    State(state): State<AppState>,
) -> Result<Json<SystemResources>, StatusCode> {
    let mut sys = state.sys.lock().await;
    let resources = get_system_resources(&mut sys);
    Ok(Json(resources))
}

async fn list_scripts(
    _auth: AdminUser,  // Admin only
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let scripts_dir = db::get_setting(&state.db, "scripts_dir")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or_else(|| "./scripts".to_string());
    
    let dir = PathBuf::from(&scripts_dir);
    let mut scripts = Vec::new();
    
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".sh") || name.ends_with(".bash") {
                    scripts.push(name.to_string());
                }
            }
        }
    }
    
    Ok(Json(scripts))
}

#[derive(Serialize)]
struct SettingsResponse {
    settings: Vec<db::Setting>,
}

async fn get_settings(
    _auth: AdminUser,  // Admin only
    State(state): State<AppState>,
) -> Result<Json<SettingsResponse>, StatusCode> {
    let settings = db::get_all_settings(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(SettingsResponse { settings }))
}

#[derive(Deserialize)]
struct UpdateSettingRequest {
    value: String,
}

async fn update_setting(
    _auth: AdminUser,  // Admin only
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<UpdateSettingRequest>,
) -> Result<StatusCode, StatusCode> {
    db::set_setting(&state.db, &key, &payload.value)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_history(
    _auth: AuthUser,  // Any authenticated user
    State(state): State<AppState>,
) -> Result<Json<Vec<TaskHistory>>, StatusCode> {
    let history = db::get_task_history(&state.db, 100)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(history))
}

async fn get_quick_actions(
    _auth: AuthUser,  // Any authenticated user
    State(state): State<AppState>,
) -> Result<Json<Vec<QuickAction>>, StatusCode> {
    let actions = db::get_quick_actions(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(actions))
}

#[derive(Deserialize)]
struct CreateQuickActionRequest {
    name: String,
    script_path: String,
    icon: Option<String>,
    display_order: Option<i32>,
}

async fn create_quick_action(
    _auth: AdminUser,  // Admin only
    State(state): State<AppState>,
    Json(payload): Json<CreateQuickActionRequest>,
) -> Result<Json<QuickAction>, StatusCode> {
    let id = uuid::Uuid::new_v4().to_string();
    let action = QuickAction {
        id,
        name: payload.name,
        script_path: payload.script_path,
        icon: payload.icon,
        display_order: payload.display_order.unwrap_or(0),
    };
    
    db::create_quick_action(&state.db, &action)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(action))
}

async fn execute_quick_action(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 1. Get Quick Action
    let actions = db::get_quick_actions(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let action = actions.into_iter().find(|a| a.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // 2. Prepare paths
    let scripts_dir = db::get_setting(&state.db, "scripts_dir")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or_else(|| "./scripts".to_string());
    
    let script_path = format!("{}/{}", scripts_dir, action.script_path);
    let task_id = uuid::Uuid::new_v4().to_string();
    let task_id_clone = task_id.clone();

    // 3. Run safely
    let db_clone = state.db.clone();
    // Use a transient registry since we don't support API-based cancellation yet
    let registry = crate::services::executor::create_task_registry(); 
    
    tokio::spawn(async move {
        let _ = crate::services::executor::run_script_task(
            script_path,
            task_id_clone,
            action.script_path,
            db_clone,
            registry,
            None // No real-time streaming to caller, just DB updates
        ).await;
    });

    // 4. Return task_id so frontend can navigate/poll
    Ok(Json(serde_json::json!({ "task_id": task_id })))
}

async fn delete_quick_action(
    _auth: AdminUser,  // Admin only
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    db::delete_quick_action(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ============ User Management Routes (Admin Only) ============

#[derive(Serialize)]
struct UserResponse {
    id: String,
    username: String,
    display_name: Option<String>,
    role: UserRole,
    is_active: bool,
    created_at: String,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        UserResponse {
            id: u.id,
            username: u.username,
            display_name: u.display_name,
            role: u.role,
            is_active: u.is_active,
            created_at: u.created_at,
        }
    }
}

async fn list_users(
    _auth: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<UserResponse>>, StatusCode> {
    let users = db::get_all_users(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

#[derive(Deserialize)]
struct CreateUserRequest {
    username: String,
    password: String,
    display_name: Option<String>,
}

async fn create_user(
    _auth: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<UserResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Validate password strength
    if let Err(msg) = validate_password(&payload.password) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": msg })),
        ));
    }
    
    // Check if username already exists
    if let Ok(Some(_)) = db::get_user_by_username(&state.db, &payload.username).await {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "Username already exists" })),
        ));
    }
    
    let password_hash = hash_password(&payload.password)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Failed to hash password" })),
        ))?;
    
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: payload.username,
        password_hash,
        display_name: payload.display_name,
        role: UserRole::Client,
        is_active: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    
    db::create_user(&state.db, &user)
        .await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Failed to create user" })),
        ))?;
    
    Ok(Json(UserResponse::from(user)))
}

async fn get_user(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<UserResponse>, StatusCode> {
    let user = db::get_user_by_id(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(UserResponse::from(user)))
}

#[derive(Deserialize)]
struct UpdateUserRequest {
    display_name: Option<String>,
    is_active: Option<bool>,
}

async fn update_user(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, StatusCode> {
    let user = db::get_user_by_id(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let is_active = payload.is_active.unwrap_or(user.is_active);
    
    // Handle display_name: None = keep existing, Some("") = clear, Some(value) = update
    let display_name = match &payload.display_name {
        Some(name) if name.is_empty() => None,  // Empty string = clear
        Some(name) => Some(name.as_str()),       // Non-empty = update
        None => user.display_name.as_deref(),    // Not provided = keep existing
    };
    
    db::update_user(&state.db, &id, display_name, is_active)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let updated_user = db::get_user_by_id(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(UserResponse::from(updated_user)))
}

async fn delete_user(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    db::delete_user(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct ResetPasswordRequest {
    password: String,
}

async fn reset_user_password(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // Validate password strength
    if let Err(msg) = validate_password(&payload.password) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": msg })),
        ));
    }
    
    // Verify user exists
    let _ = db::get_user_by_id(&state.db, &id)
        .await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Database error" })),
        ))?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "User not found" })),
        ))?;
    
    let password_hash = hash_password(&payload.password)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Failed to hash password" })),
        ))?;
    
    db::update_user_password(&state.db, &id, &password_hash)
        .await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Failed to update password" })),
        ))?;
    
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

async fn change_own_password(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // Validate new password strength
    if let Err(msg) = validate_password(&payload.new_password) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": msg })),
        ));
    }
    
    // Admin users (from env) can't change password via this endpoint
    if auth.user_id.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Admin password is managed via environment variables" })),
        ));
    }
    
    let user_id = auth.user_id.unwrap();
    
    // Get current user and verify current password
    let user = db::get_user_by_id(&state.db, &user_id)
        .await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Database error" })),
        ))?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "User not found" })),
        ))?;
    
    // Verify current password
    if !crate::services::auth::verify_password(&payload.current_password, &user.password_hash) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Current password is incorrect" })),
        ));
    }
    
    // Hash and save new password
    let password_hash = hash_password(&payload.new_password)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Failed to hash password" })),
        ))?;
    
    db::update_user_password(&state.db, &user_id, &password_hash)
        .await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Failed to update password" })),
        ))?;
    
    Ok(StatusCode::NO_CONTENT)
}
