#!/usr/bin/env bash
set -euo pipefail

# Tests for linear-graphql.sh
# Run: bash skills/integrations/linear/scripts/test-linear-graphql.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/linear-graphql.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-linear-server.py"

TEST_TOKEN="lin_api_SENTINEL_xyz123"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator"
  echo "$d"
}

REPO=$(setup_repo)

# ---------------------------------------------------------------------------
# Mock server lifecycle
# ---------------------------------------------------------------------------

MOCK_PID=""
MOCK_URL_FILE=""
MOCK_URL=""
MOCK_ERRORS_FILE=""
MOCK_BODIES_FILE=""

start_mock() {
  local scenario="$1"
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  MOCK_ERRORS_FILE=$(mktemp "$TMPDIR_BASE/errs-XXXXXX")
  MOCK_BODIES_FILE=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
  python3 "$MOCK_SERVER" --scenario "$scenario" --url-file "$MOCK_URL_FILE" \
    --captured-errors-file "$MOCK_ERRORS_FILE" \
    --captured-bodies-file "$MOCK_BODIES_FILE" &
  MOCK_PID=$!
  local i=0
  while [ ! -s "$MOCK_URL_FILE" ] && [ $i -lt 50 ]; do
    sleep 0.1
    i=$((i + 1))
  done
  if [ ! -s "$MOCK_URL_FILE" ]; then
    echo "ERROR: mock server did not start within 5s" >&2
    kill "$MOCK_PID" 2>/dev/null || true
    exit 1
  fi
  MOCK_URL=$(cat "$MOCK_URL_FILE")
}

stop_mock() {
  if [ -n "$MOCK_PID" ]; then
    kill "$MOCK_PID" 2>/dev/null || true
    wait "$MOCK_PID" 2>/dev/null || true
    MOCK_PID=""
  fi
  [ -n "$MOCK_URL_FILE" ] && {
    rm -f "$MOCK_URL_FILE"
    MOCK_URL_FILE=""
  }
  MOCK_URL=""
}

gql() {
  cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$SCRIPT" "$@"
}

# ---------------------------------------------------------------------------
# File-based sleep counter (works across child processes; no real sleeps)
# ---------------------------------------------------------------------------

SLEEP_COUNT_FILE=$(mktemp "$TMPDIR_BASE/sleepcount-XXXXXX")
SLEEP_ARGS_FILE=$(mktemp "$TMPDIR_BASE/sleepargs-XXXXXX")
export SLEEP_COUNT_FILE SLEEP_ARGS_FILE

test_record_sleep() {
  local n
  n=$(cat "$SLEEP_COUNT_FILE" 2>/dev/null || echo 0)
  echo $((n + 1)) >"$SLEEP_COUNT_FILE"
  echo "$1" >>"$SLEEP_ARGS_FILE"
}
export -f test_record_sleep

reset_sleep_counter() {
  echo 0 >"$SLEEP_COUNT_FILE"
  : >"$SLEEP_ARGS_FILE"
}
get_sleep_count() { cat "$SLEEP_COUNT_FILE" 2>/dev/null || echo 0; }
get_sleep_arg() { sed -n "${1}p" "$SLEEP_ARGS_FILE" 2>/dev/null || echo ""; }

assert_mock_clean() {
  local test_name="$1"
  local errs
  errs=$(cat "$MOCK_ERRORS_FILE" 2>/dev/null || echo "[]")
  if [ "$errs" = "[]" ] || [ -z "$errs" ]; then
    echo "  PASS: $test_name (mock recorded no errors)"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name (mock recorded errors: $errs)"
    FAIL=$((FAIL + 1))
  fi
}

VIEWER_Q='query { viewer { id name } }'
# shellcheck disable=SC2016 # $cursor is a GraphQL variable, not a shell expansion
PAGE_Q='query($cursor: String) { issues(after: $cursor) { nodes { id } pageInfo { hasNextPage endCursor } } }'

# ============================================================
echo "=== Case 1: 200 — body on stdout, exit 0 ==="
echo ""
start_mock "$SCENARIOS/viewer-200.json"
RESULT=$(gql --query "$VIEWER_Q")
stop_mock
assert_contains "200 body has viewer id" "$RESULT" '"viewer-uuid-1"'

