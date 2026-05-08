---
date: "2026-05-07T22:30:00+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0052-make-documents-locator-paths-config-driven.md"
work_item_id: "0052"
review_number: 1
verdict: REVISE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
---

## Work Item Review: Make documents-locator Agent Paths Config-Driven via Preloaded Skill

**Verdict:** REVISE

The work item is substantively strong in many respects: requirements are concrete and artefact-specific, the Summary follows the user-story format precisely, and the Acceptance Criteria use Given/When/Then consistently. However, the work item contains a critical internal contradiction that makes it impossible to implement as written — the Summary and Drafting Notes claim the approach "requires no harness change", while the Technical Notes explicitly state that the `skills:` frontmatter mechanism does not yet exist in the harness and that implementing it requires a harness change. This contradiction is reinforced by four other lenses from different angles (completeness, dependency, scope, testability), making it the dominant finding. Two additional major testability gaps (a tautological survey criterion and a criterion testing an implementation detail) further reduce confidence in the verification framing. The work is close to ready but requires targeted revision before implementation can proceed safely.

### Cross-Cutting Themes

- **Harness-change contradiction** (flagged by: clarity, completeness, dependency, scope, testability) — The core mechanism (`skills:` frontmatter preloading) is asserted to exist without a harness change in the Summary and Drafting Notes, but is simultaneously flagged as non-existent and requiring a harness change in Technical Notes. This is the single most impactful issue and must be resolved before any other work proceeds.
- **`meta/global/` gap undecided** (flagged by: completeness, dependency, scope, testability) — An explicit decision is deferred on whether the `meta/global/` path should become configurable or remain fixed, but this decision is not captured in Open Questions, Requirements, or Acceptance Criteria, leaving implementers to resolve it unilaterally.

### Findings

#### Critical

- 🔴 **Clarity / Completeness / Dependency / Scope / Testability**: Harness-change contradiction — 'no harness change' claim directly conflicts with Technical Notes gap
  **Location**: Summary, Drafting Notes, Technical Notes, Dependencies, Acceptance Criteria
  The Summary and Drafting Notes state the `skills:` preload approach "requires no harness change". The Technical Notes section simultaneously states: "Zero files in the repo use a `skills:` frontmatter key. The agent preloading mechanism described in the Summary does not yet exist in the harness. Implementing `skills:` in agent frontmatter requires a harness change." An implementer cannot determine from this work item alone whether a harness change is required. The Dependencies section lists "Blocked by: none" despite this unresolved question, and no Acceptance Criterion verifies that end-to-end preloading actually functions. If the mechanism does not exist and no criterion tests for it, an implementation could ship a `skills:` key that is silently ignored.

#### Major

- 🟡 **Completeness / Dependency / Scope / Testability**: `meta/global/` gap not carried into Requirements, Acceptance Criteria, or Open Questions
  **Location**: Technical Notes
  Technical Notes flag that `meta/global/` is referenced in the agent but no `global` key exists in `config-read-path.sh`, and state "Decision required" — but neither the decision nor a placeholder for it appears anywhere actionable. The testability framing is the sharpest: there is no Acceptance Criterion a tester can use to verify the correct treatment of `meta/global/`, meaning it could be silently ignored.

- 🟡 **Testability**: Survey-completion criterion is tautological
  **Location**: Acceptance Criteria
  The fourth criterion reads: "Given all `agents/*.md` files have been surveyed, then no other agent file contains hardcoded directory paths (survey complete: confirmed)". The parenthetical self-certifies completion — a tester cannot independently verify it. No inspectable artefact is required.

- 🟡 **Testability**: Frontmatter constraint criterion tests an implementation detail, not a verifiable outcome
  **Location**: Acceptance Criteria
  The fifth criterion ("The path-resolution skill does not carry `disable-model-invocation: true`") checks for the absence of a specific flag rather than any observable behaviour. The actual concern — that preloading will fail if the flag is set — is not expressed as a testable outcome.

