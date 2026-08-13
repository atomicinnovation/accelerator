#!/usr/bin/env bash
set -euo pipefail

# Parity gate for the `vcs detect` SessionStart hook, dispatched through the
# real `accelerator` launcher (ACCELERATOR_BIN) with ACCELERATOR_VCS_BIN set
# so dispatch resolves to the locally-built accelerator-vcs sub-binary rather
# than fetching a signed release asset. This is the end-to-end path a real
# Claude Code session exercises via hooks/hooks.json's SessionStart entry.
#
# The in-process scripts/vcs-common.sh coverage this suite used to carry
# (classify_checkout, find_repo_root, vcs_mode, ...) moved to
# scripts/test-vcs-common.sh — those helpers keep their other ~20 callers and
# are unaffected by the accelerator-vcs port.

if [ -z "${BASH_VERSION:-}" ]; then
  echo "hooks/test-vcs-detect.sh requires bash" >&2
  exit 1
fi
for tool in jj git realpath jq; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "hooks/test-vcs-detect.sh requires $tool on PATH (run via 'mise run test:integration:hooks' or install $tool)" >&2
    exit 77 # autotools 'skip' convention; harness reports as skipped
  fi
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FIXTURE_ROOT="$PLUGIN_ROOT/hooks/test-fixtures/vcs-detect"
ACCELERATOR_BIN="${ACCELERATOR_BIN:-$PLUGIN_ROOT/cli/target/debug/accelerator}"
ACCELERATOR_VCS_BIN="${ACCELERATOR_VCS_BIN:-$PLUGIN_ROOT/cli/target/debug/accelerator-vcs}"
# Suppresses the launcher's own INFO-level "resolving via ACCELERATOR_<SUB>_BIN
# override" diagnostic (cli/launcher/src/launch/outbound/mod.rs), which fires
# only on this dev-only override path this suite deliberately exercises to
# reach the locally-built accelerator-vcs — a real production dispatch (no
# override set) never emits it, so silencing it here keeps the "empty
# stderr" assertions below testing the same thing they always have: a clean
# run, not this suite's own dev-dispatch mechanics.
export ACCELERATOR_LOG="${ACCELERATOR_LOG:-warn}"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

for binary in "$ACCELERATOR_BIN" "$ACCELERATOR_VCS_BIN"; do
  if [ ! -x "$binary" ]; then
    echo "hooks/test-vcs-detect.sh requires $binary (run 'mise run build:cli:dev' or 'mise run test:integration:hooks', which depends on it)" >&2
    exit 77
  fi
