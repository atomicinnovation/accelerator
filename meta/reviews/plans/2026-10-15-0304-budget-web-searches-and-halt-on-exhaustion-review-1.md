---
type: "plan-review"
id: "2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion-review-1"
title: "Plan Review: Budget Web Searches and Halt on Exhaustion Implementation Plan"
date: "2026-10-15T22:15:45+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "usability", "compatibility", "standards", "documentation"]
review_number: 1
review_pass: 3
tags: ["research", "deep-research", "conduct", "web-profile", "research-cli", "run-ledger"]
last_updated: "2026-10-15T23:33:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Budget Web Searches and Halt on Exhaustion Implementation Plan

**Verdict:** REVISE

The plan is technically strong. The correctness lens checked every numeric
sizing row and every worst-case tree row by hand, and all of them reproduce.
The layering into domain, adapters and composition root is clean, and every
contract change is additive. It needs revising for seven major findings,
none of them critical, in four areas:

- the halt is held only in the orchestrator's memory, not in the ledger;
- a quota-exhausted spawn that still writes a file keeps that file;
- some logic is duplicated or under-structured (follow-up capping, the
  five-stage sizing function);
- three test gaps, one of which breaks an existing structure test.

### Cross-Cutting Themes

- **Halt state lives outside the ledger** (flagged by: architecture,
  usability). Budgets are run-wide state and go into the ledger. The halt is
  also run-wide state, but it depends on the orchestrator re-sending
  `--withhold-profile web` on every `--run`. If one call drops the flag, web
  spawns come back and are acknowledged as attempted. That breaks the
  invariant the CLI-side halt exists to protect.
- **Intermediate phases promise a halt that does not exist yet** (flagged
  by: documentation, compatibility, usability). From Phase 3, the floor
  warning and the configure text say "expect a halt and re-run conduct". The
  halt only arrives in Phase 6, and the remedy leaves out `/clear`.
- **Phase independence is overstated** (flagged by: correctness,
  documentation). Phase 2 uses `SearchCap` and `searched_tree` from
  Phase 1. Phases 4 and 5 have no CHANGELOG entries of their own, although
  each is meant to be mergeable alone.
- **`SearchCapSource` diverges from `CapSource`** (flagged by:
  code-quality, standards). It differs in receiver shape, in what
  `Variable` carries, and in the type of `unhonoured`, with no reason given.
- **Domain names read against their meaning** (flagged by: code-quality,
  standards). `sizes_searches()` is true when no budgets exist.
  `searched_tree` means "still to search".
- **Doc comments restate code** (flagged by: code-quality, standards). These
  are the comments on `levels()` and `searched_tree`, and the `WorstCaseTree`
  type doc. The JavaScript-parity doc comment also stays on
  `subagent_cap::from_environment` instead of moving with the parse
  (flagged by: documentation, standards).
- **Notice ordering after the move is unspecified and untested** (flagged
  by: test-coverage, correctness, compatibility). The plan does not say
  where the corrupt-ledger notice lands relative to the moved notices. No
  test pins the combined order, and a failing `--start` would no longer
  print its concurrency warnings.

### Tradeoff Analysis

- **Ledger-held halt vs stateless CLI flag**: a halt stored in the ledger
  survives context compaction and a slip by the orchestrator. The flag keeps
  the ledger smaller and the CLI's halt semantics explicit on each call.
  Recommendation: record the halt in the ledger on its first
  `--withhold-profile`, and echo `withheld` in the plan JSON. This follows
  the plan's own justification for storing budgets.
- **Hard-coded `SEARCHING_PROFILE` vs a capability declared by each
  profile**: the constant is the simpler choice and enough for now. Name it
  under What We're NOT Doing so the deferral is deliberate.

### Findings

#### Major

- 🟡 **Architecture + Usability**: The run-wide halt is held in the
  orchestrator's memory, not in the run ledger.
  **Location**: Phase 6 §3 Flag; §6 Step 2.
  One `--run` call without the flag puts web spawns back in the window. The
  ledger marks them attempted, and they surface as failures again.
- 🟡 **Correctness**: A quota-exhausted spawn that still writes a file
  keeps it.
  **Location**: Phase 6 §6 Step 5.
  The re-plan accepts the file, the node is marked complete, and an
  unsearched note passes as a searched one. That is the failure this work
  item exists to prevent.
- 🟡 **Code Quality**: `declared_follow_ups` copies the capping and
  deduplication logic of `follow_ups`.
  **Location**: Phase 1 §2.
  If the derivation rules change, the copy can drift from them silently,
  and budgets would then be sized against the wrong tree.
- 🟡 **Code Quality**: The five-stage sizing algorithm is specified as one
  function, and its scaled arithmetic is unexplained.
  **Location**: Phase 1 §4.
  The literal 4 has three different meanings. Split the function into named
  steps and give the constants names.
- 🟡 **Test Coverage**: Phase 3 breaks an existing structure test, and the
  plan has no red step for it.
  **Location**: Phase 3 §8.
  `test_research_topic_passes_the_concurrency_flag_through`
  (`test_research_structure.py:245-248`) pins "a corrupt run ledger or a
  clamped concurrency", which Phase 3 rewrites.
