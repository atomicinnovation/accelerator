---
type: "work-item-review"
id: "0368-build-performance-measurement-and-host-guidance-review-1"
title: "Work Item Review: Build-performance measurement and host guidance"
date: "2026-10-29T16:53:32+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0368"
work_item_id: "0368"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 5
tags: ["build-system", "performance", "tooling", "documentation", "macos"]
last_updated: "2026-10-30T15:29:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Build-performance measurement and host guidance

**Verdict:** REVISE

The item is precise and well grounded. It names exact flags, paths and
record fields. It keeps measured claims apart from observed and
web-researched ones, and it records every interpretation in the Drafting
Notes. It falls short in four places:
- the baseline it exists to produce has no fixed reference host or host
  settings;
- several reported measures, and the host settings' "effect", have no
  operational definition;
- the cache-state modes have no criteria;
- its blocking edge is stated three different ways across this item, 0362
  and the five siblings that do not point back.

There are no critical findings. After merging duplicates across lenses,
ten major findings remain, well above the threshold of two.

### Cross-Cutting Themes

- **The baseline has no fixed anchor** (flagged by: dependency, scope,
  completeness, testability). Requirement 7 waits on a reference-host
  decision that the item leaves with 0361, where it is also open. 0361's own
  question covers "machine and conditions". Requirement 9 then leaves the
  host with both settings applied, so the epic's final 200 s comparison
  could credit code changes with savings that came from the host settings.
- **Measures lack operational definitions** (flagged by: testability,
  clarity, completeness, dependency). No definition is given for:
  - what a setting's "measured effect" is measured in;
  - the time window and process identity behind "cumulative CPU-seconds"
    and "the IDE";
  - whether `Documenting` units count.
  The counts matter because 0366's baseline of 2,015 units includes them.
- **The cache-state and isolation modes have no criteria, yet gate merges**
  (flagged by: completeness, testability, scope, clarity). Requirement 4
  has no acceptance criterion, including for the destructive cold mode. It
  does not say how cache states interact with the priming run. It still
  sits inside the Requirements 1–7 range that gates six siblings.
- **The blocking edge is inconsistent across items** (flagged by:
  dependency, clarity, scope). This item gates on Requirements 1–7. 0362
  cites "Requirements 1–2". Neither matches what the baseline needs.
  0363–0367 do not carry the reverse edge at all.

### Findings

#### Major

- 🟡 **Dependency**: Blocks edges to 0363–0367 are not reciprocated
  **Location**: Dependencies
  `blocks` lists 0362–0367, but only 0362 has `blocked_by:
  ["work-item:0368"]`. 0363, 0364, 0365 and 0367 say "Blocked by: none", and
  0366 lists only 0362, so nothing in those items stops a merge before the
  baseline exists.

- 🟡 **Dependency + Completeness + Scope + Testability**: Reference-host
  decision is an unrecorded upstream blocker
  **Location**: Open Questions / Dependencies
  Requirement 7 cannot be discharged until 0361 picks a reference host, yet
  Dependencies says "Blocked by: none". The question has no default and no
  owner, and acceptance criterion 7 is met by a baseline from any host. 0362
  is already `ready`, so this decision sits on the epic's critical path.

- 🟡 **Dependency**: Requirement 9 changes the reference host's state
  **Location**: Requirements: 9
  The neither → exemption → both sequence leaves the host with both settings
  applied, and undoing the exemption can need a reboot. Nothing ties the
  baseline, or 0361's final run, to a host-settings condition. The epic's
  comparison can therefore absorb the settings' effect.

- 🟡 **Testability + Completeness + Clarity**: The host settings'
  "measured effect" has no defined metric or controls
  **Location**: Requirements: 9; Acceptance Criteria 8
  No metric is named: it could be median wall-clock, `syspolicyd` and
  `XprotectService` CPU-seconds, or IDE CPU-seconds. Requirement 2 gives
  medians only for durations, and nothing requires the three condition
  records to share a commit and cache state or to use fixed labels. In the
  cumulative design, the IDE exclusion's figure is its effect on top of the
  exemption, which "each setting's measured effect" does not convey.

- 🟡 **Testability + Clarity**: Reported measures lack defined windows and
  process identity
  **Location**: Requirements: 1; Acceptance Criteria 1 and 5
  "Cumulative CPU-seconds" could be read as the total since the process
  started, rather than the increase during the run summed across PIDs that
  appear during it. "The IDE" matches no stated process names, and it is
  unclear whether `fsnotifier` counts. "30-second profile" and "idle CPU
  busy before the first run" each support two readings.

- 🟡 **Testability + Dependency**: Unit counts omit `Documenting`, so they
  will not match 0366's 2,015-unit baseline
  **Location**: Requirements: 1
  The research's 851 non-`Compiling` units are "Checking or Documenting",
  and `analyse.py` counts `Documenting` among the checks. If the harness
  follows the item's wording, 0366 could pass through a change of
  definition alone.

- 🟡 **Completeness + Testability + Scope**: Cache states, including the
  destructive cold mode, have no acceptance criteria
  **Location**: Acceptance Criteria; Requirements: 4
  No criterion exercises the incremental touch or the cold deletion. None
  checks that nothing is deleted without the flag, and "every
  `node_modules`" gives no list to check against.

- 🟡 **Clarity**: The item does not say how repeat runs, cache states and
  isolation mode combine
  **Location**: Requirements: 2, 3 and 4
  The item does not say whether the touch or deletion happens once, before
  the priming run, or before each timed run. On one reading, a cold run is
  primed back to warm and then times warm no-op runs. It is also unclear
  whether isolation mode primes and repeats.

