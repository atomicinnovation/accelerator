#!/usr/bin/env bash
set -euo pipefail

# Unit tests for the wire-protocol helpers in interactive-protocol.sh.
# Run: bash scripts/test-interactive-protocol.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"
source "$SCRIPT_DIR/interactive-protocol.sh"

echo "=== escape_field / unescape_field round-trip ==="
echo ""

cases=(
  "plain ascii"
  "with TAB"$'\t'"in middle"
  "with newline"$'\n'"in middle"
  'with backslash \ in middle'
  'mixed \ '$'\t'$'\n'' chars'
  '\\'
  '\t'
  '\n'
  $'\t'
  $'\n'
  ''
  'long line: '$(printf 'A%.0s' {1..200})
)
for raw in "${cases[@]}"; do
  escaped=$(escape_field "$raw")
  # Escaped form must contain no literal TAB or newline.
  case "$escaped" in
    *$'\t'*)
      echo "  FAIL: escaped form contains TAB for input: $(printf '%q' "$raw")"
      FAIL=$((FAIL + 1)); continue ;;
    *$'\n'*)
      echo "  FAIL: escaped form contains newline for input: $(printf '%q' "$raw")"
      FAIL=$((FAIL + 1)); continue ;;
  esac
  # Use a sentinel suffix to defeat $(...)'s trailing-newline stripping:
  # both raw and round get the same suffix concatenated before comparison.
  round=$(unescape_field "$escaped"; printf X)
  round=${round%X}
  if [ "$raw" = "$round" ]; then
    echo "  PASS: round-trip $(printf '%q' "$raw")"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: round-trip $(printf '%q' "$raw")"
    echo "    Escaped: $(printf '%q' "$escaped")"
    echo "    Round:   $(printf '%q' "$round")"
    FAIL=$((FAIL + 1))
  fi
done

echo ""
echo "=== emit_frame / read_frame ==="
echo ""

# Emit then read back from a temp file.
TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

CAP="$TMPDIR_BASE/cap"
( emit_frame PROMPT "k1" "path/to/file" "10" 'value with TAB '$'\t' \
    'predicate' 'extras' 'b64payload' ) > "$CAP"
exec 9< "$CAP"
read_frame 9
exec 9<&-
assert_eq "frame type"   "PROMPT"       "$FRAME_TYPE"
assert_eq "field 0"      "k1"           "${FRAME_FIELDS[0]}"
assert_eq "field 1"      "path/to/file" "${FRAME_FIELDS[1]}"
assert_eq "field 2"      "10"           "${FRAME_FIELDS[2]}"
assert_eq "field 3 (TAB preserved)" $'value with TAB \t' "${FRAME_FIELDS[3]}"
assert_eq "field 4"      "predicate"    "${FRAME_FIELDS[4]}"
assert_eq "field 5"      "extras"       "${FRAME_FIELDS[5]}"
assert_eq "field 6"      "b64payload"   "${FRAME_FIELDS[6]}"

echo "Test: bare frame (no fields) round-trips"
CAP2="$TMPDIR_BASE/cap2"
emit_frame DONE > "$CAP2"
exec 9< "$CAP2"
read_frame 9
exec 9<&-
assert_eq "DONE frame type" "DONE" "$FRAME_TYPE"
assert_eq "DONE has zero fields" "0" "${#FRAME_FIELDS[@]}"

echo "Test: EOF on closed fd returns non-zero"
exec 9< /dev/null
RC=0
read_frame 9 || RC=$?
exec 9<&-
assert_neq "non-zero on EOF" "0" "$RC"

echo "Test: protocol log capture (test-only)"
LOGFILE="$TMPDIR_BASE/protolog"
INTERACTIVE_PROTOCOL_SIDE_LOG="$LOGFILE" emit_frame READY "$TMPDIR_BASE/log.jsonl" >/dev/null
LOGGED=$(cat "$LOGFILE")
assert_contains "log records READY" "$LOGGED" "READY"

echo ""
test_summary
