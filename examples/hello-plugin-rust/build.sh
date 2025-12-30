#!/bin/bash
set -e

echo "Building Hello Plugin (Rust)..."

# Build the plugin binary
cargo build --release

# Copy the binary to the plugins directory with .binary extension
PLUGIN_DIR="../../plugins"
mkdir -p "$PLUGIN_DIR"

# Copy the release binary
cp target/release/hello-plugin-rust "$PLUGIN_DIR/hello-plugin-rust.binary"

# Copy frontend bundle if it exists
if [ -d "frontend" ]; then
    mkdir -p "$PLUGIN_DIR/hello-plugin-rust"
    cp frontend/bundle.js "$PLUGIN_DIR/hello-plugin-rust/bundle.js"
    echo "✓ Frontend bundle copied"
fi

echo "✓ Plugin built successfully: $PLUGIN_DIR/hello-plugin-rust.binary"
echo ""
echo "To test the plugin:"
echo "1. Start the steering center server"
echo "2. Enable the plugin via the UI at http://localhost:3000/plugins"
echo ""
echo "To view plugin metadata:"
echo "./$PLUGIN_DIR/hello-plugin-rust.binary --metadata"
