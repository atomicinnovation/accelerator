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
last_updated: "2026-09-23T16:53:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Linear Pull Filters via Catalogue-Resolved Ids Implementation Plan

## Overview

This plan adds a Linear `project` pull filter. It also converts `label`,
`assignee` and `state` into catalogue-resolved ids, so that every Linear filter
key emits `{id:…}` in the `IssueFilter`. It changes three things.

**Discovery.** Four workspace connections are added: `projects`, `issueLabels`,
`users` and `workflowStates` (every team's states). `init-linear` persists them
into `catalogue.json` by reading the file, merging, and writing it back. That
write keeps the grown `teams` array and never drops data it does not own.

**Resolution happens in two places, each with one job.**

- *Pre-flight* runs in the port's `resolve_scope` step, before any request.
  - **Project and assignee.** It resolves `project` and `assignee` to ids.
  - **State and label.** It only validates the names of the team-scoped
    families, `state` and `label`. It refuses a name that no catalogued team
    carries, and carries every name forward.
- *Completion* runs in `search`, once enumeration has produced the concrete
  team ids for every scope shape. It resolves each state and label name
  against each scoped team:
  - covered teams resolve from the catalogue;
  - teams the catalogue does not yet cover are fetched live in one batch.

**Refusals are always loud.** A family that ends with zero ids is refused as
a configuration fault. It never silently disappears from the filter. Teams
fetched during a pull are written to `catalogue.json` only when an apply-mode
sync finishes, never during `--preview`.

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
| Discovery | `linear-client/src/discovery.rs:19-147` | `TEAMS` unbounded; `TEAM_STATES` for the init team | four workspace connections; team-filtered backfill |

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
- **Exit codes.**
  - `captured-exit-codes.txt` is pinned at 56 entries.
  - The search block 75–79 is full.
  - `SEARCH_NO_CATALOGUE` (77) is unmapped.
- **Test harnesses.**
  - `cli_sync.rs` scrubs credentials.
  - `sync_run_real_client.rs` drives a real `LinearClient` against
    `MockServer`, which routes by method and path only.
  - `work-cli/tests/sync_resolves_real_client.rs` reaches `resolve_scope` with
    a dummy token.
  - `scenario_inventory.rs` pins 15 scenarios.
  - `support::seed_catalogue` seeds only `workflowStates`.
  - `linear-client` has no harness that captures log output.

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
- **`label` and `state` completion.** These resolve per scoped team, for every
  shape including whole-workspace.
  - Each covered team contributes its own ids.
  - A workspace label contributes its id whenever any team is in scope.
- **Zero ids.** A family whose names match no scoped team is refused
  (`E_SEARCH_UNKNOWN_{STATE,LABEL}`), and no issues are paged.
- **Backfill.** Scoped teams the catalogue does not cover are fetched in one
  batch. Their ids are used for the run, and they are written to
  `catalogue.json` at the end of an apply-mode sync, never during a preview.
- **Other refusals.** Pre-flight refuses these before any request:
  - an unknown or ambiguous `project` or `assignee`;
  - a state or label name that no catalogued team carries;
  - on a base-only scope, a name the covered base team lacks.
- **Standalone `search`.** It is scoped to the catalogued init team. With no
  catalogued team:
  - team-scoped flags refuse with `E_SEARCH_NO_TEAM` (exit 77);
  - a text-only search runs workspace-wide, as it does today.
- **`project` validation.** `project` validates under Linear and fails at
  `configure` under Jira.
- **`init-linear`.** It persists the four workspace sections, including
  archived and disabled entities, keeps `teams`, and refuses to merge onto an
  unparseable catalogue.
- **Writers.** Every writer keeps the sections it does not own semantically
  equal and emits known keys in alphabetical order.
- **Unchanged.** Team enumeration stays unbounded. Transition keeps its
  init-team states and exit codes 122/123.

## What We're NOT Doing

- **Jira.** No Jira `project` filter, and no change to Jira lowering.
- **More filter syntax.** No negated filters (0293), no nested AND/OR, and no
  raw escape hatch.
- **Refreshing `projects`/`users`/`issueLabels` during a pull.** Backfill adds
  only the states and team labels of scoped teams the catalogue does not cover.
- **Refreshing covered teams.** A covered team's snapshot is not refreshed:
  states or labels added later to an existing team need a catalogue refresh.
  The configure docs say so.
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

The work is split into five phases. Each can be merged on its own, and no
merge leaves a filter silently dropped. Every phase lists its failing tests
first (red), then the smallest production change that makes them pass
(green). The red list includes the existing tests each change breaks, and what
each should assert afterwards.

1. **Discovery.** Lands the workspace catalogue, with a strict write path that
   loses nothing it does not own.
2. **Resolvers.** Lands the pure resolution domain. Filtering keeps its current
   init-team state behaviour.
3. **Resolution, completion and lowering.**
   - Pre-flight resolves and validates the filters.
   - `search` completes team-scoped families per scoped team, and `compose`
     lowers ids only.
   - A scoped team the catalogue does not cover refuses loudly, as does a
     family left with zero ids.
   - This phase adds `TrackerError::Unconfigured`.
4. **Backfill.** Replaces the refusal for an uncovered team with a batched
   live fetch. The fetched sections are held in a buffer and written at the end
   of an apply-mode sync.
5. **Acceptance.** Flips `project` on in config validation.

The domain is modelled explicitly:

- **Filter vocabulary.**
  - `FilterFamily` names what varies. `TeamScopedFamily` (`State`, `Label`) is
    the subset resolved per team.
  - `FilterKey` owns every scope-filter string: `Named` from config, `Resolved`
    ids (`label.id`), and `ValidatedName` (`label.name`) for team-scoped
    families.
- **Resolvers and search stages.**
  - `NameResolver` returns a `Resolution`. `TeamScopedResolver` resolves one
    name for one team.
  - Coverage is answered once, by `ResolverSet::covers(team)`, which can
    report a damaged or absent section.
  - `ResolvedSearch` carries validated names. `complete_for_teams` drains
    them. `compose` accepts only the drained `LowerableSearch`.
- **Values that are never empty.** One generic `NonEmpty<T>` backs
  `ResolvedIds` and `UnresolvedFilters`.
- **Seams.**
  - `TrackerError::Unconfigured` is the port's configuration-refusal channel
    for reads.
  - `CatalogueBackfill` is the client's seam for writing fetched sections. The
    implementation is a no-op, or a buffer that `work-cli` flushes.
- **The catalogue document.** `CatalogueDocument` owns the file's shape. Its
  sections are raw JSON in alphabetical field order.

---

## Phase 1: Workspace discovery and a strict, lossless write path

### Overview

Fetch the workspace's `projects`, `issueLabels`, `users` and `workflowStates`,
and persist them on init through one strict write path. That path loses
nothing it does not own.

### Tests first (red)

**New unit tests, `cli/linear-client/src/discovery.rs` (`#[cfg(test)]`):**

- `pagination_exceeding_its_ceiling_fails_loud`. Drives the crate-private
  `paginate` with `Ceiling::Bounded(2)` and a third page, and expects
  `SurfaceError::CatalogueTruncated` naming the connection.
- `the_catalogue_page_ceiling_is_200`.

**New tests, `cli/linear-client/tests/discovery.rs`:**

- `discover_workspace_pages_projects_to_exhaustion`,
  `…_issue_labels_…`, `…_users_…` and `…_workflow_states_…`. Each is its own
  test, scripting only that connection's two pages. The other connections
  return a single empty page.
  - Each test checks the sent query's connection name via `server.bodies()`,
    so a change in request order fails with a clear message.
  - Each compares its section against a new
    `tests/fixtures/workspace.golden.json`.
- `discover_workspace_threads_the_end_cursor`.
- `discover_workspace_requests_archived_and_disabled_entities`.
  - Projects, labels and states carry `includeArchived: true`, and users carry
    `includeDisabled: true`.
  - Projects select `archivedAt`. States and labels select `team { id }`.
- `a_page_two_failure_fails_the_whole_workspace_fetch`.
- `team_enumeration_stays_unbounded`.

**New tests, `cli/linear-client/tests/cache.rs`:**

- `refreshing_the_catalogue_preserves_the_grown_teams_array`.
- `refreshing_an_absent_catalogue_writes_every_section`.
- `refreshing_preserves_unknown_top_level_keys_and_normalises_null_sections`.
  - Extra keys survive, written after the known keys.
  - A known section stored as `null` is omitted.
- `writers_refuse_an_unreadable_or_unparseable_catalogue`. Table-driven over
  `refresh_catalogue` and `grow_catalogue`, with three cases: merge-conflict
  markers, a non-object, and a `FakeFs` read error. Each is refused and the
  file is left untouched.
- `growing_changes_only_the_teams_array`. Starts from a pre-upgrade file with
  alphabetical keys, a malformed `projects` section and an undeclared user
  field. Apart from `teams`, the file must stay byte-identical.
- `scaffold_upkeep_stays_lenient_about_an_unreadable_gitignore`.

**New tests, `cli/linear-cli/tests/flow_init.rs`:**

- `init_discover_writes_the_workspace_sections`. The written file carries every
  section. Stdout carries the team shape plus counts.
- `init_discover_writes_nothing_when_workspace_discovery_fails`.
- `init_discover_refuses_a_damaged_catalogue_naming_the_recovery`.

**New unit tests, `cli/linear-cli/src/exit_codes.rs`:**

- `catalogue_truncation_maps_to_error`
- `an_unparseable_catalogue_maps_to_error`

**Existing tests to update:**

- `flow_init::init_discover_persists_the_catalogue`. Its `team-states-200`
  scenario gains the workspace routes. It keeps its assertions and adds counts.
- `scenario_inventory.rs`. Raise the scenario count.
- The two `write_catalogue` tests. Move them onto `refresh_catalogue`.
- `FakeFs` in `tests/cache.rs:42`. Implement the widened `Filesystem::read`.

### Production changes (green)

#### 1. The catalogue document

**File**: `cli/linear-client/src/catalogue.rs`

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_labels: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub users: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_states: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_states: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
```

Known keys are written in alphabetical order, which matches today's files.
Extra keys follow them, and a `null` section is dropped.

- **`workflowStates`** stays the init team's states. Transition reads it, and a
  pre-upgrade catalogue carries it.
- **`workspaceStates`** holds every team's states, and is what filter
  resolution reads.

The typed entry types derive both `Serialize` and `Deserialize`, and every
writer (discovery and backfill) and the resolvers share them:

| Type | Fields |
|---|---|
| `CataloguedProject` | `id`, `name`, `archived_at` |
| `CataloguedLabel` | `id`, `name`, `team_id: Option<String>` |
| `CataloguedUser` | `id`, `name`, `display_name`, `email` |
| `CataloguedState` | `id`, `name`, `team_id` |

`team_id` is serialised flat as `teamId`. Both writers map the GraphQL
`team { id }` onto it, and `workspace.golden.json` fixes that shape. An entry
with a blank `id` is dropped when parsed.

#### 2. Workspace discovery

**File**: `cli/linear-client/src/discovery.rs`

Four queries, each with `first: 250`, selecting scalars plus a single
`team { id }`:

- `projects(…, includeArchived: true) { nodes { id name archivedAt } … }`
- `issueLabels(…, includeArchived: true) { nodes { id name team { id } } … }`
- `users(…, includeDisabled: true) { nodes { id name displayName email } … }`
- `workflowStates(…, includeArchived: true) { nodes { id name team { id } } … }`

`paginate(query, variables, connection, ceiling: Ceiling)` generalises
`paginate_teams`:

- Team enumeration passes `Ceiling::Unlimited`.
- Catalogue connections pass `CATALOGUE_PAGE_CEILING` (`Bounded(200)`).
- Going past the ceiling returns `SurfaceError::CatalogueTruncated`, which
  maps to `ERROR`.

#### 3. One strict, lossless write path

**File**: `cli/linear-client/src/cache.rs`

- **`Filesystem::read`.** Widened to `Result<Option<String>, CacheError>`,
  where `Ok(None)` means only "not found". `ensure_scaffold` and
  `append_line` map `Err` and `None` to empty, so they stay as lenient as
  today.
- **`LinearCache::load_for_update()`.** Reads through `self.fs`. An absent
  file gives `Ok(default)`. A file that is not a JSON object gives
  `CacheError::Unparseable { path }`.
- **`refresh_catalogue(team, workspace)`.** Replaces `write_catalogue`, under
  the shared lock. `grow_catalogue` uses the same loader and edits only
  `teams`.
- **`for_cache`.** Maps `Unparseable` to `ERROR`. The message says to resolve
  the merge conflict, or to delete `catalogue.json` and re-run init.

#### 4. Init fetches, then refreshes

**File**: `cli/linear-cli/src/main.rs`

Both fetches complete before anything is written. `discovered_summary` prints
`{team, workflowStates, workspace: {…counts}}`.

#### 5. Documentation

- **`skills/integrations/linear/init-linear/SKILL.md`.**
  - The description and Step 4 report section counts.
  - Step 3 describes the four sections and that `teams` is preserved.
  - It explains how to recover a damaged file.
  - It says the catalogue is committed and shared: pull a teammate's refresh
    first, otherwise run `accelerator linear init discover --team-id <uuid>`
    and commit the result.
- **`CHANGELOG.md` `[Unreleased]`.**
  - `Changed`: init records projects, labels, users and every team's states,
    and refuses a damaged catalogue.
  - `Security`: `catalogue.json` now records every workspace member's name,
    display name and email, and it is committed to the repo.

### Success Criteria

#### Automated Verification

- [ ] Format and lint clean: `mise run cli:check`
- [ ] Discovery tests pass: `cargo test -p linear-client --test discovery` and `cargo test -p linear-client --lib`
- [ ] Cache tests pass: `cargo test -p linear-client --test cache`
- [ ] Init flow, scenario inventory and exit-code mapping pass: `cargo test -p linear-cli`
- [ ] Read-only CI mirror green: `mise run check`

#### Manual Verification

- [ ] `init-linear` against the live tenant writes the four sections. An
      existing `teams` array survives, and the top-level key order is
      unchanged.

---

## Phase 2: The resolution domain

### Overview

This phase adds the resolution domain types and backs every family with a
catalogue resolver:

- `FilterFamily` and `TeamScopedFamily`
- `NonEmpty<T>` and `ResolvedIds`
- `NameResolver` and `Resolution`
- `TeamScopedResolver`
- `ResolverSet`, including `covers`

`StateResolver` retires. Transition moves onto a `NameResolver` for the init
team's states. `work-cli`'s grow path moves onto `Catalogue`. Until Phase 3,
`compose` resolves `state` through `team_states`, so today's init-team
behaviour holds.

### Tests first (red)

**New tests, `cli/linear-client/tests/catalogue.rs`:**

- **Projects.** Resolves a unique name. An unknown name is `NotFound`. The
  active project wins over archived ones. Two active projects are
  ambiguous, listing the candidates. A name shared only by archived projects is
  ambiguous.
- **Team-scoped families.**
  - `a_state_name_resolves_for_a_team_to_that_teams_ids`
  - `a_state_name_a_covered_team_lacks_resolves_to_nothing_for_that_team`
  - `a_team_label_resolves_only_for_its_team`
  - `a_workspace_label_resolves_for_any_team_including_uncovered_ones`
  - `a_name_is_catalogued_when_any_team_carries_it`
- **Coverage.**
  - `covers_is_true_exactly_for_teams_with_catalogued_states`
  - `labels_and_states_share_one_coverage_answer`
  - `covers_reports_a_damaged_or_absent_workspace_states_section`
  - `folded_team_sections_resolve_like_catalogued_ones`. The same mixed-case
    name resolves identically for a covered team and a team folded in by
    `with_team_sections`.
- **Users.** Match on email first, then full name, then display name. An
  ambiguous full name refuses even when the display name is unique. A value
  that matches nothing is `NotFound`.
- **Normalisation and absence.**
  - Matching is Unicode case-insensitive.
  - An empty email or display name never matches, and a blank value never
    resolves.
  - An entry with a blank id is ignored.
  - A missing section is `NotCatalogued { Outdated }`, with one case per
    section. A malformed section is `NotCatalogued { Damaged }`. A malformed
    section leaves the other families intact.
- **Team states (transition).** A single-id resolve. An ambiguous resolve.
  An absent `workflowStates` is `NotCatalogued`.
- **Loading.**
  - `resolvers_loaded_once_keep_resolving_after_the_file_is_removed`
  - `sections_are_parsed_and_indexed_on_first_resolve`
- **`NonEmpty`.**
  - `a_non_empty_collection_cannot_be_built_empty`
  - `resolved_ids_reject_a_blank_id`

**New tests, `cli/linear-client/tests/transition.rs`:**

- `a_state_absent_from_the_team_refuses_as_not_in_catalogue`. Table-driven over
  `NotFound` and `NotCatalogued`.
- `an_ambiguous_state_refuses_naming_the_count`. Table-driven over a two-id
  `Resolved` and a two-candidate `Ambiguous`.
- `transition_resolves_against_the_init_team_only`.

**Existing tests to update:**

- **`CatalogueTeam` cases.** Move them to
  `Catalogue::from_document(…).resolver_set().teams` with unchanged outcomes.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TeamScopedFamily {
    State,
    Label,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmpty<T> {
    first: T,
    rest: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedIds(NonEmpty<String>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unresolved {
    NotFound,
    NotCatalogued { section: &'static str, cause: CatalogueGap },
    TeamUncovered { team: TeamRef },
    Ambiguous {
        tier: Option<MatchTier>,
        candidates: Vec<Candidate>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Resolved(ResolvedIds),
    Unresolved(Unresolved),
}

pub trait NameResolver {
    fn resolve(&self, value: &str) -> Resolution;
}

pub trait TeamScopedResolver {
    fn catalogued(&self, value: &str) -> Result<bool, Unresolved>;
    fn resolve_for(&self, value: &str, team: &str) -> Option<ResolvedIds>;
}

pub trait TeamStatesCatalogue: TeamScopedResolver {
    fn covers(&self, team: &str) -> Result<bool, CatalogueGap>;
}

pub struct ResolverSet {
    teams: Box<dyn TeamResolver>,
    team_states: Box<dyn NameResolver>,
    workspace_states: Box<dyn TeamStatesCatalogue>,
    labels: Box<dyn TeamScopedResolver>,
    projects: Box<dyn NameResolver>,
    users: Box<dyn NameResolver>,
}
```

- **Fields are private.** A `ResolverSet` is built through its constructor and
  read through accessors.
- **Coverage.** `covers(team)` is the only way to ask, and it delegates to
  `workspace_states`. One instance therefore answers both "which ids?" and "is
  this team covered?", and a test double cannot make the two disagree.
- **Unreadable coverage.** `covers` returns `Err(CatalogueGap::Damaged)` when
  `workspaceStates` cannot be read, or `Err(CatalogueGap::Outdated)` when it is
  absent. Completion then refuses with the section's own cause before any
  backfill, instead of treating every team as uncovered.
- **Folding in fetched sections.**
  `ResolverSet::with_team_sections(&[TeamSections])` returns a set in which the
  fetched teams are folded into the `workspace_states` and `labels` indices,
  using the same `Catalogued*` types and normalisation. Every scoped team then
  resolves through one `resolve_for` path.

- **`NonEmpty<T>`.** Its fields are private. It is built from `first` plus
  `rest` and exposes `iter()` and `len()`. `ResolvedIds` adds a check that
  rejects blank ids.
- **`TeamScopedFamily`.** It converts into `FilterFamily`.
- **`TeamRef { id, key: Option<String> }`.** Names a team in refusals and in
  notes. The key comes from the catalogue's `teams` or from a fetched
  `team { key }`, and messages fall back to the id only when neither has it.
- **`TeamScopedResolver::catalogued(name)`.** Answers "does any catalogued team
  carry this name?". It errs with `NotCatalogued` when its section is missing.
- **`NameResolver` contract.** A single-valued resolver never returns more
  than one id. Transition's defensive arm for more than one id is tested.
- **Deleted.** `StateResolver`, `FixedStates` and the `lib.rs:26-27`
  re-exports. The suites' doubles are `FixedNames`, `FixedTeamScoped` and
  `FixedTeamStates`; the last one implements `TeamStatesCatalogue`, so it
  answers coverage and ids together.

#### 2. Catalogue-backed resolvers

**File**: `cli/linear-client/src/catalogue.rs`

`Catalogue::load(integrations_root)` reads the file once and is forgiving
about problems:

- An absent file yields an empty document marked `Outdated`.
- An unparseable file yields one marked `Damaged`.

`resolver_set()` builds each resolver over its raw section. Each resolver
parses its entries and builds its index inside a `OnceCell`, the first time it
resolves anything. Indices are `HashMap`s keyed on the trimmed, Unicode
lower-cased value. Blank keys are skipped.

- **`TeamStates`** (`workflowStates`). One match resolves; more are
  ambiguous.
- **`WorkspaceStates`** (`workspaceStates`). Indexes name → team → ids, and
  implements `TeamStatesCatalogue`. A team is covered when it has at least one
  catalogued state.
- **`CatalogueLabels`** (`issueLabels`). A team label resolves for its own
  team. A workspace label (`team_id` absent) resolves for any team, covered or
  not.
- **`CatalogueProjects`** and **`CatalogueUsers`.** As listed in the tests.

`Catalogue::catalogued_teams()` keeps the fold-in of the legacy `/team` entry.
`Catalogue::base_team_id()` returns `/team/id`. The module doc is rewritten.

#### 3. Client, transition, interim `compose`, and `work-cli` grow

- **Client construction.** `LinearClient::new` takes a `ResolverSet`.
  `from_config` and `build_with_override` build it from one
  `Catalogue::load`. `states()` becomes `resolvers()`.
- **`resolve_state`.** It matches exhaustively on `team_states`:
  - one id is `Ok`;
  - more than one id is `AmbiguousState { count }`;
  - `NotFound` or `NotCatalogued` is `UnknownState`;
  - `Ambiguous` is `AmbiguousState { count: candidates.len() }`.
- **Interim `compose`.** It takes `&ResolverSet` and resolves `state` through
  `team_states` with today's exactly-one rule.
- **`grow_linear_catalogue`.** It reads
  `Catalogue::load(..).catalogued_teams()`.
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

## Phase 3: Pre-flight resolution, per-team completion and id lowering

### Overview

- **Pre-flight.** `resolve_scope` resolves `project` and `assignee` to ids.
  It validates `state` and `label` names and always carries them forward as
  `ValidatedName` pairs.
- **Completion.** `search` finishes the team-scoped families once the scope's
  team UUIDs are concrete, via `complete_for_teams`. That produces a
  `LowerableSearch`, which `compose` lowers.
- **Uncovered teams.** In this phase, a scoped team the catalogue does not
  cover is refused loudly. Phase 4 replaces that refusal with backfill.
- **A new port variant.** `TrackerError::Unconfigured` gives `search` a way to
  refuse on configuration.
- **Broadened scopes.** A broadened scope passes through `resolve_scope`
  before entity resolution.
- **The standalone `search`.** It is scoped to the catalogued base team.

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

- **`cli/work-adapters/src/sync/apply.rs`** (existing classification tests):
  `an_unconfigured_error_is_classed_as_no_remote_change`, carrying its detail.
- **`cli/work-cli/src/update.rs`**:
  `an_unconfigured_push_keeps_the_baseline_and_exits_74`.

**New tests, `cli/linear-client/tests/filter.rs` and `tests/fixtures/issue-filter.txt`:**

- **Fixture.**
  - The `label`, `assignee` and `state` rows become id comparators.
  - New rows: `project`, `project-in`, `label-many-teams` and
    `state-many-teams`.
  - `FAMILIES` grows to six entries, and `parse_spec` gains `project`.
  - A `FixedNames` set supplies distinct id prefixes per family.
- `a_named_filter_bag_resolves_project_and_assignee_to_ids`
- `team_scoped_families_are_carried_as_validated_names`
- `a_named_pair_in_a_resolved_bag_is_refused` (`E_SEARCH_UNRESOLVED_SCOPE`)
- `a_validated_name_for_a_non_team_scoped_family_is_refused`
- `a_blank_resolved_id_is_refused`
- `an_unrecognised_filter_key_is_refused`
- `resolution_reports_every_unresolved_value_at_once`
- `the_refusal_renders_a_header_then_one_indented_line_per_value`
- **Refusal cases.** One per family × reason. Each asserts its `E_*` code and
  remedy. The two `NotCatalogued` causes give different remedies.
  `workspaceStates` gets a gloss. Ambiguous projects list up to five
  candidates.
- `every_named_filter_key_is_accepted_by_linear_pull_validation`

**New tests, `cli/linear-client/tests/completion.rs`:**

- `covered_teams_contribute_their_own_ids`
- `a_workspace_label_contributes_its_id_once_however_many_teams_are_scoped`.
  Ids are deduplicated per family.
- `a_damaged_workspace_states_section_refuses_as_damaged_before_completion`
- `a_family_left_with_no_ids_refuses_as_unknown`. Every scoped team lacks the
  name. The result is an `UnresolvedFilters` naming the value, never a
  `LowerableSearch` without that family.
- `an_uncovered_scoped_team_refuses_naming_the_team_by_key`. Returns
  `TeamUncovered`, named by key when the catalogue's `teams` knows it.
- `a_search_without_team_scoped_filters_needs_no_coverage`
- `compose_accepts_only_a_lowerable_search`. This is a compile-time guarantee,
  exercised by building a `LowerableSearch` and checking its composed clauses.

**New tests, `cli/linear-client/tests/resolve_scope.rs`:**

- One test per scope shape:
  - `a_base_only_scope_resolves_its_team_and_filters`
  - `a_base_plus_additional_scope_leaves_entities_as_keys`
  - `an_additional_only_scope_needs_no_base`
  - `a_whole_workspace_scope_carries_validated_names`
- `a_state_the_covered_base_team_lacks_refuses_before_any_request` (base-only)
- `a_state_name_no_catalogued_team_carries_refuses_before_any_request`
- `an_unknown_label_refuses_with_zero_requests`
  - Table-driven over all four shapes.
  - The remedy prints `--team-id <catalogued /team/id>`, or `<uuid>` only when
    the catalogue has no team.

**New tests, `cli/linear-client/tests/port.rs`:**

- Rewrite `a_flat_filter_bag_groups_same_key_values_into_one_in_clause` to use
  a resolved scope over covered teams.
- `search_completes_state_for_each_covered_scoped_team`. Two covered teams;
  the issues body carries both teams' state ids.
- `search_refuses_a_family_left_with_no_ids_without_paging`. The result is
  `Err(TrackerError::Unconfigured)` and nothing is paged.
- `search_refuses_an_uncovered_scoped_team_without_paging`
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

- `a_linear_pull_with_an_unknown_label_refuses_before_any_request`
- `a_linear_pull_against_a_pre_upgrade_catalogue_names_the_outdated_catalogue`
- `a_linear_pull_sends_seeded_label_assignee_state_and_project_ids`. No raw
  names appear, and there are no `name` comparators.
- `a_whole_workspace_state_pull_sends_every_covered_teams_ids`
  - A `Route::Sequence` returns the teams, then the issues.
  - The issues body carries every enumerated team's state id.
- `a_whole_workspace_pull_with_an_uncovered_team_refuses_as_outdated`

**New tests, `cli/linear-cli/src/exit_codes.rs` and `tests/exit_codes_parity.rs`:**

- `for_client_maps_unresolved_filters`:

  | Entries | Exit code |
  |---|---|
  | all `NotCatalogued` or `TeamUncovered` | 77 |
  | otherwise, all `State` | 78 |
  | otherwise | 89 |

  `ClientError::NoTeam` raised by the standalone search also maps to 77. The
  existing mapping to `CREATE_NO_CATALOGUE` stays for the create flow.

- `the_search_unresolved_filter_code_is_89_and_uncontested`

**New tests, `cli/linear-cli/tests/flow_search.rs`:**

- `search_by_label_sends_label_ids`
- `search_by_state_stays_scoped_to_the_init_team`
- `search_by_unknown_assignee_refuses_without_a_request`
- `search_by_state_without_a_catalogued_team_refuses_as_no_team`. Exits 77.
- `a_text_only_search_without_a_catalogued_team_runs_workspace_wide`. This is
  the documented exception to single-team scoping.

**Existing tests to update:**

- `filter.rs::an_unknown_state_is_refused_rather_than_filtered_literally`.
  Moves to pre-flight and keeps `E_SEARCH_UNKNOWN_STATE`.
- The team-key cases in `resolve_scope.rs` keep their assertions.
- `support::seed_catalogue` gains the workspace sections and keeps
  `workflowStates`.
- `scenario_inventory.rs`. Raise the scenario count.
- Every exhaustive match on `TrackerError` gains an arm. All of them follow
  the rule in §1: no remote change happened, and the exit code is 74.
  - `apply.rs:52-53` gets `FailureClass::Retryable` (retry-safe, since nothing
    was sent), and `apply.rs:387-392` gets the matching arm.
  - `work-cli/src/update.rs:228-246`: keep the baseline, and report
    `E_PUSH_UNCONFIGURED` through `for_tracker_error` (74).
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

The enum doc changes from "two classes, and closed" to three classes:

- **`Retryable`**: no remote change; a retry may clear it.
- **`Terminal`**: a remote change may have applied.
- **`Unconfigured`**: no remote change, because the configuration names no
  valid target. A retry will not clear it.

The rule for where it can appear:

- **Only `RemoteTracker::search` produces it.** The `# Errors` sections of
  `search`, `fetch_all`, `show` and `enumerate_visible_entities` change from
  "always `Retryable`" to name exactly which classes each can return.
- **Write paths treat it defensively.** They classify it as no remote change,
  keep the baseline, and exit 74.

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
    Resolved(FilterFamily),
    ValidatedName(TeamScopedFamily),
    Text,
}

