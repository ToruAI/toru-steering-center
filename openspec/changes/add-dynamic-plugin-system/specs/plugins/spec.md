# plugins Capability

## Implementation Status Summary

**FULLY IMPLEMENTED:**
- Plugin Process Isolation - separate processes with crash isolation
- Plugin Protocol - Unix sockets with JSON message protocol
- Plugin API Contract - metadata command, socket listening, HTTP request handling
- Plugin Lifecycle - enable/disable with state persistence
- Plugin Routes - `/api/plugins/route/<plugin-route>/*` routing with path forwarding
- Plugin Frontend - bundle.js serving, mount/unmount lifecycle
- Plugin Sidebar Integration - health indicators, enabled plugin display
- Plugin Manager UI - list, toggle, view logs, view details
- Plugin Key-Value Storage - HTTP API at `/api/plugins/:id/kv` (not socket-based)
- Plugin API Endpoints - full REST API for management
- Plugin Observability - database-backed logs, event tracking
- Plugin Language Support - Rust and Python support verified
- Plugin Security - path traversal prevention, 30s timeout, authentication, metadata validation, namespace isolation
- Plugin Supervision - crash detection and auto-restart with exponential backoff (verified 2026-01-13)

**NOT IMPLEMENTED:**
- TORIS integration - TORIS system not yet set up
- File-based supervisor logs - uses standard tracing instead

## ADDED Requirements

### Requirement: Plugin Process Isolation
The system SHALL run each plugin as a separate process with crash isolation.

#### Scenario: Spawn plugin process
- **WHEN** a valid `.binary` file exists in `./plugins/`
- **THEN** the system spawns a new process for the plugin

#### Scenario: Plugin crash isolation
- **WHEN** a plugin process crashes
- **THEN** the core process continues running unaffected

#### Scenario: Plugin directory missing
- **WHEN** the `./plugins/` directory does not exist
- **THEN** the system creates it and continues with no plugins loaded

#### Scenario: Invalid plugin file
- **WHEN** a `.binary` file fails to start or crashes immediately
- **THEN** the system logs an error and continues loading other plugins

### Requirement: Plugin Protocol
The system SHALL communicate with plugins via Unix domain sockets using JSON messages.

#### Scenario: Send init message
- **WHEN** a plugin process is spawned
- **THEN** the system sends an init message with socket_path and log_path

#### Scenario: Forward HTTP request
- **WHEN** an HTTP request is received for a plugin route
- **THEN** the system forwards the request to the plugin via Unix socket

#### Scenario: Receive plugin response
- **WHEN** a plugin responds to a request
- **THEN** the system forwards the response to the client

#### Scenario: Handle plugin error
- **WHEN** a plugin returns an error or times out
- **THEN** the system returns an appropriate error response to the client

### Requirement: Plugin API Contract
The system SHALL expect plugins to implement a standardized message protocol.

#### Scenario: Plugin metadata
- **WHEN** a plugin binary is invoked with `--metadata` flag
- **THEN** the plugin returns JSON metadata (id, name, version, icon, route)

#### Scenario: Plugin listens on socket
- **WHEN** a plugin starts
- **THEN** it creates a Unix socket at the specified path
- **THEN** it listens for and handles messages

#### Scenario: Plugin handles HTTP request
- **WHEN** a plugin receives an HTTP message via socket
- **THEN** it returns an HTTP response with status, headers, and body

#### Scenario: Plugin accesses KV storage
- **WHEN** a plugin needs persistent storage
- **THEN** it makes HTTP POST requests to `/api/plugins/:id/kv` (not via socket)

### Requirement: Plugin Supervision
The system SHALL monitor plugin processes and restart them on failure.

#### Scenario: Monitor plugin health
- **WHEN** a plugin is running
- **THEN** the system checks if socket exists and process is running

#### Scenario: Detect plugin crash
- **WHEN** a plugin process dies unexpectedly
- **THEN** the system detects the crash via process monitoring

#### Scenario: Restart crashed plugin
- **WHEN** a plugin crashes
- **THEN** the system waits with exponential backoff (1s, 2s, 4s, 8s, 16s)
- **THEN** the system attempts to restart the plugin automatically

#### Scenario: Disable unstable plugin
- **WHEN** a plugin crashes 10 consecutive times
- **THEN** the system disables the plugin
- **THEN** the system logs an event to the database

