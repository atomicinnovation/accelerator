---
type: "work-item-review"
id: "0278-topic-research-visualiser-doc-type-review-1"
title: "Work Item Review: Topic-Research Visualiser Doc Type and Indexer"
date: "2026-09-10T09:35:49+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0278"
work_item_id: "0278"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-09-10T11:39:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Topic-Research Visualiser Doc Type and Indexer

**Verdict:** REVISE

0278 is a dense, well-structured story: every section is present and
substantively populated, the manifest-status collapse reads identically across
Summary, Context, Requirements, and Acceptance Criteria, and the co-land
relationship with 0277 is reconciled carefully in both prose and frontmatter.
The REVISE verdict rests on three major findings rather than any structural
weakness — two testability gaps where acceptance criteria cannot be mechanically
verified as written (the prototype-match criterion and the indexer's
negative/skip assertions), and one uncaptured downstream dependency on sibling
0279, which consumes the base-`status` lifecycle and `complete` vocabulary this
story provisions. The remaining findings are minor clarity and coverage polish.

### Cross-Cutting Themes

- **Acceptance criteria leave stated constraints unverifiable** (flagged by:
  testability, completeness) — Concrete constraints named in Requirements have
  no matching mechanically-checkable criterion: the ≥3:1 contrast rule and
  hue-distinctness, the indexer's one-entry-per-set with its negative/skip
  clauses, and the dangling-reference integrity deferred here by 0277. AC7's
  "implemented visuals match the updated prototype" has no comparison procedure
  at all.
- **The Dependencies section under-captures forward and tooling couplings**
  (flagged by: dependency, completeness, clarity) — The 0277 relationship is
  captured precisely, but the forward edge to 0279 (major), the external gates
  (Claude Design prototype pass, the Docker/Linux VR harness), and the deferred
  dangling-reference deliverable are absent, and an ambiguous "its indexer"
  antecedent sits in the same section.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: "Implemented visuals match the updated prototype" has no defined pass/fail procedure
  **Location**: Acceptance Criteria (AC7)
  AC7 requires the implemented visuals to "match the updated prototype", but no
  comparison procedure or threshold is specified. The VR baselines (AC1) are
  generated from the implementation itself and only detect drift; they do not
  compare the implementation against the Claude Design prototype.

- 🟡 **Testability**: Indexer negative and skip assertions lack a fixture that can exercise them
  **Location**: Acceptance Criteria (AC2) / Requirements (Fixtures)
  AC2 asserts that individual findings and reports are not separately indexed and
  that dot-prefixed in-flight directories are skipped, but the only checked-in
  fixture is a set containing a single `manifest.md`. With no `findings/`,
  `reports/`, or dot-prefixed sibling directory, the negative and skip assertions
  are vacuously true and cannot regress-test.

- 🟡 **Dependency**: Downstream coupling to 0279 (status-collapse consumer) uncaptured
  **Location**: Dependencies
  This story collapses the set lifecycle onto the manifest's base `status` and
  provisions the `complete` vocab that 0279's `finalise` consumes, yet 0279
  appears nowhere in Dependencies and `blocks` lists only 0284. A planner reading
  0277's `blocks` edge could schedule 0279 before this delta lands.

#### Minor

- 🔵 **Completeness**: Whole-corpus dangling-reference integrity, deferred here by 0277, is not in this item's definition of done
  **Location**: Acceptance Criteria
  Sibling 0277 and epic 0121 both state that full dangling-reference integrity
  "co-lands with the 0278 indexer", assigning it to this story, yet 0278's
  Requirements and Acceptance Criteria never mention it. An implementer reading
  only 0278 has no criterion telling them the whole-corpus linkage-integrity
  check is part of this story's done.

- 🔵 **Scope**: The 0277-delta status collapse is the one requirement group separable from the visualiser goal
  **Location**: Requirements: The 0277-delta
  The collapse touches `schema.rs`, the manifest template, the `research-topic`
  skill, and the corpus-cli fixture — corpus-layer code 0277 landed, not the
  visualiser surface. Keep it here (the coupling is justified and the two co-land
  anyway), but retain the explicit Summary/Context framing so reviewers of both
  stories see the corpus delta lives in 0278 by design.

