#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

INIT="$PLUGIN_ROOT/skills/config/init/SKILL.md"
CONFIGURE="$PLUGIN_ROOT/skills/config/configure/SKILL.md"
DOCS_INTERNALS="$PLUGIN_ROOT/docs-site/src/content/docs/internals.md"
DOCS_CONFIGURATION="$PLUGIN_ROOT/docs-site/src/content/docs/configuration.md"

echo "=== Foundation: init SKILL.md ==="

assert_contains "init lists research_design_inventories path key" \
  "$(cat "$INIT")" "research_design_inventories"
assert_contains "init lists research_design_gaps path key" \
  "$(cat "$INIT")" "research_design_gaps"
# Derive the expected count from the Path Resolution list rather than
# hardcoding it (the marker's correctness is invariant-tested in
# test-config.sh; here we just need a value that auto-tracks the file).
EXPECTED_DIR_COUNT=$(grep -cE '^\*\*[A-Za-z][^*]* directory\*\*:' "$INIT")
assert_contains "init declares directory count via marker" \
  "$(cat "$INIT")" "<!-- DIR_COUNT:${EXPECTED_DIR_COUNT} -->"
assert_contains "init summary lists design inventories directory" \
  "$(cat "$INIT")" "{design inventories directory}"
assert_contains "init summary lists design gaps directory" \
  "$(cat "$INIT")" "{design gaps directory}"

echo ""

echo "=== Foundation: configure SKILL.md ==="

assert_contains "configure paths table includes research_design_inventories" \
  "$(cat "$CONFIGURE")" "research_design_inventories"
assert_contains "configure paths table includes research_design_gaps" \
  "$(cat "$CONFIGURE")" "research_design_gaps"

echo ""

echo "=== Design key call sites use canonical research_design_* form ==="
assert_exit_code "no SKILL.md or agent uses bare design_(inventories|gaps)" 1 \
  bash -c "grep -rE --exclude-dir=node_modules --exclude-dir=target 'config path design_(inventories|gaps)\\b' \"$PLUGIN_ROOT/skills\" \"$PLUGIN_ROOT/agents\""

echo ""

echo "=== Foundation: docs ==="

assert_contains "internals meta/ table lists design-inventories/" \
  "$(cat "$DOCS_INTERNALS")" "design-inventories/"
assert_contains "internals meta/ table lists design-gaps/" \
  "$(cat "$DOCS_INTERNALS")" "design-gaps/"
assert_contains "configuration template keys include design-inventory" \
  "$(cat "$DOCS_CONFIGURATION")" "design-inventory"
assert_contains "configuration template keys include design-gap" \
  "$(cat "$DOCS_CONFIGURATION")" "design-gap"

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
  ' "$file" |
    sed 's/^tools:[[:space:]]*//' |
    sed 's/^>[[:space:]]*//' |
    tr ',' '\n' |
    sed 's/^[[:space:]]*//' |
    sed 's/[[:space:]]*$//' |
    grep -v '^$' |
    sort |
    tr '\n' ',' |
    sed 's/,$//'
}

LOC_TOOLS="$(extract_tools "$LOC")"
ANA_TOOLS="$(extract_tools "$ANA")"

assert_eq "browser-locator declares exactly Bash as its tool" \
  "Bash" \
  "$LOC_TOOLS"
assert_eq "browser-analyser declares exactly Bash as its tool" \
  "Bash" \
  "$ANA_TOOLS"
assert_not_contains "browser-locator declares no mcp__playwright__ tools" \
  "$LOC_TOOLS" "mcp__playwright__"
assert_not_contains "browser-analyser declares no mcp__playwright__ tools" \
  "$ANA_TOOLS" "mcp__playwright__"

echo ""

echo "=== run.sh evaluate payload allowlist ==="

ANA_BODY="$(cat "$ANA")"
for forbidden in "fetch" "XMLHttpRequest" "document.cookie" \
  "localStorage" "sessionStorage" "indexedDB" \
  "eval" "innerHTML" "window.open"; do
  assert_contains "browser-analyser body forbids $forbidden in run.sh evaluate" \
    "$ANA_BODY" "$forbidden"
done

echo ""

echo "=== inventory-design: executor deny-list absent ==="

EXECUTOR_SRC_DIR="$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright"
assert_exit_code "evaluate-payload-rejected not in executor source" 1 \
  grep -r "evaluate-payload-rejected" "$EXECUTOR_SRC_DIR/lib" "$EXECUTOR_SRC_DIR/run.js"
assert_exit_code "no mcp__playwright__ references in executor source" 1 \
  grep -r "mcp__playwright__" "$EXECUTOR_SRC_DIR/lib" "$EXECUTOR_SRC_DIR/run.js"

echo ""

echo "=== .mcp.json ==="

