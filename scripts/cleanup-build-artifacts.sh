#!/bin/bash

# cleanup-build-artifacts.sh
# 
# Cleans up build artifacts and temporary files from the My Little Soda repository
# Helps maintain a clean repository and prevent accidental commits of build output

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "🧹 Cleaning up build artifacts in $(pwd)"

# Clean Cargo/Rust build artifacts
echo "  🗑️  Removing target/ directory..."
rm -rf target/

# Clean root-level build artifacts
echo "  🗑️  Removing root-level .o and .rcgu.o files..."
find . -maxdepth 1 -name "*.o" -o -name "*.rcgu.o" | xargs -r rm -f

# Clean temporary files
echo "  🗑️  Removing temporary and backup files..."
find . -name "*.tmp" -o -name "*~" -o -name "*.bak" -o -name "*.swp" | xargs -r rm -f

# Clean log files (but preserve .flox tracked files)
echo "  🗑️  Cleaning log files..."
rm -f .flox/log/*.log

# Clean core dumps and debug artifacts
echo "  🗑️  Removing core dumps..."
find . -name "core" -o -name "core.*" | xargs -r rm -f

# Show final size
echo "✅ Cleanup complete!"
echo "📊 Current repository size: $(du -sh . | cut -f1)"

# Show any remaining large files/directories for manual review
echo ""
echo "🔍 Largest remaining files/directories:"
du -sh * 2>/dev/null | sort -rh | head -10 || true