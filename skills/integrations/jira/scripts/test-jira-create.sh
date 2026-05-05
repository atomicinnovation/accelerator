#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-create-flow.sh
# Run: bash skills/integrations/jira/scripts/test-jira-create.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/jira-create-flow.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-jira-server.py"

TEST_TOKEN="tok-SENTINEL-xyz123"
TEST_SITE="example"
TEST_EMAIL="test@example.com"
TEST_ACCOUNT_ID="redacted-id-789"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

# ---------------------------------------------------------------------------
# Repo / mock setup helpers

# setup_repo — jira + work config with default project code
setup_repo() {
  local d; d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator"
  cat > "$d/.accelerator/config.md" <<ENDCONFIG
---
jira:
  site: $TEST_SITE
  email: $TEST_EMAIL
work:
  default_project_code: ENG
---
ENDCONFIG
  echo "$d"
}

# setup_repo_minimal — jira credentials only, no default project code
setup_repo_minimal() {
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

write_site_json() {
  local repo="$1"
  mkdir -p "$repo/.accelerator/state/integrations/jira"
  printf '{"site":"%s","accountId":"%s"}\n' "$TEST_SITE" "$TEST_ACCOUNT_ID" \
    > "$repo/.accelerator/state/integrations/jira/site.json"
}

write_fields_json() {
  local repo="$1"
  mkdir -p "$repo/.accelerator/state/integrations/jira"
  jq -cn '{
    "site": "example",
    "fields": [
      {
        "id": "summary",
        "key": "summary",
        "name": "Summary",
        "slug": "summary"
      },
      {
        "id": "customfield_10016",
        "key": "customfield_10016",
        "name": "Story Points",
        "slug": "story-points",
        "schema": {"type": "number", "custom": "com.atlassian.jira.plugin.system.customfieldtypes:float"}
      },
      {
        "id": "customfield_10020",
        "key": "customfield_10020",
        "name": "Sprint",
        "slug": "sprint",
        "schema": {"type": "array", "custom": "com.pyxis.greenhopper.jira:gh-sprint"}
      }
    ]
  }' > "$repo/.accelerator/state/integrations/jira/fields.json"
}

REPO=$(setup_repo)
write_site_json "$REPO"
write_fields_json "$REPO"

# ---------------------------------------------------------------------------
# Sleep stub for retry tests (no-op; file-based so it works across subprocesses)

_test_create_sleep_noop() { :; }
export -f _test_create_sleep_noop

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
  [[ -n "$captured_urls_file" ]] && mock_args+=("--captured-urls-file" "$captured_urls_file")
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

# Run the create flow from REPO with test credentials + mock URL
create() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

# create_no_stdin — same but with stdin forced to TTY (CI-safe; no editor invocation)
create_no_stdin() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    JIRA_BODY_STDIN_IS_TTY_TEST=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

# ---------------------------------------------------------------------------

echo "=== Case 1: --help exits 0 with usage banner ==="
echo ""

OUT_1=$(create --help 2>/dev/null)
assert_contains "usage includes --project" "$OUT_1" "--project"
assert_contains "usage includes --summary" "$OUT_1" "--summary"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 2: no --project, no work.default_project_code exits 100 ==="
echo ""

REPO_2=$(setup_repo_minimal)
write_site_json "$REPO_2"
write_fields_json "$REPO_2"
RC_2=0
(cd "$REPO_2" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 JIRA_BODY_STDIN_IS_TTY_TEST=1 \
  bash "$SCRIPT" --type Task --summary "foo" --body "x" --no-editor 2>/tmp/create-err2.tmp) || RC_2=$?
ERR_2=$(cat /tmp/create-err2.tmp)
assert_eq "no project exits 100" "100" "$RC_2"
assert_contains "E_CREATE_NO_PROJECT on stderr" "$ERR_2" "E_CREATE_NO_PROJECT"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 3: missing --type and --issuetype-id exits 101 ==="
echo ""

RC_3=0
create_no_stdin --project ENG --summary "foo" --body "x" --no-editor 2>/tmp/create-err3.tmp || RC_3=$?
ERR_3=$(cat /tmp/create-err3.tmp)
assert_eq "no type exits 101" "101" "$RC_3"
assert_contains "E_CREATE_NO_TYPE on stderr" "$ERR_3" "E_CREATE_NO_TYPE"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 4: missing --summary exits 102 ==="
echo ""

RC_4=0
create_no_stdin --project ENG --type Task --no-editor 2>/tmp/create-err4.tmp || RC_4=$?
ERR_4=$(cat /tmp/create-err4.tmp)
assert_eq "no summary exits 102" "102" "$RC_4"
assert_contains "E_CREATE_NO_SUMMARY on stderr" "$ERR_4" "E_CREATE_NO_SUMMARY"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5: unrecognised flag exits 104 ==="
echo ""