assert_exit_code ".claude-plugin/.mcp.json does not exist (MCP path removed)" 1 \
  test -e "$PLUGIN_ROOT/.claude-plugin/.mcp.json"

echo ""

echo "=== inventory-design: skill structure ==="

SKILL="$PLUGIN_ROOT/skills/design/inventory-design/SKILL.md"
assert_file_exists "inventory-design SKILL.md exists" "$SKILL"
assert_contains "name field set" "$(cat "$SKILL")" "name: inventory-design"
assert_contains "argument-hint declares positional source-id and location" \
  "$(cat "$SKILL")" 'argument-hint: "[source-id] [location]'
assert_contains "disable-model-invocation true" \
  "$(cat "$SKILL")" "disable-model-invocation: true"
assert_contains "argument-hint includes --allow-internal flag" \
  "$(cat "$SKILL")" "--allow-internal"
assert_contains "argument-hint includes --allow-insecure-scheme flag" \
  "$(cat "$SKILL")" "--allow-insecure-scheme"
assert_not_contains "allowed-tools contains no mcp__playwright__ entries" \
  "$(cat "$SKILL")" "mcp__playwright__"
# shellcheck disable=SC2016 # single-quoted assert pattern; ${CLAUDE_PLUGIN_ROOT} is a literal allowed-tools entry matched verbatim, intentionally not shell-expanded
assert_contains "allowed-tools enumerates inventory-design scripts glob" \
  "$(cat "$SKILL")" 'Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/*)'
assert_contains "loads config context" \
  "$(cat "$SKILL")" "accelerator config context"
assert_contains "loads agent names" \
  "$(cat "$SKILL")" "accelerator config agents"
assert_contains "ends with skill-instructions hook" \
  "$(tail -n 5 "$SKILL")" "accelerator config instructions inventory-design"
assert_contains "Agent Names defaults include browser-locator" \
  "$(cat "$SKILL")" "accelerator:browser-locator"
assert_contains "Agent Names defaults include browser-analyser" \
  "$(cat "$SKILL")" "accelerator:browser-analyser"

echo ""

echo "=== inventory-design: evals ==="

EVALS="$PLUGIN_ROOT/skills/design/inventory-design/evals/evals.json"
BENCH="$PLUGIN_ROOT/skills/design/inventory-design/evals/benchmark.json"
assert_file_exists "evals.json exists" "$EVALS"
assert_file_exists "benchmark.json exists" "$BENCH"
assert_eq "evals.json is valid JSON" "$(jq empty "$EVALS" 2>&1)" ""
assert_eq "benchmark.json is valid JSON" "$(jq empty "$BENCH" 2>&1)" ""

echo ""

echo "=== analyse-design-gaps: skill structure ==="

SKILL="$PLUGIN_ROOT/skills/design/analyse-design-gaps/SKILL.md"
assert_file_exists "analyse-design-gaps SKILL.md exists" "$SKILL"
assert_contains "name field set" "$(cat "$SKILL")" "name: analyse-design-gaps"
assert_contains "argument-hint two positional ids" \
  "$(cat "$SKILL")" 'argument-hint: "[current-source-id] [target-source-id]"'
assert_contains "instructs cue-phrase prose" \
  "$(cat "$SKILL")" "we need"
assert_contains "skill body invokes the cue-phrase audit subcommand" \
  "$(cat "$SKILL")" "accelerator design audit-cue-phrases"

echo ""

assert_contains "ends with skill-instructions hook" \
  "$(tail -n 5 "$SKILL")" "accelerator config instructions analyse-design-gaps"
echo "=== analyse-design-gaps: evals ==="

EVALS="$PLUGIN_ROOT/skills/design/analyse-design-gaps/evals/evals.json"
BENCH="$PLUGIN_ROOT/skills/design/analyse-design-gaps/evals/benchmark.json"
assert_file_exists "evals.json exists" "$EVALS"
assert_file_exists "benchmark.json exists" "$BENCH"
assert_eq "evals.json is valid JSON" "$(jq empty "$EVALS" 2>&1)" ""
assert_eq "benchmark.json is valid JSON" "$(jq empty "$BENCH" 2>&1)" ""

echo ""

echo "=== browser-executor preloaded skill ==="

EXEC_SCRIPT="$PLUGIN_ROOT/scripts/config-read-browser-executor.sh"
assert_file_exists "config-read-browser-executor.sh exists" "$EXEC_SCRIPT"
assert_file_executable "config-read-browser-executor.sh is executable" "$EXEC_SCRIPT"

EXEC_OUT="$("$EXEC_SCRIPT" 2>&1)"
EXPECTED_PATH="$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright/run.sh"
assert_contains "browser-executor output begins with ## Browser Executor header" \
  "$EXEC_OUT" "## Browser Executor"
assert_contains "browser-executor output names browser-executor-script key" \
  "$EXEC_OUT" "- browser-executor-script:"
