---
type: "work-item-review"
id: "0319-explicit-work-item-id-on-accelerator-work-create-review-1"
title: "Work Item Review: Explicit Work Item ID on accelerator work create"
date: "2026-10-28T21:33:39+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0146"
target: "work-item:0319"
work_item_id: "0319"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-10-29T10:32:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Explicit Work Item ID on accelerator work create

**Verdict:** REVISE

0319 is one coherent, well-bounded story with concrete Given/When/Then
criteria, explicit scope limits and named code seams. It needs revision
where the refusals interact: no order of checks is stated, so the
draft-reservation code may be unobservable and overlapping refusals have
no defined code. Dry-run failure behaviour and the skill's failure paths
are also unspecified. The remaining findings are boundary examples and
linkage hygiene.

### Cross-Cutting Themes

- **Refusal precedence** (flagged by: clarity, testability) — several
  inputs trip more than one refusal, and every canonical draft ID also
  fails the pattern check, so the distinct-code criterion is
  unsatisfiable or trivially passed until the check order is stated.
- **Dry-run outcome** (flagged by: clarity, testability, dependency) — the
  criteria say the checks "run" and the scan is "advisory", but not
  whether a failed check exits non-zero and suppresses the preview, or
  whether the checks run without a reachable tracker.
- **Meaning of "taken"** (flagged by: clarity, dependency, testability) —
  Requirement 3 says "uses", and the criteria say "filename uses"; aliases,
  other keys and letter case have no stated outcome.
- **Skill surface** (flagged by: completeness, scope, testability) — the
  Summary and Requirements under-describe the skill work the criteria
  demand, and no criterion covers the skill's response to a CLI refusal.
- **Epic linkage** (flagged by: completeness, scope) — 0146's Stories list
  omits 0319, and the placement rationale cites code locality rather than
  the epic's ID-model theme.

### Findings

#### Major

- 🟡 **Clarity + Testability**: No order of checks, so overlapping refusals
  and the draft-reservation code are undefined
  **Location**: Acceptance Criteria
  A canonical draft ID also fails the pattern check, and `--id --project`
  under `{tracker}` trips three refusals. Unless the order is stated, the
  draft-reservation code may never be observable.
- 🟡 **Testability**: The distinct-code criterion leaves the refusal classes
  ungrouped
  **Location**: Acceptance Criteria
  Read literally, the two collision criteria and the two mismatch criteria
  would each need their own code. It is also unstated whether
  `E_PATTERN_KEY_UNUSED` is reused for `--id --project`.
- 🟡 **Clarity + Testability**: Dry-run criteria describe steps, not
  observable outcomes
  **Location**: Acceptance Criteria
  A dry-run that prints a warning and exits 0, and one that exits non-zero
  with no preview, both satisfy the text. "Without taking the create lock"
  is an internal property.
- 🟡 **Testability**: Skill criteria cover only the happy path
  **Location**: Acceptance Criteria
  A skill that silently retries without `--id` after a CLI refusal passes
  every listed criterion. Handling of non-exact requests such as "400" is
  also unstated.

#### Minor

- 🔵 **Clarity + Dependency + Testability**: "Taken" is loosely defined
  **Location**: Requirements
  Requirement 3 does not say whether only the filename prefix counts, or
  also frontmatter `id` and `aliases` (retired IDs from 0230/0302/0343).
  Different-key and different-case boundaries have no examples.
- 🔵 **Completeness + Scope**: Summary and Requirements under-describe the
  skill work
  **Location**: Summary, Requirements
  The projected-ID presentation, the pre-drafting `{tracker}` refusal and
  the per-refusal codes appear only in the criteria.
- 🔵 **Dependency + Testability**: Push failure paths and tracker coupling
  not captured
  **Location**: Acceptance Criteria, Dependencies
  Only a successful push and a collision refusal are covered. There is
  nothing for a rejected or unreachable tracker writing the unsynced file
  under the given ID, nor for the other refusals under `--push`.
- 🔵 **Dependency**: Shared `run_with` surface has no named sibling
  **Location**: Drafting Notes
  Sibling stories that restructure `run_with` or the push path are not
  named, so sequencing risk is invisible.
