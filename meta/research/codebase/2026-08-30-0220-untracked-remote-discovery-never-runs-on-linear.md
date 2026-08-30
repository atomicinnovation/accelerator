---
type: codebase-research
id: "2026-08-30-0220-untracked-remote-discovery-never-runs-on-linear"
title: "Research: Untracked-Remote Discovery Never Runs on Linear (0220)"
date: "2026-08-30T17:00:35+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
topic: "Why untracked-remote discovery is skipped on Linear and where the gate, scope, and key resolution live"
work_item_id: "0220"
parent: "work-item:0220"
tags: [research, codebase, sync, linear, tracker, discovery]
revision: "3cc3fe422551f0fd064105dc12e2f1f6082a67c5"
repository: "accelerator"
last_updated: "2026-08-30T17:00:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Untracked-Remote Discovery Never Runs on Linear (0220)

**Date**: 2026-08-30T17:00:35+00:00
**Author**: Toby Clemson
**Git Commit**: 3cc3fe422551f0fd064105dc12e2f1f6082a67c5
**Branch**: working copy (no bookmark), workspace `ticket-management`
**Repository**: accelerator

## Research Question

Ground the bug in `meta/work/0220-untracked-remote-discovery-never-runs-on-linear.md`
against the live codebase: verify every cited site, confirm the mechanism, and
surface anything the work item mis-states or omits that a plan must account for.

## Summary

The work item is accurate on mechanism and mostly accurate on sites. The
discovery gate at `cli/work-adapters/src/sync/run.rs:707-709` disables
untracked-remote discovery unless `scope.project.is_some() || scope.all_projects`,
and `scope.project` is `None` on Linear because the config key
`work.default_project_code` is typically unset. The gate encodes Jira's JQL
"no project = unbounded" semantics, which Linear does not share — Linear
self-bounds to the credentialed team. Both the empty-`else` skip and a completed
search that finds nothing collapse to the same empty `Vec<ExternalId>`, so the
report cannot tell them apart. All confirmed.

Three corrections and one sharpening matter for planning:

- **The scope type is `tracker::SearchScope`, not `SyncScope`.** No `SyncScope`
  type exists anywhere in `cli/`. The work item and its Technical Notes call the
  runtime carrier "scope" loosely; the concrete type is `SearchScope`
  (`cli/tracker/src/lib.rs:244-255`), and `SyncRequest.scope` holds it.
- **There is no general team-key→UUID map in the catalogue.** The catalogue
  models exactly *one* team, exposing `/team/key` and `/team/id` as two
  independent field reads. The fix is not "look up an arbitrary key"; it is
  "recognise that `scope.project` names the one configured team, and substitute
  that team's UUID". `LinearClient` already holds `team_key` at
  `client.rs:124/156` for exactly this comparison.
- **The `RemoteTracker` trait has zero capability methods today and zero default
  methods.** Adding `self_bounds_search(&self) -> bool` either forces an impl in
  all five `impl RemoteTracker` blocks (matching the all-required style) or
  introduces the trait's first default method. This is a design choice the plan
  must make explicitly.
- **The regression-test gap is real and precise.** No mocked-Linear test
  exercises the untracked-search sync path. `sync_run_real_client.rs:238-243`
  documents that the default scope disables discovery, so it deliberately never
  searches. The new test combines two existing idioms rather than copying one.

## Detailed Findings

### The discovery gate and scope construction

The gate is in `prepare_run` (`cli/work-adapters/src/sync/run.rs:659`),
lines 707-725:

```rust
let discovery_enabled =
    !matches!(request.direction, SyncDirection::PushOnly)
        && (request.scope.project.is_some() || request.scope.all_projects);
let untracked = if discovery_enabled {
    match discover_untracked(ports.tracker, &request.scope, request.items) {
        Ok(discovered) if !discovered.complete =>
            return Err(RunError::DiscoveryIncomplete { found: discovered.ids.len() }),
        Ok(discovered) => discovered.ids,
        Err(error) => { read_failure = read_failure.or(Some(error)); Vec::new() }
    }
} else {
    Vec::new()
};
```

