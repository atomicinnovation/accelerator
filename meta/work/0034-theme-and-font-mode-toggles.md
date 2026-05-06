---
work_item_id: "0034"
title: "Theme and Font-Mode Toggles"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, theming]
---

# 0034: Theme and Font-Mode Toggles

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add light/dark theming and a monospace font-mode swap, both implemented as attribute-on-root + context-backed hook + topbar toggle button. Persist user choice across reloads (e.g. in `localStorage`).

## Context

The current app has no theming mechanism: one and only one colour palette, no `data-theme` attribute, no dark-mode tokens. The prototype provides a complete dark-theme override of every `--ac-*` colour and shadow token under `[data-theme="dark"]`, plus a `[data-font="mono"]` font-mode swap that repoints `--ac-font-display` and `--ac-font-body` to Fira Code.

The prototype's topbar exposes a `Toggle theme` button cycling between values; once the token migration cascades through `[data-theme]` and `[data-font]`, theming and font-mode become CSS-only swaps wired up by a context hook.

Reference screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`, `main-dark.png` show the same layout under both themes.

## Requirements

- Implement a theme context hook that sets `data-theme` (`light`/`dark`) on the document root.
- Implement a font-mode context hook that sets `data-font` (`default`/`mono`) on the document root.
- Persist both choices (e.g. to `localStorage`) so they survive a reload.
- Surface a `Toggle theme` and a `Toggle font` control in the new Topbar (delivered by 0035).
- Ensure every component consumes `--ac-*` tokens so the theme swap is automatic without component-level conditionals.

## Acceptance Criteria

- [ ] Given the user clicks the topbar theme toggle, when the toggle fires, then `data-theme` switches between `light` and `dark` on `<html>` and every `--ac-*` token resolves to its theme-appropriate value.
- [ ] Given the user clicks the font-mode toggle, when the toggle fires, then `data-font` switches between `default` and `mono` and `--ac-font-display` / `--ac-font-body` repoint to Fira Code.
- [ ] Given the user has set theme or font mode and reloads the page, when the app boots, then the previously chosen value is restored before first paint (no flash).
- [ ] Initial-paint flash is avoided (e.g. via inline boot script that reads `localStorage` and applies the attribute before React hydrates).

## Open Questions

- Should the initial theme follow the OS `prefers-color-scheme` setting when no stored value exists, or default to `light`?
- Is the font-mode toggle intended for end-users or only for developers/designers (i.e. should it be discoverable in the topbar or hidden behind a keyboard shortcut)?

## Dependencies

- Blocked by: 0033 (token system) and 0035 (Topbar must exist to host the toggle controls).
- Blocks: none.

## Assumptions

- `localStorage` is an acceptable persistence mechanism (no requirement to sync theme preference across devices via the backend).

## Technical Notes

- The toggle UX is described as a single `Toggle theme` button that cycles values; the same pattern can apply to `Toggle font`.
- Avoiding flash-of-wrong-theme typically requires applying the `data-theme` attribute before React hydrates — an inline boot script is the conventional approach.

## Drafting Notes

- The gap analysis describes theming both as a Token Drift item (CSS infrastructure) and as a Net-New Feature (the toggle hook). Treated as one story because the two halves are tightly coupled.
- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`, `main-dark.png`
- Related: 0033, 0035
