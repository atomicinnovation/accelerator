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
done >"$SANDBOX/.accelerator/state/migrations-applied"

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
done >"$SANDBOX/.accelerator/state/migrations-applied"
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
done >"$SANDBOX/.accelerator/state/migrations-applied"
CRLF=$(mktemp "$TMPDIR_BASE/crlf-XXXXXX")
printf 'accept\r\naccept\r\n' >"$CRLF"
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
  (cd "$sandbox" && git init -q && git config user.email t@e.x &&
    git config user.name t && git commit --allow-empty -q -m init)
  mkdir -p "$sandbox/.accelerator/state"
  # Create the session log and git-add it so it appears as a staged change
  # (the runner ignores untracked '??' lines, but committed-but-uncommitted
  # paths are flagged dirty).
  printf '{"transformation_key":"k1","outcome":"accepted"}\n' \
    >"$sandbox/.accelerator/state/migrations-0099-test-session.jsonl"
  printf '{"transformation_key":"k2","outcome":"edited"}\n' \
    >>"$sandbox/.accelerator/state/migrations-0099-test-session.jsonl"
  (cd "$sandbox" && git add ".accelerator/state/migrations-0099-test-session.jsonl")
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
    (cd "$REPO_JJ" && jj git init --quiet)
    mkdir -p "$REPO_JJ/.accelerator/state"
    printf '{"transformation_key":"k1","outcome":"accepted"}\n' \
      >"$REPO_JJ/.accelerator/state/migrations-0099-test-session.jsonl"
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
  (cd "$REPO" && git init -q && git config user.email t@e.x &&
    git config user.name t && git commit --allow-empty -q -m init)
  mkdir -p "$REPO/meta"
  echo "hi" >"$REPO/meta/something.md"
  (cd "$REPO" && git add meta/something.md)
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
  printf '%s' "$sandbox" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
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
assert_contains "runner log has INIT" "$RUNNER_LOG" "INIT"
assert_contains "migration log has READY" "$MIG_LOG" "READY"
assert_contains "migration log has DONE" "$MIG_LOG" "DONE"

echo ""
echo "Test: migration that emits FAIL — runner exits non-zero, no ledger append"
mkdir -p "$MIGRATIONS_DIR_FIXTURE/0002-fail-frame/migrations"
cat >"$MIGRATIONS_DIR_FIXTURE/0002-fail-frame/migrations/0002-fail-frame.sh" <<'FAIL_SH'
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
cat >"$MIGRATIONS_DIR_FIXTURE/0003-soft-defer/migrations/0003-soft-defer.sh" <<'NOP_SH'
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
echo "=== Phase 4: predicate routing + display + accept ==="
echo ""

# Helper: seed a 0002-predicate sandbox with a transformations file.
seed_predicate_sandbox() {
  local sandbox="$1"
  shift
  mkdir -p "$sandbox/.fixture"
  : >"$sandbox/.fixture/transformations"
  local line
  for line in "$@"; do
    printf '%s\n' "$line" >>"$sandbox/.fixture/transformations"
  done
}

# Helper: count occurrences of a frame type in a protocol log.
count_frames() {
  local log="$1" frame_type="$2"
  grep -c "^${frame_type}"$'\t' "$log" 2>/dev/null ||
    grep -c "^${frame_type}$" "$log" 2>/dev/null || echo 0
}

echo "Test: AC-2 — uniform predicate=true (all rows ambiguous → all PROMPT)"
RC=0
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-uniform-XXXXXX")
printf 'accept\naccept\naccept\n' >"$DECISIONS_FILE"
PROTO_RUN=$(mktemp "$TMPDIR_BASE/proto-run-XXXXXX")
PROTO_MIG=$(mktemp "$TMPDIR_BASE/proto-mig-XXXXXX")
# Pre-create the sandbox so we can populate the fixture *before* the
# runner starts. run_interactive_fixture uses setup_sandbox so we can't
# pre-seed via that path; instead we seed inline.
SBX=$(setup_sandbox "uniform")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" \
  "k1|f1|a1|v1|ambiguous|prose1" \
  "k2|f2|a2|v2|ambiguous|prose2" \
  "k3|f3|a3|v3|ambiguous|prose3"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  MIGRATION_PROTOCOL_LOG_RUNNER="$PROTO_RUN" \
  MIGRATION_PROTOCOL_LOG_MIGRATION="$PROTO_MIG" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
PROMPT_COUNT=$(grep -c $'^PROMPT\t' "$PROTO_MIG" || true)
MECH_COUNT=$(grep -c $'^MECHANICAL_APPLIED\t' "$PROTO_MIG" || true)
assert_eq "3 PROMPTs" "3" "$PROMPT_COUNT"
assert_eq "0 MECHANICAL_APPLIED" "0" "$MECH_COUNT"

