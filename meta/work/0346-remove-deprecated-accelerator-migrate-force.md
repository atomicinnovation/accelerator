---
type: "work-item"
id: "0346"
title: "Remove Deprecated ACCELERATOR_MIGRATE_FORCE"
date: "2026-10-15T18:41:57+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "low"
parent: "work-item:0146"
blocked_by: ["work-item:0302"]
tags: ["migrate", "deprecation"]
last_updated: "2026-10-15T18:41:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-934"
---
# 0346: Remove Deprecated ACCELERATOR_MIGRATE_FORCE

**Kind**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Remove the `ACCELERATOR_MIGRATE_FORCE` override from `accelerator migrate`,
leaving `ACCELERATOR_FORCE` as the only dirty-tree override.

## Context

0302 renames `accelerator migrate`'s dirty-tree override to
`ACCELERATOR_FORCE`, shared with `work rekey`, and keeps
`ACCELERATOR_MIGRATE_FORCE` as a deprecated alias so existing scripts keep
working for a release. This task ends that deprecation.

## Requirements

- `accelerator migrate` no longer honours `ACCELERATOR_MIGRATE_FORCE`.
- Skills, docs, and the migrate skill name only `ACCELERATOR_FORCE`.
- The release notes call out the removal as a breaking change.

## Acceptance Criteria

- [ ] Given a dirty working copy and only `ACCELERATOR_MIGRATE_FORCE=1`, when
      `accelerator migrate` runs, then it refuses and writes nothing.
- [ ] Given a dirty working copy and `ACCELERATOR_FORCE=1`, when
      `accelerator migrate` runs, then it applies.
- [ ] Given the repository, when it is searched for
      `ACCELERATOR_MIGRATE_FORCE`, then only changelog entries mention it.

## Open Questions

- Which release carries the removal.

## Dependencies

- Blocked by: work-item:0302 — Work Item Re-Key Command, which introduces
  `ACCELERATOR_FORCE` and deprecates the old name.

## Assumptions

- At least one release ships with the deprecated alias before removal.

## Technical Notes

- The override is read by the dirty-tree `Preflight`, which 0302 lifts out of
  `cli/migrate`.

## Drafting Notes

- Split out of 0302's review so the deprecated alias has an owner.

## References

- Related: 0302 — Work Item Re-Key Command
