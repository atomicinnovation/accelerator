---
type: "work-item-review"
id: "PP-994-generic-set-model-indexing-and-api-review-2"
title: "Work Item Review: Generic Set Model: Indexing and API"
date: "2026-11-23T21:57:00+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-994"
work_item_id: "PP-994"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 2
review_pass: 2
tags: []
last_updated: "2026-11-24T00:06:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Generic Set Model: Indexing and API

**Verdict:** REVISE

The item is structurally complete, densely specified, and its split from
PP-998 at the member API is a sound boundary. Review 1's open points (depth,
genericity, the consumers' needs) are closed. What remains is coverage: three
requirements that downstream stories rely on have no criterion, and the item
does not say how much frontend work it contains.

### Cross-Cutting Themes

- **Requirements with no criterion** (flagged by: testability, completeness,
  clarity, dependency) — the set slug on every response, the activity row's
  action, and members' absence from inferred relations are stated and never
  verified. PP-998 says each of its needs is "now a criterion there"; the
  slug is not.
- **Garbled Blocks entry** (flagged by: clarity, completeness, dependency) —
  the first Blocks bullet ends in "and:", and the second describes PP-1001
  and PP-1000 as gated both through PP-998 and directly.
- **Design-inventory asset rule** (flagged by: clarity, completeness,
  dependency, testability) — Requirements name `screenshots/`, Dependencies
  add `assets/`, and the Model classifies by file type. No fixture covers
  `assets/` or a markdown file under either directory.
- **Ambiguous fixtures** (flagged by: clarity, testability) — "that set" and
  "the deepest" do not resolve to one subject.

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability, Completeness, Clarity**: No criterion verifies the set
  slug on responses
  **Location**: Acceptance Criteria; Requirements: Index consumers
  "Carry a set's slug on every response that names a root or a member" has
  no criterion, and "every response" is unbounded. PP-998, PP-1001 and
  PP-1000 build links from it.
- 🟡 **Testability, Completeness**: The activity row's action rule has no
  criterion
  **Location**: Acceptance Criteria
  The row "reads as an edit to the set whatever the member's own action" and
  reads created or deleted only when the root is. The activity criterion
  checks count, order and time, never the action.
- 🟡 **Clarity**: Server-versus-frontend actor is unclear for criteria
  phrased as links, listings and pages
  **Location**: Acceptance Criteria
  The story is scoped to the indexer, file driver, watcher and API, yet
  criteria assert that a "link opens the member's set", a listing "shows one
  row", and a page "renders the same body". The item never says whether
  these are response assertions, frontend regressions, or new frontend work.

#### Minor

- 🔵 **Clarity, Completeness, Dependency**: Blocks entries are garbled and
  self-contradictory
  **Location**: Dependencies
  One bullet trails off in "and:"; the next states PP-1001 and PP-1000 as
  both transitive and direct. "This story's API shape is settled" reads as
  present fact while Open Questions lists it as unresolved.
- 🔵 **Clarity, Completeness, Dependency, Testability**: Design-inventory
  asset rule is stated three ways and `assets/` has no fixture
  **Location**: Requirements: Design inventories; Dependencies
  A markdown file under `assets/` is a document by the Model rule and an
  asset by the Dependencies wording.
- 🔵 **Clarity, Testability**: Dangling referents in criteria
  **Location**: Acceptance Criteria
  "A library holding only that set" follows a criterion that names no set;
  "the deepest" has two candidates at equal depth, one document and one
  asset, and the expected event path is unstated.
- 🔵 **Testability**: "Other doc types are unaffected" by the dot-prefix rule
  is unverified
  **Location**: Acceptance Criteria
  A global exclusion would pass every criterion.
- 🔵 **Testability**: Regression criteria rely on an unstated "before"
  **Location**: Acceptance Criteria
  "Clusters as it did before", "the title, status, and date it showed
  before", "renders the same body". Carried over from review 1.
