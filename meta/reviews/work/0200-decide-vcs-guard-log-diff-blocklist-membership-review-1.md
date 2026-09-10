---
type: "work-item-review"
id: "0200-decide-vcs-guard-log-diff-blocklist-membership-review-1"
title: "Work Item Review: Decide whether git log/diff belong in vcs guard's blocked subcommand set"
date: "2026-09-10T09:29:05+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0136"
target: "work-item:0200"
work_item_id: "0200"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-09-10T11:04:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Decide whether git log/diff belong in vcs guard's blocked subcommand set

**Verdict:** REVISE

This spike is well-written, tightly bounded, and structurally complete: its
scoped question, effort constraint, and out-of-scope declaration are all
explicit, and completeness raised no findings. Two major findings pull the
verdict to REVISE — both concern the decision's outcome space rather than its
prose. The acceptance criteria enumerate only two of the four terminal outcomes
the requirements permit (testability), and the one downstream capability the
outcome actually gates — `skills/planning/validate-plan` in pure-jj repos, the
original motivation named in 0169 — is absent from the framing entirely
(dependency).

### Cross-Cutting Themes

- **Under-specified outcome space and stakes** (flagged by: testability,
  dependency, scope) — The spike permits four terminal outcomes (change,
  no-change, informational-tier spin-out, time-box expiry) but only defines
  "done" for two, and it omits the user-facing consequence — validate-plan
  staying broken in pure-jj repos — that should weigh in the decision. The
  informational-tier outcome specifically recurs across three lenses: it has no
  acceptance criterion (testability major), its follow-up coupling is a one-way
  reference (dependency suggestion), and it blurs the decide/implement seam
  (scope suggestion).

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability**: Acceptance Criteria do not cover all terminal outcomes
  the Requirements permit
  **Location**: Acceptance Criteria
  The ACs enumerate only change (AC2) and no-change (AC3), but Requirements
  also permit an informational-tier decision (out-of-scope, requiring a spawned
  item) and time-box expiry with only a leaning recorded. Neither maps cleanly
  to AC2 or AC3, so a verifier reaching those outcomes has no "done" condition.

- 🟡 **Dependency**: Downstream unblock of `skills/planning/validate-plan` not
  captured
  **Location**: Dependencies
  0169 states explicitly that `log`/`diff` sitting in the blocked set blocks
  `validate-plan` in pure-jj repos, and dropping them would unblock it — the
  original motivation for reconsidering the blocklist. 0200 frames the decision
  purely as a consistency/steering trade-off and omits validate-plan entirely,
  so the team could keep both blocked without weighing that it leaves
  validate-plan broken.

#### Minor

- 🔵 **Testability**: 'Fifth declared departure' recording obligation is not
  reflected in any criterion
  **Location**: Acceptance Criteria
  Requirements require recording a behaviour change as the fifth declared shell-
  parity departure, but AC2 lists the tests, fixture, and green `mise run`
  without any check that the departure itself is recorded, so the parity ledger
  could silently fall out of sync.

- 🔵 **Clarity**: The 'read-only' criterion does not distinguish log/diff from
  `git status`
  **Location**: Summary
  The Summary singles out `log`/`diff` as read-only with no side effects
  "unlike `git status`/`git add`/`git commit`" — but `git status` is also
  read-only, so the stated property does not separate them; the real criterion
  silently shifts to mental-model steering, which the Context then reapplies to
  `log`/`diff` too.

#### Suggestions

- 🔵 **Scope**: Spike folds the implementation of its own decision into the
  same unit
  **Location**: Requirements
  The item is a spike but its Requirements and ACs also carry the
  implementation (updating `BLOCKED_SUBCOMMANDS`, the fixture,
  `guard_decision_table.rs`, a green `mise run`), blending decide with
  implement. Impact is low — the change is trivial by 0169's design — but kind
  and code-landing criteria are slightly at odds.

