#!/usr/bin/env bash
set -euo pipefail

# Tests for jira-update-flow.sh
# Run: bash skills/integrations/jira/scripts/test-jira-update.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/jira-update-flow.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-jira-server.py"

TEST_TOKEN="tok-SENTINEL-xyz123"
TEST_SITE="example"
TEST_EMAIL="test@example.com"
TEST_ACCOUNT_ID="redacted-id-789"
TEST_KEY="ENG-1"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

# ---------------------------------------------------------------------------
# Repo / mock setup helpers

setup_repo() {
  local d; d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator"
  cat > "$d/.accelerator/config.md" <<ENDCONFIG
---
jira:
  site: $TEST_SITE
  email: $TEST_EMAIL
work:
  default_project_code: ENG
---
ENDCONFIG
  echo "$d"
}

setup_repo_minimal() {
  local d; d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator"
  cat > "$d/.accelerator/config.md" <<ENDCONFIG
---
jira:
  site: $TEST_SITE
  email: $TEST_EMAIL
---
ENDCONFIG
  echo "$d"
}

write_site_json() {
  local repo="$1"
  mkdir -p "$repo/.accelerator/state/integrations/jira"
  printf '{"site":"%s","accountId":"%s"}\n' "$TEST_SITE" "$TEST_ACCOUNT_ID" \
    > "$repo/.accelerator/state/integrations/jira/site.json"
}

write_fields_json() {
  local repo="$1"
  mkdir -p "$repo/.accelerator/state/integrations/jira"
  jq -cn '{
    "site": "example",
    "fields": [
      {
        "id": "customfield_10016",
        "key": "customfield_10016",
        "name": "Story Points",
        "slug": "story-points",
        "schema": {"type": "number", "custom": "com.atlassian.jira.plugin.system.customfieldtypes:float"}
      }
    ]
  }' > "$repo/.accelerator/state/integrations/jira/fields.json"
}

REPO=$(setup_repo)
write_site_json "$REPO"
write_fields_json "$REPO"

# Sleep stub for retry tests
_test_update_sleep_noop() { :; }
export -f _test_update_sleep_noop

MOCK_PID=""
MOCK_URL_FILE=""
MOCK_URL=""

start_mock() {
  local scenario="$1"
  local captured_bodies_file="${2:-}"
  local captured_urls_file="${3:-}"
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  local mock_args=("--scenario" "$scenario" "--url-file" "$MOCK_URL_FILE")
  [[ -n "$captured_bodies_file" ]] && mock_args+=("--captured-bodies-file" "$captured_bodies_file")
  [[ -n "$captured_urls_file" ]] && mock_args+=("--captured-urls-file" "$captured_urls_file")
  python3 "$MOCK_SERVER" "${mock_args[@]}" &
  MOCK_PID=$!

  local i=0
  while [ ! -s "$MOCK_URL_FILE" ] && [ $i -lt 50 ]; do
    sleep 0.1; i=$((i + 1))
  done
  if [ ! -s "$MOCK_URL_FILE" ]; then
    echo "ERROR: mock server did not start within 5s" >&2
    kill "$MOCK_PID" 2>/dev/null || true; exit 1
  fi
  MOCK_URL=$(cat "$MOCK_URL_FILE")
}

stop_mock() {
  if [ -n "$MOCK_PID" ]; then
    kill "$MOCK_PID" 2>/dev/null || true
    wait "$MOCK_PID" 2>/dev/null || true
    MOCK_PID=""
  fi
  [ -n "$MOCK_URL_FILE" ] && { rm -f "$MOCK_URL_FILE"; MOCK_URL_FILE=""; }
  MOCK_URL=""
}

# Run the update flow from REPO with test credentials + mock URL
update() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

# Same but with stdin forced to TTY (no body/editor fallback)
update_no_stdin() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    JIRA_BODY_STDIN_IS_TTY_TEST=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

# ---------------------------------------------------------------------------

echo "=== Case 1: --help exits 0 with usage banner ==="
echo ""

