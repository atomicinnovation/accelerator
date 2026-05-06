#!/usr/bin/env bash

if [[ -z "${BASH_VERSION:-}" ]]; then
  echo "ensure-playwright.sh requires bash" >&2
  exit 2
fi

set -euo pipefail

# First-run bootstrap for the Playwright executor used by inventory-design.
#
# Installs Playwright + Chromium into a machine-wide cache at
#   ${HOME}/.cache/accelerator/playwright/<sha8>/
# where <sha8> is sha256(package-lock.json)[:8] of the skill-shipped lockfile.
#
# Idempotent: fast exit when the cache is already populated (sentinel present
# and playwright resolvable). macOS and Linux only.
#
# Mock env vars for testing (each independently controllable):
#   ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK=1       no-op the npm ci step
#   ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_OK=1 no-op the playwright install step
#   ACCELERATOR_PLAYWRIGHT_MOCK_NPM_EXIT=N     simulate npm ci failure with exit N
#   ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_EXIT=N simulate playwright install failure with exit N
#
# When both _OK flags are set: fast path that just touches expected files and
# writes a sentinel (equivalent to a complete successful install without network).
#
# Opt-in stale-namespace sweep: set ACCELERATOR_PLAYWRIGHT_SWEEP=1.
# Default is NO sweep (silent deletion of multi-hundred-MB caches is a UX hazard).

# -- Platform guard -----------------------------------------------------------

case "${OSTYPE:-unknown}" in
  darwin*|linux*) ;;
  *)
    echo "error: ensure-playwright.sh supports macOS and Linux only (OSTYPE=${OSTYPE:-unknown})" >&2
    exit 2
    ;;
esac

# -- Configuration ------------------------------------------------------------

CACHE_ROOT="${ACCELERATOR_PLAYWRIGHT_CACHE:-${HOME}/.cache/accelerator/playwright}"
NODE_FLOOR_MAJOR=20
DISK_FLOOR_MB=500

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_JSON="$SCRIPT_DIR/playwright/package.json"
PKG_LOCK="$SCRIPT_DIR/playwright/package-lock.json"
# Cross-platform sha256 (Linux: sha256sum, macOS: shasum -a 256)
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -c1-8
  else
    shasum -a 256 "$1" | cut -c1-8
  fi
}

LOCKHASH="$(sha256_of "$PKG_LOCK")"
NS_ROOT="$CACHE_ROOT/$LOCKHASH"
SENTINEL="$NS_ROOT/.bootstrap-sentinel"
TOP_SENTINEL="$CACHE_ROOT/.bootstrap-sentinel"
LOCKFILE="$CACHE_ROOT/bootstrap.lock"

# -- Cleanup trap -------------------------------------------------------------

# shellcheck disable=SC2329
cleanup() {
  local exit_code=$?
  if [[ $exit_code -ne 0 ]]; then
    rm -f "${SENTINEL}.tmp" "${TOP_SENTINEL}.tmp"
  fi
  # Release mkdir-based lock if we hold it
  if [[ -n "${LOCK_DIR_HELD:-}" ]]; then
    rmdir "${LOCKFILE}.d" 2>/dev/null || true
    LOCK_DIR_HELD=""
  fi
}
trap cleanup EXIT INT TERM

# -- Lock management ----------------------------------------------------------

LOCK_FD=9
LOCK_DIR_HELD=""

acquire_lock() {
  local max_wait=300  # 5 minutes
  local waited=0

  if [[ "${ACCELERATOR_LOCK_FORCE_MKDIR:-0}" != "1" ]] && command -v flock >/dev/null 2>&1; then
    # flock path: non-blocking attempt first, then wait loop
    mkdir -p "$(dirname "$LOCKFILE")"
    eval "exec $LOCK_FD>>\"$LOCKFILE\""
    while ! flock -n "$LOCK_FD" 2>/dev/null; do
      if (( waited >= max_wait )); then
        echo "error: ensure-playwright.sh: timed out waiting for bootstrap lock ($max_wait s)" >&2
        exit 1
      fi
      sleep 1
      (( waited++ ))
    done
  else
    # mkdir-based fallback (atomic on POSIX, works without flock)
    mkdir -p "$CACHE_ROOT"
    while ! mkdir "${LOCKFILE}.d" 2>/dev/null; do
      if (( waited >= max_wait )); then
        echo "error: ensure-playwright.sh: timed out waiting for bootstrap lock ($max_wait s)" >&2
        exit 1
      fi
      sleep 1
      (( waited++ ))
    done
    LOCK_DIR_HELD=1
  fi
}

release_lock() {
  if [[ -n "${LOCK_DIR_HELD:-}" ]]; then
    rmdir "${LOCKFILE}.d" 2>/dev/null || true
    LOCK_DIR_HELD=""
  fi
  # flock auto-releases on fd close / process exit
}

# -- Node version check -------------------------------------------------------

