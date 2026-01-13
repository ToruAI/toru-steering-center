use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri},
    response::{IntoResponse, Json, Response},
    routing::{any, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::routes::api::AppState;
use crate::routes::auth::{AdminUser, AuthUser};
use crate::services::logging::LogLevel;
use crate::services::plugins::PluginProcess;

/// Plugin status information
#[derive(Serialize, Clone)]
pub struct PluginStatus {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub icon: String,
    pub enabled: bool,
    pub running: bool,
    pub health: String, // "healthy", "unhealthy", "disabled"
    pub pid: Option<u32>,
    pub socket_path: Option<String>,
}

impl From<&PluginProcess> for PluginStatus {
    fn from(process: &PluginProcess) -> Self {
        let health = if !process.enabled {
            "disabled".to_string()
        } else if process.process.is_some()
            && !process.socket_path.is_empty()
            && PathBuf::from(&process.socket_path).exists()
        {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        };

        PluginStatus {
            id: process.id.clone(),
            name: process
                .metadata
                .as_ref()
                .map(|m| m.name.clone())
                .unwrap_or_else(|| process.id.clone()),
            version: process
                .metadata
                .as_ref()
                .map(|m| m.version.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            author: process.metadata.as_ref().and_then(|m| m.author.clone()),
            icon: process
                .metadata
                .as_ref()
                .map(|m| m.icon.clone())
                .unwrap_or_default(),
            enabled: process.enabled,
            running: process.process.is_some(),
            health,
            pid: process.pid,
            socket_path: if process.socket_path.is_empty() {
                None
            } else {
                Some(process.socket_path.clone())
            },
        }
    }
}

pub fn create_plugin_router() -> Router<AppState> {
    // Admin routes router
    let admin_router = Router::new()
        .route("/", get(list_plugins))
        .route("/:id", get(get_plugin))
        .route("/:id/enable", post(enable_plugin))
        .route("/:id/disable", post(disable_plugin))
        .route("/:id/bundle.js", get(get_plugin_bundle))
        .route("/:id/logs", get(get_plugin_logs))
        .route("/:id/kv", post(plugin_kv_handler));

    // Dynamic plugin routes (separate path prefix to avoid conflicts)
    // Plugins declare a route in metadata (e.g., "/hello-plugin")
    // Requests to /api/plugins/route/<plugin-route>/... are forwarded to the plugin
    let plugin_routes_router = Router::new().route("/*path", any(forward_to_plugin));

    // Combine routers - admin routes checked first
    Router::new()
        .merge(admin_router)
        .nest("/route", plugin_routes_router)
}

/// Forward HTTP request to a plugin
///
/// This handler receives requests for dynamic plugin routes.
/// Routes are checked against enabled plugins' route metadata.
/// If no plugin matches, returns 404.
///
/// # Route Pattern
/// /api/plugins/*path
///
/// # Example
/// Request: GET /api/plugins/hello-plugin/some/path?query=1
/// - Path: "hello-plugin/some/path"
/// - Plugin route: "/hello-plugin"
/// - Plugin path: "/some/path?query=1"
async fn forward_to_plugin(
    _auth: AuthUser, // Require authentication (any role)
    State(state): State<AppState>,
    Path(path): Path<String>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, StatusCode> {
    // Split path into route name and remaining path
    let (plugin_route, remaining) = path.split_once('/').unwrap_or((&path, ""));

    // Security: Validate plugin_route to prevent path traversal attacks
    if plugin_route.contains("..") || plugin_route.contains('/') {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check if this path matches an enabled plugin's route
    let supervisor = state
        .supervisor
        .as_ref()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?
        .lock()
        .await;

    let plugin_id = supervisor
        .get_plugin_for_route(&format!("/{}", plugin_route))
        .ok_or(StatusCode::NOT_FOUND)?;

    // Build the path to send to plugin
    let plugin_path = if remaining.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", remaining)
    };

    // Include query string
    let full_path = if let Some(query) = uri.query() {
        format!("{}?{}", plugin_path, query)
    } else {
        plugin_path
    };

    // Convert Axum headers to HashMap
    let mut plugin_headers = HashMap::new();
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            plugin_headers.insert(name.to_string(), value_str.to_string());
        }
    }

    // Read request body
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let body_str = if body_bytes.is_empty() {
        None
    } else {
        String::from_utf8(body_bytes.to_vec()).ok()
    };

    // Build HTTP request for plugin
    let http_request = toru_plugin_api::HttpRequest {
        method: method.to_string(),
        path: full_path,
        headers: plugin_headers,
        body: body_str,
    };

    // Forward to plugin
    let response = supervisor
        .forward_http_request(&plugin_id, &http_request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to forward request to plugin {}: {}", plugin_id, e);
            StatusCode::BAD_GATEWAY
        })?;

    // Build Axum response from plugin response
    let mut builder = Response::builder().status(response.status);

    // Set headers
    for (name, value) in response.headers {
        if let Ok(header_value) = HeaderValue::from_str(&value) {
            if let Ok(header_name) = name.parse::<axum::http::HeaderName>() {
                builder = builder.header(header_name, header_value);
            }
        }
    }

    // Set body
    let response = builder
        .body(axum::body::Body::from(response.body.unwrap_or_default()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}

/// List all plugins (available to all authenticated users)
async fn list_plugins(
    _auth: AuthUser, // Changed from AdminUser to AuthUser
    State(state): State<AppState>,
) -> Result<Json<Vec<PluginStatus>>, StatusCode> {
    let supervisor = state
        .supervisor
        .as_ref()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?
        .lock()
        .await;
    let plugins = supervisor.get_all_plugins();

    let plugin_statuses: Vec<PluginStatus> = plugins.values().map(PluginStatus::from).collect();

    Ok(Json(plugin_statuses))
}

/// Get plugin details (available to all authenticated users)
async fn get_plugin(
    _auth: AuthUser, // Changed from AdminUser to AuthUser
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PluginStatus>, StatusCode> {
    let supervisor = state
        .supervisor
        .as_ref()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?
        .lock()
        .await;
    let plugin = supervisor
        .get_plugin_status(&id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(PluginStatus::from(plugin)))
}

/// Enable a plugin
async fn enable_plugin(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut supervisor = state
        .supervisor
        .as_ref()
        .ok_or((
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({ "error": "Plugin supervisor not initialized" })),
        ))?
        .lock()
        .await;

    // Check if plugin exists
    if supervisor.get_plugin_status(&id).is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Plugin not found" })),
        ));
    }

    supervisor.enable_plugin(&id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Failed to enable plugin: {}", e) })),
        )
    })?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Disable a plugin
