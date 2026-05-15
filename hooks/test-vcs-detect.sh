#!/usr/bin/env bash
set -euo pipefail

# Preflight: hard-require bash and the VCS binaries this suite drives.
# Local developers who skip `mise install` hit a single, named diagnostic
# rather than opaque fixture-build failures.
if [ -z "${BASH_VERSION:-}" ]; then
  echo "hooks/test-vcs-detect.sh requires bash" >&2
  exit 1
fi
for tool in jj git realpath jq; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "hooks/test-vcs-detect.sh requires $tool on PATH (run via 'mise run test:integration:hooks' or install $tool)" >&2
    exit 77   # autotools 'skip' convention; harness reports as skipped
  fi
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOK="$SCRIPT_DIR/vcs-detect.sh"
FIXTURE_ROOT="$PLUGIN_ROOT/hooks/test-fixtures/vcs-detect"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

# Note: test harness uses `set -euo pipefail` (matches
# hooks/test-migrate-discoverability.sh); the sourced libraries it
# exercises (scripts/vcs-common.sh, hooks/vcs-detect.sh) deliberately
# do NOT set these flags, so as to inherit caller options per the
# established `*-common.sh` convention (scripts/config-common.sh:3-6).

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
  local d; d=$(new_workdir)
  (cd "$d" && jj git init --quiet)
  printf '%s\n' "$d"
}

make_main_git_checkout() {
  local d; d=$(new_workdir)
  (cd "$d" && git init -q && git config user.email t@e.x && git config user.name T)
  # Create one commit so `git worktree add` later works.
  (cd "$d" && git commit --allow-empty -q -m "init")
  printf '%s\n' "$d"
}

# Bare repo fixture: exercises find_git_main_worktree_root's bare-repo
# guard. Bare repos have no main worktree, so the helper must return 1.
make_bare_git_repo() {
  local d; d=$(new_workdir)
  (cd "$d" && git init --bare -q)
  printf '%s\n' "$d"
}

# Multi-value fixture builders set named globals (FIXTURE_*).
# Reset the globals each call so leftovers from a previous fixture
# can never bleed into the next.
make_jj_secondary_workspace() {
  FIXTURE_PARENT="" FIXTURE_SECONDARY=""
  FIXTURE_PARENT=$(make_main_jj_workspace)
  local secondary; secondary=$(new_workdir)
  rm -rf "$secondary"
  (cd "$FIXTURE_PARENT" && jj workspace add --quiet "$secondary")
  FIXTURE_SECONDARY=$(realpath "$secondary")
}

make_git_linked_worktree() {
  FIXTURE_PARENT="" FIXTURE_WORKTREE=""
  FIXTURE_PARENT=$(make_main_git_checkout)
  local worktree; worktree=$(new_workdir)
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
  target=$(new_workdir); rm -rf "$target"
  # Step 1: git worktree at the target (creates target with .git file).
  (cd "$FIXTURE_GIT_PARENT" && git worktree add -q "$target")
  # Step 2: jj workspace at a tmp path, then graft .jj/ into target.
  jj_tmp=$(new_workdir); rm -rf "$jj_tmp"
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
  printf '%s' "$FIXTURE_JJ_PARENT/.jj/repo" > "$target/.jj/repo"
  rm -rf "$jj_tmp"
  FIXTURE_TARGET=$(realpath "$target")
  # Smoke-checks (pure filesystem assertions — do NOT invoke vcs-common.sh
  # helpers here, because fixture builders are defined before the `source`
  # line and we want them callable in any order).
  [ -f "$FIXTURE_TARGET/.jj/repo" ] || { echo "colocated fixture missing .jj/repo file" >&2; exit 1; }
  [ -e "$FIXTURE_TARGET/.git" ] || { echo "colocated fixture missing .git marker" >&2; exit 1; }
  [ "$(cat "$FIXTURE_TARGET/.jj/repo")" = "$FIXTURE_JJ_PARENT/.jj/repo" ] || {
    echo "colocated fixture: .jj/repo content does not point at jj_parent" >&2
    exit 1
  }
}

