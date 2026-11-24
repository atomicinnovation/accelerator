---
type: "plan-validation"
id: "2026-11-24-PP-996-library-links-by-document-id-validation"
title: "Validation Report: Library Links by Document ID Implementation Plan"
date: "2026-11-24T18:24:27+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-11-24-PP-996-library-links-by-document-id"
relates_to: ["work-item:PP-1003"]
tags: ["visualiser", "frontend", "server", "library-links"]
last_updated: "2026-11-24T18:24:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Library Links by Document ID Implementation Plan

### Implementation Status

✓ Phase 1: Carry the document's `id` on index entries and search rows -
Fully implemented
✓ Phase 2: Resolve library URLs by `id`, slug, then stem - Fully implemented
✓ Phase 3: Link every entry-holding surface through `libraryLink` - Fully
implemented
✓ Phase 4: Find each activity row's document in the type listing - Fully
implemented

Every checkbox in the plan is ticked, and the code matches each Desired End
State bullet.

### Automated Verification Results

Run on revision `919c94d1`, the empty working copy on top of the last plan
commit.

✓ Full default task: `mise run` exited 0 in 224s, leaving the tree clean
✓ Frontend unit tests: 2766 of 2766 passed across 129 files
✓ Visualiser unit tests: 376 and 372 passed in its two lanes
✓ Visualiser e2e: 360 passed, including `library-links.spec.ts`
✓ No reference to `path-utils` remains:
`rg -n "path-utils|fileSlug" --glob '!**/testing/**' cli/visualiser/frontend`
prints nothing, and `api/path-utils.ts` is gone

### Code Review Findings

#### Matches Plan:

- `IndexEntry.id` comes from `document_id` (`server/src/indexer.rs:1613-1622`),
  and `plan_id_from_entry` reads it. The work-item `raw_id` path is
  unchanged.
- `SearchResultRow` carries `id` and a forward-slashed `relPath`
  (`server/src/api/search.rs:32,35`).
- `library-document-address.ts` exports `documentIdOf`, `libraryLink`,
  `libraryHref`, `resolveLibraryDocument` and `descriptiveSlugOf` as
  specified. `filenameStemOf` is private, and the module does not import
  `router.ts`.
- All eight call sites link through `libraryLink`. Related items render a
  router `<Link>`, and wiki links use `libraryHref`.
- Search results and not-found suggestions are keyed by `relPath`, which
  removes the old shared-slug key collisions.
- The activity feed looks rows up by `relPath` through
  `use-activity-row-documents.ts`. An unlinked row is a plain `div`, and the
  hover rules sit on `.linkedRow` only.
- The boundary rules and production scan match the plan, including the glob
  self-checks. No other module under `frontend/src` builds a
  `/library/<type>/<document>` target. The remaining `/library/` strings are
  type-level only.

#### Deviations from Plan:

The phases' Implementation Notes record most deviations. These are not
recorded:

- `LibraryTypeView.test.tsx:533-540` adds a separate test instead of
  extending the child-document test. It is named "renders the document in
  place of the listing" but only asserts the empty-listing heading is absent.
- `fetch.test.ts:87-96` adds a test that a wire entry without `id`
  normalises to `null`. This is an improvement.
- `DevDesignSystem.tsx:943` and `resolve-drop-outcome.test.ts:12` gained
  `id: null` literals.
- The `start-server.mjs:73` comment mentions the wider fixture snapshot.
- The Rust work-item test asserts `id == "42"` but never compares it with
  `work_item_id`, so it does not show the independence the plan names.
- In the activity-feed live-row test, `mountWith` replaces the first
  `fetchDocs.mockImplementation`, so that first mock does nothing.

#### Potential Issues:

- Stale comment: the `missingDocumentId` prop's doc comment in
  `routes/library/recovery/NotFoundSurface.tsx:12-14` still calls it "the
  missing document slug". This breaks the repository's comment policy.
- Stale listing: during the refetch after a delete event, `useQueries`
  still returns the old listing, so the row stays linked briefly. A click in
  that window opens "Document not found".
- Windows paths: `IndexEntry.rel_path` is a `PathBuf` serialised as-is, while
  activity events normalise to `/`. On Windows the activity lookup would miss
  and every row would render unlinked. `filenameStemOf` splits on `/` only;
  that predates this plan.
- Boundary rule reach: the stem-helper rule matches only `function`,
  `const`, `let` and `var` declarations. A class or object member of that
  name would pass.
- Untested ids: no test pins how `interpolatePath` and the router's param
  decoding handle an `id` containing `/`. Validation conventions make such an
  id unlikely.
- Related items from `relates_to`: the visualiser draws no declared relation
  from `relates_to`. A related set appears only when it shares the plan's
  lifecycle cluster. PP-1003 covers this.

### Manual Testing Required:

The implementing agents ran every manual check and all six Manual Testing
Steps in headless Chromium against throwaway repos and recorded them in the
plan. This validation did not re-run them. A person should still confirm the
hover and focus feel in a real browser:

1. Activity feed:
  - [ ] Edit a second topic-research set's manifest under `mise run dev`,
        and follow its activity row to that set
  - [ ] Delete a scratch plan, and check its row has no hover or focus
        affordance once the listing refetches

### Recommendations:

Fixed alongside this report:

- The `missingDocumentId` doc comment now describes a document id.
- The `LibraryTypeView` child-document test is renamed "child-doc URL hides
  the listing". The test router renders no child route, so it cannot assert
  that the document renders.
- The Rust work-item test asserts that `id: "42"` survives while the
  numeric scheme rejects it as a `work_item_id`, which shows the two are
  independent.
- The dead `fetchDocs` mock in the live-row test is gone.

The Windows path mismatch needs no action, because the visualiser does not
support Windows.
