---
type: "plan"
id: "2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion"
title: "Budget Web Searches and Halt on Exhaustion Implementation Plan"
date: "2026-10-15T22:00:14+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:0304"
parent: "work-item:0304"
derived_from: ["codebase-research:2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion"]
relates_to: ["plan:2026-10-10-0303-research-concurrency-subagent-limit", "plan:2026-10-13-0310-recovering-from-an-interrupted-conduct-run"]
tags: ["research", "deep-research", "conduct", "web-profile", "research-cli", "run-ledger"]
revision: "d1b70ff310f18e9f8f1a457e8af73d81929a08e3"
repository: "accelerator"
last_updated: "2026-10-16T16:13:17+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Budget Web Searches and Halt on Exhaustion Implementation Plan

## Overview

`research topic outstanding` sizes a per-level `WebSearch` budget for every
web researcher from Claude Code's session search cap. It stores the budgets
in the run ledger and hands each web spawn its node's budget. `conduct`
fails a web node whose search returns the session-limit notice. It then
halts web researcher spawns for the rest of the run, finishes the run's
other work, and ends with a halt notice naming the recovery. Work item:
`meta/work/0304-fail-web-nodes-on-websearch-quota-exhaustion.md`.

## Current State Analysis

- The web profile has no `WebSearch` cap and no outcome for an exhausted
  quota (`skills/research/profiles/web-profile/SKILL.md:57-69`). At the cap,
  Claude Code returns a notice in place of results ("Web search was not
  performed: this session's web search budget is used up … Continue with the
  information already gathered …" in 2.1.301). The researcher follows that
  advice and writes notes from `WebFetch` or memory.
- The CLI reads one harness variable, `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`,
  in `cli/research-cli/src/subagent_cap.rs`. ADR-0071 admits it to the
  coupling guard (`tasks/lint/claude_coupling.py:44-49`), and its parse lives
  in the private `harness_slots` (`subagent_cap.rs:85-99`).
- The run ledger (`cli/research/src/conduct/ledger.rs:44-49`, codec in
  `cli/research-adapters/src/conduct/ledger.rs:103-171`) stores no budget.
  `owed_quarantine` is the precedent for an optional field that defaults
  when absent (`research-adapters/.../ledger.rs:166-168`).
- `TreeWalk::walk` (`cli/research/src/topic/tree.rs:161-184`) stops at the
  first level with a missing node, so nothing counts the nodes below the
  frontier.
- The rendered `pairs` come from the concurrency window
  (`cli/research-cli/src/topic_command.rs:502-507`). The full outstanding set
  exists only as `plan.pairs`.
- The window offers every unattempted spawn in rank order
  (`cli/research/src/conduct/window.rs:40-75`). Acknowledging a batch marks
  every offered spawn attempted (`ledger.rs:115-136`), so the skill alone
  cannot hold back a spawn without it turning up later as "wrote no note".
- `run_outstanding` already takes seven parameters
  (`topic_command.rs:243-251`), clippy's `too_many_arguments` threshold.

## Desired End State

- `outstanding --start` (or the first `--run` over a ledger without budgets)
  sizes per-level budgets over the worst-case tree of every outstanding web
  pair. It stores them in the ledger and reports `search_budgets`,
  `search_demand` and `search_cap`. When even budgets of 1 do not fit
  `cap − reserve`, it emits the floor warning, once per run.
- Every web `single_pass` pair and web node in a run plan carries
  `search_budget`. The web profile caps `WebSearch` calls at it. Each web
  level note records it as `search_budget`, which the schema admits as an
  optional extra.
- A web researcher that gets the session-limit notice writes nothing and
  replies "web search quota exhausted".
- `conduct` defines halting failures in one place. A quota-exhausted spawn
  halts the run: any file it wrote anyway is quarantined, and the next plan
  carries `--withhold-profile web`. The ledger records the withheld profile
  for the rest of the run, so the CLI offers no web researcher spawn while
  it still offers non-web researchers and composers, and run plans report
  `withheld`. Capacity-refused web spawns are not retried. Step 7 runs in
  full, and the summary opens with the halt notice, which names the web pairs
  left unfinished from the tick plan's new `unanswered` field.
- A requested depth is 1 to 10. `conduct` clamps a larger configured or
  flagged depth to 10 with a warning, and `outstanding --depth` refuses
  one. So the worst-case tree never holds more than ten levels. Findings
  keep whatever depth they were stamped with.

Verify with `mise run` (the bare default task) exiting 0, plus the manual
halt run in **Manual Testing Steps**.

### Key Discoveries

- ✅ **Parser parity.** Claude Code 2.1.301 declares both getters
  identically: `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS:()=>ci`,
  `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION:()=>Ri`, with
  `ci=M.int({min:1,digitsOnly:!0})` and `Ri=M.int({min:1,digitsOnly:!0})`,
  each read as `a.VAR ?? default` (20 and 200). So the search cap reuses
  `harness_slots` and its 0303 case tables unchanged. This settles the work
  item's parse-edge assumption.
- ✅ **Sizing algorithm.** An integer version reproduces all 16 numeric
  acceptance criteria, including the re-run (35/17) and the would-be resize
  (9/5), in a scratch model. Scale `k` by 4 into `j`, so every budget is
  `max(1, ⌊j·w/4⌋)`. Binary-search the largest `j` that fits, then top up.
  The top-up loop ran at most 3 iterations across the criteria. The model
  also rounded `j` down to a multiple of `4 / w_max`. That rounding never
  changes a budget, because every budget already floors per level, so the
  plan drops it. `j = 1` gives every level 1, so it is the floor. The
  floor is checked first, and when it fits it becomes the search's lower
  bound.
- **Weights are fixed, not derived.** Weights are 4/2/1 by requirement. They
  coincide with `follow_up_cap` today but must not move if the starting
  follow-up cap does, so they get their own `level_weight`.
- **The worst-case count must not push trims.** Calling `follow_ups()` at the
  frontier would record trims for frontier-level notes earlier than today,
  changing `trims` output. The count is a separate pure walk over a cloned
  `known` set.
- **The halt must be CLI-side.** A withheld spawn must never be offered.
  Otherwise acknowledgement makes it attempted and step 6 records it as
  "wrote no note", and it takes window slots ahead of deeper non-web nodes
  (`window.rs:78-84` ranks by level before profile).
- **The halt is run-wide state, so the ledger keeps it.** It is kept for
  the same reason as the budgets. A halt the orchestrator had to restate on
  every plan would be lost to one omitted flag or to context compaction,
  and the withheld spawns would come straight back. `RunMemory`, which
  `window` already reads, gains the withheld profiles beside `attempted`.
- **`--depth` needs a ceiling.** Today `walk` stops at the first level with
  missing nodes, so a huge `--depth` costs nothing. The worst-case tree
  holds every level down to the depth, so `--depth 4000000000` would hang
  even a query plan. A compact tail does not rescue it: top-up breaks the
  tail's uniformity (AC 6's `11/5/3/3/2`), so every layer would need a
  run-length shape. From level 3 down, each node has one follow-up, so
  research never needs anywhere near ten levels.
- **The ceiling bounds requests, not records.** `Depth::new` also reads a
  finding's stamped `depth` (`research-adapters/src/topic.rs:256-262`). A
  bounded `Depth::new` would read a finding stamped `depth: 12` as depth 1
  and report it `shallow`. So `Depth::new` stays unbounded, and a new
  `Depth::requested` bounds only the requested depth. The ten-level bound
  on the worst-case tree therefore depends on planning only ever seeing
  depths from `Depth::requested`. Today that means `OutstandingFlags::parse`.
  Any new production path into planning must build its depth the same
  way.
- **The skill owns the ceiling for configured depth.** The knob rules clamp
  an out-of-range value with a warning. They do not refuse it
  (`research-topic/SKILL.md:74-89`). A depth above 10 clamps to 10 in the
  same way, so `conduct` never passes the CLI a depth it refuses. The CLI
  refusal stays as the backstop for hand runs.
- **A shared template must carry every optional extra.**
  `template_shape.rs:368-381` (`check_extras`) requires every optional extra
  in the template. So `search_budget` goes into the shared level-note
  template, and the outputter tells non-web researchers to drop it.

## What We're NOT Doing

- An `ACCELERATOR_*` override for the search cap (Drafting Notes defer it).
- Recording searches made. `search_budget` is an allowance.
- Sizing budgets in query mode. Query plans render `search_cap` only, and
  `conduct` never spawns from them.
- Resizing budgets mid-run, or storing depth. A level with no stored budget
  (only reachable by a later plan passing a larger `--depth`) gets 1.
- Discounting level notes written below the frontier by an earlier, deeper
  run. The worst-case count treats every declared follow-up at the frontier's
  next level as unresearched. That over-counts conservatively and never
  under-counts.
- Recovery from agent usage limits, which stays 0310's. The "unfinished run"
  table row and the step 2 "run … did not end" report are unchanged.
- 0307 and 0309 behaviour. Both are `draft` today. Phases 4 and 5 carry a
  conditional step for each in case it lands first.
- Profile-declared search capability. `web` is the only profile that calls
  `WebSearch`, so `SEARCHING_PROFILE` names it. A second searching profile
  would move this to profile metadata.

## Implementation Approach

Six phases, each mergeable alone and each leaving `mise run` green:

```mermaid
flowchart LR
  P1[1 Worst-case tree and sizing] -->|SearchCap, unsearched_tree| P2[2 Search cap from environment]
  P2 -->|EnvironmentSearchCap| P3[3 Budgets in ledger and plan]
  P3 --> P4[4 Budget delivery]
  P4 --> P5[5 Quota exhausted outcome]
  P5 --> P6[6 Halting]
```

The phases form a chain: phase 2 uses phase 1's `SearchCap` and
`unsearched_tree`. Phases 1–3 add domain code and plan fields the skill
ignores until phase 4 names them. Phase 5 alone already stops the silent
fallback, at the cost of every later web spawn failing one by one. Phase 6
turns that into a clean halt. Every phase follows red-green-refactor: the
test tables come first, and each must fail for the stated reason before
the production change.

