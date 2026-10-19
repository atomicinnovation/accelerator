---
type: "work-item"
id: "0293"
title: "Negated Pull Filters"
date: "2026-09-20T17:55:13+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0146"
blocked_by: ["work-item:0292"]
relates_to: ["work-item:0229"]
external_id: "PP-870"
tags: ["sync", "linear", "jira", "scoping", "filters", "pull"]
last_updated: "2026-09-20T21:20:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0293: Negated Pull Filters

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a developer configuring a pull, I want negated filter values (e.g. `state
not in (done, cancelled)`), so that I can exclude issues by field value rather
than only include them — an "everything except" scope the current include-only
`filters` bag cannot express.

## Context

0229 shipped an include-only `filters` bag — keys AND'd, values within a key
OR'd into an `IN` / `eq`. Negation is asymmetrically supported beneath it. Jira's
JQL composer already splits a value family into `field IN (...)` (positive) and
`field NOT IN (...)` (negated, via a leading `~` on the value) at
`cli/jira-client/src/jql.rs:262-298` — a dormant seam, as `filters` itself was
before 0229. Linear's lowering (`cli/linear-client/src/filter.rs`) emits only
positive `eq` / `in` with no negation, and — building on 0292 — resolves
`project` / `label` / `assignee` to catalogue ids, so this story's negations
lower over those ids, not display names. Neither the config surface nor the port's
`filters: Vec<(String, String)>` (`cli/tracker/src/lib.rs:326`) exposes polarity
as anything but that ad-hoc `~`-prefix string. This story promotes negation to a
first-class, validated config surface across both trackers and pins its
semantics on optional and to-many fields, where `NOT IN` diverges per tracker.

Web research (2026) confirms Linear's `IssueFilter` expresses every negation
this story needs server-side: `neq` / `nin` on id and string comparators, an
`every` quantifier on the `labels` collection (there is no `none`), and `and` /
`or` operators — but no top-level `not`. Set-membership negation is therefore
fully server-side on both trackers; the client-side fetch-then-filter fallback
the request anticipated is unnecessary within this scope.

## Requirements

- Config surface: extend `filters` so a key can carry negated values alongside
  or instead of positive ones. Proposed shape — a per-key operator object
  `{ in: [...], not_in: [...] }`, with a bare list `key: [a, b]` remaining sugar
  for `{ in: [a, b] }`. A key may carry both `in` and `not_in`.
- Port: carry polarity explicitly on the filter-value seam, superseding the
  stringly-typed `~`-prefix convention — `SearchScope.filters` values gain a
  polarity and `PullConfig.filters` carries positive/negative sets per key. This
  is a public-API-pinned `tracker` change.
- Jira lowering: feed the existing `family_clauses` split from the explicit
  polarity (not the `~` string), grouping same-key positive and negative values
  into one `IN` and one `NOT IN`, AND'd.
- Linear lowering (all server-side): add negation comparators and pin the
  relation forms —
  - `state`: names resolved to UUIDs via the catalogue, negated as
    `state: { id: { nin: [...] } }` (single value `neq`). State is always set,
    so there is no unset case.
  - `assignee` (email→id via the catalogue, per 0292):
    `assignee: { id: { nin: [...] } }` / `neq`.
  - `project` (name→id via the catalogue, per 0292):
    `project: { id: { nin: [...] } }` / `neq`.
  - `label` (to-many, name→id via the catalogue, per 0292):
    `labels: { every: { id: { nin: [...] } } }` — not a bare `nin` (which means
    "has some label not in the set") and not a `none` quantifier (Linear has
    none); `every` gives "has no label in the set" and keeps label-less issues.
  - No top-level `not` on `IssueFilter`: each negation is pushed to a comparator
    or the `every` / `null` relation construct. Arbitrary predicate negation is
    not server-expressible and is out of scope (see below).
- Empty/optional-field semantics: decide, per field, whether a negated filter
  keeps issues where the field is unset. Both trackers exclude unset rows under
  negation by default and both offer a re-include construct: Jira
  `OR <field> IS EMPTY` (via the existing `Search.empty` seam); Linear an
  `{ or: [ {neg}, { null: true } ] }` branch for the nullable relations
  (`assignee`, `project`). `labels: { every: ... }` already keeps label-less
  issues and `state` has no unset case. The chosen rule must be identical on
  both trackers.
- Validation: accepted operators are `in` / `not_in`; reject any other operator
  key at `configure`, naming the accepted set. The accepted-filter-*key* schema
  (0229's, or 0292's per-tracker split) is unchanged — polarity is orthogonal to
  which keys are accepted.

Out of scope:

- Nested AND/OR (`all` / `any`) — the separate future candidate on 0146.
- A raw JQL / `IssueFilter` escape hatch — separate future candidate.
- New filter keys — 0292 owns Linear `project`; this story only adds polarity to
  existing keys.
- Range / comparison operators (`>`, `<`, date windows) — set-membership
  negation only.
- Arbitrary / uniform predicate negation (a top-level `not`). Linear has no
  `not: IssueFilter`, so only set-membership negation (`not_in` / not-equal,
  plus the label `every` form) is in scope; anything requiring client-side
  fetch-then-filter is excluded.

## Acceptance Criteria

- [ ] Given `filters: { state: { not_in: [done, cancelled] } }` on Jira, when a
      pull runs, then the JQL constrains `status NOT IN ('done', 'cancelled')`.
- [ ] Given the same on Linear, when a pull runs, then the `IssueFilter`
      constrains `state: { id: { nin: [<done-uuid>, <cancelled-uuid>] } }` (names
      resolved via the catalogue).