- 🟡 **Testability**: The record criterion checks that fields are present,
  not that their values are correct
  **Location**: Acceptance Criteria 1
  A record whose lock waits are always 0, or whose queueing delay starts
  from the wrong reference point, would pass. The Technical Notes promise
  fixture-tested analysis, but no criterion states the expected outputs for
  a known input.

- 🟡 **Scope**: The critical-path harness is bundled with non-blocking host
  guidance
  **Location**: Dependencies
  Requirements 1–7 gate six siblings, while Requirements 8–9 block nothing
  and need about 12 full builds plus a host change that can need a reboot to
  undo. Because the item's status cannot show when the block lifts, readers
  have to parse requirement ranges in prose.

#### Minor

- 🔵 **Dependency + Clarity + Scope**: The condition that lifts the block
  is stated three ways
  **Location**: Dependencies
  The item gives "Requirements 1–7", then "discharged once the baseline is
  recorded", while 0362 cites "Requirements 1–2". The warm baseline needs
  only Requirements 1, 2, 5 and 7, not isolation mode, the cache states or
  the macOS refusal.

- 🔵 **Dependency + Testability + Clarity**: The named consumers' needs do
  not match what the harness produces
  **Location**: Dependencies: Consumers
  0365's criterion measures the *nextest phase* of `test:unit:cli`, but
  isolation mode reports only each task's duration and exit status. Nothing
  says whether `--runs` applies in isolation mode, or whose isolated
  baseline 0365 compares against.

- 🔵 **Completeness + Scope**: Two acceptance criteria require things the
  Requirements never state
  **Location**: Requirements: 8; Assumptions
  Acceptance criterion 8 requires the security tradeoff and the
  file-watching caveat, and criterion 9 requires the closure exclusion.
  These appear only in the criteria, the Assumptions and the Technical
  Notes.

- 🔵 **Dependency**: The host-setting figures depend on 0362, 0363 and 0367
  landing, with no ordering stated
  **Location**: Requirements: 9
  0363 removes the relinking of 369 binaries that drives `syspolicyd`, and
  0367 stops the `npm ci` churn that feeds the indexer. Figures taken
  before those land would overstate the settings' benefit.

- 🔵 **Dependency**: The build's live network inputs are not named as
  external dependencies
  **Location**: Requirements
  The measured build depends on the GitHub release download (until 0362
  lands), the RustSec advisory database, and the npm registry in cold mode.
  A slow or offline fetch would skew the baseline without the record
  showing why.

- 🔵 **Dependency**: 0369 changes what `default` runs but is not listed as
  related
  **Location**: Dependencies
  0369 adds lanes to `default`, such as `lint:workflows:check`, and
  reorders tasks behind `format:fix`. If it lands between the baseline and
  0361's final run, that comparison mixes in work 0369 adds.

- 🔵 **Clarity**: It is unclear who commits records and which records are
  committed
  **Location**: Requirements: 5
  It could be the task or the operator who commits, and "each record" could
  mean every run or only the baseline and condition records.

- 🔵 **Testability**: The failed-task criterion has a trigger that cannot
  be reproduced and an unbounded "every other task"
  **Location**: Acceptance Criteria 2
  The trigger is a live advisory, and tasks that depend on the failed one
  may never run. The harness's own exit status and the handling of
  partially failed runs in the median are not stated.

- 🔵 **Testability**: The priming run and one-at-a-time isolation cannot
  be seen in the record
  **Location**: Acceptance Criteria 3 and 4
  The record holds neither the priming run nor start and end times for each
  isolated task. The default of three runs is never exercised, because
  criterion 3 passes `--runs 3`.

#### Suggestions

- 🔵 **Testability + Completeness**: The gitignored raw-log location has
  no criterion
  **Location**: Requirements: 5
  A harness that writes `run.log` into a tracked path would pass every
  criterion.

- 🔵 **Testability**: There is no measured noise floor
  **Location**: Requirements: 2
  The research measured about ±15% variance between runs, while savings
  such as 0367's B4 are about 5 s. Reporting a range beside each median
  would show which savings are distinguishable from noise.

- 🔵 **Clarity**: "Needs a quiet host" contradicts the decision to record
  idle load
  **Location**: Assumptions
  The Assumptions require a quiet host, while the Drafting Notes
  deliberately accept a busy one. A reason that holds either way is that
  `measure:build` itself runs `default`.

- 🔵 **Clarity**: Several terms are used without definition
  **Location**: Context; Requirements
  The undefined terms are "queueing delay", "CPU busy", "warm no-op",
  "Developer Tools", "responsible app" and `fsnotifier`.

- 🔵 **Clarity**: The Summary states the settings' benefit as fact
  **Location**: Summary
  "Gives contributors a faster local build" is what Requirement 9 exists to
  measure.

### Strengths

- ✅ Requirements and criteria name exact flags, paths, fields and
  enforcement points, so most statements have only one reading.
- ✅ Context keeps the research's measurements apart from what it observed
  but did not separate by experiment, and from later web research.
- ✅ The Drafting Notes record every interpretation and its source,
  including the cumulative three-condition design and what it leaves
  unisolated.
- ✅ The explicit out-of-scope list keeps host-dependent thresholds out of
  the criteria, so no pass/fail bar depends on the machine.
- ✅ Dependencies says the edge gates merging only and separates the
  harness requirements from the guidance that blocks nothing.
