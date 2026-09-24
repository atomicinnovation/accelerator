---
type: "plan"
id: "2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids"
title: "Linear Pull Filters via Catalogue-Resolved Ids Implementation Plan"
date: "2026-09-22T11:02:21+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0292"
parent: "work-item:0292"
derived_from: ["codebase-research:2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids"]
tags: ["linear", "pull-filters", "catalogue", "discovery", "filter-lowering", "assignee", "project", "sync"]
revision: "634e2aa521ccf1411e9327a64cf166fd543525ea"
repository: "accelerator"
last_updated: "2026-09-24T18:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Linear Pull Filters via Catalogue-Resolved Ids Implementation Plan

## Overview

This plan adds a Linear `project` pull filter. It also converts `label`,
`assignee` and `state` into catalogue-resolved ids, so that every Linear filter
key emits `{id:…}` in the `IssueFilter`. It changes three things.

**One catalogue shape, synced teams only.** `catalogue.json` converges on a
`baseTeam` pointer and a `teams` array. Each team entry carries that team's
`states`, `labels`, `members` and `projects`. A top-level `labels` array holds
workspace labels, which belong to no team. Only **synced teams** are
catalogued: the base team, and each team that owns a tracked work item. That
extends 0229's rule ("teams items were imported from") so that a lost `teams`
entry is re-derived from the corpus. No team is catalogued merely because the
credential can see it. The legacy `team` and `workflowStates` keys are written
as projections of the base entry for one minor release, then removed.

**One writer.** `init-linear` and apply-mode sync finalisation both write
team entries section by section. Both fetch those entries through the same
team-filtered primitive, and both use the same locked read-merge-write. That
write never drops data it does not own.

**Resolution happens in two places, each with one job.**

- *Pre-flight* runs in the port's `resolve_scope` step, before any request.
  It validates every filter's structure. For a base-only scope whose team
  entry covers the configured families, it also resolves every value, so a bad
  value is refused with zero requests.
- *Completion* runs in `search`, once enumeration has produced the concrete
  team ids for every scope shape. It resolves every value against the scoped
  teams:
  - teams whose entry covers the configured families resolve from the
    catalogue;
  - the other teams are fetched live in one batch, limited to the sections
    the configured families need.

**Refusals are always loud.** A value that resolves to nothing in any scoped
team is refused as a configuration fault. It never silently disappears from
the filter.

**The catalogue heals itself.** An apply-mode sync completes the entry of
every synced team that lacks one or more sections, and adds an entry for each
synced team the catalogue does not name. Teams that are fetched but not synced
are used for the run and never written. `--preview` never writes.

**Prerequisite spike.** Whether members and projects can be fetched for many
teams in one paginated pass, with each node attributed to its team, is
unverified. The spike in "Prerequisite: team-section fetch spike" runs first
and settles this before Phase 1 §2 is built.

## Current State Analysis

The change touches five layers. Four are in `linear-client` (an unpinned
provider crate) and one is in `tracker-support` (config validation). It also
adds one variant to the pinned `tracker` port, and makes small changes to the
`work-adapters` sync engine and the `work-cli` composition root.

| Layer | Anchor | Today | Gap |
|---|---|---|---|
| Config acceptance | `tracker-support/src/pull.rs:272` | one shared `FILTER_SCHEMA`; loop `:366-379` ignores `tracker` | `project` under Linear only |
| Pre-flight | `linear-client/src/client.rs:739` | `resolve_scope` substitutes the base team key only | resolve/validate every filter value, every scope shape |
| Completion and lowering | `linear-client/src/filter.rs:108-156`, `client.rs:775-812` | `state`→init-team id inside `compose`; `assignee`/`label`→`{name:…}` | per-team completion after enumeration; `compose` lowers ids only |
| Resolvers | `linear-client/src/catalogue.rs:28-160` | `CatalogueStates`, `CatalogueTeam`, each loading the file | one parsed catalogue; one resolver abstraction |
| Discovery | `linear-client/src/discovery.rs:19-147` | `TEAMS` unbounded; `TEAM_STATES` for the init team | one team-filtered fetch of complete team entries; workspace labels |

### Key Discoveries

- **Unknown filter keys are silently dropped at intake.** The intake match at
  `client.rs:794-802` has a `_ => {}` arm at `:800`.
- **State filters narrow to the init team today.** `catalogue.json` stores
  `workflowStates` only for the init team (`discovery.rs:105-147`).
  - A multi-team `state` pull therefore lowers to one team's id.
  - `run_search` (`linear-cli/src/main.rs:103-110`) sets `team_id: None`. Only
    that init-team state id keeps `search --state` to a single team, which is
    what the search skill documents.
- **Resolution failures on the pull path are reported as transient.**
  - `compose` runs inside `fetch_page` (`client.rs:295`).
  - `page_all` (`:371-377`) records its error as `Completeness::Transient`.
  - Sync then reports `DiscoveryIncomplete` ("cut short… retry").
- **`search` has no configuration-refusal channel.**
  - `RemoteTracker::search` returns `TrackerError`, which is only `Retryable`
    or `Terminal` (`tracker/src/lib.rs:145-179`), and a read is always
    `Retryable`.
  - `discover_untracked` (`run.rs:546-551`) propagates the error, and the
    engine maps it to `DiscoveryStatus::Failed` (`run.rs:873-878`), so the run
    continues without discovery.
  - Code that matches every `TrackerError` variant:
    - `into_detail` and `Display` in the port;
    - `apply.rs:52-53,387-392`;
    - `work-cli/src/update.rs:228-246`, which clears the baseline on
      `Terminal`;
    - `work-cli/src/create.rs:416-421` (`tracker_error_detail`);
    - `work-cli/src/exit_codes.rs:88-93`;
    - `tracker/tests/errors.rs:125-131`.
  - The frozen dispatch oracle (`tracker/tests/errors.rs`) records 74 as
    resolving above the port and asserts that exactly two codes reach it. The
    port and `work-cli` exit-code docs describe "two classes, and closed".
  - `ConfiguredTrackers::new` is a `const fn`, built in
    `work-cli/src/main.rs:434-457`. `sync::run_sync` receives only
    `&dyn TrackerRegistry` (`sync.rs:1149-1154`).
  - The `accelerator-work` binary has no endpoint override, so its subprocess
    tests cannot reach a `MockServer`
    (`work-cli/tests/sync_resolves_real_client.rs:4-12`).
  - `TrackerError` is in the `tracker` public-API snapshot
    (`tracker/tests/fixtures/public-api.txt:77-86`).
- **`resolve_scope` is the port's pre-flight seam.**
  - Its `ScopeError` maps to `RunError::DiscoveryUnconfigured`
    (`run.rs:856-860`).
  - Sync prints that as `refused: discovery is unconfigured — {detail}`
    (`work-cli/src/sync.rs:1459`) and exits 74 (`UNCONFIGURED`).
- **Broadened scopes skip `resolve_scope` today.**
  - `run.rs:838-861` routes them through `scope::resolve_entities`, which
    clones `filters` (`scope.rs:67`).
  - Linear's `resolve_scope` substitutes the base key for any `Keyed` scope.
  - Jira's `resolve_scope` accepts broadened scopes
    (`jira-client/src/client.rs:710-724`).
  - `RecordingTracker` (`tracker-test-support/src/lib.rs:429-445`) records
    neither `resolve_scope` nor enumeration.
- **Whole-workspace team ids are known only inside `search`.**
- **Preview runs discovery.**
  - `prepare_run` runs in both modes (`run.rs:1006-1008`).
  - The sync skill promises that `--preview` makes no local write, and
    `a_previewed_remote_only_target_writes_nothing_yet_confirms_it` pins it.
  - `grow_linear_catalogue` writes only at finalisation, after the report,
    for applied items (`work-cli/src/sync.rs:947-1030,1409`).
  - It prints `note: … The catalogue is version-controlled and repo-wide.`
- **Resolvers are injected, and `transition` depends on that.**
  - `LinearClient::new` takes `Box<dyn TeamResolver>` and
    `Box<dyn StateResolver>` (`client.rs:139-145`).
  - `transition.rs:28` uses `states().resolve_all`, which yields exit codes
    122 and 123.
  - `lib.rs:26-27` re-exports `CatalogueStates` and `CatalogueTeam`.
- **Construction sites that need the new parameters.**
  - `LinearClient::from_config` is called from
    `work-cli/src/tracker_registry.rs:214` and
    `linear-cli/src/context.rs:163`.
  - `LinearClient::new` is called from `linear-cli/src/context.rs:213`,
    `linear-client/tests/contract.rs:147` and
    `work-adapters/tests/sync_run_real_client.rs:190`.
  - `LinearCache<'a>` borrows `&'a dyn Filesystem` (`cache.rs:87-96`).
- **`CatalogueTeam::load` has a consumer outside the crate.**
  `grow_linear_catalogue` (`work-cli/src/sync.rs:964-978`) uses it.
- **Finalisation sees only the port.**
  - `grow_linear_catalogue` receives `&dyn RemoteTracker`, and
    `ConfiguredTrackers::resolve` returns `Box<dyn RemoteTracker>`. Nothing
    Linear-specific is reachable from it.
  - It maps imported team keys to ids through `enumerate_visible_entities`
    (`sync.rs:984-1010`).
  - Neither `grow_linear_catalogue` nor `imported_team_keys` has a test.
  - A reported item's `planned.id` is the local work-item id for tracked
    items (`work/src/sync/plan.rs:129`). Only `CreateFromRemote` entries
    carry the Linear identifier there. The external ids live on the corpus
    that `run_sync` loads (`sync.rs:105-125`).
- **`ClientError::NoTeam` is raised for every flow.** `resolve_credentials`
  (`auth.rs:80-90`) raises it inside `client_or_report()` when neither
  `linear.team_id` nor the catalogue names a team. `for_client` maps it to
  `CREATE_NO_CATALOGUE` (105) without knowing the flow.
- **`FailureClass`** (`apply.rs:26-29`) is `Retryable` or `Terminal`.
  `render_report` and `exit_code_for_report` (`work-cli/src/sync.rs:200-260`)
  match on it.
- **Today's writers sort every key.** `grow_catalogue` re-serialises through
  `serde_json::Value`, whose maps are `BTreeMap`s, so every nested key is
  alphabetical. It appends new `teams` entries at the tail, and
  deduplicates by key only.
- **`teams` is the reconcile list, and 0229 limits it to synced teams.**
  - `grow_linear_catalogue` adds a team only when items were imported from it,
    "never the whole enumerated workspace", because the committed file is
    repo-wide (0229 plan, §3).
  - `fetch_all` (`client.rs:~705-730`) pages the base team plus every `teams`
    entry on every sync. Adding a team to `teams` therefore adds a paged read
    to every later sync.
  - `teams` was added under a new key so that older binaries degrade
    gracefully; it never contains the base team.
- **`/team` has readers beyond the resolvers.** `auth.rs:126,130` falls back
  to `/team/key` and `/team/id` for credentials, and
  `linear-cli/src/main.rs:464` reads `/team/key` for the key write-back.
- **Today's state entries carry `type` and `position`**
  (`discovery.rs:152-159`), and `TEAM_STATES` excludes archived states.
- **The catalogue write paths.**
  - `write_catalogue` (`cache.rs:113`) overwrites the whole file.
  - `grow_catalogue` (`cache.rs:132-150`) falls back to `{}` when it cannot
    parse the file.
  - `Filesystem::read` returns `Option<String>` (`cache.rs:50,261-263`). Its
    other callers are `ensure_scaffold` (`:190`) and
    `SystemFilesystem::append_line` (`:291`).
  - The implementors are `SystemFilesystem` and `FakeFs`
    (`tests/cache.rs:42`).
  - `for_cache` (`exit_codes.rs:115-120`) is an exhaustive `const fn`.
  - `serde_json` has no `preserve_order`.
- **`paginate_teams` is unbounded and crate-private** (`discovery.rs:74`).
- **`issue(id:)` accepts an identifier.** `SHOW` (`client.rs:58-63`) sends
  identifiers such as `PP-869`, so selecting `team { id key name }` on it
  gives the issue's current team. Every existing Linear query is anonymous.
- **Exit codes.**
  - `captured-exit-codes.txt` is pinned at 56 entries.
  - The search block 75–79 is full.
  - `SEARCH_NO_CATALOGUE` (77) is unmapped.
- **Test harnesses.**
  - `cli_sync.rs` scrubs credentials.
  - `sync_run_real_client.rs` drives a real `LinearClient` against
    `MockServer`, which routes by method and path only. Every Linear
    operation posts to `/graphql`, so multi-query tests rely on
    `Route::Sequence`, which serves by hit index and repeats its last entry.
  - `work-cli/tests/sync_resolves_real_client.rs` reaches `resolve_scope` with
    a dummy token.
  - `scenario_inventory.rs` pins 15 scenarios.
  - `support::seed_catalogue` seeds only `workflowStates`.
  - `linear-client` has no harness that captures log output.
  - The `linear-cli` flow tests load mocks through
    `cli_test_support::Scenario::install`, which keys on `(method, path)`.
  - `drive_sync` discards `run_sync`'s `ExitCode` (`sync.rs:2457-2465`).

## Desired End State

A Linear pull configured with `project`, `label`, `assignee` or `state`
filters emits an `IssueFilter` keyed entirely on resolved ids. Every refusal
reaches sync as `DiscoveryUnconfigured` (exit 74), under a
`pull filters could not be resolved:` header, with one indented line per value
and its remedy. Verify:

- **`project`.**
  - `project: [Alpha]` lowers to `project: { id: { eq: <alpha-uuid> } }`.
  - `[Alpha, Beta]` lowers to `{ id: { in: [...] } }`.
  - An active project named like archived ones resolves to the active one.
- **Completion.** Every value resolves against the scoped teams, for every
  shape including whole-workspace.
  - Each scoped team contributes its own state and label ids. Within one
    team, a single active match wins over archived ones, and two active
    matches are ambiguous.
  - A workspace label contributes its id whenever any team is in scope.
  - `assignee` resolves over the scoped teams' members, and `project` over
    their projects, each deduplicated by id.
- **Unknown values.** Every value that matches nothing in the scoped teams is
  refused (`E_SEARCH_UNKNOWN_*`), even when other values of its family
  resolve, and no issues are paged.
- **Live fetch.** Scoped teams whose entry lacks a section the configured
  families need are fetched in one batch, for those sections only.
- **Self-heal.** At the end of an apply-mode sync, every synced team's entry is
  complete: synced teams the catalogue does not name gain an entry, and
  incomplete entries are filled in. Teams fetched but not synced are never
  written. A preview writes nothing.
- **Pre-flight refusals.** On a base-only scope whose team entry covers the
  configured families, pre-flight refuses an unknown or ambiguous value of any
  family before any request.
- **Standalone `search`.** It is scoped to the catalogued init team.
  - With no team at all, client construction fails with exit 105, as today.
  - With `linear.team_id` configured but no catalogued base team, filter flags
    refuse with `E_SEARCH_NO_TEAM` (exit 77), and a text-only search runs
    workspace-wide, as it does today.
- **`project` validation.** `project` validates under Linear and fails at
  `configure` under Jira.
- **`init-linear`.** It sets `baseTeam`, writes complete entries for the base
  team and every existing `teams` entry, refreshes the workspace `labels`, and
  refuses to merge onto an unparseable catalogue. Archived states, labels and
  projects, and disabled members, are included.
