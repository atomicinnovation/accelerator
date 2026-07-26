#!/usr/bin/env bash
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../../.." && pwd)"

# -- PID-liveness helpers -------------------------------------------------
# Inlined from the visualiser skill's former launcher-helpers.sh, whose sole
# surviving consumer was this executor. start_time_of resolves a process's
# start time (the identity half of a PID-recycle guard); start_time_matches
# compares it against the daemon-recorded value.
start_time_of() {
  local pid="$1"
  if [ -r "/proc/$pid/stat" ] && [ -r "/proc/stat" ]; then
    local tail
    tail="$(sed -E 's/.*\) //' "/proc/$pid/stat")"
    local starttime_ticks
    starttime_ticks="$(echo "$tail" | awk '{print $20}')"
    local hz
    hz="$(getconf CLK_TCK 2>/dev/null || echo 0)"
    [ "$hz" -gt 0 ] || return 1
    local btime
    btime="$(awk '/^btime / {print $2}' /proc/stat)"
    echo $((btime + starttime_ticks / hz))
  elif command -v ps >/dev/null 2>&1 && [ "$(uname -s)" = "Darwin" ]; then
    # Force C locale on both sides: `ps lstart` and `date -j -f` both
    # localise day/month names (and on de_DE even the field order),
    # which makes the fixed `%a %b %d %H:%M:%S %Y` pattern unparseable
    # under non-English locales. The daemon writes its start-time under
    # LANG=C (see lib/state.js processStartSeconds); without matching
    # here, every reuse check fails and the launcher respawns the
    # daemon between commands — losing page state (e.g. a prior
    # `navigate`) on the way through.
    local out
    out="$(LANG=C LC_ALL=C ps -p "$pid" -o lstart= 2>/dev/null | tr -s ' ' ' ' | sed 's/^ //;s/ $//')"
    [ -n "$out" ] || return 1
    LANG=C LC_ALL=C date -j -f "%a %b %d %H:%M:%S %Y" "$out" +%s 2>/dev/null
  else
    return 1
  fi
}

# Compare an expected start-time (recorded by the daemon) against the
# observed start-time (from start_time_of). Tolerates a 1-second drift
# because the daemon captures `Math.floor(Date.now()/1000)` after a few
# milliseconds of module-loading, which can cross a whole-second boundary
# relative to the kernel fork time that ps lstart / /proc reports. A
# 1-second drift cannot be a PID recycle, so the looser check still
# detects stale PIDs.
# Args: expected actual. Returns 0 if expected is empty or within ±1s.
start_time_matches() {
  local expected="$1" actual="$2" diff
  [ -z "$expected" ] && return 0
  [ -z "$actual" ] && return 1
  diff=$((actual - expected))
  [ "$diff" -lt 0 ] && diff=$((-diff))
  [ "$diff" -le 1 ]
}

# test-run.sh sources this file to exercise start_time_of under different
# locales in isolation; when sourced rather than executed, stop here — only
# the helpers above are needed, not the launcher body below.
if (return 0 2>/dev/null); then return 0; fi

# shellcheck disable=SC1091
source "$PLUGIN_ROOT/scripts/vcs-common.sh"

# -- Project state dir resolution ----------------------------------------

PROJECT_ROOT="$(find_repo_root 2>/dev/null || true)"
if [[ -z "${PROJECT_ROOT:-}" ]]; then
  echo '{"error":"no-repo","message":"inventory-design must be run inside a git or jj repository (no enclosing repo found)","category":"usage"}' >&2
  exit 2
fi

TMP_REL="$("${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}" config path tmp)"
STATE_DIR="$PROJECT_ROOT/$TMP_REL/inventory-design-playwright"
mkdir -p "$STATE_DIR"
chmod 0700 "$STATE_DIR" 2>/dev/null || true

# -- Playwright namespace resolution -------------------------------------

CACHE_ROOT="${ACCELERATOR_PLAYWRIGHT_CACHE:-${HOME}/.cache/accelerator/playwright}"
PKG_LOCK="$SCRIPT_DIR/package-lock.json"
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -c1-8
  else shasum -a 256 "$1" | cut -c1-8; fi
}
LOCKHASH="$(sha256_of "$PKG_LOCK")"
NS_ROOT="$CACHE_ROOT/$LOCKHASH"

if [[ ! -f "$NS_ROOT/node_modules/playwright/package.json" ]]; then
  echo "{\"error\":\"playwright-not-installed\",\"message\":\"Playwright not installed at $NS_ROOT — run ensure-playwright.sh first\",\"category\":\"bootstrap\"}" >&2
  exit 3
fi

# -- Reuse short-circuit (before locking) --------------------------------

INFO="$STATE_DIR/server-info.json"
PID_FILE="$STATE_DIR/server.pid"
LOCK="$STATE_DIR/launcher.lock"
STOPPED="$STATE_DIR/server-stopped.json"

