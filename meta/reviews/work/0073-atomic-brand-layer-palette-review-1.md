---
date: "2026-05-23T13:43:40Z"
type: work-item-review
producer: review-work-item
target: "work-item:0073"
work_item_id: "0073"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
id: "0073-atomic-brand-layer-palette-review-1"
title: "0073-atomic-brand-layer-palette-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-23T13:43:40Z"
last_updated_by: Toby Clemson
---

## Work Item Review: 0073 Atomic Brand-Layer Palette

**Verdict:** REVISE

The work item is well-structured, internally consistent, and tightly scoped to a single coherent unit of work — introducing the `--atomic-*` brand-layer palette and rewiring exact-hex matches in the `--ac-*` semantic layer. All standard sections are present and substantively populated, and the story is appropriately sized. However, three acceptance criteria (AC1, AC2, AC5) lack the procedural precision needed for a verifier to produce a deterministic pass/fail decision, and several implicit couplings (ADR-0026 governance, the prototype HTML as a frozen source-of-truth, visual-regression infrastructure) are not surfaced as Dependencies entries.

### Cross-Cutting Themes

- **AC2 "exact match" is under-specified** (flagged by: clarity, testability) — both the comparison procedure (case sensitivity, shorthand expansion, RGB vs string equality) and the timing of "currently-resolved" (story start vs PR open vs merge) are ambiguous, allowing two reviewers to produce different rewrite sets and both claim AC2 passes.
- **ΔE threshold in AC5 is undefined** (flagged by: clarity, testability) — neither the ΔE variant (CIE76 vs CIE2000), the aggregation rule (mean vs max vs 95th percentile), nor the computation tool is specified, leaving the escape clause subjective.
- **`~30 tokens` count is approximate where AC1 demands exhaustive coverage** (flagged by: clarity, testability) — the contrast between "~30" and AC1's "all" leaves an unbounded expected set that cannot be deterministically verified.
- **Overlap with 0077 on overlay/shadow tokens** (flagged by: scope, dependency) — overlay tokens included in this story could collide with 0077's audit, creating a latent scope-bleed or ordering coupling that is acknowledged but not bounded.

### Findings

#### Major

- 🟡 **Testability**: AC1 uses unbounded "all" without freezing the expected token set
  **Location**: Acceptance Criteria (AC1)
  AC1 defers the expected set to a moving source (the prototype HTML) rather than enumerating it. If the prototype gains or loses a token mid-implementation, the criterion silently changes scope, and reviewers cannot detect a missed token without re-deriving the expected set themselves.

- 🟡 **Testability**: AC2's "exactly matches" comparison procedure is undefined
  **Location**: Acceptance Criteria (AC2)
  Hex equality could mean string-identical, normalised lowercase, expanded shorthand, or RGB-tuple equality — each yields different rewrite sets. The Clarity lens additionally flagged that "currently-resolved" is timestamp-ambiguous (story start vs PR open vs merge), compounding the issue if brand values shift mid-implementation.

- 🟡 **Testability**: AC5 ΔE < 5 escape clause weakens the pass/fail boundary
  **Location**: Acceptance Criteria (AC5)
  ΔE is not bound to a colour space (CIE76 vs CIE2000 differ materially), an aggregation rule, a sampling region, or a tool. The criterion devolves into reviewer judgement on what "side-by-side comparison" is adequate. The Clarity lens raised the same concern as a minor.

#### Minor

- 🔵 **Clarity**: "BigGlyph" used as a proper noun without definition
  **Location**: Summary / Dependencies
  A reader who has not seen 0082 cannot tell whether BigGlyph is a component, a feature, or an external library. Weakens the rationale for the story for anyone not already familiar with 0082.

- 🔵 **Dependency**: ADR-0026 governance coupling not listed in Dependencies
  **Location**: Dependencies
  ADR-0026 is described in Technical Notes as a binding constraint ("should be consulted before introducing any exception to the var(--atomic-X) rewrite rule"), but is not surfaced in Dependencies, so planners scanning that section will miss the governance coupling.

- 🔵 **Dependency**: Prototype HTML source-of-truth coupling not captured as a dependency
  **Location**: Dependencies
  Assumptions states designer changes to brand values would invalidate visual-diff baselines and require re-baselining, making the prototype file a real coupling. Without a captured dependency the team has no signal to coordinate with whoever may edit the prototype.

- 🔵 **Dependency**: Visual-regression baseline regeneration pathway not captured as a coupling
  **Location**: Acceptance Criteria (AC5)
  AC5 allows for baselines that require regeneration, which depends on the Playwright visual-regression project being runnable in the implementer's environment and on a defined regeneration path. This infrastructure dependency is not surfaced.

