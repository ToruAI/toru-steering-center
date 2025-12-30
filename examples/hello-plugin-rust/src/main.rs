use std::collections::HashMap;
use std::env;
use toru_plugin_api::{
    PluginContext, PluginError, HttpMessageResponse, HttpRequest, HttpResponse, KvMessageResponse,
    KvOp, Message, PluginMetadata, PluginProtocol, ToruPlugin,
};

struct HelloPlugin {
    ctx: Option<PluginContext>,
}

impl HelloPlugin {
    fn new() -> Self {
        Self { ctx: None }
    }

    fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "hello-plugin-rust".to_string(),
            name: "Hello World (Rust)".to_string(),
            version: "0.1.0".to_string(),
            author: Some("ToruAI".to_string()),
            icon: "🦀".to_string(),
            route: "/hello-rust".to_string(),
        }
    }

    fn get_bundle_js() -> &'static [u8] {
        // This will be replaced with actual frontend bundle
        include_bytes!("../frontend/bundle.js")
    }
}

#[async_trait::async_trait]
impl ToruPlugin for HelloPlugin {
    fn metadata() -> PluginMetadata {
        Self::metadata()
    }

    async fn init(&mut self, ctx: PluginContext) -> Result<(), PluginError> {
        eprintln!("[HelloPlugin] Initializing with instance_id: {}", ctx.instance_id);
        self.ctx = Some(ctx);
        Ok(())
    }

    async fn handle_http(&self, req: HttpRequest) -> Result<HttpResponse, PluginError> {
        eprintln!("[HelloPlugin] HTTP request: {} {}", req.method, req.path);

        // Simple routing
        let (status, body) = if req.path == "/bundle.js" {
            // Serve frontend bundle
            (
                200,
                Some(String::from_utf8_lossy(Self::get_bundle_js()).to_string()),
            )
        } else if req.path == "/" || req.path == "" {
            // Simple JSON response
            let response = serde_json::json!({
                "message": "Hello from Rust plugin!",
                "instance_id": self.ctx.as_ref().map(|c| &c.instance_id).unwrap_or(&"unknown".to_string()),
                "time": chrono::Utc::now().to_rfc3339(),
            });
            (200, Some(serde_json::to_string(&response)?))
        } else {
            (404, Some("Not found".to_string()))
        };

        Ok(HttpResponse {
            status,
            headers: {
                let mut h = HashMap::new();
                if req.path == "/bundle.js" {
                    h.insert("Content-Type".to_string(), "application/javascript".to_string());
                } else {
                    h.insert("Content-Type".to_string(), "application/json".to_string());
                }
                h
            },
            body,
        })
    }

