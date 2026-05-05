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
CONFIG=$(cat "$REPO/.accelerator/config.md" 2>/dev/null || echo "")
assert_contains "config has work key" "work: meta/work" "$CONFIG"
assert_not_contains "config has no tickets key" "tickets:" "$CONFIG"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "state file has migration ID" "0001-rename-tickets-to-work" "$APPLIED"

echo ""

# ── Test 2: Re-running is idempotent ─────────────────────────────────────────
echo "Test: Re-running is idempotent"
REPO=$(setup_old_repo)
# First run
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" > /dev/null 2>&1
BEFORE_STATE=$(cat "$REPO/.accelerator/state/migrations-applied")
# Second run
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
AFTER_STATE=$(cat "$REPO/.accelerator/state/migrations-applied")
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
mkdir -p "$REPO/.accelerator/state"
printf '0001-rename-tickets-to-work\n' > "$REPO/.accelerator/state/migrations-applied"
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
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
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
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
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
CONFIG=$(cat "$REPO/.accelerator/config.md" 2>/dev/null || echo "")
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
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
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
mkdir -p "$REPO/.accelerator/state"
printf '0099-future-migration\n' > "$REPO/.accelerator/state/migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
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
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "state file has 0001" "0001-rename-tickets-to-work" "$APPLIED"
assert_contains "state file has 0002" "0002-noop" "$APPLIED"
FIRST_LINE=$(head -1 "$REPO/.accelerator/state/migrations-applied")
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
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_not_contains "no state entry on abort" "0001-rename-tickets-to-work" "$APPLIED"
# Re-run with ACCELERATOR_MIGRATE_FORCE=1 bypasses the check
RC=0
cd "$REPO" && \
  ACCELERATOR_MIGRATE_FORCE=1 CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" \
  > /dev/null 2>&1 || RC=$?
assert_eq "exit 0 with FORCE flag" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
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
SKIPPED=$(cat "$REPO/.accelerator/state/migrations-skipped" 2>/dev/null || echo "")
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
SKIPPED=$(cat "$REPO/.accelerator/state/migrations-skipped" 2>/dev/null || echo "")
assert_not_contains "skip file no longer has ID" "0001-rename-tickets-to-work" "$SKIPPED"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "migration applied after unskip" "0001-rename-tickets-to-work" "$APPLIED"

echo "Test: --skip is idempotent"
REPO=$(setup_old_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 0001-rename-tickets-to-work \
  > /dev/null 2>&1
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 0001-rename-tickets-to-work \
  > /dev/null 2>&1
COUNT=$(grep -c "^0001-rename-tickets-to-work$" "$REPO/.accelerator/state/migrations-skipped")
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
SKIPPED=$(cat "$REPO/.accelerator/state/migrations-skipped" 2>/dev/null || echo "")
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
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_not_contains "skipped migration NOT applied under FORCE" "0001-rename-tickets-to-work" "$APPLIED"

echo "Test: applied + skipped same ID — applied wins, warning emitted"
REPO=$(setup_old_repo)
mkdir -p "$REPO/.accelerator/state"
printf '0001-rename-tickets-to-work\n' > "$REPO/.accelerator/state/migrations-applied"
printf '0001-rename-tickets-to-work\n' > "$REPO/.accelerator/state/migrations-skipped"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "warns about cross-state inconsistency" "BOTH" "$OUTPUT"
assert_contains "no pending output (applied wins)" "No pending migrations" "$OUTPUT"

echo "Test: empty .migrations-skipped is treated as no-skip"
REPO=$(setup_old_repo)
mkdir -p "$REPO/.accelerator/state"
: > "$REPO/.accelerator/state/migrations-skipped"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
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
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
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
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
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
mkdir -p "$REPO/.git" "$REPO/.accelerator/state"
printf '0001-rename-tickets-to-work\n' > "$REPO/.accelerator/state/migrations-applied"
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
  mkdir -p "$repo/.git" "$repo/meta" "$repo/.accelerator/state"
  # Mark 0001 as applied so only 0002 runs
  printf '0001-rename-tickets-to-work\n' > "$repo/.accelerator/state/migrations-applied"
  printf '%s\n' "$repo"
}

