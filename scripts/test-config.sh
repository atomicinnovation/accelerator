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
READ_SKILL_CONTEXT="$SCRIPT_DIR/config-read-skill-context.sh"
READ_SKILL_INSTRUCTIONS="$SCRIPT_DIR/config-read-skill-instructions.sh"

# Source config-common.sh for direct function tests
source "$SCRIPT_DIR/config-common.sh"

PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

assert_contains() {
  local test_name="$1" needle="$2" haystack="$3"
  if printf '%s' "$haystack" | grep -qF "$needle"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected to contain: $needle"
    echo "    Actual: $haystack"
    FAIL=$((FAIL + 1))
  fi
}

assert_empty() {
  local test_name="$1" actual="$2"
  if [ -z "$actual" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected empty, got: $actual"
    FAIL=$((FAIL + 1))
  fi
}

assert_file_exists() {
  local test_name="$1" file_path="$2"
  if [ -f "$file_path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected file to exist: $file_path"
    FAIL=$((FAIL + 1))
  fi
}

assert_file_not_exists() {
  local test_name="$1" file_path="$2"
  if [ ! -f "$file_path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected file to not exist: $file_path"
    FAIL=$((FAIL + 1))
  fi
}

assert_file_content_eq() {
  local test_name="$1" file_path="$2" expected="$3"
  local actual
  actual=$(cat "$file_path" 2>/dev/null) || {
    echo "  FAIL: $test_name"
    echo "    File not found: $file_path"
    FAIL=$((FAIL + 1))
    return
  }
  if [ "$expected" = "$actual" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected content: $(printf '%q' "$expected")"
    echo "    Actual content:   $(printf '%q' "$actual")"
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

echo "Test: work.id_pattern reads from team config"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
work:
  id_pattern: "{project}-{number:04d}"
  default_project_code: "PROJ"
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "work.id_pattern" "{number:04d}")
assert_eq "reads work.id_pattern" "{project}-{number:04d}" "$OUTPUT"
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "work.default_project_code" "")
assert_eq "reads work.default_project_code" "PROJ" "$OUTPUT"

echo "Test: work.id_pattern defaults when unset"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: senior-dev
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "work.id_pattern" "{number:04d}")
assert_eq "default returned" "{number:04d}" "$OUTPUT"
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "work.default_project_code" "")
assert_eq "empty returned" "" "$OUTPUT"

echo "Test: work.id_pattern local override wins"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
work:
  id_pattern: "{number:04d}"
  default_project_code: ""
---
FIXTURE
cat > "$REPO/.claude/accelerator.local.md" << 'FIXTURE'
---
work:
  id_pattern: "{project}-{number:04d}"
  default_project_code: "ENG"
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "work.id_pattern" "{number:04d}")
assert_eq "local wins for id_pattern" "{project}-{number:04d}" "$OUTPUT"
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "work.default_project_code" "")
assert_eq "local wins for default_project_code" "ENG" "$OUTPUT"

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

echo "Test: No config files -> outputs nothing (initialised repo)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tmp" && touch "$REPO/meta/tmp/.gitignore"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY")
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: No config files, uninitialised repo -> outputs init hint"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY")
assert_contains "init hint shown" "has not been initialised" "$OUTPUT"

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

echo "Test: No config files -> outputs nothing (initialised repo)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tmp" && touch "$REPO/meta/tmp/.gitignore"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DETECT")
assert_eq "outputs nothing" "" "$OUTPUT"

echo "Test: No config files, uninitialised repo -> outputs init hint JSON"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DETECT")
assert_contains "init hint in JSON" "has not been initialised" "$OUTPUT"

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

echo "Test: No config files -> outputs all agents with default names"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS")
if echo "$OUTPUT" | grep -q "## Agent Names" && \
   echo "$OUTPUT" | grep -q '\- \*\*reviewer agent\*\*: accelerator:reviewer' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase locator agent\*\*: accelerator:codebase-locator' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase analyser agent\*\*: accelerator:codebase-analyser' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase pattern finder agent\*\*: accelerator:codebase-pattern-finder' && \
   echo "$OUTPUT" | grep -q '\- \*\*documents locator agent\*\*: accelerator:documents-locator' && \
   echo "$OUTPUT" | grep -q '\- \*\*documents analyser agent\*\*: accelerator:documents-analyser' && \
   echo "$OUTPUT" | grep -q '\- \*\*web search researcher agent\*\*: accelerator:web-search-researcher'; then
  echo "  PASS: outputs all 7 agents with default names"
  PASS=$((PASS + 1))
else
  echo "  FAIL: outputs all 7 agents with default names"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Config with agents section -> outputs labeled agent names with overrides"
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
if echo "$OUTPUT" | grep -q "## Agent Names" && \
   echo "$OUTPUT" | grep -q '\- \*\*reviewer agent\*\*: my-custom-reviewer' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase locator agent\*\*: my-locator'; then
  echo "  PASS: outputs labeled agent names with overrides"
  PASS=$((PASS + 1))
else
  echo "  FAIL: outputs labeled agent names with overrides"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Config with partial overrides -> overridden agent shows new name, others show defaults"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: my-reviewer
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS")
if echo "$OUTPUT" | grep -q '\- \*\*reviewer agent\*\*: my-reviewer' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase locator agent\*\*: accelerator:codebase-locator'; then
  echo "  PASS: overridden agent shows new name, others show defaults"
  PASS=$((PASS + 1))
else
  echo "  FAIL: overridden agent shows new name, others show defaults"
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
if echo "$OUTPUT" | grep -q '\- \*\*reviewer agent\*\*: local-reviewer' && \
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
if echo "$OUTPUT" | grep -q '\- \*\*reviewer agent\*\*: custom-reviewer' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase locator agent\*\*: custom-locator'; then
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

echo "Test: Agent key with same value as default -> shows default name"
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
if echo "$OUTPUT" | grep -q '\- \*\*reviewer agent\*\*: reviewer' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase locator agent\*\*: my-locator'; then
  echo "  PASS: identity override shows default name, other override applied"
  PASS=$((PASS + 1))
else
  echo "  FAIL: identity override shows default name, other override applied"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Agent lines appear in fixed order (AGENT_KEYS order)"
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
# Extract just the agent lines (lines starting with -)
ROWS=$(echo "$OUTPUT" | grep '^- \*\*' || true)
FIRST_ROW=$(echo "$ROWS" | head -1)
LAST_ROW=$(echo "$ROWS" | tail -1)
if echo "$FIRST_ROW" | grep -q 'reviewer agent' && \
   echo "$LAST_ROW" | grep -q 'web search researcher agent'; then
  echo "  PASS: rows in AGENT_KEYS order"
  PASS=$((PASS + 1))
else
  echo "  FAIL: rows in AGENT_KEYS order"
  echo "    Rows: $(printf '%q' "$ROWS")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Config with frontmatter but no agents section -> outputs all defaults"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_count: 5
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_AGENTS")
if echo "$OUTPUT" | grep -q "## Agent Names" && \
   echo "$OUTPUT" | grep -q '\- \*\*reviewer agent\*\*: accelerator:reviewer'; then
  echo "  PASS: outputs all defaults when no agents section"
  PASS=$((PASS + 1))
else
  echo "  FAIL: outputs all defaults when no agents section"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

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
assert_eq "outputs default" "accelerator:reviewer" "$OUTPUT"

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
assert_eq "outputs default" "accelerator:reviewer" "$OUTPUT"

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

echo "Test: config-read-context.sh appears in exactly 21 skills"
CONTEXT_COUNT=$(grep -r --exclude-dir=node_modules 'config-read-context.sh' "$SKILLS_DIR" | wc -l | tr -d ' ')
assert_eq "21 skills have context injection" "21" "$CONTEXT_COUNT"

echo "Test: config-read-agents.sh appears in exactly 17 skills"
AGENTS_COUNT=$(grep -r --exclude-dir=node_modules 'config-read-agents.sh' "$SKILLS_DIR" | wc -l | tr -d ' ')
assert_eq "17 skills have agent override injection" "17" "$AGENTS_COUNT"

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
  "visualisation/visualise"
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

