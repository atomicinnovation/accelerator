---
type: codebase-research
id: "2026-06-12-0087-error-screen-affordances"
title: "Research: 404 / Error Screen with Affordances (work item 0087)"
date: "2026-06-12T20:20:18+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0087"
parent: "work-item:0087"
relates_to: ["work-item:0041", "work-item:0082", "work-item:0074", "work-item:0054"]
topic: "404 / Error Screen with Affordances"
tags: [research, codebase, frontend, error-states, routing, search, page-shell, big-glyph, suggestions]
revision: "c0a447de2b29473893289377f9e8a499b3fa37e6"
repository: "build-system"
last_updated: "2026-06-12T20:20:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: 404 / Error Screen with Affordances (work item 0087)

**Date**: 2026-06-12T20:20:18+00:00
**Author**: Toby Clemson
**Git Commit**: c0a447de2b29473893289377f9e8a499b3fa37e6
**Branch**: HEAD (detached; jj workspace `build-system`)
**Repository**: build-system

## Research Question

For work item 0087 ("404 / Error Screen with Affordances"), establish the live
state of every integration point the story names: the inline not-found / fetch-
error branches in `LibraryDocView`, the router's lack of a `notFoundComponent`,
the `Page` shell API (0041), the client-side slug-aggregation precedent for the
"Did you mean…" suggestion engine, the authoritative search-ranking convention
(`classify()`) to mirror, the shipped BigGlyph hero (0082) and per-doc-type hue
tints (0074), and the prototype's reusable layout/copy-voice building blocks.

## Summary

The work item's description is **accurate and well-researched** — almost every
claim it makes was confirmed against live code, with only small refinements
needed. Concretely:

- **No dedicated 404 surface exists.** "Document not found" is rendered inline
  in `LibraryDocView.tsx` across three branches (one true 404, two fetch
  errors), all reusing the same `Document not found` H1. There is **no
  `notFoundComponent`** on `createRouter` — confirmed.
- **Unknown doc *types* never reach the detail view.** The `/library/$type`
  route's `parseParams` `throw redirect({ to: "/library" })`s on a type that
  fails `isDocTypeKey`, so the back-to-type affordance only matters when the
  type is valid but the slug is not — exactly as the story states.
- **The `Page` shell (0041) is shipped** and is the established wrapper, but it
  has **no glyph or hue prop** — those are composed into the `eyebrow`/`children`
  `ReactNode` slots by each caller. The closest precedent for a not-found surface
  is `LibraryDocView`'s own current usage, which already drives a `Document not
  found` title through `Page`.
- **The slug-aggregation precedent is the wiki-link resolver**, but with a
  caveat: it fires a *fixed pair* of `useQuery` hooks (only `decisions` +
  `work-items`), not a `DOC_TYPE_KEYS` loop. Aggregating across all 13 keys is a
  generalisation the story introduces; the reusable parts are `fetchDocs(type)`,
  the `queryKeys.docs(type)` cache key, and the `entry.slug ?? fileSlugFrom­RelPath(entry.relPath)`
  link convention. **No fuzzy matching exists anywhere** — confirmed.
- **The authoritative ranking is `classify()` in `server/src/api/search.rs`**,
  with a 4-variant `Bucket` enum (`ExactSlug=0, Prefix=1, Interior=2, Body=3`)
  and a per-bucket `sort_by_cached_key((Reverse(mtime_ms), rel_path))`. Matching
  is ASCII-case-insensitive. The frontend trusts the server order and does not
  re-sort. The prototype's `rankCorpus` is an illustrative JS equivalent.
- **BigGlyph (0082) and per-doc-type tints (0074) are both shipped and `done`.**
  BigGlyph degrades gracefully to `DefaultBigGlyph` (neutral hue 215) for an
  unknown/absent type — ideal for the router-level catch-all. **Watch out:** the
  *small* `Glyph` returns `null` for an unknown type rather than a neutral
  fallback, so a tinted eyebrow/row icon would silently vanish on the catch-all.
- **The prototype has no 404 screen** but ships every building block: the
  `.ac-empty-page` hero+illustration layout, the `.ac-search__empty` microcopy,
  the `.ac-search__row` link pattern, `.ac-topbar__btn`, and a sentence-case,
  no-apology, mono-quoted-query copy voice. The one genuine gap is a
  CTA-button-with-recovery-action — no filled/primary button primitive exists.

**Critical path note:** the app does **not** live at repo-root `frontend/`. All
TypeScript/Rust source is under `skills/visualisation/visualise/`. Every
`frontend/src/...` path in the work item resolves to
`skills/visualisation/visualise/frontend/src/...`. The same applies to `server/`.

## Detailed Findings

### 1. Current not-found / fetch-error handling (`LibraryDocView.tsx`)

File: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`

