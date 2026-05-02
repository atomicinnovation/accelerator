#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/jira-fields.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-jira-server.py"

TEST_TOKEN="tok-SENTINEL-xyz123"
TEST_SITE="example"
TEST_EMAIL="test@example.com"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local d; d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.claude"
  cat > "$d/.claude/accelerator.md" <<ENDCONFIG
---
jira:
  site: $TEST_SITE
  email: $TEST_EMAIL
---
ENDCONFIG
  echo "$d"
}

REPO=$(setup_repo)

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

# Run jira-fields.sh from the test repo with injected credentials and mock URL
fields() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

# Source jira-fields.sh to access jira_field_slugify directly.
# The BASH_SOURCE guard prevents CLI dispatch.
source "$SCRIPT"

# ============================================================
echo "=== Case 1: slugify basic name ==="
echo ""

assert_eq "story points slug" "story-points" "$(jira_field_slugify "Story Points")"
echo ""

# ============================================================
echo "=== Case 2: slugify with spaces ==="
echo ""

assert_eq "epic link slug" "epic-link" "$(jira_field_slugify "Epic Link")"
echo ""

# ============================================================
echo "=== Case 3: slugify non-alphanumeric chars ==="
echo ""

assert_eq "customer champion slug" "customer-champion" "$(jira_field_slugify "Customer Champion?")"
echo ""

# ============================================================
echo "=== Case 4: slugify leading/trailing whitespace ==="
echo ""

assert_eq "spaces slug" "spaces" "$(jira_field_slugify "  Spaces  ")"
echo ""

# ============================================================
echo "=== Case 5: refresh writes fields.json with expected shape ==="
echo ""

FIELDS_JSON="$REPO/meta/integrations/jira/fields.json"
REFRESH_META="$REPO/meta/integrations/jira/.refresh-meta.json"

start_mock "$SCENARIOS/fields-200.json"
fields refresh
stop_mock

SITE_VAL=$(jq -r '.site' "$FIELDS_JSON")
assert_eq "fields.json has correct site" "$TEST_SITE" "$SITE_VAL"

FIELD_COUNT=$(jq '.fields | length' "$FIELDS_JSON")
assert_eq "fields.json has 9 fields" "9" "$FIELD_COUNT"

# Verify slug is computed and no lastRefreshed in fields.json
SLUG_SP=$(jq -r '.fields[] | select(.id=="customfield_10016") | .slug' "$FIELDS_JSON")
assert_eq "story-points slug in cache" "story-points" "$SLUG_SP"

if jq -e 'has("lastRefreshed")' "$FIELDS_JSON" > /dev/null 2>&1; then
  echo "  FAIL: fields.json must not contain lastRefreshed (use .refresh-meta.json)"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: fields.json has no lastRefreshed key"
  PASS=$((PASS + 1))
fi

if [ -f "$REFRESH_META" ]; then
  echo "  PASS: .refresh-meta.json exists"
  PASS=$((PASS + 1))
else
  echo "  FAIL: .refresh-meta.json not written"
  FAIL=$((FAIL + 1))
fi
echo ""

# ============================================================
echo "=== Case 6: resolve by slug ==="
echo ""

RESULT=$(fields resolve story-points)
assert_eq "resolve slug story-points" "customfield_10016" "$RESULT"
echo ""

# ============================================================
echo "=== Case 7: resolve by ID (pass-through) ==="
echo ""

RESULT=$(fields resolve customfield_10016)
assert_eq "resolve id direct" "customfield_10016" "$RESULT"
echo ""

# ============================================================
echo "=== Case 8: resolve nonexistent exits 50 ==="
echo ""

assert_exit_code "nonexistent exits 50" 50 fields resolve nonexistent-field-xyz
ERR=$(fields resolve nonexistent-field-xyz 2>&1 || true)
assert_contains "E_FIELD_NOT_FOUND on stderr" "E_FIELD_NOT_FOUND" "$ERR"
echo ""

# ============================================================
echo "=== Case 9: resolve by friendly name ==="
echo ""

RESULT=$(fields resolve "Story Points")
assert_eq "resolve by name" "customfield_10016" "$RESULT"
echo ""

# ============================================================
echo "=== Case 10: list prints fields array ==="
echo ""

LIST=$(fields list)
IS_ARRAY=$(printf '%s\n' "$LIST" | jq 'type')
assert_eq "list output is array" '"array"' "$IS_ARRAY"

LISTED_ID=$(printf '%s\n' "$LIST" | jq -r '.[3].id')
assert_eq "list fourth entry is customfield_10016" "customfield_10016" "$LISTED_ID"
echo ""

# ============================================================
echo "=== Case 11: resolve against absent cache exits 51 ==="
echo ""

