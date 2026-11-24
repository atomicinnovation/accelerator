---
type: "work-item-review"
id: "PP-1000-sources-rendering-in-the-visualiser-review-1"
title: "Work Item Review: Sources Rendering in the Visualiser"
date: "2026-11-23T17:39:12+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-1000"
work_item_id: "PP-1000"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-11-24T00:06:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Sources Rendering in the Visualiser

**Verdict:** REVISE

The item is a coherent vertical slice with named actors and literal inputs in
most criteria. Its weaknesses are at its edges with three siblings: the set
page (which view hosts the rendering, and who owns the sixth-view baseline),
the set model (the API shape it rides on), and the reference design (which
covers fewer states than this item renders).

### Cross-Cutting Themes

- **Integration with the set page** (flagged by: scope, dependency, clarity,
  testability) — "can land before or after" hides who wires the rendering
  into the set page's member view and who owns the sixth-view baseline.
- **Transport undecided** (flagged by: dependency, testability, scope,
  completeness) — how structured sources reach the frontend waits on the set
  model's API shape, so the API criterion has nothing to inspect.
- **States without a design or an outcome** (flagged by: dependency,
  testability, completeness) — withdrawn mark, absent venue, rejected tier,
  `None found.`, a missing Sources section, and unread lines.

### Findings

#### Critical

None.

#### Major

- 🟡 **Scope, Dependency**: The sixth-view baseline is owned by both this
  story and the set page
  **Location**: Requirements, Dependencies
  Whichever lands second must wire the rendering into the other's view and
  re-take the baseline; neither item says who.
- 🟡 **Dependency**: Dependence on the set model's member API shape is not a
  recorded planning gate
  **Location**: Open Questions
  The shape can be settled with only the set page and PP-134 in view.
- 🟡 **Testability**: API criterion has no defined endpoint and an
  unverifiable negative
  **Location**: Acceptance Criteria
  Titles and venues are themselves Sources text, and the item never says
  whether the served body keeps the raw section.
- 🟡 **Testability**: Non-topic-research criterion omits the discriminating
  input
  **Location**: Acceptance Criteria
  Only a non-topic-research document with a tier-shaped Sources section
  exercises the scoping.
- 🟡 **Testability**: A missing Sources section and unread lines have no
  expected outcome
  **Location**: Acceptance Criteria

#### Minor

- 🔵 **Testability, Clarity**: The merged-tier criterion's outcome fits
  neither of its inputs
  **Location**: Acceptance Criteria
- 🔵 **Testability**: "Distinct element" and marker content lack an
  observable definition
  **Location**: Acceptance Criteria
- 🔵 **Testability**: Rejected-tier and metadata criteria lack concrete
  inputs
  **Location**: Acceptance Criteria
- 🔵 **Completeness, Clarity**: The metadata requirement says where the block
  belongs, not what is shown
  **Location**: Requirements
- 🔵 **Clarity, Testability, Scope**: "Wherever shown" names no views, and
  "for every profile" is undefined
  **Location**: Context, Requirements
- 🔵 **Dependency**: The reference design's sixth view covers fewer states
  than this story renders
  **Location**: Dependencies
- 🔵 **Completeness, Scope**: Open questions have no resolution path and set
  the size
  **Location**: Open Questions
- 🔵 **Scope**: The `research` crate edge has no single owner
  **Location**: Dependencies
- 🔵 **Clarity**: Several terms are undefined (profile, merged tier, depth-1)
  **Location**: Requirements

#### Suggestions

- 🔵 **Completeness**: Add the three blockers to References
- 🔵 **Scope**: Reword parser-owned behaviour as rendering requirements
- 🔵 **Testability**: Tie the baseline to the approved design; add a
  criterion for `tier-1 (retracted)`
- 🔵 **Clarity**: Say who made the topic-research-only scoping call

### Strengths

- ✅ One purpose, a full vertical slice, and an M size consistent with it.
- ✅ Context restates the entry grammar; actors are explicit.
- ✅ Each blocker states what it supplies.
- ✅ Colour independence is a checkable outcome.

### Recommended Changes

1. **Settle the set-page integration** (addresses: sixth-view baseline)
   Put the rendering in a component both views share; give the tiered-sources
   baseline to this story.
2. **Record the API shape as a planning gate** and name this story on the set
   model's open question.
