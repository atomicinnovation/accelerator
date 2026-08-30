---
type: "work-item-review"
id: "0186-remove-exec-probe-from-bootstrap-warm-path-review-1"
title: "Work Item Review: Remove the Exec Probe from the Bootstrap Warm Path"
date: "2026-07-31T11:19:09+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0136"
target: "work-item:0186"
work_item_id: "0186"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: ["shell", "performance", "bootstrap"]
last_updated: "2026-08-01T12:51:32+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Remove the Exec Probe from the Bootstrap Warm Path

**Verdict:** REVISE

0186 is a tightly-scoped, well-evidenced task: one mechanism (`probe_dir`) in
one file (`bin/accelerator`), a measured attribution that isolates the cost
(107.9 ms fresh-write-and-exec against 10.6 ms re-exec), and a safety argument
stated inline rather than assumed. Every template section bar Drafting Notes is
substantively filled, and the criteria are unusually self-aware — AC1 names why
a residue check would be non-discriminating and pre-empts the root-bypass false
pass. Two structural gaps drive the REVISE: the item's headline objective (the
~108 ms saving) is gated only by a record-the-numbers criterion with no
threshold, so the full AC set can pass with zero latency gained; and the shim
double-hash Open Question is simultaneously mis-costed, out of scope, and the
owner of a residual that 0169's latency gate depends on.

### Cross-Cutting Themes

- **The shim double-hash Open Question is doing too much work** (flagged by:
  clarity, dependency, scope, testability) — four lenses converged on the same
  paragraph from different angles: its ~23 ms figure belongs to a different
  change than the one it describes (clarity), deferring it leaves the residual
  unowned while 0186 is declared 0169's unblocker (dependency), a "yes"
  resolution would pull a trust-boundary change into a redundancy-removal task
  (scope), and no criterion requires the decision to be recorded (testability).
- **AC1's warm-path test is under-specified and non-discriminating** (flagged
  by: clarity, testability) — warming a cache requires writing into the
  directory the criterion makes non-writable, the setup sequence is unstated,
  and a probe kept but made non-fatal would satisfy the assertion while
  retaining the full cost.
- **No target for a performance work item** (flagged by: testability,
  completeness) — the source research recommends a comparative bound against
  `hooks/vcs-guard.sh`; 0186 carried across only the measurement protocol, not
  the expected result. Sibling 0169 does set a bound (`G ≤ 1.1 × B`).
- **Test surface and harness ownership are unstated** (flagged by:
  completeness, dependency, testability) — the criteria refer to "the test" as
  an existing obligation, Requirements list only production changes, and the
  warm-cache harness the criteria presuppose is currently owned by 0169, which
  is downstream.
- **`bin/accelerator` is contested and the line references are fragile**
  (flagged by: dependency, clarity) — the hard sequencing constraint against
  in-progress 0182 lives only in prose (`relates_to`, no direction, no
  reciprocal edge, epic says "no blockers"), and several targets are pinned by
  line number alone in a file 0182 is actively editing.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability / Completeness**: Latency criterion records a number but
  defines no pass/fail threshold
  **Location**: Acceptance Criteria (warm-path saving measured and recorded)
  The only criterion covering the item's primary objective requires that
  before/after medians be "measured and recorded" — no threshold, so any pair of
  numbers satisfies it, including a zero or negative saving. The source research
  (§12) recommends a comparative bound against today's shell guard, which was
  not carried across.

- 🟡 **Dependency / Scope**: Deferring the shim double-hash leaves the residual
  warm-path cost unowned, yet 0186 is the declared unblocker of 0169's latency
  gate
  **Location**: Open Questions / Dependencies
  Removing only the probe puts the bootstrap at roughly 41 ms by §12's own
  decomposition, against 0169's `G ≤ 1.1 × B` gate (≈38.6 ms) *plus* a
  sub-binary exec and verify on top. 0169 cannot pick up the residual because it
  is downstream, and no successor item exists.

- 🟡 **Dependency**: Hard sequencing constraint against in-progress 0182 is
  recorded only as "Related", with no direction and no reciprocal edge
  **Location**: Dependencies (Related: 0182) / Frontmatter: relates_to
  The prose says the two "must be sequenced rather than developed concurrently"
  — a hard ordering constraint — but `blocked_by` is empty, which goes first is
  unstated, 0182 does not reference 0186, and parent epic 0136 annotates this
  task as "(no blockers)".

- 🟡 **Clarity / Testability**: AC1's precondition is under-specified — warming
  a cache requires writing to the directory the criterion makes non-writable
  **Location**: Acceptance Criteria (criterion 1)
  The criterion never states what makes the invocation "warm", nor how a
  non-writable directory came to hold a verified launcher. It also implicitly
  requires the warm path to be write-free, whereas Open Questions describes
  warm-path shim staging into `cache_dir` that this item deliberately leaves in
  place — so a failure would not distinguish "probe still present" from "staging
  still writes".

- 🟡 **Testability**: The warm-to-cold fall-through that justifies the removal
  is not covered by any criterion
  **Location**: Requirements (first bullet) / Acceptance Criteria
  The safety argument is that a `noexec` directory makes `verify_launcher` fail
  into the cold branch, but AC2 tests an *empty* (cold) cache dir made
  non-executable, not a *populated* cache dir that has become non-executable.
  The load-bearing claim goes untested.

- 🟡 **Testability**: No criterion verifies the cold happy path or that
  `ensure_dir` still creates the cache dir
  **Location**: Requirements (second bullet) / Acceptance Criteria
  Every criterion covers the warm path (AC1, AC4) or a cold *failure* (AC2);
  none asserts that a first run against a non-existent cache dir still creates
  the directory, probes, fetches and succeeds — the most likely regression from
  the split.

- 🟡 **Dependency**: The warm-path behavioural criterion implies a
  launcher-build test harness that the downstream item (0169) owns
  **Location**: Acceptance Criteria (warm-path behavioural check)
  A warm invocation presupposes a populated cache with a verified launcher and
  shim, i.e. a harness able to build and stage the CLI. 0169 records that this
  wiring does not yet exist and needs `build:cli:dev` plus an
  `accelerator_env()`-style helper — and 0169 is downstream, so the capability
  cannot be inherited from it.

- 🟡 **Clarity**: The shim-hash question conflates two different changes and
  quotes the saving for the larger one
  **Location**: Open Questions
  The question asks about removing the *second* `sha256_file` call and states
  "~23 ms", but §12 attributes ~23 ms to `sha256_file` **×2** at ~11.7 ms each,
  and the change it proposes for that saving is skipping shim staging when
  `shim_source`'s directory and `cache_dir` coincide — not deleting one hash.
  Two readings, materially different payoffs.

#### Minor

- 🔵 **Clarity**: First two requirements disagree on whether `probe_dir` is
  removed or split
  **Location**: Requirements
  Requirement 1 says "Remove the exec probe (`probe_dir`, …)"; Requirement 2
  says "Split `probe_dir` into `ensure_dir` … and the write-chmod-exec probe".
  A reader skimming only Requirement 1 (or the Summary) could delete `probe_dir`
  outright and lose the `mkdir -p` the warm path still needs.

- 🔵 **Completeness**: No Drafting Notes section, despite several interpretive
  calls baked into the item
  **Location**: Drafting Notes (absent)
  146 other work items carry the section. The redundancy rationale, the
  retain-rather-than-delete choice for the cold-path probe, and the shim
  deferral all read as settled facts rather than author calls open to challenge.

- 🔵 **Clarity / Testability**: "The existing diagnostic text" is neither quoted
  nor located, and AC2 has no privilege precondition
  **Location**: Acceptance Criteria (criterion 2)
  A substring assertion against an unnamed message cannot distinguish the
  intended probe diagnostic from an unrelated write/traversal error — removing a
  directory's execute bit also blocks writing inside it. Unlike AC1, AC2 states
  no `id -u` guard even though root traverses regardless of the execute bit.

- 🔵 **Clarity**: Line-number referents will not resolve once the sequenced
  sibling change lands
  **Location**: Requirements / Technical Notes / Open Questions
  Most citations pair a line range with a function name and survive drift, but
  `:256` and `:352` are identified by line alone — in the same file 0182 is
  editing.

- 🔵 **Completeness / Testability**: Requirements omit the test work the
  criteria presuppose, and no suite is named
  **Location**: Requirements / Acceptance Criteria
  Requirements list only production changes while two criteria refer to "the
  test" as an existing obligation. Neither names the target suite, so the new
  coverage may land where CI does not pick it up — and a passing one-off check
  would not prevent the probe being reintroduced later.