- ✅ The closure criterion delegates to the existing `test_mise.py` guard,
  and the baseline-timing criterion is a fact about commit ancestry, so
  both can be checked mechanically.
- ✅ The Technical Notes give concrete implementation seams: replacements
  for `ts`, `gdate` and `bc`, analysis tested against fixtures, the
  lower-bound caveat, and the order in which to measure the conditions.

### Recommended Changes

1. **Fix the blocking edge at both ends** (addresses: blocks edges not
   reciprocated; the condition that lifts the block is stated three ways;
   consumer-specific modes in the gate)
   Narrow the gate to Requirements 1, 2, 5 and 7, the warm full-run path
   plus its baseline, and state one condition that discharges it. Add
   `blocked_by: ["work-item:0368"]` to 0363–0367 with 0362's merge-only
   wording, and correct 0362's "Requirements 1–2".
2. **Anchor the baseline** (addresses: reference-host blocker; Requirement
   9 changes host state; the baseline criterion accepts any host)
   Either propose a default reference host here or record a Blocked-by on
   0361's question. Fix the host-settings condition the baseline uses: for
   example, the "neither" condition at the same commit doubles as the
   Requirement 7 baseline. Require 0361's final run to use the same
   condition, and bind acceptance criterion 7 to that host.
3. **Decide whether to split the host guidance out** (addresses: bundling)
   Moving Requirements 8–9 into a sibling under 0361, blocked by this item,
   lets this item close when the baseline lands. It would also absorb the
   ordering of Requirement 9 against 0362, 0363 and 0367.
4. **Define each measure** (addresses: windows and process identity;
   `Documenting` units)
   - **Host-process CPU:** the increase during the run, summed across PIDs,
     over a stated window.
   - **The IDE:** a named set of process names, with or without
     `fsnotifier`.
   - **The CPU profile:** 30-second buckets across the whole run.
   - **Idle CPU:** a defined pre-run window.
   - **Unit counts:** `Documenting` units counted, so totals compare with
     2,015.
5. **Define the host settings' effect** (addresses: host-settings metric)
   Use fixed condition labels and require the three records to share a
   commit, cache state and `--runs 3`. Define the effect as the change in
   median wall-clock and in each tracked process's CPU-seconds from the
   previous condition. State that the IDE figure is incremental on the
   exemption.
6. **Specify and test the cache-state modes** (addresses: cache states
   have no criteria; how runs, states and modes combine; raw-log location;
   priming visibility)
   Say when the touch or deletion happens relative to the priming run and
   each timed run, and whether isolation mode primes and repeats. Then add
   criteria for:
   - an incremental run;
   - a cold run with the flag, and a run without it;
   - an enumerated `node_modules` list;
   - a gitignored log directory;
   - a priming run kept in the record and marked as discarded.
7. **Add criteria with known answers for the analysis** (addresses: the
   record criterion checks presence only)
   Use fixture logs with stated lock-wait, queueing-delay and per-verb
   counts.
8. **Move the criteria-only commitments into Requirements** (addresses:
   two criteria require things the Requirements never state; the quiet-host
   contradiction)
   Add the security tradeoff and the file-watching caveat under Requirement
   8. Add a Requirement that keeps `measure:build` out of `check` and
   `default`, justified by the fact that it runs `default` itself.
9. **Match the consumers' needs, or drop the claims** (addresses: consumer
   mismatch)
   Either report the nextest phase and support `--runs` in isolation mode,
   or say that 0365 measures the phase itself.
10. **Tidy-ups** (addresses: remaining minors and suggestions)
    - Relate 0369.
    - Name the network inputs, and retake a baseline after a failure the
      network caused.
    - Say who commits records.
    - Make the failed-task trigger deterministic.
    - Report a range beside each median.
    - Define the terms.
    - Hedge the Summary.

## Per-Lens Results

### Clarity

**Summary**: From a clarity standpoint, 0368 is unusually precise.
Requirements and criteria name exact flags, paths, record fields and
enforcement points, Context keeps measured, observed-but-unisolated and
web-research claims apart, and the Drafting Notes say where the author
interpreted. The main gaps are how repeat runs, cache states and isolation
mode combine; undefined time windows and process identity for several
reported measures; no named metric behind the host settings' "measured
effect"; and a blocking-discharge condition stated two ways here and
numbered differently in 0362.

**Strengths**:
- Requirements and acceptance criteria name concrete flags, paths, fields
  and enforcement points, so most statements have only one reading.
- Context separates what the research measured, what it observed but did
  not isolate, and what later web research added.
- The Drafting Notes name each interpretation and where it came from.
- The out-of-scope list is explicit and agrees with Requirement 6 and the
  Assumptions.
- Dependencies says the edge gates merging only and keeps the blocking
  harness requirements apart from the host guidance.

**Findings**:
- 🟡 major (high) — Requirements: 2, 3 and 4 — **Repeat runs, cache states
  and isolation mode do not say how they combine.** Nothing says whether
  the touch or delete happens once, before the priming run, or before each
  timed run; isolation mode is equally silent on priming and repeats. Two
  reasonable implementations would produce cold and incremental records
  that measure different things, and 0366 relies on the cold counts.
- 🔵 minor (high) — Requirements: 1 — **"The IDE" has no rule for
  identifying it.** Context names IntelliJ, then RustRover, and the
  Assumptions widen to "JetBrains IDEs"; it is unstated whether helpers such
  as `fsnotifier` count. That changes the figure Requirement 9 needs.