echo ""
echo "Test: AC-2 — uniform predicate=false (all resolved → all MECHANICAL_APPLIED)"
RC=0
PROTO_MIG=$(mktemp "$TMPDIR_BASE/proto-mig-XXXXXX")
SBX=$(setup_sandbox "uniform-resolved")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" \
  "k1|f1|a1|v1|resolved|prose1" \
  "k2|f2|a2|v2|resolved|prose2"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  MIGRATION_PROTOCOL_LOG_MIGRATION="$PROTO_MIG" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
PROMPT_COUNT=$(grep -c $'^PROMPT\t' "$PROTO_MIG" || true)
MECH_COUNT=$(grep -c $'^MECHANICAL_APPLIED\t' "$PROTO_MIG" || true)
assert_eq "0 PROMPTs" "0" "$PROMPT_COUNT"
assert_eq "2 MECHANICAL_APPLIED" "2" "$MECH_COUNT"

echo ""
echo "Test: AC-2 — mixed (k1 ambiguous, k2 resolved, k3 ambiguous)"
RC=0
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-mixed-XXXXXX")
printf 'accept\naccept\n' >"$DECISIONS_FILE"
PROTO_MIG=$(mktemp "$TMPDIR_BASE/proto-mig-XXXXXX")
SBX=$(setup_sandbox "mixed")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" \
  "k1|f1|a1|v1|ambiguous|p1" \
  "k2|f2|a2|v2|resolved|p2" \
  "k3|f3|a3|v3|ambiguous|p3"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  MIGRATION_PROTOCOL_LOG_MIGRATION="$PROTO_MIG" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
PROMPT_COUNT=$(grep -c $'^PROMPT\t' "$PROTO_MIG" || true)
MECH_COUNT=$(grep -c $'^MECHANICAL_APPLIED\t' "$PROTO_MIG" || true)
assert_eq "2 PROMPTs" "2" "$PROMPT_COUNT"
assert_eq "1 MECHANICAL_APPLIED" "1" "$MECH_COUNT"

echo ""
echo "Test: AC-3 — display elements (proposed, source, predicate) + inline help + session-log banner"
RC=0
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-display-XXXXXX")
printf 'accept\n' >"$DECISIONS_FILE"
SBX=$(setup_sandbox "display")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" \
  "k1|meta/work/0070-X.md|23|0034-foo|ambiguous|the linkage paragraph"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "proposed value rendered" "$OUTPUT" "0034-foo"
assert_contains "source path:anchor rendered" "$OUTPUT" "meta/work/0070-X.md:23"
assert_contains "predicate value rendered" "$OUTPUT" "ambiguous"
assert_contains "inline help on first prompt" "$OUTPUT" \
  "[accept | edit <new-value> | skip]"
assert_contains "session-log banner" "$OUTPUT" \
  "Session log: $SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"

echo ""
echo "Test: AC-4 — declared display extras (prose line) visible"
assert_contains "prose line visible" "$OUTPUT" "Surrounding prose: the linkage paragraph"

echo ""
echo "Test: AC-5 — accept persists records to JSONL with canonical schema"
RC=0
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-accept-XXXXXX")
printf 'accept\naccept\n' >"$DECISIONS_FILE"
SBX=$(setup_sandbox "accept-persists")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" \
  "k1|art1|a1|v1|ambiguous|p1" \
  "k2|art2|a2|v2|ambiguous|p2"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"

LOG="$SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"
assert_file_exists "session log written" "$LOG"
COUNT=$(wc -l <"$LOG" | tr -d ' ')
assert_eq "2 records persisted" "2" "$COUNT"
if command -v python3 >/dev/null 2>&1; then
  BAD=$(
    python3 - "$LOG" <<'PY'
import json, sys
bad = 0
for ln in open(sys.argv[1]):
    ln = ln.rstrip('\n')
    if not ln: continue
    try: json.loads(ln)
    except Exception: bad += 1
print(bad)
PY
  )
  assert_eq "every record is valid JSON" "0" "$BAD"
fi
FIRST=$(head -1 "$LOG")
case "$FIRST" in
  '{"transformation_key":'*)
    echo "  PASS: canonical first field"
    PASS=$((PASS + 1))
    ;;
  *)
    echo "  FAIL: first field is not transformation_key"
    echo "    Got: $FIRST"
    FAIL=$((FAIL + 1))
    ;;
