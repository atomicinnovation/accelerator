#!/usr/bin/env bash
set -euo pipefail

# Test harness for ticket management companion scripts
# Run: bash skills/tickets/scripts/test-ticket-scripts.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Shared assertion helpers (assert_eq, assert_exit_code,
# assert_file_executable, assert_stderr_empty, test_summary) plus the
# PASS/FAIL counters. See scripts/test-helpers.sh for the exposed surface.
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

NEXT_NUMBER="$SCRIPT_DIR/ticket-next-number.sh"
READ_STATUS="$SCRIPT_DIR/ticket-read-status.sh"
READ_FIELD="$SCRIPT_DIR/ticket-read-field.sh"

# Temporary-directory scaffolding is local to this harness because
# setup_repo encodes the .git-marker requirement of find_repo_root; it is
# not in test-helpers.sh.
TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$repo_dir/.git"
  echo "$repo_dir"
}

# ============================================================
echo "=== ticket-next-number.sh ==="
echo ""

# Test 1: No meta/tickets/ directory → outputs 0001
echo "Test: No meta/tickets/ directory"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" 2>/dev/null)
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 2: Empty meta/tickets/ directory → outputs 0001
echo "Test: Empty meta/tickets/ directory"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 3: Directory with 0003-foo.md → outputs 0004
echo "Test: Directory with 0003-foo.md"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
touch "$REPO/meta/tickets/0003-foo.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0004" "0004" "$OUTPUT"

# Test 4: Directory with gaps (0001, 0005) → outputs 0006 (uses highest)
echo "Test: Directory with gaps (uses highest)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
touch "$REPO/meta/tickets/0001-first.md"
touch "$REPO/meta/tickets/0005-fifth.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0006" "0006" "$OUTPUT"

# Test 5: Directory with non-ticket files only (README.md) → outputs 0001
echo "Test: Directory with non-ticket files only"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
touch "$REPO/meta/tickets/README.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 6: Mixed ticket and non-ticket files → outputs next after highest ticket
echo "Test: Mixed ticket and non-ticket files"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
touch "$REPO/meta/tickets/0002-something.md"
touch "$REPO/meta/tickets/README.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0003" "0003" "$OUTPUT"

# Test 7: --count 3 with highest 0002 → outputs 0003, 0004, 0005
echo "Test: --count 3 with highest 0002"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
touch "$REPO/meta/tickets/0002-something.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" --count 3)
EXPECTED=$(printf "0003\n0004\n0005")
assert_eq "outputs 0003, 0004, 0005" "$EXPECTED" "$OUTPUT"

# Test 8: --count 0 (invalid) → exits 1
echo "Test: --count 0 (invalid)"
assert_exit_code "exits 1" 1 bash "$NEXT_NUMBER" --count 0

# Test 9: --count abc (invalid) → exits 1
echo "Test: --count abc (invalid)"
assert_exit_code "exits 1" 1 bash "$NEXT_NUMBER" --count abc

# Test 10: Highest 9999 → exits 1 with "ticket number space exhausted" error
echo "Test: Highest 9999 (space exhausted)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
touch "$REPO/meta/tickets/9999-last.md"
RC=0
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" 2>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_eq "no stdout output" "" "$OUTPUT"

# Test 11: Files with 5-digit prefix (00003-foo.md) → glob does not match, outputs 0001
echo "Test: 5-digit prefix files ignored"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
touch "$REPO/meta/tickets/00003-foo.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 12: Existing ADR-style files mixed in → ignored, outputs 0001
echo "Test: ADR-style files ignored"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
touch "$REPO/meta/tickets/ADR-0003-something.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 13: --count with no value → exits 1
echo "Test: --count with no value"
assert_exit_code "exits 1" 1 bash "$NEXT_NUMBER" --count

# Test 14: Highest 9998 with --count 2 → outputs 9999 only and exits 1
echo "Test: Highest 9998 with --count 2 (partial overflow)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
touch "$REPO/meta/tickets/9998-second-to-last.md"
RC=0
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" --count 2 2>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_eq "outputs 9999 only" "9999" "$OUTPUT"

