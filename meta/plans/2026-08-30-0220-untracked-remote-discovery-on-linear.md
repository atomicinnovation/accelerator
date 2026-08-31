---
type: "plan"
id: "2026-08-30-0220-untracked-remote-discovery-on-linear"
title: "Untracked-Remote Discovery on Linear Implementation Plan"
date: "2026-08-30T17:33:07+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "done"
work_item_id: "work-item:0220"
parent: "work-item:0220"
derived_from: ["codebase-research:2026-08-30-0220-untracked-remote-discovery-never-runs-on-linear"]
tags: ["sync", "linear", "tracker", "discovery"]
revision: "0f1624a83df9437999143ce09064485de53666b5"
repository: "accelerator"
last_updated: "2026-08-30T20:03:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Untracked-Remote Discovery on Linear Implementation Plan

## Overview

Untracked-remote discovery returns zero pulls on Linear for two independent
reasons: with the scope key unset the discovery gate skips the search silently,
and with the key set the key is passed to Linear as a team **key** where the
filter wants the team **UUID**, matching no team. This plan lifts key→UUID
resolution into a new `resolve_scope` port method that validates the discovery
scope **before any push**: a missing or unresolvable key refuses the run
pre-flight (`74`, nothing sent), while a resolved key drives a UUID-bounded
search. It then makes every proceeding outcome — ran, skipped, transiently
failed — visible in the sync report so a skip is never again mistaken for an
empty result.

## Current State Analysis

The discovery gate in `cli/work-adapters/src/sync/run.rs:707-709` enables the
untracked search only when `!PushOnly && (scope.project.is_some() ||
scope.all_projects)`. On Linear `scope.project` is built from
`work.default_project_code` (`cli/work-cli/src/sync.rs:431-438`), typically
unset, so the gate's `else` branch (`run.rs:723-725`) yields an empty
`Vec<ExternalId>` with no search and no report line.

Two facts make the "required key" decision collapse the fix:

- The gate **already opens** when the key is set (`scope.project.is_some()`), so
  the with-key happy path needs no gate or trait change — only the key→UUID
  resolution.
- `RemoteTracker::search` has exactly one production caller,
  `discover_untracked` (`run.rs:425`), reached only when `scope.project` is
  `Some`. The two other callers are test infrastructure
  (`cli/tracker-test-support/src/contract.rs:306`,
  `cli/tracker-test-support/src/seed.rs:188`), both passing explicit scopes.
  Neither relies on `None` meaning "credential team", so Linear's `search` may
  require the key outright.

With the key set to a team key like `PP`, Linear's `search`
(`cli/linear-client/src/client.rs:661-686`) copies it verbatim into
`Search.team_id`, and `compose` (`cli/linear-client/src/filter.rs:69-70`) emits
`{"team":{"id":{"eq":"PP"}}}` — a filter Linear evaluates against the team UUID.
`PP` matches nothing and discovery returns empty, silently, unlike the state
path which refuses unknown names via `ClientError::UnknownState`. The
credential-team fallback (`client.rs:666`) exists only as access control, not as
scope authority, and is the mechanism the work item retires.

The observability gap is total: `read_failure` drives only the exit code
(`cli/work-cli/src/sync.rs:228`) and is never rendered; a search error folds into
`read_failure` and an empty `Vec` (`run.rs:718-721`), indistinguishable from a
completed empty search. `baseline_degradation` is the only degradation currently
rendered (`sync.rs:454-466`).

### Key Discoveries:

- The with-key defect is the key→UUID trap in `search`
  (`cli/linear-client/src/client.rs:661-686`); the fix lifts resolution into a
  pre-flight `resolve_scope` so the same step also validates the scope before any
  push.
- `StateResolver` is the port precedent to mirror: trait in
  `cli/linear-client/src/filter.rs:18-30`, cache-backed `CatalogueStates` in
  `cli/linear-client/src/catalogue.rs`, `FixedStates` test double, injected as
  `Box<dyn StateResolver>` (`client.rs:125`, built in `from_config` at
  `client.rs:164`).
- `LinearClient.team_key` (`client.rs:124`) serves `in_scope`
  (`client.rs:193-200`) — identifier-prefix scoping, the `PP` in `PP-42` — a
  different job from key→UUID resolution. `TeamResolver` is **additive**; it does
  not replace `team_key`.
- `resolve_scope` performs no network I/O, so any error it returns is a config
  fault, distinct from a transient `search` failure without needing a discriminant
  on `TrackerError` (both are `Retryable`; the port contract forbids `Terminal` on
  reads, `cli/tracker/src/lib.rs:424`). `prepare_run` runs before the apply/push
  phase (`run.rs:788`), so refusing an invalid scope there sends nothing — both
  AC-3's no-key and AC-5's unresolvable-key exit `UNCONFIGURED` (74) honestly.
  Only a post-resolution transient search failure exits `RETRYABLE` (70).
- `RemoteTracker::search`'s only production caller is gated on
  `scope.project.is_some()`; a required key is safe.
- The public `run()` returns `Result<RunReport, RunError>`; the CLI already
  renders hard `RunError` variants as clear messages (`sync.rs:491-525`).

## Desired End State

A bidirectional Linear sync with a configured team key discovers untracked team
issues and reports them as planned pulls, with the search filter carrying the
resolved team UUID and nothing enumerating the wider workspace. A run with the
key unset, or set to a key that resolves to no catalogue team, reports the
reason and exits non-clean rather than returning zero silently. Every run states
whether discovery ran, was skipped, or failed — and why — distinguishably from a
search that completed with no results.