esac
assert_contains "outcome is accepted" "$(cat "$LOG")" '"outcome":"accepted"'
assert_not_contains "no user_value for accepted" "$(cat "$LOG")" '"user_value"'

echo ""
echo "Test: artifacts mutated for accepted keys"
assert_file_exists "art1 mutated" "$SBX/art1"
assert_file_exists "art2 mutated" "$SBX/art2"
assert_contains "art1 content" "$(cat "$SBX/art1")" "a1=v1"
assert_contains "art2 content" "$(cat "$SBX/art2")" "a2=v2"

echo ""
echo "Test: write-ahead-log ordering — APPLY appears after RECORDED in protocol log"
RC=0
PROTO_RUN_F=$(mktemp "$TMPDIR_BASE/proto-run-wal-XXXXXX")
PROTO_MIG_F=$(mktemp "$TMPDIR_BASE/proto-mig-wal-XXXXXX")
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-wal-XXXXXX")
printf 'accept\n' >"$DECISIONS_FILE"
SBX=$(setup_sandbox "wal-ordering")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" "k1|f|a|v|ambiguous|p"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  MIGRATION_PROTOCOL_LOG_RUNNER="$PROTO_RUN_F" \
  MIGRATION_PROTOCOL_LOG_MIGRATION="$PROTO_MIG_F" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
# In the migration-side log: RECORDED must appear before APPLIED_CONFIRM,
# and the runner's APPLY must appear in the runner log AFTER the
# migration emitted RECORDED.
RECORDED_LINE=$(grep -n $'^RECORDED\t' "$PROTO_MIG_F" | head -1 | cut -d: -f1)
APPLIEDCONF_LINE=$(grep -n $'^APPLIED_CONFIRM\t' "$PROTO_MIG_F" | head -1 | cut -d: -f1)
if [ -n "$RECORDED_LINE" ] && [ -n "$APPLIEDCONF_LINE" ] &&
  [ "$RECORDED_LINE" -lt "$APPLIEDCONF_LINE" ]; then
  echo "  PASS: RECORDED before APPLIED_CONFIRM"
  PASS=$((PASS + 1))
else
  echo "  FAIL: RECORDED/APPLIED_CONFIRM order wrong"
  FAIL=$((FAIL + 1))
fi
APPLY_LINE=$(grep -n $'^APPLY\t' "$PROTO_RUN_F" | head -1 | cut -d: -f1)
if [ -n "$APPLY_LINE" ]; then
  echo "  PASS: APPLY frame present in runner log"
  PASS=$((PASS + 1))
else
  echo "  FAIL: no APPLY frame in runner log"
  FAIL=$((FAIL + 1))
fi

echo ""
echo "=== Phase 5: edit, skip, validation re-prompt ==="
echo ""

echo "Test: AC-6 — edit verb persists outcome=edited with user_value"
RC=0
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-edit-XXXXXX")
printf 'edit corrected-value\n' >"$DECISIONS_FILE"
SBX=$(setup_sandbox "edit")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" "k1|f|a|original-value|ambiguous|p"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
LOG="$SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"
assert_contains "outcome edited" "$(cat "$LOG")" '"outcome":"edited"'
assert_contains "user_value present" "$(cat "$LOG")" '"user_value":"corrected-value"'
assert_contains "proposed_value retained" "$(cat "$LOG")" '"proposed_value":"original-value"'
assert_file_exists "artifact written" "$SBX/f"
assert_contains "artifact has user value" "$(cat "$SBX/f")" "a=corrected-value"
assert_not_contains "artifact does NOT have original proposed" \
  "$(cat "$SBX/f")" "a=original-value"

echo ""
echo "Test: AC-7 — skip does not mutate artifact and records outcome=skipped"
RC=0
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-skip-XXXXXX")
printf 'skip\n' >"$DECISIONS_FILE"
SBX=$(setup_sandbox "skip")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" "k1|target|a|v1|ambiguous|p"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
LOG="$SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"
assert_file_exists "session log written" "$LOG"
assert_contains "outcome skipped" "$(cat "$LOG")" '"outcome":"skipped"'
assert_not_contains "no user_value for skipped" "$(cat "$LOG")" '"user_value"'
assert_file_not_exists "target artifact NOT written" "$SBX/target"
# migration_apply_decision must not have been called — verified via the
# fixture's sentinel log (which is only written if apply ran).
if [ ! -f "$SBX/.fixture/applied/log" ]; then
  echo "  PASS: migration_apply_decision NOT called for skip"
  PASS=$((PASS + 1))
else
  echo "  FAIL: migration_apply_decision was called for skip"
  echo "    Sentinel content: $(cat "$SBX/.fixture/applied/log")"
  FAIL=$((FAIL + 1))
