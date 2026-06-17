---
type: work-item-review
id: "0122-audit-and-correct-missing-executable-bits-on-skill-entrypoint-scripts-review-1"
title: "Work Item Review: Audit and Correct Missing Executable Bits on Skill Entrypoint Scripts"
date: "2026-06-20T13:57:18+00:00"
author: "Toby Clemson"
producer: review-work-item
status: complete
target: "work-item:0122"
work_item_id: "0122"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: [scripts, permissions, ci, lint]
last_updated: "2026-06-20T16:17:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Audit and Correct Missing Executable Bits on Skill Entrypoint Scripts

**Verdict:** REVISE

This is a structurally complete, internally consistent, and well-bounded
task-kind work item: completeness found no gaps, scope confirmed a single
coherent unit, and clarity confirmed the central entrypoint-versus-library
distinction is defined precisely. The verdict is REVISE solely on two **major
testability findings** — the classification step (the central act of the work)
and the four named suspects both lack a definitive, auditable pass/fail target,
so a wrong classification could still pass the criteria. A recurring secondary
theme across three lenses is that the two unresolved Open Questions leave the
delivered scope, the continuous-enforcement boundary, and a convention coupling
undefined.

### Cross-Cutting Themes

- **Unresolved Open Questions leave scope and enforcement undefined** (flagged
  by: testability, scope, dependency) — the bidirectional-guard question widens
  the unit of work (scope), leaves the "library = 100644" half of AC1 without a
  clear continuous-vs-one-time enforcement boundary (testability), and the
  `mise run fix` question couples to the established "shell has no autofixer"
  convention (dependency). Resolving both before implementation pins all three
  down.
- **Classification correctness is asserted, not auditable** (flagged by:
  testability, clarity) — the heuristic-based classification has no definitive
  per-file procedure or evidence trail (testability AC1/AC2), and the
  `shell_sources()` helper the work depends on is named without a locating
  pointer (clarity).

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: AC1 classification has no definitive procedure to produce a pass/fail per file
  **Location**: Acceptance Criteria
  The "entrypoint vs library" determination is the very thing being verified,
  but AC1 provides no definitive procedure to settle it — a path-reference
  search excluding `source`/`.` is a heuristic, so a reviewer cannot
  conclusively confirm a file was classified correctly versus plausibly.
  Suggest AC1 reference an auditable classifier output (each `.sh` file with its
  detected class and the path-reference evidence justifying it).

- 🟡 **Testability**: AC2 "correctly resolved" lacks a stated expected classification per suspect
  **Location**: Acceptance Criteria
  AC2 requires the four named suspects to be "correctly resolved" but states no
  expected class/mode for each, so "correctly" has no fixed referent. A wrong
  classification could still pass. Suggest baking the expected resolution into
  AC2 (e.g. "all four resolve as entrypoints → mode 100755", or whatever the
  audit concludes).

#### Minor

- 🔵 **Dependency**: Shared `lint:shell` extension point co-occupied with 0107 not captured as an ordering touchpoint
  **Location**: Dependencies
  Both 0122 and 0107 add a new check into the same `scripts`/`lint:shell` family
  wired into `mise run check`; the Dependencies section lists 0107 only as a
  generic companion relation. If both land independently they may conflict on
  the task wiring. Suggest a coordination note, or an explicit statement that
  they are independent additions.

- 🔵 **Dependency**: Plugin packaging/release path named as the real risk surface but not captured as a downstream coupling
  **Location**: Drafting Notes
  The Drafting Notes flag the packaged plugin as where a missing bit actually
  manifests and as the priority-bump trigger, yet Dependencies names no coupling
  to release/packaging. Suggest recording it so a step-1 discovery of a
  currently-shipped broken entrypoint has a named escalation target.

- 🔵 **Testability**: AC4 "no false positives" has no enumerable verification target
  **Location**: Acceptance Criteria
  "No false positives from vendored scripts" is an open-ended negative claim
  naming only one illustrative example. Suggest reframing as a checkable
  behaviour (the guard's enumerated input excludes `node_modules/`/`workspaces/`/
  `target/`, verified by confirming a known Playwright bundled `.sh` is absent).

- 🔵 **Testability**: AC5 documentation criterion is verifiable only as existence, not adequacy
  **Location**: Acceptance Criteria
  AC5 specifies no content threshold — a one-line mention and a complete rule
  satisfy it equally. Suggest tying it to usability (states the
  shebang-plus-path-reference test and gives the test-runner-vs-test-helper
  discriminating example).

- 🔵 **Testability**: Unresolved bidirectional-guard question leaves AC verification scope undefined
  **Location**: Open Questions
  AC1 asserts every sourced-only library is 100644, but if the guard is
  entrypoint-only that half has no automated backing. Resolve the Open Question
  and align AC1/AC3 on which directions are continuously enforced vs one-time
  corrected.

