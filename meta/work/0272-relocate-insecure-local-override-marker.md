---
type: "work-item"
id: "0272"
title: "Relocate Insecure-Local Override Marker to .accelerator"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "done"
kind: "task"
priority: "medium"
tags: ["security", "config", "cleanup"]
last_updated: "2026-08-31T20:26:32+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0272: Relocate Insecure-Local Override Marker to .accelerator

**Kind**: Task
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Move the credential-override marker from `.claude/insecure-local-ok` to
`.accelerator/allow-insecure-local`, consolidating Accelerator-owned files
under `.accelerator` and aligning the marker's basename with the
`ACCELERATOR_ALLOW_INSECURE_LOCAL` environment variable the marker pairs with.
Hard cutover: the old path is dropped, not read as a fallback.

## Context

The marker is the second half of the insecure-local override.
`refuse_insecure_personal_config` (referred to below as "the resolver") lets a
non-`0600` personal config file through only when
`ACCELERATOR_ALLOW_INSECURE_LOCAL=1` is set **and** a regular,
non-symlink, VCS-tracked marker file is present
(`cli/tracker-support/src/credentials.rs:418`). Each CLI's context builds the
marker path independently as `root.join(".claude/insecure-local-ok")`. Every
other Accelerator-owned file already lives under `.accelerator/` (`config.md`,
`config.local.md`); the marker is the outlier. The plugin only reads the
marker — users commit it themselves — so there is nothing for the plugin to
write or auto-migrate. The insecure-local functionality is unreleased, so no
existing repo relies on the current path.

## Requirements

- Change the marker path in all three production context builders —
  `cli/jira-cli/src/context.rs`, `cli/linear-cli/src/context.rs`,
  `cli/work-cli/src/tracker_registry.rs` — from `.claude/insecure-local-ok` to
  `.accelerator/allow-insecure-local`.
- Update the test-support fixtures that build `insecure_marker` (in the
  `jira-client`, `linear-client`, and `tracker-support` tests) to the new
  path and basename.
- Update the docstring on `refuse_insecure_personal_config`
  (`credentials.rs:377`) and the user-facing guidance in
  `skills/config/configure/SKILL.md` (~line 810) to name
  `.accelerator/allow-insecure-local`.
- Hard cutover: do not read `.claude/insecure-local-ok` as a fallback and do
  not add a migration reminder. The functionality is unreleased, so no
  transition aid is required.
- Leave the override semantics unchanged — the env-var, regular-file,
  non-symlink, and VCS-tracked checks all stay identical; only the path moves.

## Acceptance Criteria

- Given `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` and a regular, VCS-tracked
  `.accelerator/allow-insecure-local`, when the resolver runs against a
  non-`0600` personal config, then the override is honoured and the read
  proceeds.
- Given the same environment variable but only a legacy
  `.claude/insecure-local-ok` present, when the resolver runs against a
  non-`0600` personal config, then it refuses with `E_LOCAL_PERMS_INSECURE` —
  the old path is no longer honoured.
- Given `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` and a symlinked
  `.accelerator/allow-insecure-local`, when the resolver runs against a
  non-`0600` personal config, then it still refuses with `E_LOCAL_PERMS_INSECURE`
  — the non-symlink gate holds at the new path.
- Given `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` and a `.accelerator/allow-insecure-local`
  that is not VCS-tracked, when the resolver runs against a non-`0600` personal
  config, then it still refuses with `E_LOCAL_PERMS_INSECURE` — the VCS-tracked
  gate holds at the new path.
- A grep for `insecure-local-ok` across `cli/`, `skills/`, and `hooks/` returns
  no matches; only historical `meta/` research and plans may retain it.
- A grep for `allow-insecure-local` returns a match in both
  `cli/tracker-support/src/credentials.rs` (the
  `refuse_insecure_personal_config` docstring) and
  `skills/config/configure/SKILL.md`, confirming the docs name the new path.
- Each of the three context builders — `cli/jira-cli/src/context.rs`,
  `cli/linear-cli/src/context.rs`, and `cli/work-cli/src/tracker_registry.rs` —
  constructs `.accelerator/allow-insecure-local` (verifiable by a grep for the
  new literal returning a match in all three files), so no builder is left
  pointing at a stale or mistyped path.
- `mise run check` passes with the updated fixtures.

## Dependencies

- Blocked by: none.
- Blocks: none.
- Ordering: must land before the insecure-local override feature is released.
  The hard cutover is safe only while the feature is unreleased; once released,
  a repo could commit `.claude/insecure-local-ok` and the rename would become a
  breaking migration. No release/feature work item tracks this yet — record the
  ref here if one is created.

## Assumptions

- The override's security semantics are unchanged — only the marker's location
  and basename move; the env-var, regular-file, non-symlink, and VCS-tracked
  gate stays as-is.
- The insecure-local functionality is unreleased, so the hard cutover carries
  no migration risk — no committed marker exists in any released repo to
  strand.

## Technical Notes

- The runtime failure (`E_LOCAL_PERMS_INSECURE`, `credentials.rs:191`) tells the
  user to `chmod 600` and does not name the marker path, so no error string
  needs updating — only the docstring and the configure-skill doc reference the
  marker by name.
- `.accelerator/.gitignore` ignores only `config.local.md`, so
  `.accelerator/allow-insecure-local` is trackable and satisfies the
  VCS-tracked check.
- The resolver reads `context.insecure_marker` abstractly; the only behavioural
  change is the path each context builder constructs.

## Drafting Notes

- Renamed the basename to `allow-insecure-local` (from `insecure-local-ok`) to
  mirror the `ACCELERATOR_ALLOW_INSECURE_LOCAL` env var — a naming call a
  reviewer may want to revisit.
- Corrected the source backlog's "reads/writes it" and "migration for existing
  repos" framing: the plugin only reads a user-committed marker, so there is no
  write path; the functionality is unreleased, so the hard cutover needs no
  migration aid.
- Added a `security` tag — the marker gates a credential-security check.
- Sharpened the title to reflect the rename; the original intent is retained.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
- Code: `cli/tracker-support/src/credentials.rs` (override check),
  `skills/config/configure/SKILL.md` (override documentation)
