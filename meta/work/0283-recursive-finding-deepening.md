---
type: "work-item"
id: "0283"
title: "Recursive Finding Deepening"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "story"
priority: "high"
parent: "work-item:0121"
tags: ["research", "skills", "deep-research"]
last_updated: "2026-10-04T12:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-867"
---

# 0283: Recursive Finding Deepening

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As an Accelerator user researching a subject, I want a finding to drill into
its own follow-up questions to a depth I choose, so that one focus area yields
a deeper answer without me commissioning each sub-question as a new round.

At `depth > 1`, `conduct` researches each (focus area, profile) pair as a tree:
each researcher in the tree is a node and writes one level note recording up to
`n` follow-up questions, `n` starting at 4 and halving per descent (dzhng's
per-level halving, applied here to the follow-up cap rather than to dzhng's
per-query fan-out); `conduct` spawns the next level from the follow-ups
recorded on disk until the configured depth, then a composer writes the pair's
single finding from its notes. A pair with no notes at `depth: 1` keeps
today's single researcher. A concurrency cap bounds how many agents `conduct`
spawns at once, at every depth. This is the epic's
acknowledged descope candidate — droppable if the epic runs long, keeping the
knob (0282).

## Context

`depth > 1` multiplies spend, so it lands only after the output-quality gate
(0280) validates the premise and after the knob (0282) lets users bound it.
At `depth: 1` a finding answers its question from one researcher's pass, so a
sub-question it surfaces goes unanswered unless the user commissions it as a
focus area in a later round. A finding's recursion is governed by `depth` and
does not consume the round's `breadth` budget, which continues to bound focus
areas only.

Researchers cannot spawn subagents — `agents/researcher.md` carries no Task
tool, and Claude Code subagents cannot nest — so the recursion cannot live
inside a researcher. `conduct` orchestrates each level instead, and the
filesystem carries the research between levels, so `conduct` routes questions
and paths but never holds research content.

## Requirements

- **Level orchestration.** At `depth > 1`, `conduct` spawns one researcher per
  missing node that `research topic outstanding` derives from the notes
  on disk, re-running `outstanding` after every batch; fresh runs and resumes
  take the same path. Levels are barriered within a pair, not across pairs:
  `outstanding` reports a pair's level-`L` nodes only once every level-`L − 1`
  node of that pair has a validated note, so one batch may hold different
  pairs at different levels alongside composers.
- **Follow-up cap.** A node's cap is 4 at level 1 and `ceil(parent cap / 2)`
  at each deeper level. `conduct` injects the cap at spawn, and the researcher
  records at most that many `follow_ups`, most valuable first. When a note
  records more, `outstanding` takes the first `cap` entries and warns; it never
  rejects the note. The warning goes to stderr and names the note's lineage and
  the count trimmed.
- **Follow-ups at every level.** Every node, including those at the depth
  limit, records its capped follow-up questions; `outstanding` derives children
  only from notes whose `level` is less than the resolved `depth`.
  `follow_ups: []` therefore always means the node judged its question answered
  (pruning), never that it hit the limit.
- **Duplicate avoidance.** `conduct` injects into each researcher the questions
  its tree already covers — the pair question, its ancestors' questions, and
  the follow-ups recorded so far — as questions not to propose again.
  `outstanding` applies the cap trim first, then walks a pair's candidates in
  lineage order — level ascending, then positions compared left to right
  numerically — and skips one whose normalised question equals the pair
  question, any shallower node's question, or an earlier candidate's at the
  same level. The level barrier means every candidate at a level is known
  before any is derived, so the lowest lineage deterministically keeps a
  duplicate. A skipped follow-up is not backfilled from past the cap, and its
  siblings keep their positions, so lineages stay stable.
- **Content-free orchestration.** The notes on disk are the single source of
  truth for follow-ups. A researcher returns only a short summary, never its
  follow-ups or note body.
