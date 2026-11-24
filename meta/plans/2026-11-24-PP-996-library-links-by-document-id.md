---
type: "plan"
id: "2026-11-24-PP-996-library-links-by-document-id"
title: "Library Links by Document ID Implementation Plan"
date: "2026-11-24T08:22:04+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "draft"
work_item_id: "work-item:PP-996"
parent: "work-item:PP-996"
derived_from: ["codebase-research:2026-11-24-PP-996-library-links-to-sets-and-inventories-resolve-wrong"]
tags: ["visualiser", "frontend", "server", "library-links", "topic-research", "design-inventories"]
revision: "83417415e7968e08d76e1b6aa8cf2c330dc34d59"
repository: "accelerator"
last_updated: "2026-11-24T08:22:04+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Library Links by Document ID Implementation Plan

## Overview

Every visualiser link to a library document is keyed on the document's
frontmatter `id`, falling back to its filename's stem, and built by one link
builder. Every library URL is resolved by one resolver that matches `id`,
then slug, then stem. This fixes links to topic-research sets and design
inventories, which today all point at `/library/<type>/manifest` or
`/library/<type>/inventory` and open whichever entry sorts first by path
(PP-996). It also fixes links to documents that share a descriptive slug,
such as `x-review-1` and `x-review-2`.

## Current State Analysis

- Four sites build the link from the filename stem through
  `fileSlugFromRelPath` (`cli/visualiser/frontend/src/api/path-utils.ts:6-8`):
  the activity feed (`components/ActivityFeed/ActivityFeed.tsx:123-130`),
  related items, which build a string `href`
  (`components/RelatedArtifacts/RelatedArtifacts.tsx:94-97`), lifecycle
  views (`routes/lifecycle/LifecycleClusterView.tsx:204,217-219`) and wiki
  links (`api/wiki-links.ts:186-191`). Kanban cards do the same for work
  items (`routes/kanban/WorkItemCard.tsx:46,96-98`).
- Three sites link by slug: the type listing
  (`routes/library/LibraryTypeView.tsx:265-271`), search results
  (`components/Sidebar/SearchResultsPanel.tsx:67-70`) and not-found
  suggestions (`routes/library/recovery/NotFoundSurface.tsx:76-78`, fed by
  `use-nearby-slug-suggestions.ts:58`). A slug is not unique within a doc
  type.
- `LibraryDocView` (`routes/library/LibraryDocView.tsx:83-85`) and
  `useActiveDocRelPath` (`api/use-active-doc-relpath.ts:33-35`) each resolve
  the URL with one `find` whose predicate is "slug or stem matches", over a
  listing sorted by path (`cli/visualiser/server/src/api/docs.rs:39-40`).
- The route is `/library/$type/$fileSlug` (`router.ts:116-120`), mirrored by
  the test router (`src/test/router-helpers.tsx:32-36`).
- The server's `IndexEntry` (`server/src/indexer.rs:143-190`) carries no
  document `id`; `build_entry` (`indexer.rs:1502-1600`) reads the frontmatter
  `id` only for work items. `SearchResultRow` (`server/src/api/search.rs:29-35`)
  carries `docType`, `title`, `slug` and `mtimeMs`, and no path.
- An `ActivityEvent` is `{action, docType, path, timestamp}` with `path`
  equal to the entry's `relPath` (`server/src/watcher.rs:319-321`).
  `dispatchSseEvent` invalidates `queryKeys.docs(docType)` on every
  doc-changed event (`api/use-doc-events.ts:82,122-124`). Templates are
  never indexed (`indexer.rs:335`), so no activity row is a template.
- Frontmatter validation already requires a quoted `id`
  (`cli/corpus/src/frontmatter_validation/template_shape.rs:306`) and reports
  duplicates (`violation.rs:127`), so a string `id` unique within a type is
  the corpus's own contract.
- The e2e fixtures' four topic-research manifests and two design inventories
  (`server/tests/fixtures/meta/research/{topics,design-inventories}/`) carry
  no `id`. Their decisions carry `adr_id` only; their work items carry
  `id: "0007"` and so on.

## Desired End State

- `IndexEntry` and `SearchResultRow` each carry `id: string | null`, the
  document's frontmatter `id`; the search row also carries `relPath`.