Each phase states only what is true once it merges. The floor warning and
the configure text promise a halt only from phase 6. Each phase that
changes behaviour extends `CHANGELOG.md`'s `## [Unreleased]`, under the
`### Added` bullet **`research topic outstanding` plans deepened
research** unless a phase names another bullet or section.

Crate placement follows the existing layering:

- **Domain** (`cli/research`): sizing, the worst-case tree, `SearchCap` and
  the window's withholding, all pure.
- **Adapters** (`research-adapters`): the ledger codec.
- **Composition root** (`research-cli`): environment reads, wording and JSON.

---

## Phase 1: Worst-Case Tree and Budget Sizing

### Overview

A depth ceiling, then two pure domain capabilities with no caller yet:

- each outstanding pair's worst-case tree of unresearched nodes per level;
- per-level budgets sized from a tree and a search cap.

### Changes Required

#### 0. Depth ceiling (red, then green)

**File**: `cli/research/src/topic/tree.rs`

Tests first:

- `a_requested_depth_of_one_to_ten_is_accepted` (1, 10)
- `a_requested_depth_outside_one_to_ten_is_refused` (0, 11, `u32::MAX`)
- `a_recorded_depth_has_no_ceiling`: `Depth::new(12)` is `Some`

Then add:

```rust
impl Depth {
    pub const DEEPEST: u32 = 10;

    #[must_use]
    pub const fn requested(levels: u32) -> Option<Self> {
        if levels > Self::DEEPEST { None } else { Self::new(levels) }
    }
}
```

`Depth::new` is unchanged, so `requested` refuses anything outside 1 to
`DEEPEST`.

**File**: `cli/research-adapters/src/topic.rs`

In `a_findings_depth_is_read_from_its_frontmatter` (`:830`), add the row
`("depth: 12\n", false)`: the finding is not shallow at the table's depth
of 3. The row passes before and after the change. It pins that the ceiling
never touches recorded depth, and it would fail if `stamped_depth` used
`Depth::requested`.

**File**: `cli/research-cli/src/topic_command.rs`,
`cli/research-cli/tests/topic_outstanding.rs`,
`cli/research-cli/src/cli.rs`

Update the expectation at `topic_outstanding.rs:819` first, and add a case
for `--depth 11`. Then `OutstandingFlags::parse` builds the depth with
`Depth::requested`, which also refuses 0, so the `positive` helper goes
with its last caller. The message comes from `format!("E_TOPIC_RESEARCH_DEPTH:
--depth must be 1 to {}, got '{depth}'", Depth::DEEPEST)`, which follows
the `--run must be 1 to 64` precedent. The `--depth` help (`cli.rs:99-101`)
names the range.

**File**: `tests/unit/tasks/test_research_structure.py` (red), then
`skills/research/research-topic/SKILL.md` and
`skills/config/configure/SKILL.md`

- `test_depth_above_the_ceiling_clamps_to_it`: **Valid** reads "an integer
  of 1 or more, and for depth at most 10, passes unchanged", because the
  rules apply in order. A later rule carries the clamp for depth above 10,
  with the warning `Warning: research.topic.depth must be at most 10, got
  '{value}' — clamping to 10`. That warning has the same shape as the
  clamp-to-1 warning.
- `test_the_depth_ceiling_reads_alike_in_prose_and_in_the_cli`: the 10 in
  those rules matches `pub const DEEPEST: u32 = 10;` in `tree.rs`, read
  with `_rust_text`.
- `test_configure_documents_the_depth_ceiling`: configure says depth above
  10 clamps to 10 with a warning, and that only breadth has no upper bound.

In `research-topic/SKILL.md`, **Valid** (`:81`) becomes "an integer of 1
or more, and for depth at most 10, passes unchanged". Then add a rule
after **Out of range or malformed**. **Above the depth ceiling**: a depth
above 10, whether from the flag or from config, clamps to 10, and the verb
warns as above. Replace
"There is no upper bound; breadth is the per-round cost guard" (`:88`) with
"Breadth has no upper bound; it is the per-round cost guard". In
`configure/SKILL.md:445-446`, replace "Breadth and depth have no upper
bound" with "Depth above 10 is clamped to 10 with a warning. Breadth has no
upper bound". In the `depth` table row (`:437`), add "(1–10)".

**Docs and changelog**:

- In the `docs-site/src/content/docs/research.md` exit-code table (`:545`),
  the `E_TOPIC_RESEARCH_DEPTH` row reads "`--depth` is not 1 to 10", with
  the recovery "pass 1 to 10". The `--depth N` paragraph (`:474-476`) names
  the range.
- In `CHANGELOG.md`, add "(1 to 10)" after `--depth N` in the
  plans-deepened-research bullet (`:134`). Extend the
  `research.topic.depth` bullet (`:355`) with "A configured depth above 10
  clamps to 10 with a warning." `--depth` is itself unreleased, so there is
  no separate entry.

#### 1. Worst-case tree tests (red)

**File**: `cli/research/src/topic/tree.rs` (inline tests, using the
existing `note()`, `directory()`, `derived_from()` helpers)

`derive` now returns a `Derivation`. Existing tests read `.stage` and
`.trims` in place of the tuple fields.

| Test | Setup | Expected `worst_case` per level |
|---|---|---|
| `a_single_pass_pair_is_one_unresearched_root` | no directory, depth 1 | `{1: 1}` |
| `a_pair_with_no_notes_counts_every_capped_descendant` | no directory, depth 3 | `{1: 1, 2: 4, 3: 8}` |
| `a_depth_beyond_three_continues_at_one_follow_up_per_node` | no directory, depth 5 | `{1: 1, 2: 4, 3: 8, 4: 8, 5: 8}` |
| `written_notes_contribute_their_declared_follow_ups` | AC 15: `1` with 4 follow-ups, `2-1` declaring 1, `2-2` declaring 2, depth 3 | `{2: 2, 3: 7}` |
| `declared_follow_ups_are_capped_and_deduplicated_as_derivation_would` | `1` with `B`,`C`,`D`,`E`; `2-1` declaring `X`,`X`,`Y`; `2-2` declaring `B`; depth 3 | `{2: 2, 3: 5}` |
| `a_frontier_at_the_depth_limit_counts_only_itself` | `1` with `B`,`C`; `2-1` written; depth 2 | `{2: 1}` |
| `a_composable_pair_has_no_unresearched_nodes` | complete tree at depth 2 | empty |
| `counting_the_frontier_records_no_trims_derivation_would_not` | `1` with `B`,`C`; `2-1` declaring 3 follow-ups (cap 2); `2-2` missing; depth 3 | `trims` empty, `worst_case` `{2: 1, 3: 4}` |
| `worst_case_trees_join_level_by_level` | `{1: 1, 2: 4}` joined with `{2: 2, 3: 7}` | `{1: 1, 2: 6, 3: 7}`; level 4 holds 0 |
| `joining_saturates_rather_than_overflowing` | `{1: u64::MAX}` joined with itself | `{1: u64::MAX}` |

The `counting_the_frontier…` row pins the trims caveat from Key Discoveries.

#### 2. Worst-case tree (green)

**File**: `cli/research/src/topic/tree.rs`

```rust
/// An upper bound, not a forecast: every node not yet written is assumed
/// to declare its level's full follow-up cap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorstCaseTree {
    nodes: BTreeMap<u32, u64>,
}

impl WorstCaseTree {
    #[must_use]
    pub fn root() -> Self { /* {1: 1} */ }

    fn below_frontier(
        level: u32,
        missing: u64,
        declared: u64,
        depth: Depth,
    ) -> Self { /* level: missing; level+1: declared + missing·cap(level);
                   each deeper: previous·cap(previous) */ }

    #[must_use]
    pub fn nodes_at(&self, level: u32) -> u64 { /* 0 when absent */ }

    pub fn populated_levels(&self) -> impl Iterator<Item = (u32, u64)> + '_ { … }

    #[must_use]
    pub fn is_empty(&self) -> bool { … }

    #[must_use]
    pub fn joined(mut self, other: &Self) -> Self { /* saturating per-level sum */ }
}

pub struct Derivation {
    pub stage: Stage,
    pub trims: Vec<Trim>,
    pub worst_case: WorstCaseTree,
}
```

- `derive` returns `Derivation`. `Stage::SinglePass` gets
  `WorstCaseTree::root()`, and `Stage::Compose` an empty tree.
- Refactor first, under the existing tree tests. Extract the cap from
  `follow_ups()` (`tree.rs:219-254`) into one helper,
  `capped_follow_ups(note, level)`. It yields `(position, follow_up)` for a
  note's first `follow_up_cap(level)` follow-ups. `follow_ups()` drives its
  `recorded` push, its freshness check and its `questions` insert from that
  helper, so `take(cap)` appears once. Every existing tree test stays
  green.
- In `walk`, on the first level `L` with missing nodes, `declared` counts
  the follow-ups from `capped_follow_ups` over the frontier's notes whose
  normalised question inserts into a clone of `known`. When `L == depth`,
  `declared` is 0. The clone is the only state the count touches, so it
  records neither questions nor trims. It applies the same cap and the same
  deduplication as derivation.
- All arithmetic saturates (`u64::saturating_mul`/`saturating_add`).

**File**: `cli/research/src/topic/plan.rs`

`OutstandingPair` gains `pub worst_case: WorstCaseTree`. `Planner::stage_for`
(`plan.rs:264-279`) becomes `derivation_for`. It returns the `Derivation`,
and the planner destructures it into the pair's stage and worst case and
the plan's trims. Update the `OutstandingPair {` literals in the
`conduct/ledger.rs`, `conduct/window.rs` and `topic/plan.rs` tests, using
`WorstCaseTree::default()` where the test does not care.

#### 3. Sizing tests (red)