- **Concurrency.** `conduct` spawns every batch — researchers at any depth,
  including `depth: 1`, and composers — at most `concurrency` at a time,
  resolved flag (`conduct --concurrency N`) > config
  (`research.topic.concurrency`) > default `24`. `--concurrency` on `outline`
  is ignored with a one-line note, as the other misplaced flags are. A batch is one set of parallel spawns that
  `conduct` issues together and waits on in full before issuing the next;
  researchers and composers of different pairs may share a batch. `conduct`
  fills each batch to `concurrency` from everything outstanding, so N spawns
  pending at one re-run of `outstanding` take `ceil(N / concurrency)` batches.
  A non-integer or value below 1 clamps to 1 with a warning naming the invalid
  value, as 0282's knobs do.
- **Level notes.** Each node writes one note to
  `findings/<stem>.levels/<lineage>.md`, where `<stem>` is the finding's
  filename stem `<nn>-<name>` and lineage encodes the
  parent chain: the level-1 node is `1`, and a level-`L` node is `L` followed
  by `L − 1` positions, ancestor first, each the 1-based position of the
  question in its parent's capped `follow_ups` — `3-2-1` is the first
  follow-up of `2-2`, itself the second of `1`. Notes are written through
  a new level-note outputter and template, carry frontmatter
  `kind: level-note`, `question`, `source_profile`, `round`, `level`, `depth`
  (the resolved depth the node ran under, a record that neither derivation
  nor composition consults), and `follow_ups`, and are registered
  under the umbrella `topic-research` type. The body mirrors the finding
  template — an answer to the node's question and a tier-tagged Sources list.
  Notes are kept after composition.
- **Composer.** Once `outstanding` reports no missing nodes for a pair at the
  resolved depth, `conduct` spawns
  a dedicated `composer` agent (tools `Read, Write` only; configurable through
  `accelerator config agent composer`) that reads only that pair's notes within
  the resolved depth and writes the pair's finding at its existing path through
  `finding-outputter` — standalone prose answering the pair question, not a
  level-by-level walk — stamped with the resolved `depth`. Lacking fetch
  tools, it can cite only sources already in the notes.
- **`depth: 1` unchanged in shape.** A pair with no `.levels/` directory, at
  `depth: 1`, gets one researcher that writes the finding directly; no
  `.levels/` directory and no composer. That researcher stamps `depth: 1`; a
  legacy finding without `depth` reads as `depth: 1`.
- **Profile confinement.** Every node and the composer of a pair run with that
  pair's injected profile, and the finding keeps its `source_profile`.
- **No human between levels**, and a finding's recursion does not consume the
  round's `breadth` budget.
- **Write guard.** The guard confines by path shape, as it does findings today;
  that each spawn writes only its one injected path is a prompt contract.
  Researcher subagents may additionally write
  `<set>/findings/<stem>.levels/<lineage>.md`, where `<stem>` carries its
  `<nn>-` index and `<lineage>` is well-formed: a level `L ≥ 1` followed by
  exactly `L − 1` positive positions. Every other path under `.levels/` is
  rejected. The `composer` agent may write only `<set>/findings/<stem>.md`,
  never under `.levels/`.
- **Resume.** A pair stays outstanding until its finding validates.
  `research topic outstanding` takes the resolved depth (`--depth N`)
  and reports two categories of pair. A pair with missing nodes lists them: a
  pair with no validated `1.md` — none, or only `.1.md.invalid` — has missing
  node `1`; otherwise its missing nodes are the capped, deduplicated
  `follow_ups` of notes whose `level` is less than that depth, minus validated
  notes. A pair with no missing nodes awaits only composition. A re-run spawns
  only the missing nodes, then composes. `outstanding` validates each note it
  reads and treats a failing one as missing; `conduct` validates each note
  once its batch returns, as it does findings, quarantining an invalid one as
  `.<lineage>.md.invalid`.
- **Index retention.** A `.levels/` directory holds its `<nn>` index as a
  quarantine marker does, keyed on the `question` of its `1.md` or
  `.1.md.invalid`, so a resumed pair is allocated the same finding stem and a
  new focus area never takes that index.
- **Depth changes on resume.** A re-run at a smaller `--depth` composes from the
  notes whose `level` is within that depth and stamps it — at `--depth 1`,
  from `1.md` alone, spawning no researcher; a re-run at a larger `--depth`
  extends the tree from the recorded follow-ups of notes at the old limit. A
  pair whose finding validates is complete, however that finding was written,
  and is never re-deepened.