- 🔵 **Testability**: "Once the write settles" and "is now" lack a bound
  **Location**: Acceptance Criteria
  No debounce contract backs "exactly one"; the latest-time criterion has no
  exact comparand and omits an asset change.
- 🔵 **Testability, Completeness**: Related-items coverage stops at
  `relates_to` and omits inferred relations
  **Location**: Acceptance Criteria
  `parent`, `target` and the work-item references are untested on a member,
  and nothing asserts a member's absence from the related response's
  inferred items.
- 🔵 **Testability**: The source-naming criterion has a judgement-based
  boundary
  **Location**: Acceptance Criteria
  "In code that handles sets" is decided by the reviewer.
- 🔵 **Clarity**: "The member API" is named three ways and its inclusion of
  the root varies
  **Location**: Requirements; Acceptance Criteria; Open Questions
- 🔵 **Clarity**: The related-items requirement leaves its subject implicit
  **Location**: Requirements: Index consumers
  It never says the document may be a member, nor names the work-item
  reference fields.
- 🔵 **Completeness**: The nested-doc-root assumption sits only under Open
  Questions
  **Location**: Assumptions
- 🔵 **Dependency**: PP-996 is absent, and a drafting note points at the
  wrong item
  **Location**: Drafting Notes; Dependencies
  The corrected set links moved to PP-996, not PP-998. Whether PP-996
  consumes this story's slug field is unrecorded.
- 🔵 **Dependency**: On-disk conventions name the skills but not every writer
  **Location**: Dependencies; References
  The run ledger's filename belongs to the research CLI, and the
  design-inventory skill is not in References.
- 🔵 **Scope**: Asset indexing and the design-inventory migration have no
  consumer here
  **Location**: Requirements: Design inventories
  A recorded author decision; roughly a quarter of the criteria serve it.
- 🔵 **Scope**: The API shape and ADR gate downstream planning on a different
  clock from the implementation
  **Location**: Dependencies
  `blocks` cannot distinguish "blocks planning" from "blocks delivery".

#### Suggestions

- 🔵 **Testability**: Open-question deliverables have no exit criterion
  **Location**: Open Questions
- 🔵 **Scope**: The presentation declaration ships with no behaviour here
  **Location**: Requirements: Model
  Carried over from review 1, not addressed.
- 🔵 **Scope**: No delivery order is stated for an L story
  **Location**: Technical Notes
- 🔵 **Dependency**: The PP-995 soft ordering is one-sided
  **Location**: Dependencies
- 🔵 **Dependency**: Frontend consumers of the changed search and activity
  responses are not named
  **Location**: Dependencies
- 🔵 **Completeness**: PP-995 is missing from References
  **Location**: References
- 🔵 **Clarity**: "Three changes" is punctuated as two; "a reader" is
  undefined
  **Location**: Summary
- 🔵 **Clarity**: Several terms rely on context outside the item
  **Location**: Open Questions

### Strengths

- ✅ The Model defines set, root, member, document and asset before use, and
  separates an index record from a listed item.
- ✅ Nearly every criterion names a concrete fixture, an action and an
  observable outcome.
- ✅ The excluded-file criterion bounds its negative assertion with a
  sentinel write.
- ✅ Genericity is verified behaviourally through a test-only doc type with a
  different root filename.
- ✅ All four dependants are in `blocks` and reciprocate with `blocked_by`.
- ✅ Out-of-scope work is named with its owner: asset bytes to PP-134, member
  URLs to PP-998, the set page to PP-1001.
- ✅ Drafting Notes record each scope decision and who made it.

### Recommended Changes

1. **Add a set-slug criterion and enumerate the responses** (addresses: no
   criterion verifies the set slug)
   List the responses in scope (member API, search, related, type listing,
   document detail, activity) and say whether change events are included.