The three branches the work item describes are **confirmed**, evaluated in this
order (the story lists them differently; the live order is list-error →
content-error → unmatched-slug):

1. **Branch (b) — doc-list fetch errored** (`LibraryDocView.tsx:95-102`),
   checked first: `if (listError) { title = "Document not found"; body = <p
   role="alert" className={styles.error}>Failed to load document list: …</p> }`.
2. **Branch (c) — doc-content fetch errored** (`LibraryDocView.tsx:103-112`):
   `else if (content.isError) { title = "Document not found"; body = <p
   role="alert" …>Failed to load document content: …</p> }`.
3. **Branch (a) — unknown slug under a valid type** (`LibraryDocView.tsx:113-115`):
   `else if (!entry && entries.length > 0) { title = "Document not found"; body
   = <p>Document not found.</p> }`. The success branch is `else if (entry &&
   content.data)` (line 116).

**Refinement to the story:** branch (a) is gated on `entries.length > 0`. If the
list is still loading (default `[]`) or genuinely empty, this branch does *not*
fire — the page falls through to the `Loading…` default (`LibraryDocView.tsx:91-93`).
So branch (a) only triggers for a genuinely unmatched slug under a type with at
least one document.

Data flow:
- Params: `useParams({ strict: false })` (`LibraryDocView.tsx:43-46`);
  `rawType` narrowed via `isDocTypeKey` to `DocTypeKey | undefined`
  (`LibraryDocView.tsx:50-51`); `fileSlug = propSlug ?? params.fileSlug ?? ""`.
- Doc-list query: inline `useQuery` → `fetchDocs(type!)`, keyed
  `queryKeys.docs(type)`, `enabled: type !== undefined` (`LibraryDocView.tsx:53-62`).
- Entry match (`LibraryDocView.tsx:64-66`):
  `entries.find((e) => e.slug === fileSlug || fileSlugFromRelPath(e.relPath) === fileSlug)`
  — accepts *either* the slug or the relPath-derived stem.
- Doc content via `useDocPageData(entry?.relPath)` (`use-doc-page-data.ts:8-12`),
  gated on `entry?.relPath` so it stays idle until an entry matches.

Two earlier guard returns sit *outside* `<Page>` (`LibraryDocView.tsx:84-89`):
`if (type === undefined) return <p role="alert">Unknown doc type: …</p>` and
`if (!fileSlug) return <p role="alert">Missing file slug.</p>`. The first is
effectively dead code under normal routing because the router redirects unknown
types first (see §2). On all not-found branches `hasResolvedDocument` is false
(`LibraryDocView.tsx:211`), so `Page` renders with no eyebrow and no actions.

### 2. Router: no `notFoundComponent`; unknown-type redirect (`router.ts`)

File: `skills/visualisation/visualise/frontend/src/router.ts`

- **No `notFoundComponent`** — confirmed. `createRouter({ routeTree })`
  (`router.ts:218`) is configured with *only* `routeTree`. No
  `defaultNotFoundComponent`, `defaultErrorComponent`, or
  `defaultPendingComponent`, and no per-route `notFoundComponent`. Unmatched
  routes fall to TanStack Router's built-in default.
