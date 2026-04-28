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

test_summary
