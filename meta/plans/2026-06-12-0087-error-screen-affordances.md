---
type: plan
id: "2026-06-12-0087-error-screen-affordances"
title: "404 / Error Screen with Affordances Implementation Plan"
date: "2026-06-12T22:02:45+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0087"
parent: "work-item:0087"
derived_from: ["codebase-research:2026-06-12-0087-error-screen-affordances"]
relates_to: ["work-item:0041", "work-item:0082", "work-item:0074", "work-item:0054"]
tags: [design, frontend, error-states, routing, search, suggestions]
revision: "6547fec39be919e1f9669d4c4ecac36fd2e41077"
repository: "build-system"
last_updated: "2026-06-12T23:07:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 404 / Error Screen with Affordances Implementation Plan

## Overview

Replace the three conflated inline branches in `LibraryDocView` (one true 404,
two fetch errors, all sharing a bare `Document not found` rendering) with two
purpose-built surfaces rendered through the `Page` shell: a reusable
**not-found surface** (used both for the unknown-slug 404 and a new router-level
catch-all) and a sibling **load-error surface** for fetch failures. The
not-found surface offers recovery affordances — a `Back to library` link, a
conditional `Back to {type} list` link, and a `Did you mean…` block of up to
five nearby-slug suggestions ranked by a client-side engine that mirrors the
server's `classify()` bucketing. A new `notFoundComponent` on the router gives
truly-unmatched URLs the same surface they currently lack.

## Current State Analysis

All paths below are rooted at `skills/visualisation/visualise/` — the app does
**not** live at repo-root `frontend/`.

- **No 404 surface exists.** `frontend/src/routes/library/LibraryDocView.tsx`
  renders three mutually-exclusive branches, all under the same
  `Document not found` H1:
  - list-error (`LibraryDocView.tsx:95-102`) — `useQuery` for `fetchDocs` rejected;
  - content-error (`LibraryDocView.tsx:103-112`) — `useDocPageData` content query rejected;
  - true-404 (`LibraryDocView.tsx:113-115`) — `!entry && entries.length > 0`.

  Branches 1 and 2 are network/server failures, not missing documents, yet show
  identical "Document not found" copy. The success branch is `entry &&
  content.data` (`LibraryDocView.tsx:116`). All three not-found/error branches
  render through `<Page>` with `hasResolvedDocument === false`, suppressing the
  eyebrow and actions (`LibraryDocView.tsx:209-229`).

  **Refinement (confirmed):** the true-404 branch is gated on
  `entries.length > 0` (`LibraryDocView.tsx:113`). If the list is still loading
  (default `[]`) or genuinely empty, the page falls through to the `Loading…`
  default (`LibraryDocView.tsx:91-93`). So branch (a) only triggers for a
  genuinely-unmatched slug under a type with ≥1 document. The plan preserves this
  gate exactly.

- **No `notFoundComponent`.** `frontend/src/router.ts:218` is
  `createRouter({ routeTree })` — no `defaultNotFoundComponent`, no per-route
  override. Truly-unmatched URLs fall to TanStack Router's built-in default.

- **Unknown *types* never reach the detail view.** `libraryTypeRoute`'s
  `parseParams` (`router.ts:110-115`) `throw redirect({ to: "/library" })`s for a
  type failing `isDocTypeKey`. Because `libraryDocRoute` is its child, this fires
  for the detail route too — so the back-to-type affordance only matters when the
  type segment is valid but the slug is not.

- **`Page` shell (0041) is shipped and presentation-only.** `Page.tsx:4-11`:
  `eyebrow?`, `title` (a `ReactNode` H1), `subtitle?`, `actions?`,
  `maxWidth?`, `children`. No glyph/hue prop — per-type glyph + colour are
  composed into the `eyebrow`/`children` slots by callers. `EmptyState.tsx:29-47`
  is the closest precedent: it composes `BigGlyph` into a hero column, sets
  `--ac-empty-page-hue`, and uses `.ac-empty-page`-style CSS
  (`EmptyState.module.css`).

- **No suggestion / fuzzy machinery exists.** The wiki-link resolver
  (`use-wiki-link-resolver.ts:46-89`) fires a *fixed pair* of `useQuery` hooks
  (`decisions` + `work-items`) and builds **exact-match** `Map` indexes
  (`wiki-links.ts`). There is no `useQueries` usage anywhere to copy, and no
  Levenshtein/trigram/prefix-ranking logic. The reusable contracts are
  `fetchDocs(type)` (`fetch.ts:108-114`), `queryKeys.docs(type)`
  (`query-keys.ts:50`), and the candidate string `entry.slug ??
  fileSlugFromRelPath(entry.relPath)` (`path-utils.ts:6-8`,
  `LibraryTypeView.tsx:265-271`).

- **Authoritative ranking = `classify()`** (`server/src/api/search.rs:41-129`):
  a 4-variant `Bucket` enum (`ExactSlug=0, Prefix=1, Interior=2, Body=3`), first
  match wins top-down, ASCII-case-insensitive, then per-bucket
  `sort_by_cached_key((Reverse(mtime_ms), rel_path))`. Templates are skipped
  (`search.rs:114`); slug-less entries dropped in `project()` (`search.rs:85-93`).
  The `>= 2` minimum is **client-side only** (`use-search.ts:7-8`).

- **BigGlyph (0082) / per-type tints (0074) shipped.** `BigGlyph` degrades to
  `DefaultBigGlyph` @ `DEFAULT_BIG_HUE = 215` for an off-union type
  (`BigGlyph.tsx:75-82`) — ideal for the catch-all. **Gotcha:** the small `Glyph`
  **returns `null`** for an off-union type (`Glyph.tsx:81-89`), so a bare
  type-keyed `Glyph` eyebrow would silently vanish on the catch-all.

### Key Discoveries

- True-404 gate is `!entry && entries.length > 0` (`LibraryDocView.tsx:113`) —
  must be preserved so loading/empty states don't render the 404.
- `IndexEntry.slug` is **nullable** (`types.ts:120`); the mtime field is
  `mtimeMs` (`types.ts:128`, not `mtime`/`mtime_ms`); `relPath` is present
  (`types.ts:119`). Because the engine sorts over `IndexEntry` (not the wire
  `SearchResult`, which drops `relPath` — `fetch.ts:206-211`), the full
  `(bucket, mtimeMs desc, relPath asc)` tie-break is faithfully implementable.