- **Route tree** (`router.ts:199-216`): `libraryRoute` (`/library`,
  `LibraryLayout`) → `libraryIndexRoute` (`LibraryOverviewHub`), templates routes
  (registered first for literal-path specificity), and `libraryTypeRoute`
  (`/$type`, `LibraryTypeView`) → `libraryDocRoute` (`/$fileSlug`,
  `LibraryDocView`). Full detail path: `/library/$type/$fileSlug`.
- **Unknown-type redirect** (`router.ts:110-115`), on `libraryTypeRoute` only:
  ```ts
  parseParams: (raw): { type: DocTypeKey } => {
    if (!isDocTypeKey(raw.type)) { throw redirect({ to: "/library" }); }
    return { type: raw.type };
  }
  ```
  Because `libraryDocRoute` is a *child*, this runs for the detail route too, so
  `/library/<unknown-type>/<slug>` redirects before `LibraryDocView` mounts.
  `libraryDocRoute` has **no `parseParams`** — `$fileSlug` is never validated at
  the router boundary, which is why an unknown slug reaches branch (a).
- The index route `beforeLoad` redirects `/` → `/library` (`router.ts:67-73`).
  `withCrumb` (`router.ts:28-45`) wraps routes to inject breadcrumb loaders.

### 3. The `Page` shell (work item 0041) — shipped, no glyph/hue prop

File: `skills/visualisation/visualise/frontend/src/components/Page/Page.tsx`
(no `index.ts`; importers use `components/Page/Page` directly).

Prop interface (`Page.tsx:4-11`):
```ts
export interface PageProps {
  eyebrow?: ReactNode;
  title: ReactNode;          // the H1; ReactNode so callers pass rich nodes
  subtitle?: ReactNode;
  actions?: ReactNode;       // right-aligned, bottom-aligned to the title
  maxWidth?: "default" | "narrow";
  children: ReactNode;
}
```
- The H1 is `<h1 className={styles.title}>{title}</h1>` (`Page.tsx:33`), fully
  customisable per-usage — so `Document not found` vs `Page not found` vs
  `Couldn't load this document` are all just different `title` values.
- The `<header className={styles.header}>` is the `.ac-pagehead` equivalent
  (CSS comment, `Page.module.css:15-16`).
- **No `glyph`/`hero`/`tint`/`hue` prop.** Per-doc-type glyph + colour are
  composed into the `eyebrow` node by callers — e.g. `LibraryDocView.tsx:215`
  passes `eyebrow={<EyebrowLabel type={type} />}`, and `EyebrowLabel` renders a
  framed `<Glyph docType={type} size={16} framed />`. A 404 surface needing a
  hero glyph composes `BigGlyph` into `children` (the empty-state precedent),
  not into `Page`.
- Shipped/complete: work item `0041` is `status: done`
  (`meta/work/0041-library-page-wrapper-and-overview-hub.md:7`); `Page.test.tsx`
  covers H1, slots, divider, max-width/padding tokens.
- **Two spec deviations** worth noting: horizontal padding is `--sp-7` (not
  `--sp-6` as the 0041 spec text said; `Page.module.css:8`, asserted in
  `Page.test.tsx:86-88`), and there is intentionally no glyph/hue prop.

Closest precedent for the new surface: `LibraryDocView.tsx:213-229`, which
already drives a `Document not found` title + error `<p role="alert">` body
through `Page` with eyebrow/actions suppressed.

### 4. Client-side slug aggregation for "Did you mean…" suggestions

Types (`skills/visualisation/visualise/frontend/src/api/types.ts`):
- `IndexEntry` (`types.ts:116-148`), camelCase fields. Key ones: `slug: string |
  null` (`types.ts:120` — **nullable**, must be handled), `mtimeMs: number`
  (`types.ts:128` — the field is `mtimeMs`, *not* `mtime`/`mtime_ms`), `relPath:
  string` (`types.ts:119`), `type: DocTypeKey`.
