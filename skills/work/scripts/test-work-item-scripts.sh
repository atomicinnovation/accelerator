#!/usr/bin/env bash
set -euo pipefail

# Test harness for work item management companion scripts
# Run: bash skills/work/scripts/test-work-item-scripts.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Shared assertion helpers (assert_eq, assert_exit_code,
# assert_file_executable, assert_stderr_empty, test_summary) plus the
# PASS/FAIL counters. See scripts/test-helpers.sh for the exposed surface.
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

NEXT_NUMBER="$SCRIPT_DIR/work-item-next-number.sh"
READ_STATUS="$SCRIPT_DIR/work-item-read-status.sh"
READ_FIELD="$SCRIPT_DIR/work-item-read-field.sh"

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
echo "=== work-item-next-number.sh ==="
echo ""

# Test 1: No meta/work/ directory → outputs 0001
echo "Test: No meta/work/ directory"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" 2>/dev/null)
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 2: Empty meta/work/ directory → outputs 0001
echo "Test: Empty meta/work/ directory"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 3: Directory with 0003-foo.md → outputs 0004
echo "Test: Directory with 0003-foo.md"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0003-foo.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0004" "0004" "$OUTPUT"

# Test 4: Directory with gaps (0001, 0005) → outputs 0006 (uses highest)
echo "Test: Directory with gaps (uses highest)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0001-first.md"
touch "$REPO/meta/work/0005-fifth.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0006" "0006" "$OUTPUT"

# Test 5: Directory with non-ticket files only (README.md) → outputs 0001
echo "Test: Directory with non-ticket files only"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/README.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 6: Mixed ticket and non-ticket files → outputs next after highest ticket
echo "Test: Mixed ticket and non-ticket files"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0002-something.md"
touch "$REPO/meta/work/README.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0003" "0003" "$OUTPUT"

# Test 7: --count 3 with highest 0002 → outputs 0003, 0004, 0005
echo "Test: --count 3 with highest 0002"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0002-something.md"
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
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/9999-last.md"
RC=0
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" 2>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_eq "no stdout output" "" "$OUTPUT"

# Test 11: Files with 5-digit prefix (00003-foo.md) → glob does not match, outputs 0001
echo "Test: 5-digit prefix files ignored"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/00003-foo.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 12: Existing ADR-style files mixed in → ignored, outputs 0001
echo "Test: ADR-style files ignored"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/ADR-0003-something.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 13: --count with no value → exits 1
echo "Test: --count with no value"
assert_exit_code "exits 1" 1 bash "$NEXT_NUMBER" --count

# Test 14: Highest 9998 with --count 2 → outputs 9999 only and exits 1
echo "Test: Highest 9998 with --count 2 (partial overflow)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/9998-second-to-last.md"
RC=0
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER" --count 2 2>/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
assert_eq "outputs 9999 only" "9999" "$OUTPUT"

# Test 15: Filename without hyphen (0001.md) → glob does not match, outputs 0001
echo "Test: Filename without hyphen ignored"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
touch "$REPO/meta/work/0001.md"
OUTPUT=$(cd "$REPO" && bash "$NEXT_NUMBER")
assert_eq "outputs 0001" "0001" "$OUTPUT"

echo ""

# ============================================================
echo "=== work-item-read-status.sh ==="
echo ""

# Test 1: Valid frontmatter status: draft → outputs "draft"
echo "Test: Valid frontmatter status: draft"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat > "$REPO/meta/work/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
status: draft
---

# 0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs draft" "draft" "$OUTPUT"

# Test 2: Valid frontmatter status: ready → outputs "ready"
echo "Test: Valid frontmatter status: ready"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat > "$REPO/meta/work/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
status: ready
---

# 0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs ready" "ready" "$OUTPUT"

# Test 3: Quoted value status: "draft" → outputs "draft" (strips quotes)
echo "Test: Quoted value status: \"draft\""
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat > "$REPO/meta/work/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
status: "draft"
---

# 0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs draft (strips quotes)" "draft" "$OUTPUT"

# Test 4: No space after colon status:draft → outputs "draft"
echo "Test: No space after colon"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat > "$REPO/meta/work/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
status:draft
---