echo "Test: config-read-agents.sh appears after config-read-context.sh and config-read-skill-context.sh"
AGENT_SKILLS=(
  "planning/create-plan"
  "planning/review-plan"
  "planning/stress-test-plan"
  "planning/implement-plan"
  "planning/validate-plan"
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
  EXPECTED_LINE=$((CONTEXT_LINE + 2))
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

echo "Test: No review config with pr mode -> outputs all PR-relevant defaults"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "## Review Configuration" && \
   echo "$OUTPUT" | grep -q '\- \*\*max inline comments\*\*: 10' && \
   echo "$OUTPUT" | grep -q '\- \*\*dedup proximity\*\*: 3' && \
   echo "$OUTPUT" | grep -q '\- \*\*pr request changes severity\*\*: critical' && \
   echo "$OUTPUT" | grep -q '\- \*\*min lenses\*\*: 4' && \
   echo "$OUTPUT" | grep -q '\- \*\*max lenses\*\*: 8'; then
  echo "  PASS: outputs all PR-relevant defaults"
  PASS=$((PASS + 1))
else
  echo "  FAIL: outputs all PR-relevant defaults"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: No review config with plan mode -> outputs all plan-relevant defaults"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
if echo "$OUTPUT" | grep -q "## Review Configuration" && \
   echo "$OUTPUT" | grep -q '\- \*\*plan revise severity\*\*: critical' && \
   echo "$OUTPUT" | grep -q '\- \*\*plan revise major count\*\*: 3' && \
   echo "$OUTPUT" | grep -q '\- \*\*min lenses\*\*: 4' && \
   echo "$OUTPUT" | grep -q '\- \*\*max lenses\*\*: 8'; then
  echo "  PASS: outputs all plan-relevant defaults"
  PASS=$((PASS + 1))
else
  echo "  FAIL: outputs all plan-relevant defaults"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Partial config (only some keys) -> overridden values show default annotation"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_inline_comments: 15
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q '\- \*\*max inline comments\*\*: 15 (default: 10)' && \
   echo "$OUTPUT" | grep -q '\- \*\*dedup proximity\*\*: 3$'; then
  echo "  PASS: overridden value annotated, default value plain"
  PASS=$((PASS + 1))
else
  echo "  FAIL: overridden value annotated, default value plain"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Full config -> outputs all overrides with default annotations"
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
if echo "$OUTPUT" | grep -q '\*\*max inline comments\*\*: 15 (default: 10)' && \
   echo "$OUTPUT" | grep -q '\*\*dedup proximity\*\*: 5 (default: 3)' && \
   echo "$OUTPUT" | grep -q '\*\*min lenses\*\*: 3 (default: 4)' && \
   echo "$OUTPUT" | grep -q '\*\*max lenses\*\*: 10 (default: 8)' && \
   echo "$OUTPUT" | grep -q "Core lenses" && \
   echo "$OUTPUT" | grep -q "Disabled lenses" && \
   echo "$OUTPUT" | grep -q "Verdict"; then
  echo "  PASS: outputs all overrides with annotations"
  PASS=$((PASS + 1))
else
  echo "  FAIL: outputs all overrides with annotations"
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

echo "Test: Directory without SKILL.md -> skipped (no custom lens in output)"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/empty-lens"
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "## Review Configuration" && \
   ! echo "$OUTPUT" | grep -q "custom"; then
  echo "  PASS: empty lens dir skipped, defaults still emitted"
  PASS=$((PASS + 1))
else
  echo "  FAIL: empty lens dir skipped, defaults still emitted"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

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

echo "Test: plan_revise_severity: critical (same as default) -> shown without annotation"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  plan_revise_severity: critical
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
if echo "$OUTPUT" | grep -q '\*\*plan revise severity\*\*: critical$'; then
  echo "  PASS: default severity shown without annotation"
  PASS=$((PASS + 1))
else
  echo "  FAIL: default severity shown without annotation"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

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
if echo "$OUTPUT" | grep -q "max inline comments" && \
   ! echo "$OUTPUT" | grep -q "plan revise"; then
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
if ! echo "$OUTPUT" | grep -q "max inline comments" && \
   echo "$OUTPUT" | grep -q "plan revise major count.*2"; then
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
for var_name in "max inline comments" "dedup proximity" "min lenses" "max lenses" "core_lenses" "disabled_lenses"; do
  if ! grep -q "$var_name" "$REVIEW_PR"; then
    echo "  FAIL: '$var_name' not found in review-pr SKILL.md"
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
for var_name in "min lenses" "max lenses" "core_lenses" "disabled_lenses" "plan revise severity" "plan revise major count"; do
  if ! grep -q "$var_name" "$REVIEW_PLAN"; then
    echo "  FAIL: '$var_name' not found in review-plan SKILL.md"
    PLAN_CONFIG_OK=false
    FAIL=$((FAIL + 1))
    break
  fi
done
if [ "$PLAN_CONFIG_OK" = true ]; then
  echo "  PASS: all plan config variable names appear in review-plan SKILL.md"
  PASS=$((PASS + 1))
fi

echo "Test: No config files -> always outputs Review Configuration (regression guard)"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -q "## Review Configuration"; then
  echo "  PASS: always outputs Review Configuration"
  PASS=$((PASS + 1))
else
  echo "  FAIL: always outputs Review Configuration"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo ""

# ============================================================
echo "=== config-read-review.sh per-type catalogue ==="
echo ""

echo "Test: pr mode emits all 13 code lens names and no others"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
EXPECTED_LENSES="architecture code-quality compatibility correctness database documentation performance portability safety security standards test-coverage usability"
CATALOGUE_OK=true
for lens in $EXPECTED_LENSES; do
  if ! echo "$OUTPUT" | grep -q "| $lens |"; then
    echo "  FAIL: pr mode missing lens '$lens'"
    CATALOGUE_OK=false
    FAIL=$((FAIL + 1))
    break
  fi
done
if [ "$CATALOGUE_OK" = true ]; then
  echo "  PASS: pr mode emits all 13 code lenses"
  PASS=$((PASS + 1))
fi

echo "Test: plan mode emits all 13 code lens names"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
CATALOGUE_OK=true
for lens in $EXPECTED_LENSES; do
  if ! echo "$OUTPUT" | grep -q "| $lens |"; then
    echo "  FAIL: plan mode missing lens '$lens'"
    CATALOGUE_OK=false
    FAIL=$((FAIL + 1))
    break
  fi
done
if [ "$CATALOGUE_OK" = true ]; then
  echo "  PASS: plan mode emits all 13 code lenses"
  PASS=$((PASS + 1))
fi

# Helper local to this block: extract sorted built-in lens names
# from a catalogue output. Accepts the output on stdin.
_extract_builtin_lens_names() {
  grep "| built-in |" \
    | awk -F'|' '{gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2}' \
    | sort \
    | tr '\n' ' ' \
    | sed 's/ $//'
}

echo "Test: work-item mode catalogue contains exactly 5 built-in rows"
REPO=$(setup_repo)
WORK_ITEM_OUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>/dev/null)
CATALOGUE_LINES=$(echo "$WORK_ITEM_OUT" | awk '/\| .* \| .* \| built-in \|/ {c++} END {print c+0}')
assert_eq "work-item mode emits 5 built-in lens rows" 5 "$CATALOGUE_LINES"

echo "Test: work-item mode catalogue emits the expected sorted lens set"
SORTED_LENSES=$(echo "$WORK_ITEM_OUT" | _extract_builtin_lens_names)
assert_eq "work-item mode sorted lens set" \
  "clarity completeness dependency scope testability" \
  "$SORTED_LENSES"

echo "Test: work-item-mode output is byte-identical to its committed golden fixture"
WORK_ITEM_GOLDEN="$SCRIPT_DIR/test-fixtures/config-read-review/work-item-mode-golden.txt"
assert_eq "work-item-mode output matches golden fixture" \
  "$(cat "$WORK_ITEM_GOLDEN")" \
  "$WORK_ITEM_OUT"

echo "Test: none of the five work-item lenses appear in pr or plan mode"
REPO=$(setup_repo)
PR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
PLAN_OUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
LEAKED=""
for lens in completeness testability clarity scope dependency; do
  if echo "$PR_OUT" | grep -q "| $lens |"; then
    LEAKED="$LEAKED pr:$lens"
  fi
  if echo "$PLAN_OUT" | grep -q "| $lens |"; then
    LEAKED="$LEAKED plan:$lens"
  fi
done
assert_eq "no work-item lens leaks into pr or plan catalogue" "" "$LEAKED"

echo "Test: core_lenses override emits informational note in work-item mode"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  core_lenses: [completeness, testability, clarity]
---
FIXTURE
STDERR_NOTE=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>&1 1>/dev/null)
if echo "$STDERR_NOTE" | grep -q "Note: built-in work-item lens"; then
  echo "  PASS: core_lenses override emits informational note in work-item mode"
  PASS=$((PASS + 1))
else
  echo "  FAIL: core_lenses override emits informational note in work-item mode"
  echo "    Stderr: $(printf '%q' "$STDERR_NOTE")"
  FAIL=$((FAIL + 1))
fi

echo "Test: empty core_lenses does not emit informational note in work-item mode"
REPO=$(setup_repo)
STDERR_EMPTY=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>&1 1>/dev/null)
if echo "$STDERR_EMPTY" | grep -q "Note: built-in work-item lens"; then
  echo "  FAIL: empty core_lenses emits unexpected informational note in work-item mode"
  echo "    Stderr: $(printf '%q' "$STDERR_EMPTY")"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: empty core_lenses does not emit informational note in work-item mode"
  PASS=$((PASS + 1))
fi

echo "Test: unknown mode -> exit 1 and usage contains pr|plan|work-item"
STDERR_OUTPUT=$(bash "$READ_REVIEW" "bad-mode" 2>&1 || true)
if echo "$STDERR_OUTPUT" | grep -q "pr|plan|work-item"; then
  echo "  PASS: usage string includes work-item"
  PASS=$((PASS + 1))
else
  echo "  FAIL: usage string includes work-item"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi
assert_exit_code "unknown mode exits 1" 1 bash "$READ_REVIEW" "bad-mode"

echo "Test: custom lens with applies_to: [plan] appears only in plan mode"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/plan-only-lens"
cat > "$REPO/.claude/accelerator/lenses/plan-only-lens/SKILL.md" << 'FIXTURE'
---
name: plan-only
description: Appears in plan only
applies_to: [plan]
---
FIXTURE
PR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
PLAN_OUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
WORK_ITEM_OUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>/dev/null || true)
if ! echo "$PR_OUT" | grep -q "| plan-only |" && \
   echo "$PLAN_OUT" | grep -q "| plan-only |" && \
   ! echo "$WORK_ITEM_OUT" | grep -q "| plan-only |"; then
  echo "  PASS: applies_to: [plan] restricts lens to plan mode only"
  PASS=$((PASS + 1))
else
  echo "  FAIL: applies_to: [plan] restricts lens to plan mode only"
  echo "    PR has it: $(echo "$PR_OUT" | grep -c "| plan-only |")"
  echo "    Plan has it: $(echo "$PLAN_OUT" | grep -c "| plan-only |")"
  echo "    Work-item has it: $(echo "$WORK_ITEM_OUT" | grep -c "| plan-only |")"
  FAIL=$((FAIL + 1))
fi

echo "Test: custom lens without applies_to appears in all three modes"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/all-modes-lens"
cat > "$REPO/.claude/accelerator/lenses/all-modes-lens/SKILL.md" << 'FIXTURE'
---
name: all-modes
description: Appears everywhere
---
FIXTURE
PR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
PLAN_OUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
WORK_ITEM_OUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>/dev/null || true)
if echo "$PR_OUT" | grep -q "| all-modes |" && \
   echo "$PLAN_OUT" | grep -q "| all-modes |" && \
   echo "$WORK_ITEM_OUT" | grep -q "| all-modes |"; then
  echo "  PASS: no applies_to means all modes"
  PASS=$((PASS + 1))
else
  echo "  FAIL: no applies_to means all modes"
  echo "    PR: $(echo "$PR_OUT" | grep "| all-modes |")"
  echo "    Plan: $(echo "$PLAN_OUT" | grep "| all-modes |")"
  echo "    Work-item: $(echo "$WORK_ITEM_OUT" | grep "| all-modes |")"
  FAIL=$((FAIL + 1))
fi

echo "Test: custom lens with applies_to: [work-item, plan] appears in work-item+plan but not pr"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/work-item-plan-lens"
cat > "$REPO/.claude/accelerator/lenses/work-item-plan-lens/SKILL.md" << 'FIXTURE'
---
name: work-item-plan
description: Work-item and plan modes only
applies_to: [work-item, plan]
---
FIXTURE
PR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
PLAN_OUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
WORK_ITEM_OUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>/dev/null || true)
if ! echo "$PR_OUT" | grep -q "| work-item-plan |" && \
   echo "$PLAN_OUT" | grep -q "| work-item-plan |" && \
   echo "$WORK_ITEM_OUT" | grep -q "| work-item-plan |"; then
  echo "  PASS: applies_to: [work-item, plan] includes work-item and plan but not pr"
  PASS=$((PASS + 1))
else
  echo "  FAIL: applies_to: [work-item, plan] includes work-item and plan but not pr"
  FAIL=$((FAIL + 1))
fi

echo "Test: core_lenses: [architecture] in plan mode produces no warning (valid cross-mode lens)"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  core_lenses: [architecture]
---
FIXTURE
STDERR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" plan 2>&1 1>/dev/null)
if ! echo "$STDERR_OUT" | grep -q "unrecognised"; then
  echo "  PASS: architecture in core_lenses produces no warning in plan mode"
  PASS=$((PASS + 1))
