---
type: "work-item"
id: "0230"
title: "Tracker-Owned Work Item ID Generation"
date: "2026-08-30T14:35:09+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "in-progress"
kind: "story"
priority: "low"
parent: "work-item:0146"
blocked_by: ["work-item:0228", "work-item:0291"]
blocks: ["work-item:0302"]
tags: ["sync", "tracker", "id-generation", "drafts", "promotion"]
last_updated: "2026-09-24T22:35:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-760"
---
# 0230: Tracker-Owned Work Item ID Generation

**Kind**: Story
**Status**: In Progress
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a developer whose work items live in Jira or Linear, I want the tracker to own
work-item ID generation when configured, so that a work item's local `id` always
matches its remote identifier. When the tracker is reachable, creation
stub-creates the remote issue and adopts its key as `id`; when it is not, the
item is written as a referenceable `draft-` item that `accelerator work sync`
later promotes to its tracker ID. An `id` changes only through promotion or an
explicit re-key, and both record the retired ID as an alias.

## Context

Today the local `id` is minted locally and can differ from `external_id`. Making
the tracker the source of truth means obtaining the tracker ID before writing the
local file, so a final `id` is set once and never rewritten — which resolves the
immutability tension for the online path.

This repo's corpus shows the gap: 294 items, all synced, all with a local `NNNN`
`id` distinct from their `external_id` (`0230` ↔ `PP-760`). The existing
`work create --push` path already creates the remote issue first and the local
file second, but mints `id` locally and falls back to an unsynced `local-save`
when the tracker is unreachable. Tracker-owned IDs remove local minting from the
online path and replace `local-save` with a draft carrying a provisional ID whose
shape can never equal a tracker key.

## Requirements

**Configuration**

- `work.id_pattern: "{tracker}"` enables tracker-owned IDs.
- `{tracker}` must be the pattern's only token and requires `work.integration` to
  be `jira` or `linear`; any other combination fails configuration validation.

**Online creation**

- When the tracker is reachable, `work create --push` adopts the created issue's
  key as the item's `id`, filename prefix, and H1; `id` equals `external_id`. The
  issue is minted into the tracker's creation-home entity.

**Drafts**

- A creation that yields no tracker issue — a declined push, `local-save`, or
  `loud-terminal` — writes a draft to `meta/work/drafts/`.
- A draft's `id` is `draft-` followed by 6 lowercase Crockford base32 characters.
  A draw whose suffix is all digits, or which collides with an existing `id` or
  alias in the corpus, is rejected and redrawn.
- Drafts are first-class: valid typed-link targets (`work-item:draft-…`),
  resolved by `work resolve`, and listed by `list-work-items` with a draft marker.

**Promotion**

- Promotion logic lives in `accelerator work sync`, which promotes drafts by
  default; `--no-promote` skips them. `accelerator work promote <draft-id>` is a
  thin entry point onto the same logic, scoped to one draft.
- Promotion creates the remote issue, moves the file to
  `meta/work/<tracker-id>-<slug>.md`, sets `id` and the H1, and appends the draft
  ID to `aliases`.
- Promotion replaces every occurrence of the draft ID within `meta/` — typed links
  and prose. Nothing outside `meta/` is rewritten.
- Promotion honours the item's `pending_push` marker, so a draft left by
  `loud-terminal` adopts an already-created remote issue instead of duplicating it.

**Identity**

- An `id` changes only through draft promotion or an explicit re-key.
- `aliases: []` is a general retired-ID list; `work resolve` falls back to it.
- `work resolve` also matches on `external_id`.

**Legacy corpus**

- Items whose IDs were minted under an earlier pattern keep them. Pushing an
  unsynced legacy item sets only `external_id`.
- `work resolve` and `corpus frontmatter validate` accept legacy ID shapes
  alongside `{tracker}`.

**Sync pull**

- Under `{tracker}`, a remote-only issue pulled into the corpus takes its
  `external_id` as `id`; no local number is allocated.

**Batch extraction**

- `extract-work-items` attempts a best-effort remote create per item, reusing the
  per-item `work create --push` outcomes, in topological `parent` order; each
  child is created with its parent's tracker ID.
- Items whose create fails become drafts. Children of a failed parent are still
  created remotely, without a parent; their remote parent is set when the parent
  is promoted.
- The skill ends with a per-item outcome summary.

**`create-work-item`**

- Under `{tracker}`, the push offer's decline option reads "No, save as draft".

**Out of scope**

- Pushing `blocks`, `blocked_by`, and `relates_to` to the tracker.
- Trackers other than Jira and Linear.
- Re-keying existing IDs after an `id_pattern` change (0302).

## Acceptance Criteria

- [ ] Given `id_pattern: "{tracker}"`, `integration: linear`, and a reachable
      tracker, when `work create --push` runs, then the written item's `id`,
      filename prefix, and H1 all equal its `external_id`.
- [ ] Given `{tracker}` combined with another token, or with `work.integration`
      unset, `trello`, or `github-issues`, when configuration is validated, then
      it fails naming the violated rule.
- [ ] Given `{tracker}` and a create whose outcome is `local-save`,
      `loud-terminal`, or a declined push, when the item is written, then it lands
      in `meta/work/drafts/` with an `id` matching `^draft-[0-9a-hjkmnp-tv-z]{6}$`,
      a suffix that is not all digits, and no `external_id`.
