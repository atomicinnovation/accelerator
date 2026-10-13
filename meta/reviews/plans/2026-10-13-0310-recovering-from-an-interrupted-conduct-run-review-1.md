---
type: "plan-review"
id: "2026-10-13-0310-recovering-from-an-interrupted-conduct-run-review-1"
title: "Plan Review: Recovering from an Interrupted Conduct Run Implementation Plan"
date: "2026-10-13T13:28:29+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-13-0310-recovering-from-an-interrupted-conduct-run"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "usability", "compatibility"]
review_number: 1
review_pass: 4
tags: ["research", "conduct", "run-ledger", "recovery"]
last_updated: "2026-10-13T17:49:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Recovering from an Interrupted Conduct Run Implementation Plan

**Verdict:** REVISE

The plan is structurally sound. The classification is a pure function in
`research::conduct`, the CLI emits an additive conditional key, and the rollout
never ships a release with neither signal. Its Rust compiles against the real
APIs. The classification rules, however, diverge from the work item in two
places: a finding counts as valid when it answers any outline pair rather than
its own, and an off-question root note escapes the plan-rejection check
because `--start` moves its pair to a fresh stem. The quarantine list is also
single-shot, so a second interruption (likely, when usage limits are the
cause) silently loses it. Six further major findings concern the report's
usability, the boundary with 0307's ledger schema, and gaps in test pinning.

### Cross-Cutting Themes

- **Finding validity is "any outline pair", not "its own pair"** (flagged by:
  architecture, correctness, code-quality, test-coverage). `answers_an_outline_pair`
  adds a third validity rule, alongside `Observed`'s `answer().is_some()` and
  the planner's per-pair `answers`. A misdirected finding at `02-b-web.md`
  naming `A?` is reported as "written by run" and is not quarantined, while
  `B?` is re-offered.
- **The quarantine list is single-shot** (flagged by: safety, architecture,
  correctness). `--start` overwrites the replaced ledger before rendering. If
  the new run stops before step 3, the next `--start` classifies the new run's
  batch at fresh stems, and the original invalid file is never listed again.
- **Stale classification under a live earlier run** (flagged by: safety,
  architecture, correctness). Step 3 renames on a judgement made at `--start`.
  An in-flight agent may have fixed the file in between, so valid work gets
  set aside. This is recoverable, but nothing records the tradeoff.
- **Hand CLI users lose the stderr signal** (flagged by: usability,
  compatibility, safety). Phase 3 drops the readable-ledger line from both
  `warnings` and stderr.

### Tradeoff Analysis

- **Durability vs scope**: making the quarantine list survive a second
  interruption means carrying invalid stems in the new ledger's memory. That
  adds to the ledger, which the plan's "only `run` and `pending` are read"
  boundary resists. The alternative, documenting the gap under What We're NOT
  Doing, keeps the scope narrow but leaves the exact failure the story targets
  reachable. Recommendation: at minimum, document it and add a walkthrough
  step. Carrying the stems is the stronger fix, if the 0307 boundary allows an
  additive memory field.
- **Skill-clean warnings vs CLI-level signal**: dropping the stderr line keeps
  the skill's output clean, but removes a cheap concurrency hazard warning at
  the CLI boundary. A stderr-only line (no `warnings` entry) satisfies both.

### Findings

#### Major

- 🟡 **Architecture + Correctness + Code Quality + Test Coverage**: A pair's
  finding is valid if it answers any outline pair, not its own
  **Location**: Phase 1, Section 2: `answers_an_outline_pair`; Key Discoveries
  The work item says valid means "answers its outline pair". The planned rule
  accepts a finding naming a sibling question, so a misdirected file is
  reported as written, is not quarantined, and keeps counting towards
  `finding_count`.
- 🟡 **Correctness**: A root note that answers the wrong question is classed
  valid on `--start`
  **Location**: Phase 1, Section 2: `note_at` / `plan_rejects`
  With empty claims, the off-question `.levels/` directory claims its index for
  `Other?`, and the pair moves to a fresh stem (`plan.rs:965-981`). So
  `plan_rejects(01-a-web:1)` finds no pair and `notes()` holds it, which
  classes it as valid. Verified against
  `a_pair_keeps_its_stem_when_its_root_note_records_another_question`.
- 🟡 **Safety + Architecture + Correctness**: The quarantine list is lost if
  the new run stops before step 3
  **Location**: Phase 2, Section 2; Phase 3, Step 3 Clear
  The list exists only in one plan's stdout. A second interruption leaves the
  invalid leftover visible and counted for good.
