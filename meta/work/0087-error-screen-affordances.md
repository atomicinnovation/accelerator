---
id: "0087"
title: "404 / Error Screen with Affordances"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: draft
priority: low
tags: [design, frontend, error-states]
type: work-item
schema_version: 1
last_updated: "2026-05-21T09:16:34+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0041"]
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
relates_to: ["work-item:0054"]
---

# 0087: 404 / Error Screen with Affordances

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Replace the current bare `Document not found` screen with a 404 / error
surface that offers at minimum a link back to the library landing and
the parent type's list, and ideally search suggestions for nearby
slugs.

## Context

The current app's `error-not-found` renders the `Page` shell with H1
`"Document not found"` and a single `<p>Document not found.</p>` line —
no back-link, retry, or related-suggestions affordance. The prototype
has no 404 screen at all (unknown hashes were not exercised), so the
current implementation supersedes the prototype here; the gap analysis
flags the missing affordances as the gap to close.

## Requirements

- Replace the bare `Document not found` body with a 404 surface
  containing:
  - A `Back to library` link to `/library`.
  - A `Back to {type} list` link to the parent doc-type list view
    (when the missing route's doc type can be inferred from the URL).
  - A `Did you mean…` block listing up to five nearby slugs derived
    from substring or Levenshtein match against the indexer's known
    slugs.
- Keep the existing H1 ("Document not found") and Page wrapper chrome.

## Acceptance Criteria

- [ ] Navigating to a non-existent document URL renders the 404 surface
  with the new affordances.
- [ ] The `Back to library` link is present and works.
- [ ] When the URL's doc-type segment matches a known doc type, the
  `Back to {type} list` link renders pointing to that type's list view.
- [ ] When at least one nearby-slug suggestion exists, the
  `Did you mean…` block renders up to five suggestions as links.
- [ ] When no suggestions exist, the `Did you mean…` block is omitted
  (not rendered as empty).

## Open Questions

- Is substring match sufficient for nearby-slug suggestions, or is
  Levenshtein/fuzzy match worth the extra complexity?
- Should the suggestion ranking incorporate mtime (more recent first)?

## Dependencies

- Blocked by: 0041 (Page wrapper).
- Related: 0054 (Sidebar search infrastructure could power suggestions,
  but is optional).
- Blocks: none.

## Assumptions

- The indexer already exposes the full list of known slugs, so client-
  side suggestion generation is possible without a new endpoint.

## Technical Notes

- The route component for `error-not-found` is the integration point.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0041, 0054
