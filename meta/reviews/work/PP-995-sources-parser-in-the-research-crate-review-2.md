---
type: "work-item-review"
id: "PP-995-sources-parser-in-the-research-crate-review-2"
title: "Work Item Review: Sources Parser in the Research Crate"
date: "2026-11-23T21:57:00+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:PP-868"
target: "work-item:PP-995"
work_item_id: "PP-995"
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

## Work Item Review: Sources Parser in the Research Crate

**Verdict:** REVISE

The item is one coherent, well-bounded task and nearly every rule is pinned
by a literal input. Review 1's contract gap (section state) is closed. Two
defects sit in the one rule that decides entry versus unread line: the entry
shape is written `(drafts/url)`, and the title and URL have no grammar. Both
are small edits; the verdict follows the two-major threshold.

### Cross-Cutting Themes

- **Entry shape `drafts/url`** (flagged by: clarity, dependency,
  testability) — Context and the normative entry rule write
  `- [title](drafts/url) — `, while both skills write `(url)` and every
  criterion uses `https://`. It looks like a path-rewrite artefact; PP-1000
  carries the same text.
- **Title, URL and key shapes undefined under exact matching** (flagged by:
  testability, clarity, completeness) — the rule says matches are exact and
  never says what a title, a URL, or a snake_case key may contain.
- **Claims about the Python reference** (flagged by: clarity, dependency,
  testability, scope) — "share the accepted tier texts" and "differ on
  purpose in three places" do not hold against
  `tasks/eval/documents/sources.py`, and "pass on both" is neither a
  requirement nor checkable.

### Findings

#### Critical

None.

#### Major

- 🟡 **Clarity, Dependency, Testability**: The entry shape is written
  `(drafts/url)`, contradicting the skills and the criteria
  **Location**: Context; Requirements: Line rules
  Under "matches are exact", `drafts/` reads as a required literal prefix,
  which would make every real entry an unread line.
- 🟡 **Testability, Clarity, Completeness**: Entry title and URL have no
  grammar and no boundary criteria
  **Location**: Requirements: Line rules
  A title containing `]`, a URL containing `)` or a space, and an empty title
  or URL have no defined outcome. Two implementers could split the same line
  differently and both pass.

#### Minor

- 🔵 **Clarity, Dependency, Testability**: The divergence record against the
  Python reference is incomplete and partly inaccurate
  **Location**: Dependencies
  The reference accepts `tier-3 (unknown)`, `tier-3 (withdrawn; retracted)`,
  `tier-3 (also tier-9)`, `tier-3 ()`, an empty block value, and digits in
  keys, and fails on a trailing ` — `. The item lists three differences.
- 🔵 **Testability, Clarity**: Block-line key and value boundaries are not
  pinned
  **Location**: Requirements: Line rules
  `doi2`, `_doi`, `a__b`, and a whitespace-only value have no stated outcome.
  PP-924 relies on this rule.
- 🔵 **Testability**: Section-state boundaries lack criteria
  **Location**: Acceptance Criteria
  An empty section, a section of only unread lines, `None found.` beside an
  unread line, and ` None found.` are untested.
- 🔵 **Testability**: A blank line between an entry and its block line is
  untested
  **Location**: Acceptance Criteria
  The one place a blank line changes the result. The rule says "blank lines"
  where the ignore rule says "blank and whitespace-only".
- 🔵 **Testability**: Line endings and trailing whitespace are undefined
  under exact matching
  **Location**: Requirements: Line rules
  CRLF input, or a trailing space after `## Sources`, could read as absent.
- 🔵 **Clarity**: "Tier", "parsed tier" and "parsed tier text" drift
  **Location**: Requirements
  An entry's "tier" is a parsed tier text or the rejected text, not the
  crate's `Tier`. Carried over from review 1 in part.
- 🔵 **Dependency**: The consumer's unsettled need to drop the raw Sources
  section may reach back into this parser
  **Location**: Dependencies
  The parser returns no section location. If the server strips the section,
  it re-reads the boundary rule this item exists to own.
- 🔵 **Scope**: "A test outside the `research` crate" could pull a crate
  edge into scope
  **Location**: Acceptance Criteria
  Read as a test in another workspace crate, it needs the edge PP-1000 or
  PP-1001 owns.
