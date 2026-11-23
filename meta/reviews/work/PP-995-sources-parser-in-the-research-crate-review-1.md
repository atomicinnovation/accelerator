---
type: "work-item-review"
id: "PP-995-sources-parser-in-the-research-crate-review-1"
title: "Work Item Review: Tier-Text Parser in the Research Crate"
date: "2026-11-23T16:16:47+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-995"
work_item_id: "PP-995"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-11-23T21:18:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Tier-Text Parser in the Research Crate

**Verdict:** REVISE

The item is a coherent, well-bounded unit: one pure parser, literal inputs in
most criteria, and nothing blocking it. Three decisions are still open, and
each changes what gets built: what the parser takes as input, where the line
between tier text and not-tier-text falls, and what round-tripping means. All
five lenses reached at least one of them.

### Cross-Cutting Themes

- **Input boundary undecided** (flagged by: completeness, scope, dependency,
  clarity, testability) — whether the parser reads a whole Sources entry or
  tier text alone is an open question. It decides the deliverable, the size,
  and whether the set page must split entries itself.
- **Accept/reject boundary undecided** (flagged by: clarity, testability) —
  the item does not say whether the parser checks syntax only or also the
  rule's constraints, so `tier-1 (retracted)`, `tier-3 ()`,
  `tier-3 (also tier-3)` and out-of-order parts have no stated outcome.
- **Round-trip depends on the above** (flagged by: clarity, testability,
  scope) — a value of a base tier, two marks and an `also` list cannot render
  back to text whose parts were written out of order.

### Findings

#### Critical

None.

#### Major

- 🟡 **Completeness, Scope, Dependency**: Parser input boundary is unresolved
  **Location**: Open Questions
  The first open question asks whether the parser reads a whole Sources entry
  or tier text alone, and nothing answers it. Requirements, criteria and the S
  size assume tier text alone, while entry splitting has no stated owner
  between this item and the set page.
- 🟡 **Clarity, Testability**: Accept/reject boundary is undecided for
  near-miss forms
  **Location**: Requirements, Acceptance Criteria
  "Text that is not tier text" has no single boundary. A mark on a tier other
  than `tier-3`, an empty suffix, a repeated mark, an `also` tier equal to the
  base, an `also` list in the wrong order, and spacing or case variants all
  lack an outcome; two parsers that disagree on every one would pass.
- 🟡 **Clarity, Testability**: Round-trip criterion presupposes an answer to
  the ordering question
  **Location**: Acceptance Criteria
  "A parsed value renders back to the text it was parsed from" names no inputs
  and cannot hold for accepted non-canonical text. It can be read as silently
  deciding the open question or as applying to canonical text only.

#### Minor

- 🔵 **Testability, Clarity**: "Every tier-text example in the rule" is not
  enumerated
  **Location**: Acceptance Criteria
  The verifier must find the examples and infer each parsed value, and it is
  unclear whether the worked example's whole-entry lines count.
- 🔵 **Testability**: "No other Rust code parses tier text" has no check
  **Location**: Requirements
  The statement is already true before any work, so it verifies nothing here.
- 🔵 **Clarity**: "An error that carries the text" does not say which text
  **Location**: Requirements
  The whole input unchanged, or the part that failed to parse.
- 🔵 **Scope**: Summary promises one reading for every consumer; two parsers
  remain
  **Location**: Summary
  The Python eval parser stays, so the item delivers a single Rust owner.
- 🔵 **Dependency**: Three holders of one grammar with no synchronisation
  recorded
  **Location**: Assumptions
  The skill, the Python parser and the Rust parser can drift apart unnoticed.
- 🔵 **Dependency, Clarity**: Blocks entry names its consumer by description
  **Location**: Dependencies
  The set page is PP-1001; the link is recorded on that side only.
- 🔵 **Scope**: Round-trip adds a rendering capability to a parsing item
  **Location**: Acceptance Criteria
  It was added during decomposition and no consumer is named for it.

