#!/usr/bin/env bash
# This harness sources the script-under-test (linear-common.sh via the computed
# $LINEAR_COMMON) inside many subshell test cases; the path is intentionally
# dynamic.
# shellcheck disable=SC1090
# Helper functions in the subshell test cases are passed by name to
# linear_with_lock and invoked by it, not within this library.
# shellcheck disable=SC2329
set -euo pipefail

# Tests for linear-common.sh
# Run: bash skills/integrations/linear/scripts/test-linear-common.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
LINEAR_COMMON="$SCRIPT_DIR/linear-common.sh"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$repo_dir/.git"
  mkdir -p "$repo_dir/.accelerator"
  echo "$repo_dir"
}

# ---------------------------------------------------------------------------
echo "=== linear_state_dir ==="
echo ""

echo "Test: linear_state_dir returns <repo_root>/.accelerator/state/integrations/linear"
REPO=$(setup_repo)
STATE_DIR=$(cd "$REPO" && source "$LINEAR_COMMON" && linear_state_dir)
assert_eq "state dir under repo root" "$REPO/.accelerator/state/integrations/linear" "$STATE_DIR"

echo "Test: linear_state_dir creates the directory if missing"
REPO=$(setup_repo)
cd "$REPO" && source "$LINEAR_COMMON" && linear_state_dir >/dev/null
assert_dir_exists "state dir created" "$REPO/.accelerator/state/integrations/linear"

echo "Test: linear_state_dir respects paths.integrations override"
REPO=$(setup_repo)
cat >"$REPO/.accelerator/config.md" <<'FIXTURE'
---
paths:
  integrations: .state/integrations
---
FIXTURE
STATE_DIR=$(cd "$REPO" && source "$LINEAR_COMMON" && linear_state_dir)
assert_eq "custom integrations path used" "$REPO/.state/integrations/linear" "$STATE_DIR"

echo ""

# ---------------------------------------------------------------------------
echo "=== linear_die / linear_warn ==="
echo ""

echo "Test: linear_die writes message to stderr and exits non-zero"
REPO=$(setup_repo)
STDERR=$(cd "$REPO" && source "$LINEAR_COMMON" && linear_die "E_TEST: failed" 2>&1) || true
assert_contains "stderr contains message" "$STDERR" "E_TEST: failed"

echo "Test: linear_die exits non-zero"
REPO=$(setup_repo)
EXIT_CODE=0
(cd "$REPO" && source "$LINEAR_COMMON" && linear_die "msg") 2>/dev/null || EXIT_CODE=$?
if [ "$EXIT_CODE" -ne 0 ]; then
  echo "  PASS: linear_die exits non-zero"
  PASS=$((PASS + 1))
else
  echo "  FAIL: linear_die should exit non-zero"
  FAIL=$((FAIL + 1))
fi

echo "Test: linear_warn writes Warning: prefix and continues"
REPO=$(setup_repo)
RESULT=$(cd "$REPO" && source "$LINEAR_COMMON" && linear_warn "ok" 2>/dev/null && echo "continued")
assert_eq "execution continues after linear_warn" "continued" "$RESULT"

echo ""

# ---------------------------------------------------------------------------
echo "=== linear_jq_field ==="
echo ""

echo "Test: extracts a present field"
REPO=$(setup_repo)
RESULT=$(cd "$REPO" && source "$LINEAR_COMMON" && linear_jq_field '{"name":"lin"}' '.name')
assert_eq "field extracted" "lin" "$RESULT"

echo "Test: returns empty for missing path"
REPO=$(setup_repo)
RESULT=$(cd "$REPO" && source "$LINEAR_COMMON" && linear_jq_field '{"name":"lin"}' '.missing')
assert_eq "empty for missing" "" "$RESULT"

echo "Test: nested field extraction"
REPO=$(setup_repo)
RESULT=$(cd "$REPO" && source "$LINEAR_COMMON" && linear_jq_field '{"a":{"b":"deep"}}' '.a.b')
assert_eq "nested field" "deep" "$RESULT"

echo ""

# ---------------------------------------------------------------------------
echo "=== linear_atomic_write_json ==="
echo ""

echo "Test: valid JSON written to target"
REPO=$(setup_repo)
TARGET="$REPO/out.json"
(cd "$REPO" && source "$LINEAR_COMMON" && printf '{"ok":1}' | linear_atomic_write_json "$TARGET")
CONTENT=$(cat "$TARGET")
assert_eq "valid JSON written" '{"ok":1}' "$CONTENT"

echo "Test: invalid JSON exits non-zero with E_BAD_JSON, leaves target intact"
REPO=$(setup_repo)
TARGET="$REPO/out.json"
printf '{"original":1}' >"$TARGET"
EXIT_CODE=0
STDERR=$(cd "$REPO" && source "$LINEAR_COMMON" && printf 'not json' | linear_atomic_write_json "$TARGET" 2>&1) || EXIT_CODE=$?
if [ "$EXIT_CODE" -ne 0 ]; then
  echo "  PASS: exits non-zero on invalid JSON"
  PASS=$((PASS + 1))
else
  echo "  FAIL: should exit non-zero on invalid JSON"
  FAIL=$((FAIL + 1))
fi
assert_contains "E_BAD_JSON on stderr" "$STDERR" "E_BAD_JSON"
assert_eq "original file intact after bad JSON" '{"original":1}' "$(cat "$TARGET")"