- 🟡 **Compatibility**: The ledger reader needs every field, so the "only
  `run` and `pending` are read" boundary does not hold
  **Location**: What We're NOT Doing / Migration Notes
  `ledger_from` (`research-adapters/src/conduct/ledger.rs:140-168`) returns
  `Corrupt` when `claims`, `attempted`, `just_acknowledged` or `seen` is
  missing. If 0307 adds fields that are required when read, a pre-upgrade
  ledger falls back to the corrupt warning, with no report.
- 🟡 **Usability**: Docs still say deleting a leftover ledger is safe, which
  now silently skips recovery
  **Location**: Phase 4, Section 3
  Deleting the ledger loses both the report and the quarantine, leaving the
  invalid leftovers counted.
- 🟡 **Usability**: The "unfinished run" row reaches the researcher only after
  they have re-run
  **Location**: Phase 4, Section 2
  Pointing from the report to "re-run `conduct`" is circular inside the re-run,
  and the wait-for-reset advice arrives after the moment it mattered.
- 🟡 **Usability**: The report lists bare stems, so the researcher cannot tell
  which question was affected
  **Location**: Phase 3, Section 2 (report instruction)
  An invalid pair re-offered as `03-a-web` was reported as `01-a-web`, with no
  question shown to link the two.
- 🟡 **Code Quality**: The one retained docstring gives a reason the plan
  contradicts
  **Location**: Phase 1, Section 2: `findings_to_quarantine`
  "because no offered spawn's path reaches them" is false for the compose case
  that Key Discoveries itself flags.
- 🟡 **Test Coverage**: No test pins the three report group labels to their
  classes
  **Location**: Phase 3, Section 1
  Swapping two headings, or dropping "quarantined", passes every automated
  check. The first acceptance criterion is checked only by hand.

#### Minor

- 🔵 **Correctness**: The mixed-batch fixture's stems and order cannot occur
  **Location**: Phase 2, Section 1
  Seeding `01` and `05` raises the allocation floor to 6, and the window ranks
  compositions last. The literal expected JSON is unreachable.
- 🔵 **Correctness**: The "fresh index" invariant also fails when a quarantine
  marker carries the question
  **Location**: Current State Analysis / Key Discoveries
  `claimed_by_a_file` reuses a marker's index, so the invariant is "whenever a
  file claim holds the index", not "compose only".
- 🔵 **Safety + Architecture + Correctness**: Step 3 renames on a
  classification that a still-running earlier run may have made stale
  **Location**: Phase 3, Step 3 Clear
  Re-check validity before renaming, or record the accepted tradeoff.
- 🔵 **Safety**: Quarantine is tied to a first batch that may never happen
  **Location**: Phase 3, Step 3 Clear
  A plan with nothing outstanding but a non-empty `quarantine` can go straight
  to step 7, and `end-run` then drops the list.
- 🔵 **Safety**: Outline edits between runs turn valid findings into
  quarantine targets
  **Location**: Phase 1, `answers_an_outline_pair`
  A reworded question quarantines good research under an "invalid" label.
- 🔵 **Usability**: "offered again" is promised even when the spawn may not be
  offered in this run
  **Location**: Phase 3, Section 2
  An unplanned node, a skipped pair, or a spawn held back by the concurrency
  window.
- 🔵 **Usability**: A refused leftover note appears twice, in the report and
  again under "Refused and re-researched"
  **Location**: Phase 3, Section 2
  Decide which surface owns it.
- 🔵 **Code Quality**: The hoisted `let mut replaced` is set as a side effect
  inside the mode match
  **Location**: Phase 2, Section 2
  Compute it as its own expression instead.
- 🔵 **Code Quality**: Phase 3 leaves `notice` as a leftover two-case shape
  **Location**: Phase 3, Section 3
  Inline `CONSEQUENCE` and rename the method to `corrupt_ledger_notice`.
- 🔵 **Code Quality**: Private type names don't match the domain language
  **Location**: Phase 1, Section 2
  `SetReading` suggests I/O, `LastWrite::Unwritten` contradicts itself, and
  the local `run` shadows a field.
- 🔵 **Architecture**: The node-rejection lookup repeats the stage walk in
  `window::spawns_of`
  **Location**: Phase 1, `plan_rejects`
- 🔵 **Test Coverage**: Several Phase 2 tests cannot fail before the CLI
  change
  **Location**: Phase 2 Success Criteria
  The absence and characterisation tests are guards, not red-first tests.
- 🔵 **Test Coverage**: The order tests may not catch a sorting mutation
  **Location**: Phase 1 order test; Phase 2 mixed-batch test
- 🔵 **Test Coverage**: The matching filters in `plan_rejects` have no direct
  unit guard
  **Location**: Phase 1
