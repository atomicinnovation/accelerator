---
type: "codebase-research"
id: "2026-11-18-PP-893-pass-conduct-spawn-context-by-reference"
title: "Pass Conduct Spawn Context by Reference"
date: "2026-11-18T20:05:34+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "PP-893"
parent: "work-item:PP-893"
topic: "Pass Conduct Spawn Context by Reference"
tags: ["research", "codebase", "research-topic", "conduct", "run-ledger", "research-guard", "config-template", "cli"]
revision: "22e2773e3d1a9f2beb9967be6e61fe05bc3b2eb1"
repository: "accelerator"
last_updated: "2026-11-18T20:05:34+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Pass Conduct Spawn Context by Reference

**Date**: 2026-11-18T20:05:34+00:00
**Author**: Toby Clemson
**Git Commit**: 22e2773e3d1a9f2beb9967be6e61fe05bc3b2eb1
**Branch**: detached working copy (jj), not on a bookmark
**Repository**: accelerator

## Research Question

What in the codebase does work item 0307 touch, and how does it work
today? The item makes `research-topic`'s `conduct` pass spawn context by
reference: templates by path, and node context (question,
`known_questions`, search budget) through a new read-only
`accelerator research topic node` query answered from an offered-node
snapshot in the run ledger.

## Summary

The change spans five areas. All of them have clear seams, and each has a
close precedent in the code.

1. **Plan rendering**: `PlanOutput::node` in
   `cli/research-cli/src/topic_command.rs:955-978` is the one place a
   node's eight fields are put together. `question` and `known_questions`
   come from the domain `MissingNode` (`cli/research/src/topic/tree.rs:83-89`),
   which `tree::derive` computes from the *live* tree. Dropping the two
   fields from the plan is a local edit there.
2. **Run ledger**: `PendingBatch` (`cli/research/src/conduct/memory.rs:15-62`)
   holds only `number` and `Vec<SpawnRef>`. `RunLedger::record` replaces it
   from each window's `offered` list. `--spawned N` acknowledges it, which
   empties it. A snapshot kept beside it and rebuilt in `record` therefore
   has exactly the item's "current offered batch" lifetime with no new
   lifecycle logic. The ledger JSON is hand-rolled, with no serde derives
   (`cli/research-adapters/src/conduct/ledger.rs`), and optional keys
   default through `map_or` in `ledger_from`.
3. **Read-only query**: `consultable`, `counts` and `report-path` set the
   pattern. Each is a clap variant in `cli/research-cli/src/cli.rs`,
   dispatched in `main.rs::topic`, resolving the set with
   `topic_command::resolve_set` and calling a generic `run_*` over
   filesystem ports that returns `Printed`. `end-run` and `continued_ledger`
   already produce the exact errors the query needs for a missing or
   corrupt ledger (`E_TOPIC_RESEARCH_RUN_LEDGER`) and a superseded run
   (`E_TOPIC_RESEARCH_RUN_SUPERSEDED`).
4. **Research guard**: the allowlist is the single constant
   `PERMITTED_PREFIX = "accelerator research fetch "`
   (`cli/research/src/confinement.rs:13`), checked with `strip_prefix` and
   then a lexer. There is no denylist of topic subcommands; they are blocked
   only because they lack the prefix. Adding the node query means a second
   permitted prefix. The constant is in the public-API snapshot.
5. **Templates by path**: `accelerator config template` resolves three
   tiers: config path, user override, then plugin default. It prints only
   fenced content. `ResolvedTemplate.abs_path` already exists, and
   `templates show` and `templates list` print a display path, but nothing
   prints a usable absolute path. A path-printing mode is a small launcher
   change.

The skill prose follows from these: step 4's spawn bullets, step 2's field
list, the template sections, the agents, the web profile, the outputters
and the troubleshooting table.

Several design points the work item leaves open, or gets slightly wrong,
are listed under **Open Questions**. The main one is that the node query
needs the pair's **profile** to decide whether to print `search_budget`,
and the item's snapshot field list does not include it.

## Detailed Findings

### 1. How `outstanding` builds and renders a node today

- **Clap surface**: `TopicAction` (`cli/research-cli/src/cli.rs:99-179`)
  has `Outstanding`, `EndRun`, `ReportPath`, `Counts` and `Consultable`.
  Number and run flags are raw strings, validated by hand so bad values
  exit 1 with the tool's own `E_*` message rather than clap's exit 2
  (`cli.rs:107-109`).
- **Dispatch**: `main.rs::topic` (`cli/research-cli/src/main.rs:177-217`)
  maps each variant to `Result<Printed, String>`. `Err` goes to stderr with
  exit 1 (`failure`, `main.rs:428-431`). Every handler starts with
  `working_project()` and then `topic_command::resolve_set`.
- **Slug resolution**: `resolve_set` (`topic_command.rs:84-98`) collapses
  every non-resolved outcome to
  `E_TOPIC_RESEARCH_UNRESOLVED: no topic-research set '{slug}'`, exit 1.
  This is the query's unknown-slug case.
