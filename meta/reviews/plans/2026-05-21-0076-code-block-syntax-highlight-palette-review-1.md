---
date: "2026-05-22T01:00:00+01:00"
type: plan-review
producer: review-plan
target: "plan:2026-05-21-0076-code-block-syntax-highlight-palette"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, documentation]
review_pass: 3
status: complete
id: "2026-05-21-0076-code-block-syntax-highlight-palette-review-1"
title: "2026-05-21-0076-code-block-syntax-highlight-palette-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-22T01:00:00+01:00"
last_updated_by: Toby Clemson
---

## Plan Review: Code-Block Syntax-Highlight Palette

**Verdict:** REVISE

The plan is unusually rigorous on test-first sequencing, drift detection, and parity contracts — phase ordering is sound, the prototype-tokens fixture is an excellent durable seam, and the EXCEPTIONS hygiene mechanism is correctly leveraged. However, the review surfaced three concrete correctness defects that would prevent Phase 1 or Phase 2 from going green (rgba whitespace normalisation gap on `--code-stroke`, missing `routeTree.addChildren` registration for the showcase route, and a Phase 4 deletion that silently drops bold/colour styling on `.hljs-section`/`.hljs-strong`/`.hljs-emphasis`), plus several major coverage gaps in testing infrastructure (`migration.test.ts` declared-token set + `AC5_FLOOR` ratchet not updated; github.css removal not guarded by any automated test; cascade-source-order claims not test-protected; multiple inverted/regex-based assertions are bypass-able). Revising before implementation is strongly recommended — most fixes are small, surgical, and the plan's existing structure absorbs them cleanly.

### Cross-Cutting Themes

- **Brittle CSS-rule regex testing** (flagged by: architecture, code-quality, test-coverage, correctness, standards) — The `${escaped}[^{}]*\{([^}]*)\}` pattern duplicated across `code-syntax.test.ts` and `LibraryTemplatesView.test.tsx` is selector-substring-permissive (`.hljs-attr` matches inside `.hljs-attribute`; `.hljs-meta` matches inside `.hljs-meta.doctype`), and its `some(b => ...)` check can pass spuriously when only one of several matched blocks carries the expected colour. A future cascade reorder or selector rename would silently regress under green Vitest output.
- **Theme-invariance convention not codified** (flagged by: architecture, standards, documentation) — `--code-*`/`--tk-*` create a new "theme-invariant family" category but the proposed ADR §3a documents only the specific instance, not the criteria, the test-guards, or the `tokens.ts` signalling for future theme-invariant families. The next family to face the same choice will rediscover this.
- **Test infrastructure ratchets/guards not updated alongside new tokens** (flagged by: test-coverage, standards, correctness) — Three required updates are missing from the plan: (a) extend the `declared` Set in `migration.test.ts:342` to include `CODE_SURFACE_TOKENS` + `SYNTAX_TOKENS` (otherwise Phase 2/3 fail the var-resolves-to-declared test); (b) bump `AC5_FLOOR` per the documented protocol when Phase 3 adds three new `var(--code-*)` references; (c) add an explicit automated assertion that the github.css import is absent from `main.tsx` after Phase 3.
- **Showcase route role ambiguity** (flagged by: architecture, code-quality, documentation, standards) — `/code-syntax-showcase` is positioned as "dev/test-only Playwright fixture, not a design-system page" but has no durable code-level marker, no `.module.css` companion (departs from the four-file showcase precedent), is missing `routeTree.addChildren` registration, and has FIXTURES that quietly couple to specific Playwright-asserted classes without documentation.
- **Defined-but-unused `--tk-*` tokens are unmarked** (flagged by: architecture, code-quality, documentation) — Six tokens ship without an active selector in the shared layer but the inline comments in `global.css` and `tokens.ts` don't flag which six. A future cleanup pass could legitimately delete them.

### Tradeoff Analysis

- **Phase 2 transient two-colour-system vs. larger PR scope** — Phase 2 deliberately leaves the templates preview rendering the old palette (local higher-specificity rules win) until Phase 4 ships. Architecture flags the visible inconsistency; the plan's phase-independence rationale gains reviewable PR size. Recommendation: invert phase 4's CSS deletion into phase 2 (strip local rules immediately so the shared layer wins everywhere) while leaving the test inversion and Playwright work in phase 4. This eliminates the transient state without bloating phase 2.

### Findings

#### Critical

(none)

#### Major

- 🟡 **Correctness**: Fixture parity test will fail on `--code-stroke` due to whitespace normalisation gap
  **Location**: Phase 1, §6b (prototype fixture ↔ tokens.ts parity test)
  Fixture is `"rgba(255,255,255,0.07)"` (no inner whitespace); `tokens.ts` declares `'rgba(255, 255, 255, 0.07)'` with spaces. The plan's `.replace(/\s+/g, ' ')` normalisation finds nothing to replace on the fixture side; the comparison fails on first run, blocking Phase 1 success criteria.

- 🟡 **Correctness**: New showcase route is never added to `routeTree.addChildren` — route will be unreachable
  **Location**: Phase 2, §4 (Dev-only showcase route + component)
  The plan's snippet uses plain-object shape `{ path, component }` rather than `createRoute({ getParentRoute: () => rootRoute, ... })`, and never updates `rootRoute.addChildren([...])`. Without registration, `page.goto('/code-syntax-showcase')` fails and the Playwright AC2/AC3 spec cannot run.

