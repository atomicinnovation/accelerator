#!/usr/bin/env bash
set -euo pipefail

# Tests for work-item-fetch-remote.sh (the work → integrations READ bridge),
# driven by the integrations' mock HTTP servers (the bridge invokes integration
# scripts by ABSOLUTE path, so PATH stubs cannot intercept them — mirror the
# create-bridge harness instead). Covered against BOTH trackers.
# Run: bash skills/work/scripts/test-work-item-fetch-remote.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

FETCH="$SCRIPT_DIR/work-item-fetch-remote.sh"

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
{"team": {"id": "team-x-uuid", "key": "TX", "name": "Team X"}, "workflowStates": [{"id": "state-prog", "name": "In Progress", "type": "started", "position": 0}]}
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

# --- Mock server (threads body + url capture, unlike the create harness) -----

MOCK_PID=""
MOCK_URL_FILE=""
MOCK_URL=""
MOCK_BODIES_FILE=""
MOCK_URLS_FILE=""

start_mock() {
  local mock="$1" scenario="$2"
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  MOCK_BODIES_FILE=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
  MOCK_URLS_FILE=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
  python3 "$mock" --scenario "$scenario" --url-file "$MOCK_URL_FILE" \
    --captured-bodies-file "$MOCK_BODIES_FILE" \
    --captured-urls-file "$MOCK_URLS_FILE" &
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

# Number of requests the mock actually received this run.
req_count() { jq 'length' "$MOCK_BODIES_FILE"; }
captured_body() { jq -r ".[$1] // \"\"" "$MOCK_BODIES_FILE"; }

dispatch_linear() {
  local repo="$1"
  shift
  cd "$repo" && ACCELERATOR_LINEAR_TOKEN="$LINEAR_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$FETCH" "$@"
}

dispatch_jira() {
  local repo="$1"
  shift
  cd "$repo" && ACCELERATOR_JIRA_TOKEN="$JIRA_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$FETCH" "$@"
}

# ============================================================
echo "=== Dispatch: unrecognised / empty / not-available <sys> ==="
echo ""
RC=0
(bash "$FETCH" --integration bogus search >/dev/null 2>&1) || RC=$?
assert_eq "bogus → unrecognised (73)" "73" "$RC"
RC=0
(bash "$FETCH" --integration "" search >/dev/null 2>&1) || RC=$?
assert_eq "empty → unrecognised (73)" "73" "$RC"
RC=0
(bash "$FETCH" --integration trello search >/dev/null 2>&1) || RC=$?
assert_eq "trello → not-available (72)" "72" "$RC"
RC=0
(bash "$FETCH" --integration github-issues search --keys X-1 >/dev/null 2>&1) || RC=$?
assert_eq "github-issues → not-available (72)" "72" "$RC"
RC=0
(bash "$FETCH" --integration jira show >/dev/null 2>&1) || RC=$?
assert_eq "show without --external-id → retryable (70)" "70" "$RC"
echo ""

# ============================================================
echo "=== jira: plain search injects --fields and forwards filters ==="
echo ""
REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/fetch-plain-search-200.json"
OUT=$(dispatch_jira "$REPO" --integration jira search --label foo 2>/dev/null)
stop_mock
BODY=$(captured_body 0)
assert_contains "request injects --fields updated" "$BODY" "updated"
assert_contains "request forwards --label foo" "$BODY" "foo"
assert_eq "response carries fields.updated" "2026-06-01T10:00:00.000+0000" \
  "$(printf '%s' "$OUT" | jq -r '.issues[0].fields.updated')"
echo ""

# ============================================================
echo "=== jira: search --keys → key-in + all-projects, no project clause ==="
echo ""
REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/fetch-keys-200.json"
OUT=$(dispatch_jira "$REPO" --integration jira search --keys ENG-1,ENG-2,ENG-9 2>/dev/null)
stop_mock
BODY=$(captured_body 0)
assert_contains "JQL is a key-set clause" "$BODY" "key in (ENG-1,ENG-2,ENG-9)"
assert_not_contains "no project = clause (all-projects)" "$BODY" "project ="
assert_eq "found = the keys present remotely" "ENG-1,ENG-2" \
  "$(printf '%s' "$OUT" | jq -r '.found | keys | sort | join(",")')"
assert_eq "missing key is absent (complete fetch)" "ENG-9" \
  "$(printf '%s' "$OUT" | jq -r '.absent | join(",")')"
assert_eq "nothing indeterminate on a clean fetch" "0" \
  "$(printf '%s' "$OUT" | jq '.indeterminate | length')"
assert_eq "found entry carries updated" "2026-06-01T10:00:00.000+0000" \
  "$(printf '%s' "$OUT" | jq -r '.found["ENG-1"].updated')"
echo ""

# ============================================================
echo "=== jira: --keys chunks at the 50-key cap (1 req at 50, 2 req at 51) ==="
echo ""
KEYS50=""
for i in $(seq 1 50); do KEYS50="${KEYS50:+$KEYS50,}ENG-$i"; done
KEYS51="$KEYS50,ENG-51"
REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/fetch-keys-200.json"
dispatch_jira "$REPO" --integration jira search --keys "$KEYS50" >/dev/null 2>&1
N=$(req_count)
stop_mock
assert_eq "50 keys → exactly 1 request" "1" "$N"

REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/fetch-keys-twochunks.json"
OUT=$(dispatch_jira "$REPO" --integration jira search --keys "$KEYS51" 2>/dev/null)
N=$(req_count)
stop_mock
assert_eq "51 keys → 2 requests (chunked)" "2" "$N"
assert_eq "merged across chunks (ENG-1 + ENG-51 recovered)" "true" \
  "$(printf '%s' "$OUT" | jq -r '(.found | has("ENG-1")) and (.found | has("ENG-51"))')"
echo ""

# ============================================================
echo "=== jira: --keys paginates to exhaustion and merges pages ==="
echo ""
REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/fetch-keys-paginated.json"
OUT=$(dispatch_jira "$REPO" --integration jira search --keys ENG-1,ENG-2 2>/dev/null)
N=$(req_count)
BODY2=$(captured_body 1)
stop_mock
assert_eq "two pages fetched" "2" "$N"
assert_contains "page 2 carries the nextPageToken" "$BODY2" "p2"
assert_eq "pages merged (ENG-1,ENG-2)" "ENG-1,ENG-2" \
  "$(printf '%s' "$OUT" | jq -r '.found | keys | sort | join(",")')"
echo ""

# ============================================================
echo "=== jira: a failed chunk → indeterminate, NEVER absent ==="
echo ""
REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/fetch-keys-400.json"
RC=0
OUT=$(dispatch_jira "$REPO" --integration jira search --keys ENG-1,ENG-2 2>/dev/null) || RC=$?
stop_mock
assert_eq "bridge still exits 0 (markers, not failure)" "0" "$RC"
assert_eq "un-fetched keys are indeterminate" "ENG-1,ENG-2" \
  "$(printf '%s' "$OUT" | jq -r '.indeterminate | sort | join(",")')"
assert_eq "nothing wrongly marked absent" "0" \
  "$(printf '%s' "$OUT" | jq '.absent | length')"
echo ""

# ============================================================
echo "=== jira: show forwards to the show flow (raw issue) ==="
echo ""
REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/issue-200.json"
OUT=$(dispatch_jira "$REPO" --integration jira show --external-id ENG-1 2>/dev/null)
stop_mock
assert_eq "show returns the issue" "ENG-1" "$(printf '%s' "$OUT" | jq -r '.key')"
REPO=$(setup_jira_repo)
start_mock "$JIRA_MOCK" "$JIRA_SCN/issue-404.json"
RC=0
dispatch_jira "$REPO" --integration jira show --external-id ENG-1 >/dev/null 2>&1 || RC=$?
stop_mock
assert_eq "show 404 → read failure (70)" "70" "$RC"
echo ""

# ============================================================
echo "=== linear: --keys → one team-wide search indexed by identifier ==="
echo ""
REPO=$(setup_linear_repo)
start_mock "$LINEAR_MOCK" "$LINEAR_SCN/fetch-keys-complete-200.json"
OUT=$(dispatch_linear "$REPO" --integration linear search --keys BLA-1,BLA-2,BLA-9 2>/dev/null)
N=$(req_count)
stop_mock
assert_eq "complete fetch indexes the tracked subset" "BLA-1,BLA-2" \
  "$(printf '%s' "$OUT" | jq -r '.found | keys | sort | join(",")')"
assert_eq "merged across the auto-paginated pages (2 distinct)" "2" \
  "$(printf '%s' "$OUT" | jq '.found | length')"
assert_eq "untracked team issue (BLA-3) excluded" "false" \
  "$(printf '%s' "$OUT" | jq -r '.found | has("BLA-3")')"
assert_eq "absent from a complete (truncated:false) fetch" "BLA-9" \
  "$(printf '%s' "$OUT" | jq -r '.absent | join(",")')"
assert_eq "found carries updated but NOT a body" "true" \
  "$(printf '%s' "$OUT" | jq -r '(.found["BLA-1"] | has("updated")) and (.found["BLA-1"] | has("body") | not)')"
echo ""

# ============================================================
echo "=== linear: truncated fetch → indeterminate, NEVER absent ==="
echo ""
REPO=$(setup_linear_repo)
start_mock "$LINEAR_MOCK" "$LINEAR_SCN/fetch-keys-truncated-200.json"
OUT=$(dispatch_linear "$REPO" --integration linear search --keys BLA-1,BLA-9 2>/dev/null)
stop_mock
assert_eq "confirmed key still found" "BLA-1" \
  "$(printf '%s' "$OUT" | jq -r '.found | keys | join(",")')"
assert_eq "un-confirmed key is indeterminate on truncation" "BLA-9" \
  "$(printf '%s' "$OUT" | jq -r '.indeterminate | join(",")')"
assert_eq "truncation never yields remote-absent" "0" \
  "$(printf '%s' "$OUT" | jq '.absent | length')"
echo ""

# ============================================================
echo "=== linear: native codes map into the shared 70/72 taxonomy ==="
echo ""
REPO=$(setup_linear_repo)
start_mock "$LINEAR_MOCK" "$LINEAR_SCN/bearer-401.json"
RC=0
dispatch_linear "$REPO" --integration linear search --keys BLA-1 >/dev/null 2>&1 || RC=$?
stop_mock
assert_eq "linear search auth failure → read failure (70)" "70" "$RC"
REPO=$(setup_linear_repo)
start_mock "$LINEAR_MOCK" "$LINEAR_SCN/show-issue-200.json"
OUT=$(dispatch_linear "$REPO" --integration linear show --external-id BLA-42 2>/dev/null)
stop_mock
assert_eq "linear show returns the issue (Markdown body)" "BLA-42" \
  "$(printf '%s' "$OUT" | jq -r '.data.issue.identifier')"
REPO=$(setup_linear_repo)
start_mock "$LINEAR_MOCK" "$LINEAR_SCN/show-issue-404.json"
RC=0
dispatch_linear "$REPO" --integration linear show --external-id BLA-999 >/dev/null 2>&1 || RC=$?
stop_mock
assert_eq "linear show not-found (82) → read failure (70)" "70" "$RC"
echo ""

# ============================================================
echo "=== linear: plain search forwards user filters verbatim ==="
echo ""
REPO=$(setup_linear_repo)
start_mock "$LINEAR_MOCK" "$LINEAR_SCN/search-filter-state-200.json"
OUT=$(dispatch_linear "$REPO" --integration linear search --state "In Progress" 2>/dev/null)
stop_mock
assert_eq "plain search returns the tracker's nodes" "2" \
  "$(printf '%s' "$OUT" | jq '.data.issues.nodes | length')"
echo ""

test_summary
