---
type: work-item-review
id: "0110-surface-root-cause-analyses-in-visualiser-review-1"
title: "Work Item Review: Surface Root Cause Analyses in the Visualiser Under a New Operate Category"
date: "2026-06-13T09:54:13+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0110"
work_item_id: "0110"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: ["visualiser", "rca", "doc-types", "library"]
last_updated: "2026-06-13T10:35:09+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Surface Root Cause Analyses in the Visualiser Under a New Operate Category

**Verdict:** REVISE

This is a strong, well-structured story: every section is present and
substantively populated, scope is coherent and correctly bounded, and the
RCA / `issue-research` / `rca` naming triad is carefully reconciled against
prior work items. The single area needing work is **testability** — two major
findings flag that the "match the prototype" and "RCA type included and
covered" acceptance criteria lack defined pass conditions, and three minor
findings note unspecified expected values (count/preview), an unaddressed
prototype-visible RCA `status` field, and a missing zero-RCA empty state.
Tightening these criteria into checklist-style, observable assertions would
move this to APPROVE.

### Cross-Cutting Themes

- **Verification anchored to the prototype but without a concrete checklist**
  (flagged by: testability, completeness) — multiple criteria defer correctness
  to "match the prototype", yet the prototype carries concrete, assertable
  details (HSL hue 310, short label "RCA", count = 4, an RCA `status` field,
  category placement) that the criteria never enumerate. The fidelity bar is
  real but currently un-checkable, and the prototype-visible RCA `status` field
  is captured nowhere in requirements or criteria.
- **Soft, conditional reliance on 0096 not yet closed** (flagged by:
  dependency, completeness) — the work treats 0096's `rca` registration and the
  BigGlyph hero as "functionally done / will-exist", but both rest on
  unconfirmed state, so the true scope and readiness are slightly overstated.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: Design-fidelity criterion has no defined match tolerance
  **Location**: Acceptance Criteria (criterion 6)
  The "match in structure and visual treatment" criterion defines no threshold
  for what counts as a match, so a reviewer can argue it passes or fails for
  almost any implementation. Enumerate the specific elements that must match
  (hue, short label, category placement, shared list-view layout).

- 🟡 **Testability**: "RCA type included and covered" in the test-suite criterion is unbounded
  **Location**: Acceptance Criteria (criterion 7)
  "Covered" has no defined scope — a single trivial assertion, or no new test at
  all, would technically satisfy it. Replace with the specific tests expected
  (server emits Operate/RCA; frontend renders the card/listing; E2E navigates
  library → listing → detail).

#### Minor

- 🔵 **Dependency**: 0096 dependency is conditional on an un-updated status and unverified registration
  **Location**: Dependencies
  0096 is treated as a satisfied prerequisite, but its "satisfied" state rests
  on an unconfirmed assumption rather than a closed work item, overstating
  readiness to start. Note that confirming the `rca` registration is present is
  a precondition (or first step) of this work.

- 🔵 **Dependency**: 0093 (typed-linkage / stem mapping) relied on in body but only listed as Related
  **Location**: Dependencies
  Context and Technical Notes lean on 0093's stem/typed-linkage mapping, but it
  appears only under References > Related, not in the "Builds on" list. Promote
  it to Dependencies if its mapping must be in place; otherwise leave as Related.

- 🔵 **Testability**: Count and latest-preview have no expected value to assert against
  **Location**: Acceptance Criteria (criterion 1)
  "Correct document count and a latest-document preview" gives the verifier no
  derivation rule. State it explicitly (count = number of `issue-research`
  artifacts; preview = most recently modified RCA), mirroring 0041 semantics.

- 🔵 **Testability**: RCA status field shown in the prototype is not in any verifiable criterion
  **Location**: Requirements / Acceptance Criteria
  The prototype models each RCA with a `status` field (e.g. "resolved",
  "monitoring") in listing/detail rows, but no requirement or criterion mentions
  it. Add a criterion if in scope; note its exclusion if not.

- 🔵 **Testability**: No empty-state criterion for zero RCAs
  **Location**: Acceptance Criteria
  Every criterion presupposes a repo containing `issue-research` artifacts; the
  zero-RCA path (does the Operate card appear with count 0, or hide?) is
  gestured at in Open Questions but not resolved into a testable expectation.

#### Suggestions

- 🔵 **Clarity**: HSL acronym used without expansion
  **Location**: Context / Requirements / Technical Notes
  "HSL map" is anchored to 0074 but the acronym (hue/saturation/lightness) is
  never spelled out. Expand on first use.