echo ""

# ---------------------------------------------------------------------------
echo "=== linear_with_lock ==="
echo ""

echo "Test: (a) live-holder serialisation — two writers serialise"
REPO=$(setup_repo)
OUTPUT_FILE="$REPO/lock-output.txt"
rm -f "$OUTPUT_FILE"
(
  cd "$REPO" && source "$LINEAR_COMMON"
  _hold_write() {
    sleep 0.2
    echo "writer-1" >>"$OUTPUT_FILE"
  }
  ACCELERATOR_TEST_MODE=1 LINEAR_LOCK_TIMEOUT_SECS=10 LINEAR_LOCK_SLEEP_SECS=0.05 \
    linear_with_lock _hold_write
) &
PID1=$!
sleep 0.05
(
  cd "$REPO" && source "$LINEAR_COMMON"
  _quick_write() { echo "writer-2" >>"$OUTPUT_FILE"; }
  ACCELERATOR_TEST_MODE=1 LINEAR_LOCK_TIMEOUT_SECS=10 LINEAR_LOCK_SLEEP_SECS=0.05 \
    linear_with_lock _quick_write
) &
PID2=$!
wait "$PID1" "$PID2"
LINE_COUNT=$(wc -l <"$OUTPUT_FILE" | tr -d ' ')
assert_eq "both writers completed" "2" "$LINE_COUNT"

echo "Test: (b) dead-holder recovery — stale lock with dead PID is reclaimed"
REPO=$(setup_repo)
STATE_DIR=$(cd "$REPO" && source "$LINEAR_COMMON" && linear_state_dir)
LOCKDIR="$STATE_DIR/.lock"
mkdir -p "$LOCKDIR"
echo "999999" >"$LOCKDIR/holder.pid"
echo "12345" >"$LOCKDIR/holder.start"
echo "fake-script.sh" >"$LOCKDIR/holder.cmd"
(
  cd "$REPO" && source "$LINEAR_COMMON"
  _noop() { echo "ran"; }
  ACCELERATOR_TEST_MODE=1 LINEAR_LOCK_TIMEOUT_SECS=5 LINEAR_LOCK_SLEEP_SECS=0.05 \
    linear_with_lock _noop
) >/tmp/linear-lock-test-b-$$.txt 2>/dev/null
RESULT=$(cat /tmp/linear-lock-test-b-$$.txt 2>/dev/null || echo "")
rm -f /tmp/linear-lock-test-b-$$.txt
assert_eq "dead-holder lock reclaimed and function ran" "ran" "$RESULT"

echo "Test: (c) timeout diagnosis — times out with E_REFRESH_LOCKED exit 53"
REPO=$(setup_repo)
STATE_DIR=$(cd "$REPO" && source "$LINEAR_COMMON" && linear_state_dir)
(
  cd "$REPO" && source "$LINEAR_COMMON"
  _hold_for_test() { sleep 5; }
  ACCELERATOR_TEST_MODE=1 LINEAR_LOCK_TIMEOUT_SECS=30 LINEAR_LOCK_SLEEP_SECS=0.05 \
    linear_with_lock _hold_for_test
) &
HOLDER_PID=$!
_waited=0
until [[ -d "$STATE_DIR/.lock" ]] || [[ "$_waited" -ge 50 ]]; do
  sleep 0.1
  _waited=$((_waited + 1))
done
EXIT_CODE=0
STDERR=$(
  cd "$REPO" && source "$LINEAR_COMMON"
  _noop() { echo "should-not-run"; }
  ACCELERATOR_TEST_MODE=1 LINEAR_LOCK_TIMEOUT_SECS=1 LINEAR_LOCK_SLEEP_SECS=0.05 \
    linear_with_lock _noop 2>&1
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
assert_contains "E_REFRESH_LOCKED in stderr" "$STDERR" "E_REFRESH_LOCKED"

echo ""

# ---------------------------------------------------------------------------
echo "=== linear_require_dependencies ==="
echo ""
echo "Test: exits 0 when jq, curl, awk are present"
REPO=$(setup_repo)
EXIT_CODE=0
(cd "$REPO" && source "$LINEAR_COMMON" && linear_require_dependencies 2>/dev/null) || EXIT_CODE=$?
assert_eq "dependencies satisfied" "0" "$EXIT_CODE"
echo ""

# ---------------------------------------------------------------------------
echo "=== _linear_emit_generic_hint ==="
echo ""
source "$LINEAR_COMMON"
# shellcheck disable=SC2069 # intentional: capture stderr on stdout, discard real stdout
_hint() { _linear_emit_generic_hint "$1" 2>&1 >/dev/null || true; }
_hint_rc() {
  local rc=0
  _linear_emit_generic_hint "$1" 2>/dev/null || rc=$?
  echo "$rc"
}
assert_contains "code 11 emits credentials hint" "$(_hint 11)" "credentials"
assert_eq "code 11 returns 0" "0" "$(_hint_rc 11)"
assert_contains "code 35 emits rate-limit hint" "$(_hint 35)" "rate-limited"
assert_contains "code 36 emits complexity hint" "$(_hint 36)" "complexity"
HINT_99=$(_hint 99)
assert_empty "code 99 emits no hint" "$HINT_99"
assert_eq "code 99 returns 1" "1" "$(_hint_rc 99)"
echo ""

test_summary
