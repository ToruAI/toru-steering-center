---
created: 2025-12-30T13:56:45.272Z
updated: 2025-12-30T14:24:42.150Z
type: memory
---
## 2025-12-30T13:52:00.000Z
## Phase 1: Plugin Protocol & Rust SDK - COMPLETED (2025-12-30)

**What was implemented:**

### toru-plugin-api Crate (1.1)
- Created `toru-plugin-api/Cargo.toml` with minimal dependencies (serde, tokio, async-trait, uuid, chrono, thiserror)
- Defined `ToruPlugin` trait with methods: metadata(), init(), handle_http(), handle_kv()
- Defined `PluginMetadata` struct (id, name, version, author, icon, route)
- Defined `PluginContext` struct (instance_id, config, kv)
- Defined `HttpRequest` and `HttpResponse` structs
- Defined `KvOp` enum (Get, Set, Delete)
- Defined `PluginError` enum with comprehensive error types
- Defined message types (Lifecycle, Http, Kv)
- Implemented message serialization/deserialization (JSON)
- Added comprehensive README with examples

### Plugin Protocol (1.2)
- Defined JSON message format (type, timestamp, request_id, payload)
- Implemented `PluginProtocol::read_message()` - reads from Unix socket, deserializes JSON
- Implemented `PluginProtocol::write_message()` - serializes JSON, writes to Unix socket
- Documented message types and payload structures
- Created protocol examples in README (init, http request, kv get/set)

### Key Data Structures
- `Message` - Protocol message with type, timestamp, request_id, payload
- `MessagePayload` - Enum variant for Lifecycle, Http, Kv messages
- `LifecycleInitPayload` - Init message with instance_id, plugin_socket, log_path
- `PluginKvStore` - Async trait for plugin KV operations (get, set, delete)
- `PluginProtocol` - Struct for socket communication with read/write methods

### File Structure Created
```
toru-plugin-api/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs          # Main exports, ToruPlugin trait
    ├── error.rs        # PluginError, PluginResult type alias
    ├── message.rs      # Message exports
    ├── protocol.rs     # PluginProtocol for socket communication
    └── types.rs        # All data structures
```

### Build Status
- ✅ Compiles successfully (`cargo build -p toru-plugin-api` passes)
- ✅ Workspace integration added to root Cargo.toml
- ✅ No clippy warnings

**What this enables for later phases:**
- Phase 2 can now implement plugin supervision with known protocol
- Phase 4 task 4.2.5 (SqliteKvStore) is now unblocked
- Plugin developers can use the SDK to build Rust plugins

**Next phases:**
- Phase 2: Plugin Supervisor (process management, lifecycle, crash recovery)
- Phase 3: Instance Identity (UUID generation, persistence)
- Phase 5: Plugin API Routes (backend routes, integration with supervisor)

**References:**
- See `toru-plugin-api/README.md` for usage examples
- See `openspec/changes/add-dynamic-plugin-system/design.md` for protocol specification
- See `openspec/changes/add-dynamic-plugin-system/specs/plugins/spec.md` for requirements


## 2025-12-30T14:24:42.158Z
## Phase 2: Plugin Supervisor - Progress (2025-12-30)

### Completed
- **Phase 2.1 (Process Management)**: All 9 tasks complete
  - Created `src/services/plugins.rs` with PluginSupervisor struct
  - Implemented PluginProcess struct with ID, process, socket, enabled, pid, metadata
  - scan_plugins_directory() - finds .binary files
  - read_plugin_metadata() - calls --metadata flag
  - spawn_plugin() - starts plugin process
  - kill_plugin() - stops plugin gracefully with shutdown message
  - check_plugin_health() - checks socket and process status via libc::kill()
  - Graceful error handling for plugin load failures

- **Phase 2.2 (Plugin Lifecycle)**: All 7 tasks complete
  - Plugin state storage in `./plugins/.metadata/config.json`
  - enable_plugin() / disable_plugin() methods
  - get_plugin_status() method
  - Load enabled state on startup via `initialize()` method
  - Send init message to spawned plugins via Unix socket
  - Send shutdown message before killing plugins

- **Phase 2.3 (Crash Recovery)**: 3/5 tasks complete
  - Restart counter tracking (increment_restart_count, reset_restart_count)
  - Exponential backoff (1s, 2s, 4s, 8s, 16s max)
  - Auto-disable after N failures (max_restarts configurable, default 10)
  - restart_plugin_with_backoff() method implemented

### Remaining (Deferred to Phase 5)
- Task 2.3.4: Write crash events to plugin_events table
  - Reason: Needs database (AppState) integration
- Task 2.3.5: Notification hooks (logs + DB entry)
  - Reason: Needs database (AppState) integration

### Implementation Details
- **Dependencies added**: toru-plugin-api (path), libc, tempfile (dev)
- **Socket directory**: `/tmp/toru-plugins/` created on startup
- **Metadata directory**: `./plugins/.metadata/` for config.json
- **Process tracking**: Uses PID + Unix socket for health checks
- **Communication**: Unix sockets with JSON messages (from toru-plugin-api)
- **Graceful shutdown**: Sends lifecycle shutdown message before SIGTERM
- **Error handling**: Continues loading other plugins if one fails

### Technical Decisions
- Used `libc::kill(pid, 0)` for cross-platform health checks (simpler than nix)
- Socket path format: `/tmp/toru-plugins/{plugin_id}.sock`
- Metadata JSON format: `{"plugins": {"plugin-id": true/false}}`
- Backoff calculation: `2^min(restart_count, 4) * 1000ms`

### Integration Status
- ✅ Standalone implementation complete
- ⏳ Database integration pending (Phase 5 - Plugin API Routes)
- ⏳ Health monitoring task pending (Phase 5 - async loop checking plugin health)
