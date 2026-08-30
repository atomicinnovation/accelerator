---
type: "work-item"
id: "0220"
title: "Untracked-Remote Discovery Never Runs on Linear"
date: "2026-08-22T22:38:58+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "done"
kind: "bug"
priority: "medium"
parent: "work-item:0146"
relates_to: ["work-item:0194", "work-item:0204"]
external_id: "PP-749"
tags: ["sync", "linear", "tracker", "correctness"]
last_updated: "2026-08-30T15:03:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0220: Untracked-Remote Discovery Never Runs on Linear

**Kind**: Bug
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Untracked-remote discovery is skipped entirely when the active tracker is Linear.
The discovery gate demands a configured project scope, a constraint only Jira
genuinely has; the Linear client bounds its own search to a team. The gate
short-circuits before the client is consulted, so a bidirectional sync reports
zero untracked pulls without having searched — indistinguishable in the report
from a search that ran and found nothing.

## Context

The scope is built in `cli/work-cli/src/sync.rs:434-438` from
`work.default_project_code`, with `all_projects` hardcoded `false`. The gate reads
it in `cli/work-adapters/src/sync/run.rs:707-709`:

    let discovery_enabled =
        !matches!(request.direction, SyncDirection::PushOnly)
            && (request.scope.project.is_some() || request.scope.all_projects);

With the key unset the gate is false and the `else` branch yields an empty vector
silently (`run.rs:723-725`) — no report line, no warning, no `search` call.

The gate encodes Jira's semantics. Jira's JQL (Jira Query Language) builder refuses without a project
(`E_JQL_NO_PROJECT`, `cli/jira-client/src/jql.rs:169-174`), so an unset project
there does mean unbounded and the gate is correct. Linear is the opposite: its
search falls back to the credentialed team (`cli/linear-client/src/client.rs:663-666`),
resolved from `catalogue.json`. The search was already bounded; the gate suppressed
one that was never at risk of flooding.

That credentialed-team fallback is access control, not the intended scope
authority. This fix makes the configured key the scope authority and requires it,
so an unkeyed run is a misconfiguration to report rather than a search to run
silently against whatever team the credential happens to reach — which is why the
Fix shape below both enables Linear discovery *and* hard-fails when the key is
unset.

The tracker port already carries the seam for a tracker-aware fix: the trait is at
`cli/tracker/src/lib.rs:321`, `search` at `:427`, and only Jira and Linear are
wired (`cli/work-cli/src/tracker_registry.rs`).

Observed 2026-08-22 in this repo: `work.integration: linear`, no
`work.default_project_code`, catalogue team `PP` / Product Pod. A full
bidirectional sync reported 0 pulls while unpulled remote issues existed.

## Requirements

**Reproduction**

1. Configure `work.integration: linear` with a populated catalogue and no
   `work.default_project_code`.
2. Ensure at least one issue in the team has no local work item.
3. Run a bidirectional (default, non-push-only) sync in preview:
   `accelerator work sync --preview`. The gate short-circuits on `PushOnly`, so the
   direction must be bidirectional for the defect to be exercised.

**Expected** — the untracked issue is discovered and reported as a planned pull,
with the search bounded to the keyed team.

**Actual** — the run reports zero pulls. Discovery never executed, and nothing
distinguishes that from a completed search.

**Fix shape** — throughout, *key* denotes the single value held in
`work.default_project_code` (config) and `scope.project` (runtime): a project key
on Jira, a team key on Linear. Two coupled changes:

1. Make the discovery gate tracker-aware: replace `scope.project.is_some()` with a
   port capability ("does this tracker self-bound its search?"), so Linear enables
   discovery with `scope.project` unset while Jira still refuses an unbounded
   search.
2. Bound Linear discovery to the configured key's team and require the key.
   `scope.project` holds a team *key* (e.g. `PP`); resolve it to the team UUID via
   the catalogue before it reaches the filter (`cli/linear-client/src/filter.rs:69-70`
   wants the UUID — passing the raw key matches no team, the compounding trap
   below). When the key is unset, report a config error rather than an empty
   result.

This item uses the current config field `work.default_project_code`; the rename to
`work.key` and the layered `<tracker>.<entity>_key` ownership are the sibling
configuration-model child under 0146.

**Compounding trap** — the obvious remedy fails silently. `scope.project` is passed
straight through as `team_id`, and `filter.rs:69-70` builds
`{"team": {"id": {"eq": …}}}`, which wants the team UUID. Setting
`work.default_project_code: PP` matches no team and yields zero untracked again,
for a second reason — hence the key→UUID resolution above.

## Acceptance Criteria

