---
type: "work-item"
id: "PP-998"
external_id: "PP-998"
aliases: ["draft-ka9f8n"]
title: "Set Member Routes and View"
date: "2026-11-23T19:08:53+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "ready"
kind: "story"
priority: "high"
parent: "work-item:PP-868"
blocks: ["work-item:PP-1001", "work-item:PP-1000"]
blocked_by: ["work-item:PP-994", "work-item:PP-996"]
tags: ["research", "visualiser"]
last_updated: "2026-11-24T00:06:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
relates_to: ["work-item:PP-134"]
---
# PP-998: Set Member Routes and View

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As someone reading a multi-document set in the visualiser, I want each
member document to open at its own URL, so that a search result or a shared
link takes me to that document and not to the set's root.

A reader sees five changes. A member document opens at its own URL as a
plain document. A search result and a related-items row for a member each
open that member. An open member refreshes, with the
external-edit toast, when it changes on disk. And a URL built from a root's
filename, such as `/library/topic-research/manifest`, stops resolving: the
one behaviour this story removes.

## Context

Child of PP-868 — Topic-Research Set Detail Page. This story was split from
the set model (PP-994) at the member API: that story indexes a set's members
and serves them, and this one gives them routes and a view.

Routes today are `/library/<type>` and `/library/<type>/<slug>`, where the
slug names a set by its directory. Nothing deeper exists, so a member cannot
be addressed. A URL built from a root's filename, such as
`/library/topic-research/manifest`, also resolves today, to whichever set
lists first; PP-996 stops the visualiser producing such links, and this
story stops the URL resolving.

Terms follow the set model: a set's **root** is its entry document, a
**member** is any other file in it, a member is a **document** (markdown) or
an **asset**, and a doc type's **set declaration** marks it as a set.

The view here is deliberately plain. Four things remain after the
topic-research set page (PP-1001) lands: the route, the member lookup, the
member links, and the plain view as the fallback for any other set doc
type. For topic research the view itself is interim: PP-1001 replaces it
with set navigation around the member. A design inventory holds no member document as its skill
writes it today, though a markdown file placed in one would open in the
plain view. Topic research is therefore the plain view's only production
use in practice, so the view is kept to what the route and the links
need.

## Requirements

- Give every member document of a set, of any set doc type, its own URL
  under one route shape, so a link to a member document survives a reload.
- Resolve a set's URL, `/library/<type>/<slug>`, by the set's directory
  slug alone; member URLs sit beneath it. A root's filename stops resolving:
  `/library/topic-research/manifest`, which today opens whichever set lists
  first, shows not-found. Add no redirects.
- Render the root when it is addressed through the member route. The root
  is not a member, but its set-relative path is a valid path on the member
  route, and the page renders the root in place at that URL, with no
  redirect. The rule that a root's filename stops resolving concerns the
  slug position alone.
- Render a member document whose frontmatter is malformed or absent: its
  body, with the filename as its title.
- Show not-found for a member path under a set that does not exist, and
  for a path naming a directory.
- Give an asset member no URL. PP-134 adds its route together with serving
  its bytes; until then the member route with an asset's set-relative path
  in place of a document's shows not-found.
- Render a member document at its URL through the shared `LibraryDocView`:
  its title, frontmatter, and body, with a breadcrumb back to its set and
  no other set navigation. Show the not-found state for a member that does
  not exist.
- Have the frontend resolve the document being read from a member URL, so
  the external-edit toast and content refresh work for a member as they do
  for any document. When the open member is removed, the outcome is the
  one any document has today: the toast says it was deleted, and the page
  shows the not-found state without a reload.
- Open a member's own URL from every surface that names a member: its
  search result and a related-items row. The activity feed and lifecycle
  views name only sets, and a wiki link names an ADR or a work item, never
  a member.
- Build member links with the slug-based link builder PP-996 introduces
  for sets, so a set link and a member link share one form.
