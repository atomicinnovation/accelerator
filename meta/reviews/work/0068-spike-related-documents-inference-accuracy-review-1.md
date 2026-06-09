---
date: "2026-05-24T00:00:00+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0068"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
id: "0068-spike-related-documents-inference-accuracy-review-1"
title: "0068-spike-related-documents-inference-accuracy-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-24T00:00:00+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: Spike: Evaluate `Related Documents` Body-Section Inference Accuracy

**Verdict:** REVISE

The spike is well-bounded around a single research question with a clear time-box, a concrete downstream consumer (ADR 0062), and structurally complete sections. The dominant gaps are not structural but semantic: the recommendation that the spike must deliver lacks a decision rubric, sample size, and verifiable success conditions, and the linkages between this spike and its sibling work items (0062 / 0069 / 0070) are introduced unevenly across sections, leaving readers to reconcile their relationships from the parent epic. Three `major` findings (two testability, one dependency) trigger the REVISE verdict.

### Cross-Cutting Themes

- **Ambiguous identity and role of sibling work items 0062 / 0069 / 0070** (flagged by: clarity ×2, dependency ×2) — the Summary references 0069/0070 without context, Acceptance Criteria introduces 0062 without prior mention, and Dependencies qualifies 0069/0070 as "possibly blocked" despite the Summary treating them as direct consumers. The references chain is internally inconsistent.
- **The recommendation is not verifiable** (flagged by: clarity, testability ×2) — the spike's central deliverable is a recommendation between two paths, but no threshold maps accuracy counts to that decision, no sample-size floor is specified, and the manual-classification labels ("correct/uncertain/wrong") are not consistently distinguished from parser-emitted confidence bands.

### Findings

#### Major
- 🟡 **Dependency**: Downstream consumers 0069 and 0070 are weakly qualified as 'possibly' blocked despite being named as direct consumers in the Summary
  **Location**: Dependencies
  The Summary unambiguously names 0069/0070 as consumers of the spike's recommendation, but Dependencies lists them only as "possibly", understating the coupling.

- 🟡 **Testability**: No threshold defined for choosing interactive vs deterministic
  **Location**: Acceptance Criteria
  Without a defined decision rule (e.g. "<X% wrong AND <Y% uncertain → deterministic"), two reviewers could read the same counts and reach opposite recommendations. The criterion cannot be conclusively passed or failed.

- 🟡 **Testability**: Sample size for accuracy measurement is unspecified
  **Location**: Acceptance Criteria / Assumptions
  The "~100 inferences" figure appears as a feasibility note in Assumptions, not as a verification requirement. A run producing 12 inferences would technically satisfy the criterion yet be statistically too thin to support a recommendation.

#### Minor
- 🔵 **Clarity**: Forward-referenced work items 0069 and 0070 lack context on first mention
  **Location**: Summary
  A reader encountering this work item first cannot tell whether 0069/0070 are existing items, planned siblings, or hypothetical IDs.

- 🔵 **Clarity**: Three-way classification scheme uses slightly different labels between Requirements and Acceptance Criteria example
  **Location**: Requirements / Acceptance Criteria
  Requirements say "correct / plausibly-correct-but-uncertain / wrong"; the AC example substitutes "confident" for "correct". Combined with the confidence-band note in Technical Notes, it's unclear whether parser-emitted confidence is the same as manual-correctness verdict.

- 🔵 **Clarity**: "Recommendation for 0062" mentioned in AC without prior reference in Summary or Context
  **Location**: Acceptance Criteria
  0062 is introduced for the first time in the AC; readers cannot tell from this work item alone whether 0062 is the ADR for 0070 or a separate decision.

- 🔵 **Dependency**: Recommendation target named as '0062' in body but ADR identifier convention may not match work-item numbering
  **Location**: Acceptance Criteria
  Ambiguity over whether '0062' refers to a work item creating the ADR or to the ADR's own `adr_id`.

