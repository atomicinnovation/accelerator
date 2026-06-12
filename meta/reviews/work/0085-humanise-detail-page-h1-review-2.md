---
type: work-item-review
id: "0085-humanise-detail-page-h1-review-2"
title: "Work Item Review: Humanise Detail-Page H1 Across All Doc Kinds"
date: "2026-06-11T00:44:21+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0085"
relates_to: ["work-item-review:0085-humanise-detail-page-h1-review-1"]
work_item_id: "0085"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 2
review_pass: 2
tags: []
last_updated: "2026-06-11T12:38:11+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Humanise Detail-Page H1 Across All Doc Kinds

**Verdict:** COMMENT

The work item is in strong shape — completeness found zero concerns, scope confirms a tightly-bounded atomic story, and the prior review's three passes already resolved every major finding. This fresh review surfaces no critical or major issues that stand up to scrutiny: the one finding raised at major severity (clarity, mixed-prefix examples) is, on inspection, based on a misreading — the two worked examples are internally consistent with the stated single-pass strip rule, and the testability lens independently praised those same examples for *removing* ambiguity. The remaining findings are all minor or suggestion-level refinements: a frontmatter/body inconsistency on the blocker set, two test-oracle hardening opportunities, and some wording polish.

Work item is acceptable as-is and ready for planning. The one finding worth actioning before implementation is the `blocked_by` frontmatter omission of 0070 — a structured-field/prose mismatch that scheduling tooling could trip over.

### Cross-Cutting Themes

- **Mixed-prefix worked examples — contradiction or clarity?** (flagged by: clarity as a concern, testability as a strength) — The two lenses reached opposite conclusions about the same AC2 examples. Independent verification confirms the examples are *correct and consistent* with "strip only the leading matching prefix, then split the remainder on hyphens." The clarity concern is downgraded to a minor "explain the asymmetry" nit; it is not a genuine contradiction.
- **Test oracles assert against the implementation, not concrete values** (flagged by: testability) — Two AC criteria (AC1's `== humanise_slug(stem)` and AC4's negative "files unchanged") lack independent concrete oracles. Minor hardening opportunities, not blockers.

### Findings

#### Major

_None. The clarity lens raised one finding at major severity (mixed-prefix examples), but verification shows the examples are consistent with the stated rule; it is recorded below at minor severity._

#### Minor

- 🔵 **Dependency**: `blocked_by` frontmatter omits 0070, which the body names as a gating blocker
  **Location**: Frontmatter: blocked_by
  The frontmatter `blocked_by` lists only `["work-item:0065", "work-item:0066"]`, but Dependencies, Context, Assumptions, and Drafting Notes all consistently name 0070 (corpus migration) as a third gating blocker. Tooling reading the structured field will treat the story as unblocked one dependency too early.

- 🔵 **Clarity** (downgraded from major): Mixed-prefix examples would benefit from a one-line note explaining the asymmetry
  **Location**: Acceptance Criteria (AC2)
  The two examples (`"2026-05-21-0042-foo" == "0042 Foo"` and `"0042-2026-05-21-foo" == "2026 05 21 Foo"`) are correct and consistent with the single-pass rule, but the reason `0042` survives as one token while a residual date becomes three tokens (its internal hyphens become ordinary separators once it is no longer the leading prefix) is left for the reader to infer.

- 🔵 **Testability**: All-variants invariant test asserts against a function, not a concrete value
  **Location**: Acceptance Criteria (AC1)
  AC1 asserts the resolved title `== humanise_slug(stem)` rather than a concrete literal. A bug in `humanise_slug` would corrupt both the production path and the test oracle, and the assertion would still pass — it verifies wiring but not the humanised value.

- 🔵 **Testability**: AC4 (frontend files unchanged) is a negative criterion with no positive verification procedure
  **Location**: Acceptance Criteria (AC4)
  "`LibraryDocView.tsx` and `Page.tsx` are unchanged" has no positive test and can be silently broken by an unrelated edit. Reframe as a diff-level assertion the PR review or a CI guard can confirm.

- 🔵 **Testability**: Title-casing rule under-specified for residual numeric/date segments and acronyms
  **Location**: Requirements
  "Title-case each segment" is pinned only by the worked examples; the rule for a purely numeric segment, a trailing numeric (`...review-1`), or an already-uppercase segment is not stated as a testable rule.

- 🔵 **Clarity**: Doc-kind counts (thirteen total vs three affected) are never explicitly linked
  **Location**: Context / Requirements
  Context names "three doc kinds" that fall through today; Requirements and AC assert the fix covers all thirteen `DocTypeKey::all()` variants. The relationship between the two numbers (fix all thirteen so future additions are protected, even though only three break today) is left implicit.

