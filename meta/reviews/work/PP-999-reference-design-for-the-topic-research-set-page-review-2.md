---
type: "work-item-review"
id: "PP-999-reference-design-for-the-topic-research-set-page-review-2"
title: "Work Item Review: Reference Design for the Topic-Research Set Page"
date: "2026-11-23T21:57:00+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-999"
work_item_id: "PP-999"
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

## Work Item Review: Reference Design for the Topic-Research Set Page

**Verdict:** REVISE

One coherent, well-bounded task whose deliverables are enumerable artefacts.
Review 1's blocker is closed: the artifact is located and its export forms
described. Four gaps remain: the conversion the task waits on has no owner or
fallback, the granularity of views 4, 6 and 8 is ambiguous in a numbering the
item calls a contract, view 4a may be unsatisfiable, and no criterion checks
the label text the two stories already fix.

### Cross-Cutting Themes

- **The artboard conversion** (flagged by: dependency, scope, completeness) —
  an external precondition with no owner, date, check or fallback. Open
  Questions reads "None" while Assumptions leave the standalone export
  unverified and say nothing of the source form.
- **"Settled" has no observable trigger** (flagged by: dependency, scope,
  clarity, testability) — approval waits on two draft stories' requirements
  being settled, and the stories wait on approval.
- **Views or sub-views** (flagged by: clarity, testability) — 13 or 16 index
  entries; the criteria treat 4 and 8 as single views and 6 as three.
- **Label wording** (flagged by: testability, clarity, scope) — authority
  runs both ways between the export and the stories, and no criterion checks
  the strings the stories already state.
- **Elements in no view** (flagged by: scope, testability) — set navigation,
  checkbox state, `depth`, `answered` coverage, a report's `question`.
- **The source form** (flagged by: completeness, scope, testability) —
  required, with no criterion.

### Findings

#### Critical

None.

#### Major

- 🟡 **Dependency, Scope, Completeness**: The start gate and the export
  capability rest on an external conversion that no work item owns
  **Location**: Dependencies; Assumptions; Open Questions
  The conversion had not finished on 2026-11-23. This task gates PP-1001 and
  PP-1000 and, through them, PP-868 and PP-22. PP-868 says the reference
  design has no blockers.
- 🟡 **Clarity, Testability**: Unclear whether views 4, 6 and 8 are views in
  their own right or containers
  **Location**: Requirements; Acceptance Criteria
  "Each of the nine views and the seven sub-views opens on its own" reads as
  16 entries, yet 4, 6 and 8 have no content of their own. PP-1001's
  baselines cite views by number.
- 🟡 **Clarity**: View 4a asks for the synthesis open and a report's Gaps in
  one view
  **Location**: Requirements
  The page shows one member at a time. If Gaps are in the report's body, 4a
  cannot show them while the synthesis is open.
- 🟡 **Testability, Clarity**: No criterion checks that the export uses label
  text the stories already state
  **Location**: Acceptance Criteria
  The stories fix "may be out of date: the set has been reopened", "N lines
  in this section could not be read", `retracted`, `withdrawn` and `also`.
  The export can pass with other wording, leaving PP-1001 two criteria that
  cannot both hold.

#### Minor

- 🔵 **Dependency, Scope, Clarity, Testability**: Approval is gated on two
  draft stories settling, with no observable trigger
  **Location**: Dependencies
  The approval line does not record which revisions of the stories it was
  approved against.
- 🔵 **Completeness, Scope, Testability**: The source form is required and
  has no criterion
  **Location**: Acceptance Criteria
- 🔵 **Scope, Testability**: Elements the two stories render appear in no
  view
  **Location**: Requirements
  A design with no set navigation or checkbox states would pass.
- 🔵 **Testability**: View-content criteria bundle about thirty checks into
  four boxes
  **Location**: Acceptance Criteria
  "Linked in lineage order" passes with one note; 8a omits the link back to
  the set; view 9 does not say the error names the manifest.
- 🔵 **Dependency**: Definitions inherited from PP-995, PP-994 and PP-998
  are not named as content sources
  **Location**: Dependencies
  Views 6 to 8 show states those items define.
- 🔵 **Clarity**: "View" names both design renderings and application views
  **Location**: Summary
  The Summary's second sentence uses three senses and cites view 6 before it
  is defined.
