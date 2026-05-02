#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-show-flow.sh
# Run: bash skills/integrations/jira/scripts/test-jira-show.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/jira-show-flow.sh"
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

write_fields_json_with_textarea() {
  local repo="$1"
  mkdir -p "$repo/meta/integrations/jira"
  jq -cn '{
    "site": "example",
    "fields": [
      {
        "id": "customfield_10100",
        "key": "customfield_10100",
        "name": "Details",
        "schema": {
          "custom": "com.atlassian.jira.plugin.system.customfieldtypes:textarea"
        }
      }
    ]
  }' > "$repo/meta/integrations/jira/fields.json"
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

# Run the show flow from REPO with test credentials + mock URL
show() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

# ---------------------------------------------------------------------------

echo "=== Case 1: basic fetch returns issue JSON ==="
echo ""

start_mock "$SCENARIOS/issue-200.json"
OUT1=$(show ENG-1 --no-render-adf 2>/dev/null)
stop_mock

KEY1=$(printf '%s' "$OUT1" | jq -r '.key')
assert_eq "response has correct key" "ENG-1" "$KEY1"
SUMMARY1=$(printf '%s' "$OUT1" | jq -r '.fields.summary')
assert_eq "response has correct summary" "First issue" "$SUMMARY1"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 2: --fields CSV and repeatable produce identical query params ==="
echo ""

URLS2=$(mktemp "$TMPDIR_BASE/urls2-XXXXXX.json")
start_mock "$SCENARIOS/issue-url-capture.json" "" "$URLS2"
# CSV form
show ENG-1 --fields summary,status --no-render-adf >/dev/null 2>&1
# Repeatable form
show ENG-1 --fields summary --fields status --no-render-adf >/dev/null 2>&1
# Mixed form
show ENG-1 --fields "summary" --fields "status" --no-render-adf >/dev/null 2>&1
stop_mock

URL2_CSV=$(jq -r '.[0]' "$URLS2" 2>/dev/null || echo "")
URL2_REP=$(jq -r '.[1]' "$URLS2" 2>/dev/null || echo "")
URL2_MIX=$(jq -r '.[2]' "$URLS2" 2>/dev/null || echo "")
assert_contains "CSV form has fields=summary" "fields=summary" "$URL2_CSV"
assert_contains "CSV form has status in fields" "status" "$URL2_CSV"
assert_contains "repeatable form has fields=summary" "fields=summary" "$URL2_REP"
assert_contains "mixed form has fields=summary" "fields=summary" "$URL2_MIX"
assert_eq "repeatable URL matches CSV URL" "$URL2_CSV" "$URL2_REP"
assert_eq "mixed URL matches CSV URL" "$URL2_CSV" "$URL2_MIX"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 3: --expand override replaces default ==="
echo ""

URLS3=$(mktemp "$TMPDIR_BASE/urls3-XXXXXX.json")
start_mock "$SCENARIOS/issue-url-capture.json" "" "$URLS3"
show ENG-1 --expand changelog,transitions --no-render-adf >/dev/null 2>&1
stop_mock

URL3=$(jq -r '.[0]' "$URLS3" 2>/dev/null || echo "")
assert_contains "expand param has changelog" "changelog" "$URL3"
assert_contains "expand param has transitions" "transitions" "$URL3"
assert_not_contains "default expand schema not present" "schema" "$URL3"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 4: default (no --comments) omits comments from expand ==="
echo ""

URLS4=$(mktemp "$TMPDIR_BASE/urls4-XXXXXX.json")
start_mock "$SCENARIOS/issue-url-capture.json" "" "$URLS4"
show ENG-1 --no-render-adf >/dev/null 2>&1
stop_mock

URL4=$(jq -r '.[0]' "$URLS4" 2>/dev/null || echo "")
assert_not_contains "no --comments means no comments in expand" "comments" "$URL4"
assert_contains "default expand has names" "names" "$URL4"
assert_contains "default expand has schema" "schema" "$URL4"
assert_contains "default expand has transitions" "transitions" "$URL4"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5a: --comments 5 with 8 comments retains last 5 ==="
echo ""

start_mock "$SCENARIOS/issue-with-comments.json"
OUT5A=$(show ENG-1 --comments 5 --no-render-adf 2>/dev/null)
stop_mock

COUNT5A=$(printf '%s' "$OUT5A" | jq '.fields.comment.comments | length')
assert_eq "8 comments sliced to 5" "5" "$COUNT5A"
LAST5A=$(printf '%s' "$OUT5A" | jq -r '.fields.comment.comments[-1].id')
assert_eq "last comment is c8" "c8" "$LAST5A"
FIRST5A=$(printf '%s' "$OUT5A" | jq -r '.fields.comment.comments[0].id')
assert_eq "first of retained is c4" "c4" "$FIRST5A"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5b: --comments 5 with 2 comments retains both ==="
echo ""

start_mock "$SCENARIOS/issue-with-2-comments.json"
OUT5B=$(show ENG-1 --comments 5 --no-render-adf 2>/dev/null)
stop_mock

