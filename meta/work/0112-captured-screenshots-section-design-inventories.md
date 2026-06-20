---
type: work-item
id: "0112"
title: "Captured Screenshots Section on Design Inventory Pages"
date: "2026-06-15T11:54:23+00:00"
author: Toby Clemson
producer: refine-work-item
status: draft
kind: story
priority: medium
relates_to: ["work-item:0097", "work-item:0111"]
tags: ["visualiser", "frontend", "polish", "markdown", "milestone-closeout"]
last_updated: "2026-06-15T11:54:23+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-134
---

# 0112: Captured Screenshots Section on Design Inventory Pages

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a user of the visualiser, I want design-inventory pages to render a
"Captured screenshots" section populated with the real images from the
inventory's `screenshots/` directory, so that I can browse what was captured
without leaving the page.

This is a **standalone** story, originally surfaced as fix F1 of the
milestone-closeout container story 0111 and split out from it. Unlike the other
closeout fixes (which are small CSS/wording parity changes), this is a
**net-new feature** requiring both new Rust server work and net-new SPA UI, so
it warrants its own research → plan → implement cycle rather than shipping
inline with the polish pass.

## Context

Split out of 0111 — Visualiser Frontend Fixes for First Milestone Closeout
(originally fix F1 there). Tracked standalone, not as a child.

The design prototype defines a "Captured screenshots" section for
design-inventory pages: a bordered/rounded section with a heading row carrying a
label and a count, then a 3-column grid of bordered/rounded thumbnails with
monospace filename captions. In the prototype this is **entirely hardcoded CSS
placeholders with no data source** — there are no real images and no backing
server data.

Screenshot files already live on disk under each inventory's slug directory
(`<root>/YYYY-MM-DD-HHMMSS-{id}/screenshots/`, alongside `inventory.md` and
`assets/`), but the server never enumerates or serves them: the indexer's
`list` yields only the inventory manifest (`.md`-only / nested-manifest
filters), and the existing `GET /api/docs/*path` handler (`doc_fetch`)
hardcodes `Content-Type: text/markdown`, so it is not image-aware. The data path
to surface real screenshots is therefore net-new on both server and frontend.

The authoritative reference for the intended look is the design prototype at
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`
(`src/view-library.jsx`, `src/app.css`).

## Requirements

- **Server: enumerate screenshots.** Provide a way for the SPA to discover the
  image files under a given inventory's `screenshots/` directory (the
  `LocalFileDriver::read` auth primitive already permits paths under a
  configured doc-type root, including a `screenshots/` subdir; enumeration is
  the gap — `list` only yields the manifest).
- **Server: serve images with correct MIME.** Image files must be served with an
  image-aware content type rather than the hardcoded `text/markdown` of the
  current `doc_fetch` handler. `mime_guess` is already a server dependency.
- **Frontend: render the section.** Design-inventory pages render a "Captured
  screenshots" section in the prototype's visual style — bordered/rounded
  section, a heading row with a label and a count, and a 3-column grid of
  bordered/rounded thumbnails. Thumbnails are real `<img>` elements (not CSS
  placeholders). Each thumbnail caption is the screenshot's filename in
  monospace. Clicking a thumbnail opens the full image in a new browser tab.
- **Frontend: omit when empty.** Inventories with no `screenshots/` directory or
  an empty one omit the section entirely (no empty grid, no empty state).

## Acceptance Criteria

- [ ] Given a design inventory with a `screenshots/` directory containing N
      images, when its page renders, then a "Captured screenshots" section
      appears with a count reading the number present (e.g. "14 screenshots")
      and a 3-column grid of thumbnails styled per the prototype.
- [ ] Given a screenshot thumbnail, when clicked, then the full image opens in a
      new browser tab; and each thumbnail's caption is the screenshot's
      filename.
- [ ] Given a design inventory with no `screenshots/` directory or an empty one,
      then the section is omitted (no empty grid).
- [ ] Given a screenshot image request to the server, when served, then the
      response carries an image-appropriate `Content-Type` (e.g. `image/png`,
      `image/jpeg`) rather than `text/markdown`.

## Open Questions

- Should the screenshot-enumeration capability be a new dedicated endpoint, or
  an extension of the existing docs/inventory data the SPA already fetches for
  the page? (Resolve during planning.)

## Dependencies

- Blocked by: [none]
- Blocks: General release of the first visualiser version (shared with 0111).

## Assumptions

- The section is omitted entirely when no screenshots exist, rather than showing
  an empty state.
- Screenshot files remain on disk under the inventory slug directory's
  `screenshots/` subdir, as today; no new storage location is introduced.

## Technical Notes

- **Server enumeration gap.** `Indexer`/`LocalFileDriver::list`
  (`server/src/file_driver.rs:266-345`, `.../indexer.rs:286`) lists only `*.md`
  for normal types and the manifest for nested-manifest types
  (`nested_manifest_filename()` → `"inventory.md"`,
  `server/src/docs.rs:144-149`). Screenshot files are never enumerated.
- **Server auth primitive already permits these paths.** `file_driver.read`
  (`server/src/file_driver.rs:347-389`) canonicalises and calls
  `path_is_allowed` (`.../file_driver.rs:145-153`), which permits any path under
  a configured doc-type root — including a `screenshots/` subdir — subject to the
  10 MiB `MAX_DOC_BYTES` cap (`.../file_driver.rs:21,381-386`).
- **Content-type gap.** `GET /api/docs/*path` → `doc_fetch`
  (`server/src/api/docs.rs:44-107`) hardcodes
  `Content-Type: text/markdown; charset=utf-8` (`.../api/docs.rs:99-102`); it is
  not image-aware. `mime_guess` is already a dependency (used by
  `serve_embedded`, `server/src/assets.rs:102`).
- **Routes.** API routes are assembled in `server/src/api/mod.rs:26-53`
  (`mount`), merged in `server/src/server.rs:231-270`
  (`build_router_with_spa`).
- **Frontend has no inventory-specific render path.** Inventories render through
  the generic `routes/library/LibraryDocView.tsx` (doc type
  `"design-inventories"`, `api/types.ts:352-356`). The two-column body grid is
  `LibraryDocView.tsx:128-209` with the aside section pattern at
  `LibraryDocView.module.css:1-18,34-38`. A screenshots section would be net-new
  UI added here. No `screenshot` render path exists today.

## Drafting Notes

- Split out of 0111 during refinement as the one closeout item that is both a
  new feature and outsized (server + frontend, net-new data path), warranting
  its own cycle. Tracked as a standalone story (not a child of 0111); the other
  seven closeout fixes remain inline in 0111.
- Kind set to `story` (not the default story→task derivation) because the
  extraction rationale is precisely that this needs its own research → plan →
  implement cycle.
- Caption/count/click behaviour (filename caption, count-present,
  open-in-new-tab) were chosen by the author over the prototype's hardcoded
  equivalents.

## References

- Source: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`
- Related: 0097, 0111 (split out of 0111 — originally fix F1)
