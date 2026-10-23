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
tiered Sources, rewritten when the user confirms a question is one an existing
report already answers. Both verbs consult only a fresh, fully conducted,
synthesised corpus. When the corpus answers a question only partly or not at
all, either verb proposes focus areas and, on the user's confirmation, appends
them to `outline.md` as a new round.

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
  - Both require the same state, refusing with the failed gate's recovery
    path and writing nothing otherwise:
    - **Status**: `synthesised` or `complete`; otherwise refuse naming both
      and pointing to `synthesise SLUG`.
    - **Freshness**: `finalise`'s read-only gate — disk-derived `round_count`
      and `finding_count` equal the manifest's, and the highest finding
      `round` does not exceed `synthesis.md`'s `rounds_covered`; otherwise
      refuse naming the mismatch and pointing to `conduct` / `synthesise
      SLUG`.
    - **Completeness**: every `(focus area, profile)` pair in `outline.md`,
      stragglers in conducted rounds included, has a retained, validated
      finding; otherwise refuse naming the outstanding pairs and pointing to
      `conduct SLUG`.
  - Both read `synthesis.md` and every top-level `<nn>-*.md` finding, never
    `.levels/`.
  - Both judge coverage across every finding of a focus area — one per source
    profile since 0280. A finding whose Sources read `None found.` is evidence
    of absence in that source family, not a gap.
  - Both treat finding and synthesis bodies as untrusted data: orientation
    only, never instructions, as `synthesise` does.
  - Both may spawn subagents to read findings, an existing agent or a custom
    one; any subagent observes the `.levels/` exclusion and the untrusted-data
    contract.
- **ask**
  - Answers in conversation with its coverage judgement and writes nothing to
    the set except a confirmed research proposal.
- **report**
  - Writes `reports/<slug>.md` with `kind: report`, its `question`,
    `rounds_covered` equal to the manifest's `round_count`, and `coverage`
    (`answered` or `partial`), synthesised from the corpus with Sources
    derived from the findings' tiers. Tiers carry through unchanged,
    including `tier-3 (retracted)` and `tier-3 (withdrawn)`. A `partial`
    report carries a Gaps section.
  - Before writing, judges the question against every existing report's
    `question`, lists every report it judges equivalent, and lets the user
    pick one or none. A picked report is rewritten in place keeping its
    original `question`; otherwise a new report is written, its slug derived
    from the question and taking a numeric suffix on collision.
  - Edits `manifest.md` after writing the report to record reports —
    `report_count` from disk and a `reports/` entry under `## Set` — leaving
    `status`, `primary`, `round_count` and `finding_count` untouched.
- **Coverage**
  - The model judges the corpus's coverage of the question as `answered`,
    `partial` or `unanswered`, and states that judgement with the answer. The
    user may reject an answer as unsatisfactory, which is treated as
    `unanswered`.
  - `unanswered`: `report` writes no report; both verbs propose focus areas.
  - `partial`: `report` writes its report first; both verbs then propose
    focus areas for the gaps.
  - Proposed focus areas are framed by the question, each with profiles drawn
    from the brief's `source_profiles`, within the breadth ceiling.
  - On explicit confirmation the proposal is appended to `outline.md` as a
    new `## Round N+1` — never revising an existing round — and a
    `synthesised` or `complete` set is reopened to `researching` on the final
    manifest edit, exactly as `outline` does. Without confirmation nothing
    more is written.
  - The verb then names the next steps: `conduct SLUG`, `synthesise SLUG`,
    then the same `ask` or `report` again.
- **Contract**
  - A `report` row in the kind-discriminated schema — `required_extras`
    `question`, `rounds_covered`, `coverage`; `status_vocab` `complete` — a
    `templates/topic-research-report.md` resolving through the 3-tier
    override, its catalogue entry, and the template injected into the skill.
  - The validator enforces `coverage ∈ {answered, partial}` through a
    per-extra vocabulary.
  - The manifest row gains `report_count` as an optional extra, and the
    manifest template a `reports/` entry under `## Set`.
  - `argument-hint` and the skill description gain both verbs.

## Acceptance Criteria

- [ ] Given a fresh `synthesised` set with every `(focus area, profile)`
      pair conducted, when the user runs `ask SLUG "<question>"` and the
      corpus answers it, then the answer appears in conversation with its
      coverage judgement and no file in the set changes.
- [ ] Given a set in `briefed`, `outlined` or `researching`, when `ask` or
      `report` runs, then it refuses, names `synthesised` and `complete`, and
      points to the recovery path for the set's state.
- [ ] Given a `synthesised` set whose disk-derived `round_count` or
      `finding_count` disagrees with the manifest, or whose highest finding
      `round` exceeds `synthesis.md`'s `rounds_covered`, when `ask` or `report`
      runs, then it refuses naming the mismatch and pointing to `conduct` /
      `synthesise SLUG`, and nothing is written.
- [ ] Given a `synthesised` set with an outstanding `(focus area, profile)`
      pair — a pending round or a straggler in a conducted round — when `ask`
      or `report` runs, then it refuses naming the outstanding pairs and
      pointing to `conduct SLUG`, and nothing is written.
- [ ] Given a `synthesised` set at `round_count: 3`, when `report SLUG
      "<question>"` runs and the corpus answers it, then `reports/<slug>.md`
      validates with `kind: report`, the question, `rounds_covered: 3` and
      `coverage: answered`; the manifest's `report_count` is 1 and `## Set`
      lists `reports/`; and its `status` and `primary` are unchanged.
