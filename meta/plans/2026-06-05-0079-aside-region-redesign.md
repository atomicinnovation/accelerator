---
type: plan
id: "2026-06-05-0079-aside-region-redesign"
title: "Detail-Page Aside Region Redesign Implementation Plan"
date: "2026-06-05T22:36:44+00:00"
author: "Toby Clemson"
producer: create-plan
status: done
work_item_id: "0079"
reviewer: "Toby Clemson"
tags: [design, frontend, detail-page, aside, eyebrow, lifecycle-cluster]
revision: "4daa574d16893179833fe762052c84a863e2d635"
repository: "visualisation-system"
last_updated: "2026-06-06T01:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
relates_to: ["work-item:0079", "codebase-research:2026-06-05-0079-aside-region-redesign"]
---

# Detail-Page Aside Region Redesign Implementation Plan

## Overview

Settle and implement a single canonical aside structure for the detail page.
Three independent, independently-mergeable changes:

1. **Option B aside restructure** — collapse the three `RelatedArtifacts`
   groups (`Targets` / `Referenced by` / `Same lifecycle`) into a single
   `Related artifacts` list whose rows carry an accent `(declared)` or faint
   `(inferred)` text tag; remove the legend and the `2px solid` / `2px dashed`
   border differentiation.
2. **Cluster block** — add a dedicated `Cluster` aside section (after `File`)
   that links to the lifecycle pipeline view (`/lifecycle/<slug>`) with the
   cluster title and `<n> artifacts · <updated>` metadata, rendered whenever
   the document is a member of any lifecycle cluster.
3. **Eyebrow unification** — bring the aside section labels and the lifecycle
   stage-rail labels onto the canonical page-eyebrow typography (Fira Code mono,
   11px, `0.12em`, uppercase, `--ac-fg-faint`).

## Current State Analysis

All three surfaces live in the frontend
(`skills/visualisation/visualise/frontend/`). The server requires **no
changes** — it already computes declared/inferred classification and already
serves full lifecycle-cluster objects.

### Aside structure (Option B target)

- `RelatedArtifacts.tsx:18-105` renders, in fixed order: an unconditional
  `<Legend>` (`:37`, `:66-75`), then three `RelatedGroup`s — `Targets`
  (`declaredOutbound`), `Referenced by` (`declaredInbound`), `Same lifecycle`
  (`inferredCluster`) — each an `<h4 class=groupHeading>` + `<ul>` of rows. Each
  row is `<Glyph framed size=16>` + `<a href="/library/{type}/{slug}">` +
  `<span class=badge>{declared|inferred}</span>`
  (`RelatedArtifacts.tsx:84-105`).
- `RelatedArtifacts.module.css`: `.legend` (`:1-8`), `.groupHeading` (`:14-20`,
  `--size-xxs` / weight 600 / uppercase / `--ac-fg-muted`, no mono, no
  tracking), `.groupDeclared { border-left: 2px solid var(--ac-accent) }` /
  `.groupInferred { border-left: 2px dashed var(--ac-fg-faint) }` (`:42-43`),
  `.badge*` (`:45-55`).
- The component does **not** compute provenance — it maps each
  server-provided array to a fixed label + `kind`
  (`RelatedArtifacts.tsx:38-61`). Provenance is server-encoded by array
  membership (`server/src/related.rs:22-88`); declared/inferred are mutually
  exclusive by the server's dedup (`related.rs:70-81`).
- The wire shape `RelatedArtifactsResponse` (`types.ts:224-228`) is three
  `IndexEntry[]` arrays — unchanged by this work.

### Aside section headings (the eyebrow target after Option B)

- The aside section labels are the `<h3>` headings in `LibraryDocView.tsx`
  (`Related artifacts` `:106`, `File` `:124`), styled by `.aside h3`
  (`LibraryDocView.module.css:10`): `--size-xxs` (12px), weight 600, uppercase,
  **`--ac-fg-faint`** (colour already matches the eyebrow), **no** font-family
  (inherits Inter), **no** letter-spacing.
- The per-group `<h4 class=groupHeading>` headings disappear under Option B, so
  `.aside h3` — not `.groupHeading` — is the surviving "aside section label"
  surface the eyebrow criterion targets.

### Cluster block data source

- `cluster-via-label.ts:9-31` returns a **debug string** (`"clustered via:
  …"`), not the cluster object — the work item points at the wrong helper.
- The fields the block needs come from a `LifecycleCluster`
  (`types.ts:209-218`): `title` (direct), count = `entries.length`, updated =
  `formatMtime(lastChangedMs)` (`format.ts:19`), nav slug = `slug`.
- A cluster's `slug` is a **representative** slug chosen server-side
  (`clusters.rs::pick_representative_slug`) — usually the work item's slug, and
  **not reliably derivable** from an arbitrary member entry's fields. The robust
  membership/data path is therefore: fetch the cluster **list**
  (`fetchLifecycleClusters`, `fetch.ts:148-153`; cached under
  `queryKeys.lifecycle()`) and find the cluster whose `entries[]` contains this
  document **by `path`**.
- Server confirms each clustered document is in **exactly one** returned cluster
  (singletons and orphan-by-design buckets included; no filtering); documents
  with `slug == null && clusterKey == null` are in **no** cluster and carry
  `completeness == null` (`indexer.rs:181-185`). This is the negative case.
- **Consequence — the negative case is narrow**: every *slug-bearing*
  lifecycle-participating entry falls into at least a singleton slug-bucket, and
  every orphan-by-design type into its own per-path bucket
  (`clusters.rs` tests `plan_without_typed_linkage_falls_back_to_slug_bucket`,
  `orphan_types_with_colliding_slugs_do_not_merge`); only a slug-less
  **lifecycle-participating** entry yields `bucket_key = None` and is excluded
  (`entries_without_slug_are_excluded` uses a `Plans` entry). Note a slug-less
  **orphan-by-design** type (notes/decisions/design-*) still gets a
  `__orphan__::<path>` bucket, so it is *not* a no-cluster case. So the path-match
  returns a non-null cluster for **nearly every detail page** — frequently a
  self-only `1 artifact` cluster. Per the decision below we still render on any
  membership; the practical upshot is the block is near-ubiquitous (not
  occasional), and the only genuine "no Cluster block" case is a slug-less
  lifecycle-participating document. **For tests, prefer the robust path-match
  negative**: a cluster list deliberately missing the doc's `path` (exercises the
  hook's negative branch independent of server bucketing rules); if using a
  no-slug fixture, make it a lifecycle-participating type (e.g. a `Plan` with
  `slug == null`), not a slug-less note/decision.
