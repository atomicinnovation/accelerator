---
type: "plan-validation"
id: "2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently-validation"
title: "Validation Report: Sync Pulls of Tracked Work Items Fail Silently Implementation Plan"
date: "2026-10-24T14:26:57+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "plan:2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently"
target: "plan:2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently"
tags: ["sync", "work-cli", "work-adapters", "store", "sync-work-items"]
last_updated: "2026-10-24T14:26:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Sync Pulls of Tracked Work Items Fail Silently

### Implementation Status

✓ Phase 1: Store Containment and Permission Detail - Fully implemented
✓ Phase 2: Console Seam and `run_sync_with` - Fully implemented
✓ Phase 3: Pull Safeguards - Fully implemented
✓ Phase 4: Two Bounded Stores - Fully implemented
✓ Phase 5: Failure Classification by Landing, and Reporting - Fully
implemented
✓ Phase 6: Dossiers Scoped to the Run - Fully implemented
✓ Phase 7: Skill, Guide, Changelog and Skill Checks - Fully implemented

Every automated and manual criterion in the plan is ticked. The manual
Linear and skill checks were run, and the four defects they found are filed
as 0351–0354.

### Automated Verification Results

✓ `mise run` exits 0 at `mosromnq` ("Clear comments the code already
says"), which is later than any green run the plan records. It took ~50 min,
and the working copy was unchanged afterwards, so nothing was reformatted.
That run covers `check`, `fix`, the full test suite, `public-api:check` and
`docs:check`.

### Code Review Findings

#### Matches Plan:

- **Phase 1:** `canonicalize_allowing_missing` refuses `..` in the missing
  tail, on both writes and reads. `NotWritable` keeps the OS text for
  writes and for lockdirs, and `NotDurable` exists.
- **Phase 3:** `pull_steps` runs lock → read → recheck → keep-edits → hash
  → re-read → recheck → write → baseline while holding the `ForFile` lock.
  The copy decision and the kept-edits retention follow the plan, and
  unclassifiable items become their own `retryable` rows.
- **Phase 4:**
  - `SyncFiles` carries the two stores. `rooted_store` refuses
    non-`Normal` components.
  - `ValidatedRoots::check` runs before discovery and exits 74 for
    outside, overlapping or symlinked roots.
  - Markers are written and cleared only through the integration-state
    store.
- **Phase 5:**
  - `ApplyError::class()` is total over nine classes, and an `Io` failure
    is classed by landing.
  - `NotWritable` on a pull is a retryable `write-local` failure.
  - The `error:` lines and the exit-code precedence are as planned.
  - Inventory records `#\tunidentified`; `PlannedAction` has private
    fields with `Instruction::{Order, Resolution}`.
- **Phase 6:** `DossierAuthority` retires a dossier only for
  `Settled`/`NotInConflict` items, or for orphans when the inventory
  `proves_absence()`. A preview retires nothing, and dossiers are built
  after apply.
- **Phase 7:**
  - The skill reports `retryable`/`terminal` rows whatever the exit code
    and never reports such a run complete.
  - It relays kept copies and confirms before every order.
  - The changelog covers every exit-code change listed in Migration Notes.

#### Deviations from Plan:

These departures are not recorded in any phase's As Implemented section:

- Planned tests that are missing or renamed:
  - `a_relinked_items_dossier_is_retired_and_kept_on_preview` (Phase 6) is
    missing. `Relink` → `Resolution` is covered only at unit level
    (`work-adapters/tests/dossiers.rs`).
  - `an_integrations_directory_symlinked_out_of_the_repository_is_refused`
    (Phase 4) is missing. Only the work-directory backstop exists
    (`work-cli/src/sync/stores.rs`).
  - `the_sync_recovery_directory_ignores_its_contents` (Phase 3) is
    missing. It is covered by the older
    `prepare_writes_an_ignore_rule_for_everything`.
  - `a_refused_write_keeps_its_class_through_the_attempt`,
    `a_resolve_rewrite_is_recorded` and
    `conflict_status_of_each_reported_item` exist under other names or are
    split across several tests.
- `ConflictStatus` maps `RecordBaseline` on `Unsynced`, `RemotelyModified`
  or `Conflict` to `Unobserved`, not `NotInConflict`
  (`work-adapters/src/sync/dossiers.rs:115-118`). This is the conservative
  direction, and the planner probably never produces it.
- The pull's own re-read maps every `io::Error`, `InvalidData` included, to
  a retryable `read-local` failure (`work-adapters/src/sync/apply.rs:1724-1726`).
  So a file that becomes non-UTF-8 between planning and the pull exits 70,
  not the planned `unparseable` 75.
- Phase 5's As Implemented says "no apply path reaches a tracker failure
  after a landing". A local create lands its Attempted marker, and a relink
  first clears the `external_id`, before `land_created`. A `create` failure
  there keeps the tracker's class instead of becoming terminal.
- Recovery-copy containment allows only plain components and refuses
  symlinks from the state root downwards
  (`corpus-adapters/src/recovery.rs:46-76`). Directories above the state
  root, including `.accelerator`, are never checked.
- The per-write symlink backstop returns a generic `UnsafePath` with no
  `paths.work` remedy text. Only the pre-flight check names the remedy.
- Phase 7's As Implemented still says the manual checks are outstanding,
  although they are now ticked.

#### Potential Issues:

- An unreadable pending-push marker reads as absent
  (`work-adapters/src/sync/apply.rs:1219`, `read_to_string(..).ok()`).
  EACCES or non-UTF-8 content becomes `None`, so the run proceeds with a
  create. A `Created { PP-12 }` marker made unreadable therefore leads to a
  second issue instead of a `marker-unreadable` refusal. The `.ok()` dates
  from before this branch, but it defeats the refusal Phase 5 added.
- The `<integrations>/<tracker>/` subdirectory is never checked
  pre-flight: `ValidatedRoots::integration_state` derives it unchecked
  (`work-cli/src/bounded_roots.rs:328-330`).
  - A committed symlink at `.accelerator/state/integrations/linear`
    therefore passes pre-flight.
  - `pending_push::prepare_dir` (`work-adapters/src/sync/pending_push.rs:112-115`)
    then runs raw `create_dir_all` and writes `.gitignore` through the
    link, outside the repository.
  - The bounded writes fail afterwards, but not with exit 74.
- Some failures exit non-zero with no `error:` line, which contradicts the
  help text's "Every failure prints `error: …`" (`work-cli/src/cli.rs:153`):
  - A tracker read failure and `DiscoveryStatus::Failed` count towards 70
    in `exit_code_for_report` (`work-cli/src/sync.rs:957-959`) but are not
    in `RunReport::failures()`.
  - A failed or refused identity row exits 4 the same way.
- Work files can still vanish silently. `markdown_files_in` drops
  directory entries it cannot iterate and any path where `is_file()` is
  false, a dangling `0005-x.md` symlink included
  (`work-adapters/src/filesystem.rs:79-87`).
- `failure_line` applies `single_line` but not `printable`
  (`work-cli/src/report_records.rs:67-72`). ANSI escapes in a tracker
  cause reach stderr unescaped, although newline forging is still
  prevented.
- Kept-copy directories built part-way are never cleaned up
  (`work-adapters/src/sync/apply.rs:913-926`). A failure after `prepare` or
  after `write_once` leaves an `Unrecorded` directory or a permanent
  `KeptEarlier` copy, and a truncated copy is possible.
- The copy record is a single `original\tcopy` line, so a tab or newline in
  a work-file path loses the copy's note or mis-attributes it.
- Several `create_dir_all`, `publish` and `FileRemove` paths still map
  `PermissionDenied` to `Io` rather than `NotWritable`
  (`store/src/lib.rs:161`, `corpus-adapters/src/store.rs:101,171-174,205`).
  The OS text survives, but the variant does not.
- The skill and docs disagree with the code in several places:
  - `SKILL.md:317` says "The other four refusal classes", but the table
    lists five.
  - `exit_codes.rs:42-43` still calls `70` "Safe to retry".
  - `SKILL.md:738-739` says a tracker-side field rejection surfaces "as a
    `71`". I did not verify the current class.

### Manual Testing Required:

1. Already done, per the plan's ticked criteria: the Linear pull, the
   read-only work directory, the two-conflict `--target`, and the skill
   drive.
2. If the marker issue is fixed:
   - [ ] `chmod 000` a `Created` pending-push marker, then run `work sync`.
     Expect a `marker-unreadable` refusal and no new issue.

### Recommendations:

- Fix, or file as a follow-up, the unreadable-marker path before release.
  It can create a duplicate tracker issue.
- Pre-flight check the tracker subdirectory, or route
  `pending_push::prepare_dir` through the integration-state store.
- Either add `error:` lines for discovery/read failures and identity
  failures, or narrow the help text's "Every failure" claim.
- Add the missing relink dossier and integrations-symlink backstop tests,
  or record their omission in As Implemented.
- Correct Phase 5's "no tracker failure after a landing" statement and
  Phase 7's "outstanding" paragraph.
- Fix the skill's refusal-class count and the stale "Safe to retry" doc on
  exit code 70.

### Follow-up

Fixed on the branch after this validation:

- An unreadable pending-push marker is refused as `marker-unreadable`.
- A symlinked `<integrations>/<tracker>/` directory refuses the run with
  exit 74 before any tracker call.
- A failed tracker read prints `error: -: show failed:` and a failed
  discovery `error: -: discover failed:`. The help and skill now say a
  refused key change is a `failed` row only.
- A dangling Markdown link in the work directory is reported as
  unreadable, and an entry that cannot be listed fails the listing.
- The relink dossier and integrations-symlink backstop tests exist.
- The skill's refusal-class count and its tracker-rejection exit claim are
  corrected, and so are the plan's two inaccurate notes.

Found not to be issues:

- Control characters already reach stderr escaped: `Console::warn` applies
  `printable` to every line.
- `exit_codes.rs` calls `70` "Safe to retry" meaning a re-run cannot
  double-apply, which holds.

Left as they are: part-built kept-copy cleanup, tabs or newlines in copy
records, `PermissionDenied` mapped to `Io` on directory creation, and the
pull's re-read of a newly non-UTF-8 file exiting 70 rather than 75.