REPO11=$(setup_repo)
assert_exit_code "absent cache exits 51" 51 \
  bash -c "cd '$REPO11' && ACCELERATOR_JIRA_TOKEN='$TEST_TOKEN' \
    ACCELERATOR_TEST_MODE=1 bash '$SCRIPT' resolve story-points"

ERR11=$(cd "$REPO11" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 bash "$SCRIPT" resolve story-points 2>&1 || true)
assert_contains "absent cache: E_FIELD_CACHE_MISSING on stderr" "E_FIELD_CACHE_MISSING" "$ERR11"
echo ""

# ============================================================
echo "=== Case 12: byte-idempotent refresh ==="
echo ""

CONTENT_BEFORE=$(cat "$FIELDS_JSON")

start_mock "$SCENARIOS/fields-200.json"
fields refresh
stop_mock

CONTENT_AFTER=$(cat "$FIELDS_JSON")
assert_eq "refresh is byte-idempotent" "$CONTENT_BEFORE" "$CONTENT_AFTER"
echo ""

# ============================================================
echo "=== Case 13: concurrent refresh — loser exits 53 ==="
echo ""

REPO13=$(setup_repo)

start_mock "$SCENARIOS/fields-slow-200.json"

# Process A: refresh — will acquire lock, call mock (0.3s delay), succeed
(cd "$REPO13" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
  bash "$SCRIPT" refresh) &
PID_A=$!

# Wait until A has acquired the lock before starting B
LOCKDIR="$REPO13/meta/integrations/jira/.lock"
i=0
while [ ! -d "$LOCKDIR" ] && [ $i -lt 30 ]; do
  sleep 0.05; i=$((i + 1))
done

# Process B: refresh with timeout 0 — will fail immediately (lock already held)
(cd "$REPO13" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
  JIRA_LOCK_TIMEOUT_SECS=0 \
  bash "$SCRIPT" refresh) &
PID_B=$!

EXIT_A=0; wait "$PID_A" || EXIT_A=$?
EXIT_B=0; wait "$PID_B" || EXIT_B=$?

stop_mock

assert_eq "concurrent: process A succeeded" "0" "$EXIT_A"
assert_eq "concurrent: process B got E_REFRESH_LOCKED" "53" "$EXIT_B"

FIELDS13="$REPO13/meta/integrations/jira/fields.json"
if [ -f "$FIELDS13" ]; then
  echo "  PASS: fields.json written by winner"
  PASS=$((PASS + 1))
else
  echo "  FAIL: fields.json missing after concurrent refresh"
  FAIL=$((FAIL + 1))
fi
echo ""

# ============================================================
echo "=== Case 14: refresh writes schema.custom for textarea custom fields ==="
echo ""

REPO14=$(setup_repo)
FIELDS14="$REPO14/meta/integrations/jira/fields.json"

start_mock "$SCENARIOS/fields-with-schema-200.json"
(cd "$REPO14" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
  bash "$SCRIPT" refresh)
stop_mock

TEXTAREA_CUSTOM=$(jq -r '.fields[] | select(.id=="customfield_10100") | .schema.custom' "$FIELDS14")
assert_eq "textarea schema.custom persisted" \
  "com.atlassian.jira.plugin.system.customfieldtypes:textarea" \
  "$TEXTAREA_CUSTOM"
echo ""

# ============================================================
echo "=== Case 15: refresh omits schema on standard fields ==="
echo ""

HAS_SCHEMA=$(jq -r '.fields[] | select(.id=="summary") | has("schema")' "$FIELDS14")
assert_eq "standard field has no schema key" "false" "$HAS_SCHEMA"

HAS_SCHEMA_DESC=$(jq -r '.fields[] | select(.id=="description") | has("schema")' "$FIELDS14")
assert_eq "description field has no schema key" "false" "$HAS_SCHEMA_DESC"
echo ""

# ============================================================
echo "=== Case 16: refresh preserves non-textarea schema.custom verbatim ==="
echo ""

TEXTFIELD_CUSTOM=$(jq -r '.fields[] | select(.id=="customfield_10200") | .schema.custom' "$FIELDS14")
assert_eq "textfield schema.custom preserved" \
  "com.atlassian.jira.plugin.system.customfieldtypes:textfield" \
  "$TEXTFIELD_CUSTOM"

FLOAT_CUSTOM=$(jq -r '.fields[] | select(.id=="customfield_10300") | .schema.custom' "$FIELDS14")
assert_eq "float schema.custom preserved" \
  "com.atlassian.jira.plugin.system.customfieldtypes:float" \
  "$FLOAT_CUSTOM"
echo ""

# ============================================================
test_summary
