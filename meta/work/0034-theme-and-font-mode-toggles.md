---
work_item_id: "0034"
title: "Theme and Font-Mode Toggles"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: ready
priority: medium
parent: ""
tags: [design, frontend, theming]
---

# 0034: Theme and Font-Mode Toggles

**Type**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add light/dark theming and a monospace font-mode swap, both implemented as
attribute-on-root + context-backed hook + topbar toggle button. Persist user
choice across reloads via `localStorage`, falling back to the OS
`prefers-color-scheme` setting on first visit.

## Context

The current app has no theming mechanism: one and only one colour palette and
no `data-theme` attribute on the document root. The dark-mode token values
were introduced in 0033 — a complete override of every `--ac-*` colour and
shadow token under `[data-theme="dark"]` — but remain inert until a
`data-theme` attribute is present. 0033 was intended to also ship a `[data-font="mono"]` font-mode swap, but that
CSS block was not delivered; authoring it is in scope for this story.

Reference screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`,
`main-dark.png` show the same layout under both themes.

## Requirements

- Implement a theme context hook that sets `data-theme` (`light`/`dark`) on
  the document root.
- Implement a font-mode context hook that sets `data-font` (`display`/`mono`)
  on the document root.
- Author the `[data-font="mono"]` CSS block in `global.css` (not delivered by
  0033), reassigning `--ac-font-display` and `--ac-font-body` to `"Fira Code"`
  (already defined as `--ac-font-mono` at `global.css:87`).
- When no stored preference exists, initialise `data-theme` from the OS
  `prefers-color-scheme` media query (`dark` → `data-theme="dark"`, anything
  else → `data-theme="light"`).
- Persist both choices to `localStorage` so they survive a reload.
- Populate the `data-slot="theme-toggle"` and `data-slot="font-mode-toggle"`
  placeholder elements in the Topbar (created by 0035) with functional toggle
  buttons wired to the theme and font-mode context hooks.
- Verify that any component whose CSS or TSX file is modified as part of this
  story's implementation consumes `--ac-*` tokens and contains no hard-coded
  hex colour values, so the theme swap is automatic without component-level
  conditionals.

## Acceptance Criteria

- [ ] Given the user clicks the topbar theme toggle, then `data-theme` switches
  between `light` and `dark` on `<html>` and all tokens enumerated in the
  `[data-theme="dark"]` block (`global.css:138–160`) resolve to their
  theme-appropriate values, verified by the pre-existing dark-block parity
  assertion in `global.test.ts` (line 167).
- [ ] Given the user clicks the font-mode toggle, then `data-font` switches
  between `display` and `mono` and `--ac-font-display` / `--ac-font-body` both
  compute to `"Fira Code"` on `document.documentElement`.
- [ ] Given `localStorage` contains `"ac-theme"` and/or `"ac-font-mode"`
  entries from a prior session, when the app boots, then the stored
  `data-theme` / `data-font` values are applied to `<html>` before any paint.
  Verified by setting those keys in DevTools, reloading with a Performance
  recording, and confirming no flash-of-wrong-theme occurs — the boot script
  must set the attributes before the browser evaluates the first stylesheet.
- [ ] Given the user has no stored theme preference and their OS is in dark
  mode, when the app boots, then `data-theme="dark"` is applied; when the OS
  is not in dark mode, `data-theme="light"` is applied.
- [ ] Given `localStorage` is unavailable (e.g. private-browsing mode), when
  the app boots, then the OS `prefers-color-scheme` fallback is used,
  `data-theme` is correctly set on `<html>`, and no exception is thrown (no
  uncaught `SecurityError` in the console).
- [ ] Given the app is loaded, the inline boot script is the first element in
  `<head>`, placed before any `<link rel="stylesheet">` tags, so `data-theme`
  and `data-font` are present on `<html>` when the browser evaluates its first
  stylesheet. Verified by inspecting the rendered HTML source and confirming
  the inline `<script>` tag appears before all `<link rel="stylesheet">` tags.
- [ ] `suppressHydrationWarning` is set on `<html>`, and no React hydration
  warning appears in the browser console on initial load when `data-theme` or
  `data-font` were set by the boot script before React hydrates. Verified by
  loading the app with a stored non-default preference and checking the console
  for hydration warnings.

## Dependencies

- Blocked by: 0033 (token system — dark-mode colour tokens delivered;
  `[data-font="mono"]` CSS block not delivered, authorship in scope for this
  story) and 0035 (Topbar must exist with its placeholder toggle slots before
  this story can populate them with functional controls).
- Blocked by: Confirmation that the deployment's Content Security Policy
  permits the inline boot script (see Assumptions); without this the
  flash-prevention guarantee in the boot-script AC cannot be met.
- Blocks: downstream component re-skin stories — all component work that
  depends on theme-swap behaviour requires the `data-theme` / `data-font`
  attribute wiring this story delivers (see design-gap document sequencing).

## Assumptions

- `localStorage` is an acceptable persistence mechanism (no requirement to
  sync theme preference across devices via the backend).
- The app's Content Security Policy (if any) permits the inline boot script,
  either via `'unsafe-inline'`, a SHA-256 hash of the script, or a nonce.
  Without this, the flash-prevention guarantee cannot be met.

## Technical Notes

- The boot script must be a classic (non-`async`, non-`defer`) inline
  `<script>` placed as the **first child of `<head>`**, before any
  `<link rel="stylesheet">`. Parser-blocking execution is what guarantees the
  attribute lands before first paint.
- Wrap all `localStorage` access in `try/catch` — Firefox and Safari in
  private-browsing mode throw `SecurityError` on any localStorage access,
  which would otherwise prevent the attribute from being set at all.
- Add `suppressHydrationWarning` to the `<html>` element (in `index.html` or
  the JSX root) to suppress React's reconciler warning about the pre-hydration
  attribute mutation.
- Treat the boot script as **read-only at startup**. All post-boot theme and
  font-mode changes must go through React context, which calls
  `document.documentElement.setAttribute(...)` synchronously and writes to
  `localStorage`. If the boot script and context diverge on `localStorage` key
  names or attribute names, flash-of-wrong-theme recurs on subsequent visits.
- Prior art: next-themes (pacocoursey/next-themes) follows the identical
  pattern, as does Mantine (`data-mantine-color-scheme` on `<html>`).
- **`index.html` current state** (`index.html:1–16`): `<html lang="en">` at
  line 2 — no `data-theme`, `data-font`, or `suppressHydrationWarning`. First
  `<head>` child is `<meta charset>` (line 3), followed by preload links for
  `Inter-Regular.woff2` and `Sora-Bold.woff2` (lines 7–10). The inline boot
  script inserts as the new first child of `<head>`, ahead of those preload
  links.
- **`[data-font="mono"]` CSS block is missing**: `global.css` has no
  `[data-font="mono"]` selector; it was not delivered by 0033. The 0034
  implementer must write it — it should
  reassign `--ac-font-display` and `--ac-font-body` to `"Fira Code"` (already
  defined as `--ac-font-mono` at `global.css:87`). A matching
  `MONO_FONT_TOKENS` export will likely be needed in `tokens.ts`, and a parity
  test in `global.test.ts` (which already tests dark-block equivalence at
  line 167).
- **Dark theme CSS** (`global.css:138–160`): 20 token overrides under
  `[data-theme="dark"]`. A byte-equivalent mirror under
  `@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) }`
  lives at lines 166–191 — the escape hatch is already wired, so setting
  `data-theme="light"` in the boot script correctly suppresses the OS fallback.
- **React context pattern** (`use-doc-events.ts`): `createContext<T>(default)`
  at module level → named export (`DocEventsContext`) → consumer hook
  (`useDocEventsContext`) calling `useContext`. Provider placed in
  `RootLayout.tsx:18`. Follow this pattern for `ThemeContext` / `useTheme`
  and `FontModeContext` / `useFontMode`; providers wrap around or nest inside
  the existing `DocEventsContext.Provider`.
- **`localStorage` keys**: no `localStorage` usage exists anywhere in `src/`.
  Establish new keys — suggest `"ac-theme"` and `"ac-font-mode"` — and keep
  them identical in both the boot script and the React context files to prevent
  key divergence causing flash-of-wrong-theme on reload.

## Drafting Notes

- The gap analysis describes theming both as a Token Drift item (CSS
  infrastructure) and as a Net-New Feature (the toggle hook). Treated as one
  story because the two halves are tightly coupled.
- OS `prefers-color-scheme` confirmed as the first-visit fallback (user
  answered explicitly during enrichment). `light` is not the unconditional
  default.
- Font-mode toggle resolved as end-user-visible in the topbar; the user
  audience for this app is developers and designers so discoverability is
  appropriate.
- "Before any paint" resolved as the strict parser-blocking inline script
  guarantee (not a near-zero-flash `useEffect` approach). AC and Technical
  Notes reflect this.
- Context updated to reflect that 0033 shipped dark-mode token values and the
  `--ac-font-mono` brand token; the `[data-font="mono"]` CSS override block was
  not delivered by 0033 and is authored by this story.
- Attribute value `data-font="display"` adopted (not `default`) to align with
  the prototype design inventory's canonical naming
  (`meta/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md:227`).
  The plan (`meta/plans/2026-05-08-0034-theme-and-font-mode-toggles.md`) records
  this decision.

## References

- Source: `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`,
  `main-dark.png`
- Related: 0033, 0035
- Prior art: https://github.com/pacocoursey/next-themes
- Prior art: https://css-tricks.com/flash-of-inaccurate-color-theme-fart/
