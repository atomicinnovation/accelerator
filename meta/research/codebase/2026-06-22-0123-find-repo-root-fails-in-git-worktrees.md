---
type: codebase-research
id: "2026-06-22-0123-find-repo-root-fails-in-git-worktrees"
title: "Research: find_repo_root fails in git worktrees (-d test on .git)"
date: "2026-06-22T14:01:39+00:00"
author: Phil Helm
producer: research-codebase
status: complete
work_item_id: "0123"
parent: "work-item:0123"
relates_to: ["codebase-research:2026-05-15-0058-workspace-worktree-boundary-detection"]
topic: "find_repo_root fails in git worktrees (-d vs -e marker test)"
tags: [research, codebase, scripts, vcs, git, worktree, conductor, vcs-common]
revision: "de38f5413b80f247fa8104ebebca0ed43d9a234b"
repository: "accelerator"
last_updated: "2026-06-22T14:01:39+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# Research: find_repo_root fails in git worktrees (-d test on .git)

**Date**: 2026-06-22T14:01:39+00:00 (UTC)
**Author**: Phil Helm
**Git Commit**: de38f5413b80f247fa8104ebebca0ed43d9a234b
**Branch**: main
**Repository**: accelerator

## Research Question

Work item [`0123`](../../work/0123-find-repo-root-fails-in-git-worktrees.md)
reports that `find_repo_root()` in `scripts/vcs-common.sh` uses a directory
test (`[ -d "$dir/.git" ]`) to detect a repo root, which fails inside a git
linked worktree because `.git` is a regular *file* (a `gitdir:` pointer) there.
This research verifies the live state of the code, maps the full blast radius
of callers, locates the existing test infrastructure for adding a regression
test, and reconciles the bug with the historical record — while checking
whether the proposed one-line fix is actually sufficient.

## Summary

