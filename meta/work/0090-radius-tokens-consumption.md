---
work_item_id: "0090"
title: "Radius Tokens Consumption"
date: "2026-05-23T00:00:00+00:00"
author: Toby Clemson
type: story
status: draft
priority: low
parent: ""
tags: [design, frontend, tokens, radius]
---

# 0090: Radius Tokens Consumption

**Type**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Establish a radius token scale (`--radius-*`) and migrate current-app
component CSS so every `border-radius` declaration resolves to a
`var(--radius-*)` token, mirroring the typography-size precedent set by
0075.

## Context

0075 carved radius outliers out of its consumption-reconciliation scope.
Two known current-app outliers are documented there:

- `RelatedArtifacts` badge `border-radius: 2px`
- Markdown `<pre>` `border-radius: 6px`

Beyond these two, current-app radius usage has not been inventoried; a
discovery sweep is part of this story.

## Requirements

- Define a `--radius-*` token scale covering at least the px values
  currently in use across the current app (including `2px` and `6px`).
- Migrate every current-app `border-radius` declaration to a
  `var(--radius-*)` token reference. No literal px or rem `border-radius`
  values in current-app CSS.
- Land the migration in a single PR series so consumption is uniform on
  merge.

## Open Questions

- Discovery sweep needed to enumerate all current-app radius usages.
- Naming convention for the radius scale (e.g. `--radius-xs` … or named
  per use-case like `--radius-badge`, `--radius-block`).

## Dependencies

- Related: 0033 (token infrastructure precedent), 0075 (typography
  size-scale consumption — establishes the consume-tokens-everywhere
  pattern this story extends to radius).

## References

- Source carve-out: `meta/work/0075-typography-size-scale-consumption.md`
  (Assumptions, Drafting Notes).
