---
type: "work-item"
id: "0302"
title: "Work Item Re-Key Command"
date: "2026-09-24T22:35:39+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "ready"
kind: "story"
priority: "medium"
parent: "work-item:0146"
blocked_by: ["work-item:0230"]
blocks: ["work-item:0343", "work-item:0346"]
relates_to: ["work-item:0228", "work-item:0297", "work-item:0345"]
external_id: "PP-879"
tags: ["sync", "tracker", "id-generation", "rekey"]
last_updated: "2026-10-15T22:09:14+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0302: Work Item Re-Key Command

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a developer who changes `work.id_pattern`, I want a command that translates
existing work-item IDs into the new pattern's shape, so that the corpus stays
uniform instead of carrying legacy IDs indefinitely. An `id` changes only
through draft promotion, a tracker-side key change (both 0230), or re-keying;
all three retire the old ID through 0230's ID retirement, which records it as
an alias. A `/rekey-work-items` skill previews the change and asks before
applying it.

## Context

Switching `id_pattern` — `{number:04d}` → `{key}-{number:04d}`, or → `{tracker}`
(0230) — leaves existing items on their old shape. 0230 deliberately keeps them
(the legacy corpus is valid, resolvable by `external_id` and `aliases`) and defers
translation to an explicit, user-triggered command rather than a migration, so it
can run whenever the pattern changes, in either direction.

0230 has shipped the primitives this command composes: ID retirement, alias
resolution, draft minting, and key-change detection. A corpus that has lived
through several pattern switches can hold `0042`, `ACC-0042`, `PP-760`, and
`draft-` IDs at once; the command converges all of them onto the current
pattern.

## Requirements

- Under `{tracker}`, `work rekey --preview` and `work rekey` with no run plan
  refresh each synced item's `external_id` from the tracker before computing
  targets, using 0230's key-change detection, so a target derived from
  `external_id` is the issue's current key. It reads through the configured
  retrieval strategy; under the bulk strategy one `fetch_all` partitions the
  keys into `found`, `absent`, and `indeterminate`, and each key not `found`
  is looked up individually with `locate`. Any read it cannot complete makes
  the tracker unreachable: it exits 70 and writes nothing. The run plan
  carries the refreshed `external_id`, which the apply writes. Executing or
  finishing a recorded run plan, and any run under `{key}`/`{number}`
  patterns, makes no tracker call.
- `fetch_all` reports an issue as found only under its current key, as a
  contract obligation every tracker adapter is tested against, so a moved
  issue always reaches `locate`.
- `accelerator work rekey` converges every item onto the current
  `work.id_pattern` in one run, whatever shape its `id` has:
  - `{tracker}`: a synced item (one with an `external_id`) targets its
    `external_id`; an unsynced item becomes a `draft-` item in
    `meta/work/drafts/` per 0230; an existing `draft-` item is left for sync
    to promote.
  - `{key}`/`{number}` patterns: a tracker key (an issue identifier the
    tracker assigned, recognised by `id` equalling `external_id`) or a
    `draft-` ID takes the next allocated number, in ascending `date` order,
    then by old `id` compared byte-wise; any other `id` is from an earlier
    numeric pattern (`0042`, `ACC-0042`) and keeps its sequence number,
    re-rendered. Fresh numbers start above the highest sequence number the
    run keeps or finds in any existing `id` or alias in the target shape.
- A run plan records each item's `old → new` ID, its refreshed
  `external_id`, and every path the run will write. It is either proposed
  or in progress, and is removed when the run completes:
  - `work rekey --preview` prints one `old → new` line per changing item and
    records a proposed run plan, replacing any proposed one, changing
    nothing under `meta/`; it refuses while a run plan is in progress.
  - `work rekey` with no run plan computes one and records it in progress
    before its first write.
  - `work rekey` with a proposed run plan executes it exactly as previewed,
    refusing if the identity fingerprint (every item's path, `id`, `aliases`,
    and `external_id`) has changed since it was recorded.
  - `work rekey` with a run plan in progress finishes it before computing
    anything new, without a fingerprint check; it ignores dirty paths in the
    run plan's write set and still refuses on any other.
  - `work rekey --discard` removes a proposed run plan, does nothing when
    there is none, and refuses while a run plan is in progress.
  - A refused run, including one that exits 70, leaves any run plan as it
    found it.
