---
created: 2025-12-15T09:25:00.585Z
updated: 2025-12-30T12:45:00.000Z
type: context
---
# Backend Context

## Stack
- Rust with Axum web framework
- SQLite via rusqlite (bundled)
- Tokio async runtime
- sysinfo for system monitoring

## Structure
- `main.rs` - Server setup, routing, static file serving
- `db.rs` - Database schema and queries
- `routes/` - API endpoints and WebSocket handler
  - `plugins.rs` - Plugin management (planned)
- `services/` - System monitoring, script execution
  - `plugins.rs` - Plugin supervisor (planned)

## Patterns
- AppState holds shared resources (db, system monitor, **plugin supervisor**)
- WebSocket handler spawns child processes for scripts
- Task cancellation via signal channels
- All routes under /api prefix
- SPA fallback serves index.html for client-side routing

## Database Tables
- `settings` - Key-value config (scripts_dir, instance_id)
- `task_history` - Execution logs with timestamps, exit codes, output
- `quick_actions` - User-defined buttons with script paths, icons, order
- `users` - Client credentials (Argon2 hashes)
- `sessions` - Active web sessions (server-side)
- `login_attempts` - Rate limiting and security audit
- **`plugin_kv`** - Plugin key-value storage (planned, per-plugin namespace)
- **`plugin_events`** - Plugin lifecycle events (planned: started, stopped, crashed, restarted, disabled)

## WebSocket Protocol
Client sends:
- `{"type": "run", "script": "name.sh"}`
- `{"type": "cancel", "task_id": "uuid"}`

Server sends:
- `{"type": "started", "task_id": "uuid"}`
- `{"type": "stdout", "data": "..."}`
- `{"type": "stderr", "data": "..."}`
- `{"type": "exit", "code": 0}`
- `{"type": "cancelled"}`
- `{"type": "error", "data": "..."}`

## Environment Variables
- `ADMIN_USERNAME`: Admin username (default: "admin")
- `ADMIN_PASSWORD`: **REQUIRED** Admin password
- `PRODUCTION`: Enable Secure cookies (1/true)
- `STEERING_HOST` / `STEERING_PORT`: Network binding (default: 127.0.0.1:3000)
- `RUST_LOG`: Log level filter

## Plugin System (Planned)

### Architecture
```
[Core Process]
    ├─ spawns → [Plugin Process 1] (acme-integration.binary)
    │             └─ Unix socket: /tmp/toru-plugins/acme.sock
    ├─ spawns → [Plugin Process 2] (weather-widget.binary)
    │             └─ Unix socket: /tmp/toru-plugins/weather.sock
    └─ monitors → Health, logs, restart on crash
```

### Plugin Supervisor (services/plugins.rs)
```rust
pub struct PluginSupervisor {
    plugins: HashMap<String, PluginProcess>,
    restart_counts: HashMap<String, u32>,
}

pub struct PluginProcess {
    id: String,
    process: Child,
    socket: UnixStream,
    enabled: bool,
}

// Key methods:
async fn scan_plugins_directory() -> Vec<String>
async fn read_plugin_metadata(path: &Path) -> PluginMetadata  // --metadata flag
async fn spawn_plugin(id: &str) -> Result<()>
async fn kill_plugin(id: &str) -> Result<()>
async fn check_plugin_health(id: &str) -> HealthStatus
async fn on_plugin_crash(id: &str)
async fn restart_plugin_with_backoff(id: &str)
```

### Plugin Protocol (Unix Socket IPC)
**Message format (JSON):**
```rust
pub struct PluginMessage {
    pub r#type: MessageType,  // Lifecycle, Http, Kv
    pub timestamp: String,
    pub request_id: String,
    pub payload: serde_json::Value,
}

pub enum MessageType {
    Lifecycle { action: LifecycleAction },
    Http { request: HttpRequest },
    Kv { operation: KvOp },
}
```

**Lifecycle messages:**
- `init` - Pass instance_id, socket_path, log_path to plugin
- `shutdown` - Tell plugin to gracefully stop

**HTTP messages:**
- Request: method, path, headers, body
- Response: status, headers, body

