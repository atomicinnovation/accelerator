---
type: "codebase-research"
id: "2026-09-11-0229-per-tracker-pull-scope-configuration"
title: "Research: Per-Tracker Pull Scope Configuration (0229)"
date: "2026-09-11T10:03:48+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0229"
parent: "work-item:0229"
topic: "Per-Tracker Pull Scope Configuration"
tags: ["research", "codebase", "sync", "tracker", "pull-scope", "search-scope", "config", "jira", "linear"]
revision: "53dffc3c029bf4d59acfba58a6da0e451eba7d4b"
repository: "accelerator"
last_updated: "2026-09-11T14:45:40+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Recorded post-research walkthrough decisions Q1-Q4: all_* base-entity handling; multi-scope discovery via live enumeration plus grow-on-pull committed index; extended truncation-to-hard-error scope with two stale-premise corrections; and no sequencing constraint against 0285 (landed) / 0255 (orthogonal)"
schema_version: 1
---

# Research: Per-Tracker Pull Scope Configuration (0229)

**Date**: 2026-09-11T10:03:48+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 53dffc3c029bf4d59acfba58a6da0e451eba7d4b
**Branch**: qyzmlvlo (jj working copy off `main`)
**Repository**: accelerator

## Research Question

What does the codebase look like today for the story at
`meta/work/0229-per-tracker-pull-scope-configuration.md` — the per-tracker `pull`
block that broadens pull discovery with `additional_*` / `all_*` scopes, a
normalised `filters` bag, and configurable `max_items` / `max_pages` ceilings,
plus discovered-set dedup and total ordering? Where are the seams, what already
exists, and what is genuinely new work?

## Summary

The story's central seam is real and dormant exactly as described:
`SearchScope.filters` (`cli/tracker/src/lib.rs:267`) is a wired-but-empty
`Vec<(String, String)>` that both clients already lower, and production always
builds it empty (`cli/work-cli/src/sync.rs:889`). Promoting it to a validated
config surface is well-founded. Two of the story's premises, however, are stale
against current code and must be re-framed before planning.

**Two premises need correction.** First, the "silent truncation → hard error"
work is **already done for the pull path**: `discover_untracked` refuses on
`Discovery { complete: false }` via `RunError::DiscoveryIncomplete`
(`cli/work-adapters/src/sync/run.rs:802-807`), pinned by a test. Silent
truncation survives only in the standalone `search` subcommands and the keyed
reconcile reads — not the untracked pull. Second, the "credentialed-team
fallback" the story says `all_teams` must suppress **was removed by 0220**; it
was replaced by two deliberate flood-guards, and `all_teams` must relax those
guards under an explicit flag rather than suppress a fallback that no longer
exists.

**The genuinely new work is larger than a config surface.** Multi-scope
discovery does not exist — `SearchScope` holds one entity, there is no
discovered-set self-dedup, and there is no numeric-sequence ordering (`PP-10`
currently sorts before `PP-2`). OR-within-a-key (`IN`) is unexercised on both
trackers. The `unlimited` sentinel, config-sourced ceilings, structured config
values, and a multi-team Linear catalogue are all net-new. Each is small in
isolation; together they are the substance of the story, and several land on the
same `run.rs` engine region three sibling stories (0257, 0285, 0255) also touch.

## Detailed Findings

### The search-scope seam (`SearchScope` + `filters`)

`SearchScope` is a three-field struct with no pagination or ceiling fields
(`cli/tracker/src/lib.rs:257-269`):

```rust
pub struct SearchScope {
    pub project: Option<String>,   // single base entity: Jira project / Linear team
    pub all_projects: bool,        // the one entity-neutral "whole workspace" flag
    pub filters: Vec<(String, String)>,  // the dormant seam
}
```