- 🟡 **Test Coverage**: The AC 28 halt-notice scenario and the shape of
  `unanswered` entries are pinned only in prose.
  **Location**: Phase 6 §5.
  No CLI test asserts the `stage` values, partly researched pairs,
  compose-stage pairs, or the post-`end-run` tick plan.
- 🟡 **Test Coverage**: No test decodes the empty stored budgets of a
  no-web run on a later plan.
  **Location**: Phase 3 §3.
  If `{"levels": {}, "demand": 0}` decodes as `Corrupt`, every no-web run
  would break on its second plan, and every listed test would still pass.

#### Minor

- 🔵 **Correctness**: The worst-case tree stores one entry per level, so an
  unbounded `--depth` can exhaust memory, query plans included.
  **Location**: Phase 1 §2.
- 🔵 **Correctness**: Phase 2 depends on Phase 1 types, although the plan
  calls the two independent.
  **Location**: Implementation Approach; Phase 2 §4, §6.
- 🔵 **Correctness**: The halt notice has no source for unfinished web pairs
  when `end-run` fails.
  **Location**: Phase 6 §6 Step 7.
- 🔵 **Usability / Documentation / Compatibility**: The floor warning and
  the configure text promise a halt before Phase 6, and the remedy leaves
  out `/clear`.
  **Location**: Phase 3 §5, §9.
- 🔵 **Compatibility**: The floor-warning wording and its tests must name
  the variable only through `SearchCapSource`.
  **Location**: Phase 3 §5.
  A literal variable name in `search_wording.rs` fails the coupling guard.
- 🔵 **Compatibility**: The 0307 and 0309 conditional steps are checked only
  when the phase starts, not when it merges.
  **Location**: Phase 4 §8; Phase 5 §4.
- 🔵 **Compatibility**: The Claude Code version window the cap depends on is
  not recorded.
  **Location**: Phase 2 §1, §9.
  The window is no limit before v2.1.212, refills from v2.1.290, and parity
  verified at 2.1.301.
- 🔵 **Compatibility**: Level-note templates that users have overridden lack
  a `search_budget` line.
  **Location**: Phase 4 §2–3.
- 🔵 **Code Quality / Standards**: `SearchCapSource` diverges from the shape
  of `CapSource`.
  **Location**: Phase 2 §4.
- 🔵 **Code Quality / Standards**: `sizes_searches()` and `searched_tree`
  read against their meaning.
  **Location**: Phase 1 §4; Phase 3 §2.
- 🔵 **Code Quality / Standards**: Doc comments in the code sketches restate
  the code.
  **Location**: Phase 1 §2, §4; Phase 2 §2.
- 🔵 **Code Quality**: The renderer decides which spawns get a budget, a
  domain rule that belongs beside `searched_tree`.
  **Location**: Phase 3 §6.
- 🔵 **Code Quality**: The sizing snippet threads the ledger option twice
  and makes `run_outstanding` longer.
  **Location**: Phase 3 §4.
- 🔵 **Code Quality**: `below_frontier` takes two adjacent `u64` counts that
  are easy to swap, and `from_levels` is a public constructor that exists
  only for tests.
  **Location**: Phase 1 §2, §4.
- 🔵 **Architecture**: `OutstandingPair.worst_case` makes sizing data a
  concern of every topic-planner consumer.
  **Location**: Phase 1 §2.
- 🔵 **Architecture**: Which profiles search is hard-coded in both Rust and
  the halting table, and the deferral is unrecorded.
  **Location**: Phase 1 §4; Phase 6 §6.
- 🔵 **Test Coverage**: The empty-value error and the repeated form of
  `--withhold-profile` are untested.
  **Location**: Phase 6 §3.
- 🔵 **Test Coverage**: No test pins the notice order after the start
  notices move.
  **Location**: Phase 2 §6.
- 🔵 **Test Coverage**: The byte-equal pins for the step 2 report only
  arrive in Phase 5, after Phase 3 has already edited step 2.
  **Location**: Phase 5 §3.
- 🔵 **Test Coverage**: The structure tests for AC 27, 30 and 31 are loosely
  specified.
  **Location**: Phase 6 §7.
- 🔵 **Test Coverage**: The domain the enumerated property test covers is
  underspecified.
  **Location**: Phase 1 §3.
- 🔵 **Usability**: `unanswered` is a new synonym for the outstanding set,
  alongside `pairs`.
  **Location**: Phase 6 §4.
- 🔵 **Usability**: `--withhold-profile` accepts unknown profile names
  without warning.
  **Location**: Phase 6 §3.
- 🔵 **Usability**: "from the default" differs from the established
  wording, "Claude Code's default".
  **Location**: Phase 2 §5; Phase 3 §5.
- 🔵 **Standards**: `E_TOPIC_RESEARCH_WITHHOLD` should be
  `E_TOPIC_RESEARCH_WITHHOLD_PROFILE`.
  **Location**: Phase 6 §3.
