---
type: "work-item-review"
id: "0351-sync-preview-hides-pending-push-refusals-review-1"
title: "Work Item Review: Sync Preview Hides Pending-Push Refusals"
date: "2026-11-12T16:43:08+00:00"
author: "Phil Helm"
producer: "review-work-item"
status: "complete"
target: "work-item:0351"
work_item_id: "0351"
reviewer: "Phil Helm"
verdict: "REVISE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 1
tags: ["sync", "preview", "pending-push"]
last_updated: "2026-11-12T16:43:08+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Work Item Review: Sync Preview Hides Pending-Push Refusals

**Verdict:** REVISE

This is a tightly scoped bug with one root cause: the preview never reads the
pending-push marker. Its reproduction is complete, with observed rows and exit
codes, and its first acceptance criterion is fully specified. Two problems
need fixing before it is ready for planning. First, the reused-`Created`-marker
behaviour is described inconsistently, can only be tested as an absence, and
depends on an unresolved Open Question. Second, the item does not mention the
`sync-work-items` skill workaround that exists because of this bug and that
the fix will make wrong. The exit-code precedence requirement also has no
acceptance criterion.

### Cross-Cutting Themes

- **Reused `Created` marker is under-specified** (flagged by: clarity,
  testability, completeness, scope) — Requirements says the marker "previews
  as the link it would make", the criterion says "reports no new create", and
  the Open Question leaves the action word undecided. The case also falls
  outside the refusal-only title, Summary and reproduction.
- **The skill's manual marker-check workaround is an uncaptured co-change**
  (flagged by: dependency, completeness, scope, testability, clarity) —
  `skills/work/sync-work-items/SKILL.md` ("Read the run's output") says a
  preview "does not yet read a create's pending-push marker" and tells the
  agent to check `pending-push/<id>.json` by hand. The fix makes this text
  stale, and its content test pins the skill. The Context also overstates the
  user impact, because the skill already warns about the gap.
- **Apply-run parity is the only oracle for several criteria** (flagged by:
  testability, clarity) — the refusal variants, exit precedence and
  "unchanged" output are defined only by comparison with the apply run. Reason
  tokens and exact lines are not named.

### Findings

#### Major

- 🟡 **Clarity / Testability**: The preview of a reused `Created` marker is
  described inconsistently and is only asserted as an absence
  **Location**: Requirements / Acceptance Criteria / Open Questions
  The three sections use "reuse", "link" and "no new create" for the same
  behaviour. An implementation that drops the row, prints `noop` or keeps
  `create-from-local` would all pass the criterion as written.
- 🟡 **Dependency**: The skill workaround and its content test are an
  uncaptured co-change
  **Location**: Dependencies
  The skill's preview paragraph, its Step 2 list of what a preview carries,
  `tests/unit/tasks/test_sync_work_items_skill_structure.py` and possibly the
  changelog all need updating. "Blocks: none" hides this.
- 🟡 **Testability**: The exit-code precedence requirement has no acceptance
  criterion
  **Location**: Requirements
  The only exit-code check uses a single refused item. Hard-coding 71, or
  letting a conflict's 4 win, would pass every criterion.

#### Minor

- 🔵 **Testability / Clarity**: The refusal-variant criterion does not name
  the reason tokens or the exit code
  **Location**: Acceptance Criteria
  Name `marker-unreadable`, `fingerprint-mismatch` and `already-written`, and
  give the row, stderr and exit code for each. Also say whether the remedy hint
  after `…` must match.
- 🔵 **Clarity**: The preview's `would fail` line is credited to the apply run
  **Location**: Requirements
  The apply run prints `create failed: …`. State that the preview prints the
  same line with `failed` replaced by `would fail`.
- 🔵 **Clarity**: Project terms are used without definition
  **Location**: Context / Acceptance Criteria / Technical Notes
  "Bare create" and "create order" are easy to confuse. "Fingerprint" and
  "request digest" name the same thing. "Carries" is never defined.
- 🔵 **Clarity**: The claim about the skill's guidance contradicts the skill
  **Location**: Context
  The skill already tells users that a preview cannot confirm these items.
  Restate the impact to say that the preview needs this manual workaround.
- 🔵 **Completeness**: The reuse defect has no reproduction of its own
  **Location**: Summary
  No observed preview or apply output is recorded for the `Created`-marker
  case.
- 🔵 **Testability**: No criterion covers a key followed by this run's identity
  settlement
  **Location**: Acceptance Criteria
  `already-written` has two carriers. Either cover the second one or declare it
  out of scope.
