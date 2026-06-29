#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

INIT="$PLUGIN_ROOT/skills/config/init/SKILL.md"
CONFIGURE="$PLUGIN_ROOT/skills/config/configure/SKILL.md"
DOCS_INTERNALS="$PLUGIN_ROOT/docs/internals.md"
DOCS_CONFIGURATION="$PLUGIN_ROOT/docs/configuration.md"

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
  bash -c "grep -rE --exclude-dir=node_modules --exclude-dir=target 'config-read-path\\.sh[[:space:]]+design_(inventories|gaps)\\b' \"$PLUGIN_ROOT/skills\" \"$PLUGIN_ROOT/agents\""

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

# Unchanged: https + path acceptance, scheme rejections, .. escape rejection
assert_exit_code "accepts https URL" 0 "$VALIDATE" "https://prototype.example.com"
assert_exit_code "rejects file:// scheme" 1 "$VALIDATE" "file:///etc/passwd"
assert_exit_code "rejects javascript: scheme" 1 "$VALIDATE" "javascript:alert(1)"
assert_exit_code "rejects data: scheme" 1 "$VALIDATE" "data:text/html,<script>"
assert_exit_code "accepts code-repo path inside project root" 0 "$VALIDATE" "./examples/design-test-app"
assert_exit_code "rejects path with .. escape" 1 "$VALIDATE" "../../etc/passwd"

# Default-allow cases (localhost / 127.0.0.1)
assert_exit_code "accepts http://localhost without flag" 0 "$VALIDATE" "http://localhost:8080"
assert_exit_code "accepts http://localhost (no port) without flag" 0 "$VALIDATE" "http://localhost/"
assert_exit_code "accepts http://127.0.0.1 without flag" 0 "$VALIDATE" "http://127.0.0.1:3000"
assert_exit_code "accepts https://localhost without flag" 0 "$VALIDATE" "https://localhost:8443"

# Canonicalisation: equivalent forms of localhost are all default-allowed
assert_exit_code "accepts http://LOCALHOST (uppercase)" 0 "$VALIDATE" "http://LOCALHOST:8080"
assert_exit_code "accepts http://localhost. (trailing dot)" 0 "$VALIDATE" "http://localhost./"
assert_exit_code "accepts http://localhost:8080/path?q=1" 0 "$VALIDATE" "http://localhost:8080/path?q=1"

# Internal-host cases: rejected without --allow-internal, accepted with it
assert_exit_code "rejects http://127.0.0.2 without flag" 1 "$VALIDATE" "http://127.0.0.2/"
assert_exit_code "accepts http://127.0.0.2 with --allow-internal" 0 "$VALIDATE" "http://127.0.0.2/" --allow-internal
assert_exit_code "rejects http://10.0.0.1 without flag" 1 "$VALIDATE" "http://10.0.0.1/"
assert_exit_code "accepts http://10.0.0.1 with --allow-internal" 0 "$VALIDATE" "http://10.0.0.1/" --allow-internal
assert_exit_code "rejects http://192.168.1.1 without flag" 1 "$VALIDATE" "http://192.168.1.1/"
assert_exit_code "accepts http://192.168.1.1 with --allow-internal" 0 "$VALIDATE" "http://192.168.1.1/" --allow-internal

# RFC1918 boundary (172.16/12 — most error-prone arithmetic)
assert_exit_code "rejects http://172.16.0.1 (lower edge) without flag" 1 "$VALIDATE" "http://172.16.0.1/"
assert_exit_code "rejects http://172.31.255.255 (upper edge) without flag" 1 "$VALIDATE" "http://172.31.255.255/"
assert_stderr_contains "172.16.0.1 reject names RFC1918" "RFC1918" \
  "$VALIDATE" "http://172.16.0.1/"
assert_stderr_contains "172.31.255.255 reject names RFC1918" "RFC1918" \
  "$VALIDATE" "http://172.31.255.255/"
# 172.15.x and 172.32.x are *outside* RFC1918 — they are public hosts on http,
# so without --allow-insecure-scheme they are rejected as insecure-scheme,
# NOT as RFC1918. This differentiates the two reject paths.
assert_exit_code "rejects http://172.15.255.255 (just outside RFC1918) without flag" 1 "$VALIDATE" "http://172.15.255.255/"
assert_stderr_contains "172.15.255.255 reject names insecure-scheme" "--allow-insecure-scheme" \
  "$VALIDATE" "http://172.15.255.255/"
