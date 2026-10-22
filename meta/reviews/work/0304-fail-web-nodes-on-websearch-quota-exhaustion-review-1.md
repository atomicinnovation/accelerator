---
type: "work-item-review"
id: "0304-fail-web-nodes-on-websearch-quota-exhaustion-review-1"
title: "Work Item Review: Fail Web Nodes When the WebSearch Quota Runs Out"
date: "2026-10-15T13:42:07+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0304"
work_item_id: "0304"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: ["research", "deep-research", "conduct", "web-profile"]
last_updated: "2026-10-15T21:02:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Fail Web Nodes When the WebSearch Quota Runs Out

**Verdict:** REVISE

The item has every section a story needs, explains why the work matters and
draws careful lines against 0303, 0309 and 0310. All six worked budget
examples are arithmetically correct. The verdict is driven by a handful of
definitions that admit more than one implementation: the halting-class
test, the exhaustion signal, where an unhonoured cap is reported, and the
"largest real `k`" rule. Two further gaps add to it: couplings with 0307
and with the Claude Code quota contract are not recorded, and several
assertions cannot be observed or do not tell right behaviour from wrong.
Scope recommends splitting proactive budget sizing from reactive fail-and-halt.

### Cross-Cutting Themes

- **The budget rule needs its examples to pin it down** (flagged by: clarity,
  testability). "Largest real `k`" has no maximum, and "worst-case demand" and
  "increase with level" are never defined. The orchestrator checked this on
  ~300 random inputs: reading k as the supremum and reading it as the linear
  relaxation `(cap − reserve) / Σ(nodes·weight)` give different final budgets
  on 27 of them. Example: 10 pairs at depth 5 with cap 500 gives 7/4/1/1/1
  under one reading and 7/2/2/1/1 under the other. The six worked examples
  happen to agree under both.
- **Where an unhonoured cap is reported** (flagged by: clarity, testability).
  The Search cap requirement says an invalid value is reported, and the floor
  warning lists it. The `abc`/`0` criteria say "any warning", so they pass
  when nothing is printed.
- **0310's record is out of step** (flagged by: clarity, dependency). 0310
  still expects 0304 to own the agent-usage-limit recovery wording and to
  replace 0310's inline wait-or-raise step. The row 0304 calls "a run that
  did not end" is called "unfinished run" in 0310.

### Findings

#### Major

- 🟡 **Clarity + Testability**: "Largest real `k`" is ill-defined, and the readings diverge
  **Location**: Requirements: Budget allocation
  Demand is a step function of k, so the set of k that fits is half-open and
  has no largest member. The supremum reading and the linear-relaxation
  reading produce different budgets on 27 of ~300 sampled inputs, so tests
  beyond the worked examples cannot settle which is right.
- 🟡 **Clarity**: The halting-class definition does not hold for its only member
  **Location**: Requirements: Halting-failure class
  "Every further spawn in this session would also fail" is false for quota
  exhaustion: openalex, arxiv and composer spawns still succeed, and the quota
  refills. The condition that matters, "the orchestrator survives", appears
  only in Context.
- 🟡 **Clarity**: The exhaustion signal is called "no error", a "notice" and an "error string"
  **Location**: Context / Requirements: Quota exhausted / Assumptions
  The one signal the outcome rests on has three names, and the first suggests
  there is nothing to detect.
- 🟡 **Clarity + Testability**: The unhonoured-value report is unplaced and its criteria pass vacuously
  **Location**: Requirements: Search cap / Floor warning; Acceptance Criteria
  It is unclear whether an invalid value gets its own notice on `--start` or
  appears only inside the floor warning. The `abc`/`0` criteria have no
  fixture that forces any output.
- 🟡 **Testability**: Worst-case demand is asserted but observable only inside the floor warning
  **Location**: Acceptance Criteria
  Five criteria assert a demand figure that no requirement makes
  `outstanding` emit when the floor warning does not fire.
- 🟡 **Testability**: The resume fixture cannot tell declared follow-ups from caps
  **Location**: Acceptance Criteria (resumed run)
  Each written level-2 note declares 2 follow-ups, which equals the cap, so an
  implementation that ignores declared follow-ups also gets 31/16. Verified
  replacement: declarations of 1 and 2 give a tree of 2 + 7, budgets 35/17
  and demand 189.
