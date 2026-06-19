#!/usr/bin/env bash
set -euo pipefail

# Tests for work-item-update-remote.sh (the work → integrations WRITE bridge),
# driven by the integrations' mock HTTP servers (not PATH stubs — the bridge
# invokes integration scripts by absolute path). Covered against BOTH trackers.
# Run: bash skills/work/scripts/test-work-item-update-remote.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

UPDATE="$SCRIPT_DIR/work-item-update-remote.sh"

LINEAR_SCN="$PLUGIN_ROOT/skills/integrations/linear/scripts/test-fixtures/scenarios"
LINEAR_MOCK="$PLUGIN_ROOT/skills/integrations/linear/scripts/test-helpers/mock-linear-server.py"
JIRA_SCN="$PLUGIN_ROOT/skills/integrations/jira/scripts/test-fixtures/scenarios"
JIRA_MOCK="$PLUGIN_ROOT/skills/integrations/jira/scripts/test-helpers/mock-jira-server.py"

LINEAR_TOKEN="lin_api_test123"
JIRA_TOKEN="tok-SENTINEL-xyz123"
LINEAR_STATE_REL=".accelerator/state/integrations/linear"

# Make retry backoff instant for the jira 5xx-exhaustion case (the hook must be
# exported so it is visible to jira-request.sh, three processes down).
test_nosleep() { :; }
export -f test_nosleep

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

setup_linear_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/lin-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator" "$d/$LINEAR_STATE_REL"
  cat >"$d/.accelerator/config.md" <<'CFG'
---
work:
  integration: linear
---
CFG
  cat >"$d/$LINEAR_STATE_REL/catalogue.json" <<'CAT'
{"team": {"id": "team-x-uuid", "key": "TX", "name": "Team X"}, "workflowStates": []}
CAT
  echo "$d"
}

setup_jira_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/jira-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator"
  cat >"$d/.accelerator/config.md" <<'CFG'
---
jira:
  site: example
  email: test@example.com
work:
  integration: jira
  default_project_code: ENG
---
CFG
  echo "$d"
}

MOCK_PID=""
MOCK_URL_FILE=""
MOCK_URL=""
MOCK_BODIES_FILE=""

