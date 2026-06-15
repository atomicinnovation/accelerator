---
type: plan-review
id: "2026-06-15-0111-visualiser-frontend-closeout-fixes-review-1"
title: "Plan Review: Visualiser Frontend Closeout Fixes"
date: "2026-06-15T20:57:16+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-06-15-0111-visualiser-frontend-closeout-fixes"
target: "plan:2026-06-15-0111-visualiser-frontend-closeout-fixes"
reviewer: Toby Clemson
verdict: APPROVE
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "standards", "compatibility"]
review_number: 1
review_pass: 3
tags: ["visualiser", "frontend", "markdown", "lifecycle", "sidebar", "milestone-closeout"]
last_updated: "2026-06-15T21:20:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Visualiser Frontend Closeout Fixes

**Verdict:** REVISE

This is a well-researched, low-risk closeout pass — six independently
mergeable phases, each anchored to verified file/line references and an
established codebase precedent. The design-level reasoning is sound,
particularly the L2 parity-contract analysis and the retained-boolean
separation. However, the plan misses two classes of **mechanical
enforcement** the project relies on, and as written **three of the six
phases would ship red** against their own "tests pass" success criteria:
Phase 1 and Phase 2 break the `migration.test.ts` token-ledger gates, and
Phase 6's test-update enumeration omits at least four breaking
`toHaveLength(8)` assertions. None of these are deep design flaws — they
are completeness gaps — but they must be closed before implementation.

### Cross-Cutting Themes

- **Incomplete Phase 6 test enumeration** (flagged by: correctness,
  compatibility, test-coverage) — Removing `decisions` drops the rendered
  stage count from 8 to 7, but the plan's Phase 6 §4 list misses
  `Pipeline.test.tsx:13`, `PipelineMini.test.tsx:15`, and
  `LifecycleIndex.test.tsx:198-209` (`toHaveLength(8)`), plus the
  `data-stage="decisions"` assertion at `LifecycleIndex.test.tsx:124-128`
  (which would throw on a null query after removal). Three independent
  lenses found these. As written, `mise run test:unit:frontend` fails.

- **Project's mechanical token-enforcement harness overlooked** (flagged
  by: standards) — `src/styles/migration.test.ts` is a per-file,
  exact-count `EXCEPTIONS` ledger plus a "var() references resolve to
  declared tokens" gate. Phase 1's new `9px`/`1px` literals and Phase 2's
  new `8px` literal and undeclared `--code-scrollbar-*` tokens both break
  it. The plan never mentions updating the ledger or `tokens.ts`.

- **The L2 strategy choice (shared-model edit vs view-layer omit)**
  (flagged by: architecture, code-quality) — The plan deliberately chose
  strategy A (edit the shared cross-language model) over the
  research-recommended and prototype-used strategy B (view-layer filter).
  This has a wider blast radius than the cluster-scoped AC implies.

- **The retained `has_decision` boolean and the testability of "decisions
  still surface"** (flagged by: test-coverage, code-quality, correctness)
  — After L2, `has_decision`/`hasDecision` is set and serialised but drives
  no rendering, and the AC's "decision still appears in related-artifacts"
  cannot be asserted in the named unit file (that surface lives in
  `LibraryDocView`, not `LifecycleClusterView`).

### Tradeoff Analysis

- **L2 coupling vs fixture stability (strategy A vs B)**: Architecture and
  code-quality both recommend the prototype's strategy B (filter
  `decisions` only at the lifecycle view + its denominator), which keeps the
  shared model and Rust `STAGE_PUSH_ORDER` intact and bounds the change to
  one surface. The plan chose strategy A (edit the shared model in
  lockstep), whose stated rationale is avoiding churn to 10+ `hasDecision`
  fixtures — but strategy B would not require that churn either. The
  counter-argument for A is that it makes "decisions is not a pipeline
  stage" canonical on *every* surface (kanban, index, `/dev`), not just the
  lifecycle cluster. **Recommendation**: decide explicitly which semantics
  you want — if "decisions is genuinely never a stage anywhere", keep A but
  add kanban + index VR coverage and document the cross-surface intent; if
  "lifecycle excludes decisions but the model is unchanged", switch to B.

### Findings

#### Critical

- 🔴 **Standards**: Phase 1 table/hr px literals break the exact-count `EXCEPTIONS` ledger
  **Location**: Phase 1, Section 2: Table + hr CSS
  The new CSS adds `9px` (×2) and `1px` (×4) literals to
  `MarkdownRenderer.module.css`, whose ledger entries are currently `9px`
  count 1 and `1px` count 7. The "declared count equals observed count"
  hygiene test fails on both under- and over-count, so Phase 1's
  `test:unit:frontend` criterion cannot be met without bumping the ledger
  (or tokenising the literals).

- 🔴 **Standards**: Phase 2 scrollbar tokens are undeclared and the `8px` literal has no ledger entry
  **Location**: Phase 2, Sections 1+2: scrollbar tokens and rules
  The new `--code-scrollbar-thumb`/`--code-scrollbar-track` are added to
  `global.css` but not to `CODE_SURFACE_TOKENS` in `tokens.ts`, so the
  "var() references resolve to declared tokens" gate fails; separately the
  `8px` scrollbar height has no `EXCEPTIONS` entry for this file. Two
  distinct migration-harness gates break — Phase 2 ships red.

#### Major

