---
date: "2026-05-26T15:30:00Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-26-0088-markdown-body-width-harmonisation.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards]
review_pass: 2
status: complete
---

## Plan Review: 0088 Markdown Body Width Harmonisation

**Verdict:** COMMENT

Plan is acceptable but could be improved — see major findings below.

The plan is architecturally sound and faithful to the codebase's
two-mirror token discipline, the `migration.test.ts` ratchet, and the
established Playwright `tokens.spec.ts` idiom. The TDD red/green
ordering is verifiable and the regex assertions correctly match the
proposed CSS rule shape. The two main concerns are concentrated on
the new Playwright surface: the specs lack a content-readiness gate
on async-loaded markdown body, and the body-size change (the story's
substantive visual delta) is observed *only* by PNG diff with no
deterministic resolved-size guard. Several lenses also independently
flag the duplication of the `relativeTimeMask`/theme-flip helpers
across visual-regression specs as the natural moment to extract
shared infrastructure.

### Cross-Cutting Themes

- **Visual-regression helper duplication** (flagged by:
  architecture, code-quality, test-coverage, standards) — the new
  specs re-declare `relativeTimeMask` and the `data-theme` +
  `requestAnimationFrame` block inline rather than extracting to
  `tests/visual-regression/helpers.ts`. This will be the third+
  copy point and the standards lens notes the codebase already
  carries three different theme-flip variants. The plan should
  either extract now or explicitly justify the per-spec copy.
- **Regex pinning vs ratchet enforcement** (flagged by:
  architecture, code-quality, test-coverage, correctness,
  standards) — multiple lenses observed that the new
  whitespace-sensitive regex pinning the `.markdown` rule shape
  is low-marginal-value: the existing `EXCEPTIONS hygiene` +
  `var(--NAME) references resolve` ratchets already cover the
  architecturally meaningful invariants, and the bespoke regex
  pins implementation detail (exact `min(a, b)` argument order,
  trailing semicolon, etc.). The shell-grep assertions in
  Phase 2 success criteria duplicate the same negative-guard
  already enforced by the ratchet. Coherent direction: soften
  or drop the bespoke regex case, drop the shell-grep checks,
  lean on the existing ratchet.
- **Body-size silent to CI** (flagged by: test-coverage,
  standards) — the substantive visible change of the story is the
  new `font-size: var(--size-body)` (20px) on markdown `<p>`/`<li>`
  text. The plan explicitly opts *out* of adding a one-line case
  to `typography-resolved-sizes.spec.ts` (which already navigates
  to `/library/plans/first-plan` for the H1 case) and delegates
  the observation entirely to PNG baselines. A regression that
  subtly resizes body text could ride along under a re-baseline.

### Tradeoff Analysis

- **Test brittleness vs explicit coverage**: Architecture and code
  quality argue the new regex case is redundant against existing
  ratchets and the new PNG baselines; test coverage and standards
  argue *more* deterministic coverage is needed (a resolved-size
  case for `<p>`). Resolution: drop the brittle whitespace-regex
  case, add the cheap deterministic resolved-size case. Both
  lenses align on this combination.

### Findings

#### Major

- 🟡 **Test Coverage**: Screenshot specs lack a readiness gate
  for async-loaded markdown body content
  **Location**: Phase 2, Section 4 & 5: new Playwright specs
  `library-doc-view.spec.ts` does `page.goto(...)` then
  immediately screenshots. `LibraryDocView` renders `<p>Loading…</p>`
  until `useDocPageData` resolves; `RelatedArtifacts` shows a
  `Loading…` placeholder. The dark-theme `requestAnimationFrame`
  only commits the theme attribute, not data arrival. The
  screenshot may capture the loading placeholder instead of the
  markdown body. Add `await page.locator('[class*="markdown"] p').first().waitFor()`
  (and an analogous wait on `/code-syntax-showcase`) before
  `toHaveScreenshot`.

