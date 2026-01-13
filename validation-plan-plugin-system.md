# Plugin System Validation Plan

**Date:** 2026-01-13
**Branch:** plugin2
**Status:** Ready for validation

---

## Current State Assessment

### Build Status
| Component | Status | Notes |
|-----------|--------|-------|
| Backend (Rust) | PASS | `cargo build --release` - 10 warnings (non-critical) |
| Frontend | PASS | `frontend/dist/` exists, built 2024-12-30 |
| Example Plugins | PASS | Both binaries present in `./plugins/` |

### Test Coverage
| Category | Tests | Status |
|----------|-------|--------|
| Unit tests | 8 | PASS |
| Integration tests | 15 | PASS |
| **Total** | **23** | **ALL PASSING** |

### Example Plugins Available
1. **hello-plugin-rust.binary** (1.1MB)
   - Metadata: `{"id": "hello-plugin-rust", "name": "Hello World (Rust)", "version": "0.1.0", "route": "/hello-rust"}`
   - Build: `examples/hello-plugin-rust/build.sh`

2. **hello-plugin-python.binary** (8.7KB)
   - Metadata: `{"id": "hello-plugin-python", "name": "Hello World (Python)", "version": "0.1.0", "route": "/hello-python"}`
   - Build: `examples/hello-plugin-python/build.sh`

### Plugin Config State
Both plugins are marked as **enabled** in `plugins/.metadata/config.json`.

---

## Validation Tests Checklist (V.1-V.12)

### V.1 - Build and load Rust example plugin
**Type:** Manual (requires running server)
**Status:** READY TO TEST

**Steps:**
1. Start server: `ADMIN_PASSWORD=dev cargo run`
2. Login to http://localhost:3000
3. Navigate to Plugins page
4. Verify `Hello World (Rust)` appears with status
5. Enable if not enabled

**Verification:**
- Plugin shows in list
- No errors in server console
- Plugin status shows "healthy"

---

### V.2 - Build and load Python example plugin
**Type:** Manual (requires running server)
**Status:** READY TO TEST

**Steps:**
1. Same as V.1
2. Verify `Hello World (Python)` appears

**Notes:**
- Python plugin requires Python 3 on host
- Uses asyncio for Unix socket communication

---

### V.3 - Enable/disable plugin via UI
**Type:** Manual
**Status:** READY TO TEST

**Steps:**
1. Go to Plugins management page
2. Click toggle to enable a disabled plugin
3. Verify server spawns process (check logs)
4. Click toggle to disable
5. Verify process is killed

**Verification:**
- Toggle changes state
- Backend logs show spawn/kill events
- Routes become available/unavailable

---

### V.4 - Plugin view renders and responds to clicks
**Type:** Manual
**Status:** READY TO TEST

**Steps:**
1. Enable a plugin
2. Click plugin name in sidebar
3. Verify plugin UI renders
4. Interact with plugin (click buttons, etc.)

**Verification:**
- Plugin view container loads
- `bundle.js` loaded from `/api/plugins/:id/bundle.js`
- Plugin responds to user input

---

### V.5 - Plugin KV storage works
**Type:** Manual
**Status:** READY TO TEST

**Steps:**
1. Use a plugin that uses KV storage
2. Store a value via plugin
3. Retrieve the value
4. Restart server
5. Verify value persists

**Technical Details:**
- KV API: `plugin_kv_get()`, `plugin_kv_set()`, `plugin_kv_delete()`
- Storage: SQLite `plugin_kv` table
- Test coverage: T9, T10, T11 (unit), T18 (integration)

---

### V.6 - Plugin appears in sidebar when enabled
**Type:** Manual
**Status:** READY TO TEST

**Steps:**
1. Login as any user
2. Enable a plugin (admin)
3. Verify plugin appears in sidebar for all users
4. Click plugin in sidebar
5. Verify plugin page loads

**Access Control:**
- All authenticated users see enabled plugins
- Only admin can enable/disable

---

### V.7 - Plugin hidden from sidebar when disabled
**Type:** Manual
**Status:** READY TO TEST

**Steps:**
1. Disable a plugin (admin)
2. Verify plugin disappears from sidebar
3. Verify direct URL returns 404

---

### V.8 - Server starts with no plugins (empty directory)
**Type:** Automated-compatible
**Status:** TESTED (T3)

**Automated Coverage:** Test T3 verifies directory creation.

**Manual Verification:**
1. Move all `.binary` files out of `./plugins/`
2. Start server
3. Verify server starts without error
4. Navigate to Plugins page - should show empty list

---

### V.9 - Server handles invalid .binary files gracefully
**Type:** Automated-compatible
**Status:** TESTED (T2, T4)

