---
type: "work-item-review"
id: "PP-998-set-member-routes-and-view-review-1"
title: "Work Item Review: Set Member Routes and View"
date: "2026-11-23T19:19:10+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-998"
work_item_id: "PP-998"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 1
tags: []
last_updated: "2026-11-24T00:06:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Set Member Routes and View

**Verdict:** REVISE

A coherent, frontend-only story of plausible M size, with a clean seam
against the set model and explicit exclusions. It leaves one behaviour
undecided, where a link to a member leads from related items and wiki
links, and it relies on things from the set model that the set model does
not promise.

### Cross-Cutting Themes

- **Links to a member** (flagged by: clarity, completeness, testability) —
  the item fixes where links to a set go, but not where a related-items row
  or wiki link for a member goes.
- **Needs from the set model** (flagged by: dependency) — a member body
  fetchable by path, the set declaration readable by the frontend, the
  search-row locator, and the set slug on four surfaces.
- **Residue of the split** (flagged by: scope, dependency, clarity) —
  siblings still credit the set model with this story's deliverables.

### Findings

#### Critical

None.

#### Major

- 🟡 **Clarity, Testability, Completeness**: Whether related items and wiki
  links ever open a member is unstated
  **Location**: Requirements, Assumptions, Drafting Notes
  Assumptions say search and direct links are the only ways to a member;
  Drafting Notes say the view exists so "related" has something to observe.
- 🟡 **Dependency**: Needs from the set model beyond the three named are not
  captured
  **Location**: Dependencies
  The member API returns no bodies, and no criterion there serves a member
  document's body or exposes the set declaration to the frontend.
- 🟡 **Testability**: The naming criterion may pass vacuously
  **Location**: Acceptance Criteria
  It forbids the identifiers but not the string keys `topic-research` and
  `design-inventories`, and covers two units of the six in scope.
- 🟡 **Testability**: Several member-URL input classes have no outcome
  **Location**: Acceptance Criteria
  The root addressed as a member, a member with malformed frontmatter, an
  unknown set, and removal while open.

#### Minor

- 🔵 **Scope**: The wrong-set link fix is independent of member routes and
  waits behind an L blocker
- 🔵 **Scope, Dependency, Clarity**: Siblings attribute this story's
  deliverables to the set model
- 🔵 **Scope, Dependency**: Asset-route scope is handed to PP-134, which
  does not claim it; PP-134 is absent from Dependencies
- 🔵 **Clarity, Completeness**: The route shape is presented as open and
  partly decided
- 🔵 **Clarity**: "As it is today" sits beside an unstated behaviour change
- 🔵 **Clarity**: Which story starts query invalidation on member events
- 🔵 **Clarity**: "No toast on the root during a run" is over-broad
- 🔵 **Dependency**: The upstream planning-order constraint is not mirrored
- 🔵 **Testability**: Live-update criterion omits "without a reload" and
  the path form
- 🔵 **Testability**: The set-link criterion bundles sixteen checks
- 🔵 **Testability**: The test-only doc type criterion verifies routes only
- 🔵 **Testability**: "As before" and "no other set navigation" lack an
  observable

#### Suggestions

- 🔵 **Scope**: Say what of the plain member view outlives the set page
- 🔵 **Completeness**: Add `relates_to` for PP-134
- 🔵 **Clarity**: Define root, member and asset in Context
- 🔵 **Testability**: Record worked example URLs once the shape is settled

### Strengths

- ✅ One layer, one seam, explicit exclusions, and an M size that fits.
- ✅ Each requirement traces to a criterion, including the negative ones.
- ✅ The toast criterion pairs a positive case with a negative control.
- ✅ The set-link criterion targets today's failure directly.
- ✅ The interim state before the set page is stated.

### Recommended Changes

1. **Decide where a link to a member leads** from related items and wiki
   links, and add a criterion.
2. **Record the needs from the set model**, and add them there as criteria.
3. **Tighten the genericity check**: forbid the string keys, list every
   unit in scope, and widen the test-only doc type criterion.
4. **Give each member-URL input class an outcome.**
5. **Correct the attributions** in PP-1001 and PP-1000.
6. **Decide on the link fix**: say what couples it to the route work, or
   lift it into a small unblocked bug.

## Per-Lens Results

### Clarity
**Summary**: Tightly written; one contradiction over member links.
**Findings**: major — links to a member. minor — route shape open and
decided; "as it is today"; query invalidation; toast on the root; asset as
a member URL; scope of the naming rule; ambiguous criterion phrases.
suggestion — terms defined only in siblings.

### Completeness
**Summary**: Structurally complete.
**Findings**: minor — non-search surfaces. suggestion — open question lacks
candidates; PP-134 missing from frontmatter.

### Dependency
**Summary**: Core couplings mapped; needs from the blocker are not.
**Findings**: major — needs from the set model. minor — planning order;
PP-134. suggestion — sibling attributions.

### Scope
**Summary**: Coherent and M-sized; one independent concern bundled.
**Findings**: minor — link fix; sibling attributions; asset route.
suggestion — interim view.

### Testability
**Summary**: Mostly concrete; three gaps.
**Findings**: major — naming criterion; links to a member; member-URL input
classes. minor — live update; set-link bundle; test-only type; "as before".
suggestion — route shape.

---
*Review generated by /accelerator:review-work-item*

## Changes After the Last Review Pass — 2026-11-23

The verdict above is the last review's. These changes were made afterwards
and have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- 🟡 **Clarity, Testability, Completeness**: Links to a member — Addressed by the author's decision: a member link opens the member from search, related items, and wiki links.
- 🟡 **Dependency**: Needs from the set model — Addressed: listed here and adopted there as criteria.
- 🟡 **Testability**: Naming criterion — Addressed: string keys forbidden, every unit listed.
- 🟡 **Testability**: Member-URL input classes — Addressed with one outcome each.
- 🔵 **Scope**: Wrong-set link fix — Lifted into its own bug, PP-996; this story keeps only the end of the filename-built URL.
- 🔵 Route shape narrowed; set URL resolution stated positively; query invalidation and toast claims corrected; planning order; PP-134 — Addressed.
- 🔵 Live-update, test-only doc type, "as before", no other set navigation — Addressed.
- Not addressed: worked example URLs, which wait on the route shape.

## Approval — 2026-11-24

**Verdict:** APPROVE

Approved by the author; not re-reviewed after the changes above.
