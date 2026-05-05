#!/usr/bin/env bash
set -euo pipefail

# Test harness for skills/config/init/scripts/init.sh
# Run: bash skills/config/init/scripts/test-init.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
INIT_SCRIPT="$SCRIPT_DIR/init.sh"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$repo_dir/.git"
  echo "$repo_dir"
}

# Portable tree hash (mirrors test-migrate.sh:26-34)
tree_hash() {
  local root="$1"
  shift
  if command -v md5sum >/dev/null 2>&1; then
    find "$root" -type f "$@" -exec md5sum {} \; | awk '{print $1}' | sort | md5sum | awk '{print $1}'
  else
    find "$root" -type f "$@" -exec md5 -q {} \; | sort | md5 -q
  fi
}

echo "=== init.sh ==="
echo ""

# ── Test 1: fresh repo creates 14 meta directories with .gitkeep ─────────────
echo "Test: fresh repo creates 14 meta directories with .gitkeep"
REPO=$(setup_repo)
PROJECT_ROOT="$REPO" bash "$INIT_SCRIPT"
for dir in \
  meta/plans meta/research meta/decisions meta/prs meta/validations \
  meta/reviews/plans meta/reviews/prs meta/reviews/work \
  meta/templates meta/work meta/notes \
  meta/design-inventories meta/design-gaps \
  meta/tmp; do
  assert_dir_exists "directory $dir exists" "$REPO/$dir"
  assert_file_exists ".gitkeep in $dir" "$REPO/$dir/.gitkeep"
done

# ── Test 2: inner tmp .gitignore written with ADR-0019 pattern ────────────────
echo "Test: fresh repo writes inner tmp .gitignore with ADR-0019 pattern"
REPO=$(setup_repo)
PROJECT_ROOT="$REPO" bash "$INIT_SCRIPT"
EXPECTED_GI=$(printf '*\n!.gitkeep\n!.gitignore\n')
assert_file_content_eq "tmp .gitignore content" "$REPO/meta/tmp/.gitignore" "$EXPECTED_GI"

# ── Test 3: root .gitignore gets .claude/accelerator.local.md rule ───────────
echo "Test: fresh repo appends .claude/accelerator.local.md to root .gitignore"
REPO=$(setup_repo)
PROJECT_ROOT="$REPO" bash "$INIT_SCRIPT"
if grep -qFx '.claude/accelerator.local.md' "$REPO/.gitignore"; then
  echo "  PASS: rule present in root .gitignore"
  PASS=$((PASS + 1))
else
  echo "  FAIL: rule missing from root .gitignore"
  FAIL=$((FAIL + 1))
fi

# ── Test 4: re-running on already-initialised repo is idempotent ─────────────
echo "Test: re-running on already-initialised repo is idempotent"
REPO=$(setup_repo)
PROJECT_ROOT="$REPO" bash "$INIT_SCRIPT"
HASH_BEFORE=$(tree_hash "$REPO")
PROJECT_ROOT="$REPO" bash "$INIT_SCRIPT"
HASH_AFTER=$(tree_hash "$REPO")
assert_eq "tree hash unchanged after second run" "$HASH_BEFORE" "$HASH_AFTER"

# ── Test 5: root .gitignore rule not duplicated on re-run ────────────────────
echo "Test: re-running on repo where root .gitignore already contains rule does not duplicate it"
REPO=$(setup_repo)
PROJECT_ROOT="$REPO" bash "$INIT_SCRIPT"
PROJECT_ROOT="$REPO" bash "$INIT_SCRIPT"
COUNT=$(grep -cFx '.claude/accelerator.local.md' "$REPO/.gitignore" || true)
assert_eq "rule appears exactly once" "1" "$COUNT"

# ── Test 6: respects paths.tmp override ──────────────────────────────────────
echo "Test: respects paths.tmp override via paths.tmp config key"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
paths:
  tmp: custom-tmp
---
FIXTURE
PROJECT_ROOT="$REPO" bash "$INIT_SCRIPT"
assert_file_exists "custom tmp .gitignore exists" "$REPO/custom-tmp/.gitignore"
assert_file_not_exists "default meta/tmp .gitignore absent" "$REPO/meta/tmp/.gitignore"

echo ""
test_summary