`request.direction` is `SyncDirection` (`cli/work/src/sync/decide.rs:6-10`;
variants `Bidirectional`, `PushOnly`, `PullOnly`). The gate suppresses discovery
only for `PushOnly` — so the bug requires a bidirectional or pull-only run, as
the work item's reproduction states. `discover_untracked`
(`run.rs:420-441`) calls `tracker.search(scope)` and subtracts the local
`external_id` set from the remote `found` set.

The scope is built in `cli/work-cli/src/sync.rs:431-438`:

```rust
let default_project =
    crate::config::effective_nonempty(config, "work.default_project_code")
        .unwrap_or_default();
let scope = tracker::SearchScope {
    project: (!default_project.is_empty()).then_some(default_project),
    all_projects: false,
    filters: Vec::new(),
};
```

`scope.project` is the raw config string, passed straight through to the
tracker's `search` with no id resolution. `all_projects` is hardcoded `false`.

**Correction — the type.** `SearchScope` is defined at
`cli/tracker/src/lib.rs:244-255` with fields `project: Option<String>`,
`all_projects: bool`, `filters: Vec<(String, String)>`. Its doc records the
precedence: when both `project` and `all_projects` are set, `project` wins.
`SyncRequest.scope` (`run.rs:101`) carries it. Grep for `SyncScope` returns
nothing — the work item's naming is informal.

### No report line distinguishes skip from empty

The empty-`else` (`run.rs:723-725`) and a completed search returning an empty set
(`run.rs:717`, `Ok(discovered) => discovered.ids` with `ids` empty) produce the
identical value: an empty `untracked: Vec<ExternalId>`. `render_report`
(`cli/work-cli/src/sync.rs:172-205`) iterates only `report.reported`; untracked
issues surface only as `create-from-remote` rows. With an empty set, no
discovery row is emitted in either case. The only discovery-specific message is
the *failure* path `RunError::DiscoveryIncomplete` (`sync.rs:509-517`), which
fires only when a search was truncated. This confirms the observability gap the
work item folds in — the silence is what kept the defect invisible.

### The tracker port has no capability surface

`RemoteTracker` (`cli/tracker/src/lib.rs:321`) declares seven required methods —
`create`, `update`, `show`, `fetch_all`, `search` (`:427`), `preview_create`,
`validate_update` — all operation-performing, all ending in `;` with no default
body. There is no introspection or capability method anywhere on the trait, and
no default method precedent to copy. `search` takes `&SearchScope` and returns
`Discovery { found, complete }`.

Five impls exist: `JiraClient` (`cli/jira-client/src/client.rs:469`),
`LinearClient` (`cli/linear-client/src/client.rs:585`), and three test doubles —
`RecordingTracker` (`cli/tracker-test-support/src/lib.rs:203`), `FixedTracker`
(`cli/tracker/tests/port.rs:65`), `Fake` (`cli/work-adapters/tests/sync_apply.rs:102`).

**Design choice for the plan.** A `self_bounds_search(&self) -> bool` capability
sits naturally right after `search` (`lib.rs:427`). As a *required* method it
matches the trait's uniform style but touches all five impls. As a *default*
method (`{ false }`) it touches only Linear's impl but becomes the trait's first
default — a style divergence. Either sets a new precedent; there is nothing to
imitate.

The registry (`cli/work-cli/src/tracker_registry.rs:167-187`) matches on the
`work.integration` string: `jira` and `linear` are wired; `trello` and
`github-issues` are recognised but return `SelectionError::NotAvailable`. So two
trackers ship; a future self-bounding adapter would hit this same gate.

### Linear self-bounds, but the key never becomes a UUID

`LinearClient::search` (`cli/linear-client/src/client.rs:661-666`):

```rust
fn search(&self, scope: &SearchScope) -> Result<Discovery, TrackerError> {
    let mut search = Search {
        team_id: scope
            .project
            .clone()
            .or_else(|| Some(self.credentials().team_id.clone())),
        ..Search::default()
    };
```

Two branches feed the same `Search.team_id` field with different *kinds* of
value. `scope.project` is used verbatim (a team **key**, e.g. `"PP"`); the
fallback `credentials().team_id` is a genuine team **UUID** resolved at
construction via `resolve_team` (`auth.rs:81`, reading catalogue pointer
`/team/id`). `compose` (`cli/linear-client/src/filter.rs:64-71`) emits whatever
it holds into `{"team": {"id": {"eq": team}}}`, which Linear compares against the
team UUID. So `"PP"` matches no team, discovery returns empty, and — unlike the
state path, which refuses unknown names via `ClientError::UnknownState`
(`filter.rs:72-80`) — nothing errors. This is the "compounding trap": the fix
that only flips the gate would still return zero, silently, for a second reason.