done

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
  # See scripts/test-vcs-common.sh's copy of this builder for the graft-step
  # rationale (both `git worktree add` and `jj workspace add` refuse an
  # existing non-empty target).
  FIXTURE_JJ_PARENT="" FIXTURE_GIT_PARENT="" FIXTURE_TARGET=""
  FIXTURE_JJ_PARENT=$(make_main_jj_workspace)
  FIXTURE_GIT_PARENT=$(make_main_git_checkout)
  local target jj_tmp
  target=$(new_workdir)
  rm -rf "$target"
  (cd "$FIXTURE_GIT_PARENT" && git worktree add -q "$target")
  jj_tmp=$(new_workdir)
  rm -rf "$jj_tmp"
  (cd "$FIXTURE_JJ_PARENT" && jj workspace add --quiet "$jj_tmp")
  mv "$jj_tmp/.jj" "$target/.jj"
  printf '%s' "$FIXTURE_JJ_PARENT/.jj/repo" >"$target/.jj/repo"
  rm -rf "$jj_tmp"
  FIXTURE_TARGET=$(realpath "$target")
  [ -f "$FIXTURE_TARGET/.jj/repo" ] || {
    echo "colocated fixture missing .jj/repo file" >&2
    exit 1
  }
  [ -e "$FIXTURE_TARGET/.git" ] || {
    echo "colocated fixture missing .git marker" >&2
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
# inside a pure-jj parent. Exercises the nested-git-in-jj classification.
make_git_worktree_in_jj_parent() {
  FIXTURE_JJ_PARENT="" FIXTURE_GIT_PARENT="" FIXTURE_TARGET=""
  FIXTURE_JJ_PARENT=$(make_main_jj_workspace)
  FIXTURE_GIT_PARENT=$(make_main_git_checkout)
  local target="$FIXTURE_JJ_PARENT/sub"
  # git worktree add requires a non-existent target.
  (cd "$FIXTURE_GIT_PARENT" && git worktree add -q "$target")
  FIXTURE_TARGET=$(realpath "$target")
}

run_hook() {
  local cwd="$1"
  (
    cd "$cwd" && ACCELERATOR_VCS_BIN="$ACCELERATOR_VCS_BIN" "$ACCELERATOR_BIN" \
      vcs detect --format=hook --fail-safe --descriptive
  )
}

# Extract additionalContext from the hook's JSON envelope.
extract_context() {
  jq -r '.hookSpecificOutput.additionalContext' <<<"$1"
}

echo "=== accelerator vcs detect ==="
echo ""

# ── golden snapshots are free of host-specific path artefacts ─────────────────
# (Determinism guard: rejects snapshots accidentally regenerated on a host
# whose TMPDIR resolves under /private/var, /var/folders, or a $HOME path.)
# These are the same goldens cli/vcs-cli/tests/detect_goldens.rs compares the
# compiled accelerator-vcs binary's output against.
echo "Test [goldens]: golden snapshots free of host-specific path artefacts"
for snap in "$FIXTURE_ROOT/main-jj-workspace.json" "$FIXTURE_ROOT/main-git-checkout.json"; do
  for needle in '/private/var' '/var/folders' '/Users/' '/home/'; do
    assert_not_contains "no host artefact ($(basename "$snap"): $needle)" \
      "$(cat "$snap")" "$needle"
  done
done

# ── main jj workspace output matches the golden ────────────────────────────────
# jq -S . canonicalises both sides before comparing: the goldens are committed
# pretty-printed, kernel::hooks builds compact JSON by hand, and the two are
# equivalent, not byte-identical — the same comparison
# cli/vcs-cli/tests/detect_goldens.rs makes (there via parsed serde_json::Value
# equality; here via canonical-text equality, since this suite runs the
# compiled binary through the real launcher dispatch rather than linking it).
echo "Test [goldens]: main jj workspace output matches the golden"
d=$(make_main_jj_workspace)
OUTPUT=$(run_hook "$d")
GOLDEN=$(jq -S . "$FIXTURE_ROOT/main-jj-workspace.json")
assert_eq "main jj output unchanged" "$GOLDEN" "$(jq -S . <<<"$OUTPUT")"
# Defence-in-depth: the boundary block must never leak into a main checkout
# even if the golden is ever rebaselined incorrectly.
assert_not_contains "no boundary header (main jj)" "$OUTPUT" "WORKSPACE BOUNDARY DETECTED"
assert_not_contains "no boundary field (main jj)" "$OUTPUT" "Boundary (active workspace):"
assert_not_contains "no parent field (main jj)" "$OUTPUT" "Parent repository"

# ── main git checkout output matches the golden ────────────────────────────────
echo "Test [goldens]: main git checkout output matches the golden"
d=$(make_main_git_checkout)
OUTPUT=$(run_hook "$d")
GOLDEN=$(jq -S . "$FIXTURE_ROOT/main-git-checkout.json")
assert_eq "main git output unchanged" "$GOLDEN" "$(jq -S . <<<"$OUTPUT")"
assert_not_contains "no boundary header (main git)" "$OUTPUT" "WORKSPACE BOUNDARY DETECTED"
assert_not_contains "no boundary field (main git)" "$OUTPUT" "Boundary (active workspace):"
assert_not_contains "no parent field (main git)" "$OUTPUT" "Parent repository"

# ── Plain non-repo directory — exits 0, empty stderr, valid JSON,
#        no boundary content for any of the three prohibition phrases. ─────────
echo "Test [non-repo]: plain non-repo directory exits 0 with no boundary content"
d=$(new_workdir)
STDOUT_FILE=$(mktemp)
STDERR_FILE=$(mktemp)
RC=0
(
  cd "$d" && ACCELERATOR_VCS_BIN="$ACCELERATOR_VCS_BIN" "$ACCELERATOR_BIN" \
    vcs detect --format=hook --fail-safe --descriptive
) >"$STDOUT_FILE" 2>"$STDERR_FILE" || RC=$?
STDOUT=$(cat "$STDOUT_FILE")
STDERR=$(cat "$STDERR_FILE")
rm -f "$STDOUT_FILE" "$STDERR_FILE"
assert_eq "exit 0" "0" "$RC"
assert_eq "empty stderr" "" "$STDERR"
# Stdout, if non-empty, must be valid JSON parseable by jq.
if [ -n "$STDOUT" ]; then
  echo "$STDOUT" | jq -e . >/dev/null ||
    {
      echo "FAIL: non-repo stdout is not valid JSON" >&2
      exit 1
    }
fi
# All three prohibition phrases must be absent — not just `edit`.
assert_not_contains "no edit prohibition" "$STDOUT" "do not edit files in"
assert_not_contains "no vcs prohibition" "$STDOUT" "do not run VCS commands against"
assert_not_contains "no research prohibition" "$STDOUT" "do not grep, find, or research files in"
assert_not_contains "no boundary header" "$STDOUT" "WORKSPACE BOUNDARY DETECTED"

echo "=== boundary block: jj secondary and git linked worktree ==="

# ── jj secondary workspace boundary block ───────────────────────────────────────────────────────────────────
echo "Test [jj-workspace]: jj secondary workspace emits boundary block"
make_jj_secondary_workspace
OUTPUT=$(run_hook "$FIXTURE_SECONDARY")
CTX=$(extract_context "$OUTPUT")
assert_contains "boundary header" "$CTX" "WORKSPACE BOUNDARY DETECTED"
assert_contains "workspace path present" "$CTX" "Boundary (active workspace): $FIXTURE_SECONDARY"
assert_contains "jj parent labelled" "$CTX" "Parent repository (jj): $FIXTURE_PARENT"
assert_contains "edit prohibition" "$CTX" "do not edit files in $FIXTURE_PARENT"
assert_contains "vcs prohibition" "$CTX" "do not run VCS commands against $FIXTURE_PARENT"
assert_contains "research prohibition" "$CTX" "do not grep, find, or research files in $FIXTURE_PARENT"

# ── git linked worktree boundary block ─────────────────────────────────────────────────────────────────────────
echo "Test [git-worktree]: git linked worktree emits boundary block"
make_git_linked_worktree
OUTPUT=$(run_hook "$FIXTURE_WORKTREE")
CTX=$(extract_context "$OUTPUT")
assert_contains "boundary header" "$CTX" "WORKSPACE BOUNDARY DETECTED"
assert_contains "worktree path present" "$CTX" "Boundary (active workspace): $FIXTURE_WORKTREE"
assert_contains "git parent labelled" "$CTX" "Parent repository (git): $FIXTURE_PARENT"
assert_contains "edit prohibition" "$CTX" "do not edit files in $FIXTURE_PARENT"
assert_contains "vcs prohibition" "$CTX" "do not run VCS commands against $FIXTURE_PARENT"
assert_contains "research prohibition" "$CTX" "do not grep, find, or research files in $FIXTURE_PARENT"

echo "=== boundary block: colocated and cross-VCS ==="

# ── Colocated — single block, both parents named separately ──────────────
echo "Test [colocated]: colocated checkout emits single block with both parents"
make_colocated_secondary
OUTPUT=$(run_hook "$FIXTURE_TARGET")
CTX=$(extract_context "$OUTPUT")
# Exactly one boundary line, with the shared target path as its value.
COUNT=$(grep -c "Boundary (active workspace): $FIXTURE_TARGET" <<<"$CTX" || true)
assert_eq "exactly one boundary line" "1" "$COUNT"
assert_contains "jj parent labelled" "$CTX" "Parent repository (jj): $FIXTURE_JJ_PARENT"
assert_contains "git parent labelled" "$CTX" "Parent repository (git): $FIXTURE_GIT_PARENT"
# Both sets of canonical prohibitions present (full phrases, not just keywords).
assert_contains "jj edit" "$CTX" "do not edit files in $FIXTURE_JJ_PARENT"
assert_contains "git edit" "$CTX" "do not edit files in $FIXTURE_GIT_PARENT"
assert_contains "jj vcs" "$CTX" "do not run VCS commands against $FIXTURE_JJ_PARENT"
assert_contains "git vcs" "$CTX" "do not run VCS commands against $FIXTURE_GIT_PARENT"
assert_contains "jj research" "$CTX" "do not grep, find, or research files in $FIXTURE_JJ_PARENT"
assert_contains "git research" "$CTX" "do not grep, find, or research files in $FIXTURE_GIT_PARENT"

# ── jj secondary nested inside a pure-git parent ─────────────────────────
echo "Test [nesting]: jj-in-git nesting names BOTH parents (jj inner, git outer)"
make_jj_secondary_in_git_parent
OUTPUT=$(run_hook "$FIXTURE_TARGET")
CTX=$(extract_context "$OUTPUT")
# Classification must distinguish nested-jj-in-git from plain jj-secondary
# (the previous design returned jj-secondary and dropped the git parent).
assert_contains "boundary header" "$CTX" "WORKSPACE BOUNDARY DETECTED"
assert_contains "boundary path" "$CTX" "Boundary (active workspace): $FIXTURE_TARGET"
assert_contains "jj parent labelled" "$CTX" "Parent repository (jj): $FIXTURE_JJ_PARENT"
assert_contains "git parent labelled" "$CTX" "Parent repository (git): $FIXTURE_GIT_PARENT"
# BOTH parents must carry the full prohibition triplet.
assert_contains "jj edit" "$CTX" "do not edit files in $FIXTURE_JJ_PARENT"
assert_contains "jj vcs" "$CTX" "do not run VCS commands against $FIXTURE_JJ_PARENT"
assert_contains "jj research" "$CTX" "do not grep, find, or research files in $FIXTURE_JJ_PARENT"
assert_contains "git edit" "$CTX" "do not edit files in $FIXTURE_GIT_PARENT"
assert_contains "git vcs" "$CTX" "do not run VCS commands against $FIXTURE_GIT_PARENT"
assert_contains "git research" "$CTX" "do not grep, find, or research files in $FIXTURE_GIT_PARENT"
# Anchor on the helper outputs the work item names explicitly.
JJ_WS_REAL=$( (cd "$FIXTURE_TARGET" && realpath "$(jj workspace root)"))
GIT_COMMON_REAL=$( (cd "$FIXTURE_TARGET" && realpath "$(dirname "$(git rev-parse --git-common-dir)")"))
assert_eq "inner boundary == jj workspace root" "$JJ_WS_REAL" "$FIXTURE_TARGET"
assert_eq "outer parent == git common-dir parent" "$GIT_COMMON_REAL" "$FIXTURE_GIT_PARENT"

# ── git linked worktree nested inside a pure-jj parent ──────
echo "Test [nesting]: git-in-jj nesting names BOTH parents (git inner, jj outer)"
make_git_worktree_in_jj_parent
OUTPUT=$(run_hook "$FIXTURE_TARGET")
CTX=$(extract_context "$OUTPUT")
assert_contains "boundary header" "$CTX" "WORKSPACE BOUNDARY DETECTED"
assert_contains "boundary path" "$CTX" "Boundary (active workspace): $FIXTURE_TARGET"
assert_contains "git parent labelled" "$CTX" "Parent repository (git): $FIXTURE_GIT_PARENT"
assert_contains "jj parent labelled" "$CTX" "Parent repository (jj): $FIXTURE_JJ_PARENT"
assert_contains "git edit" "$CTX" "do not edit files in $FIXTURE_GIT_PARENT"
assert_contains "git vcs" "$CTX" "do not run VCS commands against $FIXTURE_GIT_PARENT"
assert_contains "git research" "$CTX" "do not grep, find, or research files in $FIXTURE_GIT_PARENT"
assert_contains "jj edit" "$CTX" "do not edit files in $FIXTURE_JJ_PARENT"
assert_contains "jj vcs" "$CTX" "do not run VCS commands against $FIXTURE_JJ_PARENT"
assert_contains "jj research" "$CTX" "do not grep, find, or research files in $FIXTURE_JJ_PARENT"

echo "=== hooks.json registration ==="

# ── hooks/hooks.json SessionStart vcs-detect entry intact ────────────────
# Order-independent (no SessionStart[N] indexing): finds the entry by its
# command string rather than assuming a fixed array position, so reordering
# hooks.json's SessionStart array does not break this guard. A non-matching
# selector resolves to `null`, which every assertion below fails against
# rather than vacuously passing.
echo "Test [hooks.json]: hooks.json SessionStart entry has matcher='', one hook, expected command"
HOOKS_JSON="$PLUGIN_ROOT/hooks/hooks.json"
# shellcheck disable=SC2016 # single-quoted jq expressions; ${CLAUDE_PLUGIN_ROOT} is expanded by Claude Code at runtime, intentionally not shell-expanded
DETECT_SELECTOR='[.hooks.SessionStart[] | select(.hooks[0].command == "${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs detect --format=hook --fail-safe --descriptive")][0]'
assert_json_eq "matcher empty" \
  "$DETECT_SELECTOR.matcher" "" "$HOOKS_JSON"
assert_json_eq "one hook entry" \
  "$DETECT_SELECTOR.hooks | length" "1" "$HOOKS_JSON"
assert_json_eq "type command" \
  "$DETECT_SELECTOR.hooks[0].type" "command" "$HOOKS_JSON"

echo ""
test_summary
