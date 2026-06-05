---
type: codebase-research
id: "2026-06-05-0079-aside-region-redesign"
title: "Research: Detail-Page Aside Region Redesign (0079)"
date: "2026-06-05T21:43:50+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0079"
topic: "Detail-Page Aside Region Redesign — Option B aside structure, Cluster block, eyebrow typography unification"
tags: [research, codebase, related-artifacts, aside, eyebrow, typography, lifecycle-cluster, detail-page]
revision: "8058920f5698ac50a0cd809db4499d719cc82ef5"
repository: "visualisation-system"
last_updated: "2026-06-05T21:43:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Detail-Page Aside Region Redesign (0079)

**Date**: 2026-06-05T21:43:50+00:00
**Author**: Toby Clemson
**Git Commit**: 8058920f5698ac50a0cd809db4499d719cc82ef5
**Branch**: HEAD (jj working copy)
**Repository**: visualisation-system

## Research Question

For work item 0079 (Detail-Page Aside Region Redesign): understand the current
state of the three affected surfaces so the work can be planned —
(1) the `RelatedArtifacts` aside and its declared/inferred data model,
(2) the new `Cluster` block and the lifecycle-cluster data it needs, and
(3) eyebrow typography across the page eyebrow, aside section labels, and
stage-rail labels. Confirm the work item's stated assumptions against the
as-shipped code.

## Summary

The redesign decomposes cleanly into three largely independent changes, all in
the frontend (`skills/visualisation/visualise/frontend/`):

1. **Option B aside restructure** is a *purely presentational* frontend change.
   The server already computes declared-vs-inferred classification and returns
   three arrays (`declaredOutbound`, `declaredInbound`, `inferredCluster`); the
   component currently renders them as three fixed-label groups with a legend
   and solid/dashed border modifiers. Collapsing to a single `Related
   artifacts` list with `(declared)`/`(inferred)` text tags requires no
   server, API, or type changes — only edits to `RelatedArtifacts.tsx` and
   `RelatedArtifacts.module.css`.

2. **The `Cluster` block** is net-new wiring, and the work item's data-source
   assumption is **incorrect in an important way**. `cluster-via-label.ts` does
   NOT expose the cluster title, count, or updated timestamp — it returns a
   debug *string* ("clustered via: …"). The fields the block needs come from
   the `LifecycleCluster` object fetched by `fetchLifecycleCluster(slug)`:
   `title` (direct), count (derive as `entries.length`), and `lastChangedMs`
   (the updated timestamp). This means the block needs an *additional fetch*,
   and the planning must account for sourcing the cluster slug + data, not
   merely reading them off the existing related payload.

3. **Eyebrow unification** is a CSS-only change confined to three rules. The
   work item's characterisation is accurate: the page eyebrow
   (`Page.module.css .eyebrow`) is already the canonical treatment; the aside
   labels (`.groupHeading`) and the rail labels (`Pipeline .label`) diverge
   and must be brought onto the same five resolved values. Note `PipelineMini`
   has **no text labels at all** (dots only), so the "PipelineMini `.label`"
   target named in the acceptance criteria does not exist — only `Pipeline`
   has a `.label`.

## Detailed Findings

### Area 1 — Current `RelatedArtifacts` aside structure

**Component**: `skills/visualisation/visualise/frontend/src/components/RelatedArtifacts/RelatedArtifacts.tsx`

- Props: `related: RelatedArtifactsResponse` and optional `showUpdatingHint`
  (`RelatedArtifacts.tsx:6-16`).
- Empty state: single `<p>` when all three arrays are empty
  (`RelatedArtifacts.tsx:19-29`).
- Updating hint: `<p aria-live="polite">Updating…</p>` when
  `showUpdatingHint` (`RelatedArtifacts.tsx:32-36`).
- **Legend**: rendered unconditionally above the groups
  (`RelatedArtifacts.tsx:37`, defined `66-75`) — a `<dl>` with
  `Declared` → "explicit cross-reference in frontmatter." and
  `Inferred` → "shares a slug with this document." This is the legend the
  acceptance criteria require removed.