- 🔵 **Standards**: The CHANGELOG instructions do not follow how
  `[Unreleased]` grows its existing bullets.
  **Location**: Phase 2 §9; Phase 3 §9; Phase 6 §8.
- 🔵 **Documentation**: Phases 4 and 5 have no CHANGELOG step of their own.
  **Location**: Phases 4–5.
- 🔵 **Documentation**: The docs-site sentence that counts the plan's
  fields, "four arrays, two counts and the cap", will go stale.
  **Location**: Phase 2 §9; Phase 3 §9; Phase 6 §8.
- 🔵 **Documentation**: The reserve is described three slightly different
  ways in the skill, the configure text and the code.
  **Location**: Phase 3 §8–9; Phase 1 §4.
- 🔵 **Documentation**: The quota recovery text is duplicated in two tables.
  **Location**: Phase 5 §2; Phase 6 §6.

#### Suggestions

- 🔵 **Test Coverage**: Pin that the plan after a pre-0304 sizing emits no
  further floor warning, and test a budget of 1 at an unstored level end to
  end.
- 🔵 **Test Coverage**: Name a test for the conditional 0309 step, and add
  phrase tests for the docs-site half of AC 37.
- 🔵 **Test Coverage**: Do the parse refactor in two steps, moving the code
  first and the tests second.
- 🔵 **Usability**: The template's literal `search_budget: 1` looks valid.
  Prefer a placeholder.
- 🔵 **Code Quality**: The `holds_back` bit on `RankedSpawn` duplicates
  `Withheld::holds_back`.
- 🔵 **Architecture**: Say that each halting reason must map to a set of
  withheld profiles.
- 🔵 **Standards**: Add an explicit `public-api:update` step to Phases 3 and
  6.
- 🔵 **Standards**: Give ADR-0072 the frontmatter `parent:
  "work-item:0304"` and `relates_to: ["adr:ADR-0071"]`.
- 🔵 **Correctness / Compatibility**: State the full order of start notices,
  and keep concurrency notices printed on a failing `--start`.
- 🔵 **Documentation**: Add a halt-and-recovery paragraph to the docs-site,
  and contrast `unanswered` with `pairs`.

### Strengths

- ✅ The sizing algorithm and the worst-case tree formula reproduce every
  numeric acceptance criterion. The correctness lens checked each row by
  hand. Binary search is sound because demand is monotone in `j`, and the
  top-up's stopping condition is exactly AC 7's maximality condition.
- ✅ Crate placement follows the existing functional-core split: pure
  domain in `research`, the codec in `research-adapters`, and environment
  reads and wording in `research-cli`.
- ✅ Withholding happens CLI-side, after the `attempted` partition, so a
  withheld spawn is never offered, never acknowledged and never reported as
  "wrote no note".
- ✅ Every contract change is additive: plan JSON, an optional ledger key
  with the `owed_quarantine` default, and an optional schema extra. The
  plugin-pinned launcher keeps the skill and the binary in step.
- ✅ The plan says why budgets persist when concurrency did not. `PlanRequest`
  removes a parameter-list smell where it starts. `level_weight` is
  deliberately decoupled from `follow_up_cap`.
- ✅ ADR-0072 extends ADR-0071 rather than editing it, and the coupling
  guard admits the variable by name and file.
- ✅ The plan maps nearly every acceptance criterion to a named test,
  including an enumerated property test and a test that Rust and the prose
  use the same wording.

### Recommended Changes

1. **Store the halt in the ledger** (addresses: run-wide halt state).
   Store a `withheld` set on `RunLedger` that defaults to empty when absent.
   `--withhold-profile` adds to it, and `window` reads it from the ledger.
   Echo `"withheld"` in run plans. Step 2 then passes the flag only on the
   first plan after a halt. Add ledger and codec tests.
2. **Quarantine a halted spawn's output file** (addresses: a quota-exhausted
   spawn keeps its file). In step 5, rename any file at the output path to
   `.…invalid` before the re-plan. Add a structure test.
3. **Extract a shared helper for fresh follow-ups** (addresses:
   `declared_follow_ups` duplicates `follow_ups`). Both `follow_ups` and
   the worst-case count should consume the same pure iterator.
4. **Break `sized` into named steps and name its constants** (addresses:
   the five-stage sizing function). Sketch `budgets_at`,
   `largest_fitting_scale`, `aligned_to_heaviest_level` and `topped_up`, and
   `WEIGHT_SCALE` and `RESERVE_SHARE`.
5. **Close the three test gaps** (addresses: the broken structure test, the
   AC 28 scenario, empty stored budgets).
   - Phase 3: first update the assertion at
     `test_research_structure.py:245-248` (red), then edit the skill.
   - Phase 6: add an AC 28 CLI test asserting the full `unanswered` entries
     on the tick plan.
   - Phase 3: add a positive codec case for empty stored budgets, and a
     follow-up `--run` in `a_run_without_web_pairs_sizes_nothing`.
