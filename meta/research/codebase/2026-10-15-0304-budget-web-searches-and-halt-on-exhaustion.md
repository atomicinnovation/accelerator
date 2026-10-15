---
type: "codebase-research"
id: "2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion"
title: "Codebase seams for 0304: budget web searches and halt on exhaustion"
date: "2026-10-15T21:19:05+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0304"
parent: "work-item:0304"
relates_to: ["codebase-research:2026-10-10-0303-research-concurrency-subagent-limit", "codebase-research:2026-10-13-0310-recovering-from-an-interrupted-conduct-run", "codebase-research:2026-09-26-0283-recursive-finding-deepening"]
topic: "Codebase seams for 0304: budget web searches and halt on exhaustion"
tags: ["research", "codebase", "research-topic", "conduct", "web-profile", "research-cli", "run-ledger", "topic-tree", "frontmatter-schema"]
revision: "cefdb91b95219535496b07eafd8bdc509b1133d7"
repository: "accelerator"
last_updated: "2026-10-15T21:19:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Codebase seams for 0304: budget web searches and halt on exhaustion

**Date**: 2026-10-15T21:19:05+00:00
**Author**: Toby Clemson
**Git Commit**: cefdb91b95219535496b07eafd8bdc509b1133d7 (jj working copy, unpushed)
**Branch**: none (detached; trunk at `ef059b4833ad`)
**Repository**: accelerator

## Research Question

Where and how does the codebase need to change to implement work item 0304
(`meta/work/0304-fail-web-nodes-on-websearch-quota-exhaustion.md`)? The item
covers per-level `WebSearch` budgets sized from
`CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` and stored in the run ledger, their
delivery to web spawns and level notes, a Quota exhausted outcome in the web
profile, and a halting-failure class in `conduct`.

## Summary

The CLI half has clear precedents to follow. The plan half has two gaps the
work item does not name.

- **Search cap** follows `cli/research-cli/src/subagent_cap.rs` almost
  exactly: an injectable `lookup` seam, a `CapSource`-style provenance, and a
  pure wording module. One difference: 0304 reports an unhonoured *harness*
  value, which the subagent cap never does. ADR-0071 and the Claude-coupling
  lint also have to admit the new variable by name.
- **Budgets in the ledger** follow the `owed_quarantine` precedent: an absent
  key decodes to a default, and a malformed one is `Corrupt`. The codec is
  hand-written over `serde_json::Value`, with no serde derive.
- **Worst-case tree** needs a new walk. `TreeWalk::walk` stops at the first
  level with a missing node, so the frontier is always a single level, and
  nothing in the codebase counts nodes per level.
- **Sizing input** must be `plan.pairs`. The renderer emits `window.pairs`,
  which the concurrency window narrows.
- **Halting has no CLI support today.** If `conduct` withholds offered web
  spawns, the ledger still marks them attempted when the batch is
  acknowledged. Every later plan then lists them under `unfinished` as
  "wrote no note", and they keep taking window slots ahead of deeper non-web
  spawns. The plan needs a decision here (see Open Questions).
- **Step 7's tick plan cannot list every unfinished web pair.** The tick plan
  is a query-mode plan, and its `pairs` are narrowed to the first
  `concurrency` spawns.
- **Prose seams** are all located: spawn bullets, capacity-refusal rounds,
  step 5, the summary order, the failure table, the web profile's outcomes,
  the level-note outputter and template, the schema row, the configure
  section, and the docs-site page. Each has an existing structural test style
  to pin it.
- ✅ **The sizing algorithm in Requirements reproduces all 16 numeric
  acceptance criteria.** I checked this with a scratch Python model (see
  Budget algorithm check).

## Detailed Findings

### Search cap resolution and notices

The concurrency cap is the template. Follow the shipped code, not the 0303
plan text, where the two differ.