OUT_1=$(update --help 2>/dev/null)
assert_contains "usage includes KEY" "$OUT_1" "KEY"
assert_contains "usage includes --summary" "$OUT_1" "--summary"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 2: no issue key exits 110 ==="
echo ""

RC_2=0
update_no_stdin --summary "x" 2>/tmp/update-err2.tmp || RC_2=$?
ERR_2=$(cat /tmp/update-err2.tmp)
assert_eq "no key exits 110" "110" "$RC_2"
assert_contains "E_UPDATE_NO_KEY on stderr" "$ERR_2" "E_UPDATE_NO_KEY"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 3: --summary sets fields.summary ==="
echo ""

BODIES_3=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_3"
update "$TEST_KEY" --summary "updated title" 2>/dev/null
stop_mock

CAPTURED_3=$(jq -r '.[0]' "$BODIES_3")
assert_eq "summary in fields" "updated title" "$(jq -r '.fields.summary' <<< "$CAPTURED_3")"
assert_eq "no update key" "null" "$(jq -r '.update // "null"' <<< "$CAPTURED_3")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 4: --body sets fields.description as ADF ==="
echo ""

BODIES_4=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_4"
update "$TEST_KEY" --body "Hello update" 2>/dev/null
stop_mock

CAPTURED_4=$(jq -r '.[0]' "$BODIES_4")
assert_eq "description is ADF doc" "doc" "$(jq -r '.fields.description.type' <<< "$CAPTURED_4")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 5: --add-label foo --add-label bar sets update.labels ==="
echo ""

BODIES_5=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_5"
update "$TEST_KEY" --add-label foo --add-label bar 2>/dev/null
stop_mock

CAPTURED_5=$(jq -r '.[0]' "$BODIES_5")
assert_eq "add-labels in update" '[{"add":"foo"},{"add":"bar"}]' \
  "$(jq -c '.update.labels' <<< "$CAPTURED_5")"
assert_eq "no fields.labels" "null" "$(jq -r '.fields.labels // "null"' <<< "$CAPTURED_5")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 6: --remove-label sets update.labels with remove op ==="
echo ""

BODIES_6=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_6"
update "$TEST_KEY" --remove-label stale 2>/dev/null
stop_mock

CAPTURED_6=$(jq -r '.[0]' "$BODIES_6")
assert_eq "remove-label in update" '[{"remove":"stale"}]' \
  "$(jq -c '.update.labels' <<< "$CAPTURED_6")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 7: --label sets fields.labels (set semantics) ==="
echo ""

BODIES_7=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_7"
update "$TEST_KEY" --label one --label two 2>/dev/null
stop_mock

CAPTURED_7=$(jq -r '.[0]' "$BODIES_7")
assert_eq "labels in fields" '["one","two"]' "$(jq -c '.fields.labels' <<< "$CAPTURED_7")"
assert_eq "no update.labels" "null" "$(jq -r '.update.labels // "null"' <<< "$CAPTURED_7")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 8: --label and --add-label conflict exits 111 ==="
echo ""

RC_8=0
update_no_stdin "$TEST_KEY" --label one --add-label two 2>/tmp/update-err8.tmp || RC_8=$?
ERR_8=$(cat /tmp/update-err8.tmp)
assert_eq "label conflict exits 111" "111" "$RC_8"
assert_contains "E_UPDATE_LABEL_MODE_CONFLICT on stderr" "$ERR_8" "E_UPDATE_LABEL_MODE_CONFLICT"
assert_contains "mutually exclusive message" "$ERR_8" "mutually exclusive"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 9: --add-component sets update.components with add op ==="
echo ""

BODIES_9=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_9"
update "$TEST_KEY" --add-component "API" 2>/dev/null
stop_mock

CAPTURED_9=$(jq -r '.[0]' "$BODIES_9")
assert_eq "add-component in update" '[{"add":{"name":"API"}}]' \
  "$(jq -c '.update.components' <<< "$CAPTURED_9")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 10: --remove-component sets update.components with remove op ==="
