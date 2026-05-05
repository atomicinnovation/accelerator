#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-request.sh
# Run: bash skills/integrations/jira/scripts/test-jira-request.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/jira-request.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-jira-server.py"

# Sentinel credentials
TEST_TOKEN="tok-SENTINEL-xyz123"
TEST_SITE="example"
TEST_EMAIL="test@example.com"

# ---------------------------------------------------------------------------
# Fake repo setup
# ---------------------------------------------------------------------------

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local d; d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator"
  cat > "$d/.accelerator/config.md" <<ENDCONFIG
---
jira:
  site: $TEST_SITE
  email: $TEST_EMAIL
---
ENDCONFIG
  echo "$d"
}

REPO=$(setup_repo)

# ---------------------------------------------------------------------------
# Mock server lifecycle helpers
# ---------------------------------------------------------------------------

MOCK_PID=""
MOCK_URL_FILE=""
MOCK_URL=""

start_mock() {
  local scenario="$1"
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  python3 "$MOCK_SERVER" --scenario "$scenario" --url-file "$MOCK_URL_FILE" &
  MOCK_PID=$!

  local i=0
  while [ ! -s "$MOCK_URL_FILE" ] && [ $i -lt 50 ]; do
    sleep 0.1; i=$((i + 1))
  done
  if [ ! -s "$MOCK_URL_FILE" ]; then
    echo "ERROR: mock server did not start within 5s" >&2
    kill "$MOCK_PID" 2>/dev/null || true; exit 1
  fi
  MOCK_URL=$(cat "$MOCK_URL_FILE")
}

stop_mock() {
  if [ -n "$MOCK_PID" ]; then
    kill "$MOCK_PID" 2>/dev/null || true
    wait "$MOCK_PID" 2>/dev/null || true
    MOCK_PID=""
  fi
  [ -n "$MOCK_URL_FILE" ] && { rm -f "$MOCK_URL_FILE"; MOCK_URL_FILE=""; }
  MOCK_URL=""
}

# Run jira-request.sh from the fake repo, injecting test credentials and mock URL
req() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$SCRIPT" "$@"
}

# ---------------------------------------------------------------------------
# Test sleep counter (file-based so it works across child processes)
# ---------------------------------------------------------------------------

SLEEP_COUNT_FILE=$(mktemp "$TMPDIR_BASE/sleepcount-XXXXXX")
SLEEP_ARGS_FILE=$(mktemp "$TMPDIR_BASE/sleepargs-XXXXXX")
export SLEEP_COUNT_FILE SLEEP_ARGS_FILE

test_record_sleep() {
  local n; n=$(cat "$SLEEP_COUNT_FILE" 2>/dev/null || echo 0)
  echo $(( n + 1 )) > "$SLEEP_COUNT_FILE"
  echo "$1" >> "$SLEEP_ARGS_FILE"
}
export -f test_record_sleep

reset_sleep_counter() {
  echo 0 > "$SLEEP_COUNT_FILE"
  : > "$SLEEP_ARGS_FILE"
}

get_sleep_count()    { cat "$SLEEP_COUNT_FILE" 2>/dev/null || echo 0; }
get_sleep_arg()      { sed -n "${1}p" "$SLEEP_ARGS_FILE" 2>/dev/null || echo ""; }

# ============================================================
echo "=== Case 1: GET 200 — body on stdout, exit 0 ==="
echo ""

start_mock "$SCENARIOS/get-200.json"
RESULT=$(req GET /rest/api/3/myself)
stop_mock
assert_contains "GET 200 body has accountId" "$RESULT" '"redacted-account-id"'

start_mock "$SCENARIOS/get-200.json"
assert_exit_code "GET 200 exits 0" 0 req GET /rest/api/3/myself
stop_mock
echo ""

# ============================================================
echo "=== Case 2: POST 200 with JSON body ==="
echo ""