The work item's diagnosis is **correct and confirmed against live source**:
[`find_repo_root`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/scripts/vcs-common.sh#L8-L18)
tests `[ -d "$dir/.jj" ] || [ -d "$dir/.git" ]` at line 11, so in a git linked
worktree (where `.git` is a file) the walk runs to `/` and returns 1. Callers
that use the bare `PROJECT_ROOT="$(find_repo_root)"` form under
`set -euo pipefail` (the visualiser launchers) abort with empty stderr — exactly
the reported symptom. The sibling helper `_ancestor_has_marker` already uses
`-e`, confirming this is a local inconsistency, not a design choice.

Three findings sharpen the work item:

1. **The blast radius is ~26 production call sites across 3 sourcing tiers.**
   Most callers either don't run under `set -e` (the hooks) or guard with
   `|| repo_root=""` (jira/linear), so they degrade rather than abort. The
   visualiser scripts are the ones that hard-abort. The `-d`→`-e` change is
   safe across all of them (it only *adds* matches — a worktree's `.git` file —
   and never removes a previously-matching directory).

2. **There is a latent *second* `-d` bug in the same file that the work item
   scopes out.** [`vcs_mode()`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/scripts/vcs-common.sh#L27-L36)
   (lines 29, 31) uses the identical `-d` test on `.jj`/`.git`. After 0123's
   fix, `find_repo_root` will correctly return the worktree root, but `vcs_mode`
   called on that root returns `none` (because `.git` is a file), which routes
   [`work-item-file-dirty.sh`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/skills/work/scripts/work-item-file-dirty.sh#L101-L104)
   into its "fail safe to dirty" branch — so **every work-item file reads as
   dirty inside a git worktree**. This is a correctness degradation, not a
   crash, but it is the same root defect and is *not* fixed by patching
   `find_repo_root` alone. See [Open Questions](#open-questions).

3. **The fix was anticipated by work item 0058 and the test suite was
   deliberately left open for it.** 0058 (done) added authoritative-probe
   helpers (`classify_checkout`, `find_git_main_worktree_root`) that handle
   worktrees correctly, but explicitly declared "No refactor of
   `find_repo_root`" out of scope. The existing regression block in
   `hooks/test-vcs-detect.sh` (lines 424–446) carries a comment stating it
   deliberately does *not* lock in `find_repo_root`'s worktree behaviour "to
   keep room for a future fix." 0123 is that future fix, and the
   `make_git_linked_worktree` fixture it needs already exists.

## Detailed Findings

### The bug: `find_repo_root` (`scripts/vcs-common.sh:8-18`)

```bash
 8  find_repo_root() {
 9    local dir="$PWD"
10    while [ "$dir" != "/" ]; do
11      if [ -d "$dir/.jj" ] || [ -d "$dir/.git" ]; then
12        echo "$dir"
13        return 0
14      fi
15      dir="$(dirname "$dir")"
16    done
17    return 1
18  }
```

- Seeds from `$PWD` (takes no argument), walks up while `dir != /`, and tests
  `-d` (directory) on the markers at line 11. A git linked worktree's `.git` is
  a regular file, so the test is false at every level; the walk reaches `/`,
  exits the loop, and returns 1.
- The `_ancestor_has_marker` helper in the same file
  ([lines 45-62](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/scripts/vcs-common.sh#L45-L62))
  uses `[ -e "$dir/$marker" ]` (line 49) — existence, not directory-ness —
  confirming the `-d` in `find_repo_root` is an inconsistency.

### The abort mechanism (`set -euo pipefail` + command substitution)

The empty-stderr abort is specific to callers that assign `find_repo_root`'s
output via an unguarded command substitution **and** run under `set -e`:

- [`launch-server.sh:13`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/skills/visualisation/visualise/scripts/launch-server.sh#L13)
  — `PROJECT_ROOT="$(find_repo_root)"`, with `set -euo pipefail` at line 2.
  When the substitution returns 1, `set -e` aborts the script with exit 1 and
  no message → the blank error surfaced by `/accelerator:visualise`.
- [`status-server.sh:13`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/skills/visualisation/visualise/scripts/status-server.sh#L13)
  and [`stop-server.sh:13`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/skills/visualisation/visualise/scripts/stop-server.sh#L13)
  share the pattern.

By contrast, two classes of caller survive a returned 1:

- **Hooks** ([`vcs-detect.sh:22`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/hooks/vcs-detect.sh#L22),
  [`vcs-guard.sh:19`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/hooks/vcs-guard.sh#L19))
  do not set `-euo pipefail` at the top, so the failed substitution yields an
  empty `REPO_ROOT` that the following `[ -z "$REPO_ROOT" ]` guard handles.
- **Jira/Linear flows** use the defensive form
  `repo_root=$(find_repo_root 2>/dev/null) || repo_root=""`
  ([`jira-create-flow.sh:212`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/skills/integrations/jira/scripts/jira-create-flow.sh#L212),
  [`jira-search-flow.sh:251`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/skills/integrations/jira/scripts/jira-search-flow.sh#L251)).

This explains why the visualiser is the loud failure while other worktree
breakage is silent/degraded.

### Blast radius: callers of `find_repo_root`

`find_repo_root` reaches consumers through three sourcing tiers:

- **Tier 1** — files that source `vcs-common.sh` directly.
- **Tier 2** — [`config-common.sh:8`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/scripts/config-common.sh#L8)
  sources it and re-exposes via `config_project_root`
  ([line 18](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/scripts/config-common.sh#L18));
  every `config-*` consumer inherits it.
- **Tier 3** — `jira-common.sh` / `linear-common.sh` each source it and are
  sourced by all their flow scripts.

Production call sites by category:

| Category | Call sites |
|---|---|
| VCS hooks/helpers | `hooks/vcs-detect.sh:22`, `hooks/vcs-guard.sh:19`, `scripts/vcs-status.sh:8`, `scripts/vcs-log.sh:8` |
| Config readers | `scripts/config-common.sh:18`, `scripts/config-read-path.sh:62` |
| Visualiser | `launch-server.sh:13`, `status-server.sh:13`, `stop-server.sh:13` |
| Work items | `work-item-resolve-id.sh:36`, `work-item-file-dirty.sh:76`, `work-item-sync-baseline.sh:56`, `work-item-next-number.sh:57` |
| ADRs/decisions | `adr-next-number.sh:37` |
| Design inventory | `inventory-design/scripts/playwright/run.sh:15` |
| Config init | `config/init/scripts/init.sh:14` |
| Jira (via `jira-common.sh`) | `jira-common.sh:71`, `jira-auth.sh:93,134`, `jira-init-flow.sh:64,165`, `jira-search-flow.sh:251`, `jira-create-flow.sh:212` |
| Linear (via `linear-common.sh`) | `linear-common.sh:70`, `linear-auth.sh:99,169`, `linear-init-flow.sh:67` |

The `-d`→`-e` change is **monotonic**: `-e` matches a superset of `-d` (any
directory that matched `-d` still exists), so no currently-working caller can
regress; the change only *adds* the worktree `.git`-file match. The one
behavioural edge to keep in mind: `-e` would also match a `.git`/`.jj` that is
some other file type (e.g. a stray symlink), but that is not a realistic repo
layout and the authoritative-probe helpers (`classify_checkout`) are unaffected
either way.

### The second `-d` defect: `vcs_mode()` (out of 0123's stated scope)

[`vcs_mode()`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/scripts/vcs-common.sh#L27-L36)
selects the VCS *command set* for a given root:

```bash
27  vcs_mode() {
28    local root="$1"
29    if [ -d "$root/.jj" ]; then
30      printf 'jj\n'
31    elif [ -d "$root/.git" ]; then
32      printf 'git\n'
33    else
34      printf 'none\n'
35    fi
36  }
```

In a git worktree, `$root/.git` is a file, so `vcs_mode` returns `none`. Its
sole production caller,
[`work-item-file-dirty.sh:45`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/skills/work/scripts/work-item-file-dirty.sh#L45),
switches on the result and treats anything other than `jj`/`git` as
indeterminate, returning "dirty" by default
([lines 101-104](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/skills/work/scripts/work-item-file-dirty.sh#L101-L104)):

```bash
101    *)
102      # Indeterminate VCS mode → fail safe to dirty.
103      return 0
104      ;;
```

So even after 0123's fix lands, work-item dirty-detection silently degrades to
"always dirty" inside any git worktree. Applying the *same* `-d`→`-e` reasoning
to `vcs_mode` (lines 29, 31) would fix it: `-e "$root/.jj"` (false) →
`-e "$root/.git"` (true, the pointer file) → returns `git`, which is correct.
The `.jj`-WINS ordering is preserved under `-e` for colocated checkouts.

Note: [`hooks/vcs-detect.sh:28-36`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/hooks/vcs-detect.sh#L28-L36)
*also* uses `-d` on the markers, but its `else` arm defaults to `git`
(line 35), which happens to be correct for a git worktree — so it is benign and
needs no change.

### Test infrastructure for the regression test

- The fixture already exists:
  [`make_git_linked_worktree`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/hooks/test-vcs-detect.sh#L86-L94)
  (lines 86-94) builds a main checkout (with the empty commit `git worktree add`
  requires), allocates a fresh path, removes it, and runs
  `git worktree add` — producing a real worktree whose `.git` is a file. It sets
  `FIXTURE_PARENT` and `FIXTURE_WORKTREE` globals.
- It is currently exercised against `find_git_main_worktree_root` (line 297),
  `classify_checkout` (line 361), and the hook end-to-end (line 469) — but
  **never against `find_repo_root`**.
- The `find_repo_root` regression block lives at
  [lines 424-446](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/hooks/test-vcs-detect.sh#L424-L446)
  and covers only main jj, main git, and jj-secondary. Its closing comment
  (lines 443-446) explicitly documents the deliberate omission of the
  git-worktree case "to keep room for a future fix." `vcs-common.sh` is sourced
  at line 254, so `find_repo_root` is in scope inside this block.
- **Adding the test**: append a stanza to the 424-446 block —
  `make_git_linked_worktree; RESULT=$( (cd "$FIXTURE_WORKTREE" && find_repo_root)); assert_eq "git linked worktree" "$FIXTURE_WORKTREE" "$RESULT"` —
  and update the 443-446 comment to reflect that the behaviour is now asserted.
  The suite is a flat top-to-bottom script (no test framework); assertions come
  from `scripts/test-helpers.sh` (`assert_eq`, etc.), never `exit` on failure,
  and tally PASS/FAIL via `test_summary`. Run with
  `bash hooks/test-vcs-detect.sh` or `mise run test:integration:hooks`. Missing
  tooling (`jj`/`git`/`realpath`/`jq`) exits 77 (skip).
- No other shell test file references `find_repo_root` — `hooks/test-vcs-detect.sh`
  is the sole home for its regression coverage.

## Code References

- [`scripts/vcs-common.sh:8-18`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/scripts/vcs-common.sh#L8-L18) — `find_repo_root`; the `-d` bug is line 11.
- [`scripts/vcs-common.sh:27-36`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/scripts/vcs-common.sh#L27-L36) — `vcs_mode`; identical `-d` defect (lines 29, 31), out of 0123's stated scope.
- [`scripts/vcs-common.sh:45-62`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/scripts/vcs-common.sh#L45-L62) — `_ancestor_has_marker`; already uses `-e` (line 49), the consistency precedent.
- [`scripts/vcs-common.sh:127-155`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/scripts/vcs-common.sh#L127-L155) — `find_git_main_worktree_root`; the 0058 probe-based helper that handles worktrees correctly.
- [`skills/visualisation/visualise/scripts/launch-server.sh:13`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/skills/visualisation/visualise/scripts/launch-server.sh#L13) — the hard-aborting caller (reported symptom).
- [`skills/work/scripts/work-item-file-dirty.sh:45,101-104`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/skills/work/scripts/work-item-file-dirty.sh#L45) — `vcs_mode` consumer that degrades to "always dirty" in a worktree.
- [`hooks/test-vcs-detect.sh:86-94`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/hooks/test-vcs-detect.sh#L86-L94) — `make_git_linked_worktree` fixture.
- [`hooks/test-vcs-detect.sh:424-446`](https://github.com/atomicinnovation/accelerator/blob/de38f5413b80f247fa8104ebebca0ed43d9a234b/hooks/test-vcs-detect.sh#L424-L446) — `find_repo_root` regression block + the "room for a future fix" comment.

## Architecture Insights

- **Two parallel detection strategies coexist in `vcs-common.sh`.** The legacy
  lexical path-walk (`find_repo_root`, `vcs_mode`) tests filesystem markers and
  predates worktree awareness; the 0058 authoritative-probe layer
  (`classify_checkout`, `find_git_main_worktree_root`) asks git/jj directly and
  is worktree-correct. The bug is that the most widely-sourced entry point
  (`find_repo_root`, ~26 call sites) is still on the legacy strategy. The clean
  long-term direction is for `find_repo_root`/`vcs_mode` to delegate to the
  probe layer, but 0123 (rightly) chooses the minimal `-d`→`-e` patch.
- **`-e` vs `-d` is the load-bearing distinction for worktree topology.** Across
  the file the file-vs-directory nature of a marker *is* the signal: `.git` file
  = git worktree; `.jj/repo` file = jj secondary workspace. `find_repo_root`
  only needs *existence* (it returns a path, not a topology), so `-e` is exactly
  right; the helpers that need topology correctly inspect file-vs-dir explicitly.
- **`set -e` + unguarded command substitution is a recurring silent-failure
  trap.** The same returned-1 produces three different outcomes depending on the
  caller's idiom (abort / empty-string-then-guard / `|| fallback`). The durable
  fix is making `find_repo_root` succeed; hardening individual callers is a
  separate, larger concern.
- **The 80-col / bash-3.2 constraints apply.** `-e` is bash-3.2-safe; the change
  must pass `scripts/lint-bashisms.sh` and `mise run check`.

## Historical Context

- [`meta/work/0058-workspace-worktree-boundary-detection.md`](../../work/0058-workspace-worktree-boundary-detection.md)
  (done) — identified the "`.git` is a regular file in a linked worktree" signal
  and built the probe-based helpers, but **explicitly carved `find_repo_root`
  out of scope** ("No refactor of `find_repo_root`. Its directory-only `-d` test
  is the underlying gap that motivates the new helpers, but its callers … are
  out of scope"). 0123 is the deferred follow-up.
- [`meta/research/codebase/2026-05-15-0058-workspace-worktree-boundary-detection.md`](2026-05-15-0058-workspace-worktree-boundary-detection.md)
  — its "Critical gap in current detection" section names the `find_repo_root`
  `-d` failure mode precisely, and Open Question #4 recommended leaving it
  unchanged at the time. The irony: the research framed this as "the failure
  mode 0058 fixes," but the design fixed it only for the new helpers.
- [`meta/plans/2026-05-15-0058-workspace-worktree-boundary-detection.md`](../../plans/2026-05-15-0058-workspace-worktree-boundary-detection.md)
  — its test plan (lines ~910-913) is the origin of the "deliberately do NOT
  lock in `find_repo_root`'s behaviour … keeps room for a future fix" comment
  now living in `hooks/test-vcs-detect.sh`.
- [`meta/work/0020-vcs-abstraction-layer.md`](../../work/0020-vcs-abstraction-layer.md)
  (draft) — origin of the VCS layer and its directory-presence heuristic, but it
  never produced an ADR and imposes **no binding constraint** on `find_repo_root`.
  The `-d`→`-e` change contradicts no ratified decision.
- `meta/reviews/plans/2026-05-08-0030-…-review-1.md:292` — context on
  `find_repo_root`'s blast radius (why `config-read-path.sh` sources
  `vcs-common.sh` directly rather than `config-common.sh`).

## Related Research

- [`2026-05-15-0058-workspace-worktree-boundary-detection.md`](2026-05-15-0058-workspace-worktree-boundary-detection.md)
  — the foundational worktree-detection research; this document is its direct
  follow-up for the carved-out `find_repo_root` gap.

## Open Questions

1. **Should `vcs_mode()` be fixed in the same change?** The work item scopes the
   fix to `find_repo_root:11` only and lists "any broader refactor of … the VCS
   detection helpers beyond the `-d`→`-e` change" as out of scope. But `vcs_mode`
   (lines 29, 31) has the *identical* `-d` defect, and leaving it unpatched means
   `work-item-file-dirty.sh` reports every file as dirty inside a git worktree —
   a real, if quiet, correctness regression for work-item skills used from
   Conductor. The fix is the same one-line `-d`→`-e` transformation and is
   arguably within the spirit (not the letter) of the stated scope.
   **Recommendation: include `vcs_mode` in the change and add a parallel
   regression test**, since the work item's own acceptance criterion ("all ~30
   downstream callers work unchanged in Conductor workspaces") is not fully met
   otherwise.
2. **Should the regression test also assert `vcs_mode` returns `git` for the
   worktree fixture?** If `vcs_mode` is fixed, an `assert_eq "git" "$(… vcs_mode)"`
   stanza against `make_git_linked_worktree` would lock it in symmetrically with
   the `find_repo_root` assertion.
3. **Nested-subdirectory coverage** — the work item suggests optionally testing
   `find_repo_root` from a nested subdir within the worktree. The fixture creates
   the worktree root only; a nested case would need an extra `mkdir -p`.
