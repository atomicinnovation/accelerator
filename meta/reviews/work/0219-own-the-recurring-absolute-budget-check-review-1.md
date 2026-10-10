---
type: "work-item-review"
id: "0219-own-the-recurring-absolute-budget-check-review-1"
title: "Work Item Review: Track warm-dispatch latency per release in CI"
date: "2026-10-10T13:48:08+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0136"
target: "work-item:0219"
work_item_id: "0219"
reviewer: "Toby Clemson"
verdict: "REVISE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-10-10T16:32:55+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Track warm-dispatch latency per release in CI

**Verdict:** REVISE

The work item is structurally complete and well argued: every section is
populated, the CI mechanics name their actors, and `Δ_merge` is pinned by a
formula. Its weaknesses concentrate in the statistical signals and the
calibration path — the drift, tuning and ceiling criteria are underspecified
enough that any implementation passes them, and nothing bounds false positives
— and in its size, which bundles four separately deliverable workstreams and a
calendar-gated tuning step into one task.

### Cross-Cutting Themes

- **Signals are underspecified** (flagged by: testability, clarity,
  completeness, dependency) — the drift detector is unchosen and its criterion
  tautological; the tuning rule, cap and actor are undefined; the ceiling
  signal names no statistic, backend or ceiling source and contradicts the
  warm-up criterion; `Δ_merge` names no digest backend.
- **The item is several items** (flagged by: scope, testability, clarity) —
  harness method, CI lane and storage, GitHub-hosted calibration, and signals
  plus alerting are each deliverable alone, and the tuning step cannot close
  until about 20 prereleases per platform exist.
- **Calibration has no stated path** (flagged by: dependency, scope,
  testability, clarity) — the item never says which sessions produce the
  GitHub-hosted entries, who commits them, or how darwin-x64 is excused when its
  runner is gone.
- **Shared work has no owner** (flagged by: dependency, scope, completeness) —
  two-version harness support with 0299 and issue-opening with 0225 are both
  "shared" with neither item building them.

### Findings

#### Major

- 🟡 **Scope**: Four separately deliverable workstreams bundled into one task
  **Location**: Requirements
  Harness paired method, CI lane with privileged storage, GitHub-hosted
  calibration, and signals with alerting can each be built, merged and verified
  alone; the drafting notes already concede a split.
- 🟡 **Scope / Testability**: Completion gated on ~20 future prereleases and
  real regressions
  **Location**: Acceptance Criteria
  The tuning and tuned-signal criteria are calendar-driven, and no criterion
  lets the signal logic be exercised from constructed records, so the item
  either stays open for months or closes with its signals never observed.
- 🟡 **Testability / Completeness / Dependency**: Drift criterion passes for
  any detector
  **Location**: Acceptance Criteria; Open Questions
  The criterion is phrased against the detector's own output, the detector is
  unchosen and not listed as an open decision, and a third-party library choice
  would add a supply-chain dependency.
- 🟡 **Testability / Clarity**: Threshold tuning has no checkable rule, cap or
  actor
  **Location**: Requirements: Bootstrap and tuning; Acceptance Criteria
  Any committed threshold satisfies the criterion, and the passive "a change
  commits" leaves open whether the lane or a person derives and commits it.
- 🟡 **Testability / Clarity**: Ceiling signal underspecified and contradicts
  the warm-up rule
  **Location**: Requirements: Signals; Acceptance Criteria
  It fires at 10 cohort sessions while warm-up silences it until 20 platform
  sessions, and it names neither the rolled statistic (median vs p90), the
  backend, nor whether the ceiling is 0189's or the recalibrated one.
- 🟡 **Testability**: No must-not-fire criteria bound false positives
  **Location**: Acceptance Criteria
  An implementation that alerts on every merge meets every stated criterion,
  which defeats the paired design's whole rationale.
- 🟡 **Clarity**: `Δ_merge` does not say which digest backend it covers
  **Location**: Requirements: Paired method
  `G` is measured per backend in 0189; session length, threshold count and
  issue keying all depend on whether `Δ_merge` is fast, fallback or both.
- 🟡 **Dependency / Scope**: Two-version harness ownership with 0299
  unresolved
  **Location**: Open Questions; Dependencies
  Left as an open question; 0299 is `ready` and its own Dependencies record
  neither the capability nor 0219.