- 🟡 **Test Coverage**: Body-size change is unobserved by any
  deterministic assertion
  **Location**: Phase 2, Testing Strategy: "No new resolved-size
  case is added to typography-resolved-sizes.spec.ts"
  The plan explicitly delegates observation of the new
  `font-size: var(--size-body)` (20px) on markdown `<p>` to PNG
  baselines. `typography-resolved-sizes.spec.ts` already exercises
  `/library/plans/first-plan` and asserts exact-px font-size on
  `[class*="markdown"] h1`. A one-line addition to `CASES`
  (`{ name: 'MarkdownRenderer body p', route: '/library/plans/first-plan', selector: '[class*="markdown"] p', expected: '20px' }`)
  gives the body-size invariant a fast, deterministic guard
  alongside the slower PNG comparison.

#### Minor

- 🔵 **Architecture**: Third consumer inherits a uniform change
  without any verification ratchet
  **Location**: What We're NOT Doing: 'Not touching FrontmatterTable'
  Plan applies the new `font-size` declaration uniformly but
  exempts the (claimed) third consumer from any verification.
  Note: this finding interacts with the correctness finding below
  — `FrontmatterTable` may not in fact be a `MarkdownRenderer`
  consumer at all.

- 🔵 **Correctness**: FrontmatterTable is not actually a
  MarkdownRenderer consumer
  **Location**: 'What We're NOT Doing' + Manual Testing Steps §6
  Reading `FrontmatterTable.tsx` shows it only imports the
  `Resolver` type from `MarkdownRenderer/wiki-link-plugin` — it
  does not render `<MarkdownRenderer>` and does not apply the
  `.markdown` class. The "100% branch dominates" reasoning is
  moot. The plan should strike the bullet (or restate it
  factually) and drop manual step §6. Optional: correct the
  research note that introduced the misclassification.

- 🔵 **Correctness**: No explicit web-font readiness wait before
  screenshot on long-form prose
  **Location**: Phase 2 Steps 4 & 5
  Risk of flaky PNG diffs from fallback-font glyph metrics in
  the captured frame. Add `await page.evaluate(() => document.fonts.ready)`
  in both new specs, independent of the theme-flip rAF.

- 🔵 **Architecture**: Mixing unit families in LAYOUT_TOKENS
  dilutes the family's invariants
  **Location**: Phase 1, §1
  `'72ch'` is the first font-relative entry in `LAYOUT_TOKENS`
  alongside hard-pixel siblings. Add an inline comment in both
  `global.css` and `tokens.ts` flagging that the value resolves
  in the consumer element's font context.

- 🔵 **Architecture / Code Quality / Test Coverage / Standards**:
  `relativeTimeMask` and theme-flip helpers duplicated rather
  than extracted
  **Location**: Phase 2, Sections 4 & 5
  See Cross-Cutting Themes above. Either extract to
  `tests/visual-regression/helpers.ts` or document the per-spec
  copy as a deliberate convention.

- 🔵 **Code Quality / Architecture**: Two Playwright specs copy
  the theme-loop scaffold instead of joining the `ROUTES` table
  **Location**: Phase 2, Sections 4 & 5
  `tokens.spec.ts` encodes the same pattern as a `ROUTES` table.
  Consider extending the ROUTES table with the two new entries
  rather than adding two new files, or document the rationale
  for per-surface files.

- 🔵 **Code Quality / Test Coverage**: Pinning regex is
  whitespace- and ordering-sensitive
  **Location**: Phase 2, Section 1
  Regex bakes in exact `, 100%` ordering and trailing-semicolon
  spelling; a future `clamp(0, var, 100%)` or reordered `min()`
  args fails the test for non-semantic reasons. Either soften
  the regex to assert only the architectural facts (file mentions
  the new var-ref AND `var(--size-body)` AND has no `720px`),
  or replace with a small CSS-rule parser.

- 🔵 **Architecture**: Regex-pinned CSS rule shape couples a
  structural test to syntactic detail
  **Location**: Phase 2, §1
  The bespoke regex case is mostly redundant against existing
  ratchet + new PNG baselines. Consider dropping the case
  entirely or softening it.

- 🔵 **Correctness**: Discoverability assertion does not
  short-circuit the subsequent regex assertions
  **Location**: Phase 2 Step 1
  If `css` is undefined, the two regex `it`s emit confusing
  "expected `''` to match …" failures. Use
  `(css ? it : it.skip)` so a missing file produces one real
  failure instead of one + two noisy ones.

