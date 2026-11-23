---
type: "work-item-review"
id: "PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite-review-1"
title: "Work Item Review: Shorten the slowest tests in the CLI unit suite"
date: "2026-11-22T23:27:07+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:PP-943"
work_item_id: "PP-943"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 5
tags: ["performance", "testing"]
last_updated: "2026-11-23T01:48:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Shorten the slowest tests in the CLI unit suite

**Verdict:** REVISE

PP-943 is well grounded: every figure traces to a named record and commit,
the measurement commands are exact, and coverage preservation is stated in
both Requirements and Acceptance Criteria. It needs revision because the
criteria cannot settle three things it requires: that the
own-work-versus-contention investigation happened, that coverage survived a
restructure, and how a split or renamed test is timed. The Summary also
states the tail-test count inconsistently.

### Cross-Cutting Themes

- **Requirement 1 has no deliverable or criterion** (flagged by:
  testability, completeness, scope) — the contention investigation can be
  skipped and the item still passes; scope adds that fixed halving targets
  are committed before that investigation could show they are unreachable.
- **Two identifier schemes for related items** (flagged by: clarity,
  completeness, dependency) — frontmatter uses PP-953 / PP-957, the body
  uses 0361 / 0365, and nothing maps one to the other.
- **Tail-test count: two or three** (flagged by: clarity, scope) — Summary
  and Drafting Notes say two; Context, Requirement 5 and the criteria say
  three.
- **Record comparability** (flagged by: dependency, testability) — only the
  phase-median criterion requires `compare-builds` to show no differing
  condition; the per-test criteria and the reliance on one host are
  implicit.

### Findings

#### Major

- 🟡 **Testability / Completeness / Scope**: Requirement 1 (own work vs
  contention) has no acceptance criterion or named output
  **Location**: Requirements
  No criterion names where the alone and under-suite timings are recorded,
  so the diagnosis the Context calls the warning case could be skipped. The
  halving targets are also fixed before that diagnosis exists, with no
  re-scope path if contention dominates.
- 🟡 **Testability**: Coverage-preservation criterion gives no verification
  procedure
  **Location**: Acceptance Criteria
  AC3 lists no baseline (scenarios × colocations, halt-point counts per
  VCS) and no check, so a restructure that renames tests leaves it settled
  only by reading the code.
- 🟡 **Testability**: Per-test timing criteria do not say how to measure a
  split or renamed test
  **Location**: Acceptance Criteria
  Splitting each colocation into its own test (Requirement 4's first
  candidate) leaves no single successor; max-of-successors and sum give
  opposite verdicts from the same logs.
- 🟡 **Clarity / Scope**: Number of tail tests stated as two and as three
  **Location**: Summary
  "Two tests in `matrix_private_build` and `rekey_run` finish last" and
  "the two tail tests were brought into scope" contradict the three tests in
  the Context table, Requirement 5 and AC2.

#### Minor

- 🔵 **Clarity / Completeness / Dependency**: Parent and related items use
  two identifier schemes
  **Location**: Dependencies
  PP-953 / PP-957 in frontmatter, 0361 / 0365 in the body; PP-957 never
  appears in the body.
- 🔵 **Testability / Dependency**: Comparability precondition covers only
  AC4, and the single measurement host is implicit
  **Location**: Acceptance Criteria
  AC1 and AC2 could be checked against records taken under different
  conditions; both records must come from the same host and host settings.
- 🔵 **Testability**: Zero-tolerance phase-median guard is sensitive to
  noise
  **Location**: Acceptance Criteria
  The reference runs spread ~126–144 s, so "no longer than before" can fail
  on noise.
- 🔵 **Clarity**: "The slowest `guarded_resume` tests" leaves the target
  set open
  **Location**: Requirements
  AC1 targets the whole binary's test-seconds, while Requirement 2 reads as
  a subset.
- 🔵 **Clarity**: "Position" means start order in one sentence and finish
  order in the next
  **Location**: Context
  The load-versus-tail argument depends on the distinction.
