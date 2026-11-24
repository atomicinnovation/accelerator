---
type: "work-item"
id: "PP-1002"
external_id: "PP-1002"
aliases: ["draft-a228cz"]
title: "Derived Set Response for Topic Research"
date: "2026-11-23T23:08:45+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "ready"
kind: "story"
priority: "high"
parent: "work-item:PP-868"
blocks: ["work-item:PP-1001", "work-item:PP-1000"]
blocked_by: ["work-item:PP-994"]
relates_to: ["work-item:PP-995", "work-item:PP-997", "work-item:PP-923"]
tags: ["research", "visualiser"]
last_updated: "2026-11-24T00:06:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-1002: Derived Set Response for Topic Research

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As a maintainer of the visualiser, I want the server to derive what a
topic-research set page shows beyond its member listing, through the
`research` crate, so that the visualiser states the research domain's
rules by calling them and writes none of its own.

The story adds the visualiser server's first dependency on the `research`
crate, the rules the response needs (five exported as they are, three
written new in that crate), and one response: the **derived set
response**. It has no UI. The topic-research set page (PP-1001) renders
what it returns.

## Context

Child of PP-868 — Topic-Research Set Detail Page. Split from the
topic-research set page (PP-1001) on 2026-11-23, at the author's direction:
the response contract and the crate edge can be reviewed, landed, and
reverted apart from the page, as the set model was split from member routes
at the member API.

The set model (PP-994) indexes a set's members and serves them, each
document with its parsed frontmatter and its set-relative path. It derives
nothing from them. A set page needs more: which findings answer which
outline item, what state each profile is in, whether the synthesis is
stale. The `research` crate already holds those rules, for the planner
that the `conduct` verb runs.

Terms, as PP-1001 defines them. A set holds seven document kinds:
`manifest`, `brief`, `outline`, `finding`, `level-note`, `synthesis`, and
`report`. The **root** is the document `manifest.md`; a **member** is any
other document, so there are six member kinds. The manifest's `status` is
one of five: `briefed`, `outlined`, `researching`, `synthesised`,
`complete`. An **outline item** is one checkbox line
under one `## Round N` heading, with a question, a checkbox state, and
assigned profiles. A **focus area** is a question together with every
outline item that asks it. A **pair** is one outline item with one of its
assigned profiles. A level note's **lineage** is its filename stem,
dash-separated numbers such as `2-1`. A **reopened** set reads
`status: researching` with `primary: synthesis.md`.

## Requirements

### The response

- Serve, for one topic-research set, the derived set response beside the
  member listing. It holds:
  - the outline's rounds, each with its outline items in written order, and
    each item's question, checkbox state, and assigned profiles;
  - the focus areas, each with the findings that belong to it;
  - each pair's state: researched, outstanding, or skipped;
  - the unmatched findings;
  - the unplaced members, each with its reason;
  - each finding's level notes in lineage order;
  - the set's status, or in its place a set-level error;
  - the stale mark on the synthesis and on each report;
  - the reopened mark.
- Carry no document body in the response.
- Derive every item above on the server. The frontend derives none of them.
- Derive the response from the member index as it stands when the request
  arrives, and cache nothing across requests. PP-1001 re-requests it when
  a change event arrives; that is how derived state updates live.

### The rules

- Read an outline item's `— profiles: …` suffix (absent means `web`) as its
  assigned profiles, accepting `—`, `–`, and `--` alike before `profiles:`.
- Place findings through the planner's rule, at two levels. A finding
  **belongs to a focus area** when its question equals the focus area's
  after Unicode compatibility folding (NFKC) and whitespace collapsing. A
  finding **answers a pair** when it belongs to the focus area and its
  `source_profile` is the pair's profile. Both read frontmatter, not
  filenames. Group outline items into focus areas by the same comparison.
- Give a pair's state as researched when a finding answers it, in whichever
  round; otherwise skipped when the brief's `source_profiles` omits the
  profile; otherwise outstanding. A brief with no `source_profiles` skips
  nothing.
