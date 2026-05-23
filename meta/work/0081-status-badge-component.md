---
work_item_id: "0081"
title: "StatusBadge — Decompose FrontmatterChips and Map Status + Verdict to Tone"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: done
priority: medium
parent: ""
tags: [design, frontend, components, chips]
---

# 0081: StatusBadge — Decompose FrontmatterChips and Map Status + Verdict to Tone

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a reader of plan-review, work-item-review, and validation pages, I want
verdict and result chips to carry the same semantic colour as status chips,
so that review and validation outcomes signal themselves at a glance
without my having to read the chip label.

Today `FrontmatterChips` bundles three concerns: (a) rendering a list of
chips from a frontmatter document, (b) tone logic for `status` (and,
conceptually, other outcome-bearing keys), and (c) generic per-key chip
rendering. This story decomposes that component into a chip-list
dispatcher (which retains the `FrontmatterChips` name and call sites), a
new generic `FrontmatterChip`, and three tone-aware wrappers —
`StatusBadge`, `VerdictBadge`, and `ResultBadge` — each composing
`FrontmatterChip` and resolving its vocabulary's frontmatter values to
coloured `Chip` variants from 0038 (the `Chip` primitive).

## Context

The prototype's `StatusBadge` maps both `status` and `verdict` frontmatter
keys to a coloured chip tone (`Accepted` → green, `Draft` → amber, `pass`
→ green, and so on). The current app's `FrontmatterChips` only colours the
`status` key via `statusToChipVariant` and renders the `verdict` and
`result` keys as neutral — the validation detail page renders the `pass`
result as a neutral chip with no semantic colour.

Three distinct outcome vocabularies coexist in the corpus: validation
emits `result: pass | partial | fail`; plan-review and work-item-review
emit `verdict: APPROVE | REVISE | COMMENT`; PR-review emits
`verdict: APPROVE | REQUEST_CHANGES | COMMENT`. This work item maps each
vocabulary to chip tones via its own helper (`statusToVariant`,
`verdictToVariant`, `resultToVariant`); it does not unify the
vocabularies.

## Requirements

- Decompose `FrontmatterChips` into a chip-list renderer plus a family
  of per-key chip components, each with a single responsibility:
  - **Chip-list renderer** (the refactored `FrontmatterChips` — same
    name, same call sites, narrower responsibility): walks a frontmatter
    document and dispatches each key to the appropriate chip component
    in frontmatter source order. Owns ordering and dispatch, not per-chip
    semantics.
  - **`FrontmatterChip`** (new): generic per-key chip. Renders one
    `key: value` pair as a `Chip` from 0038, with a default `neutral`
    tone and no domain-specific knowledge of any vocabulary.
  - **`StatusBadge`** (new): composes `FrontmatterChip`, resolves
    `status` values to a coloured `Chip` variant via `statusToVariant`.
    Dispatched to by the chip-list renderer for the `status` key.
  - **`VerdictBadge`** (new): composes `FrontmatterChip`, resolves
    `verdict` values via `verdictToVariant`. Dispatched to for the
    `verdict` key.
  - **`ResultBadge`** (new): composes `FrontmatterChip`, resolves
    `result` values via `resultToVariant`. Dispatched to for the
    `result` key.
- Canonical tone mapping (using 0038's variant identifiers — 0038 ships
  six: `neutral` / `indigo` / `green` / `amber` / `red` / `violet`):
  - **`status`** (canonical set defined in
    `frontend/src/api/status-variant.ts` — preserve current mapping
    exactly). All values below are literal string matches against the
    `status` frontmatter value (case-insensitive, per the lookup rule):
    | Status value                                | Variant  |
    |---------------------------------------------|----------|
    | `Accepted`, `Done`                          | green    |
    | `In progress`, `live`, `Proposed`, `active` | indigo   |
    | `Approve w/ changes`                        | amber    |
    | `Todo`, `absent`                            | neutral  |
    | Any other value (including dates, author-name-shaped strings, typos, empty) | neutral |
  - **`verdict`** (plan-review and work-item-review): `APPROVE` → green,
    `REVISE` → amber, `REQUEST_CHANGES` → red, `COMMENT` → neutral.
  - **`result`** (validation): `pass` → green, `partial` → amber,
    `fail` → red.
  - Any unmapped value falls back to `neutral`.
- Value lookup is **case-insensitive** across all three vocabularies:
  `approve`, `Approve`, and `APPROVE` all resolve to the same chip
  variant.
- Apply across plan-review and work-item-review pages (verdict chips)
  and validation pages (result chips).

## Acceptance Criteria

**Decomposition**