- 🔵 **Test Coverage**: No test checks that the report comes before step 3, or
  that the row pointer exists
  **Location**: Phase 3, Section 1
- 🔵 **Compatibility**: Hand runs of `--start` lose the stderr signal
  **Location**: Phase 3, Section 3

#### Suggestions

- 🔵 **Architecture**: Drop `findings_to_quarantine` from the pinned API and
  derive the paths in `render` from `invalid()`.
- 🔵 **Code Quality**: Make the precedence of rejection over acceptance in
  `note_at` explicit.
- 🔵 **Correctness**: A stale refused note left by an unfinished step 3 is
  reported as "written but invalid".
- 🔵 **Compatibility**: Record the warning removal under `### Removed`.
- 🔵 **Compatibility**: Pin that `replaced_run` leaves every other key
  unchanged.
- 🔵 **Test Coverage**: Add a question-normalisation case to the validity
  tests.
- 🔵 **Test Coverage**: Add an automated proxy for the `finding_count`
  exclusion.
- 🔵 **Usability**: Word the valid group as "written by run {run}; kept as
  answered".

### Strengths

- ✅ The functional core and imperative shell are cleanly separated:
  `ReplacedRun::of` is pure over in-memory values, so the domain crate stays
  I/O-free under `cli/pup.ron`.
- ✅ The CLI computes and the skill renames: the "CLI never renames files"
  boundary holds.
- ✅ Rollout is evolution-safe. The key is added beside the notice, and the
  notice is dropped in the same change the skill reads the key, so no release
  shows neither.
- ✅ `ReplacedLedger { Absent, Readable, Corrupt }` models the three states
  explicitly and keeps the ledger alive until classification.
- ✅ The plan's Rust matches the real APIs: field names, signatures and
  borrows all check out.
- ✅ The step 3 ordering (quarantine list first, then the offered-path rule on
  an existing file) renames each file exactly once.
- ✅ The opening sentence never asserts the earlier run stopped, and a
  structure test enforces this.
- ✅ The unit test names read as the classification rules, and absence is
  tested for every case the work item lists.

### Recommended Changes

1. **Classify a pair against its own pair** (addresses: any-outline-pair
   validity, outline-edit quarantine)
   Resolve the spawn's own pair: the outline item and profile whose
   `Stem::allocated(stem.index(), slug, profile)` reproduces the stem. Valid
   only when the finding answers that pair. Give the rule one named home that
   the planner can share. Add
   `a_pair_whose_finding_answers_another_outline_pair_is_invalid` and a
   normalisation case.
2. **Check a root node's note against its candidate directly** (addresses:
   off-question root note classed valid)
   Do not rely on `plan.pairs` at the same stem. Compare the note's normalised
   question with the candidate question for the stem, and add an empty-claims
   unit test.
3. **Make the quarantine list durable, or declare the gap** (addresses:
   single-shot list, first batch that may never happen)
   Tie quarantine to showing the report, not to the first spawn. Then either
   carry the unquarantined stems into the new ledger, or add the gap to What
   We're NOT Doing plus a walkthrough step.
4. **Restate the 0307 boundary** (addresses: ledger reader needs every field)
   Say that the ledger must read when fields added later are missing. Add a
   tolerance test, and record the default-when-absent requirement against
   0307.
5. **Rework the report's user-facing surface** (addresses: bare stems,
   circular row, "safe to delete", "offered again", double reporting)
   Show the question and profile, plus the new stem for re-offered pairs. Make
   the report's trailing line speak to the current run, and put the wait-then-
   re-run guidance in the docs. Recommend re-running over deleting the ledger.
6. **Tighten the tests** (addresses: group labels, fixture, order, red-first,
   report position)
   Pin each heading to its class. Fix the mixed-batch fixture's seeds and
   expected literals. Use a non-lexical batch order. Separate guard tests from
   red-first tests. Assert the report comes before step 3.
7. **Code tidy-ups** (addresses: docstring, hoisted `mut`, leftover `notice`,
   names)
   Reword or drop the docstring, compute `replaced` as an expression, collapse
   `notice` in Phase 3, and rename the private types.
8. **Keep a stderr-only line for a readable replacement** (addresses: hand CLI
   users lose the signal), and file the removal under `### Removed`.

## Per-Lens Results

### Architecture

**Summary**: Structurally sound: the classification is a pure, I/O-free value
in `research::conduct`, the CLI owns I/O and rendering, and the phases change
the contract additively. The main weakness is in the domain model: a third,
broader definition of finding validity, and a duplicate walk over plan stages
to find rejections. The quarantine list also lives only in the one response
that overwrites its source ledger.