- 🟡 **Correctness**: Phase 4 deletion drops `font-weight`/`color` on `.hljs-section`, `.hljs-strong`, `.hljs-emphasis`
  **Location**: Phase 4, §2 (Remove local hljs rules) and §1b (PREVIOUSLY_LOCAL_MAPPINGS assertions)
  Existing templates-preview rules style `.hljs-section`/`.hljs-strong` as `font-weight: 600` plus a colour, and `.hljs-emphasis` with a colour plus italic. The shared layer adds only `font-style: italic` and `font-weight: 600` (no colour) for emphasis/strong, and only `color: var(--tk-header)` (no weight) for section. After Phase 4, markdown headings in the templates preview lose bold; emphasis/strong lose colour distinction. The Phase 4 tests do not detect this.

- 🟡 **Test Coverage**: Structural CSS test cannot prove `.language-diff` overrides win the cascade
  **Location**: Phase 2, §1 (`code-syntax.test.ts`)
  The Vitest spec regex-scans the file text for `color: var(--tk-dhdr)` but doesn't prove specificity/order resolution. A future refactor that changes specificity (e.g. moving to `[data-language='diff']`) would silently break the override under green Vitest output. Only Playwright catches it.

- 🟡 **Test Coverage**: No automated test asserts github.css import is removed from `main.tsx`
  **Location**: Phase 3, §5 (Remove github.css import)
  Phase 3's success criteria verify positive properties (resolved colours, EXCEPTIONS hygiene) but no assertion locks the absence of `import 'highlight.js/styles/github.css'`. A future contributor could re-add it to "fix a missing class" and CI stays green.

- 🟡 **Test Coverage**: Several AC2 mapping-table rows depend on hljs class emissions not validated by the showcase fixture
  **Location**: Phase 2, §5 (Playwright AC2 locator table)
  Rows depending on `.hljs-built_in` (python `print`), `.hljs-property` (TS `obj.prop` — not in fixture), `.hljs-meta` (in yaml fence — unlikely emission), `.hljs-meta.doctype` (parenthetically flagged "add to fixture") will fail at first run for emission reasons, not styling reasons. The `.first()` locator produces a confusing "colour undefined" diagnostic.

- 🟡 **Test Coverage**: Inverted CSS-text assertion is bypass-able under selector renames
  **Location**: Phase 4, §1 (`.previewBody :global(.hljs-` regex)
  The negated regex catches only the exact form. A future contributor moving the rule to `.previewPane :global(.hljs-meta)` or wrapping in `@media` escapes the guard. The "shared layer is the single source of truth" invariant is selector-form-bound.

- 🟡 **Standards**: `__fixtures__/` subdirectory has no precedent in the codebase
  **Location**: Phase 1, §1 (Prototype fixture)
  Zero existing `__fixtures__/` directories in the visualiser; the dunder convention is Jest-flavoured and clashes with the visualiser's existing flat test-co-location. Setting precedent here forces a future choice between `__fixtures__/` and flat for every fixture.

- 🟡 **Standards**: Two-letter `--tk-*` prefix breaks the established full-word family-prefix convention
  **Location**: Phase 1, §4 (token declarations)
  Existing prefixes are mostly semantic full words (`--ac-*`, `--radius-*`, `--shadow-*`, `--lh-*`, `--tracking-*`; `--sp-*` is the only short form). `--tk-*` is opaque (does it mean "token", "syntax token", something else?) and the 27 declarations dominate `:root` with a non-self-documenting prefix. The new `CODE_SURFACE_TOKENS` family uses the full-word `--code-*` pattern alongside it — inconsistent within the same story.

- 🟡 **Standards**: Plan omits updating the `declared` token Set in `migration.test.ts`
  **Location**: Phase 2 / Phase 3 — `var(--NAME) references resolve to declared tokens`
  `migration.test.ts:342` builds a `declared` Set from existing token families (`LIGHT_COLOR_TOKENS`, …, `LAYOUT_TOKENS`). Phase 2 introduces `var(--tk-*)` references in `code-syntax.global.css` (caught by the `*.global.css` glob) and Phase 3 adds `var(--code-*)` in `MarkdownRenderer.module.css`. Without spreading the two new families into `declared`, this test will fail for every new var reference. The plan only says it "still passes".

- 🟡 **Standards**: AC5_FLOOR ratchet bump procedure not specified
  **Location**: Phase 3, Success Criteria
  The two-sided ratchet at `migration.test.ts:381-405` requires implementers to bump `AC5_FLOOR` upward in the same commit that adds new `var(--*)` references. Phase 3 adds three (`--code-bg`, `--code-fg`, `--code-stroke`); the plan does not mention bumping, leaving the ratchet slack.

- 🟡 **Architecture**: New "theme-invariant family" category is introduced as a one-off rather than formalised as a token-system convention
  **Location**: Desired End State bullets 1-2; Phase 3 §3 (ADR-0026 amendment)
  Every other family lives in three places (`:root`, `[data-theme="dark"]`, `@media`); the new family lives only in `:root` by design. Proposed §3a documents this instance narratively but doesn't codify when a family may legitimately be theme-invariant, what tests guard the asymmetry, or how `tokens.ts` should signal it.

- 🟡 **Architecture**: Phase 2 leaves the application in a deliberate two-colour-system state until phase 4 ships
  **Location**: Phase 2 Success Criteria
  Local `.previewBody :global(.hljs-*)` rules (0,2,0) continue to win over the new shared layer (0,1,0) until Phase 4. Visual result depends on the route: `/library/work-items/*` shows new palette, `/library/templates/*` shows old. If Phase 4 stalls, the "single source of truth" goal is partially undelivered indefinitely.