- 🔵 **Scope, Testability**: Parity with the Python test cases is asserted
  and is neither in nor out of scope
  **Location**: Dependencies

#### Suggestions

- 🔵 **Clarity**: Two superseded rules still read as current
  **Location**: Drafting Notes
  The second-review note says `None found.` is ignored and a block line
  attaches to the most recent entry.
- 🔵 **Clarity**: "Handles the document body itself" does not say what the
  server does
  **Location**: Dependencies
- 🔵 **Scope**: Round-trip display has no named consumer
  **Location**: Requirements
  It is the stated reason for strict shape.
- 🔵 **Scope**: Size M sits at the upper edge after the line rules were
  added
  **Location**: Technical Notes
- 🔵 **Dependency**: PP-924's conditional coupling is recorded only here
  **Location**: Dependencies
- 🔵 **Testability**: "Leave `Tier` and `Record` as they are" has no
  criterion
  **Location**: Requirements

### Strengths

- ✅ Both parsers are named and their relationship stated, so "the parser"
  is rarely ambiguous.
- ✅ Entry, block line, unread line and the three section states are defined
  once and reused.
- ✅ Almost every criterion gives a literal input and an exact result.
- ✅ Rejection is specified by an observable property, backed by twelve
  malformed inputs that each isolate one violation.
- ✅ Leniency is pinned from both sides: rule-breaking but well-shaped text
  is accepted, mis-shaped text rejected.
- ✅ Out-of-scope work is named: no wire shape, no crate edge, no change to
  the Python parser or to `Tier` and `Record`.
- ✅ The consumer edge is recorded in three places and reciprocated by
  PP-1000.

### Recommended Changes

1. **Restore `(url)` in Context and the entry rule** (addresses: entry shape
   `drafts/url`)
   Correct the same text in PP-1000.
2. **State the title, URL and key grammars** (addresses: title and URL
   grammar; block-line boundaries)
   For example: the title runs to the last `](`, the URL holds no `)` or
   whitespace, a key is `[a-z][a-z0-9_]*`. Add criteria for a bracketed
   title, a URL containing `)`, empty parts, a digit-bearing key, and a
   whitespace-only value.
3. **Rewrite the Python-reference claim** (addresses: divergence record;
   parity)
   Say the reference accepts a superset and is no oracle for rejection, or
   list every difference. Drop "pass on both" or make it a criterion.
4. **Add the boundary criteria** (addresses: section state; blank line;
   line endings)
   An empty section, `None found.` plus prose, a blank and a whitespace-only
   line before a block line, and CRLF input.
5. **Use one term for the parsed value** (addresses: "tier" drift)
6. **Record the section-location follow-on** (addresses: raw Sources
   section)
   Note it as contingent on the PP-1000 and PP-994 wire decision.
7. **State where the out-of-crate test lives** (addresses: crate edge)
   An integration test under the crate's own `tests/`.
8. **Mark the superseded drafting notes** (addresses: superseded rules)

## Per-Lens Results

### Clarity

**Summary**: Unusually precise. The defects are the `drafts/url` placeholder
in the normative rule, undefined title and URL shapes, and a Dependencies
paragraph that does not match the reference file.

**Findings**: major — entry shape `(drafts/url)` (high). minor — title and
URL content unstated; Python-reference claims; "tier" terminology.
suggestion — key shape; superseded drafting notes; "handles the document
body".

### Completeness

**Summary**: Structurally complete for a task; every section is populated
and frontmatter is intact.

**Findings**: minor — entry and block-key token shapes undefined although
matching is exact (medium).

### Dependency

**Summary**: Couplings are mapped thoroughly and the consumer edge is
reciprocated.

**Findings**: minor — divergence record incomplete and partly inaccurate;
consumer's need to drop the raw Sources section. suggestion — PP-924
coupling one-sided; `drafts/url` differs from the upstream definition.

### Scope

**Summary**: One coherent unit; kind and size fit.

**Findings**: minor — out-of-crate test could pull in a crate edge; Python
parity neither in nor out of scope. suggestion — round-trip display has no
consumer; M sits at the upper edge.

### Testability

**Summary**: Highly verifiable; the main gap is the entry line itself.

