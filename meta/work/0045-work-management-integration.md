---
id: "0045"
title: "Work Management Integration"
date: "2026-05-04T23:56:41+00:00"
author: Toby Clemson
kind: epic
status: done
priority: high
tags: [work-management, integrations, jira, linear, sync]
type: work-item
schema_version: 1
last_updated: "2026-07-06T13:29:26+00:00"
last_updated_by: Toby Clemson
derived_from: ["codebase-research:2026-04-08-ticket-management-skills", "codebase-research:2026-04-25-rename-tickets-to-work-items", "codebase-research:2026-04-26-remaining-ticket-references-post-migration", "codebase-research:2026-04-27-create-work-item-open-from-existing", "codebase-research:2026-04-28-configurable-work-item-id-pattern", "codebase-research:2026-04-29-jira-cloud-integration-skills"]
external_id: PP-67
---

# 0045: Work Management Integration

**Kind**: Epic
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

## Summary

As a developer using the Accelerator plugin, I want a complete local-first work
management system that can synchronise with popular remote issue trackers (Jira
and Linear), so that I can manage work items locally in my repository while
keeping them in sync with my team's chosen tracking tool.

The initiative covers three layers: (1) a local work item lifecycle implemented
as Accelerator skills, (2) per-system integrations for Jira and Linear, and
(3) a bidirectional sync skill that reconciles local work items with remote
issues on demand. All streams are complete.

> **Scope note**: The remaining Trello and GitHub Issues + Projects integrations
> were split out into a separate epic, **0181 (Work Management Additional
> Integrations)**, and are tracked there. This epic covers the shipped Jira and
> Linear integrations plus the shared foundation.

## Context

Work items are structured markdown files under `meta/work/`, each with YAML
frontmatter capturing identity (`work_item_id`), lifecycle state, and metadata.
The `work_item_id` is the remote issue key: it is allocated by the remote system
on first push and written into the local file at that point. A work item with
`work_item_id: "PROJ-0042"` corresponds to Jira issue PROJ-0042. Work items that
have not yet been pushed carry a local numeric ID allocated by
`work-item-next-number.sh`; the numeric ID signals "not yet synced".

Integration skills follow the pattern established by `skills/integrations/jira/`:
a per-system `init-<system>` skill persists credentials, project, and field
catalogues to `meta/integrations/<system>/`; verb-decomposed read and write
skills handle individual API operations; and `sync-work-items` orchestrates
bidirectional sync across the configured system.

> **Path note**: the `meta/integrations/<system>/` references throughout this
> epic are illustrative and predate the **Work management system configuration**
> stream, which makes the integrations path configurable. The resolved default
> is `.accelerator/state/integrations/<system>/` (via `config-read-path.sh
> integrations`); `meta/integrations/` is legacy and guard-banned. Integration
> stories (e.g. 0048) use "the configured integrations path" and supersede the
> literal paths below.

Background research:
- `meta/research/codebase/2026-04-08-ticket-management-skills.md` — original skill design
- `meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md` — terminology rename
- `meta/research/codebase/2026-04-26-remaining-ticket-references-post-migration.md` — post-rename cleanup
- `meta/research/codebase/2026-04-27-create-work-item-open-from-existing.md` — enrich-existing mode
- `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md` — ID pattern DSL
- `meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md` — Jira integration design

## Requirements

### Completed

**Local work item lifecycle** — Seven Accelerator skills managing work items in
`meta/work/`: `/create-work-item` (interactive creation and enrichment of
existing items), `/extract-work-items` (batch extraction from specifications and
PRDs), `/list-work-items` (discovery and filtering), `/update-work-item` (status
transitions), `/review-work-item` (multi-lens quality review: completeness,
clarity, testability, scope, dependencies), `/stress-test-work-item`
(adversarial examination), `/refine-work-item` (decomposition and technical
enrichment).