echo "Test: pattern lacks {project} — no-op, stays pending"
REPO=$(mktemp -d "$TMPDIR_BASE/m0002-noproj-XXXXXX")
cp -R "$FIXTURE_0002/." "$REPO/"
mkdir -p "$REPO/.git" "$REPO/meta" "$REPO/.accelerator/state"
# Override config to have no {project}
printf '%s\n' '---' 'work:' '  id_pattern: "{number:04d}"' '---' > "$REPO/.claude/accelerator.md"
printf '0001-rename-tickets-to-work\n' > "$REPO/.accelerator/state/migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied")
assert_not_contains "stays pending" "0002-rename-work-items-with-project-prefix" "$APPLIED"
# Files unchanged
assert_eq "files unchanged" "1" "$([ -f "$REPO/meta/work/0001-add-foo.md" ] && echo 1 || echo 0)"

echo "Test: pattern has {project} but default_project_code empty — exits non-zero"
REPO=$(mktemp -d "$TMPDIR_BASE/m0002-nocode-XXXXXX")
cp -R "$FIXTURE_0002/." "$REPO/"
mkdir -p "$REPO/.git" "$REPO/meta" "$REPO/.accelerator/state"
printf '%s\n' '---' 'work:' '  id_pattern: "{project}-{number:04d}"' '---' > "$REPO/.claude/accelerator.md"
printf '0001-rename-tickets-to-work\n' > "$REPO/.accelerator/state/migrations-applied"
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
grep -v "0002-rename-work-items-with-project-prefix" "$REPO2/.accelerator/state/migrations-applied" > "$REPO2/.accelerator/state/migrations-applied.tmp" || true
mv "$REPO2/.accelerator/state/migrations-applied.tmp" "$REPO2/.accelerator/state/migrations-applied"
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

# ============================================================
echo ""
echo "=== Migration 0003: relocate accelerator state ==="
echo ""

FIXTURE_0003="$SCRIPT_DIR/test-fixtures/0003"

ONLY_0003_DIR="$TMPDIR_BASE/only-0003-migrations"
mkdir -p "$ONLY_0003_DIR"
cp "$MIGRATIONS_DIR/0003-relocate-accelerator-state.sh" "$ONLY_0003_DIR/"

MIGRATION_0003="$MIGRATIONS_DIR/0003-relocate-accelerator-state.sh"

# Creates a fully-seeded legacy repo. 0001+0002 marked applied so only 0003 runs.
setup_0003_repo() {
  local repo
  repo=$(mktemp -d "$TMPDIR_BASE/m0003-XXXXXX")
  cp -R "$FIXTURE_0003/." "$repo/"
  mkdir -p "$repo/.accelerator/state"
  printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n' \
    > "$repo/.accelerator/state/migrations-applied"
  printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n' \
    > "$repo/meta/.migrations-applied"
  printf '%s\n' "$repo"
}

# ── Test 1: dirty-tree refusal covers .accelerator/ ──────────────────────────
echo "Test: dirty-tree refusal applies to .accelerator/ changes"
REPO=$(mktemp -d "$TMPDIR_BASE/m0003-dirty-XXXXXX")
mkdir -p "$REPO/.accelerator/state"
printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n' \
  > "$REPO/.accelerator/state/migrations-applied"
git -C "$REPO" init -q
git -C "$REPO" -c user.email="test@test.com" -c user.name="Test" add .
git -C "$REPO" -c user.email="test@test.com" -c user.name="Test" commit -qm "initial"
printf '\n# extra\n' >> "$REPO/.accelerator/state/migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit (dirty .accelerator/)" "0" "$RC"
assert_contains "error mentions dirty tree" "$OUTPUT" "dirty"

echo ""

# ── Test 2: end-to-end move from fully-seeded legacy repo ────────────────────
echo "Test: end-to-end move — all sources reach destinations, sources absent after"
REPO=$(setup_0003_repo)
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_file_exists ".accelerator/config.md created" "$REPO/.accelerator/config.md"
assert_file_not_exists ".claude/accelerator.md removed" "$REPO/.claude/accelerator.md"
assert_dir_exists ".accelerator/skills created" "$REPO/.accelerator/skills"
assert_dir_not_exists ".claude/accelerator/skills removed" "$REPO/.claude/accelerator/skills"
assert_dir_exists ".accelerator/lenses created" "$REPO/.accelerator/lenses"
assert_file_exists ".accelerator/skills/my-skill/context.md" \
  "$REPO/.accelerator/skills/my-skill/context.md"
assert_file_exists ".accelerator/lenses/my-lens-lens/SKILL.md" \
  "$REPO/.accelerator/lenses/my-lens-lens/SKILL.md"
