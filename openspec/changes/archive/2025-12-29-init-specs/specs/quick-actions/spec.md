## ADDED Requirements

### Requirement: Quick Action Management
The system SHALL allow Admins to create shortcuts for frequently used scripts.

#### Scenario: Create Quick Action
- **WHEN** an Admin creates a Quick Action
- **THEN** they MUST specify a name and a script path
- **AND** MAY specify an icon and display order

#### Scenario: List Quick Actions
- **WHEN** any user requests Quick Actions
- **THEN** the system returns all configured actions sorted by display order

### Requirement: Quick Action Execution
The system SHALL allow users to execute scripts via Quick Actions.

#### Scenario: Execute Quick Action
- **WHEN** a user triggers a Quick Action
- **THEN** the system looks up the associated script path
- **AND** initiates a new execution task
- **AND** returns a `task_id` for tracking