- 🔵 **Testability**: "Output is unchanged" and "never writes" need concrete
  checks
  **Location**: Acceptance Criteria
  State the no-marker row and exit code. Scope the immutability check to the
  bytes in `pending-push/` before and after the preview, including the case
  where no marker exists.
- 🔵 **Dependency**: A preview that now exits 71 affects callers that gate on
  exit 0
  **Location**: Requirements
  Add a changelog note about the new preview exit code.
- 🔵 **Dependency**: 0300's landed code is recorded only as `relates_to`
  **Location**: Dependencies
  0300 merged in #169, so a line in Dependencies can say so.
- 🔵 **Dependency**: Possible overlap with siblings 0352–0354
  **Location**: Context
  0352 (PR #188) changes the skill's membership table row for a
  `create-from-local` item's duplicate `noop` row. This item turns some
  `create-from-local` rows into `failed` rows, so the two share that table.

#### Suggestions

- 🔵 **Scope**: Unclear whether the discovery skip is part of preview parity
  **Location**: Requirements
  Say whether the preview should report `#\tdiscovery\tskipped\tpending-push`
  when a refusal is found.
- 🔵 **Clarity**: "Every failure" overstates the preview's guarantee
  **Location**: Summary
  Narrow it to failures that can be determined from local state.

### Strengths

- ✅ The item covers one defect with one root cause, and its boundary is
  explicit: adopt and create orders are already previewed, no-marker output
  stays unchanged, and a preview writes no markers.
- ✅ The reproduction is complete and concrete: item 0004, the `TES` sandbox,
  `prior-attempt-unknown`, and exit 0 against exit 71.
- ✅ The first acceptance criterion is fully specified as an exact row, stderr
  substring and exit code.
- ✅ The Technical Notes point to the pure `push_precondition` function the fix
  can reuse, and the Assumptions explain why a preview may read a marker.
- ✅ Kind, priority and parent fit the work.

### Recommended Changes

1. **Settle the reuse case** (addresses: reused `Created` marker inconsistent /
   absence-only; reuse lacks reproduction; Open Question)
   Choose the action word, or keep `create-from-local` with the marker's key in
   column 4. Use one term throughout and rewrite the criterion positively with
   the exact row, no `error:` line, and exit 0. Widen the title and Summary to
   "preview ignores pending-push markers", or split the reuse case into its own
   item if it introduces a new action word.
2. **Add the skill co-change** (addresses: skill workaround co-change; skill
   claim contradicts skill; skill guidance not covered by a criterion)
   Add a Change and a criterion: drop the manual marker check at
   `skills/work/sync-work-items/SKILL.md` ("Read the run's output"), extend the
   preview guarantee, and update the skill's content test. Reword the Context
   impact accordingly. Note the overlap with 0352's membership-table change.
3. **Add an exit-precedence criterion** (addresses: exit-code precedence has no
   criterion; preview exit-code consumers)
   Use a mixed fixture, for example one `Attempted` marker plus one unresolved
   conflict, where the preview exits 71. Add a changelog line.
4. **Make each refusal variant explicit** (addresses: refusal-variant
   criterion; `would fail` line; undefined terms; identity-settlement carrier)
   Write one criterion per reason token, state the `would fail` line form,
   define "bare create", and use one word for the fingerprint.
5. **Tighten the negative criteria** (addresses: "unchanged" and "never
   writes"; discovery skip; "every failure")
   State the no-marker row and exit code, give the byte-level marker check,
   decide whether the discovery skip is in scope, and narrow the Summary's
   "every failure".

## Per-Lens Results

### Clarity

**Summary**: The item is mostly clear. It names concrete commands, rows, exit
codes and a reproduction, and the preview is clearly the actor in each
requirement. The main problem is the reused-`Created`-marker behaviour, which
the Requirements, the Acceptance Criteria and the Open Question describe in
ways that disagree. Some project terms are undefined, one line attributes the
preview's `would fail` wording to the apply run, and the claim about the
skill's guidance does not match the skill.

**Strengths**: Observable preview and apply outputs are set side by side. The
item gives a dated, concrete instance. The requirements are in active voice.
The Assumptions settle the read-during-preview question.

**Findings**:
- 🟡 major / high — Requirements / Acceptance Criteria / Open Questions: The
  preview of a reused `Created` marker is described inconsistently ("reuse",
  "link" and "no new create" are used for the same behaviour, and the action
  word is open).
- 🔵 minor / high — Requirements: The preview's `would fail` line is credited
  to the apply run, which prints `create failed`. It is unclear whether the
  trailing cause must match.
- 🔵 minor / medium — Context / Acceptance Criteria: Undefined terms ("orders",
  "bare create", "carries", "fingerprint" versus "request digest"), and no
  reason tokens in the second criterion.
- 🔵 minor / high — Context: The claim that the skill's guidance leads users
  into a refusal contradicts the skill's existing manual marker check.
- 🔵 suggestion / medium — Summary: "Every failure" overstates the preview's
  documented guarantee.

### Completeness

**Summary**: The item is structurally complete. All sections are present and
populated, the frontmatter is valid, and the reproduction is full. The gaps:
retiring the skill's stop-gap guidance is missing, the reuse behaviour has no
observed outcome, and an Open Question that decides expected output is
unresolved.

**Strengths**: The reproduction has all four elements. The Context gives the
discovery, root cause, scope and user harm. The criteria bound "done". All
supporting sections are populated.

**Findings**:
- 🔵 minor / high — Requirements: The Changes list omits retiring the skill's
  stop-gap guidance.
- 🔵 minor / medium — Open Questions: An unresolved question decides the
  expected output of a stated requirement.
- 🔵 minor / medium — Summary: The second defect (reused `Created` marker)
  has no reproduction of its own.

### Dependency

**Summary**: "Blocked by: none" and "Blocks: none" hide several couplings. The
most important is the `sync-work-items` skill: its prose and content test were
written around this bug. Smaller couplings: the release state of 0300's code,
overlap with siblings 0352–0354, and callers that gate on a preview exiting 0.

**Strengths**: The Context names the exact code dependencies. It separates out
the unaffected paths. The Assumptions show that no external dependency is
involved.

**Findings**:
- 🟡 major / high — Dependencies: The skill workaround, the Step 2 preview
  sentence, the content test, the recovery skill, the user guide and the
  changelog are an uncaptured co-change. A new action word would also change
  the skill's row contract.
- 🔵 minor / medium — Dependencies: Reliance on 0300's landed code is recorded
  only as `relates_to`. (0300 has since merged, in #169.)
- 🔵 minor / low — Context: Sibling defects 0352–0354 may touch the same
  preview and report path.
- 🔵 minor / medium — Requirements: Callers that rely on a preview exiting 0
  are not captured as consumers, and no changelog entry is planned.

### Scope

**Summary**: The item is tightly scoped to one defect in one path, `bug` is
the right kind, and the boundary is explicit. The remaining questions are at
the edges: Created-marker reuse goes beyond the refusal-only Summary, the skill
update is not mentioned, and discovery-skip parity is undecided.

**Strengths**: One root cause, delivered as a unit. The boundary is explicit.
The item is sized appropriately under parent 0146.

**Findings**:
- 🔵 minor / high — Requirements: Removing the skill's manual marker-check
  workaround is not in scope.
- 🔵 minor / medium — Summary: Created-marker reuse goes beyond the
  refusal-only title and Summary, and a new action word would widen the output
  contract.
- 🔵 suggestion / medium — Requirements: Unclear whether the
  `#\tdiscovery\tskipped\tpending-push` record is part of preview parity.

### Testability

**Summary**: The item is mostly verifiable, with a three-step reproduction and
a fully specified first criterion. Three of the five criteria define their
expected outcome only by comparison with the apply run or by absence. Exit
precedence has no criterion, and the reuse output depends on the Open
Question.

**Strengths**: The reproduction is complete. The first criterion is exact.
The apply run is a usable oracle. There are regression and immutability
guards.

**Findings**:
- 🟡 major / high — Acceptance Criteria: The reused `Created` marker criterion
  only asserts an absence.
- 🟡 major / high — Requirements: The exit-code precedence requirement has no
  acceptance criterion.
- 🔵 minor / high — Acceptance Criteria: The refusal-variant criterion does not
  name the reason tokens or the exit code.
- 🔵 minor / medium — Acceptance Criteria: No criterion covers a key followed
  by this run's identity settlement.
- 🔵 minor / medium — Acceptance Criteria: "Output is unchanged" and "never
  writes" need concrete checks.
- 🔵 suggestion / medium — Summary: The skill guidance described in the
  Summary is not covered by any criterion.

---
*Review generated by /accelerator:review-work-item*
