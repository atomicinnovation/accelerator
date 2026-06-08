---
type: work-item-review
id: "0095-markdown-checkboxes-always-dark-mode-styled-review-1"
title: "Work Item Review: Theme-Reactive Markdown Task-List Checkboxes"
date: "2026-06-08T21:51:31+00:00"
author: "Toby Clemson"
producer: review-work-item
status: complete
target: "work-item:0095"
work_item_id: "0095"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: [visualiser, markdown, theme, dark-mode, bug]
last_updated: "2026-06-08T22:51:14+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Theme-Reactive Markdown Task-List Checkboxes

**Verdict:** REVISE

This is a strong, well-bounded bug work item — internally consistent across all
sections, appropriately atomic, with exact DOM structure and named tokens that
make most outcomes inspectable. The verdict is REVISE solely because two `major`
testability findings cross the configured major-count threshold (2): the
"match the prototype design reference" and "adequate contrast" acceptance
criteria lack defined pass/fail procedures. These are narrow, mechanical fixes
to the Acceptance Criteria rather than structural problems — the work item is
close to ready.

### Cross-Cutting Themes

- **Subjective vs. objective acceptance criteria** (flagged by: testability, clarity) — The work item shines where it asserts named tokens and concrete DOM, but several criteria fall back on undefined judgement words ("match", "adequate", "visible") that can be argued either way at sign-off. The same imprecision shows up in clarity's note that the Summary's `color-scheme` description is looser than the Technical Notes. Tightening the criteria to inspectable, token- or threshold-based checks resolves the bulk of the findings.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: "Match the prototype design reference" has no defined pass/fail procedure
  **Location**: Acceptance Criteria
  The criterion specifies no procedure for what counts as a match — pixel-equivalent, visually equivalent, or structurally equivalent — so two reviewers could reach opposite verdicts. Suggest a visual-regression snapshot within tolerance, or an enumerated list of visual properties (box dimensions, border weight, tick shape, fill colour).

- 🟡 **Testability**: "Adequate border/background contrast" lacks a measurable threshold
  **Location**: Acceptance Criteria
  "Adequate" defines no threshold, so a verifier cannot produce a definitive pass/fail and the box could be claimed compliant even when barely visible. Suggest verifying the named tokens are used (`--ac-stroke-strong` border on `--ac-bg-card`), or stating a minimum WCAG contrast ratio.

#### Minor

- 🔵 **Completeness**: Bug lacks explicit, structured reproduction steps
  **Location**: Context
  The defect is described narratively but never laid out as input → action → expected → actual. A reader cannot deterministically reproduce the original defect before fixing it; the "may paint" phrasing leaves trigger conditions implicit.

- 🔵 **Dependency**: Sibling bug 0094 cross-references this item but the coupling is only "Related"
  **Location**: Dependencies
  0094 and 0095 touch the same `MarkdownRenderer`/theme-token surface, but the link carries no note on whether the fixes land independently or share refactoring — risking duplicated or conflicting CSS work if scheduled separately.

- 🔵 **Testability**: "No regression to the FilterPill" is near-tautological without a defined check
  **Location**: Acceptance Criteria
  Names no specific property to verify, so a real regression could pass review unnoticed. Suggest pointing at a concrete guard (existing FilterPill tests/snapshots continue to pass unchanged).

- 🔵 **Testability**: Tick visibility/colour not verifiable beyond "visible"
  **Location**: Acceptance Criteria
  Technical Notes state the tick is a hardcoded `#fff` on the accent fill, but the criterion only says "visible" and never asserts the tick colour — leaving its theme-correctness (the exact contrast failure mode the bug is about) unverified.

#### Suggestions

- 🔵 **Clarity**: Summary's `color-scheme` description is looser than Technical Notes
  **Location**: Summary
  Summary says `color-scheme` is "left at the permissive `light dark` default in light theme", which could read as "light theme has its own permissive setting"; in fact light theme simply has no dedicated rule and inherits `:root`. Align the wording.

- 🔵 **Clarity**: Acronym "GFM" used without definition
  **Location**: Dependencies
  "GFM" appears in the 0076 entry without expansion. Spell out as "GitHub Flavored Markdown" on first use, or lean on the already-named `remark-gfm` package.