- 🟡 **Code Quality**: Brittle CSS-rule regex duplicated in two test files
  **Location**: Phase 2 §1; Phase 4 §1(b)
  The same `${escaped}[^{}]*\{([^}]*)\}` regex appears in `code-syntax.test.ts` and `LibraryTemplatesView.test.tsx`. Substring-permissive (matches `.hljs-attribute` when searching `.hljs-attr`); multi-rule false-positive (only one captured block needs the right colour for `some()` to pass).

- 🟡 **Code Quality**: `hexToRgbString` validation is inconsistent and loses diagnostic context
  **Location**: Phase 1 §3
  Rejects 3-digit and bare hex but doesn't validate inner characters — `hexToRgbString('#ZZZZZZ')` returns `'rgb(NaN, NaN, NaN)'`. Diverges from neighbouring `parseHex` (which accepts 3-digit) with no comment explaining the divergence. Error message is generic, hurting Playwright diagnostics.

- 🟡 **Documentation**: Proposed ADR §3a omits the "why theme-invariant" rationale and forward guidance
  **Location**: Phase 3, §3 ADR-0026 amendment
  Drafted text states the family is theme-independent but doesn't record *why* (prototype renders identically against light and dark page chrome — there are screenshot files proving this) or *when* a future contributor should add a new `--tk-*` mapping vs reuse an existing one. Once the plan is archived, the ADR is the only durable artefact.

- 🟡 **Documentation**: Six defined-but-unused `--tk-*` tokens not flagged in code
  **Location**: Phase 1 §4 (global.css) and §5 (tokens.ts)
  `--tk-macro`, `--tk-key`, `--tk-flag`, `--tk-heredoc`, `--tk-lifet`, `--tk-atrule` ship without consumers in the shared layer. Status captured only in plan; future readers may delete them as dead code (drift fixture catches prototype-side deletions, not in-repo).

- 🟡 **Documentation**: Showcase route's "dev-only, not a design-system page" intent has no durable code-level marker
  **Location**: Phase 2, §4
  `CodeSyntaxShowcase.tsx` is structurally identical to `GlyphShowcase`/`ChipShowcase` (which ARE design-system pages); without a header comment marking its dev-only/Playwright-fixture role, a future contributor working on story 0083 may link it from DevDesignSystem.

#### Minor

- 🔵 **Architecture**: Showcase route conflates "developer preview" and "Playwright test fixture" responsibilities (Phase 2 §4)
- 🔵 **Architecture**: Six unmapped `--tk-*` tokens ship without an architectural retirement path (Desired End State bullets 14-15)
- 🔵 **Architecture**: Compound `.hljs-meta.doctype` vs `.hljs-meta` may produce spurious test passes for mis-ordered rules (Phase 2 §1)
- 🔵 **Architecture**: `__fixtures__/` subdirectory convention introduced without formal scope (Phase 1 §1)
- 🔵 **Code Quality**: Showcase FIXTURES couple page rendering to test-spec needs without documentation (Phase 2 §4)
- 🔵 **Code Quality**: Acknowledged "ad-hoc rgba formatter at Playwright spec sites" invites duplication (Phase 1 §3)
- 🔵 **Code Quality**: Conflating "border styling" under one `1px` EXCEPTION dilutes diagnostic value (Phase 3 §2)
- 🔵 **Code Quality**: Twenty-seven syntax tokens with cryptic 2-3 letter suffixes lack a legend in tokens.ts (Phase 1 §5)
- 🔵 **Test Coverage**: Dark-theme repeat risks copy-paste fan-out without parameterisation (Phase 2 §5)
- 🔵 **Test Coverage**: Diff spec lacks paired "general .hljs-meta resolves to tk-deco" assertion (Phase 2 §5)
- 🔵 **Test Coverage**: AC4 edge cases beyond the six listed are unconsidered (Phase 3 §1)
- 🔵 **Test Coverage**: hexToRgbString tests omit 8-digit hex and rgba canonical form (Phase 1 §3)
- 🔵 **Test Coverage**: CodeSyntaxShowcase smoke test scope is under-specified (Phase 2 §6)
- 🔵 **Test Coverage**: Optionality of templates-preview Playwright verification leaves Phase 4 under-covered (Phase 4 §3)
- 🔵 **Correctness**: Plan rationale for diff-rule ordering misattributes source order vs specificity (Phase 2 §2)
- 🔵 **Correctness**: Compound `.hljs-meta.doctype` class may never be emitted by highlight.js (Phase 2 §2 and §1)
- 🔵 **Correctness**: Drift-test path-resolution snippet is pseudocode and could mis-resolve at runtime (Phase 1 §2)
- 🔵 **Correctness**: Selector-prefix substring matches inflate the rule-search domain (Phase 2 §1)
- 🔵 **Standards**: ADR amendment style left ambiguous — both `§3a` and "extend §3" offered (Phase 3 §3)
- 🔵 **Standards**: `SYNTAX_TOKENS` export name doesn't share a `CODE_*` prefix with sibling family (Phase 1 §5)
- 🔵 **Standards**: Showcase route lacks a `*.module.css` companion despite glyph/chip precedent (Phase 2 §4)
- 🔵 **Standards**: Theme-invariant palette bypasses the project's existing 3:1 contrast contract without a tracked successor (Desired End State; Phase 1)
- 🔵 **Standards**: `:root` insertion point breaks the implicit concrete-to-abstract ordering (Phase 1 §4)
- 🔵 **Documentation**: Screenshot re-baseline procedure referenced but not pointed to (Migration Notes)
- 🔵 **Documentation**: New shared CSS file's header comment is shallower than the `wiki-links.global.css` precedent it cites (Phase 2 §2)
- 🔵 **Documentation**: `tokens.ts` comment for the new families could link back to the prototype source (Phase 1 §5)