**KV messages:**
- `get` - Get key from plugin namespace
- `set` - Store key in plugin namespace
- `delete` - Delete key from plugin namespace

### Plugin Routes (routes/plugins.rs)
```
GET /api/plugins              - List all plugins
GET /api/plugins/:id          - Get plugin details
POST /api/plugins/:id/enable   - Spawn plugin process
POST /api/plugins/:id/disable  - Kill plugin process
GET /api/plugins/:id/bundle.js - Serve frontend bundle
GET /api/plugins/:id/logs     - Get plugin logs

Dynamic plugin routes:
/api/plugins/:id/*            - Forwarded to plugin via Unix socket
```

### Crash Recovery Strategy
1. Detect plugin process death (via tokio process monitoring)
2. Increment restart counter for plugin
3. Log crash event to `plugin_events` table
4. If restart_count < 10:
   - Wait with exponential backoff (1s, 2s, 4s, 8s, 16s)
   - Attempt to restart plugin
5. If restart_count >= 10:
   - Disable plugin
   - Log event
   - Notify maintainer (via logs + DB)

### Instance Identity (db.rs)
```rust
pub fn get_or_create_instance_id(conn: &Connection) -> String {
    // Check settings table for "instance_id"
    // If missing, generate UUID v4
    // Store in settings table
    // Return instance_id
}
```
Passed to plugins via init message for license validation.

### Plugin KV Storage (db.rs)
```rust
// Table: plugin_kv (plugin_id, key, value)
pub fn plugin_kv_get(conn: &Connection, plugin_id: &str, key: &str) -> Option<String>
pub fn plugin_kv_set(conn: &Connection, plugin_id: &str, key: &str, value: &str)
pub fn plugin_kv_delete(conn: &Connection, plugin_id: &str, key: &str)
```
Isolated per-plugin namespace via plugin_id column.

### Logging Strategy
**Plugin logs:**
- Write to `/var/log/toru/plugins/<plugin-id>.log`
- JSON format: `{"timestamp": "...", "level": "info", "plugin": "...", "message": "...", "error": "..."}`
- Plugins write directly (or core can forward via socket)
- TORIS watches these files

**Supervisor logs:**
- Write to `/var/log/toru/plugin-supervisor.log`
- Events: spawned, killed, crashed, restarted, disabled
- JSON format for easy parsing by TORIS

### Security Considerations
- All plugin routes require authentication (session cookie)
- Plugins are trusted code (no sandboxing)
- Plugins can execute shell commands (full system access)
- Plugins can read/write files anywhere
- Plugins can open network connections
- Plugins can access SQLite DB (via KV API or direct queries)

### Dependencies to Add (Cargo.toml)
```toml
[dependencies]
# Already have: axum, tokio, rusqlite, serde, serde_json

# New for plugins:
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
# Note: Unix socket support is in tokio::net::UnixListener / UnixStream
```

### Integration with main.rs
```rust
// At startup:
let plugin_supervisor = Arc::new(RwLock::new(PluginSupervisor::new()));
plugin_supervisor.write().await.initialize().await?;

// Add to AppState:
let app_state = Arc::new(AppState {
    db: Arc::new(db),
    // ... other fields
    plugin_supervisor,
});

// Mount routes:
app.nest("/api/plugins", plugin_routes::router(app_state.clone()));
app.fallback(static_files_handler);
```

### toru-plugin-api Crate (Separate package)
```rust
// Public trait for Rust plugin authors:
#[async_trait]
pub trait ToruPlugin {
    fn metadata() -> PluginMetadata;
    async fn init(&mut self, ctx: PluginContext) -> Result<(), PluginError>;
    async fn handle_http(&self, req: HttpRequest) -> Result<HttpResponse, PluginError>;
    async fn handle_kv(&mut self, op: KvOp) -> Result<Option<String>, PluginError>;
}

// Helper functions:
async fn listen_on_unix_socket(plugin: impl ToruPlugin) -> Result<()>
async fn read_message(stream: UnixStream) -> Result<PluginMessage>
async fn write_message(stream: UnixStream, msg: PluginMessage) -> Result<()>
```

Plugin binary implements trait and calls `listen_on_unix_socket()` in main().
