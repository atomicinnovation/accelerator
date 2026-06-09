---
date: "2026-05-30T00:45:00Z"
type: work-item-review
producer: review-work-item
target: "work-item:0069"
work_item_id: "0069"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
id: "0069-migration-framework-interactive-validation-hooks-review-1"
title: "0069-migration-framework-interactive-validation-hooks-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-30T00:45:00Z"
last_updated_by: Toby Clemson
---

## Work Item Review: 0069 — Extend Migration Framework with Interactive Validation Hooks

**Verdict:** REVISE

The work item is well-structured, with all expected story sections populated and a clear conditional framing. However, multiple lenses surface the same structural gap: the contract for the interactive hook is underspecified — what "low-confidence" means, what "edit" does, how resumption is observed, and which work item (this one or 0092) actually owns the contract definition. A separate cross-cutting issue is an inconsistent description of 0062 (called the migration-strategy ADR in Summary/Context but the linkage-application ADR in Dependencies), which obscures the very decision that gates this story.

### Cross-Cutting Themes

- **0062 is described inconsistently** (flagged by: clarity, dependency) — Summary/Context call 0062 the "migration-strategy ADR"; Dependencies calls it the "linkage-application ADR". A reader cannot tell which decision actually gates the story.
- **The hook contract is underspecified** (flagged by: clarity, dependency, scope, testability) — Requirement 2 reads as contract authoring, but Dependencies says 0092 owns the contract; Acceptance Criteria leave low-confidence threshold, edit semantics, and invoker unspecified. Whoever owns the contract must own these decisions before this story can be planned.
- **Resumability is declared but not verifiable** (flagged by: testability, dependency) — AC3 promises resume-from-last-unprocessed, but the persistence mechanism, the meaning of "unprocessed" for edit/skip outcomes, and idempotency on full re-run are not pinned down. Open Questions explicitly flag this as unresolved.

### Findings

#### Major

- 🟡 **Clarity + Dependency**: 0062 described inconsistently across sections
  **Location**: Summary / Context / Dependencies
  Summary and Context name 0062 the "migration-strategy ADR"; Dependencies names it the "linkage-application ADR — decides whether and how this migration uses the contract". A reader cannot resolve which decision gates this story, undermining the conditional logic the work item depends on.

- 🟡 **Clarity**: Ambiguous referents "this migration" and "it's built"
  **Location**: Dependencies
  "This migration" has no clear referent in a story about extending the framework, and "corpus migration consumes this extension if it's built" is ambiguous about whether "it" is the extension or the corpus migration.

- 🟡 **Dependency**: 0092 framework-level contract not surfaced in Context or Requirements
  **Location**: Context / Requirements
  Dependencies names 0092 as the work item that defines the optional interactive contract this story implements, but neither Context nor Requirements mentions 0092. A reader of the body alone would assume this story defines the contract, when in fact it consumes one.

- 🟡 **Testability**: "Low-confidence transformations" lacks a measurable threshold
  **Location**: Acceptance Criteria (AC2)
  Neither AC2 nor Requirements define what confidence score or threshold qualifies a transformation as low-confidence. A tester cannot construct an input that deterministically triggers (or suppresses) the prompt.

- 🟡 **Testability**: Resume semantics underspecified for verification
  **Location**: Acceptance Criteria (AC3)
  AC3 promises resume-from-last-unprocessed but does not say how confirmation is persisted, what "unprocessed" means after an edit or skip, or what happens if the underlying source changed between runs. AC5's "partial-run-then-resume" scenario has no oracle.

- 🟡 **Testability**: "Edit" control has no defined behaviour to verify
  **Location**: Acceptance Criteria (AC2)
  AC2 lists "edit" as one of three controls but nothing specifies what the user edits, what input format they provide, or how the framework validates the edited value. AC5's mixed accept/edit/skip test cannot be executed without that contract.

#### Minor

- 🔵 **Clarity**: Undefined domain term "linkage-inference"
  **Location**: Open Questions
  The third open question references "linkage-inference" but this term is not defined in the work item; Context only mentions "body-section inference".