assert_exit_code "rejects http://172.32.0.0 (just outside RFC1918) without flag" 1 "$VALIDATE" "http://172.32.0.0/"
assert_stderr_contains "172.32.0.0 reject names insecure-scheme" "--allow-insecure-scheme" \
  "$VALIDATE" "http://172.32.0.0/"

# Link-local / cloud metadata
assert_exit_code "rejects http://169.254.169.254 without flag" 1 "$VALIDATE" "http://169.254.169.254/"
assert_exit_code "accepts http://169.254.169.254 with --allow-internal" 0 "$VALIDATE" "http://169.254.169.254/" --allow-internal

# IPv6
assert_exit_code "rejects [::1] without flag" 1 "$VALIDATE" "http://[::1]/"
assert_exit_code "accepts [::1] with --allow-internal" 0 "$VALIDATE" "http://[::1]/" --allow-internal
assert_exit_code "rejects [fe80::1] without flag" 1 "$VALIDATE" "http://[fe80::1]/"
assert_exit_code "accepts [fe80::1%eth0] (zone-id stripped) with --allow-internal" 0 "$VALIDATE" "http://[fe80::1%eth0]/" --allow-internal
assert_exit_code "rejects [::ffff:127.0.0.1] (IPv4-mapped) without flag" 1 "$VALIDATE" "http://[::ffff:127.0.0.1]/"
assert_exit_code "accepts [::ffff:127.0.0.1] with --allow-internal" 0 "$VALIDATE" "http://[::ffff:127.0.0.1]/" --allow-internal
assert_exit_code "rejects [::] without flag" 1 "$VALIDATE" "http://[::]/"
assert_exit_code "accepts [::] with --allow-internal" 0 "$VALIDATE" "http://[::]/" --allow-internal
assert_exit_code "rejects [::1]:8080 (port present) without flag" 1 "$VALIDATE" "http://[::1]:8080/"

# 0.0.0.0 (commonly resolves to local, RFC1122-reserved)
assert_exit_code "rejects http://0.0.0.0 without flag" 1 "$VALIDATE" "http://0.0.0.0/"
assert_exit_code "accepts http://0.0.0.0 with --allow-internal" 0 "$VALIDATE" "http://0.0.0.0/" --allow-internal

# Numeric / encoded IPv4 forms — rejected outright as malformed (no flag bypass)
assert_exit_code "rejects http://2130706433 (decimal-encoded 127.0.0.1)" 1 "$VALIDATE" "http://2130706433/"
assert_exit_code "rejects http://0x7f000001 (hex-encoded 127.0.0.1)" 1 "$VALIDATE" "http://0x7f000001/"
assert_exit_code "rejects http://0177.0.0.1 (octal-encoded)" 1 "$VALIDATE" "http://0177.0.0.1/"

# Userinfo segments are rejected outright (the `user@127.0.0.1@evil.com` confusion class)
assert_exit_code "rejects http://user@example.com (userinfo)" 1 "$VALIDATE" "http://user@example.com/" --allow-insecure-scheme
assert_exit_code "rejects http://user:pass@127.0.0.1@evil.com" 1 "$VALIDATE" "http://user:pass@127.0.0.1@evil.com/" --allow-internal --allow-insecure-scheme

# http-to-public-host: gated on --allow-insecure-scheme (NOT --allow-internal)
assert_exit_code "rejects http://example.com without flag" 1 "$VALIDATE" "http://example.com/"
assert_stderr_contains "http://example.com reject names insecure-scheme" "--allow-insecure-scheme" \
  "$VALIDATE" "http://example.com/"
assert_stderr_not_contains "http://example.com reject does NOT name --allow-internal" "internal address" \
  "$VALIDATE" "http://example.com/"
assert_exit_code "accepts http://example.com with --allow-insecure-scheme" 0 "$VALIDATE" "http://example.com/" --allow-insecure-scheme
assert_exit_code "rejects http://example.com with only --allow-internal" 1 "$VALIDATE" "http://example.com/" --allow-internal
assert_exit_code "accepts http://example.com with both flags" 0 "$VALIDATE" "http://example.com/" --allow-internal --allow-insecure-scheme

# Stale-text guard: the obsolete `(not available in v1)` parenthetical from the
# original script must be gone after this phase
assert_stderr_not_contains "no obsolete (not available in v1) text" "not available in v1" \
  "$VALIDATE" "http://10.0.0.1/"

# Stderr content checks for new default-allow path: no error printed
assert_stderr_empty "http://localhost succeeds silently" \
  "$VALIDATE" "http://localhost:8080"

