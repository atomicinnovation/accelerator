---
type: "work-item"
id: "PP-996"
external_id: "PP-996"
aliases: ["draft-7qknfk"]
title: "Links to a Research Set or Design Inventory Open the Wrong One"
date: "2026-11-23T21:17:40+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "ready"
kind: "bug"
priority: "high"
parent: "work-item:PP-868"
blocks: ["work-item:PP-998"]
relates_to: ["work-item:PP-994"]
tags: ["research", "visualiser"]
last_updated: "2026-11-24T00:06:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-996: Links to a Research Set or Design Inventory Open the Wrong One

**Kind**: Bug
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

In the visualiser, a link to a topic-research set or a design inventory from
the activity feed or related items, and a link to a design inventory from a
lifecycle view, opens whichever one sorts first by path, not the one named.
With more than one of either, most such links land on the wrong document.

## Context

Child of PP-868 — Topic-Research Set Detail Page. Lifted out of set member
routes and view (PP-998) after review: the defect exists today, its
fix needs neither the set model nor member routes, and it lands before
member routes (PP-998).

Both doc types are directories whose entry document has a fixed name:
`manifest.md` for topic research, `inventory.md` for design inventories. In
this item a **set** means a topic-research set only, and an **inventory** a
design inventory. Each has a library URL, `/library/<type>/<slug>`, whose
slug comes from the directory's name and is carried on its index entry.

Four call sites build the link from the entry document's filename instead,
so every topic-research set links to `/library/topic-research/manifest`.
The page then shows the first entry whose filename matches, in the type
listing's order, which is ascending by path. Three of the four are reachable
for a set or an inventory:

| Surface        | Set           | Inventory     |
|----------------|---------------|---------------|
| Activity feed  | wrong link    | wrong link    |
| Related items  | wrong link    | wrong link    |
| Lifecycle view | never shown   | wrong link    |
| Wiki links     | no such link  | no such link  |

A lifecycle view never renders topic research. A wiki link names an ADR or a
work item (`[[ADR-NNNN]]`, `[[WORK-ITEM-<id>]]`) and cannot target a set or
an inventory, so that call site carries the faulty expression with no
reachable defect.

Search results and the type listing build the link from the slug and are
correct.

## Requirements

Reproduction, read from the code and not yet run:

1. Have two topic-research sets, `a` and `b`, each with a `manifest.md`.
2. Change `b/manifest.md` on disk.
3. Open the activity feed and follow the row for the change.

Expected: the page shows set `b`, at `/library/topic-research/b`.

Actual: the link is `/library/topic-research/manifest`, and the page shows
set `a`, whose path sorts first.

The other reachable cases follow the same pattern:

- A related-items row for a set or an inventory, shown on a document whose
  `relates_to` or `target` names it, links to `/library/<type>/manifest` or
  `/library/<type>/inventory`.
- A lifecycle view's link to an inventory in its cluster is
  `/library/design-inventories/inventory`.

The fix:

- Build every library link to a document through one link builder. It
  takes what every call site can supply, a doc type, a slug where the
  document has one, and its path, and returns `/library/<type>/<slug>`,
  falling back to the filename only where there is no slug. This is the
  rule the type listing applies today. A search row supplies its type and
  slug; the other call sites supply an index entry's. The builder names no
  doc type.
- Use the builder at all six call sites: the activity feed, related items,
  lifecycle views, wiki links, search results, and the type listing. Member
  links later extend it (PP-998).
- On the activity feed, whose events carry a path and no slug, find the
  entry by matching the event's path in the type-listing response for the
  event's doc type. This lookup is the fix for every doc type and stays:
  it does not depend on whether the set model (PP-994) has landed, and no
  later story removes it.
- Render an activity row with no link when the type-listing response holds
  no entry for its path: while that response is still loading, and for
  good once the document has been deleted.

## Acceptance Criteria

- [ ] Given two topic-research sets `a` and `b` with different titles, each
      changed on disk while the visualiser runs, then each activity row's
      link is `/library/topic-research/<that set's
      slug>` and the page it opens shows that set's title; the same holds
      for two design inventories and `/library/design-inventories/<slug>`,
      where the slug is the one on that inventory's type-listing entry.
- [ ] Given a plan whose `relates_to` names set `a` and another naming set
      `b`, then each plan's related-items row links to
      `/library/topic-research/<that set's slug>`; the same holds for two
      design inventories each named as a plan's `target`.
- [ ] Given two design inventories in lifecycle clusters, then each
      lifecycle link is `/library/design-inventories/<that inventory's
      slug>`.
- [ ] Given a wiki link `[[ADR-0067]]` and a wiki link to a work item, then
      each links to that document's type-listing row's URL and opens the
      page showing its title.
