---
type: adr
id: "ADR-0043"
title: "Pure-numeric typography size-token naming"
date: "2026-06-13T21:42:35+00:00"
author: Toby Clemson
producer: create-adr
status: accepted
supersedes: ["adr:ADR-0036"]
relates_to: ["adr:ADR-0026", "adr:ADR-0030", "adr:ADR-0031", "adr:ADR-0034", "adr:ADR-0036", "adr:ADR-0039", "work-item:0099", "work-item:0075", "work-item:0091"]
tags: [visualiser, frontend, css, design-tokens, typography]
last_updated: "2026-06-13T21:42:35+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# ADR-0043: Pure-numeric typography size-token naming

**Date**: 2026-06-13
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0036 established the typography `font-size` consumption rule and the
`--size-*` scale that backs it. That scale grew by accretion into three
interleaved naming families — t-shirt tiers (`--size-xs`, `--size-sm`,
`--size-md`, …), semantic single-purpose names (`--size-eyebrow`,
`--size-row`, `--size-subtitle`, `--size-prose`), and `-sm`/`-lg` sub-pixel
tweens (`--size-xxs-sm`, `--size-3xs-lg`) — packed at every 0.5px step through
the sub-14px band. The result is a scale where the token name no longer
communicates its value or its ordering: `--size-eyebrow` (11px) sorts below
`--size-xxs` (12px) by value but neither name says so, and a reader cannot
distinguish `--size-3xs-lg` (10.5px) from `--size-3xs` (10px) without consulting
the declaration.

The semantic names compound the problem by misdescribing their consumers.
`--size-subtitle` (13px) sizes toast bodies and sidebar search rows, not
subtitles; `--size-eyebrow` (11px) sizes count indicators; a `WorkItemCard`
rule's own comment admits it picked "the nearest token" rather than one named
for its use. The semantic layer therefore carries a *false* signal — it implies
a single conceptual consumer that does not exist.

ADR-0036 also anticipated this strain: its scale-extension policy required a
design-review justification for each new `-sm`/`-lg` tween precisely because the
awkward naming made off-tier half-steps expensive to admit. The naming friction
was doing load-bearing work as an informal brake on scale proliferation.

This decision remaps the entire `--size-*` scale to a single pure-numeric
scheme, preserving every px value exactly. It fully supersedes ADR-0036,
carrying forward that ADR's font-size consumption rule and px-anchoring stance
unchanged.

## Decision Drivers

- **Name should communicate value and ordering**: three interleaved families at
  0.5px granularity make the name opaque; a reader must consult the declaration
  to recover either the value or the sort order.
- **No false semantics**: the semantic single-purpose names misdescribe their
  actual consumers, so the apparent intent signal is misleading rather than
  helpful.
- **Half-steps must be nameable without a new family**: the sub-14px band needs
  0.5px granularity, and the t-shirt scheme had no honest way to name a
  half-step (`--size-xxs-sm` overloads the `-sm` tier suffix and gives no
  magnitude cue).
- **Continuity with the sibling radius decision**: ADR-0039 already adopted a
  px-encoded measurement ladder for `--radius-*`; typography is the natural next
  family to align.
- **Preserve the consumption rule and the px-anchoring stance**: this change is
  names-only; the rule ADR-0036 established and the still-open px-vs-rem question
  must both survive intact.

## Considered Options

1. **Retain ADR-0036's mixed naming; rename only the offending token(s)
   case-by-case.** Rejected — this is what work item 0094 attempted for the
   single 11px token and found impossible to do cleanly: `--size-xxs-xs`
   overloads the existing `-xs` tier and gives no magnitude cue. Patching one
   name at a time leaves the scale internally inconsistent and adds a fourth
   ad-hoc convention.

2. **Adopt a pure-numeric `px×10` scheme across the entire scale.** Chosen —
   see Decision. Self-names every step by its px value, dissolves the half-step
   naming problem, removes the false semantic signal, and aligns with ADR-0039's
   px-encoded radius ladder.

3. **Keep a semantic alias layer over numeric tokens** (consumers reference
   `--size-eyebrow`, which resolves to `--size-110`). Rejected — the semantic
   names misdescribe their consumers, so preserving them as the consumer-facing
   vocabulary perpetuates the false signal this decision exists to remove.
   Consumers reference the numeric tokens directly.

## Decision

