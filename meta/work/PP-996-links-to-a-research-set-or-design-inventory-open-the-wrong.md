---
type: "work-item"
id: "PP-996"
external_id: "PP-996"
aliases: ["draft-7qknfk"]
title: "Links to a Research Set or Design Inventory Open the Wrong One"
date: "2026-11-23T21:17:40+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "bug"
priority: "high"
parent: "work-item:PP-868"
relates_to: ["work-item:PP-998", "work-item:PP-994"]
tags: ["research", "visualiser"]
last_updated: "2026-11-23T21:17:40+00:00"
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
the activity feed, related items, a lifecycle view, or a wiki link opens
whichever set or inventory lists first, not the one named. With more than
one of either, most such links land on the wrong document.

## Context

Child of PP-868 — Topic-Research Set Detail Page. Lifted out of set member
routes and view (PP-998) after review: the defect exists today, its
fix needs neither the set model nor member routes, and it can land first.

A set or inventory is a directory whose entry document has a fixed name:
`manifest.md` for topic research, `inventory.md` for design inventories. Its
library URL is `/library/<type>/<slug>`, where the slug comes from the
directory's name. Four surfaces build the link from the entry document's
filename instead, so every topic-research set links to
`/library/topic-research/manifest`, and the page shows the first set whose
filename matches.

Search results and the type listing build the link from the slug and are
correct.

## Requirements

Reproduction:

1. Have two topic-research sets, `a` and `b`, each with a `manifest.md`.
2. Change `b/manifest.md` on disk.
3. Open the activity feed and follow the row for the change.

Expected: the page shows set `b`, at `/library/topic-research/b`.

Actual: the link is `/library/topic-research/manifest`, and the page shows
set `a`.

The same happens from a related-items row, a lifecycle view, and a wiki link
that targets a set, and for design inventories at
`/library/design-inventories/inventory`.

The fix:

- Build every link to a set or inventory from its slug, on the activity
  feed, related items, lifecycle views, and wiki links.
- Use one link builder for all four, which search and the type listing can
  share and which member links can later extend.

## Acceptance Criteria

- [ ] Given two topic-research sets, each recently changed, then each
      activity row's link is `/library/topic-research/<that set's slug>` and
      opens that set.
- [ ] Given two topic-research sets, each related to from another document,
      then each related-items row opens the set it names.
- [ ] Given two topic-research sets in a lifecycle view, then each link opens
      the set it names.
- [ ] Given a wiki link targeting each of two topic-research sets, then each
      opens the set it names.
- [ ] Given two design inventories, then links to each from the same
      surfaces, where a design inventory can appear on them, open the
      inventory named.
- [ ] Given a search result and a type-listing row for a set, then each
      still opens that set.
- [ ] Given a document that is not a set, then its links from the four
      surfaces are unchanged.

## Open Questions

- Which of the four surfaces can show a design inventory? Design
  inventories take no part in lifecycle clusters, so the lifecycle view may
  not apply.

## Dependencies

- Blocked by: nothing.
- Blocks: nothing. Set member routes and view (PP-998) extends the
  link builder to members and stops a root's filename resolving as a URL.
- Related: the set model (PP-994) later carries a set's slug on every
  response, which removes the need to look it up.

## Assumptions

- The slug can be found today without a server change, by matching the
  linked path against the entries the type listing already returns. This
  was read from the code, not tried.
- `/library/topic-research/manifest` keeps resolving to the first set after
  this fix; nothing links to it any more. Making it not-found belongs to
  PP-998.

## Technical Notes

**Size**: S — four call sites and one shared link builder.

- The four call sites derive the link from the last path segment:
  `cli/visualiser/frontend/src/components/ActivityFeed/ActivityFeed.tsx:123`,
  `components/RelatedArtifacts/RelatedArtifacts.tsx:96`,
  `routes/lifecycle/LifecycleClusterView.tsx:204`, and
  `api/wiki-links.ts:187`, through `fileSlugFromRelPath`
  (`api/path-utils.ts:6-8`).
- `LibraryDocView` resolves `/library/$type/$fileSlug` with
  `entries.find(e => e.slug === fileSlug || fileSlugFromRelPath(e.relPath)
  === fileSlug)` (`routes/library/LibraryDocView.tsx:83-85`), which is why
  the filename form opens the first match.
- The correct links come from `entry.slug`
  (`routes/library/LibraryTypeView.tsx:270`,
  `components/Sidebar/SearchResultsPanel.tsx:69-70`).
- An activity event carries a path and no slug
  (`cli/visualiser/server/src/activity_feed.rs:43-60`).

## Drafting Notes

- Lifted from PP-998 on 2026-11-23 at the author's direction, after a
  review noted the defect is independent of member routes.
- Kind `bug`, since it is a defect in shipped behaviour. Priority inherits
  the parent's; with one research set on disk the defect cannot be seen.
- Not reproduced by running the visualiser; the behaviour is read from the
  code cited above.

## References

- Parent: PP-868, under the epic PP-22
- Related: PP-998 (member routes and view), PP-994 (set model),
  PP-862 (registered the topic-research doc type)