RC_5=0
create_no_stdin --project ENG --type Task --summary "foo" --body "x" --nope 2>/tmp/create-err5.tmp || RC_5=$?
ERR_5=$(cat /tmp/create-err5.tmp)
assert_eq "bad flag exits 104" "104" "$RC_5"
assert_contains "E_CREATE_BAD_FLAG on stderr" "$ERR_5" "E_CREATE_BAD_FLAG"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 6: happy path with inline --body ==="
echo ""

BODIES_6=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/create-201-capture.json" "$BODIES_6"
OUT_6=$(create --project ENG --type Task --summary "create test" --body "Hello world" 2>/dev/null)
stop_mock

CAPTURED_6=$(jq -r '.[0]' "$BODIES_6")
assert_eq "response key" "ENG-123" "$(jq -r '.key' <<< "$OUT_6")"
assert_eq "project key in body" "ENG" "$(jq -r '.fields.project.key' <<< "$CAPTURED_6")"
assert_eq "summary in body" "create test" "$(jq -r '.fields.summary' <<< "$CAPTURED_6")"
assert_eq "issuetype name in body" "Task" "$(jq -r '.fields.issuetype.name' <<< "$CAPTURED_6")"
assert_eq "description is ADF doc" "doc" "$(jq -r '.fields.description.type' <<< "$CAPTURED_6")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 7: --body-file takes precedence over piped stdin ==="
echo ""

BODY_FILE_7=$(mktemp "$TMPDIR_BASE/bodyfile-XXXXXX")
printf 'from file content\n' > "$BODY_FILE_7"
BODIES_7=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/create-201-capture.json" "$BODIES_7"
printf 'from stdin content\n' | create --project ENG --type Task --summary "foo" --body-file "$BODY_FILE_7" > /dev/null 2>&1
stop_mock

CAPTURED_7=$(jq -r '.[0]' "$BODIES_7")
BODY_7_STR=$(jq -c '.fields.description' <<< "$CAPTURED_7")
assert_contains "body from file not stdin" "$BODY_7_STR" "from file content"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 8: piped stdin used when no --body or --body-file ==="
echo ""

BODIES_8=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/create-201-capture.json" "$BODIES_8"
printf 'from stdin input\n' | create --project ENG --type Task --summary "foo" > /dev/null 2>&1
stop_mock

CAPTURED_8=$(jq -r '.[0]' "$BODIES_8")
BODY_8_STR=$(jq -c '.fields.description' <<< "$CAPTURED_8")
assert_contains "body from stdin" "$BODY_8_STR" "from stdin input"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 9: no body source, stdin TTY, --no-editor exits 105 ==="
echo ""

RC_9=0
create_no_stdin --project ENG --type Task --summary "foo" --no-editor 2>/tmp/create-err9.tmp || RC_9=$?
ERR_9=$(cat /tmp/create-err9.tmp)
assert_eq "no body exits 105" "105" "$RC_9"
assert_contains "E_CREATE_NO_BODY on stderr" "$ERR_9" "E_CREATE_NO_BODY"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 10: --assignee @me resolves from site.json ==="
echo ""

BODIES_10=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/create-201-capture.json" "$BODIES_10"
create --project ENG --type Task --summary "foo" --body "x" --assignee @me > /dev/null 2>&1
stop_mock

CAPTURED_10=$(jq -r '.[0]' "$BODIES_10")
assert_eq "assignee accountId in body" "$TEST_ACCOUNT_ID" "$(jq -r '.fields.assignee.accountId' <<< "$CAPTURED_10")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 11: --assignee @me with site.json missing exits 106 ==="
echo ""

REPO_11=$(setup_repo)
write_fields_json "$REPO_11"
# No write_site_json — intentionally absent
RC_11=0
(cd "$REPO_11" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 JIRA_BODY_STDIN_IS_TTY_TEST=1 \
  bash "$SCRIPT" --project ENG --type Task --summary "foo" --body "x" --assignee @me --no-editor \
  2>/tmp/create-err11.tmp) || RC_11=$?
ERR_11=$(cat /tmp/create-err11.tmp)
assert_eq "missing site cache exits 106" "106" "$RC_11"
assert_contains "init-jira hint" "$ERR_11" "init-jira"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 11a: --assignee email address exits 107 ==="
echo ""

RC_11a=0
create_no_stdin --project ENG --type Task --summary "foo" --body "x" \
  --assignee user@example.com --no-editor 2>/tmp/create-err11a.tmp || RC_11a=$?