fi

echo ""
echo "Test: AC-8 — validation re-prompt loop (empty edit → error, then valid edit succeeds)"
RC=0
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-revalidate-XXXXXX")
printf 'edit \nedit recovered\n' >"$DECISIONS_FILE"
SBX=$(setup_sandbox "revalidate")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" "k1|artifact|a|original|ambiguous|p"
PROTO_MIG_F=$(mktemp "$TMPDIR_BASE/proto-mig-revalidate-XXXXXX")
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  MIGRATION_PROTOCOL_LOG_MIGRATION="$PROTO_MIG_F" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
LOG="$SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"
COUNT=$(wc -l <"$LOG" | tr -d ' ')
assert_eq "exactly one record persisted" "1" "$COUNT"
assert_contains "final outcome edited" "$(cat "$LOG")" '"outcome":"edited"'
assert_contains "final user_value=recovered" "$(cat "$LOG")" '"user_value":"recovered"'
# Validate_err must have been emitted exactly once.
VE_COUNT=$(grep -c $'^VALIDATE_ERR\t' "$PROTO_MIG_F" || true)
assert_eq "VALIDATE_ERR emitted exactly once" "1" "$VE_COUNT"
# The user-facing output should contain the validator's reject message.
assert_contains "validator message surfaced" "$OUTPUT" \
  "empty value not allowed"
# The inline-help line should reappear on the re-prompt (frequency rule).
HELP_COUNT=$(grep -c "accept | edit <new-value> | skip" <<<"$OUTPUT" || true)
if [ "$HELP_COUNT" -ge 2 ]; then
  echo "  PASS: inline help re-rendered after VALIDATE_ERR"
  PASS=$((PASS + 1))
else
  echo "  FAIL: inline help not re-rendered (count=$HELP_COUNT)"
  FAIL=$((FAIL + 1))
fi

echo ""
echo "Test: mid-stream FAIL — partial session log, no ledger append"
mkdir -p "$MIGRATIONS_DIR_FIXTURE/0004-midstream-fail/migrations"
cat >"$MIGRATIONS_DIR_FIXTURE/0004-midstream-fail/migrations/0004-midstream-fail.sh" <<'MID_FAIL_SH'
#!/usr/bin/env bash
# DESCRIPTION: Fail mid-stream — Phase 5 negative test.
# INTERACTIVE: yes
set -euo pipefail
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"

migration_emit_transformations() {
  harness_emit_transformation key=k1 path=p1 anchor=a proposed=v1 \
    predicate_value=ambiguous display="x"
  harness_emit_transformation key=k2 path=p2 anchor=a proposed=v2 \
    predicate_value=ambiguous display="x"
  harness_emit_transformation key=k3 path=p3 anchor=a proposed=v3 \
    predicate_value=ambiguous display="x"
}
migration_evaluate_predicate() { return 0; }
migration_validate_edit() { return 0; }

migration_apply_decision() {
  local key="$1"
  if [ "$key" = "k3" ]; then
    harness_reject "synthetic apply failure on k3"
    return 1
  fi
  return 0
}