else
  echo "  FAIL: architecture in core_lenses produces no warning in plan mode"
  echo "    Stderr: $(printf '%q' "$STDERR_OUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: cross-mode filter - work-item-only custom lens in core_lenses shows Filtered info in pr mode"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/work-item-custom-lens"
cat > "$REPO/.claude/accelerator/lenses/work-item-custom-lens/SKILL.md" << 'FIXTURE'
---
name: work-item-custom
description: Work-item only custom lens
applies_to: [work-item]
---
FIXTURE
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  core_lenses: [architecture, work-item-custom]
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
if echo "$OUTPUT" | grep -qi "Filtered core lenses"; then
  echo "  PASS: cross-mode filtered core lenses shown in Review Configuration"
  PASS=$((PASS + 1))
else
  echo "  FAIL: cross-mode filtered core lenses shown in Review Configuration"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: unknown lens xyz in core_lenses still produces unrecognised warning"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  core_lenses: [architecture, xyz]
---
FIXTURE
STDERR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" plan 2>&1 1>/dev/null)
if echo "$STDERR_OUT" | grep -q "unrecognised.*xyz"; then
  echo "  PASS: unrecognised lens xyz still warns"
  PASS=$((PASS + 1))
else
  echo "  FAIL: unrecognised lens xyz still warns"
  echo "    Stderr: $(printf '%q' "$STDERR_OUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: applies_to: [prr] -> unrecognised mode warning, lens absent from all catalogues"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/bad-mode-lens"
cat > "$REPO/.claude/accelerator/lenses/bad-mode-lens/SKILL.md" << 'FIXTURE'
---
name: bad-mode
description: Has unrecognised mode
applies_to: [prr]
---
FIXTURE
STDERR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>&1 1>/dev/null)
PR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>/dev/null)
PLAN_OUT=$(cd "$REPO" && bash "$READ_REVIEW" plan 2>/dev/null)
if echo "$STDERR_OUT" | grep -qi "unrecognised mode.*prr\|prr.*unrecognised" && \
   ! echo "$PR_OUT" | grep -q "| bad-mode |" && \
   ! echo "$PLAN_OUT" | grep -q "| bad-mode |"; then
  echo "  PASS: unrecognised mode warns and excludes lens from all catalogues"
  PASS=$((PASS + 1))
else
  echo "  FAIL: unrecognised mode warns and excludes lens from all catalogues"
  echo "    Stderr: $(printf '%q' "$STDERR_OUT")"
  echo "    PR has bad-mode: $(echo "$PR_OUT" | grep -c "| bad-mode |")"
  FAIL=$((FAIL + 1))
fi

echo "Test: applies_to: [] -> empty applies_to warning, lens absent from all catalogues"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/empty-applies-lens"
cat > "$REPO/.claude/accelerator/lenses/empty-applies-lens/SKILL.md" << 'FIXTURE'
---
name: empty-applies
description: Has empty applies_to
applies_to: []
---
FIXTURE
STDERR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>&1 1>/dev/null)
PR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>/dev/null)
if echo "$STDERR_OUT" | grep -qi "empty applies_to" && \
   ! echo "$PR_OUT" | grep -q "| empty-applies |"; then
  echo "  PASS: empty applies_to warns and excludes lens"
  PASS=$((PASS + 1))
else
  echo "  FAIL: empty applies_to warns and excludes lens"
  echo "    Stderr: $(printf '%q' "$STDERR_OUT")"
  echo "    PR output: $(printf '%q' "$PR_OUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: applies_to: pr (scalar) -> parsed as [pr], appears in pr only"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/scalar-applies-lens"
cat > "$REPO/.claude/accelerator/lenses/scalar-applies-lens/SKILL.md" << 'FIXTURE'
---
name: scalar-applies
description: Scalar applies_to
applies_to: pr
---
FIXTURE
PR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
PLAN_OUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
if echo "$PR_OUT" | grep -q "| scalar-applies |" && \
   ! echo "$PLAN_OUT" | grep -q "| scalar-applies |"; then
  echo "  PASS: scalar applies_to treated as [pr]"
  PASS=$((PASS + 1))
else
  echo "  FAIL: scalar applies_to treated as [pr]"
  echo "    PR has it: $(echo "$PR_OUT" | grep -c "| scalar-applies |")"
  echo "    Plan has it: $(echo "$PLAN_OUT" | grep -c "| scalar-applies |")"
  FAIL=$((FAIL + 1))
fi

echo "Test: applies_to: [pr, pr] (duplicate) -> deduplicated, appears in pr exactly once"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/lenses/dedup-applies-lens"
cat > "$REPO/.claude/accelerator/lenses/dedup-applies-lens/SKILL.md" << 'FIXTURE'
---
name: dedup-applies
description: Duplicate applies_to entries
applies_to: [pr, pr]
---
FIXTURE
PR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
COUNT=$(echo "$PR_OUT" | grep -c "| dedup-applies |" || true)
if [ "$COUNT" -eq 1 ]; then
  echo "  PASS: duplicate applies_to deduplicated"
  PASS=$((PASS + 1))
else
  echo "  FAIL: duplicate applies_to deduplicated (count=$COUNT)"
  FAIL=$((FAIL + 1))
fi

echo "Test: pr mode still emits PR verdict override when pr_request_changes_severity set to major"
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
  echo "  PASS: pr mode verdict override unchanged after refactor"
  PASS=$((PASS + 1))
else
  echo "  FAIL: pr mode verdict override unchanged after refactor"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: plan mode still emits plan verdict override when plan_revise_severity set to major"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  plan_revise_severity: major
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
if echo "$OUTPUT" | grep -q "Verdict.*REVISE.*major"; then
  echo "  PASS: plan mode verdict override unchanged after refactor"
  PASS=$((PASS + 1))
else
  echo "  FAIL: plan mode verdict override unchanged after refactor"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: work-item mode emits work-item revise severity and count with defaults"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>/dev/null || true)
if echo "$OUTPUT" | grep -q '\*\*work-item revise severity\*\*: critical$' && \
   echo "$OUTPUT" | grep -q '\*\*work-item revise major count\*\*: 2$'; then
  echo "  PASS: work-item mode emits verdict defaults without annotation"
  PASS=$((PASS + 1))
else
  echo "  FAIL: work-item mode emits verdict defaults without annotation"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: work_item_revise_severity: major -> annotated with default"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  work_item_revise_severity: major
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>/dev/null || true)
if echo "$OUTPUT" | grep -q '\*\*work-item revise severity\*\*: major (default: critical)'; then
  echo "  PASS: work-item severity override annotated"
  PASS=$((PASS + 1))
else
  echo "  FAIL: work-item severity override annotated"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: work_item_revise_major_count: 5 -> annotated with default"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  work_item_revise_major_count: 5
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>/dev/null || true)
if echo "$OUTPUT" | grep -q '\*\*work-item revise major count\*\*: 5 (default: 2)'; then
  echo "  PASS: work-item major count override annotated"
  PASS=$((PASS + 1))
else
  echo "  FAIL: work-item major count override annotated"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: work_item_revise_major_count: 0 -> warning, falls back to 2"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  work_item_revise_major_count: 0
---
FIXTURE
STDERR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>&1 1>/dev/null || true)
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>/dev/null || true)
if echo "$STDERR_OUT" | grep -q "Warning.*work_item_revise_major_count" && \
   echo "$OUTPUT" | grep -q '\*\*work-item revise major count\*\*: 2$'; then
  echo "  PASS: invalid major count warns and falls back to default"
  PASS=$((PASS + 1))
else
  echo "  FAIL: invalid major count warns and falls back to default"
  echo "    Stderr: $(printf '%q' "$STDERR_OUT")"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: work_item_revise_severity: sometimes -> warning, falls back to critical"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  work_item_revise_severity: sometimes
---
FIXTURE
STDERR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>&1 1>/dev/null || true)
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>/dev/null || true)
if echo "$STDERR_OUT" | grep -q "Warning.*work_item_revise_severity" && \
   echo "$OUTPUT" | grep -q '\*\*work-item revise severity\*\*: critical$'; then
  echo "  PASS: invalid severity warns and falls back to default"
  PASS=$((PASS + 1))
else
  echo "  FAIL: invalid severity warns and falls back to default"
  echo "    Stderr: $(printf '%q' "$STDERR_OUT")"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: work_item_revise_severity: none -> severity-based REVISE disabled verdict line"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  work_item_revise_severity: none
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>/dev/null || true)
if echo "$OUTPUT" | grep -q "severity-based REVISE disabled"; then
  echo "  PASS: work-item severity none produces disabled verdict line"
  PASS=$((PASS + 1))
else
  echo "  FAIL: work-item severity none produces disabled verdict line"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo "Test: work-item mode catalogue contains completeness, not in pr or plan"
REPO=$(setup_repo)
PR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
PLAN_OUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
WORK_ITEM_OUT=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>/dev/null || true)
if echo "$WORK_ITEM_OUT" | grep -q "| completeness |" && \
   ! echo "$PR_OUT" | grep -q "| completeness |" && \
   ! echo "$PLAN_OUT" | grep -q "| completeness |"; then
  echo "  PASS: completeness in work-item only"
  PASS=$((PASS + 1))
else
  echo "  FAIL: completeness in work-item only"
  echo "    Work-item has it: $(echo "$WORK_ITEM_OUT" | grep -c "| completeness |" || true)"
  echo "    PR has it: $(echo "$PR_OUT" | grep -c "| completeness |" || true)"
  echo "    Plan has it: $(echo "$PLAN_OUT" | grep -c "| completeness |" || true)"
  FAIL=$((FAIL + 1))
fi

echo "Test: cross-mode core_lenses: [architecture, completeness] produces no warning in pr, plan, or work-item mode"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  core_lenses: [architecture, completeness]
---
FIXTURE
PR_STDERR=$(cd "$REPO" && bash "$READ_REVIEW" pr 2>&1 1>/dev/null)
PLAN_STDERR=$(cd "$REPO" && bash "$READ_REVIEW" plan 2>&1 1>/dev/null)
WORK_ITEM_STDERR=$(cd "$REPO" && bash "$READ_REVIEW" work-item 2>&1 1>/dev/null || true)
if ! echo "$PR_STDERR" | grep -q "unrecognised" && \
   ! echo "$PLAN_STDERR" | grep -q "unrecognised" && \
   ! echo "$WORK_ITEM_STDERR" | grep -q "unrecognised"; then
  echo "  PASS: cross-mode core_lenses produces no unrecognised warning"
  PASS=$((PASS + 1))
