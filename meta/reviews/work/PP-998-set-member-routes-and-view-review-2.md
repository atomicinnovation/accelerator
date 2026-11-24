---
type: "work-item-review"
id: "PP-998-set-member-routes-and-view-review-2"
title: "Work Item Review: Set Member Routes and View"
date: "2026-11-23T21:57:00+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-998"
work_item_id: "PP-998"
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

## Work Item Review: Set Member Routes and View

**Verdict:** REVISE

A coherent, frontend-only story with explicit boundaries and strongly
testable criteria. Review 1's two majors (where member links lead, and the
needs from the set model) are resolved. Lifting the link fix into PP-996
left a new gap: this story builds on PP-996 and does not record it as a
blocker. Removal of an open member also has no stated outcome.

### Cross-Cutting Themes

- **PP-996 is an undeclared blocker** (flagged by: dependency, scope) —
  Requirements and Technical Notes build on PP-996's link builder, and
  retiring the filename URL is safe only after PP-996 lands.
- **Removal of an open member** (flagged by: testability, clarity,
  completeness, scope) — required "as for any document", with no stated
  outcome and no criterion. PP-1001 defines its own removed state.
- **The set slug upstream** (flagged by: dependency, clarity, scope) —
  Dependencies say each need is "now a criterion there"; the slug is a
  requirement of PP-994 with no criterion.
- **Unnamed units** (flagged by: clarity, completeness, testability) — "the
  two member-lookup hooks" and "the member view" scope a criterion and are
  never identified.

### Findings

#### Critical

None.

#### Major

- 🟡 **Dependency, Scope**: PP-996 is an implied upstream blocker recorded
  only as related
  **Location**: Dependencies; Frontmatter
  "Build member links with the slug-based link builder PP-996 introduces."
  If this story lands first, the builder does not exist and four surfaces
  link to a URL this story makes not-found.
- 🟡 **Testability, Clarity, Completeness, Scope**: Removal of an open
  member has no criterion or expected outcome
  **Location**: Requirements; Acceptance Criteria
  Not-found, a toast, or stale content: any can be claimed as passing.

#### Minor

- 🔵 **Dependency, Clarity, Scope**: "Each now a criterion there" does not
  hold for the set slug
  **Location**: Dependencies
- 🔵 **Dependency, Scope**: Upstream data for related-items and wiki-link
  member links is not captured
  **Location**: Requirements; Dependencies
  PP-994 does not mention wiki links. Server work could land in this M
  story.
- 🔵 **Dependency**: PP-134's wait on the route shape is missing from the
  planning order
  **Location**: Open Questions; Dependencies
  Drafting Notes say PP-134 waits "on the server work alone".
- 🔵 **Clarity, Completeness, Testability**: "The two member-lookup hooks"
  and "the member view" have no single referent
  **Location**: Requirements; Acceptance Criteria
  Technical Notes name one hook; `LibraryDocView` is a view. The
  identifier-absence criterion scans a set of files the item does not
  enumerate.
- 🔵 **Clarity**: "The navigation data" refers to nothing this story
  introduces
  **Location**: Acceptance Criteria
  It marks the observation point of a negative assertion.
- 🔵 **Clarity**: The root addressed through the member route sits
  awkwardly with the filename rule
  **Location**: Requirements
  Reconcilable, never reconciled; "as at the set's URL" could be read as a
  redirect.
- 🔵 **Testability**: "No link to any other member" can fail for a correct
  implementation
  **Location**: Acceptance Criteria
  A body wiki link or a related-items row to a sibling is legitimate.
- 🔵 **Testability**: The malformed-member criterion omits absent
  frontmatter and the exact title
  **Location**: Acceptance Criteria
  `doc.md`, `doc`, or the set-relative path.
- 🔵 **Testability**: The wiki-link criterion lacks the input that addresses
  a member
  **Location**: Acceptance Criteria
- 🔵 **Testability**: Root-filename not-found lacks a precondition and a
  non-set regression check
  **Location**: Acceptance Criteria
  A set whose directory is named `manifest` should still resolve.
