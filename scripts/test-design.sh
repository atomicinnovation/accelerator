#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

# What remains of this driver.
#
# Its skill, agent and docs structure assertions moved to
# test-skill-frontmatter-conformance.sh; its assertions over the retained
# JavaScript moved into that suite's own node --test files. What is left is the
# one delegation whose target still exists — and that target, with this file,
# belongs to the vendored-runtime work that removes the last two shell scripts.

echo "=== inventory-design: ensure-playwright.sh ==="
bash "$PLUGIN_ROOT/skills/design/inventory-design/scripts/test-ensure-playwright.sh"

echo ""

test_summary