#### Suggestions

- 🔵 **Documentation**: Six new behaviour tests do not name AC4 of story 0076 as their source (Phase 3 §1)
- 🔵 **Documentation**: Two parallel REQUIRED_MAPPINGS / PREVIOUSLY_LOCAL_MAPPINGS tables share rows without cross-reference (Phase 2 §1 and Phase 4 §1)
- 🔵 **Code Quality**: Mapping table and token list are two parallel data structures with no enforced cross-reference (Phase 2 §1)

### Strengths

- ✅ Test-first cadence enforced phase-by-phase, with explicit "fails before step X lands" notes pinning regression-protection intent
- ✅ Three-way parity (prototype fixture ↔ `tokens.ts` ↔ `global.css`) is the right structural floor for theme-invariant token families; one `it()` per token localises failures cleanly
- ✅ Drift-detection test against `prototype-standalone.html` turns the prototype into a live reference rather than a one-shot copy, preventing silent divergence
- ✅ Resolved-colour assertions correctly delegated to Playwright (jsdom `var()` substitution unreliability explicitly cited)
- ✅ Truncation-guard extension (`--tk-ddel`) correctly addresses the `readCssVar` flat-block silent-truncation failure mode
- ✅ Phase sequencing correctly identifies phase 1 as foundation (no consumer change), phase 2 as activation point, and phases 3-4 as independent leaves
- ✅ Token values match the prototype source verbatim — 5 surface + 27 syntax tokens, exact hex values
- ✅ Layer boundary follows the `wiki-links.global.css` precedent — single shared CSS module, imported globally, consumed by DOM scope rather than per-component import
- ✅ Hex casing convention is deliberate: lowercase in `tokens.ts`/`global.css`, uppercase in the fixture for byte-equality with the prototype source
- ✅ `1px` EXCEPTION count bump (2→3) is explicitly documented and the `EXCEPTIONS hygiene` test is named as the safety net
- ✅ Split between `CODE_SURFACE_TOKENS` (chrome) and `SYNTAX_TOKENS` (inline spans) respects natural cohesion lines
- ✅ github.css load-order tie-break analysis is correct: new layer imported after `global.css` lands later than github.css at line 7

### Recommended Changes

Ordered by priority. Each change references the finding it addresses; group similar fixes into single edits where convenient.

1. **Add `routeTree.addChildren` registration for the showcase route** (addresses: "showcase route is never added to routeTree")
   Update Phase 2 §4 to use the `createRoute({ getParentRoute: () => rootRoute, path, component })` shape matching `glyphShowcaseRoute`, and instruct adding `codeSyntaxShowcaseRoute` to `rootRoute.addChildren([...])` alongside the other showcase routes.

2. **Tighten rgba whitespace normalisation in the fixture parity test** (addresses: "fixture parity test will fail on --code-stroke")
   Change `.replace(/\s+/g, ' ')` to `.replace(/\s+/g, '')` (strip all whitespace) on both sides of the comparison. Apply the same normalisation in the drift-detection test for symmetry.

3. **Keep `font-weight: 600` and emphasis colour intact in Phase 4** (addresses: "Phase 4 deletion drops font-weight/color")
   Add `font-weight: 600;` to `.hljs-section` and `.hljs-strong` rules in the shared layer (matches prototype intent). For emphasis, either accept the colour-loss explicitly with a documented rationale or add a muted colour token. Add a regression test in Phase 4 that asserts a `<h2>` rendered through the templates preview has `font-weight: 600`.

4. **Add explicit Phase 1 step: update `migration.test.ts` declared-token Set** (addresses: "declared token set not updated")
   Spread `CODE_SURFACE_TOKENS` and `SYNTAX_TOKENS` keys into the `declared` Set construction at `migration.test.ts:342-352`. Add a matching Success Criteria checkbox under Phase 1.

5. **Add Phase 3 step: bump `AC5_FLOOR`** (addresses: "AC5_FLOOR ratchet bump procedure not specified")
   Phase 3 adds three new `var(--code-*)` references in `MarkdownRenderer.module.css`. Explicit step: "Bump `AC5_FLOOR` from 423 to <observed count after migration> per the documented bump protocol." Add a Success Criteria checkbox.

6. **Add automated assertion that github.css is removed** (addresses: "no automated test asserts github.css is removed")
   Extend an existing test (or add `main.import-hygiene.test.ts`) that reads `main.tsx?raw` and asserts no match for `/highlight\.js\/styles\/github\.css/`.

7. **Resolve hljs-emission preconditions for the Playwright AC2 spec** (addresses: "AC2 mapping-table rows depend on emissions not validated")
   Before Phase 2 implementation begins, run a Vitest probe rendering each fixture string through `rehype-highlight` and record observed class names. Pin the FIXTURES content (especially: add a `<!DOCTYPE html>` line to the HTML fixture; add a template-literal interpolation to the TS fixture; add a `print(x)` line to the python fixture). Add an `expect(locator).toHaveCount.greaterThan(0)` precondition per assertion so missing emissions fail with a clear diagnostic. Drop the `.hljs-meta.doctype` mapping if hljs does not emit the compound class.

