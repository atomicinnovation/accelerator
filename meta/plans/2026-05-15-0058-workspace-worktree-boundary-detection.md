---
date: "2026-05-15T15:30:00+01:00"
type: plan
skill: create-plan
work-item: "0058"
status: accepted
---

# Workspace and Worktree Boundary Detection Implementation Plan

## Overview

Extend the existing `SessionStart` VCS-detection hook so that, when a session
starts inside a jj secondary workspace or a git linked worktree, the model
receives an `additionalContext` block that pins the workspace/worktree
boundary path, names the parent repository path, and prohibits edits, VCS
commands, and grep/find research outside the boundary. The change is scoped
to context-injection — no PreToolUse enforcement, no schema change to the
hook's existing JSON envelope.

The work follows strict test-driven development: every production change in
`scripts/vcs-common.sh` and `hooks/vcs-detect.sh` lands behind a red test
written in the same phase. Phase 1 sets up the test harness and *also*
captures byte-identity golden snapshots of the current hook output against
the unchanged code, so AC5 functions as a regression guard for every
subsequent phase.

## Current State Analysis

- `hooks/vcs-detect.sh` (83 lines) is a `SessionStart` hook that emits a
  single `{"hookSpecificOutput":{"hookEventName":"SessionStart",
  "additionalContext": <string>}}` envelope. It branches on three modes
  (`jj`, `jj-colocated`, `git`) computed from `.jj` / `.git`
  *directory* presence at the repo root. No notion of secondary workspace
  or linked worktree exists today.
- `scripts/vcs-common.sh` (19 lines) exposes only `find_repo_root` — a
  directory-walk that tests `-d $dir/.jj` or `-d $dir/.git`. Because the
  test is directory-only:
  - A git linked worktree (where `.git` is a *file* containing
    `gitdir: …/worktrees/<name>`) is silently skipped; the walk continues
    into the parent repo.
  - A jj secondary workspace (where `.jj` is a directory whose `repo` entry
    is a *file*) is found correctly but indistinguishable from a main
    workspace.
  - Bare git repos and submodules are not handled — neither has a main
    worktree at the gitdir parent. New helpers must reject bare repos
    (`--is-bare-repository`) and defer to
    `--show-superproject-working-tree` for submodules.
- `hooks/hooks.json:3-12` registers `vcs-detect.sh` against `SessionStart`
  with empty matcher. AC8 mandates this entry remain byte-identical.
- `hooks/test-migrate-discoverability.sh` is the only existing test file
  under `hooks/`, but **no `run_shell_suites` task points at the `hooks/`
  subtree**, so `hooks/` tests do not currently run in CI. A new
  `test:integration:hooks` task is required.
- `mise.toml [tools]` declares `uv`, `python`, `gh`, `rust`, `node`,
  `shellcheck`. **`jj` is not present.** CI runs on `ubuntu-latest` via
  `jdx/mise-action@v4` (`.github/workflows/main.yml:14-31`) — without `jj`
  in `[tools]`, fixture builders that invoke `jj init` / `jj workspace
  add` cannot run.
- The pre-existing review at
  `meta/reviews/work/0058-workspace-worktree-boundary-detection-review-1.md`
  carries two binding majors into implementation:
  - AC1 prohibition phrases must appear as canonical, contiguous
    substrings — not as a bag of keywords.
  - AC5 golden snapshots must be captured before any change to
    `vcs-detect.sh` or `vcs-common.sh`.

## Desired End State

After this plan is complete:

- `scripts/vcs-common.sh` exposes four new sourcable helpers:
  - `_jj_workspace_is_secondary <workspace_root>` — internal predicate
    isolating the jj-internal `.jj/repo` file-vs-directory marker
    (the single migration point for jj-vcs/jj#8758).
  - `find_jj_main_workspace_root <dir>` — realpath of jj main workspace.
  - `find_git_main_worktree_root <dir>` — realpath of git main worktree,
    with bare-repo / submodule / `GIT_DIR` handling.
  - `classify_checkout <dir>` — prints a structured `KEY=VALUE` record
    (`KIND`, `BOUNDARY`, `JJ_PARENT`, `GIT_PARENT`) on stdout, exits 0
    on every call. `KIND` ranges over `main`, `jj-secondary`,
    `git-worktree`, `colocated`, `nested-jj-in-git`, `nested-git-in-jj`,
    `none`. The hook parses this record once instead of re-running
    probes per case arm.
  `find_repo_root` is untouched (locked in by a regression test).
- `hooks/vcs-detect.sh` emits its existing VCS-mode `additionalContext`
  unchanged when inside a main checkout (byte-identical to the pre-change
  golden snapshots) and **appends** a boundary block when inside a jj
  secondary workspace, a git linked worktree, both (colocated), or
  either direction of cross-VCS nesting.
- The boundary block contains, with all paths `realpath`-normalised
  (`REPO_ROOT` is also locally normalised inside the hook):
  - The active workspace/worktree absolute path emitted exactly once
    as `Boundary (active workspace): <path>`.
  - For each non-empty parent: a `Parent repository (<jj|git>): <path>`
    line followed by the three AC1 prohibition phrases — `do not edit
    files in <parent>`, `do not run VCS commands against <parent>`,
    `do not grep, find, or research files in <parent>` — produced by a
    single `_emit_parent_block` helper (single source of truth for the
    prohibition wording).
- When a VCS binary is missing inside a non-main, non-none checkout,
  the hook emits a `systemMessage` diagnostic so the user is not left
  guessing why the boundary block is partial (mirrors the existing
  `jq`-missing systemMessage pattern).
- `hooks/test-vcs-detect.sh` exercises every AC against real `jj git
  init`, `jj workspace add`, `git init`, `git init --bare`, and
  `git worktree add` fixtures (including the symmetric jj-in-git and
  git-in-jj nesting directions); locks in `find_repo_root`'s unchanged
  behaviour for well-defined fixtures; runs in CI through a new
  `test:integration:hooks` task; and is the authoritative regression
  guard for the hook output.
- Golden snapshots live at
  `hooks/test-fixtures/vcs-detect/{main-jj-workspace,main-git-checkout}.json`,
  alongside a committed `regenerate.sh` (the only sanctioned way to
  rebuild them) and a `CAPTURE-SOURCE.txt` recording the source commit
  hashes of the production files they were captured against.
- `hooks/hooks.json` is byte-identical to its pre-change content (AC8,
  asserted via three structural `jq` checks).
- `hooks/vcs-detect.sh` carries a leading-comment block (AC9) whose
  canonical phrases (`alternative considered: split into a sibling
  workspace-detect.sh`, `shared REPO_ROOT and VCS_MODE computation`,
  `one coherent SessionStart message`, `meta/work/0058`) the AC9 test
  asserts as contiguous substrings — not loose keywords.

Verification: `mise run test` passes end-to-end on a clean clone, and the
new `hooks/test-vcs-detect.sh` covers all nine acceptance criteria.

### Key Discoveries

- **AC5 capture order is binding** — review pass-2 major #19 mandates
  capturing the main-checkout snapshots *before* changing
  `vcs-detect.sh`/`vcs-common.sh`. Phase 1 is therefore production-code
  read-only; only test infrastructure and snapshots land. Capture is
  scripted (`regenerate.sh`) with `TMPDIR=/tmp` and
  `GIT_CEILING_DIRECTORIES` pinned, run on Linux as the canonical host,
  and source-commit-hashed in `CAPTURE-SOURCE.txt` so a future re-run
  cannot accidentally rebaseline from changed production code.
- **Authoritative probes, not path-walking** — work-item Requirements §4
  prohibits path-walking. Detection uses `jj workspace root` plus the
  `.jj/repo` file-vs-directory test for jj, and `git rev-parse
  --git-dir` vs `--git-common-dir` for git.
- **`realpath` is a global rule** — every emitted path field and every
  `<parent>` substitution inside the prohibitions must be
  `realpath`-normalised so macOS `/private/var` vs `/var` compares
  equal. To keep the rule visible at the emission site, `REPO_ROOT` is
  also locally `realpath`-normalised inside `vcs-detect.sh` (without
  touching `find_repo_root` itself).
- **Function-naming convention** is unprefixed snake_case
  (`find_repo_root`, not `vcs_find_repo_root`) — confirmed in
  `meta/reviews/plans/2026-04-29-jira-integration-phase-1-foundation-review-1.md:608`.
- **`*-common.sh` libraries inherit caller shell options** — no `set
  -euo pipefail` inside `vcs-common.sh` (`scripts/config-common.sh:5`
  documents this convention explicitly).
- **`vcs-detect.sh` has no `set` flags either** — extension code must
  match this convention or carefully introduce stricter modes without
  breaking existing fall-through paths.
- **Test harnesses do use `set -euo pipefail`** —
  `hooks/test-migrate-discoverability.sh:2` establishes the precedent
  for test files, separate from the inherit-caller-options rule for
  sourced libraries.
- **Missing-binary degradation** — the existing hook already exits 0
  with a `systemMessage` warning if `jq` is missing
  (`hooks/vcs-detect.sh:4-7`); new code adopts the same pattern for
  missing `jj` and `git` inside non-main checkouts.
- **jj `.jj/repo` marker is officially internal** — jj-vcs/jj#8758 is
  the open upstream tracker for `jj workspace repo-root`. The
  `_jj_workspace_is_secondary` helper is the single place the marker
  is interpreted, so the eventual upstream migration is a one-line
  change.
- **Bare-repo and submodule edge cases** —
  `find_git_main_worktree_root` rejects bare repos
  (`--is-bare-repository`) and defers to
  `--show-superproject-working-tree` for submodules, plus scrubs an
  externally-set `GIT_DIR` so ambient environment cannot poison
  probe results.
- **Structured-record output** — `classify_checkout` prints four
  `KEY=VALUE` lines (`KIND`, `BOUNDARY`, `JJ_PARENT`, `GIT_PARENT`)
  rather than a single enum string. The hook parses this once;
  emission becomes pure formatting, and cross-VCS nesting can be
  emitted with both parents without re-running probes.

## What We're NOT Doing

- **No PreToolUse enforcement.** This plan does not modify
  `hooks/vcs-guard.sh` or add a sibling boundary-enforcement hook. The
  work item is explicit: context-injection only.
- **No refactor of `find_repo_root`.** Its directory-only `-d` test is
  the underlying gap that motivates the new helpers, but its callers
  (visualiser launchers, jira scripts, etc.) are out of scope. The new
  helpers sit alongside it.
- **No JSON-envelope schema change.** The new boundary content is folded
  into the existing `additionalContext` *string* as appended prose
  lines; no new sibling fields next to `additionalContext`. (Decided
  to preserve AC5 byte-identity on the main-checkout snapshots.)
- **No ADR.** Work item 0020 remains the open ADR-creation task for
  VCS-detection design. The placement rationale lives only in the AC9
  top-of-file comment.
- **No changes to `find_repo_root` callers.** Realpath normalisation
  is local to the new helpers and the new boundary fields; existing
  callers continue to receive un-normalised paths.
- **No new ticket numbering or work-item splits.** Treated as a single
  story per the work item's structure.

## Implementation Approach

Five phases, sized so each delivers a coherent merge-able slice and each
phase's tests cover only what that phase implements:

1. **Phase 1 — Test infrastructure + regression guards.** No production
   code changes. Wires `hooks/` into CI; adds `jj` to `mise.toml` (with
   pre-merge `mise ls-remote` and `--quiet` flag verification); creates
   the test scaffold with bash / `jj` / `git` / `realpath` / `jq`
   preflight; builds fixture builders with named-globals contract
   (including a colocated builder in the correct order: `git worktree
   add` first, then `jj workspace add`), a bare-repo fixture, and
   `make_*_in_*_parent` fixtures for cross-VCS nesting; captures
   golden snapshots via a committed `regenerate.sh` script with
   `TMPDIR` and `GIT_CEILING_DIRECTORIES` pinned and a sidecar
   `CAPTURE-SOURCE.txt`; lands AC5 + AC6 tests (with host-path-artefact
   guard, stderr-empty, JSON-validity assertions). Tests pass against
   unchanged code.
2. **Phase 2 — Detection helpers (AC7).** Add four helpers to
   `scripts/vcs-common.sh`: `_jj_workspace_is_secondary` isolation
   function; `find_jj_main_workspace_root` (with defensive post-resolve
   invariant); `find_git_main_worktree_root` (with bare-repo /
   submodule / `GIT_DIR` handling and `command -v git` guard);
   `classify_checkout` (returns a structured `KEY=VALUE` record with
   seven `KIND` values). All four behind red tests. Lock in
   `find_repo_root`'s unchanged behaviour as a regression guard.
   `find_repo_root` is otherwise untouched.
3. **Phase 3 — Single-VCS secondary cases (AC1, AC2).** Wire
   `vcs-detect.sh` to call `classify_checkout` once, parse the
   structured record, factor `_emit_parent_block` for single-source-of-
   truth prohibition wording, locally `realpath`-normalise `REPO_ROOT`,
   handle the trailing-newline strip of `$()` explicitly, and emit a
   missing-binary `systemMessage` diagnostic. Canonical AC1
   prohibition strings asserted as full contiguous substrings.
4. **Phase 4 — Colocated and cross-VCS nesting (AC3, AC4).** Three new
   case arms (`colocated`, `nested-jj-in-git`, `nested-git-in-jj`),
   each reusing `build_boundary_block`. Symmetric `make_jj_secondary_in_git_parent`
   and `make_git_worktree_in_jj_parent` fixtures cover both nesting
   directions; AC4 test asserts BOTH parents appear in CTX with full
   prohibition phrasing (closing the major behavioural gap in the
   pre-restructure design).
5. **Phase 5 — Placement comment, AC8 structural assertions,
   missing-binary regression test, final polish.** AC9 leading-comment
   block with canonical contiguous phrases (asserted via `awk`-scanned
   leading-block, not `head -n N`); AC8 enforced via three structural
   `jq` checks; regression test masks `jj` / `jq` from PATH and asserts
   graceful exit with the systemMessage diagnostic.

---

## Phase 1: Test Infrastructure and Regression Guards

### Overview

Make `hooks/` testable in CI, add `jj` to the toolchain, build the
fixture infrastructure for every acceptance-criterion case, and capture
the AC5 golden snapshots against unchanged production code. Lands AC5
+ AC6 tests that pass on current code and guard subsequent phases.

### Changes Required

#### 1. Add `jj` to the mise toolchain

**File**: `mise.toml`
**Changes**: Add `jj` to the `[tools]` block alongside the existing
declarations.

```toml
[tools]
uv = "0.11.6"
python = "3.14.4"
gh = "2.89.0"
rust = "1.90.0"
node = "22"
shellcheck = "0.11.0"
jj = "0.36.0"
```

**Verification before merge** (binding, not optional):

1. `mise ls-remote jj | grep -F 0.36.0` — confirm 0.36.0 is offered by
   the `core:jj` mise plugin. If not, pick the highest available stable
   patch on the 0.36.x line (or the highest mise-supported jj version)
   and update this pin.
2. After `mise install`, run `jj git init --help | grep -- '--quiet'`
   and `jj workspace add --help | grep -- '--quiet'` to confirm both
   subcommands accept the `--quiet` flag the fixture builders use.
3. The CI test job runs on `ubuntu-latest` only
   (`.github/workflows/main.yml:14-31`); macOS-specific defects
   (BSD `mktemp`/`realpath` quirks, `/private/var` TMPDIR symlinks)
   will not be caught by CI. This is an accepted gap for this work
   item — local macOS contributors are expected to surface any
   divergence during manual verification.

#### 2. Wire `hooks/` into the integration-test runner

**File**: `tasks/test/integration.py`
**Changes**: Add a `hooks` task mirroring the existing `config` task.

```python
@task
def hooks(context: Context):
    """Integration tests for the hooks/ subtree."""
    run_shell_suites(context, "hooks")
```

**File**: `mise.toml`
**Changes**: Add a `test:integration:hooks` task block and append it to
`test:integration.depends`.

```toml
[tasks."test:integration:hooks"]
description = "Run hooks integration tests"
run = "invoke test.integration.hooks"
```

Append `"test:integration:hooks"` to the existing
`[tasks."test:integration"].depends` list
(`mise.toml:114-122`).

#### 3. Create fixture builders

**File**: `hooks/test-vcs-detect.sh` (new)
**Changes**: New file. Mirrors the bootstrap shape of
`hooks/test-migrate-discoverability.sh:1-24`. Sourced fixture helpers
that build *real* jj/git state. Multi-value fixture builders set named
globals (the conventional bash idiom for multiple returns) rather
than pipe-encoding into stdout, so call sites are self-documenting.

```bash
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

# Tests follow…
```

Call sites that previously used pipe-encoded multi-value returns
(`IFS='|' read -r parent secondary < <(make_jj_secondary_workspace)`)
become straightforward `make_jj_secondary_workspace; ...$FIXTURE_PARENT...`
under the new named-globals contract. Phase 2/3/4 tests follow that
pattern.

#### 4. Capture golden snapshots (AC5 prerequisite)

**Files to create**:
- `hooks/test-fixtures/vcs-detect/main-jj-workspace.json` (new)
- `hooks/test-fixtures/vcs-detect/main-git-checkout.json` (new)
- `hooks/test-fixtures/vcs-detect/regenerate.sh` (new) — checked-in
  regeneration script, single source of truth for how snapshots are
  produced.
- `hooks/test-fixtures/vcs-detect/CAPTURE-SOURCE.txt` (new) — records
  the source commit hash of `hooks/vcs-detect.sh` and
  `scripts/vcs-common.sh` the snapshots were captured against, so a
  future contributor can verify the snapshots are still valid.

**`regenerate.sh`** — capture script (committed and re-runnable):

```bash
#!/usr/bin/env bash
set -euo pipefail

# Regenerate AC5 golden snapshots for vcs-detect.sh.
#
# Pre-conditions:
#   - jj, git, jq, realpath on PATH (via `mise install` from repo root)
#   - hooks/vcs-detect.sh and scripts/vcs-common.sh are in the pre-0058
#     state (verified against CAPTURE-SOURCE.txt by the AC5 test).
#
# Determinism guarantees:
#   - TMPDIR is explicitly /tmp (or realpath-resolved). macOS
#     /var/folders -> /private/var symlinks are normalised so the
#     resulting JSON does not embed host-specific path artefacts.
#   - GIT_CEILING_DIRECTORIES scopes git's upward discovery to the
#     temp dir, preventing accidental walks into the accelerator's
#     own .git or any ancestor repo.
#   - Fixture builders match the test-suite fixture builders exactly
#     (same `jj git init` invocation, same `git init -q + commit
#     --allow-empty` shape) so capture and replay produce identical
#     stdout.
#
# Linux is the canonical capture host. Snapshots captured on macOS
# may diverge in path normalisation; CI runs Linux only.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
HOOK="$PLUGIN_ROOT/hooks/vcs-detect.sh"

TMPDIR=/tmp
WORK=$(mktemp -d "$TMPDIR/vcs-detect-capture-XXXXXX")
export GIT_CEILING_DIRECTORIES="$WORK"
trap 'rm -rf "$WORK"' EXIT

# Main jj workspace.
WORKDIR="$WORK/main-jj" && mkdir -p "$WORKDIR" && (cd "$WORKDIR" && jj git init --quiet)
(cd "$WORKDIR" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$HOOK") \
  > "$SCRIPT_DIR/main-jj-workspace.json"

# Main git checkout (with one empty commit, matching the test fixture).
WORKDIR="$WORK/main-git" && mkdir -p "$WORKDIR"
(cd "$WORKDIR" && git init -q && git config user.email t@e.x && git config user.name T)
(cd "$WORKDIR" && git commit --allow-empty -q -m init)
(cd "$WORKDIR" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$HOOK") \
  > "$SCRIPT_DIR/main-git-checkout.json"

# Record source provenance so the AC5 test can verify the snapshots
# match the production code state they were captured against.
{
  printf 'hooks/vcs-detect.sh: '
  (cd "$PLUGIN_ROOT" && git log -n1 --format=%H hooks/vcs-detect.sh 2>/dev/null \
    || jj log -r 'latest(::@ & file("hooks/vcs-detect.sh"))' --no-graph -T 'commit_id' 2>/dev/null \
    || echo UNKNOWN)
  printf '\nscripts/vcs-common.sh: '
  (cd "$PLUGIN_ROOT" && git log -n1 --format=%H scripts/vcs-common.sh 2>/dev/null \
    || jj log -r 'latest(::@ & file("scripts/vcs-common.sh"))' --no-graph -T 'commit_id' 2>/dev/null \
    || echo UNKNOWN)
  printf '\nCaptured: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'Host: %s\n' "$(uname -s)"
} > "$SCRIPT_DIR/CAPTURE-SOURCE.txt"

echo "Captured snapshots into $SCRIPT_DIR"
```

**Procedure** (executed once during Phase 1; result committed):

```bash
# Run from a clean checkout with NO production-code changes.
chmod +x hooks/test-fixtures/vcs-detect/regenerate.sh
hooks/test-fixtures/vcs-detect/regenerate.sh
```

**Determinism guard (committed as part of the AC5 test, see §5)**: a
grep-based assertion that the captured snapshots contain no
host-specific path artefacts (`/private/var`, `/var/folders`,
`/Users/`). If a regeneration on the wrong host accidentally
introduces such substrings, this guard catches it before the
snapshot is used as a regression baseline.

Phases 2–5 rebuild equivalent fixtures dynamically and assert
byte-identity against these checked-in files.

#### 5. Initial test body (AC5 + AC6, passes against unchanged code)

**File**: `hooks/test-vcs-detect.sh`
**Changes**: Append the test cases below the fixture builders.

```bash
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
```

(`assert_eq` and `assert_not_contains` signatures match
`scripts/test-helpers.sh`. The `jq -e .` validity check is a one-shot
inline assertion — `test-helpers.sh` does not currently expose a
dedicated JSON-validity helper.)

### Success Criteria

#### Automated Verification

- [ ] `mise tasks ls` lists `test:integration:hooks`.
- [ ] `mise run test:integration:hooks` discovers and runs
      `hooks/test-vcs-detect.sh`.
- [ ] All four tests in `hooks/test-vcs-detect.sh` pass against
      *unchanged* `vcs-detect.sh` and `vcs-common.sh`:
      `mise run test:integration:hooks`.
- [ ] Full CI suite still green: `mise run test`.
- [ ] Shellcheck clean on the new test file:
      `shellcheck hooks/test-vcs-detect.sh`.
- [ ] Fixture files exist and are non-empty:
      `test -s hooks/test-fixtures/vcs-detect/main-jj-workspace.json &&
       test -s hooks/test-fixtures/vcs-detect/main-git-checkout.json`.
- [ ] `CAPTURE-SOURCE.txt` records the source commit of
      `hooks/vcs-detect.sh` and `scripts/vcs-common.sh` the snapshots
      were captured from.
- [ ] `regenerate.sh` is executable and reproduces byte-identical
      output when re-run against the same source commits.

#### Manual Verification

- [ ] `jj --version` works in a fresh shell with mise applied.
- [ ] Golden snapshots inspected by hand contain only the existing
      VCS-mode strings (no boundary phrasing leaking in).
- [ ] Commit log shows this phase landed before any change to
      `hooks/vcs-detect.sh` or `scripts/vcs-common.sh`.

---

## Phase 2: Detection Helpers in `scripts/vcs-common.sh`

### Overview

Add four sourcable helper functions to `scripts/vcs-common.sh` that
use authoritative VCS probes to identify the checkout kind:

- `_jj_workspace_is_secondary <workspace_root>` — internal predicate
  isolating the jj-internal `.jj/repo` file-vs-directory marker. The
  *only* place that marker is interpreted, so when
  `jj workspace repo-root` lands upstream (jj-vcs/jj#8758) the
  migration is a single-line change inside this function.
- `find_jj_main_workspace_root <dir>` — returns the realpath of the
  jj main workspace, or exits 1.
- `find_git_main_worktree_root <dir>` — returns the realpath of the
  git main worktree, or exits 1. Handles bare repos, submodules, and
  scrubs `GIT_DIR`.
- `classify_checkout <dir>` — prints a structured `KEY=VALUE` record
  on stdout (`KIND`, `BOUNDARY`, `JJ_PARENT`, `GIT_PARENT`) so a
  single call gives the hook everything it needs to format the
  boundary block. Probes are run once, classifier and emitter share
  the result.

Test-driven: helper tests land red, implementation follows, helper
tests turn green. AC5/AC6 from Phase 1 remain green throughout because
the production-code change is additive (new functions only) and
`classify_checkout` returns `KIND=main` for main checkouts — the
Phase 3 case statement, when introduced, will have no `main` arm.

### Changes Required

#### 1. Helper tests

**File**: `hooks/test-vcs-detect.sh` (extend)
**Changes**: Append a new section `=== vcs-common.sh helpers ===` that
sources `vcs-common.sh` and exercises each helper across every fixture
shape from Phase 1 (now including the new bare-repo fixture). Multi-
value fixture builders use the `FIXTURE_*` named-global contract
introduced in Phase 1.

```bash
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
JJ_BIN_DIR=$(dirname "$(command -v jj)")
NEW_PATH=$(printf '%s' "$PATH" | tr ':' '\n' | grep -vF "$JJ_BIN_DIR" | paste -sd: -)
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
GIT_BIN_DIR=$(dirname "$(command -v git)")
NEW_PATH=$(printf '%s' "$PATH" | tr ':' '\n' | grep -vF "$GIT_BIN_DIR" | paste -sd: -)
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
```

#### 2. Helper implementations

**File**: `scripts/vcs-common.sh`
**Changes**: Append the four new functions below `find_repo_root`. No
`set` flags (inherit caller's options per
`scripts/config-common.sh:3-6` convention).

```bash
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
    dir=$(dirname "$dir")
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
#
# KIND semantics:
#   main             — single-VCS primary checkout. No boundary, no parents.
#   jj-secondary     — jj secondary workspace, no surrounding git. BOUNDARY
#                      is the active workspace; JJ_PARENT is the jj main.
#   git-worktree     — git linked worktree, no surrounding jj. BOUNDARY is
#                      the active worktree; GIT_PARENT is the git main.
#   colocated        — same path is BOTH a jj secondary AND a git linked
#                      worktree, with independent parents. BOUNDARY is the
#                      shared active path; both JJ_PARENT and GIT_PARENT
#                      are non-empty.
#   nested-jj-in-git — jj secondary workspace whose target sits inside a
#                      pure-git parent (different VCS scopes). BOUNDARY is
#                      the jj workspace root; JJ_PARENT is the jj main;
#                      GIT_PARENT is the surrounding git main.
#   nested-git-in-jj — git linked worktree whose target sits inside a
#                      pure-jj parent. Symmetric. BOUNDARY is the git
#                      worktree root.
#   none             — neither jj nor git can find a repo at <dir>. Bare
#                      git repos also resolve to KIND=none because they
#                      have no main worktree.
#
# Missing-binary degradation: if `jj` is not on PATH, the jj probe is
# skipped silently and KIND collapses toward the git-only side (and vice
# versa). The hook emits a systemMessage diagnostic for this case so the
# user is not left guessing why the boundary block disappeared. See
# Phase 3 for the diagnostic.
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
```

Note on minimum tool versions (require git >= 2.5 for
`--git-common-dir`, `--show-superproject-working-tree`, and
`git worktree add`; jj at the version pinned in `mise.toml`). The
plan's mise toolchain provisioning satisfies both; document a brief
inline comment above `find_git_main_worktree_root` recording the git
minimum.

### Success Criteria

#### Automated Verification

- [ ] All helper tests pass: `mise run test:integration:hooks`.
- [ ] AC5 and AC6 tests from Phase 1 still pass (additive change only).
- [ ] Full test suite green: `mise run test`.
- [ ] Shellcheck clean on `scripts/vcs-common.sh`:
      `shellcheck scripts/vcs-common.sh`.
- [ ] Each helper invocable from a bare shell. The structured-record
      contract is also verifiable directly:
      `bash -c 'source scripts/vcs-common.sh; classify_checkout .' \
        | grep -E '^(KIND|BOUNDARY|JJ_PARENT|GIT_PARENT)='`
      produces exactly four lines.
- [ ] `find_repo_root` unchanged-behaviour regression tests pass
      (locked into the Phase 2 helper-test block).

#### Manual Verification

- [ ] In a real jj secondary workspace under `workspaces/`,
      `classify_checkout` prints `KIND=jj-secondary` plus the expected
      BOUNDARY and JJ_PARENT values; `find_jj_main_workspace_root`
      returns the accelerator repo root.
- [ ] In a real git linked worktree, `classify_checkout` prints
      `KIND=git-worktree`.
- [ ] In a manually-constructed bare git repo,
      `classify_checkout` prints `KIND=none` and
      `find_git_main_worktree_root` exits 1.
- [ ] `find_repo_root` output is unchanged in every existing caller
      (`grep -rn 'find_repo_root' scripts hooks skills`).

---

## Phase 3: Boundary Block for Single-VCS Secondary Cases (AC1, AC2)

### Overview

Wire `hooks/vcs-detect.sh` to consume the structured-record output of
`classify_checkout` once, factor a single `_emit_parent_block` helper
shared with Phase 4, locally `realpath`-normalise `REPO_ROOT` inside
the hook so all paths in the emitted message share one normalisation
regime, and add a missing-binary `systemMessage` diagnostic so silent
under-detection becomes a loud user-facing warning. AC1 and AC2 land
as the consuming tests.

### Changes Required

#### 1. Tests (red first)

**File**: `hooks/test-vcs-detect.sh` (extend)
**Changes**: Append the AC1 and AC2 test sections. Use named-globals
fixture contract; use `jq <<<` here-string (project convention in
`scripts/test-helpers.sh`) rather than `echo | jq`.

```bash
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
```

#### 2. Hook extension (green)

**File**: `hooks/vcs-detect.sh`
**Changes**: After the existing `case "$VCS_MODE"` block builds
`CONTEXT`, locally normalise `REPO_ROOT`, call `classify_checkout`
once, parse the structured record, factor `_emit_parent_block` for
prohibition wording, and append a boundary block when the kind
demands it. The trailing-newline subtlety of `$(...)` is handled by
explicitly appending `$'\n'` after concatenation, so future contributors
do not need to re-derive the trim behaviour.

```bash
# After the existing case "$VCS_MODE" block, before the jq invocation:

# Locally realpath-normalise REPO_ROOT inside this hook only (we do NOT
# touch find_repo_root, which has ~45 external callers). After this line,
# every path in the emitted additionalContext message shares one
# normalisation regime — REPO_ROOT and the new boundary paths.
REPO_ROOT=$(realpath "$REPO_ROOT" 2>/dev/null || printf '%s' "$REPO_ROOT")

# Single source of truth for AC1 prohibition wording. Used by every kind
# that emits one or more parent blocks.
_emit_parent_block() {
  local label="$1" parent="$2"
  printf 'Parent repository (%s): %s\n' "$label" "$parent"
  printf 'do not edit files in %s\n' "$parent"
  printf 'do not run VCS commands against %s\n' "$parent"
  printf 'do not grep, find, or research files in %s\n' "$parent"
}

# Build the boundary block. The kind_suffix is "" for single-VCS kinds
# and " (colocated)" / " (nested)" for the Phase 4 dual-parent kinds.
build_boundary_block() {
  local kind_suffix="$1" boundary="$2" jj_parent="$3" git_parent="$4"
  printf '\n\nWORKSPACE BOUNDARY DETECTED%s\n' "$kind_suffix"
  printf 'You are inside a checkout that is NOT the main repository.\n'
  printf 'Boundary (active workspace): %s\n' "$boundary"
  printf '\n'
  if [ -n "$jj_parent" ]; then
    _emit_parent_block "jj" "$jj_parent"
    [ -n "$git_parent" ] && printf '\n'
  fi
  if [ -n "$git_parent" ]; then
    _emit_parent_block "git" "$git_parent"
  fi
}

# Probe once, parse once. C_JJ_MISSING / C_GIT_MISSING default to "0"
# so the diagnostic branch never fires for older `classify_checkout`
# implementations that did not emit those fields (forward-compat).
CHECKOUT_RECORD=$(classify_checkout .)
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
done <<< "$CHECKOUT_RECORD"

# Missing-binary diagnostic. Fires based on the classifier's
# JJ_MISSING / GIT_MISSING fields rather than KIND, because a missing
# binary collapses KIND toward `none` or single-VCS — gating on KIND
# would silently fail to diagnose the most common scenario (jj missing
# inside a lone jj secondary workspace, where KIND=none). Mirrors the
# existing jq-missing pattern in this hook.
SYSTEM_MESSAGE=""
if [ "$C_JJ_MISSING" = "1" ]; then
  SYSTEM_MESSAGE="vcs-detect.sh: jj binary not on PATH; jj-side boundary detection was skipped (ancestor .jj marker present)."
elif [ "$C_GIT_MISSING" = "1" ]; then
  SYSTEM_MESSAGE="vcs-detect.sh: git binary not on PATH; git-side boundary detection was skipped (ancestor .git marker present)."
fi

# Single-VCS arms (Phase 3). Phase 4 adds colocated and nested arms
# below; the build_boundary_block helper already handles dual-parent
# emission because it iterates over non-empty JJ_PARENT and GIT_PARENT.
case "$C_KIND" in
  jj-secondary|git-worktree)
    BOUNDARY_OUT=$(build_boundary_block "" "$C_BOUNDARY" "$C_JJ_PARENT" "$C_GIT_PARENT")
    # $() strips trailing newlines; explicitly restore one so future
    # appended content is not run-on with the prohibition lines.
    CONTEXT="${CONTEXT}${BOUNDARY_OUT}"$'\n'
    ;;
esac
```

(`main` and `none` produce no boundary content by design. Phase 4
adds `colocated`, `nested-jj-in-git`, and `nested-git-in-jj` arms;
the `build_boundary_block` helper is already shape-complete for two
parents, so Phase 4 is a small case-statement extension rather than
a new formatter.)

The existing jq invocation that emits the JSON envelope is extended
to add a top-level `systemMessage` field **only when `SYSTEM_MESSAGE`
is non-empty** (omitted entirely otherwise, so main-checkout AC5
byte-identity is unaffected):

```bash
jq -n \
  --arg context "$CONTEXT" \
  --arg sys "$SYSTEM_MESSAGE" \
  '{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: $context}}
   + (if $sys == "" then {} else {systemMessage: $sys} end)'
```

The `+` operator and conditional ensure the `systemMessage` key is
absent (not present-but-empty) in the main-checkout case, preserving
the AC5 golden-snapshot byte-identity contract.

### Success Criteria

#### Automated Verification

- [ ] AC1 and AC2 tests pass: `mise run test:integration:hooks`.
- [ ] AC5 byte-identity tests still pass — `classify_checkout` returns
      `KIND=main` for main checkouts and the case statement has no
      `main` arm, so `CONTEXT` is unchanged before the jq invocation.
      (The local `realpath` of `REPO_ROOT` is a no-op on a canonical
      Linux path; on macOS it normalises but the AC5 fixtures are
      captured on Linux as the canonical form, so byte-identity holds
      when replayed on macOS via realpath-resolved TMPDIR.)
- [ ] AC6 (no boundary phrasing in non-repo) still passes with the
      strengthened assertions from Phase 1 §5.
- [ ] Full suite green: `mise run test`.
- [ ] Shellcheck clean: `shellcheck hooks/vcs-detect.sh`.

#### Manual Verification

- [ ] Starting Claude Code from inside a real `workspaces/<name>` jj
      secondary workspace shows the boundary block in the session
      diagnostics.
- [ ] The boundary block prose is readable and unambiguous; the
      `Parent repository (jj):` and three prohibition lines all use
      the same `realpath`-normalised parent path.
- [ ] Starting Claude Code with `jj` temporarily removed from PATH
      (rename the binary) inside a jj secondary workspace produces a
      `systemMessage` explaining that jj-side detection was skipped.
- [ ] No new content leaks into main-checkout sessions.

---

## Phase 4: Colocated and Cross-VCS Nesting (AC3, AC4)

### Overview

Extend the case statement to handle the three multi-parent kinds:
`colocated` (same path is both jj secondary AND git linked worktree),
`nested-jj-in-git` (jj secondary inside a pure-git parent), and
`nested-git-in-jj` (git linked worktree inside a pure-jj parent).
Because `build_boundary_block` from Phase 3 already iterates over
non-empty `JJ_PARENT` and `GIT_PARENT`, the hook extension is a small
case-statement addition rather than a new formatter. The Phase 3
review identified that cross-VCS nesting silently dropped the outer
parent under the original design; the structured-record refactor and
the new `nested-*` `KIND` values close that gap.

Two cross-VCS fixtures land: jj-in-git AND git-in-jj. The work item
explicitly names jj-in-git (AC4); the symmetric direction is added
because the new `nested-git-in-jj` `KIND` value must be exercised end-
to-end too.

### Changes Required

#### 1. Tests

**File**: `hooks/test-vcs-detect.sh` (extend)
**Changes**:

```bash
echo "=== boundary block: colocated and cross-VCS ==="

# Cross-VCS fixture: a jj secondary workspace whose target sits inside
# a pure-git parent (AC4).
make_jj_secondary_in_git_parent() {
  FIXTURE_JJ_PARENT="" FIXTURE_GIT_PARENT="" FIXTURE_TARGET=""
  FIXTURE_GIT_PARENT=$(make_main_git_checkout)
  FIXTURE_JJ_PARENT=$(make_main_jj_workspace)
  local target="$FIXTURE_GIT_PARENT/sub"
  (cd "$FIXTURE_JJ_PARENT" && jj workspace add --quiet "$target")
  FIXTURE_TARGET=$(realpath "$target")
}

# Symmetric cross-VCS fixture: a git linked worktree whose target sits
# inside a pure-jj parent. Exercises the nested-git-in-jj KIND.
make_git_worktree_in_jj_parent() {
  FIXTURE_JJ_PARENT="" FIXTURE_GIT_PARENT="" FIXTURE_TARGET=""
  FIXTURE_JJ_PARENT=$(make_main_jj_workspace)
  FIXTURE_GIT_PARENT=$(make_main_git_checkout)
  local target="$FIXTURE_JJ_PARENT/sub"
  # git worktree add requires a non-existent target.
  (cd "$FIXTURE_GIT_PARENT" && git worktree add -q "$target")
  FIXTURE_TARGET=$(realpath "$target")
}

# ── AC3: colocated — single block, both parents named separately ──────────────
echo "Test [AC3]: colocated checkout emits single block with both parents"
make_colocated_secondary
OUTPUT=$(run_hook "$FIXTURE_TARGET")
CTX=$(extract_context "$OUTPUT")
# Exactly one boundary line, with the shared target path as its value.
COUNT=$(grep -c "Boundary (active workspace): $FIXTURE_TARGET" <<< "$CTX" || true)
assert_eq "exactly one boundary line" "1" "$COUNT"
assert_contains "jj parent labelled" "$CTX" "Parent repository (jj): $FIXTURE_JJ_PARENT"
assert_contains "git parent labelled" "$CTX" "Parent repository (git): $FIXTURE_GIT_PARENT"
# Both sets of canonical prohibitions present (full phrases, not just keywords).
assert_contains "jj edit"      "$CTX" "do not edit files in $FIXTURE_JJ_PARENT"
assert_contains "git edit"     "$CTX" "do not edit files in $FIXTURE_GIT_PARENT"
assert_contains "jj vcs"       "$CTX" "do not run VCS commands against $FIXTURE_JJ_PARENT"
assert_contains "git vcs"      "$CTX" "do not run VCS commands against $FIXTURE_GIT_PARENT"
assert_contains "jj research"  "$CTX" "do not grep, find, or research files in $FIXTURE_JJ_PARENT"
assert_contains "git research" "$CTX" "do not grep, find, or research files in $FIXTURE_GIT_PARENT"

# ── AC4: jj secondary nested inside a pure-git parent ─────────────────────────
echo "Test [AC4]: jj-in-git nesting names BOTH parents (jj inner, git outer)"
make_jj_secondary_in_git_parent
OUTPUT=$(run_hook "$FIXTURE_TARGET")
CTX=$(extract_context "$OUTPUT")
# Classification must distinguish nested-jj-in-git from plain jj-secondary
# (the previous design returned jj-secondary and dropped the git parent).
assert_contains "boundary header" "$CTX" "WORKSPACE BOUNDARY DETECTED"
assert_contains "boundary path"  "$CTX" "Boundary (active workspace): $FIXTURE_TARGET"
assert_contains "jj parent labelled" "$CTX" "Parent repository (jj): $FIXTURE_JJ_PARENT"
assert_contains "git parent labelled" "$CTX" "Parent repository (git): $FIXTURE_GIT_PARENT"
# BOTH parents must carry the full prohibition triplet.
assert_contains "jj edit"      "$CTX" "do not edit files in $FIXTURE_JJ_PARENT"
assert_contains "jj vcs"       "$CTX" "do not run VCS commands against $FIXTURE_JJ_PARENT"
assert_contains "jj research"  "$CTX" "do not grep, find, or research files in $FIXTURE_JJ_PARENT"
assert_contains "git edit"     "$CTX" "do not edit files in $FIXTURE_GIT_PARENT"
assert_contains "git vcs"      "$CTX" "do not run VCS commands against $FIXTURE_GIT_PARENT"
assert_contains "git research" "$CTX" "do not grep, find, or research files in $FIXTURE_GIT_PARENT"
# Anchor on the helper outputs the work item names explicitly.
JJ_WS_REAL=$( (cd "$FIXTURE_TARGET" && realpath "$(jj workspace root)") )
GIT_COMMON_REAL=$( (cd "$FIXTURE_TARGET" && realpath "$(dirname "$(git rev-parse --git-common-dir)")") )
assert_eq "inner boundary == jj workspace root" "$JJ_WS_REAL" "$FIXTURE_TARGET"
assert_eq "outer parent == git common-dir parent" "$GIT_COMMON_REAL" "$FIXTURE_GIT_PARENT"

# ── AC4 (symmetric): git linked worktree nested inside a pure-jj parent ──────
echo "Test [AC4]: git-in-jj nesting names BOTH parents (git inner, jj outer)"
make_git_worktree_in_jj_parent
OUTPUT=$(run_hook "$FIXTURE_TARGET")
CTX=$(extract_context "$OUTPUT")
assert_contains "boundary header" "$CTX" "WORKSPACE BOUNDARY DETECTED"
assert_contains "boundary path"   "$CTX" "Boundary (active workspace): $FIXTURE_TARGET"
assert_contains "git parent labelled" "$CTX" "Parent repository (git): $FIXTURE_GIT_PARENT"
assert_contains "jj parent labelled"  "$CTX" "Parent repository (jj): $FIXTURE_JJ_PARENT"
assert_contains "git edit"     "$CTX" "do not edit files in $FIXTURE_GIT_PARENT"
assert_contains "git vcs"      "$CTX" "do not run VCS commands against $FIXTURE_GIT_PARENT"
assert_contains "git research" "$CTX" "do not grep, find, or research files in $FIXTURE_GIT_PARENT"
assert_contains "jj edit"      "$CTX" "do not edit files in $FIXTURE_JJ_PARENT"
assert_contains "jj vcs"       "$CTX" "do not run VCS commands against $FIXTURE_JJ_PARENT"
assert_contains "jj research"  "$CTX" "do not grep, find, or research files in $FIXTURE_JJ_PARENT"

# ── classify_checkout coverage for the new nested KIND values ────────────────
echo "Test [AC7]: classify_checkout KIND=nested-jj-in-git"
make_jj_secondary_in_git_parent
parse_classification "$( (cd "$FIXTURE_TARGET" && classify_checkout .) )"
assert_eq "KIND=nested-jj-in-git" "nested-jj-in-git" "$C_KIND"
assert_eq "BOUNDARY=target" "$FIXTURE_TARGET" "$C_BOUNDARY"
assert_eq "JJ_PARENT=jj" "$FIXTURE_JJ_PARENT" "$C_JJ_PARENT"
assert_eq "GIT_PARENT=git" "$FIXTURE_GIT_PARENT" "$C_GIT_PARENT"

echo "Test [AC7]: classify_checkout KIND=nested-git-in-jj"
make_git_worktree_in_jj_parent
parse_classification "$( (cd "$FIXTURE_TARGET" && classify_checkout .) )"
assert_eq "KIND=nested-git-in-jj" "nested-git-in-jj" "$C_KIND"
assert_eq "BOUNDARY=target" "$FIXTURE_TARGET" "$C_BOUNDARY"
assert_eq "JJ_PARENT=jj" "$FIXTURE_JJ_PARENT" "$C_JJ_PARENT"
assert_eq "GIT_PARENT=git" "$FIXTURE_GIT_PARENT" "$C_GIT_PARENT"
```

#### 2. Hook extension

**File**: `hooks/vcs-detect.sh`
**Changes**: Extend the Phase 3 case statement with three new arms.
All three reuse `build_boundary_block` from Phase 3 — no new formatter
is needed because the helper already iterates over non-empty
`JJ_PARENT` and `GIT_PARENT`. The `kind_suffix` distinguishes the
header text per the AC3 / AC4 readability requirement.

```bash
# Replace the Phase 3 single-VCS-only case block with this fuller form.
# build_boundary_block and _emit_parent_block are unchanged from Phase 3.
case "$C_KIND" in
  jj-secondary|git-worktree)
    BOUNDARY_OUT=$(build_boundary_block "" \
      "$C_BOUNDARY" "$C_JJ_PARENT" "$C_GIT_PARENT")
    CONTEXT="${CONTEXT}${BOUNDARY_OUT}"$'\n'
    ;;
  colocated)
    BOUNDARY_OUT=$(build_boundary_block " (colocated)" \
      "$C_BOUNDARY" "$C_JJ_PARENT" "$C_GIT_PARENT")
    CONTEXT="${CONTEXT}${BOUNDARY_OUT}"$'\n'
    ;;
  nested-jj-in-git|nested-git-in-jj)
    BOUNDARY_OUT=$(build_boundary_block " (nested)" \
      "$C_BOUNDARY" "$C_JJ_PARENT" "$C_GIT_PARENT")
    CONTEXT="${CONTEXT}${BOUNDARY_OUT}"$'\n'
    ;;