- List a finding that belongs to no focus area as unmatched. When
  `outline.md` is missing or cannot be read, every finding is unmatched.
- Place a finding that carries a field or value this story does not know,
  other than `kind`, as any other finding.
- List a member as unplaced, with one of four reasons: unparseable
  frontmatter; a `kind` that is not one of the six member kinds, which
  includes `kind: manifest` on a document that is not the root document;
  no `kind`, which includes a document with no frontmatter at all; or a
  `kind` that does not belong where the document sits, by this layout:

  | Kind         | Belongs at                              |
  |--------------|-----------------------------------------|
  | `brief`      | `brief.md` in the set directory         |
  | `outline`    | `outline.md` in the set directory       |
  | `synthesis`  | `synthesis.md` in the set directory     |
  | `finding`    | `findings/<stem>.md`                    |
  | `level-note` | `findings/<stem>.levels/<lineage>.md`   |
  | `report`     | `reports/<name>.md`                     |
- Order a finding's level notes by lineage: segment by segment as numbers,
  a parent before its children, so `2-2` precedes `2-10`.
- Mark the synthesis stale when its `rounds_covered` is below the
  manifest's `round_count`, or when the set is reopened. Mark a report
  stale when its `rounds_covered` is below `round_count`, and for no other
  reason.
- Report a set-level error, naming the manifest, when `manifest.md` has
  unparseable frontmatter or a `status` outside the five. Under either,
  the manifest is not trusted: the set's status, the stale marks, and the
  reopened mark are absent from the response, which is distinct from
  reporting them as not stale or not reopened. Staleness is not worked out
  from a readable `round_count` beside a bad `status`. Everything else is
  derived as usual: rounds and outline items, focus areas and their
  findings, pair states, unmatched findings, unplaced members, and level
  notes.

### Where the rules live

Every rule above lives in the `research` crate. The visualiser server calls
them and assembles one response; it holds no rule of its own.

- Export five rules that exist: the planner's question comparison, the
  outline grammar, `SetStatus`, freshness, and lineage ordering.
- Write three rules new in `research`, and export them: a pair's state,
  with "skipped" meaning the brief omits the profile; the reopened mark;
  and member placement, which holds the layout table and gives each
  unplaced member its reason.

### The crate edge

- Add the visualiser server's dependency on the `research` crate, as this
  story's first change, so that PP-1000 can build on it.
- Add a table of question comparisons beside the planner's tests, read by
  the planner's test and by the visualiser's.

## Acceptance Criteria

- [ ] Given a `researching` set with `primary: synthesis.md`,
      `round_count: 2`, one round of two outline items each assigning `web`
      and `openalex`, a `web` and an `openalex` finding for the first
      item's question, one finding whose question no item asks, a synthesis
      and one report each with `rounds_covered: 1`, then the response holds
      the round; both items with their checkbox states; `web` and
      `openalex` researched on the first item and outstanding on the
      second; the two findings under the first item's focus area; the third
      finding as unmatched; the set's status; the set as reopened; and the
      synthesis and the report each as stale.
- [ ] Given that set with `status: synthesised` and every `rounds_covered`
      equal to `round_count`, then the response marks nothing stale and
      does not mark the set reopened.
- [ ] Given a reopened set whose synthesis and report each have
      `rounds_covered` equal to `round_count`, then the synthesis is stale,
      the report is not, and the set is reopened; given a `synthesised` set
      with `round_count: 3` whose synthesis and report each have
      `rounds_covered: 2`, then both are stale and the set is not reopened.
- [ ] Given an `openalex` finding whose question matches an outline item
      assigning only `web`, then the response holds it under that focus
      area, not as unmatched, and `web` is outstanding; given two `web`
      findings for one question, then both are held under the focus area.
- [ ] Given a finding with an extra field `state: superseded`, then it is
      placed under its focus area as any other finding.