6. **Correct the phase graph and per-phase docs** (addresses: Phase 2
   depends on Phase 1, the premature halt promise, CHANGELOG gaps).
   - Draw P1 → P2.
   - Word the Phase 3 floor remedy without the halt, and extend it in
     Phase 6 with `/clear`.
   - Give Phases 4 and 5 their own CHANGELOG step, extending existing
     `[Unreleased]` bullets.
7. **Bound `--depth` or make the tree's tail compact** (addresses:
   unbounded `--depth`). Add `a_huge_depth_sizes_without_materialising_every_level`.
8. **Settle names and shapes** (addresses: naming, `SearchCapSource`
   shape, error code).
   - Rename `sizes_searches` to `awaits_search_budgets` and `searched_tree`
     to `unsearched_tree`.
   - Mirror `CapSource`, or justify the divergence.
   - Rename the error code to `E_TOPIC_RESEARCH_WITHHOLD_PROFILE`.
9. **Pin the start-notice order and keep the coupling guard green**
   (addresses: notice order, the floor-warning variable name). State the
   full notice order, and keep the concurrency and corrupt-ledger notices
   where they are. Build `search_floor` text only through
   `SearchCapSource`.
10. **Trim doc comments** (addresses: restating doc comments). Drop the
    `levels()` and `searched_tree` docs, and move the JavaScript-parity doc
    comment onto `positive_whole_number`.

## Per-Lens Results

### Architecture

**Summary**: The plan follows the existing layering and the phase graph is
clean. The main structural concern is that run-wide halt state lives only in
the orchestrator's memory, while budgets go into the ledger, with no reason
given for the difference. Smaller concerns are sizing data leaking into
`OutstandingPair`, and a second hard-coded profile name.

**Strengths**: pure domain sizing and withholding; budget persistence
justified against 0303; CLI-side withholding preserves the ledger
invariants; `harness_setting.rs` extracted before the second variable;
`PlanRequest`; `withheld` on `RunMode::Continue` makes withholding
impossible to express on `--start`; `unanswered` owned by the CLI;
`owed_quarantine`-style defaults.

**Findings**:

- 🟡 major / medium — **Run-wide halt state lives in orchestrator memory,
  not the run ledger** (Phase 6 §3, §6). If one plan call omits the flag,
  web spawns are re-offered and acknowledged as attempted. Make the halt
  sticky in `RunLedger`, or record why the per-call flag is preferred.
- 🔵 minor / medium — **Search-sizing data leaks into every
  `OutstandingPair`** (Phase 1 §2). Carry the trees beside the plan, keyed
  by stem.
- 🔵 minor / medium — **Which profiles spend web searches is hard-coded in
  two places** (Phase 1 §4, Phase 6 §6). Record the deferral, and phrase the
  prose in terms of "searching profiles".
- 🔵 suggestion / low — **Withholding is keyed by profile, while halting is
  framed as a general class** (Phase 6 §2, §6). State that each halting
  reason must map to a set of withheld profiles.

### Code Quality

**Summary**: The plan follows the codebase's layering and orders each phase
red then green. The risks are a hand-mirrored copy of the follow-up logic, a
dense sizing algorithm, domain rules in the renderer, and a growing
`run_outstanding`.

**Strengths**: the `Derivation` struct; `PlanRequest`; `positive_whole_number`
extraction; `level_weight` decoupled from `follow_up_cap`; halting defined
once; pure, injectable sizing; withholding in the domain window.

**Findings**:

- 🟡 major / high — **`declared_follow_ups` duplicates the capping and
  deduplication logic of `follow_ups`** (Phase 1 §2). Extract a shared pure
  iterator.
- 🟡 major / medium — **The five-stage sizing algorithm is one function with
  unexplained scaled arithmetic** (Phase 1 §4). Break it into named steps
  and name the constants.
- 🔵 minor / medium — **Which spawns get a budget is decided in the JSON
  renderer** (Phase 3 §6). Add a domain query such as
  `SearchBudgets::for_spawn`.
- 🔵 minor / high — **The sizing snippet is convoluted and makes
  `run_outstanding` longer** (Phase 3 §4). Move the decision into the
  domain, compute `searched_tree` once, and extract a helper.
- 🔵 minor / medium — **`SearchCapSource` diverges from the shape of
  `CapSource` without reason** (Phase 2 §4).
- 🔵 minor / medium — **`sizes_searches()` reads as the opposite of what it
  means** (Phase 3 §2).
- 🔵 minor / medium — **`below_frontier`'s signature is primitive-heavy, and
  `from_levels` is a public constructor only tests need** (Phase 1 §2, §4).
- 🔵 suggestion / medium — **Some planned doc comments restate the code**
  (Phase 1 §2, §4).
- 🔵 suggestion / low — **Withholding is computed twice: a bit on
  `RankedSpawn` plus `Withheld::holds_back`** (Phase 6 §2).

### Test Coverage

**Summary**: Coverage is strong. Each numeric acceptance criterion maps to a
named test, there is an enumerated property test, and boundaries are
covered deliberately. The gaps are where behaviour crosses components:
AC 28 and the shape of `unanswered`, an existing structure test that
Phase 3 breaks, and decoding of empty stored budgets.

