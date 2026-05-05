#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-attach-flow.sh
# Run: bash skills/integrations/jira/scripts/test-jira-attach.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/jira-attach-flow.sh"
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

attach() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

# ---------------------------------------------------------------------------

echo "=== Case 1: happy path — single file → POST 200 → exit 0, attachment JSON on stdout ==="
echo ""

FILE_1=$(mktemp "$TMPDIR_BASE/file-XXXXXX")
printf 'test content\n' > "$FILE_1"
start_mock "$SCENARIOS/attach-post-200.json"
RC_1=0
OUT_1=$(attach ENG-1 "$FILE_1" 2>/dev/null) || RC_1=$?
stop_mock

assert_eq "single file: exits 0"              "0"  "$RC_1"
assert_contains "single file: stdout has id"  "$OUT_1" '"id"'
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 2: two files → POST 200 → both filename= parts in captured body ==="
echo ""

FILE_2A=$(mktemp "$TMPDIR_BASE/a-XXXXXX.txt")
FILE_2B=$(mktemp "$TMPDIR_BASE/b-XXXXXX.txt")
printf 'file a\n' > "$FILE_2A"
printf 'file b\n' > "$FILE_2B"
BODIES_2=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/attach-post-200-two-files.json" "$BODIES_2"
RC_2=0
attach ENG-1 "$FILE_2A" "$FILE_2B" 2>/dev/null || RC_2=$?
stop_mock

BODY_2=$(jq -r '.[0]' "$BODIES_2")
COUNT_2=$(printf '%s' "$BODY_2" | grep -o 'filename=' | wc -l | tr -d ' ')
assert_eq "two files: exits 0"                "0" "$RC_2"
assert_eq "two files: body has 2 filename="   "2" "$COUNT_2"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 3: --describe dry-run — no POST, stdout has key and files ==="
echo ""

FILE_3=$(mktemp "$TMPDIR_BASE/file-XXXXXX")
RC_3=0
OUT_3=$(attach --describe ENG-1 "$FILE_3" 2>/dev/null) || RC_3=$?

assert_eq "describe: exits 0"               "0"     "$RC_3"
assert_eq "describe: key is ENG-1"          "ENG-1" "$(jq -r '.key' <<< "$OUT_3")"
assert_eq "describe: files[0] is temp path" "$FILE_3" "$(jq -r '.files[0]' <<< "$OUT_3")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 4: --describe guard — no POST made ==="
echo ""

FILE_4=$(mktemp "$TMPDIR_BASE/file-XXXXXX")
URLS_4=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/attach-describe-guard.json" "" "$URLS_4"
RC_4=0
attach --describe ENG-1 "$FILE_4" 2>/dev/null || RC_4=$?
stop_mock

assert_eq "describe guard: exits 0"         "0"   "$RC_4"
assert_eq "describe guard: no API calls"    "[]"  "$(jq -c '.' "$URLS_4")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5a: single file not found → exit 132, no network call ==="
echo ""

RC_5A=0
attach ENG-1 /nonexistent/path/file.txt \
  2>/tmp/attach-err5a.tmp || RC_5A=$?
ERR_5A=$(cat /tmp/attach-err5a.tmp)
assert_eq "missing file: exits 132"                         "132"                  "$RC_5A"
assert_contains "missing file: E_ATTACH_FILE_MISSING on stderr" "$ERR_5A" "E_ATTACH_FILE_MISSING"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5b: two files, second missing → exit 132 (fail-fast), no network call ==="
echo ""

FILE_5B=$(mktemp "$TMPDIR_BASE/file-XXXXXX")
RC_5B=0
attach ENG-1 "$FILE_5B" /nonexistent/path/second.txt \
  2>/tmp/attach-err5b.tmp || RC_5B=$?
ERR_5B=$(cat /tmp/attach-err5b.tmp)
assert_eq "missing second file: exits 132"                      "132"                  "$RC_5B"
assert_contains "missing second file: E_ATTACH_FILE_MISSING on stderr" "$ERR_5B" "E_ATTACH_FILE_MISSING"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 6: no files supplied → exit 131, no network call ==="
echo ""

RC_6=0
attach ENG-1 2>/tmp/attach-err6.tmp || RC_6=$?
ERR_6=$(cat /tmp/attach-err6.tmp)
assert_eq "no files: exits 131"                         "131"               "$RC_6"
assert_contains "no files: E_ATTACH_NO_FILES on stderr" "$ERR_6" "E_ATTACH_NO_FILES"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 7: no issue key supplied → exit 130, no network call ==="
echo ""

RC_7=0
attach 2>/tmp/attach-err7.tmp || RC_7=$?
ERR_7=$(cat /tmp/attach-err7.tmp)
assert_eq "no key: exits 130"                         "130"             "$RC_7"
assert_contains "no key: E_ATTACH_NO_KEY on stderr"   "$ERR_7" "E_ATTACH_NO_KEY"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 8: POST 403 → exits 12 ==="
echo ""

FILE_8=$(mktemp "$TMPDIR_BASE/file-XXXXXX")
start_mock "$SCENARIOS/attach-post-403.json"
RC_8=0
attach ENG-1 "$FILE_8" 2>/tmp/attach-err8.tmp || RC_8=$?
stop_mock
assert_eq "403: exits 12" "12" "$RC_8"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 9: POST 401 → exits 11 ==="
echo ""

FILE_9=$(mktemp "$TMPDIR_BASE/file-XXXXXX")
start_mock "$SCENARIOS/attach-post-401.json"
RC_9=0
attach ENG-1 "$FILE_9" 2>/tmp/attach-err9.tmp || RC_9=$?
stop_mock
assert_eq "401: exits 11" "11" "$RC_9"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 10: unrecognised flag → exit 133 ==="
echo ""

RC_10=0
attach ENG-1 --unknown-flag \
  2>/tmp/attach-err10.tmp || RC_10=$?
ERR_10=$(cat /tmp/attach-err10.tmp)
assert_eq "bad flag: exits 133"                           "133"               "$RC_10"
assert_contains "bad flag: E_ATTACH_BAD_FLAG on stderr"   "$ERR_10" "E_ATTACH_BAD_FLAG"
echo ""

# ---------------------------------------------------------------------------

test_summary