- `DOC_TYPE_KEYS` (`types.ts:23-37`) — 13 keys: `decisions, work-items, plans,
  research, plan-reviews, pr-reviews, work-item-reviews, validations, notes,
  pr-descriptions, design-gaps, design-inventories, templates`.
- `isDocTypeKey` (`types.ts:40-42`) — membership check, the same predicate the
  router uses.
- `VIRTUAL_DOC_TYPE_KEYS = ["templates"]` (`types.ts:48-50`) and
  `isPhysicalDocTypeKey` (`types.ts:59-63`) — `templates` is virtual; decide
  whether to include it in aggregation. Note the server's `classify()` **skips
  Templates entirely** (`search.rs:114`), so excluding it keeps parity.
- `DOC_TYPE_LABELS` / `DOC_TYPE_LABELS_SINGULAR` (`types.ts:68-101`) — usable in
  suggestion UI without a query.

Fetch layer (`skills/visualisation/visualise/frontend/src/api/fetch.ts`):
- `fetchDocs(type): Promise<IndexEntry[]>` (`fetch.ts:108-114`) — one HTTP call
  per type (`GET /api/docs?type=…`); throws `FetchError` (carrying `.status`) on
  non-ok. There is **no batch/all-types endpoint** — confirms the story's
  premise.
- Cache key `queryKeys.docs(type)` → `["docs", type]` (`query-keys.ts:50`).
  Aggregating across all types reuses cache entries already populated by library
  views and the wiki-link resolver.

The precedent — wiki-link resolver
(`skills/visualisation/visualise/frontend/src/api/use-wiki-link-resolver.ts`,
`wiki-links.ts`):
- It fires a **fixed pair** of `useQuery` hooks (`decisions` + `work-items`,
  `use-wiki-link-resolver.ts:47-59`) — **not** a `DOC_TYPE_KEYS.map` and **not**
  `useQueries`. Aggregating over all 13 keys is a generalisation 0087 introduces;
  `useQueries` is the idiomatic way to fan a dynamic key list, but there is no
  existing `useQueries` usage to copy.
- Index build is memoised (`use-wiki-link-resolver.ts:73-76`) into `Map`-based
  **exact-match** indexes (`buildWikiLinkIndex`, `wiki-links.ts:132-151`);
  resolution is a single `Map.get` (`resolveWikiLink`, `wiki-links.ts:173-192`).
  **No Levenshtein/trigram/prefix logic anywhere** — fuzzy ranking is net-new.
- Warming semantics use `isPending` (not `isFetching`) to avoid flicker on
  background refetch (`use-wiki-link-resolver.ts:34-45, 61`).

Detail-route link convention (the canonical form):
- `LibraryTypeView.tsx:265-271`: `<Link to="/library/$type/$fileSlug"
  params={{ type, fileSlug: entry.slug ?? fileSlugFromRelPath(entry.relPath) }}>`.
- `fileSlugFromRelPath` (`path-utils.ts:6-8`) = last path segment minus `.md`.
- Reverse resolution accepts either form (`use-active-doc-relpath.ts:33-35`,
  mirrored in `LibraryDocView.tsx:65`), so the suggestion candidate string to
  match against and link with is `entry.slug ?? fileSlugFromRelPath(entry.relPath)`.

### 5. Authoritative ranking convention (`classify()` in `search.rs`)

File: `skills/visualisation/visualise/server/src/api/search.rs`

- **Bucket enum** (`search.rs:41-48`), discriminants ARE the rank order:
  ```rust
  #[repr(u8)] enum Bucket { ExactSlug = 0, Prefix = 1, Interior = 2, Body = 3 }
  ```
- **`classify()`** (`search.rs:56-80`), first-match-wins top-down:
  ExactSlug (`slug == q`, `:60-64`) → Prefix (`title.starts_with(q) ||
  slug.starts_with(q)`, `:65-69`) → Interior (`title.contains(q) ||
  slug.contains(q)`, `:70-74`) → Body (`body_preview.contains(q)`, `:75-78`) →
  `None` (`:79`). Prefix uses `starts_with`, Interior uses `contains` — exactly
  as the story states.
