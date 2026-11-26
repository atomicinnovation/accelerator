---
type: "codebase-research"
id: "2026-11-24-PP-996-library-links-to-sets-and-inventories-resolve-wrong"
title: "Why links to a topic-research set or design inventory open the wrong one, and what a fix touches"
date: "2026-11-24T00:27:22+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "PP-996"
parent: "work-item:PP-996"
topic: "Why links to a topic-research set or design inventory open the wrong one, and what a fix touches"
tags: ["research", "codebase", "visualiser", "frontend", "library-links", "topic-research", "design-inventories", "activity-feed", "lifecycle"]
revision: "c7d80a5677b676f2ba20ef1a9f6f50b83186dd64"
repository: "accelerator"
last_updated: "2026-11-24T00:27:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Why links to a topic-research set or design inventory open the wrong one, and what a fix touches

**Date**: 2026-11-24T00:27:22+00:00
**Author**: Toby Clemson
**Git Commit**: c7d80a5677b676f2ba20ef1a9f6f50b83186dd64
**Branch**: detached `HEAD` (jj working copy)
**Repository**: accelerator

## Research Question

For PP-996: where does the bug come from, and what would a fix touch? In
the visualiser (`cli/visualiser/`, server and frontend), how do links from
the activity feed, related items and lifecycle views resolve to a
topic-research set or design inventory? Where does the lookup pick
whichever entry sorts first by path instead of the one named? Which tests
cover these paths today?

Decisions taken without a user to ask:
- The research was confined to PP-996's scope. The set model (PP-994) and
  member routes (PP-998) are mentioned only where they constrain this fix.
- GitHub permalinks were not added. The working copy is a detached jj
  revision that may not be pushed, so the references are local
  `file:line`s.
- Paths are relative to `cli/visualiser/frontend/src/` unless they start
  with `server/`, `cli/` or `frontend/`.

## Summary

The work item's diagnosis is confirmed by the code:

- **What is wrong.** Four call sites build the URL's `$fileSlug` from the
  entry document's filename stem, `fileSlugFromRelPath(relPath)`
  (`api/path-utils.ts:6-8`).
  - For nested-manifest types that stem is always `manifest` (topic
    research) or `inventory` (design inventories).
  - The four sites are the activity feed, related items, lifecycle views
    and wiki links.
- **Why the first set opens.** `LibraryDocView` resolves the URL with
  `entries.find(e => e.slug === fileSlug || fileSlugFromRelPath(e.relPath)
  === fileSlug)` (`routes/library/LibraryDocView.tsx:83-85`).
  - It searches the `/api/docs?type=` listing, which the server sorts by
    absolute path (`server/src/api/docs.rs:40`).
  - So `/library/topic-research/manifest` opens the set whose directory
    sorts first.
- **What already carries the right slug.**
  - The server gives each nested-manifest entry a slug derived from its
    **parent directory name** (`server/src/indexer.rs:1512-1521`).
  - Related items, lifecycle clusters and wiki links all hold full
    `IndexEntry`s that carry `slug`.
  - Activity events carry only a project-relative `path` equal to the
    entry's `relPath` (`server/src/watcher.rs:319-321`). Matching
    `event.path === entry.relPath` in `fetchDocs(event.docType)` is
    therefore feasible.
  - The docs query is already invalidated on every `doc-changed` SSE
    (`api/use-doc-events.ts:82,122-124`).
- **No server change is needed.**
- **Main risk for planning: the descriptive slug is not unique within a
  type.**
  - Moving the stem-based call sites onto `slug ?? stem` (the work item's
    "rule the type listing applies today") will break links that work
    today.
  - Plan reviews `…-foo-review-1.md` and `…-foo-review-2.md` both get slug
    `foo` (`cli/corpus/src/slug.rs:29-32,151-159`), and so do two dated
    plans with the same tail.
  - The type listing already has this defect for such documents, and
    this repo's own `meta/reviews/plans/` has 9 such collision groups.
  - Lifecycle clusters gather documents by slug, so a cluster's review-1
    and review-2 links would both open review-1.
  - The plan must settle this before committing to the builder rule.