start_mock "$SCENARIOS/post-200.json"
RESULT=$(req POST /rest/api/3/issue --json '{"fields":{"project":{"key":"ENG"}}}')
stop_mock
assert_contains "POST 200 returns key" "$RESULT" '"ENG-1"'
echo ""

# ============================================================
echo "=== Case 3: POST multipart — X-Atlassian-Token: no-check header ==="
echo ""

start_mock "$SCENARIOS/post-multipart-200.json"
tmpatt=$(mktemp "$TMPDIR_BASE/attach-XXXXXX")
echo "attachment" > "$tmpatt"
RESULT=$(req POST /rest/api/3/issue/ENG-1/attachments --multipart "file=@$tmpatt")
stop_mock
assert_contains "POST multipart returns []" "$RESULT" "[]"
echo ""

# ============================================================
echo "=== Case 4: 401 — exit 11, body on stderr ==="
echo ""

start_mock "$SCENARIOS/error-401.json"
ERR=$(req GET /rest/api/3/myself 2>&1 >/dev/null || true)
stop_mock
assert_contains "401 error body on stderr" "$ERR" "401"

start_mock "$SCENARIOS/error-401.json"
assert_exit_code "401 exits 11" 11 req GET /rest/api/3/myself
stop_mock
echo ""

# ============================================================
echo "=== Case 5: 403 — exit 12 ==="
echo ""
start_mock "$SCENARIOS/error-403.json"
assert_exit_code "403 exits 12" 12 req GET /rest/api/3/myself
stop_mock
echo ""

# ============================================================
echo "=== Case 6: 404 — exit 13 ==="
echo ""
start_mock "$SCENARIOS/error-404.json"
assert_exit_code "404 exits 13" 13 req GET /rest/api/3/myself
stop_mock
echo ""

# ============================================================
echo "=== Case 7: 410 — exit 14 ==="
echo ""
start_mock "$SCENARIOS/error-410.json"
assert_exit_code "410 exits 14" 14 req GET /rest/api/3/myself
stop_mock
echo ""

# ============================================================
echo "=== Case 8: 429 Retry-After delta-seconds → 200 ==="
echo ""

start_mock "$SCENARIOS/retry-after-delta.json"
reset_sleep_counter
RESULT=$(JIRA_RETRY_SLEEP_FN=test_record_sleep req GET /rest/api/3/myself)
stop_mock
assert_eq "delta retry: exactly one sleep" "1" "$(get_sleep_count)"
assert_eq "delta retry: sleep = 1s" "1" "$(get_sleep_arg 1)"
assert_contains "delta retry: 200 body" "$RESULT" '"redacted-account-id"'
echo ""

# ============================================================
echo "=== Case 9: 429 Retry-After HTTP-date (future) → 200 ==="
echo ""

# Generate date 2 seconds in the future (macOS: date -v; GNU: date -d)
future_date=$(LC_ALL=C date -v +2S "%a, %d %b %Y %H:%M:%S GMT" 2>/dev/null \
  || LC_ALL=C date -d "+2 seconds" +"%a, %d %b %Y %H:%M:%S GMT" 2>/dev/null \
  || echo "")

if [ -n "$future_date" ]; then
  tmp_scen=$(mktemp "$TMPDIR_BASE/scen-XXXXXX.json")
  sed "s/__HTTP_DATE_FUTURE__/$future_date/" "$SCENARIOS/retry-after-http-date.json.tmpl" > "$tmp_scen"

  start_mock "$tmp_scen"
  reset_sleep_counter
  RESULT=$(JIRA_RETRY_SLEEP_FN=test_record_sleep req GET /rest/api/3/myself)
  stop_mock
  rm -f "$tmp_scen"

  assert_eq "http-date retry: one sleep" "1" "$(get_sleep_count)"
  sv="$(get_sleep_arg 1)"; sv="${sv:-0}"
  if [ "$sv" -ge 1 ] && [ "$sv" -le 60 ]; then
    echo "  PASS: http-date retry: sleep $sv in [1,60]"; PASS=$((PASS+1))
  else
    echo "  FAIL: http-date retry: sleep $sv out of [1,60]"; FAIL=$((FAIL+1))
  fi