echo ""

BODIES_10=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_10"
update "$TEST_KEY" --remove-component "Legacy" 2>/dev/null
stop_mock

CAPTURED_10=$(jq -r '.[0]' "$BODIES_10")
assert_eq "remove-component in update" '[{"remove":{"name":"Legacy"}}]' \
  "$(jq -c '.update.components' <<< "$CAPTURED_10")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 11: --component sets fields.components (set semantics) ==="
echo ""

BODIES_11=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_11"
update "$TEST_KEY" --component "Only" 2>/dev/null
stop_mock

CAPTURED_11=$(jq -r '.[0]' "$BODIES_11")
assert_eq "component in fields" '[{"name":"Only"}]' \
  "$(jq -c '.fields.components' <<< "$CAPTURED_11")"
assert_eq "no update.components" "null" "$(jq -r '.update.components // "null"' <<< "$CAPTURED_11")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 12: --priority sets fields.priority ==="
echo ""

BODIES_12=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_12"
update "$TEST_KEY" --priority "High" 2>/dev/null
stop_mock

CAPTURED_12=$(jq -r '.[0]' "$BODIES_12")
assert_eq "priority in fields" "High" "$(jq -r '.fields.priority.name' <<< "$CAPTURED_12")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 13: --assignee @me resolves from site.json ==="
echo ""

BODIES_13=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_13"
update "$TEST_KEY" --assignee @me 2>/dev/null
stop_mock

CAPTURED_13=$(jq -r '.[0]' "$BODIES_13")
assert_eq "assignee accountId resolved" "$TEST_ACCOUNT_ID" \
  "$(jq -r '.fields.assignee.accountId' <<< "$CAPTURED_13")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 14: --assignee \"\" unassigns (accountId: null) ==="
echo ""

BODIES_14=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_14"
update "$TEST_KEY" --assignee "" 2>/dev/null
stop_mock

CAPTURED_14=$(jq -r '.[0]' "$BODIES_14")
assert_eq "assignee null for unassign" "null" "$(jq -r '.fields.assignee.accountId' <<< "$CAPTURED_14")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 14a: --assignee email exits 117 ==="
echo ""

RC_14a=0
update_no_stdin "$TEST_KEY" --assignee user@example.com 2>/tmp/update-err14a.tmp || RC_14a=$?
ERR_14a=$(cat /tmp/update-err14a.tmp)
assert_eq "email assignee exits 117" "117" "$RC_14a"
assert_contains "E_UPDATE_BAD_ASSIGNEE on stderr" "$ERR_14a" "E_UPDATE_BAD_ASSIGNEE"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 14b: --assignee with invalid chars exits 117 ==="
echo ""

RC_14b=0
update_no_stdin "$TEST_KEY" --assignee '5b10!@#$' 2>/tmp/update-err14b.tmp || RC_14b=$?
ERR_14b=$(cat /tmp/update-err14b.tmp)
assert_eq "invalid assignee exits 117" "117" "$RC_14b"
assert_contains "E_UPDATE_BAD_ASSIGNEE on stderr" "$ERR_14b" "E_UPDATE_BAD_ASSIGNEE"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 15: --parent ENG-99 sets fields.parent ==="
echo ""

BODIES_15=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_15"
update "$TEST_KEY" --parent ENG-99 2>/dev/null
stop_mock

CAPTURED_15=$(jq -r '.[0]' "$BODIES_15")
assert_eq "parent key in fields" "ENG-99" "$(jq -r '.fields.parent.key' <<< "$CAPTURED_15")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 16: --parent \"\" clears parent (null) ==="
echo ""

BODIES_16=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_16"
update "$TEST_KEY" --parent "" 2>/dev/null
stop_mock

CAPTURED_16=$(jq -r '.[0]' "$BODIES_16")
assert_eq "parent null clears parent" "null" "$(jq -r '.fields.parent' <<< "$CAPTURED_16")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 17: --custom story-points=8 coerced to number ==="
echo ""

