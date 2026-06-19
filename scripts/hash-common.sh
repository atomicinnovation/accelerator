#!/usr/bin/env bash
# hash-common.sh — portable SHA-256 helpers (general-purpose, repo-root library).
#
# Sourced, never executed. Sits beside the other shared libraries
# (atomic-common.sh, config-common.sh, vcs-common.sh) and owns the ONE portable
# full-digest sha256 idiom for the whole repo, so callers compose it rather than
# re-copying `sha256sum || shasum -a 256` per script.
#
#   hash_sha256_file <file>   # full hex digest of a file's bytes
#   hash_sha256_stdin         # full hex digest of stdin
#
# The backend is chosen by DETECTION (command -v), matching the existing
# launcher idiom — not an exit-status `||` fallback — so behaviour is identical,
# not merely digest-equal. `_HASH_BIN` is honoured if pre-set (a test seam that
# forces a specific backend on a host that has both), otherwise auto-detected.

if [ -z "${_HASH_BIN:-}" ]; then
  if command -v sha256sum >/dev/null 2>&1; then
    _HASH_BIN="sha256sum"
  else
    _HASH_BIN="shasum -a 256"
  fi
fi

# $_HASH_BIN is INTENTIONALLY word-split into command + flags.
# shellcheck disable=SC2086
hash_sha256_file() { $_HASH_BIN "$1" | awk '{print $1}'; }
# shellcheck disable=SC2086
hash_sha256_stdin() { $_HASH_BIN | awk '{print $1}'; }
