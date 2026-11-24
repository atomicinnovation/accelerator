---
type: "plan-review"
id: "2026-11-24-PP-995-sources-parser-in-the-research-crate-review-1"
title: "Plan Review: Sources Parser in the Research Crate Implementation Plan"
date: "2026-11-24T09:31:14+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-24-PP-995-sources-parser-in-the-research-crate"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "standards", "usability"]
review_number: 1
review_pass: 2
tags: ["research", "visualiser", "sources", "tier-text"]
last_updated: "2026-11-24T10:00:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Sources Parser in the Research Crate Implementation Plan

**Verdict:** REVISE

The plan is a well-contained, additive, test-first design: two pure parsers
inside the research domain crate, following established crate idioms, with
every PP-995 acceptance criterion mapped to a named behavioural test and the
tier-text round-trip holding by construction. Five major findings push the
verdict to REVISE. Two would fail the lint gate as written (`# Errors` doc,
`unwrap_err`), one is a missing test for the deliberate title-search
direction, one is a misleading end-of-body removal test, and one is an API
gap for PP-1000's "in place" rendering. The rest are local naming,
convention and state-modelling refinements.

### Cross-Cutting Themes

- **`TierReading::from` as an unplanned public parsing `From` impl**
  (flagged by: code-quality, standards, usability) — contradicts "parsers are
  inherent `parse` methods" and adds an unlisted snapshot line.
- **Mark getter names differ from `Record`** (flagged by: standards,
  usability) — `is_retracted`/`is_withdrawn` vs `Record::retracted`/`withdrawn`.
- **`CitedSource` changes shape between independently mergeable phases**
  (flagged by: architecture, test-coverage) — adding `pub metadata` in Phase 3
  breaks struct literals and downstream destructuring.
- **A `LineSpan` is not tied to the body it was read from** (flagged by:
  architecture, usability) — a frontmatter-stripped vs full body mix-up
  silently removes the wrong lines.
- **The right-to-left title rule** (flagged by: test-coverage, correctness) —
  untested direction; the work item's two sentences disagree on a venue link
  followed by a further ` — ` segment.
- **"Eleven" citing-once examples, ten listed** (flagged by: test-coverage,
  correctness).

### Tradeoff Analysis

- **Minimal public surface vs consumer ergonomics**: usability wants a
  position-keeping primitive and a `SourcesSection`-level removal helper;
  the plan and work item keep the surface to what PP-995 specifies. Escalated
  to the author, because PP-1000 asks both for in-place rendering and for no
  line counting of its own.

### Findings

#### Major

- 🟡 **Usability**: No primitive for placing structured Sources where the section sat
  **Location**: Phase 4: Removing the Section by Its Span
  `remove_from` joins the text around the section with no marker, so PP-1000
  cannot render Sources in place without counting lines itself.
- 🟡 **Test Coverage**: No test pins the "last `](`" title rule
  **Location**: Phase 2, Section 3: CitedSource::parse title rule
  Swapping `rmatch_indices` for `match_indices` breaks no planned test.
- 🟡 **Correctness**: End-of-body removal test promises a final-newline distinction remove_from cannot produce
  **Location**: Phase 4, Section 1: Tests first
  Both bodies give the same string; the description invites "fixing"
  `remove_from` wrongly.
- 🟡 **Standards**: TierText::parse lacks the `# Errors` doc section pedantic clippy requires
  **Location**: Phase 1, Section 3: The parser
  `missing_errors_doc` under `-D warnings` fails `cli:check`.
- 🟡 **Standards**: Rejection test uses `unwrap_err()`, which the crate lints against
  **Location**: Phase 1, Section 1: Tests first
  Research tests allow only `expect_used`; use `expect_err`.

#### Minor

- 🔵 **Code Quality / Standards / Usability**: `TierReading::from(&str)` adds a public parsing conversion
  **Location**: Phase 2, Section 3
- 🔵 **Code Quality**: Rejection of out-of-order or repeated marks is implicit; shadowed names
  **Location**: Phase 1, Section 3
- 🔵 **Code Quality**: Block adjacency is a hand-maintained boolean flag
  **Location**: Phase 3, Section 2
- 🔵 **Code Quality**: `read_from(title, after)` bundles five token rules behind a vague name
  **Location**: Phase 2, Section 3
- 🔵 **Code Quality**: Conditional instruction about the dead `as_written` field
  **Location**: Phase 2, Section 2
- 🔵 **Architecture**: First topic-to-sources module dependency is unacknowledged
  **Location**: Phase 1, Section 3
- 🔵 **Architecture**: Tier label vocabulary gets a second hand-written definition
  **Location**: Phase 1, Section 3
- 🔵 **Architecture / Test Coverage**: Independently mergeable phases change `CitedSource`'s shape
  **Location**: Phase 3, Section 2