**Configurable work item ID pattern** — `work.id_pattern` DSL (`{number:04d}`
default; `{project}-{number:04d}` for external tracker alignment),
`work.default_project_code`, per-project counters, migration
`0002-rename-work-items-with-project-prefix.sh`, and skip-tracking extension to
the migration framework.

**Jira Cloud integration** — Eight skills under `skills/integrations/jira/`:
`init-jira`, `search-jira-issues`, `show-jira-issue`, `create-jira-issue`,
`update-jira-issue`, `comment-jira-issue`, `transition-jira-issue`,
`attach-jira-issue`. ADF ↔ Markdown round-trip via `jira-md-to-adf.sh` and
`jira-adf-to-md.sh`. Auth via `ACCELERATOR_JIRA_TOKEN_CMD` indirection. Field
and project catalogues cached in `meta/integrations/jira/`.

### Delivered

**Work management system configuration** — A `work.integration` config key
(values: `jira`, `linear`, `trello`, `github-issues`) that declares the active
remote system. When set, integration skills scope automatically to
`work.default_project_code` and core work management skills show sync status and
offer to push to the remote.

**Core skills sync integration** — When `work.integration` is configured:
- `/list-work-items` shows a per-item sync status indicator (synced / unsynced /
  conflict), where a numeric `work_item_id` signals unsynced and a remote-format
  ID signals synced
- `/create-work-item` offers to push the newly created work item to the remote
  system; the remote system allocates the issue ID, which is then written as
  `work_item_id` in the local file — the local file is not written until the
  push succeeds or the user declines to push

**Linear integration** — GraphQL-based. Eight skills under
`skills/integrations/linear/`: `init-linear`, `search-linear-issues`,
`show-linear-issue`, `create-linear-issue`, `update-linear-issue`,
`transition-linear-issue`, `comment-linear-issue`, `attach-linear-issue`
(`attach-linear-issue` added over the original seven because Linear supports both
link-based and binary file attachments — see work item 0048). State transitions
require resolving team-scoped `WorkflowState` UUIDs. Linear issue identifiers
(e.g. `BLA-123`) map directly to `work_item_id`. A `linear-graphql.sh` request
helper handles GraphQL query/mutation construction, cursor-based pagination, and
the `400 RATELIMITED` response code.

**`sync-work-items` skill** — Bidirectional, on-demand. Reads `work.integration`
to identify the active system. For each local work item whose `work_item_id`
matches the remote key format (i.e. has been pushed), compares local and remote
state against a last-sync timestamp persisted in
`meta/integrations/<system>/last-sync.json`. On conflict (both sides changed
since last sync), the remote version is the default; the user is shown a
side-by-side diff and must confirm or override before any local write occurs.

## Stories

- — Local work management foundation (seven skills) [complete — no work item]
- — Configurable work item ID pattern and project code [complete — no work item]
- — Jira Cloud integration [complete — no work item]
- — Work management system configuration (`work.integration` key)
- — Core skills sync integration (`/list-work-items` sync status; `/create-work-item` push offer)
- — Linear integration
- — `sync-work-items` skill

Split into epic 0181 (Work Management Additional Integrations):

- 0049 — Trello integration
- 0050 — GitHub Issues + Projects integration

## Acceptance Criteria