- **Pipeline**: `run_outstanding` (`topic_command.rs:290-386`) runs these
  steps in order:
  1. `opened()` reads the ledger and checks the run for `--run`.
  2. `read_round_inputs` reads the set.
  3. `RoundPlan::of` derives the plan.
  4. Search budgets are worked out (`OpenedRun::ledger`, `:464-492`).
  5. `window()` chooses the batch.
  6. `ledger.record(...)` stores the batch and writes the ledger before
     rendering (`:359-363`).
- **Node rendering**: `PlanOutput::node` (`topic_command.rs:955-978`) emits
  `lineage`, `level`, `question`, `cap` (`MissingNode::cap()` =
  `follow_up_cap(level)`), `id` (`NoteRef::id(&self.set_slug())`), `path`
  (`set_root.join(NoteRef::path())`), `known_questions`, `spawn`
  (`SpawnRef::Node(address)`), plus `rejected` and `search_budget` when
  present. `set_slug()` (`:984-989`) is the resolved directory's name, not
  the user's slug argument. The node query must use the same value so its
  `id` matches.
- **Search budget**: `PlanOutput::search_budget`
  (`topic_command.rs:945-953`) returns
  `ledger?.search_budgets()?.at(level).unwrap_or(NonZeroU32::MIN)`, gated
  by `searches(profile)`. The `SEARCHING_PROFILE = "web"` constant is in
  `cli/research/src/conduct/search_budget.rs:12`, not in `topic_command.rs`
  as the work item says. Note the fallback to 1 for an uncovered level; the
  query should do the same.
- **Where node context comes from** (`cli/research/src/topic/tree.rs`):
  - `TreeWalk::walk` (`:255-297`) offers only the shallowest incomplete
    level (the frontier).
  - `known_questions(lineage)` (`:395-408`) is the pair question, then the
    ancestor questions, then every recorded follow-up from levels above the
    frontier, deduplicated by normalised form. Because the walk re-reads
    notes on every plan, a sibling note written mid-batch changes later
    derivations. This is why the item needs a snapshot taken when the node
    is offered.
  - Follow-up positions are 1-based slots in the parent note. A skipped
    duplicate leaves a gap, so `2-1` can be absent while `2-2` exists.
- **Note addressing**:
  - `NoteRef::path()` gives `findings/<stem>.levels/<lineage>.md`
    (`cli/research/src/topic/layout/note_ref.rs:19-21`).
  - `NoteRef::id(set_slug)` gives `<set_slug>.<stem>.<lineage>` (`:26-28`).
  - `Display` gives `<stem>:<lineage>` (`:31-35`).
  - `SpawnRef::parse` (`cli/research/src/conduct/spawn.rs:20-28`) already
    parses `<stem>:<lineage>` into `SpawnRef::Node(NoteRef)`, refusing
    non-canonical lineages. It is a ready-made parser for `--node`.

### 2. The run ledger and pending batch

- **Domain types** (`cli/research/src/conduct/`):
  - `SpawnRef { Pair(Stem), Node(NoteRef) }` (`spawn.rs:11-15`). A
    `single_pass` researcher and a composer are both `Pair`.
  - `PendingBatch { number: u32, spawns: Vec<SpawnRef> }`
    (`memory.rs:15-19`). It has these operations:
    - `acknowledge(spawned)` (`:29-35`): a matching number takes the spawns
      and bumps the number. A non-matching number is silently ignored.
    - `replace(spawns)` (`:39-51`): returns the displaced spawns and bumps
      the number only when the old batch was non-empty and differs.
  - `RunMemory` (`memory.rs:95-103`): `pending`, `attempted`,
    `just_acknowledged`, `seen`, `owed_quarantine`, `withheld`.
  - `RunLedger { run, claims, memory, search_budgets: Option<SearchBudgets> }`
    (`cli/research/src/conduct/ledger.rs:46-52`). It has these operations:
    - `continue_as` (`:141-162`) checks `stored.run == run` and returns
      `Superseded { by }` if not, then applies `--spawned`.
    - `record(plan, observed, window)` (`:168-184`) calls
      `pending.replace(window.offered.clone())`.
- **The offer lifecycle, matched to the item's semantics**:
  - **Offer**: every `outstanding --start` or `--run` plan calls `record`,
    so the pending batch always equals the latest `window.offered`.
  - **Repeat plan before `--spawned`**: spawns still outstanding are offered
    again. A spawn whose note has landed drops out and is "displaced" into
    `attempted`. The window ranks by level, so a newly unlocked child can
    join. Tests in `ledger.rs`:
    `a_repeated_plan_without_acknowledgement_re_offers_the_same_batch`
    (`:405-420`) and
    `replacing_an_unacknowledged_batch_after_its_notes_land_attempts_it`
    (`:468-474`). The item wants a node still pending to keep its
    first-offered context. The snapshot rebuild in `record` must therefore
    carry over the old snapshot's entry for any spawn that is in both the
    old `pending.spawns` and the new `offered`, and take a fresh one from
    `window.pairs` only for new spawns.
  - **`--spawned N`**: `acknowledge` empties `spawns`. The same plan's
    `record` then fills it with the next offer, so the snapshot naturally
    covers only the new batch. That is the item's "a spawn ref offered only
    in the first offer exits non-zero".
  - **Capacity re-issue (0303)**: happens inside step 4 before any re-plan.
    The ledger is untouched, so the query answers unchanged.
  - **Halt**: the halt plan runs with `--spawned {batch} --withhold-profile
    web`. It acknowledges the halting batch and `record`s a non-web offer.
    Withheld web spawns go to `window.withheld_spawns` and are never in
    `offered`, so they are never snapshotted and the query refuses them.
    Until that plan runs, the halting batch is still pending and answers.
    This matches the item's last criterion.
    - The withholding is the `is_withheld` partition in `window()`
      (`cli/research/src/conduct/window.rs:44-87`, `:105-107`). The
      **`held_back` filter the item names does not exist in the window**:
      the only `held_back` is in `topic/consultation.rs:34-46`, for
      `consultable`.