- 🟡 **Dependency / Scope / Testability**: Calibration path unstated
  **Location**: Requirements: Calibration
  No requirement says which sessions produce the GitHub-hosted entries or who
  commits them, and the "all four keys calibrated" criterion conflicts with the
  darwin-x64-unmeasured criterion. The dependency lens's premise that the
  harness refuses an unknown key is correct: `run_session`
  (`tasks/measure.py:1752`) raises before measuring when the platform has no
  entry, despite the `warm_dispatch` task's "uncalibrated context" message. An
  initial aggregator note calling the premise stale was wrong and is corrected
  here.

#### Minor

- 🔵 **Completeness / Clarity**: 0189 amendment only in Acceptance Criteria,
  and "the same change" has no referent
  **Location**: Requirements; Acceptance Criteria
- 🔵 **Testability**: Requirements without criteria — tag checkout (the
  wrong-binary hazard), interleave seed, concurrency group, job-summary contents
  **Location**: Requirements
- 🔵 **Testability / Clarity**: darwin-x64 "cannot be scheduled" is not a
  precondition a tester can set up, and "all four" criteria do not allow for it
  **Location**: Acceptance Criteria
- 🔵 **Dependency**: GitHub App blocker names no owner and ignores the existing
  `ACCELERATOR_RELEASER` App, which already pushes to `main`
  **Location**: Dependencies
- 🔵 **Dependency**: Joining `accelerator-release` couples the lane to the
  release pipeline's keep-in-sync invariant and can delay releases
  **Location**: Requirements: Storage
- 🔵 **Dependency**: Release host not named as an external system; outage
  classification unstated
  **Location**: Assumptions; Dependencies
- 🔵 **Clarity**: "Bootstrap" means both the resampling estimator and the
  20-session warm-up
  **Location**: Requirements; Acceptance Criteria
- 🔵 **Clarity**: Three platform vocabularies with no mapping
  **Location**: Requirements: Lane
- 🔵 **Clarity**: "Predecessor" shifts meaning under gap-pairing, and
  `Δ_merge` can span several merges
  **Location**: Summary; Acceptance Criteria
- 🔵 **Clarity**: Release, prerelease and merge used interchangeably
  **Location**: Title; Summary
- 🔵 **Clarity**: Local terms without gloss — `G`, instrument floor, farm,
  cohort absolutes, report-only, sanity cap, `Main`
  **Location**: Requirements; Technical Notes

#### Suggestions

- 🔵 **Scope / Dependency**: Unclear who builds the issue-opening mechanism
  shared with 0225
  **Location**: Dependencies
- 🔵 **Completeness**: Committed record contents and layout only partly
  specified
  **Location**: Requirements
- 🔵 **Dependency**: 0217 link recorded one way (`blocks` there, `relates_to`
  here)
  **Location**: Frontmatter: relates_to
- 🔵 **Scope**: Summary omits calibration, the ceiling signal and the 0189
  amendment
  **Location**: Summary
- 🔵 **Clarity**: "This one" in the Context callout has no antecedent
  **Location**: Context

### Strengths

- ✅ Every template section is populated with substantive content, and the
  frontmatter is complete and consistent with the body.
- ✅ Context explains why the lane must follow `prerelease` — the harness
  measures the published release — removing the most likely misreading.
- ✅ CI mechanics name actor and trigger: one aggregating job, one commit, a
  GitHub App, `[skip ci]`, touching only `meta/measurements/`.
- ✅ `Δ_merge` is defined by formula on first use.
- ✅ Edge cases have their own criteria: gap-pairing, issue deduplication,
  invalidated sessions committed with their reason.
- ✅ Drafting Notes separate settled intent from interpretation, and every
  related item carries the reason it relates.

### Recommended Changes

1. **Decompose into a parent and children** (addresses: four workstreams
   bundled; completion gated on future prereleases; calibration path unstated;
   two-version ownership)
   Make 0219 the parent under 0136 and split in delivery order: two-version
   paired harness method (shared with 0299, which takes a `blocked_by`); a
   report-only CI lane with storage and runner identity; GitHub-hosted
   calibration from the lane's own uncalibrated sessions; signals and alerting
   verified against fixture records; and a calendar-gated tuning item blocked on
   20 valid sessions per platform.
2. **Specify each signal by input and output** (addresses: drift criterion;
   ceiling signal; `Δ_merge` backend; no must-not-fire criteria)
   State the backend(s) `Δ_merge` covers; for the ceiling, name the rolled
   statistic per cell and the ceiling source, and give it the warm-up
   precondition; phrase drift as synthetic step and stationary series with
   expected issue or silence; add negative criteria for within-threshold,
   report-only and stationary cases.
