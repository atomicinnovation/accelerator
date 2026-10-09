---
type: "work-item"
id: "0318"
title: "Give verify_applied Read Access to the Corpus"
date: "2026-10-09T09:34:53+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0312"
relates_to: ["work-item:0202", "work-item:0119"]
tags: ["migration-engine", "m0007", "interactive-contract", "resume"]
last_updated: "2026-10-09T09:34:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0318: Give verify_applied Read Access to the Corpus

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As someone resuming an interrupted interactive migration, I want an accepted
or edited decision whose change has since been lost to be asked again, so
that resume never silently skips a change that is not on disk.
`verify_applied` exists for this, but it receives no `MigrationContext`, so
no migration can read the artefact it is meant to verify.

## Context

The ADR-0037 successor records `verify_applied`'s purpose: confirming on
resume that a recorded accept or edit is still applied. Its signature
(`cli/migrate/src/interactive.rs:78-87`) takes only the transformation and
the record. 0172's plan called m0007 keeping the default `true` a "disclosed
narrowing" of bash, whose equivalent re-read the target file (plan
L2274-2281).

## Requirements

- Pass read-only corpus access to `verify_applied` at both engine call
  sites (`engine.rs:49`, `engine.rs:141`).
- Implement it in m0007. A single-valued key must hold the recorded value;
  a list-valued key must contain it. The recorded value is `user_value` for
  `edited` and `proposed_value` for `accepted`.
- Update `skills/config/migrate/SKILL.md`'s callback contract to match.

## Acceptance Criteria

- [ ] Given an accepted m0007 record whose linkage was removed from the
      document by hand, when the run resumes, then that transformation is
      prompted again.
- [ ] Given an accepted m0007 record whose linkage is still present, when
      the run resumes, then it replays silently with no prompt.
- [ ] Given an edited record, when the run resumes, then verification checks
      `user_value`, not `proposed_value`.
- [ ] Given a skipped record, when the run resumes, then `verify_applied` is
      not called.

## Open Questions

None.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- Read-only access is enough. `verify_applied` stays in the repeatable,
  side-effect-free set, so it must not receive a context that can write.

## Technical Notes

- `MigrationContext` combines reads (`read`, `parse_frontmatter`) with
  writes. Splitting out a read-only view keeps the purity rule enforceable
  by type rather than by convention.

## Drafting Notes

- Kind is `story`: this restores a user-facing resume guarantee, not a
  defect in shipped behaviour.

## References

- `meta/plans/2026-08-07-0172-migration-engine-subdomain.md` (L2274-2281)
- Related: 0202, 0119
