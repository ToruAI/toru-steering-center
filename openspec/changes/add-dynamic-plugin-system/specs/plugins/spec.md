# plugins Capability

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
- **WHEN** a plugin receives an HTTP message
- **THEN** it returns an HTTP response with status, headers, and body

#### Scenario: Plugin handles KV operation
- **WHEN** a plugin receives a KV message (get/set/delete)
- **THEN** it performs the operation and returns the result

### Requirement: Plugin Supervision
The system SHALL monitor plugin processes and restart them on failure.

#### Scenario: Monitor plugin health
- **WHEN** a plugin is running
- **THEN** the system checks its socket status periodically

#### Scenario: Detect plugin crash
- **WHEN** a plugin process dies
- **THEN** the system detects the crash via process monitoring

#### Scenario: Restart crashed plugin
- **WHEN** a plugin crashes
- **THEN** the system waits with exponential backoff (1s, 2s, 4s, 8s, 16s)
- **THEN** the system attempts to restart the plugin

#### Scenario: Disable unstable plugin
- **WHEN** a plugin crashes 10 consecutive times
- **THEN** the system disables the plugin
- **THEN** the system logs an event and writes to the database

### Requirement: Plugin Lifecycle
The system SHALL support enabling and disabling plugins by starting/stopping their processes.

#### Scenario: Enable plugin
- **WHEN** `POST /api/plugins/:id/enable` is called for a disabled plugin
- **THEN** the plugin process is spawned
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
- **WHEN** an enabled plugin declares a route prefix (e.g., `/acme`)
- **THEN** requests to `/plugins/acme/*` are forwarded to the plugin
- **THEN** the full path is passed to the plugin (e.g., `/acme/certificate`)

#### Scenario: Disabled plugin routes
- **WHEN** a plugin is disabled
- **THEN** requests to its routes return 404 Not Found

#### Scenario: Plugin returns HTTP response
- **WHEN** a plugin handles a request successfully
- **THEN** the system returns the plugin's response (status, headers, body) to the client

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
The system SHALL provide plugins with isolated key-value storage.

#### Scenario: Store value
- **WHEN** a plugin sends a KV set message
- **THEN** the value is stored in the plugin's namespace in the database

#### Scenario: Retrieve value
- **WHEN** a plugin sends a KV get message
- **THEN** the value from the plugin's namespace is returned

#### Scenario: Delete value
- **WHEN** a plugin sends a KV delete message
- **THEN** the value is removed from the database

#### Scenario: Namespace isolation
- **WHEN** two plugins use the same key
- **THEN** each plugin sees only its own value

#### Scenario: KV persistence
- **WHEN** the server restarts
- **THEN** plugin KV data persists

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
The system SHALL provide structured logging for TORIS integration.

#### Scenario: Write plugin logs
- **WHEN** a plugin writes a log message
- **THEN** the message is written to `/var/log/toru/plugins/<plugin-id>.log`
- **THEN** the message is in JSON format (timestamp, level, message, optional error)

#### Scenario: Write supervisor logs
- **WHEN** the plugin supervisor performs an action (spawn, kill, restart)
- **THEN** an event is written to `/var/log/toru/plugin-supervisor.log`

#### Scenario: Log plugin events
- **WHEN** a plugin event occurs (started, stopped, crashed, restarted, disabled)
- **THEN** the event is written to the `plugin_events` table in the database

#### Scenario: TORIS reads logs
- **WHEN** TORIS watches the log directories
- **THEN** it can read and parse plugin logs
- **THEN** it can monitor plugin health and activity

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
