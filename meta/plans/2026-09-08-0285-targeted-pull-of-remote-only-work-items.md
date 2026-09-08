---
type: "plan"
id: "2026-09-08-0285-targeted-pull-of-remote-only-work-items"
title: "Targeted Pull of Remote-Only Work Items and Resolution Normalisation Implementation Plan"
date: "2026-09-08T20:35:43+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0285"
parent: "work-item:0285"
derived_from: ["codebase-research:2026-09-08-0285-targeted-pull-of-remote-only-work-items"]
tags: ["work", "sync", "targeting", "pull"]
revision: "9c49306cdec8d222b253eac36d05cc769592f064"
repository: "accelerator"
last_updated: "2026-09-08T21:50:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Targeted Pull of Remote-Only Work Items and Resolution Normalisation Implementation Plan

## Overview

Extend `work sync --target <id|external-id|path>` so a token with no local file
is looked up on the remote tracker and, if it exists there, pulled into a new
local file and reconciled. The same change redefines targeted resolution: the
genuine local/local collision becomes a hard usage error (exit 2), and the
suppressed-remote note is retired because every dual-shape token now resolves to
exactly one deterministic outcome.

The three behaviours ship together because they are one function. All of
`resolve_targets` (`cli/work-cli/src/sync.rs:456`) decides them, and introducing
remote lookup is precisely what forces the collision and ordinary-synced-item
arms to be redefined.

## Current State Analysis

`resolve_targets` resolves every `--target` token against the local corpus only,
through a filesystem-pure `resolver` closure. The tracker is resolved *after*
selection completes (`sync.rs:699`), so resolution today has no network access —
a deliberate invariant from 0257 that keeps target validation
credential-independent (`sync.rs:654-655`).

Three arms of the token loop carry the behaviour this plan changes:

- **No-local-match** (`sync.rs:514-527`) — a token that resolves to neither a
  local file nor a single `external_id` entry pushes `NoMatch` → exit 3, with
  the message "untracked remote issues are imported only by a full (untargeted)
  sync". This is the abort the plan inverts.
- **Collision** (`sync.rs:484-492` via `suppressed_remote` at `438-449`) — a
  token that resolves locally to file A *and* keys a different file B's
  `external_id` records a non-fatal `Suppressed` note and lets A win.
- **Ordinary synced item** (same `Some(item)` arm) — a token equal to its own
  file's `external_id` finds no *different* file, so `suppressed_remote` returns
  `None` and it already reconciles with no note. This arm needs no logic change;
  the requirement here is satisfied once the collision arm stops emitting notes.

The engine reuse target is clean. `create_from_remote` (`apply.rs:270`) takes
only an `&ExternalId`, drives `tracker.show → author_from_remote → baseline`
with `local_synced_at = run_start_epoch`, and bypasses discovery. The single
`discovery_suppressed()` branch (`run.rs:786`) forces the targeted `untracked`
set empty; that one branch simultaneously governs pull accounting
(`run.rs:822`), the preview loop (`run.rs:886`), and the apply loop
(`run.rs:929`). A targeted pull is a non-empty, targeted `untracked` set at that
branch — not a discovery search.

### Key Discoveries

- **`show` cannot report absence** (`cli/tracker/src/lib.rs:415-425`, verified).
  A non-resolving id is `TrackerError::Retryable`, indistinguishable from a
  transient fault. Only `fetch_all([id])`, whose `absent` partition is provable
  (`lib.rs:221-247`, verified), yields a clean exit 3.
- **Resolution accumulates all failures and aborts before the engine**
  (`sync.rs:539`, `build_selection` at `594-606`). One bad `--target` aborts the
  whole selection with zero writes. Extending absence into this accumulator
  preserves that all-or-nothing property across a mixed batch.
- **The ordinary-synced-item arm already reconciles silently** (`sync.rs:447`,
  the `item.id != local_match.id` guard). No logic change; deletion of the
  `Suppressed` machinery is the whole of the requirement.
- **`create_from_remote` produces a byte-identical file to a discovery import.**
  Id and filename allocation live in `author_from_remote`
  (`cli/work-cli/src/sync_author.rs:97-159`); both create paths route through it,
  satisfying the allocation-parity criterion by construction.