**Strengths**: The functional core and imperative shell are separated. The
CLI computes and the skill renames. The rollout is evolution-safe. Only `run`
and `pending` are read. `ReplacedLedger` models the three states. The compose
overlap is named and resolved.

**Findings**:

- 🟡 major / high: **A third, broader definition of finding validity** (Phase
  1, Section 2). The planned rule is "answers any outline item and profile",
  whereas the work item and step 3 say "answers its own pair". A finding at
  `02-b-web.md` naming `A?`/web is reported as written and not quarantined.
  Resolve the spawn's own pair through `Stem::allocated` or the replaced
  ledger's `claims()`.
- 🔵 minor / medium: **`plan_rejects` repeats the stage walk in
  `window::spawns_of`** (Phase 1). Extract a shared projection from a plan's
  spawns to `(SpawnRef, Option<NoteRejection>)`.
- 🔵 minor / medium: **The quarantine list is lost if the run stops before
  step 3** (Phase 2/3). Either document the gap or carry the invalid stems in
  the new ledger's memory.
- 🔵 minor / low: **A possibly live run's files are quarantined** (Phase 3).
  Record the tradeoff: the marker preserves the file.
- 🔵 suggestion / medium: **The pinned API returns paths as strings**. Derive
  `quarantine` in `render` from `invalid()`, or return `&Stem`.

### Code Quality

**Summary**: A small, pure, well-named domain module that follows the
`Observed`/`window` patterns. The risks are: a second meaning for "a valid
finding", a docstring the plan contradicts, `run_outstanding` growing through
a hoisted `mut`, and naming and leftover tidy-ups.

**Strengths**: A pure function with `UnicodeText` injected. A three-state
enum instead of an option plus a flag. The conditional-key pattern is reused.
Test names read as the rules. The phases are small.

**Findings**:

- 🟡 major / medium: **The retained docstring gives a reason the plan
  contradicts** (Phase 1, `findings_to_quarantine`). Drop it, or reword it to
  "listed whether or not the new plan offers their path again".
- 🟡 major / medium: **A second private definition of "a finding that
  counts"** (Phase 1, `answers_an_outline_pair`). Give the concept one home,
  e.g. a method on `RoundInputs` or `Outline`, or state why `Observed` stays
  narrower.
- 🔵 minor / high: **Hoisted `let mut replaced` set inside the mode match**
  (Phase 2). Compute it as its own expression, and consider
  `ReplacedLedger::classify`.
- 🔵 minor / high: **Phase 3 leaves `notice` as a leftover two-case shape**.
  Inline `CONSEQUENCE` and rename the method to `corrupt_ledger_notice`.
- 🔵 minor / medium: **Private type names don't match the domain language**.
  Prefer `SpawnOutcome` to `LastWrite`, rename the local `run` to `report`,
  and avoid `SetReading`.
- 🔵 suggestion / low: **The precedence of rejection over acceptance is
  implicit in the if-order**. Use an explicit early-return guard.

### Test Coverage

**Summary**: Strong and proportionate: one unit test per rule, presence and
absence integration tests, and structure tests for the prose. The gaps: group
labels are unpinned, there is no sibling-misdirection case, some mutations
survive, and the Phase 2 red-first claim is overstated.

**Strengths**: One unit test per rule, in the `observed.rs` fixture style.
Whole-value `assert_eq!` with absolute paths. Absence is tested for every
listed case. Regression guards cover the step 7 tick and stem survival. The
notice tests are updated test-first. The "never asserts stopped" check is
mechanical.

**Findings**:

- 🟡 major / high: **No test pins the three report group labels to their
  classes** (Phase 3). Add
  `test_the_report_groups_each_class_under_its_heading`.
- 🟡 major / medium: **No test covers a finding that answers a sibling outline
  pair** (Phase 1). Add a unit test (and ideally an integration case)
  asserting the intended class, alongside the planner's view.
- 🔵 minor / high: **Several Phase 2 tests cannot fail first**. Split the
  criterion into red-first tests and guards.
- 🔵 minor / medium: **The order tests may not catch a sort, and the
  integration expectation is left open**. Build the batch in a non-lexical
  order and commit to literals.
- 🔵 minor / medium: **The `plan_rejects` filters have no direct unit guard**.
  Add a test with same-lineage nodes in two pairs and different lineages in
  one pair.
- 🔵 minor / medium: **No test that the report precedes step 3, or that the
  row pointer exists**. Compare indices, and assert the Phase 4 pointer.
- 🔵 suggestion / medium: **No question-normalisation case**. Use an outline
  of `A b?` and a finding of `  A \t b？`.
- 🔵 suggestion / low: **The `finding_count` exclusion is only checked
  manually**. Assert that the plain plan after the rename shows the items
  incomplete, with no unmatched warnings.

