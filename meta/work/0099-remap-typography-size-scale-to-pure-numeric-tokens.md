---
id: "0099"
title: "Remap Typography Size Scale To Pure-Numeric Tokens"
date: "2026-06-02T16:30:00+00:00"
author: Toby Clemson
producer: review-plan
status: ready
kind: task
priority: medium
tags: [visualiser, design-tokens, typography, refactor, tech-debt, adr]
last_updated: "2026-06-13T08:27:01+00:00"
last_updated_by: Toby Clemson
schema_version: 1
type: work-item
relates_to: ["plan:2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown", "work-item:0091", "work-item:0075", "work-item:0090", "adr:ADR-0036"]
---

# 0099: Remap Typography Size Scale To Pure-Numeric Tokens

**Kind**: Task
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Remap the visualiser's typography `--size-*` scale to a consistent
**pure-numeric** naming scheme (e.g. `--size-110` = 11px, `--size-115` =
11.5px), superseding ADR-0036 via a new successor ADR. The current scale
interleaves three
incompatible naming schemes — t-shirt tiers, semantic single-purpose names, and
`-sm`/`-lg` tween suffixes (interpolated half-steps between integer tiers) — at
every 0.5px step in the sub-14px band, so the name no longer communicates the
value or the ordering.

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

The upper band (`--size-hero` … `--size-md`/`--size-sm`) is more regular but is
**included** so the whole scale is internally consistent (see Technical Notes
for the full mapping).

A pure-numeric scheme makes design intent fully recoverable from the name,
removes all ambiguity, and extends trivially to any half-step — at the cost of
breaking from the t-shirt convention.

## Requirements

- Adopt a single, consistent pure-numeric `--size-*` naming scheme across the
  **entire** typography scale (`--size-<px×10>`, e.g. `--size-110` = 11px,
  `--size-145` = 14.5px, `--size-680` = 68px), retiring the t-shirt tiers,
  semantic names, and `-sm`/`-lg` suffixes. No band is left on the old names.
  The encoding is literally px×10 with no zero-padding, so names are
  variable-width — `--size-95` (9.5px, two digits) sits alongside `--size-110`
  (11px, three digits).
- Consumers reference the numeric tokens **directly** — no semantic alias layer
  is retained. The retired semantic names (`--size-eyebrow`, `--size-row`,
  `--size-subtitle`, `--size-prose`) disappear as *names*; their px values live
  on under numeric tokens.
- Preserve every computed px value exactly — this is a pure rename, no visual
  change to any surface.
- Update all consumers, the `tokens.ts` mirror, and the convention comment in
  `global.css` in lockstep; rely on the `var()`-resolves-to-declared-token test
  as the completeness gate.
- Supersede ADR-0036 with a new ADR — the **successor ADR**, whose ID is
  allocated at creation time and which is referred to as "the successor ADR"
  throughout this work item — recording the scheme, rationale, and migration.
  Keep the prototype drift fixture green: the design-prototype token-drift guard
  pins only the prototype's `--code-*`/`--tk-*`/`--atomic-*` values, not
  `--size-*`, so this remap leaves it untouched.
- The successor ADR must carry ADR-0036's px-anchoring stance forward as a
  still-open Neutral consequence pointing at 0091 (the px-vs-rem review). This
  remap changes token *names* only, not their unit — so the px-vs-rem trade-off
  must survive the rename rather than being dropped or silently re-decided. 0091
  will later resolve the unit axis, likely by amending the successor ADR (not
  ADR-0036, which is superseded by then).
