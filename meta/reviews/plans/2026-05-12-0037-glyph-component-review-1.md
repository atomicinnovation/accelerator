---
date: "2026-05-12T12:30:00Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-12-0037-glyph-component.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability, documentation, performance]
review_pass: 3
status: complete
---

## Plan Review: Glyph Component Implementation Plan

**Verdict:** REVISE

The plan is well-structured, fits established codebase conventions (Brand, PipelineDots, tokens.spec.ts), and makes principled choices about TDD ordering, statelessness, and theme-cascade-driven rendering. However, eight reviewers surface 14 major findings clustering around four cross-cutting themes that should be resolved before implementation: runtime/compile-time safety of the `docType` contract, test coverage that doesn't actually verify the work item's ACs end-to-end, convention drift (notably `accessibleLabel` vs the codebase's established `ariaLabel`), and downstream-consumer ergonomics for the 8 work items that will integrate Glyph. None are critical, but together they amount to material gaps in a plan that is otherwise unusually thorough.

### Cross-Cutting Themes

- **Runtime safety of `docType` is unguarded** (flagged by: code-quality, correctness, usability) — `GlyphDocTypeKey = Exclude<DocTypeKey, 'templates'>` is hardcoded against one literal; a future virtual key silently joins Glyph's contract. `ICON_COMPONENTS[docType]` returns `undefined` for out-of-band strings (URL params, server JSON, casts) producing an opaque "Icon is not a function" deep in render. The runtime filter `DOC_TYPE_KEYS.filter(k => k !== 'templates')` mirrors the same hardcoded exclusion in two places, and no helper (`isGlyphDocTypeKey`, `GLYPH_DOC_TYPE_KEYS`) is provided for 8 downstream consumers to narrow.
- **Test coverage doesn't verify the work item's ACs** (flagged by: test-coverage, code-quality) — `@ts-expect-error` directives are not run by `npm test` (no `typecheck` script exists), so the type-contract tests are silently inert under CI's default gate. AC #4's resolved-hex check is moved to Playwright but Playwright captures pixels not computed style, so the resolved-hex resolution is functionally untested. Phase 3's 12 hand-derived icons have no automated visual gate before Phase 5 baselines lock — a mistraced icon becomes ground truth. `maxDiffPixelRatio: 0.05` over a 36-glyph grid permits ~39,000 pixels of drift (≈ 4 fully-wrong icons).
- **Theme/cascade invariants are fragile** (flagged by: code-quality, correctness, architecture) — `fill` on outer `<svg>` relies on inheritance into 12 separately-authored icon files; any child path overriding `fill` silently breaks the contract. `viewBox="0 0 24 24"` is hardcoded but the canonical screenshot's source grid isn't measured. One `requestAnimationFrame` for theme swap may not settle the cascade for 36 SVGs on slower CI runners.
- **Convention drift across 8 downstream consumers** (flagged by: standards, usability, documentation) — `accessibleLabel` diverges from the codebase's `ariaLabel` (used by `TopbarIconButton`, `ThemeToggle`, `FontModeToggle`). No `isGlyphDocTypeKey`/`GLYPH_DOC_TYPE_KEYS` helpers means 8 consumers reinvent narrowing. Showcase discoverability hinges on a brand-new README with no in-source pointer. Consumer contract (don't override `fill`, provide adjacent label or `accessibleLabel`, no nested `<svg>`) is undocumented.

### Tradeoff Analysis

- **Test rigour vs. ship velocity**: tightening `maxDiffPixelRatio` to 0.005 or splitting into 36 per-cell clips would close real coverage gaps but at the cost of test runtime and baseline-maintenance friction. Per-cell clips are the strongest fix; if velocity wins, document explicitly that the showcase screenshot is a structural smoke check, not per-icon regression.
- **Runtime guards vs. minimalism**: adding a dev-only `console.warn` for unknown `docType` improves diagnosability at the cost of a runtime branch and a small departure from the "Glyph is pure dispatch" framing. Worth doing given 8 downstream surfaces, several of which read `DocTypeKey` from untyped sources.
- **Convention bind-up vs. work-item literal**: the work item AC text uses `accessibleLabel`; the codebase uses `ariaLabel`. The plan inherits the AC text without flagging the divergence. Resolving requires either an AC update or an explicit deliberate-departure note — either is fine, but silence creates drift.

### Findings

#### Critical
*(none)*

#### Major

- 🟡 **Code Quality**: Inherited-fill design is fragile to any icon overriding `fill` on a child
  **Location**: Phase 2, Section 4 — Glyph dispatch + accessibility
  Glyph sets `fill="var(--ac-doc-${docType})"` on the outer `<svg>` and relies on 12 separately-authored icon files never setting `fill` on any child path. A copy-pasted `fill="#XXXXXX"` from a design source silently breaks the theme contract; no Vitest assertion catches it.

- 🟡 **Code Quality**: No runtime guard for invalid `docType` from untyped data sources
  **Location**: Phase 2, Section 4
  `ICON_COMPONENTS[docType]` returns `undefined` for out-of-band strings (URL params, server JSON, casts), producing "Icon is not a function" deep in render. `isDocTypeKey` already exists in `types.ts:22-24`.

