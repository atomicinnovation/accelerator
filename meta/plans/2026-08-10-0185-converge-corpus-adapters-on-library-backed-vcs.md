---
type: plan
id: "2026-08-10-0185-converge-corpus-adapters-on-library-backed-vcs"
title: "Converge corpus-adapters on the Library-Backed VCS Adapter Implementation Plan"
date: "2026-08-10T15:52:48+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0185"
parent: "work-item:0185"
derived_from: ["codebase-research:2026-08-10-0185-converge-corpus-adapters-library-backed-vcs"]
tags: [rust, vcs, cleanup, tech-debt]
revision: "1e785e44f25480111414fe805bf645510d028fef"
repository: "accelerator"
last_updated: "2026-08-10T16:52:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Converge corpus-adapters on the Library-Backed VCS Adapter Implementation Plan

## Overview

`cli/corpus-adapters` reads repository facts (`name`, `revision`) through
`vcs_adapters::facts`, which today wires the subprocess-based `CommandProbe`
(spawns `jj`/`git`). 0188 delivered a fully-implemented, unwired
library-backed alternative, `InProcessProbe` (`gix`/`jj-lib`, no subprocess
spawn). This plan repoints `facts`'s composition root onto `InProcessProbe`,
deletes `CommandProbe`, and extends the crate's zero-spawn test guarantee to
cover the corpus metadata-read path — closing the two-implementation state
0169 deliberately left behind.

## Current State Analysis

- `vcs_adapters::facts` (`cli/vcs-adapters/src/lib.rs:22-26`) hard-wires
  `MarkerWalkRoot` + `CommandProbe::new()` with no injection seam.
- `CommandProbe` (`cli/vcs-adapters/src/subprocess.rs:65-139`) spawns `jj log
  -r @ -T commit_id` / `git rev-parse HEAD` for `revision`, and `git remote
  get-url origin` for `OriginRemote`, through shared `scrub_environment`/
  `run_capped` helpers.
- `InProcessProbe` (`cli/vcs-adapters/src/library.rs:190` onward) already
  implements every port `CommandProbe` implements — `RepoRoot`, `VcsProbe`,
  `OriginRemote`, plus taxonomy queries `CommandProbe` doesn't need — with no
  gaps. `tests/detection.rs` already asserts full `RepoFacts` equality
  between the two adapters across seven checkout shapes.
- The sole non-hook, non-`vcs`-subdomain production consumer of
  `vcs_adapters::facts` is `VcsBackedRepoFactsProbe::facts`
  (`cli/corpus-adapters/src/metadata.rs:214`), a one-line delegation reached
  through the injected `RepoFactsProbe` port from `derive_at`
  (`metadata.rs:228-236`).
- The existing zero-spawn proof (`cli/corpus-adapters/tests/zero_spawn.rs`)
  exercises `InProcessProbe`'s individual queries via a reference binary
  (`vcs-adapters-fixture`, declared in `cli/vcs-adapters/Cargo.toml:23-25`),
  but that binary never calls `vcs_adapters::facts` or
  `VcsBackedRepoFactsProbe` — the metadata-read path is unproven.
- `tests/detection.rs` runs a transitional dual-adapter comparison
  (`facts` via `CommandProbe`, `library_facts` via `InProcessProbe`, asserted
  equal by `assert_implementations_agree`) that must collapse to the
  library-backed path alone once `CommandProbe` is gone.
  `tests/library.rs` runs a second, separate dual-adapter comparison
  (`assert_parity`, plus a direct `CommandProbe::new().revision(...)` call
  in `an_unsnapshotted_edit_is_the_one_documented_divergence`) that must
  collapse too — easy to miss because it lives in a different file from the
  one named in the work item's Technical Notes.

### Key Discoveries:

- The corpus-adapters call site has moved behind a `RepoFactsProbe`
  injection seam since the work item's own Technical Notes were last
  checked — the repoint target is the single line
  `cli/corpus-adapters/src/metadata.rs:214`, not an inline call inside
  `derive_at`.
- `cli/corpus-adapters/tests/work_item_pattern_parity.rs`, named in the work
  item's AC2, does not exist in the tree. The suites that must keep passing
  unchanged are the ones that actually exist and touch this path:
  `cli/corpus-adapters/tests/zero_spawn.rs`,
  `cli/corpus-adapters/tests/metadata.rs`, and
  `cli/corpus-adapters/tests/parity.rs`.
- `scrub_environment`/`run_capped` (`subprocess.rs:231-246,248-324`) are
  shared with 0198's `run_vcs_text` (the `status`/`log` subprocess path) and
  must survive `CommandProbe`'s deletion; `run_checked`/
  `wait_capped_checked` (`subprocess.rs:329-407`) are used only by
  `CommandProbe`'s `OriginRemote` impl and delete cleanly alongside it.
- `markers.rs` is shared ancestor-walk/marker logic that both adapters
  delegate to rather than duplicate; it must not be touched.
- `gix` 0.85 cannot read a sha256 repository at all — every query returns
  `Err`, which `InProcessProbe::revision` already folds to `None` via its
  documented fallible-to-`Option` contract. No new error handling is
  required; only the policy needs recording.
- `cli/visualiser/server` does not call `vcs_adapters::facts`,
  `VcsBackedRepoFactsProbe`, or `derive_at` anywhere today — the whole
  `gix`/`jj-lib` closure is dead-code-eliminated from the shipped binary.
  The work item's framing that this switch "first makes the server
  reachable" does not hold against the current codebase; the MPL-2.0
  re-check (Phase 4) records whatever the unstripped build actually shows
  rather than assuming reachability changes.