- Per item, retire the old ID through 0230's ID retirement: rename the file,
  set `id` and H1, append the old ID to `aliases` and drop the new `id` from
  them. References to the old ID within `meta/` are rewritten to the new ID:
  typed links always, and prose, whole tokens only, when the old ID has a
  distinctive shape: a `draft-` ID, or a prefixed ID (`<PREFIX>-<number>`),
  whether a tracker key such as `PP-760` or a local ID such as `ACC-0042`. A
  bare numeric old ID has only its typed links rewritten.
- Refuse to apply on a dirty working copy, using the dirty-tree detection
  `accelerator migrate` uses, over the same scope; for `work rekey` only, a
  working copy that cannot be probed counts as dirty, while `migrate` keeps
  treating it as clean. `ACCELERATOR_FORCE=1` overrides the refusal for
  both commands; `accelerator migrate` keeps honouring
  `ACCELERATOR_MIGRATE_FORCE` as a deprecated alias, to be removed in a future
  release.
- Refuse the whole run if any target `id` collides with another item's `id`
  or alias, or with another item's target in the same run; an item's own
  `id` and aliases never collide with its target.
- Refuse the whole run, naming the items, while `work sync` has an unfinished
  retirement record (a key change or promotion `work sync` recorded but has
  not completed) or any item the run would re-key has a `pending_push`
  marker; the user finishes them with `work sync` first.
- `work sync` refuses, naming the run plan, while a re-key run plan is in
  progress; the user finishes it with `work rekey` first.
- A run changes nothing when every item's `id` already equals its target,
  no synced item's issue has moved, and no run plan is in progress.
- A `/rekey-work-items` skill wraps the command with preview and confirmation.

**Out of scope**

- Rewriting references outside `meta/` or in remote issue bodies.
- Rewriting references to IDs that were already aliases before the run, and
  promoting prose mentions to typed links (work-item:0343).
- Deciding whether bare numbers in prose name work items (work-item:0345).
- Removing the deprecated `ACCELERATOR_MIGRATE_FORCE` alias (work-item:0346).

## Acceptance Criteria

- [ ] Given `{tracker}` and legacy item `0042` with `external_id: "PP-760"`,
      when `work rekey` runs, then the file is `meta/work/PP-760-<slug>.md`,
      `id` and H1 are `PP-760`, `aliases` contains `0042`, every
      `work-item:0042` in `meta/` reads `work-item:PP-760`, and a prose
      mention of `0042` is unchanged.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"`, and synced item
      `PP-760` named in `meta/` prose, when `work rekey` runs, then its `id`
      is the next allocated `ACC-` ID and the prose names it.
- [ ] Given `{tracker}` and legacy item `0042` with `external_id: "PP-760"`
      whose issue has moved to `ENG-42` since the last sync, when
      `work rekey` runs, then `id` and `external_id` are `ENG-42` and
      `aliases` contains `0042`.
- [ ] Given `{tracker}` and an unreachable tracker, when `work rekey` runs,
      then it exits 70 and writes nothing.
- [ ] Given `{tracker}` and a bulk read that leaves a key `absent` or
      `indeterminate` whose `locate` fails, when `work rekey` runs, then it
      exits 70 and writes nothing.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"`, highest number
      `0041`, synced item `PP-760`, and an unreachable tracker, when
      `work rekey` runs, then it makes no tracker call and `PP-760` becomes
      `ACC-0042`.
- [ ] Given a tracker adapter and an issue moved from `PP-760` to `ENG-42`,
      when `fetch_all` is asked for `PP-760`, then `PP-760` is not in
      `found`, and `locate` reports `ENG-42` (a contract test every adapter
      passes).
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"` and item `0042`, when
      `work rekey` runs, then its `id` is `ACC-0042`.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"` and items `ACC-0041`,
      legacy `0042`, and synced `PP-760`, when `work rekey` runs, then `0042`
      becomes `ACC-0042` and `PP-760` becomes `ACC-0043`.