**Correction — no arbitrary key→UUID lookup exists.** The catalogue
(`<integrations_root>/linear/catalogue.json`) models one team.
`catalogue_team_key` (`auth.rs:97`, pointer `/team/key`) and `catalogue_team`
(`auth.rs:101`, pointer `/team/id`) are independent single-field reads; there is
no `key == "PP" → id` function. The resolution the fix needs is: treat a
`scope.project` equal to the catalogue team key as the configured team and
substitute its UUID (equivalently, fall through to `credentials().team_id`). The
seam is the single assignment at `client.rs:663-666`. `LinearClient` already
holds `team_key: Option<String>` (`client.rs:124`, populated at `client.rs:156`),
so the comparison value is in hand. A key that matches no catalogue team must
surface as an error, not an empty result (AC 5).

### Jira's gate is correct — for Jira

`compose` in `cli/jira-client/src/jql.rs:169-174` refuses an unscoped query:

```rust
if search.project.is_none() && !search.all_projects {
    return Err(ClientError::BadJql {
        reason: "E_JQL_NO_PROJECT: specify a project or all_projects".to_owned(),
    });
}
```

`compose` emits `project = <quoted>` only when `search.project` is `Some`
(`jql.rs:177-179`); an absent project produces genuinely unbounded JQL — every
issue the credentials can see. So the refusal is correct *for Jira specifically*.
`JiraClient::search` (`client.rs:591-593`) delegates to `discover`
(`client.rs:265-285`), which copies the scope field-for-field into `jql::Search`
and calls `compose`. The gate's correctness rests entirely on Jira's semantics,
which is precisely why moving it behind a tracker capability is the right shape.

### Test idioms for the regression

The discovery gate is exercised in `cli/work-adapters/tests/sync_create.rs`
against `RecordingTracker`, seeded via `.discovering(found, complete)`
(`tracker-test-support/src/lib.rs:165-173`). Representative cases: a push-only
run asserts no `Call::Search` fired (`sync_create.rs:390`); an over-threshold set
yields `RunError::Refused` (`:318`); a scoped pull-only happy path (`:437`); an
incomplete discovery is refused (`:481`). The enabling tests use a `scoped()`
helper returning `SearchScope { project: Some("ENG"), .. }` (`:242`).

Mocked Linear uses `http_test_support::MockServer` with `client_for(&server,
config)` (`cli/linear-client/tests/support/client.rs:37`, constants
`TEAM_KEY = "ENG"` / `TEAM_ID` UUID). `search_projection.rs:40-101` registers a
paged `Route::Sequence`, runs `search_detailed`, then asserts on
`server.bodies(...)`. Filter shape is pinned by a committed TSV fixture
`cli/linear-client/tests/fixtures/issue-filter.txt` — the team row is
`team	team=T1	{"team":{"id":{"eq":"T1"}}}`, and a coverage guard
(`filter.rs:80-98`) fails if any family loses its row. Catalogue resolution is
tested in `cli/linear-client/tests/auth.rs` via a `with_catalogue` helper writing
`{"team":{"id":"…","key":"ENG"}}`.

**The gap.** `sync_run_real_client.rs:238-243` is Jira-backed and documents that
the default scope disables discovery, so it never searches. No test drives the
untracked-search sync path against a mocked Linear. The AC-8 regression test is
new coverage combining the `MockServer`/`client_for` idiom with `sync_create.rs`
gate assertions — there is no single template to copy.

## Code References

