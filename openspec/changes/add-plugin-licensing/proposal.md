# Change: Add Plugin Licensing

## Why

Toru Steering Center's plugin system needs optional licensing to support:
1. **Proprietary plugins** - Delivered to paying clients as compiled binaries
2. **Instance-locked keys** - Plugins tied to specific deployments, cannot be shared
3. **Offline validation** - No license server dependency, works without internet
4. **True ownership** - License works forever once issued, no vendor lock-in

Without licensing, proprietary plugins cannot be protected or monetized. Licensing enables the business model while maintaining the open source core and plugin ecosystem.

## What Changes

- **NEW capability:** Instance identity (unique UUID per deployment)
- **NEW API:** License validation via HMAC-SHA256 signatures
- **NEW feature:** License key generation CLI tool (internal)
- **NEW feature:** License validation in plugin SDK
- **NEW examples:** License validation in Rust and Python plugins
- **MODIFIED:** Plugin protocol includes instance_id in init message
- **MODIFIED:** PluginContext struct includes instance_id field

## Impact

- New spec: `plugins` (adds licensing requirements)
- Affected code:
  - `src/main.rs` - instance_id generation on first run
  - `src/db.rs` - add instance_id to settings table
  - `toru-plugin-api` - add PluginContext.instance_id
  - `src/services/plugins.rs` - pass instance_id in init message
  - New internal CLI: `tools/license-generator` for signing keys
  - Example plugins: demonstrate license validation

## Scope

This proposal covers:
- Instance ID generation and persistence (UUID v4)
- License key format: `base64(instance_id:expiry:hmac_signature)`
- HMAC-SHA256 signing with secret key
- License validation in plugins (optional - community plugins can skip)
- License generator tool (internal, not shipped to clients)
- Examples in Rust and Python SDKs
- Expiry support (ISO date or "never")
- Works offline (no network required)

Out of scope:
- License marketplace or distribution
- Online license server (offline by design)
- License revocation (impossible offline)
- Subscription management (manual for now)

## Design Philosophy

- **Offline-first** - No network dependency, reliable operation
- **Instance-locked** - Keys useless outside specific deployment
- **Simple implementation** - ~50 lines for validation, ~30 lines for generation
- **Trust-based** - Plugins can choose to validate or not (community vs proprietary)
- **No vendor lock-in** - Once issued, license works forever
