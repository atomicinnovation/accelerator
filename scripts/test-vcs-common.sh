#!/usr/bin/env bash
set -euo pipefail

# Test harness for scripts/vcs-common.sh: the in-process taxonomy helpers
# (find_repo_root, vcs_mode, find_jj_main_workspace_root,
# find_git_main_worktree_root, classify_checkout, _jj_workspace_is_secondary).
# These functions keep their other ~20 callers after 0169 retires
# hooks/vcs-detect.sh and hooks/vcs-guard.sh (the accelerator-vcs sub-binary's
# own parity coverage lives under cli/vcs-cli/tests/ and
# cli/vcs-adapters/tests/), so this suite moved verbatim out of the retired
# hooks/test-vcs-detect.sh rather than being deleted alongside it.

if [ -z "${BASH_VERSION:-}" ]; then
  echo "scripts/test-vcs-common.sh requires bash" >&2
  exit 1
fi
for tool in jj git realpath; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "scripts/test-vcs-common.sh requires $tool on PATH (run via 'mise run test:integration:config' or install $tool)" >&2
    exit 77 # autotools 'skip' convention; harness reports as skipped
  fi
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"
source "$PLUGIN_ROOT/scripts/vcs-common.sh"

# Scope git's discovery to TMPDIR_BASE so a stray `.git` further up
# (e.g., the accelerator's own checkout when running tests locally)
# cannot leak into fixture-builder probes.
TMPDIR_BASE=$(mktemp -d)
export GIT_CEILING_DIRECTORIES="$TMPDIR_BASE"
trap 'rm -rf "$TMPDIR_BASE"' EXIT

new_workdir() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  realpath "$d"
}

make_main_jj_workspace() {
  local d
  d=$(new_workdir)
  (cd "$d" && jj git init --quiet)
  printf '%s\n' "$d"
}

make_main_git_checkout() {
  local d
  d=$(new_workdir)
  (cd "$d" && git init -q && git config user.email t@e.x && git config user.name T)
  # Create one commit so `git worktree add` later works.
  (cd "$d" && git commit --allow-empty -q -m "init")
  printf '%s\n' "$d"
}

# Bare repo fixture: exercises find_git_main_worktree_root's bare-repo
# guard. Bare repos have no main worktree, so the helper must return 1.
make_bare_git_repo() {
  local d
  d=$(new_workdir)
  (cd "$d" && git init --bare -q)
  printf '%s\n' "$d"
}

# Multi-value fixture builders set named globals (FIXTURE_*).
# Reset the globals each call so leftovers from a previous fixture
# can never bleed into the next.
make_jj_secondary_workspace() {
  FIXTURE_PARENT="" FIXTURE_SECONDARY=""
  FIXTURE_PARENT=$(make_main_jj_workspace)
  local secondary
  secondary=$(new_workdir)
  rm -rf "$secondary"
  (cd "$FIXTURE_PARENT" && jj workspace add --quiet "$secondary")
  FIXTURE_SECONDARY=$(realpath "$secondary")
}

make_git_linked_worktree() {
  FIXTURE_PARENT="" FIXTURE_WORKTREE=""
  FIXTURE_PARENT=$(make_main_git_checkout)
  local worktree
  worktree=$(new_workdir)
  rm -rf "$worktree"
  (cd "$FIXTURE_PARENT" && git worktree add -q "$worktree")
  FIXTURE_WORKTREE=$(realpath "$worktree")
}

