mod db;
mod routes;
mod services;

use axum::{
    http::{header, StatusCode, Uri},
    response::IntoResponse,
    routing::get,
    Router,
};
use rust_embed::RustEmbed;
use std::net::SocketAddr;
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::Mutex;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::db::init_db;
use crate::routes::{create_api_router, handle_websocket};
use crate::routes::api::AppState;

#[derive(RustEmbed)]
#[folder = "frontend/dist"]
struct Assets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing with default level INFO, can be overridden with RUST_LOG env var
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("steering_center=info,tower_http=debug"))
        )
        .init();
    
    // Initialize database
    let db = init_db()?;
    tracing::info!("Database initialized");
    
    // Initialize system monitor
    let sys = Arc::new(Mutex::new(System::new_all()));
    
    // Create app state
    let state = AppState { db, sys };
    
    // Create API router
    let api_router = create_api_router();
    
    // Create main router
    let app = Router::new()
        .route("/api/ws", get(handle_websocket))
        .nest("/api", api_router)
        .fallback(static_handler)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);
    
    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();
    
    if path.is_empty() {
        path = "index.html".to_string();
    }
    
    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            if let Some(content) = Assets::get("index.html") {
                 ([(header::CONTENT_TYPE, "text/html")], content.data).into_response()
            } else {
                 (StatusCode::NOT_FOUND, "404 Not Found").into_response()
            }
        }
    }
}
