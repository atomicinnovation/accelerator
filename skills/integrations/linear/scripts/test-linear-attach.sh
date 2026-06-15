#!/usr/bin/env bash
set -euo pipefail

# Tests for linear-attach-flow.sh (link + binary upload)
# Run: bash skills/integrations/linear/scripts/test-linear-attach.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

SCRIPT="$SCRIPT_DIR/linear-attach-flow.sh"
SCENARIOS="$SCRIPT_DIR/test-fixtures/scenarios"
MOCK_SERVER="$SCRIPT_DIR/test-helpers/mock-linear-server.py"

TEST_TOKEN="lin_api_test123"

TMPDIR_BASE=$(mktemp -d)
trap 'stop_mock; rm -rf "$TMPDIR_BASE"' EXIT

REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/.git" "$REPO/.accelerator"
ATTACH_FILE=$(mktemp "$TMPDIR_BASE/asset-XXXXXX.txt")
printf 'binary asset contents\n' >"$ATTACH_FILE"

# --- sleep seam (file-based) ---
SLEEP_COUNT_FILE=$(mktemp "$TMPDIR_BASE/sleepcount-XXXXXX")
export SLEEP_COUNT_FILE
test_record_sleep() {
  local n
  n=$(cat "$SLEEP_COUNT_FILE" 2>/dev/null || echo 0)
  echo $((n + 1)) >"$SLEEP_COUNT_FILE"
}
export -f test_record_sleep
reset_sleep() { echo 0 >"$SLEEP_COUNT_FILE"; }
get_sleeps() { cat "$SLEEP_COUNT_FILE" 2>/dev/null || echo 0; }

MOCK_PID=""
MOCK_URL_FILE=""
MOCK_URL=""
MOCK_BODIES_FILE=""
MOCK_HEADERS_FILE=""
MOCK_ERRORS_FILE=""

# Start the mock on an ephemeral port (no PUT to the same mock needed).
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

free_port() {
  python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'
}

