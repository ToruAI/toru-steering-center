#!/usr/bin/env python3
"""
Hello Plugin (Python Example)
A simple Toru plugin written in Python
"""

import sys
import os
import json
import struct
import socket
import asyncio
from datetime import datetime, timezone
from typing import Optional, Dict, Any


# Plugin metadata
METADATA = {
    "id": "hello-plugin-python",
    "name": "Hello World (Python)",
    "version": "0.1.0",
    "author": "ToruAI",
    "icon": "🐍",
    "route": "/hello-python"
}

# Plugin state
instance_id: Optional[str] = None
kv_store: Dict[str, str] = {}  # Simple in-memory KV store


def get_bundle_js():
    """Read the frontend bundle from file."""
    bundle_path = os.path.join(os.path.dirname(__file__), "frontend", "bundle.js")
    with open(bundle_path, "r") as f:
        return f.read()


def handle_http_request(payload: Dict[str, Any]) -> Dict[str, Any]:
    """Handle HTTP requests."""
    method = payload.get("method", "GET")
    path = payload.get("path", "/")

    print(f"[HelloPlugin] HTTP request: {method} {path}", file=sys.stderr)

    # Serve frontend bundle
    if path == "/bundle.js":
        return {
            "status": 200,
            "headers": {"Content-Type": "application/javascript"},
            "body": get_bundle_js(),
        }
    # Simple JSON response
    elif path == "/" or path == "":
        response = {
            "message": "Hello from Python plugin!",
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
    print(f"[HelloPlugin] KV operation: {action}", file=sys.stderr)

    if action == "get":
        key = payload.get("key")
        if key is None:
            return {"value": None}
        return {"value": kv_store.get(key)}
    elif action == "set":
        key = payload.get("key")
        value = payload.get("value")
        if key is not None:
            kv_store[key] = str(value)
            print(f"[HelloPlugin] Set {key} = {value}", file=sys.stderr)
        return {"value": None}
    elif action == "delete":
        key = payload.get("key")
        if key is not None and key in kv_store:
            del kv_store[key]
            print(f"[HelloPlugin] Deleted {key}", file=sys.stderr)
        return {"value": None}
    else:
        raise ValueError(f"Unknown KV action: {action}")


def handle_init(payload: Dict[str, Any]) -> None:
    """Handle init message."""
    global instance_id
    instance_id = payload.get("instance_id")
    socket_path = payload.get("plugin_socket")
    log_path = payload.get("log_path")

    print(f"[HelloPlugin] Initializing with instance_id: {instance_id}", file=sys.stderr)
    print(f"[HelloPlugin] Socket path: {socket_path}", file=sys.stderr)
    print(f"[HelloPlugin] Log path: {log_path}", file=sys.stderr)


def handle_shutdown() -> None:
    """Handle shutdown message."""
    print("[HelloPlugin] Shutdown received", file=sys.stderr)
    sys.exit(0)


def create_message(message_type: str, request_id: Optional[str] = None, payload: Optional[Dict] = None) -> bytes:
    """Create a message with the specified type and payload."""
    message = {
        "type": message_type,
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }

    if request_id:
        message["request_id"] = request_id

    if payload:
        message["payload"] = payload

    return json.dumps(message).encode("utf-8")


def read_message(conn: socket.socket) -> Optional[Dict]:
    """Read a message from the socket connection."""
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
    except (socket.error, struct.error, json.JSONDecodeError) as e:
        print(f"[HelloPlugin] Error reading message: {e}", file=sys.stderr)
        return None


def write_message(conn: socket.socket, message: Dict) -> bool:
    """Write a message to the socket connection."""
    try:
        message_bytes = json.dumps(message).encode("utf-8")
        length = len(message_bytes)

        # Write length (4 bytes, big-endian)
        conn.sendall(struct.pack(">I", length))

        # Write message body
        conn.sendall(message_bytes)

        return True
    except (socket.error, struct.error) as e:
        print(f"[HelloPlugin] Error writing message: {e}", file=sys.stderr)
        return False


def handle_message(conn: socket.socket, message: Dict) -> None:
    """Handle an incoming message."""
    message_type = message.get("type")
    request_id = message.get("request_id")
    payload = message.get("payload", {})

    print(f"[HelloPlugin] Received message: {message_type}", file=sys.stderr)

    try:
        if message_type == "lifecycle":
            action = payload.get("action")

            if action == "init":
                handle_init(payload)
            elif action == "shutdown":
                handle_shutdown()

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

        else:
            print(f"[HelloPlugin] Unknown message type: {message_type}", file=sys.stderr)

    except Exception as e:
        print(f"[HelloPlugin] Error handling message: {e}", file=sys.stderr)


def main():
    """Main entry point."""
    args = sys.argv[1:]

    # Handle --metadata flag
    if args and args[0] == "--metadata":
        print(json.dumps(METADATA, indent=2))
        return

    print("[HelloPlugin] Starting...", file=sys.stderr)

    # Get socket path from environment or use default
    socket_path = os.environ.get("TORU_PLUGIN_SOCKET", f"/tmp/toru-plugins/{METADATA['id']}.sock")

    print(f"[HelloPlugin] Socket path: {socket_path}", file=sys.stderr)

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

    print(f"[HelloPlugin] Listening on socket...", file=sys.stderr)

    # Accept connections
    try:
        while True:
            try:
                conn, _ = server_sock.accept()
                print(f"[HelloPlugin] Connection accepted", file=sys.stderr)

                # Handle messages
                while True:
                    message = read_message(conn)
                    if message is None:
                        break

                    handle_message(conn, message)

                conn.close()
            except KeyboardInterrupt:
                break
            except Exception as e:
                print(f"[HelloPlugin] Error accepting connection: {e}", file=sys.stderr)

    finally:
        server_sock.close()
        if os.path.exists(socket_path):
            os.unlink(socket_path)


if __name__ == "__main__":
    main()
