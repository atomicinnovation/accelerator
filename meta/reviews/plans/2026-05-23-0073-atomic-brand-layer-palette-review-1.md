---
date: "2026-05-23T15:30:43+0000"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-23-0073-atomic-brand-layer-palette.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, documentation]
review_pass: 2
status: complete
---

## Plan Review: 0073 Atomic Brand-Layer Palette — Implementation Plan

**Verdict:** REVISE

The plan is a strong, well-decomposed TDD-first proposal that establishes a
clean brand → semantic → consumer layering, aligns with ADR-0026 §5's
existing eligibility framework, and respects byte-true resolved-value
preservation across the 9 light + 16 dark + 16 mirror declaration
rewrites. Many design tradeoffs are settled inline in the decisions table.
However, the plan accumulates ~11 major findings concentrated in three
areas: (1) several decisions are deliberately deferred to the implementer
(helper folding, export placement, ΔE script's PNG-decoder, alias storage
shape) where the plan's stated style is to commit; (2) test coverage
guarding the rule going forward is thinner than the one-shot enforcement
(no fixture↔BRAND_COLOR_TOKENS parity loop, no AC2 invariant, ΔE script
shipped untested); and (3) ADR-0026 §6 documentation needs sharpening
plus the Phase 4 ordering risks shipping the rule before its governance.

### Cross-Cutting Themes

- **Normaliser proliferation and helper placement** (flagged by:
  Architecture, Code Quality, Standards, Test Coverage) — After Phase 1+2
  the codebase has three near-identical CSS value normalisers
  (`canonical()` in the fixture test, `canonicaliseTokenValue` in
  `global.test.ts`, the new `canonicaliseBrand`). The plan says "may be
  folded… or kept distinct" rather than committing. Additionally, the new
  helper lives in `src/styles/` despite ADR-0026 §5 introducing
  `src/styles/testing/` as the canonical home for shared test-only
  helpers — which is the ADR the plan itself amends.

- **Deferred decisions break the plan's own decision-table style**
  (flagged by: Architecture, Code Quality, Correctness, Documentation,
  Standards) — `BRAND_COLOR_TOKENS` placement, the ΔE script's PNG
  decoder choice (pngjs/Sharp/Playwright bundled), alias storage shape
  (resolved hex vs `var()` strings), and the `extractRootBlockBody`
  disambiguation rule are all left to implementation time despite the
  plan committing on six other judgement calls in the inline design
  table.

- **Missing automated invariants for future regression** (flagged by:
  Test Coverage, Code Quality, Documentation) — AC2's exact-match rule
  is enforced one-shot during implementation but no test asserts the
  invariant going forward. A future contributor adding an `--ac-*` token
  whose hex collides with an existing brand colour gets no failing test.
  Similarly the fixture↔TS-side BRAND_COLOR_TOKENS parity loop is absent
  (only fixture↔prototype and CSS↔TS exist), and the new ΔE script ships
  with no unit tests despite being the AC5 evidence gate.

- **`extractRootBlockBody` rule based on inaccurate prototype mental
  model** (flagged by: Correctness, Architecture, Test Coverage) — The
  plan says "the prototype's other `:root` rules under media queries are
  intentionally ignored" but the prototype actually has two top-level
  (non-`@media`) `:root` blocks; the brand block is selected by source
  order, not by the stated `@media` filter. Implementation will work by
  accident; future prototype edits could silently mis-target.

- **BRAND_COLOR_TOKENS alias storage requires future-maintainer
  discipline** (flagged by: Architecture, Code Quality, Test Coverage,
  Correctness) — Aliases are stored as resolved hex with a comment
  documenting the target. The CSS uses `var()` indirection. If the
  prototype retargets an alias, both representations must update
  together; no test enforces the link. The unit test labelled "chases
  through alias tokens (var → var → hex)" is also misleading because the
  TS map stores resolved hex, so only a single hop is ever exercised.

- **Phase 4 documentation lands after the rule has shipped** (flagged
  by: Architecture, Documentation) — Phase 1+2 can merge before Phase 4
  amends ADR-0026 §6, leaving the brand → semantic indirection rule
  undocumented in its canonical decision record during the window in
  which the rule is being applied. §6's "exactly matches" wording is
  also under-specified about rgba scope and alias-target tie-breakers.

### Tradeoff Analysis

- **Comparator simplicity vs test strictness**: Wiring `canonicaliseBrand`
  into `expectMatches` on both sides (Phase 2 §2.1) is the cleanest
  comparator implementation but strips internal whitespace from both
  sides — slightly relaxing existing rgba/shadow value comparisons that
  today catch internal-whitespace drift between TS and CSS. Either
  accept the looser comparison (simple) or keep two comparators (more
  strict, slightly more code).

- **TS-side resolved-hex purity vs structural alias representation**:
  Storing aliases as resolved hex in `BRAND_COLOR_TOKENS` keeps the TS
  map a clean name→hex lookup and the parity comparator simple. Storing
  them as `var()` strings would preserve alias structure and make the
  "var → var → hex" recursion test genuinely exercise recursion, at the
  cost of more complex comparator logic and an explicit cycle guard.
  The plan's choice is defensible but worth restating with the implied
  test-name correction.

