---
date: "2026-05-06T21:30:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-06-0033-design-token-system.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, performance, security]
review_pass: 3
status: complete
---

## Plan Review: 0033 Design Token System

**Verdict:** REVISE

The plan is structurally sound and TDD-disciplined: it preserves the existing CSS↔TS parity invariant, extends it cleanly to six token categories, and lands the migration as a CSS-only diff with bounded blast radius. The phase boundaries map neatly onto migration concerns. However, several load-bearing details need to be tightened before implementation: the test harness deliberately commits a failing suite that breaks the trunk, the `EXCEPTIONS` exception model is too coarse to be the canonical PR source, the rgba contrast story is hand-waved, the dark theme drops shadow overrides the inventory mandates, the WCAG AA contrast assertion has no defined failure path, and one Phase 3 mapping is semantically wrong (warning-text → `--ac-err` instead of `--ac-warn`).

### Cross-Cutting Themes

- **Phase 2 commits failing tests to the trunk** (flagged by: architecture, test-coverage) — Phase 2 explicitly ends red and only progressively turns green over phases 3–4, contradicting the plan's own "each phase ends green" invariant. Either collapse 2–4 into a single PR, pre-populate `EXCEPTIONS` with all current literals and progressively *remove* them, or gate the new describes behind `it.skip` until Phase 4 lands.

- **`EXCEPTIONS` matcher is too coarse to be the canonical PR source** (flagged by: architecture, code-quality, test-coverage, standards) — `permitted(file, literal)` whitelists every occurrence of a `(file, literal)` pair, so one legitimate `1.5px` ring exception silently permits any other `1.5px` literal in the same file. Add per-occurrence pinning (count or selector context) so the exception list truly enumerates "this specific call site".

- **WCAG AA failure response is undefined** (flagged by: architecture, test-coverage, standards) — Plan acknowledges `--ac-fg-muted` (`#5F6378`) on `--ac-bg` (`#FBFCFE`) may not reach 4.5:1 yet asserts `>= 4.5` with no specified failure path. Resolve up-front: either pre-compute the ratio (back-of-envelope ~5.5:1), tighten the muted token to pass, or restrict its use to large-text contexts (3:1).

- **rgba / composed contrast hand-waved** (flagged by: code-quality, test-coverage, standards) — `contrast.ts` only accepts hex via `parseHex`. The plan says "compose against the surface" but adds no helper, so strokes (`--ac-stroke*`) and `color-mix()` tints are never contrast-tested. Either extend `contrast.ts` with `parseRgba` + `composeOverSurface`, or carve rgba/composed tokens out of contrast scope explicitly.

- **AC2 / AC6 lean entirely on manual verification** (flagged by: test-coverage, standards) — AC2's font-link wiring is verified only via DevTools network panel; AC6's ΔE / pixel-diff thresholds have no automated measurement. Add an `index.html.test.ts` that asserts the `<link>` tags and weight set, and consider Playwright `toHaveScreenshot()` for AC6 since `@playwright/test` is already a devDep.

- **`color-mix()` introduced without an explicit convention / fallback statement** (flagged by: architecture, code-quality, standards) — First use of `color-mix()` in this codebase, established by precedent in PR prose only. Pin the convention (composition vs token expansion, `in srgb` vs `in oklch`, browser-support assumption) in the plan or a tokens.ts comment.

### Tradeoff Analysis

- **Self-hosting vs Google Fonts**: performance lens is neutral (DX impact small, `preconnect` mitigates handshake cost), security lens flags persistent IP/UA leak to Google plus absence of any CSP. For a local-dev-only tool the bar is low, but the privacy regression vs the current zero-third-party state is real and gets harder to reverse the more downstream stories depend on it. Recommendation: either self-host (~150 KB asset; eliminates CSP/GDPR question and unlocks SRI) or add a tracked "before non-localhost deployment" trigger to the work item.

### Findings

#### Critical

(none)

#### Major

- 🟡 **architecture / test-coverage**: Phase 2 lands deliberately failing tests, breaking the "each phase ends green" invariant
  **Location**: Phase 2 — Success Criteria; Implementation Approach
  Phase 2 explicitly ends with `migration.test.ts` failing for ~16 hex tests, ~16 px-rem tests, and the AC5 aggregate. If phases land as separate PRs/commits, CI is red across the trunk between Phase 2 and end-of-Phase-4, blocking unrelated work and eroding trust in the suite.

- 🟡 **architecture / standards**: Legacy `--color-*` parity coverage is dropped in Phase 1 before consumers migrate in Phase 3
  **Location**: Phase 1, §1 (parity test rewrite) + §2 (`COLOR_TOKENS` retention)
  The Phase 1 rewrite of `global.test.ts` removes the `Object.entries(COLOR_TOKENS)` describe, but the eight `--color-*` declarations and six fallback consumers persist until Phase 3. The single-source-of-truth invariant has a temporal hole. Mark the temporary export `@deprecated` and either keep one parity loop until Phase 3, or collapse the legacy retirement into Phase 1.

- 🟡 **architecture / code-quality / standards**: `color-mix()` chosen for error tints without naming the tradeoff or convention
  **Location**: Phase 4, "Special Conventions"; Phase 3, §3
  First use of `color-mix()` in the codebase, fixed `in srgb`, with no statement of the composition-vs-token-expansion choice or the `in srgb` / `in oklch` colour-space decision. Document the rationale in tokens.ts or the plan, and pin the percentage ladder (8% / 30% / hover-higher) so future modules don't drift recipes.

- 🟡 **code-quality / test-coverage / standards**: `EXCEPTIONS` matcher conflates contexts within a single file
  **Location**: Phase 2, §1 — `permitted()`
  `EXCEPTIONS.some(e => file.endsWith(e.file) && e.literal === literal)` permits all instances of a `(file, literal)` pair. A future `2px` margin slips in for free if any `2px` exception exists in that file. Add a `count` field (assert exact-count equality) or pin to a property/selector context.

- 🟡 **code-quality / test-coverage**: `contrast.ts` rgba composition is hand-waved
  **Location**: Phase 1, §5
  `parseHex` doesn't accept rgba; the plan tells the implementer to "compose against the surface" but adds no helper. Either extend `contrast.ts` with `parseRgba`/`composeOverSurface` and add stroke/tint contrast assertions, or explicitly carve rgba/composed tokens out of contrast scope so the gap is intentional.

- 🟡 **code-quality**: Wave 4a → 4b checkpoint produces no traceable mapping artefact
  **Location**: Phase 4, Wave 4a / 4b
  "Locked-in conventions" between waves are described as "documented in PR" but never persisted in a checkable artefact. Add an end-of-4a deliverable (a `mapping-conventions.md` or a typed constant) listing every `from → to` decision so Wave 4b's 13 modules apply a known set rather than the implementer's recall.

- 🟡 **test-coverage / architecture**: Phase 1 success criterion contradicts the WCAG AA "raise in PR" instruction
  **Location**: Phase 1, §5 (line 482 vs Success Criteria line 495)
  "All six AA assertions pass" and "if it doesn't reach 4.5:1, raise as PR finding" cannot both be true. Pre-resolve: pre-compute the ratio (likely passes ~5.5:1), or tighten the muted token, or restrict to large-text 3:1.