- 🔵 **Clarity**: "Unified-schema migration" used without definition
  **Location**: Context
  Context refers to "the unified-schema migration's body-section inference pass" as if previously named, but it has not been introduced here. The reader must infer from parent 0057.

- 🔵 **Clarity**: Passive "when invoked, the hook presents" obscures the invoker
  **Location**: Acceptance Criteria (AC2)
  It is not stated who or what invokes the hook — the framework, the migration, or the user via a CLI flag — which affects how "mechanical-only migration unaffected" is enforced.

- 🔵 **Clarity**: "Amends that contract" leaves scope of amendment vague
  **Location**: Technical Notes / Acceptance Criteria
  AC4 says docs "amend or reference" ADR-0023; Technical Notes says the story "explicitly amends" it. Unclear whether ADR-0023 itself must be edited, a new ADR written, or only a cross-reference added.

- 🔵 **Scope**: Overlap with 0092 risks bundling contract and implementation
  **Location**: Requirements / Dependencies
  Requirement 2 ("Define the hook's contract...") and Requirement 4 (documentation of the contract) read as contract-definition work, while Dependencies states 0092 owns the contract. Risk of duplicated effort or two-headed contract design.

- 🔵 **Completeness**: Story does not identify the user or system whose need is being met
  **Location**: Summary / Context
  For a story, the lens looks for an explicit for-whom. The work item describes what and why but does not name the consumer (e.g., migration authors of the unified-schema migration).

- 🔵 **Dependency**: VCS-based recoverability assumption implies tooling coupling not captured
  **Location**: Assumptions / Technical Notes
  "VCS revert remains the safety net" and "transactional VCS commits per accepted transformation" imply coupling to jj/git semantics and working-copy conventions that are not surfaced in Dependencies.

- 🔵 **Dependency**: ADR-0023 amendment not captured as a deliverable
  **Location**: Acceptance Criteria / Dependencies
  AC4 requires amending or referencing ADR-0023, but ADR-0023 sits under "Related" not "Blocks/amends". Whether amending the ADR is in-scope here or a separate gated item is unclear.

- 🔵 **Dependency**: Downstream consumers beyond 0070 not considered
  **Location**: Dependencies
  If the open question resolves toward "general-purpose hook", other migrations become downstream consumers — none are listed.

- 🔵 **Testability**: AC1's "identically to today" is unbounded
  **Location**: Acceptance Criteria (AC1)
  "Identically" admits output-equivalence, byte-for-byte log parity, exit-code equality, or any combination — different verifiers will disagree.

- 🔵 **Testability**: Documentation criterion is subjective
  **Location**: Acceptance Criteria (AC4)
  A single sentence referencing ADR-0023 would satisfy the literal wording; the criterion provides little verification value.

- 🔵 **Testability**: Test bullet does not specify the migration under test
  **Location**: Acceptance Criteria (AC5)
  AC5 enumerates scenarios but does not name a migration or fixture corpus — different implementers could exercise different migrations.

- 🔵 **Testability**: Idempotency-on-re-run requirement not promoted to an AC
  **Location**: Requirements / Acceptance Criteria
  Requirements list idempotency on re-run; AC3 only covers resume, which is weaker. Running an already-completed migration twice has no verified behaviour.

#### Suggestions

- 🔵 **Scope**: General-purpose vs single-purpose hook question gates scope
  **Location**: Open Questions
  A single-purpose hook is a much smaller increment than a general-purpose extension. The story's size could vary 2–3x until this is resolved.

### Strengths