- 🟡 **Clarity**: Term 'bang command' / 'bang-preprocessed' used without definition
  **Location**: Context, Requirements
  "Bang command", "bang-command-processed", and "bang-preprocessed" appear in load-bearing positions (the `!` character is explained parenthetically but the semantics — what preprocessing does and why it governs the `disable-model-invocation` constraint — is assumed knowledge). A reader unfamiliar with the harness cannot determine what Requirement 2's "preload constraint" means.

- 🟡 **Clarity**: Unexplained causal link between `disable-model-invocation: true` and preload incompatibility
  **Location**: Requirements, Technical Notes
  Requirement 2 states "The skill must not set `disable-model-invocation: true` (preload constraint)" without explaining why the flag is incompatible with preloading. An implementer who does not know the harness internals cannot verify compliance or understand the failure mode.

#### Minor

- 🔵 **Clarity**: Config file naming inconsistency across sections
  **Location**: Context, References
  The work item uses `.accelerator/config.md` / `.accelerator/config.local.md` throughout, while the referenced tech-debt note uses `config.user.yaml` / `config.team.yaml`. No note explains the relationship between the two naming conventions.

- 🔵 **Clarity**: Output format of `config-read-all-paths.sh` is underspecified
  **Location**: Technical Notes
  Requirement 1 and Technical Notes specify the script must emit "a structured block" with only a parenthetical example, leaving the actual required format undefined. Two implementers could produce incompatible outputs that both satisfy the stated requirement.

- 🔵 **Completeness**: No Acceptance Criterion covers the path-resolution skill's reusability constraint
  **Location**: Acceptance Criteria
  Requirement 2 explicitly states the skill can be preloaded by any agent, not only documents-locator. No criterion verifies this reusability property — a tightly-coupled implementation would pass all acceptance gates.

- 🔵 **Dependency**: 0030 post-delivery rework in `config-read-all-paths.sh` not captured
  **Location**: Dependencies
  Technical Notes note that once 0030 lands, `config-read-all-paths.sh` should be updated to source `config-defaults.sh`. This creates an untracked coupling: the team planning 0030 will not know 0052's script needs a follow-up edit.

- 🔵 **Testability**: First criterion's precondition scope is incomplete
  **Location**: Acceptance Criteria
  The first criterion specifies `paths.work: work-items` without clarifying that all other path keys should remain at defaults. A tester who inadvertently configures additional keys could get misleading results.

### Strengths

- ✅ Requirements are concrete and artefact-specific: each names a file path, script, or frontmatter key — no ambiguity about the implementation surface.
- ✅ The Summary is a well-formed user-story statement with a precise operator persona, want, and benefit.
- ✅ The Acceptance Criteria use Given/When/Then consistently with observable system states (e.g., "searches `work-items/` rather than `meta/work/`").
- ✅ The Assumptions section explicitly binds the "auto-discovery" constraint — the binding rule driving the `config-read-all-paths.sh` design is stated clearly.
- ✅ The Open Questions section is correctly scoped and does not embed unresolved design choices as hidden assumptions.
- ✅ The Context section provides genuine motivation, links to the originating tech-debt note, and explains why the existing approach is limited without restating the Summary.
- ✅ The Technical Notes section is unusually thorough: it includes a pre-flight codebase survey, the full path-key vocabulary (13 keys), and two explicit gap callouts.
- ✅ The dependency relationship with 0030 is correctly characterised — independent but should align on vocabulary — with no scope entanglement.
- ✅ The survey scope is bounded by Requirement 7, preventing open-ended discovery risk.

### Recommended Changes

1. **Resolve the harness-change contradiction** (addresses: critical finding)
   Verify whether the `skills:` frontmatter preloading mechanism exists in the harness today. If it does not exist: remove the "no harness change" language from the Summary and Drafting Notes, add the harness change as a Requirement, add it to Dependencies as a blocker (or split into two stories), and add an end-to-end Acceptance Criterion that verifies the preloaded path block actually appears in the agent's context. If it does exist: remove the Technical Notes gap note and update the Drafting Notes accordingly.

