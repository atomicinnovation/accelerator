---
id: "ADR-0039"
date: "2026-06-04T00:00:00+01:00"
author: Toby Clemson
status: accepted
supersedes: ["adr:ADR-0026"]
tags: [visualiser, frontend, css, design-tokens, radius]
type: adr
title: "ADR-0039: Border-radius consumption rule"
schema_version: 1
last_updated: "2026-06-04T00:00:00+01:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0026", "adr:ADR-0031", "adr:ADR-0030", "adr:ADR-0034", "adr:ADR-0036"]
---

# ADR-0039: Border-radius consumption rule

**Date**: 2026-06-04
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0026 §3 ("Irreducible literal categories") classifies the 6px code-block
`<pre>` radius as permanently irreducible — listed verbatim as "In-between
border radii: `6px`, between `--radius-sm` (4px) and `--radius-md` (8px)" — and
records such literals in the `migration.test.ts` EXCEPTIONS harness with
`kind: 'irreducible'`. A codebase audit (see References) found this was the tip
of a larger gap: **25 literal `border-radius` declarations across 12
component-CSS files**, spanning **seven distinct values** (`0`, `2px`, `3px`,
`6px`, `8px`, `12px`, `50%`). `8px` and `12px` already equal `--radius-md` and
`--radius-lg`; the rest sit in the structural gaps of the original four-step
0033 ladder (`--radius-sm` 4px, `--radius-md` 8px, `--radius-lg` 12px,
`--radius-pill` 999px), which by design had nothing below 4px and nothing
between 4 and 8 — which is exactly why 2px/3px/6px were ruled irreducible.

The result is a radius scale that exists but is only partially consumed, with
the EXCEPTIONS ledger acting as the de-facto rulebook — the same erosion
ADR-0036 corrected for typography. This ADR adopts the equivalent
consume-tokens-everywhere rule for radius.

## Decision Drivers

- **Token-system value-prop**: a defined-but-not-consumed scale contradicts its
  own purpose. Consumption should be the rule, not the majority case.
- **Drift surface**: per-occurrence radius EXCEPTIONS accumulate over time and
  erode the rule, especially in long-lived chrome (Sidebar, FilterPill).
- **Enforceability**: a categorical rule is mechanically checkable; an
  irreducible-literal ledger requires human judgement at every migration.
- **Naming must not invent fiction**: the work item that triggered this ADR
  proposed *use-case* names for the between-step values (`--radius-block` for
  `6px`, an unnamed token for `3px`). The audit showed these names misdescribe
  their consumers — see Considered Options — so a naming policy that avoids
  inventing a use-case where none exists is preferred.

## Considered Options

1. **Retain ADR-0026 §3's irreducible classification for radius; widen the
   scale only.** Rejected: the irreducible ledger admits drift indefinitely and
   the rule has no mechanical stopping point — the same failure ADR-0036
   diagnosed for typography.

2. **Adopt consume-tokens-everywhere with *use-case* names for between-step
   values** (the work item's literal proposal: `--radius-block: 6px`, a
   use-case name for `3px`). Rejected on the evidence:
   - `3px` spans six unrelated surfaces — a clear-button, a scrollbar thumb, a
     facet-option row, and three keyboard-hint chips — with no clean single
     use-case to name it after. AC8's "name it for its use" cannot be honoured
     honestly.
   - `--radius-block` mis-describes its consumers: only 2 of the 7 `6px` sites
     are code blocks; the rest are cards, panels, and a pipeline tile. Naming
     the most common radius in the app after a minority consumer is misleading.
   These tensions are inherent to use-case-naming a value that recurs across
   unrelated surfaces.

3. **Adopt consume-tokens-everywhere with a px-encoded measurement ladder plus
   shape-intent semantic tokens.** Chosen — see Decision. Self-names every
   measurement value by its px magnitude, dissolving the use-case-naming
   problem, and keeps semantic names only where the *intent* (not the
   measurement) is the point.

## Decision

**The rule**: every `border-radius` declaration (the shorthand or any of the
four longhand corner properties — `border-top-left-radius`,
`border-top-right-radius`, `border-bottom-left-radius`,
`border-bottom-right-radius`) in current-app CSS under
`skills/visualisation/visualise/frontend/src/` must resolve to a
`var(--radius-*)` token reference. No literal `px`, `rem`, `em`, percentage, or
bare-`0` radius values are permitted.

