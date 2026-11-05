---
type: "work-item"
id: "0319"
title: "Explicit Work Item ID on accelerator work create"
date: "2026-10-09T09:34:53+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0146"
blocks: ["work-item:0371"]
relates_to: ["work-item:0228", "work-item:0230", "work-item:0302", "work-item:0343"]
tags: ["work", "cli", "allocation"]
last_updated: "2026-11-02T11:36:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-905"
---

# 0319: Explicit Work Item ID on accelerator work create

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As someone creating work items while other branches have unmerged items,
I want to pass the work item ID, under `work.key` or any other key, to
`accelerator work create`, directly or through `/create-work-item`, so
that new items avoid numbers already used on other branches without
bypassing the CLI's atomic write and tracker push.

## Context

`work create` always takes max+1 over the local work directory
(`allocate_id` in `cli/work-cli/src/create.rs`). It cannot see numbers used
on other branches, so parallel work clashes at merge. Today's workaround
writes the file by hand, which skips the `NNNN` substitution (replacing
the template's ID placeholder throughout the body), the create-time
validation, and `--push`, leaving the item unsynced. `/create-work-item`
projects the ID with `work id next` before calling `work create`, so
a user-requested ID has no path through the skill either.

## Requirements

- Add an `--id <ID>` option to `accelerator work create`. When omitted,
  allocation stays max+1 per key, ignoring any gap an `--id` leaves. The
  per-key scan ignores letter case in every allocator, `work create` and
  `work id next` alike, so `pp-0500` counts in `PP`'s sequence and the
  skill's projection agrees with the write.
- Require an `--id` in the exact shape the configured `work.id_pattern`
  produces: the `{number}` padding exactly, and a key segment wherever the
  pattern has `{key}`. Do not normalise short or unpadded numbers. The
  shape does not constrain the key's value or letter case: accept any key
  matching `[A-Za-z][A-Za-z0-9]*`, not only `work.key`, and write the ID
  in the letter case given.
- Accept any number from 1 to the pattern's cap (9999 under
  `{number:04d}`). A number outside that range is a pattern mismatch.
- Refuse a taken ID. The held IDs are every work item's `id`, `aliases`
  and `external_id`, and every filename's ID prefix (what the pattern's
  scan captures, at any width). A held ID the pattern can parse takes the
  `--id` when its key, ignoring letter case, and its number are equal, so
  `400-<slug>.md` and `id: "400"` both take `0400`. Any other held ID,
  such as a draft ID, takes it when the two are equal ignoring letter
  case.
- Refuse `--id` when the pattern is `{tracker}`, since the tracker owns
  every ID.
- Refuse `--id` together with `--project`, since the ID already carries its
  key.
- Refuse, under every pattern, a draft ID: `draft-` followed by six
  Crockford base-32 characters, at least one a letter, in any letter case
  (the form `canonical_draft_id` accepts). That form is reserved for
  provisional drafts.
- Run the `--id` refusals in this order, so every refusal has an input
  that reaches it: `--id` with `--project`, the `{tracker}` pattern, draft
  reservation, pattern mismatch, then collision. The `--id` with
  `--project` refusal runs before the existing `--project` checks. In a
  real create, all five run after the existing re-key guard
  (`E_REKEY_IN_PROGRESS`), which refuses before anything is read.
- Give each of these five refusals its own `E_` identifier, distinct from
  each other and from every existing `work create` identifier, including
  `E_PATTERN_KEY_UNUSED` and `E_PATTERN_OVERFLOW`. Each exits with status
  1, as `work create`'s existing argument refusals do. Each message
  contains its cause: both `--id` and `--project` (`--id` with
  `--project`), the configured pattern verbatim (`{tracker}`, mismatch),
  `draft-` (draft reservation), or the holder's file path (collision).
  Below, "exits with the X refusal" means exit status 1 with X's `E_`
  identifier.
- Under `--push`, run every `--id` check before any tracker request. A
  push that fails after the checks pass behaves as it does without `--id`,
  and the item is written under the given ID.
