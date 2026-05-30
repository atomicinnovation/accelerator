#!/usr/bin/env bash
set -euo pipefail

# Test harness for the optional interactive migration contract.
# Run: bash skills/config/migrate/scripts/test-migrate-interactive.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

DRIVER="$SCRIPT_DIR/run-migrations.sh"
MIGRATIONS_DIR_FIXTURE="$SCRIPT_DIR/test-fixtures/interactive"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# Helper: stand up a sandbox PROJECT_ROOT for an interactive-fixture run.
# Each fixture directory under test-fixtures/interactive/ that contains its
# own migrations is invoked via ACCELERATOR_MIGRATIONS_DIR; per-test sandbox
# state lives under $TMPDIR_BASE.
setup_sandbox() {
  local name="$1"
  local sandbox
  sandbox=$(mktemp -d "$TMPDIR_BASE/$name-XXXXXX")
  mkdir -p "$sandbox/.git" "$sandbox/.accelerator/state"
  printf '%s\n' "$sandbox"
}

echo "=== Phase 1: env-var plumbing + session-log pre-flight UX ==="
echo ""

echo "Test: ACCELERATOR_MIGRATE_DECISIONS_FILE=/dev/null is a no-op on" \
     "a no-pending repo"
SANDBOX=$(setup_sandbox "decisions-devnull")
for f in "$PLUGIN_ROOT/skills/config/migrate/migrations"/[0-9][0-9][0-9][0-9]-*.sh; do
  basename "$f" .sh
done > "$SANDBOX/.accelerator/state/migrations-applied"