**Scale naming policy**: measurement tokens are **px-encoded** —
`--radius-<px>`, where the numeric suffix is the literal px value
(`--radius-2` = 2px, `--radius-6` = 6px, `--radius-12` = 12px). Shape-intent
tokens are **semantic**: `--radius-pill` (999px capsule) and `--radius-full`
(50% circle). This replaces the t-shirt-style names (`--radius-sm/md/lg`)
introduced under ADR-0026 and removes the need for use-case names entirely.
**Note the deliberate divergence from the sibling `--sp-N` spacing scale, whose
numeric suffix is an ordinal step index (`--sp-1` = 4px, `--sp-2` = 8px), not a
px value.** Radius is px-keyed; spacing is step-keyed. The two numeric-suffix
families must not be conflated — a reader cannot assume `--radius-N` and
`--sp-N` share a numbering basis.

**Scope of supersession**: this is a genuine *replacement* of ADR-0026 §3's
"In-between border radii" irreducible classification, for radius only (hence
`supersedes`, following ADR-0036's model, rather than a `relates_to`
supplement). The §3 "Border / outline widths: 1px, 2px" row is border *widths*,
not radius, and is unaffected. **ADR-0026 is now the target of two `supersedes`
edges retiring different sections — ADR-0036 retires its typography rows, and
this ADR retires its radius row.** A reader traversing the linkage graph should
attribute the typography clauses to ADR-0036 and the radius clause to ADR-0039.
The supersession is recorded **here**: ADR-0026 is itself `accepted` and is left
untouched per ADR-0031 (only `proposed` ADRs admit content edits; ADR-0026 is
only partially superseded and so is not transitioned to `superseded` either).
Its §3 radius row physically remains as historical record and is governed by
this ADR going forward — so this prose is the discoverable source of truth,
since ADR-0026 carries no inline pointer to this ADR.

**Scale extension policy**: the ladder enumerates only values actually consumed
by current-app CSS — it is **not** a complete 1px grid. An off-ladder radius
value is handled by adding a new ladder step, with a recorded rationale and
PR/ADR sign-off, **not** by tolerance-band substitution onto a nearby step and
**not** by an unreviewed literal.

**Value-mutation policy**: because a measurement token's name encodes its value,
a value change is made by **adding a new step and re-pointing consumers**, never
by mutating an existing token's value in place — an in-place edit would make the
name lie (`--radius-6` resolving to anything but 6px).

**Escape valve**: none for radius literals. Unlike the per-occurrence EXCEPTIONS
ledger that admits budgeted spacing/width literals, the dedicated radius gate
admits zero radius literals. An off-ladder need is met by extending the ladder
(above), not by exempting a site.

## Consequences

### Positive

- Enforceable as a category-level invariant; the authoritative implementation
  is the `BORDER_RADIUS_LITERAL_RE` gate in `src/styles/migration.test.ts`.
- Design intent is recoverable from the token name: a measurement token's
  suffix *is* its px value, and the two semantic tokens name a shape intent.
- A single ADR governs radius rather than per-occurrence EXCEPTIONS; the
  irreducible ledger is no longer the radius rulebook.

### Negative

- **Name↔value coupling**: a px-encoded name forbids in-place value changes
  (the value-mutation policy above is the cost of self-naming). Re-pointing
  consumers is more work than editing one token value, but keeps every name
  truthful.
- One-time relearning cost: the `--radius-sm/md/lg` → `--radius-4/8/12` rename
  touches every existing consumer and changes a vocabulary contributors knew.
  The in-`global.css` comment block documents the convention so the scale is
  self-introducing.

### Neutral

- **Cross-family inconsistency with `--sp-N`**: radius suffixes are px values,
  spacing suffixes are ordinal step indices. This is an accepted, explicitly
  documented divergence — px-encoding is the right fit for a sparse,
  value-keyed radius ladder, whereas the dense spacing scale reads naturally as
  ordered steps. The `global.css` comment flags the distinction at the point of
  declaration.

## References

- `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
  — §3's "In-between border radii" radius classification, superseded by this
  ADR (left untouched per ADR-0031; this ADR's `supersedes` edge is the record).
- `meta/decisions/ADR-0030-adr-template.md` — template followed.
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` — immutability and
  supersession convention applied (why ADR-0026 is not edited).
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — `supersedes` linkage
  shape; reciprocal edge is derivable and needs no write on ADR-0026.
- `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md` — the
  sibling consumption-rule ADR whose argumentation shape this ADR mirrors.