- **Three fixed groups** (`RelatedArtifacts.tsx:38-61`), rendered only when
  their array is non-empty, in this fixed order:

  | Array | Label | `kind` | testId |
  |---|---|---|---|
  | `related.declaredOutbound` | `Targets` | `declared` | `related-group-declared-outbound` |
  | `related.declaredInbound` | `Referenced by` | `declared` | `related-group-declared-inbound` |
  | `related.inferredCluster` | `Same lifecycle` | `inferred` | `related-group-inferred` |

  The component does NOT compute declared-vs-inferred — it maps each
  server-provided array to a fixed label and `kind`.
- **Row construction** (`RelatedGroup`, `RelatedArtifacts.tsx:84-105`): each
  row is `<Glyph docType framed size=16>` + `<a href="/library/{type}/{slug}">`
  + `<span class=badge>{kind}</span>`. The link is a plain `<a href>` (not a
  TanStack `<Link>`), built via `fileSlugFromRelPath` (`path-utils.ts:6-8`).
  Every row links to that artifact's own detail page regardless of group.

**CSS**: `skills/visualisation/visualise/frontend/src/components/RelatedArtifacts/RelatedArtifacts.module.css`

- Section-label typography — `.groupHeading` (`RelatedArtifacts.module.css:14-20`):
  `font-size: var(--size-xxs)` (12px), `font-weight: 600`,
  `text-transform: uppercase`, `color: var(--ac-fg-muted)`, no `font-family`
  (inherits Inter body font), no `letter-spacing`.
- Legend — `.legend` (`RelatedArtifacts.module.css:1-8`): `--size-xxs`,
  `--ac-fg-muted`, inline `dt`/`dd`.
- **Border differentiation** (`RelatedArtifacts.module.css:39-43`):
  `.groupDeclared { border-left: 2px solid var(--ac-accent); }` and
  `.groupInferred { border-left: 2px dashed var(--ac-fg-faint); }` — the
  `2px solid`/`2px dashed` treatment the criteria require removed.
- Badges (`RelatedArtifacts.module.css:45-55`): shared `.badge` plus
  `.badgeDeclared` (accent) / `.badgeInferred` (faint) colour overrides; these
  render the literal text `declared` / `inferred`.

**Implication for Option B**: collapse the three group renders into one list,
replace the per-group `<h4>` labels + legend with a single `Related artifacts`
heading, and convert the existing `.badge` into the `(declared)`/`(inferred)`
text tag. Remove `.legend`, `.groupDeclared`, `.groupInferred`. The bidirectional
`Targets`/`Referenced by` distinction is simply dropped (both are `declared`).
No data reshaping is needed — the component still receives the same three arrays
and just concatenates them with per-row tags derived from which array each row
came from.

### Area 2 — Declared vs inferred data model (server-computed)

The frontend does **not** read `fm.target` and does **not** compute
same-lifecycle inference. Classification is entirely server-side.

- Wire shape: `RelatedArtifactsResponse` (`src/api/types.ts:224-228`) — three
  `IndexEntry[]` arrays, no per-entry provenance field. Provenance is encoded by
  *which array* an entry lands in.
- Fetch: `fetchRelated` (`src/api/fetch.ts:192-197`) → `GET /api/related/{path}`,
  thin pass-through, no client transform.
- Server resolution: `server/src/related.rs::resolve_related` (`related.rs:22-88`):
  - Inferred cluster (`related.rs:32-51`): sibling entries in the same lifecycle
    cluster (matched on `cluster_key`, falling back to `slug`), self excluded.
  - Declared outbound (`related.rs:53` → `indexer.rs:752-783`): the typed-linkage
    `target:` frontmatter (via `target_path_from_entry`, `indexer.rs:869-887`,
    which reads `entry.frontmatter.get("target")`) plus work-item cross-refs
    (`work_item_id:`/`parent:`/`related:`).
  - Declared inbound (`related.rs:55-68`): reverse `target:` index +
    work-item-id back-references, deduped.
  - Dedup (`related.rs:70-81`): entries appearing in a declared list are removed
    from the inferred list (declared is the more specific signal — so an entry
    shown as `(inferred)` provably has no declared edge).

