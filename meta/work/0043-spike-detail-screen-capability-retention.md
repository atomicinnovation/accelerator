---
work_item_id: "0043"
title: "Spike: Confirm Detail-Screen Capability Retention"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: spike
status: draft
priority: high
parent: ""
tags: [design, spike, scope]
---

# 0043: Spike: Confirm Detail-Screen Capability Retention

**Type**: Spike
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Confirm with stakeholders that the prototype's omission of per-document detail screens, wiki-link rewrite, malformed-frontmatter banner, and the deferred-fetching hint are unintentional TBDs rather than deliberate removals — so existing capability can be retained and re-skinned with the new design system.

## Context

The current app provides per-document detail screens at `/library/{type}/{slug}` rendering frontmatter chips, the markdown body via `MarkdownRenderer`, a related-artifacts aside (Targets / Referenced by / Same lifecycle), and inline "Document not found." error states. The prototype contains no per-document detail screen at all — clicking a row in `library-plans` / `library-decisions` is a no-op.

The current app's `MarkdownRenderer` integrates `react-markdown` + `remarkGfm` + a custom `remarkWikiLinks` plugin that rewrites `[[ADR-N]]`, `[[WORK-ITEM-N]]`, and `[[WORK-ITEM-PROJ-N]]` markers into anchor links, pending markers, or unresolved markers. The prototype demonstrates markdown rendering only inside the templates tier-preview pane and shows no wiki-link affordance.

The current app surfaces a malformed-frontmatter banner inside per-document detail when the backend reports a YAML parse failure (`state === 'malformed'`). The prototype includes no analogous banner because it has no detail screen.

The current app's `useDeferredFetchingHint` exposes a 250ms-debounced "Updating…" hint to suppress flicker on fast re-fetches inside the related-artifacts aside on detail pages. The prototype shows no analogous hint because it lacks the surfaces that consume it.

The gap analysis explicitly flags all four as TBDs in the prototype crawler — the absence is suspected to be incompleteness rather than removal — but stakeholder confirmation is required before any redesign work proceeds.

## Requirements

This spike must produce confirmed answers to the following four questions, captured as ADRs or as updates to the gap-analysis document:

1. **Detail-screen retention**: Are per-document detail screens at `/library/{type}/{slug}` retained? If yes, the redesign must apply the new design system (token layer, Glyph, Chip, Page wrapper) to the existing detail surface.
2. **Wiki-link rewrite retention**: Is the `[[ADR-N]]` / `[[WORK-ITEM-N]]` / `[[WORK-ITEM-PROJ-N]]` rewrite a required capability? It is load-bearing for cross-document navigation today.
3. **Malformed-frontmatter banner retention**: Is the banner retained on the redesigned detail screen?
4. **Deferred-fetching hint retention**: Is `useDeferredFetchingHint` (the 250ms-debounced "Updating…" hint) retained?

Time-box: ~1 day to gather stakeholder confirmation and document outcomes.

## Acceptance Criteria

- [ ] Each of the four questions above has a written, stakeholder-confirmed answer (yes/no, with optional design notes).
- [ ] Outcomes are captured either in updates to the gap-analysis document or as one or more ADRs.
- [ ] Where retention is confirmed, follow-up work items are created to apply the new design system to the retained surfaces.

## Open Questions

- Who is the stakeholder for these decisions — the design owner, the product owner, or both?

## Dependencies

- Blocked by: none.
- Blocks: any future "redesigned detail screen" work item (cannot proceed until scope is confirmed).

## Assumptions

- The "TBD" interpretation in the gap-analysis prototype crawler is correct — the prototype is incomplete rather than prescriptive about detail screens.

## Technical Notes

- These questions are not technical design questions; they are scope confirmations. The spike is research-only.

## Drafting Notes

- Bundled four "confirm with stakeholders" questions into one spike because they all concern the detail-screen capability surface and can be confirmed in a single conversation.
- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Related: 0033, 0037, 0038, 0041