- 🔵 **Dependency**: Prototype design's "approved/agreed" state is asserted, not linked
  **Location**: Technical Notes / Assumptions
  The approach depends on the prototype design being "the agreed solution", but the agreement is in prose with no decision-record anchor. If an ADR/approval artifact exists, link it; otherwise note the prototype is the de facto source of truth.

- 🔵 **Scope**: Done-state label treatment is an acknowledged scope expansion bundled with the glyph fix
  **Location**: Assumptions
  Muting + striking labels is a distinct concern from the theme-unreactive control, but the two are cohesive (same element, same prototype) and the expansion is self-documented. No action required; splitting is optional.

### Strengths

- ✅ Scope is stated identically across Summary, Context, Requirements, Acceptance Criteria, Assumptions, and Drafting Notes — a reader cannot misread the boundary.
- ✅ Every standard section is present and substantively populated; even Open Questions is explicitly resolved rather than left blank, and frontmatter is complete and valid (kind=bug, status=draft).
- ✅ Requirements name exact DOM structure and specific tokens (`--ac-stroke-strong`, `--ac-bg-card`, `--ac-accent`, `--ac-fg-muted`), making several outcomes inspectable rather than impressionistic.
- ✅ Boundaries against the FilterPill component are explicit and called out as out of scope in Requirements, Technical Notes, and an acceptance criterion.
- ✅ Drafting Notes transparently capture the title change and scope expansion, so the item's evolution is legible to a reviewer.
- ✅ The bug's mechanism and expected-vs-actual behaviour are precisely stated, giving a verifier a clear baseline; Dependencies pins the prototype to exact files/lines.

### Recommended Changes

1. **Define a pass/fail procedure for "match the prototype design reference"** (addresses: "Match the prototype design reference" has no defined pass/fail procedure)
   Replace with either a visual-regression snapshot check within the project's pixel-diff tolerance in both themes, or enumerate the specific visual properties that must match (box dimensions, 1.5px border, tick shape, accent fill).

2. **Make "adequate contrast" objectively checkable** (addresses: "Adequate border/background contrast" lacks a measurable threshold)
   Drop "adequate" in favour of verifying the named tokens (`--ac-stroke-strong` border on `--ac-bg-card`), or state a minimum WCAG contrast ratio between the border and adjacent surface per theme.

3. **Tighten the FilterPill and tick criteria** (addresses: "No regression to the FilterPill" is near-tautological; Tick visibility/colour not verifiable)
   Point the FilterPill criterion at the existing FilterPill tests/snapshots passing unchanged, and assert the tick renders `#fff` on the `--ac-accent` fill and stays legible in both themes.

4. **Add a short reproduction block** (addresses: Bug lacks explicit, structured reproduction steps)
   State input (markdown with `- [ ]` / `- [x]`), action (render in light theme), expected (light-appropriate checkbox), actual (dark/low-contrast native control).

5. **Clarify the 0094 coupling and minor clarity points** (addresses: Sibling bug 0094 coupling; Summary `color-scheme` wording; GFM acronym)
   Note in the 0094 entry whether the fixes share the `MarkdownRenderer` surface; align the Summary's `color-scheme` wording with Technical Notes; expand or drop the bare "GFM" acronym.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: This work item is unusually clear and internally consistent. The scope (replace native checkbox with a token-driven glyph and add done-state label treatment) is stated identically across Summary, Context, Requirements, Acceptance Criteria, Assumptions, and Drafting Notes; pronouns resolve unambiguously; and domain terms like 'theme-reactive', 'token-driven', and 'glyph' are defined in Context. The only clarity issues are minor: one undefined acronym and a subtly inconsistent characterisation of the colour-scheme behaviour between Summary and Technical Notes.

**Strengths**:
- Scope expansion (done-state label mute + strike-through) is flagged consistently across six sections, so a reader cannot misread the boundary.
- Domain vocabulary is defined or anchored in Context with a cross-reference to the established pattern (0037).
- Pronouns resolve to a single explicit referent throughout.
- Drafting Notes explicitly reconcile the prior (incorrect) framing with the corrected one.

**Findings**:
- 🔵 suggestion (medium) — Summary | Summary says color-scheme is 'pinned to dark for the dark theme', Technical Notes says it is set per-theme with no light forcing. The Summary's "left at the permissive default in light theme" could be read as light theme having its own setting, when in fact it inherits `:root` because no light-specific rule exists. Align the Summary wording with the Technical Notes.
- 🔵 suggestion (high) — Dependencies | Acronym 'GFM' used without definition in the 0076 entry. Expand on first use, or rely on the already-named `remark-gfm` package reference.

