#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-common.sh
# Run: bash skills/integrations/jira/scripts/test-jira-common.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
JIRA_COMMON="$SCRIPT_DIR/jira-common.sh"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"


TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$repo_dir/.git"
  mkdir -p "$repo_dir/.claude"
  echo "$repo_dir"
}

# ---------------------------------------------------------------------------
echo "=== jira_repo_root (via find_repo_root) ==="
echo ""

echo "Test: find_repo_root locates .git-marked directory"
REPO=$(setup_repo)
RESULT=$(cd "$REPO" && source "$JIRA_COMMON" && find_repo_root)
assert_eq "find_repo_root returns test repo" "$REPO" "$RESULT"

echo ""

# ---------------------------------------------------------------------------
echo "=== jira_state_dir ==="
echo ""

echo "Test: jira_state_dir returns <repo_root>/meta/integrations/jira"
REPO=$(setup_repo)
STATE_DIR=$(cd "$REPO" && source "$JIRA_COMMON" && jira_state_dir)
assert_eq "state dir under repo root" "$REPO/meta/integrations/jira" "$STATE_DIR"

echo "Test: jira_state_dir creates the directory if missing"
REPO=$(setup_repo)
cd "$REPO" && source "$JIRA_COMMON" && jira_state_dir >/dev/null
assert_file_exists "state dir created" "$REPO/meta/integrations/jira"

echo "Test: jira_state_dir respects paths.integrations override"
REPO=$(setup_repo)
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  integrations: .state/integrations
---
FIXTURE
STATE_DIR=$(cd "$REPO" && source "$JIRA_COMMON" && jira_state_dir)
assert_eq "custom integrations path used" "$REPO/.state/integrations/jira" "$STATE_DIR"

echo ""

# ---------------------------------------------------------------------------
echo "=== jira_die / jira_warn ==="
echo ""

echo "Test: jira_die writes message to stderr and exits non-zero"
REPO=$(setup_repo)
STDERR=$(cd "$REPO" && source "$JIRA_COMMON" && jira_die "E_TEST: something failed" 2>&1) || true
assert_contains "stderr contains message" "E_TEST: something failed" "$STDERR"

echo "Test: jira_die exits non-zero"
REPO=$(setup_repo)
EXIT_CODE=0
(cd "$REPO" && source "$JIRA_COMMON" && jira_die "msg") 2>/dev/null || EXIT_CODE=$?
if [ "$EXIT_CODE" -ne 0 ]; then
  echo "  PASS: jira_die exits non-zero"
  PASS=$((PASS + 1))
else
  echo "  FAIL: jira_die should exit non-zero"
  FAIL=$((FAIL + 1))
fi

echo "Test: jira_warn writes Warning: prefix to stderr"
REPO=$(setup_repo)
STDERR=$(cd "$REPO" && source "$JIRA_COMMON" && jira_warn "something notable" 2>&1)
assert_contains "stderr contains Warning: prefix" "Warning: something notable" "$STDERR"

echo "Test: jira_warn does not exit"
REPO=$(setup_repo)
RESULT=$(cd "$REPO" && source "$JIRA_COMMON" && jira_warn "ok" 2>/dev/null && echo "continued")
assert_eq "execution continues after jira_warn" "continued" "$RESULT"

echo ""

# ---------------------------------------------------------------------------
echo "=== jira_jq_field ==="
echo ""

echo "Test: extracts a present field"
REPO=$(setup_repo)
RESULT=$(cd "$REPO" && source "$JIRA_COMMON" && jira_jq_field '{"name":"atlas"}' '.name')
assert_eq "field extracted" "atlas" "$RESULT"

echo "Test: returns empty for missing path"
REPO=$(setup_repo)
RESULT=$(cd "$REPO" && source "$JIRA_COMMON" && jira_jq_field '{"name":"atlas"}' '.missing')
assert_eq "empty for missing" "" "$RESULT"

echo "Test: nested field extraction"
REPO=$(setup_repo)
RESULT=$(cd "$REPO" && source "$JIRA_COMMON" && jira_jq_field '{"a":{"b":"deep"}}' '.a.b')
assert_eq "nested field" "deep" "$RESULT"

echo ""

# ---------------------------------------------------------------------------
echo "=== jira_atomic_write_json ==="
echo ""

