---
type: "work-item-review"
id: "0352-sync-reports-every-unsynced-item-twice-review-1"
title: "Work Item Review: Sync Reports Every Unsynced Item Twice"
date: "2026-11-10T09:43:10+00:00"
author: "Phil Helm"
producer: "review-work-item"
status: "complete"
parent: "work-item:0146"
target: "work-item:0352"
work_item_id: "0352"
reviewer: "Phil Helm"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: ["sync", "report", "work-adapters"]
last_updated: "2026-11-10T11:53:04+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Work Item Review: Sync Reports Every Unsynced Item Twice

**Verdict:** REVISE

A tightly scoped, well-evidenced bug: one root cause, a verbatim reproduction,
and a fix shape grounded in named code paths, sized S and bundled correctly.
What holds it back is internal consistency after refinement — the Summary,
Requirements and Acceptance Criteria disagree with the Technical Notes on the
skill rule, on orders that reach no create, and on what "settles" means for the
watermark — and the row-count criteria do not say which row must survive.

### Cross-Cutting Themes

- **Order criterion contradicts the out-of-scope `order-refused` case**
  (flagged by: clarity, testability) — AC4 has no precondition, so it covers
  orders on drafts and under `--pull-only` that the Technical Notes say keep two
  lines.
- **Answered open question still listed as open** (flagged by: clarity,
  completeness, scope) — Technical Notes answer the `advance_ids` question, but
  Open Questions still asks it.
- **Watermark change under-specified** (flagged by: clarity, scope,
  testability) — "settles" is undefined, the Summary omits the baseline change,
  and only the failure direction has a criterion.
- **Relationships disagree between frontmatter and Dependencies** (flagged by:
  clarity, dependency) — `relates_to` lists 0300 and 0296; Dependencies names
  only 0351; 0296's relevance is unexplained.

### Findings

#### Major

- 🟡 **Clarity + Testability**: AC4's precondition includes orders the work
  item says keep two rows
  **Location**: Acceptance Criteria
  "Given an adopt or create order … exactly one row" is unqualified, yet an order
  on a draft or under `--pull-only` keeps its `noop` row plus an `order-refused`
  line, explicitly out of scope.
- 🟡 **Testability**: "Exactly one row" criteria do not say which row survives
  **Location**: Acceptance Criteria
  AC1 and AC4 only count rows; a wrong fix that dropped the create row and kept
  `noop` would pass both.
- 🟡 **Clarity**: Summary says `/sync-work-items` has no rule for the `noop`
  row; later sections say it does
  **Location**: Summary
  Assumptions, AC6 and Technical Notes all describe an interim rule at
  `SKILL.md:466` that this work removes.
- 🟡 **Clarity**: Watermark requirement uses undefined "settles" and is unclear
  for items without a create
  **Location**: Requirements
  "Settles" could mean completes or succeeds, and "for an unsynced item" sweeps
  in drafts and `--pull-only` items whose `noop` rows still advance.
- 🟡 **Dependency**: Implied dependency on 0300 recorded as "Blocked by: none"
  **Location**: Dependencies
  The fix builds on code 0300 introduced. *Reviewer note: 0300 is `done` and
  that code is on main, so this needs only a clarifying clause, not a blocker.*

#### Minor

- 🔵 **Completeness**: Reproduction steps omit non-draft status and an
  "Actual:" line
  **Location**: Requirements
  A reader starting from a draft will not see the bug.
- 🔵 **Testability**: No reproduction trigger for the failed-create case
  **Location**: Requirements
  AC2 and AC5 assume a failing create without defining how it fails.
- 🔵 **Completeness**: Changes list omits removing the skill rule and its test
  **Location**: Requirements
  Only AC6 and the size line mention it.
- 🔵 **Clarity / Completeness / Scope**: Open question still open though
  Technical Notes answer it
  **Location**: Open Questions
- 🔵 **Scope**: Summary omits the watermark change
  **Location**: Summary
  The item changes persisted baseline behaviour, not just reporting.
- 🔵 **Testability**: Watermark verified only in the failure direction
  **Location**: Acceptance Criteria
  Nothing checks that a successful create still advances once the `noop` row is
  gone.
- 🔵 **Testability**: First-row helpers cannot verify "exactly one row"
  **Location**: Technical Notes
  `row()` / `outcome_of` return an id's first row, so tests need every row for
  the id.