make_colocated_secondary() {
  # Colocated == same path is BOTH a jj secondary AND a git linked worktree.
  # Build two independent parents, then assemble a single colocated target.
  #
  # FIXTURE CONSTRUCTION IS NON-TRIVIAL because both `git worktree add` and
  # `jj workspace add` refuse an existing non-empty target. We work around
  # this by:
  #   1. Running `git worktree add` first into a fresh path (creates .git
  #      file + checked-out content).
  #   2. Running `jj workspace add` to a SEPARATE tmp path, then grafting
  #      the resulting .jj/ directory into the target. The grafted
  #      .jj/repo file's relative path no longer resolves correctly, so
  #      we overwrite it with an ABSOLUTE path back to the jj parent's
  #      .jj/repo directory. find_jj_main_workspace_root's algorithm
  #      (`cd $workspace_root/.jj && cd $(cat $marker) && pwd`) handles
  #      absolute and relative paths uniformly because `cd <abs>` works
  #      regardless of cwd.
  #
  # If a future jj release adds a flag for adding a workspace at an
  # existing path (e.g., --existing-dir / --here), simplify this builder
  # to use it directly and skip the graft step.
  FIXTURE_JJ_PARENT="" FIXTURE_GIT_PARENT="" FIXTURE_TARGET=""
  FIXTURE_JJ_PARENT=$(make_main_jj_workspace)
  FIXTURE_GIT_PARENT=$(make_main_git_checkout)
  local target jj_tmp
  target=$(new_workdir)
  rm -rf "$target"
  # Step 1: git worktree at the target (creates target with .git file).
  (cd "$FIXTURE_GIT_PARENT" && git worktree add -q "$target")
  # Step 2: jj workspace at a tmp path, then graft .jj/ into target.
  jj_tmp=$(new_workdir)
  rm -rf "$jj_tmp"
  (cd "$FIXTURE_JJ_PARENT" && jj workspace add --quiet "$jj_tmp")
  mv "$jj_tmp/.jj" "$target/.jj"
  # Rewrite .jj/repo with an absolute path back to jj_parent. Standard jj
  # writes a relative path, but absolute paths are accepted by the
  # `cd $(cat ...)` algorithm and are portable across BSD/GNU realpath
  # (no `--relative-to` flag needed).
  #
  # NOTE: jj reads `.jj/repo` verbatim and does NOT trim trailing
  # whitespace — a trailing newline turns the resolved path into a
  # nonexistent "<path>\n" and breaks `jj workspace root`. Use `%s` with
  # no newline.
  printf '%s' "$FIXTURE_JJ_PARENT/.jj/repo" >"$target/.jj/repo"
  rm -rf "$jj_tmp"
  FIXTURE_TARGET=$(realpath "$target")
  # Smoke-checks (pure filesystem assertions — do NOT invoke vcs-common.sh
  # helpers here, because fixture builders are defined before the `source`
  # line and we want them callable in any order).
  [ -f "$FIXTURE_TARGET/.jj/repo" ] || {
    echo "colocated fixture missing .jj/repo file" >&2
    exit 1
  }
  [ -e "$FIXTURE_TARGET/.git" ] || {
    echo "colocated fixture missing .git marker" >&2
    exit 1
  }
  [ "$(cat "$FIXTURE_TARGET/.jj/repo")" = "$FIXTURE_JJ_PARENT/.jj/repo" ] || {
    echo "colocated fixture: .jj/repo content does not point at jj_parent" >&2
    exit 1
  }
}

# Cross-VCS fixture: a jj secondary workspace whose target sits inside
# a pure-git parent.
make_jj_secondary_in_git_parent() {
  FIXTURE_JJ_PARENT="" FIXTURE_GIT_PARENT="" FIXTURE_TARGET=""
  FIXTURE_GIT_PARENT=$(make_main_git_checkout)
  FIXTURE_JJ_PARENT=$(make_main_jj_workspace)
  local target="$FIXTURE_GIT_PARENT/sub"
  (cd "$FIXTURE_JJ_PARENT" && jj workspace add --quiet "$target")
  FIXTURE_TARGET=$(realpath "$target")
}

# Symmetric cross-VCS fixture: a git linked worktree whose target sits
# inside a pure-jj parent.
make_git_worktree_in_jj_parent() {
  FIXTURE_JJ_PARENT="" FIXTURE_GIT_PARENT="" FIXTURE_TARGET=""
  FIXTURE_JJ_PARENT=$(make_main_jj_workspace)
  FIXTURE_GIT_PARENT=$(make_main_git_checkout)
  local target="$FIXTURE_JJ_PARENT/sub"
  # git worktree add requires a non-existent target.
  (cd "$FIXTURE_GIT_PARENT" && git worktree add -q "$target")
  FIXTURE_TARGET=$(realpath "$target")
}

