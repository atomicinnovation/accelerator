---
type: "work-item"
id: "0229"
title: "Per-Tracker Pull Scope Configuration"
date: "2026-08-30T14:35:09+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0146"
tags: ["sync", "scoping", "tracker", "configuration"]
last_updated: "2026-08-30T14:35:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-759"
---
# 0229: Per-Tracker Pull Scope Configuration

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a developer syncing work items, I want to control how broadly a pull discovers
remote issues per tracker, so that discovery is bounded by an explicit, validated
scope rather than a hard-coded assumption. Introduce a per-tracker `pull` block:
`additional_teams` / `additional_projects`, `all_teams` / `all_projects`, and a
generic `filters` bag, each validated at config time. This realises 0146's existing
"restrict sync by label/project" requirement.

## Context

The creation entity (from the configured key) is always the implicit base scope; the
`pull` block only broadens it. `all_teams` / `all_projects` searches the whole
accessible workspace — Jira omits the JQL `project =` clause, Linear drops the team
filter and suppresses the credentialed-team fallback — bounded only by `max_pulls`.
The port already models arbitrary filters (`SearchScope.filters` in
`cli/tracker/src/lib.rs`); this promotes that seam to the primary mechanism.

## Requirements

- Per-tracker `pull` block with `additional_*` entities, `all_*` flag, and a
  `filters` bag.
- The creation entity (keyed project/team) is always included in scope implicitly.
- `all_*` is mutually exclusive with `additional_*`; specifying both is a config
  error.
- Each tracker declares a filter schema (accepted keys, required keys) so an
  invalid or missing required filter fails at config time, not at sync time.
- The port carries one entity-neutral "whole workspace" boolean; adapters interpret
  it; the config surface uses each tracker's vocabulary.

## Acceptance Criteria

- [ ] Given `additional_*` or `filters` are configured, when a pull runs, then
      discovery is broadened accordingly, still including the keyed entity.
- [ ] Given `all_teams` / `all_projects` is set, when a pull runs, then discovery
      spans the whole accessible workspace, bounded by `max_pulls`.
- [ ] Given both `all_*` and `additional_*` are set, then configuration validation
      fails.
- [ ] Given a required filter key is missing or an unsupported key is present, then
      the failure is reported at `configure`, not at `sync`.
- [ ] Given no `pull` configuration, when a pull runs, then discovery stays bounded
      to the keyed entity.

## Dependencies

- Blocked by: the layered configuration key model (sibling under 0146).
- Blocks: none.

## Technical Notes

- Jira's `project_key` satisfies the JQL project requirement, so `all_projects` is
  pure opt-in breadth rather than a precondition.
- Depends on the key model landing first so the base scope resolves from the
  canonical key.

## Drafting Notes

- Filters modelled as a bag plus a per-tracker schema rather than fully free-form,
  so tracker-specific required keys (Jira project) have a place to fail early.

## References

- Parent: 0146