- Under `--dry-run`, with or without `--push`, run every `--id` check
  before the tracker preview. A refusal matches the real create's exit
  status and identifier; a pass leaves the dry-run output unchanged.
- Let `/create-work-item` take a user-requested ID:
  - The user supplies it through a dedicated `--id <ID>` skill argument,
    parsed before `work resolve`, so it is never read as a reference to an
    existing item.
  - The skill settles the requested ID with `work id check` before
    composing the item. An exact-form request is used as given. Otherwise
    `work id check` may make only two corrections, which the skill
    proposes: left-pad a short number, and, under a `{key}` pattern, add
    `work.key` to a request with no key. A key the request carries is kept,
    in its letter case. The skill continues only once the user confirms
    the proposal. A request no such correction
    makes exact has no exact form: out of range, over-padded, a draft ID,
    a key under a keyless pattern, a missing separator, or not an ID. The
    skill explains it against the pattern before composing the item,
    without calling `work create`.
  - Under `{tracker}`, the skill refuses the requested ID before composing
    the item.
  - The skill passes the ID via `--id` on both the push-gate dry-run
    (`work create --push --dry-run`) and the real create, and shows it as
    the projected ID (the ID the skill reports before writing). It relays
    any CLI refusal verbatim.
  - After a declined proposal, a CLI refusal or a request with no exact
    form, the user may name another ID or explicitly ask for automatic
    allocation. After a `{tracker}` refusal, only automatic allocation
    remains, through the skill's normal `{tracker}` flow. The skill waits
    for the user's choice and never makes it; once the user asks for
    automatic allocation, it proceeds as it would with no requested ID.
- Group the ID verbs under `work id`. `work id canonicalise` and
  `work id next` replace `canonicalise-id` and `next-number` with unchanged
  behaviour and no alias. A new `work id check <ID>` prints the exact ID,
  the exact ID with a note that its key differs from `work.key`, or the one
  correction the skill may propose, or refuses as `work create --id`
  would, writing nothing.
- Move work-item identity out of the general-purpose `corpus` crates into
  `work`, consolidating the pattern grammar, the canonicaliser, the
  tracker-key grammar and the taken rule on the way. `corpus` keeps general
  ports for slugs, clusters and linkage.
- Add the `--id` flag only to `work create`, and take a requested ID only
  in `/create-work-item` and `work id check`. `extract-work-items` and
  `create-batch` keep automatic allocation, and no change to
  `/sync-work-items` is in scope; its criterion below guards against
  regression.

## Acceptance Criteria

- [ ] Given `--id 0400` under `{number:04d}` and 0400 not taken, when
      `work create` runs, then it writes `0400-<slug>.md` with `id: "0400"`
      and H1 `# 0400: <title>`, and no `NNNN` remains in the file.
- [ ] Given `--id XY-0400` under `{key}-{number:04d}` with `work.key: PP`
      and `XY-0400` not taken, when `work create` runs, then it writes
      `XY-0400-<slug>.md` with `id: "XY-0400"` and H1
      `# XY-0400: <title>`, and no `NNNN` remains in the file.
- [ ] Given `{key}-{number:04d}` with `work.key: PP` and nothing taken,
      when `work create` runs with each of `--id xy-0400` and
      `--id Xy-0400`, then it writes the item with that exact `id`,
      filename prefix and H1.
- [ ] Given `{key}-{number:04d}` and nothing taken, when `work create` runs
      with each of `--id X-0400` and `--id X1-0400`, then it writes the
      item.
- [ ] Given `--id` naming a taken ID, whether through a filename under any
      slug, an `id`, an `aliases` entry or an `external_id`, when `work
      create` runs, then it exits with the collision refusal, naming the
      holder's file path, and writes nothing.
- [ ] Given `{number:04d}` and, separately, an existing `400-<slug>.md`,
      an existing `00400-<slug>.md` and an item with `id: "400"`, when
      `work create --id 0400` runs, then it exits with the collision
      refusal, since the key and number are equal.
