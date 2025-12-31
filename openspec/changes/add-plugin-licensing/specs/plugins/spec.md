# plugins Capability

## ADDED Requirements

### Requirement: Instance Identity
The system SHALL generate and persist a unique instance ID on first run.

#### Scenario: Generate instance ID
- **WHEN** the server starts for the first time
- **THEN** a UUID v4 is generated
- **THEN** the instance ID is stored in the database settings table

#### Scenario: Retrieve instance ID
- **WHEN** the server starts subsequently
- **THEN** the existing instance ID is loaded from the database

#### Scenario: Expose instance ID to plugin
- **WHEN** a plugin's init message is sent
- **THEN** the message contains the instance ID

### Requirement: Plugin Licensing
The system SHALL support optional license validation where plugins can verify a license key against the instance ID.

#### Scenario: License validation
- **WHEN** a plugin's `init()` is called
- **THEN** the plugin can validate a license key against the instance ID

#### Scenario: License failure
- **WHEN** license validation fails
- **THEN** the plugin logs an error
- **THEN** the plugin exits with a non-zero status

#### Scenario: No license required
- **WHEN** a plugin does not implement license checking
- **THEN** the plugin loads normally (community plugins)

#### Scenario: Proprietary plugin with valid license
- **WHEN** a proprietary plugin has a valid license
- **THEN** the plugin starts normally and functions

## MODIFIED Requirements

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

#### Scenario: Plugin receives init message
- **WHEN** a plugin starts
- **THEN** the system sends an init message with instance_id, socket_path, and log_path
- **THEN** the plugin initializes with the provided context