else
  echo "  FAIL: cross-mode core_lenses produces no unrecognised warning"
  echo "    PR stderr: $(printf '%q' "$PR_STDERR")"
  echo "    Plan stderr: $(printf '%q' "$PLAN_STDERR")"
  echo "    Work-item stderr: $(printf '%q' "$WORK_ITEM_STDERR")"
  FAIL=$((FAIL + 1))
fi

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

echo "Test: No config overrides -> config-dump shows prefixed agent defaults"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
review:
  max_inline_comments: 15
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
if echo "$OUTPUT" | grep -q 'agents\.reviewer.*accelerator:reviewer.*default' && \
   echo "$OUTPUT" | grep -q 'agents\.codebase-locator.*accelerator:codebase-locator.*default'; then
  echo "  PASS: config-dump shows prefixed agent defaults"
  PASS=$((PASS + 1))
else
  echo "  FAIL: config-dump shows prefixed agent defaults"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
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

echo "Test: config-read-review.sh appears after config-read-agents.sh (with fallback) in review-pr"
AGENTS_LINE=$(grep -n 'config-read-agents.sh' "$SKILLS_DIR/github/review-pr/SKILL.md" | head -1 | cut -d: -f1)
REVIEW_LINE=$(grep -n 'config-read-review.sh' "$SKILLS_DIR/github/review-pr/SKILL.md" | head -1 | cut -d: -f1)
if [ "$REVIEW_LINE" -gt "$AGENTS_LINE" ]; then
  echo "  PASS: review config after agents in review-pr (agents:$AGENTS_LINE, review:$REVIEW_LINE)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: review config should be after agents in review-pr (agents:$AGENTS_LINE, review:$REVIEW_LINE)"
  FAIL=$((FAIL + 1))
fi

echo "Test: config-read-review.sh appears after config-read-agents.sh (with fallback) in review-plan"
AGENTS_LINE=$(grep -n 'config-read-agents.sh' "$SKILLS_DIR/planning/review-plan/SKILL.md" | head -1 | cut -d: -f1)
REVIEW_LINE=$(grep -n 'config-read-review.sh' "$SKILLS_DIR/planning/review-plan/SKILL.md" | head -1 | cut -d: -f1)
if [ "$REVIEW_LINE" -gt "$AGENTS_LINE" ]; then
  echo "  PASS: review config after agents in review-plan (agents:$AGENTS_LINE, review:$REVIEW_LINE)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: review config should be after agents in review-plan (agents:$AGENTS_LINE, review:$REVIEW_LINE)"
  FAIL=$((FAIL + 1))
fi

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

echo "Test: paths.work configured"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  work: docs/work
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" "work" "meta/work")
assert_eq "outputs configured path" "docs/work" "$OUTPUT"

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

echo "Test: paths.review_work configured"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  review_work: docs/reviews/work
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" "review_work" "meta/reviews/work")
assert_eq "outputs configured path" "docs/reviews/work" "$OUTPUT"

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
LIST_TEMPLATE="$SCRIPT_DIR/config-list-template.sh"
SHOW_TEMPLATE="$SCRIPT_DIR/config-show-template.sh"
EJECT_TEMPLATE="$SCRIPT_DIR/config-eject-template.sh"
DIFF_TEMPLATE="$SCRIPT_DIR/config-diff-template.sh"
RESET_TEMPLATE="$SCRIPT_DIR/config-reset-template.sh"
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
if echo "$STDERR_OUTPUT" | grep -q "pr-description" && \
   echo "$STDERR_OUTPUT" | grep -q "plan" && \
   echo "$STDERR_OUTPUT" | grep -q "research" && \
   echo "$STDERR_OUTPUT" | grep -q "adr" && \
   echo "$STDERR_OUTPUT" | grep -q "validation" && \
   echo "$STDERR_OUTPUT" | grep -q "work-item"; then
  echo "  PASS: error lists available templates (including pr-description and work-item)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: error lists available templates (including pr-description and work-item)"
  echo "    Actual stderr: $STDERR_OUTPUT"
  FAIL=$((FAIL + 1))
fi
assert_exit_code "exits 1 for unknown template" 1 bash "$READ_TEMPLATE" "nonexistent"

echo "Test: pr-description template -> outputs plugin default"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_TEMPLATE" "pr-description")
if echo "$OUTPUT" | grep -q "PR Title" && echo "$OUTPUT" | grep -q "Summary"; then
  echo "  PASS: pr-description template content output"
  PASS=$((PASS + 1))
else
  echo "  FAIL: pr-description template content output"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

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

echo "Test: describe-pr uses config-read-path.sh for prs and config-read-template.sh for pr-description"
DESCRIBE_SKILL="$SKILLS_DIR/github/describe-pr/SKILL.md"
if grep -q 'config-read-path.sh prs' "$DESCRIBE_SKILL" && \
   grep -q 'config-read-template.sh pr-description' "$DESCRIBE_SKILL"; then
  echo "  PASS: describe-pr has prs path and pr-description template injections"
  PASS=$((PASS + 1))
else
  echo "  FAIL: describe-pr has prs path and pr-description template injections"
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

echo "Test: respond-to-pr uses config-read-path.sh"
if grep -q 'config-read-path.sh review_prs' "$SKILLS_DIR/github/respond-to-pr/SKILL.md"; then
  echo "  PASS: respond-to-pr has review_prs path injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: respond-to-pr has review_prs path injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: implement-plan uses config-read-path.sh"
if grep -q 'config-read-path.sh plans' "$SKILLS_DIR/planning/implement-plan/SKILL.md"; then
  echo "  PASS: implement-plan has plans path injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: implement-plan has plans path injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: validate-plan has plans path injection"
if grep -q 'config-read-path.sh plans' "$SKILLS_DIR/planning/validate-plan/SKILL.md"; then
  echo "  PASS: validate-plan has plans path injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: validate-plan has plans path injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: review-plan has plans path injection"
if grep -q 'config-read-path.sh plans' "$SKILLS_DIR/planning/review-plan/SKILL.md"; then
  echo "  PASS: review-plan has plans path injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: review-plan has plans path injection"
  FAIL=$((FAIL + 1))
fi

echo ""

# ============================================================
echo "=== config-read-skill-context.sh ==="
echo ""

echo "Test: No skill name argument -> exits with error"
assert_exit_code "exits 1 with no argument" 1 bash "$READ_SKILL_CONTEXT"

echo "Test: No customisation directory -> no output"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_CONTEXT" create-plan)
assert_empty "no output without directory" "$OUTPUT"

echo "Test: Skill directory exists but no context.md -> no output"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_CONTEXT" create-plan)
assert_empty "no output without context.md" "$OUTPUT"

echo "Test: context.md exists with content -> outputs section with header"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
printf 'Some context content.\n' > "$REPO/.claude/accelerator/skills/create-plan/context.md"
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_CONTEXT" create-plan)
EXPECTED="## Skill-Specific Context

The following context is specific to the create-plan skill. Apply this
context in addition to any project-wide context above.

Some context content."
assert_eq "outputs full section with header" "$EXPECTED" "$OUTPUT"

echo "Test: context.md exists but is empty -> no output"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
touch "$REPO/.claude/accelerator/skills/create-plan/context.md"
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_CONTEXT" create-plan)
assert_empty "no output for empty file" "$OUTPUT"

echo "Test: context.md exists with only whitespace/blank lines -> no output"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
printf '   \n\n  \n' > "$REPO/.claude/accelerator/skills/create-plan/context.md"
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_CONTEXT" create-plan)
assert_empty "no output for whitespace-only file" "$OUTPUT"

echo "Test: context.md with leading/trailing blank lines -> trimmed output"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/review-pr"
printf '\n\nTrimmed content.\n\n\n' > "$REPO/.claude/accelerator/skills/review-pr/context.md"
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_CONTEXT" review-pr)
assert_contains "content is trimmed" "Trimmed content." "$OUTPUT"
# Should not start with blank lines in content section
CONTENT_AFTER_HEADER=$(echo "$OUTPUT" | tail -1)
assert_eq "last line is trimmed content" "Trimmed content." "$CONTENT_AFTER_HEADER"

echo "Test: Output includes skill name in header text"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/review-pr"
printf 'Content.\n' > "$REPO/.claude/accelerator/skills/review-pr/context.md"
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_CONTEXT" review-pr)
assert_contains "header mentions skill name" "review-pr skill" "$OUTPUT"

echo "Test: Multiple skills with context -> each reads only its own"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
mkdir -p "$REPO/.claude/accelerator/skills/review-pr"
printf 'Plan context.\n' > "$REPO/.claude/accelerator/skills/create-plan/context.md"
printf 'PR context.\n' > "$REPO/.claude/accelerator/skills/review-pr/context.md"
OUTPUT_PLAN=$(cd "$REPO" && bash "$READ_SKILL_CONTEXT" create-plan)
OUTPUT_PR=$(cd "$REPO" && bash "$READ_SKILL_CONTEXT" review-pr)
assert_contains "plan reads its own context" "Plan context." "$OUTPUT_PLAN"
assert_contains "pr reads its own context" "PR context." "$OUTPUT_PR"
# Verify isolation
if printf '%s' "$OUTPUT_PLAN" | grep -qF "PR context."; then
  echo "  FAIL: plan output should not contain pr context"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: plan output does not contain pr context"
  PASS=$((PASS + 1))
fi

echo ""

# ============================================================
echo "=== config-read-skill-instructions.sh ==="
echo ""

echo "Test: No skill name argument -> exits with error"
assert_exit_code "exits 1 with no argument" 1 bash "$READ_SKILL_INSTRUCTIONS"

echo "Test: No customisation directory -> no output"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_INSTRUCTIONS" review-pr)
assert_empty "no output without directory" "$OUTPUT"

echo "Test: Skill directory exists but no instructions.md -> no output"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/review-pr"
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_INSTRUCTIONS" review-pr)
assert_empty "no output without instructions.md" "$OUTPUT"

echo "Test: instructions.md exists with content -> outputs section with header"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/review-pr"
printf 'Always check for tests.\n' > "$REPO/.claude/accelerator/skills/review-pr/instructions.md"
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_INSTRUCTIONS" review-pr)
EXPECTED="## Additional Instructions

The following additional instructions have been provided for the
review-pr skill. Follow these instructions in addition to all
instructions above.

Always check for tests."
assert_eq "outputs full section with header" "$EXPECTED" "$OUTPUT"

echo "Test: instructions.md exists but is empty -> no output"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/review-pr"
touch "$REPO/.claude/accelerator/skills/review-pr/instructions.md"
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_INSTRUCTIONS" review-pr)
assert_empty "no output for empty file" "$OUTPUT"

echo "Test: instructions.md exists with only whitespace/blank lines -> no output"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/review-pr"
printf '   \n\n  \n' > "$REPO/.claude/accelerator/skills/review-pr/instructions.md"
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_INSTRUCTIONS" review-pr)
assert_empty "no output for whitespace-only file" "$OUTPUT"