- **Writers.** Init and sync finalisation share one write path. It replaces
  each section an update carries and keeps the sections it does not, keeps
  entries and records sorted by id, rewrites the legacy projections, keeps
  unknown keys at every level, and emits every key in alphabetical order.
- **Compatibility window.** For one minor release, `team` and `workflowStates`
  are written as projections of the base entry, and readers fall back to them
  when `baseTeam` is absent.
- **Unchanged.** Team enumeration stays unbounded. Transition keeps its
  init-team states and exit codes 122/123.

## What We're NOT Doing

- **Jira.** No Jira `project` filter, and no change to Jira lowering.
- **More filter syntax.** No negated filters (0293), no nested AND/OR, and no
  raw escape hatch.
- **Cataloguing teams that are not synced.** A team the credential can see, or
  a team in scope with no imported items, is fetched for the run but never
  written.
- **Refreshing complete entries during a pull.** A complete entry is not
  refreshed by sync: states, labels, members or projects added later to a
  synced team need `init-linear`, which refreshes every synced team. The
  configure docs say so.
- **Pruning `teams`.** A team whose items are all gone stays catalogued.
- **Assignees outside the scoped teams.** An assignee must be a member of a
  team in scope. There is no workspace-user fallback. The CHANGELOG records
  this as a break.
- **Value-targeted lookups for uncovered teams.** Uncovered teams are fetched
  by section. Lookups by value are the fallback design, adopted only if the
  spike breaches its budget.
- **A dedicated "refresh workspace sections only" action.**
- **Choosing between same-named projects in config.** No id or qualifier
  syntax; logging each name→id resolution is not in scope either.
- **Jira guidance.** No Jira hint pointing to `additional_projects`.
- **Sync's refusal prefix.** Sync's `refused: discovery is unconfigured`
  prefix is unchanged.
- **Transition.** No cross-team state resolution for transition.
- **Also deferred:**
  - resolving values at config time;
  - checking that remote entities exist (0227);
  - a `catalogue.json` schema version;
  - a `--project` flag on `search`.

## Implementation Approach

A spike comes first, then four phases. Each phase can be merged on its own,
and no merge leaves a filter silently dropped or refuses a catalogue that
exists today. Every phase lists its failing tests first (red), then the
smallest production change that makes them pass (green). The red list includes
the existing tests each change breaks, and what each should assert afterwards.

0. **Spike.** Settles how team entries are fetched for many teams at once.
1. **The catalogue document and init.** Lands the converged shape, the legacy
   projections, the team-entry fetch, and the one strict write path.
2. **Resolvers.** Lands the pure resolution domain over team entries.
   Filtering keeps its current init-team state behaviour.
3. **Resolution, completion, live fetch and self-heal.**
   - Pre-flight validates, and fully resolves a base-only scope whose team
     entry covers the configured families.
   - `search` fetches the needed sections of uncovered scoped teams,
     completes every value over the scoped teams, and `compose` lowers ids
     only.
   - Apply-mode finalisation writes complete entries for synced teams.
   - This phase adds `TrackerError::Unconfigured`.
   - It is one phase so that no release refuses today's incomplete catalogues.
4. **Acceptance.** Flips `project` on in config validation.

The domain is modelled explicitly:

- **Filter vocabulary.**
  - `FilterFamily` (`State`, `Label`, `Assignee`, `Project`) names what
    varies. Every family resolves against the scoped teams.
  - `FilterKey` owns every scope-filter string: `Named` from config (`label`),
    and `ValidatedName` (`validated:label`) once pre-flight has accepted it.
- **The catalogue.**
  - `TeamEntry` is one team's `id`, `key`, `name` and its `TeamSections`
    (`states`, `labels`, `members`, `projects`, each optional).
  - `CatalogueSection` names the four sections plus `WorkspaceLabels`, and
    `SectionSet` is a set of them. `FilterFamily::sections()` gives the
    sections a family needs: `Label` needs `labels` and `WorkspaceLabels`.
  - An entry **covers** a `SectionSet` when it carries every per-team section
    in it (and, for `WorkspaceLabels`, the catalogue carries workspace
    labels). An entry is **complete** when it covers all sections. Completion
    asks about coverage; only healing asks about completeness.
  - A **synced team** is the base team or a team that owns a tracked work
    item. Only synced teams are written.
- **Resolvers and search stages.**
  - `TeamScopedResolver::resolve(value, teams)` returns a `Resolution` over
    the given teams. `NameResolver` stays for transition's init-team states,
    and returns a `SingleResolution` that holds at most one id.
  - Coverage is answered once, by `ResolverSet::covers(team, &SectionSet)`,
    which can report a damaged section.
  - The stages are named for what they guarantee: `ConfiguredSearch` (named
    pairs from config), `ValidatedSearch` (names that passed pre-flight) and
    `LowerableSearch` (ids). `complete_for_teams` drains a `ValidatedSearch`,
    and `compose` accepts only a `LowerableSearch`.
- **Values that are never empty.** One generic `NonEmpty<T>` backs
  `ResolvedIds` and `UnresolvedFilters`.
- **Seams.**
  - `TrackerError::Unconfigured` is the port's configuration-refusal channel
    for reads.
  - `TeamEntryFetch` is the single fetch of team sections, implemented by
    `LinearClient` and used by init, completion and healing.
  - `CatalogueHealing` owns the run's buffer of fetched entries and the
    healing policy. `work-cli` holds it behind a provider-neutral
    `RunFinaliser`.
- **The catalogue document.** `CatalogueDocument` owns the file's shape, and
  one parse function reads it under an explicit `Strictness`. Every key is
  written in alphabetical order.

---

## Prerequisite: team-section fetch spike

This spike runs before Phase 1 §2, and its outcome is recorded here before
that section is built.

**Question.** For a set of team ids, can each of the four sections be fetched
in one paginated pass, with each node attributed to its team?

| Section | Candidate batched query | Attribution |
|---|---|---|
| `states` | `workflowStates(filter: { team: { id: { in } } }, includeArchived: true)` | `team { id }` |
| `labels` | `issueLabels(filter: { team: { id: { in } } }, includeArchived: true)` | `team { id }` |
| `members` | `teamMemberships(…)` filtered by team, or `users(includeDisabled: true)` with nested `teams { nodes { id } }` | `team { id }`, or a nested connection |
| `projects` | `projects(filter: { accessibleTeams: … }, includeArchived: true)` with nested `teams { nodes { id } }` | nested connection |

**Also to settle:**

- whether nested connections need their own pagination, and how they behave
  at the ceiling;
- the complexity points per page for each query, at the chosen outer and
  nested `first:` sizes;
- `issueLabels(filter: { team: { null: true } })` for workspace labels;
- whether `CATALOGUE_PAGE_CEILING` bounds each team or the whole batch, and
  whether the `in` list is chunked to make it per team; the ceiling is then
  set to three times the adopted per-section page budget;
- the size of a complete catalogue for the live tenant.

**Fallback.** For a section with no batched query, fetch one
`team(id) { … }` per team, paged within the team.

**Exit criteria.** The spike passes only if every section meets both limits
below, measured against the live tenant and extrapolated to N teams:

| Limit | N = 50 | N = 200 |
|---|---|---|
| `states`, `labels`: requests per section, one fetch | ≤ 5 | ≤ 20 |
| `members`, `projects`: requests per 1,000 records | ≤ 10 | ≤ 10 |
| Complexity points per page | ≤ 2,500 | ≤ 2,500 |
| Complexity points per filtered pull | ≤ 50,000 | ≤ 100,000 |

`states` and `labels` are filtered by team, so their cost follows the team
count. `members` and `projects` attribute through nested team connections,
so their cost follows the workspace's user and project counts; the spike
measures them at the live tenant's counts. The per-pull total is compared
with Linear's hourly complexity allowance at one pull every 15 minutes, and
the result is recorded in the configure docs.

The per-team fallback is valid only for init and healing. On the pull path
it costs at least N requests, so a section that needs it there switches to
value-targeted lookups.

A section that breaches a limit switches uncovered, never-synced teams to
value-targeted lookups for that section: `users` filtered by email, name or
display name, or `projects` filtered by name, each bounded by the number of
configured values and attributed through `teams { nodes { id } }`. The spike
records that switch here, and Phase 3 gains the lookups before it is
built.

**Outputs.** The chosen query and page sizes per section, the request count
for N teams, the measured cost, the ceiling scope, and the catalogue size.
These feed Phase 1 §2 and the Performance section.

### Outcome (2026-09-24)

Measured against the live tenant: 8 visible teams, 68 states, 164 team
labels, 10 workspace labels, 16 users (63 memberships) and 39 projects. The
spike **passes** for every section, so the pull path keeps batched queries
and no value-targeted lookups are needed.

| Section | Query | Page size | Points per page | Attribution |
|---|---|---|---|---|
| identities | `teams(filter: { id: { in } }, includeArchived: true)` | 250 | 625 | the node itself |
| `states` | `workflowStates(filter: { team: { id: { in } } }, includeArchived: true)` | 250 | 725 | `team { id }` |
| `labels` | `issueLabels(filter: { team: { id: { in } } }, includeArchived: true)` | 250 | 675 | `team { id }` |
| workspace labels | `issueLabels(filter: { team: { null: true } }, includeArchived: true)` | 250 | 625 | none |
| `members` | `users(includeDisabled: true)` with `teams(first: 10)` | 100 | 1,570 | nested `teams` |
| `projects` | `projects(filter: { accessibleTeams: { some: { id: { in } } } }, includeArchived: true)` with `teams(first: 10)` | 100 | 1,550 | nested `teams` |

- **Cost follows `first:`, not the records returned.** Linear refuses any
  query over 10,000 points (`Query too complex`). A nested `teams(first: 50)`
  costs about 67 points per outer slot, which breaches the per-page limit at
  `first: 50`; `teams(first: 10)` costs about 15.6.
- **Members come from `users`, not `teamMemberships`.** `teamMemberships`
  is cheaper (976 points for 250 memberships), but it takes no filter and no
  `includeDisabled`. The tenant has no disabled members, so whether it
  returns them could not be observed. `users(includeDisabled: true)` states
  the guarantee explicitly. All three sources agreed on the tenant's 63
  team–member pairs.
- **Nested team connections fail loud.** No user is in more than 8 teams and
  no project in more than 2. A node whose nested `teams` reports
  `hasNextPage` fails the fetch with `CatalogueTruncated` naming the nested
  connection, rather than paging it.
- **Request counts**, extrapolated at the tenant's per-team averages (8.5
  states, 20.5 labels): `states` 2 requests at N = 50 and 7 at N = 200;
  `labels` 5 and 17. `members` and `projects` take 10 requests per 1,000
  records. All are within the exit criteria.
- **Cost per filtered pull** (every section fetched live, 1,000 users):
  about 26,000 points at N = 50 and about 49,000 at N = 200. Linear's
  allowance is 3,000,000 points an hour, so one pull every 15 minutes uses
  under 7% of it at N = 200.
- **Ceilings bound each section across the whole batch**, and the `in` list
  is not chunked. `states`, `labels`, workspace labels and the identity
  lookup allow 60 pages (three times the 20-page budget). `members` and
  `projects` allow 30 pages (three times the 10 pages budgeted per 1,000
  records). A ceiling of three times the tenant's one measured page would
  refuse any workspace past 300 users.
- **Catalogue size.** A complete catalogue for the 8 teams is about 330
  records.

---

## Phase 1: The catalogue document, init, and one strict write path

### Overview

This phase lands the converged `catalogue.json` shape, the legacy projections,
the team-entry fetch, and one strict write path that loses nothing it does not
own. Init writes complete entries for the base team and every existing
`teams` entry. Sync keeps growing `teams` as today, now through the same write
path. The work before Phase 3 still reads `/workflowStates`, so filtering is
unchanged.

### Tests first (red)

**Harness, `cli/http-test-support` and `cli/cli-test-support`:**

This lands first, because the multi-query tests below need it.

- **Operation names.** Every discovery and catalogue query this plan adds or
  touches is named: `TeamEnumeration`, `TeamIdentities`,
  `TeamOfIdentifier`, `TeamStates`, `TeamLabels`, `WorkspaceLabels`,
  `TeamMembers` and `TeamProjects`. Several share a root field (`teams`,
  `issueLabels`), so a root field cannot tell them apart.
- **Operation keys.** `RequestKey::graphql(operation)` keys a route on
  `POST /graphql` plus the operation name parsed from the body's `query`,
  falling back to the first root field for an anonymous query. Each
  operation gets its own `Route::Sequence`.
- **Coexistence.** A body is matched to an operation key first. When no
  operation key matches, the plain `POST /graphql` route (if registered)
  serves it, so the ~100 existing plain registrations keep working.
- **Recording.** Each request is recorded under its operation key and under
  the plain `POST /graphql` key, so existing plain-key `hits` assertions keep
  their meaning. A route's `Route::Sequence` advances only on the requests
  that route served, tracked apart from the recorded `hits`, so an
  operation-served request never moves a plain sequence on.
- **Unmatched operations.** A body that matches neither is answered with
  `UNMATCHED_STATUS` and recorded. `server.unmatched()` lists them. When a
  server that registered any operation key is dropped with unmatched
  operations, it panics naming them (unless the thread is already
  panicking), so no test depends on remembering an assertion.
- **Accessors.** The existing `hits` and `bodies` accept the new key, so they
  count and return one operation's requests.
- **Scenario loader.** `cli_test_support::Expectation` gains an optional
  `operation` field. `Scenario::install` maps it to `RequestKey::graphql`,
  and `body_expectations()` returns the same key `install` used. The
  `linear-cli` flow scenarios below use it.
- Tests:
  - `operations_are_served_their_own_sequences_whatever_the_order`
  - `two_operations_sharing_a_root_field_get_separate_sequences`
  - `an_anonymous_query_is_keyed_on_its_first_root_field`
  - `a_request_is_counted_under_its_operation_and_the_plain_key`
  - `a_plain_sequence_advances_only_on_requests_it_serves`
  - `a_plain_route_serves_bodies_no_operation_key_matches`
  - `dropping_a_server_with_unmatched_operations_panics_naming_them`
  - `a_scenario_operation_keys_both_its_route_and_its_body_expectation`

Every "only these sections" and "nothing fetched" test in this plan asserts
exact per-operation request counts through `hits(&RequestKey::graphql(…))`.

**New unit tests, `cli/linear-client/src/discovery.rs` (`#[cfg(test)]`):**

- `pagination_exceeding_its_ceiling_fails_loud`. Drives the crate-private
  `paginate` with `Ceiling::Bounded(2)` and a third page, and expects
  `SurfaceError::CatalogueTruncated` naming the connection.
- `pagination_past_its_deadline_is_retryable`. Expiry is a transient
  failure, never `CatalogueTruncated`.
- `one_deadline_spans_every_section_of_a_fetch`.
- `each_sections_ceiling_is_three_times_its_page_budget`.

**New tests, `cli/linear-client/tests/discovery.rs`:**

- `fetching_team_entries_pages_each_section_to_exhaustion`. Table-driven over
  the four sections. Each case scripts only that section's two pages.
  - Each case checks the sent query through its operation's `bodies`.
  - Each compares its entries against a new
    `tests/fixtures/team-entries.golden.json`.
- `each_section_query_sends_its_page_sizes`. Asserts the outer and nested
  `first:` values the spike chose.
- `fetching_team_entries_filters_by_the_requested_team_ids`. The query shape
  follows the spike's outcome.
