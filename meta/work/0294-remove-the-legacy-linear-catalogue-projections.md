---
type: "work-item"
id: "0294"
title: "Remove the Legacy Linear Catalogue Projections"
date: "2026-09-23T22:48:52+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "low"
parent: "work-item:0146"
blocked_by: ["work-item:0292"]
derived_from: ["plan:2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids"]
relates_to: ["work-item:0229", "work-item:0292"]
external_id: "PP-878"
tags: ["linear", "catalogue", "sync", "cleanup"]
last_updated: "2026-09-23T22:48:52+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0294: Remove the Legacy Linear Catalogue Projections

**Kind**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Work item 0292 moves Linear's `catalogue.json` onto a single shape: a
`baseTeam` pointer, `teams[]` entries that each carry their states, labels,
members and projects, and top-level workspace `labels`. For one minor release
it also writes the old `team` and `workflowStates` keys, derived from the base
entry, and readers fall back to them. This task removes both once that
release window closes, which leaves one catalogue shape with one reader.

## Context

The projections exist so that three kinds of reader keep working through the
upgrade:

- pre-0292 plugin binaries;
- the `auth.rs` credential fallback, which reads `/team/key` and `/team/id`;
- `report_team_key_writeback`.

The legacy read covers catalogues that no 0292-or-later init or apply sync
has touched. It derives an incomplete base entry from `team` plus
`workflowStates`, and reads 0229 `{key, id, name}` `teams` entries as
entries with no sections. From 0292 onwards, the first apply sync or init
turns every such file into the new shape.

## Requirements

- Stop writing the `team` and `workflowStates` projections in
  `record_team_entries`. An existing file loses both keys on its next write.
- Delete the legacy read in `Catalogue::load`, `TeamStates` and
  `Catalogue::base_team_id`. A file with no `baseTeam` has no base team.
- Move `auth.rs` and `report_team_key_writeback` onto the `baseTeam` entry
  only.
- Delete the tests and fixtures that exist only for the legacy shape, and the
  compatibility-window notes in the `init-linear`, `configure` and
  `sync-work-items` skills.
- `CHANGELOG.md` `[Unreleased]`:
  - `Removed`: the legacy keys and the legacy read.
  - `Migrations`: a catalogue that no 0292-or-later init or apply sync has
    written must be refreshed with `/accelerator:init-linear` first, and
    every teammate must be on 0292 or later.

Out of scope: any other change to the catalogue shape, and removing 0229
`teams` entries that lack sections. The latter is already healed by the
apply sync.

## Acceptance Criteria

- [ ] Given any catalogue write (init or apply-sync finalisation), when it
      completes, then `catalogue.json` has no `team` or `workflowStates` key,
      and every other key is unchanged.
- [ ] Given a catalogue with `baseTeam` and a complete base entry, when
      `transition`, `linear search`, sync and credential resolution run, then
      each behaves as it did before this task.
- [ ] Given a catalogue with `team` and `workflowStates` but no `baseTeam`,
      when a filtered pull or `linear search` runs, then it refuses with
      `E_SEARCH_NO_TEAM` and names `/accelerator:init-linear` as the remedy,
      instead of reading the legacy keys.
- [ ] Given the same legacy catalogue, when credentials resolve without a
      configured `linear.team_key`, then the legacy `/team/key` is not used.
- [ ] Given the codebase after this task, when searched, then no production
      reader of `/team` or `/workflowStates` remains in `cli/`.

## Open Questions

- Which release closes the window? It is one minor release after 0292 ships,
  and that version is unknown until 0292 is released.

## Dependencies

- Blocked by: 0292.
- Blocks: none.

## Assumptions

- One minor release is enough for every repository to have run an apply sync
  or init on 0292 or later, so the legacy read is no longer needed. Any
  repository that hasn't gets a loud refusal with an init remedy, not silent
  degradation.

## Technical Notes

- Writer: `LinearCache::record_team_entries`
  (`cli/linear-client/src/cache.rs`).
- Readers: `Catalogue::load`, `TeamStates` and `base_team_id`
  (`cli/linear-client/src/catalogue.rs`), `auth.rs:126,130`, and
  `linear-cli/src/main.rs` (`report_team_key_writeback`).
- The `CatalogueDocument` fields `team` and `workflow_states` go. An existing
  file that still holds them keeps them as unknown keys until the next write
  drops them, unless the writer is told to drop them explicitly. Decide which
  during planning.

## Drafting Notes

- Treated "the legacy read" as covering both the legacy base (`team` plus
  `workflowStates`) and reading 0229 `{key, id, name}` entries as incomplete.
  The second is kept, because 0292's self-heal needs it for any `teams` entry
  that lacks sections, whatever version wrote it.
- Made the post-removal behaviour for a never-healed file a loud refusal,
  following 0292's rule that refusals are never silent.

## References

- Plan: `meta/plans/2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids.md`
- Related: 0292, 0229, 0146
