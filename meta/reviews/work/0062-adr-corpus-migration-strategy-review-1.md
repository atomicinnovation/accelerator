---
date: "2026-05-26T10:45:00Z"
type: work-item-review
producer: review-work-item
target: "work-item:0062"
work_item_id: "0062"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 5
status: complete
id: "0062-adr-corpus-migration-strategy-review-1"
title: "0062-adr-corpus-migration-strategy-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-26T10:45:00Z"
last_updated_by: Toby Clemson
---

## Work Item Review: 0062 — ADR: Interactive Validation for Corpus Migration

**Verdict:** REVISE

The work item is structurally complete and the five decision points map cleanly between Requirements and Acceptance Criteria, with spike 0068's numerical findings quoted consistently throughout. Two AC weaknesses around unbounded language ("fully specifies" the hook contract; identifying ADR-0023 clauses) leave room for a content-thin ADR to pass review, and a cross-cutting concern about scope and dependency capture surfaces around the vocabulary-gap and broad ADR-0023 amendment routing decisions. Verdict is REVISE on the strength of the two major testability findings.

### Cross-Cutting Themes

- **Vocabulary-gap routing into this ADR** (flagged by: scope, dependency, testability, clarity) — multiple lenses converge on the in-place vocabulary-policy decisions being under-handled: scope questions whether they belong here at all, dependency notes the implied ADR-0034 / 0061 amendment coupling isn't captured, testability flags that the `Source:`-prose half of AC 5 has no closed answer set, and clarity notes the shorthand assumes the reader has read spike 0068.
- **Broad ADR-0023 amendment scope** (flagged by: scope, testability) — scope flags that a permanent framework-contract change arguably warrants its own ADR; testability flags that AC 4's "identifies the specific clause(s)" lacks a granularity criterion.
- **Hook-contract specification** (flagged by: clarity, testability) — clarity flags a soft tension between the committed "interactive hooks" direction and the "adopt or reject hybrid" framing in Technical Notes; testability flags that AC 2's "fully specifies" gives no per-element pass criterion.

### Findings

#### Major
- 🟡 **Testability**: 'Fully specifies the hook contract' is unbounded without per-element pass criteria
  **Location**: Acceptance Criteria (AC 2)
  AC 2 requires the ADR to "fully specify" the hook contract across four elements (trigger, what is shown, accept/edit/skip semantics, resumability) but provides no per-element pass criterion. A reviewer cannot conclusively decide whether a one-line treatment of resumability is "full specification".

- 🟡 **Testability**: 'Identifies the specific ADR-0023 clause(s)' lacks a verifiable minimum
  **Location**: Acceptance Criteria (AC 4)
  Granularity is unspecified — citing ADR-0023 by number, naming a section heading, or quoting the "no prompts" language are all defensible readings. A weak identification could be argued as meeting the AC while leaving downstream readers unable to locate the amended language.

#### Minor
- 🔵 **Clarity**: Hybrid-shape framing creates soft tension with committed 'interactive hooks' direction
  **Location**: Technical Notes
  Summary/AC #1 commit to interactive hooks, but Technical Notes says the ADR "can adopt or reject" the hybrid shape. Reader has to infer that hybrid is a refinement on top of interactive hooks rather than a competing alternative.

- 🔵 **Clarity**: `Source:` prose and 'broader workstream' shorthand assume reader has read spike 0068
  **Location**: Requirements (bullet 5) and Acceptance Criteria (bullet 6)
  Vocabulary-gap decision points are referenced in highly compressed form; readers who haven't read spike 0068's failure-pattern catalogue cannot tell what an example looks like or why it requires an ADR-level decision.

- 🔵 **Dependency**: '0030 (ADR template)' refers to ADR-0030, not work item 0030
  **Location**: Dependencies
  Work item 0030 is "Centralise PATH and TEMPLATE config arrays" — unrelated. The ADR-template decision lives in ADR-0030. The mixing of bare numbers across work-item IDs and ADR IDs is ambiguous; this entry is mislabeled in the Related list.

- 🔵 **Dependency**: Vocabulary-amendment coupling to ADR-0034 / work item 0061 not captured
  **Location**: Dependencies
  The ADR will functionally amend ADR-0034 (typed-linkage vocabulary) by resolving the in-place vocabulary gaps — analogous to how ADR-0023 is being amended — but only the ADR-0023 amendment coupling is explicitly named. Future readers searching for amendments to typed-linkage vocab will miss this ADR.

- 🔵 **Scope**: Vocabulary-gap resolution may be orthogonal to interactive-validation ADR
  **Location**: Requirements
  Spike 0068 explicitly flagged the `Source:` question as warranting "a small ADR within epic 0057" — i.e. a sibling ADR. Vocabulary resolutions don't depend on the interactive-hook contract and could be revised separately. Author has acknowledged the routing in Drafting Notes and invited reviewer challenge.

- 🔵 **Scope**: Broad ADR-0023 amendment scope arguably warrants its own ADR
  **Location**: Requirements
  Adding "a permanent optional interactive contract that future migrations may opt into" is a framework-contract change of broader applicability than the corpus-migration use case driving this work. Future migration authors looking for the optional interactive contract will land on a corpus-migration-titled ADR.

- 🔵 **Testability**: 'Reasoning grounded in spike 0068's calibration data' is judgement-laden
  **Location**: Acceptance Criteria (AC 3)
  "Grounded in" is a soft predicate — a single passing reference to the 88%/90% figures technically satisfies it, while a stricter reader might expect the decision rationale to materially use the indistinguishability finding.

