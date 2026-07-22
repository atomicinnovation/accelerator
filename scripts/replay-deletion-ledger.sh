#!/usr/bin/env bash
set -euo pipefail

# Deletion-ledger replay (0167 Phase 7).
#
# For every row of `meta/inventories/0167-deletion-ledger.md`, assert the named
# covering-gate test prefix resolves to at least one real `#[test]` in the
# final-state gate file — the gate that SURVIVES the Phase 7 deletion. This is
# stronger than `check-inventory.sh`'s "a row exists" check: it forces every
# deleted script's behaviour to be pinned by a test in a surviving file, so the
# removed `test-config.sh` cannot have been the only thing covering it.
#
# Known-positive floor: the resolved-row count must meet a floor, AND a built-in
# negative self-test proves a mis-named gate row (a prefix that resolves to no
# test, or a row naming a missing file) makes the replay fail — otherwise the
# replay could pass vacuously, which is the tautology it exists to prevent.
#
# `--no-tests` skips the cargo run (presence + floor only); by default the
# surviving config-read suite is also run so "present AND passing" is asserted,
# not just presence. Run once and commit the output as the replay artefact.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LEDGER="$REPO_ROOT/meta/inventories/0167-deletion-ledger.md"

RUN_TESTS=1
[ "${1:-}" = "--no-tests" ] && RUN_TESTS=0

# Every ledger row's final-state gate is one of these surviving files.
READ_TESTS="cli/launcher/tests/config_read.rs"
DRIFT_TEST="cli/config/src/catalogue.rs"

# The removal set has 20 rows; the replay must resolve at least this many.
ROW_FLOOR=20

fail() {
  echo "replay-deletion-ledger: $1" >&2
  exit 1
}

# The final-state gate file a row names: the surviving read-test module, or the
# catalogue drift test. Prints the repo-relative path, empty if unrecognised.
final_state_file() {
  case "$1" in
    *config_read.rs*) echo "$READ_TESTS" ;;
    *drift*) echo "$DRIFT_TEST" ;;
    *) echo "" ;;
  esac
}

# 0 if $1 (a `fn`-name prefix) resolves to at least one test fn in file $2.
resolves() {
  grep -qE "fn ${1}" "$REPO_ROOT/$2"
}

# Negative self-test: an absent prefix must NOT resolve, and a present one must;
# a missing final-state file must be caught. Proves the floor is not vacuous.
self_test() {
  if resolves "zz_absent_gate_" "$READ_TESTS"; then
    fail "self-test: a bogus prefix resolved — the replay would pass vacuously"
  fi
  if ! resolves "get_" "$READ_TESTS"; then
    fail "self-test: a known-good prefix did not resolve — resolver is broken"
  fi
  if [ -n "$(final_state_file 'no-such-gate.rs')" ]; then
    fail "self-test: an unrecognised final-state gate was accepted"
  fi
  echo "replay: self-test ok (bogus prefix rejected, known-good prefix resolves)"
}

replay_rows() {
  local line path prefix final_col file resolved=0
  while IFS= read -r line; do
    case "$line" in
      '| `'*) ;;
      *) continue ;;
    esac
    # First backtick-quoted cell is the removal-set path.
    # shellcheck disable=SC2016 # backticks are literal table syntax, not expansion
    path="$(printf '%s\n' "$line" | sed -E 's/^\| `([^`]*)`.*/\1/')"
    # The `<prefix>_*` token names the covering gate.
    # shellcheck disable=SC2016 # backticks are literal table syntax, not expansion
    prefix="$(printf '%s\n' "$line" |
      grep -oE '`[a-z_]+\*`' | head -1 | tr -d '`*')"
    # The final-state gate is the last populated cell (a trailing `|` leaves an
    # empty field after it, so it is the second-to-last field).
    final_col="$(printf '%s\n' "$line" | awk -F'|' '{print $(NF - 1)}')"
    file="$(final_state_file "$final_col")"

    [ -n "$prefix" ] || fail "row names no covering-gate prefix: $path"
    [ -n "$file" ] || fail "row $path names an unrecognised final-state gate"
    [ -f "$REPO_ROOT/$file" ] || fail "final-state gate file is absent: $file"
    if ! resolves "$prefix" "$file"; then
      fail "row $path: covering gate '${prefix}*' resolves to no test in $file \
— the surviving gate does not cover the deleted script"
    fi
    printf '  %-40s %-20s -> %s\n' "$path" "${prefix}*" "$file"
    resolved=$((resolved + 1))
  done <"$LEDGER"

  if [ "$resolved" -lt "$ROW_FLOOR" ]; then
    fail "resolved $resolved row(s), expected >= $ROW_FLOOR (floor breach)"
  fi
  echo "replay: $resolved ledger row(s) resolve to a surviving test"
}

echo "== 0167 deletion-ledger replay =="
[ -f "$LEDGER" ] || fail "missing ledger: $LEDGER"
self_test
replay_rows

if [ "$RUN_TESTS" -eq 1 ]; then
  echo "replay: running the surviving config-read suite to assert it passes..."
  if command -v cargo >/dev/null 2>&1; then
    (cd "$REPO_ROOT/cli" && cargo test -p accelerator --test config_read >/dev/null 2>&1) ||
      fail "the surviving config-read suite did not pass"
    echo "replay: config-read suite passed (present AND passing)"
  else
    echo "replay: cargo not on PATH — presence asserted, passing deferred to CI"
  fi
fi

echo "replay-deletion-ledger: OK"