esac
```

(`main` and `none` produce no boundary content by design.)

### Success Criteria

#### Automated Verification

- [ ] AC3 and AC4 (both directions) tests pass:
      `mise run test:integration:hooks`.
- [ ] `classify_checkout` KIND=nested-jj-in-git and KIND=nested-git-in-jj
      tests pass.
- [ ] All prior tests still pass.
- [ ] Full suite green: `mise run test`.
- [ ] Shellcheck clean: `shellcheck hooks/vcs-detect.sh`.

#### Manual Verification

- [ ] In a hand-constructed colocated fixture, both parent paths appear
      in the emitted block; the shared boundary path appears exactly
      once.
- [ ] In a hand-constructed jj-in-git nesting fixture, both the jj
      parent and the git parent appear in the boundary block with the
      full three-prohibition triplet each. (This is the case that
      silently failed under the pre-restructure design.)
- [ ] In a hand-constructed git-in-jj nesting fixture, same as above
      with parents swapped.

---

## Phase 5: Placement Comment, AC8 Compliance, Missing-Binary Test, and Final Polish

### Overview

Add the AC9 placement-rationale comment to the top of `vcs-detect.sh`
with canonical phrasing (the AC9 test asserts contiguous phrases, not
loose keywords). Replace the brittle AC8 hard-coded JSON string with
three structural `jq` assertions. Add a missing-binary regression test
that runs the hook with `PATH` munged to hide `jj` / `git` / `jq`
respectively, verifying graceful degradation. Run the full suite as
the final acceptance gate.

### Changes Required

#### 1. Tests

**File**: `hooks/test-vcs-detect.sh` (extend)
**Changes**:

```bash
echo "=== AC8 / AC9 / missing-binary final guards ==="

