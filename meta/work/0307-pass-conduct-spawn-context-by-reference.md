---
type: "work-item"
id: "0307"
title: "Pass Conduct Spawn Context by Reference"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0121"
relates_to: ["work-item:0283", "work-item:0303", "work-item:0304", "work-item:0308", "work-item:0310"]
tags: ["research", "deep-research", "conduct", "cli"]
last_updated: "2026-10-17T16:30:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0307: Pass Conduct Spawn Context by Reference

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`conduct`'s orchestrator copies each spawn's full context — the template,
the node question and the whole `known_questions` list — into every Task
prompt it generates. Agents still need that context, but having the
orchestrator write it costs output tokens, bloats its own context, and
invites transcription errors. Pass references instead — paths, a slug, a run
and a node id — and let each researcher load the bulky values itself:
static files with `Read`, its node context with a read-only CLI query.

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
- **Per-response ceiling**: step 4 issues a whole batch in one message, so
  20 spawns × ~5 KB ≈ 25k output tokens in one response, close to the output
  limit.
- **Accumulation**: every prompt stays in the orchestrator's context,
  bringing compaction forward in a long run.
- **Fidelity**: step 4's "byte for byte" and "verbatim" rules depend on a
  model re-typing JSON the CLI already holds exactly; the run's orchestrator
  resorted to intermediate files.

Per-spawn size grows with depth and spawn count grows with depth × breadth,
so the orchestrator's cost grows faster than either. Passing references
makes it constant per spawn.

## Requirements

- **Template by path**: every spawn (researcher and composer) receives the
  template's path instead of its inlined text, as it already does for the
  profile and outputter.
- **Node context query**: a new read-only command —
  `accelerator research topic node <slug> --run <run> --node <id>`, name
  settled in implementation — prints a node's `question`, `lineage`,
  `level`, `cap`, `known_questions` and `path` as JSON.
- **Snapshot semantics**: the query answers from the run ledger's record of
  the offered batch, not the live tree, so sibling notes written during the
  batch never change a node's `known_questions`.
- **Research-node spawns by reference**: a research-node researcher's prompt
  carries only the slug, run, node id, source profile name, resolved depth,
  round, timestamp, author, and the profile, outputter and template paths.
  The researcher calls the query first and works from its output.
- **Research guard**: the research guard permits the node query alongside
  `accelerator research fetch`.
- **Untrusted questions**: the researcher agent definition, not the
  per-spawn prompt, states that a node question came from an earlier agent
  and that the profile, not the question, decides which sources are
  legitimate.
- **Unchanged spawns**: composer and `single_pass` spawns keep their inline
  question and paths; only their template moves to a path. A `single_pass`
  web spawn also keeps its inline `search_budget`.
- **Search budget by query**: for a node of a `web` pair, the node query
  also prints `search_budget`. The value is the run ledger's stored
  `search_budgets` at the node's level, `SearchBudgets::at(level)`, the same
  value `outstanding` renders on the node today. A node of any other profile
  prints none. The budgets are sized once per run and never resized, so the
  query reads them from the ledger and needs no per-node budget snapshot.
- **Search budget leaves the prompt**: a research-node spawn no longer
  injects a search budget. The web profile's **Search Budget** section and
  the level-note outputter's **search budget** injected value read it from
  the node query instead of the task prompt. A web level note still stamps
  it as `search_budget`; a non-web note still omits the line.

## Acceptance Criteria

- [ ] Given an offered research node, when the node query runs with its
      slug, run and id, then it prints `question`, `lineage`, `level`,
      `cap`, `known_questions` and `path` byte-identical to that node's
      entry in the `outstanding` plan.
- [ ] Given a batch in which a sibling node writes its note before the query
      runs, when the query runs for another node in the batch, then its
      `known_questions` equals the plan as offered.
- [ ] Given a node id not in the run's current offered batch, or a
      superseded run, when the node query runs, then it exits non-zero with
      an error code and prints nothing on stdout.
- [ ] Given the research guard, when a researcher runs the node query, then
      it is allowed; any other `accelerator research topic` subcommand stays
      blocked.
