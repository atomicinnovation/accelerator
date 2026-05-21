---
work_item_id: "0075"
title: "Typography Size-Scale Consumption Reconciliation"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, tokens, typography]
---

# 0075: Typography Size-Scale Consumption Reconciliation

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Settle a single canonical rule for how the typography size scale is
consumed (either everything pulls from `--size-*` tokens or the scale is
dropped), then migrate every hard-coded outlier in component CSS onto
that rule.

## Context

The brand size scale (`--size-hero` … `--size-chip-md`) is defined in
both the current app and the prototype stylesheets at identical px
values, but consumption diverges sharply. The current app consumes the
tokens for some surfaces (Page H1 uses `--size-h3`, chips use `--size-chip`,
markdown body inherits `--size-sm`) while the prototype defines them and
then hard-codes pixel values per component (body `14px`, page H1 `28px`,
eyebrow `11px`, chip `10.5px`, markdown body `14.5px`).

The current app also has its own off-scale outliers:
- `MarkdownRenderer` H1 at `1.75rem`
- `Page.module.css` eyebrow at `11px`, subtitle at `13px`
- `RelatedArtifacts` badge radius at `2px`
- Markdown `<pre>` radius at `6px`

## Requirements

- Decide between two canonical rules:
  1. **Consume tokens everywhere** — migrate every hard-coded `font-size`
     to a `var(--size-*)` reference; widen the scale where no token fits.
  2. **Drop the scale** — remove `--size-*` tokens and replace consumer
     references with the literal px values the prototype hard-codes.
- Migrate every identified outlier onto the chosen rule in a single PR
  series so consumption is uniform.
- Document the decision in the PR description so future contributors
  understand the rule.

## Acceptance Criteria

- [ ] A written decision exists naming the chosen rule and the reason.
- [ ] Every outlier listed in Context is migrated onto the chosen rule.
- [ ] A grep for hard-coded `font-size` literals in component CSS
  modules returns either zero matches (rule 1) or only the expected
  prototype-aligned px values (rule 2).

## Open Questions

- Which rule is canonical — token consumption or px literals? The
  prototype's behaviour of defining tokens then ignoring them suggests
  the prototype's intent is unclear; decision required.

## Dependencies

- Blocked by: 0033 (size scale tokens defined).
- Blocks: any downstream component-redesign work that touches typography.

## Assumptions

- The chosen rule applies uniformly; no per-component carve-outs.

## Technical Notes

- 0033 introduced the eleven-step `--size-*` scale; that scope did not
  enforce uniform consumption.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0033
