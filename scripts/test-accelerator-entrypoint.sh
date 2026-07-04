#!/usr/bin/env bash
#
# test-accelerator-entrypoint.sh — hermetic tests for bin/accelerator.
#
# No network: fetches are stubbed via ACCELERATOR_BOOTSTRAP_DOWNLOADER serving a
# local "server" dir, and host detection is driven by injected uname. Signatures
# are real (minisign) and verified by the real compiled accelerator-verify shim,
# so the fail-closed root-of-trust path is exercised end-to-end.
#
# Skips cleanly when minisign or cargo is unavailable.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BOOTSTRAP="${REPO_ROOT}/bin/accelerator"

pass=0
fail=0
note() { printf '  %s\n' "$1"; }
ok() {
  pass=$((pass + 1))
  printf 'ok   - %s\n' "$1"
}
bad() {
  fail=$((fail + 1))
  printf 'FAIL - %s\n' "$1"
}

command -v minisign >/dev/null 2>&1 || {
  echo "skipping: minisign not on PATH"
  exit 0
}
command -v cargo >/dev/null 2>&1 || {
  echo "skipping: cargo not on PATH"
  exit 0
}

# Build + locate the real verify shim.
(cd "${REPO_ROOT}/cli" && cargo build --quiet -p accelerator-verify) || {
  echo "skipping: could not build accelerator-verify"
  exit 0
}
SHIM_BIN="${REPO_ROOT}/cli/target/debug/accelerator-verify"
[ -x "${SHIM_BIN}" ] || {
  echo "skipping: shim binary not found at ${SHIM_BIN}"
  exit 0
}

# Host platform alias (same normalisation the bootstrap uses).
case "$(uname -m)" in
  arm64 | aarch64) HOST_ARCH=arm64 ;;
  x86_64 | amd64) HOST_ARCH=x64 ;;
  *)
    echo "skipping: unsupported host arch"
    exit 0
    ;;
esac
case "$(uname -s)" in
  Darwin) HOST_OS=darwin ;;
  Linux) HOST_OS=linux ;;
  *)
    echo "skipping: unsupported host os"
    exit 0
    ;;
esac
PLATFORM="${HOST_OS}-${HOST_ARCH}"

WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

# A committed-style release keypair.
minisign -G -W -f -p "${WORK}/release.pub" -s "${WORK}/release.key" \
  >/dev/null 2>&1
minisign -G -W -f -p "${WORK}/attacker.pub" -s "${WORK}/attacker.key" \
  >/dev/null 2>&1

# A fake launcher: records its argv and exits with an env-chosen code.
make_launcher() {
  cat >"$1" <<'LAUNCHER'
#!/bin/sh
if [ -n "${LAUNCHER_ARGS_OUT:-}" ]; then
	printf '%s\n' "$@" >"${LAUNCHER_ARGS_OUT}"
fi
exit "${LAUNCHER_EXIT:-0}"
LAUNCHER
  chmod +x "$1"
}

# A stub downloader: copies "${SERVER_DIR}/<basename>" to the destination,
# appending each requested URL to "${DL_LOG}".
make_downloader() {
  cat >"$1" <<'DL'
#!/bin/sh
printf '%s\n' "$1" >>"${DL_LOG}"
base=$(basename "$1")
if [ -f "${SERVER_DIR}/${base}" ]; then
	cp "${SERVER_DIR}/${base}" "$2"
	exit 0
fi
exit 22
DL
  chmod +x "$1"
}

DOWNLOADER="${WORK}/downloader.sh"
make_downloader "${DOWNLOADER}"

# Build a fresh plugin root + server for one scenario. Args: <secret-key-for-sig>
new_harness() {
  local secret="$1"
  local root
  root="$(mktemp -d "${WORK}/root.XXXXXX")"
  mkdir -p "${root}/.claude-plugin" "${root}/keys" "${root}/bin"
  printf '{\n  "name": "accelerator",\n  "version": "9.9.9-test"\n}\n' \
    >"${root}/.claude-plugin/plugin.json"
  cp "${WORK}/release.pub" "${root}/keys/accelerator-release.pub"
  cp "${BOOTSTRAP}" "${root}/bin/accelerator"
  cp "${SHIM_BIN}" "${root}/bin/accelerator-verify-${PLATFORM}"

  local server
  server="$(mktemp -d "${WORK}/server.XXXXXX")"
  make_launcher "${server}/accelerator-${PLATFORM}"
  minisign -S -s "${secret}" -m "${server}/accelerator-${PLATFORM}" \
    -x "${server}/accelerator-${PLATFORM}.minisig" >/dev/null 2>&1
  printf '%s\n%s\n' "${root}" "${server}"
}

