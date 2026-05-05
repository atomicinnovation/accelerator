---
work_item_id: "0048"
title: "Linear Integration"
date: "2026-05-06T17:49:44+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: "0045"
tags: [work-management, integrations, linear]
---

# 0048: Linear Integration

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Implement eight skills under `skills/integrations/linear/` covering the full
CRUD lifecycle against Linear's GraphQL API, including issue management, state
transitions, comments, and attachments. Linear issue identifiers (e.g. `BLA-123`)
map directly to `work_item_id` in local work item files.

## Context

Linear uses a single GraphQL endpoint with cursor-based pagination and
team-scoped `WorkflowState` UUIDs for state transitions. Rate-limit errors
arrive as HTTP `400` with `"code": "RATELIMITED"` in GraphQL error extensions
rather than the standard `429`. Linear accepts Markdown natively for issue
descriptions and comments — no conversion layer is required, unlike the Jira
ADF integration. The pattern follows the established Jira integration under
`skills/integrations/jira/`.

## Requirements

- Eight skills:
  - `init-linear` — authenticate, persist team details (UUID, key, name) and
    WorkflowState catalogue to the linear subdirectory under the configured
    integrations path; scope to a single team
  - `search-linear-issues` — query issues by filter
  - `show-linear-issue` — display a single issue
  - `create-linear-issue` — create an issue; write the remote-allocated
    identifier (e.g. `BLA-123`) as `work_item_id` in the local work item file
  - `update-linear-issue` — update fields on an existing issue
  - `transition-linear-issue` — transition state using WorkflowState UUIDs
    resolved from the cached catalogue
  - `comment-linear-issue` — add a Markdown comment to an issue
  - `attach-linear-issue` — attach a resource to an issue: link-based
    attachments (external URLs via `attachmentCreate`) and binary file uploads
    (two-step pre-signed URL flow)

## Acceptance Criteria

- [ ] Given `init-linear` is run with valid credentials, when authentication
  succeeds, then the team's details and WorkflowState catalogue are persisted
  to the linear subdirectory under the configured integrations path
- [ ] Given `init-linear` has been run, when `create-linear-issue` is invoked,
  then a Linear issue is created and its remote-allocated identifier (e.g.
  `BLA-123`) is written as `work_item_id` in the local work item file
- [ ] Given `init-linear` has been run, when `transition-linear-issue` is
  invoked with a target state name, then the skill resolves the corresponding
  WorkflowState UUID from the cached catalogue and applies the transition
  without a live catalogue lookup
- [ ] Given a valid external URL, when `attach-linear-issue` is invoked, then
  a link-based attachment is created on the issue via `attachmentCreate`
- [ ] Given a local file path, when `attach-linear-issue` is invoked, then the
  file is uploaded via the pre-signed URL flow and the resulting URL is
  embedded in the issue
- [ ] Given the API responds with HTTP `400` and `"code": "RATELIMITED"`, when
  any Linear skill handles the response, then the error is surfaced clearly,
  not treated as a generic failure, and a backoff delay is computed from the
  `X-RateLimit-Requests-Reset` response header
- [ ] Given cursor-based pagination is required, when results span multiple
  pages, then all pages are retrieved transparently before results are returned

## Open Questions

- —

## Dependencies

- Blocked by: 0046
- Blocks: 0051

## Assumptions

- Auth follows the `ACCELERATOR_LINEAR_TOKEN_CMD` indirection pattern
  established by the Jira integration. The resolved token is a Linear personal
  API key (`lin_api_...`) passed in the `Authorization` header without a
  `Bearer` prefix — using `Bearer` causes authentication failure.
- A single team is configured at `init-linear` time and stored in the
  catalogue; no per-invocation `--team` flag is required.
- Linear accepts Markdown natively for issue descriptions and comments;
  no conversion layer is needed.
- WorkflowState UUIDs are team-scoped and cached by `init-linear`; state
  transitions resolve names to UUIDs from the cache, not via live API calls.
- Binary file uploads via `attach-linear-issue` use a two-step server-side
  pre-signed URL flow (`fileUpload` mutation → HTTP PUT); this cannot be done
  client-side.

## Technical Notes

- A dedicated `linear-graphql.sh` request helper is required; `jira-request.sh`
  cannot be reused due to the differing rate-limit response format and GraphQL
  construction. It should handle: query/mutation construction, Relay-style
  cursor pagination (`pageInfo.hasNextPage` + `endCursor`), and `400 RATELIMITED`
  detection with backoff using `X-RateLimit-Requests-Reset` (epoch ms).
- The `schpet/linear-cli` (TypeScript, v2.0.0 April 2026, actively maintained)
  is a useful reference for auth patterns, the `LINEAR_API_KEY` env var
  convention, and agent-friendly UX (`--json` output, VCS-aware context
  detection).
- Linear personal API keys are long-lived and user-scoped, providing access to
  all teams the user is a member of. `init-linear` should call
  `teams { nodes { id name key } }` to discover and select the target team.
- Rate limits: 2,500 requests/hr and 3,000,000 complexity points/hr per user
  for API keys. Single query maximum: 10,000 complexity points.

## Drafting Notes

- `linear-graphql.sh` helper kept in Technical Notes only — implementation
  detail, not user-facing concern.
- Storage references use "the linear subdirectory under the configured
  integrations path" as the integrations path is being made configurable in a
  separate change.
- Eight skills (not seven as in the epic) — `attach-linear-issue` added since
  both link-based and binary file upload attachments are confirmed supported by
  the API.
- Linear uses Markdown natively — confirmed via research; no ADF-equivalent
  conversion needed.
- Single-team scoping at `init-linear` time confirmed by user; team UUID and
  key stored in catalogue.

## References

- Source: `meta/work/0045-work-management-integration.md`
- Research: Linear Developers documentation (auth, rate limiting, pagination,
  attachments, WorkflowStates)
- Research: [schpet/linear-cli](https://github.com/schpet/linear-cli)