pub struct Search {
    pub team_ids: Vec<String>,
    pub values: BTreeMap<FilterFamily, Vec<String>>,
    pub text: Option<String>,
}

pub struct ResolvedSearch {
    pub team_ids: Vec<String>,
    pub ids: BTreeMap<FilterFamily, ResolvedIds>,
    pub names: BTreeMap<TeamScopedFamily, NonEmpty<String>>,
    pub text: Option<String>,
}

pub struct LowerableSearch {
    team_ids: Vec<String>,
    ids: BTreeMap<FilterFamily, ResolvedIds>,
    text: Option<String>,
}
```

`FilterKey::parse` and `FilterKey::as_str` are the only places that know the
field strings: `label`, `label.id`, `label.name` and `text`.

- **`Search::from_pairs`.** Reads the named pairs.
- **`preflight(&Search, &ResolverSet, base_only_team)`.**
  - Resolves `project` and `assignee` to ids.
  - For `state` and `label`, calls `catalogued(name)`. A name no catalogued
    team carries is `NotFound`. A missing section is `NotCatalogued`.
  - When the scope is base-only and its team is covered, it also refuses a
    name that team lacks.
  - Collects every problem into one `UnresolvedFilters`.
- **`ResolvedSearch::to_pairs` / `from_pairs`.** Round-trip `ids` and `names`.
  `from_pairs` rejects named keys, blank ids, `ValidatedName` pairs for a
  non-team-scoped family, and unknown keys.
- **`complete_for_teams(resolved, scoped_teams: &[TeamRef], &ResolverSet)`.**
  This is a pure function.
  - Checks `covers` for each scoped team. A damaged or absent
    `workspaceStates` section is refused with its own `NotCatalogued` cause.
  - Refuses each uncovered team as `TeamUncovered { team }`. Phase 4 removes
    this by folding fetched sections into the `ResolverSet` before completion.
  - For each carried name, collects `resolve_for(name, team)` across the
    scoped teams, deduplicated per family through a `BTreeSet`.
  - A family with no ids at the end becomes `NotFound` ("no team in scope
    carries …").
  - The result is a `LowerableSearch`, or `UnresolvedFilters`.
- **`compose(&LowerableSearch) -> Value`.** It cannot fail.
  `ignore_case_comparator` is deleted.

The doc comments are rewritten.

#### 3. Client: pre-flight in `resolve_scope`, completion in `search`

**File**: `cli/linear-client/src/client.rs`, `cli/linear-cli/src/main.rs`

- **`resolve_scope`** runs `preflight` for every shape.
  - It substitutes the base team key only for `Keyed` with an empty
    `additional`.
  - The refusal remedy always names `Catalogue::base_team_id()`.
  - The stale inline comment is deleted.
- **`search`** runs `ResolvedSearch::from_pairs`, then `complete_for_teams`
  over the concrete scoped teams, then `compose`, and pages once.
  - Each scoped team is a `TeamRef`, keyed from the catalogue's `teams` where
    it is known.
  - An `UnresolvedFilters` here becomes
    `TrackerError::Unconfigured { detail }`, before any issues request.
  - `page_all`/`fetch_page` take the composed `Value`.
  - `fetch_all` composes each team-only search once.
- **`search_detailed`** (standalone search) resolves, completes against the
  base team, then composes. `run_search` sets `team_ids` from
  `Catalogue::base_team_id()`. When there is none:
  - team-scoped flags refuse with `E_SEARCH_NO_TEAM` (exit 77);
  - a `--text`-only search runs workspace-wide, as today, and the search skill
    documents that exception.

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

#### 5. Error variant, messages and exit codes

**File**: `cli/linear-client/src/error.rs`, `cli/linear-cli/src/exit_codes.rs`

`ClientError::UnknownState` is replaced by
`UnresolvedFilters { unresolved: NonEmpty<UnresolvedFilter> }`. It renders
under the `pull filters could not be resolved:` header, one indented line per
entry.

| Reason | Identifier | Remedy |
|---|---|---|
| `NotFound` | `E_SEARCH_UNKNOWN_{STATE,PROJECT,LABEL,ASSIGNEE}` | check the value. If it was added or renamed in Linear, or added to an existing team, pull the latest committed `catalogue.json` or refresh with `accelerator linear init discover --team-id <base team id>` and commit |
| `NotCatalogued { Outdated }` | `E_SEARCH_CATALOGUE_OUTDATED` | `catalogue.json` has no `<section>` section, because it was written before this discovery or by an older plugin. Same pull-or-refresh remedy |
| `TeamUncovered` | `E_SEARCH_CATALOGUE_OUTDATED` | `catalogue.json` has no states for team `<key>` (the id only when the key is unknown). Same pull-or-refresh remedy |
| `NotCatalogued { Damaged }` | `E_SEARCH_CATALOGUE_DAMAGED` | `<section>` in `catalogue.json` cannot be read. Resolve the merge conflict, or delete the file, then refresh |
| `Ambiguous` (assignee) | `E_SEARCH_AMBIGUOUS_ASSIGNEE` | names the tier and count. Use the user's email |
| `Ambiguous` (project) | `E_SEARCH_AMBIGUOUS_PROJECT` | lists up to five candidates. Rename one in Linear |

`for_client` maps refusals in this order:

1. every entry `NotCatalogued` or `TeamUncovered` → 77;
2. otherwise, every entry `State` → 78;
3. otherwise → 89.

`for_client` stops being a `const fn`. The module doc records 89 as a
deliberate divergence, borrowed from the show flow's decade.

#### 6. Documentation

- **`search-linear-issues/SKILL.md`.**
  - The assignee tiers.
  - Search is scoped to the init team. With no catalogued team, a text-only
    search runs workspace-wide and team-scoped flags refuse with 77.
  - The `E_SEARCH_*` refusals.
  - Exit codes 77, 78 and 89, including the state-only change from 78 to 77.
- **`sync-work-items/SKILL.md`.**
  - An exit 74 whose output carries the refusal header is a pull-filter
    refusal. Surface each line verbatim.
  - For an outdated catalogue, pull the committed one first, then run
    `/accelerator:init-linear`.
  - For a damaged catalogue, resolve the conflict.
  - Never suggest `--push-only` for these refusals.
- **`configure/SKILL.md` "Pull filters".**
  - Label and state names match per team in scope. A workspace label matches
    everywhere.
  - The assignee tiers.
  - Values resolve through `catalogue.json`.
  - A state or label added to an existing team needs a catalogue refresh.
- **`CHANGELOG.md` `[Unreleased]`.**
  - `Changed`: filters now resolve to ids.
  - `Migrations`:
    - Refresh and commit the catalogue.
    - A state-only search moves from exit 78 to 77.
    - A sync with a pull filter that cannot be resolved now refuses with exit
      74. Before, an unknown state exited 5 with "cut short… retry", and an
      unknown label or assignee was silently tolerated.

### Success Criteria

#### Automated Verification

- [ ] Format and lint clean: `mise run cli:check`
- [ ] Port variant and snapshot: `cargo test -p tracker` and `mise run public-api:check`
- [ ] Vocabulary, fixture, completion and pre-flight: `cargo test -p linear-client`
- [ ] Harness ordering: `cargo test -p tracker-test-support`
- [ ] Engine wiring and real-client sync: `cargo test -p work-adapters`
- [ ] Exit mappings, search scoping and inventory: `cargo test -p linear-cli -p work-cli`
- [ ] Read-only CI mirror green: `mise run check`

#### Manual Verification

- [ ] With a refreshed catalogue, a whole-workspace pull filtering on `state`
      returns every team's matching issues.
- [ ] A pull against a pre-upgrade catalogue refuses with
      `E_SEARCH_CATALOGUE_OUTDATED`.
- [ ] `accelerator linear search --state "In Progress"` returns only the init
      team's issues.

---

## Phase 4: Backfilling teams the catalogue does not cover

### Overview

This phase replaces Phase 3's refusal of uncovered teams with a live fetch,
and keeps `complete_for_teams` pure. `LinearClient::search` does the I/O:

1. **Partition.** It splits the scoped teams into covered and uncovered, using
   `ResolverSet::covers`.
2. **Fetch.** If a team-scoped filter is carried and any scoped team is
   uncovered, it calls `fetch_team_sections(uncovered)`. That is two paginated,
   team-filtered passes.
3. **Hold.** Only after both passes succeed for every uncovered team does it
   call `CatalogueBackfill::hold`, exactly once.
4. **Complete.** It calls `complete_for_teams` over
   `resolvers.with_team_sections(&sections)`. Every scoped team is now covered
   and resolves through the same `resolve_for` path and normalisation.

The zero-ids refusal from Phase 3 still applies.

How held sections are written:

- **`work-cli`** passes one shared `Arc<BufferedBackfill>`. It writes the
  buffer at finalisation only in `RunMode::Apply`, and drops it in
  `RunMode::Preview`.
- **Every other caller** (`linear-cli`, and the tracker-registry default)
  passes `NoBackfill`.

### Tests first (red)

**New tests, `cli/linear-client/tests/discovery.rs`:**

- `fetching_team_sections_filters_both_connections_by_the_uncovered_team_ids`.
  - Both `workflowStates` and `issueLabels` are filtered with
    `team: { id: { in: [...] } }`.
  - Both select `team { id key }`.
  - Both page through `paginate` with `CATALOGUE_PAGE_CEILING`.
- `fetching_several_teams_makes_one_paginated_pass_per_connection`

**New tests, `cli/linear-client/tests/port.rs`:**

These use a recording `CatalogueBackfill`.

- `search_fetches_an_uncovered_team_and_includes_its_ids`
- `covered_and_uncovered_teams_both_contribute`
- `nothing_is_fetched_when_every_scoped_team_is_covered`
- `nothing_is_fetched_without_team_scoped_filters`
- `a_family_left_with_no_ids_after_backfill_still_refuses`
- `fetched_sections_are_held_exactly_once`
- `a_labels_pass_failure_after_a_states_pass_holds_nothing`. The states pass
  succeeds and the labels pass fails. The result is `Retryable`, nothing is
  held and nothing is paged.
- `a_truncated_team_section_fetch_refuses_as_unconfigured`. The message names
  the connection and the 200-page ceiling. It also says a refresh would hit the
  same ceiling, so the fix is to narrow the scope.
- `a_backfilled_team_is_named_by_key_in_refusals`

**New tests, `cli/linear-client/tests/cache.rs`:**

- `recording_team_sections_inserts_entries_in_sorted_position`
  - Entries are ordered by team id, then by entry id.
  - Duplicates are skipped.
  - Every other section stays semantically equal.
  - The entry shape matches `workspace.golden.json`, including `teamId`.
- `recording_team_sections_refuses_an_unparseable_catalogue`
- `recording_team_sections_refuses_a_non_array_target_section`
- `finalising_grows_teams_and_records_sections_in_one_write`

**New unit tests, `cli/work-cli/src/sync.rs` (`#[cfg(test)]`):**