- `DOC_TYPE_KEYS` has 13 entries; `templates` is virtual
  (`VIRTUAL_DOC_TYPE_KEYS`, `types.ts:48-50`) and `isPhysicalDocTypeKey`
  (`types.ts:59-63`) filters it out — matching the server skipping Templates.
- Test infra: vitest + Testing Library, `setupFiles: ./src/test/setup.ts`
  (`vite.config.ts`). Component tests wrap in `QueryClient({ retry:false })` +
  `MemoryRouter` (`src/test/router-helpers.tsx`). Router/`Link`-href tests use
  `buildRouter(url)` + `waitForPath` (`src/test/router-fixtures.ts`) and
  `renderWithRouterAt` (`src/test/router-helpers.tsx`).

## Desired End State

- A `NotFoundSurface` component renders for the unknown-slug 404 (H1
  `Document not found`) and the router catch-all (H1 `Page not found`).
- A `LoadErrorSurface` component renders for both fetch-error branches (H1
  `Something went wrong loading this document`), with **no** `Did you mean…`
  block.
- Both surfaces compose a shared `RecoverySurface` shell and always show a
  `Back to library` link, show a `Back to {type} list` link only when a valid
  type is present, and show a per-type framed-glyph eyebrow only when a valid
  type is present (none on the catch-all).
- The unknown-slug 404 surface lists up to five nearby-slug suggestions when ≥1
  match exists and the missing slug is ≥2 chars, ordered prefix-before-interior
  then mtime-desc then relPath-asc, each linking to its detail route; the block
  is omitted entirely when empty, and shows a working hint (in a polite live
  region) while the suggestion fan-out is still resolving.
- `router.ts` has a `defaultNotFoundComponent` (a `CatchAllNotFound` component
  reference, defined in a `.tsx` file), shared with the `buildRouter` test
  fixture; `/garbage` renders `NotFoundSurface` (catch-all variant) rather than
  the framework default.

Verification: `mise run test:unit:frontend` and `mise run types:frontend:check`
pass; manual checks per phase below.

## What We're NOT Doing

- **No Levenshtein / fuzzy / trigram matching** — substring + prefix bucketing
  only (story-resolved; deferred as future work).
- **No retry control** on the load-error surface (explicitly out of scope).
- **No new server endpoint** — suggestions aggregate client-side over
  per-type `fetchDocs`.
- **No batch/all-types fetch endpoint** — confirmed none exists (`fetch.ts`).
- **No changes to `classify()`** or the server search ranking.
- **No new filled/primary button primitive** — back-links reuse the existing
  shipped ghost-link treatment (`HeaderActionButton.module.css` `.btn`) as
  TanStack `<Link>`s. (The `.ac-topbar__btn` name in the work item is a prototype
  literal with no class in the shipped app.)
- **No `templates` in suggestion aggregation** (mirrors server skipping Templates).
- **No change to the unknown-type redirect** (`router.ts:110-115`) — it stays;
  the catch-all only covers URLs that match no route at all.
- **No exact-slug suggestion bucket** — an exact slug is a *found* document, not
  a 404.

## Implementation Approach

