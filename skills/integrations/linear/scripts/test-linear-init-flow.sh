#!/usr/bin/env bash
set -euo pipefail

# Tests for linear-init-flow.sh
# Run: bash skills/integrations/linear/scripts/test-linear-init-flow.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/linear-init-flow.sh"
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

MOCK_PID=""
MOCK_URL_FILE=""
MOCK_URL=""

start_mock() {
  local scenario="$1"
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  python3 "$MOCK_SERVER" --scenario "$scenario" --url-file "$MOCK_URL_FILE" &
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

# Run the flow from $1 (repo), passing remaining args; injects creds + mock URL.
flow() {
  local repo="$1"
  shift
  cd "$repo" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    LINEAR_LOCK_TIMEOUT_SECS=5 LINEAR_LOCK_SLEEP_SECS=0.05 \
    bash "$SCRIPT" "$@"
}

STATE_REL=".accelerator/state/integrations/linear"

# ============================================================
echo "=== Case 1: verify persists viewer.json {id,name} ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/viewer-200.json"
flow "$REPO" verify >/dev/null 2>&1
stop_mock
assert_file_exists "viewer.json written" "$REPO/$STATE_REL/viewer.json"
assert_json_eq "viewer.json id" '.id' "viewer-uuid-1" "$REPO/$STATE_REL/viewer.json"
assert_json_eq "viewer.json name" '.name' "Test User" "$REPO/$STATE_REL/viewer.json"

echo "Test: inner .gitignore lists viewer.json (gitignored)"
assert_contains "gitignore has viewer.json" "$(cat "$REPO/$STATE_REL/.gitignore")" "viewer.json"
echo ""

# ============================================================
echo "=== Case 2: Bearer-prefixed / invalid token → non-zero + auth message ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/bearer-401.json"
EXIT_CODE=0
STDERR=$(flow "$REPO" verify 2>&1 1>/dev/null) || EXIT_CODE=$?
stop_mock
if [ "$EXIT_CODE" -ne 0 ]; then
  echo "  PASS: verify exits non-zero on auth failure"
  PASS=$((PASS + 1))
else
  echo "  FAIL: verify should exit non-zero on auth failure"
  FAIL=$((FAIL + 1))
fi
assert_contains "auth-failure message mentions Bearer" "$STDERR" "Bearer"
echo ""

# ============================================================
echo "=== Case 3: list-teams prints team JSON ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/teams-200.json"
OUT=$(flow "$REPO" list-teams 2>/dev/null)
stop_mock
assert_contains "lists Team X key" "$OUT" "TX"
assert_contains "lists Team Y key" "$OUT" "TY"
KEYS=$(printf '%s' "$OUT" | jq -r '[.[].key] | join(",")')
assert_eq "exactly the two teams" "TX,TY" "$KEYS"
echo ""

# ============================================================
echo "=== Case 4: discover --team-id persists catalogue.json ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/team-states-200.json"
flow "$REPO" discover --team-id team-x-uuid >/dev/null 2>&1
stop_mock
assert_file_exists "catalogue.json written" "$REPO/$STATE_REL/catalogue.json"
assert_json_eq "catalogue team id" '.team.id' "team-x-uuid" "$REPO/$STATE_REL/catalogue.json"
assert_json_eq "catalogue team key" '.team.key' "TX" "$REPO/$STATE_REL/catalogue.json"
COUNT=$(jq '.workflowStates | length' "$REPO/$STATE_REL/catalogue.json")
assert_eq "catalogue has 3 states" "3" "$COUNT"
assert_json_eq "In Progress state UUID resolvable" \
  '.workflowStates[] | select(.name=="In Progress") | .id' "state-prog" "$REPO/$STATE_REL/catalogue.json"

echo "Test: catalogue.json is NOT gitignored (team-shared, committed)"
assert_not_contains "gitignore omits catalogue.json" "$(cat "$REPO/$STATE_REL/.gitignore")" "catalogue.json"
echo ""

# ============================================================
echo "=== Case 5: single-team scoping — selecting Y persists only Y's states ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/team-states-y-200.json"
flow "$REPO" discover --team-id team-y-uuid >/dev/null 2>&1
stop_mock
assert_json_eq "catalogue scoped to Team Y" '.team.key' "TY" "$REPO/$STATE_REL/catalogue.json"
Y_STATES=$(jq -r '[.workflowStates[].name] | sort | join(",")' "$REPO/$STATE_REL/catalogue.json")
assert_eq "only Y's states persisted" "Backlog,Shipped" "$Y_STATES"
# None of Team X's states leak in
assert_not_contains "no Team X 'In Progress' state" \
  "$(jq -r '[.workflowStates[].name] | join(",")' "$REPO/$STATE_REL/catalogue.json")" "In Progress"
echo ""

# ============================================================
echo "=== Case 6: team with no states → E_INIT_NO_TEAM (62) ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/team-no-states-200.json"
EXIT_CODE=0
STDERR=$(flow "$REPO" discover --team-id team-x-uuid 2>&1 1>/dev/null) || EXIT_CODE=$?
stop_mock
assert_eq "no-states discover exits 62" "62" "$EXIT_CODE"
assert_contains "E_INIT_NO_TEAM in stderr" "$STDERR" "E_INIT_NO_TEAM"
assert_file_not_exists "no catalogue written on failure" "$REPO/$STATE_REL/catalogue.json"
echo ""

# ============================================================
echo "=== Case 7: --non-interactive with no token → E_INIT_NEEDS_CONFIG (60) ==="
echo ""
REPO=$(setup_repo)
EXIT_CODE=0
STDERR=$(cd "$REPO" && ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" --non-interactive verify 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_eq "non-interactive no-config exits 60" "60" "$EXIT_CODE"
assert_contains "E_INIT_NEEDS_CONFIG in stderr" "$STDERR" "E_INIT_NEEDS_CONFIG"
echo ""

test_summary