harness_run
MID_FAIL_SH
RC=0
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-midfail-XXXXXX")
printf 'accept\naccept\naccept\n' >"$DECISIONS_FILE"
SBX=$(setup_sandbox "midfail")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0004-midstream-fail/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit" "0" "$RC"
APPLIED=$(cat "$SBX/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_not_contains "ledger does NOT contain mid-failed migration" \
  "$APPLIED" "0004-midstream-fail"
LOG="$SBX/.accelerator/state/migrations-0004-midstream-fail-session.jsonl"
assert_file_exists "session log present (partial)" "$LOG"
COUNT=$(wc -l <"$LOG" | tr -d ' ')
# Per the write-ahead-log invariant: the runner persists the RECORDED
# JSONL line BEFORE emitting APPLY (and thus before the harness calls
# migration_apply_decision). When k3's apply fails, k3's record is
# already durable. So 3 records is correct — the residual risk (k3
# treated as applied on resume even though mutation failed) is the
# documented bounded failure mode.
assert_eq "all 3 records persisted (WAL invariant)" "3" "$COUNT"

echo ""
echo "=== Phase 6: resumability + source-drift ==="
echo ""

echo "Test: AC-10 — partial-run resume skips already-decided keys"
RC=0
SBX=$(setup_sandbox "resume-partial")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" \
  "k1|f1|a|v1|ambiguous|p" \
  "k2|f2|a|v2|ambiguous|p" \
  "k3|f3|a|v3|ambiguous|p" \
  "k4|f4|a|v4|ambiguous|p" \
  "k5|f5|a|v5|ambiguous|p"
# Pre-create a session log: k1 accepted, k2 edited, k3 skipped.
mkdir -p "$SBX/.accelerator/state"
LOG="$SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"
cat >"$LOG" <<'EOF'
{"transformation_key":"k1","schema_version":1,"outcome":"accepted","proposed_value":"v1","timestamp":"2026-05-30T12:00:00Z","band":"ambiguous","prose":"p"}
{"transformation_key":"k2","schema_version":1,"outcome":"edited","proposed_value":"v2","user_value":"custom","timestamp":"2026-05-30T12:00:00Z","band":"ambiguous","prose":"p"}
{"transformation_key":"k3","schema_version":1,"outcome":"skipped","proposed_value":"v3","timestamp":"2026-05-30T12:00:00Z","band":"ambiguous","prose":"p"}
EOF
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-resume-XXXXXX")
printf 'accept\naccept\n' >"$DECISIONS_FILE"
PROTO_MIG_F=$(mktemp "$TMPDIR_BASE/proto-mig-resume-XXXXXX")
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  MIGRATION_PROTOCOL_LOG_MIGRATION="$PROTO_MIG_F" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
RESUMED_APPLIED_COUNT=$(grep -c $'^RESUMED_APPLIED\t' "$PROTO_MIG_F" || true)
RESUMED_SKIPPED_COUNT=$(grep -c $'^RESUMED_SKIPPED\t' "$PROTO_MIG_F" || true)
PROMPT_COUNT=$(grep -c $'^PROMPT\t' "$PROTO_MIG_F" || true)
assert_eq "2 RESUMED_APPLIED (k1+k2)" "2" "$RESUMED_APPLIED_COUNT"
assert_eq "1 RESUMED_SKIPPED (k3)" "1" "$RESUMED_SKIPPED_COUNT"
assert_eq "2 PROMPTs (k4+k5)" "2" "$PROMPT_COUNT"
# Final log: 5 records total.
FINAL_COUNT=$(wc -l <"$LOG" | tr -d ' ')
assert_eq "5 records in final log" "5" "$FINAL_COUNT"

echo ""
echo "Test: AC-11 — full-run idempotency (re-run with empty decisions)"
RC=0
# Second run with NO decisions — should resume everything and emit 0 PROMPTs.
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-empty-XXXXXX")
: >"$DECISIONS_FILE"
PROTO_MIG_F=$(mktemp "$TMPDIR_BASE/proto-mig-idem-XXXXXX")
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  MIGRATION_PROTOCOL_LOG_MIGRATION="$PROTO_MIG_F" \
  bash "$DRIVER" 2>&1) || RC=$?
# Already-applied migration: runner sees no pending, exits.
assert_eq "second run exit 0" "0" "$RC"
APPLIED=$(cat "$SBX/.accelerator/state/migrations-applied")
LEDGER_COUNT=$(grep -c "^0002-predicate$" <<<"$APPLIED" || true)
assert_eq "ledger contains migration exactly once" "1" "$LEDGER_COUNT"

echo ""
echo "Test: AC-12 — source-drift detected, stale record removed, re-prompt"
RC=0
SBX=$(setup_sandbox "drift")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" "k1|f1|a|v_new|ambiguous|p"
# Pre-create a session log marking k1 as accepted but with proposed=v_old
# (differs from the live emission's v_new).
mkdir -p "$SBX/.accelerator/state"
LOG="$SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"
cat >"$LOG" <<'EOF'
{"transformation_key":"k1","schema_version":1,"outcome":"accepted","proposed_value":"v_old","timestamp":"2026-05-30T12:00:00Z","band":"ambiguous","prose":"p"}
EOF
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-drift-XXXXXX")
printf 'accept\n' >"$DECISIONS_FILE"
PROTO_MIG_F=$(mktemp "$TMPDIR_BASE/proto-mig-drift-XXXXXX")
PROTO_RUN_F=$(mktemp "$TMPDIR_BASE/proto-run-drift-XXXXXX")
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  MIGRATION_PROTOCOL_LOG_MIGRATION="$PROTO_MIG_F" \
  MIGRATION_PROTOCOL_LOG_RUNNER="$PROTO_RUN_F" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