- **Counting.** `finding_count` and `round_count` count only the top-level
  `<nn>-*.md` files directly in `findings/`; `synthesise` reads only those.
- **Surface.** Remove the dormant-depth notice from `conduct` and the "no effect
  until 0283" caveat from `configure help`, and document `depth > 1` recursion,
  the
  `research.topic.concurrency` key and its default, and the `composer` agent
  key. `conduct`'s closing summary names each failed node by lineage with its
  reason and next step.

## Acceptance Criteria

- [ ] Given `depth: 1` and pairs with no `.levels/` directory, when `conduct`
      runs, then each pair gets exactly one
      researcher that writes the finding directly, and no `.levels/` directory
      is created.
- [ ] Given a pair at `depth: 2` whose level-1 note records 4 `follow_ups`,
      when `conduct` runs, then it spawns exactly 4 level-2 researchers and 1
      composer; each level-2 spawn prompt asks for up to 2 follow-ups, no
      researcher's returned summary contains a `follow_ups` entry of its note
      as a normalised substring, and no level-3 researcher is spawned.
- [ ] Given a seeded `1.md` recording 6 `follow_ups` and seeded `2-1.md` to
      `2-4.md` recording 3 each, when `outstanding --depth 3` runs, then it
      reports exactly the 8 lineages `3-k-1` and `3-k-2` for k in 1–4, and
      warns for each trimmed note.
- [ ] Given a pair at `depth: 3` with no notes, when `conduct` runs, then the
      level-1 spawn prompt carries cap 4; given instead a seeded `1.md`
      recording 2 `follow_ups` and a seeded `2-1.md` recording 1, then the
      `2-2` spawn prompt carries cap 2 and the `3-1-1` spawn prompt cap 1.
- [ ] Given a note whose `follow_ups` exceed its level's cap, when
      `outstanding` derives missing nodes, then it takes the first `cap`
      entries, writes a stderr warning naming the note's lineage and the count
      trimmed, exits 0, and the note stays valid.
- [ ] Given a note whose follow-up normalises to the pair question or to a node
      already in the tree, when `outstanding --depth 3` derives missing nodes,
      then that follow-up is skipped and the remaining siblings keep their
      lineage positions — `1.md` recording [the pair question, B, C] yields
      missing nodes `2-2` and `2-3`, and no `2-1`; `1.md` recording [B, B]
      yields only `2-1`; `2-1.md` recording [X] and `2-2.md` recording [X, Y]
      yield `3-1-1` and `3-2-2`, and no `3-2-1`; a follow-up in `2-1.md` equal
      to `2-2`'s question is skipped.
- [ ] Given a pair at `depth: 3` whose `2-4` note is missing while `2-1.md`
      to `2-3.md` exist, when `outstanding` derives missing nodes, then it
      reports `2-4` and no level-3 node for that pair.
- [ ] Given `--concurrency 3`, pair A awaiting only composition and pair B at
      `depth: 2` with 4 missing level-2 nodes, when `conduct` runs, then it
      spawns A's composer and B's level-2 researchers in batches of 3 and 2,
      then B's composer in a batch of 1.
- [ ] Given a researcher at level 2 or deeper, then its spawn prompt lists the
      pair question, its ancestors' questions and the tree's recorded
      follow-ups as questions not to propose.
- [ ] Given a seeded `1.md` whose `follow_ups` no researcher in the session
      returned, when `conduct` runs, then each level-2 spawn prompt carries the
      question at its lineage position in the note's capped `follow_ups`.
- [ ] Given a node whose note records `follow_ups: []`, when `conduct` runs,
      then it spawns no children of that node and the pair still composes.
- [ ] Given `breadth: 2`, an outline of 2 focus areas and `depth: 3`, when
      `conduct` runs, then both focus areas are researched to depth 3 whatever
      their node counts.
