---
type: "work-item-review"
id: "0363-remove-repeated-work-from-the-rust-test-lanes-review-1"
title: "Work Item Review: Remove repeated work from the Rust test lanes"
date: "2026-11-13T01:39:20+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0363"
work_item_id: "0363"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-11-13T13:38:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Remove repeated work from the Rust test lanes

**Verdict:** REVISE

The work item is well grounded. Its Context backs both changes with
measured evidence, its Requirements name exact commands and flags, and its
scope boundaries (A4 already shipped, local de-instrumentation superseded,
manual full clean only) are explicit. The weakness is the definition of
done. The Summary promises that no test and no coverage figure is dropped,
but no acceptance criterion checks either promise, and the rotation
criterion could be met by weaker tests. Several smaller clarity problems
come from research labels and terms that are never explained.

### Cross-Cutting Themes

- **The "no dropped test or coverage figure" promise is never verified**
  (flagged by: testability, completeness). The central correctness risk
  that Context spends most effort on (stale profiles inflating coverage
  from 77.99% to 89.94%) is checked only indirectly, through which profraw
  files exist. Nothing compares totals against a full clean, and nothing
  checks the test count.
- **Requirements and Summary claims have no matching criterion**
  (flagged by: completeness, testability, clarity). Requirement 4 (the
  docstring), CI instrumentation staying on, and the macOS CI speed-up
  have no criteria. The Summary's "longest local task" is also not named.
- **The shape of the coverage totals output is underspecified**
  (flagged by: testability, clarity). The output is called a "totals line",
  a "totals summary" and the "`totals` object", and its content is never
  stated.
- **The CI cache assumption is opaque** (flagged by: dependency, clarity).
  "Rebuilt from cache" does not say whether the cache can hold profraw
  files, and the item does not reference the CI workflow or cache step.

### Findings

#### Major

- 🟡 **Testability / Completeness**: No criterion verifies that coverage
  figures match a full clean
  **Location**: Acceptance Criteria
  The Summary promises that no coverage figure is dropped, and Context
  shows how bare `--no-clean` silently inflated figures. Yet AC2 checks
  only which profraw files exist, and nothing compares the totals with a
  run made straight after a full clean.
- 🟡 **Testability**: The "no test dropped" claim has no verifiable
  criterion
  **Location**: Acceptance Criteria
  Deleting a rotation test, or marking it `#[ignore]`, would easily meet
  AC4's CPU target. No criterion checks the nextest test count, or that
  the three named `log::tests` tests still run and pass.
- 🟡 **Testability**: "Still asserts rotation" can be met by weaker tests
  **Location**: Acceptance Criteria
  AC4's rotation clause can be checked only by reading the code, and a
  test asserting "some rotation happened" would count. It should be
  testable by mutation (changing `MAX_BYTES`/`MAX_FILES` makes a test
  fail), or it should list the concrete assertions.

#### Minor

- 🔵 **Completeness / Testability**: Requirement 4 (docstring remedy) has
  no acceptance criterion
  **Location**: Acceptance Criteria
  The docstring is the only recorded mitigation for the remaining
  stale-binary risk, but the item could close without it.
- 🔵 **Testability / Completeness**: The CI instrumentation and macOS CI
  speed-up claims have no criteria
  **Location**: Summary, Requirements
  Requirement 1 says instrumentation stays on in CI, and the Summary
  promises a shorter macOS unit job. Neither is verified. Either add
  criteria or reword the CI speed-up as an expected side effect.
- 🔵 **Testability / Clarity**: The content of the totals summary is not
  specified
  **Location**: Requirements, Acceptance Criteria, Technical Notes
  It is called a "totals line", a "totals summary" and the "`totals`
  object". State which metrics it shows (line/function/region) and its
  shape.
- 🔵 **Testability**: The timing and CPU criteria lack a measurement
  procedure and host
  **Location**: Acceptance Criteria
  AC1 and AC4 do not define the host, what "warm" means, the run count,
  or the CPU-s measurement command. AC1 leaves only 20 s of headroom over
  the projected 160 s.