COUNT5B=$(printf '%s' "$OUT5B" | jq '.fields.comment.comments | length')
assert_eq "2 comments kept when N > length" "2" "$COUNT5B"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5c: --comments 5 with empty array retains empty array ==="
echo ""

start_mock "$SCENARIOS/issue-empty-comments.json"
OUT5C=$(show ENG-1 --comments 5 --no-render-adf 2>/dev/null)
stop_mock

COUNT5C=$(printf '%s' "$OUT5C" | jq '.fields.comment.comments | length')
assert_eq "empty comments array unchanged" "0" "$COUNT5C"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5d: --comments 5 with missing comment block leaves response unchanged ==="
echo ""

start_mock "$SCENARIOS/issue-no-comment-block.json"
OUT5D=$(show ENG-1 --comments 5 --no-render-adf 2>/dev/null)
stop_mock

HAS_COMMENT5D=$(printf '%s' "$OUT5D" | jq 'has("fields") and (.fields | has("comment"))')
assert_eq "no comment block: fields.comment absent" "false" "$HAS_COMMENT5D"
KEY5D=$(printf '%s' "$OUT5D" | jq -r '.key')
assert_eq "issue key still present" "ENG-1" "$KEY5D"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5e: --comments 3 --no-render-adf retains ADF comment bodies ==="
echo ""

start_mock "$SCENARIOS/issue-with-adf.json"
OUT5E=$(show ENG-1 --comments 3 --no-render-adf 2>/dev/null)
stop_mock

BODY_TYPE5E=$(printf '%s' "$OUT5E" | jq -r '.fields.comment.comments[0].body | type')
assert_eq "--no-render-adf keeps ADF comment bodies as objects" "object" "$BODY_TYPE5E"
COUNT5E=$(printf '%s' "$OUT5E" | jq '.fields.comment.comments | length')
assert_eq "all 3 ADF comments retained" "3" "$COUNT5E"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 6: --render-adf renders description and comment bodies ==="
echo ""

start_mock "$SCENARIOS/issue-with-adf.json"
OUT6=$(show ENG-1 --comments 2 --render-adf 2>/dev/null)
stop_mock

DESC6=$(printf '%s' "$OUT6" | jq -r '.fields.description')
assert_eq "ADF description rendered to Markdown" "hello from ADF" "$DESC6"
COMMENT_BODY6_TYPE=$(printf '%s' "$OUT6" | jq -r '.fields.comment.comments[0].body | type')
assert_eq "ADF comment body rendered to string" "string" "$COMMENT_BODY6_TYPE"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 7: missing key argument exits 80 ==="
echo ""

RESULT7=0
ERR7=$(show 2>&1 >/dev/null) || RESULT7=$?
assert_eq "no key exits 80" "80" "$RESULT7"
assert_contains "E_SHOW_NO_KEY on stderr" "E_SHOW_NO_KEY" "$ERR7"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 8: bad --comments values exit 81 ==="
echo ""

for BAD_COMMENTS in -1 200 abc; do
  RESULT=0
  show ENG-1 --comments "$BAD_COMMENTS" 2>/dev/null || RESULT=$?
  assert_eq "--comments $BAD_COMMENTS exits 81" "81" "$RESULT"
done
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 9: bad flag exits 82 with usage banner ==="
echo ""

RESULT9=0
ERR9=$(show ENG-1 --bogus-flag 2>&1 >/dev/null) || RESULT9=$?
assert_eq "bad flag exits 82" "82" "$RESULT9"
assert_contains "E_SHOW_BAD_FLAG on stderr" "E_SHOW_BAD_FLAG" "$ERR9"
assert_contains "Usage banner on stderr" "Usage:" "$ERR9"
assert_contains "offending flag name on stderr" "bogus-flag" "$ERR9"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 10a: 404 propagates as exit 13 ==="
echo ""

RESULT10A=0
start_mock "$SCENARIOS/issue-404.json"
show ENG-1 --no-render-adf 2>/tmp/show-test-err10a.tmp || RESULT10A=$?
stop_mock
ERR10A=$(cat /tmp/show-test-err10a.tmp)

assert_eq "404 exits 13" "13" "$RESULT10A"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 10b: 401 propagates as exit 11 with init-jira hint ==="
echo ""

RESULT10B=0
start_mock "$SCENARIOS/issue-401.json"
show ENG-1 --no-render-adf 2>/tmp/show-test-err10b.tmp || RESULT10B=$?
stop_mock
ERR10B=$(cat /tmp/show-test-err10b.tmp)

assert_eq "401 exits 11" "11" "$RESULT10B"
assert_contains "401 hint mentions init-jira" "init-jira" "$ERR10B"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 10c: 403 propagates as exit 12 with init-jira hint ==="
echo ""

RESULT10C=0
start_mock "$SCENARIOS/issue-403.json"
show ENG-1 --no-render-adf 2>/tmp/show-test-err10c.tmp || RESULT10C=$?
stop_mock
ERR10C=$(cat /tmp/show-test-err10c.tmp)

assert_eq "403 exits 12" "12" "$RESULT10C"
assert_contains "403 hint mentions init-jira" "init-jira" "$ERR10C"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 11: path traversal rejected by jira-request.sh (exit 17) ==="
echo ""

