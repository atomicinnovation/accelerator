---
type: "work-item-review"
id: "PP-1001-topic-research-set-page-review-1"
title: "Work Item Review: Topic-Research Set Page"
date: "2026-11-23T16:16:47+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-1001"
work_item_id: "PP-1001"
reviewer: "Toby Clemson"
verdict: "REVISE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-11-23T21:18:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Topic-Research Set Page

**Verdict:** REVISE

The item is precise at the level of data rules: literal frontmatter values,
literal tier text, and a negative case beside most positive ones. Its
weaknesses are structural. It never says what the set page is made of, it
leans on decisions its two blockers have left open, and it leaves three seams
unowned: the member reading view, the splitting of a Sources entry, and the
reference design as a gate.

### Cross-Cutting Themes

- **What the set page consists of** (flagged by: clarity, scope, testability,
  dependency) — "the set page", "the page", "the navigation" and "opened" fit
  two structures: a shell that hosts each member, or an index plus separate
  member pages. Neither this item nor the set model says which delivers the
  view that renders a member at its URL.
- **Sources-entry splitting has no owner** (flagged by: clarity, scope,
  dependency, completeness) — the page must split an entry into title, URL,
  tier text, venue and metadata block; the parser leaves that open and this
  item forbids tier-text parsing in the frontend without saying where
  splitting lives.
- **Blockers' open questions are this item's inputs** (flagged by:
  dependency, scope) — the member API shape, the member route shape and the
  parser's input are undecided upstream.
- **The reference design is a gate recorded nowhere as one** (flagged by:
  dependency, scope, completeness, clarity, testability) — it has no locator,
  no stated ordering, no place for the approval, and no criterion tying the
  built page to it.
- **Live change is under-specified** (flagged by: testability, scope) —
  removal from navigation, whether new content appears after the toast, and
  direct loading of a member URL have no criterion; one parent criterion on
  live content refresh is carried by no child.

### Findings

#### Critical

None.

#### Major

- 🟡 **Clarity, Scope**: The set page's structure has no single referent, and
  the member view is unowned
  **Location**: Requirements: Navigation, Live change
  The page "links to" each member yet "opens on" the `primary` document, and
  "the navigation" updates as if persistent. The set model gives every member
  a URL and assumes the manifest page is unchanged; neither item states which
  one renders a finding, level note or report at its URL.
- 🟡 **Clarity, Dependency, Scope**: No actor named for splitting a Sources
  entry
  **Location**: Requirements: Sources
  Nothing names the component that finds the Sources section, splits an entry
  and attaches the metadata block. If the parser takes tier text alone, this
  item silently gains a second grammar implementation.
- 🟡 **Dependency**: Blockers' unresolved open questions are decisions this
  page consumes
  **Location**: Dependencies
  Dependencies name the blockers but not what the page needs from each. The
  blockers can complete with the API shape, route shape and parser input
  answered in a way that leaves the page short of an interface.
- 🟡 **Dependency, Scope**: Reference design and author approval gate the
  build but are not a recorded dependency
  **Location**: Requirements: Reference design
  The design needs neither blocker and could start now, so the item reads as
  wholly blocked when its first deliverable is not. The approval, an action
  by a named person, can stall the work without showing on the record.
- 🟡 **Testability**: Live-change criteria leave three required outcomes
  unverified
  **Location**: Acceptance Criteria
  Removal from navigation, whether changed content is shown after the toast,
  and loading a member's URL directly have no criterion. A page that keeps
  deleted members listed or breaks deep links would pass.
- 🟡 **Testability**: Unparseable-manifest criterion does not define its
  input or how the page is reached
  **Location**: Acceptance Criteria
  Which failures count, whether such a set still has a library entry, and
  whether its members stay navigable are unstated.
- 🟡 **Scope**: Sources rendering is a separable slice with a different
  blocker
  **Location**: Requirements: Sources
  Eight of 34 criteria need only the parser; the rest need only the set
  model. Bundled, the whole L story waits on both.

#### Minor

- 🔵 **Clarity**: Profile states and merged focus areas are not mutually
  exclusive
  **Location**: Requirements: Outline and focus areas
  A profile with a finding that the brief omits is both researched and
  skipped; a merged focus area may carry two checkbox states.
