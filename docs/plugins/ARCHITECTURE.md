# Toru Plugin System Architecture

Deep dive into the plugin system architecture for AI agents and advanced developers.

## Table of Contents

- [System Overview](#system-overview)
- [Component Diagram](#component-diagram)
- [Data Flow](#data-flow)
- [Plugin Supervisor](#plugin-supervisor)
- [Process Management](#process-management)
- [Security Model](#security-model)
- [Extension Points](#extension-points)
- [Performance Characteristics](#performance-characteristics)
- [Observability](#observability)

## System Overview

The Toru plugin system is designed for:
- **Crash isolation**: Plugin failures don't crash the core
- **Language flexibility**: Rust, Python, or any language with Unix socket support
- **Observability**: Structured logging for TORIS monitoring
- **Simplicity**: No complex IPC, just JSON over Unix sockets
- **Production stability**: Maintainer deploys plugins to client VPS instances

### Design Principles

1. **Process isolation over dynamic libraries**: Prevents core crashes
2. **Trust over sandboxing**: Plugins are vetted, not sandboxed
3. **Async over blocking**: Non-blocking I/O throughout
4. **Structured logs over metrics**: TORIS watches log files
5. **Simple over clever**: JSON over binary protocols

## Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    Toru Steering Center (Core)                  │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                      Axum HTTP Server                      │ │
│  │  - Serves frontend (static files)                          │ │
│  │  - Handles /api/plugins/* routes                           │ │
│  │  - Forwards plugin requests to PluginRouter                │ │
│  └──────────────────────────┬─────────────────────────────────┘ │
│                             │                                    │
│  ┌──────────────────────────▼─────────────────────────────────┐ │
│  │                      Plugin Router                          │ │
│  │  - Maps /api/plugins/:id/* → Plugin process                │ │
│  │  - Correlates requests/responses (request_id)              │ │
│  │  - Enforces timeouts                                       │ │
│  └──────────────────────────┬─────────────────────────────────┘ │
│                             │                                    │
│  ┌──────────────────────────▼─────────────────────────────────┐ │
│  │                    Plugin Supervisor                        │ │
│  │  - Spawns plugin processes                                 │ │
│  │  - Monitors health (via socket keepalive)                  │ │
│  │  - Restarts crashed plugins (exponential backoff)          │ │
│  │  - Logs events to /var/log/toru/plugin-supervisor.log      │ │
│  └──────────────────────────┬─────────────────────────────────┘ │
│                             │                                    │
│  ┌──────────────────────────▼─────────────────────────────────┐ │
│  │                     KV Storage (SQLite)                     │ │
│  │  - plugin_kv table (plugin_id, key, value)                 │ │
│  │  - plugin_events table (lifecycle events)                  │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                  │
└──────────────────────────────┬───────────────────────────────────┘
                               │ Unix Sockets
              ┌────────────────┼────────────────┐
              │                │                │
    ┌─────────▼────────┐  ┌───▼────────┐  ┌───▼────────┐
    │   Plugin A       │  │  Plugin B  │  │  Plugin C  │
    │   (acme.sock)    │  │ (wx.sock)  │  │ (db.sock)  │
    │                  │  │            │  │            │
    │   [Rust Binary]  │  │  [Python]  │  │  [Rust]    │
    │   - Handles HTTP │  │  - Handles │  │  - Handles │
    │   - KV storage   │  │    HTTP    │  │    HTTP    │
    │   - Frontend     │  │  - KV      │  │  - KV      │
    └──────────────────┘  └────────────┘  └────────────┘
              │                │                │
              ▼                ▼                ▼
       /var/log/toru/    /var/log/toru/   /var/log/toru/
       plugins/acme.log  plugins/wx.log   plugins/db.log
```

## Data Flow

### HTTP Request Flow

```
1. User Browser
   │
   │ GET /plugins/acme/certificates
   │
   ▼
2. Axum HTTP Server
   │
   │ Route: /api/plugins/:id/*
   │
   ▼
3. Plugin Router
   │
   │ - Extract plugin_id = "acme"
   │ - Generate request_id = "req-uuid-1234"
   │ - Forward to plugin via Unix socket
   │
   ▼
4. Plugin Process (acme)
   │
   │ - Read message from socket
   │ - Parse JSON
   │ - Handle HTTP request
   │ - Generate response
   │ - Write response to socket
   │
   ▼
5. Plugin Router
   │
   │ - Match request_id
   │ - Return response to HTTP server
   │
   ▼
6. Axum HTTP Server
   │
   │ - Send response to browser
   │
   ▼
7. User Browser
```

**Latency breakdown:**
- HTTP parsing: ~10 μs
- Plugin routing: ~5 μs
- Unix socket write: ~2 μs
- Plugin processing: Variable (depends on plugin logic)
- Unix socket read: ~2 μs
- Response routing: ~5 μs
- HTTP serialization: ~10 μs

**Total overhead: ~34 μs** (excluding plugin business logic)

### KV Operation Flow

```
1. Plugin Code
   │
   │ ctx.kv.get("setting_name")
   │
   ▼
2. Plugin KV Client (in toru-plugin-api)
   │
   │ - Generate request_id
   │ - Send KV get message to core
   │
   ▼
3. Core Plugin Router
   │
   │ - Route to KV storage handler
   │
   ▼
4. SQLite Database
   │
   │ SELECT value FROM plugin_kv
   │ WHERE plugin_id = 'acme' AND key = 'setting_name'
   │
   ▼
5. Core Plugin Router
   │
   │ - Send KV response message to plugin
   │
   ▼
6. Plugin KV Client
   │
   │ - Match request_id
   │ - Return value to plugin code
   │
   ▼
7. Plugin Code
```

**Latency: ~500 μs to 5 ms** (dominated by SQLite, not protocol)

### Plugin Lifecycle Flow

```
┌─ On Core Startup ─────────────────────────────────────────────┐
│                                                                │
│  1. Scan ./plugins/*.binary                                    │
│  2. For each plugin:                                           │
│     - Run: ./plugin.binary --metadata                          │
│     - Parse JSON metadata                                      │
│     - Check if enabled in DB                                   │
│  3. For enabled plugins:                                       │
│     - Spawn plugin process                                     │
│     - Send init message with instance_id                       │
│     - Wait for socket to be created                            │
│     - Register routes in Plugin Router                         │
│     - Start health monitoring                                  │
│                                                                │
└────────────────────────────────────────────────────────────────┘

┌─ During Runtime ──────────────────────────────────────────────┐
│                                                                │
│  Plugin Supervisor runs every 5 seconds:                       │
│  - Check socket connectivity (send ping)                       │
│  - Check process alive (via PID)                               │
│  - If plugin crashed:                                          │
│    → Log crash event                                           │
│    → Increment restart counter                                 │
│    → Wait exponential backoff (1s, 2s, 4s, 8s, 16s)           │
│    → Restart plugin                                            │
│    → If restarts > 10: disable plugin, notify maintainer       │
│                                                                │
└────────────────────────────────────────────────────────────────┘

┌─ On Core Shutdown ────────────────────────────────────────────┐
│                                                                │
│  For each running plugin:                                      │
│  - Send shutdown message                                       │
│  - Wait up to 5 seconds for graceful exit                      │
│  - If still running: SIGTERM                                   │
│  - Wait 2 seconds                                              │
│  - If still running: SIGKILL                                   │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

## Plugin Supervisor

The `PluginSupervisor` is the core component managing plugin processes.

### Responsibilities

1. **Process Management**
   - Spawn plugin processes
   - Track PIDs and sockets
   - Send lifecycle messages (init, shutdown)

2. **Health Monitoring**
   - Periodic health checks (every 5 seconds)
   - Detect crashes
   - Restart failed plugins

3. **Resource Management**
   - Cleanup stale socket files
   - Enforce plugin limits (future)
   - Log resource usage

4. **Event Logging**
   - Record all plugin lifecycle events
   - Structured JSON logs for TORIS

### Data Structures

```rust
pub struct PluginSupervisor {
    /// Active plugin processes
    plugins: HashMap<String, PluginProcess>,

    /// Restart attempt counters
    restart_counts: HashMap<String, u32>,

    /// Plugin metadata cache
    metadata: HashMap<String, PluginMetadata>,

    /// Database handle (for KV storage and events)
    db: Arc<Database>,
}

pub struct PluginProcess {
    /// Plugin unique ID
    id: String,

    /// Process handle
    process: Child,

    /// Unix socket stream
    socket: Option<UnixStream>,

    /// Socket path
    socket_path: PathBuf,

    /// Enabled/disabled state
    enabled: bool,

    /// Last health check timestamp
    last_health_check: Instant,

    /// Crash count in current window
    crash_count: u32,
}
```

### State Machine

```
┌──────────┐
│ Stopped  │
└────┬─────┘
     │ spawn()
     ▼
┌──────────┐
│ Starting │ ────error────┐
└────┬─────┘              │
     │ init sent         │
     ▼                    ▼
┌──────────┐         ┌─────────┐
│ Running  │────────>│ Crashed │
└────┬─────┘ crash   └────┬────┘
     │                     │
     │ health check OK     │ restart()
     │                     │
     │<────────────────────┘
     │
     │ shutdown()
     ▼
┌──────────┐
│ Stopping │
└────┬─────┘
     │ process exited
     ▼
┌──────────┐
│ Stopped  │
└──────────┘
```

### Health Checks

```rust
async fn check_plugin_health(&mut self, plugin_id: &str) -> Result<()> {
    let plugin = self.plugins.get_mut(plugin_id).ok_or(...)?;

    // Check 1: Process alive
    if !plugin.process.try_wait()?.is_none() {
        self.on_plugin_crashed(plugin_id).await?;
        return Err(PluginError::Crashed);
    }

    // Check 2: Socket connectivity
    if let Some(socket) = &mut plugin.socket {
        // Try to send a ping message
        let ping = Message::new_lifecycle("ping", None);
        if protocol.write_message(socket, &ping).await.is_err() {
            self.on_plugin_crashed(plugin_id).await?;
            return Err(PluginError::SocketClosed);
        }
    }

    plugin.last_health_check = Instant::now();
    Ok(())
}
```

### Restart Strategy

```rust
async fn on_plugin_crashed(&mut self, plugin_id: &str) -> Result<()> {
    let count = self.restart_counts
        .entry(plugin_id.to_string())
        .or_insert(0);

    *count += 1;

    // Log crash event
    self.log_event(plugin_id, "crashed", format!("restart_count={}", count)).await?;

    // Too many crashes: disable plugin
    if *count > 10 {
        self.disable_plugin(plugin_id).await?;
        self.notify_maintainer(plugin_id, "plugin_disabled_after_crashes").await?;
        return Ok(());
    }

    // Exponential backoff: 1s, 2s, 4s, 8s, 16s (max)
    let delay_seconds = 2u64.pow((*count - 1).min(4));
    tokio::time::sleep(Duration::from_secs(delay_seconds)).await;

    // Restart plugin
    self.spawn_plugin(plugin_id).await?;

    Ok(())
}
```

## Process Management

### Spawning Plugins

```rust
async fn spawn_plugin(&mut self, plugin_id: &str) -> Result<()> {
    let binary_path = format!("./plugins/{}.binary", plugin_id);
    let socket_path = format!("/tmp/toru-plugins/{}.sock", plugin_id);
    let log_path = format!("/var/log/toru/plugins/{}.log", plugin_id);

    // Clean up old socket if exists
    if Path::new(&socket_path).exists() {
        fs::remove_file(&socket_path)?;
    }

    // Spawn process
    let mut child = Command::new(&binary_path)
        .env("TORU_PLUGIN_SOCKET", &socket_path)
        .env("TORU_PLUGIN_ID", plugin_id)
        .env("TORU_INSTANCE_ID", &self.instance_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Wait for socket to be created (up to 5 seconds)
    let socket_ready = self.wait_for_socket(&socket_path, Duration::from_secs(5)).await?;
    if !socket_ready {
        child.kill().await?;
        return Err(PluginError::SocketTimeout);
    }

    // Connect to socket
    let socket = UnixStream::connect(&socket_path).await?;

    // Send init message
    let init_payload = LifecycleInitPayload {
        instance_id: self.instance_id.clone(),
        plugin_socket: socket_path.clone(),
        log_path: log_path.clone(),
    };
    let init_msg = Message::new_lifecycle("init", Some(init_payload));

    let mut protocol = PluginProtocol::new();
    protocol.write_message(&mut socket, &init_msg).await?;

    // Store plugin process
    let plugin = PluginProcess {
        id: plugin_id.to_string(),
        process: child,
        socket: Some(socket),
        socket_path: PathBuf::from(socket_path),
        enabled: true,
        last_health_check: Instant::now(),
        crash_count: 0,
    };

    self.plugins.insert(plugin_id.to_string(), plugin);
    self.log_event(plugin_id, "started", "").await?;

    Ok(())
}
```

### Environment Variables

Plugins receive:

- `TORU_PLUGIN_SOCKET`: Unix socket path
- `TORU_PLUGIN_ID`: Plugin ID from metadata
- `TORU_INSTANCE_ID`: Unique instance identifier (for licensing)

### Process Lifecycle

```
spawn() → [process starts]
         → [plugin creates socket]
         → [core connects to socket]
         → [core sends init message]
         → [plugin is ready]

shutdown() → [core sends shutdown message]
           → [plugin exits gracefully (5s timeout)]
           → [if not exited: SIGTERM (2s timeout)]
           → [if not exited: SIGKILL]
           → [cleanup socket file]
```

## Security Model

### Trust Model

Toru plugins operate under a **trust model**, not a sandbox model:

- Plugins have full system access (shell, files, network, DB)
- Maintainer vets all proprietary plugins
- Community plugins are reviewed before recommendation
- No capability restrictions (cgroups, seccomp, etc.)

**Rationale:**
- Plugins need full capabilities for their use cases
- Sandboxing adds complexity without real security benefit
- Trust is established through code review, not runtime restrictions

### Attack Surface

| Component | Risk | Mitigation |
|-----------|------|------------|
| Plugin binary | Malicious code | Code review + maintainer builds |
| Unix socket | Local privilege escalation | Socket permissions (600), user isolation |
| JSON protocol | DoS via large messages | Message size limits (1MB max) |
| HTTP requests | Web attacks (XSS, SQLi) | Plugin responsibility to validate inputs |
| KV storage | Plugin data leakage | Namespace isolation (plugin_id prefix) |

### Isolation Boundaries

```
┌──────────────────────────────────────────────┐
│         Linux User: toru                     │
│                                              │
│  ┌──────────────┐  ┌─────────────────────┐  │
│  │ Core Process │  │ Plugin A Process    │  │
│  │  (PID 1000)  │  │  (PID 1001)         │  │
│  └──────────────┘  └─────────────────────┘  │
│         │                    │               │
│         │  Unix Socket       │               │
│         └────────────────────┘               │
│                                              │
│  Same UID, same permissions                  │
│  Isolation via process boundaries only       │
└──────────────────────────────────────────────┘
```

**Future Enhancement (v2):**
- Run each plugin as separate Linux user
- Use cgroups for resource limits
- Add seccomp profiles for syscall filtering

### Audit Trail

All plugin events logged to:
- `/var/log/toru/plugin-supervisor.log` (core events)
- `/var/log/toru/plugins/<plugin-id>.log` (plugin-specific events)
- SQLite `plugin_events` table (for querying)

Log format:
```json
{
  "timestamp": "2025-12-30T12:00:00.000Z",
  "level": "info",
  "plugin_id": "acme",
  "event": "started",
  "details": {}
}
```

## Extension Points

### Adding New Message Types

To add a new message type (e.g., `"websocket"`):

1. **Update protocol spec** (`PROTOCOL.md`)
2. **Add to `MessagePayload` enum** (`toru-plugin-api/src/types.rs`):
   ```rust
   pub enum MessagePayload {
       Lifecycle { ... },
       Http { ... },
       Kv { ... },
       Websocket {  // New
           request_id: String,
           payload: WebsocketPayload,
       },
   }
   ```
3. **Update `ToruPlugin` trait** (if needed):
   ```rust
   #[async_trait]
   pub trait ToruPlugin {
       // Existing methods...
       async fn handle_websocket(&self, msg: WebsocketMessage) -> PluginResult<()>;
   }
   ```
4. **Update plugin router** to handle new message type
5. **Maintain backward compatibility** (old plugins ignore new message type)

### Custom Plugin Storage

Beyond KV storage, plugins can:
- Use SQLite directly (via file path)
- Use external databases (PostgreSQL, Redis)
- Use file system (`/var/lib/toru/plugins/<id>/`)

### Hooks (Future)

Potential extension points:
- **Pre-request hook**: Modify HTTP requests before routing
- **Post-response hook**: Modify HTTP responses after plugin processing
- **Scheduled jobs**: Trigger plugin actions on schedule
- **Event bus**: Pub/sub between plugins

## Performance Characteristics

### Latency

| Operation | Latency | Notes |
|-----------|---------|-------|
| Unix socket write | 1-3 μs | Linux kernel overhead |
| Unix socket read | 1-3 μs | Linux kernel overhead |
| JSON serialize (1KB) | 10-20 μs | serde_json |
| JSON deserialize (1KB) | 15-30 μs | serde_json |
| **Total protocol overhead** | **~50 μs** | **Negligible** |
| Plugin business logic | Variable | Depends on plugin |
| SQLite KV get | 100-500 μs | Database I/O |
| SQLite KV set | 500-5000 μs | Database I/O + fsync |

### Throughput

| Workload | Throughput | Bottleneck |
|----------|------------|------------|
| Small HTTP requests (<1KB) | 50,000 req/s | Plugin logic |
| Large HTTP requests (>10KB) | 10,000 req/s | Network bandwidth |
| KV operations | 2,000 ops/s | SQLite write IOPS |

### Resource Usage

| Resource | Per Plugin | Notes |
|----------|------------|-------|
| Memory (idle) | 2-5 MB | Rust: 2MB, Python: 5MB |
| Memory (active) | 5-50 MB | Depends on workload |
| CPU (idle) | <0.1% | Event-driven |
| CPU (active) | Variable | Depends on workload |
| Disk I/O | <1 MB/s | Logs |

### Scalability

- **Max plugins**: 100+ (limited by system resources, not architecture)
- **Max concurrent requests per plugin**: 1,000+ (async I/O)
- **Max socket connections**: Unlimited (one per core request)

## Observability

### Structured Logging

All logs are JSON-formatted for easy parsing by TORIS.

**Core logs** (`/var/log/toru/plugin-supervisor.log`):
```json
{
  "timestamp": "2025-12-30T12:00:00.000Z",
  "level": "info",
  "component": "plugin_supervisor",
  "message": "Plugin started",
  "plugin_id": "acme",
  "pid": 12345
}
```

**Plugin logs** (`/var/log/toru/plugins/acme.log`):
```json
{
  "timestamp": "2025-12-30T12:00:01.000Z",
  "level": "info",
  "plugin_id": "acme",
  "message": "HTTP request handled",
  "method": "GET",
  "path": "/certificates",
  "duration_ms": 15.3
}
```

### Metrics (via Logs)

TORIS aggregates metrics from logs:
- Plugin uptime
- Request count
- Error rate
- Response latency (p50, p95, p99)
- Crash frequency

### Debugging

**Enable debug logging:**
```bash
RUST_LOG=debug ./steering-center
```

**View plugin stderr:**
```bash
journalctl -f | grep "acme"
```

**Inspect socket:**
```bash
# List plugin sockets
ls -l /tmp/toru-plugins/

# Test socket connectivity
nc -U /tmp/toru-plugins/acme.sock
```

**Test plugin independently:**
```bash
# Run plugin in foreground
TORU_PLUGIN_SOCKET=/tmp/test-acme.sock ./plugins/acme.binary

# Send test message
echo '{"type":"lifecycle","timestamp":"2025-12-30T12:00:00Z","payload":{"action":"init","instance_id":"test"}}' | \
  socat - UNIX-CONNECT:/tmp/test-acme.sock
```

### TORIS Integration

TORIS (observability agent) watches:
- `/var/log/toru/plugin-supervisor.log` (core events)
- `/var/log/toru/plugins/*.log` (plugin events)
- Process metrics via `/proc/<pid>/` (CPU, memory, I/O)

TORIS provides:
- Real-time dashboards
- Alerting (e.g., plugin crash rate > threshold)
- Historical analysis

## Future Enhancements

### Planned (v2)

- [ ] Resource limits (CPU, memory) via cgroups
- [ ] Hot-reload (swap plugin binary without restart)
- [ ] Plugin marketplace (remote installation)
- [ ] Inter-plugin communication (event bus)
- [ ] Webhook notifications (alternative to logs)

### Under Consideration

- [ ] WebAssembly plugins (for stricter sandboxing)
- [ ] gRPC protocol (alternative to JSON)
- [ ] Distributed plugins (run on separate machines)
- [ ] Plugin versioning (multiple versions active)

---

**Feedback?** This architecture is designed to evolve. Open an issue on GitHub or join our Discord to discuss improvements.