### Correctness

**Summary**: The Rust lines up with the real APIs, and the step 3 ordering is
sound. Two classification rules misjudge on `--start`: an off-question root
note escapes plan rejection, and a pair's finding counts as valid when it
answers any outline pair. The mixed-batch fixture's stems and order cannot
occur.

**Strengths**: The code compiles against the real types. Classification is
pure and runs after `RoundPlan::of`. The step 3 ordering renames once.
Refusal takes precedence over acceptance. Absence follows the
conditional-key pattern.

**Findings**:

- 🟡 major / high: **An off-question root note is classed valid on `--start`**
  (Phase 1, `note_at`/`plan_rejects`). With empty claims the pair moves to a
  fresh stem, so no plan pair matches. Compare the note with the candidate
  question for the stem, or run `tree::derive`, and add an empty-claims test.
- 🟡 major / medium: **A pair's finding is valid if it answers any outline
  pair** (Phase 1). Valid only when the finding's own answer reallocates to
  the same stem.
- 🔵 minor / high: **The mixed-batch fixture is unreachable** (Phase 2).
  Seeding `01` and `05` makes the floor 6, and compositions rank last. Reseed
  or derive the expected values.
- 🔵 minor / medium: **The "fresh index" claim also fails for markers**.
  Restate it as "whenever a file claim holds the index".
- 🔵 minor / medium: **The quarantine obligation is lost on a second
  interruption**. Carry it forward, or document it.
- 🔵 suggestion / low: **A stale refused note left by an unfinished step 3 is
  reported as written**. Document this, or compare against `seen` digests.
- 🔵 suggestion / low: **Step 3 renames without re-checking**. Document this,
  or apply the "does not complete its pair" check.

### Safety

**Summary**: Destructive actions are small and reversible: dot-prefixed
markers with unique suffixes, a list limited to the replaced batch, and a
corrupt ledger that fails safe. The gaps are about timing: the list is
single-shot, and the classification can go stale under a live earlier run.

**Strengths**: Quarantine renames rather than deletes. The CLI is read-only
towards findings. A corrupt ledger fails safe. The ordering renames once.
The report keeps the concurrency hazard visible. The rollout never ships
neither signal.

**Findings**:

- 🟡 major / medium: **The quarantine list is single-shot** (Phase 2/3).
  Carry the paths in the new ledger, or widen the rule. Otherwise, document
  the gap and add an interrupt-before-step-3 walkthrough.
- 🔵 minor / medium: **Step 3 renames on a classification that may be stale**.
  Re-validate before renaming, or emit a digest per path.
- 🔵 minor / medium: **Quarantine is tied to a first batch that may never
  happen**. Rename after the report, whether or not anything spawns, and add a
  test.
- 🔵 minor / medium: **Outline edits turn valid findings into quarantine
  targets**. Show the cause as its own group, or say how to restore a marker.
- 🔵 suggestion / low: **Direct CLI users lose the stderr hazard signal**.

### Usability

**Summary**: A clear improvement over the concurrency-flavoured warning: a
conditional opening sentence and outcome-oriented groups. The gaps are in
reading and acting on the report: bare stems, re-offered pairs at different
stems, a recovery row that arrives too late, and docs that still bless
deleting the ledger.

**Strengths**: The opening sentence makes no claim. The group labels are
outcome-oriented. Each case gets one predictable signal. The JSON is ordered
and the paths are ready to use. The rollout never ships neither signal.

**Findings**:

- 🟡 major / high: **Docs still say deleting the ledger is safe** (Phase 4,
  Section 3). Recommend re-running, and say what deleting skips.
- 🟡 major / medium: **The row's guidance arrives only after re-running**
  (Phase 4, Section 2). Speak to the current run in the report, and put the
  wait-then-re-run guidance in the docs.
- 🟡 major / high: **The report lists bare stems** (Phase 3, Section 2).
  Show the question and profile, and the new stem for re-offered pairs.
- 🔵 minor / medium: **"offered again" may not hold in this run**. Use
  non-committal wording.
- 🔵 minor / medium: **A refused leftover note is reported twice**. Choose
  one surface, or carry the `rejected` reason.
- 🔵 suggestion / medium: **Hand CLI users lose the human-readable signal**.
  Keep a stderr-only line.
- 🔵 suggestion / low: **The valid group doesn't say what happens next**.
  Word it as "written by run {run}; kept as answered".

### Compatibility

**Summary**: The CLI-to-skill contract is handled well: an additive
conditional key, an "ignore unknown fields" skill, and a safe phase order.
The weakness is the stated 0307 boundary: the ledger reader needs every field
to decode.