- 🔵 **Testability**: Measurable contrast/hue constraint is stated in Requirements but asserted by no criterion
  **Location**: Acceptance Criteria / Requirements (Library placement and rendering)
  The light colour pair "clears ≥3:1 contrast" and "the hue is clear of research
  (hue 28)" appear in Requirements but no acceptance criterion asserts them; AC1's
  VR baselines only pin whatever pixels ship. A colour failing the ratio or
  clashing with `research`'s hue could pass every criterion.

- 🔵 **Testability**: "Legible colour" for the status chip is unquantified and exercises only one of five states
  **Location**: Acceptance Criteria (AC3)
  AC3 requires the chip to render "with a legible colour" (no threshold) and
  exercises only `synthesised`, while Requirements call for a mapping across all
  five states. A missing or default-grey mapping for one of the other four could
  ship undetected.

- 🔵 **Clarity**: "Six boolean predicates" is unenumerated and mislabels a non-boolean method
  **Location**: Requirements
  The doc-type requirement says the "six boolean predicates mirror
  `DesignInventories`" but names none of them, and groups
  `nested_manifest_filename()` — which returns `Some("manifest.md")`, not a
  boolean — among them. A reader cannot verify which methods to set or the values
  they take from the work item alone.

- 🔵 **Clarity**: "its indexer" has an ambiguous antecedent (0277 vs this story)
  **Location**: Dependencies
  The Blocked-by bullet reads "Blocked by: 0277 … its indexer keys on the
  `manifest.md` the engine writes." The nearest antecedent is 0277, yet the
  indexer is delivered by this story; 0277 is the engine that writes the manifest.

- 🔵 **Dependency**: Claude Design and the Docker/Linux VR harness gate completion but are absent from Dependencies
  **Location**: Dependencies
  Two completion gates named only in Requirements/Notes/Assumptions — the external
  Claude Design canvas the visuals must match, and the pinned Docker/Linux VR
  harness that regenerates the committed baselines — are absent from Dependencies,
  so their availability and the prototype-then-implement ordering are not where a
  scheduler looks.

#### Suggestions

- 🔵 **Scope**: The story is not independently deliverable (co-land with 0277)
  **Location**: Dependencies
  0278 "must co-land with the engine (0277) so the vertical demo lands whole", so
  it cannot merge as a standalone increment. No change needed — a deliberate,
  documented split; flagged only so the co-land is understood as an accepted
  unit-of-delivery coupling rather than an oversight.

- 🔵 **Clarity**: "VR" acronym used repeatedly without expansion in this work item
  **Location**: Requirements / Technical Notes
  "VR" appears ~5 times but is never expanded within 0278; it is spelled out as
  "visual-regression (VR)" only in the parent epic 0121.

- 🔵 **Clarity**: Summary names the collapse target but not the source field being removed
  **Location**: Summary
  The Summary says the story "collapses the set-lifecycle status onto the
  manifest's base `status`" without naming the `research_status` field being
  removed, so a Summary-only read may miss that a distinct field is eliminated
  rather than relabelled.

- 🔵 **Clarity**: "The lifecycle value" for the corpus-cli fixture is unspecified
  **Location**: Requirements
  The 0277-delta requirement says the corpus-cli fixture's base `status` "carries
  the lifecycle value" using the definite article for a value never identified,
  whereas the visualiser fixture is pinned to `synthesised`.

### Strengths

- ✅ Every expected section is present and substantively populated — no empty or
  placeholder sections; even Open Questions carries a real answer plus the
  rationale for deferral.
- ✅ The Summary is a complete As-a/I-want/so-that story naming the beneficiary
  and benefit; frontmatter is complete and correct (`kind: story`,
  parent/blocks/relates_to linkage all present).
