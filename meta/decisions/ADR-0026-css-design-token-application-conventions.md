---
adr_id: ADR-0026
date: "2026-05-07T00:00:00+01:00"
author: Toby Clemson
status: accepted
tags: [visualiser, frontend, css, design-tokens]
---

# ADR-0026: CSS Design-Token Application Conventions

**Date**: 2026-05-07
**Status**: Accepted
**Author**: Toby Clemson

## Context

Work item 0033 introduced a full CSS design-token system (`--ac-*`, `--sp-*`,
`--radius-*`, `--size-*`) and migrated all 16 component CSS modules from
hardcoded literals to `var()` references. Three non-obvious decisions arose
during that migration and need to be recorded as conventions so future CSS
work applies them consistently:

1. How to replace hardcoded tint colours (e.g. `#fef2f2` error background)
   with computed equivalents that respond to theme changes.
2. What tolerance to accept when a literal falls between two token values,
   and when to reject the substitution altogether.
3. Which categories of literal are structurally irreducible — i.e. have no
   token equivalent and must be kept verbatim rather than approximated.

## Decision Drivers

- Tint backgrounds expressed as hardcoded hex stop responding to theme
  changes when the base colour changes; they need a computation strategy.
- Forcing every literal into the nearest token risks rounding errors large
  enough to be visually noticeable; rules are needed to bound acceptable drift.
- Some literals (sub-pixel border widths, fixed layout dimensions) have no
  semantic token equivalent; approximating them with the nearest token
  degrades the design.
- The `migration.test.ts` enforcement harness requires a single, unambiguous
  rule set to determine what belongs in the `EXCEPTIONS` array vs what must
  be migrated.

## Considered Options

### Tint computation

1. **Pre-computed hex** — calculate the blended hex at design time and hardcode it.
2. **CSS `color-mix(in srgb, ...)`** — compute the tint at paint time against the current surface token.
3. **Dedicated tint tokens** (`--ac-err-tint-bg`, etc.) — add explicit tokens for every tint variant.

### Tolerance for off-scale spacing and typography

1. **Zero tolerance** — only substitute when the literal matches a token exactly.
2. **Bounded drift** — substitute when the nearest token is within ±2px; mark the rest irreducible.
3. **Always round** — always substitute with the nearest token regardless of drift.

### Irreducible category

1. **Case-by-case** — decide per file at migration time, no standing rules.
2. **Category-based rules** — define fixed categories of literal that are always irreducible.

## Decision

### 1. Tint computation via `color-mix()`

Replace hardcoded tint backgrounds and borders with `color-mix()` computed
against the current surface token. This makes every tint theme-aware without
adding dedicated tint tokens.

**Convention**: always use `in srgb`, always blend against `var(--ac-bg)`:

```css
/* background tint */
background: color-mix(in srgb, var(--ac-err) 8%, var(--ac-bg));
/* border tint */
border-color: color-mix(in srgb, var(--ac-err) 30%, var(--ac-bg));
/* hover-state tint */
background: color-mix(in srgb, var(--ac-err) 18%, var(--ac-bg));
```

**Locked percentages**:

| Role        | Percentage | Typical use                          |
|-------------|------------|--------------------------------------|
| bg tint     | 8%         | Panel/card background wash           |
| hover state | 18%        | Hovered row or interactive bg        |
| border tint | 30%        | Coloured border on tinted background |

These percentages are fixed across all semantic colours (`--ac-err`,
`--ac-warn`, `--ac-ok`, `--ac-accent`). Using non-standard percentages for
a one-off case is not permitted; instead, a new tint role with a named
percentage must be agreed and recorded here.

### 2. Tolerance bands for spacing and typography

**Spacing**: substitute with the nearest `--sp-*` token when the literal falls
within ±2px of a token boundary. Reject (mark irreducible) when drift exceeds
2px. The `--sp-*` scale is 4px steps (4 / 8 / 12 / 16 / 24 / 32 …), so
the ±2px band covers up to a 50% step size for the smallest tokens.

| Example literal   | Nearest token                   | Drift | Decision    |
|-------------------|---------------------------------|-------|-------------|
| `0.6rem` (9.6px)  | `--sp-2` (8px)                  | 1.6px | substitute  |
| `0.7rem` (11.2px) | `--sp-3` (12px)                 | 0.8px | substitute  |
| `0.4rem` (6.4px)  | `--sp-1` (4px) / `--sp-2` (8px) | 2.4px | irreducible |

**Typography**: substitute with the nearest `--size-*` token when pixel drift
is within ±2px. The scale steps are 12 / 14 / 16 / 20 / 22px.

| Example literal    | Nearest token       | Drift | Decision    |
|--------------------|---------------------|-------|-------------|
| `0.8rem` (12.8px)  | `--size-xxs` (12px) | 0.8px | substitute  |
| `0.85rem` (13.6px) | `--size-xs` (14px)  | 0.4px | substitute  |
| `1.6rem` (25.6px)  | `--size-lg` (22px)  | 3.6px | irreducible |