- **Tests.**
  - No test, unit or e2e, links to a topic-research set or design
    inventory.
  - The committed fixtures already hold four topic-research sets and two
    inventories.
  - Several existing tests assert today's stem-form hrefs and will change.

## Detailed Findings

### The faulty call sites (stem-only links)

`fileSlugFromRelPath` returns the last path segment with `.md` stripped
(`api/path-utils.ts:6-8`).

1. **Activity feed**
   - `components/ActivityFeed/ActivityFeed.tsx:123` sets
     `const fileSlug = fileSlugFromRelPath(r.path)`.
   - `:127-130` renders it as
     `<Link to="/library/$type/$fileSlug" params={{ type: r.docType, fileSlug }}>`.
   - The row is an `ActivityEvent {action, docType, path, timestamp}`
     (`api/types.ts:222-227`) with no slug.
   - Its data comes from `fetchActivity(LIMIT)` with `LIMIT = 5` (`:45-48`)
     and from live SSE events merged in through
     `useDocEventsContext().subscribe` (`:60,69-99`).
   - It also queries `queryKeys.types()` for labels (`:53-56`).
2. **Related items**
   - `components/RelatedArtifacts/RelatedArtifacts.tsx:96` builds a plain
     string `href={`/library/${entry.type}/${fileSlugFromRelPath(entry.relPath)}`}`.
   - It is an `<a>`, not a router `Link`.
   - Entries are full `IndexEntry`s from `GET /api/related/<relPath>`
     (`api/fetch.ts:265-273`; server `server/src/api/related.rs:15-21,30-69`).
     They carry `slug`, but `fetchRelated` does not run `normaliseEntry`.
   - The display id at `:80-82` already uses
     `entry.workItemId || (entry.slug ?? fileSlugFromRelPath(entry.relPath))`,
     so id and href disagree today.
3. **Lifecycle views**
   - `routes/lifecycle/LifecycleClusterView.tsx:204` sets
     `const fileSlug = fileSlugFromRelPath(entry.relPath)`.
   - `:217-219` renders a `Link` with `params={{ type: entry.type, fileSlug }}`.
   - `:211` builds the display id the same way.
   - Entries come from `GET /api/lifecycle/<slug>`, normalised
     (`api/fetch.ts:205-227`).
   - **Inventories can appear here; topic research cannot.**
     - In `cli/corpus/src/cluster.rs`, DesignInventories has stage order 9
       (`:512`) and sets `has_design_inventory` (`:565`).
     - It is orphan-by-design but merges into a cluster through a bridge
       (`server/src/clusters.rs:855-880`).
     - TopicResearch gets `u8::MAX` (`:514`) and contributes nothing to
       completeness (`:556`).
   - This confirms the work item's surface table.
4. **Wiki links**
   - `api/wiki-links.ts:187-190`:
     `fileSlug = fileSlugFromRelPath(entry.relPath)` and
     ``href: `/library/${entry.type}/${fileSlug}` ``.
   - The href goes into an mdast link node
     (`components/MarkdownRenderer/wiki-link-plugin.ts:91,116-119`) and into
     a plain `<a>` in `FrontmatterTable.tsx:50`.
   - Only decisions and work items can be resolved:
     - `use-wiki-link-resolver.ts:36-43` queries just those two types.
     - `buildWikiLinkIndex` keeps only those types (`wiki-links.ts:138,145`).
     - The grammar is `ADR-(\d+)` / `WORK-ITEM-(<id>)` (`wiki-links.ts:24-26`).
   - No defect is reachable here for sets or inventories.

### The two correct call sites

- **Type listing:** `routes/library/LibraryTypeView.tsx:265-271` uses
  `fileSlug: entry.slug ?? fileSlugFromRelPath(entry.relPath)`, with
  `type` taken from the route.