echo "=== scripts/vcs-common.sh ==="

# ── _jj_workspace_is_secondary (jj internal-marker isolation function) ────────
echo "Test: _jj_workspace_is_secondary returns 1 in a main workspace"
d=$(make_main_jj_workspace)
RC=0
_jj_workspace_is_secondary "$d" || RC=$?
assert_eq "main workspace returns 1" "1" "$RC"

echo "Test: _jj_workspace_is_secondary returns 0 in a secondary workspace"
make_jj_secondary_workspace
RC=0
_jj_workspace_is_secondary "$FIXTURE_SECONDARY" || RC=$?
assert_eq "secondary workspace returns 0" "0" "$RC"

# ── find_jj_main_workspace_root ───────────────────────────────────────────────
echo "Test: find_jj_main_workspace_root in a main jj workspace"
d=$(make_main_jj_workspace)
RESULT=$( (cd "$d" && find_jj_main_workspace_root .))
assert_eq "returns the workspace root" "$d" "$RESULT"

echo "Test: find_jj_main_workspace_root in a jj secondary workspace"
make_jj_secondary_workspace
RESULT=$( (cd "$FIXTURE_SECONDARY" && find_jj_main_workspace_root .))
assert_eq "returns the parent main workspace" "$FIXTURE_PARENT" "$RESULT"

# Failure-mode contract: plain non-repo dir must return exit 1, empty stdout.
echo "Test: find_jj_main_workspace_root failure in a plain directory"
d=$(new_workdir)
RC=0
RESULT=$( (cd "$d" && find_jj_main_workspace_root .)) || RC=$?
assert_eq "exits 1 (plain)" "1" "$RC"
assert_eq "empty stdout (plain)" "" "$RESULT"

# ── find_git_main_worktree_root ───────────────────────────────────────────────
echo "Test: find_git_main_worktree_root in a main git checkout"
d=$(make_main_git_checkout)
RESULT=$( (cd "$d" && find_git_main_worktree_root .))
assert_eq "returns the checkout root" "$d" "$RESULT"

echo "Test: find_git_main_worktree_root in a git linked worktree"
make_git_linked_worktree
RESULT=$( (cd "$FIXTURE_WORKTREE" && find_git_main_worktree_root .))
assert_eq "returns the parent main checkout" "$FIXTURE_PARENT" "$RESULT"

# Failure-mode contracts: plain non-repo and bare-repo → exit 1, empty stdout.
echo "Test: find_git_main_worktree_root failure in a plain directory"
d=$(new_workdir)
RC=0
RESULT=$( (cd "$d" && find_git_main_worktree_root .)) || RC=$?
assert_eq "exits 1 (plain)" "1" "$RC"
assert_eq "empty stdout (plain)" "" "$RESULT"

echo "Test: find_git_main_worktree_root failure in a bare git repo"
d=$(make_bare_git_repo)
RC=0
RESULT=$( (cd "$d" && find_git_main_worktree_root .)) || RC=$?
assert_eq "exits 1 (bare)" "1" "$RC"
assert_eq "empty stdout (bare)" "" "$RESULT"

# ── classify_checkout — structured KEY=VALUE record ──────────────────────────
# Parser sets globals C_KIND, C_BOUNDARY, C_JJ_PARENT, C_GIT_PARENT. The
# record's JJ_MISSING/GIT_MISSING fields powered hooks/vcs-detect.sh's
# missing-binary diagnostic; that suite retired with the shell hook (0169), so
# this parser no longer reads them.
parse_classification() {
  C_KIND=""
  C_BOUNDARY=""
  C_JJ_PARENT=""
  C_GIT_PARENT=""
  while IFS='=' read -r k v; do
    case "$k" in
      KIND) C_KIND=$v ;;
      BOUNDARY) C_BOUNDARY=$v ;;
      JJ_PARENT) C_JJ_PARENT=$v ;;
      GIT_PARENT) C_GIT_PARENT=$v ;;
    esac
  done <<<"$1"
}