BODIES_17=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_17"
update "$TEST_KEY" --custom story-points=8 2>/dev/null
stop_mock

CAPTURED_17=$(jq -r '.[0]' "$BODIES_17")
assert_eq "story points as number" "8" "$(jq '.fields.customfield_10016' <<< "$CAPTURED_17")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 18: --custom + --add-label + --summary all placed correctly ==="
echo ""

BODIES_18=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_18"
update "$TEST_KEY" --custom story-points=8 --add-label x --summary "updated" 2>/dev/null
stop_mock

CAPTURED_18=$(jq -r '.[0]' "$BODIES_18")
assert_eq "summary in fields" "updated" "$(jq -r '.fields.summary' <<< "$CAPTURED_18")"
assert_eq "story-points in fields" "8" "$(jq '.fields.customfield_10016' <<< "$CAPTURED_18")"
assert_eq "add-label in update" '[{"add":"x"}]' "$(jq -c '.update.labels' <<< "$CAPTURED_18")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 19: --no-notify adds notifyUsers=false to URL ==="
echo ""

URLS_19=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "" "$URLS_19"
update "$TEST_KEY" --summary "x" --no-notify 2>/dev/null
stop_mock

CAPTURED_URL_19=$(jq -r '.[0]' "$URLS_19")
assert_contains "notifyUsers=false in URL" "$CAPTURED_URL_19" "notifyUsers=false"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 20: without --no-notify URL has no notifyUsers param ==="
echo ""

URLS_20=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "" "$URLS_20"
update "$TEST_KEY" --summary "x" 2>/dev/null
stop_mock

CAPTURED_URL_20=$(jq -r '.[0]' "$URLS_20")
NOTIFY_IN_URL_20=$(printf '%s' "$CAPTURED_URL_20" | grep -c "notifyUsers" || true)
assert_eq "no notifyUsers in URL without flag" "0" "$NOTIFY_IN_URL_20"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 21: --print-payload does not call the API ==="
echo ""

URLS_21=$(mktemp "$TMPDIR_BASE/urls-XXXXXX")
start_mock "$SCENARIOS/print-payload-guard-update.json" "" "$URLS_21"
PAYLOAD_21=""
RC_21=0
PAYLOAD_21=$(update "$TEST_KEY" --summary "x" --print-payload 2>/dev/null) || RC_21=$?
stop_mock

assert_eq "print-payload exits 0" "0" "$RC_21"
CAPTURED_URLS_21=$(jq -c '.' "$URLS_21")
assert_eq "no API calls made" "[]" "$CAPTURED_URLS_21"
assert_eq "method is PUT" "PUT" "$(jq -r '.method' <<< "$PAYLOAD_21")"
assert_contains "path contains key" "$(jq -r '.path' <<< "$PAYLOAD_21")" "$TEST_KEY"
assert_eq "body is JSON object" "object" "$(jq -r '.body | type' <<< "$PAYLOAD_21")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 22: --print-payload still validates args ==="
echo ""

RC_22=0
update_no_stdin --summary "x" --print-payload 2>/dev/null || RC_22=$?
assert_eq "print-payload validates missing key" "110" "$RC_22"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 23: PUT 404 exits 13 with hint ==="
echo ""

start_mock "$SCENARIOS/update-404.json"
RC_23=0
update "$TEST_KEY" --summary "x" 2>/tmp/update-err23.tmp || RC_23=$?
stop_mock
ERR_23=$(cat /tmp/update-err23.tmp)
assert_eq "404 exits 13" "13" "$RC_23"
assert_contains "not found hint" "$ERR_23" "not found"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 24: PUT 400 bad customfield — exits 34, refresh-fields hint ==="
echo ""