- 🔵 **Testability**: "The suite passes" lacks a defined invocation
  **Location**: Acceptance Criteria (AC3, AC4)
  AC3 and AC4 say the test suites "pass" without specifying the command, environment, or CI configuration under which "pass" is observed, allowing a "passes locally but not in CI" ambiguity.

- 🔵 **Testability / Clarity**: Token-count claim "~30 tokens" is not testable
  **Location**: Requirements / Acceptance Criteria (AC1)
  The Summary and Requirements use "~30" while AC1 demands "all". Reviewers have no anchor count against which to detect missing tokens beyond the drift detector itself.

- 🔵 **Dependency**: Ordering relationship with 0077 not made explicit
  **Location**: Dependencies
  Since this story declares overlay tokens, whichever of 0073/0077 ships first sets the convention the other must follow. The Dependencies entry for 0077 notes overlap but not ordering.

#### Suggestions

- 🔵 **Scope**: Visual-regression AC may pull in 0077's shadow/overlay concerns
  **Location**: Acceptance Criteria (AC5) / Dependencies
  If overlay tokens prove theme-dependent, satisfying AC5 could require resolving 0077's concerns inside this story. Add an explicit out-of-scope note that defers overlay/shadow theme-dependence to 0077, mirroring AC2's "stays a literal" escape hatch.

### Strengths

- ✅ All standard sections (Summary, Context, Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes, References) are present and substantively populated.
- ✅ Story-kind requirements satisfied: explicit user-role framing ("As a design system maintainer, I want…") and five concrete acceptance criteria.
- ✅ Cross-section scope is internally consistent — Summary, Requirements, and Acceptance Criteria all converge on the same unit of work.
- ✅ Explicit scope boundary on the rewrite rule (AC2 + Drafting Notes: "exact resolved-hex match only; near-matches stay as literals and are documented in the PR") prevents scope creep into colour harmonisation.
- ✅ Specialised terms (`--ac-*`, `--atomic-*`, `--code-*`, `--tk-*`) are anchored to concrete file paths and line references.
- ✅ Assumptions explicitly call out theme-invariance and one-to-one mapping premises rather than leaving them inferred.
- ✅ Drift-detection extension (AC4) is committed in-scope with an explicit cost/benefit rationale, signalling deliberate scoping rather than accidental omission.
- ✅ Upstream blocker (0033), downstream consumer (0082), and related story (0077) are all named in Dependencies.

### Recommended Changes

1. **Pin AC1's expected token set** (addresses: AC1 unbounded "all", "~30 tokens" not testable)
   Either enumerate the brand token names in AC1 / an appendix, or rewrite AC1 as "fixture JSON token count equals the count parsed from `prototype-standalone.html` and lists every `--atomic-*` declaration found inline". Replace "~30" in Summary and Requirements with the exact count once enumerated.

2. **Specify AC2's comparison procedure and timing baseline** (addresses: AC2 comparison undefined, AC2 timing implicit)
   Add a sentence to AC2 such as: "Hex values are compared after normalising to lowercase six-digit form; the resolved-value baseline is the state of `global.css` on `main` at the time the implementation branch is cut." Optionally bind verification to a small node script that reports unrewritten exact matches.

3. **Tighten or drop AC5's ΔE clause** (addresses: AC5 ΔE escape clause, ΔE undefined)
   Either drop the ΔE escape entirely (require zero baseline regeneration without explicit approval) or specify it precisely — e.g., "max per-pixel ΔE2000 < 5 over the changed regions, computed via [named tool], reported per regenerated baseline".

4. **Expand the Dependencies section** (addresses: ADR-0026 coupling, prototype HTML coupling, visual-regression infrastructure, 0077 ordering)
   Add entries for: (a) ADR-0026 as a governance/constrained-by coupling; (b) `prototype-standalone.html` as a frozen source-of-truth artefact; (c) the Playwright visual-regression infrastructure as a tooling pre-condition; (d) an explicit ordering note vs 0077 for overlay tokens.

5. **Define test invocations in AC3/AC4** (addresses: "the suite passes" lacks invocation)
   Add a one-line invocation reference, e.g., "verified via `npm test src/styles/global.test.ts src/styles/prototype-tokens.fixture.test.ts` with CI green on the PR branch".

6. **Define BigGlyph on first use** (addresses: BigGlyph proper noun without definition)
   Add a brief parenthetical gloss at the first Summary mention or link 0082 inline.