- **Where the snapshot's data is at `record` time**:
  - `window.pairs` holds the narrowed `OutstandingPair`s with
    `Stage::ResearchNodes(Vec<MissingNode>)` for exactly the offered nodes
    (`window.rs:69`, `:144-166`).
  - `OutstandingPair.pair.profile` gives the profile.
  - `record` currently takes `&Window`, so no new plumbing is needed to
    reach the data.
- **Serialised ledger** (`cli/research-adapters/src/conduct/ledger.rs`):
  - **Location**: `<set_root>/.conduct-run.json` (`LEDGER_NAME`, `:34`).
  - **Writing**: `ledger_to` (`:107-152`) builds the JSON with `json!`.
    - Verified at `:120-152`: `owed_quarantine` and `withheld` are **always
      written**, empty or not. Only `search_budgets` is written
      conditionally.
    - The item says 0304 made both `search_budgets` and `withheld` "written
      only when set". That is true only of `search_budgets`. Both default
      when absent on read.
  - **Reading**: `ledger_from` (`:166-198`) treats a missing required key
    as `None`, which becomes `LedgerError::Corrupt`.
    - The required keys are `run`, `pending.{number,spawns}`, `claims`,
      `attempted`, `just_acknowledged` and `seen.{notes,answered}`.
    - The optional keys use
      `.map_or_else(|| Some(default), parser)?`, so they default when absent
      and are corrupt when malformed.
    - A new snapshot key fits this pattern directly. Growing
      `pending.spawns` entries into objects would instead require
      `spawn_list` to accept both strings and objects. Under 0310's
      constraint, a separate key is simpler.
  - **Key-list test**: `a_written_ledger_names_its_fields_after_the_memory_it_keeps`
    (`:493-533`) asserts the alphabetical top-level key list, because
    `serde_json::Map` is a BTreeMap. A new key joins that list.
    `full_ledger()` (`:402-439`) and `well_formed()` (`:445-454`, the legacy
    shape) are the fixtures to extend.
- **0310's interrupted-run report**:
  - `Recovery::of` (`cli/research/src/conduct/replaced_run.rs:79-101`)
    reads only `run`, `claims`, `pending.spawns()` and `owed_quarantine`.
  - On `--start`, a corrupt ledger downgrades to a warning
    (`topic_command.rs:535-537`). The run is not failed, but the report is
    lost.
  - So the snapshot must default when absent, and must not change the
    `pending.spawns` encoding unless decoding stays compatible.
- **Public API**: `cli/research/tests/fixtures/public-api.txt` lists the
  ledger and memory items (e.g. `:121`, `:135-165`). Changing
  `PendingBatch`'s public signatures, or adding a snapshot type, means
  regenerating it. Callers of `PendingBatch::new`:
  - `memory.rs` tests
  - `window.rs:574`
  - `replaced_run.rs:501`
  - adapter tests

### 3. Modelling the read-only node query

Closest model: **`consultable`**.

1. `cli.rs:172-178` defines the variant.
2. `main.rs:205-207` dispatches it, and `:288-292` resolves the set.
3. `cli/research-cli/src/consultable_command.rs:30-55` has
   `run_consultable<F: DirReader + FileReader + DirectoryProbe>` returning
   `Printed`.

`report_path_command::parse_question` (`:16-23`) shows argument validation
before set resolution. The query's checks, in order, could be:

| Check | Existing mechanism | Error |
|---|---|---|
| `--run` well-formed | `topic_command::parse_run` (`:261-268`) | `E_TOPIC_RESEARCH_RUN` |
| `--node` well-formed | `SpawnRef::parse`, requiring the `Node` variant | new code, e.g. `E_TOPIC_RESEARCH_NODE` |
| slug resolves | `resolve_set` | `E_TOPIC_RESEARCH_UNRESOLVED` |
| ledger present and readable | `read_run_ledger` → `Ok(None)` / `Corrupt` | `no_usable_ledger(run)` (`:710-715`) |
| run owns ledger | `ledger.run() == run`, as in `run_end_run` (`:555-577`) | `superseded(run, owner)` (`:703-708`) |
| node in snapshot | new | new message |

Nothing in the query needs `read_round_inputs` or the profiles directory.
It reads the ledger, looks the spawn ref up, and renders:
- `id` from `NoteRef::id(set_root.file_name())`
- absolute `path` from `set_root.join(NoteRef::path())`
- `search_budget` from `ledger.search_budgets()` for a web node

