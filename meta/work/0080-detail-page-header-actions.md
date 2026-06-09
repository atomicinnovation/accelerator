---
id: "0080"
title: "Detail-Page Header Actions (Open in Editor, Copy Link)"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: draft
priority: medium
tags: [design, frontend, detail-page]
type: work-item
schema_version: 1
last_updated: "2026-05-21T09:16:34+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0041", "work-item:0039"]
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
---

# 0080: Detail-Page Header Actions (Open in Editor, Copy Link)

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Wire concrete `Open in editor` and `Copy link` actions into the
existing `Page.actions` slot on every detail-page route, with `Copy link`
writing the canonical document URL to the clipboard and `Open in editor`
invoking an editor deep-link when configured.

## Context

The prototype's `DocPage` ships two right-aligned topbar buttons —
`Open in editor` (`Icon name="edit"`) and `Copy link` (`Icon name="link"`)
— on every detail page. They are decorative in the prototype but
represent a deliberate affordance slot. The current app's `Page` chrome
already supports this via the `actions?` prop (`PageProps`,
`Page.tsx:4-11`) but does not populate it for `LibraryDocView`.

## Requirements

- Render two right-aligned action buttons in the `Page.actions` slot on
  every detail-page route: `Open in editor` and `Copy link`.
- `Copy link` writes the canonical document URL (the current route's
  absolute URL) to the system clipboard.
- `Open in editor` invokes an editor deep-link (e.g. `vscode://file/…`)
  when an editor protocol is configured for the workspace; renders
  disabled or hidden when not configured.
- Use the existing Glyph or icon system for button glyphs.

## Acceptance Criteria

- [ ] Both action buttons are visible in the page header on every
  detail-page route.
- [ ] Clicking `Copy link` writes the canonical absolute URL to the
  clipboard and shows a Toaster confirmation (consumes 0039).
- [ ] Clicking `Open in editor` invokes the configured editor deep-link
  when present; the button is disabled or hidden when no protocol is
  configured.
- [ ] Both buttons consume `--ac-*` tokens and follow the existing
  `TopbarIconButton` styling precedent.

## Open Questions

- How does the workspace declare which editor deep-link protocol to
  use? New `config.md` field, environment variable, or runtime detection?
- Should the deep-link URL include line number / column, or just file
  path?

## Dependencies

- Blocked by: 0041 (Page.actions slot exists), 0039 (Toaster for
  copy-link confirmation).
- Blocks: none.

## Assumptions

- The clipboard API is available in the target browsers used by the
  visualiser audience.

## Technical Notes

- `Page.actions` prop is already wired; this story populates it for the
  detail-page route component.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0039, 0041