**Strengths**: numeric acceptance criteria are pinned at both the unit and
the CLI level; the AC 7 property test is deterministic; boundaries at the
reserve and at `overruns`, and `u32::MAX`; window withholding edge cases;
Rust and prose wording checked against each other; the pre-0304 ledger is
covered on both paths.

**Findings**:

- 🟡 major / high — **Phase 3 breaks an existing structure test with no red
  step** (Phase 3 §8). The test is
  `test_research_topic_passes_the_concurrency_flag_through`, at
  `test_research_structure.py:245-248`, and was verified.
- 🟡 major / high — **The AC 28 halt-notice scenario and the shape of
  `unanswered` are only pinned in prose** (Phase 6 §5).
- 🟡 major / medium — **No test that a no-web run's empty stored budgets
  decode on a later plan** (Phase 3 §3).
- 🔵 minor / high — **The empty-value error and the repeated form of
  `--withhold-profile` are untested** (Phase 6 §3).
- 🔵 minor / medium — **The notice order after the start-notice push moves
  is untested** (Phase 2 §6).
- 🔵 minor / medium — **The byte-equal pins of the step 2 report arrive
  after Phase 3 edits step 2** (Phase 5 §3).
- 🔵 minor / medium — **The halted-refusal and Failures-listing tests are
  loosely specified against AC 27, 30 and 31** (Phase 6 §7).
- 🔵 minor / medium — **The property test's enumeration domain is
  underspecified** (Phase 1 §3).
- 🔵 suggestion / medium — **No test that the floor warning stays silent on
  the plan after a pre-0304 sizing, and none for the default budget at an
  unstored level** (Phase 3 §7).
- 🔵 suggestion / medium — **The conditional 0309 step names no test**
  (Phase 5 §4).
- 🔵 suggestion / low — **The parse refactor rewrites its own safety net in
  the same step** (Phase 2 §2).

### Correctness

**Summary**: The arithmetic holds. Every sizing row and every tree row was
checked by hand, and withholding sits correctly after the attempted
partition. The remaining issues are at the edges:

- a quota-exhausted spawn's stray file;
- unbounded depth;
- Phase 2 depending on Phase 1;
- the halt notice when `end-run` fails.

**Strengths**: all 13 non-floor sizing rows, plus 9/5, were checked by hand;
scaling by 4 gives exactly the required candidate set; binary-search bounds
are sound; the top-up terminates and equals maximality; the worst-case count
is a true upper bound; withholding preserves `unfinished` and frees slots;
budgets survive `continue_as` and `record`.

**Findings**:

- 🟡 major / medium — **A quota-exhausted spawn that still writes a file
  is recorded as failed, but its file is kept** (Phase 6 §6 Step 5).
  Quarantine the file before the re-plan.
- 🔵 minor / high — **The worst-case tree stores one entry per level up to
  an unbounded `--depth`** (Phase 1 §2).
- 🔵 minor / high — **Phase 2 uses Phase 1 types although the plan calls
  them independent** (Implementation Approach).
- 🔵 minor / medium — **The halt notice has no source for unfinished web
  pairs when `end-run` fails** (Phase 6 §6 Step 7).
- 🔵 suggestion / low — **Moving the start notices may reorder them
  relative to the corrupt-ledger notice** (Phase 2 §6).

### Usability

**Summary**: Strong for both consumers, the human user and the LLM
orchestrator. The remaining friction is that the orchestrator must restate
the halt on every call, the floor remedies leave out `/clear`, `unanswered`
is a new near-synonym, and the default-cap wording differs from the
existing wording.

**Strengths**: the `search_` field vocabulary mirrors `subagent_cap`; the
unhonoured-value phrasing is reused; the error message mirrors the
`--spawned` one; warnings fire only once; the halt notice separates the
spawns that ended with it from the unfinished pairs and rules out the
refill; the session-limit notice is matched loosely; withholding is
CLI-side.

**Findings**:

- 🟡 major / medium — **The orchestrator must restate the halt state on
  every continue call** (Phase 6 §3, §6).
- 🔵 minor / high — **The floor warning's remedies leave out the steps that
  make them work** (Phase 3 §5, §9).
- 🔵 minor / medium — **`unanswered` is a new synonym for the outstanding
  set alongside `pairs`** (Phase 6 §4).
- 🔵 minor / medium — **`--withhold-profile` accepts unknown profile names
  without warning** (Phase 6 §3).
- 🔵 minor / medium — **The default-cap phrasing differs from the existing
  concurrency diagnostics** (Phase 2 §5, Phase 3 §5).
- 🔵 suggestion / low — **The template's default `search_budget` value
  relies on the LLM deleting it** (Phase 4 §2).

### Compatibility

**Summary**: Every contract change is additive. The ledger key is optional,
the schema extra is tolerated by older validators, and the binary is pinned
to the plugin version. The risks are at the edges: when the conditional
steps are checked, intermediate prereleases, the Claude Code version window,
user-overridden templates, and the coupling guard.

