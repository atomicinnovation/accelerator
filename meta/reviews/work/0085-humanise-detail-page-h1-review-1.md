---
date: "2026-05-31T21:18:21Z"
type: work-item-review
producer: review-work-item
target: "work-item:0085"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
id: "0085-humanise-detail-page-h1-review-1"
title: "0085-humanise-detail-page-h1-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-31T21:18:21Z"
last_updated_by: Toby Clemson
---

## Work Item Review: Humanise Detail-Page H1 Across All Doc Kinds

**Verdict:** REVISE

The work item is structurally complete, tightly scoped, and well-anchored to specific files and dependencies — completeness and scope lenses found no concerns of substance, and dependency capture is unusually thorough. Two major testability gaps drive the REVISE verdict: an unresolved open question about prefix handling that determines the expected output of `humanise_slug` (and therefore two of its required unit tests), and an unbounded "no detail-page route renders" criterion that spans thirteen doc kinds without a defined verification procedure. Several minor findings cluster around AC4's live-corpus spot check and incidental frontmatter/tag accuracy.

### Cross-Cutting Themes

- **Unresolved prefix-handling question shadows the test specification** (flagged by: clarity, testability) — The Open Question about preserving vs stripping leading numeric/date prefixes leaves AC2's "leading numeric IDs" and "leading ISO dates" unit tests without a defined expected output.
- **AC4 post-migration smoke check is awkwardly placed** (flagged by: dependency, scope, testability) — The criterion verifies the outputs of blockers 0065/0066/0070 against a re-indexed live corpus, couples this story's completion to operational state outside its surface, and uses a loose "spot-check" pass threshold whose internal-path claim isn't directly observable from `entry.title`.

### Findings

#### Major
- 🟡 **Testability**: Expected `humanise_slug` output for numeric/date prefixes is unresolved
  **Location**: Open Questions / Acceptance Criteria
  The Open Question asks whether `humanise_slug` should preserve or strip leading numeric IDs and ISO dates, yet AC2 mandates unit tests covering exactly those two cases. Without a chosen behaviour, the unit tests have no defined expected output.

- 🟡 **Testability**: AC1 covers all thirteen doc kinds without a defined verification procedure
  **Location**: Acceptance Criteria (AC1)
  AC1 spans thirteen `DocTypeKey` variants but no procedure is specified for how a verifier confirms the invariant across every kind. The "any" quantifier without a defined input set leaves the criterion effectively unbounded.

#### Minor
- 🔵 **Clarity**: Helper-placement requirement leaves two valid interpretations
  **Location**: Requirements
  The second requirement says to introduce `humanise_slug` "in `api/library.rs` (or a new shared module if more humanisers are anticipated)". A reader cannot tell whether the placement is committed or deferred; Technical Notes restates the same either/or without resolving it.

- 🔵 **Clarity**: Unresolved prefix-handling question shadows the `humanise_slug` acceptance criteria
  **Location**: Open Questions
  The Open Question is flagged "for resolution during implementation" but AC2 lists tests for "leading numeric IDs" and "leading ISO dates" without saying which behaviour they should encode.

- 🔵 **Clarity**: References 0060 and 0063 lack inline context
  **Location**: References
  0060 and 0063 are listed in References but their role isn't stated anywhere in the body (0063 is touched on once in Drafting Notes; 0060 not at all).

- 🔵 **Dependency**: Parent epic 0057 referenced as Related rather than parent
  **Location**: Frontmatter: parent / Dependencies
  Frontmatter `parent` is empty, but Dependencies and Context both identify epic 0057 as the parent of the gating dependencies. Planning roll-ups traversing `parent` will not associate this story with 0057.

- 🔵 **Dependency**: Ordering relative to in-flight sibling header work (0074, 0084) not stated
  **Location**: Dependencies
  Drafting Notes mentions the in-progress sibling header work and suggests a priority bump if co-landing is desired, but Dependencies doesn't say whether merge order matters.

- 🔵 **Testability**: Spot-check criterion has loose pass threshold
  **Location**: Acceptance Criteria (AC4)
  "Spot-check" isn't a defined procedure, and "resolves via `frontmatter.title`" is an internal cascade-path claim not directly observable from `entry.title` alone.