- 🔵 **Clarity**: Context line ranges conflict with the corrected ranges in
  Technical Notes
  **Location**: Context
- 🔵 **Clarity**: "Expected: one row per item, carrying its create's outcome"
  contradicts items that keep `noop`
  **Location**: Requirements
- 🔵 **Clarity**: "Order", "adopt" and "baseline watermark" used without
  definition
  **Location**: Acceptance Criteria
- 🔵 **Dependency**: Ordering with 0351 implied but not captured; 0351 missing
  from `relates_to`
  **Location**: Dependencies
- 🔵 **Dependency**: Report consumers (user guide, changelog) not enumerated
  **Location**: Assumptions

#### Suggestions

- 🔵 **Clarity**: Fix-shape partition sentence parses more than one way
  **Location**: Technical Notes
- 🔵 **Clarity / Dependency**: Frontmatter `relates_to` and Dependencies
  disagree; 0296 unexplained
  **Location**: Frontmatter: relates_to
- 🔵 **Testability**: Push-only preview test update and run scopes (targeted vs
  whole-corpus) not tied to a criterion
  **Location**: Acceptance Criteria

### Strengths

- ✅ Single root cause; every change follows from one partition, so bundling the
  row fix, the watermark fix and the skill-rule removal is correct.
- ✅ Context quotes the broken output verbatim and covers preview and apply,
  targeted and whole-corpus, released and branch binaries.
- ✅ Context names each actor in the defect chain and what it does.
- ✅ Scope boundary is explicit: the `order-refused` double line is named as
  pre-existing and out of scope.
- ✅ AC5 asserts on the stored `local_synced_at`, not timing-dependent effects,
  and is limited to the only observable case.
- ✅ The consumer-side workaround is removed in the same item, so producer and
  consumer change together.
- ✅ Kind (bug) and size (S) fit the work.

### Recommended Changes

1. **Narrow AC4 and name the surviving row in AC1/AC4** (addresses: AC4
   precondition; which row survives; "Expected" contradiction)
   Limit AC4 to unsynced, non-draft items with an order under a push-capable
   run. State the single row: `create-from-local` / `adopt` in preview, the
   create's outcome in apply, or its `failed` row. Narrow "Expected" to items
   the run creates.
2. **Fix the Summary** (addresses: skill-rule contradiction; Summary omits
   watermark)
   Say the skill carries an interim rule this work removes, and that the
   leftover entry also advances a failed create's watermark.
3. **Define the watermark rule precisely and test both directions** (addresses:
   "settles"; watermark only failure-direction)
   Replace "settles" with "succeeds" (its create writes a baseline entry), say
   drafts and `--pull-only` items keep today's behaviour, and add a criterion
   that a successful create still advances.
4. **Close the open question** (addresses: answered open question)
   Mark it resolved with a pointer to Technical Notes.
5. **Complete Requirements and the reproduction** (addresses: Changes omits
   skill rule; reproduction gaps; no failed-create trigger)
   Add the skill-rule removal to Changes; say the item must be non-draft; add
   "Actual:"; add a failed-create reproduction (for example a tracker create
   rejection).
6. **Tidy Dependencies and relationships** (addresses: 0300 dependency; 0351
   ordering; relates_to mismatch)
   Note 0300 is done, so nothing blocks; add `work-item:0351` to `relates_to`
   (a frontmatter edit, via `/update-work-item`); say what 0296 contributes.
7. **Refresh Context line ranges and drop the drift note** (addresses: Context
   line ranges)
8. **Optional polish** (addresses: test helpers; undefined terms; partition
   sentence; push-only test criterion; report consumers)
   Note that tests must collect every row for an id; gloss "order" and "adopt";
   list the partition conditions as separate clauses; tie the push-only test
   update to AC6.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item describes the defect precisely with literal rows,
named code paths and a reproduction. Internal contradictions weaken it: the
Summary's "no rule" versus the interim rule, the order AC versus the Technical
Notes, and an undefined "settles". Undefined domain terms and stale line ranges
add friction.

**Strengths**:
- Literal tab-separated rows make "reports twice" unambiguous.
- Context names every actor in the defect chain and its effect.
- Requirements separate created from non-created unsynced items; ACs keep the
  split.