2. **Resolve the `meta/global/` gap and make it actionable** (addresses: major cross-cutting finding)
   Move the `meta/global/` decision into the Open Questions section. Once resolved, add the corresponding Requirement and Acceptance Criterion (either "given `paths.global` is configured, the agent searches the configured value" or "given no `paths.global` key exists, `meta/global/` is always included as a fixed path").

3. **Replace the tautological survey criterion** (addresses: major, testability)
   Rephrase: "Given all `agents/*.md` files have been surveyed, then the survey result — listing each file and whether hardcoded paths were found — is recorded in the Technical Notes or a linked document." This makes the criterion independently verifiable.

4. **Replace the frontmatter flag criterion with a behavioural outcome** (addresses: major, testability)
   Rephrase: "Given the path-resolution skill is listed in the agent's `skills:` frontmatter, when the agent is invoked, then the preloaded path block is present in the agent's context before it acts." If the frontmatter flag check is still considered necessary, move it to Technical Notes as a constraint rather than an Acceptance Criterion.

5. **Define 'bang command' / 'bang-preprocessed' when first introduced** (addresses: major, clarity)
   Add a one-sentence definition or a link to harness documentation in the Context section when these terms first appear, explaining what preprocessing does and why it must execute before agent invocation.

6. **Explain the `disable-model-invocation` / preload incompatibility** (addresses: major, clarity)
   Add a brief explanation in Requirement 2 or Technical Notes: e.g., "skills carrying this flag are excluded from the preload pipeline by the harness, so setting it would prevent the path block from being injected."

7. **Acknowledge the config file naming discrepancy** (addresses: minor, clarity)
   Add a note in the References section or Context that the referenced tech-debt note uses an older config file naming convention (`config.user.yaml` / `config.team.yaml`), so readers are not left wondering whether there are two different systems.

8. **Specify the output format of `config-read-all-paths.sh` normatively** (addresses: minor, clarity)
   Either specify the format in Technical Notes (e.g., "must emit a Markdown list with one `key: value` entry per line, wrapped in a labelled fenced block") or explicitly defer it and note that Requirement 4's agent body update must match whatever format is chosen.

9. **Add a reusability Acceptance Criterion for the path-resolution skill** (addresses: minor, completeness)
   e.g., "The path-resolution skill carries no documents-locator-specific content and can be listed under `skills:` in any other agent definition without modification."

10. **Note the 0030 post-delivery rework in Dependencies** (addresses: minor, dependency)
    Add a note that landing 0030 after 0052 will require a follow-up update to `config-read-all-paths.sh` to source `config-defaults.sh`, making this coupling visible from both sides.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is largely well-written with precise technical language and concrete, named actors throughout. However, a significant internal contradiction exists between the Summary's claim that the `skills:` preload approach 'requires no harness change' and the Technical Notes section's identification of a gap confirming that 'implementing `skills:` in agent frontmatter requires a harness change.' Additionally, a pair of undefined terms ('bang-command', 'bang-preprocessed') are used in load-bearing technical requirements without definition or link, and the config file naming conventions shift inconsistently across sections without reconciliation.

**Strengths**:
- Requirements are written with named actors and concrete artifacts throughout — each requirement identifies a specific file path, script name, or frontmatter key
- The user story format in the Summary provides unambiguous actor identification
- The Acceptance Criteria use the Given/When/Then structure consistently and state observable system states
- The Assumptions section explicitly surfaces and binds the 'auto-discovery' definition
- The open question in Open Questions is clearly scoped

**Findings**:

- **CRITICAL** (high confidence) — Summary / Technical Notes: Direct contradiction between 'no harness change' claim and identified harness change requirement. The Summary states the approach 'requires no harness change', but the Technical Notes section explicitly states 'implementing `skills:` in agent frontmatter requires a harness change.' The Drafting Notes section repeats the contradiction.

- **MAJOR** (high confidence) — Context / Requirements: Term 'bang command' / 'bang-preprocessed' used without definition. These terms appear in load-bearing positions without being defined or linked to a definition.

- **MAJOR** (high confidence) — Context / Technical Notes: Unexplained causal link between `disable-model-invocation: true` and preload incompatibility. Requirement 2 states the constraint without explaining why the flag is incompatible with preloading.

