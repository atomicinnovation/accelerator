#!/usr/bin/env bash
set -euo pipefail

# Tests for linear-show-flow.sh
# Run: bash skills/integrations/linear/scripts/test-linear-show.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/linear-show-flow.sh"
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

start_mock() {
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  python3 "$MOCK_SERVER" --scenario "$1" --url-file "$MOCK_URL_FILE" &
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

show() {
  cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$SCRIPT" "$@"
}

# ============================================================
echo "=== Case 1: show reports I/T/S/A/D for each field ==="
echo ""
start_mock "$SCENARIOS/show-issue-200.json"
RESULT=$(show BLA-42 2>/dev/null)
stop_mock
assert_eq "identifier I" "BLA-42" "$(printf '%s' "$RESULT" | jq -r '.data.issue.identifier')"
assert_eq "title T" "The title T" "$(printf '%s' "$RESULT" | jq -r '.data.issue.title')"
assert_eq "state S" "In Review" "$(printf '%s' "$RESULT" | jq -r '.data.issue.state.name')"
assert_eq "assignee A" "Carol" "$(printf '%s' "$RESULT" | jq -r '.data.issue.assignee.name')"
assert_eq "description D" "The description D, in **Markdown**." \
  "$(printf '%s' "$RESULT" | jq -r '.data.issue.description')"
echo ""

# ============================================================
echo "=== Case 2: --comments N slices to the last N ==="
echo ""
start_mock "$SCENARIOS/show-issue-200.json"
RESULT=$(show BLA-42 --comments 1 2>/dev/null)
stop_mock
NC=$(printf '%s' "$RESULT" | jq '.data.issue.comments.nodes | length')
assert_eq "kept exactly 1 comment" "1" "$NC"
assert_eq "kept the LAST comment" "second comment" \
  "$(printf '%s' "$RESULT" | jq -r '.data.issue.comments.nodes[0].body')"
echo ""

# ============================================================
echo "=== Case 3: unknown identifier → E_SHOW_NOT_FOUND (82) ==="
echo ""
start_mock "$SCENARIOS/show-issue-404.json"
EXIT_CODE=0
STDERR=$(show BLA-999 2>&1 1>/dev/null) || EXIT_CODE=$?
stop_mock
assert_eq "not found exits 82" "82" "$EXIT_CODE"
assert_contains "E_SHOW_NOT_FOUND in stderr" "$STDERR" "E_SHOW_NOT_FOUND"
echo ""

# ============================================================
echo "=== Case 4: no identifier → E_SHOW_NO_KEY (80) ==="
echo ""
EXIT_CODE=0
STDERR=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_eq "no key exits 80" "80" "$EXIT_CODE"
assert_contains "E_SHOW_NO_KEY in stderr" "$STDERR" "E_SHOW_NO_KEY"
echo ""

test_summary
