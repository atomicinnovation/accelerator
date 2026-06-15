#!/usr/bin/env bash
# Black-box suite for a9r-resolve.sh — the hot-path binary-resolution helper
# the config-read shims build on. Exercises the precedence order and, more
# importantly, the trust gates: a present binary that is symlinked,
# world-writable, not in the verified checksums manifest, configured by a
# team-committed key, or lacking the config-read subcommands must NOT be
# returned, so the shim falls back to bash rather than executing it.
#
# Each case runs a9r_bin in a clean subshell (env -u clears the resolution
# vars) so ambient A9R_BIN / A9R_FORCE_BASH from the parity run cannot leak in.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

RESOLVE_LIB="$SCRIPT_DIR/a9r-resolve.sh"

if ! command -v jq >/dev/null 2>&1; then
  echo "test-a9r-resolve.sh: jq is required" >&2
  exit 1
fi

TMP_BASE=$(mktemp -d)
trap 'rm -rf "$TMP_BASE"' EXIT

sha_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

# A fake binary that speaks the config-read subcommands (config-read-path
# --help exits 0), so it passes the subcommand probe.
make_a9r_fake() {
  local p="$1"
  cat >"$p" <<'EOF'
#!/usr/bin/env bash
case "${1:-}" in
  config-read-path | config-read-value)
    [ "${2:-}" = "--help" ] && exit 0
    echo "FAKE-$1"
    exit 0
    ;;
esac
exit 0
EOF
  chmod 0755 "$p"
}

# A fake pre-rename, visualiser-only binary: it only understands --config and
# rejects the config-read subcommands (exit 2), so the probe must reject it.
make_legacy_fake() {
  local p="$1"
  cat >"$p" <<'EOF'
#!/usr/bin/env bash
case "${1:-}" in
  --config) exit 0 ;;
  *)
    echo "unknown subcommand: ${1:-}" >&2
    exit 2
    ;;
esac
EOF
  chmod 0755 "$p"
}

# resolve_in <cwd> [VAR=VAL ...] — run a9r_bin in a clean subshell from <cwd>
# with ONLY the given resolution-var overrides set. Captures stdout to
# RESOLVE_OUT and the exit code to RESOLVE_RC.
RESOLVE_OUT=""
RESOLVE_RC=0
resolve_in() {
  local cwd="$1"
  shift
  RESOLVE_RC=0
  # SC2016: the single quotes are intentional — $0 must expand in the INNER
  # bash (bound to "$RESOLVE_LIB"), not the outer shell.
  # shellcheck disable=SC2016
  RESOLVE_OUT="$(cd "$cwd" && env \
    -u A9R_BIN -u A9R_FORCE_BASH -u ACCELERATOR_VISUALISER_BIN \
    -u A9R_BIN_DIR -u A9R_PLATFORM "$@" \
    bash -c 'source "$0"; a9r_bin' "$RESOLVE_LIB" 2>/dev/null)" || RESOLVE_RC=$?
}

# Empty bin dir so the cache tier always misses for the non-cache cases.
EMPTY_BIN="$TMP_BASE/empty-bin"
mkdir -p "$EMPTY_BIN"
NON_CACHE=(A9R_BIN_DIR="$EMPTY_BIN" A9R_PLATFORM=test-plat)

A9R_FAKE="$TMP_BASE/a9r-fake"
make_a9r_fake "$A9R_FAKE"
LEGACY_FAKE="$TMP_BASE/legacy-fake"
make_legacy_fake "$LEGACY_FAKE"

# Neutral cwd with no VCS markers, so find_repo_root falls back to $PWD and no
# stray config is read for the env-only cases.
NEUTRAL="$TMP_BASE/neutral"
mkdir -p "$NEUTRAL"

echo "=== a9r-resolve precedence + escape hatch ==="

# A9R_FORCE_BASH forces fallback even when A9R_BIN would otherwise resolve.
resolve_in "$NEUTRAL" "${NON_CACHE[@]}" A9R_FORCE_BASH=1 A9R_BIN="$A9R_FAKE"
assert_eq "A9R_FORCE_BASH forces fallback (non-zero)" "1" "$RESOLVE_RC"
assert_empty "A9R_FORCE_BASH prints nothing" "$RESOLVE_OUT"