Production constructs it in exactly one place
(`cli/work-cli/src/sync.rs:882-890`): `project` from `resolve_active_scope_key`,
`all_projects: false` hard-coded, `filters: Vec::new()`. The base key is derived
by dispatching on the active integration (`cli/work-cli/src/sync.rs:702-721`):
`jira` → `jira_client::auth::project_code`, `linear` →
`linear_client::auth::team_key`, else the legacy
`work.default_project_code`. The value flows into `SyncRequest.scope`
(`cli/work-adapters/src/sync/run.rs:132`), through
`ports.tracker.resolve_scope` then `discover_untracked` →
`tracker.search(scope)` (`run.rs:795-818`, `526-547`).

**The port stays entity-neutral; adapters diverge at `resolve_scope`.** Jira's
`resolve_scope` returns the scope essentially unchanged
(`cli/jira-client/src/client.rs:592-604`); Linear's mutates it, substituting the
team key for the team UUID (`cli/linear-client/src/client.rs:674-699`). This
asymmetry matters: `additional_*` and `all_*` changes must be made twice, in two
different shapes.

⚠️ **Precedence collision to resolve.** The struct doc states `project` wins when
both `project` and `all_projects` are set (`cli/tracker/src/lib.rs:259-264`). The
story keeps the keyed base entity "always implicit" while `all_*` searches the
whole workspace — but if the base entity always populates `project`, and
`project` wins, `all_*` never takes effect. The story must decide whether
`all_teams` overrides a populated `project` or requires the construction site to
leave `project` unset.

### Jira JQL lowering

Filters lower through a `Family` intermediate: each `(field, value)` pair becomes
a single-value `Family` (`cli/jira-client/src/client.rs:270-277`), and
`family_clauses` emits `field IN (...)` / `field NOT IN (...)`
(`cli/jira-client/src/jql.rs:230-237`), joined with `" AND "`
(`jql.rs:205`). The project clause is `project = '<code>'`, emitted only when
`project` is `Some` (`jql.rs:177-179`); `all_projects` contributes no clause, it
only unlocks the no-project guard (`jql.rs:169-174`).

| Aspect | Current behaviour | Reference |
| --- | --- | --- |
| Keys AND'd | Yes — single `" AND "` join | `jql.rs:205` |
| Values OR'd within a key | Capable (`IN`) but **not exercised** | `jql.rs:230-237` |
| Same-key merge | ❌ two pairs → two AND'd clauses | `client.rs:273-276` |
| No-project search | Live path when `all_projects` set | `client.rs:596`, `jql.rs:177` |
| Value escaping | Single-quoted, `'`→`''`, control bytes rejected | `jql.rs:70-91` |
| Field/key escaping | ❌ raw passthrough, no allow-list | `jql.rs:213-215, 232` |

⚠️ Two findings bear on the story. The current `filters`→`families` mapping puts
**one value per family**, so `{label: [a, b]}` would lower to `label IN ('a') AND
label IN ('b')` (empty result), not `label IN ('a', 'b')`. Achieving the AC's
OR-within-a-key needs the mapping to group same-key values into one `Family`.
Separately, filter **keys** interpolate raw into JQL with no allow-list
(`jql.rs:213-215`) — the story's per-tracker filter schema (reject unsupported
keys at `configure`) is the missing guard, and there is no key validation
anywhere on this path today.

Search pages on an opaque `nextPageToken` cursor at 100/page, capped at
`transport.config().max_pages` (default 20), returning `complete: false` on
cap-hit/deadline/failure (`client.rs:290-321`).

### Linear filter lowering, catalogue resolution, and the 0220 flood-guards

Filters lower in two hops: `search` maps known keys onto a typed `Search`
(`cli/linear-client/src/client.rs:709-721`), then `compose` emits each field as
one `IssueFilter` member (`cli/linear-client/src/filter.rs:82-112`).

| `filters` key | `IssueFilter` emission | Operator |
| --- | --- | --- |
| `state` | `{"state":{"id":{"eq":<uuid>}}}` | name→UUID, unknown refused |
| `assignee` | `{"assignee":{"name":{"eqIgnoreCase":v}}}` | equality |
| `label` | `{"labels":{"name":{"eq":v}}}` | equality |
| `text` | `{"title":{"containsIgnoreCase":v}}` | contains |
| (team, not from `filters`) | `{"team":{"id":{"eq":<uuid>}}}` | equality |

