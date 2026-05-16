#!/usr/bin/env bash
set -uo pipefail

# Test harness for skills/github/scripts/pr-base-repo.sh
# Run: bash skills/github/scripts/test-pr-base-repo-scripts.sh
#
# Uses a PATH-stubbed `gh` (installed by setup_gh_stub) so each test
# runs hermetically. Tree-state regression guards (tests 22/23) honour
# the PHASE env var so harness can be GREEN at every phase boundary
# while still locking the final state in CI.
#
# Note: `set -e` intentionally omitted so a failing assertion does not
# abort the harness; the assert_* helpers tally failures into FAIL and
# test_summary returns non-zero at the end if any failed.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
# shellcheck source=/dev/null
source "$PLUGIN_ROOT/scripts/test-helpers.sh"
# shellcheck source=/dev/null
source "$SCRIPT_DIR/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/pr-base-repo.sh"

phase="${PHASE:-final}"
case "$phase" in
  1|2|3|4|5|6|final) : ;;
  *) echo "unknown PHASE: $phase (expected 1-6 or final)" >&2; exit 2 ;;
esac

TMPDIR_BASE=$(mktemp -d)
ORIG_PATH="$PATH"
# Restore PATH inside the trap because some tests strip PATH down to
# isolate missing-tool preflight checks; without restoring it here
# `rm` itself can be unreachable when the trap fires.
trap 'PATH="$ORIG_PATH"; rm -rf "$TMPDIR_BASE"' EXIT

# Allocate a fresh per-test tempdir and reset PATH to a clean baseline.
# Returns the tempdir path on stdout.
# Resets the per-test env (PATH, gh-stub env vars) and allocates a
# fresh tempdir into the global CASE_DIR. Sets a global (not via
# command-substitution) so PATH/env mutations propagate back to the
# main test shell rather than being trapped inside a subshell.
new_case() {
  # Reset PATH BEFORE invoking mktemp so a prior test that scoped PATH
  # down (e.g. the missing-jq test) doesn't leave us unable to find
  # mktemp here.
  export PATH="$ORIG_PATH"
  unset GH_PR_VIEW_OUT GH_PR_VIEW_ERR GH_PR_VIEW_RC
  unset GH_API_OUT GH_API_ERR GH_API_RC
  unset GH_ARGV_LOG GH_STDIN_LOG
  unset TMPDIR
  CASE_DIR=$(mktemp -d "$TMPDIR_BASE/case-XXXXXX")
}

write_file() {
  printf '%s' "$2" > "$1"
}

echo "=== pr-base-repo.sh tests (PHASE=$phase) ==="

# ---------------------------------------------------------------
# Test 1: Script is executable.
echo ""
echo "--- test 1: executable ---"
assert_file_executable "pr-base-repo.sh is executable" "$SCRIPT"

# ---------------------------------------------------------------
# Test 2: Usage at 0 args → exit 2 with `Usage:` on stderr.
echo ""
echo "--- test 2: usage at zero args ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
stderr_file="$T/stderr"
rc=0
"$SCRIPT" 2>"$stderr_file" >/dev/null || rc=$?
assert_eq "test 2: usage exit code is 2" 2 "$rc"
assert_contains "test 2: usage stderr mentions Usage:" \
  "$(cat "$stderr_file")" "Usage:"