- 🔵 **Architecture / Usability**: A LineSpan is not tied to the body it was read from
  **Location**: Phase 4, Section 2
- 🔵 **Test Coverage**: `None found.` clearing block adjacency is untested
  **Location**: Phase 3, Section 2
- 🔵 **Test Coverage**: No test covers a lone `\r`
  **Location**: Phase 2, Section 1
- 🔵 **Test Coverage / Correctness**: "Eleven" examples named, ten listed
  **Location**: Phase 1, Section 1
- 🔵 **Correctness**: The span depends on `last` advancing for ignored lines
  **Location**: Phase 2, Section 3
- 🔵 **Correctness**: Right-to-left title search misreads a venue link followed by ` — `
  **Location**: Phase 2, Section 3
- 🔵 **Correctness**: Additions-only snapshot checks can pass on an empty working-copy diff
  **Location**: Desired End State; Phase 1-3 Success Criteria
- 🔵 **Standards**: Mark getters named `is_retracted`/`is_withdrawn`, unlike `Record`
  **Location**: Phase 1, Section 3
- 🔵 **Standards**: New source files lack the `//!` module doc
  **Location**: Phase 1, Section 3; Phase 2, Section 2
- 🔵 **Standards**: `use std::error::Error;` departs from the crate's fully qualified style
  **Location**: Phase 1, Section 3
- 🔵 **Standards**: `pub(super)` is not used anywhere else in the crate
  **Location**: Phase 2, Section 2
- 🔵 **Usability**: Removing the section needs an Option dance at every call site
  **Location**: Phase 4; Phase 2

#### Suggestions

- 🔵 **Code Quality**: `TierReading::Rejected(String)` discards `NotTierText`
  **Location**: Phase 2, Section 3
- 🔵 **Architecture**: Tier text is nested under the Sources section
  **Location**: Implementation Approach: Module layout
- 🔵 **Architecture**: Offer a parse-and-remove convenience tied to one body
  **Location**: Phase 4, Section 2
- 🔵 **Test Coverage**: Turn the throwaway real-finding check into a committed test
  **Location**: Phase 2 Manual Verification
- 🔵 **Standards**: `SEPARATOR` is ambiguous beside its siblings
  **Location**: Phase 2, Section 3
- 🔵 **Usability**: The `Tier` type has to be found in an unrelated module
  **Location**: Implementation Approach: Module layout
- 🔵 **Usability**: The rejection message does not say what was expected
  **Location**: Phase 1: NotTierText Display

### Strengths

- ✅ Pure, I/O-free parsers in the domain crate; import fence and manifest unchanged.
- ✅ The section state enum makes a span exist only with a section.
- ✅ One private line rule is shared by the parser and the span removal.
- ✅ The plan follows crate idioms throughout (`Outline::parse`, `StoredCount::Unreadable`, `refused`, inherent `parse`, one item per `use`).
- ✅ Strict shape checking makes the `Display` round-trip hold by construction.
- ✅ Every acceptance criterion maps to a named behavioural table-loop test that uses public exports only.
- ✅ Each phase is additive and test-first, with a snapshot check per phase.

### Recommended Changes

1. **Add `# Errors` to `TierText::parse`; use `expect_err` in the rejection test** (addresses: both standards majors)
2. **Pin the title-search direction with a two-candidate test** (addresses: last `](` rule, title misread)
3. **Restate the end-of-body removal case as "both bodies give the same string"** (addresses: end-of-body removal test)
4. **Decide on a position-keeping primitive for PP-1000** (addresses: in-place placement; escalated)
5. **Make `TierReading::of` private; rename getters to `retracted`/`withdrawn`; add module docs, `impl std::error::Error`, `pub(crate)`, `SEGMENT_SEPARATOR`** (addresses: standards and code-quality minors)
6. **Fix `CitedSource`'s final shape in Phase 2 and add an entry test helper** (addresses: shape change between phases)
7. **Add tests for `None found.` adjacency and a lone `\r`; state that `last` advances for every line; count ten examples; scope `jj diff` to `--from <base>`** (addresses: remaining test and correctness minors)

## Per-Lens Results

### Architecture

**Summary**: The change is structurally contained, with sound module
boundaries, enum-based state and one shared line rule. The remaining
concerns are an unacknowledged topic-to-sources edge, duplicated tier
labels, tier text placed under the Sources section, a struct shape that
changes between phases, and a span that is not tied to its body.

**Findings**: see the Minor and Suggestion entries above (all minor or
suggestion, with medium or high confidence).

### Code Quality

**Summary**: The parsers are small, pure and idiomatic. The concerns are
local: an unplanned public `From`, rules implied by the control flow, a
hand-managed adjacency flag, a dense `read_from`, and a hedged dead-field
step.

**Findings**: five minor findings and one suggestion, listed above.

### Test Coverage

