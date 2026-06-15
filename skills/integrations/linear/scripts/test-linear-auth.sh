#!/usr/bin/env bash
set -euo pipefail

# Tests for linear-auth.sh / linear-auth-cli.sh
# Run: bash skills/integrations/linear/scripts/test-linear-auth.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

AUTH_CLI="$SCRIPT_DIR/linear-auth-cli.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator"
  echo "$d"
}

write_team_config() {
  local repo="$1" extra_line="${2:-}"
  {
    echo "---"
    if [ -n "$extra_line" ]; then
      echo "linear:"
      printf '%s\n' "$extra_line"
    fi
    echo "---"
  } >"$repo/.accelerator/config.md"
}

write_local_config() {
  local repo="$1" extra_line="${2:-}"
  {
    echo "---"
    if [ -n "$extra_line" ]; then
      echo "linear:"
      printf '%s\n' "$extra_line"
    fi
    echo "---"
  } >"$repo/.accelerator/config.local.md"
  chmod 600 "$repo/.accelerator/config.local.md"
}

# ---------------------------------------------------------------------------
echo "=== token source precedence ==="
echo ""

echo "Test 1: ACCELERATOR_LINEAR_TOKEN env var wins over config sources"
REPO=$(setup_repo)
write_team_config "$REPO" "  token: team-token"
OUTPUT=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN=env-token bash "$AUTH_CLI")
assert_contains "token=env-token in stdout" "$OUTPUT" "token=env-token"

echo "Test 2: ACCELERATOR_LINEAR_TOKEN_CMD resolves with trailing whitespace trimmed"
REPO=$(setup_repo)
write_team_config "$REPO"
OUTPUT=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN_CMD='printf "tok-cmd\r\n\n\n  "' bash "$AUTH_CLI")
assert_eq "token=tok-cmd (stripped)" "token=tok-cmd" "$(printf '%s\n' "$OUTPUT" | grep '^token=')"

echo "Test 3: config.local.md linear.token wins over config.md linear.token"
REPO=$(setup_repo)
write_team_config "$REPO" "  token: team-token"
write_local_config "$REPO" "  token: local-token"
OUTPUT=$(cd "$REPO" && bash "$AUTH_CLI")
assert_contains "local token wins" "$OUTPUT" "token=local-token"

echo "Test 4: config.local.md linear.token_cmd wins over config.md linear.token"
REPO=$(setup_repo)
write_team_config "$REPO" "  token: team-token"
write_local_config "$REPO" "  token_cmd: echo cmd-token"
OUTPUT=$(cd "$REPO" && bash "$AUTH_CLI")
assert_contains "local token_cmd wins" "$OUTPUT" "token=cmd-token"

echo "Test 5: config.md linear.token used when nothing else set"
REPO=$(setup_repo)
write_team_config "$REPO" "  token: shared-token"
OUTPUT=$(cd "$REPO" && bash "$AUTH_CLI")
assert_contains "shared token" "$OUTPUT" "token=shared-token"