3. **Settle the tuning rule, cap and actor** (addresses: threshold tuning)
   Move the proposed rule out of Open Questions with a cap value, name who
   commits thresholds and what prompts them, and require the committed value to
   be recomputable from committed records.
4. **Record the calibration path** (addresses: calibration path unstated;
   darwin-x64 precondition)
   Calibrated entries derive from the lane's uncalibrated-context sessions;
   name who commits them; excuse darwin-x64 explicitly when its runner is
   unavailable, with an observable trigger (unknown-label failure or start
   timeout).
5. **Tighten dependencies** (addresses: App owner; concurrency coupling;
   release host; 0225 mechanism; 0217 link)
   Decide reuse of `ACCELERATOR_RELEASER` versus a new App and name the owner;
   record the `accelerator-release` coupling and its keep-in-sync rule; add the
   release host as an external system with outage classification; assign the
   issue-opening mechanism; reconcile the 0217 link.
6. **Fix vocabulary** (addresses: bootstrap overloaded; platform vocabularies;
   predecessor; release/prerelease/merge; undefined terms; "this one")
   Rename the warm-up period, add a label-key-short-name table, define
   "predecessor" as the most recent validly measured prerelease, state that one
   merge yields one prerelease, gloss `G`, C1-C4, farm and report-only.
7. **Close requirement-to-criterion gaps** (addresses: requirements without
   criteria; 0189 amendment)
   Add criteria for the measured version equalling the `prerelease` tag, the
   recorded interleave seed, and job-summary contents; add a Requirements bullet
   for the 0189 amendment naming the change it lands with.

## Per-Lens Results

### Clarity

**Summary**: The item is precise and argues well, and most terms trace to
linked documents. Clarity problems sit where vocabularies meet: "bootstrap"
carries two meanings, platforms are named three ways, merge/prerelease/release
are near-synonyms, the ceiling signal is unspecified, and key actions are
passive.

**Strengths**: named actors and triggers in CI requirements; `Δ_merge` defined
by formula; Context removes the wrong-binary misreading; Drafting Notes mark
interpretations; consistent Given/When/Then with concrete artefacts.

**Findings**:

- 🟡 major / high — Requirements: Signals; Acceptance Criteria — Ceiling signal
  does not say which statistic, backend or ceiling values it compares.
- 🟡 major / medium — Requirements: Paired method — `Δ_merge` does not say
  which digest backend it covers.
- 🟡 major / high — Requirements: Bootstrap and tuning; Acceptance Criteria —
  Unclear who or what commits the derived thresholds, and when; same gap for
  calibration.
- 🔵 minor / high — Requirements; Acceptance Criteria — "Bootstrap" means two
  different things.
- 🔵 minor / medium — Requirements: Lane — Three platform vocabularies with no
  mapping.
- 🔵 minor / medium — Summary; Acceptance Criteria — "Predecessor" has two
  meanings under gap-pairing.
- 🔵 minor / medium — Title; Summary; Context — Release, prerelease and merge
  used interchangeably.
- 🔵 minor / medium — Acceptance Criteria — "Amended in the same change" has
  no clear referent.
- 🔵 minor / medium — Acceptance Criteria — Signal preconditions and "all four"
  criteria read inconsistently.
- 🔵 minor / medium — Requirements; Technical Notes — Local terms used without
  gloss.
- 🔵 suggestion / high — Context — "This one" has no antecedent.

### Completeness

**Summary**: Structurally very complete for a task: every section is present
and substantive, and the frontmatter is valid. Remaining gaps are small.

**Strengths**: complete, consistent frontmatter; Context explains the forces;
Requirements specific enough to act on; criteria map to most requirements
including edge cases; all supporting sections populated.

**Findings**:

- 🔵 minor / high — Requirements — 0189 amendment appears in Acceptance
  Criteria but not in Requirements.
- 🔵 minor / medium — Open Questions — Change-point detector choice not listed
  as an open decision.
- 🔵 suggestion / low — Requirements — Committed record contents and layout
  only partly specified.

### Dependency

**Summary**: Main relationships are captured with reasons, but several implied
couplings are missing: the calibration path, ordering with 0299, the release
concurrency group, the release host, and the App's owner.

**Strengths**: the App blocker lists exact permissions; every related item
carries its reason; runner availability and Intel macOS loss are covered;
Technical Notes explain the pipeline couplings.