- `cli/research-cli/src/subagent_cap.rs:54-79`:
  `from_environment(lookup: impl Fn(&str) -> Option<String>) -> EnvironmentCap`.
  Production injects `|name| std::env::var(name).ok()` at
  `cli/research-cli/src/main.rs:224`. Unit tests inject a `HashMap`
  (`subagent_cap.rs:118-123`).
- `subagent_cap.rs:16-49`: `CapSource { Variable(&'static str), HarnessDefault }`
  with `raised_by()` and `reported_as()` (`"default"`), plus
  `EnvironmentCap { cap, source, unhonoured: Option<UnhonouredCap> }`.
- `subagent_cap.rs:85-99` (`harness_slots`): mirrors Claude Code's parse.
  It trims JavaScript whitespace (U+0085 kept, U+FEFF stripped), accepts an
  optional `+` and ASCII digits only, rejects `0` through `NonZeroU32`, and
  saturates overflow to `u32::MAX`. The 0303 plan's parser case tables
  (`meta/plans/2026-10-10-0303-research-concurrency-subagent-limit.md:310-312`)
  are a ready template for 0304's open parse-edge assumption.
- `subagent_cap.rs:57-64`: only `ACCELERATOR_SUBAGENT_CAP` is ever reported
  as unhonoured. A bad `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` falls back
  silently, and the test at `:202-218` pins that. 0304 requires the opposite
  for `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`.
- `subagent_cap.rs:10-14`: `HARNESS_DEFAULT` is a `const` built with
  `match NonZeroU32::new(20)`, so it needs no `unwrap`. A test at `:271-279`
  pins it to the domain default and the config catalogue.
- `cli/research-cli/src/concurrency_wording.rs:13-18`: `unhonoured_cap` gives
  `"{variable} is '{value}', not a positive integer — ignoring it"`. All
  wording functions are pure and return a `String`. The house style is:
  name the source, quote the value, em dash, the action taken, then the
  remedy after `;`.
- `cli/research-cli/src/topic_command.rs:262-269`: notices are added only in
  the `RunMode::Start` arm. `start_notices` (`:429-442`) chains the
  unhonoured notice before the clamp warnings. Emission (`:327-340`) sends
  each notice to stderr as `warning: …` and also appends it to the JSON
  `warnings`. The test `concurrency_warnings_are_raised_only_when_a_run_starts`
  (`cli/research-cli/tests/topic_outstanding.rs:2470-2488`) pins the
  start-only rule.
- `topic_command.rs:562-565`: `subagent_cap: {slots, source}` is rendered
  unconditionally. A search-cap field would sit beside it.

**Crate layering.** `cli/research` is the domain crate: `SubagentCap`,
`Concurrency` and `window()`, with no environment access and no wording.
`cli/research-adapters` does file I/O, including the ledger codec.
`cli/research-cli` is the composition root: environment reads, wording and
JSON rendering. So the budget-sizing algorithm and the worst-case tree belong
in `cli/research` as pure functions. Env reading and notice wording belong in
`cli/research-cli`.

**Coupling guard.** `tasks/lint/claude_coupling.py:44-49`
(`_OPTIONAL_HARNESS_VARIABLES`) admits a `CLAUDE_*` name only in its reader
and that reader's test. `tests/unit/tasks/test_claude_coupling.py:229`
(`test_the_admitted_set_is_exactly_the_subagent_cap`) pins the admitted set.
Both must change, and the test needs renaming.

### Run ledger

- `cli/research/src/conduct/ledger.rs:44-49`:
  `RunLedger { run: RunId, claims: ClaimedIndexes, memory: RunMemory }`.
  `RunMemory` (`cli/research/src/conduct/memory.rs:62-69`) holds `pending`,
  `attempted`, `just_acknowledged`, `seen` and `owed_quarantine`. The ledger
  holds no depth, cap, profiles or budgets.
