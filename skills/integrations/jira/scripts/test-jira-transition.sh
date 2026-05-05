#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-transition-flow.sh
# Run: bash skills/integrations/jira/scripts/test-jira-transition.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/jira-transition-flow.sh"
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

transition() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

transition_no_creds() {
  cd "$REPO" && ACCELERATOR_TEST_MODE=1 \
    bash "$SCRIPT" "$@"
}

# ---------------------------------------------------------------------------

echo "=== Case 1: happy path — state name match → POST 204 → exit 0 ==="
echo ""

start_mock "$SCENARIOS/transition-post-204.json"
RC_1=0
transition ENG-1 "In Progress" 2>/dev/null || RC_1=$?
stop_mock

assert_eq "happy path exits 0" "0" "$RC_1"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 2: case-insensitive match → POST body has id 21 ==="
echo ""

BODIES_2=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/transition-post-204-capture.json" "$BODIES_2"
RC_2=0
transition ENG-1 "in progress" 2>/dev/null || RC_2=$?
stop_mock

CAPTURED_2=$(jq -r '.[0]' "$BODIES_2")
assert_eq "case-insensitive: exits 0"         "0"    "$RC_2"
assert_eq "case-insensitive: transition id"   "21"   "$(jq -r '.transition.id' <<< "$CAPTURED_2")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 3: --describe with STATE_NAME — GET resolves transition, no POST ==="
echo ""

start_mock "$SCENARIOS/transition-list-200.json"
RC_3=0
OUT_3=$(transition --describe ENG-1 "In Progress" 2>/dev/null) || RC_3=$?
stop_mock

assert_eq "describe: exits 0"               "0"    "$RC_3"
assert_eq "describe: key is ENG-1"          "ENG-1" "$(jq -r '.key' <<< "$OUT_3")"
assert_eq "describe: state is non-null"     "In Progress" "$(jq -r '.state' <<< "$OUT_3")"
assert_eq "describe: transition_id is 21"   "21"   "$(jq -r '.transition_id' <<< "$OUT_3")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 3b: --describe ENG-1 Nonexistent — no match → exit 122 ==="
echo ""

start_mock "$SCENARIOS/transition-list-200.json"
RC_3B=0
transition --describe ENG-1 "Nonexistent" 2>/dev/null || RC_3B=$?
stop_mock

assert_eq "describe nonexistent: exits 122" "122" "$RC_3B"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 4: --describe guard — no POST made ==="
echo ""

URLS_4=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/transition-describe-guard.json" "" "$URLS_4"
RC_4=0
transition --describe ENG-1 "In Progress" 2>/dev/null || RC_4=$?
stop_mock

assert_eq "describe guard: exits 0"         "0"   "$RC_4"
assert_eq "describe guard: no API POSTs"    "[]"  "$(jq -c '.' "$URLS_4")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5: ambiguous state name → exit 123; stdout is JSON array with both IDs ==="
echo ""

start_mock "$SCENARIOS/transition-list-ambiguous-200.json"
RC_5=0
OUT_5=$(transition ENG-1 "In Review" 2>/dev/null) || RC_5=$?
stop_mock

assert_eq "ambiguous: exits 123"            "123"   "$RC_5"
assert_eq "ambiguous: stdout is array"      "array" "$(jq -r 'type' <<< "$OUT_5")"
assert_contains "ambiguous: id 41 present"  "$OUT_5" "41"
assert_contains "ambiguous: id 42 present"  "$OUT_5" "42"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 6: --transition-id bypasses GET; POST body has correct ID ==="
echo ""

BODIES_6=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/transition-post-204-direct.json" "$BODIES_6"
RC_6=0
transition ENG-1 --transition-id 21 2>/dev/null || RC_6=$?
stop_mock

CAPTURED_6=$(jq -r '.[0]' "$BODIES_6")
assert_eq "transition-id: exits 0"          "0"  "$RC_6"
assert_eq "transition-id: POST body id"     "21" "$(jq -r '.transition.id' <<< "$CAPTURED_6")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 7: state name not found → exit 122 ==="
echo ""