- One module, `cli/visualiser/frontend/src/api/library-document-address.ts`,
  owns library addressing. It exports:
  - `documentIdOf(document)`, the document's `id`, or its filename's stem
    when it has none: the segment every link carries;
  - `libraryLink(document)`, returning
    `{ to: "/library/$type/$documentId", params: { type, documentId } }`
    with `documentId` from `documentIdOf`;
  - `libraryHref(link)`, rendering that target with the router's
    `interpolatePath`;
  - `resolveLibraryDocument(entries, documentId)`, matching `id`, then slug,
    then stem, each as a separate pass over the listing;
  - `descriptiveSlugOf(entry)`, the `slug ?? stem` that related items show
    and not-found suggestions rank by. It is a human-facing label, never an
    address; links always carry `documentIdOf`.
- All eight call sites link through `libraryLink`; related items render a
  router `<Link>`; wiki links use `libraryHref`.
- The route parameter is `$documentId`.
- The activity feed finds each row's entry in the type listing for the
  row's doc type by `relPath`, and renders an unlinked row when the listing
  holds no such entry.
- `path-utils.ts` is gone; the stem helper is private to the address module.
- A boundary test fails if any production module other than the address
  module assembles a `/library/<type>/<document>` target, defines a stem
  helper or compares a URL segment against an entry, or if the address
  module names `TopicResearch`, `DesignInventories`, `topic-research` or
  `design-inventories`.

Verify with `mise run` (the bare default task) exiting 0, plus the manual
steps under Testing Strategy.

### Key Discoveries:

- `interpolatePath` is exported by `@tanstack/react-router` 1.168
  (`node_modules/@tanstack/react-router/dist/esm/index.d.ts:1`), so
  `libraryHref` needs no router instance and the address module does not
  import `router.ts`, which avoids an import cycle.
- `plan_id_from_entry` (`indexer.rs:1626-1635`) already reads a plan's `id`
  the way the new field does: as a string, with an empty string meaning none.
- The activity-row lookup can follow the `useQueries` fan-out in
  `routes/library/recovery/use-nearby-slug-suggestions.ts:28-34`.
- e2e specs that edit fixtures on disk restore them in `finally`
  (`frontend/e2e/kanban-conflict.spec.ts`). The global snapshot covers only
  `meta/work/` (`e2e/global-setup.ts:17-26`), so Phase 4 extends it.

## What We're NOT Doing

- Making `/library/topic-research/manifest` not-found, or routes for set
  members (PP-998). The stem pass keeps that URL resolving to the first set.
- The set model (PP-994), or any change to activity events. Rows keep being
  identified by path.
- Enforcing `id` uniqueness in the visualiser. Validation reports
  duplicates; the resolver opens the first match by path.
- Falling back to a slug for a set or an inventory that has no `id`. Such a
  document links by stem, as the work item specifies, and is invalid under
  frontmatter validation.
- Showing slug-less documents in search. The projector's
  slug-required rule (`search.rs:84-91`) stays.
- Changing how suggestions are ranked, or what ids related items and
  lifecycle cards display (lifecycle shows `documentIdOf(entry)` where it
  showed the stem; that equals the stem for any document without an `id`).
- Reading non-string `id` values. Validation rejects an unquoted `id`, so
  the field is `None` for one.
- Guaranteeing that a stem link round-trips for a document with no `id`.
  Its stem can equal another document's slug, and the slug pass then opens
  that document. Validation requires an `id`, so only an invalid corpus is
  exposed.

## Implementation Approach

Four phases, each mergeable on its own with `mise run` green:

1. The server adds the fields. The frontend ignores them.
2. URL resolution moves into the resolver and the route parameter is
   renamed. Links are unchanged, but an `id` URL now opens.
3. Every site that holds an entry or a search row links through the
   builder. This fixes related items, lifecycle and the slug collisions.
4. The activity feed's lookup fixes the last reachable defect. The stem
   helper becomes private, and the boundary test locks the rule in.

Each phase is test-first: write the failing behaviour test, make it pass,
then refactor.

---

## Phase 1: Carry the document's `id` on index entries and search rows

### Overview

Additive wire fields, so `id` is read on the server once instead of out of
the frontmatter by each client.

### Changes Required:

#### 1. Failing tests first

**File**: `cli/visualiser/server/src/indexer.rs` (inline tests)

- A topic-research set whose `manifest.md` has `id: "prompt-caching"` gets
  an entry with `id == Some("prompt-caching")`.
- A design inventory `2026-05-06-135214-current-app/inventory.md` with that
  `id` gets the directory name as `id`.