#### Suggestions

- 🔵 **Clarity**: `shell_sources()` referenced as a known helper without a locating link
  **Location**: Requirements
  The required enumeration helper is cited four times but the work item never
  says which file defines it. Suggest a one-line pointer to its location.

- 🔵 **Clarity**: VCS acronym used without expansion
  **Location**: Context
  "VCS" is unexpanded on first use. Negligible for this team; expand for polish.

- 🔵 **Clarity**: "fresh checkout" could be read as git clone or jj working-copy checkout
  **Location**: Acceptance Criteria
  Given the item stresses git/jj visibility differences, "fresh checkout" is
  mildly ambiguous. Suggest reusing the Assumptions' "committed mode recorded by
  the VCS" wording.

- 🔵 **Scope**: Documentation deliverable rides along with the correction-and-guard work
  **Location**: Requirements
  The docs requirement is separable from the chmod and guard. Cohesive enough to
  keep in scope; suggest treating it as the lowest-priority sub-deliverable so it
  cannot block the fix.

- 🔵 **Scope**: Bidirectional-guard open question could quietly widen scope
  **Location**: Open Questions
  Extending to the library direction broadens the unit of work. Resolve before
  implementation; if bidirectional is chosen, reflect it in the Summary.

- 🔵 **Dependency**: `mise run fix` wiring decision has an unstated coupling to the shell-no-autofixer convention
  **Location**: Open Questions
  Wiring an auto-`chmod` into `mise run fix` would be the repo's first shell
  autofixer. If pursued, note the deviation in Dependencies; otherwise record
  the question as resolved in favour of the existing convention.

### Strengths

- ✅ The central entrypoint-versus-sourced-library distinction is explicitly and
  precisely defined in Requirements (shebang + path-reference excluding
  `source`/`.`), removing the main interpretation risk.
- ✅ Scope is consistent across Summary, Requirements, and Acceptance Criteria —
  the audit/correct/guard/document quartet appears identically with no drift,
  and the work reads as one coherent increment.
- ✅ Structurally complete and densely populated for a task: Summary, Context,
  Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions,
  Technical Notes, and Drafting Notes all carry substantive content; frontmatter
  is coherent (kind/status/priority appropriate).
- ✅ AC3 (exec-bit-removed → check fails naming the file; all-correct → passes)
  is an exemplary round-trip testable criterion, and criteria express outcomes
  as observable file modes (100755/100644) rather than implementation steps.
- ✅ Dependencies are accurately framed: 0106 as the causal convention, 0098 as
  the already-done host toolchain, 0107 as companion — all correctly relates-to
  rather than blockers, with frontmatter and prose mutually consistent.
- ✅ The Assumptions section pre-empts the principal ambiguity (how broadly
  "executable as part of claude skills" is read) by stating both the chosen
  interpretation and the narrower alternative.

### Recommended Changes

1. **Make the classification auditable** (addresses: AC1 procedure, AC2 expected
   resolution) — require the implementation to emit a classifier listing of
   every `.sh` file with its detected class and the path-reference evidence, and
   state the expected class/mode for each of the four named suspects directly in
   AC2.
2. **Resolve the two Open Questions before implementation** (addresses:
   bidirectional scope, AC verification scope, `mise run fix` coupling) — decide
   whether the guard is bidirectional and whether auto-`chmod` joins
   `mise run fix`; then align AC1/AC3 on continuous-vs-one-time enforcement and
   update the Summary/Dependencies to match.
3. **Tighten AC4 and AC5 into checkable behaviours** (addresses: AC4 no-false-
   positives, AC5 doc adequacy) — reframe AC4 around the enumerated excluded
   input set with a Playwright-script spot check, and tie AC5 to the rule's
   usability (names the test + the discriminating example).
4. **Capture the two implied couplings** (addresses: 0107 shared extension
   point, packaging risk surface) — note the shared `lint:shell` wiring with
   0107 and the plugin-packaging escalation target in Dependencies.