- 🔵 **Testability**: AC2 does not say how "only those the second run
  wrote" is checked
  **Location**: Acceptance Criteria
  Give a procedure, for example that every profraw file's mtime is later
  than the second run's start, and that the file count is close to a
  single run's.
- 🔵 **Clarity**: "Kernel" means both a crate and the OS kernel
  **Location**: Context
  "Kernel line coverage" refers to a crate, while "kernel time" means the
  OS. Name the crate by its package name.
- 🔵 **Clarity**: Research labels and terms are used without definition
  **Location**: Context
  A2/A3/A4, "the quick-wins experiment", "pass" and "its `dev-frontend`
  pass" are not explained for a reader who has not read the research.
- 🔵 **Clarity**: "Third finding" clashes with the A-numbered labels
  **Location**: Context
  It reads as though it might be A3. Call it A4, as the Drafting Notes do.
- 🔵 **Clarity**: "The longest local task" is not named
  **Location**: Summary
  Name the task(s), and say which change shortens which task or job.
- 🔵 **Clarity**: The "Blocked by: none" entry raises a dependency and
  then dismisses it
  **Location**: Dependencies
  Say plainly that the item waited on 0368's baseline so that timings stay
  comparable, and that the condition is now met.
- 🔵 **Scope**: Two independent changes in different components are
  bundled
  **Location**: Summary
  Keeping them together is defensible, but note that A2 and A3 may merge
  separately, so a problem with one does not hold back the other or 0370.
- 🔵 **Scope**: The coverage-summary output fix falls outside the stated
  purpose
  **Location**: Requirements
  Requirement 2 is not repeated work, and the Summary does not mention it.
- 🔵 **Scope**: The parent epic's child list still assigns A4 to this item
  **Location**: Drafting Notes
  0361 still lists 0363 as covering (A2, A3, A4).
- 🔵 **Dependency**: Reliance on specific cargo-llvm-cov behaviour is not
  captured
  **Location**: Requirements
  Record the version the flags and JSON shape were verified against, and
  how the tool is pinned locally and in CI.
- 🔵 **Dependency**: The equivalence of chunked writes depends on
  file-rotate's byte-based splitting
  **Location**: Context
  Record the crate and its mid-write split behaviour as an assumption, so
  that an upgrade flags these tests for re-checking.

#### Suggestions

- 🔵 **Dependency / Clarity**: The CI cache assumption is opaque and not
  referenced
  **Location**: Assumptions
  Name the CI workflow and cache step, and say that the profraw-only clean
  (not the cache) is what keeps the figures correct.
- 🔵 **Clarity**: "Runs alone" means different things in AC1 and AC4
  **Location**: Acceptance Criteria
  For AC4, say that the module is filtered on its own on an idle host.
- 🔵 **Clarity**: Unclear whether the docstring must also say no periodic
  clean is scheduled, and what happens to `ACCELERATOR_COVERAGE`
  **Location**: Requirements
  State both explicitly.

### Strengths

- ✅ Context backs both changes with measured evidence (61 s clean, 247 s
  vs 160 s, 13 M syscalls, 89.94% vs 77.99%) and explains why bare
  `--no-clean` is unsafe.
- ✅ Requirements name exact commands and flags and explicitly ban bare
  `--no-clean`, so the change has only one reading.
- ✅ Performance criteria give both a target and today's baseline, so
  pass/fail is unambiguous.
- ✅ AC2 turns the stale-profile hazard into something you can observe,
  rather than an implementation instruction.
- ✅ Scope is explicit: A4 and the third research finding are marked as
  shipped, local de-instrumentation as superseded, and full-clean cadence
  as manual only.
- ✅ The research's open question about whether rotation is byte-based is
  answered, so chunked writes rest on a stated basis.
- ✅ Dependencies are justified rather than asserted: 0370's gating on
  0362/0367 is visible, and the satisfied 0368 baseline is traced to its
  file.
- ✅ Drafting Notes separate proposed targets from measured values.

### Recommended Changes