- A plan with no `id` gets `None`; one with `id: ""` gets `None`.
- A work item's `id` is its raw frontmatter string, independent of
  `work_item_id`'s normalisation.

**File**: `cli/visualiser/server/tests/api_search.rs`

- A search hit on a dated plan whose frontmatter has an `id` returns a row
  with `"id"` and `"relPath"`. A hit on one without an `id` returns
  `"id": null` and its `relPath`.

**File**: `cli/visualiser/server/tests/api_docs.rs`

- `/api/docs?type=plans` serialises `"id"` on each entry.

#### 2. `IndexEntry.id`

**File**: `cli/visualiser/server/src/indexer.rs`

Add `pub id: Option<String>` to `IndexEntry`. Fill it in `build_entry` from
the parsed frontmatter:

```rust
fn document_id(state: &FrontmatterState) -> Option<String> {
    match state {
        FrontmatterState::Parsed(map) => map
            .get("id")
            .and_then(|v| v.as_str())
            .filter(|id| !id.is_empty())
            .map(str::to_owned),
        _ => None,
    }
}
```

In the refactor step, have `plan_id_from_entry` read `entry.id`. Leave the
work-item branch's `raw_id` reading the raw frontmatter value: it passes an
empty `id` on to `work_item_identity`, which `document_id` would filter, so
folding it onto the helper would change work-item identity.

Add `id: None` to every `IndexEntry` literal: `src/test_support.rs` (both
factories), the `indexer.rs` test literals in `sample_entry_with_work_item`
and `sample_entry_without_work_item`, and `tests/parity.rs:184`.
`src/clusters.rs:242` builds its entries through `entry_for_test`, and the
third `indexer.rs` literal spreads `..sample_entry_with_work_item(…)`, so
neither needs the field.

#### 3. `SearchResultRow.id` and `relPath`

**File**: `cli/visualiser/server/src/api/search.rs`

Add `id: Option<String>` and `rel_path: String` to `SearchResultRow`, and
fill them in `project` from `entry.id` and
`entry.rel_path.to_string_lossy().replace('\\', "/")`. The path lets the
builder fall back to the stem for a row without an `id`, as it does for an
index entry.

### Success Criteria:

#### Automated Verification:

- [x] New and existing visualiser tests pass: `mise run test:unit:visualiser`
      and `mise run test:integration:visualiser`
- [x] Server lint and format pass: `mise run server:check`
- [x] Full default task passes: `mise run`

#### Manual Verification:

- [x] `curl localhost:<port>/api/docs?type=topic-research` against a repo
      with sets shows each set's `id`. Checked against a scratch repo whose
      two set manifests and one inventory carry a quoted `id`, because the
      e2e fixtures gain their `id`s only in Phase 3: each set and the
      inventory served its `id`, and a search row served `id` and `relPath`.

---

## Phase 2: Resolve library URLs by `id`, slug, then stem

### Overview

One resolver behind both URL consumers, and the route parameter renamed to
`$documentId`. Links stay as they are, but a URL holding a document's `id`
now opens it.

### Changes Required:

#### 1. Frontend types

**File**: `cli/visualiser/frontend/src/api/types.ts` and `api/fetch.ts`

- Add `id: string | null` to `IndexEntry` (`types.ts`). Normalise a missing
  wire `id` to `null` in `normaliseEntry`, to keep `IndexEntry` consistent
  with its other late fields, and
  add it to the `WireIndexEntry` omit list.
- Add `id: string | null` and `relPath: string` to `SearchResult`
  (`fetch.ts:242`). The server embeds its own frontend, so search rows
  always come from the same build and need no older-server normalisation.
- Add `id: null` to `makeIndexEntry` (`api/test-fixtures.ts`).

#### 2. Failing resolver tests

**File**: `cli/visualiser/frontend/src/api/library-document-address.test.ts`

- The URL segment matches an entry's `id`.
- The `id` wins over another entry whose slug equals it, even when the slug
  entry sorts first.
- The slug matches when no `id` does. The stem matches when neither does.
- The slug wins over another entry whose stem equals it, even when the stem
  entry sorts first.
- `review-2`'s stem opens `review-2` when `review-1` and `review-2` share a
  slug.
- No match returns `undefined`.

#### 3. The resolver

**File**: `cli/visualiser/frontend/src/api/library-document-address.ts` (new)

