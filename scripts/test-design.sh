#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

INIT="$PLUGIN_ROOT/skills/config/init/SKILL.md"
CONFIGURE="$PLUGIN_ROOT/skills/config/configure/SKILL.md"
README="$PLUGIN_ROOT/README.md"

echo "=== Foundation: init SKILL.md ==="

assert_contains "init lists design_inventories path key" \
  "$(cat "$INIT")" "design_inventories meta/design-inventories"
assert_contains "init lists design_gaps path key" \
  "$(cat "$INIT")" "design_gaps meta/design-gaps"
assert_contains "init declares directory count via marker" \
  "$(cat "$INIT")" "<!-- DIR_COUNT:14 -->"
assert_contains "init summary lists design inventories directory" \
  "$(cat "$INIT")" "{design inventories directory}"
assert_contains "init summary lists design gaps directory" \
  "$(cat "$INIT")" "{design gaps directory}"

echo ""

echo "=== Foundation: configure SKILL.md ==="

assert_contains "configure paths table includes design_inventories" \
  "$(cat "$CONFIGURE")" "design_inventories"
assert_contains "configure paths table includes design_gaps" \
  "$(cat "$CONFIGURE")" "design_gaps"

echo ""

echo "=== Foundation: README ==="

assert_contains "README meta/ table lists design-inventories/" \
  "$(cat "$README")" "design-inventories/"
assert_contains "README meta/ table lists design-gaps/" \
  "$(cat "$README")" "design-gaps/"
assert_contains "README template keys include design-inventory" \
  "$(cat "$README")" "design-inventory"
assert_contains "README template keys include design-gap" \
  "$(cat "$README")" "design-gap"

echo ""

# Subsequent phases append further sections to this same file.
# test_summary runs once at the end of the file after all sections are added.
test_summary
