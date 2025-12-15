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
# One command to build and run
./build-and-run.sh
```

Or manually:

```bash
cd frontend && npm install && npm run build && cd ..
cargo build --release
./target/release/steering-center
```

Open `http://localhost:3000`

**Custom port:**
```bash
./target/release/steering-center --port 8080
```

## Tech Stack

| Layer | Technology |
|-------|------------|
| Backend | Rust (Axum) + SQLite |
| Frontend | React + TypeScript + Vite |
| Styling | Tailwind CSS + shadcn/ui |
| Real-time | WebSocket streaming |

**Resource usage:** Under 100MB RAM. Single binary. No Docker required.

## Deploy to VPS

### Cloudflare Tunnel (Recommended)

```bash
# On your VPS
./build-and-run.sh

# Configure cloudflared to forward to localhost:3000
# Add authentication via Cloudflare Zero Trust
```

### Systemd Service

```ini
[Unit]
Description=Steering Center
After=network.target

[Service]
Type=simple
WorkingDirectory=/path/to/steering-center
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
| `STEERING_HOST` | `127.0.0.1` | Bind address (`0.0.0.0` for external) |
| `STEERING_PORT` | `3000` | Server port |
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
