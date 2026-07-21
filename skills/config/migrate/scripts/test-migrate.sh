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

# Generic runner tests exercise runner mechanics against the bundled MECHANICAL
# migrations. Interactive migrations (# INTERACTIVE: yes — e.g. 0007) are driven
# by their own suites (test-migrate-0007.sh / test-migrate-interactive.sh): their
# whole-corpus self-validation and FIFO handshake make them unsuitable for the
# minimal fixtures these tests apply the full set to. Pin the default migrations
# dir to the non-interactive set so adding an interactive migration doesn't break
# runner-mechanics tests. Tests that set ACCELERATOR_MIGRATIONS_DIR inline still
# override this default.
MECHANICAL_MIGRATIONS_DIR="$TMPDIR_BASE/mechanical-migrations"
mkdir -p "$MECHANICAL_MIGRATIONS_DIR"
for _m in "$MIGRATIONS_DIR"/[0-9][0-9][0-9][0-9]-*.sh; do
  if ! grep -qE '^# INTERACTIVE:[[:space:]]*yes$' < <(head -5 "$_m"); then
    cp "$_m" "$MECHANICAL_MIGRATIONS_DIR/"
  fi
done
export ACCELERATOR_MIGRATIONS_DIR="$MECHANICAL_MIGRATIONS_DIR"

# A migrations directory containing only 0001, used by tests that
# don't want 0002 or 0003 interfering with "No pending" assertions.
ONLY_0001_DIR="$TMPDIR_BASE/only-0001-migrations"
mkdir -p "$ONLY_0001_DIR"
cp "$MIGRATIONS_DIR/0001-rename-tickets-to-work.sh" "$ONLY_0001_DIR/"

# A migrations directory containing only 0001 and 0002, used by tests that
# focus on 0002 behaviour without 0003 running.
ONLY_0001_0002_DIR="$TMPDIR_BASE/only-0001-0002-migrations"
mkdir -p "$ONLY_0001_0002_DIR"
cp "$MIGRATIONS_DIR/0001-rename-tickets-to-work.sh" "$ONLY_0001_0002_DIR/"
cp "$MIGRATIONS_DIR/0002-rename-work-items-with-project-prefix.sh" "$ONLY_0001_0002_DIR/"

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
  printf -- '---\nticket_id: 0001\n---\n\n# 0001: Foo\n' >"$repo_dir/meta/tickets/0001-foo.md"
  mkdir -p "$repo_dir/meta/reviews/tickets"
  printf -- '---\ntype: work-item-review\n---\n\n# foo-review-1\n' \
    >"$repo_dir/meta/reviews/tickets/foo-review-1.md"
  mkdir -p "$repo_dir/.claude"
  printf -- '---\npaths:\n  tickets: meta/tickets\n---\n' \
    >"$repo_dir/.claude/accelerator.md"
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
assert_contains "work_item_id in file" "$CONTENT" "work_item_id: 0001"
assert_not_contains "ticket_id absent" "$CONTENT" "ticket_id:"
assert_file_exists "meta/reviews/work/foo-review-1.md created" \
  "$REPO/meta/reviews/work/foo-review-1.md"
CONFIG=$(cat "$REPO/.accelerator/config.md" 2>/dev/null || echo "")
assert_contains "config has work key" "$CONFIG" "work: meta/work"
assert_not_contains "config has no tickets key" "$CONFIG" "tickets:"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "state file has migration ID" "$APPLIED" "0001-rename-tickets-to-work"

echo ""

# ── Test 2: Re-running is idempotent ─────────────────────────────────────────
echo "Test: Re-running is idempotent"
REPO=$(setup_old_repo)
# First run
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" >/dev/null 2>&1
BEFORE_STATE=$(cat "$REPO/.accelerator/state/migrations-applied")
# Second run
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
AFTER_STATE=$(cat "$REPO/.accelerator/state/migrations-applied")
assert_eq "exit 0" "0" "$RC"
assert_contains "outputs no pending" "$OUTPUT" "No pending migrations"
assert_eq "state file unchanged" "$BEFORE_STATE" "$AFTER_STATE"

echo ""

# ── Test 3: Pre-populated state file skips migration ─────────────────────────
echo "Test: Pre-populated state file skips migration on first run"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta/work"
printf -- '---\nwork_item_id: 0001\n---\n\n# 0001: Foo\n' >"$REPO/meta/work/0001-foo.md"
mkdir -p "$REPO/.claude"
printf -- '---\npaths:\n  work: meta/work\n---\n' >"$REPO/.claude/accelerator.md"
mkdir -p "$REPO/.accelerator/state"
printf '0001-rename-tickets-to-work\n' >"$REPO/.accelerator/state/migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "no pending output" "$OUTPUT" "No pending migrations"

echo ""

# ── Test 4: Failed migration aborts without updating state file ───────────────
# Uses a stub migration that exits non-zero to induce the failure — the
# relocation migrations no longer abort on collision (they merge), so a failing
# stub is the stable way to exercise the orchestrator's "failure → state not
# updated" contract.
echo "Test: Failed migration aborts without updating state file"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/.git" "$REPO/meta"
FAIL_DIR=$(mktemp -d "$TMPDIR_BASE/failmigs-XXXXXX")
cat >"$FAIL_DIR/9003-stub-fail.sh" <<'STUB'
#!/usr/bin/env bash
# DESCRIPTION: stub migration that fails
echo "boom" >&2
exit 1
STUB
chmod +x "$FAIL_DIR/9003-stub-fail.sh"
RC=0
cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$FAIL_DIR" \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
assert_neq "non-zero exit" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_not_contains "failed migration NOT recorded as applied" "$APPLIED" "9003-stub-fail"

echo ""

# ── Test 5: Per-migration idempotency (direct invocation) ────────────────────
echo "Test: Per-migration idempotency (direct invocation)"
MIGRATION="$MIGRATIONS_DIR/0001-rename-tickets-to-work.sh"
REPO=$(setup_old_repo)
RC=0
PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$MIGRATION" >/dev/null 2>&1 || RC=$?
assert_eq "first run exit 0" "0" "$RC"
assert_dir_exists "meta/work created" "$REPO/meta/work"
assert_dir_not_exists "meta/tickets removed" "$REPO/meta/tickets"
# Second run
RC=0
PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$MIGRATION" >/dev/null 2>&1 || RC=$?
assert_eq "second run exit 0" "0" "$RC"
assert_dir_exists "meta/work still exists" "$REPO/meta/work"

echo ""

# ── Test 6: Empty repo (no meta/tickets/) is a no-op ─────────────────────────
echo "Test: Empty repo is a no-op (state file still written)"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "state file has migration ID" "$APPLIED" "0001-rename-tickets-to-work"

echo ""

# ── Test 7: paths.tickets override is respected ──────────────────────────────
echo "Test: paths.tickets override is respected — pinned directory preserved"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta/custom-tix"
printf -- '---\nticket_id: 0001\n---\n\n# 0001: Foo\n' >"$REPO/meta/custom-tix/0001-foo.md"
mkdir -p "$REPO/.claude"
printf -- '---\npaths:\n  tickets: meta/custom-tix\n---\n' >"$REPO/.claude/accelerator.md"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_dir_exists "meta/custom-tix still exists" "$REPO/meta/custom-tix"
CONTENT=$(cat "$REPO/meta/custom-tix/0001-foo.md")
assert_contains "work_item_id in file" "$CONTENT" "work_item_id: 0001"
assert_not_contains "no ticket_id" "$CONTENT" "ticket_id:"
CONFIG=$(cat "$REPO/.accelerator/config.md" 2>/dev/null || echo "")
assert_contains "config key renamed to work" "$CONFIG" "work: meta/custom-tix"
assert_not_contains "config has no tickets key" "$CONFIG" "tickets:"
assert_dir_not_exists "meta/work not spuriously created" "$REPO/meta/work"

echo ""

# ── Test 8: Both default dirs exist — merge, source wins, no abort ───────────
# Runs through the orchestrator (with the 0001-only migrations dir) so the real
# dispatch-context `source fs-common.sh` path under PATH `bash` is exercised,
# and 0006 does NOT run — so a destination-resident file keeps its ticket_id,
# pinning that 0001's merge leaves destination content for 0006 to canonicalise.
echo "Test: Both meta/tickets/ and meta/work/ exist — merge with source-wins, no abort"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta/tickets" "$REPO/meta/work"
mkdir -p "$REPO/meta/reviews/tickets" "$REPO/meta/reviews/work"
# tickets-side (source) files: 0001-foo unique; shared overlaps meta/work.
printf -- '---\nticket_id: 0001\n---\n\nSRC-FOO\n' >"$REPO/meta/tickets/0001-foo.md"
printf -- '---\nticket_id: 0003\n---\n\nSRC-SHARED\n' >"$REPO/meta/tickets/shared.md"
# work-side (destination) files: shared overlaps (differing); dest-only resident.
printf -- '---\nwork_item_id: 0003\n---\n\nDEST-SHARED\n' >"$REPO/meta/work/shared.md"
printf -- '---\nticket_id: 0009\n---\n\nDEST-ONLY\n' >"$REPO/meta/work/dest-only.md"
# review pair: r-shared overlaps (differing); r-dest resident.
printf -- 'SRC-RSHARED\n' >"$REPO/meta/reviews/tickets/r-shared.md"
printf -- 'DEST-RSHARED\n' >"$REPO/meta/reviews/work/r-shared.md"
printf -- 'DEST-RONLY\n' >"$REPO/meta/reviews/work/r-dest.md"
mkdir -p "$REPO/.claude"
printf -- '---\n---\n' >"$REPO/.claude/accelerator.md"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 — merge, no abort" "0" "$RC"
# Legacy sources removed after the merge.
assert_dir_not_exists "meta/tickets removed" "$REPO/meta/tickets"
assert_dir_not_exists "meta/reviews/tickets removed" "$REPO/meta/reviews/tickets"
# tickets→work pair merged; Step 2 rewrote ticket_id→work_item_id in the source.
FOO=$(cat "$REPO/meta/work/0001-foo.md")
assert_contains "unique source file moved" "$FOO" "SRC-FOO"
assert_contains "source frontmatter canonicalised by Step 2" "$FOO" "work_item_id: 0001"
SHARED=$(cat "$REPO/meta/work/shared.md")
assert_contains "leaf collision: source content wins" "$SHARED" "SRC-SHARED"
assert_not_contains "destination content overwritten" "$SHARED" "DEST-SHARED"
assert_contains "source frontmatter canonicalised" "$SHARED" "work_item_id: 0003"
# Destination-resident file untouched by 0001 — left for 0006 (which did not run
# here), so it still carries the legacy ticket_id.
DEST_ONLY=$(cat "$REPO/meta/work/dest-only.md")
assert_contains "destination-resident file preserved" "$DEST_ONLY" "DEST-ONLY"
assert_contains "destination-resident ticket_id left for 0006" "$DEST_ONLY" "ticket_id: 0009"
# review pair merged with source-wins on the overlap.
assert_file_content_eq "review overlap: source wins" "$REPO/meta/reviews/work/r-shared.md" "SRC-RSHARED"
assert_file_content_eq "review dest-resident preserved" "$REPO/meta/reviews/work/r-dest.md" "DEST-RONLY"

echo ""

# ── Test 9: Malformed YAML in user config — refuse to migrate ────────────────
echo "Test: Malformed config aborts migration before any changes"
REPO=$(setup_old_repo)
# Overwrite with unclosed frontmatter
printf -- '---\npaths:\n  tickets: meta/tickets\n' >"$REPO/.claude/accelerator.md"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit" "0" "$RC"
assert_contains "error mentions config file" "$OUTPUT" "accelerator.md"
# meta/tickets/ should be untouched (step 2 never ran)
CONTENT=$(cat "$REPO/meta/tickets/0001-foo.md")
assert_contains "ticket_id unchanged" "$CONTENT" "ticket_id: 0001"

echo ""

# ── Test 10: Corrupt state file — preserve unknown IDs, warn ─────────────────
echo "Test: Unknown migration ID preserved and warned about"
REPO=$(setup_old_repo)
mkdir -p "$REPO/.accelerator/state"
printf '0099-future-migration\n' >"$REPO/.accelerator/state/migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "unknown ID preserved" "$APPLIED" "0099-future-migration"
assert_contains "new ID added" "$APPLIED" "0001-rename-tickets-to-work"
assert_contains "warning about unknown ID" "$OUTPUT" "0099-future-migration"

echo ""

# ── Test 11: Both ticket_id and work_item_id — no duplicate key ──────────────
echo "Test: Both ticket_id and work_item_id in frontmatter — no duplicate after migration"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta/tickets"
printf -- '---\nticket_id: 0001\nwork_item_id: 0001\n---\n\n# 0001: Foo\n' \
  >"$REPO/meta/tickets/0001-foo.md"
mkdir -p "$REPO/.claude"
printf -- '---\n---\n' >"$REPO/.claude/accelerator.md"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
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
  >"$REPO/meta/tickets/0001-with space.md"
mkdir -p "$REPO/.claude"
printf -- '---\n---\n' >"$REPO/.claude/accelerator.md"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_file_exists "file with space migrated" "$REPO/meta/work/0001-with space.md"
CONTENT=$(cat "$REPO/meta/work/0001-with space.md")
assert_contains "work_item_id in spaced file" "$CONTENT" "work_item_id: 0001"

echo ""

# ── Test 13: Driver — two pending migrations applied in order ─────────────────
echo "Test: Driver applies two pending migrations in order"
REPO=$(setup_old_repo)
FIXTURE_MIGRATIONS="$TMPDIR_BASE/fixture-migrations-$$"
mkdir -p "$FIXTURE_MIGRATIONS"
cp "$MIGRATIONS_DIR/0001-rename-tickets-to-work.sh" "$FIXTURE_MIGRATIONS/"
MARKER="$TMPDIR_BASE/0002-ran-$$"
cat >"$FIXTURE_MIGRATIONS/0002-noop.sh" <<EOF
#!/usr/bin/env bash
# DESCRIPTION: No-op test migration
touch "$MARKER"
EOF
chmod +x "$FIXTURE_MIGRATIONS/0002-noop.sh"
RC=0
cd "$REPO" &&
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
    ACCELERATOR_MIGRATIONS_DIR="$FIXTURE_MIGRATIONS" \
    bash "$DRIVER" >/dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "state file has 0001" "$APPLIED" "0001-rename-tickets-to-work"