- [ ] Given outline items suffixed `— profiles: web, arxiv`,
      `– profiles: web, arxiv`, and `-- profiles: web, arxiv`, then each is
      returned assigning `web` and `arxiv`; given an item with no suffix,
      then it is returned assigning `web`.
- [ ] Given an outline item assigning `web`, `openalex`, and `arxiv`, with
      `web` and `openalex` findings, in a set whose brief's
      `source_profiles` omits `arxiv`, then `arxiv` is returned skipped;
      given that set also holds an `arxiv` finding for the question, then
      researched; given a brief with no `source_profiles`, then no pair is
      skipped.
- [ ] Given the same question asked in rounds 1 and 3, then the response
      holds one focus area naming both outline items, and a round-1 `web`
      finding makes `web` researched on both.
- [ ] Given a set with findings and no `outline.md`, then every finding is
      returned unmatched.
- [ ] Given a set holding one well-placed member of each of the six kinds,
      and also a member with unparseable frontmatter, one with
      `kind: manifest` at `findings/m.md`, one with `kind: bogus`, one with
      frontmatter and no `kind`, one with no frontmatter, a `kind: finding`
      document under `reports/`, and a `kind: level-note` document at
      `findings/n.md`, then each of those seven is returned unplaced with
      its own one of the four reasons, and none of the six well-placed
      members is.
- [ ] Given a finding with level notes `1`, `2`, `2-1`, `2-2`, and `2-10`,
      then the response lists them in that order; given a finding with no
      `<stem>.levels/` directory, then it lists none.
- [ ] Given a set whose `manifest.md` has unclosed frontmatter, or a
      `status: bogus` beside `round_count: 3` and a synthesis with
      `rounds_covered: 2`, then in each case the response carries a
      set-level error naming the manifest; holds no status, no stale mark
      for the synthesis or any report, and no reopened mark; and still
      holds the outline's rounds, the focus areas, the pair states, the
      unmatched findings, the unplaced members, and the level notes.
- [ ] Given members whose bodies each hold a marker string unique to that
      member, then no marker appears in the response.
- [ ] This story adds a table of question comparisons beside the planner's
      tests. Each row holds two questions and whether they compare equal:
      a whitespace-only difference and a compatibility-form difference,
      equal; a case difference and a punctuation difference, not equal; and
      a compatibility-form punctuation mark, such as a full-width `？`
      against `?`, equal. The planner's test and the visualiser's test both
      read that one file and assert its stated values, and the response
      places a finding under a focus area exactly when the row says its
      question and the focus area's compare equal.
- [ ] The visualiser server compiles each of the eight rules from the
      `research` crate's public API, and that crate's public-API snapshot
      diff holds additions only. The dependency lint passes with the
      visualiser server depending on `research`.
- [ ] The visualiser server's source holds no parser for `## Round`
      headings or `profiles:` suffixes, and no table of kinds and
      directories.

## Open Questions

- The response's shape (planning-time): its endpoint and fields. PP-994's
  member API shape is settled first and leaves room for it, a constraint
  PP-994's own open question records; this story's planning then settles
  the shape. PP-1001 consumes the answer.

## Dependencies

- Blocked by: PP-994, the set model, for the member index (each document's
  parsed frontmatter, its malformed or absent mark, and its set-relative
  path, even under a malformed root) and for the member API shape this
  response sits beside. The server reads `outline.md`'s body itself,
  through the file driver the document endpoint uses.
- Planning order: planned once PP-994's member API shape is settled and
  its ADR accepted. This story's response shape is its first deliverable
  and releases PP-1001's planning.
- Blocks: PP-1001, the topic-research set page, which renders the
  response; and PP-1000, Sources rendering, which calls the Sources parser
  across the crate edge this story adds.
- Shared file: the Sources parser (PP-995) changes the same `research`
  public-API snapshot; whichever lands later regenerates it.
