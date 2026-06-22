---
type: work-item
id: "0124"
title: "Converge legacy lexical VCS detection (find_repo_root / vcs_mode) on the 0058 probe layer"
date: "2026-06-22T14:38:56+00:00"
author: Phil Helm
producer: create-work-item
status: draft
kind: task
priority: medium
relates_to: ["work-item:0123", "work-item:0058", "work-item:0020"]
tags: [tech-debt, scripts, vcs, git, jj, worktree, vcs-common]
last_updated: "2026-06-22T14:38:56+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# 0124: Converge legacy lexical VCS detection on the 0058 probe layer

**Kind**: Task (tech debt)
**Status**: Draft
**Priority**: Medium
**Author**: Phil Helm

## Summary

`scripts/vcs-common.sh` contains **two parallel VCS-detection strategies** that
can give different answers for the same directory, with nothing keeping them in
sync:

1. **Legacy lexical path-walk** — `find_repo_root` and `vcs_mode` infer VCS facts
   by walking up the directory tree and testing for `.git` / `.jj` filesystem
   markers. No VCS binary is invoked. Used by ~26 call sites.
2. **The 0058 authoritative-probe layer** — `classify_checkout`,
   `find_git_main_worktree_root`, `find_jj_main_workspace_root` — which ask
   git / jj directly (`git rev-parse …`, `jj workspace root`) and are correct
   across worktrees, submodules, bare repos, and colocated/nested topologies.

This drift is not theoretical: work item [`0123`](0123-find-repo-root-fails-in-git-worktrees.md)
(the `-d`→`-e` fix) exists *because* 0058 taught the probe layer about git
worktrees but left the lexical helpers' directory-only marker test untouched and
untracked, and the gap resurfaced months later as a production bug (the
visualiser aborting in Conductor workspaces). 0123 resyncs the two strategies
for *today's known topologies*; it does not remove the divergence. The next
uncovered topology — or a future git/jj layout change handled only in the probe
layer — will split them apart again.

This work item is to **converge the two strategies** so there is one reliable
source of truth for repo-root and VCS-mode detection.

## Context

This is the tracked follow-up that 0058 should have produced. It is deliberately
*not* folded into 0123: 0123 is an urgent, one-character, monotonic, zero-new-
dependency bug fix, whereas this convergence is a higher-risk redesign of the
single most widely-sourced detection function and must be done with care.

See the 0123 plan's "Follow-up" section
([`meta/plans/2026-06-22-0123-find-repo-root-fails-in-git-worktrees.md`](../plans/2026-06-22-0123-find-repo-root-fails-in-git-worktrees.md))
for the originating analysis.

## The root cause being addressed

`find_repo_root` / `vcs_mode` detect by **guessing from filesystem markers**.
This heuristic is correct for common cases but structurally wrong for whole
categories of layout. The 0123 `-d`→`-e` change fixes exactly one category
(git linked worktrees, where `.git` is a file). The guessing approach itself
remains, so other categories can still bite:

- **Submodules** — a submodule working dir has a `.git` *file*; the lexical walk
  stops there and returns the *submodule* as the root, whereas the probe layer
  resolves to the *superproject* via `--show-superproject-working-tree`. The two
  strategies give different answers.
- **Bare repos / poisoned `GIT_DIR`** — the probe layer handles "bare repo has no
  worktree" and scrubs an ambient `GIT_DIR`; the lexical helpers are oblivious.
- **`vcs_mode` → `none`** — any topology where markers don't sit where the walk
  expects re-triggers the silent "fail-safe-to-dirty → every work-item file reads
  dirty" degradation that 0123 fixed for worktrees.

## Known constraints (why this is non-trivial)

1. **The lexical code cannot simply be deleted — it becomes a fallback.** These
   helpers are required to work with **no git/jj on `PATH`** (a supported
   scenario — see `_ancestor_has_marker` and the PATH-stripping tests). A
   pure-probe implementation returns nothing in that environment, a new
   regression in the opposite direction. The probe path must fall back to the
   marker walk when the binary is absent.
