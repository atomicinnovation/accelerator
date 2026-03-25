#!/usr/bin/env bash
set -euo pipefail

# Test harness for config reader scripts.
# Run: bash scripts/test-config.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
READ_VALUE="$SCRIPT_DIR/config-read-value.sh"
READ_CONTEXT="$SCRIPT_DIR/config-read-context.sh"
READ_AGENTS="$SCRIPT_DIR/config-read-agents.sh"
READ_AGENT_NAME="$SCRIPT_DIR/config-read-agent-name.sh"
READ_REVIEW="$SCRIPT_DIR/config-read-review.sh"
CONFIG_DUMP="$SCRIPT_DIR/config-dump.sh"
CONFIG_SUMMARY="$SCRIPT_DIR/config-summary.sh"
CONFIG_DETECT="$SCRIPT_DIR/../hooks/config-detect.sh"

# Source config-common.sh for direct function tests
source "$SCRIPT_DIR/config-common.sh"

PASS=0
FAIL=0

assert_eq() {
  local test_name="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected: $(printf '%q' "$expected")"
    echo "    Actual:   $(printf '%q' "$actual")"
    FAIL=$((FAIL + 1))
  fi
}

assert_exit_code() {
  local test_name="$1" expected_code="$2"
  shift 2
  local actual_code=0
  "$@" >/dev/null 2>&1 || actual_code=$?
  if [ "$expected_code" -eq "$actual_code" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected exit code: $expected_code"
    echo "    Actual exit code:   $actual_code"
    FAIL=$((FAIL + 1))
  fi
}

# Create a temporary directory base
TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# Helper: create a fake repo with .git dir
setup_repo() {
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$repo_dir/.git"
  echo "$repo_dir"
}

# ============================================================
echo "=== config_extract_frontmatter ==="
echo ""

echo "Test: File with valid frontmatter"
REPO=$(setup_repo)
cat > "$REPO/test.md" << 'FIXTURE'
---
key: value
other: thing
---

Body content here.
FIXTURE
OUTPUT=$(config_extract_frontmatter "$REPO/test.md")
EXPECTED=$(printf 'key: value\nother: thing')
assert_eq "outputs frontmatter content" "$EXPECTED" "$OUTPUT"

echo "Test: File with no frontmatter"
REPO=$(setup_repo)
cat > "$REPO/test.md" << 'FIXTURE'
# Just a heading

Some content.
FIXTURE
OUTPUT=$(config_extract_frontmatter "$REPO/test.md" 2>/dev/null || true)
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: File with unclosed frontmatter"
REPO=$(setup_repo)
cat > "$REPO/test.md" << 'FIXTURE'
---
key: value
no closing delimiter
FIXTURE
OUTPUT=$(config_extract_frontmatter "$REPO/test.md" 2>/dev/null || true)
assert_eq "outputs nothing" "" "$OUTPUT"
assert_exit_code "exits 1" 1 config_extract_frontmatter "$REPO/test.md"

echo "Test: --- on line 1 as frontmatter AND later in body"
REPO=$(setup_repo)
cat > "$REPO/test.md" << 'FIXTURE'
---
key: value
---

Body content.

---

More body after horizontal rule.
FIXTURE
OUTPUT=$(config_extract_frontmatter "$REPO/test.md")
assert_eq "only extracts between first and second ---" "key: value" "$OUTPUT"

echo "Test: Trailing spaces on --- delimiter"
REPO=$(setup_repo)
printf -- '---   \nkey: value\n---   \n\nBody.\n' > "$REPO/test.md"
OUTPUT=$(config_extract_frontmatter "$REPO/test.md")
assert_eq "still recognised as delimiter" "key: value" "$OUTPUT"

echo "Test: Empty frontmatter (--- on line 1 and --- on line 2)"
REPO=$(setup_repo)
printf -- '---\n---\n\nBody.\n' > "$REPO/test.md"
OUTPUT=$(config_extract_frontmatter "$REPO/test.md")
assert_eq "outputs nothing (empty frontmatter)" "" "$OUTPUT"

echo ""

# ============================================================
echo "=== config_extract_body ==="
echo ""

echo "Test: File with valid frontmatter and body"
REPO=$(setup_repo)
cat > "$REPO/test.md" << 'FIXTURE'
---
key: value
---

Body content here.
FIXTURE
OUTPUT=$(config_extract_body "$REPO/test.md")
EXPECTED=$(printf '\nBody content here.')
assert_eq "outputs only body after closing ---" "$EXPECTED" "$OUTPUT"

echo "Test: File with no frontmatter"
REPO=$(setup_repo)
cat > "$REPO/test.md" << 'FIXTURE'
# Just a heading

Some content.
FIXTURE
OUTPUT=$(config_extract_body "$REPO/test.md")
EXPECTED=$(printf '# Just a heading\n\nSome content.')
assert_eq "outputs entire file" "$EXPECTED" "$OUTPUT"

echo "Test: File with unclosed frontmatter"
REPO=$(setup_repo)
cat > "$REPO/test.md" << 'FIXTURE'
---
key: value
no closing delimiter
FIXTURE
OUTPUT=$(config_extract_body "$REPO/test.md")
assert_eq "outputs nothing (malformed)" "" "$OUTPUT"

echo "Test: --- horizontal rule in body after frontmatter"
REPO=$(setup_repo)
cat > "$REPO/test.md" << 'FIXTURE'
---
key: value
---

Body content.

---

More body after horizontal rule.
FIXTURE
OUTPUT=$(config_extract_body "$REPO/test.md")
EXPECTED=$(printf '\nBody content.\n\n---\n\nMore body after horizontal rule.')
assert_eq "includes horizontal rule in body" "$EXPECTED" "$OUTPUT"

echo "Test: Empty body after frontmatter"
REPO=$(setup_repo)
printf -- '---\nkey: value\n---\n' > "$REPO/test.md"
OUTPUT=$(config_extract_body "$REPO/test.md")
assert_eq "outputs nothing" "" "$OUTPUT"

echo ""

# ============================================================
echo "=== config-read-value.sh ==="
echo ""

echo "Test: No config files -> outputs default"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "agents.reviewer" "reviewer")
assert_eq "outputs default" "reviewer" "$OUTPUT"

echo "Test: Top-level key present"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
enabled: true
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "enabled" "false")
assert_eq "outputs value" "true" "$OUTPUT"

echo "Test: Nested key (section.key) present"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: senior-dev
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "agents.reviewer" "reviewer")
assert_eq "outputs nested value" "senior-dev" "$OUTPUT"

echo "Test: Key not found -> outputs default"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: senior-dev
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "agents.planner" "default-planner")
assert_eq "outputs default" "default-planner" "$OUTPUT"

echo "Test: Key not found, no default -> outputs nothing"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: senior-dev
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "agents.planner")
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: Local overrides team for same key"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: team-reviewer
---
FIXTURE
cat > "$REPO/.claude/accelerator.local.md" << 'FIXTURE'
---
agents:
  reviewer: my-reviewer
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "agents.reviewer" "default")
assert_eq "local overrides team" "my-reviewer" "$OUTPUT"

echo "Test: Values with double quotes are stripped"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
name: "quoted value"
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "name" "default")
assert_eq "strips double quotes" "quoted value" "$OUTPUT"