3. **Rewrite the API criterion** with a fixture once the transport is known,
   and state whether the served body keeps the Sources section.
4. **Add the missing outcomes**: non-topic-research fixture, no Sources
   section, unread lines, per-input merged-tier results, metadata display.
5. **Extend the design's sixth view** or record which states are this
   story's own call.

## Per-Lens Results

### Clarity
**Summary**: Clearly written; ambiguities are local.
**Findings**: minor — merged-tier criterion; undefined terms; "receives no
Sources text"; "wherever shown"; metadata outcome. suggestion — "my call";
two sentences that parse two ways.

### Completeness
**Summary**: Structurally complete.
**Findings**: minor — metadata display; open questions lack a resolution
path. suggestion — References omit the blockers.

### Dependency
**Summary**: Blockers named with reasons; gaps sit between siblings.
**Findings**: major — two-way integration with the set page; set model API
shape not a planning gate. minor — sixth view covers fewer states; set model
prose omits this story. suggestion — upstream grammar skills.

### Scope
**Summary**: One coherent vertical slice; boundary with the set page is weak.
**Findings**: major — sixth-view baseline owned twice. minor — host depends
on landing order; open questions; crate edge owner. suggestion — parser-owned
requirements.

### Testability
**Summary**: Core rendering is verifiable; three criteria are not.
**Findings**: major — API criterion; non-topic-research criterion; missing
section and unread lines. minor — merged-tier outcome; "distinct element";
rejected-tier and metadata inputs; "wherever shown". suggestion — baseline
conformance; rule-breaking text.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-11-23

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Scope, Dependency**: Sixth-view baseline owned twice — Resolved
- 🟡 **Dependency**: API shape not a planning gate — Partially resolved (recorded; no owner for the wire decision)
- 🟡 **Testability**: API criterion — Partially resolved (fixture added; the response is unnamed and omits title, URL, metadata and unread lines)
- 🟡 **Testability**: Non-topic-research criterion — Resolved
- 🟡 **Testability**: Missing section and unread lines — Resolved
- 🔵 **Testability, Clarity**: Merged-tier outcomes; metadata display — Resolved
- 🔵 **Scope**: Crate edge owner — Resolved

### New Issues Introduced
- 🟡 **Clarity, Dependency**: The member view is attributed to the set model; it belongs to PP-998, and References omit that blocker.
- 🟡 **Testability**: "Element" and "mark" have no observable definition, so tier criteria pass against the literal text.
- 🟡 **Testability, Completeness, Scope, Dependency**: The shared component and its wiring into the set page have no criterion, and landing order changes the baseline.
- 🔵 **Clarity, Testability**: Which topic-research documents get structured Sources is stated three ways.
- 🔵 **Clarity**: "As written" means raw text in one place and structured elements in another.
- 🔵 **Dependency, Scope**: Topic-research-only rendering meets the siblings' "never name a doc type" rule with no stated mechanism.
- 🔵 **Testability**: Notice plural wording, order and initial state; empty-state text; rejected tier without "no tier elements".
- 🔵 **Completeness**: Drafting-agent calls await the author's confirmation.
- 🔵 **Scope, Clarity**: Summary names fewer elements than Requirements deliver.

### Assessment
The seam with the set page is settled in principle but lacks a criterion.
The largest gap is that "separate element" is never made observable.

## Changes After the Last Review Pass — 2026-11-23

The verdict above is the last review's. These changes were made afterwards
and have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- 🟡 **Clarity, Dependency**: Member view attribution — Addressed: PP-998 throughout.
- 🟡 **Testability**: "Element" and "mark" — Addressed: an element is separately addressable and its whole text is its label; the written tier text appears nowhere in a parsed entry.
- 🟡 Shared component and wiring — Addressed: observation at the member URL, and a conditional criterion for the set page with the baseline re-taken unmasked.
- 🔵 Which documents — Confirmed by the author: topic research only, decided on the server; a brief with Sources renders structurally.
- 🔵 "As written"; wire fields; no second rendering; plural notice; empty-state text; rejected tier; two-entry metadata — Addressed.
- 🔵 Wire-contract owner; Summary; terms; acceptance record — Addressed.
- Unread lines: shown in a collapsed notice, by the author's decision.
- The drafting agent's calls on this item were confirmed by the author on 2026-11-23.

## Approval — 2026-11-24

**Verdict:** APPROVE

Approved by the author; not re-reviewed after the changes above.