    async fn handle_kv(&mut self, op: KvOp) -> Result<Option<String>, PluginError> {
        eprintln!("[HelloPlugin] KV operation: {:?}", op);

        match op {
            KvOp::Get { key } => {
                // Return a simple value for demonstration
                match key.as_str() {
                    "counter" => Ok(Some("0".to_string())),
                    _ => Ok(None),
                }
            }
            KvOp::Set { key, value } => {
                eprintln!("[HelloPlugin] Setting {} = {}", key, value);
                Ok(None)
            }
            KvOp::Delete { key } => {
                eprintln!("[HelloPlugin] Deleting {}", key);
                Ok(None)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    // Handle --metadata flag
    if args.len() > 1 && args[1] == "--metadata" {
        let metadata = HelloPlugin::metadata();
        println!("{}", serde_json::to_string_pretty(&metadata).unwrap());
        return;
    }

    eprintln!("[HelloPlugin] Starting...");

    // Get socket path from environment or use default
    let plugin_id = HelloPlugin::metadata().id;
    let socket_path = env::var("TORU_PLUGIN_SOCKET").unwrap_or_else(|_| {
        format!("/tmp/toru-plugins/{}.sock", plugin_id)
    });

    eprintln!("[HelloPlugin] Socket path: {}", socket_path);

    // Ensure socket directory exists
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        std::fs::create_dir_all(parent).expect("Failed to create socket directory");
    }

    // Remove socket file if it exists
    if std::path::Path::new(&socket_path).exists() {
        std::fs::remove_file(&socket_path).expect("Failed to remove existing socket");
    }

    // Bind to Unix socket
    let listener = tokio::net::UnixListener::bind(&socket_path)
        .expect("Failed to bind to socket");

    eprintln!("[HelloPlugin] Listening on socket...");

    let mut plugin = HelloPlugin::new();
    let mut protocol = PluginProtocol::new();

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                eprintln!("[HelloPlugin] Connection accepted");

                // Handle messages
                loop {
                    match protocol.read_message(&mut stream).await {
                        Ok(message) => {
                            eprintln!("[HelloPlugin] Received message: {:?}", message.message_type);

                            // Handle message
                            match &message.payload {
                                toru_plugin_api::MessagePayload::Lifecycle { action, .. } => {
                                    if action == "init" {
                                        if let Ok(ctx) = parse_init_payload(&message) {
                                            if let Err(e) = plugin.init(ctx).await {
                                                eprintln!("[HelloPlugin] Init error: {}", e);
                                            }
                                        }
                                    } else if action == "shutdown" {
                                        eprintln!("[HelloPlugin] Shutdown received");
                                        std::process::exit(0);
                                    }
                                }
                                toru_plugin_api::MessagePayload::Http { request_id, payload } => {
                                    match plugin.handle_http(payload.clone()).await {
                                        Ok(http_response) => {
                                            let http_resp = HttpMessageResponse {
                                                status: http_response.status,
                                                headers: http_response.headers,
                                                body: http_response.body,
                                            };
                                            let response_msg = Message::new_http(
                                                request_id.clone(),
                                                toru_plugin_api::types::HttpRequest {
                                                    method: "GET".to_string(),
                                                    path: "".to_string(),
                                                    headers: HashMap::new(),
                                                    body: Some(serde_json::to_string(&http_resp).unwrap()),
                                                },
                                            );
                                            if let Err(e) = protocol.write_message(&mut stream, &response_msg).await {
                                                eprintln!("[HelloPlugin] Failed to write HTTP response: {}", e);
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("[HelloPlugin] Error handling HTTP: {}", e);
                                        }
                                    }
                                }
                                toru_plugin_api::MessagePayload::Kv { request_id, payload } => {
                                    match plugin.handle_kv(payload.clone()).await {
                                        Ok(value) => {
                                            let kv_resp = KvMessageResponse { value };
                                            let response_msg = Message::new_http(
                                                request_id.clone(),
                                                toru_plugin_api::types::HttpRequest {
                                                    method: "GET".to_string(),
                                                    path: "".to_string(),
                                                    headers: HashMap::new(),
                                                    body: Some(serde_json::to_string(&kv_resp).unwrap()),
                                                },
                                            );
                                            if let Err(e) = protocol.write_message(&mut stream, &response_msg).await {
                                                eprintln!("[HelloPlugin] Failed to write KV response: {}", e);
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("[HelloPlugin] Error handling KV: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[HelloPlugin] Failed to read message: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[HelloPlugin] Failed to accept connection: {}", e);
            }
        }
    }
}

fn parse_init_payload(message: &Message) -> Result<PluginContext, PluginError> {
    if let toru_plugin_api::MessagePayload::Lifecycle {
        action: _,
        payload,
    } = &message.payload
    {
        if let Some(init_payload) = payload {
            return Ok(PluginContext {
                instance_id: init_payload.instance_id.clone(),
                config: toru_plugin_api::PluginConfig::default(),
                kv: Box::new(DummyKvStore),
            });
        }
    }
    Err(PluginError::Protocol("No init payload".to_string()))
}

struct DummyKvStore;

#[async_trait::async_trait]
impl toru_plugin_api::PluginKvStore for DummyKvStore {
    async fn get(&self, _key: &str) -> toru_plugin_api::PluginResult<Option<String>> {
        Ok(None)
    }

    async fn set(&self, _key: &str, _value: &str) -> toru_plugin_api::PluginResult<()> {
        Ok(())
    }

    async fn delete(&self, _key: &str) -> toru_plugin_api::PluginResult<()> {
        Ok(())
    }
}
