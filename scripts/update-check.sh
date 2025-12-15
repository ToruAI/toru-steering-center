#!/bin/bash
# Check for system updates (works on Debian/Ubuntu systems)

echo "=== Checking for Updates ==="
if command -v apt-get &> /dev/null; then
    echo "Running apt-get update..."
    apt-get update -qq
    echo ""
    echo "Checking for upgradable packages..."
    apt list --upgradable 2>/dev/null | head -20
    echo ""
    echo "To upgrade, run: sudo apt-get upgrade"
elif command -v yum &> /dev/null; then
    echo "Running yum check-update..."
    yum check-update 2>&1 | head -20 || echo "No updates available or requires sudo"
elif command -v pacman &> /dev/null; then
    echo "Checking for updates with pacman..."
    pacman -Qu 2>&1 | head -20 || echo "No updates available"
else
    echo "Package manager not recognized. This script works with apt-get, yum, or pacman."
fi