echo "Test: valid JSON written to target"
REPO=$(setup_repo)
TARGET="$REPO/out.json"
(cd "$REPO" && source "$JIRA_COMMON" && printf '{"ok":1}' | jira_atomic_write_json "$TARGET")
CONTENT=$(cat "$TARGET")
assert_eq "valid JSON written" '{"ok":1}' "$CONTENT"

echo "Test: invalid JSON exits non-zero with E_BAD_JSON on stderr"
REPO=$(setup_repo)
TARGET="$REPO/out.json"
EXIT_CODE=0
STDERR=$(cd "$REPO" && source "$JIRA_COMMON" && printf 'not json' | jira_atomic_write_json "$TARGET" 2>&1) || EXIT_CODE=$?
if [ "$EXIT_CODE" -ne 0 ]; then
  echo "  PASS: exits non-zero on invalid JSON"
  PASS=$((PASS + 1))
else
  echo "  FAIL: should exit non-zero on invalid JSON"
  FAIL=$((FAIL + 1))
fi
assert_contains "E_BAD_JSON on stderr" "E_BAD_JSON" "$STDERR"

echo "Test: invalid JSON leaves existing target unchanged"
REPO=$(setup_repo)
TARGET="$REPO/out.json"
printf '{"original":1}' > "$TARGET"
(cd "$REPO" && source "$JIRA_COMMON" && printf 'bad' | jira_atomic_write_json "$TARGET" 2>/dev/null) || true
CONTENT=$(cat "$TARGET")
assert_eq "original file intact after bad JSON" '{"original":1}' "$CONTENT"

echo "Test: concurrent writers — final content is exactly one write (atomicity)"
REPO=$(setup_repo)
TARGET="$REPO/concurrent.json"
mkdir -p "$(dirname "$TARGET")"
_writer_1() { sleep 0.15; printf '{"writer":1}' | jira_atomic_write_json "$TARGET"; }
_writer_2() { sleep 0.05; printf '{"writer":2}' | jira_atomic_write_json "$TARGET"; }
(cd "$REPO" && source "$JIRA_COMMON" && _writer_1() { sleep 0.15; printf '{"writer":1}' | jira_atomic_write_json "$TARGET"; }; jira_state_dir >/dev/null; _writer_1) &
PID1=$!
(cd "$REPO" && source "$JIRA_COMMON" && _writer_2() { printf '{"writer":2}' | jira_atomic_write_json "$TARGET"; }; jira_state_dir >/dev/null; _writer_2) &
PID2=$!
wait "$PID1" "$PID2"
CONTENT=$(cat "$TARGET" 2>/dev/null || echo "")
if [ "$CONTENT" = '{"writer":1}' ] || [ "$CONTENT" = '{"writer":2}' ]; then
  echo "  PASS: concurrent write is atomic (content is exactly one write)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: concurrent write produced unexpected content: $CONTENT"
  FAIL=$((FAIL + 1))
fi
# No orphaned temp files
ORPHANS=$(find "$(dirname "$TARGET")" -name '.atomic-write.*' 2>/dev/null | wc -l | tr -d ' ')
assert_eq "no orphaned tmp files after concurrent writes" "0" "$ORPHANS"

echo "Test: interrupted writer leaves no tmp file (SIGTERM — trap fires)"
REPO=$(setup_repo)
TARGET="$REPO/interrupted.json"
mkdir -p "$(dirname "$TARGET")"
# Start a writer that pauses long enough to kill
(cd "$REPO" && source "$JIRA_COMMON" && (printf '{"x":1}' | atomic_write "$TARGET" &); BGPID=$!; sleep 0.05; kill -TERM "$BGPID" 2>/dev/null || true; wait "$BGPID" 2>/dev/null || true) 2>/dev/null || true
sleep 0.1
ORPHANS=$(find "$(dirname "$TARGET")" -name '.atomic-write.*' 2>/dev/null | wc -l | tr -d ' ')
assert_eq "no orphaned tmp file after SIGTERM" "0" "$ORPHANS"

echo "Test: unwritable directory causes non-zero exit"
REPO=$(setup_repo)
RO_DIR="$REPO/readonly"
mkdir -p "$RO_DIR"
chmod 555 "$RO_DIR"
TARGET="$RO_DIR/out.json"
EXIT_CODE=0
(cd "$REPO" && source "$JIRA_COMMON" && printf '{"ok":1}' | jira_atomic_write_json "$TARGET" 2>/dev/null) || EXIT_CODE=$?
chmod 755 "$RO_DIR"
if [ "$EXIT_CODE" -ne 0 ]; then
  echo "  PASS: unwritable directory exits non-zero"
  PASS=$((PASS + 1))
