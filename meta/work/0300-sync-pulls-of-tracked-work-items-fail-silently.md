---
type: "work-item"
id: "0300"
title: "Sync Pulls of Tracked Work Items Fail Silently"
date: "2026-10-05T20:03:25+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "in-progress"
kind: "bug"
priority: "high"
parent: "work-item:0146"
relates_to: ["work-item:0257", "work-item:0285", "work-item:0296"]
external_id: "PP-884"
tags: ["bug", "sync", "work-cli"]
last_updated: "2026-10-10T23:17:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0300: Sync Pulls of Tracked Work Items Fail Silently

**Kind**: Bug
**Status**: In Progress
**Priority**: High
**Author**: Toby Clemson

## Summary

`accelerator work sync` fails every pull into an already-tracked local work
item, and reports the failure as a bare `failed … -` row with exit 0. Remote
edits to synced items therefore never reach the work directory, and a
`--resolve <id>=remote` conflict choice cannot take effect. A `--target` run
also deletes the conflict dossiers of items it did not target, so the
remaining conflicts lose their diffs mid-resolution. The fix deliberately
changes the exit code of every apply failure that today exits 0.

## Context

Terms used below:

- **Work directory**: the configured work-items directory (`work.dir`,
  `meta/work/` by default).
- **Baseline**: the per-tracker record of each item's last-synced local and
  remote hashes (`<paths.integrations>/<tracker>/last-sync.json`); it
  *advances* when a sync records the new hashes.
- **Planned item**: an item the run assigned a sync state to — every tracked
  item on a full run, only the targets on a `--target` run.
- **Failure class**: the classification of an apply failure, which the
  report renders in the detail column and the exit code derives from:
  retryable (70), terminal (71), unconfigured (74) or rejected (75).
- **Exit codes**: 0 clean, 1 error (a run-level failure outside the
  applier, such as unreadable configuration), 4 conflicts awaiting
  resolution, plus the failure classes above. When a run has several
  outcomes the most severe wins, in the order terminal, error, conflicts
  (4), rejected, unconfigured, retryable; this fix leaves that order
  unchanged.
- **Integration-state directory**: `<paths.integrations>/<tracker>/`, which
  holds the baseline, the conflict dossiers and other per-tracker sync
  state.
- **Apply mode**: a run without `--preview`, which writes files and the
  baseline rather than only planning.
- **Item apply**: the writes one run makes for a single planned item (its
  file, its baseline entry, its tracker calls, its markers). An *apply
  failure* is a failure inside one item apply; dossier persistence after
  the run is not part of any item apply.
- **Pending-push marker**: a file under the integration-state directory
  recording that a create-from-local push is in flight, so an interrupted
  create is not repeated.
- **Local-author port**: the separate port through which items created from
  remote are written into the work directory.
- **Conflicted**: a planned item whose row's state is `conflict` and whose
  item apply did not resolve it.
- **Conflict dossier**: the per-item section diff a run writes to
  `<paths.integrations>/<tracker>/conflicts/<id>.md` for
  `/sync-work-items` to present during resolution.
- **Report row**: one tab-separated line per item with columns *id*,
  *action*, *state* and *detail*.

The sync applier writes pulled content through `SyncPorts.writer`. The CLI
wires that port to the same `FileCorpusStore` it builds for the baseline,
rooted at the integration-state directory
(`<paths.integrations>/<tracker>/`). The store's containment check refuses
any path outside that root, so the write to `meta/work/<id>-<slug>.md` is
rejected. Creating items from remote is unaffected because it writes through
the separate local-author port.

The refusal surfaces as an I/O apply error, which has no failure class. The
report renders an unclassified failure with detail `-`, discards its message,
and the exit-code calculation treats it as clean.

Separately, every run deletes all dossiers in the conflicts directory before
writing dossiers for the conflicts among its planned items. A full run plans
every item, so this refreshes the set. A `--target` run plans only its
targets, so it deletes the dossiers of other items that are still
conflicted. The next full run regenerates them; until then, resolving
conflicts one `--target` at a time leaves the rest with no diff to present.

Observed on 2026-10-05 against Linear: ten items (0183, 0184, 0197, 0204,
0207, 0209, 0227, 0229, 0230, 0257) reported `failed remotely-modified -`.
The full run exited 4 only because conflicts were also pending.

## Requirements

Reproduction A — clean pull:

1. Have a synced work item whose remote issue was edited after the last sync,
   with a clean local file.
2. Run `accelerator work sync --target <id>`.

Expected: the local file is rewritten with the pulled content under its
unchanged frontmatter, the baseline entry advances, and the row's action is
`pull`.