The binary harness cannot reach a mock endpoint, so the flush is tested as a
unit.

- `finalising_the_linear_catalogue`. This is table-driven over `RunMode` and
  uses a pre-filled `BufferedBackfill` and a tempdir catalogue.
  - **Apply:** the sections are written, and the combined `note:` names the
    teams by key.
  - **Preview:** the file is byte-identical and nothing is printed.
- `a_failed_catalogue_finalisation_warns_that_pulls_will_refetch`. It injects
  a failing `Filesystem` into the `LinearCache`, and asserts the warning text
  and that the run's exit code is unaffected.
- `run_sync_hands_its_backfill_to_the_registry`. The `TrackerRegistry` double
  used by the in-crate `run_sync` tests (`sync.rs:2464`) records the `Arc` it
  receives.

**New tests, `cli/work-adapters/tests/sync_run_real_client.rs`:**

- `a_whole_workspace_pull_backfills_a_team_created_after_init`. A
  `Route::Sequence` returns the teams (one uncovered), then the team-filtered
  states, then the labels, then the issues. The issues body carries the
  covered teams' ids and the new team's state id, and the recording backfill
  holds the new team's sections.
- `a_later_pull_after_recording_makes_no_section_fetch`.
  1. The first run's held sections are written through
     `LinearCache::record_team_sections` into a tempdir.
  2. A fresh client is built from `Catalogue::load(tempdir).resolver_set()`,
     with a new `MockServer`.
  3. `server.bodies()` shows no team-sections query, and the issues body
     carries the backfilled id.

