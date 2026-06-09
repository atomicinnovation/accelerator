---
id: "0079"
title: "Detail-Page Aside Region Redesign"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
producer: extract-work-items
status: done
kind: story
priority: medium
tags: [design, frontend, detail-page, aside]
last_updated: "2026-06-05T21:38:10+00:00"
last_updated_by: Toby Clemson
schema_version: 1
type: work-item
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
relates_to: ["work-item:0043"]
---

# 0079: Detail-Page Aside Region Redesign

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Settle and implement a single canonical aside structure for the detail
page: adopt the prototype's flatter section model (Option B) — a
three-section aside of `Related artifacts`, `File`, and `Cluster` (no
separate `Declared links` group) — add the dedicated Cluster section
linking to the lifecycle pipeline, and unify the aside section-label
typography with the page eyebrow and the lifecycle stage-rail labels on
one eyebrow rule.

## Context

The current `RelatedArtifacts` produces three groups (declared
`Targets`, declared `Referenced by`, inferred `Same lifecycle`) with a
legend and a `2px solid accent` / `2px dashed faint` visual
differentiation. The prototype produces a flatter model: a single
`Related artifacts` group whose rows carry a `(declared)` accent tag or
a faint `(inferred)` tag, plus a `File` section and a `Cluster` section
(when a matching cluster exists), with no inline legend. This work item
adopts the prototype's flatter model (Option B).

A *declared* relation is one named by the document's own `target`
frontmatter key (`fm.target` in the prototype source); an *inferred*
relation is a same-lifecycle artifact discovered by cluster membership
rather than declared on the document. Under Option B both remain visible
as rows in the single `Related artifacts` group — declared rows carry an
accent `(declared)` tag, inferred same-lifecycle rows carry a faint
`(inferred)` tag. The `Cluster` block is an *additional* cluster-level
affordance, not a replacement for the inferred rows.

Note on the cited source: the design-gap document's prose describes the
prototype aside as four sections including a *separate* `Declared links`
block. That prose is looser than the prototype's actual structure — the
prototype markup uses a single `.ac-related` list with a per-row
`.ac-related__tag.is-declared` (accent) modifier and no distinct
`Declared links` element. Option B follows the prototype's real
structure, folding declared relations into the single group as an accent
tag; it deliberately supersedes the gap doc's "four sections" phrasing.

The user value of the `Cluster` block is a one-click path from any
document into its lifecycle pipeline view: when a matching lifecycle
cluster exists, the block navigates to `/lifecycle/<slug>` with the
cluster title and `<n> artifacts · <updated>` metadata. The current app
surfaces same-lifecycle relations only as per-artifact `/library/...`
detail links and offers no dedicated "go to cluster" affordance.

The eyebrow typography is closer to settled than the original draft
implied — and one earlier claim was stale. As-shipped:

- **Page eyebrow** (`Page.module.css .eyebrow`) already consumes
  Fira Code mono, `--size-eyebrow` (11px), `0.12em` tracking,
  uppercase, `--ac-fg-faint`. This already matches the prototype.
- **Aside section labels** (`RelatedArtifacts.module.css`) are the real
  outlier: sans default (Inter), `--size-xxs` (12px), weight 600,
  uppercase, `--ac-fg-muted`.
- **Lifecycle stage-rail labels** (`Pipeline.module.css .label`) are
  already Fira Code mono at `--size-4xs`, `0.04em`, **not** uppercase —
  they were previously described as "Inter 12 / weight 400", which is
  incorrect.

So the canonical eyebrow rule already exists as `Page.module.css
.eyebrow`; the task is to make the aside section labels and the
stage-rail labels consume it, promoting the rail labels to the uppercase
eyebrow treatment.

A document "belongs to a lifecycle cluster" (the condition that renders
the `Cluster` block) when the cluster-lookup helper
(`cluster-via-label.ts`) resolves a non-empty cluster for the document's
lifecycle label; otherwise no `Cluster` block renders.