run_hook() {
  local cwd="$1"
  (cd "$cwd" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$HOOK")
}

# Strip from PATH every directory that resolves <binary> via `type -p`,
# returning the resulting PATH on stdout. macOS commonly provides git in
# both /opt/homebrew/bin AND /usr/bin, so stripping a single dirname (as
# the original plan sketch did) leaves the binary still resolvable.
#
# Iterates until no more occurrences are found, then prints the cleaned
# PATH. Used by the missing-binary tests to make `command -v $binary`
# actually fail inside a subshell.
strip_binary_from_path() {
  local binary="$1"
  local path="$PATH"
  local found
  while found=$(PATH="$path" type -p "$binary" 2>/dev/null) && [ -n "$found" ]; do
    local dir
    dir=$(dirname "$found")
    path=$(printf '%s' "$path" | tr ':' '\n' | grep -vxF "$dir" | paste -sd: -)
  done
  printf '%s' "$path"
}

echo "=== vcs-detect.sh ==="
echo ""

# ── AC5: golden snapshots are free of host-specific path artefacts ────────────
# (Determinism guard: rejects snapshots accidentally regenerated on a host
# whose TMPDIR resolves under /private/var, /var/folders, or a $HOME path.)
echo "Test [AC5]: golden snapshots free of host-specific path artefacts"
for snap in "$FIXTURE_ROOT/main-jj-workspace.json" "$FIXTURE_ROOT/main-git-checkout.json"; do
  for needle in '/private/var' '/var/folders' '/Users/' '/home/'; do
    assert_not_contains "no host artefact ($(basename "$snap"): $needle)" \
      "$(cat "$snap")" "$needle"
  done
done

# ── AC5: main jj workspace output is byte-identical to golden ─────────────────
echo "Test [AC5]: main jj workspace output byte-identical to golden"
d=$(make_main_jj_workspace)
OUTPUT=$(run_hook "$d")
GOLDEN=$(cat "$FIXTURE_ROOT/main-jj-workspace.json")
assert_eq "main jj output unchanged" "$GOLDEN" "$OUTPUT"
# Defence-in-depth: the boundary block must never leak into a main checkout
# even if the golden is ever rebaselined incorrectly.
assert_not_contains "no boundary header (main jj)" "$OUTPUT" "WORKSPACE BOUNDARY DETECTED"
assert_not_contains "no boundary field (main jj)" "$OUTPUT" "Boundary (active workspace):"
assert_not_contains "no parent field (main jj)" "$OUTPUT" "Parent repository"

# ── AC5: main git checkout output is byte-identical to golden ─────────────────
echo "Test [AC5]: main git checkout output byte-identical to golden"
d=$(make_main_git_checkout)
OUTPUT=$(run_hook "$d")
GOLDEN=$(cat "$FIXTURE_ROOT/main-git-checkout.json")
assert_eq "main git output unchanged" "$GOLDEN" "$OUTPUT"
assert_not_contains "no boundary header (main git)" "$OUTPUT" "WORKSPACE BOUNDARY DETECTED"
assert_not_contains "no boundary field (main git)" "$OUTPUT" "Boundary (active workspace):"
assert_not_contains "no parent field (main git)" "$OUTPUT" "Parent repository"

# ── AC6: plain non-repo directory — exits 0, empty stderr, valid JSON,
#        no boundary content for any of the three prohibition phrases. ─────────
echo "Test [AC6]: plain non-repo directory exits 0 with no boundary content"
d=$(new_workdir)
STDOUT_FILE=$(mktemp); STDERR_FILE=$(mktemp)
RC=0
(cd "$d" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$HOOK") \
  > "$STDOUT_FILE" 2> "$STDERR_FILE" || RC=$?
STDOUT=$(cat "$STDOUT_FILE"); STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
assert_eq "empty stderr" "" "$STDERR"
# Stdout, if non-empty, must be valid JSON parseable by jq.
if [ -n "$STDOUT" ]; then
  echo "$STDOUT" | jq -e . >/dev/null \
    || { echo "FAIL: AC6 stdout is not valid JSON" >&2; exit 1; }
fi
# All three AC1 prohibition phrases must be absent — not just `edit`.
assert_not_contains "no edit prohibition" "$STDOUT" "do not edit files in"
assert_not_contains "no vcs prohibition" "$STDOUT" "do not run VCS commands against"
assert_not_contains "no research prohibition" "$STDOUT" "do not grep, find, or research files in"
assert_not_contains "no boundary header" "$STDOUT" "WORKSPACE BOUNDARY DETECTED"

source "$PLUGIN_ROOT/scripts/vcs-common.sh"

echo "=== vcs-common.sh helpers ==="

# ── _jj_workspace_is_secondary (jj internal-marker isolation function) ────────
echo "Test [AC7]: _jj_workspace_is_secondary returns 1 in a main workspace"
d=$(make_main_jj_workspace)
RC=0; _jj_workspace_is_secondary "$d" || RC=$?
assert_eq "main workspace returns 1" "1" "$RC"

echo "Test [AC7]: _jj_workspace_is_secondary returns 0 in a secondary workspace"
make_jj_secondary_workspace
RC=0; _jj_workspace_is_secondary "$FIXTURE_SECONDARY" || RC=$?
assert_eq "secondary workspace returns 0" "0" "$RC"

# ── find_jj_main_workspace_root ───────────────────────────────────────────────
echo "Test [AC7]: find_jj_main_workspace_root in a main jj workspace"
d=$(make_main_jj_workspace)
RESULT=$( (cd "$d" && find_jj_main_workspace_root .) )
assert_eq "returns the workspace root" "$d" "$RESULT"

echo "Test [AC7]: find_jj_main_workspace_root in a jj secondary workspace"
make_jj_secondary_workspace
RESULT=$( (cd "$FIXTURE_SECONDARY" && find_jj_main_workspace_root .) )
assert_eq "returns the parent main workspace" "$FIXTURE_PARENT" "$RESULT"

# Failure-mode contract: plain non-repo dir must return exit 1, empty stdout.
echo "Test [AC7]: find_jj_main_workspace_root failure in a plain directory"
d=$(new_workdir)
RC=0; RESULT=$( (cd "$d" && find_jj_main_workspace_root .) ) || RC=$?
assert_eq "exits 1 (plain)" "1" "$RC"
assert_eq "empty stdout (plain)" "" "$RESULT"

# ── find_git_main_worktree_root ───────────────────────────────────────────────
echo "Test [AC7]: find_git_main_worktree_root in a main git checkout"
d=$(make_main_git_checkout)
RESULT=$( (cd "$d" && find_git_main_worktree_root .) )
assert_eq "returns the checkout root" "$d" "$RESULT"

echo "Test [AC7]: find_git_main_worktree_root in a git linked worktree"
make_git_linked_worktree
RESULT=$( (cd "$FIXTURE_WORKTREE" && find_git_main_worktree_root .) )
assert_eq "returns the parent main checkout" "$FIXTURE_PARENT" "$RESULT"

# Failure-mode contracts: plain non-repo and bare-repo → exit 1, empty stdout.
echo "Test [AC7]: find_git_main_worktree_root failure in a plain directory"
d=$(new_workdir)
RC=0; RESULT=$( (cd "$d" && find_git_main_worktree_root .) ) || RC=$?
assert_eq "exits 1 (plain)" "1" "$RC"
assert_eq "empty stdout (plain)" "" "$RESULT"

echo "Test [AC7]: find_git_main_worktree_root failure in a bare git repo"
d=$(make_bare_git_repo)
RC=0; RESULT=$( (cd "$d" && find_git_main_worktree_root .) ) || RC=$?
assert_eq "exits 1 (bare)" "1" "$RC"
assert_eq "empty stdout (bare)" "" "$RESULT"

# ── classify_checkout — structured KEY=VALUE record ──────────────────────────
# Parser sets globals C_KIND, C_BOUNDARY, C_JJ_PARENT, C_GIT_PARENT,
# C_JJ_MISSING, C_GIT_MISSING.
parse_classification() {
  C_KIND=""; C_BOUNDARY=""; C_JJ_PARENT=""; C_GIT_PARENT=""
  C_JJ_MISSING="0"; C_GIT_MISSING="0"
  while IFS='=' read -r k v; do
    case "$k" in
      KIND) C_KIND=$v ;;
      BOUNDARY) C_BOUNDARY=$v ;;
      JJ_PARENT) C_JJ_PARENT=$v ;;
      GIT_PARENT) C_GIT_PARENT=$v ;;
      JJ_MISSING) C_JJ_MISSING=$v ;;
      GIT_MISSING) C_GIT_MISSING=$v ;;
    esac
  done <<< "$1"
}