Actual: the row is `<id>\tfailed\tremotely-modified\t-`, the file and
baseline are untouched, nothing is printed to stderr, and the exit code is 0.
`--preview` plans the same item as a `pull`; `--per-item-reads` (fetching
each remote issue individually instead of in bulk) makes no difference.

Reproduction B — resolve to remote:

1. Have a synced work item edited both locally and remotely, so a run
   reports it as conflicted and writes its dossier.
2. Run `accelerator work sync --target <id> --resolve <id>=remote`.

Expected: as Reproduction A, and the item's dossier is removed. Actual: as
Reproduction A; the resolution pull is written through the same writer and
fails the same way.

Reproduction C — dossier pruning:

1. Have two conflicted items, `<a>` and `<b>`, each with a dossier after a
   full run.
2. Run `accelerator work sync --target <a>`.

Expected: `<b>`'s dossier is untouched. Actual: `<b>`'s dossier is deleted.

The fix must:

- Confine every sync write to one of two roots, each enforced by its own
  bounded writer:
  - the work-item writer, confined to the work directory, which the
    applier uses for pulled work-item files;
  - the integration-state writer, confined to the integration-state
    directory, which the applier uses for pending-push markers,
    `BaselineStore` uses for the baseline, and the CLI's dossier
    persistence uses for conflict dossiers.

  The local-author port's wiring is outside this change.
- Give every `ApplyError` variant a failure class, so that no row renders
  detail `-` and no apply failure leaves the exit code at 0. An I/O failure
  is retryable (exit 70) when no earlier write in the same item apply has
  landed anywhere — local file, baseline or tracker — so a re-run re-plans
  the item identically. It is terminal (exit 71) once any earlier write in
  that item apply has landed, because the file, baseline and tracker then
  disagree and a re-run would report a spurious conflict. Writes made for
  other items in the same run do not count. The rule governs; the table
  lists every operation and context that can fail today, including
  `author-local`, whose classification is in scope even though its port
  wiring is not:

  | Operation | Earlier write landed | Class |
  |---|---|---|
  | `read-local` | never | retryable |
  | `write-local` (pull) | no | retryable |
  | `author-local` (create from remote) | no | retryable |
  | `marker-write` before the tracker create | no | retryable |
  | `hash-local` after a pull's `write-local` | yes | terminal |
  | `baseline-set` after a local or tracker write | yes | terminal |
  | `link-external-id` after the tracker create | yes | terminal |
  | `marker-write` after the tracker create | yes | terminal |

  Tracker-originated variants keep their current classes: a retryable,
  terminal, unconfigured or rejected `TrackerError` maps to the class of
  the same name. Their operation label is the tracker call that failed
  (`show`, `update` or `create`).
- For every apply failure, put the failure class in the row's detail column
  and print a stderr line naming the item, the failed operation and the
  cause. A failed dossier write is not an apply failure; it keeps today's
  warning on stderr.
- On a `--target` run, every conflicted planned item has a dossier holding
  its current section diff, whether or not it had one before — including a
  conflicted item whose resolution pull failed; every other planned item
  has no dossier; and the dossiers of unplanned items are untouched. A full
  run keeps today's behaviour: afterwards only the dossiers of conflicted
  items remain.
- Update the `sync-work-items` skill so that, whatever the exit code, it
  reports every `retryable` or `terminal` row as a failure naming the item,
  offers a re-run for `retryable` rows, tells the user a `terminal` row
  needs attention rather than a re-run, and never reports such a run as a
  completed sync.

## Acceptance Criteria

- [ ] Given a tracked item that was modified remotely and is clean locally,
      when `work sync` runs in apply mode, then the local file's frontmatter is
      byte-identical to before, its body is the content the sync engine
      reconstructed from the remote issue (exact fidelity is verified under
      0296), the baseline entry advances, and the row's action is `pull`; an
      immediate `work sync --preview` then plans no action for the item.
- [ ] Given a conflicted item `<a>` and another conflicted item `<b>`, both
      with dossiers, when `work sync --target <a> --resolve <a>=remote` runs,
      then `<a>`'s file is rewritten as in the first criterion, its baseline
      entry advances, its dossier is removed, its row's action is `pull`, and
      `<b>`'s dossier is unchanged.
- [ ] Given a run whose only problem is a pull's failed local write — the
      work directory is made non-writable, with no pending conflicts and no
      other failures — when `work sync` runs, then the row's detail is
      `retryable`, stderr contains a line naming the item id, the operation
      `write-local` and the operating system's permission error, the item's
      file and baseline entry are unchanged, and the exit code is 70.
- [ ] Given that same item, when the work directory is made writable and
      `work sync` runs again, then the row's action is `pull` and the
      baseline entry advances.
