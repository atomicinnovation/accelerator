---
type: "work-item"
id: "0302"
title: "Work Item Re-Key Command"
date: "2026-09-24T22:35:39+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0146"
blocked_by: ["work-item:0230"]
external_id: "PP-879"
tags: ["sync", "tracker", "id-generation", "rekey"]
last_updated: "2026-09-24T22:35:39+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0302: Work Item Re-Key Command

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a developer who changes `work.id_pattern`, I want a command that translates
existing work-item IDs into the new pattern's shape, so that the corpus stays
uniform instead of carrying legacy IDs indefinitely. An `id` changes only
through draft promotion, a tracker-side key change (both 0230), or re-keying;
all three retire the old ID through 0230's ID retirement, which records it as
an alias.

## Context

Switching `id_pattern` — `{number:04d}` → `{key}-{number:04d}`, or → `{tracker}`
(0230) — leaves existing items on their old shape. 0230 deliberately keeps them
(the legacy corpus is valid, resolvable by `external_id` and `aliases`) and defers
translation to an explicit, user-triggered command rather than a migration, so it
can run whenever the pattern changes, in either direction.

## Requirements

- Before computing targets, `work rekey` refreshes each synced item's
  `external_id` from the tracker using 0230's key-change detection, so a target
  derived from `external_id` is the issue's current key. `--apply` writes the
  refreshed `external_id`; the dry run only reports it. Under `{tracker}`, a
  tracker it cannot reach makes `--apply` refuse and write nothing.
- `accelerator work rekey` computes each item's target `id` under the current
  `work.id_pattern`: `{tracker}` → `external_id`; `{key}`/`{number}` patterns →
  the item's existing sequence number re-rendered.
- Dry-run by default, printing one `old → new` line per item; `--apply` writes.
- Per item, retire the old ID through 0230's ID retirement: rename the file,
  set `id` and H1, append the old ID to `aliases`, and rewrite it within
  `meta/` — typed links always, and prose, whole tokens only, when the old ID
  is a `draft-` token or a `<KEY>-<number>` key. A bare numeric ID has only its
  typed links rewritten.
- Refuse to apply on a dirty working tree, and refuse the whole run if any
  target `id` collides with an existing `id` or alias.
- Under `{tracker}`, an unsynced item has no target; it becomes a `draft-` item
  per 0230.
- Re-running with no pattern change is a no-op.
- A `--normalise-aliases` pass rewrites references still using an aliased ID
  (e.g. promoted drafts referenced from unmerged workspaces).
- A `/rekey-work-items` skill wraps the command with preview and confirmation.

**Out of scope**

- Rewriting references outside `meta/` or in remote issue bodies.

## Acceptance Criteria

- [ ] Given `{tracker}` and legacy item `0230` with `external_id: "PP-760"`,
      when `work rekey --apply` runs, then the file is
      `meta/work/PP-760-<slug>.md`, `id` and H1 are `PP-760`, `aliases` contains
      `0230`, every `work-item:0230` in `meta/` reads `work-item:PP-760`, and
      a prose mention of `0230` is unchanged.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"`, and item `PP-760`
      named in `meta/` prose, when `work rekey --apply` runs, then the prose
      names its new `ACC-` ID.
- [ ] Given `{tracker}` and legacy item `0230` with `external_id: "PP-760"`
      whose issue has moved to `ENG-42` since the last sync, when
      `work rekey --apply` runs, then `id` and `external_id` are `ENG-42` and
      `aliases` contains `0230`.
- [ ] Given `{tracker}` and an unreachable tracker, when `work rekey --apply`
      runs, then it exits non-zero and writes nothing.
- [ ] Given `{key}-{number:04d}` with `work.key: "ACC"` and item `0042`, when
      `work rekey --apply` runs, then its `id` is `ACC-0042`.
- [ ] Given no `--apply`, when `work rekey` runs, then it prints the mapping
      and writes nothing.
- [ ] Given a target `id` that collides with an existing `id` or alias, when
      `work rekey --apply` runs, then it exits non-zero naming both items and
      writes nothing.
- [ ] Given a dirty working tree, when `work rekey --apply` runs, then it
      refuses.
- [ ] Given `{tracker}` and an unsynced legacy item, when `work rekey --apply`
      runs, then it becomes a `draft-` item in `meta/work/drafts/`.
- [ ] Given a re-keyed corpus, when `work rekey --apply` runs again, then
      nothing changes.
- [ ] Given a re-keyed item, when `work resolve` runs on its old ID, then it
      returns the item's new path.

## Open Questions

None.

## Dependencies

- Blocked by: 0230 — supplies `aliases`, alias resolution, the draft form, ID
  retirement, and tracker-side key-change detection.
- Blocks: none.

## Drafting Notes

- A command rather than a migration, per user direction: it must be re-runnable
  on every pattern change, which a one-shot migration is not.
- Prose is rewritten only for distinctive ID shapes (`draft-` tokens and
  `<KEY>-<number>` keys), because bare numeric IDs collide with ordinary
  numbers in prose.

## References

- Related: 0230 — Tracker-Owned Work Item ID Generation
- Parent: 0146 — Work Item Synchronisation Enhancements