DRIFT_COUNT=$(grep -c $'^DRIFT\t' "$PROTO_MIG_F" || true)
CLEARED_COUNT=$(grep -c $'^DRIFT_CLEARED\t' "$PROTO_RUN_F" || true)
PROMPT_COUNT=$(grep -c $'^PROMPT\t' "$PROTO_MIG_F" || true)
assert_eq "1 DRIFT emitted" "1" "$DRIFT_COUNT"
assert_eq "1 DRIFT_CLEARED emitted" "1" "$CLEARED_COUNT"
assert_eq "1 PROMPT (after drift clear)" "1" "$PROMPT_COUNT"
# Final log: v_old record gone, v_new record present.
LOG_CONTENT=$(cat "$LOG")
assert_not_contains "v_old removed" "$LOG_CONTENT" '"proposed_value":"v_old"'
assert_contains "v_new persisted" "$LOG_CONTENT" '"proposed_value":"v_new"'

echo ""
echo "Test: resume — unknown outcome in session log → fail-fast"
RC=0
SBX=$(setup_sandbox "bad-outcome")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" "k1|f|a|v|ambiguous|p"
mkdir -p "$SBX/.accelerator/state"
LOG="$SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"
cat >"$LOG" <<'EOF'
{"transformation_key":"k1","schema_version":1,"outcome":"banana","proposed_value":"v","timestamp":"2026-05-30T12:00:00Z"}
EOF
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit on unknown outcome" "0" "$RC"
assert_contains "error mentions unknown outcome" "$OUTPUT" "unknown outcome"

echo ""
echo "Test: resume — unknown schema_version → fail-fast with discard hint"
RC=0
SBX=$(setup_sandbox "bad-schema")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" "k1|f|a|v|ambiguous|p"
mkdir -p "$SBX/.accelerator/state"
LOG="$SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"
cat >"$LOG" <<'EOF'
{"transformation_key":"k1","schema_version":99,"outcome":"accepted","proposed_value":"v","timestamp":"2026-05-30T12:00:00Z"}
EOF
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit on unknown schema_version" "0" "$RC"
assert_contains "error mentions unknown schema_version" "$OUTPUT" \
  "unknown schema_version"
assert_contains "error suggests rm of session log" "$OUTPUT" \
  "rm $LOG"

echo ""
echo "Test: resume — user_value with TSV/JSON-escape-significant characters round-trips"
RC=0
SBX=$(setup_sandbox "user-value-escapes")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" "k1|f|a|v|ambiguous|p"
mkdir -p "$SBX/.accelerator/state"
LOG="$SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"
# Edited record with user_value containing tab/newline/backslash/quote.
cat >"$LOG" <<'EOF'
{"transformation_key":"k1","schema_version":1,"outcome":"edited","proposed_value":"v","user_value":"weird \"quote\" and\\backslash and\ttab","timestamp":"2026-05-30T12:00:00Z"}
EOF
PROTO_MIG_F=$(mktemp "$TMPDIR_BASE/proto-mig-escape-XXXXXX")
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  MIGRATION_PROTOCOL_LOG_MIGRATION="$PROTO_MIG_F" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
RESUMED_COUNT=$(grep -c $'^RESUMED_APPLIED\t' "$PROTO_MIG_F" || true)
PROMPT_COUNT=$(grep -c $'^PROMPT\t' "$PROTO_MIG_F" || true)
assert_eq "1 RESUMED_APPLIED" "1" "$RESUMED_COUNT"
assert_eq "0 PROMPTs (no drift)" "0" "$PROMPT_COUNT"

echo ""
echo "Test: orphan resume record (key no longer emitted) is preserved + completes"
RC=0
SBX=$(setup_sandbox "orphan-record")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
# Migration emits only k1; session log has a record for k_orphan.
seed_predicate_sandbox "$SBX" "k1|f|a|v|ambiguous|p"
mkdir -p "$SBX/.accelerator/state"
LOG="$SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"
cat >"$LOG" <<'EOF'
{"transformation_key":"k_orphan","schema_version":1,"outcome":"accepted","proposed_value":"v","timestamp":"2026-05-30T12:00:00Z"}
EOF
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-orphan-XXXXXX")
printf 'accept\n' >"$DECISIONS_FILE"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 (orphan does not block)" "0" "$RC"
# Orphan record should still be in the log (we don't garbage-collect).
assert_contains "k_orphan record preserved" "$(cat "$LOG")" '"transformation_key":"k_orphan"'
assert_contains "k1 record added" "$(cat "$LOG")" '"transformation_key":"k1"'

echo ""
echo "Test: migration_verify_applied — detects missing mutation, triggers re-prompt"
mkdir -p "$MIGRATIONS_DIR_FIXTURE/0005-verify-applied/migrations"
cat >"$MIGRATIONS_DIR_FIXTURE/0005-verify-applied/migrations/0005-verify-applied.sh" <<'VERIFY_SH'
#!/usr/bin/env bash
# DESCRIPTION: Resume-integrity check fixture — Phase 6.
# INTERACTIVE: yes
set -euo pipefail
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"

