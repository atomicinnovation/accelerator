---
type: "plan"
id: "2026-08-31-0198-vcs-agnostic-status-log-renderer"
title: "VCS-agnostic status/log renderer Implementation Plan"
date: "2026-08-31T12:07:44+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0198"
parent: "work-item:0198"
derived_from: ["codebase-research:2026-08-31-0198-vcs-agnostic-status-log-renderer"]
relates_to: ["adr:ADR-0066"]
tags: ["rust", "vcs", "cli", "gix", "jj-lib", "status", "log"]
revision: "306ca7ccd78c8be8d5234ce0099bea32ce4c984b"
repository: "accelerator"
last_updated: "2026-08-31T20:10:32+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# VCS-agnostic status/log renderer Implementation Plan

## Overview

Replace the last two `vcs` subcommands that spawn a process — `status` and
`log` — with an in-process renderer over `gix` (git) and `jj-lib` (jj), feeding
the single VCS-agnostic output format fixed by ADR-0066. The two subprocess
renderers in `cli/vcs-adapters/src/subprocess.rs` become a backend-neutral model
plus a pure renderer in `cli/vcs/`, populated by two adapters in
`cli/vcs-adapters/src/library/`, with the never-fail fallback preserved. Once
both backends are library-backed, `vcs_adapters::subprocess` is deleted and a
zero-spawn assertion proves neither `status` nor `log` launches a child.

The behavioural migration lands atomically (both backends flip together), so
`main` never renders one backend in ADR format and the other in native text.

## Current State Analysis

Backend selection is already in-process; only the text rendering shells out.
`cli/vcs-cli/src/status.rs:14-22` and `log.rs:14-22` run the identical dance —
`InProcessProbe.discover(start)` → `probe.kind(root)` → derive `dir` — then call
`subprocess::status`/`subprocess::log`. The swap point is one line each
(`status.rs:21`, `log.rs:21`); the discover/kind/dir derivation above stays.

`run_vcs_text` (`subprocess.rs:66-99`) runs exactly four commands under a
scrubbed environment and a 10-second cap, folding any failure to a
backend-specific `(... unavailable)` literal (`subprocess.rs:104-113`). The two
public entry points are `#[must_use] -> String` and never return `Err`.

The library adapter already carries most of the data path:

| Capability | State today | Location |
|---|---|---|
| git working-copy status (paths) | Reusable; change-kind discarded | `library/dirty_paths.rs:34-62` |
| jj snapshot + `TreeDiffIterator` | Reusable; change-kind + conflict dropped | `library/dirty_paths.rs:75-160` |
| jj settings-loaded workspace (Route B) | Reusable | `dirty_paths.rs:75-116` |
| repo-open helpers, error type, `is_unborn_head` | Reusable | `library.rs:836-926` |
| git recent-commit revwalk | Net-new (only `head_commit()` today) | `library.rs:582-584` |
| git change-kind classification | Net-new (status item read for path only) | `dirty_paths.rs:59` |
| jj recent-change first-parent walk | Net-new | — |

The `match kind` dispatch shape already exists (`dirty_paths(root, kind)`,
`library.rs:391-401`); the new renderer follows it.

## Desired End State

`vcs status` and `vcs log` compute their output in-process for both backends and
render it in the ADR-0066 format. `vcs_adapters::subprocess` no longer exists.
Under `git`/`jj` shadowed at every reachable absolute path, both subcommands
match their ADR-format goldens and launch no child process. The never-fail
contract holds under fault injection, and `ACCELERATOR_LOG` surfaces a
`gix`/`jj-lib` token on the fallback path. `mise run` exits 0 end to end.

Verify: `mise run` green; `mise run test:integration:zero-spawn:strong` green
(CI, Linux); the `/commit` skill still injects usable orientation text.

### Key Discoveries

- ADR-0066 is **accepted** and fixes the format
  (`meta/decisions/ADR-0066-vcs-agnostic-status-log-output-format.md`) — AC1 is
  satisfied ahead of this plan.
- `blob-diff` is **already present** in the effective `gix` feature graph via
  jj-lib unification (`tests/integration/deny/test_vcs_library_graph.py:59-66`);
  declaring it on `vcs-adapters` is hygiene, adds no crates, leaves the licence
  closure untouched.
- `kernel::logging::init()` (`cli/kernel/src/logging.rs:26-36`) is **not** called
  by `cli/vcs-cli/src/main.rs`; the launcher's init dies at the `exec()` image
  swap, so every fallback `warn!` is discarded today (AC6 gap).
- The zero-spawn harness (`cli/vcs-test-support/src/stubs.rs`) and the strong CI
  job (`.github/workflows/main.yml:317-371`) exist but cover only the
  detection/facts path, not status/log.
- jj Route B snapshots but drops the lock without `finish()`
  (`dirty_paths.rs:144`), so it writes nothing and reports state **as of the last
  operation** — ADR-0066 line 148 records this; unlike the `jj` binary it does
  not mutate the repo.

### Resolved research questions

- **jj first-parent walk**: a manual peel of `parent_ids().first()` from the
  working-copy commit's first parent, stopping at the root commit, taking five —
  no revset engine, `@`/root excluded by construction.
- **sha256 git repos**: `gix` 0.85 returns `Err`, which folds to the
  `(status|log unavailable)` fallback like any adapter failure (the 0185 policy).
  Rather than leave this as a one-off spike observation, a dedicated `sha256-git`
  state pins the fallback golden (Phase 1 §7), so a future `gix` that partially
  supports sha256 fails the test instead of silently changing behaviour.
- **`--stat` counts**: ADR-0066 carries change-type + path, no counts.
  Change-kind derives from the git status item and the jj tree diff's
  before/after presence — no blob-content diff is needed.

### API and behaviour verified by spike (2026-08-31)

A throwaway probe built real git and jj merge-conflict repositories and read
them through the pinned `gix` 0.85 / `jj-lib` 0.43 in the calling process. It
settled every deferred API and one behavioural surprise:

- **git change-kind + conflict come from one read.** `gix::status`'s
  `Item::summary()` yields `Summary::{Conflict, Removed, TypeChange, Modified,
  Added (untracked), Renamed, Copied}`; a merge-conflicted file reports
  `Some(Conflict)`. No `repository.index()` stage-walk is needed — conflict is a
  first-class status item, resolving the plan's earlier either/or.
- **git branch** reads from `repository.head_name()` (`Some(refs/heads/main)`;
  `None` when detached).