8. **Extract a shared `assertSelectorMapsTo` helper with a tightened regex** (addresses: "brittle regex duplicated", "selector-prefix substring matches", "compound .hljs-meta.doctype may produce spurious passes")
   Create `src/styles/__testing__/cssRules.ts` exporting `assertSelectorMapsTo(css, selector, token)`. Tighten the matching: split the rule's selector list on commas, exact-match the trimmed selector. Use from both `code-syntax.test.ts` and `LibraryTemplatesView.test.tsx`.

9. **Strengthen the Phase 4 "no local hljs rules" assertion** (addresses: "inverted assertion is bypass-able")
   Replace the `.previewBody :global(.hljs-` regex with a broader check: scan for any `:global(.hljs-` occurrence in the templates-preview module text. Optionally add a project-wide `migration.test.ts` rule: no `.module.css` file (except `code-syntax.global.css`) contains `:global(.hljs-`.

10. **Decide and commit on `__fixtures__/` directory naming and `--tk-*` prefix** (addresses: "`__fixtures__/` has no precedent", "two-letter --tk-* prefix")
    Either flat-co-locate (`prototype-tokens.fixture.json` next to `tokens.ts`) or commit to `__fixtures__/` and document the convention. Either rename `--tk-*` → `--syntax-*` (consistent with full-word prefix style and self-documenting), or keep `--tk-*` with an explicit comment in both `global.css` and `tokens.ts` defining the abbreviation, and document in ADR-0026.

11. **Decide ADR amendment style and expand the rationale** (addresses: "ADR amendment style left ambiguous", "ADR §3a omits the why-theme-invariant rationale")
    Commit to one style — recommend a new top-level §5 "Code-block surface and syntax palette" (alongside §4 "Two-blue collapse"), leaving §3 housekeeping as a separate note. Expand the §5 draft to record: (a) why theme-invariance is intentional (prototype renders identically against both surfaces — link to the two fullpage screenshots); (b) when to add a new `--tk-*` vs reuse; (c) test-guard requirements; (d) optionally codify "theme-invariant token families" as a token-system convention with explicit eligibility criteria.

12. **Add a Phase 2 source-order test for `.language-diff` overrides** (addresses: "structural CSS test cannot prove cascade")
    For each diff override rule, assert its source-offset in the CSS string is greater than the corresponding general rule's offset. Cheap insurance against a future refactor that flips specificity without changing source order.

13. **Tighten `hexToRgbString` validation and add a `formatRgba` helper** (addresses: "hexToRgbString validation inconsistent", "ad-hoc rgba formatter invites duplication")
    Validate via `/^#[0-9a-f]{6}$/i`; produce diagnostic-rich error messages. Add a small `formatRgba(r,g,b,a)` helper alongside in the same commit so Phase 3's `--code-stroke` Playwright assertions can use a canonical formatter without duplication.

14. **Annotate the six unmapped `--tk-*` tokens** (addresses: "defined-but-unused tokens unmarked", "no architectural retirement path")
    Add an inline comment to the `--tk-*` block in both `global.css` and `tokens.ts` listing the six unmapped tokens and naming the expected future consumer per token (e.g. `--tk-lifet // Rust lifetimes — no current hljs consumer`).

15. **Add file-header comment to `CodeSyntaxShowcase.tsx`** (addresses: "showcase route intent has no durable marker", "showcase route conflates responsibilities")
    Header comment: "Dev-only Playwright fixture surface for `tests/visual-regression/code-block-resolved-colours.spec.ts`. NOT part of the design-system index — see `meta/work/0083` for the documented showcase. Do not link from DevDesignSystem." Reference story 0076 for provenance.

16. **Consider inverting Phase 4's CSS deletion into Phase 2** (addresses: "Phase 2 leaves application in two-colour-system state")
    Move the deletion of `.previewBody :global(.hljs-*)` rules from Phase 4 to Phase 2 (the test inversion and Playwright fixture work stays in Phase 4). Phase 2 ships with the shared layer winning everywhere; Phase 4 is purely test-tightening. Eliminates the transient state without bloating the phase 2 PR.

17. **Parameterise the Playwright theme-repeat block** (addresses: "dark-theme repeat risks copy-paste fan-out")
    Use `test.describe.each([{theme: 'light'}, {theme: 'dark'}])(...)` with identical test bodies. Self-documents the theme-invariance contract and prevents one-sided drift.

18. **Cover screenshot-rebaseline procedure or call out the gap** (addresses: "screenshot re-baseline procedure not pointed to")
    Either link Migration Notes to the actual npm script / README procedure, or explicitly call out the documentation gap as something to fill before Phase 3/4 ships.

Smaller cleanups (suggestions, minor items) can be folded in opportunistically: per-key comments on `SYNTAX_TOKENS`, expanding the EXCEPTION reason text for `1px`, adding `print(x)` and doctype lines to fixtures, etc.

---

## Per-Lens Results

### Architecture

**Summary**: The plan establishes a clean shared-layer architecture for hljs colouring, mirroring the established `wiki-links.global.css` precedent and properly closing the gap left by story 0033's `--code-*`/`--tk-*` omissions. Phase sequencing is well-reasoned — phases 3 and 4 are correctly identified as independent leaves of phase 2, and the test-first ordering with the prototype-drift fixture creates durable architectural pressure against regression. Principal concerns: (a) the theme-invariance decision becomes a new token-family category but isn't formalised in ADR-0026 as a convention; (b) phase 2 intentionally ships a transient two-competing-colour-systems state for the templates preview; (c) the dev-only showcase route mixes 'design review' and 'Playwright fixture' responsibilities.