2. **Add an activity-action criterion** (addresses: activity row's action)
   Member added and removed reads edited; a new set reads created; a deleted
   root reads deleted.
3. **State the frontend boundary** (addresses: server-versus-frontend actor)
   Say for each link, listing and page criterion which component produces the
   outcome, and whether the story includes frontend change.
4. **Rewrite the Blocks entries as one list** (addresses: garbled Blocks)
   Four direct edges, each with what it consumes, and one note on the
   PP-998 path. Reword the planning-order sentence as a constraint.
5. **State the asset rule once, by file type** (addresses: design-inventory
   asset rule)
   Extend the fixture with an `assets/` directory and a markdown file.
6. **Repair the fixtures** (addresses: dangling referents; "before"
   baselines; settle bound)
   Describe each set inline, name the changed file and its event path, and
   pin the baseline values.
7. **Add the non-set dot-prefix case and an inferred-relations case**
   (addresses: dot-prefix; related-items coverage)
8. **Record PP-996 and correct the drafting note** (addresses: PP-996 absent)
9. **Move the nested-doc-root assumption into Assumptions; add PP-995 to
   References** (addresses: assumption placement; References)

## Per-Lens Results

### Clarity

**Summary**: Dense but precise; vocabulary is defined before use. Ambiguity
concentrates in the server-versus-frontend boundary, the Blocks entry, and
dangling referents in the criteria.

**Findings**: major — server-versus-frontend actor (medium confidence).
minor — garbled Blocks entry; dangling referents ("that set", "the deepest",
"it"); "the member API" naming; unbounded "every response" and "a set's
slug"; design-inventory asset rule; related-items subject. suggestion —
"three changes"; terms defined elsewhere.

### Completeness

**Summary**: Structurally complete and densely populated; frontmatter is
intact and consistent with the body.

**Findings**: minor — three requirements with no criterion (set slug,
activity created/deleted, inferred relations); unfinished Blocks bullet;
nested-doc-root assumption only under Open Questions; `assets/` missing from
Requirements. suggestion — PP-995 missing from References.

### Dependency

**Summary**: Couplings are well mapped and reciprocated; nothing would stop
the work starting.

**Findings**: minor — contradictory Blocks entries; PP-996 unmentioned and a
stale drafting note; on-disk conventions name skills but not every writer.
suggestion — PP-995 ordering one-sided; frontend consumers of search and
activity responses unnamed.

### Scope

**Summary**: Large but mostly coherent; the core is indivisible and the
split from PP-998 is sound. The reviewer did not read PP-1000, PP-862,
PP-995, the research-topic skill, or the two ADRs.

**Findings**: minor — asset slice has no consumer here (a recorded
decision); API shape and ADR are a separately gating deliverable.
suggestion — presentation declaration ships with no behaviour; no delivery
order.

### Testability

**Summary**: Criteria are strong; the weaknesses are coverage gaps, not
vague wording. The reviewer did not read the sibling items or the ADRs in
full.

**Findings**: major — no slug criterion (high); no activity-action criterion
(medium). minor — "the deepest" ambiguous; non-set dot-prefix unverified;
`assets/` has no fixture; unstated "before" baselines; settle and "now"
unbounded; related-items coverage; judgement-based source-naming check.
suggestion — open-question deliverables have no exit criterion.

---
*Review generated by /accelerator:review-work-item*

## Changes After the Review — 2026-11-23

The verdict above is the review's. These changes were made afterwards and
have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- 🟡 **Testability**: Set slug — Addressed: the responses that carry it are named and a criterion checks each. Change events carry none.
- 🟡 **Testability**: Activity row's action — Addressed with a criterion.
- 🟡 **Clarity**: Server-versus-frontend actor — Addressed by the author's decision: the story is server-only, its criteria are response assertions, and a member's search row carries its set's slug.
- 🔵 Blocks entries; dangling referents; "the member API"; related-items subject; nested-doc-root assumption; PP-996; References — Addressed.
- 🔵 Asset rule — Addressed: stated once by file type. `assets/` was dropped: the inventory-design skill writes no such directory.
- 🔵 Non-set dot-prefix; inferred relations; settle bound; "is now"; source-naming check; API-shape exit criterion — Addressed with criteria.
- 🔵 Derived set response — The API shape must leave room for it; recorded in Open Questions and as a fifth blocked item, PP-1002.
- Not addressed: the asset slice and the presentation declaration still have no consumer here (recorded decisions); no delivery order; "clusters as before" now compares against the set without members.
- Drafting agent's calls awaiting the author: no slug on change events; any markdown file in an inventory is a document member.

## Re-Review (Pass 2) — 2026-11-23

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Testability**: No criterion verifies the set slug — Partially resolved (criterion added; it names no field outside search and has no design-inventory case)
- 🟡 **Testability**: Activity row's action — Partially resolved (criterion added; its outcome depends on event order, see below)
- 🟡 **Clarity**: Server-versus-frontend actor — Resolved (server-only, response assertions)
- 🔵 Blocks entries; dangling referents; member API naming; related-items subject; nested-doc-root assumption; PP-996; References — Resolved
- 🔵 Asset rule; non-set dot-prefix; inferred relations; settle bound — Resolved, with residue below
- 🔵 **Scope**: API shape and ADR gate planning on a different clock — Still present, now raised as major
- 🔵 **Scope**: Asset slice and presentation declaration have no consumer here — Still present (recorded decision)

### New Issues Introduced
- 🟡 **Testability, Clarity**: Set creation and root deletion depend on unstated event ordering. "Created together reads created" conflicts with "a member change reads as an edit" and "the latest change replaces the row". A set arriving by rename of its temporary directory, the main real path, has no criterion; nor has root deletion with members still on disk.
- 🟡 **Testability**: Genericity is verified for four behaviours only. The test-only doc type covers the member API, search, the type listing and one event; the source search covers three files, while consumer rules live in five others.
- 🟡 **Scope**: The decision deliverables that gate five stories are bundled with an L implementation, with no status of their own.
- 🔵 **Clarity, Completeness**: Two drafting-agent calls read as settled in Requirements and unconfirmed in Drafting Notes.
- 🔵 **Clarity, Testability**: "Slug" has three meanings; which field carries the set's slug is named for search alone.
- 🔵 **Clarity**: The inferred-relations criterion's premise ("fall in one lifecycle cluster") contradicts the rule that a member is never clustered.
- 🔵 **Testability**: Several criteria assert on "the index", not on a response; latest-change time is undefined after a removal; the asset-search check can pass vacuously.
- 🔵 **Dependency**: PP-1002 is missing from the planning order; the nested-doc-root check is not ordered before the ADR; PP-996 is missing from `relates_to`.
- 🔵 **Scope**: The Summary names fewer changes than Requirements deliver.

### Assessment
The three earlier majors are closed or reduced to detail. Two of the new majors come from criteria added in this pass: the activity-action criterion exposed an ordering rule the item never states, and the genericity backstop is narrower than the rule it guards. The third is the scope point carried from review 2. The item needs one more pass, mostly on the watcher's event semantics.

## Changes After the Re-Review — 2026-11-23

Made after pass 2 and not re-reviewed; the verdict above is the reviewers'.

- 🟡 Event ordering — Addressed by the author's decision: the row's action is the root's own; a set arriving by rename emits one created event per file; a deleted root removes its members with a deleted event each; latest-change time counts removals.
- 🟡 Genericity — Addressed: the test-only doc type criterion covers every consumer, and the source search covers the whole server.
- 🟡 Decision deliverables — Kept in the story by the author's decision, as a named first stage with a dated decisions line that releases downstream planning; the nested-doc-root check comes first.
- 🔵 The activity row no longer carries the set's slug (PP-996 looks it up). Unconfirmed calls are listed in Open Questions.
- Not addressed: which field carries the set's slug outside search; criteria that assert on "the index".

## Approval — 2026-11-24

**Verdict:** APPROVE

Approved by the author; not re-reviewed after the changes above.
