# user-management Specification

## Purpose
TBD - created by archiving change init-specs. Update Purpose after archive.
## Requirements
### Requirement: User Administration
The system SHALL allow Admins to manage Client user accounts.

#### Scenario: List users
- **WHEN** an Admin requests the user list
- **THEN** the system returns all users with their roles and active status

#### Scenario: Create user
- **WHEN** an Admin creates a new user
- **THEN** the system verifies the username is unique
- **AND** validates the initial password strength
- **AND** stores the new user with `client` role by default

#### Scenario: Update user
- **WHEN** an Admin updates a user
- **THEN** they can modify the display name and active status
- **BUT** cannot change the username

#### Scenario: Delete user
- **WHEN** an Admin deletes a user
- **THEN** the user record and all associated sessions are removed

### Requirement: Administrative Password Reset
The system SHALL allow Admins to reset user passwords.

#### Scenario: Force password reset
- **WHEN** an Admin resets a user's password
- **THEN** the new password is hashed and stored
- **AND** all existing sessions for that user are invalidated