**Implication**: the "a declared relation is one named by the document's
`target` frontmatter key" definition in the work item maps to
`declaredOutbound` (plus the work-item cross-ref fields). Both `declaredOutbound`
and `declaredInbound` become `(declared)` rows under Option B;
`inferredCluster` becomes the `(inferred)` rows.

### Area 3 — Detail route and doc-type registry

- Detail view: `src/routes/library/LibraryDocView.tsx`. Related data enters at
  `LibraryDocView.tsx:58` via `useDocPageData(entry.relPath)` →
  `useRelated` (`use-related.ts:9-15`) → `fetchRelated`. The view assembles no
  relation data itself; it renders `<RelatedArtifacts related={related.data}
  showUpdatingHint={…} />` inside the aside section
  (`LibraryDocView.tsx:104-122`). The eyebrow is `<EyebrowLabel type={type} />`
  passed as `<Page eyebrow=…>` (`LibraryDocView.tsx:159`), shown only once the
  document resolves.
- Route registration: `/library/$type/$fileSlug` (`router.ts:109-113`), params
  `type` (narrowed to `DocTypeKey`) and `fileSlug`.
- Registry: `DOC_TYPE_KEYS` (`src/api/types.ts:14-19`) has 13 members;
  `VIRTUAL_DOC_TYPE_KEYS` (`types.ts:30-31`) is just `templates`. The 12
  physical detail-page types (registry minus virtual) match the work item's
  enumeration exactly: `decisions`, `work-items`, `plans`, `research`,
  `plan-reviews`, `pr-reviews`, `work-item-reviews`, `validations`, `notes`,
  `pr-descriptions`, `design-gaps`, `design-inventories`.
  `isPhysicalDocTypeKey` (`types.ts:40-44`) is the runtime discriminator.

### Area 4 — Cluster block data source (CORRECTION to work item assumption)

The work item's Dependencies/Technical Notes state the Cluster block's title,
count, and updated timestamp come from `cluster-via-label.ts`. **This is not
correct.**

- `src/routes/lifecycle/cluster-via-label.ts` exports a single function
  `clusterViaLabel(entry, cluster)` (`cluster-via-label.ts:9-31`) that returns a
  plain **`string`** like `"clustered via: parent → work-item:0040"` or
  `"clustered via: slug"`. It is a presentational debug-tag helper used in the
  cluster timeline (`LifecycleClusterView.tsx:227-232`). It exposes **none** of
  title / count / updated, and it does no fetching.
- The real source is the `LifecycleCluster` interface
  (`src/api/types.ts:209-218`):
  ```ts
  interface LifecycleCluster {
    slug: string
    title: string
    entries: IndexEntry[]
    completeness: Completeness
    lastChangedMs: number
    clusterKey: string | null
  }
  ```
  Mapping to what the block renders:
  - **Title** → `cluster.title` (direct).
  - **`<n> artifacts`** → derive as `cluster.entries.length` (no first-class
    count field exists; `LifecycleClusterView` does not render a count today, so
    this derivation is net-new but trivial).
  - **`<updated>`** → `formatMtime(cluster.lastChangedMs)` (the standard
    relative formatter, `src/api/format.ts:19`).
  - **Navigation slug** → `cluster.slug`.
- Fetch path: `fetchLifecycleCluster(slug)` (`src/api/fetch.ts:155-160`) →
  `GET /api/lifecycle/{slug}` → normalised `LifecycleCluster`.
- Route: `/lifecycle/$slug` (`router.ts:127-131`, exported as
  `lifecycleClusterRoute`), param name `slug`.

