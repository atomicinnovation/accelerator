---
type: "work-item-review"
id: "PP-1000-sources-rendering-in-the-visualiser-review-2"
title: "Work Item Review: Sources Rendering in the Visualiser"
date: "2026-11-23T21:57:00+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-1000"
work_item_id: "PP-1000"
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

## Work Item Review: Sources Rendering in the Visualiser

**Verdict:** REVISE

A coherent vertical slice with literal inputs and outputs in almost every
criterion. Review 1's majors are closed: "element" is defined and the seam
with the set page has criteria. The new gaps are at boundaries: which
documents are covered, what the parser's less common states render as, a
need the parser explicitly does not meet, and a baseline neither story owns
in one landing order.

### Cross-Cutting Themes

- **Which documents** (flagged by: clarity, testability, completeness,
  scope) — four kinds in the Summary, "any kind" in Requirements, a brief in
  the last criterion.
- **Baseline content** (flagged by: clarity, completeness, scope,
  testability) — the requirement lists four cases, the criterion eight, and
  neither names sub-views 6a, 6b and 6c.
- **The unmasked view 6 baseline** (flagged by: dependency, scope) — unowned
  when this story lands before PP-1001. The PP-1001 review found the same.
- **Wire-shape ownership** (flagged by: clarity, dependency, scope) — Open
  Questions point at PP-994, Dependencies say this story owns it, and
  Requirements already presuppose the document response.
- **Section-state outcomes** (flagged by: testability, clarity) — present
  with no entries, and declared empty with unread lines, have no stated
  rendering.

### Findings

#### Critical

None.

#### Major

- 🟡 **Clarity, Testability, Completeness, Scope**: The set of documents
  covered is stated three ways
  **Location**: Summary; Requirements; Acceptance Criteria
  "Topic-research document" is never defined, and "kind" and "doc type" are
  not related. The brief criterion's outcome, "renders structurally, as a
  finding's does", has no observable.
- 🟡 **Testability, Clarity**: State combinations the parser can return have
  no stated outcome
  **Location**: Acceptance Criteria; Requirements
  A section with only unread lines, `None found.` plus an unread line, and a
  heading with nothing under it. The empty state is specified twice with
  different triggers.
- 🟡 **Dependency**: Suppressing the literal Sources section needs a section
  location PP-995 says it does not return
  **Location**: Dependencies
  The server would re-implement PP-995's section-boundary rules: a second
  reading of the grammar.
- 🟡 **Dependency, Scope**: The unmasked view 6 baseline in the set page has
  no owner when this story lands first
  **Location**: Acceptance Criteria; Dependencies
  This story's criterion then "passes on the member URL alone", on a view
  PP-1001 replaces for topic research.
- 🟡 **Clarity, Completeness, Scope, Testability**: Visual-regression
  baseline content differs between Requirements and Acceptance Criteria
  **Location**: Requirements; Acceptance Criteria
  Four cases against eight. "View 6" is three sub-views, and "the revision
  compared" does not say a revision of what.

#### Minor

- 🔵 **Clarity, Dependency, Scope**: Who settles the wire-shape question
  reads differently in two sections
  **Location**: Open Questions; Dependencies
- 🔵 **Clarity, Testability**: "Element", "separately addressable" and "the
  written tier text appears nowhere" are not pinned down
  **Location**: Requirements; Acceptance Criteria
  For a bare `tier-2` the written text is the label, so the member-URL
  criterion fails literally. Carried over from review 1 in part.
- 🔵 **Testability**: A rejected tier has no observable that tells it from a
  tier element
  **Location**: Acceptance Criteria
- 🔵 **Testability**: "As written" has no pass condition for whitespace or
  markdown-looking text
  **Location**: Acceptance Criteria
  `### Notes` and `  - doi: 10.1/x` could be rendered as markdown.
- 🔵 **Testability**: The response criterion does not pin rejected tier,
  unread lines or section state
  **Location**: Acceptance Criteria
- 🔵 **Testability**: Three requirements are implementation constraints with
  no criterion
  **Location**: Requirements
  "The frontend parses no entry text", "no view names a doc type", "one
  component".
- 🔵 **Testability**: Empty-state text is deferred to an export that does
  not exist
  **Location**: Acceptance Criteria
  The notice text is given literally here; the empty state is not.
- 🔵 **Scope**: Part of the scope is conditional on landing order
  **Location**: Dependencies; Technical Notes
  Wiring, the baseline, and the crate edge. Size M describes the
  unconditional part.