# 0001: Test
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs draft" "draft" "$OUTPUT"

# Test 5: Trailing whitespace → outputs "draft" (stripped)
echo "Test: Trailing whitespace"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
printf -- '---\nticket_id: 0001\nstatus: draft  \n---\n\n# 0001: Test\n' \
  > "$REPO/meta/work/0001-test.md"
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
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
mkdir -p "$REPO/meta/work"
cat > "$REPO/meta/work/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
status: draft
---

# 0001: Test

status: ready
FIXTURE
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs draft (ignores body)" "draft" "$OUTPUT"

# Test 10: Empty status value → outputs empty string
echo "Test: Empty status value"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
printf -- '---\nticket_id: 0001\nstatus: \n---\n\n# 0001: Test\n' \
  > "$REPO/meta/work/0001-test.md"
OUTPUT=$(bash "$READ_STATUS" "$REPO/meta/work/0001-test.md")
assert_eq "outputs empty string" "" "$OUTPUT"

# Test 11: No arguments → exits 1
echo "Test: No arguments"
assert_exit_code "exits 1" 1 bash "$READ_STATUS"

echo ""

# ============================================================
echo "=== work-item-read-field.sh ==="
echo ""

# Helper: create a standard ticket fixture in a temp repo
make_ticket() {
  local repo="$1"
  mkdir -p "$repo/meta/work"
  cat > "$repo/meta/work/0001-test.md" << 'FIXTURE'
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
OUTPUT=$(bash "$READ_FIELD" type "$REPO/meta/work/0001-test.md")
assert_eq "outputs story" "story" "$OUTPUT"

# Test 2: Read priority field → outputs "high"
echo "Test: Read priority field"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" priority "$REPO/meta/work/0001-test.md")
assert_eq "outputs high" "high" "$OUTPUT"

# Test 3: Read status field → outputs "draft" (works same as read-status)
echo "Test: Read status field"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" status "$REPO/meta/work/0001-test.md")
assert_eq "outputs draft" "draft" "$OUTPUT"

# Test 4: Read parent field → outputs "0001"
echo "Test: Read parent field"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" parent "$REPO/meta/work/0001-test.md")
assert_eq "outputs 0001" "0001" "$OUTPUT"

# Test 5: Read missing field → exits 1 with error
echo "Test: Read missing field"
REPO=$(setup_repo)
make_ticket "$REPO"
assert_exit_code "exits 1" 1 bash "$READ_FIELD" nonexistent "$REPO/meta/work/0001-test.md"

# Test 6: Quoted field value → strips quotes
echo "Test: Quoted field value strips quotes"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" parent "$REPO/meta/work/0001-test.md")
assert_eq "outputs 0001 (no quotes)" "0001" "$OUTPUT"

# Test 7: Field with array value tags: [a, b] → outputs "[backend, performance]" verbatim
echo "Test: Array field value returned verbatim"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" tags "$REPO/meta/work/0001-test.md")
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
OUTPUT=$(bash "$READ_FIELD" type "$REPO/meta/work/0001-test.md")
assert_eq "outputs story (not epic from body)" "story" "$OUTPUT"

# Test 14: Duplicate key → first-match-wins
echo "Test: Duplicate key first-match-wins"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat > "$REPO/meta/work/0001-dup.md" << 'FIXTURE'
---
status: first
status: second
---
FIXTURE
OUTPUT=$(bash "$READ_FIELD" status "$REPO/meta/work/0001-dup.md")
assert_eq "returns first occurrence" "first" "$OUTPUT"

# Test 15: Prefix-collision (query `tag`, fixture has only `tags:`) → exits 1
echo "Test: Prefix collision does not match"
REPO=$(setup_repo)
make_ticket "$REPO"
assert_exit_code "exits 1" 1 bash "$READ_FIELD" tag "$REPO/meta/work/0001-test.md"

# Test 16a: Literal-match — fixture has `sub.type: foo`, query `sub.type` → outputs "foo"
echo "Test: Dots matched literally (positive)"
REPO=$(setup_repo)
make_ticket "$REPO"
OUTPUT=$(bash "$READ_FIELD" "sub.type" "$REPO/meta/work/0001-test.md")
assert_eq "outputs foo" "foo" "$OUTPUT"

# Test 16b: Negative-match — fixture has `subXtype: foo`, query `sub.type` → exits 1
echo "Test: Dots not treated as regex wildcard (negative)"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat > "$REPO/meta/work/0001-nodot.md" << 'FIXTURE'
---
subXtype: foo
---
FIXTURE
assert_exit_code "exits 1" 1 bash "$READ_FIELD" "sub.type" "$REPO/meta/work/0001-nodot.md"

# Test 17: Value with trailing whitespace after closing quote → outputs "draft" (no orphan quote)
echo "Test: Trailing whitespace after closing quote"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
printf -- '---\nstatus: "draft"  \n---\n' > "$REPO/meta/work/0001-trailing.md"
OUTPUT=$(bash "$READ_FIELD" status "$REPO/meta/work/0001-trailing.md")
assert_eq "outputs draft (no orphan quote)" "draft" "$OUTPUT"

echo ""

# ============================================================
echo "=== work-item-update-tags.sh ==="
echo ""

UPDATE_TAGS="$SCRIPT_DIR/work-item-update-tags.sh"

# Helper: create a ticket with specific tags content
make_tagged_ticket() {
  local repo="$1"
  local tags_line="$2"
  mkdir -p "$repo/meta/work"
  cat > "$repo/meta/work/0001-test.md" << FIXTURE
---
ticket_id: 0001
status: draft
${tags_line}
---

# 0001: Test Ticket
FIXTURE
}

# Test 1: Add to existing flow-style array
echo "Test: Add to existing flow-style array"
REPO=$(setup_repo)
make_tagged_ticket "$REPO" "tags: [api, search]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add backend)
assert_eq "outputs new array" "[api, search, backend]" "$OUTPUT"

# Test 2: Add duplicate (no-change)
echo "Test: Add duplicate tag"
REPO=$(setup_repo)
make_tagged_ticket "$REPO" "tags: [api, search]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add api)
assert_eq "outputs no-change" "no-change" "$OUTPUT"

# Test 3: Remove existing tag
echo "Test: Remove existing tag"
REPO=$(setup_repo)
make_tagged_ticket "$REPO" "tags: [api, backend, search]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" remove backend)
assert_eq "outputs remaining tags" "[api, search]" "$OUTPUT"

