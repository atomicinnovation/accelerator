---
id: "0099"
title: "Remap Typography Size Scale To Pure-Numeric Tokens"
date: "2026-06-02T16:30:00+00:00"
author: Toby Clemson
producer: review-plan
status: draft
kind: task
priority: medium
tags: [visualiser, design-tokens, typography, refactor, tech-debt, adr]
last_updated: "2026-06-02T16:30:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
type: work-item
relates_to: ["plan:2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown"]
supersedes: ["adr:ADR-0036"]
---

# 0099: Remap Typography Size Scale To Pure-Numeric Tokens

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Remap the visualiser's typography `--size-*` scale to a consistent
**pure-numeric** naming scheme (e.g. `--size-110` = 11px, `--size-115` =
11.5px), superseding ADR-0036. The current scale interleaves three
incompatible naming schemes — t-shirt tiers, semantic single-purpose names, and
`-sm`/`-lg` tween suffixes — at every 0.5px step in the sub-14px band, so the
name no longer communicates the value or the ordering.

## Context

Spun out of the 0094 (inline code styling) plan review. 0094 needed an 11px
token for table-cell inline code; the existing 11px token is `--size-eyebrow`,
a "semantic single-purpose" token. Renaming just that one token to fit a ladder
proved impossible to do cleanly (`--size-xxs-xs` overloaded the existing `-xs`
tier suffix and gave no step-magnitude cue), because the underlying scale is
itself inconsistent. Rather than add a fourth ad-hoc name, 0094 was slimmed to
consume `--size-eyebrow` as-is, and the systemic fix was deferred here.

The sub-14px band today (by value):

| px | token | scheme |
|---|---|---|
| 14.5 | `--size-prose` | semantic |
| 14 | `--size-xs` | t-shirt tier |
| 13 | `--size-subtitle` | semantic |
| 12.5 | `--size-row` | semantic |
| 12 | `--size-xxs` | t-shirt tier |
| 11.5 | `--size-xxs-sm` | tween suffix |
| 11 | `--size-eyebrow` | semantic |
| 10.5 | `--size-3xs-lg` | tween suffix |
| 10 | `--size-3xs` | t-shirt tier |
| 9.5 | `--size-4xs` | tier-named, but a half-step |

The upper band (`--size-hero` … `--size-md`/`--size-sm`) is more regular but
should be assessed for inclusion so the whole scale is internally consistent.

A pure-numeric scheme (decision per review discussion) makes design intent
fully recoverable from the name, removes all ambiguity, and extends trivially
to any half-step — at the cost of breaking from the t-shirt convention.

## Scope / Blast Radius

- **~100 consumer references** across the frontend CSS modules. Approximate
  current counts: `--size-xxs` 36, `--size-xs` 24, `--size-3xs-lg` 11,
  `--size-xxs-sm` 8, `--size-eyebrow` 8, `--size-subtitle` 5, `--size-3xs` 3,
  `--size-prose` 3, `--size-lg`/`--size-md`/`--size-h3`/`--size-row` 2 each,
  `--size-body`/`--size-4xs` 1.
- The declarations in `src/styles/global.css` (incl. the naming-convention
  comment).
- The `TYPOGRAPHY_TOKENS` mirror in `src/styles/tokens.ts`.
- Guardrail tests in `src/styles/migration.test.ts` (the
  `var()`-resolves-to-declared-token test auto-tracks the key set; the ADR-0036
  font-size ban and AC5 ratchet must stay green) and `global.test.ts` parity.
- **ADR-0036** (typography font-size consumption rule) must be **superseded** by
  a new ADR documenting the pure-numeric scheme and the migration.

## Requirements

- Adopt a single, consistent pure-numeric `--size-*` naming scheme across the
  whole typography scale (proposed: `--size-<px×10>`, e.g. `--size-110` = 11px,
  `--size-145` = 14.5px), retiring the t-shirt tiers, semantic names, and
  `-sm`/`-lg` suffixes.
- Preserve every computed px value exactly — this is a pure rename, no visual
  change to any surface.
- Update all consumers, the `tokens.ts` mirror, and the convention comment in
  lockstep; rely on the `var()`-resolves-to-declared-token test as the
  completeness gate.
- Supersede ADR-0036 with a new ADR recording the scheme, rationale, and
  migration; keep the prototype drift fixture green (it pins only
  `--code-*`/`--tk-*`/`--atomic-*`, not `--size-*`).
- Sweep up `--size-eyebrow` (consumed by 0094's table-cell rule) as part of the
  remap.

## Acceptance Criteria

- [ ] Every `--size-*` token declared in `global.css` follows the pure-numeric
  scheme; no t-shirt-tier, semantic, or `-sm`/`-lg` size-token names remain.
- [ ] All consumer references across the frontend resolve to the renamed tokens;
  the `var()`-resolves-to-declared-token test passes with no stale references.
- [ ] Every affected surface renders at byte-identical computed font sizes
  before and after (no visual regression; screenshot baselines unchanged).
- [ ] A new ADR supersedes ADR-0036, documents the pure-numeric scheme, and
  records the migration; ADR-0036 is marked superseded per the ADR lifecycle.
- [ ] The full vitest + Playwright suites pass, including the ADR-0036 font-size
  ban (now enforced by the successor ADR), the EXCEPTIONS hygiene check, and the
  AC5 ratchet.

## Dependencies

- **Decoupled from 0094**: 0094 consumes `--size-eyebrow` by its current name
  and needs no rework when this lands — the remap simply renames that reference
  with the rest. This work should NOT block 0094.
- Touches the same `global.css` / `tokens.ts` / `migration.test.ts` surfaces as
  any in-flight typography work; coordinate sequencing to avoid churn conflicts.

## Open Questions

- Encoding for half-steps: `--size-110`/`--size-115` (px × 10) vs another form?
- Scope: sub-14px band only, or the entire scale (hero → 4xs) for full
  consistency?
- Whether any semantic alias layer is retained for readability, or the numeric
  tokens are consumed directly everywhere.

## References

- Originating plan: `meta/plans/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown.md`
  (Key Discoveries; What We're NOT Doing; Migration Notes → "scale-remap initiative")
- Plan review: `meta/reviews/plans/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown-review-1.md`
  (re-review pass 2 surfaced the naming inconsistency)
- Tokens: `skills/visualisation/visualise/frontend/src/styles/global.css:160-187`,
  `skills/visualisation/visualise/frontend/src/styles/tokens.ts:146-175`
- Guardrails: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`,
  `skills/visualisation/visualise/frontend/src/styles/global.test.ts`
- To supersede: `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md`