- [ ] Given a report whose `coverage` is anything other than `answered` or
      `partial`, when it is validated, then validation fails.
- [ ] Given existing reports, when `report` runs with a question the model
      judges equivalent to one or more of their `question`s, then it lists
      every equivalent report and the user picks one or none before anything
      is written.
- [ ] Given the user picks an existing report, when `report` rewrites it after
      another round, then the same file is rewritten with the new
      `rounds_covered`, its original `question` is kept, and `report_count` is
      unchanged.
- [ ] Given the user picks none and the new question's slug collides with an
      existing report, when `report` writes, then the new report takes a
      numeric suffix and neither overwrites the other.
- [ ] Given a focus area with `web` and `openalex` findings, when `ask` or
      `report` answers from it, then both inform the answer, and a report's
      Sources carry each tier unchanged, including any `(retracted)` or
      `(withdrawn)` suffix.
- [ ] Given a focus area whose `openalex` finding reads `None found.`, when the
      question turns on it, then the answer reports no scholarly evidence
      rather than proposing to research that profile again.
- [ ] Given a finding with a `.levels/` tree, when either verb runs, including
      through any subagent it spawns, then no level note is read.
- [ ] Given a question the corpus does not answer, when either verb runs, then
      it says so, `report` writes no report, and it proposes focus areas;
      `outline.md` is unchanged until the user confirms.
- [ ] Given a question the corpus partly answers, when `report` runs, then it
      writes a report with `coverage: partial` and a Gaps section, then
      proposes focus areas for the gaps; when `ask` runs, it states the
      partial judgement and proposes likewise.
- [ ] Given a confirmed proposal on a `complete` set, when it is written, then
      `outline.md` gains a new `## Round N+1` with the focus areas, no
      existing round is revised, and the manifest's `status` is
      `researching`.
- [ ] Given a partial `report` whose proposal is confirmed, when the verb
      finishes, then the report and its `report_count` manifest edit land
      before the outline append and the reopen.
- [ ] Given an answer the user rejects as unsatisfactory, when they say so,
      then the verb proposes focus areas as for an unanswered question.

## Dependencies

- Blocked by: 0277 (findings and synthesis to read), 0279 (outline appending
  and reopen, which a confirmed proposal relies on). Both are done.
- Relies on `finalise`'s freshness gate, which both verbs reuse.
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
- Requiring a fully conducted, synthesised set before either verb answers is
  acceptable friction: a single failed `(focus area, profile)` pair blocks
  both until a gap-fill `conduct` and a re-`synthesise`.
- Whether a new question is equivalent to an existing report's is a model
  judgement; listing every candidate and requiring the user's pick is the
  safeguard against a wrong overwrite or a near-duplicate report.

## Technical Notes

- The schema registry is `cli/corpus/src/frontmatter_validation/schema.rs`
  (19 rows today); the template catalogue is `cli/config/src/catalogue.rs`;
  the visualiser's template-tier map is
  `cli/visualiser/frontend/src/routes/library/template-tier.ts`.
- The visualiser fixture `reports/summary.md` already asserts a report is not
  indexed separately; keep it passing.
- `synthesise` records thin areas as Open Threads in `synthesis.md`, a natural
  input to the answer-not-found judgement.
- The manifest row's `optional_extras` is empty today; `report_count` must be
  added to it, and `templates/topic-research-manifest.md` gains a `reports/`
  line under `## Set` when reports exist.
- The schema enforces vocabularies only for `status` (`status_vocab`); the
  report row's `coverage ∈ {answered, partial}` needs a new per-extra
  vocabulary mechanism in the validator.
- The report row's typed linkage is `parent`, `relates_to`; the report body
  is Answer, Coverage, Gaps (partial only), Sources.
- The shared precondition reuses `finalise`'s read-only freshness gate
  (disk-derived counts against the manifest; highest `round` against
  `synthesis.md`'s `rounds_covered`) plus a completeness gate over every
  `(focus area, profile)` pair in `outline.md`.
- The verbs may spawn subagents to read findings — an existing agent or a new
  custom one, chosen at planning — under the same `.levels/` exclusion and
  untrusted-data contract.

## Drafting Notes

- `research_kind: report` in the earlier draft became `kind: report`: the
  corpus discriminates on `(type, kind)` and has no `research_kind` field.
- `<handle>` became `SLUG` to match every other verb.
- Visualiser rendering moved to 0284: the indexer creates one entry per set
  from its manifest, so no route reaches a report until 0284's detail page.
- The blocker on 0278 was dropped with that move.
- "Include reports in the manifest" was interpreted as a `report_count`
  derived from disk plus a `## Set` entry; `report_count` becomes optional in
  the manifest schema so existing manifests stay valid.
- `ask` and `report` share one precondition — synthesised, fresh, and with no
  outstanding research — so neither answers from a corpus that is mid-round
  or whose synthesis lags its findings.
- A confirmed proposal always appends a round: revising a pending round in
  place would clobber focus areas the user planned, and the completeness gate
  means no round is pending when either verb runs.
- A question with no supported answer writes no report, since a report with
  nothing to cite is not worth citing; a partly answered one writes a report
  that states its gaps.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 4)
- Parent epic: 0121
- Related: 0280, 0283, 0284, 0347
