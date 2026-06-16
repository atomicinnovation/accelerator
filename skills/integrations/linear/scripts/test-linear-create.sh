#!/usr/bin/env bash
set -euo pipefail

# Tests for linear-create-flow.sh (incl. work_item_id writeback)
# Run: bash skills/integrations/linear/scripts/test-linear-create.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/linear-create-flow.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-linear-server.py"

TEST_TOKEN="lin_api_test123"
STATE_REL=".accelerator/state/integrations/linear"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

# Repo with a catalogue (team id) and a work-item file under work/.
setup_repo() {
  local wid_value="$1"
  local d
  d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator" "$d/$STATE_REL" "$d/work"
  cat >"$d/$STATE_REL/catalogue.json" <<'CAT'
{"team": {"id": "team-x-uuid", "key": "TX", "name": "Team X"}, "workflowStates": [{"id": "s1", "name": "Todo", "type": "unstarted", "position": 0}]}
CAT
  cat >"$d/work/item.md" <<EOF
---
id: "0048"
title: Implement the widget
work_item_id: $wid_value
status: ready
---

This is the body.

It has two paragraphs.
EOF
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

create() {
  local repo="$1"
  shift
  cd "$repo" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$SCRIPT" "$@"
}

# ============================================================
echo "=== Case 1: create from numeric work_item_id — title/description/writeback ==="
echo ""
REPO=$(setup_repo "0048")
start_mock "$SCENARIOS/create-201-capture.json"
OUT=$(create "$REPO" work/item.md --quiet 2>/dev/null)
stop_mock
assert_eq "prints the new identifier" "BLA-123" "$OUT"
assert_eq "work_item_id rewritten to BLA-123" "work_item_id: BLA-123" \
  "$(grep '^work_item_id:' "$REPO/work/item.md")"
BODY=$(jq -r '.[0]' "$MOCK_BODIES_FILE")
assert_eq "issueCreate input.title == work-item title" "Implement the widget" \
  "$(printf '%s' "$BODY" | jq -r '.variables.input.title')"
DESC=$(printf '%s' "$BODY" | jq -r '.variables.input.description')
assert_contains "description carries the body" "$DESC" "This is the body."
assert_contains "description carries both paragraphs" "$DESC" "two paragraphs"
assert_eq "input.teamId from catalogue" "team-x-uuid" \
  "$(printf '%s' "$BODY" | jq -r '.variables.input.teamId')"
echo ""

# ============================================================
echo "=== Case 2: byte-identical remainder apart from the work_item_id line ==="
echo ""
REPO=$(setup_repo "0048")
BEFORE=$(grep -v '^work_item_id:' "$REPO/work/item.md")
start_mock "$SCENARIOS/create-201-capture.json"
create "$REPO" work/item.md --quiet >/dev/null 2>&1
stop_mock
AFTER=$(grep -v '^work_item_id:' "$REPO/work/item.md")
assert_eq "remainder byte-identical" "$BEFORE" "$AFTER"
echo ""

# ============================================================
echo "=== Case 3: already-synced (quoted remote id) → exit 102, no API call ==="
echo ""
REPO=$(setup_repo '"BLA-999"')
EXIT_CODE=0
STDERR=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" work/item.md 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_eq "already-synced exits 102" "102" "$EXIT_CODE"
assert_contains "E_CREATE_ALREADY_SYNCED in stderr" "$STDERR" "E_CREATE_ALREADY_SYNCED"
assert_eq "quoted work_item_id unchanged" 'work_item_id: "BLA-999"' \
  "$(grep '^work_item_id:' "$REPO/work/item.md")"
echo ""

# ============================================================
echo "=== Case 4: malformed returned identifier → 106, file untouched ==="
echo ""
REPO=$(setup_repo "0048")
BEFORE=$(cat "$REPO/work/item.md")
start_mock "$SCENARIOS/create-malformed-identifier-201.json"
EXIT_CODE=0
STDERR=$(create "$REPO" work/item.md --quiet 2>&1 1>/dev/null) || EXIT_CODE=$?
stop_mock
assert_eq "malformed identifier exits 106" "106" "$EXIT_CODE"
assert_contains "E_CREATE_BAD_IDENTIFIER in stderr" "$STDERR" "E_CREATE_BAD_IDENTIFIER"
assert_eq "file left untouched" "$BEFORE" "$(cat "$REPO/work/item.md")"
echo ""

# ============================================================
echo "=== Case 5: writeback fails after successful create → 107, loud message ==="
echo ""
# A file with TWO work_item_id lines: the already-synced guard reads the first
# (numeric → proceed), the create succeeds remotely, then the writeback fails
# closed because config_set_frontmatter_field rejects a field matched twice.
# Deterministic and independent of filesystem permissions / euid.
REPO=$(setup_repo "0048")
cat >"$REPO/work/item.md" <<'EOF'
---
id: "0048"
title: Implement the widget
work_item_id: 0048
work_item_id: 0048
status: ready
---

Body.
EOF
start_mock "$SCENARIOS/create-201-capture.json"
EXIT_CODE=0
STDERR=$(create "$REPO" work/item.md --quiet 2>&1 1>/dev/null) || EXIT_CODE=$?
stop_mock
assert_eq "writeback-fail exits 107" "107" "$EXIT_CODE"
assert_contains "E_CREATE_WRITEBACK_FAILED in stderr" "$STDERR" "E_CREATE_WRITEBACK_FAILED"
assert_contains "message names the created identifier" "$STDERR" "BLA-123"
assert_contains "message warns against re-run" "$STDERR" "duplicate"
echo ""

# ============================================================
echo "=== Case 6: --print-payload makes no API call, no writeback ==="
echo ""
REPO=$(setup_repo "0048")
OUT=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" work/item.md --print-payload 2>/dev/null)
assert_eq "print-payload operation" "issueCreate" "$(printf '%s' "$OUT" | jq -r '.operation')"
assert_eq "print-payload did not write back" "work_item_id: 0048" \
  "$(grep '^work_item_id:' "$REPO/work/item.md")"
echo ""

# ============================================================
echo "=== Case 7: missing file → 100 ==="
echo ""
EXIT_CODE=0
(cd "$TMPDIR_BASE" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  bash "$SCRIPT" /no/such/file.md >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "missing file exits 100" "100" "$EXIT_CODE"
echo ""

test_summary