- [ ] Given `{number:04d}` and item `ACC-0042` with `aliases: ["0042"]`,
      when `work rekey` runs, then its `id` is `0042` and `aliases` is
      `["ACC-0042"]`.
- [ ] Given `{number:04d}` and item `ACC-0042` named in `meta/` prose, when
      `work rekey` runs, then the prose names `0042`.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"` and a corpus holding
      `0041`, synced `PP-760`, and `draft-k3x9`, when `work rekey` runs, then
      `0041` becomes `ACC-0041`, the other two take the next allocated `ACC-`
      numbers, and every `id` matches `ACC-NNNN`.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"`, highest number
      `0041`, and items `PP-760` dated `2026-02-01` and `draft-k3x9` dated
      `2026-01-01`, when `work rekey` runs, then `draft-k3x9` becomes
      `ACC-0042` and `PP-760` becomes `ACC-0043`.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"`, highest number
      `0041`, and items `draft-a1b2` and `PP-761` both dated `2026-01-01`,
      when `work rekey` runs, then `PP-761` becomes `ACC-0042` and
      `draft-a1b2` becomes `ACC-0043`, old `id`s compared byte-wise.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"`, highest number
      `0041`, item `draft-k3x9` dated `2026-01-01` and named in `meta/` prose,
      item `PP-760` dated `2026-02-01`, and a prose token `PP-7601`, when
      `work rekey` runs, then the prose names `ACC-0042` where it named
      `draft-k3x9`, and `PP-7601` is unchanged.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"` and items `ACC-0040`,
      `0041`, and `draft-k3x9`, when `work rekey --preview` runs, then it
      prints exactly `0041 → ACC-0041` and `draft-k3x9 → ACC-0042`, records
      a proposed run plan, and changes no file under `meta/`.
- [ ] Given a dirty working copy, when `work rekey --preview` runs, then it
      prints the mapping and records a proposed run plan.
- [ ] Given `{tracker}` and an unreachable tracker, when
      `work rekey --preview` runs, then it exits 70 and records no run plan.
- [ ] Given `/rekey-work-items` is invoked, when it runs, then it shows the
      `work rekey --preview` mapping and asks for confirmation before
      applying.
- [ ] Given the `/rekey-work-items` preview and the user declines, then no
      file under `meta/` changes and the skill discards the proposed run
      plan.
- [ ] Given the `/rekey-work-items` preview and the user confirms, then the
      resulting `id`s equal the previewed `old → new` lines, even if a
      tracker key moved after the preview.
- [ ] Given `{tracker}`, a proposed run plan mapping `0042 → PP-760`, an
      unchanged corpus, and the issue moving to `ENG-42` before apply, when
      `work rekey` applies, then it makes no tracker call and the item's `id`
      is `PP-760`.
- [ ] Given a proposed run plan and a corpus whose identity fingerprint has
      changed since, when `work rekey` applies, then it exits non-zero naming
      the changed items, tells the user to re-preview, and writes nothing.
- [ ] Given a proposed run plan and a corpus changed since, when
      `work rekey --preview` runs again, then the proposed run plan holds
      only the new mapping.
- [ ] Given a proposed run plan and a dirty working copy, when `work rekey`
      runs, then it refuses and the proposed run plan is unchanged.
- [ ] Given a proposed run plan in a repository with no prior `rekey` ignore
      rule, when `work rekey` runs, then the working copy is not reported
      dirty because of the run plan.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"`, a run plan in
      progress mapping `draft-k3x9 → ACC-0042` and `PP-760 → ACC-0043`, and
      a run interrupted after `ACC-0043-<slug>.md` is created and some
      `work-item:PP-760` links are rewritten but before `PP-760-<slug>.md` is
      removed, when `work rekey` runs again, then it finishes without the
      dirty-tree refusal: `draft-k3x9` is `ACC-0042`, exactly one item is
      `ACC-0043` with `aliases` containing `PP-760`, no `PP-760-<slug>.md`
      remains,
      every `work-item:PP-760` reads `work-item:ACC-0043`, and the run plan
      is removed.
- [ ] Given the same interrupted run and an uncommitted edit to a `meta/`
      file outside the run plan's recorded paths, when `work rekey` runs
      again, then it refuses naming that file and writes nothing.