echo "Test: Values with single quotes are stripped"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- "---\nname: 'single quoted'\n---\n" > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "name" "default")
assert_eq "strips single quotes" "single quoted" "$OUTPUT"

echo "Test: Values with trailing whitespace are trimmed"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nname: hello   \n---\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "name" "default")
assert_eq "trims trailing whitespace" "hello" "$OUTPUT"

echo "Test: Empty frontmatter -> outputs default"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\n---\n\nBody.\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "name" "default")
assert_eq "outputs default" "default" "$OUTPUT"

echo "Test: No frontmatter (plain markdown file) -> outputs default"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
# Just a heading

Some content.
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "name" "default")
assert_eq "outputs default" "default" "$OUTPUT"

echo "Test: Array values are output as-is"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
lenses: [security, architecture, performance]
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "lenses" "default")
assert_eq "outputs array as-is" "[security, architecture, performance]" "$OUTPUT"

echo "Test: Values containing colons (URLs)"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
url: https://example.com/path
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "url" "default")
assert_eq "outputs URL correctly" "https://example.com/path" "$OUTPUT"

echo "Test: Blank line within a YAML section does not terminate scanning"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: senior-dev

  planner: architect
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "agents.planner" "default")
assert_eq "finds key after blank line" "architect" "$OUTPUT"

echo "Test: Key with underscore matches exactly"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_count: 5
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "review.max_count" "default")
assert_eq "matches underscore key" "5" "$OUTPUT"

echo "Test: Unclosed frontmatter -> outputs default, warning to stderr"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
key: value
no closing
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "key" "default" 2>/dev/null)
assert_eq "outputs default" "default" "$OUTPUT"
# Verify warning goes to stderr
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "key" "default" 2>&1 1>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning"; then
  echo "  PASS: warning emitted to stderr"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warning emitted to stderr"
  echo "    Expected: warning message on stderr"
  echo "    Actual:   $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo ""

# ============================================================
echo "=== config-read-context.sh ==="
echo ""

echo "Test: No config files -> outputs nothing"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_CONTEXT")
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: Team config with body"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
key: value
---

This is the project context.
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_CONTEXT")
EXPECTED=$(printf '## Project Context\n\nThe following project-specific context has been provided. Take this into\naccount when making decisions, selecting approaches, and generating output.\n\nThis is the project context.')
assert_eq "outputs body under Project Context header" "$EXPECTED" "$OUTPUT"

echo "Test: Local config with body"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.local.md" << 'FIXTURE'
---
key: value
---

My personal context.
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_CONTEXT")
EXPECTED=$(printf '## Project Context\n\nThe following project-specific context has been provided. Take this into\naccount when making decisions, selecting approaches, and generating output.\n\nMy personal context.')
assert_eq "outputs local body under Project Context header" "$EXPECTED" "$OUTPUT"

echo "Test: Both configs with bodies -> outputs both, team first"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
key: value
---

Team context.
FIXTURE
cat > "$REPO/.claude/accelerator.local.md" << 'FIXTURE'
---
key: value
---

Personal context.
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_CONTEXT")
EXPECTED=$(printf '## Project Context\n\nThe following project-specific context has been provided. Take this into\naccount when making decisions, selecting approaches, and generating output.\n\nTeam context.\n\nPersonal context.')
assert_eq "outputs both, team first" "$EXPECTED" "$OUTPUT"

echo "Test: Config with frontmatter but no body"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$READ_CONTEXT")
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: Config with empty body"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n\n\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$READ_CONTEXT")
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: Config with unclosed frontmatter -> outputs nothing"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
key: value
no closing
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_CONTEXT" 2>/dev/null)
assert_eq "outputs nothing (not entire file)" "" "$OUTPUT"

echo ""

# ============================================================
echo "=== config-summary.sh ==="
echo ""

echo "Test: No config files -> outputs nothing"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY")
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: Team config present -> lists it"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY")
if echo "$OUTPUT" | grep -q "Team config:"; then
  echo "  PASS: lists team config"
  PASS=$((PASS + 1))
else
  echo "  FAIL: lists team config"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Both configs present -> lists both"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.local.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY")
if echo "$OUTPUT" | grep -q "Team config:" && echo "$OUTPUT" | grep -q "Personal config:"; then
  echo "  PASS: lists both configs"
  PASS=$((PASS + 1))
else
  echo "  FAIL: lists both configs"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Config with frontmatter sections -> lists section names"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: senior-dev
review:
  max_count: 5
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY")
if echo "$OUTPUT" | grep -q "Configured sections:" && echo "$OUTPUT" | grep -q "agents" && echo "$OUTPUT" | grep -q "review"; then
  echo "  PASS: lists section names"
  PASS=$((PASS + 1))
else
  echo "  FAIL: lists section names"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Duplicate section keys across team and local -> deduplicated"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: team-val
---
FIXTURE
cat > "$REPO/.claude/accelerator.local.md" << 'FIXTURE'
---
agents:
  reviewer: local-val
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY")
# Count occurrences of "agents" in the Configured sections line
SECTIONS_LINE=$(echo "$OUTPUT" | grep "Configured sections:" || true)
AGENT_COUNT=$(echo "$SECTIONS_LINE" | grep -o "agents" | wc -l | tr -d ' ')
assert_eq "agents appears once (deduplicated)" "1" "$AGENT_COUNT"

echo "Test: Config with whitespace-only body -> no project context reported"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n\n   \n\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY")
if echo "$OUTPUT" | grep -q "Project context:"; then
  echo "  FAIL: should not report project context for whitespace-only body"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: no project context reported"
  PASS=$((PASS + 1))
fi

echo "Test: Config with frontmatter but no top-level keys -> no Configured sections"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\n---\n\nBody.\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY")
if echo "$OUTPUT" | grep -q "Configured sections:"; then
  echo "  FAIL: should not have Configured sections for empty frontmatter"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: no Configured sections line"
  PASS=$((PASS + 1))
fi

echo "Test: Section keys with hyphens and digits -> included in output"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
my-section2:
  key: value
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY")
if echo "$OUTPUT" | grep -q "my-section2"; then
  echo "  PASS: hyphen/digit keys included"
  PASS=$((PASS + 1))
else
  echo "  FAIL: hyphen/digit keys included"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Unclosed frontmatter -> warns to stderr, does not crash"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
key: value
no closing
FIXTURE
EXIT_CODE=0
STDERR_OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY" 2>&1 1>/dev/null) || EXIT_CODE=$?
if [ "$EXIT_CODE" -eq 0 ] && echo "$STDERR_OUTPUT" | grep -q "Warning"; then
  echo "  PASS: warns to stderr, does not crash"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warns to stderr, does not crash"
  echo "    Exit code: $EXIT_CODE"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo ""

# ============================================================
echo "=== config-detect.sh (hook output) ==="
echo ""

echo "Test: No config files -> outputs nothing"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DETECT")
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: Config present -> outputs valid JSON"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: senior-dev
---

Project context here.
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DETECT" 2>/dev/null)
# Validate JSON structure
if echo "$OUTPUT" | jq -e '.hookSpecificOutput.additionalContext' >/dev/null 2>&1; then
  echo "  PASS: outputs valid JSON with additionalContext"
  PASS=$((PASS + 1))
