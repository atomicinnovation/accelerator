---
work_item_id: "0050"
title: "GitHub Issues and Projects Integration"
date: "2026-05-06T17:49:44+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: "0045"
tags: [work-management, integrations, github-issues, github-projects]
---

# 0050: GitHub Issues and Projects Integration

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Implement eight skills under `skills/integrations/github-issues/` covering the
full CRUD lifecycle for GitHub Issues, plus Projects v2 single-select status
field management for multi-state workflow tracking. All skills delegate to the
`gh` CLI as a hard prerequisite. GitHub issue identifiers are stored as
`{owner}/{repo}#{number}` (e.g. `atomic-innovation/accelerator#42`) for global
uniqueness across multi-repo workspaces.

## Context

GitHub Issues alone is a flat tracker (open/closed); multi-state workflow
tracking requires Projects v2 with a `single_select` status field. The epic
treats GitHub Issues + Projects as a single integration because they are
typically used together in practice, and because the Projects v2 status field
is the only mechanism for the kind of state machine sync the work management
system expects. As of September 2025, GitHub provides REST API support for
most Projects v2 operations and the `gh` CLI covers both layers fully. The
established `skills/github/` directory already uses the `gh` CLI as a primary
dependency, and continuing that precedent significantly reduces the
implementation surface here.

## Requirements

- Eight skills:
  - `init-github-issues` — authenticate, persist repository details, the
    repository's configured Project v2 details, label catalogue, milestone
    catalogue, and Projects v2 status-field option catalogue (option IDs and
    names) to the github-issues subdirectory under the configured integrations
    path
  - `search-github-issues` — query issues by filter via `gh issue list` and
    `gh search issues` for cross-repo searches
  - `show-github-issue` — display a single issue including its Projects v2
    status field value
  - `create-github-issue` — create an issue and add it to the configured
    Project v2; write the remote-allocated identifier as
    `{owner}/{repo}#{number}` in the local work item file's `work_item_id`
  - `update-github-issue` — update fields on an existing issue (title, body,
    labels, assignees, milestone)
  - `comment-github-issue` — add a Markdown (GFM) comment to an issue
  - `close-github-issue` — close an issue with a `state_reason`; default to
    `completed` and prompt the user to confirm or override (`not_planned`,
    `duplicate`)
  - `transition-github-project-status` — set the Projects v2 single-select
    status field for an issue, resolving the option ID from the cached
    catalogue

## Acceptance Criteria

- [ ] Given `gh` is not installed, when any skill is invoked, then the skill
  surfaces a clear error directing the user to install `gh` and exits without
  attempting any operation
- [ ] Given `init-github-issues` is run with valid credentials, when
  authentication succeeds, then repository details, the configured Project v2's
  details, label catalogue, milestone catalogue, and Projects v2 status-field
  option catalogue (option ID and name pairs) are persisted to the
  github-issues subdirectory under the configured integrations path
- [ ] Given `init-github-issues` has been run, when `create-github-issue` is
  invoked, then an issue is created, added to the configured Project v2, and
  its identifier is written as `{owner}/{repo}#{number}` in the local work
  item file's `work_item_id`
- [ ] Given an issue exists in the configured Project v2, when
  `transition-github-project-status` is invoked with a target status name, then
  the skill resolves the corresponding single-select option ID from the cached
  catalogue and updates the status field via `gh project item-edit`
- [ ] Given `close-github-issue` is invoked without an explicit `state_reason`,
  when the user confirms, then the issue is closed with `state_reason:
  completed`; when the user overrides, then the supplied reason is used
- [ ] Given results span multiple pages, when listing or searching, then
  pagination is handled transparently by `gh` (no manual cursor handling
  required at the skill layer)
- [ ] Given the `gh` rate limit is depleted, when any skill encounters a rate
  limit error from `gh`, then the error is surfaced clearly to the user

## Open Questions

- —

## Dependencies

- Blocked by: 0046
- Blocks: 0051

## Assumptions

- `gh` CLI is a hard prerequisite for the integration; skills do not provide
  a direct REST/GraphQL fallback. Auth flows through `gh auth` and the
  `GH_TOKEN` environment variable (preferred over `GITHUB_TOKEN`).
- A single repository is configured at `init-github-issues` time and stored in
  the catalogue; no per-invocation `--repo` flag is required.
- A single Project v2 per repository is configured at `init-github-issues`
  time. New issues created via `create-github-issue` are automatically added
  to the configured project; existing issues not yet in any project are added
  to the configured project on first interaction with status field skills.
- GitHub Issues `{owner}/{repo}#{number}` identifier format is used as
  `work_item_id` to ensure global uniqueness across multi-repo workspaces.
- GitHub accepts GFM Markdown natively for issue bodies and comments; no
  conversion layer is needed. The 65,536-character body limit applies.
- `state_reason` is treated as an opaque string set: `completed`, `not_planned`,
  `duplicate`, plus future additions GitHub may introduce (per the May 2024
  `duplicate` precedent).

## Technical Notes

- All skills delegate to `gh` for both Issues operations (`gh issue *`) and
  Projects v2 operations (`gh project *`). Hard dependency simplifies the
  skill layer significantly compared to a REST/GraphQL fallback path.
- Projects v2 single-select option lookup: `gh project item-edit` requires the
  option **ID**, not the option **name**. The catalogue cached by
  `init-github-issues` therefore stores `{name → id}` pairs so transitions
  resolve names locally without a live `gh project field-list` call.
- Adding an issue to a project and updating its status field are two separate
  operations (`gh project item-add` returns the new item ID, then
  `gh project item-edit` sets the field). `create-github-issue` and
  `transition-github-project-status` must each handle both phases.
- Cross-repo search uses `gh search issues` and may hit GitHub's lower search
  rate limit (30 requests/minute) — distinct from the 5,000 requests/hour
  primary limit. Surface this in error messages so users distinguish the two
  buckets.
- The single-repo `#42` short-form mentioned in the epic's open question is
  not adopted here; the long form `{owner}/{repo}#{number}` is kept for
  global uniqueness, leaving the short-form choice to a possible future
  enhancement story.

## Drafting Notes

- Storage references use "the github-issues subdirectory under the configured
  integrations path" — consistent with the Linear and Trello stories.
- The eighth skill is named `transition-github-project-status` to make explicit
  that it operates on the Projects v2 status field rather than the issue's
  open/closed state. The closer-to-state-machine semantic is the Projects v2
  status field, not the issue itself, which is binary.
- `gh` confirmed as a hard prerequisite per user direction — no REST/GraphQL
  fallback in skills. This deliberately trades fewer install-time options for
  significantly less integration complexity.
- `close-github-issue` defaults to `completed` with a confirmation prompt per
  user direction; `not_planned` and `duplicate` are accepted as overrides.
- "Default project for issues without one" interpreted as: the
  `init-github-issues` skill captures a single Project v2 per repository, and
  any issue interacted with — newly created or existing — is added to that
  project if not already present. If an issue is already in another project,
  status transitions still target the configured project's status field; the
  implementing plan should decide what happens if this assumption breaks (e.g.
  a status name is ambiguous across projects).

## References

- Source: `meta/work/0045-work-management-integration.md`
- Research: GitHub REST API documentation (Issues, Projects v2, search,
  pagination, rate limits)
- Research: `gh` CLI manual (`gh issue`, `gh project`, `gh search`)
- Research: GitHub Changelog — REST API for Projects (11 Sep 2025)
