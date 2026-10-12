---
type: "work-item-review"
id: "0310-recovering-from-an-interrupted-conduct-run-review-1"
title: "Work Item Review: Recovering from an Interrupted Conduct Run"
date: "2026-10-12T21:03:34+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0310"
work_item_id: "0310"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-10-12T22:39:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Recovering from an Interrupted Conduct Run

**Verdict:** REVISE

0310 is a coherent, well-evidenced story. Its causal argument (the
orchestrator shares the account quota, so recovery must come from the next
run) is clear, and its boundary with 0304 is drawn on both sides. It falls
short where the Requirements meet the Acceptance Criteria: the two sections
cover different ground, the unchecked-findings mechanism rests on unresolved
open questions, and the usage-limit recovery wording and `RunLedger`
retention overlap 0304 and 0307 with no ownership or landing order stated.

### Cross-Cutting Themes

- **Requirements and acceptance criteria drift apart** (flagged by:
  testability, completeness, scope, clarity) — the Leftover writes
  requirement covers findings but AC3 covers only notes. The recovery
  guidance content has no criterion. AC5 asks for manifest and outline
  reconciliation that no requirement names.
- **Usage-limit recovery wording is owned twice** (flagged by: dependency,
  scope, clarity) — the new "interrupted run" row repeats 0304's "agent usage
  limit" recovery, AC6 does not separate the two rows, and no landing order
  or reconciliation owner is recorded.
- **The unchecked-findings mechanism is undecided** (flagged by: testability,
  scope, clarity, completeness, dependency) — the open questions decide how
  unvalidated findings are identified and whether `RunLedger` must change.
  Until they are answered, AC4 can't be built, a written-but-invalid spawn
  falls into both report groups, and the story's size is uncertain.
- **"Written" is undefined for a spawn** (flagged by: testability, clarity)
  — partial output (notes without a finding, truncated files) has no defined
  report group.

### Findings

#### Major

- 🟡 **Clarity**: A written finding that fails validation is both "written
  by that run" and "re-offered"
  **Location**: Requirements / Acceptance Criteria
  AC1 splits the last batch into re-offered and written spawns. AC4 then
  re-offers written findings that fail validation, so those spawns fall into
  both groups. The Requirement re-offers the "finding" but AC4 re-offers "its
  pair".
