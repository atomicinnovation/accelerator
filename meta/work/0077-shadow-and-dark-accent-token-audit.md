---
work_item_id: "0077"
title: "Shadow and Dark-Accent Token Audit"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: task
status: ready
priority: medium
parent: ""
tags: [design, frontend, tokens, audit]
---

# 0077: Shadow and Dark-Accent Token Audit

**Kind**: Task
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Audit the current app's `--ac-shadow-soft` / `--ac-shadow-lift` values
and the dark-theme `--ac-accent` / `--ac-accent-2` values against the
prototype, then either align them with the prototype's elevation curve
and brighter dark accents, or document the intentional divergence.
The audit and any required token-value migration land in the same PR
— no follow-up work item is created for the migration itself.

## Context

The prototype defines `--ac-shadow-soft` and `--ac-shadow-lift` with
light- and dark-theme variants (`0 1px 2px rgba(10,17,27,0.04),
0 8px 28px rgba(10,17,27,0.06)` and `0 2px 4px rgba(10,17,27,0.06),
0 20px 60px rgba(10,17,27,0.10)`); the current app's
`global.css:172-173,239-240` declares the same token names per theme but
the values were not captured in the inventory, indicating either drift
or undocumented parity.

The prototype's dark theme remaps `--ac-accent` to `#8A90E8` and
`--ac-accent-2` to `#E86A6B` so accents preserve contrast on the
deep-night surface. The current app's dark mirror is documented from
source only and was not visually verified, so we need to confirm whether
the dark accent in the current app actually shifts and, if it does not,
migrate it to the brighter prototype values.

## Requirements

- Quote the current `--ac-shadow-soft` / `--ac-shadow-lift` light and
  dark declarations from `global.css:172-173,239-240` verbatim in the
  PR description comparison, alongside the prototype's declared values.
- Either align the current values with the prototype's elevation curve,
  or document the intentional divergence in the PR description for this
  work item (no separate ADR or token-inventory entry required).
- The four-token consumer enumeration in Acceptance Criterion #4
  doubles as the gate for the dark-accent migration decision: if no
  consumer of `--ac-accent-2` appears in the enumeration, AC#3's
  no-consumer fallback applies and the migration step is skipped for
  that token.
- If the dark accent computed values do not (when normalised to
  `rgb()`) equal `rgb(138, 144, 232)` / `rgb(232, 106, 107)`,
  migrate them to the prototype values within this PR.

## Acceptance Criteria

- [ ] Documented comparison of current vs prototype shadow values for
  both themes, captured in the PR description.
- [ ] Shadow values either match the prototype, or the PR description
  carries a divergence justification that names the reason
  (accessibility, brand intent, oversight, or performance) and either
  cites a prior decision/ADR or records the author's deliberate
  rationale in at least two sentences.
- [ ] Dark `--ac-accent` and `--ac-accent-2` computed values are read
  via `getComputedStyle(document.documentElement)` under
  `data-theme="dark"` and recorded in the PR description. If, when
  normalised to `rgb()` notation, they do not equal
  `rgb(138, 144, 232)` / `rgb(232, 106, 107)` (the prototype's
  `#8A90E8` / `#E86A6B`), the migration is performed in this PR and
  a Playwright dark-theme snapshot of at least one consumer surface
  confirms the new accent renders. If `--ac-accent-2` has no active
  consumer (per the four-token enumeration in AC#4), source
  verification alone satisfies this criterion for that token and the
  absence is recorded in the PR description.
- [ ] Consumer surfaces enumerated by grepping `src/` for
  `--ac-shadow-soft`, `--ac-shadow-lift`, `--ac-accent`, and
  `--ac-accent-2`; the resulting list is recorded in the PR
  description. Before/after Playwright snapshots are captured in
  light and dark for every enumerated surface; any surface whose
  pixel diff exceeds 0.1% has its baseline refreshed and the diff
  recorded in the PR description, otherwise the unchanged baseline
  is recorded as evidence. If the enumerated list exceeds 6
  surfaces, capture no baselines in this PR and raise a follow-up
  work item that enumerates the deferred surfaces, names the themes
  to capture, links back to this audit as parent, and inherits this
  criterion's detection procedure.

## Open Questions

- If shadow values diverge from the prototype, what is the justification
  — accessibility, brand intent, or oversight? Resolved per case during
  the audit and recorded in the PR description.

## Dependencies

- Blocked by: none ([0033 Design Token System](0033-design-token-system.md)
  and [0034 Theme and Font-Mode Toggles](0034-theme-and-font-mode-toggles.md)
  both delivered).
- Consumes: the dark-theme Playwright fixture introduced in 0034 as
  live verification tooling — any future fixture refactor surfaces
  this audit as a downstream consumer.
- May raise: a follow-up visual-regression baseline-refresh work item
  if the consumer enumeration in AC#4 exceeds 6 surfaces (per AC#4's
  follow-up clause).
- Blocks: none directly; downstream design polish depends on this audit
  for accurate elevation expectations. No downstream work items
  currently reference this audit.

## Assumptions

- The prototype's shadow elevation curve is the intended target; any
  current-app divergence is drift unless documented otherwise.
- The prototype's dark `--ac-accent` (`#8A90E8`) and `--ac-accent-2`
  (`#E86A6B`) are the intended dark-theme accent values; the current
  app's dark accent is presumed to lag unless the visual check proves
  otherwise.

## Technical Notes

- Verification uses a Playwright snapshot under `data-theme="dark"` —
  the existing dark-theme fixture from 0034 is the entry point.
- Shadow tokens render on any surface that consumes `--ac-shadow-soft`
  / `--ac-shadow-lift`; expect impact on cards, asides, the topbar lift
  on scroll, and the glyph framing in eyebrows. Surface scope is "any
  surface where shadow or accent rendering changes" rather than a
  pre-enumerated list.
- The dark accents resolve through the `:root[data-theme="dark"]`
  override in `global.css`; verify both the computed value via
  `getComputedStyle(document.documentElement)` and at least one
  consumer surface to catch any more-specific selectors that override
  the dark-theme accent declaration.
- Diffing tip: snapshot the token values into a JSON fixture first
  (computed-style read) before re-running visual regression — that
  decouples token-value drift from rendering drift in failure triage.

## Drafting Notes

- Schema mismatch in original draft (`type:` instead of `kind:`)
  treated as drift from the work-item template, not an intentional
  alternate vocabulary — renamed to match.
- Divergence-justification artefact placed in the PR description per
  the author's choice; not promoted to an ADR because the audit is
  scoped to value alignment, not a design-direction decision.
- "Any surface where shadow or accent rendering changes" left
  deliberately broad rather than enumerated, on the assumption that
  the audit is the discovery step for impacted surfaces and pre-listing
  them would invert the work order.
- Verification standardised on Playwright (with computed-style assist)
  rather than manual capture, to keep the dark-theme verification
  reproducible in CI alongside 0034's existing fixture.
- Blocker references retained as historical context inside Dependencies
  even though both items are done, so the audit's prerequisites stay
  readable.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: [0033 Design Token System](0033-design-token-system.md),
  [0034 Theme and Font-Mode Toggles](0034-theme-and-font-mode-toggles.md)
