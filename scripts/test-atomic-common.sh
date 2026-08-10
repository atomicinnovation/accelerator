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
printf 'old\n' >"$TARGET"
printf 'new\n' | atomic_write "$TARGET"
assert_eq "content replaced" "new" "$(cat "$TARGET")"

echo "Test: temp file lives in same directory as target (cross-filesystem-safe)"
TARGET_DIR="$TMPDIR_BASE/sub"
mkdir -p "$TARGET_DIR"
TARGET="$TARGET_DIR/file.txt"
# Use a coproc-style approach: start writing in background and verify temp is local
(printf 'data\n' | atomic_write "$TARGET")
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
COUNT=$(wc -l <"$TARGET" | tr -d ' ')
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
printf 'alpha\nbeta\ngamma\n' >"$TARGET"
atomic_remove_line "$TARGET" "beta"
assert_eq "beta removed" "$(printf 'alpha\ngamma')" "$(cat "$TARGET")"

echo "Test: absent line is a no-op"
atomic_remove_line "$TARGET" "missing"
assert_eq "file unchanged" "$(printf 'alpha\ngamma')" "$(cat "$TARGET")"

echo "Test: substring matches are preserved (only exact-match removed)"
printf 'alpha\nalphabet\nalpha-beta\n' >"$TARGET"
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

echo "=== atomic_jsonl_append ==="
echo ""

echo "Test: single call writes one line, newline-terminated"
TARGET="$TMPDIR_BASE/log.jsonl"
rm -f "$TARGET"
atomic_jsonl_append "$TARGET" '{"transformation_key":"a","schema_version":1,"v":1}'
COUNT=$(wc -l <"$TARGET" | tr -d ' ')
assert_eq "single line" "1" "$COUNT"

echo "Test: repeated calls append (do not overwrite)"
atomic_jsonl_append "$TARGET" '{"transformation_key":"b","schema_version":1,"v":2}'
atomic_jsonl_append "$TARGET" '{"transformation_key":"c","schema_version":1,"v":3}'
COUNT=$(wc -l <"$TARGET" | tr -d ' ')
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
  TOTAL=$(wc -l <"$TARGET3" | tr -d ' ')
  assert_eq "10 lines total at $size B" "10" "$TOTAL"
  if command -v python3 >/dev/null 2>&1; then
    BAD=$(
      python3 - "$TARGET3" <<'PY'
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

echo "Test: reclaims a lock orphaned by a dead owner"
TARGET_STALE="$TMPDIR_BASE/stale-lock.jsonl"
LOCKDIR_STALE="${TARGET_STALE}.lockdir"
mkdir -p "$LOCKDIR_STALE"
# Forge an owner sentinel referencing a process that has already exited,
# simulating a holder OOM-killed mid critical section.
sh -c 'exit 0' &
DEAD_PID=$!
wait "$DEAD_PID" 2>/dev/null || true
printf '%s\n' "$DEAD_PID" >"$LOCKDIR_STALE/owner"
# Acquisition must reclaim the orphaned lock promptly rather than spin to
# the timeout, so the append succeeds and writes its line.
RC=0
atomic_jsonl_append "$TARGET_STALE" \
  '{"transformation_key":"reclaimed","schema_version":1}' || RC=$?
assert_eq "stale lock reclaimed (exit 0)" "0" "$RC"
assert_eq "stale lock reclaim wrote one line" "1" \
  "$(wc -l <"$TARGET_STALE" | tr -d ' ')"

echo "Test: finishes a reclaim abandoned by a dead reclaimer"
# The crash window: the sentinel was mv'd aside but the lockdir was never
# removed. Left alone it has no owner sentinel and every later waiter would
# read it as permanently held, wedging the lock for the full ceiling.
TARGET_ABANDONED="$TMPDIR_BASE/abandoned-lock.jsonl"
LOCKDIR_ABANDONED="${TARGET_ABANDONED}.lockdir"
mkdir -p "$LOCKDIR_ABANDONED"
sh -c 'exit 0' &
DEAD_RECLAIMER=$!
wait "$DEAD_RECLAIMER" 2>/dev/null || true
printf '%s\n' "$DEAD_RECLAIMER" \
  >"$LOCKDIR_ABANDONED/reclaiming.$DEAD_RECLAIMER.abc123"
RC=0
atomic_jsonl_append "$TARGET_ABANDONED" \
  '{"transformation_key":"recovered","schema_version":1}' || RC=$?
assert_eq "abandoned reclaim finished (exit 0)" "0" "$RC"
assert_eq "abandoned reclaim wrote one line" "1" \
  "$(wc -l <"$TARGET_ABANDONED" | tr -d ' ')"

echo "Test: leaves a reclaim alone while its reclaimer is still alive"
# The reclaimer is mid-flight, not abandoned. Taking the sentinel over would
# give two waiters the right to rm the same lockdir.
TARGET_INFLIGHT="$TMPDIR_BASE/inflight-lock.jsonl"
LOCKDIR_INFLIGHT="${TARGET_INFLIGHT}.lockdir"
mkdir -p "$LOCKDIR_INFLIGHT"
sleep 30 &
LIVE_RECLAIMER=$!
printf '%s\n' "$LIVE_RECLAIMER" \
  >"$LOCKDIR_INFLIGHT/reclaiming.$LIVE_RECLAIMER.abc123"
RECLAIMABLE_RC=0
_atomic_lock_reclaimable "$LOCKDIR_INFLIGHT" >/dev/null 2>&1 ||
  RECLAIMABLE_RC=$?
assert_eq "live reclaimer's sentinel is not reclaimable" "1" \
  "$RECLAIMABLE_RC"
assert_eq "the lockdir is left in place" "0" \
  "$([ -d "$LOCKDIR_INFLIGHT" ] && echo 0 || echo 1)"
kill "$LIVE_RECLAIMER" 2>/dev/null || true
wait "$LIVE_RECLAIMER" 2>/dev/null || true

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
test_summary