- ✅ Conditional framing (gated on 0062 and 0068) is explicit in both Summary and Context, including the explicit close-without-implementation case.
- ✅ Frontmatter is complete and valid (kind=story, status=draft, priority=medium, parent and tags populated).
- ✅ Acceptance Criteria contain five distinct, specific bullets covering opt-in behaviour, hook UX, resumability, documentation, and test coverage.
- ✅ Requirements and Acceptance Criteria mirror each other, signalling a well-bounded deliverable.
- ✅ Dependencies section explicitly lists upstream blockers (0092, 0062, 0068), downstream consumer (0070), and related items (0057, 0023) with reasoning.
- ✅ Out-of-scope is implicitly clear: default mechanical path preserved; corpus migration delegated to 0070.

### Recommended Changes

1. **Reconcile the description of 0062 across Summary, Context, and Dependencies** (addresses: 0062 described inconsistently)
   Decide whether 0062 is the migration-strategy ADR, the linkage-application ADR, or both bundled. State the single canonical title and decision once, and use it consistently in all three sections.

2. **Clarify the contract ownership between 0069 and 0092** (addresses: 0092 contract not surfaced, Overlap with 0092)
   Add a sentence to Context naming 0092 as the contract owner and reframe Requirement 2 as "implement / consume the contract defined by 0092" if 0092 owns it. Otherwise, drop the 0092 blocker. Pick one.