- Nav + meta pattern to copy: `LifecycleIndex.tsx:92-123` (`<Link
  to="/lifecycle/$slug" params={{ slug }}>` + `formatMtime` +
  `pluralise(n,'artifact')` at `:53-55`).

### Eyebrow typography (the five properties)

| Property | `.eyebrow` (`Page.module.css:30-40`) | `.aside h3` (`LibraryDocView.module.css:10`) | `.label` (`Pipeline.module.css:64-73`) |
|---|---|---|---|
| font-family | `--ac-font-mono` ✓ | inherited Inter ✗ | `--ac-font-mono` ✓ |
| font-size | `--size-eyebrow` 11px ✓ | `--size-xxs` 12px ✗ | `--size-4xs` 9.5px ✗ |
| letter-spacing | `0.12em` ✓ | undeclared ✗ | `0.04em` ✗ |
| text-transform | uppercase ✓ | uppercase ✓ | undeclared ✗ |
| color | `--ac-fg-faint` ✓ | `--ac-fg-faint` ✓ | `--ac-fg-faint` ✓ |

- `--tracking-caps: 0.12em` exists in `global.css` but is unused; adopting it in
  all three rules makes the shared tracking explicit (the "shared token" path
  the AC permits).
- `PipelineMini` has **no** `.label` (dots only) — that AC clause is vacuous.
- The rail label keeps its active-stage override
  (`.stage[data-active='true'] .label { color: var(--ac-fg) }`,
  `Pipeline.module.css:75-77`) as intentional emphasis; the equality check
  targets an **inactive** rail label.
- **`.label` is a shared rule across two variants.** `Pipeline` renders the
  `.chain` with `data-variant='card'` in the lifecycle-index cluster cards
  (`LifecycleIndex.tsx:118`) and `data-variant='panel'` in the cluster detail
  rail (`LifecycleClusterView.tsx:92`). The bare `.label` rule
  (`Pipeline.module.css:64-73`) styles **both**. The eyebrow promotion targets
  only the **detail rail**, so it must be scoped to
  `.chain[data-variant='panel'] .label` (the variant attribute pattern already
  used for `.tile`, `Pipeline.module.css:32-40`); the index-card labels
  (`card` variant) are explicitly **out of scope** and keep `--size-4xs` /
  `0.04em` / mixed-case.

## Desired End State

- The detail-page aside renders, in fixed DOM order: `Related artifacts`
  (single list, per-row `(declared)`/`(inferred)` tags, no legend, no
  solid/dashed borders) → `File` → `Cluster` (when the doc is in a cluster).
- The `Cluster` block shows title + `<n> artifacts · <updated>` and navigates to
  `/lifecycle/<slug>`; absent only when the doc is in no cluster.
- The page eyebrow, the aside `<h3>` section labels, and the (inactive)
  **panel-variant** rail `.label` resolve to identical values for **six**
  properties: font-family (mono), font-size (11px), letter-spacing (`0.12em` →
  `1.32px` computed), text-transform (uppercase), color (`--ac-fg-faint`), and
  **font-weight** (the eyebrow declares none and inherits; `.aside h3` drops its
  explicit `600` to match — see Phase 3). The lifecycle-index **card-variant**
  rail labels are unchanged.

### Key Discoveries

- Option B is a pure render-layer change — no server/API/type edits
  (`related.rs:22-88`, `types.ts:224-228`).
- Cluster data must come from `fetchLifecycleClusters` matched by `path`, **not**
  `cluster-via-label.ts` and **not** slug-derivation (`clusters.rs`
  representative-slug logic).
- The eyebrow target is `.aside h3` (`LibraryDocView.module.css:10`), not
  `.groupHeading`, once Option B lands.
- `Pipeline .label` is a **shared rule across the `card` and `panel` variants**;
  the eyebrow promotion must be scoped to `.chain[data-variant='panel']` so the
  lifecycle-index cluster-card labels are not restyled.
- The Cluster block is **near-ubiquitous**: nearly every slug-bearing doc is in
  at least a singleton cluster, so the only true negative case is a `slug == null`
  document. `useDocCluster` must also distinguish loading/error from "no cluster"
  (all otherwise collapse to `null`) so the block degrades visibly.
- The shared test router (`src/test/router-helpers.tsx`) has **no
  `/lifecycle/$slug` route** — it must be added before the `<Link>` href tests.
- Existing tests assert the **old** structure and must be rewritten first
  (`RelatedArtifacts.test.tsx`, `aside-row-resolved-colours.spec.ts`).

## What We're NOT Doing

- No server, API contract, or `types.ts` shape changes.
- Not modernising the existing `RelatedArtifacts` row links from `<a href>` to
  typed `<Link>` (out of scope; new Cluster wiring uses `<Link>`).
- Not restyling any lifecycle-view label other than the **panel-variant**
  `Pipeline .label` (the cluster-detail rail). The `card`-variant `.label`
  (lifecycle-index cluster cards), `.tcardStage`, and the pipeline-panel eyebrow
  are out of scope — Phase 3 scopes the rule to `.chain[data-variant='panel']`
  precisely so the index cards are not touched.
- Not touching the eyebrow **icon or size** work owned by 0074 / 0075 (only
  font-family / tracking / transform / colour here).
- Not filtering singleton / orphan clusters out of the Cluster block (per
  decision: render on any cluster membership).
- Not changing the `<n>` count to exclude the current document — it is
  `cluster.entries.length` (all members), matching `LifecycleIndex`.

## Implementation Approach

Three sequential phases, each a complete, shippable PR that leaves the app
working. Sequencing (1 → 2 → 3) avoids rework: Phase 1 establishes `.aside h3`
as the section-label surface (removing `.groupHeading`), Phase 2 adds the third
`.aside h3` (`Cluster`), and Phase 3's eyebrow rule then covers all aside
section labels in one pass. Each phase is test-driven: rewrite/author the
failing tests first, then implement to green.