- 🔵 **Clarity**: The draft form is delegated to `canonical_draft_id`, and
  "in any case" has two readings
  **Location**: Requirements
  The reserved form is never described, and "in any case" can mean letter
  case or every circumstance.
- 🔵 **Clarity**: "The pattern's key syntax" is undefined
  **Location**: Requirements
  The `work.key: draft` precondition suggests acceptance of lower-case keys
  depends on `work.key`.
- 🔵 **Clarity**: "The `--id` refusal can follow the same check" is
  ambiguous
  **Location**: Technical Notes
  It is unclear which refusal is meant, and "follow" could mean reusing the
  existing error, which contradicts distinct codes.
- 🔵 **Testability**: Pattern-mismatch criterion has no edge examples
  **Location**: Acceptance Criteria
  A missing key, a disallowed key, `04000`, `10000` and `0000` have no
  expected outcome.
- 🔵 **Testability**: Concurrency criterion relies on a race and
  unspecified slugs
  **Location**: Acceptance Criteria
  With the same title, the old exact-path guard refuses the second run, so
  the new scan stays untested.
- 🔵 **Testability**: Interaction with automatic allocation is never checked
  **Location**: Assumptions
  The max+1-after-`--id` behaviour has no criterion, and an `--id` at the
  cap (`9999`) is unaddressed.
- 🔵 **Completeness + Scope**: 0146 does not list 0319
  **Location**: Frontmatter: parent
  The parent link is one-way, and its rationale cites code locality rather
  than 0146's ID model (0230, 0302).

#### Suggestions

- 🔵 **Completeness**: Scope exclusion and the allocation rule sit outside
  Requirements
  **Location**: Technical Notes, Assumptions
  Move the `create-batch` exclusion and the max+1 rule to where readers
  look for boundaries.
- 🔵 **Dependency**: 0228 and 0230 contracts not recorded
  **Location**: Frontmatter: relates_to
  The `{key}` syntax and draft-ID form come from those stories; 0312 is
  origin, not coupling.
- 🔵 **Clarity**: Internal terms are unexplained
  **Location**: Context
  "The `NNNN` substitution" and "projected ID" assume knowledge of the
  template and the skill.

### Strengths

- ✅ Every requirement serves one purpose, letting a caller name the ID
  `work create` writes, with no unrelated work bundled in.
- ✅ Scope limits are explicit and justified: local-only collisions, max+1
  without gap-filling, and `create-batch` excluded.
- ✅ Criteria are concrete Given/When/Then pairs with observable outcomes,
  and paired counter-examples (`--id 400`, `draft-0400`) pin edges.
- ✅ The push-collision criterion asserts that no create is sent, which a
  recording tracker double can verify directly.
- ✅ Technical Notes name the real seams (`allocate_id`,
  `refuse_to_overwrite`, `run_with`, `IdOwnership::Tracker`), and Drafting
  Notes explain exact over fuzzy matching.
- ✅ Each refusal states its reason, so intent survives implementation.

### Recommended Changes

1. **State the order of `--id` checks** (addresses: no order of checks;
   distinct-code grouping; "follow the same check")
   Add a requirement fixing the order, with draft reservation before the
   pattern and `{tracker}` checks. Group the refusal classes, say whether
   `--id --project` reuses `E_PATTERN_KEY_UNUSED`, and add one overlapping
   case.
2. **Rewrite the dry-run criteria as outcomes** (addresses: dry-run steps;
   tracker coupling)
   On a failed check, the dry-run exits non-zero with the same `E_` code,
   prints no preview and writes nothing. A valid ID prints the preview and
   exits 0. Recast the lock clause as "while another process holds the
   create lock, without waiting".
3. **Add skill failure criteria** (addresses: skill happy path only;
   Summary and Requirements under-describe the skill)
   Relay the CLI refusal verbatim, never fall back to automatic allocation,
   and state how non-exact requests are handled. Widen the Summary and the
   skill requirement to match.
4. **Pin "taken"** (addresses: "taken" loosely defined)
   Write "filename prefix only" into Requirement 3, and state the outcome
   for aliases, a different key and a different-case key.
