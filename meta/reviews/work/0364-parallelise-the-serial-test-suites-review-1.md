---
type: "work-item-review"
id: "0364-parallelise-the-serial-test-suites-review-1"
title: "Work Item Review: Parallelise the serial test suites"
date: "2026-11-15T12:16:43+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0361"
target: "work-item:0364"
work_item_id: "0364"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: ["build-system", "performance", "testing"]
last_updated: "2026-11-15T15:09:21+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Parallelise the serial test suites

**Verdict:** REVISE

The item is well evidenced and structurally complete. Every section is
populated, the Context backs each claim with measured timings, and the
Drafting Notes make the author's interpretations explicit. Its weaknesses are
in how "done" is defined:
- Several acceptance criteria rest on things that are not pinned down: a
  reference host, a recorded baseline, a race, or CI runs after merge.
- The rule for flakes that get split out contradicts the Dependencies section.
- The Summary promises more than the criteria require.

The scope is coherent but bundles two suites that could land separately.

### Cross-Cutting Themes

- **The CI median criterion** (flagged by: dependency, testability, scope).
  The closing criterion compares 10 CI runs before the change with 10 after
  it. It depends on CI history that may not be retained and on CI activity
  after merge. It has no tolerance for noise, and it keeps the item open after
  landing. All three lenses suggest recording the baseline before merging and
  either adding a tolerance or moving the check to 0361 or a follow-up item.
- **Split-out flakes and the definition of done** (flagged by: clarity,
  dependency, scope). The Assumptions say the item "stays blocked" on any
  flake that is split out. Requirement 4 says every flake is fixed here, and
  Dependencies says "Blocked by: none". The flake-fixing duty also has no
  limit, so one hard flake in one suite holds back both.
- **Requirement 6 is under-specified and not verified** (flagged by:
  completeness, testability, clarity, scope). Requirement 6 is the
  wait-shrinking and budget-keeping requirement. No criterion checks that the
  waits shrank or that the readiness, pidfile and frontend-active budgets kept
  their values. The "timeout branch" criterion has no baseline. It is unclear
  whether production defaults or only test overrides change. The title does
  not mention this part of the work.
- **Timing criteria don't name a host** (flagged by: testability, clarity).
  Worker counts depend on the host, but the 75 s and 60 s targets, the 10-run
  validation and the "warm workspace" precondition are not tied to a host or
  defined.

### Findings

#### Major

- 🟡 **Testability**: Wall-time thresholds have no reference host even though
  worker counts scale with the host
  **Location**: Acceptance Criteria
  "Under 75 s" and "under 60 s" are conditioned only on "a warm workspace".
  With `50%` and `-n auto`, the same change can pass on a 12-core laptop and
  fail on a 4-core machine.
- 🟡 **Testability**: "Still exercise each timeout branch they exercised
  before" has no recorded baseline
  **Location**: Acceptance Criteria
  The item doesn't list which negative-path tests target which timeout
  branches, or how a branch is shown to be reached. Shrinking the waits is
  exactly the change that can quietly turn a timeout test into a happy-path
  test.
- 🟡 **Testability**: Fixture-independence criterion relies on a
  nondeterministic concurrent pass and an audit with no defined form
  **Location**: Acceptance Criteria
  A passing concurrent run only shows that the interleaving was harmless that
  time. The audit's required content isn't defined; a suitable form would be
  the set of files written and the set read, with an empty intersection.
- 🟡 **Dependency / Testability / Scope**: CI median criterion depends on CI
  history and on 10 future CI runs, has no noise tolerance, and extends the
  item past merge
  **Location**: Acceptance Criteria
  Neither external coupling appears in Dependencies. A strict "not higher"
  test on shared 3–4 vCPU runners can fail on noise alone, and the item can't
  close at merge.
- 🟡 **Dependency**: Coordination with sibling items under 0361 isn't
  captured
  **Location**: Dependencies
  The research's combined what-if analysis shows that the A1–A4 and B1–B6
  changes interact. A sibling item that lands during the 10-run validation, or
  between the before and after CI windows, makes them invalid.
- 🟡 **Clarity / Dependency**: "Stays blocked" on split-out flakes
  contradicts "Blocked by: none"
  **Location**: Assumptions, Dependencies
  An implementer who finds a hard flake can't tell whether to fix it here,
  split it out and wait, or merge.