**Strengths**:
- Layer boundary follows the `wiki-links.global.css` precedent; loose coupling between the layer and its two consumers
- Phase sequencing correctly identifies phase 1 as foundation, phase 2 as activation, phases 3-4 as independent leaves
- The prototype-tokens fixture introduces a durable drift-detection seam with appropriate inward dependency direction
- The split between `CODE_SURFACE_TOKENS` and `SYNTAX_TOKENS` respects natural cohesion lines
- Test-first ordering creates architectural pressure against silent regression

**Findings**: 2 major, 4 minor (see consolidated list above)

### Code Quality

**Summary**: The plan is methodical, test-driven, and aligns with existing conventions. The main risks are duplication of a brittle regex helper across three test files, an under-anchored selector match that produces false positives without raising errors, an unprincipled error contract in `hexToRgbString`, and a hard-coded showcase FIXTURES array that quietly mixes feature data with Playwright-spec-specific spans.

**Strengths**:
- Test-first sequencing with named precedent files
- Reuses the existing `describe.each` parity matrix
- Truncation-guard test extension explicitly called out
- Three-way parity emits one `it()` per token for clean failure localisation
- Hex casing convention is deliberate and well-motivated
- EXCEPTIONS bump documented with `EXCEPTIONS hygiene` named as the safety net

**Findings**: 2 major, 4 minor, 1 suggestion (see consolidated list above)

### Test Coverage

**Summary**: Unusually rigorous on test coverage: every phase opens tests-first, the parity matrix is at the right granularity, the drift-detection fixture closes the prototype↔code feedback loop, and Playwright is correctly chosen as the venue for resolved-colour assertions. Real gaps: the structural test for `code-syntax.global.css` can be satisfied by source-text order alone (not cascade-resolved precedence); the locator strategy depends on hljs class emission only asserted for a couple of tokens; the github.css removal lacks an explicit automated guard; the Phase 4 inverted assertions are easy to bypass.

**Strengths**:
- Tests-first cadence enforced phase-by-phase
- Three-way parity using existing `readCssVar` pattern
- Drift detection against `prototype-standalone.html`
- Resolved-colour assertions correctly delegated to Playwright
- Truncation-guard extension addresses the `readCssVar` flat-block failure mode
- `EXCEPTIONS hygiene` mechanism is leveraged for auto-verification
- Both theme repeats of the Playwright spec are explicitly required

**Findings**: 4 major, 6 minor (see consolidated list above)

### Correctness

**Summary**: Mostly logically sound — token values match the prototype source byte-for-byte, the cascade/specificity reasoning resolves correctly in outcome (even where the stated reasoning is wrong), and the migration test count bookkeeping is verified against the actual module. However: a whitespace-normalisation gap will make the `--code-stroke` parity test fail, a missing `routeTree.addChildren` registration leaves the showcase route unreachable, and silent dropping of font-style/weight pairings affects `.hljs-emphasis`/`.hljs-strong`/`.hljs-section` in the templates preview.

**Strengths**:
- Token values match the prototype source verbatim
- Truncation-guard extension correctly targets the last `--tk-*` token
- The `1px` EXCEPTION count bump (2→3) is correct (existing count verified against the module)
- Cascade reasoning for the Phase 2 transient state is correct
- Line-number citations against existing files are accurate
- github.css load-order analysis is correct

**Findings**: 3 major, 4 minor (see consolidated list above)

### Standards

**Summary**: Generally respects established visualiser conventions — `*.global.css` matches `wiki-links.global.css` precedent, `as const` token families with key-type exports mirror `TYPOGRAPHY_TOKENS`, and the `/code-syntax-showcase` route name follows existing showcase precedent. However it introduces two notable convention departures (`__fixtures__/` subdirectory with no precedent, `--tk-*` 2-letter prefix breaking the established full-word pattern), and omits at least two ratchet/declared-token updates that downstream tests will require. ADR amendment style is a judgement call left unresolved.

**Strengths**:
- Filename and `*.global.css` pattern correctly follow precedent
- `as const` token families and paired type exports mirror existing families
- Dev-only showcase route name and locator contract reuse precedent verbatim
- `:root` lowercase + fixture uppercase split is deliberate and well-justified
- Truncation-guard, MIRROR parity, and EXCEPTIONS hygiene mechanisms accounted for

**Findings**: 4 major, 5 minor (see consolidated list above)

### Documentation

**Summary**: Comprehensive on test specification and code structure but underweights durable code-level documentation. Critical contextual decisions (why the palette is theme-invariant, which six `--tk-*` tokens are defined-but-unused, that `/code-syntax-showcase` is a dev-only test fixture) live only in the plan document; once archived, future contributors editing `tokens.ts`, `global.css`, or the showcase route will lack the rationale. The proposed ADR §3a amendment is correctly placed but skips the "why" and "when to add a new mapping" guidance.

**Strengths**:
- Plan explicitly references the `wiki-links.global.css` convention as precedent
- Each `--tk-*` declaration block carries an inline comment block in `global.css`
- Fixture casing divergence is parenthetically explained
- Plan correctly identifies that ADR-0026 §3 table and Appendix table both need updating
- Test-first ordering means test files serve as behavioural documentation

**Findings**: 3 major, 3 minor, 1 suggestion (see consolidated list above)

---