else
  echo "  SKIP: http-date retry (date -v not available)"
fi
echo ""

# ============================================================
echo "=== Case 9a: 429 Retry-After past date → clamp to 1s ==="
echo ""

past_date=$(LC_ALL=C date -v -30S "%a, %d %b %Y %H:%M:%S GMT" 2>/dev/null \
  || LC_ALL=C date -d "-30 seconds" +"%a, %d %b %Y %H:%M:%S GMT" 2>/dev/null \
  || echo "")

if [ -n "$past_date" ]; then
  tmp_scen=$(mktemp "$TMPDIR_BASE/scen-XXXXXX.json")
  sed "s/__HTTP_DATE_PAST__/$past_date/" "$SCENARIOS/retry-after-past.json.tmpl" > "$tmp_scen"

  start_mock "$tmp_scen"
  reset_sleep_counter
  RESULT=$(JIRA_RETRY_SLEEP_FN=test_record_sleep req GET /rest/api/3/myself)
  stop_mock
  rm -f "$tmp_scen"

  assert_eq "past date: one sleep" "1" "$(get_sleep_count)"
  assert_eq "past date: sleep = 1 (floor)" "1" "$(get_sleep_arg 1)"
else
  echo "  SKIP: past date (date -v not available)"
fi
echo ""

# ============================================================
echo "=== Case 9b: 429 malformed Retry-After → jitter + warning ==="
echo ""

start_mock "$SCENARIOS/retry-after-malformed.json"
reset_sleep_counter
WARN=$(JIRA_RETRY_SLEEP_FN=test_record_sleep req GET /rest/api/3/myself 2>&1 >/dev/null || true)
stop_mock
assert_eq "malformed retry: one sleep" "1" "$(get_sleep_count)"
assert_contains "malformed retry: warning on stderr" "$WARN" "malformed Retry-After"
echo ""

# ============================================================
echo "=== Case 9d: 429 tz-naive Retry-After → falls back to jitter ==="
echo ""

start_mock "$SCENARIOS/retry-after-no-tz.json"
reset_sleep_counter
WARN=$(JIRA_RETRY_SLEEP_FN=test_record_sleep req GET /rest/api/3/myself 2>&1 >/dev/null || true)
stop_mock
assert_eq "no-tz retry: one sleep" "1" "$(get_sleep_count)"
assert_contains "no-tz retry: warning on stderr" "$WARN" "malformed Retry-After"
echo ""

# ============================================================
echo "=== Case 10: 429 exhausted (4 attempts, 3 sleeps) — exit 19 ==="
echo ""

start_mock "$SCENARIOS/retry-exhausted.json"
reset_sleep_counter
assert_exit_code "429 exhausted exits 19" 19 \
  bash -c "cd '$REPO' && ACCELERATOR_JIRA_TOKEN='$TEST_TOKEN' ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST='$MOCK_URL' \
    JIRA_RETRY_SLEEP_FN=test_record_sleep bash '$SCRIPT' GET /rest/api/3/myself"
stop_mock
assert_eq "exhausted: 3 sleeps before giving up" "3" "$(get_sleep_count)"
echo ""

# ============================================================
echo "=== Case 11: 500 — exit 20 ==="
echo ""
start_mock "$SCENARIOS/error-500.json"
assert_exit_code "500 exits 20" 20 req GET /rest/api/3/myself
stop_mock
echo ""

# ============================================================
echo "=== Case 12: Network refused — exit 21 ==="
echo ""
assert_exit_code "refused exits 21" 21 \
  bash -c "cd '$REPO' && ACCELERATOR_JIRA_TOKEN='$TEST_TOKEN' \
    ACCELERATOR_TEST_MODE=1 ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST='http://127.0.0.1:1' \
    bash '$SCRIPT' GET /rest/api/3/myself"
