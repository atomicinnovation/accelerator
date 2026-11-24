---
type: "work-item-review"
id: "PP-994-generic-set-model-indexing-and-api-review-1"
title: "Work Item Review: Generic Set Model for Multi-Document Doc Types"
date: "2026-11-23T16:16:47+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-994"
work_item_id: "PP-994"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-11-24T00:06:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Generic Set Model for Multi-Document Doc Types

**Verdict:** REVISE

The item is one coherent change, members entering the index, with each index
consumer given its own rule and concrete, countable criteria. It fails at its
seam with the set page: it makes members searchable and addressable without
saying what a member's URL shows, and it requires members to be served
without a criterion for it. Both are what the two blocked items build on.

### Cross-Cutting Themes

- **What a member URL shows** (flagged by: clarity, scope, testability) — the
  Summary and Assumptions say nothing a user sees changes, while the
  requirements return members from search and give each a reloadable URL. No
  section says what renders there, so the boundary with the set page is
  undefined.
- **The served contract is unverified** (flagged by: testability,
  completeness, scope, dependency) — serving a set's root and members has a
  requirement, an open question on its shape, and no criterion. Several
  criteria assert index state with no observation point.
- **Decisions parked that set the size** (flagged by: scope, completeness,
  dependency) — redirect-or-break for old URLs, whether a member change bumps
  the set's latest time, and the unchecked nesting of doc roots.

### Findings

#### Critical

None.

#### Major

- 🟡 **Clarity, Scope, Testability**: "Changes nothing a user sees" conflicts
  with members becoming searchable and addressable
  **Location**: Summary, Assumptions, Acceptance Criteria
  The member-URL criterion says only that the member "is addressed". Whether a
  member URL renders the member, a placeholder or the root is undefined, so
  the route, search and related criteria can pass on a route that renders
  nothing useful, and neither this item nor the set page owns the member view.
- 🟡 **Testability**: Serving a set's members has no criterion, and index
  criteria have no observation point
  **Location**: Acceptance Criteria, Open Questions
  The contract the set page and PP-134 consume is the least verifiable part of
  the story. "The index holds", "indexed as asset members" and "marked
  malformed" name no place where a verifier looks.

#### Minor

- 🔵 **Testability**: "Show what they showed before" has no baseline
  **Location**: Acceptance Criteria
  The compared properties are not named, and topic research has no equivalent
  criterion.
- 🔵 **Testability, Clarity**: Activity criterion fixes an incidental time
  window
  **Location**: Acceptance Criteria
  "Within a minute" reads as a coalescing window; interleaved sets, root
  changes and asset changes are not covered.
- 🔵 **Testability**: Genericity is checked only by a name-absence check
  **Location**: Acceptance Criteria
  "Set handling" has no boundary, and the check omits consumers and routes.
- 🔵 **Testability, Completeness**: Several requirements have no criterion
  **Location**: Requirements
  Asset path, size and time; missing frontmatter; dot-prefixed files emitting
  no event; the presentation declaration; the latest-time question.
- 🔵 **Dependency**: Existing change-event subscribers are an unnamed consumer
  **Location**: Requirements: Live update
  Member events will reach the toast and query invalidation before the set
  page that governs them exists.
- 🔵 **Completeness, Dependency**: Unchecked doc-root nesting sits only in
  Technical Notes
  **Location**: Technical Notes
  It is a precondition of resolving a member's doc type by path.
- 🔵 **Dependency**: Blocks edges are prose-only and PP-134 still points at
  the parent
  **Location**: Dependencies
- 🔵 **Dependency**: Open route and API shapes are contracts the blocked items
  consume
  **Location**: Open Questions
  PP-134's own question on screenshot enumeration is largely settled here.
- 🔵 **Dependency**: ADR-0069 is referenced but the crate coupling is not
  stated
  **Location**: References
  The set declaration probably lives in `corpus`, a shared crate.
- 🔵 **Scope, Completeness**: Open questions leave the size undecided and
  carry no resolution point
  **Location**: Open Questions
- 🔵 **Scope**: Presentation declaration and member serving have no consumer
  in this story
  **Location**: Requirements: Model
- 🔵 **Scope**: Asset indexing and the design-inventory migration are a
  separable slice
  **Location**: Requirements: Design inventories
  Kept on purpose to prove the model on two types; the reason is not stated.
- 🔵 **Scope**: The wrong-set link defect is an existing bug bundled in
  **Location**: Requirements: Routes
- 🔵 **Clarity**: "Entry" means both an index record and a listed item
  **Location**: Requirements: Index consumers