**Open planning question this raises**: the detail page currently has the
related payload but **not** the `LifecycleCluster`. To render the Cluster block
the view needs (a) the cluster *slug* for this document and (b) a fetch of the
cluster. The entry's `clusterKey`/`slug` (`IndexEntry`, `types.ts:78-110`)
identify membership; resolving from the document to its cluster slug, then
calling `fetchLifecycleCluster`, is the net-new data path. The "belongs to a
cluster" render condition (work item Context) should be re-expressed in terms of
this resolve succeeding (non-empty cluster), since `cluster-via-label.ts` is not
the membership oracle the work item implies — it only *labels* an
already-known membership (`null` clusterKey → slug/orphan fallback).

### Area 5 — Cluster block construction patterns

The closest existing analogue to the proposed block is the lifecycle index card
(`src/routes/lifecycle/LifecycleIndex.tsx:92-123`): a `<Link to="/lifecycle/$slug"
params={{ slug }}>` wrapping a title + a meta row with `formatMtime` and an
artifact count via a local `pluralise(n, 'artifact')` helper
(`LifecycleIndex.tsx:53-55`, not exported — copy the 3 lines or re-derive).

- Preferred navigation form (typed `<Link>`): `LifecycleClusterView.tsx:200-204`
  (`<Link to="/library/$type/$fileSlug" params={…}>`). New code should use
  `<Link>` rather than the raw `<a href>` the existing RelatedArtifacts rows use.
- `<n> artifacts · <updated>` separator convention: the codebase renders the
  literal middle dot as a JSX string between spans, e.g.
  `LifecycleClusterView.tsx:222-226` (`{' · modified '}<time>{formatMtime(…)}</time>`)
  and the `updated <relative>` phrasing with a `ClockIcon` at
  `LifecycleClusterView.tsx:74-84`.
- Aside section shell: each aside block is a `<section><h3>…</h3>…</section>`
  inside `<div className={styles.aside}>` (`LibraryDocView.tsx:104-122`) — the
  Cluster block slots in as a third such section after `File`.

### Area 6 — Eyebrow typography comparison (the five properties)

All tokens live in `src/styles/global.css`. The three surfaces:

| Property | Page eyebrow `.eyebrow` (`Page.module.css:30-40`) | Aside label `.groupHeading` (`RelatedArtifacts.module.css:14-20`) | Rail label `.label` (`Pipeline.module.css:64-73`) |
|---|---|---|---|
| font-family | `--ac-font-mono` (Fira Code) | inherited `--ac-font-body` (Inter) | `--ac-font-mono` (Fira Code) |
| font-size | `--size-eyebrow` = **11px** | `--size-xxs` = **12px** | `--size-4xs` = **9.5px** |
| letter-spacing | `0.12em` (literal) | `normal` (undeclared) | `0.04em` (literal) |
| text-transform | `uppercase` | `uppercase` | `none` (undeclared) |
| color | `--ac-fg-faint` | `--ac-fg-muted` | `--ac-fg-faint` (active → `--ac-fg`) |

Token values (`global.css`): `--size-eyebrow: 11px` (l.184), `--size-xxs: 12px`
(l.182), `--size-4xs: 9.5px` (l.187); `--ac-fg-faint` light `#8b90a3` / dark
`#6c7088`; `--ac-fg-muted` light `rgb(95,99,120)` / dark `#a0a5b8`;
`--ac-font-mono: "Fira Code", ui-monospace, monospace` (l.155). There is also an
unused `--tracking-caps: 0.12em` (l.193) that equals the eyebrow letter-spacing
but is not referenced by any of the three rules.

**What diverges** (to reconcile for the equality criterion):
- Aside label: wrong font-family (Inter not mono), wrong size (12px not 11px),
  missing letter-spacing, wrong colour (muted not faint).
- Rail label: wrong size (9.5px not 11px), wrong letter-spacing (0.04em not
  0.12em), missing uppercase. Font-family and colour already match.