Verify by running a bidirectional preview sync against a mocked Linear with a
seeded untracked issue and observing a `create-from-remote` planned pull plus a
`discovery ran` report line; and by running with no key and observing a
pre-flight `discovery unconfigured` refusal — nothing pushed — with a `74` exit.

## What We're NOT Doing

- No `RemoteTracker` capability boolean (`self_bounds_search`). Instead the port
  gains an operational `resolve_scope` method — a validation/resolution seam, not
  a capability flag — so each tracker validates its own discovery scope. This is
  a deliberate expansion of the port surface (public-api-pinned; snapshot regen
  required) chosen so a config fault is caught *before* any mutation.
- No change to Jira's search or JQL. `E_JQL_NO_PROJECT`
  (`cli/jira-client/src/jql.rs:169-174`) stays as the deep guard, but the
  discovery-path refusal now lives in Jira's `resolve_scope` (which returns the
  same error class): an unscoped run is refused pre-flight before `search`, so
  AC-6's "performs no unbounded search" holds. This is a deliberate behavioural
  change for Jira: an unscoped bidirectional Jira run exits `0` today and `74`
  after, the intended uniform treatment — a run that cannot discover is a
  misconfiguration to refuse on either tracker.
- No multi-team key→UUID map. The catalogue models one team; resolution is a
  single equality against the catalogue team key.
- No `all_projects`/all-teams discovery on Linear; that is 0146's scope/config
  redesign.
- No rename of `work.default_project_code`. This plan targets the current field;
  reconciliation with the 0146 `work.key` rename is a stated assumption below.
- No new exit code. A pre-flight config refusal reuses `UNCONFIGURED` (74),
  whose "nothing was sent" invariant holds because `prepare_run` aborts before
  the apply/push phase (`run.rs:788`).

## Implementation Approach

The fix validates the discovery scope **before any side effect**. A missing or
unresolvable key is a configuration fault caught pre-flight in `prepare_run`,
which aborts the whole run before the push side executes — so nothing is sent and
`UNCONFIGURED` (74) is honest. Only a genuine transient search failure (network),
which happens after the scope is validated, degrades gracefully and exits
`RETRYABLE` (70). This is why the fault classes need separating at their source:
resolution (deterministic, local) is lifted into a new `resolve_scope` port
method, distinct from `search` (network).

Two test-driven phases. **Phase 1** is the working fix: the `TeamResolver` +
`resolve_scope` machinery, the pre-flight call in `prepare_run` that refuses an
invalid scope with `RunError::DiscoveryUnconfigured`, and a `search` that consumes
the pre-resolved scope. It satisfies AC-1..AC-6 and AC-8. **Phase 2** is
observability on top: a `DiscoveryStatus` report line for the runs that proceed
(ran / push-only / transient-failure), satisfying AC-7. Phase 2 builds on Phase
1's gate shape, so it merges after; Phase 1 is self-contained and shippable
alone (transient search errors fold into the existing `read_failure` path until
Phase 2 gives them a line).

## Phase 1: Discovery validates its scope pre-flight and refuses an invalid one

### Overview