- 🔵 **Completeness**: Technical Notes cover one of the three member-link
  surfaces
  **Location**: Technical Notes
- 🔵 **Scope, Clarity**: The Summary names two reader-visible changes;
  Requirements deliver five
  **Location**: Summary
  The retired URL, the one behaviour removed, is not visible at the top.

#### Suggestions

- 🔵 **Scope**: Retiring the filename-built set URL is separable
  **Location**: Requirements
  It could move into PP-996 as its closing step.
- 🔵 **Scope**: The plain member view may have no production consumer once
  the set page lands
  **Location**: Context
- 🔵 **Testability**: Compound criteria hide partial failure; the generic
  case omits related items and wiki links
  **Location**: Acceptance Criteria
- 🔵 **Clarity**: The "deliberately plain" paragraph is hard to parse and
  has a stray line break
  **Location**: Context
- 🔵 **Clarity**: A few terms lack a definition or link
  **Location**: Assumptions; Drafting Notes
  "Tier text", "a run", "the end of the filename-built URL".

### Strengths

- ✅ Every requirement serves one purpose: addressing a member document by
  URL.
- ✅ Context defines root, member, document, asset and set declaration, and
  the rest of the item uses them consistently.
- ✅ Not-found cases are enumerated individually.
- ✅ Negative toast assertions have a defined observation point.
- ✅ Genericity is verified behaviourally with a test-only doc type.
- ✅ Out-of-scope work is named with its owner, and redirects are excluded
  outright.
- ✅ Planning order is captured separately from delivery order.

### Recommended Changes

1. **Record PP-996 as a blocker** (addresses: PP-996 implied blocker)
   Add it to `blocked_by` and Dependencies, mirror it in PP-996, and add the
   edge to PP-868's diagram. Or move the not-found change into PP-996.
2. **State the outcome when an open member is removed and add a criterion**
   (addresses: removal of an open member)
   Or move removal wholly to PP-1001 and drop the clause.
3. **Correct "each now a criterion there"** (addresses: set slug)
   Pair it with the slug criterion review 2 of PP-994 recommends.
4. **Extend "Needs from the set model"** (addresses: related-items and
   wiki-link data)
   Name what the related response and wiki-link resolution supply, or state
   that this story derives both.
5. **Name the two hooks and the member view** (addresses: unnamed units)
   Give the identifier-absence criterion a file list or glob.
6. **Tighten five criteria** (addresses: navigation data; no link to
   another member; malformed title; wiki-link input; root-filename
   precondition)