- [ ] Given a frontmatter document with keys `status: Accepted`,
  `verdict: APPROVE`, `priority: medium`, and `tags: [design, frontend]`,
  when the chip-list renderer renders, then four chips paint in
  frontmatter source order with variants `green`, `green`, `neutral`,
  `neutral` respectively.
- [ ] Given the same fixture, each rendered chip carries an observable
  hook (a `data-testid` attribute) identifying which component produced
  it — `StatusBadge` for the `status` chip, `VerdictBadge` for the
  `verdict` chip, `FrontmatterChip` for the `priority` and `tags`
  chips — so the dispatch contract is testable from the DOM.
- [ ] Given `FrontmatterChip` is rendered directly (not via the
  chip-list renderer) with `name="status"` and `value="Accepted"`, when
  it paints, then the chip uses the `neutral` variant — i.e.
  `FrontmatterChip` itself applies no domain-specific tone logic.
- [ ] Given `StatusBadge` is rendered with any value, when it paints,
  then it resolves to a coloured `Chip` variant per the canonical
  status tone mapping in Requirements.
- [ ] Given `VerdictBadge` is rendered with any value, when it paints,
  then it resolves to a coloured `Chip` variant per the canonical
  verdict tone mapping in Requirements.
- [ ] Given `ResultBadge` is rendered with any value, when it paints,
  then it resolves to a coloured `Chip` variant per the canonical
  result tone mapping in Requirements.
- [ ] Given a frontmatter document with keys in a specific source order
  (e.g. `verdict`, then `status`, then `priority`), when the chip-list
  renderer renders, then the chips appear in that same source order.

**Status tone (preserves current `statusToVariant` mapping)**

- [ ] Given a frontmatter document with `status: Accepted`, when
  `StatusBadge` renders, then the chip uses the `green` variant.
- [ ] Given a frontmatter document with `status: Done`, when
  `StatusBadge` renders, then the chip uses the `green` variant.
- [ ] Given a frontmatter document with `status: In progress`,
  `Proposed`, `live`, or `active`, when `StatusBadge` renders, then
  the chip uses the `indigo` variant.
- [ ] Given a frontmatter document with `status: Approve w/ changes`,
  when `StatusBadge` renders, then the chip uses the `amber` variant.
- [ ] Given a frontmatter document with `status: Todo` or `absent`,
  when `StatusBadge` renders, then the chip uses the `neutral`
  variant.
- [ ] Given a frontmatter document with `status` set to an unmapped
  value (e.g. `SomeUnknownValue`, a date like `2026-05-21`, or an
  arbitrary author-shaped string), when `StatusBadge` renders, then
  the chip falls back to the `neutral` variant.

**Result tone — validation vocabulary**

- [ ] Given a frontmatter document with `result: pass`, when the
  validation detail page renders, then the `result` chip uses the
  `green` variant (no longer neutral).
- [ ] Given a frontmatter document with `result: partial`, when the
  validation detail page renders, then the `result` chip uses the
  `amber` variant.
- [ ] Given a frontmatter document with `result: fail`, when the
  validation detail page renders, then the `result` chip uses the
  `red` variant.

**Verdict tone — plan-review and work-item-review vocabulary**

- [ ] Given a frontmatter document with `verdict: APPROVE`, when the
  plan-review page renders, then the `verdict` chip uses the `green`
  variant.
- [ ] Given a frontmatter document with `verdict: REVISE`, when the
  plan-review page renders, then the `verdict` chip uses the `amber`
  variant.
- [ ] Given a frontmatter document with `verdict: REQUEST_CHANGES`,
  when a verdict-rendering page paints (PR-review surface; defensive
  coverage on plan-review and work-item-review), then the `verdict`
  chip uses the `red` variant.
- [ ] Given a frontmatter document with `verdict: COMMENT`, when the
  plan-review page renders, then the `verdict` chip uses the `neutral`
  variant.

**Unmapped verdict / result values**

- [ ] Given a frontmatter document with `verdict` set to an unmapped
  value (e.g. an empty string, an arbitrary token like `xyz`, or a
  garbage value), when `VerdictBadge` renders, then the chip falls back
  to the `neutral` variant.
- [ ] Given a frontmatter document with `result` set to an unmapped
  value, when `ResultBadge` renders, then the chip falls back to the
  `neutral` variant.

**Case-insensitive lookup**

- [ ] Given a frontmatter document with `verdict: approve` (lowercase)
  or `verdict: Approve` (mixed case), when `VerdictBadge` renders,
  then the chip uses the `green` variant — i.e. value lookup is
  case-insensitive.
- [ ] Given a frontmatter document with `status: ACCEPTED`, `In_Progress`,
  or `approve-with-changes`, when `StatusBadge` renders, then it
  resolves per the canonical tone mapping — i.e. value lookup is
  case-insensitive and separator-insensitive.

