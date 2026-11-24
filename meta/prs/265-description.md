---
type: "pr-description"
id: "265"
title: "[PP-996] Link each library document by its id so sets and inventories open the right page"
date: "2026-11-24T18:58:45+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-996"
parent: "work-item:PP-996"
relates_to: ["work-item:PP-994", "work-item:PP-998", "work-item:PP-1003", "work-item:PP-1004"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/265"
pr_number: 265
tags: ["visualiser", "frontend", "server", "library-links", "topic-research", "design-inventories"]
revision: "7b24d4ff4162436df610299ff5dca569172a990b"
repository: "accelerator"
last_updated: "2026-11-24T22:17:59+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [PP-996] Link each library document by its id so sets and inventories open the right page

## Summary

Every visualiser link to a library document now carries the document's
published `id`, falling back to its filename stem. One resolver opens a
library URL by `id`, then stem, then slug. The server publishes an `id` only
while no other document of the same type shares it. A design inventory or
research set with no unique `id` is published under its directory name.
Before this PR, links to topic-research sets and design inventories pointed at
`/library/<type>/manifest` or `/library/<type>/inventory` and opened whichever
entry sorted first by path. Links to documents sharing a slug, such as two
reviews of one plan, had the same problem.

## Changes

- **Server, published id:** `IndexEntry` gains `id` (the published id, or
  `null`) and `frontmatter_id` (the document's own non-empty `id`, not
  serialised). `publish_document_ids` recomputes `id` for the whole index on
  `rescan` and on every `refresh_one` edit or removal. A duplicate's `id` is
  withdrawn while it is shared and comes back once the other copy changes.
  `raw_frontmatter_id` is now the one reader of the frontmatter `id`, for
  both the work-item identity and the published id. `plan_id_from_entry`
  reads `frontmatter_id`, so typed `plan:` links are unaffected.
- **Server, search rows:** `SearchResultRow` gains `id` and a
  forward-slashed `relPath`. Search rows and watcher events share
  `IndexEntry::served_rel_path()`. Both fields are additive.
- **Address module:** `frontend/src/api/library-document-address.ts` owns
  library addressing. It exports `documentIdOf`, `libraryLink`,
  `libraryHref` (via the router's `interpolatePath`),
  `resolveLibraryDocument`, `displayedIdentifierOf` and `descriptiveSlugOf`.
  The resolver checks stem before slug, so every link `documentIdOf` emits
  resolves back to the document it came from.
- **Route:** `/library/$type/$fileSlug` becomes `/library/$type/$documentId`.
  `LibraryDocView` and `useActiveDocRelPath` share the resolver, which runs
  three separate passes so an earlier pass wins whatever the listing order.
- **Call sites:** related items, lifecycle cards, wiki links, the type
  listing, search results, kanban cards and not-found suggestions all link
  through `libraryLink`. Related items now render a router `<Link>`. Related
  items and lifecycle cards both label a document with
  `displayedIdentifierOf`: its work item ID, otherwise the id its URL uses.
- **Search from older servers:** `fetchSearch` reads rows as
  `WireSearchResult` and normalises them. A row from a server that predates
  ids carries neither `id` nor `relPath`, so it is addressed by its slug, as
  before this PR. Rows key on type, path (or slug) and mtime.
- **Activity feed:** events carry a path and no `id`, so
  `use-activity-row-documents.ts` finds each row's entry by `relPath` in its
  doc type's listing. A row with no entry, while the listing loads or after
  a delete, is a plain `div` with no link and no hover affordance. PP-1004
  moves the id onto the event.
- **Boundary test:** `api/library-addressing-boundary.test.ts` scans
  production modules. It fails if any module but the address module builds a
  `/library/<type>/<document>` target, defines a stem helper, or compares a
  URL segment (`documentId`, or a slug against a `params.*` value) with an
  entry. `api/path-utils.ts` is deleted.
- **Fixtures and e2e:** the four topic-research manifests and two design
  inventories get a quoted `id` equal to their directory name. The new
  `e2e/library-links.spec.ts` covers listings, search, shared-slug reviews
  and activity rows after on-disk edits. Snapshot logic moves to
  `e2e/fixture-snapshot.ts`, which snapshots those six files and `meta/work/`.
  It refuses a snapshot naming any other file, before writing anything.
  Vitest now collects `e2e/*.test.ts`, and Playwright collects only
  `*.spec.ts`.
- **Template:** `templates/design-inventory.md` now asks for
  `{directory-name}` as the `id`, matching what `inventory-design` writes.

## Context

- Work item:
  `meta/work/PP-996-links-to-a-research-set-or-design-inventory-open-the-wrong.md`
- Research:
  `meta/research/codebase/2026-11-24-PP-996-library-links-to-sets-and-inventories-resolve-wrong.md`
- Plan: `meta/plans/2026-11-24-PP-996-library-links-by-document-id.md`
- Validation:
  `meta/validations/2026-11-24-PP-996-library-links-by-document-id-validation.md`
  (result: pass)
- Review: `meta/reviews/prs/265-review-1.md`. Every finding is addressed or
  deferred, and all review threads are resolved.
- Follow-up: PP-1004 puts each document's published id on activity events.
- Stacked on #262 (PP-995), which is stacked on #261 and #268.

## Testing

- [x] `mise run` exits 0 end to end (272s) at `ba511e27`: 2778 frontend
      unit tests, 385 and 381 visualiser unit tests, 8406 cli tests and 360
      e2e tests. An earlier run failed on 2 `github::octocrab_client` tests,
      which this PR does not touch. They timed out at about 16s under load
      and pass on their own in 4.6s.
- [x] Each review fix was test-first. Item 8's test (the child-doc URL hides
      an already-rendered listing) and item 9's test (template activity rows
      stay unlinked) were checked by removing the guard each one covers and
      seeing the test fail.