- [ ] Given `{key}-{number:04d}` and an existing `XY-0400`, when `work
      create --id xy-0400` runs, then it exits with the collision refusal,
      since case is ignored.
- [ ] Given `{key}-{number:04d}` and an item holding `xy-0400` as its
      filename prefix, an `aliases` entry or its `external_id`, when `work
      create --id XY-0400` runs, then it exits with the collision refusal.
- [ ] Given `{key}-{number:04d}` with `work.key: PP` and an existing
      `PP-0400-<slug>.md`, when `work create --id XY-0400` runs, then it
      writes the item, since a different key makes a different ID.
- [ ] Given another process holds the create lock and writes
      `0400-other-title.md` before releasing it, when `work create --id
      0400` with a different title acquires the lock, then it exits with
      the collision refusal, naming `0400-other-title.md`, and writes
      nothing.
- [ ] Given `--id` that does not match the configured `id_pattern`, when
      `work create` runs, then it exits with the mismatch refusal, naming
      the pattern, and writes nothing.
- [ ] Given `--id 400` under `{number:04d}`, when `work create` runs, then
      it exits with the mismatch refusal and writes nothing.
- [ ] Given each of `04000`, `10000` and `0000` under `{number:04d}`, and
      each of `0400`, `1X-0400` and `X_Y-0400` under `{key}-{number:04d}`,
      when `work create` runs with it as `--id`, then it exits with the
      mismatch refusal and writes nothing.
- [ ] Given `--id 0001` and, separately, `--id 9999` under `{number:04d}`,
      neither taken, when `work create` runs, then it writes the item.
- [ ] Given the `{tracker}` pattern, when `work create` runs with `--id`,
      then it exits with the `{tracker}` refusal, naming the pattern, and
      writes nothing.
- [ ] Given `--id` with each of `--project XY` and `--project 1X`, under
      each of `{number:04d}`, `{key}-{number:04d}` and `{tracker}`, when
      `work create` runs, then it exits with the `--id` with `--project`
      refusal, naming both flags, and writes nothing.
- [ ] Given each of `draft-k7mq3x` and `DRAFT-K7MQ3X`, under `{number:04d}`
      and under `{key}-{number:04d}`, when `work create` runs with it as
      `--id`, then it exits with the draft-reservation refusal, naming the
      reserved form, and writes nothing.
- [ ] Given each of `draft-k7mq3`, `draft-k7mq3xy` and `draft-k7mq3u`
      under `{number:04d}`, when `work create` runs with it as `--id`, then
      it exits with the mismatch refusal, since none is a draft ID: the
      first two have the wrong length, and `u` is outside the Crockford
      alphabet.
- [ ] Given `--id draft-040000` under `{key}-{number:06d}` with
      `work.key: draft`, when `work create` runs, then it writes the item,
      since a draft suffix needs at least one letter.
- [ ] Given `--id draft-k7mq3x` under `{tracker}`, when `work create` runs,
      then it exits with the `{tracker}` refusal.
- [ ] Given `--id 400` under `{number:04d}` and an existing
      `400-<slug>.md`, when `work create` runs, then it exits with the
      mismatch refusal.
- [ ] Given the five `--id` refusals, when each fires, then its `E_`
      identifier differs from the other four and from every existing
      `work create` identifier, and every criterion expecting that refusal
      sees the same identifier.
- [ ] Given each refusal, when it fires, then its message contains both
      `--id` and `--project`, the configured pattern verbatim, `draft-`, or
      the holder's file path, as its class requires.
- [ ] Given `--id PP-0400 --project X` under `{tracker}`, when `work
      create` runs, then it exits with the `--id` with `--project` refusal
      and writes nothing.
- [ ] Given items up to 0010 and `0400-<slug>.md` written via `--id`, when
      `work create` runs without `--id`, then it writes `0401-<slug>.md`.
- [ ] Given `{key}-{number:04d}` with `work.key: PP`, items `PP-0001` to
      `PP-0010`, and `XY-0400` written via `--id`, when `work create` runs
      without `--id`, then it writes `PP-0011-<slug>.md`, since each key
      keeps its own sequence.
