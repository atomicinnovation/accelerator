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
  printf '%s\n' "$FIXTURE_JJ_PARENT/.jj/repo" > "$target/.jj/repo"
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

echo ""
test_summary
