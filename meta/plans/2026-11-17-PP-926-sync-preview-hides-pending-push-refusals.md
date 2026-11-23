---
type: "plan"
id: "2026-11-17-PP-926-sync-preview-hides-pending-push-refusals"
title: "Sync Preview Reports Pending-Push Refusals Implementation Plan"
date: "2026-11-17T14:14:34+00:00"
author: "Phil Helm"
producer: "create-plan"
status: "draft"
work_item_id: "work-item:PP-926"
parent: "work-item:PP-926"
relates_to: ["work-item:PP-884", "work-item:PP-927"]
tags: ["sync", "preview", "pending-push"]
revision: "4626fa4798b7f965019a98aea5a60187f0a2e423"
repository: "accelerator"
last_updated: "2026-11-17T14:33:59+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# Sync Preview Reports Pending-Push Refusals Implementation Plan

## Overview

`accelerator work sync --preview` reports every create-from-local as not
applied without reading its pending-push marker, so it shows a clean create for
an item the run will refuse. This plan moves the run's decision for a bare
create into one function that the run and the preview both call. The preview
then reports the run's failure in the run's words and returns the run's exit
code. After that, the `/sync-work-items` skill's manual workaround is retired.

The work lands as one PR, in three phases. Each phase leaves the full suite
green.

## Current State Analysis

- A bare create is an unsynced item planned as a `Noop` with no `--resolve`
  order (`create.planned.order().is_none()`). Only this path calls
  `push_precondition` directly.
- A create with a `--resolve` order, and a relink, go through `settle_order`.
  Its `order_precondition` calls `push_precondition`
  (`cli/work/src/sync/push_precondition.rs:206`), and it can refuse from local
  state alone: `AlreadyWritten`, `KeyCarried`, or an adopt while the corpus is
  incomplete. The preview does not check these. `refused_orders`
  (`run.rs:1383-1405`) covers only orders that reach no create, and
  `previewed_adoptions` (`run.rs:1315-1341`) only consults the tracker.
- The run's decision lives in `ItemApplier::create_from_local_steps`
  (`cli/work-adapters/src/sync/apply.rs:1157-1212`):
  1. Compute the request digest.
  2. Run `self.read_marker(request)`.
  3. Run `push_precondition`.
  4. Act on the result:
     - Refuse: build `ApplyError::PendingPushRefused`.
     - ReuseId while the corpus is incomplete: build
       `ApplyError::CorpusIncomplete`.
     - Otherwise: link or create.

  Nothing is written before the decision.
- `read_marker` (`apply.rs:1216-1257`) uses only `std::fs` and
  `self.files.tracking`. It distrusts a marker that version control tracks or
  that records a key the tracker could not have issued, and it maps both to
  `marker-unreadable`. It has two callers: `create_from_local_steps`
  (`apply.rs:1168`) and the adopt arm of `relink_steps` (`apply.rs:1143`).
  `settle_order` receives the reading as an argument.
- `apply_local_create` (`cli/work-adapters/src/sync/run.rs:1749-1805`) checks
  inputs before any marker is read:
  - `create_inputs` (`run.rs:1440-1465`). Undecodable bytes give
    `Failed(LocalUnparseable)` on `ReadLocal`, an unparseable frontmatter
    block gives it on `ParseLocal`, and any other read error gives a retryable
    `Io`. A missing `title` or `kind` reads as empty and does not fail.
  - `CreateStatePaths::for_item` (`run.rs:1727-1747`). An id that cannot name
    a marker gives `Io` on `MarkerWrite`.
- The preview branch (`run.rs:2370-2411`) does
  `reported.push(create.report(ItemOutcome::NotApplied))` for every create. It
  has in scope everything the decision needs:
  - `request` (corpus and inventory)
  - `settled`
  - `ports.files.tracking`
  - `ports.author`
  - `run_start_epoch`
- `validate_pushes` (`run.rs:1000-1037`) is the existing pattern for a preview
  failure. It runs the same gate the run uses and reports the gate's
  `ApplyError` as `ItemOutcome::Failed`.
- The CLI renders preview and apply through the same code
  (`cli/work-cli/src/sync.rs:570-616, 899-908, 974-987`). A preview
  `Failed(PendingPushRefused)` row therefore already prints
  `<id>\tfailed\tunsynced\tterminal` and
  `error: <id>: create would fail: pending push refused (<reason>): …`, and
  exits 71. The CLI needs no change.
