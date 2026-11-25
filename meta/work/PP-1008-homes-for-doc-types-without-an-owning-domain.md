---
type: "work-item"
id: "PP-1008"
external_id: "PP-1008"
aliases: ["draft-gbacrw"]
title: "Homes for Doc Types Without an Owning Domain"
date: "2026-11-25T23:20:34+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "spike"
priority: "medium"
parent: "work-item:PP-1006"
relates_to: ["work-item:PP-790", "work-item:PP-173"]
tags: ["cli", "architecture", "doc-types"]
last_updated: "2026-11-25T23:20:34+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-1008: Homes for Doc Types Without an Owning Domain

**Kind**: Spike
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Decide which context owns each doc type that has no domain crate today —
decisions, plans, plan reviews, validations, PR descriptions, PR reviews
and notes — and how templates separate from the corpus, so that every doc
type can be declared by its owner under ADR-0080.

## Context

ADR-0080 lets each context declare its doc types in its own declarations
crate. Seven doc types and templates have no context to do so. The author
leans towards a decisions vertical, with a CLI that owns the ADR commands
(PP-790), a planning context, and a notes context. PR documents could
belong to `collaboration`, whose `github` crate is an adapter today, or to
a new `github` context. Templates have no root directory and are the
exception in every doc type rule.

## Requirements

Research questions, with the author's leanings as starting hypotheses:

1. Decisions: the shape of a decisions vertical (domain, declarations, CLI
   owning the ADR commands), and how it absorbs PP-790.
2. Plans, plan reviews and validations: confirm a planning context, and
   its smallest viable shape.
3. PR descriptions and PR reviews: the `collaboration` context or a new
   `github` context.
4. Notes: confirm a notes context, and its smallest viable shape.
5. Templates: how they separate from the corpus, and what the visualiser
   and configuration see in place of a doc type.
6. Whether any doc type still needs a lightweight registration mechanism,
   short of a full vertical, once 1–5 are settled.

Time-box: 2 days.

## Acceptance Criteria

- [ ] An ADR records the owning context of every doc type listed above, and
      how templates are held.
- [ ] A follow-up work item exists for each new vertical or move, under
      this spike's parent epic.
- [ ] Question 6 is answered yes or no, with its reason.

## Open Questions

- None beyond the research questions.

## Dependencies

- Blocked by: none. The ADR can be written before PP-994 lands.
- Blocks: the follow-up items it raises.

## Assumptions

- A context with only declarations and no behaviour yet (such as notes)
  is acceptable under ADR-0069 and ADR-0080.

## Drafting Notes

- The author's leanings are recorded as hypotheses for the spike to
  confirm, not as decisions.

## References

- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
- `meta/decisions/ADR-0080-domain-declarations-crates.md`
- `meta/work/PP-790-move-adr-to-own-subcommand.md`
- `meta/work/PP-173-collaboration-pr-skills.md`
- `meta/work/PP-925-move-the-work-context-s-ports-out-of-work-adapters.md`