It writes nothing. That can be tested with the `tree()` before/after
snapshot from `cli/research-cli/tests/topic_project/mod.rs` and the
ledger-bytes-unchanged check
(`topic_outstanding.rs:1728`, `a_superseded_continuation_leaves_the_owners_ledger_bytes_unchanged`).

Distinguishing "never started" from "superseded": `continue_as` reports
both as `Superseded { by: stored.run }`, since there is one ledger per set
and any id that is not its owner is "superseded". The item's two cases
(superseded run, and a run id never started) both exit non-zero, so the
existing error covers both. Only the wording differs.

### 4. The research guard

- **Registration**: `hooks/hooks.json:41-63` registers it under PreToolUse
  for `Bash` and `Write|Edit|MultiEdit|NotebookEdit`, running
  `bin/accelerator research guard --fail-safe --non-blocking`. It is not
  registered in agent frontmatter.
- **Handler**: `cli/research-cli/src/guard.rs`. `run()` (`:52-85`) passes
  the main thread and unconfined agents, then `judge` calls
  `research::confinement::decide`. A block exits 2.
- **Decision**: `cli/research/src/confinement.rs`.
  - `PERMITTED_PREFIX` (`:13`) is the single allowlist entry.
  - `command_decision` (`:307-316`): `strip_prefix(PERMITTED_PREFIX)`, else
    `Block::WrongCommand`; then
    `Lexer::new(arguments).first_construct()`, which blocks separators,
    pipes, `$`, backticks, substitutions, `<` and most `>`.
  - A composer is blocked from every command (`Block::NoCommands`).
  - The refusal text names the prefix:
    `E_RESEARCH_GUARD_COMMAND: {agent} may run only '{PERMITTED_PREFIX}…'`
    (`:501-509`).
- **To permit the node query**: replace the constant with a set of
  permitted prefixes, e.g. fetch plus `"accelerator research topic node "`
  with its trailing space. Try each in `command_decision` and run the same
  lexer on the remainder. This keeps:
  - `outstanding`, `end-run`, `report-path`, `counts`, `consultable` on
    `WrongCommand`, with no denylist;
  - `topic nodex` and a bare `topic node` blocked, as `fetchx` and a bare
    `fetch` are today;
  - chaining blocked.

  The refusal wording must then name both commands.
- **Tests to touch or extend**:
  - `cli/research/tests/confinement.rs`:
    - `a_command_without_the_prefix_is_the_wrong_command` (`:188-199`)
    - refusal text (`:397-441`)
    - baseline test (`:110-152`), which substitutes
      `PERMITTED_PREFIX.trim_end()`
  - `cli/research-cli/tests/guard.rs`:
    - `the_researcher_may_run_nothing_else` (`:169-175`, exact message)
    - composer blocked (`:386`)
    - configured names (`:498-504`)
  - Python:
    - `tests/integration/hooks/test_research_guard_registration.py`
    - `tests/integration/research/test_lexer_differential.py` (`PREFIX`
      at `:42`)
  - The public-API fixture line `public-api.txt:542`.
- **Permissions**: `research-topic`'s `allowed-tools` already grants
  `Bash(accelerator research topic *)`
  (`skills/research/research-topic/SKILL.md:22`), and researchers inherit
  it. The skill needs no new Bash grant; only the docs' fallback allow
  rules gain the node rule.

### 5. `accelerator config template` and a path-printing mode

- **Clap**: `ConfigAction::Template { name, kind, allow_legacy_layout,
  fail_safe }` (`cli/launcher/src/launch/inbound/cli.rs:291-306`). It maps
  to `config_cli::Action::Template` (`cli/launcher/src/launch/mod.rs:120-130`).
- **Handler**: `cli/launcher/src/config_command/inbound/cli.rs:220-255`.
  - `resolve_template` then `template_render::fenced`.
  - Not found is a `Failure::Refusal` that fails closed even under
    `--fail-safe` (tested at `cli/launcher/tests/config_read.rs:1978-1989`).
- **`--kind`**: `template_view::resolve`
  (`cli/launcher/src/config_command/core/template.rs:26-39`) tries
  `<name>-<kind>` first, then `<name>`. Its unit tests are at `:285-336`.
  No integration test passes `--kind`.
- **Three tiers** (`FileConfigStore::resolve_template`,
  `cli/config-adapters/src/store.rs:395-437`):
  1. the `templates.<name>` config path, absolute or project-relative;
  2. the user override `<paths.templates>/<name>.md`, default
     `.accelerator/templates`;
  3. the plugin default `<plugin root>/templates/<name>.md`.

  The work item names only tiers 2 and 3. The path mode must also honour
  tier 1. That tier is the realistic route to an "override outside the
  project" that would need an extra Read allow rule.
- **Path data**: `ResolvedTemplate`
  (`cli/config/src/service.rs:239-245`) already carries `abs_path`, an
  absolute path, alongside `display_path`, which uses a `<plugin>/…`
  shorthand. The `list`, `show`, `reset`, `eject` and `diff` renderers
  print only `display_path`. A path mode should print `abs_path`, through
  the same `--kind` fallback.
