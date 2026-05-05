#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-search-flow.sh
# Run: bash skills/integrations/jira/scripts/test-jira-search.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/jira-search-flow.sh"
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
    "site":"example",
    "fields":[
      {"id":"summary","key":"summary","name":"Summary","slug":"summary"},
      {"id":"status","key":"status","name":"Status","slug":"status"},
      {"id":"customfield_10016","key":"customfield_10016","name":"Story Points","slug":"story-points"}
    ]
  }' > "$repo/.accelerator/state/integrations/jira/fields.json"
}

REPO=$(setup_repo)
write_site_json "$REPO"

MOCK_PID=""
MOCK_URL_FILE=""
MOCK_URL=""

start_mock() {
  local scenario="$1"
  local captured_bodies_file="${2:-}"
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  local mock_args=("--scenario" "$scenario" "--url-file" "$MOCK_URL_FILE")
  [[ -n "$captured_bodies_file" ]] && mock_args+=("--captured-bodies-file" "$captured_bodies_file")
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

# Run the search flow from REPO with test credentials + mock URL
search() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

# ---------------------------------------------------------------------------

echo "=== Case 1: basic search composes JQL and POSTs ==="
echo ""

start_mock "$SCENARIOS/search-200.json"
OUT=$(search --project ENG --status 'In Progress' 2>/tmp/search-test-err.tmp)
ERR=$(cat /tmp/search-test-err.tmp)
stop_mock

ISSUE_COUNT=$(printf '%s' "$OUT" | jq '.issues | length')
assert_eq "response has 2 issues" "2" "$ISSUE_COUNT"
assert_contains "JQL in stderr has project" "$ERR" "project = 'ENG'"
assert_contains "JQL in stderr has status IN" "$ERR" "status IN ('In Progress')"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 2: --jql raw escape hatch ==="
echo ""

start_mock "$SCENARIOS/search-200.json"
OUT2=$(search --all-projects --jql "reporter = currentUser()" 2>/tmp/search-test-err.tmp)
ERR2=$(cat /tmp/search-test-err.tmp)
stop_mock

assert_contains "raw JQL warning on stderr" "$ERR2" "raw JQL passed through"
assert_contains "JQL echo shows clause" "$ERR2" "reporter = currentUser()"
PARSE_OK=$(printf '%s' "$OUT2" | jq 'type' 2>/dev/null || echo "invalid")
assert_eq "output is JSON" '"object"' "$PARSE_OK"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 3: --assignee @me resolves from site.json ==="
echo ""

start_mock "$SCENARIOS/search-200.json"
ERR3=$(search --project ENG --assignee @me 2>&1 >/dev/null)
stop_mock

assert_contains "@me resolved to accountId in JQL" "$ERR3" "$TEST_ACCOUNT_ID"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 4: --assignee @me without site.json exits 72 ==="
echo ""

REPO4=$(setup_repo)
# No site.json written
RESULT4=0
(cd "$REPO4" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 \
  bash "$SCRIPT" --project ENG --assignee @me 2>/tmp/search-test-err.tmp) || RESULT4=$?
ERR4=$(cat /tmp/search-test-err.tmp)

assert_eq "@me without site.json exits 72" "72" "$RESULT4"
assert_contains "E_SEARCH_NO_SITE_CACHE on stderr" "$ERR4" "E_SEARCH_NO_SITE_CACHE"
assert_contains "points at /init-jira" "$ERR4" "init-jira"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5: pagination — first page returns nextPageToken ==="
echo ""

start_mock "$SCENARIOS/search-paginated-page1.json"
OUT5=$(search --project ENG --status Done 2>/dev/null)
stop_mock

TOKEN5=$(printf '%s' "$OUT5" | jq -r '.nextPageToken')
assert_eq "nextPageToken returned" "abc" "$TOKEN5"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 6: pagination — second page via --page-token ==="
echo ""

start_mock "$SCENARIOS/search-paginated-page2.json"
OUT6=$(search --project ENG --status Done --page-token abc 2>/dev/null)
stop_mock

KEY6=$(printf '%s' "$OUT6" | jq -r '.issues[0].key')
assert_eq "second page issue key" "ENG-2" "$KEY6"
HAS_TOKEN=$(printf '%s' "$OUT6" | jq 'has("nextPageToken")')
assert_eq "no nextPageToken on last page" "false" "$HAS_TOKEN"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 7: --page-token validation ==="
echo ""

CTRL_TOKEN=$'tok\tab'
RESULT7=0
search --project ENG --page-token "$CTRL_TOKEN" 2>/tmp/search-test-err.tmp || RESULT7=$?
ERR7=$(cat /tmp/search-test-err.tmp)
assert_eq "control char token exits 70" "70" "$RESULT7"
assert_contains "E_SEARCH_BAD_PAGE_TOKEN on stderr" "$ERR7" "E_SEARCH_BAD_PAGE_TOKEN"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 8: --limit validation ==="
echo ""

for BAD_LIMIT in 0 200 abc -1; do
  RESULT=0
  search --project ENG --limit "$BAD_LIMIT" 2>/dev/null || RESULT=$?
  assert_eq "--limit $BAD_LIMIT exits 71" "71" "$RESULT"
done

# --limit 50 accepted; verify maxResults appears in captured body
BODIES8=$(mktemp "$TMPDIR_BASE/bodies8-XXXXXX.json")
start_mock "$SCENARIOS/search-fields-capture.json" "$BODIES8"
search --project ENG --limit 50 2>/dev/null
stop_mock

BODY8=$(jq -r '.[0]' "$BODIES8" 2>/dev/null || echo "")
MAX_RESULTS=$(printf '%s' "$BODY8" | jq -r '.maxResults' 2>/dev/null || echo "")
assert_eq "--limit 50 sets maxResults 50" "50" "$MAX_RESULTS"

# Verify --limit error message mentions constraint and remediation
ERR8_MSG=$(search --project ENG --limit 200 2>&1 || true)
assert_contains "--limit error mentions constraint" "$ERR8_MSG" "between 1 and 100"
assert_contains "--limit error mentions page-token" "$ERR8_MSG" "page-token"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 9: --fields resolves slugs (CSV and repeatable forms) ==="
echo ""

write_fields_json "$REPO"

BODIES9=$(mktemp "$TMPDIR_BASE/bodies9-XXXXXX.json")
start_mock "$SCENARIOS/search-fields-capture.json" "$BODIES9"
# CSV form
search --project ENG --fields summary,story-points,status 2>/dev/null
# Repeatable form
search --project ENG --fields summary --fields story-points --fields status 2>/dev/null
# Mixed form
search --project ENG --fields "summary,story-points" --fields status 2>/dev/null
stop_mock

# All three bodies should have customfield_10016 (resolved from story-points)
FIELDS_CSV=$(jq -r '.[0]' "$BODIES9" | jq -c '.fields' 2>/dev/null || echo "")
FIELDS_REP=$(jq -r '.[1]' "$BODIES9" | jq -c '.fields' 2>/dev/null || echo "")
FIELDS_MIX=$(jq -r '.[2]' "$BODIES9" | jq -c '.fields' 2>/dev/null || echo "")
assert_contains "CSV form contains customfield_10016" "$FIELDS_CSV" "customfield_10016"
assert_eq "repeatable form produces same fields array" "$FIELDS_CSV" "$FIELDS_REP"
assert_eq "mixed form produces same fields array" "$FIELDS_CSV" "$FIELDS_MIX"

# Unknown slug passes through with stderr warning
WARN9=$(search --project ENG --fields unknown-xyz-field 2>&1 >/dev/null || true)
assert_contains "unknown slug warning on stderr" "$WARN9" "unknown-xyz-field"
assert_contains "warning mentions init-jira" "$WARN9" "init-jira"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 10: --render-adf pipes through M1 walker ==="
echo ""

start_mock "$SCENARIOS/search-with-adf.json"
OUT10_NO_RENDER=$(search --project ENG 2>/dev/null)
stop_mock

DESC_TYPE=$(printf '%s' "$OUT10_NO_RENDER" | jq -r '.issues[0].fields.description | type')
assert_eq "without --render-adf description is object (ADF)" "object" "$DESC_TYPE"

start_mock "$SCENARIOS/search-with-adf.json"
OUT10_RENDER=$(search --project ENG --render-adf 2>/dev/null)
stop_mock

DESC_RENDERED=$(printf '%s' "$OUT10_RENDER" | jq -r '.issues[0].fields.description')
assert_eq "with --render-adf description is rendered Markdown" "hello from ADF" "$DESC_RENDERED"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 11: negation prefix carries through ==="
echo ""

start_mock "$SCENARIOS/search-200.json"
ERR11=$(search --project ENG --status '~Done' 2>&1 >/dev/null)
stop_mock

assert_contains "negated status in JQL" "$ERR11" "status NOT IN ('Done')"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 12: combined flags compose correctly ==="
echo ""

start_mock "$SCENARIOS/search-200.json"
ERR12=$(search --project ENG --type Bug --label backend --label '~stale' 2>&1 >/dev/null)
stop_mock

assert_contains "project in JQL" "$ERR12" "project = 'ENG'"
assert_contains "issuetype in JQL" "$ERR12" "issuetype IN ('Bug')"
assert_contains "labels IN in JQL" "$ERR12" "labels IN ('backend')"
assert_contains "labels NOT IN in JQL" "$ERR12" "labels NOT IN ('stale')"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 13: bad flag exits 73 with usage banner ==="
echo ""

RESULT13=0
ERR13=$(search --project ENG --bogus-flag 2>&1 >/dev/null) || RESULT13=$?
assert_eq "bad flag exits 73" "73" "$RESULT13"
assert_contains "E_SEARCH_BAD_FLAG on stderr" "$ERR13" "E_SEARCH_BAD_FLAG"
assert_contains "Usage banner on stderr" "$ERR13" "Usage:"
assert_contains "offending flag name on stderr" "$ERR13" "bogus-flag"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 14: --help and -h print usage to stdout and exit 0 ==="
echo ""

HELP_OUT=$(search --help 2>/dev/null)
assert_exit_code "--help exits 0" 0 search --help
assert_contains "--help: Usage present" "$HELP_OUT" "Usage:"
assert_contains "--help: shows flags" "$HELP_OUT" "--project"
assert_contains "--help: shows negation note" "$HELP_OUT" "~"

H_OUT=$(search -h 2>/dev/null)
assert_eq "-h output identical to --help" "$HELP_OUT" "$H_OUT"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 15: default project from config ==="
echo ""

start_mock "$SCENARIOS/search-200.json"
ERR15=$(search 2>&1 >/dev/null)  # no --project flag
stop_mock

assert_contains "default project ENG in JQL" "$ERR15" "project = 'ENG'"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 16: --project overrides config default ==="
echo ""

start_mock "$SCENARIOS/search-200.json"
ERR16=$(search --project FOO 2>&1 >/dev/null)
stop_mock

assert_contains "overridden project FOO in JQL" "$ERR16" "project = 'FOO'"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 17: stdout is JSON only ==="
echo ""

start_mock "$SCENARIOS/search-200.json"
OUT17=$(search --project ENG 2>/dev/null)
stop_mock

PARSE17=$(printf '%s' "$OUT17" | jq 'type' 2>/dev/null || echo "invalid")
assert_eq "stdout is parseable JSON" '"object"' "$PARSE17"
echo ""

# ---------------------------------------------------------------------------
test_summary
