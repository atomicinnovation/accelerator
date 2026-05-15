#!/usr/bin/env bash

# Shared VCS utility functions sourced by hooks and wrapper scripts.
# This is the single source of truth for repo-root detection logic.

# Find the repository root by walking up the directory tree.
# Outputs the root path and returns 0 if found, returns 1 if not.
find_repo_root() {
  local dir="$PWD"
  while [ "$dir" != "/" ]; do
    if [ -d "$dir/.jj" ] || [ -d "$dir/.git" ]; then
      echo "$dir"
      return 0
    fi
    dir="$(dirname "$dir")"
  done
  return 1
}

# ─────────────────────────────────────────────────────────────────────────────
# Internal: does any ancestor of <dir> (inclusive) contain a <marker> entry?
# Exit 0 if found, 1 otherwise. Used ONLY by classify_checkout's missing-
# binary diagnostic fallback to detect the "binary absent but the user is
# inside a VCS checkout" case — NOT by primary detection logic, which uses
# authoritative VCS probes per the work item's §4 prohibition on path-
# walking for classification.
_ancestor_has_marker() {
  local dir="$1" marker="$2"
  dir=$(cd "$dir" 2>/dev/null && pwd) || return 1
  while [ -n "$dir" ] && [ "$dir" != "/" ]; do
    [ -e "$dir/$marker" ] && return 0
    # Use parameter expansion (a shell builtin) rather than `dirname`,
    # which lives in /usr/bin and may be unavailable when callers strip
    # paths from PATH to simulate a missing VCS binary.
    case "$dir" in
      */*) dir="${dir%/*}"; [ -z "$dir" ] && dir="/" ;;
      *) dir="" ;;
    esac
  done
  return 1
}

# ─────────────────────────────────────────────────────────────────────────────
# Internal: is the jj workspace at <workspace_root> a secondary workspace?
# Returns exit 0 if YES, 1 if NO (or marker missing).
#
# This is the SINGLE PLACE the jj-internal .jj/repo file-vs-directory marker
# is interpreted. When `jj workspace repo-root` lands upstream
# (jj-vcs/jj#8758), update this one function to invoke the new CLI and
# every caller picks up the new contract automatically. The work item
# acknowledges this coupling explicitly; isolating it here keeps the
# blast radius at one line.
_jj_workspace_is_secondary() {
  local workspace_root="$1"
  local marker="$workspace_root/.jj/repo"
  # Main workspace marker: .jj/repo is a directory.
  # Secondary workspace marker: .jj/repo is a file whose contents are a
  # relative path back to the main repo's .jj/repo directory.
  [ -f "$marker" ]
}

# ─────────────────────────────────────────────────────────────────────────────
# Find the main jj workspace root (the canonical jj repo) for a given
# directory. Works whether <dir> is inside the main workspace or a
# secondary workspace. Returns the realpath of the main workspace root on
# stdout and exits 0; exits 1 with empty stdout if jj is unavailable,
# <dir> is not inside any jj workspace, or the resolved secondary path
# does not satisfy the main-workspace invariant.
find_jj_main_workspace_root() {
  local dir="${1:-$PWD}"
  command -v jj >/dev/null 2>&1 || return 1
  local workspace_root
  workspace_root=$(cd "$dir" 2>/dev/null && jj workspace root 2>/dev/null) || return 1
  [ -n "$workspace_root" ] || return 1
  if ! _jj_workspace_is_secondary "$workspace_root"; then
    # Main workspace — workspace_root IS the answer.
    realpath "$workspace_root"
    return 0
  fi
  # Secondary workspace: read .jj/repo (relative path), resolve, walk up.
  local marker="$workspace_root/.jj/repo"
  local rel main_repo
  rel=$(cat "$marker") || return 1
  main_repo=$(cd "$workspace_root/.jj" && cd "$rel" 2>/dev/null && pwd) || return 1
  # main_repo points at <main>/.jj/repo; main workspace root is two-up.
  local candidate
  candidate=$(realpath "$main_repo/../..") || return 1
  # Defensive invariant: the resolved candidate must itself look like a
  # main workspace (so a future jj layout change cannot silently produce
  # a wrong-but-non-empty answer).
  [ -d "$candidate/.jj/repo" ] || return 1
  printf '%s\n' "$candidate"
}

# ─────────────────────────────────────────────────────────────────────────────
# Find the main git worktree root for a given directory. Returns the
# realpath on stdout and exits 0 on success; exits 1 with empty stdout in
# any of these cases:
#   - git unavailable
#   - <dir> is not inside a git repo
#   - bare repository (no main worktree exists)
#   - GIT_DIR is set in the caller's environment (untrusted)
# For submodules, defers to `git rev-parse --show-superproject-working-tree`
# so the returned root is the superproject's worktree, not the gitdir parent.
# Requires git >= 2.5 for --git-common-dir / --show-superproject-working-tree.
find_git_main_worktree_root() {
  local dir="${1:-$PWD}"
  command -v git >/dev/null 2>&1 || return 1
  # Scrub a caller-set GIT_DIR: re-enter with the variable explicitly
  # cleared so probe results cannot be poisoned by ambient env.
  if [ -n "${GIT_DIR:-}" ]; then
    GIT_DIR="" find_git_main_worktree_root "$dir"
    return $?
  fi
  # Bare repos have no main worktree.
  if [ "$(cd "$dir" 2>/dev/null && git rev-parse --is-bare-repository 2>/dev/null)" = "true" ]; then
    return 1
  fi
  # Submodules: the superproject's worktree is the answer if present.
  local super
  super=$(cd "$dir" 2>/dev/null && git rev-parse --show-superproject-working-tree 2>/dev/null || true)
  if [ -n "$super" ]; then
    realpath "$super"
    return 0
  fi
  local common_dir
  common_dir=$(cd "$dir" 2>/dev/null && git rev-parse --git-common-dir 2>/dev/null) || return 1
  [ -n "$common_dir" ] || return 1
  # When --git-common-dir is relative, it is relative to PWD.
  if [ "${common_dir#/}" = "$common_dir" ]; then
    common_dir="$(cd "$dir" && cd "$common_dir" && pwd)"
  fi
  realpath "$(dirname "$common_dir")"
}

# ─────────────────────────────────────────────────────────────────────────────
# Classify the checkout kind of a given directory. Always exits 0 — the
# classification is the output, not the status. Prints a six-line
# KEY=VALUE record on stdout that callers parse via
# `while IFS='=' read -r k v; do ... done`:
#
#   KIND=<one of: main, jj-secondary, git-worktree, colocated,
#                  nested-jj-in-git, nested-git-in-jj, none>
#   BOUNDARY=<realpath of the active workspace; empty for main and none>
#   JJ_PARENT=<realpath of the jj parent repo; empty if not applicable>
#   GIT_PARENT=<realpath of the git parent repo; empty if not applicable>
#   JJ_MISSING=<1 if jj is not on PATH AND an ancestor of dir has a .jj
#               marker; else 0>
#   GIT_MISSING=<1 if git is not on PATH AND an ancestor of dir has a
#                .git marker; else 0>
#
# Note that paths containing `=` are not supported by the parser idiom
# above. Path values are never percent-encoded; the contract assumes
# realistic project paths. (The same constraint applies to paths
# containing literal newlines.)
classify_checkout() {
  local dir="${1:-$PWD}"
  local in_jj=0 jj_secondary=0 jj_main_root="" jj_workspace_root="" jj_missing=0
  local in_git=0 git_worktree=0 git_main_root="" git_worktree_root="" git_missing=0

  # ── jj probe ─────────────────────────────────────────────────────────────
  if command -v jj >/dev/null 2>&1; then
    if jj_workspace_root=$(cd "$dir" 2>/dev/null && jj workspace root 2>/dev/null) \
       && [ -n "$jj_workspace_root" ]; then
      in_jj=1
      if _jj_workspace_is_secondary "$jj_workspace_root"; then
        jj_secondary=1
        jj_main_root=$(find_jj_main_workspace_root "$dir") || jj_main_root=""
      else
        jj_main_root=$(realpath "$jj_workspace_root")
      fi
    fi
  else
    # Binary absent: detection cannot run. Use a single-purpose ancestor
    # path-walk to flag whether the user would have hit a jj workspace if
    # jj were installed, so the hook can emit a missing-binary diagnostic
    # rather than silently degrading. This walk is for DIAGNOSTIC ONLY —
    # detection still requires authoritative probes per the work item.
    _ancestor_has_marker "$dir" .jj && jj_missing=1
  fi

  # ── git probe ────────────────────────────────────────────────────────────
  if command -v git >/dev/null 2>&1; then
    local git_dir git_common_dir is_bare
    is_bare=$(cd "$dir" 2>/dev/null && git rev-parse --is-bare-repository 2>/dev/null || true)
    if [ "$is_bare" != "true" ] \
       && git_dir=$(cd "$dir" 2>/dev/null && git rev-parse --git-dir 2>/dev/null) \
       && [ -n "$git_dir" ]; then
      git_common_dir=$(cd "$dir" && git rev-parse --git-common-dir 2>/dev/null || true)
      if [ -n "$git_common_dir" ]; then
        in_git=1
        # Absolutise both via cd/pwd. After this, the two strings can be
        # compared directly — no second realpath round needed.
        [ "${git_dir#/}" = "$git_dir" ] && git_dir="$(cd "$dir" && cd "$git_dir" && pwd)"
        [ "${git_common_dir#/}" = "$git_common_dir" ] && git_common_dir="$(cd "$dir" && cd "$git_common_dir" && pwd)"
        if [ "$git_dir" != "$git_common_dir" ]; then
          git_worktree=1
        fi
        git_worktree_root=$(cd "$dir" && realpath "$(git rev-parse --show-toplevel)")
        git_main_root=$(find_git_main_worktree_root "$dir") || git_main_root=""
      fi
    fi
  else
    # Diagnostic-only ancestor walk (see jj branch above for rationale).
    _ancestor_has_marker "$dir" .git && git_missing=1
  fi

  # ── Classify ─────────────────────────────────────────────────────────────
  local kind="" boundary="" jj_parent="" git_parent=""

  # Arm-ordering is load-bearing: `colocated` must precede the `nested-*`
  # arms because a true colocated checkout also satisfies the nested
  # predicates (in_jj=1 && in_git=1 with different jj_main_root and
  # git_main_root); first-match-wins on the case-cascade picks the right
  # one. All multi-parent arms gate on `[ -n $jj_main_root ] &&
  # [ -n $git_main_root ]` so a defensive-invariant failure inside
  # find_jj_main_workspace_root degrades gracefully to a single-VCS arm
  # rather than emitting a misleading multi-parent record.
  if [ $in_jj -eq 0 ] && [ $in_git -eq 0 ]; then
    kind="none"
  elif [ $jj_secondary -eq 1 ] && [ $git_worktree -eq 1 ] \
       && [ -n "$jj_main_root" ] && [ -n "$git_main_root" ]; then
    kind="colocated"
    boundary=$(realpath "$jj_workspace_root")
    jj_parent="$jj_main_root"
    git_parent="$git_main_root"
  elif [ $jj_secondary -eq 1 ] && [ $in_git -eq 1 ] \
       && [ -n "$jj_main_root" ] && [ -n "$git_main_root" ] \
       && [ "$jj_main_root" != "$git_main_root" ]; then
    kind="nested-jj-in-git"
    boundary=$(realpath "$jj_workspace_root")
    jj_parent="$jj_main_root"
    git_parent="$git_main_root"
  elif [ $git_worktree -eq 1 ] && [ $in_jj -eq 1 ] \
       && [ -n "$jj_main_root" ] && [ -n "$git_main_root" ] \
       && [ "$jj_main_root" != "$git_main_root" ]; then
    kind="nested-git-in-jj"
    boundary="$git_worktree_root"
    jj_parent="$jj_main_root"
    git_parent="$git_main_root"
  elif [ $jj_secondary -eq 1 ]; then
    kind="jj-secondary"
    boundary=$(realpath "$jj_workspace_root")
    jj_parent="$jj_main_root"
  elif [ $git_worktree -eq 1 ]; then
    kind="git-worktree"
    boundary="$git_worktree_root"
    git_parent="$git_main_root"
  else
    kind="main"
  fi

  printf 'KIND=%s\n' "$kind"
  printf 'BOUNDARY=%s\n' "$boundary"
  printf 'JJ_PARENT=%s\n' "$jj_parent"
  printf 'GIT_PARENT=%s\n' "$git_parent"
  printf 'JJ_MISSING=%s\n' "$jj_missing"
  printf 'GIT_MISSING=%s\n' "$git_missing"
}