**Summary**: Coverage is strong. The gaps are mutation-surviving rules
(title direction, `None found.` adjacency, a lone `\r`), test churn when a
field is added, and the miscount.

**Findings**: one major (title direction, medium confidence), four minor
findings and one suggestion.

### Correctness

**Summary**: The core logic traces correctly against every criterion. The
risks are the end-of-body removal test wording, the unstated update of
`last`, the title search on an edge case, the miscount, and vacuous diff
checks.

**Findings**: one major (end-of-body removal, medium confidence) and four
minor findings.

### Standards

**Summary**: The plan follows crate conventions closely. Two snippets would
fail clippy as written, and there are smaller departures in naming, docs
and visibility.

**Findings**: two majors (high confidence), five minor findings and one
suggestion.

### Usability

**Summary**: The surface reads well and maps onto PP-1000. The main gap is
that there is no position-keeping body primitive. Smaller friction comes
from the Option dance, unanchored spans, getter names, `TierReading::from`,
and the discoverability of `Tier`.

**Findings**: one major (in-place placement, medium confidence), four minor
findings and two suggestions.

## Edits Applied After Pass 1

Applied without user input (mechanical or self-judged): every finding above
except the in-place placement primitive (escalated), plus these deliberate
skips: `TierReading::Rejected` stays `String`, as the work item specifies;
`tier_text` stays under `sources_section`, as the work item specifies; the
`SourcesSection`-level removal helper waits on the escalation; the real-finding
check stays manual; the `NotTierText` message is unchanged; the title rule
keeps the work item's literal "last `](`" reading, now pinned by a test.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-24T10:00:08+00:00

**Verdict:** COMMENT

Before this pass, the author decided the pass-1 escalation (E1): add
`LineSpan::split(self, body) -> (&str, &str)` in Phase 4, with `remove_from`
as the two halves joined, mirroring `remove_from`'s tests.

### Previously Identified Issues
- 🟡 **Usability**: No primitive for placing structured Sources where the section sat — Resolved (`LineSpan::split`)
- 🟡 **Test Coverage**: No test pins the "last `](`" title rule — Resolved
- 🟡 **Correctness**: End-of-body removal test wording — Resolved
- 🟡 **Standards**: `# Errors` doc on `TierText::parse` — Resolved
- 🟡 **Standards**: `unwrap_err` in the rejection test — Resolved
- 🔵 **Code Quality / Standards / Usability**: `TierReading::from` — Resolved (private `TierReading::of`)
- 🔵 **Code Quality**: implicit canonical-order rejection, adjacency boolean, `read_from` naming, hedged `as_written` — Resolved
- 🔵 **Architecture**: topic-to-sources edge, duplicated tier labels, `CitedSource` shape change — Resolved
- 🔵 **Architecture / Usability**: span not tied to its body — Resolved (documented precondition)
- 🔵 **Test Coverage**: `None found.` adjacency, lone `\r`, ten examples — Resolved
- 🔵 **Correctness**: `last` advancing for ignored lines, vacuous `jj diff` — Resolved
- 🔵 **Standards**: getter names, module docs, error impl style, `SEPARATOR` — Resolved; `pub(super)` — superseded (see below)
- 🔵 **Usability**: Option dance for removal — Accepted (`split` covers the consumer; no extra helper)

### New Issues Introduced
- 🟡 **Standards**: `pub(crate)` in the private `lines` module trips nursery's `redundant_pub_crate` — fixed after this pass (plain `pub`)
- 🔵 **Standards**: `match` on `split_once` likely flagged by `option_if_let_else` — fixed after this pass (`let`-`else` with `Self::bare`)
- 🔵 **Code Quality**: line-offset arithmetic outside `lines` — fixed after this pass (`lines::offset_of`; `Line` needs no `as_written`)
- 🔵 **Correctness / Test Coverage**: the initial `last` for an empty section is unstated and its span untested — fixed after this pass (`opened_at` sets `last = start`; span asserted with and without a following heading)
- 🔵 **Usability**: PP-1000 not pointed at `split` — fixed after this pass (Migration Notes); PP-1000 itself still names `remove_from`
- 🔵 **Usability**: `LineSpan::end` exclusivity undocumented — fixed after this pass (type doc)
- 🔵 **Test Coverage**: `NotTierText` `Display` untested — fixed after this pass (asserted in the rejection test)
- 🔵 **Architecture**: `BASE_TIERS` restates the variant list — accepted tradeoff (`Tier` is unchanged by this item)
- 🔵 **Code Quality**: positional booleans in the tier-text test helper — not applied (suggestion)

### Assessment

One major finding, below the revise threshold of three, and no critical
findings: the plan is acceptable. All pass-2 findings except two
suggestions were applied to the plan after this pass. One follow-up lies
outside this plan: PP-1000's server step should name `split`.