echo "Test: instructions.md with leading/trailing blank lines -> trimmed output"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/commit"
printf '\n\nTrimmed instructions.\n\n\n' > "$REPO/.claude/accelerator/skills/commit/instructions.md"
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_INSTRUCTIONS" commit)
assert_contains "content is trimmed" "Trimmed instructions." "$OUTPUT"
CONTENT_AFTER_HEADER=$(echo "$OUTPUT" | tail -1)
assert_eq "last line is trimmed content" "Trimmed instructions." "$CONTENT_AFTER_HEADER"

echo "Test: Output includes skill name in header text"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/commit"
printf 'Instructions.\n' > "$REPO/.claude/accelerator/skills/commit/instructions.md"
OUTPUT=$(cd "$REPO" && bash "$READ_SKILL_INSTRUCTIONS" commit)
assert_contains "header mentions skill name" "commit skill" "$OUTPUT"

echo "Test: Multiple skills with instructions -> each reads only its own"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude/accelerator/skills/commit"
mkdir -p "$REPO/.claude/accelerator/skills/review-pr"
printf 'Commit instructions.\n' > "$REPO/.claude/accelerator/skills/commit/instructions.md"
printf 'Review instructions.\n' > "$REPO/.claude/accelerator/skills/review-pr/instructions.md"
OUTPUT_COMMIT=$(cd "$REPO" && bash "$READ_SKILL_INSTRUCTIONS" commit)
OUTPUT_PR=$(cd "$REPO" && bash "$READ_SKILL_INSTRUCTIONS" review-pr)
assert_contains "commit reads its own instructions" "Commit instructions." "$OUTPUT_COMMIT"
assert_contains "pr reads its own instructions" "Review instructions." "$OUTPUT_PR"
if printf '%s' "$OUTPUT_COMMIT" | grep -qF "Review instructions."; then
  echo "  FAIL: commit output should not contain pr instructions"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: commit output does not contain pr instructions"
  PASS=$((PASS + 1))
fi

echo ""

# ============================================================
echo "=== config-summary.sh (per-skill customisations) ==="
echo ""

echo "Test: No per-skill directories -> no per-skill line in summary"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY")
if echo "$OUTPUT" | grep -q "Per-skill"; then
  echo "  FAIL: should not mention per-skill without directories"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: no per-skill line in summary"
  PASS=$((PASS + 1))
fi

echo "Test: One skill with context.md -> summary lists skill with (context)"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
printf 'Some context.\n' > "$REPO/.claude/accelerator/skills/create-plan/context.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY" 2>/dev/null)
assert_contains "lists skill with context" "create-plan (context)" "$OUTPUT"

echo "Test: One skill with instructions.md -> summary lists skill with (instructions)"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
mkdir -p "$REPO/.claude/accelerator/skills/review-pr"
printf 'Some instructions.\n' > "$REPO/.claude/accelerator/skills/review-pr/instructions.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY" 2>/dev/null)
assert_contains "lists skill with instructions" "review-pr (instructions)" "$OUTPUT"

echo "Test: One skill with both files -> summary lists skill with (context + instructions)"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
printf 'Context.\n' > "$REPO/.claude/accelerator/skills/create-plan/context.md"
printf 'Instructions.\n' > "$REPO/.claude/accelerator/skills/create-plan/instructions.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY" 2>/dev/null)
assert_contains "lists skill with both" "create-plan (context + instructions)" "$OUTPUT"

echo "Test: Multiple skills with customisations -> all listed"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
mkdir -p "$REPO/.claude/accelerator/skills/review-pr"
printf 'Context.\n' > "$REPO/.claude/accelerator/skills/create-plan/context.md"
printf 'Instructions.\n' > "$REPO/.claude/accelerator/skills/review-pr/instructions.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY" 2>/dev/null)
assert_contains "lists create-plan" "create-plan (context)" "$OUTPUT"
assert_contains "lists review-pr" "review-pr (instructions)" "$OUTPUT"

echo "Test: Skill directory with no recognised files -> not listed"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
printf 'Some other file.\n' > "$REPO/.claude/accelerator/skills/create-plan/notes.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY" 2>/dev/null)
if echo "$OUTPUT" | grep -q "Per-skill"; then
  echo "  FAIL: should not list skill with no recognised files"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: skill with no recognised files not listed"
  PASS=$((PASS + 1))
fi

echo "Test: Empty context.md and instructions.md -> skill not listed"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
touch "$REPO/.claude/accelerator/skills/create-plan/context.md"
touch "$REPO/.claude/accelerator/skills/create-plan/instructions.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY" 2>/dev/null)
if echo "$OUTPUT" | grep -q "Per-skill"; then
  echo "  FAIL: should not list skill with empty files"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: skill with empty files not listed"
  PASS=$((PASS + 1))
fi

echo "Test: Whitespace-only context.md -> skill not listed (matches reader behaviour)"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
printf '   \n\n  \n' > "$REPO/.claude/accelerator/skills/create-plan/context.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY" 2>/dev/null)
if echo "$OUTPUT" | grep -q "Per-skill"; then
  echo "  FAIL: should not list skill with whitespace-only files"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: skill with whitespace-only files not listed"
  PASS=$((PASS + 1))
fi

echo "Test: Unrecognised skill directory name -> stderr warning emitted"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
mkdir -p "$REPO/.claude/accelerator/skills/nonexistent-skill"
printf 'Content.\n' > "$REPO/.claude/accelerator/skills/nonexistent-skill/context.md"
STDERR_OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY" 2>&1 1>/dev/null)
assert_contains "warns about unrecognised skill" "does not match any known skill name" "$STDERR_OUTPUT"

echo "Test: Known skill directory name -> no stderr warning"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
printf 'Content.\n' > "$REPO/.claude/accelerator/skills/create-plan/context.md"
STDERR_OUTPUT=$(cd "$REPO" && bash "$CONFIG_SUMMARY" 2>&1 1>/dev/null)
if [ -z "$STDERR_OUTPUT" ]; then
  echo "  PASS: no stderr warning for known skill name"
  PASS=$((PASS + 1))
else
  echo "  FAIL: unexpected stderr for known skill name"
  echo "    Stderr: $(printf '%q' "$STDERR_OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo ""

# ============================================================
echo "=== Preprocessor placement (per-skill) ==="
echo ""

echo "Test: config-read-skill-context.sh appears in exactly 21 skills"
SKILL_CONTEXT_COUNT=$(grep -r --exclude-dir=node_modules 'config-read-skill-context.sh' "$SKILLS_DIR" | wc -l | tr -d ' ')
assert_eq "21 skills have skill-context injection" "21" "$SKILL_CONTEXT_COUNT"

echo "Test: config-read-skill-instructions.sh appears in exactly 21 skills"
SKILL_INSTRUCTIONS_COUNT=$(grep -r --exclude-dir=node_modules 'config-read-skill-instructions.sh' "$SKILLS_DIR" | wc -l | tr -d ' ')
assert_eq "21 skills have skill-instructions injection" "21" "$SKILL_INSTRUCTIONS_COUNT"

echo "Test: config-read-skill-context.sh appears immediately after config-read-context.sh in each skill"
ALL_SKILLS=(
  "planning/create-plan"
  "planning/review-plan"
  "planning/stress-test-plan"
  "planning/implement-plan"
  "planning/validate-plan"
  "research/research-codebase"
  "github/review-pr"
  "github/describe-pr"
  "github/respond-to-pr"
  "decisions/create-adr"
  "decisions/extract-adrs"
  "decisions/review-adr"
  "vcs/commit"
  "visualisation/visualise"
)
SKILL_CTX_PLACEMENT_OK=true
for skill in "${ALL_SKILLS[@]}"; do
  SKILL_FILE="$SKILLS_DIR/$skill/SKILL.md"
  CONTEXT_LINE=$(grep -n 'config-read-context\.sh[^-]' "$SKILL_FILE" | head -1 | cut -d: -f1)
  SKILL_CTX_LINE=$(grep -n 'config-read-skill-context.sh' "$SKILL_FILE" | head -1 | cut -d: -f1)
  EXPECTED_LINE=$((CONTEXT_LINE + 1))
  if [ "$SKILL_CTX_LINE" -ne "$EXPECTED_LINE" ]; then
    echo "  FAIL: $skill - context at line $CONTEXT_LINE, skill-context at line $SKILL_CTX_LINE (expected $EXPECTED_LINE)"
    SKILL_CTX_PLACEMENT_OK=false
    FAIL=$((FAIL + 1))
    break
  fi
done
if [ "$SKILL_CTX_PLACEMENT_OK" = true ]; then
  echo "  PASS: all skill-context injections immediately after context injection"
  PASS=$((PASS + 1))
fi

echo "Test: config-read-skill-instructions.sh is the last preprocessor line in each skill"
SKILL_INSTR_PLACEMENT_OK=true
for skill in "${ALL_SKILLS[@]}"; do
  SKILL_FILE="$SKILLS_DIR/$skill/SKILL.md"
  LAST_PREPROCESSOR_LINE=$(grep -n '^!`' "$SKILL_FILE" | tail -1 | cut -d: -f1)
  SKILL_INSTR_LINE=$(grep -n 'config-read-skill-instructions.sh' "$SKILL_FILE" | head -1 | cut -d: -f1)
  if [ "$SKILL_INSTR_LINE" -ne "$LAST_PREPROCESSOR_LINE" ]; then
    echo "  FAIL: $skill - skill-instructions at line $SKILL_INSTR_LINE, last preprocessor at line $LAST_PREPROCESSOR_LINE"
    SKILL_INSTR_PLACEMENT_OK=false
    FAIL=$((FAIL + 1))
    break
  fi
done
if [ "$SKILL_INSTR_PLACEMENT_OK" = true ]; then
  echo "  PASS: all skill-instructions injections are last preprocessor line"
  PASS=$((PASS + 1))
fi

echo "Test: Skill name argument in each preprocessor call matches the skill's frontmatter name field"
SKILL_NAME_MATCH_OK=true
for skill in "${ALL_SKILLS[@]}"; do
  SKILL_FILE="$SKILLS_DIR/$skill/SKILL.md"
  FM_NAME=$(awk '/^name:/{print $2; exit}' "$SKILL_FILE")
  CTX_ARG=$(grep 'config-read-skill-context.sh' "$SKILL_FILE" | sed 's/.*config-read-skill-context.sh //' | sed 's/`$//')
  INSTR_ARG=$(grep 'config-read-skill-instructions.sh' "$SKILL_FILE" | sed 's/.*config-read-skill-instructions.sh //' | sed 's/`$//')
  if [ "$FM_NAME" != "$CTX_ARG" ] || [ "$FM_NAME" != "$INSTR_ARG" ]; then
    echo "  FAIL: $skill - frontmatter name=$FM_NAME, context arg=$CTX_ARG, instructions arg=$INSTR_ARG"
    SKILL_NAME_MATCH_OK=false
    FAIL=$((FAIL + 1))
    break
  fi
