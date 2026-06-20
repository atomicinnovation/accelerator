---
id: "0095"
title: "Theme-Reactive Markdown Task-List Checkboxes"
date: "2026-06-02T12:11:27+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: bug
priority: medium
tags: [visualiser, markdown, theme, dark-mode, bug]
last_updated: "2026-06-08T23:12:30+00:00"
last_updated_by: Toby Clemson
schema_version: 1
type: work-item
relates_to: ["work-item:0034", "work-item:0077"]
external_id: PP-117
---

# 0095: Theme-Reactive Markdown Task-List Checkboxes

**Kind**: Bug
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Task-list checkboxes in rendered artifact markdown (`- [ ]` / `- [x]`) are
currently emitted by `remark-gfm` (the GitHub Flavored Markdown plugin) as
native, unstyled `<input type="checkbox">` controls. Native controls consume none of the visualiser's theme tokens — their
only theme-aware lever is the CSS `color-scheme` property. The dark theme sets
`color-scheme: dark`, but light theme has no dedicated `color-scheme` rule and so
inherits the permissive `light dark` default from `:root`. As a result the
browser may paint the checkbox with its dark-mode
appearance in light theme, where it looks broken or low-contrast. The fix is to
replace the native control with a custom, token-driven glyph and label per the
approved prototype design, making the checkbox theme-reactive and adding
done-state label treatment.

## Context

The visualiser switches light/dark via a `data-theme` attribute on `<html>` and
re-declares its `--ac-*` colour tokens per theme. Theme-reactive components are
expected to source colours from those tokens rather than detect the theme
themselves (the pattern established by the Glyph component, 0037). The native
markdown checkbox breaks this contract because it is not styled by the app at
all — it is painted by the user agent. The prototype design replaces it with a
custom span-based glyph (the same approach as the FilterPill faux-checkbox),
which is theme-reactive by construction and additionally mutes and strikes
through the label of completed items.

### Steps to Reproduce

1. Render artifact markdown containing a task list, e.g. `- [x] done` and
   `- [ ] todo`.
2. Switch the visualiser to light theme (`data-theme="light"`), on an OS set to
   prefer dark.
3. **Expected**: the checkboxes render with a light-mode appearance and adequate
   contrast against the light surface.
4. **Actual**: the native checkbox is painted with the browser's dark-mode
   appearance, so it looks broken or low-contrast against the light UI. (Dark
   theme renders correctly.)

## Requirements

- Render markdown task lists as a custom structure rather than the native
  control: `ul.ac-md-tasklist` (no list marker) → `li.ac-md-task` (with an
  `is-done` modifier when checked) → `span.ac-md-task__box` (containing a check
  icon when checked) + `span.ac-md-task__label`.
- The box and check consume `--ac-*` theme tokens so they adapt to light and
  dark automatically: unchecked box uses a `--ac-stroke-strong` border (falling
  back to `--ac-stroke` when that token is undefined, matching the prototype's
  `var(--ac-stroke-strong, var(--ac-stroke))`) on an `--ac-bg-card` background;
  checked box fills with `--ac-accent` and shows a tick.
- Completed (checked) items mute the label (`--ac-fg-muted`) and strike it
  through (`text-decoration: line-through`, decoration colour
  `--ac-stroke-strong` falling back to `--ac-fg-faint`, matching the prototype's
  `var(--ac-stroke-strong, var(--ac-fg-faint))`).
- The native `<input type="checkbox">` produced by `remark-gfm` is no longer
  rendered for task lists.

## Acceptance Criteria

- [ ] Given an unchecked task item rendered in light or dark theme, when the
  DOM is inspected, then no native `<input type="checkbox">` is present and the
  box uses a `--ac-stroke-strong` border (with the `--ac-stroke` fallback) on an
  `--ac-bg-card` background.
- [ ] Given a checked task item rendered in light or dark theme, when inspected,
  then the box border and background are both `--ac-accent` and the tick renders
  in `#fff`, with a tick-to-fill contrast ratio of at least 3:1 against the
  `--ac-accent` fill in both themes.
- [ ] Given a checked task item, then a tick/check icon is rendered inside the
  box; and given any rendered task list, then no list-item marker (bullet) is
  shown.
- [ ] Given a checked task item rendered in light or dark theme, then its label
  is muted (`--ac-fg-muted`) and struck through (`text-decoration: line-through`);
  an unchecked item's label renders as normal body text.