5. **Add boundary and failure-path criteria** (addresses: mismatch edges;
   concurrency; allocation interaction; push failure)
   Cover the mismatch examples, a deterministic concurrency setup with a
   different slug, the next automatic ID after `--id`, and `--id --push`
   with a rejected create.
6. **Tidy linkage** (addresses: 0146 linkage; 0228/0230; shared surface)
   Add 0319 to 0146's Stories, add 0228 and 0230 to `relates_to`, and name
   0351 as a sibling that also restructures `run_with`.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Tightly written, with a shared intent across sections and
observable outcomes. The gaps are in how refusals interact and what a
failed check means under `--dry-run`, plus boundaries handed to undefined
rules (`canonical_draft_id`, "the pattern's key syntax") and a loose
meaning of "used".

**Strengths**:
- Scope boundaries are stated outright, with a worked max+1 example.
- Each refusal gives its reason.
- Concrete pattern, key and ID combinations settle foreign-key acceptance.
- Drafting Notes answer the exact-versus-fuzzy question.
- Outcomes are observable system states.

**Findings**:
- 🟡 major, medium — Acceptance Criteria — Unclear which refusal wins when
  several apply. The draft-form ID under `{number:04d}` also mismatches,
  `{tracker}` may count as a mismatch, and `--id --project` under
  `{tracker}` hits three refusals. If the dry-run list is the check order,
  the pattern check blocks the others.
- 🟡 major, medium — Acceptance Criteria — Unclear what a failed check does
  under dry-run. "Reports", "prints the preview" and "advisory" allow
  either a warning with exit 0 or a refusal with no preview, and the
  criteria mix `--push --dry-run` with bare `--dry-run`.
- 🔵 minor, medium — Requirements — "Used" shifts between sections. It is
  unclear whether frontmatter `id` or `aliases` count, given 0146 moves
  retired IDs into `aliases`.
- 🔵 minor, high — Requirements — The draft form is delegated to
  `canonical_draft_id` without a description, and "in any case" is
  ambiguous.
- 🔵 minor, medium — Requirements — "The pattern's key syntax" is undefined,
  and the `work.key: draft` precondition hints that acceptance depends on
  `work.key`.
- 🔵 minor, medium — Technical Notes — "The `--id` refusal can follow the
  same check" names no refusal, and "follow" may imply reusing the
  existing code.
- 🔵 suggestion, medium — Context — "`NNNN` substitution" and "projected ID"
  are unexplained.

### Completeness

**Summary**: Structurally complete. Every section has real content, the
Summary is a proper user story and the Context explains the forces behind
it. Small gaps: some behaviour appears only in the criteria, and 0146 does
not list 0319.

**Strengths**:
- The Summary names the role, the capability and the benefit.
- The Context explains why, including the costs of the workaround.
- Requirements enumerate the full refusal surface.
- The criteria cover the happy paths, refusals, push, dry-run, concurrency
  and the skill.
- Assumptions pre-empt the cross-branch question.
- Technical and Drafting Notes tie the work to named seams and
  rationale.
- The frontmatter is complete and plausible.

**Findings**:
- 🔵 minor, medium — Requirements — The skill and error-reporting behaviour
  appear only in the criteria: projected-ID presentation, the pre-drafting
  `{tracker}` refusal, distinct codes, named file or pattern, and the
  lock-free dry-run scan.
- 🔵 suggestion, medium — Technical Notes — The scope exclusion and the
  max+1 rule sit outside Requirements.
- 🔵 minor, medium — Frontmatter: parent — 0146's Stories list omits 0319.

### Dependency

**Summary**: Mostly self-contained, and "none/none" is broadly justified.
The gaps are uncaptured couplings to 0146 siblings: the alias contract
(0230/0302/0343), unlinked contract sources (0228/0230), an unnamed shared
push surface, and the tracker coupling of `--push`/`--dry-run`.

**Strengths**:
- Explicit "none" entries without invented couplings.
- Cross-branch coupling is bounded in Assumptions.
- Existing seams are named in Technical Notes.
- Other allocation callers are explicitly excluded.
- The parent link carries a stated reason.