- 🔵 **Standards**: New spec test names break the established
  id-prefixed convention
  **Location**: Phase 2, Steps 4 / 5
  Either declare a `ROUTES` constant and use the
  `${id} (${theme})` shape from `tokens.spec.ts`, or wrap with
  `test.describe(...)` per `chip-showcase.spec.ts`.

- 🔵 **Standards**: New describe block references
  `cssBySrcRelative` declared lower in the file
  **Location**: Phase 2, Step 1
  Specify that the new describe block lands *after* the
  `cssBySrcRelative` declaration at lines 391–395 — placing
  before would TDZ-error. Recommend placing immediately after
  the `EXCEPTIONS hygiene` describe to inherit scoping
  unambiguously.

- 🔵 **Standards**: Automated verification uses ad-hoc `grep`
  rather than the existing ratchet
  **Location**: Phase 2: Success Criteria — Automated Verification
  The shell-grep assertions duplicate the negative guard already
  enforced by the `migration.test.ts` ratchet. Drop them.

- 🔵 **Standards**: Snapshot filenames in the plan omit
  Playwright's `-<project>-<platform>` suffix
  **Location**: Phase 2, Step 6
  Existing baselines under `tokens.spec.ts-snapshots/` use the
  form `library-light-visual-regression-darwin.png` +
  `-linux.png`. The plan's four-file list is actually 8 PNGs
  (darwin + linux); update the list and success-criteria
  accordingly.

- 🔵 **Test Coverage**: Baselines captured under developer-local
  conditions risk drift from CI
  **Location**: Phase 2, Section 6
  Either document the baseline-capture environment expectations,
  or run the new specs in CI first with `--update-snapshots` and
  pull the canonical PNGs back into the PR.

#### Suggestions

- 🔵 **Code Quality**: Two assertions on the same rule could be
  one composite check with a clearer failure message
  **Location**: Phase 2, Section 1
  Optional readability nudge — collapse the two regex `it`s or
  expose the extracted rule body in the assertion message.

- 🔵 **Code Quality**: Consider co-locating font-size and
  max-width with an explanatory comment
  **Location**: Phase 2, Section 2
  Add a one-line CSS comment above `.markdown` capturing the
  `1ch lives on .markdown` invariant from the plan's
  Implementation Approach §1, so a future refactor that splits
  the declarations across selectors has an in-file warning.

- 🔵 **Architecture**: Token storing literal CSS `72ch` means
  downstream resolved-value tests cannot equate token to a
  computed-px value
  **Location**: Implementation Approach §2 / Phase 1
  Add a one-liner in Migration Notes acknowledging that the
  prose cap's effective px is observable only via consumer-
  element measurement.

- 🔵 **Test Coverage**: FrontmatterTable left without even a
  smoke-level test trace
  **Location**: What We're NOT Doing
  Subsumed by the correctness finding above — if FrontmatterTable
  is not actually a consumer, this concern dissolves.

### Strengths

- ✅ Honours the two-mirror token discipline and lets the existing
  parametric parity test drive Phase 1 test-first.
- ✅ Co-locates the `ch`-based cap and the explicit
  `font-size: var(--size-body)` on the same `.markdown` rule —
  the only architecturally correct place for a `ch` measure.
- ✅ Uses `min(var(--token), 100%)` so the cap composes with the
  parent grid track without overflow.
- ✅ Bakes the `ch` unit into the token value (string `'72ch'`)
  per existing `LAYOUT_TOKENS` convention.
- ✅ Phase split is genuinely independent — Phase 1's token is
  harmless if never consumed.
- ✅ Removes a known irreducible exception rather than adding one
  — the migration ratchet moves forward.
- ✅ Continues the `--ac-content-max-width{,-narrow,-prose}`
  semantic-suffix naming.
- ✅ TDD red/green steps spelled out with explicit failure modes;
  ordering verifiable end-to-end.
- ✅ Regex literals in Phase 2 step 1 correctly match the
  proposed CSS shape; `\s*` tolerates inner whitespace
  variations.
- ✅ The new Playwright specs follow the established
  `tokens.spec.ts` idiom (1440x900, theme loop, `data-theme`
  mutation with rAF, 0.05 maxDiffPixelRatio, animations
  disabled).