assert_contains "state file has 0002" "$APPLIED" "0002-noop"
FIRST_LINE=$(head -1 "$REPO/.accelerator/state/migrations-applied")
assert_eq "0001 recorded first" "0001-rename-tickets-to-work" "$FIRST_LINE"
assert_file_exists "0002 marker file created" "$MARKER"

echo ""

# ── Test 14: Driver — clean-tree pre-flight aborts on dirty repo ──────────────
echo "Test: Clean-tree check aborts on uncommitted changes in meta/"
REPO=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
mkdir -p "$REPO/meta/tickets"
printf -- '---\nticket_id: 0001\n---\n\n# 0001: Foo\n' >"$REPO/meta/tickets/0001-foo.md"
mkdir -p "$REPO/.claude"
printf -- '---\n---\n' >"$REPO/.claude/accelerator.md"
# Initialise a real git repo and commit the file so it is tracked
git -C "$REPO" init -q
git -C "$REPO" -c user.email="test@test.com" -c user.name="Test" add .
git -C "$REPO" -c user.email="test@test.com" -c user.name="Test" commit -qm "initial"
# Modify the tracked file without committing — makes the tree dirty
printf '\n# extra line\n' >>"$REPO/meta/tickets/0001-foo.md"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit (dirty tree)" "0" "$RC"
assert_contains "error mentions dirty working tree" "$OUTPUT" "dirty"
# Golden capture of the entire refusal block. On a dirty tree the pre-flight
# exits before any stdout, so OUTPUT is exactly these two stderr lines —
# byte-for-byte. This pins the canonical refusal text (the only place the
# ACCELERATOR_MIGRATE_FORCE hint appears) so the Phase 1 porcelain-stripping
# refactor and the Phase 3 refuse_dirty_tree extraction cannot drift it.
EXPECTED_DIRTY_REFUSAL="Error: dirty working tree — uncommitted changes detected in meta/, .claude/accelerator*.md, or .accelerator/.
Commit or discard those changes first, or set ACCELERATOR_MIGRATE_FORCE=1 to skip this check."
assert_eq "exact dirty-tree refusal block" "$EXPECTED_DIRTY_REFUSAL" "$OUTPUT"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_not_contains "no state entry on abort" "$APPLIED" "0001-rename-tickets-to-work"
# Re-run with ACCELERATOR_MIGRATE_FORCE=1 bypasses the check
RC=0
cd "$REPO" &&
  ACCELERATOR_MIGRATE_FORCE=1 CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" \
    >/dev/null 2>&1 || RC=$?
assert_eq "exit 0 with FORCE flag" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "migration applied with FORCE" "$APPLIED" "0001-rename-tickets-to-work"

# ============================================================
echo ""
echo "=== Per-run path manifest (resume-safety recording) ==="
echo ""

# AC1: a migration that mutates scoped paths then fails leaves the manifest
# listing exactly those paths — including the failing migration's partial
# writes. Uses a REAL git repo (Test 14 model) with the target files committed
# clean, so the diff-based recorder observes them as modified-tracked: a fake
# `mkdir .git` would make `git status` yield nothing and the manifest empty.
echo "Test: manifest records partial writes after mid-run failure"
REPO=$(mktemp -d "$TMPDIR_BASE/m-manifest-XXXXXX")
mkdir -p "$REPO/meta/work"
printf 'a\n' >"$REPO/meta/work/aaa.md"
printf 'b\n' >"$REPO/meta/work/bbb.md"
git -C "$REPO" init -q
git -C "$REPO" -c user.email=t@t -c user.name=T add .
git -C "$REPO" -c user.email=t@t -c user.name=T commit -qm initial
MANIFEST_DIR=$(mktemp -d "$TMPDIR_BASE/manifestmigs-XXXXXX")
cat >"$MANIFEST_DIR/9001-stub-ok.sh" <<'STUB'
#!/usr/bin/env bash
# DESCRIPTION: stub appends to aaa then succeeds
printf 'x\n' >>"$PROJECT_ROOT/meta/work/aaa.md"
exit 0
STUB
# 9002 re-touches aaa (already recorded by 9001's success) AND writes bbb, then
# fails — exercising both partial-failure capture (bbb) and atomic_append_unique
# dedup (aaa recorded across two recording points).
cat >"$MANIFEST_DIR/9002-stub-fail.sh" <<'STUB'
#!/usr/bin/env bash
# DESCRIPTION: stub appends to bbb and aaa then fails
printf 'y\n' >>"$PROJECT_ROOT/meta/work/bbb.md"
printf 'z\n' >>"$PROJECT_ROOT/meta/work/aaa.md"
exit 1
STUB
chmod +x "$MANIFEST_DIR/9001-stub-ok.sh" "$MANIFEST_DIR/9002-stub-fail.sh"
RC=0
cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$MANIFEST_DIR" \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
assert_neq "non-zero exit (mid-run failure)" "0" "$RC"
MANIFEST="$REPO/.accelerator/state/migrations-run-paths.txt"
assert_file_exists "manifest written on failure" "$MANIFEST"
assert_eq "aaa recorded exactly once (dedup across two points)" "1" \
  "$(grep -cFx 'meta/work/aaa.md' "$MANIFEST" || true)"
assert_eq "bbb (failing migration's partial write) recorded once" "1" \
  "$(grep -cFx 'meta/work/bbb.md' "$MANIFEST" || true)"
assert_eq "manifest has exactly those two paths" "2" \
  "$(grep -c . "$MANIFEST" || true)"

echo ""

echo "Test: manifest + run-id deleted on full success"
REPO=$(mktemp -d "$TMPDIR_BASE/m-manifest-ok-XXXXXX")
mkdir -p "$REPO/meta"
git -C "$REPO" init -q
git -C "$REPO" -c user.email=t@t -c user.name=T commit -q --allow-empty -m initial
OK_DIR=$(mktemp -d "$TMPDIR_BASE/okmigs-XXXXXX")
cat >"$OK_DIR/9001-stub-ok.sh" <<'STUB'
#!/usr/bin/env bash
# DESCRIPTION: stub that succeeds without writing
exit 0
STUB
chmod +x "$OK_DIR/9001-stub-ok.sh"
RC=0
cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$OK_DIR" \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
assert_eq "exit 0 on full success" "0" "$RC"
assert_file_not_exists "run-paths.txt removed on success" \
  "$REPO/.accelerator/state/migrations-run-paths.txt"
assert_file_not_exists "run.id removed on success" \
  "$REPO/.accelerator/state/migrations-run.id"

echo ""

echo "Test: fresh run truncates a leftover manifest and mints a new run-id"
REPO=$(mktemp -d "$TMPDIR_BASE/m-manifest-stale-XXXXXX")
mkdir -p "$REPO/meta/work"
printf 'c\n' >"$REPO/meta/work/ccc.md"
git -C "$REPO" init -q
git -C "$REPO" -c user.email=t@t -c user.name=T add .
git -C "$REPO" -c user.email=t@t -c user.name=T commit -qm initial
mkdir -p "$REPO/.accelerator/state"
# Pre-seed a leftover manifest + run-id from an imagined prior run.
printf 'meta/work/STALE.md\n' >"$REPO/.accelerator/state/migrations-run-paths.txt"
printf 'old-stale-run-id\n' >"$REPO/.accelerator/state/migrations-run.id"
STALE_DIR=$(mktemp -d "$TMPDIR_BASE/stalemigs-XXXXXX")
cat >"$STALE_DIR/9001-stub-fail.sh" <<'STUB'
#!/usr/bin/env bash
# DESCRIPTION: stub appends to ccc then fails (so manifest persists)
printf 'd\n' >>"$PROJECT_ROOT/meta/work/ccc.md"
exit 1
STUB
chmod +x "$STALE_DIR/9001-stub-fail.sh"
RC=0
cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$STALE_DIR" \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
assert_neq "non-zero exit" "0" "$RC"
MANIFEST="$REPO/.accelerator/state/migrations-run-paths.txt"
assert_eq "leftover STALE path truncated away" "0" \
  "$(grep -cFx 'meta/work/STALE.md' "$MANIFEST" || true)"
assert_eq "this run's own write recorded" "1" \
  "$(grep -cFx 'meta/work/ccc.md' "$MANIFEST" || true)"
EXPECTED_REV=$(git -C "$REPO" rev-parse HEAD)
RECORDED_REV=$(head -n1 "$REPO/.accelerator/state/migrations-run.id")
assert_eq "run-id reset to current base revision" "$EXPECTED_REV" "$RECORDED_REV"

# ============================================================
echo ""
echo "=== Guarded resume (manifest-driven) ==="
echo ""

# A pending stub that succeeds, so a guarded resume proceeds into the apply loop.
GR_OK_DIR=$(mktemp -d "$TMPDIR_BASE/gr-ok-migs-XXXXXX")
cat >"$GR_OK_DIR/9100-gr-ok.sh" <<'STUB'
#!/usr/bin/env bash
# DESCRIPTION: pending stub that succeeds so a guarded resume proceeds
exit 0
STUB
chmod +x "$GR_OK_DIR/9100-gr-ok.sh"

# Build a repo under <vcs> whose tree is dirty at meta/work/{owned,foreign}.md
# (modified-tracked under git; tracked-created under jj). Echoes the repo path.
gr_setup_repo() {
  local vcs="$1" repo
  repo=$(mktemp -d "$TMPDIR_BASE/gr-$vcs-XXXXXX")
  mkdir -p "$repo/meta/work"
  printf 'base\n' >"$repo/meta/work/owned.md"
  printf 'base\n' >"$repo/meta/work/foreign.md"
  if [ "$vcs" = jj ]; then
    (cd "$repo" && jj git init --quiet)
  else
    git -C "$repo" init -q
    git -C "$repo" -c user.email=t@t -c user.name=T add .
    git -C "$repo" -c user.email=t@t -c user.name=T commit -qm initial
    printf 'x\n' >>"$repo/meta/work/owned.md"
    printf 'x\n' >>"$repo/meta/work/foreign.md"
  fi
  printf '%s\n' "$repo"
}

# Echo the current base revision the runner would record (change_id of @ for jj,
# HEAD for git) — must run from inside the repo for the jj case.
gr_base_rev() {
  local repo="$1" vcs="$2"
  if [ "$vcs" = jj ]; then
    (cd "$repo" && jj log -r @ --no-graph --no-pager -T change_id 2>/dev/null)
  else
    git -C "$repo" rev-parse HEAD
  fi
}

# Seed the manifest (one path per arg after the revision) + the run-id sidecar.
gr_seed() {
  local repo="$1" rev="$2" p
  shift 2
  mkdir -p "$repo/.accelerator/state"
  : >"$repo/.accelerator/state/migrations-run-paths.txt"
  for p in "$@"; do
    printf '%s\n' "$p" >>"$repo/.accelerator/state/migrations-run-paths.txt"
  done
  printf '%s\n' "$rev" >"$repo/.accelerator/state/migrations-run.id"
}

# Run the driver once (single invocation — a guarded resume MUTATES the tree, so
# a second run would see a changed tree). Sets GR_OUT and GR_RC.
gr_run() {
  local repo="$1"
  GR_RC=0
  GR_OUT=$(cd "$repo" && ACCELERATOR_MIGRATIONS_DIR="$GR_OK_DIR" \
    PROJECT_ROOT="$repo" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
    bash "$DRIVER" 2>&1) || GR_RC=$?
}

