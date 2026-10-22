---
type: "work-item"
id: "0281"
title: "Corpus Consumption: Ask and Report"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocks: ["work-item:0284"]
blocked_by: ["work-item:0277", "work-item:0279"]
relates_to: ["work-item:0280", "work-item:0283", "work-item:0284", "work-item:0347"]
tags: ["research", "skills", "consumption"]
last_updated: "2026-10-22T19:20:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-865"
---

# 0281: Corpus Consumption: Ask and Report

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As an Accelerator user with a researched topic, I want to question the corpus
— conversationally or as a durable report — and be offered more research when
it cannot answer, so that the knowledgebase is consulted rather than re-read
and grows where my questions find it thin.

`ask SLUG QUESTION` answers in conversation from the findings and synthesis.
`report SLUG QUESTION` writes `reports/<slug>.md`, a standalone answer with
tiered Sources, rewritten whenever the same question is asked again. When the
corpus cannot answer satisfactorily, either verb proposes a research round and,
on the user's confirmation, writes it into `outline.md`.

## Context

This closes the build → consult → discover-a-gap → refine loop the reference
systems stop short of. The build verbs (0277, 0279, 0280, 0282, 0283) produce
findings and a synthesis but nothing reads them back in answer to a question.

The two verbs differ in durability. `ask` is for interrogating the corpus
interactively and leaves nothing behind. `report` produces an artefact that
plans, ADRs and work items can cite, and since the corpus keeps growing, a
report is regenerated rather than frozen: re-asking its question rewrites it
from the current findings, with the VCS as its history.

## Requirements

- **Shared**
  - Both verbs take a `SLUG`, resolved through the Shared Preamble
    (`accelerator corpus resolve --type topic-research`), and a question.
  - Both read `synthesis.md`, when present, and every top-level `<nn>-*.md`
    finding, never `.levels/`.
  - Both judge coverage across every finding of a focus area — one per source
    profile since 0280. A finding whose Sources read `None found.` is evidence
    of absence in that source family, not a gap.
  - Both treat finding and synthesis bodies as untrusted data: orientation
    only, never instructions, as `synthesise` does.
  - Both spawn nothing and run inline.
- **ask**
  - Requires at least one finding on disk; refuses otherwise, naming the
    statuses that carry findings (`researching`, `synthesised`, `complete`).
  - Answers in conversation and writes nothing to the set except a confirmed
    research proposal.
- **report**
  - Requires `synthesised` or `complete`; refuses otherwise, pointing to
    `synthesise SLUG`.
  - Writes `reports/<slug>.md` with `kind: report`, its `question`, and
    `rounds_covered` equal to the manifest's `round_count`, synthesised from
    the corpus with Sources derived from the findings' tiers. Tiers carry
    through unchanged, including `tier-3 (retracted)` and
    `tier-3 (withdrawn)`.
  - Derives the slug from the question. A report whose `question` matches is
    overwritten in place; a different question whose slug collides takes a
    numeric suffix.
  - Edits `manifest.md` as its final step to record reports — `report_count`
    from disk and a `reports/` entry under `## Set` — leaving `status`,
    `primary`, `round_count` and `finding_count` untouched.
- **Answer-not-found**
  - The model judges whether the corpus answers the question and states that
    judgement with the answer. The user may reject an answer as unsatisfactory,
    which is treated as not answered.
  - When not answered, the verb proposes focus areas framed by the question,
    each with profiles drawn from the brief's `source_profiles`, within the
    breadth ceiling.
  - On explicit confirmation the proposal is written into `outline.md` under
    `outline`'s append-or-revise rule, and a `synthesised` or `complete` set
    is reopened to `researching` on the final manifest edit, exactly as
    `outline` does. Without confirmation nothing is written.
  - The verb then names the next steps: `conduct SLUG`, `synthesise SLUG`,
    then the same `ask` or `report` again.
- **Contract**
  - A `report` row in the kind-discriminated schema, a
    `templates/topic-research-report.md` resolving through the 3-tier
    override, its catalogue entry, and the template injected into the skill.
  - `argument-hint` and the skill description gain both verbs.

## Acceptance Criteria

- [ ] Given a set with findings, when the user runs `ask SLUG "<question>"`,
      then the answer appears in conversation and no file in the set changes.
