#!/usr/bin/env bash
set -uo pipefail

# Test harness for skills/github/describe-pr/scripts/pr-update-body.sh
# Run: bash skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh
#
# Uses a PATH-stubbed `gh` (via setup_gh_stub) plus an optional
# PATH-stubbed `jq` (via setup_fake_jq) to simulate failures.
# `set -e` is intentionally omitted so failing asserts don't abort.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
# shellcheck source=/dev/null
source "$PLUGIN_ROOT/scripts/test-helpers.sh"
# shellcheck source=/dev/null
source "$SCRIPT_DIR/../../scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/pr-update-body.sh"

phase="${PHASE:-final}"
case "$phase" in
  1|2|3|4|5|6|final) : ;;
  *) echo "unknown PHASE: $phase (expected 1-6 or final)" >&2; exit 2 ;;
esac

TMPDIR_BASE=$(mktemp -d)
ORIG_PATH="$PATH"
# Snapshot the absolute path to bash before any test scopes PATH down,
# so the missing-jq test can invoke bash without leaning on PATH lookup
# (a prefix-env-var assignment also gates command resolution).
BASH_BIN="$(command -v bash)"
# Restore PATH inside the trap because some tests strip PATH down to
# isolate missing-tool preflight checks; without restoring it here
# `rm` itself can be unreachable when the trap fires.
trap 'PATH="$ORIG_PATH"; rm -rf "$TMPDIR_BASE"' EXIT

# Resets the per-test env and allocates a fresh tempdir into the
# global CASE_DIR. Setting a global (not via command substitution)
# is required so PATH mutations propagate back to the main shell.
new_case() {
  # Reset PATH BEFORE invoking mktemp so a prior test that scoped PATH
  # down (e.g. the missing-jq test) doesn't leave us unable to find
  # mktemp here.
  export PATH="$ORIG_PATH"
  unset GH_PR_VIEW_OUT GH_PR_VIEW_ERR GH_PR_VIEW_RC
  unset GH_API_OUT GH_API_ERR GH_API_RC
  unset GH_ARGV_LOG GH_STDIN_LOG
  unset FAKE_JQ_REAL_PATH
  unset TMPDIR
  CASE_DIR=$(mktemp -d "$TMPDIR_BASE/case-XXXXXX")
}

write_file() {
  printf '%s' "$2" > "$1"
}

# Standard same-repo base-repo payload reused across many tests.
default_payload() {
  echo '{"baseRepository":{"owner":{"login":"acme"},"name":"app"}}'
}

# Standard upstream payload reused by the cross-fork tests.
upstream_payload() {
  echo '{"baseRepository":{"owner":{"login":"upstream-org"},"name":"upstream-repo"}}'
}

echo "=== pr-update-body.sh tests (PHASE=$phase) ==="

# ---------------------------------------------------------------
# Test 1: Script is executable.
echo ""
echo "--- test 1: executable ---"
assert_file_executable "pr-update-body.sh is executable" "$SCRIPT"

# ---------------------------------------------------------------
# Test 2: Usage at 0 args.
echo ""
echo "--- test 2: usage at zero args ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
stderr_file="$T/stderr"
rc=0
"$SCRIPT" 2>"$stderr_file" >/dev/null || rc=$?
assert_eq "test 2: zero-arg exit 2" 2 "$rc"
assert_contains "test 2: stderr says Usage:" \
  "$(cat "$stderr_file")" "Usage:"

# ---------------------------------------------------------------
# Test 3: Usage at 1 arg.
echo ""
echo "--- test 3: usage at one arg ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
stderr_file="$T/stderr"
rc=0
"$SCRIPT" 119 2>"$stderr_file" >/dev/null || rc=$?
assert_eq "test 3: one-arg exit 2" 2 "$rc"
assert_contains "test 3: stderr says Usage:" \
  "$(cat "$stderr_file")" "Usage:"

