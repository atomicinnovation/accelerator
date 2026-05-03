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

echo "=== inventory-design: skill structure ==="

SKILL="$PLUGIN_ROOT/skills/design/inventory-design/SKILL.md"
assert_file_exists "inventory-design SKILL.md exists" "$SKILL"
assert_contains "name field set" "$(cat "$SKILL")" "name: inventory-design"
assert_contains "argument-hint declares positional source-id and location" \
  "$(cat "$SKILL")" 'argument-hint: "[source-id] [location]'
assert_contains "disable-model-invocation true" \
  "$(cat "$SKILL")" "disable-model-invocation: true"
assert_contains "allowed-tools enumerates browser_navigate" \
  "$(cat "$SKILL")" "mcp__playwright__browser_navigate"
assert_contains "allowed-tools enumerates browser_snapshot" \
  "$(cat "$SKILL")" "mcp__playwright__browser_snapshot"
assert_contains "allowed-tools enumerates browser_take_screenshot" \
  "$(cat "$SKILL")" "mcp__playwright__browser_take_screenshot"
assert_contains "allowed-tools enumerates browser_evaluate" \
  "$(cat "$SKILL")" "mcp__playwright__browser_evaluate"
assert_contains "allowed-tools enumerates browser_click" \
  "$(cat "$SKILL")" "mcp__playwright__browser_click"
assert_contains "allowed-tools enumerates browser_type" \
  "$(cat "$SKILL")" "mcp__playwright__browser_type"
assert_contains "allowed-tools enumerates browser_wait_for" \
  "$(cat "$SKILL")" "mcp__playwright__browser_wait_for"
assert_not_contains "allowed-tools must not use mcp__playwright__* wildcard" \
  "$(cat "$SKILL")" "mcp__playwright__*"
assert_contains "loads config context" \
  "$(cat "$SKILL")" "config-read-context.sh"
assert_contains "loads agent names" \
  "$(cat "$SKILL")" "config-read-agents.sh"
assert_contains "ends with skill-instructions hook" \
  "$(tail -n 5 "$SKILL")" "config-read-skill-instructions.sh inventory-design"
assert_contains "Agent Names defaults include browser-locator" \
  "$(cat "$SKILL")" "accelerator:browser-locator"
assert_contains "Agent Names defaults include browser-analyser" \
  "$(cat "$SKILL")" "accelerator:browser-analyser"

echo ""

echo "=== inventory-design: validate-source.sh behavioural ==="

VALIDATE="$PLUGIN_ROOT/skills/design/inventory-design/scripts/validate-source.sh"
assert_file_exists "validate-source.sh exists" "$VALIDATE"
assert_file_executable "validate-source.sh is executable" "$VALIDATE"

assert_exit_code "accepts https URL" 0 "$VALIDATE" "https://prototype.example.com"
assert_exit_code "rejects file:// scheme" 1 "$VALIDATE" "file:///etc/passwd"
assert_exit_code "rejects javascript: scheme" 1 "$VALIDATE" "javascript:alert(1)"
assert_exit_code "rejects data: scheme" 1 "$VALIDATE" "data:text/html,<script>"
assert_exit_code "rejects http://localhost without --allow-internal" 1 "$VALIDATE" "http://localhost:8080"
assert_exit_code "rejects http://127.0.0.1 (loopback) without --allow-internal" 1 "$VALIDATE" "http://127.0.0.1:8080"
assert_exit_code "rejects http://169.254.169.254 (link-local AWS metadata)" 1 "$VALIDATE" "http://169.254.169.254/"
assert_exit_code "rejects RFC1918 10.x.x.x" 1 "$VALIDATE" "http://10.0.0.1/"
assert_exit_code "rejects RFC1918 192.168.x.x" 1 "$VALIDATE" "http://192.168.1.1/"
assert_exit_code "accepts code-repo path inside project root" 0 "$VALIDATE" "./examples/design-test-app"
assert_exit_code "rejects path with .. escape" 1 "$VALIDATE" "../../etc/passwd"

echo ""

echo "=== inventory-design: resolve-auth.sh behavioural ==="

