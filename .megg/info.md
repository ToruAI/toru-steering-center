---
created: 2025-12-15T09:25:00.581Z
updated: 2025-12-30T12:45:00.000Z
type: context
---
# Toru Steering Center

## Overview
Self-hosted dashboard template for monitoring system resources and executing scripts with real-time terminal output. Designed for ToruAI clients running on VPS behind Cloudflare tunnels (localhost only).

**Business Model:**
- Open source core (community can fork/modify)
- Proprietary plugins delivered to paying clients (compiled binaries)
- True ownership (clients keep everything when they stop paying)
- No vendor lock-in (clients own their deployment)
- Semi-automated deployment (maintainer deploys and manages all VPS instances)

## Architecture
- **Monolith binary**: Rust backend serves both API and static frontend
- **Backend**: Rust (Axum) + SQLite - chosen for robustness and low resource usage (<100MB RAM)
- **Frontend**: Vite + React + TypeScript + Tailwind CSS + shadcn/ui
- **Communication**: REST API + WebSocket for real-time streaming
- **Security**: Hybrid Auth (Env for Admin, DB for Clients) + Hardened (IP Rate Limit, Strict Cookies)
- **Extensibility**: Process-isolated plugin system with Unix socket IPC (current implementation: in proposal phase)

## Branding (from ToruAI)
- **Primary**: #493FAA (deep violet)
- **Primary Hover**: #7E64E7 (lighter violet)
- **Logo colors**: #493FAA, #7E64E7, #8E7EF7, #A294FF
- **Background**: #F9F9F9 (offwhite), #FFFFFF
- **Text**: #191919 (primary), #494949 (secondary)
- **Favicon**: Use T logo (favicone_bialetlo.png style - white background)

## Build & Run
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

Server runs on http://localhost:3000

## Project Structure
- `/src` - Rust backend (routes, services, database)
- `/frontend` - React frontend (pages, components, hooks)
- `/scripts` - Shell scripts (configurable directory)
- `/docs` - Design documentation
- `/openspec` - Spec-driven development (proposals, specs, tasks)
- `/openspec/changes/add-dynamic-plugin-system` - Current active change: Process-isolated plugin system

## Rules
- Code must be production-quality and work seamlessly
- Use shadcn/ui components for all UI elements
- Follow existing patterns in the codebase
- Keep the binary lightweight - avoid unnecessary dependencies
- All features must work on mobile (touch-friendly)
- WebSocket messages follow the established protocol (run, cancel, started, stdout, stderr, exit, cancelled, error)

## Active Work: Plugin System

**Status:** Proposal validated (2025-12-30), implementation pending

**Architecture Decision:** Process-isolated plugins (NOT dynamic libraries)
- Each plugin runs as separate process
- Communication via Unix domain sockets (microsecond overhead)
- JSON protocol (stable, language-agnostic)
- Crash isolation: plugin failure doesn't crash core
- Auto-restart with exponential backoff
- Instance-locked licensing (HMAC-signed keys, offline)

**Why this architecture:**
- Server stability is critical (maintainer deploys all instances)
- Plugins need full system access (shell, files, network, DB)
- Proprietary plugins in Rust, community plugins in Rust or Python
- Observability for TORIS (structured logs, process metrics)
- No ABI issues (protocol is stable JSON)

**Deployment Model:**
- Maintainer builds proprietary plugins via GitHub Actions
- Plugins are `.binary` files dropped into `./plugins/`
- Core spawns processes on startup, enables/disables dynamically
- No server restart required to add/remove plugins
- TORIS watches `/var/log/toru/plugins/` for observability

**See `openspec/changes/add-dynamic-plugin-system/`** for full proposal, design, and tasks.

## TORIS Integration
- TORIS is an observability agent (open-source based)
- Watches log files (structured JSON format)
- Monitors system health, plugin status
- Will also have its own interface
- Plugin supervisor logs to `/var/log/toru/plugin-supervisor.log`
- Plugin logs to `/var/log/toru/plugins/<id>.log`

## OpenSpec Workflow
Use `openspec` commands for spec-driven development:
- `openspec list` - See active changes
- `openspec show <change-id>` - View proposal details
- `openspec validate <change-id> --strict` - Validate proposals
- `openspec archive <change-id>` - Mark change complete (after deployment)