#### Suggestions

- 🔵 **Testability**: AC2 minimum-coverage list leaves degenerate inputs (empty / prefix-only stem) unspecified
  **Location**: Acceptance Criteria (AC2)
  The fallback exists precisely for malformed/legacy documents, yet there is no defined expected output for an empty stem, a prefix-only stem (`"2026-05-21"` with nothing after), or consecutive hyphens.

- 🔵 **Scope**: `kind: story` may overstate a defensive, no-expected-user-visible-change unit
  **Location**: Frontmatter: kind
  The change is a one-line fallback swap plus a helper and tests, framed as a "defensive belt-and-braces layer." Whether `chore` fits better than `story` depends on team norms a reviewer cannot observe.

- 🔵 **Dependency**: ADR 0060 named as establishing the relied-upon invariant but absent from Dependencies
  **Location**: Dependencies / References
  0060 establishes `title` as a base field on every artifact type — the invariant the primary cascade path relies on — but appears only under References, not as a Builds-on/Related dependency.

- 🔵 **Clarity**: Idiomatic "belt-and-braces" may not read clearly for all audiences
  **Location**: Summary
  Context already states the same idea plainly; the idiom is optional polish.

### Strengths

- ✅ Completeness found zero concerns: every expected story section is present and substantively populated, frontmatter integrity is sound (`kind: story`, `status: ready`), and an implementer could begin without follow-up questions.
- ✅ The title cascade (`frontmatter.title` → first H1 → raw `filename_stem`) is stated identically across Summary, Context, Requirements, and Technical Notes, giving one coherent model.
- ✅ `humanise_slug` behaviour is pinned with concrete input/output examples, including the tricky mixed-prefix case — testability rated these directly executable and ambiguity-removing.
- ✅ Dependency capture is exemplary: three gating blockers with per-blocker rationale, explicit ordering relative to sibling header work (0074/0084), and a justified empty Blocks field.
- ✅ Drafting Notes document a deliberate scope-collapse decision (deferring the kind-aware synthesis layer to 0065/0066/0070 rather than carrying dead-code tech debt), demonstrating conscious scope discipline.
- ✅ AC3 enumerates all three cascade branches (a/b/c) with the precondition for each, giving complete coverage of the resolution logic.

### Recommended Changes

1. **Add `"work-item:0070"` to the `blocked_by` frontmatter** (addresses: `blocked_by` omits 0070) — Bring the structured field into line with the three-blocker set described throughout the body, so schedulers don't treat the story as ready early. This is the one change worth making before planning.

2. **Add a one-line note to AC2 explaining the mixed-prefix asymmetry** (addresses: mixed-prefix examples) — e.g. "once the leading prefix is stripped, any hyphens inside a *residual* date/id are ordinary segment separators." The examples are already correct; this just removes the inference burden.

3. **Add a concrete-literal oracle to AC1** (addresses: all-variants test asserts against a function) — Keep the `== humanise_slug(stem)` wiring assertion, but also assert at least one fixed stem against a concrete literal (e.g. `"0042-test-fixture"` → `"Test Fixture"`) so a value-level regression is caught independently.

4. **Reframe AC4 as a diff-level assertion** (addresses: negative criterion) — e.g. "the PR diff touches no files under the frontend `LibraryDocView`/`Page` paths," which a reviewer or CI guard can confirm.

5. **State the title-casing rule explicitly** (addresses: under-specified casing) — e.g. "uppercase the first character of each hyphen segment, leave the rest untouched; digit-led segments emitted verbatim," plus one fixture for a trailing numeric segment.

6. **(Optional) Link the thirteen-vs-three counts** (addresses: doc-kind count) — A half-sentence in Context, e.g. "three kinds break today, but the fallback and its test cover all thirteen so future additions are protected."

7. **(Optional) Specify a degenerate-input expectation in AC2; surface 0060 in Dependencies; reconsider `kind`** — Low-impact polish; safe to defer to implementation or PR review.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item communicates its intent with strong precision: the title cascade, the role of the humanised fallback, and the worked `humanise_slug` examples leave little room for misinterpretation. The clarity agent raised the mixed-prefix examples at major severity, but verification during aggregation found the examples consistent with the stated single-pass rule (and the testability lens praised them) — recorded here at minor severity. A few jargon terms and one count cross-reference are minor/suggestion-level.

**Strengths**:
- The title cascade is stated identically and unambiguously in Summary, Context, Requirements, and Technical Notes.
- `humanise_slug` behaviour is pinned down with concrete input/output examples, removing guesswork about casing and prefix handling.
- The actor and trigger are explicit throughout — the server-side `title_from` performs the derivation; the frontend consumes `entry.title` verbatim.
- Open Questions explicitly records that the prefix strip-vs-preserve decision was resolved.

