---
date: "2026-06-05T21:38:10Z"
type: work-item-review
producer: review-work-item
target: "work-item:0079"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
id: "0079-aside-region-redesign-review-1"
title: "0079-aside-region-redesign-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-06-05T21:38:10Z"
last_updated_by: Toby Clemson
---

## Work Item Review: Detail-Page Aside Region Redesign

**Verdict:** REVISE

The work item is structurally complete, densely populated, and carves a coherent
detail-page aside slice out of a much larger design-gap analysis — Option B is
defined once and reused consistently, the eyebrow-rule unification names exact CSS
rules and token values, and the 0040 blocker is captured with full rationale. The
blocking concerns are all about precision rather than structure: the handling of
**inferred** relations after the `Same lifecycle` group collapses is left ambiguous
(flagged independently by clarity and testability), and three acceptance criteria
are not yet definitively verifiable — an unbounded "every detail-page route" clause,
a missing inferred-tag/section-ordering check, and an eyebrow-equality criterion that
asserts "same rule" without enumerating the properties to compare. Four major findings
across two lenses cross the REVISE threshold.

### Cross-Cutting Themes

- **Inferred-relation handling is under-specified** (flagged by: clarity, testability) —
  Context replaces the `Same lifecycle` group, but Assumptions reintroduces
  "same-lifecycle rows", and there is no acceptance criterion for the inferred case.
  An implementer cannot tell whether inferred relations remain as faint-tagged rows,
  are subsumed by the Cluster block, or both — and a verifier has nothing to assert.
- **Reconciliation with the cited source's `Declared links` block** (flagged by:
  clarity, testability) — The referenced gap doc describes the prototype aside as four
  sections including a separate `Declared links` block; Option B folds it into the
  single group. The work item never states this departure, so a reader/verifier using
  the prototype as the oracle could read the spec two ways.
- **Rail-label restyle crosses the 0040 ownership boundary** (flagged by: dependency,
  scope) — The eyebrow unification reaches into Pipeline/PipelineMini rail labels owned
  by in-progress 0040. The coupling is captured as a blocker, but its impact on this
  story's independent deliverability/verification deserves to be explicit.

### Findings

#### Major
- 🟡 **Clarity**: How 'inferred' rows are produced after the Same lifecycle group collapses is unspecified
  **Location**: Requirements
  Context/Requirements never say what becomes of inferred `Same lifecycle` relations
  once the group collapses, while Assumptions refers to "same-lifecycle rows" as a
  defined element. Two different asides both satisfy the criteria as written.

- 🟡 **Testability**: "on every detail-page route" is unbounded with no enumerated route set
  **Location**: Acceptance Criteria
  The first criterion requires Option B "on every detail-page route" and the eyebrow
  criterion "across the detail page and the lifecycle views", but neither enumerates
  the route/doc-kind set. A tester cannot bound "every" — coverage could be claimed
  after one kind.

- 🟡 **Testability**: Inferred-tag and section ordering are not given verifiable outcomes
  **Location**: Acceptance Criteria
  There is no criterion for the inferred case (faint `(inferred)` tag in the single
  group), and section ordering is asserted as a set rather than an ordered sequence.
  A mis-tagged inferred row or reordered sections could still pass every criterion.

- 🟡 **Testability**: Eyebrow-equality criterion asserts "same rule" without enumerating the properties to compare
  **Location**: Acceptance Criteria
  "Same eyebrow rule … or a shared token of identical values" forces a computed-value
  comparison but does not enumerate which properties must match (font-family, size,
  letter-spacing, text-transform, colour — and is weight/line-height included?).

#### Minor
- 🔵 **Completeness**: Story does not identify the user or system whose need is being met
  **Location**: Context
  Framed entirely as design reconciliation; the source gap doc's user-facing rationale
  (a one-click path into the lifecycle pipeline) is not captured.

- 🔵 **Clarity**: Work item's Option B omits the 'Declared links' block named in the referenced source without noting the difference
  **Location**: Context
  Cross-checking against the cited source surfaces two different descriptions of "the
  prototype's structure"; the deliberate departure is not stated.

- 🔵 **Clarity**: 'declared target' references an undefined frontmatter concept
  **Location**: Acceptance Criteria
  The trigger for the declared (accent) tag — a `target` frontmatter key (`fm.target`
  in the source) — is left to reader inference within the work item.