1. **Add a coverage-equivalence criterion** (addresses: no criterion
   verifies that coverage figures match a full clean)
   "Given a prior coverage run, when `test:unit:cli` runs again, then its
   line/function/region totals equal those from a run made straight after
   `cargo llvm-cov clean --workspace`."
2. **Add a test-count criterion** (addresses: "no test dropped" claim)
   The nextest count for `test:unit:cli` is unchanged, and
   `retains_at_most_three_rotated_segments` and its two siblings still run
   and pass.
3. **Make AC4's rotation clause concrete** (addresses: "still asserts
   rotation" can be met by weaker tests)
   List the assertions (each rotated segment ≤ `MAX_BYTES`, at most
   `MAX_FILES` kept, a single write across the cap is split), or require
   that mutating `MAX_BYTES`/`MAX_FILES` makes a test fail.
4. **Cover the remaining requirements and claims** (addresses: Requirement
   4 has no criterion; CI claims have no criteria)
   Add a docstring criterion and a criterion that CI still instruments,
   and reword the macOS CI speed-up as an expected side effect unless it
   will be measured.
5. **Pin down the totals output** (addresses: totals summary content)
   Use one term, and state that the output is a single totals line with
   line, function and region percentages and no per-file rows.
6. **Specify the measurement procedure** (addresses: timing/CPU procedure;
   AC2 verification; "runs alone")
   Name the host, define "warm", take the median of three runs, give the
   CPU-s command, and give the mtime/count check for AC2.
7. **Tidy the research vocabulary** (addresses: kernel ambiguity, research
   labels, "third finding", "longest local task", Blocked-by wording)
   Gloss A2/A3/A4 and the quick-wins experiment, name the kernel crate,
   call the fixed item A4, and name the task(s) the Summary means.
8. **Record the external behaviour assumptions** (addresses:
   cargo-llvm-cov reliance, file-rotate splitting, CI cache)
   Add entries for the cargo-llvm-cov version and pinning, file-rotate's
   byte-split behaviour, and the CI workflow/cache step.
9. **Scope housekeeping** (addresses: bundled changes, summary fix outside
   stated purpose, parent child list)
   Note that A2 and A3 may merge separately, mention the summary-output fix
   in the Summary, and update 0361's child list to (A2, A3).

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is mostly clear. It names concrete commands,
flags, file paths and measured figures, and its Summary, Requirements and
Acceptance Criteria agree on scope. The weak spots are borrowed labels and
terms from the source research that are never defined (A2/A3/A4,
"quick-wins experiment", "pass"), one confusing clash where "kernel" means
two different things, and some drift in what the coverage output is called
and in which task the Summary means.

**Strengths**:
- Requirements give exact commands and flags and explicitly ban bare
  `--no-clean`.
- Context explains why bare `--no-clean` is unsafe with a concrete
  before/after figure.
- Summary, Requirements and Acceptance Criteria all cover the same two
  concerns, and the third research finding is ruled out.
- Drafting Notes say where the 180 s and 2 CPU-s targets come from.

**Findings**:
- 🔵 minor (high) — Context — "Kernel" means both a crate and the OS
  kernel. "Kernel line coverage" seems to refer to a workspace crate, while
  "kernel time" and "through the kernel" mean the OS. Name the crate by its
  package name.
- 🔵 minor (high) — Context — Research labels and terms used without
  definition: A2/A3/A4, "the quick-wins experiment", and "pass" / "its
  `dev-frontend` pass". Gloss each one, and say that the pass belongs to
  `test:unit:visualiser`.
- 🔵 minor (high) — Context — "Third finding" clashes with the A-numbered
  labels and with "a third time". Call it A4.
- 🔵 minor (medium) — Summary — "The longest local task" is not named.
  Context gives two candidates. Say which change shortens which task.
- 🔵 minor (medium) — Requirements — The coverage output is called a
  "totals line", a "totals summary" and the "`totals` object". Use one term
  and state the output shape.
- 🔵 minor (medium) — Dependencies — "Blocked by: none" raises a dependency
  on 0368 and 0361's 200 s target, then dismisses it. State it directly or
  remove it.
