---
type: "work-item"
id: "0229"
title: "Per-Tracker Pull Scope Configuration"
date: "2026-08-30T14:35:09+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "ready"
kind: "story"
priority: "medium"
parent: "work-item:0146"
blocked_by: ["work-item:0228"]
relates_to: ["work-item:0227", "work-item:0220"]
tags: ["sync", "scoping", "tracker", "configuration", "discovery"]
external_id: "PP-759"
last_updated: "2026-09-09T22:33:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0229: Per-Tracker Pull Scope Configuration

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a developer syncing work items, I want to control how broadly a pull discovers
remote issues per tracker, so that discovery is bounded by an explicit, validated
scope rather than a hard-coded single-entity assumption. Introduce a per-tracker
`pull` block: `additional_teams` / `additional_projects`, `all_teams` /
`all_projects`, a normalised `filters` bag, and configurable `max_items` /
`max_pages` ceilings — each structurally validated at config time (remote-existence
of named entities is checked separately, at sync time or via 0227). Broadening discovery across
multiple scopes also requires deduplicating overlapping results and reconciling them
in a total, deterministic order (by identifier prefix, then numeric sequence). This realises 0146's "restrict sync by label/project" requirement,
which does not ship today in any active form.

## Context

The keyed base entity — the creation home resolved from 0228's canonical key — is
always the implicit base scope; the `pull` block only broadens it. Today the base scope is `work.default_project_code`, which scopes
discovery for *both* trackers — Jira turns it into a `project = <code>` Jira Query
Language (JQL) clause, Linear reinterprets the same value as a team key resolving to a
single team. 0228
detaches base-scope resolution from that tracker-specific key; this story layers the
`pull` block on top of the canonical key it introduces.