- 🟡 **test-coverage**: AC6's pixel-diff and ΔE thresholds have no automated measurement
  **Location**: Phase 5
  ΔE < 5, ±2px, ±1px, and "5% of viewport area" thresholds are eyeball-evaluated. `@playwright/test` is already a devDep; add `toHaveScreenshot()` for the seven routes × two themes, or downgrade AC6 to qualitative reviewer judgement so the work item and reality match.

- 🟡 **test-coverage**: AC2 has no automated test for `index.html` link tags
  **Location**: Phase 1, §4
  No test asserts that `index.html` contains preconnect to googleapis/gstatic + a stylesheet href containing `family=Sora`, `family=Inter`, `family=Fira+Code` and the right weight set. A future edit removes Fira Code and only manual testing catches it.

- 🟡 **correctness**: `--color-warning-text` mapped to `--ac-err` instead of `--ac-warn`
  **Location**: Phase 3, §2 (line 663) and §3 (line 679)
  The plan maps both warning-text sites to `var(--ac-err)`, while the same callouts' bg/border map to `--ac-warn` family. The result is warning-orange bg + error-red text in one component — semantically incoherent and a likely AC6 violation. Map to `var(--ac-warn)` (verifying contrast against the composed warn-bg).

- 🟡 **standards**: Dark shadow overrides missed — `--ac-shadow-soft` / `--ac-shadow-lift` are theme-variant per inventory
  **Location**: Phase 1, §2/§3 (`SHADOW_TOKENS` single export; ":root shadow declarations marked theme-invariant")
  The inventory's dark `--ac-*` table includes per-theme values for `--ac-shadow-soft` and `--ac-shadow-lift`. The plan's `SHADOW_TOKENS` export and the `:root`-only emission would leave dark mode rendering with light shadow values that vanish on the dark surface. Split shadows into `LIGHT_SHADOW_TOKENS` / `DARK_SHADOW_TOKENS` and emit under both blocks.

- 🟡 **standards**: No `prefers-color-scheme` fallback during the 0033 → 0034 window
  **Location**: Phase 1, §3; "What We're NOT Doing"
  Until 0034 ships the toggle, OS-dark-mode users see the light palette regardless of preference. Either add a `@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { ... } }` mirror, or document the deferral explicitly in "What We're NOT Doing".

#### Minor

- 🔵 **architecture / code-quality**: `EXCEPTIONS` constant and AC4 grep are two parallel enforcement channels that can drift
  **Location**: Phase 2, §1 vs Phase 4 Automated Verification
  Generate the PR's exception list from `EXCEPTIONS` (e.g., a small script) rather than asking the reviewer to mentally reconcile two sources.

- 🔵 **architecture / code-quality / test-coverage**: `readCssVar` non-greedy block regex is fragile against future CSS nesting / `@media` blocks
  **Location**: Phase 1, §1
  Works today (flat blocks) but silently truncates if a future `@supports` / nested selector lands inside `:root` or `[data-theme="dark"]`. Either add a "flat-block-only" invariant comment or use a brace-depth scanner.

- 🔵 **code-quality / test-coverage**: PX_REM_RE / AC4 grep miss `em` literals (e.g. `calc(1.4em * 3)` already in `LifecycleClusterView`)
  **Location**: Phase 2, §1
  Widen the regex or document the carve-out and reason.

- 🔵 **code-quality / test-coverage**: AC5 aggregate threshold `>= 300` is brittle and inflatable by typos
  **Location**: Phase 2, §1
  `var(--ac-bgg)` (typo) increments the count without referencing any defined token. Reframe as per-file invariants ("every non-trivial `.module.css` references at least one colour and one spacing token") and validate token names against the union of declared keys.

- 🔵 **code-quality / test-coverage**: `0px` / `0rem` resets not auto-excluded from PX_REM_RE
  **Location**: Phase 2, §1
  AC4 admits resets as escape-hatch literals; the test forces every occurrence into `EXCEPTIONS`, bloating it and obscuring genuine exceptions. Special-case resets in the regex.

- 🔵 **code-quality**: `var(--token, #fallback)` retirement is documented as convention but not enforced by any test
  **Location**: Phase 4, "Special Conventions"
  Add a one-line `migration.test.ts` assertion that `var\(--[^,)]+,` does not appear in any `.module.css`.

- 🔵 **code-quality / standards**: Cross-phase coupling around temporary `COLOR_TOKENS` retention has no `@deprecated` marker
  **Location**: Phase 1, §2
  Add `/** @deprecated Migrating to LIGHT_COLOR_TOKENS / DARK_COLOR_TOKENS in 0033 Phase 3. Do not add new consumers. */` to the retained export.

- 🔵 **correctness**: AC1 is a parity check, not a grep check — labelling drift in "Desired End State" §6
  **Location**: Desired End State, item 6
  Reword to "AC1 parity tests pass, and AC3 / AC4 / AC5 grep checks all pass".

- 🔵 **correctness**: Prose at line 530 mentions deprecated `{ as: 'raw' }` while the code uses the current `query: '?raw'` form
  **Location**: Phase 2, §1 (prose vs code)
  Update the prose to match.

