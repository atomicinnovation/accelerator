---
work_item_id: "0084"
title: "Detail-Page Chip Strip Cap (Status, Date, Author)"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: in-progress
priority: medium
parent: ""
tags: [design, frontend, detail-page, chips]
---

# 0084: Detail-Page Chip Strip Cap (Status, Date, Author)

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a reader of any detail page, I want the subtitle chip strip to
show a small, predictable set of identity-and-state chips, so that
the page header is scannable and consistent regardless of doc kind.

Rebalance the chip strip in the detail-page subtitle slot so it
displays a hard-limited set of at most three chips — status, date,
author — and moves all other frontmatter content to the
frontmatter table delivered by 0078.

## Context

The current app's chip strip in the subtitle slot includes every
frontmatter key for some doc kinds (work-item: id, type, status,
priority, tags, title, author, date) and is empty for others (notes
render with no chips, leaving H1 sitting directly over the divider).
On design-inventory pages the chips duplicate author and timestamp
values (`last_updated` mirrors `date`, `last_updated_by` mirrors
`author`), and on design-gap and research pages chips carry full
file paths or 40-character git hashes that dominate the strip.

The prototype's chip row caps at four chips (status + verdict +
date + author). This story adopts a three-chip cap
(status + date + author) instead — `verdict` is dropped because a
verdict chip beside a status chip creates two coloured-tone slots
that compete for attention rather than complement each other.
`verdict` continues to surface in the frontmatter table (0078).

## Requirements

- Enforce a hard maximum chip set of three, in fixed left-to-right
  order: status → date → author.
- Render only the chips whose underlying frontmatter key exists and
  is non-null/non-empty-string; the strip can render fewer than three
  but never more, and never reorders.
- When zero chips qualify (e.g. a notes file with no `date` or
  `author`), the strip renders an empty container with a rendered
  height equal to that of a one-chip strip, so the subtitle slot
  occupies the same vertical space regardless of chip count and the
  H1 remains vertically offset from the divider.
- Remove every other key currently surfaced in the chip strip on any
  doc kind (id, type/kind, priority, tags, title, slug, paths, hashes,
  mirrors, verdict).
- The cap is enforced inside `FrontmatterChips`; callers cannot opt
  more chips in.
- The frontmatter table (0078) remains the canonical surface for
  every other frontmatter key.

## Acceptance Criteria

- [ ] Given any detail-page route, when the page renders, then the
  chip strip displays at most three chips, drawn only from the
  canonical set `{status, date, author}`, in that fixed order.
- [ ] Given a doc whose frontmatter omits `status`, `date`, or
  `author` (or has them as null/empty string), when the page renders,
  then the corresponding chip is omitted (no placeholder).
- [ ] Given a doc with all three keys, when the page renders, then
  exactly three chips appear, ordered status → date → author.
- [ ] Given a notes file with only `date` and `author`, when the
  page renders, then two chips appear in canonical order (`date`,
  then `author`) and the chip-strip container has non-zero rendered
  height.
- [ ] Given a doc with none of `status`, `date`, or `author`, when
  the page renders, then the chip-strip container renders empty with
  a rendered height equal to that of a one-chip strip, keeping the
  H1 vertically offset from the divider.
- [ ] Given any non-empty subset of {`status`, `date`, `author`}
  present in frontmatter, when the page renders, then the chips
  appear in canonical order (status → date → author) with no
  reordering, regardless of which keys are absent.
- [ ] Given a doc with `last_updated` alongside `date`, the `date`
  chip (sourced from the `date` frontmatter key) uses the
  creation-anchored value; `last_updated` is rendered only in the
  frontmatter table.
- [ ] Given a doc with `last_updated_by` alongside `author`, the
  `author` chip (sourced from the `author` frontmatter key) uses
  that value; `last_updated_by` is rendered only in the frontmatter
  table.