`all_teams` / `all_projects` searches the whole accessible workspace — literally
everything the configured credential can see, with no implicit active-only filter;
Jira omits the JQL `project =` clause, Linear drops the team filter and suppresses
the credentialed-team fallback (Linear's default of scoping an unfiltered search to
the credential's own team, introduced in 0220) — bounded only by `max_items` /
`max_pages`.

The port already models arbitrary filters (`SearchScope.filters` in
`cli/tracker/src/lib.rs`), and both clients already consume them, but production
always constructs the list empty — a wired-but-dormant seam. This story promotes
that seam to the primary mechanism and gives it a validated config surface.

## Requirements

**Config surface**

- Per-tracker `pull` block with `additional_teams` / `additional_projects`, an
  `all_teams` / `all_projects` flag, a `filters` bag, and `max_items` / `max_pages`
  ceilings. The config surface uses each tracker's vocabulary; the port stays
  entity-neutral (one "whole workspace" boolean the adapters interpret).
- The config catalogue registers scalar defaults today; extending it to carry
  structured (map / array) values for the `filters` bag is in scope for this story.
- The keyed base entity is always included in scope implicitly (from 0228's key
  model).
- The `pull` block may appear in both team and personal config. A personal block
  **wholly replaces** the team block — no field-level merge.

**Filters**

- Filters are normalised and flat: keys are AND'd; multiple values within a key are
  OR'd (an `IN` clause). No raw pass-through query strings. For example, filters
  `{label: [a, b], state: [open]}` lower to `state = open AND label IN (a, b)`.
- Each tracker declares a filter schema of accepted keys (indicatively `label`,
  `state`, `assignee` for both trackers; the concrete per-tracker set is fixed during
  implementation) so an unsupported filter key fails at `configure`. The schema also
  supports declaring required keys, but no tracker declares one today (see
  Assumptions), so only the unsupported-key branch is exercised. Validation is
  structural only.
- Remote-existence of named filter entities is delegated to 0227's `accelerator config
  validate`; this story adds no proactive config-time remote check. 0227's command
  must in turn learn to validate the `pull` block surface this story introduces.
- `all` / `any` are reserved keys that cannot be filter field names. Their presence
  fails validation with a "nested filters not yet supported" error, reserving them
  for a future story that adds full AND/OR nesting with no config migration.

**Scope resolution and ceilings**

- `all_*` is mutually exclusive with `additional_*`; specifying both is a config
  error, evaluated on the effective (post-override) block.
- `max_items` bounds the count of discovered issues destined for reconciliation; it
  is the per-tracker config default for the existing `max_pulls` reconcile ceiling.
  `max_pages` is the per-tracker config default for the transport pagination cap
  (today fixed at 20).
- Both `max_items` and `max_pages` accept the literal `unlimited` sentinel, meaning
  the ceiling does not bound discovery. No other value denotes unrestricted.
- Crossing `max_items` (the discovered-issue count exceeds the ceiling) errors and
  reconciles nothing (all-or-nothing).
- Reaching the `max_pages` cap (an incomplete discovery result) errors rather than
  truncating silently.
- A named `additional_*` or base entity that cannot be resolved on the remote at sync
  time errors and aborts the pull. This sync-time resolution is distinct from 0227's
  proactive config-time validation — the two checks fire at different times.

**Result handling**

- The discovered set is deduplicated by remote work-item identifier: one issue
  reached via several scopes appears once. Two distinct remote issues bearing
  different identifiers are both retained even when they represent the same
  underlying work.
- Discovered issues are reconciled in a total, deterministic order: ascending by the
  identifier's prefix (lexical), then ascending by its numeric sequence component. So
  `PP-2` precedes `PP-10` (numeric, not lexical, within a prefix), and all `PP-*`
  precede all `XX-*`. This fully orders a multi-scope set spanning several prefixes.

**Boundaries**

- Jira and Linear only; GitHub and other trackers are out of scope. GitHub is deferred
  to 0050 (GitHub Issues and Projects Integration), which sits under the 0181
  additional-integrations epic.
- Strictly pull-side discovery; push is untouched.

## Acceptance Criteria

- [ ] Given `additional_*` is configured, when a pull runs, then the emitted
      per-tracker search targets the keyed base entity and each configured
      `additional_*` entity.
- [ ] Given filters `{label: [a, b], state: [open]}`, when a pull runs, then the
      emitted per-tracker query applies keys AND'd and values within a key OR'd: on
      Jira the JQL constrains `state = open AND label IN (a, b)`; on Linear the filter
      object constrains `state` to `open` AND `label` to any of `a`, `b`.
- [ ] Given `all_teams` / `all_projects` is set, when a pull runs, then the emitted
      search carries no project/team scope constraint (Jira omits the `project =`
      clause; Linear drops the team filter and the credentialed-team fallback),
      bounded only by `max_items` / `max_pages`.
- [ ] Given both `all_*` and `additional_*` are set in the effective block, when
      configuration is validated, then validation fails.
- [ ] Given an unsupported filter key is present, then the failure is reported at
      `configure`, not at `sync`.
- [ ] Given a reserved grouping key (`all` / `any`) appears in `filters`, when
      configuration is validated, then it fails with "nested filters not yet
      supported".
- [ ] Given `max_items` or `max_pages` is set to a value that is neither a
      non-negative integer nor `unlimited`, when configuration is validated, then
      validation fails at `configure`.
- [ ] Given team config `additional_teams: [X]` with `filters: {label: [a]}` and
      personal config `additional_teams: [Y]` with no `filters`, when scope resolves,
      then the effective block is the personal one alone: discovery is broadened by `Y`,
      and both `X` and the team's `filters` are dropped (whole-block replacement, not a
      field-level merge that would retain the team `filters`).
- [ ] Given discovery crosses `max_items`, when a pull runs, then the pull errors and
      reconciles nothing.
- [ ] Given discovery reaches the `max_pages` cap, when a pull runs, then the pull
      errors rather than truncating silently.
- [ ] Given `max_items` is configured to `3` (distinct from the built-in default 25),
      when a pull discovers 5 issues, then the pull errors on crossing the configured
      ceiling (all-or-nothing, per above) — whereas the default 25 would have
      completed — proving the configured value, not the default, is in effect.
- [ ] Given `max_pages` is configured to `5` (distinct from the built-in default 20),
      when discovery spans 6 pages, then the pull errors on reaching the configured cap
      rather than truncating (per above) — whereas the default 20 would have
      completed — proving the configured value, not the default, is in effect.
- [ ] Given a discovery set larger than both default ceilings, when `max_items` and
      `max_pages` are both set to `unlimited`, then the pull completes and reconciles
      the full set without a ceiling error.
- [ ] Given a named base or `additional_*` entity cannot be resolved on the remote,
      when a pull runs, then the pull errors and aborts.
- [ ] Given the same remote issue is discovered via multiple scopes, when results
      merge, then it appears once (dedup by remote work-item identifier); two distinct
      issues under different identifiers are both retained.
- [ ] Given discovery yields issues `XX-3`, `PP-10`, `PP-2`, when they are reconciled,
      then they are processed as `PP-2`, `PP-10`, `XX-3` (ascending by prefix, then by
      numeric sequence component within a prefix, not lexical string order).
- [ ] Given no `pull` configuration, when a pull runs, then discovery stays bounded to
      the keyed base entity.

## Open Questions

- None outstanding. Two prior questions are now resolved: structured (map / array)
  `pull.filters` values require extending the config catalogue, and that work is in
  scope for this story (see Requirements: Config surface); no tracker declares a
  required filter key, so all filter keys are optional (see Assumptions).

## Dependencies

- Blocked by: 0228 (Layered Configuration Key Model) — the base scope resolves from
  the canonical key this story layers on.
- Relates to: 0227 (accelerator config validate Command) — bidirectional. This story
  delegates remote-existence validation of named entities to 0227's command; in turn
  0227's command must learn to validate the new `pull`-block config surface this story
  introduces. 0227 is not a hard blocker: 0229's structural config-time validation
  stands alone, and remote-existence checks also fire at sync time.
- Blocks: the pull-block validation portion of 0227 — 0227's `config validate` command
  cannot validate the `pull`-block surface until this story defines it. This gates only
  that portion of 0227, not the whole command; cross-referenced from 0227's
  Dependencies.
- External systems: sync-time entity resolution and multi-scope discovery depend on the
  Jira and Linear remote APIs being reachable and behaving as specified. This coupling
  is inherited from the sync engine, not new to this story.

## Assumptions

- All filter keys are optional (no required keys), since the base keyed entity already
  bounds discovery. The per-tracker schema still provides a home for required-key
  validation should a tracker ever need one.

## Technical Notes

- `SearchScope.filters: Vec<(String, String)>` (`cli/tracker/src/lib.rs:268`) is the
  dormant seam; production builds it empty (`cli/work-cli/src/sync.rs:859`), and both
  clients already lower it (Jira → `field IN (...)`, `cli/jira-client/src/jql.rs`;
  Linear → known keys, `cli/linear-client/src/filter.rs`). The flat leaf model maps
  1:1 onto this type; nesting later needs a recursive type change.
- Two distinct ceilings exist today: the global `max_pulls` / `max_pushes` reconcile
  refusal (`cli/work-adapters/src/sync/run.rs:882`, CLI default 25) and the
  fixed transport `max_pages` cap of 20 (`cli/tracker-support/src/transport.rs`).
  `pull.max_items` supplies a per-tracker config default for the former;
  `pull.max_pages` for the latter.
- Linear entity resolution is catalogue-backed, not a remote call: a configured
  `additional_teams` key resolves to a UUID via `catalogue.json` (0220 prior art), so a
  key missing from the catalogue is a local mapping gap, not a remote-absence failure —
  the "cannot be resolved at sync time" abort covers both, but the Linear failure mode
  is catalogue population, which must cover every configured additional team.
- Incomplete discovery today yields `Discovery { complete: false }` (silent
  truncation); this story promotes it to a hard error.
- The port stays entity-neutral (`all_projects` boolean); adapters interpret it
  against their one scope-entity kind.

## Drafting Notes

- Kept per-tracker `project` / `team` config vocabulary; a generic
  `additional_scopes` / `all_scopes` was considered and rejected — the per-tracker
  noun is self-documenting given the single-tracker reality, and the port is already
  entity-neutral underneath.
- `filters` is flat now but forward-compatible: `all` / `any` are reserved as
  grouping keys so full AND/OR nesting can be added later with no config migration and
  no change to existing filter semantics.
- Standardised the ceiling config keys as `max_items` / `max_pages`, mapping onto the
  existing internal `max_pulls` and transport page cap.
- The configurable ceilings and the truncation-to-hard-error change ship in this
  increment deliberately, not as a follow-on: broadening discovery without bounded,
  fail-loud ceilings risks an unbounded flood or a silently truncated pull, so the two
  are treated as one deliverable.
- The story deliberately keeps scope broadening, the `filters` bag (with its
  config-catalogue structured-value extension and per-tracker schema), the ceilings,
  and result dedup/ordering as one increment rather than splitting the filters
  mechanism out. All four share the `pull` block and the single `SearchScope.filters`
  seam, and every piece exists to make one capability — bounded, validated pull
  discovery — usable and safe; a filters-only or broadening-only partial landing would
  ship an unbounded or unfilterable half-feature. This enlarges the unit beyond 0146's
  original child sketch (broadened to match in 0146).
- Chose `unlimited` as the unrestricted sentinel over `0`, which reads ambiguously as
  "pull nothing".
- Scoped strictly to pull-side discovery; push named as an explicit non-goal.
- GitHub deferred to 0050 (the GitHub Issues story under the 0181
  additional-integrations epic), reconciling 0146, which names the umbrella 0181.

## References

- Parent: 0146
- Blocked by: 0228 — Layered Configuration Key Model
- Related: 0227 — accelerator config validate Command; 0220 — Untracked-Remote
  Discovery Never Runs on Linear (prior art on Linear discovery scope)
- Code: `cli/tracker/src/lib.rs`, `cli/work-cli/src/sync.rs`,
  `cli/jira-client/src/jql.rs`, `cli/linear-client/src/filter.rs`,
  `cli/work-adapters/src/sync/run.rs`, `cli/tracker-support/src/transport.rs`
