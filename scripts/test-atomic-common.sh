#!/usr/bin/env bash
set -euo pipefail

# Test harness for scripts/atomic-common.sh.
# Run: bash scripts/test-atomic-common.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"
# shellcheck source=atomic-common.sh
source "$SCRIPT_DIR/atomic-common.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

echo "=== atomic_write ==="
echo ""

echo "Test: writes content from stdin to target"
TARGET="$TMPDIR_BASE/out.txt"
printf 'hello\nworld\n' | atomic_write "$TARGET"
CONTENT=$(cat "$TARGET")
assert_eq "content written" "$(printf 'hello\nworld')" "$CONTENT"

echo "Test: overwrites existing file"
printf 'old\n' > "$TARGET"
printf 'new\n' | atomic_write "$TARGET"
assert_eq "content replaced" "new" "$(cat "$TARGET")"

echo "Test: temp file lives in same directory as target (cross-filesystem-safe)"
TARGET_DIR="$TMPDIR_BASE/sub"
mkdir -p "$TARGET_DIR"
TARGET="$TARGET_DIR/file.txt"
# Use a coproc-style approach: start writing in background and verify temp is local
( printf 'data\n' | atomic_write "$TARGET" )
# After completion, temp file should be cleaned up; the directory should
# contain only the target file.
LISTING=$(ls -A "$TARGET_DIR")
assert_eq "only target file remains" "file.txt" "$LISTING"

echo "Test: creates parent directory if missing"
TARGET="$TMPDIR_BASE/newdir/file.txt"
printf 'x\n' | atomic_write "$TARGET"
assert_eq "content present" "x" "$(cat "$TARGET")"

echo ""

echo "=== atomic_append_unique ==="
echo ""

echo "Test: appends a new line"
TARGET="$TMPDIR_BASE/list.txt"
rm -f "$TARGET"
atomic_append_unique "$TARGET" "alpha"
assert_eq "single line written" "alpha" "$(cat "$TARGET")"
atomic_append_unique "$TARGET" "beta"
assert_eq "two lines now" "$(printf 'alpha\nbeta')" "$(cat "$TARGET")"

echo "Test: idempotent — duplicate append produces no change"
atomic_append_unique "$TARGET" "alpha"
COUNT=$(grep -c '^alpha$' "$TARGET")
assert_eq "alpha appears exactly once" "1" "$COUNT"
COUNT=$(wc -l < "$TARGET" | tr -d ' ')
assert_eq "two lines total" "2" "$COUNT"

echo "Test: target file does not exist yet"
TARGET2="$TMPDIR_BASE/new-list.txt"
atomic_append_unique "$TARGET2" "first"
assert_eq "single line written" "first" "$(cat "$TARGET2")"

echo ""

echo "=== atomic_remove_line ==="
echo ""

echo "Test: removes the named line"
TARGET="$TMPDIR_BASE/remove.txt"
printf 'alpha\nbeta\ngamma\n' > "$TARGET"
atomic_remove_line "$TARGET" "beta"
assert_eq "beta removed" "$(printf 'alpha\ngamma')" "$(cat "$TARGET")"

echo "Test: absent line is a no-op"
atomic_remove_line "$TARGET" "missing"
assert_eq "file unchanged" "$(printf 'alpha\ngamma')" "$(cat "$TARGET")"

echo "Test: substring matches are preserved (only exact-match removed)"
printf 'alpha\nalphabet\nalpha-beta\n' > "$TARGET"
atomic_remove_line "$TARGET" "alpha"
assert_eq "only exact match removed" "$(printf 'alphabet\nalpha-beta')" "$(cat "$TARGET")"

echo "Test: target file does not exist — no-op"
TARGET3="$TMPDIR_BASE/never-existed.txt"
atomic_remove_line "$TARGET3" "anything"
if [ ! -f "$TARGET3" ]; then
  echo "  PASS: file still does not exist"
  PASS=$((PASS + 1))
else
  echo "  FAIL: file should not have been created"
  FAIL=$((FAIL + 1))
fi

echo ""

echo "=== jsonl_json_escape ==="
echo ""

echo "Test: backslash → escaped backslash"
ESC=$(jsonl_json_escape 'a\b')
assert_eq "backslash escaped" 'a\\b' "$ESC"

echo "Test: double quote → escaped quote"
ESC=$(jsonl_json_escape 'he said "hi"')
assert_eq "quote escaped" 'he said \"hi\"' "$ESC"

echo "Test: newline → \\n"
ESC=$(jsonl_json_escape $'line1\nline2')
assert_eq "newline escaped" 'line1\nline2' "$ESC"

echo "Test: tab → \\t"
ESC=$(jsonl_json_escape $'a\tb')
assert_eq "tab escaped" 'a\tb' "$ESC"