assert_contains "browser-executor output contains absolute run.sh path" \
  "$EXEC_OUT" "$EXPECTED_PATH"

# The resolver must fail loudly rather than emit a stale path if the
# target moves. Simulate by pointing the resolver at a non-existent path
# (override via env var).
NONEXISTENT_OUT="$(ACCELERATOR_BROWSER_EXECUTOR_OVERRIDE=/tmp/does-not-exist/run.sh "$EXEC_SCRIPT" 2>&1 || true)"
assert_contains "resolver refuses missing run.sh" \
  "$NONEXISTENT_OUT" "run.sh not found"

EXEC_SKILL="$PLUGIN_ROOT/skills/config/browser-executor/SKILL.md"
assert_file_exists "browser-executor SKILL.md exists" "$EXEC_SKILL"
assert_contains "browser-executor SKILL.md sets user-invocable: false" \
  "$(cat "$EXEC_SKILL")" "user-invocable: false"
EXEC_FRONTMATTER="$(awk '/^---$/{f++; next} f==1' "$EXEC_SKILL")"
assert_not_contains "browser-executor SKILL.md frontmatter does not set disable-model-invocation: true" \
  "$EXEC_FRONTMATTER" "disable-model-invocation: true"
assert_contains "browser-executor SKILL.md invokes config-read-browser-executor.sh" \
  "$(cat "$EXEC_SKILL")" "config-read-browser-executor.sh"

for agent in agents/browser-locator.md agents/browser-analyser.md; do
  body="$(cat "$PLUGIN_ROOT/$agent")"
  assert_contains "$agent declares accelerator:browser-executor skill" \
    "$body" "accelerator:browser-executor"
  assert_contains "$agent has preload guard checking for Browser Executor block" \
    "$body" "Preload guard"
  assert_contains "$agent guard names the expected key" \
    "$body" "browser-executor-script:"
done

echo ""

echo "=== inventory-design: ensure-playwright.sh ==="
bash "$PLUGIN_ROOT/skills/design/inventory-design/scripts/test-ensure-playwright.sh"

echo ""

echo "=== PROTOCOL.md is in sync with daemon dispatch ==="
PROTOCOL_MD="$PLUGIN_ROOT/skills/design/inventory-design/PROTOCOL.md"
DAEMON_SRC_FOR_SYNC="$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright/lib/daemon.js"
assert_file_exists "PROTOCOL.md exists" "$PROTOCOL_MD"

for cmd in ping daemon-status daemon-stop navigate snapshot links screenshot evaluate click type wait_for; do
  assert_contains "PROTOCOL.md documents the $cmd command" \
    "$(cat "$PROTOCOL_MD")" "### \`$cmd\`"
done

assert_contains "PROTOCOL.md has Environment Variables section" \
  "$(cat "$PROTOCOL_MD")" "## Environment Variables"
DAEMON_ENV_VARS=$(grep -oE 'ACCELERATOR_PLAYWRIGHT_[A-Z_]+' "$DAEMON_SRC_FOR_SYNC" | sort -u)
for var in $DAEMON_ENV_VARS; do
  assert_contains "PROTOCOL.md Environment Variables section names $var" \
    "$(cat "$PROTOCOL_MD")" "$var"
done

echo ""

echo "=== daemon: links is in BLOCKING_OPS ==="
DAEMON_SRC="$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright/lib/daemon.js"
assert_contains "BLOCKING_OPS includes 'links'" \
  "$(grep -E '^const BLOCKING_OPS' "$DAEMON_SRC")" "'links'"

echo ""

echo "=== browser-locator links contract ==="
assert_contains "browser-locator body documents the links command" \
  "$(cat "$LOC")" "{browser-executor-script} links"
assert_contains "browser-locator body uses pathname as route identifier" \
  "$(cat "$LOC")" "Use \`pathname\`"
assert_contains "browser-locator body restricts route names to links output" \
  "$(cat "$LOC")" "Routes come from"
assert_contains "browser-locator body requires same_origin filter" \
  "$(cat "$LOC")" "same_origin: true"
assert_not_contains "browser-analyser body does NOT advertise the links command" \
  "$(cat "$ANA")" "{browser-executor-script} links"

echo ""

echo "=== daemon: owner-PID watcher removed ==="
PLAYWRIGHT_DIR="$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright"
# Repo-wide sweep: no source file under the playwright/ tree references
# any part of the watcher mechanism. Catches future regressions
# regardless of which file (or new test) reintroduces the symbol.
assert_exit_code "no watcher identifier references under playwright/ tree" 1 \
  grep -rnE '\bownerPid\b|--owner-pid|\bOWNER_POLL_MS\b' "$PLAYWRIGHT_DIR"

echo ""

echo "=== inventory-design: playwright executor ==="
bash "$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright/test-run.sh"

echo ""

test_summary