- **`ItemSelection::Targeted` is a single lever** (`run.rs:98-141`). It gates
  discovery, preview imports, apply imports, and document-watermark advance from
  one enum. A targeted pull extends `Targeted` to carry confirmed remote-only
  ids rather than forcing `untracked` empty.

## Desired End State

`work sync --target PP-999`, where `PP-999` exists remotely but not locally,
creates a new local file for `PP-999`, populates it from the remote issue,
reconciles it, and advances its watermark — no exit-3 abort. A second run does
not re-pull it. The pull counts against `--max-pulls` and shows under
`--preview` with zero writes.

Resolution is deterministic per token (the rows this item changes; the unchanged
0257 malformed/unmanaged/ambiguous/outside-dir cases still map to their existing
codes and take precedence per `2 > 6 > 3 > 70`):

```text
token resolves to a single local file A ......... reconcile A, no note
  (by path, by A's local id, or by A's own external_id)
token is A's local id AND a different file B's
  external_id ................................... exit 2, name A and B, no remote call
token has no local match, present remotely ...... pull-create and reconcile
token has no local match, present, --push-only .. exit 2, name the token, no remote call
token has no local match, provably absent ....... exit 3, name the token
token has no local match, indeterminate ......... exit 70 (retryable)
```

A confirmed-present target is fetched a second time by `show` inside
`create_from_remote`; if the issue is deleted in that window `show` reports a
retryable failure (it cannot report absence), so the create surfaces as a
per-item `Failed` row with a non-zero exit, never a silent success. Under
`--preview` every "pull-create" row is a would-create with zero writes.

Verify by the acceptance criteria in `meta/work/0285-...md:92-127`, the extended
tests in `cli/work-cli/tests/cli_sync_targets.rs`, and new engine tests in
`cli/work-adapters`.

## What We're NOT Doing

- **No `search`/discovery for a targeted pull.** The existence gate is
  `fetch_all([candidates])` (by-id, scoped); `discover_untracked` stays gated off
  under `Targeted`.
- **No new id-allocation scheme.** A targeted pull routes through the existing
  `author_from_remote` path unchanged.
- **No change to the full (untargeted) sync.** The `ItemSelection::All` path,
  its report, its write set, and per-item watermark semantics stay
  byte-identical.
- **No change to standard conflict resolution** (conflict dossiers, dirty
  guard). It applies unchanged once a targeted item is present locally.
- **No retry loop around the tracker.** `indeterminate` maps to exit 70 and the
  operator re-runs; the engine does not poll.

## Implementation Approach

Resolution stays filesystem-pure; the remote probe is a *separate* pre-flight
step in `run_sync` after the tracker is resolved, so a locally-resolvable
failure (collision, usage, outside-dir) still aborts credential-free. A
no-local-match token is no longer a failure — it becomes a *remote candidate*
carried out of `resolve_targets`, de-duplicated by `canonical_external_key` so
two spellings of one issue never yield two pulls. A remote-only candidate under
`--push-only` is contradictory and aborts as a usage error (exit 2) before any
tracker contact.

Exit-code precedence stays single-sourced. `absent` and `indeterminate` become
`TargetResolutionFailure` variants carrying their own `exit_code`, so the
`2 > 6 > 3 > 70` chain lives in one `rank` function even though it is evaluated
at two sequenced moments (local failures before credentials, remote failures
after). Within one `fetch_all` batch `absent` (3) therefore dominates
`indeterminate` (70) by rank, not by iteration order.

Once credentials resolve, a single `fetch_all(candidates)` partitions the
candidates through a pure function over its `FetchOutcome`: `absent` folds into
the failure accumulator (exit 3), `indeterminate` maps to exit 70, `found`
becomes the confirmed pull set threaded into the engine. The all-or-nothing
zero-write property holds at this gate: any `absent` or `indeterminate` aborts
before a write. The subsequent per-id create loop is not transactional — a
mid-batch `create_from_remote` failure leaves earlier creates written (each with
a valid baseline) and is recoverable idempotently on re-run.

The engine change is a capability, not a behaviour switch:
`ItemSelection::Targeted` gains a `pull_ids` slice injected at the discovery
branch as the `untracked` set, so `create_from_remote`, preview, accounting, and
apply run exactly as they do for a discovery import. The discovery status stays
`SkippedTargeted` while `pull_ids` is empty and becomes `TargetedPull` only when
a pull actually occurs, so an existing reconcile-only targeted run emits the same
report line as before. The CLI passes an empty slice until Phase 3 populates it,
keeping Phase 2 free of CLI-visible behaviour change.