- `fetching_team_entries_requests_archived_and_disabled_entities`.
- `fetching_only_the_requested_sections`. Asking for `states` and `labels`
  sends zero members or projects queries.
- `fetching_team_entries_threads_the_end_cursor`.
- `a_page_two_failure_fails_the_whole_fetch`.
- `a_team_returned_with_no_states_is_a_bad_response`. This replaces the
  `discover_team` guard of the same name.
- **Attribution.** For each section that attributes through a nested
  connection:
  - `a_node_shared_by_two_requested_teams_appears_in_both_entries`
  - `a_node_for_an_unrequested_team_is_dropped`
  - `a_nested_team_connection_past_one_page_*`, which pages or fails loud
    according to the spike's finding, and never truncates.
- **Per-team fallback**, only if the spike chooses it for a section: paging
  within a team, and one team's failure failing the whole fetch.
- `a_requested_team_absent_from_the_response_is_reported_unreturned`.
  Presence comes from the identity lookup, and the fetch returns the entries
  it got plus the ids it did not.
- `a_present_team_with_no_labels_or_projects_gets_empty_sections`. The team
  is not reported unreturned, and its sections are `Some([])`.
- `the_identity_lookup_pages_past_one_page_of_ids`. More requested ids than
  one page of 250.
- `the_identity_lookup_includes_archived_teams`.
- `workspace_labels_are_a_section_of_the_same_fetch`. A `SectionSet` holding
  `WorkspaceLabels` sends one `WorkspaceLabels` operation under the fetch's
  deadline, and fills `SectionFetch::workspace_labels`.
- `team_enumeration_stays_unbounded`.

**New tests, `cli/linear-client/tests/cache.rs`:**

- **Section-level merge.**
  - `recording_replaces_each_carried_section_and_keeps_the_rest`.
  - `two_entries_with_one_id_in_an_update_merge_section_by_section`. When
    both carry a section, the later one wins.
  - `a_renamed_team_key_replaces_the_stored_key`.
  - `a_replaced_section_keeps_each_records_stored_extra`.
  - `a_new_entry_is_inserted_in_id_order_and_others_are_unchanged`.
- **Projections and the base team.**
  - `recording_writes_the_legacy_projections_from_the_base_entry`. `team` is
    `{id, key, name}`, and `workflowStates` is the base entry's non-archived
    states as `{id, name, type, position}`.
  - `recording_onto_a_legacy_file_sets_base_team_from_the_legacy_team`.
  - `recording_never_re_points_an_existing_base_team`. Only an update that
    carries a new `baseTeam` (init) changes it.
- **Losslessness.**
  - `recording_onto_an_absent_catalogue_writes_every_key`.
  - `recording_preserves_unknown_keys_at_every_level`. Unknown top-level,
    entry-level and record-level keys survive a rewrite of their entry, and
    no section key is duplicated into `extra`.
  - `recording_normalises_null_sections`.
  - `the_written_file_matches_the_golden_byte_for_byte`. Every key is
    alphabetical, entries and records are in id order, and a re-write of an
    unchanged document is byte-identical. The golden holds an integer
    `position` (`1002`) and a fractional one (`1002.5`), both unchanged.
- **Refusals.**
  - `writers_refuse_an_unreadable_or_unparseable_catalogue`. Table-driven
    over merge-conflict markers, a non-object, a non-array `teams`, a
    `teams` entry that fails typed parsing, an entry with a blank id, and a
    `FakeFs` read error. Each is refused, naming the entry where there is one,
    and the file is left untouched.
- **Legacy and mixed-version reads.**
  - `growing_a_team_changes_only_its_entry_and_the_projections`. Starts from
    a pre-upgrade file with a legacy `team`, `workflowStates` and a 0229
    `teams` entry. The new entry is added in id order, and nothing else
    changes.
  - `a_legacy_catalogue_reads_as_incomplete_entries`. `team` plus
    `workflowStates` reads as a base entry with `states` only. A 0229 `teams`
    entry reads as an entry with no sections.
  - `a_teams_entry_matching_the_legacy_team_is_the_base_entry`. Legacy
    `workflowStates` fills only a missing `states` section.
  - `unsorted_teams_written_by_an_older_binary_read_and_re_sort`.
  - `duplicate_team_ids_merge_section_by_section_on_read`.
- **Scaffold upkeep.**
  - `scaffold_upkeep_stays_lenient_about_an_unreadable_gitignore`.
    `ensure_scaffold` warns and skips its appends; `record_team_entries`
    returns `Ok`, and the catalogue holds the new entries.
  - `appending_to_an_unreadable_gitignore_refuses_rather_than_overwriting`.

**New tests, `cli/linear-cli/tests/flow_init.rs`:**

- `init_discover_writes_complete_entries_for_base_and_synced_teams`. The
  seeded file has a 0229 `teams` entry. After init, `baseTeam` is set, both
  entries are complete, and top-level `labels` is written. Stdout carries the
  team shape plus per-team section counts.
- `init_discover_never_catalogues_other_visible_teams`. The mock serves a
  third visible team, and it is absent from the file.
- `init_discover_writes_nothing_when_a_fetch_fails`.
- `init_discover_refuses_a_damaged_catalogue_naming_the_recovery`. The
  message leads with restoring the last good version from VCS.
- `init_discover_notes_a_legacy_file_without_teams`. A file with only `team`
  and `workflowStates` prints a `note:` that synced teams may have been
  erased by an older binary, naming the recovery.

**New tests, `cli/linear-client/tests/auth.rs`:**

- `the_credential_team_falls_back_to_the_base_entry_then_the_legacy_team`.

**New unit tests, `cli/work-cli/src/sync.rs` (`#[cfg(test)]`):**

`grow_linear_catalogue` has no tests today, so this pins its behaviour before
the rewire. It uses a tempdir catalogue, a `RunReport` with applied
`CreateFromRemote` items, and a `RemoteTracker` double whose
`enumerate_visible_entities` names the team.

- `growing_catalogues_a_team_imported_from_for_the_first_time`. The new
  `{id, key, name}` entry is written, and the `note:` names its key.
- `growing_ignores_a_team_already_catalogued`.

**New unit tests, `cli/linear-cli/src/exit_codes.rs`:**

- `catalogue_truncation_maps_to_error`
- `an_unparseable_catalogue_maps_to_error`

**Existing tests to update:**

- `flow_init::init_discover_persists_the_catalogue`. Its `team-states-200`
  scenario gains the team-entry routes. It keeps its legacy-key assertions,
  which now hold through the projections.
- `scenario_inventory.rs`. Raise the scenario count.
- The two `write_catalogue` tests and the `grow_catalogue` tests. Move them
  onto `record_team_entries`.
- `FakeFs` in `tests/cache.rs:42`. Implement the widened `Filesystem::read`.
- **Retired with `discover_team`.**
  - `discover_team_queries_the_states_and_matches_the_catalogue_golden`
    becomes `fetching_team_entries_pages_each_section_to_exhaustion`.
  - `catalogue.golden.json` becomes `team-entries.golden.json`.
  - `a_team_with_no_states_is_a_bad_response` moves onto the new fetch.

### Production changes (green)

#### 1. The catalogue document

**File**: `cli/linear-client/src/catalogue.rs`

```rust
pub struct CatalogueDocument {
    base_team: Option<String>,
    labels: Option<Vec<CataloguedLabel>>,
    teams: Vec<TeamEntry>,
    legacy_base: Option<LegacyBase>,
    extra: Map<String, Value>,
}

pub enum Strictness {
    Forgiving,
    Strict,
}

pub enum CatalogueParseError {
    NotAnObject,
    TeamsNotAnArray,
    Entry { index: usize, id: Option<String>, reason: String },
}

impl CatalogueDocument {
    pub fn parse(text: &str, strictness: Strictness) -> Result<Self, CatalogueParseError>;
    pub fn base_entry(&self) -> Option<&TeamEntry>;
    pub fn to_json(&self) -> String;
}
```

- **One parse, two policies.** `Strict` (the writer) refuses any `teams`
  entry that fails typed parsing, including one with a blank id, as
  `CatalogueParseError::Entry`, which the writer maps to
  `CacheError::Unparseable` naming the entry. `Forgiving` (the resolvers)
  maps the same error to `CatalogueGap::Damaged` for that entry and keeps the
  rest.
- **`base_entry()` is the only legacy interpretation.** `auth.rs`,
  `report_team_key_writeback`, `Catalogue::base_team_id()` and `TeamStates`
  all read through it, so 0294 deletes the legacy read in one place.
- **Projections are not stored.** `team` and `workflowStates` are derived
  from `base_entry()` inside `to_json`.
- **Byte-stable output.** `to_json` serialises through `serde_json::Value`,
  so every key at every level is alphabetical. That matches what pre-0292
  binaries write, so mixed versions do not churn the key order.

The shape it writes:

```jsonc
{
  "baseTeam": "<uuid>",
  "labels": [{ "archivedAt": null, "id": "…", "name": "Security" }],
  "team": { "id": "…", "key": "PP", "name": "Product Pod" },
  "teams": [
    {
      "id": "…", "key": "PP",
      "labels": [{ "archivedAt": null, "id": "…", "name": "Bug" }],
      "members": [{ "active": true, "displayName": "…", "email": "…", "id": "…", "name": "…" }],
      "name": "Product Pod",
      "projects": [{ "archivedAt": null, "id": "…", "name": "Alpha" }],
      "states": [{ "archivedAt": null, "id": "…", "name": "In Review", "position": 1002, "type": "started" }]
    }
  ],
  "workflowStates": [{ "id": "…", "name": "In Review", "position": 1002, "type": "started" }]
}
```

- **`teams` holds the base team too.** `baseTeam` points into it.
- **Records are nested per team.** A user or project shared by several teams
  appears in each entry. Readers deduplicate by id.
- **`team` and `workflowStates` are projections.** Every write derives them
  from the base entry: `team` is `{id, key, name}`, and `workflowStates` is
  the non-archived states in today's shape. They exist for one minor release
  so older binaries and the `auth.rs` fallback keep working. Work item 0294
  deletes them and the legacy read.
- **Reading a legacy file.** With no `baseTeam`, the base team id is
  `/team/id`.
  - A `teams` entry with that id is the base entry. Legacy `workflowStates`
    fills its `states` only when that section is missing.
  - With no such entry, the base entry is derived from `team` plus
    `workflowStates`, with `states` only.
  - A 0229 `teams` entry of `{key, id, name}` reads as an entry with no
    sections.
  - All of these are incomplete, and Phase 3 heals them.
- **Older writers.** A pre-0292 grow appends unsorted entries and can add a
  second entry with an existing id. Readers sort entries by id, and merge
  same-id entries section by section.

The typed entry types derive both `Serialize` and `Deserialize`, and the
fetch, the writer and the resolvers share them. Each carries a
`#[serde(flatten)] extra: Map<String, Value>`, so fields written by a later
version survive a rewrite:

| Type | Fields |
|---|---|
| `TeamEntry` | `id`, `key`, `name`, `states`, `labels`, `members`, `projects`, `extra` |
| `CataloguedState` | `id`, `name`, `type`, `position: Number`, `archived_at`, `extra` |
| `CataloguedLabel` | `id`, `name`, `archived_at`, `extra` |
| `CataloguedMember` | `id`, `name`, `display_name`, `email`, `active`, `extra` |
| `CataloguedProject` | `id`, `name`, `archived_at`, `extra` |

- **Sections are direct fields.** `TeamEntry` declares its four sections
  (each `Option<Vec<_>>`) as named fields, and exposes them through a
  `sections()` view. A second flattened struct would make serde copy every
  section key into `extra` as well.
- **`position` is a `serde_json::Number`.** Linear types it as `Float`, and
  today's file holds integers such as `1002`. `Number` keeps both exactly;
  `f64` would rewrite `1002` as `1002.0`.

#### 2. The team-entry fetch

**File**: `cli/linear-client/src/discovery.rs`

`fetch_team_entries(team_ids, sections: SectionSet)` returns a
`SectionFetch`. Each section uses the query the spike chose. States, labels
and projects pass `includeArchived: true`, and members pass
`includeDisabled: true`.

- **Team presence** comes from one identity lookup, `TeamIdentities`:
  `teams(filter: { id: { in } }, includeArchived: true) { id key name }`,
  paged through `paginate` with `first: 250`, never from section nodes. A
  requested id the lookup does not return is `unreturned`.
- **Empty sections.** A present team whose requested section has no nodes
  gets `Some(vec![])`, which covers that section. Only `states` must be
  non-empty; an empty `states` is a bad response.
- A team in `unreturned` gets no entry, partial or otherwise.

- **Workspace labels** are a section of the same fetch: when the
  `SectionSet` holds `WorkspaceLabels`, a `WorkspaceLabels` pass fetches
  labels with no team, under the same deadline.

`LinearClient` implements `TeamEntryFetch` over this function, so init,
completion and healing share one fetch. Phase 3 adds the second method,
`team_of_identifier`, with its first caller:

```rust
pub struct SectionFetch {
    pub entries: Vec<TeamEntry>,
    pub workspace_labels: Option<Vec<CataloguedLabel>>,
    pub unreturned: Vec<String>,
}

pub trait TeamEntryFetch {
    fn fetch_team_entries(
        &self,
        team_ids: &[String],
        sections: &SectionSet,
    ) -> Result<SectionFetch, SurfaceError>;
}
```

`paginate(query, variables, connection, page: PageSize, ceiling: Ceiling)`
generalises `paginate_teams`:

- `PageSize` carries the outer `first:` and any nested `first:` for a section,
  as the spike chose them.
- Team enumeration passes `Ceiling::Unlimited`.
- **Ceilings per section.** `CatalogueSection::page_ceiling()` gives each
  section its own ceiling, at three times that section's budgeted pages:
  - `states`, `labels`, `WorkspaceLabels` and the identity lookup: three
    times the team-scaled page budget (60 pages at the proposed 20);
  - `members`, `projects`: three times the pages the spike measured for the
    live tenant's record counts, so they grow with records, not teams.
- **Deadline.** Each `fetch_team_entries` call creates one `Deadline` from
  the transport config and shares it across the identity lookup, every
  section pass and the workspace-labels pass. It is separate from
  `page_all`'s. Expiry is a transient `SurfaceError`, which maps to
  `TrackerError::Retryable`, never to `CatalogueTruncated`.
- Going past a ceiling returns `SurfaceError::CatalogueTruncated` naming the
  section, which maps to `ERROR`. For `states` and `labels` the remedy is to
  narrow the scope; for `members` and `projects`, which are not filtered by
  team, it says the workspace has outgrown the catalogue's record budget.

`TEAM_STATES` and `discover_team` are deleted.

#### 3. One strict, lossless write path

**File**: `cli/linear-client/src/cache.rs`

- **`Filesystem::read`.** Widened to `Result<Option<String>, CacheError>`,
  where `Ok(None)` means only "not found".
  - `ensure_scaffold` runs before the catalogue write. When the `.gitignore`
    cannot be read, it warns and skips its appends, so it stays as lenient as
    today and never fails a write that went ahead.
  - `append_line` treats only `Ok(None)` as empty and propagates `Err`, so it
    never overwrites a `.gitignore` it could not read.
- **`LinearCache::load_for_update()`.** Reads through `self.fs` and parses
  with `Strictness::Strict`. An absent file gives `Ok(default)`. A file that
  is not a JSON object, whose `teams` is not an array, or with a `teams` entry
  that fails typed parsing, gives `CacheError::Unparseable { path, entry }`.