- 🔵 **Clarity**: "Operate" category meaning left implicit relative to the phase model
  **Location**: Requirements (requirement 1)
  The item never states whether Operate is a sixth lifecycle phase or a
  differently-typed grouping that sits outside the DEFINE→REMEMBER progression.
  One clarifying sentence would resolve it.

- 🔵 **Clarity**: Bare code identifiers assume reader familiarity
  **Location**: Context / Technical Notes
  `STEM_TO_GLYPH`, `RelatedArtifacts`, "stem `rca`" appear without stating their
  kind (constant, component, map key) or location. Optionally annotate first use.

- 🔵 **Completeness**: BigGlyph hero existence left conditional rather than resolved
  **Location**: Assumptions
  The RCA BigGlyph hero "either exists or will be created", leaving its existence
  unresolved. Confirm from the prototype and state the outcome so the deliverable
  scope is fully captured.

- 🔵 **Scope**: Requirement 7 (design fidelity) restates scope already covered by requirements 3 and 4
  **Location**: Requirements (requirement 7)
  Requirement 7 re-asserts the "matching the prototype" obligation already in
  requirements 3 and 4 without adding a distinct unit of work. Fold it in or
  remove it.

### Strengths

- ✅ RCA is expanded on first use, and the `rca` / `issue-research` / RCA
  equivalence — a genuine source of confusion — is explicitly reconciled in
  Technical Notes and Drafting Notes.
- ✅ Frontmatter is complete and valid (kind=story, status=draft, priority=high,
  plus id, title, author, dates, schema_version).
- ✅ Summary is in proper user-story form, naming both the actor and the
  motivation, followed by a concise problem statement.
- ✅ Acceptance Criteria has seven Given/When/Then bullets covering library,
  listing, detail, related-artifacts, search, design parity, and test passage.
- ✅ Every internal upstream blocker implied by Context/Requirements is
  explicitly enumerated in Dependencies, and "No external dependencies" is
  accurate since all couplings are internal.
