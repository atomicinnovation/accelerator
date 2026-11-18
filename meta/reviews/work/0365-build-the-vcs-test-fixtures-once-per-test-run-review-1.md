---
type: "work-item-review"
id: "0365-build-the-vcs-test-fixtures-once-per-test-run-review-1"
title: "Work Item Review: Build the VCS test fixtures once per test run"
date: "2026-11-18T13:55:46+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0365"
work_item_id: "0365"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: ["performance", "testing", "rust", "vcs"]
last_updated: "2026-11-18T18:25:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Build the VCS test fixtures once per test run

**Verdict:** REVISE

This is a well-evidenced, coherent task. Its measured baseline, mutation
audit, exact speed criterion and explicit out-of-scope boundary are all
stronger than usual. It needs a revision pass because the requirements
disagree about which tests may write to a shared tree and which must adopt
it. Three requirements (6, 8 and 9) also have no acceptance criterion, so the
item could be closed with the coverage lane broken or with a failed shared
build reported in every test that adopts it. All five lenses ran, and none
found a structural problem with the scope.

### Cross-Cutting Themes

- **The list of tests that write to shared trees is inconsistent** (flagged
  by: clarity, testability). Requirement 4 says "the audit found two"
  writers. The audit table, however, says the six `status_log_goldens` tests
  write jj store objects, and `Hermetic::rooted_at` writes under its base.
  Requirement 1 and acceptance criteria 1 and 3 require adoption, while
  Requirement 4 lets writers keep a private build. Neither the adoption
  criterion nor the "unchanged" criterion pins down which tests must adopt
  and which are checked for writes.
- **Requirements 6, 8 and 9 have no acceptance criteria** (flagged by:
  completeness, testability, scope). These cover reporting a failed build
  once, the renames, and the coverage lane under `cargo llvm-cov nextest`.
- **The build-once mechanism and the tooling it relies on are left open**
  (flagged by: completeness, dependency, testability). Requirement 9 defers
  the choice of mechanism, but the Open Questions do not list it. The
  experimental nextest setup-scripts flag, the pinned nextest and llvm-cov
  versions, and the different lanes (filtered nextest, coverage on and off,
  CI legs) are not recorded as couplings or checked by any criterion.
- **Cleanup semantics differ** (flagged by: clarity, testability).
  Requirement 7 says trees "do not accumulate across runs"; the acceptance
  criterion says "no tree from that run remains". Neither covers
  interrupted runs or says where the trees live.

### Findings

#### Major

- 🟡 **Clarity + Testability**: The list of writers to shared trees
  disagrees with the audit table, and the "unchanged" check covers only
  `scrub` and `conflicts`
  **Location**: Requirements, Acceptance Criteria
  Requirement 4 names two writers, but the audit says `status_log_goldens`
  writes jj store objects through `jj status`. Requirement 2 lists those
  tests as unconditional adopters, and the Open Question says they may need
  private builds. Acceptance criterion 3 checks only `scrub` and
  `conflicts`, so writes by the golden tests are never checked.