- **Case-insensitive** via ASCII lowercasing: query lowercased once
  (`search.rs:108`), fields lowercased per-comparison (`:57-58, :75`). It is
  `to_ascii_lowercase` — non-ASCII is *not* case-folded. Client-side, plain
  `.toLowerCase()` is close enough (minor non-ASCII divergence).
- **Within-bucket sort** (`search.rs:122-129`):
  `bucket.sort_by_cached_key(|e| (std::cmp::Reverse(e.mtime_ms),
  e.rel_path.to_string_lossy().into_owned()))` — primary key `mtime_ms`
  **descending** (newest first), stable tiebreak `rel_path` **ascending**.
  Confirmed, semantically identical to the story's quote.
- Buckets flattened in order 0→3 (`search.rs:131-135`); **Templates skipped**
  (`search.rs:114`); entries without a slug dropped in `project()`
  (`search.rs:85-93`). No `>= 2` minimum on the server (that gate is
  client-side).

For 0087, only the **slug-relevant** buckets matter (Prefix, Interior). An
ExactSlug match cannot occur on a 404 surface (an exact slug is a *found*
document). Body matching is out of scope. So the client engine reduces to:
prefix-slug bucket above interior-slug bucket, both case-insensitive, then
`mtimeMs` desc, then `relPath` asc.

Frontend hook (`skills/visualisation/visualise/frontend/src/api/use-search.ts`):
- `>= 2` minimum gate (`use-search.ts:7-8`): `const enabled =
  debounced.length >= 2`, applied to the trimmed value after a 200ms debounce.
  This is the minimum the story mirrors.
- Result type `SearchResult` (`fetch.ts:206-211`): `{ docType, title, slug,
  mtimeMs }` — **no `relPath`**. The client trusts server order and does not
  re-sort. **Implication for 0087:** since the suggestion engine sorts
  client-side over `IndexEntry` data, `relPath` *is* available there (it is on
  `IndexEntry`), so the `relPath`-ascending stable tiebreak is implementable.

Result-row + empty-state markup (note: real DOM classes are CSS Modules, e.g.
`styles.searchRowLink`, not literal `.ac-search__*`):
- Row pattern (`SearchResultsPanel.tsx:64-95`,
  `skills/visualisation/visualise/frontend/src/components/Sidebar/`): a TanStack
  `<Link to="/library/$type/$fileSlug" params={{ type: r.docType, fileSlug:
  r.slug }}>` with `role="option"`, leading `<Glyph>`, body (title + mono
  `docType/slug` sub-line), trailing chevron. Reuse this `to`/`params` shape for
  suggestion links.
- "No matches" microcopy (`SearchResultsPanel.tsx:98-107`): container
  `role="status" aria-live="polite"`, a `No matches` title, and a body echoing
  the query in a mono span with concrete recovery hints — the voice to mirror.
- `NoResultsPanel.tsx`
  (`skills/visualisation/visualise/frontend/src/routes/library/`) is a *separate*
  filter-state empty component (Clear-filter button), not the search-empty
  pattern — don't conflate.

### 6. BigGlyph hero (0082) and per-doc-type tints (0074) — both shipped

BigGlyph (`skills/visualisation/visualise/frontend/src/components/BigGlyph/BigGlyph.tsx`):
- Props (`BigGlyph.tsx:48-59`): `docType: DocTypeKey`, `size?: number` (default
  96), `hue?: number` (numeric HSL override, resolved with `??` so `hue={0}` is
  honoured, `:75`). Always `aria-hidden="true"` (decorative, `:88`).
