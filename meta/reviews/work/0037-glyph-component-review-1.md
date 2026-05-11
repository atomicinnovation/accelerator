---
date: "2026-05-12T00:06:54Z"
type: work-item-review
skill: review-work-item
target: "meta/work/0037-glyph-component.md"
work_item_id: "0037"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
---

## Work Item Review: Glyph Component

**Verdict:** REVISE

The work item is structurally complete and well-written, with strong user-story framing, enumerated doc types, and given/when/then acceptance criteria. However, three cross-cutting issues — an unresolved Storybook/showcase deliverable, an integration AC that defers verification to other work items, and a screenshot-only contract for per-doc-type colours — surface across multiple lenses and weaken the work item's readiness for implementation. Several internal contradictions in Technical Notes (LIFECYCLE_PIPELINE_STEPS count, "twelve" vs "thirteen" `DocTypeKey` values, stale 0033 reference) should be reconciled before planning starts.

### Cross-Cutting Themes

- **Storybook / showcase deliverable is unresolved** (flagged by: clarity, dependency, testability) — Requirements mandates a "Storybook (or equivalent) entry", Technical Notes says no Storybook tooling exists and defers the format to implementation, and no AC pins down what artefact passes the check. Implementer, reviewer, and planner each get a different answer.
- **Consumer-integration AC leaks scope outside this story** (flagged by: clarity, scope, testability) — The final AC defers verification to eight downstream work items, so 0037 cannot be marked done on its own terms; the Summary's "thread it through every doc-type reference" wording reinforces this scope drift.
- **`--ac-doc-*` token sub-namespace and the stale 0033 reference** (flagged by: clarity, dependency, scope) — Dependencies says "Blocked by: none — 0033 done", Technical Notes says the per-doc-type colour sub-namespace doesn't exist yet and "the Blocked by 0033 dependency may be stale". The two sections need to converge.
- **Per-doc-type colours live only in the screenshots** (flagged by: clarity, testability) — Eleven of twelve doc types have their fill colour defined only by the canonical PNGs, with no token-name convention promoted into Requirements and no comparison procedure for verification.

### Findings

#### Major
- 🟡 **Clarity**: Unresolved self-correction about LIFECYCLE_PIPELINE_STEPS count
  **Location**: Technical Notes
  Two contradictory statements about how many keys LIFECYCLE_PIPELINE_STEPS renders are both present in Technical Notes ("11 of 12" vs "11 of 13"), with the later bullet correcting the earlier rather than the earlier being edited. A reader cannot tell which count is authoritative.

- 🟡 **Clarity**: "Twelve `DocTypeKey` values" framing contradicts the thirteen-key disclosure
  **Location**: Summary / Requirements / Acceptance Criteria
  Summary, Requirements, AC, and Assumptions all assert "twelve `DocTypeKey` values", but Technical Notes states the union actually contains thirteen (twelve scoped here plus virtual `templates`). AC 7's "rejected at the type level" claim would be false against the actual union as currently typed.

- 🟡 **Clarity**: Storybook requirement contradicts Technical Notes' "no Storybook configured" note
  **Location**: Requirements / Technical Notes
  Requirements mandates a Storybook-or-equivalent entry; Technical Notes records no Storybook tooling is configured and defers the choice. A reader cannot tell whether the Requirement is binding or advisory.

- 🟡 **Clarity**: Summary's "thread it through every doc-type reference" contradicts the Technical Notes deferral
  **Location**: Summary
  Summary describes threading Glyph through eight consumer surfaces; Technical Notes defers that threading to 0036/0040/0041/0042/0043/0053/0054/0055. AC 8 softens this but the Summary still reads as if integration is in scope of 0037.

- 🟡 **Testability**: Consumer-integration AC defers verification with no concrete check
  **Location**: Acceptance Criteria (final bullet)
  The final AC explicitly defers verification to downstream work items, so it cannot be passed or failed within the scope of 0037.

- 🟡 **Testability**: Verification depends on screenshots whose comparison procedure is unspecified
  **Location**: Acceptance Criteria (bullets 1, 3, 4) / Requirements
  Multiple ACs require matching "icon shape and colour shown in the updated library-view screenshots" with no tolerance, tool, or hex-extraction procedure defined.