- **Search results:** `components/Sidebar/SearchResultsPanel.tsx:67-70`
  uses `params={{ type: r.docType, fileSlug: r.slug }}`.
  - `SearchResult` is `{docType, title, slug: string, mtimeMs}`
    (`api/fetch.ts:242-247`).
  - The server drops slug-less entries from search:
    `let slug = entry.slug.clone()?;` (`server/src/api/search.rs:84-91`).

### Call sites outside the work item's six

These also build `/library/<type>/<fileSlug>` links. The work item's
"six call sites" and AC 9 don't list them:

- `routes/kanban/WorkItemCard.tsx:46,96-98`:
  `fileSlugFromRelPath(entry.relPath)` with `type: "work-items"`. Stem form.
  It never shows a set or an inventory.
- `routes/library/recovery/NotFoundSurface.tsx:76-78`:
  `params={{ type: s.type, fileSlug: s.slug }}`, where `s.slug` is
  `e.slug ?? fileSlugFromRelPath(e.relPath)`
  (`routes/library/recovery/use-nearby-slug-suggestions.ts:58`).
- `api/use-active-doc-relpath.ts:33-35` is a resolver, not a builder. It
  repeats `LibraryDocView`'s first-match predicate with the same
  `queryKeys.docs(type)` query. The file says it is deliberately not shared
  with `LibraryDocView` (`:14-16`).

The plan should decide whether these move onto the builder too. Moving
them is consistent with "every library link through one builder", but AC 9
does not require it.

### URL resolution and why the first entry by path wins

- **Route.** Defined at `router.ts:104-120`: `/library/$type` has a
  `parseParams` that redirects unknown types, with child `/$fileSlug`.
- **Query.** `LibraryDocView.tsx:76-81` runs
  `useQuery({ queryKey: queryKeys.docs(type), queryFn: () => fetchDocs(type!) })`.
  `fetchDocs` calls `GET /api/docs?type=` (`api/fetch.ts:144-150`).
- **Match.** At `:83-85`, `Array.find` takes the first entry matching
  *either* the descriptive slug or the filename stem.
- **Order.** The server sorts with
  `entries.sort_by(|a, b| a.path.cmp(&b.path))` on the absolute canonical
  path (`server/src/api/docs.rs:39-40`). Every entry shares the
  project-root prefix, so this is the same as `relPath` order.
- **Not found.** The not-found surface appears only when
  `!entry && entries.length > 0` (`:128-129`).
- **Out of scope.** The work item's assumptions keep
  `/library/topic-research/manifest` resolving to the first set after this
  fix. Making it not-found is PP-998's job.

### Server: index entries and slugs

- **`IndexEntry`.** Defined at `server/src/indexer.rs:143-190`, serialised
  camelCase.
  - `path` is absolute.
  - `relPath` is `path.strip_prefix(project_root)` (`:1580-1588`).
  - `slug` is `Option<String>`, which serialises as null.
  - The frontend type is `api/types.ts:125-167`, with
    `slug: string | null` at `:129`.
- **Nested-manifest types.**
  - `cli/corpus/src/doc_type.rs:165-171` maps DesignInventories →
    `inventory.md` and TopicResearch → `manifest.md`.
  - The slug source is the parent directory name plus `.md`
    (`server/src/indexer.rs:1512-1521`).
  - Only `<root>/<dir>/<manifest>` is listed
    (`server/src/file_driver.rs:258-319`). Dot-prefixed directories and
    directories without a manifest are skipped, and member files are never
    indexed.