- **Unknown/absent fallback:** `const draw = BIG_GLYPHS[docType] ??
  DefaultBigGlyph` (`BigGlyph.tsx:76`); `DefaultBigGlyph` is a rotated paper
  sheet (`icons/DefaultBigGlyph.tsx:7-47`); off-union hue falls back to
  `DEFAULT_BIG_HUE = 215` (neutral blue, `:46`). **This graceful degradation is
  exactly what the router-level catch-all needs** (no doc type → DefaultBigGlyph
  at hue 215). DEV builds warn on unknown type (`:77-82`).
- `BIG_GLYPHS: Record<DocTypeKey, BigGlyphDraw>` (`BigGlyph.tsx:27-41`) maps all
  13 keys to draw *functions* `(palette) => ReactElement` invoked as
  `draw(bigPalette(hue))` (`:91`). Palette = 6 hue-derived tones
  (`bigPalette.ts:27-37`).
- Current usage: `EmptyState.tsx:32` (`<BigGlyph docType={docType} size={96} />`
  — the primary production hero, the closest analogue to a 404 surface) and a
  dev-only showcase route.

Per-doc-type tints (0074) — two parallel representations:
- **Numeric hue** `DOC_TYPE_HUE: Record<DocTypeKey, number>` (`styles/tokens.ts:11-25`)
  feeds BigGlyph + the empty-page gradient (`--ac-empty-page-hue`).
- **CSS-var tints** `--ac-doc-<key>` declared in `styles/global.css` (light
  `~107+`, dark `~366+` where all collapse to white), mirrored as hex in
  `tokens.ts:61-87/132-158`. Opt-in via the small `Glyph` component:
  `DOC_TYPE_COLOR_VAR` / `DOC_TYPE_TOKEN_KEY` (`Glyph.constants.ts:12-44`), applied
  as `style={{ color: var(--ac-doc-<key>) }}` (`Glyph.tsx:90, 115, 135`).
  Detail-page consumers: `EyebrowLabel.tsx:13`, `RelatedArtifacts.tsx:116, 123-126`.
- Fallbacks: `templates` → `ac-fg-muted` (neutral, `Glyph.constants.ts:25`).
  **Caveat:** the small `Glyph` returns `null` for a docType not in
  `ICON_COMPONENTS` (`Glyph.tsx:81-89`) — a tinted eyebrow/row icon would
  *silently disappear* on the catch-all (no valid type), unlike BigGlyph which
  renders DefaultBigGlyph. Design the catch-all eyebrow to avoid a bare `Glyph`
  with an absent type.
- Both shipped: `meta/work/0082-…:8` and `meta/work/0074-…:8` are `status: done`;
  a 26-combination visual-regression spec exists for the showcase.

### 7. Prototype building blocks and copy voice