done
if [ "$SKILL_NAME_MATCH_OK" = true ]; then
  echo "  PASS: all skill name arguments match frontmatter name fields"
  PASS=$((PASS + 1))
fi

echo "Test: configure skill does NOT contain per-skill preprocessor lines"
CONFIGURE_FILE="$SKILLS_DIR/config/configure/SKILL.md"
if grep -q 'config-read-skill-context.sh\|config-read-skill-instructions.sh' "$CONFIGURE_FILE"; then
  echo "  FAIL: configure skill should not have per-skill preprocessor lines"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: configure skill correctly excluded"
  PASS=$((PASS + 1))
fi

echo ""

# ============================================================
echo "=== config-detect.sh (per-skill customisations) ==="
echo ""

echo "Test: Per-skill customisations appear in hook additionalContext JSON"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
mkdir -p "$REPO/.claude/accelerator/skills/create-plan"
printf 'Context content.\n' > "$REPO/.claude/accelerator/skills/create-plan/context.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DETECT" 2>/dev/null)
ADDITIONAL_CONTEXT=$(echo "$OUTPUT" | jq -r '.hookSpecificOutput.additionalContext' 2>/dev/null || true)
assert_contains "per-skill info in additionalContext" "Per-skill customisations" "$ADDITIONAL_CONTEXT"

echo "Test: Unrecognised skill name warning appears in stderr (not in JSON)"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\nkey: value\n---\n' > "$REPO/.claude/accelerator.md"
mkdir -p "$REPO/.claude/accelerator/skills/nonexistent-skill"
printf 'Content.\n' > "$REPO/.claude/accelerator/skills/nonexistent-skill/context.md"
STDERR_OUTPUT=$(cd "$REPO" && bash "$CONFIG_DETECT" 2>&1 1>/dev/null)
STDOUT_OUTPUT=$(cd "$REPO" && bash "$CONFIG_DETECT" 2>/dev/null)
assert_contains "warning in stderr" "does not match any known skill name" "$STDERR_OUTPUT"
# Verify warning is not in the JSON output
if echo "$STDOUT_OUTPUT" | grep -qF "does not match"; then
  echo "  FAIL: warning should not appear in JSON stdout"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: warning not in JSON stdout"
  PASS=$((PASS + 1))
fi

echo ""

# ============================================================
echo "=== config_enumerate_templates ==="
echo ""

echo "Test: Lists all template keys from plugin templates directory"
OUTPUT=$(config_enumerate_templates "$PLUGIN_ROOT")
assert_contains "contains plan" "plan" "$OUTPUT"
assert_contains "contains research" "research" "$OUTPUT"
assert_contains "contains adr" "adr" "$OUTPUT"
assert_contains "contains validation" "validation" "$OUTPUT"
assert_contains "contains pr-description" "pr-description" "$OUTPUT"
assert_contains "contains work-item" "work-item" "$OUTPUT"
LINE_COUNT=$(echo "$OUTPUT" | wc -l | tr -d ' ')
assert_eq "outputs 6 keys" "6" "$LINE_COUNT"

echo "Test: Returns nothing if templates directory is empty"
EMPTY_ROOT=$(mktemp -d "$TMPDIR_BASE/empty-plugin-XXXXXX")
mkdir -p "$EMPTY_ROOT/templates"
OUTPUT=$(config_enumerate_templates "$EMPTY_ROOT")
assert_empty "empty output for empty directory" "$OUTPUT"

echo "Test: Only returns .md files (ignores other extensions)"
MIXED_ROOT=$(mktemp -d "$TMPDIR_BASE/mixed-plugin-XXXXXX")
mkdir -p "$MIXED_ROOT/templates"
echo "template" > "$MIXED_ROOT/templates/plan.md"
echo "not a template" > "$MIXED_ROOT/templates/readme.txt"
echo "also not" > "$MIXED_ROOT/templates/notes.json"
OUTPUT=$(config_enumerate_templates "$MIXED_ROOT")
assert_eq "only returns plan" "plan" "$OUTPUT"

echo "Test: Returns nothing if directory exists with only non-.md files"
NOMD_ROOT=$(mktemp -d "$TMPDIR_BASE/nomd-plugin-XXXXXX")
mkdir -p "$NOMD_ROOT/templates"
echo "not md" > "$NOMD_ROOT/templates/readme.txt"
OUTPUT=$(config_enumerate_templates "$NOMD_ROOT")
assert_empty "empty output for non-md files" "$OUTPUT"

echo ""

# ============================================================
echo "=== config_resolve_template ==="
echo ""

echo "Test: Resolves to plugin default when no config or override exists"
REPO=$(setup_repo)
RESOLUTION=$(cd "$REPO" && config_resolve_template "plan" "$PLUGIN_ROOT")
IFS=$'\t' read -r SOURCE PATH_VAL <<< "$RESOLUTION"
assert_eq "source is plugin default" "$CONFIG_TEMPLATE_SOURCE_PLUGIN_DEFAULT" "$SOURCE"
assert_contains "path contains templates/plan.md" "templates/plan.md" "$PATH_VAL"

echo "Test: Resolves to user override when present in templates directory"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
echo "# Custom" > "$REPO/meta/templates/plan.md"
RESOLUTION=$(cd "$REPO" && config_resolve_template "plan" "$PLUGIN_ROOT")
IFS=$'\t' read -r SOURCE PATH_VAL <<< "$RESOLUTION"
assert_eq "source is user override" "$CONFIG_TEMPLATE_SOURCE_USER_OVERRIDE" "$SOURCE"
assert_contains "path contains meta/templates/plan.md" "meta/templates/plan.md" "$PATH_VAL"

echo "Test: Resolves to config path when templates.<key> is set"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
mkdir -p "$REPO/custom"
echo "# Config Path" > "$REPO/custom/my-plan.md"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
templates:
  plan: custom/my-plan.md
---
FIXTURE
RESOLUTION=$(cd "$REPO" && config_resolve_template "plan" "$PLUGIN_ROOT")
IFS=$'\t' read -r SOURCE PATH_VAL <<< "$RESOLUTION"
assert_eq "source is config path" "$CONFIG_TEMPLATE_SOURCE_CONFIG_PATH" "$SOURCE"
assert_contains "path contains custom/my-plan.md" "custom/my-plan.md" "$PATH_VAL"

echo "Test: Config path takes precedence over user override"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
mkdir -p "$REPO/custom"
mkdir -p "$REPO/meta/templates"
echo "# Config Path" > "$REPO/custom/my-plan.md"
echo "# User Override" > "$REPO/meta/templates/plan.md"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
templates:
  plan: custom/my-plan.md
---
FIXTURE
RESOLUTION=$(cd "$REPO" && config_resolve_template "plan" "$PLUGIN_ROOT")
IFS=$'\t' read -r SOURCE PATH_VAL <<< "$RESOLUTION"
assert_eq "source is config path (precedence)" "$CONFIG_TEMPLATE_SOURCE_CONFIG_PATH" "$SOURCE"

echo "Test: Returns 1 when template key is unknown"
REPO=$(setup_repo)
RC=0
cd "$REPO" && config_resolve_template "nonexistent" "$PLUGIN_ROOT" >/dev/null 2>&1 || RC=$?
assert_eq "returns 1 for unknown key" "1" "$RC"

echo "Test: Emits warning to stderr when config path is missing but falls back"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
templates:
  plan: nonexistent/plan.md
---
FIXTURE
STDERR_OUTPUT=$(cd "$REPO" && config_resolve_template "plan" "$PLUGIN_ROOT" 2>&1 1>/dev/null)
assert_contains "warning about missing config path" "Warning" "$STDERR_OUTPUT"

echo ""

# ============================================================
echo "=== config_format_available_templates ==="
echo ""

echo "Test: Formats template keys as comma-separated list"
OUTPUT=$(config_format_available_templates "$PLUGIN_ROOT")
assert_contains "contains plan" "plan" "$OUTPUT"
assert_contains "contains comma separator" ", " "$OUTPUT"

echo "Test: Returns '(none found)' when no templates exist"
EMPTY_ROOT=$(mktemp -d "$TMPDIR_BASE/empty-fmt-XXXXXX")
mkdir -p "$EMPTY_ROOT/templates"
OUTPUT=$(config_format_available_templates "$EMPTY_ROOT")
assert_eq "none found message" "(none found)" "$OUTPUT"

echo ""

# ============================================================
echo "=== config-dump.sh pr-description template key ==="
echo ""

echo "Test: Output contains templates.pr-description row"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\ntemplates:\n  pr-description: custom/pr.md\n---\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
assert_contains "pr-description key in dump" "templates.pr-description" "$OUTPUT"

echo "Test: Output contains templates.work-item row"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\ntemplates:\n  work-item: custom/work-item.md\n---\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
assert_contains "work-item key in dump" "templates.work-item" "$OUTPUT"

echo "Test: Output contains paths.review_work row"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\npaths:\n  review_work: docs/reviews/work\n---\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
assert_contains "review_work key in dump" "paths.review_work" "$OUTPUT"

echo ""

# ============================================================
echo "=== config-read-template.sh regression (refactored) ==="
echo ""

echo "Test: Unknown template lists all 6 template names including pr-description and work-item"
REPO=$(setup_repo)
STDERR_OUTPUT=$(cd "$REPO" && bash "$READ_TEMPLATE" "nonexistent" 2>&1 1>/dev/null || true)
assert_contains "error lists plan" "plan" "$STDERR_OUTPUT"
assert_contains "error lists research" "research" "$STDERR_OUTPUT"
assert_contains "error lists adr" "adr" "$STDERR_OUTPUT"
assert_contains "error lists validation" "validation" "$STDERR_OUTPUT"
assert_contains "error lists pr-description" "pr-description" "$STDERR_OUTPUT"
assert_contains "error lists work-item" "work-item" "$STDERR_OUTPUT"

echo ""

# ============================================================
echo "=== config-list-template.sh ==="
echo ""

echo "Test: No config -> all 5 templates show plugin default source"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tmp" && touch "$REPO/meta/tmp/.gitignore"
OUTPUT=$(cd "$REPO" && bash "$LIST_TEMPLATE")
LINE_COUNT=$(echo "$OUTPUT" | grep -c '| `' || true)
assert_eq "6 template rows" "6" "$LINE_COUNT"
for KEY in plan research adr validation pr-description work-item; do
  if echo "$OUTPUT" | grep "\`$KEY\`" | grep -q "plugin default"; then
    echo "  PASS: $KEY shows plugin default"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $KEY shows plugin default"
    echo "    Output: $(echo "$OUTPUT" | grep "$KEY" || echo "(not found)")"
    FAIL=$((FAIL + 1))
  fi
done

