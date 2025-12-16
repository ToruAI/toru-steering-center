use axum::{
    async_trait,
    extract::{ConnectInfo, FromRequestParts, State},
    http::{HeaderMap, request::Parts, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::db::{LoginAttempt, UserRole};
use crate::routes::api::AppState;
use crate::services::auth::{
    authenticate_admin, authenticate_user, create_user_session, validate_session,
    SESSION_DURATION_DAYS,
};

pub const SESSION_COOKIE_NAME: &str = "session_id";
const ADMIN_DISPLAY_NAME_DEFAULT: &str = "Administrator";

/// Rate limiting thresholds: (attempts, lockout_minutes)
const RATE_LIMIT_TIERS: &[(i32, i64)] = &[
    (3, 1),    // After 3 failures: 1 minute
    (6, 3),    // After 6 failures: 3 minutes
    (9, 10),   // After 9 failures: 10 minutes
    (12, 30),  // After 12 failures: 30 minutes
];

pub fn create_auth_router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/login-history", get(get_login_history))
}

/// Helper to check if running in production/secure mode
fn is_secure_mode() -> bool {
    let prod = std::env::var("PRODUCTION")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);
    let secure = std::env::var("SECURE_COOKIES")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);
    prod || secure
}

/// Build a session cookie with proper security flags
fn build_session_cookie(session_id: String) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE_NAME, session_id))
        .path("/")
        .http_only(true)
        .secure(is_secure_mode())
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::days(SESSION_DURATION_DAYS))
        .build()
}

/// Calculate lockout duration based on failed attempts
fn get_lockout_duration(failed_attempts: i32) -> Option<i64> {
    for &(threshold, minutes) in RATE_LIMIT_TIERS.iter().rev() {
        if failed_attempts >= threshold {
            return Some(minutes);
        }
    }
    None
}

/// Check if user is rate limited and return remaining lockout time
async fn check_rate_limit(pool: &crate::db::DbPool, username: &str, ip: Option<&str>) -> Option<i64> {
    // Check failures in the last hour
    let one_hour_ago = (Utc::now() - Duration::hours(1)).to_rfc3339();
    
    // Check username rate limit
    let failed_attempts_user = crate::db::get_recent_failed_attempts(pool, username, &one_hour_ago)
        .await
        .unwrap_or(0);
        
    // Check IP rate limit if available
    let failed_attempts_ip = if let Some(ip_addr) = ip {
        crate::db::get_recent_failed_attempts_by_ip(pool, ip_addr, &one_hour_ago)
            .await
            .unwrap_or(0)
        } else {
            0
        };
    
    // Use the higher failure count
    let failed_attempts = std::cmp::max(failed_attempts_user, failed_attempts_ip);
    
    if let Some(lockout_minutes) = get_lockout_duration(failed_attempts) {
        // Find the most recent failure time (either by user or IP)
        let last_failure_user = crate::db::get_last_failed_attempt(pool, username).await.ok().flatten();
        let last_failure_ip = if let Some(ip_addr) = ip {
            crate::db::get_last_failed_attempt_by_ip(pool, ip_addr).await.ok().flatten()
        } else {
            None
        };
        
        // Pick the latest timestamp
        let last_failure = match (last_failure_user, last_failure_ip) {
            (Some(u), Some(i)) => if u > i { Some(u) } else { Some(i) },
            (Some(u), None) => Some(u),
            (None, Some(i)) => Some(i),
            (None, None) => None,
        };

        if let Some(last_ts) = last_failure {
            if let Ok(last_time) = chrono::DateTime::parse_from_rfc3339(&last_ts) {
                let last_time_utc = last_time.with_timezone(&Utc);
                let lockout_until = last_time_utc + Duration::minutes(lockout_minutes);
                let now = Utc::now();
                if now < lockout_until {
                    let remaining = (lockout_until - now).num_seconds();
                    return Some(remaining);
                }
            }
        }
    }
    None
}

