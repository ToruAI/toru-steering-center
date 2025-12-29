# Tasks: Add Dynamic Library Plugin System

## Phase 1: Plugin API Crate

### 1.1 Create toru-plugin-api crate
- [ ] 1.1.1 Create `toru-plugin-api/Cargo.toml` with minimal dependencies
- [ ] 1.1.2 Define `ToruPlugin` trait with metadata, init, register_routes, frontend_bundle
- [ ] 1.1.3 Define `PluginMetadata` struct (id, name, version, author, icon, route)
- [ ] 1.1.4 Define `PluginContext` struct (instance_id, config, kv)
- [ ] 1.1.5 Define `PluginRoute` struct for registering HTTP handlers
- [ ] 1.1.6 Define `PluginKvStore` trait for key-value storage
- [ ] 1.1.7 Define `PluginError` enum for error handling
- [ ] 1.1.8 Add documentation and examples in README

## Phase 2: Core Plugin Loader

### 2.1 Plugin Loading
- [ ] 2.1.1 Add `libloading` dependency to main Cargo.toml
- [ ] 2.1.2 Create `src/services/plugins.rs` with PluginManager struct
- [ ] 2.1.3 Implement `scan_plugins_directory()` to find .so files
- [ ] 2.1.4 Implement `load_plugin()` using libloading
- [ ] 2.1.5 Implement `create_plugin()` symbol lookup and call
- [ ] 2.1.6 Handle plugin load errors gracefully (log and skip)
- [ ] 2.1.7 Store loaded plugins in PluginManager

### 2.2 Instance Identity
- [ ] 2.2.1 Add `get_or_create_instance_id()` function in db.rs
- [ ] 2.2.2 Generate UUID v4 on first run
- [ ] 2.2.3 Store instance_id in settings table
- [ ] 2.2.4 Pass instance_id to plugins via PluginContext

### 2.3 Plugin Key-Value Storage
- [ ] 2.3.1 Add `plugin_kv` table to database schema
- [ ] 2.3.2 Implement `plugin_kv_get(plugin_id, key)` in db.rs
- [ ] 2.3.3 Implement `plugin_kv_set(plugin_id, key, value)` in db.rs
- [ ] 2.3.4 Implement `plugin_kv_delete(plugin_id, key)` in db.rs
- [ ] 2.3.5 Create SqliteKvStore implementing PluginKvStore trait

### 2.4 Plugin Lifecycle
- [ ] 2.4.1 Create `./plugins/.metadata/config.json` for state storage
- [ ] 2.4.2 Implement `enable_plugin()` in PluginManager
- [ ] 2.4.3 Implement `disable_plugin()` in PluginManager
- [ ] 2.4.4 Implement `get_plugin_status()` in PluginManager
- [ ] 2.4.5 Load enabled state on startup
- [ ] 2.4.6 Call `init()` only for enabled plugins

## Phase 3: Plugin API Routes

### 3.1 Backend Routes
- [ ] 3.1.1 Create `src/routes/plugins.rs`
- [ ] 3.1.2 Implement `GET /api/plugins` - list all plugins
- [ ] 3.1.3 Implement `GET /api/plugins/:id` - get plugin details
- [ ] 3.1.4 Implement `POST /api/plugins/:id/enable` - enable plugin
- [ ] 3.1.5 Implement `POST /api/plugins/:id/disable` - disable plugin
- [ ] 3.1.6 Implement `GET /api/plugins/:id/bundle.js` - serve frontend
- [ ] 3.1.7 Register dynamic plugin routes from enabled plugins
- [ ] 3.1.8 Add auth middleware (require login for all plugin routes)

### 3.2 Integration
- [ ] 3.2.1 Initialize PluginManager in main.rs
- [ ] 3.2.2 Add PluginManager to AppState
- [ ] 3.2.3 Mount plugin routes in router

## Phase 4: Frontend - Plugin Manager

### 4.1 Plugin List Page
- [ ] 4.1.1 Create `frontend/src/pages/Plugins.tsx`
- [ ] 4.1.2 Add API client functions in `lib/api.ts`
- [ ] 4.1.3 Display plugin cards (name, version, status, icon)
- [ ] 4.1.4 Implement enable/disable toggle
- [ ] 4.1.5 Show plugin details on click
- [ ] 4.1.6 Add route to App.tsx
- [ ] 4.1.7 Add "Plugins" entry to sidebar