- 🟡 **Dependency / Scope / Clarity**: The failure-reason table and
  usage-limit wording overlap 0304, with no landing order or owner
  **Location**: Dependencies / Requirements: Recovery guidance
  Both items add usage-limit recovery rows ("wait for the reset, or raise the
  spend limit") to the same table, and 0304 also rewrites steps 5 and 7.
  "The message" has no referent from the new run's side, because the new run
  never sees the dead run's error.
- 🟡 **Testability / Completeness / Clarity**: Leftover findings are not
  covered; only notes are checked
  **Location**: Acceptance Criteria
  AC3 covers notes only, and only says where they must not appear. Leftover
  findings misreported as unexpected, or notes silently dropped, would both
  pass.
- 🟡 **Testability / Completeness**: The recovery-guidance content has no
  criterion
  **Location**: Acceptance Criteria
  AC6 checks only that a distinct row exists. A row with empty or wrong
  next-step guidance would pass.
- 🟡 **Testability**: The unchecked-findings precondition can't be built
  while the open question is unresolved
  **Location**: Acceptance Criteria
  AC4 assumes a way to tell that a finding never went through step 5, which
  the Open Questions leave undecided. Nothing covers the passing path, where
  a valid unchecked finding is counted and its pair is not re-offered.
- 🟡 **Testability**: The usage-limit criterion relies on a trigger that
  can't be reproduced and on a vague "up to date"
  **Location**: Acceptance Criteria
  The mechanism only sees a stale ledger, so a real usage limit adds nothing
  a test can observe. "Brought up to date from disk" names no concrete
  expected state.
- 🟡 **Dependency / Scope**: 0310 and 0307 may both change `RunLedger`, with
  no coordination recorded
  **Location**: Dependencies / Open Questions
  Both items ask whether the ledger must retain more per offered batch. No
  ordering or schema owner is stated, so the two risk conflicting changes and
  rework.

#### Minor

- 🔵 **Completeness / Scope**: The manifest and outline reconciliation
  appears only in the acceptance criteria
  **Location**: Requirements
  AC5 requires the manifest counts and outline checkboxes to be brought up to
  date, but no requirement names this. It is unclear whether this is new
  work or a regression check on existing step 7.
- 🔵 **Testability / Clarity**: "Output on disk" versus "wrote nothing"
  leaves partial output undefined
  **Location**: Requirements: Interrupted-run report / AC1
  A spawn with notes but no finding, or a truncated file, has no defined
  report group.
- 🔵 **Dependency**: The 0308 coupling is understated
  **Location**: Dependencies
  0308's offered-batch guard decides whether a superseded run's in-flight
  writes are refused. That conflicts with, or constrains, 0310's leftover-
  writes attribution.
- 🔵 **Dependency**: The Claude Code error-wording contract is not recorded
  as an external dependency
  **Location**: Dependencies
  The `Agent terminated early …` wrapper (v2.1.199+, above the v2.1.144
  floor), the `You've hit your …` prefix and the `· resets` suffix are
  relied on but not listed.
- 🔵 **Testability**: The negative wording criterion has no concrete check
  **Location**: Acceptance Criteria (AC2)
  "Does not assert that it stopped" is a judgement call. It names no
  required phrases and no banned ones.
- 🔵 **Testability**: There is no negative case for a run that ended cleanly
  or had no prior ledger
  **Location**: Acceptance Criteria
  An implementation that always prints the report would pass. No criterion
  covers the structured plan form.
- 🔵 **Clarity**: Actors and destinations are hidden behind "the plan
  reports", "are reported" and "being counted"
  **Location**: Requirements
  The requirements leave unclear whether the CLI or the skill produces each
  output, and where leftover writes and counts appear.
- 🔵 **Clarity**: Pipeline jargon is used without definition or link
  **Location**: Context / Requirements / Acceptance Criteria
  "Steps 5–7", "tick", "pair", "pending window" and "quarantined" are not
  glossed on first use.
- 🔵 **Scope**: Validating unchecked findings is a separable concern
  **Location**: Requirements
  If the open question resolves into new validation plumbing, splitting it
  into a sibling story would let the report and guidance ship on their own.

#### Suggestions

- 🔵 **Completeness**: The open questions have no resolution path
  **Location**: Open Questions
  State when and how each question will be answered, or the working
  assumption the requirements are written against.
- 🔵 **Dependency**: 0283 is missing from the Dependencies section
  **Location**: Dependencies
  It is the origin of the ledger and offered-spawn semantics, and it appears
  in the frontmatter and References but not in Dependencies.

### Strengths

- ✅ The Summary and Context state the key constraint precisely (shared
  account quota, so the run can't halt itself), and that justifies the
  next-run recovery framing.
- ✅ The Context quotes the observed error and warning text verbatim, which
  gives concrete fixtures and a baseline to test against.
- ✅ The boundary with 0304 is explicit and consistent in both items: 0304
  owns failures the orchestrator survives, 0310 owns runs that die without
  ending.
- ✅ Widening from usage limits to any interruption is justified by the
  mechanism (it can't tell causes apart). It reflects the real unit of work,
  not scope creep.
- ✅ AC1 gives a countable outcome (N, M, N − M), and AC2 tests a subtle
  safety property: never asserting that the earlier run stopped.
- ✅ The Assumptions and Drafting Notes say which parts are inferred and
  which are unconfirmed.
- ✅ "Blocked by: none known" is accurate: 0283 is done and 0303 has landed.

### Recommended Changes

1. **Resolve or bound the open questions** (addresses: unchecked-findings
   precondition; RunLedger coordination with 0307; separable concern; open
   questions resolution path)
   State the identification rule for an unvalidated finding (e.g. a finding
   file belonging to a spawn in the replaced ledger's last batch), and say
   whether `RunLedger` must change. If it must, name 0307 or 0310 as the
   owner of the per-batch record and state the landing order.
2. **Settle the report's classification rules** (addresses: written finding
   in both groups; partial output undefined)
   Say that leftover findings are validated before the report is built,
   which group a spawn whose finding fails validation goes in, and what
   output each spawn kind must leave to count as written. Use one term for
   the unit that is re-offered (spawn or pair) everywhere.
3. **Assign ownership of the usage-limit wording to one item** (addresses:
   overlap with 0304)
   Have the "interrupted run" row advise re-running `conduct` and defer
   wait-or-raise detail to 0304's agent-usage-limit row (or the reverse).
   Add "either may land first; the second reconciles the table" to the 0304
   dependency entry. Say that "the message" means the error the user saw in
   the earlier session. Extend AC6 to cover separation from 0304's rows.
4. **Align the Requirements and Acceptance Criteria** (addresses: leftover
   findings; recovery-guidance content; manifest/outline reconciliation)
   Extend AC3 to cover findings and to say where leftovers appear (listed as
   run X's writes). Add a criterion for the guidance row's next step, with
   and without a reset time. Either add a reconciliation requirement or
   reword AC5 as a check on existing step 7 behaviour.
5. **Make the criteria testable with fixtures** (addresses: usage-limit
   criterion; negative wording; negative case; unchecked-findings passing
   path)
   Rewrite AC5 against a stale-ledger fixture with a concrete expected
   manifest and outline state. Pin the required and banned phrases for AC2.
   Add a no-report case for a clean `end-run` or no prior ledger, a passing
   unchecked-finding case, and a criterion that names the structured plan
   fields.
6. **Tighten the Dependencies section** (addresses: 0308 coupling; Claude
   Code contract; 0283)
   Name the shared 0308 decision about writes from a superseded run. Add an
   external entry for the Claude Code error wording with its version
   (v2.1.199) and what degrades if it changes. Add 0283 (done).
7. **Clarify actors and terms** (addresses: actors obscured; jargon)
   Name the producer of each output (the CLI plan or the skill) and gloss
   steps 5–7, "pair", "pending window" and "quarantined" on first use.

## Per-Lens Results

### Clarity

**Summary**: The work item tells a clear causal story. It explains why
recovery must come from the next run, and the evidence behind that reasoning
is concrete. The clarity problems sit where the Requirements meet the
Acceptance Criteria and the sibling item 0304. It is unclear what gets
re-offered for a written finding that fails validation, and which spawns
count as "written". The "interrupted run" row's usage-limit guidance
overlaps 0304's "agent usage limit" row. It also points at a "message" the
new run may never see. Several pipeline terms (steps 5–7, pair, tick,
pending window, the plan) are used without definition or link.

**Strengths**:
- The Summary and Context state the key constraint unambiguously: the
  orchestrator shares the account quota, so the interrupted run cannot halt
  itself. That makes the shift from halting to next-run recovery easy to
  follow.
- The Context quotes the observed error and warning text verbatim, so
  readers don't have to guess what the user actually saw.
- It explicitly separates its scope from 0304: 0304 covers failures the
  orchestrator survives, this item covers runs that die without ending.
- The required wording ("did not end; if it is still going …", never
  asserting that the run stopped) is stated precisely, and AC2 restates it
  the same way.

**Findings**:
- 🟡 major / high — **A written finding that fails validation is both
  "written by that run" and "re-offered"** (Requirements / Acceptance
  Criteria). The Interrupted-run report splits the last batch into spawns
  "whose output is on disk" and spawns "with none", and AC1 lists the M
  empty spawns as re-offered and the other N − M as "written by that run".
  But the Unchecked findings requirement and AC4 say a written finding that
  fails validation is quarantined and re-offered, so such a spawn falls in
  both groups. Separately, the Requirement says the failed finding itself is
  "re-offered", while AC4 says "its pair" is re-offered. Say whether
  validation happens before the report is built, how such a spawn appears,
  and use one term for the unit that is re-offered.
- 🟡 major / medium — **"Interrupted run" row overlaps 0304's "agent usage
  limit" row and cites an unavailable "message"** (Requirements: Recovery
  guidance). 0304 already defines an "agent usage limit" failure reason with
  almost the same recovery. AC6 separates the new row from `rate_limited`
  and the concurrent-subagent-limit row, but not from 0304's row. The new
  run never sees the dead run's API error, so "the message" has no clear
  referent. State how the two rows relate, and say that "the message" means
  the error the user saw in the earlier session.
- 🔵 minor / medium — **"Output on disk" versus "wrote nothing" leaves
  partial output undefined** (Requirements: Interrupted-run report). Define
  what counts as a spawn's output for this split, and say which group a
  spawn with only partial artefacts belongs to.
- 🔵 minor / medium — **Actors and destinations obscured by "the plan
  reports", "are reported" and "being counted"** (Requirements). Name the
  producer in each case (CLI plan output versus skill), and the place where
  the leftover writes and the validated-finding counts appear.
- 🔵 minor / medium — **Pipeline jargon used without definition or link**
  (Context / Requirements / Acceptance Criteria). "Steps 5–7", "tick",
  "composers", "offered", "pair", "pending window" and "quarantined" are
  used without definition. Name steps 5–7 briefly and gloss the remaining
  terms on first use.
- 🔵 suggestion / low — **Leftover writes covers notes and findings, but AC3
  covers notes only** (Requirements: Leftover writes / Acceptance Criteria).
  Clarify whether findings can currently be reported as unexpected and, if
  so, name the section they would appear in.

### Completeness

**Summary**: Work item 0310 is a well-populated story. The Summary names the
persona and the need, and the Context explains the trigger, gives the
observed evidence and spells out why recovery has to happen on the next
run. Requirements, Acceptance Criteria, Open Questions, Dependencies,
Assumptions and Technical Notes all have real content, and the frontmatter
is complete and valid. The remaining gaps are small: Requirements and
Acceptance Criteria don't fully cover each other, and the open questions
bear directly on one requirement but don't say how they will be settled.

**Strengths**:
- The Summary identifies who the story serves, what they need, and why the
  interrupted run can't recover itself.
- The Context grounds the work in a concrete observed failure and draws a
  clear line against 0304's halting-failure class.
- Requirements are split into named units, each specific enough to start
  from.
- There are six Given/When/Then acceptance criteria, and most map clearly to
  a requirement.
- The frontmatter is complete and the body header matches it.
- Assumptions and Drafting Notes record the inferred parts instead of
  leaving them implicit.

**Findings**:
- 🔵 minor / high — **Manifest/outline reconciliation appears only in
  Acceptance Criteria** (Requirements). AC5 requires the manifest counts and
  outline checkboxes to be brought up to date from disk, but no requirement
  covers this. Add a requirement, or say explicitly that the normal
  end-of-run steps already do this and the criterion is a regression check.
- 🔵 minor / medium — **Some requirement content has no acceptance
  criterion** (Acceptance Criteria). Leftover findings and the guidance
  row's next step have no criterion. Extend AC3 to cover findings, and add a
  criterion for the guidance with and without a reset time.
- 🔵 suggestion / medium — **Open questions that shape requirements have no
  resolution path** (Open Questions). Note how and when each question will
  be answered, or state the working assumption the requirements are written
  against.

### Dependency

**Summary**: 0310 has a Dependencies section that names its sibling stories
(0303, 0304, 0307, 0308) and briefly says how each one touches it. It
correctly records no hard blockers, since 0283's ledger has shipped and 0303
has landed. The gaps are in coordination: 0304 and 0307 change the same
failure-reason table, step 5/step 7 prose and ledger snapshot that 0310
relies on, but neither landing order nor who reconciles the shared changes
is stated. 0310 also depends on Claude Code's error wording, an external
contract that is described in Context but not recorded as a dependency.

**Strengths**:
- The Dependencies section says how each related item touches 0310, not
  just the IDs.
- "Blocked by: none known" is correct: 0283 is done and 0303 has landed.
- The drafting notes record that the halting-failure class moved to 0304,
  and 0304's own Dependencies agree.

**Findings**:
- 🟡 major / high — **No landing order or reconciliation owner for the
  failure-reason table, shared with 0304** (Dependencies). Both items add
  rows to the table and give "agent usage limit" recovery text, and 0304
  also rewrites steps 5 and 7. 0303 handled the same overlap explicitly
  ("Either item may land first; the second reconciles the table"). Add a
  sequencing note, and consider extending the distinct-row criterion to
  cover 0304's halting-reason rows.
- 🟡 major / medium — **0310 and 0307 may both change `RunLedger`, with no
  coordination recorded** (Dependencies / Open Questions). Both items ask
  whether the ledger must retain more per offered batch. State the ordering,
  or name which item owns extending the per-batch record.
- 🔵 minor / medium — **The 0308 coupling is understated: superseded-run
  writes interact with 0308's offered-batch guard** (Dependencies). 0308's
  guard decides whether a replaced run's in-flight writes are refused, which
  constrains 0310's leftover-writes attribution. Name this shared decision
  and say which item settles it.
- 🔵 minor / medium — **Claude Code error-wording contract is not recorded
  as an external dependency** (Dependencies). The wrapper (v2.1.199+, above
  the v2.1.144 floor), the prefix and the reset suffix are relied on. Add an
  external entry with the version and what degrades if they change.
- 🔵 suggestion / low — **Frontmatter and References name 0283 and 0121, but
  Dependencies does not** (Dependencies). Add 0283 (done) as the origin of
  the ledger and offered-spawn semantics.

### Scope

**Summary**: 0310 is a coherent story: every requirement serves one purpose,
recovering on the next `conduct` from a run that died without ending. It is
sized sensibly for a single increment across the CLI ledger and the
research-topic skill. Moving the halting-failure class out to 0304 gave it a
clean boundary. Four scope edges remain loose. Validating unchecked findings
is a separable correctness concern whose mechanism is still an open
question. One acceptance criterion asks for manifest and outline
reconciliation that no requirement names. The usage-limit recovery wording
overlaps 0304's. An open question could pull a `RunLedger` model change
into scope.

**Strengths**:
- The Summary, Context and Requirements share one framing, and the Context
  explains why a halt-in-place approach is out of scope.
- The boundary with 0304 is drawn explicitly and consistently in both items.
- Widening the item to any interruption reflects the real unit of work
  rather than scope creep.
- The story fits within a single team's ownership.

**Findings**:
- 🔵 minor / medium — **Validating unchecked findings is a separable concern
  with an unresolved mechanism** (Requirements). If the open question
  resolves into new validation machinery, split it into a sibling story so
  the report and guidance can ship on their own.
- 🔵 minor / high — **An acceptance criterion requires manifest and outline
  reconciliation that no requirement covers** (Acceptance Criteria). Add a
  requirement, or reword AC5 as a check on existing step 7 behaviour.
- 🔵 minor / medium — **Usage-limit recovery wording overlaps with 0304**
  (Requirements). Make one item the owner of the usage-limit recovery
  wording.
- 🔵 suggestion / low — **An open question on ledger retention could widen
  the story into a ledger model change** (Open Questions). Answer it before
  sizing. If `RunLedger` must change, split out the retention change or
  sequence this story after 0307.

### Testability

**Summary**: Most of the acceptance criteria use Given/When/Then with
concrete preconditions, and the N/M split in AC1 can be tested directly.
Coverage has gaps, though. The Leftover-writes requirement covers findings
but its criterion only covers notes. The Recovery-guidance content has no
criterion at all. The usage-limit criterion is framed around an event that
can't be reproduced, when the mechanism can't tell causes apart anyway. The
unchecked-findings criterion depends on an unresolved open question, so its
precondition can't yet be built.

**Strengths**:
- AC1 gives a precise, countable outcome, so a verifier can build a fixture
  and assert exact lists.
- Requirements are each paired with a matching criterion, and the criteria
  describe observable behaviour.
- The Context gives exact observed error text and the current warning
  string, which make good fixtures.
- AC2 tests a subtle safety property that a naive implementation could
  easily get wrong.

**Findings**:
- 🟡 major / high — **Recovery-guidance content has no criterion**
  (Acceptance Criteria). AC6 only checks that a distinct row exists. Add a
  criterion for the row's next step: re-run `conduct <slug>`, wait for the
  reset time when given, raise the spend limit when not.
- 🟡 major / high — **Leftover findings not covered; only notes are
  checked** (Acceptance Criteria). Change AC3 so that leftover notes and
  findings are listed as written by run X and appear under neither
  unexpected section.
- 🟡 major / medium — **Unchecked-findings precondition can't be built
  while the open question is unresolved** (Acceptance Criteria). State the
  identification rule and add the passing case: a valid unchecked finding is
  counted and its pair is not re-offered.
- 🟡 major / medium — **Usage-limit criterion relies on a trigger that
  can't be reproduced and on a vague "up to date"** (Acceptance Criteria).
  Write the precondition as a stale-ledger fixture, and spell out that the
  manifest's per-pair counts and the outline checkboxes match disk before
  the first batch.
- 🔵 minor / medium — **What counts as a spawn having "written" output is
  undefined** (Acceptance Criteria). Say what output each spawn kind must
  leave, and add a partial-output case.
- 🔵 minor / medium — **Negative wording criterion lacks a concrete check**
  (Acceptance Criteria). Require "did not end" and "if it is still going",
  and ban "stopped", "crashed" and "was interrupted" said of the earlier
  run.
- 🔵 minor / medium — **No negative case for runs that ended cleanly or had
  no prior ledger** (Acceptance Criteria). Add a no-report case and a
  plan-level criterion naming the structured fields (run id, written spawns,
  unwritten spawns).

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-10-12T21:24:04+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: A written finding that fails validation is both "written
  by that run" and "re-offered" — Partially resolved (three classes now
  defined; the re-offered pair's index is still unstated)
- 🟡 **Dependency / Scope / Clarity**: The failure-reason table and
  usage-limit wording overlap 0304 — Partially resolved (0304 owns the
  wording; "either may land first" conflicts with a criterion that needs
  0304's rows)
- 🟡 **Testability / Completeness / Clarity**: Leftover findings not covered
  — Resolved
- 🟡 **Testability / Completeness**: Recovery-guidance content has no
  criterion — Resolved
- 🟡 **Testability**: Unchecked-findings precondition can't be built —
  Resolved
- 🟡 **Testability**: Usage-limit criterion non-reproducible and vague —
  Resolved
- 🟡 **Dependency / Scope**: 0310 and 0307 may both change `RunLedger` —
  Partially resolved (the 0307 entry reads as self-contradictory and is not
  mirrored as a Blocks link)
- 🔵 **Completeness / Scope**: Manifest and outline reconciliation only in
  the criteria — Resolved
- 🔵 **Testability / Clarity**: Partial output undefined — Partially resolved
  (defined for findings, not for notes)
- 🔵 **Dependency**: 0308 coupling understated — Resolved
- 🔵 **Dependency**: Claude Code error-wording dependency — Resolved
- 🔵 **Testability**: Negative wording criterion lacks a concrete check —
  Resolved
- 🔵 **Testability**: No negative case — Resolved
- 🔵 **Clarity**: Actors obscured — Partially resolved (the quarantine actor
  is still unnamed)
- 🔵 **Clarity**: Pipeline jargon — Resolved
- 🔵 **Scope**: Unchecked findings separable — Resolved (narrowed to invalid
  leftovers)
- 🔵 **Completeness**: Open questions lack a resolution path — Resolved
- 🔵 **Dependency**: 0283 missing — Resolved

### New Issues Introduced

- 🟡 **Clarity / Testability**: "Has not offered" in Late writes is
  ambiguous — it could mean "not yet" or "never", and no setup can be built
  for a spawn that meets the precondition
- 🟡 **Clarity / Testability**: The re-offered pair's index is unstated —
  original index after quarantine, or a new one; this decides the output
  path and the late-write classification
- 🟡 **Clarity**: No actor for quarantining leftover findings — the CLI or
  the skill, and before or after the plan is produced
- 🟡 **Testability**: Valid and invalid are undefined for node spawns —
  the classification covers notes but only findings' validation is
  described
- 🟡 **Dependency**: The recovery-guidance criterion needs 0304's rows,
  which conflicts with "either may land first" — no fallback is given for
  0310 landing first
- 🔵 **Scope / Dependency**: Late writes depend on 0308's decision, and on
  0307's refusal of superseded-run queries; neither item links back
- 🔵 **Clarity / Testability**: The ban on "interrupted" clashes with the
  "interrupted run" row name
- 🔵 **Clarity / Testability**: The corrupt-ledger exception sits only in
  Assumptions and has no criterion
- 🔵 **Completeness / Testability**: The user-facing report folds the J
  invalid spawns into "offered again"
- 🔵 **Clarity**: Nothing says that the user decides whether the earlier
  session showed a usage limit
- 🔵 **Clarity**: The observed `rate_limit` / HTTP 429 error could be
  confused with the existing `rate_limited` row
- 🔵 **Dependency**: The step 7 summary layout is shared with 0304's halt
  notice, but no reconciliation is noted for it
- 🔵 **Scope**: Mark the step 7 criterion as a regression guard; quarantine
  could ship separately (suggestions)

### Assessment

All six original major findings except the 0304 ordering are resolved or
narrowed. The open questions, coverage gaps and testability of the criteria
are fixed. The remaining majors cluster on two new requirements: Late writes
(ambiguous precondition, contingent on 0308 and 0307) and Invalid leftover
findings (actor, index, node validity). Moving Late writes to a follow-up
gated on 0308, and pinning the actor and index for quarantine, would clear
most of them.

## Re-Review (Pass 3) — 2026-10-12T22:31:18+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity / Testability**: "Has not offered" in Late writes is
  ambiguous — Resolved (late writes left to 0308)
- 🟡 **Clarity / Testability**: The re-offered pair's index is unstated —
  Resolved (fresh index; the marker keeps the old one)
- 🟡 **Clarity**: No actor for quarantining leftover findings — Resolved
  (step 3, Clear, from paths `outstanding --start` lists)
- 🟡 **Testability**: Valid and invalid undefined for node spawns — Resolved
  (the plan accepts or rejects the note)
- 🟡 **Dependency**: The recovery-guidance criterion needs 0304's rows —
  Resolved (inline fallback, mirrored in 0304)
- 🔵 **Scope / Dependency**: Late writes depend on 0308 and 0307 — Partially
  resolved (removed, but 0310 and 0308 each name the other as the owner of
  any follow-up)
- 🔵 **Clarity / Testability**: The ban on "interrupted" clashes with the row
  name — Resolved (renamed "unfinished run")
- 🔵 **Clarity / Testability**: Corrupt-ledger exception only in Assumptions
  — Resolved
- 🔵 **Completeness / Testability**: The report folds J into "offered again"
  — Resolved
- 🔵 **Clarity**: Who judges the usage limit — Resolved
- 🔵 **Clarity**: `rate_limit` versus `rate_limited` — Resolved
- 🔵 **Dependency**: Shared step 7 summary layout with 0304 — Resolved (0310
  no longer changes the summary)
- 🔵 **Scope**: Mark the step 7 criterion as a regression guard — Resolved

### New Issues Introduced

- 🔵 **Dependency**: 0310 and 0308 each name the other as the owner of
  late-write reporting, so no item tracks it
- 🔵 **Testability**: AC3's banned wording may clash with the existing
  warning's conditional "it stops at its next batch"; nothing says whether
  the report replaces that warning
- 🔵 **Testability**: The CLI's list of paths to quarantine has no direct
  criterion
- 🔵 **Testability**: No criterion checks that the marker keeps the old
  index reserved
- 🔵 **Testability**: No verification approach is named for the skill-side
  criteria (AC1, AC3, AC6, AC8)
- 🔵 **Clarity**: The prior run's ledger is called "replaced", "readable",
  "stale" and "superseded"; "stale" is undefined
- 🔵 **Clarity**: "The plan" is both the CLI's JSON and an actor
- 🔵 **Clarity**: "Researcher" names both the human persona and the
  subagent
- 🔵 **Clarity**: Leftover-file labels drift ("unchecked", "leftover",
  "written but invalid", "written by that run")
- 🔵 **Completeness**: No Open Questions section, although late-write
  handling is deferred
- 🔵 **Scope**: Quarantine could ship on its own; the row has two forms
  depending on landing order
- 🔵 Suggestions: the Summary implies building pick-up, which already works;
  the `rejected` clause and the ellipsis in the report wording; undefined
  "depth" and "composer"; K = N and empty-batch boundary cases; a 0307
  schema change could make old ledgers read as corrupt; a note on how
  invalid notes are handled

### Assessment

No critical or major findings remain, so the work item is ready for
planning. What is left is minor: one-term vocabulary for the replaced ledger
and the leftover files, a single owner for late-write follow-up, and a few
extra criteria for the CLI's quarantine list and the index reservation. The
late-write ownership and the AC3 / existing-warning interaction are worth
fixing before planning; the rest can be absorbed during planning.

## Verdict Change — 2026-10-12T22:39:25+00:00

**Verdict:** APPROVE

Changed from COMMENT by the reviewer after pass 3. Two pass-3 minors were
addressed without a further pass: 0308 now owns reporting late writes if it
lets them land, and the report supersedes the "replaced the ledger of run"
warning with fixed wording, which AC2 and AC3 check. The remaining minors
are left to planning.
