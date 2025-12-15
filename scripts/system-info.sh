#!/bin/bash
# Display system information

echo "=== System Information ==="
echo "Hostname: $(hostname)"
echo "Kernel: $(uname -r)"
echo "OS: $(uname -o)"
echo "Architecture: $(uname -m)"
echo ""
echo "=== CPU Information ==="
lscpu | grep -E "Model name|CPU\(s\)|Thread|Core|Socket" || echo "lscpu not available"
echo ""
echo "=== Memory Information ==="
free -h || echo "free command not available"
echo ""
echo "=== Disk Usage ==="
df -h / | tail -1


