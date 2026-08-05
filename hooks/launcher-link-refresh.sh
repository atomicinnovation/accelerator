#!/usr/bin/env bash
set -uo pipefail
CDPATH=

# SessionStart hook: keep ${CLAUDE_PLUGIN_DATA}/bin/accelerator pointing at the
# current installation's launcher, so a user's own PATH symlink survives plugin
# upgrades without being re-created. Exits 0 in all cases.
#
# The root is self-located, never taken from the environment: this hook
# publishes a persistent, channel-global pointer, so a stale ambient value would
# poison the terminal command until someone noticed. Two of the four sibling
# hooks are env-first; both use the value transiently and then exec. This one
# is invoked by its own absolute path, so its location already *is* the value
# an ambient root would have supplied.
#
# Accumulated rather than emitted inline: at most one JSON object may reach
# stdout, and two conditions can hold in one run.
NOTICE=""

finish() {
  [ -n "$1" ] && printf '[accelerator] %s\n' "$1" >&2
  if [ -n "${NOTICE}" ]; then
    jq -n --arg m "[accelerator] ${NOTICE}" '{systemMessage: $m}' 2>/dev/null ||
      printf '[accelerator] %s\n' "${NOTICE}" >&2
  fi
  exit 0
}

SCRIPT_DIR=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
PLUGIN_ROOT=$(cd -P -- "${SCRIPT_DIR}/.." && pwd -P)
LAUNCHER="${PLUGIN_ROOT}/bin/accelerator"

# Read before the CLAUDE_PLUGIN_DATA guard: a trust-chain signal must not be
# suppressed on the Claude Code versions that lack a convenience feature.
UNVERIFIED_LOG="${ACCELERATOR_CACHE_DIR:-${PLUGIN_ROOT}/bin}/.accelerator-unverified.log"
if [ -s "${UNVERIFIED_LOG}" ]; then
  NOTICE="an unverified launcher was recorded in ${UNVERIFIED_LOG}"
fi

case "${CLAUDE_PLUGIN_DATA:-}" in
  /*) ;;
  *) finish "CLAUDE_PLUGIN_DATA unavailable; no terminal link refreshed. See docs/internals.md#terminal-invocation" ;;
esac

[ -x "${LAUNCHER}" ] ||
  finish "launcher not executable: ${LAUNCHER}; leaving the terminal link alone"

DATA_BIN="${CLAUDE_PLUGIN_DATA}/bin"
LINK="${DATA_BIN}/accelerator"
STAGED="${LINK}.new.$$"

# Before mkdir -p, which would otherwise fail first for a regular file and
# follow a dangling symlink out of the tree for the other flavour.
if [ -L "${DATA_BIN}" ] || { [ -e "${DATA_BIN}" ] && [ ! -d "${DATA_BIN}" ]; }; then
  finish "${DATA_BIN} is not a plain directory; remove it to let Accelerator manage the terminal link"
fi
# shellcheck disable=SC2174 # deliberate: -m must apply to DATA_BIN only. If CLAUDE_PLUGIN_DATA itself must be created it is Claude Code's directory to own, so it takes the umask-derived mode.
mkdir -p -m 0700 "${DATA_BIN}" ||
  finish "could not create ${DATA_BIN}; a terminal 'accelerator' command may be stale"

# mv -f onto a symlink-to-directory writes *inside* it and reports success, so
# the temp-plus-rename dance needs this clear to stay correct.
if [ -L "${LINK}" ] && [ -d "${LINK}" ]; then
  rm -f "${LINK}" ||
    finish "could not clear ${LINK}; a terminal 'accelerator' command may be stale"
fi
if [ -e "${LINK}" ] && [ ! -L "${LINK}" ]; then
  NOTICE="${NOTICE}${NOTICE:+; }${LINK} exists and is not a symlink, so the terminal 'accelerator' command will not be updated. Remove it to let Accelerator manage the link."
  finish ""
fi

# ln -sfn onto a *real* directory succeeds, linking inside it, so a stale
# staging directory on a recycled pid would rename a directory onto LINK.
if [ -e "${STAGED}" ] || [ -L "${STAGED}" ]; then
  finish "stale staging path ${STAGED} is in the way; remove it and start a new session"
fi
trap 'rm -rf "${STAGED}" 2>/dev/null || true' EXIT

PREVIOUS=$(readlink "${LINK}" 2>/dev/null || true)
ln -sfn "${LAUNCHER}" "${STAGED}" ||
  finish "could not stage ${STAGED}; a terminal 'accelerator' command may be stale"
mv -f "${STAGED}" "${LINK}" ||
  finish "could not install ${LINK}; a terminal 'accelerator' command may be stale"
[ -L "${LINK}" ] ||
  finish "${LINK} is not a symlink after refresh; remove it and start a new session"

if [ -n "${PREVIOUS}" ] && [ "${PREVIOUS}" != "${LAUNCHER}" ]; then
  finish "terminal link re-pointed: ${PREVIOUS} -> ${LAUNCHER}"
fi
finish ""
