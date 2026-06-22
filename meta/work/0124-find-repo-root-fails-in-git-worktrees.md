---
type: work-item
id: "0124"
title: "find_repo_root fails in git worktrees (-d test on .git)"
date: "2026-06-22T13:47:05+00:00"
author: Phil Helm
producer: create-work-item
status: draft
kind: bug
priority: high
relates_to: ["work-item:0058", "work-item:0020", "work-item:0125"]
tags: [bug, scripts, vcs, git, worktree, conductor]
last_updated: "2026-06-22T14:18:48+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# 0124: find_repo_root fails in git worktrees (-d test on .git)

**Kind**: Bug
**Status**: Draft
**Priority**: High
**Author**: Phil Helm

## Summary

`find_repo_root()` in
[`scripts/vcs-common.sh`](../../scripts/vcs-common.sh#L8-L18) detects a
repository root by testing `[ -d "$dir/.git" ]`. In a **git worktree**, `.git`
is a regular **file** (a `gitdir:` pointer), not a directory, so the test never
matches. The walk-up runs all the way to `/` and the function returns `1`.
Callers running under `set -euo pipefail` then abort on the failed command
substitution — silently, with no stderr. The one-line fix is to test for
existence (`-e`) rather than directory-ness (`-d`), so plain repos, worktrees,
and jj are all handled.

## Context

First observed via `/accelerator:visualise` failing with an empty error message
from inside a Conductor workspace (a git linked worktree) on 2026-06-22.

`find_repo_root` is described in-file as "the single source of truth for
repo-root detection logic" and is sourced by ~30 scripts, so the blast radius is
wide. Affected callers include:

- VCS hooks and helpers: `hooks/vcs-guard.sh`, `hooks/vcs-detect.sh`,
  `scripts/vcs-status.sh`, `scripts/vcs-log.sh`
- Config readers: `scripts/config-common.sh`, `scripts/config-read-path.sh`
- Skills: visualiser (`launch-server.sh`, `stop-server.sh`,
  `status-server.sh`), Linear, Jira, work-items, ADRs, design inventory,
  config init

Conductor workspaces are git worktrees, so **every Conductor-based session is
affected**.

Closely related: work item 0058 (Workspace and Worktree Boundary Detection at
Session Start) identified the exact "`.git` is a regular file in a linked
worktree" signal during its research, yet `find_repo_root` was never updated to
account for it. This bug is that gap.

## Reproduction

From inside a git worktree (e.g. a Conductor workspace such as
`/Users/phelm/conductor/workspaces/accelerator/<name>`):

```
$ ls -la .git
-rw-r--r-- ... .git          # a FILE, not a directory
$ file .git
.git: ASCII text             # "gitdir: /path/to/parent/.git/worktrees/<name>"
```

Run `/accelerator:visualise`. It fails with:

```
Error: Shell command failed for pattern "!`…/visualise/scripts/visualiser.sh ""`":
```

(empty stderr).

Call chain:

1. `visualiser.sh ""` → `exec launch-server.sh`
2. `launch-server.sh` sources `vcs-common.sh` and runs
   `PROJECT_ROOT="$(find_repo_root)"`
3. `find_repo_root` walks up from `$PWD` testing
   `[ -d "$dir/.jj" ] || [ -d "$dir/.git" ]`
4. In a worktree `.git` is a file, so `-d` is false at every level; the loop
   reaches `/` and `return 1`
5. Under `set -euo pipefail`, the failed command substitution aborts
   `launch-server.sh` with exit 1 and no stderr → the blank error surfaced by
   the slash command

## Root cause

The offending line:

```bash
# scripts/vcs-common.sh:11
if [ -d "$dir/.jj" ] || [ -d "$dir/.git" ]; then
```

The sibling helper `_ancestor_has_marker` in the same file already uses `-e`
(exists) rather than `-d`, so this is a local inconsistency, not a deliberate
design choice.

## Expected behaviour

`find_repo_root` returns the worktree path with exit 0 from inside a git linked
worktree, exactly as it does from a plain checkout — so all ~30 downstream
callers (visualiser, hooks, config readers, integration skills) work unchanged
in Conductor workspaces.

## Deliverables

1. **Fix the existence test** in `scripts/vcs-common.sh:11`:

   ```bash
   if [ -e "$dir/.jj" ] || [ -e "$dir/.git" ]; then
   ```

   `-e` is bash-3.2-safe and portable. No other logic change required.

2. **Add a worktree regression test** for `find_repo_root`. The
   `make_git_linked_worktree` fixture already exists in
   `hooks/test-vcs-detect.sh` (it constructs a worktree with a `.git`
   **file**), but it is only exercised against `find_git_main_worktree_root`
   and `classify_checkout` — never against `find_repo_root`. Add a test that
   runs `find_repo_root` from inside that fixture and asserts it returns the
   worktree path with exit 0. Optionally also cover a nested subdirectory
   within the worktree.

3. **Apply the same `-d` → `-e` fix to `vcs_mode`** in
   `scripts/vcs-common.sh` (lines 29, 31). It carries the identical defect:
   in a git worktree it returns `none`, which routes
   `skills/work/scripts/work-item-file-dirty.sh` into its "fail safe to dirty"
   branch so every work-item file reads as dirty. Without this, the acceptance
   criterion "all ~30 downstream callers work unchanged in Conductor
   workspaces" is not met. Add a `vcs_mode` worktree regression test (returns
   `git`) and an end-to-end test of the consumer in a real worktree.