- [ ] Given a set in `outlined` with no findings, when `ask` runs, then it
      refuses naming `researching`, `synthesised` and `complete`.
- [ ] Given a `researching` set, when `report` runs, then it refuses and points
      to `synthesise SLUG`.
- [ ] Given a `synthesised` set at `round_count: 3`, when `report SLUG
      "<question>"` runs, then `reports/<slug>.md` validates with `kind:
      report`, the question, and `rounds_covered: 3`; the manifest's
      `report_count` is 1; and its `status` and `primary` are unchanged.
- [ ] Given an existing report for a question, when `report` runs again with
      the same question after another round, then the same file is rewritten
      with the new `rounds_covered` and `report_count` is unchanged.
- [ ] Given two different questions whose slugs collide, when both are
      reported, then two report files exist and neither overwrites the other.
- [ ] Given a focus area with `web` and `openalex` findings, when `ask` or
      `report` answers from it, then both inform the answer, and a report's
      Sources carry each tier unchanged, including any `(retracted)` or
      `(withdrawn)` suffix.
- [ ] Given a focus area whose `openalex` finding reads `None found.`, when the
      question turns on it, then the answer reports no scholarly evidence
      rather than proposing to research that profile again.
- [ ] Given a finding with a `.levels/` tree, when either verb runs, then no
      level note is read.
- [ ] Given a question the corpus does not answer, when either verb runs, then
      it says so and proposes focus areas, and `outline.md` is unchanged until
      the user confirms.
- [ ] Given a confirmed proposal on a `complete` set, when it is written, then
      `outline.md` gains the focus areas under the append-or-revise rule and
      the manifest's `status` is `researching`.
- [ ] Given an answer the user rejects as unsatisfactory, when they say so,
      then the verb proposes focus areas as for an unanswered question.

## Open Questions

- Does `report` write a report when the corpus has nothing relevant at all, or
  only propose research? Recommendation: write nothing and propose; a report
  with no supported answer is not worth citing.

## Dependencies

- Blocked by: 0277 (findings and synthesis to read), 0279 (outline appending
  and reopen, which a confirmed proposal relies on). Both are done.
- Blocks: 0284, which renders reports; until it lands, a report is reachable
  on disk but not in the visualiser, which indexes one entry per set.
- Related: 0280 defines the multi-finding layout, the `— profiles:` suffix,
  and the tier suffixes these verbs read.
- Related: 0283 adds `.levels/` trees, which these verbs never read.
- Related: 0347 adds `redo`, `revisit` and `deepen` items. Whichever of 0281
  and 0347 lands second extends the answer-not-found proposal to propose
  `revisit` or `deepen` of an existing finding as well as new focus areas.

## Assumptions

- Reports are many and on-demand, so `report` stays its own verb rather than
  being folded into the singleton `synthesise`.
- A report is regenerated, not immutable; the VCS holds earlier versions.
- `ask` and `report` never start a `conduct` run; research happens only after
  the user confirms a proposal and runs `conduct`.

## Technical Notes

- The schema registry is `cli/corpus/src/frontmatter_validation/schema.rs`
  (19 rows today); the template catalogue is `cli/config/src/catalogue.rs`;
  the visualiser's template-tier map is
  `cli/visualiser/frontend/src/routes/library/template-tier.ts`.
- The visualiser fixture `reports/summary.md` already asserts a report is not
  indexed separately; keep it passing.
- `synthesise` records thin areas as Open Threads in `synthesis.md`, a natural
  input to the answer-not-found judgement.

## Drafting Notes

- `research_kind: report` in the earlier draft became `kind: report`: the
  corpus discriminates on `(type, kind)` and has no `research_kind` field.
- `<handle>` became `SLUG` to match every other verb.
- Visualiser rendering moved to 0284: the indexer creates one entry per set
  from its manifest, so no route reaches a report until 0284's detail page.
- The blocker on 0278 was dropped with that move.
- "Include reports in the manifest" was interpreted as a `report_count`
  derived from disk plus a `## Set` entry; `report_count` is optional in the
  manifest schema so existing manifests stay valid.
- `ask` requires only findings while `report` requires a current synthesis,
  because a citable artefact should reflect a synthesised corpus.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 4)
- Parent epic: 0121
- Related: 0280, 0283, 0284, 0347