```ts
export function resolveLibraryDocument(
  entries: readonly IndexEntry[],
  documentId: string,
): IndexEntry | undefined {
  return (
    entries.find((e) => e.id === documentId) ??
    entries.find((e) => e.slug === documentId) ??
    entries.find((e) => filenameStemOf(e.relPath) === documentId)
  );
}
```

`filenameStemOf` is this module's own copy of `fileSlugFromRelPath`'s body.
`path-utils.ts` stays until Phase 4, which removes its last callers.

#### 4. Route parameter rename

- `router.ts`: `path: "/$documentId"`, crumb `params.documentId`.
- `src/test/router-helpers.tsx`: `/library/$type/$documentId`.
- `LibraryDocView`: the prop and param become `documentId`. Replace the
  `find` with `resolveLibraryDocument(entries, documentId)`. Rename
  `NotFoundSurface`'s `missingSlug` prop to `missingDocumentId` and pass
  `documentId`; ranking still compares it against descriptive slugs, so the
  ranking helpers in `rank-slug-suggestions.ts` keep their names. Update
  `NotFoundSurface.test.tsx` and `CatchAllNotFound.tsx` to the new name.
- `LibraryTypeView` (`:106-118,168`): its `useParams({ strict: false })`
  cast, `hasFileSlug` and the `<Outlet />` gate read `params.fileSlug`, and
  the cast hides the rename from the type checker. Read `params.documentId`
  and rename `hasFileSlug` to `hasDocumentId`. The existing child-document
  test (`LibraryTypeView.test.tsx:481`) pins `hasDocumentId` only; extend it
  to assert that the listing is absent at `/library/work-items/some-slug`,
  which pins the `<Outlet />` gate.
- `useActiveDocRelPath`: read `params.documentId` and call the resolver.
  Remove its doc comment's "intentionally not shared" paragraph, which the
  shared resolver makes false.
- Every `<Link to="/library/$type/$fileSlug" params={{ …, fileSlug }}>` (the
  activity feed, lifecycle, type listing, search, kanban and not-found
  suggestions) becomes `$documentId` / `documentId` with its value
  unchanged. Phase 3 replaces each with the builder.
- Update the tests that name the parameter or prop: `router.test.tsx:222`,
  `LibraryDocView.test.tsx`, `LibraryDocView.smoke.test.tsx`,
  `use-active-doc-relpath.test.tsx`, `Breadcrumbs.test.tsx`,
  `RootLayout.externalEdit.test.tsx`, `use-dev-activation.test.tsx`,
  `KanbanBoard.test.tsx` and `router-helpers.test.tsx`.

#### 5. Behaviour tests at the consumers

- `LibraryDocView.test.tsx`: one document is reached by its `id`, its slug,
  and its stem, and each URL shows its title.
- `use-active-doc-relpath.test.tsx`: the same three URLs each yield that
  document's `relPath`.

### Success Criteria:

#### Automated Verification:

- [x] Frontend unit tests pass: `mise run test:unit:frontend`
- [x] Frontend format, lint and types pass: `mise run frontend:check`
- [x] Full default task passes: `mise run`

#### Manual Verification:

- [ ] `/library/work-items/0007` in the e2e fixture server opens
      `0007-parent-epic`; `/library/work-items/parent-epic` and
      `/library/work-items/0007-parent-epic` still open it.

---

## Phase 3: Link every entry-holding surface through `libraryLink`

### Overview

The builder and its string form are added, and the seven sites that already
hold an entry or a search row adopt them. This fixes related items and
lifecycle links to sets and inventories, and every link to a document that
shares a slug.

### Changes Required:

#### 1. Failing builder tests

**File**: `cli/visualiser/frontend/src/api/library-document-address.test.ts`

- A set entry whose `id` is `b` and whose `relPath` is
  `meta/research/topics/b/manifest.md` gives
  `{ to: "/library/$type/$documentId", params: { type: "topic-research", documentId: "b" } }`.
- An entry with no `id` at `meta/plans/foo.md` gives `documentId: "foo"`.
- `documentIdOf` gives the `id`, or the stem when the `id` is null.
- `libraryHref` renders `/library/plans/foo`, and percent-encodes a
  reserved character in an `id`.
- `libraryHref(libraryLink(entry))` equals the `href` of a rendered
  `<Link {...libraryLink(entry)}>`, for a plain `id` and one with a reserved
  character, so a router upgrade cannot make the two forms drift.
- For every entry in a listing with unique `id`s,
  `resolveLibraryDocument(entries, documentIdOf(e))` returns `e`.
- `descriptiveSlugOf` gives the slug, or the stem when the slug is null.

