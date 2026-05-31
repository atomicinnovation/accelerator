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
echo "=== Phase 3: header detection + handshake ==="
echo ""

# Helper: run a migration from a fixture's migrations/ directory inside
# a freshly-prepared sandbox. The sandbox path is exported to the caller
# via a side-channel file (the test wraps each call in $(...) so any
# in-process variable assignment is lost on subshell exit).
INTERACTIVE_FIXTURE_SANDBOX_FILE="$TMPDIR_BASE/.last-sandbox"
run_interactive_fixture() {
  local fixture_name="$1"
  local sandbox
  sandbox=$(setup_sandbox "$fixture_name")
  printf '%s' "$sandbox" > "$INTERACTIVE_FIXTURE_SANDBOX_FILE"
  local mig_dir="$MIGRATIONS_DIR_FIXTURE/$fixture_name/migrations"
  ACCELERATOR_MIGRATIONS_DIR="$mig_dir" \
  PROJECT_ROOT="$sandbox" \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
    bash "$DRIVER"
}
last_sandbox() { cat "$INTERACTIVE_FIXTURE_SANDBOX_FILE"; }

echo "Test: existing mechanical migrations (0001-0006) classified mechanical"
# Source the lib so we can call is_interactive_migration directly.
PLUGIN_ROOT_TEST="$PLUGIN_ROOT" bash -c '
  PROJECT_ROOT=/tmp PLUGIN_ROOT="'"$PLUGIN_ROOT"'" CLAUDE_PLUGIN_ROOT="'"$PLUGIN_ROOT"'"
  source "'"$PLUGIN_ROOT"'/scripts/atomic-common.sh"
  source "'"$PLUGIN_ROOT"'/skills/config/migrate/scripts/interactive-lib.sh"
  fail=0
  for f in "'"$PLUGIN_ROOT"'/skills/config/migrate/migrations"/[0-9][0-9][0-9][0-9]-*.sh; do
    if is_interactive_migration "$f"; then
      echo "ERROR: $f classified as interactive" >&2
      fail=1
    fi
  done
  exit $fail
' && {
  echo "  PASS: 0001-0006 are mechanical"
  PASS=$((PASS + 1))
} || {
  echo "  FAIL: at least one of 0001-0006 misclassified"
  FAIL=$((FAIL + 1))
}

echo ""
echo "Test: empty interactive fixture completes and is appended to migrations-applied"
RC=0
OUTPUT=$(run_interactive_fixture 0001-empty-interactive 2>&1) || RC=$?
SANDBOX=$(last_sandbox)
RC=${RC:-0}
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$SANDBOX/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "ledger contains the migration" "$APPLIED" "0001-empty-interactive"

echo ""
echo "Test: handshake exchange captured in protocol log"
PROTO_RUNNER=$(mktemp "$TMPDIR_BASE/proto-runner-XXXXXX")
PROTO_MIG=$(mktemp "$TMPDIR_BASE/proto-mig-XXXXXX")
RC=0
OUTPUT=$(MIGRATION_PROTOCOL_LOG_RUNNER="$PROTO_RUNNER" \
         MIGRATION_PROTOCOL_LOG_MIGRATION="$PROTO_MIG" \
         run_interactive_fixture 0001-empty-interactive 2>&1) || RC=$?
SANDBOX=$(last_sandbox)
assert_eq "exit 0 with proto logs" "0" "$RC"
RUNNER_LOG=$(cat "$PROTO_RUNNER")
MIG_LOG=$(cat "$PROTO_MIG")
assert_contains "runner log has INIT"   "$RUNNER_LOG" "INIT"
assert_contains "migration log has READY" "$MIG_LOG"  "READY"
assert_contains "migration log has DONE"  "$MIG_LOG"  "DONE"

echo ""
echo "Test: migration that emits FAIL — runner exits non-zero, no ledger append"
mkdir -p "$MIGRATIONS_DIR_FIXTURE/0002-fail-frame/migrations"
cat > "$MIGRATIONS_DIR_FIXTURE/0002-fail-frame/migrations/0002-fail-frame.sh" <<'FAIL_SH'
#!/usr/bin/env bash
# DESCRIPTION: Emit FAIL after READY — Phase 3 negative test.
# INTERACTIVE: yes
set -euo pipefail
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"
migration_emit_transformations() { :; }
migration_evaluate_predicate() { return 0; }
migration_validate_edit() { return 0; }
migration_apply_decision() { return 0; }

# Override harness_run to emit FAIL right after handshake.
harness_run_fail() {
  read_frame
  emit_frame READY ".accelerator/state/migrations-0002-fail-frame-session.jsonl"
  emit_frame FAIL "synthetic failure for testing"
  exit 1
}
harness_run_fail
FAIL_SH
RC=0
OUTPUT=$(run_interactive_fixture 0002-fail-frame 2>&1) || RC=$?
SANDBOX=$(last_sandbox)
assert_neq "non-zero exit" "0" "$RC"
assert_contains "FAIL message surfaced" "$OUTPUT" "synthetic failure for testing"
APPLIED=$(cat "$SANDBOX/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_not_contains "ledger does NOT contain failed migration" "$APPLIED" \
  "0002-fail-frame"

echo ""
echo "Test: pre-handshake MIGRATION_RESULT: no_op_pending is honoured (soft-defer)"
mkdir -p "$MIGRATIONS_DIR_FIXTURE/0003-soft-defer/migrations"
cat > "$MIGRATIONS_DIR_FIXTURE/0003-soft-defer/migrations/0003-soft-defer.sh" <<'NOP_SH'
#!/usr/bin/env bash
# DESCRIPTION: Soft-defer before harness handshake — Phase 3 mechanical-contract test.
# INTERACTIVE: yes
set -euo pipefail
# Emit the sentinel BEFORE sourcing the harness. The runner detects it
# via exact-prefix match on stdout.
printf 'MIGRATION_RESULT: no_op_pending\n'
exit 0
NOP_SH
RC=0
OUTPUT=$(run_interactive_fixture 0003-soft-defer 2>&1) || RC=$?
SANDBOX=$(last_sandbox)
assert_eq "exit 0 on soft-defer" "0" "$RC"
APPLIED=$(cat "$SANDBOX/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_not_contains "ledger does NOT contain soft-deferred migration" \
  "$APPLIED" "0003-soft-defer"
assert_contains "user-facing 'no-op (stays pending)' message" "$OUTPUT" \
  "no-op (stays pending)"

echo ""
test_summary