# Start the mock on a FIXED port from a templated scenario so the binary PUT
# (whose uploadUrl the fixture embeds) reaches the same mock.
start_mock_tmpl() {
  local tmpl="$1"
  local port
  port=$(free_port)
  MOCK_URL="http://127.0.0.1:$port"
  local scen
  scen=$(mktemp "$TMPDIR_BASE/scenXXXXXX")
  sed "s|__MOCK_URL__|$MOCK_URL|g" "$tmpl" >"$scen"
  MOCK_URL_FILE=$(mktemp "$TMPDIR_BASE/url-XXXXXX")
  MOCK_HEADERS_FILE=$(mktemp "$TMPDIR_BASE/hdrs-XXXXXX")
  MOCK_ERRORS_FILE=$(mktemp "$TMPDIR_BASE/errs-XXXXXX")
  python3 "$MOCK_SERVER" --scenario "$scen" --url-file "$MOCK_URL_FILE" --port "$port" \
    --captured-headers-file "$MOCK_HEADERS_FILE" \
    --captured-errors-file "$MOCK_ERRORS_FILE" &
  MOCK_PID=$!
  local i=0
  while [ ! -s "$MOCK_URL_FILE" ] && [ $i -lt 50 ]; do
    sleep 0.1
    i=$((i + 1))
  done
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

attach() {
  cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
    ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" \
    bash "$SCRIPT" "$@"
}

# Captured PUT headers (the first captured request) as a lowercase-name set.
put_header_names_lc() {
  jq -r '(.[0] // {}) | keys[] | ascii_downcase' "$MOCK_HEADERS_FILE" 2>/dev/null || true
}
put_header_value() {
  jq -r --arg k "$1" '(.[0] // {}) | to_entries[] | select(.key|ascii_downcase == ($k|ascii_downcase)) | .value' \
    "$MOCK_HEADERS_FILE" 2>/dev/null | head -1
}

# ============================================================
echo "=== Case 1: link attach calls attachmentCreate with the supplied URL ==="
echo ""
start_mock "$SCENARIOS/attach-link-200.json"
attach BLA-1 --url "https://example.com/doc" --title "A doc" --quiet >/dev/null 2>&1
stop_mock
BODY=$(jq -r '.[0]' "$MOCK_BODIES_FILE")
assert_eq "link url sent" "https://example.com/doc" \
  "$(printf '%s' "$BODY" | jq -r '.variables.input.url')"
assert_eq "link issueId == identifier" "BLA-1" \
  "$(printf '%s' "$BODY" | jq -r '.variables.input.issueId')"
echo ""

# ============================================================
echo "=== Case 2: binary attach — fileUpload → PUT → attachmentCreate, header allow-list ==="
echo ""
start_mock_tmpl "$SCENARIOS/attach-binary-success.json.tmpl"
EXIT_CODE=0
attach BLA-2 --file "$ATTACH_FILE" --quiet >/dev/null 2>&1 || EXIT_CODE=$?
stop_mock
assert_eq "binary attach succeeds" "0" "$EXIT_CODE"
NAMES=$(put_header_names_lc)
assert_contains "PUT carried Content-Type" "$NAMES" "content-type"
assert_contains "PUT carried Cache-Control" "$NAMES" "cache-control"
assert_contains "PUT carried allow-listed x-amz-acl" "$NAMES" "x-amz-acl"
assert_not_contains "PUT carried NO Authorization" "$NAMES" "authorization"
assert_not_contains "PUT dropped non-allow-listed header" "$NAMES" "x-not-allowed"
assert_eq "mock recorded no errors" "[]" "$(cat "$MOCK_ERRORS_FILE")"
echo ""

# ============================================================
echo "=== Case 3: off-host / look-alike uploadUrl → E_ATTACH_BAD_UPLOAD_URL, no PUT ==="
echo ""
start_mock "$SCENARIOS/attach-binary-bad-upload-url.json"
EXIT_CODE=0
STDERR=$(attach BLA-3 --file "$ATTACH_FILE" --quiet 2>&1 1>/dev/null) || EXIT_CODE=$?
stop_mock
assert_eq "look-alike host exits 135" "135" "$EXIT_CODE"
assert_contains "E_ATTACH_BAD_UPLOAD_URL in stderr" "$STDERR" "E_ATTACH_BAD_UPLOAD_URL"
echo ""

# ============================================================
echo "=== Case 4: PUT 30x to non-allow-listed host is not followed → upload-failed ==="
echo ""
start_mock_tmpl "$SCENARIOS/attach-binary-redirect.json.tmpl"
reset_sleep
EXIT_CODE=0
STDERR=$(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" LINEAR_RETRY_SLEEP_FN=test_record_sleep \
  bash "$SCRIPT" BLA-4 --file "$ATTACH_FILE" --quiet 2>&1 1>/dev/null) || EXIT_CODE=$?
stop_mock
assert_eq "redirect not followed → 136" "136" "$EXIT_CODE"
assert_contains "E_ATTACH_UPLOAD_FAILED in stderr" "$STDERR" "E_ATTACH_UPLOAD_FAILED"
echo ""

# ============================================================
echo "=== Case 5: failing PUT (5xx) retries (bounded) then E_ATTACH_UPLOAD_FAILED ==="
echo ""
start_mock_tmpl "$SCENARIOS/attach-binary-upload-fail.json.tmpl"
reset_sleep
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST="$MOCK_URL" LINEAR_RETRY_SLEEP_FN=test_record_sleep \
  bash "$SCRIPT" BLA-5 --file "$ATTACH_FILE" --quiet >/dev/null 2>&1) || EXIT_CODE=$?
stop_mock
assert_eq "upload-fail exits 136" "136" "$EXIT_CODE"
assert_eq "bounded retry: 2 sleeps for 3 attempts" "2" "$(get_sleeps)"
echo ""

# ============================================================
echo "=== Case 6: CRLF-valued header is dropped from the PUT (value-level guard) ==="
echo ""
start_mock_tmpl "$SCENARIOS/attach-binary-crlf-header.json.tmpl"
EXIT_CODE=0
attach BLA-6 --file "$ATTACH_FILE" --quiet >/dev/null 2>&1 || EXIT_CODE=$?
stop_mock
assert_eq "crlf-header attach succeeds" "0" "$EXIT_CODE"
NAMES=$(put_header_names_lc)
assert_contains "clean allow-listed header forwarded" "$NAMES" "x-amz-good"
assert_not_contains "CRLF-valued header dropped" "$NAMES" "x-amz-bad"
echo ""

# ============================================================
echo "=== Case 7: PUT ok then attachmentCreate fails → E_ATTACH_REGISTER_FAILED ==="
echo ""
start_mock_tmpl "$SCENARIOS/attach-binary-register-fail.json.tmpl"
EXIT_CODE=0
STDERR=$(attach BLA-7 --file "$ATTACH_FILE" --quiet 2>&1 1>/dev/null) || EXIT_CODE=$?
stop_mock
assert_eq "register-fail exits 137" "137" "$EXIT_CODE"
assert_contains "E_ATTACH_REGISTER_FAILED in stderr" "$STDERR" "E_ATTACH_REGISTER_FAILED"
assert_contains "message names the orphaned asset" "$STDERR" "orphaned"
assert_not_contains "signed query string redacted" "$STDERR" "SECRET-TOKEN"
echo ""

# ============================================================
echo "=== Case 8: target validation — none / both / file-missing / bad url ==="
echo ""
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  bash "$SCRIPT" BLA-8 >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "no target exits 131" "131" "$EXIT_CODE"
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  bash "$SCRIPT" BLA-8 --url https://x.test --file "$ATTACH_FILE" >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "both targets exits 132" "132" "$EXIT_CODE"
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  bash "$SCRIPT" BLA-8 --file /no/such/file >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "missing file exits 133" "133" "$EXIT_CODE"
EXIT_CODE=0
(cd "$REPO" && ACCELERATOR_LINEAR_TOKEN="$TEST_TOKEN" ACCELERATOR_TEST_MODE=1 \
  bash "$SCRIPT" BLA-8 --url "ftp://nope" >/dev/null 2>&1) || EXIT_CODE=$?
assert_eq "bad url exits 134" "134" "$EXIT_CODE"
echo ""

test_summary