### Findings

#### Major

- 🟡 **Architecture**: Governance ADR amended after the code it governs has shipped
  **Location**: Phase 4 / Implementation Approach phase-ordering note
  Phase 4 (ADR-0026 §6 introducing the brand → semantic indirection
  rule) is allowed to ship after Phases 1-2, leaving the new
  architectural rule undocumented in its canonical decision record
  during the window in which it is being applied. Reorder to require
  Phase 4 in the same PR as Phase 2.

- 🟡 **Code Quality**: Two near-identical `rgb()→#hex` normalisers risk drifting
  **Location**: Phase 1 §1.1 (fixture `canonical()`) and §1.6 (`canonicaliseBrand.ts`)
  The same regex + padding logic appears in two places with no shared
  primitive. A future fix to one (e.g. supporting space-separated
  `rgb(r g b)`) will silently leave the other broken.

- 🟡 **Code Quality**: Ambiguous instruction: "may be folded… or kept distinct"
  **Location**: Phase 1 §1.5 / §1.6 (`canonicaliseTokenValue` folding)
  Decision left to implementation time; will produce three semantically
  overlapping normalisers if "kept distinct" wins by default.

- 🟡 **Test Coverage**: No fixture↔BRAND_COLOR_TOKENS parity loop
  **Location**: Phase 1 §1.5
  The existing fixture↔tokens.ts parity loop (`global.test.ts:221-235`)
  only covers CODE_* families. Without extending it to the brand layer,
  the TS map can drift from the fixture independently of the prototype.

- 🟡 **Test Coverage**: No automated invariant guards AC2's exact-match rule
  **Location**: Phase 2 §2.2/§2.3
  AC2 is enforced one-shot during implementation. A future contributor
  adding a new `--ac-*` token whose hex matches a brand value, or
  adding a new `--atomic-*` whose hex collides with an existing literal
  `--ac-*`, gets no failing test.

- 🟡 **Test Coverage**: ΔE2000 diff script ships untested despite gating AC5
  **Location**: Phase 3 §3.2
  The script is non-trivial (PNG decode, channel math, CIEDE2000
  aggregation, threshold gating) and ships with no unit tests. Wrong
  threshold comparison or wrong colour-space wiring could silently
  rubber-stamp regenerated baselines.

- 🟡 **Correctness**: Brand-block extractor mis-describes prototype's :root layout
  **Location**: Phase 1 §1.1 `extractRootBlockBody`
  The plan says "the prototype's other :root rules under media queries
  are intentionally ignored". Inspection shows the prototype has two
  top-level (non-`@media`) `:root` blocks; the brand block is selected
  only because it happens to be first in source order. Anchor on
  content (`--atomic-night` presence) or document the source-order
  dependency.

- 🟡 **Standards**: Shared test-only helper placed outside `src/styles/testing/`
  **Location**: Phase 1 §1.6 (`canonicaliseBrand.ts`)
  ADR-0026 §5 — the ADR this plan amends — introduces
  `src/styles/testing/` as the canonical home for shared test-only
  helpers. The `contrast.ts` precedent the plan cites is production-
  imported and therefore inapplicable. Move to `src/styles/testing/`.

- 🟡 **Documentation**: "Exactly matches" rule under-specified in ADR §6
  **Location**: Phase 4.2 ADR-0026 §6 'Decision' paragraph
  §6 omits three operationally important constraints the plan relies
  on: alpha-bearing rgba is out of scope, alias vs target tie-breaker,
  and what "normalised" means. A future contributor reading §6 in
  isolation will apply the rule inconsistently.

- 🟡 **Documentation**: ΔE script documentation strategy would not enable reuse
  **Location**: Phase 3.2
  Hand-wavy: "Document the invocation in scripts/README.md (create if
  absent)" with no file-header comment specified, no AC5 threshold (< 5)
  recorded anywhere persistent, and the conditional PNG-decoder branch
  ('pngjs if available, else Sharp, else Vitest helper') itself
  undocumented.

- 🟡 **Documentation**: PR-description requirements have no canonical template
  **Location**: Phase 2.2/2.3/3 manual-verification checklists
  Literal-residue tables, near-miss callouts, MAIN_CSS_SHA, and ΔE
  outcome statements are referenced across four sub-sections but never
  consolidated into a PR-description outline. High risk that one or
  more required artefacts (especially the `#1d2030` vs `#1d2131`
  callout) is omitted from the actual PR.

#### Minor

- 🔵 **Architecture**: Three overlapping CSS-value normalisers with unresolved consolidation
  **Location**: Phase 1 §1.6
  Same root cause as the Code Quality "ambiguous folding" finding;
  worth surfacing as an architectural concern because three normalisers
  invite coupling-by-convention rather than coupling-by-interface.

