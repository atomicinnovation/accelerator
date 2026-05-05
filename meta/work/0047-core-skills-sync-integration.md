---
work_item_id: "0047"
title: "Core Skills Sync Integration"
date: "2026-05-06T17:49:44+00:00"
author: Toby Clemson
type: story
status: draft
priority: high
parent: "0045"
tags: [work-management, integrations, sync, list-work-items, create-work-item]
---

# 0047: Core Skills Sync Integration

**Type**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

When `work.integration` is configured, extend `/list-work-items` to show a
colour-coded sync status label per item reflecting content parity with the
remote, and extend `/create-work-item` to offer an interactive push to the
remote after creation. On push acceptance, the remote allocates the issue ID,
which is written as `work_item_id` in the local file; on decline, a local
numeric ID is used instead.

## Context

With `work.integration` configured (prerequisite story 0046), the core work
management skills need sync awareness. Two skills are in scope: `/list-work-items`
gains visibility into the sync state of each work item relative to the remote,
and `/create-work-item` gains a post-creation push offer so newly created items
can be pushed immediately. The convention underpinning both is that a numeric
`work_item_id` means "never pushed"; a remote-format ID means the item exists
in the remote system, but content may or may not be current.

## Requirements

- `/list-work-items`: when `work.integration` is configured, display a
  colour-coded sync status label inline with each item's ID reflecting content
  parity with the remote:
  - **synced** — the item exists in the remote system and its content is
    logically equivalent to the remote (no changes on either side since last sync)
  - **unsynced** — the item has never been pushed (numeric `work_item_id`); it
    does not exist in the remote system
  - **locally modified** — local content has changed since last sync; remote has not
  - **remotely modified** — remote content has changed since last sync; local has not
  - **conflict** — both local and remote content have changed since last sync;
    visually distinct from locally modified and remotely modified
- `/create-work-item`: when `work.integration` is configured, present an
  interactive confirmation prompt after the work item is drafted, offering to
  push it to the remote
  - On acceptance: the remote creates the issue, returns its key, and the local
    file is written with the remote-allocated key as `work_item_id`; on push
    failure, offer a retry; if retry also fails, fall back to saving locally
    with a numeric ID and inform the user to sync later
  - On decline: the local file is written with a local numeric ID as
    `work_item_id`
  - The local file is not written until the push succeeds, the user declines,
    or push has failed and fallback to local save is confirmed

## Acceptance Criteria

- [ ] Given `work.integration` is not configured, when `/list-work-items` is
  invoked, then no sync status label is shown
- [ ] Given `work.integration` is configured, when `/list-work-items` is
  invoked, then each item shows a colour-coded label reflecting content parity:
  synced (content matches on both sides), unsynced (never pushed, numeric ID),
  locally modified (local ahead of remote), remotely modified (remote ahead of
  local), or conflict (both sides changed since last sync)
- [ ] Given `work.integration` is configured and a work item is remotely
  modified, when `/list-work-items` is invoked, then the item is visually
  distinct from locally modified and conflict items
- [ ] Given `work.integration` is configured and `/create-work-item` completes,
  then an interactive confirmation prompt is shown offering to push to the remote
- [ ] Given the user accepts the push offer and the push succeeds, then the
  local file is written once with the remote-allocated key as `work_item_id`
- [ ] Given the user accepts the push offer and the push fails, then a retry is
  offered; if the retry also fails, the local file is saved with a numeric ID
  and the user is informed to run `/sync-work-items` later
- [ ] Given the user declines the push offer, then the local file is written
  with a numeric `work_item_id`

## Open Questions

- —

## Dependencies

- Blocked by: 0046
- Blocks: 0051 (conflict indicator depends on `last-sync.json`)

## Assumptions

- The numeric-ID-as-unsynced convention (a numeric `work_item_id` means "never
  pushed") is the canonical signal for the unsynced state; all other states
  require `last-sync.json` to exist.
- Items in repos that have never run `/sync-work-items` have no `last-sync.json`
  and therefore show only synced or unsynced — locally modified, remotely
  modified, and conflict states are not available without a baseline sync
  timestamp.
- "Colour-coded" means terminal ANSI colour output; the specific colour scheme
  is left to the implementing plan.

## Technical Notes

- The retry-then-fallback failure path means `/create-work-item` must hold the
  drafted work item in memory across at least two network attempts before
  deciding whether to write the local file.
- The implementing plan should define how many retries are attempted before the
  fallback is offered.

## Drafting Notes

- Sync status is about content parity with the remote, not ID format — a
  remote-format ID alone does not mean synced. This was clarified during
  extraction; the epic's language was ID-centric but intent is content-centric.
- Five sync states defined: synced, unsynced, locally modified, remotely
  modified, conflict. The epic named only three (synced, unsynced, conflict);
  locally modified and remotely modified were added to cover the asymmetric
  cases, confirmed during extraction.
- Push failure UX: retry-first, then fall back to local save with numeric ID.
- Colour scheme for sync status labels left to implementing plan.

## References

- Source: `meta/work/0045-work-management-integration.md`