- 🔵 **Testability**: Cascade-layer test scope is not fully enumerated
  **Location**: Acceptance Criteria (AC3)
  AC3 requires tests "verifying each layer fires under the expected conditions" but doesn't enumerate the layers, leaving room for disagreement about whether three tests or six (positive + negative per layer) are required.

#### Suggestions
- 🔵 **Clarity**: 'Three doc kinds' enumeration could name the kinds using canonical plurals
  **Location**: Context
  Context uses singular `work-item-review`/`plan-review`/`validation`; Requirements uses plural `work-item-reviews`/`plan-reviews`/`validations`. Aligning the spelling removes a small inconsistency.

- 🔵 **Dependency**: No mention of downstream visual/QA verification step for re-indexed corpus
  **Location**: Dependencies
  AC4 implicitly depends on the live index being re-run against the migrated corpus after 0070 lands — a runtime/ops step not named in Dependencies.

- 🔵 **Scope**: `frontend` tag may overstate scope
  **Location**: Tags / Frontmatter
  Requirements, AC, and Technical Notes all state the change is server-side only. The `frontend` tag (and arguably `design`) misrepresents the surface area.

- 🔵 **Scope**: Post-migration smoke check straddles this story and its blockers
  **Location**: Acceptance Criteria (AC4)
  AC4 primarily verifies the outputs of 0065/0066/0070 rather than this story's fallback helper, coupling completion to the operational state of other stories' corpus migrations.

### Strengths
- ✅ Every expected section for a story is present and substantively populated; frontmatter is complete with recognised values.
- ✅ Summary, Context, Requirements, Acceptance Criteria, and Technical Notes describe the same narrow change surface (one function in `frontmatter.rs` plus one helper), with named files and line numbers anchoring referents.
- ✅ Dependencies section is exemplary: blockers (0065, 0066, 0070), foundational predecessor (0041), related work with shared surfaces/sources, and an explicit `Blocks: none` declaration — each annotated with its semantic reason.
- ✅ Drafting Notes explicitly document the scope-collapse decision and why a kind-aware synthesis layer was pushed out via dependencies, demonstrating intentional scope discipline.
- ✅ AC2 enumerates exact unit-test cases (`humanise_slug` fixtures) and AC5/AC6 are crisp file-diff and inline-comment invariants — mechanically verifiable.

### Recommended Changes

1. **Resolve the prefix-handling open question and pin it to AC2** (addresses: Expected `humanise_slug` output for numeric/date prefixes is unresolved; Unresolved prefix-handling question shadows the acceptance criteria) — Pick a default (recommend: preserve verbatim, since the fallback is defensive and the prefix carries information). Add a worked example to AC2, e.g. `humanise_slug("0042-templates-view-redesign-review-1") == "0042 Templates View Redesign Review 1"`. Remove or close the Open Question.

2. **Bind AC1 to a concrete verification procedure** (addresses: AC1 covers all thirteen doc kinds without a defined verification procedure) — Rephrase to: "A unit test feeds a representative stem (no `frontmatter.title`, no first H1) through `title_from` for each `DocTypeKey` variant and asserts the resolved title differs from the raw stem and matches `humanise_slug(stem)`." This avoids the unbounded "any" quantifier and turns the criterion into a single deterministic test.

3. **Reframe or relocate AC4** (addresses: Spot-check criterion has loose pass threshold; Post-migration smoke check straddles this story and its blockers; No mention of downstream re-indexed corpus) — Either: (a) drop AC4 from this story and add a verification line to 0070's corpus-migration story; or (b) reframe AC4 as a synthetic-fixture check ("for each `DocTypeKey` variant, a fixture with `frontmatter.title` populated produces an H1 matching `frontmatter.title` verbatim"). If keeping a live check, name the re-indexed-corpus dependency explicitly.

4. **Commit to a placement for `humanise_slug`** (addresses: Helper-placement requirement leaves two valid interpretations) — Default to placing it next to `humanise_status` in `api/library.rs`, with a note that extraction to `humanise.rs` is deferred until a third humaniser appears.