**Existing tests to update:**

These are the Phase 3 tests for uncovered teams, which this phase reverses.

- `completion.rs::an_uncovered_scoped_team_refuses_naming_the_team_by_key`
  becomes `a_team_left_uncovered_after_folding_still_refuses`. This is the
  defensive case where the passed-in sections omit a team.
- `port.rs::search_refuses_an_uncovered_scoped_team_without_paging` is
  replaced by `search_fetches_an_uncovered_team_and_includes_its_ids`.
- `sync_run_real_client.rs::a_whole_workspace_pull_with_an_uncovered_team_refuses_as_outdated`
  is replaced by `a_whole_workspace_pull_backfills_a_team_created_after_init`.

### Production changes (green)

#### 1. Batched team-section fetch

**File**: `cli/linear-client/src/discovery.rs`

`fetch_team_sections(team_ids)` makes two passes through `paginate` and
`CATALOGUE_PAGE_CEILING`:

- `workflowStates(filter: { team: { id: { in: $teams } } }, includeArchived: true)`
- `issueLabels(filter: { team: { id: { in: $teams } } }, includeArchived: true)`

Both select `team { id key }`. It returns `Vec<TeamSections>` built from the
shared `CataloguedState` and `CataloguedLabel` types, with each team's key
recorded for messages.