### Completeness

**Summary**: This bug work item is unusually well-populated: every standard section is present and substantively filled, and the frontmatter is complete and valid. The main completeness gap for a bug is the absence of explicit, structured reproduction steps (input/action/expected/actual) — the defect is described narratively but never laid out as a reproducible sequence.

**Strengths**:
- All expected sections present and substantively populated — no placeholders; Open Questions explicitly resolved.
- Frontmatter complete and valid (kind=bug, status=draft, priority=medium, id/title/dates/tags present).
- Context explains the forces (the data-theme/--ac-* token contract and why the native control violates it).
- Requirements are specific and implementable without author follow-up.
- Drafting Notes transparently capture the title change and scope expansion.

**Findings**:
- 🔵 minor (high) — Context | Bug lacks explicit, structured reproduction steps. The defect is described narratively but there is no step-by-step input/action/expected/actual sequence. Add a short Reproduction block.

### Dependency

**Summary**: This bug work item has well-captured couplings: it explicitly names the prototype design it must match, the FilterPill pattern it follows, and a Dependencies section listing five related work items with clear rationale. The defect is internal (CSS/rendering) with no external systems implied. The only notable gap is that the cross-reference with sibling bug 0094 is captured only as 'Related' rather than as an explicit ordering relationship.

**Strengths**:
- Dependencies enumerates five related work items with a one-line rationale each.
- The prototype design dependency is captured in References and pinned to exact files/lines in Technical Notes.
- FilterPill correctly identified as a sibling pattern and explicitly scoped out.
- The defect is purely internal — absence of external/vendor/cross-team couplings is correct, not a gap.

**Findings**:
- 🔵 minor (medium) — Dependencies | Sibling bug 0094 cross-references this item but the coupling is only 'Related'. Both touch the same `MarkdownRenderer`/theme-token surface; add a note on whether the fixes are independent or share refactoring.
- 🔵 suggestion (low) — Technical Notes | Prototype design is a named upstream artefact but its 'approved' state is asserted, not linked. If a decision record exists, reference it; otherwise note the prototype is the de facto source of truth.

### Scope

**Summary**: This is a well-bounded bug work item describing a single coherent change — replacing native markdown task-list checkboxes with a custom token-driven glyph so they become theme-reactive. All three core sections describe the same scope, boundaries against FilterPill are explicit, and the work is appropriately sized for a single increment. The one scope-relevant signal is the explicitly acknowledged expansion to include done-state label treatment.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria describe a single unified scope with no mismatch.
- Boundaries stated explicitly — FilterPill named as out of scope across multiple sections.
- The scope expansion is surfaced honestly in Assumptions and Drafting Notes rather than buried.
- Appropriately atomic: a single self-contained component change one team can deliver and verify as one increment.

**Findings**:
- 🔵 suggestion (medium) — Assumptions | Done-state label treatment is an acknowledged scope expansion bundled with the glyph fix. The two concerns are cohesive (same element, same prototype) so bundling is defensible; splitting into a sibling item is optional. No action required.

### Testability

**Summary**: This bug work item is unusually well-specified for testability: most Acceptance Criteria define concrete, observable outcomes under explicitly named conditions (checked/unchecked, light/dark). The main weaknesses are one criterion that relies on visual comparison to an external design reference and a couple of terms ('adequate contrast', 'no regression') that lack a defined pass/fail threshold. The bug's trigger and expected-vs-actual behaviour are clearly stated.

**Strengths**:
- Most criteria are Given/When/Then with explicit preconditions and concrete observable outcomes.
- The bug's mechanism, expected, and actual behaviour are precisely stated, giving a clear baseline.
- Requirements name exact DOM structure and specific tokens, making outcomes inspectable.
- Open Questions records that the color-scheme/accent-color question is moot, removing an ambiguity.

**Findings**:
- 🟡 major (high) — Acceptance Criteria | "Match the prototype design reference" has no defined pass/fail procedure. No procedure for what counts as a match — pixel/visual/structural — so reviewers could reach opposite verdicts. Specify a visual-regression snapshot within tolerance, or enumerate visual properties.
- 🟡 major (high) — Acceptance Criteria | "Adequate border/background contrast" lacks a measurable threshold. "Adequate" defines no threshold; verify the named tokens are used, or state a minimum WCAG contrast ratio.
- 🔵 minor (medium) — Acceptance Criteria | "No regression to the FilterPill" is near-tautological without a defined check. Point at a concrete guard (existing FilterPill tests/snapshots pass unchanged).
- 🔵 minor (medium) — Acceptance Criteria | Tick visibility/colour not verifiable beyond "visible". Technical Notes state the tick is hardcoded `#fff` on the accent fill; assert the tick colour and legibility in both themes.