- [ ] Given a run plan in progress, when `work rekey --preview` runs, then
      it exits non-zero, tells the user to run `work rekey` to finish it, and
      changes nothing.
- [ ] Given a proposed run plan, when `work rekey --discard` runs, then no
      run plan remains and no file under `meta/` changes.
- [ ] Given a run plan in progress, when `work rekey --discard` runs, then
      it exits non-zero, tells the user to run `work rekey` to finish it, and
      leaves the run plan unchanged.
- [ ] Given no run plan, when `work rekey --discard` runs, then it exits 0
      and changes nothing.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"` and items `0042` and
      `ACC-0042`, neither aliasing the other, when `work rekey` runs, then it
      exits non-zero naming both items and writes nothing.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"`, item `0042`, and
      item `ACC-0050` with `aliases: ["ACC-0042"]`, when `work rekey` runs,
      then it exits non-zero naming both items and writes nothing.
- [ ] Given `{number:04d}` and items `ACC-0042` and `XYZ-0042`, when
      `work rekey` runs, then it exits non-zero naming both items and writes
      nothing.
- [ ] Given `{key}-{number:04d}` and `draft-k3x9` with a `created`
      `pending_push` marker, when `work rekey` runs, then it exits non-zero
      naming `draft-k3x9` and writes nothing.
- [ ] Given an unfinished `work sync` retirement record for `0042 → PP-760`,
      when `work rekey` runs, then it exits non-zero naming `0042`, tells the
      user to run `work sync`, and writes nothing.
- [ ] Given `{tracker}`, legacy item `0042` with `external_id: "PP-760"`, and
      an existing `draft-k3x9` with an `attempted` `pending_push` marker, when
      `work rekey` runs, then `0042` becomes `PP-760` and `draft-k3x9` is
      unchanged.
- [ ] Given a re-key run plan in progress, when `work sync` runs, then it
      exits non-zero telling the user to run `work rekey` and changes
      nothing.
- [ ] Given a dirty working copy, when `work rekey` runs, then it exits
      non-zero naming the dirty paths and writes nothing.
- [ ] Given a working-copy probe that fails, when `work rekey` runs, then
      it exits non-zero naming the failed probe and writes nothing; with
      `ACCELERATOR_FORCE=1` it applies.
- [ ] Given a dirty working copy and `ACCELERATOR_FORCE=1`, when
      `work rekey` or `accelerator migrate` runs, then it applies.
- [ ] Given a dirty working copy and `ACCELERATOR_MIGRATE_FORCE=1`, when
      `accelerator migrate` runs, then it applies.
- [ ] Given `ACCELERATOR_MIGRATE_FORCE=1`, when `accelerator migrate` runs,
      then it prints a deprecation notice naming `ACCELERATOR_FORCE` to
      stderr.
- [ ] Given a dirty working copy and only `ACCELERATOR_MIGRATE_FORCE=1`, when
      `work rekey` runs, then it exits non-zero naming the dirty paths and
      writes nothing.
- [ ] Given a working-copy probe that fails, when `accelerator migrate` runs,
      then it treats the working copy as clean, as before.
- [ ] Given `{tracker}` and unsynced legacy item `0042`, when `work rekey`
      runs, then `meta/work/drafts/<draft-id>-<slug>.md` exists with `id` and
      H1 `<draft-id>`, `aliases` contains `0042`, every `work-item:0042` in
      `meta/` reads `work-item:<draft-id>`, and no `0042-<slug>.md` remains in
      `meta/work/`.
- [ ] Given `{tracker}` and synced item `PP-760` whose issue has not moved,
      when `work rekey` runs, then the item is unchanged.
- [ ] Given `{tracker}` and synced item `PP-760` whose issue has moved to
      `ENG-42`, when `work rekey` runs, then `id` and `external_id` are
      `ENG-42` and `aliases` contains `PP-760`.
- [ ] Given `{tracker}` and an existing `draft-` item, when `work rekey`
      runs, then the item is unchanged and no tracker issue is created.
- [ ] Given a re-keyed corpus, when `work rekey` runs again, then nothing
      changes.
- [ ] Given a re-keyed item, when `work resolve` runs on its old ID, then it
      returns the item's new path.