- [x] `rg -n "path-utils|fileSlug" --glob '!**/testing/**' cli/visualiser/frontend`
      prints nothing
- [x] Edge cases covered by unit tests:
  - an `id` beating another entry's stem or slug, and a stem beating a slug
  - every emitted link resolving back to its document, including a stem
    that equals another document's slug
  - ids shared within a type (withdrawn, or replaced by the directory name
    for manifests), shared across types (kept), and restored after an edit
    or delete
  - unquoted numeric and malformed frontmatter ids (not published)
  - a document whose id holds space, `?`, `#`, `/` and `%` opening at its
    `libraryHref`
  - search responses from a server that predates ids
  - activity rows that are pending, deleted, templates, or gain a link on a
    live refetch
- [x] Manual checks in headless Chromium against throwaway repos: every
      phase's manual criteria and all six of the plan's Manual Testing Steps
      (before the review response)
- [ ] Hover and focus feel of linked and unlinked activity rows in a real
      browser under `mise run dev`

## Notes for Reviewers

- **Old URLs mostly keep working.** Slug and stem URLs resolve through the
  later passes. There are two exceptions. A slug or stem that equals another
  document's `id` opens that other document. A slug that equals another
  document's stem opens the document with that stem. Both trade an old
  bookmark for making every link the app emits open the right page.
- **Documents whose `id` differs from their stem change canonical URL**
  (`ADR-0001`, `0042`). An unquoted numeric `id` is not published, so that
  document falls back to its stem without warning.
- **An edit can change another document's published `id`.** The edited
  document's `doc-changed` event refetches its type's whole listing, which
  carries the other document's new id.
- **`/library/topic-research/manifest`** still opens the first set by path.
  Making it not-found, and set member routes, are PP-998.
- **Three e2e URL expectations changed** because the listing and kanban now
  link by `id` or stem. `navigation.spec.ts` expects
  `/library/root-cause-analyses/2026-06-10-example-rca` and
  `/library/plans/2026-01-01-first-plan`. `kanban-click-vs-drag.spec.ts`
  expects `/library/work-items/0001`.
- **A stale e2e snapshot now fails setup loudly.** The snapshot is keyed by
  path under the fixtures' `meta/`. An `.e2e-fixture-snapshot.json` left by
  an interrupted run from before this change names unknown files, so setup
  stops and says to delete it.
- **Related items now label a plan by its id or stem** (`2026-04-18-foo`),
  not its slug (`foo`), matching its lifecycle card and URL.
- **Brief stale link after a delete:** an activity row stays linked until
  the invalidated listing refetches. A click in that window opens "Document
  not found". PP-1004 removes the listing join.
- **The boundary test is textual.** It matches strings and declarations, so
  a stem helper declared as a class member, or a route param destructured
  under another name, would pass. The resolver and call-site tests carry the
  behaviour.
- **`relates_to` draws no related item.** A set reaches a plan's related
  items only as a lifecycle-cluster sibling. That gap is PP-1003, not this
  PR.
- **Not-found copy** still says "We couldn't find a document with the slug
  …", although the value it quotes is now a document id. The recovery
  module's slug naming is deferred.