#### 2. Fetch, hold, fold, complete in `search`

**File**: `cli/linear-client/src/client.rs`

`search` follows the four steps in the Overview. `complete_for_teams` stays
the pure Phase 3 function.

- **Network or GraphQL failure** in either pass becomes
  `TrackerError::Retryable`. It is raised before any hold and before any
  issues request. The engine reports it as `DiscoveryStatus::Failed`, as it
  does for enumeration failures.
- **`CatalogueTruncated`** becomes `TrackerError::Unconfigured` with the
  ceiling remedy.

#### 3. The backfill seam and the apply-mode flush

**Files**: `cli/linear-client/src/client.rs`, `cli/linear-client/src/cache.rs`,
`cli/work-cli/src/tracker_registry.rs`, `cli/work-cli/src/main.rs`,
`cli/work-cli/src/sync.rs`, `cli/linear-cli/src/context.rs`

- **The trait.** `pub trait CatalogueBackfill: Send + Sync { fn hold(&self,
  sections: Vec<TeamSections>); }`. It has two implementations:
  - `NoBackfill`.
  - `BufferedBackfill(Mutex<Vec<TeamSections>>)`, with `take()`.
- **The client.** `LinearClient::new` and `from_config` take an
  `Arc<dyn CatalogueBackfill>`:
  - `linear-cli`'s `context.rs:163,213` and `contract.rs:147` pass
    `Arc::new(NoBackfill)`.
  - `client_with_resolvers` defaults to `NoBackfill`.
  - `sync_run_real_client.rs:190` passes a recording buffer.
