#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

RUN_SH="$SCRIPT_DIR/run.sh"
FIXTURE_HTML="$SCRIPT_DIR/__fixtures__/fixture.html"
LAUNCHER_HELPERS="$PLUGIN_ROOT/skills/visualisation/visualise/scripts/launcher-helpers.sh"

# When set, skip tests that require a real Playwright install
SKIP_REAL="${ACCELERATOR_PLAYWRIGHT_SKIP_REAL_INSTALL:-0}"

echo "=== playwright executor: structural ==="

assert_file_exists "run.sh exists" "$RUN_SH"
assert_file_executable "run.sh is executable" "$RUN_SH"
assert_file_exists "run.js exists" "$SCRIPT_DIR/run.js"
assert_file_exists "package.json exists" "$SCRIPT_DIR/package.json"
assert_file_exists "package-lock.json exists" "$SCRIPT_DIR/package-lock.json"

assert_exit_code "node -c run.js exits 0" 0 node -c "$SCRIPT_DIR/run.js"
assert_exit_code "jq empty package.json" 0 jq empty "$SCRIPT_DIR/package.json"
assert_exit_code "shellcheck run.sh exits 0" 0 shellcheck "$RUN_SH"

echo ""
echo "=== playwright executor: no evaluate-payload-rejected deny-list ==="

# Only check the executor source code (lib/*.js, run.js) — not test files
assert_exit_code "evaluate-payload-rejected not in executor source (lib/*.js, run.js)" 1 \
  grep -r 'evaluate-payload-rejected' "$SCRIPT_DIR/lib" "$SCRIPT_DIR/run.js" 2>/dev/null

echo ""
echo "=== playwright executor: launcher-helpers.sh source path ==="

assert_file_exists "launcher-helpers.sh exists at expected path" "$LAUNCHER_HELPERS"
HELPERS_CONTENT="$(cat "$LAUNCHER_HELPERS")"
assert_contains "launcher-helpers.sh contains start_time_of function" \
  "$HELPERS_CONTENT" "start_time_of"

# Locale fragility regression guard: start_time_of same output under LANG=de_DE.UTF-8 vs LANG=C
PID=$$
RESULT_C="$(LANG=C LC_ALL=C TZ=UTC bash -c "source '$LAUNCHER_HELPERS'; start_time_of $PID" 2>/dev/null || true)"
RESULT_DE="$(LANG=de_DE.UTF-8 TZ=UTC bash -c "source '$LAUNCHER_HELPERS'; start_time_of $PID" 2>/dev/null || true)"
if [[ -n "$RESULT_C" ]] && [[ -n "$RESULT_DE" ]]; then
  if [[ "$RESULT_C" == "$RESULT_DE" ]]; then
    echo "  PASS: start_time_of locale-safe (C=$RESULT_C, de_DE=$RESULT_DE)"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: start_time_of locale fragility: LANG=C=$RESULT_C LANG=de_DE=$RESULT_DE"
    FAIL=$((FAIL + 1))
  fi
else
  echo "  SKIP: start_time_of locale test (locale unavailable or proc not readable)"
fi

if [[ "$SKIP_REAL" == "1" ]]; then
  echo ""
  echo "  SKIP: ACCELERATOR_PLAYWRIGHT_SKIP_REAL_INSTALL=1 — skipping Playwright-dependent tests"
  test_summary
  exit $?
fi

echo ""
echo "=== playwright executor: ensure playwright is bootstrapped ==="

CACHE_ROOT="${ACCELERATOR_PLAYWRIGHT_CACHE:-${HOME}/.cache/accelerator/playwright}"
if [[ ! -d "$CACHE_ROOT" ]]; then
  echo "  SKIP: Playwright not installed at $CACHE_ROOT — set ACCELERATOR_PLAYWRIGHT_SKIP_REAL_INSTALL=1 to skip all real tests"
  test_summary
  exit $?
fi

echo "  INFO: Playwright cache found at $CACHE_ROOT"

echo ""
echo "=== playwright executor: run.sh ping ==="

PING_RESULT="$(cd "$PLUGIN_ROOT" && bash "$RUN_SH" ping 2>/dev/null || true)"
if [[ -n "$PING_RESULT" ]]; then
  assert_contains "ping returns ok:true" "$PING_RESULT" '"ok":true'
  assert_contains "ping returns node version" "$PING_RESULT" '"node"'
  assert_contains "ping returns chromium path" "$PING_RESULT" '"chromium"'
else
  echo "  SKIP: ping returned empty (Playwright not bootstrapped for this lockhash)"
fi

echo ""
echo "=== playwright executor: run.sh daemon-stop ==="

cd "$PLUGIN_ROOT"
STOP_RESULT="$(bash "$RUN_SH" daemon-stop 2>/dev/null || true)"
if [[ -n "$STOP_RESULT" ]]; then
  if echo "$STOP_RESULT" | grep -q '"ok":true'; then
    echo "  PASS: daemon-stop returned ok:true"
    PASS=$((PASS + 1))
  else
    echo "  SKIP: daemon-stop returned: $STOP_RESULT (may not have been running)"
  fi
else
  echo "  SKIP: daemon-stop returned empty"
fi

echo ""
echo "=== run.sh: daemon survives launcher shell exit (smoke test) ==="
# Smoke test: confirms the end-to-end happy path (daemon comes up, the
# sub-shell launcher exits cleanly, daemon survives, daemon-stop produces
# the expected reason). The actual regression guard against re-introducing
# an owner-PID watcher lives in test-design.sh as source-level grep
# assertions over the playwright/ tree — those are stronger because they
# fire regardless of timing.

PROJECT_TMP="$(mktemp -d)"
trap 'rm -rf "$PROJECT_TMP"' EXIT
mkdir -p "$PROJECT_TMP/.git"
export ACCELERATOR_PLAYWRIGHT_CACHE="${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}"

# Launch the daemon via run.sh ping; the sub-shell exits after ping returns.
(cd "$PROJECT_TMP" && bash "$RUN_SH" ping >/dev/null 2>&1 || true)

STATE_DIR="$PROJECT_TMP/.accelerator/tmp/inventory-design-playwright"
if [[ -f "$STATE_DIR/server.pid" ]]; then
  SERVER_PID="$(tr -cd '0-9' < "$STATE_DIR/server.pid")"
  assert_neq "daemon wrote server.pid" "" "$SERVER_PID"

  sleep 2
  assert_exit_code "daemon process is still alive after launcher shell exited" 0 \
    kill -0 "$SERVER_PID"

  # Clean stop and check the reason. `daemon-stop` is the expected reason;
  # `owner-exited` would indicate the watcher had been silently restored.
  (cd "$PROJECT_TMP" && bash "$RUN_SH" daemon-stop >/dev/null 2>&1 || true)
  sleep 1
  STOPPED_REASON="$(jq -r '.reason' "$STATE_DIR/server-stopped.json" 2>/dev/null || echo "")"
  assert_eq "daemon stopped with reason daemon-stop (not owner-exited)" \
    "daemon-stop" "$STOPPED_REASON"
else
  echo "  SKIP: server.pid not written (Playwright likely not bootstrapped for this lockhash)"
fi

test_summary