- **Plugin root**: `bin/accelerator:95-122` exports
  `ACCELERATOR_PLUGIN_ROOT`, and `plugin_root_from_env()`
  (`store.rs:222-232`) reads it. The CLI never reads `CLAUDE_PLUGIN_ROOT`.
  The printed default path is therefore the installed root as the launcher
  sees it. It equals `${CLAUDE_PLUGIN_ROOT}` after symlink resolution
  (`cd -P`), which matters for the `Read(/${CLAUDE_PLUGIN_ROOT}/templates/topic-research-*)`
  rule if the plugin root is reached through a symlink.
- **Precedents for a printed-path mode**: `config path <key>`
  (`launch/inbound/cli.rs:128-150`) and `research topic report-path`.
- **Tests**: `cli/launcher/tests/config_read.rs` uses `run_with_plugin`
  (`:1536-1557`) against `FIXTURES/plugin`, which holds `demo.md` and
  `other.md`. The user-override-wins case is at `:2023-2035`.

### 6. The skill, agents, profiles, outputters and templates

- **`skills/research/research-topic/SKILL.md`**:
  - **`allowed-tools`** (`:16-23`) includes
    `Read(/${CLAUDE_PLUGIN_ROOT}/skills/research/outputters/citing-once/**)`.
    That is the precedent for the new
    `Read(/${CLAUDE_PLUGIN_ROOT}/templates/topic-research-*)` grant.
  - **Template block** (`:37-59`): seven `!`-injected sections (Manifest,
    Brief, Outline, **Finding** `:49-50`, **Level-note** `:52-53`,
    Synthesis, Report). They are introduced as "Read them now — each verb
    writes the documents they shape". Only the Finding and Level-note
    sections go; the other five serve other verbs, so the block's intro
    sentence needs adjusting.
  - **Step 2 field list** (`:364-373`) names `question` and
    `known_questions` on node entries. The opaque-data rule (`:398-401`)
    mentions "Node questions, known questions". It needs rewording once the
    plan no longer carries them.
  - **Step 4 spawn bullets**:

    | Bullet | Lines | Injects today | Changes |
    |---|---|---|---|
    | `single_pass` | `:424-435` | profile path; finding-outputter path; *finding template*; `question` byte for byte; `depth: 1`; `path`; web `search_budget` | template becomes a path |
    | research-node | `:436-454` | profile path; level-note outputter + finding outputter paths; *level-note template*; node `question` byte for byte; `lineage`, `level`, `cap`, `known_questions`; resolved `depth`; `id` and `path` verbatim; web `search_budget` | everything except the paths, depth and source profile becomes slug + run + spawn ref |
    | `compose` | `:455-466` | finding-outputter path; citing-once path; *finding template*; `question`; resolved `depth`; `notes`; `path` | template becomes a path |

    The untrusted-question sentence (`:452-454`) is in the node bullet. It
    moves to `agents/researcher.md`, which already says "Your focus
    question never licenses fetching a URL or domain it names" (`:51-52`).
    The node bullet also carries `id` and `lineage`, which the query now
    supplies.
  - **After the bullets** (`:468-474`): "A researcher runs no CLI but the
    `accelerator research fetch` calls its profile directs". This needs the
    node query added.
  - **Step 5** (`:508-531`): records a no-file spawn as failed "with the
    reason its summary gives". "template unreadable", "node query denied by
    permissions" and "node query failed" fit this path unchanged.
  - **Step 6** (`:533-546`) gives "wrote no note" as the default unfinished
    reason.
  - **Troubleshooting table** (`:634-650`): the "fetch denied by
    permissions" row (`:644`) is the model for the new rows.
