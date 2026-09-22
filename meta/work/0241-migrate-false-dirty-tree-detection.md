---
type: "work-item"
id: "0241"
title: "False Dirty-Tree Detection on jj-Colocated Repositories"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "bug"
priority: "high"
parent: "work-item:0136"
relates_to: ["work-item:0119", "work-item:0286", "work-item:0124"]
tags: ["migration", "vcs", "jj"]
last_updated: "2026-09-22T21:40:39+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-771"
---

# 0241: False Dirty-Tree Detection on jj-Colocated Repositories

**Kind**: Bug
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As a user of a jj-colocated repository, I want the dirty-tree checks to agree
with `jj status`, so that `accelerator migrate` resumes partial runs and
refuses only over genuinely foreign changes.

On jj, the migrate run base is anchored to the working-copy commit `@`, whose
id changes on every snapshot, so the guarded resume never engages. The jj
dirty-path snapshot also ignores the global and repo-local git excludes that
`jj status` honours. Both force users onto `ACCELERATOR_MIGRATE_FORCE=1`,
which removes the safety net the pre-flight exists to provide.

## Context

Reported in the further-ideas backlog as "`accelerator migrate` detect dirty
tree when not dirty", and reproduced in GitHub issue #97: in a jj-colocated
repository, resuming a stalled 0007 migration with `--decisions-file` is
refused as a dirty working tree even though every dirty path is the
migration's own output and the base is unchanged. The stall message promises
a resume that never happens.

The guarded resume (built under 0119) resumes only when the recorded run id
equals the current VCS revision. Under git that revision is `HEAD`, the
commit beneath the working tree, and is stable across edits. Under jj it is
`@` itself, which is rewritten on every snapshot, including the `jj status`
the user runs to inspect the dirt.

The governing principle: the current jj change is treated exactly as git
treats its working copy. Its parent is the base; its content is the dirt.

## Requirements

1. On jj, the VCS revision fact is the working-copy commit's parent, the
   analogue of git's `HEAD`, so it is invariant under working-copy edits and
   snapshots and advances only when a new change is started.
2. The jj dirty-path snapshot honours `core.excludesFile` and
   `.git/info/exclude` as `jj status` does, for every consumer of the shared
   working-copy snapshot: the migrate pre-flight, `work sync` dirtiness, and
   the VCS status renderer.
3. The jj dirty-path snapshot does not report untracked files larger than
   jj's `snapshot.max-new-file-size` (default 1 MiB), which `jj status` would
   not auto-track.
4. The migrate dirty-tree refusal names each foreign dirty path.
5. Git behaviour is unchanged apart from requirement 4.

## Acceptance Criteria

- [ ] Given a jj-colocated repository where migration 0007 has stalled for
  want of decisions, when the user writes the named decisions file, runs
  `jj status`, then runs `accelerator migrate --decisions-file <path>`, then
  the guarded resume engages and completes without
  `ACCELERATOR_MIGRATE_FORCE`.
- [ ] Given a jj repository, when a file under a migrate scope is edited and
  the working copy is snapshotted, then the reported revision is unchanged;
  and when `jj new` or `jj commit` starts a new change, then it changes.
- [ ] Given a jj repository whose working-copy commit is a merge, when the
  revision is read, then it identifies all of the parents deterministically
  regardless of their order.
- [ ] Given a jj repository whose only untracked file under `meta/` matches a
  pattern in `core.excludesFile`, when the dirty paths are computed, then
  nothing is reported and `accelerator migrate` proceeds; and likewise for a
  pattern in `.git/info/exclude`.
- [ ] Given a jj repository with an untracked file under `meta/` larger than
  `snapshot.max-new-file-size`, when the dirty paths are computed, then that
  file is not reported.
- [ ] Given foreign dirt at `meta/a.md` and `.accelerator/b`, when
  `accelerator migrate` refuses, then stderr names both paths alongside the
  existing refusal text.
- [ ] Given the git equivalents of the resume, exclude, and refusal
  scenarios above, then the outcome matches today's behaviour apart from the
  listed paths.

## Open Questions

- Moving the jj revision fact to the parent of `@` also changes the revision
  stamped into corpus document metadata. Is that the intended semantics
  there, or should metadata keep stamping `@`?

## Dependencies

- Blocked by: none
- Blocks: none
- Related: 0119 (built the guarded resume this repairs), 0286 (per-VCS wording
  of the migrate skill's dirty-path guidance), 0124 (a separate false-dirty
  bug in git-worktree root detection)

## Assumptions

- Issue #97 and the backlog entry describe the same defect. The exclude and
  size parity requirements are inferred from the code rather than observed.
- Anchoring to the parent of `@` means content already snapshotted into an
  undescribed or described-but-not-closed `@` counts as dirty, matching git's
  uncommitted changes.

## Technical Notes

- Run-base check: `cli/migrate/src/preflight.rs` compares the recorded run id
  with `revision`, sourced via `FileMigrationContext::revision` in
  `cli/migrate-adapters/src/context.rs`.
- jj revision: `jj_revision` in `cli/vcs-adapters/src/library.rs` returns the
  view's working-copy commit id; git's `git_revision` returns `HEAD`.
- jj snapshot: `snapshot::working_copy_diff` in
  `cli/vcs-adapters/src/library/snapshot.rs` passes
  `base_ignores: GitIgnoreFile::empty()` and `max_new_file_size: u64::MAX`;
  its diff baseline is already the parent tree of `@`.
- Refusal text: `DIRTY_TREE_REFUSAL` in `cli/migrate-cli/src/render.rs`;
  `PreflightError::ForeignDirt` carries no paths today.
- The git dirty path via gix already drops stat-only `NeedsUpdate` entries,
  so git is not implicated.

## Drafting Notes

- "Every consumer" is read as every caller of `dirty_paths` and
  `working_copy_diff`; the revision change is made at the fact level, so
  corpus metadata is affected too (see Open Questions).
- The size-parity requirement follows from treating `@` like git's working
  copy, not from an observed failure.
- Priority raised to high because #97 makes the guarded resume unusable on
  jj and forces the force-bypass.
- Title broadened from `migrate` because the defect lives in the shared VCS
  adapter.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
- Reproduction: https://github.com/atomicinnovation/accelerator/issues/97
- Related: 0119, 0286, 0124
