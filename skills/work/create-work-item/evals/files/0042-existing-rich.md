---
work_item_id: "0042"
title: "Add Full-Text Search to Documentation Index"
date: "2026-01-15T09:00:00+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [search, documentation]
---

# 0042: Add Full-Text Search to Documentation Index

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a documentation user, I want to search for content by keyword so that I can find relevant pages without browsing the full index manually. This affects all users who rely on the docs for implementation guidance, and becomes more painful as the documentation grows.

## Context

The documentation site currently has no search capability. Users must navigate via the sidebar hierarchy or use browser Ctrl+F on individual pages. As the docs have grown to over 200 pages, discoverability has become a common complaint in user feedback surveys (Q4 2025: 34% of respondents cited navigation difficulty). The engineering team has evaluated three candidates: Pagefind (static, no backend), Algolia DocSearch (hosted, free for open-source), and a simple Fuse.js client-side index.

## Requirements

- A search input must be accessible from every documentation page, either in the header or a persistent sidebar panel.
- Queries must return ranked results drawn from all indexed documentation pages within 2 seconds on a standard broadband connection.
- Each result must show the page title, a short content excerpt matching the query, and a direct link to the matched section.
- The search index must rebuild automatically as part of the documentation CI/CD pipeline — no manual re-indexing step.
- The solution must support the current MkDocs-based documentation site without requiring a migration to a different site generator.

## Acceptance Criteria

- Given a documentation user types a keyword into the search input, when they submit the query, then results appear within 2 seconds with titles, excerpts, and links.
- Given the documentation CI pipeline runs after a content commit, when the build completes, then the search index reflects the new or changed page content.
- Given a user clicks a search result link, when the page loads, then the browser scrolls to the matched section heading.
- Given the search input is focused, when the user presses Escape, then the search panel closes and focus returns to the previously focused element.

## Open Questions

- Should results be scoped to the current documentation version, or should cross-version search be supported from day one?

## Dependencies

- Blocked by: nothing
- Blocks: nothing

## Assumptions

- Pagefind is the preferred solution unless integration testing reveals a blocking incompatibility with MkDocs.
- "Documentation" means the public-facing docs site, not internal engineering docs.

## Technical Notes

Pagefind builds a static index at documentation build time and ships it alongside the HTML. No backend or external API key is required. Integration with MkDocs is via a post-build hook that runs `pagefind --site site/`.

## Drafting Notes

- Scoped to the public documentation site based on the user's phrasing; internal engineering wiki excluded.
- "Within 2 seconds" treated as a p95 target on broadband, not a hard SLA — would need clarification if there is a formal performance budget.
- Pagefind chosen as default recommendation because it has zero operational cost; final decision deferred to implementation spike.

## References

- Research: Pagefind documentation at https://pagefind.app
- Research: Algolia DocSearch at https://docsearch.algolia.com
- Related: none