if [[ -f "$INFO" ]] && [[ -f "$PID_FILE" ]]; then
  EXISTING_PID="$(tr -cd '0-9' <"$PID_FILE" 2>/dev/null || true)"
  EXPECTED_START="$(jq -r '.start_time // empty' "$INFO" 2>/dev/null || true)"
  if [[ -n "$EXISTING_PID" ]] && kill -0 "$EXISTING_PID" 2>/dev/null; then
    ACTUAL_START="$(start_time_of "$EXISTING_PID" 2>/dev/null || true)"
    if start_time_matches "$EXPECTED_START" "$ACTUAL_START"; then
      # Daemon is alive — run the command directly
      export ACCELERATOR_PLAYWRIGHT_STATE_DIR="$STATE_DIR"
      export NODE_PATH="$NS_ROOT/node_modules"
      export ACCELERATOR_PLAYWRIGHT_NS_ROOT="$NS_ROOT"
      exec node "$SCRIPT_DIR/run.js" "$@"
    fi
  fi
  # Stale files — fall through to recovery
  rm -f "$INFO" "$PID_FILE"
fi

# -- Lock acquisition ----------------------------------------------------

if command -v flock >/dev/null 2>&1 && [[ "${ACCELERATOR_LOCK_FORCE_MKDIR:-0}" != "1" ]]; then
  exec 9>"$LOCK"
  if ! flock -n 9; then
    echo '{"error":"another-launcher-running","message":"Another inventory-design launcher is running. Wait for it to finish.","category":"usage"}' >&2
    exit 1
  fi
else
  if ! mkdir "${LOCK}.d" 2>/dev/null; then
    echo '{"error":"another-launcher-running","message":"Another inventory-design launcher is running. Wait for it to finish.","category":"usage"}' >&2
    exit 1
  fi
  trap 'rmdir "${LOCK}.d" 2>/dev/null || true' EXIT
fi

# Under lock: re-check (another process may have just spawned the daemon)
if [[ -f "$INFO" ]] && [[ -f "$PID_FILE" ]]; then
  EXISTING_PID="$(tr -cd '0-9' <"$PID_FILE" 2>/dev/null || true)"
  EXPECTED_START="$(jq -r '.start_time // empty' "$INFO" 2>/dev/null || true)"
  if [[ -n "$EXISTING_PID" ]] && kill -0 "$EXISTING_PID" 2>/dev/null; then
    ACTUAL_START="$(start_time_of "$EXISTING_PID" 2>/dev/null || true)"
    if start_time_matches "$EXPECTED_START" "$ACTUAL_START"; then
      export ACCELERATOR_PLAYWRIGHT_STATE_DIR="$STATE_DIR"
      export NODE_PATH="$NS_ROOT/node_modules"
      export ACCELERATOR_PLAYWRIGHT_NS_ROOT="$NS_ROOT"
      # The EXIT trap is dropped by exec, so release the mkdir-fallback
      # lock dir explicitly. With flock the FD is closed automatically on
      # process exit, so this is a no-op there.
      rmdir "${LOCK}.d" 2>/dev/null || true
      exec node "$SCRIPT_DIR/run.js" "$@"
    fi
  fi
  rm -f "$INFO" "$PID_FILE"
fi

rm -f "$STOPPED"

# -- Spawn daemon --------------------------------------------------------

BOOTSTRAP_LOG="$STATE_DIR/server.bootstrap.log"
: >"$BOOTSTRAP_LOG"
chmod 0600 "$BOOTSTRAP_LOG"

export NODE_PATH="$NS_ROOT/node_modules"
export ACCELERATOR_PLAYWRIGHT_NS_ROOT="$NS_ROOT"
nohup node "$SCRIPT_DIR/run.js" daemon \
  --state-dir "$STATE_DIR" \
  >>"$BOOTSTRAP_LOG" 2>&1 &
DAEMON_PID=$!
disown "$DAEMON_PID" 2>/dev/null || true

# -- Poll for server-info.json ------------------------------------------

# 30-second wait covers parallel-test load: node startup + a few module
# imports normally take well under a second, but when many integration
# suites run concurrently (mise's parallel test task) the daemon's
# server.listen callback can slip well past 5s.
for _ in $(seq 1 300); do
  [[ -f "$INFO" ]] && [[ -f "$PID_FILE" ]] && break
  sleep 0.1
done

if [[ ! -f "$INFO" ]]; then
  # Kill the still-bootstrapping daemon. Leaving it would mean a later
  # launcher reuses it (info file appears eventually) but on a page that
  # never received this launcher's command, surfacing as e.g. about:blank
  # for a follow-up `links` call.
  kill -TERM "$DAEMON_PID" 2>/dev/null || true
  echo "{\"error\":\"daemon-start-timeout\",\"message\":\"Daemon did not start within 30s. Check $BOOTSTRAP_LOG for details.\",\"category\":\"bootstrap\"}" >&2
  exit 1
fi

# -- Release lock and run command ----------------------------------------

export ACCELERATOR_PLAYWRIGHT_STATE_DIR="$STATE_DIR"
export ACCELERATOR_PLAYWRIGHT_NS_ROOT="$NS_ROOT"
# The EXIT trap is dropped by exec, so release the mkdir-fallback lock dir
# explicitly. With flock the FD is closed automatically on process exit.
rmdir "${LOCK}.d" 2>/dev/null || true
exec node "$SCRIPT_DIR/run.js" "$@"