- **MINOR** (medium confidence) — Context / Acceptance Criteria: Config file naming inconsistency across sections. Work item uses `.accelerator/config.md` while the referenced note uses `config.user.yaml`.

- **MINOR** (medium confidence) — Technical Notes: Undefined term 'structured block' used to specify script output format. The parenthetical example is not a normative specification.

### Completeness

**Summary**: Work item 0052 is substantively complete for a story with a well-formed user-story Summary, rich Context, detailed Requirements, and a populated Acceptance Criteria section. The main completeness concern is a material unresolved gap documented in Technical Notes — the described mechanism is stated to require a harness change that may not yet exist, yet this is not surfaced in Assumptions, Open Questions, or Acceptance Criteria. A secondary gap is the meta/global/ path decision, which is flagged in Technical Notes but not carried forward into Requirements, Acceptance Criteria, or Open Questions.

**Strengths**:
- The Summary is a well-formed user-story statement that clearly names the operator persona, the want, and the benefit
- The Context section provides genuine motivation and links to the originating tech-debt note
- Requirements are specific and enumerated with concrete file paths
- The Technical Notes section supplies a pre-flight codebase survey, the full path-key vocabulary, and two explicit gap callouts
- The Assumptions section is genuinely informative
- Dependencies correctly identify the relationship with work item 0030

**Findings**:

- **MAJOR** (high confidence) — Technical Notes: Harness-change gap is not surfaced in Open Questions, Requirements, or Acceptance Criteria. The Drafting Notes simultaneously assert 'no harness change required', creating a direct contradiction.

- **MINOR** (high confidence) — Technical Notes: meta/global/ path gap is not carried into Requirements or Open Questions. The gap is documented but not actionable.

- **MINOR** (medium confidence) — Acceptance Criteria: No acceptance criterion covers the path-resolution skill's reusability constraint stated in Requirement 2.

### Dependency

**Summary**: The work item captures its relationship with 0030 clearly and correctly characterises it as non-blocking but alignment-relevant. The most significant dependency gap is the harness change implicitly required by the `skills:` frontmatter mechanism — the Technical Notes section explicitly surfaces this as an unresolved concern, but the Dependencies section still lists no blockers. A secondary gap is the unresolved `meta/global/` path decision, which requires an external decision before implementation can be finalised.

**Strengths**:
- 0030 is correctly characterised as a non-blocking related item with a clear explanation of why 0052 can proceed independently
- The work item names the source of truth for its key vocabulary (`config-read-path.sh`)
- The survey result is captured inline in Technical Notes, confirming the scope assumption

**Findings**:

- **MAJOR** (high confidence) — Dependencies: Harness change required by `skills:` mechanism is not captured as a blocker. Despite Technical Notes explicitly flagging the mechanism as non-existent, Dependencies lists 'Blocked by: none'.

- **MINOR** (high confidence) — Technical Notes: `meta/global/` path gap requires a decision that is not captured as a dependency.

- **MINOR** (medium confidence) — Dependencies: 0030 relationship is noted as 'Related' but the downstream rework impact (needing to update `config-read-all-paths.sh` after 0030 lands) is not captured.

### Scope

**Summary**: The work item is tightly scoped to a single coherent concern — making hardcoded paths in the documents-locator agent config-driven — and the requirements, acceptance criteria, and summary all describe the same unit of work. However, a critical gap in the Technical Notes reveals that the core implementation mechanism does not yet exist in the harness, contradicting the Drafting Notes claim of 'no harness change required'. If a harness change is needed, this story silently absorbs a second independent deliverable.

**Strengths**:
- All requirements serve a single unified purpose with no unrelated concerns bundled in
- The scope boundary is stated clearly; Requirement 7 properly bounds the open-ended survey risk
- The story type is appropriate for a system-behaviour change with user-visible impact
- The auto-discovery requirement is scoped correctly within the story
- The dependency on work item 0030 is correctly characterised with no scope entanglement

**Findings**:

- **MAJOR** (high confidence) — Technical Notes: Core implementation mechanism may require an undeclared harness change. If the harness change is in fact necessary, this story silently absorbs a second independent deliverable inside what is presented as a single story.

- **MINOR** (medium confidence) — Technical Notes: Unresolved meta/global/ gap may silently expand scope at implementation time. The implementer must resolve it unilaterally, with either outcome inconsistent with what the story commits to deliver.

### Testability

**Summary**: The work item's Acceptance Criteria are largely well-specified, with concrete preconditions and observable outcomes for the primary behavioural scenarios. However, two criteria have testability gaps: the survey-completion criterion is tautological as written, and the frontmatter constraint criterion tests an implementation detail rather than a verifiable outcome. Additionally, the significant unresolved gap in the Technical Notes — whether the `skills:` preload mechanism actually exists in the harness — is not surfaced in any Acceptance Criterion.

**Strengths**:
- The first three Acceptance Criteria follow a clear Given/When/Then pattern with specific preconditions and observable outcomes
- Requirement 5 maps cleanly to the second Acceptance Criterion
- Requirement 6 maps to the third Acceptance Criterion with a concrete example (`paths.specs: meta/specs`)
- Technical Notes section provides a grounded codebase survey and identifies two gaps demonstrating thorough pre-implementation analysis

**Findings**:

- **MAJOR** (high confidence) — Acceptance Criteria: Survey-completion criterion is tautological — can always be claimed as met. The parenthetical "(survey complete: confirmed)" is self-certifying with no independently verifiable artefact required.

- **MAJOR** (high confidence) — Acceptance Criteria: Frontmatter constraint criterion tests an implementation detail, not a verifiable outcome. Tests absence of a specific flag rather than observable behaviour.

- **MAJOR** (high confidence) — Acceptance Criteria: No criterion covers the `meta/global/` gap identified in Technical Notes. The explicit decision point has no corresponding Acceptance Criterion.

- **MAJOR** (high confidence) — Acceptance Criteria: No criterion covers the harness prerequisite. An implementation could ship a `skills:` key that is silently ignored, and all other Acceptance Criteria might still pass if fallback defaults coincidentally match expected values.

- **MINOR** (medium confidence) — Acceptance Criteria: First criterion does not specify the complete precondition scope. The absence of constraints on other path keys could cause misleading test results.

## Re-Review (Pass 2) — 2026-05-07

**Verdict:** REVISE

### Previously Identified Issues

- ✅ **Clarity**: Harness-change contradiction — Resolved. Context now correctly states the mechanism is "extended to agent definitions as part of this story"; Summary and Drafting Notes no longer claim "no harness change".
- ✅ **Completeness / Dependency / Scope / Testability**: `meta/global/` gap — Resolved. Decision recorded in Technical Notes; Requirement 8 and a new AC added.
- ✅ **Testability**: Survey AC tautological — Resolved. Now requires the result to be recorded as an inspectable artefact.
- ✅ **Testability**: Frontmatter flag AC — Resolved. Replaced with a behavioural context-injection criterion.
- ✅ **Clarity**: 'bang command' / 'bang-preprocessed' undefined — Resolved. Defined inline in Context.
- ✅ **Clarity**: `disable-model-invocation` / preload incompatibility unexplained — Resolved. Explained in Requirement 2.
- ✅ **Clarity**: Config file naming inconsistency — Resolved. Note added to References.
- ✅ **Clarity**: 'structured block' underspecified — Resolved. Output format specified normatively in Technical Notes.
- ✅ **Dependency**: 0030 post-delivery rework — Resolved. Captured in Blocks entry.
- ✅ **Testability**: First AC precondition scope — Resolved. Precondition now pins all other keys at defaults.
- 🔵 **Completeness**: Reusability AC absent — Partially resolved. AC added, but testability flags it still needs stronger verification framing (see new issues).

### New Issues Introduced