- 🔵 **Testability**: 'Canonical interpretation of Source: prose' has no closed answer set
  **Location**: Acceptance Criteria (AC 5)
  Unlike the "broader workstream" half (new-vocab / reuse / documented-limitation), the `Source:`-prose half does not enumerate acceptable resolutions. A vague "the production parser should consider context" answer could be argued to satisfy the AC without deciding the canonical reading.

- 🔵 **Testability**: No AC verifies that vocabulary-gap resolutions are reflected back into the typed-linkage vocabulary
  **Location**: Acceptance Criteria (AC 6) / Requirements
  If the resolution is "new vocab type" or "reuse", there's no AC asserting the resulting vocabulary state is documented unambiguously in this ADR. Downstream 0069/0070 could still inherit an ambiguous vocabulary.

#### Suggestions
- 🔵 **Clarity**: 'Routing' of cheap-fix parser recommendations is informal phrasing
  **Location**: Summary
  Requirement 4 clarifies the meaning; only matters for readers who scan the Summary in isolation. Replace with "ownership of the spike's cheap-fix parser recommendations (this ADR, 0069, 0070, or out-of-scope)".

- 🔵 **Clarity**: 'Permanent optional contract' is undefined as a term of art
  **Location**: Requirements (bullet 3) and Acceptance Criteria (bullet 4)
  The phrase carries weight in AC #4 but is never defined — context makes meaning recoverable, but an explicit definition on first use in Context would help reviewers grade AC #4.

- 🔵 **Dependency**: ADR-0030 (template) and ADR-0033 / ADR-0034 are implicit prerequisites worth naming
  **Location**: Dependencies
  All prerequisites are done so no scheduling impact; the cost is purely traceability. Annotate 0060 / 0061 in Related to mark them as upstream decisions consumed/amended by this ADR.

- 🔵 **Testability**: No AC asserts the ADR conforms to the ADR-0030 template
  **Location**: Acceptance Criteria (overall)
  For a task whose deliverable is an ADR document, format conformance (status, decision-makers, supersedes, derived_from) is a natural pass/fail check that's currently absent.

### Strengths

- ✅ All standard work-item sections are present and contain substantive, non-placeholder content; Open Questions is explicitly closed out with rationale rather than left ambiguous.
- ✅ Frontmatter integrity is high: kind, status, priority, parent, and tags are all populated with recognised values.
- ✅ The five decision points map one-to-one between Requirements and Acceptance Criteria with identical vocabulary, making Requirements coverage trivially verifiable.
- ✅ Spike 0068's quantitative findings (11.3% wrong-rate vs 5% threshold; ~5.3% cheap-fix counterfactual; 88% vs 90% high/medium accuracy) are quoted identically across Summary, Context, and Acceptance Criteria — no internal contradiction.
- ✅ Pronouns and definite noun phrases resolve unambiguously throughout; no place where "it" or "the system" could plausibly mean two different things.
- ✅ Partitioning against sibling work items 0069 and 0070 is explicit: 0062 owns policy/contract, 0069 owns framework implementation, 0070 owns the migration.
- ✅ Routing decisions (cheap-fix parser, vocabulary gaps, broad-vs-narrow amendment) are surfaced in Drafting Notes with a "reviewer should reject if…" invitation.
- ✅ Upstream input (spike 0068) and downstream consumers (0069, 0070) are explicitly captured with reciprocal entries in their respective work items, and ADR-0023 amendment coupling is named with the specific reason and likely target clause.
- ✅ Each acceptance criterion is bound to a discrete, named decision the ADR must record; AC 4 and the second half of AC 5 enumerate closed sets of acceptable resolutions.

### Recommended Changes

1. **Decompose AC 2 into four sub-bullets, each with a minimum pass criterion** (addresses: 'Fully specifies the hook contract' is unbounded)
   Restructure as: trigger criterion (names exactly one confidence band or named predicate that fires the prompt); what is shown (enumerates the named display elements); accept/edit/skip semantics (one-sentence behavioural definition for each); resumability (names the persistence artefact and the re-entry semantics).

2. **Tighten AC 4 with a verifiable granularity bar** (addresses: 'Identifies the specific ADR-0023 clause(s)' lacks a verifiable minimum)
   Rewrite as: "The ADR quotes or names by section heading at least one specific clause from ADR-0023 (e.g. the no-prompts language flagged in Technical Notes) and states the replacement/addition."

3. **Decide and document the routing of the vocabulary-gap decisions** (addresses: vocabulary-gap orthogonality, vocabulary-amendment coupling, `Source:` lacks closed answer set, vocabulary resolutions not reflected back)
   Either (a) keep the resolutions in this ADR and (i) broaden title/Summary to make vocabulary-policy scope visible upfront, (ii) annotate 0061 / ADR-0034 in Related as the supplementation target alongside ADR-0023, and (iii) add a sub-AC requiring vocabulary terms be defined unambiguously when introduced/reused; or (b) split the vocabulary-gap decisions into a separate sibling ADR-task work item under epic 0057.

4. **Decide and document the routing of the broad ADR-0023 amendment** (addresses: broad amendment scope arguably warrants its own ADR)
   Either (a) keep bundled and add a sentence in the ADR itself explaining why the broad framework change is co-located with the migration-specific strategy, or (b) split into a sibling ADR (framework-contract evolution) that this migration-strategy ADR references.

5. **Fix the mislabeled `0030 (ADR template)` reference** (addresses: '0030 (ADR template)' refers to ADR-0030)
   Re-label as `ADR-0030 (ADR template)` (or drop the bare-number form) and audit the Related list for any other bare numbers that should be ADR-prefixed.