- 🟡 **Clarity**: "Cuts each by more than half" contradicts the Playwright
  target and the CI criterion
  **Location**: Summary
  A 75 s target against 131 s is a cut of about 43%, and CI only needs to be
  "no slower". That gives three different bars for success.

#### Minor

- 🔵 **Completeness / Testability**: Requirement 6's shrunk waits and kept
  budgets have no acceptance criterion
  **Location**: Acceptance Criteria
  xdist alone might meet the 60 s target. The waits could then go unchanged,
  or the CI-protecting budgets could be cut, and no criterion would catch
  either.
- 🔵 **Testability**: Running the two Playwright projects concurrently has no
  criterion
  **Location**: Requirements
  The 75 s target could be met with the projects still running one after the
  other.
- 🔵 **Scope**: Two suite changes that could ship independently are bundled
  into one item
  **Location**: Summary
  Together with the blocked-on-flakes rule, a single Playwright flake would
  hold back the dev-suite saving (163 s to 40 s), and vice versa.
- 🔵 **Scope**: The wait-shrinking requirement goes beyond the "parallelise"
  scope in the title and Summary
  **Location**: Requirements
  The research presents it as an alternative to xdist, and xdist alone already
  meets the 60 s target.
- 🔵 **Scope**: Fixing every exposed flake has no natural upper bound
  **Location**: Requirements
  There is no time-box or count beyond which flakes become their own items.
- 🔵 **Dependency**: The Docker visual-regression lane uses the same
  Playwright config but isn't named as affected
  **Location**: Requirements
  `test:e2e:visualiser:docker` and the CI `test-visual-regression` job are
  outside the `mise run` validation.
- 🔵 **Clarity**: Unclear how the global and per-project worker counts
  combine, and which counts must be host-relative
  **Location**: Requirements
  "High enough" has no value given. The plural "worker counts" in the CI
  criterion could be read as covering `chromium`'s fixed count of 1.
- 🔵 **Clarity**: Requirement 6 doesn't say whether production timeouts or
  test-only overrides shrink
  **Location**: Requirements
  Changing production defaults would change how the shipped tooling behaves.
- 🔵 **Clarity**: Several terms are used without definition
  **Location**: Context
  A5/A6, circusd, arbiter, "the fakes", "a warm workspace", and "a full
  `mise run` load".

#### Suggestions

- 🔵 **Testability**: The host and load for the 10-run flake validation are
  not specified
  **Location**: Acceptance Criteria
- 🔵 **Scope**: Kind `task` may understate the combined scope
  **Location**: Frontmatter: kind
- 🔵 **Dependency**: `tasks/test/integration.py` is also edited by sibling A4
  **Location**: Technical Notes
- 🔵 **Clarity**: The 0368/0361 baseline note doesn't say why it bears on
  merging
  **Location**: Dependencies
- 🔵 **Clarity**: Ambiguous "that" in "Today that rests on a config comment"
  **Location**: Context

### Strengths

- ✅ The Context gives measured evidence: a wall-time table for each project
  and worker count, 163 s serial against 40 s under xdist, and the fixed waits
  that dominate each dev test.
- ✅ The Requirements name exact settings and values (`workers: "50%"`,
  `retries` 0, `-n auto`, `retain-on-failure`). The Technical Notes give file
  locations and confirm that Playwright 1.59.1 supports per-project workers.
- ✅ The Drafting Notes state each interpretation outright: retries in CI, the
  trace mode, the meaning of "independent", and "no slower" as a median.
- ✅ Requirement 4 forbids quarantining, skipping or re-adding retries, so the
  flake criterion can't be met by suppressing tests.
- ✅ The scope is deliberately bounded: per-test fixture copies are excluded,
  "faster on CI" is an aim rather than a gate, and the independence work is
  limited to resolved-styles reading nothing that chromium mutates.
- ✅ Requirement 6 separates pure-delay waits from the readiness budgets that
  protect loaded CI runners.
- ✅ The Dependencies section explains why the earlier gate on 0368 has
  cleared and links to the recorded measurement.

### Recommended Changes

1. **Pin the timing criteria to a reference host and define "warm"**
   (addresses: no reference host for the thresholds; terms used without
   definition; host for the 10-run validation)
   Name the machine and its CPU count on which the 131 s and 163 s baselines
   were measured. Either state the 75 s and 60 s targets for that host or for
   a host with at least N CPUs, or say that CI is governed only by the median
   criterion. Define "warm workspace". Say where the 10-run validation runs.