- 🔵 minor (high) — Requirements: 1; Acceptance Criteria — **The time
  windows of three reported measures are ambiguous.** These are the
  30-second profile (a single window or buckets across the run), the
  cumulative CPU-seconds (since process start or during the run, and
  whether the post-build tail counts) and the idle CPU (before the priming
  run or the first timed run).
- 🔵 minor (medium) — Requirements: 9; Acceptance Criteria 8 — **The
  metric behind each host setting's "measured effect" is not named.** The
  cumulative design means the IDE exclusion's figure is its effect on top
  of the exemption, yet "each setting's measured effect" reads as two
  independent effects.
- 🔵 minor (high) — Dependencies — **The condition that lifts the block is
  stated two ways, and 0362 cites different requirement numbers.** Here it
  is "through Requirements 1–7" and also "discharged once the baseline is
  recorded"; 0362 cites "Requirements 1–2".
- 🔵 minor (medium) — Requirements: 5 — **It is unclear who commits
  records and which records are committed.** It could be the task or the
  operator, and every run or only the baseline and condition records.
- 🔵 suggestion (medium) — Assumptions — **"Needs a quiet host"
  contradicts the decision to record idle load instead of refusing a busy
  host.** Reconcile the two, and justify keeping the task out of `default`
  by the fact that it runs `default` itself.
- 🔵 suggestion (medium) — Dependencies: Consumers — **It is unclear which
  baseline 0365 compares its isolated run against.** The only baseline
  recorded is the full-run warm no-op one.
- 🔵 suggestion (medium) — Context; Requirements — **Several terms are used
  without definition**: queueing delay, CPU busy, warm no-op, Developer
  Tools, responsible app and `fsnotifier`.
- 🔵 suggestion (low) — Summary — **The Summary states as fact a benefit
  the item has yet to measure.** Present it as expected, and say what the
  guidance does if the effect is negligible.

### Completeness

**Summary**: 0368 is a structurally complete task. Every expected section is
present and has real content, the frontmatter is intact, and Context
explains why both the harness and the host guidance are needed. The main gap
is that no acceptance criterion covers Requirement 4's incremental or cold
cache states. Three smaller gaps: two criteria require things the
Requirements never state, the host-setting comparison metric is not named,
and the only Open Question has no default.

**Strengths**:
- Every section a task needs is present and substantive, with an explicit
  out-of-scope list.
- The frontmatter is complete and consistent with the body.
- Context sets out the forces behind both halves of the work and says
  plainly that the host effects were not separated by experiment.
- Dependencies names which requirements carry the blocking edge, when it is
  discharged, and which consumers rely on which requirement.
- The Assumptions and Drafting Notes record each interpretation and its
  source.
- The Technical Notes give an implementer concrete starting points.

**Findings**:
- 🟡 major (high) — Acceptance Criteria — **The cache-state modes and
  raw-log handling have no acceptance criteria.** Nothing defines done for
  the incremental or cold modes, including that a cold run deletes only
  with the flag, or for raw logs landing in a gitignored directory.
- 🔵 minor (high) — Requirements — **Two acceptance criteria require things
  the Requirements never state.** These are the security tradeoff and
  file-watching caveat (criterion 8), and the closure exclusion (criterion
  9), which appears only as an Assumption.
- 🔵 minor (medium) — Requirements — **The metric for comparing the
  host-setting conditions is not named.** Requirement 2's medians cover
  only durations, so the per-process CPU figures that the C2 row targets
  would not be summarised.
- 🔵 minor (medium) — Open Questions — **The baseline-host Open Question has
  no default or decision owner.** Propose a default, for example the
  research workstation, or require it settled before the item moves to
  ready.

### Dependency

**Summary**: 0368 sets merge-only blocking edges to all six siblings, says
how they are discharged, and names its consumers. The capture is one-sided:
only 0362 points back. The reference-host decision that Requirement 7
waits on appears only as an open question, not as a blocker. "The host
guidance blocks nothing" misses that Requirement 9 changes the state of the
reference host, and that sibling items change the effect Requirement 9
measures.

**Strengths**:
- Dependencies says exactly what the blocking edge means, and 0362 uses
  matching discharge wording.
- The frontmatter `blocks` list covers every sibling, and acceptance
  criterion 7 makes the ordering checkable by commit ancestry.
- The item names its couplings to existing artefacts: the `measure:*`
  convention, the `test_mise.py` guard and the 0189 precedent.
- Open Questions makes the reference-host gap visible.
- The order of the conditions within Requirement 9 is written down.

**Findings**:
- 🟡 major (high) — Dependencies — **The blocks edges to 0363–0367 are not
  reciprocated.** If any of them merges first, Requirement 7 cannot be met
  and 0361's 200 s target loses its pre-change baseline.
- 🟡 major (high) — Open Questions — **The reference-host decision is an
  unrecorded upstream blocker on the critical path.** 0361's question
  covers "machine and conditions", 0362 is already `ready`, and the
  decision has no owner or deadline. Add a Blocked-by entry scoped to
  Requirement 7.
- 🟡 major (medium) — Requirements — **Requirement 9 alters the reference
  host's state.** The host ends with both settings applied, and 0361
  compares "on the same machine" but not under the same settings. Take the
  "neither" condition as the baseline and require 0361's final run to
  match.
- 🔵 minor (high) — Dependencies — **0368 and 0362 disagree on which
  requirements gate the edge.** Neither "1–7" nor "1–2" matches what the
  baseline needs, which is Requirements 1, 2, 5 and 7.
- 🔵 minor (medium) — Dependencies — **The named consumers' needs do not
  match what the harness commits to produce.** 0365 needs the nextest
  phase, and 0366's 2,015 includes `Documenting` units. Nothing requires
  isolated or cold baselines before those items change anything.