Test commands (per project convention — `mise` tasks, no lint task):
- Frontend unit: `mise run test:unit:frontend`
- E2E / visual-regression: `mise run test:e2e:visualiser`
- Typecheck only (no lint): the frontend typecheck task.

---

## Phase 1: Option B Aside Restructure

### Overview

Collapse the three `RelatedGroup`s into a single `Related artifacts` list whose
rows carry an accent `(declared)` or faint `(inferred)` text tag; remove the
legend and the solid/dashed border modifiers. Purely presentational.

### Changes Required

#### 1. Rewrite the component test (red first)

**File**: `src/components/RelatedArtifacts/RelatedArtifacts.test.tsx`
**Changes**: Replace the old-structure assertions with Option B expectations:

- Keep: all-empty message; `showUpdatingHint` behaviour; per-row decorative
  `Glyph` with `data-doc-type` + `aria-hidden`.
- Remove: `h4` group-heading assertions (`Targets` / `Referenced by` / `Same
  lifecycle`); legend `Declared`/`Inferred` text; `groupDeclared`/`groupInferred`
  modifier-class assertions; the three `related-group-*` testid assertions.
- Add:
  - all rows from `declaredOutbound` + `declaredInbound` render with a
    `(declared)` tag; all rows from `inferredCluster` render with an
    `(inferred)` tag — in one list (single `<ul>`), no sub-group headings
    (`screen.queryByRole('heading', { level: 4 })` is null).
  - row order is declared-first then inferred (declaredOutbound,
    declaredInbound, inferredCluster concatenated). Assert the order
    **positionally** over a fixture whose entries all have **distinct `path`s**
    (so dedup cannot shorten the list and shift the indices — keep the
    bidirectional-dedup case as a *separate* test) and distinguishable titles
    (`getAllByRole('listitem')` text in exact index order), so a reordered
    concatenation or a dropped source array fails.
  - **declared dedup**: a fixture where the same entry (same `path`) appears in
    **both** `declaredOutbound` and `declaredInbound` renders that entry **once**
    (one row, one `(declared)` tag) — guards the bidirectional-duplicate /
    colliding-key bug.
  - each row still links to `/library/{type}/{slug}`.
  - no legend element renders.

#### 2. Restructure the component

**File**: `src/components/RelatedArtifacts/RelatedArtifacts.tsx`
**Changes**: Replace the three conditional `RelatedGroup` renders + `<Legend>`
with a single list. Concatenate `[...declaredOutbound, ...declaredInbound]`
(tag `declared`) and `inferredCluster` (tag `inferred`); render one `<ul>` of
rows under the single section. Each row keeps `<Glyph docType framed size=16>` +
`<a href>` and renders the tag text. Delete the `Legend` function and the
`RelatedGroup`/`kind`-to-label mapping.

- **Empty-state must key off the combined rows**, not a single array: compute
  emptiness from `rows.length === 0` (equivalently all three source arrays
  empty) so an inferred-only or declared-only document still renders correctly.
  Keep the existing all-empty message and the `showUpdatingHint` branch.
- **Decouple display copy from the discriminant**: render the visible tag text
  from an explicit map rather than interpolating the `kind` literal, so copy and
  CSS class can diverge later (and `aria-label`/copy can differ from the class):
  ```tsx
  const TAG_TEXT = { declared: '(declared)', inferred: '(inferred)' } as const
  ```
- **Testids/classes (non-optional renames)** — the old `related-group-*` family
  no longer fits a single list. Use `related-list` for the `<ul>`,
  `related-row` per `<li>`, and `related-tag` (plus a `kind`-specific class) for
  the tag `<span>`, keeping the greppable `related-` prefix.

- **Dedup the declared list by `path`** before mapping to rows. The server does
  **not** dedup across `declaredOutbound` and `declaredInbound`
  (`related.rs:53-68` builds them independently, deduping only *within* each list
  and only between inferred-vs-declared). An artifact with a **bidirectional**
  declared relationship (A's `related:` targets B, and B's `target:` is A)
  appears in **both** arrays; the old grouped UI tolerated this because the two
  appearances sat under distinct `Targets` / `Referenced by` headings, but the
  flat Option B list would render it twice with identical `(declared)` tags and a
  **colliding `key={entry.path}`** (React reconciliation warning). Keep the first
  occurrence when deduping. (Directionality is intentionally collapsed at the
  render boundary — the server contract is untouched, so it can be re-surfaced
  later if needed.)

```tsx
const declaredAll = [...related.declaredOutbound, ...related.declaredInbound]
const seen = new Set<string>()
const declared = declaredAll.filter((e) => !seen.has(e.path) && seen.add(e.path))
const inferred = related.inferredCluster
const rows = [
  ...declared.map((entry) => ({ entry, kind: 'declared' as const })),
  ...inferred.map((entry) => ({ entry, kind: 'inferred' as const })),
]
if (rows.length === 0) return /* existing all-empty message */
// single <ul data-testid="related-list"> of rows; each row:
//   <li data-testid="related-row" data-kind={kind} key={entry.path}> Glyph + <a> +
//     <span data-testid="related-tag" class={tagClass(kind)}>{TAG_TEXT[kind]}</span>
```

> **Stable selector hook**: add `data-kind={kind}` (`"declared"`/`"inferred"`) to
> the `<li>`. Visual-regression specs run against the built bundle where CSS-module
> class names (`.tagInferred` etc.) are **hashed and unselectable** — the existing
> spec selects only via `data-testid`/`data-*` and stable `svg[data-doc-type]`.
> `data-kind` gives the specs a stable way to scope to inferred vs declared rows
> without depending on a module class. (The CSS rules themselves still use the
> module classes — module scoping resolves those correctly inside `.module.css`.)

#### 3. Trim the CSS