- [ ] Given a depth-3 `conduct` run, the orchestrator's prompt text for each
      research-node spawn is the same size regardless of the node's level or
      `known_questions` length.
- [ ] Given any spawn, its prompt contains a template path and no inlined
      template text.
- [ ] Given a run with stored search budgets, when the node query runs for
      an offered node of a `web` pair, then it prints `search_budget` equal
      to that node's `search_budget` in the `outstanding` plan; for an
      `openalex` or `arxiv` node it prints no `search_budget`.
- [ ] Given a research-node spawn, its prompt carries no search budget, and
      a web researcher stamps the node query's `search_budget` on its level
      note; given a `single_pass` web spawn, its prompt still carries its
      `search_budget`.
- [ ] Given a run halted by an exhausted web search quota, when the node
      query runs for a node of a `web` pair, then it exits non-zero, because
      the run withholds `web` researcher spawns and never offers the node.

## Open Questions

- Does the ledger already retain enough per offered node (notably
  `known_questions`) to answer the query, or must it record more at offer
  time?
- Should the query also accept the node's `path` as its key, to rule out id
  mix-ups?

## Dependencies

- Blocked by: none known
- Blocks: none known
- Related: 0304, landed, which gives each web node a search budget and
  stores the run's budgets in the ledger. It landed first, so this item
  owns carrying the budget through the node query; see **Search budget by
  query**. 0304 also added two optional ledger keys, `search_budgets` and
  `withheld`, each written only when set and defaulting when absent. This
  item's snapshot fields follow the same precedent. 0304's tests that pin
  today's inline budget and change with this item:
  `test_web_spawns_carry_their_search_budget` (its node assertion only),
  `test_the_web_profile_limits_searches_to_the_injected_budget` and
  `test_only_web_level_notes_stamp_a_search_budget` in
  `tests/unit/tasks/test_research_structure.py`, and
  `a_written_ledger_names_its_fields_after_the_memory_it_keeps` in
  `cli/research-adapters/src/conduct/ledger.rs`, whose key list grows with
  any new ledger field.
- Related: 0308, which also changes what the research guard lets
  researchers run.
- Related: 0303, which re-issues capacity-refused spawns of an offered
  batch in follow-up messages within step 4. This item owns making the
  "current offered batch" node query answer for those re-issued spawns,
  and its per-response ceiling applies to each follow-up message.
- Related: 0310, which reads a replaced ledger's `run`, `pending` spawn refs
  and `claims` at `outstanding --start` to report an interrupted run. It needs no ledger
  change, but the ledger decodes only when every field it expects is
  present in its current encoding. So this item must keep existing fields'
  encoding (or keep decoding the old one, for example if `pending.spawns`
  entries grow into objects), and its snapshot fields must default when
  absent from a stored ledger; otherwise a ledger written before this item
  lands decodes as corrupt and yields no report.

## Assumptions

- Researchers reliably call a CLI query before starting work when their
  agent definition instructs it. A researcher that skips it has no question
  to research, so the failure is visible rather than silent.

## Technical Notes

- Spawn injection list: step 4's three spawn bullets in
  `skills/research/research-topic/SKILL.md`.
- The node's budget as rendered today: `PlanOutput::search_budget` in
  `cli/research-cli/src/topic_command.rs`, which only offers a budget for a
  pair whose profile is `SEARCHING_PROFILE`.
- Withholding: `RunMemory::withheld` and the window's `held_back` filter in
  `cli/research/src/conduct/`; a withheld spawn is never in the offered
  batch.
- Plan production: `cli/research/src/topic/plan.rs`; run ledger:
  `cli/research/src/conduct/ledger.rs`.
- Templates the orchestrator inlines today: the **Finding template** and
  **Level-note template** sections of `skills/research/research-topic/SKILL.md`.

## Drafting Notes

- The issue's suggested shapes (CLI-rendered prompts, prompt files) were
  rejected by the user in favour of references plus agent-side retrieval.
- Whole-node-context retrieval and keeping composers inline were user
  decisions.
- `single_pass` pairs are treated like composers because they carry no
  `known_questions` and their prompts are small.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/144
- Related: 0121, 0283, 0303, 0308