**Findings**:

- 🟡 major / medium — Requirements: Lane / Calibration — Calibration sessions
  must exist before the lane, with no trigger. *Aggregator note: the premise
  holds — `run_session` (`tasks/measure.py:1752`) raises before measuring when
  the platform has no entry.*
- 🟡 major / high — Open Questions; Dependencies — Ordering with 0299 for
  two-version support unresolved, and 0299 does not record it.
- 🔵 minor / medium — Dependencies — App blocker names no owner; the existing
  `ACCELERATOR_RELEASER` App is not mentioned.
- 🔵 minor / medium — Requirements: Storage — Joining `accelerator-release`
  couples the lane to the release pipeline.
- 🔵 minor / high — Assumptions; Dependencies — Release host not named as an
  external system.
- 🔵 minor / medium — Technical Notes — Third-party detector library not
  recorded as a dependency.
- 🔵 suggestion / high — Frontmatter: relates_to — 0217 link recorded one way.
- 🔵 suggestion / low — Dependencies — Builder of the issue mechanism shared
  with 0225 unstated.

### Scope

**Summary**: Every requirement serves one goal, but the item is too big to be
one unit of delivery: harness method, provenance, calibration, CI lane, four
signals and a data-gated tuning step, filed as one task that cannot close until
about 20 prereleases per platform are measured.

**Strengths**: one coherent goal with nothing unrelated; explicit boundaries;
drafting notes flag the split; related work named with reasons.

**Findings**:

- 🟡 major / high — Requirements — Four separately deliverable workstreams
  bundled into one task.
- 🟡 major / high — Acceptance Criteria — Completion depends on ~20 future
  prereleases and a later tuning change.
- 🔵 minor / medium — Open Questions — Two-version harness ownership
  undecided.
- 🔵 minor / medium — Requirements — Calibration depends on sessions from the
  lane this item builds.
- 🔵 suggestion / medium — Dependencies — Who builds the issue mechanism
  shared with 0225 unstated.
- 🔵 suggestion / medium — Summary — Summary omits calibration and the ceiling
  signal.

### Testability

**Summary**: Most criteria are crisp Given/When/Then with observable outcomes.
The weak points are the statistical signals: drift and tuning criteria are
judged against undefined detectors and rules, several signal criteria need real
regressions to exercise, the ceiling criterion contradicts the warm-up
criterion, and nothing bounds false positives.

**Strengths**: definitive pass/fail checks against GitHub state; gap-pairing
edge case; deduplication criterion; invalidated sessions recorded; the warm-up
criterion is falsifiable.

**Findings**:

- 🟡 major / high — Acceptance Criteria — Drift criterion is judged against the
  detector itself.
- 🟡 major / high — Acceptance Criteria — Threshold derivation criterion passes
  whatever rule is chosen.
- 🟡 major / high — Acceptance Criteria — Ceiling criterion contradicts the
  warm-up criterion.
- 🟡 major / medium — Acceptance Criteria — Signal criteria can only be
  exercised by waiting for real releases and regressions.
- 🟡 major / high — Acceptance Criteria — No must-not-fire criteria bound false
  positives.
- 🔵 minor / medium — Acceptance Criteria — darwin-x64 "cannot be scheduled"
  is not a settable precondition.
- 🔵 minor / medium — Requirements — Several requirements have no matching
  criterion.
- 🔵 minor / medium — Acceptance Criteria — Calibration criterion does not
  check the derivation rules.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-10-10T15:33:33+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Scope**: Four separately deliverable workstreams bundled — Still
  present (deferred to `/refine-work-item` by decision)
- 🟡 **Scope / Testability**: Completion gated on future prereleases —
  Partially resolved (fixture verification added; calendar gating remains)
- 🟡 **Testability / Completeness / Dependency**: Drift criterion passes for
  any detector — Partially resolved (input/output form added; fixtures not
  deterministic)
- 🟡 **Testability / Clarity**: Threshold tuning rule, cap and actor —
  Resolved
- 🟡 **Testability / Clarity**: Ceiling signal underspecified — Resolved
- 🟡 **Testability**: No must-not-fire criteria — Resolved
- 🟡 **Clarity**: `Δ_merge` backend unstated — Resolved
- 🟡 **Dependency / Scope**: Two-version ownership with 0299 — Resolved
- 🟡 **Dependency / Scope / Testability**: Calibration path unstated —
  Partially resolved (path stated; warm-up validity open; existing entries
  unaddressed)
