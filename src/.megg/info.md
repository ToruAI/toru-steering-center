---
created: 2025-12-15T09:25:00.585Z
updated: 2025-12-15T09:25:00.585Z
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