echo "Test [AC7]: classify_checkout KIND=main (jj)"
d=$(make_main_jj_workspace)
parse_classification "$( (cd "$d" && classify_checkout .) )"
assert_eq "KIND=main" "main" "$C_KIND"
assert_eq "BOUNDARY empty" "" "$C_BOUNDARY"
assert_eq "JJ_PARENT empty" "" "$C_JJ_PARENT"
assert_eq "GIT_PARENT empty" "" "$C_GIT_PARENT"

echo "Test [AC7]: classify_checkout KIND=main (git)"
d=$(make_main_git_checkout)
parse_classification "$( (cd "$d" && classify_checkout .) )"
assert_eq "KIND=main" "main" "$C_KIND"
assert_eq "BOUNDARY empty" "" "$C_BOUNDARY"

echo "Test [AC7]: classify_checkout KIND=jj-secondary"
make_jj_secondary_workspace
parse_classification "$( (cd "$FIXTURE_SECONDARY" && classify_checkout .) )"
assert_eq "KIND=jj-secondary" "jj-secondary" "$C_KIND"
assert_eq "BOUNDARY=secondary" "$FIXTURE_SECONDARY" "$C_BOUNDARY"
assert_eq "JJ_PARENT=parent" "$FIXTURE_PARENT" "$C_JJ_PARENT"
assert_eq "GIT_PARENT empty" "" "$C_GIT_PARENT"

