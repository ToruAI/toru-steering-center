# Steering Center

**Control center for your digital assets.**

![Steering Center Dashboard](image.png)

An open-source, lightweight control panel for VPS deployments. Built by [ToruAI](https://toruai.com) for client transparency - every managed VPS gets one, so clients always know what's running on their infrastructure.

## Why Steering Center?

Most server dashboards are built for sysadmins. Steering Center is built for **clients and stakeholders** - people who need visibility and control without the complexity.

| Traditional Tools | Steering Center |
|-------------------|-----------------|
| Designed for ops teams | Designed for clients |
| System metrics only | System + business data (KPIs, agents, conversions) |
| Complex setup | Single binary, runs in 2 minutes |
| Feature bloat | Minimal starter - add only what you need |
| Technical UI | Clean, modern, client-friendly |

**No vendor lock-in.** It runs on your VPS, you own everything.

## What Can You Build?

This is a foundation. Out of the box you get system monitoring and script execution. Fork it and add:

- **Docker dashboards** - container status, logs, controls
- **Database metrics** - PostgreSQL, Redis, MongoDB stats
- **AI agent monitoring** - track your autonomous agents
- **Business KPIs** - conversions, revenue, user activity
- **Custom pages** - build dashboards for any data source
- **Alerts & notifications** - Slack, email, webhooks

## Prerequisites

- **Rust** - Latest stable ([install](https://rustup.rs))
- **Node.js 20+** - With npm ([install](https://nodejs.org))
- **Unix-like OS** - Linux or macOS

## Quick Start

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

Open `http://localhost:3000`

## Authentication

**Toru Steering Center** uses a hybrid authentication system to ensure security without external dependencies.

- **Admin (Owner)**: Authenticated via environment variables (`ADMIN_USERNAME` / `ADMIN_PASSWORD`). Has full access to settings, user management, and arbitrary script execution.
- **Client (User)**: Authenticated via SQLite database. Managed by Admin via UI. Limited to viewing dashboards and running pre-approved "Quick Actions".

### Credentials

| Role | Username | Password | Storage |
|------|----------|----------|---------|
| **Admin** | `admin` (default) | Set via `ADMIN_PASSWORD` env var | Environment (Stateless) |
| **Client** | Custom | Set by Admin | SQLite (Argon2 Hash) |

## Tech Stack

| Layer | Technology |
|-------|------------|
| Backend | Rust (Axum) + SQLite |
| Frontend | React + TypeScript + Vite |
| Styling | Tailwind CSS + shadcn/ui |
| Real-time | WebSocket streaming |

**Resource usage:** Under 100MB RAM. Single binary. No Docker required.

## Deploy to VPS

Since authentication is built-in, you can deploy securely without a VPN, though a reverse proxy with SSL (Nginx/Caddy/Cloudflare) is **highly recommended** for production.

### Systemd Service

```ini
[Unit]
Description=Steering Center
After=network.target

[Service]
Type=simple
WorkingDirectory=/path/to/steering-center
# Set your secure admin password here
Environment="ADMIN_PASSWORD=change_this_to_something_secure"
# Optional: Enable secure cookies if behind HTTPS
Environment="PRODUCTION=true"
ExecStart=/path/to/steering-center/target/release/steering-center
Restart=always

[Install]
WantedBy=multi-user.target
```

## Configuration

### CLI Options

```bash
steering-center [OPTIONS]

Options:
  -p, --port <PORT>    Port to listen on [default: 3000]
  -H, --host <HOST>    Host to bind to [default: 127.0.0.1]
  -h, --help           Print help message
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ADMIN_USERNAME` | `admin` | Admin username |
| `ADMIN_PASSWORD` | **REQUIRED** | Admin password (must be set) |
| `STEERING_HOST` | `127.0.0.1` | Bind address (`0.0.0.0` for external) |
| `STEERING_PORT` | `3000` | Server port |
| `PRODUCTION` | `false` | Set to `true` to enable Secure cookies |
| `RUST_LOG` | `info` | Log level |

CLI options take priority over environment variables.

## Project Structure

```
/frontend    # React dashboard (Vite + TypeScript + shadcn/ui)
/src         # Rust backend (Axum API + WebSocket)
/scripts     # Shell scripts for execution
```

## API

| Endpoint | Description |
|----------|-------------|
| `GET /api/resources` | CPU, RAM, storage, uptime |
| `GET /api/scripts` | Available scripts |
| `POST /api/quick-actions` | Create one-click actions |
| `GET /api/history` | Execution history |
| `WS /api/ws` | Real-time terminal output |

## Philosophy

- **Transparency** - Clients see exactly what runs on their infrastructure
- **Ownership** - No SaaS lock-in, everything on their VPS
- **Simplicity** - Start minimal, extend as needed
- **Cost optimization** - Lightweight enough for the smallest VPS

## License

MIT - Use it however you want.

---

Built with care by [ToruAI](https://toruai.com)
