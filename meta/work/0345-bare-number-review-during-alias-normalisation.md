---
type: "work-item"
id: "0345"
title: "Bare-Number Review During Alias Normalisation"
date: "2026-10-15T18:23:22+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0146"
blocked_by: ["work-item:0343", "work-item:0344"]
relates_to: ["work-item:0297"]
tags: ["rekey", "aliases", "references", "interactive"]
last_updated: "2026-10-15T18:23:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0345: Bare-Number Review During Alias Normalisation

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a developer normalising a re-keyed corpus, I want each bare number in prose
that could name a work item put to me with its surrounding context, so that I
can decide which ones are references and have those promoted to typed links,
rather than leaving every bare numeric reference stale after a re-key.

## Context

Alias normalisation (work-item:0343) rewrites distinctive aliases in prose,
but a bare number such as `0042` cannot be told apart from an ordinary number,
so it never rewrites one. After a numeric corpus is re-keyed, prose mentions
of the old numbers stay behind, resolvable only through `aliases`.

work-item:0297 plans a migration that reports such candidates with file and
line without rewriting them. This story is the interactive counterpart: it
asks, through the decision engine extracted in work-item:0344, and rewrites
what the user confirms.

## Requirements

- Under `work rekey --normalise-aliases`, a bare number in prose is a
  candidate only when, as a whole token, it equals some item's current `id`
  or one of its aliases.
- Numbers inside code spans and fenced code blocks are never candidates.
- Each candidate is presented through the work-item:0344 engine with its
  surrounding lines and the item it would name. The choices are: accept
  (promote to `work-item:<current id>`), skip, or edit.
- Decisions are recorded, so a re-run does not ask again about an unchanged,
  already-decided occurrence.
- With no TTY and no decisions file, the run stalls and lists its candidates,
  per the engine's contract.

**Out of scope**

- Numbers that name no item's `id` or alias.
- Qualifying references to other artifact types (work-item:0297).

## Acceptance Criteria

- [ ] Given item `ACC-0042` with `aliases: ["0042"]` and prose
      `see 0042`, when the candidate is accepted, then the prose reads
      `see work-item:ACC-0042`.
- [ ] Given prose holding `2026` and `8080` and no item whose `id` or alias
      is either, when the pass runs, then neither is offered.
- [ ] Given `0042` inside a code span or fenced block, when the pass runs,
      then it is not offered.
- [ ] Given a skipped candidate, when the pass runs again on an unchanged
      file, then it is not offered again and the text is unchanged.
- [ ] Given no TTY and no decisions file, when the pass runs, then it stalls
      listing each candidate with its context, and resuming with a decisions
      file applies the decisions.

## Open Questions

- If work-item:0297's migration ships first, should its report feed these
  candidates instead of a fresh scan?

## Dependencies

- Blocked by: work-item:0343 — the normalisation pass this extends.
- Blocked by: work-item:0344 — the decision engine that presents candidates.
- Related: work-item:0297 — reports the same candidates without deciding
  them.

## Assumptions

- Restricting candidates to numbers equal to some item's `id` or alias keeps
  the prompt count proportionate; years, ports and counts rarely match a
  four-digit work-item number.

## Technical Notes

- `work::identity::resolve_identity` answers whether a token names an item's
  `id` or alias.
- A candidate's decision key should identify the occurrence by file and
  surrounding text, not by line number alone, so edits elsewhere in the file
  do not invalidate recorded decisions.

## Drafting Notes

- Priority is low because unrewritten bare numbers still resolve through
  `aliases`; the cost of leaving them is inconsistency, not breakage.
- Parented under work-item:0146 alongside work-item:0343, since it extends
  the same re-key pass.

## References

- Source: work-item:0302 — Work Item Re-Key Command (stress test)
- Related: work-item:0297 — Fully Qualified Artifact References in Prose