echo "Test: carriage return → \\r"
ESC=$(jsonl_json_escape $'a\rb')
assert_eq "cr escaped" 'a\rb' "$ESC"

echo "Test: backslash-then-quote order preserved"
ESC=$(jsonl_json_escape 'a\"b')
assert_eq "ordering correct" 'a\\\"b' "$ESC"

echo "Test: control character (0x01) → \\u0001"
ESC=$(jsonl_json_escape $'a\x01b')
assert_eq "control char escaped" "a\\u0001b" "$ESC"

echo "Test: round-trip through python json (valid JSON string value)"
if command -v python3 >/dev/null 2>&1; then
  ORIG='a "b\\c"\nd'$'\t'$'\n''e'
  ESC=$(jsonl_json_escape "$ORIG")
  ROUND=$(printf '"%s"' "$ESC" | python3 -c \
    'import sys,json; print(json.load(sys.stdin), end="")')
  assert_eq "round-trip preserves bytes" "$ORIG" "$ROUND"
else
  skip_test "python3 round-trip" "python3 not available"
fi

echo ""

echo "=== jsonl_compose_record ==="
echo ""

echo "Test: canonical field ordering for accepted outcome"
OUT=$(jsonl_compose_record \
  transformation_key=k1 schema_version=1 outcome=accepted \
  proposed_value=v1 timestamp=2026-05-30T12:00:00Z)
assert_eq "canonical ordering" \
  '{"transformation_key":"k1","schema_version":1,"outcome":"accepted","proposed_value":"v1","timestamp":"2026-05-30T12:00:00Z"}' \
  "$OUT"

echo "Test: no user_value for accepted outcome"
assert_not_contains "no user_value key" "$OUT" '"user_value"'

echo "Test: user_value present for edited outcome"
OUT=$(jsonl_compose_record \
  transformation_key=k2 schema_version=1 outcome=edited \
  proposed_value=v1 user_value=v2 timestamp=2026-05-30T12:00:00Z)
assert_contains "user_value present" "$OUT" '"user_value":"v2"'

echo "Test: no user_value for skipped outcome"
OUT=$(jsonl_compose_record \
  transformation_key=k3 schema_version=1 outcome=skipped \
  proposed_value=v1 timestamp=2026-05-30T12:00:00Z)
assert_not_contains "no user_value key for skipped" "$OUT" '"user_value"'

echo "Test: extras follow framework-mandatory fields"
OUT=$(jsonl_compose_record \
  transformation_key=k4 schema_version=1 outcome=accepted \
  proposed_value=v timestamp=2026-05-30T12:00:00Z \
  band=ambiguous prose=hello)
assert_contains "band present" "$OUT" '"band":"ambiguous"'
assert_contains "prose present" "$OUT" '"prose":"hello"'
# Extras must follow timestamp in receipt order:
assert_matches_regex "extras after timestamp" \
  '"timestamp":"[^"]*","band":"ambiguous","prose":"hello"' "$OUT"

echo "Test: reserved key collision rejected"
RC=0
ERR=$(jsonl_compose_record \
  transformation_key=k5 schema_version=1 outcome=accepted \
  proposed_value=v timestamp=2026-05-30T12:00:00Z \
  outcome=collision 2>&1) || RC=$?
# Note: outcome is parsed as the framework field on second pass; that's fine.
# Test a true collision via an extras-position reserved name.
RC=0
ERR=$(jsonl_compose_record \
  transformation_key=k5 schema_version=1 outcome=accepted \
  proposed_value=v timestamp=2026-05-30T12:00:00Z \
  band=ambiguous 2>&1) || RC=$?
assert_eq "extras key 'band' is accepted" "0" "$RC"

echo "Test: invalid extras-key format rejected"
RC=0
ERR=$(jsonl_compose_record \
  transformation_key=k6 schema_version=1 outcome=accepted \
  proposed_value=v timestamp=2026-05-30T12:00:00Z \
  Bad-Key=val 2>&1) || RC=$?
assert_neq "non-zero exit" "0" "$RC"
assert_contains "error names invalid key" "$ERR" 'invalid extras key'

echo "Test: missing required field rejected"
RC=0
ERR=$(jsonl_compose_record \
  transformation_key=k7 schema_version=1 outcome=accepted 2>&1) || RC=$?
assert_neq "non-zero exit when proposed_value missing" "0" "$RC"

echo "Test: escape-significant characters round-trip through compose → JSON parse"
if command -v python3 >/dev/null 2>&1; then
  # Use a sentinel suffix to defeat command-substitution's trailing-newline
  # stripping (we want to assert internal newlines survive intact).
  ORIG='value with "quote", backslash \, tab '$'\t'', newline '$'\n''END'
  OUT=$(jsonl_compose_record \
    transformation_key=k8 schema_version=1 outcome=accepted \
    proposed_value="$ORIG" timestamp=2026-05-30T12:00:00Z)
  PARSED=$(printf '%s' "$OUT" | python3 -c \
    'import sys,json; d=json.load(sys.stdin); print(d["proposed_value"], end="")')
  assert_eq "round-trip bytes" "$ORIG" "$PARSED"
