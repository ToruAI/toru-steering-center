# Execution Plan: Plugin System Final Push

**Created:** 2026-01-13
**Status:** Ready for Execution
**Target:** Clean build, passing tests, ready for manual validation (V.1-V.12)

---

## Executive Summary

Before manual testing can proceed, we must:
1. Fix 4 security/reliability issues identified in code review
2. Rewrite integration tests to actually test PluginSupervisor methods (not mocks)
3. Clean up dead code (integrate or remove unused methods)
4. Achieve clean build with zero warnings

**Total Estimated Effort:** 6-8 hours

---

## Phase 1: Security Fixes (BLOCKING)

**Priority:** Critical - Must fix before any manual testing
**Owner:** BOB
**Effort:** 2 hours

### Task 1.1: Path Traversal Validation
**File:** `src/routes/plugins.rs` (line ~120)
**Issue:** `forward_to_plugin()` doesn't validate plugin route for traversal attacks

**Current (vulnerable):**
```rust
let (plugin_route, remaining) = path.split_once('/').unwrap_or((&path, ""));
```

**Required Fix:**
```rust
// Validate plugin_route doesn't contain path traversal sequences
if plugin_route.contains("..") || plugin_route.contains('/') {
    return Err(StatusCode::BAD_REQUEST);
}
```

**Attack vectors blocked:**
- `GET /api/plugins/route/../../admin`
- `GET /api/plugins/route/hello-plugin/../other-plugin`

**Definition of Done:**
- [ ] Validation added before `get_plugin_for_route()` call
- [ ] Test added for traversal attempt (returns 400)
- [ ] Manual verification: curl request with `../` returns 400

---

### Task 1.2: Metadata Injection Validation
**File:** `src/services/plugins.rs` (line ~162-188)
**Issue:** `read_plugin_metadata()` trusts all JSON from plugin binary output

**Required Fix:**
```rust
// After parsing metadata
if !metadata.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
    return Err(anyhow::anyhow!("Invalid plugin ID format: must be alphanumeric with hyphens"));
}
if !metadata.route.starts_with('/') || metadata.route.contains("..") {
    return Err(anyhow::anyhow!("Invalid plugin route: must start with / and not contain .."));
}
// Limit metadata field lengths to prevent DoS
if metadata.name.len() > 100 || metadata.author.len() > 100 {
    return Err(anyhow::anyhow!("Metadata field too long (max 100 chars)"));
}
```

**Definition of Done:**
- [ ] Validation added after `serde_json::from_str()`
- [ ] Test added for malicious metadata (returns error)
- [ ] Build succeeds

---

### Task 1.3: Plugin Response Timeout
**File:** `src/services/plugins.rs` (line ~772-776)
**Issue:** No timeout on plugin HTTP response read

**Required Fix:**
```rust
let response_msg = tokio::time::timeout(
    Duration::from_secs(30),
    protocol.read_message(&mut stream)
)
.await
.map_err(|_| anyhow::anyhow!("Plugin response timeout after 30s"))?
.context("Failed to read HTTP response from plugin")?;
```

**Definition of Done:**
- [ ] Timeout added to `forward_http_request()`
- [ ] Configurable via constant (30s default)
- [ ] Build succeeds

---

### Task 1.4: Remove Duplicate EOF Check
**File:** `src/services/plugins.rs` (line ~228-251)
**Issue:** Duplicate `Ok(0) => break` in stderr reader match

**Required Fix:**
Remove the duplicate pattern at line ~248. The match arm structure should be:
```rust
match stderr.read(&mut buffer).await {
    Ok(0) => break,  // EOF
    Ok(n) => { /* logging */ }
    Err(_) => break,
}
```

**Definition of Done:**
- [ ] Duplicate `Ok(0)` removed
- [ ] Build succeeds (no unreachable code warning)

---

### Phase 1 Exit Criteria
- [ ] All 4 security/reliability fixes applied
- [ ] `cargo build` succeeds
- [ ] No new warnings introduced
- [ ] Existing tests still pass

---

## Phase 2: Test Rewrite (HIGH PRIORITY)

**Priority:** High - Tests should validate actual PluginSupervisor behavior
**Owner:** BOB
**Effort:** 3 hours
**Depends on:** Phase 1 complete

### Problem Statement

Current integration tests in `tests/plugins_integration.rs` are **mocked tests**, not real integration tests:
- T1-T4 create shell scripts, don't test PluginSupervisor
- T5-T8 test UUID generation directly, not via supervisor
- T12-T15 test concepts, not actual enable/disable behavior
- T18-T19 simulate KV operations, don't use real sockets
- T23 simulates events, doesn't write to actual database

**The tests pass, but they don't test the actual PluginSupervisor implementation.**

### Task 2.1: Create Test Fixtures