# ── AC8: hooks/hooks.json SessionStart vcs-detect entry intact ────────────────
# Three structural assertions, not a string-equality check. Survives
# harmless reformatting and any future optional-field additions to the
# entry while still enforcing AC8's intent: the hook fires on every
# SessionStart with the same matcher/command.
echo "Test [AC8]: hooks.json SessionStart entry has matcher='', one hook, expected command"
HOOKS_JSON="$PLUGIN_ROOT/hooks/hooks.json"
assert_eq "matcher empty" \
  "" \
  "$(jq -r '.hooks.SessionStart[0].matcher' "$HOOKS_JSON")"
assert_eq "one hook entry" \
  "1" \
  "$(jq '.hooks.SessionStart[0].hooks | length' "$HOOKS_JSON")"
assert_eq "command points at vcs-detect.sh" \
  '${CLAUDE_PLUGIN_ROOT}/hooks/vcs-detect.sh' \
  "$(jq -r '.hooks.SessionStart[0].hooks[0].command' "$HOOKS_JSON")"
assert_eq "type command" \
  "command" \
  "$(jq -r '.hooks.SessionStart[0].hooks[0].type' "$HOOKS_JSON")"

# ── AC9: top-of-file placement comment present (canonical phrases) ────────────
# Scan the leading contiguous comment block (rather than head -n N) so a
# future contributor can grow the file header without breaking AC9.
# Canonical phrases are asserted as full contiguous substrings, not loose
# keywords like "workspace-detect.sh" or "REPO_ROOT" that could appear in
# any unrelated comment.
echo "Test [AC9]: vcs-detect.sh top-of-file comment names alternative + rationale"
# Extract the leading comment block: shebang line, optional blank line,
# then contiguous `#`-prefixed lines. awk emits up to the first non-comment
# non-blank line.
LEADING_BLOCK=$(awk '
  NR==1 && /^#!/ { print; next }
  /^[[:space:]]*$/ { print; next }
  /^[[:space:]]*#/ { print; next }
  { exit }
' "$PLUGIN_ROOT/hooks/vcs-detect.sh")
assert_contains "names alternative considered" \
  "$LEADING_BLOCK" \
  "alternative considered: split into a sibling workspace-detect.sh"
assert_contains "names shared-computation rationale" \
  "$LEADING_BLOCK" \
  "shared REPO_ROOT and VCS_MODE computation"
assert_contains "names coherent-message rationale" \
  "$LEADING_BLOCK" \
  "one coherent SessionStart message"
# Provenance link so the rationale is traceable.
assert_contains "links to work item" \
  "$LEADING_BLOCK" \
  "meta/work/0058"

# ── Missing-binary graceful degradation ──────────────────────────────────────
# When jj is missing from PATH but we're inside a jj secondary workspace,
# the hook must (a) exit 0, (b) emit a systemMessage explaining the skip,
# (c) NOT crash with a partial boundary block.
#
# IMPORTANT bash semantics: `VAR=val cmd` only scopes VAR for a SIMPLE
# command; on assignments-without-command (`PATH=x OUTPUT=$(...)`), both
# tokens are parsed as assignments to the current shell and PATH leaks.
# On compound commands (`PATH=x (subshell)`), bash either rejects or
# parses as assignment-then-subshell, leaking PATH. The correct idiom
# is to put the PATH assignment INSIDE a subshell wrapper or to scope it
# via env-prefix on a function/command, capturing the function output
# with `$(VAR=val funcname ...)` so VAR is local to the substitution's
# subshell only.
echo "Test [missing-binary]: jj absent — hook exits 0 with systemMessage"
make_jj_secondary_workspace
ORIG_PATH=$PATH
JJ_DIR=$(dirname "$(command -v jj)")
NEW_PATH=$(printf '%s' "$PATH" | tr ':' '\n' | grep -vF "$JJ_DIR" | paste -sd: -)
# Capture hook output and exit code with PATH scoped to the substitution's
# subshell. The `PATH="$NEW_PATH" run_hook ...` env-prefix applies to the
# function call; bash inherits it into the function's `( bash "$HOOK" )`
# subshell; the outer shell's PATH is untouched.
RC=0
OUTPUT=$(PATH="$NEW_PATH" run_hook "$FIXTURE_SECONDARY") || RC=$?
assert_eq "PATH not leaked" "$ORIG_PATH" "$PATH"
assert_eq "exits 0 with jj missing" "0" "$RC"
SYS_MSG=$(jq -r '.systemMessage // ""' <<< "$OUTPUT")
assert_contains "systemMessage names jj" "$SYS_MSG" "jj binary not on PATH"
# Stdout must be valid JSON regardless.
jq -e . >/dev/null <<< "$OUTPUT" \
  || { echo "FAIL: hook stdout not valid JSON with jj missing" >&2; exit 1; }
# Defence-in-depth: the boundary block must not name the jj parent in
# the degraded case (jj-side detection was skipped, so the field has no
# trustworthy value).
CTX=$(jq -r '.hookSpecificOutput.additionalContext // ""' <<< "$OUTPUT")
assert_not_contains "no jj-parent line with jj absent" "$CTX" "Parent repository (jj):"

# Same drill for jq missing — the EXISTING jq-missing path is unchanged
# (hook exits 0 with systemMessage warning). Regression-test it. Use a
# subshell wrapper for PATH scoping (env-prefix does not work on `(...)`).
echo "Test [missing-binary]: jq absent — hook exits 0 with systemMessage (existing behaviour)"
JQ_DIR=$(dirname "$(command -v jq)")
NEW_PATH=$(printf '%s' "$PATH" | tr ':' '\n' | grep -vF "$JQ_DIR" | paste -sd: -)
RC=0
( PATH="$NEW_PATH"; cd "$FIXTURE_SECONDARY" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$HOOK" ) >/dev/null 2>&1 || RC=$?
assert_eq "PATH not leaked" "$ORIG_PATH" "$PATH"
assert_eq "exits 0 with jq missing" "0" "$RC"
```

#### 2. Top-of-file comment

**File**: `hooks/vcs-detect.sh`
**Changes**: Insert the placement-rationale comment immediately after
the shebang. The block is intentionally written so the AC9 test's
canonical phrases (`alternative considered: split into a sibling
workspace-detect.sh`, `shared REPO_ROOT and VCS_MODE computation`,
`one coherent SessionStart message`, `meta/work/0058`) appear as
contiguous substrings — they are not loose keywords. The Desired End
State described AC9 as a "two-line" comment, but the canonical
phrasing requires more space; treat AC9 as "a contiguous leading
comment block of any length" rather than line-count-bound.

```bash
#!/usr/bin/env bash

# Placement decision (AC9, meta/work/0058):
# VCS-mode detection and workspace/worktree boundary detection live in
# this single hook. The alternative considered: split into a sibling
# workspace-detect.sh. We extend rather than split because (a) one
# coherent SessionStart message reads better than two stacked
# additionalContext blocks; (b) shared REPO_ROOT and VCS_MODE
# computation avoids redundant probes.
```

### Success Criteria

#### Automated Verification

- [ ] AC8 (structural), AC9 (canonical phrasing), and the missing-binary
      tests all pass: `mise run test:integration:hooks`.
- [ ] All previous tests still pass.
- [ ] Full CI suite green: `mise run test`.
- [ ] `jj diff --name-only -r <merge-base>..@` (or
      `git diff --name-only main...HEAD`) omits `hooks/hooks.json`.

#### Manual Verification

- [ ] Reading `hooks/vcs-detect.sh` cold, a new contributor can tell
      from the top-of-file comment why boundary detection is in this
      hook rather than a sibling.
- [ ] A full smoke-test session inside the accelerator's own
      `workspaces/` directory shows the boundary block on session
      start and the model refuses to write to the parent repo when
      prompted to do so.
- [ ] Rename `jj` temporarily out of `PATH`, start a session inside a
      jj secondary workspace, and confirm the systemMessage appears
      and the session is otherwise functional.

---

## Testing Strategy

### Unit Tests

The hook is small enough that the unit/integration distinction is
blurred. Everything lands as integration tests under
`hooks/test-vcs-detect.sh` because the meaningful exercise is the
end-to-end hook invocation against real jj/git state.

Per-phase coverage:

- Phase 1 — AC5 (golden snapshots + host-path-artefact guard +
  defence-in-depth absence of boundary markers), AC6 (exit code +
  empty stderr + JSON validity + absence of all three prohibition
  phrases).
- Phase 2 — AC7: `_jj_workspace_is_secondary` success and failure;
  `find_jj_main_workspace_root` and `find_git_main_worktree_root`
  success and failure paths (including the bare-repo failure for
  git); `classify_checkout` `KIND` coverage for `main`, `jj-secondary`,
  `git-worktree`, `colocated`, `none`, plus the structured-record
  field assertions; `find_repo_root` unchanged-behaviour regression
  guard.
- Phase 3 — AC1, AC2. Canonical prohibition phrases asserted as full
  contiguous substrings.
- Phase 4 — AC3 (single coherent colocated block with the boundary
  line appearing exactly once); AC4 in BOTH directions (jj-in-git and
  git-in-jj) with both parents named; `classify_checkout` `KIND`
  coverage for the two nested values.
- Phase 5 — AC8 (three structural `jq` assertions on `hooks.json`);
  AC9 (canonical contiguous phrases via `awk`-scanned leading-comment
  block, not line-count-bound); missing-binary regression test
  (PATH-munged graceful exit with systemMessage).

### Integration Tests

- `hooks/test-vcs-detect.sh` runs through `mise run
  test:integration:hooks` (new task wired in Phase 1).
- `mise run test` is the umbrella suite and must remain green at every
  phase boundary.

### Manual Testing Steps

1. From the accelerator main checkout: start a session; confirm no
   boundary block in the SessionStart context.
2. From a real `workspaces/<name>` jj secondary workspace under the
   accelerator repo: start a session; confirm boundary block with the
   workspace path, the accelerator repo path, and the three AC1
   prohibitions.
3. Construct a hand-built git linked worktree somewhere; start a
   session inside it; confirm the AC2 boundary block.
4. Ask the model to "list files in the parent repo" or "edit
   <parent-path>/README.md" — confirm it declines and cites the
   boundary.
5. Re-run the AC5 main-checkout sessions and visually diff against the
   pre-Phase-1 golden snapshots; they should match byte-for-byte.

## Performance Considerations

- `classify_checkout` shells out to `jj workspace root` and `git
  rev-parse` on every session start. Each call is single-digit
  milliseconds; the existing hook already invokes `find_repo_root`
  per session, so total session-start overhead remains well under
  100ms in the worst case.
- `realpath` is a libc call and negligible.
- No caching needed; the hook fires once per session.

## Migration Notes

- **Main-checkout behaviour unchanged.** Sessions in main checkouts
  produce byte-identical `additionalContext` to today's output (AC5).
  No behavioural change for users who are not inside secondary
  workspaces or linked worktrees.
- **`find_repo_root` unchanged.** Every existing caller (45+ in
  visualiser launchers, jira scripts, work-item scripts, etc.)
  continues to receive identical results. A regression test in Phase 2
  locks this in.
- **Path normalisation divergence is documented.** New helpers always
  `realpath`-normalise their output; `find_repo_root` does not. Inside
  `vcs-detect.sh` we locally normalise `REPO_ROOT` so the in-hook
  message presents consistent paths. The divergence at other call
  sites is intentional and out of scope.
- **`mise.toml` gains a `jj` toolchain entry.** Existing contributors
  running `mise install` will fetch it on next sync. CI rebuilds the
  mise cache automatically. The Phase 1 pre-merge verification step
  confirms the version is supported by `mise core:jj`.
- **CI test job is Linux-only.** macOS-specific defects in the new
  fixture builders or helpers will not be caught by CI. Local macOS
  contributors are expected to flag any divergence during manual
  verification. Snapshots are captured on Linux as the canonical form.
- **Claude Code 2.1.0+ silent `additionalContext` delivery is
  assumed.** The work item names this as a requirement. On older
  Claude Code versions, the boundary block surfaces as a visible chat
  message at every session start — meaningfully more disruptive than
  the existing VCS-mode block. The plan does not gate emission on a
  Claude Code version probe; the trade-off is a conscious choice
  recorded here for downstream support contexts.
- **Coupling to jj's internal `.jj/repo` marker is acknowledged.**
  `_jj_workspace_is_secondary` is the only place the marker is
  interpreted, and `find_jj_main_workspace_root` validates the
  resolved candidate via a defensive post-resolve invariant. When
  jj-vcs/jj#8758 lands upstream (`jj workspace repo-root`), the
  migration is a one-line change inside `_jj_workspace_is_secondary`.

## References

- Work item: `meta/work/0058-workspace-worktree-boundary-detection.md`
- Research: `meta/research/codebase/2026-05-15-0058-workspace-worktree-boundary-detection.md`
- Review of work item:
  `meta/reviews/work/0058-workspace-worktree-boundary-detection-review-1.md`
- Review of plan (this plan):
  `meta/reviews/plans/2026-05-15-0058-workspace-worktree-boundary-detection-review-1.md`
- Files to change:
  - `hooks/vcs-detect.sh:1-83` (extension)
  - `scripts/vcs-common.sh:1-19` (additive — four new helpers)
  - `mise.toml` (toolchain + integration task)
  - `tasks/test/integration.py:21-30` (new `hooks` task)
- Files to create:
  - `hooks/test-vcs-detect.sh`
  - `hooks/test-fixtures/vcs-detect/main-jj-workspace.json`
  - `hooks/test-fixtures/vcs-detect/main-git-checkout.json`
  - `hooks/test-fixtures/vcs-detect/regenerate.sh`
  - `hooks/test-fixtures/vcs-detect/CAPTURE-SOURCE.txt`
- Files to leave untouched:
  - `hooks/hooks.json` (AC8 byte-identity, asserted via structural
    `jq` checks).
  - All existing callers of `find_repo_root` (see research §`Live
    call sites of find_repo_root`); the function itself is also
    untouched.
- Related work: 0020 (VCS abstraction ADR, still open).
- Test patterns followed:
  - `hooks/test-migrate-discoverability.sh:1-105` (bootstrap shape)
  - `scripts/test-helpers.sh:19-30,195,287-301`
    (`assert_eq`, `assert_exit_code`, `assert_contains`)
  - `scripts/test-config.sh:1935-1939` (golden-text equality pattern)
- Upstream jj tracking issue for `jj workspace repo-root`:
  https://github.com/jj-vcs/jj/issues/8758
- git-worktree(1): https://git-scm.com/docs/git-worktree
- Minimum git version: 2.5 (`--git-common-dir`,
  `--show-superproject-working-tree`, `git worktree add`).
- Claude Code hooks reference: https://code.claude.com/docs/en/hooks
