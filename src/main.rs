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
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::Mutex;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::db::init_db;
use crate::routes::api::AppState;
use crate::routes::{
    create_api_router, create_auth_router, create_plugin_router, handle_websocket,
};

#[derive(RustEmbed)]
#[folder = "frontend/dist"]
struct Assets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Parse CLI arguments
    let args: Vec<String> = env::args().collect();
    let (cli_port, cli_host) = parse_args(&args);

    // Show help if requested
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    // Initialize tracing with default level INFO, can be overridden with RUST_LOG env var
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("steering_center=info,tower_http=debug")
            }),
        )
        .init();

    // Check for Secure Cookie capability
    let is_prod = env::var("PRODUCTION")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);
    let force_secure = env::var("SECURE_COOKIES")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);

    if !is_prod && !force_secure {
        tracing::warn!("Running without PRODUCTION/SECURE_COOKIES=true - Cookies will NOT be marked Secure (OK for localhost)");
    } else {
        tracing::info!("Secure cookies ENABLED");
    }

    // Initialize database
    let db = init_db()?;
    tracing::info!("Database initialized");

    // Get or create instance ID
    let instance_id = crate::db::get_or_create_instance_id(&db).await?;
    tracing::info!("Instance ID: {}", instance_id);

    // Initialize plugin supervisor
    let log_dir = env::var("TORU_LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./logs"));
    let supervisor = match crate::services::plugins::PluginSupervisor::new(
        "./plugins",
        10, // max 10 consecutive restarts before disabling
        instance_id.clone(),
        log_dir,
    ) {
        Ok(s) => {
            let sup = Arc::new(Mutex::new(s));
            // Initialize and start plugin supervision
            {
                let mut guard = sup.lock().await;
                match guard.initialize().await {
                    Ok(initialized) => {
                        tracing::info!(
                            "Plugin supervisor initialized with {} plugins",
                            initialized
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize plugins: {}", e);
                    }
                }
            }
            Some(sup)
        }
        Err(e) => {
            tracing::warn!("Failed to initialize plugin supervisor: {}", e);
            None
        }
    };

    // Clean up expired sessions and old login attempts on startup
    if let Err(e) = crate::db::cleanup_expired_sessions(&db).await {
        tracing::warn!("Failed to cleanup expired sessions: {}", e);
    }
    if let Err(e) = crate::db::cleanup_old_login_attempts(&db).await {
        tracing::warn!("Failed to cleanup old login attempts: {}", e);
    }
    if let Err(e) = crate::db::cleanup_old_plugin_events(&db).await {
        tracing::warn!("Failed to cleanup old plugin events: {}", e);
    }
    tracing::info!("Session cleanup completed");

    // Initialize system monitor
    let sys = Arc::new(Mutex::new(System::new_all()));

    // Create app state
    let state = AppState {
        db: db.clone(),
        sys,
        supervisor,
    };

    // Spawn background task to clean up expired sessions daily
    let db_cleanup = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(24 * 60 * 60)); // 24 hours
        loop {
            interval.tick().await; // Wait for next tick

            // Skip first tick if we want (interval.tick() completes immediately first time in some versions,
            // but since we just ran cleanup in main, effectively we wait 24h)

            tracing::info!("Running daily session cleanup");
            if let Err(e) = crate::db::cleanup_expired_sessions(&db_cleanup).await {
                tracing::warn!("Failed to cleanup expired sessions: {}", e);
            }
            if let Err(e) = crate::db::cleanup_old_login_attempts(&db_cleanup).await {
                tracing::warn!("Failed to cleanup old login attempts: {}", e);
            }
            if let Err(e) = crate::db::cleanup_old_plugin_events(&db_cleanup).await {
                tracing::warn!("Failed to cleanup old plugin events: {}", e);
            }
        }
    });

    // Create API router
    let api_router = create_api_router();
    let auth_router = create_auth_router();
    let plugin_router = create_plugin_router();

    // Create main router
    let app = Router::new()
        .route("/api/ws", get(handle_websocket))
        .nest("/api/auth", auth_router)
        .nest("/api/plugins", plugin_router)
        .nest("/api", api_router)
        .fallback(static_handler)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    // Priority: CLI args > env vars > defaults
    // Bind to localhost only by default - use Cloudflare Tunnel or reverse proxy for external access
    let host: [u8; 4] = cli_host
        .or_else(|| env::var("STEERING_HOST").ok())
        .and_then(|h| parse_host(&h))
        .unwrap_or([127, 0, 0, 1]);

    let port: u16 = cli_port
        .or_else(|| env::var("STEERING_PORT").ok().and_then(|p| p.parse().ok()))
        .unwrap_or(3000);

    let addr = SocketAddr::from((host, port));
    tracing::info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn parse_args(args: &[String]) -> (Option<u16>, Option<String>) {
    let mut port = None;
    let mut host = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-p" | "--port" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            "-H" | "--host" => {
                if i + 1 < args.len() {
                    host = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            arg if arg.starts_with("--port=") => {
                port = arg.trim_start_matches("--port=").parse().ok();
            }
            arg if arg.starts_with("--host=") => {
                host = Some(arg.trim_start_matches("--host=").to_string());
            }
            _ => {}
        }
        i += 1;
    }

    (port, host)
}

fn parse_host(h: &str) -> Option<[u8; 4]> {
    let parts: Vec<&str> = h.split('.').collect();
    if parts.len() == 4 {
        let octets: Result<Vec<u8>, _> = parts.iter().map(|p| p.parse()).collect();
        if let Ok(o) = octets {
            return Some([o[0], o[1], o[2], o[3]]);
        }
    }
    None
}

fn print_help() {
    println!("Steering Center - Control center for your digital assets");
    println!();
    println!("USAGE:");
    println!("    steering-center [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -p, --port <PORT>    Port to listen on [default: 3000]");
    println!("    -H, --host <HOST>    Host to bind to [default: 127.0.0.1]");
    println!("    -h, --help           Print this help message");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!("    STEERING_PORT        Port to listen on");
    println!("    STEERING_HOST        Host to bind to");
    println!("    RUST_LOG             Log level (e.g., debug, info, warn, error)");
    println!("    TORU_LOG_DIR         Directory for plugin logs [default: ./logs]");
    println!("    PRODUCTION           Set to 'true' for production mode");
    println!("    SECURE_COOKIES       Set to 'true' to mark cookies as Secure");
    println!();
    println!("EXAMPLES:");
    println!("    steering-center                    # Start on localhost:3000");
    println!("    steering-center -p 8080            # Start on localhost:8080");
    println!("    steering-center --host 0.0.0.0     # Bind to all interfaces");
    println!();
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