**File**: `src/components/RelatedArtifacts/RelatedArtifacts.module.css`
**Changes**: Remove `.legend` + `.legend dt/dd`, `.groupHeading`,
`.groupDeclared`, `.groupInferred`. Rename `.groupList`/`.groupItem` →
`.list`/`.item` and `.badgeDeclared`/`.badgeInferred` → `.tagDeclared`/
`.tagInferred` (the post-Option-B reality has no "group" or "badge"; the renames
are **non-optional** so class names match the single-list structure). Repurpose
the tag colours as the inline `(declared)`/`(inferred)` tag styling — accent
(`--ac-accent`) for declared, faint (`--ac-fg-faint`) for inferred. No
`2px solid`/`2px dashed` rule remains.

> **Contrast note**: with the legend removed, the `(inferred)` tag is now
> standalone meaning-bearing text rather than a decorative badge. Per decision
> the terms stand on their own words (no tooltip/caption gloss), but the faint
> colour must still be verified against WCAG 1.4.3 (4.5:1) — see Phase 1 Manual
> Verification. If `--ac-fg-faint` on `(inferred)` is below threshold, darken the
> tag token (not the row text) to clear it while keeping the accent/faint
> declared-vs-inferred distinction.

#### 4. Update the aside-row visual-regression spec

**File**: `tests/visual-regression/aside-row-resolved-colours.spec.ts`
**Changes**: Replace `related-group-inferred` / `related-group-declared-*`
testid selectors with the new `related-list` / `related-row` / `related-tag`
selectors. Remove the `inferred group border is 2px dashed` test. Keep the
per-doc-type row-icon colour assertions and the row-container invariance
(transparent bg, `0px` border, `align-items: center`) retargeted at the new list
rows. **Retarget the coverage-guard count to inferred-row icons specifically** — the
original guard counts `svg[data-doc-type]` icons in the inferred group
(`[data-testid="related-group-inferred"] svg[data-doc-type]`). Now that declared
and inferred share one `<ul>`, scope the locator to the *icons within inferred
rows* using the new stable `data-kind` hook —
`[data-testid="related-row"][data-kind="inferred"] svg[data-doc-type]` — **not** a
CSS-module class (`.tagInferred` is hashed and unselectable in the built bundle)
and not all rows/icons in the list, so it stays an exact
`PHYSICAL_DOC_TYPE_KEYS.length - 1` assertion rather than over-counting
declared-row icons the anchor fixture carries. Likewise retarget the per-doc-type
icon-colour assertions and the row-container invariance checks from
`[data-testid="related-group-inferred"] …` to `[data-testid="related-row"] …`
(scoping by `[data-kind="inferred"]` where the original was inferred-specific).
The precise selector is load-bearing for the equality.

### Success Criteria

#### Automated Verification

- [x] Rewritten unit test fails before the component change, passes after:
  `mise run test:unit:frontend`
- [x] Frontend unit suite passes: `mise run test:unit:frontend`
- [x] Typecheck passes (frontend typecheck task)
- [x] Updated aside-row visual-regression spec passes:
  `mise run test:e2e:visualiser`

#### Manual Verification

- [ ] Detail-page aside shows one `Related artifacts` list; declared rows carry
  an accent `(declared)` tag, inferred rows a faint `(inferred)` tag.
- [ ] No legend renders; no row has a `2px solid`/`2px dashed` border.
- [ ] The `(inferred)` faint tag meets WCAG 1.4.3 text contrast (≥ 4.5:1)
  against the aside background in both light and dark themes (it is now
  meaning-bearing text, not a decorative badge). Darken the tag token if not.
- [ ] Verified across a representative document per physical doc type
  (`DOC_TYPE_KEYS` minus `templates`): `decisions`, `work-items`, `plans`,
  `research`, `plan-reviews`, `pr-reviews`, `work-item-reviews`, `validations`,
  `notes`, `pr-descriptions`, `design-gaps`, `design-inventories`.

---

## Phase 2: Cluster Block

### Overview

Add a `Cluster` aside section (after `File`) that resolves the document's
lifecycle cluster from the cluster list and links to `/lifecycle/<slug>` with
title + `<n> artifacts · <updated>`. Renders whenever the document is a member
of any cluster; absent otherwise.

### Changes Required

#### 1. Shared `pluralise` helper (extract for reuse)

**File**: `src/api/format.ts`
**Changes**: Export `pluralise(n, singular, plural?)` (lifted verbatim from
`LifecycleIndex.tsx:53-55`) so both the index card and the Cluster block share
one implementation. Update `LifecycleIndex.tsx` to import it. Now that it is
shared infrastructure, give it first-class coverage: add a direct unit test in
`src/api/format.test.ts` for `n = 0` (`0 artifacts`), `n = 1` (`1 artifact`),
`n > 1` (`3 artifacts`), and the explicit-`plural?` override — rather than
relying solely on the indirect `RelatedCluster` rendering test.

> Helper placement is kept in `format.ts` (alongside the other shared formatters
> the new block already imports — `formatMtime`); a dedicated `strings.ts` was
> considered but rejected as unnecessary churn for one extra helper.

#### 2. Cluster-resolution hook

**File**: `src/api/use-doc-cluster.ts` (new)
**Changes**: `useDocCluster(entry: IndexEntry | undefined)` wraps
`useQuery(queryKeys.lifecycle(), fetchLifecycleClusters)` (shares the cache with
`LifecycleIndex`) and derives the matching cluster:

```ts
export function useDocCluster(entry: IndexEntry | undefined) {
  const query = useQuery({
    queryKey: queryKeys.lifecycle(),
    queryFn: fetchLifecycleClusters,
    enabled: !!entry,
  })
  const cluster = useMemo(
    () =>
      entry
        ? (query.data?.find((c) => c.entries.some((e) => e.path === entry.path)) ?? null)
        : null,
    [query.data, entry?.path],
  )
  // Return the full query result + the derived value, matching the sibling
  // read-side hooks (`useRelated`/`useDocContent` return the bare UseQueryResult;
  // `useDocPageData` returns members that are full query results). This keeps the
  // `.isPending`/`.isError`/`.data`/`.isFetching`/`.refetch` surface consistent
  // and lets the caller distinguish loading/error from "genuinely no cluster"
  // (all three otherwise collapse to cluster === null).
  return { ...query, cluster }
}
```

