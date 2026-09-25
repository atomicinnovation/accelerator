---
type: "work-item"
id: "0283"
title: "Recursive Finding Deepening"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
tags: ["research", "skills", "deep-research"]
last_updated: "2026-09-25T08:16:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-867"
---

# 0283: Recursive Finding Deepening

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As an Accelerator user researching a subject, I want a finding to drill into
its own follow-up questions to a depth I choose, so that one focus area yields
a deeper answer without me commissioning each sub-question as a new round.

At `depth > 1`, `conduct` researches each (focus area, profile) pair as a tree:
every researcher writes a level note recording up to `n` follow-up questions,
`n` starting at 4 and halving per descent (the dzhng halving); `conduct` spawns
the next level from the follow-ups recorded on disk until the configured depth,
then a composer writes the pair's single finding from its notes. A concurrency
cap bounds how many agents `conduct` spawns at once. This is the epic's
acknowledged descope candidate — droppable if the epic runs long, keeping the
knob (0282).

## Context

`depth > 1` multiplies spend, so it lands only after the output-quality gate
(0280) validates the premise and after the knob (0282) makes the ceiling
affordable. A finding's recursion is governed by `depth` and does not consume
the round's `breadth` budget, which continues to bound focus areas only.

Researchers cannot spawn subagents — `agents/researcher.md` carries no Task
tool, and Claude Code subagents cannot nest — so the recursion cannot live
inside a researcher. `conduct` orchestrates each level instead, and the
filesystem carries the research between levels, so `conduct` routes questions
and paths but never holds research content.

## Requirements

- **Level orchestration.** At `depth > 1`, `conduct` spawns each pair's level-1
  researcher, then level by level until `depth`, one researcher per missing
  node that `corpus topic-research outstanding` derives from the notes on disk.
  `conduct` re-runs `outstanding` after each level; fresh runs and resumes take
  the same path.
- **Follow-up cap.** A node's cap is 4 at level 1 and `ceil(parent cap / 2)`
  below it. `conduct` injects the cap at spawn, and the researcher records at
  most that many `follow_ups`, most valuable first. When a note records more,
  `outstanding` takes the first `cap` entries and warns; it never rejects the
  note.
- **Follow-ups at every level.** Every node, including those at the depth
  limit, records its capped follow-up questions; `outstanding` derives children
  only from notes whose `level` is below the resolved `depth`. `follow_ups: []`
  therefore always means the node judged its question answered (pruning), never
  that it hit the limit.
- **Duplicate avoidance.** `conduct` injects into each researcher the questions
  its tree already covers — the pair question, its ancestors' questions, and
  the follow-ups recorded so far — as questions not to propose again.
  `outstanding` skips a follow-up whose normalised question equals the pair
  question or any node already in the tree. A skipped follow-up keeps its
  position, so lineages stay stable.
- **Content-free orchestration.** The notes on disk are the single source of
  truth for follow-ups. A researcher returns only a short summary, never its
  follow-ups or note body.
- **Concurrency.** `conduct` spawns every batch — researchers at any depth,
  including `depth: 1`, and composers — at most `concurrency` at a time,
  resolved flag (`conduct --concurrency N`) > config
  (`research.topic.concurrency`) > default `24`. `--concurrency` on `outline`
  is a misplaced flag.
- **Level notes.** Each node writes one note to
  `findings/<finding-stem>.levels/<lineage>.md`, where lineage encodes the
  parent chain (`1`, `2-k`, `3-k-j`, …), each index being the 1-based position
  of the question in its parent's capped `follow_ups`. Notes are written through
  a new level-note outputter and template, carry frontmatter
  `kind: level-note`, `question`, `source_profile`, `round`, `level`, `depth`
  (the resolved depth the node ran under), and `follow_ups`, and are registered
  under the umbrella `topic-research` type. Sources are tier-tagged as in
  findings. Notes are kept after composition.
- **Composer.** Once every expected node has a validated note, `conduct` spawns
  a dedicated `composer` agent (tools `Read, Write` only; configurable through
  `accelerator config agent composer`) that reads only that pair's notes within
  the resolved depth and writes the pair's finding at its existing path through
  `finding-outputter` — standalone prose answering the pair question, not a
  level-by-level walk — stamped with the resolved `depth`. Lacking fetch
  tools, it can cite only sources already in the notes.
