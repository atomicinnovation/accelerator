---
work_item_id: "0030"
title: "Centralise PATH_DEFAULTS to scripts/config-defaults.sh"
date: "2026-04-26T00:00:00+00:00"
author: Toby Clemson
type: task
status: proposed
priority: low
parent: ""
tags: [config, refactoring]
---

# 0030: Centralise PATH_DEFAULTS to scripts/config-defaults.sh

**Type**: Task
**Status**: Proposed
**Priority**: Low
**Author**: Toby Clemson

## Summary

The `PATH_KEYS` and `PATH_DEFAULTS` arrays are duplicated across approximately
15 call sites. Extracting them into a single sourced file would make the next
default rename a one-line edit rather than a 15-site grep-and-replace.

## Context

During the `0001-rename-tickets-to-work` migration (tracked in
`meta/plans/2026-04-25-rename-tickets-to-work-items.md`), `paths.tickets` /
`paths.review_tickets` had to be updated in `scripts/config-dump.sh`,
`scripts/config-read-path.sh`, 11 SKILL.md files, and 2 helper scripts — 15
sites in total. A single canonical defaults file would have reduced that to one
edit plus a source line everywhere else.

## Requirements

- Extract `PATH_KEYS` and `PATH_DEFAULTS` arrays into
  `scripts/config-defaults.sh` (new file).
- Replace all 15 duplicated definitions with `source
  "${CLAUDE_PLUGIN_ROOT}/scripts/config-defaults.sh"` (or equivalent).
- Ensure no change in observable behaviour: all existing config-dump, init, and
  path-resolution tests continue to pass.

## Acceptance Criteria

- [ ] `scripts/config-defaults.sh` exists and defines `PATH_KEYS` and
      `PATH_DEFAULTS`.
- [ ] No other file in the repo defines its own copy of those arrays.
- [ ] `mise run test` is green.

## Open Questions

- Should `TEMPLATE_KEYS` and `TEMPLATE_DEFAULTS` be centralised in the same
  file, or handled separately?

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- All 15 call sites are bash scripts that can safely `source` a shared file.

## Technical Notes

- Reference: `scripts/config-dump.sh:174-200` and `scripts/config-read-path.sh:17`
  for the current duplicate definitions.

## Drafting Notes

- Scope is intentionally limited to `PATH_KEYS`/`PATH_DEFAULTS`. Template keys
  are a separate concern and should be a separate work item if tackled.

## References

- Source: `meta/plans/2026-04-25-rename-tickets-to-work-items.md`
- Related: `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