start_mock "$SCENARIOS/transition-list-200.json"
RC_7=0
transition ENG-1 "Nonexistent State" 2>/dev/null || RC_7=$?
stop_mock

assert_eq "not found: exits 122" "122" "$RC_7"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 8: --resolution NAME included in POST body ==="
echo ""

BODIES_8=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/transition-post-204-capture.json" "$BODIES_8"
RC_8=0
transition ENG-1 "In Progress" --resolution "Fixed" 2>/dev/null || RC_8=$?
stop_mock

CAPTURED_8=$(jq -r '.[0]' "$BODIES_8")
assert_eq "resolution: exits 0"                     "0"       "$RC_8"
assert_eq "resolution: fields.resolution.name"      "Fixed"   "$(jq -r '.fields.resolution.name' <<< "$CAPTURED_8")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 9: --comment TEXT assembled as ADF in POST body ==="
echo ""

BODIES_9=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/transition-post-204-capture.json" "$BODIES_9"
RC_9=0
transition ENG-1 "In Progress" --comment "test comment" 2>/dev/null || RC_9=$?
stop_mock

CAPTURED_9=$(jq -r '.[0]' "$BODIES_9")
assert_eq "comment: exits 0"                        "0"    "$RC_9"
assert_eq "comment: ADF doc type"                   "doc"  "$(jq -r '.update.comment[0].add.body.type' <<< "$CAPTURED_9")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 10: missing issue key → exit 120, no network call ==="
echo ""

RC_10=0
transition 2>/tmp/transition-err10.tmp || RC_10=$?
ERR_10=$(cat /tmp/transition-err10.tmp)
assert_eq "no key: exits 120"               "120"               "$RC_10"
assert_contains "no key: E_TRANSITION_NO_KEY on stderr" "$ERR_10" "E_TRANSITION_NO_KEY"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 11: missing state name and --transition-id → exit 121 ==="
echo ""

RC_11=0
transition ENG-1 2>/tmp/transition-err11.tmp || RC_11=$?
ERR_11=$(cat /tmp/transition-err11.tmp)
assert_eq "no state: exits 121"             "121"               "$RC_11"
assert_contains "no state: E_TRANSITION_NO_STATE on stderr" "$ERR_11" "E_TRANSITION_NO_STATE"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 12: GET transitions 401 → exit 11 ==="
echo ""

start_mock "$SCENARIOS/transition-list-401.json"
RC_12=0
transition ENG-1 "In Progress" 2>/tmp/transition-err12.tmp || RC_12=$?
stop_mock
ERR_12=$(cat /tmp/transition-err12.tmp)
assert_eq "401: exits 11"                   "11"     "$RC_12"
assert_contains "401: hint on stderr"       "$ERR_12" "Hint:"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 13: GET transitions 404 → exit 13 ==="
echo ""

start_mock "$SCENARIOS/transition-list-404.json"
RC_13=0
transition ENG-1 "In Progress" 2>/tmp/transition-err13.tmp || RC_13=$?
stop_mock
ERR_13=$(cat /tmp/transition-err13.tmp)
assert_eq "404: exits 13"                   "13"           "$RC_13"
assert_contains "404: hint on stderr"       "$ERR_13" "not found"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 14: POST transition 400 → exit 34 ==="
echo ""

start_mock "$SCENARIOS/transition-post-400.json"
RC_14=0
transition ENG-1 "Done" 2>/tmp/transition-err14.tmp || RC_14=$?
stop_mock
ERR_14=$(cat /tmp/transition-err14.tmp)
assert_eq "400: exits 34"                   "34"           "$RC_14"
assert_contains "400: error body on stderr" "$ERR_14" "resolution"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 15: unrecognised flag → exit 124 ==="
echo ""