#### 2. The builder

**File**: `cli/visualiser/frontend/src/api/library-document-address.ts`

```ts
export type LibraryDocument = Pick<IndexEntry, "type" | "id" | "relPath">;

export interface LibraryLink {
  to: "/library/$type/$documentId";
  params: { type: DocTypeKey; documentId: string };
}

export function documentIdOf(document: LibraryDocument): string {
  return document.id ?? filenameStemOf(document.relPath);
}

export function libraryLink(document: LibraryDocument): LibraryLink {
  return {
    to: "/library/$type/$documentId",
    params: { type: document.type, documentId: documentIdOf(document) },
  };
}

export function libraryHref(link: LibraryLink): string {
  return interpolatePath({ path: link.to, params: link.params })
    .interpolatedPath;
}

export function descriptiveSlugOf(
  entry: Pick<IndexEntry, "slug" | "relPath">,
): string {
  return entry.slug ?? filenameStemOf(entry.relPath);
}
```

#### 3. Call sites, each red first

Each site's test starts by asserting the new href and failing. Then
`<Link {...libraryLink(entry)}>` replaces the hand-built target.

- **Related items** (`RelatedArtifacts.tsx`). Render a router `<Link>` with
  the same class and `aria-label`. The display id becomes
  `entry.workItemId || descriptiveSlugOf(entry)`. `RelatedArtifacts.test.tsx`
  renders through `renderWithRouterAt`. New cases: two plans relating to
  sets `a` and `b` link to `/library/topic-research/a` and `/b`. Two plans
  targeting two inventories link to each inventory's `id`, which is its
  directory name. Two reviews sharing slug `foo` link to their own stems.
  A plan with no `id` at `meta/plans/foo.md` links to `/library/plans/foo`.
  Rewrite the four existing href assertions (`:164-189`) to the builder's
  output.
- **Lifecycle** (`LifecycleClusterView.tsx`). Use the builder. The display
  id becomes `entry.workItemId || documentIdOf(entry)`.
  New cases in `LifecycleClusterView.test.tsx`: two inventories at
  `meta/research/design-inventories/<dir>/inventory.md` with
  `id: "<dir>"` link to `/library/design-inventories/<dir>`. A plan
  without an `id` links to `/library/plans/<stem>`. Update the assertion at
  `:166`.
- **Wiki links** (`wiki-links.ts`). `resolveWikiLink` returns
  `href: libraryHref(libraryLink(entry))`, and its doc comment drops the
  stem rationale. A work item `id: "0042"` gives
  `/library/work-items/0042`. `[[ADR-0067]]` to a decision with
  `id: "ADR-0067"` gives `/library/decisions/ADR-0067`, and
  `LibraryDocView.test.tsx` opens that URL to the decision's title. A
  decision with no `id` keeps its stem. Update
  `wiki-links.test.ts:231-271`, `use-wiki-link-resolver.test.tsx:59`,
  `wiki-link-plugin.test.ts:69,210,228` and `LibraryDocView.test.tsx:278,484`.
- **Type listing** (`LibraryTypeView.tsx`). Use `libraryLink(entry)`. New
  test: two plan-reviews sharing slug `foo` link to their own stems, and a
  set with `id: "b"` links to `/library/topic-research/b`.
- **Search results** (`SearchResultsPanel.tsx`). Use
  `libraryLink({ type: r.docType, id: r.id, relPath: r.relPath })`, and key
  rows by `relPath`. Update the hrefs at `SearchResultsPanel.test.tsx:120-123,195`
  to rows carrying an `id`, and add a set row with `id: "b"`. Two rows
  sharing slug `foo` with different `id`s and `relPath`s both render, each
  with its own href.
- **Kanban** (`WorkItemCard.tsx`). Use `libraryLink(entry)`. In
  `KanbanBoard.test.tsx`, a card with `id: "0007"` links to
  `/library/work-items/0007`.
- **Not-found suggestions**. Add `id: string | null` to `SlugCandidate`
  (`rank-slug-suggestions.ts`), and in `use-nearby-slug-suggestions.ts`
  build each candidate with `id: e.id` and `slug: descriptiveSlugOf(e)`.
  Add `id: null` to the `cand()` defaults in
  `rank-slug-suggestions.test.ts:11-19`. `NotFoundSurface` links with
  `libraryLink(s)` and keys rows by `relPath`. Ranking is unchanged. Rewrite
  the `SlugCandidate` doc comment so it no longer names
  `fileSlugFromRelPath`. New tests, driven through `NotFoundSurface` from a
  mocked `fetchDocs` listing: a suggestion for an entry with `id: "x"` links to
  `/library/<type>/x`, and two suggestions sharing a slug both render, each
  linking to its own `id`.