start_mock "$SCENARIOS/viewer-200.json"
assert_exit_code "200 exits 0" 0 gql --query "$VIEWER_Q"
stop_mock
echo ""

# ============================================================
echo "=== Case 2: HTTP 401 — exit 11 ==="
echo ""
start_mock "$SCENARIOS/bearer-401.json"
assert_exit_code "401 exits 11" 11 gql --query "$VIEWER_Q"
stop_mock
echo ""

# ============================================================
echo "=== Case 3: GraphQL auth error in 200 body — exit 11 ==="
echo ""
start_mock "$SCENARIOS/graphql-auth-error-200.json"
ERR=$(gql --query "$VIEWER_Q" 2>&1 >/dev/null || true)
stop_mock
assert_contains "200-body auth error: message on stderr" "$ERR" "Authentication failed"
start_mock "$SCENARIOS/graphql-auth-error-200.json"
assert_exit_code "200-body auth error exits 11" 11 gql --query "$VIEWER_Q"
stop_mock
echo ""

# ============================================================
echo "=== Case 4: complexity 400 — exit 36, names 10,000, no stdout ==="
echo ""
start_mock "$SCENARIOS/complexity-400.json"
OUT=$(gql --query "$VIEWER_Q" 2>/dev/null || true)
stop_mock
assert_eq "complexity: no stdout result" "" "$OUT"
start_mock "$SCENARIOS/complexity-400.json"
ERR=$(gql --query "$VIEWER_Q" 2>&1 >/dev/null || true)
stop_mock
assert_contains "complexity: message names 10,000" "$ERR" "10,000"
start_mock "$SCENARIOS/complexity-400.json"
assert_exit_code "complexity exits 36" 36 gql --query "$VIEWER_Q"
stop_mock
echo ""

# ============================================================
echo "=== Case 5: bad-request 400 (negative control) — exit 34, not retried ==="
echo ""
start_mock "$SCENARIOS/bad-request-400.json"
reset_sleep_counter
assert_exit_code "bad-request exits 34" 34 \
  bash -c "cd '$REPO' && ACCELERATOR_LINEAR_TOKEN='$TEST_TOKEN' ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST='$MOCK_URL' \
    LINEAR_RETRY_SLEEP_FN=test_record_sleep bash '$SCRIPT' --query 'query{x}'"
stop_mock
assert_eq "bad-request not retried (0 sleeps)" "0" "$(get_sleep_count)"
echo ""

# ============================================================
echo "=== Case 6: bad-request-mentions-10000 — RATELIMITED (35), not complexity ==="
echo ""
start_mock "$SCENARIOS/bad-request-mentions-10000.json"
assert_exit_code "10,000-in-ratelimit classifies as 35" 35 \
  bash -c "cd '$REPO' && ACCELERATOR_LINEAR_TOKEN='$TEST_TOKEN' ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST='$MOCK_URL' \
    LINEAR_RETRY_SLEEP_FN=test_record_sleep bash '$SCRIPT' --query 'query{x}'"
stop_mock
echo ""

# ============================================================
echo "=== Case 7: graphql-errors-200 (non-auth) — terminal 34, no retry, msg ==="
echo ""
start_mock "$SCENARIOS/graphql-errors-200.json"
reset_sleep_counter
ERR=$(bash -c "cd '$REPO' && ACCELERATOR_LINEAR_TOKEN='$TEST_TOKEN' ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST='$MOCK_URL' \
  LINEAR_RETRY_SLEEP_FN=test_record_sleep bash '$SCRIPT' --query 'query{x}'" 2>&1 >/dev/null || true)
stop_mock
assert_contains "200-body error: message on stderr" "$ERR" "Internal error processing request"
assert_eq "200-body error not retried (0 sleeps)" "0" "$(get_sleep_count)"
echo ""

# ============================================================
echo "=== Case 8: RATELIMITED then 200 — exit 0, body, exactly one sleep ==="
echo ""
start_mock "$SCENARIOS/ratelimited-400-then-200.json"
reset_sleep_counter
RESULT=$(LINEAR_RETRY_SLEEP_FN=test_record_sleep gql --query "$VIEWER_Q")
stop_mock
assert_eq "then-200: exactly one sleep" "1" "$(get_sleep_count)"
assert_contains "then-200: 200 body on stdout" "$RESULT" '"viewer-uuid-1"'
echo ""

