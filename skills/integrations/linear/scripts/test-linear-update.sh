#!/usr/bin/env bash
set -euo pipefail

# Tests for linear-update-flow.sh
# Run: bash skills/integrations/linear/scripts/test-linear-update.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/linear-update-flow.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-linear-server.py"

TEST_TOKEN="lin_api_test123"
STATE_REL=".accelerator/state/integrations/linear"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator" "$d/$STATE_REL"
  cat >"$d/$STATE_REL/catalogue.json" <<'CAT'
{"team": {"id": "team-x-uuid", "key": "TX", "name": "Team X"},
 "workflowStates": [
   {"id": "state-todo", "name": "Todo", "type": "unstarted", "position": 0},
   {"id": "state-prog", "name": "In Progress", "type": "started", "position": 1}
 ]}
CAT
  echo "$d"
}

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

update() {
  local repo="$1"
  shift
  cd "$repo" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$SCRIPT" "$@"
}

# ============================================================
echo "=== Case 1: --title + --state → payload has catalogue-resolved stateId ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/update-200-capture.json"
update "$REPO" BLA-5 --title "New Title" --state "In Progress" --quiet >/dev/null 2>&1
stop_mock
BODY=$(jq -r '.[0]' "$MOCK_BODIES_FILE")
assert_eq "issueUpdate id == identifier" "BLA-5" \
  "$(printf '%s' "$BODY" | jq -r '.variables.id')"
assert_eq "input.title == T" "New Title" \
  "$(printf '%s' "$BODY" | jq -r '.variables.input.title')"
assert_eq "input.stateId == catalogue-resolved UUID" "state-prog" \
  "$(printf '%s' "$BODY" | jq -r '.variables.input.stateId')"
echo ""

# ============================================================
echo "=== Case 2: case-insensitive state resolution ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/update-200-capture.json"
update "$REPO" BLA-5 --state "in progress" --quiet >/dev/null 2>&1
stop_mock
BODY=$(jq -r '.[0]' "$MOCK_BODIES_FILE")
assert_eq "lowercase state resolves to state-prog" "state-prog" \
  "$(printf '%s' "$BODY" | jq -r '.variables.input.stateId')"
echo ""

# ============================================================
echo "=== Case 3: no mutating flags → E_UPDATE_NO_OPS (111) ==="
echo ""
REPO=$(setup_repo)
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" BLA-5 >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "no-ops exits 111" "111" "$EXIT_CODE"
echo ""

# ============================================================
echo "=== Case 4: unknown state → E_UPDATE_BAD_STATE (114); no key → 110 ==="
echo ""
REPO=$(setup_repo)
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" BLA-5 --state "Nope" >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "unknown state exits 114" "114" "$EXIT_CODE"
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" --title X >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "no key exits 110" "110" "$EXIT_CODE"
echo ""

# ============================================================
echo "=== Case 5: --print-payload makes no API call ==="
echo ""
REPO=$(setup_repo)
OUT=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" BLA-5 --title T --print-payload 2>/dev/null)
assert_eq "print-payload operation" "issueUpdate" "$(printf '%s' "$OUT" | jq -r '.operation')"
assert_eq "print-payload id" "BLA-5" "$(printf '%s' "$OUT" | jq -r '.id')"
echo ""

test_summary
