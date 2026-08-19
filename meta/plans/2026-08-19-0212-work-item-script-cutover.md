---
type: plan
id: "2026-08-19-0212-work-item-script-cutover"
title: "Work-Item Script Cutover Implementation Plan"
date: "2026-08-19T11:18:42+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0212"
parent: "work-item:0212"
derived_from: ["codebase-research:2026-08-19-0212-work-item-script-cutover"]
relates_to: ["work-item:0213", "work-item:0211", "work-item:0171"]
tags: [rust, cutover, work-items, fixtures, cli, tracker]
revision: "792cb868f0b365c43d7f91680a221920c02e92d4"
repository: "accelerator"
last_updated: "2026-08-19T12:12:17+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Work-Item Script Cutover Implementation Plan

## Overview

Retire the eighteen `work-item-*.sh` / `test-work-item-*.sh` scripts and the
build-system machinery that props them up, replacing every capability they carry
with Rust behind `accelerator work …`. The work item was scoped as
"repoint + delete", but three of its acceptance criteria cannot pass without
*new* `RemoteTracker` port operations and a substantial sync-engine extension —
so this plan builds that feature work first, then repoints the skills, then
performs the irreversible deletions last.

## Current State Analysis

The bash cluster was a set of composable primitives; the Rust `accelerator work
sync` fused them into one umbrella command whose stages are internal. Three
capabilities the skills still reach for as isolated pieces have no CLI seam:

- **Untracked remote discovery** — `sync-work-items` lists remote issues with no
  local counterpart via `work-item-fetch-remote.sh … search`. `fetch_all(ids)`
  is key-scoped and total over requested ids (`cli/tracker/src/lib.rs:199-236`);
  it cannot express an unkeyed query.
- **Create-preview field resolution** — `work-item-create-remote.sh --dry-run`
  surfaces an unresolvable Jira project before the confirm gate. The `create`
  port method is create-only.
- **Update-preview payload validation** — `work-item-update-remote.sh --dry-run`
  is what `/sync-work-items --preview` uses to validate every push against the
  live tracker. The Rust `--preview` early-returns before the apply loop
  (`cli/work-adapters/src/sync/run.rs:183-197`) and makes **no** port call.

The sync engine itself holds none of `sync-work-items`' remote-discovery
orchestration: the `Action` enum is `Push | Pull | SkipConflict | SkipDirty |
Prompt | Noop` (`cli/work/src/sync/decide.rs:24-31`) with no create-from-remote
variant; `GatheredFacts` (`cli/work-adapters/src/sync/fetch.rs:53-60`) has no
remote-only field; the apply loop **drops** keyless items
(`run.rs:219-225`) and never calls `create`.

The parity tests pin bash↔Rust by construction — a rename propagates into
assertions rather than passing silently — so they must be converted, not
deleted, preserving the committed goldens byte-for-byte. 0210 froze the oracle:
`cli/work-adapters/tests/fixtures/bash-parity-baseline.txt` records **68 files**
under `test-fixtures/`, per-test fixture-case ids, and pre-conversion assertion
counts.

### Key Discoveries

- ✅ The dirty-overwrite guard is live-wired into `accelerator work sync` and
  fail-safe: `Action::Pull` is reachable only when `Dirtiness::Clean`
  (`decide.rs:87-103`), and `Dirtiness::Unknown` decides as `Dirty`
  (`decide.rs:12-21`). The work item's size-bounding assumption resolves in its
  favour — no new dirty-guard behaviour is needed.
- The numeric `E_DISPATCH` owner is `cli/work-cli/src/exit_codes.rs:5-16`, a
  superset of the `.md` (adds 0–5), **not** the `tracker` crate. The class enum
  `TrackerError` (`cli/tracker/src/lib.rs:144-179`) is a separate two-class
  taxonomy pinned by `dispatch-codes.txt`.
- `cli/tracker/src/errors.rs` does not exist and the four "item-4" comments are
  already provider-neutral — nothing to scrub there. The live bash coupling is
  in `exit_codes_parity.rs`, `dispatch-codes.txt`, and
  `cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt:16`.
- `cli/work-adapters/tests/project_remote_parity.rs` does not exist; projection
  parity lives in `cli/remote-projection/tests/parity.rs`,
  `cli/jira-client/tests/projection_corpus.rs`,
  `cli/linear-client/tests/projection_corpus.rs`.
- The `SHELL_LIBRARIES` entry for `work-item-bridge-codes.sh` is at
  `tasks/lint/scripts.py:34`, not line 18. `_EXPECTED_WORK_SUITES = 5` is at
  `tasks/test/integration.py:51`; the `_require_suite_floor` helper
  (`:81-107`) backs six subtree floors and must survive — only the `work`
  constant and the `work` task's call come out.
- `RemoteTracker` has six impl sites, none with default bodies: the two real
  clients, `RecordingTracker`, `work-adapters/tests/sync_apply.rs::Fake`,
  `cli/tracker/tests/port.rs::FixedTracker`, and the trait definition. Every new
  method must be implemented at all six.

## Desired End State

`ls skills/work/scripts/*.sh` matches nothing, the directory is gone, and a
repository-wide grep for the eighteen basenames and for `skills/work/scripts`
(excluding `meta/`) is empty. The three work skills invoke `accelerator work …`
exclusively. Every capability the deleted scripts carried is reachable through
the CLI, with the three previously port-less capabilities discharged against new
port operations and verified behaviourally. `mise run` exits 0 end-to-end.

## What We're NOT Doing

- **The interactive prompts** — the pull-overwrite confirmation and the
  conflict-resolution loop belong to 0213, which shares `sync-work-items/
  SKILL.md`. This plan builds the non-interactive engine seams 0213 hangs its
  prompts on, and stops there. Whichever child lands second rebases onto the
  other.
- **The whole-repository `jq`/`curl` empty-set equality assertion** — that is
  0211's, which lands last. This child only removes those tools from the *work*
  skills' `allowed-tools`; the jira and linear skills still declare both at this
  child's boundary.
- **CI-gating the credentialed contract run** — kept manual and developer-run,
  as it is today. No repository secrets, no dedicated CI job.
- **A `work normalise` subcommand** — the normalise pipe is folded into the sync
  engine rather than re-exposed; a bare subcommand would re-open the drift risk
  the internal digest exists to remove.
- **Retiring the bashisms linter, exec-bit guard, or `SHELL_LIBRARIES` frozenset
  wholesale** — those are 0174's, unblocked by this child's deletions.

## Implementation Approach

Build feature-complete Rust first (Phases 1–4), repoint the skills onto it
(Phase 5), then delete (Phase 6), then verify against live tenants (Phase 7).
The ordering is forced by one invariant: **no script may be deleted while any
Rust test, skill, or task still references it**, so the replacements and
conversions strictly precede the deletions. Each phase merges independently with
a green `mise run check` (and, for phases touching Rust, a green `mise run
cli:check`) at its boundary. Test-driven throughout: contract properties and
parity goldens are written red before the code that satisfies them.

The three port additions override 0204's frozen-port protocol and 0171's
"additive item parented to 0136" rule. Per the planning decision, they are
absorbed into this child; Phase 2 records the override in 0171's `## Decisions`,
naming 0212 as the item that reopens the port.

---

## Phase 1: Relocate fixtures and convert the parity tests to pure Rust

### Overview

Move the 68-file corpus out of `skills/work/scripts/test-fixtures/` into
per-crate `tests/fixtures/` trees, and convert every script-coupled Rust test to
read those in-crate fixtures and shell out to nothing. After this phase no Rust
test invokes any `work-item-*.sh`; the scripts remain on disk, now unread by the
Rust suite. This is the safe first step — it removes the test↔script coupling
without deleting anything a skill or task still names.

### Changes Required

