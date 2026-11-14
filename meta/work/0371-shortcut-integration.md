---
type: "work-item"
id: "0371"
title: "Shortcut Integration"
date: "2026-11-13T22:45:09+00:00"
author: "Tom Scott"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0181"
relates_to: ["work-item:0171", "work-item:0290", "work-item:0291"]
tags: ["work-management", "integrations", "shortcut", "sync"]
last_updated: "2026-11-13T22:45:09+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---
# 0371: Shortcut Integration

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Tom Scott

## Summary

As a developer whose team tracks work in Shortcut, I want Accelerator to sync
local work items with Shortcut stories and epics, so that I can work from the
filesystem as Jira and Linear users already do.

This adds Shortcut as a third remote tracker. It gives `work.integration:
shortcut`, an `accelerator-shortcut` sub-binary, and eight
`*-shortcut-*` skills, built the way the Jira and Linear connectors are.

## Context

Epic 0181 groups the trackers still to add after Jira and Linear. The
`RemoteTracker` port, the sync engine and the shared contract suite already
exist (work items 0204, 0194 and 0210), so a new tracker adds a client crate,
a composition-root arm, a binary and skills, without changing the port.

Shortcut differs from the precedents in four ways:
- Story ids are bare integers. Users see them as `sc-1234`.
- Epics are separate entities with their own endpoints.
- The v4 API puts the workspace slug in every path and uses bearer tokens.
- Teams have no key, only a uuid, a name and a nullable mention name.

## Requirements

- A `shortcut-client` adapter crate implements `RemoteTracker` against
  Shortcut's REST API v4.
- `work.integration: shortcut` syncs each work item's title and body.
- `create` sends kind `epic` to a Shortcut epic. It maps every other kind to a
  story, with `story`→`feature`, `bug`→`bug`, and `task` and `spike`→`chore`.
- Synced items carry `external_id: "sc-<id>"` for a story and
  `"epic-<id>"` for an epic.
- Teams are named by their team key: the mention name, or the display name
  when there is none.
- `catalogue.json` holds the workspace, teams and workflows. Only
  `accelerator shortcut init discover` writes it.
- An `accelerator-shortcut` sub-binary provides `init`, `show`, `search`,
  `create`, `update`, `comment`, `transition` and `attach`.
- Eight skills drive the binary. `attach` adds URL links only.
- Pull filters keep Jira's and Linear's meaning: values for one field are
  OR'd, and fields are AND'd.

Out of scope:
- status, priority, label, owner or estimate sync (0290 owns field mapping);
- parent/child sync, including a story's `epic_id` (0291);
- epics in discovery;
- file upload for `attach`;
- `id_pattern: "{tracker}"` for Shortcut;
- the v3 API.

## Acceptance Criteria

- [ ] Given `work.integration: shortcut`, a token and a catalogue, when the
      user runs `/sync-work-items` and pushes a local `story`, then Shortcut
      holds a `feature` story and the work item gains
      `external_id: "sc-<id>"`.
- [ ] Given the same setup, when the user pushes a local `epic`, then
      Shortcut holds an epic and the work item gains
      `external_id: "epic-<id>"`.
- [ ] Given a synced story edited in Shortcut, when the user syncs, then the
      change is pulled into the work item.
- [ ] Given a story edited on both sides, when the user syncs, then a
      conflict dossier appears, and `adopt:sc-<id>` resolves it.
- [ ] Given a story created in Shortcut under the configured team, when sync
      runs discovery, then the story appears as an untracked remote.
- [ ] Given `additional_teams` or `all_teams`, when sync runs discovery, then
      every configured or visible team is searched by uuid, and no other
      team's stories appear.
- [ ] Given a read-only `sct_r_` token, when the user pushes, then each item
      fails as retryable, and the message names the read-only scope.
- [ ] Given no `catalogue.json`, when any non-init command runs, then it makes
      no request, and the message says to pull a teammate's catalogue or run
      `/accelerator:init-shortcut`.
- [ ] Each of `accelerator shortcut init`, `show`, `search`, `create`,
      `update`, `comment`, `transition` and `attach` works, and its skill
      drives it.
- [ ] The offline contract suite passes in the default profile for a story
      subject and an epic subject. The live suite passes against a scratch
      workspace through `mise run test:integration:tracker-contract`.
- [ ] `mise run` exits 0.
- [ ] The docs site has a Shortcut tab on the issue-trackers page, and a page
      for the sub-binary.

## Dependencies

- Blocked by: none. The port, the sync engine and the contract suite exist.
- Blocks: none yet. Phase 7 of the plan creates follow-up items for file
  upload, parent/child sync, epics in discovery, status and priority mapping,
  a tracker registry, and client duplication.

## Assumptions

- Shortcut's v4 API, in beta, stays close enough to its published spec for
  the plan's Phase 2 live evidence to hold. If v4 changes incompatibly, the
  `paths` module and the fixtures change.
- Teams that use Shortcut mostly name teams by mention name. Teams without
  one fall back to the display name.

## Technical Notes

The approved plan holds the design, the seven phases and the test list. The
phases merge in the order 1, 2, 3, 5, 4, 6, 7.

## Drafting Notes

- I chose `story` rather than `epic`: this is one connector, and the plan's
  phases are pull requests, not child work items.
- I chose `medium` priority, because no deadline came up. The parent epic is
  `high`.
- The acceptance criteria restate the plan's Desired End State and manual
  checks. The plan stays the one detailed source.

## References

- Plan: `meta/plans/2026-11-13-0371-shortcut-integration.md`
- Review: `meta/reviews/plans/2026-11-13-0371-shortcut-integration-review-1.md`
- Parent epic: `meta/work/0181-work-management-additional-integrations.md`
- Related: 0171, 0204, 0194, 0210, 0211, 0290, 0291
- Shortcut v4 reference: https://developer.shortcut.com/api/rest/v4