- 🔵 **Completeness / Clarity**: 0189 amendment placement — Resolved
- 🔵 **Testability**: Requirements without criteria — Partially resolved
- 🔵 **Testability / Clarity**: darwin-x64 precondition — Resolved
- 🔵 **Dependency**: App owner and releaser reuse — Resolved
- 🔵 **Dependency**: Concurrency-group coupling — Resolved
- 🔵 **Dependency**: Release host as external system — Resolved
- 🔵 **Clarity**: "Bootstrap" overloaded — Resolved
- 🔵 **Clarity**: Platform vocabularies — Resolved
- 🔵 **Clarity**: "Predecessor" shifting — Resolved
- 🔵 **Clarity**: Release/prerelease/merge — Partially resolved (post-stable
  prerelease from `release` unaddressed)
- 🔵 **Clarity**: Undefined terms — Partially resolved
- 🔵 **Scope / Dependency**: 0225 mechanism ownership — Resolved
- 🔵 **Completeness**: Record layout — Resolved
- 🔵 **Dependency**: 0217 one-way link — Still present (left for an edit on
  0217)
- 🔵 **Scope**: Summary omissions — Resolved
- 🔵 **Clarity**: "This one" antecedent — Resolved

### New Issues Introduced

- 🟡 **Completeness / Testability**: Warm-up session validity has no rule or
  default, yet warm-up counting, invalidation, baseline selection and tuning
  all depend on it
- 🟡 **Testability**: Drift fixtures have no seed, distribution or error
  budget, so the criteria can be flaky or tuned to one draw
- 🟡 **Scope / Dependency / Testability**: Closure depends on merge cadence —
  20 valid sessions per platform-backend plus a human tuning change — and the
  calibrated-provenance criterion is open-ended
- 🟡 **Clarity**: `V` is never defined
- 🟡 **Clarity**: "Without an entry" conflicts with the existing linux-arm64
  and darwin-arm64 entries
- 🟡 **Clarity**: Platform versus platform-backend granularity shifts across
  warm-up, entries, ceilings and issue keys
- 🟡 **Clarity**: Runner-identity tuple and "current cohort" not pinned
- 🔵 **Completeness**: Detector parameters have no derivation rule
- 🔵 **Completeness**: Session-duration question has no interim timeout
- 🔵 **Scope**: Detector choice is unresolved research inside a delivery item
- 🔵 **Testability**: Missing cohort-reset, report-only-ceiling and
  invalidation-negative cases
- 🔵 **Testability**: Lane-trigger and commit-only constraints lack criteria
- 🔵 **Clarity**: `t` reuses 0189's ceiling symbol; percentage versus ratio
  units, and log versus ratio space for the drift shift
- 🔵 **Clarity**: Actors that evaluate signals and open the tuning issue
  unnamed
- 🔵 **Dependency**: App blocker stated item-wide though only two jobs need
  it
- 🔵 **Dependency**: No seed baseline for the first lane run
- 🔵 **Dependency**: 0225 back-reference absent; existing max-age issue guard
  unreferenced
- 🔵 **Dependency**: 0299's before/after figures may straddle harness
  revisions
- 🔵 **Dependency**: GitHub-hosted runners not named as an external system
- 🔵 **Scope**: Building the open-or-comment step for 0225 widens scope
- 🔵 **Testability**: Bootstrap seed and resample count not recorded, so the
  interval is not reproducible

### Assessment

Every first-pass finding on signal specification, tuning, alerting negatives,
backend coverage, dependencies and vocabulary is resolved. The revision
surfaced a second layer: four clarity gaps in the new vocabulary (`V`,
existing-entry handling, granularity, cohort), the warm-up validity rule, and
deterministic drift fixtures. The scope and closure-gating findings remain by
design and are resolved by decomposition. The item is not ready for planning;
settle warm-up validity, apply the clarity fixes, then decompose under
`/refine-work-item`, placing tuning in its own calendar-gated child.

## Re-Review (Pass 3) — 2026-10-10T16:32:55+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Completeness / Testability**: Warm-up session validity — Resolved
- 🟡 **Testability**: Drift fixtures not deterministic — Resolved (a
  follow-on σ/τ circularity is raised below)
- 🟡 **Scope / Dependency / Testability**: Closure gated on merge cadence —
  Still present (deferred to decomposition)
- 🟡 **Scope**: Several deliverables in one task — Still present (deferred to
  decomposition)
