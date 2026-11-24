---
type: "work-item-review"
id: "PP-1002-derived-set-response-for-topic-research-review-1"
title: "Work Item Review: Derived Set Response for Topic Research"
date: "2026-11-24T00:10:00+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-1002"
work_item_id: "PP-1002"
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

## Work Item Review: Derived Set Response for Topic Research

**Verdict:** REVISE

A coherent, server-only story, split from PP-1001 at a real seam, with
concrete criteria and reciprocal dependencies. It was drafted in one pass
from PP-1001's Server section, and three gaps show it: several required
rules belong to none of the five exports the story is sized on, the
staleness criteria cannot tell its two rules apart, and the response under
a set-level error is half specified.

### Cross-Cutting Themes

- **Rules outside the five exports** (flagged by: scope, clarity,
  completeness, dependency, testability) — pair state, lineage order, the
  reopened mark, and three of the four unplaced reasons have no named
  export. The kind-and-directory rule has no stated source and may not
  exist in `research` today. The Summary says the server "writes none of
  its own".
- **The set-level error response** (flagged by: testability, clarity) —
  "everything that does not depend on the manifest" is not enumerated, and
  the manifest-dependent fields have no expected value.
- **Freshness of the response** (flagged by: scope, dependency) — PP-1001
  needs derived state to update live and derives nothing itself. Nothing
  here says the response reflects the index at request time.
- **Terms defined only in PP-1001** (flagged by: clarity, completeness) —
  the six member kinds, the five statuses, root and member.

### Findings

#### Critical

None.

#### Major

- 🟡 **Scope, Clarity, Completeness, Dependency**: Several required rules
  have no named export, contradicting "writes none of its own"
  **Location**: Summary; Requirements: The rules; Technical Notes
  The implementer must write them in the visualiser server, against the
  story's purpose, or add them to `research`, breaking the criterion that
  the snapshot shows five exports. Size M may be understated.
- 🟡 **Testability**: The staleness criteria confound the two synthesis
  causes and never test the report's "no other reason" rule
  **Location**: Acceptance Criteria
  The only stale fixture is both reopened and behind on rounds, for the
  synthesis and the report alike. Marking a report stale whenever the set
  is reopened would pass.
- 🟡 **Testability, Clarity**: The set-level error criterion leaves the
  manifest-dependent fields with no expected value
  **Location**: Acceptance Criteria; Requirements: The rules
  Status, the stale marks and the reopened mark could be absent, false or
  computed. With a bad `status` and a valid `round_count`, staleness by
  count is still computable.

#### Minor

- 🔵 **Scope, Dependency**: Ownership of keeping the response current is
  not placed
  **Location**: Requirements: The response; Dependencies
  A cached response would pass every criterion and break PP-1001's
  live-change criteria.
- 🔵 **Testability**: No criterion separates "belongs to a focus area" from
  "answers a pair"
  **Location**: Acceptance Criteria
  An `openalex` finding for a `web`-only item, and two findings for one
  pair, are untested here, where they must be true.
- 🔵 **Testability, Completeness, Clarity**: The kind-and-directory rule is
  tested by one example and its mapping is never stated
  **Location**: Requirements: The rules; Acceptance Criteria
- 🔵 **Clarity, Completeness**: Enumerations and terms are defined only in
  PP-1001
  **Location**: Context
  "`kind: manifest` outside the root" reads as a directory;
  "no `.levels/` directory" reads as a dot-prefixed one.
- 🔵 **Completeness**: A behavioural rule sits only in Dependencies
  **Location**: Dependencies
  A finding with an unknown field is placed as any other.
- 🔵 **Testability**: The 60/600/10 fixture has no bearing on its outcome
  **Location**: Acceptance Criteria
  "Carries no document body" passes the same on one finding and names no
  probe.
- 🔵 **Testability**: Several rules have no fixture that would fail them
  **Location**: Acceptance Criteria
  Checkbox states and written order, parent before child, an unreadable
  outline, a set with no brief.
- 🔵 **Clarity**: The first criterion's fixture leaves two inputs to
  inference
  **Location**: Acceptance Criteria
  The brief's `source_profiles`, and `round_count: 2` beside one round.
- 🔵 **Dependency**: How the server reads `outline.md`'s body is not among
  what PP-994 supplies
  **Location**: Dependencies
- 🔵 **Dependency**: Planning-order gates sit in Open Questions
  **Location**: Open Questions; Dependencies
  PP-994's planning-order list omits this story.
- 🔵 **Dependency**: PP-923 is understated and missing from `relates_to`
  **Location**: Dependencies; Frontmatter
  PP-997 says PP-923 changes how findings relate to focus areas.