**File**: `cli/research/tests/search_budget.rs` (new, black-box over the
public API, following `cli/research/tests/concurrency.rs`)

Helpers: `tree(&[(level, nodes)])` builds a `WorstCaseTree` through the
public `WorstCaseTree::from_levels` test seam (see below), and `cap(n)`
builds a `SearchCap`.

| Test | Cases (cap; tree) | Expected budgets / demand / overruns |
|---|---|---|
| `the_reserve_is_a_twentieth_of_the_cap_rounded_up` | 200, 192, 201, 60, 1 | reserves 10, 10, 11, 3, 1; available 190, 182, 190, 57, 0 |
| `budgets_fill_the_cap_less_its_reserve` | the 13 non-floor rows of the research doc's table: 200 `10/40/80`; 200 `3/12/24`; 500 `10/40/80`; 500 `10/40/80/80/80`; 200 `10/40`; 200 `2/8/16/16/16`; 192 `14/56/112`; 200 `8/32/64`; 200 `1/4/8`; 201 `1/4/8`; 60 `1/4/8`; 200 `10`; 200 `–/2/7` | 3/2/1·190; 15/6/3·189; 11/5/2·470; 7/4/1/1/1·470; 7/3·190; 11/5/3/3/2·190; 1/1/1·182; 7/2/1·184; 42/19/9·190; 42/19/9·190; 13/5/3·57; 19·190; –/35/17·189; none overrun |
| `budgets_of_one_that_overrun_stay_at_one` | 200 `20/80/160`; 200 `15/60/120` | 1/1/1·260 overruns; 1/1/1·195 overruns |
| `a_resized_remainder_would_grant_more` | 200 `–/10/20` | –/9/5·190 |
| `a_level_without_remaining_nodes_takes_no_budget` | 200 `–/2/7` | `levels()` yields 2 and 3 only |
| `an_empty_tree_sizes_no_budgets_and_no_demand` | 200; empty | no levels, demand 0, does not overrun |
| `a_level_without_a_budget_is_budgeted_one` | 200 `–/2/7`, `at(1)`, `at(4)` | 1, 1 |
| `sized_budgets_are_maximal_shaped_and_within_the_cap` | every tree with 1–4 levels, each level 0–3 shallow nodes × caps-shaped growth, caps 1–260 (enumerated, not random) | demand ≤ available unless every budget is 1; every budget ≥ 1; non-increasing with depth over levels with nodes; no level can rise by 1 without breaching one of those (AC 7) |
| `a_huge_cap_sizes_without_overflow` | `u32::MAX`; `1/4/8` | finishes; demand ≤ available; no level can rise by 1 without breaching the cap or the shape |

#### 4. Sizing (green)

**File**: `cli/research/src/conduct/search_budget.rs` (new, declared in
`cli/research/src/conduct.rs`)

```rust
//! How many `WebSearch` calls each web researcher may make: per-level
//! budgets sized from the session's search cap over the worst-case tree of
//! web nodes a run may still spawn.

pub const SEARCHING_PROFILE: &str = "web";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchCap(NonZeroU32);

impl SearchCap {
    #[must_use]
    pub const fn new(searches: NonZeroU32) -> Self { … }
    #[must_use]
    pub const fn searches(self) -> NonZeroU32 { … }
    /// Kept back for searches outside the run: the main conversation,
    /// `outline`, and whatever the session spent before `conduct` began.
    #[must_use]
    pub const fn reserve(self) -> u32 { self.0.get().div_ceil(RESERVE_ONE_IN) }
    #[must_use]
    pub const fn available(self) -> u32 { self.0.get() - self.reserve() }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchBudgets {
    budgets: BTreeMap<u32, NonZeroU32>,
    demand: u64,
}

impl SearchBudgets {
    #[must_use]
    pub fn sized(tree: &WorstCaseTree, cap: SearchCap) -> Self { … }
    #[must_use]
    pub fn from_parts(budgets: BTreeMap<u32, NonZeroU32>, demand: u64) -> Self { … }
    #[must_use]
    pub fn at(&self, level: u32) -> NonZeroU32 { /* NonZeroU32::MIN when absent */ }
    pub fn levels(&self) -> impl Iterator<Item = (u32, NonZeroU32)> + '_ { … }
    #[must_use]
    pub const fn demand(&self) -> u64 { … }
    /// Only budgets of 1 at every level can overrun: sizing otherwise
    /// shrinks them to fit.
    #[must_use]
    pub fn overruns(&self, cap: SearchCap) -> bool { self.demand > u64::from(cap.available()) }
}

#[must_use]
pub fn unsearched_tree(pairs: &[OutstandingPair]) -> WorstCaseTree { … }

const WEIGHT_SCALE: u64 = level_weight(1);
const RESERVE_ONE_IN: u32 = 20;

const fn level_weight(level: u32) -> u64 {
    match level {
        1 => 4,
        2 => 2,
        _ => 1,
    }
}
```

`reserve` is the cap divided by `RESERVE_ONE_IN`, rounded up. `sized` is a
pipeline of named private steps over `scale`, a budget multiplier in units
of `1 / WEIGHT_SCALE`:

```rust
pub fn sized(tree: &WorstCaseTree, cap: SearchCap) -> Self {
    if tree.is_empty() {
        return Self::from_parts(BTreeMap::new(), 0);
    }
    let floor = budgets_at(tree, 1);
    if floor.overruns(cap) {
        return floor;
    }
    topped_up(tree, budgets_at(tree, largest_fitting_scale(tree, cap)), cap)
}
```

- The empty tree returns first, so every later step may assume at least
  one populated level.
- `budgets_at(tree, scale)`: `max(1, ⌊scale·w/WEIGHT_SCALE⌋)` for each
  populated level. Budgets are computed in `u64`. Demand is Σ nodes ×
  budget, and saturates. Each budget is stored with
  `u32::try_from(..).unwrap_or(u32::MAX)`, so huge caps stay monotone in
  the scale rather than truncating. At scale 1,
  every level gets 1, because no weight exceeds `WEIGHT_SCALE`. That is the
  floor, and when it overruns it is returned as it is.
- `largest_fitting_scale`: binary search over
  `[1, WEIGHT_SCALE·available + WEIGHT_SCALE]`. Demand is monotone in the
  scale. At the upper end, the heaviest level's budget alone exceeds
  `available`, so the upper end never fits.
- `topped_up`: loops over populated levels, deepest first. It raises the
  first level whose `demand + nodes ≤ available` and whose budget + 1 is no
  more than the next shallower level's budget; the shallowest level has no
  bound. It stops when no level rises.

`WorstCaseTree::from_levels(&[(u32, u64)]) -> Self` is public for the
black-box tests and for `unsearched_tree`'s own unit tests.

`unsearched_tree` folds `joined` over pairs whose `pair.profile ==
SEARCHING_PROFILE`. Inline unit tests check that web pairs sum (AC 11's
8 web + 2 openalex → `8/32/64`) and that non-web pairs add nothing.

#### 5. Public API pin

Run `mise run public-api:update` and commit the regenerated
`cli/research/tests/fixtures/public-api.txt`.

### Success Criteria

#### Automated Verification

- [x] Depth, tree and sizing tests fail first for the stated reasons, then
      pass: `cargo test -p research -p research-adapters -p research-cli`
      (from `cli/`)
- [x] Depth-ceiling structure tests fail first, then pass:
      `uv run pytest tests/unit/tasks/test_research_structure.py`
- [x] Whole Rust workspace checks pass: `mise run cli:check`
- [x] Public API pin matches: `mise run public-api:check`
- [x] Full local CI mirror: `mise run`

#### Manual Verification

- [x] `accelerator research topic outstanding <slug> --profiles-dir …
      --depth 11` exits 1 with the new `E_TOPIC_RESEARCH_DEPTH` message.
- [ ] `conduct <slug> --depth 12` warns that it is clamping to 10, and its
      first plan carries `--depth 10`. Stop the run after the first plan.

---

## Phase 2: Search Cap from the Environment

### Overview

`outstanding` resolves the search cap, renders it on every plan, and
reports an unhonoured value on `--start` when the run has web nodes to
research. The coupling guard and a new ADR admit the variable.

### Changes Required

#### 1. ADR (first, so the admission is decided before it is coded)

Create ADR-0073 with `/accelerator:create-adr`, then accept it with
`/accelerator:review-adr`. Title: "Admit the session search cap as an
optional harness variable". It takes the second of three options: follow
ADR-0071 exactly, the harness variable alone with an unhonoured value
reported, or a fixed cap. It extends ADR-0071 and does not supersede it:

- it admits `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`, read in
  `cli/research-cli/src/search_cap.rs`;
- it records two deliberate departures from ADR-0071's pattern. There is no
  `ACCELERATOR_*` counterpart: an override set above the harness's cap
  costs far more than for the subagent cap, since capacity-refused spawns
  are retried in rounds but searches budgeted beyond the quota halt the
  run until `/clear`. The limit is also documented only for interactive
  terminal sessions, so an opt-out is deferred until `conduct` runs
  off-terminal. An unhonoured harness value is reported when the run has
  web nodes left to research, because budgets sized against a silently
  ignored cap mislead the user;
- it records the parser-parity evidence from Key Discoveries, against
  2.1.301.

#### 2. Shared harness parse (refactor under existing tests)

**File**: `cli/research-cli/src/harness_setting.rs` (new; declared in
`main.rs`)