# ---------------------------------------------------------------
# Test 4: Usage at 3 args.
echo ""
echo "--- test 4: usage at three args ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
body_file="$T/body.md"
write_file "$body_file" "hello"
stderr_file="$T/stderr"
rc=0
"$SCRIPT" 119 "$body_file" extra 2>"$stderr_file" >/dev/null || rc=$?
assert_eq "test 4: three-arg exit 2" 2 "$rc"
assert_contains "test 4: stderr says Usage:" \
  "$(cat "$stderr_file")" "Usage:"

# ---------------------------------------------------------------
# Test 5: Missing body file.
echo ""
echo "--- test 5: missing body file ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
stderr_file="$T/stderr"
rc=0
"$SCRIPT" 119 "$T/does-not-exist.md" 2>"$stderr_file" >/dev/null || rc=$?
assert_eq "test 5: missing-file exit 2" 2 "$rc"
assert_contains "test 5: stderr names the missing file" \
  "$(cat "$stderr_file")" "does-not-exist.md"

# ---------------------------------------------------------------
# Test 6: Same-repo PR — resolver argv.
echo ""
echo "--- test 6: resolver argv shape ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
GH_PR_VIEW_OUT="$T/pr-view.json"
write_file "$GH_PR_VIEW_OUT" "$(default_payload)"
export GH_PR_VIEW_OUT
body_file="$T/body.md"
write_file "$body_file" "hello world"
"$SCRIPT" 119 "$body_file" >/dev/null 2>"$T/stderr" || true
# Pull the pr-view line out of the recorded argv log.
pr_view_line=$(grep "^pr view" "$GH_ARGV_LOG" || true)
assert_eq "test 6: pr view argv shape" \
  "pr view 119 --json baseRepository" "$pr_view_line"
unset GH_PR_VIEW_OUT

# ---------------------------------------------------------------
# Test 7: Cross-fork PR — PATCH URL targets upstream.
echo ""
echo "--- test 7: PATCH URL targets upstream coords ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
GH_PR_VIEW_OUT="$T/pr-view.json"
write_file "$GH_PR_VIEW_OUT" "$(upstream_payload)"
export GH_PR_VIEW_OUT
body_file="$T/body.md"
write_file "$body_file" "hello"
"$SCRIPT" 119 "$body_file" >/dev/null 2>"$T/stderr" || true
api_line=$(grep "^api " "$GH_ARGV_LOG" || true)
# Strip the variable --input <tmppath> tail so we can assert the
# stable prefix exactly without coupling to mktemp's path scheme.
api_prefix=${api_line% --input *}
assert_eq "test 7: api argv targets upstream/upstream pulls/119" \
  "api --method PATCH repos/upstream-org/upstream-repo/pulls/119" \
  "$api_prefix"
# Belt-and-braces: ensure no fork coords sneak in.
if grep -qF "acme/app" "$GH_ARGV_LOG"; then
  echo "  FAIL: test 7: fork coords must not appear in api argv"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: test 7: no fork coords in api argv"
  PASS=$((PASS + 1))
fi
unset GH_PR_VIEW_OUT

# ---------------------------------------------------------------
# Test 8: PATCH method explicit.
echo ""
echo "--- test 8: PATCH method explicit ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
GH_PR_VIEW_OUT="$T/pr-view.json"
write_file "$GH_PR_VIEW_OUT" "$(default_payload)"
export GH_PR_VIEW_OUT
body_file="$T/body.md"
write_file "$body_file" "x"
"$SCRIPT" 119 "$body_file" >/dev/null 2>"$T/stderr" || true
api_line=$(grep "^api " "$GH_ARGV_LOG" || true)
assert_contains "test 8: api argv includes --method PATCH" \
  "$api_line" "--method PATCH"
unset GH_PR_VIEW_OUT