- **Slug rules** (`cli/corpus/src/slug.rs:9-39`):

  | Doc type | Slug rule | Example |
  |---|---|---|
  | topic-research | directory name verbatim (`:34-36`) | `prompt-caching-economics` |
  | design-inventories | `strip_prefix_date_and_optional_id` (`:135-149`): needs a leading `YYYY-MM-DD-`, strips it and one ID-shaped segment; null without a date | `2026-05-06-140608-foo` → `140608-foo` (test at `server/src/indexer.rs:2886-2942`) |
  | plans, codebase-research, validations, notes, pr-descriptions, design-gaps, root-cause-analyses | same as design-inventories; undated files get null | `meta/plans/foo.md` → null, so stem `foo` |
  | decisions | strips `ADR-NNNN-` | `ADR-0001-x` → `x` |
  | work-items | the tail after the ID | `0007-parent-epic` → `parent-epic` |
  | plan-reviews, pr-reviews | the dated rule, then strips `-review-N`; null without the suffix (`:29-32,151-159`) | |
  | work-item-reviews | `:61-75` | |
  | templates | always null | |

- **Wire strings.** `topic-research` and `design-inventories`
  (`cli/corpus/src/doc_type.rs:174-199`).

### Server: activity events

- **Shape.** `ActivityEvent { action, docType, path, timestamp }`
  (`server/src/activity_feed.rs:28-35`), projected from
  `SsePayload::DocChanged` (`:43-60`).
- **`path` is project-relative.** It is built as
  `rel_of(entry) = entry.rel_path…replace('\\', "/")`
  (`server/src/watcher.rs:319-321`), so it equals the entry's `relPath`.
  The PATCH-frontmatter broadcast uses `doc_rel` as well
  (`server/src/api/docs.rs:318-324`).
- **Where events come from.** Each event comes from diffing the index
  before and after a rescan (`server/src/watcher.rs:280-366`).
  - For a set or inventory, the path is always `<dir>/manifest.md` or
    `<dir>/inventory.md`.
  - A change to a set's member file emits **no** event, because the
    manifest's etag is unchanged.
  - Deletion emits `Deleted` with the old relPath (`:303-315`).

### What the activity-feed lookup can reuse

- **Query.** There is no `useDocs` wrapper. Consumers call `useQuery`
  inline with `queryKeys.docs(type)` (`["docs", type]`,
  `api/query-keys.ts:50`) and `fetchDocs(type)`.
  - Inline users: `LibraryDocView.tsx:76-81`, `LibraryTypeView.tsx:130-135`,
    `use-active-doc-relpath.ts:26-31` and `use-wiki-link-resolver.ts:36-43`.
- **Several types at once.** Rows can be of several doc types.
  `use-nearby-slug-suggestions.ts:20-36` already fans out over several
  `docs` queries with `useQueries`.
- **Freshness.** `dispatchSseEvent` invalidates `queryKeys.docs(event.docType)`
  on every doc-changed event (`api/use-doc-events.ts:82,122-124`).
  - A deleted document's entry therefore disappears from the refetched
    listing, and its row can lose its link as the work item requires.
  - A newly created document's row gains its link once the refetch lands.
- **Loading.** While a listing is loading there is no entry, so the row
  has no link. That matches the requirement.

## Code References

