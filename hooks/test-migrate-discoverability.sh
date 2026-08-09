#!/usr/bin/env bash
set -euo pipefail

# Parity gate for the `migrate --discoverability-hook` SessionStart hook,
# dispatched through the real `accelerator` launcher (ACCELERATOR_BIN) with
# ACCELERATOR_MIGRATE_BIN set so dispatch resolves to the locally-built
# accelerator-migrate sub-binary rather than fetching a signed release
# asset. This is the end-to-end path a real Claude Code session exercises
# via hooks/hooks.json's SessionStart entry.
#
# Repoints the bash-era hooks/migrate-discoverability.sh suite at the
# compiled binary — same scenarios, same assertions, run through the real
# launcher dispatch rather than the retired shell script directly.

if [ -z "${BASH_VERSION:-}" ]; then
  echo "hooks/test-migrate-discoverability.sh requires bash" >&2
  exit 1
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "hooks/test-migrate-discoverability.sh requires jq on PATH (run via 'mise run test:integration:hooks' or install jq)" >&2
  exit 77 # autotools 'skip' convention; harness reports as skipped
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ACCELERATOR_BIN="${ACCELERATOR_BIN:-$PLUGIN_ROOT/cli/target/debug/accelerator}"
ACCELERATOR_MIGRATE_BIN="${ACCELERATOR_MIGRATE_BIN:-$PLUGIN_ROOT/cli/target/debug/accelerator-migrate}"
export ACCELERATOR_LOG="${ACCELERATOR_LOG:-warn}"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

for binary in "$ACCELERATOR_BIN" "$ACCELERATOR_MIGRATE_BIN"; do
  if [ ! -x "$binary" ]; then
    echo "hooks/test-migrate-discoverability.sh requires $binary (run 'mise run build:cli:dev' or 'mise run test:integration:hooks', which depends on it)" >&2
    exit 77
  fi
done

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

run_hook() {
  local repo="$1"
  (
    cd "$repo" && ACCELERATOR_MIGRATE_BIN="$ACCELERATOR_MIGRATE_BIN" \
      "$ACCELERATOR_BIN" migrate --discoverability-hook --format=hook --fail-safe
  )
}

echo "=== accelerator migrate --discoverability-hook ==="
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
# Only 0001 applied — every later migration is pending — so the hook emits a
# warning with the file path.
printf '0001-rename-tickets-to-work\n' >"$REPO/.accelerator/state/migrations-applied"
OUTPUT=$(run_hook "$REPO")
assert_contains "references new state file path" "$OUTPUT" ".accelerator/state/migrations-applied"
assert_contains "warning emitted for pending migration" "$OUTPUT" "is behind the plugin"

# ── Test 5: state-file fallback uses meta/.migrations-applied when .accelerator/ absent ──
echo "Test: state-file fallback uses meta/.migrations-applied when .accelerator/ absent"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta"
printf '0001-rename-tickets-to-work\n' >"$REPO/meta/.migrations-applied"
OUTPUT=$(run_hook "$REPO")
assert_contains "references legacy state file path" "$OUTPUT" "meta/.migrations-applied"

# ── Test 6: partial-recovery — .accelerator/ exists but its state file does not ─
echo "Test: partial-recovery state — .accelerator/ exists but its state file does not"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/.accelerator"
mkdir -p "$REPO/meta"
printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n' \
  >"$REPO/meta/.migrations-applied"
OUTPUT=$(run_hook "$REPO")
# Must read from meta/ fallback (per-file existence, not per-directory)
assert_contains "uses legacy fallback" "$OUTPUT" "meta/.migrations-applied"
assert_contains "warns about pending migration" "$OUTPUT" "is behind the plugin"

# ── Test 7: hook exits 0 in every scenario ────────────────────────────────────
echo "Test: hook exits 0 on non-Accelerator repo"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
RC=0
run_hook "$REPO" >/dev/null 2>&1 || RC=$?
assert_eq "exits 0" "0" "$RC"

echo "Test: hook exits 0 on pre-migration repo"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/.claude"
touch "$REPO/.claude/accelerator.md"
RC=0
run_hook "$REPO" >/dev/null 2>&1 || RC=$?
assert_eq "exits 0" "0" "$RC"

echo "Test: hook exits 0 on fully-migrated repo with no pending migrations"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/.accelerator/state"
printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n0003-relocate-accelerator-state\n0004-restructure-meta-research-into-subject-subcategories\n0005-rename-work-item-type-to-kind\n0006-canonicalise-work-item-id-and-author\n' \
  >"$REPO/.accelerator/state/migrations-applied"
RC=0
run_hook "$REPO" >/dev/null 2>&1 || RC=$?
assert_eq "exits 0" "0" "$RC"

# ── Test 8: the envelope is valid JSON carrying a systemMessage ───────────────
echo "Test: the envelope is valid JSON carrying a systemMessage, not raw stderr"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta"
OUTPUT=$(run_hook "$REPO")
echo "$OUTPUT" | jq -e '.systemMessage | contains("is behind the plugin")' >/dev/null ||
  {
    echo "FAIL: expected a systemMessage carrying the advisory" >&2
    exit 1
  }

echo "=== hooks.json registration ==="

# ── hooks/hooks.json SessionStart migrate-discoverability entry intact ────────
# Order-independent (no SessionStart[N] indexing): finds the entry by its
# command string rather than assuming a fixed array position, so reordering
# hooks.json's SessionStart array does not break this guard.
echo "Test: hooks.json SessionStart entry has matcher='', one hook, expected command"
HOOKS_JSON="$PLUGIN_ROOT/hooks/hooks.json"
# shellcheck disable=SC2016 # single-quoted jq expressions; ${CLAUDE_PLUGIN_ROOT} is expanded by Claude Code at runtime, intentionally not shell-expanded
DISCOVERABILITY_SELECTOR='[.hooks.SessionStart[] | select(.hooks[0].command == "${CLAUDE_PLUGIN_ROOT}/bin/accelerator migrate --discoverability-hook --format=hook --fail-safe")][0]'
assert_json_eq "matcher empty" \
  "$DISCOVERABILITY_SELECTOR.matcher" "" "$HOOKS_JSON"
assert_json_eq "one hook entry" \
  "$DISCOVERABILITY_SELECTOR.hooks | length" "1" "$HOOKS_JSON"
assert_json_eq "type command" \
  "$DISCOVERABILITY_SELECTOR.hooks[0].type" "command" "$HOOKS_JSON"

echo ""
test_summary
