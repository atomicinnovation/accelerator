#!/usr/bin/env bash
set -euo pipefail

# Test harness for the migration framework.
# Run: bash skills/config/migrate/scripts/test-migrate.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

DRIVER="$SCRIPT_DIR/run-migrations.sh"
MIGRATIONS_DIR="$SCRIPT_DIR/../migrations"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# ── Additional assert helpers ────────────────────────────────────────────────

assert_contains() {
  local name="$1" needle="$2" haystack="$3"
  if printf '%s' "$haystack" | grep -qF "$needle"; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name"
    echo "    Expected to contain: $needle"
    echo "    Actual: $haystack"
    FAIL=$((FAIL + 1))
  fi
}

assert_not_contains() {
  local name="$1" needle="$2" haystack="$3"
  if ! printf '%s' "$haystack" | grep -qF "$needle"; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name"
    echo "    Expected NOT to contain: $needle"
    echo "    Actual: $haystack"
    FAIL=$((FAIL + 1))
  fi
}

assert_file_exists() {
  local name="$1" path="$2"
  if [ -f "$path" ]; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name"
    echo "    Expected file: $path"
    FAIL=$((FAIL + 1))
  fi
}

assert_file_not_exists() {
  local name="$1" path="$2"
  if [ ! -f "$path" ]; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name"
    echo "    Expected file to not exist: $path"
    FAIL=$((FAIL + 1))
  fi
}

assert_dir_exists() {
  local name="$1" path="$2"
  if [ -d "$path" ]; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name"
    echo "    Expected directory: $path"
    FAIL=$((FAIL + 1))
  fi
}

assert_dir_not_exists() {
  local name="$1" path="$2"
  if [ ! -d "$path" ]; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name"
    echo "    Expected directory to not exist: $path"
    FAIL=$((FAIL + 1))
  fi
}

assert_neq() {
  local name="$1" unexpected="$2" actual="$3"
  if [ "$unexpected" != "$actual" ]; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name"
    echo "    Expected something other than: $(printf '%q' "$unexpected")"
    FAIL=$((FAIL + 1))
  fi
}

# ── setup_old_repo: temp dir with old ticket structure (no VCS dir) ──────────
# No .git or .jj means find_repo_root falls back to $PWD, and the
# VCS clean-tree check is skipped entirely.
setup_old_repo() {
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$repo_dir/meta/tickets"
  printf -- '---\nticket_id: 0001\n---\n\n# 0001: Foo\n' > "$repo_dir/meta/tickets/0001-foo.md"
  mkdir -p "$repo_dir/meta/reviews/tickets"
  printf -- '---\ntype: work-item-review\n---\n\n# foo-review-1\n' \
    > "$repo_dir/meta/reviews/tickets/foo-review-1.md"
  mkdir -p "$repo_dir/.claude"
  printf -- '---\npaths:\n  tickets: meta/tickets\n---\n' \
    > "$repo_dir/.claude/accelerator.md"
  echo "$repo_dir"
}

echo "=== run-migrations.sh ==="
echo ""

# ── Test 1: Apply pending migration succeeds ─────────────────────────────────
echo "Test: Apply pending migration succeeds"
REPO=$(setup_old_repo)
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_file_exists "meta/work/0001-foo.md created" "$REPO/meta/work/0001-foo.md"
assert_file_not_exists "meta/tickets/0001-foo.md removed" "$REPO/meta/tickets/0001-foo.md"
CONTENT=$(cat "$REPO/meta/work/0001-foo.md")
assert_contains "work_item_id in file" "work_item_id: 0001" "$CONTENT"
assert_not_contains "ticket_id absent" "ticket_id:" "$CONTENT"
assert_file_exists "meta/reviews/work/foo-review-1.md created" \
  "$REPO/meta/reviews/work/foo-review-1.md"
CONFIG=$(cat "$REPO/.claude/accelerator.md")
assert_contains "config has work key" "work: meta/work" "$CONFIG"
assert_not_contains "config has no tickets key" "tickets:" "$CONFIG"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_contains "state file has migration ID" "0001-rename-tickets-to-work" "$APPLIED"

echo ""

# ── Test 2: Re-running is idempotent ─────────────────────────────────────────
echo "Test: Re-running is idempotent"
REPO=$(setup_old_repo)
# First run
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
BEFORE_STATE=$(cat "$REPO/meta/.migrations-applied")
# Second run
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
AFTER_STATE=$(cat "$REPO/meta/.migrations-applied")
assert_eq "exit 0" "0" "$RC"
assert_contains "outputs no pending" "No pending migrations" "$OUTPUT"
assert_eq "state file unchanged" "$BEFORE_STATE" "$AFTER_STATE"

echo ""

# ── Test 3: Pre-populated state file skips migration ─────────────────────────
echo "Test: Pre-populated state file skips migration on first run"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta/work"
printf -- '---\nwork_item_id: 0001\n---\n\n# 0001: Foo\n' > "$REPO/meta/work/0001-foo.md"
mkdir -p "$REPO/.claude"
printf -- '---\npaths:\n  work: meta/work\n---\n' > "$REPO/.claude/accelerator.md"
printf '0001-rename-tickets-to-work\n' > "$REPO/meta/.migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "no pending output" "No pending migrations" "$OUTPUT"