- 🔵 **Clarity**: Reopened-report mark is triggered by `researching`, not by
  the stated definition of reopened
  **Location**: Requirements: Reports
  It is unstated whether stale and reopened marks can appear together.
- 🔵 **Testability**: Unreadable-member criterion bundles four cases
  **Location**: Acceptance Criteria
  "The reason" and "wrong directory" are undefined; "unreadable" covers a
  valid document.
- 🔵 **Testability**: The 60/600/10 fixture size has no effect on pass or
  fail
  **Location**: Acceptance Criteria
  The assertion holds with one finding and one note; "content" is undefined.
- 🔵 **Testability**: Two structural criteria can only be verified by
  inspection
  **Location**: Acceptance Criteria
  The join and no-frontend-parsing criteria assert the absence of code.
- 🔵 **Testability**: "Shown as absent" and "distinct element" have no
  observable form
  **Location**: Acceptance Criteria
- 🔵 **Testability**: Lineage-order example cannot tell lineage order from
  string order
  **Location**: Acceptance Criteria
  `1`, `2-1`, `2-2` sort the same either way; `2-2` against `2-10` does not.
- 🔵 **Testability**: Status and default-document criteria skip states the
  requirements name
  **Location**: Acceptance Criteria
  Three of five statuses, an absent `primary`, and a missing brief.
- 🔵 **Scope**: A server-side read model is carried under the name of a page
  **Location**: Technical Notes
  The join, outline parsing, freshness and member classification are server
  work the set model cannot own.
- 🔵 **Scope, Testability**: A parent criterion on live content refresh is
  carried by no child
  **Location**: Acceptance Criteria
- 🔵 **Scope, Dependency, Clarity**: Conditional scope from PP-923 is
  open-ended
  **Location**: Dependencies
- 🔵 **Dependency**: Member lookup for the active document is unowned
  **Location**: Technical Notes
- 🔵 **Dependency**: Reliance on existing `cli/research` code across a new
  crate edge is not in Dependencies
  **Location**: Dependencies
  Exports may need to change, which moves a pinned public-API snapshot.
- 🔵 **Dependency**: No Blocks entry although the story closes PP-22
  **Location**: Dependencies
- 🔵 **Completeness**: Context omits the current-state problem
  **Location**: Context
- 🔵 **Completeness, Testability**: Visual-regression baselines are sized but
  not required
  **Location**: Technical Notes
- 🔵 **Completeness, Clarity**: Reference design has no locator, approval
  slot or stated gate
  **Location**: Requirements: Reference design
- 🔵 **Completeness**: Open Questions is sparse relative to what is undecided
  **Location**: Open Questions
- 🔵 **Clarity**: "The three" has no antecedent in the tier criterion
  **Location**: Acceptance Criteria
- 🔵 **Clarity**: Several domain terms are undefined
  **Location**: Requirements
  Pair, lineage, `follow_ups`, merged tier, and member against sub-document.
- 🔵 **Clarity**: Manifest is listed as a document to read but absent from
  navigation
  **Location**: Summary

#### Suggestions

- 🔵 **Testability**: Tie the implemented page to the approved reference
  design
  **Location**: Acceptance Criteria
- 🔵 **Dependency**: Name the blockers by ID in the body
  **Location**: Dependencies
- 🔵 **Scope**: Re-kind PP-868 as an epic or thin container
  **Location**: Frontmatter: parent

### Strengths

- ✅ Criteria use literal frontmatter values, outline suffixes and tier text,
  so fixtures can be written from the text.
- ✅ Most criteria pair a positive case with a negative one.
- ✅ The reopened set is defined by observable frontmatter, and the join rule
  names its owner and states what does and does not match.
- ✅ Both blockers are recorded in frontmatter and body, and reciprocated.
- ✅ Scope edges are explicit: quarantined files, the run ledger, uninstalled
  profiles, asset bytes and focus-area identifiers are all out.
- ✅ Summary, Requirements and Acceptance Criteria line up group for group.

### Recommended Changes

1. **Say what the set page is** (addresses: structure has no referent, live
   change)
   State the structure once, the set's own URL, what opening a finding means,
   and whether this item or the set model delivers the member view.