- 🔵 **Dependency**: CI execution-environment coupling behind the
  permission-based tests is not captured
  **Location**: Acceptance Criteria (root hard-fail; `chmod -x` cold-path check)
  Neither the non-root runner identity nor permission-honouring filesystem
  semantics is recorded as a dependency. If any lane runs as root (a common
  container default), the deliberate hard-fail turns a correct implementation
  into a red build.

- 🔵 **Dependency**: Upstream provenance of the bootstrap being modified is not
  named
  **Location**: Dependencies / Requirements
  `probe_dir`, `resolve_cache_dir`, the staged verify shim and the cold/warm
  branch structure come from 0164 (fetch-verify-cache) and 0167 (invocation
  contract); neither id appears. 0169 records the same distinction as
  "completed dependencies (not blocking)".

- 🔵 **Scope**: The shim double-hash Open Question can pull a second,
  independent concern into the item
  **Location**: Open Questions
  No Requirement or Acceptance Criterion covers the change, yet Validation
  Results pre-commits to recording a decision. If it resolves "yes"
  mid-implementation, a task-sized redundancy removal grows a security-relevant
  change — the same bundling that justified extracting 0186 from 0169.

- 🔵 **Testability**: AC1 is satisfiable by a probe made non-fatal rather than
  removed
  **Location**: Acceptance Criteria (criterion 1)
  An implementation that keeps the write-chmod-exec probe on the warm path but
  tolerates its failure also satisfies AC1, retaining the full ~108 ms. Combined
  with the missing latency threshold, no criterion actually pins "the probe does
  not run on the warm path".

#### Suggestions

- 🔵 **Testability**: The shim double-hash decision is not bound by any
  acceptance criterion
  **Location**: Open Questions / Validation Results
  The item can close with every criterion ticked while the trust-boundary
  decision remains unrecorded.

- 🔵 **Clarity**: Project shorthands used without a definition or pointer
  **Location**: Summary / Context / Requirements
  Warm path / cold path — the central organising concept — is never defined and
  is named four ways ("warm path", "warm cache", "cold path", "cold branch").
  "Fetch-verify-cache design" and "review-2, pass 4" have no pointers, unlike
  the bash 3.2 floor which is properly linked to ADR-0049.

- 🔵 **Clarity**: Bare "the probe" collides with the sibling story's reserved
  terminology
  **Location**: Requirements / Technical Notes
  0169 reserves "probe layer" for the shell VCS detection functions and states
  it is never abbreviated to "probe".

### Strengths

- ✅ The Context table plus the attribution note make the causal claim
  unambiguous: 107.9 ms against 10.6 ms isolates first-exec cost from filesystem
  work, and the ~97 ms delta is arithmetically consistent with both rows and the
  Summary's ~108 ms.
- ✅ Requirements state not just what to change but why it is safe — the warm
  path already execs the shim and launcher from `cache_dir`, and
  `resolve_cache_dir` has no fallback, so moving the probe changes only where
  the diagnostic fires. This pre-empts the obvious reader objection.
- ✅ AC1 explicitly identifies why the obvious check (probe residue) is
  non-discriminating and substitutes a behavioural test, and pre-empts the
  root-bypass false pass with a hard-fail rather than a skip.
- ✅ AC2 rules out silent skipping — an unrunnable check becomes a recorded
  manual check rather than a disappeared one.
- ✅ AC3 names the concrete gates (`scripts/lint-bashisms.sh`, shfmt,
  ShellCheck) and AC4 fixes a reproducible measurement protocol (median of 20,
  one darwin-arm64 host, single session, no build running).
- ✅ Single coherent purpose in one file: the two substantive Requirements are
  the removal and the mechanical split that enables it; bash 3.2 is a constraint
  rather than a bundled concern.
- ✅ The extraction rationale is stated and defensible (bash bootstrap under the
  3.2 floor, not the Rust CLI) and the item has standalone value today via
  `hooks/config-detect.sh`, independent of 0169.
- ✅ Clean measurement boundary with the downstream story — 0186 records an
  absolute before/after median while 0169 retains the host-relative parity
  threshold; no duplicated or competing criterion.
- ✅ The `Blocks` edge is stated with its reason and is reciprocated in 0169's
  `blocked_by`; provenance is fully traced to the research section carrying the
  measurements.
- ✅ Every template section bar Drafting Notes is present and substantively
  populated with no placeholder content, and Validation Results pre-declares a
  slot for each pending verification.
- ✅ The platform coupling is captured as an explicit Assumption (macOS
  first-exec penalty, no Linux equivalent), pre-empting a false expectation of
  equivalent savings on the other shipped platform.

### Recommended Changes