- `cli/visualiser/frontend/src/api/path-utils.ts:6-8`: `fileSlugFromRelPath`.
- `cli/visualiser/frontend/src/components/ActivityFeed/ActivityFeed.tsx:45-60,123-130`: activity rows and the stem link.
- `cli/visualiser/frontend/src/components/RelatedArtifacts/RelatedArtifacts.tsx:80-82,94-96`: related-items id and string href.
- `cli/visualiser/frontend/src/routes/lifecycle/LifecycleClusterView.tsx:204,211,217-219`: lifecycle link.
- `cli/visualiser/frontend/src/api/wiki-links.ts:24-26,138-162,186-191`: wiki-link grammar, index and href.
- `cli/visualiser/frontend/src/api/use-wiki-link-resolver.ts:36-43`: only decisions and work items are queried.
- `cli/visualiser/frontend/src/routes/library/LibraryTypeView.tsx:265-271`: the correct `slug ?? stem` link.
- `cli/visualiser/frontend/src/components/Sidebar/SearchResultsPanel.tsx:67-70`: the correct slug link.
- `cli/visualiser/frontend/src/routes/library/LibraryDocView.tsx:76-85,128-129`: URL resolution by first match.
- `cli/visualiser/frontend/src/api/use-active-doc-relpath.ts:18-36`: the duplicate first-match resolver.
- `cli/visualiser/frontend/src/routes/kanban/WorkItemCard.tsx:46,96-98`: a seventh stem-form link (work items only).
- `cli/visualiser/frontend/src/routes/library/recovery/NotFoundSurface.tsx:76-78`, `use-nearby-slug-suggestions.ts:20-36,58`: suggestion links (`slug ?? stem`) and the `useQueries` fan-out pattern.
- `cli/visualiser/frontend/src/api/types.ts:125-167,213-227`: `IndexEntry`, SSE and activity event types.
- `cli/visualiser/frontend/src/api/query-keys.ts:50,73`: `docs` and `disabled` keys.
- `cli/visualiser/frontend/src/api/use-doc-events.ts:82,106-148`: SSE invalidation of `docs(type)`.
- `cli/visualiser/server/src/api/docs.rs:30-42`: type listing, sorted by path.
- `cli/visualiser/server/src/indexer.rs:1502-1552,1580-1588`: `build_entry` slug source and relPath.
- `cli/visualiser/server/src/watcher.rs:280-366,319-321`: event derivation and relative path.
- `cli/visualiser/server/src/activity_feed.rs:28-60`: activity event.
- `cli/visualiser/server/src/api/search.rs:84-91`: search drops slug-less entries.
- `cli/corpus/src/slug.rs:9-39,135-159`: slug rules.
- `cli/corpus/src/cluster.rs:216-222,505-516,548-566`: lifecycle stage order; inventories in, topic research out.
- `cli/corpus/src/doc_type.rs:165-199`: manifest filenames and wire strings.

## Tests Covering These Paths Today

No test links to a topic-research set or a design inventory, and none
uses two of either and then checks their links.

**Frontend unit tests (Vitest)**

- **`ActivityFeed.test.tsx`.** No href assertions at all (tests at
  `:111-250`, with paths like `meta/plans/foo.md`). The link is untested.
- **`RelatedArtifacts.test.tsx:152-189`.** `it("each row links to
  /library/{type}/{slug}")` asserts stem hrefs:
  - `/library/plans/2026-04-18-foo` (`:164`)
  - `/library/plan-reviews/2026-04-18-foo-review-1` (`:167`)
  - `/library/decisions/ADR-0001-example` (`:172`)
  - `/library/root-cause-analyses/2026-06-10-example-rca` (`:189`)
  - **All four change under the builder.**
- **`LifecycleClusterView.test.tsx`.**
  - `:166` asserts `/library/plans/2026-04-18-foo`. This changes.
  - `:263` uses a flat `meta/design-inventories/2026-05-01-foo.md`, not
    the real `<dir>/inventory.md` layout.
- **Wiki-link tests.** These hrefs all change.
  - `wiki-links.test.ts:231,239,255,258,271`: `/library/decisions/ADR-0017-foo`,
    `/library/work-items/0001-foo` and `/library/work-items/PROJ-0042-foo`.
  - `use-wiki-link-resolver.test.tsx:59`.
  - `components/MarkdownRenderer/wiki-link-plugin.test.ts`: `link.url` at
    `:69,210,228`.
- **`LibraryTypeView.test.tsx`.** No href assertions. `:381-407` renders
  one topic-research entry with slug `prompt-caching-economics`.
- **`SearchResultsPanel.test.tsx:120-123,195`.** Slug hrefs. Unaffected.
- **`LibraryDocView.test.tsx`.** Resolution is exercised through the
  `fileSlug` prop. Href assertions:
  - `:278`: `/library/decisions/ADR-0001-example`
  - `:484`: `a[href$="/library/work-items/0041-page-wrapper"]`, from the
    frontmatter table's wiki-link hrefs. These change.
- **`path-utils`.** No dedicated test file. The fallback is covered
  indirectly by `use-active-doc-relpath.test.tsx:107` and
  `recovery/rank-slug-suggestions.test.ts:127`.

