---
type: "work-item-review"
id: "0338-linear-update-overwrites-fields-it-was-not-given-review-1"
title: "Work Item Review: Linear Update Overwrites Fields It Was Not Given"
date: "2026-11-05T16:12:31+00:00"
author: "Phil Helm"
producer: "review-work-item"
status: "complete"
target: "work-item:0338"
work_item_id: "0338"
reviewer: "Phil Helm"
verdict: "REVISE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 1
tags: []
last_updated: "2026-11-05T16:12:31+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Work Item Review: Linear Update Overwrites Fields It Was Not Given

**Verdict:** REVISE

A well-scoped, well-evidenced bug: every symptom traces to one root cause
(the `issueUpdate` input is not built from exactly the flags given), the
reproduction is honest about what was observed versus inferred, and most
criteria pin an exact request input. It needs a revision pass at the edges —
whether sync's `update_op` path is off-limits is ambiguous, the
assignee/priority criterion checks presence rather than exact input, and the
`--state` JSON-output removal, skill edit and CHANGELOG entry are under-motivated
and unverified.

### Cross-Cutting Themes

- **Sync's `update_op` boundary is a hint, not a requirement** (flagged by:
  clarity, scope, testability) — Context and Technical Notes say `work sync` is
  unaffected and a fix reaching `update_op` "breaks it", but this is neither in
  Out of scope nor an acceptance criterion, and "breaks it" reads as either a
  prohibition or a heads-up.
- **`--state` JSON output removal is unexplained and under-tracked** (flagged
  by: clarity, scope, dependency, completeness) — "the JSON a `--state` update
  printed goes" changes an output contract without saying why the fix needs
  it, without a reproduction step for `--state`, and without a CHANGELOG
  `Changed` entry for other consumers.
- **Skill and CHANGELOG deliverables have no criteria** (flagged by:
  completeness, testability) — two of the five Changes could be left undone
  with every box ticked.
- **"Request" and "as typed" are loosely defined** (flagged by: clarity,
  testability) — unclear whether catalogue lookups count as requests, and
  whether `priority` is sent as the integer `1` or the string `"1"`.
- **Combined-input assumption is unverified** (flagged by: completeness,
  dependency, testability) — Linear accepting `assigneeId` and `priority`
  alongside the rest in one input has no evidence or verification step,
  unlike `stateId`.

### Findings

#### Major

- 🟡 **Testability**: Assignee/priority criterion checks presence, not exact
  input, leaving reproduction step 3 unverified
  **Location**: Acceptance Criteria
  "Both appear in the request's input" would pass an implementation still
  sending `title: ""` alongside `priority`; no criterion pins
  `update <KEY> --priority 1` to an exact input.
- 🟡 **Clarity**: Unclear whether the fix may change `update_op` and the sync
  behaviour pinned in port.rs
  **Location**: Technical Notes
  "A fix that reaches `update_op` breaks it" can be read as a prohibition or a
  heads-up, and bare `update` names both the CLI subcommand and the tracker
  port's method.

#### Minor

- 🔵 **Completeness / Testability**: Skill Step 4 and CHANGELOG changes have no
  acceptance criterion
  **Location**: Acceptance Criteria
  Two of the five listed Changes have no matching criterion.
- 🔵 **Clarity / Testability**: "The request" does not say whether catalogue
  lookups count
  **Location**: Acceptance Criteria
  "One request is sent" and "refuses before any request" read differently if
  `--state` resolution contacts Linear.
- 🔵 **Clarity**: "With each value as typed" could mean verbatim strings or
  native GraphQL types
  **Location**: Requirements
  `priority` could be sent as `"1"` or `1`; the presence-only criterion
  accepts either.
- 🔵 **Clarity / Scope / Dependency**: Removal of the `--state` JSON output is
  terse, unmotivated, and its consumers unlisted
  **Location**: Requirements
  Nothing says why folding `--state` into one `issueUpdate` requires changing
  the success output, or whether anything besides the skill parses the JSON;
  the CHANGELOG requirement records only the `Fixed` case.
- 🔵 **Completeness**: Reproduction does not cover every defect the Summary
  claims
  **Location**: Requirements
  No reproduction step for `--state` with another flag, nor for
  `--assignee-id`.
- 🔵 **Dependency**: Linear `issueUpdate` schema dependency rests on an
  unverified assumption
  **Location**: Assumptions
  `stateId` is proven via `transition`; `assigneeId` and `priority` are not.
- 🔵 **Testability**: Success-output criterion is unbounded and does not
  account for `--quiet`
  **Location**: Acceptance Criteria
  "Any successful `update`" leaves the verifier to choose cases, and the
  skill's `--quiet` flag may contradict it.
- 🔵 **Testability**: No criterion guards sync's unchanged `update` behaviour
  **Location**: Acceptance Criteria
  An implementation changing `update_op` and editing `port.rs` to match would
  meet every criterion.
- 🔵 **Testability**: No check against real Linear for the combined-input
  assumption or the unobserved reproduction steps
  **Location**: Assumptions
  Every criterion can pass against a test double while real updates are still
  rejected.