## Re-Review (Pass 2) — 2026-05-22

**Verdict:** COMMENT

The pass-1 revision addresses **all 18 prior major findings** at the level the plan intends. Critical correctness defects (route registration, rgba whitespace parity, Phase 4 font-weight regression) are fixed; the shared `assertSelectorMapsTo` helper closes the brittle-regex theme at its root and is reused by both consuming test files; ADR-0026 §5 codifies "theme-invariant token families" as a named convention with explicit eligibility criteria; test infrastructure ratchets (`AC5_FLOOR`, declared-token Set, `main.import-hygiene.test.ts`) all have explicit plan steps. The edits introduce a cluster of **new minor findings** centered on the new `src/styles/testing/cssRules.ts` helper (file-header, parser robustness, hardcoded `color:` property, naming, ownership) — none rise to major, but a follow-up tightening pass on the helper would convert a "looks correct" implementation into a robust one.

### Previously Identified Issues

#### Critical
(none in pass 1)

#### Major (18)
- ✅ **Architecture**: Theme-invariant family not formalised — **Resolved** (ADR-0026 §5 with eligibility criteria, operational guidance, References)
- 🟡 **Architecture**: Phase 2 transient two-colour-system state — **Partially resolved** (user chose to keep phases as-is; reviewer notes the trade-off could be framed more explicitly in ADR §5 Consequences)
- ✅ **Correctness**: `--code-stroke` whitespace parity gap — **Resolved** (`canonical()` strips all whitespace symmetrically)
- ✅ **Correctness**: Showcase route never added to `routeTree.addChildren` — **Resolved** (`createRoute(...)` + explicit `addChildren([...])` instruction; verified against `router.ts` shape)
- ✅ **Correctness**: Phase 4 drops `font-weight`/colour on `.hljs-section`/`.hljs-strong`/`.hljs-emphasis` — **Resolved** (font-weight added to shared layer; emphasis colour-loss documented in ADR §5 Consequences; test asserts no `color:` in `.hljs-emphasis` body)
- ✅ **Test Coverage**: Structural CSS test cannot prove `.language-diff` cascade — **Resolved** (paired `selectorOffset` source-order assertions)
- ✅ **Test Coverage**: No automated test asserts github.css removed — **Resolved** (`main.import-hygiene.test.ts` with absence + load-order assertions)
- ✅ **Test Coverage**: AC2 mapping rows depend on unvalidated emissions — **Resolved** (pre-implementation probe, `toBeVisible()` preconditions, FIXTURES enriched, `.hljs-meta.doctype` dropped)
- ✅ **Test Coverage**: Phase 4 inverted assertion bypass-able — **Resolved** (broadened to `:global(.hljs-`)
- ✅ **Code Quality**: Brittle CSS-rule regex duplicated — **Resolved** (shared `cssRules.ts` helper with exact-match contract)
- ✅ **Code Quality**: `hexToRgbString` validation inconsistent — **Resolved** (three-stage validation + `formatRgba`)
- ✅ **Standards**: `__fixtures__/` has no precedent — **Resolved** (`fixtures/`, ADR §5 scope)
- ✅ **Standards**: Two-letter `--tk-*` prefix — **Resolved** (kept per user choice with per-token comments + ADR §5 explicit gloss + `CODE_SYNTAX_TOKENS` rename)
- ✅ **Standards**: `migration.test.ts` declared token Set not updated — **Resolved** (Phase 1 §7 new explicit step)
- ✅ **Standards**: `AC5_FLOOR` ratchet bump unspecified — **Resolved** (Phase 3 §4 explicit step)
- ✅ **Documentation**: ADR §3a omits why-theme-invariant rationale — **Resolved** (ADR-0026 §5 Context/Decision/Why/Eligibility/Operational/Consequences)
- ✅ **Documentation**: Six unmapped tokens not flagged — **Resolved** (per-key comments + block comment + ADR §5 operational guidance — triple-flagged)
- ✅ **Documentation**: Showcase dev-only intent has no durable marker — **Resolved** (file-header comment in `CodeSyntaxShowcase.tsx`)

#### Minor (~26)
All prior minor findings resolved or made suggestion-level. The two that remain as low-impact residue:
- 🟡 **Standards**: `:root` insertion point still anchored to line numbers (suggestion-level — line numbers paired with named-block anchors, low risk)
- 🟡 **Test Coverage**: Phase 4 Playwright templates-preview block kept optional per user choice (acceptable trade-off; Vitest structural is the floor)

### New Issues Introduced

The pass-2 edits introduce a cluster of new minor findings — chiefly around the new `src/styles/testing/cssRules.ts` helper. None are blocking; collectively they would benefit from a brief follow-up tightening pass on that helper before implementation.

#### `cssRules.ts` helper cluster (cross-cutting theme; flagged by 5 of 6 lenses)

- 🔵 **Code Quality**: `parseFlatCssRules` brace scanner is fragile on CSS edge cases — string literals containing `{`/`}`, comments with stray braces. (Latent for current consumers; future-callers risk.)
- 🔵 **Correctness**: `parseFlatCssRules` comment claims `@media` recursion but implementation does not recurse. The comment mis-sells the helper.
- 🔵 **Code Quality**: `assertSelectorMapsTo(css, selector, token)` name implies generality but hardcodes the `color:` property. Future callers expecting `background:`/`border-color:` will be surprised.
- 🔵 **Correctness**: The `color:` regex inside `assertSelectorMapsTo` is not anchored at a property-name boundary, so `border-color:`, `background-color:`, `outline-color:` all satisfy the pattern. Latent for current rules.
- 🔵 **Architecture**: `cssRules.ts` is now a public helper shared across two test domains without a clear ownership boundary or API stability note.
- 🔵 **Standards**: New `src/styles/testing/` subdirectory introduced without ADR-level scope note (parallel to the `fixtures/` charter that ADR §5 does include).
- 🔵 **Documentation**: `cssRules.ts` has no file-header comment explaining purpose, exact-match invariant, or consumers — discoverability flows backwards from imports.