assert_dir_exists ".accelerator/templates created" "$REPO/.accelerator/templates"
assert_dir_not_exists "meta/templates removed" "$REPO/meta/templates"
assert_dir_exists ".accelerator/state/integrations/jira created" \
  "$REPO/.accelerator/state/integrations/jira"
assert_dir_not_exists "meta/integrations/jira removed" \
  "$REPO/meta/integrations/jira"
assert_file_exists "jira fields.json moved" \
  "$REPO/.accelerator/state/integrations/jira/fields.json"
assert_dir_exists ".accelerator/tmp created" "$REPO/.accelerator/tmp"
assert_dir_not_exists "meta/tmp removed" "$REPO/meta/tmp"
assert_file_exists ".accelerator/state/migrations-applied exists" \
  "$REPO/.accelerator/state/migrations-applied"
assert_file_not_exists "meta/.migrations-applied removed" \
  "$REPO/meta/.migrations-applied"

echo ""

# ── Test 3: inner Jira .gitignore contains exact rules ───────────────────────
echo "Test: inner Jira .gitignore contains exact rules site.json, .refresh-meta.json, .lock/"
REPO=$(setup_0003_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
JIRA_GI="$REPO/.accelerator/state/integrations/jira/.gitignore"
assert_file_exists "inner .gitignore created" "$JIRA_GI"
GI_CONTENT=$(cat "$JIRA_GI")
assert_contains "site.json rule" "site.json" "$GI_CONTENT"
assert_contains ".refresh-meta.json rule" ".refresh-meta.json" "$GI_CONTENT"
assert_contains ".lock/ rule" ".lock/" "$GI_CONTENT"
# Exact-line matches (not substring)
assert_eq "site.json exact line" "1" \
  "$(grep -cFx 'site.json' "$JIRA_GI" || true)"
assert_eq ".refresh-meta.json exact line" "1" \
  "$(grep -cFx '.refresh-meta.json' "$JIRA_GI" || true)"
assert_eq ".lock/ exact line" "1" \
  "$(grep -cFx '.lock/' "$JIRA_GI" || true)"
assert_file_exists ".gitkeep created in jira dir" \
  "$REPO/.accelerator/state/integrations/jira/.gitkeep"

echo ""

# ── Test 4: paths.tmp unset — meta/tmp/ moves to .accelerator/tmp/ ───────────
echo "Test: paths.tmp unset — meta/tmp/ moves to .accelerator/tmp/"
REPO=$(setup_0003_repo)
# Fixture has no paths.tmp override
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
assert_dir_exists ".accelerator/tmp created" "$REPO/.accelerator/tmp"
assert_dir_not_exists "meta/tmp absent" "$REPO/meta/tmp"
assert_file_exists "session.json moved" "$REPO/.accelerator/tmp/session.json"

echo ""

# ── Test 5: paths.tmp overridden to custom path — meta/tmp/ untouched ────────
echo "Test: paths.tmp overridden to custom path — meta/tmp/ left untouched"
REPO=$(setup_0003_repo)
printf -- '---\npaths:\n  tmp: custom/tmp\n---\n' \
  >> "$REPO/.claude/accelerator.md"
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
assert_dir_exists "meta/tmp still present" "$REPO/meta/tmp"
assert_dir_not_exists ".accelerator/tmp not created" "$REPO/.accelerator/tmp"

echo ""

# ── Test 6: paths.tmp = "meta/tmp" literal — treated as explicit override ─────
echo "Test: paths.tmp = meta/tmp literal — explicit override leaves meta/tmp untouched"
REPO=$(setup_0003_repo)
printf -- '---\npaths:\n  tmp: meta/tmp\n---\n' \
  >> "$REPO/.claude/accelerator.md"
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
assert_dir_exists "meta/tmp still present (literal override)" "$REPO/meta/tmp"
assert_dir_not_exists ".accelerator/tmp not created" "$REPO/.accelerator/tmp"

echo ""

# ── Test 6a: paths.tmp = "meta/tmp/" (trailing slash) — also treated as set ──
echo "Test: paths.tmp with trailing slash — treated as explicit override"
REPO=$(setup_0003_repo)
printf -- '---\npaths:\n  tmp: meta/tmp/\n---\n' \
  >> "$REPO/.claude/accelerator.md"
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
assert_dir_exists "meta/tmp still present (slash override)" "$REPO/meta/tmp"

echo ""

# ── Test 6b: tmp under nested non-paths block — not detected as override ──────
echo "Test: tmp under non-paths block — awk anchor prevents false positive, meta/tmp moved"
REPO=$(setup_0003_repo)
printf -- '---\nsome_section:\n  tmp: meta/tmp\n---\n' \
  >> "$REPO/.claude/accelerator.md"
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
assert_dir_exists ".accelerator/tmp created (nested not detected)" "$REPO/.accelerator/tmp"
assert_dir_not_exists "meta/tmp moved away" "$REPO/meta/tmp"

echo ""

# ── Test 7: idempotency — re-run reports 0003 already applied ────────────────
echo "Test: idempotency — re-running after success reports no pending migrations"
REPO=$(setup_0003_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 on re-run" "0" "$RC"
assert_contains "no pending on re-run" "No pending migrations." "$OUTPUT"

echo ""

# ── Test 8: root .gitignore rewrite ──────────────────────────────────────────
echo "Test: root .gitignore — legacy rule replaced by anchored new rule"
REPO=$(setup_0003_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
GI="$REPO/.gitignore"
assert_file_exists ".gitignore still exists" "$GI"
assert_eq "old unanchored rule removed" "0" \
  "$(grep -cFx '.claude/accelerator.local.md' "$GI" || true)"
assert_eq "new anchored rule present" "1" \
  "$(grep -cFx '.accelerator/config.local.md' "$GI" || true)"

echo "Test: root .gitignore new rule not duplicated on re-run"
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
assert_eq "new rule present exactly once after re-run" "1" \
  "$(grep -cFx '.accelerator/config.local.md' "$GI" || true)"

echo ""

# ── Test 8a: .gitignore refuses on line with trailing content ─────────────────
echo "Test: root .gitignore rewrite refuses on legacy line with trailing content"
REPO=$(setup_0003_repo)
# Replace the clean rule with one that has a trailing comment
sed -i.bak 's|^\.claude/accelerator\.local\.md$|.claude/accelerator.local.md  # custom note|' \
  "$REPO/.gitignore"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit on trailing content" "0" "$RC"
assert_contains "error message names the offending line" "custom note" "$OUTPUT"
# File unchanged — no destructive write
CUSTOM_LINE=$(grep -F '# custom note' "$REPO/.gitignore" || echo "")
assert_contains "original line preserved" "$CUSTOM_LINE" "# custom note"

echo ""

# ── Test 9: Jira rules removed from .gitignore ───────────────────────────────
echo "Test: root .gitignore Jira legacy rules removed"
REPO=$(setup_0003_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
GI="$REPO/.gitignore"
assert_eq "jira .lock rule removed" "0" \
  "$(grep -cFx 'meta/integrations/jira/.lock' "$GI" || true)"
assert_eq "jira .refresh-meta.json rule removed" "0" \
  "$(grep -cFx 'meta/integrations/jira/.refresh-meta.json' "$GI" || true)"

echo ""

# ── Test 10: no_op_pending when no legacy sources ─────────────────────────────
echo "Test: no_op_pending sentinel when .accelerator/ has minimal scaffold and no sources"
REPO=$(mktemp -d "$TMPDIR_BASE/m0003-noop-XXXXXX")
mkdir -p "$REPO/.accelerator/state"
touch "$REPO/.accelerator/state/.gitkeep"
printf 'config.local.md\n' > "$REPO/.accelerator/.gitignore"
RC=0
OUTPUT=$(PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  bash "$MIGRATION_0003" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "no_op_pending sentinel in stdout" "MIGRATION_RESULT: no_op_pending" "$OUTPUT"

echo ""

# ── Test 11: idempotency from partial states ──────────────────────────────────
echo "Test: partial-state idempotency — config.md moved, skills/ pending — completes cleanly"
REPO=$(setup_0003_repo)
# Manually move only .claude/accelerator.md to simulate a mid-run state
mkdir -p "$REPO/.accelerator"
mv "$REPO/.claude/accelerator.md" "$REPO/.accelerator/config.md"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1 || RC=$?
assert_eq "exit 0 on partial recovery" "0" "$RC"
assert_file_exists ".accelerator/config.md still present" "$REPO/.accelerator/config.md"
assert_dir_exists ".accelerator/skills completed" "$REPO/.accelerator/skills"
assert_dir_not_exists ".claude/accelerator/skills gone" "$REPO/.claude/accelerator/skills"

echo "Test: partial-state idempotency — all sources moved, state file not yet merged"
REPO=$(setup_0003_repo)
# Move all sources manually, leave meta/.migrations-applied in place
mkdir -p "$REPO/.accelerator"
mv "$REPO/.claude/accelerator.md" "$REPO/.accelerator/config.md"
mv "$REPO/.claude/accelerator/skills" "$REPO/.accelerator/skills"
mv "$REPO/.claude/accelerator/lenses" "$REPO/.accelerator/lenses"
mkdir -p "$REPO/.accelerator/templates"
mv "$REPO/meta/templates/"* "$REPO/.accelerator/templates/" 2>/dev/null || true
rmdir "$REPO/meta/templates" 2>/dev/null || true
mkdir -p "$REPO/.accelerator/state/integrations"
mv "$REPO/meta/integrations/jira" "$REPO/.accelerator/state/integrations/"
rmdir "$REPO/meta/integrations" 2>/dev/null || true
mv "$REPO/meta/tmp" "$REPO/.accelerator/tmp"
# meta/.migrations-applied still exists
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1 || RC=$?
assert_eq "exit 0 on partial recovery (state pending)" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied")
assert_contains "0003 recorded" "0003-relocate-accelerator-state" "$APPLIED"
assert_file_not_exists "meta/.migrations-applied removed" "$REPO/meta/.migrations-applied"

echo ""

# ── Test 11a: conflict detection ──────────────────────────────────────────────
echo "Test: conflict detection — both .claude/accelerator.md and .accelerator/config.md exist with differing content"
REPO=$(setup_0003_repo)
mkdir -p "$REPO/.accelerator"
printf 'different content\n' > "$REPO/.accelerator/config.md"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit on conflict" "0" "$RC"
assert_contains "error names both paths" "accelerator.md" "$OUTPUT"
assert_contains "error names both paths (dest)" "config.md" "$OUTPUT"
assert_file_exists ".claude/accelerator.md still present" "$REPO/.claude/accelerator.md"
assert_file_exists ".accelerator/config.md still present (not wiped)" "$REPO/.accelerator/config.md"

echo ""

# ── Test 12: state-file merge with deduplication ──────────────────────────────
echo "Test: state-file merge — meta/.migrations-applied lines preserved and deduplicated"
REPO=$(setup_0003_repo)
# setup_0003_repo seeds both new and legacy state with 0001+0002
# After migration, new state file should be union {0001, 0002, 0003}
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied")
assert_contains "0001 preserved in merged state" "0001-rename-tickets-to-work" "$APPLIED"
assert_contains "0002 preserved in merged state" "0002-rename-work-items-with-project-prefix" "$APPLIED"
assert_contains "0003 recorded" "0003-relocate-accelerator-state" "$APPLIED"
# 0001 not duplicated despite appearing in both source and dest before merge
assert_eq "0001 exactly once" "1" \
  "$(grep -c '^0001-rename-tickets-to-work$' "$REPO/.accelerator/state/migrations-applied")"
assert_eq "0002 exactly once" "1" \
  "$(grep -c '^0002-rename-work-items-with-project-prefix$' "$REPO/.accelerator/state/migrations-applied")"

echo ""

# ── Test 13: trailing scaffold — no meta/templates/ → .accelerator/templates/ not created ─
echo "Test: trailing scaffold — no meta/templates/ source means .accelerator/templates/ not created"
REPO=$(setup_0003_repo)
rm -rf "$REPO/meta/templates"
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" > /dev/null 2>&1
assert_dir_not_exists ".accelerator/templates not pre-created" "$REPO/.accelerator/templates"

echo ""

# ── Test 14: pinned-override warning for paths.templates and paths.integrations ─
echo "Test: pinned-override warning emitted for paths.templates and paths.integrations"
REPO=$(setup_0003_repo)
printf -- '---\npaths:\n  templates: custom/templates\n  integrations: custom/ints\n---\n' \
  >> "$REPO/.claude/accelerator.md"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 (warning not error)" "0" "$RC"
assert_contains "warning names templates key" "paths.templates" "$OUTPUT"
assert_contains "warning names integrations key" "paths.integrations" "$OUTPUT"
assert_contains "warning names templates value" "custom/templates" "$OUTPUT"
# Migration still moved the files unconditionally
assert_dir_exists ".accelerator/templates still moved" "$REPO/.accelerator/templates"
assert_dir_not_exists "meta/templates gone" "$REPO/meta/templates"

echo ""

# ── Test 14a: no warning when neither key is pinned ──────────────────────────
echo "Test: no pinned-override warning when neither paths.templates nor paths.integrations is set"
REPO=$(setup_0003_repo)
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_not_contains "no templates warning" "$OUTPUT" "paths.templates"
assert_not_contains "no integrations warning" "$OUTPUT" "paths.integrations"

echo ""

test_summary