- The visualiser is not the only dispatched sub-binary in this position,
  though: four of the six `DISPATCHED_SUBBINARIES` tokens
  (`tasks/shared/paths.py:29-36`) already reach `vcs_adapters` today,
  independent of this plan, through call sites unrelated to `facts`/
  `derive_at` — `cli/vcs-cli` and `cli/collaboration-cli` construct
  `InProcessProbe` directly for `detect`/`guard`/origin resolution;
  `cli/migrate-adapters` (`context.rs`'s `FileMigrationContext::revision`
  and `dirty_path_scanner.rs`'s `VcsDirtyPathScanner::dirty_paths`, both
  called unconditionally from `migrate-cli`'s default run path) does the
  same for migration scanning; `cli/work-adapters`'s
  `VcsBackedIdentityProbe` (`author.rs`) does the same for `work create`'s
  author resolution, unrelated to the `derive_at` path this plan repoints.
  Their shipped binaries (`accelerator-vcs`, `accelerator-collaboration`,
  `accelerator-migrate`, `accelerator-work`) very likely already carry the
  closure. 0188's MPL-2.0 verification checked none of them. Only `corpus`
  becomes newly reachable through this plan's switch — `corpus-adapters`
  has no other call site into `vcs_adapters` today. Phase 4 broadens its
  re-check to every one of the six dispatched sub-binary tokens, not the
  visualiser alone.
- `InProcessProbe` already runs unbounded, synchronously, in production
  today on the `SessionStart`/`PreToolUse` hook path (`vcs detect`/`vcs
  guard`, wired via `cli/vcs-cli/src/main.rs:27-69`). This switch extends
  the same code path's reach — most sharply at `work-cli`'s `create`, which
  holds a work-item creation lock for the duration of the `derive_at` call
  this port serves, and whose reclaim mechanism only reclaims a dead
  holder — so a hang here is a new, not merely inherited, blast radius even
  though the underlying exposure is not new in kind.
- No `cli/corpus-adapters` write path depends on the CLI's snapshot-on-read
  side effect. The two production callers of `derive_at` are `corpus-cli`'s
  `corpus metadata derive` (prints to stdout only) and `work-cli`'s `work
  create` (persists frontmatter, but reads only `metadata.datetime_utc`,
  never `.revision`/`.repository_name`). This does not extend to `SKILL.md`
  workflows (`create-plan`, `research-issue`, `create-note`, and others)
  that call `corpus metadata derive` directly and copy its printed
  `Current Revision:` line into committed `meta/` frontmatter — those
  consumers inherit the same staleness window, accepted as a best-effort
  provenance degradation rather than a correctness regression.

## Desired End State

`vcs_adapters::facts` resolves `RepoFacts` entirely in-process via
`InProcessProbe`, with no `Command::new` for `jj`/`git` remaining anywhere in
`cli/vcs-adapters`'s non-test code that serves `facts`. `CommandProbe` no
longer exists. A black-box test proves the corpus-adapters metadata-read path
spawns no subprocess. Three policy decisions (sha256 handling, containment
bound, snapshot-on-read dependency) are recorded in the codebase, not left
open. `mise run` is green end to end.

**Verification**: `mise run check` passes; `cargo nextest run -p
vcs-adapters -p corpus-adapters` passes, including the new zero-spawn
extension; a repo-wide search for `CommandProbe` and `MarkerWalkRoot`
returns no matches outside history; unstripped `--release` builds of all
six `DISPATCHED_SUBBINARIES` (`accelerator-visualiser`, `accelerator-vcs`,
`accelerator-work`, `accelerator-corpus`, `accelerator-collaboration`,
`accelerator-migrate`) are each checked against the MPL-2.0 literals and
`cli/deny.toml`'s exception comment reflects the actual finding for all
six.

## What We're NOT Doing

- Not touching `status`/`log`'s separate subprocess path (`run_vcs_text` in
  `subprocess.rs`) — owned by 0198, explicitly out of scope, and the reason
  `scrub_environment`/`run_capped` survive.
- Not adding a timeout, memory cap, or crash-isolation wrapper around
  `InProcessProbe` — Phase 1 records a deliberate decision that none is
  needed, not an implementation of one.
- Not building a reusable bounded-execution primitive for every
  `InProcessProbe` call site (`facts`, `detect`, `guard`) — out of scope for
  this item; it would also address the pre-existing hook-path exposure,
  which nothing here requires.
- Not creating `cli/corpus-adapters/tests/work_item_pattern_parity.rs` — the
  work item's AC2 reference to it is stale; the plan targets the suites that
  actually exist.
- Not changing `markers.rs` — the shared ancestor-walk/marker logic both
  `MarkerWalkRoot` and `InProcessProbe` delegate to stays as it is.
  `MarkerWalkRoot` itself is not left unchanged: it is deleted in Phase 3
  once its last caller (the dual-adapter comparisons) is gone — see Phase 3,
  item 1.
- Not widening `is_full_revision_id` or any revision-format validation to
  accept 64-hex sha256 ids — `gix` cannot read such a repository regardless
  of hex width, so widening the check would not change behaviour.

## Implementation Approach

Four independently-mergeable phases, ordered so every behavioural change is
preceded by the policy decisions it depends on, and every deletion is
preceded by the collapse of whatever still references the thing being
deleted:

1. Record the three pending policy decisions as doc comments — no behaviour
   change, unblocks the repoint.
2. Add failing zero-spawn coverage for the corpus metadata-read path, then
   flip the composition root — classic red/green within one mergeable
   change, since a failing test can't ship alone without breaking CI.
3. Delete `CommandProbe` and `MarkerWalkRoot`, and collapse the transitional
   dual-adapter test comparisons in both `detection.rs` and `library.rs` —
   now safe, since nothing else references any of them.
4. Re-run the MPL-2.0 licence check across every dispatched sub-binary that
   reaches `vcs_adapters`, and update the recorded finding.

## Phase 1: Record the pending policy decisions

### Overview

Three decisions the work item's Acceptance Criteria require to be "made and
recorded" before the composition root changes. Mostly documentation-only —
no logic changes to non-test `.rs` code — with two test additions that back
the decisions being recorded rather than leaving them as unproven claims:
a sha256 `revision()` case (no existing test actually exercises `revision`
against a sha256 fixture) and a git-side malformed-ref-data case (the
containment-bound decision's "failure distinguishes itself from absence"
claim currently has jj-side proof only).

### Changes Required:

#### 1. sha256 revision-handling policy

**File**: `cli/vcs/src/lib.rs`
**Changes**: Add a doc comment to `VcsProbe::revision` (`:65-73`) recording
that a sha256 repository is unsupported: `gix` 0.85 cannot read one at all
(every query returns `Err`), so `revision` folds that failure to `None`
through its existing fallible-to-`Option` contract — the same path as any
other probe failure. Follows the precedent set by
`OriginRemote::origin_url`'s doc comment (`cli/vcs/src/origin_remote.rs:12-22`)
of recording port-behaviour policy at the trait method.

```rust
/// Reports a repository's idiom and its working-copy revision.
pub trait VcsProbe {
    fn kind(&self, root: &Path) -> VcsKind;

    /// The full working-copy revision, or `None` when the repository has none
    /// and when the probe cannot answer. A caller cannot distinguish the two;
    /// an adapter is expected to log the failure.
    ///
    /// A sha256-format repository is unsupported: the underlying `gix` query
    /// fails to read one at all, so this folds to `None` like any other
    /// probe failure, rather than misreading the revision.
    fn revision(&self, root: &Path, kind: VcsKind) -> Option<String>;
}
```

Also add a test proving this: extend `cli/vcs-adapters/tests/queries.rs` (or
add a sibling near `an_unsupported_object_format_fails_rather_than_misreads`)
with a case that calls `InProcessProbe.revision(&sha256_root, VcsKind::Git)`
against the crate's existing S256 fixture and asserts `None`. The two
existing sha256 tests (`queries.rs`, `classify.rs`) cover `is_bare`/
`worktree`/`dual_roots`/`classify` failing on this fixture, but neither
calls `revision`, so the doc comment's claim is otherwise unproven.

#### 2. Containment-bound decision

**File**: `cli/vcs-adapters/src/library.rs`
**Changes**: Add a doc comment above `InProcessProbe` (`:190`) recording the
decision that no timeout, memory cap, or crash-isolation bound is added.
Rationale: the same unbounded exposure already runs in production today for
`vcs detect`/`vcs guard` on the hook path; no crash-isolation precedent
exists anywhere in `cli/`; and `InProcessProbe`'s `Result<Option<T>, Error>`
return already distinguishes failure from absence, which is containment of
meaning, not blast radius, but is the containment this port promises. The
comment names what this removes, not only what it declines to add:
`CommandProbe` ran the `facts()` call site under a 10-second cap with
kill-on-timeout, and this decision takes that away from it. The sharpest
blast radius reached *through `facts()`* is `work-cli`'s `create`, which
holds a work-item creation lock for the duration of the call — the lock's
reclaim mechanism only reclaims a dead holder, so a hang here fails every
subsequent `work create` after its own five-minute lock-acquisition wait,
until an operator kills the hung process (a comparable, pre-existing
exposure already applies to `migrate-adapters`' own direct, non-`facts`
`InProcessProbe` calls under `migrate`'s run lock — untouched by this
plan). The revisit condition is stated structurally, not by ticket number:
no work item currently owns adding a server call site into `facts`, so
there is nothing to cross-reference yet; the condition is written so it
applies regardless of which future item does it.

```rust
/// Parses repository-controlled data in the caller's address space — no
/// subprocess boundary, no time bound, no memory bound, no crash isolation.
///
/// This removes a protection that existed at the `facts()` call site before
/// this switch: `CommandProbe` ran under a 10-second cap with
/// kill-on-timeout. The sharpest blast radius reached through `facts()` is
/// `work-cli`'s `create`, which holds a work-item creation lock for the
/// duration of the `derive_at` call this port serves — the lock's reclaim
/// mechanism only reclaims a dead holder, so a hang here fails every
/// subsequent `work create` after its own five-minute lock-acquisition
/// wait, until an operator kills the hung process.
///
/// This is a deliberate decision, not an oversight: the same unbounded
/// exposure already runs in production for the `vcs detect`/`vcs guard`
/// hook path, no crash-isolation precedent exists anywhere in this
/// workspace, and adding one here would be a first-of-its-kind primitive
/// introduced ahead of any incident that demonstrates the need. Revisit
/// when any code under `cli/visualiser/server` calls `vcs_adapters::facts`,
/// `derive_at`, or `VcsBackedRepoFactsProbe` — this decision was priced
/// against a single-shot CLI/hook caller, not a persistent multi-request
/// one — or if a lock-holding hang is observed in practice.
pub struct InProcessProbe;
```

#### 3. Malformed git-ref revision test

**File**: `cli/vcs-adapters/tests/library.rs`
**Changes**: `an_unreadable_checkout_state_reports_absence_rather_than_a_wrong_commit`
already proves the jj side of the containment-bound decision's "failure
distinguishes itself from absence" claim — writing garbage bytes over
`.jj/working_copy/checkout` and asserting `revision` returns `None` rather
than panicking. There is no git-side equivalent. Add a sibling test that
corrupts a git repository's ref data after a valid initial read (e.g.
overwrite `.git/HEAD` or the relevant `refs/heads/*` file with non-UTF-8 or
truncated bytes) and asserts `InProcessProbe.revision(&root, VcsKind::Git)`
returns `None` rather than panicking, so both dispatch paths this switch
widens the reach of have a proven graceful-failure case, not only the jj
one.

#### 4. Snapshot-on-read dependency confirmation

**File**: `cli/corpus-adapters/src/metadata.rs`
**Changes**: Add a doc comment to `VcsBackedRepoFactsProbe`
(`:211-212`) recording that no corpus-adapters write path depends on the
CLI's snapshot-on-read side effect. `work-cli`'s `create` (the one
production write path that persists frontmatter) reads only
`metadata.datetime_utc`, never `.revision`/`.repository_name`;
`corpus-cli`'s `corpus metadata derive` only prints to stdout. This
confirmation is scoped to `cli/corpus-adapters`' own Rust write paths; it
does not cover `SKILL.md` workflows (`create-plan`, `research-issue`,
`create-note`, and others) that call `corpus metadata derive` directly and
copy its printed `Current Revision:` line into committed `meta/`
frontmatter — the doc comment records that scope explicitly rather than
implying the question is closed for those consumers too.

```rust
/// Resolves repository facts through the library-backed VCS adapter.
///
/// No `cli/corpus-adapters` write path depends on the CLI's
/// snapshot-on-read side effect (writing a new commit for unsnapshotted
/// working-copy changes): the one production write path that persists
/// frontmatter (`work create`) reads only `metadata.datetime_utc`, never
/// `.revision`/`.repository_name`.
///
/// This does not extend to `SKILL.md` workflows that call `corpus metadata
/// derive` directly and copy its printed `Current Revision:` line into
/// committed `meta/` frontmatter (e.g. `create-plan`, `research-issue`,
/// `create-note`). Those consumers inherit a staleness window this switch
/// introduces: an artefact authored with unsnapshotted working-copy edits
/// present records the last recorded operation's commit rather than a
/// freshly snapshotted one. Accepted as a best-effort provenance
/// degradation, not a correctness regression — nothing downstream treats
/// these fields as exact — but it is a real, if narrow, change to
/// persisted data, not only to stdout.
#[derive(Debug, Clone, Copy, Default)]
pub struct VcsBackedRepoFactsProbe;
```

### Success Criteria:

#### Automated Verification:

- [x] Workspace builds: `cargo build --workspace` (run from `cli/`)
- [x] New sha256 `revision()` test passes and demonstrably calls `revision`
      (not only `is_bare`/`worktree`/`dual_roots`/`classify`):
      `cargo test -p vcs-adapters --features bash-parity --test queries`
- [x] New malformed-git-ref `revision()` test passes:
      `cargo test -p vcs-adapters --features bash-parity --test library`
- [x] `cli` component check passes: `mise run cli:check`
- [x] Full check suite passes: `mise run check`

#### Manual Verification:

- [x] Each doc comment reads as a decision record (states the choice and the
      reason), not a description of what the code already shows

---

## Phase 2: Extend zero-spawn coverage, then repoint `facts`

### Overview

Prove — before making the change — that the corpus metadata-read path can be
made to spawn no subprocess, then make it true. The `vcs-test-support`
`Stubs` harness only patches a spawned child process's `PATH`, so the new
assertion needs the code under test running as a subprocess itself; the
existing `vcs-adapters-fixture` binary can't be extended to cover this,
since `vcs-adapters` cannot depend on `corpus-adapters` (wrong dependency
direction) — a new `corpus-adapters`-local reference binary is needed.

### Changes Required:

#### 1. New reference binary for the metadata-read path

**File**: `cli/corpus-adapters/tests/fixtures/corpus_adapters_fixture.rs`
(new)
**Changes**: A minimal binary, mirroring `vcs-adapters-fixture`'s shape and
placement, that takes a repository path as its argument, constructs a
`VcsBackedRepoFactsProbe`, calls `.facts(path)`, and prints the result to
stdout. Declared in `cli/corpus-adapters/Cargo.toml` as a `[[bin]]` sourced
from `tests/fixtures/`, not `src/bin/`, matching `vcs-adapters-fixture`'s
own placement — its `Cargo.toml` comment records that choice as keeping the
binary off the crate's normal build surface, and `tasks/build.py`'s
release-staging glob only looks under `src/bin/`, so a diagnostic-only
binary placed there would be one glob change away from shipping. Matching
that sibling exactly (not merely its placement): the `[[bin]]` entry itself
carries no `required-features` gate and always builds; only the two new
zero-spawn tests that invoke it (change #2 below) are gated behind
`bash-parity`, the same split `vcs-adapters-fixture` uses between its
always-built binary and its `#![cfg(feature = "bash-parity")]`-gated
tests.

#### 2. Failing-first zero-spawn assertion

**File**: `cli/corpus-adapters/tests/zero_spawn.rs`
**Changes**: Two new tests, `a_git_metadata_read_spawns_no_subprocess` and
`a_jj_metadata_read_spawns_no_subprocess`, following the existing test's
pattern: build a git fixture repository and a jj fixture repository (via
`vcs-test-support`'s `fixtures` module), run the new
`corpus-adapters-fixture` binary twice per fixture (unrestricted vs.
`Stubs`-applied `PATH`), assert identical stdout and `!stubs.spawns()` for
each. Two kinds, not one: `facts` dispatches differently per `VcsKind`
(`git_revision`/`jj_revision` are separate code paths in `library.rs`), and
the point of this test is proving the *composition* —
`VcsBackedRepoFactsProbe` → `vcs_adapters::facts` → `InProcessProbe` —
spawns nothing end-to-end for either dispatch path, not re-proving the
individual queries' zero-spawn property (already proven per-query by the
existing `vcs-adapters-fixture`-driven test).

Confirm both tests fail against the current `CommandProbe` wiring (a marker
is written) before proceeding — this is the red step. Do not skip running it
red; the point is proving the assertion actually exercises the composition
root being changed.

#### 3. Repoint the composition root

**File**: `cli/vcs-adapters/src/lib.rs`
**Changes**: Replace the `MarkerWalkRoot`/`CommandProbe::new()` wiring with
`InProcessProbe`, serving both the `RepoRoot` and `VcsProbe` positions from
the single unit-struct value.

```rust
use crate::library::InProcessProbe;

#[must_use]
pub fn facts(start: &Path) -> Option<RepoFacts> {
    vcs::facts(start, &InProcessProbe, &InProcessProbe)
}
```

Also update the crate's module-level doc comment (`:1-10`), which
currently states "[`subprocess`] ... is what [`facts`] uses" — false the
moment this repoint lands. At minimum, correct that one sentence to name
`library`/`InProcessProbe` as what `facts` uses now; Phase 3's deletion of
`MarkerWalkRoot` further changes what's true of `subprocess` (see Phase 3,
item 1's note on this same doc comment), so a second, smaller update lands
there rather than getting both changes right in one pass here.

One narrow behavioural note carried by this repoint, not requiring a code
change: `InProcessProbe::discover`/`.repository_root` canonicalise
(resolve symlinks); `MarkerWalkRoot`'s did not. `VcsBackedRepoFactsProbe`
only forwards `.name`/`.revision`, not `.root`, so this only surfaces if
the repository directory itself (not merely an ancestor) is a symlink, in
which case the canonicalised final path component can change the
`Repository Name:` value persisted into frontmatter. Narrow enough not to
warrant new test infrastructure for this item, but worth naming here
rather than leaving silent.

Confirm Phase 2's new tests now pass (green), alongside every existing
`vcs-adapters`/`corpus-adapters` test.

### Success Criteria:

#### Automated Verification:

- [ ] Both new zero-spawn tests fail before the composition-root change
      (verified manually during implementation, not a lasting CI state)
- [ ] Both new zero-spawn tests pass after the composition-root change:
      `cargo test -p corpus-adapters --features bash-parity --test
      zero_spawn`
- [ ] Existing corpus-adapters suites pass unchanged: `cargo test -p
      corpus-adapters --test parity --test metadata`
- [ ] `cli` component check passes: `mise run cli:check`
- [ ] `mise run check` passes

#### Manual Verification:

- [ ] `corpus metadata derive` run against a real jj repository still
      prints the expected `Repository Name:`/`Current Revision:` lines

---

## Phase 3: Delete `CommandProbe` and collapse the dual-adapter comparison

### Overview

Nothing production-facing references `CommandProbe` after Phase 2. This
phase removes it, its `OriginRemote`-only helpers, its redundant unit tests,
and the transitional test scaffolding that compared it against
`InProcessProbe` — in both `tests/detection.rs` and `tests/library.rs`. The
latter is easy to miss: it imports `CommandProbe` directly, and its
`an_unsnapshotted_edit_is_the_one_documented_divergence` test is the sole
automated proof of the snapshot-on-read behavioural difference Phase 1
documents, so its rework (not deletion) matters.

### Changes Required:

#### 1. Delete `CommandProbe`, `MarkerWalkRoot`, and their dedicated helpers

**File**: `cli/vcs-adapters/src/subprocess.rs`
**Changes**: Delete the `CommandProbe` struct, its preceding doc comment,
and its `VcsProbe`/`OriginRemote` impls (`:63-139` — the doc comment
("Reads the repository's idiom from its markers and its revision by
running the matching VCS binary.") sits just above the struct at `:63-64`;
stopping the range at `:65` leaves it dangling, silently re-attached to
whatever follows). Delete `run_checked`/`wait_capped_checked` and their
preceding doc comment (`:326-407`, used only by `CommandProbe`'s
`OriginRemote` impl; same off-by-a-comment-block reasoning as above). Delete
the three `CommandProbe`-specific unit tests together with the
`origin_repo()` helper that only they call (`:570-632` — `origin_repo()`
sits just above the tests and has no caller once they're gone; deleting the
tests alone without it leaves an orphaned private function). Within that
same test module, also drop its own now-unused imports: `use
std::path::PathBuf;` (`:414`, used only by the deleted `origin_repo()`'s
return type) and `CommandProbe` from the `use super::{run_capped,
run_vcs_text, scrub_environment, CommandProbe, StatusOrLog};` list (`:421`,
used only by the three deleted tests) — narrowing that line to `use
super::{run_capped, run_vcs_text, scrub_environment, StatusOrLog};`. These
are a second, deeper scope from the top-level imports trimmed below, easy
to miss for the same reason the top-level ones were missed in the previous
revision of this plan: the compiler will catch the omission (`unused_imports`
denies the build under this workspace's `warnings = "deny"` policy), but
getting the list right here avoids the wasted iteration.

Also delete `MarkerWalkRoot` and its private `jj_repository_root` helper
(`:29-61`): once change #3 below reworks `tests/library.rs`'s parity assertions,
`MarkerWalkRoot` has no remaining caller anywhere in the crate — its only
uses today are the dual-adapter comparisons in `detection.rs` and
`library.rs`, both collapsed by this phase. (`InProcessProbe` has its own,
separate `jj_repository_root` in `library.rs`, so this deletion doesn't
touch the surviving implementation.) Keep `scrub_environment`,
`run_capped`/`wait_capped` (shared with 0198's `run_vcs_text`), and every
test exercising them directly via generic shell stand-ins (`:442-568`).

Trim the module's top-level imports (`:12-22`) to what the surviving code
(`status`, `log`, `run_vcs_text`, `scrub_environment`, `run_capped`,
`wait_capped`) actually uses — this workspace's `warnings = "deny"` lint
policy (`cli/Cargo.toml`) turns leftover imports into a build failure, not
a lint warning:
- `use std::fs;` — delete; only the deleted `jj_repository_root` used it.
- `use std::path::{Path, PathBuf};` — narrow to `use std::path::Path;`;
  `PathBuf` was only returned by the deleted `MarkerWalkRoot`/
  `jj_repository_root` code.
- `use vcs::origin_remote::OriginRemote;` — delete; only `CommandProbe`'s
  deleted impl used it.
- `use vcs::{RepoRoot, VcsKind, VcsProbe};` — narrow to
  `use vcs::VcsKind;`; `status`/`log` still take a `VcsKind` parameter,
  but `RepoRoot`/`VcsProbe` were only implemented by the deleted structs.
- `use crate::markers::{carries_any_marker, marker_kind, walk_up};` —
  delete entirely; all three were used only by `MarkerWalkRoot::discover`
  and `CommandProbe::kind`, both deleted.

Also update the module-level doc comment (`:1-10`), which currently
describes this file as "a marker walk for the root, and the VCS binaries
themselves for the working-copy revision" — no longer accurate once the
marker walk (`MarkerWalkRoot`) is gone. Retitle it to describe what
remains: the `status`/`log` subprocess text retrieval 0198 owns, no longer
a `RepoRoot`/`VcsProbe` implementation.

`run_capped`'s own doc comment (`:285`) reads "Unlike
[`CommandProbe::revision`], empty output is not itself treated as a
failure here..." — an intra-doc link to a symbol this change deletes.
`run_capped` itself survives, so this comment isn't removed by any
deletion range above; rewrite the comparison to describe the general
failure-folding semantics it's contrasting against rather than naming the
now-deleted `CommandProbe::revision`.

Finish the crate-level doc comment update Phase 2 started
(`cli/vcs-adapters/src/lib.rs:1-10`): its sentence "What both agree on —
the ancestor walk and the marker reading — lives in a third, private
module that each delegates *to*" describes `subprocess`/`library` as two
peer delegators into `markers`; once `MarkerWalkRoot` is deleted here,
`library`/`InProcessProbe` is the only remaining delegator, so that framing
no longer holds either.

#### 2. Collapse the dual-adapter comparison

**File**: `cli/vcs-adapters/tests/detection.rs`
**Changes**: Remove the `CommandProbe`/`MarkerWalkRoot` imports, the
`facts`/`library_facts` pairing, and `assert_implementations_agree`'s
two-probe comparison. Replace every call site with a single helper — e.g.
rename `library_facts` to `facts` — that calls `vcs::facts(&x,
&InProcessProbe, &InProcessProbe)` directly, and update each of the seven
shape-specific tests to assert against that single result rather than
comparing two. "Directly" means bypassing the file's own private
`facts_via(start, root: &dyn RepoRoot, probe: &dyn VcsProbe)` indirection,
not just the deleted `facts`/`CommandProbe` pairing: `facts_via` exists
solely to let the old `facts`/`library_facts` pair construct two
differently-probed calls for `assert_implementations_agree` to compare, and
has no other caller. Once that comparison is gone, delete `facts_via`
itself along with the `use vcs::RepoRoot;`/`use vcs::VcsProbe;` imports it
required — leaving it in place, unreachable from a single-probe helper,
would orphan it the same way `subprocess.rs`'s `origin_repo()` was
orphaned by change #1 above until caught.

#### 3. Rework `tests/library.rs`'s parity assertions

**File**: `cli/vcs-adapters/tests/library.rs`
**Changes**: Remove the `vcs_adapters::subprocess::CommandProbe`/
`MarkerWalkRoot` imports. Delete `assert_parity` outright rather than
reducing it: its three assertions (`kind`, `repository_root`, `revision`)
are each a comparison against the now-removed subprocess implementations,
so a "single-implementation" version of the same assertion
(`InProcessProbe.kind(root) == InProcessProbe.kind(root)`) would be
tautological.

Deleting `assert_parity`'s `revision` comparison removes the only place in
the crate that oracle-verifies `InProcessProbe`'s git-side revision against
the real `git` binary — the jj side keeps its own oracle
(`jj_revision_oracle`, used by three other tests in this file), so losing
the git side without replacement would be a new asymmetry, not a symmetric
simplification, and would leave `detection.rs`'s `is_full_revision_id` (a
40-hex-character format check, not a value check) as the only thing
standing between a git-side mutation and a passing test suite. Close this
by adding a `git_revision_oracle` helper mirroring the existing
`jj_revision_oracle` (`git rev-parse HEAD` via the file's own `run`
helper), and use it in the renamed git-kind test below.

The five call sites
(`a_plain_git_repository_agrees_with_the_subprocess_pair`,
`a_commitless_repository_agrees_and_reports_no_revision`,
`a_colocated_repository_agrees_and_is_driven_as_jj`,
`a_secondary_workspace_resolves_to_the_repository_it_shares`,
`a_main_workspace_resolves_to_itself`) already carry their own
independently meaningful `kind`/`repository_root` assertions ahead of the
`assert_parity(&root)` call (e.g. `VcsKind::Jj` for the colocated case), so
nothing of value is lost there by dropping the call along with the helper.
Rename the three whose names describe the deleted comparison
(`a_plain_git_repository_agrees_with_the_subprocess_pair`,
`a_commitless_repository_agrees_and_reports_no_revision`,
`a_colocated_repository_agrees_and_is_driven_as_jj`) to describe what they
assert standalone instead — e.g. `a_plain_git_repository_reports_git_kind`
— matching how the other two in the group are already named around their
own standalone assertion. When renaming
`a_colocated_repository_agrees_and_is_driven_as_jj`, don't drop straight to
`a_colocated_repository_is_driven_as_jj`: `cli/vcs-adapters/tests/detection.rs`
already has a different test by that exact name, and a same-named test in a
sibling file is confusing to grep for even though it's not a compile
conflict — keep a `library`-scoped qualifier instead, e.g.
`a_colocated_checkout_is_driven_as_jj_in_process`, matching this file's own
`a_colocated_checkout_roots_at_its_own_markers` naming a few lines above
it. Add the new `git_revision_oracle` assertion to
the renamed `a_plain_git_repository_reports_git_kind` test specifically
(`assert_eq!(InProcessProbe.revision(&root, VcsKind::Git),
Some(git_revision_oracle(&root)?))`), so git-side revision correctness
keeps an independent oracle check rather than only a kind check. Update or
remove the `// --- Parity with the subprocess pair ---` section-divider
comment above this group, since the tests beneath it no longer compare
against a subprocess pair.

Rewrite `an_unsnapshotted_edit_is_the_one_documented_divergence` to
snapshot via the real `jj` binary directly (mirroring the file's own
`jj_revision_oracle` helper, e.g. `run("jj", &["log", "-r", "@", ...])`)
rather than through `CommandProbe::new().revision(...)`, so this file keeps
its role of proving the documented divergence — the in-process route does
not snapshot; asking the real `jj` binary does — after `CommandProbe` is
gone.

Retitle the module doc comment: "parity with `CommandProbe` on every port
method" no longer describes what the file proves once there is only one
implementation. The file has three distinct test groups after this rework,
not two — describe all three rather than compressing them: (1) the
boundary rule (`the_boundary_is_the_nearest_marker_not_an_ancestor` and its
neighbours, unchanged by this phase); (2) `InProcessProbe`'s standalone
`kind`/`repository_root` behaviour across real checkout shapes (the
renamed former-parity group — fixed-value checks, not oracle comparisons,
except where noted next); (3) revision-oracle agreement against the real
`jj` and `git` binaries directly (`jj_revision_oracle`, used by the
existing jj-route tests, and the new `git_revision_oracle`, used by
`a_plain_git_repository_reports_git_kind`). Don't fold (2) into (3) — most
of group (2)'s tests assert a fixed expected value, not an oracle
comparison; only the one test that also carries the new
`git_revision_oracle` assertion belongs to both groups.

### Success Criteria:

#### Automated Verification:

- [ ] No `CommandProbe` reference remains: `grep -r CommandProbe cli/
      --include=*.rs` returns no matches
- [ ] No `MarkerWalkRoot` reference remains: `grep -r MarkerWalkRoot cli/
      --include=*.rs` returns no matches
- [ ] No `Command::new` for `jj`/`git` remains in `vcs-adapters`'s non-test
      code serving `facts`: manual review of `cli/vcs-adapters/src/*.rs`
      confirms `run_vcs_text` (0198's path) is the only surviving
      `std::process::Command` use
- [ ] `cargo test -p vcs-adapters --features bash-parity` passes, including
      the reworked `tests/library.rs` and `tests/detection.rs`
- [ ] `cli` component check passes: `mise run cli:check`
- [ ] `mise run check` passes
- [ ] `mise run` passes end to end

#### Manual Verification:

- [ ] `an_unsnapshotted_edit_is_the_one_documented_divergence`'s rewritten
      form still fails if the "does not snapshot" behaviour regresses (spot
      check by temporarily reverting the assertion's expected value)

---

## Phase 4: Re-run the MPL-2.0 licence check

### Overview

`cli/deny.toml`'s `uluru` exception was recorded conditionally on
`vcs-adapters` being unreachable from the visualiser's call graph, verified
at the time (per 0188) only against `accelerator-visualiser`, the launcher,
and `accelerator-verify`. But `cli/vcs-cli`, `cli/collaboration-cli`,
`cli/migrate-adapters`, and `cli/work-adapters` each already construct
`InProcessProbe` directly and call it unconditionally today, independent of
this plan — dead-code elimination cannot remove directly-called code, so
their shipped, dispatched binaries (`accelerator-vcs`,
`accelerator-collaboration`, `accelerator-migrate`, `accelerator-work`)
very likely already carry the closure. This phase's scope is therefore
every `DISPATCHED_SUBBINARIES` token (`tasks/shared/paths.py:29-36`) — all
six: `visualiser`, `vcs`, `work`, `corpus`, `collaboration`, `migrate` — not
a hand-picked subset. Enumerate from that list directly rather than
re-deriving it by memory: a prior pass at this same broadening missed
`migrate` by doing exactly that. Records whatever the build actually shows
for each; the point is confirming with evidence, not assuming any outcome,
including for the four binaries already suspected to carry it.

### Changes Required:

#### 1. Build and inspect unstripped release binaries

**Procedure**: Build all six dispatched sub-binaries —
`accelerator-visualiser`, `accelerator-vcs`, `accelerator-work`,
`accelerator-corpus`, `accelerator-collaboration`, `accelerator-migrate` —
with `cargo build --release --config profile.release.strip=false` (the
workspace's `[profile.release]` sets `strip = true` by default, so a plain
`cargo build --release` would still strip; the override is needed to
inspect debug-adjacent build artefacts, though the two grep targets below
are `.rodata` string literals that `strip` does not remove either way).
Grep each binary for the two literals that indicate the `gix`/`jj-lib`
closure is present: `extensions.objectFormat` (gix) and `There is no
Jujutsu repo` (jj-lib).

#### 2. Update the recorded finding

**File**: `cli/deny.toml`
**Changes**: Update the `uluru` exception's comment (`:67-81`) with the
actual current finding for every binary checked above, not only the
visualiser. If `accelerator-vcs`, `accelerator-collaboration`,
`accelerator-migrate`, or `accelerator-work` are found to already carry the
closure — independent of this plan, since each calls `InProcessProbe`
directly today through a call site unrelated to `facts`/`derive_at` —
record that as a pre-existing finding this item surfaces, and treat
MPL-2.0 §3.2's notice obligation as already live for those binaries
regardless of what this switch does to the visualiser or `corpus`. Only
`corpus`'s reachability is actually caused by this plan's switch — `work`
already reaches the closure through `VcsBackedIdentityProbe`'s author
resolution, not through the `derive_at` path this plan touches. For each
binary where the closure is unreachable, record that explicitly and note
the trigger condition remains live for whenever a server or other call
site is added. Where the closure is reachable (already, or newly via
`corpus`), flag that a third-party attribution artefact is required for
`_release_uploads()` (`tasks/github.py:231-248`) and its
`test_workflows.py` coverage assertion — as a follow-up, since building
that artefact generation is a separate, larger piece of work than this
item's stated scope of re-running the check.

### Success Criteria:

#### Automated Verification:

- [ ] `cargo build --release --config profile.release.strip=false -p
      accelerator-visualiser -p accelerator-vcs -p accelerator-work -p
      accelerator-corpus -p accelerator-collaboration -p
      accelerator-migrate` succeeds
- [ ] `mise run build-system:check` passes (if `deny.toml`'s comment is the
      only change, this only needs to stay green, not gain new coverage)
- [ ] `mise run check` passes
- [ ] `mise run` passes end to end

#### Manual Verification:

- [ ] The grep-for-literals procedure was actually run against each of the
      six built binaries, and its outcome (present/absent for each
      literal, per binary) is recorded in the updated comment
- [ ] If `accelerator-vcs`, `accelerator-collaboration`,
      `accelerator-migrate`, or `accelerator-work` are found to already
      carry the closure, each is called out explicitly as a pre-existing
      finding rather than folded silently into a comment scoped to the
      visualiser or attributed incorrectly to this plan's switch
- [ ] If any binary's closure is found reachable, a follow-up work item is
      filed for the attribution artefact rather than attempting it inline
      here

---

## Testing Strategy

### Unit Tests:

- `VcsProbe::revision`'s sha256 folding behaviour is partially covered by
  `cli/vcs-adapters/tests/queries.rs`'s
  `an_unsupported_object_format_fails_rather_than_misreads` and
  `cli/vcs-adapters/tests/classify.rs`, but neither calls `revision` itself
  — Phase 1 adds the missing case so the doc comment's claim is backed by a
  test, not only by adjacent coverage of `is_bare`/`worktree`/
  `dual_roots`/`classify`.
- `InProcessProbe`'s existing unit tests (`library.rs:1009-1107`) already
  cover the four cases `CommandProbe`'s deleted tests covered, plus one they
  didn't (`an_unreadable_repository_is_an_error`) — no new unit test needed
  for the deletion itself.
- The containment-bound decision's "failure distinguishes itself from
  absence" claim has jj-side proof today
  (`an_unreadable_checkout_state_reports_absence_rather_than_a_wrong_commit`
  in `tests/library.rs`) but no git-side equivalent — Phase 1 adds a
  malformed-git-ref case so both dispatch paths this switch widens the
  reach of are backed.

### Integration Tests:

- Phase 2's new `zero_spawn.rs` tests are the key end-to-end proof for this
  item: the corpus metadata-read path, driven through the real
  `VcsBackedRepoFactsProbe`/`derive_at` composition, spawns no subprocess —
  for both a git and a jj fixture, since `facts` dispatches differently per
  `VcsKind`.
- Phase 3's collapsed `detection.rs` continues proving `InProcessProbe`
  agrees with itself across all seven checkout shapes (no longer "agrees
  with `CommandProbe`", since there's only one implementation left).
- Phase 3's reworked `tests/library.rs` keeps proving the documented
  snapshot-on-read divergence, now against the real `jj` binary directly
  rather than through `CommandProbe`.

### Manual Testing Steps:

1. Run `corpus metadata derive` against a real jj repository with
   unsnapshotted working-copy edits present; confirm `Current Revision:`
   names the last recorded commit, not a freshly snapshotted one (the one
   known, accepted behavioural difference from before this switch).
2. Run `corpus metadata derive` against a bare repository; confirm it still
   reports no facts.
3. Run `vcs detect`/`vcs guard` (the hook path) after the switch; confirm
   no regression, since these paths already used `InProcessProbe` before
   this item.

## Performance Considerations

None expected — `InProcessProbe` avoids process-spawn overhead entirely
compared to `CommandProbe`, so the metadata-read path should be faster, not
slower.

## Migration Notes

No data migration. The one user-visible behavioural change — loss of the
jj snapshot-on-read side effect on `corpus metadata derive` — is stdout-only
for `cli/corpus-adapters`' own write paths (per Phase 1's recorded
confirmation), but skills that copy `corpus metadata derive`'s printed
`Current Revision:` line into committed `meta/` frontmatter (`create-plan`,
`research-issue`, `create-note`, and others) inherit the same staleness
window: an artefact authored with unsnapshotted working-copy edits present
records the last recorded operation's commit rather than a freshly
snapshotted one. Accepted as a best-effort provenance degradation, not a
correctness regression — nothing downstream treats these fields as exact —
but it is a real, if narrow, change to persisted data, not only to stdout.

## References

- Original work item:
  `meta/work/0185-converge-corpus-adapters-on-library-backed-vcs.md`
- Related research:
  `meta/research/codebase/2026-08-10-0185-converge-corpus-adapters-library-backed-vcs.md`
- Prior review (APPROVE):
  `meta/reviews/work/0185-converge-corpus-adapters-on-library-backed-vcs-review-1.md`
- Adapter delivery: `meta/work/0188-library-backed-vcs-adapter.md`
- Boundary rationale:
  `meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md`