### Requirement: Plugin Lifecycle
The system SHALL support enabling and disabling plugins by starting/stopping their processes.

#### Scenario: Enable plugin
- **WHEN** `POST /api/plugins/:id/enable` is called for a disabled plugin
- **THEN** the plugin process is spawned
- **THEN** the system waits up to 2 seconds for the socket to become available
- **THEN** the plugin's routes become available
- **THEN** the plugin appears in the sidebar

#### Scenario: Disable plugin
- **WHEN** `POST /api/plugins/:id/disable` is called for an enabled plugin
- **THEN** the system sends a shutdown message to the plugin
- **THEN** the plugin process is killed
- **THEN** the plugin's routes return 404
- **THEN** the plugin is hidden from the sidebar

#### Scenario: Persist enabled state
- **WHEN** a plugin is enabled or disabled
- **THEN** the state persists across server restarts

#### Scenario: List plugins
- **WHEN** the system starts
- **THEN** it loads the enabled state from `./plugins/.metadata/config.json`
- **THEN** it spawns processes for enabled plugins only

### Requirement: Plugin Routes
The system SHALL register plugin-provided routes under the plugin's configured path.

#### Scenario: Register plugin routes
- **WHEN** an enabled plugin declares a route prefix (e.g., `/hello-plugin`)
- **THEN** requests to `/api/plugins/route/hello-plugin/*` are forwarded to the plugin
- **THEN** the remaining path is passed to the plugin (e.g., `/some/path`)
- **THEN** query strings are preserved and passed to the plugin

#### Scenario: Disabled plugin routes
- **WHEN** a plugin is disabled
- **THEN** requests to its routes return 404 Not Found

#### Scenario: Plugin returns HTTP response
- **WHEN** a plugin handles a request successfully
- **THEN** the system returns the plugin's response (status, headers, body) to the client
- **THEN** responses have a 30-second timeout to prevent hanging on unresponsive plugins

### Requirement: Plugin Frontend
The system SHALL serve plugin frontend bundles and render them in a container.

#### Scenario: Serve frontend bundle
- **WHEN** `GET /api/plugins/:id/bundle.js` is requested
- **THEN** the system returns the plugin's embedded JavaScript bundle

#### Scenario: Mount plugin view
- **WHEN** user navigates to a plugin's route
- **THEN** the frontend loads the bundle from `/api/plugins/:id/bundle.js`
- **THEN** the frontend calls `mount(container, api)` with the DOM element and API object

#### Scenario: Unmount plugin view
- **WHEN** user navigates away from a plugin's route
- **THEN** the frontend calls `unmount(container)` for cleanup

### Requirement: Plugin Sidebar Integration
The system SHALL display enabled plugins in the sidebar navigation.

#### Scenario: Show enabled plugins
- **WHEN** plugins are enabled
- **THEN** the sidebar shows each plugin with its icon and name
- **THEN** a health indicator (green/red dot) shows if the plugin is running

#### Scenario: Hide disabled plugins
- **WHEN** plugins are disabled
- **THEN** they do not appear in the sidebar

#### Scenario: Show crashed plugin
- **WHEN** a plugin has crashed
- **THEN** the health indicator is red
- **THEN** clicking the plugin shows error details and logs

### Requirement: Plugin Manager UI
The system SHALL provide a page to view and manage installed plugins.

#### Scenario: List plugins
- **WHEN** user navigates to Plugin Manager
- **THEN** all installed plugins are shown with name, version, status, and health

#### Scenario: Toggle plugin
- **WHEN** user clicks enable/disable toggle
- **THEN** the plugin state changes
- **THEN** the UI updates
- **THEN** a toast notification confirms the action

#### Scenario: View plugin logs
- **WHEN** user clicks "View Logs" for a plugin
- **THEN** a modal or sidebar shows the plugin's logs
- **THEN** logs are formatted and readable

#### Scenario: View plugin details
- **WHEN** user clicks on a plugin card
- **THEN** a details panel shows metadata (author, description, version)

### Requirement: Plugin Key-Value Storage
The system SHALL provide plugins with isolated key-value storage via HTTP API.

**Implementation:** KV operations use HTTP endpoint `/api/plugins/:id/kv` with JSON payloads, NOT socket messages. This allows plugins to make authenticated HTTP requests for storage.

