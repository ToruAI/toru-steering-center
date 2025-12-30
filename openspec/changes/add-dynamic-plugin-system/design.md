# Design: Process-Isolated Plugin System

## Context

Toru Steering Center is an open source VPS control panel. The business model requires:
- Open source core (community can fork/modify)
- Proprietary plugins delivered to paying clients
- True ownership (clients keep everything when they stop paying)
- No vendor lock-in
- Server stability is critical

**Constraints:**
- Single binary deployment model
- <100MB RAM target
- Mostly Linux VPS deployments
- Plugins are trusted (either from maintainer or community-vetted)
- Full plugin capabilities (shell, files, network, DB)

**Stakeholders:**
- End users: Install and use plugins
- Plugin developers: Build plugins in Rust or other languages
- Maintainer: Distribute proprietary plugins to clients
- TORIS: Observability via log watching

## Goals / Non-Goals

**Goals:**
- Crash isolation (one bad plugin shouldn't crash the core)
- Simple plugin loading from directory
- Native Rust performance (no WASM overhead)
- Instance-locked licensing for proprietary plugins
- WordPress-style frontend (one button, one view, full freedom)
- Normal Rust development experience for plugin authors
- Language flexibility (Rust for proprietary, any language for community)
- Observability for TORIS (structured logs, process metrics)

**Non-Goals (MVP):**
- Plugin marketplace / remote installation
- Cross-platform (.dylib, .dll) - Linux only
- Sandboxing / capability security (trust model instead)
- Hot-reload without restart
- Inter-plugin communication
- Resource limiting (cgroups) - nice to have for v2
- Webhook notifications (logs + DB sufficient for now)

## Architecture

```
[Core Process (TSC)]
    │
    ├─ spawns → [Plugin Process 1] (acme-integration)
    │             └─ Unix socket: /tmp/toru-plugins/acme.sock
    │
    ├─ spawns → [Plugin Process 2] (weather-widget)
    │             └─ Unix socket: /tmp/toru-plugins/weather.sock
    │
    └─ monitors → Plugin health, logs, restart on crash
```

## Decisions

### Decision 1: Process Isolation

**Choice:** Each plugin runs as a separate process, communicates via Unix domain sockets

**Rationale:**
- Crash isolation: plugin dies, core survives
- Auto-restart: core can restart failed plugins
- Language flexibility: any language that can do Unix sockets
- No ABI issues: protocol is stable
- Performance: Unix sockets have microsecond overhead (negligible)

**Alternatives considered:**
- Dynamic libraries (.so) - No crash isolation, ABI compatibility issues
- WASM (wasmtime) - Over-engineered for trusted code, sandboxing not needed
- HTTP/Webhooks - Too much overhead (milliseconds vs microseconds)

### Decision 2: Plugin Protocol

**Choice:** JSON messages over Unix sockets

**Message format:**
```json
{
  "type": "http|kv|lifecycle",
  "timestamp": "2025-12-30T12:00:00Z",
  "request_id": "uuid",
  "payload": { ... }
}
```

**Message types:**

**Lifecycle messages:**
```json
// Init
{
  "type": "lifecycle",
  "action": "init",
  "payload": {
    "instance_id": "uuid",
    "plugin_socket": "/tmp/toru-plugins/my-plugin.sock",
    "log_path": "/var/log/toru/plugins/my-plugin.log"
  }
}

// Shutdown
{
  "type": "lifecycle",
  "action": "shutdown"
}
```

**HTTP messages:**
```json
// Request
{
  "type": "http",
  "request_id": "uuid",
  "payload": {
    "method": "POST",
    "path": "/acme/certificate",
    "headers": { "Content-Type": "application/json" },
    "body": "{...}"
  }
}

// Response
{
  "type": "http",
  "request_id": "uuid",
  "payload": {
    "status": 200,
    "headers": { "Content-Type": "application/json" },
    "body": "{\"status\":\"ok\"}"
  }
}
```

**KV messages:**
```json
// Get
{
  "type": "kv",
  "action": "get",
  "key": "my-setting"
}

// Set
{
  "type": "kv",
  "action": "set",
  "key": "my-setting",
  "value": "some-value"
}
```

**Rationale:**
- Simple and debuggable (human-readable JSON)
- serde + tokio for efficient serialization
- Easy to implement in any language

### Decision 3: Plugin API Trait (Rust SDK)

**Choice:** Public `ToruPlugin` trait in separate crate for Rust plugins

```rust
// toru-plugin-api (open source crate)
#[async_trait]
pub trait ToruPlugin {
    fn metadata() -> PluginMetadata;

    async fn init(&mut self, ctx: PluginContext) -> Result<(), PluginError>;

    async fn handle_http(&self, req: HttpRequest) -> Result<HttpResponse, PluginError>;

    async fn handle_kv(&mut self, op: KvOp) -> Result<Option<String>, PluginError>;
}

pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub icon: String,
    pub route: String,
}

pub struct PluginContext {
    pub instance_id: String,
    pub config: PluginConfig,
    pub kv: Box<dyn PluginKvStore>,
}
```

**Plugin binary entrypoint:**
```rust
#[tokio::main]
async fn main() {
    let mut plugin = MyPlugin::new();

    // Read socket path from env var
    let socket_path = std::env::var("TORU_PLUGIN_SOCKET").unwrap_or_else(|_| {
        format!("/tmp/toru-plugins/{}.sock", plugin.metadata().id)
    });

    let listener = UnixListener::bind(&socket_path).unwrap();

    // Handle messages
    for stream in listener.incoming() {
        let msg = read_message(stream).unwrap();
        let response = plugin.handle_message(msg).await;
        write_message(stream, response).unwrap();
    }
}
```

**Rationale:**
- Clean contract for Rust plugin authors
- Separate crate allows independent versioning
- Async trait required for network I/O, DB calls

### Decision 4: Instance-Locked Licensing

**Choice:** HMAC-signed license keys tied to instance UUID

**How it works:**
1. On first run, Toru generates unique instance ID (UUID v4), stored in DB
2. Client sends instance ID to maintainer
3. Maintainer generates signed license key
4. Plugin validates: key signature matches instance ID

**Key format:**
```
base64(instance_id:expiry:hmac_signature)
```

Where:
- `instance_id` - Must match current instance
- `expiry` - "never" or ISO date (e.g., "2025-12-31")
- `hmac_signature` - HMAC-SHA256(instance_id:expiry, SECRET_KEY)

**Rationale:**
- Works offline (no license server dependency)
- True ownership (license works forever once issued)
- Cannot be shared (tied to specific instance)
- Simple implementation (~50 lines)

**Alternative considered:**
- Online license server - Contradicts "no vendor lock-in" goal

### Decision 5: Frontend Mount API

**Choice:** JavaScript bundle with `mount(container, api)` contract

**Plugin frontend contract:**
```javascript
window.ToruPlugins = window.ToruPlugins || {};
window.ToruPlugins["my-plugin"] = {
    mount(container, api) {
        // container: DOM element to render into
        // api: { fetch, navigate, showToast }
        // Plugin has FULL CONTROL here
    },
    unmount(container) {
        // Cleanup when navigating away
    }
};
```

**Core loads plugin:**
1. Fetch `/api/plugins/:id/bundle.js`
2. Inject `<script>` tag
3. Call `window.ToruPlugins[id].mount(container, api)`
4. On navigate away: call `unmount()`

**Rationale:**
- WordPress-style simplicity (one view, full freedom)
- Plugin can use React, Vue, vanilla JS, anything
- No framework lock-in
- Frontend bundle embedded in binary via `include_bytes!`

### Decision 6: Plugin Storage

**Choice:** File-based plugin directory + SQLite metadata

**Directory structure:**
```
./plugins/
├── acme-integration.binary
├── weather-widget.binary
└── .metadata/
    └── config.json      # Enabled/disabled state
```

**Database additions:**
```sql
-- Instance identity
INSERT INTO settings (key, value) VALUES ('instance_id', 'uuid-here');

-- Plugin key-value storage (per-plugin namespace)
CREATE TABLE plugin_kv (
    plugin_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT,
    PRIMARY KEY (plugin_id, key)
);

-- Plugin events (for observability)
CREATE TABLE plugin_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plugin_id TEXT NOT NULL,
    event_type TEXT NOT NULL,  -- started, stopped, crashed, restarted, disabled
    timestamp TEXT NOT NULL,
    details TEXT  -- JSON
);
```

**Rationale:**
- Plugins as files = easy deployment (just copy binary)
- Unix sockets = fast IPC
- Minimal database changes
- KV store for plugin settings/state
- Events table for observability

### Decision 7: Plugin Supervision

**Choice:** Core process monitors and restarts plugins

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

impl PluginSupervisor {
    pub async fn spawn_plugin(&mut self, plugin_id: &str) -> Result<()> {
        let socket_path = format!("/tmp/toru-plugins/{}.sock", plugin_id);
        let child = Command::new(plugin_path)
            .env("TORU_PLUGIN_SOCKET", &socket_path)
            .spawn()?;
        // ...
    }

    pub async fn on_plugin_crash(&mut self, id: &str) {
        // Restart with exponential backoff
        let count = self.restart_counts.entry(id.to_string()).or_insert(0);
        *count += 1;

        if *count > 10 {
            // Too many crashes, disable plugin
            self.disable_plugin(id).await?;
            self.notify_maintainer(id).await?;
        } else {
            let delay = 2u64.pow((*count - 1).min(4)) * 1000; // 1s, 2s, 4s, 8s, 16s
            tokio::time::sleep(Duration::from_millis(delay)).await;
            self.spawn_plugin(id).await?;
        }
    }
}
```

### Decision 8: Logging for TORIS

**Choice:** Structured JSON logs to file

**Log format:**
```json
{
  "timestamp": "2025-12-30T12:00:00Z",
  "level": "info",
  "plugin": "acme",
  "message": "Started"
}

{
  "timestamp": "2025-12-30T12:00:01Z",
  "level": "error",
  "plugin": "acme",
  "message": "Failed to handle request",
  "error": "Invalid domain"
}

{
  "timestamp": "2025-12-30T12:00:05Z",
  "level": "warn",
  "plugin": "acme",
  "message": "Restarted after crash"
}
```

**Log location:**
- `/var/log/toru/plugins/<plugin-id>.log` - Plugin-specific logs
- `/var/log/toru/plugin-supervisor.log` - Core plugin manager logs

**Rationale:**
- Easy for TORIS to watch these files
- Structured JSON = easy to parse and query
- One file per plugin = simple isolation

### Decision 9: Plugin Lifecycle

**States:**
```
[File in ./plugins/] → [Spawned] → [Running]
                             ↓
                        [Crashed] → [Restarting]
                             ↓
                        [Disabled] (after N failures)
```

**On startup:**
1. Scan `./plugins/*.binary`
2. Read metadata from each (via `--metadata` flag or companion file)
3. For enabled plugins:
   - Spawn plugin process
   - Send init message with instance ID
   - Wait for plugin to create socket
   - Register routes
   - Start health monitoring

**API Endpoints:**
- `GET /api/plugins` - List loaded plugins
- `GET /api/plugins/:id` - Get plugin details
- `POST /api/plugins/:id/enable` - Enable plugin (spawn process)
- `POST /api/plugins/:id/disable` - Disable plugin (kill process)
- `GET /api/plugins/:id/bundle.js` - Serve frontend bundle
- `GET /api/plugins/:id/logs` - Get plugin logs (for debugging)

## Risks / Trade-offs

| Risk | Impact | Mitigation |
|------|--------|------------|
| Malicious plugin spawns processes | Medium | Trust model: vet community plugins |
| Plugin socket path conflicts | Low | Use plugin_id in path, clean on startup |
| Plugin consumes too much memory | Medium | Monitor via TORIS, manually restart server |
| Plugin restart loop | Medium | Disable after N failures, notify maintainer |
| License key leaked | Medium | Keys are instance-specific, useless elsewhere |
| Protocol changes break old plugins | Low | Version protocol in message, document breaking changes |

## Migration Plan

Since this replaces the previous WASM-based design (never deployed):
1. Remove all WASM-related code from `plugins` branch
2. Implement new process-isolated system
3. No data migration needed

**Phases:**
1. **Phase 1:** Plugin protocol + toru-plugin-api crate (Rust SDK)
2. **Phase 2:** Plugin supervisor + process management
3. **Phase 3:** Instance ID + licensing
4. **Phase 4:** Unix socket communication
5. **Phase 5:** Frontend plugin container
6. **Phase 6:** Plugin Manager UI
7. **Phase 7:** Observability (logging, events)
8. **Phase 8:** Rust plugin example + Python plugin example
9. **Phase 9:** Documentation

## Open Questions

1. **Plugin configuration:** How do plugins receive configuration?
   - **Decision:** Via `PluginContext.config` which reads from environment variables or config file

2. **Plugin KV isolation:** Should plugins have isolated key-value storage?
   - **Decision:** Yes, `plugin_kv` table with plugin_id namespace

3. **Restart requirement:** Should plugin enable/disable require TSC restart?
   - **Decision:** No - spawn/kill process dynamically

4. **Plugin metadata format:** How does core discover plugin metadata?
   - **Decision:** Plugin binary supports `--metadata` flag returning JSON metadata

5. **Python SDK:** Should we provide a Python package for community plugins?
   - **Decision:** Yes - `toru-plugin-python` package with protocol implementation
