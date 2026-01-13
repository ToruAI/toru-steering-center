# Phase 7: Logging & Observability - Completion Summary

**Date:** 2025-12-30
**Status:** ✅ Complete

## Overview

Implemented comprehensive structured logging infrastructure for the plugin system, including per-plugin JSON logs, supervisor event logging, automatic log rotation, and a paginated/filterable API for log retrieval.

## What Was Built

### 1. Core Logging Module (`src/services/logging.rs`)

Created a complete logging infrastructure with:

- **LogLevel enum**: Trace, Debug, Info, Warn, Error (with severity filtering)
- **LogEntry struct**: Structured JSON log entries with:
  - `timestamp`: RFC3339 format
  - `level`: Log level string
  - `message`: Log message
  - `plugin` (optional): Plugin identifier
  - `error` (optional): Error details
  - `pid` (optional): Process ID

- **LogConfig**: Configurable settings:
  - `max_file_size`: 10MB default before rotation
  - `max_rotated_files`: 5 files to keep
  - `log_dir`: `/var/log/toru` default

- **PluginLogger**: Per-plugin log management:
  - Auto-creates `/var/log/toru/plugins/` directory
  - Each plugin gets its own log file: `/var/log/toru/plugins/<id>.log`
  - Automatic size-based rotation (10MB → timestamped backup)
  - Cleanup of old rotated logs (keeps 5 most recent)
  - Paginated log retrieval with level filtering

- **SupervisorLogger**: Core system event logging:
  - Logs to `/var/log/toru/plugin-supervisor.log`
  - Events: spawn, kill, enable, disable, crash, restart
  - Structured JSON format for TORIS integration

### 2. Supervisor Integration

Updated `src/services/plugins.rs` to use logging:

- Added `plugin_logger` and `supervisor_logger` fields to `PluginSupervisor`
- Logs all major plugin lifecycle events:
  - **Spawn**: Plugin ID, PID
  - **Kill**: Plugin disabled event
  - **Enable/Disable**: State changes
  - **Restart with backoff**: Attempt number, delay time
  - **Max restarts reached**: Disable event with reason
- Plugin log path passed to plugins via init message

### 3. Enhanced Log API (`src/routes/plugins.rs`)

Updated `GET /api/plugins/:id/logs` with:

- **Query parameters**:
  - `page`: Page number (default 0)
  - `page_size`: Items per page (default 100)
  - `level`: Filter by log level (trace/debug/info/warn/error)
- **Response format**:
  ```json
  {
    "logs": [LogEntry...],
    "page": 0,
    "page_size": 100
  }
  ```
- **Behavior**:
  - Returns newest logs first (descending timestamp)
  - Filters by log level if specified
  - Efficient pagination (no loading entire file into memory)
  - Returns empty array if log file doesn't exist

### 4. Frontend API Client (`frontend/src/lib/api.ts`)

Updated TypeScript interfaces and functions:

- **PluginLogEntry interface**: Now matches backend struct exactly
- **PluginLogsResponse interface**: Added for paginated responses
- **getPluginLogs function**: Now supports options object:
  ```typescript
  api.getPluginLogs(pluginId, {
    page: 0,
    page_size: 100,
    level: 'error'  // optional filter
  })
  ```

## Key Features

### Log Rotation
- **Size-based**: Triggers when log file reaches 10MB
- **Timestamped backups**: `plugin-20251230-154522.log` format
- **Automatic cleanup**: Keeps only 5 most recent rotated files
- **No service interruption**: Rotation is atomic (rename + create new)

### Structured JSON Format
All logs are structured for easy parsing by TORIS:

```json
{
  "timestamp": "2025-12-30T15:30:45.123Z",
  "level": "Info",
  "message": "Plugin spawned: hello-plugin (PID: Some(12345))",
  "plugin": "hello-plugin",
  "error": null,
  "pid": null
}
```

### TORIS Integration
- Logs written to `/var/log/toru/` for easy monitoring
- Supervisor logs to `/var/log/toru/plugin-supervisor.log`
- Plugin logs to `/var/log/toru/plugins/<id>.log`
- JSON format for easy parsing and alerting
- Structured fields for filtering (level, plugin, error)

## Testing Notes

### Unit Tests
Added tests in `src/services/logging.rs`:
- `test_log_entry_creation`: Verifies log entry builder pattern
- `test_log_level_severity`: Confirms severity ordering
- `test_log_level_from_str`: Tests level parsing

### Integration Tests
Ready for testing once Phase 8 (Example Plugins) is complete:
- **T20**: Plugin logs written to correct file
- **T21**: Logs are valid JSON
- **T22**: Logs API returns correct logs
- **T23**: Plugin events written to database (deferred to Phase 5 integration)

## Architecture Decisions

1. **Size-based rotation over time-based**: Simpler to implement, easier to control disk usage
2. **Separate log files per plugin**: Easier to debug specific plugins, better isolation
3. **JSON format**: Structured logs are easier to parse for monitoring/alerting
4. **RFC3339 timestamps**: Standard format with timezone, sortable as strings
5. **Pagination on read, not write**: Write path is fast append; read path handles pagination

## Open Questions

1. **Database logging (T23)**: Plugin events should also go to `plugin_events` table
   - This is blocked until supervisor is fully integrated with AppState
   - Can be added in Phase 5 integration or as a follow-up task

2. **Log shipping to external services**: Should logs be shipped to external monitoring?
   - Not in scope for Phase 7
   - Can be added via TORIS or external log aggregator

## Next Steps

Phase 8: Example Plugins
- Build Rust example plugin
- Build Python example plugin
- Test logging with actual plugins
- Verify log rotation in production

## Files Changed

- ✅ `src/services/logging.rs` (new)
- ✅ `src/services/mod.rs` (export logging module)
- ✅ `src/services/plugins.rs` (integrate logging)
- ✅ `src/routes/plugins.rs` (enhanced logs API)
- ✅ `frontend/src/lib/api.ts` (pagination support)
- ✅ `openspec/changes/add-dynamic-plugin-system/tasks.md` (marked complete)
