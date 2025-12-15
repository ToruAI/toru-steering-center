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