- 🔵 **Testability**: 'Working prototype' has no observable success condition
  **Location**: Acceptance Criteria (prototype parser)
  A trivial parser that emits zero linkages would arguably satisfy "runs end-to-end", defeating the spike's purpose.

- 🔵 **Testability**: 'Failure patterns' criterion lacks an enumeration target
  **Location**: Requirements / Acceptance Criteria
  No minimum count or coverage condition — one pattern would satisfy the AC literally.

#### Suggestions
- 🔵 **Clarity**: Passive phrasing obscures who performs the manual classification
  **Location**: Requirements
  Single-rater vs multi-rater classification is unstated.

- 🔵 **Completeness**: Frontmatter uses `kind` instead of `type`
  **Location**: Frontmatter
  Note: this aligns with the rename committed in parent epic 0057 (`type:` → `kind:`), so the finding is likely a false positive against an updated convention.

- 🔵 **Scope**: Prototype-reuse question hints at potential scope creep
  **Location**: Open Questions
  Resolve before promotion to `ready`: explicitly state the prototype is throwaway, consistent with Technical Notes.

- 🔵 **Testability**: Confidence-band scoring is suggested but not made verifiable
  **Location**: Technical Notes / Acceptance Criteria
  Either promote confidence-band scoring into AC or explicitly mark it optional.