- 🟡 **Clarity**: Requiring every consumer to adopt contradicts
  Requirement 4's private-build option
  **Location**: Requirements
  Requirement 1 and acceptance criterion 1 ("none of the 19 built its
  own") fail if `scrub` takes the private-build option that Requirement 4
  allows. Acceptance criterion 3 assumes both writers run against the
  shared fixtures.
- 🟡 **Clarity**: "Hermetic environment rooted at the shared build"
  appears to write into the shared tree
  **Location**: Requirements
  `Hermetic::rooted_at` writes `hermetic-home/`, `hermetic-xdg/` and
  `hermetic-jj.toml` under its base. Read literally, every adopting test
  would therefore change the tree that acceptance criterion 3 requires to
  stay unchanged.
- 🟡 **Completeness + Testability + Scope**: Requirements 6, 8 and 9 have
  no acceptance criteria
  **Location**: Acceptance Criteria
  No criterion checks that a failed shared build is reported once, that
  the variables are renamed, or that the coverage lane works. The item
  could be closed with the coverage lane broken or a failed build reported
  in every adopting test.
- 🟡 **Testability**: "Built once" and "adopted" have no observable
  signal
  **Location**: Acceptance Criteria
  Criteria 1 and 2 do not say what evidence shows a single build: a log
  line, setup-script output, a count of spawned processes, or per-test
  timings. A silent fallback to private builds would show up only
  indirectly, in the speed criterion.

#### Minor

- 🔵 **Clarity + Testability**: Requirement 7 and the cleanup criterion
  state different strengths, and neither names a location or covers
  interrupted runs
  **Location**: Requirements, Acceptance Criteria
  Sweeping old trees at the start of the next run meets Requirement 7 but
  fails the criterion. Neither says where the trees live, or whether a
  run killed by SIGKILL counts.
- 🔵 **Testability**: The criterion for a single test must leave its
  pass/fail decision to Requirement 4 or the open question
  **Location**: Acceptance Criteria
  The second criterion accepts "adopted it or keeps a private build", so
  it can pass with almost no adoption. Once the `jj status` question is
  answered, it should list exactly which tests must adopt.
- 🔵 **Testability**: Running a test on its own is checked only for plain
  `cargo test`
  **Location**: Acceptance Criteria
  A filtered nextest run is the most likely to break (a setup script may
  not run), yet no criterion checks it. "Any consumer" is not tied to the
  audit table either.
- 🔵 **Clarity**: Requirement 6 does not say what reports the failure or
  what the adopting tests then do
  **Location**: Requirements
  The tests could fail early, fall back to private builds or skip, and
  each would satisfy the wording.
- 🔵 **Clarity**: It is unclear whether the self-test's adversarial state
  belongs to the shared build
  **Location**: Requirements
  If the self-test adds that state on top of a tree it adopted, it is a
  writer under Requirement 4.
- 🔵 **Clarity**: The timing figures in Context refer to unclear sources
  **Location**: Context
  "The second timed run" names no record. "The 17 slowest tests" does not
  say which 17 of the 19.
- 🔵 **Clarity**: Several terms are used without definition
  **Location**: Requirements, Acceptance Criteria
  `build_states` golden tests (Context calls them `status_log_goldens`),
  "matrix base", "hermetic environment", "test-seconds", "zero-spawn
  lane", "differing condition" and "noise verdict".
- 🔵 **Completeness**: The deferred choice of mechanism is not an open
  question
  **Location**: Open Questions
  Requirement 9 defers it, and the Technical Notes flag the experimental
  nextest feature. Whether relying on that feature is acceptable needs a
  decision and someone to sign it off.
- 🔵 **Dependency**: The tooling and lanes the mechanism must fit are not
  listed as couplings
  **Location**: Dependencies
  These are cargo-nextest (possibly its experimental setup scripts) and
  cargo-llvm-cov as pinned in `mise.toml`, the `ACCELERATOR_COVERAGE=off`
  path, and both CI `test-unit` legs.
- 🔵 **Dependency**: The read-only safety relies on jj, jj-lib and gix
  behaviour that is not recorded
  **Location**: Assumptions
  Upgrading any of them could start writing to shared trees. Record them,
  and note that an upgrade means re-running the mutation audit.
- 🔵 **Dependency**: The baseline's ordering relative to sibling items
  sits only in the Drafting Notes, and the claim about 0364 is wrong
  **Location**: Drafting Notes
  The Drafting Notes say the baseline was taken "once 0362, 0363 and 0364
  had landed", but 0364's Landing Status shows both suites "not landed"
  (verified). 0363's Dependencies says 0365 records the ordering, yet
  Dependencies does not.
- 🔵 **Dependency**: The coupling to the measurement host and to sibling
  items for the fixed-baseline comparison is not captured
  **Location**: Dependencies
  These are engywook with `devtools` settings, not overlapping 0370's
  recording, and 0366 changing debug info between the before and after
  measurements.
- 🔵 **Scope**: Parent epic 0361's decomposition still describes the
  matrix-only scope
  **Location**: Summary
  0361 still lists "0365: Build the VCS fixture matrix once per test run
  (B1)" (verified), and the file slug still says `vcs-fixture-matrix`.
- 🔵 **Scope**: Two separable fixture families with different risks are
  delivered together
  **Location**: Requirements
  The matrix carries about 730 of the 870 test-seconds at low risk. The
  status-log adoption carries the `jj status` uncertainty and the
  `conflicts` rewrite. Consider landing the matrix first.

#### Suggestions

- 🔵 **Scope**: What happens to the adopt variables is left implicit when
  they no longer have a role
  **Location**: Requirements
  Say whether removing them, and the adopt-path code behind them, is in
  scope.
- 🔵 **Testability**: "Still makes every assertion it makes today" has no
  verification method
  **Location**: Acceptance Criteria
  With `--ignore-working-copy`, `conflicts` could keep its assertions
  while checking a weaker state.
- 🔵 **Clarity**: "Reduces the suite's contention" is unexplained, given
  that the Context finds the phase is not CPU-saturated
  **Location**: Dependencies
  Name the resource under contention.

### Strengths

- ✅ The Context gives measured, specific figures from a named baseline
  file. The per-consumer test-seconds add up to the stated 730, and it
  records the two explanations the research ruled out (nextest's per-test
  overhead and CPU saturation).
- ✅ The mutation audit table lists every consumer by fully qualified test
  binary, with its test count, which fixture family it builds and its
  effect on the fixtures. This gives a bounded, checkable set.
- ✅ The speed criterion can be reproduced exactly: it names the baseline
  file, the commands, a 156.9 s threshold, a required "beyond" noise
  verdict and the no-differing-condition precondition, and the Drafting
  Notes justify why 25 s clears the noise band.
- ✅ The test-count floor (at least 7,727) guards against tests being
  dropped silently.
- ✅ The out-of-scope boundary is explicit (cap, bookmark and parity sets)
  and justified, and the `guarded_resume` work was split cleanly into
  0391, with the blocking edge recorded on both sides.
- ✅ The open question about `jj status` has a stated fallback, so it
  shrinks the saving rather than blocking delivery.
- ✅ The Technical Notes give file and line references for both adopt
  paths, both writers and the hermetic base writes.

### Recommended Changes

1. **Settle who writes and who adopts, and align Requirements 1, 2 and 4
   with acceptance criteria 1–3** (addresses: writer list disagrees with
   the audit; adopt-everywhere contradicts the private-build option;
   private-build escape; adversarial state)
   Decide whether writing jj store objects without an operation counts as
   writing to a shared tree. Rewrite Requirement 4's "found two" to match.
   Make Requirement 1 and acceptance criterion 1 conditional on
   Requirement 4, as criterion 2 already is, or remove the private-build
   option for `scrub`. Say where the self-test's adversarial state is
   built.
2. **Say where an adopting test's hermetic directories live** (addresses:
   hermetic environment rooted at the shared build)
   Say whether they are per test and outside the shared tree, and whether
   files the test harness writes count as changes.