5. **Enumerate the cascade layers in AC3** (addresses: Cascade-layer test scope is not fully enumerated) — Restate as: "Tests assert (a) `frontmatter.title` present → returned verbatim; (b) `frontmatter.title` absent + first H1 present → H1 returned; (c) both absent → `humanise_slug(stem)` returned."

6. **Set `parent: "0057"` in frontmatter** (addresses: Parent epic 0057 referenced as Related rather than parent) — Move 0057 out of the Related list in Dependencies, since the parent linkage now carries the relationship.

7. **State ordering with 0074/0084 explicitly** (addresses: Ordering relative to in-flight sibling header work not stated) — One line in Dependencies, e.g. "This work item can land independently of 0074/0084; the header markup consumes `entry.title` verbatim regardless of hue or chip strip."

8. **Trim tags** (addresses: `frontend` tag may overstate scope) — Drop `frontend` and `design`; keep `backend`, `detail-page`, and optionally add `indexer`.

9. **Align singular/plural doc-kind names** (addresses: 'Three doc kinds' enumeration) — In Context, use `work-item-reviews`, `plan-reviews`, `validations` to match Requirements and `DocTypeKey` variants.

10. **Annotate or remove References 0060 and 0063** (addresses: References 0060 and 0063 lack inline context) — Add a one-phrase qualifier inline (e.g. "0063 — frontmatter field canonicalisation that this file predates") or move them into Drafting Notes where their role is already partially explained.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is well-written with mostly unambiguous referents and a coherent narrative across sections. A few minor clarity concerns exist around helper-placement wording, an unresolved open question that shadows an acceptance criterion, and a couple of references whose role is not fully explained, but none undermine the reader's ability to understand intent.

**Strengths**:
- The Summary, Context, and Requirements describe the same scope: humanise the residual filename-stem fallback in the server-side title cascade.
- Technical Notes name specific files, functions, and line numbers, so referents like 'the cascade' and 'the indexer' resolve unambiguously.
- Domain jargon ('cascade', 'humanise', 'frontmatter.title', 'doc kind') is used consistently and tied to concrete locations in the codebase.
- Dependencies clearly state which upstream stories must ship first and why, removing ambiguity about ordering.
- Drafting Notes explicitly record the scope-collapse history, which helps a reader reconcile any stale assumptions in related documents.

**Findings**: 3 minor + 1 suggestion (helper placement; prefix-handling Open Question; References 0060/0063; doc-kind plural alignment).

### Completeness

**Summary**: The work item is structurally complete and substantively populated across all expected sections for a story. Frontmatter integrity is intact with recognised `kind`, `status`, and `priority`. Kind-appropriate content is present — clear motivation in Context and verifiable done-criteria in Acceptance Criteria.

**Strengths**:
- Every expected section for a story is present and substantively populated — no empty or placeholder sections.
- Frontmatter is complete and uses recognised values (kind: story, status: draft, priority: low).
- Context section explains the motivation rather than merely restating the summary.
- Acceptance Criteria contains six specific bullets covering routes, helper behaviour, cascade tests, post-migration verification, frontend non-change, and inline documentation.
- Open Questions, Dependencies, Assumptions, and Drafting Notes are all populated.
- Story-appropriate content present: the system whose need is being met is identified, and criteria define when the story is done.

**Findings**: None.

### Dependency

**Summary**: The work item has unusually thorough dependency capture: upstream blockers (0065, 0066, 0070), foundational predecessor (0041), related work sharing surfaces or sources, and an explicit 'Blocks: none' declaration are all present. The Drafting Notes explain why the blockers were chosen. No significant uncaptured couplings are evident; minor observations concern parent-epic linkage and ordering relative to in-flight sibling work.

**Strengths**:
- Upstream blockers are named explicitly with a rationale explaining what would break if they slipped.
- Dependencies distinguishes Blocked by / Builds on / Related / Blocks, giving each relationship a precise semantic.
- `Blocks: none` is stated explicitly rather than left implicit.
- Related items are annotated with the reason for the coupling.
- Assumptions restates the dependency on 0065/0066/0070 in terms of the runtime invariant they establish.

**Findings**: 2 minor + 1 suggestion (parent epic 0057; ordering vs 0074/0084; re-indexed-corpus dependency for AC4).