#### 1. Relocate the fixture corpus

**From**: `skills/work/scripts/test-fixtures/`
**To**: per-crate `tests/fixtures/`, by actual consumer (corrected against the
work item's map):

| Corpus | Home |
|---|---|
| `sync-classify.json`, `sync-decide.golden`, `sync-label.golden`, `push-decide.golden`, `normalise/case-*` | `cli/work/tests/fixtures/` |
| `sync-baseline/case-*` | `cli/work-adapters/tests/fixtures/` |
| `project-remote/case-*` | `cli/remote-projection/tests/fixtures/` |
| `section-diff/case-*` | `cli/work-adapters/tests/fixtures/` |

The ten orphan goldens with no runtime reader — `canonicalise-id`,
`template-field-hints`, `file-dirty`, `next-number`, `read-field`, `resolve-id`,
`update-tags`, and the loose provenance-header goldens `normalise.golden`,
`section-diff.golden`, `project-remote.golden` — are **deleted**, each recorded
in 0171's `## Decisions` with the reason (bash tested them inline; Rust carries
its own inline oracles). Relocated count plus enumerated deletions must equal the
68-file baseline number.

#### 2. Convert the parity tests

Switch each reader from the reach-into-repo form
(`CARGO_MANIFEST_DIR/../.. + skills/work/scripts/test-fixtures`) to the in-crate
form (`CARGO_MANIFEST_DIR/tests/fixtures`):

- `cli/work/tests/`: `sync_push_decide.rs`, `sync_classify.rs`, `sync_decide.rs`,
  `normalise_parity.rs`, `sync_label_parity.rs`
- `cli/work-cli/tests/`: `exit_codes_parity.rs`, `cli_diff_parity.rs`
- `cli/remote-projection/tests/`: `parity.rs`, `corpus_hashes.rs`
- `cli/jira-client/tests/projection_corpus.rs`,
  `cli/linear-client/tests/projection_corpus.rs`
- `cli/work-adapters/tests/`: `sync_baseline_corpus.rs`,
  `sync_baseline_shellout_parity.rs`, `bash_parity_baseline.rs`

`corpus_hashes.rs` content-hashes the `sync-baseline` corpus, but that corpus
relocates to `cli/work-adapters/tests/fixtures/` (§1) — so the content-hash guard
**moves with its corpus into `cli/work-adapters/tests/`** rather than staying in
`remote-projection`, keeping the guard and the engine tests reading the same
fixture bytes (no cross-crate read, no duplicated copy).

The two tests that currently **shell out** (`sync_baseline_shellout_parity.rs`
→ `work-item-normalise.sh`/`-project-remote.sh`; `diff_shellout_parity.rs`
→ the diff script) are rewritten to assert against the committed goldens rather
than a live bash run. Conversion removes the independent differential oracle:
after it, the Rust recipe is only ever checked against goldens the same recipe
regenerates, so a future regeneration of code + golden together would pass
tautologically. Document in the converted test that its goldens **are the frozen
bash oracle and must never be regenerated from Rust output**. The tripwire this
relies on must actually exist: today only `corpus_hashes.rs` content-hashes the
corpus, and only the 4 sync-baseline cases — the `bash_parity_baseline.rs` guard
compares **case-directory-name sets, not content**, so the section-diff and
normalise goldens would have no content guard after conversion and could be
regenerated code-and-golden-together tautologically. **Extend a content-hash
guard (as `corpus_hashes.rs` does for sync-baseline) to every relocated golden
the converted shellout tests used to protect**, so §4 enforces byte-identity, not
just set membership — only then is the differential-oracle removal safe. Where a
shellout test's only purpose was bash↔Rust differential comparison and a
pure-Rust oracle already covers the same cases, it is deleted with its reason
recorded rather than converted.

#### 3. Delete the differential tests per 0171 D10

**File**: `cli/tracker-support/tests/mapper_differential.rs`
**Change**: delete. It shells out to `work-item-create-remote.sh` /
`-update-remote.sh` (`:58-59`); 0171 D10 records it "deleted by 0212 with the
assets they drive". Record the deletion and its reason in 0171's `## Decisions`.

#### 4. Update the corpus-set guard in lockstep

**File**: `cli/work-adapters/tests/fixtures/bash-parity-baseline.txt` and its
reader `bash_parity_baseline.rs`
**Change**: the guard reds the build when the corpus set changes, so update the
recorded set to the relocated layout in the same change. The reader today joins a
**single** root (`skills/work/scripts/test-fixtures/<directory>`, `:61`) and
walks one tree; restructure its present-case computation to resolve each recorded
corpus to its new **per-crate** home across the four destination trees, and add a
check that every relocated corpus is covered by a recorded row so no destination
drifts unguarded. Left on the old single root it breaks when Phase 6 deletes the
directory; repointed at only one destination it stops guarding the other three.
The per-case ids and the byte-identity of every golden are preserved.

### Success Criteria

#### Automated Verification

- [x] `skills/work/scripts/test-fixtures/` no longer exists (relocated + deleted
      accounts for all 68 files).
- [x] No Rust test references `skills/work/scripts`: `! grep -rl
      "skills/work/scripts" cli/` returns nothing.
- [x] The full Rust workspace passes: `mise run cli:check` and `mise run
      test:unit:cli` (the underlying nextest run) both exit 0.
- [x] `mise run check` exits 0.

#### Manual Verification

- [x] Every relocated golden is byte-identical to its pre-move content
      (`jj diff` shows pure renames, no content edits).
- [x] Each deleted orphan golden and `mapper_differential.rs` is listed in
      0171's `## Decisions` with its reason.

---

## Phase 2: Add the three RemoteTracker port operations

### Overview

Add `search`, `preview_create`, and `validate_update` to the `RemoteTracker`
trait, implement them in both clients and all four fakes, and pin each with a
contract property. Purely additive — no deletions, no engine changes. This
reopens the port frozen by 0204; the override is recorded here.

### Changes Required

#### 1. New port vocabulary and trait methods

**File**: `cli/tracker/src/lib.rs`
**Changes**: three new types and three new trait methods. The types live in
`tracker` itself (pup limits the crate to `std`/`core`/`alloc`/`crate`).

```rust
pub struct SearchScope {
    pub project: Option<String>,
    pub all_projects: bool,
    pub filters: Vec<(String, String)>,
}

pub struct Discovery {
    pub found: Vec<(ExternalId, RemoteTimestamp)>,
    pub complete: bool,
}

pub enum FieldResolution {
    Resolved(String),
    Unset,
    Unresolvable(String),
}

pub struct CreatePreview {
    pub project: FieldResolution,
    pub issue_type: FieldResolution,
}

pub enum ValidationOutcome {
    Valid,
    Rejected { reasons: Vec<String> },
}
```

`Discovery` is distinct from `FetchOutcome`: it carries a truncation flag rather
than partitioning over requested ids, because a discovery query has no requested
set. `SearchScope`'s `project: Option<String>` and `all_projects: bool` can
express a contradictory pair; it mirrors the existing `jira-client` `Search`
shape, so rather than re-model it as an enum here, a field doc comment states the
precedence (`project` wins when both are set) so the resolution is not left
implicit in the composer. `CreatePreview` models **both** the project and the issue-type as a
three-state `FieldResolution`, so the caller can distinguish `Unset` (nothing
configured — a benign default) from `Unresolvable` (a configured value absent
remotely — the state the create-preview AC must flag) and recover each field's
*source*. A two-state `Option<String>` would collapse both cases into `None` and
lose the source annotation the create skill renders (the five tab-separated
fields `jira\t<issue type>\t<type source>\t<project>\t<project source>`).
`ValidationOutcome` distinguishes a `Valid` payload from a `Rejected { reasons }`
one so `validate_update` reports *why* a payload is invalid rather than folding it
into an error class. Three new trait methods:

```rust
fn search(&self, scope: &SearchScope) -> Result<Discovery, TrackerError>;
fn preview_create(&self, kind: &str) -> Result<CreatePreview, TrackerError>;
fn validate_update(
    &self,
    id: &ExternalId,
    title: &str,
    body: &str,
) -> ValidationOutcome;
```

Each carries a doc comment stating its **remote-contact profile** separately, not
a shared "pre-flight" banner, because the three differ. Each `Result`-returning
method also carries an explicit `# Errors` section stating its `Retryable`/
`Terminal` behaviour, matching the existing `RemoteTracker` methods' convention
(and satisfying clippy `missing_errors_doc`):

- `search` — unkeyed remote read; `Retryable` only, never `Terminal`, and a
  cap-hit returns `Discovery { complete: false }` rather than an error.
- `preview_create` — performs a remote existence check (Jira resolves the
  project against `discover_projects`), so it can fail `Retryable` on a transport
  failure; an unresolvable key is a successful `Ok(CreatePreview { project:
  Unresolvable(..) })`, not an error.
- `validate_update` — a **local payload-composition check** for both providers
  (your decision), so it makes no remote call and cannot mutate; it returns a
  `ValidationOutcome` directly, needing no `TrackerError`. It detects
  locally-checkable omissions (a required field the composed payload leaves
  empty) and reports them as `Rejected { reasons }`. Live-tracker field
  validation is intentionally dropped — see §2/§3.

These three operations join `RemoteTracker`'s existing four on one port. That is
a deliberate choice, not drift: splitting the two preview/discovery reads onto a
separate `dyn`-compatible sub-trait was considered and rejected — the sync engine
and the skills consume the port as one seam, and a second trait would fragment
the fake-implementation surface for little gain. The seven-method port is
recorded as intentional in 0171's `## Decisions`.

#### 2. Jira client

**Files**: `cli/jira-client/src/jql.rs`, `client.rs`, `discovery.rs`
**Changes**:
- `search` drives the existing `jql::compose` (test-wired today) instead of
  `key_clause`, and **returns a truncation flag** rather than collapsing a
  cap-hit to `Err` (`client.rs:186-244` currently destroys the completeness
  signal).
- `preview_create` calls `discovery::discover_projects` for the remote existence
  check (your decision: meet the AC's "unresolvable project key" literally, not
  the bash config-unset-only behaviour). It returns `FieldResolution::Resolved`
  for the project when the configured key exists, `Unset` when none is
  configured, and `Unresolvable` when a configured key is absent from
  `discover_projects`; the issue-type field is resolved the same three ways
  against the type catalogue (`Resolved(<type>)` kind-mapped, `Unset` defaulted,
  `Unresolvable` for an unknown kind) so both the type value and its source are
  recoverable. This needs a `SurfaceError → TrackerError` shim, which does not
  exist yet; the shim maps a transport/reachability `SurfaceError` to
  `TrackerError::Retryable` (a read that provably applied no mutation is never
  `Terminal`) and is pinned by an offline unit test.
- `validate_update` is a **local payload-composition check** (your decision:
  drop the live-tracker validation the bash `--dry-run` performed; both providers
  validate locally only). It reuses the shared compose helper — extracted from
  the inline block at `client.rs:366` — to build the payload, then returns
  `ValidationOutcome::Rejected { reasons }` for any locally-detectable omission (a
  required field the composed payload leaves empty) and `Valid` otherwise, making
  no `transport.send` call. The extracted helper is shared with `update`.

#### 3. Linear client

**Files**: `cli/linear-client/src/filter.rs`, `client.rs`
**Changes**:
- `search` lets `fetch_page` take a caller-populated `Search` (the team search
  already runs in production) and returns the `(index, truncated)` the loop at
  `client.rs:376-394` already computes.
- `preview_create` is trivial — `CreatePreview { project:
  FieldResolution::Unset, issue_type: FieldResolution::Unset }` (single-team,
  catalogue-fixed; Linear has no project key to resolve).
- `validate_update` is a **local payload-composition check** (Linear's GraphQL
  has no non-mutating update-validation endpoint, so a remote pre-flight is not
  available there anyway; local-only keeps both providers uniform). It composes
  the payload locally and returns `Rejected { reasons }` for a locally-detectable
  omission, `Valid` otherwise, making no `self.call`.

#### 4. The four fakes and the contract harness

**Files**: `RecordingTracker`, `cli/work-adapters/tests/sync_apply.rs::Fake`,
`cli/tracker/tests/port.rs::FixedTracker`, `cli/tracker-test-support/src/
contract.rs`
**Changes**: implement all three methods at each fake (no default bodies), and
give the engine-facing fakes seed/configure hooks so Phase 3's edge cases are
drivable — mirroring the existing `set_show_result`/`fail_show`/`fail_update`
seams: seedable `search` results **including a truncation flag**, a
configurable `create` outcome (success id / `Terminal`), and configurable
`preview_create`/`validate_update` outcomes. An inertly-stubbed fake (empty
`Discovery`, `unimplemented!()`) would leave Phase 3's truncation, over-threshold,
and `Terminal`-create tests unable to induce their conditions.

Add these contract properties and wire each into `run_all` and
`timed_conformance`, bumping `run_all`'s hard-coded property count
(`contract.rs:288-298`) by the number added:

- **`search_truncation_property`** — *induces* truncation (a fake capped below the
  seeded set; a real client via a scope it cannot see past) and asserts
  `complete == false`, mirroring `unaccounted_id_is_indeterminate_not_absent`. A
  happy-path-only property would pass with `complete` hard-coded `true` — the
  exact regression this port change exists to prevent.
- **`preview_create_no_mutation_property`** — asserts remote state is unchanged
  after `preview_create` (no create observed on the fake), pinning the
  no-mutation invariant that an Ok/Err-shape-only property would miss.
- **`preview_create_resolution_property`** — asserts the three `FieldResolution`
  outcomes for **both** fields: a seeded existing project/type → `Resolved`, none
  → `Unset`, a seeded absent value → `Unresolvable`.
- **`validate_update_outcome_property`** — asserts a payload missing a
  locally-required field returns `Rejected { reasons }` naming the field, and a
  complete payload returns `Valid` (validate_update is local, so no remote-state
  assertion is needed — the type makes mutation unrepresentable).

Add a success criterion that `timed_conformance` emits an evidence record for
each new property (its record count increases by the number added), so a property
wired into `run_all` but forgotten in `timed_conformance` cannot slip the
committed contract evidence.

#### 5. Regenerate the public-api fixture and record the override

**File**: `cli/tracker/tests/fixtures/public-api.txt`
**Change**: regenerate (only `tracker` is pinned by cargo-public-api; clients and
adapters are exempt). **Review the diff** rather than accepting it wholesale: it
must contain only the three additive trait methods plus `SearchScope`,
`Discovery`, `FieldResolution`, `CreatePreview`, and `ValidationOutcome` (and
their derived impls) — no other new `pub` item. An accidentally-`pub` helper on
the extracted compose block or a leaked `SurfaceError` shim type slipping into
the surface is a regression the guard exists to catch.