- 🔵 minor (medium) — Requirements — **The measured build's live network
  inputs are not named.** These are GitHub releases, the advisory database
  and the npm registry. Retake a baseline after a failure the network
  caused rather than committing it.
- 🔵 minor (medium) — Requirements — **The host-setting figures depend on
  0362, 0363 and 0367 landing, with no ordering stated.** Either measure
  after they land, or name the commit the figures apply to and measure
  again later.
- 🔵 minor (medium) — Dependencies — **0369 changes what `default` runs but
  is not related to this item.** Say whether the baseline must come before
  or after 0369, and record that 0369 must keep `measure:build` out of
  `default`.

### Scope

**Summary**: 0368 is a well-bounded task within one ownership domain, with
an explicit out-of-scope line and recorded reasons for each scope choice.
Its main weakness is that it bundles the epic's critical-path deliverable
with host guidance and a three-condition experiment that block nothing. It
also puts consumer-specific harness modes inside the gating edge, so its
status cannot show when the block lifts. Smaller issues: the Summary,
Requirements and Acceptance Criteria do not quite describe the same
boundary, and the reference-host decision belongs to another item.

**Strengths**:
- The explicit out-of-scope line makes the boundary easy to state.
- Dependencies separates the gating requirements from those that block
  nothing.
- The Drafting Notes record why each scope choice was made, including the
  cap of about 9 timed runs.
- All the work sits in one ownership domain.
- Each consumer and the requirement it relies on can be traced.

**Findings**:
- 🟡 major (medium) — Dependencies — **The critical-path harness is bundled
  with non-blocking host guidance.** The coupling runs one way only. Split
  Requirements 8–9 into a sibling that this item blocks, so that the item
  closes when the baseline lands.
- 🔵 minor (medium) — Requirements — **Consumer-specific harness modes sit
  inside the merge-gating edge.** Isolation mode serves 0365, and the cache
  states serve 0366, which compares against the research's figure rather
  than a pre-change baseline. Narrow the gate to Requirements 1, 2 and 5–7.
- 🔵 minor (medium) — Open Questions — **The baseline's reference-host
  decision belongs to another item.** Bring it into scope, or make its
  resolution a precondition for ready, and say whether Requirement 9 must
  use the same host.
- 🔵 suggestion (medium) — Acceptance Criteria — **The exclusion from
  `check` and `default` is checked but stated only as an assumption.** Add
  a requirement or an out-of-scope bullet.

### Testability

**Summary**: Most of 0368's acceptance criteria are well-framed,
observable Given/When/Then checks, and the item sensibly avoids gating on
host-dependent figures. The main gaps:
- the criteria check that fields are present, not that their values are
  correct;
- the cache states have no criterion;
- the host settings' "measured effect" has no metric or controls;
- the harness's figures are not tied to what 0365 and 0366 need to verify
  their own savings.

**Strengths**:
- The criteria consistently name a precondition, an action and an
  observable outcome.
- The closure criterion delegates to an existing mechanical guard.
- The non-macOS criterion is a well-formed negative case with three
  observable outcomes.
- The baseline-timing criterion is a commit-ancestry fact that can be
  checked after the event.
- The out-of-scope statement keeps flaky, machine-specific thresholds out.
- Analysis tested against fixtures gives a seam for deterministic
  verification.
- The documentation criterion turns guidance into concrete content claims.

**Findings**:
- 🟡 major (high) — Acceptance Criteria 1 — **The record criterion checks
  field presence, not value correctness.** Add fixture criteria with known
  answers for lock wait, queueing delay and unit counts.
- 🟡 major (medium) — Requirements: 1 — **The host-process CPU-seconds
  have no defined window or process identity.** Define the figure as the
  per-run increase across PIDs, name the IDE process names, and record an
  absent value, not 0, when no IDE runs.
- 🟡 major (high) — Requirements: 4 — **The cache states, including the
  destructive cold mode, have no acceptance criterion.** Add criteria for
  runs with and without the flag and for the incremental touch, and
  enumerate the `node_modules` directories.
- 🟡 major (high) — Acceptance Criteria 8 / Requirements: 9 — **The host
  settings' "measured effect" has no defined metric, and its controls are
  unverified.** Fix the label set, require the records to share a commit
  and cache state, and define the effect as the change from the previous
  condition.
- 🟡 major (medium) — Requirements: 1 — **The unit counts omit
  `Documenting`, so they will not match the 2,015-unit baseline 0366
  uses.** Count `Documenting` units and add a fixture with all three verbs.
- 🔵 minor (medium) — Dependencies — **Isolation-mode output does not give
  consumers the figures their criteria compare.** Support `--runs` in
  isolation mode and either report the nextest phase or note that 0365
  measures it.
- 🔵 minor (medium) — Acceptance Criteria 5 and 7 — **The baseline
  criterion can be met by a baseline that cannot anchor the epic.** Bind it
  to 0361's reference host, and record whether the working copy matched the
  recorded commit.
- 🔵 minor (medium) — Acceptance Criteria 2 — **The failed-task criterion
  has a trigger that cannot be reproduced and an unbounded "every other
  task".** Use an injected failure, report started and skipped tasks
  separately, and state the harness's exit status.
- 🔵 minor (medium) — Acceptance Criteria 3 and 4 — **The priming run and
  one-at-a-time isolation cannot be seen in the record.** Keep the priming
  run's wall-clock, marked as discarded, record per-task timestamps in
  isolation mode, and test the default of three runs.