- **The registry.**
  - `ConfiguredTrackers` gains a field
    `backfill: Option<Arc<dyn CatalogueBackfill>>`, so `new` stays `const`
    with `None`.
  - A new `with_linear_backfill(Arc<dyn CatalogueBackfill>)` sets it.
  - `resolve(&self)` clones the `Arc` into each `LinearClient`, falling back
    to `NoBackfill` when the field is `None`.
- **Composition.**
  - `main.rs::run_sync` (`:434-457`) builds one `Arc<BufferedBackfill>`. It
    passes one clone to `ConfiguredTrackers::with_linear_backfill` and another
    to `sync::run_sync`.
  - `sync::run_sync` gains a `backfill: Option<Arc<BufferedBackfill>>`
    parameter. Its two callers, `main.rs:451` and the test at `sync.rs:2464`,
    are updated.
- **Finalisation.** `grow_linear_catalogue` and the backfill flush merge into
  one `finalise_linear_catalogue`. It runs after `render_report` on the
  `Ok(report)` branch.
  - **Preview:** does nothing.
  - **Apply:** takes the buffer and calls
    `LinearCache::finalise_catalogue(&new_teams, &sections)`. That is one
    locked write through `load_for_update`, which:
    - grows `teams`;
    - inserts sections in sorted position;
    - refuses a target section that is not an array.

    It then prints one combined `note: … version-controlled and repo-wide —
    commit it`.
  - **On failure:** prints `warning: … each pull will fetch these teams again
    until catalogue.json is updated and committed.`