- 🔵 **correctness / standards**: Inventory-verbatim uppercase hex (#FBFCFE) clashes with existing lowercase hex convention in the codebase
  **Location**: Phase 1, §2 (line 296)
  Either lowercase both sides (parity is byte-equality, either case works as long as TS and CSS agree) or document the mixed-case acceptance.

- 🔵 **correctness**: Dark-theme contrast suite covers only tokens present in `DARK_COLOR_TOKENS`; semantic colours (`--ac-warn`/`--ac-err`/`--ac-ok`/`--ac-violet`) are theme-invariant and need explicit handling
  **Location**: Phase 1, §5
  Add a top-of-file comment in `tokens.ts` listing the theme-invariant semantics so future contrast assertions don't accidentally `DARK_COLOR_TOKENS['ac-warn']` (undefined).

- 🔵 **correctness**: Phase 3's `--color-*` retirement needs CSS+TS deletions in the same commit
  **Location**: Phase 3, §4
  Document the atomicity requirement so a partial commit can't fail parity.

- 🔵 **standards**: Hex casing convention conflicts with existing project files (lowercase elsewhere)
  **Location**: Phase 1, §2 (line 296)
  Same as the correctness finding above; standards lens emphasises the project-wide consistency angle.

- 🔵 **standards**: TypeScript export naming asymmetric (`LIGHT_COLOR_TOKENS` but `TYPOGRAPHY_TOKENS` not `LIGHT_TYPOGRAPHY_TOKENS`)
  **Location**: Phase 1, §2
  Either split shadow per theme (also resolves the missing-dark-shadow finding) or document the prefix policy in a top-of-file comment.

- 🔵 **standards**: Forced-colors mode block not extended for new tokens or for `color-mix()` composed surfaces
  **Location**: Phase 1, §3
  Add a brief Phase 1 note that forced-colors testing was performed for the new token set.

- 🔵 **standards**: Stroke / tinted-background tokens have no contrast assertion
  **Location**: Phase 1, §5
  Either add `--ac-stroke-strong` 3:1 against adjacent surfaces (WCAG 1.4.11) or document strokes as decorative-only.

- 🔵 **performance**: Font-payload estimate (30–80 KB) is on the low side; eight-weight latin payload is closer to 80–200 KB
  **Location**: Performance Considerations
  Cosmetic; widen the estimate or measure once.

- 🔵 **performance**: Inter loaded at 4 weights without an audit of which the migrated CSS actually emits
  **Location**: Phase 1, §4
  Audit; drop unreferenced weights and re-add them as later stories need them.

- 🔵 **security**: Privacy/IP leak to Google for every developer on first font fetch
  **Location**: Phase 1, §4 + "What We're NOT Doing"
  Local-dev today, but the dependency becomes load-bearing for downstream stories. Either self-host (~150 KB, removes CSP/GDPR question) or add a tracked "before non-localhost deployment" trigger to the work item.

#### Suggestions

- 🔵 **architecture**: Five-phase migration as a single PR has a large review surface
  **Location**: Implementation Approach
  Consider splitting Phases 1–3 (token foundation + harness + legacy retirement) from Phase 4 (16-module migration) as separate PRs.

- 🔵 **architecture**: No-fallback `var(--ac-*)` convention isn't codified anywhere persistent
  **Location**: Phase 4, "Special Conventions"
  Add a comment block at the top of `tokens.ts` or `global.css` stating the convention, or capture in a follow-up ADR.

- 🔵 **performance**: Render-blocking stylesheet link is acceptable for dev-only; preload is not warranted now
  **Location**: Phase 1, §4
  No action; revisit if first-paint feel becomes a complaint.

- 🔵 **performance**: Self-hosting was rejected on non-performance grounds (fine)
  **Location**: Drafting Notes / Performance Considerations
  No change needed; revisit if offline-dev fidelity becomes important.

- 🔵 **security**: New third-party origin is a good prompt to introduce a baseline CSP
  **Location**: Phase 1, §4
  No CSP exists today (verified across `index.html`, Vite config, and the Rust visualiser binary). Adding the third-party origin is the natural moment.

- 🔵 **security**: No SRI on the Google Fonts stylesheet link
  **Location**: Phase 1, §4
  Google Fonts is hostile to SRI; document the trade-off explicitly. Self-hosting resolves this.

- 🔵 **security**: GDPR/DPA flag deferred to ops without an owner or trigger condition
  **Location**: Performance Considerations + work item Dependencies
  Add a concrete trigger ("before any non-localhost deployment").

- 🔵 **security**: `migration.test.ts` glob is test-time only; flag if pattern moves to runtime
  **Location**: Phase 2, §1
  One-line comment that the glob is test-time only.

- 🔵 **standards**: No project lint config detected; conventions enforced only by `migration.test.ts` + human review
  **Location**: Cross-cutting
  Out-of-scope follow-up: stylelint with `declaration-property-value-disallowed-list` for hex literals.

- 🔵 **test-coverage**: Top-level `await import()` for `wiki-links.global.css` is unnecessary; static import works
  **Location**: Phase 2, §1
  Replace `(await import(...)).default` with `import wikiLinks from '...'` or include in the eager glob.

- 🔵 **test-coverage**: E2E suite is referenced as a regression gate but no specific tests are named
  **Location**: Testing Strategy — Integration tests
  Confirm specific Playwright tests exist or remove the claim.

- 🔵 **test-coverage**: No test enforces semantic correctness of hex→token mappings
  **Location**: Phase 3 / Phase 4
  Best addressed by Playwright visual regression (see AC6 finding).

### Strengths

- ✅ TDD invariant ("never edit a CSS file without a failing assertion that justifies the edit") gives every migration step a verifiable purpose and a clean review trail.
- ✅ Preserves and extends the existing CSS↔TS parity invariant via `readCssVar(name, scope)` rather than reinventing it.
- ✅ Single source of truth: `EXCEPTIONS` typed constant is the canonical source for both per-file test enforcement and the PR's irreducible-literal list.
- ✅ Bounded blast radius: CSS-only diff confined to `src/styles/`, 16 `.module.css` files, and `index.html` — no TSX/routing boundaries touched.
- ✅ Splitting colour tokens into `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS` cleanly aligns the TS surface with the CSS scoping model.
- ✅ Phase 3 explicitly retires the legacy `--color-*` system, closing the parallel-token-system tech-debt door at the same time.
- ✅ Per-file granularity in `migration.test.ts` (one `it()` per CSS module) gives precise failure attribution.
- ✅ AC3/AC4/AC5 grep predicates are mirrored as vitest assertions so they fail at unit-test time, not only at PR-grep time.
- ✅ WCAG AA contrast assertions explicitly extended to dark, preventing the dark palette from shipping unverified.
- ✅ `preconnect` correctly carries `crossorigin` on `gstatic` and omits it on `googleapis` — both per spec.
- ✅ `display=swap` set on the Google Fonts request — eliminates FOIT and is the right call for a dev-only target.
- ✅ Plan correctly identifies and corrects the work item's misattribution of the parity loop to `global.test.ts`.
- ✅ Plan acknowledges the indigo-vs-blue collapse as a documented AC6 drift rather than silently hiding it.

### Recommended Changes

Ordered by impact:

1. **Resolve the Phase 2 "deliberately failing tests on trunk" problem** (addresses: Phase 2 lands deliberately failing tests; Plan invariant contradiction)
   Pick one of: (a) collapse Phases 2–4 into a single PR with a green harness at merge, (b) pre-populate `EXCEPTIONS` with every current literal at Phase 2 commit so it lands green, then progressively *delete* exceptions in Phases 3–4 as files migrate, or (c) gate the new describes behind `it.skip` until Phase 4 closes. Update Phase 2's Success Criteria to match.

2. **Tighten the `EXCEPTIONS` matcher to per-occurrence pinning** (addresses: matcher conflates contexts; AC4 grep-vs-test drift)
   Change `EXCEPTIONS` shape to `{ file, literal, count, reason }` (or `{ file, line, literal, reason }`); assert `permittedMatches.length === sum(expectedCount)` so adding one exception doesn't whitelist further occurrences.

3. **Pre-resolve the WCAG AA muted-foreground question and remove the contradiction** (addresses: contrast assertion contradicts Phase 1 success criterion)
   Compute `contrastRatio('#5F6378', '#FBFCFE')` now (back-of-envelope ~5.5:1, likely passes). If it fails, tighten the muted token or restrict to large-text 3:1 — don't punt to PR-time.

4. **Fix the warning-text → `--ac-err` mismap** (addresses: warning-text mapped to error family)
   Change Phase 3 §2 line 663 and §3 line 679 from `var(--ac-err)` to `var(--ac-warn)` (or a derived darker warn for legibility, documented as an AC6 drift). Verify `contrastRatio` against the composed warn-bg.

5. **Add per-theme dark shadow tokens** (addresses: dark shadow overrides missed)
   Move `--ac-shadow-soft` / `--ac-shadow-lift` out of theme-invariant `SHADOW_TOKENS`; emit per-theme values under both `:root` and `[data-theme="dark"]`. Add parity test coverage in both scopes.

6. **Decide and codify the rgba/composed contrast policy** (addresses: rgba hand-waved)
   Either extend `contrast.ts` with `parseRgba` + `composeOverSurface` and add stroke / `color-mix` tint contrast assertions, or carve rgba/composed tokens out of contrast scope explicitly (with rationale tied to WCAG 1.4.11 decorative-strokes). Pick one; don't leave it to implementation.

7. **Restore continuous parity for legacy `--color-*` during phases 1–3** (addresses: legacy parity hole; temporary export coupling)
   Retain the existing `Object.entries(COLOR_TOKENS)` describe in `global.test.ts` until Phase 3 deletes both the CSS declarations and the TS export in the same commit. Add `/** @deprecated */` to the retained export. Document atomicity in Phase 3 §4.

8. **Add an automated test for AC2 (`index.html` font links)** (addresses: AC2 manual-only)
   New `index.html.test.ts` that imports `index.html?raw` and asserts: preconnect to googleapis + gstatic; stylesheet href contains `family=Sora`, `family=Inter`, `family=Fira+Code`; weight set matches the prototype's usage.

9. **Decide AC6 automation now** (addresses: AC6 thresholds have no measurement)
   Either add Playwright `toHaveScreenshot()` for the seven routes × two themes (`@playwright/test` is already a devDep), or downgrade AC6 to qualitative reviewer judgement. The current state (quantitative thresholds, manual measurement) is contradictory.

10. **Pin the `color-mix()` convention** (addresses: color-mix() introduced without convention; first-use-by-precedent)
    Add a section to the plan (or a comment in `tokens.ts`) stating: composition over token-expansion choice, `in srgb` colour space (or `in oklch`), the percentage ladder (8% / 30% / hover-higher), and the browser-support assumption. Optionally add a `migration.test.ts` regex check that any `color-mix` against `--ac-err`/`--ac-warn` matches the documented ladder.

11. **Add `prefers-color-scheme` fallback or document the deferral** (addresses: OS-dark-mode users on light palette during 0033 → 0034)
    Either add `@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { ... } }`, or move the deferral to "What We're NOT Doing" with a rationale.

12. **Add a Wave 4a → Wave 4b mapping artefact** (addresses: locked-in mappings have no traceable home)
    End-of-4a deliverable: a `mapping-conventions.md` (or typed constant) listing every `from → to` decision. Wave 4b applies it; PR drift section is a curated subset.

13. **Cosmetic fixes** (addresses: prose/code drift; AC1 mislabel; lowercase hex; `0px`/`em` regex carve-outs; `var(--token, #fallback)` test enforcement; `@deprecated` marker; export-name asymmetry)
    Bundle into one cleanup pass. None individually load-bearing but together they reduce ambiguity for the implementer.

14. **Decide the Google Fonts vs self-host question or attach a trigger** (addresses: privacy/IP leak; CSP gap; SRI gap; GDPR deferral)
    For local-dev-only this is acceptable, but the work item should record either (a) a self-host plan with the asset budget (~150 KB) or (b) a "before non-localhost deployment" trigger that re-evaluates the dependency. Optionally introduce a baseline CSP at the same time.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally sound at the macro level (preserves the single-source-of-truth pattern, extends it cleanly, lands as a CSS-only diff). Several seams need tightening: Phase 2 lands deliberately failing tests, the `EXCEPTIONS` constant and AC4 grep are parallel enforcement channels that can drift, the legacy `COLOR_TOKENS` parity is dropped before consumer migration, and the `color-mix()` choice is made implicitly without acknowledging the browser-support tradeoff the research flagged.

**Strengths**: Preserves CSS↔TS parity invariant; `EXCEPTIONS` typed constant as single source of truth; CSS-only blast radius; clean light/dark TS split; explicit retirement of legacy tokens; TDD invariant.

**Findings**:
- 🟡 major / high — *Phase 2 lands deliberately failing tests* — Phase 2 — Success Criteria
- 🟡 major / high — *Legacy `--color-*` parity coverage dropped in Phase 1* — Phase 1, §1/§2
- 🟡 major / medium — *`color-mix()` chosen without acknowledging the browser-support tradeoff* — Phase 4, "Special Conventions"
- 🔵 minor / high — *Two parallel enforcement mechanisms (typed EXCEPTIONS vs raw grep)* — Phase 2 vs Phase 4
- 🔵 minor / medium — *Block-extraction regex fragile against future CSS nesting* — Phase 1, §1
- 🔵 minor / medium — *Contrast-failure escape clause turns invariant into PR commentary* — Phase 1, §5
- 🔵 suggestion / medium — *Single-PR scope creates a large review surface* — Implementation Approach
- 🔵 suggestion / low — *No-fallback convention not codified anywhere persistent* — Phase 4, "Special Conventions"

### Code Quality

**Summary**: Plan is well-structured and TDD-disciplined. Main concerns: under-specified RGBA-composition hand-wave, `EXCEPTIONS` shape that conflates contexts, brittle aggregate `var(--*)` threshold, and several mapping/audit-trail decisions deferred to implementation time without a defined format.

**Strengths**: TDD invariant; `EXCEPTIONS` as canonical source; explicit `var(--token, #fallback)` retirement; clean phase boundaries; reuses parity-test idiom; corrects work-item parity-loop misattribution.

**Findings**:
- 🟡 major / high — *RGBA-handling note hand-waved; `contrast.ts` only accepts hex* — Phase 1, §5
- 🟡 major / high — *EXCEPTIONS matcher conflates contexts within a single file* — Phase 2, §1
- 🟡 major / medium — *Wave 4a → 4b judgement-call audit trail loose; no checkpoint* — Phase 4
- 🔵 minor / medium — *PX_REM_RE misses `em` units and `calc()` literals* — Phase 2, §1
- 🔵 minor / high — *Aggregate `var(--*)` count of 300 is brittle* — Phase 2, §1
- 🔵 minor / medium — *`color-mix()` adoption introduces a new pattern with no consumption-side tests* — Phase 4, Wave 4a §1
- 🔵 minor / medium — *`readCssVar` regex assumes well-formed input and doesn't escape `name`* — Phase 1, §1
- 🔵 suggestion / high — *`var(--token, #fallback)` retirement is not enforced by any test* — Phase 4
- 🔵 suggestion / medium — *Cross-phase coupling around temporary `COLOR_TOKENS` retention is undertested* — Phase 1, §2

### Test Coverage

**Summary**: Unusually rigorous TDD framing with per-category parity coverage and per-file enforcement via `migration.test.ts`. Several aspects are fragile or incomplete: deliberately-failing test commits to trunk, exception model permits all instances of a literal, AC2 / AC6 are manual-only, contrast for rgba/stroke tokens is sidestepped.

**Strengths**: TDD invariant; six-category parity; light + dark contrast; per-file failure granularity; vitest-time grep equivalents; preserved focus-ring assertions.

**Findings**:
- 🟡 major / high — *Plan deliberately commits a failing test suite to trunk* — Phase 2 Success Criteria
- 🟡 major / high — *Exception model permits all instances of a literal in a file* — Phase 2 §1
- 🟡 major / high — *Plan's response to potentially-failing contrast contradicts Phase 1 success criteria* — Phase 1, §5
- 🟡 major / high — *AC6's pixel-diff and ΔE thresholds have no automated measurement* — Phase 5
- 🟡 major / high — *AC2 has no automated test — only manual DevTools verification* — Phase 1, §4
- 🟡 major / medium — *Stroke and tinted-background tokens are not contrast-tested* — Phase 1, §5
- 🔵 minor / high — *Aggregate `var(--*)` count does not detect typo'd token names* — Phase 2, §1
- 🔵 minor / medium — *Mixing eager glob and top-level await for wiki-links is inconsistent* — Phase 2, §1
- 🔵 minor / medium — *`readCssVar` non-greedy regex fragile against future structural changes* — Phase 1, §1
- 🔵 minor / high — *`0px`/`0rem` resets not auto-excluded by px/rem regex* — Phase 2, §1
- 🔵 minor / medium — *No test enforces semantic correctness of hex→token mappings* — Phase 3 / Phase 4
- 🔵 minor / medium — *E2E suite referenced as regression gate but no specific tests named* — Testing Strategy

### Correctness

**Summary**: Most mechanical claims (Vite/Vitest versions, `?raw` imports, glob syntax, regex equivalence) are correct. Two genuine defects: warning-text → `--ac-err` semantic mismap, and prose mentioning deprecated `{ as: 'raw' }` while code uses the current form. A handful of smaller issues (TDD ordering, regex fragility, AC1 labelling) are worth addressing.

**Strengths**: AC4 regex equivalence verified; Vite 6 + Vitest 3 support `?raw`; `import.meta.glob` syntax correct; indigo-vs-blue collapse documented as drift; sound TDD ordering; `var(--token, #fallback)` retirement clean.

**Findings**:
- 🟡 major / high — *warning-text mapped to `--ac-err` instead of `--ac-warn`* — Phase 3, §2 line 663 / §3 line 679
- 🔵 minor / high — *Prose mentions deprecated `{ as: 'raw' }` while code uses `query/import` form* — Phase 2 §1
- 🔵 minor / high — *AC1 is a parity check, not a grep check* — Desired End State item 6
- 🔵 minor / medium — *`:root \{([\s\S]*?)\}` non-greedy match assumes flat block* — Phase 1, §1
- 🔵 minor / medium — *Inventory-verbatim hex casing is a brittle invariant* — Phase 1, §2 line 296
- 🔵 minor / medium — *Dark contrast assertions only cover overriding tokens — semantic colours need explicit handling* — Phase 1, §5
- 🔵 minor / medium — *Phase 3 CSS+TS deletions need atomicity (cross-phase coupling)* — Phase 3, §4

### Standards

**Summary**: Strong adherence to the existing parity convention and regex-test idiom. Concerns: WCAG AA failure response policy unclear, `[data-theme="dark"]` strategy omits `prefers-color-scheme` fallback, inventory-verbatim uppercase hex clashes with project lowercase convention, TS export naming asymmetric, font-loading underuses preload/subset directives.

**Strengths**: Parity test reused faithfully; forced-colors mode preserved; AA contrast extended to dark; test fixtures excluded from grep correctly; legacy `--color-*` retired rather than left coexisting.

**Findings**:
- 🟡 major / high — *WCAG AA failure response policy ambiguous — "raise in PR" is not a standards-conformant resolution* — Phase 1, §5
- 🟡 major / high — *No `prefers-color-scheme` fallback during 0033 → 0034 window* — Phase 1, §3
- 🟡 major / high — *Dark shadow overrides missed — inventory specifies per-theme `--ac-shadow-soft` / `--ac-shadow-lift`* — Phase 1, §2/§3
- 🔵 minor / high — *Hex casing convention conflicts with existing project files (lowercase elsewhere)* — Phase 1, §2 line 296
- 🔵 minor / high — *TypeScript export naming inconsistent (LIGHT_COLOR_TOKENS vs TYPOGRAPHY_TOKENS)* — Phase 1, §2
- 🔵 minor / medium — *Font-loading link omits standards-recommended preload / subset directives* — Phase 1, §4
- 🔵 minor / high — *`color-mix()` introduces a new convention without project precedent* — Phase 3, §3
- 🔵 minor / medium — *AC4 grep / EXCEPTIONS scoping risk* — Phase 2 / Phase 4
- 🔵 minor / medium — *Forced-colors mode coverage not extended to new `--ac-*` tokens* — Phase 1, §3
- 🔵 minor / high — *Temporary `COLOR_TOKENS` retention has no `@deprecated` marker* — Phase 1, §2
- 🔵 minor / medium — *Stroke contrast not asserted (WCAG 1.4.11)* — Phase 1, §5
- 🔵 suggestion / low — *No project lint config; conventions enforced only by harness + review* — Cross-cutting

### Performance

**Summary**: Modest, well-suited to local-dev target. `display=swap` and `preconnect` adequate; CSS bundle growth negligible. Font payload estimate optimistic; harness eager glob acceptable but worth noting.

**Strengths**: `preconnect` to both origins (with correct `crossorigin` on gstatic); single combined CSS request; `display=swap`; tight weight selection (8 weights total); negligible CSS bundle growth; Phase 1 retains `--color-*` briefly to avoid breaking parity mid-migration.

**Findings**:
- 🔵 minor / medium — *Font payload estimate likely understated (closer to 80–200 KB cold-cache)* — Performance Considerations
- 🔵 minor / high — *Inter at 4 weights generous without an audit of which weights migrated CSS emits* — Phase 1, §4
- 🔵 minor / high — *Eager glob + per-file describe inflates vitest startup but is bounded* — Phase 2, §1
- 🔵 suggestion / medium — *Render-blocking stylesheet link acceptable for dev-only; preload not warranted now* — Phase 1, §4
- 🔵 suggestion / low — *Self-hosting was rejected on non-performance grounds (fine)* — Drafting Notes

### Security

**Summary**: New third-party origin coupling for a tool that today ships only to local dev. No CSP exists today. Most material concern is privacy/IP leak to Google on every fresh font fetch; supply-chain and SRI gaps are secondary. Severity calibrated to local-dev-only deployment.

**Strengths**: Plan acknowledges third-party coupling rather than silently introducing it; `preconnect` `crossorigin` correctness; DevTools 200-check in Manual Verification; minimal `wght@` axis; test fixtures excluded from migration globs.

**Findings**:
- 🔵 minor / high — *Privacy/IP leak to Google for every developer on first font fetch* — Phase 1, §4 + What We're NOT Doing
- 🔵 suggestion / high — *No CSP exists today; new third-party origin is a good prompt to introduce one* — Overall plan
- 🔵 suggestion / high — *No SRI on the Google Fonts stylesheet link* — Phase 1, §4
- 🔵 suggestion / medium — *Eager glob import of CSS files runs at vitest time, not request time* — Phase 2, §1
- 🔵 suggestion / medium — *GDPR/DPA flag deferred to ops without an owner or trigger* — Performance Considerations

## Re-Review (Pass 2) — 2026-05-06T20:30:00+01:00

**Verdict:** REVISE

The structural changes from review-1 all landed correctly: Phase 2 harness lands green via pre-populated `EXCEPTIONS`, the legacy `COLOR_TOKENS` parity describe is retained until atomic Phase 3 deletion, the `color-mix()` convention is fully pinned, the WCAG hedge is removed (ratios pre-resolved against inventory), `prefers-color-scheme` is wired, dark shadow tokens have first-class exports, the warning-text mismap is corrected, AC2 has a unit test (`fonts.test.ts`), AC6 is now Playwright-automated, and Google Fonts is replaced with self-hosted woff2 — closing the privacy/CSP/GDPR/SRI bundle in one move. The remaining concerns split into two groups: (1) **fixable correctness defects** introduced during the editing pass — `__dirname` will throw under ESM, the Playwright `cluster-card` selector doesn't exist in the codebase, the contrast table cites `#1b1f2e` instead of the inventory's `#14161F`, and a `prefers-color-scheme` equality assertion is described in success criteria but not in the code listing — and (2) **residual minor concerns** — `EXCEPTIONS` file-matching uses `endsWith` on basenames (collision risk), AC5 `it.skip` defers the most ambitious gate, the new `composeOverSurface`/`parseRgba` helpers have no unit tests, dark contrast for theme-invariant `--ac-err`/`--ac-ok`/`--ac-violet` is asserted only for `--ac-warn`, and the transient mapping artefact is placed under `meta/decisions/` (project ADR namespace).

The verdict is REVISE rather than APPROVE because the Phase 1 ESM `__dirname` bug and the missing Playwright selector are blocking — the tests as written will not run.

### Previously Identified Issues

- 🟡 **architecture**: Phase 2 lands deliberately failing tests — **Resolved** (pre-populated `EXCEPTIONS`)
- 🟡 **architecture**: Legacy `--color-*` parity coverage dropped — **Resolved** (parity describe retained, atomic deletion)
- 🟡 **architecture**: `color-mix()` without browser-support tradeoff — **Resolved** (convention pinned)
- 🔵 **architecture**: Two parallel enforcement mechanisms (EXCEPTIONS vs grep) — **Partially resolved** (harness is preferred CI channel; greps remain as secondary)
- 🔵 **architecture**: Block-extraction regex fragile — **Partially resolved** (invariant comment added; not test-enforced)
- 🔵 **architecture**: Contrast-failure escape clause — **Resolved** (hedge removed; ratios pre-resolved)
- 🔵 **architecture (suggestion)**: Single-PR scope — **Still present** (acknowledged tradeoff)
- 🔵 **architecture (suggestion)**: No-fallback convention not codified — **Resolved** (enforced by `migration.test.ts`)
- 🟡 **code-quality**: RGBA hand-waved — **Resolved** (`composeOverSurface` helper added)
- 🟡 **code-quality**: EXCEPTIONS conflates contexts — **Resolved** (per-occurrence `count` model)
- 🟡 **code-quality**: Wave 4a→4b checkpoint — **Resolved** (`0033-token-mapping-conventions.md` artefact)
- 🔵 **code-quality**: PX_REM_RE misses em — **Resolved** (em added)
- 🔵 **code-quality**: AC5 threshold brittle — **Still present** (acknowledged; `it.skip` recommendation)
- 🔵 **code-quality**: `color-mix()` no consumption-side tests — **Resolved** (warn-on-warn-tinted-bg assertion)
- 🔵 **code-quality**: `readCssVar` doesn't escape `name` — **Still present** (no change)
- 🔵 **code-quality (suggestion)**: `var(--*, fallback)` not enforced — **Resolved** (test added)
- 🔵 **code-quality (suggestion)**: COLOR_TOKENS retention undertested — **Resolved** (parity describe + `@deprecated`)
- 🟡 **test-coverage**: Failing test suite to trunk — **Resolved**
- 🟡 **test-coverage**: Permitted() over-permissive — **Resolved**
- 🟡 **test-coverage**: AA contradiction — **Resolved**
- 🟡 **test-coverage**: AC6 manual — **Partially resolved** (Playwright pixel-diff lands; ΔE bound is not automated — see new finding)
- 🟡 **test-coverage**: AC2 manual — **Resolved** (with caveat: see new "fonts.test.ts gaps" finding)
- 🟡 **test-coverage**: Stroke/tint untested — **Resolved**
- 🔵 **test-coverage**: Var count typo'd names — **Resolved** (`var(--NAME)` validity test)
- 🔵 **test-coverage**: Top-level await inconsistent — **Resolved** (static globs only)
- 🔵 **test-coverage**: `readCssVar` regex fragile — **Partially resolved** (comment only; not test-enforced)
- 🔵 **test-coverage**: 0px/0rem auto-exclusion — **Resolved**
- 🔵 **test-coverage**: Semantic mapping correctness — **Still present** (relies on visual regression)
- 🔵 **test-coverage**: E2E suite — **Resolved** (downgraded; Playwright is authoritative gate)
- 🟡 **correctness**: warning-text → `--ac-err` mismap — **Resolved** (now `--ac-warn`)
- 🔵 **correctness**: Prose `{ as: 'raw' }` — **Resolved**
- 🔵 **correctness**: AC1 labelling — **Resolved**
- 🔵 **correctness**: `:root` regex non-greedy — **Partially resolved** (comment; not enforced)
- 🔵 **correctness**: Capitalisation invariant — **Resolved** (lowercase normalised; case-insensitive comparator)
- 🔵 **correctness**: Dark contrast semantic colours — **Partially resolved** (only `--ac-warn` asserted)
- 🔵 **correctness**: Phase 3 atomicity — **Resolved**
- 🟡 **standards**: WCAG AA policy — **Resolved**
- 🟡 **standards**: `prefers-color-scheme` — **Resolved** (with mirror duplication concern)
- 🟡 **standards**: Dark shadow overrides — **Resolved**
- 🔵 **standards**: Hex casing — **Resolved**
- 🔵 **standards**: TS export naming — **Resolved**
- 🔵 **standards**: Font preload directives — **Resolved** (preload added with correct `crossorigin`)
- 🔵 **standards**: `color-mix()` precedent — **Resolved** (convention pinned)
- 🔵 **standards**: AC4 grep / EXCEPTIONS scoping — **Partially resolved** (harness preferred; greps remain)
- 🔵 **standards**: Forced-colors not extended — **Resolved** (focus-ring delegates to system colour `Highlight`)
- 🔵 **standards**: COLOR_TOKENS without `@deprecated` — **Resolved**
- 🔵 **standards**: Stroke contrast — **Resolved**
- 🔵 **standards (suggestion)**: No project lint config — **Still present**
- 🔵 **performance**: Font payload underestimated — **Still present** (estimate widened; subsetting not adopted)
- 🔵 **performance**: Inter at 4 weights — **Still present**
- 🔵 **performance**: Eager glob bounded — **Still present** (acceptable; pre-empted in plan)
- 🔵 **performance (suggestion)**: Self-hosting was rejected — **Reversed** (now self-hosted)
- 🔵 **security**: Privacy/IP leak — **Resolved** (self-hosted)
- 🔵 **security (suggestion)**: No CSP — **Still present** (urgency reduced now no third-party origins)
- 🔵 **security (suggestion)**: No SRI on Google Fonts — **Resolved** (no longer needed)
- 🔵 **security (suggestion)**: Eager glob test-time — **Still present** (low risk; project-internal scope)
- 🔵 **security (suggestion)**: GDPR/DPA — **Resolved** (no third-party origin)

### New Issues Introduced

- 🟡 **correctness** (high confidence): `fonts.test.ts` uses `__dirname` which is undefined under ESM — `frontend/package.json` declares `"type": "module"`, so the test will throw `ReferenceError` at module evaluation. **Fix**: derive via `dirname(fileURLToPath(import.meta.url))` or use Vite's `import.meta.glob('../../public/fonts/*.woff2', { eager: true, query: '?url' })`.

- 🟡 **correctness** (high confidence): Phase 5 Playwright `[data-testid="cluster-card"]` selector does not exist in the codebase (verified by grep across `frontend/src/`). The `lifecycle-cluster-after-click` test will time out. **Fix**: either add the testid to `LifecycleClusterView.tsx` (in tension with "no TSX changes other than `index.html`") or replace with an existing selector (e.g., a class from `LifecycleClusterView.module.css`).

- 🟡 **test-coverage** (major / high): AC5 `it.skip` recommendation defers the ≥300 gate to end-of-Phase-4 with no progressive ratchet, leaving the 50× expansion (~6 → ~300) without per-commit enforcement. **Fix**: replace `it.skip` with a `BASELINE` constant bumped per wave (~6 → ~150 → 300) so the test ratchets monotonically.

- 🟡 **test-coverage** (major / high): `composeOverSurface` / `parseRgba` / widened `contrastRatio` have no dedicated unit tests; the contrast assertions consume them but a buggy alpha-composition formula could mask wrong intermediate colours. **Fix**: add a `contrast.helpers.test.ts` (or extend `contrast.test.ts`) with pinned cases — `parseRgba('rgba(255,0,0,0.5)')`, `composeOverSurface('rgba(0,0,0,0.5)', '#ffffff')` → `#808080`, etc.

- 🟡 **test-coverage** (major / medium): `prefers-color-scheme` mirror equality assertion is in the success criteria but absent from the `global.test.ts` code listing, and `readCssVar` explicitly forbids nested at-rules so it cannot be reused. **Fix**: show the actual extract-and-compare code (separate two-step regex over the `@media` block body), or have Playwright `page.emulateMedia({ colorScheme: 'dark' })` cover one route as a sanity check.

- 🔵 **correctness** (high confidence): Pre-resolution table cites `--ac-fg` as `#1b1f2e`, but the inventory (line 113) defines `--ac-fg = #14161F`. The actual ratio against `#FBFCFE` is 17.57 (still passes 4.5), but the table doesn't match the canonical source. **Fix**: update to `(#14161f)` and the recomputed ratio.

- 🔵 **correctness** (high confidence): The `// eslint-disable-next-line @typescript-eslint/no-deprecated` comments in the planned `global.test.ts` and `tokens.ts` snippets target a rule not configured (no eslint config / no `@typescript-eslint` dependency in `frontend/`). **Fix**: delete the comments (they're inert) or include adding eslint as a Phase 1 task with the correct rule name verified at install time.

- 🔵 **architecture / code-quality** (medium): `EXCEPTIONS` file matching uses `path.endsWith(e.file)` on basenames — fragile under future basename collisions. **Fix**: store `file` entries as paths relative to `src/` (e.g. `routes/library/LibraryDocView.module.css`) and add an invariant test that every entry resolves to exactly one CSS module.

- 🔵 **code-quality / test-coverage**: `fonts.test.ts` covers file existence and `@font-face` declarations but does not assert AC2's "each family is referenced via a typography token" clause. **Fix**: add an assertion that `TYPOGRAPHY_TOKENS['ac-font-display']` references `Sora`, body references `Inter`, mono references `Fira Code`.

- 🔵 **code-quality**: Transient `meta/decisions/0033-token-mapping-conventions.md` placed in the project's ADR namespace, with the plan stating it can be deleted post-merge — muddies the directory's "every entry is canonical" invariant. **Fix**: place under `meta/work/` adjacent to the work item, or `meta/notes/`.

- 🔵 **architecture**: `prefers-color-scheme` mirror duplicates dark declarations rather than referencing the canonical block; only an unspecified equality test guards drift. (Cross-cuts with the test-coverage finding above.) **Fix at minimum**: add an inline `/* MIRRORED: keep in sync with [data-theme="dark"] above; identity asserted in global.test.ts */` comment block.

- 🔵 **architecture**: Phase 1 Playwright baselines blend "pre-migration app" with "new self-hosted typography" (both land in Phase 1) — typography-induced and substitution-induced visual change cannot be separately audited. **Fix or accept**: either two-pass capture (before fonts / after fonts but before substitution), or document the conflation in the PR.

- 🔵 **architecture / code-quality**: Theme-invariant semantic colours absent from `DARK_COLOR_TOKENS` encode "theme-invariant" via export-location absence rather than explicitly. **Fix**: introduce `INVARIANT_COLOR_TOKENS` (or `SEMANTIC_COLOR_TOKENS`) carrying `--ac-ok`/`--ac-warn`/`--ac-err`/`--ac-violet`; LIGHT/DARK become theme-variant only.

- 🔵 **correctness / test-coverage**: Dark contrast for theme-invariant tokens is partial — only `--ac-warn` is asserted in dark. `--ac-err = #CB4647` on dark `--ac-bg` is 4.06 (above 3:1 UI minimum but close); `--ac-violet = #7B5CD9` is 3.95. **Fix**: add the three missing assertions.

- 🔵 **standards**: Zero-reset regex carve-out (`(?!0(?:px|rem|em)\b)`) excludes `0px`/`0rem`/`0em` but not `0.0px`. Low-likelihood edge case. **Fix**: comment that `0.0px` etc must use `EXCEPTIONS`, or broaden the negative lookahead to `\b0(?:\.0+)?(?:px|rem|em)\b`.

- 🔵 **performance**: `permittedCount` linearly filters `EXCEPTIONS` per literal lookup; `EXCEPTIONS hygiene` allocates a fresh `RegExp` per entry. At Phase 2's pre-populated baseline (~360 entries × ~17 files), this is super-linear but bounded (<100 ms). **Fix (optional)**: build `Map<file, Map<literal, count>>` once at module top-level.

- 🔵 **performance**: Playwright suite "run after every Wave 4a/4b commit" inflates dev-loop and CI runtime (~16 commits × 14+ tests). **Fix**: run Playwright once per wave (4a end, 4b end) plus PR-prep, not per commit.

- 🔵 **security**: No integrity verification documented for the woff2 binaries acquired from Google Fonts download. **Fix**: prefer `@fontsource/*` (npm lockfile integrity applies) or commit `public/fonts/SHA256SUMS` and assert in `fonts.test.ts`.

- 🔵 **test-coverage**: AC6 `ΔE < 5` bound has no automated measurement; only `maxDiffPixelRatio` is gated. (Already accepted by the plan's pivot from quantitative ΔE to pixel-diff under Q2=a.) **Fix or accept**: either declare ΔE explicitly out-of-scope for automated AC6 verification, or add per-mapping ΔE assertions in vitest derived from `0033-token-mapping-conventions.md`.

- 🔵 **test-coverage**: `EXCEPTIONS hygiene` test catches stale entries (occurrences === 0) but not over-counted entries (literal occurs 5× while `count: 10`); the gate degrades to "must not exceed historical maximum" rather than "must match current state". **Fix**: assert `occurrences === sumOfCountsForFileLiteral` rather than just `> 0`.

- 🔵 **test-coverage**: Wave 4a token mapping conventions (8/18/30 percentage ladder, two-blue collapse, surface = `--ac-bg`) are honour-system; no test asserts each `color-mix` site uses one of the three percentages or composes against `--ac-bg`. **Fix (optional)**: add a regex describe to enforce the convention.

- 🔵 **test-coverage**: Visual-regression suite does not exercise the `prefers-color-scheme` path (only `dataset.theme = 'dark'`). **Fix (optional)**: add per-route variants using `page.emulateMedia({ colorScheme: 'dark' })` against the same baselines.

### Assessment

The plan is structurally sound — every major and most minor findings from review-1 are resolved. The remaining work splits into:

1. **Two blocking correctness defects** (`__dirname` under ESM, missing Playwright selector) that need plan edits before implementation can start cleanly.
2. **Two test-coverage majors** (AC5 progressive ratchet, contrast helpers untested) that materially change what the harness actually catches.
3. **A `prefers-color-scheme` parity assertion** that needs to be specified, not just promised.
4. **Cosmetic/correctness fixes** (wrong `--ac-fg` hex, inert eslint comments, transient artefact directory) that are quick edits.
5. **Minor structural concerns** (EXCEPTIONS basename matching, theme-invariant colour export-location, Playwright runtime cost) that can be deferred to a follow-up if accepted now with documentation.

A third pass after these edits would likely return APPROVE.

## Re-Review (Pass 3) — 2026-05-06T21:30:00+01:00

**Verdict:** APPROVE *(after pass-3 edits applied)*

Pass-3 found one **critical** correctness defect introduced by the pass-2 fix to the AC2 wiring test (`import.meta.glob('/public/fonts/*.woff2', ...)` does not scan Vite's `public/` directory because those files are deliberately excluded from the module graph), plus three highly-confident majors (AC5 ratchet was a manually-edited constant rather than a per-commit regression catcher, `EXCEPTIONS hygiene` count regex was substring-based and would falsely match `1px` inside `11px`, the `@media` regex anchor relied on incidental column-0 indentation). All pass-3 findings have been addressed in-place; the verdict reflects the post-edit state.

### Previously Identified Issues

- 🔴 **correctness**: `import.meta.glob('/public/fonts/...')` doesn't scan public/ — **Resolved** (switched to `readdirSync(fileURLToPath(new URL('../../public/fonts/', import.meta.url)))`)
- 🟡 **test-coverage**: AC5 ratchet manually-edited, no per-commit regression catch — **Resolved** (two-sided floor + target gate; `AC5_FLOOR ≤ observed ≤ AC5_FLOOR + 0` slack; final-state gate auto-activates when floor reaches target)
- 🟡 **test-coverage**: EXCEPTIONS hygiene count regex substring-based — **Resolved** (now counts via the same `HEX_RE` / `PX_REM_EM_RE` regex family as the migration tests, then filters to exact-literal hits)
- 🟡 **code-quality**: Vite glob `/public/fonts/*.woff2` non-idiomatic — **Resolved** (replaced with node:fs + import.meta.url path)
- 🟡 **code-quality**: `@media` regex anchor relies on column-0 brace — **Resolved** (replaced regex with brace-balanced `extractBlockBody` scanner; format-tolerant)
- 🔵 **code-quality**: `contrastRatio` flag-arg signature smell — **Resolved** (split into strict `contrastRatio(fg, bg)` for hex/hex and explicit `contrastRatioComposed(fg, bg, surface)` for rgba)
- 🔵 **code-quality**: `srcRelative()` brittle to glob-pattern changes — **Resolved** (now throws with explicit error message if the input shape is unexpected)
- 🔵 **test-coverage**: parseHex 3/8-digit edge cases untested — **Resolved** (pinned tests added; 8-digit behaviour explicitly pinned at implementation time)
- 🔵 **correctness**: `--ac-ok` row in contrast table left as TBD — **Resolved** (filled with computed 4.47 ratio against dark `--ac-bg`)
- 🔵 **correctness**: MIRROR-A comment references describe title that doesn't exactly match — **Resolved** (rephrased to clearly indicate paraphrase + provide a search hint)
- 🔵 **performance**: EXCEPTIONS hygiene rebuilds regex per entry — **Mitigated** (the new word-bounded count uses the same global match arrays as `violations()`, eliminating duplicate per-entry scans; `RegExp` allocation reduced to one `matchAll` per file rather than one per entry)
- 🔵 **security**: SHA256SUMS test was `it.skip` placeholder — **Resolved** (now an unconditional, implemented assertion with both `node:fs` and `node:crypto` imports specified; missing SUMS is a hard failure, not a silent skip)

### New Issues Introduced

(none — pass-3 edits address pass-3 findings without introducing new structural concerns)

### Assessment

The plan now has:

1. **Two-sided AC5 ratchet** that catches per-commit regressions (`observed >= AC5_FLOOR`), drift-ahead failures (`AC5_FLOOR <= observed`), and a final-state gate that auto-activates at wave-4b.
2. **Brace-balanced `extractBlockBody`** that replaces fragile multiline regex anchors with a small character-counting scanner — format-tolerant.
3. **EXCEPTIONS hygiene** that counts via the migration tests' own regex family, eliminating the substring-vs-token-boundary mismatch.
4. **`fonts.test.ts` correctness fixed** — `node:fs` + `import.meta.url`-derived path for woff2 existence + a real SHA-256 integrity assertion against committed `SHA256SUMS`.
5. **Contrast helpers cleanly split** (`contrastRatio` strict-hex, `contrastRatioComposed` explicit-surface) with pinned unit tests for `parseHex`, `parseRgba`, and both contrast paths.
6. **Theme-invariant semantics** — all four (`--ac-warn`, `--ac-err`, `--ac-ok`, `--ac-violet`) are now contrast-asserted in dark with concrete inventory-derived ratios in the table.
7. **`color-mix()` convention** is mechanically enforced (8/18/30 ladder, `--ac-bg` surface).
8. **`prefers-color-scheme` parity** is unit-asserted via brace-balanced extraction *and* visual-asserted via one Playwright `emulateMedia` route.

The plan is ready for implementation. Residual minor concerns from pass-2 (single-PR review surface, no project lint config, font-payload upper-bound estimate) are accepted tradeoffs documented in the plan.
