# Design: Dynamic Library Plugin System

## Context

Toru Steering Center is an open source VPS control panel. The business model requires:
- Open source core (community can fork/modify)
- Proprietary plugins delivered to paying clients
- True ownership (clients keep everything when they stop paying)
- No vendor lock-in

**Constraints:**
- Single binary deployment model
- <100MB RAM target
- Mostly Linux VPS deployments
- Plugins are trusted (either from maintainer or community-vetted)

**Stakeholders:**
- End users: Install and use plugins
- Plugin developers: Build plugins in Rust
- Maintainer: Distribute proprietary plugins to clients

## Goals / Non-Goals

**Goals:**
- Simple plugin loading from directory
- Native Rust performance (no WASM overhead)
- Instance-locked licensing for proprietary plugins
- WordPress-style frontend (one button, one view, full freedom)
- Normal Rust development experience for plugin authors

**Non-Goals (MVP):**
- Plugin marketplace / remote installation
- Cross-platform (.dylib, .dll) - Linux only
- Sandboxing / capability security (trust model instead)
- Hot-reload without restart
- Inter-plugin communication

## Decisions

### Decision 1: Dynamic Libraries (.so files)

**Choice:** Use `libloading` crate to load plugins as dynamic libraries

**Rationale:**
- Native performance (no interpretation layer)
- Compiled binaries protect proprietary source code
- Standard Rust tooling and debugging
- Simple implementation (~150 lines plugin loader)

**Alternatives considered:**
- WASM (wasmtime) - Too complex, sandboxing not needed for trusted code
- Embedded Lua/JS - Different language, performance overhead
- Feature flags - Requires recompilation per client

**Plugin structure:**
```
my-plugin/
├── Cargo.toml
├── src/lib.rs          # Plugin implementation
└── frontend/
    └── dist/bundle.js  # Optional frontend (embedded)
```

**Build output:**
```
libmy_plugin.so         # Deployed to ./plugins/
```

### Decision 2: Plugin API Trait

**Choice:** Public `ToruPlugin` trait in separate crate

```rust
// toru-plugin-api (open source crate)
pub trait ToruPlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn init(&mut self, ctx: PluginContext) -> Result<(), PluginError>;
    fn register_routes(&self) -> Vec<PluginRoute>;
    fn frontend_bundle(&self) -> Option<&'static [u8]>;
    fn settings_schema(&self) -> Option<SettingsSchema>;
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

**Plugin export:**
```rust
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn ToruPlugin {
    Box::into_raw(Box::new(MyPlugin::new()))
}

#[no_mangle]
pub extern "C" fn destroy_plugin(plugin: *mut dyn ToruPlugin) {
    unsafe { drop(Box::from_raw(plugin)); }
}
```

**Rationale:**
- Clean contract between core and plugins
- Separate crate allows independent versioning
- `Send + Sync` required for async Axum handlers

### Decision 3: Instance-Locked Licensing

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
- Hardware ID - Too restrictive for VPS (hardware changes)
- Time-limited - Contradicts "ownership" goal

### Decision 4: Frontend Mount API

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
- Frontend bundle embedded in .so via `include_bytes!`

**Alternative considered:**
- JSON UI spec - Limits plugin flexibility
- iframe sandbox - Adds complexity, not needed for trusted code
- Server-rendered HTML - Less interactive

### Decision 5: Plugin Storage

**Choice:** File-based plugin directory + SQLite metadata

**Directory structure:**
```
./plugins/
├── acme-integration.so
├── weather-widget.so
└── .metadata/
    └── config.json      # Enabled/disabled state
```

**Database additions:**
```sql
-- Instance identity
INSERT INTO settings (key, value) VALUES ('instance_id', 'uuid-here');

-- Plugin key-value storage (optional, per-plugin namespace)
CREATE TABLE plugin_kv (
    plugin_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT,
    PRIMARY KEY (plugin_id, key)
);
```

**Rationale:**
- Plugins as files = easy deployment (just copy .so)
- Minimal database changes
- KV store for plugin settings/state

### Decision 6: Plugin Lifecycle

**States:**
```
[File in ./plugins/] -> [Loaded] -> [Enabled]
                                        |
                                        v
                                   [Disabled]
```

**On startup:**
1. Scan `./plugins/*.so`
2. Load each with `libloading`
3. Call `create_plugin()` to get trait object
4. Call `init(ctx)` for enabled plugins
5. Register routes for enabled plugins

**API Endpoints:**
- `GET /api/plugins` - List loaded plugins
- `POST /api/plugins/:id/enable` - Enable plugin
- `POST /api/plugins/:id/disable` - Disable plugin
- `GET /api/plugins/:id/bundle.js` - Serve frontend bundle

## Risks / Trade-offs

| Risk | Impact | Mitigation |
|------|--------|------------|
| Malicious plugin crashes server | High | Trust model: vet community plugins |
| ABI compatibility | Medium | Version plugin API, document breaking changes |
| License key leaked | Medium | Keys are instance-specific, useless elsewhere |
| Plugin memory leaks | Medium | Monitor memory, restart server periodically |
| Rust version mismatch | Low | Document required Rust version in API crate |

## Migration Plan

Since this replaces the previous WASM-based design (never deployed):
1. Remove all WASM-related code from `plugins` branch
2. Implement new dynamic library system
3. No data migration needed

**Phases:**
1. **Phase 1:** Core plugin loader + API trait
2. **Phase 2:** Instance ID + licensing
3. **Phase 3:** Frontend plugin container
4. **Phase 4:** Plugin Manager UI
5. **Phase 5:** Example plugin + documentation

## Open Questions

1. **Plugin configuration:** How do plugins receive configuration?
   - **Decision:** Via `PluginContext.config` which reads from environment or config file

2. **Plugin KV isolation:** Should plugins have isolated key-value storage?
   - **Decision:** Yes, `plugin_kv` table with plugin_id namespace

3. **Restart requirement:** Should plugin enable/disable require restart?
   - **Decision:** Yes for MVP (simpler), consider hot-reload for v2