#### Suggestions

- 🔵 **Completeness**: Add `relates_to` for PP-892 and PP-864
  **Location**: Frontmatter: relates_to
- 🔵 **Dependency**: Carry down the parent's note that PP-891 and PP-897 do
  not change the grammar
  **Location**: Dependencies
- 🔵 **Clarity**: "Own the grammar" mixes an outcome with a standing invariant
  **Location**: Requirements
- 🔵 **Scope**: Consider `kind: task` for an enabling component
  **Location**: Frontmatter: kind
- 🔵 **Testability**: State that the error lets a caller recover the whole
  input unchanged
  **Location**: Acceptance Criteria

### Strengths

- ✅ Context restates the grammar in full, so the item reads without opening
  the citing-once rule.
- ✅ Five of seven criteria give literal inputs and exact structured outcomes,
  which translate directly into table-driven tests.
- ✅ One purpose, one crate, nothing blocking it, and an S size with a stated
  rationale.
- ✅ The crate edge from the visualiser is recorded and assigned to the set
  page; the import allow-list and public-API snapshot are noted.
- ✅ Drafting Notes say which criteria go beyond the parent's.

### Recommended Changes

1. **Decide the input boundary** (addresses: parser input boundary)
   State in Requirements that the parser takes tier text alone and name the
   owner of splitting a Sources entry, or widen the title, requirements,
   criteria and size to cover the whole entry.
2. **State the validation rule** (addresses: accept/reject boundary)
   Say whether the parser checks syntax only or also the rule's constraints,
   and give one accept-or-reject outcome for each near-miss class.
3. **Scope or drop round-trip** (addresses: round-trip criterion)
   Decide the ordering question, then limit round-trip to canonical text and
   say what accepted non-canonical text renders to, or drop the criterion.
4. **Enumerate the rule's examples** (addresses: "every example" not
   enumerated)
   List the distinct tier-text strings with their expected base tier, marks
   and `also` tiers.
5. **Tidy the links** (addresses: blocks entry, relates_to, grammar holders)
   Name PP-1001, add `relates_to`, and record the skill and the Python
   parser as peers that must agree.

## Per-Lens Results

### Clarity

**Summary**: Tightly written, with terms introduced before use and literal
inputs in the criteria. The ambiguity sits at the grammar's edge.

**Strengths**: Grammar defined in Context; literal inputs; open decisions
stated openly; resolvable file references.

**Findings**:
- major, high, Acceptance Criteria — Round-trip criterion presupposes an
  answer to the open ordering question.
- major, medium, Requirements — "Text that is not tier text" has no single
  boundary.
- minor, high, Open Questions — Scope and actor for splitting a Sources entry
  are unsettled across sections.
- minor, medium, Acceptance Criteria — "Every tier-text example" and "the
  value the rule describes" are loose referents.
- minor, medium, Requirements — "An error that carries the text" does not say
  which text.
- suggestion, medium, Requirements — "Own the grammar" mixes an outcome with
  a standing invariant.
- suggestion, high, Dependencies — Blocked item is described but not
  identified.

### Completeness

**Summary**: Structurally complete for a story. One open question decides the
shape of the deliverable.

**Strengths**: Summary names who, what and why; seven specific criteria;
populated Dependencies, Assumptions and Technical Notes.

**Findings**:
- major, high, Open Questions — Parser input boundary is unresolved, so
  Requirements do not define the deliverable.
- minor, medium, Open Questions — Unresolved suffix-order question leaves
  rejection and round-trip partly undefined.
- suggestion, medium, Frontmatter: relates_to — Related items appear in
  References but not in frontmatter.

### Dependency

**Summary**: Couplings are mostly captured. The contract with the one
consumer is undecided.

**Strengths**: Both directions stated; crate edge assigned; in-repo
constraints noted; independent of the set model.

