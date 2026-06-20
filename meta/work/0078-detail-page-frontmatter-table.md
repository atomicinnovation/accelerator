---
id: "0078"
title: "Detail-Page Frontmatter Table"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: done
priority: medium
tags: [design, frontend, detail-page, markdown]
type: work-item
schema_version: 1
last_updated: "2026-05-21T09:16:34+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0041"]
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
relates_to: ["work-item:0084", "work-item:0088", "work-item:0085"]
external_id: PP-100
---

# 0078: Detail-Page Frontmatter Table

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Render every frontmatter key/value pair (with dimmed dashes for empty
values) as a CSS-grid table above the markdown body on each detail
page, with work-item ID values (per the configured `work.id_pattern`,
including project-prefixed forms) auto-linkified. The table complements
(does not replace) the chip strip in the subtitle slot — chip strip
carries the headline signal, table carries the full frontmatter.

## Context

The prototype renders a `.ac-fm` CSS-grid (`auto 1fr`, Fira Code 11.5px,
`--ac-bg-sunken` background, `--ac-stroke` border, padding `12px 14px`)
showing every frontmatter key/value pair directly above the markdown
body. The current app's `FrontmatterChips` renders the same information
as a chip strip in the subtitle slot, which collapses everything to
pill width and forces awkward truncation on long values — captured
screenshots show a 40-character git commit hash on research, a full
file path on design-gaps, and a six-lens comma list on plan-reviews.

The chip strip and the table communicate at different fidelity levels;
both are needed. Chip-strip rebalancing is a separate item (0084).

## Requirements

- Add a frontmatter table component rendered above the markdown body on
  every detail page, between the page header and the markdown body.
- The table is a CSS grid with two columns: key (auto width) and value
  (`1fr`), styled per prototype (Fira Code 11.5px, sunken background,
  stroke border, `12px 14px` padding).
- Rows render in **source order** — the order keys appear in the file's
  YAML frontmatter — with no canonical sort imposed.
- The table is **always expanded** — no collapse affordance.
- Every frontmatter key declared in the source file is rendered as a
  row, including keys whose values are null, undefined, empty string, or
  empty array; empty values render with a dimmed em-dash in the value
  column (`var(--ac-text-muted)`).
- Scalar values matching the configured `work.id_pattern` (default
  `WORK-####` and any configured project-prefixed form such as
  `PROJ-####`) auto-linkify to the corresponding work-item route, using
  the existing `useWikiLinkResolver` so the table and markdown body
  resolve identically.
- Array values render as a comma-separated list in the value column;
  array elements matching a work-item pattern render as individual
  anchors separated by plain-text commas. Object values render as a
  JSON-serialised string (parity with `FrontmatterChips`).
- Table width is capped to the same max-width as the markdown body
  (per 0088).
- The table is consumed by the existing detail-page loader's frontmatter
  parse output — no new endpoint needed.

## Acceptance Criteria

- [ ] On each detail-page route, a frontmatter table component is
  rendered above the markdown body and below the page header.
- [ ] Every frontmatter key declared in the source file is rendered as
  a row, in source order, one row per key. Verified against a canonical
  work-item fixture containing the standard story frontmatter keys
  (`work_item_id, title, date, author, kind, status, priority, parent,
  tags`) in that order: the table renders nine rows in that order.
- [ ] Keys with null, undefined, empty-string, or empty-array values
  render with a dimmed em-dash in the value column rather than being
  omitted.
- [ ] Scalar values matching the configured `work.id_pattern` (default
  `WORK-####` and any configured project-prefixed pattern e.g.
  `PROJ-####`) render as anchor links to the matching work-item route,
  resolved through `useWikiLinkResolver`.
- [ ] Array values render as a comma-separated list in the value
  column; each array element matching a work-item pattern renders as
  its own anchor, with commas between anchors as plain text.
- [ ] Values containing both free text and a work-item token render
  the matching substring as an anchor and the surrounding text as
  plain text (e.g., `see WORK-0041 for context` renders `WORK-0041`
  as a link with the rest of the value as plain text).
- [ ] Object values render as a JSON-serialised string (matching
  `FrontmatterChips` behaviour for parity).
- [ ] Table uses CSS grid with template `auto 1fr` (key column auto
  width, value column `1fr`).