- **Lifecycle shared slugs**. In `LifecycleClusterView.test.tsx`, two
  reviews in one cluster sharing a slug link to their own stems.

#### 4. e2e

**File**: `cli/visualiser/frontend/e2e/cross-refs.spec.ts`

- `PARENT_FILE_SLUG` becomes the parent's `id`, `0007`. Drop the comment
  that explains the filename form.

**File**: `cli/visualiser/server/tests/fixtures/meta/research/topics/*/manifest.md`
and `.../design-inventories/*/inventory.md`

- Add a quoted `id`: the directory name for each set and each inventory.
  These fixtures carry deliberately minimal frontmatter and are not
  template-shaped, so only the `id` is added.

**File**: `cli/visualiser/frontend/e2e/library-links.spec.ts` (new)

- The topic-research type listing links each set to
  `/library/topic-research/<dir>`, and following
  `mcp-transport-layer` shows its title.
- Searching `example` gives a design-inventory row linking to
  `/library/design-inventories/2026-05-26-example`.
- The plan-reviews listing links `2026-01-01-first-plan-review-1` and
  `-review-2` to their own stems, and the `-review-2` page shows the second
  review's text.

### Success Criteria:

#### Automated Verification:

- [ ] Frontend unit tests pass: `mise run test:unit:frontend`
- [ ] e2e passes: `mise run test:e2e:visualiser`
- [ ] Server tests still pass with the new fixture ids:
      `mise run test:unit:visualiser` and `mise run test:integration:visualiser`
- [ ] Each edited fixture's `id` is quoted and equals its directory name,
      which the `library-links.spec.ts` link assertions confirm
- [ ] Full default task passes: `mise run`

#### Manual Verification:

- [ ] In a repo with two topic-research sets, a plan whose `relates_to` names
      the second set links to it from its related items, and the link opens
      that set.
- [ ] A lifecycle cluster holding a design inventory links to that
      inventory's own page.

---

## Phase 4: Find each activity row's document in the type listing

### Overview

The activity feed, whose events carry a path and no `id`, looks up the entry
in the type listing. This fixes the last reachable defect. The stem helper
then moves fully inside the address module, and a boundary test keeps it
there.

### Changes Required:

#### 1. Failing tests first

**File**: `cli/visualiser/frontend/src/components/ActivityFeed/ActivityFeed.test.tsx`

Mock `fetchDocs` alongside `fetchActivity`, and render through
`renderWithRouterAt`.

- Rows for `meta/research/topics/a/manifest.md` and
  `meta/research/topics/b/manifest.md`, with entries whose `id`s are `a` and
  `b`, link to `/library/topic-research/a` and `/b`. The same holds for two
  inventories at `<dir>/inventory.md`.
- Rows of two doc types each find their entry in their own type's listing.
- A row for a plan with no `id` at `meta/plans/foo.md` links to
  `/library/plans/foo`.
- While the listing is pending, the row shows its text and has no link.
- A row whose path is absent from the settled listing shows its text and
  has no link. Run this for a deleted set and for a deleted plan.
- A live row for a new document gains its link once the listing refetches.
  Drive it through the real event path: deliver a doc-changed event through
  the subscribe and `dispatchSseEvent` machinery, with `fetchDocs` returning
  the new entry on refetch. `dispatchSseEvent` skips self-caused events, but
  those come from the visualiser's own writes to documents already in the
  listing, so no new entry depends on them.
- An unlinked row's list item holds its visible text (doc-type label,
  action, filename) and no `link`, and the row lacks the `.linkedRow`
  class.

**File**: `cli/visualiser/frontend/src/components/Sidebar/Sidebar.test.tsx`

- Mock `fetchDocs` alongside `fetchActivity`, so the feed's listing queries
  resolve in the sidebar tests.

#### 2. The lookup

**File**: `cli/visualiser/frontend/src/components/ActivityFeed/use-activity-row-documents.ts` (new)

