---
date: "2026-06-01T22:18:41+01:00"
author: "Toby Clemson"
revision: "53e5936ccef94b353d2aec867e95314f77cb564a"
repository: accelerator
topic: "Lifecycle clustering breaks because work-item slugs and plan/research/review slugs use different shapes; validations/PR-descriptions/PR-reviews never join a cluster"
tags: [research, codebase, visualiser, clusters, slug, work-item-id, typed-linkage]
status: complete
last_updated: "2026-06-01T00:00:00+00:00"
last_updated_by: "Toby Clemson"
type: codebase-research
id: "2026-06-01-lifecycle-clustering-slug-mismatch"
title: "Research: lifecycle clustering breaks because slugs don't agree across doc types"
schema_version: 1
relates_to: ["adr:ADR-0033", "adr:ADR-0034", "adr:ADR-0025", "adr:ADR-0028", "codebase-research:2026-05-24-0068-related-documents-inference-accuracy", "work-item:0057", "work-item:0040"]
supersedes: ["adr:ADR-0034", "adr:ADR-0033"]
derived_from: ["codebase-research:2026-05-24-0068-related-documents-inference-accuracy", "codebase-research:2026-05-21-0064-canonicalise-work-item-id-and-author-fields", "codebase-research:2026-05-30-0065-update-artifact-templates-to-unified-schema", "adr:ADR-0033", "codebase-research:2026-04-28-configurable-work-item-id-pattern", "codebase-research:2026-05-31-0040-pipeline-visualisation-overhaul"]
---

# Research: lifecycle clustering breaks because slugs don't agree across doc types

**Date**: 2026-06-01 22:18 BST
**Author**: Toby Clemson
**Git Commit**: 53e5936ccef94b353d2aec867e95314f77cb564a
**Branch**: main
**Repository**: accelerator

## Research Question

Lifecycle clusters in the visualiser group `IndexEntry`s by `slug`. For
work item `0040` the work-item file `meta/work/0040-pipeline-visualisation-overhaul.md`
yields slug `pipeline-visualisation-overhaul`, but its plan
`meta/plans/2026-05-31-0040-pipeline-visualisation-overhaul.md` yields
slug `0040-pipeline-visualisation-overhaul`. The two slugs don't agree
and the entries don't cluster. Similarly, validations, PR descriptions
and PR reviews never seem to land in a cluster. **Analyse the corpus
under `meta/` to understand how clustering breaks today and lay out
options to fix it.**

## Summary

The slug derivation in `server/src/slug.rs` strips a `YYYY-MM-DD-`
date prefix for plans/research/plan-reviews/validations/notes/etc but
**does not** strip a subsequent `NNNN-` work-item-ID prefix. Work-item
filenames strip `NNNN-`. The two derivations are incompatible whenever
the artifact's filename embeds the work-item ID.

The corpus is mid-migration. Newer plans / research / plan-reviews
adopt the `YYYY-MM-DD-NNNN-slug.md` convention (about half of plans,
half of research, half of plan-reviews). Older artifacts use
`YYYY-MM-DD-slug.md`. The two coexist in the same directories. The
slug rule favours neither — it produces a "name-only" slug for old
files and a "NNNN-name" slug for new ones, and clustering succeeds
only when one of these accidentally equals the work item's `slug`
(the descriptive tail of the work-item filename).

Three independent failure modes are visible:

1. **ID-prefix bleed-through** — new-convention plans/research/plan-
   reviews carry the work-item ID inside their slug, so the work-item
   and its plan land in different buckets. Worst affected: ~47 plans,
   36 research files, 47 plan-reviews.
2. **Convention drift on other types** — validations almost always use
   the `YYYY-MM-DD-<descriptive>-validation.md` form with no
   work-item ID and a non-strippable `-validation` suffix. PR
   descriptions and PR reviews don't exist on disk yet, but their slug
   rules inherit the same shape. There is no path by which their slug
   matches a work-item slug; they would always orphan.
