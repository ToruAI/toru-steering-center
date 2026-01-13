---
created: 2025-12-30T13:56:45.272Z
updated: 2026-01-13T22:33:14.766Z
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


## 2025-12-30T15:20:16.530Z
## Phase 6 Completion: Frontend - Plugin Manager

**Completed:** 2025-12-30

**Fixed Issues:**
- Fixed missing closing brace in `handleTogglePlugin` function in `Plugins.tsx` (line 64 was missing `}` after `setTogglingId(null);`)
- Fixed icon name references in `Layout.tsx`: Changed `Plugin2` to `Plug2` (lucide-react naming convention)

**Build Status:** ✅ Successful - TypeScript/Vite build passes

**Files Modified:**
1. `frontend/src/lib/api.ts` - Added plugin API client functions (listPlugins, getPlugin, enablePlugin, disablePlugin, getPluginLogs)
2. `frontend/src/pages/Plugins.tsx` - Plugin management page with cards, toggle switches, logs dialog (admin-only)
3. `frontend/src/pages/PluginView.tsx` - Dynamic plugin container with mount/unmount lifecycle
4. `frontend/src/App.tsx` - Added `/plugins` and `/plugin/:pluginId` routes
5. `frontend/src/components/Layout.tsx` - Sidebar integration showing enabled plugins with health indicators

**Technical Implementation:**
- Admin-only access for plugin management
- Plugin bundles loaded via dynamic script tags (`window.toru_plugin_<id>`)
- Mount/unmount pattern for plugin lifecycle
- Health status badges (healthy/unhealthy/disabled)
- Logs dialog with timestamped entries
- Responsive sidebar and mobile menu integration

**Phase 6 Progress:** 26/26 tasks completed
**Overall Progress:** 92/172 tasks (53.5%)

## 2025-12-31T13:59:40.115Z
# Licensing Extraction Decision (2025-12-31)

## Context
Plugin system proposal (`add-dynamic-plugin-system`) included both core plugin functionality AND licensing. This was too much scope for a single change.

## Action Taken
Extracted licensing into separate proposal `add-plugin-licensing` to:
- Simplify plugin system MVP (remove blocking dependencies)
- Enable faster completion of core plugin features
- Make licensing optional enhancement for future use

## What Moved to `add-plugin-licensing`:
- Instance Identity (UUID v4 generation/persistence)
- Plugin Licensing (HMAC-SHA256 validation)
- License Generator CLI tool (internal)
- License validation in plugin SDKs
- Examples with license validation (Rust + Python)

## What Stayed in `add-dynamic-plugin-system`:
- All phases 1-7: Protocol, supervision, KV, routes, frontend, logging, examples
- Phase 8: Documentation (restored - was accidentally removed)
- 125/140 tasks (Phase 8 in progress)
- 15/15 tests passing

## Validation
Both changes pass `openspec validate --strict`.

## Documentation Note
Phase 8 (Documentation) was initially deleted in error, then restored. Critical docs include:
- toru-plugin-api README ✅
- Python plugin guide (pending)
- Plugin structure and build process (pending)
- Frontend mount API (pending)
- Plugin lifecycle and supervision (pending)
- Protocol specification (pending)
- Plugin manager internals (pending)
- Logging format and TORIS integration (pending)
- Architecture/message flow diagrams (pending)

## Impact
- Plugin system MVP is now smaller and focused
- Licensing can be added later when actually needed
- No blocking dependencies between the two features


## 2026-01-13T17:46:38.238Z
## 2026-01-13 - Phase 2: Integration Tests Rewritten

**Context:** Phase 1 completed plugin supervisor implementation. Phase 2 required rewriting integration tests to actually test the real PluginSupervisor methods instead of mocks.

**Problem:** Current tests in `tests/plugins_integration.rs` were creating shell scripts and testing concepts, not actual PluginSupervisor code paths.

**Implementation:**
1. Created `src/lib.rs` to expose modules (`db`, `services`) for integration testing
2. Rewrote test helpers to use actual PluginSupervisor with temp directories
3. Modified tests to copy real `hello-plugin-rust.binary` for realistic testing
4. Updated tests to call actual methods:
   - T1-T4: `scan_plugins_directory()`, `spawn_plugin()`, `read_plugin_metadata()`
   - T12-T15: `enable_plugin()`, `disable_plugin()`, `is_plugin_enabled()`, restart counter methods
   - T19: `forward_http_request()` with actual error handling
   - T23: `plugin_event_log()` with real database operations

**Key Fixes:**
- Added 500ms delays after spawning plugins to allow socket creation (async process)
- Fixed plugin route assertion (actual route is `/hello-rust` not `/hello-plugin-rust`)
- T23 uses isolated temp database to avoid test interference
- Removed unused imports and variables

**Results:**
- All 15 integration tests pass
- Tests now verify actual PluginSupervisor behavior
- Tests use real plugin binary for authentic integration testing
- Health checks account for async socket creation timing

**Reversible:** Yes - could revert to mocks if needed, but real integration tests are more valuable.


## 2026-01-13T17:57:45.843Z
## 2026-01-13 - Phase 3: Dead Code Cleanup - Warning Resolution Strategy

**Context:** After implementing the plugin system, cargo build showed multiple dead code warnings for planned features that weren't yet integrated.

