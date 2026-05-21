---
work_item_id: "0079"
title: "Detail-Page Aside Region Redesign"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, detail-page, aside]
---

# 0079: Detail-Page Aside Region Redesign

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Settle a single canonical aside structure for the detail page covering
section vocabulary, the declared/referenced/inferred relation split
question, addition of a dedicated Cluster section linking to the
lifecycle pipeline, and unification of the aside eyebrow typography rule
with the page eyebrow and lifecycle stage rail.

## Context

The current `RelatedArtifacts` produces three groups (declared
`Targets`, declared `Referenced by`, inferred `Same lifecycle`) with a
legend and a `2px solid indigo` / `2px dashed faint` visual
differentiation. The prototype produces four sections in fixed order
(`Related artifacts` always, `Declared links` when `fm.target` exists,
`File` always, `Cluster` when a matching cluster exists) with a flatter
`(declared)` / `(inferred)` text tag and no inline legend.

The aside eyebrow typography also drifts: the current app uses
Inter 12 / weight 600 / uppercase for aside H3s, while the prototype
uses Fira Code 10.5 / uppercase. The lifecycle stage rail uses Inter 12
weight 400 — a third style. A single eyebrow rule should apply
consistently across the aside, lifecycle stage rail, and page eyebrow.

The Cluster block (when a matching cluster exists) navigates to
`#/lifecycle/<slug>` with cluster title and `<n> artifacts · <updated>`
metadata. The current app surfaces same-lifecycle relations as links
inside `Same lifecycle` but offers no dedicated "go to cluster"
affordance.

## Requirements

- Decide on one canonical aside structure:
  - Option A — current trichotomy migrated into the prototype's section
    ordering (declared `Targets` + `Referenced by` + inferred
    `Same lifecycle` + `Cluster` + `File`).
  - Option B — prototype's flatter model adopted (single `Related
    artifacts` group + `Declared links` block + `File` + `Cluster`).
- Add a dedicated `Cluster` block when a matching lifecycle cluster
  exists; the block navigates to the lifecycle pipeline view with
  cluster title and `<n> artifacts · <updated>` metadata.
- Unify aside eyebrow typography with the page eyebrow and lifecycle
  stage rail — pick one (Inter 12/600/uppercase, Fira Code
  10.5/uppercase, or Inter 12/400/uppercase) and apply uniformly.

## Acceptance Criteria

- [ ] A decision is recorded on aside structure (Option A or B) with
  rationale.
- [ ] The chosen aside structure is implemented on every detail-page
  route.
- [ ] A `Cluster` block appears when a matching lifecycle cluster
  exists, with click-through to the cluster's lifecycle view.
- [ ] Aside eyebrows, the page eyebrow, and lifecycle stage rail labels
  all consume the same typography rule.

## Open Questions

- Does the team value the bidirectional declared/referenced split
  (current Option A) or accept the prototype's flatter model
  (Option B)? Equivalent to 0043's pending Q3-style decision on
  detail-page aside behaviour, but specific to this section structure.
- Which eyebrow rule wins?

## Dependencies

- Blocked by: 0040 (lifecycle cluster routes exist for the `Cluster`
  block to link to).
- Related: 0043 (detail-screen capability-retention spike — the
  declared/inferred split decision is closely related).

## Assumptions

- Both declared and inferred relations remain in scope on the detail
  page (resolved by 0043).

## Technical Notes

- `RelatedArtifacts` currently lives in `src/components/RelatedArtifacts/`.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0040, 0043
