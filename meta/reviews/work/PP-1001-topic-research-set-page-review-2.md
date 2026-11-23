---
type: "work-item-review"
id: "PP-1001-topic-research-set-page-review-2"
title: "Work Item Review: Topic-Research Set Page"
date: "2026-11-23T21:57:00+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-1001"
work_item_id: "PP-1001"
reviewer: "Toby Clemson"
verdict: "REVISE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 2
review_pass: 2
tags: []
last_updated: "2026-11-23T23:44:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Topic-Research Set Page

**Verdict:** REVISE

A coherent, densely specified story whose criteria pair literal inputs with
observable outcomes. Review 1's six pass-3 majors are closed: the
contradictions are gone and the server deliverable is scoped. Three gaps
remain: the rule that the page is chosen by presentation declaration is
unverified, the derived set response constrains PP-994's API shape and
PP-994 does not record it, and "unreadable" does not say whether the member
can be opened.

### Cross-Cutting Themes

- **The derived set response** (flagged by: dependency, clarity,
  completeness) — Open Questions say "none of this story's own" and then
  list it. PP-994's shape is settled first and names only structured sources
  as what it must leave room for.
- **The unmasked view 6 baseline** (flagged by: scope, dependency) — unowned
  when PP-1000 lands first. The PP-1000 review found the same.
- **Baselines per view or per sub-view** (flagged by: testability, clarity)
  — nine, eleven, or up to fourteen. Carried from review 1, not addressed.
- **Requirements with no criterion** (flagged by: testability,
  completeness) — presentation declaration, the no-brief error case, a
  member URL under an erroring set, level notes linked from an open level
  note.
- **What the server derives** (flagged by: clarity, scope) — unreadable
  reasons, lineage order and report staleness are absent from the Server
  list.

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability, Completeness**: No criterion verifies that the set page
  is chosen by presentation declaration
  **Location**: Requirements: Server; Acceptance Criteria
  Every criterion uses a topic-research set, so a name-based branch passes.
  PP-994 and PP-998 verify the equivalent rule with a test-only doc type.
- 🟡 **Dependency, Clarity, Completeness**: The requirement on PP-994's API
  shape is recorded only on the consuming side
  **Location**: Dependencies; Open Questions
  The shape "must leave room for the derived set response". PP-994 can be
  planned and its ADR accepted without it.
- 🟡 **Clarity**: "Unreadable" does not say whether the member can be opened
  **Location**: Requirements: Unreadable members
  "Every other document stays readable" implies it cannot; Navigation and
  PP-998 render its body at its URL. A list of paths and a list of links
  both satisfy the text.

#### Minor

- 🔵 **Scope, Dependency**: The unmasked view 6 baseline is owned by neither
  story if PP-1000 lands first
  **Location**: Requirements: Visual regression
- 🔵 **Testability, Clarity**: The number of baselines is indeterminate for
  views with sub-views
  **Location**: Acceptance Criteria
  6b and 6c differ from 6a only inside the masked region.
- 🔵 **Testability, Completeness, Clarity**: Three requirement branches have
  no criterion
  **Location**: Acceptance Criteria
  No brief under a set-level error, where "none" open is not an observable
  state; a member URL under the error; level notes linked from an open level
  note. A `round_count` change is also uncovered.
- 🔵 **Scope**: The server and `research`-crate slice is separable without
  interim UI
  **Location**: Requirements: Server
  The recorded objection to splitting, interim UI nobody designed, does not
  apply to this seam. Technical Notes already place it first.
- 🔵 **Scope**: Scope depends on landing order relative to PP-1000
  **Location**: Requirements: Sources
  Wiring, the crate edge, and the snapshot regeneration.
- 🔵 **Clarity, Scope**: The actor is unclear for several derivations and
  for choosing the set page
  **Location**: Requirements: Server
  Whether the kind-and-directory rule is a `research` export is undecided.
- 🔵 **Clarity, Testability**: "Pair" has a second meaning in the
  question-table criterion, and a compatibility-form punctuation mark can
  contradict the punctuation-mismatch criterion
  **Location**: Acceptance Criteria
  The table has no expected column.
- 🔵 **Testability**: The server-response criterion's fixture is
  under-specified and omits derived items
  **Location**: Acceptance Criteria
  Unmatched findings, the reopened mark and report staleness could be
  derived in the frontend and still pass.
- 🔵 **Testability**: The reading-position criterion fixes neither the edit
  nor the measure
  **Location**: Acceptance Criteria
- 🔵 **Testability**: "Raises no toast" and "without a reload" have no
  observation point
  **Location**: Acceptance Criteria
  PP-998 anchors the same negative on a refetch.
