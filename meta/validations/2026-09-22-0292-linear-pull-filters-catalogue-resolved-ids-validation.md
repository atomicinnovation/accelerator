---
type: "plan-validation"
id: "2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids-validation"
title: "Validation Report: Linear Pull Filters via Catalogue-Resolved Ids Implementation Plan"
date: "2026-09-24T16:12:00+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids"
tags: ["linear", "pull-filters", "catalogue", "sync"]
last_updated: "2026-09-24T18:23:11+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Linear Pull Filters via Catalogue-Resolved Ids Implementation Plan

Every phase is implemented, the full local CI mirror is green, and the live
checks pass. The first validation run returned `partial` because of five
planned items: one documentation requirement, one discovery test, one
operation name, a new error identifier the plan ruled out, and two stale
docs. All five are now fixed (see "Fixed after the first validation"), so the
result is `pass`.

### Implementation Status

✓ Spike: team-section fetch. Outcome recorded 2026-09-24; batched queries
adopted.
✓ Phase 1: catalogue document, init, one strict write path. Implemented;
the missing test and operation name are added.
✓ Phase 2: resolution domain. Implemented; `ResolverSet` shape differs.
✓ Phase 3: resolution, completion, live fetch and self-heal. Implemented;
its documentation and error-identifier gaps are fixed.
✓ Phase 4: config acceptance of `project` under Linear. Implemented.

### Automated Verification Results

✓ Full local CI mirror: `mise run` exited 0 at revision `8c8a99cf` (~383s),
  and again with the fixes below applied.
  It covered format, lint, types, docs coverage, and the entire test suite.
  The formatters left the working copy unchanged.
✓ Every per-phase `cargo test -p …`, `mise run cli:check`,
  `mise run check` and `mise run public-api:check` criterion is subsumed by
  that run.

No test was run in isolation beyond this.

### Code Review Findings

#### Matches Plan

- **Harness.** `RequestKey::graphql` keys routes on operation names.
  Operation and plain routes coexist, every request is recorded under both
  keys, sequences advance per serving route, and unmatched operations panic
  on drop (`cli/http-test-support/src/lib.rs`). All eight named tests exist.
- **Discovery.**
  - `fetch_team_entries` follows the spike's queries and page sizes.
  - Ceilings are 60 pages for team-scaled connections and 30 for
    `members`/`projects`, and one `Deadline` spans a fetch.
  - A nested `teams` connection past one page fails loud with
    `CatalogueTruncated`. `discover_team` is deleted.
- **Catalogue document and cache.**
  - One `CatalogueDocument` parses under `Strictness`, and the legacy base is
    adopted at parse time.
  - `record_team_entries` implements every merge rule, keeps unknown keys at
    every level, never re-points `baseTeam`, and writes keys alphabetically.
  - `write_catalogue` and `grow_catalogue` are deleted.
- **Resolution.** The domain types exist with private fields. Active-wins
  applies per group, members match email → full name → display name, and
  `TeamStates` reads non-archived base states. Transition keeps exit codes
  122/123.
- **Port.**
  - `TrackerError::Unconfigured` is added, and the three-class doc table
    exists.
  - The frozen oracle records 74 as reaching the port, and the public-API
    snapshot is updated.
  - `exit_code_for_report` ranks 71 > 4 > 74 > 70.
- **Pre-flight and completion.**
  - `resolve_scope` runs `preflight` for every shape, and substitutes the
    base key only for base-only scopes.
  - `complete_scope` partitions the teams, fetches once, holds once after
    success, and completes over `with_fetched`.
  - `compose(&LowerableSearch)` cannot fail.
- **Engine.** Broadened scopes pass through `resolve_scope` before entity
  resolution. `Unconfigured` from `search` becomes
  `RunError::DiscoveryUnconfigured`.
