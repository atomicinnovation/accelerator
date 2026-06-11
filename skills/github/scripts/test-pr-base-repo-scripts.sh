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
  1 | 2 | 3 | 4 | 5 | 6 | final) : ;;
  *)
    echo "unknown PHASE: $phase (expected 1-6 or final)" >&2
    exit 2
    ;;
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
  printf '%s' "$2" >"$1"
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
new_case
T=$CASE_DIR
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
new_case
T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"url":"https://github.com/acme/app/pull/119"}'
out=$(GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$T/stderr") || true
assert_eq "test 3: stdout is acme/app" "acme/app" "$out"

# ---------------------------------------------------------------
# Test 4: Upstream URL parses to upstream coords. Renamed from
# "cross-fork resolves to upstream" — the stubbed harness cannot model
# real cross-fork behaviour (the stub dispatches only on `$1 $2`), so
# this test only verifies the URL-parsing branch with an
# upstream-shaped payload. The real cross-fork-safety property is
# covered by a manual-verification step in the plan.
echo ""
echo "--- test 4: upstream URL parses to upstream coords ---"
new_case
T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" \
  '{"url":"https://github.com/upstream-org/upstream-repo/pull/119"}'
out=$(GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$T/stderr") || true
assert_eq "test 4: stdout matches the URL's upstream coords" \
  "upstream-org/upstream-repo" "$out"

# ---------------------------------------------------------------
# Test 4b: GHE host parses correctly — locks the host-agnostic
# property the regex permits.
echo ""
echo "--- test 4b: GHE host parses correctly ---"
new_case
T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" \
  '{"url":"https://github.acme.corp/team-a/repo/pull/119"}'
out=$(GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$T/stderr") || true
assert_eq "test 4b: GHE host extracts owner/repo correctly" \
  "team-a/repo" "$out"

# ---------------------------------------------------------------
# Test 4c: percent-encoded chars in owner rejected — locks the
# tightened charset against percent-encoded smuggling.
echo ""
echo "--- test 4c: percent-encoded chars in owner rejected ---"
new_case
T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"url":"https://github.com/ac%2fme/app/pull/119"}'
stderr_capture="$T/stderr"
rc=0
GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 4c: percent-encoded owner rejected with exit 1" 1 "$rc"
assert_contains "test 4c: stderr names URL-extraction failure" \
  "$(cat "$stderr_capture")" "could not extract owner/repo from url"

# ---------------------------------------------------------------
# Test 4d: leading-dot repo (.github) accepted.
echo ""
echo "--- test 4d: leading-dot repo (.github) accepted ---"
new_case
T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"url":"https://github.com/acme/.github/pull/119"}'
out=$(GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$T/stderr") || true
assert_eq "test 4d: leading-dot repo accepted" \
  "acme/.github" "$out"

# ---------------------------------------------------------------
# Test 4e: percent-encoded chars in repo rejected — locks the charset
# on the repo side (test 4c only covered the owner side).
echo ""
echo "--- test 4e: percent-encoded chars in repo rejected ---"
new_case
T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"url":"https://github.com/acme/app%2fevil/pull/119"}'
stderr_capture="$T/stderr"
rc=0
GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 4e: percent-encoded repo rejected with exit 1" 1 "$rc"
assert_contains "test 4e: stderr names URL-extraction failure" \
  "$(cat "$stderr_capture")" "could not extract owner/repo from url"

# ---------------------------------------------------------------
# Test 5: Recorded argv shape (exact full-line match).
echo ""
echo "--- test 5: argv shape ---"
new_case
T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"url":"https://github.com/acme/app/pull/119"}'
GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 >/dev/null 2>"$T/stderr" || true
# shellcheck disable=SC2154 # exported by setup_gh_stub and written by the gh stub before use
argv=$(cat "$GH_ARGV_LOG")
assert_eq "test 5: argv is exactly 'pr view 119 --json url'" \
  "pr view 119 --json url" "$argv"

# ---------------------------------------------------------------
# Test 6: Resolver failure — stderr preserved, conditional hint fires.
echo ""
echo "--- test 6: failure with no-default-remote replay+hint ---"
new_case
T=$CASE_DIR
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
new_case
T=$CASE_DIR
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
# Test 8: Malformed URL guard — missing owner segment.
echo ""
echo "--- test 8: malformed URL guard ---"
new_case
T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
# Missing the owner segment — extraction must yield empty owner and exit 1.
write_file "$payload" '{"url":"https://github.com//app/pull/119"}'
stderr_capture="$T/stderr"
stdout_capture="$T/stdout"
rc=0
GH_PR_VIEW_OUT="$payload" \
  "$SCRIPT" 119 >"$stdout_capture" 2>"$stderr_capture" || rc=$?