- ✅ Scope is coherent and well-bounded: all seven requirements serve the single
  purpose of surfacing the RCA doc type, and the Operate category + RCA type are
  correctly kept in one story (RCA is the category's only member).
- ✅ Nearly every piece of domain jargon is anchored to a specific prior work
  item ID, giving the reader a definition trail.
- ✅ The References section points to a design prototype directory that exists on
  disk, so the design-fidelity criteria are anchored to a real artefact.

### Recommended Changes

1. **Tighten the design-fidelity criterion into a checklist** (addresses:
   "Design-fidelity criterion has no defined match tolerance", "RCA status field
   shown in the prototype is not in any verifiable criterion") — Replace
   criterion 6's "match in structure and visual treatment" with an enumerated
   list of assertable elements pulled from the prototype: RCA card hue (HSL 310),
   short label "RCA", Operate category placement/ordering, shared list-view
   layout, BigGlyph hero, and the RCA `status` field rendering (or an explicit
   note that status is out of scope).

2. **Make the test-suite criterion specify the expected tests** (addresses:
   "'RCA type included and covered' is unbounded") — Replace "included and
   covered" in criterion 7 with the concrete tests expected: a server test
   asserting Operate/RCA emission, a frontend unit test rendering the RCA
   card/listing, and an E2E test navigating library → RCA listing → RCA detail.

3. **State the count/preview derivation rule** (addresses: "Count and
   latest-preview have no expected value to assert against") — In criterion 1,
   define "correct" as count = number of `issue-research` artifacts and preview =
   most recently modified RCA, mirroring 0041 card semantics. This also resolves
   the Open Question.

4. **Resolve the zero-RCA empty state** (addresses: "No empty-state criterion for
   zero RCAs") — Add a criterion fixing the observable outcome when no
   `issue-research` artifacts exist, consistent with other zero-count doc-type
   cards.

5. **Make the 0096/BigGlyph preconditions explicit** (addresses: "0096 dependency
   is conditional…", "BigGlyph hero existence left conditional") — Note in
   Dependencies that confirming 0096's `rca` registration is present is a
   precondition/first step, and resolve whether the RCA BigGlyph already exists
   or must be created.

6. **Minor tidy-ups** (addresses: "0093 only listed as Related", "Requirement 7
   restates scope", "HSL acronym", "Operate meaning", "bare identifiers") —
   Promote 0093 to Dependencies if it is a true prerequisite; fold requirement 7
   into requirements 3/4; expand HSL on first use; add one sentence on what
   "Operate" is relative to the phase model.

## Per-Lens Results

### Clarity

**Summary**: A clear, internally consistent work item: the RCA acronym is
expanded on first use, the rca/issue-research/RCA naming triad is explicitly
reconciled in Technical Notes, and the server-driven scope is stated identically
across Context, Requirements, Assumptions, and Acceptance Criteria. Most domain
jargon is anchored to prior work items by ID. The only clarity concerns are
minor: an undefined HSL acronym, a couple of opaque code-identifier references,
and one place where the meaning of "Operate" as a phase concept is left implicit.

**Strengths**:
- RCA is expanded on first use, and the rca / issue-research / RCA equivalence is
  explicitly reconciled in both Technical Notes and Drafting Notes.
- Scope is consistent across sections with no contradictions.
- Nearly every piece of domain jargon is anchored to a specific prior work item
  ID, giving the reader a definition trail.

**Findings**:
- 🔵 suggestion (high) — **HSL acronym used without expansion** (Context): "HSL
  map" / "HSL hue map" is used without expansion (HSL = hue/saturation/lightness)
  although anchored to 0074. Expand on first use.
- 🔵 suggestion (medium) — **"Operate" category meaning left implicit relative to
  the phase model** (Requirements): the item never states whether Operate is a
  sixth lifecycle phase or a differently-typed grouping. Add one clarifying
  sentence.
- 🔵 suggestion (low) — **Bare code identifiers assume reader familiarity**
  (Context): `STEM_TO_GLYPH`, `RelatedArtifacts`, "stem `rca`", "registration
  maps" appear without stating where they live or what kind of artefact they are.

### Completeness

**Summary**: A thoroughly complete story. Every expected section is present and
substantively populated, the frontmatter is intact with a recognised
kind/status/priority, and the kind-specific content a story demands — a
user-framed need, motivating context, and verifiable done-criteria — is all
present. The referenced design prototype directory is accessible. No
completeness gaps of consequence were found.

**Strengths**:
- Frontmatter is complete and valid (kind=story, status=draft, priority=high).
- Summary is in proper user-story form naming actor and motivation.
- Context explains why the work is needed and grounds it in prior work items.
- Acceptance Criteria has seven Given/When/Then bullets, well above the floor.
- Requirements are itemised and distinct from acceptance criteria; optional
  sections are genuinely populated.
- The References section points to an accessible design prototype directory.

**Findings**:
- 🔵 suggestion (low) — **BigGlyph hero existence left conditional rather than
  resolved** (Assumptions): Requirement 2 and AC3 depend on an RCA BigGlyph hero,
  but Assumptions states it "either exists or will be created", leaving its
  existence unresolved within the work item.

### Dependency

**Summary**: A strong, explicit Dependencies section captures every internal
upstream blocker (0041, 0074, 0054, 0057, 0082, 0096) the body implies, and
correctly states there are no external dependencies. The main gap is the soft,
status-conditional reliance on 0096; a secondary gap is that 0093 appears only as
"Related" rather than in Dependencies. No downstream consumers are implied, so
the empty Blocks set is appropriate.

**Strengths**:
- Every "Builds on" coupling implied by Context and Requirements is explicitly
  enumerated in Dependencies.
- The "No external dependencies" statement is accurate and well-judged.
- The conditional nature of the 0096 dependency is surfaced in both Context and
  Dependencies rather than left implicit.

**Findings**:
- 🔵 minor (high) — **0096 dependency is conditional on an un-updated status and
  unverified registration** (Dependencies): 0096 is treated as satisfied but rests
  on an unconfirmed assumption; Requirement 2's scope silently expands if its
  registration is incomplete. Note confirming it is a precondition.
- 🔵 minor (medium) — **0093 (typed-linkage / stem mapping) relied on in body but
  only listed as Related** (Dependencies): the typed-linkage mapping the
  related-artifacts integration depends on is not visible as a prerequisite. Add
  to "Builds on" if required; leave as Related if purely informational.

### Scope

**Summary**: One coherent, well-bounded unit of work: making the RCA doc type a
first-class, navigable type in the visualiser, end-to-end. All seven requirements
serve that single purpose by mirroring the established per-doc-type pattern. The
work spans server and frontend but both live in the single visualiser component
under one owning team, so this is intra-component breadth, not a cross-service
scope problem. Sizing is appropriate for a story and boundaries are explicit.

**Strengths**:
- All seven requirements serve a single unified purpose rather than bundling
  independent concerns.
- The Operate category and RCA doc type are correctly kept in one story (RCA is
  the category's only member).
- Scope boundaries are stated explicitly in Assumptions ("does not require
  reconciling unrelated drift").
- Summary, Requirements, and Acceptance Criteria describe the same scope.
- Dependencies are framed as build-ons this story consumes, so it is
  independently deliverable.

**Findings**:
- 🔵 suggestion (medium) — **Requirement 7 (design fidelity) restates scope
  already covered by requirements 3 and 4** (Requirements): Requirement 7
  re-asserts the same fidelity bar already attached to requirements 3 and 4 without
  adding a distinct unit of work. Fold it in or remove it.

### Testability

**Summary**: Most Acceptance Criteria are framed as observable Given/When/Then
behaviours with concrete triggers and outcomes. The principal weaknesses are the
two "match the prototype" criteria, whose pass condition has no defined
tolerance, and the final test-suite criterion, whose phrase "covered" is
unbounded. The criteria also omit a measurable expected value for the document
count and latest-preview, which the prototype actually specifies concretely.

**Strengths**:
- Five of seven Acceptance Criteria use explicit Given/When/Then framing with a
  concrete trigger and observable outcome.
- Criteria anchor verification to a single named artifact set ("issue-research"
  documents).
- Open Questions explicitly flags the count/latest-preview semantics as
  provisional.

**Findings**:
- 🟡 major (high) — **Design-fidelity criterion has no defined match tolerance**
  (Acceptance Criteria, criterion 6): "match in structure and visual treatment"
  defines no threshold; a reviewer can argue pass or fail for almost any
  implementation. Enumerate the specific structural/visual elements that must
  match (hue 310, short label "RCA", placement, shared list-view layout).
- 🟡 major (medium) — **"RCA type included and covered" in the test-suite
  criterion is unbounded** (Acceptance Criteria, criterion 7): "covered" has no
  defined scope; a single trivial assertion would satisfy it. Replace with the
  specific tests expected (server emits Operate/RCA; frontend renders
  card/listing; E2E navigates library → listing → detail).
- 🔵 minor (medium) — **Count and latest-preview have no expected value to assert
  against** (Acceptance Criteria, criterion 1): "correct" is defined only by
  implication; the prototype shows count = 4 and a most-recent preview. State the
  derivation rule.
- 🔵 minor (medium) — **RCA status field shown in the prototype is not in any
  verifiable criterion** (Requirements): the prototype models each RCA with a
  status field ("resolved", "monitoring") but no criterion mentions it. Add a
  criterion if in scope; note exclusion if not.
- 🔵 minor (low) — **No empty-state criterion for zero RCAs** (Acceptance
  Criteria): every criterion presupposes a repo containing issue-research
  artifacts; the zero-RCA path is gestured at in Open Questions but not resolved
  into a testable expectation.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-13T10:15:34+00:00

**Verdict:** REVISE

The revisions resolved every original finding — all 12 are confirmed fixed.
Two **new** major testability findings remain, both residuals of the same
fix: a couple of acceptance criteria still trail subjective "matching the
prototype" phrasing even though the dedicated parity criterion now enumerates
the checks. These are trivial wording tightenings. A handful of new minor
findings (a `relates_to` frontmatter/prose mismatch, missing test-fixture
anchoring, an un-promoted label flag) round out the delta.

### Previously Identified Issues
- 🟡 **Testability**: Design-fidelity criterion has no defined match tolerance — **Resolved** (AC now enumerates hue 310, labels, placement, layout, hero)
- 🟡 **Testability**: "RCA type included and covered" unbounded — **Resolved** (final AC names server/unit/E2E tests)
- 🔵 **Testability**: Count and latest-preview have no expected value — **Resolved** (count = #issue-research artifacts; preview = most recent)
- 🔵 **Testability**: RCA status field not in any criterion — **Resolved** (added to Reqs 3/6 and listing AC)
- 🔵 **Testability**: No empty-state criterion for zero RCAs — **Resolved** (zero-count AC added)
- 🔵 **Dependency**: 0096 dependency conditional/unverified — **Resolved** (explicit Precondition with verify-first step + fallback)
- 🔵 **Dependency**: 0093 only listed as Related — **Resolved** (promoted to Dependencies "Builds on")
- 🔵 **Scope**: Requirement 7 restates scope of 3 & 4 — **Resolved** (folded into Reqs 3/4, standalone removed)
- 🔵 **Completeness**: BigGlyph hero existence left conditional — **Resolved** (now in scope, create-if-missing)
- 🔵 **Clarity**: HSL acronym unexpanded — **Resolved** (expanded in Req 2 and Technical Notes)
- 🔵 **Clarity**: "Operate" meaning implicit — **Resolved** (clarified as peer grouping between Ship and Remember)
- 🔵 **Clarity**: Bare code identifiers — **Resolved** (annotated with kind in Technical Notes)

### New Issues Introduced
- 🟡 **Testability**: Detail-page criterion keeps a trailing "matching the prototype design" clause that is open-ended now that glyph/hue/hero are individually enumerated — drop the clause or reference the parity criterion.
- 🟡 **Testability**: Parity criterion's "uses the shared list-view layout" / "compared against the authoritative prototype" framing has no defined comparison method — restate as a structural check ("reuses the same list-view component as existing doc types") or defer to the named E2E/visual-regression coverage.
- 🔵 **Dependency**: `relates_to` frontmatter (0041, 0074, 0054, 0096) is narrower than the Dependencies prose — add `work-item:0057`, `work-item:0082`, `work-item:0093`.
- 🔵 **Dependency**: No downstream consumers captured — add "Blocks: none known" or name dependents.
- 🔵 **Dependency / Testability**: Non-empty listing/detail/count/preview/search/related-artifacts criteria are not anchored to a named test fixture (RCA artifacts + a linking artifact), so pass conditions depend on ambient repo state — note the fixture in Assumptions/Dependencies.
- 🔵 **Completeness**: Drafting Notes still flags the "Root cause analysis" label phrasing as an open question, but Open Questions shows only the resolved entry — reconcile the two.
- 🔵 **Clarity**: RCA = `issue-research` equivalence is stated only in Technical Notes, after both terms are used; the word "status" is overloaded (work-item status vs RCA frontmatter `status`).
- 🔵 **Scope**: Operate category is latently separable from RCA surfacing (acknowledged as a conscious, defensible coupling — no split needed).

### Assessment
The work item improved substantially — the original blocking testability gaps
are gone. It still carries two major findings (the configured REVISE
threshold), but both are one-line wording residuals of the parity work rather
than structural problems. Applying those two tightenings, the high-confidence
`relates_to` frontmatter fix, and a test-fixture note would clear the bar for
APPROVE.

## Re-Review (Pass 3) — 2026-06-13T10:35:09+00:00

**Verdict:** APPROVE

Final verification pass on testability and dependency (the two lenses carrying
pass-2 majors and high-confidence minors). Both pass-2 major testability
findings and all three pass-2 dependency minors are confirmed resolved.
Dependency returned zero findings. Testability returned three new minor
findings — all fixture-determinism polish that is naturally pinned down when
test fixtures are authored, none blocking implementation.

### Previously Identified Issues
- 🟡 **Testability**: Detail-page criterion's open-ended "matching the prototype design" clause — **Resolved** (clause dropped; glyph/hue/hero enumerated)
- 🟡 **Testability**: Parity criterion's undefined layout-comparison method — **Resolved** (discrete properties enumerated; broader visual parity delegated to named E2E/visual-regression coverage)
- 🔵 **Dependency**: `relates_to` narrower than prose — **Resolved** (0057, 0082, 0093 added to frontmatter)
- 🔵 **Dependency**: No downstream consumers captured — **Resolved** ("Blocks: none known" added)
- 🔵 **Dependency**: Runtime `issue-research` fixture coupling uncaptured — **Resolved** (captured in Assumptions, non-empty + empty cases)

### New Issues Introduced
- 🔵 **Testability** (minor): Visual-parity deferral points at "E2E/visual-regression coverage", but the final criterion names only an E2E navigation test — no visual-regression snapshot or baseline source is specified.
- 🔵 **Testability** (minor): The fixture in Assumptions has no fixed cardinality (how many RCAs, which statuses) and is not referenced from the criteria it backs, so the count/status assertions are self-referential.
- 🔵 **Testability** (minor): The search criterion names no concrete query string or expected matching fixture RCA.

### Assessment
Both blocking majors are resolved and the dependency surface is fully clean.
The work item is ready for implementation. The three residual testability
minors are fixture-pinning details (exact RCA count, status mix, visual-
regression baseline, search query) that are naturally settled while authoring
the test fixtures named in the final acceptance criterion; they do not block
planning or implementation. Verdict raised to APPROVE.