**Strengths**: the ledger codec follows `owed_quarantine`, and empty
budgets prevent resizing; old binaries tolerate new ledgers; plan JSON
changes are additive; the parse is reused with evidence of parity; the
schema extra is tolerated in both directions; the plugin-pinned launcher
keeps the skill and the binary in step.

**Findings**:

- 🔵 minor / medium — **The 0307 and 0309 conditionals are checked only
  when a phase starts, not when it merges** (Phase 4 §8, Phase 5 §4).
- 🔵 minor / medium — **Intermediate releases ship warnings that promise a
  halt not yet implemented** (Phase 3 §5, §9).
- 🔵 minor / medium — **The Claude Code version window the search cap
  depends on is not recorded** (Phase 2 §1, §9; Phase 3 §9).
- 🔵 minor / medium — **User-overridden level-note templates lack the
  `search_budget` line the outputter expects** (Phase 4 §2–3).
- 🔵 minor / medium — **The floor-warning wording and its tests must not
  spell out the harness variable outside the admitted files** (Phase 3 §5,
  Phase 2 §8).
- 🔵 suggestion / low — **Moving start notices after the plan is computed
  changes stderr on a failing `--start`** (Phase 2 §6).

### Standards

**Summary**: The plan follows conventions closely: it mirrors 0303, adds a
new ADR rather than editing ADR-0071, keeps the plan JSON additive, and
documents the exit-code table. The gaps are small consistency issues: the
error-code name, the shape of `SearchCapSource`, ambiguous names, doc
comments that restate code, and how the CHANGELOG grows.

**Strengths**: ADR-0072 extends ADR-0071 rather than editing it; the
coupling guard admits the variable by name and file; the error phrasing
matches the existing errors; plan JSON is additive; the template's comment
alignment is kept; module names mirror their 0303 siblings.

**Findings**:

- 🔵 minor / medium — **The error code does not follow the flag-named
  pattern** (Phase 6 §3). Use `E_TOPIC_RESEARCH_WITHHOLD_PROFILE`.
- 🔵 minor / medium — **`SearchCapSource` diverges in shape from its
  `CapSource` precedent** (Phase 2 §4).
- 🔵 minor / medium — **Some new domain names read against their meaning**
  (Phase 1 §4, Phase 3 §2).
- 🔵 minor / medium — **Several doc comments in the code sketches restate
  what the code expresses** (Phase 1 §2, §4; Phase 6 §2).
- 🔵 minor / medium — **The CHANGELOG instructions do not follow how
  `[Unreleased]` grows existing entries** (Phase 2 §9, Phase 3 §9,
  Phase 6 §8).
- 🔵 suggestion / medium — **The `public-api:update` step is listed only in
  Phase 1** (Phases 3, 6).
- 🔵 suggestion / low — **ADR-0072's linkage frontmatter is not specified**
  (Phase 2 §1).

### Documentation

**Summary**: Documentation is treated as part of the deliverable, and the
key prose is pinned by tests. The gaps come from the phasing: a halt is
promised early, CHANGELOG entries are missing for some phases, and existing
prose (the field-count sentence, the parity doc comment) will go stale.

**Strengths**: each phase adds its own docs step; the prose is pinned by
structure and wording-parity tests; the configure paragraph mirrors its
neighbour; the migration notes are clear; ADR-0072 records the departures
from ADR-0071; most doc comments carry the reason, not the mechanics.

**Findings**:

- 🔵 minor / high — **The Phase 3 docs and the floor warning promise a halt
  that only arrives in Phases 5–6**.
- 🔵 minor / high — **The CHANGELOG entries for Phase 4 and Phase 5
  behaviour are missing or deferred**.
- 🔵 minor / high — **The docs-site sentence that counts the plan's fields
  will go stale as fields are added** (`research.md:464`, plus the node
  field list at `:480-484`).
- 🔵 minor / medium — **The parse-semantics doc comment is not moved with
  the parse** (Phase 2 §2).
- 🔵 minor / medium — **The reserve is described inconsistently across the
  skill, the configure text and the code**.
- 🔵 minor / medium — **The quota-exhausted recovery text is duplicated in
  two tables** (Phase 5 §2, Phase 6 §6).
- 🔵 suggestion / medium — **The docs-site does not explain the halt, or
  how `unanswered` differs from `pairs`** (Phase 6 §8).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-15T23:16:13+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Architecture + Usability**: Run-wide halt state lives in orchestrator memory — Resolved
