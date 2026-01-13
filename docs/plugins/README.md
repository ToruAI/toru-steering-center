# Toru Plugin Development Guide

Welcome to the Toru Plugin System. This guide will help you build plugins for Toru Steering Center, whether you're building proprietary plugins in Rust or community plugins in Python.

## Table of Contents

- [Quick Start](#quick-start)
- [Architecture Overview](#architecture-overview)
- [Creating a Rust Plugin](#creating-a-rust-plugin)
- [Creating a Python Plugin](#creating-a-python-plugin)
- [Plugin Protocol Reference](#plugin-protocol-reference)
- [Frontend Development](#frontend-development)
- [Deployment](#deployment)
- [Troubleshooting](#troubleshooting)

## Quick Start

Get a plugin running in 5 minutes.

### Rust Example (Minimal)

```rust
// Cargo.toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[dependencies]
toru-plugin-api = { git = "https://github.com/toruai/toru-steering-center" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
```

```rust
// src/main.rs
use toru_plugin_api::*;
use std::collections::HashMap;

struct MyPlugin;

#[async_trait::async_trait]
impl ToruPlugin for MyPlugin {
    fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "my-plugin".to_string(),
            name: "My Plugin".to_string(),
            version: "0.1.0".to_string(),
            author: Some("Me".to_string()),
            icon: "🚀".to_string(),
            route: "/my-plugin".to_string(),
        }
    }

    async fn init(&mut self, ctx: PluginContext) -> PluginResult<()> {
        println!("Plugin initialized: {}", ctx.instance_id);
        Ok(())
    }

    async fn handle_http(&self, req: HttpRequest) -> PluginResult<HttpResponse> {
        Ok(HttpResponse {
            status: 200,
            headers: HashMap::from([
                ("Content-Type".to_string(), "application/json".to_string())
            ]),
            body: Some(r#"{"status":"ok"}"#.to_string()),
        })
    }

    async fn handle_kv(&mut self, op: KvOp) -> PluginResult<Option<String>> {
        Ok(None)
    }
}

#[tokio::main]
async fn main() {
    toru_plugin_api::run_plugin::<MyPlugin>().await;
}
```

### Python Example (Minimal)

```python
#!/usr/bin/env python3
import json
import sys

METADATA = {
    "id": "my-plugin",
    "name": "My Plugin",
    "version": "0.1.0",
    "author": "Me",
    "icon": "🚀",
    "route": "/my-plugin"
}

def handle_http(payload):
    return {
        "status": 200,
        "headers": {"Content-Type": "application/json"},
        "body": json.dumps({"status": "ok"})
    }

if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--metadata":
        print(json.dumps(METADATA))
        sys.exit(0)

    # See full example for socket implementation
    from toru_plugin import run_plugin
    run_plugin(METADATA, handle_http)
```

## Architecture Overview

Toru uses a **process-isolated plugin architecture** for stability and flexibility.

```
┌─────────────────────────────────────────────────────┐
│         Toru Steering Center (Core Process)         │
│                                                      │
│  ┌────────────────────────────────────────────────┐ │
│  │        Plugin Supervisor                       │ │
│  │  - Spawns plugin processes                     │ │
│  │  - Monitors health                             │ │
│  │  - Auto-restarts on crash                      │ │
│  │  - Routes HTTP requests                        │ │
│  └────────────────────────────────────────────────┘ │
│                       │                              │
└───────────────────────┼──────────────────────────────┘
                        │
          ┌─────────────┼─────────────┐
          │             │             │
    ┌─────▼─────┐ ┌────▼─────┐ ┌────▼─────┐
    │ Plugin A  │ │ Plugin B │ │ Plugin C │
    │ (Rust)    │ │ (Python) │ │ (Rust)   │
    │           │ │          │ │          │
    │ Unix      │ │ Unix     │ │ Unix     │
    │ Socket    │ │ Socket   │ │ Socket   │
    └───────────┘ └──────────┘ └──────────┘
```

### Why Process Isolation?

- **Crash isolation**: Plugin failure doesn't crash the core
- **Language flexibility**: Any language that supports Unix sockets
- **Auto-restart**: Core can restart failed plugins automatically
- **No ABI issues**: Stable JSON protocol instead of binary API
- **Performance**: Unix sockets have microsecond overhead

### Communication Flow

```
1. User visits /plugins/my-plugin
           │
           ▼
2. Core forwards HTTP request to plugin
           │
           ▼
3. Plugin process receives request via Unix socket
           │
           ▼
4. Plugin handles request and returns response
           │
           ▼
5. Core forwards response to user
```

## Creating a Rust Plugin

### Project Setup

```bash
cargo new --bin my-plugin
cd my-plugin
```

Edit `Cargo.toml`:

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[dependencies]
toru-plugin-api = { git = "https://github.com/toruai/toru-steering-center" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.6", features = ["v4", "serde"] }
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
```

### Implementing the ToruPlugin Trait

```rust
use toru_plugin_api::*;
use std::collections::HashMap;

struct MyPlugin {
    ctx: Option<PluginContext>,
}

impl MyPlugin {
    fn new() -> Self {
        Self { ctx: None }
    }
}

#[async_trait::async_trait]
impl ToruPlugin for MyPlugin {
    fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "my-plugin".to_string(),
            name: "My Plugin".to_string(),
            version: "0.1.0".to_string(),
            author: Some("Your Name".to_string()),
            icon: "🚀".to_string(),
            route: "/my-plugin".to_string(),
        }
    }

    async fn init(&mut self, ctx: PluginContext) -> PluginResult<()> {
        eprintln!("[MyPlugin] Initializing with instance_id: {}", ctx.instance_id);
        self.ctx = Some(ctx);
        Ok(())
    }

    async fn handle_http(&self, req: HttpRequest) -> PluginResult<HttpResponse> {
        eprintln!("[MyPlugin] HTTP request: {} {}", req.method, req.path);

        // Route handling
        match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/") => {
                let response = serde_json::json!({
                    "message": "Hello from my plugin!",
                    "version": "0.1.0"
                });

                Ok(HttpResponse {
                    status: 200,
                    headers: HashMap::from([
                        ("Content-Type".to_string(), "application/json".to_string())
                    ]),
                    body: Some(serde_json::to_string(&response)?),
                })
            }
            ("GET", "/bundle.js") => {
                // Serve frontend bundle
                Ok(HttpResponse {
                    status: 200,
                    headers: HashMap::from([
                        ("Content-Type".to_string(), "application/javascript".to_string())
                    ]),
                    body: Some(include_str!("../frontend/bundle.js").to_string()),
                })
            }
            _ => {
                Ok(HttpResponse {
                    status: 404,
                    headers: HashMap::new(),
                    body: Some("Not found".to_string()),
                })
            }
        }
    }

    async fn handle_kv(&mut self, op: KvOp) -> PluginResult<Option<String>> {
        eprintln!("[MyPlugin] KV operation: {:?}", op);

        match op {
            KvOp::Get { key } => {
                // Get from plugin storage
                Ok(None)
            }
            KvOp::Set { key, value } => {
                eprintln!("[MyPlugin] Setting {} = {}", key, value);
                Ok(None)
            }
            KvOp::Delete { key } => {
                eprintln!("[MyPlugin] Deleting {}", key);
                Ok(None)
            }
        }
    }
}
```

### Main Entry Point

The `toru-plugin-api` crate provides a helper to run your plugin:

```rust
#[tokio::main]
async fn main() {
    // Handle --metadata flag
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--metadata" {
        let metadata = MyPlugin::metadata();
        println!("{}", serde_json::to_string_pretty(&metadata).unwrap());
        return;
    }

    // Run plugin with automatic socket setup
    let mut plugin = MyPlugin::new();
    toru_plugin_api::run_plugin(&mut plugin).await;
}
```

### Using KV Storage

Store plugin settings or state:

```rust
async fn handle_http(&self, req: HttpRequest) -> PluginResult<HttpResponse> {
    if let Some(ctx) = &self.ctx {
        // Store a value
        ctx.kv.set("last_visit", "2025-12-30T12:00:00Z").await?;

        // Retrieve a value
        let last_visit = ctx.kv.get("last_visit").await?;

        // Delete a value
        ctx.kv.delete("old_key").await?;
    }

    // ... rest of handler
}
```

### Building and Testing

```bash
# Build release binary
cargo build --release

# Test metadata output
./target/release/my-plugin --metadata

# Run plugin locally
TORU_PLUGIN_SOCKET=/tmp/my-plugin.sock ./target/release/my-plugin
```

## Creating a Python Plugin

### Project Structure

```
my-plugin/
├── my_plugin.py          # Main plugin implementation
├── frontend/
│   └── bundle.js         # Frontend bundle
└── requirements.txt      # Python dependencies (if any)
```

### Implementation

```python
#!/usr/bin/env python3
"""
My Plugin - Python Example
"""

import sys
import os
import json
import struct
import socket
from datetime import datetime, timezone
from typing import Optional, Dict, Any

# Plugin metadata
METADATA = {
    "id": "my-plugin",
    "name": "My Plugin",
    "version": "0.1.0",
    "author": "Your Name",
    "icon": "🚀",
    "route": "/my-plugin"
}

# Plugin state
instance_id: Optional[str] = None
kv_store: Dict[str, str] = {}  # Simple in-memory KV store


def get_bundle_js() -> str:
    """Read the frontend bundle from file."""
    bundle_path = os.path.join(os.path.dirname(__file__), "frontend", "bundle.js")
    with open(bundle_path, "r") as f:
        return f.read()


def handle_http_request(payload: Dict[str, Any]) -> Dict[str, Any]:
    """Handle HTTP requests."""
    method = payload.get("method", "GET")
    path = payload.get("path", "/")

    print(f"[MyPlugin] HTTP request: {method} {path}", file=sys.stderr)

    # Serve frontend bundle
    if path == "/bundle.js":
        return {
            "status": 200,
            "headers": {"Content-Type": "application/javascript"},
            "body": get_bundle_js(),
        }
    # API endpoint
    elif path == "/" or path == "":
        response = {
            "message": "Hello from my Python plugin!",
            "instance_id": instance_id or "unknown",
            "time": datetime.now(timezone.utc).isoformat(),
        }
        return {
            "status": 200,
            "headers": {"Content-Type": "application/json"},
            "body": json.dumps(response),
        }
    else:
        return {
            "status": 404,
            "headers": {"Content-Type": "application/json"},
            "body": json.dumps({"error": "Not found"}),
        }


def handle_kv_operation(payload: Dict[str, Any]) -> Dict[str, Any]:
    """Handle KV operations."""
    action = payload.get("action")

    if action == "get":
        key = payload.get("key")
        return {"value": kv_store.get(key)}
    elif action == "set":
        key = payload.get("key")
        value = payload.get("value")
        if key is not None:
            kv_store[key] = str(value)
        return {"value": None}
    elif action == "delete":
        key = payload.get("key")
        if key is not None and key in kv_store:
            del kv_store[key]
        return {"value": None}
    else:
        raise ValueError(f"Unknown KV action: {action}")


def handle_init(payload: Dict[str, Any]) -> None:
    """Handle init message."""
    global instance_id
    instance_id = payload.get("instance_id")
    print(f"[MyPlugin] Initialized: {instance_id}", file=sys.stderr)


def read_message(conn: socket.socket) -> Optional[Dict]:
    """Read a message from the socket (4-byte length + JSON)."""
    try:
        # Read message length (4 bytes, big-endian)
        length_bytes = conn.recv(4)
        if not length_bytes:
            return None

        length = struct.unpack(">I", length_bytes)[0]

        # Read message body
        message_bytes = b""
        while len(message_bytes) < length:
            chunk = conn.recv(length - len(message_bytes))
            if not chunk:
                return None
            message_bytes += chunk

        # Deserialize JSON
        return json.loads(message_bytes.decode("utf-8"))
    except Exception as e:
        print(f"[MyPlugin] Error reading message: {e}", file=sys.stderr)
        return None


def write_message(conn: socket.socket, message: Dict) -> bool:
    """Write a message to the socket (4-byte length + JSON)."""
    try:
        message_bytes = json.dumps(message).encode("utf-8")
        length = len(message_bytes)

        # Write length (4 bytes, big-endian)
        conn.sendall(struct.pack(">I", length))

        # Write message body
        conn.sendall(message_bytes)

        return True
    except Exception as e:
        print(f"[MyPlugin] Error writing message: {e}", file=sys.stderr)
        return False


def handle_message(conn: socket.socket, message: Dict) -> None:
    """Handle an incoming message."""
    message_type = message.get("type")
    request_id = message.get("request_id")
    payload = message.get("payload", {})

    try:
        if message_type == "lifecycle":
            action = payload.get("action")
            if action == "init":
                handle_init(payload)
            elif action == "shutdown":
                sys.exit(0)

        elif message_type == "http":
            # Handle HTTP request
            http_payload = payload.get("payload", {})
            response = handle_http_request(http_payload)

            # Send response
            response_message = {
                "type": "http",
                "timestamp": datetime.now(timezone.utc).isoformat(),
                "request_id": request_id,
                "payload": response,
            }
            write_message(conn, response_message)

        elif message_type == "kv":
            # Handle KV operation
            kv_payload = payload.get("payload", {})
            response = handle_kv_operation(kv_payload)

            # Send response
            response_message = {
                "type": "kv",
                "timestamp": datetime.now(timezone.utc).isoformat(),
                "request_id": request_id,
                "payload": response,
            }
            write_message(conn, response_message)

    except Exception as e:
        print(f"[MyPlugin] Error handling message: {e}", file=sys.stderr)


def main():
    """Main entry point."""
    args = sys.argv[1:]

    # Handle --metadata flag
    if args and args[0] == "--metadata":
        print(json.dumps(METADATA, indent=2))
        return

    print("[MyPlugin] Starting...", file=sys.stderr)

    # Get socket path from environment or use default
    socket_path = os.environ.get(
        "TORU_PLUGIN_SOCKET",
        f"/tmp/toru-plugins/{METADATA['id']}.sock"
    )

    # Ensure socket directory exists
    socket_dir = os.path.dirname(socket_path)
    if socket_dir:
        os.makedirs(socket_dir, exist_ok=True)

    # Remove socket file if it exists
    if os.path.exists(socket_path):
        os.unlink(socket_path)

    # Create Unix socket
    server_sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server_sock.bind(socket_path)
    server_sock.listen(5)

    print(f"[MyPlugin] Listening on {socket_path}", file=sys.stderr)

    # Accept connections
    try:
        while True:
            conn, _ = server_sock.accept()

            # Handle messages
            while True:
                message = read_message(conn)
                if message is None:
                    break
                handle_message(conn, message)

            conn.close()
    finally:
        server_sock.close()
        if os.path.exists(socket_path):
            os.unlink(socket_path)


if __name__ == "__main__":
    main()
```

### Building a Wrapper Script

Make your plugin executable:

```bash
#!/bin/bash
# wrapper.sh - Makes your Python plugin deployable

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$SCRIPT_DIR/my_plugin.py" "$@"
```

## Plugin Protocol Reference

### Message Format

All messages use a **4-byte length prefix** followed by **JSON payload**.

Wire format:
```
[4 bytes: message length (big-endian u32)] [N bytes: JSON message]
```

### Message Types

#### Lifecycle Messages

**Init** - Sent by core to plugin on startup:

```json
{
  "type": "lifecycle",
  "timestamp": "2025-12-30T12:00:00Z",
  "payload": {
    "action": "init",
    "instance_id": "toru-instance-abc123",
    "plugin_socket": "/tmp/toru-plugins/my-plugin.sock",
    "log_path": "/var/log/toru/plugins/my-plugin.log"
  }
}
```

**Shutdown** - Sent by core before stopping plugin:

```json
{
  "type": "lifecycle",
  "timestamp": "2025-12-30T12:05:00Z",
  "payload": {
    "action": "shutdown"
  }
}
```

#### HTTP Messages

**Request** - Core to plugin:

```json
{
  "type": "http",
  "timestamp": "2025-12-30T12:00:01Z",
  "request_id": "req-uuid-1234",
  "payload": {
    "payload": {
      "method": "POST",
      "path": "/api/action",
      "headers": {
        "Content-Type": "application/json"
      },
      "body": "{\"key\":\"value\"}"
    }
  }
}
```

**Response** - Plugin to core:

```json
{
  "type": "http",
  "timestamp": "2025-12-30T12:00:01.050Z",
  "request_id": "req-uuid-1234",
  "payload": {
    "status": 200,
    "headers": {
      "Content-Type": "application/json"
    },
    "body": "{\"result\":\"success\"}"
  }
}
```

#### KV Messages

**Get Request**:

```json
{
  "type": "kv",
  "timestamp": "2025-12-30T12:00:02Z",
  "request_id": "kv-uuid-5678",
  "payload": {
    "action": "get",
    "key": "setting_name"
  }
}
```

**Get Response**:

```json
{
  "type": "kv",
  "timestamp": "2025-12-30T12:00:02.010Z",
  "request_id": "kv-uuid-5678",
  "payload": {
    "value": "setting_value"
  }
}
```

**Set Request**:

```json
{
  "type": "kv",
  "timestamp": "2025-12-30T12:00:03Z",
  "request_id": "kv-uuid-9012",
  "payload": {
    "action": "set",
    "key": "setting_name",
    "value": "new_value"
  }
}
```

**Delete Request**:

```json
{
  "type": "kv",
  "timestamp": "2025-12-30T12:00:04Z",
  "request_id": "kv-uuid-3456",
  "payload": {
    "action": "delete",
    "key": "old_setting"
  }
}
```

### Error Handling

If your plugin encounters an error, log to stderr and return an HTTP 500 response:

```json
{
  "type": "http",
  "request_id": "req-uuid-1234",
  "payload": {
    "status": 500,
    "headers": {"Content-Type": "application/json"},
    "body": "{\"error\":\"Internal plugin error\"}"
  }
}
```

## Frontend Development

### Bundle.js Structure

Your plugin frontend is a single JavaScript file loaded by the core:

```javascript
// frontend/bundle.js
(function() {
    console.log('[MyPlugin] Loading frontend...');

    const pluginId = 'my-plugin';

    // Register plugin
    window.ToruPlugins = window.ToruPlugins || {};
    window.ToruPlugins[pluginId] = {
        // Called when user navigates to plugin
        mount(container, api) {
            console.log('[MyPlugin] Mounting...');

            // Clear container
            container.innerHTML = '';

            // Build your UI
            const header = document.createElement('h1');
            header.textContent = 'My Plugin';
            header.className = 'text-3xl font-bold mb-4';
            container.appendChild(header);

            // Add interactive elements
            const button = document.createElement('button');
            button.textContent = 'Call API';
            button.className = 'bg-purple-600 text-white px-4 py-2 rounded';
            button.onclick = async () => {
                try {
                    const response = await api.fetch('/api/plugins/my-plugin');
                    const data = await response.json();
                    api.showToast('Success: ' + data.message, 'success');
                } catch (error) {
                    api.showToast('Error: ' + error.message, 'error');
                }
            };
            container.appendChild(button);
        },

        // Called when user navigates away
        unmount(container) {
            console.log('[MyPlugin] Unmounting...');
            container.innerHTML = '';
        }
    };
})();
```

### API Object

The `api` object provides helpers:

```javascript
// Fetch API (relative to core URL)
const response = await api.fetch('/api/plugins/my-plugin/endpoint', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ key: 'value' })
});

// Show toast notification
api.showToast('Operation successful', 'success');  // success, error, info
api.showToast('Something went wrong', 'error');

// Navigate to another view
api.navigate('/dashboard');
```

### Using React/Vue/Other Frameworks

You can bundle React/Vue/etc. into your bundle.js:

```bash
# Build with your framework
npm run build

# Output should be a single bundle.js
# Use IIFE format, not ES modules
```

**Important**: Your bundle.js must be a single self-contained file (IIFE format).

## Deployment

### Building Plugins

**Rust plugins:**

```bash
cargo build --release
cp target/release/my-plugin ./plugins/my-plugin.binary
```

**Python plugins:**

```bash
# Create wrapper script
cat > plugins/my-plugin.binary << 'EOF'
#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$SCRIPT_DIR/my_plugin.py" "$@"
EOF

chmod +x plugins/my-plugin.binary

# Copy plugin files
cp -r my-plugin/* plugins/my-plugin/
```

### Installing Plugins

1. Place your plugin binary in `./plugins/` directory:
   ```
   ./plugins/
   ├── my-plugin.binary
   └── my-plugin/
       ├── frontend/
       │   └── bundle.js
       └── (other files)
   ```

2. Make it executable:
   ```bash
   chmod +x ./plugins/my-plugin.binary
   ```

3. Restart Toru or enable via API:
   ```bash
   curl -X POST http://localhost:3000/api/plugins/my-plugin/enable
   ```

### Plugin Directory Structure

```
./plugins/
├── acme-integration.binary       # Rust compiled binary
├── weather-widget.binary          # Python wrapper script
├── weather-widget/                # Python plugin files
│   ├── weather_plugin.py
│   └── frontend/
│       └── bundle.js
└── .metadata/
    └── config.json                # Enabled/disabled state
```

### Environment Variables

Toru passes these environment variables to your plugin:

- `TORU_PLUGIN_SOCKET`: Unix socket path (e.g., `/tmp/toru-plugins/my-plugin.sock`)
- `TORU_PLUGIN_ID`: Plugin ID from metadata
- `TORU_INSTANCE_ID`: Unique instance identifier

## Troubleshooting

### Plugin Not Starting

1. Check plugin binary is executable:
   ```bash
   ls -l ./plugins/my-plugin.binary
   chmod +x ./plugins/my-plugin.binary
   ```

2. Test metadata output:
   ```bash
   ./plugins/my-plugin.binary --metadata
   ```

3. Check logs:
   ```bash
   tail -f /var/log/toru/plugins/my-plugin.log
   tail -f /var/log/toru/plugin-supervisor.log
   ```

### Socket Connection Errors

1. Ensure `/tmp/toru-plugins/` directory exists:
   ```bash
   mkdir -p /tmp/toru-plugins
   ```

2. Check for stale socket files:
   ```bash
   rm /tmp/toru-plugins/*.sock
   ```

3. Verify socket path matches:
   ```bash
   # In your plugin
   echo $TORU_PLUGIN_SOCKET
   ```

### HTTP Requests Not Working

1. Verify route in plugin metadata matches URL:
   ```json
   {
     "route": "/my-plugin"  // Must match URL path
   }
   ```

2. Check request/response format matches protocol

3. Add debug logging:
   ```rust
   eprintln!("[MyPlugin] Received request: {:?}", req);
   ```

### KV Storage Issues

1. Check request_id matches in request and response
2. Verify payload format follows protocol spec
3. Use stderr logging to debug KV operations

### Frontend Not Loading

1. Verify bundle.js path in HTTP handler:
   ```rust
   if req.path == "/bundle.js" {
       // Return bundle content
   }
   ```

2. Check browser console for JavaScript errors

3. Verify plugin ID matches in:
   - Plugin metadata
   - `window.ToruPlugins[pluginId]`
   - Frontend URL

### Plugin Crashing

1. Check stderr output:
   ```bash
   journalctl -f | grep my-plugin
   ```

2. Rust: Use `RUST_BACKTRACE=1` for stack traces

3. Python: Check for unhandled exceptions

4. Review restart count:
   ```bash
   curl http://localhost:3000/api/plugins/my-plugin
   ```

### Performance Issues

1. Profile your plugin code (not the protocol)
2. Unix socket overhead is ~1-5 microseconds (negligible)
3. Check database queries and external API calls
4. Use async/await properly

## Next Steps

- See [PROTOCOL.md](./PROTOCOL.md) for complete protocol specification
- See [ARCHITECTURE.md](./ARCHITECTURE.md) for system internals
- Check [examples/](../../examples/) for complete working plugins
- Join the community for help and showcase your plugins

## Resources

- GitHub Repository: https://github.com/toruai/toru-steering-center
- Plugin API Documentation: https://docs.toruai.com/plugins
- Community Discord: https://discord.gg/toruai