- [ ] Given `research.topic.concurrency: 2` and 5 pairs at `depth: 1`, when
      `conduct` runs, then it spawns researchers in batches of 2, 2 and 1;
      given `--concurrency 3` as well, then batches of 3 and 2; given
      `--concurrency` on `outline`, then it is ignored with a one-line note,
      as the other misplaced flags are.
- [ ] Given `--concurrency 0`, or `research.topic.concurrency: many`, when
      `conduct` runs, then it spawns one agent per batch and warns, naming the
      invalid value and stating it was clamped to 1.
- [ ] Given no concurrency config or flag and 30 pairs at `depth: 1`, when
      `conduct` runs, then it spawns researchers in batches of 24 and 6; given
      instead 30 pairs awaiting only composition, then it spawns composers in
      batches of 24 and 6.
- [ ] Given an `openalex` pair at `depth: 2`, then every node and the composer
      use the `openalex` profile, and every note and the finding carry
      `source_profile: openalex`.
- [ ] Given a completed `depth: 2` pair, then its notes sit at
      `findings/<stem>.levels/<lineage>.md` with `kind: level-note` frontmatter
      including `level`, `depth` and `follow_ups`, their sources are
      tier-tagged, and a whole-corpus
      `accelerator corpus frontmatter validate` exits 0.
- [ ] Given a composed finding, then it has no heading or section named after a
      level or lineage; for a pair of at least 3 notes, its sections also do
      not map one-to-one onto the notes in lineage order.
- [ ] Given a composed finding, then every source URL it cites appears in at
      least one of its pair's notes whose `level` is within its stamped
      `depth`, it is stamped with `depth`, and `finding_count` counts it once
      and none of its notes; `round_count` and `synthesise`
      ignore `.levels/`, and the pair's notes still exist.
- [ ] Given a researcher subagent, then the guard allows
      `findings/03-a-web.levels/1.md` and `findings/03-a-web.levels/3-2-1.md`,
      and rejects `.levels/` paths `2.md` (too few positions), `2-1-1.md` (too
      many), `2-0.md`, `2-a.md`, `0.md`, `.2-1.md.invalid`, `2-1.txt`,
      `2-1/x.md`, `findings/03-a-web.levels/1.levels/1.md`, and
      `findings/a-web.levels/1.md`.
- [ ] Given the `composer` agent, then it has no fetch tools, and the guard
      allows `findings/03-a-web.md` and rejects
      `findings/03-a-web.levels/1.md` and `findings/.03-a-web.md.invalid`.
- [ ] Given a pair at `depth: 2` with a seeded `1.md` recording 2
      `follow_ups` and web fetch denied, when `conduct` runs, then the pair has
      no finding, stays outstanding, and the summary names `2-1` and `2-2`,
      each with the reason its returned summary gives and a next step offering
      a re-run of `conduct`, or a re-run with a smaller `--depth` to compose
      from the notes on disk; when `conduct` re-runs with fetch restored, then
      it spawns only those nodes, then the composer.
- [ ] Given a level-2 note that fails validation, when its batch returns, then
      `conduct` renames it `.2-1.md.invalid` and the summary names `2-1` with
      its validation error; when `conduct` re-runs, then it spawns only that
      node, then the composer.
- [ ] Given a `.levels/` directory whose level-1 note is quarantined as
      `.1.md.invalid` and no finding, when a later focus area is appended and
      `conduct` re-runs, then the pair keeps its `<nn>` index.
- [ ] Given every note of a pair but no finding, when `conduct` re-runs, then it
      spawns only the composer.
- [ ] Given focus areas A (`03`) and B (`04`) at `depth: 2` where A leaves only
      `03-a-web.levels/` and B composes `04-b-web.md`, when `conduct` re-runs,
      then A is allocated `03-a-web.md` and resumes from its notes; a newly
      appended focus area is never allocated `03`.
- [ ] Given a pair left outstanding at `depth: 3` with a validated `1.md` and
      deeper notes citing source URLs absent from `1.md`, when `conduct`
      re-runs with `--depth 1`, then the composer writes the finding, stamped
      `depth: 1`, citing at least one source from `1.md` and none found only in
      deeper notes, and no researcher is spawned.
