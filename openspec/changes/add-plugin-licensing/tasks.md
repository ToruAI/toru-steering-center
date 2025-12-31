# Tasks: Add Plugin Licensing

## Phase 1: Instance Identity

### 1.1 Database Schema
- [ ] 1.1.1 Add instance_id field to settings table (handled by INSERT OR IGNORE)
- [ ] 1.1.2 Implement `get_or_create_instance_id()` in db.rs
- [ ] 1.1.3 Generate UUID v4 on first run
- [ ] 1.1.4 Store instance_id in database settings
- [ ] 1.1.5 Retrieve instance_id on subsequent runs

### 1.2 Integration
- [ ] 1.2.1 Call `get_or_create_instance_id()` in main.rs on startup
- [ ] 1.2.2 Pass instance_id to PluginContext

## Phase 2: License Generator Tool

### 2.1 CLI Implementation
- [ ] 2.1.1 Create `tools/license-generator` binary in Cargo.toml
- [ ] 2.1.2 Add dependencies: hmac, sha2, base64, chrono
- [ ] 2.1.3 Implement `generate_license()` function
- [ ] 2.1.4 Implement `validate_license()` function
- [ ] 2.1.5 Add CLI argument parsing (instance-id, expiry, validate)

### 2.2 CLI Interface
- [ ] 2.2.1 Add `--instance-id` argument (required)
- [ ] 2.2.2 Add `--expiry` argument (default: "never")
- [ ] 2.2.3 Add `--never` flag for non-expiring licenses
- [ ] 2.2.4 Add `--validate <key>` flag for validation
- [ ] 2.2.5 Read `TORU_LICENSE_SECRET` from environment

### 2.3 Testing
- [ ] 2.3.1 Test license generation with expiry
- [ ] 2.3.2 Test license generation with `--never` flag
- [ ] 2.3.3 Test license validation (valid key)
- [ ] 2.3.4 Test license validation (invalid signature)
- [ ] 2.3.5 Test license validation (wrong instance ID)

## Phase 3: Plugin SDK Updates

### 3.1 PluginContext Changes
- [ ] 3.1.1 Add `instance_id: String` field to `PluginContext`
- [ ] 3.1.2 Update `PluginContext` struct definition in toru-plugin-api
- [ ] 3.1.3 Update documentation

### 3.2 License Validation Helper
- [ ] 3.2.1 Create `validate_license()` function in toru-plugin-api
- [ ] 3.2.2 Implement HMAC-SHA256 signature verification
- [ ] 3.2.3 Implement expiry checking
- [ ] 3.2.4 Add `LicenseError` enum (InvalidFormat, InvalidSignature, InstanceMismatch, Expired)
- [ ] 3.2.5 Use constant-time comparison for signature verification

### 3.3 Example Usage
- [ ] 3.3.1 Add example to toru-plugin-api README
- [ ] 3.3.2 Document environment variables (TORU_LICENSE_KEY, TORU_LICENSE_SECRET)
- [ ] 3.3.3 Document optional vs required validation

## Phase 4: Core System Updates

### 4.1 Init Message Updates
- [ ] 4.1.1 Add `instance_id` to init message payload
- [ ] 4.1.2 Update lifecycle message format in design docs
- [ ] 4.1.3 Pass instance_id from PluginSupervisor to plugin init

### 4.2 Plugin Supervisor
- [ ] 4.2.1 Retrieve instance_id from database
- [ ] 4.2.2 Include instance_id in init message
- [ ] 4.2.3 Update plugin process spawn code

## Phase 5: Example Plugins

### 5.1 Rust Plugin Example
- [ ] 5.1.1 Add license validation to hello-plugin-rust
- [ ] 5.1.2 Read `TORU_LICENSE_KEY` environment variable
- [ ] 5.1.3 Validate license in `init()` method
- [ ] 5.1.4 Handle license errors gracefully
- [ ] 5.1.5 Test with valid license
- [ ] 5.1.6 Test with invalid license

### 5.2 Python Plugin Example
- [ ] 5.2.1 Add license validation to hello-plugin-python
- [ ] 5.2.2 Read `TORU_LICENSE_KEY` environment variable
- [ ] 5.2.3 Validate license (hmac library)
- [ ] 5.2.4 Handle license errors gracefully
- [ ] 5.2.5 Test with valid license
- [ ] 5.2.6 Test with invalid license

## Phase 6: Documentation

### 6.1 Plugin Development Guide
- [ ] 6.1.1 Document license key format
- [ ] 6.1.2 Document license validation in plugins
- [ ] 6.1.3 Document environment variables
- [ ] 6.1.4 Document license generator usage
- [ ] 6.1.5 Document secret key management

### 6.2 Architecture Documentation
- [ ] 6.2.1 Document instance identity system
- [ ] 6.2.2 Document HMAC-SHA256 signing process
- [ ] 6.2.3 Add license flow diagram

## Quality Gates

### Per-Phase Checklist
After completing each phase, verify:
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] Tests written and passing

### Critical Path Tests (Required)

#### Instance Identity (Phase 1)
- [ ] T1: Instance ID generated on first run
- [ ] T2: Instance ID persists across restarts (same value)
- [ ] T3: Instance ID is valid UUID format
- [ ] T4: Instance ID passed to plugin in init message

#### License Generation (Phase 2)
- [ ] T5: License generator creates valid key with expiry
- [ ] T6: License generator creates valid key with --never flag
- [ ] T7: License generator validates valid key correctly
- [ ] T8: License generator rejects invalid signature

#### License Validation (Phase 3-5)
- [ ] T9: Valid license key accepted
- [ ] T10: Invalid signature rejected
- [ ] T11: Wrong instance ID rejected
- [ ] T12: Expired key rejected (if expiry set)
- [ ] T13: Plugin without license loads normally

### Code Review Checkpoints
Request AI code review after:
- [ ] R1: Instance identity implementation
- [ ] R2: HMAC-SHA256 implementation (security: constant-time comparison)
- [ ] R3: License generator security (secret key handling)

## Validation (Manual Smoke Tests)

- [ ] V.1 Generate license with expiry
- [ ] V.2 Generate license without expiry
- [ ] V.3 Validate generated license (CLI tool)
- [ ] V.4 Test Rust plugin with valid license
- [ ] V.5 Test Rust plugin with invalid license
- [ ] V.6 Test Python plugin with valid license
- [ ] V.7 Test Python plugin with invalid license
- [ ] V.8 Test plugin without license (community)
- [ ] V.9 Verify instance ID persistence across restarts

## Dependencies

- Phase 1 must be complete before Phase 4 (need instance_id)
- Phase 2 can run in parallel with Phase 1 (independent)
- Phase 3 depends on Phase 1 (need PluginContext)
- Phase 4 depends on Phase 1 (need instance_id)
- Phase 5 depends on Phase 3 (need validation helpers)
- Phase 6 can run alongside implementation

## Parallelization

- Phase 1 (Instance Identity) + Phase 2 (License Generator) can start in parallel
- Phase 3 (SDK) + Phase 6 (Documentation) can run in parallel
- Phase 5 (Examples) requires Phase 3
