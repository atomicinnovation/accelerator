---
type: "work-item"
id: "0292"
title: "Linear Project Pull Filter"
date: "2026-09-20T13:29:20+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0146"
relates_to: ["work-item:0229"]
external_id: "PP-869"
tags: ["sync", "linear", "scoping", "filters", "pull"]
last_updated: "2026-09-20T13:29:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0292: Linear Project Pull Filter

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a developer syncing a Linear-backed repo, I want `project` as a pull filter
key, so that I can narrow discovery to issues in a specific Linear Project — a
dimension orthogonal to the team scope that today's `label` / `state` /
`assignee` filters cannot express.

## Context

0229 shipped a filter schema of `label` / `state` / `assignee` shared by both
trackers (`cli/work/src/pull.rs`), with per-tracker divergence confined to field
lowering. Linear models Projects as a first-class grouping that cuts across
teams; neither the team-based scope nouns (`additional_teams` / `all_teams`) nor
the three current filters can restrict a pull to one project. `project` is the
first filter key meaningful for one tracker but not the other — Jira's `project`
is already the base scope entity and its `additional_projects` / `all_projects`
nouns — so this is the trigger to split 0229's single `FilterSchema` into
per-tracker accepted sets, a divergence 0229 deliberately deferred.

## Requirements

- Add `project` to Linear's accepted pull-filter keys; leave Jira's accepted set
  unchanged (`label`, `state`, `assignee`).
- Split the shared `FilterSchema` (`cli/work/src/pull.rs`) into per-tracker
  instances so validation accepts `project` under Linear and rejects it under
  Jira with an actionable message — naming the Jira accepted set and pointing at
  `additional_projects` / the base project scope for project narrowing.
- Lower a Linear `project` filter into the `IssueFilter` as
  `project: { name: <comparator> }`, keys AND'd with the existing filters and
  values within the key OR'd (single → `eq`, multiple → `in`), matching the
  `label` / `assignee` name-comparator precedent.
- Structural validation only — no config-time remote-existence check of the
  named project (consistent with 0229; remote validation stays 0227's concern).

Out of scope:

- A Jira `project` filter (redundant with base scope + `additional_projects` /
  `all_projects`).
- Project-based scope broadening (an `additional_projects`-style noun for
  Linear); this is a filter, not a scope entity.
- Resolving project names to identifiers or a project catalogue; filter by name.
- Nested AND/OR and the raw escape hatch (separate future candidates on 0146).

## Acceptance Criteria

- [ ] Given a Linear `pull.filters` block with `project: [Alpha]`, when a pull
      runs, then the emitted `IssueFilter` constrains `project.name` `eq`
      `"Alpha"`, AND'd with any other filters.
- [ ] Given `project: [Alpha, Beta]`, when a pull runs, then the filter
      constrains `project.name` `in` `["Alpha", "Beta"]` (values OR'd).
- [ ] Given `project` under an active Jira integration, when configuration is
      validated, then it fails at `configure`, naming `project` unsupported for
      Jira and listing the Jira accepted set.
- [ ] Given `project` under an active Linear integration alongside `label` /
      `state`, when validated, then validation passes.
- [ ] Given no `project` filter, when a pull runs, then Linear discovery is
      unchanged from today.

## Open Questions

- Match by project name (proposed — no catalogue needed) or by project
  identifier (requires a project resolver / catalogue growth)? Name-based keeps
  this small but inherits name ambiguity and no typo-catching until 0227.

## Dependencies

- Blocked by: none. 0229 (the shared filter schema this splits) is implemented.
- Blocks: none.

## Assumptions

- `project` is Linear-only; Jira's accepted set stays `label` / `state` /
  `assignee`. If a Jira `project` filter is wanted instead, scope changes
  materially (it overlaps base and `additional_projects` scope).
- Lowering by `project.name` is acceptable — Linear's `ProjectFilter` exposes
  `name`; confirm `eq` / `in` on `project.name` via a client contract check
  before relying on it, per 0229's precedent.

## Technical Notes

- Accepted set: `FILTER_SCHEMA` at `cli/work/src/pull.rs:272`, validated in
  `validate` (same file, which already takes a `Tracker`, so the split keys off
  the existing argument). The comment there already anticipates a per-tracker
  split when accepted keys diverge — this is that moment.
- Linear lowering: `cli/linear-client/src/filter.rs` — `Search` gains a
  `project: Vec<String>` field and a `project` arm reusing the existing
  `comparator`. Golden fixture `cli/linear-client/tests/fixtures/issue-filter.txt`
  plus a `parse_spec` grammar extension for the new field.
- `FilterSchema` type: `cli/tracker/src/lib.rs`. `work` and `tracker` are
  public-API-pinned, so a schema-shape or `validate`-signature change regenerates
  the snapshot (`mise run public-api:update`).

## Drafting Notes

- Chose story over task — it crosses the config schema, the Linear client
  lowering, and its golden/contract tests, though it is vertically thin.
- Recommended name-based lowering and a Linear-only accepted set as the minimal
  self-contained increment; both are flagged for challenge above.
- Parented to 0146 (0229's parent epic) and related to 0229, which it extends.

## References

- Related: 0229 — Per-Tracker Pull Scope Configuration (the shared filter schema
  this splits); 0146 — parent epic
- Code: `cli/work/src/pull.rs`, `cli/linear-client/src/filter.rs`,
  `cli/tracker/src/lib.rs`
