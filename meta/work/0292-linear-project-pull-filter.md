---
type: "work-item"
id: "0292"
title: "Linear Pull Filters via Catalogue-Resolved Ids"
date: "2026-09-20T13:29:20+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "done"
kind: "story"
priority: "medium"
parent: "work-item:0146"
blocks: ["work-item:0227", "work-item:0293", "work-item:0294"]
relates_to: ["work-item:0227", "work-item:0229", "work-item:0293"]
external_id: "PP-869"
tags: ["sync", "linear", "scoping", "filters", "pull", "catalogue", "assignee"]
last_updated: "2026-09-23T22:15:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0292: Linear Pull Filters via Catalogue-Resolved Ids

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a developer syncing a Linear-backed repo, I want a `project` pull filter and
every Linear name-based filter resolved to a stable Linear id, so that discovery
narrows by project and matches `label` / `assignee` / `project` unambiguously
rather than by mutable, non-unique display names.

## Context

0229 shipped a filter schema of `label` / `state` / `assignee` shared by both
trackers (validation in `cli/tracker-support/src/pull.rs`; the `FilterSchema`
type in `cli/tracker/src/lib.rs`), with per-tracker divergence confined to field
lowering. Linear lowers `state` through the `catalogue.json` name→UUID map
(`cli/linear-client/src/catalogue.rs`), refusing an unknown state, but lowers
`label` and `assignee` by raw display name — one value to `eq` (`assignee` to
`eqIgnoreCase`), several to `in` (e.g. `labels: { name: { in } }`,
`assignee: { name: { in } }`). Projects cut across teams and cannot be
reached by the team scope nouns or the three current filters.

Research on Linear's GraphQL surface, confirmed against the live schema, shows
project, label, and user display names are non-unique and mutable (projects also
span teams); the UUID is a stable, unambiguous key, and a user additionally has
a unique email. `IssueFilter` accepts an id comparator on each — verified live:
`project: { id: { eq | in } }`, `labels: { id: { eq | in } }` (the collection's
implicit "some", no wrapper), `assignee: { id: { eq | in } }` — so every
name-based Linear filter can key on the catalogue-resolved id. This story adds
`project` and converts `label` and `assignee` to catalogue-resolved ids, leaving
every Linear filter key id-keyed in the emitted `IssueFilter` (`state` already
is; `team` scope has been id-keyed since 0220).

## Requirements

- Add `project` to Linear's accepted pull-filter keys; leave Jira's accepted set
  unchanged (`label`, `state`, `assignee`). Split the shared `FILTER_SCHEMA`
  (`cli/tracker-support/src/pull.rs`; the `FilterSchema` type in
  `cli/tracker/src/lib.rs`) per tracker so `project` is accepted under Linear and
  rejected under Jira with an actionable message. `FILTER_SCHEMA` is a private
  const, so per-tracker acceptance need not change the public `FilterSchema` type
  or `validate` signature.