Source: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/`.
The prototype has **no 404 screen** (unknown hashes coerced to last route), but
ships every reusable block:

- **Hero+illustration layout** — `LibraryIndexEmpty` (`view-empty.jsx:72-106`):
  `.ac-page` > `.ac-pagehead` (eyebrow + h1 + sub) > `.ac-empty-page` two-column
  grid (`96px 1fr`) of `BigGlyph` hero | body (mono eyebrow + title + lede +
  dashed-bordered foot). Hue injected via inline `--ac-empty-page-hue`. CSS at
  `app.css:1086-1158`: dashed `1px dashed var(--ac-stroke-strong)` border, hue-
  tinted radial gradient, single-column collapse at 820px. **No CTA button** in
  this layout — recovery is purely textual.
- **`.ac-search__empty` microcopy** (`search.jsx:252-259`): `role="status"`,
  `No matches` title, body echoing the failed query inside a mono span wrapped
  in literal quotes (`"{settled}"`) + concrete recovery hints. CSS
  `app.css:440-456`.
- **`.ac-search__row` suggestion-link pattern** (`search.jsx:225-246`): a real
  `<a href>` (native middle/cmd-click) with `onClick` intercepting plain clicks;
  three-column grid `26px 1fr 12px` (glyph | body | chevron). CSS
  `app.css:392-431`.
- **`rankCorpus`** (`search.jsx:52-80`) — the illustrative JS equivalent of
  `classify()`: 2-char minimum, case-insensitive substring on title/slug/id,
  prefix beats anywhere, ties broken `mtime_ms` desc then `path` asc, capped at
  40. **Not a second source of truth** — the server's `classify()` is
  authoritative.
- **`.ac-topbar__btn`** (`app-shell.jsx:165-167`, `app.css:143-144`): the only
  button primitive — ghost/outline-on-hover, `gap: 6px` supports icon+label.
  **No filled/primary button exists** — a recovery-CTA is net-new design.
- **`DEFAULT_BIG`** (`big-glyphs.jsx:408-415`) — the prototype's fallback glyph,
  selected via `BIG_GLYPHS[type] || DEFAULT_BIG`; matches the shipped
  `DefaultBigGlyph`.
- **Copy voice** (evidenced): sentence case + terminal period (`No research
  yet.`, `view-empty.jsx:97`); failed/missing query in a mono span (`Nothing in
  meta/ matches "{settled}".`, `search.jsx:256`); no apologies/blame; concrete
  recovery hints with literal examples (`Try a slug, a doc id (e.g. PROJ-0007),
  or a fragment of a title.`); em-dash asides, `…` for in-progress, `·` metadata
  separators.

## Code References

- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:84-116` — the two guard returns + three not-found/error branches to split and replace
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:64-66` — entry-match (slug OR relPath stem)
- `skills/visualisation/visualise/frontend/src/router.ts:218` — `createRouter({ routeTree })`, no `notFoundComponent`
- `skills/visualisation/visualise/frontend/src/router.ts:110-115` — `parseParams` unknown-type redirect to `/library`
- `skills/visualisation/visualise/frontend/src/components/Page/Page.tsx:4-11` — `PageProps` (no glyph/hue prop)
- `skills/visualisation/visualise/frontend/src/api/types.ts:116-148` — `IndexEntry` (`slug` nullable, `mtimeMs`, `relPath`)
- `skills/visualisation/visualise/frontend/src/api/types.ts:23-42` — `DOC_TYPE_KEYS`, `isDocTypeKey`
- `skills/visualisation/visualise/frontend/src/api/fetch.ts:108-114` — `fetchDocs(type): Promise<IndexEntry[]>`
- `skills/visualisation/visualise/frontend/src/api/use-wiki-link-resolver.ts:47-76` — fixed-pair `useQuery` + memoised index precedent
- `skills/visualisation/visualise/frontend/src/api/wiki-links.ts:132-192` — exact-match index build/resolve (no fuzzy)
- `skills/visualisation/visualise/frontend/src/api/path-utils.ts:6-8` — `fileSlugFromRelPath`
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx:265-271` — canonical detail-route `<Link>` (`slug ?? fileSlugFromRelPath`)
- `skills/visualisation/visualise/server/src/api/search.rs:41-80` — `Bucket` enum + `classify()`
- `skills/visualisation/visualise/server/src/api/search.rs:122-129` — `sort_by_cached_key((Reverse(mtime_ms), rel_path))`
- `skills/visualisation/visualise/frontend/src/api/use-search.ts:7-8` — `length >= 2` gate
- `skills/visualisation/visualise/frontend/src/components/Sidebar/SearchResultsPanel.tsx:64-107` — result-row link + "No matches" microcopy
- `skills/visualisation/visualise/frontend/src/components/BigGlyph/BigGlyph.tsx:48-94` — props + `DefaultBigGlyph`/`DEFAULT_BIG_HUE` fallback
- `skills/visualisation/visualise/frontend/src/routes/library/EmptyState.tsx:24-32` — BigGlyph hero + `--ac-empty-page-hue` usage (closest surface precedent)
- `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx:81-90` — small Glyph returns `null` for unknown type (catch-all caveat)

## Architecture Insights

- **The fetch-error / 404 split the story mandates is a genuine correctness
  fix.** Today all three branches share the `Document not found` H1 even though
  two are network/server failures. Splitting them (distinct heading like
  `Couldn't load this document`, no suggestions block) is the right model and
  has no current code resisting it.
