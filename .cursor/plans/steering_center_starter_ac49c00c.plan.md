---
name: Steering Center Starter
overview: Build a complete open-source starter template for self-hosted Rust + React dashboards with real-time WebSocket terminal, system monitoring, and mobile-first UI.
todos:
  - id: rust-init
    content: Initialize Rust project with Cargo.toml and dependencies
    status: pending
  - id: frontend-init
    content: Initialize Vite + React + TypeScript frontend with Tailwind and shadcn/ui
    status: pending
  - id: backend-core
    content: Implement main.rs with Axum server, state management, and static file serving
    status: pending
  - id: database
    content: Implement db.rs with SQLite schema and query functions
    status: pending
  - id: api-routes
    content: Implement REST API routes (health, resources, scripts, settings, history, quick-actions)
    status: pending
  - id: websocket
    content: Implement WebSocket handler for real-time script execution and cancellation
    status: pending
  - id: services
    content: Implement system.rs and executor.rs services
    status: pending
  - id: frontend-layout
    content: Create Layout component with mobile navigation and React Router setup
    status: pending
  - id: frontend-dashboard
    content: Implement Dashboard page with system stats and quick actions
    status: pending
  - id: frontend-scripts
    content: Implement Scripts page with terminal output and WebSocket integration
    status: pending
  - id: frontend-settings
    content: Implement Settings page with configuration forms
    status: pending
  - id: frontend-hooks
    content: Create useWebSocket and useSystemStats hooks
    status: pending
  - id: example-scripts
    content: Create example shell scripts in /scripts directory
    status: pending
  - id: documentation
    content: Write comprehensive README.md with setup and extension guides
    status: pending
---

# Steering Center - Open Source Starter Template

## Architecture Overview

**Monolith binary** serving both API and static frontend:

- Backend: Rust (Axum) with SQLite
- Frontend: Vite + React + TypeScript + shadcn/ui
- Communication: REST API + WebSocket for real-time streaming

## Phase 1: Project Foundation

### 1.1 Initialize Rust Backend

- Create Cargo project with dependencies:
  - `axum` (web framework + WebSocket)
  - `tokio` (async runtime)
  - `serde` / `serde_json` (serialization)
  - `rusqlite` (SQLite)
  - `sysinfo` (system monitoring)
  - `tower-http` (static file serving, CORS)
  - `tracing` (logging)
  - `uuid` (task IDs)

### 1.2 Initialize React Frontend

- Vite + React + TypeScript template
- Install: Tailwind CSS, shadcn/ui, lucide-react, react-router-dom
- Configure build output to `frontend/dist`

### 1.3 Project Structure

```
/my-dashboard
├── /frontend
│   ├── /src
│   │   ├── /components    # UI components
│   │   ├── /pages         # Dashboard, Scripts, Settings
│   │   ├── /hooks         # useWebSocket, useSystemStats
│   │   ├── /lib           # API client, utilities
│   │   └── main.tsx
│   ├── package.json
│   └── vite.config.ts
├── /src                   # Rust backend
│   ├── main.rs
│   ├── /routes
│   ├── /services
│   └── db.rs
├── /scripts               # Example scripts
├── Cargo.toml
└── README.md
```

## Phase 2: Backend Implementation

### 2.1 Core Server (`main.rs`)

- Initialize tracing, database, system monitor
- Create shared `AppState` (db pool, task registry, sysinfo)
- Mount API routes under `/api`
- Serve static files from `frontend/dist`
- SPA fallback (all non-API routes serve `index.html`)

### 2.2 Database (`db.rs`)

```sql
-- settings: key-value config
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);

-- task_history: execution log
CREATE TABLE task_history (
  id TEXT PRIMARY KEY,
  script_name TEXT,
  started_at TEXT,
  finished_at TEXT,
  exit_code INTEGER,
  output TEXT
);

-- quick_actions: customizable buttons
CREATE TABLE quick_actions (
  id TEXT PRIMARY KEY,
  name TEXT,
  script_path TEXT,
  icon TEXT,
  display_order INTEGER
);
```

### 2.3 Routes

| Endpoint | Method | Description |

|----------|--------|-------------|

| `/api/health` | GET | Health check |

| `/api/resources` | GET | CPU, RAM, uptime |

| `/api/scripts` | GET | List available scripts |

| `/api/settings` | GET/PUT | App configuration |

| `/api/history` | GET | Task execution history |

| `/api/quick-actions` | GET/POST/DELETE | Manage quick actions |

| `/api/ws` | WebSocket | Real-time terminal |

### 2.4 WebSocket Protocol (`ws.rs`)

**Client sends:**

- `{"type": "run", "script": "backup.sh"}`
- `{"type": "cancel", "task_id": "uuid"}`

**Server sends:**

- `{"type": "started", "task_id": "uuid"}`
- `{"type": "stdout", "data": "line of output"}`
- `{"type": "stderr", "data": "error line"}`
- `{"type": "exit", "code": 0}`
- `{"type": "cancelled"}`

### 2.5 Services

- `system.rs`: CPU/RAM/uptime via sysinfo crate
- `executor.rs`: Spawn child processes, stream output, handle cancellation

## Phase 3: Frontend Implementation

### 3.1 Core Setup

- React Router with 3 routes: `/`, `/scripts`, `/settings`
- Layout component with mobile navigation
- API client module for REST endpoints

### 3.2 Dashboard Page (`/`)

- System stats cards (CPU %, RAM %, Uptime)
- Auto-refresh every 2 seconds
- Quick actions grid (from database)

### 3.3 Scripts Page (`/scripts`)

- Script selector (dropdown or list)
- Terminal output area (monospace, dark theme)
- Run/Cancel buttons
- WebSocket connection management
- Auto-scroll to bottom during execution

### 3.4 Settings Page (`/settings`)

- Scripts directory path input
- Quick actions management (add/remove/reorder)
- Save button with feedback

### 3.5 Components

- `SystemStatCard` - Individual stat display
- `Terminal` - Output display with streaming
- `QuickActionButton` - Script launcher
- `Layout` - Navigation + content wrapper

### 3.6 Hooks

- `useWebSocket` - Connection lifecycle, message handling
- `useSystemStats` - Polling `/api/resources`

## Phase 4: Integration and Polish

### 4.1 Build Pipeline

- Frontend: `npm run build` outputs to `frontend/dist`
- Backend: `cargo build --release` produces single binary
- Binary serves static files and API from same port (3000)

### 4.2 Example Scripts

Create `/scripts` directory with examples:

- `system-info.sh` - Display system information
- `disk-usage.sh` - Show disk usage
- `update-check.sh` - Check for system updates

### 4.3 Documentation

- README.md with:
  - Quick start guide
  - Architecture overview
  - How to add new API endpoints
  - How to add new pages
  - Deployment guide (Cloudflare Zero Trust)
  - Configuration options

### 4.4 .gitignore

- `target/` (Rust build)
- `frontend/node_modules/`
- `frontend/dist/`
- `steering.db`
- `.env`

## Deliverables

1. Complete Rust backend with all routes and WebSocket
2. Complete React frontend with 3 pages
3. SQLite database schema
4. Example scripts
5. Comprehensive README
6. Clean, well-commented code suitable for open-source