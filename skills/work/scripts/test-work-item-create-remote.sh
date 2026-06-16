#!/usr/bin/env bash
set -euo pipefail

# Tests for work-item-create-remote.sh (the work → integrations dispatcher) and
# work-item-push-decide.sh (the deterministic decision seam).
# Run: bash skills/work/scripts/test-work-item-create-remote.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

DISPATCH="$SCRIPT_DIR/work-item-create-remote.sh"
DECIDE="$SCRIPT_DIR/work-item-push-decide.sh"

LINEAR_SCN="$PLUGIN_ROOT/skills/integrations/linear/scripts/test-fixtures/scenarios"
LINEAR_MOCK="$PLUGIN_ROOT/skills/integrations/linear/scripts/test-helpers/mock-linear-server.py"
JIRA_SCN="$PLUGIN_ROOT/skills/integrations/jira/scripts/test-fixtures/scenarios"
JIRA_MOCK="$PLUGIN_ROOT/skills/integrations/jira/scripts/test-helpers/mock-jira-server.py"

LINEAR_TOKEN="lin_api_test123"
JIRA_TOKEN="tok-SENTINEL-xyz123"
LINEAR_STATE_REL=".accelerator/state/integrations/linear"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

# --- Repo fixtures ----------------------------------------------------------

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
{"team": {"id": "team-x-uuid", "key": "TX", "name": "Team X"}, "workflowStates": [{"id": "s1", "name": "Todo", "type": "unstarted", "position": 0}]}
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

# --- Mock server ------------------------------------------------------------

MOCK_PID=""
MOCK_URL_FILE=""
MOCK_URL=""