#### 4. Documentation

- **`sync-work-items/SKILL.md`.**
  - An apply pull may add states and labels for teams created after init to
    `catalogue.json`. The `note:` names those teams.
  - Commit the file together with the pulled items.
  - A preview never writes.
- **`configure/SKILL.md` "Pull filters".**
  - A team created after init is fetched on the first pull that reaches it.
  - A name that exists only on such a team still needs a catalogue refresh.
- **`CHANGELOG.md` `[Unreleased]` `Changed`.** An apply-mode sync may update
  `catalogue.json` for teams created after init.

### Success Criteria

#### Automated Verification

- [ ] Format and lint clean: `mise run cli:check`
- [ ] Batched fetch, fold, single hold and sorted recording pass: `cargo test -p linear-client`
- [ ] Real-client backfill and the no-refetch check pass: `cargo test -p work-adapters --test sync_run_real_client`
- [ ] Apply-only finalisation and registry hand-off pass: `cargo test -p work-cli`
- [ ] Read-only CI mirror green: `mise run check`

#### Manual Verification

- [ ] Create a team after `init-linear`, then run a whole-workspace pull with a
      `state` filter:
  - [ ] `--preview` leaves `catalogue.json` unchanged.
  - [ ] An apply pull fetches the new team's issues, updates the catalogue,
        and prints the `note:`.