⚠️ Keys AND (one JSON object) — confirmed. **OR-within-a-key does not exist**:
each `Search` field is `Option<String>` (last-wins on duplicate keys), every
emission is `eq`/`containsIgnoreCase`, there is no `in` operator. The story's
OR-within-a-key is new on Linear as well as Jira. Unrecognised filter keys are
silently dropped (`_ => {}`, `client.rs:719`) — again, the schema-at-`configure`
guard is missing.

**The credentialed-team fallback was removed by 0220.** The story (and the 0220
research it cites) describe `team_id: scope.project.clone().or_else(|| Some(...))`
— that code is gone. It was replaced by two guards that exist to stop a 997+
workspace-wide flood:

1. `resolve_scope` refuses an unkeyed scope: `E_SEARCH_NO_TEAM`
   (`client.rs:679-685`).
2. `search` defensively refuses a `None` team id: `E_SEARCH_UNRESOLVED_SCOPE`
   (`client.rs:702-708`).

Dropping the team filter means letting `team_id: None` reach `compose`, which
emits an empty filter — precisely the whole-workspace enumeration 0220
forbade. So `all_teams` is a **deliberate inversion**: relax both guards, but
only under the explicit flag, so an *accidental* unresolved key still refuses.
`compose` already emits no `team` member when `team_id` is `None`
(`filter.rs:87-89`), so the filter-drop itself needs no change there.

**Catalogue resolution is single-team.** `CatalogueTeam`
(`cli/linear-client/src/catalogue.rs:107-137`) holds one `(key, id)` pair from
`<integrations_root>/linear/catalogue.json` and resolves by trimmed equality; any
key but the one catalogue team resolves to `None` →
`E_SEARCH_UNKNOWN_TEAM`. Multi-team `additional_teams` resolution needs both
`catalogue.json` and `CatalogueTeam` extended to a multi-entry map. Resolution is
local-only (no network); a missing key is a local mapping gap, indistinguishable
from remote absence — the story's sync-time resolution error covers both.

Linear pages on a Relay cursor at 250/page (`filter.rs:71`), same
`max_pages` cap (`client.rs:326-362`).

### The two ceilings and discovery completeness

| Ceiling | Where defaulted | Enforced | Config-sourced today? |
| --- | --- | --- | --- |
| `max_pulls` / `max_pushes` (25) | CLI arg `cli.rs:277-284` | post-discovery refusal `run.rs:880-891` | ❌ CLI arg only |
| `max_pages` (20) | `transport.rs:28` | in-client paging loop | ❌ always `TransportConfig::default()` |

`max_pulls` is a post-discovery, all-or-nothing refusal: `prepare_run` sums
`plan.pull_count() + untracked.len()` and returns `RunError::Refused` before any
write (`run.rs:880-891`). `max_pages` is a transport-layer cap, never sourced
from config — every construction site uses `TransportConfig::default()`.

⚠️ **The "silent truncation → hard error" premise is stale for the pull path.**
`discover_untracked` already hard-errors on `complete: false`
(`run.rs:802-807`), pinned by `an_incomplete_discovery_is_refused_with_guidance`
(`cli/work-adapters/tests/sync_create.rs:794-827`). The genuinely silent
(exit-0) truncation lives in: the standalone `search` subcommands
(`cli/jira-cli/src/main.rs`, `cli/linear-cli/src/main.rs`) that surface
`"truncated": true` without failing; and the keyed reconcile reads, where
`fetch_chunk` marks truncated keys *indeterminate* (`client.rs:252-254`). The
story's real ceiling work is making `max_pages` **configurable**, not inventing
the hard error.

