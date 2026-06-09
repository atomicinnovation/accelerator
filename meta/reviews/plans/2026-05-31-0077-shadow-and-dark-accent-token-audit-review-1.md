---
date: "2026-06-01T01:00:00+00:00"
type: plan-review
producer: review-plan
target: "plan:2026-05-31-0077-shadow-and-dark-accent-token-audit"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, documentation]
review_pass: 3
status: complete
id: "2026-05-31-0077-shadow-and-dark-accent-token-audit-review-1"
title: "2026-05-31-0077-shadow-and-dark-accent-token-audit-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-06-01T01:00:00+00:00"
last_updated_by: Toby Clemson
---

## Plan Review: Shadow and Dark-Accent Token Audit Implementation Plan

**Verdict:** REVISE

The plan is architecturally lightweight, well-sited, and reuses the canonical
`setTheme` / `tokens.ts` primitives correctly — the audit-as-confirmation
framing is sound, and Phase 2 maps one-to-one onto the four work-item ACs.
However, four major concerns surfaced across multiple lenses: the spec's
string-equality assertion is fragile against Chromium custom-property
re-serialisation (Correctness + Test Coverage), the spec exercises only the
`[data-theme="dark"]` mirror and leaves the `@media (prefers-color-scheme:
dark)` MIRROR-B path untested (Test Coverage), AC#2's vacuous-truth treatment
is under-evidenced (Documentation), and AC#1's quote-block layout is
delegated to research without a canonical form (Documentation). The plan
also carries a small internal inconsistency on consumer counts (27 vs 26)
that will surface in the PR description if not reconciled.

### Cross-Cutting Themes

- **String-equality fragility** (flagged by: correctness, test-coverage,
  code-quality) — Asserting `getPropertyValue('--ac-shadow-X').trim() ===
  LIGHT_SHADOW_TOKENS['ac-shadow-X']` is brittle to internal whitespace
  re-serialisation by Chromium and to cosmetic edits in `global.css`. The
  existing sibling specs (`chip-resolved-colours.spec.ts`) deliberately
  read resolved `color`/`fill` properties and use `parseRgb` / `hexToRgb`
  rather than raw string compare. The audit's headline assertion should
  follow the same pattern.

- **Source-of-truth duplication in the EXPECTED map** (flagged by:
  architecture, code-quality, test-coverage) — The four token names appear
  three times in the snippet (light EXPECTED, dark EXPECTED, evaluate
  block). A small `TOKEN_KEYS = [...] as const` tuple removes the
  duplication, tightens typing, and makes the spec data-driven.

- **Computed-value harvesting is under-specified** (flagged by:
  correctness, code-quality, documentation) — Multiple sections rely on
  the implementer inserting a temporary `console.log(actual)` to harvest
  values for the PR description, but the spec's `expect.toEqual` on
  success prints nothing. Either the spec attaches values via
  `test.info().attach(...)`, or the team accepts that `tokens.ts` IS the
  computed value because the spec proves equality and quotes `tokens.ts`
  directly. Pick one explicitly.

- **Naming/discoverability conventions deviate from sibling specs**
  (flagged by: standards, architecture) — `token-values.spec.ts` and the
  `audit-0077` test-name prefix don't match the established
  `*-resolved-*.spec.ts` family naming or the descriptive in-house test
  names. Convergence with the existing pattern (e.g.
  `root-resolved-tokens.spec.ts`) would seed the eventual consolidation
  the plan contemplates rather than create a one-off.

### Tradeoff Analysis

- **Comment discipline vs implementer guidance**: The plan's "Notes for
  the implementer" block under Phase 1 contains 5 bullets of rationale.
  Documentation lens reads this as instruction risk (may get copied into
  source comments against repo norms); Code Quality lens specifically
  recommends adding one or two of these comments. Recommendation: keep
  rationale in the plan, and explicitly mark which bullets (if any) are
  intended as source comments — the `audit-0077` traceability marker is
  the one that may belong in source, the route choice and setTheme
  rationale belong in the plan.

### Findings

#### Critical

_(none)_

#### Major

- 🟡 **Correctness**: String-equality assertion against `tokens.ts` may fail on shadow values due to browser whitespace re-serialisation of custom-property token streams
  **Location**: Phase 1, §1: New Playwright spec — `expect(actual).toEqual(EXPECTED[theme])`
  `getPropertyValue` may re-serialise internal whitespace and `rgba()` argument spacing; `.trim()` only strips leading/trailing whitespace. The repo's own `canonicaliseBrand` helper exists precisely because raw string compare of CSS values is brittle. Normalise both sides (strip internal whitespace + lowercase hex) or assert layer-by-layer numeric components.

- 🟡 **Test Coverage**: Spec exercises only the `[data-theme="dark"]` selector, not the `@media (prefers-color-scheme: dark)` MIRROR-B path
  **Location**: Phase 1: Add Computed-Style Token-Value Spec — example spec at lines 162–192
  `setTheme` sets `dataset.theme`, activating MIRROR-A only; MIRROR-B at `global.css:391–392, 422–423` is never runtime-verified. Static CSS↔CSS parity in `global.test.ts` is byte-level — it cannot catch cascade-precedence regressions on the OS-dark-mode path. Add a third sub-case that clears `dataset.theme` and emulates `prefers-color-scheme: dark` via `page.emulateMedia(...)`.

