#!/usr/bin/env bash
set -euo pipefail

# Test harness for hooks/migrate-discoverability.sh
# Run: bash hooks/test-migrate-discoverability.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOK="$SCRIPT_DIR/migrate-discoverability.sh"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# Run the hook with PROJECT_ROOT set to a temp dir and capture outputs.
run_hook() {
  local repo="$1"
  PROJECT_ROOT="$repo" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$HOOK" 2>&1 || true
}

run_hook_stderr() {
  local repo="$1"
  PROJECT_ROOT="$repo" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$HOOK" 2>&1 >/dev/null || true
}

echo "=== migrate-discoverability.sh ==="
echo ""

# ── Test 1: silent on a non-Accelerator repo ──────────────────────────────────
echo "Test: silent on a non-Accelerator repo"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
OUTPUT=$(run_hook "$REPO")
assert_empty "no output" "$OUTPUT"

# ── Test 2: triggers on pre-migration repo with .claude/accelerator.md ────────
echo "Test: triggers on pre-migration repo with .claude/accelerator.md"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/.claude"
touch "$REPO/.claude/accelerator.md"
OUTPUT=$(run_hook "$REPO")
assert_contains "warning emitted" "$OUTPUT" "is behind the plugin"

# ── Test 3: triggers on pre-migration repo with only meta/ ────────────────────
echo "Test: triggers on pre-migration repo with only meta/"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta"
OUTPUT=$(run_hook "$REPO")
assert_contains "warning emitted" "$OUTPUT" "is behind the plugin"

# ── Test 4: state-file read from new path when .accelerator/state/migrations-applied exists ─
echo "Test: state-file read from .accelerator/state/migrations-applied when it exists"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/.accelerator/state"
# Only 0001 applied — 0002 and 0003 pending — so the hook emits a warning with the file path
printf '0001-rename-tickets-to-work\n' > "$REPO/.accelerator/state/migrations-applied"
OUTPUT=$(run_hook "$REPO")
assert_contains "references new state file path" "$OUTPUT" ".accelerator/state/migrations-applied"
assert_contains "warning emitted for pending migration" "$OUTPUT" "is behind the plugin"

# ── Test 5: state-file fallback uses meta/.migrations-applied when .accelerator/ absent ──
echo "Test: state-file fallback uses meta/.migrations-applied when .accelerator/ absent"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta"
printf '0001-rename-tickets-to-work\n' > "$REPO/meta/.migrations-applied"
OUTPUT=$(run_hook "$REPO")
assert_contains "references legacy state file path" "$OUTPUT" "meta/.migrations-applied"

# ── Test 6: partial-recovery — .accelerator/ exists but its state file does not ─
echo "Test: partial-recovery state — .accelerator/ exists but its state file does not"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/.accelerator"
mkdir -p "$REPO/meta"
printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n' \
  > "$REPO/meta/.migrations-applied"
OUTPUT=$(run_hook "$REPO")
# Must read from meta/ fallback (per-file existence, not per-directory)
assert_contains "uses legacy fallback" "$OUTPUT" "meta/.migrations-applied"
assert_contains "warns about pending migration" "$OUTPUT" "is behind the plugin"

# ── Test 7: hook exits 0 in every scenario ────────────────────────────────────
echo "Test: hook exits 0 on non-Accelerator repo"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
RC=0
PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$HOOK" >/dev/null 2>&1 || RC=$?
assert_eq "exits 0" "0" "$RC"

echo "Test: hook exits 0 on pre-migration repo"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/.claude"
touch "$REPO/.claude/accelerator.md"
RC=0
PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$HOOK" >/dev/null 2>&1 || RC=$?
assert_eq "exits 0" "0" "$RC"

echo "Test: hook exits 0 on fully-migrated repo with no pending migrations"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/.accelerator/state"
printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n0003-relocate-accelerator-state\n' \
  > "$REPO/.accelerator/state/migrations-applied"
RC=0
PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$HOOK" >/dev/null 2>&1 || RC=$?
assert_eq "exits 0" "0" "$RC"

echo ""
test_summary