- [ ] No doc kind exposes any non-canonical frontmatter key as a
  chip (verified across the 12 doc kinds: decisions, work-items,
  plans, research, plan-reviews, pr-reviews, work-item-reviews,
  validations, notes, pr-descriptions, design-gaps,
  design-inventories). Verified by a parameterised test in
  `FrontmatterChips.test.tsx` that iterates the 12 kinds with
  fixture frontmatter containing extra non-canonical keys, asserting
  that none of those keys appear as a chip.

## Dependencies

- Blocked by: none (0038 chip primitive, 0078 frontmatter table, and
  0081 StatusBadge are all `status: done`).
- Depends on (schema): ADR-0033 (unified base frontmatter schema —
  governs the canonical key set this story's whitelist effectively
  pins).
- Related: 0038 (chip primitive — provides the rendered chip), 0078
  (frontmatter table — destination for everything else), 0081
  (StatusBadge — renders the `status` chip's tone).
- Enables (once created): a future schema-alignment story (TBD) to
  add `author` to plans / pr-descriptions / all three review
  templates; that story's 3-chip outcome is gated on this
  whitelist being in place.
- Blocks: none.

## Assumptions

- Frontmatter values are treated as "missing" when the key is absent,
  `null`, an empty string, or a whitespace-only string (values are
  trimmed before chip eligibility is evaluated). Numeric `0` and
  boolean `false` are not currently chip-eligible values for the
  canonical set, so this rule has no edge cases for
  status/date/author.
- The schema-alignment work to add `author` to plans /
  pr-descriptions / review templates is **out of scope** for this
  story; once those templates gain `author`, the chip strip picks it
  up automatically without further code change.

## Open Questions

- The schema-alignment follow-up story (adding `author` to plans,
  pr-descriptions, plan-reviews, work-item-reviews, and pr-reviews
  templates) has no ID yet. Once captured, update Dependencies to
  name it under "Enables (once created)" and update Blocks
  accordingly.
- Full alignment of this work item's frontmatter to ADR-0033's
  unified base schema (renaming `work_item_id` → `id` and adding
  `type: work-item`, `producer`, `schema_version`, `last_updated`,
  `last_updated_by`) is deferred to the corpus migration tracked
  under epic 0057. This story does not migrate those fields in
  flight; the partial alignment already present (`kind:` over
  `type:`) reflects opportunistic migration only.

## Technical Notes

- Implementation point: `src/components/FrontmatterChips/FrontmatterChips.tsx`
  (with `FrontmatterChips.module.css` and `FrontmatterChips.test.tsx`
  beside it).
- The cap is a property of `FrontmatterChips` itself — a fixed
  whitelist of `{status, date, author}` keys, applied uniformly
  across all consumers. No prop opens the whitelist back up.
- Status chip rendering already goes through `StatusBadge` (0081);
  this story does not change `StatusBadge` or the status→tone map.
- Verdict-bearing review kinds keep showing `verdict` in their
  frontmatter table (0078), not in the chip strip.

## Drafting Notes

- Extracted from source documents without interactive enrichment;
  refined during a /create-work-item enrich-existing pass on
  2026-05-24.
- Three-chip cap (vs. the prototype's four) is a deliberate
  divergence: a coloured verdict chip beside a coloured status chip
  competes for attention. Verdict surfaces in the frontmatter table
  instead.
- Chip order locked at status → date → author for visual consistency
  across doc kinds and to keep `FrontmatterChips` deterministic.
- `date` chip preferentially shows the **creation-anchored** value
  (`date`), not `last_updated`. Rationale: chip strip communicates
  identity ("who/when did this start"); recency lives in the
  frontmatter table.
- Schema gap recorded but not fixed here: 6 of 12 doc kinds (plans,
  pr-descriptions, plan-reviews, work-item-reviews, pr-reviews, and
  conditionally notes/validations) currently have no `author` field
  in their templates, so they'll render 2 chips today. A follow-up
  schema-alignment story would lift them to 3 — that story is
  separately planned by the author of this work item.
- `kind:` adopted in place of `type:` to align with ADR-0033's
  unified base frontmatter schema; siblings in `meta/work/` are
  currently mixed and migrating opportunistically.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0038, 0078, 0081
- Schema: `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`
  (rationale for the `type:` → `kind:` migration)
