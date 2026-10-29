---
type: "work-item"
id: "0347"
title: "Redo, Revisit and Deepen Research Findings"
date: "2026-10-22T19:17:50+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0121"
relates_to: ["work-item:0279", "work-item:0281", "work-item:0283", "work-item:0307", "work-item:0308", "work-item:0310"]
external_id: "PP-923"
tags: ["research", "deep-research", "outline", "conduct"]
last_updated: "2026-10-22T19:17:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0347: Redo, Revisit and Deepen Research Findings

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As an Accelerator user building a topic-research corpus over time, I want to
re-research or extend findings already in a set, so that the corpus grows
richer and stays current rather than freezing at what its first round found.

`outline` gains three operations on an existing finding, each written as an
outline item that `conduct` executes like any focus area: `redo` re-researches
it from scratch, `revisit` re-researches it building on the previous version,
and `deepen` extends its research tree below its current leaves. Findings stop
being immutable; the VCS holds their history.

## Context

Epic 0121 made findings immutable: `conduct` researches only pairs with no
finding on disk and never rewrites one. That keeps gap detection simple, but it
means a corpus can only widen. A finding cannot absorb newer sources, be
corrected when the literature moves, or be researched further once written.

The only route today is manual quarantine: rename `findings/<stem>.md` to
`.<stem>.md.invalid` and re-run `conduct`. `conduct`'s own summary recommends
this for a shallow finding, and 0283 makes a re-run at a larger `--depth`
extend the tree from the follow-ups its notes recorded. Depth is run-wide,
though, so deepening one finding means deepening the run, and none of this is
planned, visible in the outline, or reviewable before it spends tokens.

This item came out of refining 0281, where re-asking a question should be able
to draw on fresher findings.

## Requirements

- `outline` accepts `--redo FINDING`, `--revisit FINDING` and
  `--deepen FINDING[:LEVELS]`, each repeatable. `FINDING` is a finding stem
  (`03-cost-model-openalex`), which names one (focus area, profile) pair.
  `outline` refuses a stem that names no retained, validated finding, naming
  it, and writes nothing.
- Each flag writes one item into the round `outline` writes, under its
  existing append-or-revise rule:
  - `- [ ] redo <stem>`
  - `- [ ] revisit <stem>`
  - `- [ ] deepen <stem> — levels: <N>`

  An item written by hand in the same shape behaves identically.
- An `outline` invocation carrying any of these flags writes only the
  requested items and generates no new focus areas.
- Operation items do not count against the breadth ceiling.
- A round holds at most one operation per finding; `outline` refuses a second,
  naming the finding and the operation already planned.
- **redo** researches the pair as if new, at the run's resolved depth. The old
  finding and its `.levels/` tree are removed at `conduct`'s clear step,
  immediately before the pair's first spawn, never at `outline` time.
- **revisit** gives each researcher — the single-pass researcher at depth 1,
  each node's researcher above it — the path of the document it is replacing
  as the previous version, and instructs it to keep sources that still hold,
  add newer or missing evidence, and correct claims that newer sources
  overturn. The composer then recomposes the finding from the revisited notes.
- **deepen** extends the tree from its deepest notes' recorded `follow_ups` to
  the finding's `depth` plus `LEVELS` (default 1, capped at 10), researching
  only the new levels, then recomposes the finding.
- An operation item is complete once its finding carries a `round` at or above
  the item's round and, for `deepen`, a `depth` at or above its target.
  `outstanding` offers incomplete operation items alongside unanswered pairs,
  and `conduct`'s tick step ticks them by the same rule.
- A re-researched finding is stamped with its item's round, so `round_count`
  rises past the synthesis's `rounds_covered` and `finalise` refuses until
  `synthesise` re-runs. `finding_count` is unchanged.
- `conduct`'s shallow-finding notice recommends `deepen` instead of a manual
  rename.
- When 0281's `ask` or `report` proposes research for a question the corpus
  answers only partly or not at all, the proposal may offer `revisit` or
  `deepen` of an existing finding whose coverage of the question is thin, as
  well as new focus areas. A confirmed operation is written as its outline
  item in the appended round.
- The epic (0121) stops describing findings as immutable, and an ADR records
  the reversal and that the VCS is the history.

## Acceptance Criteria

- [ ] Given a set with finding `03-cost-model-openalex`, when the user runs
      `outline SLUG --revisit 03-cost-model-openalex`, then the next round
      gains `- [ ] revisit 03-cost-model-openalex`, no new focus area, and the
      breadth ceiling is not consulted.
