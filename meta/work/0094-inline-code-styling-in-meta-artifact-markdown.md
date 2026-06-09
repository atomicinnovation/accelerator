---
id: "0094"
title: "Inline Code Styling In Meta Artifact Markdown"
date: "2026-06-02T12:11:27+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: bug
priority: medium
tags: [visualiser, markdown, rendering, bug]
last_updated: "2026-06-02T14:26:51+00:00"
last_updated_by: Toby Clemson
schema_version: 1
type: work-item
relates_to: ["work-item:0076"]
---

# 0094: Inline Code Styling In Meta Artifact Markdown

**Kind**: Bug
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The visualiser's markdown renderer for meta artifacts renders inline code
spans (`` `like this` ``), but not with the styling defined by the latest
design prototype. Most visibly, inline code renders in the prose body font
(Inter) rather than a monospace face, leaving it visually inconsistent with
the intended design system.

## Context

Inline code is emitted by the renderer, so this is a styling gap rather than
a parsing failure. The live rule (`.markdown code:not(pre code)`) and the
prototype rule (`.ac-md-code`) already match on background and text colour —
so the divergence is specific, not wholesale. The gaps are:

- **Font family** — the live rule sets no `font-family`, so inline code
  inherits the prose body font (Inter) instead of the monospace face
  (`--ac-font-mono` → Fira Code). This is the headline defect.
- **Border** — the prototype gives inline code a soft `1px` border; the live
  rule applies none.
- **Font size** — the live rule uses `--size-xs` (14px); the prototype uses a
  smaller `11.5px` (11px inside table cells), giving inline code a distinct
  "pill" against prose.
- **Padding / radius** — minor differences (`1px 5px` / `3px` in the prototype
  vs `0.1rem var(--sp-1)` / `var(--radius-sm)` = 4px live).

Both rules deliberately scope inline code apart from fenced blocks (the live
rule via the `:not(pre code)` selector), so correcting inline styling does not
risk regressing fenced `<pre><code>` blocks.

**Reproduction**: Open any meta artifact whose body contains an inline
`` `code` `` span in the visualiser's markdown view. Expected: a monospace
(Fira Code) pill matching the prototype. Actual: the span renders in the Inter
prose font at `14px` with no border.

## Requirements

- Inline code spans render in the monospace face via `--ac-font-mono`
  (Fira Code), not the inherited prose body font.
- Inline code adopts the prototype's pill chrome: a soft `1px` border
  (`--ac-stroke-soft`), the sunken background (`--ac-bg-sunken`), `3px` rounded
  corners, and `1px 5px` padding.
- Inline code uses the prototype's font size (`11.5px`, `11px` within table
  cells), smaller than surrounding prose.
- All values consume theme tokens so the styling renders correctly in both
  light and dark mode.
- Fenced `<pre><code>` blocks remain untouched — the existing `:not(pre code)`
  scoping must continue to exclude them.

## Acceptance Criteria

- [ ] Given an artifact body containing inline `` `code` ``, when it renders,
  then the inline `<code>` computed `font-family` resolves through
  `--ac-font-mono` (Fira Code), differing from the prose `font-family` (Inter).
- [ ] Given inline code renders, then its computed `background-color` resolves
  to `--ac-bg-sunken`, its `border` is `1px solid var(--ac-stroke-soft)`, its
  `border-radius` is `3px`, and its `padding` is `1px 5px`.
- [ ] Given inline code outside a table renders, then its computed `font-size`
  is `11.5px`, not the current `14px`.
- [ ] Given inline code inside a table cell renders, then its computed
  `font-size` is `11px`.
- [ ] Given the theme is toggled to dark mode, then the inline-code computed
  `background-color` resolves to the dark `--ac-bg-sunken` (`#070b12`, vs the
  light `#f4f6fa`) and the `border-color` resolves to the dark
  `--ac-stroke-soft` (`rgba(255,255,255,0.04)`, vs the light
  `rgba(32,34,49,0.06)`).
- [ ] Given a fenced code block in the same document, when it renders, then the
  `:not(pre code)` scoping is retained and the block's computed `font-family`,
  `font-size`, and `background` are unchanged from before the change.

## Dependencies

- Related: 0076 (code-block syntax highlight palette — tokenisation precedent),
  0088 (markdown body width harmonisation), 0095 (markdown checkboxes always
  dark-mode-styled — sibling theme-token bug), 0089 (templates preview Fira
  Code whitespace — monospace surface adjacency).
- These four are purely thematic adjacencies — none introduces or relocates a
  token this work consumes, so there is no ordering or blocking constraint and
  0094 can proceed independently.

## Assumptions

- Scope is inline `` `code` `` spans only; fenced code blocks (0076's
  territory) are explicitly excluded.
- Background and text colour already match the prototype and require no change;
  this work changes font family, border, font size, padding, and radius only.

## Technical Notes

Property-by-property divergence (prototype `.ac-md-code` vs live
`.markdown code:not(pre code)`):

| Property | Prototype | Live | Diverges |
|---|---|---|---|
| font-family | `var(--ac-font-mono)` (Fira Code) | *unset* → inherits Inter | Yes (headline) |
| border | `1px solid var(--ac-stroke-soft)` | none | Yes |
| font-size | `11.5px` (`11px` in tables) | `var(--size-xs)` = 14px | Yes |
| padding | `1px 5px` | `0.1rem var(--sp-1)` | Minor |
| border-radius | `3px` | `var(--radius-sm)` = 4px | Minor |
| background | `var(--ac-bg-sunken)` | `var(--ac-bg-sunken)` | No |
| color | inherited | inherited | No |

- Live rule: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:57-60`.
- Renderer maps inline code to a plain `<code>` (no custom component); only the
  `pre` node is overridden — `MarkdownRenderer.tsx:29-44`.
- Tokens confirmed available live (`global.css`): `--ac-font-mono`,
  `--ac-bg-sunken` (light/dark), `--ac-stroke-soft` (light
  `rgba(32,34,49,0.06)` / dark `rgba(255,255,255,0.04)`, matching the
  prototype), `--radius-sm`, `--size-xs`, `--sp-1`.
- Prototype reference: `prototype-standalone.html`, `.ac-md-code` rule.

## Drafting Notes

- Reframed from "not rendered" to "rendered with incorrect styling" per author
  clarification: inline code is emitted, but the styling does not match the
  latest prototype.
- Scope narrowed to inline spans only, fenced blocks excluded, per author.
- Font-size target set to the prototype's `11.5px` per author decision, in
  preference to keeping the current 14px or a token-relative size.
- Acceptance criteria and Technical Notes derived from a direct prototype-vs-
  renderer comparison; priority confirmed `medium`.
- `--ac-stroke-soft` confirmed present in the live `global.css` during review
  (light `rgba(32,34,49,0.06)` / dark `rgba(255,255,255,0.04)`, matching the
  prototype), resolving the prior open question; acceptance criteria tightened
  to assert computed-style values.

## References

- Source: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
  (a dated design-inventory snapshot — a frozen reference, not a living target)
- Code: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css`,
  `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx`,
  `skills/visualisation/visualise/frontend/src/styles/global.css`
- Related: 0076, 0088, 0089, 0095