6. **Resolve the hybrid vs interactive framing tension in Technical Notes** (addresses: Hybrid-shape framing creates soft tension)
   Reword the hybrid bullet to make explicit that the interactive-hooks direction is committed (AC #1) and what remains open is whether the contract is hybrid (interactive only on low-confidence) or uniform (interactive on every inference).

7. **Sharpen AC 3 to require explicit use of the calibration figures** (addresses: 'Reasoning grounded in' is judgement-laden)
   Rephrase as: "The ADR's band-design rationale explicitly cites the 88%/90% high-vs-medium accuracy figures and states whether they support collapsing to two bands or sharpening the high-band gate."

8. **Add an AC for ADR-0030 template conformance** (addresses: No AC asserts the ADR conforms to the ADR-0030 template)
   Add: "The ADR conforms to the ADR-0030 template structure and frontmatter (status, decision-makers, supersedes, derived_from)."

9. **Add one-sentence examples for the vocabulary-gap shorthand** (addresses: `Source:` prose and 'broader workstream' shorthand assume reader has read spike 0068)
   Add a concrete example for each gap (e.g. a sample `- Source: meta/work/0057-...md` line on a plan artifact) so reviewers can interpret the decision points without paging into spike 0068.

10. **Define 'permanent optional contract' on first use** (addresses: 'Permanent optional contract' is undefined)
    Add a parenthetical in Context such as: "a permanent optional contract — i.e. the framework permanently supports interactive hooks as an opt-in capability, while the mechanical no-prompt path remains the default for migrations that do not opt in."

11. **Replace 'routing' phrasing in Summary with explicit ownership language** (addresses: 'Routing' of cheap-fix parser recommendations is informal)
    Replace with "ownership of the spike's cheap-fix parser recommendations (this ADR, 0069, 0070, or out-of-scope)".

12. **Annotate 0060 / 0061 in Related as upstream decisions consumed/amended by this ADR** (addresses: ADR-0030 / 0033 / 0034 are implicit prerequisites worth naming)
    Make the prerequisite relationship visible at a glance, especially given recommendation 3 may add an explicit amendment annotation to 0061 / ADR-0034.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear and internally consistent: pronouns resolve unambiguously, the spike's numerical findings are quoted consistently across Summary/Context/Acceptance Criteria, and the five decision points map cleanly between Requirements and Acceptance Criteria. The main clarity concerns are a soft tension between the committed 'interactive hooks' direction in the Summary/AC #1 and the 'adopt or reject hybrid' framing in Technical Notes, and a couple of compact phrases (`Source:` prose, 'broader workstream') whose meaning depends on cross-referencing the research doc.

**Strengths**:
- Spike 0068's quantitative findings (11.3% wrong-rate vs 5% threshold; ~5.3% cheap-fix counterfactual; 88% vs 90% high/medium accuracy) are quoted identically across Summary, Context, and Acceptance Criteria, eliminating any chance of internal contradiction on the numbers.
- The five decision points the ADR must resolve map one-to-one between Requirements and Acceptance Criteria, with the same vocabulary used in both sections — the reader can verify Requirements coverage against AC trivially.
- Pronouns and definite noun phrases ('the spike', 'this ADR', 'the framework') resolve unambiguously throughout — there is no place where 'it' or 'the system' could plausibly mean two different things.
- Drafting Notes explicitly flag the title-narrowing decision and invite reviewer rejection, removing what would otherwise be hidden ambiguity about whether the work item is committing to interactive hooks or remaining neutral.

**Findings**:
- 🔵 **minor / medium** — Technical Notes — Hybrid-shape framing creates soft tension with committed 'interactive hooks' direction. The Summary and AC #1 commit the ADR to interactive validation hooks, but Technical Notes states 'A hybrid shape ... is the spike's implicit recommendation — the ADR can adopt or reject it explicitly.' A reader has to infer that 'hybrid' is a refinement on top of the already-committed interactive direction rather than a competing alternative.
- 🔵 **minor / medium** — Requirements (bullet 5) and Acceptance Criteria (bullet 6) — `Source:` prose and 'broader workstream' shorthand assume reader has read spike 0068. The vocabulary-gap decision points are referenced in highly compressed form; a reader who has not read spike 0068 cannot tell what an example of the conflict looks like or why it requires an ADR-level decision.
- 🔵 **suggestion / low** — Summary — 'Routing' of cheap-fix parser recommendations is informal phrasing. Requirement 4 makes the meaning concrete, but a reader hitting the Summary first will not know that 'routing' means 'ownership assignment across four named options'.
- 🔵 **suggestion / low** — Requirements (bullet 3) and Acceptance Criteria (bullet 4) — 'Permanent optional contract' is undefined as a term of art. The phrase appears in Context, Requirements, AC, and Drafting Notes but is never explicitly defined.

### Completeness

**Summary**: Work item 0062 is structurally and informationally complete for a task that produces an ADR. Every standard section is present and substantively populated, frontmatter is well-formed, and kind-appropriate content (clearly scoped task definition with verifiable outcomes) is present.

**Strengths**:
- All standard work-item sections are present and contain substantive, non-placeholder content.
- Frontmatter integrity is high: kind (task), status (draft), priority (high), parent (0057), and tags are all populated with recognised values.
- Acceptance Criteria contain six specific, enumerated criteria each tied to a decision the ADR must make.
- Context substantively explains the forces behind the work item rather than restating the Summary.
- Open Questions is explicitly closed out with rationale rather than left empty ambiguously.
- References section enumerates source, research, and related artifacts.

**Findings**: (none)

### Dependency

**Summary**: The work item captures its principal couplings well: spike 0068 as the (now-completed) upstream input, work items 0069 and 0070 as downstream consumers, and ADR-0023 as the amendment target are all named explicitly with rationale. Two gaps remain: the Dependencies list contains a mislabeled reference ('0030 (ADR template)' likely refers to ADR-0030, not work item 0030, which is an unrelated config refactor), and the work item's stated intent to make vocabulary-policy decisions in-place implies an amendment of ADR-0034 / coupling to work item 0061 that is not captured alongside the explicit ADR-0023 amendment coupling.

**Strengths**:
- Downstream consumers 0069 and 0070 are explicitly listed in Blocks with rationale, matching reciprocal Blocked-by entries in 0069 and 0070.
- Upstream input (spike 0068) is captured in Blocked-by as 'none' with explanation that the spike is complete and its findings live in a named research artifact.
- ADR-0023 amendment coupling is named in Related with the specific reason, and Technical Notes identifies the likely target clause.
- Cross-ADR consumption of spike 0068 is explicitly described, and the cheap-fix routing decision is framed as resolving an ambiguity that would otherwise leak into 0069/0070.

**Findings**:
- 🔵 **minor / high** — Dependencies — '0030 (ADR template)' refers to ADR-0030, not work item 0030. Work item 0030 is 'Centralise PATH and TEMPLATE config arrays to scripts/config-defaults.sh' — unrelated. A reader following Dependencies will land on the wrong artifact.
- 🔵 **minor / medium** — Dependencies — Vocabulary-amendment coupling to ADR-0034 / work item 0061 not captured. The typed-linkage vocabulary is owned by ADR-0034 (work item 0061, done), so this ADR will functionally amend or supplement ADR-0034 — analogous to ADR-0023.
- 🔵 **suggestion / medium** — Dependencies — ADR-0030 (template) and ADR-0033 / ADR-0034 are implicit prerequisites worth naming. All prerequisites are done so no scheduling impact; cost is purely traceability.

### Scope

**Summary**: Work item 0062 is an ADR-authoring task that bundles five decision points: the interactive-hook contract, confidence-band design, ADR-0023 amendment scope, routing of cheap-fix parser recommendations, and resolution of vocabulary gaps. The five sub-decisions all sit under a single coherent purpose, and the routing choices are defended in Drafting Notes. The main scope concern is that two decisions — the vocabulary-gap resolutions and the broad ADR-0023 contract amendment — are arguably orthogonal to the interactive-validation strategy and are candidates for sibling ADRs; otherwise sizing and partitioning against 0069/0070 look sound.

**Strengths**:
- Summary, Requirements, and AC are mutually consistent — same five decision points, no scope drift.
- Partitioning against sibling work items 0069 and 0070 is explicit.
- Routing decisions are surfaced in Drafting Notes with a 'reviewer should reject if…' invitation.
- The `task` kind is appropriate — single deliverable, bounded set of decisions, no implementation scope.
- Dependencies, Assumptions, and Open Questions cleanly externalise concerns that belong elsewhere.

**Findings**:
- 🔵 **minor / medium** — Requirements — Vocabulary-gap resolution may be orthogonal to interactive-validation ADR. Spike 0068's Open Questions flag the `Source:` question as warranting 'a small ADR within epic 0057' — i.e. a sibling ADR, not a sub-decision of the migration-strategy ADR. Vocabulary resolutions don't depend on the interactive-hook contract.
- 🔵 **minor / medium** — Requirements — Broad ADR-0023 amendment scope arguably warrants its own ADR. Adding a permanent optional interactive contract is a framework-contract change of broader applicability than the corpus-migration use case driving this work.
- 🔵 **suggestion / low** — Acceptance Criteria — Cheap-fix routing decision is appropriately scoped here (positive observation supporting existing scope; no change needed). Flagged to disambiguate from the two routing decisions above which are more debatable.

### Testability

**Summary**: The work item is an ADR authoring task whose acceptance criteria are largely concrete and verifiable by reading the eventual ADR — each AC names a specific decision that must be present. The main testability weaknesses are unbounded language in the hook-contract AC ('fully specifies'), under-specification of what counts as a satisfactory 'resumability mechanism', and a missing explicit verification that the ADR actually cites the spike's binding numbers rather than just adopting the verdict.

**Strengths**:
- Each acceptance criterion is bound to a discrete, named decision the ADR must record.
- AC 1 cites specific numeric thresholds (11.3% vs 5%, ~5.3% counterfactual) a reviewer can grep for.
- AC 4 enumerates the closed set of acceptable ownership locations.
- AC 5 enumerates the closed set of acceptable resolutions for the 'broader workstream' gap.
- Open Questions explicitly notes 'None — all decision points the ADR must resolve are captured in Requirements / Acceptance Criteria', closing a common testability gap.

**Findings**:
- 🟡 **major / high** — Acceptance Criteria (AC 2) — 'Fully specifies the hook contract' is unbounded without per-element pass criteria. Reviewer cannot conclusively decide whether a one-line resumability description is 'full specification'.
- 🟡 **major / high** — Acceptance Criteria (AC 4) — 'Identifies the specific ADR-0023 clause(s)' lacks a verifiable minimum. Granularity unspecified — citing ADR-0023 by number, naming a section heading, or quoting exact language are all defensible.
- 🔵 **minor / medium** — Acceptance Criteria (AC 3) — 'Reasoning grounded in spike 0068's calibration data' is judgement-laden. A single passing reference to the 88%/90% figures technically satisfies it.
- 🔵 **minor / medium** — Acceptance Criteria (AC 5) — 'Canonical interpretation of Source: prose' has no closed answer set. Unlike the 'broader workstream' half, this half does not enumerate acceptable resolutions.
- 🔵 **minor / medium** — Acceptance Criteria (AC 6) / Requirements — No AC verifies that vocabulary-gap resolutions are reflected back into the typed-linkage vocabulary. 'Resolved in-place' is ambiguous if a one-line decision with no canonical vocabulary statement could pass.
- 🔵 **minor / low** — Acceptance Criteria (overall) — No AC asserts the ADR conforms to the ADR-0030 template. Format conformance is a natural pass/fail check that is currently absent.

## Re-Review (Pass 2) — 2026-05-26T09:25:00Z

**Verdict:** REVISE

Re-ran clarity, dependency, scope, and testability (the four lenses that had pass-1 findings). Both pass-1 majors are resolved, and most pass-1 minors and suggestions are resolved or substantively addressed. However, three new major findings surfaced — one pre-existing factual issue (a cheap-fix counterfactual figure that disagrees with the spike's executive summary) and two dependency-classification escalations (0060 and 0061 / ADR-0034 belong in `Blocked by`, not `Related`, given the new "builds on and supplements" framing). Verdict remains REVISE at the major-count threshold.

### Previously Identified Issues
- 🟡 **Testability** (AC 2): 'Fully specifies the hook contract' is unbounded — **Resolved**. Decomposed into four sub-bullets with per-element pass criteria; reviewers flagged minor refinement opportunities (named-predicate escape hatch, structural-vs-semantic check on user controls) but the core gap is closed.
- 🟡 **Testability** (AC 4): 'Identifies the specific ADR-0023 clause(s)' lacks a verifiable minimum — **Resolved**. AC now requires quoting or section-naming a specific clause and stating the replacement/addition.
- 🔵 **Clarity**: Hybrid-shape framing creates soft tension with committed 'interactive hooks' direction — **Partially resolved**. Technical Notes now commits interactive hooks and reframes hybrid vs uniform as the remaining question, but a new finding flags that the hybrid-vs-uniform call is not enumerated in Requirements/AC.
- 🔵 **Clarity**: `Source:` prose and 'broader workstream' shorthand assume reader has read spike 0068 — **Resolved**. Concrete examples added for both gaps.
- 🔵 **Dependency**: '0030 (ADR template)' mislabel — **Resolved**. Re-labelled as `ADR-0030` in both Dependencies and References.
- 🔵 **Dependency**: Vocabulary-amendment coupling to ADR-0034 / 0061 not captured — **Partially resolved**. Annotated as upstream/supplemented in Related, but the re-review reclassifies this as a `Blocked by` candidate (escalated to major in pass 2).
- 🔵 **Scope**: Vocabulary-gap resolution may be orthogonal to interactive-validation ADR — **Partially resolved**. Summary broadened to surface vocab scope; finding still raised in pass 2 but with reduced impact framing (discoverability hedge rather than re-routing).
- 🔵 **Scope**: Broad ADR-0023 amendment arguably warrants its own ADR — **Partially resolved**. AC 4 now requires a co-location-rationale paragraph; finding still raised in pass 2 with reduced severity and a suggestion that the paragraph engage the sibling-ADR alternative.
- 🔵 **Testability** (AC 3): 'Reasoning grounded in calibration data' is judgement-laden — **Resolved**. AC now requires explicit citation of the 88%/90% figures and a stated implication for band design.
- 🔵 **Testability** (AC 5): 'Canonical interpretation of `Source:` prose' has no closed answer set — **Resolved**. Closed set {`target`, `source`, `derived_from`, documented exception} now enumerated.
- 🔵 **Testability** (AC 6 / Requirements): Vocab-gap resolutions not reflected back into typed-linkage vocabulary — **Resolved**. AC now requires recording term name, definition, and applicable artifact-type pairs when a vocabulary term is introduced/reused.
- 🔵 **Testability** (overall): No AC asserts ADR-0030 template conformance — **Resolved** (new AC added; pass-2 finding refines further by asking for inline section enumeration).
- 🔵 **Suggestions** (4 from pass 1): 'Routing' phrasing replaced with 'ownership' — **Resolved**. 'Permanent optional contract' defined in Context — **Resolved**. ADR-0030 / 0033 / 0034 prerequisites annotated — **Resolved** (escalated to `Blocked by` recommendation in pass 2). Positive observation on cheap-fix routing — N/A.

### New Issues Introduced

#### Major
- 🟡 **Clarity** — Summary / Context: Cheap-fix counterfactual figure (~5.3%) contradicts spike research's executive-summary figure (~6.7%). Pre-existing in the original work item; surfaced in this pass. The spike has both numbers (line 35 says ~6.7%, line 111 says 5.3%) corresponding to different counterfactual scopes — the work item is consistent with the detailed-section figure but not the executive-summary figure. Worth reconciling the work item (and arguably the spike) so the ADR cites one authoritative number.
- 🟡 **Dependency** — Upstream vocabulary ADR (0061 / ADR-0034) listed as Related, not Blocked by. The pass-1 annotation made the supplementation visible; the pass-2 reviewer argues that "builds on and supplements" implies a scheduling block, not just a relationship.
- 🟡 **Dependency** — Foundational base-schema ADR (0060) listed as Related, not Blocked by. Same reasoning — the new "foundational decision this ADR builds on" annotation reads as a prerequisite.

#### Minor
- 🔵 **Clarity** — Resumability sub-AC uses passive voice ("names the persistence artefact… and the re-entry semantics") leaving the writer/re-entry actor implicit (introduced by AC 2 decomposition).
- 🔵 **Clarity** — Requirements (bullet 5a) asserts "vocab-canonical type for plan→work-item is `target`" as a fact while the AC asks the ADR to choose from four options — the spike treats this as contested. Either drop the assertion or restate it as the spike's framing (introduced by the bullet 5 expansion).
- 🔵 **Dependency** — Amendment relationship to ADR-0023 not classified as a coupling; the eventual edit to ADR-0023's text is invisible to anyone tracking what supersedes/amends what.
- 🔵 **Dependency** — Downstream impact on epic 0057 (resolves OQ3, unblocks 0069 before 0070) not noted in Dependencies.
- 🔵 **Scope** — Title understates the ADR's vocabulary-policy and framework-amendment scope (suggestion-grade in pass 2).
- 🔵 **Testability** — ADR-0030 conformance criterion is not decomposed; only four frontmatter fields named, "template structure" portion is judgement-laden (refinement of the new AC).
- 🔵 **Testability** — User controls sub-AC's "one-sentence behavioural definition" is a structural check, not a semantic one (refinement of AC 2).
- 🔵 **Testability** — Co-location paragraph (AC 4) has no minimum content check; a perfunctory paragraph could pass.
- 🔵 **Testability** — Trigger sub-AC's "named predicate" escape hatch is unbounded.

#### Suggestions
- 🔵 **Clarity** — Hybrid-vs-uniform decision in Technical Notes is a 5th decision not surfaced in Summary / Context / Requirements / AC; reconcile the cross-section decision count.

### Assessment

The pass-1 major findings are both cleanly resolved and the bulk of minors/suggestions are addressed, so the revision was substantive. However, three new majors keep the verdict at REVISE:

1. The cheap-fix counterfactual figure should be reconciled — this is a factual issue inherited from the original work item that a future ADR author will cite verbatim. Simplest fix: pick one figure with a short note on which counterfactual scope it covers.
2. The dependency lens is making a stronger argument on second look: the new annotations on 0060 and 0061 / ADR-0034 ("foundational… builds on", "upstream… builds on and supplements") read more naturally as `Blocked by` couplings than `Related` ones. Both upstream items are done, so promotion is metadata-only — no scheduling impact, just dependency-graph accuracy.
3. Several pass-2 minors are refinement-level (introduced by my own AC decomposition) and could be addressed in a quick follow-up pass; none are blockers individually.

The work item is in significantly better shape than pass 1 and is close to ready — one more focused pass addressing the cheap-fix figure and the two dependency promotions would clear the major bar.

## Re-Review (Pass 3) — 2026-05-26T09:45:00Z

**Verdict:** REVISE

Re-ran clarity, dependency, scope, and testability. **All three pass-2 majors are resolved**: the cheap-fix counterfactual figure now carries its scope clarification (three patterns including `sibling-as-deriv`), and 0060 and 0061/ADR-0034 are now `Blocked by` with status notes. Two new majors keep the verdict at REVISE: a clarity finding that the expanded Summary is now a dense multi-clause sentence (introduced by pass-1's Summary broadening), and a re-escalated scope finding that vocabulary-gap resolutions still belong in a sibling ADR. The scope finding repeats the routing concern the author has previously declined to split.

### Previously Identified Issues (Pass 2 → Pass 3)
- 🟡 **Clarity** (Summary/Context): Cheap-fix counterfactual figure contradicts spike's exec summary — **Resolved**. Scope-clarification phrase added ("resolving `template-path`, `prose-keyword-false-match`, and `sibling-as-deriv`") so the 5.3% figure is unambiguously the spike's detailed counterfactual.
- 🟡 **Dependency**: 0061 / ADR-0034 should be `Blocked by` — **Resolved**. Promoted with rationale and status note.
- 🟡 **Dependency**: 0060 should be `Blocked by` — **Resolved**. Promoted with rationale and status note.
- 🔵 **Dependency**: ADR-0023 amendment follow-on edit not captured — **Resolved**. Related entry now notes the implied edit to ADR-0023's text.
- 🔵 **Dependency**: 0057 OQ3 downstream impact not noted — **Resolved**. Blocks entry now records the OQ-3 resolution coupling.
- 🔵 **Clarity** (resumability passive voice) — **Not raised in pass 3**; unaddressed but no longer flagged at this severity.
- 🔵 **Clarity** (vocab-canonical type assertion) — **Not raised in pass 3**; unaddressed but no longer flagged.
- 🔵 **Scope** (title understates scope) — **Not raised in pass 3**; downgraded.
- 🔵 **Testability** (ADR-0030 conformance decomposition) — **Still present**; raised again as minor refinement.
- 🔵 **Testability** (user controls structural check) — **Not raised in pass 3**; unaddressed but no longer flagged.
- 🔵 **Testability** (co-location paragraph content check) — **Not raised in pass 3**; unaddressed but no longer flagged.
- 🔵 **Testability** (named-predicate escape hatch) — **Not raised in pass 3**; unaddressed but no longer flagged.
- 🔵 **Suggestion** (hybrid as 5th decision) — **Still present**; escalated and flagged independently by clarity and testability lenses, now arguing the decision must be reflected in Requirements / AC, not just Technical Notes.

### New Issues Introduced

#### Major
- 🟡 **Clarity** — Summary: Summary's second sentence is ~90 words and stacks five decisions inside parentheticals and em-dashes. The Summary became denser when broadened in pass 1; a reviewer or downstream story author scanning the Summary alone risks missing one of the five decisions. Suggestion: split into (i) what the ADR produces, (ii) spike verdict + numbers, (iii) bulleted/short-comma list of the five decisions.
- 🟡 **Scope** — Requirements bullet 5 / AC 6: Vocabulary-gap resolutions bundle a typed-linkage policy concern into a migration-strategy ADR. Re-escalation of a previously-acknowledged routing decision. The reviewer's recommendation is to either split the vocab decisions into a sibling ADR (which the author has already declined) or strengthen AC 4's co-location paragraph to specifically cover the vocab-gap inclusion, not just the framework-contract amendment.

#### Minor
- 🔵 **Clarity** — Requirements/AC: 'source' is used in three distinct senses (vocab type `source`; the References-section "Source:" label; the prose "Source:" on plan artifacts being analysed). Typographic distinction would help.
- 🔵 **Clarity** — Technical Notes: Hybrid-vs-uniform contract option is introduced only in Technical Notes; no corresponding entry in Requirements or AC. Risk that a drafter satisfies AC 2 by naming a band without resolving the hybrid-vs-uniform question.
- 🔵 **Clarity** — Drafting Notes: "Reviewer should reject if the title should stay neutral" lacks an explicit actor/mechanism (reject what — the ticket? the title change? the ADR PR?).
- 🔵 **Clarity** — Dependencies: "this vocabulary"/"needs it settled" in the 0061 Blocked-by entry has an ambiguous antecedent (ADR-0034 the document vs. the vocabulary it defines).
- 🔵 **Dependency** — Cheap-fix parser routing decision is itself a downstream coupling: 0069 and 0070 cannot finalise their scope until this ADR's AC-5 decision lands. Suggestion: in Blocks, add a half-sentence noting that one of 0069/0070 will materially grow in scope as a consequence of this ADR's routing decision.
- 🔵 **Scope** — Broad ADR-0023 amendment carries framework-wide policy inside a migration-specific ADR (already-acknowledged routing; co-location AC partially mitigates).
- 🔵 **Testability** — ADR-0030 conformance criterion still lacks an explicit checklist of structural sections; verifier must open ADR-0030 to know what "conforms" means.
- 🔵 **Testability** — AC 6 "broader workstream" resolution: the "documented limitation" branch has no defined evidence requirement (the "new vocab type" and "reuse" branches do via the term-name/definition/pairs requirement).
- 🔵 **Testability** — Hybrid-vs-uniform decision required by Technical Notes is not covered by any AC; a verifier checking the AC list alone could mark the ADR complete with this question unresolved.

#### Suggestions
- 🔵 **Dependency** — Vocabulary-gap resolutions imply a downstream coupling to ADR-0034: when this ADR introduces or reuses a vocab term, ADR-0034's vocabulary list will need to be updated or cross-referenced. Mirror the ADR-0023 follow-on-edit treatment for ADR-0034.
- 🔵 **Scope** — Six decision areas push the ADR toward the upper bound of single-ADR scope; consider organising the Decision section into clearly labelled sub-decisions for atomic supersession.

### Assessment

The three pass-2 majors are fully resolved and the work item's dependency capture is now strong (upstream blockers with status, downstream consumers with reason, related entries distinguishing amendment targets / template authority / parent epic). The two new majors are different in character:

- The **Summary density** finding is a real regression introduced by my pass-1 broadening. It's a focused, fixable edit (split into three sentences/bullets).
- The **vocabulary-gap bundling** finding is the same routing concern from pass 1 that the author already considered and decided to keep bundled. The pass-3 reviewer is essentially asking the author to reconsider that decision. If the routing stays, the realistic fix is to expand AC 4's co-location paragraph requirement to specifically cover the vocab-gap inclusion.

The **hybrid-vs-uniform** question is now flagged independently by clarity (cross-section count mismatch) and testability (uncovered AC) — folding it into AC 2 or adding a standalone AC would close two pass-3 findings in one edit.

Recommended next pass: (1) split the Summary, (2) extend AC 4's co-location paragraph to cover the vocab-gap bundling, (3) add hybrid-vs-uniform to Requirements/AC, (4) optionally tighten the source-term typography and the ADR-0030 conformance checklist.

## Re-Review (Pass 4) — 2026-05-26T10:30:00Z

**Verdict:** COMMENT

Full structural rewrite: pass-3's recommendations applied plus a structural split — broad ADR-0023 amendment and framework-level contract primitives extracted into newly-created sibling work item 0092 (`meta/work/0092-adr-optional-interactive-contract-for-migration-framework.md`). The restructured 0062 owns linkage-application decisions only: confidence-band design, hybrid-vs-uniform application shape, parameterisation of 0092's framework contract, parser-fix routing, and vocab-gap resolutions.

Re-ran all five lenses fresh. **All previous majors resolved.** No critical or major findings in pass 4. 14 minor findings and 1 suggestion across clarity, dependency, scope, and testability — most are polish-level refinements that don't block implementation.

### Previously Identified Issues (Pass 3 → Pass 4)
- 🟡 **Clarity** (Summary density): **Resolved**. Summary now split into three paragraphs with bulleted decision list.
- 🟡 **Scope** (vocab-gap bundling): **Partially resolved by restructure**. Framework-amendment bundling concern is gone (extracted to 0092). Vocab-gap bundling within 0062 still flagged once at minor severity (suggesting drafting-time monitoring); title-narrowing concern surfaces independently.
- 🔵 **Clarity** (source overload, hybrid-vs-uniform, "this vocabulary"): **All resolved or substantially addressed**. Source-typographic convention applied; hybrid-vs-uniform now AC 3; vocabulary antecedent clarified.
- 🔵 **Dependency** (cheap-fix scope-shift on 0069/0070): **Resolved** (Blocks entries annotated).
- 🔵 **Dependency** (ADR-0034 downstream coupling note): **Resolved** (Related entry mentions the follow-on; pass 4 escalates this as a tracking-as-action concern, not a resolution gap).
- 🔵 **Scope** (broad amendment carries framework policy in migration ADR): **Resolved** (extracted to 0092).
- 🔵 **Testability** (ADR-0030 decomposition): **Still flagged**; pass 4 wants inline section list, not just frontmatter fields.
- 🔵 **Testability** ('broader workstream' documented-limitation evidence): **Resolved**.

### New / Still-Flagged Findings

#### Minor (14)
- 🔵 **Clarity** — Dependencies / Drafting Notes refer to "AC 5" / "AC 6" but the AC list is unnumbered checkboxes; counting drifts on re-order. Suggestion: number ACs or replace with quoted phrases.
- 🔵 **Clarity** — AC 1 "5% threshold" referent depends on Context being read first; inline "the ≤5% wrong-rate threshold pre-committed by spike 0068".
- 🔵 **Clarity** — "VCS" used without expansion or link to ADR-0023.
- 🔵 **Clarity** — Requirements bullet (adopt-contract) packs four primitives in a single dense sentence; split or restructure to mirror AC 2 sub-bullets.
- 🔵 **Clarity** — Requirements bullet 5(a): "documented exception" branch undefined — does it mean a body subsection, an ADR-0034 entry, or a follow-up?
- 🔵 **Clarity** — Context "adopts that contract and parameterises it" — "it" is technically ambiguous; replace with "that contract".
- 🔵 **Dependency** — Epic 0057 OQ-3 resolution stated in prose but not surfaced as a Blocks entry (will linger as 'open' against 0057 after acceptance).
- 🔵 **Dependency** — ADR-0034 cross-reference follow-on acknowledged in Related but not captured as a tracked action (parallel to 0092's ADR-0023 case).
- 🔵 **Dependency** — Transitive ADR-0023 coupling (via 0092) not visible in 0062's Dependencies at all.
- 🔵 **Scope** — Title "Interactive Validation for Corpus Migration" understates the vocab-gap and parser-routing scope; either broaden title or add an "also resolves" clause to Summary.
- 🔵 **Testability** — AC 7 template conformance delegates to ADR-0030 without inline section list (same pattern as 0092).
- 🔵 **Testability** — AC 2 "What is shown" sub-bullet uses "e.g." rather than a required minimum set of display elements.
- 🔵 **Testability** — AC 6 co-location paragraph has no substance bar; could pass with a trivial sentence. Suggestion: require the paragraph to address (i) linkage-coupling rationale, (ii) sibling-ADR alternative, (iii) why rejected.
- 🔵 **Testability** — AC 2 "User controls" mutation targets not bounded to a defined field set; could pass with vague language.

#### Suggestion (1)
- 🔵 **Scope** — Vocab-gap bundling is the one bundling decision worth re-confirming during drafting. If the vocab-gap section grows or introduces multiple new terms, reconsider extracting a sibling vocabulary-amendment ADR derived from 0061 / ADR-0034.

### Cross-Cutting Themes (with 0092)

- **AC 7 template-conformance pattern** — both work items delegate to ADR-0030 without inlining the section list. Same fix would resolve both.
- **"e.g." in display-element enumerations** — flagged on both 0092 AC 4 and 0062 AC 2; same fix pattern (replace with required minimum set).
- **ADR-NNNN-amendment follow-on tracking** — 0092 needs a tracked follow-on for the ADR-0023 text edit; 0062 needs the same for ADR-0034. Both currently rely on prose acknowledgement.

### Assessment

The structural split was the right call: pass 3's hardest scope finding is gone, and the three-layer chain (0092 contract → 0062 application → 0069 implementation) is internally consistent with reciprocal dependency entries. Both work items are now ready for implementation — neither carries critical or major findings, and the residual minors are polish that can be addressed during ADR drafting without re-review.

The most consequential pass-4 polish items, if you want to address before drafting:
1. Number the AC list (closes the cross-reference drift finding).
2. Inline the ADR-0030 required-section list in AC 7 of both 0062 and 0092 (resolves the cross-cutting testability theme).
3. Decide whether the ADR-0023 / ADR-0034 follow-on text edits are in-scope for 0092/0062 or get their own tickets (resolves the cross-cutting dependency theme).

Everything else is fine as-is.

## Approval (Pass 5) — 2026-05-26T10:45:00Z

**Verdict:** APPROVE

Author closed out the three cross-cutting polish items: AC 7 now inlines the actual ADR-0030 required body sections and frontmatter fields (`adr_id`, `date`, `author`, `status`, `tags`); brittle "AC 5" / "AC 6" cross-references replaced with quoted phrases; the ADR-0034 "follow-on text edit" framing reframed per the corpus's accepted-ADR immutability convention (older ADRs are not mutated; the supplementation lives in the new ADR's text). Residual minor findings are polish-level and can be addressed during ADR drafting without re-review.

Work item approved and ready for implementation. Status transitioned `draft` → `ready`.