start_mock "$SCENARIOS/update-400-bad-field.json"
RC_24=0
update "$TEST_KEY" --summary "x" 2>/tmp/update-err24.tmp || RC_24=$?
stop_mock
ERR_24=$(cat /tmp/update-err24.tmp)
assert_eq "400 exits 34" "34" "$RC_24"
assert_contains "refresh-fields hint" "$ERR_24" "init-jira --refresh-fields"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 25: no mutating flags exits 112 ==="
echo ""

RC_25=0
update_no_stdin "$TEST_KEY" 2>/tmp/update-err25.tmp || RC_25=$?
ERR_25=$(cat /tmp/update-err25.tmp)
assert_eq "no ops exits 112" "112" "$RC_25"
assert_contains "E_UPDATE_NO_OPS on stderr" "$ERR_25" "E_UPDATE_NO_OPS"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 26: unrecognised flag exits 113 ==="
echo ""

RC_26=0
update_no_stdin "$TEST_KEY" --summary "x" --nope 2>/tmp/update-err26.tmp || RC_26=$?
ERR_26=$(cat /tmp/update-err26.tmp)
assert_eq "bad flag exits 113" "113" "$RC_26"
assert_contains "E_UPDATE_BAD_FLAG on stderr" "$ERR_26" "E_UPDATE_BAD_FLAG"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 27: --add-label and --remove-label same value both listed ==="
echo ""

BODIES_27=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_27"
update "$TEST_KEY" --add-label foo --remove-label foo 2>/dev/null
stop_mock

CAPTURED_27=$(jq -r '.[0]' "$BODIES_27")
assert_eq "both ops in update.labels" '[{"add":"foo"},{"remove":"foo"}]' \
  "$(jq -c '.update.labels' <<< "$CAPTURED_27")"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 28: --add-label only — no top-level fields key ==="
echo ""

BODIES_28=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_28"
update "$TEST_KEY" --add-label foo 2>/dev/null
stop_mock

CAPTURED_28=$(jq -r '.[0]' "$BODIES_28")
HAS_FIELDS_28=0
jq -e 'has("fields") | not' <<< "$CAPTURED_28" >/dev/null 2>&1 || HAS_FIELDS_28=1
assert_eq "no fields key when only update ops" "0" "$HAS_FIELDS_28"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 29: PUT 500 exits 20 with hint ==="
echo ""

start_mock "$SCENARIOS/update-500.json"
RC_29=0
(cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 \
  JIRA_RETRY_SLEEP_FN=_test_update_sleep_noop \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
  bash "$SCRIPT" "$TEST_KEY" --summary "x" 2>/tmp/update-err29.tmp) || RC_29=$?
stop_mock
ERR_29=$(cat /tmp/update-err29.tmp)
assert_eq "500 exits 20" "20" "$RC_29"
assert_contains "5xx hint" "$ERR_29" "Hint:"
echo ""

# ---------------------------------------------------------------------------
echo "=== Case 30: ADF round-trip wiring ==="
echo ""

BODY_MD_30="Update body with **bold** text"
BODY_FILE_30=$(mktemp "$TMPDIR_BASE/body-XXXXXX")
printf '%s\n' "$BODY_MD_30" > "$BODY_FILE_30"
EXPECTED_ADF_30=$(printf '%s\n' "$BODY_MD_30" | bash "$SCRIPT_DIR/jira-md-to-adf.sh")

BODIES_30=$(mktemp "$TMPDIR_BASE/bodies-XXXXXX")
start_mock "$SCENARIOS/update-204-capture.json" "$BODIES_30"
update "$TEST_KEY" --body-file "$BODY_FILE_30" >/dev/null 2>/dev/null
stop_mock

CAPTURED_30=$(jq -r '.[0]' "$BODIES_30")
ADF_RC_30=0
jq -e --argjson exp "$EXPECTED_ADF_30" \
  '.fields.description == $exp' <<< "$CAPTURED_30" >/dev/null 2>&1 || ADF_RC_30=$?
assert_eq "ADF round-trip matches expected output" "0" "$ADF_RC_30"
echo ""

# ---------------------------------------------------------------------------

test_summary