migration_emit_transformations() {
  harness_emit_transformation key=k1 path=marker anchor=a proposed=v \
    predicate_value=ambiguous display="x"
}
migration_evaluate_predicate() { return 0; }
migration_validate_edit() { return 0; }

migration_apply_decision() {
  printf 'mutated\n' > "$PROJECT_ROOT/marker"
}

# Verifies the mutation actually landed. Returns non-zero if marker
# file is missing or empty.
migration_verify_applied() {
  [ -s "$PROJECT_ROOT/marker" ]
}

harness_run
VERIFY_SH
RC=0
SBX=$(setup_sandbox "verify-applied")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
# Pre-create the resume record but do NOT create the marker file —
# simulates crash between record-persist and mutation.
mkdir -p "$SBX/.accelerator/state"
LOG="$SBX/.accelerator/state/migrations-0005-verify-applied-session.jsonl"
cat >"$LOG" <<'EOF'
{"transformation_key":"k1","schema_version":1,"outcome":"accepted","proposed_value":"v","timestamp":"2026-05-30T12:00:00Z"}
EOF
DECISIONS_FILE=$(mktemp "$TMPDIR_BASE/dec-verify-XXXXXX")
printf 'accept\n' >"$DECISIONS_FILE"
PROTO_MIG_F=$(mktemp "$TMPDIR_BASE/proto-mig-verify-XXXXXX")
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0005-verify-applied/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  ACCELERATOR_MIGRATE_DECISIONS_FILE="$DECISIONS_FILE" \
  MIGRATION_PROTOCOL_LOG_MIGRATION="$PROTO_MIG_F" \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
DRIFT_COUNT=$(grep -c $'^DRIFT\t' "$PROTO_MIG_F" || true)
PROMPT_COUNT=$(grep -c $'^PROMPT\t' "$PROTO_MIG_F" || true)
RESUMED_COUNT=$(grep -c $'^RESUMED_APPLIED\t' "$PROTO_MIG_F" || true)
assert_eq "1 DRIFT (mutation absent)" "1" "$DRIFT_COUNT"
assert_eq "1 PROMPT (after drift)" "1" "$PROMPT_COUNT"
assert_eq "0 RESUMED_APPLIED" "0" "$RESUMED_COUNT"
assert_file_exists "marker mutated" "$SBX/marker"

echo ""
echo "=== Phase 7: doc-example drift test (AC-13) ==="
echo ""

# Extract the worked-example transcript and session-log excerpt from
# SKILL.md, drive the doc-example fixture with the scripted decisions
# that produced them, and diff the redacted captures against the
# extracted regions. Catches doc-vs-implementation drift in CI.

SKILL_MD="$PLUGIN_ROOT/skills/config/migrate/SKILL.md"

extract_block() {
  local start="$1" end="$2" file="$3"
  awk -v s="$start" -v e="$end" '
    $0 ~ s {capture=1; next}
    $0 ~ e {capture=0; next}
    capture
  ' "$file"
}

TRANSCRIPT_DOC=$(extract_block 'transcript-start' 'transcript-end' "$SKILL_MD")
SESSION_LOG_DOC=$(extract_block 'session-log-start' 'session-log-end' "$SKILL_MD")

# Pre-assertion: extracted regions must be non-empty and contain a
# sentinel substring (guards against marker-typo silent passes).
if [ -z "$TRANSCRIPT_DOC" ]; then
  echo "  FAIL: SKILL.md transcript region empty (missing @transcript-start / @transcript-end markers)"
  FAIL=$((FAIL + 1))
elif ! grep -qF "Proposed:" <<<"$TRANSCRIPT_DOC"; then
  echo "  FAIL: SKILL.md transcript region missing 'Proposed:' sentinel"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: transcript region non-empty and sentinel present"
  PASS=$((PASS + 1))
fi
if [ -z "$SESSION_LOG_DOC" ]; then
  echo "  FAIL: SKILL.md session-log region empty"
  FAIL=$((FAIL + 1))
elif ! grep -qF '"transformation_key"' <<<"$SESSION_LOG_DOC"; then
  echo "  FAIL: SKILL.md session-log region missing '\"transformation_key\"' sentinel"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: session-log region non-empty and sentinel present"
  PASS=$((PASS + 1))
fi

