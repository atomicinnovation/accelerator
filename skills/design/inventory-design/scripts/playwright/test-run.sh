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

# Locale fragility regression guard: start_time_of must return the same
# epoch under LANG=de_DE.UTF-8 as under LANG=C. The previous form treated
# an empty de_DE result as "locale unavailable" and SKIPped — but on macOS
# de_DE.UTF-8 is installed by default, so an empty result means the bug
# is back, not that the locale is missing. We probe locale availability
# explicitly via `locale -a` and only SKIP when it's truly absent.
PID=$$
RESULT_C="$(LANG=C LC_ALL=C TZ=UTC bash -c "source '$LAUNCHER_HELPERS'; start_time_of $PID" 2>/dev/null || true)"
# Capture `locale -a` into a variable rather than piping into `grep -q`:
# under `set -o pipefail`, grep's early-exit can leave locale with SIGPIPE
# (exit 141), which falsely fails the locale-availability probe.
LOCALES_AVAILABLE="$(command -v locale >/dev/null 2>&1 && locale -a 2>/dev/null || true)"
if [[ -z "$RESULT_C" ]]; then
  echo "  SKIP: start_time_of locale test (start_time_of unavailable under C — likely no proc, no ps)"
elif [[ $'\n'"$LOCALES_AVAILABLE"$'\n' != *$'\n'"de_DE.UTF-8"$'\n'* ]]; then
  echo "  SKIP: start_time_of locale test (de_DE.UTF-8 not installed)"
else
  RESULT_DE="$(LANG=de_DE.UTF-8 TZ=UTC bash -c "source '$LAUNCHER_HELPERS'; start_time_of $PID" 2>/dev/null || true)"
  if [[ "$RESULT_C" == "$RESULT_DE" ]]; then
    echo "  PASS: start_time_of locale-safe (C=$RESULT_C, de_DE=$RESULT_DE)"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: start_time_of locale fragility: LANG=C='$RESULT_C' LANG=de_DE='$RESULT_DE'"
    FAIL=$((FAIL + 1))
  fi
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
echo "=== run.sh links command ==="
FIXTURE_PATH="$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright/__fixtures__/links.html"
FIXTURE_URL="file://$FIXTURE_PATH"

LINKS_PROJECT_TMP="$(mktemp -d)"
mkdir -p "$LINKS_PROJECT_TMP/.git"
export ACCELERATOR_PLAYWRIGHT_CACHE="${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}"

(cd "$LINKS_PROJECT_TMP" && bash "$RUN_SH" navigate "{\"url\":\"$FIXTURE_URL\"}" || true)
LINKS_OUT="$(cd "$LINKS_PROJECT_TMP" && bash "$RUN_SH" links 2>/dev/null || true)"

if [[ -n "$LINKS_OUT" ]] && echo "$LINKS_OUT" | grep -q '"links"'; then
  # Envelope: includes the current page URL so callers can verify context.
  assert_contains "links output names the current page URL" "$LINKS_OUT" '"url":"file://'
  assert_contains "links output is JSON with links field" "$LINKS_OUT" '"links"'

  # Same-origin relative paths are resolved into pathnames.
  assert_contains "links output includes /work-items pathname" "$LINKS_OUT" '"pathname":"/work-items"'
  assert_contains "links output includes /library/work-items pathname" "$LINKS_OUT" '"/library/work-items"'

  # Whitespace normalised in text.
  assert_contains "links output collapses internal whitespace in text" "$LINKS_OUT" '"text":"Library Items"'

  # Role preserved verbatim (null when unset).
  assert_contains "links output preserves explicit role" "$LINKS_OUT" '"role":"button"'
  assert_contains "links output uses null role for anchors without role attribute" "$LINKS_OUT" '"role":null'

  # Same-origin flag (opaque-origin guard ensures file:// page reports same_origin: false).
  assert_contains "links output marks cross-origin anchors as not same-origin" \
    "$LINKS_OUT" '"same_origin":false'
  assert_contains "links output marks mailto: as cross-origin (opaque-origin guard)" \
    "$LINKS_OUT" '"scheme":"mailto"'
  assert_not_contains "no anchor reports same_origin: true on a file:// page (opaque-origin guard)" \
    "$LINKS_OUT" '"same_origin":true'

  # Scheme.
  assert_contains "links output includes file scheme for relative same-origin" "$LINKS_OUT" '"scheme":"file"'
  assert_contains "links output includes https scheme for absolute cross-origin" "$LINKS_OUT" '"scheme":"https"'
  assert_contains "links output includes mailto scheme" "$LINKS_OUT" '"scheme":"mailto"'

  # Response MUST NOT include raw href or fully-resolved URL.
  assert_not_contains "links response does not include raw 'href' field" "$LINKS_OUT" '"href"'
  assert_not_contains "links response does not include fully-resolved 'resolved' field" "$LINKS_OUT" '"resolved"'
  assert_not_contains "links response does not echo query string from ?q=foo anchor" "$LINKS_OUT" 'q=foo'
  assert_not_contains "links response does not echo fragment from #top anchor" "$LINKS_OUT" '#top'

  # Pre-navigate (about:blank) case: links returns an empty list with the
  # blank URL envelope, not an error. Capture the PID before stopping so we
  # can wait for the daemon process to fully exit (it holds the
  # launcher.lock FD across its shutdown, so the next launcher invocation
  # is blocked until then).
  LINKS_DAEMON_PID="$(tr -cd '0-9' < "$LINKS_PROJECT_TMP/.accelerator/tmp/inventory-design-playwright/server.pid" 2>/dev/null || echo "")"
  (cd "$LINKS_PROJECT_TMP" && bash "$RUN_SH" daemon-stop >/dev/null 2>&1 || true)
  if [[ -n "$LINKS_DAEMON_PID" ]]; then
    for _ in $(seq 1 50); do
      kill -0 "$LINKS_DAEMON_PID" 2>/dev/null || break
      sleep 0.2
    done
  fi
  # Clear the launcher-lock dir left over by the mkdir-fallback path (the
  # trap that would normally clean it is dropped by `exec node …`). This
  # is a pre-existing run.sh quirk surfaced when re-spawning a daemon in
  # the same state-dir.
  rmdir "$LINKS_PROJECT_TMP/.accelerator/tmp/inventory-design-playwright/launcher.lock.d" 2>/dev/null || true
  (cd "$LINKS_PROJECT_TMP" && bash "$RUN_SH" navigate '{"url":"about:blank"}' >/dev/null 2>&1 || true)
  BLANK_OUT="$(cd "$LINKS_PROJECT_TMP" && bash "$RUN_SH" links 2>/dev/null || true)"
  assert_contains "links on about:blank names the URL" "$BLANK_OUT" '"url":"about:blank"'
  assert_contains "links on about:blank returns empty array" "$BLANK_OUT" '"links":[]'

  (cd "$LINKS_PROJECT_TMP" && bash "$RUN_SH" daemon-stop >/dev/null 2>&1 || true)
else
  echo "  SKIP: links command output empty (Playwright not bootstrapped for this lockhash)"
fi
rm -rf "$LINKS_PROJECT_TMP"

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
