# execution Specification

## Purpose
TBD - created by archiving change init-specs. Update Purpose after archive.
## Requirements
### Requirement: Script Execution
The system SHALL allow execution of shell scripts on the host system.

#### Scenario: List available scripts
- **WHEN** an Admin requests the script list
- **THEN** the system returns names of all `.sh` and `.bash` files in the configured scripts directory

#### Scenario: Execute script
- **WHEN** a script execution is requested
- **THEN** the system spawns a child process
- **AND** streams stdout and stderr in real-time
- **AND** records the execution in task history

### Requirement: Real-time Output Streaming
The system SHALL stream script execution events to clients.

#### Scenario: Stream events
- **WHEN** a script is running
- **THEN** the system sends `started`, `stdout`, `stderr`, and `exit` events via WebSocket/streaming response
- **AND** includes the `task_id` in every message

### Requirement: Task History
The system SHALL maintain an audit log of all executed scripts.

#### Scenario: Record history
- **WHEN** a script finishes execution
- **THEN** the system updates the history record with `finished_at`, `exit_code`, and captured `output`

### Requirement: Task Cancellation
The system SHALL allow cancelling running scripts.

#### Scenario: Cancel task
- **WHEN** a cancellation request is received for a running task
- **THEN** the system kills the child process
- **AND** updates the task status to reflect cancellation

