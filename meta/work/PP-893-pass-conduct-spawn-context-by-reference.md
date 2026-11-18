---
type: "work-item"
id: "PP-893"
title: "Pass Conduct Spawn Context by Reference"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "story"
priority: "medium"
parent: "work-item:PP-22"
relates_to: ["work-item:PP-867", "work-item:PP-889", "work-item:PP-890", "work-item:PP-894", "work-item:PP-896"]
tags: ["research", "deep-research", "conduct", "cli"]
last_updated: "2026-11-19T00:19:38+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-893"
aliases: ["0307"]
---

# PP-893: Pass Conduct Spawn Context by Reference

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`conduct`, the `research-topic` verb that researches a round's (focus area,
profile) pairs, has its orchestrator copy each spawn's full context — the
template, the node question, the whole `known_questions` list and a web
node's search budget — into every Task prompt it generates. Agents still
need that context, but having the orchestrator write it costs output tokens,
bloats its own context, and invites transcription errors. Pass references
instead — paths, a slug, a run and a node's spawn ref — and let the agents
load the bulky values themselves. Every researcher fetches its template with
the exact `accelerator config template` command its outputter names. The
composer, which runs no command, still receives the finding template inline;
its prompt does not grow with depth. Only a research-node researcher also
fetches its node context, search budget included, with a read-only CLI
query. Users running
deep `conduct` runs then pay for, and wait on, far fewer orchestrator output
tokens, and hit compaction and the per-response output limit later.

## Context

A `conduct <slug> --depth 3` run over 10 pairs had about 160 spawns (130
researchers, 10 composers, 20 more after an interruption). A level-3 node
prompt is about 5 KB: a 13-question `known_questions` array (~3.5 KB) plus
the inlined template (~1.5 KB). Shallower nodes are smaller, but the run
total was well over half a megabyte.

The agents reading that context is not the problem — input tokens are cheap,
parallel, and live in each agent's throwaway context. The orchestrator
generating it is:

- **Output cost**: ~125–150k output tokens per run, serial, at ~5× input
  pricing.
- **Per-response ceiling**: step 4 of `research-topic`'s `SKILL.md` issues
  a whole batch in one message, so 20 spawns × ~5 KB ≈ 25k output tokens in
  one response, close to the output limit.
- **Accumulation**: every prompt stays in the orchestrator's context,
  bringing compaction forward in a long run.
- **Fidelity**: step 4's "byte for byte" and "verbatim" rules depend on a
  model re-typing JSON the CLI already holds exactly; the run's orchestrator
  resorted to intermediate files.

Per-spawn size grows with depth and spawn count grows with depth × breadth,
so the orchestrator's cost grows faster than either. Passing references
makes it constant per spawn.

Terms used below, from 0283 and 0304: a *pair* is one (focus area,
profile) researched into one finding. A `single_pass` pair is researched
by one researcher. A `research_nodes` pair is researched as a tree of
nodes, each writing a level note, and then a *composer* writes the
pair's finding from those notes. The *research guard* is the hook that
limits which commands a researcher may run. A web pair's per-node *search
budget* differs from the session's web search *quota*: when the quota is
exhausted the run halts and *withholds* the web spawns it has not yet
offered.