**Playwright e2e** (`frontend/e2e/`)

- **Fixture.** It runs against `server/tests/fixtures/meta`
  (`start-server.mjs:27,75,99`).
- **`cross-refs.spec.ts:19-23,73`.** Says outright that related links use
  the filename form (`PARENT_FILE_SLUG = "0007-parent-epic"`) and asserts
  it. **This changes** to `/library/work-items/parent-epic`.
- **`wiki-links.spec.ts:10,29` and `navigation.spec.ts:56`.** Use
  `href*=` prefix selectors, which are probably robust to the change.
- **No spec mentions topics, manifests or inventories.**

**Rust**

- `server/tests/api_docs.rs`: no test asserts sort order, and `docs.rs`
  has no inline tests.
- `server/src/activity_feed.rs:127-209`: ring-buffer tests only.
- `server/src/indexer.rs:2886` (two inventories) and `:2945` (four
  committed topic-research sets, slug = directory name) cover slug
  derivation.

**Fixtures available for the new tests**

- `server/tests/fixtures/meta/research/topics/` has four sets:
  `ac2-coverage`, `mcp-transport-layer`, `model-context-protocol` and
  `prompt-caching-economics`. A skipped `.draft-subject.tmp` is also there.
- `.../research/design-inventories/` has two inventories:
  `2026-05-26-ac2-coverage` (slug `ac2-coverage`) and `2026-05-26-example`
  (slug `example`).
- The e2e server already serves both, so a two-set e2e scenario needs no
  new fixture.

## Architecture Insights

- **Two meanings of "slug".**
  - The server's `slug` is a *descriptive* stem with the date, ID,
    `ADR-NNNN-` and `-review-N` parts stripped. It is used for clustering
    and display, can be null, and is **not unique within a type**.
  - The filename stem is unique per directory but useless for
    nested-manifest types.
  - The resolver accepts either, first match wins, so URL identity is
    ambiguous whenever two entries share a slug, or one entry's slug
    equals another's stem.
- **Link mechanisms vary.** Links are built two ways: TanStack `Link` with
  `to`/`params`, and plain string hrefs in `RelatedArtifacts` and
  wiki-links (markdown and `FrontmatterTable`).
  - A builder returning `{ type, fileSlug }` params serves the `Link`
    sites. The string sites need the assembled `/library/<type>/<slug>`.
  - The plan should choose one shape, or provide both from the same
    module.
- **No server work.** The frontend already holds everything it needs:
  related, lifecycle and wiki entries carry `slug`, and the activity
  lookup can use the shared `docs(type)` cache.

## Risks and Surprises for Planning

1. **Moving stem links to `slug ?? stem` makes some links that work today
   ambiguous.**
   - Same-type documents share a slug: plan-review / pr-review /
     work-item-review `-review-1`/`-review-2`, two dated plans or research
     docs with the same tail, and two design inventories with the same
     tail after the date.
   - Today related-items and lifecycle links to review-2 open review-2.
     Under the builder they would open review-1.
   - Lifecycle clusters group documents *by* slug, so this is the common
     case there, not an edge case.
   - This repo's `meta/reviews/plans/` has 9 slug groups with more than
     one review.
   - The type listing has this defect today.
   - AC 6 ("equal its type-listing row's link") would lock the regression
     in.
   - Options for the plan:
     - Make the builder prefer the stem except for nested-manifest types.
       But AC 10 forbids naming those types in the builder; the builder
       could key on "entry path's filename is a fixed manifest name"
       supplied by the caller or by the doc-type metadata.
     - Have the builder emit the slug only when it is unique within the
       listing.
     - Accept the regression and raise it with the author, since it
       touches AC 6 and AC 7.
   - **This needs an author decision before the plan is fixed.**
2. **Many existing tests assert stem hrefs** and will change:
   `RelatedArtifacts.test.tsx:164-189`, `LifecycleClusterView.test.tsx:166`,
   the wiki-link tests, `LibraryDocView.test.tsx:484` and
   `e2e/cross-refs.spec.ts:73`.