echo ""

# ============================================================
echo "=== Case 13: Token absent from process listing ==="
echo ""

start_mock "$SCENARIOS/slow-200.json"
# Start request in background; sample ps while in flight
(cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
  bash "$SCRIPT" GET /rest/api/3/myself > /dev/null 2>&1) &
req_pid=$!
sleep 0.2
ps_out=$(ps -o args= -p "$req_pid" 2>/dev/null || echo "")
wait "$req_pid" 2>/dev/null || true
stop_mock

assert_not_contains "token not in ps args" "$TEST_TOKEN" "$ps_out"
echo ""

# ============================================================
echo "=== Case 14: --debug does not expose token or Authorization ==="
echo ""

start_mock "$SCENARIOS/get-200.json"
DEBUG_OUT=$(JIRA_RETRY_SLEEP_FN="" req GET /rest/api/3/myself --debug 2>&1 >/dev/null || true)
stop_mock

assert_not_contains "--debug: no token in stderr" "$TEST_TOKEN" "$DEBUG_OUT"
assert_not_contains "--debug: no Authorization in stderr" "$DEBUG_OUT" "Authorization:"
echo ""

# ============================================================
echo "=== Case 16: No credentials — exit 22 ==="
echo ""

NO_CREDS_REPO=$(mktemp -d "$TMPDIR_BASE/nocreds-XXXXXX")
mkdir -p "$NO_CREDS_REPO/.git" "$NO_CREDS_REPO/.accelerator"
cat > "$NO_CREDS_REPO/.accelerator/config.md" <<'ENDCONF'
---
jira:
  site: example
  email: test@example.com
---
ENDCONF

assert_exit_code "no token exits 22" 22 \
  bash -c "cd '$NO_CREDS_REPO' && ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST='http://127.0.0.1:1' \
    bash '$SCRIPT' GET /rest/api/3/myself"
echo ""

# ============================================================
echo "=== Case 17: Test override gate — no ACCELERATOR_TEST_MODE ==="
echo ""

ERR=$(cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:9999" \
  bash "$SCRIPT" GET /rest/api/3/myself 2>&1 >/dev/null || true)

assert_contains "override without test mode rejected" "$ERR" "E_TEST_OVERRIDE_REJECTED"
echo ""

# ============================================================
echo "=== Case 18: Test override gate — non-loopback URL rejected ==="
echo ""

ERR=$(cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="https://evil.example" \
  bash "$SCRIPT" GET /rest/api/3/myself 2>&1 >/dev/null || true)

assert_contains "non-loopback override rejected" "$ERR" "E_TEST_OVERRIDE_REJECTED"
echo ""

# ============================================================
echo "=== Case 19: Path validation ==="
echo ""

run_path_check() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
    bash "$SCRIPT" GET "$1" 2>&1 >/dev/null || true
}

assert_contains "absolute URL rejected" "$(run_path_check 'https://evil.example/x')" "E_REQ_BAD_PATH"
assert_contains "literal traversal rejected" "$(run_path_check '/../../etc/passwd')" "E_REQ_BAD_PATH"
assert_contains "embedded traversal rejected" "$(run_path_check '/rest/api/3/issue/../../field')" "E_REQ_BAD_PATH"
assert_contains "single-encoded traversal rejected" "$(run_path_check '/rest/api/3/%2e%2e%2fadmin')" "E_REQ_BAD_PATH"
assert_contains "double-encoded traversal rejected" "$(run_path_check '/rest/api/3/%252e%252e%252fadmin')" "E_REQ_BAD_PATH"
assert_contains "consecutive slashes rejected" "$(run_path_check '/rest/api/3//search')" "E_REQ_BAD_PATH"