- At CLI level, an undecodable work file is an unidentified inventory entry
  (`work::inventory::identify`) and never becomes a create. A file with an
  `id` but an unclosed frontmatter fence does become a create, and
  `create_inputs` fails it on `ParseLocal`.
- The workaround sentence "A preview does not yet read a create's pending-push
  marker" appears in three places:
  - the skill (`skills/work/sync-work-items/SKILL.md:263-270`)
  - the `work sync` help comment (`cli/work-cli/src/cli.rs:156-157`)
  - the CLI surface golden (`cli/work-cli/tests/fixtures/cli_surface.golden:211`)

  `tests/unit/tasks/test_sync_work_items_skill_structure.py:339-349` pins the
  skill's copy. The skill describes preview coverage twice: in Step 2
  (lines 240-243) and under "Report the outcome" (lines 760-766).

## Desired End State

For every bare create, the preview reports exactly what the run would report
for that item: the `failed` row, the error line (with `failed` read as `would
fail`), and the exit code. The preview never touches `pending-push/`. With no
marker, or with a reusable `Created` marker over a complete corpus, the preview
output is unchanged. Creates with a `--resolve` order and relinks preview
exactly as today. The skill, the CLI help, the user guide and the changelog
describe the new behaviour.

Verify with `mise run` (the full default task) exiting 0, plus the manual
reproduction below.

### Key Discoveries:

- `SettledView` is built from `plan.key_changes` in both modes
  (`cli/work-adapters/src/sync/identity_settlement.rs:784-790`). That means
  `carrier_of(request.corpus, settled, key)` (`run.rs:1188-1205`) gives the
  same `FollowedThisRun` answer in the preview.
- A whole-corpus preview with an `Attempted` marker already reports the
  discovery deferral and exits 70 (`run.rs:2014-2074`). After the fix, the
  row's terminal failure outranks it and the preview exits 71, as the run does.
- `ItemAttempt::finish` leaves `PendingPushRefused` and `CorpusIncomplete`
  unchanged, because nothing has landed (`attempt.rs:189-230`). The error the
  decision returns is therefore exactly what the run reports.
- A bare create's item has no `external_id`, so the run's leftover-marker
  removal (`leftover_attempts`, `run.rs:2413`) never reaches its marker. The
  preview can skip that step without splitting the modes.
- A bare create row's dossier shape is `NotInConflict` whatever its outcome
  (`cli/work-adapters/src/sync/dossiers.rs:93`). A `Failed` outcome touches no
  dossier.
- At engine level, `run_ordered` with `OrderedRun { mode: RunMode::Preview, ..
  }` (`cli/work-adapters/tests/sync_create.rs:3409-3487`) can set the mode,
  unusable files and marker tracking. Its `integration_state` port is the
  in-memory `Spy`, so a write shows in `fixture.spy.writes`, not on disk.
  `each_pending_push_refusal_is_terminal_and_names_its_reason`
  (`sync_create.rs:3245-3327`) is the table to mirror.

## What We're NOT Doing

- Changing what creates with a `--resolve` order or relinks preview. A
  relink's `Failed` outcome would change its dossier, and an order's local
  refusals belong with 0379's per-entry preview strategy. Until then, a create
  with a `--resolve` order is checked against its marker only on the run, and
  the skill says so.
- Running `create_inputs` in the preview for ordered creates or relinks.
- Changing the action word or column 4 for a reused `Created` marker. The
  preview and the run already agree on it.
- The `#\tdiscovery\tskipped\tpending-push` record. It is built while the run
  is prepared, which the preview and the run share.
- Any change to CLI rendering or exit-code derivation.
- Folding the decision into 0379's per-entry preview strategy. If 0379 lands
  first, the same function moves there.