WITHOUT_VAR_STDOUT=$(PROJECT_ROOT="$SANDBOX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 bash "$DRIVER" 2>&1)
WITH_VAR_STDOUT=$(PROJECT_ROOT="$SANDBOX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE=/dev/null \
  bash "$DRIVER" 2>&1)
assert_eq "stdout byte-identical with vs without env var" \
  "$WITHOUT_VAR_STDOUT" "$WITH_VAR_STDOUT"

echo ""
echo "Test: env-var robustness — non-existent path"
SANDBOX=$(setup_sandbox "decisions-missing")
RC=0
OUTPUT=$(PROJECT_ROOT="$SANDBOX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$TMPDIR_BASE/nope-does-not-exist" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit for missing decisions file" "0" "$RC"
assert_contains "stderr names the missing file" "$OUTPUT" "does not exist"

echo ""
echo "Test: env-var robustness — directory"
SANDBOX=$(setup_sandbox "decisions-dir")
DIR_TARGET=$(mktemp -d "$TMPDIR_BASE/decisions-dir-target-XXXXXX")
RC=0
OUTPUT=$(PROJECT_ROOT="$SANDBOX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DIR_TARGET" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit for directory decisions value" "0" "$RC"
assert_contains "stderr names directory case" "$OUTPUT" "is a directory"

echo ""
echo "Test: env-var robustness — unreadable file"
SANDBOX=$(setup_sandbox "decisions-unreadable")
UNREAD=$(mktemp "$TMPDIR_BASE/unreadable-XXXXXX")
chmod 000 "$UNREAD"
RC=0
OUTPUT=$(PROJECT_ROOT="$SANDBOX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$UNREAD" \
  bash "$DRIVER" 2>&1) || RC=$?
chmod 644 "$UNREAD"
# Root can read 000 files. Skip the strict failure assertion when running
# as root and just assert no crash.
if [ "$(id -u)" -ne 0 ]; then
  assert_neq "non-zero exit for unreadable decisions file" "0" "$RC"
  assert_contains "stderr names the unreadable case" "$OUTPUT" "not readable"
else
  skip_test "unreadable decisions file" "running as root — chmod 000 unenforced"
fi

echo ""
echo "Test: env-var robustness — empty file is accepted"
SANDBOX=$(setup_sandbox "decisions-empty")
for f in "$PLUGIN_ROOT/skills/config/migrate/migrations"/[0-9][0-9][0-9][0-9]-*.sh; do
  basename "$f" .sh
done > "$SANDBOX/.accelerator/state/migrations-applied"
EMPTY_DECISIONS=$(mktemp "$TMPDIR_BASE/empty-decisions-XXXXXX")
RC=0
OUTPUT=$(PROJECT_ROOT="$SANDBOX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$EMPTY_DECISIONS" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 with empty decisions file on no-pending repo" "0" "$RC"

echo ""
echo "Test: env-var robustness — CRLF line endings accepted (no preflight rejection)"
SANDBOX=$(setup_sandbox "decisions-crlf")
for f in "$PLUGIN_ROOT/skills/config/migrate/migrations"/[0-9][0-9][0-9][0-9]-*.sh; do
  basename "$f" .sh
done > "$SANDBOX/.accelerator/state/migrations-applied"
CRLF=$(mktemp "$TMPDIR_BASE/crlf-XXXXXX")
printf 'accept\r\naccept\r\n' > "$CRLF"
RC=0
OUTPUT=$(PROJECT_ROOT="$SANDBOX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$CRLF" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 with CRLF decisions file on no-pending repo" "0" "$RC"

echo ""
echo "=== Session-log-aware dirty-tree pre-flight ==="
echo ""

# Build a dirty-tree fixture WITHOUT FORCE so the preflight engages.
# Use jj (already detected by the runner) — create a minimal .jj/ that
# triggers the jj-branch of the dirty check but fall back to git if jj
# is unavailable in the test environment.
make_dirty_session_log_repo() {
  local name="$1"
  local sandbox
  sandbox=$(mktemp -d "$TMPDIR_BASE/$name-XXXXXX")
  ( cd "$sandbox" && git init -q && git config user.email t@e.x \
    && git config user.name t && git commit --allow-empty -q -m init )
  mkdir -p "$sandbox/.accelerator/state"
  # Create the session log and git-add it so it appears as a staged change
  # (the runner ignores untracked '??' lines, but committed-but-uncommitted
  # paths are flagged dirty).
  printf '{"transformation_key":"k1","outcome":"accepted"}\n' \
    > "$sandbox/.accelerator/state/migrations-0099-test-session.jsonl"
  printf '{"transformation_key":"k2","outcome":"edited"}\n' \
    >> "$sandbox/.accelerator/state/migrations-0099-test-session.jsonl"
  ( cd "$sandbox" && git add ".accelerator/state/migrations-0099-test-session.jsonl" )
  printf '%s\n' "$sandbox"
}

if command -v git >/dev/null 2>&1; then
  echo "Test: dirty session-log file → distinct, named error"
  REPO=$(make_dirty_session_log_repo "session-dirty")
  RC=0
  OUTPUT=$(PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
    bash "$DRIVER" 2>&1) || RC=$?
  assert_neq "non-zero exit" "0" "$RC"
  assert_contains "names the in-flight session" "$OUTPUT" \
    "in-flight interactive migration"
  assert_contains "names the resume action" "$OUTPUT" "To resume:"
  assert_contains "names the discard action" "$OUTPUT" "To discard:"
  assert_contains "names the affected file" "$OUTPUT" \
    "migrations-0099-test-session.jsonl"
  assert_contains "names the decision count" "$OUTPUT" "2 decisions recorded"
  # The git-backed dirty repo should be advised to run `git status`,
  # not `jj status` (the suggestion is VCS-aware).
  assert_contains "suggests git status for git repo" "$OUTPUT" \
    "run \`git status\`"
  assert_not_contains "does NOT suggest jj status for git repo" "$OUTPUT" \
    "run \`jj status\`"

  if command -v jj >/dev/null 2>&1; then
    echo ""
    echo "Test: dirty session-log file in jj repo → suggests jj status"
    REPO_JJ=$(mktemp -d "$TMPDIR_BASE/session-dirty-jj-XXXXXX")
    ( cd "$REPO_JJ" && jj git init --quiet )
    mkdir -p "$REPO_JJ/.accelerator/state"
    printf '{"transformation_key":"k1","outcome":"accepted"}\n' \
      > "$REPO_JJ/.accelerator/state/migrations-0099-test-session.jsonl"
    RC=0
    # cd into the sandbox: the runner's jj diff runs in cwd (it does
    # not honour --repository / $PROJECT_ROOT — pre-existing behaviour),
    # so we exercise the jj-detection branch by invoking from within
    # the sandbox.
    OUTPUT=$(cd "$REPO_JJ" && PROJECT_ROOT="$REPO_JJ" \
             CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
             bash "$DRIVER" 2>&1) || RC=$?
    assert_neq "non-zero exit" "0" "$RC"
    assert_contains "suggests jj status for jj repo" "$OUTPUT" \
      "run \`jj status\`"
    assert_not_contains "does NOT suggest git status for jj repo" "$OUTPUT" \
      "run \`git status\`"
  else
    skip_test "jj-repo suggestion test" "jj not available"
  fi

  echo ""
  echo "Test: dirty non-session paths → generic uncommitted-changes error"
  REPO=$(mktemp -d "$TMPDIR_BASE/other-dirty-XXXXXX")
  ( cd "$REPO" && git init -q && git config user.email t@e.x \
    && git config user.name t && git commit --allow-empty -q -m init )
  mkdir -p "$REPO/meta"
  echo "hi" > "$REPO/meta/something.md"
  ( cd "$REPO" && git add meta/something.md )
  RC=0
  OUTPUT=$(PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
    bash "$DRIVER" 2>&1) || RC=$?
  assert_neq "non-zero exit" "0" "$RC"
  assert_contains "generic dirty-tree error" "$OUTPUT" \
    "dirty working tree"
  assert_not_contains "no in-flight banner" "$OUTPUT" \
    "in-flight interactive migration"
else
  skip_test "session-log preflight tests" "git not available"
fi

mkdir -p "$MIGRATIONS_DIR_FIXTURE"

echo ""
test_summary
