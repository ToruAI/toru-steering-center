#!/bin/bash

# Exit on any error
set -e

echo "================================"
echo "Building Steering Center"
echo "================================"
echo ""

# Build frontend
echo "📦 Building frontend..."
cd frontend
npm install
npm run build
cd ..

echo ""
echo "✅ Frontend build complete"
echo ""

# Build backend
echo "🦀 Building Rust backend..."
cargo build --release

echo ""
echo "✅ Backend build complete"
echo ""

# Run the application
echo "================================"
echo "🚀 Starting Steering Center"
echo "================================"
echo ""
echo "Server will start on http://localhost:3000"
echo "Press Ctrl+C to stop the server"
echo ""

./target/release/steering-center