**Findings**:
- 🟡→🔵 (major, downgraded to minor on verification) **Acceptance Criteria** — Mixed-prefix examples appear to contradict the single-pass strip rule. On inspection the two examples are correct: stripping only the leading prefix and then splitting the remainder on all hyphens yields exactly `"0042 Foo"` and `"2026 05 21 Foo"`. The asymmetry (one token vs three) follows from a residual date's internal hyphens becoming ordinary separators. A one-line note would remove the inference burden.
- 🔵 (minor) **Requirements / Context** — Doc-kind count stated as thirteen (total) but only ambiguously reconciled with the three affected kinds; the relationship between the numbers is never stated.
- 🔵 (suggestion) **Summary** — "Belt-and-braces" idiom may not read clearly for all audiences; Context already states the idea plainly.

### Completeness

**Summary**: An unusually complete and well-populated story. Every expected section is present and substantively filled. Frontmatter integrity is sound with a recognised kind (story) and valid status (ready). The work item provides everything an implementer needs to begin without follow-up questions.

**Strengths**:
- Summary states the work as a single unambiguous action.
- Context explains motivation thoroughly — the current cascade, which kinds fall through, and why this becomes a defensive layer post-migration.
- Acceptance Criteria contains five specific, worked-example-backed criteria.
- Requirements name exact files, helper placement, and the prefix-stripping algorithm.
- Kind-specific content present: affected system and rationale both identified.
- Dependencies, Assumptions, and Open Questions are all meaningfully populated.

**Findings**: None.

### Dependency

**Summary**: Unusually thorough dependency capture: three gating upstream blockers named in both prose and (partially) the frontmatter, ordering constraints relative to sibling header work explicitly addressed, and a justified empty Blocks field. The only gaps are a frontmatter/body inconsistency on the blocker set and one upstream ADR (0060) treated as an invariant-establishing prerequisite without being surfaced as a dependency.

**Strengths**:
- All three gating blockers (0065, 0066, 0070) named with rationale for why each gates the work.
- Ordering relative to 0074/0084 proactively addressed ("can land independently").
- Empty Blocks field justified by the work's defensive nature.
- Assumptions ties the blockers to a concrete runtime invariant.
- No external systems or cross-team couplings implied — correctly carries none.

**Findings**:
- 🔵 (minor) **Frontmatter: blocked_by** — Omits `work-item:0070`, which the body consistently names as a gating blocker. Structured-field readers will treat the story as unblocked too early.
- 🔵 (minor) **Dependencies / References** — ADR 0060 named as establishing the relied-upon `title` base-field invariant but appears only under References, not as a Builds-on/Related dependency.

### Scope

**Summary**: A tightly-scoped, atomic story: a single server-side function edit plus a discrete `humanise_slug` helper and its tests, all serving one unified purpose. Summary, Requirements, and Acceptance Criteria describe the same coherent scope; the work lives entirely within the Rust indexer/server; and Drafting Notes show a deliberate scope-collapse to avoid bundling a kind-aware synthesis layer.

**Strengths**:
- Single coherent purpose — every requirement serves the one goal.
- Boundaries explicitly stated (in scope: `title_from`/`humanise_slug`; out: frontend files, API DTO).
- Drafting Notes document a deliberate scope-collapse decision.
- Stays within a single service boundary.
- Helper-extraction explicitly deferred until a third humaniser appears.

**Findings**:
- 🔵 (suggestion) **Frontmatter: kind** — `kind: story` may overstate a defensive, no-expected-user-visible-change unit that reads closer to a chore/task; depends on team norms.

### Testability

**Summary**: Unusually strong testability: criteria specify exact inputs and expected outputs, enumerate the full set of doc kinds, and prescribe fixture-driven cascade verification. The criteria collectively cover the Summary's intent. The main gaps are an invariant test that asserts equality to a function rather than a concrete string, a negative-only criterion (AC4), an under-specified casing rule, and unspecified degenerate inputs.

**Strengths**:
- AC2 gives concrete input/output pairs directly executable as assertions.
- AC2 explicitly disambiguates the mixed-prefix case with worked examples.
- AC3 enumerates all three cascade branches with preconditions.
- AC1 binds doc-kind scope to a concrete enumerated set (`DocTypeKey::all()`).
- The Summary intent is mapped to a concrete verification (resolved title differs from raw stem).

