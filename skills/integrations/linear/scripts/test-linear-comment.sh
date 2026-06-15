#!/usr/bin/env bash
set -euo pipefail

# Tests for linear-comment-flow.sh
# Run: bash skills/integrations/linear/scripts/test-linear-comment.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/linear-comment-flow.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-linear-server.py"

TEST_TOKEN="lin_api_test123"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator"
  echo "$d"
}
REPO=$(setup_repo)

MOCK_PID=""
MOCK_URL_FILE=""
MOCK_URL=""
MOCK_BODIES_FILE=""

start_mock() {
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  MOCK_BODIES_FILE=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
  python3 "$MOCK_SERVER" --scenario "$1" --url-file "$MOCK_URL_FILE" \
    --captured-bodies-file "$MOCK_BODIES_FILE" &
  MOCK_PID=$!
  local i=0
  while [ ! -s "$MOCK_URL_FILE" ] && [ $i -lt 50 ]; do
    sleep 0.1
    i=$((i + 1))
  done
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

comment() {
  cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$SCRIPT" "$@"
}

MD_BODY='This is a **Markdown** comment.

- one
- two'

# ============================================================
echo "=== Case 1: comment sends commentCreate.input.body equal to the Markdown ==="
echo ""
start_mock "$SCENARIOS/comment-201-capture.json"
comment BLA-7 --body "$MD_BODY" --quiet >/dev/null 2>&1
stop_mock
BODY=$(jq -r '.[0]' "$MOCK_BODIES_FILE")
SENT=$(printf '%s' "$BODY" | jq -r '.variables.input.body')
assert_eq "input.body == submitted Markdown" "$MD_BODY" "$SENT"
assert_eq "input.issueId == identifier" "BLA-7" \
  "$(printf '%s' "$BODY" | jq -r '.variables.input.issueId')"
echo ""

# ============================================================
echo "=== Case 2: --body-file is read as the comment body ==="
echo ""
BODY_FILE=$(mktemp "$TMPDIR_BASE/body-XXXXXX")
printf 'Body from a file.\n' >"$BODY_FILE"
start_mock "$SCENARIOS/comment-201-capture.json"
comment BLA-7 --body-file "$BODY_FILE" --quiet >/dev/null 2>&1
stop_mock
BODY=$(jq -r '.[0]' "$MOCK_BODIES_FILE")
assert_contains "body-file content sent" "$(printf '%s' "$BODY" | jq -r '.variables.input.body')" \
  "Body from a file."
echo ""

# ============================================================
echo "=== Case 3: no body → E_COMMENT_NO_BODY (91); no key → 90 ==="
echo ""
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" BLA-7 >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "no body exits 91" "91" "$EXIT_CODE"
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" --body "x" >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "no key exits 90" "90" "$EXIT_CODE"
echo ""

# ============================================================
echo "=== Case 4: --print-payload makes no API call ==="
echo ""
OUT=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" BLA-7 --body "hi" --print-payload 2>/dev/null)
assert_eq "print-payload operation" "commentCreate" "$(printf '%s' "$OUT" | jq -r '.operation')"
echo ""

test_summary