Create actual test plugin binary that can be used by real integration tests.

**File to create:** `tests/fixtures/test_plugin.rs` (compile to binary)

```rust
// Minimal plugin that responds to --metadata and socket connections
// Build with: cargo build -p test-plugin --release
```

**Or use existing example plugins:**
- `examples/hello-plugin-rust/` (already built)
- Copy to temp dir for tests

**Definition of Done:**
- [ ] Test plugin binary available for integration tests
- [ ] Plugin responds to --metadata correctly
- [ ] Plugin can handle socket connections

---

### Task 2.2: Rewrite Plugin Loading Tests (T1-T4)

**Current:** Creates shell scripts, runs them directly
**Required:** Use actual PluginSupervisor methods

```rust
#[tokio::test]
async fn test_t1_valid_binary_spawns_successfully() {
    let temp_dir = TempDir::new().unwrap();
    let plugins_dir = temp_dir.path().join("plugins");

    // Copy actual test plugin to temp dir
    copy_test_plugin(&plugins_dir, "test-plugin-1");

    // Create real PluginSupervisor
    let mut supervisor = PluginSupervisor::new(&plugins_dir).await.unwrap();

    // Test actual scan_plugins_directory()
    let plugins = supervisor.scan_plugins_directory().await.unwrap();
    assert_eq!(plugins.len(), 1);

    // Test actual spawn_plugin()
    supervisor.spawn_plugin("test-plugin-1").await.unwrap();
    assert!(supervisor.is_plugin_running("test-plugin-1"));
}
```

**Definition of Done:**
- [ ] T1 tests `spawn_plugin()` with real binary
- [ ] T2 tests `spawn_plugin()` with invalid binary (error handling)
- [ ] T3 tests `scan_plugins_directory()` with empty dir
- [ ] T4 tests `read_plugin_metadata()` with failing binary

---

### Task 2.3: Rewrite Lifecycle Tests (T12-T15)

**Current:** Simulates concepts with local variables
**Required:** Use actual PluginSupervisor enable/disable/restart

```rust
#[tokio::test]
async fn test_t12_enable_plugin_spawns_process() {
    let (supervisor, _temp_dir) = setup_test_supervisor().await;

    // Plugin starts disabled
    assert!(!supervisor.is_plugin_running("test-plugin"));

    // Enable should spawn process
    supervisor.enable_plugin("test-plugin").await.unwrap();
    assert!(supervisor.is_plugin_running("test-plugin"));

    // Route should be registered
    let plugin_id = supervisor.get_plugin_for_route("/test-plugin").await;
    assert!(plugin_id.is_some());
}
```

**Definition of Done:**
- [ ] T12 tests actual `enable_plugin()` spawns process
- [ ] T13 tests actual `disable_plugin()` kills process
- [ ] T14 tests config persistence across supervisor recreation
- [ ] T15 tests actual `restart_plugin_with_backoff()` behavior

---

### Task 2.4: Rewrite KV Tests (T18-T19)