- [ ] Given a draft `draft-k7mq3x`, when another item sets
      `parent: "work-item:draft-k7mq3x"`, then `corpus frontmatter validate`
      passes and `work resolve draft-k7mq3x` returns the draft's path.
- [ ] Given a draft referenced by a typed link and in the prose of a `meta/` plan,
      when `work sync` runs without `--no-promote`, then the remote issue is
      created, the file moves to `meta/work/<tracker-id>-<slug>.md` with `id` and
      H1 set to the tracker ID, `aliases` contains the draft ID, and no
      occurrence of the draft ID remains in `meta/`.
- [ ] Given drafts and other pending changes, when `work sync --no-promote` runs,
      then the drafts are unchanged and the other changes sync.
- [ ] Given two drafts, when `work promote` runs on one, then only that draft is
      promoted, with the same result as a sync promotion.
- [ ] Given a draft whose `pending_push` marker records a created issue, when it
      is promoted, then no new remote issue is created and the recorded key
      becomes its `id`.
- [ ] Given a promoted draft, when `work resolve` runs on its former draft ID,
      then it returns the promoted item's path.
- [ ] Given legacy item `0230` with `external_id: "PP-760"`, when
      `work resolve PP-760` runs, then it returns that item's path.
- [ ] Given a synced item, or any item whose `id` is not `draft-*`, when
      `work sync` runs, then its `id` is unchanged.
- [ ] Given `{tracker}` and a corpus of legacy `NNNN` items, when
      `corpus frontmatter validate` runs over them, then it passes.
- [ ] Given `{tracker}` and a remote-only issue, when `work sync` pulls it, then
      the local `id` equals its `external_id` and no local number is allocated.
- [ ] Given `extract-work-items` approves an epic and two children and the tracker
      is reachable, when the batch is written, then the epic is created before
      the children, each child's remote parent is the epic, local links use
      tracker IDs, and no subsequent sync is needed to complete the relationships.
- [ ] Given the epic's create returns `local-save`, when the batch completes, then
      the epic is a draft, both children exist remotely without a parent and carry
      `parent: "work-item:<epic draft id>"` locally, and promoting the epic sets
      both children's remote parent.
- [ ] Given a batch in which some creates fail, when the batch completes, then the
      summary lists each item's outcome, and each `loud-terminal` item is named
      with the warning that a remote issue may already exist.

## Open Questions

- Is `aliases` a field of the canonical frontmatter standard (valid on every
  artifact type) or work-item-specific? The re-key command may want it on other
  ID-bearing artifacts.

## Dependencies

- Blocked by: 0228 (Layered Configuration Key Model, done) — 0230 builds on the
  scope-key / `work.key` separation it establishes.
- Blocked by: 0291 (Parent-Child Relationship Synchronisation) — supplies the
  tracker port's parent field that parent-at-create relies on.
- Blocks: 0302 (Work Item Re-Key Command) — builds on `aliases`, alias
  resolution, and the draft form.

## Assumptions

- Jira and Linear issue identifiers are always `<KEY>-<digits>`; the draft ID's
  collision guarantee rests on this. Jira's key grammar is verified; Linear's is
  not.
- Both trackers accept a parent reference on create (Linear `issueCreate`
  `parentId`, Jira `parent` field), as 0291's port extension assumes.
- Six base32 characters (~10⁹ values) with redraw on collision suffice for any
  realistic corpus.

## Technical Notes

- The online path extends `execute_push` in `cli/work-cli/src/create.rs`; the
  draft path replaces its unsynced write on `LocalSave` / `LoudTerminal`.
- `RemoteTracker::create` in `cli/tracker/src/lib.rs` gains the parent reference
  via 0291.
- Sync pull allocates a local ID in `cli/work-adapters/src/sync/create.rs`; under
  `{tracker}` it adopts `external_id` instead.
- `extract-work-items` allocates through `work next-number --count N`; under
  `{tracker}` that is replaced by per-item creates.
- Under tracker-owned IDs `work.key` is unused: synced items take their key from
  the tracker and drafts never carry its prefix.

## Drafting Notes

- Promotion defaults on in `work sync`: an unpromoted draft is the exception, so
  opting out (`--no-promote`) is safer than opting in.
- Rewriting prose inside `meta/` is safe only because the draft token is random —
  an exact-token replacement, never a numeric one.
- Children of a failed parent are still created, to persist as much of a batch
  with the tracker as possible rather than drafting a whole subtree.
- Scope is large for a story (minting, drafts, promotion, batch creation,
  resolver changes); `/refine-work-item` may want to decompose it.

## References

- Parent: 0146 — Work Item Synchronisation Enhancements
- Blocked by: 0228 — Layered Configuration Key Model
- Blocked by: 0291 — Parent-Child Relationship Synchronisation
- Blocks: 0302 — Work Item Re-Key Command
- Code: `cli/work-cli/src/create.rs`, `cli/tracker/src/lib.rs`,
  `cli/work-adapters/src/sync/create.rs`,
  `cli/work-adapters/src/sync/pending_push.rs`