async fn disable_plugin(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut supervisor = state
        .supervisor
        .as_ref()
        .ok_or((
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({ "error": "Plugin supervisor not initialized" })),
        ))?
        .lock()
        .await;

    // Check if plugin exists
    if supervisor.get_plugin_status(&id).is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Plugin not found" })),
        ));
    }

    supervisor.disable_plugin(&id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Failed to disable plugin: {}", e) })),
        )
    })?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Get plugin frontend bundle (available to all authenticated users)
async fn get_plugin_bundle(
    _auth: AuthUser, // Changed from AdminUser to AuthUser
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    // Security: Validate plugin ID to prevent path traversal attacks
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let supervisor = state
        .supervisor
        .as_ref()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?
        .lock()
        .await;
    let plugin = supervisor
        .get_plugin_status(&id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if plugin is enabled
    if !plugin.enabled {
        return Err(StatusCode::NOT_FOUND);
    }

    // Get plugin bundle path from plugins directory
    let plugins_dir = supervisor.get_plugins_dir();
    let bundle_path = plugins_dir.join(&id).join("bundle.js");

    if !bundle_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    let content =
        fs::read_to_string(&bundle_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(([(header::CONTENT_TYPE, "application/javascript")], content))
}

#[derive(Deserialize)]
struct LogQuery {
    #[serde(default)]
    page: usize,
    #[serde(default = "default_page_size")]
    page_size: usize,
    #[serde(default)]
    level: Option<String>,
}

fn default_page_size() -> usize {
    100
}

/// Get plugin logs with pagination and filtering
async fn get_plugin_logs(
    _auth: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<LogQuery>,
) -> Result<Json<LogsResponse>, StatusCode> {
    let supervisor = state
        .supervisor
        .as_ref()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?
        .lock()
        .await;

    // Check if plugin exists
    if supervisor.get_plugin_status(&id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let plugin_logger = supervisor.plugin_logger();

    // Parse log level filter
    let filter_level = query.level.as_ref().and_then(|l| LogLevel::parse_level(l));

    // Read logs with pagination and filtering
    let logs = plugin_logger
        .read_plugin_logs(&id, filter_level, query.page, query.page_size)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LogsResponse {
        logs,
        page: query.page,
        page_size: query.page_size,
    }))
}

#[derive(Serialize)]
struct LogsResponse {
    logs: Vec<crate::services::logging::LogEntry>,
    page: usize,
    page_size: usize,
}

/// KV operation request
#[derive(Deserialize)]
struct KvOperation {
    action: String, // "get", "set", "delete"
    key: String,
    value: Option<String>,
}

/// KV operation response
#[derive(Serialize)]
struct KvResponse {
    value: Option<String>,
}

/// Handle KV storage operations for plugins
async fn plugin_kv_handler(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(op): Json<KvOperation>,
) -> Result<Json<KvResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Validate action
    match op.action.as_str() {
        "get" => {
            // Get value from database
            let value = crate::db::plugin_kv_get(&state.db, &id, &op.key)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": format!("Failed to get KV: {}", e) })),
                    )
                })?;
            Ok(Json(KvResponse { value }))
        }
        "set" => {
            // Set value in database
            let value = op.value.ok_or((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Missing 'value' field for set operation" })),
            ))?;

            crate::db::plugin_kv_set(&state.db, &id, &op.key, &value)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": format!("Failed to set KV: {}", e) })),
                    )
                })?;

            Ok(Json(KvResponse { value: Some(value) }))
        }
        "delete" => {
            // Delete value from database
            crate::db::plugin_kv_delete(&state.db, &id, &op.key)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": format!("Failed to delete KV: {}", e) })),
                    )
                })?;

            Ok(Json(KvResponse { value: None }))
        }
        _ => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("Invalid action: {}", op.action) })),
        )),
    }
}