**Findings**:
- 🔵 minor, medium — Requirements — Coupling to the ID-retirement and alias
  contract (0230, 0302, 0343) is not captured. It is unclear whether a
  retired alias counts as taken, or how `work rekey` treats foreign-key
  items.
- 🔵 suggestion, high — Frontmatter: relates_to — The 0228 (`{key}`) and
  0230 (`{tracker}`, draft ID) contracts are not recorded. 0312 is origin,
  not coupling.
- 🔵 minor, medium — Drafting Notes — The shared `work create` push surface
  has no named sibling, so sequencing risk is invisible.
- 🔵 minor, medium — Acceptance Criteria — The tracker dependency of
  `--push`/`--dry-run` is not captured. Dry-run needs `work.integration`
  today, and push failure paths are unspecified.

### Scope

**Summary**: One coherent, appropriately sized story. All requirements and
criteria concern a single `--id` option, with explicit exclusions. Small
signals: the Summary names only the CLI, and the parent rationale rests on
shared code.

**Strengths**:
- Requirements share one purpose, with no bundled refactors.
- Boundaries are explicit and justified.
- The criteria go deep on one flag, not wide across capabilities.
- `story` fits: one team, one component, one increment.
- Dependent guards already exist.

**Findings**:
- 🔵 minor, medium — Summary — The Summary names only the CLI, while the
  scope includes skill behaviour.
- 🔵 suggestion, low — Frontmatter: parent — The placement rests on code
  locality rather than 0146's ID-model theme (0230, 0302), and 0146 does
  not list 0319.

### Testability

**Summary**: Testable overall, with concrete Given/When/Then and checkable
outcomes. The gaps are process-described dry-run and distinct-code
criteria with no order of checks, so the draft-reservation code may be
unobservable. Unspecified skill failure paths, and boundaries with no
expected outcome: keys, case, number edges and a deterministic
concurrency setup.

**Strengths**:
- Concrete preconditions, actions and outcomes.
- Negative criteria assert a non-zero exit and "writes nothing", plus a
  named file.
- Paired boundary criteria (`400`, `draft-0400`).
- The no-send assertion is verifiable via a recording tracker.
- Assumptions avoid untestable cross-branch claims.

**Findings**:
- 🟡 major, medium — Acceptance Criteria — The draft-reservation refusal may
  be unobservable. Every canonical draft ID also fails the pattern or
  `{tracker}` check, and no order is stated.
- 🟡 major, high — Acceptance Criteria — The distinct-code criterion names
  no codes, grouping or precedence, and overlapping inputs have no
  expected code.
- 🟡 major, high — Acceptance Criteria — The dry-run criteria describe
  internal steps rather than observable outcomes, and the lock clause is
  internal.
- 🟡 major, medium — Acceptance Criteria — The skill criteria cover only the
  happy path, with no response to CLI refusals and no handling of
  non-exact input.
- 🔵 minor, medium — Acceptance Criteria — The collision boundary is not
  pinned for other keys, letter case or non-filename holders.
- 🔵 minor, medium — Acceptance Criteria — The pattern-mismatch criterion
  has no edge examples (missing key, `04000`, `10000`, `0000`).
- 🔵 minor, medium — Acceptance Criteria — The concurrency criterion relies
  on a race, and the same-title case is caught by the old guard.
- 🔵 minor, medium — Acceptance Criteria — Push compatibility is checked
  only for success and collision, with no failure path and no other
  refusals under `--push`.
- 🔵 minor, medium — Assumptions — The interaction with automatic
  allocation is never checked, and an `--id` at the cap is unaddressed.

## Re-Review (Pass 2) — 2026-10-29T01:57:39+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity + Testability**: No order of checks — Resolved; only one
  ordering edge has a criterion
- 🟡 **Testability**: Distinct-code grouping — Resolved; code names remain
  deferred to planning by decision
- 🟡 **Clarity + Testability**: Dry-run steps, not outcomes — Resolved
- 🟡 **Testability**: Skill happy path only — Partially resolved; the
  absence claim and the exact-form and keyed confirmation cases remain
- 🔵 **Clarity + Dependency + Testability**: "Taken" loosely defined —
  Resolved
