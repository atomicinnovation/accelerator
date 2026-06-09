---
type: plan-validation
id: "2026-06-02-0090-radius-tokens-consumption-validation"
title: "Validation Report: Radius Tokens Consumption Implementation Plan"
date: "2026-06-04T00:00:00+01:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: pass
target: "plan:2026-06-02-0090-radius-tokens-consumption"
tags: [design, frontend, tokens, radius]
last_updated: "2026-06-04T00:00:00+01:00"
last_updated_by: "Toby Clemson"
schema_version: 1
producer: validate-plan
---

## Validation Report: Radius Tokens Consumption Implementation Plan

**Result: PASS** — all eight phases fully implemented, every automated success
criterion verified, all deviations are the plan's own recorded/authorised ones
(no unrecorded divergence).

### Implementation Status

✓ Phase 1: New ADR-0039 (radius consumption rule) — fully implemented
✓ Phase 2: Px-encoded scale rename + extension — fully implemented
✓ Phase 3: Playwright regression spec (AC2) — fully implemented
✓ Phase 4: Migrate code & pipeline surfaces — fully implemented
✓ Phase 5: Migrate chrome components — fully implemented
✓ Phase 6: Migrate route surfaces — fully implemented
✓ Phase 7: Enforcement gate (AC4) + AC5_FLOOR ratchet — fully implemented
✓ Phase 8: PR description + final verification — fully implemented

Implementation lands as eight discrete, well-described commits
(`wkpxqqtkupoz` ADR-0039 → `uzyvmysn` plan-complete), matching the
migration-first / enforcement-last shape; working copy clean.

### Automated Verification Results

✓ Unit harness: `mise run test:unit:frontend` — **2176 passed / 100 files**, exit 0
✓ E2E: `mise run test:e2e:visualiser` — **413 passed** (incl. radius spec), exit 0
✓ Types: `npm run typecheck` — clean (`tsc --noEmit`, no diagnostics)
✓ AC3 sweep 1 (`module.css` `border-radius: [.0-9]`) — **zero matches** (exit 1)
✓ AC3 sweep 2 (non-`global.css` `border-radius: [.0-9]`) — **zero matches**
✓ AC3 sweep 3 (longhand corners `[.0-9]`) — **zero matches**
✓ Old token names (`var(--radius-sm|md|lg)`) in `src/` — **zero** (rename complete)
✓ ADR-0026 absent from the implementation diff — immutability respected

### Code Review Findings

#### Matches Plan

- **ADR-0039** (`meta/decisions/ADR-0039-border-radius-consumption-rule.md`):
  `status: proposed`, `supersedes: ["adr:ADR-0026"]`. The Considered Options
  section *argues* px-encoding against use-case naming (the `3px`-spans-six-
  surfaces and `--radius-block`-misdescribes-consumers tensions), exactly as
  Phase 1 required. All five bolded Decision clauses present (rule, scale-naming
  policy with the explicit `--sp-N` divergence, scope-of-supersession naming
  both `supersedes` edges into ADR-0026, scale-extension, value-mutation, escape
  valve). Consequences cover name↔value coupling, cross-family inconsistency,
  relearning cost. References cite ADR-0030/0031/0034/0036; no work-item IDs in
  the body. Mirrors ADR-0036's argumentation shape.
- **ADR-0026 untouched** (Authorised Deviation 6): not in the diff; §3 radius
  row remains as historical record; supersession recorded solely on ADR-0039.
- **Px-encoded ladder** declared in lockstep across `global.css:214-223` and
  `tokens.ts:191-201` (`RADIUS_TOKENS`), with the AC7 comment block and the
  `RadiusToken` type updated. Values identical on both sides (parity suite
  guards drift).
- **43-consumer rename** complete — no `--radius-sm/md/lg` reference survives
  anywhere in `src/`; var-resolution test green proves completeness.