- ⚠️ **jj conflict is NOT a working-copy change.** For a merge with a conflicting
  file, `TreeDiffIterator(parent_tree, new_tree)` yields **zero entries** — the
  same conflict sits in both trees and cancels — while `jj status` itself reports
  "no changes" plus a separate "unresolved conflicts" list. Deriving conflict
  from the change diff (the plan's original Phase 1 §3) would miss it entirely.
  The correct read is `MergedTree::conflicts()` on the snapshot tree, unioned
  into the report as `conflicted` (see Phase 1 §3).
- **jj log ids** — `ChangeId::reverse_hex()` returns the `z-k` alphabet
  (`outlkvpxvonk…`), matching the `jj_change_id` mask; `Store::get_commit` is
  **sync** (no `block_on`); `root_commit_id`/`parent_ids`/`change_id`/
  `description` match the snippet.
- **git log ids** — `rev_walk(...).first_parent_only().all()`, `Info.id`,
  `Info.object()`, `to_hex_with_len(12)`, and `commit.message()?.summary()` all
  exist in gix 0.85; the log renders a fixed-width 12-hex id (see Phase 1 §2).

### jj-lib coupling and upgrade cost (resolves the work item Open Question)

The work item's Open Question — whether pre-1.0 `jj-lib`'s per-release API churn
is an acceptable maintenance cost — is resolved here as **accept full migration**,
matching ADR-0066 and the atomic both-backends flip; the git-only re-scope
fallback is not taken. The standing cost is concentrated: the jj adapter's
`jj-lib`/`gix` touch points a version bump must re-verify are `MergedTree::
conflicts()`, `Store::get_commit`, `ChangeId::reverse_hex()`, `parent_ids()`/
`root_commit_id()`/`description()`, the `working_copy_diff` snapshot, and on the
git side `Item::summary()`/`Summary` variants, `head_name()`, `rev_walk().
first_parent_only().all()`, `to_hex_with_len(12)`, and `message().summary()`.
Routing all jj *snapshot* access through the `working_copy_diff` seam
concentrates the status-path coupling in one place, but the jj **log** walk
(`get_commit`, `reverse_hex()`, `parent_ids()`, `root_commit_id()`,
`description()`) sits in `jj_log`, outside that seam — so a `jj-lib` bump has two
primary sites to re-verify, the snapshot seam and the log walk, not one. The
multi-way pin (jj-lib, gix, toolchain, jj CLI) moves together per 0188, so a bump
is a deliberate, tested step, not a silent drift.

## What We're NOT Doing

- Reproducing native `jj`/`git` CLI text (byte-parity) — an explicit ADR non-goal.
- Carrying parent commit or ahead/behind in status, or author/date/graph in log.
- jj-native richness (operation log, change evolution, change-ids as data) — a
  future consumer gets a separate structured subcommand, per ADR "Negative".
- Adding any `gix` crate or re-opening the licence closure — `blob-diff` is
  already present; `default-features = false` and the `gix-credentials` ban hold.
- Changing `--fail-safe` semantics — it stays a launcher-only flag, orthogonal to
  the adapter-failure fallback.
- Snapshotting/persisting the jj working copy on `status` — it reports as of the
  last operation and persists no operation or working-copy state (the snapshot
  does write content-addressed objects to the store; see Performance
  Considerations).
- Adding a branch/bookmark mask to the shared `masks.toml` (it would weaken the
  per-state goldens); the parity harness normalises the branch value locally.

## Implementation Approach

Three phases, sequential — each builds on the last, and each is green:

1. **The renderer, both backends, and conflict** — the neutral model, the pure
   renderer and the `VcsReporter` port in `vcs`, the git and jj adapters
   (including the conflict path and its fixtures and assertion), the infallible
   boundary over the port + AC6 init/token, all goldens regenerated to the ADR
   format. This is the atomic behavioural flip.
2. **Cross-backend parity and content goldens** — the git-vs-jj parity harness
   and the content/negative goldens. Test-only hardening.
3. **Delete subprocess and extend zero-spawn** — remove the module, widen the
   `std::process` deny crate-wide, and prove status/log spawn nothing.

Phase 1 lands the shared state builder in `vcs-test-support` (where the states
are born), so Phase 2 (parity) and Phase 3 (delete + zero-spawn) each consume it
independently — Phase 3 does not transitively depend on Phase 2, and the two need
not land in a fixed order relative to each other.

TDD throughout: the pure renderer is unit-tested before the adapters; a failing
golden, parity, conflict, or zero-spawn assertion precedes each behaviour.

The model, the pure renderer, and the `VcsReporter` port sit in `vcs` (value
types and pure rendering, no I/O). `InProcessProbe` implements the port in
`vcs-adapters` behind the `match kind` dispatch and the I/O. The never-fail
`Result -> String` fallback in `vcs-cli` calls the port through a `&dyn
VcsReporter` seam, so a test-only failing reporter drives the fallback (AC6)
without touching file permissions.

```text
vcs-cli status::run / log::run   &dyn VcsReporter -> String, fallback + AC6 warn
        |
        v
vcs::VcsReporter (impl by vcs_adapters::library::InProcessProbe)     match kind
        |                                   |
        v                                   v
   git adapter (gix)                   jj adapter (jj-lib)
        \                                   /
         v                                 v
        vcs::status::render / vcs::log::render   (pure)
```

---

## Phase 1: The renderer, both backends, and conflict

### Overview

Introduce the backend-neutral model, the pure renderer, and the `VcsReporter`
port in `vcs`; both adapters (including the conflict path and its fixtures and
assertion) in `vcs-adapters`; and the infallible boundary over the port +
logging init in `vcs-cli`. Regenerate all status/log goldens to the ADR format.
After this phase both backends are library-backed and `subprocess` is unused
(deleted in Phase 3).

### Changes Required

#### 1. Neutral model and pure renderer

**File**: `cli/vcs/src/status.rs` (new), `cli/vcs/src/log.rs` (new), registered
in `cli/vcs/src/lib.rs` as `pub mod status; pub mod log;`.

`status.rs` holds the value types and the render function. The renderer owns the
sort and the empty/conflict-summary logic so neither adapter re-implements it.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Untracked,
    Conflicted,
}