- 🟡 **Correctness**: A quota-exhausted spawn that still writes a file keeps it — Resolved
- 🟡 **Code Quality**: `declared_follow_ups` duplicates `follow_ups` — Partially resolved (the cap is still applied in two loops; the helper is mislabelled "pure")
- 🟡 **Code Quality**: Five-stage sizing as one function — Resolved
- 🟡 **Test Coverage**: Phase 3 breaks an existing structure test — Resolved
- 🟡 **Test Coverage**: AC 28 and `unanswered` shape pinned only in prose — Resolved
- 🟡 **Test Coverage**: Empty stored budgets never decoded — Resolved
- 🔵 **Correctness**: Unbounded `--depth` — Resolved (the fix introduced the new major below)
- 🔵 **Correctness**: Phase 2 depends on Phase 1 — Resolved
- 🔵 **Usability / Documentation / Compatibility**: Premature halt promise — Resolved
- 🔵 **Usability**: Floor remedies omit `/clear` — Partially resolved (the CLI text still omits the restart)
- 🔵 **Compatibility**: Coupling-guard trap in floor wording — Resolved
- 🔵 **Compatibility**: Conditional steps checked only at phase start — Resolved
- 🔵 **Compatibility**: Overridden level-note templates — Resolved
- 🔵 **Code Quality / Standards**: Misleading names — Resolved
- 🔵 **Code Quality / Standards / Documentation**: Restating doc comments and the misplaced parity comment — Resolved
- 🔵 **Standards**: Error code naming — Resolved
- 🔵 **Standards / Documentation**: CHANGELOG growth and per-phase entries — Resolved
- 🔵 **Standards**: `public-api:update` only in Phase 1 — Resolved
- 🔵 **Documentation**: Reserve described inconsistently — Resolved
- 🔵 **Documentation**: Field-count sentence will go stale — Partially resolved (Phase 6 does not update it for `unanswered`)
- 🔵 **Documentation**: Docs-site does not explain the halt — Resolved
- 🔵 **Test Coverage**: Notice order untested — Partially resolved (the floor warning is not in the pinned order; a failing `--start` is untested)
- 🔵 **Test Coverage**: Byte-equal pins arrive too late — Resolved
- 🔵 **Test Coverage**: AC 27/30/31 loosely specified — Resolved
- 🔵 **Test Coverage**: Empty and repeated `--withhold-profile` — Partially resolved (the repeated form is untested at the CLI)
- 🔵 **Architecture**: Searching profile hard-coded — Resolved
- ⚪ Deliberately not addressed: `SearchCapSource` shape, profile validation, halt on `end-run` failure, duplicated recovery text, default-cap wording, `below_frontier` parameters, property-test domain, version window, `OutstandingPair.worst_case`, renderer budgets, the sizing snippet, ADR-0072 linkage

### New Issues Introduced

- 🟡 **Usability + Documentation + Architecture + Compatibility (+ Correctness, minor)**: The depth ceiling contradicts the skill's knob policy. `research-topic/SKILL.md:88` says "There is no upper bound", and `configure/SKILL.md:445-446` says "Breadth and depth have no upper bound". A configured `research.topic.depth` above 10 passes the skill's validation and then aborts `conduct` on its first plan, with an error that names a `--depth` flag the user never passed. The repo's convention for out-of-range knob values is to clamp them with a warning.
- 🟡 **Test Coverage**: Phase 6 conflicts with existing structure tests and has no red step for them. "The first round always runs." (`SKILL.md:435`, pinned at `test_research_structure.py:756`) becomes false once halted refusals skip rounds. `test_the_summary_opens_with_the_report` no longer describes the summary's opening. The position of `{withhold flags}` must keep the pinned continue-command substring intact.
- 🔵 **Architecture + Compatibility + Correctness + Test Coverage**: `Depth::new` also decodes stamped finding depths (`research-adapters/src/topic.rs:256-262`). A finding stamped `depth: 11` would read as depth 1 and be reported as shallow.
- 🔵 **Correctness**: The two withholding persistence CLI tests are vacuous. Acknowledging batch 0 already marks every web level-1 spawn attempted, so neither test can fail before the window change.
- 🔵 **Correctness + Code Quality**: The empty tree takes an undefined path through `sized`, since `scale_step` has no `w_max`. Add a guard clause, and drop `fits`/`floor` in favour of `overruns` on `budgets_at(tree, step)`.
- 🔵 **Correctness**: `aligned_to` never changes a budget, and `scale_step`'s stated rationale holds only for the heaviest level.
- 🔵 **Architecture + Usability**: The step 2 rule "Only the first plan after a halt carries the flag" is fragile if that plan call fails. It also says "omitting changes nothing", which is false for that plan. Carry the flag until `withheld` names the profile.
- 🔵 **Standards**: The depth error says "whole number" where the rest of the CLI says "positive integer". The empty-value `--withhold-profile` message departs from the "is empty; pass …" convention.
- 🔵 **Standards + Documentation + Compatibility**: The depth-ceiling CHANGELOG entry is a standalone `### Changed` bullet for a feature that has not been released. Fold it into the `--depth` bullet (`:134`) and the `research.topic.depth` bullet (`:355`). The quota-exhausted entry belongs under `### Fixed`, beside the denied-fetch outcome.
- 🔵 **Test Coverage**: The Phase 6 floor-wording red step omits the Phase 3 CLI tests that assert the remedies.
- 🔵 **Code Quality**: `Withheld::holds_back` couples `memory.rs` to topic types, and the `RankedSpawn` bit duplicates it. Also: `stage_for` now returns a `Derivation`, and the name `RESERVE_SHARE` describes a divisor.
- 🔵 **Documentation**: The docs-site does not say that `withheld` persists in the ledger, and does not add it to the run-only fields sentence.