- Branch on a doc type's set declaration only, never on a named doc type,
  in the router, the member view (`LibraryDocView`), the two member
  lookups (the hook `useActiveDocRelPath` and the lookup inside
  `LibraryDocView`), and the link builder.

## Acceptance Criteria

- [ ] Given the URL of a member document three directories deep in a set,
      when it is loaded directly, then the page renders that member's title,
      frontmatter, and body, with a breadcrumb that opens its set; given the
      URL of a member that does not exist, then the page shows the not-found
      state.
- [ ] Given a topic-research set with slug `x`, and no set whose slug is
      `manifest`, then `/library/topic-research/x` renders the
      root's title, frontmatter, and body, and
      `/library/topic-research/manifest` shows the not-found state; the same
      holds for a design inventory, no inventory's slug being `inventory`,
      and `/library/design-inventories/inventory`. Given a topic-research
      set whose slug is `manifest`, then `/library/topic-research/manifest`
      renders that set's root.
- [ ] Given a plan at `meta/plans/foo.md`, then any deeper path beneath
      `/library/plans/foo` shows the not-found state.
- [ ] Given a document of a doc type that is not a set, then its existing
      `/library/<type>/<slug>` URL renders its title, frontmatter, and
      body.
- [ ] Given a design inventory with a `screenshots/` directory, when one
      image is loaded as a member URL, then the page shows the not-found
      state.
- [ ] Given a phrase that appears only in one member document, when it is
      searched for, then the result's link opens that member's own URL.
- [ ] Given a member document whose `relates_to` names a plan, when the
      plan's related items are shown, then the member's row opens the
      member's own URL.
- [ ] Given a member document open at its URL, when it changes on disk, then
      its new content appears without a reload and the "External edit
      detected" toast names its project-relative path; given a different
      member of the same set changes, then once the queries that change
      event invalidates have refetched no toast has appeared; given the set's root is open and a
      member changes, then likewise none has.
- [ ] Given a member document open at its URL, when it is deleted on disk,
      then the toast names its path and says it was deleted, and without a
      reload the page shows the not-found state.
- [ ] Given a member document whose body and frontmatter reference no other
      member, open at its URL, then the page shows a breadcrumb to its set
      and no link to any other member of the set.
- [ ] Given a set at `x` with a member at `a/b/c/doc.md`: the root addressed
      through the member route renders the root, and the address bar still
      shows that URL; a member at
      `a/b/c/doc.md` with unclosed frontmatter, and one with no frontmatter,
      each render their body with the title `doc.md`; a member path
      under a set slug that does not exist shows not-found; the path `a/b`
      shows not-found.
- [ ] Given a test-only doc type declared as a set, and no change outside
      that declaration, then its member documents render at member URLs
      with a breadcrumb to their set, its search results open member URLs,
      the toast fires for an open member, and the URL built from its root's
      filename shows not-found.
- [ ] Outside tests, `router.ts`, `LibraryDocView.tsx`,
      `use-active-doc-relpath.ts`, and the link builder's module contain
      neither the identifiers `TopicResearch` and `DesignInventories` nor
      the strings `topic-research` and `design-inventories`.

## Open Questions

- Route shape (planning-time): how is a member's set-relative path encoded
  beneath its set's URL? Settled already: member URLs sit beneath
  `/library/<type>/<slug>`, reach any depth, and leave room for PP-134's
  asset route. Still open: the literal form, including whether a prefix
  segment separates documents from assets and whether `.md` shows. The
  set page (PP-1001) and PP-134 consume the answer, which is recorded as
  a dated line in this item's Drafting Notes giving the URL of
  `a/b/c/doc.md` under set `x`.

## Dependencies

- Blocked by: PP-994, the set model, for member index records, the
  member API, and change events carrying a member's path.
- Blocked by: PP-996, the link fix, for the slug-based link builder this
  story extends to members and for the corrected set links. Until PP-996
  lands, the activity feed, related items, and lifecycle views still build
  the filename URL this story makes not-found.