- Constructors:
  - `start(run, &observed)` (`ledger.rs:67-76`), optionally followed by
    `.owing_quarantine(..)`.
  - `continue_as(stored, run, spawned)` (`:115-136`). It acknowledges batch
    `spawned` and moves every pending spawn into `attempted`.
  - `from_parts` (`:156-166`), the decode constructor.
- `record(plan, observed, window)` (`:141-153`): spawns dropped from a
  changed offer also count as attempted. The 0310 plan therefore warns
  against drawing an extra private plan within a run (0310 plan L144-147).
- Codec (`cli/research-adapters/src/conduct/ledger.rs`):
  - `ledger_to` (`:103-143`) and `ledger_from` (`:145-171`) are hand-written
    over `serde_json::Value`. A missing or ill-typed required key decodes to
    `None`, which maps to `LedgerError::Corrupt` (`:74-76`).
  - The optional-field precedent is at `:166-168`:
    `value.get("owed_quarantine").map_or_else(|| Some(BTreeSet::new()), stem_set)?`.
  - `number()` (`:181-183`) reads a `u32`.
- `a_written_ledger_names_its_fields_after_the_memory_it_keeps`
  (`research-adapters/src/conduct/ledger.rs:414-443`) pins the sorted
  top-level key list. The `well_formed()` fixture lacks `owed_quarantine`, so
  it already exercises the absent-key path.
- Writers and readers (`topic_command.rs`):
  - `--start` reads any existing ledger as the replaced ledger (`:450-460`)
    and starts a fresh one (`:305-308`).
  - `--run` continues the ledger through `continued_ledger` (`:389-401`).
  - Query mode neither reads nor writes a ledger (test at
    `topic_outstanding.rs:941-949`).
  - Both run modes write the ledger before rendering (`:318-326`). That order
    is what makes `pending` trustworthy.
- `end-run` is `run_end_run` (`topic_command.rs:365-387`). The
  replaced-ledger report is built by `Recovery::of`
  (`cli/research/src/conduct/replaced_run.rs:78-102`). It reads the replaced
  ledger's `pending`, `claims`, `run` and `owed_quarantine` only, so a new
  budgets field does not affect it, provided decoding tolerates its absence.

The sizing plan is the first `--start`, or the first `--run` whose stored
ledger has no budgets. That maps onto the two ledger-producing arms:
`Start` always sizes, and `Continue` sizes only when the decoded budgets are
absent.

### Topic tree and the worst-case tree

- `cli/research/src/topic/tree.rs:45-57`: `follow_up_cap(level)` starts at
  `STARTING_FOLLOW_UP_CAP = 4` and halves with `div_ceil(2)`, giving 4, 2, 1,
  1, … (pinned at `:409-413`). This is the only ceiling division in the
  codebase; std `u32::div_ceil` serves for ⌈cap / 20⌉.
- `tree.rs:108-120`: `derive` returns `Stage::SinglePass` only when there is
  no `.levels/` directory and `depth == Depth::default()`. Depth 1 with a
  levels directory goes through the tree walk.
- `tree.rs:161-184` (`TreeWalk::walk`): returns `ResearchNodes(missing)` at
  the first level that has any missing node. The frontier is therefore one
  level, and deeper levels are never derived while it is open.
- `tree.rs:219-254` (`follow_ups`): truncates a note's `follow_ups` to the
  cap (`:240`) and deduplicates against `known` (`:246`). Dedup keeps the
  original positions, leaving gaps (`:241`). Real counts never exceed the
  worst case.
- Failed nodes have no tree state. A node with no note, or a refused note, is
  a `MissingNode` with `rejected` set (`tree.rs:70-84`, `:202-213`). "Failed"
  is a ledger fact: `attempted` membership, surfaced through
  `Window.unfinished`.
- Depth is a per-call `--depth` flag that defaults to `"1"` and is never read
  from config (`cli/research-cli/src/cli.rs:99-103`). The skill resolves
  `research.topic.depth` and passes it on every plan
  (`skills/research/research-topic/SKILL.md:251`, `:259`). The ledger does
  not store depth, so nothing stops a continued plan from passing a
  different one.