# Test 15: Filename without hyphen (0001.md) → glob does not match, outputs 0001
echo "Test: Filename without hyphen ignored"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
touch "$REPO/meta/tickets/0001.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

echo ""

# ============================================================
echo "=== ticket-read-status.sh ==="
echo ""

# Test 1: Valid frontmatter status: draft → outputs "draft"
echo "Test: Valid frontmatter status: draft"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
cat > "$REPO/meta/tickets/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
status: draft
---

# 0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs draft" "draft" "$OUTPUT"

# Test 2: Valid frontmatter status: ready → outputs "ready"
echo "Test: Valid frontmatter status: ready"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
cat > "$REPO/meta/tickets/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
status: ready
---

# 0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs ready" "ready" "$OUTPUT"

# Test 3: Quoted value status: "draft" → outputs "draft" (strips quotes)
echo "Test: Quoted value status: \"draft\""
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
cat > "$REPO/meta/tickets/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
status: "draft"
---

# 0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs draft (strips quotes)" "draft" "$OUTPUT"

# Test 4: No space after colon status:draft → outputs "draft"
echo "Test: No space after colon"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
cat > "$REPO/meta/tickets/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
status:draft
---

# 0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs draft" "draft" "$OUTPUT"

# Test 5: Trailing whitespace → outputs "draft" (stripped)
echo "Test: Trailing whitespace"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
printf -- '---\nticket_id: 0001\nstatus: draft  \n---\n\n# 0001: Test\n' \
  > "$REPO/meta/tickets/0001-test.md"
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs draft (stripped)" "draft" "$OUTPUT"

# Test 6: Missing file → exits 1
echo "Test: Missing file"
assert_exit_code "exits 1" 1 bash "$READ_STATUS" "/nonexistent/file.md"

# Test 7: File with no frontmatter → exits 1
echo "Test: File with no frontmatter"
REPO=$(setup_repo)
cat > "$REPO/no-frontmatter.md" << 'FIXTURE'
# Just a regular file

No frontmatter here.
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_STATUS" "$REPO/no-frontmatter.md"

# Test 8: Unclosed frontmatter → exits 1
echo "Test: Unclosed frontmatter"
REPO=$(setup_repo)
cat > "$REPO/unclosed.md" << 'FIXTURE'
---
ticket_id: 0001
status: draft
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_STATUS" "$REPO/unclosed.md"

# Test 9: Status in body ignored, frontmatter value returned
echo "Test: Status in body ignored, frontmatter value returned"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
cat > "$REPO/meta/tickets/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
status: draft
---

# 0001: Test

status: ready
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs draft (ignores body)" "draft" "$OUTPUT"

# Test 10: Empty status value → outputs empty string
echo "Test: Empty status value"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
printf -- '---\nticket_id: 0001\nstatus: \n---\n\n# 0001: Test\n' \
  > "$REPO/meta/tickets/0001-test.md"
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs empty string" "" "$OUTPUT"

# Test 11: No arguments → exits 1
echo "Test: No arguments"
assert_exit_code "exits 1" 1 bash "$READ_STATUS"

echo ""

# ============================================================
echo "=== ticket-read-field.sh ==="
echo ""

# Helper: create a standard ticket fixture in a temp repo
make_ticket() {
  local repo="$1"
  mkdir -p "$repo/meta/tickets"
  cat > "$repo/meta/tickets/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
type: story
priority: high
status: draft
parent: "0001"
tags: [backend, performance]
sub.type: foo
---

# 0001: Test Ticket

type: epic
FIXTURE
}

# Test 1: Read type field → outputs "story"
echo "Test: Read type field"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" type "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs story" "story" "$OUTPUT"

# Test 2: Read priority field → outputs "high"
echo "Test: Read priority field"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" priority "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs high" "high" "$OUTPUT"