```ts
export function useActivityRowDocuments(
  rows: readonly ActivityEvent[],
): (row: ActivityEvent) => IndexEntry | undefined {
  const docTypes = [...new Set(rows.map((r) => r.docType))].filter(
    isPhysicalDocTypeKey,
  );
  const listings = useQueries({
    queries: docTypes.map((type) => ({
      queryKey: queryKeys.docs(type),
      queryFn: () => fetchDocs(type),
    })),
  });
  const entriesByPath = new Map<string, IndexEntry>(
    listings.flatMap((listing) =>
      (listing.data ?? []).map((entry) => [entry.relPath, entry] as const),
    ),
  );
  return (row) => entriesByPath.get(row.path);
}
```

The shared `docs(type)` cache is already invalidated on every doc-changed
event, so a deleted document drops out and a new one appears without extra
wiring.

#### 3. Linked and unlinked rows

**File**: `cli/visualiser/frontend/src/components/ActivityFeed/ActivityFeed.tsx`

Extract the row body (glyph, two lines) into an `ActivityRowContent`
component. A row with an entry renders
`<Link {...libraryLink(entry)} className={`${styles.row} ${styles.linkedRow}`} aria-label=…>`.
One without renders the same content in a `<div className={styles.row}>`,
with no `aria-label`, which ARIA prohibits on a generic element; a screen
reader reads its text as part of the list item.

**File**: `cli/visualiser/frontend/src/components/ActivityFeed/ActivityFeed.module.css`

`.row:hover` and the `li:has(.row:hover)` border rules give every `.row`
hover feedback. Move those hover rules onto `.linkedRow`, and leave the
layout and the `.row [data-doc-type]` glyph rule on `.row`, so both rows
share layout and monochrome glyphs and only a linked row shows a hover or
focus affordance.

#### 4. Retire `path-utils.ts`

Delete `api/path-utils.ts`. Its last importers are the activity feed, which
now uses the builder, and any test that imported it.
`use-active-doc-relpath.test.tsx:107` and
`rank-slug-suggestions.test.ts:127` assert behaviour, not the helper;
rewrite any direct import as a behaviour assertion, and drop the comment at
`rank-slug-suggestions.test.ts:127` and any surviving `cross-refs.spec.ts`
comment that names `fileSlugFromRelPath`.

#### 5. Boundary test

**Files**: `cli/visualiser/frontend/src/api/testing/library-addressing-boundary.ts`
with its synthetic-source test beside it, and the production scan in
`cli/visualiser/frontend/src/api/library-addressing-boundary.test.ts`. This
follows `src/styles/testing/` (helpers and their tests) and
`src/styles/migration.test.ts` (the scan, beside the code it guards).

Write this first in the phase. After Phase 3 it fails on `ActivityFeed.tsx`'s
hand-built target and on `path-utils.ts`, and the rest of the phase turns it
green.

The rules module exports
`libraryAddressingViolations(sources: Record<string, string>): string[]`,
which applies the rules below to a map of path to source text. Its test
feeds it synthetic sources, one breaking each rule and one clean, and
asserts each breach is reported and the clean source is not. A source of
`` `/library/${dt.id}/` ``, the form `Sidebar.tsx:89-90` uses for a type
path, is clean.

The production scan reads the modules with
`import.meta.glob(…, { query: "?raw", import: "default", eager: true })`
over `src/**/*.{ts,tsx}`, excluding `**/*.test.{ts,tsx}`, `src/test/**`,
`**/testing/**` and `**/test-fixtures.ts`. The rules module holds the
patterns it hunts for, so it must sit under an excluded `testing/`
directory. The test asserts the glob found `library-document-address.ts` and
`ActivityFeed.tsx` and did not pick up the rules module, so a mis-scoped
glob cannot pass, and that
`libraryAddressingViolations` reports nothing. The rules, for every module
but `library-document-address.ts`:

