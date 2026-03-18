#!/usr/bin/env bash
set -euo pipefail

# Test harness for adr-next-number.sh and adr-read-status.sh
# Run: bash skills/decisions/scripts/test-adr-scripts.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NEXT_NUMBER="$SCRIPT_DIR/adr-next-number.sh"
READ_STATUS="$SCRIPT_DIR/adr-read-status.sh"

PASS=0
FAIL=0

assert_eq() {
  local test_name="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected: $(printf '%q' "$expected")"
    echo "    Actual:   $(printf '%q' "$actual")"
    FAIL=$((FAIL + 1))
  fi
}

assert_exit_code() {
  local test_name="$1" expected_code="$2"
  shift 2
  local actual_code=0
  "$@" >/dev/null 2>&1 || actual_code=$?
  if [ "$expected_code" -eq "$actual_code" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected exit code: $expected_code"
    echo "    Actual exit code:   $actual_code"
    FAIL=$((FAIL + 1))
  fi
}

# Create a temporary directory to simulate repo environments
TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# Helper: create a fake repo with optional ADR files
# Usage: setup_repo [ADR-NNNN-name.md ...]
setup_repo() {
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  # Create a .git dir so find_repo_root works
  mkdir -p "$repo_dir/.git"
  echo "$repo_dir"
}

# ============================================================
echo "=== adr-next-number.sh ==="
echo ""

# Test 1: No meta/decisions/ directory
echo "Test: No meta/decisions/ directory"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 2: Empty meta/decisions/ directory
echo "Test: Empty meta/decisions/ directory"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 3: Directory with ADR-0003-foo.md
echo "Test: Directory with ADR-0003-foo.md"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
touch "$REPO/meta/decisions/ADR-0003-foo.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0004" "0004" "$OUTPUT"

# Test 4: Directory with gaps (ADR-0001, ADR-0005)
echo "Test: Directory with gaps (uses highest)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
touch "$REPO/meta/decisions/ADR-0001-first.md"
touch "$REPO/meta/decisions/ADR-0005-fifth.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0006" "0006" "$OUTPUT"

# Test 5: Directory with non-ADR files
echo "Test: Directory with non-ADR files only"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
touch "$REPO/meta/decisions/README.md"
touch "$REPO/meta/decisions/DRAFT-notes.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 6: Mixed ADR and non-ADR files
echo "Test: Mixed ADR and non-ADR files"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
touch "$REPO/meta/decisions/ADR-0002-something.md"
touch "$REPO/meta/decisions/README.md"
touch "$REPO/meta/decisions/DRAFT-notes.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0003" "0003" "$OUTPUT"

# Test 7: --count 3 with highest ADR-0002
echo "Test: --count 3 with highest ADR-0002"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
touch "$REPO/meta/decisions/ADR-0002-something.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" --count 3)
EXPECTED=$(printf "0003\n0004\n0005")
assert_eq "outputs 0003, 0004, 0005" "$EXPECTED" "$OUTPUT"

# Test 8: --count 0
echo "Test: --count 0 (invalid)"
REPO=$(setup_repo)
assert_exit_code "exits 1" 1 bash "$NEXT_NUMBER" --count 0

# Test 9: --count abc
echo "Test: --count abc (invalid)"
REPO=$(setup_repo)
assert_exit_code "exits 1" 1 bash "$NEXT_NUMBER" --count abc

# Test 10: ADR-9999 overflow
echo "Test: ADR-9999 overflow"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
touch "$REPO/meta/decisions/ADR-9999-overflow.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 10000" "10000" "$OUTPUT"

echo ""

# ============================================================
echo "=== adr-read-status.sh ==="
echo ""

# Test 1: Valid frontmatter status: proposed
echo "Test: Valid frontmatter status: proposed"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
cat > "$REPO/meta/decisions/ADR-0001-test.md" << 'FIXTURE'
---
adr_id: ADR-0001
status: proposed
---

# ADR-0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/decisions/ADR-0001-test.md")
assert_eq "outputs proposed" "proposed" "$OUTPUT"

# Test 2: Valid frontmatter status: accepted
echo "Test: Valid frontmatter status: accepted"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
cat > "$REPO/meta/decisions/ADR-0001-test.md" << 'FIXTURE'
---
adr_id: ADR-0001
status: accepted
---

# ADR-0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/decisions/ADR-0001-test.md")
assert_eq "outputs accepted" "accepted" "$OUTPUT"

# Test 3: Quoted value
echo "Test: Quoted value status: \"proposed\""
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
cat > "$REPO/meta/decisions/ADR-0001-test.md" << 'FIXTURE'
---
adr_id: ADR-0001
status: "proposed"
---

# ADR-0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/decisions/ADR-0001-test.md")
assert_eq "outputs proposed (strips quotes)" "proposed" "$OUTPUT"

# Test 4: No space after colon
echo "Test: No space status:proposed"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
cat > "$REPO/meta/decisions/ADR-0001-test.md" << 'FIXTURE'
---
adr_id: ADR-0001
status:proposed
---

# ADR-0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/decisions/ADR-0001-test.md")
assert_eq "outputs proposed" "proposed" "$OUTPUT"

# Test 5: Trailing whitespace
echo "Test: Trailing whitespace"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
printf -- '---\nadr_id: ADR-0001\nstatus: proposed  \n---\n\n# ADR-0001: Test\n' \
  > "$REPO/meta/decisions/ADR-0001-test.md"
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/decisions/ADR-0001-test.md")
assert_eq "outputs proposed (stripped)" "proposed" "$OUTPUT"

# Test 6: Missing file
echo "Test: Missing file"
assert_exit_code "exits 1" 1 bash "$READ_STATUS" "/nonexistent/file.md"

# Test 7: File with no frontmatter
echo "Test: File with no frontmatter"
REPO=$(setup_repo)
cat > "$REPO/no-frontmatter.md" << 'FIXTURE'
# Just a regular file

No frontmatter here.
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_STATUS" "$REPO/no-frontmatter.md"

# Test 8: Unclosed frontmatter (single ---)
echo "Test: Unclosed frontmatter"
REPO=$(setup_repo)
cat > "$REPO/unclosed.md" << 'FIXTURE'
---
adr_id: ADR-0001
status: proposed
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_STATUS" "$REPO/unclosed.md"

# Test 9: Status-like line in body (after frontmatter)
echo "Test: Status in body ignored, frontmatter value returned"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
cat > "$REPO/meta/decisions/ADR-0001-test.md" << 'FIXTURE'
---
adr_id: ADR-0001
status: proposed
---

# ADR-0001: Test

status: accepted
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/decisions/ADR-0001-test.md")
assert_eq "outputs proposed (ignores body)" "proposed" "$OUTPUT"

# Test 10: Empty status value
echo "Test: Empty status value"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/decisions"
printf -- '---\nadr_id: ADR-0001\nstatus: \n---\n\n# ADR-0001: Test\n' \
  > "$REPO/meta/decisions/ADR-0001-test.md"
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/decisions/ADR-0001-test.md")
assert_eq "outputs empty string" "" "$OUTPUT"

# Test 11: No arguments
echo "Test: No arguments"
assert_exit_code "exits 1" 1 bash "$READ_STATUS"

echo ""

# ============================================================
echo "=== Results ==="
echo "Passed: $PASS"
echo "Failed: $FAIL"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
echo "All tests passed!"