- Pinning the `work sync` stdout row contract. 0377 does that on PR #224, and
  whichever of 0377 and 0351 lands second updates the other's preview
  expectations (see Phase 3's pre-merge check).

## Implementation Approach

1. Lift the decision out of `ItemApplier` into a free function over a marker
   reading made at the call site. Lift the input gathering out of
   `apply_local_create` into a type both paths build. This is a refactor that
   the existing `sync_create.rs` tests protect.
2. Make the preview call that function for each bare create, test first: the
   failing engine-level and CLI-level tests come before the change.
3. Retire the workaround text and record the fix.

The decision takes the whole `CreateFromLocalRequest`, although it reads only
the item id, marker path, tracker name, `carrier_of`, `corpus_blocked_by` and
the digest inputs. This is deliberate: both paths build the request through
`LocalCreateInputs`, so `corpus_blocked_by` and `tracker_name` cannot drift
between them. The request still carries the `author` port, so nothing in the
types stops the decision writing: it holds because the decision calls only
`carrier_of`, and the preview passes it only `author` and `tracking` rather
than every port. The CLI tests' `pending-push/` snapshots and the engine
tests' send assertions pin it.

---

## Phase 1: One Decision for a Bare Create

### Overview

This phase is a pure refactor with no behaviour change. The run's marker
reading, its decision, and the gathering of its inputs become callable without
an `ItemApplier`.

### Changes Required:

#### 1. Marker reading and the bare-create decision

**File**: `cli/work-adapters/src/sync/apply.rs`
**Changes**:
- Turn `ItemApplier::read_marker` into `MarkerReading::read(request,
  tracking)`. The body is the same except that it reads the tracking port from
  its argument. Update both callers (`create_from_local_steps` and the adopt
  arm of `relink_steps`) to pass `self.files.tracking`.
- Add `CreateFromLocalRequest::digest()`, and use it wherever the request
  digest is computed today (`create_from_local_steps`,
  `create_marking_progress`).
- Extract the `push_precondition` match from `create_from_local_steps` into
  `bare_create_decision`, a pure function of the request and the marker
  reading:
  - Refusals and the incomplete-corpus deferral come back as `Err`, built
    exactly as today.
  - `create_from_local_steps` reads the marker once, before it branches on
    the order, and matches on the decision to link or to create.

```rust
pub(crate) enum BareCreateDecision {
    Proceed,
    ReuseId(ExternalId),
}

impl MarkerReading {
    pub(crate) fn read(
        request: &CreateFromLocalRequest<'_>,
        tracking: &dyn RepositoryTracking,
    ) -> Self { … }
}

pub(crate) fn bare_create_decision(
    request: &CreateFromLocalRequest<'_>,
    marker: &MarkerReading,
) -> Result<BareCreateDecision, ApplyError> {
    let is_carried = |key: &ExternalId| (request.carrier_of)(key).is_some();
    match push_precondition(&marker.state(), &request.digest(), &is_carried) {
        PushPrecondition::Refuse(reason) => {
            Err(pending_push_refused(request, marker, reason))
        }
        PushPrecondition::ReuseId(key) => match request.corpus_blocked_by {
            Some(blocking) => Err(ApplyError::CorpusIncomplete {
                subject: request.item_id.to_owned(),
                deferred: IncompleteCorpusDeferral::ReuseId(key),
                blocking,
            }),
            None => Ok(BareCreateDecision::ReuseId(key)),
        },
        PushPrecondition::Proceed => Ok(BareCreateDecision::Proceed),
    }
}
```

`pending_push_refused` is the existing Refuse arm (`apply.rs:1174-1193`),
moved into its own function unchanged.

`create_from_local_steps` becomes:

```rust
let marker = MarkerReading::read(request, self.files.tracking);
if let Some(order) = request.order {
    let digest = request.digest();
    return self.settle_order(request, attempt, &marker, digest, order);
}
match bare_create_decision(request, &marker)? {
    BareCreateDecision::ReuseId(key) => self
        .link_and_baseline(request, attempt, &key)
        .map(CreatedLocally::from),
    BareCreateDecision::Proceed => self
        .create_marking_progress(request, attempt)
        .map(CreatedLocally::from),
}
```

#### 2. Inputs a create-from-local is built from

**File**: `cli/work-adapters/src/sync/run.rs`
**Changes**:
- Add `LocalCreate::is_bare()` beside `relinks()` and `adopts()`: true when
  the planned action carries no order.
- Introduce `LocalCreateInputs` to own the title, body, kind and state paths.
  - `gather` performs today's `create_inputs` read and the
    `CreateStatePaths::for_item` check, with today's failures.
  - `create_request` borrows these inputs into a `CreateFromLocalRequest`.
  - `apply_local_create` uses both. Its result is unchanged.

```rust
struct LocalCreateInputs {
    title: String,
    body: String,
    kind: String,
    paths: CreateStatePaths,
}

impl LocalCreateInputs {
    fn gather(
        create: &LocalCreate<'_>,
        request: &SyncRequest<'_>,
    ) -> Result<Self, ApplyError> { … }

    fn create_request<'a>(
        &'a self,
        create: &'a LocalCreate<'_>,
        sync: &'a SyncRequest<'_>,
        author: &'a dyn LocalAuthor,
        carrier_of: &'a dyn Fn(&ExternalId) -> Option<KeyCarrier>,
        attempted_at: u64,
    ) -> CreateFromLocalRequest<'a> { … }
}
```

### Success Criteria:

#### Automated Verification:

- [x] The existing create tests pass unchanged:
      `cargo nextest run --manifest-path cli/Cargo.toml -p work-adapters --test sync_create`
- [x] Everything else in `work-adapters` passes:
      `cargo nextest run --manifest-path cli/Cargo.toml -p work-adapters`
- [x] The CLI's pending-push tests pass:
      `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-work --bin accelerator-work pending_push`
- [x] Format and lint are clean: `mise run cli:check`

#### Manual Verification:

- [x] The diff contains no behaviour change: every change to an existing test
      is a rename forced by moved code, not a changed assertion.

---

## Phase 2: The Preview Evaluates Each Bare Create

### Overview

For each bare create, the preview gathers the create's inputs, reads its
marker and calls `bare_create_decision`. It reports an `Err` as the failed
row, and anything else as not applied. The tests below come first. Red tests
fail until the preview branch changes. Guard tests pass throughout and protect
the paths that must not change.

### Changes Required:

#### 1. Engine-level tests

**File**: `cli/work-adapters/tests/sync_create.rs`
**Changes**: Add tests that run `run_ordered` with
`OrderedRun { mode: RunMode::Preview, .. }`. Each test asserts:
- the reported outcome;
- `assert_nothing_sent` (no create call, no link);
- no path under `pending-push/` in `fixture.spy.writes` or
  `fixture.spy.removals`.

Red tests:
- `a_preview_reports_each_pending_push_refusal_the_run_would`: the same table
  as `each_pending_push_refusal_is_terminal_and_names_its_reason`
  (`prior-attempt-unknown`, `marker-unreadable`, `fingerprint-mismatch`,
  `already-written` carried by another item). It asserts
  `ItemOutcome::Failed(ApplyError::PendingPushRefused { reason, .. })` with
  the run's token and detail.
- `a_preview_distrusts_a_marker_as_the_run_does`: loops `marker_tracking`
  over `FileTracking::Tracked` and `FileTracking::Unknown`, as
  `a_tracked_marker_is_unreadable` does, and adds a `Created` key that is not a
  tracker key (`seed_created_marker_keyed_raw`). Each gives
  `marker-unreadable` with the run's distrust cause.
- `a_preview_defers_a_reuse_while_the_corpus_is_incomplete`: mirrors
  `reuse_id_is_deferred_while_the_corpus_is_incomplete` over both blocker
  reasons, with the class taken from the blocker.
- `a_preview_reports_an_unusable_create_the_run_would`: mirrors
  `a_local_create_over_wrong_content_is_unparseable_at_the_step_that_saw_it`.
  Undecodable bytes give `LocalUnparseable` on `ReadLocal`, and an unclosed
  fence (`---\nid: "0001"\ntitle: Draft\n`) gives it on `ParseLocal`. Each
  asserts `class() == Unparseable` and the exact `operation()`. It also covers
  the unsafe id from `an_unsafe_id_writes_no_marker`, which gives `Io` on
  `MarkerWrite` with "id cannot name a pending-push marker".

Guard tests:
- `a_previewed_reusable_marker_reports_no_failure`: a matching `Created`
  marker over a complete corpus gives `NotApplied`, with no Spy write or
  removal.
- `a_preview_leaves_an_ordered_create_to_its_order`: an `Attempted` marker,
  and separately a digest-mismatched `Created` marker, under an
  `adopt:<KEY>` order, a `create` order and a relink order. Each previews as
  it does today: the row is not `Failed(PendingPushRefused)`, an adopt's
  previewed adoption is listed as today, and a relink's dossier shape is
  unchanged.
- `a_previewed_create_is_the_items_only_row` stays green: with no marker the
  outcome is `NotApplied`.

**File**: `cli/work-adapters/tests/sync_settled.rs`
**Changes**: Add `a_create_whose_marker_key_is_followed_this_run_is_refused`
for both modes. This is a red test under preview and a guard under apply. The
fixture:
- a synced item carrying `ENG-1`;
- the tracker moving `ENG-1` to `ENG-2` (`moved`);
- an unsynced item with a digest-matching `Created` marker for `ENG-2`.

Both modes report `already-written` with the cause
`ENG-2 is already followed by this run's identity settlement`, and no remedy
hint. The marker's bytes on disk are unchanged after the preview.

#### 2. CLI-level tests

**File**: `cli/work-cli/src/test_support.rs`
**Changes**: Add `pending_push_tree(root, integration)` with the same shape
and `#[must_use]` as `meta_tree`, sharing its directory-collecting function.
It snapshots `<root>/<STATE_DIR>/integrations/<integration>/pending-push/`,
with the path built from `crate::sync::STATE_DIR`, and returns an empty map
when the directory is absent.

**File**: `cli/work-cli/src/sync.rs` (tests module)
**Changes**:

Red tests:
- `a_preview_reports_a_pending_push_refusal_as_the_run_would`: an `Attempted`
  marker and a targeted preview. Asserts:
  - the `0001\tfailed\tunsynced\tterminal` row;
  - an error line starting
    `error: 0001: create would fail: pending push refused (prior-attempt-unknown): `;
  - exit `TERMINAL`;
  - `pending_push_tree` unchanged.
- `a_whole_corpus_preview_ranks_a_refusal_above_the_discovery_deferral`: an
  `Attempted` marker and an untargeted preview. Asserts exit `TERMINAL`, the
  `0001\tfailed\tunsynced\tterminal` row, the unchanged
  `#\tdiscovery\tskipped\tpending-push` record, and that stdout and stderr
  match the run on a copy.
- `a_preview_and_the_run_report_the_same_create_failure`: loops over the
  seedings below with whole-corpus runs. For each, it builds two fresh
  `sync_repo()`s, previews one and applies the other. In each run's stdout and
  stderr it replaces the repository root with `<root>` and reads `would fail`
  as `failed`. It then asserts identical stdout, identical stderr and the same
  exit code. The seedings:
  - an `Attempted` marker;
  - an unreadable marker;
  - a digest-mismatched `Created` marker;
  - a `Created` marker whose key another item carries;
  - a reusable `Created` marker beside a malformed work file;
  - a work file with an `id` and an unclosed frontmatter fence.
- `a_preview_distrusts_a_committed_marker`: seeds a marker, runs `commit_all`,
  then previews. Asserts `marker-unreadable` with
  `marker is tracked by version control`.
- `a_previewed_refusal_outranks_a_conflict`: builds `conflicts_with_dossiers`
  first, then seeds an unsynced `0003` with an `Attempted` marker, and
  previews. The apply-side copy is a separate `sync_repo()` built in the same
  order. Asserts exit `TERMINAL` on both.

Guard test:
- `a_preview_without_a_refusal_is_a_clean_create`: loops over no marker and a
  reusable `Created` marker over a complete corpus. Asserts the
  `0001\tcreate-from-local\tunsynced\t-` row, no `error: 0001:` line, exit
  `CLEAN`, and `pending_push_tree` unchanged (empty with no marker).

#### 3. The preview branch (green)

**File**: `cli/work-adapters/src/sync/run.rs`
**Changes**:
- Hoist the `carrier` closure from `run.rs:2429` above the preview branch, so
  that both modes share it.
- Replace the create loop with `previewed_outcome`.

```rust
let carrier =
    |candidate: &ExternalId| carrier_of(request.corpus, settled, candidate);
```

```rust
for create in &creates_from_local {
    reported.push(create.report(previewed_outcome(
        create, request, ports, &carrier, run_start_epoch,
    )));
}
```

```rust
fn previewed_outcome(
    create: &LocalCreate<'_>,
    request: &SyncRequest<'_>,
    ports: &SyncPorts<'_>,
    carrier_of: &dyn Fn(&ExternalId) -> Option<KeyCarrier>,
    attempted_at: u64,
) -> ItemOutcome {
    if !create.is_bare() {
        return ItemOutcome::NotApplied;
    }
    let inputs = LocalCreateInputs::gather(create, request);
    let decision = inputs.and_then(|inputs| {
        let local_create = inputs.create_request(
            create,
            request,
            ports.author,
            carrier_of,
            attempted_at,
        );
        let marker = MarkerReading::read(&local_create, ports.files.tracking);
        bare_create_decision(&local_create, &marker)
    });
    match decision {
        Ok(_) => ItemOutcome::NotApplied,
        Err(error) => ItemOutcome::Failed(error),
    }
}
```

### Success Criteria:

#### Automated Verification:

- [x] Each red test fails before the preview branch changes and passes after
      it. The failing run is recorded in the PR.
- [x] Each guard test passes before and after the preview branch changes.
- [x] Engine tests pass:
      `cargo nextest run --manifest-path cli/Cargo.toml -p work-adapters --test sync_create --test sync_settled`
- [x] CLI tests pass:
      `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-work --bin accelerator-work`
- [x] The whole CLI workspace passes: `mise run test:unit:cli`
- [x] Format and lint are clean: `mise run cli:check`

#### Manual Verification:

- [ ] Against the `TES` Linear sandbox, reproduce 0351's report. Leave an
      `Attempted` marker for an unsynced item, then run
      `accelerator work sync --target <id> --preview`. It prints
      `<id>\tfailed\tunsynced\tterminal` and the `would fail` line, and exits
      71. `--target <id>` prints the same with `failed` and exits 71. The
      marker's bytes are unchanged after the preview.
- [ ] Deleting the marker turns the same preview back into
      `create-from-local` with exit 0.

---

## Phase 3: Retire the Workaround and Record the Fix

### Overview

The skill, the CLI help, the user guide and the changelog stop describing the
gap and start describing what the preview now reports.

### Changes Required:

#### 1. Skill text and its content test (test first)

**File**: `tests/unit/tasks/test_sync_work_items_skill_structure.py`
**Changes**: Replace `test_a_clean_preview_does_not_vouch_for_a_pending_push`
(lines 339-349) with `test_a_preview_reports_a_refused_pending_push`. The new
test asserts that:
- "A preview does not yet read" is absent from the skill;
- the skill no longer tells the agent to read
  `pending-push/<id>.json` after a preview;
- both statements of `--preview` coverage name the pending-push refusal for a
  create without a `--resolve` order.

Leave `TestRecoverySkill.test_it_opens_the_marker_by_its_derived_path` alone.

**File**: `skills/work/sync-work-items/SKILL.md`
**Changes**:
- Delete the workaround (lines 263-270), from "A preview does not yet read"
  through "before the real run.".
- Extend the `--preview` guarantee (lines 240-243) to:
  "Under `--preview` no baseline mutation occurs, every planned push carries a
  locally-validated payload check or the `pull-residue` or `unrenderable`
  failure the run would meet, every planned pull carries the `unrenderable`
  failure the run would meet, and every create without a `--resolve` order
  carries the pending-push refusal, the incomplete-corpus deferral or the
  work-file failure the run would meet. A create with a `--resolve` order is
  checked against its marker only on the run."
- Align the later statement (lines 760-766): "present the same plan (every
  push carrying its locally-validated payload check, and every create without
  a `--resolve` order carrying the failure the run would meet)".