7. **Add PP-134 to the planning-order bullet** (addresses: PP-134's wait)
8. **State that the root's set-relative path is valid on the member route
   and renders in place** (addresses: root through the member route)
9. **Extend the Summary's list of changes** (addresses: Summary)

## Per-Lens Results

### Clarity

**Summary**: Largely unambiguous; remaining ambiguities are local, and two
feed acceptance criteria.

**Findings**: minor — "the two member-lookup hooks" and "the member view"
(high); "the navigation data" (high); root through the member route;
removal outcome stated by analogy; "each now a criterion there".
suggestion — "deliberately plain" paragraph; undefined terms; Summary's
"two changes".

### Completeness

**Summary**: Structurally complete for a story; frontmatter is intact and
consistent.

**Findings**: minor — member-removal requirement has no outcome or
criterion; Technical Notes cover one of three link surfaces. suggestion —
the two hooks are never identified.

### Dependency

**Summary**: Main couplings are captured and reciprocated. The gap is
PP-996.

**Findings**: major — PP-996 is an implied blocker recorded as related
(high). minor — set slug has no criterion upstream; related-items and
wiki-link data uncaptured; PP-134 missing from planning order.

### Scope

**Summary**: One coherent, frontend-only increment; M fits.

**Findings**: minor — Summary understates the changes (high); wiki links
and the set slug not clearly owned; two requirements with no criterion
(high). suggestion — retiring the filename URL is separable; the plain view
may have no production consumer.

### Testability

**Summary**: Strongly testable. All URL criteria wait on the route-shape
question, which the item acknowledges.

**Findings**: major — removal of an open member has no criterion (high).
minor — "no link to any other member"; malformed-member criterion;
wiki-link input; identifier-absence scope; root-filename precondition and
non-set regression. suggestion — compound criteria.

---
*Review generated by /accelerator:review-work-item*

## Changes After the Review — 2026-11-23

The verdict above is the review's. These changes were made afterwards and
have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- 🟡 **Dependency**: PP-996 — Addressed: recorded as a blocker, mirrored in PP-996 and PP-868.
- 🟡 **Testability**: Removal of an open member — Addressed from today's behaviour: the toast says deleted and the page shows not-found, with a criterion.
- 🔵 Wiki links — Dropped as a surface by the author's decision: no wiki-link syntax names a set member. Member links come from search and related items.
- 🔵 Set slug upstream — Addressed in PP-994, which now has a criterion.
- 🔵 The two lookups and the member view — Named: `useActiveDocRelPath` and the lookup inside `LibraryDocView`; the identifier check lists files.
- 🔵 "Navigation data"; "no link to any other member"; malformed title; root-filename precondition; non-set regression; root on the member route; PP-134 planning order; Summary; Context — Addressed.
- Not addressed: compound criteria; the test-only doc type criterion does not cover related items; Technical Notes still anchor the search surface alone.

## Re-Review (Pass 2) — 2026-11-23

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Dependency**: PP-996 as an undeclared blocker — Resolved
- 🟡 **Testability**: Removal of an open member — Resolved (the deleted toast's text is still unquoted)
- 🔵 Set slug upstream; unnamed units; "navigation data"; malformed title; non-set regression; root on the member route; Summary — Resolved
- 🔵 Related-items data — Partially resolved (the slug is covered; the member's set-relative path on a related item is not)
- 🔵 PP-134 planning order — Partially resolved, now raised as major
- 🔵 "No link to any other member" — Partially resolved (inbound related items still unbounded)
- 🔵 Compound criteria; generic case omits related items — Still present

### New Issues Introduced
- 🟡 **Completeness, Clarity, Dependency, Scope, Testability**: Removing the activity feed's interim slug lookup is work this story owns, stated only in Dependencies, behind a condition that never fails (PP-994 is a blocker). All five lenses flagged it.
- 🟡 **Dependency**: PP-134's planning is gated on this story's route shape, yet PP-134 is "not a blocker", is absent from `blocks`, and is recorded in PP-868 and PP-994 as waiting on PP-994 alone.
- 🔵 **Clarity, Testability**: The root-filename precondition is stated on the directory name, but resolution is by slug, and an inventory's slug is a shortened form.
- 🔵 **Clarity, Scope**: "Topic research is the plain view's only production use" sits against PP-994 classing a markdown file in an inventory as a member document.
- 🔵 **Completeness, Dependency, Testability**: The route shape has no recorded home, and no criterion can name a literal member URL or an encoded path.
- 🔵 **Testability**: No criterion for a deeper path beneath a non-set doc type; the root-through-member-route criterion omits "no redirect".
- 🔵 **Clarity**: Literal tier text is tied to PP-1001 landing, though PP-1000 ends it.

### Assessment
Both earlier majors are closed. The lookup removal was a hand-off I recorded in Dependencies in this pass without a requirement; it needs one, or a different owner. The PP-134 gate is a pre-existing inconsistency across four items that this review is the first to rate major.

## Changes After the Re-Review — 2026-11-23

Made after pass 2 and not re-reviewed; the verdict above is the reviewers'.

- 🟡 Lookup removal — Gone: the lookup is never removed.
- 🟡 PP-134's planning gate — Stated here as a planning-order wait; PP-134 and PP-868 are not yet updated to match.
- 🔵 Slug preconditions; deeper path under a non-set type; no redirect; where the route shape is recorded; plain view's production use — Addressed.

## Approval — 2026-11-24

**Verdict:** APPROVE

Approved by the author; not re-reviewed after the changes above.