- 🔵 **Dependency**: Coupling to the research-topic on-disk contract is not
  listed
  **Location**: Dependencies
  PP-994 and PP-1000 each record their equivalent.
- 🔵 **Dependency**: PP-996's link builder reaches this story only
  transitively and is not named
  **Location**: Dependencies
- 🔵 **Dependency**: The shared question-pair fixture is absent from
  Dependencies
  **Location**: Acceptance Criteria; Dependencies
  PP-997 retires the join it encodes.
- 🔵 **Clarity**: The rule for when two outline items ask the same question
  is not stated
  **Location**: Context
- 🔵 **Clarity, Completeness**: Child count and a stale path
  **Location**: Context; Dependencies
  "Six children" against PP-868's seven; `drafts/../PP-997-…` does not
  resolve.

#### Suggestions

- 🔵 **Testability**: Label and empty-state wording is verifiable only
  against an export that does not yet exist
  **Location**: Acceptance Criteria
- 🔵 **Completeness**: The directory layout for the fourth unreadable reason
  is cited, not stated
  **Location**: Requirements: Unreadable members
- 🔵 **Scope**: The Summary omits the server and crate work
  **Location**: Summary
- 🔵 **Dependency**: Author sign-off and the wording contract are not
  recorded as dependencies; PP-995 is missing from `relates_to`
  **Location**: Dependencies; Frontmatter
- 🔵 **Clarity**: Stacked possessives, undefined terms, and mixed path forms
  **Location**: Requirements: Sources; Requirements; Technical Notes

### Strengths

- ✅ Context carries a glossary and one definition of reopened.
- ✅ Precedence is stated: a finding outranks "skipped", the error state
  overrides `primary`.
- ✅ Criteria use literal frontmatter values, filenames and label text, with
  negatives beside positives.
- ✅ The request-count criterion is relative, with no arbitrary threshold.
- ✅ The shared question table gives the planner and the visualiser one
  oracle.
- ✅ All three blockers say what they supply and are reciprocated.
- ✅ The decision to stay whole is recorded with a reason and a delivery
  order.

### Recommended Changes

1. **Add a presentation-declaration criterion** (addresses: set page chosen
   by declaration)
   A test-only doc type declaring the set-page presentation opens the set
   page; one declaring the entry-point presentation does not.
2. **Record the derived-response constraint on PP-994** (addresses: API
   shape)
   Add it to PP-994's open question and planning order. State here whose
   planning settles the response's shape, and fix the Open Questions count.
3. **Say what an unreadable entry links to and what its URL shows**
   (addresses: "unreadable")
4. **Enumerate the baselines by sub-view and make view 6 order-conditional**
   (addresses: baseline count; unmasked view 6)
   For example "1, 2, 3, 4a, 4b, 5, 6a masked, 7, 8a, 8b, 9".
5. **Add criteria for the uncovered branches** (addresses: three branches)
   State what the document area shows when no document is open.
6. **Complete the Server list** (addresses: actor for derivations)
   Unreadable reasons, lineage order, report staleness; name the component
   that chooses the page.
7. **Tighten four criteria** (addresses: "pair"; server-response fixture;
   reading position; observation point)
   Give the table an expected column.
8. **Extend Dependencies** (addresses: on-disk contract; PP-996; shared
   fixture)
9. **Record whether the server slice was considered as a split** (addresses:
   separable slice)
10. **Fix the child count and the PP-997 path** (addresses: child count and
    stale path)

## Per-Lens Results

### Clarity

**Summary**: Careful about referents; the remaining problems are local.

**Findings**: major — "unreadable" (medium). minor — "pair" reused;
baselines per view; Open Questions; child count; actor for derivations;
"none" open; same-question rule; malformed path. suggestion — possessives;
undefined terms; path forms.

### Completeness

**Summary**: Structurally complete; 38 criteria track the Requirements
closely.

**Findings**: minor — several requirements with no criterion (high); Open
Questions understate a pending decision; stale PP-997 path (high).
suggestion — directory layout cited, not stated.

### Dependency

**Summary**: Thorough. The reviewer searched PP-995, PP-997, PP-923 and the
set contract by pattern and did not open PP-22.

**Findings**: major — requirement on PP-994's API shape recorded on one side
(medium). minor — unmasked view 6 baseline (high); on-disk contract (high);
PP-996 transitive; shared fixture. suggestion — sign-off and wording
contract; PP-995 missing from `relates_to`.

### Scope

**Summary**: Coherent and large; one separable seam.

**Findings**: minor — server and crate slice separable; unmasked view 6
baseline (high); scope depends on landing order. suggestion — unreadable
classification has no owner; Summary omits server work.

