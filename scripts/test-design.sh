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

echo "=== Browser agents ==="

LOC="$PLUGIN_ROOT/agents/browser-locator.md"
ANA="$PLUGIN_ROOT/agents/browser-analyser.md"

assert_file_exists "browser-locator.md exists" "$LOC"
assert_file_exists "browser-analyser.md exists" "$ANA"

# Extract the tools: field from YAML frontmatter, sort items, join with comma.
# Handles both single-line "tools: a, b, c" and wrapped continuation lines.
# Strips leading whitespace (from YAML block-scalar continuation lines).
extract_tools() {
  local file="$1"
  # Extract text between first and second --- (the frontmatter)
  # Find the tools: line, then collect it plus any continuation lines
  awk '
    /^---/ { fm++; next }
    fm == 1 && /^tools:/ { line = $0; in_tools = 1; next }
    fm == 1 && in_tools && /^  / { line = line " " $0; next }
    fm == 1 && in_tools { in_tools = 0 }
    fm == 2 { exit }
    END { print line }
  ' "$file" \
  | sed 's/^tools:[[:space:]]*//' \
  | tr ',' '\n' \
  | sed 's/^[[:space:]]*//' \
  | sed 's/[[:space:]]*$//' \
  | grep -v '^$' \
  | sort \
  | tr '\n' ',' \
  | sed 's/,$//'
}

LOC_TOOLS="$(extract_tools "$LOC")"
ANA_TOOLS="$(extract_tools "$ANA")"

assert_eq "browser-locator declares exactly navigate+snapshot" \
  "mcp__playwright__browser_navigate,mcp__playwright__browser_snapshot" \
  "$LOC_TOOLS"
assert_not_contains "browser-locator does not declare browser_take_screenshot" \
  "$LOC_TOOLS" "browser_take_screenshot"

EXPECTED_ANA_TOOLS="mcp__playwright__browser_click,mcp__playwright__browser_evaluate,mcp__playwright__browser_navigate,mcp__playwright__browser_snapshot,mcp__playwright__browser_take_screenshot,mcp__playwright__browser_type,mcp__playwright__browser_wait_for"
assert_eq "browser-analyser declares exactly the seven Playwright tools" \
  "$EXPECTED_ANA_TOOLS" "$ANA_TOOLS"

echo ""

echo "=== browser_evaluate payload allowlist ==="

ANA_BODY="$(cat "$ANA")"
for forbidden in "fetch" "XMLHttpRequest" "document.cookie" \
                 "localStorage" "sessionStorage" "indexedDB" \
                 "eval" "innerHTML" "window.open"; do
  assert_contains "browser-analyser body forbids $forbidden in browser_evaluate" \
    "$ANA_BODY" "$forbidden"
done

echo ""

echo "=== .mcp.json ==="

MCP="$PLUGIN_ROOT/.claude-plugin/.mcp.json"
assert_file_exists ".mcp.json exists" "$MCP"
assert_eq "mcp.json declares playwright server" \
  "$(jq -r '.mcpServers.playwright.command' "$MCP")" "npx"
PLAYWRIGHT_ARG="$(jq -r '.mcpServers.playwright.args[0]' "$MCP")"
assert_contains "mcp.json playwright args pins @playwright/mcp" \
  "$PLAYWRIGHT_ARG" "@playwright/mcp@"
assert_not_contains ".mcp.json pins @playwright/mcp to a specific version (not @latest)" \
  "$PLAYWRIGHT_ARG" "@latest"
assert_eq "mcp.json is valid JSON" "$(jq empty "$MCP" 2>&1)" ""

echo ""

# Subsequent phases append further sections to this same file.
# test_summary runs once at the end of the file after all sections are added.
test_summary