# ---------------------------------------------------------------
# Round-trip body-encoding helper.
# Encodes the input file the same way the helper does, by sending it
# through pr-update-body.sh and re-extracting `.body` from the JSON
# captured in $GH_STDIN_LOG. Asserts byte-for-byte equality.
assert_body_round_trip() {
  local test_name="$1" input="$2" expected="$3"
  GH_PR_VIEW_OUT="$T/pr-view.json"
  write_file "$GH_PR_VIEW_OUT" "$(default_payload)"
  export GH_PR_VIEW_OUT
  local body_file="$T/body.md"
  write_file "$body_file" "$input"
  "$SCRIPT" 119 "$body_file" >/dev/null 2>"$T/stderr" || true
  local extracted
  extracted=$(jq -r .body "$GH_STDIN_LOG")
  assert_eq "$test_name" "$expected" "$extracted"
  unset GH_PR_VIEW_OUT
}

# ---------------------------------------------------------------
# Test 9: JSON body encoding — empty body.
echo ""
echo "--- test 9: empty body round-trip ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
assert_body_round_trip "test 9: empty body round-trip" "" ""

# ---------------------------------------------------------------
# Test 10: JSON body encoding — multi-line.
echo ""
echo "--- test 10: multi-line body round-trip ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
multi=$'Hello\n\nWorld\n'
assert_body_round_trip "test 10: multi-line round-trip" "$multi" "$multi"

# ---------------------------------------------------------------
# Test 11: JSON body encoding — shell metacharacters.
echo ""
echo "--- test 11: shell metacharacters round-trip ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
# shellcheck disable=SC2016
# Intentional literal: shell metacharacters must reach the body verbatim
# (the encoder is jq, not the shell). Single quotes are correct here.
meta='`echo bad` $(echo bad) "quote" '\''apos'\'' back\\slash'
assert_body_round_trip "test 11: shell-meta round-trip" "$meta" "$meta"

# ---------------------------------------------------------------
# Test 12: JSON body encoding — unicode.
echo ""
echo "--- test 12: unicode round-trip ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
uni="hello 🎉 café"
assert_body_round_trip "test 12: unicode round-trip" "$uni" "$uni"

# ---------------------------------------------------------------
# Test 13: JSON body encoding — no trailing newline.
echo ""
echo "--- test 13: no trailing newline round-trip ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
notail="single line no newline"
assert_body_round_trip "test 13: no-trailing-newline round-trip" \
  "$notail" "$notail"