- ✅ Accessibility-relevant: the explicit 20px body size is a
  meaningful reading-comfort improvement over inherited-root.
- ✅ Plan acknowledges the story's "refresh baselines" AC was
  misspecified and resolves it by introducing new specs rather
  than silently softening.

### Recommended Changes

1. **Add a content-readiness wait to both new Playwright specs**
   (addresses: Screenshot specs lack a readiness gate; No
   explicit web-font readiness wait) — before each
   `toHaveScreenshot` call, add
   `await page.locator('[class*="markdown"] p').first().waitFor()`
   (or the appropriate equivalent for `/code-syntax-showcase`)
   AND `await page.evaluate(() => document.fonts.ready)`.
2. **Add a deterministic body-size resolved case** (addresses:
   Body-size change is unobserved; Body-size silent to CI;
   FrontmatterTable scope clarification) — add one entry to the
   `CASES` array in `typography-resolved-sizes.spec.ts`:
   `{ name: 'MarkdownRenderer body p', route: '/library/plans/first-plan', selector: '[class*="markdown"] p', expected: '20px' }`.
3. **Correct the FrontmatterTable claim** (addresses: third
   consumer not actually a consumer) — verify the
   `FrontmatterTable.tsx` import is just `Resolver` from the
   wiki-link plugin; if confirmed, strike the "Not touching
   FrontmatterTable" bullet (or restate it factually) and drop
   step 6 of the Manual Testing Steps. Update the research note
   if appropriate.
4. **Decide and apply a single visual-regression-helpers
   approach** (addresses: helper duplication across architecture,
   code-quality, test-coverage, standards) — either extract
   `relativeTimeMask` and `applyTheme(page, theme)` to
   `tests/visual-regression/helpers.ts` and consume from both
   new specs + `tokens.spec.ts`, or document inline-per-spec as
   a deliberate convention in the plan.
5. **Soften the bespoke `.markdown` regex assertions and drop
   the shell-grep checks** (addresses: regex pinning brittleness;
   ad-hoc grep duplicates ratchet) — either drop the new
   describe block entirely and rely on the existing ratchet +
   new PNG baselines + the new resolved-size case from item 2,
   or rewrite the assertions to test only the architectural
   facts (file mentions both var-refs, no `720px` literal).
   Drop the two `! grep` lines from Success Criteria.
6. **Pin the new describe-block placement** (addresses: TDZ risk)
   — specify in Phase 2 §1 that the new describe block lands
   *immediately after* the `EXCEPTIONS hygiene` block at
   line 432, where `cssBySrcRelative` is already in scope.
7. **Correct the snapshot file count** (addresses: snapshot
   filename suffix omission) — update Phase 2 §6 to list the
   actual on-disk artefacts (8 PNGs, darwin + linux × 2 routes
   × 2 themes) per the `-<project>-<platform>` suffix
   convention. Either commit both platforms or document a
   one-platform policy.
8. **Add a one-line CSS comment to `.markdown`** (addresses:
   future maintenance foot-gun) — a comment in the CSS module
   capturing the "font-size + max-width co-located so `1ch` is
   computed against this element's own font" invariant from
   Implementation Approach §1.

---

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally sound: it tightens a leaky
abstraction (a 720px literal that secretly depended on inherited
font-size) into a single, semantically-named layout token consumed at
the only element where the `ch` measure can be meaningful. It
respects the two-mirror token discipline, the migration-test ratchet,
and the `min(var(...), 100%)` composition pattern, and the two-phase
split has clean independence boundaries. Main architectural risk:
uniform application to the unverified third consumer; minor
naming/abstraction question about whether `--ac-content-max-width-prose`
truly belongs in the layout family alongside hard-px tokens.

**Strengths**:
- Honours the established two-mirror token discipline; existing
  parametric parity test drives Phase 1 test-first.
- Co-locates the `ch`-based cap and explicit `font-size` on the
  same rule (the only architecturally correct place).
