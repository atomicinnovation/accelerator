---
work_item_id: "0049"
title: "Trello Integration"
date: "2026-05-06T17:49:44+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: "0045"
tags: [work-management, integrations, trello]
---

# 0049: Trello Integration

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Implement eight REST-based skills under `skills/integrations/trello/` covering
the full CRUD lifecycle against Trello's API. Workflow is list-position rather
than a state machine; `init-trello` auto-generates a list-name → local status
mapping (`status-map.json`) from the board's existing lists, prompts the user
to confirm, and supports hand-editing afterwards. Trello card `shortLink` is
used as `work_item_id`.

## Context

Trello organises work as cards within named lists on a single board. There is
no formal workflow state — a card's `idList` is the workflow signal. The
`init-trello` skill captures a list-name → local status mapping that the sync
skill uses to translate between list positions and local work item status
values. Markdown is accepted natively for descriptions and comments; no
conversion layer is required, in contrast to Jira's ADF integration. The
pattern follows the established Jira integration under `skills/integrations/jira/`.

## Requirements

- Eight skills:
  - `init-trello` — authenticate, persist board details (board ID, list ID
    catalogue, label catalogue) and an auto-generated list-name → local status
    mapping (`status-map.json`) to the trello subdirectory under the configured
    integrations path; confirm the auto-generated mapping with the user before
    writing and document that it is hand-editable afterwards
  - `search-trello-cards` — query cards by filter; uses Trello's search DSL
    where applicable, falling back to board-level enumeration with
    `before`/`since` paging
  - `show-trello-card` — display a single card
  - `create-trello-card` — create a card; write the card's `shortLink` as
    `work_item_id` in the local work item file
  - `update-trello-card` — update fields on an existing card
  - `move-trello-card` — move a card to a different list using `idList`,
    resolving local status values to list IDs via `status-map.json`
  - `comment-trello-card` — add a Markdown comment to a card via the
    `commentCard` action endpoint
  - `attach-trello-card` — attach a resource to a card: URL-only attachments
    and binary file uploads (`multipart/form-data`)

## Acceptance Criteria

- [ ] Given `init-trello` is run with valid credentials, when authentication
  succeeds, then board details, list ID catalogue, label catalogue, and an
  auto-generated `status-map.json` are persisted to the trello subdirectory
  under the configured integrations path
- [ ] Given the auto-generated `status-map.json` is presented at `init-trello`
  time, when the user confirms, then the mapping is written; subsequent manual
  edits to the file are honoured by all skills
- [ ] Given `init-trello` has been run, when `create-trello-card` is invoked,
  then a Trello card is created and its `shortLink` is written as
  `work_item_id` in the local work item file
- [ ] Given a local status value, when `move-trello-card` is invoked, then the
  skill resolves the corresponding list ID from `status-map.json` and moves the
  card via `PUT /1/cards/{id}/idList`
- [ ] Given the API responds with HTTP `429`, when any Trello skill handles
  the response, then the error is surfaced clearly and a backoff is computed
  from the `X-Rate-Limit-Api-Token-*` response headers
- [ ] Given a results set spans multiple pages, when listing or searching, then
  pagination via `before`/`since` (using card IDs as cursors) retrieves all
  pages transparently, deduplicating by `id`
- [ ] Given a URL or local file path, when `attach-trello-card` is invoked,
  then the appropriate Trello attachment endpoint is used (URL via
  `?url=...&name=...`; file via `multipart/form-data` with a `file` field)

## Open Questions

- —

## Dependencies

- Blocked by: 0046
- Blocks: 0051

## Assumptions

- Auth uses both an API key and a token. The Accelerator pattern extends to
  `ACCELERATOR_TRELLO_KEY_CMD` and `ACCELERATOR_TRELLO_TOKEN_CMD` indirection
  (mirroring the Jira `ACCELERATOR_JIRA_TOKEN_CMD` pattern). Direct
  `TRELLO_API_KEY` and `TRELLO_TOKEN` env vars are also accepted as fallback.
- A single board is configured at `init-trello` time and stored in the
  catalogue; no per-invocation `--board` flag is required.
- Trello card `shortLink` (8-char alphanumeric, e.g. `AbCd1234`) is used as
  `work_item_id` for readability — shortLinks appear in card URLs and are
  recognisable to users. Both `id` (24-char hex) and `shortLink` work
  interchangeably in API paths; either is stable for the card's lifetime.
- Trello accepts Markdown natively for descriptions and comments; no
  conversion layer is needed.
- `status-map.json` is hand-editable after `init-trello` runs.
- File upload size limit follows the workspace plan (10 MB on Free; 250 MB on
  Standard/Premium/Enterprise); URL attachments are not subject to this cap.

## Technical Notes

- A dedicated `trello-request.sh` request helper handles: auth header/query
  injection, `before`/`since` cursor pagination (using card `id` as cursor),
  rate-limit handling on `429` with backoff via `X-Rate-Limit-Api-Token-*`
  headers, and `multipart/form-data` for file uploads.
- Trello's search endpoint caps results at 1000 cards; `search-trello-cards`
  should fall back to board-level enumeration with paging when the cap may be
  exceeded.
- Comments are modelled as `commentCard` Actions, not first-class comments —
  add via `POST /1/cards/{id}/actions/comments`, list via
  `GET /1/cards/{id}/actions?filter=commentCard`.
- Workflow note: list IDs are stable but list names are not. The `status-map.json`
  should be persisted using list IDs (with names alongside as documentation),
  so that renaming a Trello list does not silently break sync.
- The `/1/batch` endpoint (up to 10 GETs in one request) is available and may
  be useful if the per-token ceiling of 100 requests / 10 seconds becomes
  limiting in practice.
- Rate-limit research note: stability of `shortLink` for a card's lifetime is
  implied by Trello docs and community evidence but not explicitly guaranteed.
  If a future surprise occurs (a card's `shortLink` changes), this is a known
  caveat — consider keeping the 24-char `id` in a secondary frontmatter field
  for fallback resolution.

## Drafting Notes

- `trello-request.sh` helper kept in Technical Notes only — implementation
  detail, not user-facing.
- Storage references use "the trello subdirectory under the configured
  integrations path" — consistent with the Linear integration story.
- Eight skills (matching the epic): `attach-trello-card` covers both URL and
  binary attachments per user confirmation.
- `shortLink` retained as `work_item_id` per the epic's choice, with the
  research caveat captured in Technical Notes.
- `status-map.json` should reference list IDs not list names internally —
  this is a research-derived insight (list names mutate; list IDs do not) and
  worth flagging early so the implementing plan does not accidentally key on
  fragile names.

## References

- Source: `meta/work/0045-work-management-integration.md`
- Research: Atlassian Trello REST API documentation (auth, rate limiting,
  pagination, attachments, search, comments, lists)
