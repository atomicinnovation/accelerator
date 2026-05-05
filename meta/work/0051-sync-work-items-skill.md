---
work_item_id: "0051"
title: "Sync Work Items Skill"
date: "2026-05-06T17:49:44+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: "0045"
tags: [work-management, integrations, sync]
---

# 0051: Sync Work Items Skill

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Implement `/sync-work-items`, an on-demand sync skill that reconciles local
work items in `meta/work/` with the active remote system configured by
`work.integration`. Defaults to bidirectional sync; supports `--push-only`,
`--pull-only`, and `--preview` modes. Sync is timestamp-based against
`last-sync.json`; on conflict (both sides changed since last sync), the user
is shown a section-by-section diff and must explicitly confirm or override
before any write occurs.

## Context

With `work.integration` configured and at least one integration implemented
(Jira, Linear, Trello, or GitHub Issues), the work management system needs a
unified sync surface. `/sync-work-items` reads `work.integration` to identify
the active system and dispatches to that integration's read and write APIs
to reconcile state. Because sync writes can affect remote state — which is
not recoverable via VCS revert — a preview mode is provided to inspect the
plan before any side effects occur.

## Requirements

- `/sync-work-items` reads `work.integration` to identify the active remote
  system and dispatches to that integration's read and write APIs
- Supports four modes:
  - **bidirectional** (default) — reconcile both directions
  - `--push-only` — apply local changes to remote; do not pull remote changes
  - `--pull-only` — apply remote changes locally; do not push local changes
  - `--preview` — report what would change in any of the above modes without
    applying any writes; combinable with the directional flags
- For each local work item with a remote-format `work_item_id`:
  - Compare local and remote state against `last-sync.json` timestamps
  - When local has changed and remote has not (and mode permits push):
    push local changes to remote
  - When remote has changed and local has not (and mode permits pull):
    update local file from remote
  - When both have changed: present a section-by-section diff; default to
    remote; require explicit user confirmation, or accept an override to push
    local in place of accepting remote
- For each local work item with a numeric `work_item_id` (never pushed):
  offer to push to the remote — accept/decline per item, or batch
  accept/decline all
- For untracked remote issues:
  - By default, pull only those within `work.default_project_code`
  - Accept the same filter options as the integration's `search-*`/`list-*`
    skills (e.g. assignee, label, state) to narrow further
  - Accept an `--all` flag to pull everything the integration's read APIs
    return
- Persist `last-sync.json` to the active integration's subdirectory under the
  configured integrations path on successful completion (not in `--preview`
  mode)

## Acceptance Criteria

- [ ] Given `work.integration` is not configured, when `/sync-work-items` is
  invoked, then the skill exits with a clear error explaining that an
  integration must be configured first
- [ ] Given `--preview` is supplied with any directional flag (or none), when
  `/sync-work-items` runs, then the skill reports the full set of intended
  changes (push, pull, conflict, push-numeric) without making any local writes
  or remote API writes, and `last-sync.json` is not updated
- [ ] Given default mode and no conflicts, when `/sync-work-items` completes,
  then locally tracked items reflect remote changes where remote was ahead,
  local-ahead changes are pushed, untracked remote issues within
  `work.default_project_code` are created as local work items, and
  `last-sync.json` is updated
- [ ] Given `--push-only`, when `/sync-work-items` runs, then no local file is
  modified and only local-ahead items are pushed to the remote
- [ ] Given `--pull-only`, when `/sync-work-items` runs, then no remote API
  write occurs and only locally tracked remote-ahead items and untracked
  remote issues are written locally
- [ ] Given a conflict exists, when `/sync-work-items` encounters it, then the
  user is shown a section-by-section diff with the remote version highlighted
  as default, and the local file is only overwritten after explicit user
  confirmation
- [ ] Given a conflict and the user supplies an override, when the user
  confirms, then the local version is pushed to the remote and the choice is
  logged
- [ ] Given local work items with numeric `work_item_id` exist, when
  `/sync-work-items` runs, then the user is offered to push them — per item or
  in batch; accepted items are pushed and their `work_item_id` is rewritten
  with the remote-allocated key, declined items remain unchanged