Bottom-up and test-first. Phase 1 builds the suggestion engine as a **pure,
fully-unit-tested ranking function** plus a thin `useQueries` aggregation hook.
Phase 2 builds a shared `RecoverySurface` shell and the two surfaces that
compose it (`NotFoundSurface`, `LoadErrorSurface`), consuming the hook, with
component tests. Phases 3 and 4 wire the surfaces into `LibraryDocView` and the
router respectively. Each phase compiles, passes tests, and is mergeable on its
own — Phases 1–2 add tested-but-unwired code (the established pattern for this
codebase's component-first work), and Phases 3–4 are the user-visible
activations.

All new files live under `frontend/src/routes/library/recovery/` — a neutral
name (not `not-found/`) because `LoadErrorSurface` is a fetch-error state, not a
not-found state. The catch-all in `router.ts` imports `NotFoundSurface` from
this folder; that cross-route import is intentional (the surfaces are shared
between the library detail view and the app-global router).

TDD ordering within each phase: write the failing test that encodes the
acceptance criterion (most precisely, the worked example in Phase 1), then
implement to green.

---

## Phase 1: Nearby-slug suggestion engine (pure logic + hook)

### Overview

A pure ranking function (the TDD core, mirroring `classify()`) and a
`useQueries`-based aggregation hook. Neither is wired into a rendered surface
yet, so this phase is mergeable in isolation.

### Changes Required

#### 1. Pure ranking function

**File**: `skills/visualisation/visualise/frontend/src/routes/library/recovery/rank-slug-suggestions.ts` (new)
**Changes**: Pure function reproducing the slug-relevant subset of `classify()`,
plus a single-source normalisation/gate helper reused by the hook.

```ts
import type { DocTypeKey } from "../../../api/types";

/** Minimum missing-slug length before suggestions are generated. Mirrors the
 *  search hook's gate (`use-search.ts:7-8`) and the server-side intent. */
export const MIN_SUGGESTION_LEN = 2;
export const MAX_SUGGESTIONS = 5;

/** A suggestion candidate sourced from an `IndexEntry`. `slug` is the link-ready
 *  string (`entry.slug ?? fileSlugFromRelPath(entry.relPath)`), never null. */
export interface SlugCandidate {
  type: DocTypeKey;
  slug: string;
  title: string;
  mtimeMs: number;
  relPath: string;
}

/** The single normalisation used everywhere a missing slug is measured/matched:
 *  trim, then lowercase. */
export function normaliseMissingSlug(missingSlug: string): string {
  return missingSlug.trim().toLowerCase();
}

/** The single suggestibility gate. Both the hook's `enabled` flag and
 *  `rankSlugSuggestions` call this so the gate has exactly one definition and
 *  cannot drift (was previously expressed three times with `trim` vs
 *  `trim().toLowerCase()` inconsistency — review finding). */
export function isSuggestible(missingSlug: string): boolean {
  return normaliseMissingSlug(missingSlug).length >= MIN_SUGGESTION_LEN;
}

// 0 = prefix (higher quality), 1 = interior. ExactSlug/Body are intentionally
// absent: an exact slug is a *found* doc, and body matching is out of scope.
const PREFIX = 0;
const INTERIOR = 1;

function bucket(candidateSlug: string, missingLc: string): number | null {
  const s = candidateSlug.toLowerCase();
  if (s === missingLc) return null; // exact ⇒ not a 404 candidate
  if (s.startsWith(missingLc)) return PREFIX;
  if (s.includes(missingLc)) return INTERIOR;
  return null;
}

/** Rank candidates against a missing slug. Returns up to MAX_SUGGESTIONS,
 *  ordered by (bucket asc, mtimeMs desc, relPath asc). Returns [] when the
 *  missing slug is not suggestible (after normalisation). */
export function rankSlugSuggestions(
  missingSlug: string,
  candidates: readonly SlugCandidate[],
): SlugCandidate[] {
  if (!isSuggestible(missingSlug)) return [];
  const missingLc = normaliseMissingSlug(missingSlug);

  const scored: { c: SlugCandidate; b: number }[] = [];
  for (const c of candidates) {
    const b = bucket(c.slug, missingLc);
    if (b !== null) scored.push({ c, b });
  }

  scored.sort(
    (x, y) =>
      x.b - y.b ||
      y.c.mtimeMs - x.c.mtimeMs ||
      x.c.relPath.localeCompare(y.c.relPath),
  );

  return scored.slice(0, MAX_SUGGESTIONS).map((s) => s.c);
}
```

Notes:
- `localeCompare` for the `relPath` tiebreak gives deterministic ascending
  ordering; the server uses byte order but the tiebreak only matters when bucket
  *and* mtime are equal, where any stable total order satisfies the criterion.
- Use `.toLowerCase()` (close enough to the server's `to_ascii_lowercase`; the
  minor non-ASCII divergence is acceptable per the research). `normaliseMissingSlug`
  / `isSuggestible` are the single source for both the length gate and the
  match-normalisation, so the hook's `enabled` flag can never disagree with the
  pure function's internal guard.
- **Exact-slug exclusion is corpus-wide (accepted scope):** `bucket()` drops any
  candidate whose slug equals the missing slug, across all types. On the rare
  cross-type slug collision this also hides a genuinely-reachable same-slug doc
  under a *different* type. This is accepted for this low-priority surface (the
  server's `classify()` reasons per-snapshot too); not worth the extra
  type-aware branch.

#### 2. Aggregation hook

**File**: `skills/visualisation/visualise/frontend/src/routes/library/recovery/use-nearby-slug-suggestions.ts` (new)
**Changes**: `useQueries` over the 12 physical doc types, aggregate to
`SlugCandidate[]`, rank, and return both the ranked list and a `isPending`
signal so the surface can show a loading hint and avoid mid-flight re-ranking.

```ts
import { useQueries } from "@tanstack/react-query";
import { useMemo } from "react";
import { fetchDocs } from "../../../api/fetch";
import { fileSlugFromRelPath } from "../../../api/path-utils";
import { queryKeys } from "../../../api/query-keys";
import { DOC_TYPE_KEYS, isPhysicalDocTypeKey } from "../../../api/types";
import {
  isSuggestible,
  rankSlugSuggestions,
  type SlugCandidate,
} from "./rank-slug-suggestions";

const PHYSICAL_KEYS = DOC_TYPE_KEYS.filter(isPhysicalDocTypeKey);

export interface NearbySlugSuggestions {
  /** Ranked suggestions. Empty until the fan-out has settled (see `isPending`). */
  suggestions: SlugCandidate[];
  /** True while the surface is enabled and at least one enabled query is still
   *  in flight — the surface shows a loading hint and withholds the list until
   *  this is false, so suggestions appear once in final ranked order rather than
   *  popping in and re-sorting as queries resolve. */
  isPending: boolean;
}

export function useNearbySlugSuggestions(
  missingSlug: string,
): NearbySlugSuggestions {
  const enabled = isSuggestible(missingSlug);

  const results = useQueries({
    queries: PHYSICAL_KEYS.map((type) => ({
      queryKey: queryKeys.docs(type),
      queryFn: () => fetchDocs(type),
      enabled,
    })),
  });

  // Settled = enabled and no enabled query still fetching. `r.isPending` for an
  // `enabled:false` query is irrelevant because `enabled` short-circuits below.
  const isPending = enabled && results.some((r) => r.isPending);

  return useMemo(() => {
    if (!enabled) return { suggestions: [], isPending: false };
    // Withhold the (re-)ranked list until the fan-out settles, so the rendered
    // block doesn't shuffle as individual queries resolve.
    if (isPending) return { suggestions: [], isPending: true };

    const candidates: SlugCandidate[] = [];
    for (const r of results) {
      for (const e of r.data ?? []) {
        candidates.push({
          type: e.type,
          slug: e.slug ?? fileSlugFromRelPath(e.relPath),
          title: e.title,
          mtimeMs: e.mtimeMs,
          relPath: e.relPath,
        });
      }
    }
    return {
      suggestions: rankSlugSuggestions(missingSlug, candidates),
      isPending: false,
    };
    // biome-ignore lint/correctness/useExhaustiveDependencies: `results` is a
    // fresh array identity every render; the meaningful inputs are the per-query
    // `data` references plus the settle flag, which we list explicitly. Gating
    // the body on `!isPending` means the rank runs once, when all data is ready.
  }, [enabled, isPending, missingSlug, ...results.map((r) => r.data)]);
}
```

Notes:
- **Active aggregation** (per decision): `useQueries` fires `fetchDocs` for all
  12 physical types when `enabled`, reusing warm cache entries. On a cold cache
  this is up to 12 `GET /api/docs?type=` requests; the hook reports `isPending`
  until they all settle, then ranks **once** and returns the final list. This
  removes the mid-flight re-rank and collapses the per-resolution recomputation
  to a single pass (review: usability + performance).
- `templates` excluded via `isPhysicalDocTypeKey` (server-parity).
- **Partial failure:** `r.data ?? []` drops types whose query rejected, so a
  partial 5xx ranks over the resolved subset (accepted degradation for a
  recovery surface). A fully-failed fan-out settles with no candidates, so the
  block is omitted — the same as "no matches".
- **Biome, not ESLint:** the suppression is `// biome-ignore
  lint/correctness/useExhaustiveDependencies: …` (this project lints with Biome;
  there is no ESLint). The spread-into-deps is kept but justified inline.

### Tests (write first)

**File**: `.../recovery/rank-slug-suggestions.test.ts` (new) — pure, no providers.

- **Worked example (AC):** missing `error-screen`, candidates `error-screen-v2`
  (mtime T₂ newer), `error-screens` (mtime T₁ older), `legacy-error-screen`
  (interior), `error-handling` (no match) ⇒ exactly
  `[error-screen-v2, error-screens, legacy-error-screen]`, in that order.
- Prefix outranks interior regardless of mtime.
- mtime-desc orders within a bucket.
- **Dedicated `relPath` tiebreak case:** two candidates in the **same bucket**
  with an **identical `mtimeMs`** but differing `relPath` ⇒ the lexicographically
  smaller `relPath` comes first. (Isolates the last, least-exercised sort key so
  a mutation dropping/inverting the `localeCompare` term fails a test — review
  re-pass: test-coverage.)
- >5 matches ⇒ exactly the top 5, sixth+ omitted.
- Mixed-case missing slug (`Error-Screen`) matches lowercase candidate
  (`error-screen-v2`) on both prefix and interior branches.
- Missing slug `< 2` chars ⇒ `[]`.
- **Whitespace-padded missing slug** (e.g. `"  er"`) normalises and behaves
  like `"er"` — pins the `trim()` in `normaliseMissingSlug` (review finding).
- **Null-slug candidate:** a candidate whose `slug` was derived from
  `relPath` (i.e. the `fileSlug` stem, not a frontmatter slug) is still ranked
  and matched on that stem — covers the `slug ?? fileSlugFromRelPath` path.
- Exact slug match excluded (never suggests a found document).
- No matches ⇒ `[]`.
- `isSuggestible` unit cases: `"a"`/`" "`/`""` ⇒ false; `"ab"`/`"  ab "` ⇒ true
  (the single gate both the hook and the ranker share).

**File**: `.../recovery/use-nearby-slug-suggestions.test.tsx` (new) — `QueryClient(retry:false)` wrapper, `vi.spyOn(fetchModule, "fetchDocs")`.

- Aggregates across multiple types and returns ranked candidates once settled
  (`isPending` false, `suggestions` populated).
- **Pending then settled (single-pass gate):** resolve the fan-out **staggered**
  (one type at a time across acts). Assert `suggestions` stays `[]` with
  `isPending: true` until the **last** query settles, and only then becomes the
  fully-ordered list with `isPending: false` — exercising the `!isPending` gate
  itself, not just its terminal states, so a regression that re-ranks per
  resolving query (or drops the `isPending` short-circuit) fails (review re-pass:
  test-coverage).
- **Partial failure:** some types resolve, one rejects ⇒ hook settles
  (`isPending: false`) and ranks over the resolved subset **without throwing**;
  a slug only present in the rejected type is absent.
- `enabled === false` for `< 2` chars ⇒ `{ suggestions: [], isPending: false }`
  and does **not** call `fetchDocs` (assert spy not called).
- A null-`slug` `IndexEntry` from `fetchDocs` ⇒ aggregated as its relPath stem
  (asserts the hook's `slug ?? fileSlugFromRelPath(relPath)` mapping).
- `templates` is not queried (assert `fetchDocs` never called with `"templates"`).

### Success Criteria

#### Automated Verification

- [x] Unit tests pass: `mise run test:unit:frontend`
- [x] Type checking passes: `mise run types:frontend:check`
- [x] Format/lint clean: `mise run frontend:check` (Biome lint runs here as
  warnings-as-errors — the hook's `biome-ignore` suppression must use the exact
  rule id `lint/correctness/useExhaustiveDependencies`, not an `eslint-disable`)
- [x] Worked-example test asserts the exact `[error-screen-v2, error-screens, legacy-error-screen]` order

#### Manual Verification

- [x] (None — no rendered surface yet; verified via tests.)

---

## Phase 2: `NotFoundSurface` and `LoadErrorSurface` components

### Overview

A shared `RecoverySurface` shell owns the chrome common to both surfaces
(`Page` composition + eyebrow + hero + ghost back-link row); `NotFoundSurface`
and `LoadErrorSurface` compose it and supply only what differs (H1, body copy,
and — for `NotFoundSurface` — the suggestion block). Consumes Phase 1's hook.
Not yet wired into routing/views, so mergeable in isolation.

### Changes Required

#### 1. `RecoverySurface` (shared shell)

**File**: `skills/visualisation/visualise/frontend/src/routes/library/recovery/RecoverySurface.tsx` (new)
**Changes**: The shared presentational shell, so the eyebrow / hero / back-link
affordance logic lives in one place rather than being duplicated across the two
surfaces (review: code-quality).

Props:

```ts
interface RecoverySurfaceProps {
  /** Rendered as the Page H1. */
  title: ReactNode;
  /** Valid doc type from the URL, when present. Drives the eyebrow, the
   *  per-type hero hue, and the `Back to {type} list` link. Absent ⇒ catch-all
   *  / no type (default hero hue 215, no eyebrow, no back-to-type). */
  knownType?: DocTypeKey;
  /** Body copy (sentence case, terminal period). */
  children: ReactNode;
}
```

Behaviour (owned here, shared by both surfaces):
- **Page H1:** renders `title` via `Page`'s `title` slot (single `<h1>`).
- **Eyebrow:** `<EyebrowLabel type={knownType} />` when `knownType` present;
  omitted otherwise (avoids the null-rendering `Glyph` gotcha — decision).
- **Hero:** `<BigGlyph docType={knownType} />`. `BigGlyph` is extended in this
  phase to accept an **optional** `docType` (see the hero decision below), so an
  absent type renders `DefaultBigGlyph` at hue 215 through the *same* component —
  no hand-rolled SVG shell. When `knownType` is present, hue comes from
  `DOC_TYPE_HUE[knownType]`.
- **Back-link row (always):** `Back to library` → `<Link to="/library">`.
  **Conditional:** `Back to {DOC_TYPE_LABELS_SINGULAR[knownType]} list` →
  `<Link to="/library/$type" params={{ type: knownType }}>` only when `knownType`
  present. Rendered as ghost links reusing the shipped `HeaderActionButton`
  `.btn` treatment (`components/DetailHeaderActions/HeaderActionButton.module.css`)
  — **not** the prototype's `.ac-topbar__btn` literal, which has no class in the
  shipped app (review: standards).

**Hero decision (revised — extend `BigGlyph`):** rather than hand-rolling an
`<svg>` around `DefaultBigGlyph` for the no-type case (which duplicated
`BigGlyph`'s viewBox/palette contract — review: architecture + code-quality),
make `BigGlyph`'s `docType` prop optional. `BigGlyph` already resolves
`?? DefaultBigGlyph` and `?? DEFAULT_BIG_HUE` internally (`BigGlyph.tsx:75-76`),
so accepting `docType?: DocTypeKey` and defaulting an absent value to the
existing fallback keeps a single rendering authority and avoids casting an
invalid `DocTypeKey`. This is an additive, backwards-compatible change to the
shipped 0082 component (all existing call sites pass a real `docType`).

**Also narrow the DEV guard:** `BigGlyph.tsx:77` `console.warn`s in DEV whenever
`BIG_GLYPHS[docType]` is falsy — which now includes the *intended* `docType ===
undefined` catch-all/load-error path, firing a misleading "Unknown docType" warning
on every such render (review re-pass: architecture + code-quality). Guard it on
`docType !== undefined && !BIG_GLYPHS[docType]` so an absent type stays silent
(sanctioned default) while a genuinely off-union *supplied* key still warns.

#### 2. `NotFoundSurface`

**File**: `skills/visualisation/visualise/frontend/src/routes/library/recovery/NotFoundSurface.tsx` (new)
**Changes**: Composes `RecoverySurface`; adds the missing-slug copy and the
`Did you mean…` block.

Props:

```ts
interface NotFoundSurfaceProps {
  /** The missing document slug, when one is present (unknown-slug 404).
   *  Absent on the router-level catch-all. Drives the H1, the mono-quoted
   *  query, and suggestion generation. */
  missingSlug?: string;
  /** The valid doc type from the URL's first /library/ segment, when it
   *  passes isDocTypeKey. Drives the eyebrow, the per-type hero hue, and the
   *  `Back to {type} list` link. Absent ⇒ catch-all (DefaultBigGlyph, no
   *  eyebrow, no back-to-type). */
  knownType?: DocTypeKey;
}
```

Behaviour (delegates chrome to `RecoverySurface`; adds the 404-specific parts):
- **H1 (passed as `title`):** `Document not found` when `missingSlug` is present
  (slug 404); `Page not found` when absent (catch-all).
- **Body copy:** sentence case, terminal period, no apology. When `missingSlug`
  present, quote it in a mono `<span>` (mirror `SearchResultsPanel.tsx:101-105`
  microcopy voice). The catch-all has no slug to quote.
- **`Did you mean…` block:** call `useNearbySlugSuggestions(missingSlug ?? "")`,
  which returns `{ suggestions, isPending }`. Render rules:
  - **Deferred working hint:** show a lightweight hint (e.g. "Looking for
    similar documents…") only once `isPending` has held for ~250ms, applying the
    same deferral convention as `useDeferredFetchingHint` (`api/use-deferred-fetching-hint.ts`)
    so the common **warm-cache** path (the 12 queries settle near-instantly under
    `staleTime: Infinity`) goes straight to the list with **no hint flash**
    (review re-pass: usability). Note the existing hook gates on `isFetching &&
    !isPending` (a *refetch*, not the initial load), so it can't be reused
    verbatim for our initial-load `isPending`; either generalise it with an
    `isActive` parameter or use a small local delayed-boolean keyed on
    `isPending` with the same 250ms threshold. Never render a half-populated list.
  - Once settled, render the block (an `<h2>` `Did you mean…` heading +
    suggestion links) only when `suggestions` is non-empty; omit entirely when
    empty (no empty block).
  - **Scoped live region:** announce only a concise *status string* to assistive
    tech — render a visually-hidden `role="status" aria-live="polite"` element
    carrying the deferred hint while pending and a short summary on settle (e.g.
    "5 similar documents found"). Render the `<h2>` + suggestion links as ordinary
    navigable content **outside** the `aria-live` region, so a screen reader
    summarises the outcome rather than reading all five link rows on settle
    (review re-pass: usability; the `EmptyState`/`SearchResultsPanel` precedents
    only ever announce a short status string, never a list of interactive links).
  - Each suggestion is `<Link to="/library/$type/$fileSlug" params={{ type:
    s.type, fileSlug: s.slug }}>`. **Reuse the visual row layout/CSS** from
    `SearchResultsPanel`, but render the suggestions as a plain list of links
    (`<ul>`/`<li>` of `<Link>`s) — **not** `role="listbox"`/`role="option"`,
    which is a composite-widget pattern that would mislead assistive tech and
    break per-link Tab order on a static page (review: usability).

#### 3. `LoadErrorSurface`

**File**: `skills/visualisation/visualise/frontend/src/routes/library/recovery/LoadErrorSurface.tsx` (new)
**Changes**: Sibling error surface; composes `RecoverySurface`.

```ts
interface LoadErrorSurfaceProps {
  /** Valid type from the URL, when present — same affordance rules as 404. */
  knownType?: DocTypeKey;
  /** Optional already-resolved error message, surfaced as supplementary detail
   *  (not the H1). The caller (LibraryDocView) resolves the raw error to a
   *  string via the shared `errorMessage()` helper, so this component stays
   *  purely presentational and cannot throw on a non-Error value. */
  errorMessage?: string;
}
```

Behaviour: delegates chrome to `RecoverySurface` (eyebrow when `knownType`,
hero, `Back to library` always, `Back to {type} list` when `knownType`) but:
- **H1:** `Something went wrong loading this document` (decision) — distinct from
  the 404 H1, never the string `Document not found`.
- **Body:** names a load/fetch failure (e.g. "We couldn't load this document
  right now."), sentence case, terminal period. When `errorMessage` is present,
  surface it as supplementary detail in a `role="alert"` line, mirroring the
  existing `styles.error` treatment (`LibraryDocView.tsx:98-101`).
- **No `Did you mean…` block** (a network/server failure is not a missing
  document) — `useNearbySlugSuggestions` is **not** called here.

Add a shared `errorMessage(e: unknown): string` helper (e.g. in
`recovery/error-message.ts`) replacing the inline `err instanceof Error ?
err.message : String(err)` ternary repeated in `LibraryDocView.tsx:100,108-110`;
`LibraryDocView` calls it when passing `errorMessage` to `LoadErrorSurface`
(review: code-quality). This keeps the surface presentational and immune to
non-Error values.

#### 4. Styles

**File**: `skills/visualisation/visualise/frontend/src/routes/library/recovery/RecoverySurface.module.css` (new)
**Changes**: Shared by both surfaces. (PascalCase matching the owning component —
every `*.module.css` in the frontend is named for its component; no kebab-case
module exists. Review re-pass: standards.) Reuse the `.ac-empty-page` hero+illustration
layout from `EmptyState.module.css` (two-column `96px 1fr` grid,
`--ac-empty-page-hue` radial-gradient panel, dashed border, single-column
collapse at 820px), plus a suggestion-list block modelled on the sidebar
search-row markup and a ghost back-link row reusing the `HeaderActionButton`
`.btn` treatment (not the prototype `.ac-topbar__btn` literal).

### Tests (write first)

**Test wrapper:** the surfaces call a `useQueries` hook *and* render `<Link>`s,
so tests need **both** a `QueryClientProvider` and a router context.
`renderWithRouterAt` alone provides only a `RouterProvider` (no query client),
so the hook would throw (review: test-coverage). Add a shared
`renderWithRouterAndQueryAt(ui, { url })` helper to `src/test/router-helpers.tsx`
that wraps `ui` in a `QueryClientProvider` (client `new QueryClient({
defaultOptions: { queries: { retry: false } } })`) and hands the wrapped tree to
the existing router rendering, so `<Link>` hrefs resolve and the fan-out can be
fed via `vi.spyOn(fetchModule, "fetchDocs")`. Both new surface tests use it.

**File**: `.../recovery/NotFoundSurface.test.tsx` (new).

- Known-type 404 (`missingSlug`, `knownType="work-items"`): H1
  `Document not found`; eyebrow present (`data-testid="eyebrow-label"`);
  `Back to library` link → `/library`; `Back to … list` link →
  `/library/work-items`; missing slug appears in a mono element.
- **Body copy assertion:** query the body paragraph specifically (a
  `data-testid` on the body `<p>`, not an arbitrary text node), assert the missing
  slug is inside a mono element (`<code>`/mono `<span>`) within it **and** that the
  paragraph's text content ends with `"."` (pins the terminal-period rule on the
  intended node without over-coupling to wording); keep one intentionally-exact
  assertion on the full sentence so copy edits are a deliberate test update
  (review: test-coverage).
- Catch-all (`missingSlug` absent, `knownType` absent): H1 `Page not found`; no
  eyebrow (`querySelector('[data-slot="eyebrow"]')` null); `Back to library`
  present; **no** `Back to … list` link.
- **Deferred-hint timing (fake timers):** with `fetchDocs` left unresolved, the
  working hint is **absent** before ~250ms and **present** after advancing timers
  past the threshold — pins the no-flash deferral (review re-pass: usability). The
  hint text lives in the `role="status"` region; **no** suggestion links appear yet.
- **Scoped live region:** once settled, assert the `role="status"` region carries
  only a short status string (e.g. "5 similar documents found"), and the `<h2>` +
  suggestion `<Link>`s are rendered **outside** that region (the links are not
  descendants of the `aria-live` element) — review re-pass: usability.
- `Did you mean…` ordering: once `fetchDocs` resolves the worked-example
  entries, the block lists `[error-screen-v2, error-screens, legacy-error-screen]`
  as links (assert order + hrefs), and the suggestion links are plain list links
  (no `role="option"`/`listbox`).
- **Heading level:** the `Did you mean…` heading is an `<h2>` (under the Page
  `<h1>`).
- >5 candidates ⇒ exactly 5 rendered.
- Mixed-case missing slug ⇒ matching candidate still listed.
- No matches / `< 2` chars ⇒ once settled, the `Did you mean…` block is absent
  (not an empty block).
- Suggestion link href shape: `/library/$type/$fileSlug`.

**File**: `.../recovery/LoadErrorSurface.test.tsx` (new).

- H1 is `Something went wrong loading this document` and is **not**
  `Document not found`.
- **No** `Did you mean…` block ever rendered (assert `fetchDocs` not called).
- `Back to library` present; `Back to … list` present iff `knownType` given.
- `errorMessage` given ⇒ appears in the `role="alert"` line; `errorMessage`
  absent ⇒ no alert line rendered (and renders without throwing).

**File**: `.../recovery/error-message.test.ts` (new).

- `errorMessage(new Error("boom"))` ⇒ `"boom"`; `errorMessage("boom")` ⇒
  `"boom"`; `errorMessage(undefined)`/`errorMessage(null)` ⇒ a stable fallback
  string, never a throw (covers the `unknown` inputs the content query can
  reject with).

### Success Criteria

#### Automated Verification

- [x] Unit tests pass: `mise run test:unit:frontend`
- [x] Type checking passes: `mise run types:frontend:check`
- [x] Format/lint clean: `mise run frontend:check`

#### Manual Verification

- [ ] Render both surfaces in isolation (e.g. via a temporary story/route or
  the test DOM) and confirm the hero, eyebrow, and copy read correctly in light
  and dark themes. (Deferred to the post-Phase-4 manual pass once the surfaces
  are reachable via routing.)

---

## Phase 3: Wire surfaces into `LibraryDocView`

### Overview

Replace the three inline branches (`LibraryDocView.tsx:95-115`) so the true-404
renders `NotFoundSurface` and the two fetch-error branches render
`LoadErrorSurface`. First user-visible change.

### Changes Required

#### 1. `LibraryDocView` branch split

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`
**Changes**: Return the new surfaces directly from the not-found/error branches
instead of building `title`/`body` for the shared `<Page>` render.

- **List-error** (`:95-102`) and **content-error** (`:103-112`) ⇒
  `return <LoadErrorSurface knownType={type} errorMessage={errorMessage(listErr ?? content.error)} />;`
  (each branch resolves its own error through the shared `errorMessage()` helper
  so the surface receives a `string`, not raw `unknown`). `type` is the narrowed
  `DocTypeKey` (always defined here because the `type === undefined` guard returns
  earlier at `:84-86`). Replace the inline `err instanceof Error ? … : String(err)`
  ternaries at `:100,108-110` with the same helper.
- **True-404** (`:113-115`, keep the `!entry && entries.length > 0` gate) ⇒
  `return <NotFoundSurface missingSlug={fileSlug} knownType={type} />;`
- Leave the success branch (`:116`) and the `Loading…` default untouched. The
  existing `<Page>` render at `:213-229` now only handles loading + success.
- The two early guard returns at `:84-89` (`Unknown doc type`, `Missing file
  slug`) are left as-is (the type guard is effectively dead under normal routing;
  out of scope to change here).

### Tests

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.test.tsx` (extend or add)

- Unknown slug under a valid type (mock `fetchDocs` to resolve a non-empty list
  with no matching slug) ⇒ `NotFoundSurface` renders: H1 `Document not found`,
  no old inline `Document not found.` `<p>`, `Back to library` present.
- List-fetch rejects ⇒ `LoadErrorSurface`: H1 `Something went wrong loading this
  document`, **no** `Did you mean…` block, `Back to library` present.
- Content-fetch rejects (list resolves with a matching entry, `useDocPageData`
  content query rejects) ⇒ `LoadErrorSurface` (same assertions). **Give the
  content rejection a distinct message** and assert that exact string surfaces in
  the `role="alert"` detail line — confirming the correct error
  (`errorMessage(listErr ?? content.error)`) is threaded through for this branch
  and not the list error (review re-pass: test-coverage).
- Loading (list pending) ⇒ still the `Loading…` default, **not** the 404
  (guards the `entries.length > 0` gate).
- **Null-slug entry in the list:** an `IndexEntry` with `slug: null` whose
  relPath stem the requested `fileSlug` does/doesn't match ⇒ the existing
  `slug === fileSlug || fileSlugFromRelPath(relPath) === fileSlug` match still
  works (a matching null-slug entry is *found*, a non-matching one falls to the
  404), confirming the nullable-slug path is exercised at the view boundary.
- Success branch unchanged (smoke).

### Success Criteria

#### Automated Verification

- [ ] Unit tests pass: `mise run test:unit:frontend`
- [ ] Type checking passes: `mise run types:frontend:check`
- [ ] Format/lint clean: `mise run frontend:check`
- [ ] No remaining inline `Document not found.` `<p>` body in `LibraryDocView`

#### Manual Verification

- [ ] Visit a valid type with a bogus slug (e.g. `/library/work-items/nope-nope`)
  → not-found surface with suggestions where applicable.
- [ ] Simulate a list/content fetch failure (devtools offline or a forced 5xx) →
  load-error surface with no suggestions block and the distinct heading.
- [ ] Affordance links navigate correctly (library + type list).

---

## Phase 4: Router `notFoundComponent` for the catch-all

### Overview

Give truly-unmatched URLs the not-found surface they currently lack.

### Changes Required

#### 1. Catch-all wrapper component (`.tsx`)

**File**: `skills/visualisation/visualise/frontend/src/routes/library/recovery/CatchAllNotFound.tsx` (new)
**Changes**: A named wrapper component for the catch-all, so JSX lives in a
`.tsx` file — `router.ts` is a `.ts` module and currently contains **zero JSX**
(routes are wired by identifier, e.g. `component: LibraryTypeView`); inline JSX
there would fail `tsc`/build (review: correctness + code-quality + standards).
Co-located with the surface it renders under `recovery/` (PascalCase filename =
exported symbol), rather than a root-level `src/` file where no component lives
(review re-pass: standards).

```tsx
import { NotFoundSurface } from "./NotFoundSurface";

/** Router-level catch-all: no missingSlug, no knownType ⇒ H1 `Page not found`,
 *  DefaultBigGlyph hero (hue 215), no eyebrow, `Back to library` only, no
 *  suggestions (no slug to match). */
export function CatchAllNotFound() {
  return <NotFoundSurface />;
}
```

#### 2. Shared router-options factory + `createRouter`

**File**: `skills/visualisation/visualise/frontend/src/router.ts`
**Changes**: Set `defaultNotFoundComponent: CatchAllNotFound` (a component
reference, no JSX). To make the catch-all **testable**, the
`defaultNotFoundComponent` option must be shared with the test fixture — the
existing `buildRouter` (`router-fixtures.ts`) builds its **own**
`createRouter({ routeTree, history })`, so a `defaultNotFoundComponent` set only
on the production `router` instance would never be exercised by `buildRouter`,
and the Phase 4 test could not pass (review: architecture + test-coverage).

Extract a shared options object and consume it from both:

```ts
// router.ts
import { CatchAllNotFound } from "./routes/library/recovery/CatchAllNotFound";

export const routerOptions = {
  routeTree,
  defaultNotFoundComponent: CatchAllNotFound,
};

export const router = createRouter(routerOptions);
```

Then update `buildRouter` (`router-fixtures.ts`) to spread `routerOptions` (or at
least pass `defaultNotFoundComponent: CatchAllNotFound`) alongside its injected
`history`, so the fixture router carries the same catch-all the app does.

- The unknown-*type* redirect (`router.ts:110-115`) is unchanged and still
  pre-empts `/library/<bad-type>/…`.
- The component must render inside the existing app chrome. Confirm
  `defaultNotFoundComponent` mounts within `RootLayout`/`Page` context; if
  TanStack renders it above the root layout, render it inside the same shell the
  other routes use (verify against the `buildRouter` fixture).
- **Over-deep valid-type URLs (e.g. `/library/work-items/x/y/z`) fall to this
  catch-all** and therefore lose the `Back to {type} list` affordance even though
  the type segment is valid. Accepted as a known limitation for this surface
  (deriving `knownType` from the URL in the catch-all is deferred); noted so it
  is a chosen tradeoff, not a latent bug (review: usability).

### Tests

**File**: `skills/visualisation/visualise/frontend/src/router.test.tsx` (extend)

- `buildRouter("/garbage")` (now carrying `defaultNotFoundComponent` via the
  shared `routerOptions`) → after settle, the catch-all renders: H1
  `Page not found`; `Back to library` link present (href `/library`); **no**
  `Back to … list` link; no eyebrow.
- **In-chrome assertion:** pin to a specific stable landmark `RootLayout` always
  renders — `screen.getByRole("main")` (the `<main>` wrapping the `Outlet`) —
  co-asserted with the `Page not found` H1, so the test fails concretely if the
  catch-all escapes the root shell rather than passing against a partial tree
  (review re-pass: test-coverage). This is the automated guard for the open
  verification item that TanStack mounts `defaultNotFoundComponent` inside the
  root `RootLayout`/`<Outlet>` (resolve that up front against the fixture).
- Existing redirect tests still pass (`/` → `/library`, `/library/bogus` →
  `/library`) — the catch-all must not regress the unknown-type redirect.

### Success Criteria

#### Automated Verification

- [ ] Unit tests pass: `mise run test:unit:frontend`
- [ ] Type checking passes: `mise run types:frontend:check`
- [ ] Format/lint clean: `mise run frontend:check`
- [ ] Router test asserts `/garbage` renders the `Page not found` H1

#### Manual Verification

- [ ] Navigate to `/garbage` and `/library/work-items/x/y/z` (over-deep) →
  catch-all `Page not found` surface, back-to-library works.
- [ ] `/library/<unknown-type>` still redirects to `/library` (unchanged).
- [ ] Catch-all renders within the normal app chrome (sidebar/topbar present).

---

## Testing Strategy

### Unit Tests

- **Ranking (Phase 1):** the worked example, prefix>interior, mtime-desc,
  relPath-asc tiebreak, top-5 cap, mixed-case, whitespace-normalisation, `<2`
  gate, `isSuggestible`, null-slug candidate, exact-slug exclusion, no-match ⇒ `[]`.
- **Hook (Phase 1):** aggregation across types, pending-then-settled gating,
  partial-failure resilience, disabled-below-2-chars (no fetch), null-slug
  mapping, templates excluded.
- **Surfaces (Phase 2):** affordance presence/absence by `knownType`, H1 strings,
  mono-quoted slug, terminal-period body, suggestion block presence/order/omission,
  pending working-hint + `role="status"` live region, plain-link (not listbox)
  suggestions, `<h2>` heading level, eyebrow rules, load-error has no suggestions
  and renders `errorMessage` in a `role="alert"` line; `errorMessage()` helper.

### Integration Tests

- **LibraryDocView (Phase 3):** each of the three branches routes to the correct
  surface; loading does not render the 404; success unchanged.
- **Router (Phase 4):** unmatched URL → catch-all surface; redirects preserved.

### Manual Testing Steps

1. `/library/work-items/does-not-exist` → `Document not found` + suggestions.
2. Force a fetch failure → `Something went wrong loading this document`, no
   suggestions.
3. `/garbage` → `Page not found`, back-to-library only.
4. Verify light/dark theme rendering of hero + gradient panel.
5. Verify suggestion links and back-links navigate correctly.

## Performance Considerations

- The active `useQueries` aggregation issues up to 12 `GET /api/docs?type=`
  requests on a **cold** cache when a 404 with a ≥2-char slug renders; warm cache
  entries (populated by library views / the wiki-link resolver) are reused. This
  is acceptable for a low-priority recovery surface and matches the story's
  stated assumption.
- **One-time burst:** the production QueryClient sets `staleTime: Infinity` with
  SSE `doc-changed` events as the sole invalidator (`use-doc-events.ts`), so once
  warmed the `docs(type)` entries never refetch on remount/focus — the 12-request
  burst is one-time per type until the next genuine `doc-changed` event or a
  `gcTime` (~5 min default) eviction. No batch/all-types endpoint is warranted.
- **Payload note:** `fetchDocs` returns full `IndexEntry[]` (frontmatter,
  bodyPreview, etc.) while the engine reads only `type/slug/title/mtimeMs/relPath`,
  so cold-cache cost scales with corpus *metadata* size, not the few fields used.
  Acceptable (only endpoint available, cache-shared); flag a future slim
  projection endpoint only if repos grow large.
- **No mid-flight re-rank / single recompute:** the hook withholds the ranked
  list until the enabled fan-out settles (`isPending`), then ranks **once**. This
  removes the visible re-sorting as queries resolve piecemeal *and* collapses what
  would otherwise be up to 12 redundant O(total entries) recomputations (one per
  resolving query) into a single pass. The surface shows a working hint while
  pending, so there is no layout thrash beyond the final block appearing once.
- Ranking is O(total entries) string comparisons on already-fetched data —
  negligible.

## Migration Notes

None — no persisted data or schema changes. Purely additive frontend code plus a
rewire of existing branches.

## References

- Original work item: `meta/work/0087-error-screen-affordances.md`
- Related research: `meta/research/codebase/2026-06-12-0087-error-screen-affordances.md`
- Work review: `meta/reviews/work/0087-error-screen-affordances-review-1.md`
- Inline branches to replace: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:95-115`
- Router: `skills/visualisation/visualise/frontend/src/router.ts:110-115, 218`
- Page shell (0041): `skills/visualisation/visualise/frontend/src/components/Page/Page.tsx`
- Hero precedent: `skills/visualisation/visualise/frontend/src/routes/library/EmptyState.tsx:29-47`, `EmptyState.module.css`
- Suggestion building blocks: `skills/visualisation/visualise/frontend/src/api/fetch.ts:108-114`, `path-utils.ts:6-8`, `types.ts:23-63,116-148`, `query-keys.ts:50`
- Wiki-link precedent: `skills/visualisation/visualise/frontend/src/api/use-wiki-link-resolver.ts:46-89`
- Ranking authority: `skills/visualisation/visualise/server/src/api/search.rs:41-129`
- Search-row + microcopy: `skills/visualisation/visualise/frontend/src/components/Sidebar/SearchResultsPanel.tsx:64-107`
- BigGlyph (extend with optional `docType`): `skills/visualisation/visualise/frontend/src/components/BigGlyph/BigGlyph.tsx:66-98` (internal `?? DefaultBigGlyph` / `?? DEFAULT_BIG_HUE` at `:75-76`)
- Glyph null-render gotcha: `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx:81-89`
- Ghost back-link treatment: `skills/visualisation/visualise/frontend/src/components/DetailHeaderActions/HeaderActionButton.module.css` (`.btn`)
- Live-region precedent: `EmptyState` (`role="status"`), `SearchResultsPanel` empty (`role="status" aria-live="polite"`) + loading bar
- Test fixtures: `skills/visualisation/visualise/frontend/src/test/router-helpers.tsx`, `src/test/router-fixtures.ts`
