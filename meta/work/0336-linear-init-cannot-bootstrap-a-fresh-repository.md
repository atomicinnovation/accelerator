---
type: "work-item"
id: "0336"
title: "Linear Init Cannot Bootstrap a Fresh Repository"
date: "2026-10-14T22:53:18+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
parent: "work-item:0146"
relates_to: ["work-item:0294", "work-item:0228", "work-item:0048"]
external_id: "PP-920"
tags: ["linear", "init", "configuration", "onboarding"]
last_updated: "2026-10-14T22:53:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0336: Linear Init Cannot Bootstrap a Fresh Repository

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`/init-linear` cannot onboard a repository that has no Linear catalogue yet:
every `accelerator linear init` command refuses with `E_CREATE_NO_CATALOGUE`
before it runs, because credential resolution demands a team that only the
catalogue `init discover` writes can supply. `linear.team_key` is never used to
find the team, and the error steers people to set `linear.team_id`, though
`linear.team_key` should be the only team setting a person writes.

## Context

Found on 2026-10-14 while bootstrapping a scratch repository against the `TES`
sandbox team for 0296's manual promotion check. With a valid token and
`linear.team_key: "TES"`, `init verify`, `init list-teams` and `init discover
--team-id <uuid>` each exited `105`. The only way through was to look the team
id up with a hand-written GraphQL query and set `linear.team_id`, which the
error message names as the fix.

`resolve_credentials` resolves a team for every client the CLI builds,
including the three `init` commands whose job is to find and record that team.
`resolve_team` reads `linear.team_id`, then the catalogue's base team, and
fails with `ClientError::NoTeam` when neither exists. Already-onboarded
repositories never notice, since their catalogue is committed.

0228 made `linear.team_key` the canonical, integration-owned scope key, and
`init discover` already writes the discovered key into it. What is missing is
the reverse: bootstrapping from a key a person has already set, and running
`init` at all before a catalogue exists.

## Requirements

Reproduction:

1. In a repository with a Linear token configured, no
   `.accelerator/state/integrations/linear/catalogue.json` and no
   `linear.team_id`, with or without `linear.team_key` set, run
   `accelerator linear init verify`, `init list-teams` or
   `init discover --team-id <uuid>`.
2. Each exits `105` with `E_CREATE_NO_CATALOGUE: no Linear team configured —
   set linear.team_id, or run /init-linear to write catalogue.json`.

Expected: `verify` and `list-teams` succeed, and `discover` writes the
catalogue.

Changes:

- `init verify`, `init list-teams` and `init discover` build their client
  without resolving a team: the first two need only the token, and `discover`
  takes its team from its arguments or from config.
- `init discover` accepts `--team-key <KEY>`, resolving the team id through
  the API from the teams the token can see. With neither `--team-id` nor
  `--team-key`, it uses the configured `linear.team_key`. Given both flags, it
  refuses as a usage error.
- Outside `init`, a missing team still fails, with a message that names
  `/accelerator:init-linear` (or setting `linear.team_key` and running
  `init discover`) and no longer recommends `linear.team_id`.
- `linear.team_id` is deprecated: still read, with a once-per-run deprecation
  warning through the mechanism `work.default_project_code` uses, and removed
  from every message and document that recommends it.
- `/init-linear` skips the team prompt when `linear.team_key` is set, running
  `init discover` with no team flag.
- `CHANGELOG.md` `[Unreleased]`: `Fixed` for bootstrapping a fresh repository,
  `Deprecated` for `linear.team_id`.

Out of scope: resolving the team from `linear.team_key` at runtime for `show`,
`create`, `sync` and every other non-init command, which still need the
catalogue; removing `linear.team_id`; the catalogue shape changes 0294 makes.

## Acceptance Criteria

- [ ] Given a fresh repository with a valid token and no team configured, when
      `init verify` and `init list-teams` run, then each exits 0 with
      `outcome` `verified` and `listed` respectively.
- [ ] Given the same repository, when `init discover --team-id <uuid>` runs,
      then it writes `catalogue.json` and sets `linear.team_key` to the team's
      key.
- [ ] Given `linear.team_key: "TES"` and no catalogue, when `init discover`
      runs with no team flag, then it catalogues TES and leaves
      `linear.team_key` unchanged.
- [ ] Given `--team-key` naming a key the token cannot see, when
      `init discover` runs, then it exits non-zero naming the key and writes
      nothing.
- [ ] Given `--team-key` and `--team-id` together, when `init discover` runs,
      then it refuses as a usage error and writes nothing.
- [ ] Given no catalogue, when `accelerator linear show TES-1` runs, then it
      fails with a message naming `/accelerator:init-linear` that does not
      mention `linear.team_id`.
- [ ] Given `linear.team_id` set, when any Linear command runs, then it still
      resolves that team and prints one deprecation warning.

## Open Questions

- When `linear.team_key` is set but names a team the token cannot see, should
  `/init-linear` fall back to offering the team list rather than stopping on
  `init discover`'s refusal?

## Dependencies

- Blocked by: none.
- Blocks: none.
- Related: 0294 — Remove the Legacy Linear Catalogue Projections moves
  `auth.rs` onto the `baseTeam` entry; whichever lands second adapts its team
  resolution to the other.

## Assumptions

- Only the three `init` commands lose the team requirement; every other
  command keeps it, so a repository still onboards through `/init-linear`.
- A `--team-key` lookup matches the key exactly, ignoring case, among the
  teams the token can see.

## Technical Notes

- Team resolution: `resolve_credentials` and `resolve_team`
  (`cli/linear-client/src/auth.rs:65-97`); the client is built through
  `build_client` and `build_with_override` (`cli/linear-cli/src/context.rs`).
- Error text: `ClientError::NoTeam` (`cli/linear-client/src/error.rs:21-25`),
  mapped to exit `105` in `cli/linear-cli/src/exit_codes.rs`.
- `init discover` and its `linear.team_key` write-back:
  `discover` and `report_team_key_writeback` (`cli/linear-cli/src/main.rs`).
- `linear.team_id` is registered as an open key at
  `cli/config/src/catalogue.rs:227`.
- The deprecation path to reuse: `config::resolve_with_deprecated_fallback`
  and `config::emit_deprecation_once`, as `auth::team_key` uses them.

## Drafting Notes

- Read "fresh repository" as one with no `catalogue.json`, whether or not
  `linear.team_key` is set.
- Treated 0210's statement that "`linear.team_id` does not exist" as stale:
  the key is registered in `cli/config/src/catalogue.rs` and
  `auth::resolve_team` reads it first.
- Kept runtime key-to-id resolution out of scope so non-init commands make no
  extra request; the catalogue stays the source of the team id.

## References

- Related: 0294, 0228, 0048, 0210
- Found during: `meta/plans/2026-10-10-0296-sync-round-trip-defects.md`
  (Phase 10 manual verification)