3. **Pin down what "low-confidence" means for AC2** (addresses: low-confidence threshold)
   Either specify a threshold inline, or defer to a referenced artefact (e.g. 0068's spike output) and commit AC5 fixtures to inputs at, above, and below that threshold.

4. **Specify the edit-control contract in AC2 and the resume-state contract in AC3** (addresses: Edit control undefined, Resume semantics underspecified)
   For edit: what the user is shown, what they submit, how validation/failure is handled. For resume: where confirmation is persisted, what unprocessed means after edit/skip, what happens if the source changed. Resolve the resumability Open Question before promoting.

5. **Disambiguate referents in Dependencies** (addresses: "this migration" / "it's built")
   Replace "this migration" with the named referent (e.g. "the unified-schema body-section inference migration"); rephrase "if it's built" to name the subject ("if this extension is built").

6. **Tighten AC1, AC4, AC5 to remove unbounded language** (addresses: identically-to-today, documentation subjectivity, missing fixture in AC5)
   AC1: name the observable surface (e.g. "produces the same migrated artefacts and exit code on a fixed corpus"). AC4: enumerate required topics (declaration syntax, accept/edit/skip semantics, resume behaviour, worked example). AC5: name the fixture or commit to building a synthetic one in this story.

7. **Add an idempotency AC** (addresses: idempotency not promoted)
   "Re-running a fully-completed migration produces no prompts and no further changes to the artefacts."

8. **Define unfamiliar terms on first use** (addresses: linkage-inference undefined, unified-schema migration undefined)
   Either swap "linkage-inference" for the term already used in Context, or add a one-line gloss. Gloss "unified-schema migration" with a brief reference to where 0057 introduces it.

9. **Decide single-purpose vs general-purpose before promoting from draft to ready** (addresses: General-purpose vs single-purpose question)
   The answer changes the story's size materially; resolving it lets reviewers confirm right-sizing.

10. **Name the consumer in Summary or Context** (addresses: user/system unidentified)
    Add a phrase such as "so migration authors can request user confirmation for low-confidence transformations".

11. **Surface implicit couplings if relevant** (addresses: VCS coupling, ADR-0023 amendment, downstream consumers)
    If transactional VCS commits are the chosen resumability path, note the dependency on jj workflow. If amending ADR-0023 is in-scope here, capture it explicitly in Acceptance Criteria as a deliverable rather than leaving it under "Related". If general-purpose is chosen, add additional downstream consumers (or note 0070 is the only intended one).

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear and well-structured, but the Dependencies section contradicts Summary/Context about 0062, and several ambiguous referents and undefined terms reduce precision for readers unfamiliar with parent epic 0057.

**Strengths**:
- Summary clearly states the conditional nature and what triggers closure-without-implementation.
- Acceptance Criteria use active voice with named actors.
- Cross-references to ADR-0023 and related work items are explicit and consistent across most sections.

**Findings**:
- 🟡 (major, high) 0062 described inconsistently across sections — Dependencies — Summary/Context call 0062 "migration-strategy ADR"; Dependencies calls it "linkage-application ADR".
- 🟡 (major, high) Ambiguous referent: "this migration" and "it's built" — Dependencies — "this migration" has no clear referent in a framework-extension story.
- 🔵 (minor, medium) Undefined domain term "linkage-inference" — Open Questions.
- 🔵 (minor, medium) "Unified-schema migration" used without definition — Context.
- 🔵 (minor, medium) Passive "when invoked, the hook presents" obscures the invoker — Acceptance Criteria.
- 🔵 (minor, low) "Amends that contract" leaves scope of amendment vague — Technical Notes / AC4.

### Completeness

**Summary**: All expected story sections are populated; frontmatter complete; conditional nature captured explicitly. Minor gap on identifying the for-whom.

**Strengths**:
- Frontmatter complete and valid.
- AC contains five distinct specific bullets.
- Context explains motivation rather than restating summary.
- Open Questions, Dependencies, Assumptions, Technical Notes all populated with substantive content.
- Conditional framing explicit in Summary and Context.

**Findings**:
- 🔵 (minor, medium) Story does not identify the user or system whose need is being met — Summary/Context.

### Dependency

**Summary**: Reasonably thorough Dependencies section, but Summary/Context wording about 0062 disagrees with Dependencies, the 0092 contract dependency is invisible outside the Dependencies block, and some implicit couplings (VCS, ADR-0023 amendment, downstream consumers) are not surfaced.

**Strengths**:
- Dependencies explicitly lists upstream blockers with reasoning.
- Downstream consumer (0070) captured.
- Parent epic and amended ADR named.
- Conditional dependencies called out across multiple sections.

**Findings**:
- 🟡 (major, high) 0092 framework-level contract not surfaced in Context or Requirements.
- 🟡 (major, high) 0062 described inconsistently across sections.
- 🔵 (minor, medium) VCS-based recoverability assumption implies tooling coupling not captured — Assumptions / Technical Notes.
- 🔵 (minor, medium) Documentation amendment to ADR-0023 not captured as a dependency on the ADR process — AC / Dependencies.
- 🔵 (minor, medium) Downstream consumers beyond 0070 not considered — Dependencies.

### Scope

**Summary**: Story is coherent and tightly scoped to one capability — adding an optional interactive-validation hook to the existing migration framework. Sizing is appropriate for a story; the main scope concern is potential overlap with 0092's contract work.

**Strengths**:
- Summary, Requirements, and AC describe one unit of work.
- Conditional framing makes the scope boundary explicit.
- AC mirror Requirements one-for-one.
- Out-of-scope implicitly clear (mechanical path preserved; corpus migration delegated to 0070).

**Findings**:
- 🔵 (minor, medium) Overlap with 0092 risks bundling contract and implementation — Dependencies / Requirements.
- 🔵 (suggestion, medium) General-purpose vs single-purpose hook question materially gates scope — Open Questions.

### Testability

**Summary**: AC are mostly framed as observable behaviours with clear pass/fail conditions and a defined scenario matrix, but several criteria contain ambiguous thresholds, unbounded language, and missing oracles for the resume and edit paths.

**Strengths**:
- AC5 enumerates concrete test scenarios.
- AC2 specifies user-facing controls and presentation mode.
- Conditional framing in Summary is itself testable.
- AC1 establishes a clear backwards-compatibility check.

**Findings**:
- 🟡 (major, high) "Low-confidence transformations" lacks a measurable threshold — AC2.
- 🟡 (major, high) Resume semantics underspecified for verification — AC3.
- 🟡 (major, medium) "Edit" control has no defined behaviour to verify — AC2.
- 🔵 (minor, high) AC1's "identically to today" is unbounded — AC1.
- 🔵 (minor, high) Documentation criterion is subjective — AC4.
- 🔵 (minor, medium) Test bullet does not specify the migration under test — AC5.
- 🔵 (minor, medium) Idempotency-on-re-run requirement is not promoted to an AC — Requirements / AC.

## Re-Review (Pass 2) — 2026-05-30

**Verdict:** REVISE

The pass-1 rewrite resolved the structural concerns: 0062's identity is now consistent, 0092's contract role is surfaced everywhere, conditional framing is gone, referents are named, and acceptance criteria are decomposed into observable behaviours. Completeness, scope, and dependency lenses now return clean (zero findings each). The remaining findings cluster around a single new pattern: many ACs delegate the concrete pass/fail bar to ADR-0092 (display elements, malformed-edit cases, source-drift behaviour, resume-state record format) rather than restating it. The criteria are not self-contained — a tester reading only this story cannot derive a definitive pass condition for those points. A separate cosmetic slip is inconsistent use of "ADR-task 0092" vs "ADR-0092" across sections.

### Previously Identified Issues

- 🟡 Clarity + Dependency: **0062 described inconsistently** — Resolved. Now consistently described as the linkage-application ADR; "migration-strategy" labelling removed.
- 🟡 Clarity: **Ambiguous referents "this migration" / "it's built"** — Resolved. Named referents throughout (unified-schema migration, 0070, this extension).
- 🟡 Dependency: **0092 framework-level contract not surfaced** — Resolved. 0092 is named prominently in Summary, Context, every Requirement, every AC, Technical Notes, Dependencies, and References.
- 🟡 Testability: **"Low-confidence transformations" lacks a measurable threshold** — Resolved. Trigger is now a boolean predicate; threshold lives in the consumer migration per 0092 (correct contract layering).
- 🟡 Testability: **Resume semantics underspecified** — Partially resolved. Three separate ACs (partial-run, idempotency, source-drift) now exist, but source-drift defers the concrete behaviour to 0092 without restating it.
- 🟡 Testability: **"Edit" control has no defined behaviour** — Partially resolved. Edit-input AC added with re-prompt-on-malformed behaviour and required test coverage, but the malformed-case set is delegated to 0092 without enumeration.
- 🔵 Scope: **Overlap with 0092** — Resolved. Requirement 2 (was "define the contract") is now an implementation requirement; 0092's ownership is explicit.
- 🔵 Clarity: **Undefined "linkage-inference"** — Resolved (term removed with Open Questions rewrite).
- 🔵 Clarity: **"Unified-schema migration" undefined** — Resolved. Named "the unified-schema migration (0070)" in Summary; 0070 is the canonical reference.
- 🔵 Clarity: **Passive "when invoked, the hook presents"** — Resolved. Runner is named as the invoker throughout.
- 🔵 Clarity: **"Amends that contract" vague** — Resolved. Story explicitly does not edit ADR-0023; amendment lives in 0092's text per immutability convention.
- 🔵 Completeness: **Story lacks for-whom** — Resolved. Migration authors and the first consumer (0070) are named in Summary.
- 🔵 Dependency: **VCS coupling implicit** — Resolved (kept in Assumptions; 0092 owns resumability-vs-VCS interaction).
- 🔵 Dependency: **ADR-0023 amendment scoping unclear** — Resolved. Now explicit that ADR-0023 is not edited.
- 🔵 Dependency: **Downstream consumers beyond 0070** — Resolved. Hook is general-purpose per 0092's broad amendment scope; 0070 is the first consumer.
- 🔵 Testability: **AC1 "identically to today" unbounded** — Resolved. Snapshot-comparison-on-fixture wording.
- 🔵 Testability: **Documentation criterion subjective** — Resolved (mostly). Five topics enumerated (a–e) and link requirement added; pass-2 lens still wants per-topic pass-bars.
- 🔵 Testability: **AC5 missing fixture** — Partially resolved. Fixture migrations referenced throughout but the fixture corpus itself is not named by path.
- 🔵 Testability: **Idempotency not promoted to AC** — Resolved. Dedicated resumability-idempotency AC.
- 🔵 Scope: **Single-purpose vs general-purpose** — Resolved. General-purpose per 0092's broad scope, stated in Technical Notes.

### New Issues Introduced

- 🟡 **Clarity**: Inconsistent use of "ADR-task 0092" vs "ADR-0092" across sections (Summary/Context use "ADR-task"; Requirements/ACs/Refs use "ADR-"). Slip from the rewrite. Pick one form.
- 🟡 **Clarity**: Critical behaviours defined only by external reference — display elements, malformed-edit cases, source-drift outcome are all stated as "what 0092 specifies" without inlining. A reader of this story alone cannot determine pass conditions.
- 🟡 **Testability**: Source-drift AC defers the concrete outcome (re-prompt or abort) to 0092 without naming the choice.
- 🟡 **Testability**: Edit-input malformed-cases set is not enumerated; "at least one malformed edit" is an under-specified coverage bar.
- 🟡 **Testability**: Display-elements AC requires asserting "migration-declared extras" but no fixture declares any extras.
- 🔵 **Clarity**: "Session log" and "resume state" used as if interchangeable — unclear whether one artefact or two.
- 🔵 **Clarity**: "Low-confidence" framing in Summary risks readers assuming the runner encodes a threshold (it does not — it evaluates an arbitrary predicate).
- 🔵 **Testability**: "Fixed fixture corpus" named but not located by path.
- 🔵 **Testability**: Partial-run resume AC presupposes a stable transformation ordering that the story does not define.
- 🔵 **Dependency** (suggestion): Whether fixtures already exist in `skills/config/migrate/fixtures/` or must be authored as part of this story is not stated.
- 🔵 Suggestion: "Cheap-fix counterfactual" jargon used in Context without gloss; minor "artefact" vs "artifact" spelling drift.

### Assessment

The rewrite landed the structural fixes cleanly — five of the seven majors are resolved, completeness/scope/dependency are clean, and many minors closed. The remaining major findings are a single, fixable pattern: ACs are too thin where they delegate to ADR-0092. Two options:

1. **Inline the 0092-derived specifics** into each AC (display element list, malformed-case list, source-drift outcome, resume-state record schema). Cost: some duplication of 0092's text; benefit: story is self-contained and reviewable on its own.
2. **Cite specific sections of 0092** (e.g., "per ADR-0092 §3.2") so the criterion has a precise anchor rather than a whole-document reference. Cost: needs 0092 to have stable section numbers; benefit: avoids duplication.

The work item is close to ready. A short third pass after addressing the delegation pattern and the ADR-task naming slip should clear it.

## Re-Review (Pass 3) — 2026-05-30

**Verdict:** COMMENT

The pass-2 rewrite resolved every major finding from pass 2. The "delegation to ADR-0092" pattern is gone — ADR-0037's primitives are now inlined into the ACs, the session-log/resume-state distinction is pinned to its source, ADR identifiers are correct (ADR-0037 and ADR-0038, not the producing task IDs), and the source-drift and transformation-ordering gaps ADR-0037 left silent are surfaced as explicit runner-level decisions with a documented promotion path. Completeness returns zero findings; scope returns two sizing/structure suggestions; dependency returns one minor ledger-schema observation; clarity and testability return only minor polish items.

### Previously Identified Issues (pass 2)

- 🟡 Clarity: **ADR-task vs ADR naming inconsistency** — Resolved. Canonical references now go to ADR-0037 and ADR-0038 with work items 0092/0062 explicitly labelled as their producing tasks.
- 🟡 Clarity: **Critical behaviours defined only by external reference** — Resolved. Display elements, accept/edit/skip record schemas, and source-drift behaviour are now inlined into the ACs.
- 🟡 Testability: **Source-drift defers outcome to ADR-0092** — Resolved. Source-drift is now an explicit runner-level decision (re-prompt with new value, discard old record).
- 🟡 Testability: **Malformed-cases not enumerated** — Resolved. Validation ownership moved to the migration per ADR-0037 §4; runner re-prompts with the migration's error message; AC covers one valid and one rejected edit.
- 🟡 Testability: **Migration-declared extras unverified** — Resolved. New AC requires a fixture migration to declare at least one extra display field, asserted by string-match.
- 🔵 Clarity: **Session log / resume state ambiguity** — Resolved. Defined per ADR-0037 §3 and used consistently.
- 🔵 Clarity: **"Low-confidence" risked implying runner threshold** — Resolved. Summary and Technical Notes state the runner is confidence-agnostic.
- 🔵 Testability: **Fixed-fixture-corpus unlocated** — Resolved. Fixture corpus located at `skills/config/migrate/scripts/` (existing migrations) plus author-as-part-of-this-story hook-declaring fixtures.
- 🔵 Testability: **Partial-run AC presupposes undefined ordering** — Resolved. Explicit ordering invariant plus deterministic `transformation_key` schema in session-log records.
- 🔵 Dependency: **Fixtures existence ambiguous** — Resolved. Assumptions section states existing migrations are the corpus; hook-declaring fixtures authored in this story.
- 🔵 Suggestions (cheap-fix counterfactual; artefact/artifact drift) — Resolved.

### New Issues Introduced

**Minor (9):**
- 🔵 **Clarity**: Reference to "ADR-0037 §2.2" doesn't match ADR-0037's numbering (§2 has items 1-2-3, not §2.1/§2.2). Use "ADR-0037 §2 item 2" or "ADR-0037 §2 — source location".
- 🔵 **Clarity**: "Accept-degraded" used in Requirements without inline gloss; the AC's parenthetical definition is the closest thing. Move the gloss to first use.
- 🔵 **Clarity**: Transformation-ordering Requirement bundles the ordering invariant and the key schema into one bullet; resume correctness depends on key matching, not on emission order. Split into two bullets.
- 🔵 **Clarity**: "Confidence-agnostic" qualifier in Summary lands before confidence has been introduced. Either gloss in-place or move to Technical Notes.
- 🔵 **Dependency**: ADR-0023's secondary role as ledger-schema authority (for `.accelerator/state/migrations-{applied,skipped}`) not surfaced — currently only listed as the supplemented contract.
- 🔵 **Testability**: AC1's fixture corpus scope is unbounded across the existing scripts; tester can't tell whether one failing migration is in or out of scope.
- 🔵 **Testability**: Incremental-write AC's "interrupted between two decisions" lacks an operational definition (SIGTERM vs SIGKILL vs harness-aborted).
- 🔵 **Testability**: Session-log schema uses "includes" rather than "consists of" — allows extra/missing fields to pass.
- 🔵 **Testability**: Display-elements AC must match "structural anchor" but Requirements admit "line number, section heading, or other unambiguous locator" — fixture's anchor form needs constraining.

**Suggestions (5):**
- 🔵 Dependency: Future opt-in consumers acknowledged but not enumerated (low-confidence; correct if no other consumer is currently planned).
- 🔵 Scope: Story is at the upper bound of single-increment sizing (7 requirements / 13 ACs). Either accept and note explicitly, or split documentation as a sibling.
- 🔵 Scope: Source-drift and ordering being runner-level decisions slightly muddies the three-layer chain. Could be promoted to a small supplementary ADR before implementation.
- 🔵 Testability: Full-run idempotency AC's "no further artefact changes" could specify a content-diff mechanism (e.g. `git diff --exit-code`).
- 🔵 Testability: Documentation AC's "runnable fixture-migration worked example" lacks a runnable-ness check (CI exercise?). Either tighten or downgrade to "illustrative".

### Assessment

The work item is acceptable as-is for implementation. The remaining findings are polish items — none change the unit of work, the contract being implemented, or the verification strategy. The author can choose to apply them in a final pass or proceed to planning; either is defensible.

The most impactful single change of the polish set, if any is applied, is the **§2.2 → §2 item 2** correction in the Display-elements AC: it's an objectively wrong reference, easy to fix, and preserves the precision the rest of the work item depends on.