- 🔵 **Dependency**: The PP-999 entry omits the approval gate
  **Location**: Dependencies
- 🔵 **Clarity**: The entry format is written with a `drafts/url` target
  **Location**: Context; Acceptance Criteria
  The same artefact as in PP-995.
- 🔵 **Clarity**: "Re-takes view 6's baseline there unmasked" depends on
  context only PP-1001 gives
  **Location**: Acceptance Criteria

#### Suggestions

- 🔵 **Completeness, Dependency**: PP-864, PP-892 and PP-936 are listed
  without a relationship and are absent from `relates_to`
  **Location**: References; Frontmatter
- 🔵 **Clarity**: The notice wording is plural only in Requirements
  **Location**: Requirements
- 🔵 **Clarity**: "Blocks ... together with the set page" has two readings
  **Location**: Dependencies
- 🔵 **Testability**: No criterion asserts the title links to the source URL
  **Location**: Acceptance Criteria
- 🔵 **Testability**: The set-page criterion's pass condition depends on
  landing order
  **Location**: Acceptance Criteria

### Strengths

- ✅ Tier criteria give literal inputs and enumerate the expected elements,
  their count and their order.
- ✅ Negative outcomes sit beside positive ones.
- ✅ The server contract is verifiable field by field, apart from the
  frontend.
- ✅ Actors are named: the server calls the parser, the frontend parses no
  entry text.
- ✅ Grammar parsing is excluded, so the story does not overlap PP-995.
- ✅ All four blockers say what they supply and are reciprocated.
- ✅ Drafting Notes separate the author's decisions from the drafting
  agent's calls, with dates.

### Recommended Changes

1. **Define "topic-research document" once** (addresses: which documents)
   List the member kinds that get structured sources and those that do not.
   Give the brief criterion a concrete input and outcome.
2. **Key the empty state on the section state and state all outcomes**
   (addresses: state combinations)
   Present with entries, present with none, present with only unread lines,
   declared empty with unread lines, absent.
3. **Resolve section removal with PP-995** (addresses: section location)
   Add a span or body-without-Sources export to PP-995, or state that this
   story owns removal and is coupled to PP-995's boundary rule.
4. **Give the second story to land the unmasked set-page baseline**
   (addresses: unmasked view 6 baseline)
   Mirror it in PP-1001. Better, fix the landing order.
5. **One baseline content list, named by sub-view** (addresses: baseline
   content)
   Say whether 6b and 6c need baselines and what "the revision" is.
6. **Name who decides the wire shape and when** (addresses: wire-shape
   ownership)
