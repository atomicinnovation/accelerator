---
type: plan-validation
id: "2026-06-12-0087-error-screen-affordances-validation"
title: "Validation Report: 404 / Error Screen with Affordances Implementation Plan"
date: "2026-06-13T08:06:59+00:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: "pass"
parent: "plan:2026-06-12-0087-error-screen-affordances"
target: "plan:2026-06-12-0087-error-screen-affordances"
tags: [design, frontend, error-states, routing, search, suggestions]
last_updated: "2026-06-13T08:06:59+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: 404 / Error Screen with Affordances

### Implementation Status

✓ Phase 1: Nearby-slug suggestion engine (pure logic + hook) — Fully implemented
✓ Phase 2: `NotFoundSurface` and `LoadErrorSurface` components — Fully implemented
✓ Phase 3: Wire surfaces into `LibraryDocView` — Fully implemented
✓ Phase 4: Router `notFoundComponent` for the catch-all — Fully implemented

All four phases were delivered across four atomic commits on top of the plan
commit:

- `yrlzyxtzlwlr` Add nearby-slug suggestion ranking engine and aggregation hook (Phase 1)
- `orkyumrylxvl` Add NotFoundSurface and LoadErrorSurface recovery components (Phase 2)
- `yzwuswwrzmsn` Render recovery surfaces from LibraryDocView branches (Phase 3)
- `mnytxqvuvmum` Add router catch-all not-found surface for unmatched URLs (Phase 4)

The working copy is clean — the entire implementation is committed.

### Automated Verification Results

✓ Type checking passes: `mise run types:frontend:check` (`tsc -b --noEmit`, clean)
✓ Unit tests pass: `mise run test:unit:frontend` — **120 test files, 2433 tests, all passing**
✓ Lint + format clean: `mise run frontend:check` — Biome lint (`--error-on-warnings`)
  + format both pass over 388/341 files; the `biome-ignore
  lint/correctness/useExhaustiveDependencies` suppression in the hook is the
  exact rule id the plan mandated (no `eslint-disable`)
✓ Worked-example test asserts the exact `[error-screen-v2, error-screens,
  legacy-error-screen]` order (`rank-slug-suggestions.test.ts`)
✓ No remaining rendered inline `Document not found.` `<p>` body in
  `LibraryDocView` — the only surviving match is an explanatory code comment

### Code Review Findings

#### Matches Plan:

- **All planned files created** under
  `frontend/src/routes/library/recovery/`: `rank-slug-suggestions.ts`,
  `use-nearby-slug-suggestions.ts`, `error-message.ts`, `RecoverySurface.tsx`,
  `RecoverySurface.module.css`, `NotFoundSurface.tsx`, `LoadErrorSurface.tsx`,
  `CatchAllNotFound.tsx`, plus their `.test` siblings.
- **Phase 1 ranking function** reproduces the slug-relevant subset of
  `classify()` exactly as specified: `PREFIX`/`INTERIOR` buckets only (no
  ExactSlug/Body), exact-slug exclusion, `(bucket asc, mtimeMs desc, relPath
  asc)` sort, `MAX_SUGGESTIONS = 5` cap, single `normaliseMissingSlug` /
  `isSuggestible` gate shared by the hook and the ranker.
- **Phase 1 hook** uses `useQueries` over `DOC_TYPE_KEYS.filter(isPhysicalDocTypeKey)`
  (templates excluded), gates on `enabled`/`isPending`, withholds the ranked list
  until the fan-out settles, drops rejected types via `r.data ?? []`, and ranks
  once. The `useExhaustiveDependencies` suppression is justified inline.
- **Phase 2 `BigGlyph` extension** is the additive, backwards-compatible change
  the revised plan called for: `docType?: DocTypeKey`, internal `?? DefaultBigGlyph`
  / `?? DEFAULT_BIG_HUE` fallback, and the DEV warning narrowed to
  `docType !== undefined && !BIG_GLYPHS[docType]` so the sanctioned no-type path
  stays silent.
- **Phase 2 surfaces** compose the shared `RecoverySurface` shell; eyebrow/hero/
  back-to-type are gated on `knownType`; `Back to library` is always present;
  ghost links reuse `HeaderActionButton.module.css` `.btn` (not the prototype
  `.ac-topbar__btn`); suggestions render as a plain `<ul>`/`<li>` of `<Link>`s
  (not `role="listbox"`); the `Did you mean…` heading is an `<h2>`; the deferred
  working hint uses a 250ms `useDelayedFlag` and a scoped visually-hidden
  `role="status" aria-live="polite"` region carrying only a short status string,
  with links rendered outside it.