echo "Test: classify_checkout KIND=main (jj)"
d=$(make_main_jj_workspace)
parse_classification "$( (cd "$d" && classify_checkout .))"
assert_eq "KIND=main" "main" "$C_KIND"
assert_eq "BOUNDARY empty" "" "$C_BOUNDARY"
assert_eq "JJ_PARENT empty" "" "$C_JJ_PARENT"
assert_eq "GIT_PARENT empty" "" "$C_GIT_PARENT"

echo "Test: classify_checkout KIND=main (git)"
d=$(make_main_git_checkout)
parse_classification "$( (cd "$d" && classify_checkout .))"
assert_eq "KIND=main" "main" "$C_KIND"
assert_eq "BOUNDARY empty" "" "$C_BOUNDARY"

echo "Test: classify_checkout KIND=jj-secondary"
make_jj_secondary_workspace
parse_classification "$( (cd "$FIXTURE_SECONDARY" && classify_checkout .))"
assert_eq "KIND=jj-secondary" "jj-secondary" "$C_KIND"
assert_eq "BOUNDARY=secondary" "$FIXTURE_SECONDARY" "$C_BOUNDARY"
assert_eq "JJ_PARENT=parent" "$FIXTURE_PARENT" "$C_JJ_PARENT"
assert_eq "GIT_PARENT empty" "" "$C_GIT_PARENT"

echo "Test: classify_checkout KIND=git-worktree"
make_git_linked_worktree
parse_classification "$( (cd "$FIXTURE_WORKTREE" && classify_checkout .))"
assert_eq "KIND=git-worktree" "git-worktree" "$C_KIND"
assert_eq "BOUNDARY=worktree" "$FIXTURE_WORKTREE" "$C_BOUNDARY"
assert_eq "GIT_PARENT=parent" "$FIXTURE_PARENT" "$C_GIT_PARENT"
assert_eq "JJ_PARENT empty" "" "$C_JJ_PARENT"

echo "Test: classify_checkout KIND=colocated"
make_colocated_secondary
parse_classification "$( (cd "$FIXTURE_TARGET" && classify_checkout .))"
assert_eq "KIND=colocated" "colocated" "$C_KIND"
assert_eq "BOUNDARY=target" "$FIXTURE_TARGET" "$C_BOUNDARY"
assert_eq "JJ_PARENT=jj_parent" "$FIXTURE_JJ_PARENT" "$C_JJ_PARENT"
assert_eq "GIT_PARENT=git_parent" "$FIXTURE_GIT_PARENT" "$C_GIT_PARENT"

echo "Test: classify_checkout KIND=none in a plain directory"
d=$(new_workdir)
parse_classification "$( (cd "$d" && classify_checkout .))"
assert_eq "KIND=none" "none" "$C_KIND"
assert_eq "BOUNDARY empty" "" "$C_BOUNDARY"

echo "Test: classify_checkout KIND=none in a bare git repo"
d=$(make_bare_git_repo)
parse_classification "$( (cd "$d" && classify_checkout .))"
assert_eq "KIND=none (bare)" "none" "$C_KIND"

# ── find_repo_root unchanged-behaviour regression guard ───────────────────────
# find_repo_root is deliberately not refactored by this work. Lock in its
# current behaviour across the well-defined fixture cases so a future
# accidental edit to vcs-common.sh is caught immediately.
echo "Test: find_repo_root unchanged in main jj workspace"
d=$(make_main_jj_workspace)
RESULT=$( (cd "$d" && find_repo_root))
assert_eq "main jj" "$d" "$RESULT"

echo "Test: find_repo_root unchanged in main git checkout"
d=$(make_main_git_checkout)
RESULT=$( (cd "$d" && find_repo_root))
assert_eq "main git" "$d" "$RESULT"