2. **Resolve the split-out-flake rule** (addresses: "stays blocked"
   contradicts "Blocked by: none"; flake-fixing has no upper bound; two suite
   changes bundled)
   Decide on one rule and state it in Requirement 4, the Assumptions and
   Dependencies. One option: a split-out flake is added as "Blocked by" for
   the suite it affects only, and each suite may land once its own criteria
   pass. Add a time-box or flake count beyond which flakes become their own
   items. Alternatively, split the item into Playwright and dev-suite siblings
   under 0361.
3. **Make the CI median criterion checkable** (addresses: CI median criterion)
   Record the pre-change 10-run medians and their run IDs before merging, add
   a noise tolerance (for example ≤ 5%), and name the GitHub Actions run
   history in Dependencies. Consider moving the post-landing comparison to
   0361 or a follow-up item so that this item closes at merge.
4. **Add criteria for Requirement 6 and baseline the timeout branches**
   (addresses: no criterion for Requirement 6; no baseline for timeout
   branches; production versus test-only overrides)
   Before changing anything, list each negative-path test and the timeout
   branch it targets, and require each test to assert that its branch was
   taken. Add criteria that the happy-path waits shrank and that the
   readiness, pidfile and frontend-active budgets keep their current values.
   State that only test-side values change and that production defaults stay
   as they are. Mention the wait reduction in the title or Summary, or move it
   to its own item.
5. **Make the independence audit structural** (addresses: fixture-independence
   criterion relies on a nondeterministic pass)
   Define the audit's output as the files chromium writes, the files
   resolved-styles reads, and an explicitly empty intersection. Consider a
   structural guard, such as a fixture directory that only resolved-styles
   reads.
6. **Make the Summary match the criteria** (addresses: "more than half"
   contradicts the targets)
   Either quote the local targets and the CI no-regression bar, or tighten the
   Playwright target.
7. **Record couplings with sibling items and other consumers** (addresses:
   coordination with 0361 siblings; Docker visual-regression lane; shared
   `integration.py`)
   List the 0361 children that change the shared load or the same files, and
   say that validation is re-run if one of them lands first. Name
   `test:e2e:visualiser:docker` and the visual-regression CI job as consumers
   of the changed config.
8. **Clarify the worker-count wording** (addresses: global and per-project
   worker counts)
   Say how the global cap relates to the per-project counts. Narrow the CI
   criterion to name the counts that must be host-relative: resolved-styles,
   the global cap, and xdist.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Mostly clear and well evidenced. The main problems are internal
inconsistencies: "more than half" against the 75 s and no-slower bars, and
"stays blocked" against "Blocked by: none". The relationship between the
global and per-project worker counts is ambiguous, and several terms are used
without definition.

**Strengths**: measured Context; exact settings and file locations; explicit
Drafting Notes; pure-delay waits clearly separated from readiness budgets.

**Findings**:
- 🟡 major / high, Summary: "Cuts each by more than half" contradicts the
  Playwright target and the CI criterion.
- 🟡 major / high, Assumptions: "Stays blocked" on split-out flakes
  contradicts "Blocked by: none".
- 🔵 minor / medium, Requirements: Unclear how the global and per-project
  worker counts combine, and which must be host-relative.
- 🔵 minor / medium, Requirements: Requirement 6 doesn't say whether
  production timeouts or test-only overrides shrink.
- 🔵 minor / medium, Context: Several terms are used without definition or
  link (A5/A6, circusd, arbiter, the fakes, warm workspace, full `mise run`
  load).
- 🔵 suggestion / medium, Dependencies: The 0368/0361 baseline note doesn't
  say why it bears on merging.
- 🔵 suggestion / low, Context: Ambiguous "that" in "Today that rests on a
  config comment".

### Completeness

**Summary**: A complete task. All sections are present and substantive, and
the frontmatter is valid. The only gap is the missing criteria for
Requirement 6.

**Strengths**: measured motivation; specific Requirements; nine criteria that
cover the scope; populated Open Questions, Assumptions and Dependencies;
explicit Drafting Notes; precise Technical Notes.

**Findings**:
- 🔵 minor / medium, Acceptance Criteria: No acceptance criterion for
  Requirement 6's shrunk waits or kept budgets.

### Dependency