**`EyebrowLabel`** (`src/components/EyebrowLabel/EyebrowLabel.tsx:10-17`,
`EyebrowLabel.module.css:5-9`) carries only flex layout — it inherits all five
typographic properties from the parent `.eyebrow` div, so it is not a fourth
divergent surface.

**Caveat on the acceptance criterion**: it names "`Pipeline` / `PipelineMini`
`.label`". `PipelineMini` (`PipelineMini.module.css`, `PipelineMini.tsx`) renders
**dots only — no text label and no `.label` rule**. Only `Pipeline` has a
`.label`. The criterion should be read as applying to the single `Pipeline
.label`; there is nothing to restyle on `PipelineMini`.

## Code References

- `src/components/RelatedArtifacts/RelatedArtifacts.tsx:18-105` — aside component: empty state, legend, three fixed groups, row construction.
- `src/components/RelatedArtifacts/RelatedArtifacts.module.css:1-55` — `.legend`, `.groupHeading` (l.14-20), `.groupDeclared`/`.groupInferred` borders (l.39-43), `.badge*` (l.45-55).
- `src/api/types.ts:224-228` — `RelatedArtifactsResponse` (three arrays).
- `src/api/types.ts:14-19` / `:30-31` / `:40-44` — `DOC_TYPE_KEYS`, `VIRTUAL_DOC_TYPE_KEYS`, `isPhysicalDocTypeKey`.
- `src/api/types.ts:209-218` — `LifecycleCluster` (title, entries, lastChangedMs, slug, clusterKey).
- `src/api/fetch.ts:192-197` — `fetchRelated`; `:155-160` — `fetchLifecycleCluster`.
- `src/api/format.ts:19` — `formatMtime` (relative "updated" formatter).
- `src/routes/library/LibraryDocView.tsx:58,104-122,159` — related data flow, aside sections, eyebrow.
- `src/routes/lifecycle/cluster-via-label.ts:9-31` — returns a debug string, NOT the cluster object.
- `src/routes/lifecycle/LifecycleClusterView.tsx:74-84,200-204,222-226` — updated-line, typed `<Link>`, `·` separator patterns.
- `src/routes/lifecycle/LifecycleIndex.tsx:53-55,92-123` — closest analogue: cluster card with `pluralise` + `formatMtime`.
- `src/router.ts:109-113,127-131` — `/library/$type/$fileSlug` and `/lifecycle/$slug` routes.
- `src/components/Page/Page.module.css:30-40` — canonical `.eyebrow`.
- `src/components/Pipeline/Pipeline.module.css:64-73` — rail `.label`.
- `src/components/PipelineMini/PipelineMini.module.css` — dots only, no `.label`.
- `src/styles/global.css:153-155,182-193,90-91` — font/size/colour tokens.
- `server/src/related.rs:22-88` — `resolve_related` (declared/inferred classification).
- `server/src/indexer.rs:752-783,869-887` — `declared_outbound`, `target_path_from_entry` (the only place `fm.target` is read).

## Architecture Insights

- **Provenance is server-encoded by array membership, not by a per-entry field.**
  Option B is therefore a render-layer change: the same three arrays are
  concatenated and tagged client-side. No new API contract is needed, and the
  declared/inferred semantics are guaranteed mutually exclusive by the server's
  dedup (`related.rs:70-81`).
- **The Cluster block crosses a data boundary the related payload does not
  cover.** Related data (`/api/related`) and cluster data (`/api/lifecycle`) are
  separate endpoints. The block needs a second fetch keyed by the document's
  cluster slug — the single biggest piece of net-new wiring in this work item,
  and the one the work item under-specifies (it points at the wrong helper).
- **Eyebrow unification is a token/rule-consolidation problem.** Either approach
  the work item allows (point all three at `Page.module.css .eyebrow`, or define
  a shared token tuple) works; the practical blocker is that the three rules use
  three *different* size tokens and two *different* colour tokens today, so the
  fix is per-rule property alignment, not a one-line class swap.
