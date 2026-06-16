#!/usr/bin/env bash
set -euo pipefail

# Tests for linear-create-flow.sh (external_id writeback + no-file create mode)
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

# Repo with a catalogue (team id) and a work-item file under work/. The optional
# argument is the raw external_id frontmatter value; when omitted, the file
# carries NO external_id line (the unsynced default — proves the writeback
# INSERTS the line rather than replacing one).
setup_repo() {
  local ext_value="${1-}"
  local d
  d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator" "$d/$STATE_REL" "$d/work"
  cat >"$d/$STATE_REL/catalogue.json" <<'CAT'
{"team": {"id": "team-x-uuid", "key": "TX", "name": "Team X"}, "workflowStates": [{"id": "s1", "name": "Todo", "type": "unstarted", "position": 0}]}
CAT
  {
    echo '---'
    echo 'id: "0048"'
    echo 'title: Implement the widget'
    [ -n "$ext_value" ] && echo "external_id: $ext_value"
    echo 'status: ready'
    echo '---'
    echo ''
    echo 'This is the body.'
    echo ''
    echo 'It has two paragraphs.'
  } >"$d/work/item.md"
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
echo "=== Case 1: create from an unsynced file — title/description/external_id insert ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/create-201-capture.json"
OUT=$(create "$REPO" work/item.md --quiet 2>/dev/null)
stop_mock
assert_eq "prints the new identifier" "BLA-123" "$OUT"
assert_eq "external_id inserted as BLA-123" "external_id: BLA-123" \
  "$(grep '^external_id:' "$REPO/work/item.md")"
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
echo "=== Case 2: byte-identical remainder; external_id is INSERTED inside the fence ==="
echo ""
REPO=$(setup_repo)
# The fixture has NO external_id line, so excluding external_id from the
# comparison proves the writeback INSERTED a line (not replaced one).
BEFORE=$(grep -v '^external_id:' "$REPO/work/item.md")
start_mock "$SCENARIOS/create-201-capture.json"
create "$REPO" work/item.md --quiet >/dev/null 2>&1
stop_mock
AFTER=$(grep -v '^external_id:' "$REPO/work/item.md")
assert_eq "remainder byte-identical" "$BEFORE" "$AFTER"
# Remainder-equality alone does not prove the line is inside the frontmatter —
# assert it lands within the frontmatter fence, not the body.
FM=$(bash -c "source '$PLUGIN_ROOT/scripts/config-common.sh' && config_extract_frontmatter '$REPO/work/item.md'")
assert_contains "inserted external_id is inside the frontmatter fence" \
  "$FM" "external_id: BLA-123"
BODY_PART=$(bash -c "source '$PLUGIN_ROOT/scripts/config-common.sh' && config_extract_body '$REPO/work/item.md'")
assert_eq "body has no external_id line" "" \
  "$(printf '%s' "$BODY_PART" | grep '^external_id:' || true)"
echo ""

# ============================================================
echo "=== Case 3: already-synced (non-empty external_id) → exit 102, no API call ==="
echo ""
REPO=$(setup_repo '"BLA-999"')
EXIT_CODE=0
STDERR=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" work/item.md 2>&1 1>/dev/null) || EXIT_CODE=$?
assert_eq "already-synced exits 102" "102" "$EXIT_CODE"
assert_contains "E_CREATE_ALREADY_SYNCED in stderr" "$STDERR" "E_CREATE_ALREADY_SYNCED"
assert_eq "quoted external_id unchanged" 'external_id: "BLA-999"' \
  "$(grep '^external_id:' "$REPO/work/item.md")"
echo ""

# ============================================================
echo "=== Case 3b: quote-only external_id \"\" → NOT synced → proceeds ==="
echo ""
# A quote-only/empty external_id normalises to empty, so the already-synced
# guard must NOT fire. --print-payload proves we reach payload build (no 102).
REPO=$(setup_repo '""')
EXIT_CODE=0
OUT=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" work/item.md --print-payload 2>/dev/null) || EXIT_CODE=$?
assert_eq "quote-only proceeds (exit 0, not 102)" "0" "$EXIT_CODE"
assert_eq "reached payload build" "issueCreate" "$(printf '%s' "$OUT" | jq -r '.operation')"
echo ""

# ============================================================
echo "=== Case 4: malformed returned identifier → 106, file untouched ==="
echo ""
REPO=$(setup_repo)
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
# TWO empty external_id lines: the already-synced guard reads the first (empty →
# proceed), the create succeeds remotely, then the writeback fails closed
# because the upsert helper rejects a duplicate/present key. Deterministic and
# independent of filesystem permissions / euid.
REPO=$(setup_repo)
cat >"$REPO/work/item.md" <<'EOF'
---
id: "0048"
title: Implement the widget
external_id:
external_id:
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
assert_contains "message names external_id" "$STDERR" "external_id"
assert_contains "message names the created identifier" "$STDERR" "BLA-123"
assert_contains "message warns against re-run" "$STDERR" "duplicate"
echo ""

# ============================================================
echo "=== Case 6: --print-payload makes no API call, no writeback ==="
echo ""
REPO=$(setup_repo)
OUT=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="http://127.0.0.1:1" \
  bash "$SCRIPT" work/item.md --print-payload 2>/dev/null)