## Out of scope

- Patching the installed plugin-cache copy
  (`~/.claude/plugins/cache/atomic-innovation/accelerator/<version>/`). That
  unblocks a live session but is overwritten on the next plugin update; the
  durable fix is in repo source + a release.
- Any broader refactor of `find_repo_root` / `vcs_mode` beyond the `-d` → `-e`
  change — in particular, delegating them to the 0058 authoritative-probe layer
  (`classify_checkout`, `find_git_main_worktree_root`). That convergence is real
  but separate, and is tracked as
  [`0125`](0125-converge-vcs-detection-on-probe-layer.md).

## Acceptance Criteria

- [ ] `scripts/vcs-common.sh` `find_repo_root` tests `[ -e "$dir/.git" ]`
      (and `.jj`) rather than `[ -d … ]`.
- [ ] Given a shell inside a git linked worktree, when a script sources
      `vcs-common.sh` and runs `find_repo_root`, then it returns the worktree
      root with exit 0 (no abort under `set -euo pipefail`).
- [ ] `/accelerator:visualise` (and `stop`/`status`) start successfully from
      inside a Conductor workspace / git worktree.
- [ ] A new test exercises `find_repo_root` against the
      `make_git_linked_worktree` fixture and asserts the worktree path + exit 0.
- [ ] `scripts/vcs-common.sh` `vcs_mode` tests `[ -e … ]` (lines 29, 31) rather
      than `[ -d … ]`, and returns `git` for a git linked worktree root.
- [ ] A new test asserts `vcs_mode` returns `git` for the worktree fixture, and
      an end-to-end test confirms `work-item-file-dirty.sh` reports a clean
      work-item file as clean (not "always dirty") from inside a worktree.
- [ ] The `.jj`-WINS ordering is preserved (a colocated checkout still resolves
      to `jj`).
- [ ] If the tests are reverted to `-d`, the new tests fail.
- [ ] The change passes the bash-3.2 floor (`scripts/lint-bashisms.sh`) and
      `mise run check`.

## Dependencies

- Blocked by: none
- Blocks: reliable use of any `find_repo_root`-dependent skill/hook from a git
  worktree (Conductor workspaces in particular).

## References

- Source: [`scripts/vcs-common.sh`](../../scripts/vcs-common.sh#L8-L18)
- Test fixture: `hooks/test-vcs-detect.sh` (`make_git_linked_worktree`)
- Related: [`meta/work/0058-workspace-worktree-boundary-detection.md`](0058-workspace-worktree-boundary-detection.md)
- Related: [`meta/work/0020-vcs-abstraction-layer.md`](0020-vcs-abstraction-layer.md)