Move `is_set`, `harness_slots` (renamed `positive_whole_number`) and
`is_javascript_whitespace` out of `subagent_cap.rs` unchanged. Move the
three parse tests (`a_cap_is_parsed_as_the_harness_parses_it`,
`a_cap_the_harness_ignores_is_the_default`,
`a_cap_too_large_for_u32_saturates`) with them, renamed
`a_setting_is_parsed_as_the_harness_parses_it`,
`a_setting_the_harness_ignores_is_none` and
`a_setting_too_large_for_u32_saturates`. Each is rewritten to assert on
`positive_whole_number` directly: `Some(n)`, `None`, and
`Some(NonZeroU32::MAX)`. `subagent_cap.rs` imports both functions, and its
remaining tests still pass untouched. The doc comment explaining the
JavaScript parity ("read as the harness's JavaScript parser reads its own
…", `subagent_cap.rs:51-53`) moves onto `positive_whole_number`, the
function it describes. Neither `from_environment` keeps it.

#### 3. Search cap tests (red)

**File**: `cli/research-cli/src/search_cap.rs` (inline tests over a
`HashMap` lookup, as `subagent_cap.rs:118-123`)

| Test | Cases | Expected |
|---|---|---|
| `an_unset_search_cap_is_the_harness_default_of_200` | no variable | 200, `SearchCapSource::HarnessDefault`, no unhonoured |
| `a_search_cap_is_read_as_the_harness_reads_it` | `"500"`, `"+60"`, `" 192 "`, `"0201"` | 500, 60, 192, 201; source `Variable` |
| `a_search_cap_the_harness_ignores_is_the_default_and_reported` | `"abc"`, `"0"`, `"-5"`, `"1.5"`, `"1e2"` | 200, `HarnessDefault`, unhonoured holds the raw value |
| `a_blank_search_cap_is_unset_and_unreported` | `""`, `"  "` | 200, no unhonoured |
| `the_default_search_cap_matches_claude_codes` | — | `HARNESS_DEFAULT.searches() == 200` |
| `the_source_names_the_variable_that_raises_the_cap` | — | `raised_by()` and `Variable.reported_as()` name the variable; `HarnessDefault.reported_as()` is `"default"` |

#### 4. Search cap (green)

**File**: `cli/research-cli/src/search_cap.rs` (new)

```rust
//! The session's `WebSearch` cap, read from the environment `conduct` runs
//! in.

const HARNESS_SEARCH_CAP: &str = "CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION";

pub const HARNESS_DEFAULT: SearchCap = SearchCap::new(match NonZeroU32::new(200) {
    Some(searches) => searches,
    None => NonZeroU32::MIN,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchCapSource {
    Variable,
    HarnessDefault,
}

impl SearchCapSource {
    pub const fn raised_by() -> &'static str { HARNESS_SEARCH_CAP }
    pub const fn reported_as(self) -> &'static str {
        match self {
            Self::Variable => HARNESS_SEARCH_CAP,
            Self::HarnessDefault => "default",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentSearchCap {
    pub cap: SearchCap,
    pub source: SearchCapSource,
    pub unhonoured: Option<String>,
}

pub fn from_environment(lookup: impl Fn(&str) -> Option<String>) -> EnvironmentSearchCap { … }
```

#### 5. Notice wording tests and wording

**File**: `cli/research-cli/src/search_wording.rs` (new)

Tests build the expected text with `SearchCapSource::raised_by()`, so the
variable name appears only in the admitted reader:

```rust
pub fn unhonoured_search_cap(value: &str, cap: SearchCap) -> String {
    format!(
        "{} is '{value}', not a positive integer — ignoring it; sizing search \
         budgets against the search cap of {} from the default",
        SearchCapSource::raised_by(),
        cap.searches(),
    )
}
```

Expected for `abc`: `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION is 'abc', not a
positive integer — ignoring it; sizing search budgets against the search cap
of 200 from the default`.

#### 6. Plan request and rendering

**File**: `cli/research-cli/src/topic_command.rs`, `cli/research-cli/src/main.rs`

- Introduce `pub struct PlanRequest { pub depth: Depth, pub concurrency:
  CappedConcurrency, pub search_cap: EnvironmentSearchCap, pub mode:
  RunMode }`. `run_outstanding(set_root, profiles_dir, &request, fs,
  store)` replaces the separate parameters, and reads `request.*` in
  place. `main.rs::outstanding` builds it with
  `search_cap::from_environment(|name| std::env::var(name).ok())`.
- When the mode is `RunMode::Start`, after the plan is drawn,
  `unhonoured_search_cap_notice` pushes
  `search_wording::unhonoured_search_cap` when `request.search_cap.unhonoured`
  is set **and** `!unsearched_tree(&plan.pairs).is_empty()`. The concurrency
  notices and the corrupt-ledger notice stay where they are, before the plan
  is drawn (`topic_command.rs:262-269`). A `--start` that fails to draw its
  plan therefore still prints them. The full order in stderr and `warnings`
  is: concurrency notices, corrupt-ledger notice, unhonoured search cap, and
  then, from phase 3, the floor warning.
- `PlanOutput` holds `request: &PlanRequest` in place of its `depth`,
  `concurrency` and `cap` fields. `render` adds, unconditionally, beside
  `subagent_cap`:
  `"search_cap": {"searches": n, "source": source.reported_as()}`.
- Clippy's 100-line `too_many_lines` limit holds both functions. So
  `run_outstanding` hands the ledger write to `stored` and the stderr
  lines to `printed_warnings`. `render` hands its `unfinished`, `trims`
  and `shallow` entries to `unfinished_spawn`, `trim` and
  `shallow_finding`. Phases 3 and 6 add to both functions within that
  limit.

#### 7. CLI black-box tests

**File**: `cli/research-cli/tests/topic_outstanding.rs`

- Add `const SEARCH_CAP: &str = "CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION";`.
  `command_without_an_inherited_cap()` also calls `.env_remove(SEARCH_CAP)`.
- Extend the whole-plan equality fixtures (`:217-289`, near `:789`) with
  `"search_cap": {"searches": 200, "source": "default"}`.

| Test | Setup | Expected |
|---|---|---|
| `every_plan_reports_the_search_cap_and_its_source` | query plan; then `SEARCH_CAP=500` | `{200, "default"}`; `{500, SEARCH_CAP}` |
| `an_unhonoured_search_cap_is_reported_when_a_run_starts_with_web_pairs` | 1 web pair, `SEARCH_CAP=abc`, `--start` | `warnings` and stderr hold the notice naming `abc` and 200 from the default |
| `a_zero_search_cap_is_unhonoured` | 1 web pair, `SEARCH_CAP=0`, `--start` | notice naming `'0'` |
| `an_unhonoured_search_cap_is_not_reported_after_the_start` | as above, then `--run R --spawned 0` | no such notice |
| `a_run_without_web_pairs_reports_no_search_cap_notice` | openalex and arxiv pairs only, `SEARCH_CAP=abc`, `--start` | no notice |
| `start_notices_keep_their_order` | 1 web pair, `--concurrency` above an `ACCELERATOR_SUBAGENT_CAP`, a corrupt ledger on disk, `SEARCH_CAP=abc`, `--start` | `warnings` is exactly: clamp notice, corrupt-ledger notice, search-cap notice |

#### 8. Coupling guard

**Files**: `tasks/lint/claude_coupling.py`, `tests/unit/tasks/test_claude_coupling.py`

- Add `"CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION": ("cli/research-cli/src/search_cap.rs", "cli/research-cli/tests/topic_outstanding.rs")`
  to `_OPTIONAL_HARNESS_VARIABLES`.
- Rename `test_the_admitted_set_is_exactly_the_subagent_cap` to
  `test_the_admitted_set_is_exactly_the_two_harness_caps` and assert both
  entries. `_ADMITTED` is built from `_OPTIONAL_HARNESS_VARIABLES`, so the
  missing-file test covers `search_cap.rs` unchanged.
- Add `test_permits_the_search_cap_in_its_reader_and_its_test` and
  `test_flags_the_search_cap_in_the_subagent_cap_reader`, so admission is
  by name and file together.

#### 9. Docs

- `docs-site/src/content/docs/research.md` (`outstanding` reference): rewrite
  the opening summary at `:464` ("four arrays, two counts and the cap") so
  that it no longer counts fields, for example "It prints JSON whose fields
  are listed below". Later phases then only add to the list. Add `search_cap` to the field list
  (`:464-471`). After the subagent-cap paragraph (`:493-505`), add a paragraph
  covering three points:
  - the cap is read from `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` as Claude
    Code reads it, else 200;
  - a set value that Claude Code would not honour is ignored, with a warning
    on `--start` when the run has web pairs;
  - there is no `ACCELERATOR_*` counterpart.
- `CHANGELOG.md`: extend the plans-deepened-research bullet with the
  `search_cap` field and its notice.

### Success Criteria

#### Automated Verification

- [x] New tests fail first, then pass: `cargo test -p research-cli` (from
      `cli/`)
- [x] Coupling guard and its tests pass: `mise run build-system:check` and
      `uv run pytest tests/unit/tasks/test_claude_coupling.py`
- [x] ADR validates: `accelerator corpus frontmatter validate --file meta/decisions/ADR-0073-*.md`
- [x] Full local CI mirror: `mise run`

#### Manual Verification

- [x] `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION=abc accelerator research topic outstanding <slug> --profiles-dir … --start`
      on a set with a web pair prints the notice. Delete the ledger
      afterwards.

---

## Phase 3: Budgets in the Run Ledger and the Plan

### Overview

The sizing plan sizes budgets over `plan.pairs` and stores them in the
ledger. Later plans read them back. Run plans report budgets, demand and
per-spawn budgets, and the sizing plan emits the floor warning.

Budgets persist where 0303 refused to persist concurrency
(0303 plan L162-164). Budgets are a run-wide allocation, whereas concurrency
is a per-batch admission, and AC 21 requires later plans to keep 3/2/1
rather than resize to 9/5.

### Changes Required

#### 1. Domain ledger tests (red)

**File**: `cli/research/src/conduct/ledger.rs` (inline)

| Test | Expected |
|---|---|
| `a_started_ledger_has_no_search_budgets` | `search_budgets()` is `None`; `awaits_search_budgets()` true |
| `budgeting_searches_stores_the_budgets` | `Some(budgets)`; `awaits_search_budgets()` false |
| `continuing_and_recording_keep_the_search_budgets` | after `continue_as` + `record`, unchanged |

#### 2. Domain ledger (green)

`RunLedger` gains `search_budgets: Option<SearchBudgets>`:

```rust
#[must_use]
pub const fn awaits_search_budgets(&self) -> bool { self.search_budgets.is_none() }

#[must_use]
pub fn budgeting_searches(mut self, budgets: SearchBudgets) -> Self { … }

#[must_use]
pub const fn search_budgets(&self) -> Option<&SearchBudgets> { … }
```

`from_parts` gains the `Option<SearchBudgets>` parameter.

#### 3. Codec tests (red) and codec (green)

**File**: `cli/research-adapters/src/conduct/ledger.rs`

The stored shape is `"search_budgets": {"levels": {"1": 3, "2": 2, "3": 1},
"demand": 190}`. It is written only when `Some`. A run with no web pairs
stores `{"levels": {}, "demand": 0}`, so it is not resized.

| Test | Expected |
|---|---|
| `full_ledger()` gains `Some(budgets)` | round-trip test still passes |
| `a_written_ledger_names_its_fields_after_the_memory_it_keeps` | key list gains `"search_budgets"` in sorted position |
| `a_ledger_without_search_budgets_has_none_stored` | `well_formed()` decodes; `search_budgets()` `None` |
| `a_run_without_web_pairs_stores_empty_budgets` | `{"levels": {}, "demand": 0}` round-trips as `Some` with no levels and demand 0, not `Corrupt` |
| `search_budgets_outside_their_shape_are_corrupt` | `"x"`; `{"levels": {}}`; `{"levels": {"0": 1}, "demand": 1}`; `{"levels": {"a": 1}, …}`; `{"levels": {"1": 0}, …}`; `{"levels": {"1": -1}, …}`; `{"levels": {}, "demand": -1}` → `Corrupt` |

Decode follows the `owed_quarantine` precedent: `value.get("search_budgets")
.map(search_budgets_from).transpose()`, with `None` from a malformed value
making the ledger corrupt.

#### 4. Sizing in `run_outstanding`

**File**: `cli/research-cli/src/topic_command.rs`

After `plan` and `observed` are drawn, and before `window`:

```rust
let tree = unsearched_tree(&plan.pairs);
let sized = ledger
    .as_ref()
    .filter(|ledger| ledger.awaits_search_budgets())
    .map(|_| SearchBudgets::sized(&tree, request.search_cap.cap));
if let Some(budgets) = sized.as_ref().filter(|b| b.overruns(request.search_cap.cap)) {
    notices.push(search_wording::search_floor(budgets, &request.search_cap));
}
let ledger = ledger.map(|ledger| match sized {
    Some(budgets) => ledger.budgeting_searches(budgets),
    None => ledger,
});
```

The floor warning joins `notices`, so it reaches stderr as `warning: …` and
`warnings`. It is emitted on the sizing plan whether that plan is the
`--start` or a `--run` over a ledger without budgets.

#### 5. Floor warning wording

**File**: `cli/research-cli/src/search_wording.rs`

```rust
pub fn search_floor(budgets: &SearchBudgets, cap: &EnvironmentSearchCap) -> String
```

Expected for 20 web pairs at depth 3, cap unset:

`the worst-case demand of 260 web searches exceeds the 190 left by the
search cap of 200 from the default after a reserve of 10 — every search
budget is 1; for later runs, raise CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION
and restart Claude Code, or lower the depth`

With the variable set, "from the default" reads "from
CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION". A table-driven unit test covers
both sources. The variable's name is spelt out only in `search_cap.rs` and
`topic_outstanding.rs`, the files the coupling guard admits. `search_floor`
and its tests build the name through `SearchCapSource::raised_by()` and
`reported_as()`, as phase 2's unhonoured notice does. Phase 6 adds the halt
remedy, once a halt exists.

#### 6. Rendering

**File**: `cli/research-cli/src/topic_command.rs`

- When `self.ledger` holds budgets with at least one level, add
  `"search_budgets": [{"level": 1, "budget": 3}, …]` and
  `"search_demand": 190`. A run with no web pairs, and every query plan,
  carries neither.
- `pair()`: a `single_pass` pair whose profile is `SEARCHING_PROFILE` gets
  `"search_budget": budgets.at(1)`. For `research_nodes`, pass
  `Option<NonZeroU32>`, the node's `budgets.at(level)` when the pair is web,
  into `node()`, which adds `"search_budget"` when `Some`. Budgets come from
  `self.ledger`, so query plans carry none.

#### 7. CLI black-box tests

**File**: `cli/research-cli/tests/topic_outstanding.rs`

A helper `web_set(tag, web_pairs, openalex_pairs, arxiv_pairs)` writes an
outline with one focus area per pair and the brief's `source_profiles`. A
second helper, `floor_warning(demand, available, cap, source)`, builds the
exact floor warning text. Every test that expects the floor warning
compares whole entries in `warnings` against it, so phase 6 changes the
remedy in one place.

First extend phase 2's `start_notices_keep_their_order` (red) with 20 web
pairs, so that `warnings` ends with the floor warning. Then add
`a_start_that_cannot_plan_still_reports_its_concurrency_notices`: a
clamped `--concurrency` and a missing profiles directory. It exits non-zero,
and stderr still carries the clamp notice.

| Test | Setup | Expected |
|---|---|---|
| `a_started_run_sizes_search_budgets_over_every_web_pair` | 10 web, depth 3 | budgets 3/2/1, demand 190 (AC 1) |
| `budgets_grow_for_few_pairs` | 3 web, depth 3 | 15/6/3, 189 (AC 2) |
| `a_raised_cap_raises_budgets` | 10 web, depth 3, `SEARCH_CAP=500` | 11/5/2, 470, cap `{500, SEARCH_CAP}` (AC 3) |
| `deep_runs_keep_deeper_levels_at_or_below_shallower_ones` | 10 web depth 5 cap 500; 10 web depth 2; 2 web depth 5 | 7/4/1/1/1·470; 7/3·190; 11/5/3/3/2·190 (AC 4–6) |
| `budgets_of_one_that_overrun_warn_once` | 20 web, depth 3; then `--run R --spawned 0` | 1/1/1, warning names 260, 200 from the default and the remedies, pairs still offered; later plan has no warning (AC 8) |
| `the_reserve_alone_can_force_the_floor` | 15 web, depth 3 | 1/1/1, warning names 195 (AC 9) |
| `a_cap_whose_reserve_still_fits_ones_does_not_warn` | 14 web, depth 3, `SEARCH_CAP=192` | 1/1/1, 182, no warning (AC 10) |
| `non_web_pairs_add_no_demand` | 8 web + 2 openalex, depth 3 | 7/2/1, 184; openalex nodes carry no `search_budget` (AC 11) |
| `an_unhonoured_cap_sizes_against_the_default` | 1 web, depth 3, `SEARCH_CAP=abc` | 42/19/9, 190, notice; later plan no notice (AC 12) |
| `an_unhonoured_zero_cap_warns_against_the_default` | 20 web, depth 3, `SEARCH_CAP=0` | notice naming `'0'`, floor warning naming 200 from the default (AC 13) |
| `a_run_without_web_pairs_sizes_nothing` | openalex + arxiv only, `SEARCH_CAP=abc`; `--start`, then `--run R --spawned 0` | on both plans: no `search_budgets`, no `search_demand`, no notice or warning, and no corrupt-ledger notice on the second (AC 14) |
| `a_re_run_sizes_only_the_remaining_tree` | 1 web depth 3; notes `1`, `2-1` declaring 1, `2-2` declaring 2 | budgets `[{2,35},{3,17}]`, demand 189, no level 1 (AC 15) |
| `the_reserve_rounds_up` | 1 web depth 3, `SEARCH_CAP=201`; `SEARCH_CAP=60` | 42/19/9·190; 13/5/3·57 (AC 16–17) |
| `a_single_pass_run_budgets_its_one_level` | 10 web depth 1 | budget 19, demand 190; each pair `search_budget` 19 (AC 18, CLI half) |
| `a_ledger_without_budgets_is_sized_by_the_next_plan` | write a pre-0304 ledger for the run, 20 web depth 3; `--run R`; then `--run R --spawned 0` | decodes; plan reports 1/1/1 and the floor warning; ledger now holds budgets; the next plan has no warning (AC 19) |
| `a_ledger_without_budgets_still_yields_a_replaced_run_report` | pre-0304 ledger with pending spawns; `--start` | `replaced_run` present (AC 19) |
| `later_plans_keep_the_stored_budgets` | 10 web depth 3; `--start`; write every level-1 note with 1 follow-up; `--run R --spawned 0` | ledger and plan still 3/2/1; every level-2 node `search_budget` 2 (AC 21) |
| `every_web_spawn_carries_its_budget` | 2 web + 1 openalex, depth 2 | every web node has `search_budget` = its level's budget; openalex nodes have none (AC 22, CLI half) |
| `query_plans_carry_no_budgets` | 1 web depth 3, no run flag | no `search_budgets`, `search_demand` or `search_budget` |

#### 8. Skill: fields read and resolution note

**File**: `tests/unit/tasks/test_research_structure.py` (red first)

- Pin step 2 before editing it:
  `test_the_unfinished_run_row_and_step_2_report_are_unchanged` compares the
  "unfinished run" table row (`:546`) and the step 2 report `text` block
  (`:269-290`) byte for byte against their text on `main` (AC 36). It
  passes now and guards every later edit to step 2.
- In `test_research_topic_passes_the_concurrency_flag_through`
  (`:245-248`), change the asserted phrase to the new four-item list below.
  It fails until the skill edit lands.
- `test_conduct_reads_the_search_budget_fields`
- `test_conduct_never_sizes_search_budgets_itself`
- `test_the_floor_warning_reads_alike_in_prose_and_in_the_cli`, using
  `_rust_text` over `search_wording.rs` for "and restart Claude Code, or
  lower the depth"

**File**: `skills/research/research-topic/SKILL.md`

- After the concurrency resolution paragraph (`:68-72`), add: "`outstanding`
  also sizes each web researcher's `WebSearch` budget from Claude Code's
  session search cap, `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`, else 200.
  It keeps back a twentieth of the cap, rounded up, for the main
  conversation, `outline` and searches made earlier in the session.
  `conduct` never sizes budgets itself."
- Step 2's first-plan warnings list (`:263-265`) becomes "a corrupt run
  ledger, a clamped concurrency, an unhonoured search cap, or search budgets
  that cannot fit the cap". `conduct` prints them and proceeds.
- The fields-read list (`:324-345`) names `search_cap`, `search_budgets`,
  `search_demand`, and `search_budget` on a `single_pass` pair and on a node.

#### 9. Configure and docs-site

**File**: `skills/config/configure/SKILL.md`, after "Concurrency and the
subagent cap" (`:448-465`), a paragraph **Web searches and the search
cap**:

- `conduct` gives each web researcher a `WebSearch` budget sized from the
  search cap, which is `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` when set
  to a value Claude Code would accept, otherwise 200, Claude Code's
  session limit.
- It keeps back a twentieth of the cap, rounded up, for the main
  conversation, `outline` and searches made earlier in the session.
- When even one search per node exceeds what is left, every budget is 1 and
  `conduct` warns. Raise the variable, then restart Claude Code, or lower
  the depth (`--depth` or `research.topic.depth`). The CLI warning keeps
  the short form, "lower the depth".
  `--depth`.
- Set the variable where both Claude Code and its Bash tool see it. A value
  Claude Code would not accept is ignored with a warning.

**File**: `tests/unit/tasks/test_research_structure.py`:
`test_configure_documents_the_search_cap` asserts the variable, "otherwise
200", "a twentieth of the cap, rounded up, for the main conversation,
`outline` and searches made earlier in the session", and the remedies
sentence.

**File**: `docs-site/src/content/docs/research.md`: document
`search_budgets` (`level`, `budget`), `search_demand`, and `search_budget`
on the `single_pass` pair and node descriptions (`:480-484`). Say that they
appear on run plans only, are sized once per run and stored in the ledger.
Add `search_budgets` and `search_demand` to the run-only fields sentence
(`:509-511`): they appear when the run has web pairs. Document the floor
warning.

**File**: `CHANGELOG.md`: extend the plans-deepened-research bullet with
per-level web search budgets and the floor warning.

#### 10. Public API pin

Run `mise run public-api:update` for the `research` crate (new `RunLedger`
methods, `from_parts` arity) and commit the regenerated fixture.

### Success Criteria

#### Automated Verification

- [x] Ledger, codec and CLI tests fail first, then pass:
      `cargo test -p research -p research-adapters -p research-cli`
- [x] Structure tests pass: `uv run pytest tests/unit/tasks/test_research_structure.py`
- [x] Public API pin updated and matching: `mise run public-api:check`
- [x] Full local CI mirror: `mise run`

#### Manual Verification

- [x] `outstanding --start` on a real set with 3 web pairs at depth 3
      reports 15/6/3 and demand 189. Run `end-run` afterwards.

---

## Phase 4: Budget Delivery to Researchers and Level Notes

### Overview

Each web spawn carries its budget, the web profile limits `WebSearch` calls
to it, and web level notes record it as `search_budget`.

### Changes Required

#### 1. Schema tests (red) and schema (green)

**File**: `cli/corpus/src/frontmatter_validation/schema.rs`: the level-note
row's `optional_extras: &["search_budget"]`. Extend the row tests
(`:561-608`) to pin it.

**File**: `cli/corpus/src/frontmatter_validation/mod.rs`: validator tests
using `level_note_with` (`:863-872`):

- `a_level_note_with_a_search_budget_validates`;
- `a_level_note_without_a_search_budget_validates`.

**File**: `cli/corpus-cli/tests/frontmatter_goldens.rs` (`:1053-1101`): the
printed row's `"optional_extras": ["search_budget"]`.

#### 2. Template

**File**: `templates/topic-research-level-note.md`: add after `depth: 1`:

```yaml
search_budget: 1                              # web profile only; omit for every other profile
```

`check_extras` now finds it. Confirm with `mise run` that the template-shape
check passes.

#### 3. Level-note outputter

**File**: `skills/research/outputters/level-note-outputter/SKILL.md`,
Injected Values:

- **search budget** — injected for a web node only. Stamp it as
  `search_budget`, the most `WebSearch` calls your node was allowed, not a
  count of calls made. When no search budget is injected, delete the
  `search_budget` line.

#### 4. Web profile

**File**: `skills/research/profiles/web-profile/SKILL.md`, a new
`## Search Budget` section before Outcome, mirroring the openalex phrasing:

- Make at most {search budget} `WebSearch` calls, where {search budget} is
  the search budget injected into your task prompt. `WebFetch` calls do not
  count.
- Refine the question rather than re-search with near-identical text.

#### 5. Spawn prompts

**File**: `skills/research/research-topic/SKILL.md`, step 4:

- the `single_pass` bullet: "for a `web` pair, its `search_budget` as the
  search budget";
- the `research_nodes` bullet: "for a `web` pair, the node's
  `search_budget` as the search budget".

The compose bullet is unchanged. The finding outputter and the finding
template stay untouched, so findings carry no `search_budget`.

#### 6. Structure tests

**File**: `tests/unit/tasks/test_research_structure.py`

| Test | Asserts |
|---|---|
| `test_the_web_profile_limits_searches_to_the_injected_budget` | "Make at most {search budget} `WebSearch` calls" in the web profile |
| `test_web_spawns_carry_their_search_budget` | both spawn bullets name `search_budget`; the compose bullet does not |
| `test_only_web_level_notes_stamp_a_search_budget` | the outputter names `search_budget` and the delete rule |
| `test_findings_carry_no_search_budget` | neither the finding template nor `finding-outputter` mentions `search_budget` (AC 18) |

#### 7. CLI tolerance

**File**: `cli/research-cli/tests/topic_outstanding.rs`:
`a_level_note_with_a_search_budget_is_read_like_any_other`. A written web
note carrying `search_budget: 3` is accepted and its follow-ups derived.

#### 8. Conditional: 0307

If `meta/work/0307-*.md` is `done` when this phase starts, extend 0307's
node query to print a web node's `search_budget` from the ledger snapshot.
Make a `single_pass` web spawn carry its budget inline (AC 33), and add a
CLI test for each. Otherwise skip this step. Check 0307's status again
before merging. If it landed while this phase was in flight, do this step
then.

#### 9. Changelog

**File**: `CHANGELOG.md`: extend the plans-deepened-research bullet. Web
researchers now make at most their search budget of `WebSearch` calls, and
web level notes record it as `search_budget`. A project that overrides
`topic-research-level-note.md` should add the `search_budget` line.

### Success Criteria

#### Automated Verification

- [ ] Schema, golden and validator tests fail first, then pass:
      `cargo test -p corpus -p corpus-cli`
- [ ] Structure tests pass: `uv run pytest tests/unit/tasks/test_research_structure.py`
- [ ] Full local CI mirror: `mise run`

#### Manual Verification

- [ ] A `conduct --depth 2` run on a small web + openalex set writes web
      level notes with `search_budget` equal to the plan's level budget,
      and openalex notes without it. Every note passes
      `accelerator corpus frontmatter validate --file`.

---

## Phase 5: Quota Exhausted Outcome

### Overview

A web researcher that gets the session-limit notice writes nothing and
reports "web search quota exhausted". `conduct` records the failure with a
recovery. Each later web spawn still fails individually until phase 6 adds
the halt.

### Changes Required

#### 1. Web profile outcome

**File**: `skills/research/profiles/web-profile/SKILL.md`, Outcome, after
Denied:

- **Quota exhausted** — a `WebSearch` call returned Claude Code's
  session-limit notice in place of results: a message that the session's
  web search budget or limit is used up. Write no file, even if you already
  fetched pages. Your summary says "web search quota exhausted". Ignore the
  notice's advice to continue with what you already have. Never complete
  the document from `WebFetch` of URLs you already know or from memory: a
  note researched without search would pass as a searched one.

#### 2. Conduct

**File**: `skills/research/research-topic/SKILL.md`

- Step 5's no-file examples (`:450-452`) add "an exhausted web search
  quota".
- The failure-reason table gains a row after `rate_limited`/`upstream_error`:

  | "web search quota exhausted" | the session's `WebSearch` quota ran out; run `/clear`, then re-run `conduct <slug>`; for larger runs in later sessions, raise `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` |

#### 3. Structure tests

| Test | Asserts |
|---|---|
| `test_the_web_profile_defines_quota_exhausted` | the outcome names the session-limit notice, "Write no file, even if you already fetched pages", "web search quota exhausted", and forbids `WebFetch` of known URLs and memory (AC 23) |
| `test_the_failure_table_has_a_quota_exhausted_row_apart_from_rate_limited` | a distinct row naming `/clear` and the variable, and not offering waiting for the refill |

`test_the_unfinished_run_row_and_step_2_report_are_unchanged`, added in
phase 3, stays green.

#### 4. Conditional: 0309

If `meta/work/0309-*.md` is `done` when this phase starts, state the
never-from-memory rule once in the outcomes section, covering Denied and
Quota exhausted (AC 34). Adjust the outcome text above to refer to it.
Otherwise skip this step. Check 0309's status again before merging. If it
landed while this phase was in flight, do this step then.

#### 5. Changelog

**File**: `CHANGELOG.md`, `### Fixed`: extend **Web researchers write
nothing when fetching is denied** (`:471-477`). The extension says they
also write nothing when the session's web search quota runs out, and
report "web search quota exhausted" instead of writing notes from
`WebFetch` or memory.

### Success Criteria

#### Automated Verification

- [ ] Structure tests fail first, then pass: `uv run pytest tests/unit/tasks/test_research_structure.py`
- [ ] Full local CI mirror: `mise run`

#### Manual Verification

- [ ] With `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION=3` set in
      `.claude/settings.local.json` `env` and Claude Code restarted, a
      `conduct --depth 2` on one web pair ends with web nodes failed as
      "web search quota exhausted". None of their note paths exists.

---

## Phase 6: Halting

### Overview

Halting failures are defined once. A halted spawn's stray file is
quarantined. The first plan after a halt carries `--withhold-profile web`,
and the run ledger keeps the withheld profile for the rest of the run. Later
plans therefore never offer web researchers, even when the flag is not
repeated. The run skips the capacity-refusal retries of withheld spawns,
still runs non-web researchers and composers, ends through step 7, and opens
its summary with the halt notice.

### Changes Required

#### 1. Window tests (red)

**File**: `cli/research/src/conduct/window.rs` (inline)

Each test withholds through the `RunMemory` passed to `window`.

| Test | Setup | Expected |
|---|---|---|
| `withheld_researchers_are_neither_offered_nor_remaining` | web single-pass + web nodes + openalex nodes; memory withholds `web` | only openalex offered; `remaining` counts no web spawn |
| `a_withheld_profile_still_composes` | web pair at `Compose`; memory withholds `web` | the compose spawn is offered |
| `withheld_spawns_free_their_slots` | concurrency 2; 3 level-1 web nodes, 1 level-2 openalex node; memory withholds `web` | the openalex node is offered |
| `attempted_withheld_spawns_stay_unfinished` | a web node in `attempted`; memory withholds `web` | listed in `unfinished` |
| `withholding_nothing_offers_as_before` | no memory; memory withholding nothing | unchanged from today |

#### 2. Window (green)

**File**: `cli/research/src/conduct/memory.rs`, `cli/research/src/conduct/window.rs`

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Withheld(BTreeSet<String>);

impl Withheld {
    #[must_use]
    pub fn of(profiles: impl IntoIterator<Item = String>) -> Self { … }
    #[must_use]
    pub fn joined(mut self, other: &Self) -> Self { … }
    #[must_use]
    pub fn contains(&self, profile: &str) -> bool { … }
    pub fn profiles(&self) -> impl Iterator<Item = &str> + '_ { … }
}
```

`Withheld` lives in `memory.rs` as a plain set, with no knowledge of
topic types. `RunMemory` gains `pub withheld: Withheld`. `window`'s
signature is unchanged. `spawns_of` records each spawn's profile on
`RankedSpawn`. After the `attempted` partition, `window` drops each fresh
spawn whose `withheld` contains its profile, unless its `Rank` composes.
That one filter is the whole withholding rule:

```rust
fn held_back(spawn: &RankedSpawn, withheld: &Withheld) -> bool {
    !spawn.rank.composes && withheld.contains(&spawn.profile)
}
```

#### 3. Ledger and codec (red, then green)

**File**: `cli/research/src/conduct/ledger.rs` (inline tests)

| Test | Expected |
|---|---|
| `a_started_ledger_withholds_nothing` | `memory().withheld` empty |
| `withholding_adds_to_what_the_run_already_withholds` | `{web}` then `{web, x}` gives `{web, x}`; `{web}` twice gives `{web}` |
| `continuing_and_recording_keep_the_withheld_profiles` | after `continue_as` + `record`, unchanged |

`RunLedger::withholding(self, profiles: &Withheld) -> Self` joins the
profiles into `memory.withheld`.

**File**: `cli/research-adapters/src/conduct/ledger.rs`

The stored shape is `"withheld": ["web"]`. It is written only when
non-empty, and decoded as empty when absent, following the
`owed_quarantine` precedent.

| Test | Expected |
|---|---|
| `full_ledger()` gains `withheld` `{web}` | round-trip test still passes |
| `a_written_ledger_names_its_fields_after_the_memory_it_keeps` | key list gains `"withheld"` in sorted position |
| `a_ledger_without_withheld_profiles_withholds_none` | `well_formed()` decodes; `withheld` empty |
| `withheld_profiles_outside_their_shape_are_corrupt` | `"web"`; `[1]`; `[""]` → `Corrupt` |

#### 4. Flag

**Files**: `cli/research-cli/src/cli.rs`, `topic_command.rs`, `main.rs`

- `Outstanding` gains:

  ```rust
  /// Hold back researcher spawns of this profile for the rest of the run;
  /// repeatable. Needs `--run`.
  #[arg(long, allow_hyphen_values = true)]
  withhold_profile: Vec<String>,
  ```

- `OutstandingFlags::parse` takes it. Without `--run` it exits 1 with
  `E_TOPIC_RESEARCH_WITHHOLD_PROFILE: --withhold-profile needs --run`. An
  empty value exits 1 with `E_TOPIC_RESEARCH_WITHHOLD_PROFILE:
  --withhold-profile is empty; pass a profile name`, mirroring
  `--concurrency is empty; pass a positive integer`.
- `RunFlag::Continue` and `RunMode::Continue` carry `withheld: Withheld`.
  `run_outstanding` applies `ledger.withholding(&withheld)` to the
  continued ledger before `window`, so the stored ledger keeps it.
- `render` adds `"withheld": [...]` to every run plan, sorted, and empty
  when nothing is withheld. Query plans carry none.

#### 5. `unanswered`

`render` adds `"unanswered"` to every plan. It lists every `plan.pairs`
entry, not narrowed by the window, as `{"stem", "question", "profile",
"stage"}`. `stage` takes the same values as a pair's `stage`
(`single_pass`, `research_nodes`, `compose`).

#### 6. CLI black-box tests

The two withholding tests need web spawns that are still fresh when the
flag arrives. Acknowledging a batch marks every offered spawn attempted,
and attempted spawns are never offered again with or without withholding.
So both tests share one setup: depth 2 and `--concurrency 1`. There is an
openalex pair `01-…` and web pairs `02-…` and `03-…`, so batch 0 offers
only the openalex level-1 node. Without the window change, the next plan
would offer `02-…`'s level-1 node, so both tests fail first.

| Test | Expected |
|---|---|
| `a_withheld_profile_is_not_offered_and_its_spawns_stay_unattempted` | `--start`; `--run R --spawned 0 --withhold-profile web`: offers no web researcher, `remaining` counts none, the ledger's `attempted` holds none, and `withheld` is `["web"]` |
| `a_withheld_profile_stays_withheld_without_the_flag` | then `--run R --spawned 1` with no flag: the same three assertions, and `withheld` is still `["web"]` |
| `every_repeated_withheld_profile_is_kept` | the same setup with an arxiv pair `04-…`; `--withhold-profile web --withhold-profile arxiv`: `withheld` is `["arxiv", "web"]`, and neither profile's researchers are offered |
| `a_halted_run_still_offers_non_web_researchers_and_composers` | web pair at compose + openalex nodes; `--start`, then `--run R --withhold-profile web` with no `--spawned`, which re-offers the pending batch: the compose spawn and the openalex node are both in `offered` (AC 25) |
| `withholding_needs_a_run_and_a_profile` | table: `--withhold-profile web` alone; with `--start`; `--run R --withhold-profile ''` → exit 1, `E_TOPIC_RESEARCH_WITHHOLD_PROFILE`, exact messages |
| `every_plan_lists_every_unanswered_pair_beyond_the_window` | `--concurrency 1`, 3 web pairs: `pairs` has 1, `unanswered` has 3, query and run plans alike |
| `the_tick_plan_lists_every_web_pair_left_unfinished` | AC 28. Depth 3, web pairs A, B and C plus web pair D. A has its level-1 note; B's level-1 spawn failed (no note); C was never spawned; D's tree is complete with no finding. After `end-run`, the query plan's `unanswered` is exactly A (`research_nodes`), B and C (`research_nodes`), and D (`compose`) |
| `a_re_run_after_a_halted_run_ends_offers_the_failed_nodes_afresh` | `--start`; `--run R --withhold-profile web`; `end-run`; `--start` again: no `replaced_run`, `withheld` empty, the failed nodes offered (AC 32) |

Extend the whole-plan equality fixtures with `"unanswered"`, and the run
plan fixtures with `"withheld": []`.

#### 7. Floor warning gains the halt

**File**: `cli/research-cli/src/search_wording.rs`, then
`skills/config/configure/SKILL.md`

The red step updates four things: the phase 3 wording unit test,
`floor_warning` in `topic_outstanding.rs` (which every CLI test that
expects the floor warning shares),
`test_the_floor_warning_reads_alike_in_prose_and_in_the_cli`, and
`test_configure_documents_the_search_cap`. The warning's tail then becomes
"every search budget is 1, so expect a halt, then run /clear and re-run
conduct; for later runs, raise {variable} and restart Claude Code, or lower
the depth". The configure remedies sentence gains the halt in the same
order.

#### 8. Skill: halting failures

**File**: `skills/research/research-topic/SKILL.md`

- A new subsection after the failure-reason table, **Halting failures**,
  the single definition. A halting failure is a reason the orchestrator
  survives, after which some spawns in this session would fail or produce
  degraded work until the user takes a recovery step. Each reason
  withholds a set of profiles. Its table:

  | Reason | Halts | Next plan carries | Recovery |
  |---|---|---|---|
  | "web search quota exhausted" | researcher spawns of `web` pairs: `single_pass` pairs and nodes | `--withhold-profile web` | run `/clear`, then re-run `conduct <slug>`; for larger runs in later sessions, raise `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` |

  Then: "Waiting for the refill is not a recovery: a re-run in the same
  session sizes its budgets against the full cap, not the refilled
  remainder, so it would halt again."
- **Step 2**: the continue command gains `{withhold flags}` after
  `--spawned {batch}`, so the command up to `--spawned {batch}` reads as
  it does today. Once the run halts, every continue plan carries
  `--withhold-profile {profile}` for each profile the halting reason
  withholds, until a run plan's `withheld` names it. After that the ledger
  remembers it for the rest of the run, so later plans may drop it, and
  repeating it is harmless. A plan that should name a profile in
  `withheld` but does not is reported and stops the loop, like a non-zero
  exit. The tick plan in step 7 never carries the flag.
- **Step 4, capacity refusals**: the run halts as soon as any spawn, from
  the batch message or a round, ends with a halting reason. From then on, a
  capacity-refused spawn that reason halts is not re-issued. It is recorded
  as failed with the concurrent-subagent-limit reason. Every other refused
  spawn follows the round rules unchanged, and a round runs only when it
  has a spawn to re-issue. The round rule "The first round always runs."
  (`:435`) becomes "The first round runs whenever it has a spawn to
  re-issue." Before a halt it always does.
- **Step 5**: a spawn whose summary gives a halting reason is recorded as
  failed with that reason, and the run halts. Any file at its output path
  is quarantined first: a finding to `.<name>.invalid`, a note to
  `.<lineage>.md.invalid`, under the existing refuse-to-overwrite rule. Its
  node therefore stays outstanding. Other spawns in the batch are checked
  as normal.
- **Step 6**: unchanged. The ledger never offers a withheld spawn, so the
  loop ends when the plan offers nothing else.
- **Step 7**: the tick plan's `unanswered` is read. The summary's first
  section, above "Run that did not end" and Failures, is **Halt**. It is
  printed only when the last run plan's `withheld` was non-empty, so the
  decision rests on what the CLI reported, not on what `conduct`
  remembers. Each withheld profile maps to its reason through the Halting
  failures table:

  ```text
  conduct halted: {reason}
  ended with it:
  - {stem} {lineage}: {question}
  - {stem}: {question}
  web pairs left unfinished:
  - {stem}: {question}
  {recovery}
  ```

  "ended with it" lists each spawn whose own summary gave the reason, not
  the capacity-refused ones. "web pairs left unfinished" lists each
  `unanswered` entry with `profile` `web` and a `stage` other than
  `compose`. Those spawns are also listed under Failures.
- The fields-read list names `withheld` and `unanswered`.

#### 9. Structure tests

**File**: `tests/unit/tasks/test_research_structure.py`

Existing tests change first (red):

- In `test_the_spawn_step_re_issues_capacity_refused_spawns_in_rounds`
  (`:748-764`), "The first round always runs." becomes "The first round
  runs whenever it has a spawn to re-issue."
- Rename `test_the_summary_opens_with_the_report` (`:327-334`) to
  `test_the_summary_opens_with_the_halt_then_the_report`. It asserts that
  **Halt** comes before **Run that did not end.**, which comes before
  **Failures.**
- `test_research_topic_batches_through_the_spawn_window` (`:211-219`)
  stays unchanged and green: `{withhold flags}` follows the substring it
  pins.

Then the new tests:

| Test | Asserts |
|---|---|
| `test_halting_failures_are_defined_once_with_one_member` | one **Halting failures** heading; its table has exactly one row, "web search quota exhausted" (AC 35) |
| `test_a_halt_withholds_web_researchers_from_the_next_plan_on` | step 2 names `{withhold flags}` after `--spawned {batch}`, `--withhold-profile web`, carrying it "until a run plan's `withheld` names it", and stopping when it does not |
| `test_the_halt_section_follows_the_cli_reported_withheld` | step 7 prints Halt when the last run plan's `withheld` is non-empty |
| `test_halted_capacity_refusals_are_not_retried` | step 4 names "from the batch message or a round", the concurrent-subagent-limit reason, "Every other refused spawn follows the round rules unchanged" and "a round runs only when it has a spawn to re-issue" (AC 30–31) |
| `test_a_halting_failure_is_recorded_and_its_batch_checked_as_normal` | step 5 wording (AC 24, 26) |
| `test_a_halted_spawns_stray_file_is_quarantined` | step 5 names both quarantine names for a halting reason |
| `test_the_halt_notice_names_what_it_halted` | the Halt section names ended-with spawns, the `profile` `web` and non-`compose` filter, recovery, and "Those spawns are also listed under Failures" (AC 27–28) |
| `test_the_halt_recovery_offers_clear_and_a_raised_cap_but_not_the_refill` | AC 29 |
| `test_step_7_runs_in_full_after_a_halt` | no halt-specific exit before `end-run`, tick and manifest |
| `test_the_unfinished_run_row_and_step_2_report_are_unchanged` | carried from phase 3 |

#### 10. Docs and public API

- `docs-site/src/content/docs/research.md`: document `--withhold-profile`
  and its error code in the exit-code table. Say that the run ledger keeps
  a withheld profile until `end-run`, so a later `--run` need not repeat
  the flag. Add `withheld` to the run-only fields sentence (`:509-511`).
  Document `unanswered` on every plan, saying that `unanswered` is the full
  outstanding set and `pairs` is the window over it. Next to the
  leftover-ledger paragraph (`:529-539`), add a short paragraph on the
  quota halt and its recovery.
- `CHANGELOG.md`: extend the plans-deepened-research bullet with
  `--withhold-profile`, `withheld` and `unanswered`, and add the halt to
  the `conduct` concurrency bullet.
- Run `mise run public-api:update` for the `research` crate (`Withheld`,
  `RunMemory.withheld`, `RunLedger::withholding`) and commit the fixture.

### Success Criteria

#### Automated Verification

- [ ] Window, ledger, codec and CLI tests fail first, then pass:
      `cargo test -p research -p research-adapters -p research-cli`
- [ ] Structure tests pass: `uv run pytest tests/unit/tasks/test_research_structure.py`
- [ ] Public API pin updated and matching: `mise run public-api:check`
- [ ] Full local CI mirror: `mise run`

#### Manual Verification

- [ ] The halt run in **Manual Testing Steps** behaves as described.

---

## Testing Strategy

### Unit Tests

- Sizing: every numeric acceptance criterion as a table row, plus an
  enumerated property test for fit, floor, shape and maximality (AC 7).
- Worst-case tree: frontier counting, capped and deduplicated declared
  follow-ups through the shared `capped_follow_ups`, depth bounds, and no
  new trims.
- Depth: `Depth::requested` bounded at 10, `Depth::new` unbounded for
  stamped depths.
- Parse: the 0303 tables, moved to `harness_setting.rs`, plus search-cap
  provenance and unhonoured reporting.
- Ledger: budgets and withheld profiles stored, absent keys and empty
  budgets decoded, corrupt shapes, round-trip.
- Window: withholding never touches compose or `unfinished` and frees
  slots.

### Integration Tests

- `topic_outstanding.rs` black-box runs for every CLI-observable acceptance
  criterion. These cover the sizing plan, later plans, the pre-0304 ledger,
  query mode, per-spawn budgets, the start-notice order, withholding and
  its persistence, `unanswered` on the AC 28 tick plan, and the re-run after
  a halted run.
- `test_research_structure.py` pins every prose acceptance criterion and
  the Rust-to-prose wording parity of the floor warning.

### Manual Testing Steps

1. Set `"CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION": "3"` under `env` in
   `.claude/settings.local.json` and restart Claude Code.
2. Brief and outline a throwaway set with one `web` focus area and one
   `openalex` focus area. Run `conduct <slug> --depth 2`.
3. Expect the floor warning first: demand 5 against a cap of 3 from the
   variable, reserve 1.
4. Expect web nodes to fail as "web search quota exhausted" with no note
   written. Later batches still spawn the openalex nodes and their composer.
5. Expect the summary to open with the halt notice. It names the failed
   spawns, the web pair left unfinished, and `/clear` plus a re-run. There
   should be no `.conduct-run.json` left, and the outline and manifest
   should match disk.
6. Remove the variable, `/clear`, re-run `conduct <slug> --depth 2`. Expect
   no replaced-run report, and the web nodes researched with notes stamped
   `search_budget`.

## Performance Considerations

⏱️ Sizing is `O(levels · log(4 · cap))` for the binary search, plus a
top-up of a few iterations. The depth ceiling holds `levels` to 10 at
most. The worst-case count adds one pass over each frontier's written notes
per plan. Both costs are negligible beside reading the set.

## Migration Notes

- A ledger written before this change has no `search_budgets` and no
  `withheld`. It decodes as unsized and withholding nothing. `--start`
  replaces it as before (0310's report is unaffected), and a `--run` over
  it sizes and stores budgets.
- The level-note schema only gains an optional extra, so existing notes
  stay valid. A project that overrides `topic-research-level-note.md`
  should add the `search_budget` line; the CHANGELOG entry in phase 4 says
  so.
- Plan JSON only gains fields (`search_cap`, `search_budgets`,
  `search_demand`, `search_budget`, `withheld`, `unanswered`), which keeps
  the additive contract at `cli.rs:87-88`.
- ⚠️ Compared with 1.23.0, a configured `research.topic.depth` above 10,
  which was dormant there, now clamps to 10 with a warning. A hand-run
  `--depth` above 10 is refused with `E_TOPIC_RESEARCH_DEPTH`; the flag is
  new in this release. A finding stamped with any depth keeps it.

## References

- Work item: `meta/work/0304-fail-web-nodes-on-websearch-quota-exhaustion.md`
- Research: `meta/research/codebase/2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion.md`
- Work item review: `meta/reviews/work/0304-fail-web-nodes-on-websearch-quota-exhaustion-review-1.md`
- Cap precedent: `cli/research-cli/src/subagent_cap.rs`,
  `meta/decisions/ADR-0071-optional-harness-variables-in-the-cli.md`,
  `meta/plans/2026-10-10-0303-research-concurrency-subagent-limit.md`
- Ledger precedent: `meta/plans/2026-10-13-0310-recovering-from-an-interrupted-conduct-run.md`
- Claude Code docs: `code.claude.com/docs/en/tools-reference` ("Session
  search limit"), `code.claude.com/docs/en/env-vars`