impl ChangeType {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Untracked => "untracked",
            Self::Conflicted => "conflicted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub change_type: ChangeType,
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusReport {
    pub branch: Vec<String>,
    pub changes: Vec<FileChange>,
}

#[must_use]
pub fn render(report: &StatusReport) -> String {
    let branch = if report.branch.is_empty() {
        "(none)".to_owned()
    } else {
        report.branch.join(", ")
    };
    if report.changes.is_empty() {
        return format!("Branch: {branch}\nNo changes");
    }

    let conflicted = report
        .changes
        .iter()
        .filter(|change| change.change_type == ChangeType::Conflicted)
        .count();
    let summary = if conflicted > 0 {
        format!("{} changed, {conflicted} conflicted", report.changes.len())
    } else {
        format!("{} changed", report.changes.len())
    };

    let mut ordered: Vec<&FileChange> = report.changes.iter().collect();
    ordered.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    let lines: String = ordered
        .iter()
        .map(|change| format!("  {}  {}", change.change_type.label(), change.path))
        .collect::<Vec<_>>()
        .join("\n");

    format!("Branch: {branch}\n{summary}\n{lines}")
}
```

`log.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub short_id: String,
    pub subject: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogReport {
    pub entries: Vec<LogEntry>,
}

#[must_use]
pub fn render(report: &LogReport) -> String {
    if report.entries.is_empty() {
        return "No commits".to_owned();
    }
    report
        .entries
        .iter()
        .map(|entry| {
            let subject = if entry.subject.is_empty() {
                "(no description)"
            } else {
                entry.subject.as_str()
            };
            format!("{} {subject}", entry.short_id)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

Unit tests (red first) cover: header with a branch, `(none)` when empty, `No
changes`, `<N> changed`, `<N> changed, <K> conflicted`, byte-order sort, the
five labels, `No commits`, `(no description)`, and the five-entry list.

#### 1a. The `VcsReporter` port

**File**: `cli/vcs/src/lib.rs` — a new port beside `RepoRoot`/`VcsProbe`, so the
`vcs-cli` boundary depends on an abstraction and a test-only failing reporter
satisfies AC6 without file permissions. It touches the `public-api` baseline.

```rust
pub trait VcsReporter {
    fn status_report(&self, root: &Path, kind: VcsKind) -> Result<status::StatusReport, kernel::Error>;
    fn log_report(&self, root: &Path, kind: VcsKind) -> Result<log::LogReport, kernel::Error>;
}
```

The port returns `kernel::Error`, the sanctioned port error (`vcs/Cargo.toml`
documents `kernel::Error` for exactly a fallible port that needs the shared
diagnostic; `vcs` already depends on `kernel`, not on `gix`/`jj-lib`), so no new
public error type is introduced. The adapter's internal `Error` already
converts via `impl From<Error> for kernel::Error` (`library.rs:429`) — the
orphan rule does not bite, since that impl (foreign `From` for local-to-adapter
`Error`, into `kernel::Error`) compiles today — so each adapter method
propagates with `.map_err(Into::into)` rather than a hand-written string
collapse. The sibling `Option`-returning ports (`VcsProbe`, `UserIdentityProbe`)
fold failure to `None`, but `VcsReporter` needs the `Err` arm to drive the
fallback and the AC6 diagnostic. The AC6 token (`gix`/`jj-lib`) is derived from
`kind`, and the internal `Error`'s `Display` already names the failing operation
and repository path, so `.map_err(Into::into)` over the shared `From` is
sufficient. The shared conversion (`Self::Failed(error.to_string())`, consumed by
every other fallible port — `CheckoutProbe`, `ModeProbe`, `OriginRemote`,
`DualRoots`) is **not** modified: `kernel::Error` has no `source()` channel, so
folding a cause chain into it would silently change every sibling port's message.
If surfacing the deeper `gix`/`jj-lib` cause is later wanted, it is added to the
adapter `Error`'s own `Display`, local to this crate, not to the shared `From`.

#### 2. Git adapter

**File**: `cli/vcs-adapters/src/library/status_log.rs` (new), declared `mod
status_log;` in `library.rs`. `InProcessProbe` implements `vcs::VcsReporter`,
each method dispatching on `kind` and propagating the internal `Error` as
`kernel::Error` via the existing `From` impl:

```rust
impl vcs::VcsReporter for InProcessProbe {
    fn status_report(&self, root: &Path, kind: VcsKind) -> Result<StatusReport, kernel::Error> {
        match kind {
            VcsKind::Git | VcsKind::None => status_log::git_status(root),
            VcsKind::Jj => status_log::jj_status(root),
        }
        .map_err(Into::into)
    }

    fn log_report(&self, root: &Path, kind: VcsKind) -> Result<LogReport, kernel::Error> {
        match kind {
            VcsKind::Git | VcsKind::None => status_log::git_log(root),
            VcsKind::Jj => status_log::jj_log(root),
        }
        .map_err(Into::into)
    }
}
```

`VcsKind::None` cannot arise with a discovered root (a discovered root always
classifies as `Git` or `Jj`); routing it to `git_status`/`git_log` folds to the
`(… unavailable)` fallback like a no-repo directory, which is the intended
no-repo behaviour rather than a fabricated empty report. This is a **deliberate**
divergence from `dirty_paths`, which maps `VcsKind::None → Ok(Vec::new())`: an
unavailable read is more honest than a fabricated clean result for the
orientation text. The divergence is unreachable in practice (no discovered root is
`None`) but is noted so the difference reads as intentional, not incidental.

`git_status` reads the branch from `repository.head_name()` (a branch → its
short name; detached → `(none)`), and the change set from the same
`repository.status(...)` platform `dirty_paths` already uses. Each item's
change-type and conflict come from `Item::summary()` — verified against gix 0.85
(spike, 2026-08-31): the platform yields `Summary::{Conflict, Removed,
TypeChange, Modified, Added, Renamed, Copied}`, so **no `repository.index()`
stage-walk is needed** and conflict is a first-class item.

A pure `classify` function maps one status item to the `FileChange`(s) it
produces, so the branch-heavy table is unit-tested in isolation, off any real
repository. Its input is a small neutral value the adapter constructs from each
gix item (the `Summary` plus an `untracked` discriminant for the dirwalk source),
not the raw `gix::status::Item`, so a unit test can build every case without a
repository. It returns `Vec<FileChange>` (0, 1, or 2 entries) so a rename/copy
that yields two paths is expressible:

- an untracked file — the **dirwalk item variant**, carried by the discriminant,
  not a `Summary` value — → one `Untracked`. A staged add is a tree→index
  `Summary::Added` → one `Added`; the two share the `Added` summary and are told
  apart by the source discriminant, not by `summary()` (keying on `summary()`
  alone would render untracked as `added` and break the AC3 divergence).
- `Summary::Removed` → one `Deleted`
- `Summary::{Modified, TypeChange}` → one `Modified`
- `Summary::Conflict` → one `Conflicted`
- `Summary::Renamed` → two: `Deleted` (old path) + `Added` (new path)
- `Summary::Copied` → one `Added` (new path) only; the copy source is unchanged
  and present, so it is **not** emitted as `Deleted` (git status itself reports
  only the new path for a copy)

A path can carry more than one item — a tree→index (staged) and an index→worktree
item, or (after `git rm --cached` with the file left on disk) a staged `Removed`
plus a dirwalk `Untracked`. A second pure function `resolve(items) ->
Vec<FileChange>` beside `classify` dedups them to one entry per path (counted once
in `N`) by **commit-accuracy**, not a fixed `ChangeType` rank: conflict overrides;
otherwise the **staged (tree→index) item's type wins**, because that is what the
commit will contain; otherwise the worktree/dirwalk item's type stands. So a
staged-add + worktree-modify renders `added`, a staged-add + worktree-delete
renders `added` (the index copy is what commits — a fixed `ChangeType` order would
mislabel it `deleted`), a staged-modify + worktree-delete renders `modified`, a
`git rm --cached` staged-`Removed` + on-disk-`Untracked` renders `deleted`, and a
plain untracked file (no staged item) renders `untracked`. Unit tests over
`resolve` pin each: AM→added, AD→added, MD→modified, staged-delete+untracked→
deleted, untracked-only→untracked, and conflict-overrides.

`git_log` reuses `super::is_unborn_head` (private, reachable from a `library/`
submodule via `super::`) for the no-commits case and walks first-parent ancestry
from `HEAD`, rendering a **fixed-width 12-hex** id (`to_hex_with_len(12)`, not
`shorten()`, so it is deterministic and always inside the `hex_object_id` mask):

```rust
pub(super) fn git_log(root: &Path) -> Result<LogReport, Error> {
    let repository = gix::open(root).map_err(git_err(root))?;
    let head = match repository.head_commit() {
        Ok(commit) => commit,
        Err(error) if super::is_unborn_head(&error) => {
            return Ok(LogReport::default())
        }
        Err(error) => return Err(git_err(root)(error)),
    };
    let mut entries = Vec::new();
    for info in repository
        .rev_walk([head.id])
        .first_parent_only()
        .all()
        .map_err(git_err(root))?
        .take(5)
    {
        let info = info.map_err(git_err(root))?;
        let commit = info.object().map_err(git_err(root))?;
        let message = commit.message().map_err(git_err(root))?;
        entries.push(LogEntry {
            short_id: info.id.to_hex_with_len(12).to_string(),
            subject: message.summary().to_string(),
        });
    }
    Ok(LogReport { entries })
}
```

Each `?` maps a distinct `gix` error type into `Error::Git { path, source }`.
Rather than an inline closure per site, a small generic helper carries the
mapping: `fn git_err<E>(root: &Path) -> impl Fn(E) -> Error` where `E:
std::error::Error + Send + Sync + 'static` returns a closure that boxes the
source. It monomorphises per call site, so the differing error types share one
mapping without duplication (the "one shared closure won't compile" concern held
only for a single non-generic closure). `jj_err` is the jj analogue.

#### 3. jj adapter

**File**: `cli/vcs-adapters/src/library/status_log.rs` for `jj_status`/`jj_log`,
plus a new shared `cli/vcs-adapters/src/library/snapshot.rs` for the jj
working-copy snapshot both status/log and `dirty_paths` need. `jj_status` reuses
the snapshot-and-tree-diff already proven in `dirty_paths.rs` for the change list,
and reads conflicts separately from the snapshot tree. To avoid duplicating the
~70-line snapshot path — and to keep the pre-existing `dirty_paths` module from
depending on a status/log-named module — extract it into `snapshot.rs` as
`pub(super) fn working_copy_diff(root) -> Result<Option<WorkingCopySnapshot>, Error>`,
where `WorkingCopySnapshot { changes: Vec<DiffEntry>, tree: MergedTree }` names the
two returns rather than a positional tuple. Both `status_log.rs` and
`dirty_paths.rs` depend on `snapshot.rs`, not on each other. Each `DiffEntry`
carries the path and its before/after presence, where presence encodes the
existing `is_present() && !is_tree()` keep predicate so tree-valued entries
(gitlinks, submodules) stay excluded exactly as `jj_dirty_paths` does today.
Re-point `jj_dirty_paths` at it (using only `changes` — its existing tests are the
safety net, its semantics unchanged); those tests currently have no tree-valued
case, so commit a `jj_dirty_paths` unit case exercising a gitlink/submodule so the
keep predicate is pinned, not assumed. The internal error variant is renamed from
`Error::JjDirtyPaths` to `Error::JjWorkingCopyDiff` to match the now-shared seam.
The `Option` preserves today's no-working-copy-commit behaviour: `jj_dirty_paths`
returns `Ok(Vec::new())` when `get_wc_commit_id(name)` is `None`, so
`working_copy_diff` returns `Ok(None)` there and `jj_status` short-circuits to an
empty `StatusReport`. `jj_log` does not call `working_copy_diff` (it walks
first-parent from the working-copy commit), so it carries its **own** `None` guard
on `get_wc_commit_id(name)`, returning `Ok(LogReport::default())` (`No commits`) —
mirroring the `dirty_paths.rs` guard. `jj_status` maps each change entry:

- before absent, after present → `Added` (jj auto-tracks a new file into `@`; the
  documented git-`untracked` / jj-`added` divergence, handled by the Phase 2
  parity harness)
- before present, after absent → `Deleted`
- before present, after present → `Modified`

⚠️ **Conflict is a separate read, not a change-diff property** (spike,
2026-08-31). A merge-conflicted file appears in *no* tree-diff entry — the same
conflict sits in both `parent_tree` and the snapshot and cancels, exactly as `jj
status` reports "no changes" yet lists unresolved conflicts. `jj_status`
therefore calls `snapshot_tree.conflicts()` (`MergedTree::conflicts()`) and
**unions** each conflicted path into the report as `Conflicted`, deduping against
the change entries (a path both changed and conflicted is listed once, as
`conflicted`). `jj_dirty_paths` keeps ignoring conflicts, so a pure
merge-conflict working copy with no other edits stays "clean" there, unchanged
from today.

Branch: the bookmark(s) on the working-copy commit, byte-sorted; empty → `(none)`
(the common case). `jj_log` loads the settings-loaded workspace (Route B, as
`dirty_paths` does), resolves the working-copy commit, and peels first-parent
(`get_commit` is sync — no `block_on`; errors map to a jj `Error` variant via
`jj_err`). `reverse_hex()` is a fixed 32-char string under jj's 16-byte
change-id invariant, but the render reads it length-safely with `.get(..12)`
rather than a raw slice, so a future reverse-hex width cannot panic:

```rust
let mut current = wc_commit.parent_ids().first().cloned();
let mut entries = Vec::new();
while let Some(id) = current {
    if id == *repo.store().root_commit_id() {
        break;
    }
    let commit = repo.store().get_commit(&id).map_err(jj_err(root))?;
    let change_id = commit.change_id().reverse_hex();
    entries.push(LogEntry {
        short_id: change_id.get(..12).unwrap_or(&change_id).to_owned(),
        subject: commit.description().lines().next().unwrap_or_default().to_owned(),
    });
    if entries.len() == 5 {
        break;
    }
    current = commit.parent_ids().first().cloned();
}
```

`snapshot.rs` constructs `UserSettings` (the jj snapshot needs it), so **it** —
not `status_log.rs` — is added to `_EXEMPT` in `tasks/lint/vcs_settings.py:46-51`,
paired (red first) with a `test_the_snapshot_module_is_individually_exempt` in
`tests/unit/tasks/test_vcs_settings.py`, mirroring the existing 1:1 convention
(`test_the_dirty_paths_snapshot_module_is_individually_exempt`,
`test_the_tracked_module_is_individually_exempt`). Because `working_copy_diff`
moves the `UserSettings`-constructing snapshot out of `dirty_paths.rs` into
`snapshot.rs`, the `dirty_paths.rs` `_EXEMPT` entry and its paired test are pruned
in the same change (it no longer constructs `UserSettings`), so no exemption goes
vacuous. The guard's own module docstring attributes the snapshot exemption to
`dirty_paths.rs`, so move that rationale to `snapshot.rs` in the same edit,
keeping the guard's prose and its `_EXEMPT` set coherent. `status_log.rs` itself
needs no exemption — it calls `working_copy_diff` and constructs no `UserSettings`.

#### 4. Infallible boundary and AC6

**File**: `cli/vcs-cli/src/status.rs`, `cli/vcs-cli/src/log.rs`.

```rust
pub fn run(start: &Path, reporter: &dyn vcs::VcsReporter) -> String {
    let probe = InProcessProbe;
    let root = probe.discover(start);
    let kind = root.as_deref().map_or(VcsKind::Git, |root| probe.kind(root));
    let dir = root.as_deref().unwrap_or(start);
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        reporter.status_report(dir, kind)
    }));
    match outcome {
        Ok(Ok(report)) => vcs::status::render(&report),
        Ok(Err(error)) => {
            warn!(adapter = adapter_token(kind), %error, "could not render status");
            "(status unavailable)".to_owned()
        }
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("panic");
            warn!(adapter = adapter_token(kind), panic = message, "panicked rendering status");
            "(status unavailable)".to_owned()
        }
    }
}
```

The `&dyn VcsReporter` parameter is the AC6 injection seam: `run_status`/
`run_log` in `main.rs` pass `&InProcessProbe`, while the fault-injection test
passes a failing reporter. Discovery and `kind` stay on the concrete
`InProcessProbe` (only the report read is behind the seam).
`adapter_token(kind)` returns `"jj-lib"` for `Jj`, else `"gix"` (a shared helper
in `vcs-cli`). The `warn!` tags this on an `adapter = …` field with the ADR-0066
`gix`/`jj-lib` vocabulary AC6 mandates; the existing library-adapter warnings use
`vcs = "git"/"jj"` (`library.rs`), so the two are deliberately distinct — noted
here so a log consumer knows the status/log fallback is keyed on `adapter`, not
`vcs`. `log::run` mirrors this with `(log unavailable)`; the shared
discover/`kind`/`catch_unwind`/match shell is extracted into one helper
parameterised by the report closure, the `render` function, and the fallback
literal, so `status::run` and `log::run` are thin call sites and the never-fail
logic lives in one place.

**Never-fail covers panic, not just `Err`.** The subprocess boundary being
removed contained a `gix`/`jj-lib` panic on a pathological repository inside a
child process; in-process, a panic would unwind through `main` (which has no
handler) and abort with exit 101, breaking the never-fail contract for a
`/commit`-invoked read. The `catch_unwind(AssertUnwindSafe(…))` folds a cleanly-
unwinding panic to the same `(status|log unavailable)` fallback plus a `warn!`
carrying the downcast panic message, restoring the contract for panics that
unwind cleanly. It does **not** cover a panic raised inside a destructor during
unwinding (Rust aborts on a double panic), nor a wall-clock hang or unbounded read
(a thread cannot be safely interrupted in-process — accepted, see Migration
Notes). `discover`/`kind` stay outside the guard: they are the in-process facts
path 0185 already relies on (`detect`/`guard` call it without a guard), so this
adds no new exposure — but the never-fail claim is scoped to the report read, not
the whole subcommand.

⚠️ **The panic fold depends on `panic = "unwind"`.** `catch_unwind` is a no-op
under `panic = "abort"`; `cli/Cargo.toml`'s size-tuned `[profile.release]` leaves
the default `unwind` today, but the panic-reporter test (Phase 1 §4) runs under the
dev/test profile and would stay green even if release later switched to `abort`.
The shipped-artefact guarantee is enforced by a **static manifest guard** rather
than a release-binary panic test: forcing a real `gix`/`jj-lib` panic
deterministically through the shipped binary is impractical (the degenerate
fixtures yield `Err`, not a panic, and `main` wires the concrete `InProcessProbe`
that the test-only panicking reporter cannot reach), so a small unit/build test
reads `cli/Cargo.toml` and asserts `[profile.release]` does not set `panic =
"abort"`. That is deterministic, fails loudly on the exact regression, and is a
tracked success criterion — with the dependency also recorded at the
`catch_unwind` site and on `[profile.release]`.

**File**: `cli/vcs-cli/src/main.rs` — call `kernel::logging::init()` at the top of
`main` **only when `ACCELERATOR_LOG` is set**, and pass `&InProcessProbe` into
`status::run`/`log::run`. Gating on the env var matters: `logging::init` installs
a default-`INFO` stderr subscriber when `ACCELERATOR_LOG` is unset
(`filter_from_env(None)`), so an unconditional call would make **every** vcs
subcommand — `detect` and the `guard` PreToolUse hook included — emit INFO/WARN to
stderr by default, where today (no `init` after the `exec()` image swap) they are
silent. With the gate, the unset default stays silent and the fallback `warn!`
surfaces only on the `ACCELERATOR_LOG` path, as ADR-0066 specifies. On a malformed
`ACCELERATOR_LOG`, `init` returns `Err`; print to stderr and continue with logging
uninitialised, so status/log still render (never-fail preserved).

#### 5. Feature hygiene and goldens

**File**: `cli/vcs-adapters/Cargo.toml:38` — declare `features = ["status",
"blob-diff"]`, keeping `default-features = false`. `status` is already declared
and `blob-diff` already resolves via jj-lib unification, so neither adds a crate.
Whether the status platform needs `dirwalk` for untracked enumeration is resolved
**before coding**, not deferred: if `dirwalk` (or any other feature) proves
net-new, it is a lockstep edit to `_FEATURES_PRESENT` and, should it pull a
crate, the `_BUILD_SCRIPT_CRATES`/`_PROC_MACRO_CRATES` snapshots in
`tests/integration/deny/test_vcs_library_graph.py`. The resolved feature set is
confirmed against that test (run via `test:integration:deny`), and
`gix-credentials` stays banned by `deny.toml`.

**Files**: `cli/vcs-test-support/fixtures/vcs-status-log/*.txt` — regenerate the
goldens to the ADR format by running the built binary over each state and
reviewing each against ADR-0066 (do not hand-author). Because generated goldens
cannot be red-first, the highest-risk adapter mappings additionally carry
hand-authored assertions independent of generation (§6): the change-type per
file, the five-commit cap, the unborn/empty `No commits`, and the jj bookmark
header. `no-repo-*` becomes `(status|log unavailable)`, and several states empty
today become non-empty under the always-present header.

⚠️ **Ahead/behind render identically to `clean-git` for *status* only.** ADR-0066
carries no ahead/behind, so the ahead/behind *status* goldens match `clean-git`
and stand as negative goldens proving that state does not leak. Each *log* golden
reflects its own history, not `clean-git`'s: `git-ahead` (three commits) differs
from `clean-git` (one) in line count, while `git-behind` (one commit) may
coincide with `clean-git` if their single commit shares a subject — so
regeneration accepts each log golden's own history rather than forcing any of
them to match `clean-git`.

#### 6. Conflict states, fixtures, and assertion (AC4)

**Files**: `cli/vcs-test-support/src/status_log.rs` (the shared state builders,
new — see §7), `cli/vcs-cli/tests/status_log_goldens.rs` (the golden loop,
calling the shared builder),
`cli/vcs-test-support/fixtures/vcs-status-log/conflict-{git,jj}-{status,log}.txt`
(new goldens). Add `conflict-git` and `conflict-jj` to the state set — each a
merge with conflicting edits to one tracked file, left unresolved (git: an
unresolved `git merge`; jj: `jj new` over two bookmarks with divergent edits).
The git builder must tolerate `git merge`'s non-zero exit on conflict — the
shared `Hermetic::git` helper errors on a non-zero exit, so the conflicting merge
runs through a path that expects it (or asserts the tree is left conflicted),
otherwise the fixture fails to build and the AC4 assertion silently never runs.
Both feed the golden loop (status and log) plus a **focused assertion** that the
`status` output carries the `conflicted` marker together with the unmerged path,
for both backends. This is the failing test the conflict code in §2–§3 is written
against — conflict is TDD'd where it is built, not a phase later.

The two backends reach the same rendered output by different reads (spike,
2026-08-31): git's conflict is a status item, counted in `N` naturally; jj's is
absent from the change diff and unioned in from `tree.conflicts()`. The
`conflict-jj` golden therefore shows the conflict with no accompanying change
lines — `1 changed, 1 conflicted` over a single `conflicted` file — which is what
verifies the jj union works.

#### 7. Change-type, cap, empty-history, and bookmark coverage

The fixture matrix inherited from 0169 only ever produces `added`, `modified`,
`untracked`, and `conflicted`, has no repo deeper than three commits, no unborn
git repo, and no jj bookmark — so several ADR-0066 behaviours are pinned by
nothing. Close the gaps in the same phase that introduces the code, red first:

- **`deleted` and rename** — add `deleted-{git,jj}` and `rename-{git,jj}` golden
  states (a tracked file removed; a tracked file renamed). Their goldens pin
  `Summary::Removed → deleted`, the jj before-present/after-absent → `deleted`,
  and rename → `deleted` (old) + `added` (new). Paired with a unit test over the
  pure `classify` (which also covers `Copied → added`-only) so the mapping is
  red-first, and a focused hand-authored assertion over `rename-git` (exactly two
  lines, `deleted <old>` and `added <new>`) independent of the generated golden,
  so the gix-extraction-to-`classify` wiring is not self-ratified by a golden it
  generated.
- **Five-commit cap** — build a git repo of at least six ancestors of `HEAD` and a
  jj repo of at least six ancestors of `@` (seven commits including the
  working-copy commit, since the jj walk starts at `@`'s first parent and excludes
  `@`), and assert the rendered log has exactly five lines and omits the sixth
  ancestor's subject. Every current fixture has three commits or fewer, so
  `take(5)` and the `len() == 5` break are otherwise untested and a widened bound
  would pass silently.
- **Empty history** — add an `unborn-git` state (`git init`, no commit) asserting
  `No commits` for log (exercising `is_unborn_head`) and `No changes` for status,
  and an `empty-jj` state pinning jj's `No commits` via root exclusion.
- **jj bookmarks** — assert the header over a jj working-copy commit carrying one
  bookmark (single name) and two bookmarks (the byte-sorted, comma-joined pair
  ADR-0066 fixes). Every jj fixture is otherwise bookmark-less and renders
  `(none)`, so the collection, sort, and join are unexercised. Done as a focused
  assertion rather than a shared golden, to keep the golden set branch-stable.
- **sha256 fallback** — add a `sha256-git` state (`git init
  --object-format=sha256`) with committed `(status unavailable)`/`(log
  unavailable)` goldens, pinning the `gix`-`Err` fold so a future `gix` that
  partially supports sha256 fails this test rather than silently changing
  behaviour. (The checkout `Matrix`'s `S256` shape is not reused — the status/log
  golden loop builds its own state set, so the state lives in the shared
  `status_log.rs` builder.)

The state builders — the conflict states, these coverage states, and the ten
inherited ones — are created in the shared `cli/vcs-test-support/src/status_log.rs`
here in Phase 1, not `status_log_goldens.rs`, so Phase 2's parity harness and
Phase 3's zero-spawn lane each consume one builder without Phase 3 depending on
Phase 2. `status_log_goldens.rs` calls the shared builder.

### Success Criteria

#### Automated Verification

- [x] Renderer unit tests pass: `cargo nextest run --manifest-path cli/Cargo.toml -p vcs`
- [x] Adapter + golden tests pass: `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-cli --features bash-parity`
- [x] Never-fail boundary tests — written red-first here with the §4 code: the `Err`-reporter (both git- and jj-classified `start`) and panic-reporter folds (each asserting the fallback text and the `warn!` adapter token) and the malformed-`ACCELERATOR_LOG` integration test (normal output, exit 0): `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-cli`
- [x] Release-profile guard passes: a test asserting `cli/Cargo.toml`'s `[profile.release]` does not set `panic = "abort"` (so `catch_unwind` stays effective for the shipped binary): `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-cli`
- [x] jj-settings guard + its paired exemption test pass: `mise run lint:vcs-settings:check` and `mise run test:unit:tasks`
- [x] Architecture rules pass (the new `vcs` modules obey `vcs_domain_imports_only_permitted`; `status_log.rs` and the port impl keep their `use` set inside the `vcs_adapters::library` permit list, routing `kernel::Error` through the `From` seam): `mise run pup:check` and `mise run test:integration:pup`
- [x] Feature graph unchanged: `mise run test:integration:deny` is green (not `check` — `test_vcs_library_graph.py` runs under `test:integration:deny`, not `deny:check`)
- [x] Licence closure unchanged if a gix feature was added: `mise run deny:check`
- [x] Public API baseline regenerated (diff reviewed as intended) then gated: `mise run public-api:update` then `mise run public-api:check`
- [x] Build-system component green (edits to `tasks/lint/vcs_settings.py`): `mise run build-system:check`
- [x] CLI component green: `mise run cli:check`

#### Manual Verification

- [x] Each regenerated golden reads as valid ADR-0066 output (spot-check
      `clean-git`, `dirty-git`, `detached-head-git`, `clean-jj`, `dirty-jj`,
      `colocated`, `jj-secondary`, `no-repo`, `conflict-git`, `conflict-jj`,
      `deleted-git`, `rename-git`, `unborn-git`).
- [x] The `conflict-git`/`conflict-jj` goldens show the `conflicted` marker with
      the unmerged path, and `conflict-jj` shows it with no other change lines.
- [x] `accelerator vcs status`/`log` in a real git and a real jj checkout produce
      sensible orientation text for `/commit`.

---

## Phase 2: Cross-backend parity and content goldens

### Overview

Prove the format renders identically in shape from both backends and pin the
ordinary-content goldens with a negative log assertion. Test-only; no production
behaviour changes (conflict landed in Phase 1).

### Changes Required

#### 1. Shared state builder

**File**: `cli/vcs-test-support/src/status_log.rs` — the shared `build_states`
already lives here (created in Phase 1 §7, including the `conflict-git`/
`conflict-jj` and coverage states), so the parity harness (§2) and the zero-spawn
lane (Phase 3) reuse one builder with no move required in this phase. Confirm
`fail_safe_has_no_effect_on_a_successful_status_or_log` still reaches
`build_plain_git_states` (made `pub` when the family migrated).

#### 2. git-vs-jj parity harness (AC3)

**File**: `cli/vcs-cli/tests/status_log_parity.rs` (new,
`#![cfg(feature = "bash-parity")]`). Build the same logical state in a git and a
jj repo — one untracked file, one modified tracked file (plus one git-only staged
change), over three prior commits — render `status` and `log` in each, and assert
**shape** parity rather than byte-identity (the AC itself notes the staged row and
the untracked/added divergence):

- both render a `Branch:` header line (the value is **not** compared — git
  reports its branch, jj `(none)`; branch/bookmark is the volatile field AC3
  normalises, done here by not comparing that line)
- both render an `<N> changed` summary with the same grammar
- every file line matches `^  (added|modified|deleted|untracked|conflicted)  `
  and is byte-sorted
- the modified tracked file renders as `modified <path>` in both
- log lines are compared **per line, structurally** — each matches
  `^<(HEX_OBJECT_ID|JJ_CHANGE_ID)> .+$` after masking — never by equating the two
  masked blobs: `masks::apply` substitutes `<NAME_UPPERCASED>`, so the git line
  masks to `<HEX_OBJECT_ID> …` and the jj line to `<JJ_CHANGE_ID> …`, distinct
  tokens that could never string-match across backends

An **unmasked control** pins the mask coverage concretely, not by assertion in
passing: for each log line, strip the leading id token from the *unmasked* line
and assert the remaining suffix is byte-identical to the *masked* line's suffix.
That proves the mask touched only the id span — a mask that over-matched into the
` <subject>` portion would fail this control — so no mask can silently hide a real
format difference.

#### 3. Content and negative goldens (AC3)

**Files**: the `dirty-git`/`dirty-jj` goldens (from Phase 1) already pin the
change-type markers and counts. Add a log assertion over a five-commit repo: the
rendered log has exactly five lines, each `<id> <subject>`, with a negative check
that no author, date, or ASCII-graph glyph (`@ │ ○ ◆`) appears. The author/date
half of that negative check runs against the **raw, unmasked** render — the
committed masks normalise `iso8601_timestamp`/`author_identity`, so a leaked
timestamp or author on a masked line would be rewritten to `<ISO8601_TIMESTAMP>`/
`<AUTHOR_IDENTITY>` and pass falsely; the graph-glyph and shape checks may stay
masked.

### Success Criteria

#### Automated Verification

- [x] Parity + golden tests pass: `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-cli --features bash-parity`
- [x] Mask cross-validation still green: `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-test-support` and `mise run check` (Python mask test)
- [x] `mise run check` green.

#### Manual Verification

- [x] The parity harness fails informatively when a real shape difference is
      introduced (sanity-check by temporarily breaking the renderer).

---

## Phase 3: Delete subprocess and extend zero-spawn

### Overview

Delete `vcs_adapters::subprocess`, widen the `std::process` deny crate-wide, and
prove `status`/`log` launch no child under absolute-path shadowing. Confirm the
never-fail contract under fault injection.

### Changes Required

#### 1. Delete the subprocess module (AC7)

**Files**: remove `cli/vcs-adapters/src/subprocess.rs`; drop `mod subprocess;`
and any re-export from `cli/vcs-adapters/src/lib.rs`; delete now-unused imports.
This removes `run_vcs_text`, `run_capped`, `wait_capped`, `scrub_environment`,
the `DEFAULT_CAP`/`POLL_INTERVAL` constants, and the subprocess tests. Rewrite the
`cli/vcs-adapters/src/lib.rs` crate module docstring (which describes `[subprocess]`
and carries `[subprocess]` intra-doc links) to drop the subprocess narrative — the
dangling intra-doc links are a rustdoc namespace clippy does not deny, so they
would not fail `cli:check`, but the docs must not describe a deleted module.

Deleting `scrub_environment` drops the `GIT_*`/`GIT_CONFIG*` scrub the subprocess
path forced. The existing `scrub.rs` invariance test proves the *facts* queries
answer identically under a poisoned environment — but its poison only sets
`core.bare`, and **status reads global config the facts path never surfaced**:
`core.excludesFile`, `status.showUntrackedFiles`, and `core.ignorecase` all steer
untracked enumeration. So status/log are **not** simply env-invariant, and a test
that merely re-ran them under the current `core.bare`-only poison would pass
vacuously. Two things follow, both required before deletion:

- **Characterise the real sensitivity, don't assume invariance.** Extend the
  `scrub.rs` poison with `core.excludesFile` (pointing at an excludes file that
  would hide a fixture's untracked file), `status.showUntrackedFiles`, and
  `core.ignorecase`, and route the **status/log** fixture subcommands (not just the
  `all` facts set — `scrub.rs`'s `run()` hardcodes `arg("all")`, whose query set
  excludes status/log) through the clean-vs-poisoned comparison, so the test
  actually exercises `gix::open` on the status platform's config reads.
- **Document the resulting behaviour change.** In-process status honours the user's
  global git config (`core.excludesFile`, `status.showUntrackedFiles`) exactly as
  real `git status` does — the old subprocess scrub's forced global-config-ignore
  was the anomaly, not this. This is the intended, more-consistent behaviour for
  `/commit` orientation and is recorded in Migration Notes; the test pins it rather
  than pinning a false invariance. `gix` executes none of the
  hooks/pager/aliases/credential-helpers a real `git` binary would run, so even this
  honoured-config path is a read, not code execution.

#### 2. Widen the process deny

**File**: `cli/pup.ron` — the `std::process` deny is scoped to
`^vcs_adapters::library` today (`pup.ron:334-357`). With the sole sanctioned
spawn site gone, add a crate-wide deny-only rule **named
`vcs_adapters_is_zero_spawn`** (mirroring `work_adapters_is_zero_spawn`) forbidding
`^std::process` across `^vcs_adapters($|::)`, so a future spawn anywhere in the
crate fails the architecture check. In the same edit, rewrite the kept
`vcs_adapters_library_reads_in_process` rule comment (`pup.ron:330-333`), which
still cites reaching `crate::subprocess`, to drop the now-deleted module.

**File**: `tests/integration/pup/test_import_rule.py` — add the probe pair every
pup rule carries (there is no coverage guard for `pup.ron`, so a mistyped or
deleted rule ships green otherwise). Mirror `work_adapters_is_zero_spawn`: one
probe writes a `std::process` import into a non-`library` module (e.g.
`vcs-adapters/src/lib.rs`) and asserts `vcs_adapters_is_zero_spawn` fires —
verifying the widened reach beyond `library`, the whole point of the crate-wide
scope — plus a compliant control. It runs under this phase's
`test:integration:pup` criterion.

#### 3. Never-fail fault injection (AC6)

The boundary fault-injection tests are written in **Phase 1 §4** with the code
they guard: an `Err`-returning and a panicking test-only `VcsReporter` (never file
permissions) passed to `status::run`/`log::run`, each asserting the exact
`(status|log unavailable)` text and — via an in-process capturing `tracing`
subscriber — the `warn!` adapter token. Because the token derives from `kind`, the
`Err`-reporter test runs **both** over a git-classified `start` (asserting `gix`)
and a jj-classified `start` (a bare `.jj` marker reaches `kind == Jj` without
spawning jj, asserting `jj-lib`), so a mutation that always returns `gix` or swaps
only the `Jj` arm is caught. Plus the malformed-`ACCELERATOR_LOG` integration
test. The existing `fail_safe_has_no_effect_on_a_successful_status_or_log` test
survives unchanged.

**File**: `cli/vcs-cli/tests/…` — this phase adds the one AC6 path the port-seam
tests cannot reach (they never enter `main`): a **required** integration test over
the compiled `accelerator-vcs` that forces a real adapter failure with the
deterministic `D2` degenerate shape (a `.git` gitdir pointer to nowhere — already
in the fixture matrix, not file permissions), sets `ACCELERATOR_LOG`, and asserts
a `gix`/`jj-lib` token reaches stderr. This proves `kernel::logging::init()`
actually delivers the `warn!` rather than discarding it as today — the precise
end-to-end behaviour AC6 mandates — so it is an automated test, not a Manual
Verification item.

#### 4. Zero-spawn over status/log (AC2)

**File**: the `vcs-adapters-fixture` binary — add `status`/`log` subcommands that
call `kernel::logging::init()` (reading `ACCELERATOR_LOG`, as the real `main`
now does) then `InProcessProbe`'s `VcsReporter` methods and print
`vcs::status::render`/`vcs::log::render`. This reuses the binary `zero_spawn`
already drives via `reference_artefact`, so it needs **no new cross-crate build
and no new path resolver** (`reference_artefact` is hard-coded to
`vcs-adapters-fixture`). Calling `logging::init()` here brings the one path the
real `main` adds beyond the library inside the zero-spawn envelope, closing the
gap where a spawn in the init path would otherwise be unobserved; the rest of
`main` is `println!` over the same library, and Phase 1's golden test exercises
the real binary in the normal lane.

**File**: `cli/corpus-adapters/tests/zero_spawn.rs` — add a `#[test]` that runs
`vcs-adapters-fixture status`/`log` over the shared status/log states under the
`Stubs` synthetic `PATH`, asserts the state set is **non-empty** (mirroring the
existing test's `!matrix.fixtures.is_empty()` guard, so a broken adoption inside
the shadow window cannot pass vacuously by iterating zero states), applies
`masks.toml`, compares each to its ADR golden, and asserts `stubs.spawns()? ==
None` (the method returns `Result<Option<String>, Error>`; today's queries test
asserts the same way). Keeping the test in the `zero_spawn` binary keeps the CI
selector `-E 'binary(zero_spawn)'` unchanged. Bringing masked golden comparison
into this lane is new here (the existing query test is differential, not
golden-based).

Add one **required adversarial** state to this lane — a repo whose config declares
`core.fsmonitor`, an external `filter.<name>.process` and `filter.<name>.clean`/
`smudge` bound via a `.gitattributes` `filter=<name>` (the existing `HOSTILE`
fixture declares a filter in config but binds it via no attribute, so it never
triggers — this state binds it), a `diff.*.textconv`, and `core.hooksPath` at a
hook dir — with its own `spawns()? == None` assertion, so a spawn attempt induced
by a hostile repo config is caught, not only the benign states. The ranged `gix`
pin (upstream republishes its feature set per patch) makes this the difference
between proving the happy path spawns nothing and proving a malicious config
cannot induce a spawn, so it is an always-present member of the state set — not an
either/or with a prose determination.

**File**: `cli/vcs-test-support/src/status_log.rs` — add `build_or_adopt` (the
`fixtures::Matrix` pattern: build in the normal lane, persist a manifest, adopt
inside the shadow window), so the status/log states are available once `git`/`jj`
are shadowed. This is net-new — the status/log states have no adopt path today
(only the checkout-taxonomy `Matrix` does), and `build_states` calls real
`git`/`jj`, which are gone inside the window.

**File**: `tasks/test/integration.py` — extend `zero_spawn_strong` to build the
status/log states and hand them over before the shadow window (an env handshake
mirroring `_MATRIX_ROOT`/`_build_fixture_matrix`). `vcs-adapters-fixture` is
already compiled by `_compile_zero_spawn_targets`, so no new build step is needed.
The `check-zero-spawn` CI job (`main.yml:356-361`) already invokes the task, so no
workflow-file edit is required.

#### 5. Final gate (AC8)

Run the full local CI mirror.

### Success Criteria

#### Automated Verification

- [ ] Subprocess module gone; crate compiles: `mise run cli:check`
- [ ] Architecture check passes, including the new crate-wide `std::process` rule's probe pair: `mise run pup:check` and `mise run test:integration:pup`
- [ ] AC6 real-binary token test passes: forcing the `D2` degenerate shape with `ACCELERATOR_LOG` set emits a `gix`/`jj-lib` line to stderr (automated, not manual): `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-cli --features bash-parity`
- [ ] Build-system component green (edits to `tasks/test/integration.py`): `mise run build-system:check`
- [ ] Zero-spawn (path-only, local), including the required adversarial-config state: `mise run test:integration:zero-spawn`
- [ ] `mise run` (bare default task) exits 0 end to end.

#### Manual Verification

- [ ] `ACCELERATOR_LOG=warn accelerator vcs status` on a forced-failure path emits
      a line containing `gix` or `jj-lib`.
- [ ] Strong-form zero-spawn is green on the `check-zero-spawn` CI job (Linux;
      cannot run under macOS SIP).
- [ ] `grep -rn "subprocess" cli/vcs-adapters cli/vcs-cli` returns nothing
      load-bearing.

---

## Testing Strategy

### Unit Tests

- The pure renderer: header/summary/file-list, conflict summary, all five labels,
  byte-order sort, every empty state (`No changes`, `No commits`, `(none)`,
  `(no description)`), the five-entry log cap.
- The pure `classify` function: every `Summary` variant → `FileChange`(s), the
  untracked (dirwalk variant) vs staged-add (`Summary::Added`) split, rename →
  `deleted`+`added`, and `Copied` → `added`-only.
- The pure `resolve` function (staged type wins; conflict overrides): AM→added,
  AD→added, MD→modified, staged-delete+untracked→deleted, untracked-only→
  untracked, and conflict-overrides — pinning the commit-accuracy of each same-path
  collision.

### Integration Tests

- The status/log goldens through the compiled `accelerator-vcs` (`bash-parity`),
  including the `deleted`/rename and `unborn-git`/`empty-jj` states.
- The five-commit-cap assertion over a six-plus-commit git and jj repo (exactly
  five lines, sixth omitted).
- The jj bookmark-header assertion (single and byte-sorted multi).
- The git-vs-jj parity harness with the unmasked control.
- The conflict marker/unmerged-path assertion, both backends.
- Zero-spawn over status/log under strong absolute-path shadowing (CI), including
  the adversarial-config state.
- Never-fail fault injection: `Err`, panic (`catch_unwind`), and malformed
  `ACCELERATOR_LOG`, with the `ACCELERATOR_LOG` token check.

### Manual Testing Steps

1. `accelerator vcs status` / `vcs log` in a dirty git checkout and a dirty jj
   checkout; confirm the ADR shape and useful `/commit` orientation.
2. Create a merge conflict in each backend; confirm the `conflicted` marker and
   path.
3. `ACCELERATOR_LOG=warn` on a forced-failure path; confirm the adapter token.

## Performance Considerations

In-process reads replace ~23.8 ms subprocess round-trips with ~3.6-4.7 ms cold
in-process calls (0125 evidence). The jj `status` path writes content-addressed
tree/blob objects into the store as an unavoidable consequence of computing the
new tree's id (exactly as `jj diff` does) but persists no operation and no
working-copy state — the write is idempotent, GC-reclaimable, and cannot corrupt
the repo. It does take jj's working-copy lock (via `start_working_copy_mutation`),
so a concurrent long-running `jj` holding that lock serialises against `vcs
status`; with the time cap gone (below) that wait is unbounded.

The adapter parses repository-controlled data in the caller's address space with
**no time, memory, or disk bound** — the reused snapshot keeps `max_new_file_size:
u64::MAX`, so a large working-copy file is fully read, hashed, and written as a
content-addressed blob (a disk axis, inherent to jj's tree-id computation and
GC-reclaimable, not just time/memory). Per the accepted decision (Migration
Notes), this residual DoS on a hostile or pathological repo is priced against
single-shot CLI/hook callers only (`library.rs:189-197`): a panic folds to the
fallback (`catch_unwind`, Phase 1 §4), while a hang, an OOM-kill, or unbounded
disk growth are not caught — a hang is Ctrl-C-recoverable but an OOM-kill is not a
clean unwind, and neither runs the RAII drop that releases the jj working-copy
lock.

## Migration Notes

- The `/commit` skill's injected text changes format for both backends. No
  consumer parses it and no hook reads it (`skills/vcs/commit/SKILL.md:13-14`), so
  the change is safe. The sole consumer is an **LLM prompt**, though: the rendered
  paths, subjects, and branch/bookmark are repo-controlled strings injected as
  free-form orientation, and status now widens that surface (untracked/modified
  paths and an always-present branch line, where the old `diff --cached --stat`
  was staged-only). A hostile repo could carry prompt-injection text in a filename
  or bookmark. Low likelihood (requires running `/commit` in an untrusted repo,
  and an AskUserQuestion gate precedes execution), but since this change widens the
  surface, framing the injected status/log block with a clear untrusted-data
  delimiter in `skills/vcs/commit/SKILL.md` is an **in-scope follow-up** of this
  item (a small skill-prompt edit, tracked here), not a vague future "should".
- ⚠️ **Status now honours the user's global git config.** In-process
  `gix::open(root)` reads `core.excludesFile`, `status.showUntrackedFiles`, and
  `core.ignorecase` from the user's global/system gitconfig, so untracked
  enumeration matches what real `git status` shows the developer. The old
  subprocess path forced `GIT_CONFIG_NOSYSTEM=1` and scrubbed these, so status
  output can now differ from before for a developer carrying a global
  `core.excludesFile` or `status.showUntrackedFiles=no`. This is the intended,
  more-consistent behaviour (the scrub was the anomaly) and is pinned by the
  extended `scrub.rs` test (Phase 3 §1).
- jj `status` no longer persists an operation or working-copy state (it reports
  as of the last operation, ADR-0066 line 148) — but the snapshot still writes
  content-addressed objects to the store, so `status` is not a zero-write read.
- **Fault-isolation regression, accepted.** Moving in-process removes the
  subprocess's 10-second cap and child-crash containment. The `catch_unwind`
  boundary restores never-fail for cleanly-unwinding panics, but a wall-clock hang
  (a stuck filesystem, a contended jj lock) and an unbounded read are not caught —
  recovery is Ctrl-C. This is accepted for these single-shot `/commit` callers
  rather than re-introducing a worker-thread time cap; a future hang complaint
  reopens it. Ctrl-C during a held jj working-copy lock terminates the process
  without running the RAII drop, but jj-lib 0.43 locks the working copy with an
  `flock(2)` OS-advisory lock (`jj_lib::lock::unix::FileLock`), not a marker file:
  the kernel releases the flock when the process dies, so a hard kill leaves at
  most a re-lockable lockfile — the next real `jj` operation re-`flock`s it and
  acquires immediately, with no stale-lock recovery needed. The acceptance is
  therefore unconditional: no subsequent-operation hazard follows from the removed
  RAII drop.
- Several previously-empty goldens (`clean-git`, ahead/behind, `detached-head-git`)
  become non-empty under the always-present `Branch:` header.

## References

- Work item: `meta/work/0198-vcs-agnostic-status-log-renderer.md`
- Research: `meta/research/codebase/2026-08-31-0198-vcs-agnostic-status-log-renderer.md`
- Format ADR: `meta/decisions/ADR-0066-vcs-agnostic-status-log-output-format.md`
- Replaced code: `cli/vcs-adapters/src/subprocess.rs:36-113`
- Swap points: `cli/vcs-cli/src/status.rs:21`, `cli/vcs-cli/src/log.rs:21`
- Dispatch + reuse: `cli/vcs-adapters/src/library.rs:391-401`,
  `cli/vcs-adapters/src/library/dirty_paths.rs:34-160`
- Domain crate: `cli/vcs/src/lib.rs`
- Golden harness: `cli/vcs-cli/tests/status_log_goldens.rs`
- Zero-spawn harness + CI: `cli/vcs-test-support/src/stubs.rs`,
  `cli/corpus-adapters/tests/zero_spawn.rs`,
  `tasks/test/integration.py:119-197`, `.github/workflows/main.yml:317-371`
- Guards: `cli/pup.ron:334-357`, `tasks/lint/vcs_settings.py:46-51`,
  `tests/integration/deny/test_vcs_library_graph.py:59-75`
- AC6 init: `cli/kernel/src/logging.rs:26-36`, `cli/vcs-cli/src/main.rs:82-101`
