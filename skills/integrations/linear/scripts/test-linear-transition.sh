#!/usr/bin/env bash
set -euo pipefail

# Tests for linear-transition-flow.sh
# Run: bash skills/integrations/linear/scripts/test-linear-transition.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/linear-transition-flow.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-linear-server.py"

TEST_TOKEN="lin_api_test123"
STATE_REL=".accelerator/state/integrations/linear"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

# Repo with a normal catalogue.
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

# Repo whose catalogue has two states sharing a display name.
setup_dup_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/dup-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator" "$d/$STATE_REL"
  cat >"$d/$STATE_REL/catalogue.json" <<'CAT'
{"team": {"id": "team-x-uuid", "key": "TX", "name": "Team X"},
 "workflowStates": [
   {"id": "state-done-a", "name": "Done", "type": "completed", "position": 0},
   {"id": "state-done-b", "name": "Done", "type": "canceled", "position": 1}
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

trans() {
  local repo="$1"
  shift
  cd "$repo" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$SCRIPT" "$@"
}

# ============================================================
echo "=== Case 1: transition resolves stateId from catalogue (no live lookup) ==="
echo ""
# The mock has ONLY an issueUpdate expectation — no team/catalogue query is
# served. The transition still succeeds, proving cache resolution.
REPO=$(setup_repo)
start_mock "$SCENARIOS/transition-update-200.json"
trans "$REPO" BLA-9 "In Progress" --quiet >/dev/null 2>&1
stop_mock
BODY=$(jq -r '.[0]' "$MOCK_BODIES_FILE")
assert_eq "issueUpdate carries catalogue-resolved stateId" "state-prog" \
  "$(printf '%s' "$BODY" | jq -r '.variables.input.stateId')"
assert_eq "issueUpdate id == identifier" "BLA-9" \
  "$(printf '%s' "$BODY" | jq -r '.variables.id')"
echo ""

# ============================================================
echo "=== Case 2: case-insensitive state name resolution ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/transition-update-200.json"
trans "$REPO" BLA-9 "in progress" --quiet >/dev/null 2>&1
stop_mock
BODY=$(jq -r '.[0]' "$MOCK_BODIES_FILE")
assert_eq "lowercase resolves to state-prog" "state-prog" \
  "$(printf '%s' "$BODY" | jq -r '.variables.input.stateId')"
echo ""

# ============================================================
echo "=== Case 3: duplicate display name → E_TRANSITION_STATE_AMBIGUOUS (123) ==="
echo ""
DUP=$(setup_dup_repo)
EXIT_CODE=0
STDERR=$(cd "$DUP" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" BLA-9 "Done" 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_eq "ambiguous state exits 123" "123" "$EXIT_CODE"
assert_contains "E_TRANSITION_STATE_AMBIGUOUS in stderr" "$STDERR" "E_TRANSITION_STATE_AMBIGUOUS"
echo ""

# ============================================================
echo "=== Case 4: error paths — not-in-catalogue / no-key / no-state / no-catalogue ==="
echo ""
REPO=$(setup_repo)
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" BLA-9 "Nope" >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "unknown state exits 122" "122" "$EXIT_CODE"
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  bash "$SCRIPT" 2>/dev/null) >/dev/null 2>&1 || EXIT_CODE=$?
assert_eq "no key exits 120" "120" "$EXIT_CODE"
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  bash "$SCRIPT" BLA-9 >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "no state exits 121" "121" "$EXIT_CODE"
NOCAT=$(mktemp -d "$TMPDIR_BASE/nocat-XXXXXX")
mkdir -p "$NOCAT/.git" "$NOCAT/.accelerator"
EXIT_CODE=0
(cd "$NOCAT" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" BLA-9 "Todo" >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "no catalogue exits 124" "124" "$EXIT_CODE"
echo ""

# ============================================================
echo "=== Case 5: --describe resolves without an API call ==="
echo ""
REPO=$(setup_repo)
OUT=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" BLA-9 "Todo" --describe 2>/dev/null)
assert_eq "describe resolves stateId" "state-todo" "$(printf '%s' "$OUT" | jq -r '.stateId')"
echo ""

test_summary