- 🔵 suggestion (medium) — Assumptions — "Rebuilt from cache" is unclear.
  Say whether the cache can hold profraw files, and that the profraw-only
  clean is what keeps the figures correct.
- 🔵 suggestion (low) — Acceptance Criteria — "Runs alone" means different
  things in AC1 and AC4. Define it for AC4.
- 🔵 suggestion (low) — Requirements — Unclear whether the docstring must
  say no periodic clean is scheduled, and whether `ACCELERATOR_COVERAGE`
  changes.

### Completeness

**Summary**: The task is well populated. The frontmatter is complete, the
Context explains the forces behind both changes with measured figures, the
Requirements are concrete, and Dependencies, Assumptions, Technical Notes
and Drafting Notes all have real content. The gaps are small: one
requirement has no acceptance criterion, and two outcomes the Summary
promises have no criterion defining when they are met.

**Strengths**:
- Frontmatter is complete and consistent with the body header.
- Context gives the reasons for both changes with measured evidence.
- The open question about byte-based rotation is answered.
- Scope is spelled out, including what has already shipped.
- Dependencies, Assumptions and Technical Notes have real content.
- Each requirement is specific about commands and flags.

**Findings**:
- 🔵 minor (high) — Acceptance Criteria — Requirement 4 (docstring remedy)
  has no acceptance criterion. Add one checking that the help text names
  `cargo llvm-cov clean --workspace`.
- 🔵 minor (medium) — Acceptance Criteria — No criterion covers the
  Summary's promise of unchanged coverage figures. Add a totals-equivalence
  criterion.
- 🔵 suggestion (medium) — Summary — The promised macOS CI improvement has
  no matching criterion. Add one, or reword it as a side effect.

### Dependency

**Summary**: Couplings are recorded well. The parent (0361), the
downstream consumer (0370) and the already-satisfied 0368 baseline are all
stated explicitly. What is left is implicit coupling to tooling and
infrastructure: cargo-llvm-cov flag behaviour, file-rotate's byte-based
splitting, and CI target-directory caching. None is recorded as a
dependency or a pinned constraint.

**Strengths**:
- 0370 appears in both the frontmatter and the Dependencies section,
  along with its gating on 0362/0367.
- "Blocked by: none" is justified, with a path to the 0368 baseline.
- The third research finding is marked as already fixed.
- Downstream consumers of the coverage output are considered.

**Findings**:
- 🔵 minor (medium) — Requirements — Reliance on specific cargo-llvm-cov
  behaviour (`clean --profraw-only`, `--no-clean`, `--json --summary-only`
  totals shape) is not captured. Record the verified version and how it is
  pinned.
- 🔵 minor (medium) — Context — The equivalence of the chunked-write tests
  depends on file-rotate's byte-based mid-write split. Record the crate and
  the version checked as an assumption.
- 🔵 suggestion (low) — Assumptions — CI coverage correctness depends on
  the CI cache configuration, which is not referenced. Name the workflow
  and cache step.

### Scope

**Summary**: A well-bounded task with a clear parent and explicit
exclusions, and its kind fits the size of the work. The main concern is
that it bundles two independent changes in different components. Two
smaller boundary issues: the coverage-summary fix does not fit the framing
of "repeated work", and the parent epic's list of children still assigns
A4 to this item.

**Strengths**:
- Explicit scope boundaries (A4, de-instrumentation, full-clean cadence).
- Summary, Requirements and Acceptance Criteria line up for A2 and A3.
- The "task" kind suits the size of the work.
- The item fits clearly into 0361 and states what it blocks.

**Findings**:
- 🔵 minor (medium) — Summary — Two independent changes in different
  components are bundled. Note that they can merge separately, or split
  them.
- 🔵 minor (medium) — Requirements — The coverage-summary output fix falls
  outside the stated purpose. Name it in the Summary or move it to its own
  chore.
- 🔵 minor (high) — Drafting Notes — The parent epic's child list still
  assigns A4 to this item. Update 0361.

### Testability

