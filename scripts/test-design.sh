#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

echo "=== inventory-design: ensure-playwright.sh ==="
bash "$PLUGIN_ROOT/skills/design/inventory-design/scripts/test-ensure-playwright.sh"

echo ""

test_summary