- [ ] Text in the table is rendered in Fira Code at 11.5px.
- [ ] Container background uses the `--ac-bg-sunken` CSS variable.
- [ ] Container border uses the `--ac-stroke` CSS variable.
- [ ] Container padding is `12px 14px`.
- [ ] The table's computed `max-width` equals the markdown body's
  computed `max-width` at the same viewport width (per 0088 — if 0088
  has not yet shipped, the table uses the body's pre-0088 width).
- [ ] The chip strip (`FrontmatterChips`) continues to render in the
  subtitle slot unchanged; chip-strip rebalancing is handled by 0084.
- [ ] The table is always expanded — no toggle, no collapse affordance.

## Open Questions

None — all decisions captured in Assumptions and Drafting Notes.

## Dependencies

- Blocked by: 0041 (page wrapper provides the layout slot).
- Related: 0084 (chip-strip cap — can ship independently in either
  order; paired delivery on the detail page is preferred but not
  required, so 0078 neither blocks nor is blocked by 0084).
- Related: 0088 (markdown body width harmonisation — table width must
  match body width cap).
- Related: 0085 (H1 humanisation — sits directly above the table).
- Builds on: 0043 (spike establishing wiki-link resolver pattern
  coverage — informational only, no scheduling dependency).
- Reuses: existing `useWikiLinkResolver` and `wiki-links` resolver
  (no new infrastructure).

## Assumptions

- The detail-page loader (`LibraryDocView`) already exposes the parsed
  frontmatter object; no loader changes are required.
- The existing wiki-link resolver covers every WORK pattern the table
  needs to linkify — the table reuses the same resolver as the markdown
  body, not a separate regex. If the resolver's pattern coverage is
  narrower than expected, the table's linkification matches the
  markdown body's exactly (consistent, even if incomplete).
- Numbers `0` and boolean `false` are valid values and render as
  `0` / `false` (not as the dimmed em-dash used for null/undefined/
  empty-string/empty-array values).
- The table renders below the page header and above the markdown body,
  inside the same width-capped column as the body — not in the aside
  region (0079).

## Technical Notes

- New component
  `frontend/src/components/FrontmatterTable/FrontmatterTable.tsx`
  (+ `.module.css`, `.test.tsx`) parallel to existing
  `FrontmatterChips/`.
- Mount in `frontend/src/routes/library/LibraryDocView.tsx` between the
  existing `FrontmatterChips` block (~line 80) and the
  `MarkdownRenderer` block (~line 120).
- Reuse `useWikiLinkResolver` from
  `frontend/src/api/use-wiki-link-resolver.ts` for linkification; pass
  `resolveWikiLink` into the table component the same way
  `MarkdownRenderer` consumes it.
- Reuse `formatChipValue`'s array/object handling logic where
  applicable, but diverge on null/empty handling (chips skip, table
  renders dimmed dash).
- CSS grid styling uses existing CSS variables `--ac-bg-sunken`,
  `--ac-stroke`, `--ac-text-muted` (for dimmed dash); Fira Code is
  already in the typography stack.

## Drafting Notes

- "Always expanded" interpreted as no collapse UI at all — not
  "expanded by default but collapsible". If a toggle is wanted later,
  that's a separate story.
- Row ordering follows the file's YAML key order, which means key
  order is implicitly governed by the work-item template and producer
  scripts. If a producer emits keys in an unexpected order, the table
  reflects that — no canonical sort is imposed.
- Linkification pattern is delegated to the existing resolver rather
  than re-implemented, so the table inherits whatever patterns the
  resolver supports (currently `WORK-####`, `[[ADR-N]]`,
  `[[WORK-ITEM-N]]`, project-prefixed forms). If a value contains free
  text mixed with a WORK token, only the matching substring is
  linkified — the rest remains plain text.
- The table is a complete dump (with dimmed dashes for empty values);
  the chip strip is curated (skips empties, caps to four per 0084).
  Two different fidelity surfaces, deliberately diverging behaviours.
- Width-cap matching to the markdown body (0088) is asserted as a
  styling AC, not deferred, because a table that overhangs the body
  looks broken; if 0088 ships after 0078, the table just uses the
  body's pre-0088 width.
- Frontmatter field renamed `type` → `kind` to align with the
  repository-wide schema rename (commits e843e2252, b9266ae7c, etc.).

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0041, 0084, 0085, 0088
- Spike context: 0043 (existing wiki-link plugin behaviour)
- Code: `frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`,
  `frontend/src/api/use-wiki-link-resolver.ts`,
  `frontend/src/routes/library/LibraryDocView.tsx`