- [ ] Given `{key}-{number:04d}` and `XY-0400` written via `--id`, when
      `work create --project XY` runs, then it writes `XY-0401-<slug>.md`.
- [ ] Given `{key}-{number:04d}` with `work.key: PP`, items `PP-0001` to
      `PP-0010`, and `pp-0500` written via `--id`, when `work create` runs
      without `--id`, then it writes `PP-0501-<slug>.md`, since the per-key
      scan ignores case.
- [ ] Given `9999-<slug>.md` written via `--id` under `{number:04d}`, when
      `work create` runs without `--id`, then it exits with
      `E_PATTERN_OVERFLOW` and writes nothing.
- [ ] Given `--id 0400 --push` under `{number:04d}` and a tracker that
      accepts the create, when `work create` runs, then the tracker
      receives one create request, and the file is `0400-<slug>.md` with
      `id: "0400"`, H1 `# 0400: <title>` and the returned `external_id`.
- [ ] Given `--id 0400 --push` and a tracker that rejects the create, when
      `work create` runs, then it writes `0400-<slug>.md` with `id: "0400"`
      and no `external_id`, and its exit status and outcome keyword (the
      push-outcome token on the second stdout line, such as `rejected`)
      match the same failure without `--id`.
- [ ] Given `--id 0400 --push` and an unreachable tracker, when `work
      create` runs, then it saves `0400-<slug>.md` unsynced, with no
      `external_id`, and its exit
      status and outcome keyword match the same failure without `--id`.
- [ ] Given that unsynced `0400-<slug>.md` and a tracker that is now
      reachable, when `/sync-work-items` pushes it, then the file stays
      `0400-<slug>.md` with `id: "0400"` and records the returned
      `external_id`.
- [ ] Given an `--id` that any check refuses, with `--push`, when `work
      create` runs, then it exits with that check's refusal, sends the
      tracker no request of any kind, and writes nothing.
- [ ] Given an `--id` that fails any check, when `work create --dry-run`
      runs with or without `--push`, then it exits with the same refusal
      as the real create, prints no preview, writes nothing, and sends the
      tracker no request.
- [ ] Given a valid `--id` that is not taken, when `work create --dry-run`
      runs with and without `--push`, then its output and exit status
      match the same invocation without `--id`.
- [ ] Given another process holds the create lock until the dry-run exits,
      when `work create --dry-run` runs with a taken `--id`, then it exits
      with the collision refusal while the lock is still held; with an
      untaken `--id`, it exits with the normal dry-run output while the
      lock is still held.
- [ ] Given `work create-batch` invoked with `--id 0400`, when it runs,
      then it fails as an unrecognised argument and writes nothing.
- [ ] Given `/extract-work-items` writing items under a numeric pattern,
      when it calls the CLI, then no call carries `--id`.
- [ ] Given an existing item `0400` under `{number:04d}`, when
      `/create-work-item --id 0400 <topic>` starts, then it makes no `work
      resolve` call on `0400`, does not load item 0400, and relays the
      collision refusal from `work id check` before composing an item.
- [ ] Given a requested ID, when the skill shows the projected ID, then it
      shows the requested ID; and when it creates the item, then it passes
      that ID via `--id` on the push-gate dry-run and the real create,
      having made no `work id next` call.
- [ ] Given the `{tracker}` pattern and a requested ID, when the skill
      starts, then it refuses the ID before composing the item, explaining
      that the tracker owns every ID.
- [ ] Given the CLI refuses the requested ID, when the skill calls `work
      create --id`, then it relays the CLI's message verbatim and writes
      no item.
- [ ] Given a requested "400" under `{number:04d}`, when the skill starts,
      then it asks the user to confirm the exact ID `0400` before
      composing the item, and passes only the confirmed ID.
- [ ] Given a requested `0400` under `{number:04d}`, when the skill
      starts, then it asks for no confirmation and uses `0400`.