The worst-case count needs a new pure function over the same inputs `derive`
reads. Per pair, the count at the frontier level L is the missing nodes at L.
The count at L+1 is the capped, deduplicated follow-ups declared by notes
written at L, plus `follow_up_cap(L)` for each missing node at L. Every deeper
level multiplies by its cap, down to the depth limit. Levels above L
contribute nothing. Applied to acceptance criterion 15: 2 missing level-2
nodes, plus (1 + 2) declared level-3 follow-ups and 2 × 2 under the missing
nodes, gives the stated 2 level-2 and 7 level-3 nodes. A `SinglePass` pair
counts as one level-1 node, and `Compose` pairs contribute nothing.

### Plan and window

- `cli/research/src/topic/plan.rs:39-49`: `RoundPlan { items, pairs, … }`.
  `OutstandingPair { pair: Pair { question, profile }, stem, stage }`
  (`:62-67`).
- `profile` is a plain `String`, and `DEFAULT_PROFILE = "web"` is at
  `cli/research/src/topic/outline.rs:8`. No profile enum exists, so "is a
  web pair" is a string comparison.
- `cli/research/src/conduct/window.rs:40-75`: ranks spawns by compositions
  last, then level, stem and lineage (`:78-84`). It partitions out
  `attempted` spawns as `unfinished` and admits `concurrency` of the rest.
  `narrowed` (`:125-147`) rebuilds `pairs` to keep only offered spawns.
- `topic_command.rs:502-507`: the JSON `pairs` comes from `window.pairs`,
  never `plan.pairs`. Sizing must therefore read `plan.pairs` before
  narrowing; the work item says the same.
- Budget attachment points:
  - `PlanOutput::node` (`topic_command.rs:654-669`), next to `"cap"`. It
    receives only `(NoteRef, &MissingNode)` today, so `pair()` must pass the
    profile in to gate on web.
  - The `SinglePass` arm of `pair()` (`:631-634`), with the level-1 budget.
  - `PlanOutput.ledger: Option<&RunLedger>` (`:483`) is where stored budgets
    can be read; it is `None` in query mode.
- `cli.rs:87-88` sets the output contract: "fields are only ever added".
  Whole-plan equality tests (`topic_outstanding.rs:217-289`, near `:789`)
  must gain any unconditional new key.
- `cli/research/tests/fixtures/public-api.txt` pins the domain crate's
  public API. New public items need `mise run public-api:update`.

### Conduct skill prose

All references are to `skills/research/research-topic/SKILL.md`.

- **Spawn prompts** (`:370-414`). The `single_pass` bullet (`:376-386`) and
  the `research_nodes` bullet (`:387-404`) list the values copied from the
  plan; the budget joins both, for web pairs only. The compose bullet
  (`:405-414`) carries no profile, and composers have only Read and Write
  (`agents/composer.md:7`).
- **Plan fields read** (`:321-353`). The skill ignores fields not named here
  (`:349-350`), so new plan fields must be named, including the budget, the
  plan-report fields and the floor warning.
- **Capacity-refusal rounds** (`:424-446`). The halting rule cuts these
  short: refused web spawns are not re-issued once the run is halted, and
  keep the concurrent-subagent-limit reason. The round-continuation rule
  (`:437-439`) still governs the non-web retries.
- **Step 5** (`:448-460`). A no-file spawn is recorded with "the reason its
  summary gives". A Quota exhausted reply arrives here as a failure reason,
  and this is where halting starts.
- **Step 6** (`:462-474`). Records each `unfinished` entry not already
  recorded, as "wrote no note" when it has no `rejected` text. This rule
  interacts with withheld web spawns (see Halting mechanics).