# ---------------------------------------------------------------
# Test 3: Same-repo PR resolves.
echo ""
echo "--- test 3: same-repo resolves ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"baseRepository":{"owner":{"login":"acme"},"name":"app"}}'
out=$(GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$T/stderr") || true
assert_eq "test 3: stdout is acme/app" "acme/app" "$out"

# ---------------------------------------------------------------
# Test 4: Cross-fork PR resolves to upstream.
echo ""
echo "--- test 4: cross-fork resolves to upstream ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" \
  '{"baseRepository":{"owner":{"login":"upstream-org"},"name":"upstream-repo"}}'
out=$(GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$T/stderr") || true
assert_eq "test 4: stdout targets upstream coords" \
  "upstream-org/upstream-repo" "$out"

# ---------------------------------------------------------------
# Test 5: Recorded argv shape (exact full-line match).
echo ""
echo "--- test 5: argv shape ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"baseRepository":{"owner":{"login":"acme"},"name":"app"}}'
GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 >/dev/null 2>"$T/stderr" || true
argv=$(cat "$GH_ARGV_LOG")
assert_eq "test 5: argv is exactly 'pr view 119 --json baseRepository'" \
  "pr view 119 --json baseRepository" "$argv"

# ---------------------------------------------------------------
# Test 6: Resolver failure — stderr preserved, conditional hint fires.
echo ""
echo "--- test 6: failure with no-default-remote replay+hint ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
stderr_payload="$T/pr-view-err"
write_file "$stderr_payload" "no default remote repository"
stderr_capture="$T/stderr"
rc=0
GH_PR_VIEW_ERR="$stderr_payload" GH_PR_VIEW_RC=1 \
  "$SCRIPT" 119 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 6: exit 1 on resolution failure" 1 "$rc"
assert_contains "test 6: preserved gh stderr is replayed" \
  "$(cat "$stderr_capture")" "no default remote repository"
assert_contains "test 6: set-default hint is emitted" \
  "$(cat "$stderr_capture")" "gh repo set-default"

# ---------------------------------------------------------------
# Test 7: Resolver failure — non-matching stderr, no false hint.
echo ""
echo "--- test 7: failure without false hint ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
stderr_payload="$T/pr-view-err"
write_file "$stderr_payload" "HTTP 403: SSO required"
stderr_capture="$T/stderr"
rc=0
GH_PR_VIEW_ERR="$stderr_payload" GH_PR_VIEW_RC=1 \
  "$SCRIPT" 119 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 7: exit 1 on resolution failure" 1 "$rc"
assert_contains "test 7: preserved gh stderr is replayed" \
  "$(cat "$stderr_capture")" "HTTP 403: SSO required"
if grep -qF "gh repo set-default" "$stderr_capture"; then
  echo "  FAIL: test 7: set-default hint must NOT be emitted"
  echo "    stderr: $(cat "$stderr_capture")"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: test 7: set-default hint suppressed for unrelated error"
  PASS=$((PASS + 1))
fi

# ---------------------------------------------------------------
# Test 8: Null owner guard.
echo ""
echo "--- test 8: null owner guard ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"baseRepository":{"owner":{"login":null},"name":"app"}}'
stderr_capture="$T/stderr"
stdout_capture="$T/stdout"
rc=0
GH_PR_VIEW_OUT="$payload" \
  "$SCRIPT" 119 >"$stdout_capture" 2>"$stderr_capture" || rc=$?
assert_eq "test 8: null owner exits 1" 1 "$rc"
if grep -qF "null/app" "$stdout_capture"; then
  echo "  FAIL: test 8: must NOT print null/app to stdout"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: test 8: stdout does not smuggle null/app"
  PASS=$((PASS + 1))
fi
assert_contains "test 8: stderr explains the missing field" \
  "$(cat "$stderr_capture")" "owner"

# ---------------------------------------------------------------
# Test 9: Null name guard.
echo ""
echo "--- test 9: null name guard ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"baseRepository":{"owner":{"login":"acme"},"name":null}}'
stderr_capture="$T/stderr"
stdout_capture="$T/stdout"
rc=0
GH_PR_VIEW_OUT="$payload" \
  "$SCRIPT" 119 >"$stdout_capture" 2>"$stderr_capture" || rc=$?
assert_eq "test 9: null name exits 1" 1 "$rc"
if grep -qF "acme/null" "$stdout_capture"; then
  echo "  FAIL: test 9: must NOT print acme/null to stdout"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: test 9: stdout does not smuggle acme/null"
  PASS=$((PASS + 1))
fi
assert_contains "test 9: stderr mentions name field" \
  "$(cat "$stderr_capture")" "name"

# ---------------------------------------------------------------
# Test 10: Missing jq preflight — exit 2 with remediation.
echo ""
echo "--- test 10: missing jq preflight ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
# Replace PATH entirely so no system jq is reachable. Only the
# fake-gh bin-dir is on PATH.
export PATH="$T/bin"
stderr_capture="$T/stderr"
rc=0
"$SCRIPT" 119 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 10: missing jq exits 2" 2 "$rc"
assert_contains "test 10: stderr names jq" \
  "$(cat "$stderr_capture")" "jq is required"

# ---------------------------------------------------------------
# Test 11: Missing baseRepository field → exit 1.
echo ""
echo "--- test 11: missing baseRepository field ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{}'
stderr_capture="$T/stderr"
rc=0
GH_PR_VIEW_OUT="$payload" \
  "$SCRIPT" 119 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 11: missing baseRepository exits 1" 1 "$rc"

# ---------------------------------------------------------------
# Test 12: Non-JSON stdout → clear error rather than opaque jq parse.
echo ""
echo "--- test 12: non-JSON output ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '<html><body>Sign in</body></html>'
stderr_capture="$T/stderr"
rc=0
GH_PR_VIEW_OUT="$payload" \
  "$SCRIPT" 119 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 12: non-JSON exits 1" 1 "$rc"
assert_contains "test 12: stderr mentions non-JSON" \
  "$(cat "$stderr_capture")" "non-JSON"

# ---------------------------------------------------------------
# Tree-state regression guards (phase-conditional).
echo ""
echo "=== Tree-state regression guards ==="

# Test 22 — regression guard against gh pr edit.
case "$phase" in
  1|2|3) skip_test "test 22" "deferred — guards Phase 4" ;;
  4|5|6|final)
    assert_grep_empty "test 22" \
      "$PLUGIN_ROOT/skills/" "gh pr edit" \
      --include='*.md'
    ;;
esac

# Test 23 — regression guard against cross-fork-unsafe resolver.
case "$phase" in
  1|2|3|4|5) skip_test "test 23" "deferred — guards Phase 6" ;;
  6|final)
    assert_grep_empty "test 23" \
      "$PLUGIN_ROOT/skills/github/" "gh repo view --json owner,name" \
      --include='*.md'
    ;;
esac

test_summary