- 🔵 **Architecture**: Alias resolution duplicated between CSS cascade and TS lookup
  **Location**: Phase 1 §1.4 BRAND_COLOR_TOKENS — alias entries
  CSS stores `var(--atomic-medium-purple)` indirection; TS stores
  flattened `#965dd9` hex. Drift fixture catches CSS-vs-prototype but
  not CSS-vs-TS-alias drift.

- 🔵 **Architecture**: PNG-decoder dependency for visual-diff script is unspecified
  **Location**: Phase 3 §3.2
  "pngjs if available, else Sharp, else Vitest helper" branches at
  implementation time despite the script being reusable across future
  visual-regression PRs.

- 🔵 **Architecture**: Root-block extractor assumes single unwrapped :root rule
  **Location**: Phase 1 §1.1 extractRootBlockBody
  Companion to the Correctness "mis-describes :root layout" finding;
  worth addressing via an architectural choice (content-based anchor
  vs source-order with a guard test).

- 🔵 **Code Quality**: Silent pass-through on unknown `var(--atomic-X)` masks real errors
  **Location**: Phase 2 §2.1 canonicaliseBrand defensive fallback
  Typo in `--atomic-X` reference produces unhelpful string-mismatch
  test failure. Throw or return null sentinel instead.

- 🔵 **Code Quality**: Brace-balanced extractor logic duplicated across files
  **Location**: Phase 1 §1.1
  Three near-identical brace-balanced scanners across two test files.

- 🔵 **Code Quality**: Alias resolution inlined as comments — invisible to code
  **Location**: Phase 1 §1.4
  Add a test asserting
  `BRAND_COLOR_TOKENS['atomic-violet'] === BRAND_COLOR_TOKENS['atomic-medium-purple']`
  for each alias pair, OR derive aliases at module load.

- 🔵 **Code Quality**: Vague specification: "use pngjs if not already a dep — verify"
  **Location**: Phase 3 §3.2
  Same root concern as Architecture's PNG-decoder finding, from the
  code-quality angle.

- 🔵 **Code Quality**: BRAND_COLOR_TOKENS placement deferred to implementer
  **Location**: Phase 1 §1.4
  "either placement reads well" — pick one and record the rationale.

- 🔵 **Code Quality**: Option A presented as a choice but no Option B is shown
  **Location**: Phase 1 §1.5
  Dangling "two options" framing with only Option A described; weakens
  the plan's decision-record role.

- 🔵 **Test Coverage**: canonicaliseBrand defensive fallback masks typos rather than failing loudly
  **Location**: Phase 2 §2.1
  Same concern as Code Quality silent-pass-through finding; emphasises
  the test-coverage angle.

- 🔵 **Test Coverage**: extractRootBlockBody has no direct unit test
  **Location**: Phase 1 §1.1
  Exercised only via the integration drift detector against the real
  prototype; subtle regex bugs that happen to round-trip the real
  fixture would go undetected.

- 🔵 **Test Coverage**: Alias-chain test is misleading; recursion has no cycle guard
  **Location**: Phase 1 §1.4 + Phase 2 §2.1
  "chases through alias tokens (var → var → hex)" only exercises one
  hop because aliases are pre-resolved. Either change to genuine
  multi-hop (with cycle guard) or rename the test.

- 🔵 **Test Coverage**: No test pins the expected count of rewrites per block
  **Location**: Phase 2 §2.2/§2.3/§2.4
  A partial merge dropping one rewrite would still be symmetric across
  both dark blocks and only indirectly detected.

- 🔵 **Test Coverage**: Flipping expectMatches to canonicalise both sides weakens existing assertions
  **Location**: Phase 2 §2.1
  Strips internal whitespace from both sides; a TS-side `rgb(...)`
  value would now compare equal to CSS-side hex. Add a format guard
  test on `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS` values.

- 🔵 **Correctness**: rgb() regex is narrower than CSS syntax; out-of-range channels produce invalid hex
  **Location**: Phase 1 §1.6 canonicaliseBrand
  `\d{1,3}` admits up to 999 (e.g. `rgb(256, 0, 0)` → `#10000`, 5
  chars). Domain-safe today; document the assumption or tighten the
  regex.

- 🔵 **Correctness**: "One-bit-different in green" near-miss claim is inaccurate
  **Location**: Plan §Key Discoveries and Phase 2 §2.3
  `#1d2030` vs `#1d2131` differs in both green (+1) and blue (+1), not
  just green. Fix in both locations and in the PR-description residue
  table.

- 🔵 **Correctness**: Switching expectMatches loses test strictness on rgba/shadow values
  **Location**: Phase 2 §2.1
  See Test Coverage's same finding; correctness angle is symmetric.

- 🔵 **Correctness**: Alias-chain recursion has no cycle guard
  **Location**: Phase 1 §1.6
  Safe only by the data-shape convention that aliases store resolved
  hex; a future maintainer switching to `var()` strings without
  thinking about cycles would introduce stack-overflow risk.