**Findings**:
- major, medium, Open Questions — Parser's contract with its one consumer is
  undecided and not tied to that consumer.
- minor, high, Dependencies — Blocks entry names its consumer by description,
  not by ID.
- minor, medium, Assumptions — Three holders of one grammar with no recorded
  synchronisation coupling.
- suggestion, medium, Dependencies — Parent's note on still-draft tier work
  is not carried down.

### Scope

**Summary**: Coherent and atomic. The outer boundary is the risk.

**Strengths**: One purpose; one crate; Python parser boundary stated; S size
justified; independently deliverable.

**Findings**:
- major, medium, Open Questions — Outer boundary undecided: tier text alone
  or a whole Sources entry.
- minor, medium, Summary — Summary promises one reading for every consumer;
  scope leaves two parsers.
- minor, medium, Acceptance Criteria — Round-trip criterion adds scope whose
  size depends on an open question.
- suggestion, medium, Frontmatter: kind — Enabling component filed as a
  story.

### Testability

**Summary**: Strong criteria for a parser story. Weak at the edges.

**Strengths**: Literal inputs and exact outcomes; `also` order pinned;
concrete negative inputs; pre-rule forms covered.

**Findings**:
- major, high, Acceptance Criteria — Round-trip criterion has no defined
  input set and depends on an open question.
- major, medium, Acceptance Criteria — Accept/reject boundary is undecided
  for several near-miss forms.
- minor, high, Acceptance Criteria — "Every tier-text example in the rule" is
  not enumerated and its expected values are implicit.
- minor, high, Requirements — "No other Rust code parses tier text" has no
  criterion or check.
- minor, medium, Open Questions — Whole-entry parsing is unresolved, with no
  criteria if it is in scope.
- suggestion, medium, Acceptance Criteria — "Error carrying the text" could
  state the observable property precisely.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-11-23

**Verdict:** COMMENT

The item is now "Sources Parser in the Research Crate", widened to the whole
Sources section. Work item is acceptable but could be improved — see the
major finding below.

### Previously Identified Issues
- 🟡 **Completeness, Scope, Dependency**: Parser input boundary is unresolved — Resolved (the parser reads the whole Sources section)
- 🟡 **Clarity, Testability**: Accept/reject boundary undecided for near-miss forms — Resolved (shape only, with named accepted and rejected texts)
- 🟡 **Clarity, Testability**: Round-trip criterion presupposes the ordering answer — Resolved (strict order; scoped to accepted texts)
- 🔵 **Testability**: "Every tier-text example" not enumerated — Resolved
- 🔵 **Clarity**: "An error that carries the text" — Resolved
- 🔵 **Scope**: Summary promises one reading for every consumer — Resolved
- 🔵 **Dependency**: Blocks entry names its consumer by description — Resolved
- 🔵 **Dependency**: Three holders of one grammar — Partially resolved (the finding outputter is not named as an upstream definition)
- 🔵 **Testability**: "No other Rust code parses tier text" has no check — Partially resolved (criterion replaced; the requirement sentence remains)

### New Issues Introduced
- 🟡 **Testability**: Skip-and-report rule does not say what happens to blank lines — read literally, every blank line is reported as unread; `None found.` beside entries is also unstated.
- 🔵 **Clarity, Testability**: The line between a rejected-tier entry and a skipped line is not stated — a link with no tier segment, a venue containing ` — `, and trimming are unpinned.
- 🔵 **Clarity, Testability**: The Sources section's extent is defined by one example with no observable outcome — which headings open and close it is unstated.
- 🔵 **Clarity**: Whether a block line must directly follow its entry is unstated.
- 🔵 **Testability**: A metadata value containing a colon (`pdf_url`) has no expected result.
- 🔵 **Testability**: Two suffix combinations are unexercised, and the second rejection set drops "recovers the whole input unchanged".
- 🔵 **Clarity**: "The parser" and "the grammar" have two referents since the widening.
- 🔵 **Clarity**: Round-trip asks for rendering that Requirements never mention.
- 🔵 **Scope, Dependency**: Unread-line reporting has no committed consumer.
- 🔵 **Dependency**: Three children change the same public-API snapshot; PP-924 is not mentioned.
- 🔵 **Completeness**: Open Questions is blank, not "None"; the rejected-tier reading sits only in Drafting Notes.
- 🔵 **Clarity, Completeness**: Context still says "part B of a three-way split".

