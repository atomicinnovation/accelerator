---
id: "ADR-0036"
date: "2026-05-23T00:00:00+01:00"
author: Toby Clemson
status: accepted
supersedes: ["adr:ADR-0026"]
tags: [visualiser, frontend, css, design-tokens, typography]
type: adr
title: "ADR-0036: Typography font-size consumption rule"
schema_version: 1
last_updated: "2026-05-23T00:00:00+01:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0026", "adr:ADR-0030", "adr:ADR-0031", "adr:ADR-0034", "codebase-research:2026-05-23-0075-typography-size-scale-consumption", "work-item:0075", "work-item:0091"]
---

# ADR-0036: Typography font-size consumption rule

**Date**: 2026-05-23
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0026 §2 set a ±2px tolerance band that worked well for spacing but
admitted 35 typography outliers as `irreducible` EXCEPTIONS by mid-2026;
ADR-0026 §3 listed em-relative font-sizes and heading sizes above
`--size-lg` as permanently irreducible. The cumulative effect was a
typography rule whose enforcement harness routinely accepted literals
the tokens were nominally supposed to cover, eroding the value-prop of
the scale itself. The triggering codebase audit referenced in
References enumerated 37 literal `font-size` sites across 10 files.

## Decision Drivers

- **Token-system value-prop**: a defined-but-not-consumed scale
  contradicts its own purpose.
- **Drift surface**: per-occurrence EXCEPTIONS accumulate over time and
  erode the rule, especially in long-lived areas (chip headings,
  filter pills, library overviews).
- **Enforceability**: a categorical rule is mechanically checkable;
  tolerance bands require human judgement at every migration.
- **Spacing context**: the ±2px tolerance band for spacing is
  empirically successful and should not be disturbed.

## Considered Options

1. **Retain ADR-0026's tolerance band; widen the scale only.**
   Rejected because the band-plus-scale combination admits drift
   indefinitely — the rule has no mechanical stopping point and the
   ledger of admitted EXCEPTIONS grows monotonically.
2. **Adopt consume-tokens-everywhere; widen the scale to absorb every
   used off-grid value.** Chosen — see Decision.

## Decision

**Typography (`font-size`) consumption rule**: every `font-size`
declaration in current-app CSS (component modules and global
stylesheets under `skills/visualisation/visualise/frontend/src/`) must
resolve to a `var(--size-*)` token reference. No literal `px`, `rem`,
or `em` `font-size` values are permitted, including those embedded in
`font:` shorthand. Off-grid values are handled by extending the scale
rather than by tolerance-band substitution.

**Scope of supersession**: this ADR supersedes the *typography portion*
of ADR-0026 — specifically ADR-0026 §2's Typography subsection and §3's
"em-relative font-sizes" and "Heading font-sizes above `--size-lg`"
rows. The ±2px tolerance band documented in ADR-0026 §2 for **spacing**
(`--sp-*`) continues to apply unchanged.

**Scale extension policy**: new sub-pixel `-sm` / `-lg` tweens between
integer tiers (e.g. `--size-xxs-sm` 11.5 between `--size-xxs` 12 and
`--size-eyebrow` 11) require a design-review justification that the
existing integer tier cannot be used. The scale is not infinitely
extensible — sub-pixel tweens are admitted only when design intent
demands them.

**Escape valve**: a `FONT_SIZE_LITERAL_EXCEPTIONS` array beside the
vitest category-level ban (per-occurrence shape mirroring
`EXCEPTIONS`) may admit specific literal sites in genuinely
exceptional cases (third-party CSS injection, transient migrations).
Each entry must reference an ADR or work item documenting why the
exception is justified, and a target-removal date or condition.
Entries older than 12 weeks without a documented removal blocker
should be migrated or escalated. Routine use is not permitted.

**Why a separate array from `EXCEPTIONS`**: the existing `EXCEPTIONS`
ledger in `migration.test.ts` is a per-occurrence admission ledger for
the AC4 hygiene test (every literal in scope of AC4 must declare an
exemption with a kind, count, and reason). It is the *expected
steady-state* shape — every literal the rule admits flows through it.
`FONT_SIZE_LITERAL_EXCEPTIONS`, by contrast, is a category-level
escape valve that should remain near-empty: any non-empty state is an
exception requiring a documented removal plan. Keeping the two
separate preserves "EXCEPTIONS = admitted literals,
FONT_SIZE_LITERAL_EXCEPTIONS = escape valve" as two distinct signals;
merging them would lose the policy distinction (routine vs
exceptional admissions).

## Consequences

### Positive

- Enforceable by `rg` / vitest as a category-level invariant; the
  authoritative implementation is in `src/styles/migration.test.ts`.
- Design intent is recoverable from token name (the chip rename to
  numeric-ladder names — `--size-3xs-lg`, `--size-xxs-sm` — eliminates
  the misleading prefix).
- Single ADR governs typography rather than per-occurrence EXCEPTIONS;
  the EXCEPTIONS ledger is no longer the typography rulebook.

### Negative

- Legitimate one-off literals (third-party CSS injection, hot fixes)
  require the escape valve. The escape valve has overhead; routine
  use would erode the rule.
- The scale widens with five new tokens plus two renamed sub-pixel
  tweens, increasing the vocabulary contributors must learn. The
  in-`global.css` comment block documents the naming convention so
  the scale is self-introducing.

### Neutral

- `--size-*` tokens are intentionally px-anchored. This trades
  user-controllable root-font-size scaling for token-value
  determinism. Browser-level zoom still works; users who customise
  default font-size in their browser for accessibility lose
  font-size-only scaling for typography. A future review of the
  px-vs-rem stance remains open; see References (work item 0091).

## References

- `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
  — partially superseded by this ADR (typography clauses only).
- `meta/decisions/ADR-0030-adr-template.md` — template followed.
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` —
  supersession convention applied.
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` —
  `supersedes` linkage shape.
- `meta/research/codebase/2026-05-23-0075-typography-size-scale-consumption.md`
  — codebase audit that motivated this ADR.
- `meta/work/0075-typography-size-scale-consumption.md` — work item
  that landed the rule.
- `meta/work/0091-typography-rem-vs-px-stance.md` — follow-up review
  of the px-anchored stance.