# ============================================================
echo "=== Case 9: RATELIMITED exhausted — exit 35, backoff ~30s from reset ==="
echo ""
RESET_MS=$((($(date +%s) + 30) * 1000))
TMP_SCEN=$(mktemp "$TMPDIR_BASE/scen-XXXXXX.json")
sed "s/__RESET_MS__/$RESET_MS/" "$SCENARIOS/ratelimited-exhausted.json.tmpl" >"$TMP_SCEN"
start_mock "$TMP_SCEN"
reset_sleep_counter
assert_exit_code "ratelimited exhausted exits 35" 35 \
  bash -c "cd '$REPO' && ACCELERATOR_LINEAR_TOKEN='$TEST_TOKEN' ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST='$MOCK_URL' \
    LINEAR_RETRY_SLEEP_FN=test_record_sleep bash '$SCRIPT' --query 'query{x}'"
stop_mock
rm -f "$TMP_SCEN"
assert_eq "exhausted: 3 sleeps" "3" "$(get_sleep_count)"
SV=$(get_sleep_arg 1)
SV="${SV:-0}"
if [ "$SV" -ge 28 ] && [ "$SV" -le 32 ]; then
  echo "  PASS: backoff $SV within ±2s of 30 (reset-derived, clamped [1,60])"
  PASS=$((PASS + 1))
else
  echo "  FAIL: backoff $SV not within ±2s of 30"
  FAIL=$((FAIL + 1))
fi
echo ""

# ============================================================
echo "=== Case 10: RATELIMITED no reset header — exp backoff, not tight loop ==="
echo ""
start_mock "$SCENARIOS/ratelimited-400-no-reset-header.json"
reset_sleep_counter
assert_exit_code "no-reset exhausted exits 35" 35 \
  bash -c "cd '$REPO' && ACCELERATOR_LINEAR_TOKEN='$TEST_TOKEN' ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST='$MOCK_URL' \
    LINEAR_RETRY_SLEEP_FN=test_record_sleep bash '$SCRIPT' --query 'query{x}'"
stop_mock
S3=$(get_sleep_arg 3)
S3="${S3:-0}"
if [ "$S3" -ge 2 ]; then
  echo "  PASS: third backoff $S3 >= 2 (exponential, not a 1s tight loop)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: third backoff $S3 < 2 (looks like a tight loop)"
  FAIL=$((FAIL + 1))
fi
echo ""

# ============================================================
echo "=== Case 11: pagination 3×50 — 150 nodes, hasNextPage false ==="
echo ""
start_mock "$SCENARIOS/paginate-3x50.json"
RESULT=$(gql --query "$PAGE_Q" --paginate .data.issues)
stop_mock
COUNT=$(printf '%s' "$RESULT" | jq '.data.issues.nodes | length')
assert_eq "3×50 returns 150 nodes" "150" "$COUNT"
HN=$(printf '%s' "$RESULT" | jq -r '.data.issues.pageInfo.hasNextPage')
assert_eq "3×50 stops with hasNextPage false" "false" "$HN"
TR=$(printf '%s' "$RESULT" | jq -r '.data.issues.truncated')
assert_eq "3×50 truncated false" "false" "$TR"
echo ""

# ============================================================
echo "=== Case 12: pagination empty — nodes [] (array), exit 0 ==="
echo ""
start_mock "$SCENARIOS/paginate-zero.json"
RESULT=$(gql --query "$PAGE_Q" --paginate .data.issues)
stop_mock
TYPE=$(printf '%s' "$RESULT" | jq -r '.data.issues.nodes | type')
assert_eq "empty connection: nodes is an array" "array" "$TYPE"
LEN=$(printf '%s' "$RESULT" | jq -r '.data.issues.nodes | length')
assert_eq "empty connection: nodes length 0" "0" "$LEN"
start_mock "$SCENARIOS/paginate-zero.json"
assert_exit_code "empty connection exits 0" 0 gql --query "$PAGE_Q" --paginate .data.issues
stop_mock
echo ""