3. **Widen the "unchanged" criterion to every adopting test and define
   "changed"** (addresses: "unchanged" check covers only two tests)
   For example, a content hash of each shared tree taken after it is built
   equals one taken before teardown, with any accepted exception stated.
4. **Add acceptance criteria for Requirements 6, 8 and 9** (addresses:
   Requirements 6, 8 and 9 have no criteria; Requirement 6 names no
   reporter)
   - Requirement 6: a forced build failure gives one failure that names
     the cause, and the adopting tests behave in a stated way.
   - Requirement 8: no `ZERO_SPAWN` matches remain in the repository.
   - Requirement 9: the coverage lane passes with each family built once.
     Name the mise task.
5. **Name the evidence that a family was built once and adopted**
   (addresses: no observable signal)
   For example, one build record per family in the run output, or each
   consumer's time in the nextest report falling below a stated threshold.
6. **Extend the criterion for running a test alone to a filtered nextest
   run and tie it to the audit table** (addresses: running a test alone
   checked only for `cargo test`)
7. **Make Requirement 7 and the cleanup criterion agree, and name the
   tree location** (addresses: cleanup semantics differ)
   Cover passing, failing-test, failed-build and interrupted runs.
8. **Add an open question for the build-once mechanism, and record the
   tooling and lane couplings in Dependencies** (addresses: deferred
   mechanism; tooling couplings; jj, jj-lib and gix behaviour)
   Record nextest's experimental setup scripts, the pinned nextest and
   llvm-cov versions, coverage on and off, the CI legs, and jj, jj-lib and
   gix as things to re-audit when upgraded.
9. **Fix the baseline provenance and record the measurement couplings**
   (addresses: wrong 0364 claim; host and sibling coupling)
   Correct or drop the claim that 0364 had landed, move the 0363 ordering
   into Dependencies, and name engywook with `devtools` and the overlap
   with 0370.
10. **Smaller fixes for clarity and scope** (addresses: unclear
    referents; undefined terms; epic decomposition; adopt-variable
    removal; contention)
    - Name the record and run behind the per-test figures.
    - Use one name for the four golden tests.
    - Update 0361's decomposition entry.
    - Say whether dead adopt code is removed.
    - Name the resource under contention.
    - Optionally, state that the matrix lands first.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is precise about its measurements, names
concrete test binaries and includes a mutation audit, so a reader with
domain knowledge can follow most of it. The main problems are in how the
requirements fit together. Requirements 1 and 2 and the matching criteria
require adoption while Requirement 4 allows private builds. The writer list
disagrees with the audit table. "Hermetic environment rooted at the shared
build" appears to write into trees the criteria require to stay unchanged.
Some referents and terms are undefined, and the failure-reporting and
cleanup requirements are under-specified.

**Strengths**:
- The Summary, Context and audit table use the same figures (19 matrix
  builds, 6 full-set builds, about 870 of 2,818 test-seconds), and the
  per-binary breakdown adds up to 730.
- The audit names each consumer by fully qualified test binary and states
  its effect on the fixtures.
- The "adopt path" is explained where it is first used.
- The out-of-scope boundary is explicit and justified.
- The speed criterion names the exact commands, the baseline and the
  threshold.

