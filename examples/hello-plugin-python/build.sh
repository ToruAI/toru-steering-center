#!/bin/bash
set -e

echo "Building Hello Plugin (Python)..."

# Make sure Python script is executable
chmod +x hello_plugin.py

# Create a standalone executable by adding shebang and copying
PLUGIN_DIR="../../plugins"
mkdir -p "$PLUGIN_DIR"

# Copy the Python script as a binary
cp hello_plugin.py "$PLUGIN_DIR/hello-plugin-python.binary"
chmod +x "$PLUGIN_DIR/hello-plugin-python.binary"

# Copy frontend bundle if it exists
if [ -d "frontend" ]; then
    mkdir -p "$PLUGIN_DIR/hello-plugin-python"
    cp frontend/bundle.js "$PLUGIN_DIR/hello-plugin-python/bundle.js"
    echo "✓ Frontend bundle copied"
fi

echo "✓ Plugin built successfully: $PLUGIN_DIR/hello-plugin-python.binary"
echo ""
echo "To test the plugin:"
echo "1. Start the steering center server"
echo "2. Enable the plugin via the UI at http://localhost:3000/plugins"
echo ""
echo "To view plugin metadata:"
echo "./$PLUGIN_DIR/hello-plugin-python.binary --metadata"