- Sweep up `--size-eyebrow` (consumed by 0094's table-cell rule) as part of the
  remap.

## Acceptance Criteria

- [ ] **AC1 — No old size-token names declared.** Every `--size-*` token
  declared in `global.css` follows the pure-numeric scheme. *Verify:*
  `rg -nP -- '^\s*--size-(?![0-9]+\s*:)[\w-]+\s*:' src/styles/global.css` (any
  declared `--size-` token — name followed by `:` — that is not purely numeric)
  returns zero matches. Anchoring to declaration lines scopes the check to
  declarations only: `var()` references are covered by AC3 and the convention
  comment by AC2, so the three criteria do not overlap on the same lines.
- [ ] **AC2 — Convention comment rewritten.** The naming-convention comment in
  `global.css` is rewritten to describe the pure-numeric scheme (the current
  comment documents the retired tiers). *Verify:* the comment states the
  `--size-<px×10>` encoding, gives at least one whole-step (e.g. 11px →
  `--size-110`) and one half-step (e.g. 14.5px → `--size-145`) example, and a
  grep of the comment block for the complete retired-name set, anchored to the
  `--size-` prefix —
  `--size-(hero|h1|h2|h3|h4|lg|body|md|sm|prose|xs|subtitle|row|xxs|xxs-sm|eyebrow|3xs-lg|3xs|4xs)\b`
  — returns nothing. Anchoring to the prefix matches only retired token names,
  never incidental prose words (`body`, `row`, `small`, …).
- [ ] **AC3 — Consumers resolve to renamed tokens.** All consumer references
  across the frontend resolve to the renamed tokens; the
  `var()`-resolves-to-declared-token test in `migration.test.ts` passes with no
  stale references. Pass condition: every `var(--size-*)` reference across the
  frontend resolves to a `--size-*` key declared in `global.css`, with zero
  unresolved. (This gate auto-tracks the declared key set and covers consumer
  references; the declarations themselves are covered by AC1.)
- [ ] **AC4 — Byte-identical computed font sizes (by construction).** Computed
  font sizes are unchanged on every surface. This is guaranteed at the
  declaration level, not by per-surface inspection: each renamed token declares
  the exact px value its old name carried (verifiable line-by-line against the
  Technical Notes mapping table), and AC3 guarantees every consumer now resolves
  to one of those renamed tokens — so no consumer's resolved px value can change.
  Screenshot baselines remaining byte-identical is the secondary confirmation,
  not the primary guarantee.
- [ ] **AC5 — Successor ADR created.** The successor ADR (ID allocated at
  creation) supersedes ADR-0036 and documents both the pure-numeric scheme and
  the rename migration.
- [ ] **AC6 — ADR-0036 marked superseded.** ADR-0036 is marked superseded per
  the ADR lifecycle, with the successor ADR owning the `supersedes` edge.
- [ ] **AC7 — px-anchoring carried forward.** The successor ADR contains a
  Neutral consequence restating ADR-0036's px-anchored stance as still-open and
  linking `work-item:0091`, so the px-vs-rem trade-off survives the rename
  rather than being silently re-decided.
- [ ] **AC8 — Suites green, guardrails intact.** The full vitest + Playwright
  suites pass — including the font-size ban (carried into the successor ADR; its
  test reference is updated from ADR-0036 to the successor ADR as part of this
  work), the `EXCEPTIONS` hygiene check (the per-occurrence admitted-literal
  ledger guard in `migration.test.ts`), and the aggregate `var(--*)` coverage
  ratchet in `migration.test.ts`. Each named guardrail remains present and
  enabled (not skipped, `.only`-scoped, or deleted) and executes its assertions,
  so green cannot be reached by disabling a guard.

## Open Questions

- None outstanding. The half-step encoding (`px×10`), scope (entire scale),
  alias-layer (none — numeric direct), and ADR-supersession modelling (the
  successor ADR owns the `supersedes` edge; this work item only `relates_to`
  ADR-0036) are all resolved — see Drafting Notes.

## Dependencies

- **Decoupled from 0094**: 0094 consumes `--size-eyebrow` by its current name
  and needs no rework when this lands — the remap simply renames that reference
  with the rest. This work should NOT block 0094.
- **Required deliverable — the successor ADR**: creating the new ADR that
  supersedes ADR-0036 is a same-PR artefact coupling, not an optional follow-up.
  It is the linchpin connecting this remap to 0091's later unit decision (it
  carries px-anchoring forward and 0091 amends it), so it must land with the
  rename rather than trailing it.