- [ ] Given `work.integration: linear`, a populated catalogue, a configured key,
      and at least one team issue with no local work item, when a bidirectional
      sync runs, then that issue is discovered and reported as a planned pull.
- [ ] Given the same configuration, when discovery runs, then the emitted Linear
      search filter carries the `{team: {id: {eq: …}}}` constraint for the resolved
      team UUID, and no request enumerates the wider workspace.
- [ ] Given `work.integration: linear` and no configured key, when a sync runs,
      then the run reports a config error naming the missing key, rather than
      returning zero pulls silently.
- [ ] Given a configured Linear key that is a team key (not a UUID), when discovery
      runs, then the resolved team UUID — not the raw key (e.g. `PP`) — appears in
      the search filter, and untracked issues from that team are returned.
- [ ] Given a configured Linear key that resolves to no team in the catalogue
      (absent or stale), when a sync runs, then the run reports a resolution error
      naming the unresolved key, rather than returning zero pulls silently.
- [ ] Given `work.integration: jira` with no project configured, when a sync runs,
      then discovery refuses (`E_JQL_NO_PROJECT`) and performs no unbounded search.
- [ ] Given any run where discovery does not execute, then the report states it was
      skipped and why, distinguishable from a search that completed with no results.
- [ ] A regression test covers the Linear-with-configured-key path and fails
      against the current gate.

## Dependencies

- Blocked by: none.
- Blocks: none.
- Ordering coupling (not a hard blocker): the configuration-model redesign that
  renames `work.default_project_code` → `work.key` and introduces layered
  `<tracker>.<entity>_key` ownership reads the exact field this fix reads. It is not
  yet split into its own work item — the redesign currently lives in parent epic
  0146 and must be tracked there. 0220 deliberately targets the current field so
  neither blocks the other, but whichever ships second must reconcile: if the
  rename lands first, 0220 retargets to `work.key`; if 0220 lands first, the rename
  must account for this new consumer.
- Related, no blocking relationship: 0194 and 0204 are thematically adjacent
  sync-enhancement items under 0146, not upstream prerequisites or downstream
  consumers of this fix.
- Runtime prerequisite: `catalogue.json` must carry the configured team's
  key → UUID mapping (resolved via `cli/linear-client/src/auth.rs`). A key present
  in config but absent from or stale in the catalogue resolves to no team and
  reproduces the zero-pulls symptom for a third reason; the fix must surface that
  as an error rather than an empty result.
- Test-environment prerequisite: the regression and acceptance tests run against
  mocked Linear responses, so live Linear credentials and API access are not a
  blocker for verification — they are needed only to reproduce the defect manually
  against a real workspace.

## Assumptions

- Discovery on Linear is intended behaviour, not deliberately disabled. The
  `work.default_project_code` doc comment — "Team/project-scoped by default so the
  untracked set stays bounded on a shared workspace" — reads as bounding the
  search, not suppressing it.
- The configured key is the scope authority; the credentialed team provides key →
  UUID resolution and access control, not the scope.

## Technical Notes

- Chosen mechanism: a tracker-port capability distinguishing self-bounding trackers
  (Linear) from project-requiring ones (Jira), replacing the `scope.project`-based
  gate in tracker-agnostic code. The broader scope/config redesign (per-tracker
  pull scope, `all_*`, filter schema) is 0146, not this item.
- Linear already resolves a team key → UUID via the catalogue
  (`cli/linear-client/src/auth.rs`); route the configured key through that resolver
  in the sync scope path rather than passing it raw.
- Sites: `cli/work-adapters/src/sync/run.rs:707`,
  `cli/work-cli/src/sync.rs:434`, `cli/tracker/src/lib.rs:321`,
  `cli/linear-client/src/client.rs:663`, `cli/linear-client/src/filter.rs:69`,
  `cli/linear-client/src/auth.rs`, `cli/jira-client/src/jql.rs:169`.

## Drafting Notes

- Recorded as `bug` rather than `task`: the code runs correctly and produces a
  wrong result on a configured, supported tracker.
- Reparented from 0171 (done) to 0146, the live sync-enhancements epic that now
  owns the scope-and-configuration redesign; 0220 is its first, shippable child.
- The two former open questions are resolved and moved to 0146: Linear scoping is
  keyed and required (credential is access control), and only Jira/Linear are wired
  trackers today (a future self-scoping adapter would hit the same defect).
- Fix scoped narrowly to the gate plus the Linear key→UUID resolution it depends
  on; it uses the current `default_project_code` field, which the config-model
  sibling renames.
- The silent-skip observability gap stays folded in — the silence is what kept the
  defect invisible.

## References

- Parent: 0146
- Related: 0194, 0204
