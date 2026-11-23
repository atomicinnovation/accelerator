---
type: "work-item-review"
id: "PP-996-links-to-a-research-set-or-design-inventory-open-the-wrong-review-1"
title: "Work Item Review: Links to a Research Set or Design Inventory Open the Wrong One"
date: "2026-11-23T21:57:00+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-996"
work_item_id: "PP-996"
reviewer: "Toby Clemson"
verdict: "REVISE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-11-23T23:44:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Links to a Research Set or Design Inventory Open the Wrong One

**Verdict:** REVISE

A coherent, atomic bug of plausible S size, with a concrete reproduction and
a well-drawn boundary against PP-998. Three things keep it from ready: PP-998
consumes this fix and is not recorded as blocked by it, the design-inventory
criterion hangs on an unanswered question, and neither the defect nor the
assumption the fix rests on has been run.

### Cross-Cutting Themes

- **Design-inventory surfaces undecided** (flagged by: testability, clarity,
  completeness, scope) — the open question asks which of the four surfaces
  can show a design inventory, and the fifth criterion defers to it.
- **Read from code, not run** (flagged by: testability, completeness, scope,
  dependency) — the actual outcome and the "slug without a server change"
  assumption are both untried, and the assumption has no fallback. S size
  and "blocked by: nothing" rest on it.
- **The shared link builder** (flagged by: clarity, scope, testability,
  dependency) — "can share" leaves open whether search and the type listing
  move onto it, no criterion observes it, and PP-998 depends on it.

### Findings

#### Critical

None.

#### Major

- 🟡 **Dependency**: PP-998 consumes this fix's link builder but is recorded
  as "Blocks: nothing"
  **Location**: Dependencies; Frontmatter
  PP-998 requires "the slug-based link builder PP-996 introduces" and makes
  the filename URL not-found. If it lands first, four surfaces link to a
  not-found page. PP-868's sequencing diagram has no node for this bug.
- 🟡 **Testability, Clarity, Completeness, Scope**: The design-inventory
  criterion has no fixed set of surfaces
  **Location**: Acceptance Criteria; Open Questions
  "Where a design inventory can appear on them" could be met by testing one
  surface or none. Requirements assert the defect on a lifecycle view that
  the open question says may not apply.
- 🟡 **Testability, Completeness**: The reproduction is unconfirmed and
  specified for one surface
  **Location**: Requirements; Drafting Notes
  The three other surfaces are covered by "the same happens". The actual
  result depends on `a` listing before `b`, and the ordering is unstated.

#### Minor

- 🔵 **Scope, Dependency, Completeness, Testability**: The untried slug
  assumption has no fallback, and no criterion covers a failed lookup
  **Location**: Assumptions
  If the lookup fails on a surface, the fix needs the server change PP-994
  owns. A fallback to the filename link would pass every criterion.
- 🔵 **Clarity, Scope, Testability**: The extent of the shared link builder
  is not bounded
  **Location**: Requirements
  Four call sites or six. PP-998 requires the builder to branch on the set
  declaration, which the frontend cannot read until PP-994.
- 🔵 **Dependency**: Ordering against PP-994 changes what this fix builds
  **Location**: Dependencies
  "Later" assumes this lands first. No item owns removing the interim
  lookup.
- 🔵 **Clarity**: Bare "set" sometimes includes design inventories
  **Location**: Acceptance Criteria
  Read against the item's own usage, criterion 7 says an inventory's links
  are "unchanged", contradicting criterion 5.
- 🔵 **Testability**: Three surface criteria omit the expected URL and the
  fixture
  **Location**: Acceptance Criteria
  Only the activity criterion asserts an exact link.
- 🔵 **Testability**: The "unchanged" regression criterion has no baseline
  **Location**: Acceptance Criteria

#### Suggestions

- 🔵 **Clarity**: "Lists first" does not say which ordering decides
  **Location**: Summary
- 🔵 **Clarity, Testability**: The form of a wiki link that targets a set is
  not stated
  **Location**: Requirements

### Strengths

- ✅ The reproduction states trigger, expected and actual with literal URLs.
- ✅ Context explains the cause: a fixed entry filename versus a directory
  slug, with the four wrong surfaces and the two correct ones named.
- ✅ Every topic-research criterion uses two sets, so a first-match accident
  cannot pass.
- ✅ The boundary with PP-998 is stated the same way in both items.
- ✅ Assumptions and Drafting Notes say plainly what was read from code and
  not run.
- ✅ Technical Notes give file and line for the four call sites, the
  resolver, and the two correct call sites.

### Recommended Changes

1. **Record the edge to PP-998** (addresses: PP-998 consumes this fix)
   Add `blocks: ["work-item:PP-998"]` here, PP-996 to PP-998's `blocked_by`
   and Dependencies, and an edge to PP-868's diagram.
2. **Run the reproduction and try the slug lookup** (addresses:
   reproduction unconfirmed; untried assumption)
   Record the observed result per surface. State the fallback and its effect
   on size and order if the lookup fails.
3. **Answer the open question and enumerate the surfaces** (addresses:
   design-inventory criterion)
   Name the surfaces in criterion 5 and qualify the Requirements sentence.
4. **Bound the link builder** (addresses: extent of the builder)
   Say whether search and the type listing move onto it, add a checkable
   criterion, and say how it recognises a set before PP-994.