/// Record a login attempt
async fn record_attempt(
    pool: &crate::db::DbPool,
    username: &str,
    ip: Option<String>,
    success: bool,
    failure_reason: Option<&str>,
) {
    let attempt = LoginAttempt {
        id: uuid::Uuid::new_v4().to_string(),
        username: username.to_string(),
        ip_address: ip,
        success,
        failure_reason: failure_reason.map(String::from),
        attempted_at: Utc::now().to_rfc3339(),
    };
    let _ = crate::db::record_login_attempt(pool, &attempt).await;
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    success: bool,
    user: Option<UserInfo>,
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    locked_until: Option<i64>,  // Seconds until lockout ends
}

#[derive(Serialize)]
struct UserInfo {
    id: Option<String>,
    username: String,
    display_name: Option<String>,
    role: UserRole,
}

/// Helper to get client IP, respecting proxy headers if configured
fn get_client_ip(headers: &HeaderMap, connect_info: Option<&ConnectInfo<SocketAddr>>) -> Option<String> {
    // Check if we trust proxy headers
    let trust_proxy = std::env::var("TRUST_PROXY")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);

    if trust_proxy {
        // Try Cloudflare header first
        if let Some(ip) = headers
            .get("cf-connecting-ip")
            .and_then(|h| h.to_str().ok()) 
        {
            return Some(ip.to_string());
        }
        
        // Try X-Forwarded-For
        if let Some(ip) = headers
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(',').next()) // Take first IP
            .map(|s| s.trim().to_string())
        {
            return Some(ip);
        }
    }
    
    // Fallback to direct connection IP
    connect_info.map(|ci| ci.0.ip().to_string())
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let ip = get_client_ip(&headers, connect_info.as_ref());
    
    // Check rate limiting
    if let Some(remaining_seconds) = check_rate_limit(&state.db, &payload.username, ip.as_deref()).await {
        let minutes = (remaining_seconds / 60) + 1;
        
        // Log the lockout event
        record_attempt(&state.db, &payload.username, ip.clone(), false, Some("Rate limit exceeded")).await;
        
        return (
            StatusCode::TOO_MANY_REQUESTS,
            jar,
            Json(LoginResponse {
                success: false,
                user: None,
                error: Some(format!("Too many failed attempts. Please wait {} minute(s).", minutes)),
                locked_until: Some(remaining_seconds),
            }),
        );
    }
    
    // First try admin authentication
    if authenticate_admin(&payload.username, &payload.password) {
        let session = match create_user_session(
            &state.db,
            None, // No user_id for admin
            &payload.username,
            UserRole::Admin,
        )
        .await
        {
            Ok(s) => s,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    jar,
                    Json(LoginResponse {
                        success: false,
                        user: None,
                        error: Some("Failed to create session".to_string()),
                        locked_until: None,
                    }),
                );
            }
        };

        // Record successful login
        record_attempt(&state.db, &payload.username, ip, true, None).await;

        return (
            StatusCode::OK,
            jar.add(build_session_cookie(session.id)),
            Json(LoginResponse {
                success: true,
                user: Some(UserInfo {
                    id: None,
                    username: payload.username,
                    display_name: Some(std::env::var("ADMIN_DISPLAY_NAME").unwrap_or_else(|_| ADMIN_DISPLAY_NAME_DEFAULT.to_string())),
                    role: UserRole::Admin,
                }),
                error: None,
                locked_until: None,
            }),
        );
    }

    // Try client user authentication
    if let Some(user) = authenticate_user(&state.db, &payload.username, &payload.password).await {
        let session = match create_user_session(
            &state.db,
            Some(user.id.clone()),
            &user.username,
            user.role,
        )
        .await
        {
            Ok(s) => s,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    jar,
                    Json(LoginResponse {
                        success: false,
                        user: None,
                        error: Some("Failed to create session".to_string()),
                        locked_until: None,
                    }),
                );
            }
        };

        // Record successful login
        record_attempt(&state.db, &payload.username, ip, true, None).await;

        return (
            StatusCode::OK,
            jar.add(build_session_cookie(session.id)),
            Json(LoginResponse {
                success: true,
                user: Some(UserInfo {
                    id: Some(user.id),
                    username: user.username,
                    display_name: user.display_name,
                    role: user.role,
                }),
                error: None,
                locked_until: None,
            }),
        );
    }

    // Authentication failed - record it
    record_attempt(&state.db, &payload.username, ip, false, Some("Invalid credentials")).await;
    
    (
        StatusCode::UNAUTHORIZED,
        jar,
        Json(LoginResponse {
            success: false,
            user: None,
            error: Some("Invalid username or password".to_string()),
            locked_until: None,
        }),
    )
}

