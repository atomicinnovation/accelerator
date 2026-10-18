---
type: "plan-validation"
id: "2026-10-15-0302-work-item-re-key-command-validation"
title: "Validation Report: Work Item Re-Key Command Implementation Plan"
date: "2026-10-17T23:01:23+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-15-0302-work-item-re-key-command"
tags: ["work", "rekey", "retirement", "tracker", "vcs", "migrate", "skill"]
last_updated: "2026-10-17T23:01:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Work Item Re-Key Command Implementation Plan

### Implementation Status

✓ Phase 1: `fetch_all` current-key contract - Fully implemented
✓ Phase 2: Shared dirty-tree observation and `ACCELERATOR_FORCE` - Fully implemented
✓ Phase 3: Retirement extensions - Fully implemented
✓ Phase 4: Rekey domain - Fully implemented
✓ Phase 5: `work rekey` apply and resume - Fully implemented
✓ Phase 6: Preview, discard and proposed-plan binding - Fully implemented
✓ Phase 7: `{tracker}` mode - Fully implemented
✓ Phase 8: `/rekey-work-items` skill and documentation - Fully implemented

Every success-criterion box is ticked, and each phase's "as built"
section records its deviations from the sketch. The branch is 77
revisions over trunk: 155 files, +20,426 / -505.

### Automated Verification Results

✓ `mise run` exits 0 (331 s), and the `fix` stage left the working copy
  unchanged
✓ `test:unit:cli`: 5,796 run, 5,796 passed, 2 skipped
✓ `test:integration:skill-invocation`: 143 passed
✓ `test:unit:frontend`: 2,615 passed; `test:e2e:visualiser`: 355 passed
✓ `public-api:check`, `pup:check`, `deny:check`, `docs:build`,
  `docs:audit:check` and every `lint:*` and `types:*` lane passed
✓ Test-name audit: of the 311 four-or-more-word snake_case identifiers
  the plan names, 305 exist verbatim in code; the 6 others are accounted
  for:
  - `force_bypasses_the_refusal_and_reaches_the_empty_registry_sentinel`
    was renamed, as Phase 2 records.
  - `the_tracker_pattern_is_refused_until_supported` was deleted in
    Phase 7, as planned.
  - `no_tracker_is_resolved_under_a_numeric_pattern` is held by
    `a_numeric_rekey_reaches_no_network`, as Phase 5 records.
  - `create_push_refuses_while_a_rekey_is_in_progress_and_writes_nothing`
    is folded into
    `create_refuses_while_a_rekey_is_in_progress_and_writes_nothing`, which
    loops `push` over `false` and `true` (`cli/work-cli/src/create.rs:1020`).
  - `a_completed_entry_without_an_integration_is_skipped_on_resume` is
    folded into `a_completed_entry_is_skipped_on_resume`, which loops over
    `integrated` (`cli/work-adapters/tests/rekey_execute.rs:258`).
  - `only_begin_mints_an_identity_run_guard` is an unnamed `compile_fail`
    doctest on `IdentityRunGuard`
    (`cli/work-adapters/src/identity_run.rs:37`).

Not re-run in this validation: the live tracker-contract lane
(`test:integration:tracker-contract`, which needs Jira and Linear
credentials) and the manual checks. The plan records both as run on
2026-10-17.

### Code Review Findings

#### Matches Plan:

- Guard sites: each of the six guarded commands takes the identity-run
  lock where the plan places it. `sync.rs:1390` takes it before any
  tracker read. `promote.rs:181` takes it before `tracker_owned_draft`.
  `create.rs:660` takes it after the `--dry-run` return.
  `create_batch.rs:181` takes it before the manifest is read.
  `next_number.rs:127` takes it before both the `{tracker}` branch and
  the allocating branch. `update.rs:314` takes it only under `--push`,
  before the `ForFile` lock. Every guard is a named binding held until
  the command returns.
- Lock order in `rekey::run` (`cli/work-adapters/src/rekey/run.rs:152-163`):
  identity run, then work-directory creation, then `Retirement`, then
  `Create`, and the plan loads under all three. The global order is stated
  in the `cli/corpus/src/lock.rs` module doc.
- Identity-run lock: it has a 60 s ceiling, the specified waiting lines,
  and a `command` file inside the lockdir. On timeout it re-reads the gate
  and maps the result to `RekeyRunning`, `RekeyInProgress`,
  `RekeyPlanUnreadable` or `Busy`.
- `--discard` compares start digests or absence through
  `WriteSet::unreverted`. A forced discard sets the plan aside by writing
  a copy and then removing the original. An unreadable plan prints the
  partly-applied evidence before it refuses.
- `ACCELERATOR_FORCE` is read through `vcs::forces` (`1` or `true`,
  ignoring case). `ACCELERATOR_MIGRATE_FORCE` is not read by any `work`
  crate.
- Every code in the exit-code table is in `cli/work/src/rekey/outcome.rs`
  with the planned exit code, and `cli/work-cli/src/exit_codes.rs`
  documents each one. `E_REKEY_TRACKER_PATTERN_PENDING` is gone from code,
  skills and docs.
- The plan-time `RetirementRefusal` mapping (`rekey/execute.rs:148-165`)
  covers every variant, as planned.
- The in-progress plan is saved before the first corpus write.
  `before_apply` runs on every attempt, including a rollback re-plan, and
  saves before it returns.