RC_15=0
transition ENG-1 "In Progress" --unknown-flag 2>/tmp/transition-err15.tmp || RC_15=$?
ERR_15=$(cat /tmp/transition-err15.tmp)
assert_eq "bad flag: exits 124"             "124"  "$RC_15"
assert_contains "bad flag: E_TRANSITION_BAD_FLAG on stderr" "$ERR_15" "E_TRANSITION_BAD_FLAG"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 16: --comment-file PATH where PATH does not exist → exit 125 ==="
echo ""

RC_16=0
transition ENG-1 "In Progress" --comment-file /nonexistent/path/file.txt \
  2>/tmp/transition-err16.tmp || RC_16=$?
ERR_16=$(cat /tmp/transition-err16.tmp)
assert_eq "missing comment-file: exits 125" "125" "$RC_16"
assert_contains "missing comment-file: E_TRANSITION_NO_BODY on stderr" "$ERR_16" "E_TRANSITION_NO_BODY"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 17: --resolution '' (empty) → exit 126 ==="
echo ""

RC_17=0
transition ENG-1 "In Progress" --resolution '' \
  2>/tmp/transition-err17.tmp || RC_17=$?
ERR_17=$(cat /tmp/transition-err17.tmp)
assert_eq "empty resolution: exits 126"     "126" "$RC_17"
assert_contains "empty resolution: E_TRANSITION_BAD_RESOLUTION on stderr" "$ERR_17" "E_TRANSITION_BAD_RESOLUTION"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 18: --describe --transition-id offline path — no network call ==="
echo ""

RC_18=0
OUT_18=$(transition_no_creds --describe ENG-1 --transition-id 21 2>/dev/null) || RC_18=$?
assert_eq "offline describe: exits 0"           "0"    "$RC_18"
assert_eq "offline describe: state is null"     "null" "$(jq -r '.state' <<< "$OUT_18")"
assert_eq "offline describe: transition_id 21"  "21"   "$(jq -r '.transition_id' <<< "$OUT_18")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 19: STATE_NAME and --transition-id both supplied → exit 124 (both orderings) ==="
echo ""

RC_19A=0
transition ENG-1 "Done" --transition-id 21 2>/dev/null || RC_19A=$?
assert_eq "both: ordering A exits 124"      "124" "$RC_19A"

RC_19B=0
transition ENG-1 --transition-id 21 "Done" 2>/dev/null || RC_19B=$?
assert_eq "both: ordering B exits 124"      "124" "$RC_19B"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 20: --comment-file - (dash prefix) → exit 125 ==="
echo ""

RC_20=0
transition ENG-1 "In Progress" --comment-file - \
  2>/tmp/transition-err20.tmp || RC_20=$?
ERR_20=$(cat /tmp/transition-err20.tmp)
assert_eq "dash comment-file: exits 125"    "125" "$RC_20"
assert_contains "dash comment-file: E_TRANSITION_NO_BODY on stderr" "$ERR_20" "E_TRANSITION_NO_BODY"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 21: --transition-id abc (non-numeric) → exit 124 ==="
echo ""

RC_21=0
transition ENG-1 --transition-id abc \
  2>/tmp/transition-err21.tmp || RC_21=$?
ERR_21=$(cat /tmp/transition-err21.tmp)
assert_eq "non-numeric id: exits 124"       "124" "$RC_21"
assert_contains "non-numeric id: E_TRANSITION_BAD_FLAG on stderr" "$ERR_21" "E_TRANSITION_BAD_FLAG"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 22: --no-notify appends notifyUsers=false to POST URL ==="
echo ""

URLS_22=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/transition-post-204-no-notify.json" "" "$URLS_22"
RC_22=0
transition ENG-1 "In Progress" --no-notify 2>/dev/null || RC_22=$?
stop_mock

CAPTURED_URL_22=$(jq -r '.[0]' "$URLS_22")
assert_eq "no-notify: exits 0"              "0" "$RC_22"
assert_contains "no-notify: URL has notifyUsers=false" "$CAPTURED_URL_22" "notifyUsers=false"
echo ""

# ---------------------------------------------------------------------------

test_summary