- it contains neither `/library/$type/$` nor a template literal with two
  interpolated segments after `/library/`, matched by
  `` /`\/library\/\$\{[^}]*\}\/\$\{/ ``;
- it defines neither `filenameStemOf` nor `fileSlugFromRelPath`;
- it contains none of `.slug ===`, `=== documentId` and `documentId ===`.
  Today these appear only in the two resolvers this plan replaces. A broad
  `.id ===` pattern would catch unrelated code such as `SortPill.tsx:33`, so
  the rule does not use it. This rule is a textual approximation of "no
  other module matches a URL segment against an entry"; the resolver tests
  carry the behaviour.

And for `library-document-address.ts` itself: it contains none of
`TopicResearch`, `DesignInventories`, `topic-research` and
`design-inventories`.

Allow-list nothing.

#### 6. e2e

**File**: `cli/visualiser/frontend/e2e/library-links.spec.ts`

- Edit `research/topics/mcp-transport-layer/manifest.md`, then
  `model-context-protocol/manifest.md`, restoring both in `finally`. Each
  activity row then links to `/library/topic-research/<dir>`, and following
  it shows that set's title.
- Do the same for the two design inventories.
- Each edit appends a body line, leaving titles and frontmatter unchanged, so
  specs running alongside cannot observe it.

**File**: `cli/visualiser/frontend/e2e/global-setup.ts` and its teardown

Extend the fixture snapshot from `meta/work/` to also cover the four
`research/topics/*/manifest.md` and the two
`research/design-inventories/*/inventory.md` files, so an interrupted run
cannot leave an edited manifest behind.

### Success Criteria:

#### Automated Verification:

- [ ] Frontend unit tests, including the boundary test, pass:
      `mise run test:unit:frontend`
- [ ] e2e passes: `mise run test:e2e:visualiser`
- [ ] No reference to `path-utils` remains:
      `rg -n "path-utils|fileSlug" --glob '!**/testing/**' cli/visualiser/frontend`
      prints nothing
- [ ] Full default task passes: `mise run`

#### Manual Verification:

- [ ] With the visualiser running against a repo with two sets, editing the
      second set's manifest adds an activity row that opens the second set.
- [ ] Deleting a plan leaves its activity row as plain text once the
      listing refetches, with no hover or focus affordance.

---

## Testing Strategy

### Unit Tests:

- Rust: the `id` field from frontmatter (present, absent, empty, nested
  manifest, work item), and `id` and `relPath` on search rows.
- Address module: `documentIdOf`, the builder, `libraryHref` encoding and
  its parity with `<Link>`, the resolver's pass order, collisions and
  round trip, and `descriptiveSlugOf`.
- One behaviour test per call site through its rendered href, covering
  sets, inventories, shared slugs and documents with no `id`.
- Activity feed: lookup across types, pending and missing entries, and
  live rows.
- Boundary rules, tested against synthetic violating sources, then applied
  to the production modules for the work item's two structural criteria.

### Integration Tests:

- `api_search.rs` and `api_docs.rs` assert the wire fields.
- Playwright `library-links.spec.ts`: type listing, search, shared-slug
  reviews, and activity rows after on-disk edits to sets and inventories.
  `cross-refs.spec.ts` covers the related-items `id` link.

### Manual Testing Steps:

1. `mise run dev` against a repo with at least two topic-research sets
   and two design inventories.
2. Edit each set's `manifest.md`, and follow each activity row. Each opens
   its own set.
3. Open a plan whose `relates_to` names a set, and follow its related-items
   row.
4. Open a lifecycle cluster with an inventory, and follow its card.
5. Open `/library/topic-research/<id>`, `/<slug>` and `/manifest`. The
   first two open the set; `/manifest` still opens the first set.
6. Delete a scratch plan and check that its activity row loses its link.

## Performance Considerations

The activity feed adds at most one `docs(type)` query per distinct doc type
among five rows. Those queries share cache keys that the library views
already use, so they are usually warm. The resolver's three passes are
linear over one type's listing, as the single pass was.

## Migration Notes

Existing URLs keep working: slug and stem URLs resolve through the
resolver's later passes, unless another document of the same type has that
slug or stem as its `id`, in which case the `id` pass opens that document.
The corpus's id conventions (`ADR-NNNN`, `PP-NNN`, dated stems, directory
names) make such a collision unlikely. Likewise a stem URL whose stem
equals another document's slug now always opens the slug match, where the
single pass opened whichever sorted first. Bookmarked `/library/topic-research/manifest` URLs
keep opening the first set until PP-998. No data migration is needed.

## References

- Work item: `meta/work/PP-996-links-to-a-research-set-or-design-inventory-open-the-wrong.md`
- Research: `meta/research/codebase/2026-11-24-PP-996-library-links-to-sets-and-inventories-resolve-wrong.md`
- Follow-on: `meta/work/PP-998-set-member-routes-and-view.md`
- Related: `meta/work/PP-994-generic-set-model-indexing-and-api.md`
- Fan-out pattern: `cli/visualiser/frontend/src/routes/library/recovery/use-nearby-slug-suggestions.ts:28-34`
- Fixture-editing e2e pattern: `cli/visualiser/frontend/e2e/kanban-conflict.spec.ts`
