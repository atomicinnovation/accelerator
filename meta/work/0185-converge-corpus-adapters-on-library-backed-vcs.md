---
type: work-item
id: "0185"
title: "Converge corpus-adapters on the Library-Backed VCS Adapter"
date: "2026-07-31T08:36:03+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: medium
parent: "work-item:0136"
blocked_by: ["work-item:0188"]
relates_to: ["work-item:0125", "work-item:0179"]
tags: [rust, vcs, cleanup, tech-debt]
last_updated: "2026-07-31T08:36:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0185: Converge corpus-adapters on the Library-Backed VCS Adapter

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Migrate `cli/corpus-adapters` off the subprocess-based `CommandProbe` onto the
library-backed (`gix` / `jj-lib`) VCS adapter that 0169 introduces, then delete
`CommandProbe`. This closes the two-implementations state 0169 deliberately
leaves behind, and extends the zero-`jj`/`git`-spawn guarantee from the four
hook paths to every consumer of `vcs-adapters`.

## Context

0169 replaces `vcs-adapters`' subprocess probe with in-process `gix`/`jj-lib`
bindings, but **bounds that swap to the `vcs detect|status|log|guard` paths**.
The one production consumer outside those paths —
`cli/corpus-adapters/src/metadata.rs:201`, which calls `vcs_adapters::facts` and
reads `RepoFacts.name` and `.revision` to stamp artefact frontmatter — keeps
using `CommandProbe`.

That boundary was drawn deliberately, on review-2's recommendation (2026-07-31,
unanimous across the scope, completeness and testability lenses): converging the
metadata path inside 0169 would have coupled a pre-1.0 `jj-lib` bet to an
already-shipped consumer with its own parity suite, and nothing in the hooks
migration required it. The cost is that `vcs-adapters` ships **two** probe
implementations, and the zero-spawn property holds only for the hook paths.

This task pays that back once the library-backed adapter has proven itself on
the four paths.

## Requirements

- Route `cli/corpus-adapters`' `RepoFacts` resolution through the library-backed
  adapter instead of `CommandProbe`. The call site is
  `cli/corpus-adapters/src/metadata.rs:201`; only `.name` and `.revision` are
  read (`:185-186`), so the required surface is narrow.
- Delete `CommandProbe` (`cli/vcs-adapters/src/lib.rs:73`) and its supporting
  subprocess machinery once it has no callers — including the capped-stdout
  helper and environment scrubbing that exist solely to serve it, if nothing
  else uses them.
- Preserve `vcs_adapters::facts`'s existing signature and semantics
  (`cli/vcs-adapters/src/lib.rs:225`) so the change is invisible to callers: a
  repository with no commits still yields `revision: None`, a bare repository
  still yields `None` facts, and a jj secondary workspace still reports the
  repository's name rather than the workspace directory's.
- Extend the zero-spawn assertion to cover the corpus metadata read, so the
  guarantee is crate-wide rather than path-scoped.

## Acceptance Criteria

- [ ] `CommandProbe` no longer exists in `cli/vcs-adapters`, and no
      `Command::new` for `jj` or `git` remains in the crate's non-test code.
- [ ] `cli/corpus-adapters` obtains `RepoFacts` through the library-backed
      adapter, and its existing suites pass unchanged —
      `cli/corpus-adapters/tests/parity.rs`,
      `cli/corpus-adapters/tests/metadata.rs`, and
      `cli/corpus-adapters/tests/work_item_pattern_parity.rs`.
- [ ] The zero-spawn black-box assertion introduced by 0169 (fixture repos run
      with `PATH` containing only marker-writing `git`/`jj` stubs) is extended
      to a `corpus-adapters` metadata read: no marker is written and the read
      still succeeds.
- [ ] Behaviour is unchanged at the boundaries that
      `cli/vcs-adapters/tests/detection.rs` already pins: no-commits →
      `revision: None`; bare repository → no facts; colocated → `VcsKind::Jj`;
      jj secondary workspace → the repository's name, not the workspace
      directory's.
- [ ] `mise run` is green end to end.

## Dependencies

- **Blocked by**: 0188 — the library-backed adapter does not exist until it
  lands. (Originally recorded against 0169; repointed when 0169 was split and
  the adapter work moved to 0188.)
- **Related**: 0179 (delivered the `vcs`/`vcs-adapters` crate pair this
  modifies); 0125 (converge lexical VCS detection on the probe layer — a
  separate, shell-side convergence, but the same underlying "several detection
  implementations coexist" problem, and worth sequencing consciously against).
- **Parent**: epic 0136.

## Assumptions

- 0169 lands the library-backed adapter behind the existing `RepoRoot` /
  `VcsProbe` ports, so this task is a wiring change plus a deletion rather than
  new adapter work. If 0169 instead builds a `vcs`-local adapter that does not
  generalise, this task grows and should be re-estimated.
- The corpus metadata path needs no VCS query beyond `name` and `revision`.
  Worth re-checking at implementation time — if `RepoFacts` has gained fields or
  consumers by then, the surface to preserve is wider.

## Technical Notes

- Consumer: `cli/corpus-adapters/src/metadata.rs:201` (`vcs_adapters::facts`),
  reading `.name` / `.revision` at `:185-186`; `use vcs::RepoFacts` at `:14`.
- Composition root to change: `vcs_adapters::facts`
  (`cli/vcs-adapters/src/lib.rs:225-227`) currently hard-wires `MarkerWalkRoot`
  + `CommandProbe::new()` with no injection variant. 0169 will need to alter
  this anyway; this task finishes the job by removing the old probe rather than
  leaving both wired.
- `CommandProbe`'s subprocess surface is small — two commands (`jj log -r @ -T
  commit_id` at `:110-120`, `git rev-parse HEAD` at `:124-125`) funnelling
  through a single `spawn()` at `:168` — so the deletion is well-bounded.
- Watch the crate's own unit tests: several drive the private `capped_stdout`
  and `scrub_environment` helpers directly
  (`cli/vcs-adapters/src/lib.rs:229-322`) using generic shell binaries rather
  than `jj`/`git`. They test machinery that exists only for `CommandProbe` and
  should go with it, not be retargeted.
- The `bash-parity` feature gate on `cli/vcs-adapters/tests/detection.rs` means
  "needs real `jj`/`git` binaries to build fixtures", not "shells out in
  production" — it stays relevant after this change, since fixtures are still
  built with the real binaries.

## Drafting Notes

- Raised by 0169's downstream-hand-off acceptance criterion, which requires a
  follow-up item owning this convergence rather than leaving it as unowned debt.
- Sized as a `task` rather than a story: it is a wiring change plus a deletion
  behind an adapter another item delivers, with no new behaviour and no user-
  visible change.
- Priority `medium`: nothing is broken while both implementations coexist — the
  cost is a second code path and a narrower zero-spawn guarantee, not a defect.
  It should not block epic 0136's shell-retirement work.

## References

- Boundary and rationale: `meta/work/0169-vcs-subdomain-and-hooks-migration.md`
  (Requirements → "Adapter-swap boundary"; Dependencies → "Unowned debt this
  story creates or inherits", item 1)
- Review that recommended the boundary:
  `meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md` (Pass
  3, 2026-07-31)
- Crate state and call sites:
  `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  §1
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
