---
created: 2025-12-30T15:33:10.677Z
updated: 2026-01-13T17:29:40.117Z
type: memory
---
# Phase 7 Logging & Observability - Key Decisions

**Date:** 2025-12-30

## Decision 1: Size-based log rotation (not time-based)

**Choice:** Rotate logs when they reach 10MB in size, not based on time intervals.

**Rationale:**
- Simpler implementation (no background rotation tasks needed)
- Better control of disk usage (predictable max disk usage: 10MB × 6 files)
- Check-on-write is fast (simple file size check before append)
- Time-based rotation requires background tasks and more complex state management

**Trade-offs:**
- Small logs might never rotate (not a problem for observability)
- Large burst of logs could exceed 10MB temporarily (acceptable trade-off for simplicity)

## Decision 2: Separate log files per plugin

**Choice:** Each plugin gets its own log file (`/var/log/toru/plugins/<id>.log`), not a shared log.

**Rationale:**
- Easier to debug specific plugin issues
- Better isolation (plugin logs don't interfere with each other)
- Simpler filtering (no need to parse plugin_id from every line)
- TORIS can watch individual plugin logs for alerts

**Trade-offs:**
- More file descriptors (but plugins are small in number, typically < 10)
- More filesystem operations (acceptable for modern systems)

## Decision 3: JSON structured logs

**Choice:** All logs are JSON objects with structured fields, not plain text.

**Rationale:**
- TORIS can parse and alert on structured fields (level, plugin, error)
- Easier to filter and aggregate in monitoring dashboards
- Standard format that works with many log aggregators
- Type safety with serde serialization

**Trade-offs:**
- Larger log size (JSON vs plain text)
- Slightly slower write time (JSON serialization vs simple append)
- Mitigation: Logs are not high-throughput; overhead is negligible

## Decision 4: Supervisor logs to separate file

**Choice:** PluginSupervisor events go to `/var/log/toru/plugin-supervisor.log`, not interleaved with plugin logs.

**Rationale:**
- Clear separation of concerns: supervisor events vs plugin events
- Easier to debug supervisor issues without noise from plugins
- TORIS can monitor supervisor file for critical system events
- Avoids permission issues (supervisor logs to system-owned file)

## Decision 5: Pagination on read, not write

**Choice:** Write logs as simple appends; pagination only on API read operations.

**Rationale:**
- Write path stays fast (no indexing or pagination structures)
- Read path handles pagination efficiently (read entire file, filter, paginate)
- Simple implementation with no background indexing tasks
- Logs are not queried frequently enough to warrant write-time indexing

**Trade-offs:**
- Reading large log files is slower (O(n) on read)
- Mitigation: Log rotation keeps files small (< 10MB), O(10MB) is acceptable

## Decision 6: RFC3339 timestamps

**Choice:** Use `chrono::Utc::now().to_rfc3339()` for timestamps, not Unix epoch.

**Rationale:**
- Human-readable and sortable as strings
- Includes timezone information (always UTC)
- Standard format that works with many log tools
- No need for separate timezone field

**Trade-offs:**
- Larger than Unix epoch (28 chars vs 10 chars)
- Negligible impact given structured JSON format already adds overhead

## Decision 7: Log level severity ordering

**Choice:** Map levels to numeric severity (Trace=0, Debug=1, Info=2, Warn=3, Error=4) for filtering.

**Rationale:**
- Simple to implement ("filter >= error" means show warn+error only)
- Standard log4j-style semantics
- Easy to understand for developers
- Fast comparison (integer compare)

## Deferred Decisions

1. **Log shipping to external services**: Not implemented; can be added later via TORIS
2. **Database event logging (T23)**: Blocked on supervisor integration with AppState; will be added in Phase 5 integration
3. **Compressed log rotation**: Not implemented; plain JSON is fine for 10MB files


## 2026-01-13T17:18:01.275Z
## 2026-01-13 - Validation Plan Created

**Context:** Plugin system implementation (Phase 1-7) complete. Need to validate before merge.

**Analysis Performed:**
- All 23 tests passing (8 unit + 15 integration)
- Both example plugins (Rust, Python) built and functional
- Backend and frontend builds succeed
- Plugin config shows both plugins enabled

**Key Findings:**
1. V.1-V.11 can be validated manually by running server
2. V.8, V.9 already covered by automated tests (T2, T3, T4)
3. V.12 (TORIS integration) blocked - requires TORIS setup

**Validation Plan:** Written to `/validation-plan-plugin-system.md`

**Technical Notes:**
- 10 compiler warnings (non-critical, mostly unused methods)
- One `drop(process)` warning should be fixed
- Log directory `/var/log/toru/` may need elevated permissions

**Decision:** Proceed with manual validation of V.1-V.11 before merge. V.12 deferred until TORIS is configured.

## 2026-01-13T17:29:40.117Z
## 2026-01-13 - Execution Plan Created for Final Push

**Context:** Code review identified 4 critical issues (security + reliability). Integration tests use mocks instead of actual PluginSupervisor. Dead code warnings in build.

**Decision:** Created sequenced execution plan with 5 phases:
1. **Phase 1: Security Fixes** (2h) - Path traversal, metadata injection, timeout, EOF fix
2. **Phase 2: Test Rewrite** (3h) - Replace mock tests with real PluginSupervisor integration
3. **Phase 3: Dead Code Cleanup** (1h) - Integrate unused methods or remove
4. **Phase 4: Build Gate** (0.5h) - Verify clean build
5. **Phase 5: Manual Validation** (1.5h) - V.1-V.11 tests

**Key Insights:**
- Tests currently pass but don't test actual code (shell script mocks)
- Security issues are fixable in ~2h, don't require redesign
- Dead code is mostly crash recovery methods that need integration
- TORIS integration (V.12) explicitly deferred

**Plan Location:** `/execution-plan-plugin-system.md`

**Next Action:** Execute Phase 1 security fixes (BOB)