#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-comment-flow.sh
# Run: bash skills/integrations/jira/scripts/test-jira-comment.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/jira-comment-flow.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-jira-server.py"

TEST_TOKEN="tok-SENTINEL-xyz123"
TEST_SITE="example"
TEST_EMAIL="test@example.com"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

# ---------------------------------------------------------------------------
# Repo / mock setup helpers

setup_repo() {
  local d; d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.claude"
  cat > "$d/.claude/accelerator.md" <<ENDCONFIG
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
# Sleep stub for retry tests

_test_comment_sleep_noop() { :; }
export -f _test_comment_sleep_noop

MOCK_PID=""
MOCK_URL_FILE=""
MOCK_URL=""

start_mock() {
  local scenario="$1"
  local captured_bodies_file="${2:-}"
  local captured_urls_file="${3:-}"
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  local mock_args=("--scenario" "$scenario" "--url-file" "$MOCK_URL_FILE")
  [[ -n "$captured_bodies_file" ]] && mock_args+=("--captured-bodies-file" "$captured_bodies_file")
  [[ -n "$captured_urls_file" ]]   && mock_args+=("--captured-urls-file"   "$captured_urls_file")
  python3 "$MOCK_SERVER" "${mock_args[@]}" &
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

comment() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

comment_no_stdin() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    JIRA_BODY_STDIN_IS_TTY_TEST=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

# ---------------------------------------------------------------------------

echo "=== Case 1: --help exits 0 with subcommand listing ==="
echo ""

OUT_1=$(comment --help 2>/dev/null)
assert_contains "help lists add"    "add"    "$OUT_1"
assert_contains "help lists list"   "list"   "$OUT_1"
assert_contains "help lists edit"   "edit"   "$OUT_1"
assert_contains "help lists delete" "delete" "$OUT_1"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 2: no subcommand exits 91 with usage on stderr ==="
echo ""

RC_2=0
comment_no_stdin 2>/tmp/comment-err2.tmp || RC_2=$?
ERR_2=$(cat /tmp/comment-err2.tmp)
assert_eq "no subcommand exits 91" "91" "$RC_2"
assert_contains "E_COMMENT_NO_SUBCOMMAND on stderr" "E_COMMENT_NO_SUBCOMMAND" "$ERR_2"
assert_contains "usage on stderr" "add" "$ERR_2"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 3: unknown subcommand exits 92 ==="
echo ""

RC_3=0
comment_no_stdin frobnicate 2>/tmp/comment-err3.tmp || RC_3=$?
ERR_3=$(cat /tmp/comment-err3.tmp)
assert_eq "bad subcommand exits 92" "92" "$RC_3"
assert_contains "E_COMMENT_BAD_SUBCOMMAND on stderr" "E_COMMENT_BAD_SUBCOMMAND" "$ERR_3"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 4: add with no key exits 93 ==="
echo ""

RC_4=0
comment_no_stdin add 2>/tmp/comment-err4.tmp || RC_4=$?
ERR_4=$(cat /tmp/comment-err4.tmp)
assert_eq "add no key exits 93" "93" "$RC_4"
assert_contains "E_COMMENT_NO_KEY on stderr" "E_COMMENT_NO_KEY" "$ERR_4"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5: add KEY with no body source exits 94 ==="
echo ""

RC_5=0
comment_no_stdin add ENG-1 --no-editor 2>/tmp/comment-err5.tmp || RC_5=$?
ERR_5=$(cat /tmp/comment-err5.tmp)
assert_eq "add no body exits 94" "94" "$RC_5"
assert_contains "E_COMMENT_NO_BODY on stderr" "E_COMMENT_NO_BODY" "$ERR_5"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 6: add KEY --body captures body as ADF; response body rendered ==="
echo ""

BODIES_6=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
URLS_6=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-add-201.json" "$BODIES_6" "$URLS_6"
OUT_6=$(comment add ENG-1 --body "Test comment body" 2>/dev/null)
stop_mock

CAPTURED_6=$(jq -r '.[0]' "$BODIES_6")
assert_eq "add: body is ADF doc in POST"   "doc" "$(jq -r '.body.type' <<< "$CAPTURED_6")"
assert_eq "add: response body rendered to string" "string" "$(jq -r '.body | type' <<< "$OUT_6")"
assert_eq "add: response id present"       "100" "$(jq -r '.id' <<< "$OUT_6")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 7: add KEY --body-file uses file content ==="
echo ""

BODY_FILE_7=$(mktemp "$TMPDIR_BASE/bodyfile-XXXXXX")
printf 'from file content\n' > "$BODY_FILE_7"
BODIES_7=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/comment-add-201.json" "$BODIES_7"
comment add ENG-1 --body-file "$BODY_FILE_7" >/dev/null 2>/dev/null
stop_mock

CAPTURED_7=$(jq -r '.[0]' "$BODIES_7")
BODY_7_STR=$(jq -c '.body' <<< "$CAPTURED_7")
assert_contains "add: body from file is ADF" "from file content" "$BODY_7_STR"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 8: add KEY --no-render-adf preserves ADF in response ==="
echo ""

start_mock "$SCENARIOS/comment-add-201.json"
OUT_8=$(comment add ENG-1 --body "x" --no-render-adf 2>/dev/null)
stop_mock

assert_eq "add --no-render-adf: body type is object" "object" "$(jq -r '.body | type' <<< "$OUT_8")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 9: add KEY --no-notify adds notifyUsers=false to URL ==="
echo ""

URLS_9=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-add-201.json" "" "$URLS_9"
comment add ENG-1 --body "x" --no-notify 2>/dev/null
stop_mock

CAPTURED_URL_9=$(jq -r '.[0]' "$URLS_9")
assert_contains "add --no-notify: URL has notifyUsers=false" "notifyUsers=false" "$CAPTURED_URL_9"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 10: add KEY --visibility role:Administrators included in body ==="
echo ""

BODIES_10=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/comment-add-201.json" "$BODIES_10"
comment add ENG-1 --body "x" --visibility "role:Administrators" 2>/dev/null
stop_mock

CAPTURED_10=$(jq -r '.[0]' "$BODIES_10")
assert_eq "add --visibility: type is role"            "role"           "$(jq -r '.visibility.type'  <<< "$CAPTURED_10")"
assert_eq "add --visibility: value is Administrators" "Administrators" "$(jq -r '.visibility.value' <<< "$CAPTURED_10")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 11: list with no key exits 93 ==="
echo ""

RC_11=0
comment_no_stdin list 2>/tmp/comment-err11.tmp || RC_11=$?
ERR_11=$(cat /tmp/comment-err11.tmp)
assert_eq "list no key exits 93" "93" "$RC_11"
assert_contains "E_COMMENT_NO_KEY on stderr" "E_COMMENT_NO_KEY" "$ERR_11"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 12: list KEY captures URL with startAt=0&maxResults=50; bodies rendered ==="
echo ""

URLS_12=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-list-200.json" "" "$URLS_12"
OUT_12=$(comment list ENG-1 2>/dev/null)
stop_mock

CAPTURED_URL_12=$(jq -r '.[0]' "$URLS_12")
assert_contains "list: URL has startAt=0"     "startAt=0"     "$CAPTURED_URL_12"
assert_contains "list: URL has maxResults=50" "maxResults=50" "$CAPTURED_URL_12"
assert_eq "list: first body rendered to string" "string" "$(jq -r '.comments[0].body | type' <<< "$OUT_12")"
assert_eq "list: comment count"                 "2"      "$(jq '.comments | length' <<< "$OUT_12")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 13: list KEY --no-render-adf preserves ADF in comments ==="
echo ""

start_mock "$SCENARIOS/comment-list-200.json"
OUT_13=$(comment list ENG-1 --no-render-adf 2>/dev/null)
stop_mock

assert_eq "list --no-render-adf: body type is object" "object" "$(jq -r '.comments[0].body | type' <<< "$OUT_13")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 14: list KEY --page-size 3 → URL has maxResults=3 ==="
echo ""

URLS_14=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-list-200.json" "" "$URLS_14"
comment list ENG-1 --page-size 3 2>/dev/null
stop_mock

CAPTURED_URL_14=$(jq -r '.[0]' "$URLS_14")
assert_contains "list --page-size 3: URL has maxResults=3" "maxResults=3" "$CAPTURED_URL_14"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 15: list KEY paginated — 3 pages accumulated into one response ==="
echo ""

URLS_15=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-list-paginated.json" "" "$URLS_15"
OUT_15=$(comment list ENG-1 --no-render-adf 2>/dev/null)
stop_mock

URL_COUNT_15=$(jq 'length' "$URLS_15")
assert_eq "paginated: 3 requests made"      "3" "$URL_COUNT_15"
assert_eq "paginated: 5 comments total"     "5" "$(jq '.comments | length' <<< "$OUT_15")"
assert_eq "paginated: total field"          "5" "$(jq '.total' <<< "$OUT_15")"
assert_eq "paginated: truncated false"      "false" "$(jq '.truncated' <<< "$OUT_15")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 16: list KEY --page-size 2 paginates and URL has maxResults=2 ==="
echo ""

URLS_16=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-list-paginated.json" "" "$URLS_16"
OUT_16=$(comment list ENG-1 --page-size 2 --no-render-adf 2>/dev/null)
stop_mock

FIRST_URL_16=$(jq -r '.[0]' "$URLS_16")
assert_contains "list --page-size 2: URL has maxResults=2" "maxResults=2" "$FIRST_URL_16"
assert_eq "list --page-size 2: 5 comments accumulated"    "5" "$(jq '.comments | length' <<< "$OUT_16")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 17: list KEY --first-page-only returns one page only ==="
echo ""

URLS_17=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-list-paginated.json" "" "$URLS_17"
OUT_17=$(comment list ENG-1 --first-page-only --no-render-adf 2>/dev/null)
stop_mock

URL_COUNT_17=$(jq 'length' "$URLS_17")
assert_eq "first-page-only: exactly 1 request" "1"  "$URL_COUNT_17"
assert_eq "first-page-only: 2 comments (page 1 only)" "2" "$(jq '.comments | length' <<< "$OUT_17")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 18: edit with no key exits 93 ==="
echo ""

RC_18=0
comment_no_stdin edit 2>/tmp/comment-err18.tmp || RC_18=$?
ERR_18=$(cat /tmp/comment-err18.tmp)
assert_eq "edit no key exits 93" "93" "$RC_18"
assert_contains "E_COMMENT_NO_KEY on stderr" "E_COMMENT_NO_KEY" "$ERR_18"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 19: edit KEY with no comment id exits 95 ==="
echo ""

RC_19=0
comment_no_stdin edit ENG-1 2>/tmp/comment-err19.tmp || RC_19=$?
ERR_19=$(cat /tmp/comment-err19.tmp)
assert_eq "edit no id exits 95" "95" "$RC_19"
assert_contains "E_COMMENT_NO_ID on stderr" "E_COMMENT_NO_ID" "$ERR_19"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 20: edit KEY COMMENT_ID --body — PUT captured; response rendered ==="
echo ""

BODIES_20=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/comment-edit-200.json" "$BODIES_20"
OUT_20=$(comment edit ENG-1 100 --body "fix" 2>/dev/null)
stop_mock

CAPTURED_20=$(jq -r '.[0]' "$BODIES_20")
assert_eq "edit: body is ADF doc in PUT"     "doc"    "$(jq -r '.body.type' <<< "$CAPTURED_20")"
assert_eq "edit: response body rendered"     "string" "$(jq -r '.body | type' <<< "$OUT_20")"
assert_eq "edit: response id present"        "100"    "$(jq -r '.id' <<< "$OUT_20")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 21: edit KEY COMMENT_ID --no-notify --body → URL has notifyUsers=false ==="
echo ""

URLS_21=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-edit-200.json" "" "$URLS_21"
comment edit ENG-1 100 --no-notify --body "fix" 2>/dev/null
stop_mock

CAPTURED_URL_21=$(jq -r '.[0]' "$URLS_21")
assert_contains "edit --no-notify: URL has notifyUsers=false" "notifyUsers=false" "$CAPTURED_URL_21"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 22: delete with no key exits 93 ==="
echo ""

RC_22=0
comment_no_stdin delete 2>/tmp/comment-err22.tmp || RC_22=$?
ERR_22=$(cat /tmp/comment-err22.tmp)
assert_eq "delete no key exits 93" "93" "$RC_22"
assert_contains "E_COMMENT_NO_KEY on stderr" "E_COMMENT_NO_KEY" "$ERR_22"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 23: delete KEY with no comment id exits 95 ==="
echo ""

RC_23=0
comment_no_stdin delete ENG-1 2>/tmp/comment-err23.tmp || RC_23=$?
ERR_23=$(cat /tmp/comment-err23.tmp)
assert_eq "delete no id exits 95" "95" "$RC_23"
assert_contains "E_COMMENT_NO_ID on stderr" "E_COMMENT_NO_ID" "$ERR_23"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 24: delete KEY COMMENT_ID → DELETE captured; exits 0; empty stdout ==="
echo ""

URLS_24=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-delete-204.json" "" "$URLS_24"
RC_24=0
OUT_24=$(comment delete ENG-1 100 2>/dev/null) || RC_24=$?
stop_mock

assert_eq "delete: exits 0"          "0"  "$RC_24"
assert_empty "delete: stdout empty"  "$OUT_24"
CAPTURED_URL_24=$(jq -r '.[0]' "$URLS_24")
assert_contains "delete: correct URL" "/rest/api/3/issue/ENG-1/comment/100" "$CAPTURED_URL_24"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 25: delete KEY COMMENT_ID --no-notify → URL has notifyUsers=false ==="
echo ""

URLS_25=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-delete-204.json" "" "$URLS_25"
comment delete ENG-1 100 --no-notify 2>/dev/null
stop_mock

CAPTURED_URL_25=$(jq -r '.[0]' "$URLS_25")
assert_contains "delete --no-notify: URL has notifyUsers=false" "notifyUsers=false" "$CAPTURED_URL_25"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 25a: delete KEY COMMENT_ID --describe → no API call; prints preview ==="
echo ""

URLS_25A=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-delete-describe-guard.json" "" "$URLS_25A"
RC_25A=0
OUT_25A=$(comment delete ENG-1 100 --describe 2>/dev/null) || RC_25A=$?
stop_mock

assert_eq "describe: exits 0"           "0"       "$RC_25A"
assert_eq "describe: no API calls"      "[]"      "$(jq -c '.' "$URLS_25A")"
assert_eq "describe: method is DELETE"  "DELETE"  "$(jq -r '.method'          <<< "$OUT_25A")"
assert_eq "describe: correct path"      "/rest/api/3/issue/ENG-1/comment/100" \
  "$(jq -r '.path' <<< "$OUT_25A")"
assert_eq "describe: queryParams empty" "{}"      "$(jq -c '.queryParams'     <<< "$OUT_25A")"
assert_eq "describe: body is null"      "null"    "$(jq -r '.body'            <<< "$OUT_25A")"
assert_eq "describe: irreversible true" "true"    "$(jq -r '.irreversible'    <<< "$OUT_25A")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 25b: delete KEY COMMENT_ID --describe --no-notify → notifyUsers in queryParams ==="
echo ""

URLS_25B=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-delete-describe-guard.json" "" "$URLS_25B"
OUT_25B=$(comment delete ENG-1 100 --describe --no-notify 2>/dev/null)
stop_mock

assert_eq "describe --no-notify: no API calls" "[]" "$(jq -c '.' "$URLS_25B")"
assert_eq "describe --no-notify: queryParams.notifyUsers" "false" \
  "$(jq -r '.queryParams.notifyUsers' <<< "$OUT_25B")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 25c: delete --describe (no key) exits 93 ==="
echo ""

RC_25C=0
comment_no_stdin delete --describe 2>/tmp/comment-err25c.tmp || RC_25C=$?
ERR_25C=$(cat /tmp/comment-err25c.tmp)
assert_eq "describe no key exits 93" "93" "$RC_25C"
assert_contains "E_COMMENT_NO_KEY on stderr" "E_COMMENT_NO_KEY" "$ERR_25C"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 25d: delete KEY --describe (no comment id) exits 95 ==="
echo ""

RC_25D=0
comment_no_stdin delete ENG-1 --describe 2>/tmp/comment-err25d.tmp || RC_25D=$?
ERR_25D=$(cat /tmp/comment-err25d.tmp)
assert_eq "describe no id exits 95" "95" "$RC_25D"
assert_contains "E_COMMENT_NO_ID on stderr" "E_COMMENT_NO_ID" "$ERR_25D"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 26: list against nonexistent issue exits 13 with hint ==="
echo ""

start_mock "$SCENARIOS/comment-list-404.json"
RC_26=0
comment list ENG-999 2>/tmp/comment-err26.tmp || RC_26=$?
stop_mock
ERR_26=$(cat /tmp/comment-err26.tmp)
assert_eq "404 exits 13"             "13"     "$RC_26"
assert_contains "hint for 404"       "Hint:"  "$ERR_26"
assert_contains "not found in hint"  "not found" "$ERR_26"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 27: add KEY --body x --print-payload → no API call; prints payload ==="
echo ""

URLS_27=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-add-print-payload-guard.json" "" "$URLS_27"
RC_27=0
OUT_27=$(comment_no_stdin add ENG-1 --body "x" --print-payload 2>/dev/null) || RC_27=$?
stop_mock

assert_eq "add --print-payload: exits 0"         "0"                                  "$RC_27"
assert_eq "add --print-payload: no API calls"    "[]"                                 "$(jq -c '.' "$URLS_27")"
assert_eq "add --print-payload: method is POST"  "POST"                               "$(jq -r '.method' <<< "$OUT_27")"
assert_eq "add --print-payload: correct path"    "/rest/api/3/issue/ENG-1/comment"   "$(jq -r '.path'   <<< "$OUT_27")"
assert_eq "add --print-payload: body is object"  "object"                             "$(jq -r '.body | type' <<< "$OUT_27")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 28: edit KEY COMMENT_ID --body x --print-payload → no API call ==="
echo ""

URLS_28=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-edit-print-payload-guard.json" "" "$URLS_28"
RC_28=0
OUT_28=$(comment_no_stdin edit ENG-1 100 --body "x" --print-payload 2>/dev/null) || RC_28=$?
stop_mock

assert_eq "edit --print-payload: exits 0"         "0"                                         "$RC_28"
assert_eq "edit --print-payload: no API calls"    "[]"                                        "$(jq -c '.' "$URLS_28")"
assert_eq "edit --print-payload: method is PUT"   "PUT"                                       "$(jq -r '.method' <<< "$OUT_28")"
assert_eq "edit --print-payload: correct path"    "/rest/api/3/issue/ENG-1/comment/100"      "$(jq -r '.path'   <<< "$OUT_28")"
assert_eq "edit --print-payload: body is object"  "object"                                    "$(jq -r '.body | type' <<< "$OUT_28")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 29: list KEY against empty comments — one request; output has comments:[] ==="
echo ""

URLS_29=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-list-empty-200.json" "" "$URLS_29"
OUT_29=$(comment list ENG-1 --no-render-adf 2>/dev/null)
stop_mock

assert_eq "empty: one request"       "1"     "$(jq 'length' "$URLS_29")"
assert_eq "empty: 0 comments"        "0"     "$(jq '.comments | length' <<< "$OUT_29")"
assert_eq "empty: total 0"           "0"     "$(jq '.total' <<< "$OUT_29")"
assert_eq "empty: truncated false"   "false" "$(jq '.truncated' <<< "$OUT_29")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 30: list KEY --page-size 2 with exact-page fixture → 1 request ==="
echo ""

URLS_30=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-list-exact-page-200.json" "" "$URLS_30"
OUT_30=$(comment list ENG-1 --page-size 2 --no-render-adf 2>/dev/null)
stop_mock

assert_eq "exact-page: 1 request"  "1"     "$(jq 'length' "$URLS_30")"
assert_eq "exact-page: 2 comments" "2"     "$(jq '.comments | length' <<< "$OUT_30")"
assert_eq "exact-page: truncated false" "false" "$(jq '.truncated' <<< "$OUT_30")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 31: shrinking total terminates correctly ==="
echo ""

start_mock "$SCENARIOS/comment-list-shrinking-total.json"
OUT_31=$(comment list ENG-1 --page-size 2 --no-render-adf 2>/dev/null)
stop_mock

assert_eq "shrinking: 3 comments accumulated" "3" "$(jq '.comments | length' <<< "$OUT_31")"
assert_eq "shrinking: truncated false"         "false" "$(jq '.truncated' <<< "$OUT_31")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 32: runaway pagination truncates at MAX_PAGES=20 ==="
echo ""

URLS_32=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-list-runaway.json" "" "$URLS_32"
RC_32=0
OUT_32=$(comment list ENG-1 --no-render-adf 2>/tmp/comment-err32.tmp) || RC_32=$?
stop_mock
ERR_32=$(cat /tmp/comment-err32.tmp)

assert_eq "runaway: exits 0"            "0"    "$RC_32"
assert_eq "runaway: exactly 20 GETs"    "20"   "$(jq 'length' "$URLS_32")"
assert_eq "runaway: truncated true"     "true" "$(jq '.truncated' <<< "$OUT_32")"
assert_contains "runaway: Warning on stderr" "Warning:" "$ERR_32"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 32a: natural end at exactly MAX_PAGES — no truncation ==="
echo ""

URLS_32A=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/comment-list-natural-end-at-cap.json" "" "$URLS_32A"
RC_32A=0
OUT_32A=$(comment list ENG-1 --page-size 2 --no-render-adf 2>/tmp/comment-err32a.tmp) || RC_32A=$?
stop_mock
ERR_32A=$(cat /tmp/comment-err32a.tmp)

assert_eq "natural-end: exits 0"            "0"    "$RC_32A"
assert_eq "natural-end: exactly 20 GETs"    "20"   "$(jq 'length' "$URLS_32A")"
assert_eq "natural-end: truncated false"    "false" "$(jq '.truncated' <<< "$OUT_32A")"
WARN_32A=$(printf '%s\n' "$ERR_32A" | grep "^Warning:" || true)
assert_empty "natural-end: no Warning on stderr" "$WARN_32A"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 33: bad total → E_COMMENT_BAD_RESPONSE exits 99 ==="
echo ""

start_mock "$SCENARIOS/comment-list-bad-total.json"
RC_33=0
comment list ENG-1 2>/tmp/comment-err33.tmp || RC_33=$?
stop_mock
ERR_33=$(cat /tmp/comment-err33.tmp)
assert_eq "bad total exits 99"              "99"                      "$RC_33"
assert_contains "E_COMMENT_BAD_RESPONSE on stderr" "E_COMMENT_BAD_RESPONSE" "$ERR_33"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 33a: empty page mid-pagination breaks via page_returned==0 guard ==="
echo ""

start_mock "$SCENARIOS/comment-list-empty-mid-page.json"
RC_33A=0
OUT_33A=$(comment list ENG-1 --no-render-adf 2>/dev/null) || RC_33A=$?
stop_mock

assert_eq "empty-mid-page: exits 0"                  "0"     "$RC_33A"
assert_eq "empty-mid-page: 2 comments from page 1"   "2"     "$(jq '.comments | length' <<< "$OUT_33A")"
assert_eq "empty-mid-page: truncated false"           "false" "$(jq '.truncated' <<< "$OUT_33A")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 34: list 5xx → exits 20; Hint on stderr ==="
echo ""

start_mock "$SCENARIOS/comment-list-500.json"
RC_34=0
(cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 \
  JIRA_RETRY_SLEEP_FN=_test_comment_sleep_noop \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
  bash "$SCRIPT" list ENG-1 \
  2>/tmp/comment-err34.tmp) || RC_34=$?
stop_mock
ERR_34=$(cat /tmp/comment-err34.tmp)
assert_eq "list 5xx exits 20"        "20"    "$RC_34"
assert_contains "5xx hint emitted"   "Hint:" "$ERR_34"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 35: ADF round-trip wiring for add ==="
echo ""

BODY_MD_35="Hello **comment** from _test_"
BODY_FILE_35=$(mktemp "$TMPDIR_BASE/body-XXXXXX")
printf '%s\n' "$BODY_MD_35" > "$BODY_FILE_35"
EXPECTED_ADF_35=$(printf '%s\n' "$BODY_MD_35" | bash "$SCRIPT_DIR/jira-md-to-adf.sh")

BODIES_35=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/comment-add-201.json" "$BODIES_35"
comment add ENG-1 --body-file "$BODY_FILE_35" >/dev/null 2>/dev/null
stop_mock

CAPTURED_35=$(jq -r '.[0]' "$BODIES_35")
ADF_RC_35=0
jq -e --argjson exp "$EXPECTED_ADF_35" '.body == $exp' <<< "$CAPTURED_35" >/dev/null 2>&1 \
  || ADF_RC_35=$?
assert_eq "ADF round-trip matches expected output" "0" "$ADF_RC_35"
echo ""

# ---------------------------------------------------------------------------

test_summary