- 🔵 **Clarity**: The artifact and its exports go by several names
  **Location**: Context; Requirements
  The first requirement does not say which of the two artifacts is extended.
- 🔵 **Clarity**: The export path leaves the date slug, the file name and
  "beside it" unspecified
  **Location**: Requirements
  The May model keeps both forms in a `prototype-full` subdirectory.
- 🔵 **Testability**: "Renders" has no pass condition
  **Location**: Acceptance Criteria
  No browser is named and a blank artboard could be argued to render.

#### Suggestions

- 🔵 **Completeness**: View definitions lean on terms defined only in the
  sibling stories
  **Location**: Requirements
- 🔵 **Dependency**: PP-997 is a plausible follow-up source
  **Location**: Dependencies
- 🔵 **Scope**: The label criterion lists fewer marks than "every mark and
  state"
  **Location**: Acceptance Criteria
  The four empty states, `None found.`, and the unread-lines notice are
  absent.
- 🔵 **Clarity**: Dangling referents
  **Location**: Drafting Notes; Requirements
  "My reading" appears nowhere; "its outline item" has no single referent in
  PP-1001's terms.

### Strengths

- ✅ The two export forms are defined and used consistently.
- ✅ Views and sub-views are numbered and lettered, and counted the same way
  in Summary, Requirements and criteria.
- ✅ The offline criterion gives a real procedure, and Drafting Notes say the
  May export was inspected, not opened in a browser.
- ✅ The tier criterion is observable without judgement: visible text that
  differs in greyscale.
- ✅ Ownership of each view is stated and agrees with PP-1001.
- ✅ Drift is fenced: a later change is a follow-up, not a reopening.
- ✅ `blocks` matches Dependencies and both stories reciprocate.

### Recommended Changes

1. **Give the conversion an owner and a fallback** (addresses: start gate)
   Record export capability, standalone and source, as a dependency to verify
   first. Name the fallback, such as extending the original artifact, and
   correct "no blockers" in PP-868.
2. **Fix the index entries** (addresses: views or containers; "renders")
   State, for example, "13 entries: 1, 2, 3, 4a, 4b, 5, 6a, 6b, 6c, 7, 8a,
   8b, 9", and use one granularity throughout. Define a failed render.
3. **Say where a `partial` report's Gaps show** (addresses: view 4a)
   In the navigation, or in the open report with a further sub-view.
4. **Add a verbatim-label criterion and one precedence rule** (addresses:
   label text)
   List each story-stated string and the view it appears in.
5. **Name the "settled" signal and record what was approved against**
   (addresses: approval trigger)
6. **Add a source-form criterion** (addresses: source form)
7. **Add the missing elements to views, or state they are left to the
   build** (addresses: elements in no view)
8. **Turn each view's entry into a numbered checklist** (addresses: bundled
   criteria)
9. **Name PP-995, PP-994, PP-998 and PP-997 as follow-up sources**
   (addresses: inherited definitions)
10. **Settle one name per artifact and export, and the export layout**
    (addresses: several names; export path; "view")

## Per-Lens Results

### Clarity

**Summary**: Largely precise. Two ambiguities carry weight because the
stories cite views by number.

**Findings**: major — views 4, 6 and 8 as views or containers (high); view
4a against one member at a time (medium). minor — "view" overloaded;
artifact names; export path; "settled" and a cryptic sentence; label
authority runs both ways. suggestion — dangling referents.

### Completeness

**Summary**: Structurally complete for a task; frontmatter intact.

**Findings**: minor — source form has no criterion (high); Open Questions
reads "None" beside an unverified export assumption. suggestion — terms
defined only in siblings.

### Dependency

**Summary**: Couplings are well mapped and reciprocated; the gaps are at the
edges.

**Findings**: major — conversion and export capability with no fallback
(medium). minor — approval gated on draft stories with no trigger (high);
inherited definitions not named. suggestion — PP-997.

### Scope

**Summary**: One coherent unit; kind and M size fit.

**Findings**: major — conversion is a precondition no item owns (medium).
minor — completion depends on an undefined "settled"; elements in no view;
source form has no criterion (high). suggestion — label criterion narrower
than the requirement.

### Testability

**Summary**: Largely verifiable; gaps are in coverage.