echo ""

# ── Test 4: Failed migration aborts without updating state file ───────────────
echo "Test: Failed migration aborts without updating state file"
REPO=$(setup_old_repo)
# Pre-create meta/work as a regular file — mv meta/tickets/ meta/work will fail
printf 'blocking file\n' > "$REPO/meta/work"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1 || RC=$?
assert_neq "non-zero exit" "0" "$RC"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_not_contains "state file missing migration ID" "0001-rename-tickets-to-work" "$APPLIED"
# Step 2 (frontmatter rewrite) ran before step 4 failed — file has work_item_id: in meta/tickets/
assert_file_exists "meta/tickets/0001-foo.md still present" "$REPO/meta/tickets/0001-foo.md"
CONTENT=$(cat "$REPO/meta/tickets/0001-foo.md")
assert_contains "file has work_item_id (step 2 ran)" "work_item_id: 0001" "$CONTENT"

echo ""

# ── Test 5: Per-migration idempotency (direct invocation) ────────────────────
echo "Test: Per-migration idempotency (direct invocation)"
MIGRATION="$MIGRATIONS_DIR/0001-rename-tickets-to-work.sh"
REPO=$(setup_old_repo)
RC=0
PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$MIGRATION" > /dev/null 2>&1 || RC=$?
assert_eq "first run exit 0" "0" "$RC"
assert_dir_exists "meta/work created" "$REPO/meta/work"
assert_dir_not_exists "meta/tickets removed" "$REPO/meta/tickets"
# Second run
RC=0
PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$MIGRATION" > /dev/null 2>&1 || RC=$?
assert_eq "second run exit 0" "0" "$RC"
assert_dir_exists "meta/work still exists" "$REPO/meta/work"

echo ""

# ── Test 6: Empty repo (no meta/tickets/) is a no-op ─────────────────────────
echo "Test: Empty repo is a no-op (state file still written)"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_contains "state file has migration ID" "0001-rename-tickets-to-work" "$APPLIED"

echo ""

# ── Test 7: paths.tickets override is respected ──────────────────────────────
echo "Test: paths.tickets override is respected — pinned directory preserved"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta/custom-tix"
printf -- '---\nticket_id: 0001\n---\n\n# 0001: Foo\n' > "$REPO/meta/custom-tix/0001-foo.md"
mkdir -p "$REPO/.claude"
printf -- '---\npaths:\n  tickets: meta/custom-tix\n---\n' > "$REPO/.claude/accelerator.md"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_dir_exists "meta/custom-tix still exists" "$REPO/meta/custom-tix"
CONTENT=$(cat "$REPO/meta/custom-tix/0001-foo.md")
assert_contains "work_item_id in file" "work_item_id: 0001" "$CONTENT"
assert_not_contains "no ticket_id" "ticket_id:" "$CONTENT"
CONFIG=$(cat "$REPO/.claude/accelerator.md")
assert_contains "config key renamed to work" "work: meta/custom-tix" "$CONFIG"
assert_not_contains "config has no tickets key" "tickets:" "$CONFIG"
assert_dir_not_exists "meta/work not spuriously created" "$REPO/meta/work"

echo ""

# ── Test 8: Both default dirs exist — collision aborts cleanly ───────────────
echo "Test: Collision between meta/tickets/ and meta/work/ aborts cleanly"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta/tickets" "$REPO/meta/work"
printf -- '---\nticket_id: 0001\n---\n' > "$REPO/meta/tickets/0001-foo.md"
printf -- '---\nwork_item_id: 0002\n---\n' > "$REPO/meta/work/0002-bar.md"
mkdir -p "$REPO/.claude"
printf -- '---\n---\n' > "$REPO/.claude/accelerator.md"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit" "0" "$RC"
assert_contains "error mentions meta/tickets" "meta/tickets" "$OUTPUT"
assert_contains "error mentions meta/work" "meta/work" "$OUTPUT"
assert_dir_exists "meta/tickets still present" "$REPO/meta/tickets"
assert_dir_exists "meta/work still present" "$REPO/meta/work"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_not_contains "state file has no migration ID" "0001-rename-tickets-to-work" "$APPLIED"

echo ""

# ── Test 9: Malformed YAML in user config — refuse to migrate ────────────────
echo "Test: Malformed config aborts migration before any changes"
REPO=$(setup_old_repo)
# Overwrite with unclosed frontmatter
printf -- '---\npaths:\n  tickets: meta/tickets\n' > "$REPO/.claude/accelerator.md"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit" "0" "$RC"
assert_contains "error mentions config file" "accelerator.md" "$OUTPUT"
# meta/tickets/ should be untouched (step 2 never ran)
CONTENT=$(cat "$REPO/meta/tickets/0001-foo.md")
assert_contains "ticket_id unchanged" "ticket_id: 0001" "$CONTENT"

echo ""

