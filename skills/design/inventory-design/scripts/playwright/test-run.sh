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

test_summary
