# Change: Add Process-Isolated Plugin System

## Why

Toru Steering Center needs extensibility to support:
1. **Open source core** - Community can fork and modify
2. **Proprietary plugins** - Delivered to paying clients as compiled binaries
3. **True ownership** - When clients stop paying, everything keeps working
4. **No vendor lock-in** - Clients own their deployment (binaries, not source)
5. **Server stability** - One bad plugin shouldn't crash the entire system

The previous WASM-based design was over-engineered for this use case. Process-isolated plugins provide:
- **Crash isolation** - Plugin failures don't take down the core
- **Language flexibility** - Plugins can be Rust, Python, or any language
- **No ABI headaches** - Protocol-based communication is stable
- **Full capabilities** - Plugins have access to shell, files, network, DB

## What Changes

- **NEW capability:** Process-isolated plugin system (binary executables)
- **NEW API:** Plugin process spawning and supervision
- **NEW API:** Unix domain socket communication with plugins
- **NEW API:** Plugin routes registered dynamically
- **NEW feature:** Instance-locked licensing (plugins tied to specific deployment)
- **NEW feature:** Automatic plugin restart on failure
- **NEW feature:** Structured logging for TORIS observability
- **NEW UI:** Plugin Manager page (list, enable/disable plugins)
- **NEW UI:** Plugin view container (one menu entry, full freedom inside view)
- **NEW crate:** `toru-plugin-api` (Rust plugin SDK)
- **NEW examples:** Rust plugin example, Python plugin example
- **MODIFIED:** Sidebar navigation includes plugin-registered routes
- **MODIFIED:** Settings page may show plugin-contributed sections

## Impact

- New spec: `plugins` (new capability)
- Affected code:
  - `src/main.rs` - Plugin manager initialization
  - `src/services/` - new `plugins.rs` service (process supervision)
  - `Cargo.toml` - add `serde`, `tokio` for socket communication
  - `frontend/src/pages/` - PluginManager page, PluginView container
  - `frontend/src/components/` - Sidebar plugin entries

## Scope (MVP)

This proposal covers the MVP:
- Local plugin loading from `./plugins/*.binary` directory
- Process isolation with Unix domain sockets (microsecond overhead)
- Single view per plugin (one sidebar entry, one route)
- Instance-locked licensing (HMAC-signed keys)
- JS bundle frontend (plugin has full control inside container)
- Auto-restart on plugin crash with configurable backoff
- Structured logging to `/var/log/toru/plugins/` for TORIS
- No plugin marketplace or remote installation
- Linux only (can extend to other platforms later)
- Examples: Rust + Python plugin development

## Design Philosophy

- **Stability through isolation** - Crash isolation, auto-restart
- **Language flexibility** - Rust for you, any language for community
- **Protocol over ABI** - Stable message format, no binary compatibility concerns
- **Full capabilities with trust** - Plugins can execute shell, files, network (vetted code)
- **WordPress-style frontend** - One menu button, one view, full freedom inside
- **Observability first** - Structured logs for TORIS monitoring