assert_eq "test 8: malformed-URL exits 1" 1 "$rc"
if grep -qE "^/" "$stdout_capture"; then
  echo "  FAIL: test 8: must NOT print '/app' to stdout"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: test 8: stdout does not smuggle malformed coords"
  PASS=$((PASS + 1))
fi
assert_contains "test 8: stderr names URL-extraction failure" \
  "$(cat "$stderr_capture")" "could not extract owner/repo from url"

# ---------------------------------------------------------------
# Test 9: Truncated URL guard — missing repo segment.
echo ""
echo "--- test 9: truncated URL guard ---"
new_case
T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
# Missing the repo segment — extraction must yield empty name and exit 1.
write_file "$payload" '{"url":"https://github.com/acme/pull/119"}'
stderr_capture="$T/stderr"
stdout_capture="$T/stdout"
rc=0
GH_PR_VIEW_OUT="$payload" \
  "$SCRIPT" 119 >"$stdout_capture" 2>"$stderr_capture" || rc=$?
assert_eq "test 9: truncated-URL exits 1" 1 "$rc"
if grep -qE "/$" "$stdout_capture"; then
  echo "  FAIL: test 9: must NOT print 'acme/' to stdout"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: test 9: stdout does not smuggle truncated coords"
  PASS=$((PASS + 1))
fi
assert_contains "test 9: stderr names URL-extraction failure" \
  "$(cat "$stderr_capture")" "could not extract owner/repo from url"

# ---------------------------------------------------------------
# Test 10: Missing jq preflight — exit 2 with remediation.
echo ""
echo "--- test 10: missing jq preflight ---"
new_case
T=$CASE_DIR
setup_gh_stub "$T"
# Invoke bash directly with a scoped PATH so the script's #!/usr/bin/env
# shebang isn't required to find bash on the stripped PATH. The PATH
# passed to the subshell only contains the fake-gh bin-dir, so the
# script's `command -v jq` preflight fails.
stderr_capture="$T/stderr"
rc=0
PATH="$T/bin" "$BASH_BIN" "$SCRIPT" 119 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 10: missing jq exits 2" 2 "$rc"
assert_contains "test 10: stderr names jq" \
  "$(cat "$stderr_capture")" "jq is required"

# ---------------------------------------------------------------
# Test 11: Missing url field → exit 1.
echo ""
echo "--- test 11: missing url field ---"
new_case
T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{}'
stderr_capture="$T/stderr"
rc=0
GH_PR_VIEW_OUT="$payload" \
  "$SCRIPT" 119 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 11: missing url exits 1" 1 "$rc"
assert_contains "test 11: stderr names empty/null url" \
  "$(cat "$stderr_capture")" "url was empty/null"

# ---------------------------------------------------------------
# Test 12: Non-JSON stdout → clear error rather than opaque jq parse.
echo ""
echo "--- test 12: non-JSON output ---"
new_case
T=$CASE_DIR
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
  1 | 2 | 3) skip_test "test 22" "deferred — guards Phase 4" ;;
  4 | 5 | 6 | final)
    assert_grep_empty "test 22" \
      "$PLUGIN_ROOT/skills/" "gh pr edit" \
      --include='*.md'
    ;;
esac

# Test 23 — regression guard against cross-fork-unsafe resolver.
case "$phase" in
  1 | 2 | 3 | 4 | 5) skip_test "test 23" "deferred — guards Phase 6" ;;
  6 | final)
    assert_grep_empty "test 23" \
      "$PLUGIN_ROOT/skills/github/" "gh repo view --json owner,name" \
      --include='*.md'
    ;;
esac

# Test 24 — regression guard against the broken `--json` field that
# work item 0071 fixed.
#
# Background: gh 2.65.0 does not allowlist the legacy field in
# `gh pr view --json`, so any reappearance of that flag combination
# under skills/github/ would re-break the describe-pr / review-pr /
# respond-to-pr post-step on gh 2.65.0. URL derivation is the
# replacement (see pr-base-repo.sh's header and meta/work/0071-*.md).
#
# Unconditional (no PHASE gate): Phases 1+2 land atomically, so the
# staged-landing rationale that motivated PHASE gating for tests 22
# and 23 does not apply here. The `-F --` extras force fixed-string
# matching and terminate option processing before grep parses the
# pattern's leading `--json` as a long option.
#
# The legacy field name is bound to a variable and the pattern
# constructed at runtime so the literal flag+field pair never appears
# verbatim in this file (otherwise the guard would self-match).
LEGACY_FIELD="baseRepository"
LEGACY_PATTERN="--json $LEGACY_FIELD"
assert_grep_empty "test 24 (regression guard for 0071 — see pr-base-repo.sh header)" \
  "$PLUGIN_ROOT/skills/github/" "$LEGACY_PATTERN" \
  -F --

test_summary