- [ ] Given a pull whose local write succeeds but whose baseline write fails
      — the integration-state writer is wrapped so that it fails only that
      item's baseline write, with no other problems — when `work sync` runs,
      then the local file holds the pulled content, the baseline entry is
      unchanged, the row's detail is `terminal`, stderr names the item, the
      operation `baseline-set` and the cause, and the exit code is 71.
- [ ] Given each operation in the classification table and each
      `TrackerError` kind, when its `ApplyError` is classified, then the
      class matches the table or the same-named class, and when a run whose
      only problem is that failure reports it, the row's detail is that
      class, stderr contains a line naming the item, the operation and the
      cause, and the exit code is the class's code (70, 71, 74 or 75). The
      test pins today's tracker-originated mapping before the
      classification changes.
- [ ] Classification is total: `ApplyError::class` returns a class for
      every variant, with no unclassified outcome in its type, so no report
      row can carry detail `-` for a failure.
- [ ] Given a run with pending conflicts and a `terminal` I/O failure, when
      it completes, then the exit code is 71; given a run with pending
      conflicts and a `retryable` I/O failure, then the exit code is 4. Both
      kinds of row appear in each report.
- [ ] Given another item's pull succeeds earlier in the same run, when an
      item's `write-local` fails, then that item's row is `retryable`.
- [ ] Given the work-item writer the CLI wires, when asked to write a path
      under `<paths.integrations>/<tracker>/` or a `..` path escaping the
      work directory, then it returns an error and creates no file.
- [ ] Given the integration-state writer the CLI wires, when asked to write
      a path in the work directory or a `..` path escaping
      `<paths.integrations>/<tracker>/`, then it returns an error and creates
      no file.
- [ ] Given conflicted items `<a>` and `<b>` with dossiers, and a further
      remote edit to `<a>` after its dossier was written, when `work sync
      --target <a>` runs and `<a>` is still conflicted, then `<b>`'s dossier
      is unchanged and `<a>`'s dossier contains the new remote text, not the
      earlier diff.
- [ ] Given a conflicted item `<a>` with a dossier whose `--resolve
      <a>=remote` pull fails, when the run completes, then `<a>`'s dossier
      still exists.
- [ ] Given an item `<c>` with no dossier that has become conflicted, and a
      conflicted item `<b>` with a dossier, when `work sync --target <c>`
      runs, then `<c>` has a dossier holding its section diff and `<b>`'s
      dossier is unchanged.
- [ ] Given an item `<a>` with a dossier whose local edit now matches remote,
      when `work sync --target <a>` runs without `--resolve`, then `<a>`'s
      dossier is removed and other items' dossiers are unchanged.
- [ ] Given dossiers for an item a full run no longer reports as conflicted
      and for an id that is no longer tracked, when the full run completes,
      then both dossiers are removed.
- [ ] Given `work sync` exits 70 with a `retryable` row, or exits 4 with
      both conflicted rows and a `retryable` row, when the `sync-work-items`
      skill handles the run, then it names the failed item and offers a
      re-run (alongside conflict resolution in the second case), and does
      not report the sync as complete.
- [ ] Given `work sync` exits 71 with a `terminal` row, when the
      `sync-work-items` skill handles the run, then it names the item, says
      the failure needs attention and does not offer a re-run, and does not
      report the sync as complete.
- [ ] The skill criteria above are verified by the skill's SKILL.md carrying
      explicit, exit-code-independent branches for `retryable` and `terminal`
      rows that instruct each behaviour.
- [ ] A regression test drives a pull of an existing item through the same
      writer wiring `work sync` builds, with real filesystem writers rooted
      as in production (the tracker may be stubbed), and asserts the
      outcomes of the first criterion.

## Open Questions

None.

## Dependencies

- Blocked by: none for implementation; the release is gated on 0296 (see
  below).
- Blocks: none
- Ships with: 0296 (all of it). 0296's open questions must be resolved
  before this item is released, and both merge in one combined change so
  neither can reach a release alone. Today every tracked pull fails before
  writing, so 0296's defects in pulled content (the projected title line,
  the list-marker style) cannot reach local files. Once this fix makes
  pulls succeed, they would reach every remotely edited item and be pushed
  back to Linear. 0296's pull and resolve-to-remote criteria can only be
  verified end to end with this fix in place. Both items change conflict
  dossiers — this one which dossiers are deleted, 0296 Defect 3 how they
  are rendered — so they should be integrated together.
- External: the configured remote tracker (Linear via `work.integration`).
  End-to-end verification needs it reachable and an item whose remote copy
  was edited after the last sync.