- **TanStack Router typed `<Link>` is the modern convention**; the existing
  RelatedArtifacts rows use raw `<a href>` (older). New Cluster wiring should use
  `<Link>`; the row links could optionally be modernised but that is out of
  scope for 0079.

## Historical Context

- `meta/work/0079-aside-region-redesign.md` — the work item; already reconciled
  through two review passes (Option B chosen, eyebrow rule = existing page
  eyebrow).
- `meta/reviews/work/0079-aside-region-redesign-review-1.md` — flags that
  handling of inferred relations after the `Same lifecycle` group collapses was
  left ambiguous (since resolved: inferred rows stay as `(inferred)` tags, the
  Cluster block is additive).
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — the gap doc 0079 was sliced from; note its "four sections / separate
  `Declared links` block" prose is looser than the prototype's real single-list
  structure (the work item deliberately supersedes it).
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/`
  — `prototype-standalone.html` carries the flat `.ac-related` list with
  `.ac-related__tag.is-declared` accent modifier (the structure Option B
  follows) and the Fira Code mono eyebrow labels.
- `meta/work/0040-pipeline-visualisation-overhaul.md` +
  `meta/plans/2026-05-31-0040-pipeline-visualisation-overhaul.md` +
  `meta/research/codebase/2026-05-31-0040-pipeline-visualisation-overhaul.md` —
  owns the lifecycle clusters, `/lifecycle/<slug>` routes, `cluster-via-label.ts`,
  and the `Pipeline` rail labels this work touches. Functionally complete.
- `meta/research/codebase/2026-06-01-lifecycle-clustering-slug-mismatch.md` +
  `meta/plans/2026-06-01-lifecycle-clustering-composite-key.md` — cluster
  slug/key matching, relevant to whether a Cluster block resolves.
- `meta/decisions/ADR-0025-work-item-cross-ref-aggregation.md`,
  `ADR-0034-typed-linkage-vocabulary.md` — how declared relations are
  aggregated/vocabularised (the `(declared)` rows).
- `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md` — governs
  the eyebrow/label type sizing the unification must respect.
- `meta/work/0074-per-doc-type-hues-on-detail-page.md`,
  `0075-typography-size-scale-consumption.md` — adjacent eyebrow icon/size work;
  coordinate so 0079's rule consolidation lands first.

## Related Research

- `meta/research/codebase/2026-05-31-0040-pipeline-visualisation-overhaul.md`
- `meta/research/codebase/2026-06-01-lifecycle-clustering-slug-mismatch.md`
- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
- `meta/research/codebase/2026-05-23-0075-typography-size-scale-consumption.md`
- `meta/research/codebase/2026-05-24-0074-per-doc-type-hues-on-detail-page.md`

## Open Questions

1. **Cluster slug resolution from a document.** The Cluster block needs the
   document's cluster slug to fetch `LifecycleCluster`. The `IndexEntry` carries
   `clusterKey`/`slug`, but the explicit document→cluster-slug resolution path
   (and whether `fetchLifecycleCluster` should be called from `LibraryDocView`
   or a new hook) is not yet pinned. The work item points at
   `cluster-via-label.ts` for this, which is wrong — planning must choose the
   real resolution path. (Slug-mismatch research above is relevant.)
2. **`<n> artifacts` definition.** Confirm the count should be
   `cluster.entries.length` (all cluster members) vs. excluding the current
   document. `LifecycleClusterView` renders no count today, so there is no
   precedent to match.
3. **PipelineMini scope.** The acceptance criterion names a `PipelineMini
   .label` that does not exist (mini = dots only). Confirm the criterion is
   satisfied by restyling the single `Pipeline .label`, or amend the criterion.
4. **0040 status transition.** Confirm 0040 is transitioned out of in-progress
   before 0079 closes (per the work item's prerequisite), since 0079 edits the
   `Pipeline` rail labels 0040 owns.