---

## Phase 5: Config acceptance of project under Linear

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
  - **Config and seed.** `linear.team_key` matches the seeded `team.key`. The
    seed has `team`, `teams`, `workflowStates`, `workspaceStates`, and a
    non-empty `projects` section without `Nope`.
  - **Red.** The run exits 1 with `UnsupportedFilterKey`.
  - **Green.** The run exits 74. The output carries the header and
    `E_SEARCH_UNKNOWN_PROJECT`, and does not carry
    `E_SEARCH_CATALOGUE_OUTDATED`.

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
  - The project preference rules.
  - Per-team resolution of states and team labels.
  - Workspace labels, and single-source coverage.
  - User tiers, and the exactly-one rule for team states.
  - `NotCatalogued` causes.
  - `NonEmpty`, and lazy parsing and indexing.
- **Filter vocabulary and completion.**
  - Named, resolved and validated-name round-trips.
  - The drained `LowerableSearch`.
  - The zero-ids refusal.
  - The refusal layout, the golden fixture, and the link to pull validation.
- **Exit codes.** `for_client`, `for_surface`, `for_cache`, and the `work-cli`
  mapping of `TrackerError::Unconfigured`. The value 89 is pinned.
- **Port.** `TrackerError::Unconfigured` and the snapshot.

### Integration Tests

- **Discovery.**
  - Per-connection pagination, cursors, failures and ceiling.
  - Unbounded team enumeration.
  - The batched team-filtered fetch.
- **Cache.** Refresh, grow and backfill recording: each is lossless and
  order-stable, and refuses input it cannot read.
- **Pre-flight.** Four shapes, with zero requests on refusal.
- **Completion in `search`.** Covered teams, uncovered teams (refusal in Phase
  3, backfill in Phase 4), zero ids, and fetch failure versus truncation.
- **Engine.**
  - Broadened ordering, and resolved-scope propagation.
  - `Unconfigured` is reported as `DiscoveryUnconfigured`.
- **Real-client sync.** Ids on the wire for every shape, including
  whole-workspace. Refusals. Backfill.
- **Catalogue finalisation** (unit tests in `work-cli`):
  - apply writes the catalogue and prints the combined `note:`;
  - preview leaves it byte-identical;
  - a failed write warns.
- **No refetch** (real-client): after the sections are recorded, a client
  rebuilt from the catalogue makes no section fetch.
- **Binary-level.** `project` end-to-end, through config validation and
  `resolve_scope`.
- **CLI flows.** Init, including refusing a damaged catalogue. Search scoped
  to the init team. Transition.

### Manual Testing Steps

1. Run `init-linear` on a repo with a grown `teams` array. Confirm the four
   sections are written, `teams` is intact, and the key order is unchanged.
2. Run a whole-workspace pull with `state: [In Progress]`. Confirm every
   team's matching issues come back.
3. Create a new team. Run a preview pull and confirm the catalogue is
   untouched. Run an apply pull and confirm the new team's issues come back,
   the catalogue is updated, and the `note:` is printed.
4. Run `accelerator linear search --state "In Progress"`. Only the init team's
   issues come back.
5. Use an ambiguous `assignee`. The pull refuses, naming the tier.
6. Use `project` under Jira. `configure` rejects it.

## Performance Considerations

- ⏱️ **Discovery.**
  - A page of 250 scalar nodes costs about 350 points. A page of states or
    labels with `team { id }` costs about 600. The cap is 10,000.
  - For a workspace of 2,000 users, 5,000 labels, 300 projects and 50 teams,
    init makes about 32 serial requests.
- ⏱️ **Backfill.**
  - Two paginated passes, whatever the number of uncovered teams.
  - One locked write, at the end of an apply run, after paging, so a busy lock
    never delays the pull itself.
  - It happens only when a state or label filter is configured and a scoped
    team is uncovered.
  - Until the updated catalogue is committed (on CI, or when the write
    failed), every pull fetches again. The warning says so.
- **Resolution.**
  - Pre-flight and completion each run once per pull.
  - The file is parsed once, and each section is indexed lazily.
  - A lookup is a single `HashMap` probe.
  - `in` lists grow with the number of teams, not with catalogue size.

## Migration Notes

After Phase 3 ships, an existing `catalogue.json` lacks the four workspace
sections. Any `label`, `assignee`, `state` or `project` filter then refuses
with `E_SEARCH_CATALOGUE_OUTDATED`. That is exit 74 in sync and 77 in
`search`; a state-only search exited 78 before.

`catalogue.json` is committed, so one refresh commit fixes the whole team.
The first operator runs
`accelerator linear init discover --team-id <base team id>` (or
`/accelerator:init-linear`) and commits the result. Everyone else pulls.

Behaviour after the refresh:

- **Labels and states.** They match per team in scope, and workspace labels
  match everywhere. A multi-team `state` filter now pulls every team's issues,
  which is a deliberate fix.
- **New teams.**
  - *Phase 3:* a pull whose scope reaches a team created after init is
    refused with a refresh remedy.
  - *Phase 4 onwards:* that team's states and labels are fetched during the
    pull, and written to the catalogue at the end of an apply run. Commit the
    change. Preview never writes.
- **States or labels added to an existing team.** These need a catalogue
  refresh.
- **Standalone `search`.** It stays scoped to the init team.
- **Assignees.** Assignee configs keep working when the name is unique at its
  tier. The email always works.
- **Projects.** A reused project name resolves to its only active project.
- **Damaged catalogues.** Init now refuses to overwrite a damaged catalogue.
  Resolve the conflict, or delete the file and re-run init.

- 🔒 **The committed catalogue is now a workspace snapshot.** It includes
  every member's email, and changes on refresh and on backfill.
- ⚠️ **Mixed plugin versions and shared checkouts.**
  - A pre-0292 init writes a catalogue without the workspace sections.
    `E_SEARCH_CATALOGUE_OUTDATED` names that cause.
  - A committed `project` filter makes a pre-0292 sync fail. Adopt it only
    after the whole team has upgraded.
  - Backfilled entries are inserted in sorted position, so concurrent
    backfills of different teams merge cleanly.
  - Resolve any conflicted `catalogue.json` before a pre-0292 binary runs
    sync. Its grow path overwrites an unparseable file.
  - CI pulls should either commit the refreshed catalogue or run after a
    refresh.

## References

- Original work item: `meta/work/0292-linear-project-pull-filter.md`
- Research: `meta/research/codebase/2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids.md`
- Review: `meta/reviews/plans/2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids-review-1.md`
- Prior art: `meta/plans/2026-09-11-0229-per-tracker-pull-scope-configuration.md`, `meta/work/0220-untracked-remote-discovery-never-runs-on-linear.md`
- Downstream: `meta/work/0227-accelerator-config-validate-command.md`, `meta/work/0293-negated-pull-filters.md`
- Follow-up: correct the Stories entry in parent `0146`.
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
