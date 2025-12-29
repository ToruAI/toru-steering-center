## ADDED Requirements

### Requirement: Dual-Role Authentication
The system SHALL support two distinct authentication methods for Admin and Client roles.

#### Scenario: Admin login via Environment Variables
- **WHEN** the server starts with `ADMIN_USERNAME` and `ADMIN_PASSWORD` environment variables
- **THEN** the system accepts these credentials for admin access
- **AND** these credentials are NOT stored in the database

#### Scenario: Client login via Database
- **WHEN** a client user provides valid credentials
- **THEN** the system validates the password against the stored Argon2 hash
- **AND** grants access if the user account is active

### Requirement: Session Management
The system SHALL manage user sessions securely.

#### Scenario: Session creation
- **WHEN** a user successfully logs in
- **THEN** a cryptographically secure 32-byte hex session token is generated
- **AND** the session is valid for 7 days

#### Scenario: Session validation
- **WHEN** a request includes a session cookie
- **THEN** the system validates the session existence and expiry
- **AND** verifies the associated user account is still active (for clients)

### Requirement: Password Security
The system SHALL enforce strong password policies and secure storage.

#### Scenario: Password hashing
- **WHEN** a password is stored or updated
- **THEN** it MUST be hashed using Argon2 with a random salt

#### Scenario: Password complexity
- **WHEN** a user sets a new password
- **THEN** it MUST be at least 8 characters long
- **AND** contain at least one uppercase letter, one lowercase letter, one number, and one special character

### Requirement: Rate Limiting
The system SHALL rate limit login attempts to prevent brute force attacks.

#### Scenario: Login failure tracking
- **WHEN** a login attempt fails
- **THEN** the system records the attempt with timestamp, IP, and username