- 🔵 **Testability**: Investigation evidence supporting the decision is not
  required by any criterion
  **Location**: Technical Notes
  AC1 requires a decision "with rationale" but does not require the rationale to
  cite the `git`-vs-`jj` output comparison or the warn-vs-deny axis finding the
  Technical Notes call for, so a decision could pass AC1 without being grounded
  in the evidence the spike was meant to gather.

- 🔵 **Dependency**: Conditional informational-tier follow-up left as a one-way
  reference
  **Location**: Requirements
  The spawned informational-tier item is referenced only from this spike; if it
  is created without a back-reference, the gating relationship is captured on
  one side only.

- 🔵 **Clarity**: Two distinct guard.rs files referenced with inconsistent
  paths
  **Location**: Context
  `cli/vcs-cli/src/guard.rs` is abbreviated to `vcs-cli/src/guard.rs` in the
  Context but written in full elsewhere; the bare `guard.rs` appears for both
  crates, so the two referents are easy to conflate.

### Strengths

- ✅ Subcommand arithmetic (13 = 11 + 2) reconciles across Summary, Context,
  and Open Questions, and pronoun referents resolve cleanly to `git log`/`diff`
  throughout.
- ✅ The scoped question is stated unambiguously with three concrete candidate
  outcomes and a natural stopping point, in both Summary and Open Questions.
- ✅ An explicit effort constraint (one `/conduct-spike` run) with a defined
  fallback, clarified in Drafting Notes as an effort budget rather than a wall-
  clock one.
- ✅ The informational-tier design is explicitly ruled out of scope and routed
  to a separate item, consistently across Requirements, Technical Notes, and
  Drafting Notes.
- ✅ Deciding `log` and `diff` together is coherent (shared analysis) while
  still allowing them to land differently — one decision unit, not two bundled
  concerns.
- ✅ Upstream inputs (0169, 0198) are captured, marked done and non-blocking,
  and consistent between prose Dependencies and frontmatter linkage.

### Recommended Changes