- [ ] Given `filters: { label: { in: [a], not_in: [b] } }`, when a pull runs,
      then both are applied and AND'd — Jira `labels IN ('a') AND labels NOT IN
      ('b')`; Linear the `some` / `every` equivalents.
- [ ] Given a negated `label` filter on Linear, when a pull runs, then issues
      bearing that label are excluded via the `every` relation form, not a bare
      `nin`.
- [ ] Given the chosen empty-field policy, when a negated filter runs against
      issues with the field unset, then those issues are included/excluded
      consistently on both trackers (per the Open Question resolution).
- [ ] Given an unknown operator key (e.g. `state: { gt: 5 }`), when
      configuration is validated, then it fails at `configure`, naming the
      accepted operators (`in`, `not_in`).
- [ ] Given a bare list `state: [open]`, when a pull runs, then it behaves
      exactly as today (`in` / `eq`) — backward compatibility.

## Open Questions

- Config syntax: the per-key operator object `{ in, not_in }` (proposed), a
  sibling `exclude:` bag, or a value prefix (`!done`)? The operator object
  composes best with the deferred nested-filter model and with 0292's per-key
  comparator shape.
- Empty-field default: keep unset rows by default under negation (auto-add the
  `IS EMPTY` / `null: true` branch) or exclude them (raw `NOT IN` / `nin`)?
  Recommendation and Assumption below: keep them — "state not in (done)" should
  still surface an issue with no state — applied identically on both trackers.

## Dependencies

- Blocked by: 0292 (Linear Pull Filters via Catalogue-Resolved Ids) —
  establishes the id-keyed `project` / `label` / `assignee` lowering this story
  negates.
- Relates to: 0229 (the filter mechanism, implemented) and the "Nested AND/OR
  filters" future candidate on 0146.

## Assumptions

- Negation is set-membership only (`not_in` / not-equal), not range or
  comparison operators.
- The `~`-prefix value convention in Jira's `Family` is an internal wire detail
  to be superseded by an explicit polarity, not a public config surface to
  document.
- A negated filter keeps issues where the field is unset (the auto-`IS EMPTY` /
  `null: true` branch is emitted). This is a scope-changing default — confirm or
  flip it. If wrong, negations silently drop unassigned / stateless issues.
- Linear's exclusion of relation-less rows under bare negation is confirmed by
  research but not by Linear's own prose; verify with a client-contract check
  against the live schema before relying on the `null`-branch behaviour.
- A negated `project` / `label` / `assignee` filter resolves its values through
  the catalogue (0292) before negating — `assignee` by email, `project` /
  `label` by name — so a value the catalogue cannot resolve refuses the pull
  rather than silently negating nothing.

## Technical Notes

- Jira seam (largely present): `family_clauses` at
  `cli/jira-client/src/jql.rs:262-298` already splits positives / negatives
  (leading `~`) into `IN` / `NOT IN`; `Family.values: Vec<String>` (jql.rs:167)
  carries it. Work: feed it from explicit polarity, group same-key values, and
  wire the `IS EMPTY` policy through the existing `Search.empty` / `not_empty`
  seam (jql.rs:184-185, 219-230).
- Linear: `comparator` (`filter.rs:160-175`) gains `nin` / `neq` forms over the
  catalogue-resolved ids 0292 introduces (`project` / `assignee` on `id`,
  `labels` on `every: { id }`); `assignee` is no longer name-based. `Search`
  (filter.rs:88-92) carries negated value lists per field, plus the
  nullable-relation `null` branch where the empty-field policy requires it.
  `IssueFilter` has no top-level `not`, so negation is per-field only. Golden
  fixture `issue-filter.txt` plus a `parse_spec` grammar extension; confirm the
  `every` / `null`-relation semantics with a client-contract check.
- Port: `SearchScope.filters: Vec<(String, String)>` (`cli/tracker/src/lib.rs:326`)
  carries polarity explicitly; `tracker` / `work` are public-API-pinned, so the
  snapshot regenerates (`mise run public-api:update`).
- Config: `PullConfig.filters: Vec<(String, Vec<String>)>` and its parser /
  validator in `cli/work/src/pull.rs` extend to per-key positive/negative sets;
  `validate` gains the operator-key check.

## Drafting Notes

- Reframed from "add negation" to "promote a partly-dormant, asymmetric seam to
  a validated cross-tracker surface" after finding Jira's JQL already emits
  `NOT IN` while Linear emits none.
- Recommended the `{ in, not_in }` per-key operator object; kept the
  alternatives as an Open Question because the shape interacts with 0292 and the
  deferred nested model.
- Flagged the to-many `labels` negation and the empty-field exclusion as the two
  real correctness risks — both Open Questions, not silent assumptions.
- Sized medium: a public-API port change plus divergent per-tracker negation
  semantics, layered on 0292's id-keyed lowering.
- Web research (2026) drove the Linear lowering: `neq` / `nin` comparators and
  the `labels` `every` quantifier are server-side, so no client-side filtering
  is needed for set-membership negation. The absence of a top-level `not` is
  what bounds the story to set-membership negation.

## References

- Related: 0229 — Per-Tracker Pull Scope Configuration (filter mechanism); 0292
  — Linear Pull Filters via Catalogue-Resolved Ids (blocker; establishes the
  id-keyed lowering); 0146 — parent epic
- Code: `cli/jira-client/src/jql.rs`, `cli/linear-client/src/filter.rs`,
  `cli/work/src/pull.rs`, `cli/tracker/src/lib.rs`
- Linear API filtering guide: https://linear.app/developers/filtering
- Linear GraphQL Filters changelog:
  https://linear.app/changelog/2021-09-16-graphql-filters
