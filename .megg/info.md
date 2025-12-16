---
created: 2025-12-15T09:25:00.581Z
updated: 2025-12-15T09:25:00.581Z
type: context
---
# Toru Steering Center

## Overview
Self-hosted dashboard template for monitoring system resources and executing scripts with real-time terminal output. Designed for ToruAI clients running on VPS behind Cloudflare tunnels (localhost only).

## Architecture
- **Monolith binary**: Rust backend serves both API and static frontend
- **Backend**: Rust (Axum) + SQLite - chosen for robustness and low resource usage (<100MB RAM)
- **Frontend**: Vite + React + TypeScript + Tailwind CSS + shadcn/ui
- **Communication**: REST API + WebSocket for real-time streaming
- **Security**: Hybrid Auth (Env for Admin, DB for Clients) + Hardened (IP Rate Limit, Strict Cookies)

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

## Rules
- Code must be production-quality and work seamlessly
- Use shadcn/ui components for all UI elements
- Follow existing patterns in the codebase
- Keep the binary lightweight - avoid unnecessary dependencies
- All features must work on mobile (touch-friendly)
- WebSocket messages follow the established protocol (run, cancel, started, stdout, stderr, exit, cancelled, error)