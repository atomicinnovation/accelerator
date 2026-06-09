---
id: "0083"
title: "DevDesignSystem Reference Page"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: draft
priority: low
tags: [design, frontend, dev-tools, documentation]
type: work-item
schema_version: 1
last_updated: "2026-05-21T09:16:34+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0037", "work-item:0038", "work-item:0033"]
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
---

# 0083: DevDesignSystem Reference Page

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Implement a single consolidated `DevDesignSystem` reference page in the
current app covering every design-system primitive across 24 sections,
activated by `#dev` hash / Cmd-Ctrl+Shift+D keybind / sidebar-foot
triple-click, and retire the existing `/glyph-showcase` and
`/chip-showcase` routes in favour of the consolidated reference.

## Context

The prototype's `DevDesignSystem` (`src/view-dev.jsx`) is a hidden
24-section reference page covering every primitive (Overview, Colours,
Type, Spacing, Radii & shadows, Icons, Doc-type glyphs, Empty-state
glyphs, Atomic mark, Chips, Status badges, Stage dots, Tier pills,
Buttons, Inputs & form, Sidebar nav, Cards, Tables, Markdown, Code
blocks, Frontmatter, Empty & banners, Toasts, Topbar) activated by
`#dev` hash, Cmd/Ctrl+Shift+D keybind, or a sidebar foot triple-click.
The current app exposes `/glyph-showcase` (from 0037) and
`/chip-showcase` as separate uncrumbed dev routes covering only two
primitives. The prototype's scroll-spy defect (TOC active highlight
pinned to `02 Colours`) must not be copied; the new implementation
should drive active highlight from the actual scroll source.

## Requirements

- Implement a single `DevDesignSystem` route covering 24 sections, one
  per primitive listed in Context.
- Activation triggers: `#dev` URL fragment, `Cmd/Ctrl+Shift+D` keybind,
  and a sidebar-foot triple-click affordance (any one of these
  activates the route).
- Scroll-spy TOC drives the active-section highlight from the actual
  scroll position (do not copy the prototype's pinned-to-`02 Colours`
  defect).
- Retire `/glyph-showcase` and `/chip-showcase` routes; redirect those
  paths to the corresponding sections of `DevDesignSystem` (or 404)
  and remove the routes from the router.
- Update the visualiser frontend README to reference
  `DevDesignSystem` as the canonical dev showcase.

## Acceptance Criteria

- [ ] `DevDesignSystem` renders all 24 sections with content sourced
  from the live design system (tokens, components).
- [ ] Each of the three activation triggers (`#dev` fragment,
  keybind, sidebar-foot triple-click) navigates to the route.
- [ ] The scroll-spy TOC highlights the section currently in view as
  the user scrolls; manual verification confirms the highlight tracks
  scroll position.
- [ ] `/glyph-showcase` and `/chip-showcase` are removed from the
  router (or redirect into the new route).
- [ ] Frontend README's "Developer routes" section lists
  `DevDesignSystem` and no longer lists the retired showcases.

## Open Questions

- Where should sidebar-foot triple-click affordance live — replacing the
  existing version label, attached to a sidebar element, or new?
- Is `Cmd/Ctrl+Shift+D` already bound in any browser context that would
  conflict?

## Dependencies

- Blocked by: 0037 (Glyph showcase content), 0038 (Chip variants), 0033
  (token surfaces), and the other primitives delivered by 0035, 0039,
  0040, 0041.
- Blocks: none.

## Assumptions

- All 24 sections can render from existing components and tokens; no
  new design content is required for the showcase itself.

## Technical Notes

- 0037 already established the `/glyph-showcase` pattern; this story
  generalises it and retires both showcases.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0033, 0035, 0037, 0038, 0039, 0040, 0041