- The reproduction gives a concrete command and observed output.

**Findings**:
- **major / high — Summary**: Summary says `/sync-work-items` has no rule for
  the noop row; Assumptions, AC6 and Technical Notes say an interim rule exists
  and is removed. Align the Summary.
- **major / high — Acceptance Criteria**: The adopt/create-order AC contradicts
  the Technical Notes, which say orders reaching no create produce two lines.
  Limit the AC to orders the run creates.
- **major / medium — Requirements**: "Settles" is undefined (completes or
  succeeds?), and "for an unsynced item" sweeps in drafts and `--pull-only`
  items. Replace it with the specific outcome and state the rule's reach.
- **minor / high — Context**: Context line ranges conflict with the corrected
  ranges in Technical Notes. Update the Context and drop the drift note, or cite
  symbols.
- **minor / high — Open Questions**: The open question is answered in Technical
  Notes. Close it.
- **minor / medium — Acceptance Criteria**: "Order", "adopt", "baseline
  watermark", "TES sandbox" and "mtime pre-filter" are undefined. Gloss them or
  link to definitions.
- **minor / medium — Requirements**: "Expected: one row per item, carrying its
  create's outcome" is broader than intended. Limit it to created items.
- **suggestion / medium — Technical Notes**: The partition-condition sentence
  parses ambiguously. List the conditions as separate clauses.
- **suggestion / medium — Frontmatter: relates_to**: `relates_to` (0300, 0296)
  and Dependencies (0351) disagree. Reconcile them and explain 0296.

### Completeness

**Summary**: Substantively complete; every section is populated and the
frontmatter is consistent. Small gaps: the reproduction input is
under-specified, the answered open question is still listed, and the Changes
list omits the skill-rule removal.

**Strengths**:
- The Summary names both forms of the duplicate and why it matters.
- The Context gives observed output, environment, binary version and the
  root-cause trace.
- Bug-appropriate content is present: reproduction, expected and actual.
- The ACs cover the main path, failure, exclusions, orders, watermark and skill
  rule.
- Dependencies and Assumptions are substantive.
- The frontmatter is complete and consistent.

**Findings**:
- **minor / medium — Requirements**: Reproduction omits non-draft status and
  tracker config; there is no "Actual:" line and no step reaching the
  failed-create form.
- **minor / high — Open Questions**: The question is answered in Technical
  Notes; remove it or mark it resolved.
- **minor / medium — Requirements**: Changes omits the skill-rule and
  Python-test removal required by AC6.

### Dependency

**Summary**: The Dependencies section is populated, but couplings are
understated: an implied dependency on 0300, the 0351 ordering not captured, and
report consumers beyond the skill not named.

**Strengths**:
- Dependencies is deliberate and explains the 0351 link.
- Assumptions name the main downstream consumer; the AC removes its workaround
  in the same change.
- Technical Notes trace the internal readers of the leftover entries.

**Findings**:
- **major / medium — Dependencies**: The fix builds on 0300's code (skill rule,
  `refused_orders`, `finalise_baseline`) yet says "Blocked by: none". State
  whether 0300 has merged. *(Reviewer note: 0300 is `done`; on main.)*
- **minor / medium — Dependencies**: Sequencing with 0351 is implied but not
  recorded; 0351 is missing from `relates_to`.
- **minor / medium — Assumptions**: Consumers of the stdout report (user guide,
  changelog, scripts) are not enumerated; the "no script parses" assumption is
  unverified.
- **suggestion / low — Frontmatter: relates_to**: The two relationship lists
  disagree; 0296's relevance is unexplained.

### Scope

**Summary**: Tightly scoped and atomic; every requirement follows from one
cause. Kind and size fit. Small signals: the Summary omits the watermark change,
and the answered open question is still listed.

**Strengths**:
- All changes stem from one partition and cannot be delivered separately.
- The out-of-scope `order-refused` behaviour is named explicitly.
- Kind and size fit the work.
- The skill workaround is removed in the same item.
- The 0351 relationship is non-blocking.

**Findings**:
- **minor / medium — Summary**: The Summary omits the persisted-state watermark
  change the Requirements and ACs commit to.
- **suggestion / medium — Open Questions**: Close the answered question; state
  that the watermark fix comes free with the partition.

### Testability