Record in `meta/work/0171-jira-and-linear-integrations.md` `## Decisions`: the
frozen-port override naming 0212, the seven-method port recorded as a deliberate
single-seam choice (sub-trait split considered and rejected), and each of the
three capability fates as "re-sited above the port via new operation `<name>`".
In the same edit, **flip the matching *open* register entries to *decided*** —
"Unkeyed discovery `search`", "Create `--dry-run` field-resolution preview"
(decided: `preview_create` with a live existence check and three-state
`FieldResolution` for project and issue-type), and "Update `--dry-run` payload
validation" (decided: `validate_update` is a **local** payload-composition check
returning `ValidationOutcome`; live-tracker field validation intentionally
dropped — Linear has no non-mutating validation endpoint, so a remote pre-flight
would not be uniform across providers) — so the register does not list as open
the questions this plan answers. Record too that `validate_update` **stays on the
`RemoteTracker` port despite making no remote call**: the check reuses each
client's provider-specific payload-compose helper, which the trait's dynamic
dispatch already selects, so hosting it as a free core function would re-derive
that dispatch for no gain; the accepted cost is one non-`Result`, non-remote
method on an otherwise remote-contact port, and reintroducing any remote
validation later is a signature-breaking change across all six impl sites — a
conscious, revisitable boundary, not an accident.

### Success Criteria

#### Automated Verification

- [ ] Contract properties fail before the client implementations exist and pass
      after: `mise run test:unit:cli` exits 0.
- [ ] The `search_truncation_property` fails when `complete` is forced `true` and
      passes only when a truncated discovery reports `complete == false`.
- [ ] The `preview_create_no_mutation_property` fails if `preview_create` is made
      to mutate and passes otherwise.
- [ ] The `preview_create_resolution_property` distinguishes `Resolved` / `Unset`
      / `Unresolvable` for **both** the project and the issue-type field.
- [ ] The `validate_update_outcome_property` returns `Rejected { reasons }` naming
      a missing locally-required field and `Valid` for a complete payload.
- [ ] An offline unit test pins the `SurfaceError → TrackerError` shim mapping an
      unresolvable-project surface error to `Retryable`.
- [ ] `run_all` returns a non-zero, increased property count, and
      `timed_conformance` emits an evidence record for each new property (its
      record count increases by the number added).
- [ ] `mise run cli:check` exits 0 — clippy, rustfmt, and the regenerated
      `public-api.txt` are consistent (`mise run test:integration:cli` covering
      cargo-public-api passes, per `tasks/README.md`).
- [ ] `mise run check` exits 0.

#### Manual Verification

- [ ] 0171's `## Decisions` records the override and all three fates.
- [ ] The Jira `preview_create` remote check is exercised against a real
      unresolvable key in Phase 7's manual run.

---

## Phase 3: Sync-engine orchestration seams (non-interactive)

### Overview

Extend the sync engine with the remote-discovery orchestration
`sync-work-items` performs in bash: untracked-remote pull, unsynced create, and
per-push preview validation. Build only the non-interactive seams — the prompts
are 0213's. Consumes Phase 2's port operations.

`run()` already carries `#[allow(clippy::too_many_lines)]`; rather than extend
its inline plan-then-apply match with three more behaviours, this phase lands
them as named functions — `discover_untracked` (search minus local, gated),
`apply_create` (the create-from-local and create-from-remote write path), and
`validate_pushes` (the preview loop) — so complexity stays proportional. Both new
write paths and the existing pull path route their frontmatter+body assembly
through **one shared helper** (reusing `digest::remote_body`), rather than three
near-identical `reconstruct_pulled_content`/`local_title_and_body` routines that
would drift.

### Changes Required

#### 1. Untracked remote pull (Gap A)

**Files**: `cli/work/src/sync/decide.rs`, `cli/work/src/sync/plan.rs`,
`cli/work-adapters/src/sync/fetch.rs`, `run.rs`, `apply.rs`
**Changes**:
- A new `Action::CreateFromRemote` variant, wired through the sites that need a
  real, report-affecting keyword — `from_keyword`, the `Display` impl, and
  **`action_keyword` (`cli/work-cli/src/sync.rs:152`)**, a third compiler-
  exhaustive `match action` whose keyword lands in the just-frozen report-format
  golden (the golden case must be extended for it — see Gap A report note). Note
  that `awaiting_human` (`run.rs:84`) is a non-exhaustive `matches!`, so excluding
  creates from it is a deliberate choice the compiler will **not** enforce — state
  it explicitly. (The enum is `Push|Pull|SkipConflict|SkipDirty|Prompt|Noop`
  today.)