assert_eq "print-payload operation" "issueCreate" "$(printf '%s' "$OUT" | jq -r '.operation')"
assert_eq "print-payload did not write back (no external_id line)" "" \
  "$(grep '^external_id:' "$REPO/work/item.md" || true)"
echo ""

# ============================================================
echo "=== Case 7: missing file → 100 ==="
echo ""
EXIT_CODE=0
(cd "$TMPDIR_BASE" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  bash "$SCRIPT" /no/such/file.md >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "missing file exits 100" "100" "$EXIT_CODE"
echo ""

# ============================================================
echo "=== Case 8: no-file create-and-return mode — bare identifier, no writeback ==="
echo ""
REPO=$(setup_repo)
BODY_FILE="$REPO/body.md"
printf 'Issue body from a file.\n\nSecond paragraph.\n' >"$BODY_FILE"
BODY_BEFORE=$(cat "$BODY_FILE")
start_mock "$SCENARIOS/create-201-capture.json"
OUT=$(create "$REPO" --title "Widget from flags" --body-file "$BODY_FILE" --quiet 2>/dev/null)
stop_mock
assert_eq "stdout is exactly the bare identifier" "BLA-123" "$OUT"
BODY=$(jq -r '.[0]' "$MOCK_BODIES_FILE")
assert_eq "issueCreate input.title == --title" "Widget from flags" \
  "$(printf '%s' "$BODY" | jq -r '.variables.input.title')"
assert_contains "description carries the body file" \
  "$(printf '%s' "$BODY" | jq -r '.variables.input.description')" "Issue body from a file."
assert_eq "input file (body-file) byte-unchanged" "$BODY_BEFORE" "$(cat "$BODY_FILE")"
# No-file mode must never touch the work-item file.
assert_eq "work-item file gained no external_id" "" \
  "$(grep '^external_id:' "$REPO/work/item.md" || true)"
echo ""

# ============================================================
echo "=== Case 9: no-file pre-create failure (request rejected) → 108 ==="
echo ""
REPO=$(setup_repo)
start_mock "$SCENARIOS/bad-request-400.json"
EXIT_CODE=0
create "$REPO" --title "x" --quiet >/dev/null 2>&1 || EXIT_CODE=$?
stop_mock
assert_eq "pre-send failure exits 108" "108" "$EXIT_CODE"
echo ""

# ============================================================
echo "=== Case 10: no-file post-create (response dropped after send) → 109 ==="
echo ""
# The request reaches the server (issueCreate transmitted) but the response is a
# truncated/non-JSON body — indistinguishable from a created issue whose
# response was lost. Must map to post-create (NOT safe to retry).
REPO=$(setup_repo)
start_mock "$SCENARIOS/create-response-dropped-200.json"
EXIT_CODE=0
create "$REPO" --title "x" --quiet >/dev/null 2>&1 || EXIT_CODE=$?
stop_mock
assert_eq "post-send failure exits 109" "109" "$EXIT_CODE"
echo ""

test_summary