**Summary**: Most criteria are framed as precondition/action/outcome with
numeric thresholds and baselines. The main gap is the Summary's promise of
no dropped test and no dropped coverage figure, which no criterion
verifies. Several criteria also leave out the measurement procedure or
host, and AC4's "still asserts rotation" could pass even if the tests got
weaker.

**Strengths**:
- Performance criteria give both a target and today's baseline.
- AC2 states the stale-profile hazard as something you can observe.
- Context records the experiment that makes Requirement 1 safe.
- Context confirms that rotation is byte-based.

**Findings**:
- 🟡 major (high) — Acceptance Criteria — No criterion verifies that
  coverage figures match a full clean. Add a totals-equivalence criterion
  against a run made straight after `cargo llvm-cov clean --workspace`.
- 🟡 major (medium) — Acceptance Criteria — The "no test dropped" claim has
  no verifiable criterion. Require an unchanged nextest count and that the
  three rotation tests still run and pass.
- 🟡 major (medium) — Acceptance Criteria — "Still asserts rotation" can be
  met by weaker tests. Make it testable by mutation, or list the concrete
  assertions.
- 🔵 minor (medium) — Acceptance Criteria — Timing and CPU criteria lack a
  measurement procedure and host. Define the host, what "warm" means, a
  median of three runs, and the CPU-s command.
- 🔵 minor (medium) — Acceptance Criteria — AC2 does not say how "only
  those the second run wrote" is checked. Use an mtime check and a count
  check.
- 🔵 minor (medium) — Acceptance Criteria — The content of the totals
  summary is not specified. Require line, function and region percentages.
- 🔵 minor (medium) — Requirements — CI instrumentation, the CI job
  speed-up and the docstring have no criteria. Add them, or remove the CI
  claim.

## Re-Review (Pass 2) — 2026-11-13T08:25:06+00:00

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Testability / Completeness**: No criterion verifies coverage figures match a full clean — Partially resolved (a criterion now exists, but its precondition cannot expose inflation; see new issues)
- 🟡 **Testability**: "No test dropped" claim has no verifiable criterion — Resolved
- 🟡 **Testability**: "Still asserts rotation" can be met by weaker tests — Partially resolved (concrete assertions listed; mutation clause and retention wording still loose)
- 🔵 **Completeness / Testability**: Requirement 4 has no criterion — Resolved
- 🔵 **Testability / Completeness**: CI instrumentation and macOS speed-up — Resolved
- 🔵 **Testability / Clarity**: Totals summary content — Resolved (accuracy check added as a new minor)
- 🔵 **Testability**: Timing procedure and host — Resolved
- 🔵 **Testability**: AC2 verification procedure — Resolved
- 🔵 **Clarity**: Kernel ambiguity; research labels; "third finding"; longest task; Blocked-by wording — Resolved
- 🔵 **Scope**: Bundled halves — Partially resolved (separate merge noted; partial-delivery tracking for 0370 still open)
- 🔵 **Scope**: Summary-output fix outside purpose — Resolved (now named in Summary; scope suggests the title too)
- 🔵 **Scope**: Parent child list assigns A4 — Resolved
- 🔵 **Dependency**: cargo-llvm-cov pinning; file-rotate split — Resolved
- 🔵 **Dependency / Clarity**: CI cache assumption — Resolved
- 🔵 **Clarity**: "Runs alone"; docstring wording and `ACCELERATOR_COVERAGE` — Resolved

