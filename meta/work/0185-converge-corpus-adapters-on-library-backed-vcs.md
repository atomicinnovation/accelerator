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
last_updated: "2026-08-03T16:37:02+00:00"
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

## Amendment 2026-08-03 — 0188 has landed; corrections and inheritances

**Every reference in this item that attributes the library-backed adapter or the
zero-spawn harness to 0169 is wrong.** Both are
[`0188`](0188-library-backed-vcs-adapter.md)'s, and both have now landed. This
block raises the corrections rather than rewriting the sections above; where a
statement is now false it is quoted and marked.

- **The adapter exists and ships unwired.** `vcs_adapters::library::InProcessProbe`
  implements both `vcs` ports plus six inherent taxonomy queries.
  `vcs_adapters::facts` still names `MarkerWalkRoot`/`CommandProbe`, by design.
- **0185 owns the `vcs_adapters::facts` switch**, and the switch and the
  `CommandProbe` deletion are **one atomic change** — the composition root
  cannot move until nothing else needs the subprocess pair. This closes the open
  question 0188 recorded on the ordering; 0169 wires its own classifier port
  without touching `facts`.
- **The harness criterion here describes `PATH` stubs only.** What it inherits
  is strictly larger: marker-writing stubs *plus* a platform-aware absolute-path
  shadow list *plus* the empty-config environment, published from
  `cli/vcs-test-support` and already proven across a crate boundary by
  `cli/corpus-adapters/tests/zero_spawn.rs`. The strong form runs in the
  `check-zero-spawn` CI job.
- **The transitional dual-adapter comparison must be collapsed.**
  `cli/vcs-adapters/tests/detection.rs` now runs every case through an injected
  `(&dyn RepoRoot, &dyn VcsProbe)` seam against **both** implementations.
  Deleting `CommandProbe` means collapsing it to the library-backed pair alone.
- The assumption at `:117-121` that "0169 will need to alter this anyway" is
  **stale**.
- Two References anchors are stale: 0169 has no "Adapter-swap boundary"
  heading, and its Dependencies bullet is titled "Unowned debt this story
  creates".

Four inheritances that change this item's sizing:

1. **The deletion is a file deletion.** `InProcessProbe` delegates to the
   crate-private `walk_up`/`marker_kind`/`carries_any_marker` helpers rather than
   duplicating them, and `MarkerWalkRoot`/`CommandProbe` delegate to the same
   ones. Those helpers live in their own module (`markers.rs`), and as of
   2026-08-03 the subprocess pair does too (`subprocess.rs`) — the crate root
   holds no adapter code, only `facts` and the module declarations. So retiring
   the pair is: delete `subprocess.rs`, drop its `pub mod` line, repoint `facts`
   at `InProcessProbe`, and collapse `detection.rs`'s dual comparison. Do **not**
   delete `markers.rs` with it; the surviving adapter needs it.
2. **The containment delta is real and unpriced.** `CommandProbe` parses in a
   child process with a 10-second cap, kill-on-timeout and a scrubbed
   environment. `InProcessProbe` parses repository-controlled data in the
   caller's address space with no time bound, no memory bound and no crash
   isolation — and after the switch that runs inside `cli/visualiser/server` and
   on the hook path. **Decide whether an equivalent bound is needed before
   flipping `facts`.** The queries do distinguish failure from absence
   (`Result<Option<T>, Error>`), so a corrupt repository is observable rather
   than silently reported as "no VCS here"; that is containment of *meaning*,
   not of *blast radius*.
3. **jj `revision` does NOT transfer here — 0188 delivers it.** An earlier draft
   of this block said it did, on a spike finding that jj-lib 0.43 exposes no
   read-only, settings-free route to the working-copy commit id. That finding was
   **wrong** and was reversed on 2026-08-03: `jj_lib::protos` is a public module,
   so the workspace's checkout state (`operation_id` + `workspace_name`) decodes
   through published API, and `SimpleOpStore::load` takes a path only. 0188
   implements the full chain, verified against the live CLI across pure-jj,
   colocated, commitless, secondary-workspace and multi-workspace shapes, with a
   fingerprint assertion that it writes nothing.

   **What this changes for this story:** the "atomic switch plus deletion" sizing
   is *right* after all — there is no reason to retain `CommandProbe` for
   `revision` alone, and `InProcessProbe` implements `VcsProbe` **fully**, not
   partially. `detection.rs` asserts full `RepoFacts` equality for both idioms
   today, so the switch has no revision-shaped gap to close.

   **One behavioural difference to carry knowingly**, because this story's switch
   is what exposes users to it: asking the `jj` binary **snapshots the working
   copy first**, so it reports — and writes — a newly created commit when files
   changed since the last jj command. The in-process route reports the commit as
   of the last recorded operation and writes nothing. So after the switch,
   deriving metadata stops having a write side effect on the user's repository,
   and a `RepoFacts.revision` taken with unsnapshotted edits present names the
   last recorded commit rather than a fresh one. Decide whether any consumer
   depends on the snapshot semantics; nothing in the corpus writers appears to.
4. **sha256 repositories are unsupported by gix 0.85** (measured 2026-08-03):
   every gix-backed query returns `Err` on one, rather than misreading it.
   `detection.rs`'s `is_full_revision_id` also asserts 40 hex, and a sha256
   `HEAD` is 64. Any revision validation must accept both widths or record
   sha256 as unsupported — **this item's switch is what exposes a user on such a
   repository**, so the decision lands here. Reftable repositories read normally.
5. **The MPL-2.0 licence exception has to be re-checked when `facts` flips.**
   `uluru` (MPL-2.0, `gix-pack`'s LRU pack cache) is in the normal closure of the
   published `accelerator-visualiser` binary, but 0188 verified that dead-code
   elimination removes the whole `gix`/`jj-lib` closure from it — because nothing
   in the visualiser's reachable call graph enters `vcs-adapters` at all today,
   not even `CommandProbe`. §3.2's notice obligation therefore does not bind, and
   the `cli/deny.toml` exception comment records that finding **conditionally**.
   **This item's switch is the expected trigger that invalidates it.** Once the
   trees link into a distributed binary, distributing the Executable Form
   requires telling recipients how to obtain the Source Form, which means a
   third-party attribution artefact joining `_release_uploads()` — an asset set
   that carries no licence file today, and whose coverage `test_workflows.py`
   derives from that function. Re-run 0188's check (unstripped `--release` build;
   grep for `extensions.objectFormat` and `There is no Jujutsu repo`) as part of
   the switch, not after it.

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