# ============================================================
echo "=== Case 13: pagination runaway — MAX_PAGES, truncated true, WARN ==="
echo ""
start_mock "$SCENARIOS/paginate-runaway.json"
ERR_FILE=$(mktemp "$TMPDIR_BASE/warn-XXXXXX")
RESULT=$(gql --query "$PAGE_Q" --paginate .data.issues 2>"$ERR_FILE")
stop_mock
COUNT=$(printf '%s' "$RESULT" | jq '.data.issues.nodes | length')
assert_eq "runaway: bounded node count (20 = MAX_PAGES)" "20" "$COUNT"
TR=$(printf '%s' "$RESULT" | jq -r '.data.issues.truncated')
assert_eq "runaway: truncated true" "true" "$TR"
HN=$(printf '%s' "$RESULT" | jq -r '.data.issues.pageInfo.hasNextPage')
assert_neq "runaway: hasNextPage NOT forced false" "false" "$HN"
assert_contains "runaway: WARN on stderr" "$(cat "$ERR_FILE")" "WARN:"
rm -f "$ERR_FILE"
echo ""

# ============================================================
echo "=== Case 14: no credentials — exit 22 ==="
echo ""
NO_CREDS_REPO=$(mktemp -d "$TMPDIR_BASE/nocreds-XXXXXX")
mkdir -p "$NO_CREDS_REPO/.git" "$NO_CREDS_REPO/.accelerator"
assert_exit_code "no token exits 22" 22 \
  bash -c "cd '$NO_CREDS_REPO' && ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST='http://127.0.0.1:1' \
    bash '$SCRIPT' --query 'query{x}'"
echo ""

# ============================================================
echo "=== Case 15: malformed token — exit 27 before any request ==="
echo ""
assert_exit_code "malformed token (quote) exits 27" 27 \
  bash -c "cd '$REPO' && ACCELERATOR_LINEAR_TOKEN='lin_api_\"evil' ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST='http://127.0.0.1:1' \
    bash '$SCRIPT' --query 'query{x}'"
echo ""

# ============================================================
echo "=== Case 16: test override gate — no ACCELERATOR_TEST_MODE → exit 18 ==="
echo ""
ERR=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:9999" \
  bash "$SCRIPT" --query 'query{x}' 2>&1 >/dev/null || true)
assert_contains "override without test mode rejected" "$ERR" "E_TEST_OVERRIDE_REJECTED"
echo ""

# ============================================================
echo "=== Case 17: test override gate — non-loopback URL rejected ==="
echo ""
ERR=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="https://evil.example" \
  bash "$SCRIPT" --query 'query{x}' 2>&1 >/dev/null || true)
assert_contains "non-loopback override rejected" "$ERR" "E_TEST_OVERRIDE_REJECTED"
echo ""

# ============================================================
echo "=== Case 18: connection refused — exit 21 ==="
echo ""
assert_exit_code "refused exits 21" 21 \
  bash -c "cd '$REPO' && ACCELERATOR_LINEAR_TOKEN='$TEST_TOKEN' ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST='http://127.0.0.1:1' \
    bash '$SCRIPT' --query 'query{x}'"
echo ""

# ============================================================
echo "=== Case 19: token absent from process listing ==="
echo ""
start_mock "$SCENARIOS/viewer-slow-200.json"
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
  bash "$SCRIPT" --query "$VIEWER_Q" >/dev/null 2>&1) &
req_pid=$!
sleep 0.3
# The token is piped via curl --config -, so it must never appear in argv.
ps_out=$(ps -o args= -p "$req_pid" 2>/dev/null || echo "")
wait "$req_pid" 2>/dev/null || true
stop_mock
assert_not_contains "token not in ps args" "$ps_out" "$TEST_TOKEN"
echo ""

# ============================================================
echo "=== Case 20: LINEAR_RETRY_SLEEP_FN rejected without test mode ==="
echo ""
WARN=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  LINEAR_RETRY_SLEEP_FN=test_record_sleep \
  bash "$SCRIPT" --query 'query{x}' 2>&1 >/dev/null || true)
assert_contains "no-test-mode hook rejected" "$WARN" "E_TEST_HOOK_REJECTED"
echo ""

# ============================================================
test_summary