## Open Questions

None.

## Dependencies

- Blocked by: work-item:0230 — Tracker-Owned Work Item ID Generation (done);
  supplies ID retirement, alias resolution, draft minting and key-change
  detection.
- Blocks: work-item:0343 — Alias Normalisation for Re-Keyed Work Items
  (`--normalise-aliases`, split out of this item).
- Blocks: work-item:0346 — Remove Deprecated ACCELERATOR_MIGRATE_FORCE; this
  item changes `accelerator migrate`'s override contract.
- Related: work-item:0345 — Bare-Number Review During Alias Normalisation,
  blocked by work-item:0343 and work-item:0344 (Reusable Interactive Decision
  Engine).
- Related: 0297 — qualified prose references would let re-keying rewrite
  numeric IDs in prose too; 0343 promoting prose to typed links ahead of 0297
  is an accepted trade-off.
- Related: work-item:0228 (done) — supplies `work.key` and the `{key}`
  placeholder.
- Related: work-item:0181 — Additional Integrations; new adapters inherit the
  `fetch_all` current-key contract obligation.
- External: the Linear and Jira Cloud issue APIs, reachable for the whole
  `{tracker}` refresh, and their old-key-to-current-key resolution (see
  Assumptions).

## Assumptions

- Moving between numeric patterns keeps the sequence number and never
  reallocates it.
- Under `{tracker}`, a synced item whose `id` equals its `external_id` and
  whose issue has not moved is unchanged; one whose issue has moved is
  re-keyed like any other.
- "Dirty" means what `accelerator migrate`'s check means: changes under
  `meta/`, `.claude/accelerator`, or `.accelerator/`, untracked files
  included.
- A persisted run-plan record that a re-run finishes before computing
  anything new is sufficient recovery from a partial run; no run-wide
  rollback is needed.
- Linear and Jira Cloud answer a moved issue's old key with its current key,
  as 0230 verified; the `external_id` refresh depends on it.
- An external key that moves between preview and apply is left for the next
  `work sync` to retire; the apply trusts the keys the preview refreshed.
- Under `{key}`/`{number}` patterns no tracker call is needed: the stored
  `external_id` only identifies an item as a tracker key, never supplies its
  target, and stale keys are sync's concern.

## Technical Notes

- `plan_retirement` (`cli/work/src/retirement.rs`) and
  `finish_retirement` / `apply_retirement` (`cli/work-adapters/src/retirement.rs`)
  already rename the file, update `id`, H1, `aliases`, references, and the sync
  baseline atomically. Build a `Retirement` with `new_external_id` set only for
  a moved key.
- `corpus::references::rewrite_references` already implements the
  distinctive-versus-numeric prose rule (`IdShape`, `shape_of`).
- Key-change detection lives only inside `work sync`
  (`work-adapters/src/sync/fetch.rs`, `decide_key_change` in
  `cli/work/src/sync/key_change.rs`). There a failed `locate` silently keeps
  the bulk-read key; `rekey` must surface that failure as unreachable (exit
  70) instead.
- No existing code turns an existing item into a draft, and retirement always
  targets `work_dir/<new>-<slug>.md`. Unsynced items under `{tracker}` need a
  retirement destination in `drafts/`; `mint_draft_id`
  (`cli/work/src/draft_id.rs`) supplies the ID.
- No function extracts a sequence number from an ID of arbitrary shape;
  `parse_full_id` (`cli/corpus-adapters/src/work_item_pattern.rs`) parses only
  against the current pattern. Migration `m0002`'s private `split_legacy_id`
  is the nearest precedent. Fresh numbers come from
  `work::next_number::allocate`.
- The dirty-tree check is `Preflight` (`cli/migrate/src/preflight.rs`), backed
  by `migrate-adapters/src/working_copy.rs`; it needs lifting somewhere
  `work-adapters` can depend on, with the force override renamed.
- `apply_retirement` is a compensating rollback, not a transaction: a killed
  process leaves some references rewritten and `to` and `from` both present.
  `plan_retirement` resumes such an item only when handed the same
  `old → new` pair, and `allocate` reads filenames and reserves nothing, so
  recomputing a freshly allocated target after an interruption duplicates the
  item. Persist the whole run's mapping first, as `work sync` does with its
  `RetirementRecord` (`work-adapters/src/sync/identity_settlement.rs`).