- ✅ The manifest-status collapse is stated identically across Summary, Context,
  Requirements, the 0277-delta list, and AC5 — a single reading of its intent.
- ✅ The co-land with 0277 is captured precisely, including the reasoning for
  `relates_to` over a reciprocal `blocked_by` to avoid a block cycle, and the
  on-branch code-overlap coupling is named directly.
- ✅ Strong boundary discipline: 0284 concerns (set-level detail page,
  `primary`-pointer resolution, sub-document navigation) are explicitly excluded,
  and the visualiser registration is a genuinely indivisible atomic unit.
- ✅ Most acceptance criteria are Given/When/Then anchored to mechanical gates
  (`mise run check`, `accelerator corpus frontmatter validate`,
  compiler-enforced `Record<DocTypeKey, …>` maps, committed VR baselines); the
  parity/count scope is enumerated rather than left as "every assertion".

### Recommended Changes

1. **Make AC7 verifiable** (addresses: "Implemented visuals match the updated
   prototype"). Either reframe AC7 as an explicit human-judged design-review
   checkpoint — naming the reviewer and the comparison artefacts, as epic 0121
   does for its output-quality gate — or decompose it into mechanically checkable
   sub-claims (glyph/big-glyph components exist, colour tokens present, VR
   baselines committed and green) and drop the unqualified word "match".

2. **Give the indexer fixture negative and skip coverage** (addresses: Indexer
   negative and skip assertions). Specify that the fixture set (or an added
   fixture) contains a `findings/` document, a `reports/` document, and a
   dot-prefixed in-flight directory, so the indexer test positively confirms
   exactly one entry is emitted and the others are skipped.

3. **Record the 0279 downstream coupling** (addresses: Downstream coupling to
   0279 uncaptured). Add 0279 as a Blocks entry or an explicit Dependencies note
   stating that 0279's `finalise`/lifecycle work builds on the base-`status`
   collapse and the `complete` vocab this story provisions.

4. **Assert the colour constraints as criteria** (addresses: contrast/hue not
   asserted; "legible colour" unquantified). Add a criterion that the chosen
   light foreground/background pair measures ≥3:1 contrast against the page
   background and its hue differs from `research`'s hue 28, and that a distinct,
   non-default chip colour mapping is present for each of the five lifecycle
   states.

5. **Add a dangling-reference integrity criterion** (addresses: whole-corpus
   dangling-reference integrity not in definition of done). Capture that, once
   the indexer registers the set, a `topic-research:<slug>` reference passes
   whole-corpus dangling-reference integrity — mirroring the verification 0277
   defers to the co-land.

6. **Surface the external tooling gates in Dependencies** (addresses: Claude
   Design and VR harness absent). Add the Claude Design prototype pass (noting its
   precede-implementation ordering) and the Docker/Linux VR harness as
   completion-gating couplings with their availability implications.

7. **Clarity polish** (addresses: the four clarity findings). Enumerate the
   predicates by name with intended values and separate `nested_manifest_filename()`
   from the boolean group; expand "VR" on first use; replace "its indexer" with
   "this story's indexer"; name `research_status` in the Summary; and state which
   lifecycle value the corpus-cli fixture carries.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: 0278 is a dense but generally precise work item whose scope, the
manifest-status collapse, and the co-land relationship with 0277 are internally
consistent across Summary, Context, Requirements, and Acceptance Criteria, and
which defines its domain terms inline (e.g. `DesignInventories` glossed as "the
other nested-manifest Discover type"). The clarity gaps are minor: one muddled
"six boolean predicates" reference, a repeatedly unexpanded "VR" acronym, an
ambiguous "its indexer" pronoun, and a couple of Summary/fixture referents that
are only pinned down by later sections. No unreconciled contradiction was found.

**Strengths**:
- The manifest-status collapse is stated identically and consistently across
  Summary, Context, Requirements, the 0277-delta list, and AC5.
- The apparent tension between "Blocked by: 0277" and "Co-land with 0277" is
  explicitly reconciled in both prose and frontmatter (`relates_to` instead of a
  reciprocal `blocked_by` to avoid a block cycle).
- Domain jargon is defined at point of use; the VR "darwin and linux" phrasing
  inherited from the parent epic is explicitly corrected in Assumptions.
- Cross-work-item referents (0277 as "the engine", 0284 as "the set-level detail
  page", 0279 as the story that lands `finalise`) resolve unambiguously.

**Findings**:
- 🔵 minor / medium — **"Six boolean predicates" is unenumerated and mislabels a
  non-boolean method** (Requirements). None of the six predicates are named, and
  `nested_manifest_filename()` — which returns `Some("manifest.md")`, not a
  boolean — is grouped among them, so the count and identity of the intended
  methods cannot be determined from the work item alone. Suggestion: enumerate the
  boolean predicates by name with true/false values and separate
  `nested_manifest_filename()` out.
- 🔵 suggestion / medium — **"VR" acronym used repeatedly without expansion**
  (Requirements / Technical Notes). Appears ~5 times, spelled out only in epic
  0121. Suggestion: expand to "visual-regression (VR)" on first use.
- 🔵 minor / medium — **"its indexer" has an ambiguous antecedent (0277 vs this
  story)** (Dependencies). The nearest antecedent is 0277, yet the indexer is
  delivered by 0278. Suggestion: replace with "this story's indexer".
- 🔵 suggestion / low — **Summary names the collapse target but not the source
  field being removed** (Summary). Does not name `research_status`. Suggestion:
  name it in the Summary.
- 🔵 suggestion / low — **"The lifecycle value" for the corpus-cli fixture is
  unspecified** (Requirements). Definite article for a value never identified,
  unlike the visualiser fixture pinned to `synthesised`. Suggestion: state which
  value, or reword to "a lifecycle value".

### Completeness

**Summary**: An exemplary, highly complete story. Every expected section is
present and densely populated; the frontmatter is complete and well-formed with
a recognised `kind: story`; the Summary is a clear As-a/I-want/so-that with an
identified beneficiary; and there are seven specific Given/When/Then acceptance
criteria. The only completeness observation is that a deliverable the sibling
story (0277) explicitly defers to this item's co-land — whole-corpus
dangling-reference integrity — is not captured in this item's Requirements or
Acceptance Criteria.

**Strengths**:
- All expected sections present and substantively populated — no empty,
  placeholder, or sparse sections; even Open Questions carries a real answer.
- The Summary is a complete As-a/I-want/so-that story naming beneficiary and
  benefit.
- The Context explains why the work is needed and adds investigation-derived
  facts rather than restating the Summary.
- Seven concrete Given/When/Then acceptance criteria spanning rendering,
  indexing, status-chip, click behaviour, the status collapse, registry parity,
  and the design prompt.
- Frontmatter complete and correct; Requirements grouped into five clear
  workstreams, each specific enough to start without follow-up.

**Findings**:
- 🔵 minor / medium — **Whole-corpus dangling-reference integrity, deferred here
  by sibling 0277, is not captured in this item's definition of done**
  (Acceptance Criteria). 0277 and 0121 state that "full dangling-reference
  integrity (whole-corpus mode) co-lands with the 0278 indexer", yet 0278 never
  mentions it. An implementer reading only 0278 has no criterion telling them the
  whole-corpus linkage-integrity check is part of this story's done. Suggestion:
  add a criterion that a `topic-research:<slug>` reference resolves once the
  indexer lands.

### Dependency

**Summary**: The Dependencies section captures the primary couplings clearly —
blocked-by and co-land with 0277 (with an explicit, well-reasoned block-cycle
avoidance), Blocks 0284, and the on-branch overlap where this story modifies
corpus/skill code 0277 landed. The main gap is an uncaptured downstream coupling
to sibling 0279: this story collapses the set lifecycle onto the manifest's base
`status` and introduces the `complete` vocab that 0279's `finalise` consumes —
named in Assumptions but absent from Blocks. A secondary gap is that
completion-gating external tooling (Claude Design) and the Docker/Linux VR
harness live only in Notes/Assumptions, not in the coupling record.

**Strengths**:
- The co-land relationship with 0277 is captured precisely, including the
  rationale for `relates_to` over a reciprocal `blocked_by`.
- The on-branch code-overlap coupling (the `research_status` → base `status`
  collapse modifying code 0277 landed) is named directly.
- The downstream consumer 0284 is captured as a Blocks entry with its reason.

**Findings**:
- 🟡 major / medium — **Downstream coupling to 0279 (status-collapse consumer)
  uncaptured** (Dependencies). Assumptions notes `complete` "is only written once
  0279's `finalise` lands", and this story provides the base-`status` model 0279
  extends, yet 0279 is absent from Dependencies and `blocks` lists only 0284. A
  planner could schedule 0279 before this delta lands, leaving `finalise` writing
  `complete` against a `status_vocab` this story has not yet introduced.
  Suggestion: record 0279 as a downstream dependant via a Blocks entry or explicit
  note.
- 🔵 minor / medium — **Claude Design tool and Docker/Linux VR harness gate
  completion but are absent from Dependencies** (Dependencies). The external
  Claude Design canvas the visuals must match and the pinned Docker/Linux VR
  harness are named only in Requirements/Assumptions/Notes. If either is
  unavailable, AC7 and the VR-baseline criterion cannot be met, but that risk is
  invisible in the coupling record. Suggestion: add both with their ordering and
  availability implications.

### Scope

**Summary**: 0278 is a well-bounded, coherent story: everything in the visualiser
doc-type registration, nested-manifest indexer, and library rendering serves the
single user-visible capability named in the Summary. Its one scope wrinkle is the
deliberately absorbed 0277-delta (collapsing `research_status` onto base
`status`), which reaches into corpus/schema/template/skill/fixture code, and a
hard co-land coupling to 0277 that means the story cannot be merged
independently. Both are transparently declared and traceable to the epic's
designed decomposition seam, so they are low-severity unit-of-delivery
observations rather than genuine bundling of unrelated work.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria describe the same three-part
  scope; the extra concern is declared up front, not smuggled in.
- Strong boundary discipline: 0284 concerns are explicitly excluded and rendering
  routes through the shared `LibraryDocView`.
- The visualiser registration is a genuinely indivisible unit — enum variant, six
  predicates, count/parity assertions, and frontend `Record<DocTypeKey,…>` maps
  must all land together or the build/parity checks fail.
- The delta-absorption and co-land coupling are grounded in epic 0121's designed
  seam, not accidental scope creep.

**Findings**:
- 🔵 minor / medium — **The 0277-delta status collapse is the one requirement
  group separable from the visualiser goal** (Requirements: The 0277-delta). It
  touches corpus-layer code 0277 landed rather than the visualiser surface, and
  could alternatively have lived in 0277. Suggestion: keep it here (justified —
  the card reads base `status`, so without the collapse every card reads
  "complete", and the two co-land anyway), but retain the explicit
  Summary/Context framing.
- 🔵 suggestion / medium — **The story is not independently deliverable**
  (Dependencies). It must co-land with 0277, coupling their schedules and
  rollback. Suggestion: no change needed — a deliberate, documented split; flagged
  so the co-land is understood as an accepted coupling.

### Testability

**Summary**: The Acceptance Criteria are mostly strong: most are framed as
Given/When/Then with concrete, observable outcomes anchored to mechanical gates
(`mise run check`, `accelerator corpus frontmatter validate`, compiler-enforced
`Record<DocTypeKey, …>` maps, and committed VR baselines). Two verification
weaknesses stand out: AC7's "implemented visuals match the updated prototype" has
no defined comparison procedure, and the single manifest-only fixture cannot
exercise AC2's negative/skip assertions. A concretely measurable constraint (≥3:1
contrast, hue clear of `research`) also appears in Requirements but in no
criterion.

**Strengths**:
- Most criteria are Given/When/Then with observable outcomes tied to definitive
  mechanical gates (`mise run check`, `frontmatter validate`, Docker/Linux VR
  baselines).
- AC2 quantifies the indexer outcome precisely ("exactly one library entry keyed
  on `manifest.md`").
- The parity/count work is enumerated (the `[Self; 14]` array, exhaustiveness
  test, `parity.rs`, `api_types.rs` len==14, `catalogue.rs`) and map completeness
  is compiler-enforced.
- AC5 pins a concrete runnable check — `frontmatter validate` passes for a
  manifest with base `status: briefed` and `synthesised`.

**Findings**:
- 🟡 major / high — **"Implemented visuals match the updated prototype" has no
  defined pass/fail procedure** (AC7). The VR baselines are generated from the
  implementation itself and only detect drift; they do not compare against the
  Claude Design prototype, which has no pixel-diff harness against the code. A
  verifier cannot conclusively decide whether the criterion is met. Suggestion:
  reframe as a human-judged design-review checkpoint, or decompose into
  mechanically checkable sub-claims and drop "match".
- 🔵 major / medium — **Indexer negative and skip assertions lack a fixture that
  can exercise them** (AC2 / Requirements Fixtures). The only checked-in fixture
  is a set with a single `manifest.md`; with no `findings/`, `reports/`, or
  dot-prefixed sibling, the negative and skip clauses are vacuously true. The
  indexer could regress and every test would still pass. Suggestion: add those
  entries to the fixture so the test positively confirms one entry and the others
  skipped.
- 🔵 minor / medium — **Measurable contrast/hue constraint is stated in
  Requirements but asserted by no criterion** (AC / Requirements Library
  placement). The ≥3:1 contrast and hue-distinctness constraints are pinned by no
  criterion; AC1's VR baselines only pin shipped pixels. Suggestion: add a numeric
  criterion.
- 🔵 minor / medium — **"Legible colour" for the status chip is unquantified and
  exercises only one of five states** (AC3). "Legible" has no threshold and AC3
  exercises only `synthesised`, leaving the other four mappings unverified.
  Suggestion: replace with a concrete contrast check and require a distinct
  mapping for each of the five states.

## Re-Review (Pass 2) — 2026-09-10

**Verdict:** REVISE

Re-ran all five lenses against the edited work item. All three pass-1 majors are
resolved and the minor/suggestion coverage gaps are closed, except the two
scope items intentionally left unchanged. The verdict stays REVISE because
tightening the acceptance criteria surfaced two new testability majors: an
under-specified input in the dangling-reference criterion that was added (AC9),
and a pre-existing card title/slug verification gap now exposed by the sharper
indexing criteria.

### Previously Identified Issues

- 🟡 **Testability** (AC7 "match the prototype" had no pass/fail procedure) — Resolved. Re-review cites AC7 as cleanly separating the human-judged sign-off from the mechanical VR drift guard.
- 🟡 **Testability** (indexer negative/skip assertions had no exercising fixture) — Resolved. Re-review calls AC2 "exemplary" now that the fixture provisions a `findings/` doc, a `reports/` doc, and a dot-prefixed sibling.
- 🟡 **Dependency** (0279 downstream coupling uncaptured) — Resolved. 0279 now carried on `blocks` and in Dependencies with rationale.
- 🔵 **Completeness** (dangling-reference integrity not in definition of done) — Resolved as a coverage gap (criterion added); see the AC9 refinement under New Issues.
- 🔵 **Testability** (contrast/hue asserted by no criterion) — Partially resolved. Criteria added (AC3, AC8); precision refinements remain (numeric hue delta, colour pair).
- 🔵 **Testability** (chip "legible colour", one state only) — Partially resolved. AC3 now requires ≥3:1 and five-state mappings; contrast still exercised for one state only.
- 🔵 **Clarity** ("six boolean predicates" miscount / mislabel) — Resolved.
- 🔵 **Clarity** ("its indexer" antecedent) — Resolved.
- 🔵 **Dependency** (Claude Design + VR harness gates absent) — Resolved. Both named in Dependencies with availability implications.
- 🔵 **Clarity** (VR acronym unexpanded) — Resolved.
- 🔵 **Clarity** (Summary omitted `research_status`) — Resolved.
- 🔵 **Clarity** (corpus-cli fixture value unspecified) — Resolved (pinned to `synthesised`).
- 🔵 **Scope** (0277-delta placement) — Still present. Deliberate; no change agreed.
- 🔵 **Scope** (not independently deliverable) — Still present. Deliberate; no change agreed.

### New Issues Introduced

- 🟡 **Testability** (Acceptance Criteria) — AC9, the added dangling-reference criterion, does not name the document carrying the `topic-research:<slug>` reference, so the whole-corpus check has an unspecified input. Suggestion: name the fixture (or corpus) document holding the reference to the fixture set's slug.
- 🟡 **Testability** (Requirements) — Card title/slug derivation (`build_entry`: title from manifest `title`, slug from parent directory, with fallbacks) is required but no criterion verifies the rendered title/slug. Pre-existing gap, surfaced now. Suggestion: add a criterion asserting the card title equals the manifest `title` and the slug equals the parent directory name.
- 🔵 **Testability** (AC8) — "hue distinct from `research` (28)" gives no numeric separation. Suggestion: state a minimum delta (e.g. ≥15°) and distinctness from the design-* hues.
- 🔵 **Testability** (AC3) — the ≥3:1 bar is exercised for only one of five states and the compared colour pair (chip text vs fill vs card/page background) is undefined; AC8 measures against the page background, a different surface.
- 🔵 **Testability** (AC7) — the human sign-off names no recorded artefact, so a verifier cannot confirm the gate occurred. Suggestion: note where it is captured (PR approval or a work-item note naming the reviewer).
- 🔵 **Dependency** (Dependencies) — the umbrella doc-type / frontend registration is a shared artefact also consumed by the academic (tier rendering) and consumption (`report` rendering) slices, understated in Blocks.
- 🔵 **Dependency** (Dependencies) — human design-reviewer availability is a completion coupling not listed among the gates.
- 🔵 **Clarity** (Dependencies) — the "AC7" cross-reference points into an unnumbered checkbox list; a reader must count to the seventh bullet.
- 🔵 **Clarity** (Summary) — "browse the dossiers" reads broader than the manifest-only scope delivered (sub-document navigation is 0284).
- 🔵 **Clarity** (Context) — "shared flat view" and "shared `LibraryDocView`" name the same component in one sentence.

### Assessment

The work item is materially stronger: every pass-1 major is closed, and the
clarity/dependency/completeness gaps are resolved. It is not yet at APPROVE
because two testability majors remain — both small, self-contained AC edits
(name the referencing document for AC9; add a card title/slug criterion).
Addressing those two would very likely clear the item, leaving only minor
precision refinements and the two deliberate scope observations. The verdict
remains REVISE per the major-count threshold (2 majors).

## Re-Review (Pass 3) — 2026-09-10

**Verdict:** COMMENT

Re-ran clarity, dependency, scope, and testability (completeness was clean at
pass 2). Both pass-2 majors are resolved. The verdict moves REVISE → COMMENT:
one major remains (below the two-major REVISE threshold), and it is a refinement
of a clause added during iteration. The work item is acceptable as-is; the
residual findings are precision refinements and the two deliberate scope calls.

### Previously Identified Issues

- 🟡 **Testability** (AC9 dangling-ref criterion had no named referencing document) — Resolved. AC9 now names the checked-in fixture document carrying `relates_to: ["topic-research:<slug>"]`, and the Fixtures list provisions it; cited as a strength.
- 🟡 **Testability** (card title/slug required but unverified) — Resolved. A dedicated criterion now asserts card title = manifest `title`, slug = parent directory, with the H1 → humanised-directory fallback.
- 🔵 **Testability** (AC8 hue had no numeric delta) — Resolved for `research` (≥15°); a new sub-clause on the design-* hues is now the residual major below.
- 🔵 **Testability** (AC3 contrast pair/states) — Resolved. The compared pair (chip text vs chip fill) is named and ≥3:1 is required for all five states.
- 🔵 **Testability** (AC7 sign-off had no recorded artefact) — Resolved. Sign-off recorded as a PR approval or a work-item note naming the reviewer; cited as a strength.
- 🔵 **Dependency** (shared-artefact consumers understated) — Resolved in prose; a machine-readability refinement remains below.
- 🔵 **Dependency** (reviewer availability not a gate) — Resolved. The human design reviewer is now a listed completion gate.
- 🔵 **Clarity** ("AC7" into an unnumbered list) — Resolved. The numeric references were replaced with "the design-sign-off criterion".
- 🔵 **Clarity** (Summary overshoot) — Partially resolved. The Summary now says "open to their manifest" and points navigation to 0284, but "read the research sets"/"reader-observable" still read slightly broad.
- 🔵 **Clarity** ("flat view" vs `LibraryDocView`) — Resolved. Unified to `LibraryDocView`.
- 🔵 **Scope** (0277-delta placement; not independently deliverable) — Still present. Deliberate; no change agreed.

### New Issues Introduced

- 🟡 **Testability** (Acceptance Criteria) — AC8's "distinct from the design-* type hues" clause has no enumerated design-* hue values and no separation threshold, so that half of the criterion is not definitively checkable. Suggestion: enumerate the design-* hues (design-inventories, design-gaps) and apply the same ≥15° separation.
- 🔵 **Testability** (Acceptance Criteria) — the title-fallback chain (absent title → first H1 → humanised directory) has no fixture exercising the fallback branches, and "humanised" defines no concrete transformation. Suggestion: add fixtures for the two fallback branches and state the expected humanised output.
- 🔵 **Dependency** (Dependencies) — 0280 and 0281 are named as shared-registration consumers in prose but absent from `blocks`; the transitive-through-0277 argument holds only while the co-land holds. Suggestion: add them to `blocks` or state the omission is intentional.
- 🔵 **Clarity** (Requirements) — the title fallback chain and the "≥15° / distinct from design-* hues" constraint live in the Acceptance Criteria but not the Requirements bullets an implementer would spec from; "design-*" is never expanded. Suggestion: fold both into Requirements and name the design-* types.
- 🔵 **Clarity** (Requirements) — the predicate-mirroring bullet ("not virtual and not in kanban") can be read as either the intended values or the differing booleans. Suggestion: state the values directly (virtual = false, in-kanban = false).
- 🔵 **Scope** — the two deliberate observations (co-land non-independence, delta placement) restated; no change.

### Assessment

The work item is in good shape and reads as acceptable (COMMENT). Both pass-2
majors are closed, and the iteration cleared the substantive clarity,
dependency, and completeness gaps. The one remaining major is a self-inflicted
sub-clause from tightening the hue criterion (enumerate the design-* hues and
their separation) — a one-line fix. The remaining minors are the classic
diminishing-returns tail: each tightening exposes a finer sub-clause (fixtures
for the title-fallback branches; Requirements/AC constraint parity; `blocks`
edges for 0280/0281). None blocks planning. Recommendation: optionally close the
one major and the Requirements/AC parity, then proceed — further passes are
unlikely to repay the effort.

## Verdict Override — 2026-09-10

**Verdict:** APPROVE (reviewer decision)

Toby Clemson approved the work item after the pass-3 recommendation was applied:
the remaining testability major (AC8's design-* hue clause) was closed by
enumerating `research`, `design-inventories`, and `design-gaps` with a ≥15°
separation, and the title-fallback and hue constraints were folded into
Requirements for AC/Requirements parity. These edits were not re-verified by a
further agent pass. The residual minor tail recorded above (fallback-branch
fixtures, 0280/0281 in `blocks`, stating the predicate booleans outright) and
the two deliberate scope observations are accepted as non-blocking.