- [ ] Given `{key}-{number:04d}` with `work.key: PP`, when the skill starts
      with a requested "400", then it proposes `PP-0400`; with "XY-400", it
      proposes `XY-0400`; with "xy-400", it proposes `xy-0400`.
- [ ] Given each of "10000", "04000", `draft-k7mq3x`, "XY-400" and "foo"
      requested under `{number:04d}`, and "XY400" under
      `{key}-{number:04d}`, when the skill starts, then it explains the
      request against the pattern before composing the item, without
      calling `work create`.
- [ ] Given a declined proposal, a CLI refusal, a request with no exact
      form or a `{tracker}` refusal, when the skill continues, then it
      makes no `work create` call and writes no item until the user
      chooses how to proceed.
- [ ] Given a CLI collision refusal, when the user names an untaken ID,
      then the skill passes the new ID via `--id` on both the push-gate
      dry-run and the real create.
- [ ] Given a declined proposal, a CLI refusal or a request with no exact
      form under a numeric pattern, when the user asks for automatic
      allocation, then the skill projects the ID with `work id next`
      and calls `work create` without `--id`.
- [ ] Given a `{tracker}` refusal, when the user asks for automatic
      allocation, then the skill continues through its `{tracker}` flow
      and passes no `--id`.
- [ ] Given `{number:04d}` and `0400` not taken, when `work id check 0400`
      runs, then it prints `exact` with `0400`; given `400`, it prints
      `corrected` with `0400`.
- [ ] Given `{key}-{number:04d}` with `work.key: PP` and `XY-0400` not
      taken, when `work id check XY-0400` runs, then it prints `other-key`
      with `XY-0400`, noting the configured key `PP`.
- [ ] Given any `--id` that `work create` refuses, when `work id check`
      runs with that ID, then it exits with the same refusal; and whatever
      it prints or refuses, it writes nothing.
- [ ] Given `work canonicalise-id` or `work next-number`, when either runs,
      then it runs nothing, exits 2 and names its replacement under
      `work id`.
- [ ] Given the `corpus` crates, when their public items are listed, then
      none names work-item identity.
- [ ] Given `PROJ-42`, when the CLI, filters, resolution and the
      visualiser's clusters canonicalise it, then each yields `PROJ-0042`.
- [ ] Given IDs whose keys differ only in letter case, when the visualiser
      matches them, then it ignores case and shows each ID as written.
- [ ] Given `v{number:03d}`, when an item is allocated, then it writes
      `v001`; and when `work rekey` runs, then it re-keys `v001`.
- [ ] Given a `work rekey` run plan targeting a path outside the work
      directory, when the re-key runs, then it refuses.
- [ ] Given a malformed `work.id_pattern` or `work.key`, when any command
      reads it, then it refuses, naming the setting.
- [ ] Given the existing migrations, when they run, then their outputs do
      not change, except m0002 under a newly refused pattern.

## Open Questions

None.

## Dependencies

- Blocked by: none. The `{key}` syntax (0228), and the `{tracker}`
  ownership, draft-ID form, `aliases` and identity set (0230) it builds on
  are done.
- Blocks: 0371, which makes `--dry-run` run every invocation check the
  real create runs, in the real create's order. It does so by moving the
  early return in `run_with` that today skips all validation, and it makes
  the `/create-work-item` dry-run call carry every other create flag. This
  story lands first so the skill's push gate catches a taken `--id`
  straight away; 0371 keeps the five `--id` checks in its shared
  validation and the `--id` this story adds to the skill's dry-run call.
- Coordinates with: 0302 (status ready), which converges every key onto
  `work.key`, keeping sequence numbers, and refuses when two items
  converge on one ID. `--id` inherits that conflict from `--project`,
  which already allocates a sequence per key. 0302's fresh-number floor
  already ignores key case; its other ID comparisons should follow the
  taken check's key-and-number rule. 0343 rewrites references to retired
  IDs and never changes `aliases`, so it does not affect the taken check.
  Neither imposes an ordering on this story.