- 🟡 **Dependency**: No coupling with 0307, which rewrites spawn prompts and the run ledger
  **Location**: Dependencies
  0307's reference-only spawn has no search-budget slot, and its ledger rule
  says new fields must default when absent. Whichever item lands second has
  to reconcile.
- 🟡 **Dependency**: The Claude Code session-search contract is not recorded as an external dependency
  **Location**: Dependencies
  The default cap, the variable parser, the refill and the limit notice
  appear only in Context and Assumptions. The plugin's minimum version,
  v2.1.144, is below v2.1.212, where the quota was introduced.
- 🟡 **Scope**: Proactive budget sizing and reactive fail-and-halt can be delivered separately
  **Location**: Summary / Requirements
  The title names only the halt. Sizing, persistence, delivery, provenance
  and the floor warning make up most of the requirements and about ten of the
  21 criteria, and they touch the CLI, the ledger, the corpus schema and the
  docs.

#### Minor

- 🔵 **Clarity + Dependency**: 0310's row naming and owed usage-limit wording disagree with 0310
  **Location**: Dependencies / Requirements
  Use 0310's name for the row ("unfinished run"), and say that 0310's inline
  wait-or-raise step stays as final.
- 🔵 **Clarity**: The Summary overstates the guarantee
  **Location**: Summary
  "Without overrunning it" contradicts the Assumptions, which treat overrun as
  expected and recoverable.
- 🔵 **Dependency**: Reliance on 0303's mechanisms is not marked as landed or blocking
  **Location**: Dependencies
- 🔵 **Dependency**: Overlapping web-profile edits with 0309 have no reconciliation point
  **Location**: Dependencies
- 🔵 **Scope**: `search_budget` provenance is a separable observability add-on
  **Location**: Requirements: Budget provenance
- 🔵 **Testability**: Reserve rounding is never exercised
  **Location**: Acceptance Criteria
  The caps used are 200 and 500, and 5% of each is a whole number. Note that
  10 pairs at cap 201 still gives 3/2/1 @190, so a fixture must be chosen
  where the extra unit of reserve changes the outcome.
- 🔵 **Testability**: The follow-up cap of 1 below level 3 is only implied
  **Location**: Requirements: Budget allocation
- 🔵 **Testability**: The fixed-for-the-run criterion has no scenario where resizing would differ
  **Location**: Acceptance Criteria
- 🔵 **Testability**: The quota-exhausted researcher criterion lacks a notice fixture and the partial-gathering case
  **Location**: Acceptance Criteria
- 🔵 **Testability + Completeness**: Requirements without criteria
  **Location**: Acceptance Criteria
  The uncovered requirements are a mixed web and openalex run, the 15-pair
  floor boundary (verified: 195 > 190, so it warns), the three remedies, a
  halt inside a refusal round, affected spawns also listed under Failures,
  the configure docs, and parse edge cases.

#### Suggestions

- 🔵 **Scope**: The general halting-failure class has a single member
  **Location**: Requirements: Halting-failure class
- 🔵 **Clarity**: "Summary" means three different things
  **Location**: Requirements
- 🔵 **Completeness**: Open Questions is an empty heading
  **Location**: Open Questions

### Strengths

- ✅ All six worked budget examples check out against the rule; three
  agents verified them independently.
- ✅ The examples exercise distinct paths: a shallow top-up, top-up blocked
  by the shape rule, the floor case, and a resume in which a level takes no
  budget.
- ✅ Boundaries with 0303, 0309 and 0310 are explicit, and the Drafting Notes
  record why agent usage limits and the refill recovery were dropped.
- ✅ Every requirement names its actor, and halting outcomes are specified as
  observable state on disk and in the run summary.
- ✅ The Assumptions state the risks in the external contract openly.

### Recommended Changes