run_bootstrap() {
  # Usage: run_bootstrap <root> <server> [extra env assignments...] -- [args...]
  local root="$1" server="$2"
  shift 2
  local -a envs=()
  while [ "$#" -gt 0 ] && [ "$1" != "--" ]; do
    envs+=("$1")
    shift
  done
  [ "${1:-}" = "--" ] && shift
  env -i PATH="${PATH}" HOME="${HOME:-/tmp}" \
    CLAUDE_PLUGIN_ROOT="${root}" \
    ACCELERATOR_BOOTSTRAP_DOWNLOADER="${DOWNLOADER}" \
    ACCELERATOR_RELEASE_BASE_URL="https://example.invalid/v9.9.9-test" \
    SERVER_DIR="${server}" DL_LOG="${server}/dl.log" \
    "${envs[@]}" \
    bash "${root}/bin/accelerator" "$@"
}

# --- unset CLAUDE_PLUGIN_ROOT ----------------------------------------------
out=$(env -i PATH="${PATH}" bash "${BOOTSTRAP}" 2>&1)
status=$?
if [ "${status}" -ne 0 ] && printf '%s' "${out}" | grep -q "CLAUDE_PLUGIN_ROOT"; then
  ok "unset CLAUDE_PLUGIN_ROOT fails with a named error"
else
  bad "unset CLAUDE_PLUGIN_ROOT: ${out}"
fi

# --- host detection: injected uname over all four alias combos -------------
for combo in "Darwin arm64 darwin-arm64" "Darwin aarch64 darwin-arm64" \
  "Linux x86_64 linux-x64" "Linux amd64 linux-x64"; do
  # Deliberate word-split of the space-separated combo into three positionals.
  # shellcheck disable=SC2086
  set -- $combo
  u_s="$1" u_m="$2" want="$3"
  h=$(new_harness "${WORK}/release.key")
  root=$(printf '%s' "${h}" | sed -n '1p')
  server=$(printf '%s' "${h}" | sed -n '2p')
  # Serve the launcher under the EXPECTED alias so a correct normalisation
  # fetches it; a wrong alias 404s. Also stage the shim under that alias.
  cp "${SHIM_BIN}" "${root}/bin/accelerator-verify-${want}"
  make_launcher "${server}/accelerator-${want}"
  minisign -S -s "${WORK}/release.key" \
    -m "${server}/accelerator-${want}" \
    -x "${server}/accelerator-${want}.minisig" >/dev/null 2>&1
  out=$(run_bootstrap "${root}" "${server}" \
    ACCELERATOR_UNAME_S="${u_s}" ACCELERATOR_UNAME_M="${u_m}" -- 2>&1)
  if grep -q "accelerator-${want}\$" "${server}/dl.log" 2>/dev/null; then
    ok "host detection ${u_s}/${u_m} -> ${want}"
  else
    bad "host detection ${u_s}/${u_m} -> ${want}: $(cat "${server}/dl.log" 2>/dev/null) ${out}"
  fi
done

# --- happy path: fetch, verify, cache, exec with arg + exit forwarding ------
h=$(new_harness "${WORK}/release.key")
root=$(printf '%s' "${h}" | sed -n '1p')
server=$(printf '%s' "${h}" | sed -n '2p')
args_out="${WORK}/args.$$"
run_bootstrap "${root}" "${server}" \
  LAUNCHER_ARGS_OUT="${args_out}" LAUNCHER_EXIT=7 -- alpha "be ta" >/dev/null 2>&1
code=$?
if [ "${code}" -eq 7 ] && [ "$(sed -n '1p' "${args_out}")" = "alpha" ] &&
  [ "$(sed -n '2p' "${args_out}")" = "be ta" ]; then
  ok "happy path forwards args and exit code"
else
  bad "happy path: code=${code} args=$(cat "${args_out}" 2>/dev/null)"
