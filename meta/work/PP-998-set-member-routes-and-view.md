---
type: "work-item"
id: "PP-998"
external_id: "PP-998"
aliases: ["draft-ka9f8n"]
title: "Set Member Routes and View"
date: "2026-11-23T19:08:53+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:PP-868"
blocks: ["work-item:PP-1001", "work-item:PP-1000"]
blocked_by: ["work-item:PP-994"]
tags: ["research", "visualiser"]
last_updated: "2026-11-23T21:18:04+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
relates_to: ["work-item:PP-134", "work-item:PP-996"]
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

A reader sees two changes: a member document opens at its own URL as a
plain document, and a search result for a member opens that member.

## Context

Child of PP-868 — Topic-Research Set Detail Page. It was split from the set
model (PP-994) at the member API: that story indexes a set's members
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

The view here is deliberately plain. The route, the member lookup, the
search link, and the plain view as the fallback for future set types
outlast the set page; for topic research the view itself is interim.
 The topic-research set page
(PP-1001) later replaces it for topic research with set navigation
around the member.

## Requirements

- Give every member document of a set, of any set doc type, its own URL
  under one route shape, so a link to a member document survives a reload.
- Resolve a set's URL, `/library/<type>/<slug>`, by the set's directory
  slug alone; member URLs sit beneath it. A root's filename stops resolving:
  `/library/topic-research/manifest`, which today opens whichever set lists
  first, shows not-found. Add no redirects.
- Render the root when it is addressed through the member route, as at the
  set's URL.
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
  for any document, including when the member is removed.
- Open a member's own URL from every surface that names a member: its
  search result, a related-items row, and a wiki link whose target is a
  member file. The activity feed and lifecycle views name only sets.
- Build member links with the slug-based link builder PP-996 introduces
  for sets, so a set link and a member link share one form.
- Branch on a doc type's set declaration only, never on a named doc type,
  in the router, the member view, the two member-lookup hooks, and the
  link builder.

## Acceptance Criteria

- [ ] Given the URL of a member document three directories deep in a set,
      when it is loaded directly, then the page renders that member's title,
      frontmatter, and body, with a breadcrumb that opens its set; given the
      URL of a member that does not exist, then the page shows the not-found
      state.
- [ ] Given a topic-research set with slug `x`, then `/library/topic-research/x`
      renders the root's title, frontmatter, and body, and
      `/library/topic-research/manifest` shows the not-found state; the same
      holds for a design inventory and `/library/design-inventories/inventory`.
- [ ] Given a design inventory with a `screenshots/` directory, when one
      image is loaded as a member URL, then the page shows the not-found
      state.
- [ ] Given a phrase that appears only in one member document, when it is
      searched for, then the result's link opens that member's own URL.
- [ ] Given a member document whose `relates_to` names a plan, when the
      plan's related items are shown, then the member's row opens the
      member's own URL; given a wiki link whose target is a member file,
      then it opens that member's URL.
- [ ] Given a member document open at its URL, when it changes on disk, then
      its new content appears without a reload and the "External edit
      detected" toast names its project-relative path; given a different
      member of the same set changes, then once the navigation data has
      refetched no toast has appeared; given the set's root is open and a
      member changes, then likewise none has.
- [ ] Given a member document open at its URL, then the page shows a
      breadcrumb to its set and no link to any other member of the set.
- [ ] Given a set at `x` with a member at `a/b/c/doc.md`: the root addressed
      through the member route renders the root; a member with unclosed
      frontmatter renders its body titled by its filename; a member path
      under a set slug that does not exist shows not-found; the path `a/b`
      shows not-found.
- [ ] Given a test-only doc type declared as a set, and no change outside
      that declaration, then its member documents render at member URLs
      with a breadcrumb to their set, its search results open member URLs,
      the toast fires for an open member, and the URL built from its root's
      filename shows not-found.
- [ ] Outside tests, the router, the member view, the two member-lookup
      hooks, and the link builder contain neither the identifiers `TopicResearch` and
      `DesignInventories` nor the strings `topic-research` and
      `design-inventories`.

## Open Questions

- Route shape (planning-time): how is a member's set-relative path encoded
  beneath its set's URL? Settled already: member URLs sit beneath
  `/library/<type>/<slug>`, reach any depth, and leave room for PP-134's
  asset route. The set page (PP-1001) and PP-134 consume the answer.

## Dependencies

- Blocked by: PP-994, the set model, for member index records, the
  member API, and change events carrying a member's path.
- Blocks: PP-1001, the topic-research set page, which builds on the
  member route and the lookup of the member being read; and PP-1000,
  Sources rendering, which needs member documents rendered at their own
  URLs.
- Planning order: the set model's member API shape is settled and its ADR
  accepted before this story is planned; this story's route shape is then
  settled before PP-1001 is planned.
- Needs from the set model, each now a criterion there: a member's body by
  path from the document endpoint; the set declaration exposed to the
  frontend; the member's set-relative path on the search row; and the set's
  slug on every response naming a root or member.
- Related, not a blocker: PP-134 adds the asset member route under the
  route shape settled here and replaces this story's not-found for an asset
  URL.
- Existing consumers of change events: the external-edit toast and the
  frontend's query invalidation. Query invalidation receives member events
  from the set model onward; the toast can first fire for a member here.

## Assumptions

- A set's own URL keeps rendering its root through `LibraryDocView` until
  the set page lands. Between this story and that one, a member opens as a
  plain document: tier text reads as literal `— tier-3 —`, and a member is
  reached by search, related items, a wiki link, or a direct link.
- The toast fires only for the document being read, so a member's change
  raises none for a reader on the set's root. A run that rewrites the root
  itself raises one there, as for any document.

## Technical Notes

**Size**: M — the router's first splat route, a member lookup in two hooks,
and member links on three surfaces.

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
  set model nor member routes. This story keeps only the end of the
  filename-built URL.
- The member view was assigned ahead of the set page after review, so
  routes, search, and related have something to observe.
- A link to a member opens the member on every surface that names one,
  decided on 2026-11-23.
- Set URLs are unchanged and no redirects are added, decided on 2026-11-23.

## References

- Parent: PP-868, under the epic PP-22
- Blocked by: PP-994 (set model)
- Related: PP-1001 (set page), PP-1000 (Sources rendering),
  PP-134, PP-996 (set links open the wrong set)
