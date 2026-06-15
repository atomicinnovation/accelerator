#!/usr/bin/env bash
# a9r-resolve.sh — single source of truth for resolving a usable `a9r` binary.
#
# Sourced (NOT executed) by the config-read shims and, from Phase 5, the
# visualiser launcher's acquire helper. `a9r_bin` prints a trusted binary path
# and returns 0, or prints nothing and returns non-zero so the caller falls
# back to the bash implementation. No download happens here (pure resolution);
# acquisition is the provisioning hook's job (Phase 5).
#
# This runs on the load-time hot path — invoked on nearly every skill load —
# so the trust gates are deliberately stricter than the use-time-only
# visualiser launch: a binary is executed automatically, against a far larger
# blast radius, so a present-but-untrusted binary must degrade to bash rather
# than run.
#
# bash 3.2 floor (macOS /bin/bash, ADR-0016): no associative arrays, no
# ${var,,}/${var^^}, no mapfile. Scanned by lint-bashisms.sh.

_A9R_RESOLVE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# bin/ directory holding the cached binary and its checksums manifest. Defaults
# to the visualiser skill's bin/ (relative to this library), overridable for
# tests. Mirrors launch-server.sh's $SKILL_ROOT/bin layout.
_a9r_bin_dir() {
  if [ -n "${A9R_BIN_DIR:-}" ]; then
    printf '%s\n' "$A9R_BIN_DIR"
  else
    printf '%s\n' "$_A9R_RESOLVE_DIR/../skills/visualisation/visualise/bin"
  fi
}

# Platform tag (`<os>-<arch>`) matching the released asset name and the
# checksums manifest keys. Mirrors launch-server.sh's detection. Overridable
# via A9R_PLATFORM for tests. Empty when the host is unsupported.
_a9r_platform() {
  if [ -n "${A9R_PLATFORM:-}" ]; then
    printf '%s\n' "$A9R_PLATFORM"
    return 0
  fi
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "$os" in
    darwin | linux) ;;
    *) return 0 ;;
  esac
  case "$arch" in
    arm64 | aarch64) arch="arm64" ;;
    x86_64) arch="x64" ;;
    *) return 0 ;;
  esac
  printf '%s-%s\n' "$os" "$arch"
}

# SHA-256 of a file, stdout. sha256sum (Linux) then shasum -a 256 (macOS);
# non-zero if neither is available.
_a9r_sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" 2>/dev/null | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" 2>/dev/null | awk '{print $1}'
  else
    return 1
  fi
}

# Is the file world-writable? Exit 0 if yes (→ reject). Uses BSD `stat -f`
# (macOS) then GNU `stat -c` (Linux); if neither reports a mode, errs on the
# safe side and treats it as world-writable (reject).
_a9r_world_writable() {
  local mode other
  mode="$(stat -f '%Lp' "$1" 2>/dev/null || stat -c '%a' "$1" 2>/dev/null || true)"
  [ -n "$mode" ] || return 0
  other="${mode#"${mode%?}"}"
  case "$other" in
    2 | 3 | 6 | 7) return 0 ;;
    *) return 1 ;;
  esac
}

# Does <bin> understand the config-read subcommands? A pre-rename,
# visualiser-only binary (e.g. a stale ACCELERATOR_VISUALISER_BIN) lacks them
# and must NOT be trusted on the hot path — it would error, not degrade. Probed
# with `--help` so no side effects and no config is read.
_a9r_supports_subcommands() {
  "$1" config-read-path --help >/dev/null 2>&1
}