# A9R_BIN: explicit executable override is trusted as-is.
resolve_in "$NEUTRAL" "${NON_CACHE[@]}" A9R_BIN="$A9R_FAKE"
assert_eq "A9R_BIN executable resolves (rc 0)" "0" "$RESOLVE_RC"
assert_eq "A9R_BIN executable returns its path" "$A9R_FAKE" "$RESOLVE_OUT"

# A9R_BIN set to a non-executable path → fall back (no silent trust).
NON_EXEC="$TMP_BASE/not-exec"
echo "x" >"$NON_EXEC"
chmod 0644 "$NON_EXEC"
resolve_in "$NEUTRAL" "${NON_CACHE[@]}" A9R_BIN="$NON_EXEC"
assert_eq "A9R_BIN non-executable falls back" "1" "$RESOLVE_RC"

echo "=== ACCELERATOR_VISUALISER_BIN probe ==="

# Visualiser bin that speaks the subcommands → trusted.
resolve_in "$NEUTRAL" "${NON_CACHE[@]}" ACCELERATOR_VISUALISER_BIN="$A9R_FAKE"
assert_eq "VISUALISER_BIN with subcommands resolves" "0" "$RESOLVE_RC"
assert_eq "VISUALISER_BIN returns its path" "$A9R_FAKE" "$RESOLVE_OUT"

# Legacy visualiser-only binary lacks the subcommands → fall back.
resolve_in "$NEUTRAL" "${NON_CACHE[@]}" ACCELERATOR_VISUALISER_BIN="$LEGACY_FAKE"
assert_eq "legacy VISUALISER_BIN falls back (no subcommands)" "1" "$RESOLVE_RC"

echo "=== config visualiser.binary: local only, never team ==="

mk_repo() {
  local d
  d=$(mktemp -d "$TMP_BASE/repo-XXXXXX")
  mkdir -p "$d/.git" "$d/.accelerator"
  echo "$d"
}

# Gitignored config.local.md key is honoured.
REPO_LOCAL=$(mk_repo)
cat >"$REPO_LOCAL/.accelerator/config.local.md" <<EOF
---
visualiser:
  binary: $A9R_FAKE
---
EOF
resolve_in "$REPO_LOCAL" "${NON_CACHE[@]}"
assert_eq "config.local.md visualiser.binary honoured" "0" "$RESOLVE_RC"
assert_eq "config.local.md returns configured path" "$A9R_FAKE" "$RESOLVE_OUT"

# Team-committed config.md key is ignored on the hot path (RCE guard).
REPO_TEAM=$(mk_repo)
cat >"$REPO_TEAM/.accelerator/config.md" <<EOF
---
visualiser:
  binary: $A9R_FAKE
---
EOF
resolve_in "$REPO_TEAM" "${NON_CACHE[@]}"
assert_eq "team config.md visualiser.binary ignored" "1" "$RESOLVE_RC"
assert_empty "team config.md key yields no path" "$RESOLVE_OUT"

echo "=== cached binary trust gates ==="

# Helper: a fresh bin dir with a manifest entry for asset a9r-test-plat.
# Usage: write_manifest <bindir> <sha-or-empty>
write_manifest() {
  local bindir="$1" sha="$2"
  if [ -n "$sha" ]; then
    printf '{"version":"test","binaries":{"test-plat":{"a9r-test-plat":"sha256:%s"}}}\n' \
      "$sha" >"$bindir/checksums.json"
  else
    printf '{"version":"test","binaries":{"test-plat":{}}}\n' >"$bindir/checksums.json"
  fi
}

# Valid cache: regular, 0755, SHA present in manifest → trusted.
BD_OK="$TMP_BASE/bd-ok"
mkdir -p "$BD_OK"
CACHED_OK="$BD_OK/a9r-test-plat"
make_a9r_fake "$CACHED_OK"
write_manifest "$BD_OK" "$(sha_of "$CACHED_OK")"
resolve_in "$NEUTRAL" A9R_BIN_DIR="$BD_OK" A9R_PLATFORM=test-plat
assert_eq "valid manifest-verified cache resolves" "0" "$RESOLVE_RC"
assert_eq "valid cache returns cached path" "$CACHED_OK" "$RESOLVE_OUT"