- Match by `path` (robust against representative-slug edge cases). Add a code
  comment noting `fetchLifecycleCluster(slug)` / `queryKeys.lifecycleCluster(slug)`
  are deliberately **not** used here — the representative slug is not derivable
  from an arbitrary member entry, so the list-plus-path-match is the robust path
  (and shares the index cache).
- **`isPending` caveat**: with `enabled: !!entry`, a disabled query (no entry)
  reports `isPending: true` / `fetchStatus: 'idle'`. In the view this is masked
  (the cluster section only renders inside the resolved `entry && content.data`
  branch, so the query is genuinely enabled). The `useDocCluster` loading
  **test** must therefore drive an *enabled* query with a never-resolving
  promise (the `new Promise(() => {})` pattern the existing `LibraryDocView`
  "renders Loading" test uses) so the loading assertion cannot pass via the
  disabled-idle path.
- **`cluster === null` is ambiguous on its own**: it is returned during the
  initial fetch (`query.data` undefined), on fetch error, *and* when the doc is
  genuinely in no cluster. The caller (Phase 2 §4) therefore branches on
  `isPending`/`isError` explicitly rather than on `cluster` alone, so the block
  degrades visibly instead of silently vanishing while `/api/lifecycle` loads on
  a cold cache or fails.
- **Memoise the derivation** (`useMemo` over `[query.data, entry?.path]`) to
  match the `LifecycleIndex` idiom (`LifecycleIndex.tsx:65-68`) and avoid
  re-scanning the full cluster list on unrelated re-renders (e.g. SSE
  invalidations, hint-state changes).
- The genuine "no cluster" result (`cluster === null` *after* the query settles)
  is reached only by a `slug == null` document — see Current State Analysis.

#### 3. Presentational Cluster block component

**File**: `src/components/RelatedCluster/RelatedCluster.tsx` (new) +
`RelatedCluster.module.css` (new)
**Changes**: `RelatedCluster({ cluster }: { cluster: LifecycleCluster })`
renders a typed `<Link to="/lifecycle/$slug" params={{ slug: cluster.slug }}>`
wrapping the title and a meta row:
`{pluralise(cluster.entries.length, 'artifact')}{' · '}<time>{formatMtime(cluster.lastChangedMs)}</time>`
(mirrors `LifecycleClusterView.tsx:222-226` separator + `LifecycleIndex` card
meta). Module CSS may reuse `LifecycleIndex` card idioms; keep it minimal.

#### 4. Wire into the detail view

**File**: `src/routes/library/LibraryDocView.tsx`
**Changes**: Call `useDocCluster(entry)` alongside `useDocPageData`. After the
`File` `<section>` (`:123-126`), add a third `<section>` that **degrades
visibly** — mirroring the existing `Related artifacts` section's
`isError` → `role="alert"` / `isPending && !isError` → `Loading…` / `data` →
render pattern (`LibraryDocView.tsx:107-122`). Render the section shell while the
cluster query is pending so the `<h3>Cluster</h3>` heading and a loading/error
state are shown rather than the block silently vanishing; only suppress the
section entirely once the query has **settled** with no matching cluster:

```tsx
const { cluster, isPending: clusterPending, isError: clusterError } = useDocCluster(entry)
...
{(clusterPending || clusterError || cluster) && (
  <section>
    <h3>Cluster</h3>
    {clusterError ? (
      <p role="alert" className={styles.error}>
        Failed to load cluster. Try again later.
      </p>
    ) : clusterPending ? (
      <p>Loading…</p>
    ) : cluster ? (
      <RelatedCluster cluster={cluster} />
    ) : null}
  </section>
)}
```

- **Narrow `cluster` explicitly in the inner render** (`cluster ? … : null`).
  The outer `(clusterPending || clusterError || cluster)` guard does **not**
  propagate non-null narrowing into the inner ternary, so passing `cluster`
  (typed `LifecycleCluster | null`) into `RelatedCluster`'s non-nullable
  `cluster: LifecycleCluster` prop without the inner `cluster ?` check would fail
  typecheck (and a forced cast would let a future guard edit leak `null` into a
  component that dereferences `cluster.slug`/`cluster.entries`). The explicit
  inner branch makes the non-null guarantee local and machine-checked.
- **Error copy** mirrors the neighbouring patterns rather than a bare string —
  align with `LifecycleIndex`'s `FetchError`-aware wording for the same
  `/api/lifecycle` endpoint (a recovery hint at minimum; optionally surface
  `error.message` as the `Related artifacts` section does).

DOM order is therefore `Related artifacts` → `File` → `Cluster`. The genuine
"no cluster" case (settled, `cluster === null`) renders no third section at all.

> **Consider extracting the degradation branch.** This `isError → role="alert"` /
> `isPending → Loading…` / `data → render` cascade now appears twice in the same
> file (the `Related artifacts` section, `:107-122`, and here). A tiny local
> `QueryState`/`AsideSection` helper taking `{ isPending, isError, error,
> children }` would avoid two hand-maintained copies; if extraction feels
> premature for two call sites, add a cross-linking comment so the parallel is
> intentional.

> **Note**: the cluster read query is wired directly into the view rather than
> folded into `useDocPageData`. Composing it into that hook (its stated read-side
> fanout join point) was considered but deferred — keeping the
> pending/error/data branching beside the sibling `related` branching in the view
> is the more legible match for the visible-degradation pattern above. Revisit if
> a third read-side query lands.

#### 5. Tests (red first)

**Files**: `src/test/router-helpers.tsx` (extend — prerequisite),
`src/api/test-fixtures.ts` (extend — prerequisite, add `makeLifecycleCluster`),
`src/components/RelatedCluster/RelatedCluster.test.tsx` (new),
`src/api/use-doc-cluster.test.ts` (new or fold into the view test),
`src/api/format.test.ts` (**new** — does not exist yet; `pluralise`, see §1),
`src/routes/library/LibraryDocView.test.tsx` (extend)
**Changes**:
- **Prerequisite — register the lifecycle route in the test router**: the shared
  `buildTestRouter` (`src/test/router-helpers.tsx`) registers only `/`,
  `/library/$type`, and `/library/$type/$fileSlug`. A TanStack
  `<Link to="/lifecycle/$slug">` to an **unregistered** route will not resolve to
  the expected `href` (and may warn), so the planned href assertion cannot pass
  as written. Add a `/lifecycle/$slug` route (mirroring the existing library
  routes) to `buildTestRouter` *before* authoring the tests below.
