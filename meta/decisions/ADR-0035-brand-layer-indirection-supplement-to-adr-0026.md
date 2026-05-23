---
adr_id: ADR-0035
date: "2026-05-23T00:00:00+01:00"
author: Toby Clemson
status: accepted
tags: [visualiser, frontend, css, design-tokens, brand]
---

# ADR-0035: Brand-layer indirection — supplement to ADR-0026

**Date**: 2026-05-23
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0026 (CSS Design-Token Application Conventions) defined the `--ac-*`
semantic layer (story 0033), the `color-mix()` tint rules, the
spacing/typography tolerance bands, the irreducible-literal categories,
and §5's "theme-invariant token families" framework with `--code-*` /
`--tk-*` as the first instance (story 0076). ADR-0026 is `accepted` and
therefore immutable under ADR-0031.

Story 0073 introduces the upstream `--atomic-*` brand-layer palette —
named brand colours, neutrals, aliases, and overlays sourced verbatim
from the design prototype. The brand layer creates two new questions
that ADR-0026 does not yet answer:

1. **Is the brand layer eligible for §5's `:root`-only treatment?**
   ADR-0026 §5 specifies eligibility criteria but enumerates only the
   `--code-*` / `--tk-*` instance. The brand layer is a natural second
   instance and benefits from being listed alongside.
2. **What is the rule for the `--ac-*` semantic layer referencing
   `var(--atomic-X)` brand values?** Story 0073 rewires every `--ac-*`
   declaration whose resolved hex exactly matches an `--atomic-*` value
   to reference the brand layer via `var()`. This indirection pattern
   is new and needs a canonical statement so future contributors apply
   it consistently — including how to handle near-misses, hex
   collisions, and `rgba()`/alpha cases.

A supplement to ADR-0026 records both answers without violating the
immutability of the accepted record.

## Decision Drivers

- ADR-0026 is `accepted` (per ADR-0031); the brand-layer rules must
  live in a new ADR rather than edits to the original.
- Future contributors reading ADR-0026 should be able to discover the
  brand-layer rules via cross-reference, not infer them from code.
- The rule needs to be tight enough to be tooled (the AC2-invariant
  guard test in `global.test.ts` enforces it mechanically), so its
  boundaries — exact-hex match, six-digit scope, alpha exclusion —
  must be unambiguous.
- Hex collisions (`--atomic-ash`/`--atomic-geyser` both at `#d3dbe0`,
  `--atomic-slate-2`/`--atomic-river-bed` both at `#4a545f`) and brand
  aliases (`--atomic-violet` → `--atomic-medium-purple`) need
  deterministic tie-breakers so two reviewers reach the same choice.

## Considered Options

1. **Edit ADR-0026 in place** — Add §5's family enumeration and a new
   §6 to the existing record. Rejected: violates ADR-0031's
   skill-level immutability rule for `accepted` ADRs.
2. **Supersede ADR-0026 with a new ADR** — Republish the full set of
   conventions plus the brand-layer additions. Rejected: ADR-0026 is
   still load-bearing; superseding would mark it `superseded` and bury
   a record that future readers still need.
3. **New supplementary ADR cross-referenced from ADR-0026's neighbours**
   — Add the brand-layer rules in a new ADR explicitly framed as a
   supplement; existing comments referencing the indirection rule
   point at the new ADR rather than at ADR-0026. Precedent:
   ADR-0033 ("Unified base frontmatter schema") uses this pattern to
   supplement ADR-0028 without editing it. Accepted.

## Decision

### 1. Brand layer is a new `:root`-only family under ADR-0026 §5

The `--atomic-*` palette satisfies ADR-0026 §5's three eligibility
criteria:

1. **External authoritative source**: the values are adopted verbatim
   from
   `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`,
   which does not vary by theme.
2. **No a11y differential intended**: brand colours are shared
   identically between light and dark surfaces; per-theme handling
   stays at the `--ac-*` layer.
