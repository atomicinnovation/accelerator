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
last_updated: "2026-10-18T00:11:26+00:00"
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
  of setting it aside. This matched the Phase 5 state sketch but not the
  Decisions row "a forced discard never deletes". Resolved: the row now
  says a discard that force lets past a refusal never deletes, and a
  proposed or fully reverted plan is removed whether or not force is set.

#### Potential Issues:

Every issue found in review has been resolved test-first, each in its own
commit after this report was first written:

- **Resolved — the dirty-tree check missed dirty files when `.accelerator/`
  was below the VCS root.** `guarded_working_copy` probed `facts.root` but
  built its scopes relative to the config root, so with config at
  `/mono/pkg/.accelerator` a dirty `pkg/meta/x.md` matched no scope.
  `migrate` had the same gap, and a second cause: its `vcs_kind` looked for
  `.git` or `.jj` only in the config root, picked `VcsKind::None`, and
  probed nothing. Both adapters now locate the repository through
  `RepositoryProbe::facts_at` and see its working copy through the new
  `WorkingCopyState::seen_from`. A config root outside the reported
  repository refuses as unprobeable in rekey, and is warned about and
  treated as clean in `migrate`, as its existing policy is. Covered by
  scripted and real git and jj tests in `rekey_working_copy.rs` and
  `dirty_tree_preflight.rs`. A corpus root outside the config root cannot
  occur, since `paths.*` refuses `..` and absolute values; a test pins
  that a re-key stops on one.
- **Resolved — an `external_id` not shaped like a tracker key exited 70
  under `{tracker}`.** A client's pre-flight `Retryable` became
  `TrackerUnreachable`. `keys_to_refresh` now refuses it as
  `E_REKEY_UNCLASSIFIABLE_ID` before any tracker call, and only synced,
  non-draft items' keys are refreshed.
- **Not a defect — an unsynced `MY_APP-760` moves onto `work.key`.** A
  local project key is letters and digits only, so `MY_APP-0760` could
  never be on target and keeping the key would re-key it on every run.
  `ACC-0760` is the fixed point, as the "keeps its own project key … when
  it has one" decision says. A test pins it.
- **Resolved — duplicate `id`s named the first file twice.**
  `refuse_collisions` now refuses an entry whose old `id` another item
  also holds, naming both files, ignoring case.
- **Resolved — `allocate_above` overflowed past `u64::MAX`.** It now adds
  with `checked_add` and reports the overflow with nothing allocated.
- **Resolved — `is_machinery` hid any path named `.tmp-*` or `*.lockdir`.**
  `store` now names the exact shapes it writes (`is_atomic_temporary`,
  `store::lock::is_lock_sentinel`), and the check exempts only a temporary
  of that shape or a sentinel inside a lockdir.

### Manual Testing Required:

1. Already run (recorded in the plan, 2026-10-17):
  - [x] Phases 2, 5, 6 and 8 checks in a jj scratch repository
  - [x] Phase 7's moved-issue check against Linear and its offline check
  - [x] The live tracker-contract lane against scratch Jira and Linear
2. Worth adding before release:
  - [ ] Run `work rekey` and `accelerator migrate` from a repository whose
        `.accelerator/` is in a subdirectory of the VCS root, with one
        uncommitted corpus edit, and confirm both refuse.
  - [ ] Repeat a manual pass with git rather than jj, since the session's
        VCS guard limited every recorded manual check to jj.

### Recommendations:

- Run the two manual checks above before release.