- Uses `min(var, 100%)` so the cap composes with parent grid.
- Token value bakes `ch` unit per LAYOUT_TOKENS convention.
- Phase split genuinely independent.
- Removes an irreducible-exception entry; ratchet moves forward.
- Continues `*-narrow`/`*-prose` semantic-suffix naming.
- New vitest case anchors the `.markdown` rule's shape into tests.

**Findings**:
- 🔵 Third consumer (FrontmatterTable) inherits a uniform change
  without any verification ratchet.
- 🔵 Mixing unit families in LAYOUT_TOKENS dilutes the family's
  invariants — `'72ch'` is the first font-relative entry.
- 🔵 Regex-pinned CSS rule shape couples a structural test to
  syntactic detail; mostly redundant against existing ratchet + PNGs.
- 🔵 (Suggestion) `relativeTimeMask` duplicated rather than
  extracted as a shared module.
- 🔵 (Suggestion) Token storing literal CSS `72ch` means resolved-
  value tests cannot equate token to a computed-px value at
  `:root`.

### Code Quality

**Summary**: Plan is small, well-scoped and reads cleanly. TDD
ordering is explicit; existing harness carries most enforcement
weight; net new test code is one describe + two short assertions.
Maintainability concerns are largely cosmetic — duplicated
`relativeTimeMask`, whitespace-sensitive regexes pinning the CSS
rule shape, copy-pasted Playwright spec bodies that the project's
`tokens.spec.ts` would normally fold into a single ROUTES table.

**Strengths**:
- TDD red/green steps spelled out per change with failure modes.
- Leans on existing parity loop + EXCEPTIONS hygiene + var-resolve
  ratchet instead of new machinery.
- Design intent documented inline (the `ch lives on .markdown`
  note).
- New specs follow `tokens.spec.ts` idiom verbatim.
- Token value bakes the `ch` unit as `'72ch'`.

**Findings**:
- 🔵 `relativeTimeMask` duplicated inline rather than extracted.
- 🔵 Two Playwright specs copy the theme-loop scaffold instead of
  joining the `ROUTES` table.
- 🔵 Pinning regex is whitespace- and ordering-sensitive.
- 🔵 (Suggestion) Two assertions could be one composite with a
  clearer failure message.
- 🔵 (Suggestion) Add a CSS comment co-locating font-size and
  max-width invariant.

### Test Coverage

**Summary**: Strong job leveraging the parity loop and EXCEPTIONS
hygiene as TDD drivers — vitest side is well-anchored. However, the
new Playwright specs are the only place the body-size change is
observed by CI, and they (a) capture full-page screenshots with no
readiness gate, (b) duplicate the dark-theme setup helper, and
(c) deliberately decline a cheap resolved-size unit-level guard for
`<p>` text.

**Strengths**:
- Uses the existing parametric parity loop to drive Phase 1.
- Treats the `EXCEPTIONS hygiene` block as a forcing driver.
- New describe pins both AC pins at the source level.
- Includes `expect(css).toBeDefined()` as a discoverability guard.
- Explicitly verifies the existing H1 resolved-size case is
  unaffected.

**Findings**:
- 🟡 Screenshot specs lack a readiness gate for async-loaded
  markdown body content.
- 🟡 Body-size change is unobserved by any deterministic assertion.
- 🔵 Regex assertions are brittle to inter-declaration formatting
  changes.
- 🔵 New specs duplicate the theme-flip helper rather than
  extracting shared infrastructure.
- 🔵 Baselines captured under developer-local conditions risk
  drift from CI.
- 🔵 (Suggestion) Third consumer left without even a smoke-level
  test trace.

### Correctness

**Summary**: Plan is internally consistent; regex assertions
correctly match the proposed CSS rule shape; red/green ordering is
sound; the `min(var, 100%)` composition is correct at both 1440px
and 800px viewports; pinning `font-size: var(--size-body)` on the
same rule resolves the `ch` reference frame correctly because
computed-value calculation uses the element's own computed font-size
regardless of declaration order. Minor concerns: factual error about
`FrontmatterTable` being a third consumer (it is not), potential
flake risk in the new Playwright specs without font-readiness wait.

**Strengths**:
- Regex literals correctly match the proposed CSS rule shape; `\s*`
  tolerates whitespace variations.