echo "Test [AC7]: classify_checkout KIND=git-worktree"
make_git_linked_worktree
parse_classification "$( (cd "$FIXTURE_WORKTREE" && classify_checkout .) )"
assert_eq "KIND=git-worktree" "git-worktree" "$C_KIND"
assert_eq "BOUNDARY=worktree" "$FIXTURE_WORKTREE" "$C_BOUNDARY"
assert_eq "GIT_PARENT=parent" "$FIXTURE_PARENT" "$C_GIT_PARENT"
assert_eq "JJ_PARENT empty" "" "$C_JJ_PARENT"

echo "Test [AC7]: classify_checkout KIND=colocated"
make_colocated_secondary
parse_classification "$( (cd "$FIXTURE_TARGET" && classify_checkout .) )"
assert_eq "KIND=colocated" "colocated" "$C_KIND"
assert_eq "BOUNDARY=target" "$FIXTURE_TARGET" "$C_BOUNDARY"
assert_eq "JJ_PARENT=jj_parent" "$FIXTURE_JJ_PARENT" "$C_JJ_PARENT"
assert_eq "GIT_PARENT=git_parent" "$FIXTURE_GIT_PARENT" "$C_GIT_PARENT"

echo "Test [AC7]: classify_checkout KIND=none in a plain directory"
d=$(new_workdir)
parse_classification "$( (cd "$d" && classify_checkout .) )"
assert_eq "KIND=none" "none" "$C_KIND"
assert_eq "BOUNDARY empty" "" "$C_BOUNDARY"

echo "Test [AC7]: classify_checkout KIND=none in a bare git repo"
d=$(make_bare_git_repo)
parse_classification "$( (cd "$d" && classify_checkout .) )"
assert_eq "KIND=none (bare)" "none" "$C_KIND"

# ── classify_checkout missing-binary diagnostic fields ────────────────────────
# When a VCS binary is absent AND the directory is inside that VCS's
# checkout tree, JJ_MISSING / GIT_MISSING should be set. We mask the
# binary by stripping its directory from PATH, scoped to a subshell
# wrapper (`( PATH=...; cd ...; ... )`) so the modified PATH applies to
# BOTH `cd` AND `classify_checkout`. The plain `VAR=val cmd1 && cmd2`
# form only scopes VAR to cmd1, which would defeat the test.
echo "Test [AC7]: classify_checkout JJ_MISSING=1 in jj secondary with jj absent"
make_jj_secondary_workspace
NEW_PATH=$(strip_binary_from_path jj)
parse_classification "$( ( PATH="$NEW_PATH"; cd "$FIXTURE_SECONDARY" && classify_checkout . ) )"
# (With jj absent the structured record collapses KIND toward `none`
# or `git-worktree`, but JJ_MISSING is 1 because an ancestor has a
# .jj marker — exactly the signal the hook's diagnostic needs.)
assert_eq "JJ_MISSING=1" "1" "$C_JJ_MISSING"

