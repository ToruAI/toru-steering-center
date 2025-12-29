# Change: Add Dynamic Library Plugin System

## Why

Toru Steering Center needs extensibility to support:
1. **Open source core** - Community can fork and modify
2. **Proprietary plugins** - Delivered to paying clients as compiled binaries
3. **True ownership** - When clients stop paying, everything keeps working
4. **No vendor lock-in** - Clients own their deployment (binaries, not source)

The previous WASM-based design was over-engineered for this use case. Dynamic libraries (.so files) provide the same binary protection with far less complexity.

## What Changes

- **NEW capability:** Dynamic library plugin system (.so files)
- **NEW API:** Plugin loading from `./plugins/` directory
- **NEW API:** Plugin routes registered dynamically
- **NEW feature:** Instance-locked licensing (plugins tied to specific deployment)
- **NEW UI:** Plugin Manager page (list, enable/disable plugins)
- **NEW UI:** Plugin view container (one menu entry, full freedom inside view)
- **NEW crate:** `toru-plugin-api` (public trait for plugin authors)
- **MODIFIED:** Sidebar navigation includes plugin-registered routes
- **MODIFIED:** Settings page may show plugin-contributed sections

## Impact

- New spec: `plugins` (new capability)
- Affected code:
  - `src/main.rs` - Plugin loader initialization
  - `src/services/` - new `plugins.rs` service
  - `Cargo.toml` - add `libloading` dependency
  - `frontend/src/pages/` - PluginManager page, PluginView container
  - `frontend/src/components/` - Sidebar plugin entries

## Scope (MVP)

This proposal covers the MVP:
- Local plugin loading from `./plugins/*.so` directory
- Single view per plugin (one sidebar entry, one route)
- Instance-locked licensing (HMAC-signed keys)
- JS bundle frontend (plugin has full control inside container)
- No plugin marketplace or remote installation
- Linux only (no cross-platform .so/.dylib/.dll)

## Design Philosophy

- **Simplicity over sandboxing** - Plugins are trusted code (yours or community-vetted)
- **Native performance** - No WASM overhead
- **Normal Rust development** - Standard tooling, debugging, IDE support
- **WordPress-style frontend** - One menu button, one view, full freedom inside