- [ ] A visual-regression snapshot of the rendered task list (a checked and an
  unchecked item) matches a baseline newly captured in each theme as part of
  this work, within the project's pixel-diff tolerance, confirming parity with
  the prototype design (`ui.jsx:221-238`, `app.css:778-793`).
- [ ] The existing FilterPill faux-checkbox tests/snapshots continue to pass
  unchanged (FilterPill is a separate component and must not regress).

## Open Questions

- None. The earlier question of whether to pin `color-scheme` or declare
  `accent-color` on the native control is moot — the agreed solution replaces
  the native control with a custom token-driven glyph.

## Dependencies

- Related: 0094 (inline code styling — sibling theme-token markdown bug; it
  touches the same `MarkdownRenderer` component and `--ac-*` theme-token surface
  as this item, so the two fixes share a code surface and should be coordinated
  if scheduled separately, though neither blocks the other), 0076 (code-block /
  GFM (GitHub Flavored Markdown) renderer), 0037 (glyph component — the
  token-driven, theme-reactive pattern this fix follows), 0034 (theme and font
  mode toggles), 0077 (shadow and dark accent token audit).

## Assumptions

- Adopting the prototype's custom span-glyph rendering is the agreed solution,
  not a minimal `color-scheme`/`accent-color` patch on the native control. There
  is no separate decision-record artifact for this direction; the prototype
  design files (see Technical Notes / References) are the de facto source of
  truth, so if they change this work item should be re-evaluated.
- Done-state label treatment (mute + strike-through) is in scope. This expands
  past the originally stated glyph-only boundary, which now covers the label
  text for completed items.

## Technical Notes

- Current implementation: `remark-gfm` is added to the renderer at
  `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx:72-73`;
  the `MARKDOWN_COMPONENTS` override map (`MarkdownRenderer.tsx:29-44`) does not
  override `li`/`input`, so task lists pass through as native
  `<input type="checkbox" disabled>`. There is no checkbox CSS anywhere in
  `MarkdownRenderer.module.css`. Confirmed by
  `MarkdownRenderer.test.tsx:114-123`, which asserts on real
  `input[type="checkbox"]` nodes.
- Theme mechanism: `data-theme` is set on `document.documentElement`
  (`src/api/use-theme.ts:18-32`, `src/api/boot-theme.ts:20-29`); `--ac-*` tokens
  are re-declared per theme in `src/styles/global.css` (light under `:root`,
  dark under `[data-theme="dark"]` and the mirrored
  `@media (prefers-color-scheme: dark)` block). `color-scheme` is set at
  `global.css:333` (`:root` → `light dark`) and `:405` (`[data-theme="dark"]`
  → `dark`); there is no `[data-theme="light"]` forcing `light`, and no
  `accent-color` anywhere — which is why the native control fails in light mode.
- Target design (prototype): markup at
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/ui.jsx:221-238`;
  styles at `.../prototype-full/src/app.css:778-793`. The tick colour is a
  hardcoded `#fff` painted on the accent fill (the same approach FilterPill
  uses), which reads correctly in both themes.
- Reference pattern: FilterPill's custom `<span>` faux-checkbox
  (`FilterPill.module.css:185-208`) is already theme-reactive and is the closest
  existing example; it is a separate component and out of scope here.
- Verification baselines: the new custom markup has no existing visual-regression
  baseline, so light and dark baselines for the task-list rendering must be
  captured/regenerated as part of this work (linux baselines typically lag
  darwin — use the project's baseline-regeneration workflow rather than capturing
  inline). The visual-regression acceptance criterion cannot pass until those
  baselines exist.

## Drafting Notes

- The original stub framed the defect as a custom glyph using a hardcoded or
  dark-only token. Investigation showed the opposite: the checkboxes are native
  `<input>` controls with no app styling, and the dark appearance comes from an
  unpinned `color-scheme`. The framing, requirements, and approach were rewritten
  accordingly.
- The title was changed from "Markdown Checkboxes Always Styled For Dark Mode"
  to "Theme-Reactive Markdown Task-List Checkboxes" to reflect the corrected
  understanding (the real defect is lack of theme-reactivity, not unconditional
  dark styling).
- Scope was expanded from the glyph/box only to also include done-state label
  treatment (mute + strike-through), per the updated prototype design.

## References

- Design: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/`
  (`src/ui.jsx:221-238`, `src/app.css:778-793`)
- Related: 0094, 0076, 0037, 0034, 0077