- 🔵 **Completeness**: Open Questions is empty despite a named unknown
  **Location**: Open Questions
  The own-work versus contention split, and whether the targets stay
  reachable, are open.
- 🔵 **Scope**: Three independent efforts across three crates
  **Location**: Requirements
  Each has its own cause, fix and criterion; one stalling holds the others
  open.

#### Suggestions

- 🔵 **Clarity**: Measurement jargon undefined
  **Location**: Acceptance Criteria
  "Test-seconds", "contention", "differing condition", "warm-no-op",
  "isolated".
- 🔵 **Clarity**: Colocation pairing assumes every `guarded_resume` test is
  a jj test
  **Location**: Requirements
  Non-jj scenarios, if any, fall outside the preservation wording.
- 🔵 **Completeness**: Context quantifies the cost but not who benefits
  **Location**: Context
- 🔵 **Scope**: Kind `task` may understate a multi-crate,
  research-then-implement effort
  **Location**: Frontmatter: kind

### Strengths

- ✅ Every figure is tied to a named record and commit; the 143 s
  phase median matches the reference record (142.971 s).
- ✅ AC1 and AC2 give the exact `measure:build` invocation, the data source
  (`PASS [ Ns ]` lines), the aggregation (median of three runs) and numeric
  thresholds.
- ✅ Assumptions say before and after figures come from this item's own
  commits, so drift since the reference record cannot skew the comparison.
- ✅ Coverage rules appear in both Requirements and Acceptance Criteria, so
  "faster" cannot become "fewer scenarios".
- ✅ Approach choice is left to research and planning, with candidates
  listed but not mandated.
- ✅ The tail-test table names each test fully with per-run durations and
  cost.
- ✅ Drafting Notes record where the targets came from and why scope grew;
  the phase criterion is explicitly a regression guard.

### Recommended Changes

1. **Give Requirement 1 an output and a criterion, plus a decision point**
   (addresses: Requirement 1 has no acceptance criterion; Open Questions
   empty; investigation precedes fixed targets)
   Name where alone and under-suite timings for each binary are recorded,
   add a matching criterion, and state what happens to a target if
   contention dominates. Move the unknown into Open Questions.
2. **Add a coverage baseline and a check to AC3** (addresses: coverage
   criterion gives no verification procedure)
   List the `guarded_resume` scenarios × {`--no-colocate`, `--colocate`},
   the halt-point count per VCS, and the private-versus-shared equality
   assertion; state how a verifier maps old tests to successors.
3. **State the rule for split or renamed tests in AC1 and AC2** (addresses:
   per-test timing criteria for split tests)
   For example: the after duration is the longest successor, and the sum of
   successors may not exceed the before duration.
4. **Fix the tail-test count** (addresses: two versus three tail tests)
   Say three tail tests in two binaries in the Summary and Drafting Notes.
5. **Make record comparability a precondition of every timing criterion**
   (addresses: comparability precondition; single host; noise tolerance)
   Open AC1 with the `compare-builds` no-differing-condition check, refer
   back to it from AC2 and AC4, and consider a tolerance on AC4.
6. **Map identifiers** (addresses: two identifier schemes)
   Write "Parent: PP-953 (0361)" and "Related: PP-957 (0365)" in
   Dependencies.
7. **Tighten wording** (addresses: slowest `guarded_resume` tests; position;
   jargon)
   Say the target is the whole binary, define position as start or
   completion order, and define test-seconds on first use.

## Per-Lens Results

### Clarity

**Summary**: Mostly precise, with concrete named figures and exact commands.
The tail-test count is inconsistent, two identifier schemes are unmapped,
and some jargon ("test-seconds", "differing condition", "position") is
undefined.

**Strengths**:
- Every measurement claim is tied to a named record and commit.
- The tail-test table names each test fully with durations and cost.
- Criteria name exact commands; Technical Notes name the log lines.
- Assumptions state the before and after figures come from the item's own
  commits.

**Findings**:
- 🟡 major / high — Summary — Number of tail tests stated as two and three.
  Summary and Drafting Notes say two; Context, table, Requirement 5 and the
  criteria say three. Use one count, or say "two binaries" if that was
  meant.