### Scope

**Summary**: Work item 0085 is a tightly-scoped, atomic story describing a single server-side change: replacing the raw filename-stem fallback in the title cascade with a humanised slug, plus a small helper and tests. The scope was deliberately collapsed during enrichment (documented in Drafting Notes) and the dependencies on 0065/0066/0070 keep this from absorbing a kind-aware synthesis layer.

**Strengths**:
- Single unified purpose: every requirement and acceptance criterion serves the one goal of humanising the H1 fallback path.
- Summary, Requirements, Acceptance Criteria, and Technical Notes all describe the same narrow change surface.
- Drafting Notes explicitly document the scope-collapse decision.
- Dependencies section clearly bounds the work by deferring the primary-derivation problem to 0065/0066/0070.
- Atomic and deliverable end-to-end by a single team within one increment.

**Findings**: 2 suggestions (`frontend`/`design` tags overstate scope; AC4 post-migration smoke check straddles this story and its blockers).

### Testability

**Summary**: The Acceptance Criteria are largely testable: most criteria specify concrete fixtures, named functions, or observable outcomes. The main gaps are an unresolved open question that directly determines the expected output of `humanise_slug`, an unbounded 'no detail-page route renders' criterion that doesn't specify a verification procedure across thirteen doc kinds, and a 'spot-check' criterion whose pass/fail threshold is loose.

**Strengths**:
- AC2 enumerates the exact unit-test cases `humanise_slug` must cover.
- AC3 names the specific test target (`title_from` cascade tests) and identifies the new layer that must be exercised.
- AC5 and AC6 are crisp invariants — file diffs and an inline-comment presence check are mechanically verifiable.
- Requirements names the exact function, file, and line region to be modified.

**Findings**: 2 major + 2 minor (unresolved prefix behaviour blocks AC2 test design; AC1 unbounded across 13 doc kinds; AC4 loose pass threshold; AC3 layer enumeration ambiguous).

## Re-Review (Pass 2) — 2026-05-31T20:18:44Z

**Verdict:** COMMENT

Work item is acceptable as-is but could be improved — see minor findings below. All previously identified findings are resolved. Re-review surfaced new minor issues clustering around the mixed-prefix `humanise_slug` case (called out by clarity and testability lenses) and two AC under-specification concerns. Dependency and scope lenses now have zero findings.

### Previously Identified Issues

- 🟡 **Testability**: Expected `humanise_slug` output for numeric/date prefixes is unresolved — **Resolved** (strip behaviour chosen, Open Question closed, AC2 carries worked examples)
- 🟡 **Testability**: AC1 covers all thirteen doc kinds without a defined verification procedure — **Resolved** (AC1 rewritten as deterministic test iterating `DocTypeKey::all()`, though re-review flags a residual minor about "representative stem" — see new issues)
- 🔵 **Clarity**: Helper-placement requirement leaves two valid interpretations — **Resolved** (committed to `api/library.rs`; deferral note added)
- 🔵 **Clarity**: Unresolved prefix-handling question shadows the acceptance criteria — **Resolved**
- 🔵 **Clarity**: References 0060 and 0063 lack inline context — **Resolved** (each Related entry annotated)
- 🔵 **Dependency**: Parent epic 0057 referenced as Related rather than parent — **Resolved** (user confirmed no parent; Related annotation corrected)
- 🔵 **Dependency**: Ordering relative to in-flight sibling header work (0074, 0084) not stated — **Resolved** (Ordering line added)
- 🔵 **Testability**: AC4 spot-check criterion has loose pass threshold — **Resolved** (AC4 dropped)
- 🔵 **Testability**: AC3 cascade-layer test scope is not fully enumerated — **Resolved** (layers a/b/c enumerated)
- 🔵 **Clarity**: Singular/plural doc-kind name alignment — **Resolved**
- 🔵 **Dependency**: Re-indexed corpus runtime dependency for AC4 not named — **Resolved** (AC4 dropped)
- 🔵 **Scope**: `frontend` tag overstates scope — **Resolved** (tags trimmed)
- 🔵 **Scope**: AC4 post-migration smoke check straddles this story and its blockers — **Resolved** (AC4 dropped)