**Automated Coverage:**
- T2: Invalid binary handled gracefully
- T4: Metadata failure handled gracefully

**Manual Verification:**
1. Create `./plugins/broken.binary` with invalid content
2. Start server
3. Verify server starts and logs error for broken plugin
4. Verify other plugins still work

---

### V.10 - Plugin crash triggers auto-restart
**Type:** Manual
**Status:** READY TO TEST

**Automated Coverage:** T15 tests restart counter logic.

**Manual Steps:**
1. Enable a plugin
2. Find plugin PID: `ps aux | grep plugin`
3. Kill process: `kill -9 <PID>`
4. Watch logs for restart attempt
5. Verify plugin becomes healthy again

**Expected Behavior:**
- Exponential backoff: 1s, 2s, 4s, 8s, 16s
- Disable after 10 consecutive failures
- Events logged to `plugin_events` table

---

### V.11 - Plugin logs visible in UI
**Type:** Manual
**Status:** READY TO TEST

**Steps:**
1. Enable a plugin
2. Navigate to plugin details
3. View logs section
4. Verify logs are JSON formatted
5. Verify logs include startup events

**Log Locations:**
- `/var/log/toru/plugins/<id>.log` - Plugin logs
- `/var/log/toru/plugin-supervisor.log` - Supervisor logs

**API:** `GET /api/plugins/:id/logs?page=0&page_size=100&level=info`

---

### V.12 - TORIS can read plugin logs
**Type:** Manual (requires TORIS setup)
**Status:** BLOCKED (TORIS not configured)

**Prerequisites:**
- TORIS agent installed
- File watching configured for `/var/log/toru/plugins/`

**Steps:**
1. Generate plugin events
2. Check TORIS dashboard
3. Verify logs ingested correctly

**Notes:**
- Logs are JSON formatted for easy parsing
- Each line is a valid JSON object

---

## Validation Summary

### Can Validate Now (Automated)
| Test | Status | Notes |
|------|--------|-------|
| V.8 | COVERED | T3 test |
| V.9 | COVERED | T2, T4 tests |

### Can Validate Now (Manual with Server)
| Test | Status | Effort |
|------|--------|--------|
| V.1 | READY | 5 min |
| V.2 | READY | 5 min |
| V.3 | READY | 5 min |
| V.4 | READY | 10 min |
| V.5 | READY | 10 min |
| V.6 | READY | 5 min |
| V.7 | READY | 5 min |
| V.10 | READY | 10 min |
| V.11 | READY | 5 min |

### Blocked/Deferred
| Test | Status | Blocker |
|------|--------|---------|
| V.12 | BLOCKED | Requires TORIS configuration |

---

## Risk Assessment

### Low Risk (Covered by Tests)
- Plugin loading/unloading
- Invalid binary handling
- Directory creation
- KV storage operations
- Crash detection logic

### Medium Risk (Needs Manual Testing)
- End-to-end plugin UI flow
- Real plugin communication over Unix socket
- Frontend bundle loading
- Actual process spawning

### Remaining Technical Concerns
1. **Unused methods warning:** `check_plugin_health`, `send_shutdown_message`, `restart_plugin_with_backoff` are defined but not called from main code paths. Need to verify if these are intentionally unused or missing integration.

2. **Drop reference warning:** `drop(process)` doesn't actually drop since it's a reference. Minor issue but indicates potential logic error.

3. **Log directory permissions:** `/var/log/toru/` may need sudo to create on fresh systems.

---

## Recommended Next Steps

### Immediate (This Session)
1. Run server and complete V.1-V.7 manually
2. Document any issues found
3. Test plugin crash restart (V.10)
4. Verify log output (V.11)

### Before Merge to Main
1. Fix the `drop(process)` warning (non-critical)
2. Verify log directory creation works or fails gracefully
3. Add note about TORIS testing (V.12) for future

### Post-Merge
1. Configure TORIS for plugin log monitoring
2. Complete V.12 validation
3. Consider adding smoke test script

---

## Quick Test Commands

```bash
# Run all tests
cargo test

# Build and run server
ADMIN_PASSWORD=dev cargo run

# Test plugin metadata
./plugins/hello-plugin-rust.binary --metadata
./plugins/hello-plugin-python.binary --metadata

# View plugin logs (when running)
tail -f /var/log/toru/plugins/hello-plugin-rust.log

# Check if plugin process is running
ps aux | grep hello-plugin
```

---

## Files Changed/Added

This validation plan created: `/Users/tako/GitRepos/toru-steering-center/validation-plan-plugin-system.md`
