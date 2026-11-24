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
relates_to: ["work-item:PP-994", "work-item:PP-998", "work-item:PP-1003"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/265"
pr_number: 265
tags: ["visualiser", "frontend", "server", "library-links", "topic-research", "design-inventories"]
revision: "99f9982a213971038778c7f03037be3caf9b9e74"
repository: "accelerator"
last_updated: "2026-11-24T18:58:45+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [PP-996] Link each library document by its id so sets and inventories open the right page

## Summary

Every visualiser link to a library document now carries the document's
frontmatter `id`, falling back to its filename stem, and one resolver opens
a library URL by `id`, then slug, then stem. Before this PR, links to
topic-research sets and design inventories pointed at
`/library/<type>/manifest` or `/library/<type>/inventory` and opened
whichever entry sorted first by path. Links to documents sharing a slug,
such as two reviews of one plan, had the same problem.

## Changes

- **Server:** `IndexEntry` and `SearchResultRow` gain `id`, the
  frontmatter `id` as a non-empty string or `null`. Search rows also gain a
  forward-slashed `relPath`. Both fields are additive.
  `plan_id_from_entry` now reads `entry.id`; work-item identity still reads
  the raw frontmatter.
- **Address module:** `frontend/src/api/library-document-address.ts` owns
  library addressing. It exports `documentIdOf`, `libraryLink`,
  `libraryHref` (via the router's `interpolatePath`),
  `resolveLibraryDocument` and `descriptiveSlugOf`.
- **Route:** `/library/$type/$fileSlug` becomes `/library/$type/$documentId`.
  `LibraryDocView` and `useActiveDocRelPath` share the resolver, which runs
  three separate passes so an `id` match beats an earlier slug or stem.
- **Call sites:** related items, lifecycle cards, wiki links, the type
  listing, search results, kanban cards and not-found suggestions all link
  through `libraryLink`. Related items now render a router `<Link>`, and
  search and suggestion rows are keyed by `relPath`.
- **Activity feed:** events carry a path and no `id`, so
  `use-activity-row-documents.ts` finds each row's entry by `relPath` in its
  doc type's listing. A row with no entry, while the listing loads or after
  a delete, is a plain `div` with no link and no hover affordance.
- **Boundary test:** `api/library-addressing-boundary.test.ts` scans
  production modules and fails if any module but the address module builds
  a `/library/<type>/<document>` target, defines a stem helper or compares a
  URL segment against an entry. `api/path-utils.ts` is deleted.
- **Fixtures and e2e:** the four topic-research manifests and two design
  inventories get a quoted `id` equal to their directory name. The new
  `e2e/library-links.spec.ts` covers listings, search, shared-slug reviews
  and activity rows after on-disk edits. `global-setup.ts` now snapshots
  those six files as well as `meta/work/`.

## Context

- Work item:
  `meta/work/PP-996-links-to-a-research-set-or-design-inventory-open-the-wrong.md`
- Research:
  `meta/research/codebase/2026-11-24-PP-996-library-links-to-sets-and-inventories-resolve-wrong.md`
- Plan: `meta/plans/2026-11-24-PP-996-library-links-by-document-id.md`
- Validation:
  `meta/validations/2026-11-24-PP-996-library-links-by-document-id-validation.md`
  (result: pass)
- Stacked on #262 (PP-995), which is stacked on #261 and #268.

## Testing

- [x] `mise run` exits 0 end to end (224s) at `919c94d1`: 2766 frontend
      unit tests, 376 and 372 visualiser unit tests, and 360 e2e tests
- [x] After the validation fixes: the 5 affected frontend test files (70
      tests), the 5 Rust `document_id_tests`, `mise run frontend:check` and
      `mise run server:check` pass. The full `mise run` was not re-run.
- [x] `rg -n "path-utils|fileSlug" --glob '!**/testing/**' cli/visualiser/frontend`
      prints nothing
- [x] Each phase was test-first. The boundary test failed on
      `ActivityFeed.tsx` and `path-utils.ts` before Phase 4 made it pass.
- [x] Edge cases covered by unit tests:
  - an `id` beating another entry's slug, and a slug beating a stem
  - two documents sharing a slug, and documents with no `id`
  - percent-encoded ids, with `libraryHref` matching a rendered `<Link>`
  - activity rows that are pending, deleted, or gain a link on a live
    refetch
- [x] Manual checks in headless Chromium against throwaway repos: every
      phase's manual criteria and all six of the plan's Manual Testing Steps
- [ ] Hover and focus feel of linked and unlinked activity rows in a real
      browser under `mise run dev`

## Notes for Reviewers

- **Old URLs keep working.** Slug and stem URLs resolve through the later
  passes. The exception is a slug or stem that equals another document's
  `id`: the `id` pass then opens that other document. Corpus `id`
  conventions make this unlikely.
- **`/library/topic-research/manifest`** still opens the first set by path.
  Making it not-found, and set member routes, are PP-998.
- **Three e2e URL expectations changed** because the listing and kanban now
  link by `id` or stem. `navigation.spec.ts` expects
  `/library/root-cause-analyses/2026-06-10-example-rca` and
  `/library/plans/2026-01-01-first-plan`; `kanban-click-vs-drag.spec.ts`
  expects `/library/work-items/0001`.
- **The e2e snapshot is keyed by path** under the fixtures' `meta/`, not
  by work-item filename. Delete any `.e2e-fixture-snapshot.json` left by an
  interrupted run from before this change.
- **Brief stale link after a delete:** an activity row stays linked until
  the invalidated listing refetches. A click in that window opens "Document
  not found".
- **The boundary test is textual.** It matches strings and declarations, so
  a stem helper declared as a class member would pass. The resolver and
  call-site tests carry the behaviour.
- **`relates_to` draws no related item.** A set reaches a plan's related
  items only as a lifecycle-cluster sibling. That gap is PP-1003, not this
  PR.
- **Not-found copy** still says "We couldn't find a document with the slug
  …", although the value it quotes is now a document id.