- 🟡 **Testability**: Storybook-or-equivalent deliverable lacks a chosen, verifiable form
  **Location**: Requirements (final bullet) / Technical Notes
  No AC names the artefact concretely; an implementer could ship anything from a demo page to a visual-regression suite and claim compliance.

#### Minor
- 🔵 **Clarity**: Stale "Blocked by 0033" reference in Technical Notes has no counterpart in Dependencies
  **Location**: Dependencies / Technical Notes
  Dependencies says "Blocked by: none" already; the Technical Notes bullet flagging it as stale is an orphan note.

- 🔵 **Clarity**: Five-phase pipeline naming in Context doesn't match phase vocabulary elsewhere
  **Location**: Context
  Context names five phases (DEFINE → DISCOVER → BUILD → SHIP → REMEMBER); design-gap and prototype inventory use four (Define / Build / Ship / Remember).

- 🔵 **Clarity**: Token sub-namespace name is implied but not stated normatively
  **Location**: Requirements / Acceptance Criteria
  `--ac-doc-<key>` appears only as an example in Technical Notes; Requirements and AC reference only the generic `--ac-*` layer.

- 🔵 **Dependency**: Coordination with 0038 (Chip) not reflected in Dependencies
  **Location**: Dependencies
  Open Questions describes the coupling but Dependencies does not name 0038.

- 🔵 **Dependency**: Stale "Blocked by 0033" rationale acknowledged but not resolved
  **Location**: Dependencies
  Dependencies and Technical Notes disagree on whether 0033 is sufficient for Glyph to start.

- 🔵 **Testability**: "Without a component re-render" is implementation-leaning and hard to assert
  **Location**: Acceptance Criteria (theme-swap bullet)
  The clause requires React-internal instrumentation or is tautologically satisfied — neither is a meaningful gate.

- 🔵 **Testability**: Only `decisions` is called out explicitly; per-doc-type colours not enumerated
  **Location**: Acceptance Criteria (bullet 1) / Requirements
  Eleven of twelve doc-type colours are defined only by the PNGs; a token-name and hex table would make the contract verifiable.

- 🔵 **Testability**: "Without raster artefacts" lacks a defined check
  **Location**: Acceptance Criteria (bounding-box bullet)
  Replace with an unambiguous DOM check (e.g. root element is `<svg>` with `viewBox` set).

- 🔵 **Testability**: Default-decorative behaviour does not assert absence of `role` / `aria-label`
  **Location**: Requirements / Acceptance Criteria (bullet 5)
  An implementation that adds empty `aria-label=""` could technically pass AC 5 while still being subtly wrong.

#### Suggestions
- 🔵 **Dependency**: Storybook-or-equivalent tooling dependency is unresolved
  **Location**: Requirements
  The component-demonstration deliverable depends on a tooling decision that is invisible at planning time.

- 🔵 **Scope**: Introduction of `--ac-doc-*` token sub-namespace is quasi-separable
  **Location**: Requirements / Acceptance Criteria
  Tightly coupled to Glyph in practice; flagging only because Technical Notes calls the token work out as a distinct cost driver.

- 🔵 **Scope**: Final AC depends on work explicitly out of scope
  **Location**: Acceptance Criteria (final bullet)
  Either reframe as a non-blocking integration note or move to an "Out of Scope / Downstream Verification" section.

### Strengths

- ✅ Canonical user-story framing in Summary identifies user, want, and benefit explicitly.
- ✅ Domain terms (`DocTypeKey`, `--ac-*` tokens, `data-theme`) are anchored to specific file paths and line ranges so referents are unambiguous.
- ✅ The twelve `DocTypeKey` values in scope are enumerated verbatim in Requirements and Acceptance Criteria.
- ✅ Accessibility behaviour is described in active voice with named attribute outcomes — no ambiguity about what the implementation must produce.
- ✅ All sections present and substantively populated; frontmatter complete and valid.
- ✅ Scope is tightly bounded to the Glyph component, with consumer wiring explicitly deferred to 0036/0040/0041/0042/0043/0053/0054/0055.
- ✅ Drafting Notes justifies in-scope decisions on sound atomicity grounds (e.g. bringing the three previously-missing doc types in rather than spinning them out).
- ✅ Blocks list maps cleanly onto every consumer surface named in Context.
- ✅ Type-level rejection of invalid `docType` is a definitive pass/fail check.
- ✅ Per-size bounding-box AC and ARIA-attribute ACs are concrete and measurable.

