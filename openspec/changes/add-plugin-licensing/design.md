# Design: Plugin Licensing

## Context

Toru Steering Center's business model requires:
- Open source core with proprietary plugins
- Plugins delivered as compiled binaries to paying clients
- Instance-locked licensing (one key per deployment)
- Offline operation (no license server)

**Constraints:**
- Plugins are trusted (vetted by maintainer or community)
- No internet access guaranteed on all deployments
- Clients should keep everything when they stop paying
- Simple to implement and maintain

**Stakeholders:**
- Maintainer: Generates license keys for clients
- End users: Install and use plugins
- Plugin developers: Add validation to proprietary plugins

## Goals / Non-Goals

**Goals:**
- Instance identity (unique ID per deployment)
- HMAC-SHA256 license signing
- Offline validation (no network required)
- Expiry support (date-based or "never")
- Instance locking (keys useless elsewhere)
- Simple generation (~30 lines)
- Simple validation (~50 lines)

**Non-Goals:**
- Online license server
- License revocation
- License marketplace
- Subscription management
- Feature flags tied to licenses

## Architecture

```
[Server Starts] → [Generate/Load Instance ID (UUID)]
                          ↓
                   [Plugin Init Message]
                          ↓
[Plugin Receives instance_id + License Key]
                          ↓
              [Validate: HMAC-SHA256(key, SECRET)]
                          ↓
               [Valid?] → Run
               [Invalid?] → Exit with error
```

## Decisions

### Decision 1: Instance Identity

**Choice:** UUID v4 stored in database settings

**Implementation:**
```rust
// On first run
let instance_id = Uuid::new_v4().to_string();
db::set_setting("instance_id", &instance_id).await?;

// On subsequent runs
let instance_id = db::get_setting("instance_id").await?
    .unwrap_or_else(|| Uuid::new_v4().to_string());
```

**Storage:**
```sql
INSERT INTO settings (key, value)
VALUES ('instance_id', '550e8400-e29b-41d4-a716-446655440000');
```

**Rationale:**
- Standard UUID format (RFC 4122)
- Collision-free for all practical purposes
- Easy to share with clients (via email/chat)
- Persistent across server restarts

**Alternative considered:**
- MAC address binding - Too fragile, can change
- Machine ID from systemd - Linux-only, not always available

### Decision 2: License Key Format

**Choice:** `base64(instance_id:expiry:hmac_signature)`

**Example:**
```
NTU1MmE0MzBlLTQxZDQtYTcxNi00NDY2NTU0NDAwMDAwOjIwMjYtMTItMzE6YjAyMjZhZjFmNDk3OWRjZTVjYTQ3ZmFkNGU4MTc5MTEyYzVmMDM3ZDIwYjI=
```

Where:
- `instance_id` - UUID from database (e.g., "550e8400-e29b-41d4-a716-446655440000")
- `expiry` - ISO date "2026-12-31" or "never"
- `hmac_signature` - HMAC-SHA256 of `instance_id:expiry` with secret key

**Secret key:** Environment variable `TORU_LICENSE_SECRET` (maintainer only)

**Rationale:**
- Base64 encoding hides structure (slight obfuscation)
- Expiry support without server
- Cannot be forged without secret key
- Can be shared via email/chat easily

**Alternative considered:**
- JWT tokens - Over-engineered, requires libraries
- RSA signatures - More complex, no clear benefit

### Decision 3: HMAC-SHA256 Signing

**Choice:** HMAC-SHA256 with shared secret

**Generator (internal CLI):**
```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::prelude::*;

type HmacSha256 = Hmac<Sha256>;

fn generate_license(instance_id: &str, expiry: &str, secret: &str) -> String {
    let payload = format!("{}:{}", instance_id, expiry);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(payload.as_bytes());
    let signature = mac.finalize().into_bytes();
    let encoded_sig = BASE64_STANDARD.encode(signature);
    let full_payload = format!("{}:{}:{}", instance_id, expiry, encoded_sig);
    BASE64_STANDARD.encode(full_payload)
}
```

**Validator (plugin SDK):**
```rust
fn validate_license(key: &str, instance_id: &str, secret: &str) -> Result<(), LicenseError> {
    let decoded = BASE64_STANDARD.decode(key)?;
    let parts = String::from_utf8(decoded)?.split(':').collect::<Vec<_>>();

    if parts.len() != 3 {
        return Err(LicenseError::InvalidFormat);
    }

    let key_instance_id = parts[0];
    let expiry = parts[1];
    let signature = parts[2];

    if key_instance_id != instance_id {
        return Err(LicenseError::InstanceMismatch);
    }

    if expiry != "never" && is_expired(expiry) {
        return Err(LicenseError::Expired);
    }

    let payload = format!("{}:{}", instance_id, expiry);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
    mac.update(payload.as_bytes());
    let expected_signature = mac.finalize().into_bytes();
    let decoded_sig = BASE64_STANDARD.decode(signature)?;

    if !constant_time_eq(&decoded_sig, &expected_signature) {
        return Err(LicenseError::InvalidSignature);
    }

    Ok(())
}
```