- Resolve every Linear name-based filter to its Linear id through the catalogue,
  lowering to the id comparator and refusing an unresolved or ambiguous value
  (mirroring today's `UnknownState`):
  - `project` → `project: { id: { eq | in } }`, resolving the config name to a
    project id via a `CatalogueProjects` resolver.
  - `label` → `labels: { id: { eq | in } }` (the collection's implicit "some",
    no wrapper), via a `CatalogueLabels` resolver.
  - `assignee` → `assignee: { id: { eq | in } }`, resolving the config value via
    a `CatalogueUsers` resolver that tries email, then full name, then display
    name, first tier with a unique match winning and any tier with two or more
    matches refusing.
- Converge `catalogue.json` on one shape: a `baseTeam` pointer and a `teams`
  array whose entries each carry that team's `states`, `labels`, `members`
  (`id name displayName email active`) and `projects`, plus a top-level
  `labels` array of workspace labels. Only synced teams are catalogued: the
  base team and each team a sync has imported items from (0229's rule). The
  legacy `team` and `workflowStates` keys are written as projections of the
  base entry for one minor release.
- Init and apply-mode sync write whole team entries through one fetch and one
  write path. Init refreshes the base team and every synced team; an
  apply-mode sync adds entries for teams it imported from and completes
  incomplete ones. Teams in scope but not synced are fetched live for the run
  and never written. A preview never writes.
- The config surface stays human-readable (names, or a name / email for
  `assignee`); ids are an internal resolution detail. Values resolve against
  the teams in scope. A value no team in scope carries refuses the pull,
  naming the value and directing the operator to refresh the catalogue with
  `init-linear` if it was added in Linear.
- Structural validation only — no config-time remote-existence check of named
  entities (consistent with 0229; remote validation stays 0227's concern).

Out of scope:

- A Jira `project` filter (redundant with base scope + `additional_projects` /
  `all_projects`) and any change to Jira filter lowering.
- Negated filters — 0293 layers polarity on the id-keyed forms this story lands.
- Nested AND/OR and the raw escape hatch (separate future candidates on 0146).
- Config-time name→id resolution or an on-demand catalogue refresh; resolution
  reads the persisted snapshot a pull last wrote.

## Acceptance Criteria

- [ ] Given `project: [Alpha]` with Alpha in the catalogue, when a pull runs,
      then the `IssueFilter` constrains `project: { id: { eq: <alpha-uuid> } }`.
- [ ] Given `project: [Alpha, Beta]`, when a pull runs, then it constrains
      `project: { id: { in: [<alpha-uuid>, <beta-uuid>] } }` (values OR'd).
- [ ] Given `label: [bug, urgent]`, when a pull runs, then labels lower to
      `labels: { id: { in: [<bug-uuid>, <urgent-uuid>] } }` — the implicit
      "some", on catalogue-resolved ids, not names.
- [ ] Given `assignee: [ada@example.com]` matching a catalogue email, when a
      pull runs, then it constrains `assignee: { id: { eq: <ada-uuid> } }`,
      resolved at the email tier.
- [ ] Given `assignee: [Ada Lovelace]` matching exactly one catalogue full name
      and no email, when a pull runs, then it resolves at the name tier to that
      user's id.
- [ ] Given `assignee: [ada]` matching exactly one catalogue display name and no
      email or full name, when a pull runs, then it resolves at the display-name
      tier to that user's id.
- [ ] Given `assignee: [Ada Lovelace]` where that string is one user's full name
      and a different user's display name, when a pull runs, then it resolves to
      the full-name match's id (earlier tier wins), making tier precedence
      observable through the distinct resolved id.
- [ ] Given an `assignee` value matching two or more users within a tier, when a
      pull runs, then it aborts with a non-zero exit and fetches no issues,
      naming the collision, rather than falling through to a later tier or
      picking one.
- [ ] Given a filter naming an entity that no team in scope carries, or that
      is ambiguous among them, when a pull runs, then it aborts with a
      non-zero exit and fetches no issues — naming the entity and directing
      the operator to refresh the catalogue with `init-linear` — not silently
      matching none or several.
- [ ] Given a team in scope whose catalogue entry is missing or incomplete,
      when a pull runs, then that team's needed sections are fetched live and
      its ids are used; the fetched entry is written to `catalogue.json` only
      by an apply-mode sync, and only when the team is synced.
- [ ] Given `project` under an active Linear integration, when configuration is
      validated, then it is accepted (no `UnsupportedFilterKey` error).
- [ ] Given `project` under an active Jira integration, when configuration is
      validated, then it fails at `configure`, naming `project` unsupported for
      Jira and listing the Jira accepted set.
- [ ] Given a `project` filter accepted at config validation, when a pull runs,
      then the Linear client applies it to the `IssueFilter` rather than silently
      dropping an unrecognised field.
- [ ] Given an init discovery, when it completes, then `catalogue.json` holds
      a `baseTeam` and a complete entry (`states`, `labels`, `members` with
      `displayName` and `email`, `projects`) for the base team and every synced
      team, plus workspace `labels` — and no entry for any other visible team.
- [ ] Given a team whose states, labels, members, or projects span more than
      one page, when its entry is fetched, then the entry holds an entity drawn
      from a later page and a filter naming it resolves.
- [ ] Given a catalogue written before this story, when an apply-mode sync
      completes, then every synced team's entry is complete, without a manual
      `init-linear`.
- [ ] Given no `project` / `label` / `assignee` filter, when a pull runs, then
      the constructed `IssueFilter` carries no project / label / assignee
      constraint and matches the no-filter `issue-filter.txt` golden fixture.
- [ ] Given a positive `project` filter, when a pull runs, then the constructed
      `IssueFilter` applies `project: { id }` as a positive constraint (not a
      negation) — the shape under which Linear excludes project-less issues.
- [ ] Given a `project` value naming a non-existent project, when configuration
      is validated, then validation passes with no remote call; the value is
      refused only at pull time.

## Dependencies

- Blocked by: none. 0229 (the filter mechanism) and the catalogue infrastructure
  (0048, 0220) are implemented.
- External systems: the Linear GraphQL API — the added paginated `projects` /
  `issueLabels` / `users` fetches raise per-refresh query complexity (Linear
  meters by complexity), and its availability and throughput gate every pull.
- Blocks: 0293 (Negated Pull Filters) — its `project` / `label` / `assignee`
  negation lowers over the id-keyed forms this story establishes.
- Consumed by: 0227 (accelerator config validate) — its `config validate`
  command must accept `project` under Linear and reject it under Jira in step
  with the per-tracker accepted-set split this story introduces.

## Assumptions

- Project and label display names are non-unique and mutable, so their
  resolution refuses on ambiguity (like `state`) rather than over-matching, and
  a rename between catalogue refreshes surfaces as an unresolved-name refusal.
- `assignee` accepts email, full name, or display name, resolved in that order
  with the first uniquely-matching tier winning. Email is Linear's unique user
  identifier and never resolves ambiguously; the name and display-name tiers
  can, and a tier with two or more matches refuses rather than falling through.
  Verified against the live tenant: a personal API key reads `email` for every
  workspace member (16 of 16), so the email index is buildable.
- Matching moves from today's live `eqIgnoreCase` name comparison to
  catalogue-resolved ids, so a `label` or `assignee` value absent from the
  catalogue refuses until discovery repopulates it. Accepting name and display
  name (not email alone) keeps existing name-based configs working once the
  catalogue is repopulated, so this is not a config-syntax break.
- The config surface stays human-readable; operators never write UUIDs.
- Cold-start / migration: an existing catalogue reads as incomplete entries.
  Filtered pulls fetch the missing sections live, and the first apply-mode
  sync completes every synced team's entry, so no manual re-init is needed.

## Technical Notes

- Accepted set / split: `FILTER_SCHEMA` (a private const) and
  `validate(&PullConfig, Tracker)` live in `cli/tracker-support/src/pull.rs`; the
  `FilterSchema` type is in `cli/tracker/src/lib.rs`. The `Tracker` argument
  (`tracker_support::pull::Tracker`, `Jira` / `Linear`) already keys the split,
  today only for scope nouns. Because `FILTER_SCHEMA` is private, per-tracker
  accepted keys need not touch a public-API snapshot; only a change to the
  `FilterSchema` type, the `validate` signature, or the
  `PullConfigError::UnsupportedFilterKey` variant crosses the boundary, and that
  is pinned in `cli/tracker` and `cli/tracker-support` — not `cli/work`, whose
  snapshot has no pull entries. Regenerate with `mise run public-api:update` only
  if a public signature actually changes.
- Linear lowering: `cli/linear-client/src/filter.rs` `compose` gains project,
  label, and user resolvers beside `StateResolver`; the `project` arm and the id
  forms for `label` / `assignee` replace the name comparators, following the
  existing `eq` (one value) / `in` (several) cardinality. Add a `project` field
  to the `Search` struct and a matching arm to the client's filter intake in
  `cli/linear-client/src/client.rs` (`search`), whose `_ => {}` default would
  otherwise silently drop a validated `project` filter. Extend the golden fixture
  `issue-filter.txt` and the `parse_spec` grammar; the fixture (hand-written, no
  schema validator exists) is the pin, as it is for `state` / `label` /
  `assignee`. Verified live: `project: { id }`, `labels: { id }` (implicit
  "some"), and `assignee: { id }` are all accepted; `project.id` uses
  `EntityIdentifierIDComparator`, which rejects the nil placeholder UUID that the
  plain `IDComparator` on `label` / `assignee` accepts, so project fixtures need
  real-shaped ids.
- Catalogue: `cli/linear-client/src/catalogue.rs` gains `CatalogueProjects`,
  `CatalogueLabels`, and `CatalogueUsers`. Projects and labels index name;
  `CatalogueUsers` indexes email, full name, and display name (a generic index is
  a candidate refactor). Sections are fetched per team, for synced teams only;
  whether members and projects can be fetched for many teams in one paginated
  pass is settled by a spike before planning completes. Init and apply-mode
  sync share one write path.
- Rate limiting: the added catalogue fetches (and their pagination) raise
  per-refresh query complexity (Linear meters by complexity); the filter itself
  costs the same on name or id.

## Drafting Notes

- Chose catalogue-resolved ids over name passthrough for `project`, `label`, and
  `assignee` — the user's direction and Linear's stability steer. Converting
  `label` and `assignee`, not just adding `project`, leaves every Linear filter
  key id-keyed in the emitted `IssueFilter`, alongside `state` (and the
  already-id-keyed `team` scope).
- Bundling the additive `project` filter with the migration-bearing `label` /
  `assignee` id conversion is deliberate: both share the catalogue-resolution
  mechanism, and shipping them together holds the catalogue re-init to a single
  operator migration event rather than two.
- `assignee` resolves email, then full name, then display name to a user id,
  first uniquely-matching tier winning — chosen (2026-09-20 stress test) over an
  email-only key so existing name-based configs keep working, while email stays
  available as the collision-proof option. A tier with multiple matches refuses.
- Retitled from "Linear Project Pull Filter" and raised priority low→medium: the
  item now adds catalogue infrastructure and changes shipped `label` /
  `assignee` behaviour. The filename is unchanged (enrich-mode does not rename).
- Follow-up: parent 0146's Stories entry still describes 0292 as `project.name`
  lowering — correct it to the id-keyed `project` / `label` / `assignee` scope
  once this ships.
- Stress-tested 2026-09-20 against the live Linear tenant: corrected the code
  paths (`cli/tracker-support/src/pull.rs`, not `cli/work/src/pull.rs`) and the
  public-API claim; confirmed `project` / `label` / `assignee` id forms and
  member-email readability; dropped the `labels … some` wrapper and the
  non-existent "client-contract check"; and fixed the catalogue-refresh remedy
  (init, not a plain pull). Parent 0146's Stories entry still describes 0292 as
  `project.name` lowering and is now stale.
- Plan stress test 2026-09-23: replaced the workspace-wide catalogue with
  per-team entries for synced teams only, keeping 0229's rule that the
  committed catalogue never holds the whole visible workspace. Members include
  disabled users, with active users winning within a tier. Incomplete entries
  are fetched live and healed by an apply-mode sync instead of a forced
  re-init. The legacy `team` / `workflowStates` keys survive one minor
  release as projections.

## References

- Related: 0227 — accelerator config validate Command (must extend to the new
  `project` key and the per-tracker accepted-set split); 0229 — Per-Tracker Pull
  Scope Configuration; 0293 — Negated Pull Filters (negates the id-keyed forms);
  0146 — parent epic; 0048 / 0220 — the catalogue name→UUID prior art.
- Code: `cli/tracker-support/src/pull.rs`, `cli/tracker/src/lib.rs`,
  `cli/linear-client/src/filter.rs`, `cli/linear-client/src/catalogue.rs`,
  `cli/linear-client/src/client.rs`, `cli/linear-client/src/discovery.rs`
- Linear filtering guide: https://linear.app/developers/filtering