3. **Activity rows can be of several doc types.** The lookup needs a
   `docs(type)` query per distinct type among at most 5 rows (the
   `useQueries` pattern). The tests then need `fetchDocs` mocked
   alongside `fetchActivity`.
4. **Builder input.** The builder takes type, slug-or-null and path. A
   search row has no `relPath`, but its slug is never null, so it can pass
   `null` or omit the path.
5. **Extra call sites.** `WorkItemCard`, `NotFoundSurface` and
   `use-active-doc-relpath` are outside the six. Decide whether
   `WorkItemCard` and `NotFoundSurface` adopt the builder. Changing
   `WorkItemCard` would change kanban hrefs, with the same slug-uniqueness
   caveat for work items.
6. **Member-file edits emit no activity event.** Only edits to
   `manifest.md` or `inventory.md` produce a row. That is fine for the
   acceptance criteria, which change the manifest on disk, but an e2e test
   must edit the manifest itself.
7. **The lifecycle test fixture uses a flat inventory path.**
   `LifecycleClusterView.test.tsx:263` uses
   `meta/design-inventories/2026-05-01-foo.md`. New inventory tests should
   use `<dir>/inventory.md` with an explicit `slug`.

## Historical Context

- **Work item and reviews.**
  - `meta/work/PP-996-links-to-a-research-set-or-design-inventory-open-the-wrong.md`
    is the work item.
  - Its review, `meta/reviews/work/PP-996-…-review-1.md`, ended in the
    author's approval after two REVISE passes.
  - It settled: one builder at six call sites; the activity lookup is
    permanent in either landing order against PP-994; deleted rows have no
    link.
- **Related work items.**
  - `meta/work/PP-994-generic-set-model-indexing-and-api.md` and
    `meta/work/PP-998-set-member-routes-and-view.md`: neighbours.
  - PP-998 extends the builder and makes `/manifest` not-found.
  - `meta/work/PP-862-topic-research-visualiser-doc-type.md` registered
    the doc type.
- **Research and plans.**
  - `meta/research/codebase/2026-09-10-PP-862-topic-research-visualiser-doc-type-indexer.md`
    and `meta/plans/2026-09-10-PP-862-topic-research-visualiser-doc-type.md`
    cover how nested-manifest indexing was introduced.
  - `meta/research/codebase/2026-06-01-lifecycle-clustering-slug-mismatch.md`
    and `meta/plans/2026-06-01-lifecycle-clustering-composite-key.md` are
    earlier slug-versus-filename trouble in clustering, relevant to risk 1.
  - `meta/plans/2026-04-22-meta-visualiser-phase-5-frontend-scaffold-and-library-view.md`
    is the origin of the `/library/$type/$fileSlug` route.
  - `meta/plans/2026-04-27-meta-visualiser-phase-9-cross-references-and-wiki-links.md`
    is the origin of the wiki-link href shape, and of the stem choice
    ("preserved exactly").
- **Decisions.** `meta/decisions/ADR-0068-general-slug-resolution-in-the-corpus-cli.md`
  covers slug resolution in the corpus CLI.

## Related Research

- `meta/research/codebase/2026-05-13-PP-77-sidebar-activity-feed.md`
- `meta/research/codebase/2026-06-05-PP-101-aside-region-redesign.md`
- `meta/research/codebase/2026-06-01-PP-76-sidebar-search.md`
- `meta/research/codebase/2026-06-12-PP-109-error-screen-affordances.md`

## Open Questions

- **Slug uniqueness (risk 1).** Should the builder's rule be `slug ??
  stem` as written, accepting ambiguous links for same-slug documents? Or
  should it fall back to the stem whenever the descriptive slug is not
  unique, or is not a directory-derived slug? This touches AC 6 and
  AC 10.
- **Extra call sites.** Should `WorkItemCard` and `NotFoundSurface` move
  onto the builder?
- **Builder shape.** Should it return router params, an href string, or
  both?