**Decision:** Use `#[allow(dead_code)]` attributes with TODO comments for all planned features rather than removing functional code.

**Reasoning:**
1. **Code is actually needed** - Most "unused" code is used in tests or will be needed soon:
   - `restart_counts`, `max_restarts` - Used in crash recovery tests
   - `check_plugin_health` - Needed for health check endpoints
   - `send_shutdown_message` - Needed for graceful shutdown
   - `SqliteKvStore` - Needed when plugins require persistent storage
   - Logger methods - Needed for expanded logging features

2. **Preserve working implementations** - All code is production-quality and tested
3. **Clear documentation** - TODO comments indicate where/how to integrate
4. **Build hygiene** - Achieved zero warnings without losing functionality

**Implementation:**
- Added `#[allow(dead_code)]` to 13 methods/fields/structs
- Fixed drop-reference bug (changed `drop(process)` to `let _ = process`)
- All tests pass (23 tests total: 8 unit + 15 integration)
- Zero compiler warnings achieved

**Files Modified:**
- `/src/services/logging.rs` - 5 methods/fields marked
- `/src/services/kv_store.rs` - 3 items marked  
- `/src/services/plugins.rs` - 7 methods/fields marked, 1 bug fixed

**Reversible:** Yes - Remove `#[allow(dead_code)]` as features get integrated

## 2026-01-13T19:15:16.357Z
## 2026-01-13 - Created Comprehensive Plugin System Documentation

**Context:** The plugin system has been implemented but lacked comprehensive documentation for developers building plugins and for AI agents understanding the system architecture.

**Decision:** Created three-tier documentation structure:
1. `docs/plugins/README.md` - User-facing guide for plugin developers (1,043 lines)
2. `docs/plugins/PROTOCOL.md` - Complete protocol specification (708 lines)
3. `docs/plugins/ARCHITECTURE.md` - Deep technical architecture (727 lines)

**Content Highlights:**

**README.md:**
- Quick start examples (Rust and Python in 5 minutes)
- Complete plugin implementation guides
- Frontend development with mount/unmount API
- Deployment instructions
- Troubleshooting section

**PROTOCOL.md:**
- Wire format specification (4-byte length prefix + JSON)
- All message types with examples (Lifecycle, HTTP, KV)
- Request-response flow documentation
- Error handling patterns
- Performance characteristics

**ARCHITECTURE.md:**
- System component diagram
- Data flow visualization
- Plugin supervisor internals
- Process management details
- Security model and trust boundaries
- Extension points for future development
- Observability and TORIS integration

**Reasoning:** 
- Documentation is critical for ecosystem growth
- AI agents need structured documentation to assist users
- Human developers need clear examples and reference material
- PROTOCOL.md enables third-party implementations in any language
- ARCHITECTURE.md helps maintainers and contributors understand system design

**Accuracy:** All examples verified against current implementation in:
- `toru-plugin-api/src/`
- `examples/hello-plugin-rust/`
- `examples/hello-plugin-python/`
- `openspec/changes/add-dynamic-plugin-system/design.md`

**Total:** 2,478 lines of comprehensive documentation

## 2026-01-13T22:33:14.767Z
## 2026-01-13 - Plugin System Merged to Main

**Context:** Completed the dynamic plugin system implementation and merged PR #1 to main.

### PR Review Process
- **Bob (Technical Builder)** performed full code review:
  - All 31 tests passing
  - Zero compiler warnings
  - Security fixes verified (symlink protection, DoS prevention, path traversal)
  - Frontend builds successfully
  - 69KB of comprehensive documentation

- **Garry (Strategic Director)** performed strategic review:
  - No breaking changes (purely additive)
  - All 8 phases complete
  - Clean commit history (24 commits)
  - GO recommendation for merge

### Pre-Merge Cleanup
Fixed minor issues identified in review:
1. Removed `console.log` debug statements from `frontend/src/hooks/useSystemStats.ts`
2. Removed outdated `#[allow(dead_code)]` annotations for functions that ARE used:
   - `plugin_kv_get`, `plugin_kv_set`, `plugin_kv_delete` in `src/db.rs`
   - `plugin_event_log` in `src/db.rs`
3. Ran `npm audit fix` - 0 vulnerabilities remaining
4. Archived OpenSpec change to `openspec/changes/archive/add-dynamic-plugin-system/`
5. Removed development artifacts (`code-review-plugin-system.md`, etc.)

### Post-Merge Updates
- Updated `README.md` with plugin system documentation:
  - Added Features section highlighting plugin system
  - Added Tech Stack entry for plugins
  - Updated Project Structure with new directories
  - Added plugin API endpoints to API table
  - Added Plugin System section with quick start examples
  - Link to `docs/plugins/README.md` for full guide

### Commits to Main
- `5647025` - Merge PR #1 (plugin system)
- `be5f2e8` - chore: Clean up after plugin system completion
- `2ba0a06` - docs: Update README with plugin system documentation
- `47b7b23` - fix: Move archived change to correct openspec/changes/archive/ location

### Current State
- Plugin system fully merged and documented
- OpenSpec change properly archived
- `add-plugin-licensing` remains as future work (0/87 tasks)
- All validation passes