- **Healing.**
  - `SyncedTeams::derive` reads only `ExternalId`s.
  - `CatalogueHealing::heal` implements the five steps. Nothing to heal means
    no factory call, no lock and no write, and prefixes are confirmed at
    most three per heal.
  - The `RunFinaliser` is skipped in preview and push-only runs, and
    `main.rs` shares one `Arc<CatalogueHealing>` between the registry and the
    finaliser.
- **Phase 4.** `JIRA_FILTERS`/`LINEAR_FILTERS` and `Tracker::filter_schema`
  replace `FILTER_SCHEMA`, with tests for both trackers and the binary-level
  Jira rejection.
- **Docs.** `init-linear`, `sync-work-items` and `configure` skills, plus the
  `CHANGELOG.md` `Added`, `Changed`, `Migrations` and `Security` entries.

#### Deviations from Plan

- **Structure (acceptable).** Types live in `resolution.rs` and in a
  `catalogue/` module directory.
  - `ResolverSet` holds an `Rc<dyn NameResolver>` plus a concrete
    `TeamEntries`, so there is no `FixedTeamEntries` double.
  - `paginate` takes a `PagedConnection` rather than
    `PageSize`/`Ceiling`, and there is no `page_ceiling()`.
  - `TeamResolver` is removed rather than kept as a view.
  - `FinishedRun` computes `applied_imports()` rather than storing it.
- **Tests weaker than planned.**
  - `each_section_query_sends_its_page_sizes` checks only the outer
    `first:`.
  - The nested-overflow test covers `projects` but not `members`.
  - `an_unconfigured_error_is_classed_unconfigured` exercises only update,
    not create abandonment.
  - `without_a_token_the_team_entry_fetch_is_unconfigured` asserts only that
    the message contains `"linear"`.
  - `every_key_in_scope_has_its_id_among_the_teams_paged` compares
    `catalogued_teams` with `team_by_key`, not `fetch_all`'s ids.
  - No test renders `NotCatalogued{Absent}` as `E_SEARCH_NO_TEAM`.
- **Superseded pins.** Phase 1's `growing_*` tests in `work-cli/src/sync.rs`
  are gone along with `grow_linear_catalogue`, which Phase 3 planned to
  delete. `healing.rs` covers the behaviour.
- **Init with an unreturned synced team** warns and keeps that team's stored
  entry rather than writing nothing. An unreturned base team still exits
  `BAD_RESPONSE`.
- **`CatalogueBackfill`** lacks the planned `Send + Sync` supertraits.

#### Fixed after the first validation

- **`E_PUSH_UNCONFIGURED` removed.** A push refused on configuration now
  reports the tracker's detail verbatim and exits 74, as planned.
- **Search skill completed.** It documents exit codes 77, 78 and 89 and adds
  `E_SEARCH_TEAM_UNFETCHED`, and `--limit N` is back in the flag list.
- **`one_deadline_spans_every_section_of_a_fetch` added.** It uses a new
  `Route::Delayed` in `http-test-support`. The test fails when each section
  pass is given its own deadline.
- **`TeamEnumeration` named.** The enumeration routes in
  `tests/discovery.rs` and `sync_run_real_client.rs` now key on it.
- **Stale docs corrected** at `tracker/src/lib.rs` (the `Retryable` read
  rule) and `work-cli/src/exit_codes.rs` (per-item `unconfigured` exits 74).

#### Potential Issues

- **A create refused as `Unconfigured` becomes a silent local save.**
  - In `cli/work-cli/src/create.rs:430-441`, `push_decide(74)` returns
    `LocalSave` → `Exhausted`, which drops the detail.
  - The plan's "exit 74 on write paths" is unmet for create.
  - This is defensive only today, because only `search` produces
    `Unconfigured`.
- **Recurring prefix lookups.** An unconfirmed or renamed prefix (for
  example `OLD-`, or a leftover Jira prefix) costs up to three
  `TeamOfIdentifier` requests on every apply sync. The renamed case does
  this silently. The plan accepts this cost (Performance, Finalisation), but
  a repo migrated from Jira with many prefixes pays it per prefix.