2. **Performance** — the marker walk is essentially free; probing spawns 1–3
   subprocesses per call. Several of the ~26 callers are SessionStart /
   PreToolUse hooks that fire frequently, so a naive switch adds latency to every
   tool use.
3. **Semantics change → caller audit required** — e.g. the submodule case above
   changes the returned root. All ~26 callers must be audited to confirm they
   want the new semantics before flipping the single source of truth.
4. **`find_repo_root` ≠ `find_git_main_worktree_root`** — `find_repo_root`
   returns the *current* workspace ("where am I"); `find_git_main_worktree_root`
   returns the *canonical main* checkout. These are deliberately different. The
   probe equivalent of `find_repo_root` is roughly `git rev-parse --show-toplevel`
   / `jj workspace root`, **not** the main-root helper. Do not swap one for the
   other.
5. **`vcs_mode` is a command-set selector, not topology** — it must preserve
   `.jj`-WINS for colocated checkouts (jj wins, because git's index lags the jj
   working copy). Any delegation to `classify_checkout` must map the topology
   `KIND=` verdict back to the jj/git selector, not pass it through.

## Proposed approach (sketch — to be refined during planning)

- Introduce a probe-based "current workspace root" accessor (jj-probe first, then
  `git rev-parse --show-toplevel`), with the existing lexical walk as the
  binary-absent fallback.
- Re-express `vcs_mode` as jj-probe-then-git-probe (preserving `.jj`-WINS), again
  with a lexical fallback.
- Audit all ~26 callers for the changed semantics (submodule resolution in
  particular).
- Optionally harden the loud-failure idiom: callers using
  `PROJECT_ROOT="$(find_repo_root)"` under `set -euo pipefail` abort with empty
  stderr on failure. Decide whether to make the failure legible (this is the
  *symptom-mechanism* that made 0123 a baffling blank error).

## Acceptance criteria

- [ ] `find_repo_root` / `vcs_mode` (or their replacements) return correct
      results across: plain checkout, git linked worktree, jj main, jj secondary,
      colocated, submodule, and bare repo — verified by the probe layer's
      authoritative answer.
- [ ] Binary-absent behaviour is preserved (graceful fallback, no regression vs
      today's marker-walk in PATH-stripped environments).
- [ ] `.jj`-WINS ordering preserved for colocated checkouts.
- [ ] All ~26 call sites audited and updated/confirmed for any changed semantics.
- [ ] A cross-strategy characterisation test pins where the strategies agree
      (main checkout) and where they intentionally differ (worktree: workspace
      vs main), so future drift is caught at CI time.
- [ ] Performance impact on hot hook paths assessed and acceptable.
- [ ] `mise run check` and `scripts/lint-bashisms.sh` (bash-3.2 floor) pass.

## Out of scope

- The 0123 `-d`→`-e` fix itself (separately shipped).

## Dependencies

- Relates to / follows: [`0123`](0123-find-repo-root-fails-in-git-worktrees.md)
  (the immediate fix), [`0058`](0058-workspace-worktree-boundary-detection.md)
  (built the probe layer), [`0020`](0020-vcs-abstraction-layer.md) (origin of
  the VCS abstraction).

## References

- Source: [`scripts/vcs-common.sh`](../../scripts/vcs-common.sh)
- Probe layer: `classify_checkout` (`vcs-common.sh:177`),
  `find_git_main_worktree_root` (`vcs-common.sh:127-155`),
  `find_jj_main_workspace_root` (`vcs-common.sh:90-114`)
- Legacy helpers: `find_repo_root` (`vcs-common.sh:8-18`),
  `vcs_mode` (`vcs-common.sh:27-36`)
- Originating analysis: [`meta/plans/2026-06-22-0123-find-repo-root-fails-in-git-worktrees.md`](../plans/2026-06-22-0123-find-repo-root-fails-in-git-worktrees.md) (Follow-up section)
- Research: [`meta/research/codebase/2026-06-22-0123-find-repo-root-fails-in-git-worktrees.md`](../research/codebase/2026-06-22-0123-find-repo-root-fails-in-git-worktrees.md)
