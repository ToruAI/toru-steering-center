# plugins Capability

## ADDED Requirements

### Requirement: Plugin Loading
The system SHALL load plugins from dynamic library files (.so) in the `./plugins/` directory on startup.

#### Scenario: Load valid plugin
- **WHEN** a valid `.so` file exists in `./plugins/`
- **THEN** the system loads the library and calls `create_plugin()` to obtain the plugin instance

#### Scenario: Skip invalid plugin
- **WHEN** a `.so` file fails to load or export required symbols
- **THEN** the system logs an error and continues loading other plugins

#### Scenario: Plugin directory missing
- **WHEN** the `./plugins/` directory does not exist
- **THEN** the system creates it and continues with no plugins loaded

### Requirement: Plugin API Contract
The system SHALL require plugins to implement the `ToruPlugin` trait and export `create_plugin` and `destroy_plugin` C functions.

#### Scenario: Plugin exports required symbols
- **WHEN** a plugin exports `create_plugin() -> *mut dyn ToruPlugin` and `destroy_plugin(*mut dyn ToruPlugin)`
- **THEN** the system can load and unload the plugin safely

#### Scenario: Plugin provides metadata
- **WHEN** a plugin is loaded
- **THEN** `metadata()` returns id, name, version, icon, and route

### Requirement: Plugin Lifecycle
The system SHALL support enabling and disabling plugins without unloading them from memory.

#### Scenario: Enable plugin
- **WHEN** `POST /api/plugins/:id/enable` is called for a disabled plugin
- **THEN** the plugin's routes become available and it appears in the sidebar

#### Scenario: Disable plugin
- **WHEN** `POST /api/plugins/:id/disable` is called for an enabled plugin
- **THEN** the plugin's routes return 404 and it is hidden from the sidebar

#### Scenario: Persist enabled state
- **WHEN** a plugin is enabled or disabled
- **THEN** the state persists across server restarts

### Requirement: Plugin Routes
The system SHALL register plugin-provided routes under the plugin's configured path.

#### Scenario: Register plugin routes
- **WHEN** an enabled plugin returns routes from `register_routes()`
- **THEN** those routes are accessible under `/api/plugins/:id/*`

#### Scenario: Disabled plugin routes
- **WHEN** a plugin is disabled
- **THEN** requests to its routes return 404 Not Found

### Requirement: Plugin Frontend
The system SHALL serve plugin frontend bundles and render them in a container.

#### Scenario: Serve frontend bundle
- **WHEN** `GET /api/plugins/:id/bundle.js` is requested
- **THEN** the system returns the plugin's embedded JavaScript bundle

#### Scenario: Mount plugin view
- **WHEN** user navigates to a plugin's route
- **THEN** the frontend loads the bundle and calls `mount(container, api)`

#### Scenario: Unmount plugin view
- **WHEN** user navigates away from a plugin's route
- **THEN** the frontend calls `unmount(container)` for cleanup

### Requirement: Plugin Sidebar Integration
The system SHALL display enabled plugins in the sidebar navigation.

#### Scenario: Show enabled plugins
- **WHEN** plugins are enabled
- **THEN** the sidebar shows each plugin with its icon and name

#### Scenario: Hide disabled plugins
- **WHEN** plugins are disabled
- **THEN** they do not appear in the sidebar

### Requirement: Plugin Manager UI
The system SHALL provide a page to view and manage installed plugins.

#### Scenario: List plugins
- **WHEN** user navigates to Plugin Manager
- **THEN** all loaded plugins are shown with name, version, status

#### Scenario: Toggle plugin
- **WHEN** user clicks enable/disable toggle
- **THEN** the plugin state changes and UI updates

### Requirement: Instance Identity
The system SHALL generate and persist a unique instance ID on first run.

#### Scenario: Generate instance ID
- **WHEN** the server starts for the first time
- **THEN** a UUID v4 is generated and stored in the database

#### Scenario: Retrieve instance ID
- **WHEN** the server starts subsequently
- **THEN** the existing instance ID is loaded from the database

#### Scenario: Expose instance ID
- **WHEN** a plugin's `init()` is called
- **THEN** `PluginContext.instance_id` contains the instance ID

### Requirement: Plugin Licensing
The system SHALL support optional license validation where plugins can verify a license key against the instance ID.

#### Scenario: License validation
- **WHEN** a plugin's `init()` is called
- **THEN** the plugin can validate a license key against the instance ID

#### Scenario: License failure
- **WHEN** license validation fails
- **THEN** the plugin returns an error and is not enabled

#### Scenario: No license required
- **WHEN** a plugin does not implement license checking
- **THEN** the plugin loads normally (community plugins)

### Requirement: Plugin Key-Value Storage
The system SHALL provide plugins with isolated key-value storage.

#### Scenario: Store value
- **WHEN** a plugin calls `kv.set(key, value)`
- **THEN** the value is stored in the plugin's namespace

#### Scenario: Retrieve value
- **WHEN** a plugin calls `kv.get(key)`
- **THEN** the value from the plugin's namespace is returned

#### Scenario: Namespace isolation
- **WHEN** two plugins use the same key
- **THEN** each plugin sees only its own value

### Requirement: Plugin API Endpoints
The system SHALL expose API endpoints for plugin management.

#### Scenario: List plugins endpoint
- **WHEN** `GET /api/plugins` is called
- **THEN** all loaded plugins are returned with metadata and status

#### Scenario: Get plugin endpoint
- **WHEN** `GET /api/plugins/:id` is called for an existing plugin
- **THEN** the plugin's full details are returned

#### Scenario: Get unknown plugin
- **WHEN** `GET /api/plugins/:id` is called for a non-existent plugin
- **THEN** 404 Not Found is returned
