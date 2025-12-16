use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use rand::RngCore;
use subtle::ConstantTimeEq;

use crate::db::{DbPool, Session, User, UserRole};

/// Session duration in days
pub const SESSION_DURATION_DAYS: i64 = 7;

/// Minimum password length
pub const MIN_PASSWORD_LENGTH: usize = 8;

/// Hash a password using Argon2
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// Generate a cryptographically secure random session token
pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    // Use hex encoding for URL-safe token
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Validate password strength
pub fn validate_password(password: &str) -> Result<(), &'static str> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err("Password must be at least 8 characters long");
    }
    
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_number = password.chars().any(|c| c.is_numeric());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());
    
    if !has_uppercase || !has_lowercase || !has_number || !has_special {
        return Err("Password must contain uppercase, lowercase, number, and special character");
    }
    
    Ok(())
}

/// Create a new session for a user
pub async fn create_user_session(
    pool: &DbPool,
    user_id: Option<String>,
    username: &str,
    role: UserRole,
) -> anyhow::Result<Session> {
    let now = Utc::now();
    let expires_at = now + Duration::days(SESSION_DURATION_DAYS);
    
    let session = Session {
        id: generate_session_token(),
        user_id,
        user_role: role,
        username: username.to_string(),
        created_at: now.to_rfc3339(),
        expires_at: expires_at.to_rfc3339(),
    };
    
    crate::db::create_session(pool, &session).await?;
    Ok(session)
}

/// Validate a session and return it if valid
pub async fn validate_session(pool: &DbPool, session_id: &str) -> Option<Session> {
    let session = crate::db::get_session(pool, session_id).await.ok()??;
    
    // Check if session is expired
    let expires_at = chrono::DateTime::parse_from_rfc3339(&session.expires_at).ok()?;
    if expires_at < Utc::now() {
        // Clean up expired session
        let _ = crate::db::delete_session(pool, session_id).await;
        return None;
    }
    
    // For client users, verify the user still exists and is active
    if let Some(ref user_id) = session.user_id {
        if let Ok(Some(user)) = crate::db::get_user_by_id(pool, user_id).await {
            if !user.is_active {
                let _ = crate::db::delete_session(pool, session_id).await;
                return None;
            }
        } else {
            // User no longer exists
            let _ = crate::db::delete_session(pool, session_id).await;
            return None;
        }
    }
    
    Some(session)
}

/// Authenticate admin from environment variables
pub fn authenticate_admin(username: &str, password: &str) -> bool {
    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_password = std::env::var("ADMIN_PASSWORD").ok();
    
    // Require ADMIN_PASSWORD to be set
    match admin_password {
        Some(pwd) => {
            let username_match = username.as_bytes().ct_eq(admin_username.as_bytes());
            let password_match = password.as_bytes().ct_eq(pwd.as_bytes());
            (username_match & password_match).into()
        },
        None => false,
    }
}

/// Authenticate a client user from database
pub async fn authenticate_user(pool: &DbPool, username: &str, password: &str) -> Option<User> {
    let user = crate::db::get_user_by_username(pool, username).await.ok()??;
    
    if !user.is_active {
        return None;
    }
    
    if verify_password(password, &user.password_hash) {
        Some(user)
    } else {
        None
    }
}