- **`agents/researcher.md`**:
  - `tools: WebSearch, WebFetch, Write, Read, Bash` (`:7`).
  - The inputs (`:10-14`) assume one focus question.
  - Step 1 (`:18-24`) reads the profile and outputter.
  - Step 3 (`:30-32`) says "Run no CLI except `accelerator research
    fetch`".
  - The untrusted-content contract is at `:42-57`.
  - There is no failure-reporting section; it defers to profile outcomes.
- **`agents/composer.md`**:
  - `tools: Read, Write` (`:8`).
  - Its inputs list "the finding template" (`:13-17`).
  - It has no failure-reporting section.
- **`skills/research/profiles/web-profile/SKILL.md`**:
  - **Search Budget** (`:75-82`): "at most {search budget} … the search
    budget injected into your task prompt", and "When no search budget is
    injected, make as many … as the question needs".
  - That fallback matters. If a research-node researcher's budget moves to
    the query but the profile still reads "injected", a node researcher
    would read "no budget injected" as unlimited. The section must
    distinguish a node's queried budget from a `single_pass` injected one.
  - The Denied outcome (`:89-92`) is the model for the "node query denied"
    wording.
- **`skills/research/outputters/finding-outputter/SKILL.md`**:
  - The injected values (`:20-32`) list "finding template".
  - The Shape section says "If no template was injected, refuse rather than
    improvise a shape" (`:34-52`). This becomes the "template unreadable"
    rule.
- **`skills/research/outputters/level-note-outputter/SKILL.md`**:
  - The injected values (`:25-39`) include level, depth, lineage, id,
    follow-up cap and known questions.
  - **search budget**: "injected for a web node only … When no search
    budget is injected, delete the `search_budget` line". All of these move
    to "from the node query".
- **Templates**:
  - `templates/topic-research-finding.md` has no `search_budget`.
  - `templates/topic-research-level-note.md` has `id:
    "{set-slug}.{finding-stem}.{lineage}"` and `search_budget: 1   # web
    profile only` (`:15`).

### 7. Tests that pin today's behaviour

- **`tests/unit/tasks/test_research_structure.py`**:
  - **Helpers**:
    - `_prose` (`:55`), whitespace-collapsed;
    - `_section` (`:175`);
    - `_spawn_step` (`:996`);
    - `_spawn_bullet(start, end)` (`:1053`), with boundaries
      "**a `single_pass` pair**", "**a node of a `research_nodes` pair**",
      "**a `compose` pair**" and "Each agent writes";
    - `_fields_read` (`:282`);
    - `_failure_reasons` and `_rows_naming` (`:1120-1128`);
    - `_split` + `frontmatter.splitlines()` for `allowed-tools` grants.
  - **Injected-item checks are substring checks over a bullet slice; there
    is no per-item parser.** The item's acceptance preamble requires a
    structure test that "reads a spawn bullet's injected fields from its
    list of injected items, one field per item". That parser is new work.
  - **Tests the item names, with lines**:
    - `test_research_topic_loads_the_level_note_template` (`:685`)
    - `test_the_researcher_denies_a_focus_question_licensing_a_source`
      (`:657`)
    - `test_conduct_reads_the_search_budget_fields` (`:297`)
    - `test_web_spawns_carry_their_search_budget` (`:1057`)
    - `test_the_web_profile_limits_searches_to_the_injected_budget`
      (`:1041`)
    - `test_only_web_level_notes_stamp_a_search_budget` (`:1076`)
    - `test_research_topic_grants_the_read_of_the_citing_once_rule`
      (`:163`)
  - **Not pinned anywhere**: the node bullet's untrusted-question sentence
    and its individual node fields.
  - Also touched: `test_research_topic_routes_compose_pairs_to_the_composer`
    (`:703`) and `test_findings_carry_no_search_budget` (`:1088`).
- **`cli/research-cli/tests/topic_outstanding.rs`**:
  - **Node assertions**:
    - the full node object in
      `a_missing_level_two_note_holds_back_level_three` (`:608-636`);
    - `every_web_spawn_carries_its_budget` (`:3163`);
    - `later_plans_keep_the_stored_budgets` (`:3133`).
  - **Fixture helpers** (file-local):
    - `small_set` (`:348`) and `web_set` (`:2712`); the slug is always
      `s`, so ids are `s.<stem>.<lineage>`;
    - `level_note` (`:493`) and `level_note_with` (`:479`);
    - `started` (`:926`) and `continued` (`:937`);
    - `stored_ledger` (`:2793`);
    - `LEDGER_BEFORE_BUDGETS` (`:3060`);
    - `assert_exits_1` (`:830`);
    - `no_usable_ledger` (`:1890`) and `superseded` (`:1706`);
    - `command_without_an_inherited_cap` (`:156`).
  - **No fixture yields budgets 8/5/3.** Budgets are sized by the CLI from
    the cap and the web pairs' worst-case tree. The item's fixture needs
    either a hand-written ledger
    `"search_budgets": {"levels": {"1": 8, "2": 5, "3": 3}, …}` (the stored
    shape is asserted at `:3151`), or a cap and pair count chosen to
    produce them. The first fits "literals in the test" better. The
    `03-a-web` and `04-b-openalex` stems do not yet exist as a fixture.
  - **Placement**: a new `topic_node.rs` would want these helpers, which
    live only in `topic_outstanding.rs`. `topic_project/mod.rs` has only
    `Project`, `run`, `tree` and `copy_tree`. The options are to extend the
    shared module or to put the query tests in `topic_outstanding.rs`.

### 8. Docs

`docs-site/src/content/docs/research.md`:
- `### research topic outstanding` (`:612`). Its node field set at `:642`
  lists `question`, `cap`, `id`, `path`, `known_questions`, …, and
  `search_budget` (`:645`).
- `## guard` (`:417`), with `### Command matching` (`:452`) and the
  single-command statement at `:490`.
- `### Granting the fetch` (`:500`), with an allow JSON at `:511-516`.
- The `## topic` intro allow block (`:589-610`).
- The exit-code table (`:750-763`).

There is no section titled "fallback allow rules". The item's "fallback
allow rules" are the two allow blocks at `:511-516` and `:603`. A new
`### research topic node` section would sit beside `report-path` (`:765`),
`counts` and `consultable`.

## Code References

- `cli/research-cli/src/cli.rs:99-179`: `TopicAction`, where a `Node`
  variant goes
- `cli/research-cli/src/main.rs:177-217`, `:288-292`: topic dispatch and
  the `consultable` composition root