- 🔵 **Standards**: BRAND_COLOR_TOKENS placement left ambiguous
  **Location**: Phase 1 §1.4
  Pick a placement and document the rationale; recommended:
  immediately before `CODE_SURFACE_TOKENS` to group theme-invariant
  families.

- 🔵 **Standards**: Export name deviates from prefix-named convention used by other :root-only families
  **Location**: Phase 1 §1.4
  Consider `ATOMIC_BRAND_TOKENS` to mirror `CODE_SURFACE_TOKENS` /
  `CODE_SYNTAX_TOKENS`; if `BRAND_COLOR_TOKENS` is preferred, record
  the rationale.

- 🔵 **Standards**: Test file naming/placement should be cross-referenced explicitly
  **Location**: Phase 2 §2.1
  Add "co-located with canonicaliseBrand.ts per the contrast.ts /
  cssRules.ts precedent".

- 🔵 **Standards**: .mjs extension diverges from existing scripts/ TypeScript convention
  **Location**: Phase 3 §3.2
  Existing `scripts/scan-css-literals.ts` is `.ts` + `tsx`; choose
  consistency or record the rationale for the divergence.

- 🔵 **Standards**: New ADR §6 structure diverges slightly from §5's heading conventions
  **Location**: Phase 4.2
  Add `### References` subsection (and optionally `### Why
  brand → semantic indirection`) to match §5's section shape.

- 🔵 **Standards**: Fixture ordering convention not stated
  **Location**: Phase 1 §1.2
  State that fixture preserves prototype source order; confirm
  formatter preserves blank-line grouping or drop the blank lines.

- 🔵 **Documentation**: §5 forward-references §6 but §6's relationship to §5 is undocumented
  **Location**: Phase 4.2
  Add one sentence to §6 Context: "This section concerns only
  brand → semantic indirection. Other :root-only families listed in §5
  do not currently have indirection consumers."

- 🔵 **Documentation**: CSS and TS comment blocks are near-duplicates
  **Location**: Phase 1.3 + 1.4
  Make CSS the canonical narrative; TS comment one-lines to "see CSS
  block header".

- 🔵 **Documentation**: Optional appendix update risks being skipped
  **Location**: Phase 4.3
  Either make required (one-sentence add, trivial) or drop entirely;
  current "optional but you should probably" framing is the worst of
  both worlds.

- 🔵 **Documentation**: JSDoc does not document recursion termination or unknown-var fallback
  **Location**: Phase 1.6
  Add two bullets describing the recursion behaviour and the
  unknown-ref defensive return.

#### Suggestions

- 🔵 **Architecture**: Phase 1 ships BRAND_COLOR_TOKENS as dead code until Phase 2 lands
  **Location**: Implementation Approach
  Either commit Phase 1 + Phase 2 in the same PR, or add explicit
  "consumers must not reference `--atomic-*` directly" guard text to
  the Phase-1 PR description.

- 🔵 **Architecture**: Symmetry of dual-layer model is not explicit in tokens.ts
  **Location**: Phase 2 §2.5
  Add 3-line comment above `LIGHT_COLOR_TOKENS` /
  `DARK_COLOR_TOKENS` stating that values are resolved hex even where
  the CSS uses `var()` indirection.

- 🔵 **Standards**: §5 list growth without stated retention policy
  **Location**: Phase 4.1
  State explicitly that the ":root-only family registry" is canonical
  in this ADR section; or move to a comment block in `tokens.ts`.

- 🔵 **Documentation**: Alias inline comments use ambiguous arrow notation
  **Location**: Phase 1.4
  Standardise to "resolved alias of atomic-medium-purple"; the word
  "resolved" makes the storage choice explicit.

### Strengths

- ✅ Four-phase decomposition mirrors the layering it introduces
  (brand foundation → semantic rewrite → visual evidence →
  governance), with each phase independently shippable and ending
  green — textbook evolutionary scaffolding.
- ✅ Choice to keep `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS` as
  resolved hex and resolve `var()` in the comparator preserves the
  existing "TS knows the resolved hex of every semantic token"
  invariant; explicitly justified in the inline design-decisions table.
- ✅ Aligns the brand layer to ADR-0026 §5's three-part eligibility
  criteria (external source, no a11y differential, ships with drift
  test), demonstrating architectural consistency with the
  `--code-*` / `--tk-*` precedent from story 0076.
- ✅ Consumer surface (462 `var(--ac-*)` references across 46 files)
  left untouched by construction — rewrite contained to declarations
  only, respecting the existing abstraction boundary.
- ✅ Hex-collision tokens (atomic-ash/geyser, atomic-slate-2/river-bed,
  atomic-sky/sky-2) preserved as name-keyed entries rather than
  dedup'd by value, correctly respecting semantic identity.
- ✅ Strong TDD ordering: every phase writes the failing test before
  the production-source edit; brand-layer parity integrated into the
  existing parameterised `describe.each` table rather than building
  parallel infrastructure.
- ✅ Theme-invariance regression tests for both the
  `[data-theme="dark"]` block and the `@media` mirror are added
  explicitly, mirroring the existing `--ac-violet` guard.