**Findings**: major — no criterion for story-stated label text (high).
minor — source form and artifact extension unchecked; bundled view-content
criteria; elements in no view; index count and "renders"; approval
precondition not checkable.

---
*Review generated by /accelerator:review-work-item*

## Changes After the Review — 2026-11-23

The verdict above is the review's. These changes were made afterwards and
have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- 🟡 **Dependency, Scope**: Artboard conversion — Removed by the author's decision: the design is made in the original prototype, and the task is blocked by nothing.
- 🟡 **Clarity**: Views or containers — Addressed by the author's decision: views 4, 6 and 8 are groupings, and each rendering is cited by its leaf number.
- 🟡 **Clarity**: View 4a and Gaps — Addressed by the author's decision: Gaps are part of the open report. View 4 is now 4a (synthesis open), 4b (`partial` report open), 4c (`complete`), making fourteen renderings.
- 🟡 **Testability**: Story-stated labels — Addressed: one precedence rule and a verbatim criterion, including "No sources found." for 6b.
- 🔵 "Settled" — Addressed by the author's decision: each story's latest review verdict is APPROVE or COMMENT; planning-time questions do not hold approval; the approval line records both stories' commits.
- 🔵 Source form; "renders"; follow-up sources; naming; export layout; view 6a, 8a and 9 details; "unplaced" — Addressed.
- Drafting agent's calls awaiting the author: the set navigation, checkbox states, `depth`, an `answered` report and each report's `question` were added to the views; the slug's date form.
- Not addressed: per-view checklists; a glossary of the stories' terms.

## Re-Review (Pass 2) — 2026-11-23

**Verdict:** COMMENT

Work item is acceptable but could be improved — see the major finding below.

### Previously Identified Issues
- 🟡 **Dependency, Scope**: Artboard conversion — Resolved (dropped; Context still cites an "artboard" identifier)
- 🟡 **Clarity**: Views 4, 6 and 8 as views or containers — Resolved (fourteen renderings; "view" and "sub-view" still used loosely in some criteria)
- 🟡 **Clarity**: View 4a and Gaps — Resolved
- 🟡 **Testability**: Story-stated labels unchecked — Resolved, with one label missing, see below
- 🔵 "Settled" trigger — Resolved (the task now cannot close on its own work; "Blocked by: nothing" understates that)
- 🔵 Source form — Partially resolved (criterion added; which files make up each form is not determinable from the May export)
- 🔵 Elements in no view; view 6a, 8a and 9 details; follow-up sources — Resolved
- 🔵 Bundled view-content criteria — Still present

### New Issues Introduced
- 🟡 **Clarity, Testability, Completeness**: PP-1000 takes "the label view 6a gives an unrecognised tier" from this export, yet view 6a requires only "shown as written" and the label list omits it. "Every mark and state" has no stated extent.
- 🔵 **Clarity, Testability**: View 6b is called "the `None found.` empty state" in three places and reads "No sources found." in a fourth; the notice's singular form and a two-line fixture are unstated.
- 🔵 **Dependency, Clarity, Scope**: PP-1002 is absent although it owns most states the views show; the unplaced reasons are attributed to PP-994.
- 🔵 **Completeness, Clarity**: The drafting agent's unconfirmed calls sit in Drafting Notes while Open Questions reads "None".
- 🔵 **Clarity, Testability**: Sub-views have no names, though each title must carry one.
- 🔵 **Scope, Completeness, Testability**: "Keeping the views it already holds" has no criterion.
- 🔵 **Testability**: "Exported from the same revision" cannot be checked; no browser is named.
- 🔵 **Scope, Dependency**: A follow-up has no stated home, and no rule for baselines accepted against a superseded commit.

### Assessment
All four earlier majors are closed. The one new major is a label the Sources story depends on and this task does not require; adding it to view 6a and the label list clears it. The rest is naming and record-keeping.

## Changes After the Re-Review — 2026-11-23

Made after pass 2 and not re-reviewed; the verdict above is the reviewers'.

- 🟡 Unrecognised-tier label — Addressed: required in view 6a and the label list, with its text fixed in PP-1000.
- 🔵 View 6b naming; two-line notice fixture; PP-1002 as a source; existing views; sub-view names; where a follow-up lives; unconfirmed calls in Open Questions — Addressed.

## Approval — 2026-11-24

**Verdict:** APPROVE

Approved by the author; not re-reviewed after the changes above.
