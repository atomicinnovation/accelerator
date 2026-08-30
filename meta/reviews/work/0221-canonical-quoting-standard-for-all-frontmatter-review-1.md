---
type: "work-item-review"
id: "0221-canonical-quoting-standard-for-all-frontmatter-review-1"
title: "Work Item Review: Canonical Quoting Standard for All Frontmatter"
date: "2026-08-30T07:55:23+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0136"
target: "work-item:0221"
work_item_id: "0221"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-08-30T13:34:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Canonical Quoting Standard for All Frontmatter

**Verdict:** REVISE

This is a strong, densely-specified standard-setting story: every section is present and substantive, the type-driven rule reduces to a precise per-value predicate, and the reproduction and most acceptance criteria name concrete inputs and observable outcomes. Two structural issues drive the REVISE, both concerning the producer-run enforcement path — its coverage set is unbounded (`and the like`) and its runtime guarantee has no deterministic verification procedure — compounded by a scope finding that the item bundles emitter, validator, corpus migration, and roughly a dozen skill edits into a single story. A separate cross-cutting theme is that the ratifying ADR-0065 is now `accepted`, yet three lenses found the item still describing it as `proposed`.

### Cross-Cutting Themes

- **Stale ADR-0065 status** (flagged by: completeness, dependency, clarity) — Dependencies still reads ADR-0065 "is drafted at status `proposed`", but the referenced ADR now carries `status: accepted`. The item contradicts its own reference, giving a false readiness signal about whether the blocker is discharged.
- **Producer-run enforcement is underspecified and hard to verify** (flagged by: scope, testability) — the enforcement wiring across producer skills is the item's least-pinned surface: an open-ended skill list, a non-deterministic SKILL.md runtime guarantee, and the most clearly separable slice for decomposition.
- **ADR-0065 acceptance already discharges in-flight scope** (flagged by: scope, and the stale-status theme) — with the ADR accepted, the "Ratify the standard" requirement and AC #8 describe work that is effectively done, inflating the item's apparent remaining scope.

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability**: Producer-coverage criterion uses unbounded 'and the like' list
  **Location**: Acceptance Criteria (AC #6)
  AC #6 and the Producer-run validation requirement end their enumeration of in-scope producer skills with "and the like", so a verifier cannot determine when producer coverage is complete. The single most important enforcement criterion cannot be conclusively passed — a skill silently lacking the step would go undetected.

- 🟡 **Testability**: 'Surfaces any violation before completing' has no deterministic verification procedure
  **Location**: Acceptance Criteria (AC #6)
  The enforcement lives in SKILL.md prose executed by a non-deterministic LLM at runtime, so no defined procedure conclusively confirms a skill will always surface a violation — only that the instruction is present. The criterion risks being judged met by presence of the instruction while the runtime guarantee stays unverified.

- 🟡 **Scope**: Story bundles emitter, validator, corpus migration, and cross-cutting skill wiring — closer to an epic
  **Location**: Requirements
  Declared as a single `story`, 0221 spans a `cli/document` renderer change, a `cli/corpus` validator extension, a corpus-wide migration, and enforcement wiring across ~12 producer SKILL.md files — two distinct subsystems. The producer-run validation wiring is the clearly separable slice: standard + renderer + validator + migration alone already yield a conforming corpus.

#### Minor

- 🔵 **Dependency**: 0227 declares itself blocked by 0221, but 0221 says 'Blocks: none'
  **Location**: Dependencies
  Work item 0227 lists "Blocked by: 0221" in its own Dependencies, yet 0221 asserts "Blocks: none" and records 0227 only as "Relates to". The downstream unblock relationship is invisible from this item's side, and the explicit "Blocks: none" is factually contradicted by the referenced consumer. (Confidence: high.)

- 🔵 **Clarity**: 'producer' denotes both a Rust call site and a producer skill
  **Location**: Acceptance Criteria
  The bare noun "producer" is used in two senses — the Rust emission call sites (Context, AC #1) and the SKILL.md producer skills (Requirements, AC #6) — with disambiguation resting only on whether the word "skill" is present. A reader can conflate the code-level emitter change with the skill-level validation step, which land in different files.

- 🔵 **Clarity**: 'the whole corpus validates' conflicts with config being out of validation scope
  **Location**: Requirements
  The Migrate-the-corpus requirement lists `.accelerator/config.md` and then claims "the whole corpus validates", but the same item defers config validation to 0227 and states the validator "covers the `meta/` corpus only" (0227 records config is rejected as `INVALID-TYPE`). An implementer could read this as a requirement that config pass `corpus frontmatter validate`, setting an unachievable bar.

- 🔵 **Scope**: Parent epic 0136 (shell-to-Rust migration) does not clearly match this item's theme
  **Location**: Frontmatter: parent
  0221 is parented to epic 0136 ("Migrate Shell Scripts into a Rust CLI"), but its theme is a frontmatter quoting standard, not a shell-to-Rust migration. Sibling item 0227 explicitly declined the same parent for exactly this reason.

- 🔵 **Completeness**: Story does not explicitly identify the beneficiary
  **Location**: Summary
  As a `story`, the Summary is an imperative outcome ("Establish one canonical quoting standard…") and never states for whom — unlike sibling 0227's "As a plugin maintainer, I want…". The beneficiary is inferable from Context but not stated in the need framing.

- 🔵 **Testability**: Headline intent (no non-conformant files reach the repo) maps to no verifiable criterion
  **Location**: Summary
  The Summary frames the defect as "non-conformant files reach the repository unnoticed", but the enforcement is producer-run and explicitly non-exhaustive, and no criterion asserts a repository-wide conformance guarantee — AC #7 verifies only the one-time migration end state.

- 🔵 **Testability**: ADR-ratification criterion omits verifiable specifics stated in Requirements
  **Location**: Acceptance Criteria (AC #8)
  AC #8 requires only that a scoped ADR "overrides the quoting clauses". The Requirements state two further checkable conditions — quote the overridden sentences verbatim, link both parents via `relates_to` — that no criterion captures.

#### Suggestions

- 🔵 **Clarity**: 'superseded sentences' undercuts the deliberate override-not-supersede distinction
  **Location**: Requirements
  The Ratify requirement insists the ADR "overrides" — not supersedes — the clauses, then instructs quoting "the superseded sentences verbatim". Using "superseded" for clauses labelled merely overridden blurs a load-bearing distinction. Replace with "overridden".

- 🔵 **Scope**: ADR-ratification requirement already satisfied by accepted ADR-0065
  **Location**: Requirements: Ratify the standard
  The "Ratify the standard" requirement and AC #8 call for a new ADR, but ADR-0065 already exists at `accepted`. Carrying a finished decision as an open requirement inflates apparent remaining scope; reframe it as a satisfied prerequisite or mark AC #8 done.

- 🔵 **Testability**: Sync write-back input in AC #2 lacks an executable invocation
  **Location**: Reproduction
  AC #2 names three write paths — `work create`, `work update`, the sync write-back — but Reproduction gives an exact command only for `work create`. Add the command or fixture that triggers the bidirectional-sync write-back so all three paths are independently executable.

- 🔵 **Completeness / Dependency / Clarity**: ADR-0065 described as 'proposed' though the referenced ADR is accepted
  **Location**: Dependencies
  See the cross-cutting theme above — three lenses independently flagged this. Update Dependencies (and any Open Questions / Requirements phrasing) to state ADR-0065 is now accepted.

### Strengths

- ✅ Every expected story section is present and densely populated — no empty or placeholder sections; Context explains the originating defect with specific observed incidents (0220's three `BAD-LINKAGE-SHAPE` violations, the 37-file PR #76 churn).
- ✅ Core nouns — "the canonical standard", "the shared renderer", "the validator" — each resolve to exactly one referent across every section.
- ✅ The override-not-supersession distinction between this item and ADR-0033/0034 is stated repeatedly and consistently, so both parents' continued authority is clear.
- ✅ Acceptance Criteria use explicit Given/When/Then phrasing with named actors and observable, mechanically-checkable outcomes — "byte-identical" serialisation (AC #4), validator exit codes (AC #2/#5), a regression test that must fail against the current emitter (AC #9).
- ✅ The Reproduction block supplies an exact command, expected state, and actual broken state, making the originating defect unambiguously verifiable.
- ✅ The config-validation seam to 0227 is carved cleanly and repeated consistently across Summary, Requirements, Acceptance Criteria, and Dependencies.
- ✅ The core deliverables (standard, renderer emission, validator, migration) are genuinely interdependent, justifying keeping them in one unit.

### Recommended Changes

1. **Close the producer-coverage set and split its verification** (addresses: Producer-coverage 'and the like' list; 'Surfaces any violation' non-deterministic) — Replace "and the like" in the Requirement and AC #6 with an explicit, closed enumeration of the in-scope producer SKILL.md files (or a canonical list a test asserts against). Split AC #6 into a statically verifiable part (each named SKILL.md contains the validate step, asserted by test/grep) and, if a runtime guarantee is intended, name the harness or fixture that drives a skill over a non-conformant document.

2. **Resolve the epic-vs-story sizing decision** (addresses: Story bundles emitter/validator/migration/skill-wiring) — Either promote 0221 to an epic, or split the producer-skill enforcement wiring into its own child story, leaving standard + renderer + validator + migration as the core deliverable. A judgement call — decompose only if the team treats the skill-wiring as separately schedulable.

3. **Correct the ADR-0065 status throughout** (addresses: the stale-status theme across three lenses) — Update Dependencies to state ADR-0065 is now `accepted` and the blocker discharged; reconcile the Open Questions / Requirements "Ratify" phrasing; consider marking AC #8 done or reframing the ratification requirement as a satisfied prerequisite.

4. **Fix the two internal contradictions** (addresses: 'the whole corpus validates' vs config out-of-scope; 'Blocks: none' vs 0227) — Narrow the migration claim to "every `meta/` document validates and config conforms byte-for-byte via the renderer". Change "Blocks: none" to name 0227 as a blocked consumer, keeping both sides of the coupling consistent.

5. **Tighten terminology and framing** (addresses: 'producer' overload; 'superseded sentences'; beneficiary; AC #8 specifics; sync write-back invocation) — Reserve "producer" for the code call sites and always say "producer skill" for the SKILL.md surface. Replace "the superseded sentences" with "the overridden sentences". Add a one-line beneficiary statement to the Summary. Extend AC #8 to assert the verbatim-quote and `relates_to` conditions. Add the sync write-back invocation to Reproduction.

## Per-Lens Results

### Clarity

**Summary**: The work item is largely unambiguous: "the standard", "the renderer", and "the validator" each hold a single consistent referent throughout, the reproduction and acceptance criteria are concrete, and the override-vs-supersede reasoning is spelled out carefully. The main clarity risks are a term ("producer") that silently switches between a Rust call site and a SKILL.md, and a migration claim ("the whole corpus validates") that sits in tension with config being explicitly unvalidatable. A stale status assertion about the ratifying ADR adds a smaller contradiction.

**Strengths**:
- Core nouns — "the canonical standard", "the shared renderer", "the validator" — each resolve to exactly one referent across every section.
- Acceptance Criteria use explicit Given/When/Then phrasing with named actors and observable outcomes.
- The override-not-supersession distinction is stated repeatedly and consistently.

**Findings**:
- minor (medium): 'producer' denotes both a Rust call site and a producer skill — Acceptance Criteria. Used in two senses without a consistent qualifier; reader must infer from presence of "skill".
- minor (medium): 'the whole corpus validates' conflicts with config being out of validation scope — Requirements. Contradicts the config-out-of-scope deferral and Technical Notes ("covers the `meta/` corpus only").
- suggestion (medium): 'superseded sentences' undercuts the override-not-supersede distinction — Requirements. Same sentence that insists on "overrides" instructs quoting "the superseded sentences".
- suggestion (low): ADR-0065 described as 'proposed' though the referenced ADR is accepted — Dependencies.

### Completeness

**Summary**: This story is exceptionally complete: every expected section is present and densely populated, and the frontmatter is fully specified with a recognised kind, status, and priority. Context thoroughly explains the originating defect and the broadened-scope decision, and the nine acceptance criteria concretely cover emitter, validator, migration, producer-run enforcement, and ADR ratification. The only gaps are a missing explicit beneficiary in the story framing and a Dependencies section describing ADR-0065 as still "proposed" when the referenced ADR reads "accepted".

**Strengths**:
- All expected story sections present and substantively populated.
- Context explains the motivation and originating defect in detail, including specific observed incidents.
- Acceptance Criteria contains nine specific, done-defining bullets across all deliverables.
- Frontmatter complete and correct (kind, status, priority, parent, relates_to, external_id, tags).
- Open Questions explicitly closed out; Assumptions and Drafting Notes capture the scope-growth rationale.

**Findings**:
- minor (medium): Story does not explicitly identify the beneficiary — Summary. Imperative framing, no "As a … I want …" unlike sibling 0227.
- suggestion (low): Dependencies describes ADR-0065 as 'proposed' but the referenced ADR is now accepted — Dependencies.

### Dependency

**Summary**: The work item captures its single upstream blocker (ADR-0065) with clear rationale and cleanly scopes its downstream handoff to 0227, and it involves no external systems or cross-team actions, so the coupling surface is small and largely well-mapped. The one gap is a downstream consumer: 0227 declares itself hard-blocked by 0221, yet 0221's Dependencies asserts "Blocks: none" and demotes 0227 to a mere "Relates to". A secondary point is that the ADR-0065 blocker is described as "proposed" when the referenced ADR is now accepted.

**Strengths**:
- The sole upstream blocker (ADR-0065) is explicitly captured with the reason it gates the work.
- The downstream handoff to 0227 is named across Context, Requirements, and Dependencies with a crisp scope boundary.
- The intra-story ordering (ADR accepted "before or with the validator change") is stated.
- No third-party APIs, vendor services, or cross-team actions implied — self-contained.

**Findings**:
- minor (high): 0227 declares itself blocked by 0221, but 0221 says 'Blocks: none' — Dependencies. Downstream unblock relationship invisible from this side; "Blocks: none" factually contradicted.
- suggestion (medium): ADR-0065 blocker described as 'proposed' but the referenced ADR is accepted — Dependencies. Stale blocker state could mis-sequence the work.

### Scope

**Summary**: 0221 is a largely coherent standard-setting item whose core pieces (define the standard, make the renderer emit it, extend the validator, migrate the corpus) are genuinely tightly coupled and belong together. The main scope concern is sizing: as a single story it also bundles enforcement wiring into ~12 producer SKILL.md files plus a full corpus-wide migration, spanning two subsystems (the cli/ Rust workspace and the skills/ markdown surface), which pushes it toward epic scale and contains at least one independently-shippable slice. Config validation is cleanly split to 0227.

**Strengths**:
- Config validation cleanly carved out to 0227 with a consistent out-of-scope boundary repeated across sections.
- Core deliverables genuinely interdependent — cannot migrate without the emitter, nor validate without the standard.
- Summary, Requirements, and Acceptance Criteria describe the same scope; the ratifying decision is externalised into ADR-0065.

**Findings**:
- major (medium): Story bundles emitter, validator, corpus migration, and cross-cutting skill wiring — closer to an epic — Requirements. Spans two subsystems; producer-run validation wiring is the clearly separable slice.
- minor (medium): Parent epic 0136 (shell-to-Rust migration) does not clearly match this item's theme — Frontmatter: parent. Sibling 0227 declined the same parent for this reason.
- suggestion (low): ADR-ratification requirement already satisfied by accepted ADR-0065 — Requirements: Ratify the standard. Carrying a finished decision as an open requirement inflates apparent scope.

### Testability

**Summary**: This is an unusually testable story: the canonical rule is type-driven and reduces to a precise per-value pass/fail predicate, most Acceptance Criteria name concrete inputs and observable outcomes (exit codes, byte-identical serialisation, validator behaviour), and Reproduction gives an exact command with expected/actual states. The principal weakness is the producer-run enforcement path — AC #6 relies on an open-ended "and the like" list and on non-deterministic SKILL.md-driven behaviour, neither of which yields a closed, definitively verifiable coverage set. Secondary gaps are that the headline intent is not reducible to any single criterion, and that a few verifiable specifics stated in Requirements are not lifted into the Acceptance Criteria.

**Strengths**:
- AC #2 and AC #5 define concrete inputs and binary outcomes, each a definitive pass/fail procedure.
- AC #4's "byte-identical" criterion is a strong, mechanically checkable anti-churn outcome.
- AC #7 turns the migration into a testable end state bounded to the concrete corpus at migration time.
- AC #9 specifies a regression test that must fail against the current emitter — an executable red-state anchor.
- The Reproduction block supplies an exact command, expected result, and actual broken result.

**Findings**:
- major (high): Producer-coverage criterion uses unbounded 'and the like' list — Acceptance Criteria. No closed list of exact producer SKILL.md files; coverage completeness undecidable.
- major (medium): 'Surfaces any violation before completing' has no deterministic verification procedure — Acceptance Criteria. Runtime guarantee via non-deterministic LLM; only instruction-presence is verifiable.
- minor (medium): Headline intent (no non-conformant files reach the repo) maps to no verifiable criterion — Summary. Enforcement is producer-run and non-exhaustive; no repo-wide conformance criterion.
- minor (medium): ADR-ratification criterion omits verifiable specifics stated in Requirements — Acceptance Criteria (AC #8). Verbatim-quote and relates_to conditions not captured.
- suggestion (medium): Sync write-back input in AC #2 lacks an executable invocation — Reproduction. Only the work-create leg has a runnable command.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-08-30

**Verdict:** REVISE

Re-ran scope and testability — the two lenses that carried the Pass-1 majors — against the revised work item. Both majors improved substantially; the residue is confined to tightening the two verification-shaped criteria and acknowledging one deliberate scope fault line. All Pass-2 findings were then addressed by immediate edits (recorded below); a confirmatory Pass 3 was not run.

### Previously Identified Issues

- 🟡 **Testability**: Producer-coverage 'and the like' unbounded list — **Resolved.** AC #6 now names a closed, enumerated in-scope set asserted by a static test; the re-review explicitly praised this.
- 🟡 **Testability**: 'Surfaces any violation' non-deterministic — **Partially resolved, then addressed.** The Pass-1 finding was split into a static coverage AC (resolved) plus a runtime AC; Pass 2 flagged the runtime AC still drove an LLM-executed skill with an unnamed "representative" skill and no observable signal. Post-Pass-2 edit reframed it to the deterministic command-level signal (`corpus frontmatter validate` exits non-zero and emits the violation on stderr).
- 🟡 **Scope**: Epic-scale bundling — **Still present, softened, and acknowledged.** Pass 2 narrowed the fault line to the producer-skill enforcement wiring (depends only on the extended validator, could ship independently). Per the keep-as-one-story decision, the sizing Drafting Note now records that the wiring is a distinct deliverable to split into a follow-up child story if it threatens to stall the core fix.

### New Issues Introduced

- 🔵 **Testability**: AC #8 was a scope declaration, not a checkable criterion (introduced by the Pass-1 remediation) — **addressed:** moved to Assumptions as a non-goal.
- 🔵 **Testability**: AC #1 left floats (and other non-bare-set scalars) without a stated expected form — **addressed:** AC #1 realigned to ADR-0065's closed bare-set framing (every scalar quoted except integer/boolean/null).

### Assessment

The work item is materially stronger: producer coverage is bounded and statically checkable, the runtime signal is now deterministic, AC #1 matches the ratifying ADR, and the sizing tradeoff is documented with an explicit split path. Both Pass-2 majors were addressed by the edits recorded above. The item is ready for planning; a confirmatory scope + testability pass would close it at APPROVE if desired.

## Re-Review (Pass 3) — 2026-08-30

**Verdict:** COMMENT

Confirmatory re-run of scope and testability. Scope's sizing concern dropped from major to a suggestion ("well-managed rather than acute"); testability surfaced one new high-confidence contradiction in the migration criterion. One major remains below the two-major REVISE threshold, so the verdict eases to COMMENT. The major and both observations were then addressed by immediate edits.

### Previously Identified Issues

- 🟡 **Scope**: Epic-scale bundling — **Downgraded to suggestion.** Pass 3 judged the sizing well-managed: closed producer set, explicit out-of-scope list, and a documented contingent split. Left as one story per the standing decision; no edit required.
- 🟡 **Testability**: Runtime "surfaces violation" verification — **Resolved as designed.** Pass 3 confirmed AC #6 (static presence) + AC #7 (deterministic command signal) are sound; it noted the end-to-end skill behaviour is verified by proxy. Added an Assumptions bullet stating AC #6 + AC #7 are the accepted proxy because prose-driven skill enforcement is not directly testable.

### New Issues Introduced

- 🟡 **Testability** (major, high): The migration acceptance criterion asserted `.accelerator/` config "subsequently validates", contradicting config being outside `corpus frontmatter validate` (the same class as the Pass-1 Requirements fix, missed in the parallel AC) — **addressed:** the criterion now reserves "validates under `corpus frontmatter validate`" for `meta/` and requires config to "conform byte-for-byte via the shared renderer (verified per AC #3)".
- 🔵 **Testability** (suggestion): AC #6's static assertion overstated what a text scan confirms — **addressed:** scoped explicitly to presence of the `corpus frontmatter validate` invocation.

### Assessment

No REVISE-level issue remains. The one Pass-3 major was a genuine internal contradiction, now fixed, and the two observations are closed. The work item is ready for planning. A further pass would be diminishing returns; the residual scope suggestion is a deliberate, documented sizing choice rather than a defect.

## Final Verdict — APPROVE (2026-08-30)

Approved by the reviewer after Pass 3. All majors across the three passes are resolved; the only residue is a documented, deliberate sizing choice (producer-skill wiring kept in one story, with a contingent split path recorded). The work item's status was transitioned to `ready`.