- **`record_team_entries(update: CatalogueUpdate)`.** The one writer, under
  the shared lock. `CatalogueUpdate` carries an optional new `baseTeam`, the
  entries to record, and optional workspace labels.
  - **Merge rule.** Entries in the update that share an id are first merged
    section by section; when two carry the same section, the later one wins.
    Each resulting entry then merges into the stored entry with the same id:
    - a section the entry carries replaces the stored section, and a section
      it does not carry is kept;
    - `key` and `name` take the update's values, so a key renamed in Linear
      reaches the catalogue;
    - entry-level `extra` is the union of both, with the update winning a
      shared key;
    - in a replaced section, each record keeps the stored record's `extra`
      for the same record id, under the fetched record's known fields.

    An entry with a new id is inserted in id order. Within a section, records
    are sorted by id. On read, same-id entries merge by the same rule, in
    document order.
  - **Base team.** A new `baseTeam` is set only when the update carries one,
    which only init does. When the stored document has none, the writer sets
    it from the legacy `/team/id`. Sync never re-points it.
  - The projections are rewritten from the base entry.
  - Unknown keys are kept at every level.
- **Deleted.** `write_catalogue` and `grow_catalogue`.
- **`for_cache`.** Maps `Unparseable` to `ERROR`. The message, in this order:
  1. Restore the last good `catalogue.json` from version control, or resolve
     the merge conflict.
  2. As a last resort, delete it and re-run init. Synced teams are then
     re-derived from tracked work items on the next apply sync.

#### 4. Init and sync growth move onto the one path

**Files**: `cli/linear-cli/src/main.rs`, `cli/work-cli/src/sync.rs`,
`cli/linear-client/src/auth.rs`

- **Init.** `init discover --team-id <id>` fetches complete entries for `<id>`
  plus every team already in `teams`, and the workspace labels. It writes them
  in one `record_team_entries`, with `baseTeam` set to `<id>`. Nothing is
  written unless every fetch succeeds. `discovered_summary` prints the team
  shape plus per-team section counts.
- **Sync growth.** `grow_linear_catalogue` records `{id, key, name}` entries
  through `record_team_entries`, which keeps the existing behaviour. Phase 3
  replaces it.
- **Credentials.** `auth.rs` and `report_team_key_writeback` read
  `CatalogueDocument::base_entry()`. `auth.rs` stops reading the file through
  its own `catalogue_field`.
- **A legacy file at init.** When the file has `team` but no `teams`, init
  prints a `note:` that an older binary may have erased the synced teams.
  The note names the recovery: restore from version control, or let the next
  apply sync re-derive them.

#### 5. Documentation

- **`skills/integrations/linear/init-linear/SKILL.md`.**
  - The description and Step 4 report per-team section counts.
  - Step 3 describes the shape: the base team and every synced team, never
    other visible teams.
  - It explains how to recover a damaged file: restore the last good version
    from version control first, and delete it only as a last resort.
  - It explains how to recover a file an older binary overwrote.
  - It says the catalogue is committed and shared: pull a teammate's refresh
    first, otherwise run `accelerator linear init discover --team-id <uuid>`
    and commit the result.
- **`CHANGELOG.md` `[Unreleased]`.**
  - `Changed`: `catalogue.json` records each synced team's states, labels,
    members and projects, plus workspace labels. `team` and `workflowStates`
    remain for one minor release. Init refuses a damaged catalogue.
  - `Security`: `catalogue.json` now records the name, display name and email
    of every member of a synced team, and it is committed to the repo.

### Success Criteria

#### Automated Verification

- [x] Format and lint clean: `mise run cli:check`
- [x] Per-operation routes and scenarios: `cargo test -p http-test-support -p cli-test-support`
- [x] Discovery tests pass: `cargo test -p linear-client --test discovery` and `cargo test -p linear-client --lib`
- [x] Cache and credential tests pass: `cargo test -p linear-client --test cache --test auth`
- [x] Init flow, scenario inventory and exit-code mapping pass: `cargo test -p linear-cli`
- [x] Sync growth pinned and unchanged: `cargo test -p work-cli`
- [x] Read-only CI mirror green: `mise run check`

#### Manual Verification

- [x] `init-linear` against the live tenant writes complete entries for the
      base team and every existing `teams` entry, and no other team.
- [x] A pre-0292 binary still reads the refreshed file: `linear search
      --state` and `transition` work.

---

## Phase 2: The resolution domain

### Overview

This phase adds the resolution domain types and backs every family with a
resolver over team entries:

- `FilterFamily`
- `NonEmpty<T>` and `ResolvedIds`
- `NameResolver` and `SingleResolution`; `TeamScopedResolver` and
  `Resolution`
- `CatalogueSection` and `SectionSet`
- `ResolverSet`, including `covers`, `is_complete` and `with_fetched`

`StateResolver` retires. Transition moves onto a `NameResolver` over the base
entry's states. `work-cli`'s grow path moves onto `Catalogue`. Until Phase 3,
`compose` resolves `state` through `team_states`, so today's init-team
behaviour holds.

### Tests first (red)

**New tests, `cli/linear-client/tests/catalogue.rs`:**

- **Resolution over the given teams.**
  - `a_state_name_resolves_to_each_given_teams_ids`
  - `a_state_name_no_given_team_carries_is_not_found`
  - `a_team_label_resolves_only_for_its_team`
  - `a_workspace_label_resolves_whenever_any_team_is_given`
  - `a_member_of_any_given_team_resolves_once_however_many_teams_share_them`
  - `a_project_linked_to_several_given_teams_resolves_once`
- **States and labels within one team.** The same active-wins rule as
  projects, applied per team; ids are then collected across teams.
  - `an_active_state_wins_over_an_archived_state_of_the_same_name`
  - `a_single_archived_state_resolves`
  - `two_active_labels_of_one_name_in_one_team_are_ambiguous`, naming the
    team.
  - `same_named_labels_in_different_teams_both_contribute`
  - `a_workspace_label_and_a_team_label_are_separate_groups`
- **Projects.** Resolves a unique name. An unknown name is `NotFound`. The
  active project wins over archived ones. `a_single_archived_project_resolves`.
  Two active projects are ambiguous, listing the candidates. A name shared
  only by archived projects is ambiguous.
- **Members.** Match on email first, then full name, then display name.
  - Within a tier, one active match wins over any number of disabled ones.
  - With no active match, exactly one disabled match resolves.
  - Two active matches, or two disabled and no active, are ambiguous.
  - An ambiguous full name refuses even when the display name is unique.
  - `an_earlier_tier_match_wins_over_another_users_later_tier_match`. A value
    that is one user's full name and another user's display name resolves to
    the first.
  - `a_former_member_is_not_found`. A user absent from every given team's
    members is `NotFound`, even when a workspace user matches.
  - When one user appears in several entries with different `active` values,
    the record from a fetched entry wins over a stored one.
  - A value that matches nothing is `NotFound`.
- **Coverage and completeness.**
  - `an_entry_covers_exactly_the_sections_it_carries`. Table-driven over
    section sets.
  - `a_legacy_base_entry_covers_states_only`
  - `a_0229_teams_entry_covers_nothing`
  - `the_label_family_needs_workspace_labels_too`
  - `is_complete_is_true_exactly_for_entries_with_all_four_sections`
  - `covers_reports_a_damaged_teams_section`
  - `folded_entries_resolve_like_catalogued_ones`. The same mixed-case value
    resolves identically for a catalogued entry and one folded in by
    `with_fetched`.
  - `folding_only_the_needed_sections_covers_the_family`. An entry with
    `states` only, folded with nothing, covers `State` and not `Label`.
  - `fetched_workspace_labels_fold_in`.
- **Normalisation and absence.**
  - Matching is Unicode case-insensitive.
  - An empty email or display name never matches, and a blank value never
    resolves.
  - An entry with a blank id is ignored.
  - A malformed entry is `NotCatalogued { Damaged }` and leaves the other
    entries intact.
- **Team states (transition).** A single-id resolve over the base entry's
  non-archived states. An ambiguous resolve. A legacy file resolves through
  `workflowStates`. No base team is `NotCatalogued`.
- **Loading.**
  - `resolvers_loaded_once_keep_resolving_after_the_file_is_removed`
  - `a_damaged_members_section_does_not_prevent_state_resolution`
- **Team identity.** One structure owns it.
  - `a_team_key_resolves_through_the_entries`
  - `every_key_in_scope_has_its_id_among_the_teams_paged`. Table-driven over
    legacy, 0229, converged, partly damaged and blank-id files. Every key the
    reconcile read treats as in scope has its id among the ids `fetch_all`
    pages.
- **`NonEmpty`.**
  - `a_non_empty_collection_cannot_be_built_empty`
  - `resolved_ids_reject_a_blank_id`

**New tests, `cli/linear-client/tests/transition.rs`:**

- `a_state_absent_from_the_team_refuses_as_not_in_catalogue`. Table-driven over
  `NotFound` and `NotCatalogued`.
- `an_ambiguous_state_refuses_naming_the_count`.
- `transition_resolves_against_the_base_team_only`.
- `an_archived_state_is_never_a_transition_target`.

**Existing tests to update:**

- **`CatalogueTeam` cases.** Move them to
  `Catalogue::from_document(…).resolver_set().team_by_key` with unchanged
  outcomes.
  The cases are `catalogue_team_*`, the absent-catalogue case, the grown case,
  the `catalogued` case, and the pre-upgrade case. Include the one at
  `tests/cache.rs:272`.
- **`CatalogueStates` cases.** These become cases for the team-states
  `NameResolver`. An absent catalogue now reports `NotCatalogued`.
- **`AmbiguousStates` double** (`tests/transition.rs`). It becomes a
  `NameResolver`.
- **`FixedStates` sites.** Move to `FixedNames` in a `ResolverSet`:
  `tests/support/client.rs`, `tests/contract.rs`, `tests/port.rs`,
  `tests/filter.rs` and `work-adapters/tests/sync_run_real_client.rs`.

### Production changes (green)

#### 1. Domain types

**File**: `cli/linear-client/src/filter.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FilterFamily {
    State,
    Project,
    Label,
    Assignee,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmpty<T> {
    first: T,
    rest: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedIds(NonEmpty<String>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CatalogueSection {
    States,
    Labels,
    Members,
    Projects,
    WorkspaceLabels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unresolved {
    NotFound,
    NotCatalogued { section: CatalogueSection, cause: CatalogueGap },
    TeamUnfetched { team: TeamRef },
    AmbiguousMember { tier: MatchTier, candidates: Vec<Candidate> },
    AmbiguousRecord { team: Option<TeamRef>, candidates: Vec<Candidate> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Resolved(ResolvedIds),
    Unresolved(Unresolved),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SingleResolution {
    Resolved(String),
    Unresolved(NameUnresolved),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameUnresolved {
    NotFound,
    NotCatalogued(CatalogueGap),
    Ambiguous { count: usize },
}

pub trait NameResolver {
    fn resolve(&self, value: &str) -> SingleResolution;
}

pub trait TeamScopedResolver {
    fn resolve(&self, value: &str, teams: &[TeamRef]) -> Resolution;
}

pub struct ResolverSet {
    team_states: Box<dyn NameResolver>,
    entries: Box<dyn TeamEntries>,
}
```

- **Fields are private.** A `ResolverSet` is built through its constructor and
  read through accessors.
- **`TeamEntries`** holds the parsed entries and the workspace labels, and is
  the one owner of team identity. It yields one `TeamScopedResolver` per
  family (`for_family(FilterFamily)`), and answers `team_by_key`, `covers` and
  `is_complete`. One instance answers "which team?", "which ids?" and "is
  this team covered?", so a test double cannot make them disagree.
  `TeamResolver` becomes a view over `team_by_key`.
- **Active-wins, per group.** A group is one team's records, or the
  workspace labels. Within a group:
  - one active match wins over any number of archived or disabled ones;
  - with no active match, exactly one archived or disabled match resolves;
  - two active matches, or two inactive and none active, are ambiguous.
- **Per-family rules over the given teams:**
  - `State`: the active-wins match of each given team, collected across
    teams.
  - `Label`: the active-wins match of each given team, plus the active-wins
    match among the workspace labels, collected across groups.
  - `Assignee`: the members of the given teams, deduplicated by id, through
    the tiers and active-wins. One id at most. When the same user appears in
    several entries, a fetched record wins over a stored one.
  - `Project`: the projects of the given teams, deduplicated by id, through
    active-wins. One id at most.
- **Coverage.** `covers(team, &SectionSet)` is `Ok(true)` when the entry
  carries every per-team section in the set and, for `WorkspaceLabels`, the
  catalogue carries workspace labels. `is_complete(team)` asks about all
  sections. Both return `Err(CatalogueGap::Damaged)` when `teams` cannot be
  read.
- **`CatalogueGap`** is `Absent` (no base team, or no catalogue) or `Damaged`
  (a section that cannot be parsed). A missing section of an entry is not a
  gap: the entry just does not cover it.
- **Folding in fetched data.** `ResolverSet::with_fetched(&LiveCatalogueData)`
  takes the fetched entries and any fetched workspace labels.
  `LiveCatalogueData { entries, workspace_labels }` is what a run fetched
  live; it is what `complete_scope` folds in and what the backfill holds.
  `with_fetched` returns a set in which each fetched section replaces the
  stored one, using the same types and normalisation. Every scoped team then
  resolves through one path.
- **`NonEmpty<T>`.** Its fields are private. It is built from `first` plus
  `rest` and exposes `iter()` and `len()`. `ResolvedIds` adds a check that
  rejects blank ids.
- **`TeamRef { id, key: Option<String> }`.** Names a team in refusals and in
  notes. The key comes from the entry, or from a fetched `team { key }`.
- **`NameResolver` contract.** `SingleResolution` cannot carry more than one
  id, and `NameUnresolved` carries only the failures a single-team name can
  have, so transition needs no defensive or unreachable arm.
- **Deleted.** `StateResolver`, `FixedStates` and the `lib.rs:26-27`
  re-exports. The suites' doubles are `FixedNames` and `FixedTeamEntries`.

#### 2. Catalogue-backed resolvers

**File**: `cli/linear-client/src/catalogue.rs`

`Catalogue::load(integrations_root)` reads the file once and is forgiving
about problems:

- An absent file yields an empty document.
- An unparseable file yields one marked `Damaged`.
- A legacy file reads as incomplete entries, as in Phase 1 §1.

`resolver_set()` builds the resolvers over the document. Each family's index
is built inside a `OnceCell`, the first time it resolves anything. Indices are
`HashMap`s keyed on the trimmed, Unicode lower-cased value, mapping to
`(team id, record)` pairs. Blank keys are skipped.

- **`TeamStates`** reads `base_entry()`'s non-archived states. One match
  resolves; more are ambiguous.
- `Catalogue::catalogued_teams()` lists every entry's `(key, id)`.
- `Catalogue::base_team_id()` returns `base_entry()`'s id.
- The module doc is rewritten.

#### 3. Client, transition, interim `compose`, and `work-cli` grow

- **Client construction.** `LinearClient::new` takes a `ResolverSet`.
  `from_config` and `build_with_override` build it from one
  `Catalogue::load`. `states()` becomes `resolvers()`.
- **`resolve_state`.** It matches exhaustively on `team_states`:
  - `Resolved(id)` is `Ok`;
  - `NotFound` or `NotCatalogued` is `UnknownState`;
  - `Ambiguous { count }` is `AmbiguousState { count }`.
- **Interim `compose`.** It takes `&ResolverSet` and resolves `state` through
  `team_states` with today's exactly-one rule.
