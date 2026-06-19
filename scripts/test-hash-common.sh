#!/usr/bin/env bash
set -euo pipefail

# Tests for scripts/hash-common.sh (portable SHA-256 helpers).
# Run: bash scripts/test-hash-common.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

# A fixed fixture and its independently-computed golden digest. The constant
# catches per-machine format/trim drift on whichever backend the host runs.
GOLDEN_INPUT="accelerator-hash-golden"
GOLDEN_SHA="99936098b2509b3b90cdaa2a3851af5ae62c231f69304ce2d7d3b35692d5c029"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

FIXTURE="$TMPDIR_BASE/fixture.txt"
printf '%s\n' "$GOLDEN_INPUT" >"$FIXTURE"

# Compute the golden digest under a forced backend, in a clean CHILD process so
# pre-setting _HASH_BIN cannot leak between cases (the env prefix exports it only
# to that child).
digest_file_with() {
  local bin="$1" file="$2"
  _HASH_BIN="$bin" bash -c \
    'source "$1/hash-common.sh"; hash_sha256_file "$2"' _ "$SCRIPT_DIR" "$file"
}
digest_stdin_with() {
  local bin="$1"
  _HASH_BIN="$bin" bash -c \
    'source "$1/hash-common.sh"; hash_sha256_stdin' _ "$SCRIPT_DIR"
}

# --- Default (auto-detected) backend ---------------------------------------
source "$SCRIPT_DIR/hash-common.sh"

echo "=== file and stdin agree, and match the golden digest ==="
FILE_SHA=$(hash_sha256_file "$FIXTURE")
STDIN_SHA=$(printf '%s\n' "$GOLDEN_INPUT" | hash_sha256_stdin)
assert_eq "hash_sha256_file matches golden" "$GOLDEN_SHA" "$FILE_SHA"
assert_eq "hash_sha256_stdin matches golden" "$GOLDEN_SHA" "$STDIN_SHA"
assert_eq "file and stdin agree" "$FILE_SHA" "$STDIN_SHA"
echo ""

# --- Both backends in one run ----------------------------------------------
# Exercise every backend the host actually has, so the non-default branch is not
# left untested on a single-OS leg. A host has shasum (macOS, Linux) and may also
# have sha256sum (Linux); force each that exists.
echo "=== every available backend yields the golden digest ==="
if command -v shasum >/dev/null 2>&1; then
  assert_eq "forced shasum -a 256 (file)" "$GOLDEN_SHA" \
    "$(digest_file_with "shasum -a 256" "$FIXTURE")"
  assert_eq "forced shasum -a 256 (stdin)" "$GOLDEN_SHA" \
    "$(printf '%s\n' "$GOLDEN_INPUT" | digest_stdin_with "shasum -a 256")"
fi
if command -v sha256sum >/dev/null 2>&1; then
  assert_eq "forced sha256sum (file)" "$GOLDEN_SHA" \
    "$(digest_file_with "sha256sum" "$FIXTURE")"
  assert_eq "forced sha256sum (stdin)" "$GOLDEN_SHA" \
    "$(printf '%s\n' "$GOLDEN_INPUT" | digest_stdin_with "sha256sum")"
fi
echo ""

test_summary