- [ ] Given a pair left outstanding at `depth: 3` with every level-1 and
      level-2 note validated and level-3 notes citing source URLs absent from
      them, when `conduct` re-runs with `--depth 2`, then it composes from the
      level-1 and level-2 notes, citing none of the level-3-only sources,
      stamps `depth: 2`, and spawns no researchers.
- [ ] Given a pair left outstanding at `depth: 2` with all its notes, when
      `conduct` re-runs with `--depth 3`, then it spawns level-3 researchers
      from the level-2 notes' recorded follow-ups, then composes with
      `depth: 3`.
- [ ] Given a pair whose finding is composed, when `conduct` re-runs at a larger
      `--depth`, then the pair is not re-researched and its finding is
      unchanged.
- [ ] Given `conduct` at `depth > 1`, then no dormant-depth notice is emitted,
      and `configure help` documents `depth > 1` recursion without the 0283
      caveat,
      `research.topic.concurrency` with its default `24`, and the `composer`
      agent key.
- [ ] Given `accelerator config agent composer` overridden in config, when
      `conduct` composes, then it spawns the configured agent.
- [ ] Given a legacy finding with no `depth`, then it validates and
      `outstanding --depth 3` reports its pair complete.
- [ ] `mise run` exits 0, with unit tests covering every guard path listed
      above, cap halving and over-cap trim, the dedupe examples, the level
      barrier, index retention from `1.md` and `.1.md.invalid`, and
      depth-bounded derivation at smaller and larger `--depth`, a
      composition-only pair, a dedupe case where `B` and ` B ` collapse under
      `normalised`; and clamping of `research.topic.concurrency` verified by
      the attended checks, as 0282's knobs are.

## Open Questions

- None. The design questions — where recursion runs, what halving halves, the
  output shape, notes placement, and failure handling — were resolved in the
  2026-09-25 enrichment (see Drafting Notes).

## Dependencies