- **`grow_linear_catalogue`.** It reads
  `Catalogue::load(..).catalogued_teams()`.
- **Reconcile read.** `fetch_all` and `in_scope` read team identity from
  `TeamEntries`. The base team is now inside `teams`, so a new `port.rs` case,
  `fetch_all_pages_the_base_team_once_when_it_is_catalogued`, builds a client
  from a new-shape catalogue with one extra team and asserts exactly two
  issue-search requests.
- **Test helper.** `client_with_resolvers(ResolverSet)` replaces direct calls
  to `LinearClient::new` in the `linear-client` suites.

### Success Criteria

#### Automated Verification

- [ ] Format and lint clean: `mise run cli:check`
- [ ] Resolver tests pass: `cargo test -p linear-client --test catalogue`
- [ ] Transition keeps 122/123: `cargo test -p linear-client --test transition` and `cargo test -p linear-cli --test flow_transition`
- [ ] Every touched crate passes: `cargo test -p linear-client -p linear-cli -p work-adapters -p work-cli`
- [ ] Read-only CI mirror green: `mise run check`

#### Manual Verification

- [ ] None: pure logic, covered by unit tests.

---

## Phase 3: Resolution, completion, live fetch and self-heal

### Overview

- **Pre-flight.** `resolve_scope` validates every filter's structure and
  carries each value forward as a `ValidatedName` pair. When the scope is
  base-only and the entry of the team it resolved covers the configured
  families, it also resolves every value over that team, and refuses before
  any request.
- **Live fetch.** `search` derives the `SectionSet` the configured families
  need, and partitions the scoped teams by `covers(team, &needed)`. When a
  filter is configured and a scoped team is uncovered, it fetches the needed
  sections for every uncovered team in one batch:

  | Family | Sections fetched |
  |---|---|
  | `state` | `states` |
  | `label` | `labels`, plus workspace `labels` if the catalogue lacks them |
  | `assignee` | `members` |
  | `project` | `projects` |

- **Completion.** `search` folds the fetched sections in, then finishes every
  value over the scoped teams via `complete_for_teams`, which checks the same
  `covers(team, &needed)`. That produces a `LowerableSearch`, which `compose`
  lowers.
- **Self-heal.** At the end of an apply-mode sync that was not push-only,
  `CatalogueHealing` records complete entries for every synced team that
  needs one: each synced team the catalogue does not name, and each existing
  entry that is incomplete. Synced teams are the base team plus the team-key
  prefixes of the Linear identifiers in the corpus (each tracked item's
  `external_id`) and of this run's applied imports. Local work-item ids are
  never read. Entries fetched during the run are reused, and only the
  missing sections are fetched. Teams that are not synced are never
  written.
- **A new port variant.** `TrackerError::Unconfigured` gives `search` a way to
  refuse on configuration.
- **Broadened scopes.** A broadened scope passes through `resolve_scope`
  before entity resolution.
- **The standalone `search`.** It is scoped to the catalogued base team, and
  never writes.

Phases 3 and 4 of the earlier draft are merged here. Shipping the refusal of
uncovered teams without the live fetch would refuse every catalogue that
exists today.

### Tests first (red)

**New tests, `cli/tracker/tests/errors.rs` and the `tracker` public-API snapshot:**

- `an_unconfigured_read_carries_its_detail_and_displays_as_a_configuration_fault`
- **The frozen oracle** in `errors.rs`:
  - `recorded_codes()` records `E_DISPATCH_UNCONFIGURED` (74) as
    `Class("Unconfigured")`, reachable from both the selection layer and the
    port.
  - `exactly_two_dispatch_codes_reach_the_port` becomes
    `exactly_three_dispatch_codes_reach_the_port`.
  - `each_class_routes_to_a_distinct_outcome` (`:125-131`) gains the new arm.
- **The snapshot.** `tracker/tests/fixtures/public-api.txt` gains
  `TrackerError::Unconfigured` and its `detail` field. This is a deliberate
  snapshot update.

**New unit tests, `cli/work-cli/src/exit_codes.rs`:**

- `an_unconfigured_tracker_error_exits_74`

**New unit tests on the write paths:**

One test proves the defensive mapping on each kind of write path:

- **`cli/work-adapters/src/sync/apply.rs`** (existing classification tests):
  `an_unconfigured_error_is_classed_unconfigured`, carrying its detail.
- **`cli/work-cli/src/update.rs`**:
  `an_unconfigured_push_keeps_the_baseline_and_exits_74`.

**New tests, `cli/linear-client/tests/filter.rs` and `tests/fixtures/issue-filter.txt`:**

- **Fixture.**
  - The `label`, `assignee` and `state` rows become id comparators.
  - New rows: `project`, `project-in`, `label-many-teams` and
    `state-many-teams`.
  - `FAMILIES` grows to six entries, and `parse_spec` gains `project`.
  - A `FixedTeamEntries` set supplies distinct id prefixes per family.
- `every_family_is_carried_as_validated_names`. The validated spelling is
  `validated:<family>`, which cannot be mistaken for a GraphQL path.
- `a_named_pair_in_a_validated_bag_is_refused` (`E_SEARCH_UNRESOLVED_SCOPE`)
- `a_blank_value_is_refused`
- `an_unrecognised_filter_key_is_refused`
- `resolution_reports_every_unresolved_value_at_once`
- `the_refusal_renders_a_header_then_one_indented_line_per_value`
- **Refusal cases.** One per family × reason. Each asserts its `E_*` code and
  remedy. Ambiguous projects list up to five candidates. An ambiguous state
  or label names its team. An assignee `NotFound` names membership of the
  scoped teams.
- `every_named_filter_key_is_accepted_by_linear_pull_validation`

**New tests, `cli/linear-client/tests/completion.rs`:**

- `each_scoped_team_contributes_its_own_ids`
- `a_workspace_label_contributes_its_id_once_however_many_teams_are_scoped`.
  Ids are deduplicated per family.
- `an_assignee_who_is_a_member_of_several_scoped_teams_resolves_once`
- `an_assignee_outside_every_scoped_team_is_not_found`
- `a_damaged_teams_section_refuses_as_damaged_before_completion`
- `a_value_no_scoped_team_carries_refuses_as_unknown`. The result is an
  `UnresolvedFilters` naming the value, never a `LowerableSearch` without
  that family.
- `an_unknown_value_refuses_even_when_its_familys_other_values_resolve`.
  `label: [bug, typo]` with only `bug` known refuses naming `typo`.
- `a_team_folded_with_only_the_needed_sections_completes`. A legacy base
  entry with `states` only, filtered on `state`, completes.
- `a_scoped_team_with_no_projects_does_not_refuse`. Its folded `projects` is
  `Some([])`, which covers `Project`; the value resolves through the other
  teams.
- `a_team_folded_without_a_needed_section_refuses_as_unfetched`. The
  defensive case: `TeamUnfetched`, naming the team by key.
- `a_value_ambiguous_within_one_scoped_team_refuses_naming_the_team`
- `a_value_served_only_on_a_second_page_resolves`. Built from the
  pagination golden.
- `a_search_without_filters_needs_no_coverage`
- `compose_accepts_only_a_lowerable_search`. This is a compile-time guarantee,
  exercised by building a `LowerableSearch` and checking its composed clauses.

**New tests, `cli/linear-client/tests/resolve_scope.rs`:**

- One test per scope shape:
  - `a_base_only_scope_resolves_its_team_and_filters`
  - `a_base_plus_additional_scope_leaves_entities_as_keys`
  - `an_additional_only_scope_needs_no_base`
  - `a_whole_workspace_scope_carries_validated_names`
- `a_value_the_covering_base_entry_lacks_refuses_before_any_request`.
  Table-driven over the four families, base-only.
- `a_base_entry_that_does_not_cover_the_families_defers_to_completion`
- `pre_flight_resolves_against_the_team_the_key_names`. With `team_key`
  naming a synced team other than `baseTeam`, pre-flight resolves over that
  team's entry.
- `a_broadened_scope_never_refuses_a_value_in_pre_flight`. A state carried only
  by a team outside the catalogue passes pre-flight.
- The remedy prints `--team-id <baseTeam>`, or `<uuid>` only when the
  catalogue has no base team.

**New tests, `cli/linear-client/tests/discovery.rs`:**

- `fetching_several_teams_makes_one_pass_per_section`. The request count
  matches the spike's outcome.
- `team_of_identifier_resolves_the_issues_current_team`. A table over an
  identifier whose team key was since renamed (the renamed team) and one
  from an archived team (that team).
- `an_identifier_linear_cannot_find_is_none`. The mock serves Linear's real
  response: HTTP 400, `errors[]` with "Entity not found", `data: null`.
- `any_other_lookup_failure_is_an_error`. A transport failure and a
  non-not-found `errors[]` are `Err`, never `None`.

**New tests, `cli/linear-client/tests/port.rs`:**

These use a recording `CatalogueBackfill`, and count requests per operation
through `RequestKey::graphql`.

- Rewrite `a_flat_filter_bag_groups_same_key_values_into_one_in_clause` to use
  a validated scope over complete entries.
- `search_completes_state_for_each_covered_scoped_team`. Two covered teams;
  the issues body carries both teams' state ids.
- `search_fetches_an_uncovered_team_and_includes_its_ids`
- `search_fetches_an_incomplete_synced_team_and_includes_its_ids`
- `covered_and_uncovered_teams_both_contribute`
- `search_fetches_only_the_sections_the_configured_families_need`. A
  `state` filter sends one `workflowStates` operation and zero of any other.
- `a_legacy_base_entry_filtered_on_state_needs_no_fetch`
- `nothing_is_fetched_when_every_scoped_team_is_covered`
- `nothing_is_fetched_without_filters`
- `search_refuses_a_family_left_with_no_ids_without_paging`. The result is
  `Err(TrackerError::Unconfigured)` and nothing is paged.
- `fetched_entries_are_held_exactly_once`
- `a_later_section_failure_holds_nothing`. One section's pass succeeds and
  the next fails. The result is `Retryable`, nothing is held and nothing is
  paged.
- `a_truncated_team_section_fetch_refuses_as_unconfigured`. The message names
  the connection and the page ceiling, and says the fix is to narrow the
  scope.
- `search_refuses_an_unresolved_scope`

**Harness changes, `cli/tracker-test-support/src/lib.rs`:**

- `RecordingTracker` records two new calls, `Call::ResolveScope { scope }` and
  `Call::EnumerateVisibleEntities`.
- A new builder, `rewriting_scope_filters()`, rewrites every filter value.
- A new builder, `refusing_search_as_unconfigured(detail)`, makes `search`
  refuse.
- A test pins the recording order: `records_resolve_then_enumerate_in_order`.

**New tests, `cli/work-adapters/tests/` (sync run suite):**

- `a_broadened_pull_searches_with_the_resolved_scope`
- `a_broadened_pull_resolves_before_enumerating`
- `a_broadened_scope_refusal_never_enumerates`
- `an_unconfigured_search_refuses_the_run_as_discovery_unconfigured`. The run
  returns `RunError::DiscoveryUnconfigured` carrying the detail, not
  `DiscoveryStatus::Failed`.

**New tests, `cli/work-adapters/tests/sync_run_real_client.rs`:**

These use a real client built from `Catalogue::load(tempdir).resolver_set()`.

- `a_linear_pull_with_an_unknown_label_refuses_before_any_request`. Base-only,
  complete base entry.
- `a_linear_pull_sends_seeded_label_assignee_state_and_project_ids`. No raw
  names appear, and there are no `name` comparators.
- `a_whole_workspace_state_pull_sends_every_scoped_teams_ids`
  - Per-operation routes serve the teams, the team-filtered states for the
    uncovered teams, and the issues.
  - The states operation is hit once, and the issues body carries every
    enumerated team's state id.
- `a_linear_pull_against_a_legacy_catalogue_fetches_and_succeeds`. The seeded
  file has only `team` and `workflowStates`.
- `a_later_pull_after_recording_makes_no_section_fetch`.
  1. The first run's held entries are recorded through
     `LinearCache::record_team_entries` into a tempdir.
  2. A fresh client is built from `Catalogue::load(tempdir).resolver_set()`,
     with a new `MockServer`.
  3. Every team-section operation has zero hits, and the issues body carries
     the recorded id.

**New tests, `cli/linear-client/tests/synced_teams.rs`:**

`SyncedTeams::derive` owns the Linear rule that an identifier's prefix names
a candidate team. A prefix is only a candidate: healing confirms each one
against Linear before it writes.

- `each_prefix_keeps_its_identifiers_in_sort_order`. Runs are deterministic,
  and healing can try the next identifier when one is not found.
- `local_work_item_ids_are_never_read`. The input type carries only
  `ExternalId`s; a table over numeric (`0292`) and `{project}-{number}`
  (`PROJ-0042`) local ids paired with Linear external ids (`PP-869`) yields
  only `PP`.
- `an_external_id_without_a_team_prefix_is_ignored`

**New tests, `cli/linear-client/tests/healing.rs`:**

The healing policy lives in `linear-client`, so it is tested there against a
`FakeFs` catalogue and a `TeamEntryFetch` double. `heal` takes a
`SyncedTeams`, never a report.

- `healing_with_nothing_to_heal_makes_no_request_and_no_write`. Every synced
  team is complete and workspace labels are present: the fetch factory is
  never called, no lock is taken, and the file is byte-identical.
- `healing_adds_the_base_team_from_the_catalogue`. The caller supplies no
  base key.
- `a_held_team_is_confirmed_before_its_sections_are_used`. The buffer holds
  a team whose key matches an unnamed prefix. `team_of_identifier` still
  runs; the held sections are reused only for the team it returns.
- `a_held_team_the_identifier_does_not_confirm_is_never_recorded`. A
  whole-workspace buffer holds team `PROJ`; the corpus's `PROJ-12` is not
  found on Linear. Nothing is recorded.
- `healing_catalogues_an_imported_team_that_was_never_fetched`. The buffer is
  empty. `team_of_identifier` confirms the team, and the entry is recorded
  complete. The outcome names the key.
- `healing_re_derives_synced_teams_erased_from_the_catalogue`. The file has
  only the legacy base, and the external ids name a second team, which gains
  a complete entry.
- `a_renamed_prefix_resolving_to_a_catalogued_team_records_nothing`. Tracked
  ids use `OLD-`, the team is now `NEW` and complete. The lookup confirms
  `NEW`, nothing is written, and the outcome names no failure.
- `a_moved_issue_does_not_hide_its_prefixes_team`. `ENG-3` was moved to
  `OPS`, and `ENG-4` still belongs to `ENG`. `ENG-3` confirms `OPS`, which
  owns a tracked item and is recorded. Its key differs from `ENG`, so
  `ENG-4` is tried next and confirms `ENG`, which is recorded too.
- `a_prefix_stops_after_three_identifiers`. Tracked `OLD-1`…`OLD-5` all
  resolve to renamed team `NEW`: three lookups, then `OLD` is left for the
  next heal.
- `a_not_found_identifier_falls_through_to_the_next`. `PP-1` was deleted;
  `PP-2` confirms `PP`.
- `a_prefix_linear_cannot_confirm_is_unconfirmed_and_never_written`. A
  Jira-style `PROJ-12`, with the fixture stating that no Linear issue
  `PROJ-12` exists, records nothing, and the outcome names `PROJ-12`.
- `a_colliding_foreign_id_is_indistinguishable_from_a_linear_id`. When
  Linear team `PROJ` does have issue 12, `PROJ` is confirmed and recorded.
  This pins the accepted residual risk.
