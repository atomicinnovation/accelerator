---
id: "0051"
title: "Sync Work Items Skill"
date: "2026-05-06T17:49:44+00:00"
author: Toby Clemson
kind: story
status: done
priority: medium
parent: "work-item:0045"
tags: [work-management, integrations, sync]
type: work-item
schema_version: 1
last_updated: "2026-06-18T12:36:36+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0046", "work-item:0047"]
---

# 0051: Sync Work Items Skill

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Implement `/sync-work-items`, an on-demand sync skill that reconciles local
work items in `meta/work/` with the active remote system configured by
`work.integration`. Defaults to bidirectional sync; supports `--push-only`,
`--pull-only`, and `--preview` modes. Sync uses a timestamp pre-filter with an
authoritative normalised-content comparison against the `last-sync.json`
baseline; on conflict (both sides changed since last sync), the user
is shown a section-by-section diff and must explicitly confirm or override
before any write occurs. Because this skill produces the `last-sync.json`
baseline, it also extends `/list-work-items` to render the three
baseline-dependent sync states (locally modified, remotely modified, conflict),
completing the five-state display whose baseline-independent subset
(synced/unsynced) ships in 0047. Both surfaces ship in one story because the
local-vs-remote-vs-baseline comparison that classifies the states lives in this
skill — rendering them in `/list-work-items` reuses that single derivation
rather than duplicating it.

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
- A work item is synced when its `external_id` is present and unsynced (never
  pushed) when `external_id` is absent — the sync-state classification rule
  defined in 0047. The local `id` always stays local and is never pushed.
- For each local work item with an `external_id` (a synced item):
  - Classify the item with the change-detection contract (Assumptions): a
    timestamp pre-filter against `last-sync.json`, followed by an authoritative
    normalised-content comparison against the stored baseline
  - When local has changed and remote has not (and mode permits push):
    push local changes to remote
  - When remote has changed and local has not (and mode permits pull):
    update local file from remote
  - When both have changed: present a section-by-section diff; the confirmation
    prompt's default answer is the remote version, but no local write occurs
    without explicit user confirmation, and the user may override to push local
    in place of accepting remote. In `--push-only` and `--pull-only`,
    conflicting items are instead reported in the summary and skipped, since
    resolving them would require a write the mode forbids
- For each local work item with no `external_id` (never pushed):
  offer to push to the remote — accept/decline per item, or batch
  accept/decline all
- For untracked remote issues:
  - By default, pull only those within `work.default_project_code`
  - Accept every filter option the active integration's `search-*`/`list-*`
    skill accepts (e.g. assignee, label, state), narrowing the pulled set
    identically to that skill
  - Accept an `--all` flag that bypasses only the implicit
    `work.default_project_code` project scope; any user-supplied filters
    (assignee, label, state) still apply
- Persist `last-sync.json` to the active integration's subdirectory under the
  configured integrations path on successful completion (not in `--preview`
  mode)
- Extend `/list-work-items` to render the three baseline-dependent sync states
  once `last-sync.json` exists: using the per-item status slot established by
  0047, reuse this skill's local-vs-remote-vs-baseline comparison to label each
  tracked item as **locally modified**, **remotely modified**, or **conflict**,
  alongside the synced/unsynced labels 0047 already renders. This requires a
  per-item remote read at list time; items with no baseline continue to show
  only synced/unsynced

## Acceptance Criteria

- [ ] Given `work.integration` is not configured, when `/sync-work-items` is
  invoked, then the skill exits with a clear error explaining that an
  integration must be configured first
- [ ] Given `--preview` is supplied with any directional flag (or none), when
  `/sync-work-items` runs, then the skill reports the full set of intended
  changes (push, pull, conflict, push-unsynced) without making any local writes
  or remote API writes, and `last-sync.json` is not updated
- [ ] Given default mode and no conflicts, when `/sync-work-items` completes,
  then locally tracked items reflect remote changes where remote was ahead,
  local-ahead changes are pushed, untracked remote issues within
  `work.default_project_code` are created as local work items, and
  `last-sync.json` is updated
- [ ] Given a local item differs from its remote counterpart only in trailing
  whitespace and remote-managed fields (e.g. `updated_at`), when
  `/sync-work-items` runs in default mode, then — per the normalisation rule in
  Assumptions — the item is classified as unchanged and is neither pushed nor
  flagged as a conflict
- [ ] Given an untracked remote issue within `work.default_project_code`, when
  `/sync-work-items` runs in default (or `--pull-only`) mode, then a local work
  item is created carrying the remote-allocated key as its `external_id` (its
  local `id` allocated independently)
- [ ] Given `--push-only`, when `/sync-work-items` runs, then no local file is
  modified and only local-ahead items are pushed to the remote
- [ ] Given `--pull-only`, when `/sync-work-items` runs, then no remote API
  write occurs and only locally tracked remote-ahead items and untracked
  remote issues are written locally
