---
type: "work-item"
id: "0309"
title: "Keep Follow-up Questions Answerable in the Node's Source Profile"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0121"
relates_to: ["work-item:0280", "work-item:0283", "work-item:0304"]
tags: ["research", "deep-research", "profiles", "openalex"]
last_updated: "2026-10-09T17:58:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0309: Keep Follow-up Questions Answerable in the Node's Source Profile

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

At depth above 1, a node's follow-up questions become the next level's
nodes, researched in the pair's source profile, but nothing asks the
researcher to frame follow-ups the profile can answer. `openalex` trees
drift into practitioner and implementation questions the scholarly
literature cannot answer, ending as "None found" or filled with uncited
background knowledge. Each profile will state what it can answer,
follow-ups must stay within that, and every profile will forbid uncited
claims.

## Context

A depth-3 run with two `openalex` pairs produced 26 notes, including:

- **Unanswerable nodes**: `04-openalex:3-4-1` asked whether psycopg 3
  buffers notifications that arrive outside its `notifies` generator, and
  ended "None found"; `08-openalex:2-1` asked for PostgreSQL NOTIFY
  commit-serialisation benchmarks, with no OpenAlex records.
- **Background-knowledge fill**: several openalex notes stated
  PostgreSQL-specific behaviour from general knowledge — planner `max()`
  rewrite and sequence semantics (3-1-2), lock release timing (3-2-2),
  notification coalescing (3-3-2) — flagged as inference.

Web-profile pairs in the same run answered the same questions from primary
sources.

The only rule against writing from memory is in the web profile, scoped to
its Denied outcome. The openalex and arxiv profiles say nothing about
background knowledge, so those notes broke no rule — there was none. Trees
stay single-profile: a pair is researched in one profile throughout.

## Requirements

- **Profile answerability statement**: each source profile (`web`,
  `openalex`, `arxiv`) states, in one short section, what kinds of question
  it can answer and which it cannot. For example, `openalex` answers what
  the peer-reviewed literature has studied or measured, not how a specific
  library or release behaves; `web` answers practice, tooling, standards and
  primary-source documentation.
- **Follow-up constraint**: the level-note outputter requires each follow-up
  to be a question the node's source profile can answer, as that profile's
  statement defines. A question outside it is not recorded as a follow-up.
- **No background knowledge**: every profile states that a note or finding
  contains only what a cited source supports; a claim the researcher cannot
  cite is left out, or posed as a follow-up if the profile could answer it.
  This holds for every outcome, not only Denied.

## Acceptance Criteria

- [ ] Each of the `web`, `openalex` and `arxiv` profiles has a section
      stating which questions it can and cannot answer, with at least one
      example of each.
- [ ] The level-note outputter's Shape section requires every follow-up to
      be answerable by the node's source profile, referring to the profile's
      statement.
- [ ] Each of the `web`, `openalex` and `arxiv` profiles states that only
      claims supported by a cited source may be written, and that uncited
      claims are left out rather than flagged as inference.
- [ ] The web profile's existing "Never write the document from memory" rule
      is generalised rather than duplicated, so no profile states the rule
      twice.

## Open Questions

- Should the composer contract (`agents/composer.md`) also drop any uncited
  claim it finds in a note, as a second line of defence?

## Dependencies

- Blocked by: none known
- Blocks: none known
- Related: 0304, which takes the same no-fallback stance for web nodes.

## Assumptions

- Wording in the outputter and profiles suffices to steer follow-ups; no
  CLI-side answerability check is expected.

## Technical Notes

- Follow-up Shape rules:
  `skills/research/outputters/level-note-outputter/SKILL.md:38-50`.
- Existing memory rule: `skills/research/profiles/web-profile/SKILL.md:62-65`.
- Profiles: `skills/research/profiles/{web,openalex,arxiv}-profile/SKILL.md`.

## Drafting Notes

- Cross-profile hand-off was ruled out by the user; trees stay
  single-profile.
- Leaving background knowledge out (rather than a labelled section) was a
  user decision, consistent with 0304's no-fallback stance.
- Verification is structural only, at the user's direction; effectiveness
  is judged on the next real run.
- Extended the answerability statement to `arxiv`, which the issue does not
  mention, because it has the same drift risk.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/146
- Related: 0121, 0280, 0283, 0304