- 🔵 **Completeness + Scope**: Skill work under-described — Resolved
- 🔵 **Dependency + Testability**: Push failure and tracker coupling —
  Partially resolved; exit outcome and retryable failure unstated
- 🔵 **Dependency**: Shared `run_with` sibling unnamed — Resolved; landing
  order with 0351 still open
- 🔵 **Clarity**: Draft form delegated, "in any case" — Still present
- 🔵 **Clarity**: Key syntax undefined — Still present, now entangled with
  letter case
- 🔵 **Clarity**: "Follow the same check" — Resolved
- 🔵 **Testability**: Mismatch edges — Resolved
- 🔵 **Testability**: Concurrency race — Resolved
- 🔵 **Testability**: Allocation interaction — Resolved for `{number:04d}`
- 🔵 **Completeness + Scope**: 0146 linkage — Resolved
- 🔵 **Completeness**: Scope exclusion placement — Still present
- 🔵 **Dependency**: 0228 and 0230 unrecorded — Resolved
- 🔵 **Clarity**: Unexplained terms — Still present

### New Issues Introduced

- 🔵 **Dependency**: `--id XY-0400` beside `PP-0400` creates corpora that
  0302's re-key refuses, since convergence keeps the sequence number
- 🔵 **Scope + Dependency**: No landing order with 0351, which also changes
  the skill's dry-run call and the real create's check order
- 🔵 **Clarity + Testability**: "Exact" ID versus case-insensitive taken
  check; lowercase key validity unstated, so the `xy-0400` criterion may
  be unreachable
- 🔵 **Clarity**: The `xy-0400` and `PP-0400` collision criteria omit their
  `id_pattern`
- 🔵 **Clarity**: Technical Notes give conflicting accounts of where the
  collision check runs relative to `creation_context` and the lock
- 🔵 **Testability**: The `draft-0400` exception is excluded by suffix
  length (six characters), so it never exercises the digit-only rule
- 🔵 **Testability**: Accepted bounds `0001` and `9999` and the overflow
  consequence have no criteria
- 🔵 **Testability**: Next automatic ID after a different-key `--id` is
  unspecified
- 🔵 **Clarity**: "Drafting" in the skill requirement collides with
  0230's provisional drafts
- 🔵 **Completeness**: Context omits the skill half; References lag
  `relates_to`; Requirements say only "compatible" for push and dry-run
- 🔵 **Scope**: 0146's own Requirements name no ID-model theme
- 🔵 **Clarity**: Dry-run "output matches" is ambiguous if the preview
  shows an ID; today's preview line carries none

### Assessment

All four majors are resolved or reduced to minor residue, so the item is
acceptable for planning. Two new findings warrant decisions before
implementation: the re-key conflict with 0302 and the landing order with
0351. The `draft-0400` criterion's rationale is also wrong and should be
corrected. The rest is wording and additional boundary criteria.

## Re-Review (Pass 3) — 2026-10-29T08:47:40+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🔵 **Dependency**: Re-key conflict with 0302 — Resolved; inherited from
  `--project`, recorded under Coordinates
- 🔵 **Scope + Dependency**: Landing order with 0351 — Resolved; 0319
  first, though 0351's frontmatter does not mirror the block
- 🔵 **Clarity + Testability**: Letter case and key syntax — Partially
  resolved; key syntax stated, but key identity across case is undefined
  and writing the given case is untested
- 🔵 **Clarity**: Collision criteria omit their pattern — Resolved
- 🔵 **Clarity**: Collision check placement — Resolved
- 🔵 **Testability**: `draft-0400` rationale — Resolved
- 🔵 **Testability**: Bounds and overflow — Resolved
- 🔵 **Testability**: Next ID after a different-key `--id` — Resolved for
  `work.key`; the introduced key's own sequence is untested
- 🔵 **Clarity**: "Drafting" overloaded — Resolved
- 🔵 **Completeness**: Context, References, "compatible" — Resolved
- 🔵 **Scope**: 0146 ID-model theme — Resolved
- 🔵 **Clarity**: Dry-run output ambiguity — Resolved

### New Issues Introduced

- 🟡 **Clarity**: "Code" means both the `E_` identifier and the process
  exit status, and the new refusals' exit status is unstated