# Test 3: Read status field → outputs "draft" (works same as read-status)
echo "Test: Read status field"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" status "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs draft" "draft" "$OUTPUT"

# Test 4: Read parent field → outputs "0001"
echo "Test: Read parent field"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" parent "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 5: Read missing field → exits 1 with error
echo "Test: Read missing field"
REPO=$(setup_repo)
make_ticket "$REPO"
assert_exit_code "exits 1" 1 bash "$READ_FIELD" nonexistent "$REPO/meta/tickets/0001-test.md"

# Test 6: Quoted field value → strips quotes
echo "Test: Quoted field value strips quotes"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" parent "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs 0001 (no quotes)" "0001" "$OUTPUT"

# Test 7: Field with array value tags: [a, b] → outputs "[backend, performance]" verbatim
echo "Test: Array field value returned verbatim"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" tags "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs array verbatim" "[backend, performance]" "$OUTPUT"

# Test 8: Missing file → exits 1
echo "Test: Missing file"
assert_exit_code "exits 1" 1 bash "$READ_FIELD" status "/nonexistent/file.md"

# Test 9: No frontmatter (first line is not ---) → exits 1 with error
echo "Test: No frontmatter"
REPO=$(setup_repo)
cat > "$REPO/no-fm.md" << 'FIXTURE'
# Just markdown

status: draft
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_FIELD" status "$REPO/no-fm.md"

# Test 10: Unclosed frontmatter → exits 1
echo "Test: Unclosed frontmatter"
REPO=$(setup_repo)
cat > "$REPO/unclosed.md" << 'FIXTURE'
---
status: draft
type: story
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_FIELD" status "$REPO/unclosed.md"

# Test 11: No arguments → exits 1
echo "Test: No arguments"
assert_exit_code "exits 1" 1 bash "$READ_FIELD"

# Test 12: One argument (file only, no field name) → exits 1
echo "Test: One argument only"
assert_exit_code "exits 1" 1 bash "$READ_FIELD" status

# Test 13: Field name in body ignored, frontmatter value returned
echo "Test: Body field ignored"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" type "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs story (not epic from body)" "story" "$OUTPUT"

# Test 14: Duplicate key → first-match-wins
echo "Test: Duplicate key first-match-wins"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
cat > "$REPO/meta/tickets/0001-dup.md" << 'FIXTURE'
---
status: first
status: second
---
FIXTURE
OUTPUT=$(bash "$READ_FIELD" status "$REPO/meta/tickets/0001-dup.md")
assert_eq "returns first occurrence" "first" "$OUTPUT"

# Test 15: Prefix-collision (query `tag`, fixture has only `tags:`) → exits 1
echo "Test: Prefix collision does not match"
REPO=$(setup_repo)
make_ticket "$REPO"
assert_exit_code "exits 1" 1 bash "$READ_FIELD" tag "$REPO/meta/tickets/0001-test.md"

# Test 16a: Literal-match — fixture has `sub.type: foo`, query `sub.type` → outputs "foo"
echo "Test: Dots matched literally (positive)"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" "sub.type" "$REPO/meta/tickets/0001-test.md")
assert_eq "outputs foo" "foo" "$OUTPUT"

# Test 16b: Negative-match — fixture has `subXtype: foo`, query `sub.type` → exits 1
echo "Test: Dots not treated as regex wildcard (negative)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
cat > "$REPO/meta/tickets/0001-nodot.md" << 'FIXTURE'
---
subXtype: foo
---
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_FIELD" "sub.type" "$REPO/meta/tickets/0001-nodot.md"

# Test 17: Value with trailing whitespace after closing quote → outputs "draft" (no orphan quote)
echo "Test: Trailing whitespace after closing quote"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/tickets"
printf -- '---\nstatus: "draft"  \n---\n' > "$REPO/meta/tickets/0001-trailing.md"
OUTPUT=$(bash "$READ_FIELD" status "$REPO/meta/tickets/0001-trailing.md")
assert_eq "outputs draft (no orphan quote)" "draft" "$OUTPUT"

echo ""

test_summary