- 🔵 **Scope, Dependency**: The crate edge makes PP-1000 wait on unrelated
  work
  **Location**: Requirements: The crate edge
- 🔵 **Testability**: "Shows the five exports" names one symbol
  **Location**: Acceptance Criteria

#### Suggestions

- 🔵 **Clarity**: "The page" is named as the actor that calls the rules
  **Location**: Summary
- 🔵 **Scope**: Kind `story` for a no-UI slice whose sibling parser is a
  `task`
  **Location**: Frontmatter
- 🔵 **Dependency**: PP-997 does not record that it retires this story's
  table; ADR-0069 and `cli/pup.ron` are uncited
  **Location**: Dependencies
- 🔵 **Testability**: "Writes none of its own" is verified for the question
  comparison alone
  **Location**: Summary; Acceptance Criteria

### Strengths

- ✅ One coherent, server-only increment, split from the page at a seam
  that both sides state the same way.
- ✅ Placement is defined at two levels, each with one exact condition, and
  the pair-state rule states its precedence.
- ✅ Criteria give literal fixtures: three dash forms, lineage `2-2` before
  `2-10`, the same question in rounds 1 and 3.
- ✅ The question-comparison table gives the planner and the visualiser one
  stated oracle.
- ✅ Ownership of the crate edge is settled outright.
- ✅ Blockers and consumers are reciprocated in PP-994, PP-1001, PP-1000,
  PP-995 and PP-868.

### Recommended Changes

1. **Say where each rule lives** (addresses: rules outside the five exports;
   kind-and-directory rule)
   For each rule: an existing `research` rule exported, a new `research`
   rule, or server assembly. Align the Summary, the export count and Size.
   State the kind-to-directory mapping.
2. **Separate the staleness cases** (addresses: staleness criteria)
   A reopened set with equal counts: synthesis stale, report not. A
   `synthesised` set behind on rounds: both stale, not reopened.
3. **Enumerate the error-case response** (addresses: set-level error)
   For each of the nine items: present or absent, and its value, for
   unclosed frontmatter and for an out-of-range `status`.
4. **State that the response reflects the index at request time**
   (addresses: freshness)
   Name PP-1001 as the owner of refetching on a change event.
5. **Add the placement criteria** (addresses: belongs versus answers; rules
   with no fixture)
6. **Add the kinds, statuses, root and member to Context** (addresses:
   terms)
7. **Move the unknown-field rule into Requirements** (addresses: rule in
   Dependencies)
8. **Add a planning-order line; name the body access; widen PP-923**
   (addresses: dependency findings)

## Per-Lens Results

### Clarity

**Summary**: Tightly written; the main ambiguity is where rules outside the
five exports live.

**Findings**: major — where rules outside the five exports are written
(medium). minor — set-level error not enumerated; shorthand enumerations;
kind-and-directory rule has no source; first criterion's fixture;
"cannot be read". suggestion — "the page" as actor.

### Completeness

**Summary**: Structurally complete; the gaps are information in the wrong
place or left to siblings.

**Findings**: minor — kind-to-directory rule neither stated nor located;
kinds and statuses never listed; a rule only in Dependencies. suggestion —
PP-923 missing from `relates_to`.

### Dependency

**Summary**: Couplings are mapped well and reciprocated; nothing would stop
the work starting.

**Findings**: minor — outline body access; fifth export's source; PP-1001's
live-update expectation; planning-order gates; PP-923 in `relates_to`;
PP-923 understated. suggestion — PP-997 reciprocity; governing rules for
the crate edge.

### Scope

**Summary**: One coherent increment with agreed boundaries.

**Findings**: major — rules with no named export (medium). minor — crate
edge bundled with the response; freshness ownership. suggestion — kind
`story`.

### Testability

**Summary**: Unusually concrete; the weaknesses are discrimination and
coverage.

**Findings**: major — staleness criteria confound causes (high); set-level
error fields (medium). minor — fixture size; belongs versus answers;
kind-and-directory oracle; rules with no fixture; "five exports".
suggestion — "writes none of its own".

---
*Review generated by /accelerator:review-work-item*

## Changes After the Re-Review — 2026-11-23

Made after pass 2 and not re-reviewed; the verdict above is the reviewers'.

- 🟡 Rules outside the five exports — Addressed by the author's decision: every rule lives in `research`, five exported and three written new; size L; the layout is a table, to be checked at planning.
- 🟡 Staleness criteria — Addressed with separated cases.
- 🟡 Set-level error — Addressed by the author's decision: whatever reads the manifest is absent, for both error inputs.
- 🔵 Per-request derivation; placement criteria; terms; planning order; PP-923; marker-string check — Addressed.

## Approval — 2026-11-24

**Verdict:** APPROVE

Approved by the author; not re-reviewed after the changes above.