**Findings**:
- 🟡 major / high — Requirements: Requiring every consumer to adopt
  contradicts Requirement 4's private-build option. If `scrub` keeps a
  private build, which Requirement 4 allows, acceptance criteria 1 and 3
  fail. Decide whether the private-build option is allowed, and make
  Requirement 1 and acceptance criteria 1 and 3 conditional on it, or
  remove the option.
- 🟡 major / high — Requirements: The writer list in Requirement 4
  disagrees with the audit table and the open questions. The audit says
  `status_log_goldens` writes store objects. Requirement 2 lists those
  tests as unconditional adopters, and the open question says they may
  need private builds. State whether writing store objects counts, and
  align the text.
- 🟡 major / medium — Requirements: "Hermetic environment rooted at the
  shared build" seems to require writing into the shared tree.
  `Hermetic::rooted_at` writes under its base. State where the hermetic
  directories live and whether the harness's writes count as changes.
- 🔵 minor / medium — Requirements: Requirement 6 does not say what
  reports the failure or what the adopting tests do. Name the component
  and the observable outcome.
- 🔵 minor / medium — Acceptance Criteria: The cleanup requirement and
  criterion state different strengths. Make them agree.
- 🔵 minor / high — Context: The timing figures have unclear referents
  ("second timed run", "17 slowest"). Name the record and run, and the
  tests.
- 🔵 minor / medium — Requirements: Several terms are undefined
  (`build_states`, matrix base, hermetic environment, test-seconds,
  zero-spawn lane, differing condition, noise verdict).
- 🔵 minor / medium — Requirements: It is unclear whether the self-test's
  adversarial state belongs to the shared build.
- 🔵 suggestion / low — Dependencies: "Reduces the suite's contention" is
  unexplained, given that the phase is not CPU-saturated.

### Completeness

**Summary**: The task is complete for its kind. Every expected section is
present and well populated, and the frontmatter is intact. There are two
gaps: Requirements 6, 8 and 9 have no matching acceptance criterion, and
the deferred choice of mechanism is not listed as an open question.

**Strengths**:
- The Context explains why the work is needed, with measured figures and
  the alternatives that were ruled out.
- The mutation audit table lets an implementer start without asking which
  tests are safe to share.
- The scope boundaries are explicit and justified.
- The Dependencies section has real content: the baseline, 0391 with its
  reason, and 0264.
- The Technical Notes give file and line locations.

**Findings**:
- 🟡 major / medium — Acceptance Criteria: Requirements 6, 8 and 9 have no
  acceptance criteria. Add one each: a forced build failure reported once,
  the renamed variables, and the coverage lane green.
- 🔵 minor / medium — Open Questions: The deferred choice of mechanism is
  not captured as an open question. Add it, with the candidate approaches,
  their constraints, and whether the experimental nextest flag is
  acceptable.

### Dependency

**Summary**: The item captures its work-item couplings well: the edge to
0391 is recorded on both sides, and 0264 and the baseline are named. What
it leaves out are the environment and tooling couplings its siblings
record: the reference host and sibling changes, the nextest and llvm-cov
features, and the jj, jj-lib and gix behaviour the safety argument relies
on. The Drafting Notes' claim about 0364 conflicts with 0364's own record.

**Strengths**:
- The edge to 0391 is captured on both sides, with its reason.
- The 0264 entry explains why the adopt variables are unset today.
- The measurement prerequisite is pinned to a named baseline, and the
  tooling comes from 0368, which is done.
- The parent link to 0361 is present.

**Findings**:
- 🔵 minor / medium — Dependencies: The coupling to the measurement host
  and to sibling changes is not captured (engywook with `devtools`,
  overlap with 0370, 0366 debug info). Add an environment entry, and say
  whether a fresh before-record is taken.
- 🔵 minor / medium — Drafting Notes: The baseline's ordering relative to
  0363 and 0364 is only in the Drafting Notes, and 0364's record shows it
  has not landed. Move the ordering to Dependencies and correct the 0364
  claim.
- 🔵 minor / medium — Requirements: The test-runner tooling and lane
  couplings are not listed (nextest and llvm-cov pins, experimental setup
  scripts, `ACCELERATOR_COVERAGE=off`, the CI legs).
- 🔵 minor / medium — Assumptions: The read-only safety relies on jj,
  jj-lib and gix behaviour that is not recorded. Name them, and require
  the audit to be re-run when they are upgraded.

### Scope

**Summary**: The item is one coherent unit, and its boundaries are
unusually explicit. The remaining concerns are small: the parent epic's
decomposition is stale, two separable families with different risks are
bundled together, and Requirements 6, 8 and 9 are not reflected in the
criteria.

**Strengths**:
- Requirement 3 and the audit row exclude the cap, bookmark and parity
  sets, with justification.