- `cli/research-cli/src/topic_command.rs:84-98`: `resolve_set`
- `cli/research-cli/src/topic_command.rs:261-268`: `parse_run`
- `cli/research-cli/src/topic_command.rs:353-363`: window, `record`,
  ledger write
- `cli/research-cli/src/topic_command.rs:555-577`: `run_end_run`, the
  ledger owner check
- `cli/research-cli/src/topic_command.rs:703-715`: `superseded`,
  `no_usable_ledger`
- `cli/research-cli/src/topic_command.rs:945-989`: `search_budget`, `node`,
  `absolute`, `set_slug`
- `cli/research-cli/src/consultable_command.rs:30-55`: read-only command
  model
- `cli/research/src/topic/tree.rs:83-97`, `:255-297`, `:395-408`:
  `MissingNode`, walk, `known_questions`
- `cli/research/src/topic/layout/note_ref.rs:19-35`: note path, id, display
- `cli/research/src/conduct/spawn.rs:11-38`: `SpawnRef`
- `cli/research/src/conduct/memory.rs:15-62`, `:95-112`: `PendingBatch`,
  `RunMemory`
- `cli/research/src/conduct/ledger.rs:141-184`: `continue_as`, `record`
- `cli/research/src/conduct/window.rs:27-87`, `:144-166`: `Window`,
  withholding, narrowing
- `cli/research/src/conduct/search_budget.rs:12`, `:103-105`, `:143-146`:
  `SEARCHING_PROFILE`, `at`, `searches`
- `cli/research/src/conduct/replaced_run.rs:79-134`: 0310's reader of the
  ledger
- `cli/research-adapters/src/conduct/ledger.rs:107-198`: `ledger_to`,
  `ledger_from`
- `cli/research-adapters/src/conduct/ledger.rs:493-533`: key-list test
- `cli/research/src/confinement.rs:13`, `:278-316`, `:501-509`: guard
  allowlist, decision, refusal text
- `cli/research/tests/fixtures/public-api.txt`: public-API snapshot
  (`PERMITTED_PREFIX` at `:542`)
- `cli/launcher/src/launch/inbound/cli.rs:291-306`: `config template`
  clap
- `cli/launcher/src/config_command/core/template.rs:26-50`: `--kind`
  resolution
- `cli/config-adapters/src/store.rs:395-437`, `:475-520`: three-tier
  resolution, `ResolvedTemplate`
- `cli/config/src/service.rs:239-245`: `ResolvedTemplate { abs_path,
  display_path, … }`
- `skills/research/research-topic/SKILL.md:16-23`, `:37-59`, `:364-401`,
  `:418-474`, `:634-650`: grants, templates, plan fields, spawn bullets,
  troubleshooting
- `agents/researcher.md:10-57`, `agents/composer.md:13-52`
- `skills/research/profiles/web-profile/SKILL.md:75-104`
- `skills/research/outputters/finding-outputter/SKILL.md:20-52`,
  `skills/research/outputters/level-note-outputter/SKILL.md:25-39`
- `tests/unit/tasks/test_research_structure.py`: structure tests (see §7)
- `cli/research-cli/tests/topic_outstanding.rs`,
  `cli/research-cli/tests/guard.rs`, `cli/research/tests/confinement.rs`,
  `cli/launcher/tests/config_read.rs`

## Architecture Insights

- **Domain, adapter and CLI layering is strict**:
  - `research` is pure: ledger, window and tree as domain types.
  - `research-adapters` does the hand-rolled JSON and filesystem reading.
  - `research-cli` renders JSON and wires ports in `main.rs`.

  A snapshot type belongs in `research::conduct`, built in
  `RunLedger::record` from `&Window`. Its encoding belongs in the adapter's
  `ledger_to` and `ledger_from`. The query's rendering belongs in a new
  `*_command.rs`.
- **The ledger encoding is defensive**: unknown, malformed or
  non-canonical values decode as corrupt. New optional state follows the
  `owed_quarantine` and `search_budgets` absent-defaults pattern, with
  corrupt-shape tests.
- **Each `outstanding` plan writes the ledger before printing.** The ledger
  therefore always reflects the plan the orchestrator holds. That is what
  makes "answer from the snapshot" safe for researchers spawned from that
  plan.
- **The guard is an allowlist of exact prefixes plus a lexer, not a parser
  of subcommands.** That makes a second prefix cheap and safe. The cost is
  that the refusal text and the public constant become plural.
- **Context is passed by reference elsewhere already.** ADR-0008 hands PR
  diffs to subagents through a shared temporary directory, and profile and
  outputter paths are already injected rather than inlined. ADR-0001 and
  ADR-0052 frame the filesystem and CLI as the message bus. This item
  extends that pattern to node context.
- **Snapshot redundancy**: of the fields the item says the snapshot
  records, `lineage`, `level`, `cap`, `path` (relative) and `id` are pure
  functions of the spawn ref and the set root. Only `question` and
  `known_questions` (and the profile, below) are not. The snapshot can stay
  small; the query derives the rest. Recording them anyway is harmless.

## Historical Context