# AC2/AC3/AC4 for one VCS. The change_id-vs-commit_id capture and untracked
# handling are jj-specific, so these run under both git and jj.
run_guarded_resume_cases() {
  local vcs="$1" repo rev

  echo "Test: [$vcs] guarded resume on fully-owned dirty tree"
  repo=$(gr_setup_repo "$vcs")
  rev=$(gr_base_rev "$repo" "$vcs")
  gr_seed "$repo" "$rev" "meta/work/owned.md" "meta/work/foreign.md"
  gr_run "$repo"
  assert_eq "[$vcs] exit 0 (guarded resume, no FORCE)" "0" "$GR_RC"
  assert_contains "[$vcs] resume affordance present" "$GR_OUT" \
    "own partial migration output"
  assert_contains "[$vcs] affordance lists owned path" "$GR_OUT" \
    "meta/work/owned.md"

  echo "Test: [$vcs] refuse on mixed/non-owned dirty tree"
  repo=$(gr_setup_repo "$vcs")
  rev=$(gr_base_rev "$repo" "$vcs")
  gr_seed "$repo" "$rev" "meta/work/owned.md" # foreign.md NOT owned
  gr_run "$repo"
  assert_neq "[$vcs] non-zero exit (mixed)" "0" "$GR_RC"
  assert_not_contains "[$vcs] no affordance on mixed" "$GR_OUT" \
    "own partial migration output"
  assert_contains "[$vcs] FORCE-hint refusal on mixed" "$GR_OUT" \
    "ACCELERATOR_MIGRATE_FORCE"

  echo "Test: [$vcs] fail-closed — manifest absent"
  repo=$(gr_setup_repo "$vcs")
  rev=$(gr_base_rev "$repo" "$vcs")
  gr_seed "$repo" "$rev" "meta/work/owned.md" "meta/work/foreign.md"
  rm -f "$repo/.accelerator/state/migrations-run-paths.txt"
  gr_run "$repo"
  assert_neq "[$vcs] manifest absent → non-zero" "0" "$GR_RC"
  assert_contains "[$vcs] manifest absent → FORCE hint" "$GR_OUT" \
    "ACCELERATOR_MIGRATE_FORCE"
  assert_not_contains "[$vcs] manifest absent → no affordance" "$GR_OUT" \
    "own partial migration output"

  echo "Test: [$vcs] fail-closed — manifest empty"
  repo=$(gr_setup_repo "$vcs")
  rev=$(gr_base_rev "$repo" "$vcs")
  gr_seed "$repo" "$rev" "meta/work/owned.md" "meta/work/foreign.md"
  : >"$repo/.accelerator/state/migrations-run-paths.txt"
  gr_run "$repo"
  assert_neq "[$vcs] manifest empty → non-zero" "0" "$GR_RC"
  assert_contains "[$vcs] manifest empty → FORCE hint" "$GR_OUT" \
    "ACCELERATOR_MIGRATE_FORCE"

  echo "Test: [$vcs] fail-closed — run-id sidecar absent"
  repo=$(gr_setup_repo "$vcs")
  rev=$(gr_base_rev "$repo" "$vcs")
  gr_seed "$repo" "$rev" "meta/work/owned.md" "meta/work/foreign.md"
  rm -f "$repo/.accelerator/state/migrations-run.id"
  gr_run "$repo"
  assert_neq "[$vcs] run-id absent → non-zero" "0" "$GR_RC"
  assert_contains "[$vcs] run-id absent → FORCE hint" "$GR_OUT" \
    "ACCELERATOR_MIGRATE_FORCE"

  echo "Test: [$vcs] fail-closed — run-id sidecar empty"
  repo=$(gr_setup_repo "$vcs")
  rev=$(gr_base_rev "$repo" "$vcs")
  gr_seed "$repo" "$rev" "meta/work/owned.md" "meta/work/foreign.md"
  : >"$repo/.accelerator/state/migrations-run.id"
  gr_run "$repo"
  assert_neq "[$vcs] run-id empty → non-zero" "0" "$GR_RC"
  assert_contains "[$vcs] run-id empty → FORCE hint" "$GR_OUT" \
    "ACCELERATOR_MIGRATE_FORCE"

  echo "Test: [$vcs] fail-closed — recorded base revision differs (stale)"
  repo=$(gr_setup_repo "$vcs")
  rev=$(gr_base_rev "$repo" "$vcs")
  gr_seed "$repo" "$rev" "meta/work/owned.md" "meta/work/foreign.md"
  printf 'stale-revision-xyz\n' >"$repo/.accelerator/state/migrations-run.id"
  gr_run "$repo"
  assert_neq "[$vcs] stale rev → non-zero" "0" "$GR_RC"
  assert_contains "[$vcs] stale rev → FORCE hint" "$GR_OUT" \
    "ACCELERATOR_MIGRATE_FORCE"
  assert_not_contains "[$vcs] stale rev → no affordance" "$GR_OUT" \
    "own partial migration output"
}

run_guarded_resume_cases git
if command -v jj >/dev/null 2>&1; then
  run_guarded_resume_cases jj
else
  skip_test "jj guarded-resume cases (AC2-AC4)" "jj not available"
fi

echo ""
echo "Test: guarded resume that fails again accumulates correctly (git)"
# A real partial run, re-run into a guarded resume whose later migration also
# fails, then a third re-run where it succeeds — locks in the empty-baseline /
# self-healing behaviour (a re-asserted manifest, not a truncated one).
REPO=$(mktemp -d "$TMPDIR_BASE/gr-acc-XXXXXX")
mkdir -p "$REPO/meta/work"
printf 'a\n' >"$REPO/meta/work/fileA.md"
printf 'b\n' >"$REPO/meta/work/fileB.md"
git -C "$REPO" init -q
git -C "$REPO" -c user.email=t@t -c user.name=T add .
git -C "$REPO" -c user.email=t@t -c user.name=T commit -qm initial
ACC_CTR="$TMPDIR_BASE/acc-ctr-$$"
rm -f "$ACC_CTR"
ACC_DIR=$(mktemp -d "$TMPDIR_BASE/acc-migs-XXXXXX")
cat >"$ACC_DIR/9001-acc-ok.sh" <<'STUB'
#!/usr/bin/env bash
# DESCRIPTION: appends fileA then succeeds (applied on the first run)
printf 'a\n' >>"$PROJECT_ROOT/meta/work/fileA.md"
exit 0
STUB
cat >"$ACC_DIR/9002-acc-fail.sh" <<'STUB'
#!/usr/bin/env bash
# DESCRIPTION: appends fileB; fails the first two attempts, succeeds the third
printf 'b\n' >>"$PROJECT_ROOT/meta/work/fileB.md"
n=$(cat "$ACC_CTR" 2>/dev/null || echo 0)
n=$((n + 1))
printf '%s\n' "$n" >"$ACC_CTR"
[ "$n" -ge 3 ] && exit 0
exit 1
STUB
chmod +x "$ACC_DIR/9001-acc-ok.sh" "$ACC_DIR/9002-acc-fail.sh"
ACC_MANIFEST="$REPO/.accelerator/state/migrations-run-paths.txt"