- External: the configured tracker integration, for the `--push`
  criteria. They are verified against a recording tracker, as
  `create.rs`'s existing push tests are.

## Assumptions

- Whether an ID is taken is decided by the local work directory only.
  Other branches are invisible to the CLI; choosing a number unused across
  branches stays the caller's job.
- Allocation ignores gaps an `--id` leaves because filling them would
  reintroduce cross-branch clashes.
- Accepting numbers up to the cap means an `--id 9999` under
  `{number:04d}` exhausts automatic allocation until the item is renamed
  or the pattern widened. The existing `E_PATTERN_OVERFLOW` message names
  that remedy.
- Comparing key and number means a tracker key held as `external_id`
  takes the local ID with the same key and number: when `work.key` and
  the tracker key coincide, an item with `external_id: "PP-905"` takes
  `PP-0905`.

## Technical Notes

- In a real create, the first four checks run before `creation_context`
  creates the work directory. The scheme they need is read inside it.
  `creation_context` already refuses `--project` under
  `IdOwnership::Tracker` with `E_PATTERN_KEY_UNUSED`. The collision check takes the place of the
  `allocate_id` step and runs once, under the create lock (the
  `LockdirLock` that serialises creates in the work directory). That step
  sits in the same atomic write as the `NNNN` substitution and before
  `push_legacy_item`, so a refusal sends nothing.
- `work create`'s existing argument refusals print `Error: <message>` and
  exit with `ExitCode::FAILURE` (`cli/work-cli/src/main.rs`); the `--id`
  refusals follow the same contract.
- The existing guard, `refuse_to_overwrite`, checks only the exact
  `{id}-{slug}.md` path and misses a same-ID file under another slug. The
  collision check matches the identity set `work resolve` uses, which
  `Creation::identities` in `work-adapters/src/creation.rs` already lists
  for draft minting. It adds the filename prefix read by
  `work::id_pattern`, keyed on the key parsed from the `--id` rather than
  `work.key`. Aliases count because `work resolve` treats an ID held as one
  item's alias and another's `id` as ambiguous.
- `work::id_pattern` reads a filename's key ignoring letter case, so the
  collision check and both allocators that scan per key, `allocate_id` in
  `create.rs` and `work id next` (`work-cli/src/id/next.rs`), ignore it
  alike. That is the one change to automatic allocation. The visualiser
  reads IDs through the same grammar but does not allocate. Re-key's
  `default_key_floor` already ignores key case.
- The re-key guard runs in `run_with` through the identity run, before
  `create_item`, so the `--id` checks inside `create_item` follow it.
- `--dry-run` currently returns from `run_with` before `creation_context`,
  so all five checks need adding to that path, ahead of the tracker
  preview. Its collision scan takes no lock and is advisory: a dry-run
  pass does not guarantee the real create passes, since another create
  can take the ID in between, but a dry-run refusal is still a refusal.
  Today's preview line carries no ID, so a valid `--id` leaves it
  unchanged.
- The skill's existing single argument resolves bare numbers through
  `work resolve`, which is why the requested ID needs its own `--id` skill
  argument.

## Drafting Notes

- Placed under 0146 by request, as part of its work-item ID model
  alongside 0230 and 0302.
- Raised while creating 0312–0319, which had to be written by hand to start
  numbering at 0312.
- `--id` requires the exact ID, unlike `work resolve`. Resolve is a fuzzy
  matcher for finding an existing item; `--id` names the file that gets
  written, so it should not guess. The skill may propose an exact form
  because the user confirms it.
- Accepting any key matches `--project`, which already allocates a
  sequence per key.

## References

- `cli/work-cli/src/create.rs`, `cli/work-cli/src/main.rs`,
  `cli/work/src/next_number.rs`, `cli/work-adapters/src/creation.rs`,
  `cli/work/src/id_pattern.rs`, `cli/work-cli/src/id/check.rs`,
  `cli/work-cli/src/id/next.rs`
- `skills/work/create-work-item/SKILL.md`
- Related: 0146, 0228, 0230, 0302, 0343, 0371