- `meta/work/PP-867-recursive-finding-deepening.md`,
  `meta/research/codebase/2026-09-26-PP-867-recursive-finding-deepening.md`,
  `meta/plans/2026-09-26-PP-867-recursive-finding-deepening.md`: introduced
  the trees, level notes, `known_questions` and node spawns that this item
  reshapes.
- `meta/work/PP-890-fail-web-nodes-on-websearch-quota-exhaustion.md`,
  `meta/plans/2026-10-15-PP-890-budget-web-searches-and-halt-on-exhaustion.md`:
  added `search_budgets` to the ledger and the plan, and `withheld` with
  the halt.
- `meta/plans/2026-10-13-PP-896-recovering-from-an-interrupted-conduct-run.md`:
  the replaced-run report, and the ledger-decoding constraint.
- `meta/plans/2026-10-10-PP-889-research-concurrency-subagent-limit.md`:
  capacity-refusal re-issue within step 4.
- `meta/work/PP-894-researcher-bash-scope-edit-and-grep.md`: also changes the
  guard. It has no plan yet; whichever lands second reconciles the
  allowlist.
- `meta/reviews/work/PP-893-pass-conduct-spawn-context-by-reference-review-1.md`:
  four review passes, ending APPROVE (2026-11-18T20:00).
- `meta/prs/152-description.md`: captured 0307 from GitHub issue #144.
- Decisions: ADR-0001 (context isolation), ADR-0008 (payload by file path
  to subagents), ADR-0045 (skills versus CLI division of labour), ADR-0052
  (filesystem as message bus), ADR-0017 and ADR-0047 (template and config
  extension points).

## Related Research

- `meta/research/codebase/2026-09-26-PP-867-recursive-finding-deepening.md`
- `meta/research/codebase/2026-10-10-PP-889-research-concurrency-subagent-limit.md`
- `meta/research/codebase/2026-10-13-PP-896-recovering-from-an-interrupted-conduct-run.md`
- `meta/research/codebase/2026-10-15-PP-890-budget-web-searches-and-halt-on-exhaustion.md`
- `meta/research/codebase/2026-09-08-PP-861-single-round-web-research-engine.md`

## Open Questions

1. **The profile in the snapshot.**
   - The query must print `search_budget` only for a `web` node, but the
     item's snapshot fields (`question`, `lineage`, `level`, `cap`,
     `known_questions`, `path`) omit the profile.
   - Inferring it from the stem's `-<profile>` suffix is fragile: a slug
     may end in a profile name, and a profile name may be a suffix of
     another.
   - Recommendation: snapshot the pair's profile, which `record` has from
     `window.pairs`.
2. **Snapshot shape.** Either a new top-level ledger key, e.g.
   `offered_nodes: {"<stem>:<lineage>": {…}}`, or `pending.spawns` entries
   grown into objects. The new key keeps `pending.spawns`'s encoding, which
   0310 reads, untouched. It also follows the absent-defaults pattern with
   no dual decoding. Recommendation: the new key, rebuilt in `record` with
   carry-over for spawns that stay pending.
3. **Carry-over rule.** "A node still pending keeps its first snapshot"
   means `record` must look at the *previous* `pending.spawns` and
   snapshot before calling `replace`. Behaviour on `--spawned` matters
   here: acknowledgement empties `pending` first, so an acknowledged batch
   carries nothing over. That is correct by the item's semantics.
4. **Error wording.** The query's errors for "superseded" and "never
   started" both come out of the existing owner check as
   `E_TOPIC_RESEARCH_RUN_SUPERSEDED`. Is that wording acceptable for a run
   id that was never started? The item requires only a non-zero exit.
5. **Node-ref validation.** `--node` given a `Pair` spawn ref, e.g. a
   `single_pass` stem, should be refused up front. A new error code is
   needed, e.g. `E_TOPIC_RESEARCH_NODE`, documented in the exit-code
   table.
6. **The template path mode must cover all three tiers.** It must include
   the `templates.<name>` config path, not only the templates directory
   and the plugin default as the item states. It must print `abs_path`,
   not `display_path`.
7. **Plugin-root symlinks.** The CLI's plugin default path is the
   `cd -P`-resolved root. If `${CLAUDE_PLUGIN_ROOT}` reaches the plugin
   through a symlink, the `Read(/${CLAUDE_PLUGIN_ROOT}/templates/topic-research-*)`
   grant may not match the printed path. Worth checking in the smoke run.
8. **Web profile fallback.** The web profile's "When no search budget is
   injected, make as many calls as needed" must not apply to a node whose
   budget now comes from the query. The rewording must keep a node from
   reading the absent prompt value as unlimited.
9. **Test placement.** Should node-query integration tests live in
   `topic_outstanding.rs`, where the fixture helpers are, or in a new
   `topic_node.rs` after promoting the helpers into
   `tests/topic_project/mod.rs`?
10. **Corrections to the work item's Technical Notes.**
    - `SEARCHING_PROFILE` is in `conduct/search_budget.rs`, not
      `topic_command.rs`.
    - The window has no `held_back` filter: withholding is
      `window()`'s `is_withheld` partition.
    - `withheld` is always written, so only `search_budgets` is
      "written only when set".