echo "Test: User override in meta/templates/ -> shows user override"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
echo "# Custom" > "$REPO/meta/templates/plan.md"
OUTPUT=$(cd "$REPO" && bash "$LIST_TEMPLATE")
if echo "$OUTPUT" | grep '`plan`' | grep -q "user override"; then
  echo "  PASS: plan shows user override"
  PASS=$((PASS + 1))
else
  echo "  FAIL: plan shows user override"
  echo "    Output: $(echo "$OUTPUT" | grep 'plan' || echo "(not found)")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Config path override -> shows config path"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
mkdir -p "$REPO/custom"
echo "# Config" > "$REPO/custom/my-plan.md"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
templates:
  plan: custom/my-plan.md
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$LIST_TEMPLATE")
if echo "$OUTPUT" | grep '`plan`' | grep -q "config path"; then
  echo "  PASS: plan shows config path"
  PASS=$((PASS + 1))
else
  echo "  FAIL: plan shows config path"
  echo "    Output: $(echo "$OUTPUT" | grep 'plan' || echo "(not found)")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Custom paths.templates -> finds override in custom directory"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
mkdir -p "$REPO/docs/tpl"
echo "# Custom" > "$REPO/docs/tpl/research.md"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  templates: docs/tpl
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$LIST_TEMPLATE")
if echo "$OUTPUT" | grep '`research`' | grep -q "user override"; then
  echo "  PASS: research shows user override via custom paths.templates"
  PASS=$((PASS + 1))
else
  echo "  FAIL: research shows user override via custom paths.templates"
  echo "    Output: $(echo "$OUTPUT" | grep 'research' || echo "(not found)")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Output is valid markdown table (starts with header row)"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$LIST_TEMPLATE")
FIRST_LINE=$(echo "$OUTPUT" | head -1)
assert_eq "header row" "| Template | Source | Path |" "$FIRST_LINE"
SECOND_LINE=$(echo "$OUTPUT" | sed -n '2p')
assert_eq "separator row" "|----------|--------|------|" "$SECOND_LINE"

echo "Test: Mixed sources in single run"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
mkdir -p "$REPO/custom"
mkdir -p "$REPO/meta/templates"
echo "# Config" > "$REPO/custom/my-plan.md"
echo "# Override" > "$REPO/meta/templates/research.md"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
templates:
  plan: custom/my-plan.md
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$LIST_TEMPLATE")
if echo "$OUTPUT" | grep '`plan`' | grep -q "config path" && \
   echo "$OUTPUT" | grep '`research`' | grep -q "user override" && \
   echo "$OUTPUT" | grep '`adr`' | grep -q "plugin default"; then
  echo "  PASS: mixed sources correctly labelled"
  PASS=$((PASS + 1))
else
  echo "  FAIL: mixed sources correctly labelled"
  echo "    Output: $OUTPUT"
  FAIL=$((FAIL + 1))
fi

echo ""

# ============================================================
echo "=== config-show-template.sh ==="
echo ""

echo "Test: No override -> shows Source: plugin default + raw content"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$SHOW_TEMPLATE" "plan")
FIRST_LINE=$(echo "$OUTPUT" | head -1)
assert_contains "source line says plugin default" "Source: plugin default" "$FIRST_LINE"
SECOND_LINE=$(echo "$OUTPUT" | sed -n '2p')
assert_eq "separator line" "---" "$SECOND_LINE"
# Content should NOT have code fences
if echo "$OUTPUT" | grep -q '```markdown'; then
  echo "  FAIL: should not contain code fences"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: no code fences in output"
  PASS=$((PASS + 1))
fi

echo "Test: User override -> shows Source: user override + user content"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
echo "# My Custom Plan" > "$REPO/meta/templates/plan.md"
OUTPUT=$(cd "$REPO" && bash "$SHOW_TEMPLATE" "plan")
assert_contains "source line says user override" "Source: user override" "$(echo "$OUTPUT" | head -1)"
assert_contains "contains user content" "My Custom Plan" "$OUTPUT"

echo "Test: Config path override -> shows Source: config path + content"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
mkdir -p "$REPO/custom"
echo "# Config Plan" > "$REPO/custom/my-plan.md"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
templates:
  plan: custom/my-plan.md
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$SHOW_TEMPLATE" "plan")
assert_contains "source line says config path" "Source: config path" "$(echo "$OUTPUT" | head -1)"
assert_contains "contains config content" "Config Plan" "$OUTPUT"

echo "Test: Unknown template name -> error to stderr, exit 1"
REPO=$(setup_repo)
STDERR_OUTPUT=$(cd "$REPO" && bash "$SHOW_TEMPLATE" "nonexistent" 2>&1 1>/dev/null || true)
assert_contains "error mentions available templates" "Available templates:" "$STDERR_OUTPUT"
assert_exit_code "exits 1 for unknown template" 1 bash "$SHOW_TEMPLATE" "nonexistent"

echo "Test: No argument -> usage to stderr, exit 1"
REPO=$(setup_repo)
STDERR_OUTPUT=$(cd "$REPO" && bash "$SHOW_TEMPLATE" 2>&1 1>/dev/null || true)
assert_contains "usage message" "Usage:" "$STDERR_OUTPUT"
assert_exit_code "exits 1 for no argument" 1 bash "$SHOW_TEMPLATE"

echo "Test: Content is raw (no code fences added)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
printf '# Raw Template\n\nSome content here.\n' > "$REPO/meta/templates/plan.md"
OUTPUT=$(cd "$REPO" && bash "$SHOW_TEMPLATE" "plan")
# Extract content after the --- separator
CONTENT=$(echo "$OUTPUT" | sed '1,2d')
if echo "$CONTENT" | grep -q '```'; then
  echo "  FAIL: raw content should not contain added fences"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: content is raw without added fences"
  PASS=$((PASS + 1))
fi
assert_contains "raw content present" "Some content here." "$CONTENT"

echo ""

# ============================================================
echo "=== config-eject-template.sh ==="
echo ""

echo "Test: Ejects template to default meta/templates/ directory"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$EJECT_TEMPLATE" "plan")
assert_file_exists "plan.md created" "$REPO/meta/templates/plan.md"
assert_contains "ejected message" "Ejected:" "$OUTPUT"

echo "Test: Creates templates directory if it doesn't exist"
REPO=$(setup_repo)
cd "$REPO" && bash "$EJECT_TEMPLATE" "plan" >/dev/null
assert_file_exists "directory and file created" "$REPO/meta/templates/plan.md"

echo "Test: File content matches plugin default"
REPO=$(setup_repo)
cd "$REPO" && bash "$EJECT_TEMPLATE" "plan" >/dev/null
EXPECTED=$(cat "$PLUGIN_ROOT/templates/plan.md")
assert_file_content_eq "content matches plugin default" "$REPO/meta/templates/plan.md" "$EXPECTED"

echo "Test: Exit code 2 when target exists without --force"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
echo "# Existing" > "$REPO/meta/templates/plan.md"
RC=0
cd "$REPO" && bash "$EJECT_TEMPLATE" "plan" >/dev/null 2>&1 || RC=$?
assert_eq "exit code 2" "2" "$RC"
assert_file_content_eq "file unchanged" "$REPO/meta/templates/plan.md" "# Existing"

echo "Test: --force overwrites existing file, exit 0"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
echo "# Existing" > "$REPO/meta/templates/plan.md"
RC=0
OUTPUT=$(cd "$REPO" && bash "$EJECT_TEMPLATE" --force "plan") || RC=$?
assert_eq "exit code 0" "0" "$RC"
assert_contains "overwritten message" "Overwritten:" "$OUTPUT"
EXPECTED=$(cat "$PLUGIN_ROOT/templates/plan.md")
assert_file_content_eq "content replaced with plugin default" "$REPO/meta/templates/plan.md" "$EXPECTED"

echo "Test: --all ejects all 6 templates"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$EJECT_TEMPLATE" --all)
for KEY in plan research adr validation pr-description work-item; do
  assert_file_exists "$KEY ejected" "$REPO/meta/templates/${KEY}.md"
done

echo "Test: --all --force overwrites all existing"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
echo "# Old" > "$REPO/meta/templates/plan.md"
echo "# Old" > "$REPO/meta/templates/research.md"
RC=0
OUTPUT=$(cd "$REPO" && bash "$EJECT_TEMPLATE" --all --force) || RC=$?
assert_eq "exit code 0" "0" "$RC"
EXPECTED=$(cat "$PLUGIN_ROOT/templates/plan.md")
assert_file_content_eq "plan overwritten" "$REPO/meta/templates/plan.md" "$EXPECTED"

echo "Test: --all with some existing files exits 2 without --force"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
echo "# Existing" > "$REPO/meta/templates/plan.md"
RC=0
cd "$REPO" && bash "$EJECT_TEMPLATE" --all >/dev/null 2>&1 || RC=$?
assert_eq "exit code 2" "2" "$RC"
# Non-conflicting templates should still be written
assert_file_exists "research still ejected" "$REPO/meta/templates/research.md"

echo "Test: --dry-run outputs what would happen without writing files"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$EJECT_TEMPLATE" --dry-run "plan")
assert_contains "would eject message" "Would eject:" "$OUTPUT"
assert_file_not_exists "file not created" "$REPO/meta/templates/plan.md"

echo "Test: --dry-run produces exit 0 for non-existing target"
REPO=$(setup_repo)
RC=0
cd "$REPO" && bash "$EJECT_TEMPLATE" --dry-run "plan" >/dev/null 2>&1 || RC=$?
assert_eq "exit code 0 for dry-run" "0" "$RC"

echo "Test: --dry-run with existing file shows would skip"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
echo "# Existing" > "$REPO/meta/templates/plan.md"
OUTPUT=$(cd "$REPO" && bash "$EJECT_TEMPLATE" --dry-run "plan" 2>&1) || true
assert_contains "would skip message" "Would skip:" "$OUTPUT"

