#!/usr/bin/env bash
set -euo pipefail

# Test harness for config reader scripts.
# Run: bash scripts/test-config.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
READ_VALUE="$SCRIPT_DIR/config-read-value.sh"
READ_CONTEXT="$SCRIPT_DIR/config-read-context.sh"
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
echo "=== Results ==="
echo "Passed: $PASS"
echo "Failed: $FAIL"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
echo "All tests passed!"