fi

# --- cache short-circuit: a second run performs no fetch -------------------
run_bootstrap "${root}" "${server}" -- >/dev/null 2>&1
first=$(wc -l <"${server}/dl.log")
run_bootstrap "${root}" "${server}" -- >/dev/null 2>&1
second=$(wc -l <"${server}/dl.log")
if [ "${first}" = "${second}" ]; then
  ok "cache hit performs no further fetch"
else
  bad "cache hit refetched: ${first} -> ${second}"
fi

# --- tampered cached launcher is refused and re-fetched --------------------
launcher_path="${root}/bin/accelerator-launcher-9.9.9-test-${PLATFORM}"
printf 'poisoned' >"${launcher_path}"
run_bootstrap "${root}" "${server}" LAUNCHER_EXIT=0 -- >/dev/null 2>&1
status=$?
if [ "${status}" -eq 0 ] && ! grep -q poisoned "${launcher_path}" 2>/dev/null; then
  ok "tampered cached launcher is refused and healed"
else
  bad "tampered cache not healed"
fi

# --- a non-release-key signature is refused, fail-closed -------------------
h=$(new_harness "${WORK}/attacker.key")
root=$(printf '%s' "${h}" | sed -n '1p')
server=$(printf '%s' "${h}" | sed -n '2p')
out=$(run_bootstrap "${root}" "${server}" -- 2>&1)
status=$?
if [ "${status}" -ne 0 ] && printf '%s' "${out}" | grep -q "verify"; then
  ok "non-release-key signature is refused fail-closed"
else
  bad "non-release-key not refused: ${out}"
fi

# --- an unrunnable verify shim fails closed --------------------------------
h=$(new_harness "${WORK}/release.key")
root=$(printf '%s' "${h}" | sed -n '1p')
server=$(printf '%s' "${h}" | sed -n '2p')
printf 'not a binary' >"${root}/bin/accelerator-verify-${PLATFORM}"
chmod +x "${root}/bin/accelerator-verify-${PLATFORM}"
out=$(run_bootstrap "${root}" "${server}" -- 2>&1)
status=$?
if [ "${status}" -ne 0 ]; then
  ok "an unrunnable verify shim fails closed"
else
  bad "unrunnable shim did not fail closed: ${out}"
fi

# --- read-only plugin root: override works, absence errors -----------------
h=$(new_harness "${WORK}/release.key")
root=$(printf '%s' "${h}" | sed -n '1p')
server=$(printf '%s' "${h}" | sed -n '2p')
chmod -R a-w "${root}/bin" 2>/dev/null
alt="$(mktemp -d "${WORK}/alt.XXXXXX")"
run_bootstrap "${root}" "${server}" ACCELERATOR_CACHE_DIR="${alt}" -- \
  >/dev/null 2>&1
status=$?
if [ "${status}" -eq 0 ] && [ -x "${alt}/accelerator-launcher-9.9.9-test-${PLATFORM}" ]; then
  ok "read-only root + ACCELERATOR_CACHE_DIR runs from the override"
else
  bad "read-only root + override failed"
fi
out=$(run_bootstrap "${root}" "${server}" -- 2>&1)
status=$?
if [ "${status}" -ne 0 ] && printf '%s' "${out}" | grep -q "cache directory"; then
  ok "read-only root without an override is a named error (no XDG)"
else
  bad "read-only root without override: ${out}"
fi
chmod -R u+w "${root}/bin" 2>/dev/null

# --- a stale lock (owner PID gone) is reclaimed ----------------------------
h=$(new_harness "${WORK}/release.key")
root=$(printf '%s' "${h}" | sed -n '1p')
server=$(printf '%s' "${h}" | sed -n '2p')
lock="${root}/bin/.accelerator-lock-${PLATFORM}"
mkdir -p "${lock}"
printf '999999\n' >"${lock}/pid" # a PID that is not running
out=$(run_bootstrap "${root}" "${server}" -- 2>&1)
status=$?
if [ "${status}" -eq 0 ]; then
  ok "a lock orphaned by an exited process is reclaimed"
else
  bad "stale lock wedged the bootstrap: ${out}"
fi

printf '\n%s passed, %s failed\n' "${pass}" "${fail}"
[ "${fail}" -eq 0 ]