- 🟡 **Test Coverage**: `@ts-expect-error` directives are not run by `npm test`
  **Location**: Phase 2 Success Criteria — Automated Verification
  No `typecheck` script exists in `package.json`; `tsc -b` runs only under `build`. The type-contract tests (AC #7, AC #8) are silently inert under the default CI gate.

- 🟡 **Test Coverage**: AC #4's resolved-hex check has no test that actually verifies it
  **Location**: Phase 4 + Acceptance Criteria #4
  Plan moves resolved-hex check to Playwright, but `toHaveScreenshot` captures pixels, not `getComputedStyle`. Nothing asserts `getComputedStyle(svg).fill === '<expected hex>'` in any environment.

- 🟡 **Test Coverage**: 36-case matrix is largely redundant — mutation-testing yield is low
  **Location**: Phase 4
  Every case produces identical attribute shape modulo string interpolation. Case 35 (`design-inventories × 32`) catches nothing case 1 (`decisions × 16`) doesn't. The real coverage dimension — per-icon presence in `ICON_COMPONENTS` — is not directly asserted.

- 🟡 **Test Coverage**: Phase 3 has no automated visual test; manual review is the only gate before Phase 5 baselines lock
  **Location**: Phase 3
  A mistraced icon (e.g. `plan-reviews` looking like `research`) gets codified as the Phase 5 baseline silently.

- 🟡 **Test Coverage**: 5% pixel tolerance hides per-icon regressions in a 36-glyph grid
  **Location**: Phase 5 — Playwright `maxDiffPixelRatio: 0.05`
  1024×768 × 0.05 ≈ 39,300 pixels of permitted drift; a 24×24 icon is 576 pixels. Up to ~4 fully-wrong icons stay under budget.

- 🟡 **Correctness**: `Exclude<DocTypeKey, 'templates'>` invariant is not compile-enforced
  **Location**: Phase 2, Section 2
  `DOC_TYPES` carries a `virtual: boolean` flag (`types.ts:35`) but the type alias hardcodes the literal `'templates'`. A future virtual key silently joins Glyph's contract.

- 🟡 **Correctness**: Hardcoded `viewBox="0 0 24 24"` not verified against canonical screenshot grid
  **Location**: Phase 2, Section 4 / Phase 3
  Plan asserts "matches the canonical screenshot grid" without measuring. If the prototype source grid is 16/20px, every Phase-3 path coordinate is drawn against a misaligned grid.

- 🟡 **Standards**: `accessibleLabel` prop diverges from established `ariaLabel` convention
  **Location**: Phase 2, Section 4 — GlyphProps
  `TopbarIconButton.tsx:6` defines `ariaLabel: string`; `ThemeToggle` and `FontModeToggle` pass `ariaLabel=...`. The work item text uses `accessibleLabel`, but the codebase precedent is `ariaLabel`.

- 🟡 **Standards**: WCAG 1.4.11 (non-text contrast) of eyedropped hex values is not verified against `--ac-bg`
  **Location**: Phase 1, Section 1
  Light-theme designer-eyedroppered fills (yellows, light greens) are at risk of failing 3:1 against `#fbfcfe`. No verification step is proposed; the contrast gap would be inherited by all 8 downstream consumers.

- 🟡 **Usability**: `DocTypeKey → GlyphDocTypeKey` narrowing has no provided helper
  **Location**: Phase 2 + Desired End State
  No `isGlyphDocTypeKey()` guard nor `GLYPH_DOC_TYPE_KEYS` array exported. 8 downstream consumers must each reinvent `.filter(k => k !== 'templates')` or fall back to `as` casts.

- 🟡 **Usability**: Discrete `size: 16 | 24 | 32` will force escape hatches for off-grid sizes
  **Location**: GlyphProps
  At least one downstream consumer is likely to want 20px (avatar context) and will either cast `as 24`, wrap in `transform: scale()`, or request widening mid-stream. The "What We're NOT Doing" section doesn't justify the strictness or document the resolution path.

- 🟡 **Performance**: Inline-SVG rendering cost in high-cardinality consumer pages not analysed
  **Location**: Performance Considerations
  Plan dismisses cost with "36 SVGs … negligible," but kanban (0040), activity feed (0055), search results (0054) may render hundreds of Glyphs per page, each inlining full path strings. Sprite-sheet (`<symbol>`+`<use href>`) tradeoff not considered.

#### Minor

- 🔵 **Architecture**: Static dispatch `Record` defeats tree-shaking — tradeoff reasonable but should be acknowledged in plan (Phase 2)
- 🔵 **Architecture**: `GlyphDocTypeKey` co-located in component module but is a domain type 8 downstream WIs import — consider moving to `api/types.ts` (Phase 2.2)
- 🔵 **Architecture**: Top-level `/glyph-showcase` route — consider `/dev/*` namespace before more showcases land (Phase 5.3)
- 🔵 **Architecture**: Three-CSS-block manual write seam — invariant is correctly enforced at test-time; flag as architectural debt for future codegen (Phase 1.3)
- 🔵 **Architecture**: Per-file icon packaging vs single `icons.tsx` — chosen packaging is one of two reasonable options; record rationale (Phase 2.3)
- 🔵 **Architecture**: String-typed fill seam couples 3 artefacts; typed `FILL_VAR` map would let TS catch template-literal drift (Phase 2.4)
- 🔵 **Code Quality**: Empty `Glyph.module.css` "for convention" is a smell — omit until needed (Phase 2.5)
- 🔵 **Code Quality**: Phase 2 ships a tested-but-functionally-useless component (12 identical placeholder rects) — consider folding Phase 3 into Phase 2 or marking placeholders unshippable
- 🔵 **Code Quality**: `Record<GlyphDocTypeKey, () => React.ReactElement>` is unnecessarily narrow; `React.ComponentType` preserves exhaustiveness while permitting future shape changes (Phase 2.4)
- 🔵 **Code Quality**: "No useState/useEffect" structural guarantee is unenforced; a future hook addition won't fail any test (Phase 4)
- 🔵 **Correctness**: `accessibleLabel=""` empty-string boundary is ambiguous — branch on `!== undefined` instead of truthiness (Phase 2.4)
- 🔵 **Correctness**: Eyedropper procedure underspecifies zoom/sampling — single-pixel sample may land on anti-aliased edge or transparent (Phase 1.1)
- 🔵 **Correctness**: One `requestAnimationFrame` may not settle cascade for 36 SVGs on slower CI — use `waitForFunction` (Phase 5.4)
- 🔵 **Correctness**: Runtime filter mirrors hardcoded `templates` exclusion in two places — export `GLYPH_DOC_TYPE_KEYS` once (Phase 5.1)
- 🔵 **Test Coverage**: Migration test listed as Phase 1 gate but is vacuously true — remove from criteria
- 🔵 **Test Coverage**: A11y assertions cover attribute strings only, not consumed behaviour — use Testing Library `getByRole('img', { name })` (Phase 2)
- 🔵 **Test Coverage**: No negative test catches dangling `--ac-doc-*` token for removed doc type — add bijection invariant
- 🔵 **Test Coverage**: Linux baseline captured by CI from first run accepts whatever Linux renders silently (Phase 5.5)
- 🔵 **Test Coverage**: No test verifies Glyph composes correctly when parent sets `fill`/`color` — first downstream consumer will re-derive
- 🔵 **Standards**: `icons/` subdirectory extends component-folder convention without explicit note (Phase 2.3)
- 🔵 **Standards**: Forced-colors deferral lacks explicit accessibility-standard reference (Out-of-Scope)
- 🔵 **Standards**: New frontend README sets first-precedent without aligning to any existing convention (Phase 6)
- 🔵 **Usability**: `/glyph-showcase` discoverability relies on a brand-new README — add pointer in `Glyph.tsx` or `tokens.ts` (Phase 6)
- 🔵 **Usability**: Showcase shows raw kebab keys, only one theme at a time — add friendly labels + side-by-side themes (Phase 5.1)
- 🔵 **Usability**: Long import path repeated across 8 consumers, no barrel — accept convention or evaluate path alias (Desired End State)
- 🔵 **Usability**: No dev-only console warning for unknown `docType` — opaque "Icon is not a function" at 11pm (Phase 2.4)
- 🔵 **Documentation**: `maxDiffPixelRatio: 0.05` user-decision citation is ephemeral — point at work-item Resolved Decisions instead (Phase 5.4)
- 🔵 **Documentation**: Frontend README omits prerequisites, project structure, troubleshooting (Phase 6)
- 🔵 **Documentation**: Docker path for Linux baselines is hand-waved — no image, no command (Phase 5.5)
- 🔵 **Performance**: Bundle-size claim asserted without measurement (Performance Considerations)
- 🔵 **Performance**: Per-render `a11y` object allocation in long lists — module-scope constant for `aria-hidden` branch if profiling shows pressure (Phase 2.4)
- 🔵 **Performance**: Visual-regression baseline footprint adds ~3MB to repo per Glyph snapshot set — spot-check after Phase 5 (Phase 5.5)

#### Suggestions

- 🔵 **Code Quality**: Naming/casing helper should be DRY-centralised (Phase 2.3)
- 🔵 **Code Quality**: JSDOM-vs-Playwright assertion split rationale should be in the test file as a comment, not just in the plan (Phase 4)
- 🔵 **Correctness**: `var(--ac-doc-${docType})` interpolation safety enforced only by convention — assert key shape (Phase 2.4)
- 🔵 **Correctness**: Route order in `addChildren` is not significant here; record that briefly to avoid future confusion (Phase 5.3)
- 🔵 **Standards**: GlyphShowcase uses `<table>` for a presentational grid — add `<caption>` or switch to CSS grid (Phase 5.1)
- 🔵 **Usability**: Consider whether `docType` is clearest prop name — kept, but worth justifying explicitly (Phase 2.4)
- 🔵 **Documentation**: viewBox and fill-interpolation choices lack inline rationale comparable to the `GlyphDocTypeKey` INVARIANT block (Phase 2.4)
- 🔵 **Documentation**: Plan doesn't define Consumer Contract that 8 downstream WIs will rely on (Overview)
- 🔵 **Documentation**: Line-number references will drift — consider symbolic anchors or a fixed-commit-hash reference (References)
- 🔵 **Documentation**: No place to record eyedropper `(x, y)` coordinates beyond commit message body (Phase 1.1)
- 🔵 **Performance**: Per-icon manual eyedropper is a developer-time cost — small script could compress the step (Phase 1.1)

### Strengths

- ✅ Strict scope boundary: consumer integration delegated to 8 downstream WIs, keeping Glyph's responsibility cohesive and the work item independently shippable
- ✅ Theme-swap is architecturally pushed entirely into the CSS cascade — Glyph holds no React state, no `useEffect`, no theme context; AC #4 "no React render occurred" is guaranteed by construction
- ✅ Token-parity tests are driven by iterating frozen exports, so adding a token automatically extends test coverage rather than requiring synchronized test edits
- ✅ MIRROR-A↔MIRROR-B byte-equivalence is enforced at unit-test time rather than visual-regression time — a strong choice for cheap, deterministic coverage
- ✅ Glyph owns the outer `<svg>` wrapper (size, viewBox, fill, a11y) and delegates only inner geometry to per-doc-type components — clean separation between presentation contract and shape data
- ✅ Vitest/Playwright split (attribute literal vs resolved hex) is a principled response to a JSDOM limitation rather than a workaround, and is explicitly documented with reasoning
- ✅ Phase-per-commit structure aligns the deployment boundary with the architectural boundary (tokens, contract, geometry, tests, route, docs)
- ✅ Decorative-by-default a11y (`aria-hidden="true"`) means the common case requires zero props beyond `docType` and `size` — strong progressive disclosure
- ✅ Component file layout matches every existing component (Brand, OriginPill, PipelineDots, TopbarIconButton); no barrel, named export
- ✅ Token naming consistently follows the established kebab-case `--ac-*` convention and added to existing flat-record exports
- ✅ SVG token usage `fill="var(--ac-*)"` follows the exact pattern proven by `Brand.tsx`; respects ADR-0026 by not introducing `color-mix()` for foreground fills
- ✅ Routing follows the closest precedent (`kanbanRoute`): top-level `createRoute` for a non-crumbed developer-only route
- ✅ Playwright spec faithfully mirrors `tests/visual-regression/tokens.spec.ts` shape
- ✅ WCAG 1.4.1 (use of colour) is addressed structurally: 12 distinct shapes per doc type mean colour is not the sole carrier of meaning
- ✅ Plan is exceptionally well-documented: Current State Analysis cites exact file paths and line ranges; INVARIANT comment block on `GlyphDocTypeKey` is exemplary; What We're NOT Doing pairs each exclusion with rationale

### Recommended Changes

1. **Resolve the `accessibleLabel` vs `ariaLabel` divergence** (addresses: Standards major #1, Documentation suggestion on consumer contract)
   Either rename to `ariaLabel` for codebase consistency, or document the deliberate departure in `Glyph.tsx` TSDoc. Update the work item AC text in lock-step. Branch on `accessibleLabel !== undefined` not truthiness (addresses Correctness minor on empty-string boundary).

2. **Close the type-contract and resolved-hex AC gaps** (addresses: Test Coverage major #1, #2)
   Add an explicit `"typecheck": "tsc --noEmit"` script and reference it in Success Criteria + CI so `@ts-expect-error` directives are actually run. Add one Playwright test (not screenshot) that asserts `getComputedStyle(svg).fill` resolves to the canonical hex for one Glyph per theme — closes AC #4's resolved-hex leg properly.

3. **Add runtime safety + narrowing helpers** (addresses: Code Quality major #2, Correctness major on `Exclude` invariant, Usability major on narrowing)
   Export `GLYPH_DOC_TYPE_KEYS` and `isGlyphDocTypeKey()` from `Glyph.tsx` (or `api/types.ts`). Add a dev-only `console.warn` + null-return for unknown `docType` in Glyph. Derive `GlyphDocTypeKey` from the `virtual: boolean` discriminant via a unit assertion that catches new virtual keys.

4. **Verify viewBox grid and tighten visual-regression** (addresses: Correctness major on viewBox, Test Coverage major #4 on Phase 3 fidelity, Test Coverage major #5 on 5% tolerance)
   Add a Phase 1/3 step to measure the canonical screenshot glyph grid and confirm 24×24. Either tighten `maxDiffPixelRatio` for the glyph showcase to 0.005, or split into 36 per-cell `toHaveScreenshot` clips. Either way, document the rationale in the spec file.

5. **Guard the inherited-fill invariant** (addresses: Code Quality major #1)
   Either add a Vitest assertion that walks every child element of the rendered SVG and asserts none carries a `fill`, or switch to `fill="currentColor"` on children with `color: var(--ac-doc-<key>)` on the `<svg>` so an override fails loudly.

6. **Verify WCAG 1.4.11 contrast against `--ac-bg`** (addresses: Standards major #2)
   Add a Phase 1 verification step computing 3:1 contrast for each sampled hex against `--ac-bg` in both themes. Capture failures and remap with designer sign-off, or document an SC 1.4.11 exception with rationale.

7. **Address inline-SVG cost for high-cardinality consumers** (addresses: Performance major)
   Add a numbered analysis to Performance Considerations: estimate path-string bytes × max instance count for the heaviest consumer (kanban/activity feed). If retaining inline-per-instance, document the threshold at which downstream consumers should switch to sprite reuse, so 0040/0055 reviewers know what to watch for.

8. **Document the Consumer Contract** (addresses: Documentation suggestion, Usability major on size strictness, multiple downstream consumer concerns)
   Add a short "Consumer Contract" subsection to the Overview enumerating: don't override `fill`, provide adjacent text label OR pass `ariaLabel`, no nested `<svg>` wrappers, no off-grid sizes (and the resolution path for new requests). 8 downstream WIs will cite this rather than re-derive.

9. **Fix Phase 2 placeholder commit semantics** (addresses: Code Quality minor on tested-but-useless phase)
   Either fold Phase 3 into Phase 2 (one "icons land with real geometry" commit), or do not advertise the Phase 2 commit as shippable in the Phase 2 section.

10. **Tighten miscellaneous documentation citations** (addresses: Documentation minors)
    Replace "locked in by user decision above" with an explicit reference to the work-item Resolved Decisions section. Either document a concrete Docker command for Linux baselines, or relax the success criterion to allow CI capture. Add prerequisites, project structure, and troubleshooting subsections to the new README, or scope it explicitly.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan slots cleanly into the visualiser's established architectural patterns — token layer, CSS cascade-driven theming, co-located component conventions, TanStack Router precedent — and explicitly defers consumer integration to eight downstream work items, which is correct scoping. The most consequential architectural decision is the per-doc-type icon component + `Record<GlyphDocTypeKey, ()=>ReactElement>` dispatch shape: it gives the system clean per-icon diff isolation but builds a hard-bundled coupling between Glyph and all 12 icons that defeats tree-shaking, and is plausibly the right tradeoff given the showcase requirement but should be explicitly acknowledged. Secondary concerns are the placement of `GlyphDocTypeKey` (co-located rather than alongside `DocTypeKey` in `api/types.ts`) and the fragility of the three-CSS-block manual edit seam.

**Findings**: 6 minor (static dispatch tree-shaking, type co-location, route namespace, three-block seam, per-file packaging, string-typed fill seam)

### Code Quality

**Summary**: The plan is well-structured, leans on established codebase conventions (Brand/PipelineDots), and keeps Glyph mercifully stateless. From a code-quality lens, the main maintainability concerns are: an inherited-fill design that is fragile if any icon child overrides fill; an empty CSS module shipped 'for convention'; runtime invalidity of docType not explicitly handled; and a Phase 2/Phase 3 split that creates a tested-but-functionally-useless window.

**Findings**: 2 major, 4 minor, 2 suggestions

### Test Coverage

**Summary**: The plan establishes a reasonable multi-layered test strategy but several gaps undermine the confidence it claims: the 36-combination matrix degenerates into a redundant smoke test; the resolved-hex `fill` assertion has been silently lost (Playwright captures pixels, not computed style); Phase 3 lands 12 hand-drawn icons with zero automated visual regression between Phase 3 and Phase 5 baseline capture; and `@ts-expect-error` directives are not executed by `npm test` (no `typecheck` script exists, only `build`), so the type contract test may not be run by the default CI gate.

**Findings**: 5 major, 6 minor

### Correctness

**Summary**: The plan is largely sound — it leverages existing parity machinery, builds on a proven token-in-attribute precedent (Brand.tsx), and structurally guarantees the no-state-change AC by holding no React state. However, several correctness concerns merit attention: the `Exclude<DocTypeKey, 'templates'>` invariant is documented but not compile-enforced; the `viewBox="0 0 24 24"` assumption is asserted without verifying the canonical screenshot's source grid; the `accessibleLabel=""` empty-string boundary is unspecified; and Phase 3 eyedropping omits zoom level which materially affects anti-aliased hex values.

**Findings**: 2 major, 5 minor, 1 suggestion

### Standards

**Summary**: The plan is strongly aligned with the visualiser frontend's documented and de-facto conventions, but a few standards concerns warrant attention: the `accessibleLabel` prop name diverges from the established `ariaLabel` convention used by `TopbarIconButton`, the plan elides WCAG 1.4.11 (non-text contrast) verification of the eyedropped hex values, and the addition of an `icons/` subdirectory is a novel structural extension that the plan adopts without explicitly noting it as a departure.

**Findings**: 2 major, 3 minor, 1 suggestion

### Usability

**Summary**: The plan describes a clean, idiomatic React component API with sensible defaults that fits established codebase patterns. However, several DX friction points affect the 8 downstream consumer work items that will import Glyph: the long import path, the `GlyphDocTypeKey`/`DocTypeKey` split forces consumers to narrow without a provided helper, the discrete `size: 16 | 24 | 32` union is strict by AC and may push consumers to escape-hatch casts, and `/glyph-showcase` discoverability hinges on a brand-new README with no in-app entry point.

**Findings**: 2 major, 4 minor, 1 suggestion

### Documentation

**Summary**: The plan is unusually well-documented for an implementation plan. The most significant documentation gaps concern downstream traceability: a user-recorded decision (maxDiffPixelRatio: 0.05) is asserted as authoritative without a stable citation, AC interpretation deviations are recorded in the plan but the durability of that record is uncertain, and the new frontend README is thin enough to be usable but lacks several conventional sections.

**Findings**: 3 minor, 4 suggestions

### Performance

**Summary**: The plan addresses performance briefly in a dedicated section, but its claims are largely qualitative and not grounded in numbers. Several scaling concerns specific to downstream consumers (kanban cards, lifecycle pages, activity feed — potentially dozens to hundreds of Glyphs per page) are not analysed: inline-SVG DOM weight, repeated path inlining vs sprite reuse, and per-render allocation in list contexts.

**Findings**: 1 major, 3 minor, 1 suggestion

## Re-Review (Pass 2) — 2026-05-12

**Verdict:** REVISE

The revisions resolve the bulk of pass-1 findings — 30 of 42 prior findings are now Resolved, 7 are deliberately Still-present-by-design (co-location, top-level route, three-block CSS seam, line-number refs, etc.), and the rest are Partially resolved. However, the revisions introduce a small cluster of NEW issues — most of them surgical mismatches between the plan's snippets and the live codebase — that block clean implementation. The most consequential is that several Phase 1/2 code blocks reference modules and exports that don't exist (the contrast helper, the `DOC_TYPES` static array), so an implementer following the plan literally will hit import errors before any tests can run.

### Previously Identified Issues

**Architecture (all 6 — minors)**
- 🔵 Static dispatch defeats tree-shaking — **Resolved** (acknowledged in Bundle size sub-section with measurement action)
- 🔵 GlyphDocTypeKey co-located — **Still present** (deliberately accepted)
- 🔵 Top-level `/glyph-showcase` route — **Still present** (deferred to second showcase)
- 🔵 Three-CSS-block seam — **Still present** (architectural debt; acknowledged)
- 🔵 Per-file icon packaging — **Resolved** (rationale recorded in Key Discoveries)
- 🔵 String-typed fill seam — **Resolved in new form** (`color`/`currentColor` split + no-hex-child assertion)

**Code Quality (2 major, 4 minor, 2 suggestion)**
- 🟡 Inherited-fill fragility — **Resolved** (currentColor pattern + no-hex assertion)
- 🟡 No runtime guard — **Resolved in shape** (warn + null in dev; see new finding on env detection)
- 🔵 Empty CSS module — **Resolved** (file no longer planned)
- 🔵 Phase 2 useless-component window — **Resolved** (Phase 3 folded in)
- 🔵 Record type too narrow — **Resolved** (`React.ComponentType`)
- 🔵 No-hook unenforced — **Resolved** (source-grep test)
- 🔵 Naming/casing helper not DRY — **Still present** (acceptable at N=12)
- 🔵 JSDOM/Playwright rationale in plan only — **Resolved** (now scheduled to land in test-file comment)

**Test Coverage (5 major, 6 minor)**
- 🟡 `@ts-expect-error` not run — **Resolved** (new `typecheck` script)
- 🟡 AC #4 resolved-hex unverified — **Resolved** (new `glyph-resolved-fill.spec.ts`)
- 🟡 36-case matrix redundant — **Still present** (kept for AC traceability)
- 🟡 No Phase 3 visual gate — **Partially resolved** (manual review by non-implementer required; still no automated backstop)
- 🟡 5% tolerance too loose — **Resolved** (per-cell clips; but see new minor on 16 px cells)
- 🔵 Migration test vacuous — **Resolved** (removed from criteria)
- 🔵 No-hook unenforced — **Resolved**
- 🔵 No parent-override test — **Still present**
- 🔵 A11y attribute strings — **Resolved** (Testing Library role-based)
- 🔵 Token bijection — **Still present** (orphan-token check not added)
- 🔵 Linux baseline silent — **Partially resolved** (Docker command provided; CI fallback still implicit-accept)

**Correctness (2 major, 5 minor, 1 suggestion)**
- 🟡 Exclude invariant not compile-enforced — **Resolved** (derived from `virtual` discriminant + exhaustiveness assertion)
- 🟡 viewBox 24×24 unverified — **Resolved** (Phase 1 measurement step)
- 🔵 `accessibleLabel=""` boundary — **Resolved** (`!== undefined`; empty-string test case)
- 🔵 Eyedropper zoom/sampling — **Resolved** (alpha-check + opaque-region rule)
- 🔵 Single rAF insufficient — **Resolved** (`page.waitForFunction`)
- 🔵 Filter duplication — **Resolved** (single `GLYPH_DOC_TYPE_KEYS` export)
- 🔵 Template literal safety — **Still present** (acceptable; convention-enforced)
- 🔵 Route order — **Resolved** (inline comment in router snippet)

**Standards (2 major, 3 minor, 1 suggestion)**
- 🟡 `accessibleLabel` vs `ariaLabel` — **Resolved**
- 🟡 WCAG 1.4.11 contrast — **Resolved** (Phase 1 contrast block — but see new finding on import)
- 🔵 `icons/` subdirectory — **Still present** (no explicit sanctioned-exception note)
- 🔵 Forced-colors deferral reference — **Resolved** (cites SC 1.4.1 + 1.4.11)
- 🔵 README precedent — **Partially resolved** (sections added; cross-workspace alignment not checked)
- 🔵 `<table>` in showcase — **Resolved** (CSS grid)

**Usability (2 major, 4 minor, 1 suggestion)**
- 🟡 Narrowing helper — **Resolved** (`GLYPH_DOC_TYPE_KEYS` + `isGlyphDocTypeKey` exported)
- 🟡 Size strictness — **Partially resolved** (resolution path documented; not surfaced at TS error site)
- 🔵 Showcase discoverability — **Resolved** (in-source pointer comment in `Glyph.tsx`)
- 🔵 Friendly labels / side-by-side themes — **Partially resolved** (labels added; themes still toggle-only)
- 🔵 Long import path — **Still present** (convention-accepted)
- 🔵 Dev console warning — **Resolved**
- 🔵 `docType` prop name — **Not addressed** (acceptable as-is)

**Documentation (3 minor, 4 suggestion)**
- 🔵 maxDiffPixelRatio citation — **Resolved** (points at work-item Resolved Decisions)
- 🔵 README sections — **Resolved** (Prerequisites/Layout/Troubleshooting added)
- 🔵 Docker path — **Resolved** (concrete command in plan + README)
- 🔵 viewBox/fill rationale inline — **Resolved**
- 🔵 Consumer Contract — **Resolved in plan** (but see new finding on work-item mirror)
- 🔵 Line-number references — **Still present**
- 🔵 Eyedropper (x, y) coords — **Partially resolved** (column scheduled; work-item table not pre-restructured)

**Performance (1 major, 3 minor, 1 suggestion)**
- 🟡 Inline-SVG cost in high-cardinality consumers — **Resolved** (50/200 thresholds + sprite-sheet escape valve)
- 🔵 Bundle size unmeasured — **Resolved** (estimate + measurement action + > 15 KB trip-wire)
- 🔵 Per-render `a11y` allocation — **Resolved** (acknowledged with deferral rationale)
- 🔵 VR baseline footprint — **Resolved** (recomputed 150-450 KB; oxipng follow-up)
- 🔵 Manual eyedropper — **Still present** (acceptable for one-off)

### New Issues Introduced

**Major (block clean first-run implementation)**

- 🟡 **Code Quality / Test Coverage / Correctness** (one issue, flagged by 3 lenses): Phase 1 contrast snippet imports `{ contrast } from '../utils/contrast'`. The actual helper is `contrastRatio` at `src/styles/contrast.ts`. The snippet won't compile; an implementer following the plan literally hits an import error before any contrast assertion runs.
- 🟡 **Test Coverage**: `GLYPH_DOC_TYPE_KEYS` derivation and `GlyphShowcase` label lookup both depend on `DOC_TYPES.find(t => t.key === k)?.virtual` / `?.label`, but `src/api/types.ts` does not export a static `DOC_TYPES` array — `DocType` records come from the server at runtime. The exhaustiveness assertion and showcase label rendering as specified cannot work. Plan needs a Phase 0 step to create a static `DOC_TYPES` (or `VIRTUAL_DOC_TYPE_KEYS`) export, or to fall back to hardcoded `templates` exclusion (defeating the virtual-discriminant fix).
- 🟡 **Code Quality**: Runtime guard uses `if (process.env.NODE_ENV !== 'production')`. The codebase's Vite convention is `if (import.meta.env.DEV)` (e.g. `Breadcrumbs.tsx`). Pattern drift in the canonical Glyph example.
- 🟡 **Test Coverage**: "Every child element under the `<svg>` has `fill='currentColor'`" assertion algorithm is unspecified. `svg.children` is direct-only; nested `<g>` groupings would skip child paths with hex literals. Must use `svg.querySelectorAll('*')`.
- 🟡 **Usability**: Consumer Contract lives only in the plan (`meta/plans/...`). Downstream WIs 0036/0040/0041/0042/0043/0053/0054/0055 plan against the work item, not the plan. Invariants will be effectively lost to downstream authors.
- 🟡 **Performance**: Per-cell Playwright spec runs 72 tests per platform vs prior 2 — roughly 36× runtime increase. CI-duration impact not estimated; parallelism strategy unspecified.

**Minor**

- 🔵 JSDOM may not preserve `var(--…)` in `svg.style.color` reliably — the Phase 3 assertion `expect(svg.style.color).toBe('var(--ac-doc-X)')` may pass vacuously (both sides empty) or fail unexpectedly. Use `getAttribute('style')` or `style.cssText` for robustness.
- 🔵 `hexToRgb` helper in `glyph-resolved-fill.spec.ts` doesn't handle 3-digit shorthand. Could silently mis-assert if any token resolves to `#fff`-style.
- 🔵 Resolved-fill spec only covers `decisions × {light,dark}` — 11 other token bindings unverified end-to-end. Parametrise.
- 🔵 `[data-testid="glyph-cell-decisions-24"]` selector failure mode is opaque if showcase template breaks. Strengthen Phase 4 component test to assert bijection over `(docType, size)` cells, not just count.
- 🔵 No-hooks source-grep regex can false-match comments mentioning `useEffect`. Strip comments first, or require open-paren.
- 🔵 `role="presentation"` on `<div>` is redundant; some a11y linters flag it.
- 🔵 Showcase exposes every Glyph with `ariaLabel`, so 36 `role="img"` to AT — fails to preview the decorative branch, and misrepresents typical consumer usage (the Consumer Contract recommends adjacent-text-label + decorative). Drop `ariaLabel` in showcase, put the label in the sibling `<span>`.
- 🔵 Inline `style={{ color: ... }}` defeats stylesheet caching at consumer-scale; minor allocation twin to the `a11y` literal.
- 🔵 16 px cells get the same 5% per-cell tolerance as 32 px cells (proportionally looser); consider size-scaled or absolute `maxDiffPixels`.
- 🔵 Production silent-null for unknown docType is defensible but unauditable. Document the choice or render a neutral fallback.
- 🔵 `<version>` placeholder in Docker command relies on implicit `package.json` lookup; either hard-code the pinned `1.59.1` or generate via shell.
- 🔵 New `GLYPH_DOC_TYPE_KEYS` couples Glyph module to runtime `DOC_TYPES` registry; if `DOC_TYPES` ever becomes async, the filter produces an empty array. Worth a comment.
- 🔵 Per-cell baseline strategy makes `data-testid` format a load-bearing contract surface of the showcase route — needs a comment in `GlyphShowcase.tsx`.

### Assessment

The plan is meaningfully better than at pass 1: every previously-major finding has been engaged, most are cleanly closed, and the structural decisions (`color`/`currentColor`, per-cell clips, exhaustiveness assertion, Consumer Contract) are all genuinely tighter. However, the new round of issues — concentrated in mismatches between the plan's code snippets and the live codebase (contrast helper, `DOC_TYPES`, `import.meta.env.DEV`) — means an implementer following the plan literally would hit hard errors before any tests pass. These are surgical, not structural; a third pass should be quick and focused. Recommend a small REVISE round targeting:

1. Fix the contrast helper import (`contrastRatio` from `./contrast`).
2. Add Phase 0 (or amend Phase 1) to introduce a static `DOC_TYPES` export with `{ key, virtual, label }` — OR document that the plan accepts a stable hardcoded fallback for `templates` exclusion and label lookup.
3. Switch runtime guard to `import.meta.env.DEV`.
4. Specify `querySelectorAll('*')` for the no-hex-child assertion.
5. Mirror the Consumer Contract into the work item (or extract to a TSDoc block on `Glyph`).
6. Add a CI-runtime estimate + parallelism note for the per-cell Playwright spec.

Once those land, the plan is implementation-ready.

## Re-Review (Pass 3 — spot-check) — 2026-05-12

**Verdict:** APPROVE

The 6 surgical fixes from pass 2 landed coherently against the live codebase:

- `contrastRatio` import from `./contrast` resolves to the real `src/styles/contrast.ts` helper; `parseHex` handles 3-digit shorthand.
- `VIRTUAL_DOC_TYPE_KEYS` and `DOC_TYPE_LABELS` are typed safely (`readonly DocTypeKey[]` and `Readonly<Record<DocTypeKey, string>>`); a Vitest key-equality assertion catches drift.
- `GLYPH_DOC_TYPE_KEYS` derives from the static list; INVARIANT comment makes the type-alias / runtime-filter lock-step explicit.
- Runtime guard uses `import.meta.env.DEV` matching `Breadcrumbs.tsx`.
- Descendant `fill`/`stroke` assertion uses `querySelectorAll('*')`.
- Consumer Contract lives both as a TSDoc block on `Glyph` and as a Phase 5 step mirroring into the work item's Resolved Decisions section.
- Per-cell Playwright spec has a CI-runtime estimate (2-4 min/platform) and `test.describe.configure({ mode: 'parallel' })`.

No new issues introduced. Two acceptable manual lock-steps remain (type-alias↔VIRTUAL_DOC_TYPE_KEYS; server `DocType.virtual`↔static `VIRTUAL_DOC_TYPE_KEYS`) — both are documented in code and out of scope for 0037.

Plan is implementation-ready.