3. **Drift-detection test ships with it**:
   `src/styles/prototype-tokens.fixture.test.ts` byte-compares every
   `--atomic-*` declaration in `global.css` against the prototype
   source via the `prototype-tokens.json` fixture.

`:root`-only families currently in this list (extending ADR-0026 §5):

- `--code-*` / `--tk-*` (story 0076) — code-block surface and
  syntax-highlight palettes.
- `--atomic-*` (story 0073, this ADR) — brand-layer named colours,
  neutrals, aliases, and overlays. Brand → semantic indirection rule:
  see §2 below.

Future contributors adding a `:root`-only family: if this ADR is
still `proposed`, append a row directly to the list above. Once this
ADR is `accepted` it too becomes immutable, and a further `:root`-only
family must be recorded in a new supplementary ADR (supplementing
either ADR-0026 or this one — both records remain authoritative for
the families they list).

### 2. Brand → semantic indirection rule

Where an `--ac-*` semantic token's resolved value (normalised to
lowercase six-digit hex) **exactly matches** an `--atomic-*` brand
value, the semantic declaration is rewritten as `var(--atomic-X)`.
Near-misses stay as hex literals; the PR introducing such a token
enumerates the residual literals in its description.

**Scope**: the rule applies to six-digit hex values only.

- Alpha-bearing `rgba(...)` declarations on the semantic side are out
  of scope. Overlays and tints stay as semantic-side literals even
  when their RGB triplet matches a brand colour, because the brand
  layer's overlay tokens are declared as `rgba(...)` themselves and
  consolidating them would couple two unrelated opacity decisions.
- `color-mix(...)` outputs are out of scope; if a future need arises,
  this section is the place to revisit.

**Tie-breaker for hex collisions**:

- When both a brand alias and its target match a semantic value,
  prefer the target so the indirection chain stays one hop deep
  (e.g. a future `--ac-brand-purple` matching `#965dd9` would
  reference `--atomic-medium-purple`, not `--atomic-violet` — note the
  existing `--ac-violet` is `#7b5cd9`, a near-miss, and stays as a hex
  literal).
- When two non-alias brand tokens share the same resolved hex (the
  `--atomic-ash`/`--atomic-geyser` collision at `#d3dbe0`, or
  `--atomic-slate-2`/`--atomic-river-bed` at `#4a545f`), the PR
  introducing the rewrite picks the token whose semantic meaning best
  matches the consuming `--ac-*` and records the choice in the PR
  description.

### 3. TS-side stays resolved-hex

`LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS` in `tokens.ts` store
resolved hex values, not `var(--atomic-X)` strings, even for tokens
whose CSS declaration indirects through the brand layer. The CSS↔TS
parity comparator (`canonicaliseBrand` in
`src/styles/testing/canonicaliseBrand.ts`) resolves `var(--atomic-X)`
references via the `BRAND_COLOR_TOKENS` map and compares against the
TS-side resolved hex. This preserves "TS knows the resolved hex of
every semantic token" as an invariant.

A format-guard test (in `global.test.ts`) rejects any
`var(--atomic-*)` or bare `rgb(...)` substring in `LIGHT_COLOR_TOKENS`
or `DARK_COLOR_TOKENS` to lock the invariant in place.

### 4. Operational guidance

- **Adding a new `--ac-*` token**: check if its colour matches an
  existing `--atomic-*` value (normalised lowercase six-digit hex).
  If yes, declare as `var(--atomic-X)`. If no, declare as a hex
  literal AND consider whether a new `--atomic-*` token is warranted;
  if so, add it in the same PR with prototype-source evidence and
  update `prototype-tokens.json`.
- **Adding a new `--atomic-*` token**: must be sourced from
  `prototype-standalone.html` (or a successor prototype) and
  drift-tested via `prototype-tokens.fixture.test.ts`.