**Summary**: Mostly self-contained, and it records the 0368 gate and the
Playwright version prerequisite. It doesn't capture the CI-history coupling,
the coupling with sibling items under 0361, the Docker visual-regression
consumer, or the conditional blocker for split-out flakes.

**Strengths**: the cleared 0368 gate is linked to its measurement; the
Playwright version prerequisite is recorded; the shared-state assumption is
explicit; the decision that `default` mirrors CI is stated.

**Findings**:
- 🟡 major / medium, Acceptance Criteria: The CI median criterion depends on
  CI history and on 10 future CI runs, and neither is captured.
- 🟡 major / medium, Dependencies: Coordination with sibling items under 0361
  isn't captured.
- 🔵 minor / medium, Requirements: The Docker visual-regression lane uses the
  same Playwright config but isn't named as a consumer.
- 🔵 minor / high, Assumptions: The conditional blocker for split-out flakes
  isn't reflected in Dependencies.
- 🔵 suggestion / low, Technical Notes: `tasks/test/integration.py` is also
  edited by sibling A4.

### Scope

**Summary**: Bounded and correctly parented. It groups A5 and A6 because they
share an approach, but they could land independently. Wait-shrinking and
open-ended flake fixing go beyond what the title promises.

**Strengths**: explicit out-of-scope notes; independence work limited to what
is needed; flake splitting acknowledged; correct parent link.

**Findings**:
- 🔵 minor / medium, Summary: Two suite changes that could ship independently
  are bundled into one item.
- 🔵 minor / medium, Requirements: The wait-shrinking requirement goes beyond
  the "parallelise" scope in the title and Summary.
- 🔵 minor / medium, Requirements: Fixing every exposed flake has no natural
  upper bound.
- 🔵 minor / medium, Acceptance Criteria: The CI median comparison after
  landing extends the item past merge.
- 🔵 suggestion / low, Frontmatter: kind: Kind `task` may understate the
  combined scope.

### Testability

**Summary**: Mostly concrete, with numeric thresholds and a bounded repeat
count. The criteria that rest on comparisons or concurrency are weak: there is
no reference host, no baseline for the timeout branches, a race-dependent
independence check, and a CI median with no tolerance.

**Strengths**: numeric thresholds with baselines; bounded flake validation; a
ban on suppressing tests; config criteria that can be checked directly; "no
slower" kept as the CI gate.

**Findings**:
- 🟡 major / high, Acceptance Criteria: Wall-time thresholds have no
  reference host.
- 🟡 major / high, Acceptance Criteria: The "timeout branch" criterion has no
  recorded baseline.
- 🟡 major / medium, Acceptance Criteria: The fixture-independence criterion
  relies on a nondeterministic pass and an audit with no defined form.
- 🔵 minor / medium, Acceptance Criteria: The CI median comparison has no
  noise tolerance and can only be checked after merge.
- 🔵 minor / medium, Requirements: Running the projects concurrently and
  keeping the readiness budgets have no matching criterion.
- 🔵 suggestion / low, Acceptance Criteria: The host and load for the 10-run
  flake validation are not specified.