- `sync-work-items`, `create-work-item`, `extract-work-items` and
  `refine-work-item` route `E_REKEY_IN_PROGRESS` to `/rekey-work-items`.
  The new skill follows Phase 8's steps, including the rule that Undo is
  never offered under an inherited force.
- The CHANGELOG `Added` and `Changed` entries, the docs-site pages and the
  `configure` skill list are all updated. The skill count reads 75, which
  matches the 75 `SKILL.md` files.

#### Deviations from Plan:

All the deviations below are recorded in the plan's "as built" sections,
except for the last two.

- `Forcing { Unforced, Forced }` replaces `WorkingCopyCheck`, because one
  override covers both the apply check and the discard check.
- `CorpusWorkspace` is split out of `IdentityWorkspace`, replacing
  `open_local`.
- `set_aside` writes a copy and then removes the plan, instead of renaming
  it.
- `NotRekeyed::DraftIdExhausted` was added (exit 1).
- Two retirement-planner defects were fixed test-first in Phase 5:
  whole-token path spellings, and keeping a synced item's `external_id`.
- The rekey skill adds an extra step-1 branch for "a re-key is running
  (pid)". This is not recorded in the plan; it is an improvement.
- A forced discard of a proposed plan, or of an in-progress plan that is
  fully reverted, removes the plan (`run.rs:409-411`, `:442-445`) instead
  of setting it aside. This matches the Phase 5 state sketch, which applies
  force only where it bypasses a refusal. It does not match a literal
  reading of the Decisions row "a forced discard never deletes". Neither
  case loses information: there is no evidence left to keep.

#### Potential Issues:

- **The dirty-tree check can miss dirty files when `.accelerator/` is
  below the VCS root.** This was confirmed by reading the code, not by a
  test. `guarded_working_copy` probes `facts.root`, the VCS root, and gix
  and jj report dirty paths relative to that root. The scopes are built
  relative to `discover_root`, which stops at the nearest `.accelerator/`
  (`cli/config-adapters/src/store.rs:114-126`, `cli/work-cli/src/rekey.rs:234-245`).
  - Example: in `/mono` with config at `/mono/pkg/.accelerator`, a dirty
    `pkg/meta/x.md` matches no scope, so the apply proceeds over
    uncommitted edits.
  - The write-set exemption on resume has the same mismatch, but there
    it errs the safe way and refuses.
  - `VcsWorkingCopyStatus` already makes paths relative to `facts.root`
    (`cli/work-adapters/src/sync/working_copy_status.rs`).
  - `migrate` likely shares the same gap.
- An `external_id` that is not shaped like a tracker key exits 70 under
  `{tracker}`, telling the user to retry later. It should exit 4 with
  `E_REKEY_UNCLASSIFIABLE_ID`. The cause: `run.rs:299-306` sends raw
  `external_id`s to the refresh before `TrackerKey::parse`, so a client's
  pre-flight `Retryable` for an id it cannot embed becomes
  `TrackerUnreachable`. Nothing is written.
- An unsynced ID with an underscore prefix, such as `MY_APP-760`,
  classifies as `Sequenced(760)` via `corpus::is_tracker_key`, but
  `project_key_of` (`cli/work/src/rekey/sequence.rs:26-32`) rejects `_`.
  It therefore moves to `work.key`, against the "kept number keeps its own
  project key" decision, and counts towards the floor as a bare number.
- Duplicate `id`s across two files produce a `Collision` that names the
  first file twice, because `item_holding` returns the first match
  (`cli/work/src/rekey/collision.rs:23`, `:42`). The run still refuses,
  but the message never names the second file.
- An ID or alias of `u64::MAX` makes `allocate_above` compute
  `floor + i` unchecked (`cli/work/src/next_number.rs:153-154`). Debug
  builds panic; release builds wrap and still refuse.
- `is_machinery` hides any dirty path with a component starting `.tmp-`
  or ending `.lockdir`, even a tracked user file. This is narrow, but
  wider than "only rekey's own leftovers".

### Manual Testing Required:

1. Already run (recorded in the plan, 2026-10-17):
  - [x] Phases 2, 5, 6 and 8 checks in a jj scratch repository
  - [x] Phase 7's moved-issue check against Linear and its offline check
  - [x] The live tracker-contract lane against scratch Jira and Linear
2. Worth adding before release:
  - [ ] Run `work rekey` from a repository whose `.accelerator/` is in a
        subdirectory of the VCS root, with one uncommitted corpus edit,
        and confirm it refuses.
  - [ ] Repeat a manual pass with git rather than jj, since the session's
        VCS guard limited every recorded manual check to jj.

### Recommendations:

- Fix the dirty-tree root mismatch before release. Make `dirty_paths`
  relative to the config root, or build the scopes against `facts.root`.
  Start with a failing test that nests `.accelerator/` below the VCS root.
  Check whether `migrate` needs the same fix.
- Parse each `external_id` as a `TrackerKey` before the `{tracker}`
  refresh, so a malformed key refuses with exit 4 instead of 70.
- Make `project_key_of` and `sequence_of` agree on `_` in a prefix.
- Refuse duplicate `id`s explicitly, naming both files.
- Use `checked_add` in `allocate_above`'s overflow branch.
- Decide whether "a forced discard never deletes" should hold for a
  proposed or fully reverted plan, and align either the Decisions row or
  `run.rs`.
