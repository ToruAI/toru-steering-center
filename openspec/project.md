# Project Context

## Purpose
Toru Steering Center is a self-hosted, lightweight VPS control panel designed for client transparency. It provides a dashboard for clients and stakeholders to monitor system resources, execute scripts, and manage infrastructure without deep technical knowledge.

**Key Goals:**
- Client-friendly visibility into VPS infrastructure
- Real-time system monitoring and alerting
- Script execution with live terminal output via WebSocket
- Dual-role authentication (Admin + Client users)
- No vendor lock-in - runs as a single binary on your VPS

## Tech Stack

### Backend
- **Language:** Rust (stable)
- **Web Framework:** Axum 0.7
- **Database:** SQLite (rusqlite 0.31, bundled)
- **Async Runtime:** Tokio 1
- **Auth:** Argon2 password hashing
- **System Monitoring:** sysinfo 0.30

### Frontend
- **Framework:** React 19 + React Router 7
- **Language:** TypeScript 5.9
- **Build Tool:** Vite 7
- **Styling:** Tailwind CSS 3.4 + shadcn/ui
- **Components:** Radix UI primitives
- **Charts:** Recharts
- **Icons:** Lucide React

### Deployment
- Single binary (Rust compiles frontend via rust-embed)
- Resource usage: <100MB RAM
- No Docker required
- Systemd service support
- Reverse proxy recommended (Nginx/Caddy/Cloudflare Tunnel)

## Project Conventions

### Code Style
- **Rust:** Follow standard Rust conventions, use `cargo fmt` and `cargo clippy`
- **TypeScript:** Strict mode enabled, ES2022 target
- **React:** Functional components with hooks, avoid class components
- **Naming:**
  - Rust: snake_case for functions/variables, PascalCase for types
  - TypeScript: camelCase for functions/variables, PascalCase for components/types
- **Imports:** Use `@/` alias for frontend src directory

### Architecture Patterns
- **Layered Backend:** routes → services → database
- **Embedded Assets:** Frontend compiled into Rust binary
- **Session Auth:** Secure HttpOnly cookies with configurable SameSite
- **WebSocket Protocol:** Messages follow `{type, data}` format (run, cancel, started, stdout, stderr, exit, cancelled, error)
- **API Design:** RESTful endpoints under `/api/`

### Testing Strategy
- Manual testing currently (no automated test suite)
- Backend services are testable (clear dependency boundaries)
- Future: Consider Vitest for frontend, Rust test modules for backend

### Git Workflow
- Main branch: `main`
- Feature branches for new work
- Commit messages: conventional commits preferred (feat:, fix:, docs:, etc.)

## Domain Context

### User Roles
- **Admin:** Full access, managed via environment variables (ADMIN_USERNAME, ADMIN_PASSWORD)
- **Client:** Limited access, database-backed accounts with Argon2 hashing

### Core Features
- **Dashboard:** System stats overview, quick actions grid
- **System Monitor:** CPU, memory, disk, network metrics with live updates
- **Scripts:** Execute shell scripts with real-time WebSocket output
- **Quick Actions:** Pre-approved scripts with icons for one-click execution
- **History:** Execution audit log with output details
- **Settings:** User management (admin), password changes (all users)

### Database Schema (SQLite)
- `settings` - Key-value configuration
- `users` - Client accounts (username, password_hash, role, display_name)
- `sessions` - Session tokens with expiry
- `login_attempts` - Audit log for rate limiting
- `task_history` - Script execution history
- `quick_actions` - Pre-approved script shortcuts

## Important Constraints

### Security
- Rate limiting: Graduated lockout (1→3→10→30 min after failed attempts)
- HTTPS required in production (via reverse proxy)
- Secure cookies when `PRODUCTION=true`
- Scripts must be pre-approved (no arbitrary command execution)

### Performance
- Target: <100MB RAM usage
- Avoid unnecessary dependencies
- Single-threaded SQLite (no concurrent writes)

### UX
- All features must work on mobile (touch-friendly)
- Use shadcn/ui components for consistency
- Follow ToruAI branding guidelines

## External Dependencies

### Environment Variables
| Variable | Required | Description |
|----------|----------|-------------|
| `ADMIN_PASSWORD` | Yes | Admin account password |
| `ADMIN_USERNAME` | No | Admin username (default: admin) |
| `STEERING_PORT` | No | Server port (default: 3000) |
| `STEERING_HOST` | No | Bind address (default: 127.0.0.1) |
| `PRODUCTION` | No | Enable secure cookies |
| `SCRIPTS_DIR` | No | Scripts directory path |

### Branding (ToruAI)
- **Primary:** #493FAA (deep violet)
- **Primary Hover:** #7E64E7
- **Background:** #F9F9F9 (offwhite), #FFFFFF
- **Text:** #191919 (primary), #494949 (secondary)

## Build Commands

```bash
# Frontend
cd frontend && npm install && npm run build

# Backend (release)
export ADMIN_PASSWORD="your-secure-password"
cargo build --release
./target/release/steering-center

# Development
# Terminal 1: cd frontend && npm run dev
# Terminal 2: export ADMIN_PASSWORD="dev" && cargo run
```
