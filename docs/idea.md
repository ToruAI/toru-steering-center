# Project Design Document: The "Steering Center" Dashboard

## 1\. Problem Statement

The user needs a self-hosted "Steering Center" application running on a VPS to monitor system resources (CPU, RAM, Uptime) and orchestrate heavy backend tasks (pipelines, file movements).

**Constraints & Pain Points with existing solutions:**

  * **Next.js / Refine / Supabase:** Proven to be too resource-heavy (high RAM usage), complex to self-host (requires multi-container Docker setups), and introduces unnecessary boilerplate for a single-user tool.
  * **Low-Code Tools (Budibase/Directus):** While useful, they lack the "native" feel, are not optimized for specific mobile workflows, and introduce abstraction layers that make direct OS-level control (shell commands) cumbersome.
  * **Authentication:** Maintaining a custom auth system (JWTs, Sessions, Database Users) is a security risk and development burden for a personal tool.

**Core Requirements:**

1.  **Efficiency:** Must run on low-resource VPS hardware (< 100MB RAM).
2.  **Mobile-First:** The UI must be responsive and touch-friendly.
3.  **Direct Control:** The backend must have native access to the host OS to execute shell commands.
4.  **Zero-Code Auth:** Authentication must be offloaded to the infrastructure layer (Cloudflare Zero Trust).
5.  **Real-time Feedback:** Command execution must stream output in real-time via WebSocket.
6.  **Task Control:** Running tasks must be cancellable.

-----

## 2\. Architectural "Way of Thinking"

To solve for efficiency and control, we are adopting a **"Monolithic Binary"** architecture protected by an **"Auth Proxy"**.

### The Stack Selection

  * **Backend: Rust (Axum)**
      * *Why:* Rust provides memory safety and raw performance. Axum is a thin, ergonomic web framework that allows asynchronous task handling (essential for running pipelines without freezing the UI) and compiles to a single static binary.
  * **Frontend: Vite + React + shadcn/ui**
      * *Why:* React offers the best ecosystem for UI components. Vite provides a fast build process. `shadcn/ui` (built on Tailwind) ensures the app looks professional and works on mobile devices immediately without writing custom CSS from scratch.
  * **Database: SQLite**
      * *Why:* We need a place to store "pinned data" and logs. A serverless, file-based database avoids the need for a separate Docker container (Postgres/MySQL).
  * **Security: Cloudflare Zero Trust (Tunnel)**
      * *Why:* Instead of implementing login logic in Rust, we place the app behind a Cloudflare Tunnel. Cloudflare handles the Identity Provider (Google/Email) and only forwards requests to the app if the user is authenticated. The app assumes all incoming traffic is trusted.

### System Diagram

-----

## 3\. Technical Implementation Details

### A. Directory Structure

The project will be a single repository containing both the Rust backend and the React frontend.

```text
/steering-center
├── /frontend              # The Vite + React Application
│   ├── /src
│   │   ├── /components    # Reusable UI components
│   │   ├── /pages         # Page components (Dashboard, Scripts, Settings)
│   │   ├── /hooks         # Custom hooks (useWebSocket, etc.)
│   │   ├── /lib           # API fetchers, utilities
│   │   └── main.tsx       # Entry point with React Router
│   ├── package.json
│   └── vite.config.ts     # Configured to build to ../dist
├── /src                   # The Rust Backend
│   ├── main.rs            # Entry point & server definition
│   ├── routes/
│   │   ├── mod.rs         # Route aggregation
│   │   ├── health.rs      # Health check endpoint
│   │   ├── resources.rs   # System stats API
│   │   ├── scripts.rs     # Script listing & execution
│   │   ├── settings.rs    # Settings CRUD
│   │   └── ws.rs          # WebSocket handler for terminal
│   ├── services/
│   │   ├── mod.rs
│   │   ├── system.rs      # OS interaction logic (sysinfo)
│   │   └── executor.rs    # Process spawning & management
│   └── db.rs              # SQLite setup & queries
├── /scripts               # Default scripts directory (configurable)
├── Cargo.toml             # Rust dependencies
├── steering.db            # SQLite database (created at runtime)
└── README.md
```

### B. Backend Specifications (Rust)

**1. Dependencies**
The backend requires:
- Web server framework (Axum) with WebSocket support
- Async runtime (Tokio) with process spawning capabilities
- Serialization (Serde/JSON) for API communication
- Database (SQLite via rusqlite) for settings and task history
- System monitoring (sysinfo) for CPU/RAM stats
- UUID generation for task tracking
- Logging (tracing) for debugging

**2. Core Server Architecture**
The server will:
- Initialize system monitoring and database on startup
- Maintain shared application state (system monitor, database connection, running task registry)
- Expose REST API endpoints for health checks, system resources, script listing, and settings
- Handle WebSocket connections for real-time command execution
- Serve static frontend files with SPA fallback routing (all routes serve index.html)

**3. Database Schema**
The SQLite database will contain:
- **settings table**: Key-value pairs for application configuration (e.g., scripts_path)
- **task_history table**: Records of executed scripts with timestamps, exit codes, and output
- **quick_actions table**: User-defined quick action buttons with script paths, icons, and display order

**4. WebSocket Protocol**
The WebSocket handler will:
- Accept client messages: `run` (with script name) and `cancel` (with task_id)
- Stream server messages: `started`, `stdout`, `stderr`, `exit`, `cancelled`, `error`
- Spawn child processes for script execution
- Stream stdout/stderr line-by-line to connected clients
- Support task cancellation via signal channels
- Track running tasks in memory for cancellation lookups

### C. Frontend Specifications (React + Vite)

**1. Setup**
Use the standard Vite React template with TypeScript for type safety and better developer experience.

**2. UI Framework**
- Tailwind CSS for styling
- shadcn/ui component library (built on Tailwind) for professional, mobile-ready components
- Lucide React for icons
- React Router for multi-page navigation

**3. Page Structure**
The frontend will have multiple pages:
- **Dashboard Page (`/`)**: Displays system resources (CPU, RAM) with auto-refresh every 2 seconds
- **Scripts Page (`/scripts`)**: Quick action buttons for predefined scripts, real-time output display, and cancellation controls
- **Settings Page (`/settings`)**: Configure scripts directory path and other application settings

**4. WebSocket Integration**
- Custom hook (`useWebSocket`) to manage WebSocket connection lifecycle
- Real-time output streaming for running scripts
- Task cancellation UI controls
- Connection status indicators

### D. Security & Deployment Strategy

**1. The Build Pipeline**

  * Build the frontend (Vite) which generates static HTML/JS/CSS files in `/frontend/dist`.
  * Build the Rust backend in release mode. The binary will be configured to serve files from the `/frontend/dist` directory.

**2. Cloudflare Zero Trust Configuration**

  * **Tunnel:** Install `cloudflared` on the VPS.
  * **Config:** Point the tunnel to `http://localhost:3000`.
  * **Policy:** In Cloudflare Dashboard -\> Access -\> Applications:
      * Create a new Application (e.g., `steering.mydomain.com`).
      * Set Policy: "Allow" -\> emails ending in `@my-email.com`.

-----

## 4\. Next Steps for AI Implementation

To start coding this, provide the AI with the following instructions:

1.  **Initialize:** "Create a new Rust project with the directory structure defined in the design doc."
2.  **Frontend:** "Set up the Vite frontend inside the Rust project folder and install TailwindCSS."
3.  **Backend:** "Implement the `main.rs` using Axum to serve the API and the static files."
4.  **Connect:** "Ensure the React frontend is fetching from `/api/resources` correctly."