2. **Assign Sources-entry splitting** (addresses: no actor for splitting)
   Resolve it with the parser item, then list it here or there as a
   requirement with a stated home.
3. **List what the page needs from each blocker** (addresses: blockers' open
   questions)
   Members with frontmatter and a malformed mark, a URL per member, a change
   event with the member's path, a structured tier value and the rejected
   text.
4. **Record the reference design as a gate** (addresses: reference design)
   A Dependencies entry: not blocked by either sibling, approval before
   implementation, where the approval is recorded, and a locator for the
   artifact. Consider lifting it into a small preceding item.
5. **Complete the live-change and manifest criteria** (addresses: live
   change, unparseable manifest)
   Add removal, content after the toast, and direct member URLs; name the
   manifest fixtures and the entry point.
6. **Decide on the Sources split** (addresses: separable slice)
   Split it out, or record why it stays.
7. **Tighten the remaining criteria and terms** (addresses: the minors)
   State precedence for profile states; align the reopened trigger; split the
   unreadable cases; add a `2-10` lineage case; define pair, lineage and
   `follow_ups`; fix "the three".

## Per-Lens Results

### Clarity

**Summary**: Precise on data rules. The structural referents are ambiguous.

**Strengths**: Reopened set defined; literal inputs; "skipped" bounded; join
rule owner named.

**Findings**:
- major, medium, Requirements: Navigation — "The set page", "the page", "the
  navigation" and "opened" have no single referent.
- major, medium, Requirements: Sources — No actor named for splitting a
  Sources entry.
- minor, high, Requirements: Outline and focus areas — Profile states and
  merged focus areas are not mutually exclusive as defined.
- minor, medium, Requirements: Reports — Reopened-report mark is triggered by
  `researching`, not by the stated definition of reopened.
- minor, high, Acceptance Criteria — "The three" has no clear antecedent in
  the tier criterion.
- minor, high, Requirements — Several domain terms are used without
  definition.
- minor, medium, Requirements: Reference design — The artifact, its export
  form and the approval record are not identifiable.
- minor, medium, Summary — Manifest is listed as a document to read but
  absent from the navigation list.
- minor, medium, Requirements: Unreadable members — "Unreadable" covers a
  valid document, and "wrong directory" is undefined.
- suggestion, medium, Dependencies — "This page handles them" states no
  outcome for PP-923.

### Completeness

**Summary**: Structurally complete and dense. All gaps are minor.

**Strengths**: All sections present; full user story; criteria track the
requirement groups; consistent frontmatter.

**Findings**:
- minor, medium, Context — Context omits the current-state problem the page
  solves.
- minor, high, Technical Notes — Visual-regression baselines are sized but
  not required.
- minor, medium, Requirements — Reference design has no locator, no approval
  slot and no stated gate.
- minor, medium, Open Questions — Open Questions is sparse relative to what
  the story leaves undecided.

### Dependency

**Summary**: The headline ordering is sound. The gaps are at the seams.

**Strengths**: Blockers recorded both ways; non-blocking drafts named with
reasons; follow-up marked as not a blocker.

**Findings**:
- major, high, Dependencies — Blockers' unresolved open questions are
  decisions this page consumes.
- major, high, Requirements: Reference design — Reference design and author
  approval gate the build but are not a recorded dependency.
- minor, medium, Technical Notes — Member lookup for the active document is
  unowned between this story and the set model.
- minor, medium, Dependencies — Reliance on existing `cli/research` code
  across a new crate edge is not in Dependencies.
- minor, high, Dependencies — No Blocks entry although the story closes
  PP-22.
- minor, medium, Dependencies — PP-923 is an open-ended conditional coupling.
- suggestion, high, Dependencies — Blockers are named by part label in the
  body, not by work item ID.

### Scope

**Summary**: One coherent purpose, but a large unit with a separable slice
and unsettled boundaries with its siblings.

**Strengths**: Sections align; explicit exclusions; generic model and parser
already split out; declared size.

**Findings**:
- major, medium, Requirements: Sources — Sources rendering is a separable
  slice with a different blocker from the rest of the page.