else
  echo "  FAIL: should exit non-zero for unwritable directory"
  FAIL=$((FAIL + 1))
fi
ORPHANS=$(find "$RO_DIR" -name '.atomic-write.*' 2>/dev/null | wc -l | tr -d ' ')
assert_eq "no partial tmp file in unwritable dir" "0" "$ORPHANS"

echo ""

# ---------------------------------------------------------------------------
echo "=== jira_with_lock ==="
echo ""

echo "Test: (a) live-holder serialisation — two writers serialise"
REPO=$(setup_repo)
OUTPUT_FILE="$REPO/lock-output.txt"
rm -f "$OUTPUT_FILE"
(
  cd "$REPO" && source "$JIRA_COMMON"
  _hold_write() {
    sleep 0.2
    echo "writer-1" >> "$OUTPUT_FILE"
  }
  ACCELERATOR_TEST_MODE=1 JIRA_LOCK_TIMEOUT_SECS=10 JIRA_LOCK_SLEEP_SECS=0.05 \
    jira_with_lock _hold_write
) &
PID1=$!
sleep 0.05
(
  cd "$REPO" && source "$JIRA_COMMON"
  _quick_write() { echo "writer-2" >> "$OUTPUT_FILE"; }
  ACCELERATOR_TEST_MODE=1 JIRA_LOCK_TIMEOUT_SECS=10 JIRA_LOCK_SLEEP_SECS=0.05 \
    jira_with_lock _quick_write
) &
PID2=$!
wait "$PID1" "$PID2"
LINE_COUNT=$(wc -l < "$OUTPUT_FILE" | tr -d ' ')
assert_eq "both writers completed" "2" "$LINE_COUNT"
# Both lines present — order may vary but no interleaving possible in this test
CONTAINS_1=$(grep -c "writer-1" "$OUTPUT_FILE" 2>/dev/null || echo 0)
CONTAINS_2=$(grep -c "writer-2" "$OUTPUT_FILE" 2>/dev/null || echo 0)
assert_eq "writer-1 ran" "1" "$CONTAINS_1"
assert_eq "writer-2 ran" "1" "$CONTAINS_2"

echo "Test: (b) dead-holder recovery — stale lock with dead PID is reclaimed"
REPO=$(setup_repo)
STATE_DIR=$(cd "$REPO" && source "$JIRA_COMMON" && jira_state_dir)
LOCKDIR="$STATE_DIR/.lock"
mkdir -p "$LOCKDIR"
echo "999999" > "$LOCKDIR/holder.pid"
echo "12345" > "$LOCKDIR/holder.start"
echo "fake-script.sh" > "$LOCKDIR/holder.cmd"
RESULT=""
(
  cd "$REPO" && source "$JIRA_COMMON"
  _noop() { echo "ran"; }
  RESULT=$(ACCELERATOR_TEST_MODE=1 JIRA_LOCK_TIMEOUT_SECS=5 JIRA_LOCK_SLEEP_SECS=0.05 \
    jira_with_lock _noop)
  echo "$RESULT"
) > /tmp/lock-test-b-$$.txt 2>/dev/null
RESULT=$(cat /tmp/lock-test-b-$$.txt 2>/dev/null || echo "")
rm -f /tmp/lock-test-b-$$.txt
assert_eq "dead-holder lock reclaimed and function ran" "ran" "$RESULT"

echo "Test: (c) SIGKILL holder recovery — lock from killed process is reclaimed"
REPO=$(setup_repo)
STATE_DIR=$(cd "$REPO" && source "$JIRA_COMMON" && jira_state_dir)
# Start a background process that will hold the lock, then kill it
(
  cd "$REPO" && source "$JIRA_COMMON"
  _hold_forever() { sleep 60; }
  ACCELERATOR_TEST_MODE=1 JIRA_LOCK_TIMEOUT_SECS=30 JIRA_LOCK_SLEEP_SECS=0.05 \
    jira_with_lock _hold_forever
) &
HOLDER_PID=$!
sleep 0.15  # give holder time to acquire the lock
kill -9 "$HOLDER_PID" 2>/dev/null || true
wait "$HOLDER_PID" 2>/dev/null || true
sleep 0.05  # let OS clean up
# Now try to acquire — should succeed despite orphaned lockdir
RESULT=""
EXIT_CODE=0
(
  cd "$REPO" && source "$JIRA_COMMON"
  _noop() { echo "recovered"; }
  RESULT=$(ACCELERATOR_TEST_MODE=1 JIRA_LOCK_TIMEOUT_SECS=5 JIRA_LOCK_SLEEP_SECS=0.05 \
    jira_with_lock _noop 2>/dev/null)
  echo "$RESULT"
) > /tmp/lock-test-c-$$.txt 2>/dev/null || EXIT_CODE=$?
RESULT=$(cat /tmp/lock-test-c-$$.txt 2>/dev/null || echo "")
rm -f /tmp/lock-test-c-$$.txt
assert_eq "SIGKILL-orphaned lock recovered" "recovered" "$RESULT"