### Strengths
- ✅ Single coherent research question with a bounded corpus (this repo's `meta/`) and explicit 1–2 day time-box plus fallback condition.
- ✅ Acceptance Criteria enumerate concrete deliverables (prototype, research artifact at a named location, Findings section) with required contents.
- ✅ Context meaningfully explains the motivation via parent epic 0057's open question 3, rather than restating the Summary.
- ✅ Frontmatter is complete and uses recognised values; structural sections (Requirements, AC, Dependencies, Assumptions, Open Questions, Technical Notes) are populated and substantive.
- ✅ Domain terminology ("typed linkage", "unified-schema migration", "interactive validation hooks") is anchored through the parent epic reference.

### Recommended Changes

1. **Define a decision rubric in Acceptance Criteria** (addresses: testability major — recommendation threshold)
   Add an AC like: "Recommend deterministic + report if wrong-rate ≤ X% AND uncertain-rate ≤ Y% on a sample of ≥ N inferences; otherwise recommend interactive hooks." Concrete thresholds can be drafted now and refined when findings land.

2. **Promote sample-size floor into Acceptance Criteria** (addresses: testability major — sample size)
   Lift "~100 inferences" (or equivalent floor) from Assumptions into an AC, or add "the run must cover all `meta/` body sections matching the parser's target headings."

3. **Rewrite Dependencies to reflect actual coupling strength** (addresses: dependency major)
   Promote 0070 to an unqualified Blocks entry (it consumes the recommendation in either branch). Either keep 0069 as "conditionally blocks" with explicit conditions, or move it to Related with a note that it becomes blocked only if interactive hooks are recommended.

4. **Introduce 0062 in Context and disambiguate sibling references** (addresses: clarity minor ×2, dependency minor)
   Add a one-sentence intro in Context naming 0062 ("the migration-strategy ADR work item 0062 will cite this spike's findings"). On first mention of 0069/0070 in the Summary, qualify them as "sibling work items". Clarify whether 0062 refers to a work-item ID or an `adr_id`.

5. **Tighten the prototype AC with an observable success condition** (addresses: testability minor — working prototype)
   Replace "working prototype ... runs end-to-end" with "parser runs end-to-end against `meta/` and emits at least one candidate linkage record per qualifying body section, with each record including source path, target reference, inferred type, and confidence band."

6. **Align classification labels across Requirements and AC** (addresses: clarity minor — labels)
   Use the same three labels in both sections. Explicitly distinguish manual-classification verdicts from parser-emitted confidence scores.

7. **Tighten failure-patterns AC** (addresses: testability minor — failure patterns)
   Add "every wrong or uncertain inference is attributed to a named pattern in the catalogue" or "at least one failure pattern per category of wrong/uncertain inferences."

8. **Resolve the prototype-reuse Open Question before `ready`** (addresses: scope suggestion)
   State explicitly that the prototype is throwaway; defer any production-parser construction to a follow-on story under 0057.

9. **Name the classifier and rater-count** (addresses: clarity suggestion)
   "The spike implementer manually classifies each inferred linkage … single-rater classification is acceptable for this time-box."

10. **Decide on confidence-band scoring** (addresses: testability suggestion)
    Either promote to AC ("the research artifact reports accuracy per band") or explicitly note bands are optional.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is largely clear and well-structured, with an unambiguous overall intent. The actor is consistently the spike implementer, and the parent epic (0057) provides shared vocabulary. A few minor referential gaps exist around sibling work items 0069/0070 and the relationship between manual-correctness labels and parser-emitted confidence bands.

**Strengths**:
- Summary clearly states what is being prototyped, the corpus, and the decision it informs.
- Acceptance Criteria identify outcomes (working prototype, research artifact, Findings section) as observable artifacts.
- Cross-section scope is internally consistent.
- Domain terminology is anchored through the parent epic reference.

**Findings**:
- 🔵 minor — Forward-referenced work items 0069 and 0070 lack context on first mention (Summary).
- 🔵 minor — Three-way classification scheme uses slightly different labels between Requirements and Acceptance Criteria example.
- 🔵 minor — "Recommendation for 0062" mentioned in AC without prior reference in Summary or Context.
- 🔵 suggestion — Passive phrasing obscures who performs the manual classification (Requirements).

### Completeness

**Summary**: Structurally complete and well-populated for a spike: clear scoped question, time-box, enumerable exit criteria, populated Context, Requirements, AC, Dependencies, Assumptions, Open Questions, Technical Notes. Frontmatter is valid. Only one suggestion, on field-name convention.

**Strengths**:
- Spike-appropriate content: scoped question, time-box, enumerable exit criteria.
- Context references epic 0057 rather than restating the Summary.
- Acceptance Criteria are specific and concrete.
- Frontmatter uses recognised values.

**Findings**:
- 🔵 suggestion — Frontmatter uses `kind` instead of `type` (likely a false positive; aligns with the rename committed in parent epic 0057).

### Dependency

**Summary**: The spike captures its key downstream consumer (0062) explicitly and correctly notes no upstream blockers. However, the Summary names 0069/0070 as direct consumers while Dependencies qualifies them only as "possibly" blocked. The parent linkage (0057) is captured as Related; no external systems or cross-team actions are implied.

**Strengths**:
- Upstream coupling correctly characterised: "Blocked by: none" with rationale.
- Primary downstream consumer (0062) named as a Blocks entry.
- Parent epic 0057 captured as Related, matching frontmatter `parent`.
- No external systems implied; spike is self-contained.

**Findings**:
- 🟡 major — Downstream consumers 0069 and 0070 are weakly qualified as 'possibly' blocked despite being named as direct consumers in the Summary.
- 🔵 minor — Recommendation target named as '0062' in body but ADR identifier convention may not match work-item numbering.

### Scope

**Summary**: The spike is well-bounded around a single, specific research question. Sizing and decomposition are appropriate for a spike feeding ADR 0062. One open question hints at potential scope creep around prototype reuse.

**Strengths**:
- Single coherent research question with defined decision.
- Explicit 1–2 day time-box with fallback.
- Bounded corpus.
- Requirements and AC describe the same scope.
- Clear downstream consumer (ADR 0062).

**Findings**:
- 🔵 suggestion — Prototype-reuse question hints at potential scope creep; resolve before promotion to `ready`.

### Testability

**Summary**: Concrete exit criteria with named deliverables, but several criteria contain measurability gaps. Accuracy counts are exemplified but no minimum sample size or pass/fail threshold is defined, and the recommendation criterion lacks a decision rubric.

**Strengths**:
- AC enumerate concrete deliverables.
- Research artifact criterion specifies required contents (counts, failure patterns, recommendation).
- Time-box is explicit with defined termination action.
- Requirements list a specific classification scheme.

**Findings**:
- 🟡 major — No threshold defined for choosing interactive vs deterministic (Acceptance Criteria).
- 🟡 major — Sample size for accuracy measurement is unspecified (Acceptance Criteria / Assumptions).
- 🔵 minor — 'Working prototype' has no observable success condition.
- 🔵 minor — 'Failure patterns' criterion lacks an enumeration target.
- 🔵 minor — Confidence-band scoring is suggested but not made verifiable.

## Re-Review (Pass 2) — 2026-05-24

**Verdict:** COMMENT

All three pass-1 majors are resolved (sample-size floor lifted into AC; pre-committed numeric rubric added; Dependencies rewritten with 0070 unqualified Blocks and 0069 Conditionally blocks). All six pass-1 minors are resolved. The single new major (rubric refinement escape hatch) is below the REVISE threshold (≥ 2 majors), so the work item is acceptable as-is, but the refinement clause weakens the rubric's value as a pre-registered decision rule and is worth addressing before promoting to `ready`.

### Previously Identified Issues

- ✅ 🟡 **Dependency**: Downstream consumers 0069/0070 weakly qualified — **Resolved**. 0070 promoted to unqualified Blocks; 0069 explicitly Conditionally blocks with the activating condition stated.
- ✅ 🟡 **Testability**: No threshold for choosing interactive vs deterministic — **Partially resolved**. Concrete numeric rubric committed (wrong ≤ 5%, uncertain ≤ 15%, sample ≥ 100), but a new finding flags the refinement clause as an escape hatch (see New Issues).
- ✅ 🟡 **Testability**: Sample size unspecified — **Resolved**. 100-inference floor lifted into AC with a corpus-exhaustion fallback for thinner corpora.
- ✅ 🔵 **Clarity**: 0069/0070 lack context on first mention — **Resolved**. "Sibling work item" qualifier added in Summary.
- ✅ 🔵 **Clarity**: Labels diverge between Requirements and AC — **Resolved**. Both sections now use correct/uncertain/wrong, with the manual-verdict / parser-confidence distinction stated.
- ✅ 🔵 **Clarity**: 0062 introduced cold in AC — **Resolved**. Context now names 0062 explicitly as the migration-strategy ADR work item.
- ✅ 🔵 **Dependency**: '0062' ID convention ambiguous — **Resolved**. Now phrased as "work item 0062 (migration-strategy ADR creation)" throughout.
- ✅ 🔵 **Testability**: 'Working prototype' has no observable success condition — **Resolved**. Record format spec (source path, target reference, inferred type, confidence band) and "at least one record per qualifying body section" both added.
- ✅ 🔵 **Testability**: Failure-pattern enumeration target — **Partially resolved**. Now requires per-inference attribution for every wrong/uncertain, though a new minor flags a trivial-naming loophole (see New Issues).
- ✅ 🔵 **Testability**: Confidence-band scoring not in AC — **Resolved**. Promoted to a hard AC requirement and reflected in Technical Notes.
- ✅ 🔵 **Clarity**: Classifier role passive — **Resolved**. "The spike implementer manually classifies … single-rater classification is acceptable" added to Requirements.
- ✅ 🔵 **Scope**: Prototype-reuse Open Question — **Resolved**. Throwaway prototype committed in Technical Notes; Drafting Notes carries the resolution rationale; Open Questions section now empty.
- ⚪ 🔵 **Completeness**: `kind` vs `type` frontmatter — **Skipped (false positive)** per project convention 0057.

### New Issues Introduced

- 🟡 **Testability**: Rubric refinement clause is an unbounded escape hatch
  **Location**: Acceptance Criteria (decision rubric)
  Any post-hoc threshold can be justified as "refined" so long as it is written down, which makes the headline rubric effectively unverifiable. Suggestion: drop the placeholder framing and treat the thresholds as binding (with a separate work item required to change them), or constrain the refinement to a narrow window with a documented justification rule (e.g. "thresholds may move ≤ ±5 percentage points, only if a named failure-pattern justifies the change").

- 🔵 **Clarity** / **Testability**: "Qualifying body section" undefined
  **Location**: Acceptance Criteria
  The qualifying-section set is exemplified in the Summary (three header strings) but not committed as a definition. Add a one-liner pinning the exact match rule (e.g. case-insensitivity, depth, near-miss handling) or require the research artifact to record the actual matched-heading set.

- 🔵 **Clarity**: "That question" referent in Context
  **Location**: Context
  Multi-clause antecedent reference. Either restate directly or hoist the parser-band vs manual-verdict distinction one paragraph earlier so "confidence" is unambiguous on first use.

- 🔵 **Clarity**: Rubric-refinement ordering ambiguity
  **Location**: Acceptance Criteria
  "Before the recommendation is stated" can be read as a process gate or as documentation order. Pick one and state it directly. (Largely subsumed by the major finding above.)

- 🔵 **Clarity**: "Is moot" colloquial
  **Location**: Dependencies (0069 conditional)
  Replace with a precise dispositional statement (e.g. "0069 will be closed without action" or "0069 remains on the backlog but is not blocked by this spike").

- 🔵 **Dependency**: 0069 needs a back-reference
  **Location**: Dependencies
  Once 0069 exists, it should carry `blocked_by: 0068` (or equivalent conditional note) so a planner reading 0069 in isolation discovers the gate. Note this as a follow-up when 0069 is written.

- 🔵 **Dependency**: Corpus snapshot not pinned
  **Location**: Dependencies / Assumptions
  The spike runs against `meta/` "as it exists today" but no VCS revision is pinned and no rule prevents concurrent corpus-mutating migrations during the time-box. Add a note: capture the revision at spike start and avoid concurrent migrations.

- 🔵 **Testability**: Failure-pattern catalogue admits trivial singleton naming
  **Location**: Acceptance Criteria (failure-pattern catalogue)
  Every failure could be given its own one-off pattern name and satisfy the criterion. Tighten to require generalisation (e.g. "each named pattern is attested by ≥ 2 failures, or explicitly labelled as a singleton").

- 🔵 **Testability**: Single-rater classification has no calibration check
  **Location**: Requirements
  A different rater could plausibly flip the recommendation. Either add a lightweight calibration step (e.g. blind re-label 10% of the sample a day later) or explicitly note single-rater variance as a recorded caveat in the artifact.

- 🔵 **Dependency** (suggestion): Epic 0057's open question 3 coupling is narrated only
  **Location**: Dependencies / References
  The spike resolves a specific named open question in 0057 but the linkage lives only in Context prose. Once typed linkages are available, encode this as `resolves: 0057#open-question-3` or equivalent so the resolution is machine-discoverable. Defer until 0057 ships typed linkages.

### Assessment

The work item is materially improved and acceptable for implementation as-is. The single remaining major (rubric refinement escape hatch) is a cheap fix that would meaningfully strengthen the spike's value as a pre-registered decision, and the "qualifying body section" gap is worth closing because it appears in both the corpus-exhaustion fallback and the per-record requirement. The remaining minors are polish-level and can either be addressed in a brief follow-up pass or noted and deferred.

## Approval (Pass 3) — 2026-05-24

**Verdict:** APPROVE

The pass-2 major finding (rubric refinement escape hatch) has been resolved: the rubric thresholds are now binding for this spike's verdict, with a defined escape path (record borderline observations and propose alternatives for a follow-on re-run, do not rewrite in place). The remaining minor findings are polish-level and acceptable to carry into implementation. The work item is approved for promotion to `ready`.