**em-based values** (e.g. `0.88em` for inline code, `1.4em` for line-clamp)
are structurally irreducible regardless of drift — they are relative to the
current font size, not the rem scale.

### 3. Irreducible literal categories

The following categories of literal always land in `EXCEPTIONS` with
`kind: 'irreducible'` and must not be approximated with a token:

| Category                           | Examples                    | Reason                                              |
|------------------------------------|-----------------------------|-----------------------------------------------------|
| Border / outline widths            | `1px`, `2px`                | Below `--sp-1` (4px) floor                          |
| Coloured ring widths               | `1.5px`                     | Sub-pixel; no token equivalent                      |
| Off-scale spacings                 | `0.4rem`, `0.05rem`         | Exceed ±2px drift band                              |
| Off-scale letter-spacing           | `0.06em`, `0.08em`          | Off-scale; standard caps is `0.12em`                |
| em-relative font-sizes             | `0.88em`, `1.4em`           | Relative to font-size, not rem scale                |
| Heading font-sizes above `size-lg` | `1.6rem`, `1.75rem`         | No heading-scale token in 0033                      |
| Fixed layout dimensions            | `220px`, `260px`, `1100px`  | Grid/sidebar dimensions, no token                   |
| Fixed component dimensions         | `14px` dot, `5px` inner dot | Icon pixels, no sp-* equivalent                     |
| Editor palette colours             | `#1e1e1e`, `#d4d4d4`        | Code-block dark colours, no surface token           |
| In-between border radii            | `6px`                       | Between `--radius-sm` (4px) and `--radius-md` (8px) |

### 4. Two-blue collapse

`#2563eb` and `#1d4ed8` (two slightly different Tailwind blues used across
the pre-0033 codebase) both map to `var(--ac-accent)`. The visual delta
between them is within the AC6 5%-pixel-ratio tolerance. This is a
**conscious drift** documented in the 0033 PR description, not an oversight.
Future CSS must use `var(--ac-accent)` rather than either hex value.

## Consequences

### Positive

- `color-mix()` tints remain theme-aware without multiplying the token set.
- The ±2px tolerance band gives clear, auditable guidance for migration
  decisions and EXCEPTIONS entries.
- Irreducible categories eliminate case-by-case judgement calls; the
  `migration.test.ts` harness can enforce them mechanically.

### Negative

- `color-mix()` is a relatively modern CSS function; it requires browsers
  that support it. (All current-evergreen browsers do; this is not a concern
  for the visualiser's developer-tooling audience.)
- The ±2px tolerance band was calibrated against the 4px spacing scale. If
  a finer-grained spacing scale is added later, the band may need revisiting.

### Neutral

- The heading font-size gap (no token above `--size-lg`) is deferred to a
  future type-scale extension; irreducible heading sizes accumulate in
  EXCEPTIONS until then.

## References

- `src/styles/tokens.ts` — canonical token values
- `src/styles/global.css` — `:root` token declarations
- `src/styles/migration.test.ts` — `EXCEPTIONS` enforcement harness
- Work item `meta/work/0033-design-token-system.md` — migration work item
- `meta/plans/2026-05-06-0033-design-token-system.md` — migration plan

---

### Appendix: colour literal reference

Quick lookup for hex values encountered in pre-0033 CSS. These are the
mappings applied in 0033; future edits should follow the same table.

| Hex                  | Token                                                 | Notes                      |
|----------------------|-------------------------------------------------------|----------------------------|
| `#111827`            | `--ac-fg-strong`                                      |                            |
| `#374151`            | `--ac-fg`                                             |                            |
| `#4b5563`            | `--ac-fg-muted`                                       |                            |
| `#6b7280`            | `--ac-fg-muted`                                       | Semantic match preferred   |
| `#9ca3af`            | `--ac-fg-faint`                                       |                            |
| `#d1d5db`            | `--ac-stroke`                                         |                            |
| `#e5e7eb`            | `--ac-stroke-soft`                                    |                            |
| `#f3f4f6`            | `--ac-bg-sunken`                                      |                            |
| `#ffffff`            | `--ac-bg-card`                                        |                            |
| `#2563eb`, `#1d4ed8` | `--ac-accent`                                         | Two-blue collapse (see §4) |
| `#dbeafe`            | `--ac-accent-tint`                                    |                            |
| `#991b1b`            | `--ac-err`                                            |                            |
| `#fef2f2`            | `color-mix(in srgb, var(--ac-err) 8%, var(--ac-bg))`  | bg tint                    |
| `#fee2e2`            | `color-mix(in srgb, var(--ac-err) 18%, var(--ac-bg))` | hover                      |
| `#fecaca`            | `color-mix(in srgb, var(--ac-err) 30%, var(--ac-bg))` | border                     |
| `#1e1e1e`, `#d4d4d4` | irreducible                                           | Editor palette — no token  |