- 🔵 **Clarity**: The presentation declaration has no stated effect, and
  topic research's choice is not named
  **Location**: Requirements: Model
- 🔵 **Clarity**: "Its page" and "the linkage" have no clear referent
  **Location**: Requirements: Index consumers
- 🔵 **Clarity**: "Rises by one" has no baseline or triggering action
  **Location**: Acceptance Criteria
- 🔵 **Clarity**: "A set holding … a sibling `.draft.tmp/` directory" is
  self-contradictory
  **Location**: Acceptance Criteria
- 🔵 **Clarity**: A set's "slug" and "every surface" are undefined
  **Location**: Requirements: Routes

#### Suggestions

- 🔵 **Testability**: Define what counts as one change for the live-update
  criterion
  **Location**: Acceptance Criteria
- 🔵 **Completeness, Clarity**: Add ADR-0067 and the sibling draft IDs to
  References; explain ADR-0069
  **Location**: References
- 🔵 **Completeness**: Add `relates_to` for PP-134 and PP-862
  **Location**: Frontmatter: relates_to
- 🔵 **Dependency**: Name a tracked consumer for work-item attachments, or
  say none exists
  **Location**: Context
- 🔵 **Dependency**: Name who accepts the ADR and when
  **Location**: Acceptance Criteria
- 🔵 **Scope**: Record the server/frontend seam as the fallback cut
  **Location**: Technical Notes

### Strengths

- ✅ Every requirement follows from one change, and Context explains why each
  consumer needs a root-versus-member rule.