RESULT11A=0
show "../foo" 2>/dev/null || RESULT11A=$?
assert_eq "raw .. traversal exits 17" "17" "$RESULT11A"

RESULT11B=0
show "%2e%2e/foo" 2>/dev/null || RESULT11B=$?
assert_eq "percent-encoded .. traversal exits 17" "17" "$RESULT11B"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 12: stdout is JSON only ==="
echo ""

start_mock "$SCENARIOS/issue-200.json"
OUT12=$(show ENG-1 --no-render-adf 2>/dev/null)
stop_mock

PARSE12=$(printf '%s' "$OUT12" | jq 'type' 2>/dev/null || echo "invalid")
assert_eq "stdout is parseable JSON" '"object"' "$PARSE12"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 13a: --render-adf is ON by default ==="
echo ""

start_mock "$SCENARIOS/issue-with-adf.json"
OUT13A=$(show ENG-1 2>/dev/null)
stop_mock

DESC13A=$(printf '%s' "$OUT13A" | jq -r '.fields.description | type')
assert_eq "default renders ADF description to string" "string" "$DESC13A"
DESC13A_VAL=$(printf '%s' "$OUT13A" | jq -r '.fields.description')
assert_eq "default-rendered description is Markdown" "hello from ADF" "$DESC13A_VAL"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 13b: --no-render-adf opts out of rendering ==="
echo ""

start_mock "$SCENARIOS/issue-with-adf.json"
OUT13B=$(show ENG-1 --no-render-adf 2>/dev/null)
stop_mock

DESC13B=$(printf '%s' "$OUT13B" | jq -r '.fields.description | type')
assert_eq "--no-render-adf leaves ADF description as object" "object" "$DESC13B"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 14: --help and -h print usage to stdout and exit 0 ==="
echo ""

HELP_OUT=$(show --help 2>/dev/null)
assert_exit_code "--help exits 0" 0 show --help
assert_contains "--help: Usage present" "Usage:" "$HELP_OUT"
assert_contains "--help: shows ISSUE-KEY positional" "ISSUE-KEY" "$HELP_OUT"
assert_contains "--help: shows --comments flag" "--comments" "$HELP_OUT"
assert_contains "--help: shows --render-adf flag" "--render-adf" "$HELP_OUT"
assert_contains "--help: shows --no-render-adf flag" "--no-render-adf" "$HELP_OUT"
assert_contains "--help: shows example invocation" "ENG-1" "$HELP_OUT"

H_OUT=$(show -h 2>/dev/null)
assert_eq "-h output identical to --help" "$HELP_OUT" "$H_OUT"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 15: mixed-content render (description + custom textarea + 3 comments) ==="
echo ""

write_fields_json_with_textarea "$REPO"

start_mock "$SCENARIOS/issue-mixed-content.json"
OUT15=$(show ENG-1 --comments 3 2>/dev/null)
stop_mock

DESC15=$(printf '%s' "$OUT15" | jq -r '.fields.description')
assert_eq "description rendered from ADF" "hello from ADF" "$DESC15"

CF15=$(printf '%s' "$OUT15" | jq -r '.fields.customfield_10100')
assert_eq "custom textarea rendered from ADF" "details from ADF" "$CF15"

COMMENT15_TYPE=$(printf '%s' "$OUT15" | jq -r '.fields.comment.comments[0].body | type')
assert_eq "comment bodies rendered to strings" "string" "$COMMENT15_TYPE"
COMMENT15_COUNT=$(printf '%s' "$OUT15" | jq '.fields.comment.comments | length')
assert_eq "all 3 comments present" "3" "$COMMENT15_COUNT"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 16: --expand × --fields × --comments interaction ==="
echo ""

URLS16=$(mktemp "$TMPDIR_BASE/urls16-XXXXXX.json")
start_mock "$SCENARIOS/issue-url-capture.json" "" "$URLS16"
# Without --comments: user expand only
show ENG-1 --expand changelog,transitions --fields summary,status --no-render-adf >/dev/null 2>&1
# With --comments 5: expand gets ,comments appended
show ENG-1 --expand changelog,transitions --fields summary,status --comments 5 --no-render-adf >/dev/null 2>&1
stop_mock

URL16_NO_CMT=$(jq -r '.[0]' "$URLS16" 2>/dev/null || echo "")
URL16_WITH_CMT=$(jq -r '.[1]' "$URLS16" 2>/dev/null || echo "")

assert_contains "no --comments: expand has changelog" "changelog" "$URL16_NO_CMT"
assert_contains "no --comments: expand has transitions" "transitions" "$URL16_NO_CMT"
assert_not_contains "no --comments: expand lacks comments" "comments" "$URL16_NO_CMT"
assert_contains "no --comments: fields has summary" "summary" "$URL16_NO_CMT"

assert_contains "--comments 5: expand has changelog" "changelog" "$URL16_WITH_CMT"
assert_contains "--comments 5: expand has transitions" "transitions" "$URL16_WITH_CMT"
assert_contains "--comments 5: expand has comments" "comments" "$URL16_WITH_CMT"
echo ""

# ---------------------------------------------------------------------------
test_summary