### Recommended Changes

1. **Resolve the Storybook-or-equivalent deliverable** (addresses: Storybook clarity contradiction, Storybook testability ambiguity, Storybook dependency suggestion)
   Pick a concrete artefact — either a `/glyph-showcase` route, a Vitest snapshot suite over all 72 (docType × size × theme) permutations, or a Playwright visual spec — promote that choice into Requirements, and add an AC that names the artefact and its location. Remove the "(or equivalent)" hedge.

2. **Replace the integration AC with a self-contained check** (addresses: integration AC clarity/scope/testability findings)
   Drop the "consumed correctly by Sidebar nav, …" AC. Replace with something Glyph can satisfy on its own — e.g. "Glyph exposes a named export importable from `src/components/Glyph/Glyph` and renders without runtime error in a smoke test for every (docType, size) pair". Optionally move the consumer-integration list into a new "Out of Scope / Downstream Verification" section.

3. **Reconcile "twelve vs thirteen `DocTypeKey`"** (addresses: twelve-vs-thirteen clarity finding)
   Introduce a Glyph-specific type alias (e.g. `type GlyphDocTypeKey = Exclude<DocTypeKey, "templates">`) in Requirements, rephrase Summary/AC to say "the twelve non-virtual `DocTypeKey` values", and update AC 7 to talk about `GlyphDocTypeKey`. This makes the type-level rejection claim accurate.

4. **Edit Technical Notes to remove its own self-corrections** (addresses: LIFECYCLE_PIPELINE_STEPS contradiction, stale 0033 clarity/dependency findings)
   Replace the "11 of 12" bullet with "11 of 13" and delete the "correcting the '11 of 12' wording" meta-note. Delete the "Blocked by 0033 may be stale" sentence outright (Dependencies has already resolved that question). Optionally update Dependencies to say "Builds on 0033 (token layer landed); this work item extends it with a per-doc-type colour sub-namespace".

5. **Promote the `--ac-doc-<key>` token convention and add a colour table** (addresses: token-namespace clarity finding, screenshot-only verification testability findings)
   Add a Requirement: "Introduce a per-doc-type token sub-namespace named `--ac-doc-<key>` in both theme blocks of `global.css` and the `tokens.ts` mirror". Add a 12-row table in Requirements (or a dedicated subsection) mapping `DocTypeKey` → token name → light hex → dark hex, derived from the canonical screenshots, and make this table the contractual source of truth alongside (not instead of) the PNGs.

6. **Rewrite the Summary's second paragraph** (addresses: summary-vs-TN scope contradiction)
   Scope 0037 to component + assets + tokens only. Explicitly note that consumer integration is delivered by 0036/0040/0041/0042/0043/0053/0054/0055.

7. **Tighten the theme-swap, raster-artefacts, and a11y-default ACs** (addresses: three minor testability findings)
   - Theme-swap: "Toggling `document.documentElement[data-theme]` between `light` and `dark` updates each rendered Glyph's computed `fill` colour to match the corresponding token within one paint frame".
   - Raster artefacts: "The rendered icon root element is an `<svg>` with `viewBox` set and no `<img>` raster fallback".
   - A11y default: "…the inner SVG carries `aria-hidden=\"true\"` and carries neither `role` nor `aria-label`".

8. **Add 0038 to Dependencies as a coordination entry** (addresses: 0038 coordination dependency finding)
   Add: "Coordinates with: 0038 (Chip icon-slot decision affects Glyph composition; resolved in 0038)" so the coupling is visible at the dependency level, not only in Open Questions.

