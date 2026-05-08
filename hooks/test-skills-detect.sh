#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOK="$SCRIPT_DIR/skills-detect.sh"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

setup_fake_plugin() {
  local dir
  dir=$(mktemp -d -p "$TMPDIR_BASE")
  mkdir -p "$dir/agents" "$dir/scripts" "$dir/hooks"
  cp "$PLUGIN_ROOT/scripts/config-common.sh" "$dir/scripts/"
  cp "$PLUGIN_ROOT/scripts/config-defaults.sh" "$dir/scripts/"
  cp "$PLUGIN_ROOT/scripts/vcs-common.sh" "$dir/scripts/"
  cp "$PLUGIN_ROOT/scripts/config-read-value.sh" "$dir/scripts/"
  cp "$PLUGIN_ROOT/scripts/config-read-path.sh" "$dir/scripts/"
  echo "$dir"
}

echo "=== hooks/skills-detect.sh ==="
echo ""

echo "Test: no agents with skills: frontmatter → no output"
FAKE=$(setup_fake_plugin)
cat > "$FAKE/agents/example.md" << 'EOF'
---
name: example
description: An example agent with no skills.
tools: Grep
---
Example agent body.
EOF
OUTPUT=$(CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
assert_eq "no skills frontmatter → empty output" "" "$OUTPUT"

setup_fake_skill() {
  local fake_root="$1"
  mkdir -p "$fake_root/skills/config/paths"
  cp "$PLUGIN_ROOT/skills/config/paths/SKILL.md" "$fake_root/skills/config/paths/"
  cp "$PLUGIN_ROOT/scripts/config-read-all-paths.sh" "$fake_root/scripts/"
}

echo "Test: agent with skills: [paths] → output contains ## Configured Paths"
FAKE=$(setup_fake_plugin)
setup_fake_skill "$FAKE"
cat > "$FAKE/agents/doc-locator.md" << 'EOF'
---
name: doc-locator
skills: [paths]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git"
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
assert_contains "paths skill injected" "$OUTPUT" "Configured Paths"

echo "Test: output is valid JSON with additionalContext key"
FAKE=$(setup_fake_plugin)
setup_fake_skill "$FAKE"
cat > "$FAKE/agents/doc-locator.md" << 'EOF'
---
name: doc-locator
skills: [paths]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git"
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
CONTEXT=$(echo "$OUTPUT" | jq -r '.hookSpecificOutput.additionalContext' 2>/dev/null || true)
assert_contains "valid JSON with additionalContext" "$CONTEXT" "Configured Paths"

echo "Test: config override flows through to additionalContext"
FAKE=$(setup_fake_plugin)
setup_fake_skill "$FAKE"
cat > "$FAKE/agents/doc-locator.md" << 'EOF'
---
name: doc-locator
skills: [paths]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git" "$REPO/.accelerator"
cat > "$REPO/.accelerator/config.md" << 'FIXTURE'
---
paths:
  work: custom/work-items
---
FIXTURE
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
CONTEXT=$(echo "$OUTPUT" | jq -r '.hookSpecificOutput.additionalContext' 2>/dev/null || true)
assert_contains "config override in context" "$CONTEXT" "work: custom/work-items"
assert_contains "default still present" "$CONTEXT" "plans: meta/plans"

echo "Test: two agents sharing the same skill → skill content accumulated once per occurrence"
FAKE=$(setup_fake_plugin)
setup_fake_skill "$FAKE"
cat > "$FAKE/agents/agent-a.md" << 'EOF'
---
name: agent-a
skills: [paths]
tools: Grep
---
Body.
EOF
cat > "$FAKE/agents/agent-b.md" << 'EOF'
---
name: agent-b
skills: [paths]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git"
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
CONTEXT=$(echo "$OUTPUT" | jq -r '.hookSpecificOutput.additionalContext' 2>/dev/null || true)
assert_contains "multi-agent: context present" "$CONTEXT" "Configured Paths"

echo "Test: unknown skill name → silently skipped (no crash, no output)"
FAKE=$(setup_fake_plugin)
cat > "$FAKE/agents/has-missing-skill.md" << 'EOF'
---
name: has-missing-skill
skills: [nonexistent]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git"
EXIT_CODE=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null) || EXIT_CODE=$?
assert_eq "missing skill → exit 0" "0" "$EXIT_CODE"
assert_eq "missing skill → no output" "" "$OUTPUT"

echo "Test: bang line outside \$PLUGIN_ROOT/scripts/ → silently skipped (allowlist rejection)"
FAKE=$(setup_fake_plugin)
mkdir -p "$FAKE/skills/config/malicious"
cat > "$FAKE/skills/config/malicious/SKILL.md" << 'EOF'
---
name: malicious
user-invocable: false
---
## Section
!`/bin/sh -c 'echo PWNED'`
EOF
cat > "$FAKE/agents/has-malicious-skill.md" << 'EOF'
---
name: has-malicious-skill
skills: [malicious]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git"
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
CONTEXT=$(echo "$OUTPUT" | jq -r '.hookSpecificOutput.additionalContext' 2>/dev/null || echo "")
if echo "$CONTEXT" | grep -q "PWNED"; then
  echo "  FAIL: allowlist rejected — bang output outside scripts/ was executed"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: bang line outside scripts/ was silently skipped"
  PASS=$((PASS + 1))
fi

echo "Test: skill with disable-model-invocation: true → skipped (not injected)"
FAKE=$(setup_fake_plugin)
mkdir -p "$FAKE/skills/config/disabled"
cat > "$FAKE/skills/config/disabled/SKILL.md" << 'EOF'
---
name: disabled
disable-model-invocation: true
---
## Should Not Appear
This content must not appear in additionalContext.
EOF
cat > "$FAKE/agents/has-disabled-skill.md" << 'EOF'
---
name: has-disabled-skill
skills: [disabled]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git"
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
assert_eq "disabled skill → no output" "" "$OUTPUT"

echo ""
test_summary
