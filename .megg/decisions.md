---
created: 2025-12-15T09:58:30.000Z
updated: 2025-12-29T22:33:09.223Z
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

## 2025-12-29T22:21:15.538Z
## Plugin System Architecture Decision (2025-12-29)

### Context
Initially designed a WASM-based plugin system with:
- wasmtime runtime
- Capability-based security (http, kv_store, system_info permissions)
- JSON UI spec for frontend
- Fuel limits and memory isolation
- Complex host function interface

### Why We Changed
The WASM design was **over-engineered** for the actual business model:

**Business Model:**
- Open source core (anyone can fork/modify)
- Proprietary plugins delivered to paying clients as compiled binaries
- **True ownership** - when clients stop paying, everything keeps working forever
- **No vendor lock-in** - clients own their deployment (binaries, not source code)
- Community can also build their own plugins

**Key Insight:** We don't need sandboxing because:
1. Our proprietary plugins = we trust our own code
2. Community plugins = users choose to trust them (like npm, WordPress)
3. It's their VPS, their responsibility

### Decision: Dynamic Libraries (.so files)

**Chosen approach:**
- `libloading` crate for loading .so files from `./plugins/`
- `ToruPlugin` trait in separate public crate
- Instance-locked licensing (HMAC-signed keys tied to instance UUID)
- WordPress-style frontend: one menu button, one view, plugin has full freedom inside

**What we gain:**
- 90% simpler codebase
- Native Rust performance (no WASM overhead)
- Normal development experience (debugging, IDE support)
- Full React components (no JSON UI limitations)

**What we intentionally lose:**
- Sandboxing (not needed - trust model)
- Capability security (not needed - trusted code)
- Cross-platform (Linux only - our target)

### Licensing Model
```
1. Toru generates instance UUID on first run (stored in DB)
2. Client sends instance ID to us
3. We generate HMAC-signed license key
4. Plugin validates: key must match instance ID
```

When client stops paying:
- Everything keeps working forever ✓
- They can't share plugin (tied to their instance) ✓
- They don't get updates ✓
- True ownership achieved ✓

### Frontend Philosophy
**WordPress-style:**
- One sidebar entry per plugin (icon + name)
- One route per plugin
- Plugin has FULL CONTROL inside its container
- JS bundle with `mount(container, api)` / `unmount(container)`
- Plugin can use React, Vue, vanilla JS - anything

### OpenSpec Reference
See `openspec/changes/add-dynamic-plugin-system/` for full proposal, design, tasks, and specs.

## 2025-12-29T22:21:33.023Z
## Core Philosophy & Mindset (2025-12-29)

### Ownership Over Lock-in
The fundamental value proposition of Toru Steering Center:
- Clients **own** their deployment
- When they stop paying, nothing breaks
- They own binaries, not source code (for proprietary parts)
- No vendor lock-in, no hostage situations

### Simplicity First
- Favor straightforward solutions over clever ones
- Don't build for hypothetical future needs
- WASM was a mistake - it solved problems we don't have
- Dynamic libraries solve our actual problem simply

### Trust Model
We don't need sandboxing because:
- We trust our own proprietary plugin code
- Users trust what they choose to install
- This is the same model as npm, Docker, WordPress, VS Code extensions
- It's their VPS - their choice, their responsibility

### Target: Linux VPS
- No need for cross-platform (.dylib, .dll)
- Keep it simple - .so files work perfectly
- Our clients run on Linux VPS behind Cloudflare tunnels

### Practical Over Perfect
- Don't let perfect be the enemy of good
- Ship something that works
- Iterate based on real needs, not imagined ones

## 2025-12-29T22:33:09.223Z
## Quality Strategy Decision (2025-12-29)

### Philosophy
80% of TDD benefits with 20% of the effort.

### What We Test (Critical Paths Only)
- Security: Auth, license validation
- Data integrity: DB operations, state persistence
- Integration points: Plugin loading, API contracts
- Error handling: Graceful failures, no panics

### What We Skip
- UI layout/styling
- Simple CRUD
- Trivial getters/setters
- 100% coverage (diminishing returns)

### Quality Gates (Every Change)
1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo test`
4. Code review for security-sensitive code

### Per-Task Workflow
```
IMPLEMENT → COMPILE → TEST (if critical) → CLIPPY → REVIEW (if significant)
```

### Integration Tests > Unit Tests
Test real flows, not mocked internals. A test that loads a real .so file is worth more than 10 unit tests with mocks.

### Compiler as First Defense
Use Rust's type system: newtypes, enums, Result types. Make invalid states unrepresentable.

### Reference
See `openspec/project.md` Quality Strategy section for full details.