- **Step 7** (`:476-531`). `end-run`, the tick plan (a query-mode plan with
  no run flag), the manifest, then a summary in a fixed order. The halt
  notice goes first, above Failures (`:506-528`). 0310's lesson (0310 plan
  L91-95) applies: sessions reliably print the final summary but not
  mid-run text, so the halt notice belongs there.
- **Failure-reason table** (`:533-548`). `rate_limited`/`upstream_error` is
  at `:537`, the capacity-refusal row at `:545`, and the unfinished-run row,
  left unchanged, at `:546`.
- **Concurrency resolution** (`:68-72`). This is the place to state that
  `outstanding` also sizes search budgets and that `conduct` never resolves
  them.

### Profiles, researcher, outputters

- `skills/research/profiles/web-profile/SKILL.md`: no call cap. Outcomes are
  at `:57-70`: Pages, Denied (`:62-65`: "Write no file; your summary says
  "fetch denied by permissions". Never write the document from memory
  instead …"), Unreachable and None found. Quota exhausted joins this list.
  0309 (still draft) would merge the never-from-memory rule.
- `skills/research/profiles/openalex-profile/SKILL.md:39-40`, the phrasing to
  mirror: "Make at most 3 `search` and 5 `lookup` calls that reach the CLI."
  The arxiv profile forbids `WebSearch` at `:49`.
- Profiles contain no `{…}` placeholders and no substitution step. They name
  injected values generically ("your injected paths", web `:31-32`). The
  budget becomes an injected value that the profile refers to by name, as
  the outputters' "Injected Values" lists do.
- `agents/researcher.md`: outcomes travel back only through the two-to-three
  sentence summary (`:38-40`). So "web search quota exhausted" must be that
  summary's exact wording, as with Denied's "fetch denied by permissions".
- `skills/research/outputters/level-note-outputter/SKILL.md:25-35` lists the
  injected values the note stamps. `search_budget` joins it, as web-only.
- `templates/topic-research-level-note.md:1-23`: the frontmatter, with
  unquoted integers (`round: 1`, `level: 1`, `depth: 1`).

### Frontmatter schema

- `cli/corpus/src/frontmatter_validation/schema.rs:284-302`: the level-note
  row has `optional_extras: &[]`. The schema stores key names only; types are
  enforced by readers (for example `stamped_depth`,
  `cli/research-adapters/src/topic.rs:256-262`).
- Adding `search_budget` to `optional_extras` has two knock-ons:
  - `cli/corpus/src/frontmatter_validation/template_shape.rs:368-381`
    (`check_extras`) requires every extra, optional ones included, to appear
    in the template, otherwise `TEMPLATE-MISSING-EXTRA`. The shared template
    must therefore carry `search_budget`, and the outputter must tell
    non-web researchers to drop it.
  - `cli/corpus-cli/tests/frontmatter_goldens.rs:1053-1101` pins the printed
    row, including `"optional_extras": []`.
- Row tests to extend: `schema.rs:561-608`. Validator tests: helper
  `level_note_with` at `cli/corpus/src/frontmatter_validation/mod.rs:863-872`.

### Configure and docs-site

- `skills/config/configure/SKILL.md:448-465`: "Concurrency and the subagent
  cap". The search-cap section goes beside it. It is pinned by
  `test_configure_documents_the_subagent_cap_clamp`
  (`tests/unit/tasks/test_research_structure.py:185-200`).
- `docs-site/src/content/docs/research.md:455-551`: the `outstanding`
  reference. Plan fields are at `:464-491`, the cap paragraph at `:493-505`,
  and the run ledger and `end-run` at `:507-539`. The generated
  `docs-site/src/content/docs/reference/skills/**` is gitignored, so do not
  edit it there. No test asserts on `research.md`.
- `CHANGELOG.md:140` mentions `subagent_cap`. The 0303 work added
  `[Unreleased]` entries for it.

### Test seams

| Layer | File | Pattern |
|---|---|---|
| Env parse | `cli/research-cli/src/subagent_cap.rs:101-280` | table-driven over a `HashMap` lookup |
| Sizing / tree | `cli/research/src/topic/tree.rs:289-616`, `cli/research/tests/*.rs` | pure; `note()`, `directory()`, `derived()` helpers |
| Ledger codec | `cli/research-adapters/src/conduct/ledger.rs:241-512` | `StubStore`; JSON-pointer breach tables; absent-key fixture |
| CLI black box | `cli/research-cli/tests/topic_outstanding.rs` | `plan_in(args, env)`; `assert_warned`; `command_without_an_inherited_cap()` at `:155-159` |
| Skill prose | `tests/unit/tasks/test_research_structure.py` | `_prose`, `_section`, `_conduct()`; phrase, order and Rust-wording parity (`_rust_text`, `:438-445`) |

`command_without_an_inherited_cap()` must also `env_remove` the new variable
(ADR-0071 L97-98). Without that, a developer's own setting leaks into the
tests.

## Code References

- `cli/research-cli/src/subagent_cap.rs:7-99` - cap variables, `CapSource`, `EnvironmentCap`, `from_environment`, `harness_slots`
- `cli/research-cli/src/concurrency_wording.rs:13-18` - unhonoured-value notice wording
- `cli/research-cli/src/topic_command.rs:243-356` - `run_outstanding`: mode arms, ledger write, notice emission
- `cli/research-cli/src/topic_command.rs:429-442` - `start_notices`
- `cli/research-cli/src/topic_command.rs:502-507` - JSON `pairs` built from `window.pairs`
- `cli/research-cli/src/topic_command.rs:562-565` - `subagent_cap` rendering
- `cli/research-cli/src/topic_command.rs:619-669` - `pair()` and `node()`, the budget attachment points
- `cli/research-cli/src/topic_command.rs:365-387` - `run_end_run`
- `cli/research/src/conduct/ledger.rs:44-166` - `RunLedger`, `continue_as`, `record`
- `cli/research-adapters/src/conduct/ledger.rs:103-183` - ledger codec; optional field at 166-168
- `cli/research/src/conduct/window.rs:40-147` - window, rank, `unfinished`, `narrowed`
- `cli/research/src/topic/tree.rs:45-57` - `follow_up_cap`
- `cli/research/src/topic/tree.rs:161-254` - frontier walk and follow-up expansion
- `cli/research/src/topic/outline.rs:8` - `DEFAULT_PROFILE = "web"`
- `cli/corpus/src/frontmatter_validation/schema.rs:284-302` - level-note schema row
- `cli/corpus/src/frontmatter_validation/template_shape.rs:368-381` - optional extras must be in the template
- `skills/research/research-topic/SKILL.md:370-548` - spawn, refusal rounds, steps 5–7, failure table
- `skills/research/profiles/web-profile/SKILL.md:57-70` - outcomes
- `skills/research/profiles/openalex-profile/SKILL.md:39-40` - call-cap phrasing
- `skills/research/outputters/level-note-outputter/SKILL.md:25-35` - injected values
- `templates/topic-research-level-note.md:1-23` - level-note frontmatter
- `skills/config/configure/SKILL.md:448-465` - subagent cap docs
- `docs-site/src/content/docs/research.md:455-551` - `outstanding` reference
- `tasks/lint/claude_coupling.py:44-49` - optional harness variable admissions

## Architecture Insights

### Halting mechanics

The ledger's guarantee ("never offers a spawn twice") rests on
acknowledgement. Every spawn in `pending` moves to `attempted` when
`--spawned {batch}` is passed, whether or not `conduct` spawned it. A halted
run that only filters offered spawns in prose therefore has three
consequences:

1. Withheld web spawns turn up in the next plan's `unfinished`, and step 6
   records them as failed with "wrote no note". The work item wants the halt
   notice to name only the spawns that ended with the halting reason; it
   does not want every withheld node listed as a failure.
2. Withheld web spawns take window slots. Rank orders by level before
   profile, so a level-2 openalex node waits behind every level-1 web node.
   That produces batches with nothing to spawn.
3. The loop still ends, because every withheld spawn becomes attempted, but
   it churns through one plan per window's worth of web nodes.

```text
  halt detected (step 5)
        |
        v
  next plan --run R --spawned B ----> window offers web + non-web
        |                                     |
        | (prose filter only)                 | (CLI-side exclusion)
        v                                     v
  web spawns acknowledged -> attempted   window admits non-web only
  -> "unfinished: wrote no note"         web nodes stay outstanding,
  -> slots wasted                        unattempted, unrecorded
```

A CLI-side exclusion avoids all three: for example, a flag on continued plans
naming the halted spawn class, or a halt recorded in the ledger. It also
keeps the halted spawns out of `attempted`, so a re-run treats them as fresh.
This is a design choice for the plan, not something the work item settles.

### Unfinished web pairs at step 7

The tick plan runs in query mode. Its `pairs` are narrowed to the first
`concurrency` spawns, and `items[].complete` is per focus area, not per pair.
So "every web pair with outstanding web nodes when step 7 runs" cannot be
read reliably from the tick plan once outstanding spawns exceed the
concurrency. Options: a new always-present plan field listing outstanding
pairs by stem and profile, or a query-mode plan unbounded by concurrency.
The CLI owns tree state, so the listing belongs there; the skill should not
reconstruct it.

### Divergence from 0303's non-goal

0303 rejected persisting resolved concurrency in the ledger
(0303 plan L162-164), on the grounds that each plan re-resolves it. 0304
deliberately persists budgets, so that later plans read them back instead of
resizing (acceptance criterion 21: 3/2/1 kept rather than 9/5). The plan
should record why it diverges: budgets are a run-wide allocation, whereas
concurrency is a per-batch admission.

### Budget algorithm check

I modelled Requirements step 3 (largest candidate `k` whose
`max(1, ⌊k · weight⌋)` budgets fit) and step 4 (deepest-first top-up under
the shallower-level bound) in Python. The model reproduces every numeric
criterion:

| Case | Worst-case nodes per level | Budgets | Demand |
|---|---|---|---|
| 10 web, d3, cap 200 | 10/40/80 | 3/2/1 | 190 |
| 3 web, d3 | 3/12/24 | 15/6/3 | 189 |
| 10 web, d3, cap 500 | 10/40/80 | 11/5/2 | 470 |
| 10 web, d5, cap 500 | 10/40/80/80/80 | 7/4/1/1/1 | 470 |
| 10 web, d2 | 10/40 | 7/3 | 190 |
| 2 web, d5 | 2/8/16/16/16 | 11/5/3/3/2 | 190 |
| 20 web, d3 | 20/80/160 | 1/1/1, floor warning | 260 |
| 15 web, d3 | 15/60/120 | 1/1/1, floor warning | 195 |
| 14 web, d3, cap 192 | 14/56/112 | 1/1/1 | 182 |
| 8 web + 2 openalex, d3 | 8/32/64 | 7/2/1 | 184 |
| 1 web, d3 (`abc` or 201) | 1/4/8 | 42/19/9 | 190 |
| 1 web, d3, cap 60 | 1/4/8 | 13/5/3 | 57 |
| 10 web, d1 | 10 | 19 | 190 |
| re-run, AC 15 | –/2/7 | –/35/17 | 189 |
| resize, AC 21 | –/10/20 | –/9/5 | 190 |

Two details for the implementation:

- **Candidate search is finite.** For each level, `k = n / weight` with
  `n ≤ cap − reserve`. Integer arithmetic, scaling every weight by their LCM
  of 4, avoids fractions.
- **"Shallower level" means the nearest shallower level with remaining
  nodes.** Budgets are monotone, so this equals "every shallower level".
  Levels with no remaining nodes take no budget (AC 15).

## Historical Context

- `meta/decisions/ADR-0071-optional-harness-variables-in-the-cli.md`: the
  CLI may read an optional `CLAUDE_*` variable that has a stated default,
  admitted by name in its reader and test only (L67-71). The admitted set
  "must not grow by convenience" (L95-96). Its wording is specific to the
  subagent cap and describes an `ACCELERATOR_*` counterpart taking
  precedence. 0304 deliberately has no counterpart (Drafting Notes), so the
  ADR needs an amendment or a successor that records the exception.
- `meta/plans/2026-10-10-0303-research-concurrency-subagent-limit.md`: cap
  design, start-only notices, parser case tables (L310-312), test strategy
  (L1491-1538), and docs touched (L949-998). It hands "stopping retry rounds
  early on a halting failure" to 0310 (L167-168); 0310 passed it on to 0304.
- `meta/plans/2026-10-13-0310-recovering-from-an-interrupted-conduct-run.md`:
  new ledger fields must default when absent, or an interrupted pre-upgrade
  run decodes as corrupt and loses `replaced_run` (L1493-1499). It also
  covers the summary-print lesson (L91-95) and the unfinished-run row kept
  inline (L1373-1386).
- `meta/research/codebase/2026-10-13-0310-recovering-from-an-interrupted-conduct-run.md`:
  records 0304 as owner of the halting-failure class (L375-377).
- `meta/reviews/work/0304-fail-web-nodes-on-websearch-quota-exhaustion-review-1.md`:
  verdict APPROVE. Items left to the plan:
  - parse edge cases (L369-370, L401-402);
  - membership of pairs outside the window in the halt notice (L457-459);
  - whether the notice and the floor warning both name the value (L328-330);
  - phasing along the two halves (L485);
  - an exhaustion trigger for testing (L487).

## Related Research

- `meta/research/codebase/2026-10-10-0303-research-concurrency-subagent-limit.md`
- `meta/research/codebase/2026-10-13-0310-recovering-from-an-interrupted-conduct-run.md`
- `meta/research/codebase/2026-09-26-0283-recursive-finding-deepening.md`
- `meta/research/codebase/2026-09-20-0282-tunable-depth-and-breadth.md`

## Open Questions

- ❓ **Halt mechanism.** Should a prose-only filter be used, accepting
  withheld web spawns becoming attempted, listed as `unfinished` and taking
  window slots? Or a CLI-side exclusion of web researcher spawns on continued
  plans? The second needs a new flag or ledger field and changes the window.
- ❓ **Unfinished web pairs.** Should a new plan field list outstanding pairs
  regardless of the window, or should the tick plan be unbounded? Today the
  tick plan cannot enumerate them past `concurrency` spawns.
- ❓ **Query-mode budgets.** The tick plan has no ledger. Should query-mode
  plans omit budgets, or size them for display? The work item defines plans
  only within a run.
- ❓ **ADR-0071.** Amend it, or write a successor admitting
  `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` without an `ACCELERATOR_*`
  counterpart and with its unhonoured value reported?
- **Depth drift.** Budgets are keyed by level and the ledger does not store
  depth. A later plan passing a larger `--depth` would reach levels with no
  stored budget. The skill always re-passes the same depth, so this is a
  defensive case; storing depth alongside budgets would close it.
- **Shared template.** `search_budget` must appear in the shared level-note
  template to satisfy `check_extras`, so non-web researchers need an
  explicit drop rule in the outputter. Alternatively the schema check could
  exempt it.
- **Parse edge cases** (`-5`, `1.5`, `0200`, whitespace, overflow): reuse
  `harness_slots` if Claude Code parses the search variable the same way. I
  did not verify this against Claude Code; the work item leaves it to the
  plan.
- 0307 and 0309 are both still `draft`, so their conditional acceptance
  criteria do not apply if 0304 lands first.
