#!/usr/bin/env bash
# SessionStart hook: eagerly provision the cached a9r binary so the config-read
# shims can route to it instead of bash from the very first skill load.
#
# This hook gates skill loading, so its overriding contract is: NEVER stall and
# NEVER fail a session. Every degrade path — offline, timeout, missing
# curl/wget, all-zeros sentinel, unsupported platform, version drift, SHA
# mismatch — emits nothing and exits 0. The config-read shim's bash fallback
# covers all of them, so a missing/invalid a9r binary is never fatal.
#
# Mode: bounded BLOCKING download (the simplest correct choice). The common
# case (a cache already SHA-valid) does ZERO network work and returns
# immediately; only a cold/invalid cache touches the network, and that is
# capped by download_to's hard timeout budget (ACCELERATOR_DOWNLOAD_*:
# connect 10s, total 60s), so the worst-case session-start latency is bounded.
# The shim degrades to bash for any call that races ahead of completion, so
# there is no correctness need for a background mode.
#
# bash 3.2 floor (macOS /bin/bash, ADR-0016): no associative arrays, no
# ${var,,}/${var^^}, no mapfile. Scanned by lint-bashisms.sh.

# No `set -e`: fail-open means a failing command must not abort the hook.
set -uo pipefail

# jq guard — degrade silently (matches config-detect.sh / vcs-detect.sh).
command -v jq >/dev/null 2>&1 || exit 0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SKILL_ROOT="${ACCELERATOR_VISUALISER_SKILL_ROOT:-$PLUGIN_ROOT/skills/visualisation/visualise}"
BIN_DIR="$SKILL_ROOT/bin"
MANIFEST="$BIN_DIR/checksums.json"
PLUGIN_JSON="$PLUGIN_ROOT/.claude-plugin/plugin.json"
HELPERS="$PLUGIN_ROOT/skills/visualisation/visualise/scripts/launcher-helpers.sh"

[ -f "$MANIFEST" ] || exit 0
[ -f "$PLUGIN_JSON" ] || exit 0
[ -f "$HELPERS" ] || exit 0
# shellcheck source=../skills/visualisation/visualise/scripts/launcher-helpers.sh
source "$HELPERS" 2>/dev/null || exit 0

# Platform tag — mirrors launch-server.sh and a9r-resolve.sh. Unsupported host
# degrades silently (no die_json — the launcher errs, the hook never does).
OS_RAW="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH_RAW="$(uname -m)"
case "$OS_RAW" in
  darwin | linux) OS="$OS_RAW" ;;
  *) exit 0 ;;
esac
case "$ARCH_RAW" in
  arm64 | aarch64) ARCH="arm64" ;;
  x86_64) ARCH="x64" ;;
  *) exit 0 ;;
esac
PLATFORM="$OS-$ARCH"
ASSET="a9r-$PLATFORM"
CACHE="$BIN_DIR/$ASSET"

# Expected SHA for the a9r asset. Tolerates both the nested
# (binaries[platform][asset]) and the legacy flat (binaries[platform] string)
# schema; a flat manifest has no per-asset slot, so it yields empty → degrade.
EXPECTED_RAW="$(jq -r --arg p "$PLATFORM" --arg a "$ASSET" \
  '.binaries[$p] | if type == "object" then (.[$a] // empty) else empty end // empty' \
  "$MANIFEST" 2>/dev/null || true)"
EXPECTED="${EXPECTED_RAW#sha256:}"
# No released a9r asset (key absent, or the all-zeros "not released" sentinel)
# → nothing to provision; the shim uses bash.
[ -n "$EXPECTED" ] || exit 0
case "$EXPECTED" in
  0000000000000000000000000000000000000000000000000000000000000000) exit 0 ;;
esac

# Version-drift guard: only provision the asset matching this plugin version,
# never a stale manifest's hash.
PLUGIN_VERSION="$(jq -r '.version // empty' "$PLUGIN_JSON" 2>/dev/null || true)"
MANIFEST_VERSION="$(jq -r '.version // empty' "$MANIFEST" 2>/dev/null || true)"
[ -n "$PLUGIN_VERSION" ] || exit 0
if [ -n "$MANIFEST_VERSION" ] && [ "$MANIFEST_VERSION" != "$PLUGIN_VERSION" ]; then
  exit 0
fi

# Fast cache-valid fast-path: a regular, non-symlink, SHA-matching cache needs
# no network — return immediately. This is the steady-state path.
if [ -x "$CACHE" ] && [ ! -L "$CACHE" ] &&
  [ "$(sha256_of "$CACHE" 2>/dev/null || true)" = "$EXPECTED" ]; then
  exit 0
fi

# Cold/invalid cache: bounded-blocking acquire. The release mirror override is
# untrusted on this eager, auto-executed path, so it must be HTTPS — a plain
# http:// mirror could inject the binary that the hot path later executes.
# (Tests opt out with ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD to reach a local
# fixture, exactly as the launcher's own download tests do.)
RELEASES_URL_BASE="${ACCELERATOR_VISUALISER_RELEASES_URL:-https://github.com/atomicinnovation/accelerator/releases/download}"
case "$RELEASES_URL_BASE" in
  https://*) ;;
  *) [ -n "${ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD:-}" ] || exit 0 ;;
esac
ASSET_URL="$RELEASES_URL_BASE/v$PLUGIN_VERSION/$ASSET"

# acquire_binary downloads to a sibling tempfile and publishes by atomic
# rename, so a shim resolving the cache mid-download never sees a partial file.
# Any non-zero (download failure, SHA mismatch, no downloader) → degrade.
acquire_binary "$ASSET_URL" "$EXPECTED" "$CACHE" >/dev/null 2>&1 || exit 0
exit 0