- 🔵 **Dependency**: Concurrent-edit coupling with 0074/0075 on the shared eyebrow rule is asserted away rather than coordinated
  **Location**: Dependencies
  Both this work and 0074/0075 touch the `Page.module.css .eyebrow` surface; "no
  overlap" dismisses rather than schedules the merge-conflict risk if 0074/0075 are
  in flight.

- 🔵 **Dependency**: No downstream consumers (Blocks) captured for the newly canonicalised eyebrow rule
  **Location**: Dependencies
  This work establishes the canonical eyebrow rule but lists no Blocks entries, so
  future label-styling work has no visible signal to converge on it.

- 🔵 **Dependency**: Cluster-data availability (matching-cluster lookup) is an implied prerequisite not stated as a coupling
  **Location**: Requirements
  The Cluster block's `<n> artifacts · <updated>` metadata presumes the cluster-lookup
  helper exposes that data; whether it exists or is part of 0040's in-progress work is
  not stated.

- 🔵 **Scope**: Eyebrow unification reaches into Pipeline rail labels owned by in-progress 0040
  **Location**: Requirements
  One of three requirements mutates components another in-flight story owns, so the
  story cannot be fully verified as a single self-contained increment while 0040 is
  in flight.

- 🔵 **Testability**: Legend/border-removal criterion lacks a stated verification scope
  **Location**: Acceptance Criteria
  "The legend and the 2px solid / 2px dashed border treatment are removed" reads as a
  global negative with no component/route anchor.

#### Suggestions
- 🔵 **Scope**: Three loosely-coupled concerns bundled under one aside redesign
  **Location**: Requirements
  Option B flatten, the Cluster block, and the typography unification are independent;
  acceptable as one story, but if 0040 stalls the Cluster block, consider splitting it
  from the unblocked aside/typography work.

- 🔵 **Clarity**: Final criterion's 'or a shared token of identical values' admits two distinct end states
  **Location**: Acceptance Criteria
  The "or" permits two structurally different implementations without saying whether
  the choice is open or one is preferred.

### Strengths
- ✅ Option B is defined once in Context and referenced consistently by name across
  Requirements, Acceptance Criteria, and Open Questions — no drift in meaning.
- ✅ Every expected section is present and substantively populated; even Open Questions
  is explicitly resolved rather than left blank.
- ✅ The eyebrow-rule unification names the exact canonical rule (`Page.module.css
  .eyebrow`) with concrete values, so "unify the typography" has a single referent.
- ✅ Acceptance Criteria use Given/When/Then phrasing with named actors and observable
  render states, including the negative case (no Cluster block when no cluster matches).
- ✅ The 0040 blocker is captured with full bidirectional rationale (cluster routes +
  rail-label components), and the abandoned 0043 spike is correctly handled as a
  non-blocker with its scope question resolved here.
- ✅ Drafting Notes explicitly flags and corrects the stale "Inter 12 / 400" rail-label
  claim, removing a latent contradiction.

### Recommended Changes