- 🔵 minor / medium — Dependencies — Related items referenced by two
  identifier schemes. Give the PP- ID beside each legacy number.
- 🔵 minor / medium — Requirements — "The slowest `guarded_resume` tests"
  does not say which tests. AC1 implies the whole binary; align
  Requirement 2.
- 🔵 minor / medium — Context — "Position" means start order and finish
  order. Define it once and state the load-versus-tail distinction directly.
- 🔵 suggestion / medium — Acceptance Criteria — Measurement jargon used
  without definition.
- 🔵 suggestion / low — Requirements — Colocation pairing assumes every
  `guarded_resume` test is a jj test.

### Completeness

**Summary**: Well populated, with complete frontmatter and substantive
sections. Requirement 1 has no deliverable, Open Questions is empty despite
a named unknown, and the Context says little about why the reduction
matters.

**Strengths**:
- Frontmatter complete with a recognised kind.
- Context figures traced to named records; 143 s matches 142.971 s.
- Requirements state what to change and what to preserve.
- Criteria and Technical Notes let an implementer start without questions.
- Dependencies and Assumptions explicit.
- Drafting Notes explain targets and scope changes.

**Findings**:
- 🔵 minor / medium — Requirements — Contention investigation has no
  deliverable or criterion.
- 🔵 minor / medium — Open Questions — Empty although the Context names an
  unknown.
- 🔵 suggestion / low — Context — Cost quantified, motivation thin.
- 🔵 suggestion / low — Frontmatter: parent — Parent and related items
  differ between frontmatter and body.

### Dependency

**Summary**: Mostly self-contained, with no upstream blocker and no
downstream consumer. Gaps: the parent and related links are not reconciled
across identifier schemes, and the criteria depend on one measurement host
without saying so.

**Strengths**:
- Dependencies explain why there is no blocker.
- Assumptions keep the criteria valid if the suite drifts.
- Measurement tooling is named with exact invocations.

**Findings**:
- 🔵 minor / medium — Dependencies — Parent and related links not
  reconciled. Write "Parent: PP-953 (0361)", "Related: PP-957 (0365)".
- 🔵 minor / medium — Acceptance Criteria — Comparable records depend on one
  measurement host (engywook, devtools settings). Record that both records
  come from the same host and settings.

### Scope

**Summary**: Coherent under one theme but bundles three independent efforts
across `migrate-cli`, `vcs-test-support` and `work-adapters`, and puts an
open-ended investigation before fixed targets. Large for a task; keeping it
whole is defensible.

**Strengths**:
- Bounded by named binaries and tests.
- Coverage rules in both Requirements and Criteria.
- Approach left to research and planning.
- Drafting Notes trace scope growth.
- Phase criterion framed as a regression guard.

**Findings**:
- 🔵 minor / medium — Requirements — Three independent efforts bundled
  across three crates. Consider a sibling task under PP-953, or note that
  each piece closes alone.
- 🔵 minor / medium — Requirements — Investigation precedes fixed delivery
  targets. Add a decision point or an assumption that targets are revisited.
- 🔵 suggestion / medium — Summary — Summary scopes two tail tests, the rest
  three.
- 🔵 suggestion / low — Frontmatter: kind — `task` may understate the
  effort.

### Testability

**Summary**: Mostly testable: timing criteria name the command, data
source, aggregation and thresholds against records at the item's own
commits. Gaps: Requirement 1 has no criterion, AC3 has no verification
procedure, and per-test criteria do not handle split or renamed tests.

**Strengths**:
- AC1 and AC2 give command, `--keep`, data source, median-of-three and
  numeric thresholds.
- Before and after records come from the item's own commits.
- AC4 checks comparability via `compare-builds`.
- Thresholds are labelled agreed targets, not projections.

**Findings**:
- 🟡 major / high — Requirements — Requirement 1 has no acceptance
  criterion. Add one recording alone and under-suite durations per binary at
  both commits.
- 🟡 major / high — Acceptance Criteria — Coverage-preservation criterion
  gives no verification procedure. List the baseline and state the check.