- Blocks: PP-1001, the topic-research set page, which builds on the
  member route and the lookup of the member being read; and PP-1000,
  Sources rendering, which needs member documents rendered at their own
  URLs.
- Planning order: the set model's member API shape is settled and its ADR
  accepted before this story is planned; this story's route shape is then
  settled before PP-1001 and PP-134 are planned.
- Needs from the set model, each now a criterion there: a member's body by
  path from the document endpoint; the set declaration exposed to the
  frontend; the member's set-relative path on the search row; and the set's
  slug on every response naming a root or member, which covers the related
  item naming a member.
- PP-134 is not blocked by this story's landing, but its planning waits
  on the route shape settled here, as PP-1001's does. It adds the asset
  member route under that shape and replaces this story's not-found for
  an asset URL. PP-134 and PP-868 record PP-994 as its only blocker and
  do not yet mention this planning gate.
- Existing consumers of change events: the external-edit toast and the
  frontend's query invalidation. Query invalidation receives member events
  from the set model onward; the toast can first fire for a member here.

## Assumptions

- A set's own URL keeps rendering its root through `LibraryDocView` until
  the set page lands. Between this story and that one, a member opens as a
  plain document. A source's tier text reads as literal `— tier-3 —` until
  PP-1000 lands, which is after the set page. A member is
  reached by search, related items, or a direct link.
- The toast fires only for the document being read, so a member's change
  raises none for a reader on the set's root. A research `conduct` run that
  rewrites the root itself raises one there, as for any document.

## Technical Notes

**Size**: M — the router's first splat route, a member lookup in two
places, and member links on two surfaces.

- Routes are code-based TanStack Router
  (`cli/visualiser/frontend/src/router.ts:104-120`). A member path with
  slashes needs a splat segment; none exists today.
- `LibraryDocView` (`frontend/src/routes/library/LibraryDocView.tsx:62-85`)
  and `useActiveDocRelPath`
  (`frontend/src/api/use-active-doc-relpath.ts:18-36`) resolve a document
  from the type listing by `type` and `fileSlug` and need a member lookup.
- The toast fires when a change event's path equals the active document's
  (`frontend/src/api/use-external-edit-toast.ts:32-55`).
- The link builder and the corrected call sites come from PP-996.
- Search results link by `slug`
  (`frontend/src/components/Sidebar/SearchResultsPanel.tsx:69-70`); the set
  model adds a member locator to the search row.
- Test helpers hardcode the route shape: `frontend/src/test/router-helpers.tsx:34`
  and `frontend/tests/lib/detail-route-slugs.ts`.

## Drafting Notes

- Split from the set model on 2026-11-23 at the author's direction, at the
  member API, so PP-134 and the planning of later stories wait on the
  server work alone.
- The corrected set links moved to their own bug, PP-996, on 2026-11-23,
  at the author's direction: the defect exists today and needs neither the
  set model nor member routes. This story keeps only one part of it:
  stopping the filename-built URL resolving.
- Wiki links were dropped as a surface on 2026-11-23, at the author's
  direction: a wiki link names an ADR or a work item, and no syntax names a
  set member.
- A re-review on 2026-11-23 found the removal of PP-996's activity-feed
  lookup stated here only as a dependency. The author then settled that
  the lookup is never removed, so that work is gone from this story.
- A second review on 2026-11-23 recorded PP-996 as a blocker, widened the
  Summary to all five reader-visible changes, stated how the root resolves
  on the member route, and tightened the criteria on member links, titles,
  and the root-filename URL.
- The member view was assigned ahead of the set page after review, so
  routes, search, and related have something to observe.
- A link to a member opens the member on every surface that names one,
  decided on 2026-11-23.
- Set URLs are unchanged and no redirects are added, decided on 2026-11-23.

## References

- Parent: PP-868, under the epic PP-22
- Blocked by: PP-994 (set model), PP-996 (set links open the wrong set)
- Related: PP-1001 (set page), PP-1000 (Sources rendering), PP-134