## Re-Review (Pass 2) — 2026-11-15T13:09:38+00:00

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Testability**: Wall-time thresholds have no reference host — Resolved
- 🟡 **Testability**: "Timeout branch" criterion has no recorded baseline — Partially resolved (a baseline is now required, but "negative-path" and "timeout branch" are still undefined)
- 🟡 **Testability**: Fixture independence relies on a nondeterministic pass — Partially resolved (the audit now has a defined form, but it checks one direction only)
- 🟡 **Dependency / Testability / Scope**: CI median criterion — Partially resolved (baseline, tolerance and dependency are recorded; the CI job measured and the outcome if the check fails are not defined; a contaminated pre-change window can't be re-captured)
- 🟡 **Dependency**: Coordination with siblings under 0361 — Partially resolved (siblings are listed, but "rerun" can't recover a pre-change window, and 0369 was wrongly listed as a child)
- 🟡 **Clarity / Dependency**: "Stays blocked" contradicts "Blocked by: none" — Resolved
- 🟡 **Clarity**: "More than half" contradicts the targets — Resolved
- 🔵 **Completeness / Testability**: Requirement 6 has no criteria — Resolved
- 🔵 **Testability**: Projects running concurrently has no criterion — Resolved
- 🔵 **Scope**: Two suite changes bundled — Still present (author decision: keep one item, land per suite; scope re-raised it as major)
- 🔵 **Scope**: Wait shortening goes beyond the title's scope — Resolved (named in the Summary)
- 🔵 **Scope**: No upper bound on fixing flakes — Resolved (structural split-out rule)
- 🔵 **Dependency**: Docker visual-regression lane not named — Resolved
- 🔵 **Clarity**: Global and per-project worker counts — Partially resolved ("host-relative" plus "one above" a percentage is now ambiguous)
- 🔵 **Clarity**: Production versus test-only overrides — Resolved
- 🔵 **Clarity**: Undefined terms — Resolved
- 🔵 **Clarity**: 0368 note, ambiguous "that" — Partially resolved (the Blocked-by explanation still reads awkwardly)

### New Issues Introduced
- 🟡 **Testability**: Wall-time targets don't say how many runs to time or which clock to read
- 🟡 **Clarity**: Requirement 2's audit lists and "fix any spec that falls in both lists" don't match the criterion or say which side to fix
- 🟡 **Dependency**: A pre-change CI window can't be rerun once a sibling item has contaminated it; an ordering or freeze is needed
- 🔵 **Dependency**: 0369 is listed as a child of 0361 but is blocked by it (verified against 0361)
- 🔵 **Dependency**: The reference workstation and Docker are unnamed prerequisites; no measurement tool is named
- 🔵 **Testability**: No expected worker counts for 3–4 vCPU runners
- 🔵 **Clarity**: Requirement 6's "where" clauses could mean shorten only in those cases or shorten everywhere
- 🔵 **Clarity**: The "no slower" wording in the Summary and Open Questions differs from the 5% tolerance
- 🔵 **Scope**: `retries: 0` changes the retry policy across all of CI, beyond parallelisation
- 🔵 **Completeness**: No criterion that `pytest-xdist` is declared; no closure rule once only one suite has landed
- 🔵 Suggestions: check trace-on-failure by its effect; say which project owns the visual snapshots

### Assessment
The pass-1 structural issues are fixed: the reference host, the contradiction about blockers, the overclaim in the Summary, and the missing Requirement 6 criteria. The remaining majors are about precision in wording and measurement: how to time the suites, the fixture audit's lists, the global cap formula, definitions of the timeout branches, and how to sequence the CI window against siblings. Several were introduced by the pass-1 edits. The scope lens's bundling major is a deliberate author decision. One more revision should be enough for the item to be ready to plan.

## Re-Review (Pass 3) — 2026-11-15T14:41:43+00:00

**Verdict:** REVISE

Pass-2 revision note: the Docker visual-regression lane turned out to use its
own `playwright.docker.config.ts`, so the pass-1 premise that it shares the
main config was wrong. The Docker criterion and validation step were removed
and the lane was declared out of scope.

### Previously Identified Issues
- 🟡 **Testability**: Wall-time targets don't say how many runs or which clock — Resolved (`measure:build --isolate --runs 5` median); the cache state is not yet tied to it (see new issues)
- 🟡 **Clarity / Testability**: Fixture audit lists and conflict direction — Resolved; the read-discovery procedure is still self-reported (minor)
- 🟡 **Clarity**: Global cap is host-relative plus one — Resolved (formula with worked examples)
- 🟡 **Clarity**: "Timeout branch" and "negative-path" undefined — Resolved
- 🟡 **Dependency**: A contaminated pre-change window can't be rerun — Resolved (merge preference plus paired-comparison fallback); the fallback's pass rule is not stated (minor)
- 🟡 **Scope**: Two suites bundled — Downgraded to minor; author decision, tracked by Landing Status
- 🔵 **Dependency**: 0369 listed as a child — Resolved
- 🔵 **Dependency**: Workstation and Docker prerequisites, measurement tool — Resolved
- 🔵 **Testability**: Expected CI worker counts — Resolved; the "at least one" clamp is not exercised (minor)
- 🔵 **Clarity**: "No slower" versus 5%, "where" clauses, and which CI job — Resolved
- 🔵 **Scope**: Retry policy change goes beyond parallelism — Resolved as a stated co-goal; whether a revert undoes it is open (suggestion)
- 🔵 **Completeness**: xdist criterion, closure rule — Resolved

### New Issues Introduced
- 🟡 **Testability / Completeness**: No criterion verifies that flakes were fixed rather than masked per test (per-test retries, skip/fixme, rerun markers)
- 🟡 **Testability**: Nothing checks that happy-path dev tests avoid the timeout branches once the waits shrink
- 🔵 **Dependency**: `test-e2e` and `test-integration` run on both ubuntu and macOS (verified in `main.yml`); the comparison doesn't say whether each OS is compared separately
- 🔵 **Dependency**: Runner image or hardware changes as window contamination; the item's role in 0361's closing targets and in gating 0369
- 🔵 **Clarity**: "Warm workspace" defined but unused by the timing criteria; "`default` mirrors CI" reads as current fact; "retune" doesn't fit `-n auto` and revert versus Landing Status is unclear; `grace_kill` fixed sleep or upper bound; driver defaults versus waits in the fakes
- 🔵 **Testability**: N=1 clamp not covered; no way to inject the CPU count; the paired fallback has no statistic or threshold
- 🔵 Suggestions: where the negative-path record lives; which Playwright report proves overlap; repeat validation if a sibling changes `default`'s load; list `pytest-xdist` as a new package; kind `task` may understate the work; whether a revert covers `retries: 0`

### Assessment
Every pass-2 major is resolved, and three lenses (clarity, completeness, dependency) now report no majors. The verdict is still REVISE under the two-major threshold, but both remaining majors are narrow testability gaps: masking of flakes per test, and happy-path tests that could reach a timeout branch. Each is closed by adding one acceptance criterion. Once those and the quick clarity fixes are applied, the item should be ready to plan.

## Re-Review (Pass 4) — 2026-11-15T15:02:16+00:00

**Verdict:** REVISE

### Previously Identified Issues
- 🟡 **Testability / Completeness**: Flakes masked per test — Resolved (an anti-masking criterion lists the forbidden constructs)
- 🟡 **Testability**: Happy-path tests avoiding timeout branches — Partially resolved; the new criterion contradicts happy-path teardown (see new issues)
- 🔵 **Dependency**: ubuntu and macOS runners, runner image contamination, role in 0361 — Resolved
- 🔵 **Clarity**: Warm-workspace use, "`default` mirrors CI", retune and revert semantics, `grace_kill` nature, driver versus fakes — Resolved; the revert leaves "landed" ambiguous (minor)
- 🔵 **Testability**: N=1 clamp, injectable CPU count, paired fallback rule — Resolved
- 🔵 **Scope**: Bundling, retries co-goal, kind — Still present as minor or suggestion; author decisions, recorded in Drafting Notes

### New Issues Introduced
- 🟡 **Testability / Clarity**: In happy-path teardown, the probe waits out `probe_timeout` against an absent arbiter and reports it unreachable. That is a listed "timeout branch", so the "no recorded branch outputs on the happy path" criterion can't pass as written. Verified in `tasks/shared/dev/lifecycle.py` (`_probe_status` returns `None` on `SupervisorUnreachableError`).
- 🟡 **Clarity**: The conflict definition counts any file a `resolved-styles` spec writes, which rules out the prescribed fix of giving a spec its own fixture
- 🔵 **Testability**: 10-run validation pass and restart rules; no criterion that the config applies the derived counts; fixture audit method not reproducible or automated; 5% tolerance not backed by measured noise
- 🔵 **Clarity**: What a re-landing after a revert covers; CI window terms spread out ("20-run window" versus two 10-run windows); the settle-sleep criterion names the wrong files and omits "replaced"; loose "Playwright fractions", "0368 host settings" and "everywhere"
- 🔵 **Completeness**: Runner-image and rebase-repeat obligations have no criterion; the paired procedure lives in Dependencies
- 🔵 **Dependency**: 0370 alters engywook's host settings; the paired fallback's CI triggering is unstated; no `relates_to` frontmatter

### Assessment
The two pass-3 majors are addressed, but the happy-path criterion added for one of them contradicts how teardown really behaves, and the fixture-conflict definition is too broad. Both are one-sentence fixes, so the item is converging; dependency and completeness report no majors. The remaining minors are precision refinements.

## Verdict Change — 2026-11-15T15:09:21+00:00

**Verdict:** APPROVE

Pass-4 findings were addressed in the work item without a further review
pass. Both majors were fixed: the happy-path criterion no longer forbids the
probe timeout that normal teardown always hits, and the fixture-conflict
definition now exempts files only one spec touches. The minors were also
applied, apart from the bundling, the kind and the separate retries item,
which the author decided against. The author approved the work item for
planning.