- **`depth: 1` unchanged in shape.** A pair with no `.levels/` directory, at
  `depth: 1`, gets one researcher that writes the finding directly; no
  `.levels/` directory and no composer. A finding without `depth` reads as
  `depth: 1`.
- **Profile confinement.** Every node and the composer of a pair run with that
  pair's injected profile, and the finding keeps its `source_profile`.
- **No human between levels**, and a finding's recursion does not consume the
  round's `breadth` budget.
- **Write guard.** Researcher subagents may additionally write
  `<set>/findings/<nn>-<name>.levels/<lineage>.md`, still exactly one injected
  path per spawn; every other path under `.levels/` is rejected. The `composer`
  agent may write only the pair's top-level finding path, never under
  `.levels/`.
- **Resume.** A pair stays outstanding until its finding validates.
  `corpus topic-research outstanding` takes the resolved depth (`--depth N`)
  and reports, per partially researched pair, its missing nodes — the capped,
  deduplicated `follow_ups` of notes whose `level` is below that depth, minus
  validated notes — and pairs needing composition only. A re-run spawns only
  the missing nodes, then composes. An invalid note is quarantined as
  `.<lineage>.md.invalid` and treated as missing.
- **Index retention.** A `.levels/` directory holds its `<nn>` index as a
  quarantine marker does, keyed on the `question` of its `1.md` or
  `.1.md.invalid`, so a resumed pair is allocated the same finding stem and a
  new focus area never takes that index.
- **Depth changes on resume.** A re-run at a smaller `--depth` composes from the
  notes whose `level` is within that depth and stamps it — at `--depth 1`,
  from `1.md` alone, spawning no researcher; a re-run at a larger `--depth`
  extends the tree from the recorded follow-ups of notes at the old limit. A
  pair whose finding is composed is complete and is never re-deepened.
- **Counting.** `finding_count` and `round_count` count only the top-level
  `<nn>-*.md` files directly in `findings/`; `synthesise` reads only those.
- **Surface.** Remove the dormant-depth notice from `conduct` and the "no effect
  until 0283" caveat from `configure help`, and document the engine, the
  `research.topic.concurrency` key and its default, and the `composer` agent
  key. The step 8
  summary names each failed node by lineage with its reason and next step.

## Acceptance Criteria

- [ ] Given `depth: 1`, when `conduct` runs, then each pair gets exactly one
      researcher that writes the finding directly, and no `.levels/` directory
      is created.
- [ ] Given a pair at `depth: 2` whose level-1 researcher returns 4 follow-up
      questions, when `conduct` runs, then it spawns exactly 4 level-2
      researchers and 1 composer; the level-2 notes record their follow-ups and
      none is spawned.
- [ ] Given `depth: 3`, when `conduct` runs, then level 1 spawns at most 4
      children, each level-2 node at most 2, and level 3 none; each researcher
      is told its cap at spawn.
- [ ] Given a note whose `follow_ups` exceed its level's cap, when
      `outstanding` derives missing nodes, then it takes the first `cap`
      entries and emits a warning, and the note stays valid.
- [ ] Given a note whose follow-up normalises to the pair question or to a node
      already in the tree, when `outstanding` derives missing nodes, then that
      follow-up is skipped and the remaining siblings keep their lineage
      positions.
- [ ] Given a researcher at level 2 or below, then its spawn prompt lists the
      pair question, its ancestors' questions and the tree's recorded
      follow-ups as questions not to propose.
- [ ] Given a researcher that returns follow-ups differing from its note, when
      `conduct` spawns the next level, then the children match the note's
      `follow_ups`, not the return.
- [ ] Given a node returning `follow_ups: []`, when `conduct` runs, then that
      node spawns no children and the pair still composes.
- [ ] Given `breadth: 2` and `depth: 3`, when `conduct` runs, then the round
      researches at most 2 focus areas regardless of node count.
- [ ] Given `research.topic.concurrency: 2` and 5 pairs at `depth: 1`, when
      `conduct` runs, then it spawns researchers in batches of at most 2; given
      `--concurrency 3` as well, then batches of at most 3; given
      `--concurrency` on `outline`, then it is rejected as a misplaced flag.
- [ ] Given no concurrency config or flag, then `conduct` spawns at most 24 at
      a time, composers included.
- [ ] Given an `openalex` pair at `depth: 2`, then every node and the composer
      use the `openalex` profile, and every note and the finding carry
      `source_profile: openalex`.