- 🔵 suggestion (high) — Requirements: 5 — **The gitignored raw-log
  location has no criterion.** Check it with `git check-ignore`.
- 🔵 suggestion (medium) — Requirements: 2 — **There is no measured noise
  floor, so savings cannot be told apart from run-to-run variance.**
  Report a range beside each median and the idle CPU before each run.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-10-30T01:33:15+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Dependency**: Blocks edges to 0363–0367 not reciprocated — Resolved
- 🟡 **Dependency + Completeness + Scope + Testability**: Reference-host
  decision is an unrecorded blocker — Resolved
- 🟡 **Dependency**: Requirement 9 changes the reference host's state —
  Resolved
- 🟡 **Testability + Completeness + Clarity**: Host settings' effect has no
  metric or controls — Resolved; the noise rule's wording is a new minor
- 🟡 **Testability + Clarity**: Measures lack windows and process identity
  — Partially resolved: duration, a dependency's end and a new PID's
  starting value are still undefined
- 🟡 **Testability + Dependency**: Unit counts omit `Documenting` —
  Resolved
- 🟡 **Completeness + Testability + Scope**: Cache states have no criteria
  — Partially resolved: the no-cold-flag criterion cannot fail, and
  "before each timed run" cannot be observed
- 🟡 **Clarity**: How repeat runs, cache states and isolation mode combine
  — Partially resolved: what isolation mode primes with, and in what order
  it repeats, is unstated
- 🟡 **Testability**: Record criterion checks presence only — Partially
  resolved: the fixture criterion omits duration, buckets, idle load and
  the median, min and max
- 🟡 **Scope**: Harness bundled with host guidance — Still present;
  accepted by decision, with the justification in Drafting Notes
- 🔵 **Dependency + Clarity + Scope**: The condition that lifts the block
  is stated three ways — Resolved
- 🔵 **Dependency + Testability + Clarity**: Consumers' needs do not match
  what the harness produces — Partially resolved: an isolated nextest-phase
  median is promised but not specified, and the edges to 0365 and 0366 are
  one-sided
- 🔵 **Completeness + Scope**: Two criteria require things the
  Requirements never state — Resolved
- 🔵 **Dependency**: Host-setting figures depend on 0362, 0363 and 0367
  landing — Resolved in 0368, but not recorded on those three items
- 🔵 **Dependency**: Network inputs not named — Resolved; `npm audit` in
  warm runs is a new minor
- 🔵 **Dependency**: 0369 not listed as related — Partially resolved: no
  ordering against 0361's closing comparison, and 0369 does not know
- 🔵 **Clarity**: Who commits records — Resolved; "keep" is overloaded
- 🔵 **Testability**: Failed-task trigger cannot be reproduced — Partially
  resolved: no injection mechanism, and no partial-failure case
- 🔵 **Testability**: Priming run and one-at-a-time isolation cannot be
  seen in the record — Resolved
- 🔵 Suggestions (raw logs, noise floor, quiet host, terms, Summary) —
  Resolved

### New Issues Introduced

- 🔴 **Dependency + Completeness + Clarity + Testability**: The retake
  rule conflicts with the known `deny:check` failure — Requirement 7's "a
  task failed or stalled on a network fetch" can be read as any failure,
  and every run today fails `deny:check` on RUSTSEC-2026-0414, so no
  baseline could be kept. Decision 8 meant only failures caused by the
  network, but the item does not exempt the advisory failure.
- 🟡 **Clarity + Testability**: An isolated nextest-phase median is
  promised but not specified — Requirements 2 and 3 never give the nextest
  phase a median, yet 0365's criterion compares one.
- 🟡 **Testability**: The no-cold-flag criterion cannot fail — the build
  recreates `cli/target`, and `npm ci` replaces `node_modules` on every
  run, so a harness that deletes them wrongly still passes.