- Red/green ordering verifiable end-to-end.
- `ch` reference frame correctly anchored on the same element.
- `min(var, 100%)` composition has the predicted behaviour at both
  breakpoints.
- Parity loop and var-resolve ratchet auto-pick the new key.
- `PX_REM_EM_RE` doesn't match `ch`; no new exception needed.

**Findings**:
- 🔵 FrontmatterTable is not actually a `MarkdownRenderer`
  consumer — it imports only the `Resolver` type from the
  wiki-link plugin. The plan's reasoning about the `100%` branch
  is moot.
- 🔵 No explicit web-font readiness wait before screenshot on
  long-form prose.
- 🔵 Discoverability assertion does not short-circuit the
  subsequent regex assertions (noisy failure messages on missing
  file).

### Standards

**Summary**: Plan is largely faithful to the project's two-mirror
token discipline, the semantic-suffix naming, the
`migration.test.ts` ratchet idioms, and the `tokens.spec.ts`
Playwright precedent. A few minor convention-drift points: new
Playwright specs deviate from the established id-prefixed test-name
pattern; the migration test's `cssBySrcRelative` reference must
land in the right order; shell-grep assertions in Success Criteria
duplicate existing ratchet enforcement.

**Strengths**:
- Token naming respects the semantic-suffix family.
- Two-mirror discipline honoured; column-alignment preserved.
- Token value strings bake the `ch` unit.
- Relies on `LayoutToken` auto-derivation and the var-resolve
  ratchet.
- Honours ADR-0036 / `PX_REM_EM_RE` convention.
- Recognises bidirectional EXCEPTIONS hygiene enforcement.
- Ships an accessibility improvement (explicit 20px body size).
- Resolves the misspecified baseline AC by introducing specs
  rather than silently softening.

**Findings**:
- 🔵 New spec test names break the established id-prefixed
  convention.
- 🔵 New describe block references `cssBySrcRelative` declared
  lower in the file — pin the placement.
- 🔵 Inlining `relativeTimeMask` duplicates a helper rather than
  promoting it.
- 🔵 Automated verification uses ad-hoc `grep` rather than the
  existing `migration.test.ts` ratchet.
- 🔵 Snapshot filenames in the plan omit Playwright's
  `-<project>-<platform>` suffix (8 PNGs, not 4).
- 🔵 (Suggestion) Body-size change is silent to
  `typography-resolved-sizes.spec.ts` — add a one-line case.

---

## Re-Review (Pass 2) — 2026-05-26T15:30:00Z

**Verdict:** COMMENT

The pass-1 edits land cleanly. Of the 16 distinct findings in pass 1,
14 are resolved (with the FrontmatterTable misclassification dissolved
factually rather than mitigated) and 2 architecture suggestions
remain by design (the user opted to skip them as light-touch
suggestions). Two net-new minor concerns surface in pass 2: a real
serialisation pitfall in the `document.fonts.ready` await spelling,
and a residual readiness gap on the `LibraryDocView` aside panel
that the new screenshot spec does not wait for.

### Previously Identified Issues

#### Architecture
- 🔵 **Third consumer (FrontmatterTable) without ratchet** — Dissolved (factually corrected to non-consumer)
- 🔵 **Mixing unit families in LAYOUT_TOKENS** — Still present (intentionally not addressed — light-touch suggestion)
- 🔵 **Regex-pinned CSS rule shape** — Resolved (replaced with `toContain` presence checks)
- 🔵 **`relativeTimeMask` duplication** — Resolved (extracted to `helpers.ts`)
- 🔵 **Token storing `'72ch'` observation gap** — Still present (intentionally not addressed — light-touch suggestion)

#### Code Quality
- 🔵 **`relativeTimeMask` duplicated inline** — Resolved
- 🔵 **Two specs copy theme-loop instead of joining ROUTES** — Partially resolved (each spec now uses an id-prefixed ROUTES pattern, but as separate files; new `ROUTES`-of-one is mild YAGNI)
- 🔵 **Pinning regex whitespace-sensitive** — Resolved
- 🔵 **(Suggestion) Two assertions could be one composite** — Still present (kept split — failure messages are clearer)
- 🔵 **(Suggestion) Co-locate CSS comment for invariant** — Resolved