**Work-item-review surface**

- [ ] Given a frontmatter document on a work-item-review page with a
  verdict value from the plan-review vocabulary, when the page renders,
  then the `verdict` chip uses the variant assigned by the canonical
  plan-review mapping. (Supersession path captured in Open Questions:
  if 0066 emits a different vocabulary first, this acceptance criterion
  is replaced by a follow-up.)

## Open Questions

- Will the work-item-review page emit the same verdict vocabulary as
  plan-review (per 0005's `APPROVE` / `REVISE` / `REQUEST_CHANGES` /
  `COMMENT`), or its own set? Work item 0066 flags ongoing churn in
  verdict emission across review skills. The work-item-review AC above
  assumes the plan-review vocabulary; if 0066 lands a different
  vocabulary first, a follow-up will supersede that AC.

## Dependencies

- Blocked by:
  - 0038 (Chip primitive shipped — already provides the six variants
    `neutral` / `indigo` / `green` / `amber` / `red` / `violet` plus
    sizes `sm` / `md`, no extension required).
  - 0005 (plan-review verdict semantics — source of the canonical
    plan-review verdict vocabulary the ACs assert against).
- Blocks: 0084 (Detail-Page Chip Strip Cap — consumes verdict colouring).
- Coordinates with:
  - 0066 (review-skills frontmatter — emitter of verdict values the
    chip helper will receive). **Ordering implication**: if 0066 lands
    after 0081 and emits the plan-review vocabulary, the
    work-item-review AC holds as written; if 0066 lands first and
    emits a different vocabulary, the work-item-review AC is
    superseded by a follow-up.
  - Validation-page result emitter (the `validate-plan` skill at
    `skills/planning/validate-plan/SKILL.md:131-142`; emits `result:`
    and `status:` as top-level frontmatter keys consumed by the
    chip-list renderer).

## Assumptions

- Value lookup is **case-insensitive** across all three vocabularies,
  so any normalisation (or lack thereof) by upstream layers does not
  affect the outcome.
- Validation pages surface `result:` (not `verdict:`) as the
  outcome-bearing frontmatter key; the chip-list renderer dispatches
  `result` to `ResultBadge`.

## Technical Notes

- `statusToChipVariant` lives in `frontend/src/api/status-variant.ts`
  and is the canonical source of truth for the `status` tone map. It
  is renamed to `statusToVariant` and joined by sibling helpers
  `verdictToVariant` and `resultToVariant`; all three consume a shared
  `normaliseValue` helper for case- and separator-insensitive matching.
- The current `FrontmatterChips` component is refactored in place into
  the chip-list renderer: its list-rendering loop becomes a dispatcher
  that routes each frontmatter key to the appropriate chip component
  (status → `StatusBadge`, verdict → `VerdictBadge`,
  result → `ResultBadge`, everything else → `FrontmatterChip`). The
  `FrontmatterChips` name and call sites are preserved so no consumer
  code changes.
- `StatusBadge` / `VerdictBadge` / `ResultBadge` each compose
  `FrontmatterChip` (no shared generic intermediary), hard-coding their
  vocabulary's key and mapping. Per-key chip behaviour (label rendering,
  value formatting) lives in `FrontmatterChip` alone.

## Drafting Notes

- Reconciled vocabulary against actual skill emission: plan-review and
  work-item-review emit `verdict: APPROVE | REVISE | COMMENT`;
  PR-review emits `verdict: APPROVE | REQUEST_CHANGES | COMMENT`;
  validation emits `result: pass | partial | fail` (not `verdict:`).
  Each vocabulary maps to its own variant helper.
- The original wrapper-vs-helper ambiguity in this work item has been
  resolved in favour of a full decomposition of `FrontmatterChips` into
  a chip-list dispatcher plus a per-key component family
  (`FrontmatterChip` + three vocabulary-specific badge wrappers), each
  composing `FrontmatterChip` directly.
- Case-sensitivity has been resolved in favour of case-insensitive
  lookup; the original Open Question on this is now an Assumption plus
  an explicit AC.
- 0038's shipped Chip primitive already includes `red` and `violet`
  variants alongside `green` / `amber` / `indigo` / `neutral` (per the
  0038 plan's Desired End State). No extension to 0038 is required;
  the original review-1 concern about a missing `red` variant was
  based on a partial read of 0038's Summary and is closed.
- Work-item-review surface is treated as assumed-plan-review-vocabulary
  for now; if 0066 settles on a different vocabulary, the
  work-item-review AC is superseded by a follow-up rather than blocking
  0081.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0038 (Chip primitive — dependency), 0084 (downstream
  consumer of verdict colouring), 0005 (plan-review verdict semantics),
  0066 (review-skills verdict emission).