- 🟡 major / medium — Acceptance Criteria — Per-test timing criteria do not
  say how to measure a split or renamed test. State a max/sum rule or
  require names to be kept.
- 🔵 minor / medium — Acceptance Criteria — Zero-tolerance phase-median
  guard is sensitive to noise (runs spread ~126–144 s). Add a tolerance.
- 🔵 minor / medium — Acceptance Criteria — Comparability check applies only
  to AC4. Make it a precondition of all timing criteria.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-11-23T00:16:29+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability / Completeness / Scope**: Requirement 1 has no criterion
  — Resolved; a new criterion records alone and under-suite durations, and
  the unknown sits in Open Questions.
- 🟡 **Testability**: Coverage criterion gives no verification procedure —
  Partially resolved; the baseline and test-to-successor mapping are in, but
  assertion preservation still rests on review (testability, minor).
- 🟡 **Testability**: Timing a split or renamed test — Resolved; "summed
  median" admits two aggregations (clarity, minor).
- 🟡 **Clarity / Scope**: Two versus three tail tests — Resolved.
- 🔵 **Clarity / Completeness / Dependency**: Two identifier schemes —
  Partially resolved; bare 0365 / 0361 remain after first mention.
- 🔵 **Testability / Dependency**: Comparability precondition and single
  host — Resolved.
- 🔵 **Testability**: Zero-tolerance phase guard — Resolved; the noise
  clause's wording and the narrow band above the median remain (clarity
  minor, testability suggestion).
- 🔵 **Clarity**: "The slowest `guarded_resume` tests" — Resolved.
- 🔵 **Clarity**: "Position" — Resolved.
- 🔵 **Completeness**: Empty Open Questions — Resolved.
- 🔵 **Scope**: Three efforts across three crates — Still present, and now
  major: see the first new issue.
- 🔵 **Clarity**: Undefined jargon — Resolved.
- 🔵 **Clarity**: Tests without colocations — Resolved.
- 🔵 **Completeness**: Missing motivation — Resolved.
- 🔵 **Scope**: Kind `task` — Still present (suggestion).

### New Issues Introduced

- 🟡 **Scope / Dependency / Testability / Clarity**: One before/after
  commit pair against independent per-binary landings — Requirement 6 lets
  each binary land alone, but every timing and coverage criterion reads "the
  commit before this change and one after it"; no criterion covers
  Requirement 6, and an early landing shifts contention for later ones.
- 🟡 **Testability / Completeness / Clarity**: Revised halving targets have
  no recorded outcome — the Assumption lets a target be reviewed, but the
  criteria stay unconditional and name nowhere to record a revision; the
  trigger is per test while Requirement 1 times per binary.
- 🔵 **Testability**: "Slowest test takes under 20 s" names no statistic.
- 🔵 **Testability**: Alone timing gives no run count or statistic.
- 🔵 **Clarity**: Splitting colocations shortens the slowest test but not
  total test-seconds, which Requirement 2 and AC1 target.
- 🔵 **Clarity**: "Scenario", "test" and "binary" lack fixed meanings once
  tests are split.
- 🔵 **Clarity**: AC1 and AC4 run `compare-builds` with and without
  `--task`.
- 🔵 **Dependency**: Concurrent PP-953 work affecting contention, the
  measurement tooling, and other consumers of the shared helpers are not
  named.
- 🔵 **Scope**: Load reduction and tail shortening are two goals under one
  title; Requirement 1 is spike-like work inside a delivery task.
- 🔵 **Completeness**: The run logs behind the per-test figures are absent
  from References.

### Assessment

Fourteen of fifteen first-pass findings are resolved or partially resolved.
The item still needs one revision: Requirement 6, added in this pass,
conflicts with criteria anchored on one before/after pair, and the
target-review Assumption has no recorded outcome the criteria can check.
Both are decisions for the author; the remaining minors are wording.

## Re-Review (Pass 3) — 2026-11-23T00:33:56+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Scope / Dependency / Testability / Clarity**: One before/after pair
  against independent landings — Resolved; one pair across all three, with
  a criterion that each change passes alone. Scope keeps a minor: closure
  waits on the slowest of the three.