### 4.2 Plugin View Container
- [ ] 4.2.1 Create `frontend/src/pages/PluginView.tsx`
- [ ] 4.2.2 Load plugin bundle.js dynamically
- [ ] 4.2.3 Call `mount(container, api)` after load
- [ ] 4.2.4 Call `unmount(container)` on navigation away
- [ ] 4.2.5 Provide API object with fetch, navigate, showToast
- [ ] 4.2.6 Add dynamic routes for enabled plugins

### 4.3 Sidebar Integration
- [ ] 4.3.1 Fetch enabled plugins on app load
- [ ] 4.3.2 Add plugin entries to sidebar below system items
- [ ] 4.3.3 Use plugin icon and name from metadata
- [ ] 4.3.4 Hide plugins section when no plugins enabled

## Phase 5: Example Plugin

### 5.1 Hello World Plugin
- [ ] 5.1.1 Create `examples/hello-plugin/` directory
- [ ] 5.1.2 Create Cargo.toml with toru-plugin-api dependency
- [ ] 5.1.3 Implement ToruPlugin trait
- [ ] 5.1.4 Create simple frontend (vanilla JS)
- [ ] 5.1.5 Embed frontend with include_bytes!
- [ ] 5.1.6 Add build script (build.sh)
- [ ] 5.1.7 Test installation and loading

### 5.2 Documentation
- [ ] 5.2.1 Write plugin development guide (toru-plugin-api/README.md)
- [ ] 5.2.2 Document plugin structure and build process
- [ ] 5.2.3 Document frontend mount API
- [ ] 5.2.4 Document licensing (for proprietary plugins)

## Phase 6: Licensing (Optional - for Proprietary Plugins)

### 6.1 License Validation
- [ ] 6.1.1 Create license-generator CLI tool (internal, not shipped)
- [ ] 6.1.2 Implement HMAC-SHA256 signing
- [ ] 6.1.3 Document license key format
- [ ] 6.1.4 Add license validation example to documentation

## Quality Gates

### Per-Phase Checklist
After completing each phase, verify:
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] Critical path tests written and passing
- [ ] Code review for security-sensitive code

### Critical Path Tests (Required)

#### Plugin Loading (Phase 2)
- [ ] T1: Valid .so loads successfully
- [ ] T2: Invalid .so handled gracefully (no crash, logs error)
- [ ] T3: Missing plugins directory created automatically
- [ ] T4: Plugin with missing symbols skipped with error

#### Instance Identity (Phase 2)
- [ ] T5: Instance ID generated on first run
- [ ] T6: Instance ID persists across restarts (same value)
- [ ] T7: Instance ID is valid UUID format

#### Plugin KV Storage (Phase 2)
- [ ] T8: KV set/get works for plugin
- [ ] T9: KV isolation (plugin A can't read plugin B's data)
- [ ] T10: KV persists across restarts

#### Plugin Lifecycle (Phase 2-3)
- [ ] T11: Enable plugin makes routes available
- [ ] T12: Disable plugin returns 404 on routes
- [ ] T13: Enabled state persists across restarts

#### License Validation (Phase 6)
- [ ] T14: Valid license key accepted
- [ ] T15: Invalid signature rejected
- [ ] T16: Wrong instance ID rejected
- [ ] T17: Expired key rejected (if expiry set)

### Code Review Checkpoints
Request AI code review after:
- [ ] R1: Plugin loading implementation (security: loading untrusted .so)
- [ ] R2: License validation (security: HMAC verification)
- [ ] R3: Plugin routes (security: auth middleware)

## Validation (Manual Smoke Tests)

- [ ] V.1 Build and load example plugin
- [ ] V.2 Enable/disable plugin via UI
- [ ] V.3 Plugin view renders and responds to clicks
- [ ] V.4 Plugin KV storage works
- [ ] V.5 Plugin appears in sidebar when enabled
- [ ] V.6 Plugin hidden from sidebar when disabled
- [ ] V.7 Server starts with no plugins (empty directory)
- [ ] V.8 Server handles invalid .so files gracefully

## Dependencies

- Phase 2 depends on Phase 1 (need API crate first)
- Phase 3 depends on Phase 2 (need plugin loader)
- Phase 4 depends on Phase 3 (need API endpoints)
- Phase 5 can start after Phase 2 (to test loader)
- Phase 6 is independent (can be done anytime)

## Parallelization

- Phase 1 + Phase 4.1 (UI skeleton) can start in parallel
- Phase 5 (example) + Phase 4.2-4.3 (plugin view) can run in parallel
- Documentation (5.2) can be written alongside implementation