ERR_11a=$(cat /tmp/create-err11a.tmp)
assert_eq "email assignee exits 107" "107" "$RC_11a"
assert_contains "E_CREATE_BAD_ASSIGNEE on stderr" "$ERR_11a" "E_CREATE_BAD_ASSIGNEE"
assert_contains "email not supported message" "$ERR_11a" "email"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 11b: --assignee with invalid chars exits 107 ==="
echo ""

RC_11b=0
create_no_stdin --project ENG --type Task --summary "foo" --body "x" \
  --assignee '5b10!@#$' --no-editor 2>/tmp/create-err11b.tmp || RC_11b=$?
ERR_11b=$(cat /tmp/create-err11b.tmp)
assert_eq "invalid assignee chars exits 107" "107" "$RC_11b"
assert_contains "E_CREATE_BAD_ASSIGNEE on stderr" "$ERR_11b" "E_CREATE_BAD_ASSIGNEE"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 12: --label foo --label bar ==="
echo ""

BODIES_12=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/create-201-capture.json" "$BODIES_12"
create --project ENG --type Task --summary "foo" --body "x" --label foo --label bar > /dev/null 2>&1
stop_mock

CAPTURED_12=$(jq -r '.[0]' "$BODIES_12")
LABELS_12=$(jq -c '.fields.labels' <<< "$CAPTURED_12")
assert_eq "labels array" '["foo","bar"]' "$LABELS_12"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 13: --component \"API\" ==="
echo ""

BODIES_13=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/create-201-capture.json" "$BODIES_13"
create --project ENG --type Task --summary "foo" --body "x" --component "API" > /dev/null 2>&1
stop_mock

CAPTURED_13=$(jq -r '.[0]' "$BODIES_13")
COMPS_13=$(jq -c '.fields.components' <<< "$CAPTURED_13")
assert_eq "components array" '[{"name":"API"}]' "$COMPS_13"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 14: --parent ENG-99 ==="
echo ""

BODIES_14=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/create-201-capture.json" "$BODIES_14"
create --project ENG --type Task --summary "foo" --body "x" --parent ENG-99 > /dev/null 2>&1
stop_mock

CAPTURED_14=$(jq -r '.[0]' "$BODIES_14")
assert_eq "parent key in body" "ENG-99" "$(jq -r '.fields.parent.key' <<< "$CAPTURED_14")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 15: --custom story-points=5 coerced to number ==="
echo ""

BODIES_15=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/create-with-custom-fields-capture.json" "$BODIES_15"
create --project ENG --type Task --summary "foo" --body "x" --custom story-points=5 > /dev/null 2>&1
stop_mock

CAPTURED_15=$(jq -r '.[0]' "$BODIES_15")
assert_eq "story points as number" "5" "$(jq '.fields.customfield_10016' <<< "$CAPTURED_15")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 16: --custom story-points=not-a-number exits 103 ==="
echo ""

RC_16=0
create_no_stdin --project ENG --type Task --summary "foo" --body "x" \
  --custom story-points=not-a-number --no-editor 2>/tmp/create-err16.tmp || RC_16=$?
ERR_16=$(cat /tmp/create-err16.tmp)
assert_eq "bad field value exits 103" "103" "$RC_16"
assert_contains "E_CREATE_BAD_FIELD on stderr" "$ERR_16" "E_CREATE_BAD_FIELD"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 17: --custom sprint=@json:[42] passes array literal ==="
echo ""

BODIES_17=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/create-with-custom-fields-capture.json" "$BODIES_17"
create --project ENG --type Task --summary "foo" --body "x" --custom 'sprint=@json:[42]' > /dev/null 2>&1
stop_mock

CAPTURED_17=$(jq -r '.[0]' "$BODIES_17")
assert_eq "sprint as array literal" "[42]" "$(jq -c '.fields.customfield_10020' <<< "$CAPTURED_17")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 18: --custom unknown-field exits 103 with hint ==="
echo ""

RC_18=0
create_no_stdin --project ENG --type Task --summary "foo" --body "x" \
  --custom unknown-field=value --no-editor 2>/tmp/create-err18.tmp || RC_18=$?
ERR_18=$(cat /tmp/create-err18.tmp)
assert_eq "unknown field exits 103" "103" "$RC_18"
assert_contains "refresh-fields hint" "$ERR_18" "init-jira"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 19: --issuetype-id alone satisfies type requirement ==="
echo ""

BODIES_19=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/create-201-capture.json" "$BODIES_19"
create --project ENG --issuetype-id 10001 --summary "foo" --body "x" > /dev/null 2>&1
stop_mock

CAPTURED_19=$(jq -r '.[0]' "$BODIES_19")
assert_eq "issuetype id in body" "10001" "$(jq -r '.fields.issuetype.id' <<< "$CAPTURED_19")"
assert_eq "issuetype name absent" "null" "$(jq -r '.fields.issuetype.name // "null"' <<< "$CAPTURED_19")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 20: API 400 missing-summary — exits 34, hint emitted ==="
echo ""