**Strengths**: An additive conditional key, so existing plan shapes are
untouched. Safe intermediate releases. The ledger write format is unchanged.
Only typed values are serialised. The public API snapshot is planned. The
removal is documented.

**Findings**:

- 🟡 major / medium: **The ledger reader needs every field, so the boundary
  claim does not hold** (What We're NOT Doing / Migration Notes). Restate the
  boundary, add an unknown-field tolerance test, and require 0307's fields to
  default when absent.
- 🔵 minor / medium: **Hand runs lose the stderr signal** (Phase 3, Section
  3). Keep a stderr-only line, or document the change.
- 🔵 suggestion / medium: **Record the removal under `### Removed`**.
- 🔵 suggestion / low: **Pin that `replaced_run` leaves other keys
  unchanged**. Compare against a fresh `--start` with no ledger.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-13T16:45:37+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Architecture + Correctness + Code Quality + Test Coverage**: A pair's finding is valid if it answers any outline pair — Partially resolved (own-pair rule in place; slug collisions reintroduce it, see new issues)
- 🟡 **Correctness**: Off-question root note classed valid on `--start` — Resolved
- 🟡 **Safety + Architecture + Correctness**: Quarantine list single-shot — Resolved (accepted residual window documented)
- 🟡 **Compatibility**: Ledger reader needs every field — Resolved
- 🟡 **Usability**: Docs say deleting the ledger is safe — Partially resolved (the following sentence still offers deletion as an equal option)
- 🟡 **Usability**: "Unfinished run" row reaches the researcher too late — Resolved
- 🟡 **Usability**: Report lists bare stems — Resolved
- 🟡 **Code Quality**: Retained docstring contradicted by the plan — Resolved
- 🟡 **Test Coverage**: No test pins group labels — Resolved
- 🔵 **Correctness**: Mixed-batch fixture unreachable — Resolved (traced through planner and window)
- 🔵 **Correctness**: "Fresh index" invariant fails for markers — Resolved
- 🔵 **Safety**: Quarantine tied to a first batch — Resolved
- 🔵 **Safety**: Outline edits quarantine good findings — Still present
- 🔵 **Usability**: "offered again" over-promises — Still present (reworded, still over-promises for orphaned/skipped)
- 🔵 **Usability**: Refused leftover note reported twice — Accepted
- 🔵 **Code Quality**: hoisted `mut`, leftover `notice`, private names, implicit precedence — Resolved
- 🔵 **Architecture**: `plan_rejects` repeats `window`'s stage walk — Accepted
- 🔵 **Architecture / Safety / Correctness**: Stale quarantine under a live run — Accepted
- 🔵 **Test Coverage**: red-first claim — Partially resolved (the unknown-fields "guard" asserts `replaced_run`)
- 🔵 **Test Coverage**: order, filter guards, report position — Resolved
- 🔵 **Compatibility / Usability / Safety**: Hand CLI users lose the stderr signal — Partially resolved (line kept, hazard clause dropped)
- 🔵 **Compatibility**: `### Removed` — Resolved; pin other keys unchanged — Still present
- 🔵 **Test Coverage**: `finding_count` proxy — Partially resolved

### New Issues Introduced

- 🟡 **Architecture + Correctness**: The quarantine re-plan pushes the first plan's unspawned spawns into `attempted` — `RunLedger::record` (`conduct/ledger.rs:101-115`) counts every spawn `PendingBatch::replace` displaces as attempted. Under a saturated window the post-quarantine offer can displace never-run spawns, which are then never offered again and reported unfinished. Verified.
- 🟡 **Architecture + Correctness + Code Quality**: Own-pair resolution by slug is ambiguous — `QuestionSlug::from` collapses every non-ASCII question to `focus-area` and cuts at 60 characters, so several items reproduce one stem; validity reverts to "any" among them and `own_pair_of` labels with the first. The replaced ledger's `claims()` holds the exact binding. Verified.
- 🔵 **Correctness**: The orphaned rule outranks unwritten, so a lineage with no note in an orphaned tree (including one whose root question comes from a marker) is reported as written.
- 🔵 **Safety**: A valid finding whose stem no outline pair reproduces is quarantined and labelled invalid.
- 🔵 **Usability**: The invalid label drops "quarantined" (the AC and the skill's own term).
- 🔵 **Usability**: The trailing pointer's "them" has no clear subject when only valid spawns remain; its next step names the earlier session.
- 🔵 **Usability + Safety + Compatibility**: The stderr line drops the live-run hazard, and the skill's "ignore stderr" rule covers trims only.
- 🔵 **Compatibility**: The ledger boundary covers added fields but not reshaped ones.
- 🔵 **Code Quality**: `own_pairs` returns a positional tuple; the reported pair is resolved separately from the verdict; reverse allocation has no shared home; the quarantine rule lives in `render`.
- 🔵 **Test Coverage**: "batch advances" is loose; no CLI test renders a null pair; the unknown-fields guard cannot be green first.
- 🔵 **Safety** (suggestion): No instruction for a quarantine rename that fails partway.

### Assessment

Every major finding from pass 1 is resolved or accepted, except that slug collisions partly reintroduce the "any outline pair" rule. Two new major findings come from the revisions themselves, and each was flagged by two lenses and verified against the code. The first is the re-plan's interaction with `record`; the second is slug ambiguity in resolving a stem's own pair. Both are fixable with targeted changes: remove the re-plan by treating listed findings as quarantined inside `--start`, and resolve a stem's own pair through the replaced ledger's `claims()`. Below the threshold, so COMMENT, but both should be fixed before implementation.

## Re-Review (Pass 3) — 2026-10-13T17:16:14+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Architecture + Correctness**: The quarantine re-plan strands unspawned spawns — Resolved (quarantine read into the first plan; pinned at concurrency 1)
- 🟡 **Architecture + Correctness + Code Quality**: Own-pair resolution by slug is ambiguous — Resolved (`Outline::pair_owning` via the replaced ledger's claims; correctness traced that every offered spawn's index is claimed)
- 🔵 **Correctness**: Orphan rule outranks unwritten — Resolved
- 🔵 **Safety**: Valid finding with no owning pair quarantined — Resolved (`unasked`, left in place)
- 🔵 **Usability**: Invalid label, trailing pointer, docs deletion, stderr hazard, entry format — Resolved
- 🔵 **Usability**: "offered again" over-promises — Partially resolved (orphaned notes and unowned unwritten spawns)
- 🔵 **Compatibility**: Reshaped ledger fields; skill ignoring stderr; other keys pinned — Resolved
- 🔵 **Code Quality**: positional tuple, pair separate from verdict, reverse allocation home, quarantine rule in render, third match, naming — Resolved or partially resolved (tuples moved to the judge methods)
- 🔵 **Test Coverage**: guard labels, loose batch assertion, null pair, stderr order, no-root-question — Resolved; positive literals and `finding_count` proxy — Partially resolved
- 🔵 **Safety**: Failed-rename instruction — Partially resolved (stopping drops the rest of the list)

### New Issues Introduced

- 🟡 **Code Quality + Architecture**: "Quarantine, then plan" is composed twice — `ReplacedRun::of` draws a private plan over quarantined inputs, and `run_outstanding` rebuilds the same inputs and plan; node verdicts agree with the emitted plan only by convention, and in Phase 2 they genuinely differ.
- 🔵 **Safety + Correctness**: `unasked` keeps a misdirected finding that answers a pair the outline still asks (unowned stem, named question still present).
- 🔵 **Correctness**: A set-aside misdirected finding's `(index, question)` claim can pull its named question off an existing `.levels/` tree at a higher index.
- 🔵 **Correctness**: The concurrency-1 test needs the first `--start` at concurrency ≥ 2.
- 🔵 **Usability + Architecture**: `unasked` conflates ownership with outcome; unwritten spawns with no owner are promised "offered again"; unasked entries show a bare stem though the finding's own question is known.
- 🔵 **Usability**: Orphaned-tree notes labelled "quarantined" but never renamed.
- 🔵 **Usability + Safety**: A failed rename stops after the first failure; better to attempt every rename, then list each remaining path with its manual rename. The NOT-doing note mislabels a misdirected leftover as "visible" when it is silently counted.
- 🔵 **Usability + Safety**: The stderr instruction prints with an empty `quarantine` and omits the `.<name>.invalid` form.
- 🔵 **Usability**: The trailing line names a table row the user never sees; inline its next step.
- 🔵 **Architecture + Compatibility**: The `--start` plan and ledger assume a rename only the skill performs; ignoring `replaced_run.quarantine` is no longer safe, contrary to the docs' "ignore unknown fields" rule. State the contract.
- 🔵 **Architecture + Code Quality**: `pair_owning` couples the outline parse model to run claims; `pair_owning` and `stem_at` could be `pub(crate)`.
- 🔵 **Code Quality**: `note_at` is a guard chain dressed as a match; judge methods return tuples; `quarantining` leans on a docstring and hand-builds the marker claim; `plan_rejects` belongs on `RoundPlan`.
- 🔵 **Compatibility**: The report also depends on normalisation and slug rules staying stable; CHANGELOG `Changed`/`Removed` entries describe deltas to an unreleased command, and the existing "safe to delete" entry goes stale; 0307's note should name `claims`.
- 🔵 **Test Coverage**: The equivalence test is a no-op on its fixture; two Phase 3 tests are guards; the failing-validation fixture should keep a parseable `question`; no automated `finding_count` proxy.
- 🔵 **Usability** (suggestion): Work item 0310's requirement and ACs still describe three groups.

### Assessment

Both pass-2 majors are resolved and verified by tracing: no spawn can be stranded, and ownership is exact under slug collisions. One major remains, and it is structural rather than behavioural: the quarantine-then-plan composition lives in two places. Splitting `ReplacedRun` so the CLI plans once and passes that plan to the node judgement fixes it, and also addresses the tuple and `note_at` readability points. The rest are edge cases (unowned misdirected findings, index claims across trees), wording, and test labelling. The plan is close to ready; one more targeted revision should settle it.

## Re-Review (Pass 4) — 2026-10-13T17:43:36+00:00

Lenses: architecture, code-quality, correctness.

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Code Quality + Architecture**: "Quarantine, then plan" composed twice — Resolved (`ReplacedBatch` two-step around the CLI's one plan)
- 🔵 **Safety + Correctness**: `unasked` keeps a misdirected finding — Resolved
- 🔵 **Correctness**: Set-aside claim pulls a question off its tree — Partially resolved (in-run only; see new major)
- 🔵 **Correctness**: Concurrency-1 fixture — Resolved
- 🔵 **Architecture**: Plan/ledger assume an unstated rename — Resolved; outline coupled to claims — Resolved
- 🔵 **Architecture + Usability**: `unasked` conflates ownership with outcome — Partially resolved (refused unowned nodes and invalid unowned findings land in "offered again")
- 🔵 **Code Quality**: tuples, `note_at` shape, `run_outstanding` growth — Partially resolved; hand-built marker claim, `plan_rejects` placement — Resolved

### New Issues Introduced

- 🟡 **Correctness**: A real quarantine marker claims its frontmatter question at the old index; the planner's `claimed_by_a_file` takes the minimum file claim, so a later claims-free `--start` pulls a question that recovery moved to a fresh index (and grew a tree at) back to the marker's index, orphaning the tree. Verified against `plan.rs:292-297`. Fix: prefer a `.levels/` directory's claim over a marker's in `claim_index`, which also lets `marker_claim` drop its `held` special case.
- 🔵 **Correctness + Architecture**: Ownerless spawns classed invalid (failing or still-asked findings, refused notes) are promised "offered again"; decide ownership first for nodes too.
- 🔵 **Correctness** (suggestion): `still_asks` matches question only, so a valid finding for a dropped profile is quarantined; match question and profile.
- 🔵 **Architecture + Code Quality**: The quarantine → plan → report order is a caller convention; let `ReplacedBatch` own `quarantining(inputs)`, make `as_if_quarantined` `pub(crate)`, drop the public `quarantine()`; the docstring is false in Phase 2.
- 🔵 **Architecture**: Node ownership deferred to the second step forces a cloned `claims` and lets the two steps see different outlines; resolve every owner in the first step.
- 🔵 **Architecture + Code Quality**: `held` re-derives the planner's `file_claims`; the quarantine rule is written twice (`quarantine`/`quarantined`); `Judged::Pair` admits node spawns.
- 🔵 **Code Quality**: Judge methods return tuples of different shapes and orders (introduce a `Verdict`); `Judged::Node` holds an unjudged node; the unowned branch folds two reasons into one guard; `note_outcome`'s `refused`-in-`filter`; `Owners` names a lookup but judges; `run_outstanding` names one concept three times; `marker_claim` shadows and double-normalises.

### Assessment

The structural major from pass 3 is resolved, and correctness traced every test fixture and the borrow sequence. One behavioural major remains: a marker's question claim outranks a tree on the next claims-free plan, regressing the repeated-interruption case. The planner fix is small and simplifies `marker_claim`. The rest is API shaping (let the batch own the quarantine step, resolve owners once, one `Verdict` type) and two classification-consistency edges. One more targeted revision should settle it.

## Verdict Update — 2026-10-13T17:49:22+00:00

**Verdict:** APPROVE

The pass-4 findings were applied without a fifth pass: a tree's claim now outranks a quarantine marker's in `claim_index` (reversing `the_lowest_index_a_marker_or_levels_directory_holds_wins`), ownership is decided first for nodes, `still_asks` matches question and profile, and `ReplacedBatch` owns the judge → quarantine → report sequence with one `Verdict` type. Approved for implementation; the remaining risk is compilation, which Phase 1's first `cargo test` surfaces.