echo "Test: (d) PID-recycling defence — live PID with wrong start-time is treated as stale"
REPO=$(setup_repo)
STATE_DIR=$(cd "$REPO" && source "$JIRA_COMMON" && jira_state_dir)
LOCKDIR="$STATE_DIR/.lock"
mkdir -p "$LOCKDIR"
# Use the test's own PID (live!) but a deliberately wrong start time
echo "$$" > "$LOCKDIR/holder.pid"
echo "WRONG-START-TIME-99999" > "$LOCKDIR/holder.start"
echo "fake-script.sh" > "$LOCKDIR/holder.cmd"
RESULT=""
(
  cd "$REPO" && source "$JIRA_COMMON"
  _noop() { echo "pid-recycle-ran"; }
  RESULT=$(ACCELERATOR_TEST_MODE=1 JIRA_LOCK_TIMEOUT_SECS=5 JIRA_LOCK_SLEEP_SECS=0.05 \
    jira_with_lock _noop 2>/dev/null)
  echo "$RESULT"
) > /tmp/lock-test-d-$$.txt 2>/dev/null
RESULT=$(cat /tmp/lock-test-d-$$.txt 2>/dev/null || echo "")
rm -f /tmp/lock-test-d-$$.txt
assert_eq "recycled-PID lock treated as stale" "pid-recycle-ran" "$RESULT"

echo "Test: (e) timeout diagnosis — times out with E_REFRESH_LOCKED exit 53"
REPO=$(setup_repo)
STATE_DIR=$(cd "$REPO" && source "$JIRA_COMMON" && jira_state_dir)
# Start a real holder that will stay alive long enough
(
  cd "$REPO" && source "$JIRA_COMMON"
  _hold_for_test() { sleep 5; }
  ACCELERATOR_TEST_MODE=1 JIRA_LOCK_TIMEOUT_SECS=30 JIRA_LOCK_SLEEP_SECS=0.05 \
    jira_with_lock _hold_for_test
) &
HOLDER_PID=$!
# Poll until holder acquires the lock (up to 5s) — avoids a fixed-sleep race
_waited=0
until [[ -d "$STATE_DIR/.lock" ]] || [[ "$_waited" -ge 50 ]]; do
  sleep 0.1
  _waited=$(( _waited + 1 ))
done
EXIT_CODE=0
STDERR=$(
  cd "$REPO" && source "$JIRA_COMMON"
  _noop() { echo "should-not-run"; }
  ACCELERATOR_TEST_MODE=1 JIRA_LOCK_TIMEOUT_SECS=1 JIRA_LOCK_SLEEP_SECS=0.05 \
    jira_with_lock _noop 2>&1
) || EXIT_CODE=$?
kill "$HOLDER_PID" 2>/dev/null || true
wait "$HOLDER_PID" 2>/dev/null || true
if [ "$EXIT_CODE" -eq 53 ]; then
  echo "  PASS: timeout exits 53"
  PASS=$((PASS + 1))
else
  echo "  FAIL: expected exit 53, got $EXIT_CODE"
  FAIL=$((FAIL + 1))
fi
assert_contains "E_REFRESH_LOCKED in stderr" "E_REFRESH_LOCKED" "$STDERR"
assert_contains "holder pid in stderr" "(pid " "$STDERR"

echo ""

# ---------------------------------------------------------------------------
echo "=== jira_require_dependencies ==="
echo ""

echo "Test: exits 0 when jq, curl, awk are present"
REPO=$(setup_repo)
EXIT_CODE=0
(cd "$REPO" && source "$JIRA_COMMON" && jira_require_dependencies 2>/dev/null) || EXIT_CODE=$?
assert_eq "dependencies satisfied" "0" "$EXIT_CODE"

echo ""

test_summary