- [ ] Given filter options are supplied (e.g. `--assignee`, `--label`,
  `--state`), when pulling untracked remote issues, then only matching issues
  are considered
- [ ] Given `--all`, when pulling untracked remote issues, then the project
  scope filter from `work.default_project_code` is bypassed
- [ ] Given a sync is interrupted partway through, when the user re-runs
  `/sync-work-items`, then the skill resumes safely without duplicating
  remote-only items as local items or applying conflicting writes twice

## Open Questions

- —

## Dependencies

- Blocked by: 0046, 0047, and at least one of Jira (existing), 0048, 0049, 0050
- Blocks: —

## Assumptions

- Sync is on-demand (user-invoked), not background or scheduled.
- Conflict detection is last-modified timestamp-based: file modification time
  for local, the integration's `updated_at` (or equivalent) for remote, both
  compared against `last-sync.json`. SHA-based diffing and three-way merge
  are out of scope.
- "Section-by-section diff" means a textual diff grouped by work item section
  (frontmatter, Summary, Context, Requirements, Acceptance Criteria, etc.) so
  large work items remain reviewable; rich UI is out of scope.
- Remote-default-on-conflict is the policy: the user must explicitly override
  to keep local.
- The integration's read APIs (e.g. `search-jira-issues`, `search-linear-issues`,
  `search-trello-cards`, `search-github-issues`) are reused by `/sync-work-items`
  to fetch remote state and to honour user-supplied filters; no separate
  sync-only fetch path is added.
- Local content equivalence to remote content uses a normalised comparison
  (whitespace-tolerant, ignoring fields that the integration manages
  exclusively, e.g. remote-allocated timestamps); the implementing plan
  defines the specific normalisation rules per integration.
- `--preview` is justified as an exception to the general "no preview UX on
  destructive ops" policy because sync writes can affect remote state, which
  is not recoverable via local VCS revert. Local-only destructive ops still
  rely on VCS for recovery.

## Technical Notes

- `last-sync.json` lives in the active integration's subdirectory under the
  configured integrations path (e.g. `meta/integrations/jira/last-sync.json`).
  Schema: `{ "timestamp": "<ISO8601>", "items": { "<work_item_id>":
  "<last_remote_updated_at>" } }`. The per-item map allows fine-grained
  conflict detection without a single global timestamp.
- Resumability (the last AC) requires that writes to local files and the
  remote API are committed before `last-sync.json` is updated for that item.
  An incomplete run should leave items not yet processed in their pre-sync
  state, with `last-sync.json` reflecting only successfully reconciled items.
- Filter flags accepted by `/sync-work-items` should mirror those accepted by
  the integration's `search-*` skills to keep the user's mental model
  consistent.
- The conflict-resolution UX (override mechanism) should be consistent with
  `/create-work-item`'s confirmation prompt style, established by the Core
  skills sync integration story.
- Multi-system mirroring is out of scope per the epic's "one active integration
  at a time" assumption; `/sync-work-items` operates against exactly one
  integration per invocation.

## Drafting Notes

- Priority set to medium: this is the capstone of the epic but blocked by all
  other planned stories. Priority can be raised once the integrations land.
- Modes (`--push-only`, `--pull-only`, `--preview`) added beyond what the
  epic explicitly states, per user direction. They make first-time and
  exploratory sync runs safer.
- Numeric-ID push offer added: rather than skipping never-pushed items, the
  sync run offers to push them en masse or per item — a batch version of
  `/create-work-item`'s push offer.
- `--all` flag for ignoring `work.default_project_code` scope flagged
  explicitly. Without it, sync stays scoped to the configured project, which
  is the safe default.
- `--preview` is a deliberate exception to the "no preview UX on destructive
  ops" feedback. Justified because sync affects remote state, which VCS
  revert cannot reach.
- Resumability AC retained because mid-run interruption (network failure,
  ctrl-c) is realistic and the implementing plan needs guidance on the
  per-item commit semantics.
- Section-by-section diff chosen over whole-file diff because work items can
  be long and a flat diff is hard to review when many items conflict at once.

## References

- Source: `meta/work/0045-work-management-integration.md`