**Current:** Creates shell scripts with named pipes (doesn't work)
**Required:** Use actual KV protocol over Unix sockets

```rust
#[tokio::test]
async fn test_t18_kv_requests_handled_correctly() {
    let (supervisor, _temp_dir) = setup_test_supervisor_with_plugin().await;

    // Send KV set request
    supervisor.forward_kv_request("test-plugin", KvOp::Set {
        key: "test-key".into(),
        value: "test-value".into(),
    }).await.unwrap();

    // Send KV get request
    let result = supervisor.forward_kv_request("test-plugin", KvOp::Get {
        key: "test-key".into(),
    }).await.unwrap();

    assert_eq!(result, Some("test-value".into()));
}
```

**Definition of Done:**
- [ ] T18 tests actual KV set/get via supervisor
- [ ] T19 tests error handling for disconnected socket

---

### Task 2.5: Rewrite Database Event Test (T23)

**Current:** Simulates in-memory Vec
**Required:** Use actual database with plugin_event_log

```rust
#[tokio::test]
async fn test_t23_plugin_events_written_to_database() {
    let temp_db = create_temp_database().await;
    let (supervisor, _temp_dir) = setup_test_supervisor_with_db(&temp_db).await;

    // Enable plugin (should write event)
    supervisor.enable_plugin("test-plugin").await.unwrap();

    // Query database for events
    let events = plugin_event_get_recent(&temp_db, "test-plugin", 10).await.unwrap();
    assert!(events.iter().any(|e| e.event_type == "enabled"));
}
```

**Definition of Done:**
- [ ] T23 tests actual database event logging
- [ ] Uses temp SQLite database
- [ ] Verifies events appear in plugin_events table

---

### Phase 2 Exit Criteria
- [ ] All 15 integration tests rewritten to use PluginSupervisor
- [ ] Tests use actual plugin binaries (not shell scripts with mocked behavior)
- [ ] `cargo test --test plugins` passes
- [ ] Test coverage includes: load, enable, disable, restart, KV, events

---

## Phase 3: Dead Code Cleanup (MEDIUM PRIORITY)

**Priority:** Medium - Clean build before manual testing
**Owner:** BOB
**Effort:** 1 hour
**Depends on:** Phase 2 complete (tests will reveal which methods are needed)

### Current Warnings (11 total)

**From clippy output:**

```
warning: field `log_files` is never read (logging.rs:122)
warning: associated functions `default` and `from_directory` are never used (logging.rs:142,147)
warning: methods `log` and `log_error` are never used (logging.rs:343,354)
warning: fields `restart_counts` and `max_restarts` are never read (plugins.rs:32,36)
warning: methods are never used: check_plugin_health, increment_restart_count,
         get_restart_count, should_disable_plugin, reset_restart_count,
         send_shutdown_message, restart_plugin_with_backoff (plugins.rs:340-851)
warning: needless borrow (plugins.rs:243)
warning: drop with reference does nothing (plugins.rs:554)
```

### Task 3.1: Integrate or Remove Logger Methods

**File:** `src/services/logging.rs`

**Options:**
- **Integrate:** Use `PluginLogger::default()` in `PluginSupervisor::new()`
- **Remove:** Delete if not needed for current scope

**Recommendation:** Integrate. The logging system is built and should be used.

**Changes:**
1. In `PluginSupervisor::new()`, create `PluginLogger::default()`
2. Store in supervisor struct
3. Use `plugin_logger.log_plugin()` in spawn/kill/restart methods

**Definition of Done:**
- [ ] Logger integrated or methods removed
- [ ] No warning for `log_files`, `default`, `from_directory`

---

### Task 3.2: Integrate or Remove Restart Methods

**File:** `src/services/plugins.rs`

**Methods to address:**
- `check_plugin_health()` - Should be used in health endpoint
- `increment_restart_count()` - Should be used in crash recovery
- `get_restart_count()` - Status reporting
- `should_disable_plugin()` - Crash recovery
- `reset_restart_count()` - Successful restart
- `send_shutdown_message()` - Graceful shutdown
- `restart_plugin_with_backoff()` - Crash recovery

**Recommendation:** These are crash recovery methods. Either:
1. **Integrate:** Add crash detection loop that uses these methods
2. **Remove with TODO:** Mark as future work if crash recovery is deferred

**Definition of Done:**
- [ ] Restart methods integrated or removed with TODO comment
- [ ] `restart_counts` and `max_restarts` fields used or removed
- [ ] No dead code warnings for these methods

---

### Task 3.3: Fix Minor Clippy Warnings

1. **Needless borrow (line 243):**
   ```rust
   // Change
   &output.trim()
   // To
   output.trim()
   ```

2. **Drop reference (line 554):**
   ```rust
   // Change
   drop(process);
   // To
   let _ = process;
   // Or just remove the line if lock is released naturally
   ```

**Definition of Done:**
- [ ] `cargo clippy -- -D warnings` passes with zero warnings

---

### Phase 3 Exit Criteria
- [ ] Zero compiler warnings
- [ ] Zero clippy warnings
- [ ] All dead code either integrated or removed
- [ ] Build is clean: `cargo build --release` succeeds

---

## Phase 4: Build Verification (GATE)

**Priority:** Blocking gate before manual testing
**Owner:** BOB
**Effort:** 30 minutes
**Depends on:** Phases 1-3 complete

### Task 4.1: Full Build and Test

```bash
# Clean build
cargo clean

# Build
cargo build --release

# Format check
cargo fmt --check

# Clippy with strict warnings
cargo clippy -- -D warnings

# All tests
cargo test --all

# Integration tests specifically
cargo test --test plugins -- --nocapture
```

**Definition of Done:**
- [ ] All commands succeed with exit code 0
- [ ] No warnings in build output
- [ ] Test summary: all passed, none failed

---

### Task 4.2: Example Plugin Build

```bash
# Build Rust example plugin
cd examples/hello-plugin-rust
./build.sh
ls -la ../../plugins/hello-plugin.binary

# Verify Python example plugin (no build needed)
ls -la examples/hello-plugin-python/main.py
```

**Definition of Done:**
- [ ] Rust plugin binary exists at `plugins/hello-plugin.binary`
- [ ] Python plugin script exists and is executable
- [ ] Both respond to `--metadata` flag

---

### Phase 4 Exit Criteria
- [ ] Clean build with zero warnings
- [ ] All tests pass (unit + integration)
- [ ] Example plugins built and functional
- [ ] Ready for manual validation

---

## Phase 5: Manual Validation (V.1-V.12)

**Priority:** Final validation before merge
**Owner:** Validation (human)
**Effort:** 1-2 hours
**Depends on:** Phase 4 gate passed

### Validation Checklist

| Test | Description | How to Verify |
|------|-------------|---------------|
| V.1 | Build and load Rust example plugin | Server shows plugin in logs on startup |
| V.2 | Build and load Python example plugin | Server shows plugin in logs on startup |
| V.3 | Enable/disable plugin via UI | Toggle works, status updates |
| V.4 | Plugin view renders and responds | Click plugin in sidebar, UI loads |
| V.5 | Plugin KV storage works | Plugin can store/retrieve data |
| V.6 | Plugin appears in sidebar when enabled | After enable, plugin icon shows |
| V.7 | Plugin hidden from sidebar when disabled | After disable, plugin icon gone |
| V.8 | Server starts with no plugins | Empty `./plugins/` dir, server runs |
| V.9 | Server handles invalid .binary gracefully | Add corrupted file, server ignores |
| V.10 | Plugin crash triggers auto-restart | Kill plugin process, it restarts |
| V.11 | Plugin logs visible in UI | Admin can view plugin logs |
| V.12 | TORIS can read plugin logs | (Deferred - requires TORIS setup) |

### Validation Procedure

1. **Start fresh:**
   ```bash
   rm -rf ./plugins/*
   export ADMIN_PASSWORD="test"
   cargo run --release
   ```

2. **Test empty state (V.8):**
   - Server should start successfully
   - Logs show: "No plugins found in ./plugins/"

3. **Add invalid plugin (V.9):**
   ```bash
   echo "not a real binary" > ./plugins/fake.binary
   # Restart server or trigger rescan
   ```
   - Server should log warning and continue

4. **Add valid plugins (V.1, V.2):**
   ```bash
   cp examples/hello-plugin-rust/target/release/hello-plugin ./plugins/hello-plugin.binary
   cp -r examples/hello-plugin-python ./plugins/hello-python.binary
   chmod +x ./plugins/hello-python.binary
   ```

5. **Test enable/disable (V.3, V.6, V.7):**
   - Open browser to http://localhost:3000
   - Login as admin
   - Navigate to Plugins page
   - Enable hello-plugin
   - Verify it appears in sidebar
   - Disable it
   - Verify it disappears from sidebar

6. **Test plugin view (V.4, V.5):**
   - Enable plugin
   - Click plugin in sidebar
   - Verify plugin UI loads
   - Test plugin functionality (if plugin has interactive elements)

7. **Test crash recovery (V.10):**
   ```bash
   # Find plugin PID
   ps aux | grep hello-plugin
   # Kill it
   kill -9 <PID>
   # Wait 2-5 seconds
   # Check if it restarted
   ps aux | grep hello-plugin
   ```

8. **Test logs (V.11):**
   - Navigate to plugin details
   - View logs tab
   - Verify logs are displayed

---

### Phase 5 Exit Criteria
- [ ] V.1-V.11 all pass
- [ ] V.12 documented as deferred (requires TORIS)
- [ ] No crashes or error dialogs during testing
- [ ] All functionality works as documented

---

## Dependencies Graph

```
Phase 1 (Security)
    |
    v
Phase 2 (Tests) -----> Phase 3 (Dead Code)
                            |
                            v
                       Phase 4 (Build Gate)
                            |
                            v
                       Phase 5 (Manual)
```

**Note:** Phase 3 depends on Phase 2 because test rewrites will reveal which "dead" code is actually needed.

---

## Final Checklist Before Merge

- [ ] Phase 1: All security fixes applied
- [ ] Phase 2: All tests rewritten and passing
- [ ] Phase 3: Zero warnings in build
- [ ] Phase 4: Clean build verified
- [ ] Phase 5: Manual validation V.1-V.11 passed
- [ ] No TODO/FIXME comments left in security-critical code
- [ ] CHANGELOG.md updated with plugin system features
- [ ] PR created with summary of all changes

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Test rewrite takes longer | Medium | Low | Keep existing tests as backup |
| Dead code is actually needed | Low | Medium | Phase 2 tests will reveal |
| Manual tests find new issues | Medium | Medium | Schedule buffer time |
| TORIS integration blocked | High | Low | V.12 explicitly deferred |

---

## Timeline

| Phase | Effort | Cumulative |
|-------|--------|------------|
| Phase 1 | 2h | 2h |
| Phase 2 | 3h | 5h |
| Phase 3 | 1h | 6h |
| Phase 4 | 0.5h | 6.5h |
| Phase 5 | 1.5h | 8h |

**Total:** ~8 hours of focused work (1 full day)

---

**Plan created by GARRY on 2026-01-13**
**Questions? Contact Tako (project maintainer)**
