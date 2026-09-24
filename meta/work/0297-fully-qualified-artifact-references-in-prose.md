---
type: "work-item"
id: "0297"
title: "Fully Qualified Artifact References in Prose"
date: "2026-09-25T08:50:41+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "draft"
kind: "story"
priority: "low"
relates_to: ["work-item:0230", "work-item:0295", "adr:ADR-0034"]
tags: ["references", "typed-links", "prose", "id-retirement"]
last_updated: "2026-09-25T08:50:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0297: Fully Qualified Artifact References in Prose

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a maintainer of the `meta/` corpus, I want every prose mention of a corpus
artifact to be a fully qualified reference in ADR-0034's `doc-type:id` form —
`work-item:PP-760`, `work-item:draft-k7mq3x`, `plan:0042`, `adr:ADR-0033` — so
that references read the same in prose as in frontmatter, tools can traverse
and rewrite them mechanically whatever the ID's shape, and neither readers nor
tools mistake an ID for an ordinary number.

## Context

ADR-0034 makes `doc-type:id` the canonical reference form for frontmatter
linkage, but prose across every artifact type names artifacts informally: bare
numbers ("Blocked by: 0228", "see 0146"), bare ADR IDs ("ADR-0033"), and titles.
A bare number does not say which artifact type it names, and cannot be told
apart from an ordinary number, so no tool can resolve or rewrite it safely.

ID retirement (work-item:0230) shows the cost. It rewrites a retired work-item
ID's typed links everywhere in `meta/`, but rewrites prose only when the old ID
has a distinctive shape — a `draft-` token or a `<KEY>-<number>` key. Re-keying
a numeric corpus (work-item:0295) therefore leaves prose naming retired IDs:
still resolvable through `aliases`, but inconsistent with the typed links
beside it. A prose mention written in `doc-type:id` form is the same token as a
typed link, so it is resolved and rewritten like one.

## Requirements

- Prose in every `meta/` artifact names another corpus artifact in
  `doc-type:id` form, for every artifact type with a unified `id`.
- Every skill and template that writes artifact mentions into `meta/` prose
  emits the qualified form.
- A migration qualifies existing mentions it can identify unambiguously:
  entries in recognised positions (Dependencies and References lists, e.g.
  `Parent: 0146`, `Blocked by: 0228 (…)`, `0295 — Title`) and IDs whose shape
  names their type (e.g. `ADR-0033` → `adr:ADR-0033`). It reports every other
  candidate — a bare number in paragraph prose — with its file and line,
  without rewriting it.
- ID retirement rewrites qualified prose references for every ID shape,
  including bare numeric IDs.

**Out of scope**

- Rewriting references outside `meta/`.
- Path-form references (ADR-0034's second form) to targets without an `id`.

## Acceptance Criteria

- [ ] Given `create-work-item` writes an item with a parent and a blocker, when
      the file is written, then its Dependencies and References entries name
      both as `work-item:<id>`.
- [ ] Given `create-plan` writes a plan for a work item, when the file is
      written, then its prose names the work item as `work-item:<id>`.
- [ ] Given a Dependencies entry `Blocked by: 0228 (Layered Configuration Key
      Model)` in a work item, when the migration runs, then it reads
      `Blocked by: work-item:0228 (Layered Configuration Key Model)`.
- [ ] Given a bare `ADR-0033` in the paragraph prose of a plan, when the
      migration runs, then it reads `adr:ADR-0033`.
- [ ] Given a bare `0228` in paragraph prose, when the migration runs, then it
      is unchanged and the migration's report names its file and line.
- [ ] Given a qualified prose reference `work-item:0230` and a re-key of `0230`
      to `PP-760`, when ID retirement runs, then the prose reads
      `work-item:PP-760`.

## Open Questions

- Should the visualiser render qualified prose references as links?
- Once the corpus is migrated, should ID retirement stop rewriting unqualified
  distinctive IDs in prose, rewriting qualified references only?
- Which positions beyond Dependencies and References lists can the migration
  treat as recognised for each artifact type?

## Dependencies

- Relates to: ADR-0034 (Typed Linkage Vocabulary) — defines the `doc-type:id`
  form this story extends from frontmatter to prose.
- Relates to: 0230 (Tracker-Owned Work Item ID Generation) — defines ID
  retirement, whose prose rule this convention makes uniform.
- Relates to: 0295 (Work Item Re-Key Command) — the re-key of a numeric corpus
  is where bare prose IDs go stale.

## Assumptions

- Qualified references read acceptably in rendered Markdown without link
  rendering.

## Technical Notes

- ID retirement's typed-link rewrite already matches `work-item:<id>`; the
  prose change needs no new matching rule for work items.
- Migrations run through `accelerator migrate`.

## Drafting Notes

- Raised during review of 0230, when shape-gated prose rewriting left bare
  numeric work-item IDs unrewritable; widened to every artifact type so prose
  follows one reference convention across the corpus.
- The migration rewrites only what it can identify unambiguously, because a
  bare number in paragraph prose cannot be told apart from an ID.

## References

- Related: ADR-0034 — Typed Linkage Vocabulary
- Related: 0230 — Tracker-Owned Work Item ID Generation
- Related: 0295 — Work Item Re-Key Command