- [ ] Given a completed `depth: 2` pair, then its notes sit at
      `findings/<stem>.levels/<lineage>.md` with `kind: level-note` frontmatter
      including `level`, `depth` and `follow_ups`, and a whole-corpus
      `accelerator corpus frontmatter validate` exits 0.
- [ ] Given a composed finding, then every source it cites appears in at least
      one of its pair's notes, it is stamped with `depth`, and `finding_count`
      counts it once and none of its notes.
- [ ] Given a researcher subagent, when it writes a `.levels/<lineage>.md` path,
      then the guard allows it; when it writes any other path under `.levels/`,
      then the guard rejects it.
- [ ] Given the `composer` agent, then it has no fetch tools, and the guard
      allows its injected top-level finding path and rejects every path under
      `.levels/`.
- [ ] Given a level-2 node that wrote no note (e.g. `budget_exhausted`), when
      `conduct` runs, then the pair has no finding, stays outstanding, and the
      summary names the node by lineage; when `conduct` re-runs, then it spawns
      only that node and its subtree, then the composer.
- [ ] Given every note of a pair but no finding, when `conduct` re-runs, then it
      spawns only the composer.
- [ ] Given focus areas A (`03`) and B (`04`) at `depth: 2` where A leaves only
      `03-a-web.levels/` and B composes `04-b-web.md`, when `conduct` re-runs,
      then A is allocated `03-a-web.md` and resumes from its notes; a newly
      appended focus area is never allocated `03`.
- [ ] Given a pair left outstanding at `depth: 3`, when `conduct` re-runs with
      `--depth 1`, then the composer writes the finding from `1.md` alone,
      stamped `depth: 1`, and no researcher is spawned.
- [ ] Given a pair left outstanding at `depth: 3`, when `conduct` re-runs with
      `--depth 2`, then it composes from the level-1 and level-2 notes, stamps
      `depth: 2`, and spawns no researchers.
- [ ] Given a pair left outstanding at `depth: 2` with all its notes, when
      `conduct` re-runs with `--depth 3`, then it spawns level-3 researchers
      from the level-2 notes' recorded follow-ups, then composes with
      `depth: 3`.
- [ ] Given a pair whose finding is composed, when `conduct` re-runs at a larger
      `--depth`, then the pair is not re-researched and its finding is
      unchanged.
- [ ] Given `conduct` at `depth > 1`, then no dormant-depth notice is emitted,
      and `configure help` documents the engine without the 0283 caveat.
- [ ] All checks pass under `mise run check`.

## Open Questions

- None. The design questions — where recursion runs, what halving halves, the
  output shape, notes placement, and failure handling — were resolved in the
  2026-09-25 enrichment (see Drafting Notes).

## Dependencies

