---
type: "work-item-review"
id: "PP-999-reference-design-for-the-topic-research-set-page-review-1"
title: "Work Item Review: Reference Design for the Topic-Research Set Page"
date: "2026-11-23T17:39:42+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-999"
work_item_id: "PP-999"
reviewer: "Toby Clemson"
verdict: "REVISE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-11-23T21:18:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Reference Design for the Topic-Research Set Page

**Verdict:** REVISE

The task is one coherent, well-bounded unit with a concrete sign-off and no
blockers. Its weaknesses are that the artifact it extends cannot be found
from the item, and that the boundary of what the design must cover is loose:
the two stories it gates render states no view shows, and they cite the
views by a count that leaves two designed states out.

### Cross-Cutting Themes

- **The artifact has no locator** (flagged by: completeness, clarity,
  testability) — no link, identifier or export method, so only the author
  can start the work or trace the export back.
- **Coverage boundary** (flagged by: scope, dependency, testability,
  clarity) — no view shows a `complete` set, a report both stale and
  reopened, a `partial` report's Gaps, a withdrawn mark, an absent venue,
  rejected tier text, `None found.`, or the four unreadable reasons.
- **Seven views or nine** (flagged by: scope, clarity, dependency) — seven
  numbered views plus two states; siblings baseline "the seven".
- **The design draws on unsettled sibling content** (flagged by: dependency,
  scope) — PP-1000's unread-lines question is a design decision.

### Findings

#### Critical

None.

#### Major

- 🟡 **Completeness, Clarity**: The artifact to be extended is not located or
  described
  **Location**: Context, References
  Neither its location, what it covers today, nor how an export is produced
  is stated.
- 🟡 **Testability**: "Shows each of the seven views" binds neither the
  views' contents nor how to find them
  **Location**: Acceptance Criteria
  A view that exists but omits a listed element passes, and views need not
  be labelled or separately reachable, though siblings cite them by number.
- 🟡 **Dependency**: The design depends on unsettled content in the stories
  it blocks
  **Location**: Dependencies
  Both are drafts, and one carries an open design question, yet the task is
  recorded as blocked by nothing.

#### Minor

- 🔵 **Scope, Testability, Dependency**: Coverage leaves out states the
  blocked stories must build
  **Location**: Requirements
- 🔵 **Scope, Clarity, Dependency**: The "seven views" count excludes two
  states the task delivers
  **Location**: Requirements, Drafting Notes
- 🔵 **Clarity**: Context names five statuses; the views cover four
  **Location**: Requirements
- 🔵 **Completeness, Clarity, Testability**: Export path and format are
  undecided while a criterion and a sibling depend on them
  **Location**: Open Questions, Assumptions
- 🔵 **Completeness**: Several views are named without their content
  **Location**: Requirements
- 🔵 **Testability**: The tier-label criterion can be met without the tiers
  being distinguishable
  **Location**: Acceptance Criteria
- 🔵 **Dependency**: Claude artifacts prerequisites are implied, not recorded
  **Location**: Dependencies

#### Suggestions

- 🔵 **Testability**: Tie the approval to a revision of the export
- 🔵 **Clarity**: Define the domain terms in the view list; align "merged
  tier" and "unmatched members" with the siblings
- 🔵 **Clarity**: Say who "my" and "the author" are
- 🔵 **Scope**: Reconfirm the S size once coverage is settled

### Strengths

- ✅ One purpose, no production code, no blockers; task kind fits.
- ✅ The views are an enumerated list with named contents.
- ✅ Approval is an observable artefact: a dated line in a named section.
- ✅ Colour independence is stated as a text label per tier.
- ✅ Frontmatter, Dependencies and both siblings agree on what it blocks.

### Recommended Changes

1. **Locate the artifact** (addresses: no locator)
   Add its link or name to References and one line on producing the export.
2. **Settle the coverage boundary** (addresses: coverage, five statuses)
   Fold the missing states into views 4 to 7, or name those left to the
   stories.
3. **Number every designed state** (addresses: seven or nine)
   Make the removed and error states views 8 and 9, and say which views the
   set page and Sources rendering each own.
