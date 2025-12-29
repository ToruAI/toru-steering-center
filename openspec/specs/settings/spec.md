# settings Specification

## Purpose
TBD - created by archiving change init-specs. Update Purpose after archive.
## Requirements
### Requirement: Global Configuration
The system SHALL support dynamic global configuration via a key-value store.

#### Scenario: Get settings
- **WHEN** an Admin requests settings
- **THEN** the system returns all configured key-value pairs

#### Scenario: Update setting
- **WHEN** an Admin updates a setting key
- **THEN** the value is persisted in the database
- **AND** the change takes effect immediately

### Requirement: Scripts Directory Configuration
The system SHALL allow configuring the location of executable scripts.

#### Scenario: Default scripts directory
- **WHEN** no `scripts_dir` setting is configured
- **THEN** the system defaults to `./scripts`

#### Scenario: Custom scripts directory
- **WHEN** the `scripts_dir` setting is updated
- **THEN** the system looks for scripts in the new location