### New Issues Introduced

- 🔵 **Clarity / Testability** (flagged by both lenses): Mixed-prefix case in AC2 lacks a worked example. The bullet says "first matching prefix is stripped, remainder retained" but doesn't define single-pass vs greedy behaviour; two implementers could produce different titles for `2026-05-21-0042-foo` (either `0042 Foo` or `2026 05 21 0042 Foo`).
- 🔵 **Testability**: AC1's "representative stem" leaves per-variant input under-specified. Two implementers could pick stems that exercise different prefix shapes, letting a kind-specific edge case slip through.
- 🔵 **Testability**: AC5 inline-comment criterion has no defined pass condition — a comment as terse as `// layer 1` would technically satisfy it. The criterion should name the source field each comment must mention (e.g. `frontmatter.title`, `first H1`, `humanise_slug(stem)`).
- 🔵 **Clarity** (suggestion): "H1" is used in the title and Summary without definition; only Technical Notes ties it to `<h1>` via `Page.tsx:31`.

### Assessment

The work item has shed all major findings and both lenses that previously had concerns about cross-cutting structural issues (dependency, scope) now report zero findings. The remaining issues are all minor refinements to AC wording — none block implementation. The work item is ready for planning; the residual minors can be addressed in-flight or during implementation review without re-circulating the work item.

## Re-Review (Pass 3) — 2026-05-31T21:18:21Z

**Verdict:** COMMENT

Pass 2's four new minors are resolved. Pass 3 surfaces five further minor/suggestion findings, but these are increasingly nitpicks: terminology drift between `filename_stem`/`stem`/`slug`, an undefined "first H1", a negative-assertion verification procedure, structural-vs-behavioural framing of AC2, and a robustness suggestion for AC1's non-equality assertion. None block implementation; diminishing returns are clear from the pass-on-pass finding velocity.

### Previously Identified Issues (Pass 2)

- 🔵 **Clarity + Testability**: Mixed-prefix case in AC2 lacks worked examples — **Resolved** (single-pass behaviour + two worked examples added)
- 🔵 **Testability**: AC1 "representative stem" under-specified — **Resolved** (fixed test stem `"0042-test-fixture"` pinned)
- 🔵 **Testability**: AC5 inline-comment criterion has no defined pass condition — **Resolved** (comments must name source + position)
- 🔵 **Clarity** (suggestion): H1 used without definition — **Partially resolved** (Summary now says "the page's `<h1>` heading"; "first H1" in Context/AC remains unqualified — see new issues)

### New Issues Introduced

- 🔵 **Clarity**: Terminology drift — `filename_stem`, `stem`, and `slug` are used interchangeably across Requirements, AC, and Technical Notes. The helper is named `humanise_slug` but operates on the filename stem.
- 🔵 **Clarity** (suggestion): "first H1" appears in Context, AC1, AC3, AC5 without an explicit gloss. Could be read as either the first level-1 markdown heading in the source or the rendered `<h1>` on the page.
- 🔵 **Testability**: AC4 ("`LibraryDocView.tsx` and `Page.tsx` are unchanged") lacks a verification procedure. Could be rephrased as a diff-level assertion ("The PR diff includes no modifications to these two files").
- 🔵 **Testability**: AC2 mixes implementation prescription ("implemented as a discrete helper") with the verifiable test-case enumeration. The structural requirement is already in Requirements; the AC could focus only on the test cases.
- 🔵 **Testability** (suggestion): AC1's "differs from raw stem" check is parenthetical; could be elevated to an explicit assertion to guard against a future no-op fixture.

### Assessment

The work item is structurally sound and behaviourally well-pinned. Pass-on-pass findings have shifted from substantive (majors, AC structure, dependency ordering) to surface (terminology drift, framing). Further review passes are likely to surface similar low-impact nitpicks. **Recommendation: accept as ready for planning.** The remaining minors are refactors of wording, not gaps in the specification — easily handled by the implementer or in a fast PR review without re-circulating the work item.

**Final verdict: APPROVE** — accepted by author after Pass 3; Pass 3 minors deferred to implementation/PR review.