- **No competing in-flight typography work**: as of this writing, no other
  typography work item is open on the `global.css` / `tokens.ts` /
  `migration.test.ts` surfaces beyond 0091 (below). This caveat stands for any
  future item that lands on these surfaces — coordinate sequencing then to avoid
  churn conflicts.
- **Orthogonal to 0091** (px-vs-rem stance review, status `ready`): 0091 changes
  the token *unit*, this changes their *names* — independent axes. Sequence this
  remap first; 0091's later decision resolves the unit axis, likely by *amending*
  the successor ADR rather than forking a second successor (per 0091's own
  recommendation) — a declaration-only change that does not touch the renamed
  consumer sites. **Downstream coupling**: 0091's own Dependencies and AC3 treat
  this remap's successor ADR as the artefact they prefer to chain off once 0099
  has landed, so landing the successor ADR enables 0091's preferred path — the
  enabling relationship is now visible from both sides.

## Assumptions

- All `--size-*` consumers reference tokens by literal name (`var(--size-…)`),
  never via computed or concatenated custom-property names — so a mechanical
  find-replace plus the `var()`-resolves-to-declared-token test is a complete
  completeness gate. (Scope-changing if false: dynamic references would need
  hand-auditing.)
- The upper band (hero → sm) is renamed alongside the sub-14px band; its
  regularity makes this mechanical and visually inert.
- `--size-h4` (26px) is in scope — it was absent from the original blast-radius
  count table but is a declared `--size-*` token.
- Non-size typography tokens (`--lh-*`, `--tracking-caps`) are out of scope;
  only `--size-*` names change.

## Technical Notes

### Full rename mapping

| px | old token | new token |
|---|---|---|
| 68 | `--size-hero` | `--size-680` |
| 48 | `--size-h1` | `--size-480` |
| 36 | `--size-h2` | `--size-360` |
| 28 | `--size-h3` | `--size-280` |
| 26 | `--size-h4` | `--size-260` |
| 22 | `--size-lg` | `--size-220` |
| 20 | `--size-body` | `--size-200` |
| 18 | `--size-md` | `--size-180` |
| 16 | `--size-sm` | `--size-160` |
| 14.5 | `--size-prose` | `--size-145` |
| 14 | `--size-xs` | `--size-140` |
| 13 | `--size-subtitle` | `--size-130` |
| 12.5 | `--size-row` | `--size-125` |
| 12 | `--size-xxs` | `--size-120` |
| 11.5 | `--size-xxs-sm` | `--size-115` |
| 11 | `--size-eyebrow` | `--size-110` |
| 10.5 | `--size-3xs-lg` | `--size-105` |
| 10 | `--size-3xs` | `--size-100` |
| 9.5 | `--size-4xs` | `--size-95` |

All 19 px values are distinct, so the px×10 encoding produces no name
collisions and sorts numerically.

### Surfaces to update (in lockstep)

- The `--size-*` declarations in `src/styles/global.css` (`global.css:173-191`),
  plus the naming-convention comment above them (`global.css:161-172`), rewritten
  to describe the numeric scheme.
- The `TYPOGRAPHY_TOKENS` mirror in `src/styles/tokens.ts` (block starts at
  `tokens.ts:179`).