# Symlinked cache → rejected even though the target's SHA matches.
BD_LINK="$TMP_BASE/bd-link"
mkdir -p "$BD_LINK"
REAL_TARGET="$TMP_BASE/real-target"
make_a9r_fake "$REAL_TARGET"
ln -s "$REAL_TARGET" "$BD_LINK/a9r-test-plat"
write_manifest "$BD_LINK" "$(sha_of "$REAL_TARGET")"
resolve_in "$NEUTRAL" A9R_BIN_DIR="$BD_LINK" A9R_PLATFORM=test-plat
assert_eq "symlinked cache rejected" "1" "$RESOLVE_RC"

# World-writable cache → rejected even though the SHA matches.
BD_WW="$TMP_BASE/bd-ww"
mkdir -p "$BD_WW"
CACHED_WW="$BD_WW/a9r-test-plat"
make_a9r_fake "$CACHED_WW"
write_manifest "$BD_WW" "$(sha_of "$CACHED_WW")"
chmod 0777 "$CACHED_WW"
resolve_in "$NEUTRAL" A9R_BIN_DIR="$BD_WW" A9R_PLATFORM=test-plat
assert_eq "world-writable cache rejected" "1" "$RESOLVE_RC"

# SHA not in manifest (wrong hash) → rejected.
BD_BADSHA="$TMP_BASE/bd-badsha"
mkdir -p "$BD_BADSHA"
CACHED_BADSHA="$BD_BADSHA/a9r-test-plat"
make_a9r_fake "$CACHED_BADSHA"
write_manifest "$BD_BADSHA" \
  "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
resolve_in "$NEUTRAL" A9R_BIN_DIR="$BD_BADSHA" A9R_PLATFORM=test-plat
assert_eq "cache SHA not in manifest rejected" "1" "$RESOLVE_RC"

# No manifest entry for the asset → rejected.
BD_NOENTRY="$TMP_BASE/bd-noentry"
mkdir -p "$BD_NOENTRY"
CACHED_NOENTRY="$BD_NOENTRY/a9r-test-plat"
make_a9r_fake "$CACHED_NOENTRY"
write_manifest "$BD_NOENTRY" ""
resolve_in "$NEUTRAL" A9R_BIN_DIR="$BD_NOENTRY" A9R_PLATFORM=test-plat
assert_eq "cache with no manifest entry rejected" "1" "$RESOLVE_RC"

# Flat pre-Phase-5 schema (binaries[platform] is a string) → no a9r asset, no
# jq type error → rejected.
BD_FLAT="$TMP_BASE/bd-flat"
mkdir -p "$BD_FLAT"
CACHED_FLAT="$BD_FLAT/a9r-test-plat"
make_a9r_fake "$CACHED_FLAT"
printf '{"version":"test","binaries":{"test-plat":"sha256:%s"}}\n' \
  "$(sha_of "$CACHED_FLAT")" >"$BD_FLAT/checksums.json"
resolve_in "$NEUTRAL" A9R_BIN_DIR="$BD_FLAT" A9R_PLATFORM=test-plat
assert_eq "flat-schema manifest yields no a9r asset (rejected)" "1" "$RESOLVE_RC"

# Sentinel (all-zeros) SHA → rejected.
BD_SENT="$TMP_BASE/bd-sentinel"
mkdir -p "$BD_SENT"
CACHED_SENT="$BD_SENT/a9r-test-plat"
make_a9r_fake "$CACHED_SENT"
write_manifest "$BD_SENT" \
  "0000000000000000000000000000000000000000000000000000000000000000"
resolve_in "$NEUTRAL" A9R_BIN_DIR="$BD_SENT" A9R_PLATFORM=test-plat
assert_eq "all-zeros sentinel SHA rejected" "1" "$RESOLVE_RC"

test_summary
