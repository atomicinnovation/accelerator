---
type: "work-item"
id: "0290"
title: "Bidirectional Field Mapping for Work Item Sync"
date: "2026-09-20T19:15:54+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0146"
relates_to: ["work-item:0228", "work-item:0229", "work-item:0291"]
tags: ["sync", "tracker", "jira", "linear", "mapping", "status", "priority", "kind"]
last_updated: "2026-09-20T19:15:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-876"
---

# 0290: Bidirectional Field Mapping for Work Item Sync

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Synchronise work item status, kind, and priority bidirectionally between local
items and remote trackers (Jira, Linear), via per-tracker mapping tables with
built-in defaults and config overrides, last-writer-wins conflict resolution, and
skip-and-warn on unmappable values. Closes two of epic 0146's field-mapping
acceptance criteria.

## Context

The `RemoteIssue` port (`cli/tracker/src/lib.rs`) carries only `updated` and a
projected `body`; `fetch`/`apply` sync the body alone. Epic 0146 requires
status/kind/priority mapped bidirectionally, unrealised by any child. The trackers
differ materially: Jira has native issue type, a priority scheme, and workflow
status changed only through transitions; Linear has native workflow state (direct
`stateId` write) and integer priority (0=none,1=urgent,2=high,3=medium,4=low), but
no native issue type — kind is labels-only.

## Requirements

- Extend the `RemoteTracker` port so status, kind, and priority cross it as
  structured fields on both read (`RemoteIssue`) and write (push draft), not
  embedded in `body`.
- Ship built-in per-tracker default mapping tables between Accelerator's fixed
  vocabularies (status: draft/ready/in-progress/review/done/blocked/abandoned;
  kind: story/epic/task/bug/spike; priority: high/medium/low) and each tracker's
  values, both directions; allow config to override or extend them per tracker
  (reusing the config-catalogue structured-value extension from 0229).
- Priority: the default table documents the lossy collapse across Jira's five
  default levels, Accelerator's three, and Linear's 0–4 (including the Linear
  "0 = no priority" rule).
- Kind: Jira maps to native issue types by id, constrained to the project's
  issue-type scheme (a missing type such as spike falls back per the table).
  Linear kind is emulated via user-defined labels — the kind↔label table is
  config-defined, with no built-in default.
- Per-field, per-tracker opt-out: any mapped field can be disabled in config.
- Bidirectional application: pull maps remote→local, push maps local→remote;
  Jira status is written by discovering a valid workflow transition and posting it.
- Conflict resolution: three-way (baseline/local/remote); when both sides changed
  a field since baseline, the later timestamp wins (local `last_updated` vs remote
  `updated`), and the resolution is reported.
- Unmappable value (no table entry either direction, or a Linear issue matching
  zero or multiple mapped kind labels): leave the field unchanged, sync the rest of
  the item, warn with item/field/value.
- Mapped fields join the digest and baseline so divergence is detected (today only
  body is digested).

## Acceptance Criteria

- [ ] Given a remote field differs from the baseline and the local value is
      unchanged, when sync pulls, then the local field is set via the mapping table.
- [ ] Given a local field changed since baseline and the remote is unchanged, when
      sync pushes, then the remote field is set via the reverse mapping (Jira status
      via a valid transition).
- [ ] Given both sides changed a field since baseline, when sync runs, then the
      later-timestamp value wins, the loser is overwritten, and the resolution is
      reported.
- [ ] Given a value with no mapping entry, or a Linear issue matching zero or
      multiple mapped kind labels, when sync runs, then the field is left unchanged,
      the rest of the item syncs, and a warning names item, field, and value.
- [ ] Given no config overrides, when sync runs, then the built-in default tables
      are used; given a config override for a value, then it takes precedence.
- [ ] Given a field's mapping is opted out for a tracker, when sync runs, then the
      field is neither pushed nor pulled for that tracker.
- [ ] Given a Jira target status has no valid transition from the current status,
      when sync pushes, then the status is left unchanged and a warning is surfaced.
- [ ] Given the port, when an issue is read or pushed, then status/kind/priority
      cross it as structured fields.

## Open Questions

- Local `last_updated` is item-level, not per-field: a change to one field bumps the
  item stamp, so under last-writer-wins a stale unrelated field on the more-recent
  side can win. Is item-level acceptable, or is per-field change tracking needed?
- Tie-break when local and remote timestamps are equal or within clock skew.
- Priority round-trip expectation: if Jira Highest and High both collapse to `high`,
  which Jira value is written on push back?
- Opt-out granularity: per-field per-tracker is assumed; is a global per-field
  switch also wanted?

## Dependencies

- Blocked by: none. 0228 (config key model) is done and 0229 (pull scope) is ready;
  both are orthogonal.
- Blocks: none.

## Assumptions

- Accelerator's template vocabularies are canonical; trackers map onto them.
- The baseline already stores enough to detect three-way divergence once mapped
  fields join the digest.
- Jira priority and issue-type resolution is by id via the project's schemes (names
  are not guaranteed to be the defaults).

## Technical Notes

- `RemoteIssue` (`cli/tracker/src/lib.rs`) carries only `updated` + `body`; extend
  read and push draft with structured status/kind/priority.
- `fetch.rs`/`apply.rs`/`digest.rs`/`baseline.rs`: mapped fields must be projected,
  applied, digested, and baselined.
- Jira (`cli/jira-client/src/mutation.rs`) already models `issue_type` and
  `priority` on write; status needs transition discovery (GET/POST transitions).
- Linear (`cli/linear-client/src/transition.rs`) resolves state name→UUID with a
  direct `stateId` write and no transition graph; priority is integer 0–4; kind has
  no native field, so label emulation via the config-defined table.

## Drafting Notes

- Kept status/kind/priority as one story (shared mechanism: port extension +
  mapping tables + conflict policy). Split into three if independent tracking is
  wanted.
- Treated "bidirectional" as full three-way sync with last-writer-wins, per the
  user's decision.
- Priority set to medium to match epic 0146 and sibling 0229; raise if field
  mapping is the epic's critical path.
- Per-field per-tracker opt-out inferred from the user's "opt out of kind mapping as
  well"; confirm whether a global switch is also wanted.
- Linear kind via user-defined labels, per the user's decision; no built-in default
  label table, so Linear kind mapping is config-required when enabled.

## References

- Source: `meta/work/0146-work-item-sync-enhancements.md`
- Related: 0146 (parent), 0228, 0229, 0291
