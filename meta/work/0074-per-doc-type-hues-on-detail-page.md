---
work_item_id: "0074"
title: "Per-Doc-Type Hues on Detail Page"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, detail-page, tokens]
---

# 0074: Per-Doc-Type Hues on Detail Page

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Surface the existing `--ac-doc-<key>` and `--ac-doc-bg-<key>` tokens
(introduced by 0037) on the detail page so the eyebrow, aside related
rows, and any future hero illustration carry consistent per-doc-type
colour. Today these tokens exist but are explicitly restricted to the
sidebar and library hub.

## Context

The current app declares `--ac-doc-{key}` and `--ac-doc-bg-{key}` tokens
at `src/styles/global.css:98-124` but explicitly does not consume them on
the detail page — they are restricted to the sidebar and library hub.
The prototype defines a thirteen-entry HSL map in `src/ui.jsx:264-278`
(`work` hue 12, `decisions` 355, `research` 28, `plans` 220, `notes` 50,
`design-inventories` 185, `design-gaps` 95, …) consumed by `TypeGlyph`,
`StageTile`, `BigGlyph`, and the empty-page tint.

## Requirements

- Surface `--ac-doc-<key>` and `--ac-doc-bg-<key>` tokens to the detail
  page surface so eyebrow text, aside related-row tints, and the future
  hero illustration slot consume them.
- Keep the sidebar and library hub consumption unchanged.
- Identify each detail-page surface that currently hard-codes a neutral
  colour where a per-doc-type tint would apply.

## Acceptance Criteria

- [ ] On each detail page route, the page eyebrow consumes the
  `--ac-doc-<key>` token for the current doc type.
- [ ] Aside related-row entries consume per-doc-type tint via existing
  tokens (specific mapping settled during implementation).
- [ ] A `data-doc-type="<key>"` attribute on the page root scopes
  per-doc-type token application without per-component conditionals.

## Open Questions

- Which specific aside row elements should consume the doc-type tint —
  the row icon only, a left-border accent, or background tint?

## Dependencies

- Blocked by: 0037 (per-doc-type tokens already delivered).
- Related: 0082 (BigGlyph hero illustration also consumes these tokens).

## Assumptions

- The thirteen doc-type keys in the prototype's hue map align with the
  twelve non-virtual `DocTypeKey` values plus one currently unmapped key;
  alignment confirmed during implementation.

## Technical Notes

- 0037 introduced the token sub-namespace; this story is consumption-only.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0037, 0082