- [x] Given no `work.integration` is configured, when a developer uses any work management skill, then all skills function against `meta/work/` with no external API calls
- [x] Given `work.integration: jira | linear` is configured, when a developer uses a read or write integration skill, then the skill scopes automatically to `work.default_project_code` without requiring `--project`
- [x] Given `work.integration` is configured and `/list-work-items` is invoked, then each work item row shows a sync status indicator: synced (remote-format `work_item_id`), unsynced (numeric `work_item_id`), or conflict
- [x] Given `work.integration` is configured and `/create-work-item` completes, then the skill offers to push to the remote system; on acceptance, the remote allocates the issue ID, and the local file is written with the remote-allocated key as `work_item_id`; on decline, the local file is written with a numeric ID
- [x] Given a developer runs `/init-jira` or `/init-linear`, when auth is verified, then project and field catalogues are persisted to `meta/integrations/<system>/` and the integration supports the full CRUD lifecycle (read, create, update, transition, comment, attach)
- [x] Given a work item has been pushed to a remote system, then its `work_item_id` frontmatter field exactly matches the remote issue key (e.g. `"PROJ-0042"` for Jira, `"BLA-123"` for Linear)
- [x] Given a developer runs `/sync-work-items` with no conflicts, when the sync completes, then all locally tracked work items with remote-format IDs are updated to reflect the remote state, all untracked remote issues within the configured project are created as local work items with the remote ID as `work_item_id`, and `last-sync.json` is updated
- [x] Given a conflict exists (local and remote both changed since last sync), when `/sync-work-items` encounters it, then the remote version is the default, a side-by-side diff is shown, and the local file is only overwritten after explicit user confirmation

## Open Questions

- None. The open question about GitHub Issues identifier format moved to epic
  0181 with the GitHub integration.

## Dependencies

- Blocked by: —
- Blocks: —
- Relates to: 0181 (Work Management Additional Integrations) — carries the
  remaining Trello and GitHub Issues + Projects integrations

## Assumptions

- **One active integration at a time** — `work.integration` is a single string, not an array. Multi-system mirroring is out of scope for this epic.
- **Remote system allocates the ID on first push** — there is no intermediate numeric-then-remote ID transition. When a user accepts the push offer in `/create-work-item`, the remote creates the issue first, returns its key, and the local file is written once with that key as `work_item_id`. Work items saved without pushing carry a local numeric ID and are treated as unsynced.
- **Sync is last-modified timestamp-based** — conflict detection compares file modification times against a stored last-sync timestamp. SHA-based diffing and three-way merge are out of scope.

## Technical Notes

- Linear's GraphQL API uses a single endpoint (`https://api.linear.app/graphql`), cursor-based pagination, and returns rate-limit errors as `400` with `"code": "RATELIMITED"` in GraphQL error extensions — not `429`. A dedicated `linear-graphql.sh` helper is required; `jira-request.sh` cannot be reused.
- The `schpet/linear-cli` (TypeScript, April 2026, actively maintained) is worth reviewing for Linear auth patterns and agent-friendly UX before designing `init-linear`.

Technical notes specific to the Trello and GitHub Issues integrations moved to
epic 0181.

## Drafting Notes

- Status set to `done` — all in-scope streams (local skills, configurable ID pattern, Jira and Linear integrations, and the `sync-work-items` skill) are complete. The remaining Trello and GitHub Issues integrations were split into epic 0181, so they no longer gate this epic's completion.
- "Remote allocates ID on first push" means the push is a prerequisite for writing the local file with a permanent ID. The implementing plan for the "offer to push" feature in `/create-work-item` must decide what happens on push failure: leave the draft in memory and re-offer, or write locally with a numeric ID and sync later.
- "Offer to push" UX for `/create-work-item` is intentionally left to the implementing plan — the epic captures the intent (remote allocates first, then write); the plan decides whether this is a confirmation prompt, an explicit `--push` flag, or automatic with a confirmation gate.
- The numeric-ID-as-unsynced convention (a numeric `work_item_id` means "not yet pushed") is a new semantic for an existing field. The sync status indicators in `/list-work-items` depend on this convention; it should be documented in the configure skill's `work` section.

## References

- Source: `meta/research/codebase/2026-04-08-ticket-management-skills.md`
- Source: `meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md`
- Source: `meta/research/codebase/2026-04-26-remaining-ticket-references-post-migration.md`
- Source: `meta/research/codebase/2026-04-27-create-work-item-open-from-existing.md`
- Source: `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md`
- Source: `meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md`