- The run plan lives in `.accelerator/state/rekey/`, which `work rekey`
  creates with its own ignore-everything `.gitignore` (as
  `.accelerator/tmp/` has), so it never trips the dirty-tree check and
  existing repositories need no migration. `.accelerator/state/` itself is
  tracked.
- Fresh numbers come from one `allocate` call with `count` set, assigned in
  `date` order, then old `id` byte-wise. `allocate` sees only current-pattern
  filenames, so the run raises its floor to the highest kept sequence number.
- `refuse_collisions` already ignores the retiring item's own aliases, but
  `append_alias` never removes the new `id` from `aliases`; retirement must
  strip it so a round trip leaves no self-alias.
- Under `{tracker}`, refresh through the configured `RetrievalStrategy`. With
  `Bulk`, an issue in `found` has not moved, so only `absent` and
  `indeterminate` keys need `locate`, and no `show` is needed. Unlike
  `gather` in `sync/fetch.rs`, a failed `locate` or a `fetch_all` error is
  unreachable.
- Both adapters already keep a moved issue out of `found` (Linear indexes
  current identifiers; Jira matches returned keys exactly), but `FetchOutcome`
  does not oblige it. Add that obligation to the `fetch_all` docs in
  `tracker/src/lib.rs` and a contract test beside
  `fetch_all_partitions_totally` in `tracker-test-support/src/contract.rs`.
- The working-copy probe in `migrate-adapters/src/working_copy.rs` treats a
  failed probe as clean; `rekey` refuses instead unless `ACCELERATOR_FORCE` is
  set, while `migrate` keeps its behaviour.
- Add a `Rekey` variant to the `Command` enum in `cli/work-cli/src/cli.rs`; the
  launcher forwards `work` subcommands generically, so no sub-binary
  registration applies. Model the skill on `skills/work/sync-work-items/`,
  which previews then confirms through `AskUserQuestion`.

## Drafting Notes

- A command rather than a migration, per user direction: it must be re-runnable
  on every pattern change, which a one-shot migration is not.
- Prose is rewritten only for distinctive ID shapes (`draft-` IDs and
  prefixed IDs), because bare numeric IDs collide with ordinary numbers in
  prose.
- Tracker keys and `draft-` IDs take freshly allocated numbers under numeric
  patterns, because their digits are not local sequence numbers; reusing them
  risks collisions with existing local IDs.
- `--preview` (bare command applies) over `--dry-run` or `--apply`, matching
  `work sync`: re-keying is a corpus reconciliation like sync, and the
  dirty-tree refusal keeps an apply undoable through VCS.
- Renaming `ACCELERATOR_MIGRATE_FORCE` to `ACCELERATOR_FORCE` reaches into
  `accelerator migrate`, per user direction, so both guarded commands share
  one override; the old name survives as a deprecated alias.
- Under `{tracker}` re-keying follows moved keys itself rather than deferring
  to `work sync`, because its targets are the current keys; both retire
  through the same primitive, so whichever runs first leaves the other a
  no-op.
- `{tracker}` and numeric re-keying stay one story, per user direction: the
  command's promise is one-run convergence from any mix of shapes, and both
  modes share the run plan, collision checks, and retirement.

## References

- Blocked by: 0230 — Tracker-Owned Work Item ID Generation
- Blocks: 0343 — Alias Normalisation for Re-Keyed Work Items
- Blocks: 0346 — Remove Deprecated ACCELERATOR_MIGRATE_FORCE
- Related: 0345 — Bare-Number Review During Alias Normalisation
- Related: 0297 — Fully Qualified Artifact References in Prose
- Related: 0228 — Layered Configuration Key Model (`{project}` → `{key}`)
- Parent: 0146 — Work Item Synchronisation Enhancements
- Codebase: `cli/work/src/retirement.rs`,
  `cli/work-adapters/src/retirement.rs`, `cli/corpus/src/references.rs`,
  `cli/migrate/src/preflight.rs`, `cli/corpus-adapters/src/work_item_pattern.rs`