- `guarded_resume` was split cleanly into 0391.
- The audit makes the unit of delivery concrete.
- Requirement 4 gives each writer a bounded fallback.
- The open question has a fallback that shrinks the saving rather than
  blocking.

**Findings**:
- 🔵 minor / high — Summary: The parent epic's decomposition still
  describes the matrix-only scope. Update 0361's entry.
- 🔵 minor / medium — Requirements: Two separable fixture families with
  different risks are bundled together. Land the matrix first, or split
  the status-log adoption into a sibling item.
- 🔵 minor / medium — Acceptance Criteria: Requirements 6, 8 and 9 are not
  reflected in the criteria. Add criteria, or move the optional ones into
  the Technical Notes.
- 🔵 suggestion / medium — Requirements: What happens to the adopt
  variables when they lose their role is left implicit.

### Testability

**Summary**: The item is unusually strong on verification: the criteria
use Given/When/Then, the speed target is reproducible, and there is a
test-count floor. The gaps are in coverage: Requirements 6, 9 and 8 have no
criterion, the "unchanged" check covers only `scrub` and `conflicts`, and
"built once" and "adopted" have no observable signal.

**Strengths**:
- The speed criterion can be fully reproduced, with its threshold, noise
  verdict and precondition.
- The test-count floor guards against tests being dropped.
- The audit table gives a bounded set to verify.
- Most criteria name a precondition, an action and an observable outcome.

**Findings**:
- 🟡 major / high — Acceptance Criteria: Requirement 6 (a failed shared
  build reported once) has no acceptance criterion. Add a fault-injection
  criterion.
- 🟡 major / high — Requirements: The coverage-lane requirement
  (Requirement 9) is not checked by any criterion. Add one naming the
  coverage task.
- 🟡 major / high — Acceptance Criteria: The "unchanged" check covers
  `scrub` and `conflicts` but not the golden tests the audit says write to
  the jj store. Widen it to every adopter, and define "changed" as a
  content hash of each tree.
- 🟡 major / medium — Acceptance Criteria: "Built once" and "adopted it"
  have no observable signal. Name the evidence.
- 🔵 minor / high — Acceptance Criteria: The "or keeps a private build"
  escape weakens the criterion for status-log adoption. List the expected
  split.
- 🔵 minor / high — Acceptance Criteria: Running a test alone is checked
  only for plain `cargo test`. Add filtered nextest, and tie the criterion
  to the audit.
- 🔵 minor / medium — Acceptance Criteria: The cleanup criterion names no
  location and covers only failures where cleanup can still run.
- 🔵 minor / high — Requirements: The renaming requirement has no
  verifiable criterion. Require no `ZERO_SPAWN` matches.
- 🔵 suggestion / medium — Acceptance Criteria: "Still makes every
  assertion it makes today" has no stated way to verify it.

## Re-Review (Pass 2) — 2026-11-18

**Verdict:** REVISE

The work item was revised after decisions on writers (a test that can stop
writing adopts the shared build, otherwise it works in a copy taken from
it), cleanup (each run removes its own trees when it ends, and the next run
removes trees left by interrupted runs), delivery (the matrix lands first)
and a rename to match the new title. All five lenses were re-run.

### Previously Identified Issues
- 🟡 **Clarity + Testability**: The writer list disagreed with the audit,
  and the "unchanged" check covered only two tests — Resolved. The check
  now hashes every tree, but see the new conflict with the Open Question.
- 🟡 **Clarity**: Requiring every consumer to adopt contradicted the
  private-build option — Resolved.
- 🟡 **Clarity**: The hermetic environment was rooted at the shared
  build — Resolved. Completeness notes it still has no criterion.
- 🟡 **Completeness + Testability + Scope**: Requirements 6, 8 and 9 had
  no criteria — Resolved. Two of the new criteria raise new issues
  (`ZERO_SPAWN`, fault injection).
- 🟡 **Testability**: "Built once" had no observable signal — Partially
  resolved. The criteria point at "output", but no requirement says what
  that output is.
- 🔵 **Clarity + Testability**: The cleanup wording disagreed — Resolved.
  How a run is interrupted is still not set up.
- 🔵 **Testability**: The private-build escape — Resolved.
- 🔵 **Testability**: A test run on its own was checked only with plain
  `cargo test` — Partially resolved. A filtered nextest run is now
  included, but the criterion now prescribes a mechanism.
- 🔵 **Clarity**: Requirement 6 named no reporter or outcome — Partially
  resolved. The outcome is stated; how to cause the failure is not.
- 🔵 **Clarity**: Where the adversarial state lives — Partially resolved.
  The self-test is still described both as read-only and as possibly
  working in a copy.
