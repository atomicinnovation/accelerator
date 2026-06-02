---
date: "2026-06-02T15:16:16+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0090-radius-tokens-consumption.md"
work_item_id: "0090"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
---

## Work Item Review: Radius Tokens Consumption

**Verdict:** REVISE

0090 is structurally complete, tightly scoped, and unusually clear about its
intent — every section is populated with real content, the extend-and-preserve
choice is stated consistently, and the two known outliers map concretely to
named tokens. The blocking concerns are not about what the story says, but
about two couplings it understates: its referenced sibling 0075 describes a
**hard ordering constraint and a set of mirror obligations** (EXCEPTIONS
retirement, ADR amendment, shared test harnesses) that 0090 does not capture;
and the acceptance criteria are anchored to a **discovery-sweep inventory that
does not yet exist**, leaving the most important guarantees (pixel-perfect, "no
literals remain") without a frozen reference set to verify against.

### Cross-Cutting Themes

- **0075 coupling is understated** (flagged by: clarity, dependency) — 0075's
  own Dependencies state "0090 must not begin implementation until 0075 lands"
  and attribute to 0090 the EXCEPTIONS-retirement pattern, ADR-amendment style,
  and shared enforcement harnesses. 0090 lists 0075 only as "Related" and
  mentions none of those obligations. Two documents describe the same
  relationship with different strength and scope.
- **Acceptance criteria depend on an inventory that hasn't been produced**
  (flagged by: testability, completeness, scope) — the discovery sweep is part
  of this story, so "every value", "zero matches", and "pixel-perfect vs
  pre-migration" have no enumerable baseline yet. 0075's sweep grew from 4
  outliers to 35 declarations across 9 files; the same contingency is live here.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Dependency + Clarity**: 0075 is a hard upstream blocker, captured only as "Related"
  **Location**: Dependencies
  0075's own Dependencies section states "0090 must not begin implementation
  until 0075 lands"; 0090 lists it under "Related" as a soft pattern precedent.
  A scheduler reading only 0090 could start it early and re-invent the canonical
  pattern. (Both the dependency and clarity lenses flagged this at high confidence.)

- 🟡 **Testability**: AC2 pixel-perfect check has no enumerated selectors or baseline
  **Location**: Acceptance Criteria
  AC2 asserts computed `border-radius` is "identical" to pre-migration, but names
  no selectors, no expected values, and no baseline — and the pre-migration
  inventory does not yet exist. The single most important guarantee (no visual
  change) has no defined verification procedure.

- 🟡 **Testability**: Unbounded "every" / "zero matches" anchored to a not-yet-existing inventory
  **Location**: Acceptance Criteria
  AC1 and AC3 are framed as exhaustive, but their reference set is the discovery
  sweep this story itself performs. Until that sweep is frozen, completeness
  cannot be confirmed — the criteria can be claimed met against whatever set the
  implementer happened to find.

- 🟡 **Dependency**: ADR amendment/creation coupling implied by the mirrored pattern but uncaptured
  **Location**: Requirements
  The Summary says 0090 "mirrors the consume-tokens-everywhere precedent set by
  0075"; 0075's pattern explicitly amended ADR-0026 and created ADR-0036. 0090
  names no ADR work anywhere, so the radius rule could land without the decision
  record the sibling pattern requires.

- 🟡 **Clarity**: 0075 attributes an EXCEPTIONS-retirement pattern to 0090 that 0090 never mentions
  **Location**: Requirements
  0075 says 0090 adopts its "EXCEPTIONS retirement pattern … verbatim for
  border-radius", but 0090's Requirements cover only scale extension, migration,
  a grep gate, and single-PR delivery. The two documents disagree on 0090's scope.

- 🟡 **Testability**: AC3 grep pattern under-specified relative to the longhand-coverage requirement
  **Location**: Acceptance Criteria
  AC3 and the Assumptions require the gate to cover all four longhand corner
  forms, but the only pattern offered (`grep -rn 'border-radius:\s*[0-9]'`, a
  "starting point" in Technical Notes) does not match the longhands it must
  catch. Two verifiers could run different patterns and reach different verdicts.

#### Minor

- 🔵 **Completeness**: Full work inventory deferred to an as-yet-unperformed discovery sweep
  **Location**: Requirements
  Only two outliers are enumerated; the true migration size is unknown until the
  sweep runs. 0075 carries its full dated inventory (35 declarations, 9 files) in
  Context. Either run the sweep and fold it in, or state explicitly that the
  inventory is the story's first deliverable.

- 🔵 **Dependency**: Test-harness coupling (migration.test.ts / Playwright regression) not surfaced
  **Location**: Acceptance Criteria
  0075 coupled its migration to the `migration.test.ts` EXCEPTIONS list and a
  Playwright `getComputedStyle` suite. 0090 implies similar checks but doesn't
  say whether it reuses those harnesses or must retire existing radius EXCEPTIONS
  entries.

- 🔵 **Clarity**: "CI/grep gate" / "grep/lint gate" used interchangeably
  **Location**: Requirements
  The enforcement mechanism is named four different ways across sections, and the
  ACs split it into a standalone grep check and a separate CI-fails criterion. A
  reader can't tell whether one mechanism or two is being built.

- 🔵 **Testability**: AC4 CI-gate criterion lacks a defined trigger fixture
  **Location**: Acceptance Criteria
  AC4 has no stated negative-test input (what literal, in which file) to confirm
  the gate actually fails the build rather than silently passing.

- 🔵 **Testability**: AC5 naming-convention criterion is partly subjective via "clean scale slot"
  **Location**: Acceptance Criteria
  Whether a value has a "clean scale slot" is a judgement with no defined
  threshold, so a future off-scale token's naming choice can't be definitively
  passed or failed against AC5.

#### Suggestions

- 🔵 **Scope**: Discovery sweep may expand scope beyond the two known outliers
  **Location**: Open Questions / Requirements
  If the sweep returns a large or off-grid inventory (as 0075's did), the story
  could outgrow a single delivery unit. Borrow 0075's contingency framing — state
  up front that an unexpectedly large inventory re-scopes the story (e.g. to an
  epic with per-file-group children).

- 🔵 **Clarity**: "t-shirt slot" used without definition
  **Location**: Requirements / Drafting Notes
  Gloss the term on first use — e.g. "t-shirt-style step names (sm/md/lg/pill)" —
  so the --radius-block justification is self-contained.

- 🔵 **Clarity**: "scale-based name" left implicit
  **Location**: Summary / Requirements
  The two naming categories ("scale-based" vs "use-case-named") are load-bearing
  for AC5 but never explicitly defined. A one-line definition would anchor AC5.

### Strengths

- ✅ "Current app" is explicitly defined in Assumptions ("production component
  CSS, excluding the prototype"), removing the most likely ambiguity in a
  token-consumption story.
- ✅ The two known outliers map concretely to exact target tokens throughout
  (badge 2px → `--radius-xs`; `<pre>` 6px → `--radius-block`), with the
  use-case-naming exception explained consistently across Requirements, AC, and
  Drafting Notes.
- ✅ Tightly scoped and orthogonal: every requirement serves the single purpose
  of radius-token consumption within one ownership domain (current-app CSS),
  delivered as a single PR series — the "story" kind fits.
- ✅ Structurally complete and richly populated for its kind: all sections carry
  real, non-placeholder content, and frontmatter is valid.
- ✅ The verifiable criteria that don't depend on the missing inventory (grep
  returns zero matches; CI fails on a literal; computed-value equality as the
  regression definition) are framed as objective, scriptable checks.

### Recommended Changes

1. **Reconcile the 0075 relationship** (addresses: "0075 is a hard upstream
   blocker"; "0075 attributes an EXCEPTIONS-retirement pattern to 0090").
   Decide the true relationship and make both documents agree. If 0075 must land
   first, promote it from "Related" to "Blocked by: 0075" in Dependencies. Then
   decide whether 0090 actually inherits 0075's EXCEPTIONS-retirement and
   ADR-amendment obligations — if yes, add them to Requirements; if no, correct
   0075's Dependencies and note the deliberate omission in Drafting Notes.

2. **Anchor the acceptance criteria to a frozen inventory** (addresses: "AC2 has
   no baseline"; "unbounded every/zero matches"; "full inventory deferred").
   Add an AC requiring the discovery-sweep inventory (selector → current px) to
   be recorded in a named artefact, and rephrase AC1/AC2/AC3 against that frozen
   set — including a computed-style regression spec asserting each migrated
   selector's post-migration radius equals its recorded value (mirroring 0075's
   AC7).

3. **Specify the exact gate pattern(s)** (addresses: "AC3 grep pattern
   under-specified"; "AC4 lacks a trigger fixture"; "gate named four ways").
   Pin one name for the mechanism, give the exact ripgrep alternation covering
   `border-radius` plus all four corner longhands in AC3, and add a negative-test
   fixture to AC4 (e.g. "inserting `border-radius: 7px` makes the gate exit
   non-zero").

4. **Capture the ADR / test-harness couplings** (addresses: "ADR coupling
   uncaptured"; "test-harness coupling not surfaced"). State in Dependencies /
   Technical Notes whether 0090 reuses the existing `migration.test.ts` EXCEPTIONS
   list and Playwright regression suite, and whether any existing radius
   EXCEPTIONS entries must be retired.

5. **Add a sizing contingency** (addresses: "discovery sweep may expand scope").
   Borrow 0075's framing: note that an unexpectedly large sweep result re-scopes
   the story (e.g. to an epic with per-file-group children).

6. **Tighten remaining clarity/testability nits** (addresses: "t-shirt slot",
   "scale-based name", "clean scale slot subjective"). Gloss "t-shirt slot",
   define the two naming categories once, and make AC5's slot rule objective
   ("on-scale iff equal to an existing `--radius-*` px value; otherwise record a
   one-line naming rationale in the PR").

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: 0090 is largely unambiguous: "current-app" is explicitly defined,
the two known outliers and their target tokens are named concretely, and the
extend-and-preserve intent is stated plainly in three places. The main clarity
concerns are cross-document inconsistencies with its references (0075
characterises the 0075→0090 relationship differently than 0090 does, and 0075
attributes pattern elements to 0090 that 0090 never mentions) and a couple of
terms used without definition ("CI/grep gate", "t-shirt slot").

**Strengths**:
- "Current app" explicitly defined in Assumptions.
- Two known outliers and exact target tokens named concretely throughout.
- The --radius-block use-case exception explained consistently across sections.
- Extend-and-preserve / zero-visual-change intent stated unambiguously.

**Findings**:
- 🟡 major (high) — Dependencies: Dependency relationship to 0075 contradicts how
  0075 describes it. 0090 lists 0075 as "Related" but 0075 states "0090 must not
  begin implementation until 0075 lands" and lists 0090 under "Blocks".
- 🟡 major (medium) — Requirements: 0075 attributes an EXCEPTIONS-retirement
  pattern to 0090 that 0090 never mentions. 0090's Requirements cover only scale
  extension, migration, grep gate, single-PR delivery — no EXCEPTIONS retirement
  or ADR amendment.
- 🔵 minor (medium) — Requirements: "CI/grep gate" and "grep gate" used
  interchangeably across four sections without confirming they are one mechanism.
- 🔵 suggestion (medium) — Requirements: "t-shirt slot" used without definition.
- 🔵 suggestion (low) — Summary: "scale-based name" for --radius-xs left implicit.

### Completeness

**Summary**: This story is structurally complete and richly populated for its
kind: clear Summary, motivating Context, substantive Requirements, five specific
Acceptance Criteria, and well-filled Open Questions, Dependencies, Assumptions,
and Technical Notes. Frontmatter is valid. The only completeness concern is that
the central scope-defining artefact — the discovery sweep inventory — does not
yet exist.

**Strengths**:
- Summary is a single unambiguous action statement.
- Context explains why the work exists rather than restating the summary.
- Five specific Given/When/Then acceptance criteria.
- All optional sections populated with relevant, non-placeholder content.
- Frontmatter complete and valid.

**Findings**:
- 🔵 minor (high) — Requirements: Full work inventory deferred to an
  as-yet-unperformed discovery sweep; only two outliers enumerated, whereas
  sibling 0075 carries a complete dated inventory (35 declarations, 9 files).

### Dependency

**Summary**: The work item correctly names its sibling precedents (0033, 0075,
0041, 0077) and is genuinely standalone at the system boundary (current-app CSS
only, no external systems). However, it understates one hard upstream ordering
constraint its own referenced source (0075) states explicitly, and leaves the
ADR/test-harness couplings central to the pattern it mirrors uncaptured.

**Strengths**:
- Dependencies correctly identifies 0033 as the scale source and 0075 as the
  pattern precedent.
- Genuinely self-contained at the system boundary — no third-party APIs or
  cross-team actions.
- Assumptions explicitly scope out the prototype and codify extend-and-preserve.

**Findings**:
- 🟡 major (high) — Dependencies: Hard ordering constraint on 0075 captured only
  as "Related"; 0075 says "0090 must not begin implementation until 0075 lands".
- 🟡 major (medium) — Requirements: ADR amendment/creation coupling implied by the
  mirrored pattern (0075 amended ADR-0026, created ADR-0036) but uncaptured.
- 🔵 minor (medium) — Acceptance Criteria: Test-harness coupling
  (migration.test.ts EXCEPTIONS / Playwright getComputedStyle regression) not
  surfaced as a dependency.

### Scope

**Summary**: 0090 is a tightly-bounded, coherent story: every requirement serves
the single purpose of migrating current-app border-radius declarations onto
var(--radius-*) tokens. It is a deliberate narrowing of the consume-tokens
pattern 0075 set for typography, scoped to one CSS concern within one ownership
domain, delivered as a single PR series. The only scope observation is a
contained discovery-sweep uncertainty the work item itself flags.

**Strengths**:
- All four Requirements bullets serve one unified purpose — no bundled second concern.
- Summary, Requirements, and AC describe the same scope with consistent boundaries.
- Scope boundaries explicitly stated (in/out of scope in Assumptions).
- Single ownership domain and single delivery unit.
- The "story" kind fits, consistent with precedent 0075.

**Findings**:
- 🔵 suggestion (medium) — Open Questions / Requirements: Discovery sweep may
  expand scope beyond the two known outliers; borrow 0075's contingency framing
  (re-scope to an epic if the inventory balloons, as 0075's did from 4 to 35).

### Testability

**Summary**: The Acceptance Criteria are mostly framed as observable
Given/When/Then checks with concrete pass/fail procedures, and the grep-gate and
pixel-perfect criteria are genuinely verifiable. However, several criteria lean
on unbounded language anchored to an inventory that does not yet exist, and the
pixel-perfect regression check lacks the concrete selector enumeration the
sibling 0075 spelled out.

**Strengths**:
- AC3 and AC4 give concrete, scriptable pass conditions.
- AC2 specifies pixel-perfect computed-value equality as an objective comparison.
- Requirements name the two token additions and exact px values.
- AC5 carves out the use-case-name exception explicitly.

**Findings**:
- 🟡 major (high) — Acceptance Criteria: AC2 pixel-perfect check has no enumerated
  selectors or expected values to verify against; no baseline exists yet.
- 🟡 major (high) — Acceptance Criteria: Unbounded "every" / "zero matches"
  anchored to a not-yet-existing inventory.
- 🟡 major (medium) — Acceptance Criteria: AC3 grep pattern under-specified
  relative to the longhand-coverage requirement; the offered pattern doesn't
  match the longhands.
- 🔵 minor (medium) — Acceptance Criteria: AC4 CI-gate criterion lacks a defined
  trigger fixture (negative test).
- 🔵 minor (medium) — Acceptance Criteria: AC5 naming-convention criterion is
  partly subjective via "clean scale slot".

## Re-Review (Pass 2) — 2026-06-02T15:05:12+00:00

**Verdict:** REVISE

All fifteen findings from Pass 1 are resolved or acknowledged. The verdict
remains REVISE only because two new **major** findings surfaced — both from the
testability lens, and both a direct consequence of the deliberate decision to
defer the radius inventory to pre-implementation codebase research. They are
addressable with one small clarifying edit (commit AC3 as the self-sufficient,
filesystem-determined completeness gate and fix its directory root), after
which the work item is implementation-ready.

### Previously Identified Issues

- 🟡 **Dependency + Clarity**: 0075 captured only as "Related" — **Resolved**
  (now `Blocked by: 0075 (done)` with the satisfied ordering stated; both lenses
  now cite it as a strength).
- 🟡 **Testability**: AC2 pixel-perfect had no baseline/selectors — **Resolved**
  (AC2 is now a Playwright `getComputedStyle` regression spec; minor residue on
  route/viewport context, see below).
- 🟡 **Testability**: Unbounded "every"/"zero matches" — **Partially resolved**
  (grep gate AC3 is now the backstop, but the testability lens still flags the
  inventory-deferral; see new findings).
- 🟡 **Dependency**: ADR amendment/creation coupling uncaptured — **Resolved**
  (Requirements + AC6 + Dependencies now capture the new ADR and ADR-0026 §3
  amendment).
- 🟡 **Clarity**: 0075's EXCEPTIONS-retirement pattern unmentioned — **Resolved**
  (now a Requirement and AC5).
- 🟡 **Testability**: AC3 grep pattern under-specified — **Resolved** (three
  exact ripgrep sweeps covering shorthand + four corner longhands).
- 🔵 **Completeness**: Full inventory deferred — **Resolved/acknowledged** (now
  explicit in Context/Assumptions/Drafting Notes; completeness cites it as a
  disclosed gap).
- 🔵 **Dependency**: Test-harness coupling not surfaced — **Resolved** (in
  Dependencies + Technical Notes).
- 🔵 **Clarity**: Gate named four ways — **Resolved** (standardised on a single
  CI grep gate).
- 🔵 **Testability**: AC4 had no trigger fixture — **Resolved** (`border-radius:
  7px` negative test).
- 🔵 **Testability**: AC5 "clean scale slot" subjective — **Resolved** (AC8 now
  defines on-scale objectively).
- 🔵 **Scope**: Sizing contingency — **Resolved** (Drafting Note mirrors 0075's
  epic fallback).
- 🔵 **Clarity**: "t-shirt slot" jargon — **Resolved** (removed; Naming note).
- 🔵 **Clarity**: "scale-based name" implicit — **Resolved** (defined in Naming
  note + Drafting Notes).

### New Issues Introduced

- 🟡 **Testability** (major): AC1/AC2/AC3 completeness verification depends on a
  research inventory not in (or referenced by) the work item — a verifier can't
  enumerate AC2's selectors or AC3's file set from the document alone.
- 🟡 **Testability** (major): AC3's globs are labelled "confirmed against the
  research inventory", which makes the gate's pass condition provisional — an
  under-scoped glob could return zero matches and falsely read as a pass.
- 🔵 **Testability** (minor): AC2 omits route/viewport mounting context (cf.
  0075 AC7); AC6's "removed or marked superseded" lacks a single grep-able pass
  condition; AC1/AC8's "any further off-scale value" tail is open-ended (AC3 is
  the transitive backstop); AC5's 2px-badge EXCEPTIONS status unconfirmed.
- 🔵 **Clarity** (minor): the new radius ADR is never assigned an ID (AC7 says
  "reference by ID" but none is fixed); bare reference to 0091 with no
  descriptor; `--radius-block`'s "block" referent never tied to the code-block
  `<pre>`; AC3's longhand sweep omits the `global.css` exclusion the shorthand
  sweeps use, with no stated reason.
- 🔵 **Dependency** (minor): the pre-implementation research is a hard upstream
  input but lives in Assumptions, not Dependencies; epic-fallback child ordering
  not captured as a Dependencies note.
- 🔵 **Completeness / Scope** (suggestion): frontmatter uses `type:` where some
  siblings use `kind:` (legacy-shape, 0075 also uses `type:`); ADR authoring
  bundled into a CSS-migration story (judged load-bearing, no action needed).

### Assessment

A decisive improvement: every Pass 1 finding is closed, and three of the five
lenses (clarity, completeness, scope) now report no issue above minor. The two
new majors are not regressions in the usual sense — they are the testability
lens correctly observing that deferring the inventory to research leaves the
"completeness" half of AC1–AC3 unverifiable from the document alone. The fix is
small and already half-stated in the work item: declare AC3 the self-sufficient
completeness gate (zero literal matches against a fixed current-app CSS root,
independent of the inventory) and drop the "provisional globs" hedge; scope
AC2's enumerated selector list to "once research lands". With that edit — plus
the cheap clarity wins (assign/flag the ADR ID, gloss `--radius-block`, align
AC3 globs) — the work item is ready for planning.

## Re-Review (Pass 3) — 2026-06-02T15:16:16+00:00

**Verdict:** COMMENT (lens raw output was REVISE on 3 majors; the human-assessed
verdict is COMMENT after the follow-up edits — see assessment)

Final verification pass over the three edited lenses (testability, clarity,
dependency). It surfaced three new majors: one genuine defect introduced by the
Pass-2 edits (now fixed) and two that are the structural testability tension
inherent to deferring the radius inventory to pre-implementation research
(reframed per the lens's own suggestions).

### Previously Identified Issues (Pass 2 majors)

- 🟡 **Testability**: AC1/AC2/AC3 completeness depended on a missing inventory —
  **Resolved** (AC3 is now the self-sufficient, filesystem-determined gate; AC1
  delegates completeness to AC3; AC2 carries a concrete per-inventory-row join
  condition).
- 🟡 **Testability**: AC3 globs "provisional" — **Resolved** (globs pinned to a
  fixed `src/` root; hedge removed; AC4 gate scope aligned to AC3).

### New Issues Introduced (Pass 3)

- 🟡 **Clarity** (major) — **Fixed in follow-up.** AC8's objective on-scale test
  contradicted the Naming note's treatment of `--radius-xs: 2px` as scale-based.
  AC8 reworded: scale-based = equals an existing token *or* extends the ladder at
  a regular end step; use-case = falls between steps. "Off-scale" defined inline.
- 🟡 **Testability** (major) — **Reframed.** AC2's pass set undefined until
  research lands → now one assertion per inventory-table selector, inventory
  attached to the plan before AC2 is evaluated, tolerance pinned.
- 🟡 **Testability** (major) — **Reframed.** AC1 "every value" unbounded → now
  bounds the testable claim to the two known tokens + every inventory-table
  value, delegating the open tail to AC3.
- 🔵 **Testability/Clarity/Dependency** (minors) — **Addressed.** AC3 rem/em
  coverage note; AC5 backstop tightened to mirror 0075 AC4a; scope phrasing
  standardised to "current-app CSS under `src/`"; AC4 gate scope made explicit;
  gating research given a trackable `meta/research/codebase/…` path; `Blocks:
  none` note added; AC5 cleanup scope tied to the research.

### Assessment

The finding count converged across the three passes: 7 majors → 2 majors →
(1 genuine defect + 2 structural). The genuine defect (AC8/Naming-note
contradiction) is fixed. The two remaining testability majors are not defects to
eliminate — they are the lens repeatably observing that a work item which
**defers its inventory to research** cannot present an enumerable pass set for
"every value" until that research lands. That deferral was a deliberate, recorded
decision, so the criteria have been reframed to make the deferral explicit and to
give a concrete join condition once the inventory exists — the correct terminal
state, not a deficiency. Re-running the lenses again would predictably re-raise
the same tension: a treadmill, not a signal.

Human-assessed verdict: **COMMENT** — the work item is well-formed and
planning-ready. The remaining work belongs to the pre-planning codebase research,
which the work item now tracks explicitly.

### Final Decision — 2026-06-02

Author (Toby Clemson) **APPROVED** the work item and transitioned it to
`status: ready`. The two residual testability majors are accepted as the
by-design consequence of deferring the radius inventory to pre-planning
codebase research; the frontmatter `verdict` is set to APPROVE accordingly.