- **Per-theme overrides**: remain at the `--ac-*` layer. The brand
  layer is theme-invariant by design.
- **AC2-invariant guard test** (`global.test.ts`): the test
  enumerates every `--ac-*` hex literal in `:root`, `[data-theme="dark"]`,
  and `@media (prefers-color-scheme: dark)`, fails any that matches a
  `BRAND_COLOR_TOKENS` value but is not on the allow-list. The
  allow-list is the auditable surface for intentional exceptions; an
  entry there needs a code-review reason in the PR.

## Consequences

### Positive

- Single source of truth for brand colours; downstream illustrative
  work (BigGlyph, type-tinted iconography under story 0082) consumes
  the brand layer directly without re-declaring values.
- Brand-layer changes propagate through the semantic layer
  automatically; one edit at the brand source moves every matching
  `--ac-*` consumer in lockstep.
- Resolved values are preserved by construction so the rewrite is
  visually transparent — Playwright baselines pass without
  regeneration.
- The AC2-invariant guard test catches regressions automatically; the
  rule does not rely on human discipline alone.

### Negative

- Two layers of CSS variable indirection (consumer → `--ac-*` →
  `--atomic-*`) is one more hop than story 0033's single layer. The
  runtime cost is negligible (browsers already cache resolved
  custom-property values), but readers tracing a colour by eye now
  follow two `var()` references instead of one.
- The "near-miss" judgement (e.g. `--ac-violet` `#7b5cd9` vs
  `--atomic-medium-purple` `#965dd9`) is human-determined; the
  AC2-invariant test only enforces the exact-match rule, not the
  decision to leave a near-miss as a literal vs introducing a new
  brand token.
- Two ADRs now describe the design-token system (ADR-0026 plus this
  supplement). A reader needs to consult both for the full picture.

### Neutral

- Hex collisions inside the brand layer (e.g. `--atomic-ash` /
  `--atomic-geyser`) are preserved as distinct names; the brand layer
  is name-keyed, not hex-keyed.
- `--atomic-overlay-ink`, `--atomic-stroke-light`, and
  `--atomic-shadow-soft` are declared as literal `rgba(...)` matching
  the prototype byte-for-byte. `color-mix(...)` derivations are
  deferred to a future ADR if a consumer need emerges.
- This supplement is itself eligible for further supplements: a future
  ADR can extend the `:root`-only family list in §1 above without
  amending this record.

## References

- `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
  — foundation record this supplement extends
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` —
  immutability rule that motivates the supplement-vs-edit choice
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` —
  prior precedent for the supplement pattern (supplements ADR-0028)
- `meta/work/0073-atomic-brand-layer-palette.md` — introducing work
  item
- `meta/plans/2026-05-23-0073-atomic-brand-layer-palette.md` —
  implementation plan
- `meta/work/0033-design-token-system.md` — original `--ac-*` layer
- `meta/work/0076-code-block-syntax-highlight-palette.md` — first
  `:root`-only family precedent (`--code-*` / `--tk-*`)
- `skills/visualisation/visualise/frontend/src/styles/global.css` —
  brand-layer CSS declarations and rewritten `--ac-*` references
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts` —
  `BRAND_COLOR_TOKENS`, `BRAND_ALIAS_PAIRS`, dual-layer comment
- `skills/visualisation/visualise/frontend/src/styles/testing/canonicaliseBrand.ts`
  — shared CSS↔TS parity normaliser
- `skills/visualisation/visualise/frontend/src/styles/fixtures/prototype-tokens.json`
  — byte-true snapshot of the prototype brand palette
- `skills/visualisation/visualise/frontend/src/styles/prototype-tokens.fixture.test.ts`
  — drift detector
- `skills/visualisation/visualise/frontend/src/styles/global.test.ts`
  — AC2-invariant guard, rewrite-count guard, format-guard,
  alias-target equality, theme-invariance guards