- `cli/work-adapters/src/sync/run.rs:707-725` — the discovery gate and empty-`else`
- `cli/work-adapters/src/sync/run.rs:420-441` — `discover_untracked`
- `cli/work-adapters/src/sync/run.rs:101` — `SyncRequest.scope: SearchScope`
- `cli/work-cli/src/sync.rs:431-438` — scope construction from `work.default_project_code`
- `cli/work-cli/src/sync.rs:172-205` — `render_report` (no skip-vs-empty distinction)
- `cli/tracker/src/lib.rs:244-255` — `SearchScope` definition (not `SyncScope`)
- `cli/tracker/src/lib.rs:321,427` — `RemoteTracker` trait and `search`
- `cli/work-cli/src/tracker_registry.rs:167-187` — tracker dispatch (jira/linear wired)
- `cli/linear-client/src/client.rs:661-666` — the key→UUID seam (raw pass-through)
- `cli/linear-client/src/client.rs:124,156` — `team_key` already held
- `cli/linear-client/src/filter.rs:64-71` — `{team:{id:{eq}}}` emission
- `cli/linear-client/src/auth.rs:81,97,101` — catalogue readers (single team)
- `cli/jira-client/src/jql.rs:169-174` — `E_JQL_NO_PROJECT` refusal
- `cli/work-adapters/tests/sync_create.rs:242,318,390,437,481` — gate test idioms
- `cli/work-adapters/tests/sync_run_real_client.rs:238-243` — documents the default-scope skip
- `cli/linear-client/tests/fixtures/issue-filter.txt:17` — team filter-shape row

## Architecture Insights

- **The gate lives in tracker-agnostic code but encodes one tracker's law.** The
  clean fix moves the "must this search be project-scoped?" question behind a
  `RemoteTracker` capability, so each provider answers for itself. This mirrors
  the existing division: the `tracker` crate is a pure port, provider law lives
  in the client crates.
- **Two kinds of team identity flow through one field.** `Search.team_id`
  carries a UUID on the fallback branch and a key on the scoped branch. The
  asymmetry with the state path — which *does* resolve names to UUIDs and refuses
  unknowns — is the root of the silent failure. Bringing team resolution up to
  the state path's standard (resolve-or-refuse) is the principled fix.
- **The catalogue is a single-team artifact.** Any design that assumes a
  multi-team key→UUID map is over-built for today's model. The 0146 config-model
  redesign (`work.key`, layered `<tracker>.<entity>_key`) may change this; 0220
  deliberately targets the current single field.

## Historical Context

- `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md` and
  `meta/plans/2026-08-11-0204-remote-tracker-port.md` — the port's original
  design; the seven-method shape and dyn-compatibility rationale originate here.
- `meta/research/codebase/2026-08-12-0194-tracker-crate-and-remote-sync-engine.md`
  — the sync engine and `SearchScope`/`Discovery` surface.
- `meta/research/codebase/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`
  — Jira/Linear client crates; where `compose` and the catalogue readers landed.
- `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md` — remote
  identity lives in `external_id`; relevant to the id-vs-key distinction 0220
  turns on.
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md` — why the gate
  belongs in the CLI/port, not the skill.
- `meta/work/0146-work-item-sync-enhancements.md` — parent epic owning the
  `work.key` / `<tracker>.<entity>_key` config-model redesign; no research or
  plan artifact yet.

Auto-memory corroborates the symptom: "Sync untracked-pull scope — the whole
multi-team Linear workspace floods (997+); abort the gate, only push." That note
records the flood 0220's required-key design prevents.

## Related Research

- 0204 (remote tracker port), 0194 (tracker crate + sync engine), 0210 (provider
  client crates), 0211 (integration binaries / Linear-Jira wiring), 0213
  (conflict-resolution flow). 0220 is the first shippable child of 0146.

## Open Questions

- **Capability method shape** — required (touch five impls) or default (trait's
  first default method, touch one)? A plan decision, not a code fact.
- **Where team-key→UUID resolution sits** — inline comparison against
  `self.team_key` at `client.rs:663`, or a new `TeamResolver` port mirroring
  `StateResolver`? The inline form is smaller; the port form matches the state
  precedent and is more testable. ❓ Needs the plan's call.
- **Config-model ordering with 0146** — 0220 reads `work.default_project_code`;
  whichever of {0220, the `work.key` rename} lands second must reconcile. Not a
  code question, but the plan should state the assumption.
- **`all_projects` on Linear** — hardcoded `false` and untouched by this fix. Is
  a Linear "all teams" discovery ever wanted, or is single-team-required the
  permanent contract? Out of 0220's scope; flag for 0146.