- 🟡 **Testability / Completeness / Clarity**: Revised targets with no
  recorded outcome — Resolved; a revision is written into AC1 or AC2.
  Testability adds a minor: the revision figures may come from a different
  commit than the before record.
- 🔵 **Testability**: Slowest-test statistic — Partially resolved; "median
  of the slowest test" still admits two aggregations.
- 🔵 **Testability**: Alone-timing run count — Resolved.
- 🔵 **Clarity**: Splitting colocations versus test-seconds — Resolved.
- 🔵 **Clarity**: Scenario, test and binary — Resolved.
- 🔵 **Clarity**: `compare-builds` invocations — Resolved.
- 🔵 **Dependency**: Concurrent work, tooling, helper consumers —
  Partially resolved; the concurrent-work entry is wrong about PP-958 (see
  below).
- 🔵 **Completeness**: Run logs absent from References — Resolved;
  testability now suggests recording the computed figures, since the logs
  stay on one host.

### New Issues Introduced

- 🟡 **Dependency**: No re-baselining rule for runtime-affecting changes —
  PP-958 trials `debug = "line-tables-only"` for dependencies in the dev
  profile, which changes the test binaries, so "a warm no-op record excludes
  it" is wrong. PP-957 had a rule: take a fresh before-record at current
  `main` with the item's changes reverted.
- 🟡 **Clarity**: "Target test" is undefined — Requirement 1, the
  revision Assumption and the alone-timing criterion all depend on it, and
  it is unclear whether the 20 s ceiling is subject to revision.
- 🟡 **Testability**: Fidelity of a reused or copied starting state is
  unverified — candidate approaches change each test's precondition; no
  criterion checks the copied state equals one built from scratch.
- 🔵 **Dependency**: Sharing `engywook` with PP-962 — records must not
  overlap its sessions, and `devtools`-only settings must be restored.
- 🔵 **Scope / Dependency / Clarity**: PP-953's decomposition still lists
  0391 under its pre-enrichment title.
- 🔵 **Testability**: Merged tests have no duration-attribution rule.
- 🔵 **Clarity**: "The plan's mapping below" points at the coverage list;
  Requirement 6 leaves the measurement point implicit; the conditional
  blocker is hard to parse; "slowest tests" versus "slowest test".
- 🔵 **Completeness**: The tail tests get thinner requirements than
  `guarded_resume`; Requirement 4's candidates apply only to it.
- 🔵 **Scope**: Whether reworking `shared_build` is in scope is unstated.

### Assessment

Both pass-2 majors are resolved. Three new majors surfaced, each narrower
than the last round's: one is a factual error introduced in pass 2 (the
PP-958 claim), one a missing definition, and one a genuine gap in what
"without losing confidence" requires of reused fixtures. Findings are now
converging on edge cases; one more targeted round should leave the item
ready for planning.

## Re-Review (Pass 4) — 2026-11-23T00:59:03+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Dependency**: Rebaselining rule — Resolved; the rule exists, but
  it now gives "before record" two meanings (see first new issue).
- 🟡 **Clarity**: "Target test" undefined — Resolved.
- 🟡 **Testability**: Reused-state fidelity — Partially resolved; the
  criterion exists but "equals" is undefined for repository state.
- 🔵 **Dependency**: Sharing `engywook` with PP-962 — Resolved.
- 🔵 **Scope / Dependency / Clarity**: Parent decomposition title —
  Resolved.
- 🔵 **Testability**: Merged tests — Resolved; clarity asks how merge and
  split apply to `guarded_resume`'s binary-level targets.
- 🔵 **Testability**: Revision figures' commit — Partially resolved; folded
  into the before-record ambiguity.
- 🔵 **Clarity**: Wording items — Resolved.
- 🔵 **Completeness**: Requirement 4 for the tail tests — Partially
  resolved; it applies to all three, but lists candidates only for
  `guarded_resume` (suggestion).
- 🔵 **Scope**: `shared_build` in scope — Resolved; scope suggests
  bounding it to what the chosen approach needs.

