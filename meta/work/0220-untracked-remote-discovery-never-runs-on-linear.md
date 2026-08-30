---
type: "work-item"
id: "0220"
title: "Untracked-Remote Discovery Never Runs on Linear"
date: "2026-08-22T22:38:58+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
parent: "work-item:0171"
relates_to: ["work-item:0194", "work-item:0204"]
external_id: "PP-749"
tags: ["sync", "linear", "tracker", "correctness"]
last_updated: "2026-08-22T22:38:58+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0220: Untracked-Remote Discovery Never Runs on Linear

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Untracked-remote discovery is skipped entirely when the active tracker is Linear.
The gate that bounds discovery demands a configured project scope, but the Linear
client already bounds its own search to the credentialed team. The gate
short-circuits before the client is ever consulted, so a bidirectional sync
reports zero untracked pulls without having searched — indistinguishable in the
report from a search that ran and found nothing.

## Context

The scope is built in `cli/work-cli/src/sync.rs:431` from
`work.default_project_code`, with `all_projects` hardcoded to `false` and no flag
to set it. The gate then reads it in `cli/work-adapters/src/sync/run.rs:707`:

    let discovery_enabled =
        !matches!(request.direction, SyncDirection::PushOnly)
            && (request.scope.project.is_some() || request.scope.all_projects);

With the key unset the gate is false and the `else` branch yields an empty vector
silently — no report line, no warning.

The gate encodes Jira's semantics. Jira's JQL builder genuinely refuses without a
project (`E_JQL_NO_PROJECT`, `cli/jira-client/src/jql.rs:169`), so an unset
project there does mean unbounded, and the gate is correct. Linear is the
opposite: its search falls back to the credentialed team
(`cli/linear-client/src/client.rs:441`), which resolves from
`.accelerator/state/integrations/linear/catalogue.json`. The search was already
bounded; the gate suppressed one that was never at risk of flooding.

Observed 2026-08-22 in this repo: `work.integration: linear`, no
`work.default_project_code`, catalogue team `PP` / Product Pod. A full
bidirectional sync reported 0 pulls while unpulled remote issues existed.

## Requirements

**Reproduction**

1. Configure `work.integration: linear` with a populated catalogue and no
   `work.default_project_code`.
2. Ensure at least one issue in the credentialed team has no local work item.
3. Run `accelerator work sync --preview`.

**Expected** — the untracked issue is discovered and reported as a planned pull,
with the search bounded to the credentialed team.

**Actual** — the run reports zero pulls. Discovery never executed, and nothing in
the report or on stderr distinguishes that from a completed search.

**Compounding trap** — the obvious remedy does not work. `scope.project` is passed
straight through as `team_id`, and `cli/linear-client/src/filter.rs:70` builds
`{"team": {"id": {"eq": …}}}`, which wants the team UUID. Setting
`work.default_project_code: PP` matches no team and yields zero untracked again,
silently, for a second reason.

## Acceptance Criteria

- [ ] Given `work.integration: linear`, a populated catalogue, and no
      `work.default_project_code`, when a bidirectional sync runs, then untracked
      remote issues in the credentialed team are discovered and reported.
- [ ] Given the same configuration, when discovery runs, then it is bounded to the
      credentialed team and never enumerates the wider workspace.
- [ ] Given `work.integration: jira` and no `work.default_project_code`, when a
      sync runs, then discovery remains bounded — no regression to an unbounded
      search.
- [ ] Given any run where discovery does not execute, then the report states that
      it was skipped and why, distinguishable from a search that completed with no
      results.
- [ ] Given `work.default_project_code` set to a value the tracker cannot resolve
      to a scope, then the run reports that rather than returning an empty result
      set silently.
- [ ] A regression test covers the Linear-with-no-project-code path and fails
      against the current gate.

## Open Questions

- Should `work.default_project_code` accept a Linear team key (`PP`) as well as a
  UUID, or should Linear scoping be config-free and always derive from the
  catalogue?
- Does the same gate mis-fire for any other tracker whose client self-scopes?

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- Discovery on Linear is intended behaviour rather than deliberately disabled. The
  field's own doc comment — "Team/project-scoped by default so the untracked set
  stays bounded on a shared workspace" — reads as bounding the search, not
  suppressing it.
- The credentialed team is the correct default scope when no project code is
  configured.

## Technical Notes

Two candidate mechanisms, deliberately not chosen here:

1. Widen the gate to consult the tracker, so a client that bounds its own search
   enables discovery with `scope.project` unset. Addresses the root asymmetry and
   holds for future trackers.
2. Populate `scope.project` from the Linear catalogue team id before the gate
   reads it. Smaller, but leaves the Jira-shaped assumption in place and overloads
   a field named "project code" with a UUID.

Sites involved: `cli/work-cli/src/sync.rs:431`,
`cli/work-adapters/src/sync/run.rs:707`, `cli/linear-client/src/client.rs:441`,
`cli/linear-client/src/filter.rs:70`, `cli/jira-client/src/jql.rs:169`.

## Drafting Notes

- Recorded as `bug` rather than `task`: the code runs correctly and produces a
  wrong result on a configured, supported tracker.
- The fix mechanism is left open at your direction; both candidates are recorded
  in Technical Notes rather than one being written into the criteria.
- The silent-skip observability gap is folded into this item rather than split
  out — the silence is what kept the defect invisible, so fixing the gate without
  it would leave the next mis-scoped case equally undetectable.
- Parent set to `0171` as the live integrations epic. `0194` and `0204` built the
  engine and port but are `done`, so they are `relates_to` rather than parents.
- Whether this gate arrived with the fix for the earlier multi-team flood is not
  established — the investigation read current code, not history.

## References

- Related: 0171, 0194, 0204