# Run 1 (clean tree): 9001 applies (fileA), 9002 fails (fileB) → manifest {A,B}.
RC=0
OUT=$(cd "$REPO" && ACC_CTR="$ACC_CTR" ACCELERATOR_MIGRATIONS_DIR="$ACC_DIR" \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_neq "acc run 1 fails" "0" "$RC"
assert_eq "acc run 1 manifest has fileA" "1" \
  "$(grep -cFx 'meta/work/fileA.md' "$ACC_MANIFEST" || true)"
assert_eq "acc run 1 manifest has fileB" "1" \
  "$(grep -cFx 'meta/work/fileB.md' "$ACC_MANIFEST" || true)"

# Run 2 (resume): 9001 skipped (applied), 9002 fails again. Even though only
# fileB is touched this run, the empty-baseline re-asserts fileA → manifest {A,B}.
RC=0
OUT=$(cd "$REPO" && ACC_CTR="$ACC_CTR" ACCELERATOR_MIGRATIONS_DIR="$ACC_DIR" \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_neq "acc run 2 still fails" "0" "$RC"
assert_contains "acc run 2 is a guarded resume" "$OUT" "own partial migration output"
assert_eq "acc run 2 manifest still has fileA (self-healing)" "1" \
  "$(grep -cFx 'meta/work/fileA.md' "$ACC_MANIFEST" || true)"
assert_eq "acc run 2 manifest still has fileB" "1" \
  "$(grep -cFx 'meta/work/fileB.md' "$ACC_MANIFEST" || true)"
assert_eq "acc run 2 manifest still exactly two paths" "2" \
  "$(grep -c . "$ACC_MANIFEST" || true)"

# Run 3 (resume): 9002 now succeeds → full success, manifest deleted.
RC=0
OUT=$(cd "$REPO" && ACC_CTR="$ACC_CTR" ACCELERATOR_MIGRATIONS_DIR="$ACC_DIR" \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "acc run 3 resumes to success" "0" "$RC"
assert_contains "acc run 3 is a guarded resume" "$OUT" "own partial migration output"
assert_file_not_exists "acc manifest deleted after successful resume" "$ACC_MANIFEST"
ACC_APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "9002 applied after resume" "$ACC_APPLIED" "9002-acc-fail"

# ============================================================
echo ""
echo "=== --skip / --unskip flags ==="
echo ""

echo "Test: --skip records the ID in .migrations-skipped"
REPO=$(setup_old_repo)
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 0001-rename-tickets-to-work \
  >/dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
SKIPPED=$(cat "$REPO/.accelerator/state/migrations-skipped" 2>/dev/null || echo "")
assert_contains "skip file has migration ID" "$SKIPPED" "0001-rename-tickets-to-work"

echo "Test: subsequent run reports no pending migrations"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "outputs no pending" "$OUTPUT" "No pending migrations"
assert_contains "summary lists skipped name" "$OUTPUT" "Skipped:"
assert_contains "skipped name visible" "$OUTPUT" "0001-rename-tickets-to-work"
# Migration must NOT have run
assert_file_exists "meta/tickets/0001-foo.md still present" "$REPO/meta/tickets/0001-foo.md"

echo "Test: --unskip removes the ID and migration becomes pending again"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --unskip 0001-rename-tickets-to-work \
  >/dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
SKIPPED=$(cat "$REPO/.accelerator/state/migrations-skipped" 2>/dev/null || echo "")
assert_not_contains "skip file no longer has ID" "$SKIPPED" "0001-rename-tickets-to-work"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "migration applied after unskip" "$APPLIED" "0001-rename-tickets-to-work"

echo "Test: --skip is idempotent"
REPO=$(setup_old_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 0001-rename-tickets-to-work \
  >/dev/null 2>&1
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 0001-rename-tickets-to-work \
  >/dev/null 2>&1
COUNT=$(grep -c "^0001-rename-tickets-to-work$" "$REPO/.accelerator/state/migrations-skipped")
assert_eq "ID present exactly once" "1" "$COUNT"

echo "Test: --unskip on absent ID is a no-op"
REPO=$(setup_old_repo)
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --unskip 0001-rename-tickets-to-work \
  >/dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"

echo "Test: --unapply removes an entry from the applied ledger"
REPO=$(setup_old_repo)
mkdir -p "$REPO/.accelerator/state"
printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n' \
  >"$REPO/.accelerator/state/migrations-applied"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" \
  --unapply 0001-rename-tickets-to-work >/dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_not_contains "applied ledger no longer has unapplied ID" \
  "$APPLIED" "0001-rename-tickets-to-work"
assert_contains "applied ledger keeps the other ID" \
  "$APPLIED" "0002-rename-work-items-with-project-prefix"

echo "Test: skipping unknown ID writes it and warns on next run"
REPO=$(setup_old_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 9999-future-migration \
  >/dev/null 2>&1
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "warning about unknown skipped ID" "$OUTPUT" "9999-future-migration"
SKIPPED=$(cat "$REPO/.accelerator/state/migrations-skipped" 2>/dev/null || echo "")
assert_contains "unknown skipped ID preserved" "$SKIPPED" "9999-future-migration"

echo "Test: ACCELERATOR_MIGRATE_FORCE bypasses dirty-tree only — skip still wins"
REPO=$(setup_old_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 0001-rename-tickets-to-work \
  >/dev/null 2>&1
git -C "$REPO" init -q
git -C "$REPO" -c user.email=t@t -c user.name=T add .
git -C "$REPO" -c user.email=t@t -c user.name=T commit -qm initial
printf '\nx\n' >>"$REPO/meta/tickets/0001-foo.md"
RC=0
OUTPUT=$(cd "$REPO" && ACCELERATOR_MIGRATE_FORCE=1 CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "no pending under FORCE+skip" "$OUTPUT" "No pending migrations"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_not_contains "skipped migration NOT applied under FORCE" "$APPLIED" "0001-rename-tickets-to-work"

echo "Test: applied + skipped same ID — applied wins, warning emitted"
REPO=$(setup_old_repo)
mkdir -p "$REPO/.accelerator/state"
printf '0001-rename-tickets-to-work\n' >"$REPO/.accelerator/state/migrations-applied"
printf '0001-rename-tickets-to-work\n' >"$REPO/.accelerator/state/migrations-skipped"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "warns about cross-state inconsistency" "$OUTPUT" "BOTH"
assert_contains "no pending output (applied wins)" "$OUTPUT" "No pending migrations"

echo "Test: empty .migrations-skipped is treated as no-skip"
REPO=$(setup_old_repo)
mkdir -p "$REPO/.accelerator/state"
: >"$REPO/.accelerator/state/migrations-skipped"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "migration applied" "$APPLIED" "0001-rename-tickets-to-work"

# ============================================================
echo ""
echo "=== MIGRATION_RESULT contract ==="
echo ""

# Build a temporary migrations dir with a stub migration that emits the sentinel
echo "Test: migration emitting MIGRATION_RESULT: no_op_pending stays unapplied"
REPO=$(mktemp -d "$TMPDIR_BASE/no-op-XXXXXX")
mkdir -p "$REPO/.git" "$REPO/meta"
STUB_DIR=$(mktemp -d "$TMPDIR_BASE/stubmigs-XXXXXX")
cat >"$STUB_DIR/9001-stub-no-op.sh" <<'STUB'
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
assert_not_contains "stub NOT recorded as applied" "$APPLIED" "9001-stub-no-op"
# Sentinel is stripped from the user-visible output
assert_not_contains "sentinel hidden from user" "$OUTPUT" "MIGRATION_RESULT:"

echo "Test: stub stays pending across re-runs"
RC=0
OUTPUT=$(cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$STUB_DIR" \
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "stub still listed as about-to-apply" "$OUTPUT" "9001-stub-no-op"

echo "Test: 0-exit migration WITHOUT sentinel IS recorded"
STUB2_DIR=$(mktemp -d "$TMPDIR_BASE/stubmigs2-XXXXXX")
cat >"$STUB2_DIR/9002-stub-applied.sh" <<'STUB'
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
  CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "stub recorded as applied" "$APPLIED" "9002-stub-applied"

# ============================================================
echo ""
echo "=== Pre-run banner ==="
echo ""

echo "Test: banner appears when at least one pending migration"
REPO=$(setup_old_repo)
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "banner present" "$OUTPUT" "About to apply"
assert_contains "commit-before-running warning" "$OUTPUT" "your working tree before running"
assert_contains "skip hint per migration" "$OUTPUT" "To skip:"

echo "Test: banner suppressed when no pending migrations"
REPO=$(mktemp -d "$TMPDIR_BASE/empty-XXXXXX")
mkdir -p "$REPO/.git" "$REPO/.accelerator/state"
printf '0001-rename-tickets-to-work\n' >"$REPO/.accelerator/state/migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_DIR" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_not_contains "no banner" "$OUTPUT" "About to apply"

echo ""

echo "=== Migration 0002: rename work items with project prefix ==="
echo ""

FIXTURE_0002="$SCRIPT_DIR/test-fixtures/0002"

setup_0002_repo() {
  local repo
  repo=$(mktemp -d "$TMPDIR_BASE/m0002-XXXXXX")
  cp -R "$FIXTURE_0002/." "$repo/"
  mkdir -p "$repo/.git" "$repo/meta" "$repo/.accelerator/state"
  # Mark 0001 + 0004 as applied. 0004's no-op short-circuit handles the
  # absence of legacy research dirs, but 0002 fixtures DO populate
  # meta/research/, so we explicitly gate 0004 out to keep those files
  # at their legacy locations for 0002's assertions.
  printf '0001-rename-tickets-to-work\n0004-restructure-meta-research-into-subject-subcategories\n' \
    >"$repo/.accelerator/state/migrations-applied"
  printf '%s\n' "$repo"
}

echo "Test: pattern lacks {project} — no-op, stays pending"
REPO=$(mktemp -d "$TMPDIR_BASE/m0002-noproj-XXXXXX")
cp -R "$FIXTURE_0002/." "$REPO/"
mkdir -p "$REPO/.git" "$REPO/meta" "$REPO/.accelerator/state"
# Override config to have no {project}
printf '%s\n' '---' 'work:' '  id_pattern: "{number:04d}"' '---' >"$REPO/.claude/accelerator.md"
printf '0001-rename-tickets-to-work\n' >"$REPO/.accelerator/state/migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied")
assert_not_contains "stays pending" "$APPLIED" "0002-rename-work-items-with-project-prefix"
# Files unchanged
assert_eq "files unchanged" "1" "$([ -f "$REPO/meta/work/0001-add-foo.md" ] && echo 1 || echo 0)"

echo "Test: pattern has {project} but default_project_code empty — exits non-zero"
REPO=$(mktemp -d "$TMPDIR_BASE/m0002-nocode-XXXXXX")
cp -R "$FIXTURE_0002/." "$REPO/"
mkdir -p "$REPO/.git" "$REPO/meta" "$REPO/.accelerator/state"
printf '%s\n' '---' 'work:' '  id_pattern: "{project}-{number:04d}"' '---' >"$REPO/.claude/accelerator.md"
printf '0001-rename-tickets-to-work\n' >"$REPO/.accelerator/state/migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "non-zero exit" "1" "$([ "$RC" -ne 0 ] && echo 1 || echo 0)"
assert_contains "error mentions default_project_code" "$OUTPUT" "default_project_code"
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
assert_contains "work_item_id updated" "$CONTENT" 'work_item_id: "PROJ-0001"'

echo "Test: parent quoted scalar rewrites"
CONTENT=$(cat "$REPO/meta/work/PROJ-0042-add-bar.md")
assert_contains "parent rewritten" "$CONTENT" 'parent: "PROJ-0001"'

echo "Test: parent bare scalar rewrites to quoted"
CONTENT=$(cat "$REPO/meta/work/PROJ-0099-bare-frontmatter.md")
assert_contains "bare parent rewritten" "$CONTENT" 'parent: "PROJ-0042"'

echo "Test: related inline list (quoted) rewrites"
CONTENT=$(cat "$REPO/meta/research/2026-04-02-research.md")
assert_contains "related list rewritten" "$CONTENT" '"PROJ-0001"'
assert_contains "related list item 2" "$CONTENT" '"PROJ-0042"'

echo "Test: related inline list (bare) rewrites"
CONTENT=$(cat "$REPO/meta/work/PROJ-0099-bare-frontmatter.md")
assert_contains "bare list item 0001" "$CONTENT" '"PROJ-0001"'
assert_contains "bare list item 0099" "$CONTENT" '"PROJ-0099"'

echo "Test: markdown links rewritten"
CONTENT=$(cat "$REPO/meta/plans/2026-04-01-some-plan.md")
assert_contains "link 0001 rewritten" "$CONTENT" "../work/PROJ-0001-add-foo.md"
assert_contains "link 0042 with anchor" "$CONTENT" "../work/PROJ-0042-add-bar.md#section"

echo "Test: fenced-code-block path in tagged block rewritten"
CONTENT=$(cat "$REPO/meta/research/2026-04-02-research.md")
assert_contains "code block path 0042" "$CONTENT" "meta/work/PROJ-0042-add-bar.md"
assert_contains "code block path 0001" "$CONTENT" "meta/work/PROJ-0001-add-foo.md"

echo "Test: heading-line #NNNN references rewritten"
CONTENT=$(cat "$REPO/meta/plans/2026-04-01-some-plan.md")
assert_contains "heading #0042" "$CONTENT" "#PROJ-0042"
assert_contains "multi-ref heading #0001" "$CONTENT" "#PROJ-0001"

echo "Test: negative — bare fenced block NOT rewritten"
CONTENT=$(cat "$REPO/meta/research/2026-04-03-history.md")
assert_contains "bare block preserved" "$CONTENT" "meta/work/0042-add-bar.md"

echo "Test: negative — prose 0042 NOT rewritten"
assert_contains "prose 0042" "$CONTENT" "port 0042"
assert_contains "occurrences" "$CONTENT" "0042 occurrences"
assert_contains "timestamp" "$CONTENT" "2026-04-15"

echo "Test: negative — non-path numeric in tagged block NOT rewritten"
assert_contains "non-path code" "$CONTENT" "foo --id 0042"

echo "Test: non-work-item file in meta/work/ unchanged"
CONTENT=$(cat "$REPO/meta/work/notes.md")
assert_contains "notes unchanged" "$CONTENT" "non-work-item file"
assert_not_contains "notes not renamed" "$CONTENT" "PROJ"

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
grep -v "0002-rename-work-items-with-project-prefix" "$REPO2/.accelerator/state/migrations-applied" >"$REPO2/.accelerator/state/migrations-applied.tmp" || true
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
assert_contains "collision error" "$OUTPUT" "collision"
# Original file still there
assert_eq "original preserved" "1" "$([ -f "$REPO/meta/work/0001-add-foo.md" ] && echo 1 || echo 0)"

echo "Test: skip-tracking suppresses migration 0002"
REPO=$(setup_0002_repo)
(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" --skip 0002-rename-work-items-with-project-prefix)
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001_0002_DIR" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 with skip" "0" "$RC"
assert_contains "no pending" "$OUTPUT" "No pending migrations"
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
    >"$repo/.accelerator/state/migrations-applied"
  printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n' \
    >"$repo/meta/.migrations-applied"
  printf '%s\n' "$repo"
}

# Write a well-formed legacy .claude/accelerator.md: the fixture's base work:
# block plus any extra top-level frontmatter in $2. The fixture ships without a
# trailing newline, so `>>`-appending a second `---` block would fuse the two
# fences into `------` and malform the YAML — which the launcher now rejects
# loudly. Emit one block instead.
write_legacy_config() {
  local repo="$1" extra="${2:-}"
  {
    printf -- '---\nwork:\n  id_pattern: "{project}-{number:04d}"\n'
    printf '  default_project_code: PROJ\n'
    [ -n "$extra" ] && printf '%s\n' "$extra"
    printf -- '---\n'
  } >"$repo/.claude/accelerator.md"
}

# ── Test 1: dirty-tree refusal covers .accelerator/ ──────────────────────────
echo "Test: dirty-tree refusal applies to .accelerator/ changes"
REPO=$(mktemp -d "$TMPDIR_BASE/m0003-dirty-XXXXXX")
mkdir -p "$REPO/.accelerator/state"
printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n' \
  >"$REPO/.accelerator/state/migrations-applied"
git -C "$REPO" init -q
git -C "$REPO" -c user.email="test@test.com" -c user.name="Test" add .
git -C "$REPO" -c user.email="test@test.com" -c user.name="Test" commit -qm "initial"
printf '\n# extra\n' >>"$REPO/.accelerator/state/migrations-applied"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit (dirty .accelerator/)" "0" "$RC"
assert_contains "error mentions dirty tree" "$OUTPUT" "dirty"

echo ""

# ── Test 2: end-to-end move from fully-seeded legacy repo ────────────────────
echo "Test: end-to-end move — all sources reach destinations, sources absent after"
REPO=$(setup_0003_repo)
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
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
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1
JIRA_GI="$REPO/.accelerator/state/integrations/jira/.gitignore"
assert_file_exists "inner .gitignore created" "$JIRA_GI"
GI_CONTENT=$(cat "$JIRA_GI")
assert_contains "site.json rule" "$GI_CONTENT" "site.json"
assert_contains ".refresh-meta.json rule" "$GI_CONTENT" ".refresh-meta.json"
assert_contains ".lock/ rule" "$GI_CONTENT" ".lock/"
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
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1
assert_dir_exists ".accelerator/tmp created" "$REPO/.accelerator/tmp"
assert_dir_not_exists "meta/tmp absent" "$REPO/meta/tmp"
assert_file_exists "session.json moved" "$REPO/.accelerator/tmp/session.json"

echo ""

# ── Test 5: paths.tmp overridden to custom path — meta/tmp/ untouched ────────
echo "Test: paths.tmp overridden to custom path — meta/tmp/ left untouched"
REPO=$(setup_0003_repo)
write_legacy_config "$REPO" 'paths:
  tmp: custom/tmp'
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1
assert_dir_exists "meta/tmp still present" "$REPO/meta/tmp"
assert_dir_not_exists ".accelerator/tmp not created" "$REPO/.accelerator/tmp"

echo ""

# ── Test 6: paths.tmp = "meta/tmp" literal — treated as explicit override ─────
echo "Test: paths.tmp = meta/tmp literal — explicit override leaves meta/tmp untouched"
REPO=$(setup_0003_repo)
write_legacy_config "$REPO" 'paths:
  tmp: meta/tmp'
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1
assert_dir_exists "meta/tmp still present (literal override)" "$REPO/meta/tmp"
assert_dir_not_exists ".accelerator/tmp not created" "$REPO/.accelerator/tmp"

echo ""

# ── Test 6a: paths.tmp = "meta/tmp/" (trailing slash) — also treated as set ──
echo "Test: paths.tmp with trailing slash — treated as explicit override"
REPO=$(setup_0003_repo)
write_legacy_config "$REPO" 'paths:
  tmp: meta/tmp/'
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1
assert_dir_exists "meta/tmp still present (slash override)" "$REPO/meta/tmp"

echo ""

# ── Test 6b: tmp under nested non-paths block — not detected as override ──────
echo "Test: tmp under non-paths block — awk anchor prevents false positive, meta/tmp moved"
REPO=$(setup_0003_repo)
write_legacy_config "$REPO" 'some_section:
  tmp: meta/tmp'
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1
assert_dir_exists ".accelerator/tmp created (nested not detected)" "$REPO/.accelerator/tmp"
assert_dir_not_exists "meta/tmp moved away" "$REPO/meta/tmp"

echo ""

# ── Test 7: idempotency — re-run reports 0003 already applied ────────────────
echo "Test: idempotency — re-running after success reports no pending migrations"
REPO=$(setup_0003_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 on re-run" "0" "$RC"
assert_contains "no pending on re-run" "$OUTPUT" "No pending migrations."

echo ""

# ── Test 8: root .gitignore rewrite ──────────────────────────────────────────
echo "Test: root .gitignore — legacy rule replaced by anchored new rule"
REPO=$(setup_0003_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1
GI="$REPO/.gitignore"
assert_file_exists ".gitignore still exists" "$GI"
assert_eq "old unanchored rule removed" "0" \
  "$(grep -cFx '.claude/accelerator.local.md' "$GI" || true)"
assert_eq "new anchored rule present" "1" \
  "$(grep -cFx '.accelerator/config.local.md' "$GI" || true)"

echo "Test: root .gitignore new rule not duplicated on re-run"
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1
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
assert_contains "error message names the offending line" "$OUTPUT" "custom note"
# File unchanged — no destructive write
CUSTOM_LINE=$(grep -F '# custom note' "$REPO/.gitignore" || echo "")
assert_contains "original line preserved" "$CUSTOM_LINE" "# custom note"

echo ""

# ── Test 9: Jira rules removed from .gitignore ───────────────────────────────
echo "Test: root .gitignore Jira legacy rules removed"
REPO=$(setup_0003_repo)
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1
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
printf 'config.local.md\n' >"$REPO/.accelerator/.gitignore"
RC=0
OUTPUT=$(PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  bash "$MIGRATION_0003" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "no_op_pending sentinel in stdout" "$OUTPUT" "MIGRATION_RESULT: no_op_pending"

echo ""

# ── Test 11: idempotency from partial states ──────────────────────────────────
echo "Test: partial-state idempotency — config.md moved, skills/ pending — completes cleanly"
REPO=$(setup_0003_repo)
# Manually move only .claude/accelerator.md to simulate a mid-run state
mkdir -p "$REPO/.accelerator"
mv "$REPO/.claude/accelerator.md" "$REPO/.accelerator/config.md"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
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
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
assert_eq "exit 0 on partial recovery (state pending)" "0" "$RC"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied")
assert_contains "0003 recorded" "$APPLIED" "0003-relocate-accelerator-state"
assert_file_not_exists "meta/.migrations-applied removed" "$REPO/meta/.migrations-applied"

echo ""

# ── Test 11a: both-present file → merge with source-wins (no abort) ──────────
echo "Test: both .claude/accelerator.md and .accelerator/config.md exist — source wins, no abort"
REPO=$(setup_0003_repo)
mkdir -p "$REPO/.accelerator"
printf 'different content\n' >"$REPO/.accelerator/config.md"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 — merge, no abort" "0" "$RC"
assert_file_not_exists ".claude/accelerator.md removed after move" "$REPO/.claude/accelerator.md"
CFG=$(cat "$REPO/.accelerator/config.md")
assert_contains "destination holds source content (source wins)" "$CFG" "default_project_code: PROJ"
assert_not_contains "pre-existing destination overwritten by source" "$CFG" "different content"

echo ""

# ── Test 11b: both-present directory → recursive merge, source-wins on leaf ──
echo "Test: meta/tmp/ merges into existing .accelerator/tmp/ (dir merge, source-wins on overlap)"
REPO=$(setup_0003_repo)
printf 'SRC-KEEP\n' >"$REPO/meta/tmp/keep.md"
printf 'SRC-NEW\n' >"$REPO/meta/tmp/new.md"
mkdir -p "$REPO/.accelerator/tmp"
printf 'DEST-KEEP\n' >"$REPO/.accelerator/tmp/keep.md"
printf 'DEST-ONLY\n' >"$REPO/.accelerator/tmp/dest-only.md"
RC=0
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1 || RC=$?
assert_eq "exit 0 — dir merge, no abort" "0" "$RC"
assert_dir_not_exists "meta/tmp removed after merge" "$REPO/meta/tmp"
assert_file_content_eq "new source file merged in" "$REPO/.accelerator/tmp/new.md" "SRC-NEW"
assert_file_content_eq "leaf collision: source content wins" "$REPO/.accelerator/tmp/keep.md" "SRC-KEEP"
assert_file_content_eq "destination-only file preserved" "$REPO/.accelerator/tmp/dest-only.md" "DEST-ONLY"
assert_file_exists "fixture session.json merged in" "$REPO/.accelerator/tmp/session.json"

echo ""

# ── Test 12: state-file merge with deduplication ──────────────────────────────
echo "Test: state-file merge — meta/.migrations-applied lines preserved and deduplicated"
REPO=$(setup_0003_repo)
# setup_0003_repo seeds both new and legacy state with 0001+0002
# After migration, new state file should be union {0001, 0002, 0003}
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied")
assert_contains "0001 preserved in merged state" "$APPLIED" "0001-rename-tickets-to-work"
assert_contains "0002 preserved in merged state" "$APPLIED" "0002-rename-work-items-with-project-prefix"
assert_contains "0003 recorded" "$APPLIED" "0003-relocate-accelerator-state"
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
cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" >/dev/null 2>&1
assert_dir_not_exists ".accelerator/templates not pre-created" "$REPO/.accelerator/templates"

echo ""

# ── Test 14: pinned-override warning for paths.templates and paths.integrations ─
echo "Test: pinned-override warning emitted for paths.templates and paths.integrations"
REPO=$(setup_0003_repo)
write_legacy_config "$REPO" 'paths:
  templates: custom/templates
  integrations: custom/ints'
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 (warning not error)" "0" "$RC"
assert_contains "warning names templates key" "$OUTPUT" "paths.templates"
assert_contains "warning names integrations key" "$OUTPUT" "paths.integrations"
assert_contains "warning names templates value" "$OUTPUT" "custom/templates"
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

# ════════════════════════════════════════════════════════════════════════════
echo "=== Migration 0004: restructure meta/research/ into subject subcategories ==="
echo ""

FIXTURE_0004="$SCRIPT_DIR/test-fixtures/0004"
MIGRATION_0004="$MIGRATIONS_DIR/0004-restructure-meta-research-into-subject-subcategories.sh"

# Stand up a temp repo seeded from one of the 0004 fixture trees.
# Marks 0001-0003 applied so the driver runs only 0004.
setup_0004_repo() {
  local fixture="$1"
  local repo
  repo=$(mktemp -d "$TMPDIR_BASE/m0004-XXXXXX")
  cp -R "$FIXTURE_0004/$fixture/." "$repo/"
  mkdir -p "$repo/.accelerator/state"
  printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n0003-relocate-accelerator-state\n' \
    >"$repo/.accelerator/state/migrations-applied"
  printf '%s\n' "$repo"
}

# Run 0004 directly. 0004 no longer has an internal dirty-tree / no-VCS
# pre-flight, so no bypass env var is needed — fixtures without .jj/.git proceed.
run_0004() {
  local repo="$1"
  shift || true
  PROJECT_ROOT="$repo" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
    bash "$MIGRATION_0004" "$@"
}

# ── default-layout: files move from meta/research/ to meta/research/codebase/
echo "Test: default-layout — flat research files move to meta/research/codebase/"
REPO=$(setup_0004_repo default-layout)
run_0004 "$REPO" >/dev/null 2>&1
assert_file_exists "research file moved" "$REPO/meta/research/codebase/2026-01-01-example.md"
assert_file_not_exists "legacy flat research absent" "$REPO/meta/research/2026-01-01-example.md"

echo "Test: default-layout — meta/research/.gitkeep stays in place (parent marker preserved)"
REPO=$(setup_0004_repo default-layout)
run_0004 "$REPO" >/dev/null 2>&1
assert_file_exists "parent .gitkeep preserved" "$REPO/meta/research/.gitkeep"

echo "Test: default-layout — .gitkeep created in every destination subdir"
REPO=$(setup_0004_repo default-layout)
run_0004 "$REPO" >/dev/null 2>&1
assert_file_exists "codebase .gitkeep" "$REPO/meta/research/codebase/.gitkeep"
assert_file_exists "issues .gitkeep" "$REPO/meta/research/issues/.gitkeep"
assert_file_exists "design-inventories .gitkeep" "$REPO/meta/research/design-inventories/.gitkeep"
assert_file_exists "design-gaps .gitkeep" "$REPO/meta/research/design-gaps/.gitkeep"

echo "Test: default-layout — design-inventories directory moves to meta/research/design-inventories/"
REPO=$(setup_0004_repo default-layout)
run_0004 "$REPO" >/dev/null 2>&1
assert_file_exists "inventory moved" "$REPO/meta/research/design-inventories/2026-05-06-x/inventory.md"
assert_file_exists "screenshot moved" "$REPO/meta/research/design-inventories/2026-05-06-x/screenshots/01.png"
assert_dir_not_exists "legacy meta/design-inventories removed" "$REPO/meta/design-inventories"

echo "Test: default-layout — design-gaps file moves to meta/research/design-gaps/"
REPO=$(setup_0004_repo default-layout)
run_0004 "$REPO" >/dev/null 2>&1
assert_file_exists "gap file moved" "$REPO/meta/research/design-gaps/2026-05-06-x.md"
assert_dir_not_exists "legacy meta/design-gaps removed" "$REPO/meta/design-gaps"

echo "Test: default-layout — .DS_Store is swept (not preserved into new layout)"
REPO=$(setup_0004_repo default-layout)
run_0004 "$REPO" >/dev/null 2>&1
assert_file_not_exists ".DS_Store swept" "$REPO/meta/design-inventories/.DS_Store"

echo "Test: research-override-only — files move from docs/research/ to docs/research/codebase/"
REPO=$(setup_0004_repo research-override-only)
run_0004 "$REPO" >/dev/null 2>&1
assert_file_exists "override research moved" "$REPO/docs/research/codebase/2026-01-01-example.md"
assert_file_not_exists "override flat absent" "$REPO/docs/research/2026-01-01-example.md"

echo "Test: all-overridden — design-inventories DO NOT MOVE (override honored)"
REPO=$(setup_0004_repo all-overridden)
run_0004 "$REPO" >/dev/null 2>&1
assert_file_exists "inv override preserved" "$REPO/assets/inv/2026-05-06-x/inventory.md"
assert_dir_not_exists "no nested inv created at default" "$REPO/meta/research/design-inventories"

echo "Test: all-overridden — design-gaps DO NOT MOVE (override honored)"
REPO=$(setup_0004_repo all-overridden)
run_0004 "$REPO" >/dev/null 2>&1
assert_file_exists "gaps override preserved" "$REPO/gaps/2026-05-06-x.md"

echo "Test: mixed-config — refuses to proceed (legacy + renamed both present)"
REPO=$(setup_0004_repo mixed-config)
RC=0
OUTPUT=$(run_0004 "$REPO" 2>&1) || RC=$?
assert_neq "non-zero exit on mixed state" "0" "$RC"
assert_contains "diagnostic names paths.research" "$OUTPUT" "paths.research"
assert_contains "diagnostic names paths.research_codebase" "$OUTPUT" "paths.research_codebase"
assert_file_exists "no files moved on refusal" "$REPO/meta/research/foo.md"
assert_dir_not_exists "no codebase dir created" "$REPO/meta/research/codebase"

echo "Test: destination collision — source overwrites target (merge, source-wins)"
REPO=$(setup_0004_repo default-layout)
mkdir -p "$REPO/meta/research/codebase"
printf 'pre-existing\n' >"$REPO/meta/research/codebase/2026-01-01-example.md"
RC=0
OUTPUT=$(run_0004 "$REPO" 2>&1) || RC=$?
assert_eq "exit 0 — merge, no abort" "0" "$RC"
assert_file_not_exists "source removed after move" "$REPO/meta/research/2026-01-01-example.md"
# The moved file is also inbound-link rewritten by Step 3, so assert source-wins
# by distinctive markers rather than a byte-compare against the pre-move source.
MOVED_CONTENT=$(cat "$REPO/meta/research/codebase/2026-01-01-example.md")
assert_contains "destination holds moved source content" "$MOVED_CONTENT" "# Example research"
assert_not_contains "pre-existing destination overwritten by source" "$MOVED_CONTENT" "pre-existing"

echo "Test: idempotent — re-running default-layout yields no further changes"
REPO=$(setup_0004_repo default-layout)
run_0004 "$REPO" >/dev/null 2>&1
BEFORE=$(find "$REPO" -type f -print0 | sort -z | xargs -0 md5sum 2>/dev/null | md5sum)
run_0004 "$REPO" >/dev/null 2>&1
AFTER=$(find "$REPO" -type f -print0 | sort -z | xargs -0 md5sum 2>/dev/null | md5sum)
assert_eq "second-run filesystem hash equals first-run" "$BEFORE" "$AFTER"

echo "Test: local-config-only — override read from config.local.md, files move accordingly"
REPO=$(setup_0004_repo local-config-only)
run_0004 "$REPO" >/dev/null 2>&1
assert_file_exists "local-overridden research moved" "$REPO/docs/research/codebase/foo.md"
assert_file_not_exists "local-overridden flat absent" "$REPO/docs/research/foo.md"

echo "Test: no-VCS success — 0004 runs without any VCS dir or force env var"
REPO=$(setup_0004_repo default-layout)
RC=0
OUTPUT=$(PROJECT_ROOT="$REPO" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$MIGRATION_0004" 2>&1) || RC=$?
assert_eq "exit 0 with no VCS and no force env" "0" "$RC"
assert_file_exists "research files relocated into codebase/" \
  "$REPO/meta/research/codebase/2026-01-01-example.md"

echo "Test: VCS-present clean tree — 0004 moves files and Step 3 inbound-link rewrite runs"
REPO=$(setup_0004_repo default-layout)
git -C "$REPO" init -q
git -C "$REPO" -c user.email="t@t.com" -c user.name="T" add -A
git -C "$REPO" -c user.email="t@t.com" -c user.name="T" commit -qm "init"
RC=0
run_0004 "$REPO" >/dev/null 2>&1 || RC=$?
assert_eq "exit 0 in committed-clean git repo" "0" "$RC"
assert_file_exists "research moved into codebase/" "$REPO/meta/research/codebase/2026-01-01-example.md"
MOVED=$(cat "$REPO/meta/research/codebase/2026-01-01-example.md")
assert_contains "Step 3 rewrote inbound research link (build_scan_corpus walked)" \
  "$MOVED" "meta/research/codebase/2026-01-02-sibling.md"

echo "Test: mid-batch sibling-dirty — 0004 converges after 0001/0003 dirty the tree"
# Through the orchestrator: a committed-clean repo needing 0001+0003+0004. The
# single per-invocation clean-tree gate passes at the start; 0001/0003 then
# dirty the tree before 0004 runs. 0004 must no longer re-police the tree.
REPO=$(mktemp -d "$TMPDIR_BASE/m0004-batch-XXXXXX")
mkdir -p "$REPO/meta/tickets" "$REPO/meta/research" "$REPO/.claude"
printf -- '---\nticket_id: 0001\n---\n\n# t\n' >"$REPO/meta/tickets/0001-foo.md"
printf -- '---\nwork-item: "0001"\n---\n\n# Example research\n' \
  >"$REPO/meta/research/2026-01-01-example.md"
printf -- '---\n---\n' >"$REPO/.claude/accelerator.md"
git -C "$REPO" init -q
git -C "$REPO" -c user.email="t@t.com" -c user.name="T" add -A
git -C "$REPO" -c user.email="t@t.com" -c user.name="T" commit -qm "init"
RC=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$DRIVER" 2>&1) || RC=$?
assert_eq "exit 0 — full batch converges in one pass" "0" "$RC"
assert_not_contains "no mid-batch dirty-tree abort" "$OUTPUT" "scan corpus has uncommitted changes"
# Positive postcondition: 0004 actually relocated the research file after the
# sibling-dirtied batch (proves convergence, not merely absence of one error).
assert_file_exists "research relocated into codebase/" \
  "$REPO/meta/research/codebase/2026-01-01-example.md"
assert_dir_not_exists "0001 renamed legacy tickets dir" "$REPO/meta/tickets"

echo ""

# ── Phase 4 — config-key rewrites & notifications ───────────────────────────

echo "Test: research-override-only — paths.research_codebase value is OLD/codebase"
REPO=$(setup_0004_repo research-override-only)
run_0004 "$REPO" >/dev/null 2>&1
CFG=$(cat "$REPO/.accelerator/config.md")
assert_contains "research_codebase value rewritten" "$CFG" "research_codebase: docs/research/codebase"
assert_not_contains "legacy paths.research removed" "$CFG" "research: docs/research$(printf '\n')"

echo "Test: research-override-only — paths.research_issues injected with OLD/issues"
REPO=$(setup_0004_repo research-override-only)
run_0004 "$REPO" >/dev/null 2>&1
CFG=$(cat "$REPO/.accelerator/config.md")
assert_contains "research_issues injected" "$CFG" "research_issues: docs/research/issues"

echo "Test: all-overridden — design-inv/design-gaps preserved verbatim"
REPO=$(setup_0004_repo all-overridden)
run_0004 "$REPO" >/dev/null 2>&1
CFG=$(cat "$REPO/.accelerator/config.md")
assert_contains "research_design_inventories verbatim" "$CFG" "research_design_inventories: assets/inv"
assert_contains "research_design_gaps verbatim" "$CFG" "research_design_gaps: gaps"

echo "Test: default-layout — config.md byte-identical (no paths-block injection)"
REPO=$(setup_0004_repo default-layout)
BEFORE=$(cat "$REPO/.accelerator/config.md")
run_0004 "$REPO" >/dev/null 2>&1
AFTER=$(cat "$REPO/.accelerator/config.md")
assert_eq "config unchanged when no overrides" "$BEFORE" "$AFTER"
assert_not_contains "no research_issues injected" "$AFTER" "research_issues"

echo "Test: rename notification line emitted for each rewritten key"
REPO=$(setup_0004_repo all-overridden)
OUTPUT=$(run_0004 "$REPO" 2>&1)
assert_contains "research rename notice" "$OUTPUT" "renamed paths.research → paths.research_codebase"
assert_contains "design_inventories rename notice" "$OUTPUT" "renamed paths.design_inventories → paths.research_design_inventories"
assert_contains "design_gaps rename notice" "$OUTPUT" "renamed paths.design_gaps → paths.research_design_gaps"

echo "Test: default-layout — no rename notifications emitted (no overrides)"
REPO=$(setup_0004_repo default-layout)
OUTPUT=$(run_0004 "$REPO" 2>&1)
assert_not_contains "no rename notice" "$OUTPUT" "renamed paths.research"

echo "Test: local-config-only — rewrite applies only to config.local.md"
REPO=$(setup_0004_repo local-config-only)
BEFORE_TEAM=$(cat "$REPO/.accelerator/config.md")
run_0004 "$REPO" >/dev/null 2>&1
AFTER_TEAM=$(cat "$REPO/.accelerator/config.md")
LOCAL=$(cat "$REPO/.accelerator/config.local.md")
assert_eq "team config unchanged" "$BEFORE_TEAM" "$AFTER_TEAM"
assert_contains "local config rewritten" "$LOCAL" "research_codebase: docs/research/codebase"

echo "Test: config backup .bak file created before first rewrite"
REPO=$(setup_0004_repo research-override-only)
run_0004 "$REPO" >/dev/null 2>&1
assert_file_exists "config.md.0004.bak created" "$REPO/.accelerator/config.md.0004.bak"

echo "Test: config rewrite is idempotent — second run yields no change"
REPO=$(setup_0004_repo research-override-only)
run_0004 "$REPO" >/dev/null 2>&1
BEFORE=$(cat "$REPO/.accelerator/config.md")
run_0004 "$REPO" >/dev/null 2>&1
AFTER=$(cat "$REPO/.accelerator/config.md")
assert_eq "config byte-identical on re-run" "$BEFORE" "$AFTER"

echo "Test: templates.research → templates.codebase-research rename when overridden"
REPO=$(setup_0004_repo default-layout)
# Add templates.research override after fixture copy
cat >"$REPO/.accelerator/config.md" <<'EOF'
---
templates:
  research: custom/templates/my-research.md
---
EOF
run_0004 "$REPO" >/dev/null 2>&1
CFG=$(cat "$REPO/.accelerator/config.md")
assert_contains "templates.codebase-research key" "$CFG" "codebase-research: custom/templates/my-research.md"
assert_not_contains "legacy templates.research key absent" "$CFG" "  research:"

echo ""

# ── Phase 5 — inbound-link rewriting ────────────────────────────────────────

echo "Test: inbound — markdown link rewritten to research/codebase"
REPO=$(setup_0004_repo inbound-corpus)
run_0004 "$REPO" >/dev/null 2>&1
CONTENT=$(cat "$REPO/meta/work/0050.md")
assert_contains "markdown link rewritten" "$CONTENT" "[research](meta/research/codebase/2026-05-08-foo.md)"

echo "Test: inbound — frontmatter scalar rewritten"
REPO=$(setup_0004_repo inbound-corpus)
run_0004 "$REPO" >/dev/null 2>&1
CONTENT=$(cat "$REPO/meta/work/0050.md")
assert_contains "frontmatter scalar rewritten" "$CONTENT" "research: meta/research/codebase/2026-05-08-foo.md"

echo "Test: inbound — inline backtick reference rewritten"
REPO=$(setup_0004_repo inbound-corpus)
run_0004 "$REPO" >/dev/null 2>&1
CONTENT=$(cat "$REPO/meta/work/0050.md")
# shellcheck disable=SC2016 # single-quoted literal expected-content string; the backticks are Markdown, intentionally not shell-expanded
assert_contains "inline backtick rewritten" "$CONTENT" '`meta/research/design-inventories/2026-05-06-x/inventory.md`'

echo "Test: inbound — bare narrative reference rewritten"
REPO=$(setup_0004_repo inbound-corpus)
run_0004 "$REPO" >/dev/null 2>&1
CONTENT=$(cat "$REPO/meta/work/0050.md")
assert_contains "narrative gap rewritten" "$CONTENT" "meta/research/design-gaps/2026-05-06-x.md"

echo "Test: inbound — fenced code-block paths rewritten"
REPO=$(setup_0004_repo inbound-corpus)
run_0004 "$REPO" >/dev/null 2>&1
CONTENT=$(cat "$REPO/meta/plans/2026-05-09-p.md")
assert_contains "code-block research path" "$CONTENT" "meta/research/codebase/2026-05-08-foo.md"
assert_contains "code-block gap path" "$CONTENT" "meta/research/design-gaps/2026-05-06-x.md"

echo "Test: inbound — boundary anchor prevents meta/research-templates/ rewrite"
REPO=$(setup_0004_repo inbound-corpus)
run_0004 "$REPO" >/dev/null 2>&1
CONTENT=$(cat "$REPO/meta/work/0050.md")
assert_contains "research-templates untouched" "$CONTENT" "meta/research-templates/foo.md"
assert_contains "researchers.md untouched" "$CONTENT" "meta/researchers.md"

echo "Test: inbound — moved-file internal cross-link rewritten"
REPO=$(setup_0004_repo inbound-corpus)
run_0004 "$REPO" >/dev/null 2>&1
assert_file_exists "moved research file" "$REPO/meta/research/codebase/2026-05-08-foo.md"
CONTENT=$(cat "$REPO/meta/research/codebase/2026-05-08-foo.md")
assert_contains "internal cross-link rewritten" "$CONTENT" "meta/research/codebase/2026-05-08-bar.md"

echo "Test: inbound — Step 3 banner reports scan corpus size"
REPO=$(setup_0004_repo inbound-corpus)
OUTPUT=$(run_0004 "$REPO" 2>&1)
assert_contains "scan banner emitted" "$OUTPUT" "Step 3: scanning"

echo "Test: inbound — idempotent (second run yields no further changes)"
REPO=$(setup_0004_repo inbound-corpus)
run_0004 "$REPO" >/dev/null 2>&1
BEFORE=$(find "$REPO" -type f -print0 | sort -z | xargs -0 md5sum 2>/dev/null | md5sum)
run_0004 "$REPO" >/dev/null 2>&1
AFTER=$(find "$REPO" -type f -print0 | sort -z | xargs -0 md5sum 2>/dev/null | md5sum)
assert_eq "inbound rewrite idempotent" "$BEFORE" "$AFTER"

echo ""

# ════════════════════════════════════════════════════════════════════════════
echo "=== Migration 0005: rename work-item type field to kind ==="
echo ""

FIXTURE_0005="$SCRIPT_DIR/test-fixtures/0005"
MIGRATION_0005="$MIGRATIONS_DIR/0005-rename-work-item-type-to-kind.sh"

ONLY_0005_DIR="$TMPDIR_BASE/only-0005-migrations"
mkdir -p "$ONLY_0005_DIR"
cp "$MIGRATION_0005" "$ONLY_0005_DIR/"

# setup_0005_repo: copies a fixture from test-fixtures/0005/ into mktemp.
# Fixtures contain no `.jj` or `.git` dir.
setup_0005_repo() {
  local scenario="$1"
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-0005-XXXXXX")
  cp -R "$FIXTURE_0005/$scenario/." "$repo_dir/"
  echo "$repo_dir"
}

# Run 0005 via the driver (with no-VCS bypass) so the state file is updated.
# The repo is consumed positionally for `cd`; remaining args (if any) forward
# to the driver. (The driver takes PROJECT_ROOT from the cwd, not a positional —
# and as of 0117 it rejects unrecognised positionals, so the repo must NOT leak
# through as one.)
run_0005_driver() {
  local repo="$1"
  shift
  cd "$repo" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
    ACCELERATOR_MIGRATIONS_DIR="$ONLY_0005_DIR" \
    ACCELERATOR_MIGRATE_FORCE=1 \
    bash "$DRIVER" "$@"
}

# ── default-layout ───────────────────────────────────────────────────────────
echo "Test: default-layout — type: renamed to kind:, **Type**: renamed to **Kind**:"
REPO=$(setup_0005_repo default-layout)
RC=0
OUTPUT=$(run_0005_driver "$REPO" 2>&1) || RC=$?
assert_eq "exit 0" "0" "$RC"
WI="$REPO/meta/work/0001-foo.md"
CONTENT=$(cat "$WI")
assert_contains "kind: story present" "$CONTENT" "kind: story"
assert_not_contains "no type: line" "$CONTENT" "type:"
assert_contains "**Kind**: Story present" "$CONTENT" "**Kind**: Story"
assert_not_contains "no **Type**: line" "$CONTENT" "**Type**:"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "state file has 0005" "$APPLIED" "0005-rename-work-item-type-to-kind"

echo ""

# ── legacy-adr-task ──────────────────────────────────────────────────────────
echo "Test: legacy-adr-task — frontmatter renamed, no body label before or after"
REPO=$(setup_0005_repo legacy-adr-task)
RC=0
run_0005_driver "$REPO" >/dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
WI="$REPO/meta/work/0001-adr-creation.md"
CONTENT=$(cat "$WI")
assert_contains "kind: adr-creation-task" "$CONTENT" "kind: adr-creation-task"
assert_not_contains "no type: line" "$CONTENT" "type:"
assert_not_contains "no **Kind**: body label" "$CONTENT" "**Kind**:"
assert_not_contains "no **Type**: body label" "$CONTENT" "**Type**:"

echo ""

# ── partial-prior-run (matching values) ──────────────────────────────────────
echo "Test: partial-prior-run — stale type: removed; no divergence warning"
REPO=$(setup_0005_repo partial-prior-run)
RC=0
STDERR_FILE=$(mktemp)
run_0005_driver "$REPO" 2>"$STDERR_FILE" >/dev/null || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
WI="$REPO/meta/work/0001-foo.md"
CONTENT=$(cat "$WI")
KIND_LINES=$(grep -c '^kind:' "$WI" || true)
assert_eq "exactly one kind: line" "1" "$KIND_LINES"
assert_not_contains "no type: line" "$CONTENT" "type:"
assert_not_contains "no divergence warning on match" "$STDERR" "divergent type/kind"

echo ""

# ── partial-prior-run-divergent ──────────────────────────────────────────────
echo "Test: partial-prior-run-divergent — kind: wins; stderr warning emitted"
REPO=$(setup_0005_repo partial-prior-run-divergent)
RC=0
STDERR_FILE=$(mktemp)
run_0005_driver "$REPO" 2>"$STDERR_FILE" >/dev/null || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
WI="$REPO/meta/work/0001-foo.md"
CONTENT=$(cat "$WI")
assert_contains "kind: story remains" "$CONTENT" "kind: story"
assert_not_contains "kind: bug not present" "$CONTENT" "kind: bug"
assert_not_contains "no type: line" "$CONTENT" "type:"
assert_contains "divergence warning fired" "$STDERR" "divergent type/kind"

echo ""

# ── partial-prior-run-body-label (matching) ──────────────────────────────────
echo "Test: partial-prior-run-body-label — stale **Type**: removed; no divergence warning"
REPO=$(setup_0005_repo partial-prior-run-body-label)
RC=0
STDERR_FILE=$(mktemp)
run_0005_driver "$REPO" 2>"$STDERR_FILE" >/dev/null || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
WI="$REPO/meta/work/0001-foo.md"
CONTENT=$(cat "$WI")
assert_contains "**Kind**: Story present" "$CONTENT" "**Kind**: Story"
assert_not_contains "no **Type**: line" "$CONTENT" "**Type**:"
assert_contains "frontmatter kind: unchanged" "$CONTENT" "kind: story"
assert_not_contains "no body-label divergence warning" "$STDERR" "divergent **Type**/**Kind**"

echo ""

# ── partial-prior-run-body-label-divergent ───────────────────────────────────
echo "Test: partial-prior-run-body-label-divergent — **Kind**: wins; stderr warning emitted"
REPO=$(setup_0005_repo partial-prior-run-body-label-divergent)
RC=0
STDERR_FILE=$(mktemp)
run_0005_driver "$REPO" 2>"$STDERR_FILE" >/dev/null || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
WI="$REPO/meta/work/0001-foo.md"
CONTENT=$(cat "$WI")
assert_contains "**Kind**: Story remains" "$CONTENT" "**Kind**: Story"
assert_not_contains "**Kind**: Bug not present" "$CONTENT" "**Kind**: Bug"
assert_not_contains "no **Type**: line" "$CONTENT" "**Type**:"
assert_contains "body-label divergence warning fired" "$STDERR" "divergent **Type**/**Kind**"

echo ""

# ── paths-override ───────────────────────────────────────────────────────────
echo "Test: paths-override — docs/work file renamed; meta/work not created"
REPO=$(setup_0005_repo paths-override)
RC=0
run_0005_driver "$REPO" >/dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
WI="$REPO/docs/work/0001-foo.md"
CONTENT=$(cat "$WI")
assert_contains "kind: story present" "$CONTENT" "kind: story"
assert_not_contains "no type: line" "$CONTENT" "type:"
assert_contains "**Kind**: Story present" "$CONTENT" "**Kind**: Story"
assert_dir_not_exists "meta/work not created" "$REPO/meta/work"

echo ""

# ── paths-override-missing ───────────────────────────────────────────────────
echo "Test: paths-override-missing — warning emitted; exit 0; migration recorded"
REPO=$(setup_0005_repo paths-override-missing)
RC=0
STDERR_FILE=$(mktemp)
run_0005_driver "$REPO" 2>"$STDERR_FILE" >/dev/null || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
assert_contains "missing-dir warning" "$STDERR" "work directory does not exist"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "0005 recorded" "$APPLIED" "0005-rename-work-item-type-to-kind"

echo ""

# ── body-label-only ──────────────────────────────────────────────────────────
echo "Test: body-label-only — stale **Type**: rewritten; frontmatter unchanged"
REPO=$(setup_0005_repo body-label-only)
RC=0
run_0005_driver "$REPO" >/dev/null 2>&1 || RC=$?
assert_eq "exit 0" "0" "$RC"
WI="$REPO/meta/work/0001-foo.md"
CONTENT=$(cat "$WI")
assert_contains "**Kind**: Story present" "$CONTENT" "**Kind**: Story"
assert_not_contains "no **Type**: line" "$CONTENT" "**Type**:"
assert_contains "frontmatter kind: unchanged" "$CONTENT" "kind: story"

echo ""

# ── empty-work-dir ───────────────────────────────────────────────────────────
echo "Test: empty-work-dir — exit 0; no .md files created"
REPO=$(setup_0005_repo empty-work-dir)
RC=0
STDOUT_FILE=$(mktemp)
run_0005_driver "$REPO" >"$STDOUT_FILE" 2>&1 || RC=$?
STDOUT=$(cat "$STDOUT_FILE")
rm -f "$STDOUT_FILE"
assert_eq "exit 0" "0" "$RC"
MD_COUNT=$(find "$REPO/docs/work" -name '*.md' -type f 2>/dev/null | wc -l | tr -d ' ')
assert_eq "no .md files created" "0" "$MD_COUNT"
assert_contains "0005 reports 0 rewrites under docs/work" "$STDOUT" "0005: rewrote 0 file(s) under docs/work"

echo ""

# ── idempotent ───────────────────────────────────────────────────────────────
echo "Test: idempotent — second run is byte-identical no-op"
REPO=$(setup_0005_repo default-layout)
run_0005_driver "$REPO" >/dev/null 2>&1
HASH1=$(tree_hash "$REPO/meta")
RC=0
run_0005_driver "$REPO" >/dev/null 2>&1 || RC=$?
HASH2=$(tree_hash "$REPO/meta")
assert_eq "exit 0 second run" "0" "$RC"
assert_eq "byte-identical" "$HASH1" "$HASH2"

echo ""

# ════════════════════════════════════════════════════════════════════════════
echo "=== Migration 0006: canonicalise work-item -> work_item_id; researcher -> author ==="
echo ""

FIXTURE_0006="$SCRIPT_DIR/test-fixtures/0006"
MIGRATION_0006="$MIGRATIONS_DIR/0006-canonicalise-work-item-id-and-author.sh"

ONLY_0006_DIR="$TMPDIR_BASE/only-0006-migrations"
mkdir -p "$ONLY_0006_DIR"
cp "$MIGRATION_0006" "$ONLY_0006_DIR/"

setup_0006_repo() {
  local scenario="$1"
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-0006-XXXXXX")
  cp -R "$FIXTURE_0006/$scenario/." "$repo_dir/"
  echo "$repo_dir"
}

run_0006_driver() {
  local repo="$1"
  shift
  (
    cd "$repo" &&
      CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
        ACCELERATOR_MIGRATIONS_DIR="$ONLY_0006_DIR" \
        ACCELERATOR_MIGRATE_FORCE=1 \
        bash "$DRIVER" "$@" 2>&1
  )
}

# Convenience: split combined stdout+stderr by capturing them separately.
run_0006_driver_split() {
  local repo="$1"
  shift
  local stdout_file="$1"
  shift
  local stderr_file="$1"
  shift
  (
    cd "$repo" &&
      CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
        ACCELERATOR_MIGRATIONS_DIR="$ONLY_0006_DIR" \
        ACCELERATOR_MIGRATE_FORCE=1 \
        bash "$DRIVER" "$@" >"$stdout_file" 2>"$stderr_file"
  )
}

# ── default-layout ────────────────────────────────────────────────────────
echo "Test: default-layout — clean rename across all three corpora"
REPO=$(setup_0006_repo default-layout)
RC=0
OUTPUT=$(run_0006_driver "$REPO") || RC=$?
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
RESEARCH="$REPO/meta/research/codebase/2026-01-01-foo.md"
RCA="$REPO/meta/research/issues/2026-01-02-bar.md"
assert_contains "plan has work_item_id" "$(cat "$PLAN")" 'work_item_id: "0042"'
assert_not_contains "plan no work-item:" "$(cat "$PLAN")" 'work-item:'
assert_contains "research has author:" "$(cat "$RESEARCH")" "author: Toby Clemson"
assert_contains "research has **Author**:" "$(cat "$RESEARCH")" "**Author**: Toby Clemson"
assert_not_contains "research no researcher:" "$(cat "$RESEARCH")" "researcher:"
assert_not_contains "research no **Researcher**:" "$(cat "$RESEARCH")" "**Researcher**:"
assert_contains "RCA has author:" "$(cat "$RCA")" "author: Toby Clemson"
assert_contains "RCA has **Author**:" "$(cat "$RCA")" "**Author**: Toby Clemson"
assert_not_contains "RCA no researcher:" "$(cat "$RCA")" "researcher:"
assert_contains "stdout reports plans rewrite" "$OUTPUT" "0006: rewrote 1 file(s) under meta/plans"
assert_contains "stdout reports research_codebase rewrite" "$OUTPUT" "0006: rewrote 1 file(s) under meta/research/codebase"
assert_contains "stdout reports research_issues rewrite" "$OUTPUT" "0006: rewrote 1 file(s) under meta/research/issues"

echo ""

# ── unquoted-work-item ────────────────────────────────────────────────────
echo "Test: unquoted-work-item — value normalised to double-quoted"
REPO=$(setup_0006_repo unquoted-work-item)
RC=0
OUTPUT=$(run_0006_driver "$REPO") || RC=$?
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
assert_contains "value quoted" "$(cat "$PLAN")" 'work_item_id: "0042"'
assert_contains "stdout reports 1 rewrite" "$OUTPUT" "0006: rewrote 1 file(s) under meta/plans"

echo ""

# ── single-quoted-work-item ───────────────────────────────────────────────
echo "Test: single-quoted-work-item — re-wrapped as double-quoted"
REPO=$(setup_0006_repo single-quoted-work-item)
RC=0
run_0006_driver "$REPO" >/dev/null || RC=$?
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
assert_contains "double-quoted value" "$(cat "$PLAN")" 'work_item_id: "0042"'

echo ""

# ── no-whitespace-work-item ───────────────────────────────────────────────
echo "Test: no-whitespace-work-item — colon-no-space handled"
REPO=$(setup_0006_repo no-whitespace-work-item)
RC=0
run_0006_driver "$REPO" >/dev/null || RC=$?
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
assert_contains "canonical form" "$(cat "$PLAN")" 'work_item_id: "0042"'

echo ""

# ── inline-comment-work-item ──────────────────────────────────────────────
echo "Test: inline-comment-work-item — REFUSED, line preserved, stderr warns"
REPO=$(setup_0006_repo inline-comment-work-item)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDOUT=$(cat "$STDOUT_FILE")
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
assert_contains "legacy line preserved" "$(cat "$PLAN")" "work-item: 0042 # see TRELLO-91"
assert_contains "stderr REFUSE warning" "$STDERR" "0006-REFUSE"

# Three-run idempotence
HASH1=$(tree_hash "$REPO/meta")
run_0006_driver "$REPO" >/dev/null 2>&1 || true
HASH2=$(tree_hash "$REPO/meta")
run_0006_driver "$REPO" >/dev/null 2>&1 || true
HASH3=$(tree_hash "$REPO/meta")
assert_eq "refused idempotent run 2" "$HASH1" "$HASH2"
assert_eq "refused idempotent run 3" "$HASH2" "$HASH3"

echo ""

# ── embedded-quote-work-item ──────────────────────────────────────────────
echo "Test: embedded-quote-work-item — REFUSED, line preserved, stderr warns"
REPO=$(setup_0006_repo embedded-quote-work-item)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
assert_contains "legacy line preserved" "$(cat "$PLAN")" 'work-item: foo"bar'
assert_contains "stderr REFUSE warning" "$STDERR" "0006-REFUSE"
HASH1=$(tree_hash "$REPO/meta")
run_0006_driver "$REPO" >/dev/null 2>&1 || true
HASH2=$(tree_hash "$REPO/meta")
assert_eq "refused idempotent" "$HASH1" "$HASH2"

echo ""

# ── trailing-whitespace-work-item ─────────────────────────────────────────
echo "Test: trailing-whitespace-work-item — whitespace stripped, no false refuse"
REPO=$(setup_0006_repo trailing-whitespace-work-item)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
PLAN_LINE=$(grep '^work_item_id:' "$PLAN" || true)
assert_eq "exact canonical line" 'work_item_id: "0042"' "$PLAN_LINE"
assert_not_contains "no REFUSE for quoted-with-trailing-ws" "$STDERR" "0006-REFUSE"

echo ""

# ── empty-work-item-value ─────────────────────────────────────────────────
echo "Test: empty-work-item-value — empty preserved as 'work_item_id:'"
REPO=$(setup_0006_repo empty-work-item-value)
RC=0
run_0006_driver "$REPO" >/dev/null || RC=$?
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
PLAN_LINE=$(grep -E '^work_item_id:' "$PLAN" || true)
assert_eq "empty preserved verbatim" "work_item_id:" "$PLAN_LINE"

echo ""

# ── empty-work-item-value-trailing-ws ─────────────────────────────────────
echo "Test: empty-work-item-value-trailing-ws — trailing ws stripped"
REPO=$(setup_0006_repo empty-work-item-value-trailing-ws)
RC=0
run_0006_driver "$REPO" >/dev/null || RC=$?
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
PLAN_LINE=$(grep -E '^work_item_id:' "$PLAN" || true)
assert_eq "empty no trailing ws" "work_item_id:" "$PLAN_LINE"

echo ""

# ── mixed-plan-shapes ─────────────────────────────────────────────────────
echo "Test: mixed-plan-shapes — AC #6 invariant across directory"
REPO=$(setup_0006_repo mixed-plan-shapes)
RC=0
OUTPUT=$(run_0006_driver "$REPO") || RC=$?
assert_eq "exit 0" "0" "$RC"
# Filter: only count those where value is non-quoted AND non-empty
NON_QUOTED_FILTERED=$(grep -rE '^work_item_id: [^"]' "$REPO/meta/plans" || true)
assert_eq "no unquoted non-empty work_item_id values" "" "$NON_QUOTED_FILTERED"
assert_contains "stdout reports 3 rewrites" "$OUTPUT" "0006: rewrote 3 file(s) under meta/plans"

echo ""

# ── partial-prior-run-plan (matching) ─────────────────────────────────────
echo "Test: partial-prior-run-plan — stale work-item dropped, no warn"
REPO=$(setup_0006_repo partial-prior-run-plan)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
WIID_LINES=$(grep -c '^work_item_id:' "$PLAN" || true)
assert_eq "exactly one work_item_id" "1" "$WIID_LINES"
assert_not_contains "no work-item line" "$(cat "$PLAN")" "work-item:"
assert_not_contains "no DIVERGE warning" "$STDERR" "0006-DIVERGE"

echo ""

# ── partial-prior-run-plan-unquoted ───────────────────────────────────────
echo "Test: partial-prior-run-plan-unquoted — survivor normalised to quoted"
REPO=$(setup_0006_repo partial-prior-run-plan-unquoted)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
PLAN_LINE=$(grep '^work_item_id:' "$PLAN" || true)
assert_eq "canonical quoted form" 'work_item_id: "0042"' "$PLAN_LINE"
assert_not_contains "no DIVERGE warning" "$STDERR" "0006-DIVERGE"

echo ""

# ── partial-prior-run-plan-divergent ──────────────────────────────────────
echo "Test: partial-prior-run-plan-divergent — work_item_id wins; stderr warns"
REPO=$(setup_0006_repo partial-prior-run-plan-divergent)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
assert_contains "work_item_id 0099 remains" "$(cat "$PLAN")" 'work_item_id: "0099"'
assert_not_contains "no work-item line" "$(cat "$PLAN")" "work-item:"
assert_contains "DIVERGE warning fired" "$STDERR" "0006-DIVERGE"

echo ""

# ── partial-prior-run-plan-refused-shape ──────────────────────────────────
echo "Test: partial-prior-run-plan-refused-shape — refused legacy preserved"
REPO=$(setup_0006_repo partial-prior-run-plan-refused-shape)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
assert_contains "refused legacy line preserved" "$(cat "$PLAN")" "work-item: 0042 # note"
assert_contains "canonical line preserved" "$(cat "$PLAN")" 'work_item_id: "0042"'
assert_contains "stderr REFUSE warning" "$STDERR" "0006-REFUSE"

echo ""

# ── partial-prior-run-research ────────────────────────────────────────────
echo "Test: partial-prior-run-research — stale researcher dropped, no warn"
REPO=$(setup_0006_repo partial-prior-run-research)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
R="$REPO/meta/research/codebase/2026-01-01-foo.md"
A_LINES=$(grep -c '^author:' "$R" || true)
assert_eq "exactly one author" "1" "$A_LINES"
assert_not_contains "no researcher line" "$(cat "$R")" "researcher:"
assert_not_contains "no DIVERGE warning" "$STDERR" "0006-DIVERGE"

echo ""

# ── partial-prior-run-research-divergent ──────────────────────────────────
echo "Test: partial-prior-run-research-divergent — author wins; stderr warns"
REPO=$(setup_0006_repo partial-prior-run-research-divergent)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
R="$REPO/meta/research/codebase/2026-01-01-foo.md"
assert_contains "author B remains" "$(cat "$R")" "author: B"
assert_not_contains "no researcher line" "$(cat "$R")" "researcher:"
assert_contains "stderr DIVERGE warning" "$STDERR" "0006-DIVERGE"

echo ""

# ── partial-prior-run-body-label (matching) ───────────────────────────────
echo "Test: partial-prior-run-body-label — stale **Researcher** dropped"
REPO=$(setup_0006_repo partial-prior-run-body-label)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
R="$REPO/meta/research/codebase/2026-01-01-foo.md"
assert_not_contains "no **Researcher** line" "$(cat "$R")" "**Researcher**:"
assert_contains "**Author** line" "$(cat "$R")" "**Author**: Toby"
assert_not_contains "no DIVERGE warning" "$STDERR" "0006-DIVERGE"

echo ""

# ── partial-prior-run-body-label-divergent ────────────────────────────────
echo "Test: partial-prior-run-body-label-divergent — **Author** wins, warns"
REPO=$(setup_0006_repo partial-prior-run-body-label-divergent)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
R="$REPO/meta/research/codebase/2026-01-01-foo.md"
assert_contains "**Author** B remains" "$(cat "$R")" "**Author**: B"
assert_not_contains "no **Researcher** line" "$(cat "$R")" "**Researcher**:"
assert_contains "stderr DIVERGE warning" "$STDERR" "0006-DIVERGE"

echo ""

# ── body-label-multiple ───────────────────────────────────────────────────
echo "Test: body-label-multiple — only pre-H2 occurrence rewritten"
REPO=$(setup_0006_repo body-label-multiple)
RC=0
run_0006_driver "$REPO" >/dev/null || RC=$?
assert_eq "exit 0" "0" "$RC"
R="$REPO/meta/research/codebase/2026-01-01-foo.md"
AUTHOR_LINES=$(grep -c '^\*\*Author\*\*:' "$R" || true)
RESEARCHER_LINES=$(grep -c '^\*\*Researcher\*\*:' "$R" || true)
assert_eq "exactly one **Author**" "1" "$AUTHOR_LINES"
assert_eq "exactly one **Researcher** preserved (in code block)" "1" "$RESEARCHER_LINES"

echo ""

# ── body-label-anchored-no-h2 ─────────────────────────────────────────────
echo "Test: body-label-anchored-no-h2 — rewrite happens with no H2"
REPO=$(setup_0006_repo body-label-anchored-no-h2)
RC=0
run_0006_driver "$REPO" >/dev/null || RC=$?
assert_eq "exit 0" "0" "$RC"
R="$REPO/meta/research/codebase/2026-01-01-foo.md"
assert_contains "**Author** present" "$(cat "$R")" "**Author**: Toby"
assert_not_contains "no **Researcher**" "$(cat "$R")" "**Researcher**:"

echo ""

# ── body-label-quoted-prose-pre-h2 ────────────────────────────────────────
echo "Test: body-label-quoted-prose-pre-h2 — both pre-H2 occurrences rewritten"
REPO=$(setup_0006_repo body-label-quoted-prose-pre-h2)
RC=0
run_0006_driver "$REPO" >/dev/null || RC=$?
assert_eq "exit 0" "0" "$RC"
R="$REPO/meta/research/codebase/2026-01-01-foo.md"
AUTHOR_LINES=$(grep -c '^\*\*Author\*\*:' "$R" || true)
assert_eq "both occurrences rewritten" "2" "$AUTHOR_LINES"
assert_not_contains "no **Researcher** line" "$(cat "$R")" "**Researcher**:"

echo ""

# ── frontmatter-missing-fence ─────────────────────────────────────────────
echo "Test: frontmatter-missing-fence — no rewrite, MALFORMED warning"
REPO=$(setup_0006_repo frontmatter-missing-fence)
RC=0
OUTPUT=$(run_0006_driver "$REPO") || RC=$?
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/meta/plans/0001-foo.md"
assert_contains "legacy line preserved" "$(cat "$PLAN")" "work-item: \"0001\""
assert_contains "MALFORMED warning" "$OUTPUT" "0006-MALFORMED"
assert_contains "reports 0 rewrites" "$OUTPUT" "0006: rewrote 0 file(s) under meta/plans"

echo ""

# ── paths-override-plans ──────────────────────────────────────────────────
echo "Test: paths-override-plans — docs/plans honoured"
REPO=$(setup_0006_repo paths-override-plans)
RC=0
OUTPUT=$(run_0006_driver "$REPO") || RC=$?
assert_eq "exit 0" "0" "$RC"
PLAN="$REPO/docs/plans/0001-foo.md"
assert_contains "rewrite at override path" "$(cat "$PLAN")" 'work_item_id: "0001"'
assert_dir_not_exists "meta/plans not created" "$REPO/meta/plans"
assert_contains "stdout reports docs/plans" "$OUTPUT" "0006: rewrote 1 file(s) under docs/plans"

echo ""

# ── paths-override-research-codebase ──────────────────────────────────────
echo "Test: paths-override-research-codebase — docs/research honoured"
REPO=$(setup_0006_repo paths-override-research-codebase)
RC=0
OUTPUT=$(run_0006_driver "$REPO") || RC=$?
assert_eq "exit 0" "0" "$RC"
R="$REPO/docs/research/2026-01-01.md"
assert_contains "author at override" "$(cat "$R")" "author: Toby"
assert_contains "stdout reports docs/research" "$OUTPUT" "0006: rewrote 1 file(s) under docs/research"

echo ""

# ── paths-override-research-issues ────────────────────────────────────────
echo "Test: paths-override-research-issues — docs/rca honoured"
REPO=$(setup_0006_repo paths-override-research-issues)
RC=0
OUTPUT=$(run_0006_driver "$REPO") || RC=$?
assert_eq "exit 0" "0" "$RC"
R="$REPO/docs/rca/2026-01-02.md"
assert_contains "author at override" "$(cat "$R")" "author: Toby"
assert_contains "stdout reports docs/rca" "$OUTPUT" "0006: rewrote 1 file(s) under docs/rca"

echo ""

# ── paths-missing-plans ───────────────────────────────────────────────────
echo "Test: paths-missing-plans — missing dir warns; exit 0"
REPO=$(setup_0006_repo paths-missing-plans)
RC=0
OUTPUT=$(run_0006_driver "$REPO") || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "missing-dir warning" "$OUTPUT" "plans directory does not exist"
assert_contains "reports 0 rewrites" "$OUTPUT" "0006: rewrote 0 file(s) under docs/typo-plans"
APPLIED=$(cat "$REPO/.accelerator/state/migrations-applied" 2>/dev/null || echo "")
assert_contains "migration recorded" "$APPLIED" "0006-canonicalise-work-item-id-and-author"

echo ""

# ── empty-research-issues ─────────────────────────────────────────────────
echo "Test: empty-research-issues — exit 0; zero rewrites; no changes"
REPO=$(setup_0006_repo empty-research-issues)
RC=0
OUTPUT=$(run_0006_driver "$REPO") || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "research_issues 0 rewrites" "$OUTPUT" "0006: rewrote 0 file(s) under meta/research/issues"

echo ""

# ── template-override-tier2-plan ──────────────────────────────────────────
echo "Test: template-override-tier2-plan — tier-2 plan template rewritten"
REPO=$(setup_0006_repo template-override-tier2-plan)
RC=0
run_0006_driver "$REPO" >/dev/null || RC=$?
assert_eq "exit 0" "0" "$RC"
T="$REPO/.accelerator/templates/plan.md"
assert_contains "tier-2 plan rewritten" "$(cat "$T")" 'work_item_id: "{work-item reference, if any}"'
assert_not_contains "no work-item: key" "$(cat "$T")" "^work-item:"

echo ""

# ── template-override-tier2-research ──────────────────────────────────────
echo "Test: template-override-tier2-research — tier-2 research template rewritten"
REPO=$(setup_0006_repo template-override-tier2-research)
RC=0
run_0006_driver "$REPO" >/dev/null || RC=$?
assert_eq "exit 0" "0" "$RC"
T="$REPO/.accelerator/templates/codebase-research.md"
assert_contains "author key" "$(cat "$T")" "author: [Git author]"
assert_contains "**Author** label" "$(cat "$T")" "**Author**: [Researcher name]"
assert_not_contains "no researcher key" "$(cat "$T")" "researcher:"

echo ""

# ── template-override-tier2-rca ───────────────────────────────────────────
echo "Test: template-override-tier2-rca — tier-2 rca template rewritten"
REPO=$(setup_0006_repo template-override-tier2-rca)
RC=0
run_0006_driver "$REPO" >/dev/null || RC=$?
assert_eq "exit 0" "0" "$RC"
T="$REPO/.accelerator/templates/rca.md"
assert_contains "author key" "$(cat "$T")" "author: [Git author]"
assert_contains "**Author** label" "$(cat "$T")" "**Author**: [Researcher name]"
assert_not_contains "no researcher key" "$(cat "$T")" "researcher:"

echo ""

# ── template-override-tier1 ───────────────────────────────────────────────
echo "Test: template-override-tier1 — tier-1 template at custom path rewritten"
REPO=$(setup_0006_repo template-override-tier1)
RC=0
run_0006_driver "$REPO" >/dev/null || RC=$?
assert_eq "exit 0" "0" "$RC"
T="$REPO/custom/templates/my-plan.md"
assert_contains "tier-1 rewritten" "$(cat "$T")" "work_item_id:"

echo ""

# ── template-override-both-tiers ──────────────────────────────────────────
echo "Test: template-override-both-tiers — only tier-1 rewritten"
REPO=$(setup_0006_repo template-override-both-tiers)
RC=0
run_0006_driver "$REPO" >/dev/null || RC=$?
assert_eq "exit 0" "0" "$RC"
T1="$REPO/custom/templates/my-plan.md"
T2="$REPO/.accelerator/templates/plan.md"
assert_contains "tier-1 rewritten" "$(cat "$T1")" "work_item_id:"
assert_contains "tier-2 untouched" "$(cat "$T2")" 'work-item: "{ref}"'

echo ""

# ── template-override-tier1-missing-file ──────────────────────────────────
echo "Test: template-override-tier1-missing-file — warn + no fallthrough"
REPO=$(setup_0006_repo template-override-tier1-missing-file)
RC=0
OUTPUT=$(run_0006_driver "$REPO") || RC=$?
assert_eq "exit 0" "0" "$RC"
assert_contains "missing-file warning" "$OUTPUT" "templates.plan points at missing file"
T2="$REPO/.accelerator/templates/plan.md"
assert_contains "tier-2 NOT rewritten" "$(cat "$T2")" 'work-item: "{ref}"'

echo ""

# ── template-override-missing ─────────────────────────────────────────────
echo "Test: template-override-missing — exit 0, no error, no warning"
REPO=$(setup_0006_repo template-override-missing)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
# Stderr may contain other framework messages, but no 0006 warnings about templates
assert_not_contains "no templates.plan warning" "$STDERR" "templates.plan"

echo ""

# ── paths-alias-research ──────────────────────────────────────────────────
echo "Test: paths-alias-research — dedup; rewrite once"
REPO=$(setup_0006_repo paths-alias-research)
RC=0
OUTPUT=$(run_0006_driver "$REPO") || RC=$?
assert_eq "exit 0" "0" "$RC"
R="$REPO/docs/research/2026-01-01.md"
A_LINES=$(grep -c '^author:' "$R" || true)
assert_eq "exactly one author line" "1" "$A_LINES"
# Pin the recorded owner key, not just the "aliases paths." prefix — guards the
# parallel-array owner lookup against returning the wrong recorded entry.
assert_contains "alias warning names correct owner key" "$OUTPUT" "aliases paths.research_codebase"
assert_contains "reports skip" "$OUTPUT" "skipping duplicate walk"

echo ""

# ── template-alias ────────────────────────────────────────────────────────
echo "Test: template-alias — dedup; rewrite once; warn"
REPO=$(setup_0006_repo template-alias)
RC=0
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
run_0006_driver_split "$REPO" "$STDOUT_FILE" "$STDERR_FILE" || RC=$?
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
T="$REPO/.accelerator/templates/shared.md"
WIID=$(grep -c '^work_item_id:' "$T" || true)
AUTHOR=$(grep -c '^author:' "$T" || true)
assert_eq "exactly one work_item_id" "1" "$WIID"
assert_eq "exactly one author" "1" "$AUTHOR"
assert_contains "template alias warning" "$STDERR" "resolve to the same file"

echo ""

# ── idempotent ────────────────────────────────────────────────────────────
echo "Test: idempotent — three runs byte-identical"
REPO=$(setup_0006_repo default-layout)
run_0006_driver "$REPO" >/dev/null 2>&1
HASH1=$(tree_hash "$REPO/meta")
RC=0
run_0006_driver "$REPO" >/dev/null 2>&1 || RC=$?
HASH2=$(tree_hash "$REPO/meta")
assert_eq "second-run exit 0" "0" "$RC"
assert_eq "second-run byte-identical" "$HASH1" "$HASH2"
RC=0
run_0006_driver "$REPO" >/dev/null 2>&1 || RC=$?
HASH3=$(tree_hash "$REPO/meta")
assert_eq "third-run exit 0" "0" "$RC"
assert_eq "third-run byte-identical" "$HASH2" "$HASH3"

echo ""

test_summary