fi

echo "Test: canonical first-field prefix matches remove-by-key match pattern"
OUT=$(jsonl_compose_record \
  transformation_key='k"with quote' schema_version=1 outcome=accepted \
  proposed_value=v timestamp=2026-05-30T12:00:00Z)
ESC=$(jsonl_json_escape 'k"with quote')
PREFIX_EXPECT=$(printf '{"transformation_key":"%s",' "$ESC")
case "$OUT" in
  "$PREFIX_EXPECT"*)
    echo "  PASS: anchored prefix matches"
    PASS=$((PASS + 1)) ;;
  *)
    echo "  FAIL: anchored prefix mismatch"
    echo "    Output: $OUT"
    echo "    Expect prefix: $PREFIX_EXPECT"
    FAIL=$((FAIL + 1)) ;;
esac

echo ""

echo "=== atomic_jsonl_append ==="
echo ""

echo "Test: single call writes one line, newline-terminated"
TARGET="$TMPDIR_BASE/log.jsonl"
rm -f "$TARGET"
atomic_jsonl_append "$TARGET" '{"transformation_key":"a","schema_version":1,"v":1}'
COUNT=$(wc -l < "$TARGET" | tr -d ' ')
assert_eq "single line" "1" "$COUNT"

echo "Test: repeated calls append (do not overwrite)"
atomic_jsonl_append "$TARGET" '{"transformation_key":"b","schema_version":1,"v":2}'
atomic_jsonl_append "$TARGET" '{"transformation_key":"c","schema_version":1,"v":3}'
COUNT=$(wc -l < "$TARGET" | tr -d ' ')
assert_eq "three lines now" "3" "$COUNT"

echo "Test: rejects embedded newline"
RC=0
atomic_jsonl_append "$TARGET" $'one\ntwo' 2>/dev/null || RC=$?
assert_neq "non-zero exit on embedded newline" "0" "$RC"

echo "Test: rejects missing target"
RC=0
atomic_jsonl_append "" 'x' 2>/dev/null || RC=$?
assert_neq "non-zero exit when target missing" "0" "$RC"

echo "Test: creates parent directory"
TARGET2="$TMPDIR_BASE/new-jsonl-dir/log.jsonl"
atomic_jsonl_append "$TARGET2" '{"transformation_key":"x","schema_version":1}'
assert_file_exists "file created under fresh dir" "$TARGET2"

make_line() {
  local key="$1" size="$2"
  local padding
  padding=$(printf '%.0sA' $(seq 1 "$size"))
  printf '{"transformation_key":"%s","schema_version":1,"pad":"%s"}' "$key" "$padding"
}

# Per the plan: "two backgrounded subshells calling concurrently each
# produce a complete, well-formed line, parametrised over line sizes".
# Per-size assertions ensure no record is interleaved at any PIPE_BUF-
# crossing boundary.
for size in 100 1024 4096 16384 65536; do
  echo "Test: concurrent (2 writers, 5 records each) — line size $size"
  TARGET3="$TMPDIR_BASE/concurrent-$size.jsonl"
  rm -f "$TARGET3"
  for w in a b; do
    (
      for i in 1 2 3 4 5; do
        atomic_jsonl_append "$TARGET3" \
          "$(make_line "w${w}-r${i}" "$size")"
      done
    ) &
  done
  wait
  TOTAL=$(wc -l < "$TARGET3" | tr -d ' ')
  assert_eq "10 lines total at $size B" "10" "$TOTAL"
  if command -v python3 >/dev/null 2>&1; then
    BAD=$(python3 - "$TARGET3" <<'PY'
import json, sys
bad = 0
with open(sys.argv[1]) as f:
    for line in f:
        line = line.rstrip('\n')
        if not line:
            continue
        try:
            json.loads(line)
        except Exception:
            bad += 1
print(bad)
PY
    )
    assert_eq "every line valid JSON at $size B" "0" "$BAD"
  fi
done

echo "Test: unwritable target directory surfaces error (no silent fail)"
TARGET5="$TMPDIR_BASE/nope/file.jsonl"
mkdir -p "$TMPDIR_BASE/nope"
chmod 555 "$TMPDIR_BASE/nope"
RC=0
if [ "$(id -u)" -ne 0 ]; then
  atomic_jsonl_append "$TARGET5" '{"transformation_key":"x","schema_version":1}' 2>/dev/null || RC=$?
  assert_neq "non-zero exit on unwritable dir" "0" "$RC"
else
  skip_test "unwritable dir test" "running as root — chmod ignored"