- 🟡 **Testability + Clarity**: Known-answer coverage omits headline
  figures — the fixture criterion omits duration, 30-second buckets, idle
  load and the median, min and max. The derivation of duration (from
  mise's `Finished in`, or from first to last line) is unstated.
- 🟡 **Dependency**: 0365's need for isolation mode is one-sided — 0365
  drops its edge once the baseline is recorded, but its criterion needs
  Requirement 3 merged before its before-change measurement.
- 🟡 **Dependency + Clarity**: No ordering between 0369 and 0361's closing
  comparison — re-measuring the baseline commit cannot add 0369's tasks,
  and 0369 does not mention 0368.
- 🔵 **Clarity + Testability**: The noise rule compares a change against
  an absolute range, so two readings are possible.
- 🔵 **Clarity + Dependency**: The comparison rule is stated differently
  across sections. "Host model" also differs from one named machine, and a
  macOS update between records is not covered.
- 🔵 **Clarity**: "The commit" is ambiguous in a jj workspace once a kept
  record joins the working-copy commit.
- 🔵 **Clarity**: The lower-bound claim conflicts with back-to-back runs;
  each run's tail falls inside the next run's window.
- 🔵 **Clarity**: The exit-status sentence says "the task" for the
  harness, and "cannot measure" does not list the refusals.
- 🔵 **Clarity**: "Run" and "keep" each carry several meanings.
- 🔵 **Testability**: The partial-failure case and the injection mechanism
  are unspecified.
- 🔵 **Testability + Completeness**: The IDE match, absent-not-0, the
  `unspecified` label default and per-process statistics have no
  criterion.
- 🔵 **Testability**: Preparation "before each timed run" cannot be
  observed after the run.
- 🔵 **Completeness**: Context omits why the scripts must move in-repo and
  the ±15% variance that justifies the repeat-run rules.
- 🔵 **Dependency**: 0366 does not use cold mode or `measure:build`, so
  the consumer claim is one-sided.
- 🔵 **Dependency**: Requirement 9's wait is not recorded on 0362, 0363
  and 0367.
- 🔵 **Dependency**: 0367's freshness guards would change what the cold
  state does.
- 🔵 **Dependency**: `npm audit` makes the npm registry an input to warm
  runs too.
- 🔵 **Scope**: Record retention reaches into records other items produce.
- 🔵 **Scope**: Isolation mode and the cache states keep the item open
  after the baseline (follows from the bundling decision).
- 🔵 Suggestions: the incremental state has no named consumer (scope);
  nothing says what happens to 0368's `blocks` list on discharge
  (dependency); C1, C2, `ts`, `gdate` and `bc` are unexplained (clarity).

### Assessment

The first pass resolved 9 of its 10 major themes in full or in part. The
one critical is a regression from Decision 8's wording, fixed by exempting
known pre-existing failures such as `deny:check` from the retake rule.
Three of the new majors are mechanical fixes:
- specify the isolated nextest-phase median;
- use a sentinel to show nothing was deleted without the cold flag;
- extend the fixture criterion.
Two need decisions: how 0365 tracks its need for isolation mode, and how
0369 is ordered against 0361's closing comparison. The minors are mostly
second-order precision; the item is close to ready once the critical and
majors are addressed.

## Re-Review (Pass 3) — 2026-10-30T01:56:39+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🔴 **Dependency + Completeness + Clarity + Testability**: The retake
  rule conflicts with the known `deny:check` failure — Resolved; the
  fetch-failure evidence it now relies on is a new major
- 🟡 **Clarity + Testability**: No isolated nextest-phase median —
  Resolved; the `--runs 3` criterion still omits it (minor)
- 🟡 **Testability**: No-cold-flag criterion cannot fail — Resolved by
  the sentinel, but its rationale wrongly assumes `npm ci` runs every time
- 🟡 **Testability + Clarity**: Known-answer coverage omits headline
  figures — Mostly resolved; bucket membership, iostat's first report, a
  task's start and an open lock wait without `Finished in` remain (minor)
- 🟡 **Dependency**: 0365's need for isolation mode one-sided — Resolved;
  the Blocks bullet's general rule still contradicts the carve-out (minor)
- 🟡 **Dependency + Clarity**: No ordering between 0369 and 0361 —
  Resolved
- 🟡 **Scope**: Harness bundled with host guidance — Still present, now
  framed as a dependency cycle with 0362, 0363 and 0367; accepted by
  decision
- 🔵 **Clarity + Testability**: Noise rule compares a change against a
  range — Still present; raised to major
- 🔵 **Clarity + Dependency**: Comparison rule stated differently across
  sections — Still present; raised to major
- 🔵 **Dependency**: 0366 does not use cold mode — Still present; raised
  to major
- 🔵 **Dependency**: 0367's freshness guards change the cold state —
  Still present; raised to major
- 🔵 **Testability**: Partial failure and injection mechanism — Still
  present; partial failure folds into a new major
- 🔵 **Testability + Completeness**: IDE match, absent-not-0, label
  default have no criterion — Still present
- 🔵 **Testability**: Preparation before each timed run cannot be
  observed — Still present
- 🔵 **Clarity**: Exit-status sentence calls the harness "the task" —
  Still present
- 🔵 **Clarity**: "Keep" carries several meanings — Still present
- 🔵 **Clarity**: Isolation-mode priming structure — Still present
- 🔵 **Dependency**: Requirement 9's wait is not recorded on 0362, 0363
  and 0367 — Still present
- 🔵 **Dependency + Clarity**: Reference host and its macOS version are
  not a listed dependency — Still present
- 🔵 Not raised again: "the commit" in jj, the lower-bound spillover, the
  Context rationale, `npm audit`, record retention, `blocks` pruning

### New Issues Introduced

- 🟡 **Testability**: Fetch failures cannot be checked from the committed
  record — the criteria check gitignored raw logs, and the item defines no
  pattern that counts as a failed fetch.
- 🟡 **Testability + Completeness**: The `--runs 3` criterion cannot pass
  today — it asks for a median of every task, while `deny:check` fails in
  every run and failed tasks get none. The wall-clock median over runs
  with a failure, and the partial-failure case, are untested.
- 🟡 **Testability**: The baseline's host-settings label is unpinned — a
  baseline labelled `unspecified` passes, leaving 0361 no settings to
  reproduce.
- 🔵 **Clarity**: The cache states "may land after the baseline", but the
  baseline uses the warm no-op state from Requirement 4.
- 🔵 **Clarity**: Whether a failed task's min and max are withheld is
  unstated.
- 🔵 **Completeness**: The remaining operator controls (cold flag,
  isolation invocation, incremental selection, IDE override) are unnamed.
- 🔵 **Scope**: The gating subset includes host-process attribution that
  no gated sibling needs.
- 🔵 **Dependency**: The GitHub-releases coupling outlives 0362 whenever
  the baseline commit is re-measured.
- 🔵 Suggestions: no home named for the `measure:build` operating
  procedure (completeness); the Summary understates the scope (scope); the
  cold criterion's "those directories" (clarity); idle load in cold
  records (clarity); the incremental state has no consumer and C1 and C2
  are unexplained (still present).

### Assessment

The pass-2 critical and four of its five majors are resolved. Pass 3 finds
no critical issue, but 7 majors outside the accepted bundling. Four are
pass-2 minors re-rated after the item grew: the comparison rule, the noise
rule, and 0366's and 0367's cold-mode couplings. Three are new
testability gaps in the verification contract: fetch-failure evidence,
the `--runs 3` criterion against `deny:check`, and the unpinned baseline
label. Each pass is now finding finer-grained precision issues in an item
that has grown to 10 requirements and 18 criteria. A targeted fix of these
7 majors, with only testability and dependency re-run, is a better use of
effort than another full five-lens pass.

## Re-Review (Pass 4) — 2026-10-30T13:50:25+00:00

**Verdict:** REVISE

Lenses re-run: testability and dependency only, as agreed after pass 3.

### Previously Identified Issues

- 🟡 **Clarity + Completeness + Dependency**: Comparison rule stated four
  ways — Resolved
- 🟡 **Clarity + Testability**: Noise rule compares a change against a
  range — Resolved
- 🟡 **Testability**: Fetch failures cannot be checked from the committed
  record — Resolved; the non-network fixture should be today's `deny:check`
  advisory output (minor)
- 🟡 **Testability + Completeness**: `--runs 3` criterion cannot pass
  today — Resolved
- 🟡 **Testability**: Baseline label unpinned — Resolved
- 🟡 **Dependency + Clarity**: 0366's use of cold mode one-sided —
  Resolved; the debug-info trial's link time and binary size are not
  harness figures (minor)
- 🟡 **Dependency + Clarity**: 0367's freshness guard versus the cold
  state — Resolved
- 🟡 **Scope**: Harness bundled with host guidance — Not re-run; accepted
  by decision
- 🔵 Carried over and still present: Requirement 9's wait unrecorded on
  0362, 0363 and 0367; the reference host as an unlisted prerequisite;
  `npm audit` reaching the registry in warm runs; IDE-match and override
  criteria; per-run setup evidence; bucket rules; missing criteria for the
  label default and isolation-mode figures

### New Issues Introduced

- 🟡 **Testability**: "One commit" and "working copy matched" break once
  records are kept — jj snapshots a kept record into the working-copy
  commit, so the second and third condition runs start from a different,
  unclean commit.
- 🟡 **Testability**: Absent processes have no defined median, change,
  ratio or noise verdict — the exemption may make `XprotectService`
  disappear in `devtools` runs, which is the figure the guidance most
  needs.
- 🟡 **Testability**: Failure injection and orchestration have no stated
  verification mechanism — the injected and partial failures, priming,
  cache-state setup and `--keep` behaviour can only be checked through full
  builds.
- 🔵 **Testability**: A silent task that ran cannot be told from one that
  never ran; deletion-safety criteria do not all use sentinels; the
  closure guard may not recognise `measure:build`; the `CONTRIBUTING.md`
  criterion is looser than Requirement 8; the reference host has no
  identifier a record carries.
- 🔵 **Dependency**: Discharging the `blocks` edges leaves the graph
  asymmetric and drops the lasting 0367 link; changes from outside the
  epic (the RUSTSEC fix, tool-version bumps) are not captured as 0369 is.

### Assessment

Every pass-3 major is resolved, and the dependency lens finds no major
issue. The three new testability majors concern verification mechanics.
The first two are real gaps in the item's own definitions: commit identity
under jj, and aggregates over absent processes. The third, how
orchestration is exercised without full builds, is test-design detail that
fits an implementation plan better than a work item.

## Re-Review (Pass 5) — 2026-10-30T14:52:45+00:00

**Verdict:** REVISE

Lens re-run: testability only.

### Previously Identified Issues

- 🟡 **Testability**: "One commit" and "working copy matched" break once
  records are kept — Partially resolved: "matched" is never tested as
  false, and committing a kept record between condition runs moves the
  recorded commit
- 🟡 **Testability**: Absent processes have no defined aggregates —
  Resolved
- 🟡 **Testability**: Failure injection and orchestration have no
  verification mechanism — Resolved; the cold criterion's "`deps:install:node`
  ran" clause now passes by construction against the stub (minor)
- 🔵 Carried over and still present: bucket and idle-load rules; IDE
  versus `fsnotifier` matching; the `CONTRIBUTING.md` criterion looser than
  Requirement 8; the label default, IDE override and fetch-failure warning
  with no criterion

### New Issues Introduced

- 🟡 **Testability**: The task-set comparability term is undefined and can
  block the epic's comparison — "the set of tasks the run reached" has no
  operational definition. Under either reading, children of 0361 change the
  set themselves: 0366 collapses builds, and 0367 makes tasks skip as up to
  date. Re-measuring the baseline commit cannot restore a match. Pass 3's
  comparison-rule decision introduced this.
- 🔵 **Testability**: Condition records do not control for the IDE's work
  after a restart or re-index; the condition-records criterion omits warm
  no-op and the reference host; the nextest phase is undefined for a task
  that prints several `Summary` lines.

### Assessment

Two pass-4 majors are resolved and one is partially resolved. One new
major is a regression from the pass-3 comparison rule: task-set equality
cannot survive the epic's own changes. Both remaining majors sit in the
comparison contract that 0361's closing measurement relies on.

## Verdict Change — 2026-10-30T15:29:20+00:00

**Verdict:** APPROVE

The reviewer approved the work item after both pass-5 majors were fixed:
the task-set comparison rule (Decision M) and the commit identity of kept
records. No lens re-ran after those fixes. The pass-5 minors remain open
and were accepted as they stand.