- `healing_never_records_a_team_that_was_fetched_but_not_synced`
- `healing_reuses_held_sections_and_fetches_only_the_rest`
- `healing_completes_an_incomplete_entry_even_when_nothing_was_imported`
- `completeness_counts_stored_and_held_sections_together`. A stored entry
  with `states` and `labels` plus held `members` and `projects` is recorded
  complete without a fetch.
- `missing_workspace_labels_alone_trigger_a_labels_only_fetch`
- `healing_records_the_returned_teams_and_names_the_unreturned_ones`. The
  fetch omits one team. The others are recorded, the outcome names the
  unreturned key, and no partial entry is written.
- `a_failed_identifier_lookup_reports_its_prefixes_as_a_fetch_failure`. The
  teams whose ids are known still heal, and the prefixes are not reported as
  unconfirmed.
- `a_failed_section_fetch_still_records_entries_complete_from_held_sections`.
  The rest are named, and nothing partial is written.
- `a_fetch_that_cannot_be_built_records_held_complete_entries`. The factory
  fails; the outcome carries `fetch_unavailable` with its reason.
- `a_heal_with_nothing_changed_takes_no_lock_and_writes_nothing`. Every
  lookup failed, or every confirmed team was already complete, so
  `record_team_entries` is not called, even on a legacy file.
- `a_team_with_no_projects_heals_to_a_complete_entry`. Its `projects` is
  `Some([])`, and a later heal makes no request for it.
- `healing_sets_base_team_from_the_legacy_team_and_never_re_points_it`
- `a_failed_heal_write_reports_the_failure_and_writes_nothing`. A failing
  `Filesystem`.
- `a_damaged_entry_refuses_the_heal_naming_the_entry`. The file is
  byte-identical, and the outcome carries the `Unparseable` entry.

**New unit tests, `cli/work-cli/src/sync.rs` (`#[cfg(test)]`):**

The binary harness cannot reach a mock endpoint, so the finaliser is tested as
a unit, with an injected diagnostics writer.

- `finalising_the_linear_catalogue`. Table-driven over `RunMode` and every
  `DiscoveryStatus`, with a `CatalogueHealing` over a tempdir catalogue:

  | Mode | Discovery | Heals |
  |---|---|---|
  | apply | `Ran`, `TargetedPull`, `SkippedTargeted`, `Failed` | yes |
  | apply | `SkippedPushOnly` | no |
  | preview | any | no; file byte-identical, no diagnostics |

  When it heals, the `note:` names the recorded teams by key.
- **Diagnostics**, each asserted on the injected writer:
  - `unreturned_teams_warn_that_pulls_will_refetch_them`;
  - `unconfirmed_identifiers_warn_naming_them`, saying Linear does not
    recognise them as Linear issues and pointing at their `external_id`;
  - `a_failed_heal_fetch_warns_that_pulls_will_refetch`;
  - `a_missing_credential_warns_only_when_healing_is_needed`;
  - `a_damaged_catalogue_prints_the_restore_remedy_naming_the_entry`.
- `the_finaliser_receives_the_corpus_external_ids`. The corpus holds items
  with local ids `0292` and `PROJ-0042` and external ids `PP-869` and
  `ENG-12`. The recording `RunFinaliser` receives `PP-869` and `ENG-12`, and
  no local id.
- `an_applied_import_reaches_the_finaliser`. The `RecordingTracker` serves a
  remote-only `ENG-7`; the recording finaliser receives it alongside the
  corpus ids.
- `a_failed_import_adds_no_synced_team`. A `CreateFromRemote` that failed
  contributes no identifier.
- `render_report_renders_an_unconfigured_failure`.
- `a_failing_finaliser_leaves_the_exit_code_unchanged`. `drive_sync` is
  extended to return the `ExitCode` that `run_sync` produced.
- `exit_code_for_report_ranks_outcomes`. Table-driven over mixed outcomes:
  terminal (71) > awaiting a human (4) > unconfigured (74) > retryable (70).

**New unit tests, `cli/work-cli/src/tracker_registry.rs`:**

`ConfiguredTrackers` gains two crate-private test seams:
`with_environment(&'a dyn Environment)`, which replaces the hard-coded
`SystemEnvironment` (the field is an `Option`, defaulting at use, so `new`
stays `const`), and `with_linear_endpoint(Url)`, which points the Linear
clients it builds at a loopback server. The tests never depend on the
host's `ACCELERATOR_LINEAR_TOKEN`.

- `a_client_from_resolve_holds_into_the_buffer_healing_reads`. With a fake
  environment holding a token and a `MockServer` endpoint, a filtered
  `search` through the client `resolve("linear")` returns fetches an
  uncovered team's sections. `CatalogueHealing::heal` then records that
  team with zero further fetches.
- `without_a_token_the_team_entry_fetch_is_unconfigured`. With an empty fake
  environment, `linear_team_entry_fetch` fails as `SelectionError` naming the
  missing credential.

**New tests, `cli/work-cli/tests/sync_resolves_real_client.rs`:**

- `a_linear_unknown_label_filter_refuses_with_exit_74`. The same base-only
  seed as Phase 4's project test, over `label`. The output carries the header
  and `E_SEARCH_UNKNOWN_LABEL`.

**New tests, `cli/linear-client/tests/port.rs` (workspace labels):**

- `a_label_filter_fetches_workspace_labels_only_when_the_catalogue_lacks_them`.
  Table-driven over a catalogue with and without workspace labels, every team
  covered for `labels`: exactly one workspace-labels operation, then zero.

**New tests, `cli/linear-cli/src/exit_codes.rs` and `tests/exit_codes_parity.rs`:**

- `for_client_maps_unresolved_filters`:

  | Entries | Exit code |
  |---|---|
  | all `NotCatalogued` or `TeamUnfetched` | 77 |
  | otherwise, all `State` | 78 |
  | otherwise | 89 |

  `SurfaceError::SearchNeedsCatalogueTeam`, raised by `run_search`, maps to
  77.

- `no_team_still_exits_105_for_every_flow`. `ClientError::NoTeam` keeps its
  mapping to `CREATE_NO_CATALOGUE`, including for search when client
  construction fails.

- `the_search_unresolved_filter_code_is_89_and_uncontested`

**New tests, `cli/linear-cli/tests/flow_search.rs`:**

- `search_by_label_sends_label_ids`
- `search_by_state_stays_scoped_to_the_init_team`
- `search_by_unknown_assignee_refuses_without_a_request`
- `search_against_a_legacy_catalogue_fetches_and_never_writes`
- `search_by_state_without_a_catalogued_team_refuses_as_no_team`. With
  `linear.team_id` configured and no catalogued base team, exits 77.
- `a_text_only_search_without_a_catalogued_team_runs_workspace_wide`. This is
  the documented exception to single-team scoping.
- `search_without_any_team_exits_105`. Unchanged behaviour, pinned.

**Existing tests to update:**

- `filter.rs::an_unknown_state_is_refused_rather_than_filtered_literally`.
  Moves to pre-flight and keeps `E_SEARCH_UNKNOWN_STATE`.
- The team-key cases in `resolve_scope.rs` keep their assertions.
- `support::seed_catalogue` writes complete entries, and keeps the
  projections. A new `support::seed_legacy_catalogue` writes only `team` and
  `workflowStates`, and `flow_transition.rs` keeps one case over it for the
  compatibility window.
- `scenario_inventory.rs`. Raise the scenario count.
- The Phase 1 `growing_*` tests in `work-cli/src/sync.rs` become
  `healing_*` tests in `linear-client/tests/healing.rs`.
- Every exhaustive match on `TrackerError` gains an arm. All of them follow
  the rule in §1: no remote change happened, and the exit code is 74.
  - `apply.rs:52-53` classes it as a new `FailureClass::Unconfigured`, and
    `apply.rs:387-392` abandons the attempt, as for `Retryable`, because
    nothing was sent. `an_unconfigured_error_is_classed_unconfigured` pins
    both.
  - `work-cli/src/sync.rs`: `render_report` renders it as `unconfigured`.
    `exit_code_for_report` ranks it terminal (71) > awaiting a human (4) >
    unconfigured (74) > retryable (70), so conflicts are never masked.
  - `work-cli/src/update.rs:228-246`: keep the baseline, and report through
    `for_tracker_error` (74) with `into_detail`. No new identifier.
  - `work-cli/src/create.rs:416-421`: `tracker_error_detail` is deleted in
    favour of `TrackerError::into_detail`.
  - `work-cli/src/exit_codes.rs:88-93` maps the new variant to 74.
  - `tracker/tests/port.rs`, `tracker/tests/errors.rs:128-131` and the
    matches in `tracker-test-support` are updated.

### Production changes (green)

#### 1. The port's configuration-refusal channel

**File**: `cli/tracker/src/lib.rs`

```rust
pub enum TrackerError {
    Retryable { detail: String },
    Terminal { detail: String },
    Unconfigured { detail: String },
}
```

`Unconfigured` is the read-side counterpart to `ScopeError`. It covers a scope
that turns out, once its teams are known, to name no valid target.

The enum doc changes from "two classes, and closed" to three classes on two
axes. The first axis is whether a remote change may have applied; the second
is what clears the fault:

| Class | Remote change possible | Cleared by |
|---|---|---|
| `Retryable` | no | a retry |
| `Terminal` | yes | a human checking the remote |
| `Unconfigured` | no | a configuration change |

A search-specific error type was considered and rejected. `search` is the
port method `discover_untracked` already matches on, and a second error type
would duplicate the `Retryable` mapping there and in every double.

The rule for where it can appear:

- **Only `RemoteTracker::search` produces it.** The `# Errors` sections of
  `search`, `fetch_all`, `show` and `enumerate_visible_entities` change from
  "always `Retryable`" to name exactly which classes each can return.
- **Write paths treat it defensively.** They class it as
  `FailureClass::Unconfigured`, abandon the attempt as nothing was sent, keep
  the baseline, and exit 74.

Also updated:

- `into_detail` and `Display` gain arms.
- The docs for `RecordingTracker::failing_search`, the `jira-client`
  `failure.rs` module and the `work-cli/src/exit_codes.rs` taxonomy
  (`:34-67`) describe three classes.

This updates the `tracker` public-API snapshot, which is the plan's only port
change.

#### 2. One owner of the filter vocabulary

**File**: `cli/linear-client/src/filter.rs`

```rust
pub enum FilterKey {
    Named(FilterFamily),
    ValidatedName(FilterFamily),
    Text,
}

pub struct SearchScope {
    team_ids: Vec<String>,
    text: Option<String>,
}

pub struct ConfiguredSearch {
    scope: SearchScope,
    values: BTreeMap<FilterFamily, Vec<String>>,
}

pub struct ValidatedSearch {
    scope: SearchScope,
    names: BTreeMap<FilterFamily, NonEmpty<String>>,
}

pub struct LowerableSearch {
    scope: SearchScope,
    ids: BTreeMap<FilterFamily, ResolvedIds>,
}
```

`FilterKey::parse` and `FilterKey::as_str` are the only places that know the
field strings: `label`, `validated:label` and `text`. The validated spelling
exists because the port carries filters only as `(String, String)` pairs; the
rustdoc on `ValidatedSearch` says so.

- **`ConfiguredSearch::from_pairs`.** Reads the named pairs.
- **`preflight(&ConfiguredSearch, &ResolverSet, base_only_team)`.**
  - Refuses blank values and unknown keys.
  - `base_only_team` is the team `resolve_scope` resolved from `team_key`,
    not the `baseTeam` pointer. When it is present and its entry covers the
    configured families, every value is resolved over it and each failure is
    collected.
  - Otherwise carries every value forward unresolved.
  - Collects every problem into one `UnresolvedFilters`.
- **`ValidatedSearch::to_pairs` / `from_pairs`.** Round-trip `names`.
  `from_pairs` rejects named keys, blank values and unknown keys.