### New Issues Introduced

- 🟡 **Clarity**: "Before record" has two definitions and timings — the
  parent of the first change, or a fresh record re-taken when a
  runtime-affecting change lands; Requirement 1 needs its figures before
  planning, which a late rebaseline cannot feed.
- 🟡 **Testability**: State equality undefined — a reused jj/Git state
  cannot match change IDs, commit IDs, operation log or timestamps; the
  compared projection must be named.
- 🟡 **Testability**: Cost moved outside per-test durations escapes the
  targets — building states once in a setup script or `shared_build`
  removes it from `PASS` durations; the phase guard alone allows it.
- 🔵 **Testability**: Strict summed-successor comparison sits near noise;
  binary total ambiguous when equality tests are added or tests move
  binaries; alone-run log source and retries unspecified; "same
  assertions" rests on review.
- 🔵 **Dependency**: `relates_to` lists only PP-957; consumers of the
  shared helpers and `shared_build` unnamed.
- 🔵 **Clarity**: Terms used before their definitions; aliases and jargon
  unexplained; "slowest" versus "finishing last"; criteria referenced by
  position.
- 🔵 **Scope**: Bundling, embedded spike and kind — restated; settled by
  the author.

### Assessment

Two of the three new majors trace to pass-3 edits (the rebaseline rule and
the equality criterion), and one is a genuine gap: off-test setup cost.
Each pass's additions now generate the next pass's findings, so further
passes have diminishing returns. Resolving these three and stopping is
reasonable.

## Re-Review (Pass 5) — 2026-11-23T01:35:44+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity**: Two meanings of "before record" — Resolved; baseline,
  rebaseline, before and after records are defined once. Clarity adds a
  minor: who decides a change "affects runtime".
- 🟡 **Testability**: State equality undefined — Resolved; a minimum
  compared projection and excluded fields are named.
- 🟡 **Testability**: Setup cost escaping per-test durations — Partially
  resolved; setup with its own log entry is counted, but setup moved to a
  dependency task or a cross-run cache leaves the measured window (major,
  below).
- 🔵 **Testability**: Summed-successor noise, binary total, alone-run
  source and retries, assertion mapping — Resolved.
- 🔵 **Dependency**: `relates_to` and helper consumers — Resolved.
- 🔵 **Clarity**: Terms before definitions, aliases, selection reasons,
  positional references — Resolved.
- 🔵 **Scope**: Bundling, embedded spike, kind — Still present; settled by
  the author, recorded in Drafting Notes.

### New Issues Introduced

- 🟡 **Testability**: Setup cost can leave the measured window — an
  isolated warm no-op record runs `test:unit:cli` without its
  dependencies, so a state built by a dependency task or cached across runs
  costs nothing; require reused state to be built from scratch inside the
  measured task.
- 🔵 **Dependency**: The setup-script example conflicts with PP-957, which
  rejected nextest setup scripts; reliance on PP-957's build step, copy
  helper, fixture log and `Summary [` constraint is unstated.
- 🔵 **Dependency**: jj, git and nextest behaviour relied on is unrecorded;
  the PP-958 coupling is one-way; "Blocks: none" hides gating PP-953's
  closure.
- 🔵 **Testability**: Tail tests' state-equality cost is uncounted;
  independent landing is met trivially by a joint landing; loop coverage
  rests on review.
- 🔵 **Clarity**: Aggregation order across runs for totals and split tests;
  alone-run points differ between Requirement 1 and the criterion;
  `accelerator-migrate` versus `migrate-cli`.
- 🔵 **Scope**: The Summary claims a shorter phase that no criterion
  requires.

### Assessment

One major remains, below the REVISE threshold of two, so the verdict moves
to COMMENT: the item is acceptable for planning. The major tightens the
pass-4 setup-cost rule and pairs with the PP-957 build-step minor; fixing
both is one short edit.

## Verdict Change — 2026-11-23T01:48:20+00:00

**Verdict:** APPROVE

The author approved the work item after the pass-5 major (setup cost
escaping the measured window) and its minors were addressed. Those final
edits were not re-reviewed.