- [ ] Given a conflict exists and the mode is `--push-only` or `--pull-only`,
  when `/sync-work-items` runs, then the conflicting item is reported in the
  summary and skipped — no resolution prompt is shown and neither side is
  written
- [ ] Given a conflict exists, when `/sync-work-items` encounters it, then the
  user is shown a section-by-section diff (grouped by work item section) in which
  the remote side is labelled as the default choice and the confirmation prompt's
  default answer accepts the remote version, and the local file is only
  overwritten after explicit user confirmation
- [ ] Given a conflict and the user supplies an override, when the user
  confirms, then the local version is pushed to the remote and a line recording
  the override (the item's `id` and the direction chosen) is emitted to
  the sync summary output
- [ ] Given local work items with no `external_id` exist, when
  `/sync-work-items` runs, then the user is offered to push each one per item;
  accepted items are pushed and the key returned by the integration's create
  operation is written to their `external_id` (their local `id` unchanged),
  and declined items remain unchanged
- [ ] Given multiple local work items with no `external_id` exist, when
  the user chooses the batch accept-all (or decline-all) option, then all are
  pushed (or all left unchanged) in one action, with accepted items'
  `external_id` set to their remote-allocated keys
- [ ] Given filter options accepted by the active integration's `search-*` skill
  are supplied (e.g. `--assignee`, `--label`, `--state`), when pulling untracked
  remote issues, then the set of issues `/sync-work-items` pulls equals the set
  the integration's `search-*` skill returns for the same filter arguments
  against the same remote state
- [ ] Given `--all`, when pulling untracked remote issues, then the
  `work.default_project_code` project-scope filter is bypassed while any
  user-supplied filters (e.g. `--label`) still apply
- [ ] Given a sync in which item A has been reconciled and `last-sync.json`
  updated for A, but the process is killed before item B is processed, when
  `/sync-work-items` is re-run, then A is not re-pushed or re-pulled, B is
  reconciled exactly once, and no remote-only item already created locally is
  created a second time
- [ ] Given `last-sync.json` exists and a tracked item's local content has
  changed since last sync while remote has not, when `/list-work-items` is
  invoked, then the item renders the **locally modified** label
- [ ] Given `last-sync.json` exists and a tracked item's remote content has
  changed since last sync while local has not, when `/list-work-items` is
  invoked, then the item renders the **remotely modified** label
- [ ] Given `last-sync.json` exists and both local and remote content of a
  tracked item have changed since last sync, when `/list-work-items` is invoked,
  then the item renders the **conflict** label
- [ ] Given the full set of five sync states is renderable, when
  `/list-work-items` displays labels, then no two states share an identical
  label+colour pairing
- [ ] Given `last-sync.json` exists but the remote read fails (unreachable or
  rate-limited), when `/list-work-items` is invoked, then every item still
  renders with at least its synced/unsynced label and the command exits
  successfully without error or hang

## Open Questions

- Per-integration extensions to the normalisation ignored-field set — the
  minimum rule is fixed in Assumptions; only integration-specific additions are
  deferred to the implementing plan.
- Per-item commit semantics for resumability (ordering of local write, remote
  write, and `last-sync.json` update) — deferred to the implementing plan.
- The exact sync-summary format for the override-log line — deferred to the
  implementing plan.

## Dependencies

- Blocked by: 0046 (`work.integration` config) and 0047. The hard prerequisite
  is the per-item status-slot seam this story's `/list-work-items` extension
  builds on (the extension cannot be built until it exists); the reuse of
  `/create-work-item`'s confirmation-prompt style by the conflict-resolution
  override UX is an additional consistency tie on the same edge, not a separate
  blocker.
- Integration capability: requires at least one integration's read/search and
  create APIs (the `search-*`/`list-*` skills are reused to fetch remote state
  and the create skill to push unsynced items); Jira is already complete, so
  this is satisfied today. Sync's filter flags are contractually coupled to the
  active integration's `search-*` skill. 0048 (Linear), 0049 (Trello), and 0050
  (GitHub Issues) extend sync coverage to their remotes as they land but are not
  blockers.
- Blocks: — (no hard downstream blocker). Non-blocking data relationship: 0047's
  three deferred sync states consume the `last-sync.json` baseline this story
  produces; this is intentionally not modelled as a `blocks` edge because 0051 is
  itself `blocked_by` 0047 and the reverse edge would create a cycle.

## Assumptions

- Sync is on-demand (user-invoked), not background or scheduled.
- **Change-detection contract.** A timestamp pre-filter selects candidates and a
  normalised-content comparison is authoritative. The two never disagree because
  the pre-filter may only short-circuit to *unchanged*, never declare *changed*:
  - **Local side** — if the local file's modification time is at or before the
    global `last-sync.json` `timestamp`, the local side is unchanged. Otherwise
    the current local file is re-normalised and re-hashed and that digest is
    compared against the per-item `local_hash` baseline in `last-sync.json`, and
    counts as changed only if the digests differ (so a touch or reformat that
    leaves content equivalent is not a change).
  - **Remote side** — if the integration's `updated_at` equals the per-item
    `remote_updated_at` baseline, the remote side is unchanged. Otherwise the
    remote content is fetched and compared against that baseline, and counts as
    changed only if the normalised content differs.
  - The four states follow from the (local-changed, remote-changed) pair:
    neither → synced, local-only → locally modified, remote-only → remotely
    modified, both → conflict. Sync's push/pull/conflict decisions use the same
    signals. SHA-based three-way merge is out of scope — the comparison is a
    two-way normalised-equality check against the stored baseline.
- "Section-by-section diff" means a textual diff grouped by work item section
  (frontmatter, Summary, Context, Requirements, Acceptance Criteria, etc.) so
  large work items remain reviewable; rich UI is out of scope.
- Remote-default-on-conflict is the policy: the user must explicitly override
  to keep local.
- The integration's read APIs (e.g. `search-jira-issues`, `search-linear-issues`,
  `search-trello-cards`, `search-github-issues`) are reused by `/sync-work-items`
  to fetch remote state and to honour user-supplied filters; no separate
  sync-only fetch path is added.