start_mock() {
  local mock="$1" scenario="$2"
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  MOCK_BODIES_FILE=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
  python3 "$mock" --scenario "$scenario" --url-file "$MOCK_URL_FILE" \
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

captured_body() { jq -r ".[$1] // \"\"" "$MOCK_BODIES_FILE"; }
req_count() { jq 'length' "$MOCK_BODIES_FILE" 2>/dev/null || echo 0; }

BODY=$(mktemp "$TMPDIR_BASE/body-XXXXXX")
printf 'The issue body.\n\nSecond paragraph.\n' >"$BODY"

dispatch_jira() {
  local repo="$1"
  shift
  cd "$repo" && ACCELERATOR_JIRA_TOKEN="$JIRA_TOKEN" ACCELERATOR_TEST_MODE=1 \
    JIRA_RETRY_SLEEP_FN=test_nosleep \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$UPDATE" "$@"
}

dispatch_linear() {
  local repo="$1"
  shift
  cd "$repo" && ACCELERATOR_LINEAR_TOKEN="$LINEAR_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$UPDATE" "$@"
}

# ============================================================
echo "=== Dispatch + arg validation ==="
echo ""
RC=0
(bash "$UPDATE" --integration bogus update --external-id X --title T --body-file "$BODY" >/dev/null 2>&1) || RC=$?
assert_eq "bogus → unrecognised (73)" "73" "$RC"
RC=0
(bash "$UPDATE" --integration "" update --external-id X --title T --body-file "$BODY" >/dev/null 2>&1) || RC=$?
assert_eq "empty → unrecognised (73)" "73" "$RC"
RC=0
(bash "$UPDATE" --integration trello update --external-id X --title T --body-file "$BODY" >/dev/null 2>&1) || RC=$?
assert_eq "trello → not-available (72)" "72" "$RC"
RC=0
(bash "$UPDATE" --integration github-issues update --external-id X --title T --body-file "$BODY" >/dev/null 2>&1) || RC=$?
assert_eq "github-issues → not-available (72)" "72" "$RC"
RC=0
(bash "$UPDATE" --integration jira update --title T --body-file "$BODY" >/dev/null 2>&1) || RC=$?
assert_eq "missing --external-id → retryable (70)" "70" "$RC"
RC=0
(bash "$UPDATE" --integration jira update --external-id ENG-1 --body-file "$BODY" >/dev/null 2>&1) || RC=$?
assert_eq "missing --title → retryable (70)" "70" "$RC"
RC=0
(bash "$UPDATE" --integration jira update --external-id ENG-1 --title T >/dev/null 2>&1) || RC=$?
assert_eq "missing --body-file → retryable (70)" "70" "$RC"
echo ""

# ============================================================
echo "=== jira: successful update PUTs summary + body, exits 0 ==="
echo ""
REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/update-204-capture.json"
RC=0
dispatch_jira "$REPO" --integration jira update --external-id ENG-1 \
  --title "Updated title" --body-file "$BODY" >/dev/null 2>&1 || RC=$?
PUT_BODY=$(captured_body 0)
stop_mock
assert_eq "update exits 0" "0" "$RC"
assert_contains "PUT carries the new summary" "$PUT_BODY" "Updated title"
assert_contains "PUT carries the new body" "$PUT_BODY" "The issue body."
echo ""

# ============================================================
echo "=== jira: 5xx-after-PUT → terminal (71), not auto-retried ==="
echo ""
REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/update-500.json"
RC=0
dispatch_jira "$REPO" --integration jira update --external-id ENG-1 \
  --title T --body-file "$BODY" >/dev/null 2>&1 || RC=$?
stop_mock
assert_eq "jira 5xx → terminal (71)" "71" "$RC"
echo ""

# ============================================================
echo "=== jira: --dry-run forwards --print-payload and makes no write ==="
echo ""
REPO=$(setup_jira_repo)
RC=0
OUT=$(cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$JIRA_TOKEN" ACCELERATOR_TEST_MODE=1 \
  bash "$UPDATE" --integration jira update --external-id ENG-1 \
  --title "Dry title" --body-file "$BODY" --dry-run 2>/dev/null) || RC=$?
assert_eq "dry-run exits 0" "0" "$RC"
assert_eq "dry-run previews the PUT method" "PUT" "$(printf '%s' "$OUT" | jq -r '.method')"
assert_contains "dry-run payload carries the summary" "$OUT" "Dry title"
echo ""

# ============================================================
echo "=== linear: successful update sends title + description, exits 0 ==="
echo ""
REPO=$(setup_linear_repo)
start_mock "$LINEAR_MOCK" "$LINEAR_SCN/issue-update-200.json"
RC=0
dispatch_linear "$REPO" --integration linear update --external-id BLA-1 \
  --title "Updated title" --body-file "$BODY" >/dev/null 2>&1 || RC=$?
POST_BODY=$(captured_body 0)
stop_mock
assert_eq "update exits 0" "0" "$RC"
assert_contains "mutation carries the new title" "$POST_BODY" "Updated title"
assert_contains "mutation carries the new description" "$POST_BODY" "The issue body."
echo ""

# ============================================================
echo "=== linear: dropped mutation response → terminal (71) ==="
echo ""
REPO=$(setup_linear_repo)
start_mock "$LINEAR_MOCK" "$LINEAR_SCN/issue-update-dropped-200.json"
RC=0
dispatch_linear "$REPO" --integration linear update --external-id BLA-1 \
  --title T --body-file "$BODY" >/dev/null 2>&1 || RC=$?
stop_mock
assert_eq "linear dropped response → terminal (71)" "71" "$RC"
echo ""

# ============================================================
echo "=== linear: --dry-run forwards --print-payload and makes no write ==="
echo ""
REPO=$(setup_linear_repo)
RC=0
OUT=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$LINEAR_TOKEN" ACCELERATOR_TEST_MODE=1 \
  bash "$UPDATE" --integration linear update --external-id BLA-1 \
  --title "Dry title" --body-file "$BODY" --dry-run 2>/dev/null) || RC=$?
assert_eq "dry-run exits 0" "0" "$RC"
assert_eq "dry-run previews the issueUpdate operation" "issueUpdate" \
  "$(printf '%s' "$OUT" | jq -r '.operation')"
assert_contains "dry-run input carries the title" "$OUT" "Dry title"
echo ""

test_summary