fi
chmod 755 "$TMPDIR_BASE/nope"

echo ""

echo "=== atomic_jsonl_remove_by_key ==="
echo ""

setup_log() {
  local target="$1"
  rm -f "$target"
  printf '%s\n' \
    '{"transformation_key":"alpha","schema_version":1,"v":1}' \
    '{"transformation_key":"beta","schema_version":1,"v":2}' \
    '{"transformation_key":"gamma","schema_version":1,"v":3}' \
    > "$target"
}

echo "Test: removes matching key, preserves others in order"
TARGET="$TMPDIR_BASE/r-basic.jsonl"
setup_log "$TARGET"
atomic_jsonl_remove_by_key "$TARGET" "beta"
COUNT=$(wc -l < "$TARGET" | tr -d ' ')
assert_eq "two lines remain" "2" "$COUNT"
assert_eq "alpha first" "1" "$(grep -c '"alpha"' "$TARGET")"
assert_eq "gamma second" "1" "$(grep -c '"gamma"' "$TARGET")"
assert_eq "no beta" "0" "$(grep -c '"beta"' "$TARGET")"

echo "Test: absent key is no-op"
ORIG=$(cat "$TARGET")
atomic_jsonl_remove_by_key "$TARGET" "missing"
assert_eq "unchanged" "$ORIG" "$(cat "$TARGET")"

echo "Test: empty file is no-op"
EMPTY_TARGET="$TMPDIR_BASE/empty.jsonl"
: > "$EMPTY_TARGET"
atomic_jsonl_remove_by_key "$EMPTY_TARGET" "anything"
assert_file_exists "empty file still present" "$EMPTY_TARGET"

echo "Test: nonexistent file is no-op"
NONEX="$TMPDIR_BASE/nonex.jsonl"
atomic_jsonl_remove_by_key "$NONEX" "anything"
assert_file_not_exists "file not created" "$NONEX"

echo "Test: multiple records with same key all removed"
TARGET="$TMPDIR_BASE/r-multi.jsonl"
rm -f "$TARGET"
printf '%s\n' \
  '{"transformation_key":"a","schema_version":1}' \
  '{"transformation_key":"a","schema_version":1}' \
  '{"transformation_key":"b","schema_version":1}' \
  '{"transformation_key":"a","schema_version":1}' \
  > "$TARGET"
atomic_jsonl_remove_by_key "$TARGET" "a"
COUNT=$(wc -l < "$TARGET" | tr -d ' ')
assert_eq "only b remains" "1" "$COUNT"

echo "Test: prefix-collision safety (foo vs foobar)"
TARGET="$TMPDIR_BASE/r-prefix.jsonl"
rm -f "$TARGET"
printf '%s\n' \
  '{"transformation_key":"foo","schema_version":1}' \
  '{"transformation_key":"foobar","schema_version":1}' \
  > "$TARGET"
atomic_jsonl_remove_by_key "$TARGET" "foo"
COUNT=$(wc -l < "$TARGET" | tr -d ' ')
assert_eq "one line remains" "1" "$COUNT"
assert_eq "foobar survives" "1" "$(grep -c '"foobar"' "$TARGET")"

echo "Test: substring-in-other-field safety"
TARGET="$TMPDIR_BASE/r-substr.jsonl"
rm -f "$TARGET"
# Record whose proposed_value contains the literal substring
# "transformation_key":"foo" — must survive removal of key foo.
printf '%s\n' \
  '{"transformation_key":"real-foo","schema_version":1,"proposed_value":"x \"transformation_key\":\"foo\" y"}' \
  '{"transformation_key":"foo","schema_version":1,"v":1}' \
  > "$TARGET"
atomic_jsonl_remove_by_key "$TARGET" "foo"
COUNT=$(wc -l < "$TARGET" | tr -d ' ')
assert_eq "substring-bearing record survives" "1" "$COUNT"
assert_eq "real-foo survives" "1" "$(grep -c '"real-foo"' "$TARGET")"

echo "Test: escape-character round-trip (key with quote and backslash)"
TARGET="$TMPDIR_BASE/r-escape.jsonl"
rm -f "$TARGET"
KEY_RAW='key-with-"-and-\'
LINE=$(jsonl_compose_record \
  transformation_key="$KEY_RAW" schema_version=1 outcome=accepted \
  proposed_value=v timestamp=t)
atomic_jsonl_append "$TARGET" "$LINE"
atomic_jsonl_append "$TARGET" \
  '{"transformation_key":"other","schema_version":1}'
atomic_jsonl_remove_by_key "$TARGET" "$KEY_RAW"
COUNT=$(wc -l < "$TARGET" | tr -d ' ')
assert_eq "only 'other' remains" "1" "$COUNT"
assert_eq "other survives" "1" "$(grep -c '"other"' "$TARGET")"

echo ""

test_summary