#### Suggestions

- 🔵 **Scope**: The rule that work sync is unchanged sits outside the
  out-of-scope list
  **Location**: Requirements
  The item's most important boundary is only an implementation hint.
- 🔵 **Completeness**: Unverified Linear API assumption has no route to being
  confirmed
  **Location**: Assumptions
  No Open Questions entry or fallback if a field cannot be combined.
- 🔵 **Clarity**: Exit `34` is attributed to Linear but is presumably the CLI's
  own exit code
  **Location**: Summary
  The meaning of `34` is never stated.

### Strengths

- ✅ Every symptom traces to one root cause with a single expected outcome
  ("each command changes exactly the fields named, and no other"), making it a
  coherent, single-rollback unit.
- ✅ Summary, Changes and Acceptance Criteria align closely; each symptom has a
  matching change and criterion.
- ✅ Reproduction gives exact commands, exit codes and outcomes, and honestly
  separates observed (step 1) from inferred (steps 2–3).
- ✅ Most criteria pin an exact request input, catching both extra and missing
  fields; the `--description ""` criterion targets the absent-vs-empty edge
  where the bug lives.
- ✅ Explicit Out of scope list closes off error reporting, local validation,
  unassigning and `linear transition`.
- ✅ Context explains the data-loss risk via the skill's confirm-then-write
  flow, justifying high priority.
- ✅ Technical Notes give exact source locations, the existing test gap, and
  the sync test that pins `update_op`; `stateId` reuse is backed by
  `transition`.
- ✅ The skill — the one in-repo consumer of the changed output — is listed as
  a required change; frontmatter is complete and consistent.

### Recommended Changes

1. **Make the sync boundary a requirement** (addresses: Unclear whether the fix
   may change `update_op`; No criterion guards sync's unchanged behaviour; The
   rule that work sync is unchanged sits outside the out-of-scope list)
   Add to Out of scope: "the tracker port's `update`/`retitle` and `update_op`'s
   input are unchanged", add a criterion that `port.rs`'s pinned `update` input
   is unchanged and passes, and reword the Technical Note as a constraint.
   Distinguish `linear update` (CLI) from the port's `update` throughout.
2. **Pin exact inputs for `--priority` and `--assignee-id`** (addresses:
   Assignee/priority criterion checks presence; "With each value as typed")
   Replace the presence criterion with `--priority 1` → `{"priority": 1}` and
   `--assignee-id <ID>` → `{"assigneeId": "<ID>"}`, and define "as typed" (string
   fields verbatim including empty; `priority` as integer).
3. **Define "request" as the `issueUpdate` mutation** (addresses: "The request"
   does not say whether catalogue lookups count)
   Reword `--state` criteria as "exactly one `issueUpdate` mutation, input
   exactly `{"stateId": "<resolved>", "title": "x"}`" and "no `issueUpdate`
   mutation is sent", with the refusal's exit code.
4. **Motivate and track the `--state` output change** (addresses: Removal of the
   `--state` JSON output; Reproduction does not cover every defect)
   Say in Context what `--state` prints today and why one `issueUpdate` makes
   `updated\t<KEY>` the only success output; state whether other consumers
   exist; add a `Changed` CHANGELOG entry; add a `--state <NAME> --title "x"`
   reproduction step.
5. **Add criteria for the skill and CHANGELOG** (addresses: Skill Step 4 and
   CHANGELOG changes have no acceptance criterion)
6. **Bound the success-output criterion** (addresses: Success-output criterion
   is unbounded) — name the cases, cover `--quiet`, and give exit 0 for each.
7. **Give the combined-input assumption a verification route** (addresses:
   Linear schema assumption; No check against real Linear)
   Cite `IssueUpdateInput` schema evidence in Technical Notes, and/or add a
   merge-time scratch-issue check running the reproduction commands plus a
   combined `--state --priority --assignee-id` update.
8. **Explain exit `34`** (addresses: Exit `34` is attributed to Linear) — name
   the CLI failure category on first use.

## Per-Lens Results

### Clarity

**Summary**: Mostly precise; Summary, reproduction and criteria describe the
defect consistently. The main ambiguity is the CLI/sync boundary: `update` names
both the subcommand and the port method, and "a fix that reaches `update_op`
breaks it" reads as prohibition or heads-up. Smaller looseness around "the
request", "as typed", exit `34` and the unexplained JSON removal.

**Strengths**:
- Summary enumerates each faulty behaviour precisely, restated consistently in
  reproduction and criteria.
- Observed vs inferred reproduction steps are marked.
- Most criteria give exact `issueUpdate` inputs.
- Out of scope explicitly excludes neighbouring behaviours.

**Findings**:
- 🟡 major / medium — Technical Notes — Unclear whether the fix may change
  `update_op` and the sync behaviour pinned in port.rs. "It" could be the test
  or sync; the CLI-only builder is suggested, not required. State whether
  sync's input must stay unchanged and distinguish `linear update` from the
  port's `update`.
- 🔵 minor / medium — Acceptance Criteria — "The request" does not say whether
  catalogue lookups count. Define it once as the `issueUpdate` mutation.