async fn logout(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    if let Some(session_cookie) = jar.get(SESSION_COOKIE_NAME) {
        let _ = crate::db::delete_session(&state.db, session_cookie.value()).await;
    }

    let cookie = Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .secure(is_secure_mode())
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::seconds(0))
        .build();

    (StatusCode::OK, jar.remove(cookie), Json(serde_json::json!({ "success": true })))
}

// Login history endpoint (admin only)
async fn get_login_history(
    _auth: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<LoginAttempt>>, StatusCode> {
    let attempts = crate::db::get_login_attempts(&state.db, 100)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(attempts))
}

#[derive(Serialize)]
struct MeResponse {
    authenticated: bool,
    user: Option<UserInfo>,
}

async fn me(State(state): State<AppState>, jar: CookieJar) -> Json<MeResponse> {
    let session_id = match jar.get(SESSION_COOKIE_NAME) {
        Some(cookie) => cookie.value(),
        None => {
            return Json(MeResponse {
                authenticated: false,
                user: None,
            });
        }
    };

    match validate_session(&state.db, session_id).await {
        Some(session) => {
            // Get display name for client users
            let display_name = if let Some(ref user_id) = session.user_id {
                crate::db::get_user_by_id(&state.db, user_id)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|u| u.display_name)
            } else {
                Some(std::env::var("ADMIN_DISPLAY_NAME").unwrap_or_else(|_| ADMIN_DISPLAY_NAME_DEFAULT.to_string()))
            };

            Json(MeResponse {
                authenticated: true,
                user: Some(UserInfo {
                    id: session.user_id,
                    username: session.username,
                    display_name,
                    role: session.user_role,
                }),
            })
        }
        None => Json(MeResponse {
            authenticated: false,
            user: None,
        }),
    }
}

// ============ Auth Extractors ============

/// Authenticated user info extracted from session
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Option<String>,
    #[allow(dead_code)]
    pub username: String,
    pub role: UserRole,
}

impl AuthUser {
    #[allow(dead_code)]
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }
}

/// Extractor that requires authentication (any role)
#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract cookies manually from headers
        let session_id = parts
            .headers
            .get_all("cookie")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|s| s.split(';'))
            .filter_map(|cookie| {
                let mut parts = cookie.trim().splitn(2, '=');
                let name = parts.next()?;
                let value = parts.next()?;
                if name == SESSION_COOKIE_NAME {
                    Some(value.to_string())
                } else {
                    None
                }
            })
            .next();

        let session_id = match session_id {
            Some(id) => id,
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "error": "Not authenticated" })),
                ));
            }
        };

        match validate_session(&state.db, &session_id).await {
            Some(session) => Ok(AuthUser {
                user_id: session.user_id,
                username: session.username,
                role: session.user_role,
            }),
            None => Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Session expired or invalid" })),
            )),
        }
    }
}

/// Extractor that requires admin role
#[derive(Debug, Clone)]
pub struct AdminUser(#[allow(dead_code)] pub AuthUser);

#[async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        
        if auth_user.role != UserRole::Admin {
            return Err((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({ "error": "Admin access required" })),
            ));
        }
        
        Ok(AdminUser(auth_user))
    }
}
