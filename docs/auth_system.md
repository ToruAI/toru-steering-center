# Authentication System Documentation

## 1. Problem Statement
The application required a lightweight authentication system with two distinct roles:
1.  **Admin (Owner)**: Full access to system settings, script management, and user management.
2.  **Client (User)**: Restricted access, limited to viewing the dashboard, history, and executing pre-configured "Quick Actions".

**Constraints:**
-   Self-hosted environment.
-   No SMTP server available (no email password resets).
-   Must be simple to deploy (single binary preference).

## 2. Solution Overview
We implemented a **Hybrid Authentication System**:
-   **Admin Credentials**: Stored in environment variables (`ADMIN_USERNAME`, `ADMIN_PASSWORD`). This ensures the owner always has access without database dependency for the root account.
-   **Client Credentials**: Stored in the SQLite database (`users` table).
-   **Session Management**: Server-side sessions stored in SQLite, identified by an HttpOnly cookie.

## 3. Technical Implementation

### Backend (Rust/Axum)

#### Database Schema
Three tables support the authentication system:
```sql
-- Client users (Admin is NOT stored here)
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,  -- Argon2 hash
    display_name TEXT,
    role TEXT NOT NULL DEFAULT 'client',
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

-- Active sessions
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT,                 -- NULL for Admin
    user_role TEXT NOT NULL,      -- 'admin' or 'client'
    username TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

-- Login attempts (for rate limiting and audit)
CREATE TABLE login_attempts (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    ip_address TEXT,
    success INTEGER NOT NULL,
    failure_reason TEXT,
    attempted_at TEXT NOT NULL
);
```

#### Authentication Flow
1.  **Login (`POST /api/auth/login`)**:
    *   Checks rate limiting (see below).
    *   Checks env vars for Admin match.
    *   If no match, checks `users` table (verifying Argon2 hash).
    *   On success, creates a session record and returns a `session_id` HttpOnly cookie (7-day duration).
    *   Records login attempt (success or failure) for audit.
2.  **Middleware / Extractors**:
    *   `AuthUser`: Validates session cookie, fetches user info. Used for general access.
    *   `AdminUser`: Wraps `AuthUser` and enforces `role == Admin`. Used for sensitive routes.

#### Security Measures
-   **Argon2**: Industry-standard password hashing for client users.
-   **Constant-Time Comparison**: Admin password verification uses constant-time comparison to prevent timing attacks.
-   **Cryptographically Secure Session Tokens**: 32-byte random tokens (not UUIDs).
-   **HttpOnly Cookies**: Prevents XSS attacks from stealing session tokens.
-   **Strict Cookie Policy**: Cookies use `SameSite=Strict` and `Secure` (in production) to mitigate CSRF.
-   **Session Validation**: 
    -   Checks expiration and user active status on every request.
    -   WebSocket connections are re-validated every 5 minutes.
    -   Sessions are invalidated immediately on password change.
-   **WebSocket Authentication**: WS connections require valid session.
    -   **Admin**: Can execute *any* script in the `scripts_dir`.
    -   **Client**: Can only execute scripts that are registered as **Quick Actions**.
-   **Rate Limiting**: Hybrid IP + Username tracking with exponential backoff:
    -   3 failures → 1 minute lockout
    -   6 failures → 3 minutes lockout
    -   9 failures → 10 minutes lockout
    -   12 failures → 30 minutes lockout
    -   *Mitigates username enumeration via lockout timing differences.*
-   **Password Requirements**: Minimum 8 characters AND complexity (Upper, Lower, Number, Special).
-   **Session Cleanup**: Expired sessions are cleaned on server startup.

### Frontend (React)

#### Context & State
-   `AuthContext`: Manages global user state (`user`, `loading`, `login`, `logout`).
-   `api.me()`: Called on app load to validate the existing session cookie.
-   **Global 401 Handler**: Automatically redirects to login on session expiry.

#### Routing
-   `ProtectedRoute` wrapper component:
    *   Redirects unauthenticated users to `/login`.
    *   Redirects non-admin users away from admin-only routes (`/scripts`, `/settings`).
-   `Layout`: Dynamically filters navigation items based on the user's role.

## 4. Usage Guide

### Environment Setup
Required environment variables:
```bash
ADMIN_USERNAME=admin      # Optional, defaults to "admin"
ADMIN_PASSWORD=change_me  # REQUIRED

# Production settings (optional)
PRODUCTION=1              # Enables Secure flag on cookies
# or
SECURE_COOKIES=1          # Alternative to PRODUCTION
```

### Managing Users
1.  Log in as Admin.
2.  Navigate to **Settings**.
3.  Use the "User Management" card to:
    *   Create new client users (password min 8 chars + complexity).
    *   Reset passwords (direct overwrite, no email flow).
    *   Deactivate/Delete users.

### Security Monitoring
1.  Log in as Admin.
2.  Navigate to **Settings**.
3.  View the "Security & Login History" card to:
    *   See recent login attempts (success/failure).
    *   Monitor for suspicious activity.
    *   Check 24-hour failure counts.

### Self-Service Password Change
Client users can change their own password via `PUT /api/me/password`:
```json
{
  "current_password": "old_password",
  "new_password": "new_password_min_8_chars"
}
```

## 5. API Endpoints

### Auth Routes (`/api/auth`)
| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| POST | `/login` | None | Authenticate user |
| POST | `/logout` | Any | End session |
| GET | `/me` | Any | Get current user info |
| GET | `/login-history` | Admin | Get login attempt history |

| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| GET | `/quick-actions` | Any | List available quick actions |
| POST | `/quick-actions` | Admin | Create quick action |
| DELETE | `/quick-actions/:id` | Admin | Delete quick action |
| POST | `/quick-actions/:id/execute` | Any | Execute a quick action (returns task_id) |

### User Management (`/api/users`) - Admin Only
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/users` | List all users |
| POST | `/users` | Create user |
| GET | `/users/:id` | Get user |
| PUT | `/users/:id` | Update user |
| DELETE | `/users/:id` | Delete user |
| PUT | `/users/:id/password` | Reset user password |

### Self-Service
| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| PUT | `/me/password` | Any (client) | Change own password |