- **Prerequisite — add a `makeLifecycleCluster(overrides)` factory** to
  `src/api/test-fixtures.ts` (which has `makeIndexEntry`/`makeCompleteness` but no
  cluster factory; existing cluster tests hand-build verbose `LifecycleCluster`
  literals incl. `completeness`/`lastChangedMs`/`clusterKey`). Route all new
  cluster tests through it so a future `LifecycleCluster` shape change updates one
  place, and so the awkward `slug == null` negative-case fixture is easy to build.
- **Prerequisite — stub `fetchLifecycleClusters` across the existing suite**:
  wiring `useDocCluster(entry)` into the view makes **every** existing
  `LibraryDocView` test that reaches the resolved-document branch newly call
  `fetchLifecycleClusters` → a raw `fetch('/api/lifecycle')` (there is no global
  `fetch` stub in `src/test/setup.ts`), producing rejections/console noise and an
  unmocked query. Add a default `vi.spyOn(fetchModule, 'fetchLifecycleClusters')`
  resolving `[]` in a `beforeEach` using `mockResolvedValue([])` (not
  `mockImplementationOnce`, so it survives across all tests). The new
  cluster-present/pending/error tests **override** it by re-calling
  `vi.spyOn(fetchModule, 'fetchLifecycleClusters')` inside the test body (matching
  the existing inline-spy convention) — the override must come after the default,
  so do not place the default spy after a per-test override in source order.
- `RelatedCluster`: renders title, `<n> artifacts · <updated>`, and a `<Link>`
  whose `href` resolves to `/lifecycle/<slug>` (against the newly-registered
  route); pluralisation (`1 artifact` vs `3 artifacts`). Use the router/query
  test wrappers in `src/test/`.
- `useDocCluster`: returns the cluster whose `entries` contains the doc by
  `path`; returns `cluster: null` while the query is in flight — drive an
  **enabled** query (entry provided) with a **never-resolving** promise
  (`new Promise(() => {})`) and assert `cluster: null` synchronously, so the
  loading case cannot pass via the disabled-idle path (where `isPending` is true
  but `fetchStatus` is `idle`); returns `cluster: null` with `isError: true`
  when `fetchLifecycleClusters` rejects; returns `cluster: null` (settled) when
  the cluster list contains no such cluster — the genuine negative case. Prefer a
  cluster list deliberately **missing the doc's `path`** (exercises the hook's
  negative branch directly); a no-slug fixture must be a lifecycle-participating
  type (`Plan` with `slug == null`), since a slug-less note/decision still
  buckets (per Current State Analysis).
- `LibraryDocView`:
  - with a cluster list containing the doc, a `Cluster` section renders after
    `File` (assert DOM order of the three `<h3>`s);
  - while the cluster query is **pending**, the `Cluster` `<h3>` + `Loading…`
    render (the section does not vanish);
  - when `fetchLifecycleClusters` **rejects**, the `Cluster` `<h3>` + a
    `role="alert"` error render (mirrors the related-artifacts error test);
  - only when the query has **settled with no match** does no `Cluster` section
    render — anchor the assertion on a settled signal so it cannot pass spuriously
    against the still-loading state (with `fetchLifecycleClusters` resolving `[]`
    there is no `Cluster` heading to await): `await waitFor` until the cluster
    query is non-pending (e.g. the `Loading…` text under the would-be section is
    gone / `File` is present), *then* assert
    `queryByRole('heading', { name: 'Cluster' })` is null.

### Success Criteria

#### Automated Verification

- [x] New `RelatedCluster` + `useDocCluster` tests fail before implementation,
  pass after: `mise run test:unit:frontend`
- [x] `useDocCluster` loading and error cases pass; `pluralise` direct unit
  tests pass: `mise run test:unit:frontend`
- [x] `LibraryDocView` DOM-order, loading, error, and settled-no-match
  presence/absence tests pass: `mise run test:unit:frontend`
- [x] Frontend unit suite passes: `mise run test:unit:frontend`
- [x] Typecheck passes (frontend typecheck task)

#### Manual Verification

- [ ] A document in a lifecycle cluster shows a `Cluster` block after `File`
  with the cluster title and `<n> artifacts · <updated>`; clicking navigates to
  `/lifecycle/<slug>`. (Note: because nearly every slug-bearing doc is in at
  least a singleton cluster, this block appears on almost every detail page —
  often as a self-only `1 artifact` cluster, per the render-on-any-membership
  decision.)
- [ ] On a cold cache (no prior lifecycle-index visit), the `Cluster` heading
  shows `Loading…` briefly rather than the section being absent, then resolves.
- [ ] A document in **no** cluster (a `slug == null` entry — not a slug-bearing
  orphan, which still clusters) shows no `Cluster` block.
- [ ] Count matches the lifecycle index card for the same cluster.

---

## Phase 3: Eyebrow Unification

### Overview

Bring `.aside h3` and the inactive **panel-variant** `Pipeline .label` onto the
canonical page-eyebrow typography so all three resolve identically for the six
properties (the five typographic ones plus `font-weight`). Use shared tokens
(adopt the dormant `--tracking-caps`). The `card`-variant rail labels (lifecycle
index) are deliberately left unchanged — the rule is scoped, not global.

### Changes Required

#### 1. Adopt the shared tracking token on the canonical rule

**File**: `src/components/Page/Page.module.css`
**Changes**: Change `.eyebrow` `letter-spacing: 0.12em` → `letter-spacing:
var(--tracking-caps)` (`:36`) so tracking has a single source. (Value
unchanged; resolves to the same `1.32px` at 11px.)

> **Tradeoff (accepted)**: this unifies the eyebrow recipe by *shared token*, not
> by a single shared *rule* — three component-scoped CSS modules still each
> restate the full recipe. Extracting one `:global(.ac-eyebrow)` class was
> considered but rejected (the rules live in CSS modules that can't cheaply share
> a class, and the surfaces differ in margin/active-override). The new
> cross-element identity spec (§4) is the guard against the three rules drifting.