**Summary**: Verifiable overall: concrete reproduction, verbatim broken output,
Given/When/Then criteria and a deterministic watermark check. Gaps: the
row-count criteria do not name the surviving row, AC4's precondition is too
broad, and there is no failed-create trigger.

**Strengths**:
- Verbatim broken output in Context.
- A minimal deterministic reproduction with expected outcome.
- AC5 asserts the stored `local_synced_at`, scoped to the observable case.
- AC3 excludes order-carrying items.
- AC6 is checkable by inspection.

**Findings**:
- **major / high — Acceptance Criteria**: AC4's precondition includes orders
  that keep two rows. Narrow it to non-draft, push-capable runs and name the
  row.
- **major / high — Acceptance Criteria**: AC1 and AC4 count rows without naming
  the surviving one; a fix keeping `noop` would pass.
- **minor / medium — Requirements**: No trigger for the failed-create
  reproduction.
- **minor / medium — Acceptance Criteria**: The watermark is verified only in
  the failure direction; add a success-path criterion.
- **minor / medium — Technical Notes**: The first-row helpers cannot verify
  "exactly one row"; tests must collect all rows for an id.
- **suggestion / medium — Acceptance Criteria**: The push-only preview test
  update and the targeted vs whole-corpus scope are not tied to a criterion.

## Re-Review (Pass 2) — 2026-11-10T09:50:12+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity + Testability**: AC4's precondition includes orders that keep
  two rows — Resolved
- 🟡 **Testability**: "Exactly one row" criteria do not say which row
  survives — Resolved
- 🟡 **Clarity**: Summary says the skill has no rule for the `noop` row —
  Resolved
- 🟡 **Clarity**: Watermark requirement uses undefined "settles" — Partially
  resolved ("settles" replaced, but "the run creates" is now ambiguous; see new
  issues)
- 🟡 **Dependency**: Implied dependency on 0300 — Resolved
- 🔵 **Completeness**: Reproduction omits non-draft status and "Actual:" —
  Resolved
- 🔵 **Testability**: No failed-create reproduction trigger — Partially
  resolved (named the test seam; no manual trigger)
- 🔵 **Completeness**: Changes omits skill-rule removal — Resolved
- 🔵 **Clarity / Completeness / Scope**: Answered open question still open —
  Resolved
- 🔵 **Scope**: Summary omits the watermark change — Resolved
- 🔵 **Testability**: Watermark verified only on failure — Resolved
- 🔵 **Testability**: First-row helpers cannot verify "exactly one row" —
  Resolved
- 🔵 **Clarity**: Context line ranges stale — Resolved
- 🔵 **Clarity**: "Expected" contradicts items that keep `noop` — Resolved
- 🔵 **Clarity**: Domain terms undefined — Partially resolved (order and
  watermark defined; row columns, "the run's watermark" and skill terms not)
- 🔵 **Dependency**: 0351 ordering not captured; missing from `relates_to` —
  Partially resolved (ordering captured; `relates_to` unchanged)
- 🔵 **Dependency**: Report consumers not enumerated — Resolved
- 🔵 **Clarity**: Partition sentence ambiguous — Resolved
- 🔵 **Clarity / Dependency**: `relates_to` disagrees; 0296 unexplained —
  Still present
- 🔵 **Testability**: Push-only test and run scopes not tied to a criterion —
  Partially resolved (push-only test in AC6; targeted vs whole-corpus not)

### New Issues Introduced

- 🟡 **Clarity**: "The run creates" is ambiguous between an attempted and a
  successful create — read as "succeeds", a failed create falls under "does
  not create", contradicting AC2 and AC5.
- 🔵 **Clarity**: Summary states the watermark effect unconditionally, though
  it matters only when the item already has a baseline entry.
- 🔵 **Clarity**: The "does not create" list in Requirements omits the
  matching-local-item condition from Technical Notes.
- 🔵 **Clarity**: Report row columns are never named.
- 🔵 **Clarity**: "The run's watermark" (AC6) is undefined.
- 🔵 **Clarity**: "Membership rule", "group" and "baseline entry" undefined;
  "no script parses" reads oddly beside the skill handling the row.
- 🔵 **Clarity**: "Whichever lands second updates the other's row assertions"
  is accurate in only one order.