⚠️ **The two ceilings can never both fire.** A `max_pages` truncation sets
`complete: false`, which short-circuits at `run.rs:803` *before* the `max_pulls`
count at `run.rs:880`. So `max_pulls` only ever sees a complete set, and a run
cannot exercise the `max_items` ceiling on a `max_pages`-truncated set. The
story's two ceiling ACs (crossing `max_items` vs reaching `max_pages`) are
therefore mutually exclusive per run — worth stating explicitly.

**No `unlimited` sentinel exists.** All three are plain `usize`; `0` already
means "refuse every pull" (strictest), corroborating the story's choice of
`unlimited` over `0`. Carrying `unlimited` needs a new enum/`Option` type on
each ceiling.

### Reconciliation: dedup and ordering

Both are net-new. The only dedup today is discovered-vs-local: `discover_untracked`
subtracts canonicalised local `external_id`s via a `BTreeSet<String>`
(`run.rs:531-542`), using `canonical_external_key` (whitespace-stripped,
ASCII-uppercased, `create.rs:114-120`). **There is no discovered-vs-discovered
dedup** — a provider returning the same id on two pages passes it through twice.

**There is no ordering of the discovered set.** `discover_untracked` preserves the
tracker's page order; only *local* items are sorted, and by raw string
(`sync.rs:133`), with `BTreeMap`/`BTreeSet` key-ordering by string, not numeric
sequence. So `PP-10` sorts before `PP-2` today — exactly the defect the story's
ordering AC targets. Identifier parsing exists (`WorkItemIdScheme::normalise_id`
splits prefix/digits, `cli/corpus/src/work_item_id.rs:87`; the pattern DSL's
`parse_full_id` returns `{key, number}`,
`cli/corpus-adapters/src/work_item_pattern.rs:468`) but is **lexical**, with no
numeric comparator. The story's "prefix lexical, then numeric sequence" total
order is new sorting logic over `ExternalId`.

### Config system: catalogue, levels, structured values, validation

The config concern splits across three crate layers: `cli/config/` (domain —
catalogue, key/level model, merge), `cli/config-adapters/` (filesystem +
frontmatter), and per-tracker readers in the client crates.

- **Catalogue** (`cli/config/src/catalogue.rs`) registers known keys and scalar
  defaults. Tracker keys (`jira.*`/`linear.*`) live in `EXTRA_KEYS` (no default,
  `L121-138`); `work.default_project_code`/`work.key` in `WORK_KEYS`
  (`L98-102`). A `pull` block adds keys here.
- **Levels** (`cli/config/src/level.rs`): team `.accelerator/config.md`,
  personal `.accelerator/config.local.md`, personal wins. Merge is
  last-writer-wins **per key** (`cli/config/src/service.rs:350-441`).
- **Structured values are already legal.** `Node` models
  `Scalar`/`Sequence`/`Mapping` (`cli/config/src/node.rs:5-23`), and ADR-0047
  dropped the scalar-only cap. But catalogue *defaults* are scalar; carrying a
  structured default for the `filters` bag is new catalogue territory.
- **`max_pulls` is not a config key today** — it is a CLI arg threaded through
  `sync.rs` into `run.rs`. `pull.max_items` is new catalogue→auth-read→request
  wiring.
- **Validation** at `configure` lives in `cli/launcher/src/config_command/`;
  `work.integration` is fail-closed against the catalogue
  (`core/work.rs:17-58`). There is no schema validation of value *shape* today —
  the story's per-tracker filter schema and ceiling-value validation are new,
  following the ADR-0017 fail-loud precedent.

**Whole-block replacement is consistent, not a departure.** ADR-0047's "override
the same key" read literally (treat `pull` as one key) means the personal `pull`
map replaces the team map wholesale — aligned with the plugin's replace-not-merge
house style (ADR-0017 templates). ADR-0047 legalised structured values but never
defined merge-vs-replace for them, so 0229 *fills an unspecified semantic* rather
than departing from a decided one. Bonus: whole-block replacement side-steps
ADR-0047's known "no unset sentinel" gap (a developer drops a team sub-field by
omitting it from their personal block).

### Base-scope resolution (0228) and its current-code state