### Testability

**Summary**: Unusually verifiable; the gaps are narrow.

**Findings**: major — no criterion for presentation declaration (high).
minor — three branches uncovered (high); baseline count; reading position;
no observation point; server-response fixture; punctuation contradiction.
suggestion — wording verifiable only against a future export.

---
*Review generated by /accelerator:review-work-item*

## Changes After the Review — 2026-11-23

The verdict above is the review's. These changes were made afterwards and
have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- 🟡 **Testability**: Presentation declaration — Addressed with a test-only doc type criterion.
- 🟡 **Dependency**: Derived set response and PP-994 — Addressed: recorded in PP-994's open question.
- 🟡 **Clarity**: "Unreadable" — Addressed by the author's decision: renamed unplaced, each entry a link to a member that opens, classified by the server.
- 🔵 Server slice — Split out by the author's decision into PP-1002, the derived set response: the exports, the crate edge, the response and the table of question comparisons moved there. This item renders the response.
- 🔵 Baselines — Addressed by the author's decision: twelve, named by rendering; view 6a follows landing order.
- 🔵 Gaps — Addressed by the author's decision: coverage is a label in the navigation; Gaps render in the open report.
- 🔵 Uncovered branches; observation points; reading position; punctuation criterion; same-question rule; child count; PP-997 path; PP-996; on-disk contract; Open Questions — Addressed.
- Drafting agent's calls awaiting the author: with no brief under a set-level error the document area shows the error state alone; the reader's place is kept by content, not scroll offset.
- Not addressed: label wording is verifiable only once PP-999's export exists; the glossary and mixed path forms.

## Re-Review (Pass 2) — 2026-11-23

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Testability**: Presentation declaration unverified — Resolved (the source search names no files)
- 🟡 **Dependency**: Derived response and PP-994 — Resolved
- 🟡 **Clarity**: "Unreadable" — Resolved (renamed unplaced; the title rule for a well-formed unplaced member is still open)
- 🔵 Server slice separable — Resolved (split into PP-1002), with a new boundary problem below
- 🔵 Baseline count; uncovered branches; observation points; reading position; question table; child count; PP-997 path — Resolved
- 🔵 Unmasked view 6a baseline — Resolved for 6a
- 🔵 Scope depends on landing order with PP-1000 — Still present
- 🔵 Label wording verifiable only against a future export — Still present

### New Issues Introduced
- 🟡 **Scope, Testability, Clarity**: Requirements and about a dozen criteria restate rules PP-1002 owns, while the item says the page derives none of them. Run against a stubbed response those criteria are tautological; run through the real server they re-verify the other story. The test level is never stated.
- 🟡 **Testability**: The non-derivation criterion contradicts the files on three of the nine derived items. A page that computes staleness, infers reopened, or sorts level notes itself would pass.
- 🔵 **Clarity, Scope**: "What the page derives" contradicts "derive none of it"; the rules the page does own (default member, error override, `depth` default) are not listed.
- 🔵 **Dependency, Scope**: Live update of derived state relies on a freshness guarantee PP-1002 does not state.
- 🔵 **Clarity**: "The reopened mark" has two referents; where level notes are linked while a level note is open is unnamed; the toast on removal is unstated.
- 🔵 **Testability**: The error-overrides-`primary` criterion names no `primary`; "readable", "without error" and "every other document" have no pass condition.
- 🔵 **Dependency**: PP-999's approval gate couples this story to PP-1000's review; the on-disk contract omits `kind` and the fixed filenames navigation relies on; one blocker is a local draft identifier.
- 🔵 **Completeness, Clarity**: Two drafting-agent calls read as settled in Requirements and unconfirmed in Drafting Notes.
- 🔵 **Scope**: Live change is a further separable seam the keep-whole decision did not consider.

### Assessment
The three earlier majors are closed. Both new majors come from the split: the rules moved to PP-1002, but this item's requirements and criteria still state them as the page's own. Restating them as rendering obligations on the response, and widening the one non-derivation criterion to all nine items, would clear both.

## Changes After the Re-Review — 2026-11-23

Made after pass 2 and not re-reviewed; the verdict above is the reviewers'.

- 🟡 Rules restated from PP-1002 — Addressed: a test-level criterion says which criteria run against a supplied response and which end to end; the three things the page decides are listed.
- 🟡 Non-derivation criterion — Addressed: it now contradicts the files on every derived item.
- 🔵 Landing order fixed: this page blocks PP-1000. Level-note links, removal toast, unplaced title, error precondition, reopened mark — Addressed; four calls await the author in Open Questions.
- Not addressed: Live change as a further seam; label wording waits on PP-999's export.