1. **Restate the budget rule as an unambiguous procedure** (addresses: "largest
   real `k`"; the follow-up cap of 1 below level 3). Define worst-case demand as
   Σ nodes × budget. Name the reading of k the examples assume, either the
   supremum (budgets just below the first breakpoint that fails) or the
   linear relaxation. Write "no level's budget may exceed the level above's".
   Say that the follow-up cap stays 1 for every level below the third. Add one
   example where the two readings diverge.
2. **Define the halting class by what it guarantees** (addresses: halting-class
   definition). The test is: the orchestrator survives, and further spawns
   would waste or degrade work until the user recovers. State that a halt stops
   every kind of spawn.
3. **Name the session-limit notice once** (addresses: three names for the
   signal). Use that name in Context, Requirements and Assumptions, and give a
   representative description to use as an eval fixture.
4. **Place the unhonoured-value report and give its criteria fixtures**
   (addresses: unplaced report and vacuous criteria).
5. **Make budgets and demand observable** (addresses: demand not observable).
   For example, `outstanding --start` reports both. Alternatively, drop demand
   from the criteria that do not warn.
6. **Fix the discriminating fixtures** (addresses: resume fixture, reserve
   rounding, fixed-for-the-run). Use declarations of 1 and 2 (35/17 @189), a
   cap whose reserve rounds, and a second plan whose resized budgets would
   differ.
7. **Add 0307 and the Claude Code contract to Dependencies, and tidy the
   0303, 0309 and 0310 entries** (addresses: the dependency findings and the
   0310 mismatch). Make the ledger budget field optional with a default.
8. **Decide on the split** (addresses: the scope findings). Either raise a
   sibling "Size Web Search Budgets from the Session Cap" under 0121, or keep
   the item whole and retitle it to cover both halves.
9. **Smaller fixes**: add the missing criteria, soften the Summary's guarantee,
   name the run summary and the researcher's reply separately, and fill Open
   Questions with "None".

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The item reads clearly overall and every worked example matches
the rule. The clarity problems sit in a few key definitions: the halting
class, the exhaustion signal, where an unhonoured value is reported, the
undefined terms in the budget rule, and the naming of 0310's row.

**Strengths**: Every requirement names its actor. The worked examples act as
a reference reading of the rule. Context sets out why the halting class is
limited to failures the orchestrator survives. The recovery wording is
concrete.

**Findings**:
- major / high: Halting-class definition does not hold for its only member
  (Requirements: Halting-failure class). Restate it around orchestrator
  survival, and say a halt stops non-web spawns too.
- major / medium: The exhaustion signal is called "no error", a "notice" and
  an "error string" (Context / Requirements / Assumptions). Pick one term.
- major / medium: Unclear whether an unhonoured value is reported on its own
  or only inside the floor warning (Search cap / Floor warning).
- minor / high: Terms in the budget rule are undefined or unsatisfiable
  (worst-case demand, largest k, increase with level, missing frontier nodes).
- minor / high: 0310's row is named differently, and 0310 still expects this
  item to own the usage-limit wording.
- minor / medium: The Summary overstates what budget sizing guarantees.
- suggestion / low: "Summary" refers to three different things.

### Completeness

**Summary**: Complete for a story. Every section is present and has real
content, and there are 21 Given/When/Then criteria. The gaps are small: the
Open Questions section is empty, and the configure-docs requirement has no
criterion.

**Strengths**: The Context names a concrete failed run and the documented
quota behaviour. The beneficiary is clear. The requirements are specific
enough to implement. The criteria are concrete and varied. The supporting
sections are all filled in.

**Findings**:
- minor / high: The configure-skill documentation requirement has no
  acceptance criterion.
- suggestion / medium: The Open Questions section is an empty heading.

### Dependency

**Summary**: Sibling relationships are mapped with care. Three gaps remain:
the Claude Code quota contract is not recorded, there is no coupling with
0307, and 0303 and 0309 are listed as loosely related although this item
builds on 0303 and edits the same text as 0309.

**Strengths**: The 0310 relationship is explicit about status and scope. The
0303 touchpoints are named. The Assumptions surface the risks in the external
contract.

**Findings**:
- major / high: The Claude Code session-search contract is not captured as an
  external dependency (cap, parser, notice; minimum version v2.1.144 is below
  v2.1.212).
- major / high: No coupling with 0307, which changes spawn prompts and the run
  ledger. The ledger field must default when absent.
- minor / medium: Reliance on 0303's landed mechanisms is not stated as a
  prerequisite.
- minor / medium: The overlapping web-profile edits with 0309 are not
  captured.
- suggestion / medium: 0310's record still expects 0304 to own the
  agent-usage-limit recovery.

### Scope

**Summary**: One problem with careful boundaries, but the item bundles
reactive fail-and-halt with proactive budget sizing. The title describes only
the smaller half, and the work spans five or more components and 21 criteria.

**Strengths**: Sibling boundaries are explicit. The halting class is narrowed
on purpose. Out-of-scope decisions are written down. The reactive half stands
on its own.

**Findings**:
- major / medium: Proactive budget sizing and reactive fail-and-halt can be
  delivered separately. Split them into a sibling story under 0121.
- minor / medium: `search_budget` provenance is a separable observability
  add-on.
- suggestion / low: The general halting-failure class has a single member.

### Testability

**Summary**: Highly testable, with concrete criteria and exact figures; all
six worked examples check out. The gaps are observability (demand), criteria
that pass vacuously (unhonoured values), and fixtures that cannot tell right
behaviour from wrong (resume, reserve rounding, fixed budgets).

**Strengths**: The numeric criteria are correct and cover distinct paths of
the rule. Halting is specified as observable outcomes. The negative criteria
can be checked by diff.

**Findings**:
- major / high: Worst-case demand is asserted but observable only inside the
  floor warning.
- major / high: The unhonoured-value criteria can pass vacuously.
- major / high: The resume fixture cannot tell declared follow-ups from caps.
- minor / medium: "Largest real k" has no maximum, and the intended limit is
  unstated.
- minor / high: Reserve rounding is never exercised.
- minor / medium: The follow-up cap below level 3 is only implied by the
  depth-5 example.
- minor / high: The fixed-for-the-run criterion lacks a scenario where
  resizing would differ.
- minor / medium: The quota-exhausted researcher criterion lacks a notice
  fixture and the partial-gathering case.
- minor / medium: Several requirements have no criterion: a mixed run, the
  15-pair boundary, the remedies, a halt in a refusal round, affected spawns
  under Failures, the configure docs, and parse edge cases.

## Re-Review (Pass 2) — 2026-10-15T17:24:21+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity + Testability**: "Largest real `k`" is ill-defined — Resolved
  (all 13 numeric criteria re-derived by two lenses; the 7/4/1/1/1
  criterion tells the two readings apart)
- 🟡 **Clarity**: The halting-class definition does not hold for its only member — Resolved
- 🟡 **Clarity**: Three names for the exhaustion signal — Resolved
- 🟡 **Clarity + Testability**: The unhonoured-value report is unplaced and its criteria pass vacuously — Partially resolved
  (the notice is placed; the no-web-pairs criterion still cannot fail, and
  whether the notice and the floor warning both name the value is open)
- 🟡 **Testability**: Worst-case demand is not observable — Resolved (Plan report)
- 🟡 **Testability**: The resume fixture cannot tell declared follow-ups from caps — Resolved
- 🟡 **Dependency**: No coupling with 0307 — Partially resolved (recorded
  in 0304; 0307 has no matching entry)
- 🟡 **Dependency**: Claude Code contract not recorded — Resolved
- 🟡 **Scope**: Budget sizing and fail-and-halt are bundled — Partially resolved
  (retitled and kept whole by decision; the reason is not yet in Drafting Notes)
- 🔵 **Clarity + Dependency**: 0310 mismatch — Resolved (0310 amended)
- 🔵 **Clarity**: The Summary overstates the guarantee — Resolved
- 🔵 **Dependency**: 0303 not marked as landed — Partially resolved (marked in 0304;
  0303's own status is still `draft`)
- 🔵 **Dependency**: 0309 overlap — Partially resolved (recorded in 0304 only)
- 🔵 **Scope**: `search_budget` is a separable add-on — Still present (declined)
- 🔵 **Testability**: Reserve rounding never exercised — Resolved (cap 201)
- 🔵 **Testability**: Follow-up cap below level 3 only implied — Resolved
- 🔵 **Testability**: Fixed-for-the-run criterion has no fixture — Resolved
- 🔵 **Testability**: Researcher criterion lacks a fixture — Partially resolved
  (partial-gathering case added; no trial count)
- 🔵 **Testability + Completeness**: Requirements without criteria — Resolved
- 🔵 **Scope**: Single-member halting class — Still present (declined)
- 🔵 **Clarity**: "Summary" means three things — Resolved
- 🔵 **Completeness**: Empty Open Questions — Resolved

### New Issues Introduced

- 🟡 **Dependency**: 0307 does not record that it must carry 0304's search
  budget; built to its own spec after 0304 lands, it drops the budget from
  research-node prompts.
- 🔵 **Clarity**: The halt is triggered "after step 5" in Halting but on
  any ended spawn in Retry rounds, which run before step 5.
- 🔵 **Testability**: Worst-case tree step 1 can double-count declared,
  unwritten follow-ups; only the resume example excludes that reading.
- 🔵 **Testability**: Step 4's "its demand" can be read as the level's own
  demand, not the total.
- 🔵 **Testability**: More branches without criteria: the exact-fit floor
  boundary (cap 192, 14 pairs, 182), a legacy ledger with no budget field,
  the cap source when it comes from the variable, and the notice only on
  `--start`.
- 🔵 **Testability**: How the Claude Code parse is checked for edge inputs
  (`-5`, `1.5`, `0200`, overflow) is left to undocumented behaviour.
- 🔵 **Completeness**: References omits 0307.
- 🔵 **Dependency**: 0309 and 0303 have no matching entries; 0303's note
  still points the retry-round rule at 0310.
- Suggestions: say what `k` is and what "such level" refers to, explain step
  2, 5 and 7 inline, state the ledger budget field's default, rename the file
  to match the title.

### Assessment

The item is ready for planning: every finding from pass 1 that drove the
REVISE verdict is resolved or settled by a decision. What remains is one
major finding, which needs an entry in another work item (0307) rather than
a change to 0304, and wording tightening in the allocation procedure and the
halt trigger. Addressing the 0307 entry and the step 1 and step 4 wording
before planning would remove the last ambiguities an implementer could act
on.

## Re-Review (Pass 3) — 2026-10-15T18:26:56+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Dependency**: 0307 does not record that it must carry the budget — Resolved
  (both items now give the same route through the node query)
- 🔵 **Clarity**: Two trigger points for the halt — Resolved
- 🔵 **Testability**: Step 1 can count follow-ups twice — Resolved
- 🔵 **Testability**: Step 4's "its demand" — Resolved
- 🔵 **Testability**: More branches without criteria — Resolved
  (exact-fit 192/14/182, legacy ledger, cap source)
- 🔵 **Testability**: Claude Code parse edge cases — Partially resolved
  (recorded as an Assumption for the plan; no expected-outcome table)
- 🔵 **Completeness**: References omits 0307 — Resolved
- 🔵 **Dependency**: 0309 and 0303 have no matching entries — Resolved
- 🔵 **Scope**: Reason for keeping the item whole not recorded — Resolved
- 🔵 **Scope**: Single-member halting class — Still present (declined)

### New Issues Introduced

- 🟡 **Testability**: No verification method for how the researcher reacts
  to the undocumented session-limit notice. Split into a structural check
  on the profile's wording and a named behavioural check with a fixture
  notice.
- 🔵 **Clarity**: "Halt" is applied to the run, which keeps running;
  "halted run" needs a definition.
- 🔵 **Clarity + Testability**: The halt notice's "failed spawns" and "pairs
  left unspawned" have unclear boundaries (halted capacity refusals,
  partly spawned pairs).
- 🔵 **Clarity**: "First plan" is not tied to `outstanding --start`, and
  whether the floor warning repeats on later plans is unstated.
- 🔵 **Testability**: Step 3 does not branch to the floor case when no k
  fits.
- 🔵 **Testability**: The `/clear` re-run criterion depends on a Claude Code
  action; restate it against `outstanding --start` after an ended halted
  run.
- 🔵 **Dependency**: The docs-site `outstanding` reference is not named as
  a consumer of the new plan fields; the blocked-by edges are not mirrored
  as `blocks` on 0303 and 0310.
- 🔵 **Scope**: Work that falls to this item depending on landing order
  (0307, 0309) has no conditional criterion.
- 🔵 **Completeness**: The Summary is long and does not name the
  beneficiary.
- Suggestions: a shallow-depth example (10 pairs at depth 2 gives 7/3 at
  190; checked by the orchestrator), an actor for "the cap applies in
  every environment", tense in Context, which notes declare the follow-ups
  in the resume criterion, a gloss for `single_pass`, wording for the k
  candidate set, and 0308 as another ledger reader.

### Assessment

The item is ready for planning. All numeric criteria were re-derived
independently by two lenses and match. The one major finding concerns how
a criterion will be verified, not what the item specifies: the plan can
settle it, or the criterion can be split now. The remaining findings are
minor wording and edge-definition tightening.

## Re-Review (Pass 4) — 2026-10-15T18:53:59+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Testability**: No method for checking the researcher's reaction — Resolved
  by decision (the criterion is structural only; the Assumption records that
  the behaviour itself goes unverified)
- 🔵 **Clarity**: "Halted run" not defined — Resolved
- 🔵 **Clarity + Testability**: Edges of the halt notice's lists — Partially resolved
  (the A/B/C criterion added; membership for pairs outside the window, or
  unfinished for other reasons, is still open)
- 🔵 **Clarity**: First plan not tied to `--start` — Partially resolved
  (defined; a plan that sizes a ledger with no budgets is not covered)
- 🔵 **Testability**: Step 3 floor branch — Resolved
- 🔵 **Testability**: `/clear` re-run criterion — Resolved
- 🔵 **Dependency**: Docs-site consumer and `blocks` edges — Resolved
  (0304 has no `blocked_by` frontmatter yet)
- 🔵 **Scope**: Work that depends on landing order — Resolved (conditional criteria)
- 🔵 **Completeness**: Dense Summary, no beneficiary — Resolved
- 🔵 **Scope**: Single-member halting class — Still present (declined)

### New Issues Introduced

- 🔵 **Testability**: ⌈5% of the cap⌉ invites a floating-point off-by-one
  (`0.05 * 60` evaluates to `3.0000000000000004`, giving a reserve of 4);
  state it as ⌈cap / 20⌉ and add cap 60 giving a reserve of 3.
- 🔵 **Clarity + Testability**: Report and floor warning are tied to the
  "first plan", but a ledger with no budgets is sized by a later plan.
- 🔵 **Clarity**: "Plan" also means the implementation plan in Assumptions.
- 🔵 **Clarity**: Which rule governs cap parsing, Claude Code's behaviour or
  the stated rule, is unclear.
- 🔵 **Testability**: No depth-1 criterion (10 pairs gives 19 at 190) and
  no check that findings carry no `search_budget`.
- 🔵 **Clarity**: Domain terms (pair, frontier, composer, finding) and step 2
  are not glossed.
- 🔵 **Dependency**: No `blocked_by` frontmatter; 0303's status still `draft`.
- Suggestions: phase the plan along the two halves, budgets for the `abc`
  criterion (42/19/9 at 190), one term for the run after a halt, how quota
  exhaustion is triggered for checking, follow-up caps from 0283, the cap
  visible in the Bash environment.

### Assessment

No major findings remain, and completeness reports none at all. The item is
ready for planning. The remaining minors are precision tightening; the
reserve's integer arithmetic and the sizing-plan wording are the two an
implementer would most likely get wrong.

### Changes applied after pass 4 (no further pass run)

All remaining minors were applied as one batch: the reserve as ⌈cap / 20⌉
with a cap-60 criterion (13/5/3 at 57); a defined "sizing plan" that owns
the plan report and floor warning, including for ledgers with no budgets;
unfinished pairs defined as every web pair with outstanding web nodes when
step 7 runs; "implementation plan" in Assumptions; Claude Code's parse
behaviour named as authoritative; a depth-1 criterion (19 at 190, no
`search_budget` in findings); budgets in the `abc` criterion (42/19/9 at
190); `blocked_by` frontmatter; glosses for pair, frontier, composer,
finding, `single_pass`, step 2 and `research topic outstanding`; and
"re-run" as the single term for the new run after a halt.

### Verdict changed to APPROVE — 2026-10-15T21:02:08+00:00

The reviewer approved the work item after the changes applied following
pass 4.
