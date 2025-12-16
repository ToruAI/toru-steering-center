---
created: 2025-12-15T09:25:00.585Z
updated: 2025-12-16T12:03:16.833Z
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
- `services/` - System monitoring, script execution

## Patterns
- AppState holds shared resources (db, system monitor)
- WebSocket handler spawns child processes for scripts
- Task cancellation via signal channels
- All routes under /api prefix
- SPA fallback serves index.html for client-side routing

## Database Tables
- settings: Key-value config (scripts_dir, etc.)
- task_history: Execution logs with timestamps, exit codes, output
- quick_actions: User-defined buttons with script paths, icons, order

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

## 2025-12-15T10:20:09.205Z

## Key Implementation Notes

### executor.rs - Task Registry Pattern
```rust
// TaskRegistry uses Arc<Mutex<Option<Child>>> so we can:
// 1. Store child for cancellation
// 2. Access it from streaming task
// 3. Mark as None when killed
pub type TaskRegistry = Arc<Mutex<HashMap<String, Arc<Mutex<Option<Child>>>>>>;
```

### ws.rs - Streaming Flow
1. Spawn child process
2. Take stdout/stderr handles immediately (before registry)
3. Store child in registry (for cancellation)
4. Spawn streaming task with handles
5. On completion: get child from registry, wait(), clean up

### main.rs - Environment Variables
- `STEERING_HOST`: Bind address (default: 127.0.0.1)
- `STEERING_PORT`: Port (default: 3000)
- `RUST_LOG`: Log level filter


## 2025-12-16T12:03:16.833Z
## Database Tables
- settings: Key-value config
- task_history: Execution logs
- quick_actions: User-defined scripts
- **users**: Client credentials (Argon2 hashes)
- **sessions**: Active web sessions (server-side)
- **login_attempts**: Rate limiting and security audit

## WebSocket Protocol
- **Authentication**: Usage requires valid Session cookie (checked on handshake & periodically).
- **Permissions**:
  - `run`: Admins (Any script), Clients (Only Registered Quick Actions).
  - `cancel`: Own tasks.

## Environment Variables
- `ADMIN_USERNAME`: Admin username (default: "admin")
- `ADMIN_PASSWORD`: **REQUIRED** Admin password
- `PRODUCTION`: Enable Secure cookies (1/true)
- `STEERING_HOST` / `STEERING_PORT`: Network binding