**Findings**: major — title and URL have no grammar or boundary criteria
(high). minor — Python reference differs in more places than listed;
block-line key and value boundaries; section-state boundaries; blank line
before a block line; line endings. suggestion — two requirements with no
criterion.

---
*Review generated by /accelerator:review-work-item*

## Changes After the Review — 2026-11-23

The verdict above is the review's. These changes were made afterwards and
have not been re-reviewed; each line records what the author and the
drafting agent did, not a reviewer's judgement.

- 🟡 **Clarity**: `(drafts/url)` — Addressed: restored to `(url)` here and in PP-1000.
- 🟡 **Testability**: Title and URL grammar — Addressed by the author's decision: title non-empty, ending at the last `](`; URL non-empty with no `)` or whitespace; key `[a-z][a-z0-9_]*`; value with a non-whitespace character; one trailing carriage return dropped, nothing else trimmed. Each has a criterion.
- 🔵 Python-reference claims — Addressed against the file: the reference is looser and no oracle for rejection; four intended differences listed; porting its tests is out of scope.
- 🔵 Section-state boundaries; blank line before a block line; "tier" terminology; out-of-crate test; superseded drafting notes; `Tier` and `Record` unchanged — Addressed.
- 🔵 Section location — Addressed by the author's decision: the parser reports the section's line span, and PP-1000's server drops the section by it.
- Not addressed: round-trip display has no named consumer; size M not re-confirmed; PP-924 does not yet record the key and value condition.

## Re-Review (Pass 2) — 2026-11-23

**Verdict:** COMMENT

Work item is acceptable but could be improved — see the major finding below.

### Previously Identified Issues
- 🟡 **Clarity**: Entry shape `(drafts/url)` — Resolved
- 🟡 **Testability**: Title and URL have no grammar — Partially resolved (grammar and boundary criteria added; the title rule has two readings, see below)
- 🔵 Python-reference claims — Partially resolved (rewritten; a CRLF divergence is unlisted)
- 🔵 Block-line boundaries; section-state boundaries; blank line before a block line; line endings; "tier" terminology; section location; out-of-crate test; superseded notes — Resolved
- 🔵 **Scope**: Round-trip display has no named consumer — Still present
- 🔵 **Scope**: Size M at the upper edge — Still present
- 🔵 **Dependency**: PP-924's condition recorded on one side — Still present

### New Issues Introduced
- 🟡 **Testability, Clarity**: "The title ends at the last `](` on the line" has two readings when a venue holds a markdown link. Read literally the line is unread; read as the Python pattern backtracks, it is an entry. No criterion separates them.
- 🔵 **Clarity**: The value rule, "one or more characters that are not whitespace", reads as "no whitespace allowed"; the `journal_ref` criterion needs "at least one".
- 🔵 **Clarity, Testability**: "As written" is unclear for a CRLF line: whether a reported unread line keeps its carriage return.
- 🔵 **Testability, Clarity**: The line span has no criterion for a declared-empty section, is absent from the integration-test criterion, and never states its counting base.
- 🔵 **Clarity**: "Attach each metadata block line to the entry above it" is unqualified and contradicts the adjacency rule; requirement bullets do not say which parser acts.
- 🔵 **Completeness**: Behaviour-defining content (precedence over Python, no serialisable types) sits in Dependencies.
- 🔵 **Dependency**: PP-994's use of the entry shape is absent from Dependencies; PP-936 is unnamed.
- 🔵 **Scope**: The Summary omits the section state, unread lines and line span.

### Assessment
Both earlier majors are closed. One new major remains, a two-reading rule introduced with the token grammar, and the rest is precision. The item is close to ready; a short edit to the title rule and the span would clear it.

## Changes After the Re-Review — 2026-11-23

Made after pass 2 and not re-reviewed; the verdict above is the reviewers'.

- 🟡 Title rule — Addressed: the title ends at the last `](` for which the rest of the line reads as an entry, with a criterion for a venue holding a link.
- 🔵 Value rule; line-span base and declared-empty case; CRLF and "as written"; block-line bullet; PP-994 and PP-936 in Dependencies; Summary — Addressed.
- Not addressed: round-trip display has no consumer; size M; PP-924 does not record its condition.

## Approval — 2026-11-24

**Verdict:** APPROVE

Approved by the author; not re-reviewed after the changes above.