- 🟡 **Test Coverage**: String-equality assertion against `getPropertyValue` is whitespace- and casing-fragile
  **Location**: Phase 1: Add Computed-Style Token-Value Spec — example spec line 190
  Reinforces the Correctness finding from the test-coverage angle. Sibling specs read resolved CSS properties and channel-compare; the new spec should mirror that pattern to track semantic drift rather than serialisation drift.

- 🟡 **Documentation**: AC#2 vacuous-truth treatment risks under-shooting the work item's quality bar
  **Location**: Phase 2 §2: PR description: divergence justification (AC#2)
  "One sentence is sufficient" leaves no audit trail for which values, which sources, or which test confirms the match. Direct the implementer to write 2–3 sentences naming the comparison method (verbatim quotation per AC#1 + computed-style assertion per AC#3) and cross-referencing the supporting sub-sections.

- 🟡 **Documentation**: AC#1 quote block content is delegated to research without inlining the canonical form
  **Location**: Phase 2 §1: PR description: shadow declaration comparison (AC#1)
  Layout is under-specified ("copy them directly"). Two implementers could produce materially different layouts. Either commit to a layout (e.g. 4-row table with Token / Theme / Current / Prototype columns) or remove the "copy them" instruction and acknowledge the implementer is authoring.

#### Minor

- 🔵 **Code Quality**: `EXPECTED` uses `Record<string, string>`, discarding the typed token keys (Phase 1 §1 — EXPECTED constant typing)
- 🔵 **Code Quality**: Token keys are listed three times across `EXPECTED.light`, `EXPECTED.dark`, and the evaluate block (Phase 1 §1 — spec snippet)
- 🔵 **Code Quality**: Manual verification recommends a temporary `console.log` edit to harvest computed values (Phase 1 §1, Manual Verification bullet 1)
- 🔵 **Code Quality**: New spec uses `setTheme` while the adjacent `tokens.spec.ts` uses `applyTheme` (Phase 1 §1 — helper choice)
- 🔵 **Test Coverage**: Manual capture of computed values is unreliable evidence for AC#3's PR description (Phase 1: Success Criteria — Manual Verification bullets)
- 🔵 **Test Coverage**: Spirit-reading argument leans on `tokens.spec.ts` baselines whose coverage of the four tokens is asserted, not verified, by the new spec (Desired End State / Phase 2 §4)
- 🔵 **Test Coverage**: Choice of `/library` as navigation target is unjustified for a `:root`-only computed read (Phase 1: Notes for the implementer — lines 200–202)
- 🔵 **Correctness**: Internal inconsistency: 27 consumer files vs 26 unique files (Current State Analysis line 65 vs Phase 2 §4 line 294)
- 🔵 **Correctness**: Spec asserts hex strings, but AC#3 requires recording values "normalised to `rgb()` notation" (Phase 2 §3)
- 🔵 **Correctness**: Manual-verification capture step relies on a `console.log` round-trip and mentions `applyTheme` instead of `setTheme` (Phase 1 §1 / Manual Testing Steps)
- 🔵 **Standards**: `audit-0077` test-name prefix has no precedent in the existing spec suite (Phase 1 §1 — test name template line 178)
- 🔵 **Standards**: Filename `token-values.spec.ts` doesn't follow the `*-resolved-*` convention used by sibling assertion-only specs (Phase 1 §1 — line 145)
- 🔵 **Documentation**: Spec-file implementer notes blur the line between plan content and source comments (Phase 1 §1: Notes for the implementer)
- 🔵 **Documentation**: Computed-value harvesting procedure is under-specified for reproducibility (Phase 2 §3)
- 🔵 **Documentation**: Consumer-count mismatch (research ~26, plan 27 then 26) propagates unresolved into the PR (Phase 2 §4)
- 🔵 **Documentation**: Scope note duplicates content already in 'What We're NOT Doing' without cross-linking (Phase 2 §5)
- 🔵 **Documentation**: Spirit-reading paragraph references research §Open Questions §1 but does not name the strict alternative it is rejecting (Phase 2 §4, block-quote)

#### Suggestions

- 🔵 **Architecture**: Audit-prefixed test names imply a future migration path that is not architecturally specified (Phase 1 §Notes for the implementer)
- 🔵 **Architecture**: Coupling spec to a specific route (`/library`) is incidental but unstated as an architectural choice (Phase 1 §Changes Required)
- 🔵 **Architecture**: String-equality assertion against `tokens.ts` is tautological with `prototype-tokens.fixture.test.ts` — document the cascade-override role (Phase 1 §Changes Required)
- 🔵 **Code Quality**: Single combined `page.evaluate` is the right call; document the batching invariant in a brief comment (Phase 1 §1)
- 🔵 **Test Coverage**: `EXPECTED` table is duplicative of the four imported token tables — derive it from a config tuple (Phase 1: example spec lines 162–192)
- 🔵 **Correctness**: Ordering of `goto` and `setTheme` may race the initial app-side theme initialisation — add a `waitForFunction` belt-and-braces or document the order is intentional (Phase 1 §1: spec body)

### Strengths

- ✅ Phase 1 reuses `setTheme(page, theme)` from `lib/expected-colours.ts` and consumes `LIGHT_/DARK_SHADOW_TOKENS` / `LIGHT_/DARK_COLOR_TOKENS` from `tokens.ts`, preserving the single-source-of-truth invariant.
- ✅ Imports the `Theme` discriminator from `expected-colours.ts` rather than redeclaring it locally — matches the convention across resolved-colour specs.
- ✅ Spec covers both light and dark in a single parameterised loop, not dark-only.
- ✅ Uses `expect.toEqual` on the full map so a mismatch reports all four values at once.
- ✅ Explicitly enumerates what is NOT being done (no token-value changes, no new baselines, no brand-layer promotion, no hex-casing normalisation), preventing scope creep.
- ✅ Phase 2 is structured one-to-one against the work item's four ACs, with explicit AC#N labels on each sub-section.
- ✅ Reproducible `rg` command for the consumer enumeration is quoted verbatim so claimed counts can be re-verified.
- ✅ Sources for verbatim quotation are pinned to file:line on both sides — eliminating ambiguity about what gets copied where.
- ✅ AC#4 spirit-reading argument is laid out as a quoted block with named premises, making the rationale auditable.
- ✅ Correctly identifies the audit closes as parity confirmation, so AC#2's clean "values match" branch applies without invoking the divergence-justification clause.
- ✅ Plan acknowledges and dismisses the `frontend/README.md:82` `waitForFunction` warning correctly (because `setTheme` already awaits via `waitForFunction` before the read).

### Recommended Changes

1. **Normalise string comparison in the spec** (addresses: Correctness — string-equality fragility; Test Coverage — whitespace fragility). Define a tiny per-token-type normaliser: lowercase + strip-internal-whitespace for shadows; lowercase for hex accents; or compare via `hexToRgb` / `parseRgb` for accents (matching `chip-resolved-colours.spec.ts`). Document the normaliser in a one-line comment.

2. **Add a MIRROR-B coverage sub-case per theme** (addresses: Test Coverage — MIRROR-B not exercised). Add a third dark-theme test that clears `dataset.theme` and uses `page.emulateMedia({ colorScheme: 'dark' })` to verify the `@media (prefers-color-scheme: dark)` mirror resolves the same four tokens. Two extra `expect` calls close the gap.

3. **Specify AC#2 wording in 2–3 sentences** (addresses: Documentation — AC#2 vacuous-truth). Update Phase 2 §2 to direct the implementer to write a short paragraph naming the comparison method and cross-referencing AC#1's table and AC#3's spec.

4. **Commit to an AC#1 quote-block layout** (addresses: Documentation — AC#1 layout delegated). Update Phase 2 §1 to specify the layout (recommend: 4-row table, columns Token / Theme / Current / Prototype, code-block cells, hex casing preserved per source).

5. **Reconcile the 26-vs-27 consumer count** (addresses: Correctness — internal inconsistency; Documentation — count mismatch). Update the Current State Analysis paragraph to "26" and add one sentence explaining one file is shared between the `--ac-accent` and `--ac-accent-2` consumer sets (or treat the live `rg` output at PR-open time as canonical and remove the inline tally).

6. **Decide computed-value harvest mechanism explicitly** (addresses: Code Quality / Correctness / Documentation — under-specified harvest path). Either commit the spec to emit values via `test.info().attach(...)` (or a permanent `console.log`), or state in Phase 2 §3 that the spec passing IS the proof of equality and the PR description quotes `tokens.ts` literals. Drop the "temporary `console.log`" instruction either way.

7. **Drop `audit-0077` test-name prefix; rename spec file** (addresses: Standards — test naming + filename; Architecture — convergence point). Use descriptive test names (e.g. `root token values resolve correctly (${theme})`) and rename the file to `root-resolved-tokens.spec.ts` (or similar) to match the `*-resolved-*.spec.ts` family. Record the work-item ID in a top-of-file comment if traceability is desired.

8. **Tighten the EXPECTED snippet** (addresses: Code Quality — type narrowness + duplication; Test Coverage — duplicative EXPECTED). Define `TOKEN_KEYS = ['ac-shadow-soft', 'ac-shadow-lift', 'ac-accent', 'ac-accent-2'] as const`, type EXPECTED as `Record<Theme, Record<(typeof TOKEN_KEYS)[number], string>>`, and derive the evaluate-block reads by iterating `TOKEN_KEYS` to eliminate the three-site repetition.

9. **Mark implementer-notes block as plan-only** (addresses: Documentation — notes blur with source comments). Retitle the block "Plan rationale (do not transcribe into source)" or move it outside the spec snippet. If `audit-0077` traceability is dropped per (7), no source comments are needed at all.

10. **Add MIRROR-B vs `@media` reasoning to the spirit-reading paragraph** (addresses: Documentation — strict-alternative not named). Expand the block-quote to name the strict alternative (raise a follow-up that performs no migration and produces no diff) and re-use the plan's own "manufacturing process without producing evidence" phrasing inline.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally lightweight and well-sited: it adds a single Playwright spec under `tests/visual-regression/` that reuses the canonical `setTheme` helper and consumes `tokens.ts` as the source of truth, rather than introducing a parallel constant table. The structural boundaries (test layer depending on src tokens, not the reverse) are respected, source-of-truth duplication is explicitly avoided, and the spec's responsibility (computed-style equality for four named tokens) is narrowly scoped. The only architectural friction is that the new spec sits alongside several existing `*-resolved-colours.spec.ts` files with overlapping responsibility, and the `audit-0077` naming convention is the only signal that this spec will eventually be merged or retired.

**Strengths**:
- Consumes `tokens.ts` via typed imports rather than re-declaring string literals — preserves single-source-of-truth.
- Reuses `setTheme` and the `Theme` discriminator, consistent with `*-resolved-colours.spec.ts` pattern.
- Dependency direction is correct: tests depend on `src/styles/tokens.ts`, not vice versa.
- Explicitly enumerates what is NOT being done, preventing scope creep.
- Phase boundaries cleanly separate code change (Phase 1) from documentation deliverable (Phase 2).

**Findings**:
- 🔵 _suggestion_ — Audit-prefixed test names imply a future migration path that is not architecturally specified. Without a stated convergence criterion (e.g. "merge into `tokens-resolved.spec.ts` when a second audit needs the same primitive"), the file risks becoming a permanent one-off.
- 🔵 _suggestion_ — Coupling spec to `/library` is incidental but unstated. If `/library` is renamed or made slower, the audit spec breaks for unrelated reasons. Either navigate to `/` or document the route choice as incidental.
- 🔵 _suggestion (high-confidence)_ — String-equality assertion against `tokens.ts` is tautological with `prototype-tokens.fixture.test.ts`. The load-bearing job is to detect cascade-level overrides, not to re-assert token values. Add a top-of-file comment naming the cascade-override role.

### Code Quality

**Summary**: Clean, well-scoped, mirrors sibling resolved-colour specs. The proposed snippet has a few maintainability concerns: a loose `Record<string, string>` type, asymmetric helper usage (`setTheme` vs adjacent `tokens.spec.ts`'s `applyTheme`), and a manual-verification step that recommends a temporary `console.log` edit. Minor but worth addressing before implementation.

**Strengths**:
- Reuses `setTheme` rather than duplicating attribute-mutation logic.
- Consumes `tokens.ts` exports directly.
- Imports `Theme` type from `expected-colours.ts`.
- Spec scope tightly bounded (2 tests, 4 properties each, no baselines).
- `audit-0077` prefix signals future retirement/consolidation intent.
- "What We're NOT Doing" enumerates non-changes precisely.

**Findings**:
- 🔵 _minor_ — `EXPECTED: Record<Theme, Record<string, string>>` discards typed token keys. Sibling `chip-resolved-colours.spec.ts` uses narrower `Record<(typeof COLOUR_CODED)[number], string>` pattern. Use `TOKEN_KEYS` tuple + indexed type.
- 🔵 _minor_ — Token keys appear three times in the snippet (light EXPECTED, dark EXPECTED, evaluate block). DRY via tuple-driven derivation.
- 🔵 _minor_ — Manual verification recommends temporary `console.log` edit — maintainability smell. Use `test.info().annotations.push(...)` or accept that `expect.toEqual` IS the evidence.
- 🔵 _minor_ — New spec uses `setTheme` while adjacent `tokens.spec.ts` uses `applyTheme`. Defensible choice (load-bearing `waitForFunction` await), but add a one-line comment citing `frontend/README.md:82` to pre-empt the "why not match the sibling?" question.
- 🔵 _suggestion_ — Single combined `page.evaluate` is the right call; document with a brief comment to lock in the pattern for future maintainers.

### Test Coverage

**Summary**: Spec closes AC#3's literal text (computed-style read under `data-theme="dark"`) by consuming `tokens.ts`, which is mutation-resistant. However, the spec only exercises the `[data-theme="dark"]` selector — MIRROR-B (`@media (prefers-color-scheme: dark)`) is left to existing static parity. The most material risks are (a) string-equality fragility against `getPropertyValue`'s whitespace/casing handling and (b) the AC#4 spirit-reading argument resting on existing `tokens.spec.ts` baselines whose actual coverage of the four tokens is asserted rather than verified by the new spec.

**Strengths**:
- Reuses `setTheme` and `Theme` discriminator.
- Consumes `tokens.ts` directly.
- Covers both light and dark in a single parameterised loop.
- Justifies `/library` over `/kanban` on isolation grounds.

**Findings**:
- 🟡 _major_ — Spec exercises only `[data-theme="dark"]`, not `@media (prefers-color-scheme: dark)` MIRROR-B. Add a third sub-case using `page.emulateMedia({ colorScheme: 'dark' })` with `dataset.theme` cleared.
- 🟡 _major_ — String-equality against `getPropertyValue` is whitespace/casing-fragile across Chromium versions. Normalise both sides or assert layer-by-layer numerics (matching `chip-resolved-colours.spec.ts` pattern).
- 🔵 _minor_ — Manual capture of computed values is unreliable evidence for AC#3. Either snapshot to a JSON fixture or commit a permanent `console.log` (env-gated).
- 🔵 _minor_ — Spirit-reading argument leans on `tokens.spec.ts` baselines whose coverage of the four tokens is asserted, not verified. Either tighten the PR-description wording or add one per-element computed-style read for a `--ac-accent-2` consumer.
- 🔵 _minor_ — `/library` route choice is unjustified for a `:root`-only read. Navigate to `/` or document the choice as incidental.
- 🔵 _suggestion_ — `EXPECTED` is duplicative of the four imported tables. Derive from a config tuple so future token additions are one-line edits.

### Correctness

**Summary**: Logical structure is sound for the AC#1/AC#2/AC#4 happy-path and reuses an empirically-validated theme-swap primitive. Two genuine correctness risks: (1) string-equality between `getPropertyValue('--ac-X').trim()` and the literal values in `tokens.ts` — browsers normalise/re-serialise differently from either source, particularly the multi-layer shadow strings; (2) internal numbering inconsistency (27 vs 26 consumer files) that affects AC#4 evidence. Plan also relies on a "spirit reading" of AC#3's recording requirement only partially satisfied by the spec (captures equality, not rgb()-normalised values).

**Strengths**:
- Reuses `setTheme` rather than re-implementing theme-swap-with-await.
- Consumes `tokens.ts` as the assertion expectation.
- Asserts both themes, not just dark.
- Uses `expect.toEqual` so a mismatch reports all four values at once.
- Correctly identifies the audit as parity confirmation — AC#2 clean branch applies.

**Findings**:
- 🔴 _major (high-confidence)_ — String-equality assertion may fail on shadow values due to Chromium custom-property re-serialisation. The repo's own `canonicaliseBrand` helper exists precisely because raw string compare is brittle. Normalise via internal-whitespace strip + lowercase, or assert per-layer numerics. [Severity raised to major in aggregation due to cross-lens overlap.]
- 🟡 _minor (high-confidence)_ — Internal inconsistency: 27 consumer files (Current State Analysis line 65) vs 26 (Phase 2 §4 line 294). Both exceed AC#4's six-surface threshold, so the verdict is unchanged, but the PR-description evidence will self-contradict.
- 🔵 _minor_ — Spec asserts hex strings, but AC#3 requires recording values "normalised to `rgb()` notation". The normalisation relies on manual hex→rgb at PR-authoring time. Either have the spec also assert via `hexToRgb` or tighten Phase 2 §3 with a paste-ready text template.
- 🔵 _minor_ — Manual-verification capture step is `console.log` round-trip and mentions `applyTheme` instead of `setTheme`. Inconsistent helper guidance + non-reproducible evidence. Fix the naming and use a permanent capture mechanism.
- 🔵 _suggestion (low-confidence)_ — Ordering of `goto` and `setTheme` may race app-side theme init. Add belt-and-braces `waitForFunction` or document order as intentional.

### Standards

**Summary**: Respects most established frontend conventions (consumes `tokens.ts`, reuses `setTheme`, hits a fast route, runs under existing `visual-regression` Playwright project). Two convention deviations: the proposed `audit-0077` test-name prefix is a one-off with no precedent in the existing spec suite, and the spec filename `token-values.spec.ts` doesn't follow the established `*-resolved-*.spec.ts` naming used by sibling assertion-only specs. Placement under the `visual-regression` project is consistent.

**Strengths**:
- Reuses `setTheme(page, theme)` from `lib/expected-colours.ts`.
- Consumes `tokens.ts` exports directly rather than hard-coding hex literals.
- Placement under `tests/visual-regression/` consistent with assertion-only siblings.
- Correctly excludes ADR-0035 hex-casing normalisation, treating uppercase/lowercase as semantic parity.
- Explicitly addresses the `frontend/README.md:82` `waitForFunction` warning.

**Findings**:
- 🔵 _minor (high-confidence)_ — `audit-0077` test-name prefix has no precedent. Existing tests use descriptive names. Drop the prefix; record work-item ID in a top-of-file comment if traceability desired.
- 🔵 _minor_ — Filename `token-values.spec.ts` doesn't follow `*-resolved-*.spec.ts` convention. Rename to `root-resolved-tokens.spec.ts` or similar.

### Documentation

**Summary**: Phase 2 maps cleanly onto the four ACs with sub-section numbering that mirrors AC#1–AC#4, quotes reproducible commands, and points at concrete source-of-truth files. The audit-trail rationale (especially the AC#4 spirit-reading paragraph) is defensible and reproducible. A few documentation gaps remain: the plan never spells out the verbatim quote blocks AC#1 will contain (it points at sources), the AC#2 single-sentence requirement risks under-shooting the work item's quality bar even in the vacuous case, and the implementer-notes block embedded in the spec source verges on over-commenting.

**Strengths**:
- One-to-one structure against the work item's four ACs with explicit AC#N labels.
- Reproducible `rg` command quoted verbatim with exact regex.
- Sources pinned to file:line on both sides.
- AC#4 spirit-reading argument laid out as a quoted block with named premises.
- Cross-reference to the research's Open Questions §1 anchors the spirit-reading.
- Manual Verification checklist (six items) reads as a self-contained AC sweep.

**Findings**:
- 🟡 _major_ — AC#2 "one sentence is sufficient" leaves no audit trail. Direct the implementer to write 2–3 sentences naming the comparison method.
- 🟡 _major_ — AC#1 quote-block layout is delegated to research without inlining the canonical form. Commit to a layout (4-row table) or remove the "copy them directly" instruction.
- 🔵 _minor (high-confidence)_ — Implementer-notes block under Phase 1 may get transcribed as verbose source comments against repo norms. Retitle as "Plan rationale (do not transcribe)" or move outside the spec snippet.
- 🔵 _minor (high-confidence)_ — Computed-value harvesting procedure under-specified. Prescribe one concrete procedure (temporary `console.log` + revert, or `test.info().attach`, or quote `tokens.ts` directly).
- 🔵 _minor_ — Consumer-count mismatch (research ~26, plan 27 then 26) propagates unresolved. Reconcile or defer to live `rg` output.
- 🔵 _minor_ — Phase 2 §5 scope note duplicates "What We're NOT Doing" without cross-linking. Pick one home for negative-scope statement.
- 🔵 _minor (high-confidence)_ — Spirit-reading paragraph references research §Open Questions §1 but does not name the strict alternative being rejected. Expand the block-quote with one sentence on the strict alternative.

## Re-Review (Pass 2) — 2026-06-01

**Verdict:** COMMENT

_Update after Pass-2 review: the MIRROR-B race finding (recorded
below as the sole New Major) was addressed by a follow-up edit to
the plan. The third test now (a) clears `ac-theme` localStorage
via `addInitScript`, (b) waits for React's `useTheme` useEffect to
write `data-theme="dark"` via `waitForFunction`, (c) removes the
attribute, and (d) asserts `hasAttribute('data-theme') === false`
as an invariant guard before reading tokens. The plan rationale
block was extended to document the `use-theme.ts:30–32`
interaction. With that change applied, the MIRROR-B test reliably
exercises the `@media (prefers-color-scheme: dark)` cascade path
and fails loudly if a future regression re-introduces the race._

_Additional follow-up edit applied to the plan addresses the
remaining Pass-2 minor findings: `normaliseShadow` now collapses
runs of whitespace to a single space rather than stripping
entirely (preserves the canonical separator between shadow
components); the inaccurate "robust to future rgb() declarations"
claim was replaced with an explicit hex-literal constraint and a
swap-in-`parseRgb` pointer for the future; `localStorage.removeItem('ac-theme')`
was moved into a `test.beforeEach` so all three tests benefit from
the defensive clear, not just the MIRROR-B test; all three tests
are now wrapped in `test.describe('root resolved tokens', …)` to
match the sibling resolved-* spec convention; `readRootTokens`'s
return type was tightened to the tuple-derived
`Record<ShadowKey/ColorKey, string>` via a `TokenSnapshot` alias;
the AC#1 quote-block template now contains fully-expanded shadow
declarations (no ellipses) so it can be pasted verbatim into the
PR description; Phase 2 §4 now carries an explicit directive to
update every numeric reference if the live `rg` tally diverges
from 26/56; the ALL-CAPS "NOT" in AC#3 was italicised for
register-consistency with the rest of the plan._

## Approval (Pass 3) — 2026-06-01

**Verdict:** APPROVE

All Pass-1 major findings (5) and the Pass-2 New Major (MIRROR-B
race) are resolved. The Pass-2 minor findings are either addressed
in the latest plan edit or explicitly accepted as low-priority
tradeoffs with reviewer concurrence:

- Spirit-reading argument relies on existing `tokens.spec.ts`
  baselines — accepted (reviewer recommended no code change).
- `EXPECTED_SHADOWS` / `EXPECTED_COLORS` retained for readability
  at 4-token scale — accepted (suggestion, not minor).
- "Plan rationale" header wording — accepted (stylistic).
- `normaliseShadow` numeric precision — accepted; byte-equivalence
  with `tokens.ts` is the operational constraint, `global.test.ts`
  catches drift.

The plan is approved for implementation.

All five Pass-1 major findings are resolved or substantively addressed by the edits, but the implementation of one of them — MIRROR-B coverage via a third Playwright test — introduced a new race condition that two lenses independently flagged as a real coverage gap. Plan is acceptable in its current shape, but the user should decide whether to harden the MIRROR-B test before implementation begins.

### Previously Identified Issues

#### Architecture
- 🔵 **Architecture**: Audit-prefixed test names imply a future migration path — **Resolved** (prefix dropped; descriptive test names used).
- 🔵 **Architecture**: `/library` route coupling unstated — **Resolved** (route choice now justified in plan rationale).
- 🔵 **Architecture**: String-equality tautology with `prototype-tokens.fixture.test.ts` — **Resolved** (rationale block now explicitly states the spec's load-bearing job is post-cascade computed-value parity).

#### Code Quality
- 🔵 **Code Quality**: `EXPECTED` uses `Record<string, string>` — **Partially resolved**. EXPECTED_SHADOWS/EXPECTED_COLORS are now narrowly typed via tuple-indexed types, but `readRootTokens` still casts the `page.evaluate` return as `Record<string, string>` rather than the tuple-derived union.
- 🔵 **Code Quality**: Token keys listed three times — **Resolved** (TOKEN_KEYS tuples + helper functions centralise the list).
- 🔵 **Code Quality**: Temporary `console.log` harvest — **Resolved** (Manual Verification no longer requires source mutation; `tokens.ts` quoted directly as evidence).
- 🔵 **Code Quality**: `setTheme` vs `applyTheme` inconsistency — **Resolved** (rationale now explains why `setTheme`'s `waitForFunction` post-condition is load-bearing).
- 🔵 **Code Quality**: Single `page.evaluate` undocumented — **Resolved by structure** (`readRootTokens` helper makes the single-evaluate-call pattern self-evident).

#### Test Coverage
- 🟡 **Test Coverage**: MIRROR-B not exercised — **Partially resolved**. A third test was added, BUT it races React's `useTheme` useEffect (see New Issues below).
- 🟡 **Test Coverage**: String-equality fragility — **Partially resolved**. Normalisers added (`hexToRgb`, `normaliseShadow`), but each carries residual fragility (see Minor below).
- 🔵 **Test Coverage**: Manual capture unreliable — **Resolved**.
- 🔵 **Test Coverage**: Spirit-reading rests on `tokens.spec.ts` baselines — **Still present**. The plan acknowledges this but does not add per-consumer assertions; the existing logical chain (no value change → no diff possible → baselines suffice) remains the argument.
- 🔵 **Test Coverage**: `/library` route choice unjustified — **Resolved**.
- 🔵 **Test Coverage**: EXPECTED duplicative — **Still present** (suggestion only; EXPECTED tables retained for readability at the current scale).

#### Correctness
- 🔴 **Correctness**: Shadow string-equality fragility — **Resolved** (normaliseShadow handles whitespace + case).
- 🟡 **Correctness**: 27 vs 26 inconsistency — **Resolved** (explained inline; Brand.tsx consumes both accents).
- 🔵 **Correctness**: Hex strings vs AC#3's rgb() phrasing — **Resolved** (hexToRgb both sides).
- 🔵 **Correctness**: Manual-verification round-trip + applyTheme/setTheme inconsistency — **Resolved**.
- 🔵 **Correctness**: goto/setTheme race — **Partially resolved**. Tests 1 and 2 are race-free (setTheme's waitForFunction); test 3 introduces a new and worse race (see New Issues).

#### Standards
- 🔵 **Standards**: `audit-0077` prefix has no precedent — **Resolved**.
- 🔵 **Standards**: Filename doesn't follow `*-resolved-*` — **Resolved** (renamed to `root-resolved-tokens.spec.ts`).

#### Documentation
- 🟡 **Documentation**: AC#2 vacuous-truth — **Resolved** (2–3 sentence paragraph template with comparison method and cross-references).
- 🟡 **Documentation**: AC#1 layout delegated — **Resolved** (4-row table template with column structure and file:line sources).
- 🔵 **Documentation**: Implementer notes blur plan vs source comments — **Resolved** ("Plan rationale (do not transcribe into source comments)" header).
- 🔵 **Documentation**: Computed-value harvesting under-specified — **Resolved** (assertion-as-evidence + quote `tokens.ts` literals).
- 🔵 **Documentation**: Consumer-count mismatch — **Resolved** (explained as per-token-sum 27 → 26 unique; live `rg` declared canonical).
- 🔵 **Documentation**: Scope note duplicates "What We're NOT Doing" — **Resolved** (one-sentence cross-link).
- 🔵 **Documentation**: Spirit-reading strict alternative not named — **Resolved** (block-quote expanded to name and reject the strict reading).

### New Issues Introduced

#### Major
- 🟡 **Test Coverage / Correctness** (cross-lens): **MIRROR-B test races `useTheme`'s useEffect — may exercise the wrong cascade path while passing**
  **Location**: Phase 1, §1 — third test (`root token values resolve under prefers-color-scheme: dark (no data-theme)`)

  `RootLayout` mounts `useTheme()` whose `useEffect(() => documentElement.setAttribute('data-theme', theme))` unconditionally writes `data-theme` on mount. With `emulateMedia({ colorScheme: 'dark' })` set, `readInitial` falls through to `prefersDark()` and resolves to `'dark'`, so the effect writes `data-theme="dark"`. The test's `removeAttribute('data-theme')` is not synchronised against React's mount/useEffect lifecycle — if the removal happens before the effect fires, React re-applies the attribute and MIRROR-A (`[data-theme="dark"]`) wins; if after, MIRROR-B (`@media (prefers-color-scheme: dark)` on `:root:not([data-theme="light"])`) wins. Because MIRROR-A and MIRROR-B carry byte-equivalent declarations (asserted by `global.test.ts:125`), the spec passes either way — but the test cannot reliably claim to exercise MIRROR-B, defeating its stated purpose. A regression that breaks only MIRROR-B (typo in the `@media` block, broken `:not()` selector, etc.) could escape CI.

  **Suggested fix options**:
  - (a) Add `await page.addInitScript(() => { try { localStorage.removeItem('ac-theme') } catch {} })` BEFORE `goto`, then assert `expect(await page.evaluate(() => document.documentElement.hasAttribute('data-theme'))).toBe(false)` immediately before `readRootTokens` — fails loudly if the attribute is re-applied.
  - (b) Override `useTheme`'s effect via `page.addInitScript` (e.g. set a `window.__SKIP_USE_THEME__` sentinel the hook honours), or accept the cascade-equivalence interpretation and document it: "the test asserts MIRROR-A ↔ MIRROR-B cascade-value equivalence, not MIRROR-B exclusivity" — keep `tokens.spec.ts:63` as the MIRROR-B-specific visual proof.
  - (c) Wait for React's useEffect to settle (`waitForLoadState('networkidle')`), THEN removeAttribute, THEN add the absence assertion.

#### Minor
- 🔵 **Test Coverage**: `hexToRgb` cannot handle `rgb()`-form inputs — rationale's "robust to future declarations expressed as `rgb(...)`" claim is inaccurate. `hexToRgb('rgb(...)')` produces `'rgb(NaN, NaN, NaN)'` on both sides, vacuously passing. Use `parseRgb` for the computed side, or normalise both sides through a `toRgbTuple` dispatch helper.
- 🔵 **Test Coverage**: `normaliseShadow` strips ALL whitespace, including required separators between shadow components — over-aggressive. Narrow to `replace(/\s+/g, ' ').trim()` to preserve canonical separators while still tolerating internal `rgba(...)` re-spacing.
- 🔵 **Correctness**: localStorage residue from earlier tests could pollute `readInitial` in the third test (if any prior test wrote `ac-theme`), causing `data-theme="light"` to be set on mount and the assertion to compare dark expected values against light cascade. Mitigate with `addInitScript` localStorage.removeItem.
- 🔵 **Correctness**: `normaliseShadow` does not canonicalise numeric forms (`.3` vs `0.3` vs `0.30`). Today's parity tests catch any tokens.ts-side drift, so this is latent fragility only; document the byte-equivalence constraint in a spec header comment.
- 🔵 **Code Quality**: `readRootTokens` return cast as `Record<string, string>` rather than `Record<(typeof SHADOW_KEYS)[number], string>`. One-line tightening preserves the typed-keys benefit through to the assertion path.
- 🔵 **Standards**: Spec omits the `test.describe(...)` wrapper used by every sibling resolved-* spec (e.g. `chip-resolved-colours.spec.ts` uses `test.describe(\`chip-resolved-colours (${theme})\`)`). Wrap each theme's tests in a describe block for reporter consistency.
- 🔵 **Documentation**: AC#1 table template uses ellipses (`0 1px 2px rgba(10,17,27,0.04), …`) with a separate instruction below the table to "not abbreviate the second layer". The template models the wrong shape; replace ellipses with explicit `<full second layer from app.css:36>` placeholders, or expand the second layer inside the template.
- 🔵 **Documentation**: Phase 2 §4 has competing statements about whether the quoted 26/56 tally is canonical. The Overview, Current State Analysis, and AC#2 prose all assert 26/56 as a fact, but §4 says "live `rg` output is canonical, the figures above are illustrative". If the live count drifts ±1–2, the other sections become inconsistent. Add a directive: "if the live tally differs, update every numeric reference in AC#2 and AC#4 to match" — or parameterise those templates with `<N>` placeholders.
- 🔵 **Documentation**: AC#3 uses ALL-CAPS "does NOT apply" where the surrounding plan style uses italics/backticks. Stylistic only.
- 🔵 **Documentation**: "Plan rationale (do not transcribe into source comments)" header reads as a stage direction rather than a noun-phrase title. Consider "Plan-only rationale (not for source comments)" for symmetry with other plan section titles.

### Assessment

The plan is materially stronger after the edits — all five Pass-1 major findings landed, the spec rewrite is cleaner and well-factored, and the PR-description templates make Phase 2 mechanically reproducible. The only new finding worth a hardening pass is the MIRROR-B race in test 3: it's a real coverage gap because the assertion passes silently regardless of which cascade path is exercised, so the plan's claim that all three cascade paths are CI-protected is currently overstated.

Two paths forward:

1. **Harden test 3** by adopting one of the suggested fixes (localStorage clear + absence assertion is the simplest). Recommended if the spec is meant to be load-bearing for MIRROR-B coverage in CI.
2. **Accept the race and downgrade the claim** by reframing test 3 in the plan rationale as a cascade-value-equivalence check rather than a MIRROR-B-exclusivity check. Defensible because `tokens.spec.ts:63` already provides MIRROR-B visual evidence, but it walks back the audit's "permanent CI assertion" promise slightly.

Either resolution is defensible; the choice is the plan author's. Verdict is COMMENT rather than REVISE because (a) only one major finding remains, below the configured threshold of 3, (b) the assertion is sound at the level of "values match" even if the cascade-path attribution is racy, and (c) the remaining minor findings are largely stylistic or latent.
