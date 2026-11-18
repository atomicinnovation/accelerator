---
type: "work-item"
id: "0378"
title: "Check the Sync Skill's Membership Table Against the Engine's Row Shapes"
date: "2026-11-16T11:17:16+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0372"
blocked_by: ["work-item:0373", "work-item:0377"]
relates_to: ["work-item:0352"]
tags: ["sync", "sync-work-items", "skill", "tests"]
last_updated: "2026-11-16T11:17:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0378: Check the Sync Skill's Membership Table Against the Engine's Row Shapes

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Nothing ties the rows `work sync` can print to the rules in
`/sync-work-items`. Add a fixture of the row shapes the engine prints and
check the skill's Membership table against it, so a row with no rule fails a
test instead of reaching a user.

## Context

The skill structure test scans the skill's Markdown. It reads failure
classes, operations, refusals and unidentified kinds from
`sync-vocabulary.golden`, but no combination of action, state and detail.
That is how `noop\tunsynced\tunsupported-recipe` came to be pinned by engine
tests and absent from the skill with every check green.

Later items in this epic change row shapes on purpose, which makes the gap
more likely to matter.

## Requirements

- A generated fixture lists every row shape the engine can print: action,
  state and the class of detail.
- An engine test fails when the fixture is stale.
- The skill structure test fails when a shape in the fixture matches no
  dispatch entry or Membership rule, or matches a rule whose group is
  ambiguous.

## Acceptance Criteria

- [ ] Given a new row shape printed by the engine, when the tests run, then
      the fixture test fails until the fixture is regenerated.
- [ ] Given a fixture shape with no Membership rule, when the skill
      structure tests run, then one fails and names the shape.
- [ ] Every shape printed today matches a rule, with the gaps fixed by 0373.

## Open Questions

- Step 5 has groups no Membership rule feeds (`pushed`, `pulled`,
  `pushed-unsynced`, `unsynced (not pushed)`). Should the table become
  exhaustive, or should the check allow shapes that Step 5 places by name?

## Dependencies

- Blocked by: 0373, 0377.
- Blocks: 0382.

## Assumptions

- The set of row shapes is finite and can be enumerated from the engine's
  types, as the vocabulary fixture already is.

## Technical Notes

- `cli/work-adapters/tests/fixtures/sync-vocabulary.golden` is the pattern
  to follow.

## Drafting Notes

- Scoped to the skill's table. The user guide's description of the report
  is not covered.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