- 🟡 **Correctness**: Phase 6 omits three breaking `toHaveLength(8)` test assertions
  **Location**: Phase 6, Section 4: Test updates
  `Pipeline.test.tsx:13` ("renders exactly eight stage tiles",
  `toHaveLength(8)`), `PipelineMini.test.tsx:15` (`toHaveLength(8)`), and
  `LifecycleIndex.test.tsx:198-209` ("renders 8 pipeline tiles per card",
  a distinct test from the cited `:104,112-123` range) will all fail once
  the model drops to 7 steps. Phase 6 is not mergeable as written.

- 🟡 **Correctness**: Missed `data-stage="decisions"` assertion + possible numerator drift in `LifecycleIndex.test.tsx`
  **Location**: Phase 6, Section 4: Test updates
  Beyond the cited copy edits, `LifecycleIndex.test.tsx:124-128` reads
  `querySelector('[data-stage="decisions"]')!.getAttribute(...)`; after
  removal that query returns null and the non-null assertion throws.
  The fixture's expected counter may also need to drop its numerator (e.g.
  `4/8` → `3/7`, not just `4/7`) depending on whether its `present` set
  included `decisions`.

- 🟡 **Architecture / Code Quality**: L2 strategy A alters a model shared by three surfaces, not just the cluster
  **Location**: Phase 6 (L2); What We're NOT Doing
  `WORKFLOW_PIPELINE_STEPS` / `completeness.present` are also consumed by
  `Pipeline` (lifecycle index cards) and `PipelineMini` (kanban work-item
  cards). Removing `decisions` silently drops its tile/dot and changes
  `present.size` on the kanban board and lifecycle index — surfaces outside
  the work item's L2 scope, and the plan's VR list omits kanban baselines.

- 🟡 **Architecture / Code Quality**: Divergence from the frozen prototype's strategy B is under-justified
  **Location**: Phase 6 (L2); What We're NOT Doing
  The research recommended, and the prototype uses, the view-layer omit
  (strategy B). The plan's only stated reason for choosing A is avoiding
  `hasDecision` fixture churn — which B would not incur either. Baking a
  view-specific concern into the shared cross-language model erodes the
  model/view separation the prototype established.

- 🟡 **Test Coverage**: "Decision still surfaces in related-artifacts" cannot be tested in the named file
  **Location**: Phase 6, Section 4: Test updates
  The plan puts the "decision still appears in related-artifacts"
  regression assertion in `LifecycleClusterView.test.tsx`, but that view
  renders no related-artifacts surface (`RelatedArtifacts` lives in
  `LibraryDocView`). Once `decisions` leaves `WORKFLOW_PIPELINE_STEPS`, the
  fixture's decision entry renders *nowhere* in `LifecycleClusterContent`,
  so the most behaviourally important half of AC L2 would be left to manual
  verification only.

- 🟡 **Test Coverage**: The Rust-side L2 change is effectively unverified
  **Location**: Phase 6, Section 2: Rust stage-push order
  No `clusters.rs` test asserts `decisions` in a `present` vector
  (decisions are orphan-by-design), so removing the `STAGE_PUSH_ORDER`
  tuple makes `has_decision` a dead field with **no test failing**. The
  parity test only checks the TypeScript `CANONICAL_PRESENT_ORDER`, not
  that `STAGE_PUSH_ORDER` stopped emitting decisions.

- 🟡 **Standards / Code Quality**: New `9px` cell paddings should use `--sp-2` per ADR-0026 §2
  **Location**: Phase 1, Section 2 Notes
  `9px` is 1px from `--sp-2` (8px), so ADR-0026 §2's ±2px tolerance band
  mandates substituting the token rather than propagating the borderline
  `.task` literal to two new sites. Using `var(--sp-2) var(--sp-3)` removes
  the literals and the ledger churn entirely.

#### Minor

- 🔵 **Correctness**: `DevDesignSystem.tsx` has no hardcoded `8` to replace; its sibling `toHaveLength(8)` is status badges
  **Location**: Phase 6, Section 3
  `DevDesignSystem.tsx` already uses `WORKFLOW_PIPELINE_STEPS.length`
  (no literal `8` at `~:908`). And `DevDesignSystem.test.tsx:267`'s
  `toHaveLength(8)` is the **8 statuses**, not stage tiles — it must NOT be
  changed. (Note: this directly contradicts the compatibility lens's
  suggestion to bump `:267` to 7 — correctness's deeper read is
  authoritative here.)

- 🔵 **Correctness**: aria-label numerator (`present.size`) can exceed the new 7-stage denominator
  **Location**: Phase 6, Section 3
  `present.size` includes the three long-tail keys, so a fully-populated
  cluster yields "10 of 7 stages complete". Pre-existing ("10 of 8") but
  Phase 6 edits these exact lines and widens the discrepancy; filtering the
  numerator against `WORKFLOW_PIPELINE_STEPS` (as `stagesComplete` already
  does) would fix it.

- 🔵 **Test Coverage**: No assertion guards the self-deriving aria-label denominator
  **Location**: Phase 6, Section 3
  The visible `N/7` copy is asserted, but the Pipeline aria-label string
  ("of 7 stages") is not — an off-by-one in the `.length`-based
  interpolation would pass undetected for screen-reader users.

- 🔵 **Test Coverage**: M1/M2/M3 are guarded only by full-image VR snapshots
  **Location**: Phase 1 §3 & Phase 2
  Load-bearing structural facts — the `.tableWrap` carrying
  `overflow: hidden` (the literal AC), the `hr` resolving to `--ac-stroke`
  — could be pinned cheaply by a resolved-styles/DOM test; a pixel diff can
  pass within `maxDiffPixelRatio: 0.05` even if the wrapper is missing.