# Test 4: Remove absent tag (no-change)
echo "Test: Remove absent tag"
REPO=$(setup_repo)
make_tagged_ticket "$REPO" "tags: [api, search]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" remove backend)
assert_eq "outputs no-change" "no-change" "$OUTPUT"

# Test 5: Remove last tag → []
echo "Test: Remove last tag yields empty array"
REPO=$(setup_repo)
make_tagged_ticket "$REPO" "tags: [backend]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" remove backend)
assert_eq "outputs empty array" "[]" "$OUTPUT"

# Test 6: Remove from empty [] → no-change
echo "Test: Remove from empty array"
REPO=$(setup_repo)
make_tagged_ticket "$REPO" "tags: []"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" remove backend)
assert_eq "outputs no-change" "no-change" "$OUTPUT"

# Test 7: Add to absent field → [new-tag]
echo "Test: Add to absent tags field"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat > "$REPO/meta/work/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
status: draft
---

# 0001: Test Ticket
FIXTURE
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add backend)
assert_eq "outputs single-element array" "[backend]" "$OUTPUT"

# Test 8: Add to empty [] → [new-tag]
echo "Test: Add to empty array"
REPO=$(setup_repo)
make_tagged_ticket "$REPO" "tags: []"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add backend)
assert_eq "outputs single-element array" "[backend]" "$OUTPUT"

# Test 9: Block-style detection → exit 1
echo "Test: Block-style tags rejected"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/work"
cat > "$REPO/meta/work/0001-test.md" << 'FIXTURE'
---
ticket_id: 0001
tags:
  - api
  - search
---

# 0001: Test Ticket
FIXTURE
RC=0
STDERR=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add backend 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
echo "$STDERR" | grep -q "block format" && echo "  PASS: stderr mentions block format" || { echo "  FAIL: stderr missing block format message"; FAIL=$((FAIL + 1)); }