- Blocked by: 0280 (the output-quality gate must validate the premise before the
  token-multiplying recursion lands — recorded on 0280's `blocks`; this item
  unblocks on 0280's recorded gate sign-off with all four judgements passing,
  not on 0280's merge). As of 2026-09-25, 0280 is `ready` and its plan is
  validated partially complete pending attended checks and the gate.
- 0282 (the `depth` knob) is done; its `blocks` edge is satisfied.
- Relates to: 0284 — level notes are not navigable from the set-level detail
  page; that is out of scope here.

## Assumptions

- Keeping the default `depth: 1` makes this expensive recursion strictly opt-in.
- A fixed starting cap of 4 suffices; promoting it to a
  `research.topic.fan_out` knob later is additive.
- Accepted risk: `depth > 1` multiplies fetch calls, so an `openalex` pair can
  exhaust a keyless daily budget within one round. It is not throttled; funding
  a key is the remedy.
- Accepted risk: the concurrency cap ships inside this descope candidate, so
  dropping 0283 leaves `breadth` above 8 uncapped at `depth: 1`.
- Accepted risk: near-paraphrase siblings at the same level run in parallel
  unseen by each other and still spawn; the composer merges the overlap. No
  surveyed system (dzhng, Anthropic, Sagan, open_deep_research, GPT-Researcher)
  dedupes sub-questions semantically or persists a dedupe judgement.

## Technical Notes

- Worst case per pair, excluding the composer, at cap 4: 5 nodes at `depth: 2`,
  13 at `depth: 3`, 21 at `depth: 4`. Pruning and dedupe keep real cost
  beneath this.
- Walkers of `findings/`: the visualiser indexer
  (`cli/visualiser/server/src/file_driver.rs:258`) never lists inside
  `findings/`; `corpus topic-research outstanding`
  (`cli/corpus-adapters/src/topic_research.rs:92`) lists flat and ignores
  non-`.md` names; the researcher write guard
  (`cli/corpus/src/topic_research/finding_path.rs:5`, via
  `cli/research-cli/src/write_target.rs:85`) accepts only
  `<set>/findings/<name>.md` today; `corpus frontmatter validate`
  (`cli/corpus-adapters/src/fs.rs:73`) and migrations m0007/m0008 recurse with
  no dot-skip — hence notes carry frontmatter.
- The notes directory name must never end in `.md`: `outstanding` would read it
  as a broken finding and consume an index.
- Index allocation: `Planner::index_for`
  (`cli/corpus/src/topic_research/round.rs:434`) reuses an index only from a
  retained finding or a quarantine marker, and `highest` counts only those;
  `read_findings` skips non-`.md` names. Without index retention a pair whose
  focus area composed nothing is reallocated `highest + 1` once a later focus
  area composes, orphaning its `.levels/` directory.
- Deduplication reuses `normalised` (`round.rs:292`), the whitespace-collapsing
  comparison `outstanding` already applies to outline questions.
- The guard recognises writers by `agent_type`
  (`cli/research-cli/src/guard.rs:74`), so the `composer` agent needs its own
  confinement rule.
- Testability: the guard, `outstanding`'s missing-node derivation — cap,
  halving, over-cap trim, dedupe and index retention — and note validation are
  unit-testable; spawn batching, cap injection, known-question injection and
  the researcher's ordering of follow-ups are `SKILL.md` prose contracts
  verified at eval level. The composer's no-fetch rule is enforced by its tool
  list.

## Drafting Notes

- Enriched interactively on 2026-09-25, replacing the extraction-only draft.
  Decisions: `conduct` orchestrates levels rather than in-agent deepening,
  because researchers cannot spawn subagents and orchestration makes depth and
  halving enforceable; per-node halving with a fixed starting cap of 4, not a
  new knob; one finding per pair composed from kept notes; a non-dot
  `.levels/` directory, because the dot prefix means "quarantined" in
  `findings/` and hides kept notes from file browsers; notes carry frontmatter
  and the guard is extended; failure resumes rather than keeping a partial
  finding, with `--depth` as the escape hatch; limit-level nodes record
  follow-ups so a deeper re-run extends the tree.
- Stress-tested on 2026-09-25. Decisions: the notes on disk are the single
  source of truth for follow-ups, so `outstanding` derives each level and fresh
  runs and resumes share one path; the cap is injected so the model chooses its
  follow-ups, with an ordered trim as a backstop rather than rejection; a
  `.levels/` directory holds its index through its level-1 note's question; a
  smaller `--depth`, including `--depth 1`, composes from existing notes; a
  dedicated fetch-less `composer` agent; a per-batch concurrency cap as config
  plus a `--concurrency` flag, kept in this item rather than split out despite
  the descope risk; known-question injection plus exact-normalised dedupe,
  after a sweep of the epic's prior art found no semantic sub-question dedupe
  anywhere.
- This remains the acknowledged descope candidate. If the epic runs long, drop
  this child and keep the knob (0282).
- Status kept at `draft`.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 5, recursion half)
- Parent epic: 0121
- Siblings: `meta/work/0282-tunable-depth-and-breadth.md` (the `depth` knob),
  `meta/work/0280-academic-source-profiles.md` (gate; write guard),
  `meta/work/0284-topic-research-set-detail-page.md`
- Prior art: dzhng/deep-research (per-node breadth halving)
- To modify: `skills/research/research-topic/SKILL.md` (`conduct`),
  `agents/researcher.md`, `skills/research/outputters/`,
  `templates/topic-research-finding.md`, `skills/config/configure/SKILL.md`,
  `cli/corpus/src/topic_research/round.rs` (missing nodes, cap, dedupe, index
  retention), `cli/corpus/src/topic_research/finding_path.rs` and
  `cli/research-cli/src/guard.rs` (write guard), `cli/config/src/catalogue.rs`
  (`research.topic.concurrency`, `composer` agent key)
- To create: `agents/composer.md`, a level-note outputter under
  `skills/research/outputters/`, and a level-note template