- [ ] Given a search result and a type-listing row for set `b`, then each
      links to `/library/topic-research/b`; the same holds for an inventory
      and its slug.
- [ ] Given a document of a doc type that is not a set, whose entry's slug
      differs from its filename's stem, then its activity row,
      related-items row, and lifecycle link each equal its type-listing
      row's link.
- [ ] Given a plan at `meta/plans/foo.md` that exists, then its activity
      row, related-items row, and lifecycle link are each
      `/library/plans/foo`.
- [ ] Given an activity row for a set whose directory has since been
      deleted, and one for a deleted plan, then once the type-listing
      response has refetched each row shows its text and no link.
- [ ] Outside tests, none of the six call-site modules calls
      `fileSlugFromRelPath` or assembles a `/library/` document path
      itself: each calls the builder. `LibraryDocView`, which resolves a
      URL and builds none, is outside this rule.
- [ ] Outside tests, the builder's module contains neither the identifiers
      `TopicResearch` and `DesignInventories` nor the strings
      `topic-research` and `design-inventories`.

## Open Questions

- None. Which surfaces can show a design inventory was settled from the
  code on 2026-11-23: the activity feed, related items, and lifecycle
  views can; wiki links cannot.

## Dependencies

- Blocked by: nothing.
- Blocks: PP-998, set member routes and view, which extends the link
  builder to members and stops a root's filename resolving as a URL. If
  PP-998 landed first, the affected surfaces would link to a not-found
  page.
- Related: the set model (PP-994). Either may land first, and this fix is
  the same in both orders. The lookup relies on PP-994 keeping a set's
  activity row identified by its root's path, which that story states.

## Assumptions

- Three call sites already hold the linked document's index entry, with
  its slug: related items, lifecycle views, and wiki links. Only the
  activity feed needs a lookup, and the type listing it reads is one the
  frontend already queries. This was read from the code, not tried.
- `/library/topic-research/manifest` keeps resolving to the first set after
  this fix; nothing links to it any more. Making it not-found belongs to
  PP-998.
- An activity row for a deleted document of any doc type loses its link.
  Today such a link opens a not-found page, or for a set, the wrong set.

## Technical Notes

**Size**: S — six call sites and one shared link builder, with one lookup.

- The four faulty call sites derive the link from the last path segment:
  `cli/visualiser/frontend/src/components/ActivityFeed/ActivityFeed.tsx:123`,
  `components/RelatedArtifacts/RelatedArtifacts.tsx:96`,
  `routes/lifecycle/LifecycleClusterView.tsx:204`, and
  `api/wiki-links.ts:187`, through `fileSlugFromRelPath`
  (`api/path-utils.ts:6-8`).
- `LibraryDocView` resolves `/library/$type/$fileSlug` with
  `entries.find(e => e.slug === fileSlug || fileSlugFromRelPath(e.relPath)
  === fileSlug)` (`routes/library/LibraryDocView.tsx:83-85`), over entries
  the server sorts by path (`cli/visualiser/server/src/api/docs.rs:39-40`),
  which is why the filename form opens the first match.
- The correct links come from `entry.slug ?? fileSlugFromRelPath(relPath)`
  (`routes/library/LibraryTypeView.tsx:269`) and from the search row's slug
  (`components/Sidebar/SearchResultsPanel.tsx:69-70`).
- Related items, lifecycle views, and wiki links each hold a full index
  entry, slug included. An activity event carries `{action, docType, path,
  timestamp}` and no slug
  (`cli/visualiser/server/src/activity_feed.rs:43-60`).
- Wiki links resolve against decisions and work items only
  (`api/use-wiki-link-resolver.ts:36-42`).
- A design inventory's slug is a shortened form of its directory's name; a
  topic-research set's is the directory's name.

## Drafting Notes

- Lifted from PP-998 on 2026-11-23 at the author's direction, after a
  review noted the defect is independent of member routes.
- Kind `bug`, since it is a defect in shipped behaviour. Priority inherits
  the parent's; with one research set on disk the defect cannot be seen.
- Not reproduced by running the visualiser; the behaviour is read from the
  code cited above.
- A review on 2026-11-23, followed by a reading of the code, corrected the
  defect's extent: three reachable surfaces, not four, with lifecycle
  views affecting inventories alone and wiki links unable to target either
  type. The author then confirmed one builder across all six call sites,
  and no link on an activity row whose document is gone.
- A re-review on 2026-11-23 found the fix described two ways, by landing
  order against PP-994. The author settled it: the type-listing lookup is
  the fix in both orders and is never removed, so the activity row's own
  slug is not read. Criteria were added for a non-set document whose slug
  differs from its filename, and for the builder naming no doc type.

## References

- Parent: PP-868, under the epic PP-22
- Blocks: PP-998 (member routes and view)
- Related: PP-994 (set model), PP-862 (registered the topic-research doc
  type)