- 🔵 **Clarity**: Unclear references in Context — Still present ("17
  slowest" against 19 consumers; "second timed run").
- 🔵 **Clarity**: Undefined terms — Mostly resolved. New borrowed terms:
  CI "legs", the "halves" of 0363, engywook.
- 🔵 **Completeness**: The build mechanism was not an open question —
  Resolved.
- 🔵 **Dependency**: Tooling and lane couplings — Resolved. Git is
  missing.
- 🔵 **Dependency**: jj, jj-lib and gix behaviour — Resolved.
- 🔵 **Dependency**: Baseline ordering and the claim about 0364 —
  Resolved.
- 🔵 **Dependency**: Host and sibling couplings — Partially resolved. The
  two-increment delivery and 0370's lasting change to the host settings
  are not handled.
- 🔵 **Scope**: The epic's breakdown — Resolved.
- 🔵 **Scope**: Two families delivered together — Accepted. Order stated;
  scope still notes there is one set of criteria for both increments.
- 🔵 **Scope**: What happens to the adopt variables — Resolved.
  `STATES_ROOT_VARIABLE` is not named.
- 🔵 **Testability**: Checking that assertions are kept — Partially
  resolved. A baseline commit is named; the goldens are not covered.
- 🔵 **Clarity**: Contention was unexplained — Resolved.

### New Issues Introduced
- 🟡 **Clarity + Testability**: The content-hash criterion conflicts with
  the outcome of the `jj status` Open Question. If the goldens may share,
  their store writes change the hash. The criterion also doesn't say what
  the hash covers.
- 🟡 **Clarity + Testability**: The `ZERO_SPAWN` search of the whole
  repository can never pass. Documents under `meta/`, including this one,
  contain the string. The rename half of Requirement 9 is not checked.
- 🟡 **Clarity**: Nothing separates "a run", where tests share, from "a
  test run on its own", where a test builds privately. A test can't tell a
  missing shared build from a failed one, so Requirements 6 and 7 can read
  as contradictory.
- 🟡 **Testability**: The criteria rely on "output" that no requirement
  says the build, adopt and copy paths must produce. nextest also hides
  the output of passing tests.
- 🟡 **Testability**: The forced shared-build failure has no defined way
  to cause it. A failing `git` or `jj` would also break unrelated tests.
- 🔵 **Completeness + Testability**: No criteria cover the copy properties
  in Requirement 5, the hermetic environment in Requirement 3, or the
  increment order in Requirement 11.
- 🔵 **Testability**: The assertion check leaves out the goldens. The
  test-count floor is not tied to a fresh before-record. Nothing checks
  that the cap, bookmark and parity sets stay private.
- 🔵 **Clarity**: Requirement 9 leaves out `STATES_ROOT_VARIABLE`. The
  Context doesn't explain why a test takes 30–46 s when one build takes
  3.3 s.
- 🔵 **Dependency**: The fresh before-record rule doesn't fit two
  increments. 0370 leaves engywook with changed settings. Git is not
  listed. 0363, 0366, 0368 and 0370 are missing from `relates_to`, and
  0370 has no note pointing back to this item.
- 🔵 **Scope**: The single closing criterion spans both increments; it is
  unclear whether the 25 s gate applies after the matrix increment. The
  `ZERO_SPAWN` criterion reaches beyond Requirement 9 (also above).
- 🔵 **Scope**: Suggestion: the kind "task" may understate an item that
  carries a design choice.

### Assessment
The first pass's structural contradictions are gone. The writer and
adopter model, cleanup and delivery order are now coherent and agreed.
What remains is how to verify the work: a defined record of each build,
adopt and copy; a reachable `ZERO_SPAWN` scope; the condition that turns
sharing on; a way to cause a build failure; and reconciling the content
hash with the `jj status` question. These are all small, local edits, but
together they keep the verdict at REVISE. One more pass on these should
make the item ready for planning.

## Re-Review (Pass 3) — 2026-11-18

**Verdict:** REVISE

The work item was revised after pass 2's decisions:
- the `jj status` goldens always work in copies;
- a build step in `test:unit:cli`'s invoke task hands each build to the
  tests explicitly, and builds only the families the selected tests need;
- a `--filter` option is declared on the invoke task and exposed by the
  mise task;
- guidance rules out `cargo test`;
- a fixture record plus an end-of-run summary;
- a test-only switch that makes one family's build fail.

All five lenses were re-run.

### Previously Identified Issues
- 🟡 **Clarity + Testability**: The content hash conflicted with the
  `jj status` Open Question — Resolved. The goldens always copy.
- 🟡 **Clarity + Testability**: The `ZERO_SPAWN` search scope could not
  pass — Resolved. The search is limited to `cli/`, `tasks/`, `mise.toml`
  and `.github/`, and covers both root constants.
- 🟡 **Clarity**: Nothing separated a shared run from a test run alone —
  Resolved. The task hands each build over explicitly; a test handed
  nothing builds its own.
- 🟡 **Testability**: "Output" had no defined source — Resolved for
  `test:unit:cli` runs (the fixture record). See the new issue about
  direct nextest runs.
- 🟡 **Testability**: There was no defined way to cause a build failure —
  Resolved. A test-only switch on the build step.
- 🔵 **Completeness + Testability**: Criteria were missing for the copy
  properties, the hermetic environment and the increments — Partially
  resolved. The copy-path criterion was added; the hermetic environment,
  timestamps and increment gates are still missing.
- 🔵 **Testability**: Checking that assertions are kept — Partially
  resolved. Now covers every changed consumer, but still relies on
  judgement.
- 🔵 **Testability**: The test-count floor — Partially resolved. It is
  tied to the before-record, but that record has no test-count field.
- 🔵 **Testability**: Checking the cap, bookmark and parity sets stay
  private — Resolved.
- 🔵 **Clarity**: `STATES_ROOT_VARIABLE`; the self-test's mode; borrowed
  terms — Resolved.
- 🔵 **Clarity**: Unclear figures in Context — Still present. 17 vs 19,
  "second timed run", and now 30–46 s against the research's 35–47 s.
- 🔵 **Dependency**: Two increments versus a fresh before-record; 0370's
  lasting settings; `relates_to` — Resolved.
- 🔵 **Dependency**: Git missing — Partially resolved. Git was added but
  described as pinned in `mise.toml`, which it is not (verified).
- 🔵 **Scope**: One closing criterion for both increments — Partially
  resolved. The speed criterion now applies after both, but the matrix
  increment's criteria are not identified.

### New Issues Introduced
- 🟡 **Clarity + Testability + Completeness**: The criteria for the
  matrix increment are not identified. Requirement 14 says "the criteria
  for the matrix family", but several criteria (lanes, the `ZERO_SPAWN`
  search, the failure switch, cleanup, the hash) cover both families.