echo "Test 6: config.md linear.token_cmd ignored — E_TOKEN_CMD_FROM_SHARED_CONFIG + E_NO_TOKEN"
REPO=$(setup_repo)
write_team_config "$REPO" "  token_cmd: echo ignored"
EXIT_CODE=0
STDERR=$(cd "$REPO" && bash "$AUTH_CLI" 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_contains "E_TOKEN_CMD_FROM_SHARED_CONFIG in stderr" "$STDERR" "E_TOKEN_CMD_FROM_SHARED_CONFIG"
assert_contains "E_NO_TOKEN in stderr" "$STDERR" "E_NO_TOKEN"
if [ "$EXIT_CODE" -ne 0 ]; then
  echo "  PASS: exits non-zero"
  PASS=$((PASS + 1))
else
  echo "  FAIL: expected non-zero exit"
  FAIL=$((FAIL + 1))
fi

echo "Test 7: empty resolution exits 24 (E_NO_TOKEN), nothing on stdout"
REPO=$(setup_repo)
write_team_config "$REPO"
EXIT_CODE=0
STDOUT=$(cd "$REPO" && bash "$AUTH_CLI" 2>/dev/null) || EXIT_CODE=$?
STDERR=$(cd "$REPO" && bash "$AUTH_CLI" 2>&1 1>/dev/null) || true
assert_eq "exit code 24" "24" "$EXIT_CODE"
assert_contains "E_NO_TOKEN in stderr" "$STDERR" "E_NO_TOKEN"
assert_eq "stdout is empty" "" "$STDOUT"

echo "Test 8: stdout has exactly one line — token= — no site/email, no logging"
REPO=$(setup_repo)
write_team_config "$REPO" "  token: tok"
OUTPUT=$(cd "$REPO" && bash "$AUTH_CLI")
LINE_COUNT=$(printf '%s\n' "$OUTPUT" | wc -l | tr -d ' ')
assert_eq "exactly 1 line" "1" "$LINE_COUNT"
assert_contains "token= line present" "$OUTPUT" "token="
assert_not_contains "no site= line" "$OUTPUT" "site="
assert_not_contains "no email= line" "$OUTPUT" "email="

echo "Test 9: token redaction — sentinel absent from --debug stderr, present on stdout"
REPO=$(setup_repo)
write_team_config "$REPO" "  token: tok-SENTINEL-xyz123"
DEBUG_STDERR=$(cd "$REPO" && bash "$AUTH_CLI" --debug 2>&1 1>/dev/null || true)
assert_not_contains "sentinel absent from debug stderr" "$DEBUG_STDERR" "tok-SENTINEL-xyz123"
DEBUG_STDOUT=$(cd "$REPO" && bash "$AUTH_CLI" --debug 2>/dev/null)
assert_contains "sentinel on stdout token= line" "$DEBUG_STDOUT" "token=tok-SENTINEL-xyz123"

echo "Test 10: token_cmd failure exits E_TOKEN_CMD_FAILED"
REPO=$(setup_repo)
write_team_config "$REPO"
write_local_config "$REPO" "  token_cmd: exit 1"
EXIT_CODE=0
STDERR=$(cd "$REPO" && bash "$AUTH_CLI" 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_contains "E_TOKEN_CMD_FAILED in stderr" "$STDERR" "E_TOKEN_CMD_FAILED"
assert_eq "exit code 25" "25" "$EXIT_CODE"

echo ""

# ---------------------------------------------------------------------------
echo "=== config.local.md permissions — fail-closed ==="
echo ""

echo "Test 11: mode 0644 — exits 29, no token on stdout"
REPO=$(setup_repo)
write_team_config "$REPO"
write_local_config "$REPO" "  token: local-token"
chmod 644 "$REPO/.accelerator/config.local.md"
EXIT_CODE=0
STDERR=$(cd "$REPO" && bash "$AUTH_CLI" 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_eq "exit code 29" "29" "$EXIT_CODE"
assert_contains "E_LOCAL_PERMS_INSECURE in stderr" "$STDERR" "E_LOCAL_PERMS_INSECURE"
STDOUT=$(cd "$REPO" && bash "$AUTH_CLI" 2>/dev/null) || true
assert_eq "no token leaked on stdout" "" "$STDOUT"

echo ""

# ---------------------------------------------------------------------------
echo "=== malformed-token guard (E_TOKEN_MALFORMED, 27) ==="
echo ""

echo "Test 12: env token containing a double-quote → exit 27, nothing on stdout"
REPO=$(setup_repo)
write_team_config "$REPO"
EXIT_CODE=0
STDOUT=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN='lin_api_"evil' bash "$AUTH_CLI" 2>/dev/null) || EXIT_CODE=$?
STDERR=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN='lin_api_"evil' bash "$AUTH_CLI" 2>&1 1>/dev/null) || true
assert_eq "quote token exit 27" "27" "$EXIT_CODE"
assert_contains "E_TOKEN_MALFORMED in stderr" "$STDERR" "E_TOKEN_MALFORMED"
assert_eq "no token leaked on stdout" "" "$STDOUT"

echo "Test 13: env token containing a backslash → exit 27"
REPO=$(setup_repo)
write_team_config "$REPO"
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN='lin_api_ev\il' bash "$AUTH_CLI" >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "backslash token exit 27" "27" "$EXIT_CODE"

echo "Test 14: token_cmd emitting an embedded newline → exit 27 (guard runs after trim)"
REPO=$(setup_repo)
write_team_config "$REPO"
write_local_config "$REPO" '  token_cmd: printf "lin_api_a\nlin_api_b"'
EXIT_CODE=0
STDERR=$(cd "$REPO" && bash "$AUTH_CLI" 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_eq "embedded-newline token exit 27" "27" "$EXIT_CODE"
assert_contains "E_TOKEN_MALFORMED for newline" "$STDERR" "E_TOKEN_MALFORMED"

echo "Test 15: a clean lin_api_ token passes the guard"
REPO=$(setup_repo)
write_team_config "$REPO" "  token: lin_api_cleanvalue123"
OUTPUT=$(cd "$REPO" && bash "$AUTH_CLI")
assert_eq "clean token resolves" "token=lin_api_cleanvalue123" "$OUTPUT"

echo ""
test_summary