#### 2. Promote the aside section labels

**File**: `src/routes/library/LibraryDocView.module.css`
**Changes**: Update `.aside h3` (`:10`) to the eyebrow treatment:
`font-family: var(--ac-font-mono)`, `font-size: var(--size-eyebrow)`,
`letter-spacing: var(--tracking-caps)`, keep `text-transform: uppercase` and
`color: var(--ac-fg-faint)` (already correct). **Remove the explicit
`font-weight: 600`** so the label inherits the same weight as `.eyebrow`
(`Page.module.css:30-40` declares no weight → inherits). This is a definite
decision, not an either/or: the two genuinely diverge today, so the explicit
weight must go for the six-property parity to hold. Keep `margin`.

#### 3. Promote the rail label

**Files**: `src/components/Pipeline/Pipeline.module.css`,
`src/components/Pipeline/Pipeline.tsx`
**Stable selector hook (Pipeline.tsx)**: the label span currently carries only
`className={styles.label}` (`Pipeline.tsx:64`) — a hashed module class the
visual-regression spec cannot select. Add a global BEM class alongside it
(`className={`${styles.label} ac-stagechain__label`}`), matching the existing
`ac-stagechain` / `ac-stagechain__stage` hooks on the chain and stage
(`Pipeline.tsx:27,52`). The Phase 3 §4 spec targets the rail label via this
global class + the `data-variant`/`data-active` attributes, never the module
classes.

**Changes**: Apply the eyebrow treatment **scoped to the panel variant only** —
add a new rule `.chain[data-variant='panel'] .label` (matching the existing
variant pattern at `:32-40`) with `font-size: var(--size-eyebrow)`,
`letter-spacing: var(--tracking-caps)`, `text-transform: uppercase`. The base
`.label` (`:64-73`) keeps `font-family` (mono) and `color` (`--ac-fg-faint`),
which already match, and `font-weight` is undeclared there (inherits, so already
matches the eyebrow). **Do not** change the bare `.label` size/tracking/transform
— that would restyle the `card`-variant stage labels in every lifecycle-index
cluster card (`LifecycleIndex.tsx:118`), which is out of scope. Leave the active
override (`.stage[data-active='true'] .label { color: var(--ac-fg) }`, `:75-77`)
untouched. **Add a CSS comment on the base `.label` rule** noting it is shared
across the `card` and `panel` variants and that the panel-variant eyebrow
overrides live in the scoped rule — so a future editor of the bare rule has an
in-file signal of the variant boundary (the `card` variant has no dedicated
spec to catch a cross-variant regression).

> **Accessibility note**: unlike the decorative page eyebrow, the rail `.label`
> is navigational **content** (the stage name). Uppercasing via `text-transform`
> and widening tracking can affect low-vision legibility and how some assistive
> tech announces the text. The uppercase treatment is intentional for visual
> parity, but verify (Manual Verification) that the stage label still reads as a
> stage name to a screen reader and that the active stage remains distinguishable.

#### 4. Eyebrow-equality visual-regression spec (red first)

**File**: `tests/visual-regression/eyebrow-unification-resolved.spec.ts` (new)
**Changes**: Assert the **six** resolved properties match canonical values across
the three elements (using `setTheme`, `DETAIL_ROUTE_SLUGS`, token helpers from
`lib/expected-colours.ts` / `src/styles/tokens.ts`):

- Page eyebrow text and aside `<h3>` (`Related artifacts`) on a detail page;
  inactive **panel-variant** rail `.label` on `/lifecycle/first-plan` (pin the
  concrete fixture route, as the sibling specs do — `first-plan` is a
  partially-complete cluster, so at least one `data-active='false'` stage label
  exists).
- **Guard against a vacuous pass**: before reading computed styles, assert the
  inactive-label locator matches **≥ 1** element — a zero-match locator would make
  the cross-element equality check pass against nothing. Use the **stable** hooks
  (not hashed module classes):
  `.ac-stagechain[data-variant='panel'] .ac-stagechain__stage[data-active='false'] .ac-stagechain__label`
  (the `ac-stagechain__label` global class is added to the span in §3).