## Re-Review (Pass 2) — 2026-06-08

**Verdict:** COMMENT

Lenses re-run: clarity, completeness, dependency, testability (scope skipped — its
sole pass-1 finding was an explicit "no action required" and nothing scope-related
changed). Both pass-1 `major` findings are resolved and no new major or critical
findings were introduced, so the verdict moves from REVISE to COMMENT. The work
item is acceptable for implementation; the remaining items are optional polish.

### Previously Identified Issues

- 🟡 **Testability**: "Match the prototype design reference" has no defined pass/fail procedure — **Resolved.** Replaced with a visual-regression snapshot criterion (pixel-diff tolerance, per theme) pinned to the prototype files.
- 🟡 **Testability**: "Adequate border/background contrast" lacks a measurable threshold — **Resolved.** Now asserts the named tokens (`--ac-stroke-strong` border on `--ac-bg-card`) plus absence of the native `<input>`.
- 🔵 **Completeness**: Bug lacks explicit reproduction steps — **Resolved.** A "Steps to Reproduce" block (input/action/expected/actual) was added; completeness returned zero findings this pass.
- 🔵 **Dependency**: Sibling bug 0094 coupling only "Related" — **Resolved.** The 0094 entry now notes the shared `MarkdownRenderer`/token surface and that the fixes are non-blocking but should be coordinated.
- 🔵 **Testability**: "No regression to FilterPill" near-tautological — **Resolved.** Now points at the existing FilterPill tests/snapshots passing unchanged.
- 🔵 **Testability**: Tick visibility/colour not verifiable — **Partially resolved.** Now asserts `#fff` on `--ac-accent`; "legible" still lacks a numeric threshold (re-flagged below as minor).
- 🔵 **Clarity**: Summary `color-scheme` wording looser than Technical Notes — **Resolved.** Summary now states light theme has no dedicated rule and inherits `:root`.
- 🔵 **Clarity**: bare "GFM" acronym — **Partially resolved.** Expanded in the Dependencies entry; clarity re-flags that `remark-gfm`/GFM still appears in the Summary before the definition (minor ordering point).
- 🔵 **Dependency**: prototype "agreed" state asserted not linked — **Resolved.** Assumptions now states there is no decision record and the prototype files are the de facto source of truth, with a re-evaluation trigger.

### New Issues Introduced

All minor/suggestion — none block implementation:

- 🔵 **Dependency / Testability** (cross-cutting): The visual-regression baseline AC depends on a baseline that doesn't exist yet for the new markup, and the pixel-diff tolerance is referenced rather than stated. Flagged by both lenses. Note that capturing light + dark baselines is part of this work (and may need the project's baseline-regeneration workflow).
- 🔵 **Testability** (minor): "remaining legible against the accent fill" still has no numeric threshold — either drop the clause (the `#fff`-on-`--ac-accent` colour pair is already checkable) or pin to a contrast ratio (e.g. ≥ 3:1).
- 🔵 **Testability** (low): No AC directly asserts that a checked item renders a tick glyph or that the list marker is suppressed — these named Requirements are only verified indirectly via the snapshot.
- 🔵 **Clarity** (suggestion): The prototype's CSS fallback chains (`var(--ac-stroke-strong, var(--ac-stroke))` for the border, `var(--ac-stroke-strong, var(--ac-fg-faint))` for the strike decoration) are stated as single tokens in Requirements/AC — clarify whether the fallbacks are intended.
- 🔵 **Clarity** (low): "a baseline captured in each theme" leaves the capture actor/timing implicit — clarify the baseline is newly captured as part of this work.

### Assessment

The work item is ready for implementation. The two structural testability gaps that drove the REVISE verdict are closed, the bug now has full reproduction steps, and dependency couplings are explicit. The remaining findings are optional refinements — the most worthwhile being a one-line note that the visual-regression baselines must be captured as part of this work (raised independently by both the dependency and testability lenses). These can be folded in now or left for the implementer.