7. **Add a Non-Goals / out-of-scope note for theme-dependent overlays** (addresses: AC5 scope bleed into 0077)
   Mirror AC2's escape hatch: if overlay/shadow tokens prove theme-dependent, they are deferred to 0077 and the affected `--ac-*` tokens remain literals in this story.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Generally well-written with clear referents, consistent scope, and explicit cross-references. A few minor ambiguities remain around acronyms (BigGlyph, ΔE) and one passive construction in AC2, but every statement has a single reasonable interpretation for a domain-aware reader.

**Strengths**:
- Summary clearly identifies actor and motivation in user-story form.
- Cross-section scope is internally consistent across Context, Requirements, and Acceptance Criteria.
- Specialised terms anchored to concrete file paths and line references.
- "Exact resolved-hex match only" rule stated identically across Requirements, AC2, and Drafting Notes.
- Assumptions explicitly call out theme-invariance and one-to-one mapping premises.

**Findings**:
- 🔵 minor / high — ΔE threshold used without definition or link (AC5 / Technical Notes)
- 🔵 minor / medium — "BigGlyph" used as proper noun without definition (Summary / Dependencies)
- 🔵 minor / medium — Passive "is rewritten" leaves trigger and timing implicit (AC2)
- 🔵 minor / medium — "~30 tokens" imprecise where an exact count is implied elsewhere (Requirements / AC1)

### Completeness

**Summary**: Thoroughly populated story with all expected sections present and substantively filled. Frontmatter intact with recognised kind/status/priority. Kind-specific content satisfied — explicit user-story framing plus five concrete acceptance criteria.

**Strengths**:
- All standard sections present and substantively populated, including Assumptions and Open Questions.
- Story-kind requirements satisfied with explicit user-role framing and five concrete ACs.
- Context explains motivating gap (0033 excluded `--atomic-*`) rather than restating the summary.
- Frontmatter complete with recognised values.
- Technical Notes provides implementer-ready provenance details.

**Findings**: none.

### Dependency

**Summary**: Captures primary upstream blocker (0033), downstream consumer (0082), and related parallel story (0077). However, several couplings — ADR-0026 governance, the prototype HTML source-of-truth, and the visual-regression baseline regeneration pathway — are referenced in body sections but not surfaced as Dependencies entries.

**Strengths**:
- Upstream blocker on 0033 captured with rationale.
- Downstream consumer 0082 named as Blocks entry.
- Related work 0077 noted with clear scope-overlap caveat.
- ADR-0026 referenced in Technical Notes and References.

**Findings**:
- 🔵 minor / medium — ADR-0026 governance coupling not listed in Dependencies
- 🔵 minor / high — Prototype HTML source-of-truth coupling not captured as a dependency
- 🔵 minor / medium — Visual-regression baseline regeneration pathway not captured as a coupling
- 🔵 minor / low — Ordering relationship with 0077 not made explicit

### Scope

**Summary**: Single coherent unit of work — introducing the `--atomic-*` brand-layer palette and rewiring `--ac-*` to reference it. Appropriately sized for a story (CSS + TS mirror + two test suites + visual-regression baselines) but tightly bounded by the exact-hex-match rewrite rule. Scope boundaries are explicit and the story is independently deliverable.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria all describe the same scope.
- Explicit scope boundary on the rewrite rule prevents scope creep.
- Drift detection (AC4) justified as in-scope with stated rationale.
- Dependencies bound the work cleanly.
- Story kind fits the scope — one design-system increment.

**Findings**:
- 🔵 suggestion / medium — Visual-regression AC may pull in 0077's shadow/overlay concerns (AC5 / Dependencies)

### Testability

**Summary**: ACs largely testable with concrete artefacts and procedural verification paths. However, AC2's "exactly matches" lacks a defined comparison procedure, AC5's ΔE < 5 introduces a subjective override path, and AC1's brand-token enumeration relies on the prototype source without freezing the expected token set.

**Strengths**:
- AC3 and AC4 name specific test files and fixture artefacts.
- AC5 anchors visual-regression verification to existing Playwright infrastructure.
- AC2 includes explicit escape-hatch for residual literals.
- ACs framed as observable outcomes rather than implementation prescriptions.

**Findings**:
- 🟡 major / high — AC1 uses unbounded "all" without freezing the expected token set
- 🟡 major / high — AC2's "exactly matches" comparison procedure is undefined
- 🟡 major / medium — AC5 ΔE < 5 escape clause weakens the pass/fail boundary
- 🔵 minor / medium — "The suite passes" lacks a defined invocation (AC3, AC4)
- 🔵 minor / medium — Token-count claim "~30 tokens" is not testable (Requirements / AC)