- Create-from-remote is a **parallel orchestration path**, not an item threaded
  through the id-keyed `compute_plan`/apply loop: the existing pipeline keys
  everything on local id (`GatheredFacts.per_id` is a `BTreeMap` by local id;
  the apply loop resolves via `items.iter().find(|c| c.id == planned.id)`), and a
  discovered issue has no local id, path, or digest to satisfy that.
  `discover_untracked` runs before the plan and performs only **reads** (search +
  show) to compute the discovery set and its count; the actual allocate-assemble-
  write step happens after the combined gate below. The `CreateFromRemote`
  variant exists so the count is visible to reporting and the gate, not so
  remote-only items traverse `find`. Because it is produced out-of-band (by
  `discover_untracked`, not `decide()`) and never reaches the apply loop's `find`,
  it carries a doc comment saying so — and, since the apply loop's `match
  planned.action` (`run.rs:210`) is exhaustive and *forces* an arm for every
  variant, that arm is an explicit `unreachable!` naming the invariant
  ("`CreateFromRemote` is applied by `discover_untracked`, never the id-keyed
  loop") rather than a silent no-op.
- `discover_untracked`'s write path is gated to
  `SyncDirection::PullOnly`/`Bidirectional` (mirroring the create-from-local mode
  gate), because materialising a discovered remote issue as a new local file is a
  pull-direction operation. A `--push-only` run authors **no** untracked local
  files.
- A remote-only carrier on `GatheredFacts` for discovered issues holding no
  local counterpart (none exists — `per_id` is keyed by local id). A discovered
  issue reported as a `NotApplied` `CreateFromRemote` `ReportedItem` needs a
  `PlannedAction.state`; give it a dedicated marker (or a state) **excluded from
  `awaiting_human` (`run.rs:84-93`)** and rendered as an automatic create by
  `render_report`, so a fully-automatic create never flips the run's
  awaiting-human signal or exit code. `Indeterminate`/`RemoteAbsent` must not be
  reused here — `awaiting_human` matches them.
- **Preview is read-only**: `discover_untracked`'s allocate-and-write step is
  gated on `RunMode::Apply`. The current `RunMode::Preview` early-return lives
  *after* the plan is computed (`run.rs:183`), so an ungated write path would
  create files during `--preview`, violating the read-only contract and masking
  the very discovery it was meant to preview. In `Preview`, the path performs the
  `search`/`show` reads and reports each would-create as a `NotApplied`
  `CreateFromRemote` `ReportedItem`, writing nothing.
- Untracked-set computation: `search` results minus the set of local
  `external_id`s. `tracker::ExternalId` is a bare newtype whose `Eq`/`Hash` derive
  over the raw string — there is **no** canonicalisation surface, and `per_id`
  keys on the *local id*, not `ExternalId`. So the difference is computed through
  a **free canonicalising function over `ExternalId::as_str()` living in
  `work-adapters`** (case-fold, trim, normalise project-prefix formatting),
  keeping `tracker`'s public API frozen per Phase 2 §5; a stored id differing only
  cosmetically from a search result folds equal and is excluded. The exact folding
  is defined once and covered by the dedup test. Because `Discovery.found` carries
  only `(ExternalId, RemoteTimestamp)` — no title or body — the path then issues a
  **`show` per untracked id** (bounded by the gate below) to fetch the fields
  needed for assembly. This is one `search` plus one `show` per untracked issue,
  not a single round-trip. A per-issue `show` failure records that issue as
  `Failed` and continues, leaving the remainder for a resumable re-run (created
  files re-classify as tracked), so a transient read failure never aborts the
  whole batch or corrupts the creation count.
- Id allocation (batch, up front — reusing `next-number`'s allocator), full
  frontmatter + body assembly through the shared helper (reusing
  `digest::remote_body` so the normalise pipe is subsumed), and a **create-new
  write implemented as write-to-temp then atomic rename with exclusive
  (`O_EXCL`-equivalent) semantics** — refusing when the destination already
  exists — so the destination is never partial (a crash mid-write leaves no
  stranded half-file) and an id collision (an ignored on-disk file, or a
  concurrent second sync) surfaces as an error instead of a silent clobber.
- **One combined gate, before any write.** The existing pull/push gate is
  evaluated after `compute_plan` (`run.rs:174`); the discovery reads run before
  the plan. So all three sources of write — discovered creates, planned pulls,
  planned pushes — are counted **from reads only**, summed, and checked in a
  single gate that runs **before `discover_untracked` writes any file and before
  the apply loop**. No create write may precede the pull/push refusal, or a
  refused run would leave files on disk while the diagnostic claims nothing
  changed. Creations are counted **by blast direction against the existing two
  bounds**, not a third flag: a create-from-remote writes a new *local* file
  (pull-direction), so its out-of-band discovery count is **added to
  `plan.pull_count()`** (the count cannot come from `plan.create_count()` alone —
  `CreateFromRemote` never enters `plan.actions`) and checked against
  `--max-pulls`; Gap B's create-from-local issues a new *remote* issue via the
  non-idempotent `create` (push-direction), flows through `decide()` into
  `plan.actions`, and counts against `--max-pushes`. This keeps the directional
  model intact — `--max-pushes 0` still guarantees the run touches nothing on the
  remote (it refuses a create-from-local), and the irreversible remote-create is
  bounded by the same budget as any other remote write rather than a separate,
  easily-overlooked knob with its own default. No `--max-creates` flag is added.
  `RunError::Refused` keeps `{ pulls, pushes, max_pulls, max_pushes }` but the
  diagnostic **breaks out how many of each dimension are new creations** (e.g. "12
  pulls, 12 of them new files") so the operator sees the creation blast within the
  dimension that tripped. Discovery is **team/project-scoped by default**
  (`SearchScope` with a set `project`, not `all_projects`) so the untracked set is
  inherently bounded on a shared multi-team workspace; a `complete == false`
  discovery is a refusal-with-guidance, not a limit to raise. The abort diagnostic
  names the count, the limit hit, and the concrete remedy (scope the search or
  raise `--max-pulls`).
- In `RunMode::Preview`, an over-threshold untracked set **reports the count and
  the would-be refusal without issuing the per-issue `show` fan-out** — preview
  neither aborts silently nor bursts an unbounded run of live reads; it names what
  a real run would refuse.

#### 2. Unsynced push create path (Gap B)

**Files**: `cli/work/src/sync/decide.rs`, `cli/work/src/sync/classify.rs`
context, `cli/work-adapters/src/sync/run.rs`, `apply.rs`
**Change**: route the create through `decide()`/`classify`, **not** the keyless
drop at `run.rs:219-225`. That `else` guard is unreachable for a genuinely
keyless item: `classify` returns `SyncState::Unsynced` (`classify.rs:110-111`)
for an item with no `external_id`, and `decide()` maps `Unsynced` to
`Action::Noop` (`decide.rs:80`); `Action::Push` only arises from
`LocallyModified`, which `classify` returns *after* the `external_id.is_none()`
early-return. Editing the Push guard would leave `work create --push` a silent
no-op. Instead, add an `Action::CreateFromLocal` variant produced by `decide()`
for `Unsynced` under `PushOnly`/`Bidirectional`, dispatched by the apply loop
into the shared `apply_create` path. Unlike `CreateFromRemote`, this variant
*does* flow through `decide()` and the apply loop, so wire it through the same
sites — `from_keyword` (whose `_ => return None` arm would silently make a missing
keyword non-round-trippable, uncaught by the compiler), the `Display` impl, and
`action_keyword` (`sync.rs:152`, whose keyword enters the report golden) — plus a
real apply-loop arm (it is dispatched, so no `unreachable!` here). Excluding it
from the non-exhaustive `awaiting_human` `matches!` is a deliberate,
compiler-unenforced choice to state explicitly.

**Failure/recovery invariant** — reuse the existing `pending_push` marker rather
than inventing a weaker bespoke recovery. `create` is non-idempotent
(`lib.rs:265-269`): a single `atomic_write` protects the *local* file but not the
window between a successful remote `create` and a crashed or failed write-back of
the returned `external_id` — and a crash (SIGKILL/OOM) in that window would leave
the item keyless, so the next run re-creates and duplicates the remote issue,
unrecoverable by VCS. `work create --push` already solves this exact window with
`work_adapters::sync::pending_push`: a **durable marker written before the
`create` call**, whose `ReuseId` precondition makes a re-run reuse the
already-created `external_id` (or refuse with a precise `E_PUSH_*` message)
instead of re-creating. Route `apply_create` through the same marker so recovery
is automatic and consistent with `create --push`. On a write-back failure the
diagnostic names the created `external_id` and the exact relink command; a crash
is covered by the marker the next run reads. Persist the id (clearing the marker)
before reporting success.

#### 3. Preview validation loop (Gap C)

**File**: `cli/work-adapters/src/sync/run.rs`
**Change**: replace the `NotApplied`-mapping preview early-return (`:183-197`)
with a `validate_pushes` step that **still reports the full plan** — every
planned action mapped to a `NotApplied` `ReportedItem` as today, preserving the
just-frozen report-format golden — and additionally attaches a `ValidationOutcome`
to each `Push` entry by calling `validate_update`. Reporting only `Push` entries
would drop planned `Pull`/`Prompt`/`CreateFromRemote` items from the preview and
red the golden; the loop must not shrink the report. Because `validate_update` is
now a **local** composition check (Phase 2 decision), the preview stays
network-free — it surfaces locally-detectable omissions (a required field left
empty) before any mutation, but does **not** reproduce the bash `--dry-run`'s
live-tracker field check, which is intentionally dropped. A `Rejected { reasons }`
outcome annotates its push entry without aborting the preview.

### Success Criteria

#### Automated Verification

- [ ] New engine tests fail red before the code and pass after: `mise run
      test:unit:cli` exits 0.
- [ ] A test stages a fixture work item with an uncommitted edit alongside a
      remote-modified counterpart whose pull would apply, and asserts the file's
      bytes are unchanged and the refusal diagnostic emitted (the dirty guard
      survives the engine extension).
- [ ] A test asserts an over-threshold untracked set (exceeding `--max-pulls`)
      aborts with **zero creations** and **zero `show` fetches** when
      non-interactive, and that the refusal diagnostic breaks out the new-file
      creation count within the pull dimension.
- [ ] A test asserts a create-from-local counts against `--max-pushes` (a
      `--max-pushes 0` run refuses it and creates no remote issue), and a combined
      set of create-from-remote plus create-from-local each crossing its
      respective bound trips the gate — so neither dimension is left unbounded.
- [ ] A test asserts a `--push-only` run authors **no** untracked local files
      (the discover_untracked write path is direction-gated).
- [ ] A test asserts a run whose planned pulls/pushes exceed their bound while the
      discovery set is small **refuses before any create-from-remote file is
      written** — the combined gate runs ahead of every write, so a refused run
      leaves the tree unchanged.
- [ ] A report golden includes `CreateFromRemote`/`CreateFromLocal` rows, freezing
      the extended report format (the current golden is create-free) so a
      report consumer sees the widened action-keyword set explicitly.
- [ ] A test asserts the untracked-pull happy path produces **exactly one local
      file per discovered issue** whose allocated id, `external_id`, frontmatter,
      and body match the remote projection (id from the `next-number` allocator,
      body from `digest::remote_body`).
- [ ] A test asserts a create-new write **refuses** when the allocated path
      already exists on disk, rather than overwriting it.
- [ ] A test asserts a keyless local item issues **exactly one** `create` and
      the file is rewritten once with the returned `external_id` (Gap B happy
      path via the create-from-local action, not the unreachable Push guard).
- [ ] A test asserts production code writes the `pending_push` marker **before**
      the `create` call (a fake `create` records marker state when invoked and
      asserts it is present), pinning the marker-before-create ordering the crash
      recovery depends on — not merely recovery from a pre-seeded marker.
- [ ] A test seeds the `pending_push` marker before an injected write-back
      failure and asserts the re-run **reuses** the existing `external_id` (no
      second `create`), matching `create --push`'s recovery.
- [ ] A test asserts a discovery whose results **mix tracked and untracked**
      external ids produces `CreateFromRemote` only for the untracked ones, and a
      stored id differing only cosmetically from a search result is excluded from
      the untracked set (canonicalised comparison).
- [ ] A test injects a `show` failure at position N of the untracked batch and
      asserts issues 1..N-1 are created, issue N is reported `Failed`, the run
      continues, and the pull-dimension creation count is consistent.
- [ ] A test asserts `--preview` performs **no** creation write for discovered
      untracked issues (reports them `NotApplied`) and issues no mutation call,
      and that a mixed plan (push + pull + conflict) still lists every planned
      action in the report with a `ValidationOutcome` on the push entries only.
- [ ] A test asserts `validate_update` in the preview loop returns
      `Rejected { reasons }` for a locally-missing required field and makes no
      remote call.
- [ ] `mise run cli:check` and `mise run check` exit 0.

#### Manual Verification

- [ ] `--preview` surfaces a payload with a locally-detectable missing required
      field before any mutation (the live-tracker field check is intentionally
      dropped — see Phase 2).
- [ ] The seams expose the hooks 0213 needs (a `Prompt`/create decision surfaced
      without an interactive call inside the engine).

---

## Phase 4: The `work list` command

### Overview

Add a `work list` subcommand that scans, filters, and renders work items with
their sync-status column, reusing the already-ported label vocabulary,
classifier, baseline reads, and bulk fetch. Independent of Phases 2–3 (it uses
the existing `fetch_all`); placed before the repoint that consumes it.

### Changes Required

#### 1. New subcommand

**File**: `cli/work-cli/src/cli.rs`
**Change**: a `List` variant on `Command` with a scan-root flag and an
**enumerated** filter-flag surface, since `list-work-items` has no flags today —
it parses a free-text expression in-prompt (`tagged X`, `under X`, `status X`,
multi-token shorthand like `bugs in review`, free-text title search). Each skill
filter rule maps onto a concrete flag: `--status`, `--kind`, `--priority`,
`--parent`, `--tag` (repeatable), and a positional title-substring term. Phase 5
translates the skill's natural-language parse into these flags; the natural-
language parsing itself stays in the skill. Give the `List` variant a doc comment
matching the surrounding subcommand convention — a purpose line plus a per-flag
description for each of `--status`/`--kind`/`--priority`/`--parent`/`--tag`/the
positional term — before the surface golden is frozen, so `accelerator work list
--help` is as informative as every sibling command.

#### 2. Scan + filter + render

**Files**: `cli/work-cli/src/` (new module), reusing
`cli/work/src/sync/label.rs`, `decide.rs`, `digest.rs`,
`cli/work-adapters/src/sync/baseline_store.rs`, `fetch.rs`
**Change**: render the **five-state** vocabulary users already see in
`list-work-items` — `🟢 synced`, `⚪ unsynced`, `🔵 locally modified`,
`🟣 remotely modified`, `🔴 conflict` — with the existing glyph+text mapping
preserved (your decision), so the column stays recognisable and the actionable
`conflict` signal survives. Classify each scanned item by routing its
(baseline × remote) inputs through the existing `SyncState`/`decide` classifier
and treating the five rendered states purely as a presentation mapping over that
single source of truth — no second, drift-prone classifier grown for the column.

Preserve the skill's actionable messaging: the directory-not-found guidance
(`Work items directory "{work_dir}" not found. Check the paths.work
configuration or run /create-work-item`), the per-file `skipped — no frontmatter`
/ `unclosed frontmatter` warnings, and the empty-result message.

**Degradation contract** — when the bulk `fetch_all` returns all-`indeterminate`
or a transport failure (remote unreachable), `work list` renders **presence-only
for every item and exits 0** without retrying or hanging, matching the skill's
current fallback (`list-work-items/SKILL.md:309-313`); it does not surface the
outage as a non-zero exit.

#### 3. Freeze the surface

**File**: `cli/work-cli/tests/fixtures/cli_surface.golden` and `cli_surface.rs`
**Change**: regenerate the golden to include `list`. This trips only the surface
freeze, not the thirteen-point sub-binary checklist (no new `[[bin]]`).

### Success Criteria

#### Automated Verification

- [ ] `cli_surface.golden` includes `list` and `mise run test:unit:cli` exits 0.
- [ ] A test renders a fixture corpus with all **five** status states (including
      `conflict`) and asserts the rendered glyph+text column matches the skill's
      current mapping.
- [ ] Tests assert each filter flag selects the expected subset from a fixture
      corpus — `--status`/`--kind`/`--priority`/`--parent`, a repeatable `--tag`
      composing, a positional title substring — plus a multi-flag combination and
      an empty-match case.
- [ ] A test asserts `work list` renders presence-only and **exits 0** when the
      bulk fetch returns all-`indeterminate` / a transport failure (the skill's
      degradation contract).
- [ ] A test asserts the directory-not-found, malformed-frontmatter, and
      empty-result messaging matches the skill's current wording.
- [ ] `mise run cli:check` and `mise run check` exit 0.

#### Manual Verification

- [ ] `accelerator work list` output matches `list-work-items`' current
      rendering on a real corpus (spot-check ahead/behind).

---

## Phase 5: Repoint the skills and fold EXIT_CODES.md

### Overview

Point `sync-work-items`, `create-work-item`, and `list-work-items` at
`accelerator work …` for every flow they previously shelled out to, move the
exit-code table into `exit_codes.rs`'s module doc, and regenerate the docs-site
mirror. After this phase no SKILL references a work-item script; the scripts
remain on disk, now unreferenced.

### Changes Required

#### 1. Repoint `sync-work-items/SKILL.md`

**File**: `skills/work/sync-work-items/SKILL.md`
**Changes**: replace every `${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*`
invocation with `accelerator work sync` (and its flags), preserving the
dirty-guard precondition point (`:137`) by pointing it at the CLI surface that
carries it. Drop the `skills/work/scripts/*` line from `allowed-tools`, and drop
`jq`/`curl` from this skill's `allowed-tools`. Keep the conflict-resolution and
pull-overwrite prose stable for 0213's rebase. Because Phase 3 gives `work sync`
two side-effects it never had — creating remote issues from unsynced local
drafts (bounded by `--max-pushes`) and pulling untracked remotes into new local
files (bounded by `--max-pulls`) — update the `Sync` subcommand `--help`
(`cli.rs:75-89`) to state that a run may author new remote and local artefacts,
so a developer invoking the CLI directly is not surprised by the widened blast
radius.

#### 2. Repoint `create-work-item/SKILL.md` and `list-work-items/SKILL.md`

**Files**: `skills/work/create-work-item/SKILL.md`,
`skills/work/list-work-items/SKILL.md`
**Changes**: `create` → `work create --push` for the remote push
(`:502/532/542`); the whole scan/filter/render machinery (`list-work-items`
Steps 2–4: the single-pass `awk` frontmatter extraction, filter application, and
table/hierarchy render) moves behind `work list`, with only the natural-language
filter parse staying in-prompt and translating into the enumerated flags. Remove
`jq`/`curl` and the `scripts/*` line from both `allowed-tools`.

The create skill consumes a precise stdout+exit contract from the deleted create
bridge: `--dry-run` emits `jira\t<issue type>\t<type source>\t<project>\t<project
source>` which the skill parses field-by-field (`:506-510`), and the push path
feeds the dispatcher exit code plus attempt number into the `work-item-push-
decide.sh` retry seam that maps `70`/`71`+attempt to a keyword (`:537-554`). Pin
the replacement surface with a golden reproducing the five fields the skill
parses (the `CreatePreview` rendering — `FieldResolution` for both project and
issue-type, so each value *and* its source annotation is recoverable, including
the `Unresolvable` case). The exit-70-then-retry / exit-71-terminal decision
lands in **`work create --push`'s own retry handling** — the CLI is its
authoritative home, its `--help` documents the decision, and the SKILL prose
cross-references it rather than restating the outcome table — with a behavioural
test asserting a `70` outcome retries (bounded by attempt) and a `71` outcome
terminates. Note the behavioural divergence recorded in 0171: `preview_create` is
now a credentialed round-trip, so the skill's pre-create gate must degrade
sensibly on a transport failure (distinct from an `Unresolvable` key), not treat
every non-`Resolved` result as a hard block. Freeze the **Linear-branch** preview
rendering too (a sibling golden case), not only the Jira five-field line, since
the create skill parses the two providers' preview output differently
(`create-work-item/SKILL.md:506-513`) and the Linear branch would otherwise drift
unguarded. State in the create/update skills and the preview report that update
validation is now **local-only** — a clean preview no longer guarantees a
successful push, since a tracker-side field rejection surfaces only at apply.

#### 2b. Drop the stale `scripts/*` glob from the two untouched work skills

**Files**: `skills/work/review-work-item/SKILL.md`,
`skills/work/extract-work-items/SKILL.md`
**Change**: both declare `Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)` in
`allowed-tools` (`review-work-item:11`, `extract-work-items:13`) yet neither
invokes a work-item script. Remove the glob from both (and their regenerated
docs-site mirrors) so Phase 6's repo-wide `skills/work/scripts` sweep can reach
empty — without this the Desired End State is unreachable.

#### 3. Fold and re-site EXIT_CODES.md

**Files**: `cli/work-cli/src/exit_codes.rs`, `skills/work/scripts/EXIT_CODES.md`
**Change**: move the human-readable table into `exit_codes.rs`'s module doc
(option b, per your decision). `EXIT_CODES.md` is not one table — it documents
deleted bash scripts (the `work-item-push-decide.sh` action keywords, the
per-integration native-code→taxonomy mappings) and omits the 0–5 codes
`exit_codes.rs` already defines. **Fold only the numeric taxonomy the binary
actually emits**, describing each band accurately rather than attributing all of
it to the `TrackerError` split:

- **70/71** are the `TrackerError` two-class split (`for_tracker_error` yields
  `Retryable → 70`, `Terminal → 71`).
- **0–5** (`CLEAN`/`ERROR`/`USAGE`/`RESOLVE_NOT_FOUND`/`UNRESOLVED`/
  `REFUSED_BULK_OVERWRITE`) are separately-emitted process/selection codes.
- **72–74** (`NOT_AVAILABLE`/`UNRECOGNISED`/`UNCONFIGURED`) are emitted from
  `SelectionError` (`create.rs:338-341`, `main.rs:281-289`, `sync.rs:269-277`),
  **not** from `TrackerError` — document them as tracker selection/configuration
  failures, not a tracker-error class.

If dropping the per-integration native-code→taxonomy tables removes the only
map from a provider's native failure to a surfaced `71`/`74`, confirm that trace
stays discoverable — e.g. the diagnostic text names the provider condition —
before deleting it wholesale.

Carry the **safety-critical semantics** the current `.md` holds — into the doc,
since the work skills branch on exactly this retryable/terminal distinction
(`create-work-item/SKILL.md:509,553`). Capture **both** framings of 71: the
non-idempotent-`create` hazard (a repeat double-applies) *and* the distinct
update-path rationale the source records (`EXIT_CODES.md:83` — a whole-item update
is idempotent, so there the hazard is response *uncertainty*, not double-apply),
so the terminal-code rationale is accurate for every path that emits 71. Also
carry what 74 means for reconciliation ("nothing was sent — save locally, never
reconcile") and the read-bridge degradation rule. Drop only the bash-only action
keywords and per-integration native-code tables that describe deleted scripts.

The `exit_codes.rs` **module doc is the single authoritative source**; the
subcommands' `--help` text is derived from (or cross-references) it rather than
independently restating the table, so the two runtime-discoverable surfaces
cannot drift — the same duplication hazard the project already carries for the
hand-copied 80-column width. Prefer rendering `--help` band descriptions from
per-constant doc/description; if that is over-engineering for this small table,
nominate the module doc authoritative in a note and have `--help` reference it. The `.md` is deleted in Phase 6 with the directory; its
only non-`meta/` consumer (`work-item-create-remote.sh:34`) is also deleted
there.

#### 4. Regenerate the docs-site mirror

**Files**: `docs-site/src/content/docs/reference/skills/work/*.md`
**Change**: regenerate from the repointed SKILL.md sources with **`mise run
docs:generate`** (the `generate_pages` renderer that mirrors each SKILL.md's
`allowed-tools` and invocation blocks), then run `mise run docs:check`.
`docs:check` alone only verifies a page exists per skill and no orphans — it
never compares mirror *content* to source, so a stale mirror still naming
`work-item-*.sh` would pass it. The reference-cleared check is therefore a
**direct grep** over `docs-site/src/content/docs/reference/skills/work/` for
`skills/work/scripts` and the eighteen basenames, whose empty output is the
recorded evidence — not reliance on `docs:check`. (The prior "38 mirrored
references" figure was unverifiable; the grep's empty result is the concrete
gate.)

### Success Criteria

#### Automated Verification

- [ ] No SKILL references a work-item script: `! grep -rl
      "skills/work/scripts" skills/work/` returns nothing — including
      `review-work-item` and `extract-work-items`, whose stale `allowed-tools`
      glob is removed here.
- [ ] No work `SKILL.md` declares `jq` or `curl` in `allowed-tools`.
- [ ] A test asserts a **transport-failure** `preview_create` result does not
      block local creation (the pre-create gate degrades to allow the create),
      while an `Unresolvable` key does block — so the now-credentialed pre-flight
      does not regress the offline `/create-work-item` path.
- [ ] `mise run check` exits 0.
- [ ] The mirror is regenerated via `mise run docs:generate`, then `mise run
      docs:check` exits 0, and a direct grep over
      `docs-site/src/content/docs/reference/skills/work/` for `skills/work/
      scripts` and the eighteen basenames returns nothing.

#### Manual Verification

- [ ] `/sync-work-items --preview`, `/create-work-item`, and `/list-work-items`
      each run end-to-end against a local corpus with the repointed invocations.
- [ ] `exit_codes.rs`'s module doc carries the table the `.md` held, with each
      documented integer matching the value the CLI emits.

---

## Phase 6: The irreversible deletions

### Overview

Delete all eighteen scripts, the `E_DISPATCH` bash constants and their fixture,
the build-system floor for the work suites, and the empty `skills/work/scripts/`
directory. Everything removed here is unreferenced after Phases 1–5. This is the
point of no return; it merges only with a fully green tree.

### Changes Required

#### 1. Delete the eighteen scripts and the dispatch fixture

**Files**: all thirteen `work-item-*.sh` production scripts, the five
`test-work-item-*.sh` suites, `EXIT_CODES.md`,
`cli/tracker/tests/fixtures/dispatch-codes.txt`
**Change**: delete. Then remove the now-empty `skills/work/scripts/` directory.

#### 2. Repoint the `E_DISPATCH` parity test off the deleted oracle

**Files**: `cli/work-cli/tests/exit_codes_parity.rs`,
`cli/tracker/tests/errors.rs`,
`cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt`
**Change**: `exit_codes_parity.rs` reads the deleted `work-item-bridge-codes.sh`
as its oracle. Repoint it to an **independent frozen expectation** — literal
integers pinned in the test (or a committed golden table) for the 70/71 and
72–74 codes — **not** to the `exit_codes.rs` constants it guards, which would be a
tautological self-comparison that no accidental renumbering could red. These
integers are a live contract the work skills branch on (`create-work-item/
SKILL.md:509,553`), so the guard must fail on a changed value. Remove the
`dispatch-codes.txt` reference from `errors.rs` and the bash-file reference at
`bridge-exit-code-tables.txt:16`. Leave `tracker` as the single owner of the
numeric taxonomy.

#### 3. Remove the build-system floor and library entry

**Files**: `tasks/test/integration.py`, `tasks/lint/scripts.py`
**Changes**: remove `_EXPECTED_WORK_SUITES` (`:51`) and the `work` task's
`_require_suite_floor` call (`:397-401`) — keep the shared helper, which backs
five other floors. Remove the `work-item-bridge-codes.sh` entry from
`SHELL_LIBRARIES` (`:34`) in the same change so the stale-entry guard
(`:107-112`) does not trip.

#### 4. Scrub the tracker doc references

**File**: `cli/tracker/src/lib.rs`
**Change**: the four named comments are already provider-neutral per the
research; verify none names a deleted bash artefact and adjust only if the
Phase 2 additions reintroduced one.

### Success Criteria

#### Automated Verification

- [ ] `ls skills/work/scripts/*.sh` matches nothing and the directory is gone.
- [ ] The consumer sweep is empty — for each of the eighteen basenames and for
      `skills/work/scripts`, `grep -r --exclude-dir=meta` over the repo returns
      no hits (covering `hooks/`, `templates/`, `docs-site/`, `tasks/`, agents).
      The grep command and its empty output are the recorded result.
- [ ] `grep -rl "skills/work/scripts" cli/` returns nothing.
- [ ] `tasks/lint/scripts.py` no longer names `work-item-bridge-codes.sh`;
      `tests/unit/tasks/test_exec_bits.py` passes.
- [ ] `mise run` (the bare default task) exits 0 end-to-end.

#### Manual Verification

- [ ] The `E_DISPATCH` taxonomy has one implementation; no surviving script or
      Rust test sources the removed bash definition.

---

## Phase 7: Credentialed corpus classification (manual, pre-merge)

### Overview

Verify `accelerator work sync` against live Jira and Linear scratch tenants:
seed one remote issue per relocated corpus record, run the sync, and confirm
every item classifies as `synced` with neither a push nor a pull issued. Manual,
developer-run — no CI job. This exercises Phase 3's engine and Phase 2's Jira
`preview_create` remote check against real systems.

### Changes Required

#### 1. The seed step

**Files**: a developer-run harness under `cli/work-adapters/tests/` (contract
profile) or a documented `mise` recipe
**Change**: create one remote issue per relocated corpus record on the scratch
project/team, reusing the production token vars `ACCELERATOR_JIRA_TOKEN` /
`ACCELERATOR_LINEAR_TOKEN` (no new var names). The existing contract harness only
reads two unmatched ids; the create/seed step is new machinery.

The seed writes real issues to a live tracker, so guard it: fence the step on an
**explicit allowlist of known scratch project keys / team ids** (or a required
scratch-only tenant marker) and assert membership before creating anything —
checking that an identifier was *supplied* is not the same as checking it is
*not production*, so a mistyped production key must be rejected, not accepted.
Make the seed **idempotent** (reuse existing seeded issues by a stable marker, or
provide a teardown) so a repeated run does not accumulate duplicates. Both the
allowlist predicate and the idempotency marker are pure/offline logic — cover
them with **offline unit tests** (rejects a non-scratch key, accepts a scratch
key; a second seed reuses rather than duplicates) independent of live
credentials, so a refactor cannot silently disable the production-write guard.

### Success Criteria

#### Automated Verification

- [ ] `mise run test:integration:tracker-contract` exits 0 with the contract
      profile, `ACCELERATOR_TRACKER_CONTRACT=1`, and resolved credentials (all
      three gates required; none skips).

#### Manual Verification

- [ ] With the corpus seeded, `accelerator work sync` classifies every seeded
      item `synced`, issues no push and no pull, and the exercised set includes at
      least one absent-description item per provider.
- [ ] `/sync-work-items` lists a remote issue with no matching local work item
      (untracked discovery), and `/sync-work-items --preview` surfaces an
      unresolvable Jira project key and a payload missing a required field before
      any mutation, each emitting its named diagnostic.
- [ ] A red run traced to reachability, rate limits, or Linear's query-complexity
      cap is recorded as such, not as a defect in this change.

---

## Testing Strategy

### Unit Tests

- Port contract properties for each new operation (Phase 2), red before the
  client implementations.
- Engine tests for untracked pull, unsynced create, and preview validation
  (Phase 3), each with the dirty-guard and blast-radius edge cases.
- `work list` rendering across all five status states including `conflict`, its
  enumerated filter flags (each flag and a multi-flag combination selecting the
  expected subset, plus an empty-match and repeatable-`--tag` case), and the
  degradation-exit-0 path (Phase 4).

### Integration Tests

- The converted parity tests reading relocated in-crate goldens (Phase 1),
  byte-identical to the committed 0210 oracle.
- `cli_surface.golden` and `public-api.txt` freezes (Phases 2, 4).
- The credentialed contract run against live tenants (Phase 7).

### Manual Testing Steps

1. Run each repointed skill (`/sync-work-items --preview`, `/create-work-item`,
   `/list-work-items`) against a local corpus (Phase 5).
2. Seed the scratch tenants and run the live classification (Phase 7).
3. Exercise the two `--preview` diagnostics and untracked discovery against the
   live tenants (Phase 7).

## Performance Considerations

`work list`'s status column issues a bulk `fetch_all` over the scanned corpus;
on a large work directory this is one remote round-trip, matching the bash
column's cost. No per-item reads are introduced. The untracked-pull path is **one
`search` plus one `show` per untracked issue** — `Discovery` returns only ids and
stamps, so assembling a local file requires fetching each discovered issue's
fields — not a single round-trip; the per-`show` fan-out is bounded by the same
blast-radius gate (team/project-scoped, refusing rather than fetching an
unbounded set) that guards the creations.

## Migration Notes

The deletions in Phase 6 are irreversible and recoverable only through VCS. Each
prior phase merges with a green build, so a partial cutover never leaves the tree
broken. The credentialed target (scratch Jira project, Linear team, tokens) is a
prerequisite 0171 owns; an unprovisioned target strands Phase 7's verification
but does not block Phases 1–6.

## References

- Work item: `meta/work/0212-work-item-script-cutover.md`
- Research: `meta/research/codebase/2026-08-19-0212-work-item-script-cutover.md`
- Parent: `meta/work/0171-jira-and-linear-integrations.md`
- Blocked by: `meta/work/0210-provider-client-crates-over-the-tracker-port.md`
- Blocks: `meta/work/0211-integration-binaries-and-bash-cluster-retirement.md`,
  `meta/work/0174-*.md`
- Shares `sync-work-items/SKILL.md` with: `meta/work/0213-*.md`
- Port trait: `cli/tracker/src/lib.rs:238-342`
- Sync engine: `cli/work-adapters/src/sync/run.rs`, `fetch.rs`
- Decision table: `cli/work/src/sync/decide.rs`
- CLI surface: `cli/work-cli/src/cli.rs`, `exit_codes.rs`
- Corpus oracle: `cli/work-adapters/tests/fixtures/bash-parity-baseline.txt`