# ---------------------------------------------------------------
# Test 14: Stdin pipe via --input <file>.
echo ""
echo "--- test 14: --input <file> ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
GH_PR_VIEW_OUT="$T/pr-view.json"
write_file "$GH_PR_VIEW_OUT" "$(default_payload)"
export GH_PR_VIEW_OUT
body_file="$T/body.md"
write_file "$body_file" "hello"
"$SCRIPT" 119 "$body_file" >/dev/null 2>"$T/stderr" || true
api_line=$(grep "^api " "$GH_ARGV_LOG" || true)
input_path=${api_line##* --input }
# input_path is the trailing token; should not be `-` (we use real files).
if [ "$input_path" = "-" ] || [ -z "$input_path" ]; then
  echo "  FAIL: test 14: --input must be a real path, got: $(printf '%q' "$input_path")"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: test 14: --input is a real file path"
  PASS=$((PASS + 1))
fi
if [ -s "$GH_STDIN_LOG" ]; then
  echo "  PASS: test 14: \$GH_STDIN_LOG is non-empty"
  PASS=$((PASS + 1))
else
  echo "  FAIL: test 14: \$GH_STDIN_LOG was empty"
  FAIL=$((FAIL + 1))
fi
unset GH_PR_VIEW_OUT

# ---------------------------------------------------------------
# Test 15: Tempfile cleanup on success.
echo ""
echo "--- test 15: tempfile cleanup on success ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
GH_PR_VIEW_OUT="$T/pr-view.json"
write_file "$GH_PR_VIEW_OUT" "$(default_payload)"
export GH_PR_VIEW_OUT
body_file="$T/body.md"
write_file "$body_file" "x"
"$SCRIPT" 119 "$body_file" >/dev/null 2>"$T/stderr" || true
if [ -z "$(ls -A "$TMPDIR")" ]; then
  echo "  PASS: test 15: TMPDIR empty after success"
  PASS=$((PASS + 1))
else
  echo "  FAIL: test 15: TMPDIR has leftovers: $(ls -A "$TMPDIR")"
  FAIL=$((FAIL + 1))
fi
unset GH_PR_VIEW_OUT

# ---------------------------------------------------------------
# Test 16: Tempfile cleanup on PATCH failure.
echo ""
echo "--- test 16: tempfile cleanup on PATCH failure ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
GH_PR_VIEW_OUT="$T/pr-view.json"
write_file "$GH_PR_VIEW_OUT" "$(default_payload)"
export GH_PR_VIEW_OUT GH_API_RC=1
body_file="$T/body.md"
write_file "$body_file" "x"
"$SCRIPT" 119 "$body_file" >/dev/null 2>"$T/stderr" || true
if [ -z "$(ls -A "$TMPDIR")" ]; then
  echo "  PASS: test 16: TMPDIR empty after PATCH failure"
  PASS=$((PASS + 1))
else
  echo "  FAIL: test 16: TMPDIR has leftovers: $(ls -A "$TMPDIR")"
  FAIL=$((FAIL + 1))
fi
unset GH_PR_VIEW_OUT GH_API_RC

# ---------------------------------------------------------------
# Test 17: Resolver failure propagated (exit 1 + replayed stderr + hint).
echo ""
echo "--- test 17: resolver failure propagated ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
GH_PR_VIEW_ERR="$T/pr-view-err"
write_file "$GH_PR_VIEW_ERR" "no default remote repository"
export GH_PR_VIEW_ERR GH_PR_VIEW_RC=1
body_file="$T/body.md"
write_file "$body_file" "x"
stderr_capture="$T/stderr"
rc=0
"$SCRIPT" 119 "$body_file" >/dev/null 2>"$stderr_capture" || rc=$?
assert_eq "test 17: resolver failure exit 1" 1 "$rc"
assert_contains "test 17: replays gh stderr" \
  "$(cat "$stderr_capture")" "no default remote repository"
assert_contains "test 17: emits set-default hint" \
  "$(cat "$stderr_capture")" "gh repo set-default"
unset GH_PR_VIEW_ERR GH_PR_VIEW_RC

# ---------------------------------------------------------------
# Test 18: Encode failure exit code (via fake jq).
echo ""
echo "--- test 18: encode failure ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
if ! setup_fake_jq "$T/jqbin"; then
  skip_test "test 18" "real jq required for fake-jq delegation"
else
  GH_PR_VIEW_OUT="$T/pr-view.json"
  write_file "$GH_PR_VIEW_OUT" "$(default_payload)"
  export GH_PR_VIEW_OUT
  body_file="$T/body.md"
  write_file "$body_file" "anything"
  stderr_capture="$T/stderr"
  rc=0
  "$SCRIPT" 119 "$body_file" >/dev/null 2>"$stderr_capture" || rc=$?
  assert_eq "test 18: encode failure exits 1" 1 "$rc"
  assert_contains "test 18: stderr mentions encode failed" \
    "$(cat "$stderr_capture")" "encode failed"
  assert_contains "test 18: stderr replays fake-jq stderr" \
    "$(cat "$stderr_capture")" "fake-jq: simulated encode failure"
  # Sanity: resolver reached gh (pr view recorded), but PATCH never ran.
  if grep -q "^pr view" "$GH_ARGV_LOG"; then
    echo "  PASS: test 18: resolver reached gh"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: test 18: resolver did not reach gh"
    FAIL=$((FAIL + 1))
  fi
  if grep -q "^api " "$GH_ARGV_LOG"; then
    echo "  FAIL: test 18: PATCH should not have been attempted"
    FAIL=$((FAIL + 1))
  else
    echo "  PASS: test 18: PATCH not attempted after encode failure"
    PASS=$((PASS + 1))
  fi
  unset GH_PR_VIEW_OUT
fi

# ---------------------------------------------------------------
# Test 19: PATCH failure — stage-specific stderr (exit 4).
echo ""
echo "--- test 19: PATCH failure exit 4 ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
GH_PR_VIEW_OUT="$T/pr-view.json"
write_file "$GH_PR_VIEW_OUT" "$(default_payload)"
GH_API_ERR="$T/api-err"
write_file "$GH_API_ERR" "HTTP 422: Validation Failed"
export GH_PR_VIEW_OUT GH_API_ERR GH_API_RC=1
body_file="$T/body.md"
write_file "$body_file" "x"
stderr_capture="$T/stderr"
rc=0
"$SCRIPT" 119 "$body_file" >/dev/null 2>"$stderr_capture" || rc=$?
assert_eq "test 19: PATCH failure exits 4" 4 "$rc"
assert_contains "test 19: stderr replays gh api stderr" \
  "$(cat "$stderr_capture")" "HTTP 422"
assert_contains "test 19: stderr names PATCH stage" \
  "$(cat "$stderr_capture")" "PATCH"
unset GH_PR_VIEW_OUT GH_API_ERR GH_API_RC

# ---------------------------------------------------------------
# Test 20: Success — exit 0 and PATCH was actually called.
echo ""
echo "--- test 20: success path ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
GH_PR_VIEW_OUT="$T/pr-view.json"
write_file "$GH_PR_VIEW_OUT" "$(default_payload)"
export GH_PR_VIEW_OUT
body_file="$T/body.md"
write_file "$body_file" "successful body"
rc=0
"$SCRIPT" 119 "$body_file" >/dev/null 2>"$T/stderr" || rc=$?
assert_eq "test 20: success exit 0" 0 "$rc"
pr_view_count=$(grep -c "^pr view" "$GH_ARGV_LOG" || true)
api_count=$(grep -c "^api " "$GH_ARGV_LOG" || true)
assert_eq "test 20: exactly one pr view recorded" 1 "$pr_view_count"
assert_eq "test 20: exactly one api recorded" 1 "$api_count"
unset GH_PR_VIEW_OUT

# ---------------------------------------------------------------
# Test 21: jq preflight — missing jq exits 2.
echo ""
echo "--- test 21: missing jq preflight ---"
new_case; T=$CASE_DIR
setup_gh_stub "$T"
body_file="$T/body.md"
write_file "$body_file" "x"
stderr_capture="$T/stderr"
rc=0
# Invoke bash directly so the script's shebang doesn't depend on the
# stripped PATH being able to locate bash; only the fake-gh bin-dir is
# on PATH for the subshell, so the script's jq preflight fails.
PATH="$T/bin" "$BASH_BIN" "$SCRIPT" 119 "$body_file" 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 21: missing jq exits 2" 2 "$rc"
assert_contains "test 21: stderr names jq" \
  "$(cat "$stderr_capture")" "jq is required"

# ---------------------------------------------------------------
# Tree-state regression guards (phase-conditional).
echo ""
echo "=== Tree-state regression guards ==="

case "$phase" in
  1|2|3) skip_test "test 22" "deferred — guards Phase 4" ;;
  4|5|6|final)
    assert_grep_empty "test 22" \
      "$PLUGIN_ROOT/skills/" "gh pr edit" \
      --include='*.md'
    ;;
esac

case "$phase" in
  1|2|3|4|5) skip_test "test 23" "deferred — guards Phase 6" ;;
  6|final)
    assert_grep_empty "test 23" \
      "$PLUGIN_ROOT/skills/github/" "gh repo view --json owner,name" \
      --include='*.md'
    ;;
esac

test_summary