**Rationale:**
- Standard cryptographic primitive (HMAC-SHA256)
- Constant-time comparison prevents timing attacks
- Easy to implement in any language (Rust, Python, Go, etc.)
- Secret key never leaves maintainer's machine

### Decision 4: Plugin SDK Integration

**Choice:** Optional validation in `PluginContext::init()`

**SDK addition:**
```rust
pub struct PluginContext {
    pub instance_id: String,  // Added
    pub config: PluginConfig,
    pub kv: Box<dyn PluginKvStore>,
}

#[async_trait]
pub trait ToruPlugin {
    // ...

    async fn init(&mut self, ctx: PluginContext) -> Result<(), PluginError> {
        // Optional: Validate license
        if let Some(license_key) = std::env::var("TORU_LICENSE_KEY").ok() {
            let secret = std::env::var("TORU_LICENSE_SECRET").unwrap();
            validate_license(&license_key, &ctx.instance_id, &secret)?;
        }

        // ... rest of init
        Ok(())
    }
}
```

**Plugin usage:**
```rust
async fn init(&mut self, ctx: PluginContext) -> Result<(), PluginError> {
    // Community plugin: no license check
    println!("Starting plugin...");

    // Proprietary plugin: require license
    let secret = std::env::var("TORU_LICENSE_SECRET")
        .expect("TORU_LICENSE_SECRET required for proprietary plugin");
    let key = std::env::var("TORU_LICENSE_KEY")
        .expect("TORU_LICENSE_KEY required");
    validate_license(&key, &ctx.instance_id, &secret)?;

    println!("License valid, starting plugin...");
    Ok(())
}
```

**Rationale:**
- Optional - community plugins can skip validation
- Clear error message if license is invalid
- Maintainer can compile different versions (licensed vs free)

### Decision 5: License Generator Tool

**Choice:** Internal CLI `tools/license-generator`

**CLI interface:**
```bash
# Generate license for specific instance
cargo run --bin license-generator \
    --instance-id "550e8400-e29b-41d4-a716-446655440000" \
    --expiry "2026-12-31"

# Generate license that never expires
cargo run --bin license-generator \
    --instance-id "550e8400-e29b-41d4-a716-446655440000" \
    --never

# Validate existing license
cargo run --bin license-generator \
    --validate "NTU1MmE0MzBl..."
```

**Environment:**
```bash
export TORU_LICENSE_SECRET="your-secret-key-here"
cargo run --bin license-generator --instance-id "..." --expiry "2026-12-31"
```

**Rationale:**
- Internal tool (not shipped to clients)
- Simple CLI for maintainer
- Can validate generated keys before sending to clients

### Decision 6: Secret Key Management

**Choice:** Environment variable `TORU_LICENSE_SECRET`

**Storage:**
- Maintainer's local machine only
- Never committed to git
- Never shipped to clients
- Rotatable if compromised (re-issue all keys)

**Example:**
```bash
# Maintainer's machine
export TORU_LICENSE_SECRET="super-secret-key-change-me-regularly"
cargo run --bin license-generator --instance-id "..." --expiry "2026-12-31"

# Client's deployment (plugin binary)
export TORU_LICENSE_KEY="NTU1MmE0MzBl..."
# No secret key needed - only for validation
```

**Rationale:**
- Standard practice for secrets
- Easy to rotate
- Never exposed in compiled binaries

## Risks / Trade-offs

| Risk | Impact | Mitigation |
|------|--------|------------|
| Secret key compromised | High | Rotate key, re-issue all licenses |
| Instance ID spoofing | Low | UUID v4 collision probability negligible |
| Key shared by client | Medium | Keys are instance-specific, useless elsewhere |
| Expiry hardcoded in plugin | Low | Plugin can recompile with new expiry |

## Migration Plan

Since this is new functionality (no existing licenses):
1. Add instance_id generation to main.rs
2. Update PluginContext in toru-plugin-api
3. Create license-generator tool
4. Add validation to example plugins
5. Document license format and generation

**No data migration needed.**

## Open Questions

1. **Secret key rotation:** How to handle if secret is compromised?
   - **Decision:** Manual process - rotate key, re-issue all affected licenses

2. **Plugin bundling:** Should license key be embedded in plugin binary?
   - **Decision:** No - license key provided by client via environment variable, allows re-keying without recompilation

3. **Expiry granularity:** Should expiry support time-of-day?
   - **Decision:** ISO date only (YYYY-MM-DD) for simplicity