- major, medium, Requirements: Navigation / Live change — Ownership of the
  member document view is not settled between this story and the set model.
- minor, medium, Technical Notes — The story carries a server-side read model
  that the Summary frames as a page.
- minor, medium, Requirements: Sources — Splitting a Sources entry is unowned
  between this story and the parser.
- minor, medium, Requirements: Reference design — The reference design and
  its approval gate are bundled with the implementation they gate.
- minor, medium, Acceptance Criteria — A parent criterion on live content
  refresh is carried by no child.
- minor, medium, Dependencies — Conditional scope from PP-923 is open-ended.
- suggestion, low, Frontmatter: parent — A story parented by a story.

### Testability

**Summary**: Unusually testable. A few criteria have underspecified inputs or
outcomes.

**Strengths**: Explicit inputs; negative cases; subjective terms removed;
enumerated reference views.

**Findings**:
- major, medium, Acceptance Criteria — Unparseable-manifest criterion does
  not define its input or how the page is reached.
- major, medium, Acceptance Criteria — Live-change criteria leave three
  required outcomes unverified.
- minor, medium, Acceptance Criteria — Unreadable-member criterion bundles
  four cases and leaves "the reason" and "wrong directory" undefined.
- minor, high, Acceptance Criteria — The 60/600/10 fixture size has no effect
  on the criterion's pass or fail.
- minor, medium, Acceptance Criteria — Two structural criteria can only be
  verified by inspection.
- minor, medium, Acceptance Criteria — "Shown as absent" and "distinct
  element" have no defined observable form.
- minor, high, Acceptance Criteria — Lineage-order example cannot tell
  lineage order from plain string order.
- minor, medium, Acceptance Criteria — Status and default-document criteria
  skip some states the requirements name.
- suggestion, medium, Acceptance Criteria — Nothing ties the implemented page
  to the approved reference design.
