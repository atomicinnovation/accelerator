---
type: "work-item"
id: "0295"
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
# 0295: Work Item Re-Key Command

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a developer who changes `work.id_pattern`, I want a command that translates
existing work-item IDs into the new pattern's shape, so that the corpus stays
uniform instead of carrying legacy IDs indefinitely. Re-keying is the only
operation besides draft promotion permitted to change an `id`, and it records
every retired ID as an alias.

## Context

Switching `id_pattern` — `{number:04d}` → `{key}-{number:04d}`, or → `{tracker}`
(0230) — leaves existing items on their old shape. 0230 deliberately keeps them
(the legacy corpus is valid, resolvable by `external_id` and `aliases`) and defers
translation to an explicit, user-triggered command rather than a migration, so it
can run whenever the pattern changes, in either direction.

## Requirements

- `accelerator work rekey` computes each item's target `id` under the current
  `work.id_pattern`: `{tracker}` → `external_id`; `{key}`/`{number}` patterns →
  the item's existing sequence number re-rendered.
- Dry-run by default, printing one `old → new` line per item; `--apply` writes.
- Per item: rename the file, set `id` and H1, append the old ID to `aliases`.
- Rewrite typed links (`work-item:<old>`) across `meta/`.
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
      `0230`, and every `work-item:0230` in `meta/` reads `work-item:PP-760`.
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

- Should prose mentions of an old numeric ID be rewritten? Unlike `draft-`
  tokens, a bare `0230` is not unique text, so exact-text replacement is unsafe;
  candidates are typed links only (current draft) or a recognised-forms list
  (`#0230`, `0230:`).

## Dependencies

- Blocked by: 0230 — supplies `aliases`, alias resolution, and the draft form.
- Blocks: none.

## Drafting Notes

- A command rather than a migration, per user direction: it must be re-runnable
  on every pattern change, which a one-shot migration is not.
- Prose rewrite is excluded pending the open question, because numeric IDs
  collide with ordinary numbers in prose.

## References

- Related: 0230 — Tracker-Owned Work Item ID Generation
- Parent: 0146 — Work Item Synchronisation Enhancements