# Test 10: Non-existent file → exit 1
echo "Test: Non-existent file"
RC=0
STDERR=$(bash "$UPDATE_TAGS" "/nonexistent/file.md" add backend 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"
echo "$STDERR" | grep -q "file not found" && echo "  PASS: stderr mentions file not found" || { echo "  FAIL: stderr missing file not found message"; FAIL=$((FAIL + 1)); }

# Test 11: Missing frontmatter → exit 1
echo "Test: Missing frontmatter"
REPO=$(setup_repo)
cat > "$REPO/no-fm.md" << 'FIXTURE'
# Just markdown

No frontmatter here.
FIXTURE
RC=0
STDERR=$(bash "$UPDATE_TAGS" "$REPO/no-fm.md" add backend 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"

# Test 12: Unclosed frontmatter → exit 1
echo "Test: Unclosed frontmatter"
REPO=$(setup_repo)
cat > "$REPO/unclosed.md" << 'FIXTURE'
---
ticket_id: 0001
tags: [api]
FIXTURE
RC=0
STDERR=$(bash "$UPDATE_TAGS" "$REPO/unclosed.md" add backend 2>&1 >/dev/null) || RC=$?
assert_eq "exit code 1" "1" "$RC"

# Test 13: Tag containing comma is quoted
echo "Test: Tag with comma is quoted"
REPO=$(setup_repo)
make_tagged_ticket "$REPO" "tags: [api]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add "one,two")
assert_eq "outputs quoted tag" '[api, "one,two"]' "$OUTPUT"

# Test 14: Tag containing colon is quoted
echo "Test: Tag with colon is quoted"
REPO=$(setup_repo)
make_tagged_ticket "$REPO" "tags: [api]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add "key:val")
assert_eq "outputs quoted tag" '[api, "key:val"]' "$OUTPUT"

# Test 15: Tag containing hash is quoted
echo "Test: Tag with hash is quoted"
REPO=$(setup_repo)
make_tagged_ticket "$REPO" "tags: [api]"
OUTPUT=$(bash "$UPDATE_TAGS" "$REPO/meta/work/0001-test.md" add "tag#1")
assert_eq "outputs quoted tag" '[api, "tag#1"]' "$OUTPUT"

echo ""

# ============================================================
echo "=== work-item-template-field-hints.sh ==="
echo ""

FIELD_HINTS="$SCRIPT_DIR/work-item-template-field-hints.sh"

# Test 1: Field with trailing comment → parsed values (status)
echo "Test: Status field parsed from template comment"
OUTPUT=$(bash "$FIELD_HINTS" status)
EXPECTED=$(printf "draft\nready\nin-progress\nreview\ndone\nblocked\nabandoned")
assert_eq "returns 7 status values" "$EXPECTED" "$OUTPUT"

# Test 2: Type field parsed from template comment
echo "Test: Type field parsed from template comment"
OUTPUT=$(bash "$FIELD_HINTS" type)
EXPECTED=$(printf "story\nepic\ntask\nbug\nspike")
assert_eq "returns 5 type values" "$EXPECTED" "$OUTPUT"

# Test 3: Priority field parsed from template comment
echo "Test: Priority field parsed from template comment"
OUTPUT=$(bash "$FIELD_HINTS" priority)
EXPECTED=$(printf "high\nmedium\nlow")
assert_eq "returns 3 priority values" "$EXPECTED" "$OUTPUT"

# Test 4: Unknown field with no comment → empty output
echo "Test: Unknown field returns empty output"
OUTPUT=$(bash "$FIELD_HINTS" nonexistent)
assert_eq "returns empty string" "" "$OUTPUT"

# Test 5: User-overridden template with custom values
echo "Test: User-overridden template with custom values"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
cat > "$REPO/meta/templates/ticket.md" << 'FIXTURE'
---
ticket_id: NNNN
status: open                                   # open | closed | wontfix
type: feature                                  # feature | defect
priority: p1                                   # p1 | p2 | p3 | p4
---

# NNNN: Title
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$FIELD_HINTS" status)
EXPECTED=$(printf "open\nclosed\nwontfix")
assert_eq "returns custom status values" "$EXPECTED" "$OUTPUT"

# Test 6: config-read-template failure → hardcoded fallback for known fields
echo "Test: Template read failure falls back to hardcoded defaults"
# Create a repo with no template at all and set PLUGIN_ROOT to a nonexistent
# plugin to force config-read-template.sh to fail
REPO=$(setup_repo)
# Override PLUGIN_ROOT to simulate failure — run in subshell with modified env
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="/nonexistent/plugin" bash "$FIELD_HINTS" status 2>/dev/null) || true
EXPECTED=$(printf "draft\nready\nin-progress\nreview\ndone\nblocked\nabandoned")
assert_eq "returns hardcoded status values" "$EXPECTED" "$OUTPUT"

# Test 7: Field with no trailing comment → hardcoded fallback
echo "Test: Field with no comment falls back to hardcoded"
REPO=$(setup_repo)
mkdir -p "$REPO/meta/templates"
cat > "$REPO/meta/templates/ticket.md" << 'FIXTURE'
---
ticket_id: NNNN
status: draft
type: story
priority: medium
---

# NNNN: Title
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$FIELD_HINTS" status)
EXPECTED=$(printf "draft\nready\nin-progress\nreview\ndone\nblocked\nabandoned")
assert_eq "returns hardcoded status fallback" "$EXPECTED" "$OUTPUT"

# Test 8: Tripwire — hardcoded fallback values match shipping template's comments
echo "Test: Tripwire — hardcoded fallbacks match shipping template"
# Parse shipping template status comment directly
SHIPPING_TEMPLATE="$PLUGIN_ROOT/templates/ticket.md"
STATUS_LINE=$(grep "^status:" "$SHIPPING_TEMPLATE")
STATUS_COMMENT="${STATUS_LINE#*#}"
SHIPPING_VALUES=""
while IFS= read -r token; do
  token=$(echo "$token" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  [ -n "$token" ] && SHIPPING_VALUES="${SHIPPING_VALUES}${SHIPPING_VALUES:+$'\n'}${token}"
done < <(echo "$STATUS_COMMENT" | tr '|' '\n')
HARDCODED_VALUES=$(cd /tmp && CLAUDE_PLUGIN_ROOT="/nonexistent" bash "$FIELD_HINTS" status 2>/dev/null) || true
assert_eq "hardcoded status matches shipping template" "$SHIPPING_VALUES" "$HARDCODED_VALUES"

TYPE_LINE=$(grep "^type:" "$SHIPPING_TEMPLATE")
TYPE_COMMENT="${TYPE_LINE#*#}"
SHIPPING_VALUES=""
while IFS= read -r token; do
  token=$(echo "$token" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  [ -n "$token" ] && SHIPPING_VALUES="${SHIPPING_VALUES}${SHIPPING_VALUES:+$'\n'}${token}"
done < <(echo "$TYPE_COMMENT" | tr '|' '\n')
HARDCODED_VALUES=$(cd /tmp && CLAUDE_PLUGIN_ROOT="/nonexistent" bash "$FIELD_HINTS" type 2>/dev/null) || true
assert_eq "hardcoded type matches shipping template" "$SHIPPING_VALUES" "$HARDCODED_VALUES"

PRIORITY_LINE=$(grep "^priority:" "$SHIPPING_TEMPLATE")
PRIORITY_COMMENT="${PRIORITY_LINE#*#}"
SHIPPING_VALUES=""
while IFS= read -r token; do
  token=$(echo "$token" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  [ -n "$token" ] && SHIPPING_VALUES="${SHIPPING_VALUES}${SHIPPING_VALUES:+$'\n'}${token}"
done < <(echo "$PRIORITY_COMMENT" | tr '|' '\n')
HARDCODED_VALUES=$(cd /tmp && CLAUDE_PLUGIN_ROOT="/nonexistent" bash "$FIELD_HINTS" priority 2>/dev/null) || true
assert_eq "hardcoded priority matches shipping template" "$SHIPPING_VALUES" "$HARDCODED_VALUES"

echo ""

test_summary