start_mock "$SCENARIOS/create-400-missing-summary.json"
RC_20=0
create --project ENG --type Task --summary "foo" --body "x" 2>/tmp/create-err20.tmp || RC_20=$?
stop_mock
ERR_20=$(cat /tmp/create-err20.tmp)
assert_eq "API 400 exits 34" "34" "$RC_20"
assert_contains "field error forwarded to stderr" "$ERR_20" "Summary is required"
assert_contains "hint emitted for 400" "$ERR_20" "Hint:"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 21: API 400 bad customfield — exits 34, refresh-fields hint ==="
echo ""

start_mock "$SCENARIOS/create-400-bad-customfield.json"
RC_21=0
create --project ENG --type Task --summary "foo" --body "x" 2>/tmp/create-err21.tmp || RC_21=$?
stop_mock
ERR_21=$(cat /tmp/create-err21.tmp)
assert_eq "API 400 bad customfield exits 34" "34" "$RC_21"
assert_contains "refresh-fields hint" "$ERR_21" "init-jira --refresh-fields"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 22: --print-payload does not call the API ==="
echo ""

URLS_22=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/print-payload-guard.json" "" "$URLS_22"
PAYLOAD_22=""
RC_22=0
PAYLOAD_22=$(create --project ENG --type Task --summary "foo" --body "x" --print-payload 2>/dev/null) || RC_22=$?
stop_mock

assert_eq "print-payload exits 0" "0" "$RC_22"
CAPTURED_URLS_22=$(jq -c '.' "$URLS_22")
assert_eq "no API calls made" "[]" "$CAPTURED_URLS_22"
assert_eq "method is POST" "POST" "$(jq -r '.method' <<< "$PAYLOAD_22")"
assert_eq "path is /rest/api/3/issue" "/rest/api/3/issue" "$(jq -r '.path' <<< "$PAYLOAD_22")"
assert_eq "body is JSON object" "object" "$(jq -r '.body | type' <<< "$PAYLOAD_22")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 23: --print-payload still validates args ==="
echo ""

REPO_23=$(setup_repo_minimal)
RC_23=0
(cd "$REPO_23" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 JIRA_BODY_STDIN_IS_TTY_TEST=1 \
  bash "$SCRIPT" --type Task --summary "foo" --body "x" --print-payload --no-editor \
  2>/dev/null) || RC_23=$?
assert_eq "print-payload validates missing project" "100" "$RC_23"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 24: --quiet suppresses INFO stderr lines ==="
echo ""

start_mock "$SCENARIOS/create-201.json"
create --project ENG --type Task --summary "foo" --body "x" --quiet \
  2>/tmp/create-err24.tmp >/dev/null
stop_mock
ERR_24=$(cat /tmp/create-err24.tmp)
INFO_LINES_24=$(printf '%s\n' "$ERR_24" | grep "^INFO:" || true)
assert_empty "no INFO lines with --quiet" "$INFO_LINES_24"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 25: API 500 exits 20 with hint ==="
echo ""

start_mock "$SCENARIOS/create-500.json"
RC_25=0
(cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 \
  JIRA_RETRY_SLEEP_FN=_test_create_sleep_noop \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
  bash "$SCRIPT" --project ENG --type Task --summary "foo" --body "x" \
  2>/tmp/create-err25.tmp) || RC_25=$?
stop_mock
ERR_25=$(cat /tmp/create-err25.tmp)
assert_eq "API 500 exits 20" "20" "$RC_25"
assert_contains "5xx hint emitted" "$ERR_25" "Hint:"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 26: ADF round-trip wiring ==="
echo ""

BODY_MD_26="Hello **world** from _ADF_"
BODY_FILE_26=$(mktemp "$TMPDIR_BASE/body-XXXXXX")
printf '%s\n' "$BODY_MD_26" > "$BODY_FILE_26"
EXPECTED_ADF_26=$(printf '%s\n' "$BODY_MD_26" | bash "$SCRIPT_DIR/jira-md-to-adf.sh")

BODIES_26=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/create-201-capture.json" "$BODIES_26"
create --project ENG --type Task --summary "adf test" --body-file "$BODY_FILE_26" >/dev/null 2>/dev/null
stop_mock

CAPTURED_26=$(jq -r '.[0]' "$BODIES_26")
ADF_RC_26=0
jq -e --argjson exp "$EXPECTED_ADF_26" \
  '.fields.description == $exp' <<< "$CAPTURED_26" >/dev/null 2>&1 || ADF_RC_26=$?
assert_eq "ADF round-trip matches expected output" "0" "$ADF_RC_26"
echo ""

# ---------------------------------------------------------------------------

test_summary