- 🔵 **Test Coverage**: L3 verbatim wording isn't pinned by an exact-string assertion
  **Location**: Phase 4
  `LifecycleIndex.test.tsx:82-95` asserts the *old* copy; if updated loosely
  (substring/case-insensitive) the verbatim-wording AC (ASCII apostrophe,
  US "artifacts", semicolon) would not actually be enforced.

- 🔵 **Test Coverage**: L1 VR fixture may not exercise the long-title boundary
  **Location**: Phase 3
  L1's failure mode is a *long title* compressing `.actions`; nothing
  guarantees the existing `library-doc-view` fixture renders a title long
  enough to reproduce the original wrap, so the regression could recur
  untested.

- 🔵 **Code Quality**: Retained `has_decision`/`hasDecision` becomes an orphaned representation
  **Location**: Phase 6, Section 2
  After L2 the field is computed and serialised but feeds no production
  rendering path — a dead-data smell. A one-line comment at `clusters.rs:20`
  and `types.ts:244` would mark it as a deliberate fixture/wire-stability
  retention rather than an oversight.

- 🔵 **Code Quality**: Unresolved selector question (`.codeblock pre` vs `.markdown pre`)
  **Location**: Phase 2, Section 2
  The plan leaves "confirm whether `.codeblock pre` inherits or needs the
  selector" to implementation time. `.codeblock pre` does not reset
  `overflow-x`, so it shares the `.markdown pre` scroll surface — a single
  rule set covers both; state this to avoid redundant duplication.

- 🔵 **Compatibility**: Pinned Chromium may render the standard scrollbar, not the WebKit 8px thumb
  **Location**: Phase 2 (M2)
  Modern Chromium honours `scrollbar-width`/`scrollbar-color` and ignores
  the `::-webkit-scrollbar` pseudo-elements when both are present, so the
  captured baseline may reflect a thin radius-less thumb rather than the 8px
  WebKit thumb the manual-verification criterion describes. Align the
  wording with what the runner actually produces.

- 🔵 **Compatibility**: `present` is itself a wire field whose contents change
  **Location**: Phase 6, Section 2; Migration Notes
  The serialised `present: string[]` will stop containing `"decisions"`.
  The embedded SPA (sole consumer) stays consistent because its readers gate
  on `WORKFLOW_PIPELINE_STEPS`, but Migration Notes should name `present`
  *content* (not just the arrays) as part of the contract being changed.

- 🔵 **Standards**: Muted META block (~0.525 net opacity) needs a WCAG contrast check
  **Location**: Phase 5 (L4)
  The Templates `<Link>` at ~0.525 effective opacity over the sidebar (dark
  `--ac-fg-faint` is `#6c7088`) risks falling below WCAG 1.4.3 4.5:1. Add a
  composited-contrast check to the manual verification, even if it matches
  the frozen prototype.

- 🔵 **Architecture**: L1 global guards on shared `Page` primitives carry mild coupling
  **Location**: Phase 3 (L1)
  `white-space: nowrap` on `.btn` and `flex-shrink: 0` on `.actions` become
  system-wide invariants to fix a LibraryDocView-only symptom. Acceptable
  (benign defaults), but the plan should record that it is intentionally
  changing a shared primitive's contract rather than scoping via
  `data-slot="actions"`.

#### Suggestions

- 🔵 **Architecture**: Self-deriving denominator only half-closes the drift risk
  **Location**: Phase 6, Section 3
  `WORKFLOW_PIPELINE_STEPS.length` removes frontend drift, but the
  cross-language coupling still relies on a hand-maintained
  `CANONICAL_PRESENT_ORDER` and Rust `STAGE_PUSH_ORDER`. No change needed
  for this work item (the parity test guards it); note for future work that
  the durable fix is generating one side from the other.

- 🔵 **Standards**: "1px/28px-equivalent" note is ambiguous given the exact-count gate
  **Location**: Phase 1, Section 2 Notes
  The final CSS contains no 28px value; replace the prose note with an
  explicit list of every px/rem literal and its target `EXCEPTIONS` count so
  the harness update is mechanical and verifiable.

### Strengths

- ✅ Phases are independently mergeable and ordered lowest-risk-first, with
  the only cross-language change (L2) isolated and sequenced last — a sound
  decomposition that bounds blast radius.
- ✅ Every phase reuses a verified existing precedent rather than inventing
  a mechanism: the `.codeblock` overflow-hidden wrapper for M1's rounded
  table, the dual WebKit + Firefox token-routed scrollbar from
  `RootLayout.module.css` for M2, and the multiplying `.phaseHeading`
  opacity for L4.
- ✅ The L2 retained-boolean reasoning is internally consistent:
  `canonical_rank` Decisions ⇒ 7 drives only the intra-cluster sort, not
  `present`-membership, so decisions still clusters and surfaces while
  losing stage membership.
- ✅ The cross-language parity contract (frontend `LIFECYCLE_PIPELINE_STEPS`
  ↔ Rust `STAGE_PUSH_ORDER` ↔ `CANONICAL_PRESENT_ORDER`) is correctly
  identified and edited in lockstep, with the parity test reduced to 10
  entries matching the 11→10 reduction.