**The naming scheme**: every `--size-*` token is **pure-numeric**, encoding its
px value ×10 with no zero-padding — `--size-<px×10>`. A whole step: 11px →
`--size-110`. A half step: 14.5px → `--size-145`. The smallest token,
`--size-95`, is 9.5px (not 95px). Names are therefore variable-width —
`--size-95` (two digits) sits alongside `--size-110` (three digits) — and sort
numerically by name. Any 0.5px step is nameable under this scheme without
inventing a new naming family.

**Why ×10 and not ×1**: the scale carries half-pixel steps (9.5, 10.5, 11.5,
12.5, 14.5px), which a ×1 px-encoding could not express as an integer suffix.
×10 makes every current and future half-step an integer-suffixed name.

**The font-size consumption rule (carried forward from ADR-0036, unchanged in
substance)**: every `font-size` declaration in current-app CSS (component
modules and global stylesheets under
`skills/visualisation/visualise/frontend/src/`) must resolve to a
`var(--size-*)` token reference. No literal `px`, `rem`, or `em` `font-size`
values are permitted, including those embedded in `font:` shorthand. ADR-0043 is
now the authoritative typography consumption ADR; the `migration.test.ts`
font-size ban is its enforcement harness.

**Scale-extension policy**: an off-ladder font-size value is handled by adding a
new `px×10` step with recorded design-review sign-off, never by tolerance-band
substitution onto a nearby step and never by an unreviewed literal. Note the
shift from ADR-0036: under `px×10` any 0.5px step is now *trivially nameable*, so
the naming friction that previously discouraged off-tier half-steps is gone.
**Ease of naming is not ease of admission** — the design-review sign-off is now
the *sole* guard against scale proliferation, where ADR-0036 had naming
awkwardness as an informal second brake. Reviewers must hold the line
deliberately.

**Value-mutation policy**: because the name encodes the value, a value change is
made by **adding a new step and re-pointing consumers**, never by mutating a
token's value in place — an in-place edit would make the name lie (`--size-110`
resolving to anything but 11px).

**Escape valve**: the `FONT_SIZE_LITERAL_EXCEPTIONS` array beside the vitest
font-size ban is carried forward. It remains **distinct from the `EXCEPTIONS`
ledger**, and the distinction is load-bearing: `EXCEPTIONS` is the
per-occurrence admission ledger — the *expected steady-state* shape through
which every admitted literal flows — whereas `FONT_SIZE_LITERAL_EXCEPTIONS` is a
category-level escape valve that should remain near-empty, where any non-empty
state is an exception requiring a documented removal plan. Merging them would
lose the "admitted literals vs escape valve" policy distinction. Both guards
stay live.

**Scope of supersession**: ADR-0043 **fully** supersedes ADR-0036. Unlike
ADR-0039's *partial* supersession of ADR-0026 (which retained ADR-0026's
non-radius clauses), ADR-0036 is entirely re-expressed here — its consumption
rule, scale-extension policy, escape-valve design, and px-anchoring stance are
all carried forward into this ADR — so a clean full transition of ADR-0036 to
`superseded` is correct.