- suggestion, medium, Dependencies — Conditional PP-923 obligation has no
  verifiable outcome.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-11-23

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Clarity, Scope**: The set page's structure has no single referent; member view unowned — Resolved
- 🟡 **Clarity, Dependency, Scope**: No actor for splitting a Sources entry — Resolved (moved to the Sources parser and PP-1000)
- 🟡 **Dependency**: Blockers' open questions are this page's inputs — Partially resolved (needs listed; three needs are not in the set model's criteria)
- 🟡 **Dependency, Scope**: Reference design not a recorded dependency — Resolved (PP-999)
- 🟡 **Testability**: Live-change criteria leave three outcomes unverified — Partially resolved (removal, content refresh and direct load added; derived state is not covered)
- 🟡 **Testability**: Unparseable-manifest criterion — Partially resolved (fixtures and entry point named; default member unstated)
- 🟡 **Scope**: Sources rendering is a separable slice — Resolved (split out)
- 🔵 **Clarity**: Profile-state precedence; "the three"; undefined terms — Resolved, except "focus area" (see below)
- 🔵 **Clarity**: Reopened-report trigger — Partially resolved (two detection rules remain)
- 🔵 **Testability**: Unreadable cases; lineage example; statuses — Resolved
- 🔵 **Testability**: 60/600/10 fixture has no effect on the verdict — Still present
- 🔵 **Testability**: Structural join criterion verifiable only by inspection — Still present
- 🔵 **Testability**: Marks and states without observable form — Still present
- 🔵 **Scope, Dependency**: PP-923 conditional scope — Resolved

### New Issues Introduced
- 🟡 **Scope, Dependency, Clarity, Testability**: The seven-view baseline criterion depends on Sources rendering, which is declared a non-blocker — view six shows tiered sources.
- 🟡 **Clarity**: "Focus area" is undefined and used both per outline item and across rounds.
- 🟡 **Clarity**: The join rule requires matching profiles, but placement uses the question alone.
- 🟡 **Testability**: Live-change criteria do not cover derived state a `conduct` run changes — status, stale marks, checkbox state, profile states.
- 🟡 **Scope**: The story bundles a page shell, the outline join, and live update.
- 🔵 **Completeness, Scope, Clarity**: Drafting Notes and References describe tier criteria that moved to PP-1000; Context still says "part C".
- 🔵 **Completeness**: Two criteria have no matching requirement (direct load; unknown frontmatter).
- 🔵 **Dependency, Scope**: Ownership of the `research` crate edge differs between siblings.
- 🔵 **Clarity**: Where level-note links appear is stated three ways.
- 🔵 **Testability**: Member URLs that do not resolve, and degraded brief and outline inputs, have no expected outcome.
- 🔵 **Completeness, Testability**: Removed and error states sit outside the baseline requirement; author acceptance has no recorded evidence.

### Assessment
The structural gaps from pass 1 are closed. The new majors come from rules
added during revision: the focus-area and join wording, and the baseline
criterion that crosses into PP-1000. Each needs a decision or a
definition, not new scope, apart from the author's call on splitting the
outline join.

## Re-Review (Pass 3) — 2026-11-23

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Scope, Dependency, Clarity, Testability**: View-six baseline depends on Sources rendering — Resolved (Sources region masked)
- 🟡 **Clarity**: "Focus area" undefined — Partially resolved (defined; one criterion and the pair definition still blur it)
- 🟡 **Clarity**: Join rule versus placement — Resolved
- 🟡 **Testability**: Live update of derived state — Resolved
- 🟡 **Scope**: Shell, join and live update bundled — Kept whole by the author; the recorded cut line is not a clean seam
- 🔵 **Completeness, Scope, Clarity**: Stale tier notes and part labels — Resolved
- 🔵 **Dependency, Scope**: Crate edge ownership — Resolved
- 🔵 **Clarity**: Two reopened rules — Resolved
- 🔵 **Testability**: Fixture size; marks without observable form — Still present

### New Issues Introduced
- 🟡 **Testability**: The first criterion says the page links to level notes on open, contradicting the navigation rule.
- 🟡 **Clarity, Testability**: "Six member kinds" in the requirement, "outside the seven" in the criterion.
- 🟡 **Clarity, Dependency, Scope**: Needs "from the set model beyond its stated criteria" have no owner; two of the three are already met there.
- 🟡 **Scope, Dependency**: A server-side derivation layer and `research` exports are implied but never scoped.
- 🟡 **Testability, Dependency, Clarity, Scope**: The Sources criterion is conditional on a sibling's landing order and sits under Visual regression.
- 🟡 **Testability**: The planner-parity criterion relies on an unnamed table and an unverifiable negative.
- 🔵 **Clarity, Dependency, Completeness**: Deliverables still attributed to the set model after the member-routes split; References omit PP-998.
- 🔵 **Clarity**: The error state's open document conflicts with `primary`.
- 🔵 **Clarity**: Lazy loading versus a level note loaded directly.
- 🔵 **Dependency**: The presentation declaration is not named as what the page reads; PP-923 hand-off is one-sided.
- 🔵 **Testability**: One baseline per view does not cover multi-state views; three criteria lack a precondition.

### Assessment
The earlier design gaps are closed. Two of the new majors are plain
contradictions introduced during revision; the rest are ownership between
this item and its blockers, and an unstated server deliverable.

## Changes After the Last Review Pass — 2026-11-23

The verdict above is the last review's. These changes were made afterwards
and have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- 🟡 **Testability**: First criterion and level-note links — Addressed.
- 🟡 **Clarity, Testability**: Six versus seven kinds — Addressed: six member kinds throughout.
- 🟡 **Clarity, Dependency, Scope**: Needs from the set model — Addressed: adopted there; the entry here is now a plain blocker.
- 🟡 **Scope, Dependency**: Server-side derivation — Addressed: a Server requirement group and a criterion on the response.
- 🟡 Sources criterion conditional on landing order — Addressed: its own heading, both branches stated.
- 🟡 **Testability**: Planner-parity table — Addressed: this story adds a named table both tests read.
- 🔵 Attributions after the split; References; inherited open questions — Addressed.
- 🔵 Error state's open document; lazy loading; pair definition; presentation declaration; PP-923 — Addressed.
- 🔵 Label list; preconditions on three criteria — Addressed.
- 🔵 Fixture size — Addressed: the request count on open must equal that of a one-finding set.
- Not addressed: one baseline per view where a view has sub-views (the design now letters them; this item still cites views by number).
- Kept by the author: the story stays whole.