- **Phase 3 `LibraryDocView`** returns `LoadErrorSurface` for both fetch-error
  branches (each resolving its own error through the shared `errorMessage()`
  helper) and `NotFoundSurface` for the true-404, preserving the
  `!entry && entries.length > 0` gate exactly. The inline `err instanceof Error`
  ternaries are gone.
- **Phase 4 router** extracts a shared `routerOptions` object consumed by both
  `createRouter` and the `buildRouter` fixture, sets
  `defaultNotFoundComponent: CatchAllNotFound`, and `CatchAllNotFound` lives in a
  `.tsx` file (no JSX leaked into `router.ts`). The unknown-type redirect is
  untouched.

#### Deviations from Plan:

- **`use-delayed-flag.ts` (new file, not separately named in the plan)** — the
  plan offered two options for the deferred hint ("generalise
  `useDeferredFetchingHint` with an `isActive` parameter, or use a small local
  delayed-boolean keyed on `isPending`"). The implementation took the second
  option as a dedicated `useDelayedFlag(active, delayMs = 250)` hook. This is a
  sanctioned choice, not an unplanned deviation, and is cleaner than inlining.
- **`styles/migration.test.ts` updated (+72 lines)** — not called out in the
  plan, but a required consequence of adding `RecoverySurface.module.css`: the
  repo's CSS-token migration guard test enforces that every literal in a CSS
  module is either tokenised or registered as `irreducible` with a reason. The
  new module's literals (96px hero column, 820px breakpoint, sub-token spacings,
  etc.) were registered accordingly, and the `--ac-empty-page-hue` local custom
  prop was added to `LOCAL_CUSTOM_PROPS`. This is correct adherence to an
  existing repo convention.
- **Eyebrow/glyph composition in suggestion rows** — `NotFoundSurface` renders
  per-row `<Glyph docType={s.type}>` with `DOC_TYPE_COLOR_VAR` styling, faithfully
  matching the `SearchResultsPanel` row precedent the plan referenced.

#### Potential Issues:

- **None blocking.** The known limitations are all explicitly accepted in the
  plan's "What We're NOT Doing" / per-phase notes: over-deep valid-type URLs
  (`/library/work-items/x/y/z`) fall to the catch-all and lose the
  `Back to {type} list` affordance; corpus-wide exact-slug exclusion can hide a
  same-slug doc under a different type; cold-cache fan-out issues up to 12
  `GET /api/docs?type=` requests; `fetchDocs` returns full `IndexEntry[]` while
  the engine reads only five fields. All are documented, intentional tradeoffs
  for a low-priority recovery surface.

### Manual Testing Required:

These per-phase manual items remain unchecked in the plan and require running
the app (they were explicitly deferred to a post-Phase-4 manual pass):

1. Not-found surface:
  - [ ] Visit `/library/work-items/nope-nope` → `Document not found` surface
    with suggestions where applicable; affordance links navigate correctly.
  - [ ] Confirm hero, eyebrow, and copy read correctly in **light and dark**
    themes (hero + gradient panel).

2. Load-error surface:
  - [ ] Force a list/content fetch failure (devtools offline or forced 5xx) →
    `Something went wrong loading this document`, no suggestions block, distinct
    heading.

3. Catch-all:
  - [ ] Navigate to `/garbage` and `/library/work-items/x/y/z` (over-deep) →
    `Page not found` surface, back-to-library works, renders within normal app
    chrome (sidebar/topbar present).
  - [ ] `/library/<unknown-type>` still redirects to `/library` (unchanged).

Note: the automated suite already covers the strongest correctness guards for
the above — the worked-example ordering, the deferred-hint 250ms timing (fake
timers), the scoped live-region behaviour, the three `LibraryDocView` branch
routings, and the in-chrome catch-all (`getByRole("main")` co-asserted with the
`Page not found` H1). The manual pass is primarily a visual/theme confirmation.

### Recommendations:

- Run the deferred manual visual pass (light/dark theme of the hero + gradient
  panel) before considering the work item fully closed, since no automated
  visual-regression baseline was added for the new surfaces.
- Consider a future slim projection endpoint for suggestions only if repos grow
  large enough that the full-`IndexEntry[]` cold-cache payload becomes a concern
  (flagged in the plan's Performance Considerations; not warranted now).