echo "Test [AC7]: classify_checkout JJ_MISSING=0 in a plain dir even when jj absent"
d=$(new_workdir)
parse_classification "$( ( PATH="$NEW_PATH"; cd "$d" && classify_checkout . ) )"
assert_eq "JJ_MISSING=0 (no ancestor marker)" "0" "$C_JJ_MISSING"

echo "Test [AC7]: classify_checkout GIT_MISSING=1 in git checkout with git absent"
d=$(make_main_git_checkout)
NEW_PATH=$(strip_binary_from_path git)
parse_classification "$( ( PATH="$NEW_PATH"; cd "$d" && classify_checkout . ) )"
assert_eq "GIT_MISSING=1" "1" "$C_GIT_MISSING"

# ── find_repo_root unchanged-behaviour regression guard ───────────────────────
# find_repo_root is deliberately not refactored by this work. Lock in its
# current behaviour across the well-defined fixture cases so a future
# accidental edit to vcs-common.sh is caught immediately.
echo "Test [AC7]: find_repo_root unchanged in main jj workspace"
d=$(make_main_jj_workspace)
RESULT=$( (cd "$d" && find_repo_root) )
assert_eq "main jj" "$d" "$RESULT"

echo "Test [AC7]: find_repo_root unchanged in main git checkout"
d=$(make_main_git_checkout)
RESULT=$( (cd "$d" && find_repo_root) )
assert_eq "main git" "$d" "$RESULT"

echo "Test [AC7]: find_repo_root unchanged in jj secondary workspace"
make_jj_secondary_workspace
RESULT=$( (cd "$FIXTURE_SECONDARY" && find_repo_root) )
# .jj is a directory in a jj secondary workspace, so find_repo_root finds it.
assert_eq "jj secondary" "$FIXTURE_SECONDARY" "$RESULT"
# (We deliberately do NOT lock in find_repo_root's behaviour for git linked
# worktrees: .git is a file there, find_repo_root's -d test skips it, and
# the result is implementation-detail. Leaving the assertion off keeps room
# for a future fix without breaking this regression guard.)

echo "=== boundary block: jj secondary and git linked worktree ==="

# Extract additionalContext from the hook's JSON envelope.
extract_context() {
  jq -r '.hookSpecificOutput.additionalContext' <<< "$1"
}

# ── AC1: jj secondary workspace boundary block ────────────────────────────────
echo "Test [AC1]: jj secondary workspace emits boundary block"
make_jj_secondary_workspace
OUTPUT=$(run_hook "$FIXTURE_SECONDARY")
CTX=$(extract_context "$OUTPUT")
assert_contains "boundary header" "$CTX" "WORKSPACE BOUNDARY DETECTED"
assert_contains "workspace path present" "$CTX" "Boundary (active workspace): $FIXTURE_SECONDARY"
assert_contains "jj parent labelled" "$CTX" "Parent repository (jj): $FIXTURE_PARENT"
assert_contains "edit prohibition" "$CTX" "do not edit files in $FIXTURE_PARENT"
assert_contains "vcs prohibition" "$CTX" "do not run VCS commands against $FIXTURE_PARENT"
assert_contains "research prohibition" "$CTX" "do not grep, find, or research files in $FIXTURE_PARENT"

# ── AC2: git linked worktree boundary block ───────────────────────────────────
echo "Test [AC2]: git linked worktree emits boundary block"
make_git_linked_worktree
OUTPUT=$(run_hook "$FIXTURE_WORKTREE")
CTX=$(extract_context "$OUTPUT")
assert_contains "boundary header" "$CTX" "WORKSPACE BOUNDARY DETECTED"
assert_contains "worktree path present" "$CTX" "Boundary (active workspace): $FIXTURE_WORKTREE"
assert_contains "git parent labelled" "$CTX" "Parent repository (git): $FIXTURE_PARENT"
assert_contains "edit prohibition" "$CTX" "do not edit files in $FIXTURE_PARENT"
assert_contains "vcs prohibition" "$CTX" "do not run VCS commands against $FIXTURE_PARENT"
assert_contains "research prohibition" "$CTX" "do not grep, find, or research files in $FIXTURE_PARENT"

echo ""
test_summary