else
  echo "  FAIL: outputs valid JSON with additionalContext"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: JSON structure matches SessionStart hook contract"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DETECT" 2>/dev/null)
HOOK_EVENT=$(echo "$OUTPUT" | jq -r '.hookSpecificOutput.hookEventName' 2>/dev/null || true)
assert_eq "hookEventName is SessionStart" "SessionStart" "$HOOK_EVENT"

echo ""

# ============================================================
echo "=== config-read-agents.sh ==="
echo ""

echo "Test: No config files -> outputs nothing"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS")
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: Config with agents section -> outputs override table"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: my-custom-reviewer
  codebase-locator: my-locator
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS")
if echo "$OUTPUT" | grep -q "## Agent Overrides" && \
   echo "$OUTPUT" | grep -q '| `reviewer` | `my-custom-reviewer` |' && \
   echo "$OUTPUT" | grep -q '| `codebase-locator` | `my-locator` |'; then
  echo "  PASS: outputs override table with correct rows"
  PASS=$((PASS + 1))
else
  echo "  FAIL: outputs override table with correct rows"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Config with partial overrides -> only changed agents listed"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: my-reviewer
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS")
if echo "$OUTPUT" | grep -q '| `reviewer` | `my-reviewer` |' && \
   ! echo "$OUTPUT" | grep -q 'codebase-locator'; then
  echo "  PASS: only overridden agent listed"
  PASS=$((PASS + 1))
else
  echo "  FAIL: only overridden agent listed"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Local overrides team for same agent key"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: team-reviewer
---
FIXTURE
cat > "$REPO/.claude/accelerator.local.md" << 'FIXTURE'
---
agents:
  reviewer: local-reviewer
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS")
if echo "$OUTPUT" | grep -q '| `reviewer` | `local-reviewer` |' && \
   ! echo "$OUTPUT" | grep -q 'team-reviewer'; then
  echo "  PASS: local overrides team"
  PASS=$((PASS + 1))
else
  echo "  FAIL: local overrides team"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Non-overlapping overrides across team and local -> both appear"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: custom-reviewer
---
FIXTURE
cat > "$REPO/.claude/accelerator.local.md" << 'FIXTURE'
---
agents:
  codebase-locator: custom-locator
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS")
if echo "$OUTPUT" | grep -q '| `reviewer` | `custom-reviewer` |' && \
   echo "$OUTPUT" | grep -q '| `codebase-locator` | `custom-locator` |'; then
  echo "  PASS: both overrides appear"
  PASS=$((PASS + 1))
else
  echo "  FAIL: both overrides appear"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Unknown agent keys -> ignored with warning to stderr"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: my-reviewer
  unknown-agent: something
---
FIXTURE
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS" 2>&1 1>/dev/null)
OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS" 2>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning.*unknown-agent" && \
   ! echo "$OUTPUT" | grep -q 'unknown-agent'; then
  echo "  PASS: unknown key warned and ignored"
  PASS=$((PASS + 1))
else
  echo "  FAIL: unknown key warned and ignored"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Agent key with same value as default -> not listed as override"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: reviewer
  codebase-locator: my-locator
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS")
if ! echo "$OUTPUT" | grep -q '| `reviewer`' && \
   echo "$OUTPUT" | grep -q '| `codebase-locator` | `my-locator` |'; then
  echo "  PASS: identity override not listed"
  PASS=$((PASS + 1))
else
  echo "  FAIL: identity override not listed"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Table rows appear in fixed order (AGENT_KEYS order)"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  web-search-researcher: custom-web
  reviewer: custom-reviewer
  codebase-locator: custom-locator
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS")
# Extract just the table rows (lines starting with |, excluding header)
ROWS=$(echo "$OUTPUT" | grep '^| `' || true)
FIRST_ROW=$(echo "$ROWS" | head -1)
LAST_ROW=$(echo "$ROWS" | tail -1)
if echo "$FIRST_ROW" | grep -q 'reviewer' && \
   echo "$LAST_ROW" | grep -q 'web-search-researcher'; then
  echo "  PASS: rows in AGENT_KEYS order"
  PASS=$((PASS + 1))