- Blocked by: 0280 (the output-quality gate must validate the premise before the
  token-multiplying recursion lands — recorded on 0280's `blocks`; this item
  unblocks on 0280's recorded gate sign-off with all four judgements passing,
  not on 0280's merge). As of 2026-09-25, 0280 is `ready` and its plan is
  validated partially complete pending attended checks and the gate. The gate
  runs on a released plugin, so its sign-off also means the pair model,
  profiles and researcher guard this item extends have merged; re-check the
  Technical Notes' code references against the merged code before planning.
- 0282 (the `depth` knob) is done; its `blocks` edge is satisfied.
- Registering `research.topic.concurrency` and the `composer` agent key lands
  after 0280 and reconciles the config key-count test, `dump.golden`,
  `parity.rs` and `public-api.txt`.
- External: the attended profile-confinement check needs OpenAlex reachable
  and a funded `openalex.api_key` or unspent keyless daily budget for a
  `depth: 2` round.
- Relates to: 0278 — its indexer emits one entry per set and no sub-documents,
  so level notes stay unindexed; `kind: level-note` is a new row on the
  existing umbrella `topic-research` type, not something 0278 must deliver.
- Relates to: 0281 — `ask` and `report` read only top-level `<nn>-*.md`
  findings and never `.levels/`.
- Blocks (in part): 0161 — its recursion-eval portion covers `conduct`'s
  orchestration contracts, needs this item landed first, and drops if this
  item is descoped.
- Relates to: 0284 — the set-level page enumerates only top-level `<nn>-*.md`
  findings and excludes `kind: level-note`; navigating level notes is out of
  scope here.

## Assumptions

- Keeping the default `depth: 1` makes this expensive recursion strictly opt-in.
- A fixed starting cap of 4 suffices; promoting it to a
  `research.topic.fan_out` knob later is additive.
- Accepted risk: `depth > 1` multiplies fetch calls, so an `openalex` pair can
  exhaust a keyless daily budget within one round. It is not throttled; funding
  a key is the remedy.
- Accepted risk: arXiv requests are serialised at one per 3 s, so an `arxiv`
  pair's wall-clock time grows with its node count — up to 13× at `depth: 3` —
  and 0280's per-focus-area time estimate holds only at `depth: 1`.
- Accepted risk: the concurrency cap ships inside this descope candidate, so
  dropping 0283 leaves concurrent spawns uncapped when `breadth` is raised
  above its default of 8.
- Accepted risk: near-paraphrase siblings at the same level run in parallel
  unseen by each other and still spawn; the composer merges the overlap. No
  surveyed system (dzhng, Anthropic, Sagan, open_deep_research, GPT-Researcher)
  dedupes sub-questions semantically or persists a dedupe judgement.

## Technical Notes

- Worst case per pair, excluding the composer, at cap 4: 5 nodes at `depth: 2`,
  13 at `depth: 3`, 21 at `depth: 4`. Pruning and dedupe keep real cost
  beneath this. A focus area's worst case is that times its profile count.
- Walkers of `findings/`: the visualiser indexer
  (`cli/visualiser/server/src/file_driver.rs:258`) never lists inside
  `findings/`; `research topic outstanding`
  (`cli/research-adapters/src/topic_research.rs:89`) lists flat and ignores
  non-`.md` names; the researcher write guard
  (`cli/corpus/src/topic_research/finding_path.rs:5`, via
  `cli/research-cli/src/write_target.rs:85`) accepts only
  `<set>/findings/<name>.md` today; `corpus frontmatter validate`
  (`cli/corpus-adapters/src/fs.rs:73`) and migrations m0007/m0008 recurse with
  no dot-skip — hence notes carry frontmatter.
- The notes directory name must never end in `.md`: `outstanding` would read it
  as a broken finding and consume an index.
- Index allocation: `Planner::index_for`
  (`cli/research/src/round.rs:434`) reuses an index only from a
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
  the researcher's ordering of follow-ups, limit-level nodes recording their
  follow-ups, and the composer's standalone-prose
  shape are `SKILL.md` prose contracts,
  verified by attended runs here; eval coverage belongs to 0161. The
  composer's no-fetch rule is enforced by its tool list.
- Attended runs set up an orchestration precondition by seeding `.levels/`
  notes and running `conduct` as a resume, since fresh runs and resumes share
  one path. They read a batch as the Agent calls issued in one assistant
  message, and caps and known questions from each spawn prompt. They use `web`
  pairs, except the profile-confinement criterion, which needs OpenAlex
  reachable with budget for a `depth: 2` round.

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
- The concurrency cap stays here rather than in its own story: uncapped fan-out
  becomes a practical risk only at `depth > 1`, so it ships with the feature
  that creates the risk, accepting that concurrent spawns stay uncapped when
  `breadth` is raised above its default of 8 until then.
- Kept as one story rather than split: level notes, `outstanding`'s
  derivation, the guard, the composer and resume share one path, so no subset
  ships usable recursion.
- This remains the acknowledged descope candidate. If the epic runs long, drop
  this child and keep the knob (0282).
- Promoted to `ready` on 2026-09-26 after review 1 was approved.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 5, recursion half)
- Parent epic: 0121
- Siblings: `meta/work/0282-tunable-depth-and-breadth.md` (the `depth` knob),
  `meta/work/0280-academic-source-profiles.md` (gate; write guard),
  `meta/work/0284-topic-research-set-detail-page.md`
- Prior art: dzhng/deep-research (per-node breadth halving),
  `https://github.com/dzhng/deep-research`
- To modify: `skills/research/research-topic/SKILL.md` (`conduct`),
  `agents/researcher.md`, `skills/research/outputters/`,
  `templates/topic-research-finding.md`, `skills/config/configure/SKILL.md`,
  `cli/research/src/topic/plan.rs` and `tree.rs` (missing nodes, cap,
  dedupe, index retention), `cli/research/src/topic/layout/finding_path.rs`
  and `cli/research-cli/src/guard.rs` (write guard),
  `cli/config/src/catalogue.rs`
  (`research.topic.concurrency`, `composer` agent key) and its key-count test,
  `dump.golden`, `parity.rs` and `public-api.txt`
- To create: `agents/composer.md`, a level-note outputter under
  `skills/research/outputters/`, and a level-note template