0228 (`meta/work/0228-layered-configuration-key-model.md`, status **`ready`**,
unimplemented) splits the overloaded `default_project_code` into an
integration-owned **scope key** (`jira.project_key` / `linear.team_key`) and a
work-owned **`work.key`** local-ID prefix. The base scope 0229 broadens is the
single creation-home entity resolved from the scope key — Jira by identity,
Linear by catalogue lookup. Migration is a tracker-aware read-time alias:
`work.key` ships 1.24.0, the `default_project_code` alias is removed 1.25.0.

Notably, the per-tracker scope-key reads with the legacy alias **already exist in
current code** (`resolve_active_scope_key` at `sync.rs:702-721`,
`legacy_alias.rs`, `auth.rs`), even though 0228 reads `ready`. So 0228's
base-resolution seam is substantially present; 0229 can build on the existing
`resolve_active_scope_key` seam rather than waiting for 0228 machinery that is
partly in place. The layered team/personal precedence 0229 inherits is the
existing per-key machinery, resolved on the effective post-override config.

### Integration surface with sibling stories

0229 shares the engine pull/discovery path with three siblings; the coordination
note names 0229 explicitly.

| Story | Status | Shared surface | Relation to 0229 |
| --- | --- | --- | --- |
| 0257 sync-specific items | merged (PR #107) | discovery gate, `ItemSelection` | concurrent, no ordering |
| 0285 targeted pull | approved | `run.rs:786-787` gate, pull accounting | opposite direction (narrows) |
| 0255 chunk merge | named only | same engine path | no docs in set |

**Route scope-broadening through the existing lever.** 0257/0285 put
item-narrowing and discovery-suppression behind one `ItemSelection { All,
Targeted }` value so they cannot desync (the "re-import trap"). 0229 broadening
`untracked` from a wider `search(scope)` writes to the same
`run.rs` branch; it should extend this lever, not add a parallel flag. Reuse
`canonical_external_key` for the discovered-set dedup to stay consistent with the
established engine predicate. The siblings decided *concurrency with no delivery
ordering*, so 0229 inherits a **rebase obligation**, not a semantic dependency.

Note: the 0257/0285 *research docs* use `scope`/`max_pulls` vocabulary and lack
`SearchScope`/`max_pages` — a documentation-vintage artefact, not a code reality.
The live code has both `SearchScope` (`cli/tracker/src/lib.rs`) and `max_pages`
(`cli/tracker-support/src/transport.rs`), confirmed directly.

## Code References

- `cli/tracker/src/lib.rs:257-269` — `SearchScope` struct (`project`,
  `all_projects`, `filters`); `:278-284` — `Discovery { found, complete }`;
  `:456-486` — `search` / `resolve_scope` trait methods.
- `cli/work-cli/src/sync.rs:882-890` — sole production `SearchScope` construction
  (`filters: Vec::new()`); `:702-721` — `resolve_active_scope_key`.
- `cli/work-adapters/src/sync/run.rs:802-807` — hard refusal on incomplete
  discovery; `:880-891` — `max_pulls`/`max_pushes` refusal; `:526-547` —
  `discover_untracked` + discovered-vs-local dedup.
- `cli/jira-client/src/jql.rs:164-237` — `compose`, project clause,
  `family_clauses` (`IN`/`NOT IN`), `quote` escaping; `cli/jira-client/src/client.rs:266-326`
  — `discover` (filters→families), `:592-608` — `resolve_scope`/`search`.
- `cli/linear-client/src/client.rs:674-730` — `resolve_scope` (key→UUID),
  `search` (known-key filters), the two flood-guards; `filter.rs:82-112` —
  `compose`; `catalogue.rs:107-137` — single-team `CatalogueTeam`.
- `cli/tracker-support/src/transport.rs:15-31` — `TransportConfig` (`max_pages:
  20`, never config-sourced).
- `cli/work-cli/src/cli.rs:277-284` — `--max-pulls`/`--max-pushes` default 25.
- `cli/config/src/catalogue.rs:98-138` — key catalogue; `node.rs:5-23` —
  `Scalar`/`Sequence`/`Mapping`; `service.rs:350-441` — per-key merge;
  `level.rs:6-28` — team/personal levels.
- `cli/corpus/src/work_item_id.rs:87` — `normalise_id` (lexical prefix/digit
  split); `cli/corpus-adapters/src/work_item_pattern.rs:468` — `parse_full_id`.
- `cli/work-adapters/tests/sync_create.rs:794-827` — existing incomplete-discovery
  hard-error test.

## Architecture Insights

- **The port is entity-neutral; the divergence is at `resolve_scope`.** Jira
  passes the scope through; Linear substitutes key→UUID and enforces two
  flood-guards. Every `additional_*`/`all_*` change is made twice, in two shapes.
  `resolve_scope` is public-API-pinned (six impls) — a behaviour change
  regenerates the `tracker` public-api snapshot (`mise run public-api:update`).
- **OR-within-a-key is unexercised on both trackers.** Jira has the `IN`
  machinery but the mapping never fills a multi-value family; Linear has no `in`
  operator at all. The AC needs new lowering on both sides plus a same-key
  grouping step in the filters→scope construction.
- **`SearchScope` is single-entity.** `additional_*` needs either N searches
  merged (then deduped/ordered) or a multi-entity scope type. The dedup and total
  order the story requires do not exist; both are new engine logic keyed off
  `canonical_external_key` and a numeric identifier comparator.
- **Filter keys are unvalidated and uninjected-guarded.** Jira interpolates keys
  raw into JQL; Linear silently drops unknown keys. The per-tracker schema (fail
  at `configure`) is the missing structural guard the story introduces.
- **Ceilings are ordered by construction.** `max_pages` truncation pre-empts the
  `max_pulls` count, so the two ceiling ACs cannot co-occur in one run.

## Historical Context

- `meta/decisions/ADR-0047-multi-level-userspace-configuration-model.md`
  (accepted) — the live config model: two project-scoped tiers, per-key
  last-writer-wins, arbitrary YAML structure legalised, resolution in the CLI
  core. Governs the `pull`-block surface.
- `meta/decisions/ADR-0017-configuration-extension-points.md` (superseded) —
  replace-not-merge house style and fail-loud-on-misconfiguration precedent that
  0229's whole-block replacement and configure-time validation should follow.
- `meta/plans/2026-08-30-0220-untracked-remote-discovery-on-linear.md` (done) —
  built the two Linear flood-guards and single-team `CatalogueTeam` that 0229
  must deliberately unwind for `all_teams`; explicitly deferred all-teams to
  0146 (→ 0229).
- `meta/work/0228-layered-configuration-key-model.md` (ready) — the scope-key /
  `work.key` split 0229's base scope resolves from; largely already present in
  code.
- `meta/research/codebase/2026-09-06-0257-sync-specific-work-items.md` and
  `meta/research/codebase/2026-09-08-0285-targeted-pull-of-remote-only-work-items.md`
  — the `ItemSelection` single-lever model and the integration watch-out naming
  0229 on the shared `run.rs` path.

## Related Research

- `meta/research/codebase/2026-08-30-0220-untracked-remote-discovery-never-runs-on-linear.md`
  — Linear discovery scope, the fallback-removal, catalogue resolution.
- `meta/plans/2026-08-13-0194-tracker-crate-and-remote-sync-engine.md` — origin
  of `SearchScope` and the ceilings.
- `meta/plans/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md` —
  the provider clients over the tracker port.

## Decisions

Recorded during the post-research walkthrough (2026-09-11).

- **`all_*` clears the base `project` at construction.** When the effective
  `pull` block sets `all_*`, the scope is built with `project` unset; the
  whole-workspace search covers the base entity as a superset. The documented
  `project`-wins precedence (`cli/tracker/src/lib.rs:259-264`) is untouched, and
  no adapter or public-API precedence change is needed.
- **Multi-scope discovery is a resolved entity list on `SearchScope`, resolved
  live at discovery and validated against current credential visibility.** Add
  an additive multi-entity field to `SearchScope` carrying base + `additional_*`
  (with `all_*` meaning the whole visible set). Scope entities are resolved by
  **live enumeration at pull time** — the endpoints already exist (Jira
  `/rest/api/3/project`, `cli/jira-client/src/discovery.rs:19`; Linear
  `teams { nodes }` / `list_teams`, `cli/linear-client/src/discovery.rs:19,54`)
  — because discovery is intrinsically online, so no offline guarantee is
  broken. A named entity the credential cannot currently see aborts the pull;
  this side-steps the `all_*` empty-filter flood (no 0220 guard inversion) by
  emitting an explicit enumerated list rather than an unbounded query.
- **The committed entity index grows lazily on pull for offline local
  validation.** `projects.json` / `catalogue.json` are committed, repo-level
  indices; only per-credential identity (`site.json` / `viewer.json`) is
  gitignored (`cli/jira-client/src/cache.rs:24-30`,
  `cli/linear-client/src/cache.rs:28-31`). Seeded with the primary entity at
  init, each grows when a pull imports an item from a new entity: Jira adds
  `{key, id, name}`; Linear adds the team plus its team-scoped workflow states.
  These indices back offline resolution/validation of local work items, distinct
  from the live discovery-time enumeration above.
- **Jira `fields.json` stays a standalone instance dictionary, outside the
  grow-on-pull mechanism.** It is populated from the instance-global
  `/rest/api/3/field` (`cli/jira-client/src/discovery.rs:20`); a new project
  introduces no new fields, so pulling from additional projects never updates
  it. Its only staleness case (a newly-created instance field) is a normal
  dictionary refresh / refresh-on-miss, and field-availability validation is a
  push concern this pull-only story excludes.
- **Truncation/ceiling scope is extended beyond the pull path.** The pull-path
  hard error already exists (`cli/work-adapters/src/sync/run.rs:802-807`, pinned
  by `cli/work-adapters/tests/sync_create.rs:794-827`), so its only remaining
  work is making `max_pages` configurable. The story additionally promotes the
  two surviving silent truncations to hard errors: the standalone `search`
  subcommands (`cli/jira-cli`, `cli/linear-cli`) and the keyed reconcile reads.
  ⚠️ Consequence for the keyed path — a truncated keyed reconcile read today
  degrades to *indeterminate* (never *absent*) and skips deletion while the sync
  proceeds; as a hard error it instead aborts the sync (all-or-nothing, so no
  wrongful deletion) rather than proceed, trading availability for fail-loud
  behaviour, with the now-configurable `max_pages` as the release valve.
- **Two stale premises in the work item are corrected.** "Incomplete discovery
  yields silent truncation; this story promotes it to a hard error" (Technical
  Notes) is stale — the pull hard error already exists. "`all_teams` must
  suppress the credentialed-team fallback (Linear's default… introduced in
  0220)" (Context, AC 3) is doubly stale — 0220 *removed* that fallback rather
  than introducing it, installing two flood-guards in its place, so there is
  nothing to suppress. **AC 3 is rewritten**: under the Q2 decision `all_*`
  emits an enumerated visible-set list (`project IN (...)` / `team in [...]`),
  not a no-constraint query, so the 0220 guards are not inverted.

- **No sequencing constraint against the siblings.** 0285 (targeted pull) is
  landed, so 0229 builds directly on the existing `Targeted` branch with no
  in-flight coordination — it changes the `All` branch, a different branch of
  the same `ItemSelection` switch. 0255 (chunk merge) is orthogonal to 0229's
  discovered-set dedup. The design constant stands: route broadening through the
  existing `All`-branch `scope`, not a parallel discovery flag, keeping 0257's
  desync/re-import trap closed.

## Open Questions

None outstanding — all four walkthrough questions (2026-09-11) are resolved in
the Decisions section above.