echo "Test: find_repo_root unchanged in jj secondary workspace"
make_jj_secondary_workspace
RESULT=$( (cd "$FIXTURE_SECONDARY" && find_repo_root))
# .jj is a directory in a jj secondary workspace, so find_repo_root finds it.
assert_eq "jj secondary" "$FIXTURE_SECONDARY" "$RESULT"

# 0124: find_repo_root must succeed in a git linked worktree, where .git is a
# regular file (a gitdir: pointer), not a directory — the marker test is -e,
# not -d. $FIXTURE_WORKTREE is reused by the vcs_mode stanza below.
echo "Test [0124]: find_repo_root returns worktree root in a git linked worktree"
make_git_linked_worktree
RESULT=$( (cd "$FIXTURE_WORKTREE" && find_repo_root))
assert_eq "git linked worktree root" "$FIXTURE_WORKTREE" "$RESULT"

echo "Test [0124]: find_repo_root walks up from a nested subdir in a worktree"
mkdir -p "$FIXTURE_WORKTREE/nested/deeper"
RESULT=$( (cd "$FIXTURE_WORKTREE/nested/deeper" && find_repo_root))
assert_eq "git linked worktree nested subdir" "$FIXTURE_WORKTREE" "$RESULT"

# Guards the actual production failure mode: a non-zero return aborting a
# `set -euo pipefail` caller (the visualiser launchers) with empty stderr.
echo "Test [0124]: find_repo_root exits 0 under set -e in a worktree"
RC=0
(
  set -e
  cd "$FIXTURE_WORKTREE" && find_repo_root >/dev/null
) || RC=$?
assert_eq "git linked worktree exit code" "0" "$RC"

# vcs_mode carries the identical -d defect: in a worktree it returns 'none',
# routing the dirty check into fail-safe-to-dirty. Reuse the
# $FIXTURE_WORKTREE built above rather than rebuilding it.
echo "Test [0124]: vcs_mode returns git for a git linked worktree root"
# pre-fix: .git is a file, the -d test fails → vcs_mode returns 'none'.
assert_eq "vcs_mode worktree" "git" "$(vcs_mode "$FIXTURE_WORKTREE")"

# Non-regression guard for the .jj-WINS ordering: a colocated checkout has .jj
# (a directory) and .git (a file), so this returns 'jj' under both -d and -e —
# it does not go RED, but it locks the precedence the -d→-e change must not
# disturb.
echo "Test [0124]: vcs_mode preserves .jj-WINS for a colocated checkout"
make_colocated_secondary
assert_eq "vcs_mode colocated" "jj" "$(vcs_mode "$FIXTURE_TARGET")"

# ── classify_checkout coverage for the nested KIND values ────────────────────
echo "Test: classify_checkout KIND=nested-jj-in-git"
make_jj_secondary_in_git_parent
parse_classification "$( (cd "$FIXTURE_TARGET" && classify_checkout .))"
assert_eq "KIND=nested-jj-in-git" "nested-jj-in-git" "$C_KIND"
assert_eq "BOUNDARY=target" "$FIXTURE_TARGET" "$C_BOUNDARY"
assert_eq "JJ_PARENT=jj" "$FIXTURE_JJ_PARENT" "$C_JJ_PARENT"
assert_eq "GIT_PARENT=git" "$FIXTURE_GIT_PARENT" "$C_GIT_PARENT"

echo "Test: classify_checkout KIND=nested-git-in-jj"
make_git_worktree_in_jj_parent
parse_classification "$( (cd "$FIXTURE_TARGET" && classify_checkout .))"
assert_eq "KIND=nested-git-in-jj" "nested-git-in-jj" "$C_KIND"
assert_eq "BOUNDARY=target" "$FIXTURE_TARGET" "$C_BOUNDARY"
assert_eq "JJ_PARENT=jj" "$FIXTURE_JJ_PARENT" "$C_JJ_PARENT"
assert_eq "GIT_PARENT=git" "$FIXTURE_GIT_PARENT" "$C_GIT_PARENT"

echo ""
test_summary