## Re-Review (Pass 2) — 2026-05-23T13:43:40Z

**Verdict:** APPROVE

All three major findings from pass 1 are resolved or substantially addressed; no findings at or above the major threshold remain. The work item is acceptable for implementation. Remaining minor findings are either deeper procedural nits surfaced by the tightening, or new minors introduced by the changes themselves (notably the new `culori` library dependency and the "changed regions" scoping for ΔE2000).

### Previously Identified Issues

#### Major
- 🟡 **Testability**: AC1 uses unbounded "all" without freezing the expected token set — **Resolved**. AC1 now pins enumeration to `prototype-tokens.json` with a fixture/prototype parity test as the verification step.
- 🟡 **Testability**: AC2's "exactly matches" comparison procedure is undefined — **Resolved**. Requirements and AC3 now specify lowercase six-digit hex normalisation and pin the reference snapshot to `global.css` on `main` at branch-cut.
- 🟡 **Testability**: AC5 ΔE < 5 escape clause weakens the pass/fail boundary — **Partially resolved**. ΔE is now bound to ΔE2000 / CIEDE2000 with a named library; the "changed regions" scope and aggregation rule remain underspecified (downgraded to minor in this pass).

#### Minor / Suggestion
- 🔵 **Clarity**: "BigGlyph" used as proper noun without definition — **Resolved**. Summary now glosses BigGlyph and links 0082 on first use.
- 🔵 **Dependency**: ADR-0026 governance coupling not listed in Dependencies — **Resolved**. Dependencies now has a "Governed by" entry.
- 🔵 **Dependency**: Prototype HTML source-of-truth coupling not captured — **Resolved**. Now explicitly declared frozen for the duration of implementation.
- 🔵 **Dependency**: Visual-regression infrastructure not captured as a coupling — **Resolved**. Now listed as a tooling pre-condition.
- 🔵 **Testability**: "The suite passes" lacks a defined invocation — **Resolved**. AC4 names the exact `npm test --` invocation.
- 🔵 **Testability / Clarity**: "~30 tokens" not testable / imprecise — **Resolved**. Replaced with fixture-anchored enumeration.
- 🔵 **Dependency**: Ordering relationship with 0077 not made explicit — **Partially resolved**. Now stated as a "whichever ships first sets the convention" rule; only the 0073-first branch is fully described (clarity flagged this in pass 2 as a minor).
- 🔵 **Scope**: Visual-regression AC may pull in 0077's shadow/overlay concerns — **Resolved**. New Non-Goals section defers theme-dependent overlays to 0077.

### New Issues Introduced

- 🔵 **Clarity**: ΔE2000 / CIEDE2000 used without inline definition (AC5) — readers unfamiliar with perceptual colour metrics have no anchor for the threshold.
- 🔵 **Clarity**: "Whichever story ships first" leaves the 0077-first branch undescribed (Dependencies / Related & ordering).
- 🔵 **Clarity**: "Mirror it" referent in Requirements bullet 1 is briefly ambiguous between palette and naming convention.
- 🔵 **Dependency**: Type-tinted iconography named in Summary as a downstream consumer but has no Blocks entry.
- 🔵 **Dependency**: `culori` (or equivalent CIEDE2000 implementation) is a new external library not captured in Dependencies.
- 🔵 **Testability**: AC1 parsing rule for "every `--atomic-*` declaration" from the prototype HTML is unspecified.
- 🔵 **Testability**: AC3's PR-description rationale escape hatch has no required shape, so the enumeration cannot be conclusively verified.
- 🔵 **Testability**: AC5's "changed regions" scope and aggregation rule (max vs mean) remain undefined despite the ΔE2000 specification.
- 🔵 **Testability**: AC4's "CI run is green" broadens verification beyond the named test files to all CI jobs.
- 🔵 **Scope** (suggestion): Drift-detector extension and visual-regression coverage are separable concerns bundled in scope; current in-scope justification is reasonable but flagged as a follow-up split candidate if the story grows.

### Assessment

The work item is ready for implementation. The three major testability gaps that motivated the REVISE verdict in pass 1 are addressed; what remains are procedural nits and library-capture follow-ups that are reasonable to handle either inline during implementation (e.g. capturing `culori` in `package.json` when AC5's regeneration path is first exercised) or as PR-time additions (e.g. defining the "changed regions" scope when the first regeneration is needed). None of the residual findings block planning or kick-off.