else
  echo "  FAIL: rows in AGENT_KEYS order"
  echo "    Rows: $(printf '%q' "$ROWS")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Config with frontmatter but no agents section -> outputs nothing"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_count: 5
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS")
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: AGENT_KEYS list matches actual agent .md files in the plugin"
# Extract agent keys from the script source (not by sourcing it)
SCRIPT_KEYS=$(grep -A 20 '^AGENT_KEYS=(' "$READ_AGENTS" | sed -n '/^AGENT_KEYS=(/,/^)/p' | grep -v '^AGENT_KEYS=(' | grep -v '^)' | sed 's/^[[:space:]]*//' | sort)
# List actual agent .md files (strip path and extension)
AGENT_DIR="$SCRIPT_DIR/../agents"
FILE_KEYS=$(ls "$AGENT_DIR"/*.md 2>/dev/null | xargs -I{} basename {} .md | sort)
assert_eq "AGENT_KEYS matches agent files" "$FILE_KEYS" "$SCRIPT_KEYS"

echo ""

# ============================================================
echo "=== config-read-agent-name.sh ==="
echo ""

echo "Test: No config -> outputs the default agent name"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_AGENT_NAME" "reviewer")
assert_eq "outputs default" "reviewer" "$OUTPUT"

echo "Test: Config with override for requested agent -> outputs override value"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: my-custom-reviewer
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_AGENT_NAME" "reviewer")
assert_eq "outputs override" "my-custom-reviewer" "$OUTPUT"

echo "Test: Config with override for different agent -> outputs the default"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  codebase-locator: my-locator
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_AGENT_NAME" "reviewer")
assert_eq "outputs default" "reviewer" "$OUTPUT"

echo "Test: Local overrides team for same agent key"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: team-reviewer
---
FIXTURE
cat > "$REPO/.claude/accelerator.local.md" << 'FIXTURE'
---
agents:
  reviewer: local-reviewer
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_AGENT_NAME" "reviewer")
assert_eq "local overrides team" "local-reviewer" "$OUTPUT"

echo "Test: No argument -> exits with error"
REPO=$(setup_repo)
assert_exit_code "exits with error" 1 bash "$READ_AGENT_NAME"

echo ""

# ============================================================
echo "=== Preprocessor placement tests ==="
echo ""

SKILLS_DIR="$SCRIPT_DIR/../skills"

echo "Test: config-read-context.sh appears in exactly 13 skills"
CONTEXT_COUNT=$(grep -r 'config-read-context.sh' "$SKILLS_DIR" | wc -l | tr -d ' ')
assert_eq "13 skills have context injection" "13" "$CONTEXT_COUNT"

echo "Test: config-read-agents.sh appears in exactly 8 skills"
AGENTS_COUNT=$(grep -r 'config-read-agents.sh' "$SKILLS_DIR" | wc -l | tr -d ' ')
assert_eq "8 skills have agent override injection" "8" "$AGENTS_COUNT"

echo "Test: context injection is within a few lines of first # heading"
CONTEXT_SKILLS=(
  "planning/create-plan"
  "planning/review-plan"
  "planning/implement-plan"
  "planning/validate-plan"
  "planning/stress-test-plan"
  "research/research-codebase"
  "github/review-pr"
  "github/describe-pr"
  "github/respond-to-pr"
  "decisions/create-adr"
  "decisions/extract-adrs"
  "decisions/review-adr"
  "vcs/commit"
)
CONTEXT_PLACEMENT_OK=true
for skill in "${CONTEXT_SKILLS[@]}"; do
  SKILL_FILE="$SKILLS_DIR/$skill/SKILL.md"
  HEADING_LINE=$(grep -n '^# ' "$SKILL_FILE" | head -1 | cut -d: -f1)
  CONTEXT_LINE=$(grep -n 'config-read-context.sh' "$SKILL_FILE" | head -1 | cut -d: -f1)
  DIFF=$((CONTEXT_LINE - HEADING_LINE))
  if [ "$DIFF" -lt 1 ] || [ "$DIFF" -gt 5 ]; then
    echo "  FAIL: $skill - heading at line $HEADING_LINE, context at line $CONTEXT_LINE (diff=$DIFF)"
    CONTEXT_PLACEMENT_OK=false
    FAIL=$((FAIL + 1))
    break
  fi
done
if [ "$CONTEXT_PLACEMENT_OK" = true ]; then
  echo "  PASS: all context injections within 5 lines of heading"
  PASS=$((PASS + 1))
fi

echo "Test: config-read-agents.sh appears on line after config-read-context.sh"
AGENT_SKILLS=(
  "planning/create-plan"
  "planning/review-plan"
  "planning/stress-test-plan"
  "research/research-codebase"
  "github/review-pr"
  "decisions/create-adr"
  "decisions/extract-adrs"
  "decisions/review-adr"
)
AGENT_PLACEMENT_OK=true
for skill in "${AGENT_SKILLS[@]}"; do
  SKILL_FILE="$SKILLS_DIR/$skill/SKILL.md"
  CONTEXT_LINE=$(grep -n 'config-read-context.sh' "$SKILL_FILE" | head -1 | cut -d: -f1)
  AGENTS_LINE=$(grep -n 'config-read-agents.sh' "$SKILL_FILE" | head -1 | cut -d: -f1)
  EXPECTED_LINE=$((CONTEXT_LINE + 1))
  if [ "$AGENTS_LINE" -ne "$EXPECTED_LINE" ]; then
    echo "  FAIL: $skill - context at line $CONTEXT_LINE, agents at line $AGENTS_LINE (expected $EXPECTED_LINE)"
    AGENT_PLACEMENT_OK=false
    FAIL=$((FAIL + 1))
    break
  fi
done
if [ "$AGENT_PLACEMENT_OK" = true ]; then
  echo "  PASS: all agent overrides on line after context injection"
  PASS=$((PASS + 1))
fi

echo "Test: Non-agent skills do NOT have config-read-agents.sh"
NON_AGENT_SKILLS=(
  "planning/implement-plan"
  "planning/validate-plan"
  "github/describe-pr"
  "github/respond-to-pr"
  "vcs/commit"
)
NON_AGENT_OK=true
for skill in "${NON_AGENT_SKILLS[@]}"; do
  SKILL_FILE="$SKILLS_DIR/$skill/SKILL.md"
  if grep -q 'config-read-agents.sh' "$SKILL_FILE"; then
    echo "  FAIL: $skill should not have config-read-agents.sh"
    NON_AGENT_OK=false
    FAIL=$((FAIL + 1))
    break
  fi
done
if [ "$NON_AGENT_OK" = true ]; then
  echo "  PASS: non-agent skills correctly excluded"
  PASS=$((PASS + 1))
fi

echo "Test: review-pr and review-plan have inline config-read-agent-name.sh"
if grep -q 'config-read-agent-name.sh reviewer' "$SKILLS_DIR/github/review-pr/SKILL.md" && \
   grep -q 'config-read-agent-name.sh reviewer' "$SKILLS_DIR/planning/review-plan/SKILL.md"; then
  echo "  PASS: both review skills have inline agent name substitution"
  PASS=$((PASS + 1))
else
  echo "  FAIL: review skills missing inline agent name substitution"
  FAIL=$((FAIL + 1))
fi

echo ""

# ============================================================
echo "=== config_parse_array ==="
echo ""

echo "Test: Empty array [] -> no output"
OUTPUT=$(config_parse_array "[]")
assert_eq "empty array" "" "$OUTPUT"

echo "Test: Empty string -> no output"
OUTPUT=$(config_parse_array "")
assert_eq "empty string" "" "$OUTPUT"

echo "Test: Single element [architecture]"
OUTPUT=$(config_parse_array "[architecture]")
assert_eq "single element" "architecture" "$OUTPUT"

echo "Test: Multiple elements [a, b, c]"
OUTPUT=$(config_parse_array "[a, b, c]")
EXPECTED=$(printf 'a\nb\nc')
assert_eq "multiple elements" "$EXPECTED" "$OUTPUT"

echo "Test: Hyphenated names preserved [code-quality, test-coverage]"
OUTPUT=$(config_parse_array "[code-quality, test-coverage]")
EXPECTED=$(printf 'code-quality\ntest-coverage')
assert_eq "hyphenated names" "$EXPECTED" "$OUTPUT"

echo "Test: Inconsistent spacing [a,b, c , d]"
OUTPUT=$(config_parse_array "[a,b, c , d]")
EXPECTED=$(printf 'a\nb\nc\nd')
assert_eq "inconsistent spacing" "$EXPECTED" "$OUTPUT"

echo ""

# ============================================================
echo "=== config-read-review.sh ==="
echo ""

echo "Test: No argument -> exits with error"
assert_exit_code "exits with error" 1 bash "$READ_REVIEW"

echo "Test: Invalid argument -> exits with error"
assert_exit_code "invalid argument exits" 1 bash "$READ_REVIEW" "invalid"

echo "Test: No review config -> outputs nothing"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: Partial config (only some keys) -> outputs only changed values"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_inline_comments: 15
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "Max inline comments.*15" && \
   ! echo "$OUTPUT" | grep -q "Dedup proximity"; then
  echo "  PASS: outputs only changed values"
  PASS=$((PASS + 1))
else
  echo "  FAIL: outputs only changed values"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Full config -> outputs all overrides"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_inline_comments: 15
  dedup_proximity: 5
  min_lenses: 3
  max_lenses: 10
  core_lenses: [architecture, security, test-coverage, correctness]
  disabled_lenses: [portability, compatibility]
  pr_request_changes_severity: major
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "Max inline comments.*15" && \
   echo "$OUTPUT" | grep -q "Dedup proximity.*5" && \
   echo "$OUTPUT" | grep -q "Lens count range.*3 to 10" && \
   echo "$OUTPUT" | grep -q "Core lenses" && \
   echo "$OUTPUT" | grep -q "Disabled lenses" && \
   echo "$OUTPUT" | grep -q "Verdict"; then
  echo "  PASS: outputs all overrides"
  PASS=$((PASS + 1))
else
  echo "  FAIL: outputs all overrides"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: disabled_lenses array correctly parsed"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  disabled_lenses: [portability, compatibility]
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "Disabled lenses.*portability, compatibility"; then
  echo "  PASS: disabled_lenses parsed correctly"
  PASS=$((PASS + 1))
else
  echo "  FAIL: disabled_lenses parsed correctly"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: core_lenses with hyphenated names"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  core_lenses: [architecture, code-quality, test-coverage, correctness]
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "Core lenses.*architecture, code-quality, test-coverage, correctness"; then
  echo "  PASS: hyphenated core lenses preserved"
  PASS=$((PASS + 1))
else
  echo "  FAIL: hyphenated core lenses preserved"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Custom lens directory with valid SKILL.md -> listed in output"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/compliance-lens"
cat > "$REPO/.claude/accelerator/lenses/compliance-lens/SKILL.md" << 'FIXTURE'
---
name: compliance
description: Evaluates compliance
auto_detect: Relevant when changes touch compliance code
---

# Compliance Lens
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "| compliance |" && echo "$OUTPUT" | grep -q "| custom |"; then
  echo "  PASS: custom lens listed in output"
  PASS=$((PASS + 1))
else
  echo "  FAIL: custom lens listed in output"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Multiple custom lens directories -> all listed"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/compliance-lens"
mkdir -p "$REPO/.claude/accelerator/lenses/accessibility-lens"
cat > "$REPO/.claude/accelerator/lenses/compliance-lens/SKILL.md" << 'FIXTURE'
---
name: compliance
description: Compliance lens
auto_detect: Relevant for compliance
---
FIXTURE
cat > "$REPO/.claude/accelerator/lenses/accessibility-lens/SKILL.md" << 'FIXTURE'
---
name: accessibility
description: Accessibility lens
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "| compliance |" && \
   echo "$OUTPUT" | grep -q "| accessibility |"; then
  echo "  PASS: multiple custom lenses listed"
  PASS=$((PASS + 1))
else
  echo "  FAIL: multiple custom lenses listed"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Directory without SKILL.md -> skipped"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/empty-lens"
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: Custom lens with missing name in frontmatter -> warning, skipped"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/bad-lens"
cat > "$REPO/.claude/accelerator/lenses/bad-lens/SKILL.md" << 'FIXTURE'
---
description: Missing name field
---
FIXTURE
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>&1 1>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning.*missing.*name"; then
  echo "  PASS: warning for missing name"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warning for missing name"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Custom lens with same name as built-in -> warning to stderr"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/my-security-lens"
cat > "$REPO/.claude/accelerator/lenses/my-security-lens/SKILL.md" << 'FIXTURE'
---
name: security
description: Custom security lens
---
FIXTURE
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>&1 1>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning.*security.*conflicts"; then
  echo "  PASS: warning for name conflict"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warning for name conflict"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: No .claude/accelerator/lenses/ directory -> no custom lenses listed"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  min_lenses: 3
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if ! echo "$OUTPUT" | grep -q "custom"; then
  echo "  PASS: no custom lenses"
  PASS=$((PASS + 1))
else
  echo "  FAIL: no custom lenses"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Custom lens with auto_detect -> auto-detect criteria included"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/compliance-lens"
cat > "$REPO/.claude/accelerator/lenses/compliance-lens/SKILL.md" << 'FIXTURE'
---
name: compliance
description: Compliance lens
auto_detect: Relevant when changes touch compliance
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep "compliance" | grep -q "| custom |"; then
  echo "  PASS: custom lens with auto_detect shows 'custom'"
  PASS=$((PASS + 1))
else
  echo "  FAIL: custom lens with auto_detect shows 'custom'"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Custom lens without auto_detect -> shows 'always include'"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/accessibility-lens"
cat > "$REPO/.claude/accelerator/lenses/accessibility-lens/SKILL.md" << 'FIXTURE'
---
name: accessibility
description: Accessibility lens
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep "accessibility" | grep -q "always include"; then
  echo "  PASS: custom lens without auto_detect shows 'always include'"
  PASS=$((PASS + 1))
else
  echo "  FAIL: custom lens without auto_detect shows 'always include'"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Negative min_lenses -> warning to stderr, falls back to default"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  min_lenses: -1
---
FIXTURE
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>&1 1>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning.*min_lenses"; then
  echo "  PASS: warning for negative min_lenses"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warning for negative min_lenses"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: min_lenses > max_lenses -> warning to stderr, falls back to defaults"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  min_lenses: 10
  max_lenses: 5
---
FIXTURE
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>&1 1>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning.*min_lenses.*max_lenses"; then
  echo "  PASS: warning for min > max"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warning for min > max"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Unrecognised lens name in disabled_lenses -> warning to stderr"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  disabled_lenses: [code_quality]
---
FIXTURE
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>&1 1>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning.*unrecognised.*code_quality"; then
  echo "  PASS: warning for unrecognised lens"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warning for unrecognised lens"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Lens in both core_lenses and disabled_lenses -> warning to stderr"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  core_lenses: [architecture, security]
  disabled_lenses: [security]
---
FIXTURE
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>&1 1>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning.*security.*both"; then
  echo "  PASS: warning for core+disabled conflict"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warning for core+disabled conflict"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Invalid severity value -> warning to stderr, default used"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  pr_request_changes_severity: blocker
---
FIXTURE
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>&1 1>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning.*pr_request_changes_severity"; then
  echo "  PASS: warning for invalid severity"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warning for invalid severity"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Non-integer plan_revise_major_count -> warning to stderr, default used"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  plan_revise_major_count: abc
---
FIXTURE
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" plan 2>&1 1>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning.*plan_revise_major_count"; then
  echo "  PASS: warning for non-integer"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warning for non-integer"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: disabled_lenses disabling enough to drop below min_lenses -> warning"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  min_lenses: 12
  max_lenses: 13
  disabled_lenses: [portability, compatibility, safety, database]
---
FIXTURE
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>&1 1>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning.*lenses available"; then
  echo "  PASS: warning for insufficient lenses"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warning for insufficient lenses"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: pr_request_changes_severity: major -> output shows override"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  pr_request_changes_severity: major
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "Verdict.*REQUEST_CHANGES.*major"; then
  echo "  PASS: severity major shown in output"
  PASS=$((PASS + 1))
else
  echo "  FAIL: severity major shown in output"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: pr_request_changes_severity: none -> output shows disabled"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  pr_request_changes_severity: none
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "Verdict.*disabled"; then
  echo "  PASS: severity none disables verdict"
  PASS=$((PASS + 1))
else
  echo "  FAIL: severity none disables verdict"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: plan_revise_severity: none -> output shows disabled"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  plan_revise_severity: none
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
if echo "$OUTPUT" | grep -q "severity-based REVISE disabled"; then
  echo "  PASS: plan severity none disables"
  PASS=$((PASS + 1))
else
  echo "  FAIL: plan severity none disables"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: plan_revise_major_count: 2 -> output shows override"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  plan_revise_major_count: 2
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
if echo "$OUTPUT" | grep -q "2+.*major"; then
  echo "  PASS: major count override shown"
  PASS=$((PASS + 1))
else
  echo "  FAIL: major count override shown"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: plan_revise_severity: critical (same as default) -> not listed as override"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  plan_revise_severity: critical
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
assert_eq "outputs nothing for default severity" "" "$OUTPUT"

echo "Test: Lens Catalogue always present when config exists"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  min_lenses: 3
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "### Lens Catalogue" && \
   echo "$OUTPUT" | grep -q "| architecture |" && \
   echo "$OUTPUT" | grep -q "| usability |"; then
  echo "  PASS: lens catalogue present with all 13 built-in lenses"
  PASS=$((PASS + 1))
else
  echo "  FAIL: lens catalogue present with all 13 built-in lenses"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: PR mode does not include plan-specific settings"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_inline_comments: 15
  plan_revise_major_count: 2
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "Max inline comments" && \
   ! echo "$OUTPUT" | grep -q "plan_revise"; then
  echo "  PASS: PR mode excludes plan settings"
  PASS=$((PASS + 1))
else
  echo "  FAIL: PR mode excludes plan settings"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Plan mode does not include PR-specific settings"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_inline_comments: 15
  plan_revise_major_count: 2
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
if ! echo "$OUTPUT" | grep -q "Max inline comments" && \
   echo "$OUTPUT" | grep -q "2+.*major"; then
  echo "  PASS: Plan mode excludes PR settings"
  PASS=$((PASS + 1))
else
  echo "  FAIL: Plan mode excludes PR settings"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Config variable names in PR output match review-pr SKILL.md"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_inline_comments: 15
  dedup_proximity: 5
  min_lenses: 3
  max_lenses: 10
  core_lenses: [architecture, security]
  disabled_lenses: [portability]
  pr_request_changes_severity: major
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
REVIEW_PR="$SCRIPT_DIR/../skills/github/review-pr/SKILL.md"
CONFIG_OK=true
for var_name in max_inline_comments dedup_proximity min_lenses max_lenses core_lenses disabled_lenses pr_request_changes_severity; do
  if ! grep -q "$var_name" "$REVIEW_PR"; then
    echo "  FAIL: $var_name not found in review-pr SKILL.md"
    CONFIG_OK=false
    FAIL=$((FAIL + 1))
    break
  fi
done
if [ "$CONFIG_OK" = true ]; then
  echo "  PASS: all PR config variable names appear in review-pr SKILL.md"
  PASS=$((PASS + 1))
fi

echo "Test: Config variable names in plan output match review-plan SKILL.md"
REVIEW_PLAN="$SCRIPT_DIR/../skills/planning/review-plan/SKILL.md"
PLAN_CONFIG_OK=true
for var_name in min_lenses max_lenses core_lenses disabled_lenses plan_revise_severity plan_revise_major_count; do
  if ! grep -q "$var_name" "$REVIEW_PLAN"; then
    echo "  FAIL: $var_name not found in review-plan SKILL.md"
    PLAN_CONFIG_OK=false
    FAIL=$((FAIL + 1))
    break
  fi
done
if [ "$PLAN_CONFIG_OK" = true ]; then
  echo "  PASS: all plan config variable names appear in review-plan SKILL.md"
  PASS=$((PASS + 1))
fi

echo "Test: No config files -> empty output (regression guard)"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
assert_eq "outputs nothing" "" "$OUTPUT"

echo ""

# ============================================================
echo "=== config-dump.sh ==="
echo ""

echo "Test: No config files -> outputs nothing"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: Team-only config -> all keys shown with correct source"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_inline_comments: 15
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
if echo "$OUTPUT" | grep -q "review.max_inline_comments.*15.*team"; then
  echo "  PASS: team source attribution correct"
  PASS=$((PASS + 1))
else
  echo "  FAIL: team source attribution correct"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Local-only config -> keys shown with local source"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.local.md" << 'FIXTURE'
---
review:
  max_inline_comments: 20
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
if echo "$OUTPUT" | grep -q "review.max_inline_comments.*20.*local"; then
  echo "  PASS: local source attribution correct"
  PASS=$((PASS + 1))
else
  echo "  FAIL: local source attribution correct"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Merged config -> overridden key shows local source"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_inline_comments: 15
  min_lenses: 3
---
FIXTURE
cat > "$REPO/.claude/accelerator.local.md" << 'FIXTURE'
---
review:
  max_inline_comments: 20
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
if echo "$OUTPUT" | grep "review.max_inline_comments" | grep -q "local" && \
   echo "$OUTPUT" | grep "review.min_lenses" | grep -q "team"; then
  echo "  PASS: merged config shows correct sources"
  PASS=$((PASS + 1))
else
  echo "  FAIL: merged config shows correct sources"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Default keys shown with default source"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_inline_comments: 15
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
if echo "$OUTPUT" | grep "review.dedup_proximity" | grep -q "default"; then
  echo "  PASS: default source attribution correct"
  PASS=$((PASS + 1))
else
  echo "  FAIL: default source attribution correct"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: All review config keys appear in output (completeness check)"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nreview:\n  max_inline_comments: 15\n---\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
ALL_KEYS_OK=true
for key in review.max_inline_comments review.min_lenses review.max_lenses review.dedup_proximity review.core_lenses review.disabled_lenses review.pr_request_changes_severity review.plan_revise_severity review.plan_revise_major_count; do
  if ! echo "$OUTPUT" | grep -q "$key"; then
    echo "  FAIL: missing key $key"
    ALL_KEYS_OK=false
    FAIL=$((FAIL + 1))
    break
  fi
done
if [ "$ALL_KEYS_OK" = true ]; then
  echo "  PASS: all review config keys present"
  PASS=$((PASS + 1))
fi

echo ""

# ============================================================
echo "=== Preprocessor placement: config-read-review.sh ==="
echo ""

echo "Test: config-read-review.sh appears in review-pr SKILL.md"
if grep -q 'config-read-review.sh pr' "$SKILLS_DIR/github/review-pr/SKILL.md"; then
  echo "  PASS: review-pr has review config injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: review-pr has review config injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: config-read-review.sh appears in review-plan SKILL.md"
if grep -q 'config-read-review.sh plan' "$SKILLS_DIR/planning/review-plan/SKILL.md"; then
  echo "  PASS: review-plan has review config injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: review-plan has review config injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: config-read-review.sh appears on line after config-read-agents.sh in review-pr"
AGENTS_LINE=$(grep -n 'config-read-agents.sh' "$SKILLS_DIR/github/review-pr/SKILL.md" | head -1 | cut -d: -f1)
REVIEW_LINE=$(grep -n 'config-read-review.sh' "$SKILLS_DIR/github/review-pr/SKILL.md" | head -1 | cut -d: -f1)
EXPECTED_LINE=$((AGENTS_LINE + 1))
assert_eq "review config on line after agents in review-pr" "$EXPECTED_LINE" "$REVIEW_LINE"

echo "Test: config-read-review.sh appears on line after config-read-agents.sh in review-plan"
AGENTS_LINE=$(grep -n 'config-read-agents.sh' "$SKILLS_DIR/planning/review-plan/SKILL.md" | head -1 | cut -d: -f1)
REVIEW_LINE=$(grep -n 'config-read-review.sh' "$SKILLS_DIR/planning/review-plan/SKILL.md" | head -1 | cut -d: -f1)
EXPECTED_LINE=$((AGENTS_LINE + 1))
assert_eq "review config on line after agents in review-plan" "$EXPECTED_LINE" "$REVIEW_LINE"

echo ""

# ============================================================
echo "=== config-read-path.sh ==="
echo ""

READ_PATH="$SCRIPT_DIR/config-read-path.sh"

echo "Test: No paths config -> outputs default"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" "plans" "meta/plans")
assert_eq "outputs default" "meta/plans" "$OUTPUT"

echo "Test: paths.plans configured -> outputs configured value"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  plans: docs/plans
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" "plans" "meta/plans")
assert_eq "outputs configured path" "docs/plans" "$OUTPUT"

echo "Test: paths.decisions configured -> outputs configured value"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  decisions: docs/adrs
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" "decisions" "meta/decisions")
assert_eq "outputs configured path" "docs/adrs" "$OUTPUT"

echo "Test: paths.review_plans configured"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  review_plans: docs/reviews/plans
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" "review_plans" "meta/reviews/plans")
assert_eq "outputs configured path" "docs/reviews/plans" "$OUTPUT"

echo "Test: paths.review_prs configured"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  review_prs: docs/reviews/prs
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" "review_prs" "meta/reviews/prs")
assert_eq "outputs configured path" "docs/reviews/prs" "$OUTPUT"

echo "Test: paths.templates configured"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  templates: docs/templates
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" "templates" "meta/templates")
assert_eq "outputs configured path" "docs/templates" "$OUTPUT"

echo "Test: paths.tickets configured"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  tickets: docs/tickets
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" "tickets" "meta/tickets")
assert_eq "outputs configured path" "docs/tickets" "$OUTPUT"

echo "Test: paths.notes configured"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  notes: docs/notes
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" "notes" "meta/notes")
assert_eq "outputs configured path" "docs/notes" "$OUTPUT"

echo "Test: Absolute path is output as-is"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  plans: /opt/docs/plans
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" "plans" "meta/plans")
assert_eq "outputs absolute path" "/opt/docs/plans" "$OUTPUT"

echo "Test: Local overrides team for paths"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  plans: team/plans
---
FIXTURE
cat > "$REPO/.claude/accelerator.local.md" << 'FIXTURE'
---
paths:
  plans: my/plans
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" "plans" "meta/plans")
assert_eq "local overrides team" "my/plans" "$OUTPUT"

echo ""

# ============================================================
echo "=== config-read-template.sh ==="
echo ""

READ_TEMPLATE="$SCRIPT_DIR/config-read-template.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Test: No user template -> outputs plugin default wrapped in code fences"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_TEMPLATE" "plan")
FIRST_LINE=$(echo "$OUTPUT" | head -1)
LAST_LINE=$(echo "$OUTPUT" | tail -1)
assert_eq "starts with code fence" '```markdown' "$FIRST_LINE"
assert_eq "ends with code fence" '```' "$LAST_LINE"
if echo "$OUTPUT" | grep -q "## Overview"; then
  echo "  PASS: contains plan template content"
  PASS=$((PASS + 1))
else
  echo "  FAIL: contains plan template content"
  FAIL=$((FAIL + 1))
fi

echo "Test: Template in configured templates directory (default meta/templates/) -> outputs user template"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
cat > "$REPO/meta/templates/plan.md" << 'FIXTURE'
# Custom Plan Template

## My Custom Section
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_TEMPLATE" "plan")
FIRST_LINE=$(echo "$OUTPUT" | head -1)
assert_eq "starts with code fence" '```markdown' "$FIRST_LINE"
if echo "$OUTPUT" | grep -q "My Custom Section"; then
  echo "  PASS: contains user template content"
  PASS=$((PASS + 1))
else
  echo "  FAIL: contains user template content"
  FAIL=$((FAIL + 1))
fi

echo "Test: paths.templates overridden -> looks in overridden directory"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
mkdir -p "$REPO/docs/templates"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  templates: docs/templates
---
FIXTURE
cat > "$REPO/docs/templates/plan.md" << 'FIXTURE'
# Overridden Directory Plan Template
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_TEMPLATE" "plan")
if echo "$OUTPUT" | grep -q "Overridden Directory Plan Template"; then
  echo "  PASS: finds template in overridden directory"
  PASS=$((PASS + 1))
else
  echo "  FAIL: finds template in overridden directory"
  FAIL=$((FAIL + 1))
fi

echo "Test: Config path specified (templates.<name>) and exists -> takes precedence"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
mkdir -p "$REPO/custom"
mkdir -p "$REPO/meta/templates"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
templates:
  plan: custom/my-plan.md
---
FIXTURE
cat > "$REPO/custom/my-plan.md" << 'FIXTURE'
# Config-Specified Plan
FIXTURE
cat > "$REPO/meta/templates/plan.md" << 'FIXTURE'
# Templates-Dir Plan (should NOT be used)
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_TEMPLATE" "plan")
if echo "$OUTPUT" | grep -q "Config-Specified Plan"; then
  echo "  PASS: config path takes precedence over templates dir"
  PASS=$((PASS + 1))
else
  echo "  FAIL: config path takes precedence over templates dir"
  FAIL=$((FAIL + 1))
fi

echo "Test: Template file already starts with code fence -> output as-is (no double-wrapping)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
printf '```markdown\n# Already Fenced\n```\n' > "$REPO/meta/templates/plan.md"
OUTPUT=$(cd "$REPO" && bash "$READ_TEMPLATE" "plan")
# Count occurrences of ```markdown - should be exactly 1
FENCE_COUNT=$(echo "$OUTPUT" | grep -c '```markdown' || true)
assert_eq "no double-wrapping" "1" "$FENCE_COUNT"

echo "Test: Config path specified but missing -> falls back to plugin default with warning"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
templates:
  plan: nonexistent/plan.md
---
FIXTURE
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_TEMPLATE" "plan" 2>&1 1>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning"; then
  echo "  PASS: warning emitted to stderr"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warning emitted to stderr"
  FAIL=$((FAIL + 1))
fi
OUTPUT=$(cd "$REPO" && bash "$READ_TEMPLATE" "plan" 2>/dev/null)
if echo "$OUTPUT" | grep -q "## Overview"; then
  echo "  PASS: falls back to plugin default"
  PASS=$((PASS + 1))
else
  echo "  FAIL: falls back to plugin default"
  FAIL=$((FAIL + 1))
fi

echo "Test: Config path specified as relative -> resolved against project root"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
mkdir -p "$REPO/relative/path"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
templates:
  plan: relative/path/plan.md
---
FIXTURE
cat > "$REPO/relative/path/plan.md" << 'FIXTURE'
# Relative Path Plan
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_TEMPLATE" "plan")
if echo "$OUTPUT" | grep -q "Relative Path Plan"; then
  echo "  PASS: relative path resolved correctly"
  PASS=$((PASS + 1))
else
  echo "  FAIL: relative path resolved correctly"
  FAIL=$((FAIL + 1))
fi

echo "Test: Config path specified as absolute -> used as-is"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
ABS_TEMPLATE=$(mktemp "$TMPDIR_BASE/abs-template-XXXXXX.md")
cat > "$ABS_TEMPLATE" << 'FIXTURE'
# Absolute Path Plan
FIXTURE
cat > "$REPO/.claude/accelerator.md" << FIXTURE
---
templates:
  plan: $ABS_TEMPLATE
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_TEMPLATE" "plan")
if echo "$OUTPUT" | grep -q "Absolute Path Plan"; then
  echo "  PASS: absolute path used as-is"
  PASS=$((PASS + 1))
else
  echo "  FAIL: absolute path used as-is"
  FAIL=$((FAIL + 1))
fi

echo "Test: Unknown template name -> error listing available template names"
REPO=$(setup_repo)
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_TEMPLATE" "nonexistent" 2>&1 1>/dev/null || true)
if echo "$STDERR_OUTPUT" | grep -q "plan, research, adr, validation"; then
  echo "  PASS: error lists available templates"
  PASS=$((PASS + 1))
else
  echo "  FAIL: error lists available templates"
  echo "    Actual stderr: $STDERR_OUTPUT"
  FAIL=$((FAIL + 1))
fi
assert_exit_code "exits 1 for unknown template" 1 bash "$READ_TEMPLATE" "nonexistent"

echo ""

# ============================================================
echo "=== adr-next-number.sh with path overrides ==="
echo ""

ADR_NEXT="$SCRIPT_DIR/../skills/decisions/scripts/adr-next-number.sh"

echo "Test: Default path behaviour preserved when no config exists"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
cp "$TMPDIR_BASE/repo-"*/meta/decisions/ADR-* "$REPO/meta/decisions/" 2>/dev/null || true
# Create a sample ADR file
cat > "$REPO/meta/decisions/ADR-0005-test.md" << 'FIXTURE'
---
adr_id: ADR-0005
status: proposed
---
# ADR-0005
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$ADR_NEXT")
assert_eq "outputs 0006" "0006" "$OUTPUT"

echo "Test: With paths.decisions configured, scans custom directory"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
mkdir -p "$REPO/custom/adrs"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  decisions: custom/adrs
---
FIXTURE
cat > "$REPO/custom/adrs/ADR-0003-test.md" << 'FIXTURE'
---
adr_id: ADR-0003
status: proposed
---
# ADR-0003
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$ADR_NEXT")
assert_eq "scans custom directory" "0004" "$OUTPUT"

echo "Test: With configured directory that does not exist, warns and returns 0001"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  decisions: nonexistent/adrs
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$ADR_NEXT" 2>/dev/null)
assert_eq "returns 0001" "0001" "$OUTPUT"
STDERR_OUTPUT=$(cd "$REPO" && bash "$ADR_NEXT" 2>&1 1>/dev/null)
if echo "$STDERR_OUTPUT" | grep -q "Warning"; then
  echo "  PASS: warning emitted to stderr"
  PASS=$((PASS + 1))
else
  echo "  FAIL: warning emitted to stderr"
  FAIL=$((FAIL + 1))
fi

echo ""

# ============================================================
echo "=== Skill integration checks ==="
echo ""

SKILLS_DIR="$SCRIPT_DIR/../skills"

echo "Test: create-plan uses config-read-path.sh"
if grep -q 'config-read-path.sh plans' "$SKILLS_DIR/planning/create-plan/SKILL.md"; then
  echo "  PASS: create-plan has plans path injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: create-plan has plans path injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: create-plan uses config-read-template.sh"
if grep -q 'config-read-template.sh plan' "$SKILLS_DIR/planning/create-plan/SKILL.md"; then
  echo "  PASS: create-plan has template injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: create-plan has template injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: research-codebase uses config-read-path.sh"
if grep -q 'config-read-path.sh research' "$SKILLS_DIR/research/research-codebase/SKILL.md"; then
  echo "  PASS: research-codebase has research path injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: research-codebase has research path injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: research-codebase uses config-read-template.sh"
if grep -q 'config-read-template.sh research' "$SKILLS_DIR/research/research-codebase/SKILL.md"; then
  echo "  PASS: research-codebase has template injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: research-codebase has template injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: create-adr uses config-read-path.sh"
if grep -q 'config-read-path.sh decisions' "$SKILLS_DIR/decisions/create-adr/SKILL.md"; then
  echo "  PASS: create-adr has decisions path injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: create-adr has decisions path injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: create-adr uses config-read-template.sh"
if grep -q 'config-read-template.sh adr' "$SKILLS_DIR/decisions/create-adr/SKILL.md"; then
  echo "  PASS: create-adr has template injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: create-adr has template injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: extract-adrs uses config-read-path.sh for decisions, research, plans"
EXTRACT_SKILL="$SKILLS_DIR/decisions/extract-adrs/SKILL.md"
EXTRACT_PASS=true
for key in decisions research plans; do
  if ! grep -q "config-read-path.sh $key" "$EXTRACT_SKILL"; then
    EXTRACT_PASS=false
    break
  fi
done
if $EXTRACT_PASS; then
  echo "  PASS: extract-adrs has all path injections"
  PASS=$((PASS + 1))
else
  echo "  FAIL: extract-adrs has all path injections"
  FAIL=$((FAIL + 1))
fi

echo "Test: review-adr uses config-read-path.sh"
if grep -q 'config-read-path.sh decisions' "$SKILLS_DIR/decisions/review-adr/SKILL.md"; then
  echo "  PASS: review-adr has decisions path injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: review-adr has decisions path injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: validate-plan uses config-read-path.sh"
if grep -q 'config-read-path.sh validations' "$SKILLS_DIR/planning/validate-plan/SKILL.md"; then
  echo "  PASS: validate-plan has validations path injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: validate-plan has validations path injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: validate-plan uses config-read-template.sh"
if grep -q 'config-read-template.sh validation' "$SKILLS_DIR/planning/validate-plan/SKILL.md"; then
  echo "  PASS: validate-plan has template injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: validate-plan has template injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: describe-pr uses config-read-path.sh for prs and templates"
DESCRIBE_SKILL="$SKILLS_DIR/github/describe-pr/SKILL.md"
DESCRIBE_PASS=true
for key in prs templates; do
  if ! grep -q "config-read-path.sh $key" "$DESCRIBE_SKILL"; then
    DESCRIBE_PASS=false
    break
  fi
done
if $DESCRIBE_PASS; then
  echo "  PASS: describe-pr has prs and templates path injections"
  PASS=$((PASS + 1))
else
  echo "  FAIL: describe-pr has prs and templates path injections"
  FAIL=$((FAIL + 1))
fi

echo "Test: review-plan uses config-read-path.sh"
if grep -q 'config-read-path.sh review_plans' "$SKILLS_DIR/planning/review-plan/SKILL.md"; then
  echo "  PASS: review-plan has review_plans path injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: review-plan has review_plans path injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: review-pr uses config-read-path.sh"
if grep -q 'config-read-path.sh review_prs' "$SKILLS_DIR/github/review-pr/SKILL.md"; then
  echo "  PASS: review-pr has review_prs path injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: review-pr has review_prs path injection"
  FAIL=$((FAIL + 1))
fi

echo ""

# ============================================================
echo "=== Results ==="
echo "Passed: $PASS"
echo "Failed: $FAIL"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
echo "All tests passed!"