1. **Add acceptance criteria for the two uncovered terminal outcomes**
   (addresses: Acceptance Criteria do not cover all terminal outcomes the
   Requirements permit)
   Add an AC for the informational-tier decision ("the outcome is recorded and
   a separate work item to design the tier is created and linked") and one for
   time-box expiry ("the current leaning and the specific blocker are recorded
   on this item").

2. **Name `skills/planning/validate-plan` as the downstream consumer**
   (addresses: Downstream unblock of `skills/planning/validate-plan` not
   captured)
   Record validate-plan's pure-jj blockage in Dependencies and fold it into the
   Open Questions trade-off, so each candidate outcome's user-facing
   consequence is explicit and the dependant is visibly resolved on closure.

3. **Add the fifth-departure recording obligation to AC2** (addresses: 'Fifth
   declared departure' recording obligation is not reflected in any criterion)
   Extend AC2 to require "the change is recorded as the fifth declared departure
   from shell parity", naming where that ledger lives.

4. **Sharpen the distinguishing criterion in the Summary** (addresses: The
   'read-only' criterion does not distinguish log/diff from `git status`)
   State that `log`/`diff` are singled out because they neither desynchronise
   the working copy nor duplicate a jj-only capability, and acknowledge that
   read-only-ness alone (shared with `status`) is not the criterion.

5. **Normalise the guard.rs path references** (addresses: Two distinct guard.rs
   files referenced with inconsistent paths)
   Use the full `cli/…` path form for both files on every mention.

6. **Optionally clarify the decide/implement seam** (addresses: Spike folds the
   implementation of its own decision into the same unit; Investigation
   evidence supporting the decision is not required by any criterion)
   Either state that the fixture/test change is in-scope only when trivial, or
   add a criterion tying the recorded rationale to the `git`-vs-`jj` evidence
   the Technical Notes call for.

## Per-Lens Results

### Clarity

**Summary**: This spike is unusually well-written and internally consistent for
its length: the 13 / 11 / 2 subcommand arithmetic reconciles across Summary,
Context, and Open Questions, pronoun referents resolve cleanly to git log/diff
throughout, and the 'fifth declared departure' claim checks out against 0169's
four named departures. The main clarity soft spot is that the Summary's stated
criterion for why log/diff differ from the other blocked subcommands ('read-
only, no side effects') does not cleanly separate them from git status, which is
also read-only, and the Context's counter-case reintroduces the mental-model
argument for log/diff too. Path references to the two distinct guard.rs files
are also abbreviated inconsistently.

**Strengths**:
- The subcommand counts are consistent across sections (13 = 11 + 2).
- Pronoun referents are unambiguous throughout: every 'they'/'them' in the
  Context resolves to git log/git diff without competing antecedents.
- The scope boundary is stated explicitly and consistently across Requirements,
  Technical Notes, and Drafting Notes.

**Findings**:
- 🔵 minor (confidence: medium) — **The 'read-only' criterion does not
  distinguish log/diff from `git status`** (Summary). The Summary distinguishes
  git log/diff as "read-only commands with no jj-workspace side effects, unlike
  `git status`/`git add`/`git commit`", but `git status` is itself read-only,
  so the stated property does not separate log/diff from status; the
  distinguishing reason silently shifts to mental-model steering, which the
  Context's counter-case then reapplies to log/diff. A reader cannot pin down
  the operative criterion — exactly the axis the spike must decide. Suggestion:
  state that log/diff are singled out because they neither desynchronise the
  working copy nor duplicate a jj-only capability, and acknowledge read-only-
  ness alone is not the criterion.
- 🔵 suggestion (confidence: medium) — **Two distinct guard.rs files referenced
  with inconsistent paths** (Context). `cli/vcs/src/guard.rs` and
  `cli/vcs-cli/src/guard.rs` are distinct files; the Context abbreviates the
  second to `vcs-cli/src/guard.rs` while other sections write it in full, and
  bare `guard.rs` appears for both crates. Suggestion: use the full `cli/…`
  path form consistently on every mention.

### Completeness

**Summary**: This spike is structurally and informationally complete for its
kind. It carries an explicit scoped question, a stated effort constraint (one
/conduct-spike run), enumerable decision-oriented exit criteria, and every
relevant section is present and substantively populated. Frontmatter is intact
with a recognised kind and appropriate draft status. No completeness gaps rise
to the level of a finding.

**Strengths**:
- The scoped question is stated unambiguously in both Summary and Open
  Questions.
- A concrete effort constraint is present and explicitly clarified as an effort
  budget, not wall-clock, with a defined fallback.
- Exit criteria are enumerable decisions rather than vague 'understand X'
  outcomes, covering both change and no-change branches.
- The Context genuinely explains the forces behind the work (both the case and
  counter-case) rather than restating the Summary.
- Scope boundaries are explicit: the informational-tier is declared out of
  scope with a spin-out instruction.
- Frontmatter is complete and correct: recognised kind, appropriate status and
  priority, populated linkage.

**Findings**: None.

### Dependency

**Summary**: As a spike, 0200's upstream inputs (0169 which built the
blocklist, 0198 which reshaped the diff output this decision reasons about) are
both done and correctly captured, so the spike has no hidden blockers. The
significant gap is downstream: 0169 explicitly states that dropping log/diff
would unblock `skills/planning/validate-plan` in pure-jj repos, yet 0200 never
names that consumer, framing the decision purely as a consistency/steering
trade-off.

**Strengths**:
- Upstream inputs (0169, 0198) are cleanly captured, marked done and non-
  blocking, so no hidden prerequisite gates the spike's start.
- Frontmatter `relates_to` and `parent` are consistent with the prose
  Dependencies section.
- The conditional informational-tier spin-out is explicitly reserved rather
  than silently assumed.

**Findings**:
- 🟡 major (confidence: high) — **Downstream unblock of
  `skills/planning/validate-plan` not captured** (Dependencies). 0169 states
  `validate-plan` is "blocked in pure-jj repos today because log and diff sit in
  the guard's blocked set" and that dropping them "would unblock" it — the
  original motivation for reconsidering the blocklist. 0200 omits validate-plan
  from Dependencies and frames the decision solely as a consistency/steering
  trade-off. Impact: the team could keep both blocked on consistency grounds
  without weighing that this leaves validate-plan broken, and anyone tracking
  what unblocks validate-plan would not find this spike. Suggestion: record
  validate-plan's pure-jj blockage as the downstream consumer in Dependencies
  and in the Open Questions trade-off.
- 🔵 suggestion (confidence: low) — **Conditional informational-tier follow-up
  left as a one-way reference** (Requirements). The spawned tier item would be
  gated by this spike; if it does not back-reference the spike, the coupling is
  captured on one side only. Suggestion: when the decision spawns the tier item,
  add a Blocks/relates entry here and back-link from the new item.

### Scope

**Summary**: 0200 is a tightly-bounded spike with a specific, answerable
research question, carrying a clear rationale for both the case and counter-
case. It is well time-boxed, explicitly defers the informational-tier design to
a separate follow-up, and keeps the two closely-related subcommands together
under one shared analysis, which is coherent rather than a bundling problem. The
only mild scope observation is that the spike folds the potential implementation
of its own decision into the same unit, blurring the usual decide-then-implement
boundary.

**Strengths**:
- Names exactly what is being decided with three concrete candidate outcomes and
  a natural stopping point.
- Explicitly time-boxed to one `/conduct-spike` run with a defined fallback.
- Informational-tier is explicitly ruled out of scope and routed to a separate
  item.
- Deciding log and diff together is coherent while still allowing them to land
  differently — one decision unit.
- The Assumptions section fences off the guard's threat-model as settled.

**Findings**:
- 🔵 suggestion (confidence: medium) — **Spike folds the implementation of its
  own decision into the same unit** (Requirements). The item is a spike but its
  Requirements and ACs also carry the implementation (updating
  `BLOCKED_SUBCOMMANDS`, the fixture, `guard_decision_table.rs`, a green `mise
  run`), blending decide with implement where a spike conventionally produces
  only the decision. Impact: low — the change is trivial by 0169's design — but
  kind and code-landing criteria are slightly at odds. Suggestion: keep the
  shape if comfortable with spikes that land trivial follow-through, or make the
  decide/implement seam explicit.

### Testability

**Summary**: As a spike, 0200 frames its exit as concrete, checkable
deliverables — a recorded decision with rationale (AC1) and a well-pinned code-
change branch tied to named test files, the decision-table fixture, and a green
`mise run` (AC2). The main weakness is that the Acceptance Criteria partition
only two terminal outcomes (change vs. no-change) while the Requirements
explicitly permit two further outcomes (informational-tier spin-out; time-box
expiry with only a leaning recorded) that have no verifiable done-condition. A
secondary gap is that the 'fifth declared departure' recording obligation is not
mirrored in any criterion.

**Strengths**:
- AC1 states a concrete, existence-checkable deliverable rather than an open-
  ended 'understand the trade-offs' goal.
- AC2 pins the change branch to specific observable artefacts plus a definitive
  `mise run` exit-0 gate.
- Requirements impose an explicit time-box and treat log and diff as
  independently decidable.

**Findings**:
- 🟡 major (confidence: high) — **Acceptance Criteria do not cover all terminal
  outcomes the Requirements permit** (Acceptance Criteria). The ACs enumerate
  only change (AC2) and no-change (AC3), but Requirements permit an
  informational-tier decision (requiring a spawned item) and time-box expiry
  with a leaning recorded. Neither maps cleanly to AC2 (assumes fixture/tests
  updated, impossible for a not-yet-built tier) or AC3 (asserts neither changes
  and a close). Impact: a verifier reaching either uncovered outcome has no AC
  defining 'done'. Suggestion: add ACs for the informational-tier and time-box-
  expiry outcomes.
- 🔵 minor (confidence: high) — **'Fifth declared departure' recording
  obligation is not reflected in any criterion** (Acceptance Criteria).
  Requirements require recording a behaviour change as the fifth declared shell-
  parity departure, but AC2 omits any check that the departure is recorded.
  Impact: the parity-departure ledger could silently fall out of sync.
  Suggestion: extend AC2 to include the departure recording, naming where the
  ledger lives.
- 🔵 suggestion (confidence: medium) — **Investigation evidence supporting the
  decision is not required by any criterion** (Technical Notes). The Technical
  Notes call for comparing real git vs jj output and checking the warn-vs-deny
  axis, but AC1 requires only a decision "with rationale" — not that the
  rationale cite this evidence. Impact: the recorded decision could be present
  yet not demonstrably grounded in the evidence the spike was meant to gather.
  Suggestion: if the evidence is load-bearing, add a criterion that the rationale
  references it.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-09-10

**Verdict:** REVISE

All eight pass-1 findings were addressed. Both pass-1 majors are resolved, but
the acceptance-criteria restructuring that closed the testability major
introduced two new majors: the change/no-change/tier/expiry branches now
overlap (a tier-move is itself a "treatment change", so AC2 and AC4 both fire),
and no criterion requires the recorded rationale to weigh the validate-plan
consequence that the dependency fix newly made visible. Both are tight
AC-partitioning fixes.

### Previously Identified Issues

- 🔵 **Clarity**: read-only criterion doesn't distinguish log/diff from `git
  status` — Resolved. Summary now frames the criterion as desync/duplication,
  explicitly noting read-only-ness is shared with `status`.
- 🔵 **Clarity**: inconsistent guard.rs paths — Resolved. Context now uses the
  full `cli/vcs-cli/src/guard.rs` form.
- 🟡 **Dependency**: validate-plan downstream unblock not captured — Resolved.
  Recorded as a downstream consumer in Dependencies and as Open Question 3.
- 🔵 **Dependency**: informational-tier follow-up one-way reference — Resolved.
  Dependencies now instructs a Blocks entry plus back-link.
- 🔵 **Scope**: spike folds implementation into the decision unit — Partially
  resolved. Guarded with a trivial-only clause and a spin-out for non-trivial
  changes; the lens now rates it a suggestion and calls it "already well-
  guarded", but still flags the residual spike/implement blend.
- 🟡 **Testability**: ACs don't cover all terminal outcomes — Resolved. AC4
  (tier) and AC5 (time-box expiry) added; but see new AC-overlap majors below.
- 🔵 **Testability**: fifth-departure recording not in any criterion —
  Resolved. AC2 now requires recording the departure in 0169's ledger.
- 🔵 **Testability**: decision evidence not required — Resolved. AC1 now
  requires the rationale to cite the git-vs-jj output difference and the
  warn-vs-deny finding.

### New Issues Introduced

- 🟡 **Clarity + Testability**: AC2 ("treatment changes") and AC4 ("move to
  informational tier") overlap — a tier-move is itself a treatment change, so
  both branches fire and demand opposite actions (land the code vs defer it).
  Flagged as major by both lenses. Fix: narrow AC2's trigger to "dropped from
  `BLOCKED_SUBCOMMANDS` entirely", mutually exclusive with AC4.
- 🟡 **Testability**: no criterion requires the rationale to weigh the
  validate-plan pure-jj consequence — the very motivation the dependency fix
  surfaced. Fix: extend AC1 to require the rationale to address whether the
  chosen treatment leaves validate-plan broken and whether that cost is
  outweighed.
- 🔵 **Testability + Clarity**: AC1 is phrased unconditionally ("A decision is
  recorded") but AC5 governs the no-decision path — a strict verifier marks AC1
  failed on the time-box branch. Fix: make AC1 conditional ("If a firm decision
  is reached…").
- 🔵 **Dependency**: the write-back to closed item 0169's departures ledger is
  an unrecorded cross-item coupling; and the non-trivial "separate chore" spawn
  path lacks the bidirectional-link treatment the tier item received.
- 🔵 **Clarity**: the warn-vs-deny axis is referenced in AC1 before it is
  introduced (only Technical Notes characterises it).
- 🔵 **Testability/Clarity/Dependency suggestions**: AC1 presupposes a warn-vs-
  deny finding the decision "relied on" (may not materialise); `diff` vs `diff
  --stat` scope oscillates; Technical Notes should state the jj/git-binary and
  `git.colocate=false` prerequisites for the empirical comparison.

### Assessment

Not yet ready. The two new majors are artefacts of the AC restructuring rather
than fresh design gaps, and both are resolved by narrowing AC2's trigger to the
outright-drop case and adding the validate-plan clause to AC1 — small, local
edits. Recommend applying those two fixes (plus the AC1-conditional wording),
then closing out; the remaining items are minor/suggestion polish.

## Re-Review (Pass 3) — 2026-09-10

**Verdict:** COMMENT

Both pass-2 majors are resolved, and no lens raises a major. The remaining
findings are minor/suggestion polish, several converging on one theme: the
"non-trivial change" spin-out branch is under-defined (no trigger, no
acceptance criterion), and the "declared-departures ledger" reference does not
match 0169's actual section name. The work item is acceptable for a low-
priority spike as-is.

### Previously Identified Issues (pass 2)

- 🟡 **Clarity + Testability**: AC2/AC4 branch overlap — Resolved. AC2 (and its
  Requirements bullet) now trigger on "dropped from `BLOCKED_SUBCOMMANDS`
  outright", mutually exclusive with AC4's tier-move.
- 🟡 **Testability**: rationale needn't weigh validate-plan — Resolved. AC1 now
  requires the rationale to address whether the treatment leaves validate-plan
  broken and whether the cost is outweighed.
- 🔵 **Testability + Clarity**: AC1 unconditional vs AC5 — Resolved. AC1 is now
  conditional ("If a firm decision is reached…").
- 🔵 **Dependency**: 0169 ledger write-back coupling — Resolved. Dependencies
  now states the outright-drop path amends a closed item's ledger.
- 🔵 **Dependency**: non-trivial-chore spawn coupling — Resolved. The Blocks +
  back-link instruction now covers both the tier item and the chore.
- 🔵 **Clarity**: warn-vs-deny referenced before defined — Resolved. The axis
  is now introduced in Context at the mode-composition mention.
- 🔵 **Suggestions** (warn-vs-deny presupposition, diff/--stat scope, env
  prerequisites) — Resolved. AC1 softened; Context clarifies the `diff`
  subcommand scope; Technical Notes state the jj/git-binary and
  `git.colocate=false` prerequisites.

### New Issues Introduced

- 🔵 **Testability**: the non-trivial-drop outcome has no acceptance criterion
  (AC2 covers only the trivial drop; AC4 covers only the tier), and the "firm
  decision" boundary between AC1 and AC5 is undefined.
- 🔵 **Clarity**: outcome vocabulary shifts between Context ("allowed, no
  suggestion" / "allowed with an informational note") and Requirements
  ("dropped entirely" / "informational-suggestion tier"); "non-trivial change"
  has no stated trigger; AC1's per-subcommand rationale obligation is not
  reconciled with AC3's single "shell parity" rationale; "declared-departures
  ledger" does not match 0169's "Declared behavioural changes" section name.
- 🔵 **Dependency**: the outright-drop path should reconcile 0169's "exactly
  four departures" framing (and any count assertion derived from it), not just
  append a fifth row.

### Assessment

Ready as a low-priority spike. No structural blockers remain — the outcome
space is covered for every path a conductor is likely to hit, the rationale
obligations are concrete and falsifiable, and dependencies are mapped in both
directions. The residual minors are worth a single tightening pass if touched
again: define "non-trivial" and give it an AC, unify the outcome vocabulary,
rename the ledger reference to 0169's actual section, and reconcile AC1/AC3's
rationale wording. None gates conducting the spike.

## Approval — 2026-09-10

**Verdict:** APPROVE (reviewer override)

Reviewer accepted the work item as ready to conduct despite the residual
minor/suggestion polish recorded in Pass 3. No structural blockers remain; the
outstanding items are non-gating and may be folded in if the item is edited
again. Work item transitioned to `ready`.