#### Scenario: Store value
- **WHEN** a plugin sends `POST /api/plugins/:id/kv` with `{"action": "set", "key": "...", "value": "..."}`
- **THEN** the value is stored in the plugin's namespace in the database

#### Scenario: Retrieve value
- **WHEN** a plugin sends `POST /api/plugins/:id/kv` with `{"action": "get", "key": "..."}`
- **THEN** the value from the plugin's namespace is returned in response JSON

#### Scenario: Delete value
- **WHEN** a plugin sends `POST /api/plugins/:id/kv` with `{"action": "delete", "key": "..."}`
- **THEN** the value is removed from the database

#### Scenario: Namespace isolation
- **WHEN** two plugins use the same key
- **THEN** each plugin sees only its own value (enforced by plugin_id in database)

#### Scenario: KV persistence
- **WHEN** the server restarts
- **THEN** plugin KV data persists in SQLite database

### Requirement: Plugin Security
The system SHALL enforce security constraints on plugin operations.

#### Scenario: Path traversal prevention
- **WHEN** a plugin route request contains `..` or `/` in the route segment
- **THEN** the request is rejected with 400 Bad Request

#### Scenario: Response timeout
- **WHEN** a plugin takes longer than 30 seconds to respond
- **THEN** the request is terminated and client receives a timeout error

#### Scenario: Authenticated access
- **WHEN** any plugin route or management endpoint is accessed
- **THEN** the request must be authenticated (admin or client user)

#### Scenario: KV namespace isolation
- **WHEN** a plugin accesses KV storage
- **THEN** operations are restricted to its own namespace by plugin_id

#### Scenario: Metadata validation
- **WHEN** a plugin's metadata JSON is parsed
- **THEN** the plugin ID must be alphanumeric with hyphens only
- **THEN** the route must start with `/` and not contain `..`
- **THEN** name and author fields must be under 100 characters
- **THEN** invalid metadata causes the plugin to be skipped with a logged error

### Requirement: Plugin API Endpoints
The system SHALL expose API endpoints for plugin management.

#### Scenario: List plugins endpoint
- **WHEN** `GET /api/plugins` is called
- **THEN** all installed plugins are returned with metadata and status

#### Scenario: Get plugin endpoint
- **WHEN** `GET /api/plugins/:id` is called for an existing plugin
- **THEN** the plugin's full details are returned

#### Scenario: Get unknown plugin
- **WHEN** `GET /api/plugins/:id` is called for a non-existent plugin
- **THEN** 404 Not Found is returned

#### Scenario: Get plugin logs endpoint
- **WHEN** `GET /api/plugins/:id/logs` is called
- **THEN** the plugin's logs are returned (paginated)

### Requirement: Plugin Observability
The system SHALL provide structured logging for plugin operations.

**TORIS Integration Status:** TORIS is planned but not yet set up. Log infrastructure exists but TORIS monitoring is not active.

#### Scenario: Write plugin logs
- **WHEN** a plugin writes a log message
- **THEN** the message is written to database-backed log store
- **THEN** logs are retrievable via `/api/plugins/:id/logs` endpoint

#### Scenario: Write supervisor logs [PARTIAL]
- **WHEN** the plugin supervisor performs an action (spawn, kill)
- **THEN** events are logged via standard Rust logging (tracing crate)
- **NOTE:** File-based supervisor logs to `/var/log/toru/plugin-supervisor.log` not implemented

#### Scenario: Log plugin events
- **WHEN** a plugin event occurs (started, stopped, crashed, restarted, disabled)
- **THEN** the event is written to the `plugin_events` table in the database

#### Scenario: TORIS reads logs [NOT IMPLEMENTED]
- **STATUS:** TORIS not yet set up
- **PLANNED:** TORIS will watch log directories and monitor plugin health

### Requirement: Plugin Language Support
The system SHALL support plugins written in Rust and Python (MVP), with extensibility to other languages.

#### Scenario: Rust plugin
- **WHEN** a Rust plugin is built using toru-plugin-api
- **THEN** it implements the ToruPlugin trait
- **THEN** it communicates via Unix sockets using the protocol

#### Scenario: Python plugin
- **WHEN** a Python plugin is built
- **THEN** it implements the protocol manually
- **THEN** it communicates via Unix sockets using JSON messages

#### Scenario: Other language plugin (future)
- **WHEN** a plugin is written in another language
- **THEN** it can implement the same protocol
- **THEN** it will work without changes to the core system