- On-disk contract: the rules read what the research-topic skill and its
  outputters write, per the set contract in
  `skills/research/research-topic/SKILL.md`: the directory layout,
  `## Round N` headings and the `— profiles:` suffix in `outline.md`,
  `<stem>.levels/<lineage>.md` level notes, and the frontmatter fields
  `kind`, `status`, `primary`, `round_count`, `rounds_covered`,
  `source_profiles`, `source_profile`, and `question`. A change to any of
  them must be matched here.
- Follow-up, not a blocker: PP-997 (stable focus-area identifiers) replaces
  the question-text join with an explicit identifier, and with it retires
  or rewrites the table of question comparisons and its criterion.
- Related, still draft: PP-923 may add finding states and change how
  findings relate to focus areas. A change to that relation must be
  matched in this response's placement rules.

## Assumptions

- "Skipped" is a profile the brief's `source_profiles` omits. The planner
  also skips a profile that is not installed; the server cannot see that.
- A reworded outline question leaves its earlier findings unmatched, as
  `conduct` sees them, until focus areas carry identifiers.
- Quarantined files and the run ledger are dot-prefixed, so the set model
  never indexes them and this response never names them.

## Technical Notes

**Size**: L — one new crate edge, five rules exported, three written new
in `research`, and one response assembled from them.

- Lineage ordering is in `cli/research/src/topic/layout/lineage.rs`, and
  the directory names `findings` and `reports` in
  `cli/research/src/topic/layout.rs`. The planner's outstanding and
  skipped pairs are in `cli/research/src/topic/plan.rs`; its "skipped"
  also covers uninstalled profiles, which is why pair state is written
  new. No reopened mark and no placement rule were found in the crate.
  These were read from file names and signatures, not from the logic.

- The visualiser server does not depend on `research` today
  (`cli/visualiser/server/Cargo.toml:24-33`). The dependency lint permits a
  composition root to depend on a domain crate
  (`tasks/lint/crate_dependencies.py:392-397`).
- The planner's join: `NormalisedQuestion::of` in
  `cli/research/src/topic/question.rs`, `Finding::answers` in
  `cli/research/src/topic/evidence.rs:102`, and `Warning::UnmatchedFinding`
  in `cli/research/src/topic/plan.rs:111`.
- The outline grammar is parsed in `cli/research/src/topic/outline.rs`; set
  status is `SetStatus` in `cli/research/src/topic/lifecycle.rs`; freshness
  is derived in `cli/research/src/topic/counts.rs:154-180`.
- The public-API snapshot is `cli/research/tests/fixtures/public-api.txt`.
- A full sample set with level notes and reports is at
  `skills/research/research-topic/evals/project/meta/research/topics/llm-agent-planning/`.

## Drafting Notes

- Split from PP-1001 on 2026-11-23, at the author's direction, after a
  review of PP-1001 noted that the server and crate slice has no UI and so
  escapes the objection to an interim page shell.
- The requirements and criteria here were PP-1001's Server section and the
  rules it stated for the page, restated on the response. PP-1001 keeps
  each as a criterion on what the page shows.
- A review on 2026-11-23 found the draft sized on "five exports of rules
  that exist" while requiring rules outside them. The author settled it:
  every rule lives in `research`, three of them written new by this story,
  which grew from M to L. The layout table was drafted from the set
  contract and PP-1001's wording and is to be checked at planning. The
  review also separated the staleness cases, added placement criteria, and
  stated that the response is derived per request. The author also
  settled the response under a set-level error: whatever reads the
  manifest is absent, for both error inputs.
- This story owns the crate edge outright. Before the split it fell to
  PP-1001 or PP-1000, whichever landed first.

## References

- Parent: PP-868, under the epic PP-22
- Blocked by: PP-994 (set model)
- Blocks: PP-1001 (set page), PP-1000 (Sources rendering)
- Related: PP-995 (Sources parser), PP-997 (stable focus-area
  identifiers), PP-923
- Set contract: `skills/research/research-topic/SKILL.md`