ADR-0036 itself superseded the *typography portion* of ADR-0026 (§2's Typography
subsection and §3's "em-relative font-sizes" and "Heading font-sizes above
`--size-lg`" rows). ADR-0043 assumes governance of those ADR-0026 typography
clauses going forward, so the live chain **ADR-0026 → ADR-0036 → ADR-0043** stays
discoverable from its live end. This matters because ADR-0036's body is
immutable (ADR-0031) and cannot be edited to point forward at ADR-0043.

ADR-0043 keeps `supersedes: ["adr:ADR-0036"]` **only** — it does **not** carry a
`supersedes` edge to ADR-0026, which retains its non-typography clauses (spacing
tolerance band, border widths) and the radius clause now governed by ADR-0039.
This is a **deliberate divergence from the ADR-0039 model**: ADR-0039 carried a
direct `supersedes: ["adr:ADR-0026"]` edge for the radius row it retired, but
ADR-0043 records the ADR-0026 typography-clause governance via this prose plus a
first-class `adr:ADR-0026` `relates_to` edge instead. The `relates_to` edge is a
live-node link that survives any future pruning of superseded nodes, without
over-claiming a full ADR-0026 supersession that would misrepresent the
still-live ADR-0026 clauses. The divergence is called out here so it is not
silent.

## Consequences

### Positive

- **Value and ordering are recoverable from the name**: `--size-145` is
  unambiguously 14.5px and sorts between `--size-140` and `--size-160` by name.
  No declaration lookup needed.
- **Half-steps are first-class**: any 0.5px value has an honest,
  magnitude-bearing name under `px×10`; no `-sm`/`-lg` tween overloading.
- **The false semantic signal is removed**: numeric names make no claim about a
  conceptual consumer, so a 13px token sizing toast bodies and search rows no
  longer pretends to be a "subtitle".
- **Cross-family alignment with radius**: typography now matches ADR-0039's
  px-encoded `--radius-*` ladder in spirit (px-derived numeric suffix), one
  fewer naming convention to learn.
- **A single ADR governs typography naming and consumption** rather than a rule
  split across a superseded ADR and accreted naming families.

### Negative

- **One-time vocabulary relearning**: abandoning the t-shirt and semantic names
  touches every consumer and changes a vocabulary contributors knew. The
  `global.css` comment block documents the `px×10` convention with a whole-step
  and a half-step example so the scale is self-introducing.
- **Intent-discovery traded for value-recovery**: numeric names answer "what
  value is this?" but not "which token do I want for an eyebrow?". This is
  acceptable because the retired semantic names already *misdescribed* most
  consumers — `--size-subtitle` (13px) sizes toast bodies and sidebar search
  rows; `--size-eyebrow` (11px) sizes count indicators; a `WorkItemCard` rule's
  own comment concedes it picked "the nearest token". The change removes a
  *false* signal rather than a true one (mirroring ADR-0039's `--radius-block`
  analysis, where the most common radius was misnamed after a minority
  consumer).
- **Name↔value coupling forbids in-place value edits** (the value-mutation
  policy above is the cost of self-naming): re-pointing consumers is more work
  than editing one token value, but keeps every name truthful.
- **The naming brake on scale proliferation is gone**: `px×10` makes every
  half-step trivially nameable, so design-review sign-off is now the sole guard
  (see Scale-extension policy).

### Neutral

- **px-anchoring remains an open question (carried forward from ADR-0036)**:
  `--size-*` tokens are intentionally px-anchored, trading user-controllable
  root-font-size scaling for token-value determinism. Browser-level zoom still
  works; users who customise their browser default font-size for accessibility
  lose font-size-only scaling for typography. This rename changes token *names*
  only, not their *unit* — the px-vs-rem trade-off is **carried forward intact,
  not re-decided here**. A future review of the px-vs-rem stance remains open;
  see work-item:0091. That review will likely resolve the unit axis by
  *superseding* ADR-0043 with a further successor ADR (per ADR-0031 an accepted
  ADR is immutable and can only be superseded, so the unit decision will re-state
  the consumption rule in its successor rather than amending this one).
- **Cross-family numbering bases still differ**: `--size-120` (px×10), `--sp-3`
  (ordinal step index), and `--radius-12` (×1 px value) all denote 12px-adjacent
  concepts through three different numeric-suffix conventions. This is an
  accepted, explicitly documented divergence; the `global.css` comment flags all
  three bases at the point of declaration so a reader cannot assume a shared
  numbering basis.

## References

- `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md` —
  superseded by this ADR (fully); the consumption rule and px-anchoring stance
  are carried forward here.
- `meta/decisions/ADR-0039-border-radius-consumption-rule.md` — sibling
  consumption-rule ADR whose px-encoded-ladder, scale-extension, and
  value-mutation argumentation shape this ADR mirrors.
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` — origin
  of the typography clauses (§2 Typography, §3 em-relative / heading rows) that
  ADR-0036 superseded and ADR-0043 now governs; recorded via a `relates_to` edge
  (not a `supersedes` edge — ADR-0026 retains non-typography clauses).
- `meta/decisions/ADR-0030-adr-template.md` — template followed.
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` — supersession
  convention applied (why ADR-0036's body is not edited and an accepted ADR can
  only be superseded).
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — `supersedes` /
  `relates_to` linkage shape.
- `meta/work/0099-remap-typography-size-scale-to-pure-numeric-tokens.md` — work
  item that landed this remap.
- `meta/work/0075-typography-size-scale-consumption.md` — created ADR-0036 and
  introduced the mixed naming this remap fixes.
- `meta/work/0091-typography-rem-vs-px-stance.md` — follow-up review of the
  px-anchored stance (the open unit-axis question).
