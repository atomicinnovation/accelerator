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

# A migrations directory containing only 0001, used by tests that
# don't want 0002 interfering with "No pending" assertions.
ONLY_0001_DIR="$TMPDIR_BASE/only-0001-migrations"
mkdir -p "$ONLY_0001_DIR"
cp "$MIGRATIONS_DIR/0001-rename-tickets-to-work.sh" "$ONLY_0001_DIR/"

# Hash every file under $1 (optionally filtered by find args in $2..) and emit
# a single digest of the combined per-file digests. Portable across macOS
# (md5 -q) and Linux (md5sum).
tree_hash() {
  local root="$1"
  shift
  if command -v md5sum >/dev/null 2>&1; then
    find "$root" -type f "$@" -exec md5sum {} \; | awk '{print $1}' | sort | md5sum | awk '{print $1}'
  else
    find "$root" -type f "$@" -exec md5 -q {} \; | sort | md5 -q
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
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" > /dev/null 2>&1
BEFORE_STATE=$(cat "$REPO/meta/.migrations-applied")
# Second run
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
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
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
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

# ============================================================
echo ""
echo "=== --skip / --unskip flags ==="
echo ""

echo "Test: --skip records the ID in .migrations-skipped"
REPO=$(setup_old_repo)
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 0001-rename-tickets-to-work \
  > /dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
SKIPPED=$(cat "$REPO/meta/.migrations-skipped" 2>/dev/null || echo "")
assert_contains "skip file has migration ID" "0001-rename-tickets-to-work" "$SKIPPED"

echo "Test: subsequent run reports no pending migrations"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "outputs no pending" "No pending migrations" "$OUTPUT"
assert_contains "summary lists skipped name" "Skipped:" "$OUTPUT"
assert_contains "skipped name visible" "0001-rename-tickets-to-work" "$OUTPUT"
# Migration must NOT have run
assert_file_exists "meta/tickets/0001-foo.md still present" "$REPO/meta/tickets/0001-foo.md"

echo "Test: --unskip removes the ID and migration becomes pending again"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --unskip 0001-rename-tickets-to-work \
  > /dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
SKIPPED=$(cat "$REPO/meta/.migrations-skipped" 2>/dev/null || echo "")
assert_not_contains "skip file no longer has ID" "0001-rename-tickets-to-work" "$SKIPPED"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_contains "migration applied after unskip" "0001-rename-tickets-to-work" "$APPLIED"

echo "Test: --skip is idempotent"
REPO=$(setup_old_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 0001-rename-tickets-to-work \
  > /dev/null 2>&1
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 0001-rename-tickets-to-work \
  > /dev/null 2>&1
COUNT=$(grep -c "^0001-rename-tickets-to-work$" "$REPO/meta/.migrations-skipped")
assert_eq "ID present exactly once" "1" "$COUNT"

echo "Test: --unskip on absent ID is a no-op"
REPO=$(setup_old_repo)
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --unskip 0001-rename-tickets-to-work \
  > /dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"

echo "Test: skipping unknown ID writes it and warns on next run"
REPO=$(setup_old_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 9999-future-migration \
  > /dev/null 2>&1
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "warning about unknown skipped ID" "9999-future-migration" "$OUTPUT"
SKIPPED=$(cat "$REPO/meta/.migrations-skipped" 2>/dev/null || echo "")
assert_contains "unknown skipped ID preserved" "9999-future-migration" "$SKIPPED"

echo "Test: ACCELERATOR_MIGRATE_FORCE bypasses dirty-tree only — skip still wins"
REPO=$(setup_old_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 0001-rename-tickets-to-work \
  > /dev/null 2>&1
git -C "$REPO" init -q
git -C "$REPO" -c user.email=t@t -c user.name=T add .
git -C "$REPO" -c user.email=t@t -c user.name=T commit -qm initial
printf '\nx\n' >> "$REPO/meta/tickets/0001-foo.md"
RC=0
OUTPUT=$(cd "$REPO" && ACCELERATOR_MIGRATE_FORCE=1 CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "no pending under FORCE+skip" "No pending migrations" "$OUTPUT"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_not_contains "skipped migration NOT applied under FORCE" "0001-rename-tickets-to-work" "$APPLIED"

echo "Test: applied + skipped same ID — applied wins, warning emitted"
REPO=$(setup_old_repo)
mkdir -p "$REPO/meta"
printf '0001-rename-tickets-to-work\n' > "$REPO/meta/.migrations-applied"
printf '0001-rename-tickets-to-work\n' > "$REPO/meta/.migrations-skipped"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "warns about cross-state inconsistency" "BOTH" "$OUTPUT"
assert_contains "no pending output (applied wins)" "No pending migrations" "$OUTPUT"

echo "Test: empty .migrations-skipped is treated as no-skip"
REPO=$(setup_old_repo)
mkdir -p "$REPO/meta"
: > "$REPO/meta/.migrations-skipped"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_contains "migration applied" "0001-rename-tickets-to-work" "$APPLIED"

# ============================================================
echo ""
echo "=== MIGRATION_RESULT contract ==="
echo ""

# Build a temporary migrations dir with a stub migration that emits the sentinel
echo "Test: migration emitting MIGRATION_RESULT: no_op_pending stays unapplied"
REPO=$(mktemp -d "$TMPDIR_BASE/no-op-XXXXXX")
mkdir -p "$REPO/.git" "$REPO/meta"
STUB_DIR=$(mktemp -d "$TMPDIR_BASE/stubmigs-XXXXXX")
cat > "$STUB_DIR/9001-stub-no-op.sh" << 'STUB'
#!/usr/bin/env bash
# DESCRIPTION: stub migration that defers via the no_op_pending sentinel
echo "MIGRATION_RESULT: no_op_pending"
exit 0
STUB
chmod +x "$STUB_DIR/9001-stub-no-op.sh"
RC=0
OUTPUT=$(cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$STUB_DIR" \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_not_contains "stub NOT recorded as applied" "9001-stub-no-op" "$APPLIED"
# Sentinel is stripped from the user-visible output
assert_not_contains "sentinel hidden from user" "MIGRATION_RESULT:" "$OUTPUT"

echo "Test: stub stays pending across re-runs"
RC=0
OUTPUT=$(cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$STUB_DIR" \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "stub still listed as about-to-apply" "9001-stub-no-op" "$OUTPUT"

echo "Test: 0-exit migration WITHOUT sentinel IS recorded"
STUB2_DIR=$(mktemp -d "$TMPDIR_BASE/stubmigs2-XXXXXX")
cat > "$STUB2_DIR/9002-stub-applied.sh" << 'STUB'
#!/usr/bin/env bash
# DESCRIPTION: stub migration that records as applied
echo "did some work"
exit 0
STUB
chmod +x "$STUB2_DIR/9002-stub-applied.sh"
REPO=$(mktemp -d "$TMPDIR_BASE/applied-XXXXXX")
mkdir -p "$REPO/.git" "$REPO/meta"
RC=0
cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$STUB2_DIR" \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/meta/.migrations-applied" 2>/dev/null || echo "")
assert_contains "stub recorded as applied" "9002-stub-applied" "$APPLIED"

# ============================================================
echo ""
echo "=== Pre-run banner ==="
echo ""

echo "Test: banner appears when at least one pending migration"
REPO=$(setup_old_repo)
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "banner present" "About to apply" "$OUTPUT"
assert_contains "commit-before-running warning" "your working tree before running" "$OUTPUT"
assert_contains "skip hint per migration" "To skip:" "$OUTPUT"

echo "Test: banner suppressed when no pending migrations"
REPO=$(mktemp -d "$TMPDIR_BASE/empty-XXXXXX")
mkdir -p "$REPO/.git" "$REPO/meta"
printf '0001-rename-tickets-to-work\n' > "$REPO/meta/.migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_not_contains "no banner" "About to apply" "$OUTPUT"

echo ""

echo "=== Migration 0002: rename work items with project prefix ==="
echo ""

FIXTURE_0002="$SCRIPT_DIR/test-fixtures/0002"

setup_0002_repo() {
  local repo
  repo=$(mktemp -d "$TMPDIR_BASE/m0002-XXXXXX")
  cp -R "$FIXTURE_0002/." "$repo/"
  mkdir -p "$repo/.git" "$repo/meta"
  # Mark 0001 as applied so only 0002 runs
  printf '0001-rename-tickets-to-work\n' > "$repo/meta/.migrations-applied"
  printf '%s\n' "$repo"
}

echo "Test: pattern lacks {project} — no-op, stays pending"
REPO=$(mktemp -d "$TMPDIR_BASE/m0002-noproj-XXXXXX")
cp -R "$FIXTURE_0002/." "$REPO/"
mkdir -p "$REPO/.git" "$REPO/meta"
# Override config to have no {project}
printf '%s\n' '---' 'work:' '  id_pattern: "{number:04d}"' '---' > "$REPO/.claude/accelerator.md"
printf '0001-rename-tickets-to-work\n' > "$REPO/meta/.migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/meta/.migrations-applied")
assert_not_contains "stays pending" "0002-rename-work-items-with-project-prefix" "$APPLIED"
# Files unchanged
assert_eq "files unchanged" "1" "$([ -f "$REPO/meta/work/0001-add-foo.md" ] && echo 1 || echo 0)"

echo "Test: pattern has {project} but default_project_code empty — exits non-zero"
REPO=$(mktemp -d "$TMPDIR_BASE/m0002-nocode-XXXXXX")
cp -R "$FIXTURE_0002/." "$REPO/"
mkdir -p "$REPO/.git" "$REPO/meta"
printf '%s\n' '---' 'work:' '  id_pattern: "{project}-{number:04d}"' '---' > "$REPO/.claude/accelerator.md"
printf '0001-rename-tickets-to-work\n' > "$REPO/meta/.migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "non-zero exit" "1" "$([ "$RC" -ne 0 ] && echo 1 || echo 0)"
assert_contains "error mentions default_project_code" "default_project_code" "$OUTPUT"
assert_eq "file unchanged" "1" "$([ -f "$REPO/meta/work/0001-add-foo.md" ] && echo 1 || echo 0)"

echo "Test: single-project rename — files renamed and frontmatter updated"
REPO=$(setup_0002_repo)
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_eq "0001 renamed" "1" "$([ -f "$REPO/meta/work/PROJ-0001-add-foo.md" ] && echo 1 || echo 0)"
assert_eq "0042 renamed" "1" "$([ -f "$REPO/meta/work/PROJ-0042-add-bar.md" ] && echo 1 || echo 0)"
assert_eq "0099 renamed" "1" "$([ -f "$REPO/meta/work/PROJ-0099-bare-frontmatter.md" ] && echo 1 || echo 0)"
assert_eq "old 0001 gone" "0" "$([ -f "$REPO/meta/work/0001-add-foo.md" ] && echo 1 || echo 0)"
CONTENT=$(cat "$REPO/meta/work/PROJ-0001-add-foo.md")
assert_contains "work_item_id updated" 'work_item_id: "PROJ-0001"' "$CONTENT"

echo "Test: parent quoted scalar rewrites"
CONTENT=$(cat "$REPO/meta/work/PROJ-0042-add-bar.md")
assert_contains "parent rewritten" 'parent: "PROJ-0001"' "$CONTENT"

echo "Test: parent bare scalar rewrites to quoted"
CONTENT=$(cat "$REPO/meta/work/PROJ-0099-bare-frontmatter.md")
assert_contains "bare parent rewritten" 'parent: "PROJ-0042"' "$CONTENT"

echo "Test: related inline list (quoted) rewrites"
CONTENT=$(cat "$REPO/meta/research/2026-04-02-research.md")
assert_contains "related list rewritten" '"PROJ-0001"' "$CONTENT"
assert_contains "related list item 2" '"PROJ-0042"' "$CONTENT"

echo "Test: related inline list (bare) rewrites"
CONTENT=$(cat "$REPO/meta/work/PROJ-0099-bare-frontmatter.md")
assert_contains "bare list item 0001" '"PROJ-0001"' "$CONTENT"
assert_contains "bare list item 0099" '"PROJ-0099"' "$CONTENT"

echo "Test: markdown links rewritten"
CONTENT=$(cat "$REPO/meta/plans/2026-04-01-some-plan.md")
assert_contains "link 0001 rewritten" "../work/PROJ-0001-add-foo.md" "$CONTENT"
assert_contains "link 0042 with anchor" "../work/PROJ-0042-add-bar.md#section" "$CONTENT"

echo "Test: fenced-code-block path in tagged block rewritten"
CONTENT=$(cat "$REPO/meta/research/2026-04-02-research.md")
assert_contains "code block path 0042" "meta/work/PROJ-0042-add-bar.md" "$CONTENT"
assert_contains "code block path 0001" "meta/work/PROJ-0001-add-foo.md" "$CONTENT"

echo "Test: heading-line #NNNN references rewritten"
CONTENT=$(cat "$REPO/meta/plans/2026-04-01-some-plan.md")
assert_contains "heading #0042" "#PROJ-0042" "$CONTENT"
assert_contains "multi-ref heading #0001" "#PROJ-0001" "$CONTENT"

echo "Test: negative — bare fenced block NOT rewritten"
CONTENT=$(cat "$REPO/meta/research/2026-04-03-history.md")
assert_contains "bare block preserved" "meta/work/0042-add-bar.md" "$CONTENT"

echo "Test: negative — prose 0042 NOT rewritten"
assert_contains "prose 0042" "port 0042" "$CONTENT"
assert_contains "occurrences" "0042 occurrences" "$CONTENT"
assert_contains "timestamp" "2026-04-15" "$CONTENT"

echo "Test: negative — non-path numeric in tagged block NOT rewritten"
assert_contains "non-path code" "foo --id 0042" "$CONTENT"

echo "Test: non-work-item file in meta/work/ unchanged"
CONTENT=$(cat "$REPO/meta/work/notes.md")
assert_contains "notes unchanged" "non-work-item file" "$CONTENT"
assert_not_contains "notes not renamed" "PROJ" "$CONTENT"

echo "Test: idempotency — second run is a no-op"
HASH1=$(tree_hash "$REPO/meta")
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 second run" "0" "$RC"
HASH2=$(tree_hash "$REPO/meta")
assert_eq "byte-identical" "$HASH1" "$HASH2"

echo "Test: already-rewritten input is a no-op"
REPO2=$(mktemp -d "$TMPDIR_BASE/m0002-rewritten-XXXXXX")
cp -R "$REPO/." "$REPO2/"
# Remove from applied so 0002 runs again
grep -v "0002-rename-work-items-with-project-prefix" "$REPO2/meta/.migrations-applied" > "$REPO2/meta/.migrations-applied.tmp" || true
mv "$REPO2/meta/.migrations-applied.tmp" "$REPO2/meta/.migrations-applied"
HASH1=$(tree_hash "$REPO2/meta" -name '*.md')
RC=0
OUTPUT=$(cd "$REPO2" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 already-rewritten" "0" "$RC"
HASH2=$(tree_hash "$REPO2/meta" -name '*.md')
assert_eq "no changes on rewritten input" "$HASH1" "$HASH2"

echo "Test: collision — target file exists, aborts cleanly"
REPO=$(setup_0002_repo)
# Create the target file to cause collision
touch "$REPO/meta/work/PROJ-0001-add-foo.md"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "non-zero on collision" "1" "$([ "$RC" -ne 0 ] && echo 1 || echo 0)"
assert_contains "collision error" "collision" "$OUTPUT"
# Original file still there
assert_eq "original preserved" "1" "$([ -f "$REPO/meta/work/0001-add-foo.md" ] && echo 1 || echo 0)"

echo "Test: skip-tracking suppresses migration 0002"
REPO=$(setup_0002_repo)
(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 0002-rename-work-items-with-project-prefix)
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 with skip" "0" "$RC"
assert_contains "no pending" "No pending migrations" "$OUTPUT"
assert_eq "file not renamed" "1" "$([ -f "$REPO/meta/work/0001-add-foo.md" ] && echo 1 || echo 0)"

test_summary