#### 2. CLI help and golden

**File**: `cli/work-cli/src/cli.rs`
**Changes**: Replace the sentence at lines 156-157 ("A preview does not yet
read a create's pending-push marker, so a pending push the run would refuse
fails only on the run itself.") with "A preview reports a create the run
would refuse from its pending-push marker.", and reflow the paragraph.

**File**: `cli/work-cli/tests/fixtures/cli_surface.golden`
**Changes**: Make the matching change on line 211 by hand.

#### 3. User guide

**File**: `docs-site/src/content/docs/guides/sync-work-items.mdx`
**Changes**: In "A pending push was refused", after "until you say whether it
exists.", add "A `--preview` reports the same refusal before anything is
applied."

#### 4. Changelog

**File**: `CHANGELOG.md`
**Changes**: Add this entry under `## [Unreleased]` → `### Fixed`, beside the
`work sync --preview` pull entry. It is a fix because the preview now meets
the contract it always claimed:

```markdown
- **`work sync --preview` reports a create the run would refuse.** A create
  whose pending-push marker the run would refuse, or whose work file it could
  not use, previewed as a clean create. The preview now prints the run's
  `failed` row and `would fail` line, and exits as the run would: 71 for a
  refused pending push, 75 for an unparseable work file or a reuse blocked by
  one, and 70 for a work file that could not be read or a reuse blocked by an
  environment fault. A whole-corpus preview that exited 70 for the
  pending-push discovery deferral now exits 71. Scripts that parse
  `work sync` exit codes need updating.
```

Under `### Breaking`, add a one-line pointer, because the 70-to-71 change
turns a retryable signal into one that must not be retried:

```markdown
- **`work sync --preview` exits as the run would for a refused create** —
  see Fixed. Scripts that parse `work sync` exit codes need updating.
```

### Success Criteria:

#### Automated Verification:

- [x] The skill test fails before the skill edit and passes after it:
      `uv run pytest tests/unit/tasks/test_sync_work_items_skill_structure.py -v`
- [x] The CLI surface golden matches:
      `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-work --test cli_surface`
- [ ] The read-only CI set passes: `mise run check`
- [ ] The full local CI mirror, including `docs:check`, exits 0: `mise run`

#### Manual Verification:

- [ ] Run `/sync-work-items --preview` over a repository with an `Attempted`
      marker. The skill reports the item as needing attention (a `terminal`
      pending-push refusal) and points to the recovery skill's pending-push
      recovery, without reading the marker itself.
- [ ] Before merging, check whether 0377 has landed. If it has, update its
      pinned preview row expectations for a bare create with a refused
      marker. If it has not, note on PR #224 that 0377 must pin the new row.

---

## Testing Strategy

### Unit Tests:

- `push_precondition` already has full pure coverage
  (`cli/work/src/sync/push_precondition.rs:243-544`). No change.

### Integration Tests:

- Engine level (`sync_create.rs`, `sync_settled.rs`), each under preview:
  - every refusal reason;
  - both distrust causes, with tracked and unknown tracking;
  - the incomplete-corpus deferral;
  - the reusable marker;
  - the unusable work file and the unsafe id;
  - the settlement-followed key;
  - ordered creates and relinks left as they are.

  Each test checks that nothing is sent and that the `Spy` records no write or
  removal under `pending-push/`.
- CLI level (`sync.rs`):
  - the exact row, stderr and exit code;
  - the whole-corpus 70-to-71 change;
  - preview and apply parity on fresh copies of each fixture, with the
    repository root normalised;
  - precedence over a conflict;
  - the committed-marker distrust;
  - the clean outputs.

  Each is checked against an on-disk `pending-push/` snapshot.

### Manual Testing Steps:

1. In the `TES` sandbox, leave an `Attempted` marker for an unsynced item.
2. Run `accelerator work sync --target <id> --preview`. Expect the failed row,
   the `would fail` line and exit 71, with the marker unchanged.
3. Run `accelerator work sync --target <id>`. Expect the same row with
   `failed`, and exit 71.
4. Remove the marker and preview again. Expect `create-from-local` and exit 0.

## Performance Considerations

The preview now reads each bare create's work file and marker once. This is
the same work the run already does, so a preview's cost moves towards the
run's. The new work makes no tracker calls.

## Migration Notes

A preview over a create the run would refuse now exits with the run's code
instead of 0:
- 71 for a refused pending push;
- 75 for an unparseable work file or a reuse blocked by one;
- 70 for an unreadable work file or a reuse blocked by an environment fault.

A whole-corpus preview that exited 70 for the discovery deferral now exits 71.
The changelog tells callers that parse `work sync` exit codes. There is no
data migration.

## References

- Original work item: `meta/work/PP-926-sync-preview-hides-pending-push-refusals.md`
- Work item review: `meta/reviews/work/PP-926-sync-preview-hides-pending-push-refusals-review-1.md`
- Plan review: `meta/reviews/plans/2026-11-17-PP-926-sync-preview-hides-pending-push-refusals-review-1.md`
- Pending-push machinery: `meta/plans/2026-10-11-PP-884-sync-pulls-of-tracked-work-items-fail-silently.md`
- Preview failure pattern to copy: `validate_pushes`, `cli/work-adapters/src/sync/run.rs:1000-1037`
- Engine refusal table to mirror: `cli/work-adapters/tests/sync_create.rs:3245-3327`
