---
type: work-item
id: "0228"
title: Layered Configuration Key Model
date: 2026-08-30T14:35:09+00:00
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: high
parent: "work-item:0146"
relates_to: ["work-item:0220"]
tags: [configuration, work-management, migration, tracker]
last_updated: 2026-08-30T14:35:09+00:00
last_updated_by: Toby Clemson
schema_version: 1
---
# 0228: Layered Configuration Key Model

**Kind**: Task
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Rename `work.default_project_code` to `work.key` and the `id_pattern` placeholder
`{project}` to `{key}`, and introduce layered key ownership. The tracker-native key
(`linear.team_key` / `jira.project_key`) is the canonical, integration-owned value;
`work.key` derives from it when a tracker is configured, and is set directly only in
tracker-less repos.

## Context

`default_project_code` is misnamed: its real job is the key a tracker stamps on
issue identifiers — a Jira project key, a Linear team key — the key of the entity
that owns the issue-number sequence, not a "project". It is also overloaded as the
sync scope, which is what let bug 0220 hide.

Integration skills (search/show for Jira and Linear) must keep working, and may in
future be packaged independently of the work-management framework. So the dependency
must run work → integration, never the reverse: integration skills read only their
own `jira:` / `linear:` section; the work layer reads the integration-owned key.

## Requirements

- Rename `work.default_project_code` → `work.key`; `id_pattern` placeholder
  `{project}` → `{key}`.
- Make `linear.team_key` / `jira.project_key` the canonical, integration-owned key,
  usable by the integration skills with no `work.*` present.
- Derive `work.key` from the integration key when a tracker is configured; allow it
  set directly only in tracker-less repos.
- Setting both the integration key and `work.key` is a config-validation error.
- Provide a read-time alias and migration for existing `default_project_code` and
  `{project}` configs.
- `init-linear` / `init-jira` write the discovered key into the tracker section.

## Acceptance Criteria

- [ ] Given only a `jira:` or `linear:` section with its key and no `work.*`, when
      an integration skill runs, then it functions without a work-management config.
- [ ] Given a tracker-backed repo, when the key is set under
      `<tracker>.<entity>_key` and `work.key` is omitted, then `{key}` in IDs
      resolves from the integration key.
- [ ] Given both the integration key and `work.key` are set, then configuration
      validation fails with a message naming the conflict.
- [ ] Given an existing config using `default_project_code` / `{project}`, when it
      is read after this change, then it keeps working via the alias/migration and
      renders IDs identically.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Technical Notes

- Sequenced after 0220, which still reads `work.default_project_code`; this item
  renames that touchpoint.
- The Linear team key → UUID resolver already exists
  (`cli/linear-client/src/auth.rs`); this item does not change resolution, only the
  config field the key is read from and its ownership layer.

## Drafting Notes

- Ownership deliberately inverted so integration skills never depend on `work.*` —
  chosen over a "work.key overrides the integration key" model, which would couple
  them in the combined deployment.

## References

- Parent: 0146
- Related: 0220