- 🔵 minor / medium — Requirements — "With each value as typed" could mean
  verbatim strings or native GraphQL types; `priority` as `"1"` or `1`.
- 🔵 minor / medium — Requirements — Removal of the `--state` JSON output is
  terse and unmotivated; add what it prints today and why the fix removes it.
- 🔵 suggestion / medium — Summary — Exit `34` is attributed to Linear but is
  presumably the CLI's exit code; name its category on first use.

### Completeness

**Summary**: Every expected section is present and substantive; bug content
(reproduction, expected/actual, impact, Changes, eight Given/When/Then criteria)
is strong. Gaps: reproduction covers only some Summary defects, two Changes lack
criteria, and the unverified assumption has no confirmation route.

**Strengths**:
- Numbered reproduction with explicit expected outcome.
- Context explains why it matters via the PP-714 repair and the skill flow.
- Observed vs inferred steps flagged.
- Explicit Out of scope list.
- Complete, consistent frontmatter.
- Technical Notes point to exact sources and tests.

**Findings**:
- 🔵 minor / high — Requirements — Reproduction does not cover every defect the
  Summary claims; add a `--state <NAME> --title "x"` step (and ideally
  `--assignee-id`).
- 🔵 minor / high — Acceptance Criteria — Skill and CHANGELOG changes have no
  acceptance criterion.
- 🔵 suggestion / medium — Assumptions — Unverified Linear API assumption has no
  route to being confirmed; note how it will be checked or a fallback.

### Dependency

**Summary**: Self-contained CLI bug fix; "Blocked by: none" / "Blocks: none"
are credible. Two implicit couplings: the fix depends on Linear `issueUpdate`
accepting multiple fields in one input (an unverified Assumption), and removing
`--state` JSON output changes a CLI output contract whose only named consumer is
the skill.

**Strengths**:
- Coupling with `work sync` via `update_op` is explicit, including the pinning
  test.
- The skill's Step 4 is listed as a required change.
- `relates_to` and origin are recorded; empty blockers are justified.
- `stateId` reuse is evidenced by `transition`.

**Findings**:
- 🔵 minor / medium — Assumptions — Linear `issueUpdate` schema dependency rests
  on an unverified assumption; cite `IssueUpdateInput` evidence or record it as
  an external dependency to confirm.
- 🔵 minor / medium — Requirements — Removing the `--state` JSON output has
  consumers beyond the skill that are not listed; state whether others exist and
  add a CHANGELOG `Changed` entry.

### Scope

**Summary**: Well-scoped bug: every defect stems from one root cause, Summary,
Requirements and criteria cover the same four symptoms, and the out-of-scope
line sets clear limits. Minor: the `--state` output change rides along
unexplained, and the sync-unchanged constraint lives only in Technical Notes.

**Strengths**:
- One root cause, one expected outcome; coherent and single-rollback.
- Tight alignment between Summary, Changes and criteria.
- Explicit out-of-scope fence.
- `bug`/`high` fits a contained CLI fix.
- Skill and CHANGELOG changes are necessary consequential changes.

**Findings**:
- 🔵 minor / medium — Requirements — Removing the `--state` JSON output is
  bundled in without saying why the fix needs it; justify it or split it out.
- 🔵 suggestion / medium — Requirements — The rule that work sync is unchanged
  sits outside the out-of-scope list; add it there or as a criterion.

### Testability

**Summary**: Highly testable: exact commands, exit codes and outcomes, and
mostly exact request inputs. Gaps at the edges: `--priority`/`--assignee-id`
are checked for presence only, skill and CHANGELOG have no criteria, and
"request" is ambiguous when `--state` resolution is involved.

**Strengths**:
- Reproduction gives exact command, expected and actual outcome, honest about
  observation.
- Exact-input criteria catch both extra and missing fields.
- `--description ""` criterion targets the absent-vs-empty edge.
- `E_UPDATE_NO_OPS` refusal preserved as a criterion.
- Out of scope prevents over-expectation.
- Technical Notes name the test file gap and the sync pinning test.

**Findings**:
- 🟡 major / high — Acceptance Criteria — Assignee/priority criterion checks
  presence, not exact input, leaving reproduction step 3 unverified. Use
  `{"priority": 1}` and `{"assigneeId": "<ID>"}` exact inputs and state
  `priority`'s JSON type.
- 🔵 minor / high — Acceptance Criteria — Skill Step 4 and CHANGELOG
  requirements have no criteria.
- 🔵 minor / medium — Acceptance Criteria — "Request" is not defined when state
  resolution may itself contact Linear; reword around the `issueUpdate`
  mutation.
- 🔵 minor / medium — Acceptance Criteria — Success-output criterion is
  unbounded and does not account for `--quiet`.
- 🔵 minor / medium — Acceptance Criteria — No criterion guards sync's unchanged
  `update` behaviour.
- 🔵 minor / low — Assumptions — No check against real Linear for the
  combined-input assumption or the unobserved reproduction steps; add a
  merge-time scratch-issue check.

---
*Review generated by /accelerator:review-work-item*