- All consumer references across the frontend CSS modules.
- Guardrail tests in `src/styles/migration.test.ts` (the
  `var()`-resolves-to-declared-token test auto-tracks the key set; the font-size
  ban and the aggregate `var(--*)` coverage ratchet must stay green, and the
  font-size ban's ADR reference moves from ADR-0036 to the successor ADR) and
  `global.test.ts` parity.

### Blast radius

~100 consumer references across the frontend CSS modules. Approximate counts
(sub-14px band, predating the whole-scale decision): `--size-xxs` 36,
`--size-xs` 24, `--size-3xs-lg` 11, `--size-xxs-sm` 8, `--size-eyebrow` 8,
`--size-subtitle` 5, `--size-3xs` 3, `--size-prose` 3,
`--size-lg`/`--size-md`/`--size-h3`/`--size-row` 2 each,
`--size-body`/`--size-4xs` 1. The upper band now in scope
(`--size-hero`/`-h1`/`-h2`/`-h4`/`-sm`) and any token not enumerated here are
also covered — re-count at implementation time rather than trusting these
figures.

### Migration approach

Mechanical per-token find-replace following the mapping above. The
`var()`-resolves-to-declared-token test is the completeness gate (any stale
reference fails it); screenshot baselines must remain byte-identical since no px
value changes. ADR-0036 is superseded by a new ADR — the successor ADR owns the
`supersedes` edge; this work item only relates to ADR-0036.

## Drafting Notes

- The three originally-open questions were resolved in refinement: half-step
  encoding = `px×10` (`--size-110`/`--size-115`); scope = the entire scale
  (hero → 4xs), not just the sub-14px band; and no semantic alias layer is kept
  (consumers reference numeric tokens directly).
- Kind: intentionally `task`, not `story`, even though the comparable token-rule
  precedents (0075 size-scale consumption, 0090 radius consumption) were both
  stories. Those carried conceptual design decisions; here every decision is
  already resolved (encoding, scope, alias-layer, ADR modelling — see Open
  Questions), leaving a purely mechanical per-token find-replace whose
  completeness and correctness are machine-verified by the
  `var()`-resolves-to-declared-token test and unchanged screenshot baselines.
  The larger surface (~100 references) is breadth, not complexity.
- ADR-supersession is modelled on the **successor ADR**, not this work item: the
  frontmatter `supersedes: ["adr:ADR-0036"]` was relaxed to
  `relates_to: ["adr:ADR-0036", …]`, since an ADR is superseded by another ADR,
  and the new ADR enacts the supersession.
- `--size-h4` (26px) was missing from the original Scope/Blast Radius count
  table; flagged here so the implementer does not miss it under the whole-scale
  decision.
- 0091 coordination: 0091's body recommends the px-vs-rem decision *amend* this
  successor ADR rather than supersede it. The earlier 0099 wording ("0091 will
  supersede this ADR on the unit axis") was softened to match.
- The non-template "Scope / Blast Radius" section was folded into Technical
  Notes to keep the file template-compliant and fill the empty section.
- Corrected reference anchors: `tokens.ts:146-175` (old) pointed at the colour
  tokens; the `TYPOGRAPHY_TOKENS` block actually starts at `tokens.ts:179`.
- Added `work-item:0075` (created ADR-0036 and introduced the mixed naming this
  fixes) and `work-item:0090` (rename + ADR-amend + CI-gate precedent) to
  `relates_to`.

## References

- Originating plan: `meta/plans/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown.md`
  (Key Discoveries; What We're NOT Doing; Migration Notes → "scale-remap initiative")
- Plan review: `meta/reviews/plans/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown-review-1.md`
  (re-review pass 2 surfaced the naming inconsistency)
- Progenitor: `meta/work/0075-typography-size-scale-consumption.md` (created
  ADR-0036, introduced the mixed semantic/numeric naming this remap fixes)
- Rename precedent: `meta/work/0090-radius-tokens-consumption.md` (token rename +
  ADR amendment + CI grep gate)
- Unit-axis sibling: `meta/work/0091-typography-rem-vs-px-stance.md` (px-vs-rem)
- Tokens: `skills/visualisation/visualise/frontend/src/styles/global.css:161-191`,
  `skills/visualisation/visualise/frontend/src/styles/tokens.ts:179`
- Guardrails: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`,
  `skills/visualisation/visualise/frontend/src/styles/global.test.ts`
- To supersede: `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md`
