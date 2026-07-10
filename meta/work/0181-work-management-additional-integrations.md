---
type: work-item
id: "0181"
title: "Work Management Additional Integrations"
date: "2026-07-06T13:29:26+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: epic
priority: high
relates_to: ["work-item:0045"]
external_id: PP-701
tags: [work-management, integrations, trello, github-issues, sync]
last_updated: "2026-07-06T13:29:26+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0181: Work Management Additional Integrations

**Kind**: Epic
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As a developer using the Accelerator plugin, I want work-item sync to support
additional remote trackers — Trello and GitHub Issues + Projects — beyond the
shipped Jira and Linear integrations, so that teams on those tools can manage
work items locally while keeping them in sync.

This epic was split out of 0045 (Work Management Integration), which delivered
the local work-item lifecycle, configurable ID patterns, the `work.integration`
configuration, core-skill sync integration, the Jira and Linear integrations,
and the `sync-work-items` skill. The two remaining integrations are grouped
here as a separately schedulable epic.

## Context

The integration architecture is established (see 0045): each remote system has
an `init-<system>` skill that persists credentials and catalogues to the
configured integrations path, verb-decomposed read/write skills for individual
API operations, and reconciliation through the already-shipped `sync-work-items`
skill (0051). Two integrations follow the same pattern but are not yet built:

- **Trello** — REST-based; list-position workflow rather than a state machine.
- **GitHub Issues + Projects v2** — REST for issues, REST/GraphQL for Projects.

Both plug into the existing `work.integration` config surface and the sync
engine with no changes required to the core work-management skills.

## Requirements

**Trello integration** — REST-based. Eight skills under
`skills/integrations/trello/`: `init-trello`, `search-trello-cards`,
`show-trello-card`, `create-trello-card`, `update-trello-card`,
`move-trello-card`, `comment-trello-card`, `attach-trello-card`. Workflow is
list-position rather than a state machine; a configurable list-name → local
status mapping is persisted by `init-trello` to the configured integrations
path (`status-map.json`). Trello card `shortLink` is used as the remote
identifier.

**GitHub Issues + Projects integration** — REST-based for issues; REST (GitHub
September 2025 GA) and GraphQL for Projects v2. Eight skills under
`skills/integrations/github-issues/`: `init-github-issues`,
`search-github-issues`, `show-github-issue`, `create-github-issue`,
`update-github-issue`, `comment-github-issue`, `close-github-issue`, plus
Projects v2 status field management. GitHub issue identifiers stored as
`{owner}/{repo}#{number}` for global uniqueness. Delegates to the `gh` CLI
where available; direct REST/GraphQL as fallback.

## Stories

- 0049 — Trello integration
- 0050 — GitHub Issues + Projects integration

## Acceptance Criteria

- [ ] Given `work.integration: trello` is configured, when a developer uses a
      read or write integration skill, then it scopes automatically without
      requiring per-call project flags, and the full CRUD lifecycle (read,
      create, update, move, comment, attach) is supported
- [ ] Given `work.integration: github-issues` is configured, when a developer
      uses a read or write integration skill, then basic issue CRUD plus
      multi-state status tracking via a Projects v2 `single_select` field is
      supported
- [ ] Given a work item is pushed to Trello, then its `external_id` exactly
      matches the card `shortLink` (e.g. `"AbCd1234"`); given it is pushed to
      GitHub Issues, then `external_id` matches `{owner}/{repo}#{number}`
- [ ] Given a developer runs `/init-trello` or `/init-github-issues`, when auth
      is verified, then project/board and field catalogues are persisted to the
      configured integrations path
- [ ] Given either integration is configured, when `/sync-work-items` runs,
      then local work items reconcile bidirectionally with the remote tracker
      using the same conflict rules as the Jira and Linear integrations

## Open Questions

- For GitHub Issues: should single-repo teams be able to configure a shorter
  identifier form (e.g. just `#42`) instead of `{owner}/{repo}#{number}`, to
  reduce frontmatter verbosity? (Carried over from 0045.)

## Dependencies

- Blocked by: — (the `sync-work-items` engine, 0051, is already complete)
- Blocks: —

## Assumptions

- **GitHub Issues integration includes Projects v2** for multi-state workflow
  tracking, making it the more complex of the two.
- **Trello `shortLink`** (not the 24-char hex card ID) is the remote identifier,
  chosen for readability.

## Technical Notes

- Trello's list-position workflow means the sync skill translates between named
  lists and local status values via the `status-map.json` produced by
  `init-trello`; that file is the hand-editable configuration surface.
- GitHub Projects v2 gained a REST API in September 2025 for most operations;
  fall back to GraphQL for the rest. The `gh` CLI covers both layers and is an
  existing dependency in `skills/github/`.

## Drafting Notes

- Carved out of 0045 as a **sibling** epic (via `relates_to`), not a child —
  the two draft integration stories 0049 and 0050 are reparented here, and 0045
  is transitioned to `done` since its remaining children (0046, 0047, 0048,
  0051) are all complete.
- Status left at `draft` — neither child integration has started.

## References

- Related: 0045, 0049, 0050
