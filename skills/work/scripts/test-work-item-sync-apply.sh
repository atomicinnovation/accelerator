#!/usr/bin/env bash
set -euo pipefail

# Mock-server-driven tests for work-item-sync-apply.sh's PUSH path (update bridge
# side-effect + post-push show + baseline set). The mock-free apply paths (pull,
# finalise, fault-injection resumability) live in test-work-item-scripts.sh; the
# push path needs the integration mock because it writes to and re-reads the
# remote.
# Run: bash skills/work/scripts/test-work-item-sync-apply.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"
# shellcheck source=scripts/hash-common.sh
source "$PLUGIN_ROOT/scripts/hash-common.sh"

APPLY="$SCRIPT_DIR/work-item-sync-apply.sh"
BASELINE="$SCRIPT_DIR/work-item-sync-baseline.sh"
NORMALISE="$SCRIPT_DIR/work-item-normalise.sh"

JIRA_SCN="$PLUGIN_ROOT/skills/integrations/jira/scripts/test-fixtures/scenarios"
JIRA_MOCK="$PLUGIN_ROOT/skills/integrations/jira/scripts/test-helpers/mock-jira-server.py"
JIRA_TOKEN="tok-SENTINEL-xyz123"

test_nosleep() { :; }
export -f test_nosleep

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

setup_jira_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/jira-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator" "$d/meta/work"
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

start_mock() {
  local scenario="$1"
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  python3 "$JIRA_MOCK" --scenario "$scenario" --url-file "$MOCK_URL_FILE" &
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

write_item() {
  cat >"$1" <<'ITEM'
---
id: "0050"
external_id: "ENG-1"
title: "Pushed title"
kind: story
status: ready
---

# 0050: Pushed title

## Summary

Local content to push.
ITEM
}

# ============================================================
echo "=== apply push: update + post-push show → baseline set ==="
echo ""
REPO=$(setup_jira_repo)
LOCALFILE="$REPO/meta/work/0050-x.md"
write_item "$LOCALFILE"
PUSHBODY=$(mktemp "$TMPDIR_BASE/pushbody-XXXXXX")
printf 'Local content to push.\n' >"$PUSHBODY"

start_mock "$JIRA_SCN/apply-push-204-show.json"
RC=0
(cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$JIRA_TOKEN" ACCELERATOR_TEST_MODE=1 \
  JIRA_RETRY_SLEEP_FN=test_nosleep \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
  bash "$APPLY" push --integration jira --external-id ENG-1 --id 0050 \
  --file "$LOCALFILE" --title "Pushed title" --body-file "$PUSHBODY") || RC=$?
stop_mock
assert_eq "apply push exits 0" "0" "$RC"
ENTRY=$(cd "$REPO" && bash "$BASELINE" get 0050)
assert_eq "baseline remote_updated_at from the post-push show" \
  "2026-07-09T08:00:00.000+0000" \
  "$(printf '%s' "$ENTRY" | jq -r '.remote_updated_at')"
assert_eq "baseline local_hash is the local file's normalised hash" \
  "$(bash "$NORMALISE" "$LOCALFILE" | hash_sha256_stdin)" \
  "$(printf '%s' "$ENTRY" | jq -r '.local_hash')"
assert_neq "baseline remote_hash is set from show fidelity" "" \
  "$(printf '%s' "$ENTRY" | jq -r '.remote_hash')"
echo ""

# ============================================================
echo "=== apply push: terminal update (71) → baseline left UNSET, not retried ==="
echo ""
REPO=$(setup_jira_repo)
LOCALFILE="$REPO/meta/work/0050-x.md"
write_item "$LOCALFILE"
start_mock "$JIRA_SCN/update-500.json"
RC=0
(cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$JIRA_TOKEN" ACCELERATOR_TEST_MODE=1 \
  JIRA_RETRY_SLEEP_FN=test_nosleep \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
  bash "$APPLY" push --integration jira --external-id ENG-1 --id 0050 \
  --file "$LOCALFILE" --title "Pushed title" --body-file "$PUSHBODY") || RC=$?
stop_mock
assert_eq "terminal update surfaces 71" "71" "$RC"
assert_eq "baseline entry left unset on terminal push" "" \
  "$(cd "$REPO" && bash "$BASELINE" get 0050)"
echo ""

test_summary