#### Other new minors

- 🔵 **Test Coverage**: `formatRgba` ships with unit tests but has no caller in the plan (no `--code-stroke` resolved-colour assertion is planned). Tests guard dead code.
- 🔵 **Test Coverage**: Source-order assertion mixes `selectorOffset` (open-brace offset) with `indexOf` (literal substring offset) — semantically correct but asymmetric idioms.
- 🔵 **Test Coverage**: Pre-implementation emission probe is a one-off developer ritual, not a committed test. A future hljs-grammar upgrade would force re-doing the probe.
- 🔵 **Correctness**: `main.import-hygiene.test.ts` lands in Phase 3 but the Phase 2 transient state (github.css co-existing with the new layer) has no automated guard on import order.
- 🔵 **Code Quality**: `hexToRgbString` three-stage validation has one redundant check (length-6 is covered by the regex's `{6}` quantifier) — defensible for diagnostic specificity but needs a one-line comment explaining why.
- 🔵 **Architecture**/**Code Quality**: `PREVIOUSLY_LOCAL_MAPPINGS` (Phase 4) duplicates a subset of `REQUIRED_MAPPINGS` (Phase 2); lock-step relationship is comment-only with no mechanical enforcement.
- 🔵 **Code Quality**: `FIXTURES.spans` field is documentation-only — React component never reads it; could be reframed as an enforced contract (assert `spans` ⊆ actual emitted classes).
- 🔵 **Standards**: `main.import-hygiene.test.ts` file-naming pattern (`<subject>.<concern>.test.ts`) is novel in the codebase.
- 🔵 **Standards**: 3:1 contrast deferral still listed only under "What We're NOT Doing"; not cross-referenced from ADR §5 Consequences.
- 🔵 **Documentation suggestion**: Reserved-token comments could name the grammar that would activate each token (e.g. `// reserved — C/Rust macros (hljs-meta in C grammar)`).

### Assessment

The plan is **acceptable as-is for implementation**. The pass-1 revision was substantial and high-quality: 18 majors resolved, no new majors introduced. The remaining concerns are all minor and cluster around a single new artefact (`cssRules.ts`) — the kind of helper that benefits from one more polishing pass once the implementer touches it.

Recommended quick-wins to fold into the implementation work (no plan re-write required):

1. **Add a `cssRules.ts` file-header comment** covering purpose, exact-match invariant, known consumers, ADR/story back-references.
2. **Either rename `assertSelectorMapsTo` → `assertSelectorColorIs`** OR **generalise to take a property parameter**. Pick one to match the actual contract.
3. **Anchor the `color:` regex at a property-name boundary** (`(^|[;{\\s])color\\s*:`) so `border-color:` etc. don't satisfy it.
4. **Either drop `formatRgba` + its tests** (re-introduce when a real caller arrives) OR add a `<pre>` border-colour Playwright assertion that uses it.
5. **Use `selectorOffset` on both sides** of the source-order assertion for symmetry.
6. **Add a defensive guard** in `parseFlatCssRules` that throws if the css contains an `@` at top level (advertising the constraint matches reality).
7. **Trim the misleading "recurses into braces" comment** in `parseFlatCssRules` — replace with "does NOT support @media or nested at-rule blocks".
8. **Add a one-paragraph note to ADR §5** scoping `src/styles/testing/` alongside the `fixtures/` charter.

None of these block landing the plan. The plan can move to implementation; the implementer can address the cluster while building `cssRules.ts`.

---

## Pass 3 — 2026-05-22 (Quick-wins folded; plan accepted)

**Verdict:** APPROVE

All 8 quick-wins from the pass-2 re-review were folded into the plan
without further review rounds:

1. ✅ `cssRules.ts` file-header comment added (scope, exact-match
   invariant, consumers, ADR back-reference)
2. ✅ `assertSelectorMapsTo` renamed to `assertSelectorColorIs` so
   the name matches the actual contract (colour-only)
3. ✅ Colour regex anchored at property-name boundary
   `(?:^|[;{\s])color\s*:` — `border-color:`/`background-color:`/etc.
   no longer satisfy it
4. ✅ `<pre>` border-color Playwright assertion added (deferred to
   Phase 3 alongside the border declaration) — `formatRgba` now has
   a real consumer
5. ✅ Source-order assertions in `code-syntax.test.ts` use
   `selectorOffset` on both sides for symmetric measurement
6. ✅ `parseFlatCssRules` throws with a diagnostic if input contains
   `@media`/`@supports`/`@container`/`@layer` at top level
7. ✅ Misleading "recurses into braces" comment trimmed — replaced
   with truthful "does NOT recurse" and consumer caveats
8. ✅ ADR-0026 §5 gains a `src/styles/testing/` scope paragraph
   alongside the existing `fixtures/` paragraph

The plan is **accepted** and ready for implementation. Plan
frontmatter updated: `status: accepted`,
`review: meta/reviews/plans/...-review-1.md`.

---
*Review generated by /review-plan*