- 🔵 **Testability**: AC5 can pass against unfixed code if the entry's
  watermark is seeded at or near the run start.
- 🔵 **Testability**: The unchanged `noop` + `order-refused` output for
  ordered drafts and `--pull-only` items is not pinned by a criterion.
- 🔵 **Testability**: No mixed-set (create + draft in one whole-corpus run)
  criterion.
- 🔵 **Scope**: The conditional 0351 test work is not in Requirements.
- 🔵 **Scope**: The every-row test helper is not in Changes or Size.
- 🔵 **Dependency**: Skill-rule removal and binary fix must release together.
  *Reviewer note: `bin/accelerator` fetches the binary matching the plugin
  version, so skill and binary are version-locked; a one-line Assumption
  suffices.*
- 🔵 **Dependency**: Sibling defects 0353 and 0354 not checked for overlap.
  *Reviewer note: 0353 also changes `/sync-work-items`.*
- 🔵 **Completeness**: The watermark case has no reproduction steps.

### Assessment

The five original majors are resolved, bar one that the rewording shifted
rather than removed: "the run creates" must name an attempted create
consistently (for example "plans a create for"). With one major, the work item
is acceptable for planning; the remaining minors — chiefly AC5's seeding
precondition, pinning the refused-order rows, and a mixed-set criterion — would
sharpen the tests the plan writes.

## Re-Review (Pass 3) — 2026-11-10T11:53:04+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity**: "The run creates" ambiguous between attempted and successful
  create — Resolved
- 🔵 **Clarity**: Summary states the watermark effect unconditionally — Still
  present
- 🔵 **Clarity**: "Does not create" list omits the matching-local-item
  condition — Resolved
- 🔵 **Clarity**: Report row columns never named — Not re-raised (wording
  unchanged)
- 🔵 **Clarity**: "The run's watermark" undefined — Still present (now framed
  as the watermark meaning both a per-entry and a run-wide value)
- 🔵 **Clarity**: Skill terms ("membership rule", "group") undefined — Still
  present
- 🔵 **Clarity**: "Whichever lands second" reads correctly in one order only —
  Still present
- 🔵 **Testability**: AC5 can pass against unfixed code if seeded near the run
  start — Not re-raised (wording unchanged)
- 🔵 **Testability**: Ordered draft / `--pull-only` `noop` + `order-refused`
  output not pinned — Still present (raised by clarity, scope and testability)
- 🔵 **Testability**: No mixed-set criterion — Still present (now framed as
  targeted vs whole-corpus run modes not pinned)
- 🔵 **Scope**: Conditional 0351 test work not in Requirements — Not re-raised
- 🔵 **Scope**: Every-row test helper not in Changes or Size — Not re-raised
  (wording unchanged)
- 🔵 **Dependency**: Skill-rule removal and binary fix must release together —
  Still present (suggestion)
- 🔵 **Dependency**: Siblings 0353 and 0354 not checked for overlap — Still
  present (suggestion)
- 🔵 **Completeness**: Watermark case has no reproduction steps — Not
  re-raised
- 🔵 **Clarity / Completeness / Dependency**: `relates_to` omits 0351; 0296
  unexplained — Still present
- 🔵 **Completeness / Testability**: No manual way to force a failed create —
  Still present

### New Issues Introduced

- 🔵 **Clarity / Scope**: Requirements say a no-create item "keeps its single
  `noop` row", but an ordered draft or `--pull-only` item also gets an
  `order-refused` row; AC3 narrows to items without an order, Requirements do
  not, and the out-of-scope exclusion lives only in Technical Notes.
- 🔵 **Testability**: "The watermark behaves as it does today" for no-create
  items has no criterion and does not say what today's behaviour is.
- 🔵 **Clarity / Testability / Completeness**: The literal row for a
  successful create or adopt in an apply run is never given; AC1 and AC4
  overlap and describe it differently.
- 🔵 **Clarity**: "Create" and "order" are each used in several senses
  (suggestion).

### Assessment

No critical or major findings remain: the pass-2 major is resolved. The work
item is ready for planning. The most useful remaining tightenings are to
qualify "keeps its single `noop` row" for ordered items and pin that output in
a criterion, give the literal successful-apply row, and state the no-create
watermark behaviour as a criterion. The frontmatter `relates_to` fix belongs
to `/update-work-item`.
