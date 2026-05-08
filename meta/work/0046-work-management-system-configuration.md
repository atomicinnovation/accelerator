---
work_item_id: "0046"
title: "Work Management System Configuration"
date: "2026-05-06T17:49:44+00:00"
author: Toby Clemson
type: story
status: ready
priority: high
parent: "0045"
tags: [work-management, integrations, configuration]
---

# 0046: Work Management System Configuration

**Type**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Add a `work.integration` configuration key that declares the active remote
issue tracker (`jira`, `linear`, `trello`, or `github-issues`). When set,
integration skills scope automatically to `work.default_project_code` without
requiring an explicit `--project` flag. Work items are always written to
`meta/work/` regardless of whether an integration is configured — the remote
integration augments the local-first store, it does not replace it.

## Context

The Accelerator plugin's work management system currently operates as a
local-only store (`meta/work/`). The epic (0045) designates `work.integration`
as the activation gate for any remote integration — without it, no external API
calls are made and all skills function purely against local files. This story
implements that gate and the automatic project-scoping it enables, unblocking
all downstream integration and sync stories.

## Requirements

- Add `work.integration` as a recognised config key with allowed values:
  `jira`, `linear`, `trello`, `github-issues`
- When `work.integration` is set, integration skills default to
  `work.default_project_code`, eliminating the need for an explicit
  `--project` flag
- When `work.integration` is not set, all work management skills operate
  against `meta/work/` with no external API calls
- Work items are always written to `meta/work/` as local files, regardless
  of whether `work.integration` is configured; the remote integration is
  an additional layer on top of local storage, not a replacement

## Acceptance Criteria

- [ ] Given `work.integration` is not configured, when a developer uses any
  work management skill, then all skills function against `meta/work/` with no
  external API calls
- [ ] Given `work.integration: jira` and `work.default_project_code: PROJ` are
  configured, when a developer invokes an integration skill without `--project`,
  then the skill defaults to `PROJ`
- [ ] Given `work.integration` is configured, when any work management skill
  creates or updates a work item, then the work item is written to `meta/work/`
  as a local file regardless of whether a remote push occurs
- [ ] Given `work.integration` is set to an unrecognised value, when any skill
  reads the config, then an informative error is surfaced naming the valid values
- [ ] Given `work.default_project_code` is empty and `work.integration` is set,
  when a developer invokes an integration skill, then the skill warns that a
  default project code is required

## Open Questions

- —

## Dependencies

- Blocked by: —
- Blocks: 0047, 0048, 0049, 0050, 0051

## Assumptions

- `work.integration` is a single string, not an array — one active integration
  at a time, consistent with the epic's explicit assumption.
- `work.default_project_code` already exists as a config key; this story adds
  `work.integration` alongside it.
- `work.integration` is workspace-level config only; global
  (`~/.config/accelerator/`) scoping is out of scope.
- No migration is required: this story will be implemented before the
  integration feature is first rolled out, so no existing repos will have a
  credentials directory without the key.

## Technical Notes

- Config is read via `config-read-value.sh`; the new key follows the same
  pattern as existing `work.*` keys.

## Drafting Notes

- Priority set to high: this story is the prerequisite gate for all downstream
  planned stories in the epic.
- Interpreted "scope automatically to `work.default_project_code`" as: skills
  read `work.default_project_code` from config and use it as the default,
  without the user needing to pass `--project` on the command line.
- Local-first write behaviour made explicit per user clarification: `meta/work/`
  is always the primary store; the remote integration is additive.

## References

- Source: `meta/work/0045-work-management-integration.md`