1. **Set a numeric latency gate on AC4** (addresses: "Latency criterion records
   a number but defines no pass/fail threshold"; "AC1 is satisfiable by a probe
   made non-fatal")
   Replace "measured and recorded" with a threshold — e.g. after-median at or
   below 60 ms on darwin-arm64 with a before-minus-after delta of at least
   80 ms — and name the measurement tool so both numbers are produced
   identically. This also closes the non-fatal-probe loophole numerically.

2. **Resolve the shim double-hash question now, before implementation**
   (addresses: "Deferring the shim double-hash leaves the residual warm-path
   cost unowned"; "The shim-hash question conflates two different changes";
   "Open Question can pull a second, independent concern into the item"; "The
   shim double-hash decision is not bound by any acceptance criterion")
   Split the two options with their correct figures (drop `:256` only ≈ 11.7 ms;
   skip staging when `shim_source`'s directory and `cache_dir` coincide ≈ 23 ms),
   then either declare it out of scope and raise the successor item now — noting
   in Dependencies that 0186 alone may not clear 0169's `≤ 1.1 × B` gate — or
   promote it into Requirements with its own criterion. Do not leave it as a
   discretionary mid-implementation call.

3. **Encode the 0182 ordering as a real edge** (addresses: "Hard sequencing
   constraint against in-progress 0182 is recorded only as Related";
   "Line-number referents will not resolve once the sequenced sibling change
   lands")
   State the direction in prose ("0182 lands first; this task rebases onto
   it"), move 0182 into `blocked_by` if it must precede, add the reciprocal
   reference on 0182, correct epic 0136's "(no blockers)" annotation, and pin
   every code citation to an enclosing function name so the line numbers are
   indicative rather than load-bearing.

4. **Rewrite AC1 as an explicit sequence** (addresses: "AC1's precondition is
   under-specified"; "First two requirements disagree on whether `probe_dir` is
   removed or split")
   Spell out: warm the cache with the directory writable, `chmod -w`, then
   assert the second invocation exits 0 with the expected `version` output.
   State whether the warm path is expected to be write-free after the change, or
   scope the assertion to probe-specific writes if staging may still write.
   While there, make Requirement 1 speak only about behaviour (the warm path
   performs no write-chmod-exec probe) and let Requirement 2 own the mechanism.

5. **Add the two missing criteria the split demands** (addresses: "The
   warm-to-cold fall-through … is not covered"; "No criterion verifies the cold
   happy path or that `ensure_dir` still creates the cache dir")
   One for a *populated* cache dir subsequently made non-executable, asserting
   the same `noexec` diagnostic (proving the fall-through into the cold branch);
   one for a non-existent `ACCELERATOR_CACHE_DIR`, asserting the directory is
   created and `version` exits 0.

6. **Name the test surface and its prerequisites** (addresses: "Requirements
   omit the test work the criteria presuppose"; "warm-path behavioural criterion
   implies a launcher-build test harness that 0169 owns"; "CI
   execution-environment coupling … is not captured")
   Add a Requirements bullet for the new bootstrap coverage naming the target
   suite and how it is reached from `mise run`; record in Dependencies what the
   harness needs to pre-warm the cache (`build:cli:dev` plus an
   `accelerator_env()`-style helper, or a stub launcher/shim) and whether that
   wiring is a prerequisite or part of this task; and state the
   execution-environment requirement (non-root, permission-honouring
   filesystem) plus what happens on lanes that cannot meet it.

7. **Quote the `noexec` diagnostic substring and add AC2's root guard**
   (addresses: "The existing diagnostic text is neither quoted nor located")
   Reproduce the exact substring, assert the specific non-zero exit code, and
   apply AC1's non-root precondition and hard-fail-if-root rule to AC2 as well.

8. **Add a Drafting Notes section and define the warm/cold vocabulary**
   (addresses: "No Drafting Notes section"; "Project shorthands used without a
   definition or pointer"; "Bare 'the probe' collides with the sibling story's
   reserved terminology"; "Upstream provenance … is not named")
   List the interpretations made during extraction; define warm path and cold
   path once in terms of whether a verified launcher is cached and settle on one
   name for each; keep the "exec probe" qualifier on every mention; add
   References pointers for the fetch-verify-cache design and the 0169 review-2
   document; and add a "Completed dependencies (not blocking)" line naming 0164
   and 0167 with their current states.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually precise for its size: the problem, the
measured attribution, and the reason the probe is safe to move are all stated
explicitly, and most technical referents are pinned to a named function plus
line range. The main clarity weaknesses are an under-specified precondition in
the first acceptance criterion (what makes the asserted invocation "warm" when
the cache dir is deliberately non-writable), and an Open Question whose stated
saving and stated change do not match the source it derives from — leaving two
different interpretations of what "remove the second hash" means. Remaining
issues are smaller: a remove-vs-split tension between the first two
requirements, an unquoted "existing diagnostic text" referent, line-number
citations the work item itself says a sibling change will invalidate, and a few
project shorthands used without a pointer.

**Strengths**:

- The Context table plus the "Attribution evidence" note in Technical Notes make
  the causal claim unambiguous: 107.9 ms versus 10.6 ms isolates first-exec cost
  from filesystem work, and the ~97 ms delta in the Context prose is
  arithmetically consistent with both rows and with the ~108 ms figure in the
  Summary.
- Requirements do not just state what to change but why it is safe (the warm
  path already execs the shim and launcher from the cache dir;
  `resolve_cache_dir` has no fallback so moving the probe changes only where the
  diagnostic fires), pre-empting the obvious reader objection.
- The first acceptance criterion explicitly names and rules out two ways the
  test could pass vacuously — checking for probe residue, and running as root —
  which is far more precise than most criteria of this kind.
- The Open Question carries an explicit default resolution ("take the exec-probe
  win and leave this"), so an unresolved question cannot stall delivery.
- The Dependencies section states the concrete sequencing hazard (0182 edits the
  same file) rather than just listing an edge, and the Assumptions section names
  darwin as the deliberate worst case rather than leaving the platform scope
  implicit.

**Findings**:

- 🟡 **major** / confidence: high — **"Warm invocation" against a non-writable
  cache dir leaves the test precondition ambiguous**
  *Location*: Acceptance Criteria (criterion 1)
  Criterion 1 says to point `ACCELERATOR_CACHE_DIR` at a directory that is
  executable but **not writable** and assert that a warm invocation still
  succeeds — but the work item never states what makes an invocation "warm", nor
  how a non-writable directory came to hold a verified launcher. The Technical
  Notes and Open Questions sections both say the warm path
  stages/content-addresses the verify shim *into* `cache_dir`, which reads as a
  write, so it is unclear whether the intended test is (a) populate the cache,
  then `chmod -w`, relying on staging being a no-op when the staged file already
  exists, or (b) something else entirely.
  **Impact**: An implementer who reads it the wrong way either writes a test
  that fails for reasons unrelated to the exec probe, or concludes the criterion
  is unsatisfiable and quietly weakens it — in both cases the headline
  behavioural guarantee goes unverified.
  **Suggestion**: State the precondition explicitly (what must already exist in
  `cache_dir` before the `chmod`, and in which order) and state whether the warm
  path is expected to be write-free after removal of the probe.

- 🟡 **major** / confidence: medium — **The shim-hash question conflates two
  different changes and quotes the saving for the larger one**
  *Location*: Open Questions
  The Open Question asks whether the verify shim's **second** `sha256_file` call
  (`bin/accelerator:256`) should be removed and says "it costs a further ~23 ms
  per invocation". The cited source
  (`meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  §12) attributes ~23 ms to `sha256_file` **×2** (`:252` and `:256`) at
  ~11.7 ms per call, and the change it proposes for that saving is skipping shim
  staging entirely when `shim_source`'s directory and `cache_dir` resolve to the
  same path — not deleting one hash. So the question admits two readings with
  materially different payoffs (~11.7 ms for one hash, ~23 ms for skipping
  staging).
  **Impact**: Since the question is framed as a cost/benefit trade-off against a
  trust-boundary concern, a saving stated at roughly double its actual value
  could flip the decision, and the ambiguity about which change is being
  contemplated means a "yes" answer does not tell the implementer what to build.
  **Suggestion**: Separate the two options and attach the correct per-option
  figure (drop `:256` only ≈ 11.7 ms; skip staging when source and cache
  directories coincide ≈ 23 ms), and state whether a "yes" resolution is
  implemented inside this work item or handed to a follow-up — currently only
  the *decision* appears in Validation Results, with no requirement or criterion
  covering the change itself.

- 🔵 **minor** / confidence: high — **First two requirements disagree on whether
  `probe_dir` is removed or split**
  *Location*: Requirements
  Requirement 1 says to "Remove the exec probe (`probe_dir`,
  `bin/accelerator:166-180`) from the warm path", naming `probe_dir` as the
  thing removed; Requirement 2 then says to "Split `probe_dir` into `ensure_dir`
  … and the write-chmod-exec probe (cold path only …)". Read together the intent
  is recoverable — the probe portion moves rather than disappearing — but the
  two bullets describe the same function's fate in incompatible verbs, and "the
  exec probe" is used for both the whole function and just its write-chmod-exec
  portion.
  **Impact**: A reader skimming only Requirement 1 (or the Summary's "Remove it
  from the warm path") could delete `probe_dir` outright and lose the `mkdir -p`
  that the warm path still needs.
  **Suggestion**: Make Requirement 1 speak only about the *behaviour* (the warm
  path performs no write-chmod-exec probe) and let Requirement 2 own the
  mechanism, using one consistent name for the write-chmod-exec step throughout.

- 🔵 **minor** / confidence: medium — **"The existing diagnostic text" is not
  reproduced or located**
  *Location*: Acceptance Criteria (criterion 2)
  Criterion 2 requires that a cold invocation against a non-executable cache dir
  "exits non-zero with the existing diagnostic text, asserted as a substring",
  but the text is neither quoted nor pinned to a line in `bin/accelerator`, and
  no other section reproduces it.
  **Impact**: The reader cannot tell which string the assertion will match, so
  the criterion's meaning depends on the implementer's search of the current
  source — and if the message is reworded during the change, nothing in the work
  item reveals that the criterion has drifted.
  **Suggestion**: Quote the substring to be asserted (or cite its exact
  location) so the criterion states one specific, checkable expectation.

- 🔵 **minor** / confidence: medium — **Line-number referents will not resolve
  once the sequenced sibling change lands**
  *Location*: Requirements / Technical Notes / Open Questions
  The work item pins its targets to line numbers in `bin/accelerator`
  (`:166-180`, `:184-193`, `:256`, `:310-312`, `:352`) while its own
  Dependencies section states that 0182 "also edits `bin/accelerator`" and that
  the two changes must be sequenced. Most citations pair a line range with a
  function name and so survive drift, but `:256` ("the verify shim's second
  `sha256_file` call") and `:352` ("execs the launcher binary") are identified
  by line alone.
  **Impact**: Whichever change lands second, the bare line references point at
  unrelated code, and a reader has to guess which construct was meant.
  **Suggestion**: Name the enclosing function or the distinguishing code for
  every citation (as the other references already do) and mark the line numbers
  as indicative-at-time-of-writing.

- 🔵 **minor** / confidence: medium — **Pronoun chain in the Context's closing
  paragraph shifts referent mid-passage**
  *Location*: Context
  In "Every SessionStart hook pays this today via `hooks/config-detect.sh`, and
  every future CLI-backed hook will. It was extracted from 0169 (review-2, pass
  4) as an independently deliverable change …", "this" refers to the ~108 ms
  cost while the immediately following "It" refers to the work item (or the
  change), and the final "it touches the bash bootstrap" refers to the change
  again — three referents across two sentences with no explicit subject
  re-stated.
  **Impact**: A reader can briefly parse "It was extracted from 0169" as the
  exec probe or the latency cost being extracted, which momentarily inverts the
  sentence's meaning.
  **Suggestion**: Name the subject once at the start of the extraction sentence
  (e.g. "This work item was extracted from 0169 …") so no pronoun spans the
  topic change.

- 🔵 **suggestion** / confidence: medium — **Project shorthands used without a
  definition or pointer**
  *Location*: Summary / Context / Requirements
  Several terms carry load-bearing meaning but have no definition or link in the
  work item: the **warm path / cold path** distinction (the central organising
  concept, never defined — and referred to variously as "warm path", "warm
  cache", "cold path" and "cold branch"); the **"fetch-verify-cache design"** (a
  design named in 0164/0165, referenced only in passing); and **"review-2, pass
  4"** (the review that drove the split, whose file is listed in 0169's
  References but not in this item's). By contrast the bash 3.2 floor is properly
  linked to ADR-0049, which shows the intended standard.
  **Impact**: A competent developer new to the launcher work has to reconstruct
  what makes a path warm — the very axis on which every requirement and
  criterion is stated — and cannot reach the review that justifies the item's
  existence.
  **Suggestion**: Define warm path and cold path once (in terms of whether a
  verified launcher is already cached), settle on one name for each, and add
  pointers for the fetch-verify-cache design and the review-2 document in
  References.

- 🔵 **suggestion** / confidence: low — **Bare "the probe" collides with the
  sibling story's reserved terminology**
  *Location*: Requirements / Technical Notes
  This item shortens "exec probe" to "the probe" in Requirements ("where the
  probe belongs", "the probe's result never chooses a directory") and Technical
  Notes ("the synthetic probe"), while 0169 — the story this item blocks and
  whose review produced it — reserves "probe layer" for the shell VCS detection
  functions in `scripts/vcs-common.sh` and states it is "never abbreviated to
  'probe'".
  **Impact**: Low risk in isolation, since this item touches only
  `bin/accelerator`, but a reader moving between the two items in the same epic
  can read "the probe" as the VCS probe layer.
  **Suggestion**: Keep the qualifier — "exec probe" — on every mention, matching
  0169's terminology discipline.

### Completeness

**Summary**: 0186 is a densely populated task work item: every section from the
house work-item template is present and substantively filled bar one, with no
placeholder text, and the Context carries a measured latency table plus
attribution evidence that makes the motivation self-evident. Kind-appropriate
content for a task is fully satisfied — the work to be done is defined to the
level of specific functions and line ranges, with the safety rationale for the
removal stated inline, and the single Open Question ships with an explicit
default-if-unresolved. The only structural gaps are the omitted Drafting Notes
section (present in 146 other work items in this corpus) and the absence in
Requirements of the test/verification work that the Acceptance Criteria clearly
presuppose.

**Strengths**:

- Every template section except Drafting Notes is present and substantively
  populated — no placeholder or stub content anywhere, unusual for a task-kind
  item.
- Context does real explanatory work: a measured 6-row latency table
  (darwin-arm64, warm cache, 20 iterations, dated), the isolating comparison
  that attributes the cost to macOS first-exec, and the reason it matters at
  scale (every SessionStart hook, every future CLI-backed hook).
- Requirements are implementer-ready for a task: exact call sites
  (`bin/accelerator:166-180`, `:184-193`), the concrete refactor shape
  (`probe_dir` split into `ensure_dir` plus a cold-path probe), and the
  justification that moving the probe changes only where the diagnostic fires
  because `resolve_cache_dir` has no fallback.
- The Open Question about the shim's second `sha256_file` call states its cost,
  its security rationale, and an explicit "Default if unresolved" — so the item
  can proceed without blocking, and Validation Results reserves a slot for the
  resulting decision.
- Acceptance Criteria anticipate their own false-pass modes (behavioural check
  instead of probe-residue inspection; hard-fail rather than skip when run as
  root; manual-check fallback recorded rather than silently skipped).
- Frontmatter is complete and internally coherent — `kind: task`,
  `status: ready`, `priority: high`, plus typed linkage
  (`parent: work-item:0136`, `blocks: work-item:0169`,
  `relates_to: work-item:0182`) that matches 0169's `blocked_by` list; empty
  template slots are correctly omitted.
- Validation Results pre-declares exactly the three artefacts the Acceptance
  Criteria demand (before/after medians with host and OS, cold-path check mode,
  shim decision), so post-implementation recording has a fixed home.

**Findings**:

- 🔵 **minor** / confidence: high — **No Drafting Notes section, despite several
  interpretive calls baked into the item**
  *Location*: Drafting Notes (absent)
  This work item omits the `## Drafting Notes` section that the work-item
  template defines and that 146 other work items in `meta/work/` carry, even
  though it embeds several drafting judgements a reviewer would want surfaced —
  that the warm-path probe is redundant because the shim and launcher execs are
  stronger exec tests, that the shim double-hash question defaults to "leave
  it", and that the 0182 overlap is a sequencing rather than a blocking
  relationship.
  **Impact**: Those interpretations read as settled facts rather than as author
  calls open to challenge, so a reviewer who disagrees with (for example) the
  redundancy argument has no designated place to see it flagged as an assumption
  made while drafting.
  **Suggestion**: Add a `## Drafting Notes` section listing the interpretations
  made during extraction from 0169 — chiefly the warm-path redundancy rationale,
  the choice to retain rather than delete the cold-path probe, and the decision
  to defer the shim double-hash change.

- 🔵 **minor** / confidence: medium — **Requirements omit the test work that the
  Acceptance Criteria presuppose**
  *Location*: Requirements
  The Requirements section lists only production changes (remove the probe from
  the warm path, split `probe_dir` into `ensure_dir` plus a cold-path probe,
  stay bash-3.2-safe), while two Acceptance Criteria refer to "the test" as an
  existing obligation — a non-writable-cache-dir warm invocation test with a
  root guard, and a `chmod -x` cold-path diagnostic test — and neither names
  which suite these belong in or whether they are new.
  **Impact**: An implementer reading Requirements alone would scope the change
  as a refactor and discover the test-authoring work only when checking off
  criteria, and without a named suite the new coverage may land somewhere CI
  does not pick it up.
  **Suggestion**: Add a Requirements bullet covering the new bootstrap test
  coverage and name the target suite (or state that the criteria are satisfied
  by extending an existing bootstrap test script), so the test surface is part
  of the defined work rather than implied by the criteria.

- 🔵 **suggestion** / confidence: medium — **The latency-bound criterion
  recommended by the source research was not carried across**
  *Location*: Acceptance Criteria
  The referenced research
  (`meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  §12) recommends a criterion bounding warm-path per-call latency against
  today's `hooks/vcs-guard.sh` on the same host; 0186's corresponding criterion
  only requires that before/after medians be "measured and recorded" in
  Validation Results, with no target stated anywhere in the item.
  **Impact**: The item's stated purpose is a performance fix, yet a reader
  cannot tell from the work item what result would count as success — an outcome
  that recorded no improvement would still tick every box, and 0169 (which this
  blocks) measures its own latency criterion against the assumption that this
  landed a real saving.
  **Suggestion**: Record the expected outcome explicitly — either the source's
  comparative bound (warm invocation no slower than the shell guard it replaces)
  or a concrete target derived from the Context table (e.g. warm
  `bin/accelerator version` median at or below ~50 ms on darwin-arm64). Whether
  the criterion is measurable in the abstract is the testability lens's concern;
  the completeness gap is that no expected result is captured at all.

### Dependency

**Summary**: The work item captures its most important edges explicitly and with
reasons: it blocks 0169 (reciprocated in 0169's `blocked_by`), names parent epic
0136, and records the same-file collision with in-progress 0182 — plus a
platform Assumption that bounds the measurement. The gaps are all about
schedulability rather than awareness: the 0182 collision is recorded as
`relates_to` with no direction and no reciprocal edge (the epic spine even lists
0186 as "no blockers"), the Open Question's default resolution leaves ~23 ms of
the addressable warm-path cost unowned while 0186 is declared the unblocker of
0169's latency gate, and the warm-path behavioural criterion implies a
test-harness prerequisite that 0169 — the downstream item — currently owns.

**Strengths**:

- The Blocks entry is stated with its reason ("0169 — its warm-call latency
  criterion measures against an already-fixed bootstrap") rather than as a bare
  id, and the edge is reciprocated: 0169's frontmatter lists 0186 in
  `blocked_by`.
- The same-file collision with 0182 is noticed and articulated at the function
  level ("the two changes touch different functions but the same file"), which
  is exactly the kind of coupling that usually goes unrecorded.
- Provenance is fully traced — extracted from 0169 (review-2, pass 4), parented
  to epic 0136, and pointed at the specific research section (§12) carrying the
  measurements, so the dependency graph can be reconstructed from the record.
- The platform coupling is captured as an explicit Assumption (macOS first-exec
  penalty, no Linux equivalent), pre-empting a false expectation of equivalent
  savings on the other shipped platform.
- The Open Question carries a default-if-unresolved and an instruction to record
  the decision, so the adjacent shim-hash concern cannot silently vanish.

**Findings**:

- 🟡 **major** / confidence: high — **Deferring the shim double-hash leaves the
  residual warm-path cost unowned, yet 0186 is the declared unblocker of 0169's
  latency gate**
  *Location*: Open Questions
  This task declares it blocks 0169 specifically because "its warm-call latency
  criterion measures against an already-fixed bootstrap", but the Open
  Question's stated default is to "take the exec-probe win and leave" the shim's
  second `sha256_file` (~23 ms). By the referenced research's own decomposition
  (§12: 149.1 ms total, ~108 ms probe, ~23 ms double hash, ~131 ms of 149 ms
  addressable → ~18 ms), removing only the probe leaves the bootstrap at roughly
  41 ms, while 0169's criterion requires a warm `accelerator vcs guard` at
  ≤ 1.1 × the 35.1 ms shell guard (≈38.6 ms) *plus* a sub-binary exec and verify
  on top. If the default is taken, no work item owns the residual — 0169 cannot
  pick it up because 0169 is downstream of this task.
  **Impact**: 0169 could be scheduled as unblocked and then fail its acceptance
  gate on latency, with the remaining fix having no owner, no id, and no place
  in the epic spine — the exact hidden blocker this dependency record exists to
  prevent.
  **Suggestion**: Either state in Dependencies that 0186 alone is insufficient
  for 0169's ≤1.1× gate and name the follow-up that owns the ~23 ms (a new id,
  or an explicit note that 0169 must relax the criterion), or promote the
  shim-hash fix into this task's Requirements so the Blocks claim is honest as
  written.

- 🟡 **major** / confidence: high — **Hard sequencing constraint against
  in-progress 0182 is recorded only as "Related", with no direction and no
  reciprocal edge**
  *Location*: Dependencies (Related: 0182) / Frontmatter: relates_to
  Dependencies states that 0182 (in-progress) also edits `bin/accelerator` and
  that "they must be sequenced rather than developed concurrently" — a hard
  ordering constraint — yet it is captured only as
  `relates_to: ["work-item:0182"]` with `blocked_by` empty and no statement of
  which goes first. The constraint is also invisible from the other side and
  from above: 0182's frontmatter does not list 0186 in its `relates_to`, and
  parent epic 0136 annotates this task as "0186 — Remove the Exec Probe from the
  Bootstrap Warm Path *(no blockers)*". The Requirements and Technical Notes
  additionally pin behaviour to specific line ranges in that same file
  (`:166-180`, `:184-193`, `:310-312`, `:352`), which 0182's in-flight edits
  will shift.
  **Impact**: A scheduler reading the frontmatter or the epic spine will treat
  this task as startable immediately, so the two `bin/accelerator` changes can
  be picked up in parallel and collide — and the line references the implementer
  is told to work from may already be stale.
  **Suggestion**: State the direction explicitly (e.g. "0182 lands first; this
  task rebases onto it") and encode it as a real edge — add 0182 to `blocked_by`
  (or the equivalent sequencing field) if it must precede, add the reciprocal
  reference on 0182, and correct the epic's "(no blockers)" annotation.

- 🟡 **major** / confidence: medium — **The warm-path behavioural criterion
  implies a launcher-build test harness that the downstream item (0169) owns**
  *Location*: Acceptance Criteria (warm-path behavioural check)
  The first acceptance criterion requires exercising a *warm* invocation with
  `ACCELERATOR_CACHE_DIR` pointed at an executable-but-not-writable directory —
  which presupposes a populated cache containing a verified launcher binary and
  shim, i.e. a test harness able to build and stage the CLI. The referenced 0169
  records that this wiring does not yet exist ("Land the
  `test:integration:hooks` launcher edge before repointing the parity gate — it
  cannot run against a binary until the task gains `build:cli:dev` and
  `accelerator_env()`"), and 0169 is downstream of this task, so the capability
  cannot be inherited from it. No upstream prerequisite, owning suite, or
  stubbing strategy is named anywhere in this work item.
  **Impact**: The headline acceptance criterion may be unimplementable when the
  task is picked up, forcing either an unplanned harness build inside a task
  scoped as a bash micro-change, or a silent downgrade of the criterion to a
  manual check.
  **Suggestion**: Name in Dependencies which suite hosts this test and what it
  needs to pre-warm the cache (`build:cli:dev` plus an `accelerator_env()`-style
  helper, or a stub launcher/shim), and record whether that wiring is a
  prerequisite of this task or is delivered by it.

- 🔵 **minor** / confidence: medium — **CI execution-environment coupling behind
  the permission-based tests is not captured**
  *Location*: Acceptance Criteria (root hard-fail; `chmod -x` cold-path check)
  Both behavioural criteria depend on properties of the environment the suite
  runs in: the warm-path check asserts `id -u` ≠ 0 and "hard-fails rather than
  skips if run as root", and the cold-path check needs `chmod -x` on a cache
  directory to be honoured unprivileged. Neither the CI runner identity
  (non-root) nor the filesystem's permission semantics is recorded as a
  dependency — the item only hedges that the cold-path check may become "a
  local/manual check".
  **Impact**: If any lane runs the suite as root (a common container default),
  the deliberate hard-fail turns a correct implementation into a red build, and
  the failure will look like a defect in the change rather than an environment
  coupling.
  **Suggestion**: Add a line to Dependencies naming the execution-environment
  requirement (non-root runner, permission-honouring filesystem) for the lanes
  these tests will run in, and state what happens on lanes that cannot meet it.

- 🔵 **minor** / confidence: medium — **Upstream provenance of the bootstrap
  being modified (fetch-verify-cache, invocation contract) is not named**
  *Location*: Dependencies / Requirements
  The Requirements and Technical Notes depend on machinery this task did not
  create — `probe_dir`, `resolve_cache_dir`, the staged verify shim,
  `verify_launcher` and the cold/warm branch structure all come from the
  fetch-verify-cache work (0164) and the bootstrap invocation contract (0167) —
  yet `blocked_by` is empty and neither id appears in Dependencies. The
  referenced 0169 notes that 0167's code has landed on `main` while its work
  item is still `ready`.
  **Impact**: A reader cannot tell from this record whether the contract being
  edited is settled or still moving, and the "redundant once a verified launcher
  binary is cached" argument rests on upstream design decisions with no
  traceable owner.
  **Suggestion**: Add a "Completed dependencies (not blocking)" line naming 0164
  and 0167 with their current states, mirroring how 0169 records the same
  distinction, so the provenance of the modified behaviour is visible without
  reading the parent story.

### Scope

**Summary**: 0186 is a tightly-scoped, coherent unit of work: one mechanism
(`probe_dir`) in one file (`bin/accelerator`), one measured justification
(~108 ms of a 149 ms warm invocation), and Summary/Requirements/Acceptance
Criteria that all describe the same change. The extraction from 0169 is
well-argued on risk-profile grounds (bash bootstrap under the 3.2 floor vs. Rust
CLI), it has standalone value today via `hooks/config-detect.sh`, and `task` is
an appropriate kind given sibling conventions under epic 0136. The only scope
softness is the shim double-hash Open Question, which opens a route for a
second, independent concern (a trust-boundary change) to enter the item in
flight, and leaves the adjacent ~23 ms unowned if defaulted.

**Strengths**:

- Single coherent purpose: all three Requirements serve one change — the two
  substantive ones are the removal and the mechanical `ensure_dir`/probe split
  that enables it, and the third (bash 3.2) is a constraint rather than a
  bundled concern.
- Summary, Requirements and Acceptance Criteria describe the same scope; no
  section introduces work the others do not anticipate.
- Explicit in-scope/out-of-scope boundary: the probe is retained on the cold
  path rather than deleted, and the Technical Notes state why, so the reader can
  say precisely what is and is not being changed.
- The extraction rationale is stated and defensible — Context names the source
  (0169, review-2 pass 4) and the distinguishing risk profile (bash bootstrap
  under the 3.2 floor, not the Rust CLI), and parent epic 0136 corroborates it
  as independently deliverable.
- Genuine standalone value despite small size: every SessionStart hook pays the
  cost today via `hooks/config-detect.sh`, so the item delivers benefit without
  waiting on 0169.
- Clean measurement boundary with the downstream story: 0186 records an absolute
  before/after warm median, while 0169 retains the host-relative parity
  threshold (G ≤ 1.1 × B) — no duplicated or competing latency criterion.
- Confined to one file and one owning surface — no cross-service or cross-team
  span, and the file-level collision with in-progress 0182 is disclosed as a
  sequencing constraint rather than left implicit.

**Findings**:

- 🔵 **minor** / confidence: medium — **Shim double-hash Open Question can pull
  a second, independent concern into the item**
  *Location*: Open Questions
  Work item 0186 is scoped to removing the exec probe from `bin/accelerator`'s
  warm path, but its single Open Question asks whether the verify shim's second
  `sha256_file` call (`bin/accelerator:256`, ~23 ms) should also be removed — a
  change with a different character (it turns on a trust-boundary judgement
  about a planted stub being trusted by name, not on redundancy elimination). No
  Requirement or Acceptance Criterion covers that change, yet Validation Results
  pre-commits to recording a "Shim double-hash decision", so the item is
  expected to *decide* something it has not scoped.
  **Impact**: If the question resolves "yes" mid-implementation, a task-sized
  redundancy removal silently grows a security-relevant change with untested
  requirements — the same kind of bundling that justified extracting 0186 out of
  0169 in the first place.
  **Suggestion**: Keep the stated default ("take the exec-probe win and leave
  this") but move the decision out of 0186 — either declare the shim staging
  explicitly out of scope and raise it as its own work item, or, if it must
  stay, add a Requirement and Acceptance Criterion so the trust-boundary change
  is scoped rather than discretionary.

- 🔵 **suggestion** / confidence: medium — **Deferred ~23 ms becomes unowned
  once 0186 closes**
  *Location*: Acceptance Criteria
  The referenced research (§12 of
  `2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`) identifies ~131 ms of
  149 ms as addressable — ~108 ms of exec probe plus ~23 ms of double-hashing
  the verify shim. Work item 0186 claims only the ~108 ms and defaults to
  leaving the rest, but no Acceptance Criterion creates a successor item for it,
  and sibling 0169 no longer owns it (its Summary lists the exec-probe fix as
  the extracted concern).
  **Impact**: Once 0186 closes, a measured and diagnosed optimisation with a
  documented fix shape has no owning work item, so it is likely to be
  rediscovered from scratch later.
  **Suggestion**: Add a hand-off criterion in the style 0169 already uses — if
  the double-hash question resolves as "leave it", the implementer raises a
  follow-up work item for the shim staging/hashing cost, referencing §12 for the
  measurement and the proposed same-directory-skip fix.

### Testability

**Summary**: The criteria are unusually well-engineered for a shell performance
refactor: AC1 names the behavioural substitute for an unobservable residue check
and pre-empts the root-bypass false pass, AC2 forbids a silent skip, and AC3
names the exact gates. The weak point is that the item's central objective — the
~108 ms warm-path saving — is gated only by a record-the-numbers criterion with
no threshold, so the whole AC set can pass with the probe still executing
(merely made non-fatal) and zero latency gained. Two behaviours the change's own
safety argument rests on are also uncovered: the warm-to-cold fall-through on a
noexec populated cache, and the cold happy path after `ensure_dir` is split out.

**Strengths**:

- AC1 explicitly identifies why the obvious check (probe residue) is
  non-discriminating — the probe is created and removed within one invocation —
  and substitutes a behavioural test, which is exactly the reasoning a tester
  would otherwise have to reconstruct.
- AC1 pre-empts a specific false-pass mode (root bypasses directory write
  permissions) and mandates a hard failure rather than a skip, closing the usual
  escape hatch for permission-dependent tests.
- AC2 rules out silent skipping: if the noexec check cannot run unprivileged on
  CI it must be recorded in Validation Results as a manual check, keeping the
  verification visible.
- AC3 names the concrete gates (`scripts/lint-bashisms.sh` plus
  shfmt/ShellCheck), giving a definitive pass/fail with no interpretation
  required.
- AC4 fixes a reproducible measurement protocol (median of 20 invocations, one
  darwin-arm64 host, single session, no build running), so the recorded numbers
  are at least comparable even though no target is set.
- Context and Technical Notes supply the attribution evidence (107.9 ms
  fresh-write-and-exec versus 10.6 ms re-exec of an existing probe), which makes
  the quantity under test explicit rather than assumed.
- Validation Results pre-declares a slot for each pending verification
  (before/after medians, host, noexec check mode, shim decision), so an unfilled
  criterion is visible rather than forgotten.

**Findings**:

- 🟡 **major** / confidence: high — **Latency criterion records a number but
  defines no pass/fail threshold**
  *Location*: Acceptance Criteria (warm-path saving measured and recorded)
  Work item 0186 exists to remove a ~108 ms exec probe from the
  `bin/accelerator` warm path, but the only criterion covering that objective
  says the before/after medians of 20 warm invocations are "measured and
  recorded in Validation Results" — it states no threshold, so any pair of
  numbers satisfies it, including a zero or negative saving.
  **Impact**: The item's primary goal is unverifiable — the criterion can always
  be claimed as passed, and a change that produced no speedup would still be
  signed off; note the sibling item 0169 does set a bound (`G ≤ 1.1 × B`) for
  its comparable latency criterion.
  **Suggestion**: Add an explicit gate to the criterion, e.g. "the after-median
  is at most 60 ms and the before-minus-after delta is at least 80 ms on
  darwin-arm64", and name the measurement method (e.g. hyperfine, or a bash loop
  over 20 runs taking the median) so the two numbers are produced identically.

- 🟡 **major** / confidence: medium — **The warm-to-cold fall-through that
  justifies the removal is not covered by any criterion**
  *Location*: Requirements (first bullet) / Acceptance Criteria
  The safety argument for deleting the warm-path probe is that "a `noexec`
  directory makes `verify_launcher` fail into the cold branch where the probe
  belongs", but no acceptance criterion exercises that path: AC2 tests an
  *empty* (i.e. cold) cache dir made non-executable, not a *populated* cache dir
  that has become non-executable.
  **Impact**: The load-bearing claim behind the change goes untested, so the
  regression it protects against — a warm cache on a noexec mount failing with a
  confusing error instead of the existing diagnostic — would not be caught by
  the stated verification.
  **Suggestion**: Add a criterion of the form "given a warmed cache dir
  subsequently made non-executable, the invocation exits non-zero with the same
  noexec diagnostic (asserted as a substring), demonstrating the fall-through
  into the cold branch", under the same non-root precondition AC1 defines.

- 🟡 **major** / confidence: medium — **No criterion verifies the cold happy
  path or that `ensure_dir` still creates the cache dir**
  *Location*: Requirements (second bullet) / Acceptance Criteria
  The change splits `probe_dir` into `ensure_dir` (the `mkdir -p`, always) and
  the cold-path-only probe, yet every criterion covers either the warm path
  (AC1, AC4) or a cold *failure* (AC2) — none asserts that a first-run
  invocation against a **non-existent** cache dir still creates the directory,
  probes, fetches and succeeds.
  **Impact**: The most likely regression from the refactor (the `mkdir -p` being
  lost or moved behind a branch, breaking first-use bootstrap on a clean
  machine) has no criterion that would fail; relying on "`mise run` is green"
  leaves it to whatever coverage happens to exist.
  **Suggestion**: Add a criterion such as "with `ACCELERATOR_CACHE_DIR` set to a
  path that does not yet exist, a cold invocation creates the directory and
  `bin/accelerator version` exits 0 with the expected version output", so the
  always-run `ensure_dir` half of the split is pinned.

- 🟡 **major** / confidence: medium — **AC1's precondition is under-specified
  and its pass depends on the warm path being write-free**
  *Location*: Acceptance Criteria (warm path performs no exec probe)
  AC1 says to point `ACCELERATOR_CACHE_DIR` at a directory that is "executable
  but not writable" and assert a warm invocation still succeeds, but warming a
  cache requires writing into that same directory, and the criterion does not
  state the setup sequence (populate while writable, then remove the write bit,
  then invoke). It also implicitly requires the warm path to perform *no* writes
  at all, whereas the Open Questions section describes warm-path shim staging
  into `cache_dir` that this item deliberately leaves in place.
  **Impact**: As written the test is either ambiguous to set up or potentially
  unsatisfiable for reasons unrelated to the probe, and a failure would not
  distinguish "probe still present" from "staging still writes" — the criterion
  loses its discriminating power either way.
  **Suggestion**: Spell out the sequence (warm the cache with the directory
  writable, `chmod -w`, then assert the second invocation exits 0 with the
  expected `version` output) and state explicitly whether the warm path is
  expected to be write-free after this change, or scope the assertion to
  probe-specific writes if staging may still write.

- 🔵 **minor** / confidence: medium — **Cold-path diagnostic criterion neither
  quotes the expected text nor states a privilege precondition**
  *Location*: Acceptance Criteria (noexec diagnostic on the cold path)
  AC2 requires a cold invocation against a `chmod -x` cache dir to exit non-zero
  with "the existing diagnostic text, asserted as a substring", but the expected
  string is never quoted; separately, removing a directory's execute bit also
  blocks writing files inside it, so the failure may originate from the write
  step rather than the exec probe — and unlike AC1, AC2 states no `id -u` guard
  even though root traverses directories regardless of the execute bit.
  **Impact**: A substring assertion against an unnamed message cannot
  distinguish the intended probe diagnostic from an unrelated write/traversal
  error, and the test's behaviour when run as root or on a restricted CI runner
  is undefined, so "cannot run unprivileged on CI" becomes a judgement call at
  verification time.
  **Suggestion**: Quote the exact diagnostic substring in the criterion, assert
  the specific non-zero exit code, and apply the same explicit non-root
  precondition (and hard-fail-if-root rule) that AC1 already defines.

- 🔵 **minor** / confidence: medium — **AC1 is satisfiable by a probe made
  non-fatal rather than removed**
  *Location*: Acceptance Criteria (warm path performs no exec probe)
  AC1 verifies that a warm invocation *succeeds* against a non-writable cache
  dir, which an implementation that keeps the write-chmod-exec probe on the warm
  path but tolerates its failure would also satisfy — retaining the full
  ~108 ms cost the item exists to remove.
  **Impact**: Combined with the absence of a latency threshold in the
  measurement criterion, the AC set as a whole can pass while the target cost
  remains, meaning no criterion actually pins "the probe does not run on the
  warm path".
  **Suggestion**: Strengthen AC1 with a direct absence assertion that does not
  rely on residue — e.g. run the warm invocation under `bash -x`/`PS4` tracing
  (or with a stub on the interpreter/`chmod` used by the probe) and assert no
  probe write-chmod-exec sequence appears in the trace — or add the latency
  threshold so the cost is gated numerically.

- 🔵 **suggestion** / confidence: medium — **The shim double-hash decision is
  not bound by any acceptance criterion**
  *Location*: Open Questions / Validation Results
  The Open Question asks whether the verify shim's second `sha256_file` call
  should also be removed, states a default ("take the exec-probe win and leave
  this") and instructs that the decision be recorded — and Validation Results
  has a `Shim double-hash decision — pending` slot — but no acceptance criterion
  requires that slot to be filled.
  **Impact**: The item can be closed with all criteria ticked while the
  trust-boundary decision remains unrecorded, losing the rationale for a
  deliberately deferred ~23 ms cost.
  **Suggestion**: Add a short criterion such as "the shim double-hash decision
  (remove, or keep with rationale on the planted-stub trust boundary) is
  recorded in Validation Results", so the recording obligation is part of done.

- 🔵 **suggestion** / confidence: low — **Criteria do not say where the new
  tests live, so their durability as regression guards is unclear**
  *Location*: Acceptance Criteria
  AC1 refers to "the test" and AC2 allows "automated or manual", but neither
  names the harness or suite the checks join — the repo's shell suites are
  standalone scripts wired into `mise run` tasks, so whether these become
  permanent guards or one-off local checks is left open.
  **Impact**: A verifier cannot tell whether re-running the criteria later is
  possible, and a passing one-off check would not prevent the probe being
  reintroduced on the warm path by a future edit to `bin/accelerator`.
  **Suggestion**: State in AC1/AC2 which suite the cases are added to (and that
  the suite is reached from `mise run`), or explicitly mark them as manual
  checks recorded in Validation Results so the choice is deliberate rather than
  implied.

## Re-Review (Pass 2) — 2026-07-31

**Verdict:** REVISE

All five lenses re-ran against the revised work item. Every pass-1 finding is
resolved or deliberately closed, and all five lenses opened by saying so. Pass 2
is not a repeat of pass 1: the new majors are defects **introduced by the pass-1
edits** (an over-claimed soundness rationale, a condition/body conflation, an
absolute gate calibrated on the wrong baseline) plus one genuine gap pass 1
missed entirely — that `chmod -x` cannot distinguish the exec half of the probe
from the write half. All pass-2 findings have now been addressed; a third pass
would be needed to confirm.

### Previously Identified Issues

Majors:

- 🟡 **Testability/Completeness**: no latency threshold — **Resolved.** Gate
  added, then corrected in pass 2 (see below).
- 🟡 **Dependency/Scope**: residual ~23 ms unowned vs 0169's gate — **Resolved.**
  Recorded in Dependencies with arithmetic, and pass 2 forced it further: a
  Requirement now carries a dated hand-off note to 0169, which has been appended
  to `0169`'s Dependencies.
- 🟡 **Dependency**: 0182 sequencing only "Related" — **Resolved.** `blocked_by`
  edge with stated direction, epic annotation corrected, and pass 2 added the
  reciprocal `blocks: ["work-item:0186"]` on 0182 plus the second shared
  artefact (the entrypoint suite).
- 🟡 **Clarity/Testability**: AC1 precondition under-specified — **Resolved**,
  though the rewrite over-claimed what AC1 proves; corrected in pass 2.
- 🟡 **Testability**: warm-to-cold fall-through untested — **Resolved.** New
  criterion.
- 🟡 **Testability**: cold happy path / `ensure_dir` untested — **Resolved.** New
  criterion, tightened in pass 2 to assert only what it observes.
- 🟡 **Dependency**: harness owned by downstream 0169 — **Resolved — finding was
  incorrect.** The harness exists in
  `tests/integration/entrypoint/test_accelerator_entrypoint.py`; pass 2's
  dependency lens independently confirmed this.
- 🟡 **Clarity**: shim-hash question conflated two changes — **Resolved.** Both
  options priced separately and the question resolved with test evidence.

Minors and suggestions: all ten minors and three suggestions resolved —
Drafting Notes added, diagnostic substring quoted, warm/cold defined, pronoun
chain fixed, citations pinned to functions, test suite named, execution
environment recorded, 0164/0167 provenance added, Requirements split into
behaviour and mechanism, References expanded.

### New Issues Introduced

- 🟡 **Clarity/Testability** (2 lenses): **AC1's "even one made non-fatal"
  clause was wrong.** A non-fatal probe fails its write, swallows the error and
  still exits 0 — so AC1 never discriminated that variant, contradicting AC2's
  stated purpose. **Fixed**: AC1 now claims only a *fatal* probe, and AC2 is
  relabelled load-bearing rather than belt-and-braces.
- 🟡 **Clarity**: **condition/body conflation in the write-free argument.** The
  pass-1 text said the staging `if` at `:255-261` "is not entered on a warm
  call" while elsewhere pricing the hash inside it at ~11.7 ms of retained
  warm-path cost — mutually exclusive as written. **Fixed**: the invariant now
  distinguishes the condition (evaluated always, reads only) from the body
  (skipped when digests match), so both claims hold.
- 🟡 **Testability**: **`chmod -x` cannot pin the exec half of the probe.**
  Removing a directory's execute bit also blocks creating files inside it, so a
  write-only check would satisfy both `noexec` criteria — leaving the one
  behaviour the item consciously preserves unverified. Pass 1 missed this
  entirely. **Fixed**: a criterion now requires either a real `noexec`-mount
  case or an explicit recorded gap; Drafting Notes records the choice.
- 🟡 **Testability**: **AC2's negative assertion had no positive control** and
  keyed on an implementation-chosen filename. **Fixed**: the same trace pattern
  must be asserted *present* on the cold run, and the assertion targets a
  rename-stable signal.
- 🟡 **Testability/Clarity/Completeness** (3 lenses): **the 60 ms absolute gate
  was calibrated on a pre-0182 baseline with an unrecorded host**, and "before"
  had two candidate meanings. **Fixed**: the before-median is re-measured
  post-0182 in the same session, and the gate is host-relative
  (`after ≤ before − 80 ms` and `after ≤ 0.5 × before`) with 60 ms demoted to
  advisory.
- 🟡 **Testability**: **green-path feasibility evidenced by a failure-path
  precedent.** **Fixed** — and the premise was verified in the code:
  `test_happy_path_forwards_args_and_exit_code` and
  `test_cache_hit_performs_no_further_fetch` are green-path end-to-end
  bootstraps, now cited. The one genuinely missing capability (threading
  `bash -x`/`PS4` through `_run_bootstrap`) is named as in-scope helper work.
- 🟡 **Dependency**: **the 0169 consequence never reached 0169.** **Fixed**: a
  Requirement mandates the hand-off note, and the note is appended to 0169.
- 🔵 **Scope/Completeness/Testability** (3 lenses): the shim-decision criterion
  was pre-discharged. **Fixed**: ticked with a pointer.
- 🔵 **Scope**: the write-free Requirement stated an invariant, not work —
  a scope-elasticity seam. **Fixed**: moved to Assumptions with an explicit
  "raise it, don't absorb it" boundary.
- 🔵 **Completeness**: Open Questions held no open question, triplicated across
  three sections. **Fixed**: condensed to "None outstanding" with one pointer;
  full reasoning lives once, in Validation Results.
- 🔵 Also fixed: Summary now names the exec probe and attributes ~97 ms (not
  ~108 ms) to the first-exec check; the terminology rule is softened to
  first-use-per-section so the text no longer breaks its own convention; the
  three conflicting responses to a non-conforming CI lane are replaced by one
  precedence rule in the preamble; Validation Results gained slots for the two
  warm-path criteria, lane observations and exclusions; a linux-lane criterion
  was added; and the release-artefact host is recorded as an external dependency
  of the measurement only.

### Assessment

The work item is materially stronger than at pass 1 and the verdict stays REVISE
only on the mechanical threshold (two or more majors in the pass), not on any
judgement that it is unready. Every pass-2 major is now fixed in the document.

Two substantive things changed in the underlying record beyond 0186 itself:
0169 carries a dated hand-off note that its `G ≤ 1.1 × B` gate may be
unreachable and must be relaxed or justified before acceptance, and 0182 now
declares `blocks: ["work-item:0186"]`. Epic 0136's annotation was corrected in
pass 1.

Two acknowledged residuals, both deliberate and recorded rather than closed: the
exec-specific branch of the cold-path probe has no test that distinguishes it
from a write-only check unless a `noexec` mount is added, and the ~23 ms of shim
hashing stays on the warm path to preserve a tested trust boundary. Neither
blocks implementation.

## Re-Review (Pass 3) — 2026-07-31

**Verdict:** APPROVE

> **Verdict set by the author on 2026-08-01**, overriding the mechanical
> threshold. The lens tally for this pass was two-or-more majors, which the
> configured `work_item_revise_major_count` maps to REVISE — but every pass-3
> finding was addressed in the document before the pass closed, and the
> residual findings were refinements of pass-2 repairs rather than gaps in the
> work. The work item is approved for implementation.

All five lenses re-ran. Every pass-2 finding is resolved, and the lenses said so
in their summaries. Pass 3's majors are again mostly **defects in the pass-2
fixes** — and one of them is serious: the positive control added in pass 2 to
stop a vacuous assertion was itself vacuous. Two lenses caught it independently
at high confidence, and it was confirmed against the source. All pass-3 findings
have been addressed.

### Previously Identified Issues

- 🟡 AC1 over-claimed against a non-fatal probe — **Resolved.**
- 🟡 Condition/body conflation in the write-free argument — **Resolved.**
- 🟡 `chmod -x` cannot pin the exec half — **Resolved**, and pass 3 improved the
  remedy: instead of an either/or that allowed a privileged `noexec` mount, the
  exec half is now verified by asserting in the cold-run xtrace that the probe
  file is *executed*. Near-zero cost, and the mount option moved out of scope.
- 🟡 AC2 lacked a positive control — **Resolved but wrongly**; see below.
- 🟡 Absolute 60 ms gate on a pre-0182 baseline — **Resolved**, then tightened
  in pass 3 (the `− 80 ms` clause was still absolute).
- 🟡 Green-path feasibility from a failure-path precedent — **Resolved.**
- 🟡 0169 consequence never reached 0169 — **Resolved**; pass 3 added the
  missing criterion and Validation Results slot for it.
- 🔵 Pre-discharged shim criterion, invariant-as-Requirement, triplicated Open
  Questions, Summary attribution, terminology rule, CI-lane precedence,
  Validation Results slots, linux lane, release-artefact host — **all
  Resolved.**

### New Issues Introduced

- 🟡 **The positive control was vacuous** (clarity + testability, both high
  confidence — the most important finding of the review). Pass 2 broadened AC2's
  trace pattern to "any `chmod` or any write into the resolved cache dir" so it
  would survive renaming the probe file. But the cold path independently does
  exactly those operations — `cp` plus `chmod` staging the shim
  (`bin/accelerator:257-260`, always taken against a fresh cache dir) and the
  launcher write — so the control matched on the cold run whether or not the
  probe fired. Verified against the source. **Fixed**: the signal is now the
  probe *function* name via `PS4='+${FUNCNAME[0]}:'` — rename-stable with
  respect to the probe file, yet probe-scoped. The control was also promoted to
  its own criterion and now additionally asserts the probe file is *executed*,
  which doubles as the exec-half verification.
- 🟡 **"Any write into the cache dir" is not observable in xtrace**
  (testability). Bash's xtrace prints expanded command words but not
  redirections, so a probe creating its file via `>` emits no path at all — half
  the stated predicate was unimplementable. **Fixed** by the same change.
- 🟡 **The preamble claimed all six behavioural criteria were automated suite
  cases** (clarity), while the record-the-gap criterion is discharged by writing
  a sentence and Validation Results called another "automated or manual".
  **Fixed**: each criterion is now labelled automated case, recorded check, lint
  gate, or aggregate build.
- 🟡 **The 0182 edge did not say what discharges it** (dependency) — and 0182's
  closure is gated on a manual pre-release check against a signed artifact
  (confirmed: its criterion at `:601` is unticked). Read literally,
  `blocked_by: 0182` gated this bash change — and transitively 0169 and its five
  children — on a release cut. **Fixed**: the edge is explicitly discharged when
  0182's changes reach `main`, not on its closure.
- 🟡 **The 0169 hand-off Requirement had no criterion and no Validation Results
  slot** (completeness, testability, dependency — 3 lenses), and was recorded as
  outstanding despite already being done. **Fixed**: marked discharged, retained
  as a re-confirmation criterion with a slot, and bounded to "record only —
  changing 0169's threshold is 0169's work".
- 🔵 Also fixed: the `− 80 ms` clause was absolute despite the gate being
  labelled host-relative, so a fast host could remove the whole probe and still
  fail — the ratio is now the sole pass condition and the delta is recorded;
  "~60 ms implied by the Context table" was wrong arithmetic (149.1 − 107.9 ≈ 41)
  and the derivation is now shown in Context; the residual is distinguished as
  ~11.7 ms (second hash) vs ~23 ms (skip staging) at first use; "probe" is no
  longer overloaded for the privilege check; the filesystem privilege check is
  specified concretely; the warm-to-cold criterion's purpose is softened to what
  it actually proves; the 0169 edge is qualified as acceptance-time so the two
  can run in parallel; the launcher-lockstep constraint from 0182 is carried
  into the measurement; 0165 added to completed dependencies; `resolve_cache_dir`
  citations labelled definition vs call site; the Summary matches the Context
  table's vocabulary; and `status: ready` beside a populated `blocked_by` is now
  explained.

### Assessment

Ready for implementation. The verdict stays REVISE only on the mechanical
two-major threshold; every pass-3 finding is fixed in the document, and the
remaining items are refinements of refinements rather than gaps in the work.

The pattern across three passes is worth noting: each pass found fewer *original*
gaps and more defects in the previous pass's fixes, and pass 3's headline finding
was that a guard added in pass 2 did not guard anything. That is the point at
which further passes have diminishing returns — the substantive content has been
stable since pass 2, and pass 3 changed how three criteria are asserted rather
than what the item does.

Two deliberate residuals remain, both recorded: the cold-path probe's exec half
is verified by xtrace rather than a real `noexec` mount (the mount is out of
scope), and the verify shim's staging cost stays on the warm path to preserve a
trust boundary three tests assert — with the consequence carried to 0169.