## Phase 1: Local/local collision becomes a usage error

### Overview

Turn the different-file collision into an exit-2 `TargetResolutionFailure`
naming both files, and delete the `Suppressed` machinery. The ordinary synced
item already reconciles silently, so this phase adds no reconciliation logic —
it hardens one arm and removes dead code and its report line. No remote contact.

### Changes Required

#### 1. Collision failure variant and detection

**File**: `cli/work-cli/src/sync.rs`
**Changes**: Add a `LocalCollision` variant (exit 2) and build it from the
different-file match. Replace the `Suppressed`-pushing branch. Rename
`suppressed_remote` to a name that reads as collision detection and have it
return the colliding file, not just its key.

```rust
enum TargetResolutionFailure {
    Malformed(String),
    NoMatch(String),
    Unmanaged(String),
    OutsideWorkDir(String),
    AmbiguousLocal(String),
    AmbiguousExternal(String),
    LocalCollision(String),
}
```

`LocalCollision` maps to `exit_codes::USAGE` in `exit_code`. In the `Some(item)`
arm, when a different local file claims the token, push
`TargetResolutionFailure::LocalCollision` with a message that labels each file's
role and carries both files' **paths** — the token is file A's local id and file
B's `external_id` — and says to re-run with the path of the intended file. The
renamed collision detector already returns the colliding file, so both paths are
in hand; the operator can copy one straight into the re-run without guessing
which side matched by which key.

#### 2. Delete the suppressed-remote machinery

**File**: `cli/work-cli/src/sync.rs`
**Changes**: Remove `struct Suppressed`, `ResolvedTargets.suppressed`,
`SelectedTargets.suppressed`, `suppression_line`, and `render_report`'s
`suppressed` parameter and its emission loop. `resolve_targets` returns only
matched items.

#### 3. Skill rendering