### New Issues Introduced
- 🟡 **Testability**: Coverage-equality precondition cannot expose stale-profile inflation — two identical runs merged give the same percentages even under bare `--no-clean`; seed with a *different* prior run.
- 🟡 **Testability**: Mutation criterion leaves its mutations unspecified — halving `MAX_BYTES` or raising `MAX_FILES` may pass, and tests that use the constants by name move with them.
- 🟡 **Testability**: 2 CPU-s threshold includes cargo/nextest overhead and possible recompilation — needs a pre-built precondition and a narrowed target, or a same-command baseline.
- 🔵 **Testability / Clarity**: Owner-only permission assertion not preserved by the rotation criterion.
- 🔵 **Testability**: "≤ MAX_FILES" passes when too few rotations happen; require more than `MAX_FILES` rotations.
- 🔵 **Testability**: Printed totals not checked against the export, and "no raw JSON" has no criterion.
- 🔵 **Testability**: `ACCELERATOR_COVERAGE=off` escape hatch has no regression criterion.
- 🔵 **Clarity**: "No coverage figure is dropped" conflicts with removing the per-file table.
- 🔵 **Clarity**: The quick-wins experiment's A2 was coverage-off, not this item's A2.
- 🔵 **Clarity / Testability**: 42 CPU-s baseline is sys time, but the criterion measures user + sys; feature set unspecified.
- 🔵 **Dependency / Scope**: How the two halves release 0370 under partial delivery is not stated.
- 🔵 **Dependency**: Timing verification depends on the baseline host, which is not listed as a dependency.
- 🔵 **Dependency**: CI cache action named only in Assumptions.
- 🔵 **Completeness**: References omits 0362, 0367, 0368, 0370 and the baseline file.
- 🔵 Suggestions: "each pass" wording (clarity); tail-time goal has only a proxy criterion (testability); possible link to the B1 follow-up (dependency); summary-fix rider could be named in the title (scope).

### Assessment
The first pass's findings are almost all resolved, and clarity, completeness, dependency and scope now raise only minor issues. Testability found three new major gaps in the criteria added in pass 1. The most important is that the coverage-equality check, as written, would pass under bare `--no-clean`. These are precise fixes to existing criteria rather than structural problems, but the item needs one more revision before implementation.

## Re-Review (Pass 3) — 2026-11-13T13:38:47+00:00

**Verdict:** COMMENT

### Previously Identified Issues
- 🟡 **Testability**: Coverage-equality precondition cannot expose inflation — Resolved (seeded with a wider prior full run; a control step is suggested as a new minor)
- 🟡 **Testability**: Mutation criterion leaves its mutations unspecified — Resolved
- 🟡 **Testability**: 2 CPU-s threshold includes overhead — Resolved (pre-built, `--lib`, same-command before figure)
- 🔵 **Testability**: Owner-only assertion; retention under-write; totals accuracy and raw JSON; escape-hatch regression — Resolved
- 🔵 **Clarity**: "No coverage figure dropped"; "each pass" wording — Resolved
- 🔵 **Clarity**: Quick-wins A2 variant — Partially resolved (the variant is named, but still under the A2 label)
- 🔵 **Clarity / Testability**: 42 sys-CPU-s vs user + sys — Partially resolved (labelled as sys, but the target still uses a different measure)
- 🔵 **Dependency**: 0370 release rule; verification host; CI cache — Resolved
- 🔵 **Dependency**: B1 link — Partially resolved (recorded here only, not in 0365)
- 🔵 **Completeness**: References — Resolved
- 🔵 **Scope**: Bundled halves — Still present (the per-half rules mitigate it; grouping kept by agreement)
- 🔵 **Scope**: Summary-fix rider — Still present (suggestion only)

### New Issues Introduced
- 🔵 **Testability / Clarity**: Totals-line format, precision and which command carries `--json` are unspecified.
- 🔵 **Testability**: The stale-profile check has no control step showing a bare `--no-clean` run would differ.
- 🔵 **Clarity / Testability**: How a `-p kernel` run goes "through the lane" is unclear; no criterion compares the lane's own full-workspace totals against a full clean.
- 🔵 **Testability**: The profraw freshness check passes vacuously on an empty directory.
- 🔵 **Dependency**: The soft ordering with 0365 is recorded only on this side.
- 🔵 Suggestions: no same-protocol before figure for the 180 s target (testability); CI matrix platform unspecified (clarity); status still `draft` (completeness).

### Assessment
No critical or major findings remain, so the verdict moves to COMMENT. The work item is ready for implementation. The remaining minors tighten the criteria's wording (totals format, control steps, non-vacuous checks) rather than change the work, and can be addressed now or during planning.