- For each element assert: `fontFamily` contains `Fira Code` (matches the
  eyebrow's resolved stack); `fontSize === '11px'`; `letterSpacing === '1.32px'`;
  `textTransform === 'uppercase'`; **`fontWeight`** equals the eyebrow's resolved
  weight (added to the property set so the parity decision in §2 is actually
  machine-checked, not assumed); `color === hexToRgb(...)` using the
  **theme-specific** token (`LIGHT_COLOR_TOKENS['ac-fg-faint']` /
  `DARK_COLOR_TOKENS['ac-fg-faint']` selected by the current `setTheme` — there
  is no flat `tokens` export; mirror `detail-eyebrow-resolved-colours.spec.ts`).
- Pattern: capture the page-eyebrow's resolved values, then assert the aside
  `<h3>` and the rail `.label` equal them (cross-element identity), in addition
  to the canonical literals, in **both** light and dark themes.
- Note in a comment that `PipelineMini` has no `.label` (clause vacuous), the
  rail check targets an inactive (`data-active='false'`) **panel-variant** stage
  label, and the `card`-variant index labels are intentionally not asserted
  (out of scope, unchanged).

### Success Criteria

#### Automated Verification

- [x] New eyebrow-equality spec fails before the CSS changes, passes after:
  `mise run test:e2e:visualiser`
- [x] Existing typography/eyebrow specs still pass
  (`detail-eyebrow-resolved-colours.spec.ts`,
  `typography-resolved-sizes.spec.ts`): `mise run test:e2e:visualiser`
- [x] Frontend unit suite + typecheck pass: `mise run test:unit:frontend`

#### Manual Verification

- [ ] Page eyebrow, aside section labels, and (inactive) panel-rail labels are
  visually identical in weight/size/tracking/case/colour, light and dark.
- [ ] The active rail stage label still reads brighter (`--ac-fg`) than its
  siblings.
- [ ] Lifecycle-index cluster-card (`card`-variant) stage labels are
  **unchanged** (still mixed-case `--size-4xs`) — the panel-scoping held.
- [ ] The uppercased panel rail labels still read as stage names to a screen
  reader (not spelled out / mis-announced), and remain legible at low vision.
- [ ] WCAG 1.4.3 contrast (≥ 4.5:1) is checked for **all** newly-faint or
  relocated `--ac-fg-faint` surfaces — not just the `(inferred)` tag (Phase 1):
  the recoloured aside `<h3>` section labels, the new `Cluster` heading + its
  `<n> artifacts · <updated>` meta row, and the panel rail label — in both
  themes. (The page eyebrow leans on the 1.4.3 incidental-text exception; these
  are content labels, so the exception is weaker.)
- [ ] No regression to the eyebrow icon/size (0074 / 0075 surfaces).

---

## Testing Strategy

### Unit Tests

- `RelatedArtifacts`: single list, per-row `(declared)`/`(inferred)` tags,
  declared-before-inferred order, empty-state from combined rows, no legend, no
  level-4 headings, links resolve.
- `pluralise` (`format.test.ts`): `n = 0/1/>1` and explicit-`plural?` override.
- `RelatedCluster`: title + `<n> artifacts · <updated>` + `/lifecycle/<slug>`
  link (against the test-router lifecycle route); pluralisation edge
  (`1 artifact`).
- `useDocCluster`: path-match returns the cluster; loading returns
  `cluster: null` + `isPending`; error returns `cluster: null` + `isError`;
  settled no-match returns `cluster: null` (negative case via `slug == null`).
- `LibraryDocView`: three aside sections in `Related artifacts` → `File` →
  `Cluster` order; Cluster section shows `Loading…` while pending and a
  `role="alert"` on error; section absent only when settled with no match.

### Integration / Visual-Regression Tests

- `aside-row-resolved-colours.spec.ts` retargeted to the single list (no
  dashed-border test; row-icon colours + container invariance retained;
  coverage-guard count filtered to `(inferred)` rows).
- `eyebrow-unification-resolved.spec.ts` (new): six-property equality (incl.
  `fontWeight`) across the page eyebrow, aside `<h3>`, and inactive
  panel-variant rail `.label`, light + dark; locator-count guard against a
  vacuous pass; card-variant labels not asserted (out of scope).

### Manual Testing Steps

1. Open one detail page per physical doc type; confirm the Option B aside
   (single list, tags, no legend/borders) and the eyebrow parity.
2. Open a detail page for a document in a multi-artifact cluster; confirm the
   `Cluster` block and that it navigates to the correct `/lifecycle/<slug>`. In a
   fresh tab (cold cache), confirm the heading shows `Loading…` then resolves
   rather than the section being absent.
3. Open a detail page for a `slug == null` document (the only true no-cluster
   case — a slug-bearing orphan still clusters as a singleton); confirm no
   `Cluster` block.
4. Open `/lifecycle/<slug>`; confirm the panel rail labels are uppercase 11px
   eyebrow styling and the active stage still brightens. Cross-check a
   lifecycle-index cluster card and confirm its stage labels are *unchanged*.

## Performance Considerations

`useDocCluster` fetches the full cluster list (`/api/lifecycle`) on the detail
page. It shares the `queryKeys.lifecycle()` cache with `LifecycleIndex`, so a
prior visit to the lifecycle index serves it from cache; the dataset is the
local meta directory (small). Acceptable; no pagination needed.

Two deliberate tradeoffs are accepted at this scale:

- **Cold-cache payload**: `/api/lifecycle` is the heaviest list endpoint (it
  embeds every cluster's full `entries: Vec<IndexEntry>` with frontmatter), so a
  direct detail-page load with a cold cache pulls the whole cluster corpus to
  surface one small block. This is traded for robust path-matching and cache
  sharing; if the corpus ever grows large, the escape hatch is a targeted
  single-cluster fetch (`/api/lifecycle/<slug>`) once the slug is resolvable.
- **SSE refetch frequency**: once mounted, the detail page becomes an observer of
  `queryKeys.lifecycle()`, so every `doc-changed`/`doc-invalid` event
  (`use-doc-events.ts`) now refetches the full list while the user sits on one
  document. Fine for a single-user local tool; noted so it is a known cost, not a
  surprise. The membership derivation itself is memoised (Phase 2 §2), so
  re-renders that are not driven by new data do not re-scan.

## Migration Notes

None. No persisted data, schema, or API contract changes. The
`queryKeys.lifecycle()` cache is already `v2`-namespaced.

## Coordination Notes

- 0040 (Pipeline Visualisation Overhaul) owns `Pipeline .label`; it is
  functionally complete. Confirm 0040's status is transitioned before 0079
  closes; treat any reactivation as a renewed concurrent-edit coupling on the
  rail labels.
- 0074 / 0075 touch the eyebrow **icon/size**; this work touches eyebrow
  **font-family / tracking / transform / colour**. Land this rule consolidation
  first; mirror the ordering note onto 0074 / 0075.

## References

- Original work item: `meta/work/0079-aside-region-redesign.md`
- Research: `meta/research/codebase/2026-06-05-0079-aside-region-redesign.md`
- Prototype structure: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
  (`.ac-related` list + `.ac-related__tag.is-declared`)
- Component: `src/components/RelatedArtifacts/RelatedArtifacts.tsx:18-105`,
  `RelatedArtifacts.module.css:1-55`
- Detail view: `src/routes/library/LibraryDocView.tsx:104-126,159`,
  `LibraryDocView.module.css:9-11`
- Cluster card pattern: `src/routes/lifecycle/LifecycleIndex.tsx:53-55,92-123`
- Cluster fetch/types: `src/api/fetch.ts:148-153`, `src/api/types.ts:209-218`,
  `src/api/format.ts:19`
- Eyebrow rules: `src/components/Page/Page.module.css:30-40`,
  `src/components/Pipeline/Pipeline.module.css:64-77`,
  `src/styles/global.css` (`--tracking-caps`, `--size-eyebrow`, `--ac-fg-faint`)
- Server (no changes): `server/src/related.rs:22-88`,
  `server/src/clusters.rs` (representative slug, no cluster filtering)