Introduce a `TeamResolver` port mirroring `StateResolver` and a catalogue-backed
implementation injected into `LinearClient`. Add a `resolve_scope` method to the
`RemoteTracker` port: a deterministic, local step that resolves provider identity
(Linear's team key → UUID) and validates presence, distinct from the network
`search`. Call `resolve_scope` in `prepare_run` before the push side runs, and
refuse an invalid scope with a new `RunError::DiscoveryUnconfigured` that aborts
the run — so a config fault sends nothing and exits `74`. `search` then consumes
the pre-resolved scope, so Linear's `search` uses the resolved UUID directly and
drops the credential-team fallback.

### Changes Required:

#### 1. The `TeamResolver` port and test double

**File**: `cli/linear-client/src/filter.rs`
**Changes**: Add the trait beside `StateResolver`, and a fixed test double beside
`FixedStates`.

```rust
pub trait TeamResolver {
    fn resolve(&self, key: &str) -> Option<String>;
}

#[derive(Debug, Clone, Default)]
pub struct FixedTeam(pub std::collections::BTreeMap<String, String>);

impl TeamResolver for FixedTeam {
    fn resolve(&self, key: &str) -> Option<String> {
        self.0.get(key.trim()).cloned()
    }
}
```

The map shape mirrors `FixedStates`; production resolution is single-team
(`CatalogueTeam` holds one pair), so a `FixedTeam` only ever carries one entry.

#### 2. The catalogue-backed resolver

**File**: `cli/linear-client/src/catalogue.rs`
**Changes**: Add `CatalogueTeam`, reading `/team/key` and `/team/id` from the
single-team catalogue, resolving a key to the UUID only when it matches the
catalogue team key.

```rust
#[derive(Debug, Clone, Default)]
pub struct CatalogueTeam {
    key: Option<String>,
    id: Option<String>,
}

impl CatalogueTeam {
    #[must_use]
    pub fn load(integrations_root: &Path) -> Self {
        let path = integrations_root.join("linear/catalogue.json");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .map(|catalogue| Self::from_catalogue(&catalogue))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn from_catalogue(catalogue: &Value) -> Self {
        Self {
            key: pointer_string(catalogue, "/team/key"),
            id: pointer_string(catalogue, "/team/id"),
        }
    }
}

impl TeamResolver for CatalogueTeam {
    fn resolve(&self, key: &str) -> Option<String> {
        match (&self.key, &self.id) {
            (Some(team_key), Some(team_id)) if team_key.trim() == key.trim() => {
                Some(team_id.clone())
            }
            _ => None,
        }
    }
}
```

`CatalogueTeam::load` is the third catalogue reader in the crate (after
`CatalogueStates::load` and the `auth.rs` team reads), and `from_catalogue` reads
two pointers. Extract a shared `pointer_string(&Value, &str) -> Option<String>`
helper and route `from_catalogue` through it rather than inlining; the
`read_to_string / from_str / unwrap_or_default` load shape stays copied from
`CatalogueStates::load` for now. `pointer_string` must mirror `auth.rs`'s
`catalogue_field` and filter empty strings (`.filter(|value| !value.is_empty())`),
so an empty `/team/key` reads as `None` — this keeps the flagged `auth.rs`
consolidation (a clean follow-up, out of scope) behaviour-preserving and stops
`CatalogueTeam` resolving on a blank key.

#### 3. Inject the resolver into `LinearClient`

**File**: `cli/linear-client/src/client.rs`
**Changes**: Add a `teams: Box<dyn TeamResolver>` field beside `states`
(`client.rs:125`), extend `new` to take it, and build `CatalogueTeam::load` in
`from_config` (`client.rs:156-165`).

```rust
pub struct LinearClient {
    transport: Transport,
    upload: UploadTransport,
    team_key: Option<String>,
    teams: Box<dyn TeamResolver>,
    states: Box<dyn StateResolver>,
}
```

```rust
let team_key = crate::auth::catalogue_team_key(integrations_root);
let teams = crate::catalogue::CatalogueTeam::load(integrations_root);
let states = CatalogueStates::load(integrations_root);
Ok(Self::new(transport, upload, team_key, Box::new(teams), Box::new(states)))
```

`team_key` stays: `in_scope` (`client.rs:193-200`) still needs it for the
identifier-prefix partition in `fetch_all`.

#### 4. Add `resolve_scope` and a `ScopeError` to the `RemoteTracker` port

**Files**: `cli/tracker/src/lib.rs`; all **six** `impl RemoteTracker` blocks — the
two production clients (`LinearClient`, `JiraClient`) and the four test doubles
`RecordingTracker` (`cli/tracker-test-support/src/lib.rs:203`), `FixedTracker`
(`cli/tracker/tests/port.rs:65`), `Fake` (`cli/work-adapters/tests/sync_apply.rs:102`),
and `MarkerObservingTracker` (`cli/work-adapters/tests/sync_create.rs:979`);
`cli/tracker/tests/fixtures/public-api.txt`.
**Changes**: Add a required method beside `search`, and a dedicated `ScopeError`
type for its failure. `resolve_scope` performs no network I/O; a `ScopeError` is,
by construction, a configuration fault — carried by its own type rather than
overloading `TrackerError::Retryable`, so it can never be routed through
`for_tracker_error` (which maps every `Retryable` to `70`) and mis-exit as a
transient failure.

```rust
/// A discovery scope that names no valid search target for this tracker — a
/// missing or unresolvable key. A configuration fault, distinct from a transient
/// read failure; the caller surfaces it before any mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeError {
    pub detail: String,
}

/// Resolves and validates a discovery `scope` for this tracker, without any
/// network call. Substitutes provider identity (e.g. a team key for its UUID)
/// and returns the scope ready for `search`.
///
/// # Errors
///
/// [`ScopeError`] when the scope names no valid target. It resolves locally and
/// mutates nothing, so it never reports a transient or terminal failure.
fn resolve_scope(
    &self,
    scope: &SearchScope,
) -> Result<SearchScope, ScopeError>;
```

The four test doubles return `Ok(scope.clone())` (they perform no resolution);
`RecordingTracker` also gains a `refusing_scope(ScopeError)` builder (mirroring
`failing_search`) so a run can be driven into the pre-flight refusal.
`resolve_scope` is a required method, matching the trait's uniform all-required
style. Both the method and `ScopeError` change `tracker`'s public API, so
regenerate its snapshot with `mise run public-api:update` and verify with `mise
run public-api:check` (there is no `cargo test --test public_api` target).

#### 5. Linear `resolve_scope`, and a `search` that consumes the resolved scope

**File**: `cli/linear-client/src/client.rs`
**Changes**: Implement `resolve_scope` to require `scope.project`, resolve it to
the UUID via the injected `teams`, and refuse an unresolved key with a `ScopeError`.
`search` then takes the resolved UUID — but **keeps a defensive refusal** for a
`None` team id.

```rust
fn resolve_scope(
    &self,
    scope: &SearchScope,
) -> Result<SearchScope, ScopeError> {
    let Some(key) = scope.project.as_deref() else {
        return Err(ScopeError {
            detail: "E_SEARCH_NO_TEAM: discovery needs a team key; set \
                     work.default_project_code, or run --push-only to push \
                     without discovery"
                .to_owned(),
        });
    };
    let Some(team_id) = self.teams.resolve(key) else {
        return Err(ScopeError {
            detail: format!(
                "E_SEARCH_UNKNOWN_TEAM: team key {key:?} resolves to no team \
                 in the catalogue; check work.default_project_code matches the \
                 catalogue team key, or refresh linear/catalogue.json"
            ),
        });
    };
    Ok(SearchScope {
        project: Some(team_id),
        ..scope.clone()
    })
}

fn search(&self, scope: &SearchScope) -> Result<Discovery, TrackerError> {
    let Some(team_id) = scope.project.clone() else {
        return Err(TrackerError::Retryable {
            detail: "E_SEARCH_UNRESOLVED_SCOPE: search needs a resolved team id; \
                     call resolve_scope first"
                .to_owned(),
        });
    };
    let mut search = Search {
        team_id: Some(team_id),
        ..Search::default()
    };
    for (field, value) in &scope.filters {
        match field.as_str() {
            "state" => search.state = Some(value.clone()),
            "assignee" => search.assignee = Some(value.clone()),
            "label" => search.label = Some(value.clone()),
            "text" => search.text = Some(value.clone()),
            _ => {}
        }
    }
    let (index, truncated) = self.page_all(&search);
    Ok(Discovery {
        found: index
            .into_iter()
            .map(|(id, stamp)| (ExternalId::new(id), stamp))
            .collect(),
        complete: truncated.is_none(),
    })
}
```

The defensive `None` guard is load-bearing, not decorative. Dropping the old
credential-team fallback (`.or_else(|| Some(self.credentials().team_id))`) means a
`None` `team_id` would otherwise reach `compose` (`filter.rs:69`), which emits an
**empty** filter — a workspace-wide enumeration, the 997+-issue flood this epic
exists to prevent, not an empty result. The guard makes an unresolved scope
reaching `search` refuse rather than flood, mirroring Jira's `E_JQL_NO_PROJECT`
deep guard. In the wired flow `resolve_scope` guarantees a UUID, so the guard is
defence-in-depth for any direct caller.

#### 6. Jira `resolve_scope`

**File**: `cli/jira-client/src/client.rs`
**Changes**: Implement `resolve_scope` to validate project presence, returning
the scope unchanged (Jira's JQL uses the project key directly) or the existing
`E_JQL_NO_PROJECT` error when both `project` and `all_projects` are unset. This
moves the discovery-path refusal ahead of `search`; the `compose` guard in
`jql.rs:169-174` stays as defence for any direct `search` call.

```rust
fn resolve_scope(
    &self,
    scope: &SearchScope,
) -> Result<SearchScope, ScopeError> {
    if scope.project.is_none() && !scope.all_projects {
        return Err(ScopeError {
            detail: "E_JQL_NO_PROJECT: specify a project or all_projects, \
                     or run --push-only to push without discovery"
                .to_owned(),
        });
    }
    Ok(scope.clone())
}
```

#### 7. Call `resolve_scope` pre-flight in `prepare_run`

**File**: `cli/work-adapters/src/sync/run.rs:707-725`, plus `RunError`
(`run.rs:44`) and its CLI rendering (`cli/work-cli/src/sync.rs:491-524`).
**Changes**: Before discovery, resolve the scope; refuse an invalid one with a new
hard `RunError::DiscoveryUnconfigured { detail }`. Because `prepare_run` runs
before the apply/push phase (`run.rs:788`), the refusal sends nothing. A transient
`search` failure still folds into the existing `read_failure` path (Phase 2 gives
it a report line).

The call sits at the existing gate (`run.rs:707`), which is **after**
`fetch::gather` (`run.rs:671`) has issued its tracked-item reads. The "nothing was
sent" invariant is about mutations, so it holds regardless — but a deterministic,
local config fault still pays a full remote gather before being refused. Hoisting
the `resolve_scope` refusal above `fetch::gather` for non-`PushOnly` runs would
fail fast and save that round-trip; keeping it at the gate keeps the discovery
logic in one place. Hoist it unless the gate-locality is worth the wasted gather.

```rust
let untracked = if matches!(request.direction, SyncDirection::PushOnly) {
    Vec::new()
} else {
    let resolved = ports
        .tracker
        .resolve_scope(&request.scope)
        .map_err(|error| RunError::DiscoveryUnconfigured {
            detail: error.detail,
        })?;
    match discover_untracked(ports.tracker, &resolved, request.items) {
        Ok(discovered) if !discovered.complete => {
            return Err(RunError::DiscoveryIncomplete {
                found: discovered.ids.len(),
            });
        }
        Ok(discovered) => discovered.ids,
        Err(error) => {
            read_failure = read_failure.or(Some(error));
            Vec::new()
        }
    }
};
```

`ScopeError` maps to `RunError` by a plain field move — no `TrackerError`
destructure, and no way for the config fault to leak into the transient
`for_tracker_error` path. This `resolve_scope` refusal precedes the write-bound
`RunError::Refused` check (`run.rs:734`), so a run that is both unconfigured and
over-threshold surfaces the config fault first — the deliberate ordering, since a
scope the operator must fix outranks a bound they can raise.

Add `DiscoveryUnconfigured { detail: String }` to `RunError`, and a CLI arm
mapping it to `exit_codes::UNCONFIGURED` (74) — mirroring the existing
`RunError::DiscoveryIncomplete` arm (`sync.rs:509`). Frame the rendered message
in operator language like its siblings ("refused: discovery is unconfigured — …"),
keeping the `E_SEARCH_*` / `E_JQL_*` sentinel inside the `detail` it wraps rather
than leading with it. 74's "nothing was sent" invariant holds: no apply ran.

#### 8. Update every `LinearClient::new` caller

**Files**: all five construction sites, listed below.
**Changes**: Inject a `TeamResolver` at each. The one production site beyond
`from_config` is `build_with_override` (`cli/linear-cli/src/context.rs:182`),
which builds a client directly for the endpoint-override path — inject
`CatalogueTeam::load(integrations_root)` there, mirroring `from_config`, **not** a
test double, or the override discovery path would refuse every key.

- `cli/linear-cli/src/context.rs:182` (`build_with_override`) — production:
  `CatalogueTeam::load(integrations_root)`.
- `cli/linear-client/tests/support/client.rs:68-73` (`client_for`) — `FixedTeam`
  mapping `TEAM_KEY → TEAM_ID` (constants at `support/client.rs:16-17`), so the
  search suites resolve.
- `cli/linear-client/tests/support/client.rs:92-97`
  (`client_with`/`client_with_states`) — take or default a resolver as their
  existing signatures dictate.
- `cli/linear-client/tests/contract.rs:146` — a defaulted `FixedTeam` unless the
  case exercises `search`.
- `cli/work-adapters/tests/sync_run_real_client.rs:179` — inject a `FixedTeam`
  mapping `ENG → TEAM_ID`, and migrate the shared `execute` harness (`~:211-247`).
  This is the load-bearing test fix: `execute` today hardcodes `direction:
  Bidirectional`, `scope: SearchScope::default()`, and `run(...).unwrap_or_else(||
  panic!(...))`. After Change 7, an unscoped bidirectional run refuses
  (`DiscoveryUnconfigured`), so the five existing callers (jira/linear classify,
  the two truncated-read tests, the dossier test) would panic. Give `execute`
  `scope` and `direction` parameters and a `Result<RunReport, RunError>` return;
  migrate each existing caller to pass a resolvable scope (or `PushOnly` where the
  case does not exercise discovery) and `.expect(...)` the report. The
  `TEAM_KEY`/`TEAM_ID` constants are not visible from the `work-adapters` crate, so
  declare local constants there. Verify with `cargo nextest run -p work-adapters
  sync_run_real_client`.

### Test-Driven Steps:

1. **Red** — `cli/linear-client/tests/` `resolve_scope`: `SearchScope { project:
   Some(TEAM_KEY), .. }` with a `FixedTeam` mapping `TEAM_KEY → TEAM_ID` returns
   `Ok` with `project == Some(TEAM_ID)` (the UUID, not `ENG`). Fails today:
   `resolve_scope` does not exist.
2. **Red** — the same: `SearchScope { project: Some("ZZ"), .. }` with a
   `FixedTeam` that knows only `ENG` returns `Err(TrackerError::Retryable)` whose
   detail contains `E_SEARCH_UNKNOWN_TEAM`; `SearchScope { project: None, .. }`
   returns `Err` whose detail contains `E_SEARCH_NO_TEAM`.
3. **Red** — `cli/linear-client/tests/search_projection.rs`: a `MockServer`
   `search` over a **pre-resolved** `SearchScope { project: Some(TEAM_ID), .. }`
   asserts that **every** captured body carries `{"team":{"id":{"eq":TEAM_ID}}}`
   via `server.bodies(...)` — the positive UUID projection and the negative "no
   page enumerates the wider workspace" (AC-2) in one assertion.
4. **Red** — `cli/linear-client/tests/` unit coverage for `CatalogueTeam`:
   matching key → `Some(uuid)`; non-matching key → `None`; absent catalogue →
   `None`. Mirror `cli/linear-client/tests/auth.rs`'s `with_catalogue` idiom.
5. **Red** — Jira `resolve_scope`: `None`/`!all_projects` → `Err` containing
   `E_JQL_NO_PROJECT`; a set project → `Ok(scope)` unchanged.
6. **Red** — `cli/work-adapters/tests/sync_create.rs`: a bidirectional run over a
   `RecordingTracker` whose `resolve_scope` refuses (a `refusing_scope(error)`
   seam, mirroring `failing_search`) returns `Err(RunError::DiscoveryUnconfigured)`
   **and fires no `Call::Search` and no push `Call`** — proving nothing was sent.
   A `None`-scope real-tracker path is covered end to end in step 8.
7. **Red** — `cli/work-cli/src/sync.rs`: the `RunError::DiscoveryUnconfigured`
   render arm emits a message carrying the `detail`, and `run` exits
   `exit_codes::UNCONFIGURED` (74).
8. **Red** — the AC-1/AC-4/AC-8 end-to-end regression, in
   `cli/work-adapters/tests/sync_run_real_client.rs` (parameterised per Change 8):
   drive `run()` in preview over a `MockServer` seeded with one untracked team
   issue and `SearchScope { project: Some(TEAM_KEY), .. }`. **The load-bearing red
   assertion is the captured body carrying the resolved `TEAM_ID`** — a
   `MockServer` routes by method+path and does not evaluate the team filter, so
   the seeded issue would surface even with the raw key; today the body carries
   `ENG`, which is what fails. Also assert the `create-from-remote` row. A sibling
   case: a `None`-scope bidirectional run returns
   `Err(RunError::DiscoveryUnconfigured)` (via `execute`'s new `Result` return) and
   records **zero** `POST /graphql` hits against the mock — Linear posts search and
   every mutation to the same `/graphql` route, so "nothing was sent" is checkable
   only as zero requests, not as "no push".
9. **Green** — implement changes 1-8 above.
10. **Refactor** — land the shared `pointer_string` helper from Change 2.

### Success Criteria:

#### Automated Verification:

- [x] `linear-client`/`jira-client` build and lint: `cd cli && cargo clippy -p linear-client -p jira-client --all-targets`
- [x] `resolve_scope` and search tests pass: `cd cli && cargo nextest run -p linear-client resolve_scope search`
- [x] `CatalogueTeam` tests pass: `cd cli && cargo nextest run -p linear-client catalogue_team`
- [x] The `tracker` public-api snapshot is regenerated and verified: `mise run public-api:update && mise run public-api:check`
- [x] The pre-flight refusal and e2e regression pass: `cd cli && cargo nextest run -p work-adapters sync_run_real_client sync_create`
- [x] `RunError::DiscoveryUnconfigured` exit-code test passes: `cd cli && cargo nextest run -p work-cli`
- [x] `cli/linear-cli` builds (the `build_with_override` caller compiles): `cd cli && cargo clippy -p linear-cli --all-targets`
- [x] Workspace check is green: `mise run cli:check`
- [x] Full CLI unit suite passes: `mise run test:unit:cli`

#### Manual Verification:

- [ ] Against a real workspace with `work.default_project_code` set to the team
      key and a seeded untracked issue, `accelerator work sync --preview` lists
      the issue as a `create-from-remote` pull.
- [ ] The emitted GraphQL body (captured via a proxy or debug log) carries the
      team UUID in `{team:{id:{eq:…}}}`, not the raw key.
- [ ] A bidirectional run with the key unset refuses with a `discovery
      unconfigured` message, exits `74`, and sends no push (verify no create hits
      the remote).

---

## Phase 2: Discovery outcome is visible in the report

### Overview

Thread a `DiscoveryStatus` through the report for the runs that get **past** the
Phase 1 pre-flight check, so a push-only skip, a completed search, and a transient
search failure are each visible and distinct from an empty result. Config
refusals never reach here — Phase 1 aborts them as `RunError::DiscoveryUnconfigured`
before the report exists — so this status has no `Unconfigured` variant.

### Changes Required:

#### 1. The `DiscoveryStatus` type

**File**: `cli/work-adapters/src/sync/run.rs`
**Changes**: Add the enum near `RunReport` (`run.rs:123`) and a field on both
`PreparedRun` (`run.rs:639-649`) and `RunReport` (`run.rs:123-129`).

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryStatus {
    Ran { found: usize },
    SkippedPushOnly,
    Failed { detail: String },
}
```

`Failed` is a transient `search` failure (network) — the only error that reaches
this layer, since a config fault was refused pre-flight in Phase 1. It maps to
`RETRYABLE` (70), the fate the taxonomy assigns a failed read. `SkippedPushOnly`
and `Ran` are clean.

#### 2. Set the status where the gate stands

**File**: `cli/work-adapters/src/sync/run.rs:707-725`
**Changes**: Extend Phase 1's discovery block (Change 7) to produce a
`DiscoveryStatus` alongside `untracked`, replacing the `read_failure` fold with a
`Failed` status. The pre-flight `resolve_scope` refusal and the incomplete case
stay as hard `RunError`s.

The `Failed` detail is the tracker error's inner `detail`, not `error.to_string()`:
`TrackerError`'s `Display` prepends "tracker call failed with no remote change: …",
which would read as double-labelled in the report line. Pull the inner string so
the sentinel stands alone. Rather than an inline `let Retryable | Terminal = error`
destructure, add a `TrackerError::into_detail(self) -> String` accessor on the enum
and call it here; `cli/work-cli/src/create.rs:416` already hand-rolls this exact
extraction, so the accessor centralises the two-variant exhaustiveness in one place.

```rust
let (untracked, discovery) = if matches!(
    request.direction,
    SyncDirection::PushOnly
) {
    (Vec::new(), DiscoveryStatus::SkippedPushOnly)
} else {
    let resolved = ports
        .tracker
        .resolve_scope(&request.scope)
        .map_err(|error| RunError::DiscoveryUnconfigured {
            detail: error.detail,
        })?;
    match discover_untracked(ports.tracker, &resolved, request.items) {
        Ok(discovered) if !discovered.complete => {
            return Err(RunError::DiscoveryIncomplete {
                found: discovered.ids.len(),
            });
        }
        Ok(discovered) => {
            let found = discovered.ids.len();
            (discovered.ids, DiscoveryStatus::Ran { found })
        }
        Err(error) => (
            Vec::new(),
            DiscoveryStatus::Failed {
                detail: error.into_detail(),
            },
        ),
    }
};
```

Carry `discovery` through `PreparedRun` into both the preview and apply
`RunReport` constructions (`run.rs:806-810,869-873`).

#### 3. A `failing_search` seam on `RecordingTracker`

**File**: `cli/tracker-test-support/src/lib.rs`
**Changes**: Add a `failing_search(TrackerError)` builder mirroring
`failing_show`/`failing_preview` (`lib.rs:153-165`), and have `search`
(`lib.rs:315`) return the configured error instead of unconditionally `Ok`. The
existing `discovering(found, complete)` seam stays for the success path.

`RecordingTracker` today has no way to make `search` fail, so the Phase 2 error
path (`Failed`, and that a search error no longer folds into `read_failure`) has
no test double to drive. The error must be `Retryable` or `Terminal` to match the
port contract — the removed credential fallback is gone.

#### 4. Render the discovery line

**File**: `cli/work-cli/src/sync.rs:172-205`
**Changes**: In `render_report`, emit one discovery line after the item lines and
before the summary, without disturbing the summary's own emit condition.

```rust
let discovery_line = match &report.discovery {
    DiscoveryStatus::Ran { found } => {
        format!("#\tdiscovery\tran\tfound={found}")
    }
    DiscoveryStatus::SkippedPushOnly => {
        "#\tdiscovery\tskipped\tpush-only".to_owned()
    }
    DiscoveryStatus::Failed { detail } => {
        format!("#\tdiscovery\tfailed\t{}", single_line(detail))
    }
};
lines.sort();
let summary_needed = synced_count > 0 || lines.is_empty();
lines.push(discovery_line);
if summary_needed {
    lines.push(format!("#\tsummary\tsynced\t{synced_count}"));
}
```

Two corrections to the current shape. `lines.sort()` does not order the discovery
line relative to the summary — the summary is pushed after the sort — so append
the discovery line explicitly after the sort to land it below the item lines and
above the summary. And capture `summary_needed` before pushing the discovery line:
an always-present discovery line makes `lines.is_empty()` false, which would
otherwise drop the `#\tsummary\tsynced\t0` line an empty run emits today.

`single_line` is a small helper replacing tab, newline, and carriage return with a
space, so a transport `detail` cannot break the single-record TSV format.

#### 5. Drive the exit code

**File**: `cli/work-cli/src/sync.rs:207-233`
**Changes**: In `exit_code_for_report`, fold a `Failed` discovery into the
existing `RETRYABLE` (70) arm alongside `any_retryable || read_failure.is_some()`.
`Ran` and `SkippedPushOnly` stay clean. No new precedence: a transient discovery
failure is exactly a failed read, which the taxonomy already sends to 70.

```rust
} else if any_retryable
    || report.read_failure.is_some()
    || matches!(report.discovery, DiscoveryStatus::Failed { .. })
{
    exit_codes::RETRYABLE
} else {
    exit_codes::CLEAN
}
```

The config-fault code (`UNCONFIGURED`, 74) is emitted in Phase 1 from
`RunError::DiscoveryUnconfigured`, not here — so `exit_code_for_report`, which only
runs for a successful `RunReport`, never needs to know about it.

#### 6. Reconcile the exit-code documentation

**Files**: `cli/work-cli/src/exit_codes.rs`, `skills/work/sync-work-items/SKILL.md`
**Changes**: `UNCONFIGURED` (74) now has a second source — a pre-flight discovery
`resolve_scope` refusal, not only tracker `SelectionError`. Its "nothing was sent"
invariant **still holds** (the refusal aborts before apply). In `exit_codes.rs`,
extend the `74` doc line to enumerate both sources without weakening the clause,
**and** amend the `72–74` band header (`~:34-36`), which currently attributes the
band solely to `SelectionError` — it must now name the pre-flight discovery
refusal too, or the authoritative table drifts from the code. In
`skills/work/sync-work-items/SKILL.md`, update **both** places 74 is documented —
the Step 0 config-gate branch (`~:51-57`) and the Step 2 exit-code list
(`~:104-118`) — so each names both the credential/selection fault and the
discovery-scope fault (and, for Step 0, the `--push-only` escape hatch); add the
new `#\tdiscovery` report line to the report description. The `70` entry gains "or
a discovery search failed transiently".

### Test-Driven Steps:

1. **Red** — `cli/work-adapters/tests/sync_create.rs`: a push-only run yields
   `report.discovery == SkippedPushOnly` and fires no `Call::Search` (mirror the
   push-only assertion at `sync_create.rs:390`).
2. **Red** — a scoped run over a `RecordingTracker.discovering(found, true)`
   yields `Ran { found }` with the untracked count.
3. **Red** — a scoped run over a `RecordingTracker.failing_search(Retryable)`
   yields `Failed { detail }` where `detail` equals the raw injected inner string
   (no `Display` prefix), and `report.read_failure` is `None`. Depends on the
   seam in Change 3.
4. **Red** — `cli/work-cli/src/sync.rs` tests: `render_report` emits the three
   discovery lines (`ran`, `push-only`, `failed`); the `failed` line **contains
   the injected `detail` substring**; the `#\tsummary\tsynced\t0` line survives an
   empty run; `single_line` strips tab and newline from a `detail`;
   `exit_code_for_report` returns `RETRYABLE` for `Failed` and clean for
   `SkippedPushOnly` and `Ran`.
5. **Green** — implement changes 1-4.
6. **Refactor** — collapse the two `RunReport` constructions (preview and apply,
   `run.rs:806,869`) into a shared `RunReport::from_prepared(...)` builder now
   that the added `discovery` field makes a sixth duplicated field.

### Success Criteria:

#### Automated Verification:

- [x] `work-adapters` and `work-cli` build and lint: `mise run cli:check`
- [x] Discovery-status run tests pass: `cd cli && cargo nextest run -p work-adapters discovery`
- [x] Render/exit-code tests pass: `cd cli && cargo nextest run -p work-cli render_report`
- [x] Full CLI unit suite passes: `mise run test:unit:cli`
- [x] Read-only aggregate is green: `mise run check`
- [x] The updated `skills/work/sync-work-items/SKILL.md` exit-code list matches
      `exit_codes.rs` (checked by eye — there is no automated drift check).

#### Manual Verification:

- [ ] A normal keyed run prints `discovery ran found=N` and exits `0`.
- [ ] A run whose discovery search fails transiently prints a `discovery failed`
      line and exits `70`.
- [ ] A run with zero reported items still prints a `#\tsummary\tsynced\t0` line.
      (The no-key `74` refusal is a Phase 1 concern, verified there.)

---

## Testing Strategy

### Unit Tests:

- `CatalogueTeam` resolution: match, non-match, absent catalogue (Phase 1).
- Linear `resolve_scope`: UUID substitution on a matching key,
  `E_SEARCH_UNKNOWN_TEAM` on an unresolved key, `E_SEARCH_NO_TEAM` on `None`; and
  `search` projects the pre-resolved UUID into every emitted filter (Phase 1).
- Jira `resolve_scope`: `E_JQL_NO_PROJECT` on an unscoped scope, `Ok` unchanged on
  a set project (Phase 1).
- `prepare_run` pre-flight: a refusing `resolve_scope` returns
  `RunError::DiscoveryUnconfigured` with no `Call::Search` and no push; the CLI
  maps it to `74` (Phase 1).
- `DiscoveryStatus` per branch — `Ran`, `SkippedPushOnly`, `Failed` — with the
  `Failed` `detail` carrying the raw inner string, and a search error not
  populating `read_failure` (Phase 2). The `Failed` case uses the new
  `RecordingTracker.failing_search` seam.
- `render_report` lines (three lines, the `failed` line carrying the detail, the
  preserved empty-run summary, `single_line` whitespace stripping) and
  `exit_code_for_report` — `70` for `Failed`, clean otherwise (Phase 2).

### Integration Tests:

- The AC-1/AC-4/AC-8 regression: a mocked-Linear bidirectional discovery with the
  key set surfaces the untracked issue as a `create-from-remote` pull, driven
  through `run()` in `cli/work-adapters/tests/sync_run_real_client.rs`. The
  load-bearing assertion is the captured body carrying the resolved UUID — a
  `MockServer` does not evaluate the team filter, so the raw-key body is what
  fails today.

**AC-7 coverage note.** Because a no-key/unknown-key run is now a hard
`RunError::DiscoveryUnconfigured` (not a soft report status), AC-7's
"report states it was skipped and why" splits across two mechanisms: the
`#\tdiscovery` report line covers the `ran` / `push-only` / transient-`failed`
outcomes (Phase 2), while the config-fault case is covered by the Phase 1 render
test asserting the `74` refusal message names the key. The distinguishability of a
completed-empty search (`Ran { found: 0 }`) from a skip and from a config refusal
is therefore asserted across both phases, not Phase 2 alone.

### Manual Testing Steps:

1. Set `work.integration: linear`, populate `catalogue.json`, set
   `work.default_project_code` to the team key.
2. Ensure a team issue has no local work item.
3. `accelerator work sync --preview` — expect the issue as a `create-from-remote`
   pull and `discovery ran found>=1`.
4. Unset the key and rerun — expect a pre-flight `discovery unconfigured` refusal,
   exit `74`, and **no push applied** (nothing sent).
5. Set the key to a value absent from the catalogue — expect the same pre-flight
   refusal naming the key, exit `74`, nothing sent.

## Performance Considerations

`CatalogueTeam::load` reads one small JSON file per client construction, matching
`CatalogueStates::load`; negligible. Discovery still issues one bounded team
search. No new remote calls.

## Migration Notes

No config or on-disk migration — the plan reads the existing
`work.default_project_code` field and writes no new artefacts.

**Behavioural contract change, release-note it.** An unconfigured discovery scope
now refuses a bidirectional or pull-only run pre-flight (exit `74`) where it
completed cleanly (exit `0`) before — on **both** trackers, so an unscoped Jira
run shifts `0 → 74`. Exit `74`'s meaning broadens from selection/credential faults
to also cover a per-run discovery-scope fault. Any wrapper or CI job that treated a
keyless `work sync` as a clean success will newly see `74`; the workaround is to
set the key or run `--push-only`. Cross-reference the parent epic **0146**, whose
`work.key` rename touches the same field and must account for this exit-code shift.

## Assumptions

- Discovery on Linear is intended, and the configured key is the scope authority;
  the credentialed team is access control only.
- This plan targets `work.default_project_code`. The 0146 rename to `work.key`
  and layered `<tracker>.<entity>_key` ownership reads the same field; whichever
  ships second reconciles — if the rename lands first, retarget the reader; if
  this lands first, the rename accounts for `CatalogueTeam`'s consumer and the
  scope construction at `cli/work-cli/src/sync.rs:431`.
- Team-key matching in the resolver is trimmed exact (not case-folded).
  `in_scope`'s identifier-prefix comparison (`client.rs:198`) is exact but
  **untrimmed** (`prefix == key.as_str()`), so the resolver is the more lenient
  of the two. Both derive the key from the same catalogue `/team/key`; for a
  well-formed key with no surrounding whitespace they agree, and a
  whitespace-bearing catalogue key is out of scope for this fix.

## References

- Original work item: `meta/work/0220-untracked-remote-discovery-never-runs-on-linear.md`
- Related research: `meta/research/codebase/2026-08-30-0220-untracked-remote-discovery-never-runs-on-linear.md`
- Port precedent: `cli/linear-client/src/filter.rs:18-40`, `cli/linear-client/src/catalogue.rs`
- Discovery gate: `cli/work-adapters/src/sync/run.rs:707-725`
- Key→UUID seam (moves into `resolve_scope`): `cli/linear-client/src/client.rs:661-686`
- Pre-flight abort point (`prepare_run` before apply): `cli/work-adapters/src/sync/run.rs:788`
- Exit-code taxonomy: `cli/work-cli/src/exit_codes.rs`
- Parent epic: `meta/work/0146-work-item-sync-enhancements.md`