- **Normalisation rule (minimum, fixed here).** Two contents are equivalent when
  they match after: (a) trimming leading/trailing whitespace per line and
  trailing newlines, and (b) ignoring fields not part of the locally-authored
  work item — remote-allocated/managed fields such as `updated_at`, and any field
  absent from the local work-item schema. The implementing plan may extend the
  ignored-field set per integration but may not narrow this minimum.
- `--preview` is justified as an exception to the general "no preview UX on
  destructive ops" policy because sync writes can affect remote state, which
  is not recoverable via local VCS revert. Local-only destructive ops still
  rely on VCS for recovery.
- The baseline-dependent `/list-work-items` states require a live per-item remote
  read; when the remote is unreachable or rate-limited, `/list-work-items`
  degrades gracefully to showing only synced/unsynced rather than failing.

## Technical Notes

- **Size**: L — a net-new orchestration skill spanning four modes, bidirectional
  reconciliation with conflict UX, unsynced batch push, untracked-issue
  pulling, `last-sync.json` persistence with crash-safe resumability, plus the
  `/list-work-items` three-state rendering extension. Larger than the M-sized
  integration siblings; the capstone of the epic.
- `last-sync.json` lives in the active integration's subdirectory under the
  configured integrations path (e.g.
  `.accelerator/state/integrations/jira/last-sync.json`).
  Schema: `{ "timestamp": "<ISO8601>", "items": { "<id>": {
  "remote_updated_at": "<ISO8601>", "local_hash": "<hash of normalised local
  content at last sync>" } } }`, keyed by the stable local `id`; the live
  file's `external_id` locates the remote counterpart. Per item,
  `remote_updated_at` is the remote
  baseline and `local_hash` is the local baseline the change-detection contract
  compares against; the global `timestamp` is the local-mtime pre-filter
  reference. `local_hash` is a digest of the normalised local content used purely
  as a two-way equality check against the baseline — distinct from the SHA-based
  three-way merge ruled out in Assumptions. The per-item map allows fine-grained
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
- The `/list-work-items` extension depends on the extensible per-item status slot
  established by 0047 (this story is `blocked_by` 0047, so the seam is present).
  It adds a per-item remote read to `/list-work-items` for the three
  baseline-dependent states, and reuses the same last-modified-vs-`last-sync.json`
  comparison this skill applies during reconciliation — keeping state derivation
  in a single source of truth rather than duplicating it across the two skills.

## Drafting Notes

- Priority set to medium: this is the capstone of the epic. It is unblocked once
  0046, 0047, and any one integration are done — Jira (complete) satisfies the
  integration requirement today — so the gating is lighter than "all planned
  stories". Priority can be raised as the remaining integrations land and broaden
  sync coverage.
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
- The `/list-work-items` rendering of the three baseline-dependent states was
  folded into this story when 0047's scope was narrowed to the two
  baseline-independent states (synced/unsynced). 0047 leaves the status-slot
  seam; 0051 produces the baseline and completes the five-state display, so the
  state-derivation logic lives in one place rather than being split across the
  two skills.

- Sync-state classification was re-based on `external_id` presence (synced)
  vs absence (never pushed) after the `work_item_id` field was retired: `id`
  is now the always-local identity and `external_id` holds the remote key. The
  obsolete numeric-vs-remote-format (`^[0-9]+$`) distinction was removed
  throughout. `last-sync.json` is keyed by local `id` (always present and
  stable on disk) rather than `external_id`, with the live file's `external_id`
  used to locate the remote counterpart — reconciling 0051 to the 0047
  `external_id` convention.

## References

- Source: `meta/work/0045-work-management-integration.md`