RESOLVE_AUTH="$PLUGIN_ROOT/skills/design/inventory-design/scripts/resolve-auth.sh"
assert_file_exists "resolve-auth.sh exists" "$RESOLVE_AUTH"
assert_file_executable "resolve-auth.sh is executable" "$RESOLVE_AUTH"

ENV_OUT="$(env -i ACCELERATOR_BROWSER_AUTH_HEADER=Bearer-x \
  ACCELERATOR_BROWSER_USERNAME=u ACCELERATOR_BROWSER_PASSWORD=p \
  ACCELERATOR_BROWSER_LOGIN_URL=https://x/login \
  "$RESOLVE_AUTH" 2>/dev/null)"
assert_eq "header takes precedence over form-login vars" "header" "$ENV_OUT"
assert_stderr_contains "warns when form-login vars are ignored" "ignored" \
  env -i ACCELERATOR_BROWSER_AUTH_HEADER=Bearer-x \
  ACCELERATOR_BROWSER_USERNAME=u ACCELERATOR_BROWSER_PASSWORD=p \
  ACCELERATOR_BROWSER_LOGIN_URL=https://x/login \
  "$RESOLVE_AUTH"

ENV_OUT="$(env -i ACCELERATOR_BROWSER_USERNAME=u ACCELERATOR_BROWSER_PASSWORD=p \
  ACCELERATOR_BROWSER_LOGIN_URL=https://x/login \
  "$RESOLVE_AUTH" 2>/dev/null)"
assert_eq "all-three form-login vars resolve to 'form'" "form" "$ENV_OUT"

assert_exit_code "USERNAME+PASSWORD without LOGIN_URL fails fast" 1 \
  env -i ACCELERATOR_BROWSER_USERNAME=u ACCELERATOR_BROWSER_PASSWORD=p \
  "$RESOLVE_AUTH"
assert_stderr_contains "names the missing LOGIN_URL var" "ACCELERATOR_BROWSER_LOGIN_URL" \
  env -i ACCELERATOR_BROWSER_USERNAME=u ACCELERATOR_BROWSER_PASSWORD=p \
  "$RESOLVE_AUTH"

ENV_OUT="$(env -i "$RESOLVE_AUTH" 2>/dev/null)"
assert_eq "no env vars resolve to 'none'" "none" "$ENV_OUT"

echo ""

echo "=== inventory-design: scrub-secrets.sh behavioural ==="

SCRUB="$PLUGIN_ROOT/skills/design/inventory-design/scripts/scrub-secrets.sh"
assert_file_exists "scrub-secrets.sh exists" "$SCRUB"
assert_file_executable "scrub-secrets.sh is executable" "$SCRUB"

CLEAN="$(mktemp)"
echo "An ordinary inventory body with no secrets." > "$CLEAN"
assert_exit_code "clean body passes scrubber" 0 \
  env -i ACCELERATOR_BROWSER_PASSWORD=hunter2_uniq "$SCRUB" "$CLEAN"

LEAKY="$(mktemp)"
echo "The reset link contains hunter2_uniq somewhere." > "$LEAKY"
assert_exit_code "literal env-var value triggers scrubber" 1 \
  env -i ACCELERATOR_BROWSER_PASSWORD=hunter2_uniq "$SCRUB" "$LEAKY"
assert_stderr_contains "scrubber names the env var by name (not value)" \
  "ACCELERATOR_BROWSER_PASSWORD" \
  env -i ACCELERATOR_BROWSER_PASSWORD=hunter2_uniq "$SCRUB" "$LEAKY"

rm -f "$CLEAN" "$LEAKY"

echo ""

echo "=== inventory-design: evals ==="

EVALS="$PLUGIN_ROOT/skills/design/inventory-design/evals/evals.json"
BENCH="$PLUGIN_ROOT/skills/design/inventory-design/evals/benchmark.json"
assert_file_exists "evals.json exists" "$EVALS"
assert_file_exists "benchmark.json exists" "$BENCH"
assert_eq "evals.json is valid JSON" "$(jq empty "$EVALS" 2>&1)" ""
assert_eq "benchmark.json is valid JSON" "$(jq empty "$BENCH" 2>&1)" ""

echo ""

# Subsequent phases append further sections to this same file.
# test_summary runs once at the end of the file after all sections are added.
test_summary
