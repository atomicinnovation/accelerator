#!/usr/bin/env bash
set -uo pipefail

# Dedicated unit harness for merge_move (scripts/fs-common.sh). Drives every
# branch of the helper in isolation — coverage the migration integration tests
# cannot give (they exercise only each migration's fixed path-pairs).
# Run: bash scripts/test-merge-move.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"
source "$SCRIPT_DIR/fs-common.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# fresh_workdir: a clean scratch directory for one test case.
fresh_workdir() {
  mktemp -d "$TMPDIR_BASE/case-XXXXXX"
}

# mk <path> <content>: write a file, creating parents.
mk() {
  mkdir -p "$(dirname "$1")"
  printf '%s' "$2" > "$1"
}

echo "=== merge_move ==="

# ── dest-absent plain move ───────────────────────────────────────────────────
echo "Test: dest-absent plain move"
w=$(fresh_workdir)
mk "$w/src/a.md" "alpha"
merge_move "$w/src" "$w/dst"
assert_dir_not_exists "source removed after plain move" "$w/src"
assert_file_content_eq "file relocated" "$w/dst/a.md" "alpha"

# ── missing-source no-op ─────────────────────────────────────────────────────
echo "Test: missing-source no-op"
w=$(fresh_workdir)
mk "$w/dst/keep.md" "kept"
merge_move "$w/absent" "$w/dst"
assert_exit_code "missing source returns 0" 0 merge_move "$w/absent" "$w/dst"
assert_file_content_eq "destination untouched" "$w/dst/keep.md" "kept"

# ── file-onto-file leaf collision → source wins ──────────────────────────────
echo "Test: file-onto-file leaf collision — source wins"
w=$(fresh_workdir)
mk "$w/src.md" "from-source"
mk "$w/dst.md" "from-dest"
merge_move "$w/src.md" "$w/dst.md"
assert_file_content_eq "source content wins" "$w/dst.md" "from-source"
assert_file_not_exists "source file removed" "$w/src.md"

# ── type mismatch: source FILE over destination DIRECTORY ────────────────────
echo "Test: type mismatch — source file over destination directory (rm -rf branch)"
w=$(fresh_workdir)
mk "$w/src.md" "i-am-a-file"
mk "$w/dst/buried/deep.md" "displaced"
merge_move "$w/src.md" "$w/dst"
assert_file_content_eq "source file replaced the directory wholesale" "$w/dst" "i-am-a-file"
assert_dir_not_exists "displaced destination subtree is gone" "$w/dst/buried"
assert_file_not_exists "source file removed" "$w/src.md"

# ── type mismatch: source DIRECTORY over destination FILE ────────────────────
echo "Test: type mismatch — source directory over destination file (rm -rf branch)"
w=$(fresh_workdir)
mk "$w/src/inner.md" "tree-content"
mk "$w/dst" "i-am-a-file"
merge_move "$w/src" "$w/dst"
assert_dir_exists "source directory replaced the file wholesale" "$w/dst"
assert_file_content_eq "directory contents present" "$w/dst/inner.md" "tree-content"
assert_dir_not_exists "source removed" "$w/src"

# ── nested same-named-subdir merge (recursion) + leaf collision inside ───────
echo "Test: nested same-named-subdir merge with inner leaf collision"
w=$(fresh_workdir)
mk "$w/src/sub/new.md" "src-new"
mk "$w/src/sub/shared.md" "src-shared"
mk "$w/src/top.md" "src-top"
mk "$w/dst/sub/existing.md" "dst-existing"
mk "$w/dst/sub/shared.md" "dst-shared"
merge_move "$w/src" "$w/dst"
assert_file_content_eq "new nested file merged in" "$w/dst/sub/new.md" "src-new"
assert_file_content_eq "pre-existing nested file preserved" "$w/dst/sub/existing.md" "dst-existing"
assert_file_content_eq "inner leaf collision: source wins" "$w/dst/sub/shared.md" "src-shared"
assert_file_content_eq "top-level file merged in" "$w/dst/top.md" "src-top"
assert_dir_not_exists "source removed after merge" "$w/src"

# ── filenames with spaces ────────────────────────────────────────────────────
echo "Test: filenames with spaces merge correctly"
w=$(fresh_workdir)
mk "$w/src/a file.md" "spaced-src"
mk "$w/dst/other file.md" "spaced-dst"
merge_move "$w/src" "$w/dst"
assert_file_content_eq "spaced source file merged" "$w/dst/a file.md" "spaced-src"
assert_file_content_eq "spaced dest file preserved" "$w/dst/other file.md" "spaced-dst"
assert_dir_not_exists "source removed" "$w/src"

# ── dotfile-only source merges; source dir removed ───────────────────────────
echo "Test: dotfile-only source merges and source dir is removed"
w=$(fresh_workdir)
mk "$w/src/.keep" "dot-content"
mk "$w/dst/visible.md" "visible"
merge_move "$w/src" "$w/dst"
assert_file_content_eq "dotfile merged in" "$w/dst/.keep" "dot-content"
assert_file_content_eq "visible dest file preserved" "$w/dst/visible.md" "visible"
assert_dir_not_exists "dotfile-only source removed" "$w/src"

# ── empty-source directory → source removed, no error ────────────────────────
echo "Test: empty-source directory — source removed, no error"
w=$(fresh_workdir)
mkdir -p "$w/src"
mkdir -p "$w/dst"
mk "$w/dst/x.md" "x"
assert_exit_code "empty source merge returns 0" 0 merge_move "$w/src" "$w/dst"
assert_dir_not_exists "empty source removed" "$w/src"
assert_file_content_eq "destination intact" "$w/dst/x.md" "x"

# ── partial/interrupted convergence ──────────────────────────────────────────
echo "Test: partial/interrupted convergence — second run completes the merge"
w=$(fresh_workdir)
mk "$w/src/one.md" "one"
mk "$w/src/two.md" "two"
mk "$w/src/three.md" "three"
# Simulate a prior run that only relocated one.md (dst already has it).
mk "$w/dst/one.md" "one"
merge_move "$w/src" "$w/dst"
assert_file_content_eq "first entry converged" "$w/dst/one.md" "one"
assert_file_content_eq "second entry merged" "$w/dst/two.md" "two"
assert_file_content_eq "third entry merged" "$w/dst/three.md" "three"
assert_dir_not_exists "source emptied and removed" "$w/src"

# ── unsafe-destination refusal (deletes nothing) ─────────────────────────────
echo "Test: unsafe-destination refusal — returns non-zero, deletes nothing"
w=$(fresh_workdir)
mk "$w/src/a.md" "guard"
assert_exit_code "empty destination refused" 1 merge_move "$w/src" ""
assert_exit_code "root destination refused" 1 merge_move "$w/src" "/"
assert_exit_code "trailing-slash destination refused" 1 merge_move "$w/src" "$w/dst/"
assert_exit_code "path-escaping destination refused" 1 merge_move "$w/src" "$w/dst/../escape"
assert_dir_exists "source untouched after every refusal" "$w/src"
assert_file_content_eq "source content untouched" "$w/src/a.md" "guard"

test_summary