**Findings**:
- 🔵 (minor) **Acceptance Criteria (AC1)** — Invariant test asserts against `humanise_slug(stem)`, not a concrete literal; a shared bug in the oracle and production path would pass. Add a concrete-literal assertion for one fixed stem.
- 🔵 (minor) **Acceptance Criteria (AC4)** — "Frontend files unchanged" is a negative criterion with no positive verification; reframe as a diff-level assertion.
- 🔵 (minor) **Requirements** — Title-casing rule under-specified for residual numeric/date segments and acronyms; state the rule and add fixtures.
- 🔵 (suggestion) **Acceptance Criteria (AC2)** — Minimum-coverage list leaves degenerate inputs (empty/prefix-only stem, consecutive hyphens) unspecified, though the fallback targets exactly such malformed documents.

## Re-Review (Pass 2) — 2026-06-11T12:38:11+00:00

**Verdict:** COMMENT

All findings from the initial Review-2 pass are resolved (the 0070 `blocked_by` omission, the casing-rule under-specification, the AC1 oracle, the AC4 negative criterion, the AC2 degenerate inputs, the doc-kind count linkage, and 0060's absence from Dependencies). Re-running clarity, dependency, scope, and testability surfaced no critical or major issues — only further minor/suggestion nitpicks, confirming the diminishing-returns pattern noted across Review-1's three passes. The work item is ready for planning.

### Previously Identified Issues

- 🔵 **Dependency**: `blocked_by` omits 0070 — **Resolved** (`"work-item:0070"` added to frontmatter; both dependency and the body now agree on the three-blocker set).
- 🔵 **Clarity**: Mixed-prefix asymmetry left for reader to infer — **Resolved** (AC2 now explains residual hyphens become ordinary separators). Note: the re-review clarity agent claimed the added clause inverts "first"/"second", but verification confirms the clause maps each example to its rationale correctly — recorded as not-a-defect, mirroring the analogous misread in the initial pass.
- 🔵 **Testability**: AC1 asserts against a function, not a literal — **Resolved** (concrete `"0042-test-fixture" → "Test Fixture"` oracle added; "test oracle" now glossed inline).
- 🔵 **Testability**: AC4 negative criterion — **Resolved** (reframed as a diff-level / CI-path-guard assertion; testability lens now lists it as a strength).
- 🔵 **Testability**: Title-casing rule under-specified — **Resolved** (Requirements now defines the rule, digit-led-segment handling, and the residual-hyphen rule).
- 🔵 **Clarity**: Thirteen-vs-three count linkage — **Resolved** (Context now links the counts).
- 🔵 **Testability**: AC2 degenerate inputs unspecified — **Resolved** (prefix-only and empty-stem cases added; empty-stem now an asserted equality `humanise_slug("") == ""`).
- 🔵 **Dependency**: ADR 0060 absent from Dependencies — **Resolved** (added under "Builds on").
- 🔵 **Scope**: `kind: story` may overstate the change — **Not actioned** (deliberately deferred; team-norm judgment call, re-flagged this pass as a suggestion).

### New Issues Introduced

All minor or suggestion-level; none block implementation.

- 🔵 **Clarity** (Requirements): "strip at most one leading prefix" reads ambiguously *in isolation* — the disambiguating worked examples live in AC2; a one-line cross-pointer would co-locate rule and examples.
- 🔵 **Clarity** (Requirements): "first H1" still used without a gloss (first level-1 Markdown heading vs rendered `<h1>`) — a recurring low-severity nit carried from Review-1.
- 🔵 **Testability** (AC1): the concrete-literal assertion is required "for at least one variant" without naming which, and without stating `humanise_slug` is kind-independent — the anti-shared-bug guard then only definitively covers one unnamed kind.
- 🔵 **Testability** (AC2): the title-case "leave remaining characters untouched" rule is not exercised by any example with non-trivial interior casing (e.g. `iOS`), so a naive title-case impl could pass all assertions while violating the spec.
- 🔵 **Dependency / Scope** (suggestions): builds-on couplings (0041/0060) live only in prose not frontmatter; 0057 epic-vs-children gating could be stated more explicitly; `kind: story` vs `chore`.

### Assessment

The work item has now shed every finding from both the initial Review-2 pass and (cumulatively) Review-1's three passes. The findings velocity has flattened into wording-level nitpicks that two independent reviews have repeatedly judged non-blocking. **Recommendation: accept as ready for planning.** The residual minors — a cross-pointer between Requirements and AC2, an `iOS`-style interior-casing fixture, and naming the kind-independence assumption in AC1 — are safe to handle in-flight or at PR review without re-circulating the work item.

**Final verdict: APPROVE** — accepted by author after Re-Review Pass 2; the residual Pass-2 minors are deferred to implementation/PR review.