- 🟡 **Completeness**: Open question about `config-read-all-paths.sh` vocabulary scope is unresolved and blocks Requirement 1. The question (full vocabulary vs. document-discovery subset) directly governs the script's design and affects the auto-discovery AC; an implementer would need to stop and seek clarification.
- 🟡 **Testability**: Context-presence AC (sixth criterion) has no observation procedure. "The preloaded path block is present in the agent's context before it acts" describes an internal runtime state with no defined way to inspect it. Unlike other criteria, it cannot be verified by observing agent behaviour.
- 🔵 **Clarity**: Implicit referent "skills carrying that flag" in Requirement 2 — could be pinned to the explicit key name for precision.
- 🔵 **Clarity**: "the agent" in the context-presence AC is ambiguous — could refer to documents-locator specifically or any agent bearing `skills: [paths]`.
- 🔵 **Dependency**: Internal ordering constraint (Requirement 9 must precede Requirements 2–4) is not captured.
- 🔵 **Testability**: Reusability AC (seventh criterion) lacks a trigger and named verification target.

### Assessment

Pass 2 resolved all six major and the critical finding from pass 1 — a substantial improvement. Two new major issues emerged from the edits: the long-standing Open Question about vocabulary scope is now flagged as implementation-blocking, and the newly-introduced context-presence AC cannot be verified by observable behaviour. Both are addressable with targeted edits. The work item is not yet ready for implementation.

## Re-Review (Pass 3) — 2026-05-07

**Verdict:** REVISE

### Previously Identified Issues

- ✅ **Completeness / Dependency**: Open question about vocab scope blocking Req 1 — Resolved. Resolved to subset of 11 document-discovery keys; Technical Notes updated accordingly.
- ✅ **Testability**: Context-presence AC with no observation procedure — Resolved. Reframed as behavioural outcome ("searches `custom-work/`").
- ✅ **Clarity**: "skills carrying that flag" implicit referent — Resolved. Now reads "skills carrying `disable-model-invocation: true`".
- ✅ **Clarity**: "the agent" ambiguous in AC 6 — Resolved. Pinned to "documents-locator".
- ✅ **Dependency**: Internal ordering constraint (Req 9 → Reqs 2–4) not captured — Resolved. Added to Dependencies section.
- 🔵 **Testability**: Reusability AC lacking verification target — Partially resolved. Structural check added; testability now flags AC7 tests absence of edits rather than functional correctness (see new issues).

### New Issues Introduced

- 🟡 **Completeness**: No isolated AC for Requirement 9 (harness extension). AC6 exercises it end-to-end bundled with documents-locator, but a broken harness extension compensated for elsewhere would still pass. No criterion verifies the harness supports `skills:` frontmatter for agent definitions in general.
- 🟡 **Testability**: No AC for the init-process update mandated by Requirement 8. AC4 verifies documents-locator respects `paths.global` at runtime, but does not verify the init process prompts for or persists the `global` key. The requirement could be silently omitted and all criteria would still pass.
- 🔵 **Clarity**: Ambiguous "it" pronoun in six "when invoked, then it searches" criteria — "it" resolves to documents-locator in context but is slightly imprecise; "then the agent searches" would be uniformly explicit.
- 🔵 **Clarity**: Passive clause in Requirement 2 ("are excluded from the harness preload pipeline") omits which harness component enforces the constraint.
- 🔵 **Dependency**: "Blocks (post-delivery)" framing for the 0030 coupling overstates directionality — neither story blocks the other; this is a future alignment action.
- 🔵 **Scope**: Conditional scope-expansion language in Assumptions is moot since the survey is already complete and recorded in Technical Notes.
- 🔵 **Testability**: AC3's "included in search scope" lacks a concrete observable action (what agent behaviour constitutes inclusion?).
- 🔵 **Testability**: AC7's reusability clause tests absence of edits rather than that the skill actually works in another agent context.

### Assessment

All six pass-2 findings are resolved, including the vocabulary-scope open question and the context-presence AC verifiability gap. Pass 3 surfaces two new major gaps — both around missing acceptance criteria for the ancillary requirements (Req 8 init update, Req 9 harness extension in isolation) — plus six minors, mostly precision issues. The work item is substantially improved but these two ACs are needed before the verification framing is complete.