A run's state lives in its *run ledger*, which `accelerator research
topic end-run` deletes when the run finishes. Each `outstanding` plan
*offers* a batch of spawns, which the ledger keeps as the *pending batch*
(the current offered batch) until the next plan's `--spawned <batch>`
acknowledges it. A *spawn ref* names one spawn; for a node it is
`<stem>:<lineage>`. The *halting batch* is the offered batch whose spawn
ended with a halting reason; the halt itself offers nothing, so that batch
stays the current offered batch until the next plan.

## Requirements

- **Templates by command**: a researcher's prompt carries no template. Each
  outputter names the exact command that prints its template:
  `accelerator config template topic-research --kind finding` for the
  finding outputter, `--kind level-note` for the level-note outputter. A
  researcher runs it, without `--fail-safe`, before any search or fetch.
  The launcher resolves the template as it does today, so an override
  outside the project needs no `Read` rule and the plugin root stays the
  launcher's own concern. Each outputter grants its exact command in
  frontmatter, and `research-topic`'s `Bash(accelerator config *)` reaches
  the researchers it spawns.
- **Composer stays command-free**: the composer keeps only `Read` and
  `Write`, because the research guard fails open and the composer reads
  every note of a pair. Its prompt carries the finding template inline, as
  printed by `conduct`'s template check.
- **Template check**: before its first plan, `conduct` runs both template
  commands itself and keeps the finding command's fenced block for compose
  prompts. A non-zero exit stops the run before any spawn with "template
  unavailable", and a denial stops it with "template command denied by
  permissions". Any other line from a zero exit is a warning, such as the
  fallback warning for a configured template path that does not exist: it
  is printed before the run and in the summary, and the run proceeds with
  the fallback template.
- **Node context query**: a new read-only command —
  `accelerator research topic node <slug> --run <run> --node <spawn-ref>`,
  name settled in implementation; every mention of the command or its allow
  rule below follows the chosen name — prints one JSON object with exactly
  these keys:
  - `question`, `lineage`, `level`, `cap` and `known_questions`, as
    snapshotted when the node was offered. These are the node terms 0283
    defines: `lineage` is the node's position in its pair's tree (for
    example `2-1`), `level` its depth, `cap` the most follow-up questions
    its note may record, and `known_questions` the questions already asked
    elsewhere in the tree that its follow-ups must not repeat;
  - `path`, the absolute path of the node's level note;
  - `id`, the level note's frontmatter id, `<slug>.<stem>.<lineage>` (for
    example `my-topic.03-a-web.2-1`), computed from the slug and spawn ref,
    not stored in the snapshot;
  - `search_budget`, for a node of a `web` pair only (see **Search budget
    by query**); the key is omitted for any other profile.
- **Node argument**: `--node` takes the node's spawn ref,
  `<stem>:<lineage>` (for example `03-a-web:2-1`), the same text the ledger
  already records for an offered node. The spawn ref is the input; `id` is
  the note id the query prints, and the two are distinct.
- **Query errors**: given an unknown slug, a slug with no run ledger
  (including one whose run `end-run` has ended), a run that is not the
  slug's current run (superseded or never started), or a spawn ref not in
  the run's current offered batch, the query exits non-zero, prints an
  error on stderr and prints nothing on stdout.
- **Offered-node snapshot**: an offer is any `outstanding` plan that sets
  the run's pending batch, including a repeat plan before `--spawned`
  acknowledges the batch. On each offer, the run ledger records each
  offered node's `question`, `lineage`, `level`, `cap`, `known_questions`
  and `path` alongside its spawn ref. A node still pending from the
  previous offer keeps the context snapshotted when it was first offered;
  each entry also records the pair's source profile, which the query
  needs to decide whether to print `search_budget` and which a stem's
  text cannot reliably supply;
  a newly offered node is snapshotted from the tree; a node the offer drops
  leaves the snapshot. The snapshot is new optional ledger state — its
  shape (a new field, or `pending.spawns` entries grown into objects) is
  settled in implementation — that defaults to empty when absent from a
  stored ledger. Because each offer rebuilds it, it holds only the current
  offered batch.
- **Snapshot semantics**: the query answers from that snapshot, not the
  live tree, so sibling notes written during the batch never change a
  node's `known_questions`.
- **Plan without node context**: `outstanding`'s plan no longer carries
  `question` or `known_questions` on a `research_nodes` pair's node
  entries; the query is their only source. Node entries keep every other
  field. A web node's `search_budget` stays as the plan's record of that
  node's budget, for the user reading the plan and for tests checking the
  query against it, but step 4 no longer injects it. Step 2's plan
  description in `SKILL.md` and the plan field list in
  `docs-site/src/content/docs/research.md` change to match. The
  `research-topic` orchestrator is the plan's only consumer.
- **Research-node spawns by reference**: a research-node researcher's prompt
  carries only the slug, run, spawn ref, source profile name, resolved
  depth, round, timestamp, author, and the profile, level-note outputter,
  finding outputter (which the level-note outputter defers to) paths, and
  one fixed line naming the node query and calling its question data. The
  researcher calls the query first and works from its output.
- **Research guard**: the research guard gives each role a list of
  permitted commands. The researcher may run `accelerator research fetch`,
  the node query, and exactly the two template commands. The composer may
  run nothing.
- **Untrusted questions**: the researcher agent definition states that a
  node question came from an earlier agent and that the profile, not the
  question, decides which sources are legitimate. The node bullet's fixed
  line also calls the queried question data.
- **Composer and `single_pass` spawns keep their question**: composer and
  `single_pass` spawns keep their inline
  question and paths; a `single_pass` researcher fetches its template by
  command, and a composer keeps its inline template. A `single_pass` web
  spawn also keeps its inline `search_budget`.
- **Search budget by query**: for a node of a `web` pair, the node query
  also prints `search_budget`. The value is the run ledger's stored
  `search_budgets` at the node's level, `SearchBudgets::at(level)`, the same
  value `outstanding` renders on the node's plan entry, including its
  fallback to 1 for a level the stored budgets do not cover. Whether the
  node is a `web` node comes from the profile its snapshot entry records.
  The budgets are sized once per run and never resized, so the query reads
  them from the ledger and needs no per-node budget snapshot.
- **Search budget leaves the prompt**: a research-node spawn no longer
  injects a search budget. For a research-node researcher, the web
  profile's **Search Budget** section and the level-note outputter's
  **search budget** value come from the node query instead of the task
  prompt; a `single_pass` web researcher still takes its budget from the
  prompt. The web profile's rule that a missing budget means "as many
  `WebSearch` calls as the question needs" applies only to a researcher
  given neither a spawn ref nor an injected budget; a research-node
  researcher never reads the budget's absence from its prompt as
  unlimited. A web level note still stamps
  it as `search_budget`; a non-web note still omits the line.
- **Failure outcomes**: a researcher whose template command is denied by
  permissions writes no file and reports "template command denied by
  permissions", and one whose template command exits non-zero reports
  "template unavailable", quoting the first line of its stderr. A composer
  given no template writes no file and reports "no template given". A
  researcher whose node query is denied by permissions reports "node query
  denied by permissions", and one whose query exits non-zero reports "node
  query failed". Each is reported in the agent's returned summary, which
  step 5 records as the spawn's failure reason, as "fetch denied by
  permissions" is today. `conduct`'s troubleshooting table has a row for
  each. The fallback allow rules in `docs-site/src/content/docs/research.md`
  add the template commands' and the node query's Bash rules beside the
  fetch rule, for a skill whose grant does not reach the spawn.

## Acceptance Criteria

Criteria about spawn prompts and agent behaviour are verified against the
files that produce them — step 4's spawn bullets in
`skills/research/research-topic/SKILL.md`, the researcher and composer
agent definitions, and the profile and outputter text — in structure tests
alongside `tests/unit/tasks/test_research_structure.py`, not against live
runs. A structure test reads a spawn bullet's injected fields from its
list of injected items, one field per item, and compares that list with the
required set. Node-context criteria use a fixture subject whose notes fix
each offered node's `question`, `cap` and `known_questions` in advance,
with stored search budgets of 8, 5 and 3 for levels 1, 2 and 3, so the
expected values are literals in the test, never read back from the plan or
the query under test. The CLI sizes budgets from the session search cap,
so the fixture writes 8, 5 and 3 into the run ledger's `search_budgets`
itself rather than choosing a cap that happens to yield them. In the
fixture, pair `03-a-web`'s level-1 note
records follow-ups Q1, Q2 and Q3, so the first plan offers its level-2
nodes `2-1`, `2-2` and `2-3`; node `2-1`'s note, written mid-batch, records
the follow-up Q4.

- [ ] Given that fixture's offered web node `03-a-web:2-1`, when the node
      query runs with the slug, run and that spawn ref, then it prints
      exactly the eight keys `question`, `lineage`, `level`, `cap`,
      `known_questions`, `id`, `path` and `search_budget` and no others,
      with the fixture's values, `lineage` `2-1`, `level` 2, `id`
      `<slug>.03-a-web.2-1`, the absolute path of that node's level note
      and `search_budget` 5.
- [ ] Given that fixture's offered `openalex` node `04-b-openalex:2-1`,
      when the node query runs, then it prints exactly the seven keys
      above without `search_budget`, with the fixture's values.
- [ ] Given a `research_nodes` pair, when `outstanding` prints its plan,
      then each node entry carries exactly `lineage`, `level`, `cap`, `id`,
      `path` and `spawn`, plus `rejected` when its note on disk was refused
      and `search_budget` for a web node.
- [ ] Given step 2's plan description in `SKILL.md` and the plan field list
      in `docs-site/src/content/docs/research.md`, then neither lists
      `question` or `known_questions` on a research-node entry, and both
      still list `search_budget` for a web node.
- [ ] Given that fixture's first batch offered, when `2-1` writes its note
      and the query then runs for `03-a-web:2-2`, then its
      `known_questions` equals the fixture's literal list, without Q4.
- [ ] Given `2-1`'s note written, when `outstanding` plans again before
      `--spawned`, then `2-2` and `2-3`, still pending, keep their
      first-offered `known_questions` without Q4, and `2-1`, which the
      repeat plan drops, makes the query exit non-zero.
- [ ] Given every level-2 note of `03-a-web` written and the batch
      acknowledged with `--spawned`, when the next offer includes `2-1`'s
      first child `03-a-web:3-1-1`, then the query for it answers with
      `known_questions` that include Q4. A level is derived only once every
      node above it has its note, so no level-3 node is offered while a
      level-2 sibling is pending.
- [ ] Given each of these five cases in turn — an unknown slug, a slug with
      no run ledger, a superseded run, a run id never started for the
      slug, and a spawn ref not in the run's current offered batch — when
      the node query runs, then it exits non-zero, prints an error on
      stderr and prints nothing on stdout.
- [ ] Given two successive offers in a run, when the node query runs for a
      spawn ref offered only in the first, then it exits non-zero, while it
      answers for every node of the second batch.
- [ ] Given a stored ledger written without the snapshot, when it is read,
      then it decodes with an empty snapshot and its other fields unchanged.
- [ ] Given an offered batch not yet acknowledged with `--spawned`, when a
      spawn re-issued after a capacity refusal runs the node query, then it
      answers from the snapshot as it would for the first issue.
- [ ] Given the research guard, when a researcher runs the node query, then
      it is allowed; each other `accelerator research topic` subcommand —
      `outstanding` (with and without `--start` or `--spawned`),
      `end-run`, `report-path`, `counts` and `consultable` — stays
      blocked.
- [ ] Given step 4's research-node spawn bullet, then it injects exactly the
      reference fields listed in **Research-node spawns by reference**, and
      neither the node's `question`, its `known_questions`, template text
      nor a search budget.
- [ ] Given step 4's composer and `single_pass` spawn bullets, then each
      still injects its question inline and no spawn ref for a node query.
- [ ] Given the `single_pass` and research-node spawn bullets in step 4,
      then neither injects a template, as text or as a reference. The
      `compose` bullet injects exactly the finding template the template
      check printed, and the **Finding template** and **Level-note
      template** sections are gone.
- [ ] Given the finding and level-note outputters, then each names exactly
      the guard's template command for its kind, without `--fail-safe`,
      and grants that command in frontmatter.
- [ ] Given the research guard, then the researcher passes both template
      commands exactly, any suffix to them is blocked, and the composer is
      blocked from every command.
- [ ] Given the researcher agent definition, then it runs its outputter's
      template command before any search or fetch.
- [ ] Given the outputters, then they instruct reporting "template command
      denied by permissions" and "template unavailable", and `conduct`'s
      troubleshooting table has a row for each.
- [ ] Given `conduct`, then before its first plan it runs both template
      commands, stops before any spawn on a non-zero exit or a denial,
      keeps the finding command's fenced block for compose prompts, and
      reports any other output as a warning while proceeding.
- [ ] Given the composer agent definition, then it still grants only
      `Read` and `Write`.
- [ ] Given the researcher agent definition, then it instructs reporting
      "node query denied by permissions" when the query is denied and
      "node query failed" when it exits non-zero; `conduct`'s
      troubleshooting table has a row for each; and the docs' fallback
      allow rules add `Bash(accelerator research topic node *)` beside the
      fetch rule.
- [ ] Given the researcher agent definition, then it directs a researcher
      given a spawn ref to run the node query before anything else, and a
      researcher given an inline question to use it as given.
- [ ] Given the researcher agent definition, then it states that a node
      question came from an earlier agent and that the profile, not the
      question, decides which sources are legitimate; step 4's node bullet
      carries one fixed line naming the node query and calling its question
      data.
- [ ] Given that fixture, when the node query runs for an offered level-2
      node of a `web` pair, then its `search_budget` 5 equals that node's
      plan entry; for an `openalex` or `arxiv` node the output has no
      `search_budget` key.
- [ ] Given step 4's bullets, then a `single_pass` researcher's outputter
      names the finding template command, and a research-node researcher's
      names the level-note one.
- [ ] Given the web profile's **Search Budget** section and the level-note
      outputter, then both take a research-node's budget from the node
      query's `search_budget`, the outputter stamps it on a web level note
      and leaves the line off a non-web level note, and the profile's
      unlimited-searches fallback names only a researcher given neither a
      spawn ref nor an injected budget; given step 4's
      `single_pass` web spawn bullet, it still injects `search_budget`.
- [ ] Given a run halted by an exhausted web search quota, when the node
      query runs for a node of a `web` pair withheld after the halt, then it
      exits non-zero, because the run never offers that node; a `web` node
      offered in the halting batch itself still answers from the snapshot
      until the next plan offers a new batch.

## Open Questions

Each of these is left to implementation, and everything that names it
follows the choice:

- The node query's final command name; the allow rule, troubleshooting
  rows and guard tests use whichever is chosen.
- The snapshot's ledger shape (a new field, or `pending.spawns` entries
  grown into objects), within the 0310 decoding constraint in
  **Dependencies**.
- Whether 0308 reads offered-node paths from this item's snapshot, decided
  when 0308 is planned; that decision settles the conditional **Blocks**
  entry and is recorded in both items' **Dependencies**.

## Dependencies

- Blocked by: none known
- Blocks: the release of 0121 (see **Release timing**); and 0308, only if
  0308 reads offered-node paths from this item's snapshot. If 0308 derives
  them itself, neither blocks the other.
- Release timing: this item must land before the first release that ships
  `research-topic`'s `conduct`, which is the release of the parent epic
  0121 (in progress), so 0121 must not close before this item. The
  Assumptions accept breaking custom `agents.researcher` definitions only
  while the feature is unreleased. Landing later would need migration
  guidance for custom researchers.
- Related: 0283, done, which introduced research-node trees, level notes,
  `known_questions` and the per-node spawns this item changes. Tests from
  it and its successors that pin today's inlined context and change with
  this item include `test_research_topic_loads_the_level_note_template`,
  `test_the_researcher_denies_a_focus_question_licensing_a_source` and
  `test_conduct_reads_the_search_budget_fields` (its node field list) in
  `tests/unit/tasks/test_research_structure.py`, and the node-context
  assertions in `cli/research-cli/tests/topic_outstanding.rs`.
- Related: 0304, landed, which gives each web node a search budget and
  stores the run's budgets in the ledger. It landed first, so this item
  owns carrying the budget through the node query; see **Search budget by
  query**. 0304 also added two optional ledger keys, `search_budgets` and
  `withheld`, each defaulting when absent from a stored ledger;
  `search_budgets` is also written only when set, while `withheld` is
  always written, empty or not. This item's snapshot follows the
  default-when-absent precedent. 0304's tests that pin
  today's inline budget and change with this item:
  `test_web_spawns_carry_their_search_budget` (its node assertion only),
  `test_the_web_profile_limits_searches_to_the_injected_budget` and
  `test_only_web_level_notes_stamp_a_search_budget` in
  `tests/unit/tasks/test_research_structure.py`, and
  `a_written_ledger_names_its_fields_after_the_memory_it_keeps` in
  `cli/research-adapters/src/conduct/ledger.rs`, whose key list grows with
  any new ledger field.
- Related: 0308, draft, which also changes what the research guard lets
  researchers run and confines researcher writes to paths in the active
  run's offered batch. Whichever lands second reconciles the guard's
  allowlist and its tests with the other's; 0308 may read the offered
  nodes' paths from this item's snapshot.
- Related: 0303, done, which re-issues capacity-refused spawns of an
  offered batch in follow-up messages within step 4. A re-issued spawn
  needs no new behaviour: the batch stays the current offered batch until
  the next plan's `--spawned`, so the query answers for it unchanged. This
  item covers that with an acceptance criterion, and the per-response
  output ceiling described in **Context** applies to each follow-up
  message as well as the first.
- Related: 0310, done, which reads a replaced ledger's `run`, `pending`
  spawn refs and `claims` at `outstanding --start` to report an
  interrupted run. It needs no ledger change, but the ledger decodes only
  when every field it expects is present in its current encoding. So this
  item must keep existing fields' encoding (or keep decoding the old one,
  for example if `pending.spawns` entries grow into objects), and its
  snapshot must default when absent from a stored ledger; otherwise a
  ledger written before this item lands decodes as corrupt and yields no
  report.

## Assumptions

- Researchers reliably call a CLI query before starting work when their
  agent definition instructs it, and work from its output. The acceptance
  criteria check only the instruction; that agents follow it is an
  accepted risk, checked by the manual smoke run in **Drafting Notes**
  rather than by an automated test. A researcher that cannot run it reports
  "node query denied by permissions" or "node query failed" rather than
  the generic "wrote no note", whose re-run advice would loop.
- Breaking existing custom `agents.researcher` definitions is acceptable:
  the feature is unreleased, and a custom researcher must add the node
  query instruction itself. Profiles and outputters ship with the plugin
  and cannot be overridden the way `agents.researcher` can, so their
  change to reading the budget from the query breaks no user
  customisation.
- A spawn's permission denial for its template command or the node query
  fails each spawn of the batch, as "fetch denied by permissions" does,
  rather than halting the run; template faults are caught once, before the
  first plan.
- The ledger growth from the snapshot is acceptable: it holds one batch of
  node context (roughly 20 × 3.5 KB at depth 3) and is replaced on each
  offer, so it never accumulates across a run.

## Technical Notes

- Spawn injection list: step 4's three spawn bullets in
  `skills/research/research-topic/SKILL.md`.
- The node as rendered today: `node` and `PlanOutput::search_budget` in
  `cli/research-cli/src/topic_command.rs`; the budget is only offered for a
  pair whose profile `searches`, that is `SEARCHING_PROFILE` in
  `cli/research/src/conduct/search_budget.rs`, and the note id comes from
  `NoteRef::id` in `cli/research/src/topic/layout/note_ref.rs`, with the
  set slug taken from the resolved set directory's name.
- Withholding: `RunMemory::withheld` and the `is_withheld` partition in
  `window()` in `cli/research/src/conduct/window.rs`; a withheld spawn
  goes to the window's `withheld_spawns`, never its offered batch.
- Snapshot capture: `RunLedger::record` in
  `cli/research/src/conduct/ledger.rs` already receives the window, whose
  narrowed `pairs` hold each offered node's `MissingNode` and its pair's
  profile. It replaces the pending batch through `PendingBatch::replace`,
  so carrying over a still-pending node's snapshot means consulting the
  previous batch before the replace. `--spawned` empties the batch first,
  so an acknowledged batch carries nothing over.
- Ledger encoding is hand-written JSON in
  `cli/research-adapters/src/conduct/ledger.rs` (`ledger_to` and
  `ledger_from`), not serde; an optional key defaults through `map_or` when
  absent and decodes as corrupt when malformed.
- Query model: `consultable` (`consultable_command.rs`) is the closest
  read-only subcommand. The query's ledger errors can reuse
  `no_usable_ledger` and `superseded` in `topic_command.rs`, whose owner
  check reports a never-started run id as superseded; `SpawnRef::parse`
  already parses the spawn ref.
- Guard: `cli/research/src/confinement.rs` permits a single exact prefix,
  `PERMITTED_PREFIX`, then lexes the rest; other topic subcommands are
  blocked only by lacking it. A second exact prefix,
  `accelerator research topic node ` with its trailing space, keeps them
  blocked. The constant is in `cli/research/tests/fixtures/public-api.txt`,
  and the `E_RESEARCH_GUARD_COMMAND` refusal names it.
- Plan production: `cli/research/src/topic/plan.rs`; run ledger:
  `cli/research/src/conduct/ledger.rs`.
- The ledger keeps too little to answer the query today: `PendingBatch` in
  `cli/research/src/conduct/memory.rs` holds only a batch `number` and a
  `Vec<SpawnRef>`, and a node's `SpawnRef`
  (`cli/research/src/conduct/spawn.rs`) is just `<stem>:<lineage>`. The
  node context comes from `cli/research/src/topic/tree.rs`, which computes
  `known_questions` from the live tree, so it must be captured at offer
  time.
- The pending batch is acknowledged only by the next plan's
  `--spawned {batch}`, after the batch's researchers have run, so it is
  still the current offered batch while they query it.
- Templates the orchestrator inlines today: the **Finding template** and
  **Level-note template** sections of `skills/research/research-topic/SKILL.md`.
- Those sections come from `accelerator config template topic-research
  --kind finding|level-note`, which resolves overrides and prints fenced
  content (`FileConfigStore::resolve_template` in
  `cli/config-adapters/src/store.rs`). A configured path that does not
  exist falls back with a stderr warning and exits 0. Templates by path
  were considered and rejected: the plugin default's path is the launcher's
  symlink-resolved root, which a `Read(/${CLAUDE_PLUGIN_ROOT}/…)` grant may
  not match, and an override outside the project would need its own `Read`
  rule.
- Research for this item:
  `meta/research/codebase/2026-11-18-PP-893-pass-conduct-spawn-context-by-reference.md`.
- Spawns inherit the spawning skill's grants
  (`docs-site/src/content/docs/research.md:504-506`), so `research-topic`'s
  `Bash(accelerator config *)` and `Bash(accelerator research topic *)`
  reach its researchers.
- The researcher agent is shared by `single_pass` and research-node spawns,
  so its definition covers both: a spawn ref means query first, an inline
  question means use it as given. The untrusted-question statement moves
  there from step 4's per-spawn prose.
- Dropping node `question` and `known_questions` from the plan touches the
  plan description in step 2 of `SKILL.md`, the plan field list in
  `docs-site/src/content/docs/research.md` and
  `cli/research-cli/tests/topic_outstanding.rs`.
- Node-query permission failures parallel the profiles' "fetch denied by
  permissions" outcome and its troubleshooting row in `SKILL.md`; the
  fallback allow-rule guidance lives in
  `docs-site/src/content/docs/research.md`.

## Drafting Notes

- The issue's suggested shapes (CLI-rendered prompts, prompt files) were
  rejected by the user in favour of references plus agent-side retrieval.
- Whole-node-context retrieval and keeping composers inline were user
  decisions.
- `single_pass` pairs are treated like composers because they carry no
  `known_questions` and their prompts are small.
- The query has no `--path` argument: the spawn ref already names both the
  pair and the lineage, so a second argument would not rule out any mix-up
  the ref allows. This is distinct from the `path` key the query prints.
- Template-by-path and the search budget's move stay in this item by
  choice, though either could ship alone. A research-node prompt carrying
  only references is the invariant this item establishes, and leaving the
  budget inline would break it; moving the template out of researcher
  prompts is the same move, and the template is a third of a level-3 node
  prompt.
- The item is kept whole by choice. Its natural split — the snapshot and
  read-only query first, invisible to prompts, then switching spawns to
  references — was considered; the first half delivers no saving on its
  own, so the two would ship together anyway. A split along the template
  line — templates by command first, node context second — would ship a
  saving each time, but both halves rewrite the same step 4 bullets, agent
  definitions and failure-outcome rows, so it was rejected to avoid editing
  them twice. The `single_pass` template move stays here for the same
  reason.
- The guard's per-role command list is the one piece that splits cleanly,
  touching none of the shared files; it may land first. Planning should
  sequence the rest as internal phases: snapshot and query; then the spawn,
  agent, profile and outputter switch-over; then docs and troubleshooting.
- A manual smoke run is worth doing before closing, but is not an
  acceptance criterion: `conduct` at depth 2 over one web pair passes when
  every research-node Task prompt holds no `question`, `known_questions`,
  template text or search budget, and only compose prompts hold template
  text; every level note's frontmatter `id` and
  path match the node query's `id` and `path`; and no level note's
  follow-ups repeat a question in its queried `known_questions`.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/144
- Related: 0121, 0283, 0303, 0304, 0308, 0310
