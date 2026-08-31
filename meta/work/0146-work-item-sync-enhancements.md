---
type: "work-item"
id: "0146"
title: "Work Item Synchronisation Enhancements"
date: "2026-06-22T23:41:03+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "epic"
priority: "medium"
source: "note:2026-06-22-ideas-backlog"
relates_to: ["work-item:0171"]
tags: ["sync", "linear", "jira", "tracker", "scoping", "configuration"]
last_updated: "2026-08-30T14:24:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-167"
---

# 0146: Work Item Synchronisation Enhancements

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Enhance work item synchronisation with remote trackers (Jira/Linear) with richer
field mapping, relationship handling, and — the theme that now dominates this
epic — a corrected scope-and-configuration model. Discovery, creation, and pulls
must be bounded by an explicit, tracker-appropriate scope, configured once and
validated at config time, rather than by a Jira-shaped assumption hard-coded into
tracker-agnostic engine code.

## Context

Extracted originally from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`) as a thin set of mapping/relationship
candidates. The concrete driver for the scoping work is bug 0220: untracked-remote
discovery is silently skipped on Linear because the discovery gate demands
`scope.project`, which only Jira genuinely needs. Investigating that bug surfaced
a deeper design problem in how tracker scope and the work-item key are modelled
and configured.

Three structural findings frame the scoping work:

- **The gate encodes one tracker's semantics for all.** `discovery_enabled =
  scope.project.is_some() || scope.all_projects` (`cli/work-adapters/src/sync/run.rs`)
  is Jira's `E_JQL_NO_PROJECT` constraint (`cli/jira-client/src/jql.rs`) hoisted
  into tracker-agnostic code. Linear's client self-bounds to a team, so the gate
  suppresses a search that was never at risk of flooding.
- **`default_project_code` is misnamed and overloaded.** Its real job is the key a
  tracker stamps on issue identifiers — a Jira *project* key, a Linear *team* key —
  i.e. the key of the entity that owns the issue-number sequence. It is not a
  "project", and its second, accidental role as the sync scope is what let the bug
  hide.
- **The key concept and the integration concept live at different layers.** The
  tracker-native key (`linear.team_key` / `jira.project_key`) is meaningful to the
  standalone integration skills with no work-management framework present; the
  work-item prefix (`work.key`) is the work layer's view of that same key. The
  dependency must run work → integration, never the reverse, so integration skills
  can be packaged independently in future.

## Requirements

### Existing candidates (field mapping and relationships)

- Add **status** mapping to work item synchronisation.
- Add **kind** mapping to work item synchronisation.
- Add **priority** mapping to work item synchronisation.
- Establish **parent-child relationships** between work items on synchronisation.
- **Restrict** work item sync based on configured labels or projects — realised by
  the per-tracker pull-scope child below.

### Scope-and-configuration redesign (children)

- **Tracker-aware discovery gate** — replace the `scope.project.is_some()` gate
  with a tracker capability so a self-bounding client (Linear) enables discovery,
  while a project-requiring client (Jira) still refuses an unbounded search.
  Bound Linear discovery to the configured key's team and require the key; a loud
  error replaces the silent zero-pulls. This is the fix for **0220** and the first
  child to ship.
- **Configuration key model** — rename `work.default_project_code` → `work.key`
  and the `id_pattern` placeholder `{project}` → `{key}`. Introduce the layered
  ownership: `linear.team_key` / `jira.project_key` is the canonical,
  integration-owned key; `work.key` derives from it when a tracker is configured,
  and is set directly only in tracker-less repos. Setting both is a
  config-validation error. Provide a migration / read-time alias. `init-linear` /
  `init-jira` write the key they discover.
- **Per-tracker pull scope** — a per-tracker `pull` block: `additional_teams` /
  `additional_projects` (broaden beyond the creation entity), `all_teams` /
  `all_projects` (whole accessible workspace, mutually exclusive with the
  `additional_*` list), and a generic `filters` bag. Each tracker declares a
  filter schema so required keys fail at config time rather than at sync time.
- **Tracker-owned ID generation** — optionally let a configured tracker mint the
  work-item ID (stub-create remote, adopt its identifier locally so `id ==
  external_id`), and codify the `id`-immutability boundary: immutable once synced,
  provisional-and-rewritable before first push. The largest and least urgent
  child.

## Design

The target scope-and-configuration model, common to both trackers.

**Two distinct concepts, separated.** The *creation home* is the single entity new
items are minted into (one Jira project / one Linear team); it also sets those
items' key. The *pull scope* is the set of entities discovery enumerates; it always
includes the creation home and may be broader. The old field conflated them.

**The key is primary; the tracker entity is resolved from it.** `work.key` (or the
integration-owned key it derives from) is the fundamental value — it exists even
with no tracker, as the local ID prefix. When a tracker is configured, its entity
is resolved from the key: Jira by identity (the key *is* the project key), Linear
by catalogue lookup (team key → team UUID).

**Scope comes from the key; the credential is access control.** Linear discovery is
bounded to the keyed team and requires the key, exactly as Jira requires a project.
The credentialed team from `catalogue.json` provides the key → UUID resolution table
and the access-control boundary (which teams are reachable) — never the scope.

**Unbounded is always opt-in and loud.** With the key always contributing the
creation entity to scope, the "no project" condition that produced 0220 becomes
unreachable through config. The only route to a whole-workspace search is an
explicit `all_projects` / `all_teams`, backstopped by `max_pulls`.

Config shape:

```yaml
# Linear-backed repo
work:
  id_pattern: "{key}-{number:04d}"   # {key} resolves from linear.team_key