# Read visualiser.binary from the gitignored .accelerator/config.local.md ONLY,
# never the team-committed .accelerator/config.md: a team-committed
# `visualiser.binary: ./evil` would otherwise auto-execute an attacker-chosen
# binary on every skill load of a merely-cloned or PR'd repo. Relative paths
# resolve against the project root. Prints the resolved path or nothing.
_a9r_local_binary() {
  local root local_file value
  root="$(find_repo_root 2>/dev/null || echo "$PWD")"
  local_file="$root/.accelerator/config.local.md"
  [ -f "$local_file" ] || return 0
  # Cheap gate: only spawn the reader if the key could plausibly be present.
  grep -q 'binary' "$local_file" 2>/dev/null || return 0
  value="$(ACCELERATOR_CONFIG_LOCAL_ONLY=1 \
    "$_A9R_RESOLVE_DIR/config-read-value-impl.sh" visualiser.binary "" \
    2>/dev/null || true)"
  [ -n "$value" ] || return 0
  case "$value" in
    /*) printf '%s\n' "$value" ;;
    *) printf '%s\n' "$root/$value" ;;
  esac
}

# Is the cached <bin> a safe, manifest-trusted regular file? Exit 0 if it may
# be executed. Rejects: missing, symlink, non-regular, non-executable,
# world-writable, or a SHA-256 absent from the verified checksums manifest.
# This is why fallback is NOT limited to the binary-absent case: a
# present-but-tampered or present-but-buggy binary is rejected here too.
_a9r_cache_trusted() {
  local bin="$1"
  [ -n "$bin" ] || return 1
  [ ! -L "$bin" ] || return 1
  [ -f "$bin" ] || return 1
  [ -x "$bin" ] || return 1
  ! _a9r_world_writable "$bin" || return 1

  local platform manifest asset expected actual
  platform="$(_a9r_platform)"
  [ -n "$platform" ] || return 1
  manifest="$(_a9r_bin_dir)/checksums.json"
  [ -f "$manifest" ] || return 1
  command -v jq >/dev/null 2>&1 || return 1
  asset="a9r-$platform"
  # Defensive lookup: tolerate both the flat pre-Phase-5 schema
  # (binaries[platform] is a string → no a9r asset yet → empty) and the
  # asset-nested schema Phase 5 introduces, without a jq type error.
  expected="$(jq -r --arg p "$platform" --arg a "$asset" \
    '.binaries[$p] | if type == "object" then .[$a] else empty end // empty' \
    "$manifest" 2>/dev/null || true)"
  expected="${expected#sha256:}"
  [ -n "$expected" ] || return 1
  case "$expected" in
    0000000000000000000000000000000000000000000000000000000000000000)
      return 1
      ;;
  esac
  # NOTE: hashes on every call. Cheap in practice — in dev/test A9R_BIN
  # short-circuits before reaching here, and until a release ships an `a9r`
  # asset no cached file matches the manifest, so this branch is cold. Phase 5
  # adds an mtime/size guard against the eager-hook-verified cache to skip the
  # re-hash once the cache is populated.
  actual="$(_a9r_sha256 "$bin" || true)"
  [ -n "$actual" ] || return 1
  [ "$actual" = "$expected" ] || return 1
  return 0
}

# Resolve a usable `a9r` binary. Prints the path and returns 0, or returns
# non-zero (printing nothing) so the caller falls back to bash. Precedence:
#   1. A9R_BIN                  — explicit test/dev override, trusted.
#   2. ACCELERATOR_VISUALISER_BIN — trusted only if it speaks the subcommands.
#   3. config.local.md visualiser.binary — gitignored local config only.
#   4. cached bin/a9r-<platform> — regular, safe-perms, manifest-verified file.
# A9R_FORCE_BASH (set to anything non-empty) forces fallback at the top.
a9r_bin() {
  [ -z "${A9R_FORCE_BASH:-}" ] || return 1

  # 1. Explicit override — a developer signal, trusted as-is.
  if [ -n "${A9R_BIN:-}" ]; then
    [ -x "$A9R_BIN" ] || return 1
    printf '%s\n' "$A9R_BIN"
    return 0
  fi

  # 2. Visualiser binary override — probe before trusting so a legacy
  #    visualiser-only binary degrades to bash rather than erroring.
  if [ -n "${ACCELERATOR_VISUALISER_BIN:-}" ]; then
    if [ -x "$ACCELERATOR_VISUALISER_BIN" ] &&
      _a9r_supports_subcommands "$ACCELERATOR_VISUALISER_BIN"; then
      printf '%s\n' "$ACCELERATOR_VISUALISER_BIN"
      return 0
    fi
    # Set-but-unusable: degrade down the precedence chain, never error.
  fi

  # 3. Gitignored local config key — also probed (untrusted source).
  local configured
  configured="$(_a9r_local_binary)"
  if [ -n "$configured" ] && [ -x "$configured" ] &&
    _a9r_supports_subcommands "$configured"; then
    printf '%s\n' "$configured"
    return 0
  fi

  # 4. Cached, manifest-verified binary.
  local cached
  cached="$(_a9r_bin_dir)/a9r-$(_a9r_platform)"
  if _a9r_cache_trusted "$cached"; then
    printf '%s\n' "$cached"
    return 0
  fi

  return 1
}

# find_repo_root (used by _a9r_local_binary) lives in vcs-common.sh. Sourced
# last so this library's helpers are defined first; vcs-common.sh only defines
# functions at load time, so the order is immaterial beyond tidiness.
# shellcheck source=vcs-common.sh
source "$_A9R_RESOLVE_DIR/vcs-common.sh"