- ✅ Existing dark-block byte-equivalence parity test is leveraged
  to catch MIRROR-A/MIRROR-B desync after the 16+16 rewrites — no
  new infrastructure required.
- ✅ rgb→hex arithmetic in BRAND_COLOR_TOKENS is correct for every
  spot-checked entry (verified against `prototype-standalone.html:183`).
- ✅ The 9 light + 16 dark rewrite-candidate counts match the actual
  `--ac-*` declarations in global.css.
- ✅ CSS section commenting matches the existing
  `--code-*` / `--tk-*` section header style in global.css.
- ✅ Type export pattern (`export type BrandColorToken = keyof
  typeof BRAND_COLOR_TOKENS`) co-located after the constant matches
  the `CodeSurfaceToken` / `CodeSyntaxToken` precedent.
- ✅ Near-miss callouts (`--ac-violet` vs `--atomic-medium-purple`,
  `--ac-doc-bg-*` `#1d2030` vs `--atomic-night-4` `#1d2131`)
  documented in the plan — preventing future silent consolidation
  (modulo the "one-bit-different" inaccuracy noted under Correctness).

### Recommended Changes

1. **Reorder Phase 4 to ship with or before Phase 2** (addresses:
   Architecture "Governance ADR amended after the code it governs has
   shipped"; Documentation "§6 'Exactly matches' under-specified" via
   the same touch).
   Change the Implementation Approach diagram to either inline §4
   into the Phase 2 PR or state "Phase 4 must ship in the same PR as
   Phase 2". Strengthen §6 'Decision' with explicit scope (six-digit
   hex only; rgba out of scope) and alias-target tie-breaker (prefer
   the target so chains stay one hop deep).

2. **Move `canonicaliseBrand.ts` and `canonicaliseBrand.test.ts` into
   `src/styles/testing/`, commit the helper folding decision, and
   factor a single `rgbToHex` primitive** (addresses: Standards
   "Shared test-only helper placed outside `src/styles/testing/`";
   Code Quality "Two near-identical rgb()→#hex normalisers"; Code
   Quality "Ambiguous instruction: 'may be folded or kept distinct'";
   Architecture "Three overlapping CSS-value normalisers").
   The fixture test imports `rgbToHex` (or `canonicaliseBrand` itself);
   `canonicaliseTokenValue` is folded into `canonicaliseBrand` as the
   strict superset and deleted at §1.6. Add file-header comment naming
   consumers per ADR-0026 §5.