linear:
  team_key: "PP"                     # canonical, required; resolves to UUID via catalogue
  pull:
    additional_teams: ["XX"]
    all_teams: false
    filters: { label: "sync" }

# Jira-backed repo
work:
  id_pattern: "{key}-{number:04d}"
jira:
  project_key: "PROJ"                # canonical, required; satisfies the JQL project requirement
  pull:
    additional_projects: ["OTHER"]
    all_projects: false
    filters: { label: "sync" }

# Tracker-less repo
work:
  key: "PROJ"
  id_pattern: "{key}-{number:04d}"
```

`all_projects` / `all_teams` semantics: Jira omits the JQL `project =` clause,
searching every accessible project; Linear drops the team filter and suppresses the
credentialed-team fallback, searching every accessible team. Internally the port
carries one entity-neutral boolean; each adapter interprets it, and the config
surface uses each tracker's vocabulary.

## Acceptance Criteria

- [ ] Status, kind, and priority are mapped bidirectionally during sync.
- [ ] Parent-child relationships are established/maintained on sync.
- [ ] Discovery is bounded by an explicit tracker-appropriate scope derived from the
      configured key, and never by a hard-coded per-tracker assumption in engine
      code.
- [ ] With no configured scope, a self-bounding tracker (Linear) still runs
      discovery bounded to the keyed entity; a project-requiring tracker (Jira)
      reports a config error rather than an unbounded or silently-empty search.
- [ ] A whole-workspace search happens only when `all_projects` / `all_teams` is
      explicitly set; the default is bounded.
- [ ] The tracker-native key (`linear.team_key` / `jira.project_key`) is usable by
      the integration skills with no `work.*` present; `work.key` derives from it
      when a tracker is configured, and setting both is a config-validation error.
- [ ] Pulls can be broadened by configured `additional_*` entities, `filters`, or a
      whole-workspace flag, each validated per tracker at config time.

## Open Questions

- What is the conflict-resolution policy when local and remote values diverge on a
  mapped field (status/kind/priority)?
- Under tracker-owned ID generation, how is offline / tracker-unreachable creation
  handled — block creation, or mint a provisional `key`-prefixed ID and rewrite it
  on first push (relaxing `id` immutability for unsynced items only)?

Resolved during refinement (2026-08-30):

- *How are scope/mappings configured per tracker?* — via per-tracker `pull` blocks
  with a per-tracker filter schema; the base key is the integration-owned
  `<tracker>.<entity>_key`, from which `work.key` derives.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- Builds on the existing `/sync-work-items` engine and the `external_id`
  remote-key convention.
- The key configured for a tracker names an entity the credential can write to;
  a mismatch is an authz/config error, not a silent fallback.

## Technical Notes

- The port already models arbitrary filters (`SearchScope.filters:
  Vec<(String, String)>` in `cli/tracker/src/lib.rs`) — promote it to the primary
  scoping mechanism and demote `project` to one filter among many.
- Linear already resolves a team key → UUID via the catalogue
  (`cli/linear-client/src/auth.rs`), and already falls back to the credentialed
  team at search time (`cli/linear-client/src/client.rs`); the scoping child routes
  the configured key through the existing resolver rather than building new
  machinery.
- Only Jira and Linear are wired trackers today
  (`cli/work-cli/src/tracker_registry.rs`); Trello and GitHub Issues are `0181`.
  A future self-scoping adapter (e.g. single-repo GitHub Issues) would hit the same
  gate defect the tracker-aware gate prevents.

## Stories

- 0220 — Tracker-aware discovery gate (fixes silent Linear untracked pulls);
  reparented here.
- 0228 — Configuration key model (`work.key` rename + layered
  `<tracker>.<entity>_key` ownership + migration).
- 0229 — Per-tracker pull scope (`additional_*`, `all_*`, `filters`, config-time
  schema).
- 0230 — Tracker-owned work-item ID generation (stub-mint; `id`-immutability
  boundary).
- TBD (existing candidates) — status / kind / priority mapping; parent-child
  relationships on sync.

## Drafting Notes

- Enriched 2026-08-30 from an interactive design session driven by bug 0220. The
  epic was previously a thin extract; the scope-and-configuration model, the config
  shape, and the child breakdown are new.
- Key naming settled on `work.key` (not `default_project_code` / `id_key`) because
  the value identifies the tracker entity, of which the ID prefix is one
  consequence.
- Ownership inverted relative to the first proposal: the integration section owns
  the canonical key and `work.key` derives from it, so integration skills never
  depend on `work.*` and can be packaged independently.
- Linear scope required and taken from the key, with the credential reframed as
  access control — a deliberate departure from defaulting scope to the credentialed
  team.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
- Related: 0171, 0220