- ✅ Replacing the hardcoded `8`/`/8` denominators with
  `WORKFLOW_PIPELINE_STEPS.length` fixes the exact drift class this work
  item is addressing so it cannot recur on the frontend.
- ✅ The RCA-exclusion claim is verified accurate (`u8::MAX` in
  `canonical_rank`, absent from `STAGE_PUSH_ORDER`, never iterated by
  `buildTimeline`), correctly treated as zero code change plus a regression
  test.
- ✅ Adds new VR specs for M1 tables + M3 horizontal rules, closing a real
  zero-coverage gap rather than relying on baseline regeneration alone.

### Recommended Changes

1. **Add the missing migration-harness updates to Phases 1 and 2**
   (addresses: both Standards criticals) — In Phase 1, either bump the
   `MarkdownRenderer.module.css` `9px` count to 3 and `1px` count to 11 in
   the `EXCEPTIONS` ledger (with reasons), or tokenise the paddings to
   `var(--sp-2) var(--sp-3)` (preferred — see #2). In Phase 2, add
   `--code-scrollbar-thumb`/`--code-scrollbar-track` to `CODE_SURFACE_TOKENS`
   in `tokens.ts` (and check `prototype-tokens.fixture.test.ts` drift since
   `--code-*` is the prototype-adopted theme-invariant family), and add an
   `8px` `EXCEPTIONS` entry for the file.

2. **Tokenise the table cell padding to `--sp-2`/`--sp-3`** (addresses: the
   `9px` Standards/Code-Quality major) — Eliminates the new literals and
   the ledger churn entirely; reserve `9px` only if a design sign-off records
   why 8px is unacceptable.

3. **Complete the Phase 6 §4 test-update enumeration** (addresses: the
   Correctness majors, Compatibility minor) — Add `Pipeline.test.tsx:13`
   (retitle), `PipelineMini.test.tsx:15`, and `LifecycleIndex.test.tsx:198-209`
   (8→7); explicitly remove/replace the `data-stage="decisions"` assertion
   at `LifecycleIndex.test.tsx:124-128`; recompute the newer cluster's
   expected counter from its actual workflow-present entries; and **exclude**
   `DevDesignSystem.test.tsx:267` (status badges, not stages) and drop the
   spurious `DevDesignSystem.tsx:~908` edit (already self-deriving).

4. **Relocate the "decision still surfaces" assertion and add a Rust guard**
   (addresses: the two Test-Coverage majors) — Put the "decision still in
   related-artifacts" assertion where that surface lives
   (`LibraryDocView`/`RelatedArtifacts` unit test or `tokens.spec` VR), and
   in `LifecycleClusterView.test.tsx` assert only the negative. Add a
   `clusters.rs` assertion that `has_decision = true` yields a `present`
   vector without `"decisions"`, so the Rust change is locked in.

5. **Resolve the L2 strategy decision explicitly** (addresses: the two
   Architecture/Code-Quality majors) — Either (a) keep strategy A but
   document the deliberate cross-surface intent and add kanban + lifecycle
   index VR coverage to the closeout, or (b) switch to the prototype's
   strategy B (view-layer omit) to bound the change to the lifecycle view.

6. **Fix the aria-label numerator/denominator domain mismatch** (addresses:
   the Correctness minor) — While editing `Pipeline.tsx:29`/
   `PipelineMini.tsx:14`, filter the numerator against
   `WORKFLOW_PIPELINE_STEPS` so it shares the domain of the new denominator
   and never reads "10 of 7".

7. **Pin verbatim/structural facts cheaply where VR is the only guard**
   (addresses: Test-Coverage minors) — Assert the L3 subtitle verbatim;
   add a resolved-styles check for `.tableWrap { overflow: hidden }` and the
   `hr` colour; confirm an L1 long-title fixture exists.

8. **Add the small documentation/safety touches** (addresses: Code-Quality
   and Standards minors) — Comment the retained `has_decision` field as a
   deliberate retention; resolve the `.codeblock pre` selector question in
   the plan; add a WCAG composited-contrast check to Phase 5; align Phase 2's
   manual-verification wording with the scrollbar the pinned Chromium
   actually renders.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: A low-risk, well-researched closeout pass: six of seven fixes
are CSS-, wrapper-, or string-level edits that respect established patterns
(the `.codeblock` rounding-wrapper precedent for M1, the dual WebKit+Firefox
token-routed scrollbar precedent for M2, the multiplying-opacity
`.phaseHeading` precedent for L4). The single architecturally significant
decision is L2, where the plan deliberately chose strategy A (edit the shared
cross-language `LIFECYCLE_PIPELINE_STEPS` / `STAGE_PUSH_ORDER` / `present`
model) over strategy B (the prototype's view-layer omit). That choice changes
the semantics of a model shared by three surfaces and the plan does not fully
trace that blast radius.

**Strengths**:
- Phases independently mergeable, ordered lowest-risk-first, L2 isolated last.
- M1 reuses the `.codeblock` overflow-hidden wrapper-for-rounding pattern;
  `border-collapse: collapse` makes the wrapper genuinely mandatory.
- M2 routes the always-dark scrollbar through theme-invariant `--code-*`
  tokens and follows the dual WebKit + Firefox precedent in
  `RootLayout.module.css`.
- L2 preserves `hasDecision`/`has_decision` and `canonical_rank` so decisions
  still cluster and surface in related-artifacts.
- The cross-language parity contract is correctly identified and edited
  together in Phase 6, guarded by the parity test.

**Findings**:
- 🟡 **major** (high) — *Strategy A changes a model shared by three surfaces,
  not just the lifecycle cluster* (Phase 6 / What We're NOT Doing):
  `WORKFLOW_PIPELINE_STEPS` / `completeness.present` are also consumed by
  `Pipeline` (lifecycle index cards) and `PipelineMini` (kanban cards).
  Removing `decisions` drops its tile/dot and changes `present.size` on
  surfaces outside L2's scope; the VR list omits kanban baselines. Either
  accept and add kanban+index VR coverage, or reconsider strategy B.
- 🟡 **major** (medium) — *Unjustified divergence from the frozen reference's
  stated design stance* (Phase 6 / What We're NOT Doing): research recommended
  and the prototype uses strategy B; the plan's only rationale for A is
  avoiding `hasDecision` churn, which B would not require either. Document the
  rationale or adopt B.
- 🔵 **minor** (high) — *Global guards on shared `Page` primitives carry mild
  cross-surface coupling* (Phase 3): `white-space: nowrap` / `flex-shrink: 0`
  become system-wide invariants; acceptable but should be a recorded decision
  vs scoping via `data-slot="actions"`.
- 🔵 **suggestion** (medium) — *Self-deriving denominator improves
  evolutionary fitness but only on the frontend side* (Phase 6 §3): the
  cross-language duplication (`CANONICAL_PRESENT_ORDER` + `STAGE_PUSH_ORDER`)
  remains hand-maintained; durable fix is generating one side from the other.

### Code Quality

**Summary**: Well-structured: six independently mergeable phases anchored to
verified references and established precedents. Most phases are simple
CSS/string edits with proportionate rigour. The main maintainability concerns
sit in Phase 6 (L2): the plan chose the higher-coupling shared-model strategy A
over the research-recommended view-layer strategy B, and leaves
`has_decision`/`hasDecision` as an orphaned representation set and serialised
but no longer driving any rendering.

**Strengths**:
- Each phase reuses an existing verified pattern rather than inventing one.
- Phase 6 fixes a latent maintainability bug by making the stage count
  self-deriving (`WORKFLOW_PIPELINE_STEPS.length`).
- The cross-language parity contract is called out as a lockstep three-place
  edit, respecting the single-source-of-truth design.
- Phases sized proportionately; cross-language L2 ordered last and flagged
  highest-risk.

**Findings**:
- 🟡 **major** (high) — *L2 chooses the higher-coupling shared-model edit over
  the lower-coupling view-layer omit* (What We're NOT Doing / Phase 6): spreads
  one conceptual change across `api/types.ts`, `clusters.rs`, the parity test,
  Pipeline/PipelineMini, LifecycleIndex, `/dev`, and several test files;
  reconsider B or document why A's broader blast radius is preferred.
- 🔵 **minor** (high) — *Retained `has_decision`/`hasDecision` becomes an
  orphaned representation* (Phase 6 §2): after the change the field feeds no
  production render path; add a comment at `clusters.rs:20` and `types.ts:244`
  marking the deliberate retention.
- 🔵 **minor** (medium) — *Unresolved selector question (`.codeblock pre` vs
  `.markdown pre`)* (Phase 2 §2): `.codeblock pre` inherits `overflow-x`, so a
  single `.markdown pre` rule set covers both; state this to avoid redundant
  duplication.
- 🔵 **minor** (medium) — *Unexplained literal `9px` paddings reduce
  self-documentation* (Phase 1 §2): the `9px` + `var(--sp-3)` mix reads as
  arbitrary; comment it as a deliberate value or tokenise it.

### Test Coverage

**Summary**: Generally strong: identifies the cross-language parity contract,
names specific tests to update with verified line numbers, proposes new VR
specs for previously unguarded surfaces, and adds explicit L2 regression tests.
But the highest-risk fix (L2) has a coverage gap — the "decision still appears
in related-artifacts" assertion cannot be made in the named unit file, the
Rust-side change is effectively unverified, and the new self-deriving
denominators lack an aria-label assertion.

**Strengths**:
- L2 calls for TDD and names the precise tests/lines including the parity
  anchor.
- Adds new VR specs for M1/M3 (zero coverage today).
- Adds positive no-decision-node regression and an RCA-absence guard.
- Correctly scopes the VR blast radius per phase (L4 full-page, L2
  tokens.spec).
- Each phase independently mergeable and green.

**Findings**:
- 🟡 **major** (high) — *"Decision still appears in related-artifacts" is
  untestable in the named file* (Phase 6 §4): `LifecycleClusterView` renders
  no related-artifacts surface (`RelatedArtifacts` is in `LibraryDocView`);
  once `decisions` leaves the steps, the fixture's decision renders nowhere in
  the cluster view. Relocate the assertion; assert only the negative in the
  cluster test.
- 🟡 **major** (high) — *The Rust present-vector test update is a no-op; the
  change is unverified* (Phase 6 §2): no `clusters.rs` test asserts decisions
  in `present`, so removing the tuple makes `has_decision` dead with no test
  failing. Add an assertion that `has_decision = true` yields a `present`
  without `"decisions"`.
- 🔵 **minor** (high) — *No assertion guards the self-deriving aria-label
  denominator* (Phase 6 §3): assert "of 7 stages" alongside the visible `N/7`.
- 🔵 **minor** (medium) — *VR-only coverage for M1/M2/M3* (Phase 1 §3 / Phase
  2): add resolved-styles/DOM assertions for `.tableWrap { overflow: hidden }`
  and the `hr` colour to complement the VR.
- 🔵 **minor** (medium) — *L3 verbatim wording* (Phase 4): update
  `LifecycleIndex.test.tsx:82-95` to assert the new title + full subtitle
  verbatim.
- 🔵 **minor** (medium) — *L1 long-title boundary* (Phase 3): confirm/add a
  fixture with a deliberately long title so the no-wrap guard is exercised.

### Correctness

**Summary**: Logically sound at the design level: the three-way parity
contract is correctly understood, retaining `has_decision` and
`canonical_rank` Decisions ⇒ 7 is consistent, and the numerator/denominator
refactor is correct where applied. But Phase 6's enumeration of breaking sites
is incomplete: at least three hardcoded `toHaveLength(8)` / `data-stage` test
assertions that will fail are not listed, and one cited edit target (a
hardcoded 8 in `DevDesignSystem.tsx`) does not exist. As written, Phase 6
would not satisfy its own "full unit suites pass" criterion.

**Strengths**:
- Retained-boolean reasoning correct: `canonical_rank` drives intra-cluster
  sort, not `present`-membership.
- `CANONICAL_PRESENT_ORDER` edit correct (11→10); parity test's second
  assertion still holds.
- `stagesComplete` already derived by filtering `present` against
  `WORKFLOW_PIPELINE_STEPS`, so it auto-adjusts; only the literal `/8` needs
  changing.
- Pipeline/PipelineMini already iterate `WORKFLOW_PIPELINE_STEPS`; only the
  aria-label literal is stale.
- RCA-exclusion claim verified (`u8::MAX`, absent from `STAGE_PUSH_ORDER`,
  not iterated by `buildTimeline`).

**Findings**:
- 🟡 **major** (high) — *Phase 6 omits three breaking `toHaveLength(8)`
  assertions* (Phase 6 §4): `Pipeline.test.tsx:13`, `PipelineMini.test.tsx:15`,
  `LifecycleIndex.test.tsx:198-209`; the plan's own `test:unit:frontend`
  criterion would fail. (Agent body used 🔴; structured severity is major.)
- 🟡 **major** (high) — *Missed `data-stage="decisions"` assertion and
  numerator drift in `LifecycleIndex.test.tsx`* (Phase 6 §4): `:124-128`
  throws on a null query after removal; the newer cluster's counter may need
  `3/7` not `4/7`.
- 🔵 **minor** (high) — *`DevDesignSystem.tsx` has no hardcoded 8; a sibling
  `toHaveLength(8)` is unrelated* (Phase 6 §3): `:908` is already
  self-deriving; `DevDesignSystem.test.tsx:267` is the 8 statuses and must not
  be touched.
- 🔵 **minor** (medium) — *aria-label numerator (`present.size`) can exceed the
  new 7-stage denominator* (Phase 6 §3): yields "10 of 7"; filter the numerator
  against `WORKFLOW_PIPELINE_STEPS`.
- 🔵 **minor** (medium) — *The instructed Rust present-vector test update is a
  no-op as written* (Phase 6 §2): no such test exists; reword and consider
  adding a real regression test.

### Standards

**Summary**: Respects most documented token conventions (correctly uses
existing `--size-115`/`--size-125`/`--radius-6`/`--radius-4`, places new
scrollbar tokens `:root`-only per ADR-0026 §5, cites the right ADRs), but
overlooks the project's mechanical enforcement harness `migration.test.ts` —
a per-occurrence exact-count `EXCEPTIONS` ledger plus a "references only
declared tokens" gate. Several new literals (`9px`, `1px`, `8px`) and the two
new `var(--code-scrollbar-*)` references will fail it as written, and the plan
never mentions updating the ledger or `tokens.ts`.

**Strengths**:
- Correctly reuses existing `--size-125`/`--size-115` (no new font-size
  literal) — compliant with ADR-0043.
- Routes thumb radius through `var(--radius-4)` and places `--code-scrollbar-*`
  `:root`-only per ADR-0026 §5.
- L2's self-deriving denominator removes a magic-number smell.
- Reuses the `.codeblock` wrapper and the dual WebKit+Firefox scrollbar
  precedent.
- Semantic table markup, theme-reactive `hr`, lockstep aria-label updates.

**Findings**:
- 🔴 **critical** (high) — *Phase 1 table/hr px literals break the exact-count
  `EXCEPTIONS` ledger* (Phase 1 §2): two new `9px` (→3) and four new `1px`
  (→11) fail the hygiene test; bump the ledger or tokenise.
- 🔴 **critical** (high) — *Phase 2 scrollbar tokens undeclared + `8px` has no
  ledger entry* (Phase 2 §1+2): `--code-scrollbar-*` not added to
  `CODE_SURFACE_TOKENS` in `tokens.ts` fails the declared-token gate; `8px`
  has no `EXCEPTIONS` entry. Two gates break.
- 🟡 **major** (medium) — *New `9px` cell paddings should use `--sp-2` per
  ADR-0026 §2* (Phase 1 §2 Notes): `9px` is within the ±2px tolerance band of
  `--sp-2`; use `var(--sp-2) var(--sp-3)` to remove the literals and churn.
- 🔵 **minor** (medium) — *Muted META block (~0.525 net opacity) WCAG contrast*
  (Phase 5): the Templates link risks falling below 4.5:1; add a composited
  contrast check to manual verification.
- 🔵 **suggestion/minor** (low) — *"1px/28px-equivalent" note is ambiguous*
  (Phase 1 §2 Notes): no 28px value exists in the CSS; replace the prose with
  an explicit literal inventory and target `EXCEPTIONS` counts.

### Compatibility

**Summary**: The central compatibility surface — Phase 6's cross-language
parity contract between the frontend (`api/types.ts`) and the Rust server
(`clusters.rs`) — is handled well: all three coupled loci are identified and
edited in lockstep, and the `hasDecision`/`has_decision` wire field is
retained. The change alters the behavioural contract of the serialised
`present` array (no longer contains `"decisions"`), but the only consumer is
the embedded SPA. The M2 dual WebKit + Firefox declaration is a safe additive
cross-browser improvement.

**Strengths**:
- Treats the frontend↔Rust stage model as a single contract; parity test
  updated to 10 entries.
- `Completeness.hasDecision` retained on both sides — no removed-field break;
  serde camelCase output stays aligned with the TS interface.
- M2 adds standard `scrollbar-width`/`scrollbar-color` additively alongside
  the WebKit pseudo-elements.
- Correctly verifies RCA already maps to `u8::MAX` and is absent from
  `STAGE_PUSH_ORDER` — no RCA data shape change.

**Findings**:
- 🔵 **minor** (high) — *Phase 6 omits four `toHaveLength(8)` assertions*
  (Phase 6 §4): `Pipeline.test.tsx:13`, `PipelineMini.test.tsx:15`,
  `DevDesignSystem.test.tsx:267`, `LifecycleIndex.test.tsx:207`. (Note:
  correctness lens establishes `DevDesignSystem.test.tsx:267` is status badges
  — exclude it; the rest stand.)
- 🔵 **minor** (medium) — *`present` is itself a wire field whose contents
  change* (Phase 6 §2 / Migration Notes): a cluster with a decision still emits
  `hasDecision: true` but drops `"decisions"` from `present`; only out-of-tree
  readers of `present` would be affected. Confirm the SPA is the sole consumer
  and note `present` content in Migration Notes.
- 🔵 **minor** (medium) — *Pinned Chromium may apply standard scrollbar props,
  not the WebKit thumb* (Phase 2): the captured baseline may reflect a thin
  radius-less scrollbar rather than the 8px WebKit thumb; align the
  manual-verification wording with reality when regenerating baselines.

## Re-Review (Pass 2) — 2026-06-15

**Verdict:** REVISE

The pass-1 edits resolved **every** pass-1 finding. But re-running all six lenses
against the revised plan caught **three new critical issues introduced by the
revision itself** — two faulty token-ledger counts and one wrong field in the new
aria-label numerator snippet. All three were verified against source and
**corrected immediately after this re-review** (see Assessment); a pass-3
confirmation is recommended but the corrections are mechanical and high-confidence.

### Previously Identified Issues

- 🔴 **Standards**: Phase 1 px literals break the `EXCEPTIONS` ledger — **Resolved**
  (padding tokenised to `--sp-2`; explicit ledger step added — though the step's
  arithmetic was itself wrong, see new issues).
- 🔴 **Standards**: Phase 2 scrollbar tokens undeclared + `8px` literal —
  **Resolved** (new §1b declares both in `CODE_SURFACE_TOKENS`; §3 adds the `8px`
  entry).
- 🟡 **Correctness**: Phase 6 omits breaking `toHaveLength(8)` assertions —
  **Resolved** (full enumeration: `Pipeline.test.tsx:13`, `PipelineMini.test.tsx:15`,
  `LifecycleClusterView.test.tsx:86`, `LifecycleIndex.test.tsx:104/112-123/198-209`).
- 🟡 **Correctness**: Missed `data-stage="decisions"` query + numerator drift —
  **Resolved** (`:124-128` removal listed; counter pinned).
- 🟡 **Architecture / Code Quality**: L2 shared-model blast radius —
  **Resolved** ("Cross-surface scope (intentional)" statement + kanban/index VR
  added to closeout).
- 🟡 **Architecture / Code Quality**: Divergence from prototype's strategy B —
  **Resolved** (documented as a deliberate cross-surface decision).
- 🟡 **Test Coverage**: "Decision still in related-artifacts" untestable in named
  file — **Resolved** (relocated; pass 2 refined the anchor to `LibraryDocView`
  integration — now corrected in plan).
- 🟡 **Test Coverage**: Rust-side change unverified — **Resolved** (assertion
  added; pass 2 refined it to target the push helper directly — now corrected).
- 🟡 **Standards / Code Quality**: `9px` literal — **Resolved** (tokenised).
- 🔵 All pass-1 minors (has_decision comment, `.codeblock pre` selector, WCAG
  contrast, aria-label assertion, L3 verbatim, L1 long-title fixture,
  DevDesignSystem `:267` exclusion, drop `:908` edit, `present` wire-contract
  note, Chromium scrollbar wording) — **Resolved**.

### New Issues Introduced

- 🔴 **Correctness**: The new `workflowComplete` numerator snippet (Phase 6 §3)
  queried `present.has(s.key)`, but `present` holds kebab-case `docType` strings
  (`Pipeline.tsx:32` uses `present.has(step.docType)`); `s.key` is the camelCase
  `PipelineStepKey`, so it would match nothing and render "0 of 7". **Corrected**
  to `present.has(s.docType)`.
- 🔴 **Standards / Code Quality**: Phase 1 §4 said bump the `1px` `EXCEPTIONS`
  count to **11**, but the rewrite *removes* the existing per-cell `1px` border
  (`MarkdownRenderer.module.css:66`), so the net is **10** (7 − 1 + 4). The
  exact-equality hygiene gate would fail at 11. **Corrected** to 10 with a reason
  rewrite.
- 🔴 **Standards / Code Quality**: Tokenising the cell padding removes the file's
  only `0.4rem` occurrence, orphaning its `count: 1` `EXCEPTIONS` entry
  (`migration.test.ts:304-310`) — a second hygiene-gate failure the ledger step
  did not address. **Corrected** by adding an explicit delete-the-`0.4rem`-entry
  instruction.
- 🔵 **Standards**: The §1b "check `prototype-tokens.fixture.test.ts` drift"
  instruction was misdirected (that test iterates the prototype fixture, not
  `CODE_SURFACE_TOKENS`). **Corrected** to point at the `global.test.ts:113`
  parity gate, which the dual declaration already satisfies.
- 🔵 **Correctness**: `PipelineMini.test.tsx:8` title still said "eight"; only the
  `:15` assertion was listed for change. **Corrected** (retitle added).
- 🔵 **Correctness**: The newer-cluster counter was left ambiguous (`3/7` or
  `4/7`); the fixture `present` (`:58-64`) contains `decisions`, so it is
  definitively **`3/7`**. **Corrected** (pinned).
- 🔵 **Test Coverage**: The new Rust assertion would be vacuous via `compute()`
  (clustering prevents a decision co-residing); must target the push helper.
  **Corrected**. Also added: a wide-table M1 fixture so the `overflow: hidden`
  clip boundary is actually exercised.

### Assessment

Pass 2 did its job: it caught that the pass-1 revision, while closing all original
findings, introduced three new green-gate/behaviour regressions (the `s.key`
numerator, the `1px` count of 11, the orphaned `0.4rem` entry) plus several
refinements. All were verified against the live source and corrected in the plan
immediately after this re-review. The plan is now believed sound and internally
consistent; a pass-3 confirmation would verify no further regressions, but the
remaining work is mechanical implementation against precise, source-verified
instructions.

## Re-Review (Pass 3) — 2026-06-15

**Verdict:** APPROVE

Pass 3 re-ran the four lenses that received corrective edits (correctness,
standards, code-quality, test-coverage), each tasked with confirming the specific
pass-2 corrections against the live source. **All corrections verified correct;
the standards lens returned zero findings.** No new critical or major issue. The
three pass-2 regressions are confirmed fixed:

- ✅ Numerator now keys on `present.has(s.docType)` — matches `Pipeline.tsx:32`;
  `s.key` would have rendered "0 of 7".
- ✅ `1px` count of **10** verified exact (7 occurrences today − 1 removed at `:66`
  + 4 new = 10); the `0.4rem` entry deletion confirmed (single occurrence at `:66`).
- ✅ `8px` is genuinely new (no existing entry); the `global.test.ts:113` parity
  gate is the correct target for the two new declared tokens.
- ✅ Newer-cluster counter `3/7` confirmed against the fixture `present`
  (`:58-64`); PipelineMini `:8` retitle target exists; full 8→7 enumeration
  complete with `DevDesignSystem.test.tsx:267` (status badges) correctly excluded.
- ✅ Rust assertion correctly retargeted at `derive_completeness` (non-vacuous);
  related-artifacts assertion correctly anchored at `LibraryDocView` integration.

### Previously Identified Issues

- 🔴 (pass-2) `s.key` numerator — **Resolved** (→ `s.docType`).
- 🔴 (pass-2) `1px` count of 11 — **Resolved** (→ 10).
- 🔴 (pass-2) orphaned `0.4rem` entry — **Resolved** (deletion step added).
- 🔵 (pass-2) misdirected prototype-fixture check — **Resolved** (→ `global.test.ts:113`).
- 🔵 (pass-2) PipelineMini title, `3/7` counter, Rust-assertion target,
  related-artifacts anchor, wide-table fixture — **all Resolved**.

### New Issues Introduced

Four **minor/suggestion** items only — all applied immediately after this pass:

- 🔵 **Correctness**: `LifecycleClusterView.test.tsx:115` test title becomes
  inaccurate post-L2 (wording only) — **applied** (reword instruction added).
- 🔵 **Code Quality**: Rust-assertion wording referenced a `Completeness` input,
  but `derive_completeness` takes `&[IndexEntry]` — **applied** (tightened to
  `derive_completeness(&[entry(DocTypeKey::Decisions, …)])`).
- 🔵 **Test Coverage**: the aria-label "N of 7" guard should use a *partial*
  fixture (with a long-tail key) to actually exercise the numerator-domain fix —
  **applied** (success criterion now specifies a partial fixture asserting "3 of 7").
- 🔵 **Test Coverage**: Phase 1 §5 named a non-existent `tests/e2e/resolved-styles.spec.ts`
  — **applied** (repointed to a new spec under `tests/resolved-styles/`).

### Assessment

The plan is **sound and ready for implementation**. Three review passes converged:
pass 1 closed the original two criticals + six majors, pass 2 caught and fixed
three regressions the pass-1 edits introduced, and pass 3 confirmed every
correction against source and cleared the only critical lens (standards) with zero
findings. The residual pass-3 items were trivial polish, now applied. Every
instruction is anchored to a verified file/line reference; remaining work is
mechanical execution. Recommended next step: `/implement-plan`.