# Stderr content checks for flag-gated cases: error names the right flag and the host
assert_stderr_contains "10.0.0.1 reject names --allow-internal" "--allow-internal" \
  "$VALIDATE" "http://10.0.0.1/"
assert_stderr_contains "10.0.0.1 reject names the host" "10.0.0.1" \
  "$VALIDATE" "http://10.0.0.1/"

# Unknown flags are rejected (don't silently become a location)
assert_exit_code "rejects unknown --alllow-internal (typo)" 2 "$VALIDATE" "http://localhost/" --alllow-internal

echo ""

echo "=== inventory-design: validate-source.sh helpers ==="
bash "$PLUGIN_ROOT/skills/design/inventory-design/scripts/test-validate-source.sh"

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
echo "An ordinary inventory body with no secrets." >"$CLEAN"
assert_exit_code "clean body passes scrubber" 0 \
  env -i ACCELERATOR_BROWSER_PASSWORD=hunter2_uniq "$SCRUB" "$CLEAN"

LEAKY="$(mktemp)"
echo "The reset link contains hunter2_uniq somewhere." >"$LEAKY"
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

echo "=== analyse-design-gaps: skill structure ==="

SKILL="$PLUGIN_ROOT/skills/design/analyse-design-gaps/SKILL.md"
assert_file_exists "analyse-design-gaps SKILL.md exists" "$SKILL"
assert_contains "name field set" "$(cat "$SKILL")" "name: analyse-design-gaps"
assert_contains "argument-hint two positional ids" \
  "$(cat "$SKILL")" 'argument-hint: "[current-source-id] [target-source-id]"'
assert_contains "instructs cue-phrase prose" \
  "$(cat "$SKILL")" "we need"
assert_contains "skill body invokes the cue-phrase audit script" \
  "$(cat "$SKILL")" "audit-cue-phrases.sh"
assert_file_exists "audit-cue-phrases.sh exists" \
  "$PLUGIN_ROOT/skills/design/analyse-design-gaps/scripts/audit-cue-phrases.sh"
assert_file_executable "audit-cue-phrases.sh is executable" \
  "$PLUGIN_ROOT/skills/design/analyse-design-gaps/scripts/audit-cue-phrases.sh"

echo ""

echo "=== analyse-design-gaps: audit-cue-phrases.sh behavioural ==="

AUDIT="$PLUGIN_ROOT/skills/design/analyse-design-gaps/scripts/audit-cue-phrases.sh"

COMPLIANT="$(mktemp)"
cat >"$COMPLIANT" <<'EOF'
# Gap

## Token Drift
We need to migrate the colour scale.

## Component Drift
Users need a five-variant Button.

## Screen Drift
The system must support a redesigned navigation pattern.

## Net-New Features
Implement Search to expose Cmd+K activation and recent-history previews.
EOF
assert_exit_code "audit passes on compliant fixture (all four cue patterns, capitalised)" 0 "$AUDIT" "$COMPLIANT"

NONCOMPLIANT="$(mktemp)"
cat >"$NONCOMPLIANT" <<'EOF'
# Gap

## Token Drift
The colours are different.

## Component Drift
We need a five-variant Button.
EOF
assert_exit_code "audit fails on non-compliant fixture" 1 "$AUDIT" "$NONCOMPLIANT"

LOWER_IMPL="$(mktemp)"
cat >"$LOWER_IMPL" <<'EOF'
# Gap

## Token Drift
implement foo to handle the colour migration.
EOF
assert_exit_code "audit fails when 'implement' is followed by lowercase" 1 "$AUDIT" "$LOWER_IMPL"

EMPTY_H2="$(mktemp)"
cat >"$EMPTY_H2" <<'EOF'
# Gap

## Token Drift

## Component Drift
We need a five-variant Button.
EOF
assert_exit_code "audit passes when an H2 is empty" 0 "$AUDIT" "$EMPTY_H2"

assert_file_exists "extract-work-items cue-phrase regex file exists" \
  "$PLUGIN_ROOT/scripts/extract-work-items-cue-phrases.txt"

rm -f "$COMPLIANT" "$NONCOMPLIANT" "$LOWER_IMPL" "$EMPTY_H2"
assert_contains "ends with skill-instructions hook" \
  "$(tail -n 5 "$SKILL")" "config-read-skill-instructions.sh analyse-design-gaps"

echo ""

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

echo "=== inventory-design: notify-downgrade.sh ==="
bash "$PLUGIN_ROOT/skills/design/inventory-design/scripts/test-notify-downgrade.sh"

echo ""

test_summary