## Requirements

- Adopt the prototype's flatter aside structure (Option B) and record
  the decision: a single `Related artifacts` group whose rows carry a
  declared (accent) or inferred (faint) text tag, plus a `File` section
  (always) and a `Cluster` section (when a matching cluster exists). The
  three sections render in fixed DOM order: `Related artifacts`, then
  `File`, then `Cluster`. Within `Related artifacts`, declared relations
  (those named by the document's `target` frontmatter key) carry an
  accent `(declared)` tag and inferred same-lifecycle relations carry a
  faint `(inferred)` tag, in the same list. Remove the legend and the
  `2px solid` / `2px dashed` border differentiation; the bidirectional
  `Targets` / `Referenced by` split collapses into the single group.
- Add a dedicated `Cluster` block when a matching lifecycle cluster
  exists; the block navigates to `/lifecycle/<slug>` with the cluster
  title and `<n> artifacts · <updated>` metadata.
- Unify eyebrow typography on the existing page-eyebrow rule
  (`Page.module.css .eyebrow` — Fira Code mono, `--size-eyebrow` 11px,
  `0.12em`, uppercase, `--ac-fg-faint`). Apply it to the aside section
  labels and to the lifecycle stage-rail labels (the rail labels are
  promoted from their current mono/lowercase/`--size-4xs` treatment to
  the uppercase eyebrow rule). The properties that must resolve
  identically across all three call sites are: `font-family` (Fira Code
  mono), `font-size` (11px / `--size-eyebrow`), `letter-spacing`
  (`0.12em`), `text-transform` (uppercase), and `color` (`--ac-fg-faint`).
  Either implementation is acceptable — three call sites referencing the
  single `Page.module.css .eyebrow` rule, or three call sites consuming a
  shared token — provided those five resolved values match.

## Acceptance Criteria

- [ ] The detail-page aside renders the Option B structure — `Related
  artifacts`, then `File`, then `Cluster` (when applicable) — in that
  fixed DOM order (`Related artifacts` precedes `File` precedes
  `Cluster`), for every physical doc type in the canonical registry
  (`DOC_TYPE_KEYS` in `src/api/types.ts`, minus the virtual
  `VIRTUAL_DOC_TYPE_KEYS`). As of this writing the detail-page types are:
  `decisions`, `work-items`, `plans`, `research`, `plan-reviews`,
  `pr-reviews`, `work-item-reviews`, `validations`, `notes`,
  `pr-descriptions`, `design-gaps`, `design-inventories` (`templates` is
  virtual and has no detail page). Verifying one representative document
  per type is sufficient; the criterion is not met if any type still
  renders the legacy three-group structure.
- [ ] Given a document with a declared `target` (its `target` frontmatter
  key names another artifact), when its detail page renders, then the
  target appears in the `Related artifacts` group with a declared
  (accent) `(declared)` tag — not in a separate `Targets` or `Declared
  links` group.
- [ ] Given a document with an inferred same-lifecycle relation (no
  declared `target` to that artifact), when its detail page renders, then
  the relation appears as a row in the same `Related artifacts` group
  with a faint `(inferred)` tag — not in a separate `Same lifecycle`
  group.
- [ ] On the detail-page aside (`RelatedArtifacts`), no legend element
  renders and no related-artifact row carries a `2px solid` or `2px
  dashed` border.
- [ ] Given a document that belongs to a lifecycle cluster, when its
  detail page renders, then a `Cluster` block appears (after the `File`
  section) with the cluster title and `<n> artifacts · <updated>`
  metadata and navigates to `/lifecycle/<slug>` on click.
- [ ] Given a document with no matching cluster, when its detail page
  renders, then no `Cluster` block appears.
- [ ] Three specific elements — the page eyebrow (`Page.module.css
  .eyebrow`), the aside section labels (`RelatedArtifacts`), and the
  lifecycle stage-rail labels (`Pipeline` / `PipelineMini` `.label`) —
  resolve to identical computed values for all five eyebrow properties:
  `font-family` (Fira Code mono), `font-size` (11px), `letter-spacing`
  (`0.12em`), `text-transform` (uppercase), and `color` (`--ac-fg-faint`),
  whether implemented as the single `Page.module.css .eyebrow` rule or a
  shared token. The check is scoped to those three elements only; other
  labels on the lifecycle views are out of scope.

## Open Questions

- None outstanding. The two prior decisions are resolved: Option B for
  the aside structure, and the eyebrow rule is the existing page-eyebrow
  rule applied uniformly to the aside section labels and the stage-rail
  labels.

## Dependencies

- Prerequisite (satisfied): 0040 (Pipeline Visualisation Overhaul) is
  functionally complete — its work has landed; only its work-item status
  has not yet been transitioned out of in-progress. It owns the lifecycle
  cluster routes the `Cluster` block links to (`/lifecycle/<slug>`,
  `LifecycleClusterView`, `cluster-via-label.ts`) and the `Pipeline` /
  `PipelineMini` components whose rail labels this work item restyles.
  Because 0040 is no longer in flight, the rail-label change is not a live
  concurrent-edit hazard. Confirm 0040's status is transitioned before
  this work item is closed; treat any reactivation of 0040 as a renewed
  concurrent-edit coupling on the `Pipeline` rail labels that must be
  re-coordinated.
- Prerequisite (data): the cluster-lookup helper (`cluster-via-label.ts`)
  must expose the cluster title, `<n>` artifact count, and `<updated>`
  timestamp the `Cluster` block renders. These are believed available from
  0040's landed work; confirm the helper returns all three fields in the
  shape the block needs before implementing the Cluster metadata, since
  the Cluster acceptance criterion cannot be met otherwise.
- Related: 0043 (detail-screen capability-retention spike — **abandoned**;
  the declared/inferred scope question it would have settled is now
  settled by this work item's Option B decision). 0074 / 0075 (adjacent
  eyebrow icon/size work) — these touch the eyebrow *icon and size*, while
  this work item unifies the eyebrow *font-family / tracking / transform /
  colour*. The concerns are orthogonal, but all three touch the
  `Page.module.css .eyebrow` surface; if 0074 / 0075 are still in flight
  when this work lands, sequence so this work item's rule consolidation
  lands first and 0074 / 0075 layer icon/size on top. Mirror this
  ordering note onto 0074 / 0075 (or record it as a shared coordination
  point) so the sequencing is visible from both sides rather than relying
  on this work item alone.
- Blocks: no downstream work item is currently gated on this change. This
  work item establishes `Page.module.css .eyebrow` as the canonical
  eyebrow rule; future label-styling work should consume it rather than
  reintroduce a divergent rule.

## Assumptions

- Both declared and inferred relations remain in scope on the detail
  page. This is settled by the Option B decision itself: declared
  relations show as accent `(declared)`-tagged rows and inferred
  same-lifecycle relations as faint `(inferred)`-tagged rows, both inside
  the single `Related artifacts` group, with the `Cluster` block as an
  additional cluster-level affordance (not a replacement for the inferred
  rows). This is not settled by 0043, which is abandoned.

## Technical Notes

- Frontend root: `skills/visualisation/visualise/frontend/`.
- Aside component: `src/components/RelatedArtifacts/RelatedArtifacts.tsx`
  (groups `Targets` / `Referenced by` / `Same lifecycle`) and
  `RelatedArtifacts.module.css` (legend; uppercase labels at
  `--size-xxs` weight 600 muted). Option B collapses the groups and
  removes the legend and solid/dashed borders.
- Detail route: `src/routes/library/LibraryDocView.tsx` (renders
  `RelatedArtifacts` and `EyebrowLabel`); registered at
  `/library/$type/$fileSlug` in `src/router.ts`.
- Canonical eyebrow rule: `src/components/Page/Page.module.css .eyebrow`
  (Fira Code mono, `--size-eyebrow` 11px, `0.12em`, uppercase, faint).
- Rail labels: `src/components/Pipeline/Pipeline.module.css .label`
  (mono, `--size-4xs`, `0.04em`, not uppercase) and the `PipelineMini`
  equivalent — owned by 0040 (functionally complete; no concurrent edits
  expected).
- Cluster link target: `/lifecycle/$slug` →
  `src/routes/lifecycle/LifecycleClusterView.tsx`; cluster-key helper at
  `src/routes/lifecycle/cluster-via-label.ts`. The `Same lifecycle` rows
  currently link to `/library/...` detail pages, so the `Cluster` block
  is net-new wiring.
- Verification fixtures: planning should pin one concrete example document
  per acceptance case — a document with a declared `target`, a document
  with only an inferred same-lifecycle relation, a document that resolves
  to a cluster, and one that does not — so each Given precondition maps to
  a reproducible input rather than a corpus hunt. The cluster match rule
  is the `cluster-via-label.ts` non-empty-resolve condition stated in
  Context.

## Drafting Notes

- Resolved Option B per the user's direction; this accepts dropping the
  bidirectional `Targets` vs `Referenced by` distinction, retaining
  declared/inferred only as accent/faint text tags.
- Interpreted "unify on one eyebrow rule" as adopting the already-shipped
  page-eyebrow rule (`Page.module.css .eyebrow`) rather than inventing a
  new one — the page eyebrow already matches the prototype; only the
  aside labels (and now the rail labels) diverged.
- Per the user's direction the lifecycle stage-rail label is folded into
  the eyebrow rule (promoted to uppercase 11px/`0.12em`), even though it
  is a stage name rather than a section eyebrow. This changes the rail's
  visual weight and touches the `Pipeline` component owned by in-progress
  0040 — flagged as a coordination point.
- Corrected the stale Context claim that the rail label was "Inter 12 /
  400" — it is already Fira Code mono.
- Review pass 1 revisions: clarified that inferred same-lifecycle
  relations remain as faint `(inferred)`-tagged rows in the single group
  (the `Cluster` block is additive, not a replacement); reconciled the
  gap doc's "separate `Declared links` block" prose against the
  prototype's actual single-list-with-accent-tag structure (verified via
  `.ac-related__tag.is-declared` in `prototype-standalone.html`); bounded
  the "every detail-page route" criterion to the canonical doc-type
  registry; added an explicit inferred-tag criterion and a fixed section
  order; enumerated the five eyebrow properties the equality check
  compares; anchored the legend/border-removal criterion to the
  `RelatedArtifacts` component; and downgraded the 0040 blocker to a
  satisfied prerequisite (0040 is functionally complete, status pending).
- Review pass 2 polish: added the canonical three-section count to the
  Summary; merged the duplicated `Cluster`-block paragraphs in Context and
  added the cluster match rule (`cluster-via-label.ts` non-empty resolve);
  corrected the first criterion's registry reference from the non-existent
  `LIBRARY_INDEX` to `DOC_TYPE_KEYS` in `src/api/types.ts` and enumerated
  the twelve physical detail-page doc types inline (excluding virtual
  `templates`); added an explicit `Related artifacts` → `File` → `Cluster`
  ordering assertion; scoped the eyebrow-equality check to the three named
  elements (not "the lifecycle views" broadly); added a verification-
  fixtures note to Technical Notes; promoted the cluster-helper data
  availability to its own prerequisite line; and added mirror/reactivation
  coordination notes for 0074 / 0075 and 0040.
- Frontmatter migrated from the legacy shape (`work_item_id`, `type:
  story`) to the unified shape (`type: work-item`, `id`, `kind`, plus
  `producer`, `external_id`, `last_updated`, `last_updated_by`,
  `schema_version`). `producer` set to `extract-work-items` to reflect
  the artifact's true origin.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Prototype: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
- Related: 0040, 0043, 0074, 0075