4. **Bind the criterion to contents** (addresses: "shows each view")
   One checkbox per view naming its elements; views titled and separately
   reachable.
5. **Record the reliance on the siblings** (addresses: unsettled content)
   State whether view 6 settles the unread-lines question.
6. **Decide path and format** (addresses: export path)

## Per-Lens Results

### Clarity
**Summary**: Short and coherent; the central referent cannot be located.
**Findings**: major — artifact has no locator. minor — "added to the seven
views"; five statuses, four shown; single-file is an assumption only.
suggestion — undefined domain terms; "my reading" and "the author".

### Completeness
**Summary**: Structurally complete for a task.
**Findings**: major — artifact not located or described. minor — export path
open; views named without content.

### Dependency
**Summary**: Blocking edges are accurate and symmetric; the reverse coupling
is not recorded.
**Findings**: major — design depends on unsettled content. minor — consumers
cite views by ordinal; sixth view covers less than its consumer renders;
Claude artifacts prerequisites.

### Scope
**Summary**: One coherent unit; the coverage boundary is the weak point.
**Findings**: minor — coverage leaves out states; seven-views count.
suggestion — sibling's open design question; S may be light.

### Testability
**Summary**: All four criteria are checkable by inspection; the main one is
loose.
**Findings**: major — "shows each of the seven views". minor — downstream
states uncovered; "opens with no network access"; tier-label criterion.
suggestion — approval not tied to a revision.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-11-23

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Completeness, Clarity**: The artifact is not located — Still present (now an open question for the author)
- 🟡 **Testability**: "Shows each of the seven views" — Partially resolved (nine titled views with per-group criteria; elements not individually countable)
- 🟡 **Dependency**: Design depends on unsettled sibling content — Partially resolved (reliance recorded; both stories still draft)
- 🔵 **Scope, Testability, Dependency**: Coverage of states — Resolved
- 🔵 **Scope, Clarity, Dependency**: Seven views or nine — Resolved; variants within views now uncounted
- 🔵 **Clarity**: Five statuses, four shown — Partially resolved ("how a `complete` set differs" is undefined)
- 🔵 **Testability**: Tier-label criterion — Resolved

### New Issues Introduced
- 🟡 **Testability, Clarity, Completeness**: "How a `complete` set differs" has no observable content.
- 🟡 **Testability, Completeness, Scope**: "Every mark and state a text label" is checked only for tiers.
- 🔵 **Clarity, Testability, Scope**: "Separately reachable" and "variant" are undefined; views 4, 6 and 8 hold several screens.
- 🔵 **Clarity**: View 6 lists no level-note element; "tier text shown as written" omits "rejected".
- 🔵 **Dependency**: The contract fixes numbering but not label wording, which flows both ways.
- 🔵 **Clarity, Completeness**: A drafting note asserts a single-file export the open questions leave undecided.
- 🔵 **Clarity, Testability**: "The revision approved" is ambiguous.

### Assessment
The task cannot start from the item until the author records where the
artifact is and how it exports. The other majors are two loose elements.

## Changes After the Last Review Pass — 2026-11-23

The verdict above is the last review's. These changes were made afterwards
and have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- 🟡 **Completeness, Clarity**: Artifact not located — Addressed: both artifact links, the export's two forms, and the path `meta/prototypes/<date-slug>-claude-design-prototype`, supplied by the author. Share keys are left out.
- 🟡 **Testability**: "How a `complete` set differs" — Addressed: sub-view 4b, differing in its status label alone.
- 🟡 **Testability**: "Every mark and state" — Addressed: an enumerated criterion.
- 🔵 Reachability and variants — Addressed: lettered sub-views opened from an index, titled by number and name.
- 🔵 View 6 level-note elements; rejected tier; four unreadable reasons — Addressed.
- 🔵 Contract wording; approved revision; gate wording — Addressed.
- 🔵 Export form — Corrected twice: the standalone form is one self-contained HTML file, confirmed by inspecting the May 2026 export, so the offline criterion stands.
- Open: the artboard-format artifact had not finished converting, and its standalone export is assumed, not checked.