echo "Test: Multiple positional arguments -> error, exit 1"
REPO=$(setup_repo)
RC=0
STDERR_OUTPUT=$(cd "$REPO" && bash "$EJECT_TEMPLATE" "plan" "research" 2>&1 1>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_contains "unexpected argument error" "unexpected argument" "$STDERR_OUTPUT"

echo "Test: Respects paths.templates config override"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  templates: docs/tpl
---
FIXTURE
cd "$REPO" && bash "$EJECT_TEMPLATE" "plan" >/dev/null
assert_file_exists "ejected to custom dir" "$REPO/docs/tpl/plan.md"

echo "Test: Unknown template name -> error, exit 1"
REPO=$(setup_repo)
RC=0
STDERR_OUTPUT=$(cd "$REPO" && bash "$EJECT_TEMPLATE" "nonexistent" 2>&1 1>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_contains "error lists available" "Available:" "$STDERR_OUTPUT"

echo "Test: No argument -> usage, exit 1"
REPO=$(setup_repo)
RC=0
STDERR_OUTPUT=$(cd "$REPO" && bash "$EJECT_TEMPLATE" 2>&1 1>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_contains "usage message" "Usage:" "$STDERR_OUTPUT"

echo ""

# ============================================================
echo "=== config-diff-template.sh ==="
echo ""

echo "Test: No override -> 'No customised template found' to stderr, exit 2"
REPO=$(setup_repo)
RC=0
STDERR_OUTPUT=$(cd "$REPO" && bash "$DIFF_TEMPLATE" "plan" 2>&1 1>/dev/null) || RC=$?
assert_eq "exit code 2" "2" "$RC"
assert_contains "no customised message" "No customised template found" "$STDERR_OUTPUT"

echo "Test: User override with differences -> outputs unified diff"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
cp "$PLUGIN_ROOT/templates/plan.md" "$REPO/meta/templates/plan.md"
echo "# Extra line added by user" >> "$REPO/meta/templates/plan.md"
RC=0
OUTPUT=$(cd "$REPO" && bash "$DIFF_TEMPLATE" "plan") || RC=$?
assert_contains "diff header present" "Comparing plugin default vs user override:" "$OUTPUT"
assert_contains "diff contains addition" "+# Extra line added by user" "$OUTPUT"

echo "Test: User override with known added line -> diff shows + prefix"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
printf 'Modified content\n' > "$REPO/meta/templates/plan.md"
RC=0
OUTPUT=$(cd "$REPO" && bash "$DIFF_TEMPLATE" "plan") || RC=$?
assert_contains "additions shown with +" "+Modified content" "$OUTPUT"

echo "Test: User override identical to default -> 'Templates are identical.'"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
cp "$PLUGIN_ROOT/templates/plan.md" "$REPO/meta/templates/plan.md"
OUTPUT=$(cd "$REPO" && bash "$DIFF_TEMPLATE" "plan")
assert_contains "identical message" "Templates are identical." "$OUTPUT"

echo "Test: Config path override -> diffs correctly"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
mkdir -p "$REPO/custom"
cp "$PLUGIN_ROOT/templates/plan.md" "$REPO/custom/my-plan.md"
echo "# Config override addition" >> "$REPO/custom/my-plan.md"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
templates:
  plan: custom/my-plan.md
---
FIXTURE
RC=0
OUTPUT=$(cd "$REPO" && bash "$DIFF_TEMPLATE" "plan") || RC=$?
assert_contains "config path diff shows addition" "+# Config override addition" "$OUTPUT"

echo "Test: Unknown template name -> error, exit 1"
REPO=$(setup_repo)
RC=0
STDERR_OUTPUT=$(cd "$REPO" && bash "$DIFF_TEMPLATE" "nonexistent" 2>&1 1>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_contains "error lists available" "Available:" "$STDERR_OUTPUT"

echo "Test: No argument -> usage, exit 1"
REPO=$(setup_repo)
RC=0
STDERR_OUTPUT=$(cd "$REPO" && bash "$DIFF_TEMPLATE" 2>&1 1>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_contains "usage message" "Usage:" "$STDERR_OUTPUT"

echo ""

# ============================================================
echo "=== config-reset-template.sh ==="
echo ""

echo "Test: No override -> exit 2 with 'already using plugin default' to stderr"
REPO=$(setup_repo)
RC=0
STDERR_OUTPUT=$(cd "$REPO" && bash "$RESET_TEMPLATE" "plan" 2>&1 1>/dev/null) || RC=$?
assert_eq "exit code 2" "2" "$RC"
assert_contains "already using default" "already using plugin default" "$STDERR_OUTPUT"

echo "Test: User override without --confirm -> exit 0, outputs override path and source"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
echo "# Custom" > "$REPO/meta/templates/plan.md"
OUTPUT=$(cd "$REPO" && bash "$RESET_TEMPLATE" "plan")
assert_contains "found override" "Found override:" "$OUTPUT"
assert_contains "source is user override" "user override" "$OUTPUT"
assert_contains "path shown" "meta/templates/plan.md" "$OUTPUT"
assert_file_exists "file still exists (not deleted)" "$REPO/meta/templates/plan.md"

echo "Test: Config path override without --confirm -> includes note about config entry"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
mkdir -p "$REPO/custom"
echo "# Config" > "$REPO/custom/my-plan.md"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
templates:
  plan: custom/my-plan.md
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$RESET_TEMPLATE" "plan")
assert_contains "config path source" "config path" "$OUTPUT"
assert_contains "note about config entry" "also remove the 'templates.plan' entry" "$OUTPUT"

echo "Test: Config path outside project root without --confirm -> warning shown"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
OUTSIDE_FILE=$(mktemp "$TMPDIR_BASE/outside-plan-XXXXXX.md")
echo "# Outside" > "$OUTSIDE_FILE"
cat > "$REPO/.claude/accelerator.md" << FIXTURE
---
templates:
  plan: $OUTSIDE_FILE
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$RESET_TEMPLATE" "plan")
assert_contains "outside project warning" "Warning: This file is outside the project directory" "$OUTPUT"

echo "Test: --confirm with user override -> deletes file"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
echo "# Custom" > "$REPO/meta/templates/plan.md"
OUTPUT=$(cd "$REPO" && bash "$RESET_TEMPLATE" --confirm "plan")
assert_file_not_exists "file deleted" "$REPO/meta/templates/plan.md"
assert_contains "reset message" "Reset: plan" "$OUTPUT"

echo "Test: --confirm with no override -> exit 2"
REPO=$(setup_repo)
RC=0
cd "$REPO" && bash "$RESET_TEMPLATE" --confirm "plan" >/dev/null 2>&1 || RC=$?
assert_eq "exit code 2" "2" "$RC"

echo "Test: Unknown template name -> error, exit 1"
REPO=$(setup_repo)
RC=0
STDERR_OUTPUT=$(cd "$REPO" && bash "$RESET_TEMPLATE" "nonexistent" 2>&1 1>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_contains "error lists available" "Available:" "$STDERR_OUTPUT"

echo "Test: No argument -> usage, exit 1"
REPO=$(setup_repo)
RC=0
STDERR_OUTPUT=$(cd "$REPO" && bash "$RESET_TEMPLATE" 2>&1 1>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_contains "usage message" "Usage:" "$STDERR_OUTPUT"

echo ""

# ============================================================
echo "=== Skill integration: configure skill references template scripts ==="
echo ""

CONFIGURE_SKILL="$SKILLS_DIR/config/configure/SKILL.md"

echo "Test: Configure skill SKILL.md contains config-list-template.sh"
if grep -q 'config-list-template.sh' "$CONFIGURE_SKILL"; then
  echo "  PASS: config-list-template.sh referenced"
  PASS=$((PASS + 1))
else
  echo "  FAIL: config-list-template.sh referenced"
  FAIL=$((FAIL + 1))
fi

echo "Test: Configure skill SKILL.md contains config-show-template.sh"
if grep -q 'config-show-template.sh' "$CONFIGURE_SKILL"; then
  echo "  PASS: config-show-template.sh referenced"
  PASS=$((PASS + 1))
else
  echo "  FAIL: config-show-template.sh referenced"
  FAIL=$((FAIL + 1))
fi

echo "Test: Configure skill SKILL.md contains config-eject-template.sh"
if grep -q 'config-eject-template.sh' "$CONFIGURE_SKILL"; then
  echo "  PASS: config-eject-template.sh referenced"
  PASS=$((PASS + 1))
else
  echo "  FAIL: config-eject-template.sh referenced"
  FAIL=$((FAIL + 1))
fi

echo "Test: Configure skill SKILL.md contains config-diff-template.sh"
if grep -q 'config-diff-template.sh' "$CONFIGURE_SKILL"; then
  echo "  PASS: config-diff-template.sh referenced"
  PASS=$((PASS + 1))
else
  echo "  FAIL: config-diff-template.sh referenced"
  FAIL=$((FAIL + 1))
fi

echo "Test: Configure skill SKILL.md contains config-reset-template.sh"
if grep -q 'config-reset-template.sh' "$CONFIGURE_SKILL"; then
  echo "  PASS: config-reset-template.sh referenced"
  PASS=$((PASS + 1))
else
  echo "  FAIL: config-reset-template.sh referenced"
  FAIL=$((FAIL + 1))
fi

echo ""

# ============================================================
echo "=== Template management integration tests ==="
echo ""

echo "Test: Eject then list: plan shows as user override"
REPO=$(setup_repo)
cd "$REPO" && bash "$EJECT_TEMPLATE" "plan" >/dev/null
OUTPUT=$(cd "$REPO" && bash "$LIST_TEMPLATE")
if echo "$OUTPUT" | grep '`plan`' | grep -q "user override"; then
  echo "  PASS: plan shows user override after eject"
  PASS=$((PASS + 1))
else
  echo "  FAIL: plan shows user override after eject"
  echo "    Output: $(echo "$OUTPUT" | grep 'plan' || echo "(not found)")"
  FAIL=$((FAIL + 1))
fi

echo "Test: Eject then diff (identical): produces 'Templates are identical'"
REPO=$(setup_repo)
cd "$REPO" && bash "$EJECT_TEMPLATE" "plan" >/dev/null
OUTPUT=$(cd "$REPO" && bash "$DIFF_TEMPLATE" "plan")
assert_contains "identical message" "Templates are identical." "$OUTPUT"

echo "Test: Eject + edit + diff: shows addition with + prefix"
REPO=$(setup_repo)
cd "$REPO" && bash "$EJECT_TEMPLATE" "plan" >/dev/null
echo "# User addition" >> "$REPO/meta/templates/plan.md"
RC=0
OUTPUT=$(cd "$REPO" && bash "$DIFF_TEMPLATE" "plan") || RC=$?
assert_contains "addition shown with +" "+# User addition" "$OUTPUT"

echo "Test: Eject then reset: deletes the override"
REPO=$(setup_repo)
cd "$REPO" && bash "$EJECT_TEMPLATE" "plan" >/dev/null
assert_file_exists "plan exists after eject" "$REPO/meta/templates/plan.md"
cd "$REPO" && bash "$RESET_TEMPLATE" --confirm "plan" >/dev/null
assert_file_not_exists "plan deleted after reset" "$REPO/meta/templates/plan.md"

echo ""

# ============================================================
echo "=== init SKILL.md directory count invariant ==="
echo ""

echo "Test: init SKILL.md directory count matches Path Resolution list"
INIT_SKILL="$PLUGIN_ROOT/skills/config/init/SKILL.md"
EXPECTED=$(grep -cE '^\*\*[A-Za-z][^*]* directory\*\*:' "$INIT_SKILL")
ACTUAL=$(grep -oE 'each of the [0-9]+ directories' "$INIT_SKILL" \
  | grep -oE '[0-9]+' | head -1)
assert_eq "directory count agrees with Path Resolution list" \
  "$EXPECTED" "$ACTUAL"

echo ""

test_summary
