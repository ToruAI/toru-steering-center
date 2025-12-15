#!/bin/bash
# Show disk usage for all mounted filesystems

echo "=== Disk Usage ==="
df -h
echo ""
echo "=== Largest Directories in / ==="
du -h --max-depth=1 / 2>/dev/null | sort -hr | head -10 || echo "Could not analyze root directory"