- **Enforcement gate** (`migration.test.ts:739-769`): `BORDER_RADIUS_LITERAL_RE`
  with the documented lookbehind (rejects custom-prop names, admits vendor-
  prefixed literals as a recorded limitation) and logical-property out-of-scope
  note. Fixtures (`:771-803`) cover shorthand, all four corners, `0`, `%`,
  `.5rem`, and the negative `-webkit-` literal that proves the limitation.
  `stripComments` hoisted to module scope and reused.
- **AC5 reason-hygiene** (`:805-820`): asserts no surviving `irreducible` reason
  matches `/border-radius|\bradius\b/i`. Confirmed manually: every retained
  `2px/3px/6px/12px` EXCEPTIONS entry is a non-radius use (gaps, padding,
  outline/border widths, box-shadow spreads, scrollbar dimensions) with no
  radius mention in its reason.
- **Regression spec** (`radius-resolved-radii.spec.ts`): data-driven `CASES`,
  pinned 1280×720 viewport, reads `borderTopLeftRadius`, `setup` steps end with
  a `waitFor` on the target, box-dimension assertions on the two `50%` dots.

#### Deviations from Plan (all recorded in the plan itself — not failures)

- **`--radius-1` (1px) added to the ladder** beyond the Phase 2 9-token set.
  Recorded clustering-rewrite drift (Phase 6 manual note): the
  LifecycleClusterView spine migrated `1px` → `var(--radius-1)`. Present in
  both `global.css` and `tokens.ts` in lockstep — consistent, value-guarded.
- **27 literals migrated, not 25.** The two extra: a MarkdownRenderer inline-
  code `3px` pill (added by 0094 after the research) and the `1px` spine. Both
  documented under Phase 4 / Phase 6 manual-verification drift notes.
- **AC5_FLOOR re-synced 427 → 951**, not the plan's predicted ~452. The floor
  was stale-low long before 0090 (a pre-existing harness-discipline lapse);
  0090 itself contributed +27. Recorded in Phase 7 manual verification; the
  bump-protocol comment on the `AC5_FLOOR` line documents it. Both ratchet
  assertions hold.
- **Two `50%` selectors in the spec, not three.** LifecycleClusterView
  `.stage::before` was removed by the clustering rewrite. Recorded in Phase 3
  manual verification.
- **Representative coverage in the spec** (one mountable selector per distinct
  value) rather than per-site enumeration; `8px`, `1px`, `12px` backstopped by
  the value-parity suite + categorical gate. Documented in the spec header and
  Phase 3 (`[~]` deviation).

All deviations trace to the clustering rollout and 0094 landing after the
research snapshot; each is captured in the plan's checkboxes and the spec
header. The six Authorised Deviations (px-encoded naming, `50%`→`--radius-full`,
`0`→`--radius-0`, Vitest gate over ripgrep CI step, EmptyState via AC3, ADR-0026
not edited) are all honoured.

#### Potential Issues

- None blocking. The `--radius-full` (50%) and EmptyState `--radius-12` values
  carry no Playwright assertion by design; both are guarded by the
  `global.test.ts` token-parity suite (value) and the categorical gate
  (literal-ban) — an intentional, documented backstop chain.

### Manual Testing Required

None outstanding. The per-route computed-radius spec discharges the visual
spot-check (asserts exact computed radii unchanged at every covered route —
stronger than eyeball), and gate-failure behaviour is exercised by the passing
positive regex fixtures.

### Recommendations

- ADR-0039 is `proposed`; run `/accelerator:review-adr` to transition it to
  `accepted` before/with merge so the supersession of ADR-0026 §3 is in force.
- Separate follow-up already noted in the plan: ADR-0026's body carries
  in-place typography supersession pointers from 0075 that appear to cross the
  same immutability line (captured in
  `meta/notes/2026-06-03-adr-0026-body-edited-in-place-breaks-immutability.md`).
  Out of scope for 0090; track independently.