check_node() {
  if ! command -v node >/dev/null 2>&1; then
    echo "error: Node >= $NODE_FLOOR_MAJOR is required to use inventory-design --crawler runtime|hybrid." >&2
    echo "       Install from https://nodejs.org/ (OSTYPE=${OSTYPE:-unknown})" >&2
    exit 1
  fi
  local node_major
  node_major="$(node -e 'process.stdout.write(String(process.versions.node.split(".")[0]))')"
  if (( node_major < NODE_FLOOR_MAJOR )); then
    echo "error: Node >= $NODE_FLOOR_MAJOR required; detected Node $node_major.x" >&2
    echo "       Install from https://nodejs.org/ (OSTYPE=${OSTYPE:-unknown})" >&2
    exit 1
  fi
}

# -- Disk space check ---------------------------------------------------------

check_disk() {
  local target_dir="${1:-$CACHE_ROOT}"
  mkdir -p "$target_dir"
  local avail_kb
  avail_kb="$(df -k "$target_dir" 2>/dev/null | awk 'NR==2{print $4}')" || avail_kb=0
  local avail_mb=$(( avail_kb / 1024 ))
  if (( avail_mb < DISK_FLOOR_MB )); then
    echo "error: ensure-playwright.sh: cache filesystem has ${avail_mb} MB free; >= ${DISK_FLOOR_MB} MB required." >&2
    echo "       Cache root: $target_dir" >&2
    exit 1
  fi
}

# -- Stale sweep (opt-in, content-based, clock-skew safe) --------------------
# Removes namespace directories > 90 days old (by sentinel completed_at content).
# Always preserves the active namespace ($LOCKHASH).