- [ ] Given `outline SLUG --redo 99-nonexistent`, then `outline` refuses,
      naming the stem, and `outline.md` and `manifest.md` are unchanged.
- [ ] Given a round already holding `revisit 03-…`, when `outline` is asked to
      `deepen 03-…` in that round, then it refuses, naming the planned
      `revisit`.
- [ ] Given a pending `redo 05-pricing-web` in round 4, when `conduct` runs,
      then the old finding and `.levels/` are absent only once the pair's first
      spawn is about to issue, and the new finding is stamped `round: 4`.
- [ ] Given a pending `revisit` of a depth-1 finding, when `conduct` spawns its
      researcher, then the prompt carries the existing finding's path as the
      previous version, and the rewritten finding is stamped with the item's
      round.
- [ ] Given a depth-2 finding with level notes, when `conduct` runs a pending
      `deepen <stem> — levels: 1`, then only level-3 nodes are spawned, from
      the level-2 notes' `follow_ups`, and the recomposed finding has
      `depth: 3`.
- [ ] Given a `deepen` whose target exceeds 10, then the target is clamped to
      10 with the existing depth-ceiling warning.
- [ ] Given a `conduct` interrupted mid-`revisit`, when `conduct` re-runs, then
      only notes not yet stamped with the item's round are re-spawned.
- [ ] Given a synthesised set, when an operation item is conducted, then
      `round_count` exceeds the synthesis's `rounds_covered` and `finalise`
      refuses until `synthesise` re-runs.
- [ ] Given every operation item conducted, when `conduct`'s tick step runs,
      then each item's checkbox is ticked.
- [ ] Given a question that `ask` or `report` judges partly answered because
      an existing finding covers it thinly, when the verb proposes research,
      then the proposal may include `revisit` or `deepen` of that finding,
      and on confirmation the appended round carries the operation item.

## Open Questions

- What does `revisit` do to a tree's shape above depth 1? A revisited node may
  record different `follow_ups`, orphaning descendants researched under the
  old ones. Recommendation: `revisit` keeps each node's `follow_ups` and
  lineage fixed, rewriting content only; new questions belong to `deepen` or a
  new focus area.
- How does `deepen` treat a depth-1 finding, which has no level notes and so no
  recorded `follow_ups`? Recommendation: refuse, pointing to `redo` with a
  larger `conduct --depth`.
- Should `redo` accept its own depth (`--redo STEM:DEPTH`) rather than taking
  the run's?

## Dependencies

- Blocked by: none.
- Related: 0281, whose `ask` and `report` propose new focus areas when the
  corpus cannot answer a question. This item extends those proposals to
  `revisit` and `deepen` items; if it lands first, the extension waits on
  0281's proposal flow, and the proposal criterion cannot close until 0281
  is done.
- Related: 0283, whose resume-at-larger-depth behaviour `deepen` scopes to one
  finding.
- Related: 0307, which passes spawn context by reference; `revisit`'s
  previous-version path joins whichever form that context takes.
- Related: 0308, whose write-scope guard must admit a revisit researcher
  overwriting its own offered path.
- Related: 0310, whose interrupted-run report must classify replaced files.

## Assumptions

- Operation items are planned through `outline` flags rather than new verbs,
  keeping one planner and one executor.
- The VCS is sufficient history for a replaced finding; nothing in the set
  archives earlier versions.
- One operation per finding per round; a revisit-then-deepen takes two rounds.

## Technical Notes

- `outstanding` (`accelerator research topic outstanding`) owns pair
  allocation, the run ledger and node derivation; it gains the operation
  items and the round-stamp completion rule.
- `conduct`'s step 3 (Clear) is where `redo` removes the old finding; the
  existing quarantine and marker rules show the shape.
- The finding's `depth` and each note's `round` and `level` already carry
  everything the completion rule and `deepen`'s target need.

## Drafting Notes

- Names chosen in refinement: `redo` (from scratch), `revisit` (build on the
  previous version), `deepen` (extend below the leaves); `update` was rejected
  because it collides with `/update-work-item`.
- A flags-only `outline` invocation writing no new focus areas is my
  interpretation; the alternative is to plan both in one call.
- Priority set to medium: it serves the epic's "deepens on demand" goal but
  nothing is blocked on it.

## References

- Parent epic: 0121
- Related: 0279, 0281, 0283, 0307, 0308, 0310