- 🟡 **Testability + Clarity**: The content-hash and copy-path checks
  need evidence that the run's cleanup destroys, and no one is named to
  take them. It is unclear whether they are permanent guards or one-off
  checks.
- 🟡 **Testability**: A direct `cargo nextest` run has no defined fixture
  record, yet a criterion expects it to "record a private build". "Any
  test" is not tied to the audit table's rows.
- 🟡 **Clarity**: The guidance change is worded for every suite ("nowhere
  tell readers to drop to the underlying runner"), but only
  `test:unit:cli` gains a filter.
- 🟡 **Scope** (accepted in pass 1): Split the status-log sharing into a
  sibling item. Kept whole by decision, with the order stated.
- 🔵 **Dependency**: The speed criterion measures nextest's phase, so the
  new build step's cost falls outside it. The fixture summary must not
  look like nextest's `Summary [` line.
- 🔵 **Dependency**: The GitHub-hosted runner images (copy-on-write
  support, git version) are not recorded. 0366's count of compile
  configurations is not coordinated with a new fixture entry point.
  Measurement sessions for 0364 on engywook could collide with this
  item's. 0361's closing comparison counts on this saving.
- 🔵 **Clarity**: Overlapping with test compilation conflicts with
  selecting families through `cargo nextest list`, which compiles first.
  "Before anything is built" is ambiguous.
- 🔵 **Clarity**: "Copy", "build" and "record" each have more than one
  sense. The fields of each event in Requirement 12 are unclear. "The
  `status_log_goldens` tests" means different sets of tests in different
  places.
- 🔵 **Completeness + Testability**: There are no criteria for `--filter`
  appearing in help, for separate hermetic environments, for copied
  timestamps, or for a filter that selects only status-log consumers.
- 🔵 **Completeness**: The remaining Open Question has no point by which
  it must be answered.
- 🔵 **Scope**: `--filter` and the guidance could be split out; the kind
  "task" understates the size of the work.
- 🔵 **Clarity**: Several measurement terms are not glossed ("warm
  no-op", "isolated", `devtools`). The note about partial copies sits
  outside Requirement 6.
- 🔵 **Testability**: The preference for adopting over copying cannot be
  verified.

### Assessment
Each pass has narrowed the findings. The design is now settled and
coherent, and what remains is mostly about when and how each criterion
is checked. One decision per area remains:
- which criteria gate the matrix increment;
- whether the hash and copy-path checks are permanent guards;
- what a direct nextest run records;
- the scope of the guidance;
- whether the speed criterion also bounds the task's total time.

The rest are mechanical fixes.

## Re-Review (Pass 4) — 2026-11-18

**Verdict:** REVISE

The work item was revised after pass 3's decisions:
- criteria are tagged by increment ([matrix], [status-log], [each family],
  [filters], [closure]);
- the hash and copy-path checks run on every run;
- a test writes fixture events only when given a log location;
- a `--filter` option on every multi-test `test:*` task, landing as a
  third increment;
- the speed criterion also limits the task's total duration;
- unsupported figures were removed from the Context.

All five lenses were re-run.

### Previously Identified Issues
- 🟡 **Clarity + Testability + Completeness**: The criteria for the matrix
  increment were not identified — Resolved by the tags. But see the new
  issue: several [matrix] criteria are never re-checked for the
  status-log increment.
- 🟡 **Testability + Clarity**: The hash and copy-path checks were
  destroyed by cleanup — Resolved. They are permanent guards in the build
  step, recorded in the fixture log. The path scan has no negative test.
- 🟡 **Testability**: A direct nextest run had no record — Resolved. The
  criterion now checks the outcome.
- 🟡 **Clarity**: The guidance scope across suites — Resolved, by
  widening the scope (every multi-test task gets `--filter`). Scope now
  flags the widening itself.
- 🟡 **Scope**: Split out the status-log sharing — Accepted.
- 🔵 **Dependency**: The build step falls outside the measured phase —
  Resolved. Both the duration and the phase are bounded.
- 🔵 **Dependency**: Runner images; 0366; 0364's sessions; 0361's
  reliance; git not pinned — Resolved. The 0366 rule is not mirrored in
  0366.
- 🔵 **Clarity**: Overlap with compiling the tests; terms; event fields;
  which `status_log_goldens` tests; Context figures — Resolved.
- 🔵 **Completeness + Testability**: Criteria for help output, hermetic
  roots, timestamps, and a status-log-only filter — Resolved.
- 🔵 **Testability**: The test-count source — Resolved. The assertion
  check — Partially resolved (still relies on judgement).
- 🔵 **Completeness**: The Open Question had no owner — Resolved.
- 🔵 **Scope**: The kind is "task" — Still present. Awaiting your
  decision.

### New Issues Introduced
- 🟡 **Scope**: The `--filter` options on the other suites and the
  guidance rewrite are independent of fixture sharing, yet they hold the
  item's closure. Also, the speed criterion says it is measured "once both
  fixture increments have landed", but it is tagged [closure], which
  waits for all three increments.
- 🟡 **Clarity**: The boundary of the "status-log family" is undefined.
  Is the plain-git subset part of it? Where do the private cap, bookmark
  and parity builds appear in the summary? Each family goes by several
  names.
- 🟡 **Testability**: The cleanup, interrupted-run, no-consumer-filter and
  malformed-filter criteria are tagged [matrix], so the status-log trees
  and copies are never checked against them.
- 🟡 **Testability**: The copy-path guard has no negative test (a copy
  with the shared tree's path planted in it).
- 🔵 **Clarity + Testability**: "As the audit table expects" has no answer
  for the conditional rows. The decision recorded during planning should
  be the expected result.
- 🔵 **Testability**: Matching mtimes don't prove git and jj see a copy the
  same way as a private build. Compare `git status` and `jj status`
  output too.
- 🔵 **Testability**: No criteria cover printing the summary on failure,
  the clash with `Summary [`, the git and jj versions in the log, the log
  surviving cleanup, or a direct run writing no log.
- 🔵 **Testability**: The concurrent-run cleanup has no way to hold run B
  in progress deterministically.
- 🔵 **Completeness + Testability + Clarity**: Requirement 5 does not list
  the in-scope tasks, their filter syntax, or a malformed example for
  each. The full names of the exempt guards are missing.
- 🔵 **Clarity**: The [filters] increment may land before [matrix], but its
  guidance refers to `test:unit:cli --filter`. The 870 test-seconds figure
  is used both as the tests' total duration and as the saving. A few terms
  are undefined ("adversarial state", "base directory", "0366's count").
- 🔵 **Dependency**: The 0391 edge should be released once the status-log
  increment lands. The 0366 rule should be mirrored in 0366. 0364 changes
  some of the same test tasks. The mise feature for declaring task
  arguments is not listed.
- 🔵 **Completeness**: Decisions left to planning are scattered rather
  than gathered under Open Questions.

### Assessment
Of the four major findings, two are mechanical: retagging criteria and
adding a negative test. One is a definition: what makes up each fixture
family. The fourth reopens your decision to put every suite's filter in
this item. The design itself held up across all five lenses. The findings
are now about the edges of the criteria, not about the approach, so
another full pass is likely to give diminishing returns.

## Verdict Change — 2026-11-18

**Verdict:** APPROVE

After pass 4, the work item was revised once more. The revision:
- split the closure so the fixture result does not wait for the filters
  increment;
- defined the fixture families and the private sets;
- retagged criteria to [each family];
- added a negative test for the copy-path guard and a check of the VCS
  view of copies;
- added a test-only switch that holds the run after its build step;
- gathered the decisions left to planning under Open Questions.

The reviewer judged that this addresses pass 4's major findings and
approved the work item without a fifth pass.