start_mock() {
  local mock="$1" scenario="$2"
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  python3 "$mock" --scenario "$scenario" --url-file "$MOCK_URL_FILE" &
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

dispatch_linear() {
  local repo="$1"
  shift
  cd "$repo" && ACCELERATOR_LINEAR_TOKEN="$LINEAR_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$DISPATCH" "$@"
}

dispatch_jira() {
  local repo="$1"
  shift
  cd "$repo" && ACCELERATOR_JIRA_TOKEN="$JIRA_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$DISPATCH" "$@"
}

BODY=$(mktemp "$TMPDIR_BASE/body-XXXXXX")
printf 'The issue body.\n\nSecond paragraph.\n' >"$BODY"

# ============================================================
echo "=== Routing: linear → bare identifier on stdout ==="
echo ""
REPO=$(setup_linear_repo)
start_mock "$LINEAR_MOCK" "$LINEAR_SCN/create-201-capture.json"
OUT=$(dispatch_linear "$REPO" --integration linear --title "A widget" --body-file "$BODY" 2>/dev/null)
RC=$?
stop_mock
assert_eq "linear routes successfully" "0" "$RC"
assert_eq "linear stdout is exactly the bare identifier" "BLA-123" "$OUT"
echo ""

# ============================================================
echo "=== Routing: jira → bare identifier on stdout (no JSON leakage) ==="
echo ""
REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/create-201-capture.json"
OUT=$(dispatch_jira "$REPO" --integration jira --title "A widget" --kind task --body-file "$BODY" 2>/dev/null)
RC=$?
stop_mock
assert_eq "jira routes successfully" "0" "$RC"
assert_eq "jira stdout is exactly the bare key" "ENG-123" "$OUT"
echo ""

# ============================================================
echo "=== Routing: configured integration drives the route (single source) ==="
echo ""
# The caller sources --integration from the SAME config read as the gate. Mirror
# that: resolve work.integration from config, pass it through, assert the route.
REPO=$(setup_linear_repo)
SYS=$(cd "$REPO" && "$PLUGIN_ROOT/scripts/config-read-work.sh" integration)
assert_eq "configured integration is linear" "linear" "$SYS"
start_mock "$LINEAR_MOCK" "$LINEAR_SCN/create-201-capture.json"
OUT=$(dispatch_linear "$REPO" --integration "$SYS" --title "A widget" --body-file "$BODY" 2>/dev/null)
stop_mock
assert_eq "routed tracker matches configured integration" "BLA-123" "$OUT"
echo ""

# ============================================================
echo "=== Routing: trello / github-issues → not-available (72) ==="
echo ""
REPO=$(setup_linear_repo)
RC=0
(cd "$REPO" && bash "$DISPATCH" --integration trello --title T --body-file "$BODY" >/dev/null 2>&1) || RC=$?
assert_eq "trello → not-available (72)" "72" "$RC"
RC=0
(cd "$REPO" && bash "$DISPATCH" --integration github-issues --title T --body-file "$BODY" >/dev/null 2>&1) || RC=$?
assert_eq "github-issues → not-available (72)" "72" "$RC"
echo ""

# ============================================================
echo "=== Routing: unrecognised / empty <sys> → fail closed (73) ==="
echo ""
REPO=$(setup_linear_repo)
RC=0
(cd "$REPO" && bash "$DISPATCH" --integration bogus --title T --body-file "$BODY" >/dev/null 2>&1) || RC=$?
assert_eq "bogus → unrecognised (73)" "73" "$RC"
RC=0
(cd "$REPO" && bash "$DISPATCH" --integration "" --title T --body-file "$BODY" >/dev/null 2>&1) || RC=$?
assert_eq "empty → unrecognised (73)" "73" "$RC"
echo ""

# ============================================================
echo "=== Taxonomy: linear pre-mutation failure → retryable-transport (70) ==="
echo ""
REPO=$(setup_linear_repo)
start_mock "$LINEAR_MOCK" "$LINEAR_SCN/bad-request-400.json"
RC=0
dispatch_linear "$REPO" --integration linear --title T --body-file "$BODY" >/dev/null 2>&1 || RC=$?
stop_mock
assert_eq "linear 400 (rejected pre-create) → retryable (70)" "70" "$RC"
echo ""

# ============================================================
echo "=== Taxonomy: linear post-mutation (response dropped) → terminal (71) ==="
echo ""
REPO=$(setup_linear_repo)
start_mock "$LINEAR_MOCK" "$LINEAR_SCN/create-response-dropped-200.json"
RC=0
dispatch_linear "$REPO" --integration linear --title T --body-file "$BODY" >/dev/null 2>&1 || RC=$?
stop_mock
assert_eq "linear dropped response → terminal (71)" "71" "$RC"
echo ""

# ============================================================
echo "=== Taxonomy: jira pre-mutation failure (400) → retryable (70) ==="
echo ""
REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/create-400-missing-summary.json"
RC=0
dispatch_jira "$REPO" --integration jira --title T --kind task --body-file "$BODY" >/dev/null 2>&1 || RC=$?
stop_mock
assert_eq "jira 400 → retryable (70)" "70" "$RC"
echo ""

# ============================================================
echo "=== Taxonomy: jira post-mutation failure (5xx) → terminal (71) ==="
echo ""
REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/create-500.json"
RC=0
dispatch_jira "$REPO" --integration jira --title T --kind task --body-file "$BODY" >/dev/null 2>&1 || RC=$?
stop_mock
assert_eq "jira 5xx → terminal (71)" "71" "$RC"
echo ""

# ============================================================
echo "=== Identifier safety check (tracker-agnostic backstop) ==="
echo ""
# Source the dispatcher to exercise the internal safety predicate directly.
# shellcheck source=work-item-create-remote.sh
source "$DISPATCH"
for safe in "BLA-123" "owner/repo#42" "AbCd1234" "atomic-innovation/accelerator#42" "user@host-1"; do
  if _wicr_identifier_safe "$safe"; then
    echo "  PASS: '$safe' accepted"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: '$safe' should be accepted"
    FAIL=$((FAIL + 1))
  fi
done
# Unsafe: empty, leading ---, leading #, embedded newline.
for unsafe in "" "---danger" "#comment" "$(printf 'a\nb')" "  # indented"; do
  if _wicr_identifier_safe "$unsafe"; then
    echo "  FAIL: unsafe value accepted: '$unsafe'"
    FAIL=$((FAIL + 1))
  else
    echo "  PASS: unsafe value rejected"
    PASS=$((PASS + 1))
  fi
done
echo ""

# ============================================================
echo "=== Dry-run preview (no create) ==="
echo ""
REPO=$(setup_jira_repo) # default_project_code=ENG
OUT=$(cd "$REPO" && bash "$DISPATCH" --integration jira --kind bug --dry-run)
assert_eq "jira dry-run prefix" "jira" "$(printf '%s' "$OUT" | cut -f1)"
assert_eq "jira dry-run resolves type" "Bug" "$(printf '%s' "$OUT" | cut -f2)"
assert_eq "jira dry-run resolves project" "ENG" "$(printf '%s' "$OUT" | cut -f4)"
OUT=$(cd "$REPO" && bash "$DISPATCH" --integration linear --kind bug --dry-run)
assert_contains "linear dry-run states no resolvable fields" "$OUT" "no user-resolvable"
# Unresolvable project in dry-run → pre-create (70), surfaced before any gate.
REPO=$(setup_linear_repo) # linear repo, no jira default_project_code
RC=0
(cd "$REPO" && bash "$DISPATCH" --integration jira --kind bug --dry-run >/dev/null 2>&1) || RC=$?
assert_eq "jira dry-run unresolvable project → 70" "70" "$RC"
RC=0
(bash "$DISPATCH" --integration trello --dry-run >/dev/null 2>&1) || RC=$?
assert_eq "trello dry-run → not-available (72)" "72" "$RC"
echo ""

# ============================================================
echo "=== Decision seam: every row of the push state machine ==="
echo ""
assert_eq "accept-success → write-once" "write-once" \
  "$(bash "$DECIDE" --code 0 --attempt 1)"
assert_eq "success-but-write-failed → loud-terminal" "loud-terminal" \
  "$(bash "$DECIDE" --code 0 --attempt 1 --write-failed)"
assert_eq "retryable, attempt 1 → retry" "retry" \
  "$(bash "$DECIDE" --code 70 --attempt 1)"
assert_eq "retryable, attempt 2 (exhausted) → local-save" "local-save" \
  "$(bash "$DECIDE" --code 70 --attempt 2)"
assert_eq "terminal-post-create → loud-terminal" "loud-terminal" \
  "$(bash "$DECIDE" --code 71 --attempt 1)"
assert_eq "terminal on the retry → loud-terminal" "loud-terminal" \
  "$(bash "$DECIDE" --code 71 --attempt 2)"
assert_eq "not-available → local-save" "local-save" \
  "$(bash "$DECIDE" --code 72 --attempt 1)"
assert_eq "unrecognised → local-save" "local-save" \
  "$(bash "$DECIDE" --code 73 --attempt 1)"
# A retry that then succeeds re-enters as code 0 → write-once.
assert_eq "retry-then-success → write-once" "write-once" \
  "$(bash "$DECIDE" --code 0 --attempt 2)"
echo ""

test_summary