### Assessment
The three decisions that blocked the item are settled. What remains is
section-level precision: blank lines, the entry boundary, and the section's
extent. None blocks planning, and each is a small rule plus a criterion.

## Re-Review (Pass 3) — 2026-11-23

**Verdict:** COMMENT

Work item is acceptable but could be improved — see the major finding below.

### Previously Identified Issues
- 🟡 **Testability**: Blank lines under the skip-and-report rule — Resolved
- 🔵 **Clarity, Testability**: Entry versus skipped-line boundary — Resolved (line rules), with residual edge cases below
- 🔵 **Clarity, Testability**: The Sources section's extent — Resolved, except a second `## Sources` heading
- 🔵 **Clarity**: Block-line adjacency — Resolved
- 🔵 **Testability**: Metadata value containing a colon; missing suffix combinations — Resolved
- 🔵 **Scope, Dependency**: Unread-line reporting has no consumer — Resolved
- 🔵 **Completeness**: Open Questions blank — Resolved
- 🔵 **Clarity**: "The parser" has two referents — Still present

### New Issues Introduced
- 🟡 **Dependency**: The parser returns no entries both for `None found.` and for a document with no Sources section, yet its consumer must show an empty state for one and nothing for the other.
- 🔵 **Testability, Completeness, Clarity**: Order and exact text of unread lines are stated only in Dependencies.
- 🔵 **Testability**: "No trimming", the key shape, and the first-`: ` split have no criterion that would fail.
- 🔵 **Testability, Clarity**: Edge inputs undetermined: two `## Sources` sections, a trailing ` — `, an empty block value, heading variants.
- 🔵 **Testability**: Separator strictness inside the suffix is untested.
- 🔵 **Clarity**: "Tier" stands for the base tier, the tier text, and the parsed result.
- 🔵 **Clarity**: "The tier-text parser's test cases are the rule's examples" reads as the full set.
- 🔵 **Dependency, Clarity**: Described as agreeing with the Python parser while diverging from it on purpose.
- 🔵 **Dependency**: The consumer's wire-shape question may need the section's extent.
- 🔵 **Scope**: Boundary with the existing `Record` type unstated; Summary names one parser.
- 🔵 **Completeness**: Two drafting-agent calls await the author's confirmation.

### Assessment
One contract gap with the consumer remains; everything else is edge-case
precision. The item is close to ready.

## Changes After the Last Review Pass — 2026-11-23

The verdict above is the last review's. These changes were made afterwards
and have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- 🟡 **Dependency**: `None found.` versus no section — Addressed: the parser reports a section state (absent, declared empty, present).
- 🔵 Unread-line order and exact text — Addressed: a separate list in written order, with a literal criterion.
- 🔵 "No trimming", key shape, first-`: ` split — Addressed with criteria.
- 🔵 Edge inputs — Addressed: only the first `## Sources` section is read, the heading matches exactly, a trailing ` — ` gives no venue, an empty value is unread.
- 🔵 Separator strictness — Addressed: five more rejected forms.
- 🔵 Two parsers named; "tier" terminology; test cases "include" the rule's examples — Addressed.
- 🔵 Divergence from the Python parser — Addressed: shared surface and three intended differences listed.
- 🔵 Wire-shape feedback; `Record` boundary; PP-924 condition; links — Addressed.
- 🔵 Kind — Changed to `task`.
- The drafting agent's calls on this item were confirmed by the author on 2026-11-23.