9. **Optional: reconcile the phase-count vocabulary in Context** (addresses: phase-naming clarity finding)
   Either drop the phase list from Context (Glyph's API is phase-agnostic) or align with the four-phase vocabulary used by the design-gap and prototype inventory.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is largely well-written with clear actor framing, defined types, and concrete acceptance criteria, but contains several internal contradictions that a fresh reader would have to mentally reconcile. The most material clarity issues are (a) an unresolved self-correction about LIFECYCLE_PIPELINE_STEPS counts left in Technical Notes, (b) a tension between the "twelve DocTypeKey values" framing used throughout and the late disclosure in Technical Notes that the union actually contains thirteen keys, (c) a contradiction between the Requirements-mandated Storybook entry and the Technical Notes admission that no Storybook tooling exists, and (d) a contradiction between the Summary's "thread it through every doc-type reference across the app" scope claim and the Technical Notes deferral of that threading to downstream work items. Image references were not inspected per instructions.

**Strengths**:
- Actor and outcome are explicit throughout; Acceptance Criteria use Given/When/Then phrasing with observable outcomes.
- Domain terms anchored to specific file paths and line ranges.
- Twelve DocTypeKey values enumerated verbatim in Requirements and Acceptance Criteria.
- Accessibility behaviour described in active voice with named attribute outcomes.

**Findings**: 4 major, 3 minor (see consolidated list above).

### Completeness

**Summary**: Work item 0037 is a thoroughly populated story with all expected sections present and substantively filled: Summary in user-story form, Context explaining motivation, Requirements with implementer-actionable bullets, Acceptance Criteria covering eight scenarios, plus populated Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes, and References sections. Frontmatter is complete and valid. Story-type requirements (user identified, criteria defining done, context for why) are all satisfied.

**Strengths**:
- Canonical user-story framing.
- Context explains motivating gap, canonical source of truth, and downstream impact.
- Requirements concrete and implementer-actionable.
- AC contains eight specific scenarios in given/when/then form.
- Frontmatter fully populated with recognised type, status, priority, author, date, tags.
- Optional sections all substantively populated.

**Findings**: none.

### Dependency

**Summary**: The work item has a thorough Dependencies section with a clear "Blocked by: none" rationale and a comprehensive Blocks list covering eight downstream consumers (0036, 0040, 0041, 0042, 0043, 0053, 0054, 0055) that align with the consumer surfaces named in Context. One coordination relationship with 0038 is captured in Open Questions rather than Dependencies, and the work item itself notes that the 0033 "Blocked by" rationale may be stale. No external systems or cross-team couplings are implied.

**Strengths**:
- Dependencies explicitly states "Blocked by: none" with reasoning.
- Blocks list comprehensive and traceable to consumer surfaces.
- AC explicitly defers consumer-integration verification to downstream work items.
- Technical Notes proactively flags the stale 0033 rationale.

**Findings**: 2 minor, 1 suggestion (see consolidated list above).

### Scope

**Summary**: The work item describes one coherent unit of work — a single React component (`Glyph`) with a well-bounded surface (component API, asset set for twelve doc types, token sub-namespace, accessibility behaviour, showcase entry). Consumer wiring is explicitly deferred, decomposition reads cleanly, and the Size "M" rating is justified.

**Strengths**:
- Scope tightly bounded; threading through eight consumers deferred.
- Summary, Requirements, and AC describe the same scope.
- Single team/service domain (frontend component + token additions).
- Drafting Notes justifies an in-scope decision with a sound atomicity argument.
- Open Questions and Assumptions flag adjacent concerns without expanding scope.

**Findings**: 2 suggestion (see consolidated list above).

### Testability

**Summary**: Acceptance Criteria are largely well-framed with Given/When/Then structure and observable outcomes, and most criteria reference concrete artefacts (sizes, doc-type keys, ARIA attributes). However, several criteria depend on canonical screenshots whose comparison procedure is unspecified, the last AC defers verification to downstream consumer work items, and the Storybook/showcase deliverable lacks a chosen format.

**Strengths**:
- AC use Given/When/Then framing consistently.
- Type-level rejection of invalid docType is a definitive compile-time check.
- ARIA criteria specify exact attribute values.
- Pixel-bounding-box AC for 16/24/32 gives a measurable check.

**Findings**: 3 major, 4 minor (see consolidated list above).

## Re-Review (Pass 2) — 2026-05-12

**Verdict:** REVISE (two major findings remain, both testability — see Assessment)

### Previously Identified Issues

- 🟡 **Clarity**: LIFECYCLE_PIPELINE_STEPS "11 of 12" vs "11 of 13" self-correction — **Partially resolved**. The meta-correction is gone, but the rewritten sentence ("excludes `work-item-reviews` from the twelve Glyph-supported doc types … lists 11 entries") is still ambiguous about whether work-item-reviews is in Glyph's twelve (now flagged minor).
- 🟡 **Clarity**: "Twelve vs thirteen DocTypeKey" framing — **Resolved**. `GlyphDocTypeKey = Exclude<DocTypeKey, "templates">` introduced in Requirements; Summary and AC consistently say "twelve non-virtual"; AC 7 references `GlyphDocTypeKey`.
- 🟡 **Clarity**: Storybook requirement vs no-Storybook note — **Resolved**. Requirements now mandates `/glyph-showcase` route; Technical Notes updated to match.
- 🟡 **Clarity**: Summary contradicts Technical Notes deferral — **Resolved**. Summary scoped to component+assets+tokens; consumer integration explicitly delegated to 0036/0040/0041/0042/0043/0053/0054/0055.
- 🔵 **Clarity**: Stale "Blocked by 0033" reference — **Resolved**. Sentence removed; Dependencies reframed as "Builds on 0033".
- 🔵 **Clarity**: Five-phase pipeline naming mismatch — **Resolved**. Phase list dropped from Context.
- 🔵 **Clarity**: Token sub-namespace name not normative — **Resolved**. `--ac-doc-<key>` promoted into Requirements with a 12-row colour table.
- 🔵 **Dependency**: 0038 coordination missing from Dependencies — **Resolved**. New "Coordinates with: 0038" entry added.
- 🔵 **Dependency**: Stale 0033 rationale — **Resolved**. See clarity counterpart.
- 🔵 **Dependency**: Storybook tooling dependency unresolved — **Resolved**. Showcase form decided (`/glyph-showcase` route).
- 🔵 **Scope**: `--ac-doc-*` token sub-namespace separable — **Resolved by acceptance**. Bundling justified explicitly; flagged only as a soft scope signal originally.
- 🔵 **Scope**: Final AC depends on out-of-scope work — **Resolved**. AC dropped; replaced with smoke-test AC; consumer list moved to new "Out of Scope / Downstream Verification" section.
- 🟡 **Testability**: Consumer-integration AC defers verification — **Resolved**. Smoke-test AC over 36 (docType, size) combinations replaces it.
- 🟡 **Testability**: Screenshot comparison procedure unspecified — **Partially resolved**. Colour comparison is now anchored to `var(--ac-doc-<key>)` tokens, but icon-shape comparison still has no objective procedure (still flagged major). Hex values are TBD in the colour table (newly flagged major).
- 🟡 **Testability**: Storybook-equivalent unverifiable — **Resolved**. `/glyph-showcase` AC names the route, grid, themes, and entry-point link.
- 🔵 **Testability**: "Without a component re-render" implementation-leaning — **Partially resolved**. Reworded to "within one paint frame, no React state change" but paint-frame timing is still hard to measure in JSDOM (still flagged minor).
- 🔵 **Testability**: Only `decisions` enumerated, colours not tabulated — **Resolved structurally** by the 12-row colour table; **incomplete in content** because hex columns are TBD (folded into the new "hex values TBD" major finding).
- 🔵 **Testability**: "Without raster artefacts" lacks defined check — **Resolved**. AC 2 now says "root element is `<svg>` with `viewBox` set (no `<img>` raster fallback)".
- 🔵 **Testability**: Default-decorative AC doesn't assert absence of role/aria-label — **Resolved**. AC 5 now says "carries neither `role` nor `aria-label`".

### New Issues Introduced

- 🟡 **Testability**: Icon shape correctness has no objective verification procedure (Acceptance Criteria #3 / Colour Token Table). AC 3 requires "icon shape from the updated screenshots" but no committed reference SVG, snapshot, or design-reviewer sign-off is specified. Two reviewers could legitimately disagree on a match.
- 🟡 **Testability**: Colour hex values are TBD, leaving the token contract unverifiable. The table is described as "the contract" but every cell is TBD; AC 1 binds fill to `var(--ac-doc-decisions)` etc., so token-binding can be checked but the actual *colour* cannot.
- 🔵 **Clarity**: "Both theme blocks" followed by three locations in the token sub-namespace requirement — count mismatch (Requirements).
- 🔵 **Clarity**: User-story "user" vs developer audience disambiguation could be tighter (Summary).
- 🔵 **Testability**: Smoke-test bar "without runtime error" is tautologically weak — a component returning `null` would pass (Acceptance Criteria).
- 🔵 **Testability**: `/glyph-showcase` "linked from the developer entry point (frontend README or in-app developer index)" is a disjunction without a named anchor (Acceptance Criteria).
- 🔵 **Dependency**: Canonical `DocTypeKey` source and screenshot assets are couplings named only in Technical Notes / References, not surfaced in Dependencies (Dependencies).
- 🔵 **Scope**: Showcase route is separable but cheap; cap its scope at the current AC ceiling (Requirements).
- 🔵 **Testability**: No AC verifies that `size` values outside `16 | 24 | 32` are rejected at the type level, mirroring AC 7's docType rejection (Acceptance Criteria).

### Assessment

Pass 1 fixed all four major clarity findings and two of the three major testability findings outright. The remaining REVISE verdict comes from a single underlying issue: **the visual contract (icon shapes + colour hex values) is sourced from screenshots, and no procedure is specified for either deriving the hex values before implementation or comparing rendered output against the screenshots at review time.** This residual gap fragments into two major testability findings.

Options to clear it:

1. **Eyedropper now**: Populate the hex columns in the colour token table before planning starts, and commit a `Glyph.tsx` SVG-path constant set (or per-doc-type SVG asset files) as a pre-implementation step so the table and an asset directory are both inspectable artefacts the implementer matches against.
2. **Pin a verification procedure**: Add an AC stating "rendered Glyph at 32px matches `library-view-updated-{light,dark}.png` within ΔE ≤ 2 on the icon's interior pixels, measured by Playwright visual-regression snapshot with a 0.5% pixel-diff threshold." This keeps the screenshot as the contract but makes the comparison objective.
3. **Defer formally**: Add an "Open Question / Pre-implementation step" that names eyedroppering + path-extraction as a required precursor task, blocking planning until done.

The minor and suggestion findings are housekeeping — sharpening the smoke-test AC, naming the showcase link anchor, surfacing `DocTypeKey`/screenshots as couplings in Dependencies, and adding the symmetric size-rejection AC.

## Re-Review (Pass 3) — 2026-05-12

**Verdict:** COMMENT — work item is acceptable for implementation; remaining findings are polish-level.

### Previously Identified Issues

- 🟡 **Testability**: Icon shape correctness has no objective verification procedure — **Resolved**. New "Visual Contract Verification" subsection pins a Playwright visual-regression spec at `e2e/glyph.spec.ts` with baselines under `e2e/__snapshots__/` and a `maxDiffPixelRatio: 0.005` threshold.
- 🟡 **Testability**: Colour hex values are TBD, leaving the token contract unverifiable — **Resolved procedurally**. New AC names the eyedropper procedure (`magick identify -format "%[hex:p{x,y}]"`, centre-pixel sampling, commit-logged coordinates) so the populated values are reproducible. Hex columns remain TBD as expected for a pre-implementation work item, but their derivation is now objectively specified.
- 🔵 **Clarity**: "Both theme blocks" / three locations count mismatch — **Resolved**. Now reads "in all three theme blocks of … light `:root`, the explicit `[data-theme=\"dark\"]` block, and the `@media (prefers-color-scheme: dark)` mirror".
- 🔵 **Clarity**: User-story "user" vs developer audience disambiguation — **Resolved**. Summary now opens "As an end user…" and explicitly notes the rest is for the implementer.
- 🔵 **Clarity**: LIFECYCLE_PIPELINE_STEPS wording still ambiguous — **Resolved**. Technical Notes bullet reworded to separate "Glyph supports all twelve" from "LIFECYCLE_PIPELINE_STEPS only references eleven of them".
- 🔵 **Testability**: Smoke-test "without runtime error" tautologically weak — **Resolved**. Vitest AC now asserts `<svg>` root, `width`/`height` attributes match `size`, and `getComputedStyle.fill` equals the populated table value.
- 🔵 **Testability**: Showcase link anchor disjunction under-specified — **Resolved**. AC now names "a 'Developer routes' section in `skills/visualisation/visualise/frontend/README.md`".
- 🔵 **Testability**: "Within one paint frame" hard to measure — **Resolved**. Reworded to "after one `requestAnimationFrame` yield, `getComputedStyle(glyphSvg).fill` resolves to … and no React state change has occurred (asserted via a render counter on the Glyph component)".
- 🔵 **Testability**: No symmetric size-type rejection AC — **Resolved**. New AC: "Given a `size` value not in `16 | 24 | 32` is passed, when TypeScript compiles, then the call is rejected at the type level."
- 🔵 **Dependency**: `DocTypeKey` source and screenshot assets not in Dependencies — **Resolved**. New "Reads from (artefact couplings)" entry under Dependencies names `types.ts` and the canonical screenshot paths with breakage conditions.
- 🔵 **Scope**: Showcase route separable but cheap; cap scope — **Resolved**. Showcase Requirements bullet now states "The showcase's scope is capped at this grid + theme view — any further features … belong in a separate work item".

### New Issues Introduced

- 🔵 **Clarity**: "Inner SVG" in Requirements bullets has no referent when AC 2 says the root element is an `<svg>` (Requirements). Implementer may introduce an unintended wrapper to justify "inner".
- 🔵 **Clarity**: "No React state change has occurred" in the theme-swap AC has broader scope than the parenthetical "render counter on the Glyph component" (Acceptance Criteria, theme-swap bullet).
- 🔵 **Clarity**: "Populated Colour Token Table value" in the smoke-test AC references a future state (the table currently shows TBD); explicit source (table vs `tokens.ts`) not stated (Acceptance Criteria, Vitest bullet).
- 🔵 **Testability**: Vitest and Playwright criteria depend on TBD hex values and Playwright baselines being produced first; no top-level ordering criterion makes that prerequisite explicit (Acceptance Criteria).
- 🔵 **Testability**: AC 3 says "Glyph renders correctly" — "correctly" is subjective; the trailing `fill` clause makes colour testable but shape parity is only covered by the Playwright AC, not this one (Acceptance Criteria).
- 🔵 **Testability**: Render-counter assertion needs a numeric threshold (0 vs 1 increments) and a note on StrictMode (Acceptance Criteria, theme-swap bullet).
- 🔵 **Testability**: Baseline derivation procedure under-specified — crop coordinates, grid layout, and DPI/scaling between prototype PNG and rendered showcase route are unstated (Visual Contract Verification, Playwright bullet).

### Assessment

The work item is in good shape for implementation. All 7 major findings from pass 1 and both major findings from pass 2 are resolved; dependency and scope lenses returned zero findings this pass. The remaining 7 findings are minor/suggestion polish — clarifying referents ("inner SVG", "populated table"), pinning numeric thresholds (render-counter count, StrictMode disclaimer), and tightening the baseline-derivation procedure. None of them block planning or implementation; they would surface as nice-to-have edits during PR review.

A planner can proceed from this work item as-is. Optional follow-up edits before implementation starts:

1. Replace "inner SVG" → "rendered `<svg>`" in the two accessibility Requirements bullets.
2. Tighten AC 4 to "the Glyph component does not re-render (verified via a render counter attached to Glyph, run outside StrictMode)".
3. Reword AC 3's "renders correctly" → "mounts without error" and explicitly delegate shape parity to the Playwright AC.
4. Add a top-level prerequisite line under Visual Contract Verification: "The eyedropper step (populating hex columns) and the Playwright baseline commit run before the Vitest and Playwright criteria are evaluated".
5. Specify baseline derivation precisely — either "baselines captured from `/glyph-showcase` after the eyedropper step" or named crop rectangles.