- 🟡 **Testability**: Writing the ID in the letter case given has no
  criterion
- 🔵 **Clarity + Testability**: The skill's proposal rule is undefined for
  requests carrying their own key or with no exact form
- 🔵 **Clarity**: Declined proposal, CLI refusal and `{tracker}` refusal
  have inconsistent next steps
- 🔵 **Clarity + Testability**: Whether `pp` and `PP` are one key for
  allocation is undefined
- 🔵 **Completeness + Dependency**: How the user supplies the ID to the
  skill, and who adds `--id` to its push-gate dry-run, are unstated
- 🔵 **Dependency**: 0351 lacks a mirrored `blocked_by`; the 0343 coupling
  is misattributed (0230 supplies `aliases`)
- 🔵 **Clarity**: Two refusals have no stated cause to name; "outcome
  keyword" undefined; one criterion's trigger contradicts its outcome;
  out-of-range numbers not tied to the mismatch refusal
- 🔵 **Testability**: `--push` no-request, dry-run pass under `--push`,
  unreachable-tracker parity, dry-run lock hold, `NNNN` residue,
  `--id`/`--project` across patterns, key-syntax edges and the batch-path
  exclusion lack or weaken criteria
- 🔵 **Scope**: The skill could split into its own story; the Summary
  hides the any-key capability

### Assessment

Two majors trip the REVISE threshold, but both are narrow: one is a
terminology split with a default exit status from the existing create
contract, and the other is a missing criterion. Most remaining findings
are structural and can be applied directly. Four need design decisions:
key case identity, the skill's proposal and fallback rules, how the skill
receives the ID, and who owns `--id` on the skill's push-gate dry-run.

## Re-Review (Pass 4) — 2026-10-29T10:32:57+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity**: "Code" ambiguity — Resolved; "exits with the X
  refusal" defined as exit status 1 plus the `E_` identifier
- 🟡 **Testability**: Letter case given untested — Resolved
- 🔵 **Clarity + Testability**: Skill proposal rule — Resolved, then
  narrowed further in this pass
- 🔵 **Clarity**: Inconsistent fallbacks — Partially resolved; refined in
  this pass per refusal case
- 🔵 **Clarity + Testability**: Key case identity — Resolved
- 🔵 **Completeness + Dependency**: Skill input and push-gate ownership —
  Resolved
- 🔵 **Dependency**: 0351 mirror and 0343 attribution — Resolved
- 🔵 **Clarity**: Refusal causes, outcome keyword, contradictory trigger,
  range — Resolved
- 🔵 **Testability**: Push, dry-run, lock, `NNNN`, `--project`, key syntax,
  batch exclusion — Resolved
- 🔵 **Scope**: Skill split; any-key in Summary — Split declined by
  decision; Summary resolved

### New Issues Introduced

- 🔵 **Dependency**: Other per-key allocators (`work next-number`) not
  named for the case-insensitive scan; the re-key guard's position
  unstated; no External line for the tracker
- 🔵 **Dependency**: 0302's status and its ID comparisons under the case
  rule unrecorded
- 🔵 **Clarity**: "Either" in the fallback rule has two antecedents;
  `{tracker}` offered a dead-end "name another ID"; correctable versus
  not-an-ID defined only by example; "exact" read as constraining the key;
  "taken" matching rule (string or key-and-number) undefined; "that
  class" unclear; 0351's change compressed
- 🔵 **Testability**: Skill draft-ID, non-ID and keyless-pattern requests,
  recovery after the user's choice, observable argument parsing,
  draft-form near-misses, message text, "unsynced" and the concrete push
  success lack criteria
- 🔵 **Completeness**: `extract-work-items` exclusion untested
- 🔵 **Scope**: No reason recorded for 0319 landing first; the
  `/sync-work-items` criterion lacks a scope statement

### Assessment

No major findings remain, so the item is acceptable for planning. All
pass-4 findings have been applied, three of them after design decisions:
a keyed request under a keyless pattern has no exact form, the skill may
only pad a number and add `work.key`, and "taken" compares key and
number for pattern IDs. The remaining scope signal is size, which was
accepted by decision.