**File**: `skills/work/sync-work-items/SKILL.md`
**Changes** (anchored by sentence, since line numbers drift as the file is
edited): rewrite the whole **suppressed-note sentence** ("…the remote match is
reported as suppressed.") to state both outcomes explicitly — a token equal to a
single file's own `external_id` reconciles silently with no note, and a token
that is one file's id and a *different* file's `external_id` is an exit-2
collision. Add the collision to the **exit-2 entry** naming both files. Delete
the **suppressed-remote note rendering** in the Step 5 translation block *and*
rewrite the **render-region lead-in** so it no longer references a
suppressed-remote note.

Phase boundary: Phase 1 owns every sentence touching the suppressed note; Phase 3
owns the whole **discovery-suppression sentence** ("Naming any target suppresses
untracked-remote discovery…"). Neither phase edits a sentence the other owns, so
the split never bisects a sentence.

#### 4. Invert the tests the collision change reverses

**File**: `cli/work-cli/src/sync.rs` (unit tests),
`cli/work-cli/tests/cli_sync_targets.rs`
**Changes**: The red step rewrites `a_local_id_wins_and_records_the_remote_match_as_suppressed`
to assert exit 2 naming both files (its scenario is exactly the new collision)
and renames it to describe the new behaviour, e.g.
`a_local_local_collision_is_a_usage_error_naming_both_files`, so the name no
longer asserts the opposite of the body. It deletes
`the_suppression_line_collapses_record_breaking_whitespace` and updates every
`render_report(&report, &[])` call site to the new signature. The suite must not
merely gain a `collision` test — the contradictory existing test is the failing
test that demands the change.

### Success Criteria

#### Automated Verification

- [ ] The rewritten `a_local_id_wins_...` test asserts exit 2 naming both files,
      zero writes: `cargo test -p work-cli collision`
- [ ] An own-`external_id` token reconciles with no note: `cargo test -p work-cli`
- [ ] The suppressed-line test is gone and the suite compiles without
      `Suppressed`: `cargo test -p work-cli`
- [ ] Workspace lint and format clean: `mise run cli:check`
- [ ] Read-only aggregate clean: `mise run check`

#### Manual Verification

- [ ] The `sync-work-items` skill prose reads coherently with no dangling
      reference to a suppressed-remote note, and states the own-`external_id`
      reconcile case.

---

## Phase 2: Engine carries and imports targeted pull ids

### Overview

Teach the engine's `ItemSelection::Targeted` to carry a set of confirmed
remote-only ids and import them through `create_from_remote`, folding them into
pull accounting, preview, apply, and the watermark exactly as a discovery import
folds in. The CLI passes an empty set, so this phase changes no CLI-visible
behaviour and is mergeable on its own.

### Changes Required

#### 1. Selection carries pull ids

**File**: `cli/work-adapters/src/sync/run.rs`
**Changes**: Extend the `Targeted` variant. `reconciled()` keeps its meaning
(`pull_ids` are not reconciled items). Update the `ItemSelection` doc comment to
describe `pull_ids` as a third set — remote-only ids in neither `corpus` nor the
reconciled set — so the "single lever" framing still reads honestly, and state
its precondition: an id already bound to a local file must not appear here (the
engine enforces this, see §2).

```rust
pub enum ItemSelection<'a> {
    All,
    Targeted {
        items: &'a [LocalItem],
        pull_ids: &'a [ExternalId],
    },
}
```

#### 2. Inject the pull ids at the discovery branch

**File**: `cli/work-adapters/src/sync/run.rs`
**Changes**: At the discovery branch (`run.rs:786`), return the selection's
`pull_ids` — **filtered against the corpus's canonical `external_id` set**,
reusing the existing `corpus_carries` predicate — as the `untracked` set, plus a
targeted discovery status. Filtering here, at the injection point beside
`corpus_carries` and mirroring `discover_untracked`'s own filter, makes
`Targeted { pull_ids }` safe by construction for any caller rather than trusting
each caller to pre-filter (the CLI's `fetch_all` result still de-dupes upstream,
but the engine no longer depends on it for the double-binding guarantee).

Add a `DiscoveryStatus::TargetedPull { attempted }` variant, emitted **only**
when the filtered set is non-empty; otherwise keep `SkippedTargeted`, so an
existing reconcile-only targeted run emits the same discovery line as before and
this phase changes no observable output. Add the matching arm to `discovery_line`
in the same phase (its `DiscoveryStatus` match is exhaustive, so Phase 2 does not
compile — and its `cli:check` criterion does not pass — without it), emitting the
TSV `#\tdiscovery\ttargeted-pull\t<attempted>`. Rename `discovery_suppressed()`
to `discovery_search_suppressed()`: the predicate suppresses the untracked
*search*, not a by-id targeted pull.

```rust
let (untracked, discovery) = match &request.selection {
    ItemSelection::Targeted { pull_ids, .. } => {
        let confirmed: Vec<_> =
            pull_ids.iter().filter(|id| !corpus_carries(request.corpus, id)).cloned().collect();
        if confirmed.is_empty() {
            (Vec::new(), DiscoveryStatus::SkippedTargeted)
        } else {
            let attempted = confirmed.len();
            (confirmed, DiscoveryStatus::TargetedPull { attempted })
        }
    }
    _ if matches!(request.direction, SyncDirection::PushOnly) => {
        (Vec::new(), DiscoveryStatus::SkippedPushOnly)
    }
    _ => discover_this_run(request)?,
};
```

The confirmed ids flow through `pulls = plan.pull_count() + untracked.len()`
(`run.rs:822`), the preview loop (`run.rs:886`), and the apply loop
(`run.rs:929`) with no further change. `attempted` names what it holds — the
count gated in, not the count applied. The discovery line reports `attempted`;
the *applied* result is conveyed by the per-item `CreateFromRemote` outcome rows
and the pull tally, which report any `create_from_remote` failure as `Failed`.
The two never disagree because the discovery line never claims imports —
"requested/would import N", never "imported N" (see Phase 3 §4).

#### 3. Migrate every `ItemSelection::Targeted` construction site

**Files**: `cli/work-cli/src/sync.rs`, `cli/work-adapters/tests/sync_run.rs`,
`cli/work-adapters/tests/sync_create.rs`
**Changes**: Changing `Targeted` from a tuple to a struct variant is a breaking
signature change to a `pub enum`, so every construction site must migrate or the
crate will not build. The CLI's `Scope::Targeted` arm (`sync.rs:757`) constructs
`Targeted { items: &selected.items, pull_ids: &[] }` (no behaviour change yet);
the three existing work-adapters test sites (`sync_run.rs:342`, `sync_run.rs:480`,
`sync_create.rs:275`) migrate to the struct form too. Regenerate the
`cargo-public-api` snapshot for the variant-shape change.

### Success Criteria

#### Automated Verification

- [ ] A non-empty `pull_ids` with a stub `show` creates the file, counts as a
      pull, and applies it: `cargo test -p work-adapters targeted_pull`
- [ ] An empty `pull_ids` still emits `SkippedTargeted`, proving the phase's
      output is unchanged for a reconcile-only targeted run: `cargo test -p work-adapters`
- [ ] Exceeding `--max-pulls` refuses the whole run with zero writes (no partial
      creation): `cargo test -p work-adapters`
- [ ] Preview reports `CreateFromRemote`/`NotApplied` with zero writes: `cargo test -p work-adapters`
- [ ] A candidate `found` by the gate but whose `show` returns `Retryable`
      reports a `Failed` create, leaves earlier creates intact with valid
      baselines, and re-runs cleanly: `cargo test -p work-adapters`
- [ ] A `pull_ids` entry whose canonical key already binds a corpus file is
      filtered out — no second `author_from_remote` call, no extra write:
      `cargo test -p work-adapters`
- [ ] A run mixing resolved `items` and a remote-only `pull_id`, over a corpus
      that also holds a non-targeted item, writes exactly the two targeted items
      and nothing else (AC7): `cargo test -p work-adapters`
- [ ] Of two `pull_ids` where one `show` fails, the pull tally and item rows
      report one success, not two: `cargo test -p work-adapters`
- [ ] A create whose baseline write fails after the file is authored recovers on
      re-run via the file's `external_id` with no duplicate pull: `cargo test -p work-adapters`
- [ ] The same stub issue imported via discovery and via `pull_ids` authors an
      identical id, filename, and baseline entry (AC2 parity): `cargo test -p work-adapters`
- [ ] `discovery_line` emits `#\tdiscovery\ttargeted-pull\t<N>` for
      `TargetedPull { attempted: N }` (extend `render_report_emits_each_discovery_status_line`):
      `cargo test -p work-adapters`
- [ ] A second run with the id now local reconciles via the ordinary arm and the
      watermark has advanced: `cargo test -p work-adapters`
- [ ] The `All` path is unchanged (existing full-sync tests pass): `cargo test -p work-adapters`
- [ ] Workspace lint and format clean: `mise run cli:check`

#### Manual Verification

- [ ] None — this phase is engine-internal and fully covered by tests.

---

## Phase 3: Resolve remote-only targets and gate them with `fetch_all`

### Overview

Stop failing the no-local-match arm; collect the token as a remote candidate.
After credentials resolve, gate the candidates through one
`fetch_all(candidates)`: `absent` aborts exit 3 (all-or-nothing, naming every
absent token), `indeterminate` aborts exit 70, `found` becomes the `pull_ids`
threaded into the engine's `Targeted` selection. Wire the skill rendering for
the pull-create and the narrowed exit 3.

### Changes Required

#### 1. Resolution collects remote candidates

**File**: `cli/work-cli/src/sync.rs`
**Changes**: In the no-local-match arm (`sync.rs:519-527`), the `None | Some([])`
case no longer pushes `NoMatch`; it records the token as a remote candidate.
`ResolvedTargets` carries `remote_candidates: Vec<ExternalId>` alongside `items`,
de-duplicated by `canonical_external_key` so two spellings of one issue produce
one candidate. Under a `--push-only` direction a remote-only candidate is
contradictory: push a named `PushOnlyRemoteOnly(String)` variant (exit 2) in
`build_selection`, before any tracker contact, with a message stating the
contradiction and the remedy — e.g. "'PP-999' has no local file and --push-only
cannot pull a remote-only item; drop --push-only to import it." The `Some(_)`
ambiguous-external case stays exit 2. Collision, usage, and outside-dir failures
still abort before the tracker call.

Add three variants to `TargetResolutionFailure`: `PushOnlyRemoteOnly(String)`
(exit 2), `Absent(String)` (exit 3), and `Indeterminate(String)` (exit 70), so
the push-only and `fetch_all` outcomes map through the same `exit_code` + `rank`
function as the local failures. Each variant carries a user-facing message
naming its offender, honouring the `message()` invariant the other variants
hold. Extend `rank` to give four distinct values ordering `USAGE` >
`OUTSIDE_WORKDIR` > `RESOLVE_NOT_FOUND` > `RETRYABLE`, so a batch carrying both
`absent` and `indeterminate` exits 3, not 70, by rank rather than the tie-broken
iteration order `max_by_key` would otherwise use. Because
`TargetResolutionFailure` now spans pre-credential (local, push-only) and
post-credential (`absent`, `indeterminate`) failures, add a doc line
distinguishing the two so the credential-free invariant stays discoverable from
the type.

#### 2. `fetch_all` gate in `run_sync`

**File**: `cli/work-cli/src/sync.rs`
**Changes**: After `registry.resolve(&integration)` (`sync.rs:699`), if
`remote_candidates` is non-empty call `tracker.fetch_all(&remote_candidates)` and
pass its `FetchOutcome` to a pure `partition_candidates` function that returns
either the confirmed `found` ids or a `Vec<TargetResolutionFailure>`:

`FetchOutcome::found` pairs each id with the stamp the bulk read returned
(`Vec<(ExternalId, RemoteTimestamp)>`); the stamps are intentionally discarded
because `create_from_remote` re-reads each issue's body via `show`. `absent` and
`indeterminate` are `Vec<ExternalId>`, mapped to a failure carrying a per-token
message, not a bare id:

```rust
fn partition_candidates(
    outcome: FetchOutcome,
) -> Result<Vec<ExternalId>, Vec<TargetResolutionFailure>> {
    let absent = outcome.absent.into_iter().map(|id| {
        TargetResolutionFailure::Absent(format!("no local file nor remote issue for '{id}'"))
    });
    let indeterminate = outcome.indeterminate.into_iter().map(|id| {
        TargetResolutionFailure::Indeterminate(format!("remote read for '{id}' was indeterminate; re-run when the tracker is reachable"))
    });
    let failures: Vec<_> = absent.chain(indeterminate).collect();
    if failures.is_empty() {
        Ok(outcome.found.into_iter().map(|(id, _)| id).collect())
    } else {
        Err(failures)
    }
}
```

A whole-call `Err` from `fetch_all` is not automatically retryable: per the
tracker contract it fires only on a pre-flight fault — unresolvable credentials
or an id the client cannot safely embed in its query, neither fixable by a
re-run. Inspect the error class: route an unembeddable-id fault to a usage error
(exit 2) naming the token, a credential fault to the existing `UNCONFIGURED`
(74), and only a genuinely transient pre-flight fault to `Indeterminate` (70).
The failures fold into the same accumulator and `rank` function as the local
failures, so precedence `USAGE (2) > OUTSIDE_WORKDIR (6) > RESOLVE_NOT_FOUND (3)
> RETRYABLE (70)` holds across the whole run from one authority: a batch mixing
`absent` and `indeterminate` exits 3, and a run carrying both a local failure and
a remote-absent token still exits on the local code, credential-free, because
the local failures short-circuit before the tracker call. Extracting the
partition keeps it unit-testable without a `TrackerRegistry`; wrap the whole
resolve → `fetch_all` → partition → confirmed-ids sequence in one named helper so
`run_sync` gains a single call rather than an inlined block.

#### 3. Thread confirmed ids into the engine

**File**: `cli/work-cli/src/sync.rs`
**Changes**: Build `ItemSelection::Targeted { items: &selected.items, pull_ids:
&found }` from the `fetch_all` result. The double-binding guard lives in the
engine (Phase 2 §2), so the CLI passes `found` through unfiltered; the upstream
`canonical_external_key` de-dup of `remote_candidates` still prevents two
spellings becoming two candidates. The engine filter is defensive against
canonicalisation divergence between resolution and the corpus: if it ever drops a
`found` id (the id is already locally bound yet resolution did not match it), that
id is neither pulled nor reconciled — a silent no-op for a named target. Assert
the two canonicalisations are identical, and if the filter can fire, emit a
diagnostic naming the dropped id rather than discarding it silently. The
`render_report` targeted-pull line and `discovery_line` render the `TargetedPull`
status.

#### 4. Skill rendering

**File**: `skills/work/sync-work-items/SKILL.md`
**Changes**: Rewrite the whole **discovery-suppression sentence** ("Naming any
target suppresses untracked-remote discovery…") — a targeted pull now reaches a
remote-only item by id and pulls it. Reconcile every exit-code surface, not just
the first one an author notices, so all copies stay consistent (referenced by
content, since line numbers drift):

- **Step 1 exit-code list, exit-3 entry**: narrow to "no local file *nor* remote
  issue".
- **Step 1 precedence line**: change `2 > 6 > 3` to `2 > 6 > 3 > 70`.
- **Step 1 exit-code list**: add exit 70 (`RETRYABLE`) for an indeterminate
  remote read, with recovery guidance matching the other entries' pattern
  ("re-run once the tracker is reachable"); add the `--push-only` remote-only
  target as an exit-2 usage error.
- **Step 2 target-abort summary**: narrow its exit-3 description and add the
  exit-70 case, matching Step 1. Note that a *targeted* exit-70 (indeterminate
  remote read during resolution) aborts before any write, distinguishing it from
  the engine-level exit-70 that can follow partial writes — so the operator knows
  whether a re-run is a clean retry or a resume.
- **Step 2 discovery-line enumeration** (ran / skipped-push-only /
  skipped-targeted / failed): add the new `targeted-pull` state.
- **Exit-3 recovery text**: cover the "reachable tracker required to classify a
  typo" shift — a bare typo run offline surfaces as exit 70 (unreachable) or
  exit 74 (unconfigured), not exit 3.
- **Step 1 `--preview` change-class list**: add `targeted-pull` so the
  previewability of the headline behaviour is discoverable.

Admit a targeted `create-from-remote` into `pulled-untracked:`. Pin the emitted
discovery TSV shape — `#\tdiscovery\ttargeted-pull\t<N>`, where N is the
`attempted` count — and add the matching Step 5 translation entry. The
suppressed-remote Step 5 entry is already removed by Phase 1 §3; this phase only
references it as gone. The discovery line renders "Targeted pull: would import N
remote-only item(s)" under `--preview` and "Targeted pull: requested N
remote-only item(s)" on apply — it reports the *requested* count and never claims
"imported", because the actual imports are the `pulled-untracked:` rows (one per
created item). A partial-failure run therefore shows "requested N" above a
`pulled-untracked:` list of the M ≤ N that succeeded, with the failures in their
own item rows — two honest numbers for two distinct concepts, never one
overstated count.

### Success Criteria

#### Automated Verification

- [ ] `partition_candidates` returns `found` for an all-found outcome and folds
      `absent`→3 / `indeterminate`→70, with a batch carrying both exiting 3:
      `cargo test -p work-cli partition_candidates`
- [ ] Driven through `run_sync` with a stub `TrackerRegistry`, a remote-only
      `PP-999` (stub `found`) is pulled and reconciled: `cargo test -p work-cli remote_only_target`
- [ ] A mixed `found`+`absent` batch aborts exit 3 naming the absent token, zero
      writes, found not pulled: `cargo test -p work-cli mixed_targets`
- [ ] An all-success batch mixing a local-id target, a path target, and a
      remote-only `found` id reconciles the local files, creates the remote-only
      one, and writes no non-targeted item (AC7): `cargo test -p work-cli`
- [ ] A whole-call `fetch_all` `Err` for an unembeddable id exits 2 (usage), not
      70: `cargo test -p work-cli`
- [ ] Two spellings of one remote-only issue (`PP-999`, `pp-999`) yield a single
      pull, not two files: `cargo test -p work-cli`
- [ ] `--push-only --target <remote-only>` exits 2 naming the token, with no
      `fetch_all` call: `cargo test -p work-cli`
- [ ] A locally-resolvable token makes no remote call, asserted by a recording
      stub whose `fetch_all` count is zero: `cargo test -p work-cli`
- [ ] `--preview --target PP-999` shows the would-create, writes nothing, counts
      against `--max-pulls`: `cargo test -p work-cli`
- [ ] A bare remote-only token now reaches the credential/tracker phase (the
      inverted `a_no_match_target_exits_three_before_the_credential_check`):
      `cargo test -p work-cli`
- [ ] An untargeted full sync report and write set are unchanged: `cargo test -p work-cli`
- [ ] Read-only aggregate clean: `mise run check`

#### Manual Verification

- [ ] `work sync --target <a real remote-only key>` against a live Linear
      integration creates and reconciles the local file, and a second run does
      not re-pull it.
- [ ] `work sync --target <a typo'd key>` reports exit 3 naming the token with
      zero writes.
- [ ] The `sync-work-items` skill renders the pull-create and the narrowed exit
      codes with human phrasing, no raw TSV leaking.

---

## Testing Strategy

### Unit Tests

- **Resolution (work-cli)**: collision → exit 2 naming both files with each role
  labelled; own-`external_id` token reconciles with no note; no-local-match token
  becomes a candidate; duplicate spellings collapse to one candidate; a
  remote-only candidate under `--push-only` → exit 2 with no tracker call;
  ambiguous-external stays exit 2.
- **`partition_candidates` (work-cli, pure)**: `found` → pull ids (id projected
  from the `(id, stamp)` pair); `absent` → exit 3; `indeterminate` → exit 70; a
  mixed `absent`+`indeterminate` outcome exits 3 by rank; an unembeddable-id
  whole-call `Err` → exit 2; precedence `2 > 6 > 3 > 70` across mixed local and
  remote failures. Tested directly on a constructed `FetchOutcome`, no tracker.
- **Engine (work-adapters)**: `pull_ids` create-from-remote counts as a pull,
  refuses the whole run when over `--max-pulls`, previews with zero writes,
  advances the watermark; a `found`-then-`show`-`Retryable` create reports
  `Failed` and leaves earlier creates recoverable; a baseline-write failure after
  authoring recovers on re-run; a `pull_id` already bound in the corpus is
  filtered out (no duplicate author); the same stub issue via discovery and via
  `pull_ids` authors an identical id/filename/baseline (AC2 parity); of two
  `pull_ids` where one create fails the tally reports one success; a run mixing
  resolved `items` and a `pull_id` writes exactly the targeted union (AC7); a
  re-run reconciles via the ordinary arm; an empty `pull_ids` emits
  `SkippedTargeted`; `discovery_line` emits the pinned `targeted-pull` TSV; the
  `All` path is unchanged.

### Integration Tests

The subprocess harness in `cli/work-cli/tests/cli_sync_targets.rs` is bin-only
and cannot inject a stub tracker, so it covers only the credential-independent
assertions it can actually make: a collision exits 2, a `--push-only`
remote-only target exits 2, and local resolution aborts before the credential
check. The remote-gate behaviour — a remote-only pull, a mixed-batch abort, and a
preview — is driven in-crate through `run_sync` with a stub `TrackerRegistry`
(and a recording stub for the no-remote-call assertion), which is the only seam
that can exercise the post-credential gate without live Linear/Jira.

### Manual Testing Steps

1. `work sync --target <remote-only key>` against Linear; confirm the new local
   file, its taxonomy, and reconciliation.
2. Re-run the same target; confirm no re-pull and an advanced `local_synced_at`.
3. `work sync --target <typo'd key>`; confirm exit 3, zero writes.
4. `work sync --target <a local id> --target <a different file's external_id
   that is that local id>`; confirm exit 2 naming both files.

## Performance Considerations

One extra by-id `fetch_all` round-trip per run that names a remote-only target,
on top of the `show` inside `create_from_remote`. `fetch_all` batches every
candidate into a single call, so the cost is one round-trip regardless of the
number of remote-only targets. A run naming only locally-resolvable targets
makes no additional remote call.

## Migration Notes

None. The baseline schema is unchanged; a targeted pull writes the same `Entry`
shape as a discovery import.

## References

- Original work item: `meta/work/0285-targeted-pull-of-remote-only-work-items.md`
- Research: `meta/research/codebase/2026-09-08-0285-targeted-pull-of-remote-only-work-items.md`
- Predecessor plan: `meta/plans/2026-09-06-0257-sync-specific-work-items.md`
- Resolution: `cli/work-cli/src/sync.rs:456` (`resolve_targets`)
- Reuse target: `cli/work-adapters/src/sync/apply.rs:270` (`create_from_remote`)
- Injection point: `cli/work-adapters/src/sync/run.rs:786` (`discovery_suppressed`)
- Tracker contract: `cli/tracker/src/lib.rs:413-454` (`show`, `fetch_all`)
- Skill: `skills/work/sync-work-items/SKILL.md:78-101,268-296`