# Strip markdown code fences from the extracted regions.
strip_fence() {
  awk '/^```/ {capture = !capture; next} capture' <<<"$1"
}
TRANSCRIPT_DOC_RAW=$(strip_fence "$TRANSCRIPT_DOC")
SESSION_LOG_DOC_RAW=$(strip_fence "$SESSION_LOG_DOC")

# Drive the fixture in a fresh sandbox with the documented decisions
# and capture user-facing output + on-disk session log.
run_doc_example() {
  local sandbox="$1"
  mkdir -p "$sandbox/.git" "$sandbox/.accelerator/state"
  local dec
  dec=$(mktemp "$TMPDIR_BASE/doc-decisions-XXXXXX")
  printf 'edit \nedit 0123-renamed\nskip\n' >"$dec"
  ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/doc-example/migrations" \
    PROJECT_ROOT="$sandbox" \
    CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
    ACCELERATOR_MIGRATE_FORCE=1 \
    ACCELERATOR_MIGRATE_DECISIONS_FILE="$dec" \
    bash "$DRIVER"
}

# Redaction: replace volatile bytes with stable placeholders.
redact() {
  local sandbox="$1"
  sed \
    -e "s|$sandbox|<SANDBOX>|g" \
    -e 's/"timestamp":"[^"]*"/"timestamp":"<REDACTED>"/g' \
    -e 's|/var/folders/[^/]*/[^/]*/T/[^/[:space:]]*|<TMPDIR>|g' \
    -e 's|/tmp/[^/[:space:]]*|<TMPDIR>|g'
}

# Strip noisy lines and inline-`[decisions] consumed line N: VERB`
# annotations that aren't part of the user-facing transcript.
strip_runner_noise() {
  awk '
    /^About to apply/ {next}
    /^Migrations rewrite/ {next}
    /^your working tree/ {next}
    /^rollback/ {next}
    /^unless ACCELERATOR_MIGRATE_FORCE/ {next}
    /^  0099-doc-example/ {next}
    /^    To skip:/ {next}
    /^\[0099-doc-example\] running/ {next}
    /^\[0099-doc-example\] applied/ {next}
    /^Migration complete\./ {next}
    /^\[decisions\] consumed line/ {next}
    {print}
  ' | sed -E 's/ \[decisions\] consumed line [0-9]+: [a-z]+$//' |
    awk 'NF || prev {print; prev=NF}'
}

# AC-13 determinism gate: run 5 times in fresh sandboxes, assert all
# captures are byte-identical post-redaction-and-noise-strip.
PREV_TRANSCRIPT=""
PREV_LOG=""
DETERMINISTIC=1
for run in 1 2 3 4 5; do
  SBX=$(mktemp -d "$TMPDIR_BASE/doc-run$run-XXXXXX")
  OUT=$(run_doc_example "$SBX" 2>&1)
  LOG=$(cat "$SBX/.accelerator/state/migrations-0099-doc-example-session.jsonl")
  OUT_CLEAN=$(redact "$SBX" <<<"$OUT" | strip_runner_noise)
  LOG_CLEAN=$(redact "$SBX" <<<"$LOG")
  if [ "$run" -gt 1 ]; then
    if [ "$OUT_CLEAN" != "$PREV_TRANSCRIPT" ] || [ "$LOG_CLEAN" != "$PREV_LOG" ]; then
      DETERMINISTIC=0
    fi
  fi
  PREV_TRANSCRIPT="$OUT_CLEAN"
  PREV_LOG="$LOG_CLEAN"
  rm -rf "$SBX"
done
assert_eq "5 runs are byte-identical post-redaction" "1" "$DETERMINISTIC"

# Final comparison against doc-extracted regions.
diff_transcript=$(diff <(printf '%s\n' "$TRANSCRIPT_DOC_RAW") <(printf '%s\n' "$PREV_TRANSCRIPT") || true)
diff_log=$(diff <(printf '%s\n' "$SESSION_LOG_DOC_RAW") <(printf '%s\n' "$PREV_LOG") || true)

if [ -z "$diff_transcript" ]; then
  echo "  PASS: transcript matches SKILL.md byte-for-byte"
  PASS=$((PASS + 1))
else
  echo "  FAIL: transcript drift between SKILL.md and live fixture"
  echo "$diff_transcript" | head -40 | sed 's/^/    /'
  FAIL=$((FAIL + 1))
fi

if [ -z "$diff_log" ]; then
  echo "  PASS: session log matches SKILL.md byte-for-byte"
  PASS=$((PASS + 1))
else
  echo "  FAIL: session log drift between SKILL.md and live fixture"
  echo "$diff_log" | head -40 | sed 's/^/    /'
  FAIL=$((FAIL + 1))
fi

echo ""
test_summary
