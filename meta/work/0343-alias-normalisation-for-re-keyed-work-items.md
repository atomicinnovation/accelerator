---
type: "work-item"
id: "0343"
title: "Alias Normalisation for Re-Keyed Work Items"
date: "2026-10-15T18:22:44+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0146"
blocked_by: ["work-item:0302"]
relates_to: ["work-item:0297"]
tags: ["rekey", "aliases", "references", "typed-links"]
last_updated: "2026-10-15T18:22:44+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0343: Alias Normalisation for Re-Keyed Work Items

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a developer whose corpus has been re-keyed, I want references that still
name a retired work-item ID rewritten to the item's current `id`, so that
`meta/` stops carrying stale IDs that resolve only through `aliases`. Prose
mentions of distinctive aliases become typed links, lifting the corpus's
references to the `doc-type:id` form as they are corrected.

## Context

ID retirement (work-item:0230) rewrites an old ID across `meta/` at the moment
it retires, but references written later — from unmerged workspaces, branches
merged after a promotion, or hand edits — still name the alias. They resolve
through `aliases`, but disagree with the typed links beside them.

This story was split out of work-item:0302 during its stress test, as the
`--normalise-aliases` mode of `work rekey`.

## Requirements

- `work rekey --normalise-aliases` adds a pass, after the re-key in the same
  run, over every alias in the corpus, whatever retired it: draft promotion,
  a tracker key change, or an earlier re-key.
- A typed link naming an alias is rewritten to name the item's current `id`.
- A prose mention of a distinctive alias — a `draft-` token or a
  `<KEY>-<number>` key, matched as a whole token — is promoted to a typed link
  naming the current `id`.
- Code spans and fenced code blocks are never rewritten.
- An alias that resolves `Conflicting` (held by more than one item) is skipped
  and reported, naming every item that holds it; the rest of the run applies.
- The pass joins the run plan, honours `--preview`, and is subject to the
  dirty-tree refusal, as the re-key itself is.

**Out of scope**

- Bare numeric aliases in prose (work-item:0345).
- References outside `meta/` or in remote issue bodies.

## Acceptance Criteria

- [ ] Given promoted item `PP-761` with `aliases: ["draft-k3x9"]` and prose
      `see draft-k3x9`, when `work rekey --normalise-aliases` runs, then it
      reads `see work-item:PP-761`.
- [ ] Given `work-item:draft-k3x9` in `meta/`, when the pass runs, then it
      reads `work-item:PP-761`.
- [ ] Given `draft-k3x9` inside a code span or a fenced block, when the pass
      runs, then it is unchanged.
- [ ] Given item `ACC-0042` with `aliases: ["0042"]`, when the pass runs, then
      a prose `0042` is unchanged and `work-item:0042` reads
      `work-item:ACC-0042`.
- [ ] Given alias `PP-700` held by two items, when the pass runs, then every
      mention of `PP-700` is unchanged, the output names both items, and the
      other rewrites apply.
- [ ] Given `--preview`, when the pass runs, then it lists every rewrite with
      its file and changes nothing under `meta/`.
- [ ] Given a normalised corpus, when the pass runs again, then nothing
      changes.

## Open Questions

None.

## Dependencies

- Blocked by: work-item:0302 — supplies `work rekey`, its run plan, preview
  binding and dirty-tree refusal.
- Blocks: work-item:0345 — Bare-Number Review During Alias Normalisation.
- Related: work-item:0297 — owns the qualified prose reference form this pass
  emits.

## Assumptions

- Promoting prose to typed links ahead of work-item:0297 is accepted: 0297
  will update every writer, and this pass lifts the corpus's specificity in
  the same direction.

## Technical Notes

- `corpus::references::rewrite_references` matches distinctive IDs as whole
  tokens ignoring ASCII case, and never rewrites inside a frontmatter
  `aliases:` entry; the promotion to a typed link and the code-span exclusion
  are new.
- `work::identity::resolve_identity` already distinguishes `Unique` from
  `Conflicting` across `id`, `aliases` and `external_id`.

## Drafting Notes

- "Every alias" was chosen over "aliases this run created" because the
  motivating case is references that arrive after the retirement.
- Code is excluded because a token in backticks is often a literal — a path,
  a command, an example — that a typed link would break.

## References

- Source: work-item:0302 — Work Item Re-Key Command (stress test)
- Related: work-item:0230 — Tracker-Owned Work Item ID Generation
- Related: work-item:0297 — Fully Qualified Artifact References in Prose