3. **Add three automated invariants** (addresses: Test Coverage "No
   automated invariant guards AC2"; Test Coverage "No fixture↔
   BRAND_COLOR_TOKENS parity loop"; Test Coverage "No test pins the
   expected count of rewrites per block"; Code Quality "Alias
   resolution invisible to code").
   - AC2-invariant test: iterate every `--ac-*` declaration; assert
     no `BRAND_COLOR_TOKENS` entry has the same resolved hex unless
     the declaration uses `var(--atomic-X)`. Allow-list near-misses
     in a small constant.
   - Fixture↔TS parity loop: extend the existing parameterised loop
     to include `BRAND_COLOR_TOKENS`.
   - Alias-target equality assertion: for each documented alias,
     `BRAND_COLOR_TOKENS[alias] === BRAND_COLOR_TOKENS[target]`.

4. **Fix `extractRootBlockBody` selection rule and add unit tests**
   (addresses: Correctness "Brand-block extractor mis-describes
   prototype's :root layout"; Test Coverage "extractRootBlockBody has
   no direct unit test"; Architecture "Root-block extractor assumes
   single unwrapped :root").
   Anchor by content (require the captured block contains
   `--atomic-night:`) or document explicitly that "first source-order
   top-level :root" is the disambiguator. Add 3-4 unit tests against
   synthetic HTML fragments.

5. **Decide the ΔE script's dependency surface and ship it with tests**
   (addresses: Test Coverage "ΔE2000 diff script ships untested";
   Architecture "PNG-decoder dependency unspecified"; Code Quality
   "Vague specification"; Documentation "ΔE script documentation
   strategy"; Standards ".mjs extension diverges").
   Commit to `pngjs` (smallest dep) added in §3.1 alongside `culori`.
   Ship the script as `.ts` + `tsx` matching the existing
   `scan-css-literals.ts` precedent. Add a small Vitest spec for the
   script's pure functions (identical → 0; one-channel-off → known
   ΔE; threshold-failing → exit ≠ 0). Add a self-documenting file
   header (purpose, usage, exit-code semantics, AC5 threshold link).

6. **Convert defensive fallback to a loud failure for unknown brand
   references** (addresses: Code Quality "Silent pass-through on
   unknown `var(--atomic-X)`"; Test Coverage "canonicaliseBrand
   defensive fallback masks typos"; Correctness "Alias-chain recursion
   has no cycle guard").
   Throw on unresolvable `--atomic-X` refs (closed enum at type
   level); pass non-`atomic-` `var()` refs through unchanged.
   Optionally add a `seen` cycle guard with a max depth of 8 to
   defend against future schema changes.

7. **Add a PR-description checklist section to the plan** (addresses:
   Documentation "PR-description requirements have no canonical
   template").
   Consolidate required artefacts (literal-residue tables, near-miss
   callouts including the corrected `green +1 / blue +1` description,
   MAIN_CSS_SHA, ΔE outcome statement) into one section. Fix the
   "one-bit-different" misstatement in both `Key Discoveries` and
   Phase 2 §2.3.

8. **Commit on remaining deferred decisions** (addresses: Code Quality
   "BRAND_COLOR_TOKENS placement deferred"; Code Quality "Option A
   presented as a choice but no Option B is shown"; Standards "Export
   name deviates from prefix-named convention"; Standards "New ADR §6
   structure"; Documentation "Optional appendix update"; Architecture
   "Phase 1 ships BRAND_COLOR_TOKENS as dead code").
   - Place `BRAND_COLOR_TOKENS` immediately before `CODE_SURFACE_TOKENS`.
   - Either rename to `ATOMIC_BRAND_TOKENS` or record the
     `BRAND_COLOR_TOKENS` rationale inline.
   - Drop the "two options" framing in §1.5; present Option A as the
     decision.
   - Add `### References` subsection to ADR §6.
   - Make §4.3 appendix update required (one sentence) or drop it.
   - State explicit policy on Phase 1/2 PR coupling.

9. **Tighten lower-impact documentation and naming** (addresses:
   Documentation "JSDoc does not document recursion termination";
   Documentation "Alias inline comments use ambiguous arrow notation";
   Documentation "CSS and TS comment blocks are near-duplicates";
   Architecture "Symmetry of dual-layer model is not explicit"; Test
   Coverage "Alias-chain test is misleading").
   - JSDoc: add recursion + unknown-ref bullets.
   - Alias comments: standardise to `// resolved alias of <target>`.
   - Make CSS comment the canonical narrative; TS one-liner cross-refs.
   - Add 3-line comment above `LIGHT_COLOR_TOKENS` /
     `DARK_COLOR_TOKENS` documenting the resolved-hex invariant.
   - Rename the alias-chain test to "resolves alias to hex via
     BRAND_COLOR_TOKENS" (since recursion is currently one hop).

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan introduces a clean brand → semantic → consumer
layering that closes the 0033-scoped gap honestly, with a
well-decomposed four-phase TDD approach that preserves resolved-value
equivalence. Architectural concerns sit around (a) the deliberate
divergence between CSS-side indirection and TS-side resolved-hex storage
that requires a bespoke comparator to bridge, (b) the ordering of
ADR-0026 amendment relative to the code change it governs, and (c)
overlapping normaliser helpers whose consolidation is left unresolved.

**Strengths**:
- Phase decomposition mirrors the layering it introduces; each phase
  independently shippable with green tests.
- TS-side resolved-hex invariant preserved via comparator extension.
- Aligned with ADR-0026 §5 eligibility criteria; mirrors story 0076
  precedent.
- Consumer surface (462 `var(--ac-*)` refs) left untouched by
  construction.
- Hex-collision tokens preserved as name-keyed entries respecting
  semantic identity.
- Near-miss callouts (`--ac-violet`, `--ac-doc-bg-* #1d2030`) made
  explicit.

**Findings**: 1 major (governance ADR ordering), 4 minor (normaliser
proliferation, alias duplication, PNG-decoder unspecified, root-block
extractor assumption), 2 suggestions (Phase 1 dead-code window, TS
dual-layer model not explicit).

### Code Quality

**Summary**: Helper colocation follows existing precedents and the
brand-layer comparator is well-factored, but several decisions deferred
to implementation time (helper folding, placement, ΔE decoder) risk
inconsistency, and the defensive fallback on unknown brand refs would
mask real bugs.

**Strengths**:
- Helper colocation follows `contrast.ts` precedent.
- TS-side `BRAND_COLOR_TOKENS` kept as pure resolved-hex map.
- Aliases annotated inline with arrow comment.
- Near-miss cases surfaced as PR-description tables.
- Theme-invariance guards added explicitly.
- Phasing is genuinely independent and reviewable.

**Findings**: 2 major (rgb→hex duplication, ambiguous folding), 6
minor (silent unknown-var pass, brace-balance duplication, alias
invisible to code, ΔE vague spec, BRAND placement, dangling options).

### Test Coverage

**Summary**: Strong TDD framing and good reuse of existing
infrastructure, but several risk areas lack explicit coverage —
fixture↔TS parity for brand tokens, an automated AC2 invariant, ΔE
script unit tests, comparator edge cases (cycle detection, defensive
fallback semantics, hex-collision name selection).

**Strengths**:
- TDD ordering throughout.
- Brand-layer parity integrated into existing `describe.each` table.
- Theme-invariance regression tests for both dark blocks.
- MIRROR-A↔MIRROR-B parity test leveraged for symmetry check.
- Shared canonicaliser helper extracted up front.
- Hex collisions and near-miss residues documented.

**Findings**: 3 major (fixture↔TS parity, AC2 invariant, ΔE untested),
5 minor (defensive fallback, extractRootBlockBody untested, misleading
alias-chain test, no rewrite count test, expectMatches strictness loss).

### Correctness

**Summary**: The plan is logically thorough and the bulk of algorithmic
claims hold up: 37-token count verified, rgb→hex arithmetic correct,
9 light + 16 dark counts match the actual CSS, alias recursion is
domain-safe. Two concerns: `extractRootBlockBody`'s stated
disambiguator does not match the prototype's actual layout, and a few
boundary-condition specifics in the canonicaliser regex and a
misstated "one-bit-different" near-miss could mask future regressions.

**Strengths**:
- rgb→hex arithmetic correct in spot-checks.
- 9 light + 16 dark rewrite-candidate counts match actual CSS.
- 37-token enumeration verified against the prototype.
- Alias recursion provably terminating under current data shape.
- Whitespace normalisation preserves rgba/typography/shadow equivalence.

**Findings**: 1 major (extractRootBlockBody mis-describes prototype),
4 minor (rgb regex narrowness, "one-bit" inaccuracy, expectMatches
strictness loss, recursion has no cycle guard).

### Standards

**Summary**: Largely well-aligned with project conventions —
ADR-0026 §5 extension, 0076 fixture pattern mirror, CSS comment style,
type export co-location. Three deviations worth correcting: the new
test helper should sit under `src/styles/testing/` per ADR-0026 §5;
`BRAND_COLOR_TOKENS` placement is ambiguous; export naming diverges
from the prefix-named convention used by other :root-only families.

**Strengths**:
- ADR-0026 §5 extended with a parallel entry, not a new structure.
- Fixture/drift-test pattern mirrors 0076 byte-for-byte.
- CSS section commenting matches existing `--code-*` / `--tk-*` style.
- Type export pattern co-located after constant per `CodeSurfaceToken`
  precedent.
- Theme-invariance test modelled on existing `--ac-violet` guard.
- ADR amendment shipped in same story per 0076 precedent.

**Findings**: 1 major (test-only helper placement), 7 minor (placement
ambiguity, naming convention, test-file cross-ref, .mjs vs .ts, ADR §6
structure, fixture ordering, retention policy).

### Documentation

**Summary**: Unusually well-documented for its size, with strong
"why"-focused inline comments and a clean ADR amendment. However, §6's
"exactly matches" wording is operationally under-specified, the new ΔE
script's documentation deferral leaves several reusability gaps, and
the PR-description requirements are scattered across four sub-sections
with no canonical template.

**Strengths**:
- Phase 4 cleanly amends ADR-0026 matching story 0076 precedent.
- CSS comment in §1.3 cross-references TS export, fixture, ADR, and
  prototype source — strong "why" content.
- TS comment in §1.4 explains non-obvious invariant about aliases.
- §1.6 JSDoc enumerates the four normalisation behaviours.
- ADR §6 Operational guidance gives concrete forward-looking instructions.
- Plan cross-references stories 0076, 0033, 0077, 0082 lineage.

**Findings**: 3 major (§6 under-specified, ΔE docs hand-wavy, no PR
template), 5 minor (§5↔§6 relationship, CSS/TS comment duplication,
optional appendix limbo, JSDoc gaps, ambiguous arrow notation).

## Re-Review (Pass 2) — 2026-05-23T15:30:43+0000

**Verdict:** APPROVE

All 11 pass-1 major findings are resolved. Pass-2 surfaced a small
cluster of secondary issues introduced by the revision itself
(notably a stub cycle-guard test, an unspecified
`extractAllAcDeclarations` helper, and a contradictory `--ac-violet`
example in ADR §6). The plan was iterated again to close these. The
remaining residual concerns are all minor stylistic nits with no
load-bearing impact; the plan is in good shape for implementation.

### Previously Identified Issues

#### Pass-1 majors — resolution status

- 🟡 → ✅ **Architecture**: Governance ADR amended after the code it
  governs has shipped — **Resolved**. Phase 4 now bundled with
  Phases 1+2 in PR-A.
- 🟡 → ✅ **Code Quality**: Two near-identical rgb→hex normalisers —
  **Resolved**. `rgbToHex` factored as shared primitive;
  `canonicaliseTokenValue` folded and deleted; fixture imports the
  shared helper.
- 🟡 → ✅ **Code Quality**: Ambiguous "may be folded or kept distinct"
  — **Resolved**. Decision committed; no ambiguous wording remains.
- 🟡 → ✅ **Test Coverage**: No fixture↔BRAND_COLOR_TOKENS parity
  loop — **Resolved**. Extension to existing parity loop spelled
  out in §1.5(b).
- 🟡 → ✅ **Test Coverage**: No automated AC2 invariant — **Resolved**.
  AC2-invariant guard + rewrite-count guard tests added in §2.1.
- 🟡 → ✅ **Test Coverage**: ΔE script untested — **Resolved**.
  Companion `visual-diff-ciede2000.test.ts` with 4 named cases.
- 🟡 → ✅ **Correctness**: Brand-block extractor mis-describes
  prototype's :root layout — **Resolved**. Selection now
  content-anchored on `--atomic-night:` with direct unit tests.
- 🟡 → ✅ **Standards**: Helper placed outside `src/styles/testing/`
  — **Resolved**. Moved per ADR-0026 §5 convention.
- 🟡 → ✅ **Documentation**: §6 "exactly matches" under-specified —
  **Resolved**. Scope clause (six-digit hex only) and alias-target
  tie-breaker added (and the contradictory `--ac-violet` example
  reworded in pass-2 follow-up).
- 🟡 → ✅ **Documentation**: ΔE docs hand-wavy — **Resolved**.
  Self-documenting file header committed; tests; pngjs pinned.
- 🟡 → ✅ **Documentation**: No PR-description template — **Resolved**.
  New "PR Descriptions" section with per-PR checklists; pass-2
  follow-up added the guardrail-tests bullet.

#### Minor / suggestion findings — all addressed by edits

Every minor and suggestion from pass 1 was addressed: helper
consolidation, placement decisions, naming rationale, "Two
options" framing dropped, ADR §6 References subsection added,
appendix wording cleaned, JSDoc enriched, alias arrow notation
standardised to "resolved alias of X", dual-layer invariant
documented above LIGHT/DARK_COLOR_TOKENS, "one-bit-different"
misstatement corrected to "green +1, blue +1" in three places,
TS comment block deduplicated against CSS, §5↔§6 relationship
sentence added.

### New Issues Introduced (pass-2 follow-up applied)

The revision introduced four secondary issues; each was fixed
during the pass-2 iteration:

- 🔵 **Correctness**: ADR §6 tie-breaker example used `--ac-violet`
  matching `#965dd9` despite Key Discoveries establishing
  `--ac-violet` is `#7b5cd9` (near-miss). **Fixed**: example
  reworded to use a hypothetical `--ac-brand-purple` and explicitly
  note `--ac-violet` is a near-miss.
- 🟡 **Test Coverage / Code Quality / Correctness / Documentation**:
  Cycle-guard test had a comment-only body, asserting nothing.
  **Fixed**: test now uses `vi.doMock` to inject a cyclic brand
  map and asserts `toThrow(/cycle detected/)`.
- 🟡 **Test Coverage / Code Quality / Architecture / Standards**:
  `extractAllAcDeclarations` referenced without specification.
  **Fixed**: helper spec'd at
  `src/styles/testing/extractAcDeclarations.ts` with signature,
  block-tag convention, and its own unit tests; companion
  `extractBlockBody` promoted to `src/styles/testing/cssBlocks.ts`
  as a shared primitive (also closes the brace-balanced extractor
  duplication finding).
- 🔵 **Standards**: `ATOMIC_ALIAS_PAIRS` naming contradicted the
  `BRAND_COLOR_TOKENS` rationale. **Fixed**: renamed to
  `BRAND_ALIAS_PAIRS`.

Additional small fixes applied in pass-2:
- `seen` parameter on `canonicaliseBrand` hidden behind an internal
  `resolve()` helper so the public signature is single-argument.
- Format-guard test extended to reject both `var(--atomic-` and
  bare `rgb(` on the TS side (closes the strictness-loss
  half-fix).
- PR-A checklist amended to require AC2-invariant, rewrite-count,
  and format-guard tests be green and explicitly include allow-list
  reconciliation.
- `(if present)` hedging on ADR-0025 reference replaced with the
  concrete story-0033 link.

### Residual Pass-2 Findings (not addressed; documented for transparency)

A handful of small nits surfaced in pass 2 that are not load-
bearing and have been left for the implementer to handle naturally:

- 🔵 **Code Quality**: Format-guard test name advertises three
  shapes but asserts only negative checks — pragmatically fine;
  the implementer may rename or strengthen.
- 🔵 **Test Coverage**: One-channel-off ΔE "golden" value is not
  pinned in the plan — the implementer should source it from a
  published CIEDE2000 reference rather than `culori`'s output.
- 🔵 **Test Coverage**: Pathological brace-input test for
  `extractRootBlockBody` not enumerated concretely — implementer
  to choose adversarial inputs.
- 🔵 **Standards**: Fixture ordering convention still not stated
  explicitly — implementer should record whichever convention the
  existing `--code-*` / `--tk-*` entries follow.
- 🔵 **Documentation**: Dual-layer comment in §2.5 duplicates §6
  prose — minor cross-reference improvement available but not
  required.
- 🔵 **Architecture**: Test helper imports a production module
  (`BRAND_COLOR_TOKENS`) — intentional coupling; file-header
  comment notes the dependency.

### Assessment

The plan is now in good shape for implementation. All previously
identified blocking concerns are resolved, the test-coverage
posture is strong (8 new test categories plus the ΔE script
self-tests), and the design decisions table has no remaining
deferred choices. Verdict moves from REVISE → COMMENT: ready to
proceed.

