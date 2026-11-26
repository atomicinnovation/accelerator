---
type: "work-item"
id: "PP-997"
external_id: "PP-997"
aliases: ["draft-ge9jfg"]
title: "Stable Focus-Area Identifiers in Topic-Research Sets"
date: "2026-11-23T15:48:59+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:PP-22"
relates_to: ["work-item:PP-868", "work-item:PP-923"]
tags: ["research", "outline"]
last_updated: "2026-11-23T15:48:59+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-997: Stable Focus-Area Identifiers in Topic-Research Sets

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As someone maintaining a research set, I want each focus area to carry an
identifier that does not depend on its wording, so that rewording an outline
question keeps its findings attached instead of orphaning them.

Today a finding is joined to its outline item by question text alone. An
explicit focus-area identifier, written on the outline item and stamped into
each finding and level note, makes the join survive an edit.

## Context

`conduct`'s planner matches a finding to an outline item when their questions
are equal after NFKC folding and whitespace collapsing, and the profiles
match. The rule is exact, but it runs through the wording: editing an outline
question after its findings exist leaves those findings unmatched, the focus
area reads as outstanding again, and the next `conduct` researches it afresh.

Each question already claims an index, the `<nn>` in
`findings/<nn>-<slug>-<profile>.md`, shared by every finding for that
question. The outline item does not carry it, so nothing links the two except
the text.

The set detail page (PP-868) shows findings under their focus areas using the
planner's rule, so it inherits the same weakness: a reworded question shows
its old findings as unmatched.

## Requirements

- Give each focus area an identifier that is stable for the life of the set
  and unique within it.
- Write the identifier on the outline item, in a form the outline grammar
  parses alongside the checkbox and the `— profiles:` suffix.
- Stamp the identifier into the frontmatter of every finding and level note
  written for that focus area.
- Match a finding to its focus area by identifier when both carry one, and by
  the existing question rule otherwise.
- Keep a repeated question in a later round as the same focus area, as the
  planner treats it today.
- Bring sets already on disk onto identifiers without rewriting any finding's
  content.
- Show the set detail page's focus areas by the same join the planner uses.

## Acceptance Criteria

- [ ] Given a set whose outline item and findings carry the same identifier,
      when the outline question is reworded, then the planner reports the
      focus area's pairs as answered and no finding as unmatched.
- [ ] Given the same set, when the outline question is reworded, then the set
      detail page still shows the findings under that focus area.
- [ ] Given a set written before identifiers existed, then the planner and
      the page match its findings by question as they do today.
- [ ] Given a set written before identifiers existed, when it is brought onto
      identifiers, then every outline item and every finding and level note
      carries one, and no finding's body changes.
- [ ] Given two outline items carrying the same identifier, then planning
      reports the duplicate and researches neither item under it.
- [ ] Given a finding whose identifier matches no outline item, then the
      planner reports it as unmatched, naming the finding.
- [ ] Given the same question asked in rounds 1 and 3, then both askings carry
      one identifier.

## Open Questions

- What is the identifier: the existing `<nn>` index promoted to a recorded
  value, or a new token independent of filename order?
- Does rewording a question that has findings warrant a warning, since the
  findings then answer a question the outline no longer asks verbatim?
- Are existing sets migrated by a `migrate` step, or lazily by the next
  `outline` or `conduct` run?

## Dependencies

- Blocked by: nothing.
- Blocks: nothing. PP-868 ships on the question join and adopts identifiers
  when this lands.
- Related: PP-923 (redo, revisit and deepen findings) changes how findings
  relate to focus areas and should agree with this on the join.

## Assumptions

- A reworded question is the same focus area. If rewording is meant to ask
  something new, the right action is a new outline item, and this story does
  not change that.

## Technical Notes

- The join lives in `cli/research/src/topic/plan.rs` (`Finding::answers` in
  `evidence.rs`, `NormalisedQuestion` in `question.rs`).
- The outline grammar is parsed in `cli/research/src/topic/outline.rs`.
- The finding and level-note schemas are in
  `cli/corpus/src/frontmatter_validation/schema.rs`; the templates and the
  finding and level-note outputter skills write the frontmatter.

## Drafting Notes

- Raised while stress-testing PP-868 on 2026-11-23, as the way to replace the
  text join with an explicit link. Drafted as a follow-up, not a blocker, at
  the author's direction.
- The requirement that identifier matching falls back to the question rule is
  my reading of "sets on disk keep working"; a hard cut-over with a mandatory
  migration is the alternative. The fallback was
  confirmed by the author on 2026-11-23.

## References

- Related: PP-868, PP-923
- Parent epic: PP-22