# ── Test 10: Corrupt state file — preserve unknown IDs, warn ─────────────────
echo "Test: Unknown migration ID preserved and warned about"
REPO=$(setup_old_repo)
printf '0099-future-migration\n' > "$REPO/meta/.migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_contains "unknown ID preserved" "0099-future-migration" "$APPLIED"
assert_contains "new ID added" "0001-rename-tickets-to-work" "$APPLIED"
assert_contains "warning about unknown ID" "0099-future-migration" "$OUTPUT"

echo ""

# ── Test 11: Both ticket_id and work_item_id — no duplicate key ──────────────
echo "Test: Both ticket_id and work_item_id in frontmatter — no duplicate after migration"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta/tickets"
printf -- '---\nticket_id: 0001\nwork_item_id: 0001\n---\n\n# 0001: Foo\n' \
  > "$REPO/meta/tickets/0001-foo.md"
mkdir -p "$REPO/.claude"
printf -- '---\n---\n' > "$REPO/.claude/accelerator.md"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
RESULT_FILE="$REPO/meta/work/0001-foo.md"
CONTENT=$(cat "$RESULT_FILE" 2>/dev/null || echo "")
WI_COUNT=$(printf '%s\n' "$CONTENT" | grep -c '^work_item_id:' || true)
TI_COUNT=$(printf '%s\n' "$CONTENT" | grep -c '^ticket_id:' || true)
assert_eq "exactly one work_item_id" "1" "$WI_COUNT"
assert_eq "no ticket_id" "0" "$TI_COUNT"

echo ""

# ── Test 12: Paths with spaces — handled correctly ───────────────────────────
echo "Test: Filename with spaces handled correctly"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta/tickets"
printf -- '---\nticket_id: 0001\n---\n\n# 0001: With Space\n' \
  > "$REPO/meta/tickets/0001-with space.md"
mkdir -p "$REPO/.claude"
printf -- '---\n---\n' > "$REPO/.claude/accelerator.md"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_file_exists "file with space migrated" "$REPO/meta/work/0001-with space.md"
CONTENT=$(cat "$REPO/meta/work/0001-with space.md")
assert_contains "work_item_id in spaced file" "work_item_id: 0001" "$CONTENT"

echo ""

# ── Test 13: Driver — two pending migrations applied in order ─────────────────
echo "Test: Driver applies two pending migrations in order"
REPO=$(setup_old_repo)
FIXTURE_MIGRATIONS="$TMPDIR_BASE/fixture-migrations-$$"
mkdir -p "$FIXTURE_MIGRATIONS"
cp "$MIGRATIONS_DIR/0001-rename-tickets-to-work.sh" "$FIXTURE_MIGRATIONS/"
MARKER="$TMPDIR_BASE/0002-ran-$$"
cat > "$FIXTURE_MIGRATIONS/0002-noop.sh" << EOF
#!/usr/bin/env bash
# DESCRIPTION: No-op test migration
touch "$MARKER"
EOF
chmod +x "$FIXTURE_MIGRATIONS/0002-noop.sh"
RC=0
cd "$REPO" && \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATIONS_DIR="$FIXTURE_MIGRATIONS" \
  bash "$DRIVER" > /dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_contains "state file has 0001" "0001-rename-tickets-to-work" "$APPLIED"
assert_contains "state file has 0002" "0002-noop" "$APPLIED"
FIRST_LINE=$(head -1 "$REPO/meta/.migrations-applied")
assert_eq "0001 recorded first" "0001-rename-tickets-to-work" "$FIRST_LINE"
assert_file_exists "0002 marker file created" "$MARKER"

echo ""

# ── Test 14: Driver — clean-tree pre-flight aborts on dirty repo ──────────────
echo "Test: Clean-tree check aborts on uncommitted changes in meta/"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta/tickets"
printf -- '---\nticket_id: 0001\n---\n\n# 0001: Foo\n' > "$REPO/meta/tickets/0001-foo.md"
mkdir -p "$REPO/.claude"
printf -- '---\n---\n' > "$REPO/.claude/accelerator.md"
# Initialise a real git repo and commit the file so it is tracked
git -C "$REPO" init -q
git -C "$REPO" -c user.email="test@test.com" -c user.name="Test" add .
git -C "$REPO" -c user.email="test@test.com" -c user.name="Test" commit -qm "initial"
# Modify the tracked file without committing — makes the tree dirty
printf '\n# extra line\n' >> "$REPO/meta/tickets/0001-foo.md"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit (dirty tree)" "0" "$RC"
assert_contains "error mentions dirty working tree" "dirty" "$OUTPUT"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_not_contains "no state entry on abort" "0001-rename-tickets-to-work" "$APPLIED"
# Re-run with ACCELERATOR_MIGRATE_FORCE=1 bypasses the check
RC=0
cd "$REPO" && \
  ACCELERATOR_MIGRATE_FORCE=1 CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" \
  > /dev/null 2>&1 || RC=$?
assert_eq "exit 0 with FORCE flag" "0" "$RC"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_contains "migration applied with FORCE" "0001-rename-tickets-to-work" "$APPLIED"

test_summary
