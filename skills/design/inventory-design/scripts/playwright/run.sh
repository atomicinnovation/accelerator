#!/usr/bin/env bash
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../../.." && pwd)"

# shellcheck disable=SC1091
source "$PLUGIN_ROOT/scripts/vcs-common.sh"
# shellcheck disable=SC1091
source "$PLUGIN_ROOT/skills/visualisation/visualise/scripts/launcher-helpers.sh"

# -- Project state dir resolution ----------------------------------------

PROJECT_ROOT="$(find_repo_root 2>/dev/null || true)"
if [[ -z "${PROJECT_ROOT:-}" ]]; then
  echo '{"error":"no-repo","message":"inventory-design must be run inside a git or jj repository (no enclosing repo found)","category":"usage"}' >&2
  exit 2
fi

TMP_REL="$("$PLUGIN_ROOT/scripts/config-read-path.sh" tmp)"
STATE_DIR="$PROJECT_ROOT/$TMP_REL/inventory-design-playwright"
mkdir -p "$STATE_DIR"
chmod 0700 "$STATE_DIR" 2>/dev/null || true

# -- Playwright namespace resolution -------------------------------------

CACHE_ROOT="${ACCELERATOR_PLAYWRIGHT_CACHE:-${HOME}/.cache/accelerator/playwright}"
PKG_LOCK="$SCRIPT_DIR/package-lock.json"
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -c1-8
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
  EXISTING_PID="$(tr -cd '0-9' < "$PID_FILE" 2>/dev/null || true)"
  EXPECTED_START="$(jq -r '.start_time // empty' "$INFO" 2>/dev/null || true)"
  if [[ -n "$EXISTING_PID" ]] && kill -0 "$EXISTING_PID" 2>/dev/null; then
    ACTUAL_START="$(start_time_of "$EXISTING_PID" 2>/dev/null || true)"
    if [[ -z "$EXPECTED_START" ]] || [[ "$ACTUAL_START" == "$EXPECTED_START" ]]; then
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
  EXISTING_PID="$(tr -cd '0-9' < "$PID_FILE" 2>/dev/null || true)"
  EXPECTED_START="$(jq -r '.start_time // empty' "$INFO" 2>/dev/null || true)"
  if [[ -n "$EXISTING_PID" ]] && kill -0 "$EXISTING_PID" 2>/dev/null; then
    ACTUAL_START="$(start_time_of "$EXISTING_PID" 2>/dev/null || true)"
    if [[ -z "$EXPECTED_START" ]] || [[ "$ACTUAL_START" == "$EXPECTED_START" ]]; then
      export ACCELERATOR_PLAYWRIGHT_STATE_DIR="$STATE_DIR"
      export NODE_PATH="$NS_ROOT/node_modules"
      export ACCELERATOR_PLAYWRIGHT_NS_ROOT="$NS_ROOT"
      exec node "$SCRIPT_DIR/run.js" "$@"
    fi
  fi
  rm -f "$INFO" "$PID_FILE"
fi

rm -f "$STOPPED"

# -- Spawn daemon --------------------------------------------------------

BOOTSTRAP_LOG="$STATE_DIR/server.bootstrap.log"
: > "$BOOTSTRAP_LOG"
chmod 0600 "$BOOTSTRAP_LOG"

export NODE_PATH="$NS_ROOT/node_modules"
export ACCELERATOR_PLAYWRIGHT_NS_ROOT="$NS_ROOT"
nohup node "$SCRIPT_DIR/run.js" daemon \
  --state-dir "$STATE_DIR" \
  --owner-pid "$$" \
  >> "$BOOTSTRAP_LOG" 2>&1 &
DAEMON_PID=$!
disown "$DAEMON_PID" 2>/dev/null || true

# -- Poll for server-info.json ------------------------------------------

for _ in $(seq 1 50); do
  [[ -f "$INFO" ]] && [[ -f "$PID_FILE" ]] && break
  sleep 0.1
done

if [[ ! -f "$INFO" ]]; then
  echo "{\"error\":\"daemon-start-timeout\",\"message\":\"Daemon did not start within 5s. Check $BOOTSTRAP_LOG for details.\",\"category\":\"bootstrap\"}" >&2
  exit 1
fi

# -- Release lock and run command ----------------------------------------

export ACCELERATOR_PLAYWRIGHT_STATE_DIR="$STATE_DIR"
export ACCELERATOR_PLAYWRIGHT_NS_ROOT="$NS_ROOT"
exec node "$SCRIPT_DIR/run.js" "$@"