1. **Specify what happens to inferred / same-lifecycle relations under Option B**
   (addresses: clarity "How 'inferred' rows are produced…", testability "Inferred-tag
   and section ordering…") — In Context or Requirements, state explicitly whether
   inferred same-lifecycle relations remain as individual faint `(inferred)`-tagged
   rows inside `Related artifacts`, are represented solely by the Cluster block, or
   both. Reconcile the Assumptions section's "same-lifecycle rows" wording with that
   decision so the two resolve to one element.

2. **Add a Given/When/Then criterion for the inferred case and state section order**
   (addresses: testability "Inferred-tag and section ordering…") — Mirror the existing
   declared-tag criterion: given a same-lifecycle relation exists, the row appears in
   the single group with a faint `(inferred)` tag. State the required DOM order
   explicitly (Related artifacts → File → Cluster).

3. **Bound "every detail-page route" to an enumerated set** (addresses: testability
   "'on every detail-page route' is unbounded") — Replace "every detail-page route"
   and "the lifecycle views" with an enumerated list of detail-page doc kinds, or
   reference the canonical type list (e.g. `LIBRARY_INDEX`) as the authoritative,
   finite verification population.

4. **Enumerate the properties the eyebrow-equality check compares** (addresses:
   testability "Eyebrow-equality criterion…", clarity "'or a shared token of identical
   values'…") — Make the final criterion list the exact resolved properties that must
   be equal across all three labels (computed font-family, font-size 11px,
   letter-spacing 0.12em, text-transform uppercase, colour `--ac-fg-faint`), and state
   whether the shared-rule vs shared-token implementations are both acceptable.

5. **Note the deliberate departure from the source's `Declared links` block**
   (addresses: clarity "Work item's Option B omits the 'Declared links' block…",
   testability "Source gap doc describes a separate 'Declared links' block…") — Add one
   line to Context or Drafting Notes stating Option B intentionally folds the source's
   separate `Declared links` block into the single `Related artifacts` group via the
   declared accent tag, so the work item and its cited oracle reconcile.

6. **Anchor the legend/border-removal criterion to its component** (addresses:
   testability "Legend/border-removal criterion lacks a stated verification scope") —
   Rephrase as: on the detail-page aside (`RelatedArtifacts`), no legend element
   renders and no row carries a 2px solid or 2px dashed border.

7. **Define 'declared `target`' on first use** (addresses: clarity "'declared target'
   references an undefined frontmatter concept") — Note that a declared `target` is the
   document's `target` frontmatter key, so the accent-tag trigger is unambiguous within
   the work item.

8. **Tighten the dependency couplings** (addresses: dependency findings) — Confirm
   whether 0074/0075 are complete or in-flight (and add a land-order note if the
   latter); state whether the cluster title / artifact-count / updated metadata already
   exists in the helper or comes from 0040; and add a Blocks entry (or an explicit
   "no downstream consumer" note) for the canonical eyebrow rule.

9. **(Optional) Add the user-facing rationale to Context** (addresses: completeness
   "Story does not identify the user…") — One sentence naming the beneficiary and need
   (a dedicated affordance to navigate from a document into its lifecycle cluster),
   which the source gap analysis already articulates.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear and internally consistent: Option B is
defined once in Context and reused consistently, the eyebrow-rule unification is
described precisely with concrete CSS rules and token values, and actors/outcomes in
the Acceptance Criteria are mostly stated as observable render states. The main clarity
risks are an ambiguous treatment of 'inferred' relations (the Context/Requirements never
explain how inferred rows are produced after the `Same lifecycle` group collapses, while
the Assumptions section introduces 'same-lifecycle rows' as a distinct concept) and a
couple of undefined or assumed-known terms (`fm.target`-style declared tags, the
relationship between the collapsed group and the prototype's separate `Declared links`
block named in the source).

**Strengths**:
- Option B is defined once and explicitly in Context, then referenced consistently by
  name across Requirements, Acceptance Criteria, and Open Questions — no drift.
- The eyebrow-rule unification names the exact canonical rule (`Page.module.css
  .eyebrow`) with concrete values, so "unify the typography" has a single referent.
- Acceptance Criteria use Given/When/Then phrasing naming the triggering actor and
  stating outcomes as observable render states.
- Drafting Notes explicitly flags and corrects a stale prior claim (rail label being
  "Inter 12 / 400").

**Findings**:
- 🟡 major / medium confidence — **How 'inferred' rows are produced after the Same
  lifecycle group collapses is unspecified** (Requirements). Context replaces the `Same
  lifecycle` group; Assumptions reintroduces "same-lifecycle rows" as if defined. A
  reader cannot tell whether inferred same-lifecycle artifacts appear as individual
  faint rows, are represented solely by the Cluster block, or both — two different
  asides both satisfy the criteria.
- 🔵 minor / medium confidence — **Work item's Option B omits the 'Declared links' block
  named in the referenced source without noting the difference** (Context). The source
  describes a four-section aside including a separate `Declared links` block; the work
  item folds it into the single group with no statement of the deliberate departure.
- 🔵 minor / medium confidence — **'declared target' references an undefined frontmatter
  concept** (Acceptance Criteria). The trigger for the declared accent tag (a `target`
  frontmatter key, `fm.target` in the source) is left to reader inference.
- 🔵 suggestion / low confidence — **Final criterion's 'or a shared token of identical
  values' admits two distinct end states** (Acceptance Criteria). The "or" permits two
  structurally different implementations without saying whether the choice is open.

### Completeness

**Summary**: This story is structurally complete and densely populated: every expected
section is present and carries substantive content. Context explains the motivation,
Requirements map one-to-one to Acceptance Criteria, and frontmatter is well-formed for
the unified schema. The only completeness gap is the absence of an explicit
identification of the user/system whose need is being met, which the story kind
nominally expects.

**Strengths**:
- Every expected section is present and substantively populated — no empty or
  placeholder-only sections; even Open Questions is explicitly resolved.
- Context explains the why rather than restating the Summary.
- Six specific, Given/When/Then-structured acceptance criteria, each tracing to a
  Requirement, including a negative case.
- Frontmatter complete and recognised (kind=story, status=draft, all unified-schema
  fields present and consistent).
- Dependencies, Assumptions, Technical Notes, and References give concrete file paths
  and coordination points.

**Findings**:
- 🔵 minor / medium confidence — **Story does not identify the user or system whose need
  is being met** (Context). The work is framed entirely as design reconciliation; the
  source gap doc's user-facing rationale (a one-click path into the lifecycle pipeline)
  is not captured, so a reader cannot judge the relative value of the three bundled
  changes.

### Dependency

**Summary**: Dependency capture is strong for a story of this size: the principal
upstream blocker (0040, which owns both the lifecycle cluster routes the new Cluster
block links to and the Pipeline rail-label components this work restyles) is explicitly
named with its coordination implications spelled out, and the abandoned 0043 spike is
correctly noted as non-blocking. The main gaps are a potential concurrent-edit coupling
with 0074/0075 on the shared eyebrow rule and the absence of any captured
Blocks/downstream-consumer relationship.

**Strengths**:
- The 0040 blocker is captured with full bidirectional rationale (cluster routes +
  rail-label components, with an explicit coordinate instruction).
- The abandoned 0043 spike is correctly handled as a non-blocker with its scope question
  resolved here.
- The Cluster block's dependency on existing app wiring (`cluster-via-label.ts`,
  `LifecycleClusterView`) is identified as net-new wiring rather than a cross-item
  blocker.

**Findings**:
- 🔵 minor / medium confidence — **Concurrent-edit coupling with 0074/0075 on the shared
  eyebrow rule is asserted away rather than coordinated** (Dependencies). Both sides
  touch `Page.module.css .eyebrow`; "no overlap" dismisses rather than schedules the
  merge-conflict risk if 0074/0075 are in flight.
- 🔵 minor / medium confidence — **No downstream consumers (Blocks) captured for the
  newly canonicalised eyebrow rule** (Dependencies). The work establishes the canonical
  rule but lists no Blocks entries, so the convention could silently fork.
- 🔵 minor / low confidence — **Cluster-data availability (matching-cluster lookup) is
  an implied prerequisite not stated as a coupling** (Requirements). The `<n> artifacts
  · <updated>` metadata presumes the helper exposes that data; whether it exists or
  comes from 0040's in-progress work is not stated.

### Scope

**Summary**: Work item 0079 carves a coherent detail-page aside slice out of a much
larger design-gap analysis, and its three requirements are plausibly related under a
single "aside region redesign" theme. The main scope tension is that the typography
unification reaches into the Pipeline/PipelineMini rail labels owned by in-progress
story 0040 — the one place where the unit of delivery crosses an ownership boundary. The
story is appropriately sized for a story kind; it is not too large, but the rail-label
restyling is arguably separable.

**Strengths**:
- A clean, bounded slice extracted from a sprawling gap analysis — exactly the
  aside-region cluster of drifts and nothing more.
- Scope boundaries are explicit and consistent across Summary, Requirements, and
  Acceptance Criteria.
- In-scope and out-of-scope are clearly stated (excludes 0074/0075 icon/size work and
  the abandoned 0043 spike).
- Open Questions confirms both prior design decisions are resolved, so it is a settled
  unit of delivery.

**Findings**:
- 🔵 minor / medium confidence — **Eyebrow unification reaches into Pipeline rail labels
  owned by in-progress 0040** (Requirements). One of three requirements mutates
  components another story owns and is actively changing, so the story cannot be fully
  verified as a single self-contained increment while 0040 is in flight. Consider
  whether the rail-label promotion belongs in 0040.
- 🔵 suggestion / medium confidence — **Three loosely-coupled concerns bundled under one
  aside redesign** (Requirements). Option B flatten, the Cluster block, and the
  typography unification are technically independent; acceptable as one story, but if
  0040's route dependency proves slow, consider splitting the Cluster block from the
  unblocked work.

### Testability

**Summary**: The acceptance criteria are mostly strong: four of six use explicit
Given/When/Then framing with observable outcomes, and the eyebrow-unification criterion
names a concrete shared rule to verify against. The main testability gaps are an
unbounded "on every detail-page route" clause with no enumerated route set, an
under-specified ordering/visual-tag spec for the flattened group, and a typography
criterion that asserts value equality without enumerating the exact resolved properties
a verifier must compare.

**Strengths**:
- Four criteria use precise Given/When/Then framing with stated precondition, action,
  and observable outcome.
- The Cluster-block criterion specifies the exact navigation target (`/lifecycle/<slug>`)
  and metadata string format (`<n> artifacts · <updated>`).
- The negative case is explicitly covered (no matching cluster → no Cluster block).
- The eyebrow criterion names the canonical rule and its property values in Requirements.

**Findings**:
- 🔴/🟡 major / high confidence — **"on every detail-page route" is unbounded with no
  enumerated route set** (Acceptance Criteria). Neither the criteria nor Technical Notes
  enumerate the full route/doc-kind set; "every" is a real, sizable population a verifier
  cannot bound, so coverage could be claimed after checking one kind.
- 🟡 major / medium confidence — **Inferred-tag and section ordering are not given
  verifiable outcomes** (Acceptance Criteria). No criterion covers the inferred case
  (faint `(inferred)` tag in the single group), and section order is asserted as a set
  rather than an ordered sequence.
- 🟡 major / medium confidence — **Eyebrow-equality criterion asserts "same rule" without
  enumerating the properties to compare** (Acceptance Criteria). The same-rule-OR-token
  disjunction forces a computed-value comparison but does not enumerate which properties
  must match, so two reviewers could reach different verdicts on the same DOM.
- 🔵 minor / medium confidence — **Legend/border-removal criterion lacks a stated
  verification scope** (Acceptance Criteria). It reads as a global negative with no
  component/route anchor.
- 🔵 minor / low confidence — **Source gap doc describes a separate "Declared links"
  block that Option B's criteria do not reconcile** (Requirements). A tester using the
  prototype as the oracle may assert a `Declared links` section Option B intentionally
  drops, producing a false fail.

## Re-Review (Pass 2) — 2026-06-05T21:03:46Z

**Verdict:** COMMENT

All four major findings from pass 1 are resolved, and the completeness gap is closed.
No critical or major findings remain — the work item is acceptable for implementation.
The residual findings are minor polish and a recurring (but now de-risked) observation
that the eyebrow-typography unification is technically separable from the aside
redesign. The 0079→0040 coupling that drove the pass-1 scope/dependency concerns is
discharged now that 0040 is functionally complete (its status transition is the only
outstanding item, captured as a close-time check). Verified against the prototype
source (`.ac-related__tag.is-declared` in `prototype-standalone.html`) that the
single-list-with-accent-tag structure — not a separate `Declared links` element — is
the prototype's real shape, settling the cross-source ambiguity.

### Previously Identified Issues
- 🟡 **Clarity**: How 'inferred' rows are produced after the Same lifecycle group collapses is unspecified — **Resolved**. Context, Requirements, Assumptions, and a new acceptance criterion now state inferred same-lifecycle relations remain as faint `(inferred)`-tagged rows in the single group, with the Cluster block additive.
- 🟡 **Testability**: "on every detail-page route" is unbounded — **Resolved**. Bounded to the canonical `LIBRARY_INDEX` type set with an explicit fail condition. (Residual minor: enumerate that set inline for a self-contained matrix.)
- 🟡 **Testability**: Inferred-tag and section ordering not given verifiable outcomes — **Resolved**. Added an inferred-case Given/When/Then criterion and a fixed DOM order. (Residual suggestion: assert File-after-Related explicitly, not just Cluster-after-File.)
- 🟡 **Testability**: Eyebrow-equality criterion asserts "same rule" without enumerating properties — **Resolved**. The criterion now enumerates the five computed properties and states both implementations are acceptable.
- 🔵 **Completeness**: Story does not identify the user/system whose need is being met — **Resolved**. Context now states the one-click-into-lifecycle user value.
- 🔵 **Clarity**: Option B omits the 'Declared links' block named in the source without noting it — **Resolved**. Context adds a reconciliation note verified against the prototype markup.
- 🔵 **Clarity**: 'declared target' references an undefined frontmatter concept — **Resolved**. Defined as the document's `target` frontmatter key on first use.
- 🔵 **Clarity**: Final criterion's 'or a shared token' admits two end states — **Resolved**. Both implementations explicitly declared acceptable if the five resolved values match.
- 🔵 **Dependency**: Concurrent-edit coupling with 0074/0075 on the shared eyebrow rule — **Resolved** (with residual). Now carries an explicit land-first sequencing rule; pass-2 notes the coordination is one-directional.
- 🔵 **Dependency**: No downstream consumers (Blocks) captured — **Resolved**. A reasoned Blocks note now flags the canonical eyebrow rule for future consumers.
- 🔵 **Dependency**: Cluster-data availability is an implied prerequisite not stated — **Partially resolved**. The Dependencies note asserts the helper already exposes title/count/timestamp; pass-2 suggests promoting it to its own explicit prerequisite line.
- 🔵 **Scope**: Eyebrow unification reaches into Pipeline rail labels owned by in-progress 0040 — **Resolved as a blocker**. 0040 reframed as a satisfied prerequisite (functionally complete); persists only as a "separable concern" observation, not a coupling hazard.
- 🔵 **Testability**: Legend/border-removal criterion lacks a stated verification scope — **Resolved**. Anchored to the `RelatedArtifacts` component.
- 🔵 **Testability**: Source gap doc's separate "Declared links" block not reconciled — **Resolved**. Context states Option B supersedes the gap doc's prose, verified against the prototype.
- 🔵 **Scope**: Three loosely-coupled concerns bundled — **Acknowledged, unchanged**. Still acceptable as one story; surfaced again as a minor separability note.

### New Issues Introduced
- 🔵 **Clarity** (minor): Summary doesn't state the canonical section count (three: `Related artifacts`, `File`, `Cluster`); a reader arriving from the source's "four sections" must reach Context to reconcile.
- 🔵 **Clarity** (minor): The Cluster block's user value / current-app gap is now described in two near-identical Context passages (a by-product of adding the user-value paragraph) — worth merging.
- 🔵 **Dependency** (minor): The 0074/0075 sequencing and the cluster-helper data-availability are captured one-directionally / as asserted fact rather than as independently tracked prerequisites.
- 🔵 **Testability** (minor): Several preconditions ("matching lifecycle cluster", "a document with a declared `target`", the `LIBRARY_INDEX` set) name no concrete example fixture or inline enumeration, so a verifier must source qualifying inputs from the corpus.

### Assessment
The work item is ready for implementation. The pass-1 blockers are all addressed, and
the verdict moves REVISE → COMMENT. The remaining items are optional polish — tightening
self-contained verifiability (inline fixtures / enumerations), a Summary section-count,
and de-duplicating the Cluster paragraph — none of which block planning. If desired, the
testability refinements (naming example fixtures, inlining the `LIBRARY_INDEX` kinds)
would be the highest-value follow-ups, but they can equally be handled during planning.

## Approval — 2026-06-05T21:38:10Z

**Verdict:** APPROVE

The pass-2 polish items were subsequently applied to the work item: the Summary now
states the canonical three-section count; the duplicated Cluster-block paragraphs are
merged and the cluster match rule is stated; the first criterion references the real
`DOC_TYPE_KEYS` registry (correcting the non-existent `LIBRARY_INDEX`) with the twelve
physical detail-page doc types enumerated inline and a full section-order assertion; the
eyebrow-equality check is scoped to the three named elements; a verification-fixtures
note was added; and the cluster-helper data availability plus 0074/0075 and 0040
coordination notes were tightened. With those applied, all pass-1 and pass-2 findings are
resolved or reduced to acknowledged-acceptable observations. Verdict raised COMMENT →
APPROVE at the author's direction; the work item status is transitioned `draft` → `ready`.
