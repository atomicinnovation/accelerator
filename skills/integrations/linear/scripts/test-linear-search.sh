#!/usr/bin/env bash
set -euo pipefail

# Tests for linear-search-flow.sh
# Run: bash skills/integrations/linear/scripts/test-linear-search.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/linear-search-flow.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-linear-server.py"

TEST_TOKEN="lin_api_test123"
STATE_REL=".accelerator/state/integrations/linear"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

# Repo with a catalogue (Todo / In Progress / Done).
setup_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator" "$d/$STATE_REL"
  cat >"$d/$STATE_REL/catalogue.json" <<'CAT'
{
  "team": {"id": "team-x-uuid", "key": "TX", "name": "Team X"},
  "workflowStates": [
    {"id": "state-todo", "name": "Todo", "type": "unstarted", "position": 0},
    {"id": "state-prog", "name": "In Progress", "type": "started", "position": 1},
    {"id": "state-done", "name": "Done", "type": "completed", "position": 2}
  ]
}
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

search() {
  local repo="$1"
  shift
  cd "$repo" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$SCRIPT" "$@"
}

# ============================================================
echo "=== Case 1: --state filter returns exactly the matching issues ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/search-filter-state-200.json"
RESULT=$(search "$REPO" --state "In Progress" 2>/dev/null)
stop_mock
IDS=$(printf '%s' "$RESULT" | jq -r '[.data.issues.nodes[].identifier] | sort | join(",")')
assert_eq "exactly BLA-1,BLA-2 returned" "BLA-1,BLA-2" "$IDS"
COUNT=$(printf '%s' "$RESULT" | jq '.data.issues.nodes | length')
assert_eq "exactly 2 issues" "2" "$COUNT"
assert_not_contains "no BLA-3 leaked" "$IDS" "BLA-3"
echo ""

# ============================================================
echo "=== Case 2: composed filter carries the catalogue-resolved state UUID ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/search-filter-state-200.json"
search "$REPO" --state "in progress" >/dev/null 2>&1
stop_mock
BODY=$(jq -r '.[0] // ""' "$MOCK_BODIES_FILE")
assert_contains "request body carries resolved state-prog UUID" "$BODY" "state-prog"
# Case-insensitive resolution: "in progress" resolved to "In Progress"'s UUID
assert_contains "filter targets state.id.eq" "$BODY" "state-prog"
echo ""

# ============================================================
echo "=== Case 3: multi-page search returns all 150 issues in one result ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/search-paginate-200.json"
RESULT=$(search "$REPO" 2>/dev/null)
stop_mock
COUNT=$(printf '%s' "$RESULT" | jq '.data.issues.nodes | length')
assert_eq "3×50 search returns 150 issues" "150" "$COUNT"
HN=$(printf '%s' "$RESULT" | jq -r '.data.issues.pageInfo.hasNextPage')
assert_eq "search stops with hasNextPage false" "false" "$HN"
echo ""

# ============================================================
echo "=== Case 4: --state with no catalogue → E_SEARCH_NO_CATALOGUE (72) ==="
echo ""
NOCAT=$(mktemp -d "$TMPDIR_BASE/nocat-XXXXXX")
mkdir -p "$NOCAT/.git" "$NOCAT/.accelerator"
EXIT_CODE=0
STDERR=$(cd "$NOCAT" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" --state "Todo" 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_eq "no-catalogue exits 72" "72" "$EXIT_CODE"
assert_contains "E_SEARCH_NO_CATALOGUE in stderr" "$STDERR" "E_SEARCH_NO_CATALOGUE"
echo ""

# ============================================================
echo "=== Case 5: --state unknown → E_SEARCH_BAD_STATE (73) ==="
echo ""
REPO=$(setup_repo)
EXIT_CODE=0
STDERR=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" --state "Nonexistent" 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_eq "unknown state exits 73" "73" "$EXIT_CODE"
assert_contains "E_SEARCH_BAD_STATE in stderr" "$STDERR" "E_SEARCH_BAD_STATE"
echo ""

# ============================================================
echo "=== Case 6: --limit 0 → E_SEARCH_BAD_LIMIT (71); bad flag → 70 ==="
echo ""
REPO=$(setup_repo)
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" --limit 0 >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "--limit 0 exits 71" "71" "$EXIT_CODE"
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" --bogus >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "bad flag exits 70" "70" "$EXIT_CODE"
echo ""

test_summary