### Assessment

The pass 1 majors are resolved, apart from the follow-up duplication, which is now minor. The ledger-held halt and the quarantine rule hold up under adversarial checking.

Two new majors remain, so the verdict is COMMENT: the plan is acceptable but could be improved.

- **Depth ceiling**: four lenses converge on it. Settling it means one decision about who owns the ceiling. The skill should clamp with a warning, and `Depth::new` should stay unbounded for decoding stored data, with a `Depth::requested` constructor at the request boundary.
- **Phase 6 structure tests**: these are mechanical red-step additions.

Of the new minors, the vacuous withholding tests and the empty-tree guard deserve fixing before implementation, because they undermine TDD for the very behaviour the edits added.

## Re-Review (Pass 3) — 2026-10-15T23:28:27+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Usability + Documentation + Architecture + Compatibility**: Depth ceiling contradicts the skill's knob policy — Partially resolved (the clamp rule exists, but it comes after **Valid**; see below)
- 🟡 **Test Coverage**: Phase 6 conflicts with existing structure tests — Resolved
- 🔵 **Architecture + Compatibility + Correctness + Test Coverage**: `Depth::new` decodes stamped depths — Resolved (`Depth::requested`; the `depth: 12` row guards it)
- 🔵 **Correctness**: Vacuous withholding persistence tests — Resolved (traced through `Rank` and `PendingBatch` by test coverage and correctness)
- 🔵 **Correctness + Code Quality**: Empty-tree path through `sized` — Resolved
- 🔵 **Correctness**: `aligned_to` and `scale_step` — Resolved (dropped; every Phase 1 §3 row re-derived by hand without alignment)
- 🔵 **Architecture + Usability**: Fragile first-plan-after-halt rule — Resolved
- 🔵 **Standards**: Depth error wording; empty `--withhold-profile` message — Resolved
- 🔵 **Standards + Documentation + Compatibility**: CHANGELOG placement — Resolved
- 🔵 **Test Coverage**: Floor-wording red step; notice order; repeated flag; post-sizing silence — Resolved
- 🔵 **Code Quality**: `capped_follow_ups`, `Withheld` coupling, `derivation_for`, `RESERVE_ONE_IN` — Resolved
- 🔵 **Documentation**: Field-count sentence; `withheld` persistence — Resolved
- 🔵 **Usability**: Floor remedies omit restart — Resolved

### New Issues Introduced

- 🟡 **Usability (major) + Architecture, Compatibility, Standards, Documentation, Test Coverage (minor)**: The **Above the depth ceiling** rule is placed after **Valid** ("an integer of 1 or more passes unchanged"). The rules apply "in order", so depth 12 matches **Valid** first and is never clamped. `test_depth_above_the_ceiling_clamps_to_it` does not catch this. Fix: bound **Valid** for depth, or place the ceiling rule before it, and pin either choice.
- 🔵 **Test Coverage**: `a_halted_run_still_offers_non_web_researchers_and_composers` has no setup that keeps its spawns fresh. Run `--run R --withhold-profile web` without `--spawned`, or use the `--concurrency 1` pattern.
- 🔵 **Code Quality**: `a_requested_depth_beyond_ten_is_refused` also covers 0. `derivation_for` reads as returning a tuple; return the `Derivation` instead.
- 🔵 **Standards**: State `impl Depth { pub const DEEPEST: u32 = 10; }`, and build the error message with a positional `format!` argument. The `held_back` doc comment restates the code.
- 🔵 **Correctness**: The Key Discovery saying "the floor check needs no separate step" contradicts the sketch. `Depth::requested` should be described as refusing values outside 1 to `DEEPEST`. Budgets probed for huge caps should convert with `u32::try_from(..).unwrap_or(u32::MAX)`.
- 🔵 **Documentation**: The run-only fields sentence should also name `search_budgets` and `search_demand`.
- 🔵 **Usability**: The floor warning says "lower the depth" and configure says "lower `--depth`"; neither names `research.topic.depth`.
- 🔵 **Architecture**: The ten-level bound on the worst-case tree is a convention, not a type. Say that planning only sees `Depth::requested` values.

### Assessment

Every finding from passes 1 and 2 is resolved or deliberately deferred. One new issue is rated major by one lens and minor by five, which gives a COMMENT verdict. That issue is a one-line wording fix in the knob rules, plus a structure-test assertion. The rest are wording and sketch clarifications. The plan is ready to implement once the **Valid** rule is bounded.

## Final Verdict — 2026-10-15T23:33:02+00:00

**Verdict:** APPROVE

The pass 3 findings were applied without changing the design:

- **Valid** is bounded for depth;
- `Depth::requested` is sketched;
- the composer-exemption test has a setup that keeps its spawns fresh;
- the sketch and docs clarifications are made.

The reviewer approved the plan for implementation.