#### Test Coverage
- 🟡 **Screenshot specs lack readiness gate** — Partially resolved (markdown body waited for; aside panel `Loading…` still not waited for — see new finding below)
- 🟡 **Body-size unobserved by deterministic assertion** — Resolved (`MarkdownRenderer body p` 20px case added)
- 🔵 **Regex brittleness** — Resolved
- 🔵 **Theme-flip helper duplication** — Resolved
- 🔵 **Baselines CI drift** — Resolved (8-PNG policy + workflow documented)
- 🔵 **(Suggestion) Third consumer smoke trace** — Dissolved (FrontmatterTable correctly reclassified)

#### Correctness
- 🔵 **FrontmatterTable not actually a consumer** — Resolved
- 🔵 **No web-font readiness wait** — Resolved (but see new fonts.ready spelling finding below)
- 🔵 **Discoverability assertion does not short-circuit** — Resolved (`itIfPresent = css ? it : it.skip`, matching existing idiom at `migration.test.ts:383`)

#### Standards
- 🔵 **New spec test names break id-prefixed convention** — Resolved
- 🔵 **New describe block references `cssBySrcRelative` declared lower** — Resolved (placement pinned to immediately after `EXCEPTIONS hygiene` at line 432)
- 🔵 **`relativeTimeMask` duplicated** — Resolved
- 🔵 **Shell-grep duplicates ratchet** — Resolved (dropped)
- 🔵 **Snapshot filenames omit suffix** — Resolved (8-PNG enumeration with `-<project>-<platform>` suffix)
- 🔵 **(Suggestion) Body-size silent to typography-resolved-sizes** — Resolved

### New Issues Introduced

- 🔵 **Correctness**: `await page.evaluate(() => document.fonts.ready)`
  in §6 and §7 returns a non-serialisable `FontFaceSet` object.
  Playwright awaits the promise then structured-clones the resolved
  value back to the test process — depending on version, this either
  throws a serialisation error or coerces to `{}`. Safer spelling:
  `await page.evaluate(() => document.fonts.ready.then(() => undefined))`
  in both specs.

- 🔵 **Test Coverage**: `library-doc-view.spec.ts` readiness gate
  covers the markdown body but not the `RelatedArtifacts` aside
  panel. `useDocPageData` issues two independent queries
  (`useDocContent`, `useRelated`); the aside renders its own
  `<p>Loading…</p>` while `related.isPending` is true. The
  full-page screenshot can capture a half-loaded aside even when
  the markdown body is fully rendered. Either anchor the wait on
  an aside-resolved sentinel, or `mask` the aside in the
  screenshot options.

- 🔵 **Test Coverage**: `.hljs-keyword` readiness sentinel in
  `code-syntax-showcase.spec.ts` is fixture-dependent — a future
  edit to the showcase content that drops keyword tokens turns the
  gate into an indefinite wait. Consider broader anchor
  (`pre code.hljs`) or an in-spec comment naming the assumption.

- 🔵 **Code Quality (suggestion)**: `applyTheme` light-branch
  early-return is an implicit no-op. At call site,
  `await applyTheme(page, 'light')` is silent on the intent.
  Optional: rename to `applyDarkThemeIfNeeded` or strengthen the
  helper comment.

- 🔵 **Standards (suggestion)**: `applyTheme` name promises a
  unification it doesn't deliver — `chip-showcase.spec.ts` uses a
  different polling-based theme-flip variant that this helper
  intentionally doesn't subsume. Optional: narrow the helper name
  (e.g. `applyThemeViaRaf`) or annotate the helper module.

### Assessment

The plan is in good shape and approaches implementation-ready. The
two real concerns — the `document.fonts.ready` serialisation
pitfall and the `RelatedArtifacts` aside readiness gap — are both
narrow, well-scoped, and addressable with a one-line change each.
Neither is severe enough to gate the plan; the verdict remains
COMMENT. The remaining items are suggestions the user can take or
leave on tone-and-style grounds.

If a third pass is desired, a single edit addressing the two
test-coverage concerns above would clear the pass-2 findings; the
suggestions can stay or go at the author's discretion.

---
*Review generated by /review-plan*