- **Heal on an absent catalogue** writes a `catalogue.json` with teams and
  labels but no `baseTeam`. A dangling `baseTeam` (an id absent from
  `teams`) makes `to_json` drop the `team`/`workflowStates` projections,
  which breaks pre-0292 binaries.
- **A labels-only heal prints no `note:`.** When a heal writes only workspace
  labels, `catalogue.json` changes but no "commit it" note appears.
- **Misattributed diagnostics.**
  - A catalogue read failure during heal is reported as "could not be
    written" (`finaliser.rs:151`).
  - `fetch_failed` mixes prefixes and team keys, and can name one twice.
  - `NotCatalogued{Absent}` for missing workspace labels renders as "there is
    no catalogue base team".
- **Damaged sections block filtered pulls.** `complete_scope` fetches only
  on `covers == Ok(false)`, so a damaged section is refused rather than
  repaired live. This is consistent with the remedy table.
- **An unreturned uncatalogued team is named by UUID** in `TeamUnfetched`,
  not by key.
- **`TeamStates` keeps blank state ids** (`catalogue/team_states.rs:37-41`),
  unlike the family indices. A `{id: ""}` state would resolve to
  `Resolved("")`.
- **`TeamMembers` receives an undeclared `ids` variable**
  (`discovery.rs:441-445`). Linear tolerates it, as the live check
  confirmed.
- **`record_team_entries` can return `Err` after a successful write**, when
  `ensure_scaffold` fails post-write (`cache.rs:169-171`).
- **Stale gitignored `cli/.pup/linear_client_context.json`** still lists the
  deleted resolver types. Regenerate it.

#### Comment policy

- **Stale:** `cli/linear-client/src/cache.rs:33-35` ("team and states are
  team-scoped").
- **Porting history:** `discovery.rs:12-14`.
- **Restating code:**
  - `healing.rs:28-29`
  - `healing.rs:109-115`
  - `finaliser.rs:25`
  - `tests/healing.rs:142`
  - `flow_init.rs:143`
  - `auth.rs:123-124`
  - `filter.rs:402`
- **Work-item number in test names:** `a_0229_teams_entry_covers_nothing`
  and the `"0229"` case label (`tests/catalogue.rs:669,1164`).

### Manual Testing Required

These were run against the live tenant with the workspace build on
2026-09-24:

1. Phase 3:
   - [x] On a legacy catalogue, a filtered `--preview` pull leaves
     `catalogue.json` byte-identical across two runs. Discovery ran
     (`found=0`) and did not refuse. The exit code was 4 because 12 tracked
     items have existing sync conflicts, which outrank discovery in
     `exit_code_for_report`. The filter path did not cause it.
   - [x] An apply pull completes the base entry and prints the `note:`. A
     third pull prints nothing and leaves the file unchanged.
   - [x] A whole-workspace pull filtering on `state` returns issues from
     other teams, and catalogues only the teams it imported from.
   - [x] A synced team deleted from `teams` is re-derived by the next apply
     sync.
   - [x] `accelerator linear search --state "In Progress"` returns only `PP`
     issues.
   - [ ] An ambiguous `assignee` refuses, naming the tier. This could not be
     run: no two of the tenant's 16 members share a full name or display
     name, and none are disabled. It is covered by `tests/catalogue.rs`.
2. Phase 4:
   - [x] A Jira `project` filter is refused at validation.
   - [x] A live Linear pull with a real `project` returns only that
     project's issues.

### Recommendations

- **Worth deciding:**
  - Route a create `Unconfigured` to exit 74 with its detail rather than a
    silent local save.
  - Print the `note:` whenever heal writes, including a labels-only write.
  - Guard heal against writing a catalogue with no `baseTeam`.
  - Consider recording confirmed-elsewhere prefixes so renamed and foreign
    prefixes stop costing lookups on every apply sync.
- **Tidy:**
  - Strengthen the weaker tests listed above.
  - Drop blank ids in `TeamStates`.
  - Fix the comment-policy items.
  - Advance work item 0292's status once merged.