3. **Work-item reviews are universally orphans** — `meta/reviews/work/`
   filenames have **no date prefix** (`0030-centralise-path-defaults-review-1.md`).
   The slug rule for `WorkItemReviews` requires a date and returns
   `None`. Every file in this directory is dropped from clustering
   today.

Two ADRs already govern the long-term answer: **ADR-0033** (unified
base frontmatter schema) and **ADR-0034** (typed linkage vocabulary).
Both are *accepted* but the corpus migration tracked under epic
`0057` is *in-progress*. ADR-0034 deliberately routes every artifact
back to a work item through a frontmatter chain (`parent` / `target`
/ `derived_from`), not through filename slugs.

The three viable options below trade migration cost against the
window over which clustering remains visibly broken.

## Detailed Findings

### How clustering actually groups entries

`compute_clusters` and `compute_clusters_with_backfill` bucket entries
solely by `IndexEntry.slug`
([clusters.rs:53-85](skills/visualisation/visualise/server/src/clusters.rs#L53-L85)):

```
for e in entries {
  if matches!(e.r#type, DocTypeKey::Templates) { continue; }
  let Some(slug) = e.slug.clone() else { continue }; // orphan drop
  buckets.entry(slug).or_default().push(e.clone());
}
```

Entries with `slug = None` are silently dropped (test at
`clusters.rs:458-463`). The slug is a single `Option<String>` set per
entry by `build_entry` in
[indexer.rs:1156-1185](skills/visualisation/visualise/server/src/indexer.rs#L1156-L1185).

### Slug derivation per doc type

Source: [slug.rs:14-86](skills/visualisation/visualise/server/src/slug.rs#L14-L86).
Applied to representative filenames in the corpus today:

| Filename | Doc type | Slug today | Notes |
|---|---|---|---|
| `meta/work/0040-pipeline-visualisation-overhaul.md` | WorkItems | `pipeline-visualisation-overhaul` | strip `NNNN-` (or `PROJ-NNNN-` via configured regex) |
| `meta/plans/2026-05-31-0040-pipeline-visualisation-overhaul.md` | Plans | `0040-pipeline-visualisation-overhaul` | strip date only; **ID retained** |
| `meta/plans/2026-02-22-pr-review-agents.md` | Plans | `pr-review-agents` | strip date only; no ID present |
| `meta/research/codebase/2026-05-05-0031-consolidate-accelerator-owned-files.md` | Research | `0031-consolidate-accelerator-owned-files` | strip date only; **ID retained** |
| `meta/reviews/plans/2026-05-05-0031-consolidate-accelerator-owned-files-review-1.md` | PlanReviews | `0031-consolidate-accelerator-owned-files` | strip date + `-review-N`; **ID retained** |
| `meta/validations/2026-04-21-meta-visualiser-phase-3-…-validation.md` | Validations | `meta-visualiser-phase-3-…-validation` | strip date only; `-validation` suffix **not** stripped |
| `meta/reviews/work/0030-centralise-path-defaults-review-1.md` | WorkItemReviews | **`None`** (orphan) | rule requires date prefix; corpus filenames have no date |
| `meta/decisions/ADR-0007-foo.md` | Decisions | `foo` | strip `ADR-NNNN-` |

`build_entry` in indexer.rs takes a special path for `WorkItems`
([indexer.rs:1156-1184](skills/visualisation/visualise/server/src/indexer.rs#L1156-L1184))
that uses the configured `scan_regex` to strip a project-prefix-ID
(e.g. `ENG-0040-`) before falling back to `strip_prefix_work_item_id`.
No other doc type runs through this stripper, which is why plans /
research / plan-reviews keep the leading `NNNN-` after the date is
removed.

### Corpus shape (counts per directory)

From a fresh inventory:

| Directory | `.md` files | `^NNNN-` | `^YYYY-MM-DD-NNNN-` | `^YYYY-MM-DD-[a-z]` | `^ADR-NNNN-` |
|---|---:|---:|---:|---:|---:|
| `meta/work` | 91 | 91 | 0 | 0 | 0 |
| `meta/research/codebase` | 69 | 0 | 36 | 33 | 0 |
| `meta/plans` | 92 | 0 | 47 | 45 | 0 |
| `meta/reviews/plans` | 76 | 0 | 47 | 29 | 0 |
| `meta/reviews/work` | 37 | 37 | 0 | 0 | 0 |
| `meta/validations` | 7 | 0 | 1 | 6 | 0 |
| `meta/research/design-gaps` | 2 | 0 | 0 | 2 | 0 |
| `meta/notes` | 13 | 0 | 0 | 13 | 0 |
| `meta/decisions` | 38 | 0 | 0 | 0 | 38 |
| `meta/prs` | (not present) | — | — | — | — |
| `meta/reviews/prs` | (not present) | — | — | — | — |
| `meta/research/design-inventories` | (not present) | — | — | — | — |

**Key takeaways**:

- The `YYYY-MM-DD-NNNN-` convention covers roughly half of all
  plans/research/plan-reviews — the broken-clustering cohort is large
  and recent.
- `meta/reviews/work/` files lack a date entirely; the current slug
  rule for `WorkItemReviews` rejects them outright (returns `None`),
  so every work-item review is an orphan today.
- `meta/validations/` keeps a non-strippable `-validation` suffix in
  the slug, and only 1 of 7 files carries a `NNNN-` prefix. Even if
  we strip the ID, the `-validation` suffix prevents a name-match
  against the work-item slug.
- `meta/prs/`, `meta/reviews/prs/`, `meta/research/design-inventories/`
  don't exist on disk yet. The clustering rule still has to do
  something sensible when they appear.

### Frontmatter linkage already present in the corpus

| Directory | Total | `work_item_id:` | `parent:` | `target:` | `related:` |
|---|---:|---:|---:|---:|---:|
| `meta/plans` | 92 | 36 | 4 | 6 | 1 |
| `meta/research/codebase` | 69 | 6 | 1 | 0 | 2 |
| `meta/reviews/plans` | 76 | 0 | 0 | 76 | 0 |
| `meta/validations` | 7 | 0 | 0 | 7 | 0 |
| `meta/notes` | 13 | 0 | 0 | 0 | 0 |

Notable signals:

- **Every plan-review and every validation** already carries a
  `target:` reference (76/76 and 7/7 respectively). Those are the
  exact two doc types that are most often orphaned today.
- About 39% of plans carry an explicit `work_item_id:`, which is the
  remnant of the old frontmatter-link convention before ADR-0034
  re-labelled it `parent`.
- Notes carry **no** linkage keys; under the typed-linkage vocabulary
  they're never tied to a work item, so they orphan by design (and
  rightly so — notes are free-form).

### Live indexer plumbing that already chases these references

`Indexer` already maintains secondary indexes that could power
frontmatter-based clustering ([indexer.rs:217-234](skills/visualisation/visualise/server/src/indexer.rs#L217-L234)):

- `work_item_by_id: HashMap<String, PathBuf>` — canonical ID → work-item path.
- `work_item_refs_by_target: HashMap<String, BTreeSet<PathBuf>>` — work-item ID → set of paths that reference it via `work_item_id` / `parent` / `related` frontmatter keys (read by `frontmatter::read_ref_keys` at
  [indexer.rs:1194-1195](skills/visualisation/visualise/server/src/indexer.rs#L1194-L1195)).
- `plans_by_id: HashMap<String, PathBuf>` — plan filename-stem → plan path; resolves `target: "plan:<id>"` from plan-reviews and validations.
- `reviews_by_target: HashMap<PathBuf, BTreeSet<PathBuf>>` — target path → set of reviewer paths (resolved by walking plan-review `target` frontmatter to a plan path).

The infrastructure to chase `parent` → work-item and `target` → plan →
`parent` → work-item already exists for the `/api/related/*` endpoint
([related.rs](skills/visualisation/visualise/server/src/related.rs)).
Clustering does not currently consume it.

### ADR landscape (governance)

- **ADR-0034 typed linkage vocabulary** (accepted). Defines `parent`,
  `target`, `derived_from`, `relates_to`, `supersedes`/`superseded_by`,
  `blocks`/`blocked_by`, `source`. Per the type-pair table, an
  artifact's link back to its work item travels through the
  relationship-named keys:
  - `plan` → work-item via `parent: "work-item:0042"`
  - `plan-review` / `work-item-review` → work-item via `target`
  - `validation` → plan via `target` (two-hop: validation → plan → work-item)
  - `research` clusters as `derived_from` by a plan; there is no
    direct research→work-item link
  - There is **no corpus-wide `work_item_id` linkage key** in this
    vocabulary; the slug-derived ID is decorative, not navigational.
- **ADR-0033 unified base frontmatter schema** (accepted). The
  mandatory base set (`type`, `id`, `title`, `date`, …) does **not**
  include `work_item_id`. Foreign-reference IDs sit under per-type
  extras and route through ADR-0034.
- **Epic 0057 unified-artifact-frontmatter-and-typed-cross-linking**
  (in-progress). Migration of producer skills and the corpus is the
  open work; visualiser graph rendering is explicitly deferred to a
  sibling future epic.
- **Research 0068 (related-documents inference accuracy)** measured
  body-prose link inference at 11.3% wrong-rate — well above the 5%
  threshold. The conclusion was *frontmatter is the destination* and
  body-section parsing is a stopgap. Slug-based clustering inherits
  the same fragility (it depends on an implicit author-controlled
  naming contract).

## Options

### Option A — Normalise the slug rule

Change `slug.rs` so plans/research/plan-reviews strip an optional
trailing `NNNN-` after the date, and so work-item-reviews accept a
no-date `NNNN-slug-review-N` shape. Keep slug as the cluster key.

- Pros:
  - Smallest diff. One file changed (`slug.rs`), regenerate from the
    on-disk shape.
  - Fixes the visible bug for files that *do* embed a work-item ID in
    their filename (most new plans/research/plan-reviews) and unlocks
    work-item reviews (currently all orphans).
  - No corpus migration required.
- Cons:
  - **Doesn't fix old files** (the ~half of plans/research/plan-
    reviews without an ID in the filename keep their accidental
    descriptive-slug match). Those continue to cluster only when the
    descriptive tail happens to equal the work-item's slug.
  - **Doesn't fix validations / PR-descriptions / PR-reviews** as a
    class — their filenames usually don't carry a work-item ID, and
    validations carry a `-validation` suffix that won't strip cleanly.
  - Reinforces a slug-equality contract that ADR-0034 explicitly
    walks away from.
  - Author-facing: every new filename has to follow the convention
    exactly or the cluster silently breaks.

### Option B — Cluster via the typed-linkage frontmatter chain

Introduce a per-entry `cluster_key: Option<String>` computed at
indexing time. Resolution walks the type-pair table from ADR-0034:

- `WorkItems` → `cluster_key = work_item_id`
- `WorkItemReviews` / `PlanReviews` → resolve `target` to a path; if
  the target is a work-item, use its `work_item_id`; if the target
  is a plan, walk plan → `parent` → work-item
- `Plans` / `Research` / `PrDescriptions` → walk `parent` → work-item
- `Validations` → walk `target` → plan → `parent` → work-item
- `Notes` / `DesignGaps` / `DesignInventories` → no required link;
  orphan if no frontmatter ID found

Clustering then buckets by `cluster_key` instead of `slug`. Slug stays
as a per-file identity helper (URL, breadcrumb), decoupled from
clustering.

- Pros:
  - Single source of truth (frontmatter), aligned with the accepted
    ADRs.
  - Robust to filename convention drift — works for any of the four
    naming shapes already in the corpus.
  - Naturally extends to multi-hop chains (validation → plan → work-
    item) the visualiser already needs for the related-artifacts
    endpoint.
  - Composes with the existing `work_item_refs_by_target` index
    ([indexer.rs:217-234](skills/visualisation/visualise/server/src/indexer.rs#L217-L234)).
- Cons:
  - **Corpus is mid-migration** (epic 0057). Many older plans don't
    carry `parent: "work-item:NNNN"` yet — they carry the legacy
    `work_item_id:` (36/92 plans) or nothing (52/92). The cluster
    derivation must accept both `parent` and the transitional
    `work_item_id` as fallback during the migration window.
  - **Notes / design-gaps stay orphans** by design; users may want
    those clustered too (today's behaviour for legacy filename-slug
    matches gives accidental notes membership when the slug aligns).
  - Larger code change: new index field, new resolver, watcher /
    refresh-one parity, JSON wire shape (we'd want
    `clusterKey` exposed alongside `slug` for debugging).
  - Doesn't help if the artifact's frontmatter is missing entirely
    (uncommon, but possible). A graceful fallback to Option A's
    behaviour is sensible.

### Option C — Composite cluster key (Option B with slug fallback)

Apply Option B's resolver first; if it returns `None`, fall back to a
normalised slug computed per Option A. This keeps Option B's
correctness for files with typed-linkage frontmatter while continuing
to cluster legacy files by name during the migration window.

- Pros:
  - **Best coverage** during the corpus migration: typed-linkage
    files get the canonical answer; legacy files still cluster by
    accidental slug match.
  - The fallback shrinks naturally as 0057's migration progresses.
  - Notes can opt-in to clustering via frontmatter without forcing
    the schema retroactively.
- Cons:
  - Two systems of record for clustering during the migration —
    debuggability cost. The visualiser would want to expose *why* a
    given entry clusters where it does (was it via frontmatter or
    slug?).
  - Most complex implementation (Option B + Option A, plus tests for
    both paths and their precedence).

### Option D — Defer to epic 0057's migration

Wait for the corpus migration to complete so every artifact carries
its ADR-0034 frontmatter, then implement pure Option B without a
fallback.

- Pros: cleanest end-state; one system of record for clustering.
- Cons: clustering stays visibly broken until 0057 lands across the
  whole corpus. The user-visible bug compounds as new artifacts
  arrive with the new ID-in-filename convention.

## Recommended direction

Between the four options, **Option C (composite key) is the natural
landing zone**: it does the right thing immediately for the cohort
that already carries typed-linkage frontmatter (76/76 plan-reviews,
7/7 validations, 36/92 plans) **and** keeps the existing slug-match
behaviour as a safety net for the rest until 0057 ships.

The smallest sensible diff is to:

1. Add `cluster_key` derivation in the indexer (Option B's resolver),
   reusing the existing secondary indexes
   (`work_item_by_id`, `plans_by_id`, `work_item_refs_by_target`).
2. Tighten `slug.rs` to strip an optional trailing `NNNN-` for
   dated doc types and to accept the no-date `NNNN-slug-review-N`
   shape for work-item reviews (Option A's normalisation).
3. Cluster by `cluster_key.or(slug)` — i.e. typed-linkage when
   available, normalised slug as fallback.
4. Expose `clusterKey` on the wire so the visualiser can show *why*
   an entry joins a cluster (a small debug affordance).

That sequencing means the visible bug ("the plan and the work item
sit in different clusters") collapses on day one, work-item reviews
re-enter clustering, and validations / plan-reviews get correct
parents the moment their existing `target:` frontmatter is read.

## Code References

- `skills/visualisation/visualise/server/src/clusters.rs:53-103` —
  `compute_clusters_with_backfill`; buckets by `entry.slug`.
- `skills/visualisation/visualise/server/src/slug.rs:14-86` —
  per-doc-type slug derivation.
- `skills/visualisation/visualise/server/src/indexer.rs:1156-1185` —
  `build_entry`'s work-item-specific slug branch + frontmatter-first
  `work_item_id` resolution.
- `skills/visualisation/visualise/server/src/indexer.rs:217-234` —
  secondary indexes available for typed-linkage walks
  (`work_item_by_id`, `plans_by_id`, `work_item_refs_by_target`,
  `reviews_by_target`).
- `skills/visualisation/visualise/server/src/related.rs` — existing
  consumer of those secondary indexes; the shape of a
  cluster-key resolver would mirror it.

## Architecture Insights

- The visualiser currently has **two coexisting models of identity**:
  filename-derived `slug` (used for clustering) and frontmatter
  typed links (used for `/api/related/*`). The two were specced
  before ADR-0033/0034 landed; the slug model leaked into clustering
  because it was simpler at the time. Today, the typed-linkage layer
  is richer and more correct.
- The `meta/reviews/work/` directory has its own naming convention
  (`NNNN-slug-review-N.md`) that the slug rule never accommodated;
  this is an isolated mismatch (37/37 files) that any of the options
  above can fix in one slug-rule tweak.
- `meta/research/codebase/` uses the same `YYYY-MM-DD-` convention
  even though research is conceptually `derived_from` by a plan,
  not `parent` of a work-item. Option B's resolver has to walk via
  plan to reach the work-item — research is currently never linked
  directly. That's an architectural choice from ADR-0034 (research
  is reusable across stories), so the clustering rule has to
  consciously do the indirection.
- The 0068 spike's verdict (frontmatter > body parsing) generalises
  to slugs: filename slugs are a body-prose-shaped contract enforced
  by author discipline, and they fail the same way author-written
  prose links fail. Option B's frontmatter walk is the long-term
  shape; Options A and C are migration scaffolding.

## Historical Context

- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` —
  mandates the base schema; `work_item_id` is not in the base.
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — the
  authoritative source for `parent` / `target` / `derived_from` /
  `relates_to` semantics and the type-pair resolution table.
- `meta/decisions/ADR-0025-work-item-cross-ref-aggregation.md` —
  earlier (proposed) aggregation design; ADR-0034 supersedes its
  vocabulary half but its aggregation intent matches Option B.
- `meta/decisions/ADR-0028-common-frontmatter-schema-for-meta-artifacts.md`
  — first pass at common frontmatter; ADR-0033 supersedes.
- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
  — quantified the cost of body-prose link inference; recommendation
  reinforces frontmatter-based resolution.
- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
  — epic tracking the corpus migration; in-progress, gates pure
  Option D.
- `meta/work/0040-pipeline-visualisation-overhaul.md` — the work
  item this research grew out of; the visible clustering bug
  motivated the dig.

## Related Research

- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
  — same family of slug-vs-frontmatter trade-offs.
- `meta/research/codebase/2026-05-21-0064-canonicalise-work-item-id-and-author-fields.md`
  — the field-rename work that made `work_item_id` a coherent
  frontmatter handle.
- `meta/research/codebase/2026-05-30-0065-update-artifact-templates-to-unified-schema.md`
  — template-level rollout of ADR-0033's mandatory fields.
- `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md`
  — established the configurable-pattern work that
  `WorkItemConfig::extract_id` + `normalise_id` now consume.
- `meta/research/codebase/2026-05-31-0040-pipeline-visualisation-overhaul.md`
  — research for the current visualiser overhaul; introduced
  `linkedCount` + `completeness.present` but did not touch the
  cluster key.

## Open Questions

1. **Notes and design-gaps**: should they cluster at all? Today they
   inherit accidental clustering via descriptive slug match.
   ADR-0034 doesn't link them to a work item directly. Option C
   preserves the accidental behaviour; Option B drops them as
   orphans.
2. **Multi-parent ambiguity**: if a plan carries `parent:
   "work-item:0042"` AND its filename embeds `0040`, which wins?
   Option B would prefer frontmatter (correct per ADR-0034) but
   Option A / current behaviour would surface a different cluster.
   We need to document precedence explicitly.
3. **Wire shape**: should we expose `clusterKey` on `IndexEntry`'s
   JSON, or keep it server-internal? The visualiser might want to
   display *why* an entry joined a given cluster during the
   migration window.
4. **Configurable cluster strategy?** Some teams may prefer slug-only
   clustering for simplicity (no frontmatter discipline required).
   The cluster key resolver could be config-driven (`strategy:
   slug | typed-linkage | composite`).
5. **Time horizon**: how mid-migration is epic 0057? If completion
   is imminent (weeks), Option B with no fallback may be cleaner
   than living with a hybrid forever. If completion is far (months),
   Option C is the safer bet.