7. **Tighten the element rules** (addresses: element; rejected tier; "as
   written")
   Give the "appears nowhere" criterion a suffixed input. Say what
   distinguishes rejected text. Add a markdown-shaped unread line.
8. **Pin the response for rejected tier, unread lines and section state**
   (addresses: response criterion)
9. **State the empty-state text here; replace `drafts/url`** (addresses:
   empty-state text; entry format)
10. **Give the three constraints a check or move them to Technical Notes**
    (addresses: implementation constraints)

## Per-Lens Results

### Clarity

**Summary**: Unusually precise for a rendering story; ambiguity concentrates
in a few places.

**Findings**: major — documents covered stated three ways (high); baseline
content differs (high). minor — empty state specified twice; wire-shape
ownership; "element" not pinned; `drafts/url`; "unmasked". suggestion —
plural notice wording; "Blocks ... together with".

### Completeness

**Summary**: Structurally complete. The reviewer searched the sibling items
by term and did not open the two skill files.

**Findings**: minor — baseline requirement lists fewer cases than its
criterion (high); Summary omits the brief. suggestion — three references
without a relationship.

### Dependency

**Summary**: Couplings are mapped thoroughly and reciprocated; two gaps
remain.

**Findings**: major — section location PP-995 does not return (high);
unmasked baseline unowned (medium). minor — wire-shape ownership stated two
ways; PP-999 entry omits the approval gate. suggestion — `relates_to` omits
three items.

### Scope

**Summary**: One coherent slice; the boundary with PP-1001 is the weak
point.

**Findings**: minor — unmasked baseline unowned (high); scope conditional on
landing order; baseline requirement narrower than criterion (high); Summary
names four kinds; wire-shape ownership split.

### Testability

**Summary**: Unusually concrete; the gaps are at the edges.

**Findings**: major — documents not enumerated and the brief criterion has
no observable (high); state combinations have no outcome (high). minor —
"as written"; rejected tier indistinguishable; three constraints unchecked;
baseline content; empty-state text deferred; "appears nowhere" fails
literally; response criterion. suggestion — title link; order-dependent
criterion.

---
*Review generated by /accelerator:review-work-item*

## Changes After the Review — 2026-11-23

The verdict above is the review's. These changes were made afterwards and
have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- 🟡 **Clarity, Testability**: Documents covered — Addressed: a topic-research document is defined as any document of a set, and the first criterion names five kinds.
- 🟡 **Testability**: Section states — Addressed by the author's decision: a six-row rule, a bare heading showing the empty state, and the text "No sources found.".
- 🟡 **Dependency**: Section location — Addressed by the author's decision: PP-995 reports a line span and the server drops the section by it.
- 🟡 **Dependency, Scope**: Unmasked view 6a baseline — Addressed: the second story to land takes it, mirrored in PP-1001.
- 🟡 **Clarity**: Baseline content — Addressed by the author's decision: three baselines, for views 6a, 6b and 6c.
- 🔵 Wire-shape ownership; "appears nowhere"; rejected tier label; unread lines shown literally; response fields; payload-only frontend; title link; notice wording; PP-999 gate; `drafts/url`; `relates_to` — Addressed.
- 🔵 Crate edge — Now owned by the derived set response, PP-1002, which blocks this story.
- Not addressed: the set-page criterion still has two branches by landing order.

## Re-Review (Pass 2) — 2026-11-23

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Clarity, Testability**: Documents covered stated three ways — Resolved (the root at the set URL and the outline still have no criterion)
- 🟡 **Testability**: Section states with no outcome — Resolved (one table row lists the notice unconditionally)
- 🟡 **Dependency**: Section location PP-995 does not return — Resolved
- 🟡 **Dependency, Scope**: Unmasked view 6a baseline unowned — Partially resolved (6a has an owner in both orders; 6b and 6c do not, see below)
- 🟡 **Clarity**: Baseline content differs — Resolved
- 🔵 Wire-shape ownership; "as written"; response fields; empty-state text; `drafts/url`; PP-999 gate — Resolved
- 🔵 "Element" not pinned — Partially resolved ("separately addressable" is still undefined)
- 🔵 Three constraints with no check — Partially resolved (one behavioural criterion added; "names no doc type" has no procedure)
- 🔵 Scope conditional on landing order — Still present

### New Issues Introduced
- 🟡 **Clarity, Scope, Completeness**: The three baselines have no stated surface. View 6 is a set-page design; if this story lands first only the plain member view exists, and if PP-1001 lands first no separate member view exists for topic research. Views 6b and 6c have no owner in the set page.
- 🟡 **Testability**: Dropping the literal Sources section has no criterion on the served body or on placement. A truncated body, sources moved to the foot, or a second literal `## Sources` section would all pass.
- 🟡 **Testability, Clarity, Completeness**: The rejected-tier label is taken from view 6a of an export that is not required to supply one. PP-999's review found the same gap.
- 🔵 **Clarity**: The served-fields requirement omits the section state and the rejected tier text; the shared-component sentence can be read as naming `LibraryDocView` as the component.
- 🔵 **Dependency**: The served body changes for other consumers (PP-998's view, PP-1001's page); the reverse coupling through PP-999's approval gate and the author's sign-off are unrecorded.
- 🔵 **Dependency, Scope**: PP-1002 blocks this story for one crate dependency, and reverting it would break this story.
- 🔵 **Testability**: Unscoped negatives ("no element's text holds `(`", "no warning"); the baseline criterion checks that files exist, not that a spec compares them.

### Assessment
The five earlier majors are closed or nearly so. The new majors all sit at seams this pass created or exposed: three baselines without a surface, a body drop without a criterion, and a label promised by a neighbour that does not owe it. The landing-order flexibility with PP-1001 is the root of the first; fixing the order would remove it.

## Changes After the Re-Review — 2026-11-23

Made after pass 2 and not re-reviewed; the verdict above is the reviewers'.

- 🟡 Baselines without a surface — Addressed by the author's decision: this story is blocked by PP-1001, and all three baselines are taken in the set page.
- 🟡 Body drop — Addressed: a criterion on the served body, with the structured Sources shown in place.
- 🟡 Rejected-tier label — Addressed: the text "unrecognised tier" is fixed here.
- 🔵 Served fields; table row; element definition; sign-off and body consumers in Dependencies — Addressed.
- Drafting agent's calls awaiting the author: Sources shown in place; the label's text.