- Consumer: the `sync-work-items` skill — the only caller of `work sync` in
  this repository — which branches on its exit codes and report rows, relays
  its stderr, and presents its conflict dossiers.
- Consumer: user scripts and CI jobs that call `accelerator work sync`
  directly. Apply failures that exit 0 today will exit 70 or 71, so the
  combined 0300/0296 change must add a changelog entry calling out the
  change.
- Environment: the permission-based retryable criteria need a non-root
  test runner, since root ignores directory permissions; any CI lane
  running them must run as a non-root user, and a root run skips them with
  a stated reason rather than passing vacuously.
- Precedent: 0285 (done) changed `work sync` exit-code semantics and updated
  the `sync-work-items` skill to match; it is listed for that pattern and
  imposes no ordering.

## Assumptions

- On the pull path, only the writer wiring is wrong for the write to succeed.
  Reconstruction of the pulled content has its own defects, tracked in 0296;
  the first two criteria check that the reconstructed content lands under
  unchanged frontmatter, and 0296 verifies the exact body. Baseline logic is correct
  once the write succeeds (the preview plans these pulls correctly).

## Technical Notes

- `cli/work-cli/src/sync.rs`: the sync command builds one
  `FileCorpusStore` rooted at the baseline's directory
  (`<paths.integrations>/<tracker>/`) and passes it as both the baseline
  store's writer and `SyncPorts.writer`.
- `cli/corpus-adapters/src/store.rs`: `FileCorpusStore::write` refuses any
  path outside its root via `ensure_contained`.
- `cli/work-adapters/src/sync/apply.rs`: `ItemApplier::pull` writes the
  local file through `self.writer` (operation `write-local`) before setting
  the baseline. `ApplyError::class` returns `None` for `Io` by design, so
  the fix changes that contract rather than patching an omission.
- `cli/work-cli/src/sync.rs`: `render_report` renders a `None` class as `-`,
  and `exit_code_for_report` drops unclassified failures, falling back to
  `CLEAN`. `ApplyError`'s `Display` already names the item, operation and
  cause; the report discards it.
- `cli/work-adapters/src/sync/apply.rs`: `SyncPorts.writer` also writes
  pending-push markers (`marker-write`) under `<paths.integrations>`, so
  splitting it means the applier routes marker writes to the
  integration-state writer and only pulled work-item files to the
  work-item writer. `BaselineStore::set` rewrites `last-sync.json` once per
  item, so `baseline-set` fails per item rather than at the end of the run.
- `cli/work-cli/src/sync.rs`: `persist_conflict_dossiers` calls
  `clear_stale_dossiers`, which removes every id-shaped `.md` in the
  conflicts directory regardless of which items the run planned.
- The containment refusal went unexercised because `sync_run.rs` and
  `sync_apply.rs` use spy and fake writers, and `sync_settled.rs` uses a real
  `FileCorpusStore` rooted at the whole repository — wider than the root
  the CLI wires.

## Drafting Notes

- Kind is `bug` and priority `high`, as specified. Pulls are one half of
  bidirectional sync, and every one of them is broken.
- Dossier pruning was brought into scope after review: it breaks the same
  resolve-one-at-a-time flow that a working `--resolve <id>=remote` enables.
  The title still names the primary defect.
- I/O failures are classed by whether any earlier write in the same apply
  landed — to a local file, the baseline or the tracker. Before one, a
  re-run is identical (retryable); after one, the file, baseline and
  tracker disagree, so the failure needs attention (terminal) rather than
  a blind re-run that would surface a spurious conflict.
- The retryable write failure is induced by making the work directory
  non-writable rather than a file read-only, because writes go through a
  temp-file-and-rename that a read-only target file does not stop.
  Permission-based injection needs a non-root test user; the terminal case
  uses a wrapped writer instead, so it targets `baseline-set` exactly.
- Coupled to the whole of 0296 rather than its Defect 2 alone, so its open
  list-marker question is settled deliberately before pulls start writing,
  not by default. This accepts that 0300's release waits on all of 0296.
- Two separately bounded writers are required rather than one widened store,
  because a single store cannot refuse paths outside both roots.

## References

- Related: 0146, 0257, 0285, 0296
- Sync engine origin: 0194 —
  `meta/plans/2026-08-13-0194-tracker-crate-and-remote-sync-engine.md`,
  `meta/research/codebase/2026-08-12-0194-tracker-crate-and-remote-sync-engine.md`
- Targeted sync: `meta/plans/2026-09-06-0257-sync-specific-work-items.md`
- Review: `meta/reviews/work/0300-sync-pulls-of-tracked-work-items-fail-silently-review-1.md`