- **`complete_for_teams(validated, scoped_teams: &[TeamRef], &ResolverSet)`.**
  This is a pure function.
  - Checks `covers(team, &needed)` for each scoped team, where `needed` is
    the union of the configured families' sections. A damaged `teams`
    section is refused as damaged.
  - Refuses a team still uncovered after folding as
    `TeamUnfetched { team }`, which is the defensive case.
  - Resolves each value over the scoped teams through
    `for_family(family).resolve(value, scoped_teams)`. Ids are deduplicated
    per family through a `BTreeSet`.
  - Every value that resolves to `NotFound` is refused ("no team in scope
    carries …"), even when other values of its family resolve.
  - The result is a `LowerableSearch`, or `UnresolvedFilters`.
- **`compose(&LowerableSearch) -> Value`.** It cannot fail.
  `ignore_case_comparator` is deleted.

The doc comments are rewritten.

#### 3. Client: pre-flight in `resolve_scope`; fetch, fold and complete in `search`

**File**: `cli/linear-client/src/client.rs`, `cli/linear-cli/src/main.rs`

- **`resolve_scope`** runs `preflight` for every shape.
  - It substitutes the base team key only for `Keyed` with an empty
    `additional`.
  - The refusal remedy always names `Catalogue::base_team_id()`.
  - The stale inline comment is deleted.
- **`complete_scope`** is one private method that `search` and
  `search_detailed` share. It returns
  `Result<LowerableSearch, CompletionFailure>`, and each caller maps the
  failure to its own error type. Its steps:
  1. **Partition.** It splits the scoped teams into covered and uncovered,
     using `ResolverSet::covers(team, &needed)`.
  2. **Fetch.** If any filter is carried and any scoped team is uncovered, it
     calls `TeamEntryFetch::fetch_team_entries(uncovered, &needed)` once.
     `needed` keeps `WorkspaceLabels` only when the catalogue lacks them.
     The `SectionFetch` entries and labels become a `LiveCatalogueData`.
  3. **Hold.** Only after every pass succeeds does it call
     `CatalogueBackfill::hold`, exactly once.
  4. **Complete.** It calls `complete_for_teams` over
     `resolvers.with_fetched(&live)`. A team in `SectionFetch::unreturned`
     has no folded entry, so completion refuses it as `TeamUnfetched`,
     named by the key the scope gave it.
  - Each scoped team is a `TeamRef`, keyed from its entry or its fetched key.
- **`search`** runs `ValidatedSearch::from_pairs`, then `complete_scope`, then
  `compose`, and pages once.
  - An `UnresolvedFilters` becomes `TrackerError::Unconfigured { detail }`,
    before any issues request.
  - A network or GraphQL failure in any pass becomes
    `TrackerError::Retryable`, raised before any hold and any issues request.
    The engine reports it as `DiscoveryStatus::Failed`, as it does for
    enumeration failures.
  - `CatalogueTruncated` becomes `TrackerError::Unconfigured`. Its remedy
    names the connection and the ceiling, and says to narrow the scope; it
    never suggests a refresh, which would hit the same ceiling.
  - `page_all`/`fetch_page` take the composed `Value`.
  - `fetch_all` composes each team-only search once.
- **`search_detailed`** (standalone search) runs `complete_scope` against the
  base team, with `NoBackfill`. `run_search` sets `team_ids` from
  `Catalogue::base_team_id()`.
  - When client construction fails with `ClientError::NoTeam`, search exits
    105, as today.
  - When a client exists but there is no catalogued base team, any filter
    flag refuses with `SurfaceError::SearchNeedsCatalogueTeam`
    (`E_SEARCH_NO_TEAM`, exit 77), and a `--text`-only search runs
    workspace-wide, as today. The search skill documents that exception.

#### 4. Sync engine

**File**: `cli/work-adapters/src/sync/run.rs`, `cli/work-adapters/src/sync/scope.rs`

```rust
let prepared = ports.tracker.resolve_scope(&request.scope).map_err(
    |error| RunError::DiscoveryUnconfigured { detail: error.detail },
)?;
let resolved = if scope::is_broadened(&request.scope) {
    match scope::resolve_entities(ports.tracker, &prepared) {
        // ... unchanged arms
    }
} else {
    prepared
};
```

- **Discovery match.** It gains a `TrackerError::Unconfigured { detail }` arm,
  which returns `RunError::DiscoveryUnconfigured { detail }`.
- **Docs.** The docs for `scope.rs` and `RemoteTracker::resolve_scope` state
  the widened contract.

#### 5. The healing seam and apply-mode self-heal

**Files**: `cli/linear-client/src/healing.rs` (new),
`cli/linear-client/src/client.rs`, `cli/work-cli/src/tracker_registry.rs`,
`cli/work-cli/src/main.rs`, `cli/work-cli/src/sync.rs`,
`cli/linear-cli/src/context.rs`

The Linear healing rules live in `linear-client`. `work-cli` only chooses
when to run them, through a provider-neutral `RunFinaliser`.

- **The buffer.** `pub trait CatalogueBackfill: Send + Sync { fn hold(&self,
  live: LiveCatalogueData); }`. It has two implementations: `NoBackfill`,
  and the buffer inside `CatalogueHealing`.
- **`SyncedTeams`** (`linear-client`).
  `SyncedTeams::derive(ids: impl IntoIterator<Item = &ExternalId>)` is the
  one place that reads a Linear identifier's prefix. It groups the ids by
  prefix, each group in sort order. It takes only external ids, so a local
  work-item id cannot reach it. A prefix is a candidate, not a team: healing
  confirms it against Linear.
- **`team_of_identifier`** (added to `TeamEntryFetch`, `discovery.rs`). One
  `TeamOfIdentifier` request per identifier: `issue(id: $id) { team { id key
  name } }`, the same `issue(id:)` that `show` uses. It returns
  `Ok(Some(team))` with the issue's current team, so renamed keys and
  archived teams resolve. Linear answers an unknown identifier with HTTP 400,
  an "Entity not found" `errors[]` entry and `data: null`; that is
  `Ok(None)`. Any other failure is `Err`. One request per identifier, because
  `issue` is non-null and one unknown alias would null a batched response.
- **`FetchUnavailable { reason: String }`** (`linear-client`, beside
  `HealOutcome`). The finaliser's factory closure maps the registry's
  `SelectionError` into it, with the selection error's detail as the reason.
- **`CatalogueHealing`** (`linear-client`) owns the buffer and the policy:
  - `backfill(&self) -> Arc<dyn CatalogueBackfill>` hands the buffer to a
    client.
  - `heal(&self, synced: &SyncedTeams, fetch: &dyn Fn() ->
    Result<Box<dyn TeamEntryFetch>, FetchUnavailable>, cache: &LinearCache)
    -> HealOutcome` runs the steps below. It builds the fetch only once step
    1 has found work, and it adds the base team from the catalogue's
    `base_entry()` itself, so no caller supplies a base key.
  - `HealOutcome` names, each by key or identifier: the recorded teams, the
    `unreturned` teams (Linear returned no data), the `unconfirmed`
    prefixes with the identifiers Linear could not find, prefixes and teams
    hit by a fetch failure, a `fetch_unavailable` reason, and any write
    failure, including an `Unparseable` refusal with its entry.
- **Healing steps.** Throughout, an entry is complete when its stored
  sections merged with the held and fetched ones cover all four.
  1. **Find work.** The work is: each synced prefix that is not the current
     key of a catalogued entry; each catalogued entry that is not complete;
     and the workspace labels, if the catalogue lacks them. With no work,
     healing stops: the factory is not called, no lock is taken, and nothing
     is written.
  2. **Build the fetch.** It calls the factory. If that fails, it records
     only entries already complete from stored and held sections, and
     reports `fetch_unavailable`.
  3. **Confirm prefixes.** For each unnamed prefix it calls
     `team_of_identifier` on the prefix's identifiers in order, at most
     three per prefix per heal:
     - a found team is a synced team (it owns a tracked item) and joins the
       teams to record unless it is already complete;
     - the prefix stops when a found team's key equals it; a found team with
       another key (a moved issue, or a renamed team) moves on to the next
       identifier;
     - `None` moves on to the next identifier; a prefix whose tries all
       returned `None` is `unconfirmed` and never written;
     - `Err` reports that prefix under the fetch failure, not as
       unconfirmed, and the other prefixes and known teams still heal.

     A team in the buffer is never taken on key alone: its held sections are
     used only for a team this step, or the catalogue, has confirmed.
  4. **Fetch.** It takes the buffer, keeps the held sections for the teams to
     record, and fetches their missing sections in one `fetch_team_entries`
     call, with `WorkspaceLabels` in the set when the catalogue lacks them.
     If the fetch fails, it records only entries complete from stored and
     held sections, and names the rest.
  5. **Record.** The record set is the entries whose sections change in this
     heal, plus fetched workspace labels. When it is not empty, healing calls
     `LinearCache::record_team_entries` once; otherwise it takes no lock and
     writes nothing. An unreturned team gets no entry.
- **Accepted residual risk.** External ids carry no tracker tag. A leftover
  foreign id such as a Jira `PROJ-12` whose number also exists in a Linear
  team `PROJ` is indistinguishable from a real Linear id, so `PROJ` is
  confirmed and catalogued. The `note:` names every recorded team, and the
  Security note in the CHANGELOG says so.
- **The client.** `LinearClient::new` and `from_config` take an
  `Arc<dyn CatalogueBackfill>`:
  - `linear-cli`'s `context.rs:163,213` and `contract.rs:147` pass
    `Arc::new(NoBackfill)`.
  - `client_with_resolvers` defaults to `NoBackfill`.
  - `sync_run_real_client.rs:190` passes a recording buffer.
- **The registry.**
  - `ConfiguredTrackers` gains a field
    `healing: Option<Arc<CatalogueHealing>>`, so `new` stays `const` with
    `None`.
  - A new `with_catalogue_healing(Arc<CatalogueHealing>)` sets it.
  - `resolve(&self)` passes `healing.backfill()` into each `LinearClient`,
    falling back to `NoBackfill` when the field is `None`.
  - A new `linear_team_entry_fetch(&self)` builds a `LinearClient` as a
    `Box<dyn TeamEntryFetch>`, from the same credential context and transport
    config as `resolve`, with `NoBackfill`.
  - `resolve` builds every Linear client through one crate-private
    `linear_client(context)` seam.
  - Two crate-private test seams: `with_environment(&'a dyn Environment)`
    replaces the hard-coded `SystemEnvironment` (the field is an `Option`,
    defaulting to `SystemEnvironment` at use, so `new` stays `const`), and
    `with_linear_endpoint(Url)` points the Linear clients it builds at a
    loopback server.
- **`RunFinaliser`** (`work-cli`):
  `fn finalise(&self, run: &FinishedRun<'_>, diagnostics: &mut dyn Write)`.
  `FinishedRun` carries the `RunReport`, the `RunMode`, the corpus's
  external ids, and the ids of this run's applied `CreateFromRemote` items
  (whose `planned.id` is the external id). `run_sync` builds it from the
  corpus it already loads; for tracked items, `planned.id` is the local id
  and is never used.
  - `NoFinaliser` does nothing, for Jira.
  - `LinearCatalogueFinaliser` holds the `Arc<CatalogueHealing>` and a
    factory closure over `linear_team_entry_fetch`, which `heal` calls only
    when it has work. It does nothing in preview mode or when discovery is
    `SkippedPushOnly`. Otherwise it calls `SyncedTeams::derive` with the
    corpus external ids and the applied import ids, calls `heal`, and writes
    to `diagnostics`:
    - one `note: … version-controlled and repo-wide — commit it`, naming the
      recorded teams by key;
    - for unreturned teams, a failed fetch or an unavailable fetch,
      `warning: … each pull will fetch <keys> again until catalogue.json is
      updated and committed.`, with the reason for an unavailable fetch;
    - for unconfirmed identifiers, `warning: … Linear does not recognise
      <identifiers> as Linear issues; check their external_id`;
    - for an `Unparseable` refusal, the `for_cache` remedy naming the entry.
  - The exit code is unaffected.
- **Composition.**
  - `main.rs::run_sync` (`:434-457`) builds one `Arc<CatalogueHealing>`. It
    passes one clone to `ConfiguredTrackers::with_catalogue_healing` and
    wraps another in the `LinearCatalogueFinaliser`, so both ends share one
    buffer by construction.
  - `sync::run_sync` gains a `finaliser: &dyn RunFinaliser` parameter, which
    is not optional. It calls it after `render_report` on the `Ok(report)`
    branch, with stderr as `diagnostics`. Its two callers, `main.rs:451` and
    the test at `sync.rs:2464`, are updated, and `drive_sync` returns the
    `ExitCode`.
- **`linear_client(context)`** on `ConfiguredTrackers` is the one
  crate-private seam `resolve` and `linear_team_entry_fetch` build Linear
  clients through, applying the backfill and any test endpoint.
- **Deleted.** `grow_linear_catalogue` and `imported_team_keys`.

#### 6. Error variant, messages and exit codes

**File**: `cli/linear-client/src/error.rs`, `cli/linear-cli/src/exit_codes.rs`

`ClientError::UnknownState` is replaced by
`UnresolvedFilters { unresolved: NonEmpty<UnresolvedFilter> }`. It renders
under the `pull filters could not be resolved:` header, one indented line per
entry.

| Reason | Identifier | Remedy |
|---|---|---|
| `NotFound` | `E_SEARCH_UNKNOWN_{STATE,PROJECT,LABEL,ASSIGNEE}` | check the value. If it was added or renamed in Linear, pull the latest committed `catalogue.json` or refresh with `accelerator linear init discover --team-id <base team id>` and commit. For an assignee, the value must match a member of a team in scope |
| `NotCatalogued { Absent }` | `E_SEARCH_NO_TEAM` | there is no catalogue base team. Run `/accelerator:init-linear` |
| `NotCatalogued { Damaged }` | `E_SEARCH_CATALOGUE_DAMAGED` | `<section>` in `catalogue.json` cannot be read. Restore the last good version from version control, or resolve the merge conflict |
| `TeamUnfetched` | `E_SEARCH_TEAM_UNFETCHED` | team `<key>` was in scope but Linear returned no data for it. Check the credential's access |
| `AmbiguousMember` | `E_SEARCH_AMBIGUOUS_ASSIGNEE` | names the tier and count. Use the user's email |
| `AmbiguousRecord` (project) | `E_SEARCH_AMBIGUOUS_PROJECT` | lists up to five candidates. Rename one in Linear |
| `AmbiguousRecord` (state, label) | `E_SEARCH_AMBIGUOUS_{STATE,LABEL}` | names the team and count. Rename or archive one in Linear |

`for_client` maps refusals in this order:

1. every entry `NotCatalogued` or `TeamUnfetched` → 77;
2. otherwise, every entry `State` → 78;
3. otherwise → 89.

`for_client` stops being a `const fn`. The module doc records 89 as a
deliberate divergence, borrowed from the show flow's decade.

#### 7. Documentation

- **`search-linear-issues/SKILL.md`.**
  - The assignee tiers, and active-wins.
  - Search is scoped to the init team, for `--label` and `--assignee` as well
    as `--state`. With no catalogued team, a text-only search runs
    workspace-wide and filter flags refuse with 77.
  - The `E_SEARCH_*` refusals.
  - Exit codes 77, 78 and 89.
- **`sync-work-items/SKILL.md`.**
  - An exit 74 whose output carries the refusal header is a pull-filter
    refusal. Surface each line verbatim.
  - For a damaged catalogue, restore the last good version from version
    control, or resolve the conflict.
  - Never suggest `--push-only` for these refusals.
  - A per-item `unconfigured` detail in the report is a write the tracker
    refused on configuration. Other items may already have applied, so exit
    74 with that token is not a pre-flight refusal. It ranks below
    awaiting-human (4).
  - An apply sync whose pull ran may add or complete entries in
    `catalogue.json` for synced teams. The `note:` names them. Commit the file
    together with the pulled items. A preview or a `--push-only` run never
    writes it.
- **`configure/SKILL.md` "Pull filters".**
  - Values match per team in scope, and a workspace label matches everywhere.
  - An assignee must be a member of a team in scope. The assignee tiers.
  - Teams in scope that the catalogue does not cover are fetched on every
    pull. Only synced teams are committed.
  - Values added in Linear to a synced team need a catalogue refresh.
- **`CHANGELOG.md` `[Unreleased]`.**
  - `Changed`:
    - Filters now resolve to ids. An apply-mode sync may add or complete
      entries in `catalogue.json` for synced teams.
    - `accelerator linear search` scopes `--label` and `--assignee` to the
      init team, as `--state` already was. An unknown or ambiguous filter
      value now exits 89 instead of returning an empty result. `--state` with
      no catalogued team exits 77 instead of 78.
  - `Migrations`:
    - No manual step: the first apply-mode sync completes the catalogue.
      Commit the result.
    - A sync with a pull filter that cannot be resolved now refuses with exit
      74. Before, an unknown state exited 5 with "cut short… retry", and an
      unknown label or assignee was silently tolerated.
    - An `assignee` filter must now name a member of a team in scope. A user
      who has left the team, or who assigns work without being a member, now
      refuses the sync. Use a current member, or widen the scope.
  - `Security`: an apply sync may catalogue, and so commit the members of,
    any Linear team that owns a tracked item. A leftover `external_id` from
    another tracker whose identifier also exists in Linear is
    indistinguishable from a Linear id; the `note:` names every team
    recorded, so review it before committing.

### Success Criteria

#### Automated Verification

- [ ] Format and lint clean: `mise run cli:check`
- [ ] Port variant and snapshot: `cargo test -p tracker` and `mise run public-api:check`
- [ ] Vocabulary, fixture, completion, fetch, pre-flight and healing: `cargo test -p linear-client`
- [ ] Harness ordering: `cargo test -p tracker-test-support`
- [ ] Engine wiring, real-client sync and no-refetch: `cargo test -p work-adapters`
- [ ] Exit mappings, search scoping, finalisation and inventory: `cargo test -p linear-cli -p work-cli`
- [ ] Read-only CI mirror green: `mise run check`

#### Manual Verification

- [ ] Against a legacy catalogue, a filtered `--preview` pull succeeds and
      leaves `catalogue.json` unchanged. A second preview fetches again.
- [ ] An apply pull then completes the base entry and prints the `note:`. A
      third pull makes no team-section fetch.
- [ ] A whole-workspace pull filtering on `state` returns every team's
      matching issues, and commits entries only for synced teams.
- [ ] `accelerator linear search --state "In Progress"` returns only the init
      team's issues.

---

## Phase 4: Config acceptance of project under Linear

### Overview

Split the accepted filter keys per tracker, so that `project` is accepted
under Linear and rejected under Jira.

### Tests first (red)

**New tests, `cli/tracker-support/src/pull.rs`:**

- `a_project_filter_is_accepted_under_linear`
- `a_project_filter_is_rejected_under_jira_listing_the_jira_set`
- Extend `an_unsupported_filter_key_is_rejected` to cover both trackers.

**New tests, `cli/work-cli/tests/cli_sync.rs`:**

- `a_jira_project_filter_fails_loud_before_discovery`

**New tests, `cli/work-cli/tests/sync_resolves_real_client.rs`:**

- `a_linear_project_filter_reaches_resolution`
  - **Config and seed.** `linear.team_key` matches the seeded base entry's
    key. The seed has `baseTeam`, a complete base entry whose non-empty
    `projects` lacks `Nope`, and the projections. The scope is base-only, so
    pre-flight refuses before the dummy token is ever sent.
  - **Red.** The run exits 1 with `UnsupportedFilterKey`.
  - **Green.** The run exits 74. The output carries the header and
    `E_SEARCH_UNKNOWN_PROJECT`.

**Existing tests to update:**

- `every_named_filter_key_is_accepted_by_linear_pull_validation`. It goes red
  on `project` until the split lands.

### Production changes (green)

**File**: `cli/tracker-support/src/pull.rs`

```rust
const JIRA_FILTERS: FilterSchema = FilterSchema {
    accepted: &["label", "state", "assignee"],
};
const LINEAR_FILTERS: FilterSchema = FilterSchema {
    accepted: &["label", "state", "assignee", "project"],
};

impl Tracker {
    const fn filter_schema(self) -> FilterSchema {
        match self {
            Self::Jira => JIRA_FILTERS,
            Self::Linear => LINEAR_FILTERS,
        }
    }
}
```

- **The filter loop** (`:366-379`) reads `tracker.filter_schema().accepted`.
- **Deleted.** `FILTER_SCHEMA` and its doc.
- **Doc change.** The `FilterSchema` doc drops its hard-coded key list.
- **Configure docs.** "Pull filters" gains the per-tracker keys and the
  project rule.
- **CHANGELOG.** `[Unreleased]` `Added` gains the Linear `project` filter.

### Success Criteria

#### Automated Verification

- [ ] Format and lint clean: `mise run cli:check`
- [ ] Per-tracker validation: `cargo test -p tracker-support`
- [ ] Vocabulary link: `cargo test -p linear-client --test filter`
- [ ] Jira rejection and Linear end-to-end: `cargo test -p work-cli --test cli_sync --test sync_resolves_real_client`
- [ ] Public API unchanged since Phase 3: `mise run public-api:check`
- [ ] Read-only CI mirror green: `mise run check`
- [ ] Full local CI mirror green end-to-end: `mise run`

#### Manual Verification

- [ ] `configure` rejects a `project` filter under Jira.
- [ ] A live Linear pull with `project: [<real project>]` returns only that
      project's issues.

---

## Testing Strategy

### Unit Tests

- **Resolver domain.**
  - Resolution of every family over a given set of teams.
  - Workspace labels, section-aware coverage, and completeness.
  - Member tiers, and active-wins per group for every family.
  - The exactly-one rule for team states.
  - Legacy files read as incomplete entries.
  - `NonEmpty`, and one section's damage not affecting another.
- **Filter vocabulary and completion.**
  - Named and validated-name round-trips.
  - The drained `LowerableSearch`.
  - The refusal of every unknown value, per value.
  - The refusal layout, the golden fixture, and the link to pull validation.
- **Exit codes.** `for_client`, `for_surface`, `for_cache`, and the `work-cli`
  mapping of `TrackerError::Unconfigured`. The value 89 is pinned.
- **Port.** `TrackerError::Unconfigured` and the snapshot.
- **Synced teams** (`linear-client`). Derived from external ids only, never
  from local work-item ids.
- **Finalisation** (`work-cli`). Every `DiscoveryStatus` in apply mode heals
  except `SkippedPushOnly`, preview never does, the finaliser receives the
  corpus's external ids, and a failure warns without changing the exit code.
  `exit_code_for_report` ranks 71 > 4 > 74 > 70.
- **Registry wiring** (`work-cli`). A client from `resolve`, pointed at a
  mock endpoint with a fake environment, fills the buffer healing reads.

### Integration Tests

- **Harness.** Per-operation GraphQL routes in `http-test-support` and the
  `cli-test-support` scenario loader, with exact request counts and a
  recorded, asserted set of unmatched operations.
- **Discovery.**
  - Per-section pagination, cursors, page sizes, failures, deadline and
    ceiling.
  - Per-team attribution through nested connections.
  - Fetching only the requested sections, and reporting teams not returned.
  - Unbounded team enumeration.
- **Cache.** One writer: section-level merge, lossless at every level,
  byte-stable, id-ordered, projections derived, `baseTeam` never re-pointed,
  and refusing input it cannot read.
- **Healing** (`linear-client`). Synced teams only, including teams erased
  from the catalogue and teams never fetched. Nothing to heal costs nothing.
  Every prefix is confirmed, one identifier per request, from an issue's
  current team, so renamed keys, archived teams, moved issues, deleted
  issues and foreign ids are handled; a buffered team is never taken on key
  alone. Held sections are reused.
  Unreturned teams and unconfirmed identifiers are named, never partially
  written, and a failed lookup, fetch or fetch build still heals what it
  can. A heal with nothing complete writes nothing. A damaged entry refuses
  with its remedy.
- **Init.** Complete entries for the base team and every synced team, and
  never another visible team.
- **Pre-flight.** Full resolution on a base-only scope whose entry covers the
  families, against the team `team_key` names, with zero requests on
  refusal. Deferral otherwise.
- **Completion in `search`.** Covered teams, uncovered teams fetched for only
  their needed sections, unknown values, and fetch failure versus
  truncation.
- **Reconcile read.** The base team is paged once, and every in-scope key has
  its id paged.
- **Engine.**
  - Broadened ordering, and resolved-scope propagation.
  - `Unconfigured` is reported as `DiscoveryUnconfigured`.
- **Real-client sync.** Ids on the wire for every shape, including
  whole-workspace. Refusals. A legacy catalogue fetches and succeeds.
- **No refetch** (real-client): after the entries are recorded, a client
  rebuilt from the catalogue makes no team-section fetch.
- **Binary-level.** An unknown `label` exits 74 in Phase 3. `project`
  end-to-end, through config validation and `resolve_scope`, in Phase 4.
- **CLI flows.** Init, including refusing a damaged catalogue and noting a
  legacy file. Search scoped to the init team, including against a legacy
  catalogue, and exit 105 with no team. Transition, including over a legacy
  catalogue.

### Manual Testing Steps

1. Run `init-linear` on a repo with a grown `teams` array. Confirm that
   `baseTeam` is set, every existing team has a complete entry, no other
   visible team appears, and the projections match the base entry.
2. Run a pre-0292 binary against the refreshed file. `search --state` and
   `transition` still work.
3. On a legacy catalogue, run a filtered preview pull. It succeeds and the
   file is untouched. Run an apply pull. The base entry is completed and the
   `note:` is printed.
4. Run a whole-workspace pull with `state: [In Progress]`. Every team's
   matching issues come back. Only synced teams gain entries.
5. Delete a second team's entry from `teams` and run an apply sync. The
   entry is re-derived from tracked items.
6. Run `accelerator linear search --state "In Progress"`. Only the init
   team's issues come back.
7. Use an ambiguous `assignee`. The pull refuses, naming the tier.
8. Use `project` under Jira. `configure` rejects it.

## Performance Considerations

- ⏱️ **Team-entry fetch.** Request counts depend on the spike, which must
  meet its exit criteria: for `states` and `labels`, at most 5 requests per
  section for 50 teams and 20 for 200; for `members` and `projects`, at most
  10 requests per 1,000 records; 2,500 complexity points per page; and a
  per-pull total checked against Linear's hourly allowance. A section that
  breaches the budget switches to value-targeted lookups for never-synced
  teams.
- ⏱️ **Page sizes and the ceiling.** Each section passes explicit outer and
  nested `first:` sizes, so a page stays inside Linear's 10,000-point
  complexity cap and the transport's 8 MiB response bound. The ceiling is
  three times the page budget. One `Deadline` spans a whole
  `fetch_team_entries` call, separate from issue paging's, so the worst case
  before a filtered pull pages issues is one transport deadline (300s by
  default). Expiry is retryable.
- ⏱️ **Init.** It fetches every section for the base team and each synced
  team, plus workspace labels.
- ⏱️ **Pulls.**
  - A fetch happens only when a filter is configured and a scoped team is
    uncovered, and only for the sections the configured families need.
  - Teams in scope that are never synced are fetched on every such pull. This
    is the accepted cost of committing only synced teams, bounded by the
    spike's budget.
  - A preview repeats the fetch for incomplete synced teams until an apply
    pull completes them.
- ⏱️ **Finalisation.** Nothing at all when every synced team is complete and
  workspace labels are present. Otherwise at most three `TeamOfIdentifier`
  requests per unnamed prefix, one fetch of missing sections, and one locked
  write, at the end of an apply run that was not push-only, after paging.
  A prefix that is not a current catalogued key (a renamed team, moved
  issues, or an unconfirmed foreign id) costs up to three small requests on
  every apply sync while it stays in the corpus. This is the accepted cost of
  not storing key aliases.
- ⏱️ **Identity lookup.** Each `fetch_team_entries` adds one paged
  `TeamIdentities` request (250 teams per page), counted in the spike's
  request totals.
- ⏱️ **Catalogue size.** Members and projects are stored per team, so a
  record shared by k synced teams is written k times. The spike measures the
  size of a complete catalogue for the live tenant; if it is large, a
  follow-up stores records once at the top level with per-team id lists.
- **Resolution.**
  - Pre-flight and completion each run once per pull.
  - The file is parsed once, and each family is indexed lazily.
  - A lookup is a single `HashMap` probe.
  - `in` lists grow with the number of teams, not with catalogue size.

## Migration Notes

No manual migration is needed. An existing `catalogue.json` reads as
incomplete entries: the legacy base team has `states` only, and 0229 `teams`
entries have no sections.

- **Filtered pulls keep working.** Entries that do not cover the configured
  families are fetched live on each pull, for the sections the filters need.
  A legacy base entry already covers `state`.
- **The first apply-mode sync whose pull runs heals the file.** It completes
  every synced team's entry, re-derives synced teams from tracked items, and
  sets `baseTeam` from the legacy `team`. Commit the change. Everyone else
  pulls it.
- **`init-linear` still works as a refresh.** It is how values added in
  Linear to a synced team reach the catalogue.

Behaviour after the change:

- **Filter values.** They match per team in scope, and workspace labels match
  everywhere. A multi-team `state` filter now pulls every team's issues, which
  is a deliberate fix.
- **Standalone `search`.** It stays scoped to the init team, and never writes.
- **Assignees.** ⚠️ An assignee must now be a member of a team in scope;
  anyone else refuses the sync. Configs keep working when the value is unique
  at its tier among the scoped teams' members, with active members winning.
  The email always works for a member.
- **States and labels.** Within a team, an active state or label wins over
  archived ones of the same name. Two active ones of the same name refuse.
- **Projects.** A reused project name resolves to its only active project.
- **Damaged catalogues.** Init and sync refuse to overwrite a damaged
  catalogue. Restore the last good version from version control, or resolve
  the conflict. Deleting the file is a last resort: the next apply sync
  re-derives synced teams from tracked items, and init refreshes them.

- 🔒 **The committed catalogue records every member of a synced team,**
  including their email. It never records teams that are not synced.
- ⚠️ **The compatibility window.** For one minor release, `team` and
  `workflowStates` are written as projections of the base entry. Work item 0294
  deletes them and the legacy read. 0294 records the plugin version that
  ships this plan as its minimum: the projections are removed only once that
  version is the supported floor, because an older binary would otherwise
  fail with a misleading "cut short… retry" or exit 105.
- ⚠️ **Mixed plugin versions and shared checkouts.**
  - A pre-0292 `init discover` overwrites the file with `{team,
    workflowStates}`. The new binary falls back to the legacy base, and its
    next apply sync re-derives the other synced teams from tracked items.
    Restoring the previous committed `catalogue.json` from version control
    recovers them at once. Upgrade everyone before re-running init.
  - A committed `project` filter makes a pre-0292 sync fail. Adopt it only
    after the whole team has upgraded.
  - Entries are kept in id order and every key is alphabetical, so
    concurrent writes by upgraded binaries for different teams merge cleanly.
    A pre-0292 grow appends unsorted entries; the next new write re-sorts
    them, which shows as one reordering diff.
  - Resolve any conflicted `catalogue.json` before a pre-0292 binary runs
    sync. Its grow path overwrites an unparseable file.
  - CI pulls should commit the updated catalogue, or they will fetch on every
    run.

## References

- Original work item: `meta/work/0292-linear-project-pull-filter.md`
- Research: `meta/research/codebase/2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids.md`
- Review: `meta/reviews/plans/2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids-review-1.md`
- Review: `meta/reviews/plans/2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids-review-2.md`
- Prior art: `meta/plans/2026-09-11-0229-per-tracker-pull-scope-configuration.md`, `meta/work/0220-untracked-remote-discovery-never-runs-on-linear.md`
- Downstream: `meta/work/0227-accelerator-config-validate-command.md`, `meta/work/0293-negated-pull-filters.md`
- Follow-up: correct the Stories entry in parent `0146`.
- Follow-up: `meta/work/0294-remove-the-legacy-linear-catalogue-projections.md`
  removes the `team` and `workflowStates` projections and the legacy read one
  minor release after this ships.
- Key code:

  | File | Lines |
  |---|---|
  | `cli/tracker/src/lib.rs` | 145-215, 651-672 |
  | `cli/tracker/tests/fixtures/public-api.txt` | 77-86 |
  | `cli/tracker-support/src/pull.rs` | 272, 366 |
  | `cli/linear-client/src/filter.rs` | 108-156 |
  | `cli/linear-client/src/catalogue.rs` | 28-160 |
  | `cli/linear-client/src/cache.rs` | 40-60, 87-150, 190, 255-291 |
  | `cli/linear-client/src/discovery.rs` | 19-147 |
  | `cli/linear-client/src/client.rs` | 139-200, 286-400, 700-812 |
  | `cli/linear-client/src/transition.rs` | 27-40 |
  | `cli/linear-client/src/lib.rs` | 26-27 |
  | `cli/linear-cli/src/main.rs` | 103-116, 430-453 |
  | `cli/linear-cli/src/context.rs` | 163, 180-218 |
  | `cli/linear-cli/src/exit_codes.rs` | 55-60, 115-120, 161-173 |
  | `cli/work-adapters/src/sync/run.rs` | 546-551, 838-878, 1006-1008 |
  | `cli/work-adapters/src/sync/apply.rs` | 52-53, 387-392 |
  | `cli/work-cli/src/sync.rs` | 947-1030, 1039, 1305-1307, 1409, 1459 |
  | `cli/work-cli/src/tracker_registry.rs` | 134-142, 209-220 |
  | `cli/work-cli/src/exit_codes.rs` | 34-67, 81-93 |
  | `cli/work-cli/src/update.rs` | 228-246 |
  | `cli/work-cli/src/create.rs` | 416-421 |
  | `cli/work-cli/src/main.rs` | 434-457 |
  | `cli/tracker/tests/errors.rs` | 20-131 |
  | `cli/tracker-test-support/src/lib.rs` | 33, 429-445 |
  | `cli/http-test-support/src/lib.rs` | 31-45, 157-180 |
  | `cli/linear-client/src/auth.rs` | 80-130 |
  | `cli/work-adapters/src/sync/apply.rs` (`FailureClass`) | 26-29 |