- **`Page` is deliberately presentation-only.** It owns chrome, not content
  identity. The new surface should follow the established pattern: compose the
  hero `BigGlyph` into `children` and the eyebrow into `eyebrow`, pass the H1 as
  `title`. This keeps it consistent with `EmptyState`/`LibraryDocView`.
- **Two colour models coexist.** BigGlyph takes a numeric `hue`; small-Glyph
  tints take a CSS-var reference. For the catch-all (no type), use BigGlyph's
  default (hue 215) and avoid a bare small-`Glyph` (it null-renders).
- **The suggestion engine is a *generalisation* of the wiki-link precedent, not
  a reuse.** The precedent only ever needed two types; 0087 needs all 13 (likely
  via `useQueries`). The reusable contracts are `fetchDocs`, `queryKeys.docs`,
  the nullable-slug link convention, and the bucket/tiebreak ranking — not the
  resolver hook itself.
- **Client-side ranking can be fully faithful to the server**, because it sorts
  over `IndexEntry` (which *has* `relPath` and `mtimeMs`), unlike the wire
  `SearchResult` shape which drops `relPath`. So the `relPath`-ascending stable
  tiebreak in the story's worked example is implementable.
- **`templates` is the recurring edge case.** It is virtual, the server's
  `classify()` skips it, and it has a neutral tint. Excluding it from suggestion
  aggregation keeps parity with search.

## Historical Context

- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — the source design-gap that spawned 0087.
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md`
  — explicitly records (line 71) that no 404/error screen exists in the
  prototype; 0087 is net-new design, not a port.
- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md`
  and `…2026-05-16-…-supplementary.md` — 0041 Page-wrapper research.
- `meta/research/codebase/2026-06-09-0082-big-glyph-hero-illustrations.md` — 0082
  BigGlyph internals.
- `meta/research/codebase/2026-05-24-0074-per-doc-type-hues-on-detail-page.md` —
  0074 per-doc-type hues.
- `meta/research/codebase/2026-06-01-0054-sidebar-search.md` — 0054 search
  ranking/relevance.
- `meta/plans/2026-04-28-meta-visualiser-phase-10-error-handling-accessibility-polish.md`
  — the most relevant prior error-handling plan.
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` and
  `ADR-0035-brand-layer-indirection-supplement-to-adr-0026.md` — token/hue
  application conventions. No ADR covers routing or not-found architecture.

**Gap:** 0087 currently has only the work item and one work review
(`meta/reviews/work/0087-error-screen-affordances-review-1.md`) — no
implementation plan, validation, or PR yet.

## Related Research

- `meta/research/codebase/2026-05-12-0037-glyph-component.md` — Glyph component
  (the small-glyph family that null-renders on unknown type).
- `meta/research/design-inventories/2026-05-21-004250-current-app/inventory.md`
  — the "before" current-app inventory.

## Open Questions

- **`useQueries` vs fixed hooks for 13-type aggregation.** No existing
  `useQueries` usage to copy; the resolver precedent uses fixed hooks. The plan
  should decide the aggregation mechanism and confirm the cache-warm assumption
  (the story notes it may trigger up to one `fetchDocs(type)` per key if cold).
- **Include `templates` in suggestions?** Server search excludes it; mirroring
  that excludes virtual-type slugs. Likely exclude, but confirm.
- **Catch-all eyebrow composition.** Since the small `Glyph` null-renders for an
  absent type, the router-level catch-all eyebrow needs a type-free design
  (plain label, or a neutral non-`Glyph` mark). Not yet specified.
- **Recovery-CTA styling.** No filled/primary button primitive exists; the back-
  links will use `.ac-topbar__btn`-style ghost buttons (per Technical Notes), but
  the exact component/markup is net-new and unspecified.
- **Heading for the fetch-error state.** The story suggests `Couldn't load this
  document` as an example but leaves the final string open.
