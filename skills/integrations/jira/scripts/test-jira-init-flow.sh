#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/jira-init-flow.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-jira-server.py"

TEST_TOKEN="tok-SENTINEL-xyz123"
TEST_SITE="example"
TEST_EMAIL="test@example.com"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local d; d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.claude" "$d/.accelerator"
  cat > "$d/.claude/accelerator.md" <<ENDCONFIG
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

flow() {
  cd "$REPO" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

flow_for() {
  local repo="$1"; shift
  cd "$repo" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST="${MOCK_URL:-}" \
    bash "$SCRIPT" "$@"
}

# ============================================================
echo "=== Case 1: full flow populates site.json, projects.json, fields.json ==="
echo ""

STATE_DIR="$REPO/meta/integrations/jira"
SITE_JSON="$STATE_DIR/site.json"
PROJECTS_JSON="$STATE_DIR/projects.json"
FIELDS_JSON="$STATE_DIR/fields.json"

start_mock "$SCENARIOS/init-flow-200.json"
flow
stop_mock

# site.json: exactly {site, accountId}, no other keys
SITE_KEYS=$(jq -r 'keys | sort | join(",")' "$SITE_JSON")
assert_eq "site.json has exactly site+accountId" "accountId,site" "$SITE_KEYS"
assert_eq "site.json site value" "$TEST_SITE" "$(jq -r '.site' "$SITE_JSON")"
assert_eq "site.json accountId" "redacted-account-id" "$(jq -r '.accountId' "$SITE_JSON")"

# projects.json: {site, projects: [{key,id,name}]}
PROJ_KEYS=$(jq -r 'keys | sort | join(",")' "$PROJECTS_JSON")
assert_eq "projects.json top-level keys" "projects,site" "$PROJ_KEYS"
assert_eq "projects.json site" "$TEST_SITE" "$(jq -r '.site' "$PROJECTS_JSON")"
PROJ_COUNT=$(jq '.projects | length' "$PROJECTS_JSON")
assert_eq "projects.json has 2 projects" "2" "$PROJ_COUNT"
assert_eq "first project key" "ENG" "$(jq -r '.projects[0].key' "$PROJECTS_JSON")"

# fields.json: already validated in Phase 6
FIELD_COUNT=$(jq '.fields | length' "$FIELDS_JSON")
assert_eq "fields.json populated" "9" "$FIELD_COUNT"

# No timestamps in committed files
for f in "$SITE_JSON" "$PROJECTS_JSON" "$FIELDS_JSON"; do
  if jq -e 'has("lastRefreshed") or has("lastVerified") or has("lastUpdated")' "$f" > /dev/null 2>&1; then
    echo "  FAIL: $f must not contain timestamp keys"
    FAIL=$((FAIL + 1))
  else
    echo "  PASS: $(basename "$f") has no timestamp keys"
    PASS=$((PASS + 1))
  fi
done

# After the full flow, the lock dir must be absent
assert_dir_not_exists ".lock/ absent after successful flow" "$STATE_DIR/.lock"
echo ""

# ============================================================
echo "=== Case 2: second full flow is byte-idempotent ==="
echo ""

SITE_BEFORE=$(cat "$SITE_JSON")
PROJ_BEFORE=$(cat "$PROJECTS_JSON")
FIELDS_BEFORE=$(cat "$FIELDS_JSON")

start_mock "$SCENARIOS/init-flow-200.json"
flow
stop_mock

assert_eq "site.json byte-idempotent" "$SITE_BEFORE" "$(cat "$SITE_JSON")"
assert_eq "projects.json byte-idempotent" "$PROJ_BEFORE" "$(cat "$PROJECTS_JSON")"
assert_eq "fields.json byte-idempotent" "$FIELDS_BEFORE" "$(cat "$FIELDS_JSON")"
echo ""

# ============================================================
echo "=== Case 3: --non-interactive with missing jira.site exits 60 ==="
echo ""

REPO3=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO3/.git" "$REPO3/.claude"
cat > "$REPO3/.claude/accelerator.md" <<'ENDCONFIG'
---
jira:
  email: test@example.com
---
ENDCONFIG

assert_exit_code "missing site non-interactive exits 60" 60 \
  bash -c "cd '$REPO3' && ACCELERATOR_JIRA_TOKEN='$TEST_TOKEN' \
    ACCELERATOR_TEST_MODE=1 bash '$SCRIPT' --non-interactive"

ERR3=$(cd "$REPO3" && ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN" \
  ACCELERATOR_TEST_MODE=1 bash "$SCRIPT" --non-interactive 2>&1 || true)
assert_contains "missing site: E_INIT_NEEDS_CONFIG on stderr" "$ERR3" "E_INIT_NEEDS_CONFIG"
echo ""

# ============================================================
echo "=== Case 4: list-fields prints fields array from cache ==="
echo ""

LIST=$(flow list-fields)
IS_ARRAY=$(printf '%s\n' "$LIST" | jq 'type')
assert_eq "list-fields output is array" '"array"' "$IS_ARRAY"

LIST_COUNT=$(printf '%s\n' "$LIST" | jq 'length')
assert_eq "list-fields returns all fields" "9" "$LIST_COUNT"
echo ""

# ============================================================
echo "=== Case 5: list-projects prints projects array from cache ==="
echo ""

LIST=$(flow list-projects)
IS_ARRAY=$(printf '%s\n' "$LIST" | jq 'type')
assert_eq "list-projects output is array" '"array"' "$IS_ARRAY"

LIST_COUNT=$(printf '%s\n' "$LIST" | jq 'length')
assert_eq "list-projects returns 2 projects" "2" "$LIST_COUNT"
echo ""

# ============================================================
echo "=== Case 6: refresh-fields updates fields.json only ==="
echo ""

SITE_BEFORE=$(cat "$SITE_JSON")
PROJ_BEFORE=$(cat "$PROJECTS_JSON")

start_mock "$SCENARIOS/fields-200.json"
flow refresh-fields
stop_mock

assert_eq "refresh-fields: site.json unchanged" "$SITE_BEFORE" "$(cat "$SITE_JSON")"
assert_eq "refresh-fields: projects.json unchanged" "$PROJ_BEFORE" "$(cat "$PROJECTS_JSON")"

NEW_FIELD_COUNT=$(jq '.fields | length' "$FIELDS_JSON")
assert_eq "refresh-fields: fields.json still valid" "9" "$NEW_FIELD_COUNT"
echo ""

# ============================================================
echo "=== Case 7: --non-interactive with all values set runs full flow ==="
echo ""

REPO7=$(setup_repo)

start_mock "$SCENARIOS/init-flow-200.json"
assert_exit_code "--non-interactive full flow exits 0" 0 \
  bash -c "cd '$REPO7' && ACCELERATOR_JIRA_TOKEN='$TEST_TOKEN' \
    ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST='$MOCK_URL' \
    bash '$SCRIPT' --non-interactive"
stop_mock

SITE7="$REPO7/meta/integrations/jira/site.json"
if [ -f "$SITE7" ]; then
  echo "  PASS: non-interactive: site.json written"
  PASS=$((PASS + 1))
else
  echo "  FAIL: non-interactive: site.json missing"
  FAIL=$((FAIL + 1))
fi
echo ""

# ============================================================
echo "=== Case 8: verify subcommand writes only site.json ==="
echo ""

REPO8=$(setup_repo)

start_mock "$SCENARIOS/get-200.json"
flow_for "$REPO8" verify
stop_mock

SITE8="$REPO8/meta/integrations/jira/site.json"
PROJ8="$REPO8/meta/integrations/jira/projects.json"
FIELDS8="$REPO8/meta/integrations/jira/fields.json"

if [ -f "$SITE8" ]; then
  echo "  PASS: verify: site.json written"
  PASS=$((PASS + 1))
else
  echo "  FAIL: verify: site.json missing"
  FAIL=$((FAIL + 1))
fi

if [ ! -f "$PROJ8" ]; then
  echo "  PASS: verify: projects.json not written"
  PASS=$((PASS + 1))
else
  echo "  FAIL: verify: projects.json should not exist after verify-only"
  FAIL=$((FAIL + 1))
fi

if [ ! -f "$FIELDS8" ]; then
  echo "  PASS: verify: fields.json not written"
  PASS=$((PASS + 1))
else
  echo "  FAIL: verify: fields.json should not exist after verify-only"
  FAIL=$((FAIL + 1))
fi
echo ""

# ============================================================
echo "=== Case 9: inner .gitignore written with correct rules ==="
echo ""

# Inner .gitignore must exist inside state dir (not at project root)
assert_file_exists "inner .gitignore exists at state dir" "$STATE_DIR/.gitignore"

# Must contain exactly the three rules (site.json, .refresh-meta.json, .lock/)
if grep -qFx "site.json" "$STATE_DIR/.gitignore"; then
  echo "  PASS: inner .gitignore has site.json rule"
  PASS=$((PASS + 1))
else
  echo "  FAIL: inner .gitignore missing site.json rule"
  FAIL=$((FAIL + 1))
fi
if grep -qFx ".refresh-meta.json" "$STATE_DIR/.gitignore"; then
  echo "  PASS: inner .gitignore has .refresh-meta.json rule"
  PASS=$((PASS + 1))
else
  echo "  FAIL: inner .gitignore missing .refresh-meta.json rule"
  FAIL=$((FAIL + 1))
fi
if grep -qFx ".lock/" "$STATE_DIR/.gitignore"; then
  echo "  PASS: inner .gitignore has .lock/ rule"
  PASS=$((PASS + 1))
else
  echo "  FAIL: inner .gitignore missing .lock/ rule"
  FAIL=$((FAIL + 1))
fi
RULE_COUNT=$(wc -l < "$STATE_DIR/.gitignore" 2>/dev/null | tr -d ' ' || echo 0)
assert_eq "inner .gitignore has exactly 3 rules" "3" "$RULE_COUNT"

# Idempotency: re-run must not duplicate rules
start_mock "$SCENARIOS/init-flow-200.json"
flow
stop_mock
SITE_COUNT=$(grep -cFx "site.json" "$STATE_DIR/.gitignore" || true)
assert_eq "site.json rule not duplicated" "1" "$SITE_COUNT"
META_COUNT=$(grep -cFx ".refresh-meta.json" "$STATE_DIR/.gitignore" || true)
assert_eq ".refresh-meta.json rule not duplicated" "1" "$META_COUNT"
LOCK_COUNT=$(grep -cFx ".lock/" "$STATE_DIR/.gitignore" || true)
assert_eq ".lock/ rule not duplicated" "1" "$LOCK_COUNT"

# .gitkeep must be present
assert_file_exists ".gitkeep present in state dir" "$STATE_DIR/.gitkeep"
echo ""

# ============================================================
echo "=== Case 9b: project root .gitignore NOT mutated by init-jira ==="
echo ""

REPO9B=$(setup_repo)
printf '%s\n' "# custom content" "*.log" > "$REPO9B/.gitignore"
GI_CONTENT_BEFORE=$(cat "$REPO9B/.gitignore")

start_mock "$SCENARIOS/init-flow-200.json"
flow_for "$REPO9B"
stop_mock

GI_CONTENT_AFTER=$(cat "$REPO9B/.gitignore")
assert_eq "root .gitignore not mutated" "$GI_CONTENT_BEFORE" "$GI_CONTENT_AFTER"
echo ""

# ============================================================
echo "=== Case 10: custom paths.integrations produces inner .gitignore ==="
echo ""

REPO10=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO10/.git" "$REPO10/.claude" "$REPO10/.accelerator"
cat > "$REPO10/.claude/accelerator.md" <<ENDCONFIG
---
jira:
  site: $TEST_SITE
  email: $TEST_EMAIL
work:
  default_project_code: ENG
paths:
  integrations: .state/integrations
---
ENDCONFIG

start_mock "$SCENARIOS/init-flow-200.json"
flow_for "$REPO10"
stop_mock

CUSTOM_STATE_DIR="$REPO10/.state/integrations/jira"
assert_file_exists "custom paths: inner .gitignore exists" "$CUSTOM_STATE_DIR/.gitignore"
if grep -qFx "site.json" "$CUSTOM_STATE_DIR/.gitignore"; then
  echo "  PASS: custom paths: .gitignore has site.json entry"
  PASS=$((PASS + 1))
else
  echo "  FAIL: custom paths: .gitignore missing site.json"
  FAIL=$((FAIL + 1))
fi
if grep -qFx ".lock/" "$CUSTOM_STATE_DIR/.gitignore"; then
  echo "  PASS: custom paths: .gitignore has .lock/ entry"
  PASS=$((PASS + 1))
else
  echo "  FAIL: custom paths: .gitignore missing .lock/"
  FAIL=$((FAIL + 1))
fi
if grep -qFx ".refresh-meta.json" "$CUSTOM_STATE_DIR/.gitignore"; then
  echo "  PASS: custom paths: .gitignore has .refresh-meta.json entry"
  PASS=$((PASS + 1))
else
  echo "  FAIL: custom paths: .gitignore missing .refresh-meta.json"
  FAIL=$((FAIL + 1))
fi

# Root .gitignore should NOT have legacy entries
ROOT_GI10="$REPO10/.gitignore"
if [ ! -f "$ROOT_GI10" ]; then
  echo "  PASS: custom paths: root .gitignore not created"
  PASS=$((PASS + 1))
else
  if grep -qF ".state/integrations/jira/" "$ROOT_GI10" 2>/dev/null; then
    echo "  FAIL: custom paths: root .gitignore was mutated with jira entries"
    FAIL=$((FAIL + 1))
  else
    echo "  PASS: custom paths: root .gitignore not mutated with jira entries"
    PASS=$((PASS + 1))
  fi
fi
echo ""

# ============================================================
echo "=== Case 11: .accelerator/ absent emits warning, exits 0 ==="
echo ""

REPO11=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO11/.git" "$REPO11/.claude"
cat > "$REPO11/.claude/accelerator.md" <<ENDCONFIG
---
jira:
  site: $TEST_SITE
  email: $TEST_EMAIL
work:
  default_project_code: ENG
---
ENDCONFIG

start_mock "$SCENARIOS/init-flow-200.json"
RC11=0
ERR11=$(flow_for "$REPO11" 2>&1 1>/dev/null) || RC11=$?
stop_mock

assert_eq ".accelerator/ absent: exits 0" "0" "$RC11"
assert_contains ".accelerator/ absent: warning mentions .accelerator/" \
  "$ERR11" ".accelerator/"
assert_contains ".accelerator/ absent: warning mentions accelerator:init" \
  "$ERR11" "accelerator:init"
echo ""

# ============================================================
echo "=== Case 12: .lock/ cleaned up on EXIT (failure) ==="
echo ""

REPO12=$(setup_repo)
LOCK_DIR12="$REPO12/meta/integrations/jira/.lock"
# No mock started — discover fails with connection error; lock must still be removed
flow_for "$REPO12" discover 2>/dev/null || true
assert_dir_not_exists ".lock/ cleaned up after failed discover" "$LOCK_DIR12"
echo ""

# ============================================================
test_summary