- ✅ Criteria use countable fixtures and negative assertions ("by one, not
  eleven", "no row naming a member").
- ✅ The two-set link criterion reproduces the existing defect and bounds
  "every" by naming four surfaces.
- ✅ The boundary with PP-134 is stated the same way in four sections.
- ✅ Technical Notes map the consumers, call sites and pinned tests with file
  and line anchors, and the size has a rationale.

### Recommended Changes

1. **State what a member URL shows** (addresses: no-visible-change conflict)
   For example, the member rendered through the shared document view with no
   set navigation, and not-found for a missing member. Rewrite the Summary and
   Assumptions to list the visible changes: members in search, member pages,
   corrected set links.
2. **Add a criterion for the served contract** (addresses: serving has no
   criterion)
   Shape-independent: requesting a set's members returns the root and each
   member with its relative path, member kind and frontmatter state. Use the
   same response as the observation point for the index criteria.
3. **Settle the size-setting questions** (addresses: open questions)
   Decide redirect-or-break and the latest-time question now; move the
   doc-root nesting check to Open Questions or Assumptions.
4. **Say what member events do before the set page lands** (addresses:
   unnamed subscribers)
5. **Tighten the loose criteria** (addresses: baseline, time window,
   genericity, missing criteria, and the clarity minors)
   Name compared properties; drop "within a minute" and add an interleaved
   case; add a behavioural genericity criterion; give counts a baseline; fix
   the `.draft.tmp/` wording.
6. **Record the links** (addresses: blocks edges, references)
   Name PP-1001, repoint PP-134 at this item once it has a key, and state
   that the declaration lives in `corpus`.

## Per-Lens Results

### Clarity

**Summary**: Tightly written with example-driven criteria. The gap is what a
member's own URL shows.

**Strengths**: Imperative requirements grouped by component; concrete numbers
and filenames; the activity roll-up reading stated in Assumptions.

**Findings**:
- major, high, Assumptions — "Changes nothing a user sees" conflicts with
  members becoming searchable and addressable.
- minor, high, Requirements: Index consumers — "Entry" means both an index
  record and a listed library item.
- minor, high, Requirements: Model — The presentation declaration has no
  stated effect, and topic research's choice is not named.
- minor, medium, Requirements: Index consumers — "Its page" and "the linkage"
  have no clear referent.
- minor, high, Acceptance Criteria — "Rises by one" has no baseline or
  triggering action.
- minor, medium, Acceptance Criteria — "A set holding … a sibling
  `.draft.tmp/` directory" is self-contradictory.
- minor, medium, Requirements: Routes — A set's "slug" and "every surface"
  are undefined.
- suggestion, medium, Acceptance Criteria — "Within a minute" reads as a
  time-window rule.
- suggestion, medium, References — ADR-0069 unexplained; ADR-0067 cited but
  not referenced.

### Completeness

**Summary**: Structurally complete and dense. Gaps are at the edges.

**Strengths**: Beneficiary and payoff stated; Context explains forces;
grouped requirements; provenance in Drafting Notes.

**Findings**:
- minor, medium, Acceptance Criteria — Two requirements have no criterion
  defining done.
- minor, high, Technical Notes — Unverified risk recorded only in Technical
  Notes.
- minor, medium, Open Questions — Open questions carry no resolution point.
- suggestion, high, References — References omit documents the body relies
  on.
- suggestion, low, Frontmatter: relates_to — No `relates_to` despite related
  items in the body.

### Dependency

**Summary**: Both downstream consumers are named and no upstream blocker
exists. Several couplings sit only in notes.

**Strengths**: Both directions stated; PP-134 boundary drawn; asset rework
risk visible; blast radius mapped.

**Findings**:
- minor, high, Dependencies — Blocks edges are prose-only and PP-134 still
  points at the parent.
- minor, medium, Technical Notes — Unchecked doc-root nesting precondition is
  not tracked.
- minor, medium, Open Questions — Open route and API shapes are contracts the
  blocked items consume.
- minor, medium, Requirements: Live update — Existing change-event
  subscribers are an unnamed consumer.
- minor, medium, References — ADR-0069 is referenced but the cross-crate
  coupling is not stated.
- suggestion, medium, Context — Anticipated future consumer has no tracked
  reference.
- suggestion, low, Acceptance Criteria — ADR acceptance is an approval gate
  with no named approver.

### Scope

**Summary**: One coherent unit, large but hard to divide. Concerns are at the
edges.

**Strengths**: All groups follow from one change; explicit out-of-scope
boundaries; reasoned size; one team can deliver it.

**Findings**:
- major, medium, Summary — Summary says nothing visible changes, but
  requirements make members searchable and addressable.
- minor, medium, Requirements: Model — Presentation declaration and
  set-member serving have no consumer in this story.
- minor, medium, Requirements: Design inventories — Asset indexing and the
  migration are a separable slice with no end-to-end consumer.
- minor, medium, Requirements: Routes — The wrong-set link defect is an
  existing bug bundled into the model story.
- minor, medium, Open Questions — Open questions leave the story's size
  undecided.
- suggestion, low, Technical Notes — A backend/frontend seam exists if the
  story needs dividing.

### Testability

**Summary**: Most criteria are verifiable. The two unresolved seams are not.

**Strengths**: Countable fixtures; a criterion per consumer; two-set link
criterion; isolation checked for malformed members.

**Findings**:
- major, high, Acceptance Criteria — Member URL criterion has no observable
  outcome ("the same member is addressed").
- major, high, Open Questions — Serving a set's members has no criterion, and
  index criteria have no observation point.
- minor, high, Acceptance Criteria — "Show what they showed before" has no
  defined baseline.
- minor, medium, Acceptance Criteria — Activity roll-up criterion fixes an
  incidental time window.
- minor, medium, Acceptance Criteria — Genericity is verified only by a
  name-absence check with a fuzzy scope.
- minor, high, Requirements — Several stated requirements have no
  corresponding criterion.
- suggestion, medium, Acceptance Criteria — "One change event" lacks a
  defined trigger granularity.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-11-23

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Clarity, Scope, Testability**: "Changes nothing a user sees" conflicts with searchable, addressable members — Resolved for member documents (a member renders through `LibraryDocView`); still open for asset members
- 🟡 **Testability**: Serving a set's members has no criterion — Still present (the requirement was reworded; no criterion was added)
- 🔵 **Testability**: "Show what they showed before" has no baseline — Partially resolved (properties named; baseline still "before")
- 🔵 **Testability, Clarity**: Activity criterion's time window — Resolved
- 🔵 **Testability**: Genericity checked only by name absence — Still present, now rated major
- 🔵 **Testability, Completeness**: Requirements without criteria — Partially resolved (asset record, absent frontmatter, excluded files added; presentation declaration and latest time remain)
- 🔵 **Dependency**: Change-event subscribers unnamed — Resolved
- 🔵 **Completeness, Dependency**: Doc-root nesting only in Technical Notes — Resolved
- 🔵 **Dependency**: Blocks edges prose-only — Partially resolved (frontmatter set; prose omits PP-1000; PP-134 still points at the parent)
- 🔵 **Scope, Completeness**: Open questions set the size — Still present (redirect-or-break and latest time undecided)
- 🔵 **Clarity**: "Entry", count baseline, `.draft.tmp/`, slug and surfaces — Resolved

### New Issues Introduced
- 🟡 **Clarity, Testability**: "Every member" gets a URL, but an asset member's URL has no defined outcome; "no route returns an image's bytes" cannot be checked exhaustively.
- 🟡 **Clarity, Completeness, Dependency**: Body omits PP-1000, which the frontmatter says this story blocks.
- 🟡 **Scope**: A server-side model change and a frontend routing change sit in one story with a clean seam at the member API.
- 🔵 **Clarity**: "Set declaration" and the presentation declaration are not distinguished.
- 🔵 **Clarity**: "Existing URLs may change" does not say which URLs.
- 🔵 **Clarity**: The `.invalid` example reads as a name or a suffix.
- 🔵 **Clarity, Testability**: Related-items rule leaves inferred relations and the inbound direction undefined.
- 🔵 **Testability**: Negative event assertions have no bounded observation procedure.
- 🔵 **Testability**: The set-link criterion covers topic research only.
- 🔵 **Dependency**: On-disk conventions are not tied to the skills that produce them.
- 🔵 **Scope**: The ADR and the two shape decisions gate siblings but complete only with the whole story.
- 🔵 **Clarity, Completeness**: Context still says "part A of a three-way split".

### Assessment
The member view is settled and most first-pass minors are closed. The item
still needs the API criterion that was recommended in pass 1 and not added, a
decision on asset member URLs, and a behavioural genericity criterion. The
server/frontend seam is a scope decision for the author.

## Re-Review (Pass 3) — 2026-11-23

**Verdict:** REVISE

The item is now "Generic Set Model: Indexing and API", split at the member
API.

### Previously Identified Issues
- 🟡 **Testability**: Serving members has no criterion — Resolved
- 🟡 **Testability**: Genericity verified by source inspection — Partially resolved (behavioural criterion added; it bundles five outcomes behind an unprovable precondition)
- 🟡 **Clarity, Testability**: Asset member URL undefined — Resolved (no asset URL)
- 🟡 **Clarity, Completeness, Dependency**: Body omits PP-1000 — Partially resolved (prose corrected; `blocks` now omits two direct dependants)
- 🟡 **Scope**: Server and frontend in one story — Resolved (split)
- 🔵 **Scope, Completeness**: Open questions set the size — Resolved
- 🔵 **Clarity**: Two declarations; which URLs change; `.invalid` — Resolved
- 🔵 **Testability**: "As before" baselines; negative event assertions — Still present

### New Issues Introduced
- 🟡 **Dependency**: The set page records needs this story does not carry: a malformed root still yields its members, and a set-relative path for member documents.
- 🟡 **Testability, Completeness**: No criterion exercises "at any depth".
- 🔵 **Clarity**: "Markdown with frontmatter" defines away a document with no frontmatter.
- 🔵 **Clarity**: The activity row's key and action are unstated.
- 🔵 **Clarity**: Summary says a reader sees one change; activity and latest time also change.
- 🔵 **Scope**: The presentation declaration has no committed reader.
- 🔵 **Scope**: Member search ships with a link the next story rewrites.
- 🔵 **Dependency**: The design-inventory `assets/` directory; the Sources parser's shape and the API decision.
- 🔵 **Testability**: Missing root file, asset search, nested doc roots have no outcome.
- 🔵 **Completeness, Clarity**: Consumer counts disagree (six, seven, five).

### Assessment
The split removed the scope concern and the API criterion is in. Two
consumer needs must be adopted here, and the depth and genericity criteria
need fixtures.

## Changes After the Last Review Pass — 2026-11-23

The verdict above is the last review's. These changes were made afterwards
and have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- 🟡 **Dependency**: Needs the set page records — Addressed: members under a malformed root and set-relative paths for every member are now criteria here.
- 🟡 **Testability**: "At any depth" — Addressed with a three-level fixture.
- 🟡 **Testability**: Genericity criterion — Addressed: a set type declared only in test code, with countable outcomes.
- 🔵 Document definition; activity row; Summary's visible changes; dot-prefix scope; consumer count — Addressed.
- 🔵 Needs of member routes — Addressed: a member body by path, declarations and the set slug exposed to the frontend.
- 🔵 `blocks` omitted two direct dependants — Addressed.
- 🔵 Missing root, asset search, nested doc roots — Addressed.
- 🔵 Negative event assertions unbounded — Addressed with a sentinel write.
- Not addressed: the presentation declaration still has no committed reader beyond the set page's new requirement; "as before" baselines for the design-inventory page.
- The drafting agent's calls on this item were confirmed by the author on 2026-11-23.

## Approval — 2026-11-24

**Verdict:** APPROVE

Approved by the author; not re-reviewed after the changes above.