- 🟡 **Clarity**: `V` undefined — Resolved
- 🟡 **Clarity**: Existing entries versus "without an entry" — Resolved
- 🟡 **Clarity**: Platform versus platform-backend granularity — Resolved
- 🟡 **Clarity**: Runner-identity tuple and current cohort — Resolved
- 🔵 **Completeness**: Detector parameter derivation — Resolved
- 🔵 **Completeness**: Interim timeout — Resolved
- 🔵 **Scope**: Detector choice inside a delivery item — Still present (open
  question with a default)
- 🔵 **Testability**: Cohort-reset, report-only-ceiling, invalidation-negative
  cases — Resolved
- 🔵 **Testability**: Lane-trigger and commit-only criteria — Resolved
- 🔵 **Clarity**: `t` symbol and units — Resolved
- 🔵 **Clarity**: Unnamed actors — Resolved
- 🔵 **Dependency**: App blocker scope — Resolved
- 🔵 **Dependency**: First-run baseline — Resolved
- 🔵 **Dependency**: 0225 back-reference and max-age guard — Partially
  resolved (guard found absent; back-reference left for an edit on 0225)
- 🔵 **Dependency**: 0299 harness-revision coordination — Resolved
- 🔵 **Dependency**: GitHub-hosted runners as external system — Resolved
- 🔵 **Scope**: Open-or-comment step widening scope — Resolved
- 🔵 **Testability**: Bootstrap reproducibility — Resolved
- 🔵 **Dependency**: 0217 one-way link — Still present

### New Issues Introduced

- 🟡 **Clarity**: Warm-up, "past warm-up" and "tuned" leave an undefined state
  between the 20th valid session and the tuning commit
- 🟡 **Clarity / Testability**: Calibration derivation order and inputs are
  ambiguous — which floor reading and aggregation, gates fixed before or after
  exclusion, how a value in the 40–80% band is chosen, and uncapped regression
  exclusions
- 🟡 **Testability**: Drift criteria depend on per-platform σ and τ that exist
  only after tuning, the tuning criterion never re-runs them, and no fallback
  is defined when no detector parameters meet both bounds
- 🔵 **Completeness**: No triage owner for alert issues
- 🔵 **Completeness**: Criterion amendment silent on C5 and C6
- 🔵 **Completeness / Scope**: Kind `task` with planned children unlisted
- 🔵 **Dependency**: Prereleases cut locally via `mise run prerelease` enter
  the baseline sequence uncaptured
- 🔵 **Dependency**: Conditional supply-chain dependency if a third-party
  detector is chosen
- 🔵 **Dependency**: Build order between decomposition seams unrecorded
- 🔵 **Testability**: Same-runner pairing, interleaving and the resample floor
  have no criterion
- 🔵 **Testability**: The 20% floor-stability check has no denominator
- 🔵 **Testability**: Warm-up suppression precondition cannot be constructed
- 🔵 **Testability**: Timeout revision has no exit condition
- 🔵 **Testability**: Ceiling and open-or-comment edge cases (equality,
  window roll-off, closed issues, backend-distinct keys)
- 🔵 **Testability**: No verification mode stated for lane and storage
  criteria
- 🔵 **Clarity**: "Drift" names both the in-session check and the cross-merge
  signal
- 🔵 **Clarity**: Entry provenance names one runner tuple; warm-up can span
  cohorts
- 🔵 **Clarity**: Unmeasured runs' effect on the invalidation streak
- 🔵 **Clarity**: "Immediate predecessor" around stable and post-stable
  releases
- 🔵 **Clarity**: Actor and location for regression exclusions
- 🔵 **Clarity**: One lane job versus per-platform measuring jobs
- 🔵 **Clarity**: "Cohort trend" versus the rolling-median rule
- 🔵 **Clarity**: "Session" and harness terms undefined
- 🔵 **Scope**: A thin ceiling-and-invalidation lane would discharge 0189's
  obligation sooner; amendment ownership after a split; four platforms at once

### Assessment

All eight pass-2 majors not deferred to decomposition are resolved. The three
new majors sit at the specification's finest grain — the state between
warm-up and tuning, the calibration derivation's order, and the drift
criteria's dependence on tuned parameters — and each is a precise wording
change rather than a design gap. Successive passes are now finding narrower
issues in text that decomposition will redistribute; the item is ready for
`/refine-work-item` once the three majors are fixed, with the minors carried
into the children they belong to.