5. **Give each surface criterion an exact URL and a fixture** (addresses:
   three surface criteria; "unchanged" baseline; wiki-link form)
6. **Define "set" once** (addresses: bare "set")
7. **State the order against PP-994 and the owner of the interim lookup's
   removal** (addresses: ordering against PP-994)

## Per-Lens Results

### Clarity

**Summary**: Clear on the whole; the defect, cause and surfaces are named
once and used consistently.

**Findings**: minor — bare "set" versus design inventories (high); "can
share" (high); design-inventory criterion depends on an open question.
suggestion — "lists first"; wiki-link target form.

### Completeness

**Summary**: Structurally complete for a bug; all four reproduction elements
are present.

**Findings**: minor — actual outcome inferred, not observed (high); open
question leaves a criterion's coverage undetermined. suggestion — untried
assumption has no fallback.

### Dependency

**Summary**: Self-contained frontend bug; both neighbours are named. The gap
is the undeclared edge to PP-998.

**Findings**: major — PP-998 consumes the link builder but "Blocks: nothing"
(high). minor — no-server-change assumption has an uncaptured fallback on
PP-994; ordering against PP-994 is only "Related".

### Scope

**Summary**: Coherent and atomic; kind and size fit.

**Findings**: minor — size and boundary rest on an untried assumption; link
builder's extent unbounded; design-inventory surface set undecided (high).

### Testability

**Summary**: Largely verifiable, with exact URLs and a two-set fixture.

**Findings**: major — design-inventory criterion has no fixed surfaces
(high); reproduction unconfirmed and one-surface (medium). minor — three
criteria omit URL and fixture; no criterion for a failed slug lookup;
"unchanged" has no baseline. suggestion — link-builder requirement has no
criterion.

---
*Review generated by /accelerator:review-work-item*

## Changes After the Review — 2026-11-23

The verdict above is the review's. These changes were made afterwards and
have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- The item was rewritten after the code was read. Three surfaces are reachable, not four: the activity feed and related items for both types, lifecycle views for inventories alone. A wiki link names an ADR or a work item and cannot target either type.
- 🟡 **Dependency**: PP-998 — Addressed: `blocks` PP-998, mirrored there and in PP-868's diagram.
- 🟡 **Testability**: Design-inventory surfaces — Addressed: the open question is closed and each criterion names its surfaces and URLs.
- 🟡 **Testability**: Reproduction — Partially addressed: every reachable case has a trigger and the path ordering is stated. It is still read from code, not run, and now says so beside the steps.
- 🔵 Link builder — Addressed by the author's decision: one builder, the entry's slug with a filename fallback, at all six call sites, with a criterion.
- 🔵 Failed lookup — Addressed by the author's decision: an activity row whose document is gone renders with no link.
- 🔵 Slug assumption — Narrowed: three call sites already hold the slug; only the activity feed looks it up. A fallback and the order against PP-994 are stated.
- 🔵 "Set"; "lists first"; baselines — Addressed.

## Re-Review (Pass 2) — 2026-11-23

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Dependency**: PP-998 edge undeclared — Resolved
- 🟡 **Testability**: Design-inventory criterion has no fixed surfaces — Resolved
- 🟡 **Testability**: Reproduction unconfirmed — Still present (every case now has a trigger and says it is read from code; nothing has been run)
- 🔵 Untried slug assumption — Partially resolved (narrowed to the activity feed; its fallback now conflicts with "Blocked by: nothing")
- 🔵 Link builder extent; bare "set"; "lists first"; exact URLs — Resolved
- 🔵 "Unchanged" baseline — Partially resolved (the wiki-link and search criteria still say "as before")

### New Issues Introduced
- 🟡 **Clarity, Dependency, Scope, Testability**: The activity-feed lookup is unconditional in Requirements and conditional in Dependencies. If PP-994 lands first there is no lookup, so the deleted-row rule has no mechanism, and a plan's row carries no slug at all.
- 🟡 **Testability**: No criterion exercises the builder's rule where slug and filename differ for a non-set type. The plan criterion passes on today's code and under either branch.
- 🔵 **Clarity**: The builder "takes an index entry", but a search row and an activity event are not one.
- 🔵 **Scope**: No link on a deleted row changes every doc type's behaviour and has no durable owner once PP-998 removes the lookup.
- 🔵 **Clarity**: "Called by the builder alone among the six" could forbid `LibraryDocView`'s own call.
- 🔵 **Testability**: The deleted-row criterion has no observation point; fixtures leave titles and the inventory slug implicit; "names no doc type" has no criterion.
- 🔵 **Dependency**: The interim lookup relies on PP-994 keeping the row identified by its root; its removal is not a criterion in PP-998.

### Assessment
The rewrite fixed the defect's extent and the PP-998 edge. The decisions taken in this pass (one builder everywhere, no link on a deleted row) introduced the two new majors: both need the fix to commit to one behaviour whichever of PP-994 and PP-996 lands first.

## Changes After the Re-Review — 2026-11-23

Made after pass 2 and not re-reviewed; the verdict above is the reviewers'.

- 🟡 Lookup conditional on landing order — Addressed by the author's decision: the type-listing lookup is the fix in both orders and is never removed.
- 🟡 Slug rule untested for a non-set type — Addressed with a criterion.
- 🔵 Builder input; "among the six"; observation point; explicit URLs; builder names no doc type — Addressed.
- Not addressed: the reproduction is still unrun.