# Positive case: legitimate query string accepted
start_mock "$SCENARIOS/get-200.json"
assert_exit_code "legitimate query string accepted" 0 req GET '/rest/api/3/myself?jql=project%20%3D%20ENG'
stop_mock
echo ""

# ============================================================
echo "=== Case 20: Empty 200 body — exit 0, empty stdout ==="
echo ""

start_mock "$SCENARIOS/empty-200.json"
RESULT=$(req GET /rest/api/3/myself)
stop_mock
assert_eq "empty 200 body is empty stdout" "" "$RESULT"
echo ""

# ============================================================
echo "=== Case 21: Non-JSON 200 body — exit 16 ==="
echo ""

start_mock "$SCENARIOS/non-json-200.json"
assert_exit_code "non-JSON 200 exits 16" 16 req GET /rest/api/3/myself
stop_mock
echo ""

# ============================================================
echo "=== Case 23: Unicode body preserved verbatim ==="
echo ""

start_mock "$SCENARIOS/unicode-200.json"
RESULT=$(req GET /rest/api/3/myself)
stop_mock
assert_contains "unicode: CJK preserved" "$RESULT" "测试"
assert_contains "unicode: emoji preserved" "$RESULT" "🚀"
assert_contains "unicode: café preserved" "$RESULT" "café"
echo ""

# ============================================================
echo "=== Case 24: JIRA_RETRY_SLEEP_FN rejected without ACCELERATOR_TEST_MODE ==="
echo ""

# No mock needed — hook warning is emitted before the URL override check (which exits 18)
WARN=$(cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  JIRA_RETRY_SLEEP_FN=test_record_sleep \
  bash "$SCRIPT" GET /rest/api/3/myself 2>&1 >/dev/null || true)
assert_contains "no-test-mode hook rejected" "$WARN" "E_TEST_HOOK_REJECTED"
echo ""

# ============================================================
echo "=== Case 25: JIRA_RETRY_SLEEP_FN allow-list — invalid name rejected ==="
echo ""

start_mock "$SCENARIOS/retry-after-delta.json"
WARN=$(JIRA_RETRY_SLEEP_FN=evil_fn req GET /rest/api/3/myself 2>&1 >/dev/null || true)
stop_mock
assert_contains "invalid hook name rejected" "$WARN" "E_TEST_HOOK_REJECTED"
echo ""

# ============================================================
echo "=== Case 26: Bad JIRA_SITE format — exit 15 ==="
echo ""

BAD_SITE_REPO=$(mktemp -d "$TMPDIR_BASE/badsite-XXXXXX")
mkdir -p "$BAD_SITE_REPO/.git" "$BAD_SITE_REPO/.accelerator"
cat > "$BAD_SITE_REPO/.accelerator/config.md" <<'ENDCONF'
---
jira:
  site: evil.com#
  email: test@example.com
---
ENDCONF

ERR=$(cd "$BAD_SITE_REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" GET /rest/api/3/myself 2>&1 >/dev/null || true)

assert_contains "bad site rejected" "$ERR" "E_BAD_SITE"
assert_exit_code "bad site exits 15" 15 \
  bash -c "cd '$BAD_SITE_REPO' && ACCELERATOR_JIRA_TOKEN='$TEST_TOKEN' \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST='http://127.0.0.1:1' \
    bash '$SCRIPT' GET /rest/api/3/myself"
echo ""

# ============================================================
echo "=== Case 27: HTTP 400 — exits 34, body forwarded to stderr ==="
echo ""

start_mock "$SCENARIOS/error-400.json"
EXIT_400=0
ERR_400=$(cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
  bash "$SCRIPT" POST /rest/api/3/issue 2>&1 >/dev/null) || EXIT_400=$?
stop_mock

assert_eq "HTTP 400 exits 34" "34" "$EXIT_400"
assert_contains "HTTP 400 body forwarded to stderr" "$ERR_400" "Summary is required"
echo ""

# ============================================================
test_summary