5. **Minor clarity polish** (addresses: `shell_sources()` link, VCS, "fresh
   checkout") — add a locating pointer for `shell_sources()`, optionally expand
   VCS, and disambiguate "fresh checkout".

## Per-Lens Results

### Clarity

**Summary**: The work item communicates a single coherent intent with strong
internal consistency; actors and outcomes are concrete and the entrypoint-vs-
library distinction is defined precisely. A few domain terms (`shell_sources()`,
jj workspaces, `100755`) and one acronym (VCS) are used without local
definition, but team-domain context makes most reasonable; residual ambiguities
are minor.

**Strengths**:
- The entrypoint-versus-sourced-library distinction is explicitly defined in
  Requirements, removing the main interpretation risk.
- Scope is consistent across sections with no drift.
- Assumptions pre-empts the principal ambiguity by stating the chosen
  interpretation and the narrower alternative.

**Findings**:
- 🔵 suggestion (medium): `shell_sources()` referenced as a known helper without
  a locating link (Requirements) — cited four times but never located; add a
  file pointer.
- 🔵 suggestion (low): VCS acronym used without expansion (Context) — negligible
  for this team; expand for polish.
- 🔵 suggestion (low): "fresh checkout" could be read as git clone or jj
  working-copy checkout (Acceptance Criteria) — reuse Assumptions' "committed
  mode recorded by the VCS" wording.

### Completeness

**Summary**: This task-kind work item is structurally complete and densely
populated: every expected section is present and substantive, the work to be
done is clearly defined, and frontmatter is fully populated with a recognised
kind and appropriate draft status. No completeness gaps of consequence remain.

**Strengths**:
- All expected sections present and substantively populated — no placeholders.
- Context genuinely explains motivation (load-bearing exec bit under 0106; the
  latent-break risk) rather than restating the Summary.
- Frontmatter complete and coherent (kind/status/priority appropriate).
- Acceptance Criteria has five specific enumerated criteria, above the minimum.
- Requirements describe the actual work an implementer could start from.

**Findings**: _None._

### Dependency

**Summary**: The work item captures its principal couplings well — 0106 as the
upstream convention, 0098 as the already-shipped host toolchain, 0107 as the
companion lint, all accurately framed as relates-to rather than blockers, and
"Blocked by: none" is defensible. The gaps are an unstated coupling to the
shared shell-tooling extension point that 0107 also targets, and an unnamed
downstream coupling to the plugin packaging/release path that the Drafting Notes
themselves flag as the real risk surface.

**Strengths**:
- Upstream dependency on the existing `shell_sources()` helper is named and
  correctly treated as already-available, not a blocker.
- 0098 correctly identified as done and as the guard's home.
- 0106 accurately cited as the causal reason the exec bit is load-bearing.
- `relates_to` frontmatter and the prose Dependencies section are mutually
  consistent.

**Findings**:
- 🔵 minor (high): Shared `lint:shell` extension point co-occupied with 0107 not
  captured as an ordering touchpoint (Dependencies).
- 🔵 minor (medium): Plugin packaging/release path named as the real risk
  surface but not captured as a downstream coupling (Drafting Notes).
- 🔵 suggestion (low): `mise run fix` wiring decision has an unstated coupling to
  the shell-no-autofixer convention (Open Questions).

### Scope

**Summary**: A well-bounded task-kind work item with a single coherent purpose:
correct missing executable bits on direct-invocation entrypoints and prevent
recurrence. The classify/correct/guard/document elements all serve the one goal,
reading as one increment. The "task" kind fits; the only mild tension is the
documentation deliverable and CI guard riding alongside the one-time correction,
defensible as anti-regression closure.

**Strengths**:
- Single unified purpose — every requirement serves the one goal; no separable
  second capability lurking.
- Summary, Requirements, and Acceptance Criteria describe the same scope with no
  drift.
- Scope boundaries explicit ("not chmod +x everything"; named exclusions; the
  canonical test-runner-vs-test-helper hard case).
- "task" kind well-matched to the scope.

**Findings**:
- 🔵 suggestion (medium): Documentation deliverable rides along with the
  correction-and-guard work (Requirements) — keep in scope but lowest priority.
- 🔵 suggestion (low): Bidirectional-guard open question could quietly widen
  scope (Open Questions) — resolve before implementation; reflect in Summary if
  chosen.

### Testability

**Summary**: The Acceptance Criteria are unusually strong for a task of this
kind — most are Given/When/Then with concrete machine-checkable outcomes (modes,
named offending file, exit status). The main gap is the classification step (the
central act), whose pass/fail boundary is not pinned to a definitive procedure,
plus a couple of criteria leaning on under-specified terms ("correctly
resolved", "no false positives").

**Strengths**:
- AC3 is an exemplary testable criterion — input mutation, action, and both
  positive and negative expected outcomes.
- Criteria express outcomes as observable file modes and exit status, not
  implementation instructions.
- AC2 pins the named suspects, converting "audit everything" into a checkable
  subset.
- Technical Notes supply concrete verification mechanics (`test -x`, shebang =
  first line begins `#!`).

**Findings**:
- 🟡 major (high): AC1 classification has no definitive procedure to produce a
  pass/fail per file (Acceptance Criteria).
- 🟡 major (medium): AC2 "correctly resolved" lacks a stated expected
  classification per suspect (Acceptance Criteria).
- 🔵 minor (medium): AC4 "no false positives" has no enumerable verification
  target (Acceptance Criteria).
- 🔵 minor (medium): AC5 documentation criterion is verifiable only as existence,
  not adequacy (Acceptance Criteria).
- 🔵 minor (low): Unresolved bidirectional-guard question leaves AC verification
  scope undefined (Open Questions).

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-20

**Verdict:** COMMENT

Re-ran the four lenses that had findings (clarity, dependency, scope,
testability); completeness was clean in pass 1 and was not re-run. The work item
was substantially revised — the heuristic classifier was replaced with a
maintained, checked-in **library-list**, and the four named suspects were
audited against real call sites (correcting two: `accelerator-scaffold.sh` and
`doc-type-inference.sh` are sourced libraries, not entrypoint bugs). Both pass-1
major findings are resolved. Verdict improves from REVISE to COMMENT: one
residual major remains (a finer-grained refinement of AC1's negative assertion),
below the REVISE threshold of two.

### Previously Identified Issues
- 🟡 **Testability**: AC1 classification has no definitive procedure — **Resolved**. Classification is now a deterministic membership test against the library-list; the list is the auditable artefact.
- 🟡 **Testability**: AC2 "correctly resolved" lacks expected classification per suspect — **Resolved**. AC3 now records exact per-suspect outcomes (0004 → 100755; test-interactive-protocol.sh → 100755; accelerator-scaffold.sh → 100644; doc-type-inference.sh → 100644).
- 🔵 **Testability**: AC4 "no false positives" not enumerable — **Resolved**. Reframed to a named Playwright vendored script that must be absent from the `shell_sources()` input set.
- 🔵 **Testability**: AC5 doc verifiable only as existence — **Resolved**. Now requires the registration rule and the test-runner-vs-helper example.
- 🔵 **Testability**: bidirectional-guard question leaves AC scope undefined — **Partially resolved**. Open Question reframed around the list and the enforced direction is now explicit; the library-member modes remain intentionally open pending that decision.
- 🔵 **Dependency**: shared `lint:shell` extension point with 0107 — **Resolved**. Captured as an explicit ordering note in Dependencies.
- 🔵 **Dependency**: packaging risk surface uncoupled — **Resolved**. Named as a realisation surface with an escalation target and trigger.
- 🔵 **Dependency**: `mise run fix` autofixer-convention coupling — **Resolved**. Captured in Dependencies.
- 🔵 **Scope**: documentation deliverable rides along — **Resolved/acknowledged**. Re-review confirms it is a cohesive tail and the cleanest fracture line if shrinking is ever needed; no change required.
- 🔵 **Scope**: bidirectional open question could widen scope — **Resolved**. Open Question now states the cost (stripping bits from currently-executable libraries) explicitly.
- 🔵 **Clarity**: `shell_sources()` not located — **Resolved**. Now cited as `tasks/shared/sources.py:60`.
- 🔵 **Clarity**: VCS unexpanded — **Resolved**. Expanded on first use.
- 🔵 **Clarity**: "fresh checkout" ambiguous — **Resolved**. Reworded to "the committed mode recorded by the VCS, as seen on a fresh git clone".

### New Issues Introduced
- 🟡 **Testability** (AC1): the "never invoked by path" half is a negative assertion with no defined search procedure/corpus — confirming a negative needs a stated method. (Addressed post-re-review — see Assessment.)
- 🔵 **Clarity**: "currently-shipped entrypoint" escalation trigger leans on an undefined sense of "shipped".
- 🔵 **Clarity**: suspect scripts referenced by bare basename in some sections, `scripts/`-prefixed paths in others.
- 🔵 **Dependency**: the second Open Question doesn't cross-reference its Dependencies owner bullet; conditional release coupling tracked only narratively.
- 🔵 **Testability**: library-member modes (beyond the two named suspects) unverified pending the bidirectional Open Question; AC6 "classify unaided" clause is subjective; AC2's universe could be anchored to the `shell_sources()` corpus explicitly.

### Assessment
The work item is acceptable as-is (COMMENT) and materially stronger than pass 1.
After the re-review, the one residual major (AC1's negative-assertion procedure)
and two trivial clarity nits (the "currently-shipped" definition and the
bare-path corpus anchor) were addressed directly in the work item. The remaining
minors are genuinely contingent on the two Open Questions (bidirectional
enforcement; `mise run fix` autofixer) and are best closed when those decisions
are made — appropriately deferred rather than resolved speculatively. The work
item is ready for refinement/planning.

## Approval — 2026-06-20

**Verdict:** APPROVE

After the re-review, the two deferred Open Questions were resolved (bidirectional
guard adopted; `mise run fix` autofixer declined) and propagated through the
Summary, Requirements, and Acceptance Criteria, closing the contingent minors
noted above. With every script's expected end-state now pinned and continuously
enforced, the verdict is upgraded to APPROVE by the reviewer. The work item is
approved for planning.