run_sweep() {
  for subdir in "$CACHE_ROOT"/*/; do
    [[ -d "$subdir" ]] || continue
    local sdir_hash
    sdir_hash="$(basename "$subdir")"
    [[ "$sdir_hash" == "$LOCKHASH" ]] && continue
    local sdir_sentinel="$subdir/.bootstrap-sentinel"
    [[ -f "$sdir_sentinel" ]] || continue
    local sdir_completed_at=""
    sdir_completed_at="$(jq -r '.completed_at // empty' "$sdir_sentinel" 2>/dev/null)" || continue
    [[ -z "$sdir_completed_at" ]] && continue
    local sdir_epoch=""
    if date -d "$sdir_completed_at" +%s >/dev/null 2>&1; then
      sdir_epoch="$(date -d "$sdir_completed_at" +%s)"
    elif date -j -f "%Y-%m-%dT%H:%M:%SZ" "$sdir_completed_at" +%s >/dev/null 2>&1; then
      sdir_epoch="$(date -j -f "%Y-%m-%dT%H:%M:%SZ" "$sdir_completed_at" +%s)"
    else
      continue
    fi
    local now_epoch
    now_epoch="$(date -u +%s)"
    local delta_days=$(( (now_epoch - sdir_epoch) / 86400 ))
    if (( delta_days < 0 )); then
      echo "warning: ensure-playwright.sh: skipping sweep of $sdir_hash (completed_at in future: $sdir_completed_at)" >&2
      continue
    fi
    if (( delta_days > 90 )); then
      local sdir_pv=""
      sdir_pv="$(jq -r '.playwright_version // "unknown"' "$sdir_sentinel" 2>/dev/null || echo "unknown")"
      echo "inventory-design: pruning stale Playwright cache $sdir_hash (playwright=$sdir_pv, last bootstrap $sdir_completed_at, $delta_days days old)" >&2
      rm -rf "$subdir"
    fi
  done
}

# -- Sentinel check (is cache valid?) -----------------------------------------

sentinel_valid() {
  [[ -f "$SENTINEL" ]] || return 1
  local lh
  lh="$(jq -r '.lockhash // empty' "$SENTINEL" 2>/dev/null)" || return 1
  [[ "$lh" == "$LOCKHASH" ]] || return 1
  # Verify playwright is resolvable from cache node_modules
  node -e "require.resolve('playwright', {paths: ['$NS_ROOT/node_modules']})" >/dev/null 2>&1 || return 1
  return 0
}

# -- Write sentinel -----------------------------------------------------------

write_sentinel() {
  local playwright_version
  playwright_version="$(jq -r .version "$NS_ROOT/node_modules/playwright/package.json" 2>/dev/null || echo "unknown")"
  local node_version
  node_version="$(node -v 2>/dev/null || echo "unknown")"
  local completed_at
  completed_at="$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u +%Y-%m-%dT%H:%M:%SZ)"

  jq -n \
    --arg lh "$LOCKHASH" \
    --arg nv "$node_version" \
    --arg pv "$playwright_version" \
    --arg ts "$completed_at" \
    '{lockhash: $lh, node_version: $nv, playwright_version: $pv, completed_at: $ts}' \
    > "${SENTINEL}.tmp"
  mv "${SENTINEL}.tmp" "$SENTINEL"

  # Update top-level pointer
  cp "$SENTINEL" "${TOP_SENTINEL}.tmp"
  mv "${TOP_SENTINEL}.tmp" "$TOP_SENTINEL"
}

# -- Mock fast path (both OK flags set) ---------------------------------------

if [[ "${ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK:-0}" == "1" ]] && \
   [[ "${ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_OK:-0}" == "1" ]]; then
  # Fast mock: create expected directory structure and write sentinel

  check_node

  acquire_lock

  # Re-check sentinel under lock
  if sentinel_valid; then
    release_lock
    # Stale sweep even on fast path
    if [[ "${ACCELERATOR_PLAYWRIGHT_SWEEP:-0}" == "1" ]]; then
      # Sweep handled below; skip here to avoid code duplication
      :
    else
      exit 0
    fi
  fi

  mkdir -p "$NS_ROOT/node_modules/playwright"
  chmod 0700 "$NS_ROOT"

  # Write a minimal playwright package.json to satisfy sentinel_valid
  printf '{"name":"playwright","version":"1.49.1"}' \
    > "$NS_ROOT/node_modules/playwright/package.json"

  write_sentinel
  release_lock

  [[ "${ACCELERATOR_PLAYWRIGHT_SWEEP:-0}" == "1" ]] && run_sweep

  exit 0
fi

# -- Mock failure paths -------------------------------------------------------

if [[ -n "${ACCELERATOR_PLAYWRIGHT_MOCK_NPM_EXIT:-}" ]]; then
  check_node
  acquire_lock
  mkdir -p "$NS_ROOT"
  echo "error: npm ci failed with exit ${ACCELERATOR_PLAYWRIGHT_MOCK_NPM_EXIT} (simulated)" >&2
  echo "       Check: NPM_CONFIG_REGISTRY, NODE_EXTRA_CA_CERTS, HTTPS_PROXY" >&2
  release_lock
  exit 1
fi

if [[ -n "${ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_EXIT:-}" ]]; then
  check_node
  acquire_lock
  mkdir -p "$NS_ROOT/node_modules/playwright"
  printf '{"name":"playwright","version":"1.49.1"}' \
    > "$NS_ROOT/node_modules/playwright/package.json"
  echo "error: playwright install chromium failed with exit ${ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_EXIT} (simulated)" >&2
  echo "       Check: PLAYWRIGHT_DOWNLOAD_HOST" >&2
  release_lock
  exit 1
fi

# -- Real install path --------------------------------------------------------

check_node

# 1. Pre-flight disk space check
check_disk "$CACHE_ROOT"

# 2. Create cache directory
if ! mkdir -p "$CACHE_ROOT" 2>/dev/null; then
  echo "error: ensure-playwright.sh: cache directory is not writable; tried $CACHE_ROOT" >&2
  exit 1
fi
mkdir -p "$NS_ROOT"
chmod 0700 "$CACHE_ROOT" "$NS_ROOT" 2>/dev/null || true

# 3. Acquire lock
acquire_lock

# 4. Re-check sentinel under lock (another process may have completed between our check and lock)
if sentinel_valid; then
  release_lock
  [[ "${VERBOSE:-0}" == "1" ]] && echo "inventory-design: Playwright ready (cache: $NS_ROOT)" >&2
  [[ "${ACCELERATOR_PLAYWRIGHT_SWEEP:-0}" == "1" ]] && run_sweep
  exit 0
fi

# 5. Print preamble
echo "inventory-design: first-run setup."
echo "Installing Playwright + Chromium (~150 MB; takes 1-3 min)."
echo "Cache: $NS_ROOT"
echo "Press Ctrl-C to cancel; partial state will be cleaned up."

# 6. Copy manifests into namespace
cp "$PKG_JSON" "$PKG_LOCK" "$NS_ROOT/"

# 7. npm ci (no --silent so progress is visible)
cd "$NS_ROOT"
if ! npm ci --ignore-scripts --no-fund; then
  echo "error: npm ci failed. Check your network configuration:" >&2
  echo "  NPM_CONFIG_REGISTRY, NODE_EXTRA_CA_CERTS, HTTPS_PROXY" >&2
  release_lock
  exit 1
fi

# 7a. Non-failing audit (surface advisory warnings)
npm audit --omit=dev --audit-level=high 2>&1 || \
  echo "inventory-design: npm audit reported advisories; review with \`npm audit\` in $NS_ROOT" >&2

# 8. Install Chromium browser binary
if ! npx playwright install chromium; then
  echo "error: playwright install chromium failed. Check your network configuration:" >&2
  echo "  PLAYWRIGHT_DOWNLOAD_HOST" >&2
  release_lock
  exit 1
fi

# 9. Write sentinel and top-level pointer
write_sentinel

release_lock

echo "inventory-design: setup complete."

# 10. Opt-in stale sweep
[[ "${ACCELERATOR_PLAYWRIGHT_SWEEP:-0}" == "1" ]] && run_sweep

exit 0
