# Toru Plugin Protocol Specification

Version: 1.0.0
Last Updated: 2025-12-30

## Overview

The Toru Plugin Protocol defines communication between the Toru core process and plugin processes using JSON messages over Unix domain sockets.

## Table of Contents

- [Wire Format](#wire-format)
- [Message Structure](#message-structure)
- [Message Types](#message-types)
- [Request-Response Flow](#request-response-flow)
- [Error Handling](#error-handling)
- [Version Compatibility](#version-compatibility)
- [Examples](#examples)

## Wire Format

### Binary Framing

All messages use a **length-prefixed framing protocol**:

```
┌──────────────────┬────────────────────────┐
│  Length (4 bytes)│  JSON Message (N bytes)│
│   (big-endian)   │                        │
└──────────────────┴────────────────────────┘
```

- **Length**: 4 bytes, big-endian unsigned 32-bit integer (`u32`)
- **Message**: UTF-8 encoded JSON

### Reading Messages

**Rust:**
```rust
use tokio::io::{AsyncReadExt, BufReader};

async fn read_message(stream: &mut UnixStream) -> Result<Message> {
    let mut reader = BufReader::new(stream);

    // Read 4-byte length prefix
    let mut length_buf = [0u8; 4];
    reader.read_exact(&mut length_buf).await?;
    let length = u32::from_be_bytes(length_buf) as usize;

    // Read JSON message
    let mut msg_buf = vec![0u8; length];
    reader.read_exact(&mut msg_buf).await?;

    // Deserialize
    let message: Message = serde_json::from_slice(&msg_buf)?;
    Ok(message)
}
```

**Python:**
```python
import struct
import json

def read_message(conn):
    # Read 4-byte length prefix
    length_bytes = conn.recv(4)
    if not length_bytes:
        return None

    length = struct.unpack(">I", length_bytes)[0]

    # Read JSON message
    message_bytes = b""
    while len(message_bytes) < length:
        chunk = conn.recv(length - len(message_bytes))
        if not chunk:
            return None
        message_bytes += chunk

    # Deserialize
    return json.loads(message_bytes.decode("utf-8"))
```

### Writing Messages

**Rust:**
```rust
use tokio::io::AsyncWriteExt;

async fn write_message(stream: &mut UnixStream, message: &Message) -> Result<()> {
    let json = serde_json::to_vec(message)?;
    let length = json.len() as u32;

    // Write length prefix
    stream.write_all(&length.to_be_bytes()).await?;

    // Write JSON message
    stream.write_all(&json).await?;
    stream.flush().await?;

    Ok(())
}
```

**Python:**
```python
def write_message(conn, message):
    message_bytes = json.dumps(message).encode("utf-8")
    length = len(message_bytes)

    # Write length prefix
    conn.sendall(struct.pack(">I", length))

    # Write JSON message
    conn.sendall(message_bytes)
```

## Message Structure

### Base Message Format

All messages follow this structure:

```typescript
interface Message {
  type: "lifecycle" | "http" | "kv";
  timestamp: string;  // ISO 8601 UTC timestamp
  request_id?: string;  // Optional, for request-response correlation
  payload: LifecyclePayload | HttpPayload | KvPayload;
}
```

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | Yes | Message type: `lifecycle`, `http`, or `kv` |
| `timestamp` | string | Yes | ISO 8601 UTC timestamp (e.g., `2025-12-30T12:00:00.000Z`) |
| `request_id` | string | No | UUID for correlating requests and responses |
| `payload` | object | Yes | Message-specific payload (see below) |

## Message Types

### 1. Lifecycle Messages

Used for plugin initialization and shutdown.

#### Init Message (Core → Plugin)

Sent when the plugin process is spawned.

```json
{
  "type": "lifecycle",
  "timestamp": "2025-12-30T12:00:00.000Z",
  "payload": {
    "action": "init",
    "instance_id": "toru-instance-abc123",
    "plugin_socket": "/tmp/toru-plugins/my-plugin.sock",
    "log_path": "/var/log/toru/plugins/my-plugin.log"
  }
}
```

**Payload Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `action` | string | Always `"init"` |
| `instance_id` | string | Unique instance identifier (for licensing) |
| `plugin_socket` | string | Unix socket path for this plugin |
| `log_path` | string | Path where plugin should write structured logs |

**Plugin Response:** None (init is fire-and-forget)

#### Shutdown Message (Core → Plugin)

Sent before the core stops the plugin process.

```json
{
  "type": "lifecycle",
  "timestamp": "2025-12-30T12:05:00.000Z",
  "payload": {
    "action": "shutdown"
  }
}
```

**Payload Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `action` | string | Always `"shutdown"` |

**Plugin Response:** None (plugin should exit gracefully)

### 2. HTTP Messages

Used for routing web requests to plugins.

#### HTTP Request (Core → Plugin)

```json
{
  "type": "http",
  "timestamp": "2025-12-30T12:00:01.000Z",
  "request_id": "req-550e8400-e29b-41d4-a716-446655440000",
  "payload": {
    "payload": {
      "method": "POST",
      "path": "/api/action",
      "headers": {
        "Content-Type": "application/json",
        "User-Agent": "Mozilla/5.0..."
      },
      "body": "{\"key\":\"value\"}"
    }
  }
}
```

**Payload Structure:**

```typescript
interface HttpRequestPayload {
  payload: {
    method: string;  // HTTP method (GET, POST, PUT, DELETE, etc.)
    path: string;    // Request path (relative to plugin route)
    headers: Record<string, string>;  // HTTP headers
    body?: string;   // Request body (optional)
  };
}
```

#### HTTP Response (Plugin → Core)

```json
{
  "type": "http",
  "timestamp": "2025-12-30T12:00:01.050Z",
  "request_id": "req-550e8400-e29b-41d4-a716-446655440000",
  "payload": {
    "status": 200,
    "headers": {
      "Content-Type": "application/json"
    },
    "body": "{\"result\":\"success\"}"
  }
}
```

**Payload Structure:**

```typescript
interface HttpResponsePayload {
  status: number;  // HTTP status code (200, 404, 500, etc.)
  headers: Record<string, string>;  // HTTP response headers
  body?: string;   // Response body (optional)
}
```

**Important:** The `request_id` in the response MUST match the request.

### 3. KV Messages

Used for key-value storage operations.

#### Get Operation (Core → Plugin)

```json
{
  "type": "kv",
  "timestamp": "2025-12-30T12:00:02.000Z",
  "request_id": "kv-650e8400-e29b-41d4-a716-446655440001",
  "payload": {
    "action": "get",
    "key": "setting_name"
  }
}
```

#### Get Response (Plugin → Core)

```json
{
  "type": "kv",
  "timestamp": "2025-12-30T12:00:02.010Z",
  "request_id": "kv-650e8400-e29b-41d4-a716-446655440001",
  "payload": {
    "value": "setting_value"
  }
}
```

If the key doesn't exist:

```json
{
  "type": "kv",
  "timestamp": "2025-12-30T12:00:02.010Z",
  "request_id": "kv-650e8400-e29b-41d4-a716-446655440001",
  "payload": {
    "value": null
  }
}
```

#### Set Operation (Core → Plugin)

```json
{
  "type": "kv",
  "timestamp": "2025-12-30T12:00:03.000Z",
  "request_id": "kv-750e8400-e29b-41d4-a716-446655440002",
  "payload": {
    "action": "set",
    "key": "setting_name",
    "value": "new_value"
  }
}
```

#### Set Response (Plugin → Core)

```json
{
  "type": "kv",
  "timestamp": "2025-12-30T12:00:03.005Z",
  "request_id": "kv-750e8400-e29b-41d4-a716-446655440002",
  "payload": {
    "value": null
  }
}
```

#### Delete Operation (Core → Plugin)

```json
{
  "type": "kv",
  "timestamp": "2025-12-30T12:00:04.000Z",
  "request_id": "kv-850e8400-e29b-41d4-a716-446655440003",
  "payload": {
    "action": "delete",
    "key": "old_setting"
  }
}
```

#### Delete Response (Plugin → Core)

```json
{
  "type": "kv",
  "timestamp": "2025-12-30T12:00:04.003Z",
  "request_id": "kv-850e8400-e29b-41d4-a716-446655440003",
  "payload": {
    "value": null
  }
}
```

**KV Payload Structures:**

```typescript
// Request
type KvRequestPayload =
  | { action: "get"; key: string }
  | { action: "set"; key: string; value: string }
  | { action: "delete"; key: string };

// Response
interface KvResponsePayload {
  value: string | null;
}
```

## Request-Response Flow

### Synchronous Request-Response

For HTTP and KV messages:

1. **Core sends request** with unique `request_id`
2. **Plugin processes request**
3. **Plugin sends response** with matching `request_id`
4. **Core correlates response** using `request_id`

```
Core                           Plugin
  │                              │
  ├─── HTTP Request (req-123) ──>│
  │                              │
  │                           [Process]
  │                              │
  │<── HTTP Response (req-123) ──┤
  │                              │
```

### Asynchronous Fire-and-Forget

For lifecycle messages:

1. **Core sends message** (no `request_id`)
2. **Plugin handles message**
3. **No response expected**

```
Core                           Plugin
  │                              │
  ├─── Init Message ──────────>│
  │                              │
  │                           [Initialize]
  │                              │
```

### Timeout Handling

The core will wait for responses with these timeouts:

- **HTTP requests**: 30 seconds
- **KV operations**: 5 seconds

If a plugin doesn't respond within the timeout:
- Core returns HTTP 504 Gateway Timeout to client
- Core logs timeout event
- Plugin process continues running (not killed)

## Error Handling

### Protocol Errors

If a message cannot be parsed or has invalid format:

**Plugin should:**
1. Log error to stderr
2. Close the socket connection
3. Core will detect connection loss and may restart plugin

### Application Errors

If a plugin encounters an error while handling a request:

**HTTP Error Response:**
```json
{
  "type": "http",
  "timestamp": "2025-12-30T12:00:01.100Z",
  "request_id": "req-550e8400-e29b-41d4-a716-446655440000",
  "payload": {
    "status": 500,
    "headers": {
      "Content-Type": "application/json"
    },
    "body": "{\"error\":\"Internal plugin error\",\"message\":\"Database connection failed\"}"
  }
}
```

**KV Error Response:**
```json
{
  "type": "kv",
  "timestamp": "2025-12-30T12:00:02.020Z",
  "request_id": "kv-650e8400-e29b-41d4-a716-446655440001",
  "payload": {
    "value": null
  }
}
```

### Plugin Crash

If plugin process crashes:
1. Core detects process exit
2. Core logs crash event
3. Core attempts restart with exponential backoff
4. After 10 failed restarts, plugin is disabled

### Socket Errors

If socket connection is lost:
1. Plugin should exit gracefully
2. Core will detect and attempt restart
3. Pending requests return HTTP 502 Bad Gateway

## Version Compatibility

### Protocol Versioning

Current protocol version: **1.0.0**

Future versions will be backward compatible where possible. Breaking changes will increment the major version.

### Version Negotiation

Currently, no version negotiation is performed. All plugins use protocol version 1.0.0.

Future: Init message may include `protocol_version` field.

### Deprecation Policy

When fields are deprecated:
1. Field marked as deprecated in documentation
2. Field continues to work for 2 major versions
3. Field removed in 3rd major version

## Examples

### Complete Plugin Session (Rust)

```rust
use tokio::net::{UnixListener, UnixStream};
use toru_plugin_api::*;

#[tokio::main]
async fn main() {
    let socket_path = "/tmp/toru-plugins/my-plugin.sock";
    let listener = UnixListener::bind(socket_path).unwrap();

    let mut protocol = PluginProtocol::new();

    loop {
        let (mut stream, _) = listener.accept().await.unwrap();

        loop {
            // Read message from core
            let message = match protocol.read_message(&mut stream).await {
                Ok(msg) => msg,
                Err(_) => break,  // Connection closed
            };

            // Handle message
            match message.payload {
                MessagePayload::Lifecycle { action, payload } => {
                    if action == "init" {
                        eprintln!("[Plugin] Initialized");
                    } else if action == "shutdown" {
                        std::process::exit(0);
                    }
                }
                MessagePayload::Http { request_id, payload } => {
                    // Handle HTTP request
                    let response = HttpResponse {
                        status: 200,
                        headers: HashMap::new(),
                        body: Some(r#"{"status":"ok"}"#.to_string()),
                    };

                    // Send response
                    let response_msg = Message::new_http_response(
                        request_id,
                        response
                    );
                    protocol.write_message(&mut stream, &response_msg).await.unwrap();
                }
                MessagePayload::Kv { request_id, payload } => {
                    // Handle KV operation
                    let value = match payload {
                        KvMessagePayload::Request(KvOp::Get { key }) => {
                            Some("value".to_string())
                        }
                        _ => None,
                    };

                    // Send response
                    let response_msg = Message::new_kv_response(request_id, value);
                    protocol.write_message(&mut stream, &response_msg).await.unwrap();
                }
            }
        }
    }
}
```

### Complete Plugin Session (Python)

```python
import socket
import struct
import json
from datetime import datetime, timezone

def main():
    socket_path = "/tmp/toru-plugins/my-plugin.sock"

    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(socket_path)
    server.listen(5)

    while True:
        conn, _ = server.accept()

        while True:
            # Read message from core
            message = read_message(conn)
            if not message:
                break

            message_type = message.get("type")
            request_id = message.get("request_id")
            payload = message.get("payload", {})

            # Handle message
            if message_type == "lifecycle":
                action = payload.get("action")
                if action == "init":
                    print("[Plugin] Initialized", file=sys.stderr)
                elif action == "shutdown":
                    sys.exit(0)

            elif message_type == "http":
                # Handle HTTP request
                response_msg = {
                    "type": "http",
                    "timestamp": datetime.now(timezone.utc).isoformat(),
                    "request_id": request_id,
                    "payload": {
                        "status": 200,
                        "headers": {"Content-Type": "application/json"},
                        "body": '{"status":"ok"}'
                    }
                }
                write_message(conn, response_msg)

            elif message_type == "kv":
                # Handle KV operation
                response_msg = {
                    "type": "kv",
                    "timestamp": datetime.now(timezone.utc).isoformat(),
                    "request_id": request_id,
                    "payload": {
                        "value": "some_value"
                    }
                }
                write_message(conn, response_msg)

        conn.close()
```

## Performance Characteristics

### Latency

- Unix socket overhead: **1-5 microseconds**
- JSON serialization: **10-100 microseconds** (depends on message size)
- Total protocol overhead: **~50-200 microseconds**

### Throughput

- Small messages (<1KB): **>100,000 messages/second**
- Large messages (>10KB): **>10,000 messages/second**

### Resource Usage

- Memory per connection: **~8KB**
- CPU overhead: **Negligible (<0.1%)**

## Security Considerations

### Trust Model

Plugins are **trusted code** and have full system access. The protocol provides:
- Process isolation (crash resilience)
- No capability sandboxing

### Socket Permissions

Unix sockets should have restricted permissions:
```bash
# Socket owned by toru process user
chmod 600 /tmp/toru-plugins/*.sock
```

### Input Validation

**Core responsibilities:**
- Validate plugin metadata before loading
- Rate-limit requests to plugins
- Enforce timeouts

**Plugin responsibilities:**
- Validate all inputs from HTTP requests
- Sanitize data before database queries
- Handle malformed messages gracefully

## Appendix

### JSON Schema

Full JSON Schema for protocol messages available at:
https://github.com/toruai/toru-steering-center/blob/main/docs/plugins/schema.json

### Test Suite

Protocol conformance tests available at:
https://github.com/toruai/toru-steering-center/tree/main/tests/protocol

### Reference Implementation

See `toru-plugin-api` crate for reference Rust implementation:
https://github.com/toruai/toru-steering-center/tree/main/toru-plugin-api

---

**Questions or feedback?** Open an issue on GitHub or join our Discord.
