---
created: 2025-12-15T09:58:30.000Z
updated: 2025-12-16T10:40:37.514Z
type: memory
---
# Architectural Decisions

## 2024-12-15: Frontend Asset Embedding

### Context
The project goal is a single deployable binary. Initial implementation used `ServeDir::new("frontend/dist")` which looks for files at runtime relative to the current working directory.

### Problem
When the binary is moved or run from a different location (e.g., `./target/release/steering-center`), it can't find `frontend/dist/` and serves a blank white page. The build script masked this by always running from project root.

### Decision
Use `rust-embed` to embed frontend assets into the binary at compile time.

### Consequences
- **Build order matters**: Frontend must be built BEFORE `cargo build` since assets are embedded at compile time
- **Binary size increases**: All frontend assets are bundled into the executable
- **True portability**: Binary can be copied anywhere and run without external dependencies (except SQLite db file)
- **Development workflow changes**: Need to rebuild Rust after frontend changes to see updates in release binary

### Alternatives Considered
1. **Require specific working directory** - Rejected: fragile, bad UX
2. **Config file for asset path** - Rejected: adds deployment complexity, defeats "single binary" goal
3. **Embed assets with `include_dir!`** - Viable but `rust-embed` has better ergonomics and mime type handling

### Status
Implementing `rust-embed` solution.

## 2025-12-15T10:20:05.902Z

## 2024-12-15: v0.1.0 Release Fixes

### Task Cancellation Architecture

**Problem**: Original design stored child process in registry, then immediately removed it to get stdout/stderr handles. This meant cancellation couldn't find the process.

**Solution**: Changed `TaskRegistry` from `HashMap<String, Child>` to `HashMap<String, Arc<Mutex<Option<Child>>>>`. Now:
- Take stdout/stderr handles BEFORE storing in registry
- Child stays in registry during execution
- Cancellation can find and kill the process
- Clean up registry after task completes

**Pattern**: When you need both streaming access AND cancellation, separate the I/O handles from the process handle early.

### Server Binding Security

**Decision**: Default to `127.0.0.1` (localhost only), not `0.0.0.0`.

**Rationale**: The design doc explicitly states "localhost only, Cloudflare handles external traffic". Binding to all interfaces by default is a security risk for a tool that executes shell scripts.

**Override**: `STEERING_HOST=0.0.0.0` for users who need external access.

### Quick Actions UX

**Decision**: Quick Actions navigate to Scripts page with script pre-selected rather than executing inline on Dashboard.

**Rationale**: 
- Keeps terminal output in one place (Scripts page)
- User sees what's about to run before execution
- Simpler implementation, consistent UX
- Dashboard stays clean (overview, not execution)

### History Page

**Added**: `/history` route showing last 100 executions with:
- Expandable output view
- Status badges (Running/Success/Failed)
- Re-run button per task
- Task ID and timestamps

**Note**: Output is stored in SQLite. For long-running scripts with lots of output, this could grow the DB. Consider adding output truncation or retention policy in future versions.

## 2025-12-16: Hybrid Authentication Implementation

### Context
User requested a simple authentication system with two roles: Admin (owner) and Client (viewer).

### Decision
Implemented a hybrid approach:
- **Admin**: Authenticated via environment variables (`ADMIN_USERNAME`/`ADMIN_PASSWORD`). No database record for admin credentials to prevent lockout and maintain simplicity.
- **Clients**: Authenticated via SQLite `users` table. Managed by Admin via UI.
- **Session**: Expirable SQLite-backed sessions with HttpOnly cookies.

### Consequences
- **Security**: Improved. Dashboard is no longer public on localhost.
- **UX**: Admin can now safely create restricted "view-only" accounts for clients.
- **Complexity**: Added `argon2` dependency and auth middleware.


## 2025-12-16T10:40:37.514Z
## 2025-12-16: Authentication Security Hardening

### Context
Code review identified several security issues in the initial auth implementation.

### Changes Made
1. **Session Tokens**: Replaced UUID with 32-byte cryptographically secure random tokens
2. **Rate Limiting**: Added exponential backoff (3→1min, 6→3min, 9→10min, 12→30min)
3. **Login Audit**: New `login_attempts` table tracks all auth attempts with IP, timestamp, result
4. **Secure Cookies**: Added `Secure` flag when `PRODUCTION` or `SECURE_COOKIES` env var is set
5. **WebSocket Auth**: WS connections now require valid session cookie; only admins can run scripts
6. **Password Validation**: Minimum 8 characters enforced on create/reset
7. **Self-Service Password**: Users can change their own password via `/api/me/password`
8. **Session Cleanup**: Expired sessions cleaned on server startup

### Frontend Changes
- Global 401 handler auto-redirects to login on session expiry
- Login page shows lockout countdown timer
- New LoginHistory component in Settings page
- Mobile navigation replaced with Sheet component (slide-out drawer)

### Security Trade-offs
- Admin password still in env var (plaintext) - acceptable for self-hosted app where server access = full control anyway
- No CSRF tokens - mitigated by SameSite=Strict cookies (upgraded from Lax in Round 2)

## 2025-12-16: Security Hardening (Round 2)

### Context
Detailed security review revealed vulnerabilities (Timing attack, Rate limit bypass) after initial hardening.

### Changes
1. **Timing Attack Fix**: Implemented constant-time comparison for admin authentication.
2. **IP Rate Limiting**: Added IP-based checks effectively mitigating username enumeration.
3. **Session Security**: Upgraded to `SameSite=Strict` and added immediate invalidation on password change.
4. **WebSocket**: Added periodic session re-validation.
5. **Password Policy**: Enforced complexity (Upper, Lower, Number, Special).

### Revised Trade-offs
- **SameSite=Strict**: Improved CSRF protection but requires re-login when navigating from external sources (e.g. email links).
- **Complexity**: Added `subtle` dependency and stricter policies.