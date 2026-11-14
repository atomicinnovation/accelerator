---
type: "plan"
id: "2026-11-13-0371-shortcut-integration"
title: "Shortcut Integration Implementation Plan"
date: "2026-11-13T16:10:27+00:00"
author: "Tom Scott"
producer: "create-plan"
status: "draft"
work_item_id: "work-item:0371"
parent: "work-item:0371"
relates_to: ["work-item:0181", "work-item:0171", "work-item:0290", "plan-review:2026-11-13-0371-shortcut-integration-review-1"]
tags: ["integrations", "shortcut", "tracker"]
revision: "654a72f616fcebe9e253fdc9834d75a6b227a061"
repository: "accelerator"
last_updated: "2026-11-13T22:14:43+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---

# Shortcut Integration Implementation Plan

## Overview

We add Shortcut (shortcut.com) as a third remote tracker beside Jira and
Linear. A user sets `work.integration: shortcut` and gets the same things the
other two trackers give: `/sync-work-items` reconciles local work items with
Shortcut stories and epics, and eight `*-shortcut-*` skills show, search,
create, update, transition, comment on and attach links to stories through a
dispatched `accelerator-shortcut` binary.

The connector targets Shortcut's REST API **v4** (beta). It stores story keys
in the `sc-1234` form and epic keys in the `epic-56` form. Sync carries the
title and body. `create` sends a work item of kind `epic` to a Shortcut epic,
and maps every other kind to a story's `story_type`. Attach adds URL links
only.

## Current State Analysis

The connector slots into a design that already serves two trackers:

- The sync engine sees only `Box<dyn RemoteTracker>`
  (`cli/tracker/src/lib.rs:669-900`). The port carries the title, the body and,
  on create, the kind. Status, priority, tags and parent never cross it.
- A client crate implements the port. `linear-client` (about 7,000 source
  lines, 359 tests) and `jira-client` (about 4,600 lines) are the two
  precedents.
- `cli/tracker-test-support/src/contract.rs` holds one property suite that
  every tracker passes. Each client runs it offline against a mock
  (`tests/contract_offline.rs`) and live against a tenant (`tests/contract.rs`,
  excluded from the default nextest run).
- There is no tracker registry. Each site that varies by tracker matches on
  the integration name as a string, and most have a catch-all arm. A missed
  site compiles and falls back silently.
- A dispatched sub-binary (`accelerator-linear`) serves the per-tracker skills.
  Its registration follows the fourteen-point checklist in
  `tasks/README.md:790-959`.
- Eight skills per tracker live under `skills/integrations/<tracker>/`. One
  `plugin.json` entry registers each directory
  (`.claude-plugin/plugin.json:15`).

Shortcut differs from both precedents in four ways that matter:

1. **Identity.** Shortcut story ids are bare integers. Every shared key check
   requires `[A-Za-z][A-Za-z0-9_]*-[0-9]+` (`is_tracker_key`,
   `cli/corpus/src/work_item_id.rs:310-319`). Users already see stories as
   `sc-1234` in branch names and commit links, and that form passes the check.
   Epics are a separate entity with their own endpoints and no user-visible
   key form.
2. **Transport.** v4 is REST with `Authorization: Bearer` tokens. Every path
   except `GET /api/v4/whoami` carries the workspace slug:
   `https://api.app.shortcut.com/api/v4/{slug}/...`.
3. **Create needs a workflow state.** `POST /stories` requires
   `workflow_state_id`. Each team names a default workflow, and a story may
   sit in any workflow its team uses.
4. **Teams have no key.** A v4 team has a uuid, a `name` and a nullable
   `mention_name`. Search's `team:` operator takes the display name.

### Key Discoveries

- The factory is `ConfiguredTrackers::resolve`
  (`cli/work-cli/src/tracker_registry.rs:259-291`). Its `SelectionError`
  messages list the recognised trackers (`:42-49`).
- `WORK_INTEGRATION_VALUES` (`cli/config/src/catalogue.rs:125-126`) gates
  every `work` read. `trello` and `github-issues` are listed there and
  resolve to `SelectionError::NotAvailable`, which exits 72
  (`cli/work-cli/src/exit_codes.rs:192`).
- `jira-client/src/classify.rs:50-75` classifies by outcome and operation. On
  a write, 400, 401, 403, 404, 410 and 429 prove nothing was applied and give
  `Retryable`. A refusal before send gives `Rejected`. Everything else gives
  `Terminal`. The port documents the same rule (`cli/tracker/src/lib.rs:685-711`).
- `LinearClient::from_config` only loads the catalogue. Work item 0336 keeps
  runtime catalogue lookups out of non-init commands "so non-init commands make
  no extra request" (`meta/work/0336-…md:158-159`).
- `linear-cli/src/context.rs:49-61` reads `ACCELERATOR_LINEAR_API_URL` and
  passes `cfg!(feature = "test-loopback")` to the client's `url_is_allowed`.
  The feature lives on the binary crate.
- `tracker_test_support::evidence` accepts only reduced `PASS|FAIL` records.
  Its `SECRET_SHAPES` (`evidence.rs:96`) lists `ATATT`, `lin_api_` and
  `Bearer `.
- `a_moved_issue_is_found_only_under_its_current_key_property` calls `expect`
  on `moved_issue()` (`contract.rs:430-432`). Only `run_all` and
  `timed_conformance` skip it when the subject returns `None`.
- `validate_update_reports_outcome_property` passes the id `PREVIEW-1` and
  expects `Valid` (`contract.rs:564`).
- `tests/unit/tasks/test_sync_work_items_skill_structure.py:268` pins the sync
  skill's key regex `^[A-Z][A-Z0-9_]*-[0-9]+$`.
- `remote-projection` is a pinned public-API crate, and its `Integration` enum
  is exhaustive.
- `cli/tracker/Cargo.toml:13` declares a closed downstream set
  `["work", "jira", "linear"]`.

### Recorded lessons this plan applies

- **0338:** an update sends exactly the fields it is given, in one request.
- **0336:** non-init commands make no catalogue requests and write no
  catalogue. Only `init discover` writes it.
- **0296:** the tracker may rewrite Markdown. We record Shortcut's rewrites
  from live evidence before trusting body equivalence.
- **0220:** generic engine code asks the tracker through `resolve_scope`
  instead of assuming a Jira-shaped scope.
- **0171 D7:** each client installs the ring crypto provider in its
  constructor. Mocks do not detect a missing provider.
- **0171 D10:** the live contract test is named `tests/contract.rs`, so the
  default nextest filter excludes it.
- **0048 review:** a client never resends a non-idempotent write on an
  ambiguous failure.

## Desired End State

- `work.integration: shortcut`, a `shortcut.team_key` and a token in
  `.accelerator/config.local.md`, followed by `/accelerator:init-shortcut`,
  make `accelerator work sync` push, pull and discover Shortcut stories.
- Synced work items carry `external_id: "sc-<id>"` for a story, or
  `external_id: "epic-<id>"` for an epic.
- `accelerator shortcut {init,show,search,create,update,comment,transition,attach}`
  works, and eight skills drive it.
- The offline contract suite passes in the default profile for a story subject
  and an epic subject. The live suite passes against a scratch workspace
  through `mise run test:integration:tracker-contract`.
- `mise run` exits 0.
- The docs site has a Shortcut tab on the issue-trackers page and a page for
  the sub-binary.

Verify the end state with `mise run` and the manual steps under Testing
Strategy.

## What We're NOT Doing

- Status, priority, label, owner or estimate sync. Work item 0290 owns field
  mapping, and 0321 owns the authority model.
- Parent/child sync. A story's `epic_id` and `parent_story_id` are not set
  from the local `parent` field. Work item 0291 owns it.
- Epics in discovery. `RemoteIssue` carries no kind, so a pulled epic would
  arrive with the default local kind. Discovery searches stories only until
  work item 0290 carries the kind across the port.
- File upload. `attach` adds links through `external_links_add`. A follow-up
  work item covers multipart upload.
- `id_pattern: "{tracker}"` for Shortcut. That would extend ADR-0070 and needs
  its own decision.
- Shortcut objectives and iterations as local work items.
- Epic states. Epics keep the default state Shortcut gives them, and
  `transition` works on stories only.
- Switching an existing corpus from another tracker to Shortcut. Keys from the
  old tracker fail item by item, never run-wide. A Jira key such as `SC-12`
  would parse as a Shortcut key, so a corpus must not change tracker in place.
- Automatic catalogue healing. Only `init discover` writes the catalogue, so
  Shortcut needs no run finaliser.
- The v3 API, or a fallback to it.
- A shared `cli-support` crate, or a tracker registry that replaces the
  string matches. Phase 7 records both as follow-up work items.

## Implementation Approach

We follow the order the Linear connector took through work items 0210 and
0211. That order is: shared seams, then the client, then the composition
root, then the binary and skills. Each phase is one pull request that leaves
`mise run` green and ships nothing half-wired.

The phases merge in the order 1, 2, 3, 5, 4, 6, 7. Phase 4's setup error names
`/accelerator:init-shortcut`, and Phase 5 ships that skill, so Phase 5 merges
first. The phase numbers follow the layers, not the merge order.

Every behaviour change starts as a failing test. Client tests drive a
`http_test_support::MockHTTPServer` keyed on `(method, path)`, as
`jira-client` does; GraphQL test support does not apply. Tests that assert
"not resent" or "retried" count the mock's recorded requests.

We keep every v4 path in one module, `shortcut_client::paths`. v4 is beta, so a
breaking change should touch one place.

### Tolerating a beta API

- Response and catalogue models never use `deny_unknown_fields`, so a new
  field is ignored.
- Open sets carry an `Other(String)` variant: workflow-state `type`,
  `story_type` and the error `tag`.
- `catalogue.json` is shared by teammates who may run different plugin
  versions, so it changes additively:
  - every catalogue record keeps the fields it does not know in a serde
    `flatten` map, and `init discover` merges them back on rewrite, as
    `linear-client/src/catalogue/document.rs:629-649` does;
  - `schema_version` changes only for a breaking change, never for a new
    field;
  - a newer `schema_version` than the client knows is `Unconfigured`, naming
    the plugin upgrade;
  - an older `schema_version` is `Unconfigured`, asking for `init discover`.

### Kind to Shortcut entity

The kind model in `new_entity.rs` is the one home of this table. It answers
two questions:
- `NewEntity::for_kind(kind)` gives the entity `create` makes. The client's
  `create` and `preview_create` and the CLI's `create` all call it.
- `story_type_for_kind(kind)` gives the story type a search filter matches,
  or nothing for `epic`, an empty kind or an unknown kind.

| Local `kind` | Shortcut entity | Preview |
|---|---|---|
| `story` | story, `story_type: feature` | `Resolved("feature")` |
| `bug` | story, `story_type: bug` | `Resolved("bug")` |
| `task` | story, `story_type: chore` | `Resolved("chore")` |
| `spike` | story, `story_type: chore` | `Resolved("chore")` |
| `epic` | epic | `Resolved("epic")` |
| empty | story, `story_type: feature` | `Unset` |
| anything else | — | `Unresolvable(kind)`; `create` returns `Rejected` before sending |

### Shortcut keys

`ShortcutKey` is the domain type for a remote key. It has two variants:

- `Story(StoryId)` renders as `sc-<id>`, matching Shortcut's own branch and
  commit convention.
- `Epic(EpicId)` renders as `epic-<id>`. Shortcut has no user-visible epic
  key, so this form exists only in work items.

`parse` accepts either prefix in any case. It refuses leading zeros, so each
entity has exactly one numeric form. `ShortcutKey::collection()` gives
`stories` or `epics`, and `paths` consumes it, so no port method matches on
the variant itself. `ShortcutKey::into_story()` gives a `StoryId`, or the
domain error `NotAStory(EpicId)`. `shortcut-cli` turns that error into a usage
message that lists its own epic-capable subcommands.

The kind decides the entity only at `create`. After that the key decides it,
because every later port call gets the key alone. A local `kind` edited after
creation changes nothing remote.

The client always reports an entity under the `ExternalId` it was asked for.
A requested `SC-12` comes back as `SC-12`, not `sc-12`, so `fetch_all`'s
partition stays total and `show` never reports a key change.

A key `ShortcutKey` refuses is handled per operation:

| Operation | Result |
|---|---|
| `show`, `locate`, `search` | `Retryable`, naming the key |
| `fetch_all` | the id is `indeterminate` |
| `create`, `update`, `retitle` | `Rejected` before sending |
| `validate_update` | not parsed: the check is local to title and body |

### Failure classification

`classify(Delivery, Operation, detail)` mirrors
`jira-client/src/classify.rs:50-75`, and uses its `Outcome` variant names.
`Operation` names each port and surface call and knows whether it mutates.
The transport returns a `Delivery`, which holds the last attempt's `Outcome`
and whether any attempt may have applied the request.

| Outcome | Read | Write |
|---|---|---|
| `NotSent` (the connection failed, so no byte was sent) | `Retryable` | `Retryable` |
| `RequestInvalid` (the client refused before sending) | `Retryable` | `Rejected` |
| `Status(400 \| 401 \| 403 \| 404 \| 409 \| 410 \| 422 \| 429)` | `Retryable` | `Retryable` |
| any other `Status`, including 5xx | `Retryable` | `Terminal` |
| `Transport` (timeout or reset after send) | `Retryable` | `Terminal` |
| `NonJsonBody` (a 2xx whose body is not JSON) | `Retryable` | `Terminal` |
| `UnusableBody` (a 2xx JSON body without a field the operation needs) | `Retryable` | `Terminal` |

A response that is not 2xx classifies by its `Status`, whatever its body. A
proxy's HTML 403 is `Status(403)`, not `NonJsonBody`. The operation that
decodes a 2xx body produces `UnusableBody` itself, so `classify` never parses a
body. For example, a 201 create with no usable `id` is `UnusableBody`.

The status list is explicit. Each listed status is one Shortcut uses to reject
a request before applying it, and Shortcut documents that a 429 "will not be
processed". A status outside the list counts as possibly applied.

`Delivery::may_have_applied` covers the whole retry loop, not only the last
attempt. The transport sets it when any attempt of a write ends in 5xx,
`Transport` or `NonJsonBody`. When it is set, a write is `Terminal` whatever
the last attempt returned. So a PATCH that got a 5xx and then a 429 is
`Terminal`, because the first attempt may have applied.

Filter and scope refusals never go through `classify`. `resolve_scope` returns
`ScopeError`, and `search` returns `Unconfigured`, as the port requires
(`cli/tracker/src/lib.rs:805-829`). Both lead to exit 74, which tells the user
to fix their configuration.

Each message names the actor and the fix:
- a 401, or no token, names the three token sources in precedence order and
  the Shortcut API-token settings page;
- a 403 names the missing scope, and says a read-only token cannot write when
  whoami reports only the read scope;
- a token with a `Bearer ` prefix is refused locally, naming the prefix.

The client reads scopes from whoami's `authorization.scopes`. It never slices
the token string.

### Resend policy

Each request declares a `Resend` policy, with the variant names and rules of
`jira-client/src/transport.rs:340-352`. The transport resends only on a
status. It never resends after `NotSent`, `Transport` or `NonJsonBody`.

| Request | Policy |
|---|---|
| every `GET` | `WhileRetryable`: resend on 429 and 5xx |
| `PATCH` that sets fields (`update`, `retitle`, `transition`) | `WhileRetryable` |
| `POST` (`create`, `comment`) | `OnlyWhenThrottled`: resend on 429 only |
| `PATCH` with `external_links_add` (`attach`) | `OnlyWhenThrottled` |

`attach` stays `OnlyWhenThrottled` whatever Shortcut does with a repeated
link, because a resent attach saves the user very little.

### Catalogue

`<paths.integrations>/shortcut/catalogue.json` is committed and team-shared,
like Linear's. It holds:

- `schema_version`, the workspace slug and the workspace name;
- the teams, keyed by uuid, each with its name, mention name, default
  workflow id and the ids of every workflow the team uses;
- every workflow from `GET /workflows`, with its states: id, name, type and
  position.

Rules:

- Only `init discover` writes the catalogue. It writes under the mkdir lock
  from the `store` crate, through an atomic write.
- `init discover` writes only a complete catalogue. It follows each
  `next_page_url` to the end with no page cap, so only its deadline can cut a
  listing short. A listing whose `next_page_url` repeats is a fault, not a
  cut. In either case it writes nothing, names the listing and the cause, and
  says to retry. So there is no "incomplete" catalogue state.
- `init discover` refuses to replace a catalogue whose `schema_version` is
  newer than its own, with or without `--force`. It names the plugin upgrade.
  So a teammate on an older plugin cannot downgrade the shared file.
- `ShortcutClient::from_config` reads the catalogue and makes no request. An
  absent catalogue is `Unconfigured`. The message says to pull the catalogue
  a teammate committed, or else to run `/accelerator:init-shortcut`.
- On load, the client validates every value it puts in a request path:
  - the slug matches `^[a-z0-9][a-z0-9-]*$`;
  - team ids are uuids;
  - workflow and state ids are positive integers.

  A value that fails is `Unconfigured`, naming the field and asking for
  `init discover`.
- `paths` adds the slug and ids as path segments through
  `Url::path_segments_mut`, never by string concatenation.
- The catalogue does not check team names. A team name enters a request only
  as a search value, and `filter.rs` checks it there.

### Team identity

A team's **team key** is its mention name. A team with no mention name has
its display name as its team key. The uuid is the team's identity.

`cli/shortcut-client/src/team_key.rs` is the one home of this rule:
- `TeamKey::of(mention_name, name)` derives a team's key. The catalogue's team
  record and the live team model from `GET /teams` both call it.
- `TeamKeys::over(teams)` builds the keys for a list of teams, and finds every
  **colliding key**: a key two teams share. That includes one team's display
  name matching another team's mention name.

Rules:
- `shortcut.team_key` and `shortcut.pull.additional_teams` take team keys.
  `init list-teams` shows them, and `init discover` writes one.
- `auth::team_key(config)` reads `shortcut.team_key` and nothing else. It
  returns `None` when the key is absent.
- `Catalogue::team_for_key(&TeamKey)` resolves a team key to a catalogued
  team.
- `enumerate_visible_entities` gives one `VisibleEntity` per team, with the
  uuid as `identifier`. Its `key` is the team key, or the uuid for a team whose
  key collides. So the engine's exact-key resolver
  (`work-adapters/src/sync/scope.rs:112-129`) never sees two entities with one
  key, and `all_teams` still searches every team.
- `resolve_scope` follows `LinearClient::resolve_scope`
  (`linear-client/src/client.rs:1017-1031`):
  - an absent `shortcut.team_key` is `ScopeError`, naming the key and
    `accelerator shortcut init list-teams`;
  - every configured team key, base and additional, that collides in the
    catalogue is `ScopeError`;
  - for a base-only scope, it replaces the team key with the uuid from
    `Catalogue::team_for_key`, and an unknown key is `ScopeError`, listing
    the known team keys;
  - for a broadened scope, it leaves the keys as configured, as the port
    requires (`cli/tracker/src/lib.rs:821-824`). The engine checks them
    against the live enumeration;
  - with a `state` filter on a broadened scope, an additional team key missing
    from the catalogue is `ScopeError`. The message names the team and
    `accelerator shortcut init discover`, because the client cannot check the
    state against that team's workflows. Without a `state` filter, broadened
    keys are not checked against the catalogue.

A collision message, and a refused team name in `filter.rs`, each give the
full recovery:
1. In Shortcut, give the team a mention name, or rename it.
2. Run `accelerator shortcut init discover`.
3. Set `shortcut.team_key` or `additional_teams` to the team's new key.
4. Commit the catalogue.

A mention name rarely changes, so renaming a team in Shortcut leaves the
configuration working.

### Observability

The transport and client emit `tracing` events at the levels `linear-client`
uses:
- `debug` for each retry, with the attempt number and the backoff;
- `warn` for a truncated read at a page cap, or a read cut short by the
  deadline;
- `warn` for a refused `next_page_url`;
- `debug` for each search result dropped because it belongs to another team;
- `info` for each `init discover` catalogue write.

---

## Phase 1: Make room for Shortcut

### Overview

The shared seams learn the name `shortcut` without a client behind it. A user
can set `work.integration: shortcut` and the Shortcut keys. `work sync` then
reports that Shortcut "has no client wired yet", with exit 72, exactly as
Trello does today. The configure skill does not yet advertise the value.

### Changes Required

#### 1. Accepted integration value

**File**: `cli/config/src/catalogue.rs`
**Changes**: Add `"shortcut"` to `WORK_INTEGRATION_VALUES`.

Test first: a `work` config read with `work.integration: shortcut` succeeds,
where today it fails closed.

#### 2. Credential and scope keys

**File**: `cli/config/src/catalogue.rs`
**Changes**: Add these to `EXTRA_KEYS`:

```rust
ExtraKey::open("shortcut.team_key"),
ExtraKey::overridden(
    "shortcut.token",
    Trust::Open,
    &["ACCELERATOR_SHORTCUT_TOKEN"],
),
ExtraKey::overridden(
    "shortcut.token_cmd",
    BASE_COMMAND,
    &["ACCELERATOR_SHORTCUT_TOKEN_CMD"],
),
```

Extend the consent tests at `catalogue.rs:565-651` and
`cli/config/tests/extra_keys_mirror.rs` first. Regenerate
`cli/launcher/tests/fixtures/dump/dump.golden`.

#### 3. Registry recognises Shortcut

**File**: `cli/work-cli/src/tracker_registry.rs`
**Changes**: Extend the `NotAvailable` arm to
`"trello" | "github-issues" | "shortcut"`. Add `shortcut` to both
`SelectionError` messages.

Test first: resolving `shortcut` yields `SelectionError::NotAvailable`, and
`work sync` exits 72.

#### 4. Sync skill accepts lowercase keys

**Files**: `skills/work/sync-work-items/SKILL.md`,
`tests/unit/tasks/test_sync_work_items_skill_structure.py`
**Changes**:
- Update the structure test's pin at `:268` first, so it fails.
- Replace `^[A-Z][A-Z0-9_]*-[0-9]+$` at `:302-305` and the tracker-key shape
  at `:588` with `^[A-Za-z][A-Za-z0-9_]*-[0-9]+$`. This matches
  `corpus::is_tracker_key`.
- Add `shortcut` to the skill's tracker lists at `:55-68`, in the "not built
  yet" line for exit 72.

The widened gate applies to every tracker, so a Jira or Linear user's
lowercase key such as `eng-12` now reaches a command. That is intended: the
Rust grammar already accepts it.

The sync skill has no evals, so the structure test pins the change instead:
it asserts the widened regex is present and the uppercase-only one is gone.
The `:588` site names the shape without repeating the regex, so it follows
the `:302` rule.

#### 5. Exit-code docs

**File**: `cli/work-cli/src/exit_codes.rs`
**Changes**: Add `shortcut` to the module docs' tracker set for code 72 at
`:78-81`.

### Success Criteria

#### Automated Verification

- [x] Config tests pass: `cargo nextest run -p config`
- [x] Registry tests pass: `cargo nextest run -p accelerator-work tracker_registry`
- [x] Launcher dump golden passes: `cargo nextest run -p accelerator config_read`
- [x] Build-system tests pass: `mise run test:unit:tasks`
- [x] Skill lint passes: `mise run lint:integration-skills:check`, `mise run lint:skill-permissions:check` and `mise run lint:skill-cli-refs:check`
- [x] Full run passes: `mise run`

#### Manual Verification

- [x] `accelerator config dump` lists the three Shortcut keys.
- [x] `work.integration: shortcut` makes `accelerator work sync` print the
      "recognised but has no client wired yet" message, with exit 72.

---

## Phase 2: `shortcut-client` foundation

### Overview

A new adapter crate resolves credentials, talks to v4, classifies failures,
maps keys and reads and writes the catalogue. It does not yet implement
`RemoteTracker`. A live smoke test records the v4 facts Phase 3 depends on.
Phase 2 ends with a checkpoint that revises Phase 3 from that evidence.

### Changes Required

#### 1. Crate scaffold and architecture rules

**Files**:
- `cli/shortcut-client/Cargo.toml`: `role = "adapter"`,
  `context = "shortcut"`, `kind = "product"`. It has the same dependencies as
  `linear-client` minus GraphQL test support: reqwest, rustls,
  webpki-root-certs, serde, serde_json, thiserror, tracing, url, config,
  store, remote-projection, tracker and tracker-support. It declares no
  features.
- `cli/Cargo.toml`: add the workspace member.
- `cli/tracker/Cargo.toml`: add `"shortcut"` to `downstreams`.
- `cli/pup.ron`:
  - add `shortcut_client_is_the_only_shortcut_transport`, which denies
    `work`, `work_adapters`, `jira_client` and `linear_client`;
  - add `shortcut_client_io_lives_in_cache`, which denies `std::fs` in
    `transport`, `discovery` and `workspace`;
  - add `^shortcut_client(::|$)` to the `jira_client`, `linear_client` and
    `tracker_support` deny lists;
  - add `shortcut_client` to the command-runner alternation at `:548`.
- `tests/integration/pup/test_import_rule.py`: add probe pairs for both new
  rules.
- `tests/unit/tasks/test_crate_dependencies.py`: add the crate's
  `_EXPECTED_DECLARATIONS` row and expected edges.
- `tasks/public_api.py`: classify `shortcut-client` as an exempt adapter.
- `cli/work-cli/tests/provider_isolation.rs`: add `shortcut_client` to
  `PERMITTED_HTTP_CRATES`.

Follow the library-crate checklist at `tasks/README.md:992-1049`.

#### 2. Shortcut keys

**File**: `cli/shortcut-client/src/shortcut_key.rs`
**Changes**: Add `ShortcutKey`, `StoryId` and `EpicId`, with `parse`,
`Display`, `collection` and `into_story`.

Tests first:
- `sc-1234` and `SC-1234` parse to the same story key.
- `epic-56` and `EPIC-56` parse to the same epic key.
- `sc-56` and `epic-56` are different keys.
- `1234`, `sc-`, `sc-12a`, `ENG-12`, `sc-0`, `sc-012` and `epic-0` are
  refused.
- Keys render as lowercase `sc-1234` and `epic-56`.
- `collection` gives `stories` and `epics`.
- `into_story` on an epic key gives `NotAStory` carrying the epic id.
- Every rendered key passes `tracker_support::issue_key_is_safe`.

#### 3. Kinds

**File**: `cli/shortcut-client/src/new_entity.rs`
**Changes**: Add `NewEntity::for_kind` and `story_type_for_kind` from the kind
table.

Tests first:
- `for_kind`: one case per table row, including `epic`, empty and an unknown
  kind;
- `story_type_for_kind`: `task` gives `chore`, and `epic`, empty and an
  unknown kind give nothing.

#### 4. Credentials

**File**: `cli/shortcut-client/src/auth.rs`
**Changes**:
- Resolve the token through `config::credentials::resolve_token` with
  `TokenKeys::declared("shortcut.token", "shortcut.token_cmd")`.
- Refuse a token that holds anything a header value cannot carry, or that
  starts with `Bearer `.
- Mark the `Authorization` header value as sensitive in reqwest.
- Add `auth::team_key(config) -> Result<Option<String>, _>`, which reads only
  `shortcut.team_key`.

Tests mirror `linear-client`'s auth tests:
- `team_key` returns the configured value, and `None` when it is absent;
- the precedence is environment, then command, then `config.local.md`;
- `token_cmd` in team config is refused;
- the token never appears in a `Debug` or error string;
- a missing token's message names all three sources.

#### 5. Transport

**File**: `cli/shortcut-client/src/transport.rs`
**Changes**:
- `Transport::new(base_url, credentials, TransportConfig, Sleeper, Jitter)`
  installs the ring provider, trusts only the bundled webpki roots and
  disables redirects.
- `get`, `post` and `patch` send `Authorization: Bearer <token>` and
  `Content-Type: application/json`. Each takes the request's `Resend` policy.
- The transport returns a `Delivery`: the last attempt's `Outcome`, and
  `may_have_applied`, which it sets when any attempt ends in 5xx, `Transport`
  or `NonJsonBody`. It sets `NonJsonBody` only for a 2xx.
- Requests use `tracker_support::RetryPolicy` with jittered backoff. Shortcut
  documents no `Retry-After` header, so the transport honours one only if
  present.
- Bodies are read with a size bound. A whole-operation `Deadline` applies.
- `follow(next_page_url)` admits only a URL on the configured origin whose
  path starts with `/api/v4/`. Any other URL is refused, so the bearer token
  never leaves Shortcut.

Tests first, against `MockHTTPServer`, each counting requests:
- the headers on every request;
- a `WhileRetryable` GET gets 429 then 200 and succeeds after two requests;
- a `WhileRetryable` GET gets 500 four times and gives `Retryable` after four
  requests;
- a `WhileRetryable` GET that times out is sent exactly once;
- an `OnlyWhenThrottled` POST gets 500 and is sent exactly once;
- an `OnlyWhenThrottled` POST gets 429 then 201 and is sent twice;
- an `OnlyWhenThrottled` POST that times out is sent exactly once;
- an `OnlyWhenThrottled` POST that cannot connect is sent once and reports
  `NotSent` with `may_have_applied` unset;
- a `WhileRetryable` PATCH gets 500 then 200 and is sent twice;
- a `WhileRetryable` PATCH gets 500 then 429 four times and reports
  `may_have_applied` set;
- a 403 with an HTML body gives `Status(403)`, not `NonJsonBody`;
- a `Retry-After: 2` header delays the retry by two seconds on the fake
  sleeper;
- a deadline that expires during backoff stops the retries;
- a redirect is not followed;
- an oversized body is refused;
- a foreign-host `next_page_url`, and one outside `/api/v4/`, are refused.

#### 6. Paths and the endpoint seam

**File**: `cli/shortcut-client/src/paths.rs`
**Changes**:
- `paths` is a pure function of the base URL, the slug and typed ids. It
  reads no environment.
- Export `url_is_allowed(url, allow_loopback)`. It admits an https URL whose
  host is exactly `api.app.shortcut.com`, with no userinfo and the default
  port. It admits a loopback http URL only when `allow_loopback` is true.
- Add `ShortcutClient::from_config_at(base_url, …)` beside `from_config`.

Tests first for `url_is_allowed`:
- `https://api.app.shortcut.com` is admitted;
- `https://api.app.shortcut.com.evil.io`, `https://xapi.app.shortcut.com`,
  `https://evil.io/api.app.shortcut.com`, `http://api.app.shortcut.com`,
  `https://user@api.app.shortcut.com` and `https://api.app.shortcut.com:8443`
  are refused;
- `http://127.0.0.1:9000` is refused with `allow_loopback` false and admitted
  with it true.

Tests first for `paths`: a slug or id that `Url::path_segments_mut` would
percent-encode stays one segment.

#### 7. Failure classification

**File**: `cli/shortcut-client/src/classify.rs`
**Changes**: Implement the classification table from Implementation Approach.
Parse Shortcut's `{message, tag?, errors?}` envelope when it is present.

Tests first, table-driven: one case per outcome for a read and for each write
`Operation`, with `may_have_applied` both set and unset. Add cases for:
- an envelope, a body without one, and a non-JSON 2xx body;
- a 403 with a non-JSON body on a write giving `Retryable`;
- a 410 on a write giving `Retryable`;
- a 418 on a write giving `Terminal`;
- `UnusableBody` on a write giving `Terminal`;
- the exact 401, no-token and 403 wording.

#### 8. Workspace, discovery and catalogue

**Files**: `cli/shortcut-client/src/workspace.rs`,
`cli/shortcut-client/src/discovery.rs`,
`cli/shortcut-client/src/catalogue.rs`, `cli/shortcut-client/src/cache.rs`
**Changes**:
- `workspace` calls `GET /api/v4/whoami`. It returns the slug, the member,
  the token scopes and the default workflow state.
- `discovery` lists teams through `GET /teams`, with each team's workflows,
  and workflows through `GET /workflows`, with each workflow's states. It
  follows `next_page_url` to the end under the deadline, with no page cap. It
  reports whether each listing completed, and stops on a repeated
  `next_page_url`. It writes nothing.
- `team_key.rs` holds `TeamKey` and `TeamKeys`, by the rules in Team
  identity.
- `catalogue` is the serde model, with `schema_version`, the `flatten` map of
  unknown fields on every record, and the load-time validation from
  Implementation Approach.
- `Catalogue::team_for_key` resolves a team key to a catalogued team.
- `cache` reads `catalogue.json`, and writes it atomically under the mkdir
  lock. It also scaffolds the `.gitignore` that keeps `viewer.json`, `.lock/`
  and refresh metadata out of VCS.
- `ShortcutClient::from_config` reads the catalogue and makes no request.

Tests first:
- `from_config` with no catalogue is `Unconfigured`, names
  `/accelerator:init-shortcut`, and makes no request;
- `from_config` with a catalogue makes no request;
- a team or workflow listing cut short by the deadline reports incomplete;
- a repeated `next_page_url` stops the listing and reports a fault;
- the cache refuses to write an incomplete catalogue;
- the cache refuses to replace a catalogue with a newer `schema_version`,
  with or without `--force`;
- two writers under the lock leave one valid file;
- the scaffolded `.gitignore` lists the three ignored paths;
- `TeamKey::of` gives the mention name, or the display name when the mention
  name is null;
- `TeamKeys::over` finds a collision between two null-mention teams with the
  same display name;
- `TeamKeys::over` finds a collision between one team's display name and
  another team's mention name;
- `Catalogue::team_for_key` resolves a team key to a uuid by mention name,
  and a null-mention team by display name;
- a team's display name is not a team key when the team has a mention name;
- a team whose name holds `"` or `\` loads, and other teams stay usable;
- a slug, uuid or id that fails validation is `Unconfigured`, naming the
  field;
- a catalogue with an unknown field, and a state with an unseen `type`,
  still load;
- a rewrite keeps a field the client does not know, at the top level, on a
  team and on a workflow;
- a catalogue with a newer `schema_version` is `Unconfigured`, naming the
  plugin upgrade;
- a catalogue with an older `schema_version` is `Unconfigured`, naming
  `init discover`;
- a catalogue with the current `schema_version` loads.

#### 9. Evidence hygiene

**Files**: `cli/tracker-test-support/src/evidence.rs`,
`cli/shortcut-client/tests/evidence_hygiene.rs`, `.gitignore`
**Changes**:
- Add `"sct_"` to `SECRET_SHAPES`, with a test that a Shortcut token prefix
  is a violation.
- Add `cli/shortcut-client/tests/captures/` to `.gitignore`.
- Add `evidence_hygiene.rs`, mirroring `linear-client`'s. It runs over
  `tests/evidence/` and `tests/fixtures/v4/`. It rejects `sct_`, `Bearer `,
  `@` and any slug other than the placeholder `scratch-workspace`. It also
  asserts that `git check-ignore` reports `tests/captures/` as ignored.

#### 10. Live smoke test

**File**: `cli/shortcut-client/tests/contract.rs`
**Changes**: Gate the test on `ACCELERATOR_TRACKER_CONTRACT=1`. Against a
scratch workspace, it writes raw responses to the gitignored
`tests/captures/` for these facts:

- that an `sct_rw_` token reaches whoami, and the scopes whoami reports;
- the 401, 403, 404 and 422 bodies;
- the `updated_at` string format;
- the Markdown a story description and an epic description store and return;
- the fields `POST /epics` requires, and how an epic is tied to a team;
- `GET /stories/{id}?fields=id,name,description,updated_at,app_url`;
- `GET /stories?ids=…&filter=include_archived`, including an archived story,
  and whether the response can hold an id outside the request;
- whether `GET /epics` accepts an `ids` filter;
- whether an epic and a story can share an id;
- `GET /stories/search` with `team:"<name>"`, its `limit` maximum, and its
  `next_page_url`;
- whether `team:` matches a name exactly, or by prefix or substring;
- the field on a story search result that carries its team's uuid;
- `GET /teams/{uuid}` for a known team and for an unknown uuid;
- the field on a team that lists the workflows it uses;
- `next_page_url` on `GET /teams` and `GET /workflows`;
- how search treats a `\"` inside a quoted value;
- `PATCH` with `external_links_add`;
- `POST /stories/{id}/comments` and `POST /epics/{id}/comments`;
- the `label:`, `state:`, `owner:` and `type:` search operators;
- how search combines two operators of the same kind, such as
  `label:"a" label:"b"`, and whether it has an OR form;
- how search negates an operator, such as `-label:"a"`;
- `GET /members`, and the fields that hold a member's mention name and uuid;
- a story read's `workflow_id` field;
- `PATCH /stories/{id}` with `workflow_state_id`, `owner_ids` and `estimate`;
- whether any Shortcut token format lacks the `sct_` prefix.

The test archives every story and epic it creates. Before it exits, it
asserts that no file it wrote under `tests/captures/` or `tests/evidence/`
contains the token string it ran with.

From the captures, a person writes sanitised fixtures under
`tests/fixtures/v4/`. They use the placeholder slug `scratch-workspace`, and
placeholder member ids and names. The person reviews them before commit, and
`evidence_hygiene.rs` checks them. The committed `tests/evidence/` holds only
reduced records from `tracker_test_support::evidence`.

#### 11. Checkpoint

Revise Phase 3 from the recorded facts before it starts. The revision
replaces each conditional in Phase 3 with the chosen branch, so Phase 3 holds
no "if the evidence shows" text when work on it begins. Decide:
- the `fetch_all` read strategy for each entity: batched `?ids=`, or one
  `GET` each;
- the epic payload fields the client normalises into the story shape;
- whether search honours `\"`, which decides between escaping and refusing
  quotes;
- how a filter with several values for one field lowers, by the rule in
  Phase 3, Section 4;
- the story field `search` filters on for the team uuid;
- the class of a 404 from `GET /teams/{uuid}` during search;
- whether `SECRET_SHAPES` needs a shape beyond `sct_`;
- whether Shortcut rewrites Markdown in a way that changes meaning.

#### Checkpoint decisions

Recorded from a live run against a scratch workspace on 2026-11-14:

- Shortcut stores and returns story and epic Markdown byte-for-byte, so no
  rewrite cases exist.
- `updated_at` is ISO 8601 UTC with milliseconds, such as
  `2026-11-14T08:21:34.384Z`. The client keeps the bytes as received.
- `GET /stories?ids=` returns only requested ids and omits absent ones.
  `filter=include_archived` adds archived stories, which the default filter
  drops, so every keyed read sends it.
- `GET /epics` refuses an `ids` filter with 400, so `fetch_all` reads each
  epic with its own `GET`.
- A page `limit` above 100 is a 400.
- A story search result carries its team's uuid at `team.id`.
- `POST /epics` needs only `name` and `description`, and an epic has no single
  team.
- Error bodies are `{message, tag}` for 401, `{message}` for 404, and
  `{message, errors}` for a validation 400. No 422 appeared.
- `GET /teams/{uuid}` for an unknown uuid is a 404 `{message}`. During search
  that is `Unconfigured`, naming the team and `init discover`.
- Reading an epic's id as a story is a 404, which fits one id sequence shared
  by stories and epics. The `epic-` prefix stays either way.
- Repeating `external_links_add` keeps one link. `attach` stays
  `OnlyWhenThrottled` regardless.
- `-label:` negation works. The workspace had no labels, so the run could not
  show whether repeated operators AND or OR. One search per value combination
  is correct either way, so the client uses it.
- An escaped `\"` inside a quoted value returned no result, which proves
  nothing, so `filter.rs` refuses `"` and `\`.
- `SECRET_SHAPES` needs nothing beyond `sct_`: every token seen had it.

#### Implementation notes

- `ShortcutClient::from_config` and its two no-request tests move to Phase 3,
  where the client type is built. Phase 2 delivers the parts it calls:
  `ShortcutCache::load`, the catalogue validation and `auth::team_key`.
- Discovery reads each team's workflows from `GET /teams/{uuid}/workflows`
  and each workflow's states from `GET /workflows/{id}/states`, rather than
  the nested lists an entity carries, which v4 may cut short.
- The whoami and listing decoders follow the v4 reference. The checkpoint
  confirms or corrects them from the recorded captures.

### Success Criteria

#### Automated Verification

- [x] Client tests pass: `cargo nextest run -p shortcut-client`
- [x] Evidence tests pass: `cargo nextest run -p tracker-test-support`
- [x] Architecture rules pass: `mise run pup:check` and `mise run test:integration:pup`
- [x] Crate-dependency tests pass: `mise run test:unit:tasks`
- [x] Public API check passes: `mise run public-api:check`
- [x] Dependency policy passes: `mise run deny:check`
- [x] Full run passes: `mise run`

#### Manual Verification

- [x] With a scratch workspace token,
      `ACCELERATOR_TRACKER_CONTRACT=1 cargo nextest run -p shortcut-client --test contract`
      passes and writes captures under `tests/captures/`.
- [x] Every fact in Section 10 has a recorded answer in the fixtures or in
      the revised Phase 3.
- [x] Phase 3 holds no conditional text, and each Section 11 decision is
      written into it.
- [x] `jj status` shows no file under `tests/captures/`.

---

## Phase 3: `shortcut-client` implements `RemoteTracker`

### Overview

The client implements every port operation and passes the offline contract
suite for stories and for epics. No composition root uses it yet.

### Changes Required

#### 1. Projection

**File**: `cli/remote-projection/src/lib.rs`
**Changes**:
- Add `Integration::Shortcut` and `"shortcut"` to `parse_integration`.
- The body is `/name` + `\n` + `/description`, and the stamp is `/updated_at`.
  The client unwraps the v4 `entity` envelope and each item of `entities`. It
  normalises an epic into the same shape before projecting, so the enum keeps
  one variant per integration.
- Add the new arm to the exhaustive matches in `jira-client`,
  `linear-client` and `work-adapters/tests/corpus_hashes.rs`.
- Regenerate `cli/remote-projection/tests/fixtures/public-api.txt`.

Test first: Shortcut story and normalised epic fixtures each project to
`"{name}\n{description}"`. An absent `updated_at` yields an empty stamp,
which the client maps to `RemoteTimestamp::NotReported`.

#### 2. Body dialect

**File**: `cli/shortcut-client/src/client.rs`
**Changes**: `BodyDialect` passes Markdown through, and `is_wire_document`
returns `false`. The live run stored and returned a story and an epic
description byte-for-byte, so Shortcut has no rewrite cases to record.

#### 3. Port operations

**File**: `cli/shortcut-client/src/client.rs`

Below, `{entity}` is `ShortcutKey::collection()`.

| Operation | v4 request | Notes |
|---|---|---|
| `create` | `NewEntity::for_kind` picks `POST /epics` or `POST /stories` | `OnlyWhenThrottled`. A story gets `name`, `description`, `story_type`, `team_id` and `workflow_state_id`. The state is the first `unstarted` state, by `position`, of the team's default workflow. If that workflow has none, whoami's default state is used only if it belongs to that workflow; otherwise the result is `Unconfigured`. An epic gets `name` and `description` only; an epic has no single team. Returns `sc-<id>` or `epic-<id>`. |
| `update` | `PATCH /{entity}/{id}` built from `UpdateFields` | `WhileRetryable`. Sends `description` always and `name` only when given. |
| `retitle` | `PATCH /{entity}/{id}` built from `UpdateFields` with `name` only | `WhileRetryable`. |
| `show` | `GET /{entity}/{id}?fields=id,name,description,updated_at,app_url` | Projects through `remote_projection`. Reports the requested key. |
| `locate` | the same `GET` | 404 gives `Located::NotFound`. Ids never move, so a found entity keeps its requested key. |
| `fetch_all` | stories: `GET /stories?ids=…&filter=include_archived&limit=100&fields=id,updated_at`; epics: one `GET /epics/{id}?fields=id,updated_at` each, because `/epics` refuses an `ids` filter | Story keys and epic keys go in separate chunks of 100. A map from numeric id to every requested `ExternalId` pairs each record with the ids it answers. After a complete read, an unanswered id is `absent`. After a partial read, or a response holding an unrequested id, unanswered ids are `indeterminate`. |
| `search` | for each team uuid in scope, `GET /teams/{uuid}`, then `GET /stories/search?query=…` | Stories only. The first request reads the team's current name, which `filter.rs` checks before it enters the query. Composes `team:"<name>"` plus the configured filters. Keeps only results whose team uuid matches, so a team that shares a name or a name prefix adds nothing. A dropped result does not count towards truncation. Flags truncation at the page cap or deadline. Under `all_teams`, a team whose name `filter.rs` refuses is skipped: the discovery is marked incomplete, and a `warn` names the team. For a configured team key, the same refusal is `Unconfigured`. |
| `resolve_scope` | none | By the rules in Team identity: a base-only scope gets the uuid, and a broadened scope keeps its configured keys. Validates filters. Refusals are `ScopeError`. |
| `enumerate_visible_entities` | `GET /teams` | One `VisibleEntity` per team, with the uuid as `identifier`. The `key` is the team key from `TeamKeys::over`, or the uuid for a colliding key. A listing cut short by the page cap or deadline is `Retryable`, never a partial `Ok`. |
| `preview_create` | none | `NewEntity::for_kind` gives the type. The project is the team name, or `Unresolvable` when `team_key` is unknown. |
| `validate_update` | none | Refuses an empty title. Does not parse the id. |

Archived stories and epics count as found, so sync never reports an archived
entity absent.

`cli/shortcut-client/src/update.rs` holds `UpdateFields`, the one builder for
the update `PATCH`. It sends exactly the fields it is given, in one request.
Phase 3 gives it `name` and `description`. Phase 6 adds the CLI's fields.

Tests first, against the Phase 2 fixtures:
- `UpdateFields` with only a title sends only `name`;
- `update` and `retitle` both build through `UpdateFields`;
- `fetch_all` with 100 ids sends one request; with 101 ids it sends two, and
  each request carries the right ids;
- a mixed `sc-` and `epic-` request reaches both endpoints;
- an archived story is `found`;
- a requested `SC-12` is found as `SC-12`;
- a partial page leaves only the unanswered ids `indeterminate`;
- a response holding an unrequested id leaves the unanswered ids
  `indeterminate`;
- a deadline used up by the first chunk leaves the second chunk's ids
  `indeterminate`, on a fake clock with 101 ids;
- with one `GET` per epic, a 404 for one epic gives `absent` for that epic
  only;
- every keyed story read sends `filter=include_archived`;
- `create` picks the unstarted state by `position`, not by id;
- `create` falls back to whoami's state only when it belongs to the team's
  workflow;
- `create` with an unknown kind sends nothing and returns `Rejected`;
- a 201 create with no usable `id` returns `Terminal`, through
  `UnusableBody`;
- `create` for a story gets 500 from `POST /stories`, is sent exactly once,
  and returns `Terminal`;
- `create` with kind `epic` gets 500 from `POST /epics` and is sent exactly
  once;
- `create` gets 429 then 201 and is sent twice;
- `update` gets 500 and then 429 until retries run out, and returns
  `Terminal`;
- `update` gets 429 until retries run out, with no earlier 5xx, and returns
  `Retryable`;
- `search` with a 500 from `GET /teams/{uuid}` returns `Retryable`, never a
  partial `Ok`;
- `search` with a 404 from `GET /teams/{uuid}` returns `Unconfigured`,
  naming the team and `accelerator shortcut init discover`;
- under `all_teams`, `search` skips a team whose name `filter.rs` refuses,
  marks the discovery incomplete, and searches the other teams;
- `search` keeps only results for the scoped team when another team shares
  its name;
- a dropped result does not mark the search truncated;
- `search` refuses a live team name that holds `"` or `\` with
  `Unconfigured`, naming the team;
- `enumerate_visible_entities` keys a null-mention team by display name, with
  the uuid as `identifier`;
- `enumerate_visible_entities` keys both teams of a colliding pair by uuid;
- `resolve_scope` refuses a colliding key in `additional_teams` with
  `ScopeError`, giving the four recovery steps;
- `resolve_scope` refuses a colliding base `shortcut.team_key` on a base-only
  scope with `ScopeError`, before `Catalogue::team_for_key` picks a team;
- `resolve_scope` refuses an absent `shortcut.team_key` with `ScopeError`,
  naming `init list-teams`;
- with a `state` filter, `resolve_scope` refuses an additional team key that
  is missing from the catalogue, naming `init discover`;
- without a `state` filter, `resolve_scope` passes an uncatalogued
  additional team key through unchanged;
- `enumerate_visible_entities` returns `Retryable` when `GET /teams` has more
  pages than the cap;
- `resolve_scope` gives a base-only scope the uuid, and leaves a broadened
  scope's keys unchanged;
- each refused key gives the result in the per-operation table.

#### 4. Pull filters

**File**: `cli/config/src/catalogue.rs`
**Changes**: Add the Shortcut `TrackerBlock`s:
- `shortcut.pull` with `additional_teams`, `all_teams` and the filters
  `label`, `state`, `owner` and `type`, with
  `mutually_exclusive: &[("all_teams", "additional_teams")]`;
- `shortcut.push` with `max_items`.

**File**: `cli/shortcut-client/src/filter.rs`
**Changes**:
- Lower validated filters into search operators, and quote every value.
- Keep the meaning Jira and Linear give a filter. Values for one field are
  OR'd, as `jira-client/src/jql.rs:278-298` and
  `linear-client/src/filter.rs:402-408` do. Fields are AND'd.
  - Run one search for each combination of positive values, and
    union the results, deduplicated by story id. A filter whose combinations
    exceed 10 searches per team is `ScopeError`, naming the count. The page
    cap and the deadline apply to all of a team's searches together. If any
    one search is truncated or runs out of time, the merged result is
    truncated.
  - A negated (`~`) value lowers to `-<operator>:"<value>"`, which the live
    run confirmed search accepts. Negated values are AND'd, which needs no extra searches. If
    search has no negation, a negated value is `ScopeError`.
- `filter.rs` is the one place that checks a value before it enters a query.
  It checks filter values, and the live team name `search` reads. It refuses
  any value holding `"` or `\`, because the live run could not show that
  search honours an escaped quote. A refused team name names the team, and
  says to rename it in Shortcut or to scope to another team.
- Validate a `state` against the workflows of the teams in scope, using each
  team's workflow ids from the catalogue. With `all_teams`, validate against
  every catalogued workflow. Refuse any other state, listing the valid states
  and naming `accelerator shortcut init discover` for a state added since the
  last discovery.
- `type` accepts Shortcut types (`feature`, `bug`, `chore`) and local kinds
  through `story_type_for_kind`. It refuses `epic` and an unknown kind, naming
  the accepted values.
- Every refusal is `ScopeError` from `resolve_scope`, or `Unconfigured` from
  `search` for the live team name. None goes through `classify`.

Tests first:
- each filter lowers to the expected query string;
- a value holding a quote or a space cannot add a second operator;
- an unknown state is refused with `ScopeError`, listing the valid states and
  naming `init discover`;
- a state from an additional team's workflow is accepted;
- a state used by no team in scope is refused;
- with `all_teams`, a state from any catalogued workflow is accepted;
- `additional_teams` produces one search for each team;
- `type: task` lowers to `type:chore`;
- `type: epic` is refused with `ScopeError`;
- `label: [a, b]` returns the union of stories labelled `a` or `b`, with no
  story twice;
- `label: [a, b]` with `state: [x, y]` runs four searches;
- a filter needing 11 searches per team is refused with `ScopeError`;
- when one of four searches hits the page cap, the merged result is
  truncated;
- `label: ~a` lowers to `-label:"a"`;
- a value holding `"` or `\` is refused with `ScopeError`.

#### 5. Contract create kind

**File**: `cli/tracker-test-support/src/contract.rs`
**Changes**: Add `ContractSubject::create_kind()`, which defaults to
`self.preview_kind()`. The create-based properties pass it to `create`
instead of the literal `""`. Jira and Linear keep the default, so their runs
do not change.

Test first: a subject that returns `"epic"` from `preview_kind` sees `"epic"`
in its recorded `create` call.

#### 6. Offline contract

**File**: `cli/shortcut-client/tests/contract_offline.rs`
**Changes**: Two `ContractSubject`s backed by `MockHTTPServer`, one for
stories and one for epics. The epic subject returns `"epic"` from
`preview_kind`. Each subject sets:
- `unreadable_id`: a key whose `GET` returns 500;
- `unaccountable_id`: a key in a `fetch_all` chunk whose page read fails;
- `moved_issue`: `None`;
- `truncating_scope`: a team scope whose mock returns more pages than the cap.

Call every `_property` function against both subjects except
`a_moved_issue_is_found_only_under_its_current_key_property`. Shortcut ids
never move, so that property does not apply. The mock routes and query
parameters come from the Phase 2 fixtures, not from the OpenAPI document.

`run_all` cannot drive this run, because it requires
`ACCELERATOR_TRACKER_CONTRACT=1`. So a test lists the `_property` functions
`contract.rs` exports, and fails when one is neither called for both
subjects nor named as the moved-issue exclusion.

#### 7. Live contract

**File**: `cli/shortcut-client/tests/contract.rs`
**Changes**: Add live story and epic `ContractSubject`s against the scratch
workspace. Override the `can_*` hooks to `false` where a live workspace
cannot produce the condition, as `linear-client/tests/contract.rs` does. Run
`timed_conformance` for each subject, and write its reduced records to
`tests/evidence/`. Assert that the record count matches the properties each
subject runs, so an empty run fails.

#### Implementation notes

- The projection reads Shortcut's single-entity answer directly, with the
  pointers `/entity/name`, `/entity/description` and `/entity/updated_at`.
  A story and an epic share that envelope, so the client projects the raw
  bytes with no unwrapping or epic normalisation step.
- `ShortcutClient::from_config` lands here, as Phase 2 recorded.
- On a 403, the client reads whoami once for the token's scopes, so the
  message can say a read-only token cannot write.
- The port tests were written after `client.rs`, not before it. To show they
  catch real faults, three mutations were each caught: a story create resent
  on a 5xx, `fetch_all` without `include_archived`, and `search` without the
  team-uuid filter.

### Success Criteria

#### Automated Verification

- [x] Projection tests pass: `cargo nextest run -p remote-projection`
- [x] Public API check passes: `mise run public-api:check`
- [x] Contract harness tests pass: `cargo nextest run -p tracker-test-support`
- [x] Jira and Linear offline contracts still pass: `cargo nextest run -p jira-client -p linear-client`
- [x] Client and offline contract tests pass: `cargo nextest run -p shortcut-client`
- [x] Config catalogue tests pass: `cargo nextest run -p config`
- [x] Full run passes: `mise run`

#### Manual Verification

- [x] The live contract suite passes against a scratch workspace:
      `mise run test:integration:tracker-contract` with the Shortcut
      credentials set.
- [x] The evidence file holds a record for each property each subject runs.
- [x] Every story and epic the live suite creates is archived after the run.

---

## Phase 4: `work.integration: shortcut` syncs

### Overview

The composition root wires the client in. `/sync-work-items` works against
Shortcut after this phase merges, once `init-shortcut` or a teammate's
committed catalogue provides `catalogue.json`. This phase merges after
Phase 5.

### Changes Required

#### 1. Registry

**File**: `cli/work-cli/src/tracker_registry.rs`
**Changes**: Move `shortcut` out of the `NotAvailable` arm into its own arm
that builds `ShortcutClient::from_config`. Report consent notices and refusals
as the other arms do. Add a `#[cfg(test)]` endpoint seam that calls
`from_config_at`, mirroring Linear's `with_linear_endpoint`.

Tests first:
- with a mock server, a token and a catalogue, resolving `shortcut` returns a
  tracker whose `show` reaches the mock;
- with no catalogue, resolving `shortcut` gives `Unconfigured`, exits 74 and
  names `/accelerator:init-shortcut`.

#### 2. Integration-name match sites

Each site that matches on the integration name gets an explicit decision and
a test that pins it.

| Site | Decision |
|---|---|
| `cli/config/src/catalogue.rs` `WORK_INTEGRATION_VALUES`, `EXTRA_KEYS`, `TRACKERS` | Done in Phases 1 and 3. |
| `cli/work-cli/src/tracker_registry.rs` `resolve` and `SelectionError` | This phase. |
| `cli/work-cli/src/sync.rs` `resolve_active_scope_key` | `"shortcut"` resolves through `shortcut_client::auth::team_key`. Test that it does not fall back to `work.default_project_code`. |
| `cli/work-cli/src/create.rs` `render_create_preview` | No change: Shortcut renders type and project columns. Test that pins its line. |
| `cli/work-cli/src/main.rs:531` run finaliser | No change: Shortcut gets `NoFinaliser`, because only `init` writes its catalogue. Test that pins it. |
| `cli/tracker/src/lib.rs` `supports_tracker_owned_ids` | No change. Test that `shortcut` gives `false`. |
| `cli/migrate/src/migrations/m0009.rs` `scope_key_for` | No change: Shortcut has no legacy scope key. Test that `shortcut` gives no key. |
| `cli/migrate-cli/src/sync_baselines.rs` `body_rules` | No change: no repository holds pre-0008 Shortcut baselines. `migrate-cli` gains no dependency. Test that `body_rules("shortcut")` is `None`. |
| `cli/remote-projection/src/lib.rs` `parse_integration` | Done in Phase 3. |
| `cli/work-cli/src/exit_codes.rs` docs | Move `shortcut` from the code 72 set to the wired set. Mention Shortcut in the code 74 text. |
| `skills/work/sync-work-items/SKILL.md:55-68` | Move `shortcut` out of the "not built yet" line. |
| `cli/work-cli/Cargo.toml` | Depend on `shortcut-client`. |

#### 3. Configuration docs

**File**: `skills/config/configure/SKILL.md`
**Changes**: Add `shortcut` to the integration values at `:645`. Add the three
Shortcut keys to the key table at `:42`, saying that `team_key` and
`additional_teams` take the team's mention name, or its display name when it
has no mention name. Update the "Future integrations" sentence near `:1038`.

#### 4. Engine tests against the real client

**Files**: `cli/work-adapters/tests/sync_run_real_client.rs`,
`cli/work-adapters/tests/corpus_hashes.rs`,
`cli/work-adapters/tests/fixtures/work-item-sync-baseline/case-shortcut-*`
**Changes**: Add Shortcut cases for push, pull, discovery and a conflict, and
an epic push and pull. Run them against a `ShortcutClient` and a mock server.
Parameterise the corpus hash tests over `Integration::Shortcut`. Add cases:
- a 403 on create leaves the item `Retryable`, not `MayHaveLanded`;
- a sync with `additional_teams` set resolves every configured team key
  through `resolve_entities` and searches each team by uuid;
- a sync with `all_teams` searches every visible team;
- a broadened scope naming an unknown team key exits 74, naming the key.

### Success Criteria

#### Automated Verification

- [ ] Work CLI tests pass: `cargo nextest run -p accelerator-work`
- [ ] Work adapter tests pass: `cargo nextest run -p work-adapters`
- [ ] Crate-dependency tests pass: `mise run test:unit:tasks`
- [ ] Full run passes: `mise run`

#### Manual Verification

- [ ] In a scratch repository with `work.integration: shortcut` and a
      catalogue, `accelerator work sync --push` creates a story. The work
      item gains `external_id: "sc-<id>"`.
- [ ] Pushing a work item of kind `epic` creates a Shortcut epic. The work
      item gains `external_id: "epic-<id>"`.
- [ ] Editing the story's description in Shortcut and running
      `/sync-work-items` pulls the change.
- [ ] Editing both sides produces a conflict dossier, and `adopt:sc-<id>`
      resolves through the skill.
- [ ] A story created in Shortcut under the configured team appears as an
      untracked remote during discovery.
- [ ] Pushing with a read-only token reports a retryable failure for each
      item, naming the read-only scope.

---

## Phase 5: `accelerator-shortcut` binary, read side

### Overview

The dispatched binary ships with `init`, `show` and `search`. Three skills use
it. This phase depends on Phase 3, and merges before Phase 4.

### Changes Required

#### 1. Binary crate

**Files**:
- `cli/shortcut-cli/Cargo.toml`: `name = "shortcut-cli"`, mirroring
  `cli/linear-cli/Cargo.toml:2`, `description = "Manage Shortcut stories"`,
  `[[bin]] name = "accelerator-shortcut"`, `[features] test-loopback = []`,
  inherited workspace fields and `[lints] workspace = true`.
- `src/main.rs` refuses to compile a release build with `test-loopback`. It
  carries the `TEST_LOOPBACK_MARKER`.
- `src/context.rs` reads `ACCELERATOR_SHORTCUT_API_URL`, admits it through
  `shortcut_client::url_is_allowed(url, cfg!(feature = "test-loopback"))`,
  and builds the client with `from_config_at`. This mirrors
  `linear-cli/src/context.rs:49-61`.

Test first: an inadmissible `ACCELERATOR_SHORTCUT_API_URL` gives
`ContextError::BadApiUrl`. The binary then exits 2 (`USAGE`), prints
`E_BAD_API_URL:` on stderr, and makes no request. This mirrors
`linear-cli/src/main.rs:52-57`.

#### 2. Registration

Follow `tasks/README.md` "Registering a dispatched sub-binary". For token
`shortcut`:

| Point | Action for Shortcut |
|---|---|
| 1 | Add `shortcut` to `DISPATCHED_SUBBINARIES`. Update the registry pin and `_SUBBINARY_DESCRIPTIONS` in `tests/integration/tasks/test_github.py`. Add `build:cli:shortcut:dev` to `mise.toml`. Add `shortcut` to `_LOOPBACK_SEAMED` and `_PACKAGES_NAMED_AFTER_THEIR_DIRECTORY` in `tasks/shared/dev_builds.py`. |
| 2 | Add `_SUBBINARY_MANIFESTS["shortcut"] = CLI_DIR / "shortcut-cli/Cargo.toml"`, because the crate is not at `cli/shortcut/`. |
| 3 | The `Cargo.toml` above. |
| 4 | Add `shortcut-cli` to `[workspace].members`, and commit the regenerated `cli/Cargo.lock`. |
| 5 | Add `bin/shortcut-*` to `.gitignore`. |
| 6 | No action. |
| 7 | `init-shortcut` is the binding witness. It invokes `accelerator shortcut init verify` through the `!` preprocessor, with a `Bash(...)` rule whose segment is exactly `shortcut`. It has no bare `Bash` tool and no wildcarded token segment. |
| 8 | Add `accelerator-shortcut` to `_CLI_RELEASE_BINARIES` in `tasks/build.py`. |
| 9 | No action: no symbolication archive. |
| 10 | No action: only a dispatch token is added. |
| 11 | Phase 7: a docs page, a README Concepts entry and an `ACCELERATOR_SHORTCUT_BIN` row. |
| 12 | No action: no symbolication archive. |
| 13 | Extend `cli/deny.toml` only if `mise run deny:check` fails. |
| 14 | Add `[package.metadata.accelerator]` with `role = "composition-root"`, and the crate's row in `_EXPECTED_DECLARATIONS`. |

Points 1, 2, 3, 4, 7 and 8 land in this phase's single change.

#### 3. Subcommands

**File**: `cli/shortcut-cli/src/cli.rs`

Every subcommand takes `-q`/`--quiet`, as `linear-cli`'s do.

| Subcommand | Behaviour |
|---|---|
| `init verify` | whoami, then print the workspace, member and scopes from whoami. |
| `init list-teams` | List teams by team key, display name and uuid. |
| `init discover --team <team-key> [--force]` | Discover teams and every workflow, then write the catalogue. Refuse to write a truncated listing, naming the listing and cause and saying to retry. Write the team key to `shortcut.team_key` when it is absent. Never prompt: leave a different value intact and print a `shortcut.team_key left intact` notice, or overwrite it with `--force`. Refuse to replace a catalogue with a newer `schema_version`. |
| `show <KEY> [--comments N]` | Print the story or epic as JSON with an `outcome` field. Accepts `sc-<id>`, `epic-<id>` and Shortcut story or epic URLs for the configured workspace, and echoes the canonical key. A URL for another workspace is a usage error and sends nothing. |
| `search [--state --owner --label --type --text --limit]` | Print JSON results with an `outcome` field. `--type` accepts Shortcut types and local kinds. `--owner` takes a member's mention name, which search's `owner:` operator takes. |

Three flags differ from Linear's, and each skill's argument hint says why:
- `--team` takes a team key, not Linear's `--team-id` uuid, because Shortcut
  users know their team by its mention name.
- `--owner` replaces Linear's `--assignee`, because Shortcut calls the person
  an owner.
- `--estimate` (Phase 6) replaces Linear's `--priority`, because Shortcut
  stores priority as a custom field, which is out of scope, and a story's
  points as `estimate`.

#### 4. Outcomes and exit codes

**Files**: `cli/shortcut-cli/src/keywords.rs`,
`cli/shortcut-cli/src/exit_codes.rs`
**Changes**: Keyword enums mirror Linear's. Exit codes follow Linear's bands:
- 0 to 2 for process and usage;
- 11 to 53 for transport and auth;
- 60 to 69 for init, including "catalogue listing truncated";
- 75 upward for per-flow codes;
- 70 to 74 stay reserved for dispatch.

Shortcut has no bash predecessor. So `tests/fixtures/captured-exit-codes.txt`
keeps Linear's file name, so `exit_codes_parity.rs` reads it unchanged. It is
written by hand first, and the parity test's doc says so.

#### 5. Flow tests

**Files**: `cli/shortcut-cli/tests/flow_{init,show,search,errors,consent,personal_file,pull_block,seam}.rs`,
`cli_surface.rs`, `keyword_surface.rs`, `stdout_goldens.rs`,
`stderr_diagnostics.rs`, `scenario_inventory.rs`
**Changes**: Write scenario-JSON-driven flow tests before each subcommand, as
in `linear-cli/tests/`. `scenario_inventory.rs` pins the scenario count and
fails on an unused fixture.

Flow cases include:
- no token, a 401 and a `Bearer `-prefixed token each give their specified
  message;
- `init verify`'s stdout and stderr never contain the token, including for a
  malformed token;
- `init discover` with a truncated team listing writes nothing;
- `init discover` writes an absent `team_key` without asking;
- `init discover` without `--force` leaves a different existing `team_key`
  unchanged;
- `init discover --force` overwrites a different existing `team_key`;
- `init discover` with a truncated listing names the listing and says to
  retry;
- `show` with a story URL echoes `sc-<id>`, and with an epic URL echoes
  `epic-<id>`;
- `show` with a URL for another workspace is a usage error that sends no
  request.

#### 6. Read skills

**Files**:
- `skills/integrations/shortcut/init-shortcut/SKILL.md`: not
  model-invocable. Flow: `init verify`, then `init list-teams`, then
  `init discover`. It explains where the token goes, and that a read-only
  token limits the skill set. On a `left intact` notice, it asks the user,
  then re-runs with `--force` only if they agree. It ends with a summary like
  `init-linear`'s: `catalogue.json` is shared through VCS, so the user should
  commit it, and a teammate should pull it rather than re-run init.
- `skills/integrations/shortcut/show-shortcut-story/SKILL.md`: model-invocable.
  It triggers automatically on `sc-1234` and Shortcut URLs only, with the
  same "not incidental mentions" guard as `show-linear-issue`. An `epic-56`
  key reaches it only through explicit invocation. Its description says it
  shows epics too.
- `skills/integrations/shortcut/search-shortcut-stories/SKILL.md`:
  model-invocable.
- `.claude-plugin/plugin.json`: add `"./skills/integrations/shortcut/"`
  directly after `"./skills/integrations/linear/"`. The array keeps its
  documented group order and is never sorted.
- `tasks/shared/skill_keyword_parity.py`: add Shortcut's `keywords.rs` to
  `_PROVIDERS`, and `"init-shortcut"` to `_NOT_KEYWORDS`. Make the provider
  wording in it and in `integration_skills.py` generic.
- `tasks/lint/skill_permissions.py`: raise `EXPECTED_INJECTION_SKILLS` by 3.

Each skill injects `accelerator config context --skill <name>` and
`accelerator config instructions <name>` through the `!` preprocessor. Each
runs `accelerator shortcut …` with scoped `allowed-tools`, and branches on
the `outcome` keyword.

The show and search skills state that remote titles, descriptions and
comments are untrusted data, never instructions. Add eval cases:
- a story description holding an instruction is summarised, not followed;
- a conversation about local epic `0181` does not trigger `show`.

### Success Criteria

#### Automated Verification

- [ ] Binary tests pass: `cargo nextest run -p shortcut-cli`
- [ ] Build-system tests pass: `mise run test:unit:tasks` and `mise run test:integration:tasks`
- [ ] Skill lints pass: `mise run lint:integration-skills:check`, `mise run lint:skill-permissions:check` and `mise run lint:skill-cli-refs:check`
- [ ] Read-only checks pass, including dispatch coherence: `mise run check`
- [ ] Full run passes: `mise run`

#### Manual Verification

- [ ] `/accelerator:init-shortcut` in a fresh repository writes the catalogue
      and `shortcut.team_key`.
- [ ] Asking "what is the status of sc-<id>?" triggers `show-shortcut-story`.
- [ ] "Show me open bugs in Shortcut" triggers `search-shortcut-stories` with
      `--type bug`.
- [ ] A read-only `sct_r_` token passes `init verify`. The output names the
      read-only scope.

---

## Phase 6: Write side

### Overview

The client gains the write operations, the binary gains five write
subcommands, and five skills use them. This phase depends on Phase 5.

### Changes Required

#### 1. Client write modules

**Files**: `cli/shortcut-client/src/{update,comment,transition,attach}.rs`
**Changes**: Each module builds its request, declares its `Resend` policy and
resolves names against the catalogue. `shortcut-cli` stays a thin
composition root. Client tests come before the CLI flow tests.

- `update` extends Phase 3's `UpdateFields` with `workflow_state_id`,
  `owner_ids` and `estimate`. It sends only the fields given, in one `PATCH`.
- `--owner` takes a member's mention name. `update` resolves it to the
  member's uuid with one `GET /members` lookup, which runs only when
  `--owner` is given. An unknown or ambiguous mention name lists the
  candidates.
- `story_state.rs` holds `resolve_story_state(workflow_id, name)`, the one
  state lookup that `transition` and `update --state` both use. It resolves
  the name within that workflow from the catalogue. A workflow missing from
  the catalogue fails, naming the workflow and asking for
  `accelerator shortcut init discover`. An unknown or ambiguous name lists
  the valid states and gives the same refresh hint.
- `transition` and `update --state` both first read the story with
  `GET /stories/{id}` for its `workflow_id`.
- `attach` admits https URLs only. Its `PATCH` with `external_links_add` is
  `OnlyWhenThrottled`.
- `comment` is `OnlyWhenThrottled`.

Client tests come first for each module's resend policy, counting requests:
- `comment` gets 500 and is sent exactly once;
- `comment` times out and is sent exactly once;
- `attach` gets 500 and is sent exactly once;
- `attach` gets 429 then 200 and is sent twice;
- `update` gets 500 then 200 and is sent twice;
- `transition`'s `PATCH` gets 500 then 200 and is sent twice;
- `update --state` reads the story before it sends the `PATCH`;
- `resolve_story_state` refuses a state from another workflow;
- `update --owner` with an unknown mention name sends no `PATCH`.

#### 2. Subcommands

| Subcommand | v4 request | Notes |
|---|---|---|
| `create [FILE] [--title] [--body-file]` | `POST /stories` or `POST /epics` | Reads the title, kind and body from work item frontmatter. `NewEntity::for_kind` picks the entity. Writes `external_id: "sc-<id>"` or `"epic-<id>"` back to the file only after a successful create. |
| `update <KEY> [--title --description --state --owner --estimate]` | `PATCH /{entity}/{id}` | `--state` and `--estimate` take story keys only. |
| `comment add <KEY> (--body \| --body-file)` | `POST /{entity}/{id}/comments` | `OnlyWhenThrottled`. |
| `transition <sc-ID> [STATE \| --state]` | `GET /stories/{id}`, then `PATCH /stories/{id}` with `workflow_state_id` | Stories only. |
| `attach <sc-ID> --url <URL> [--title]` | `PATCH /stories/{id}` with `external_links_add` | Stories only. `--file` exits with a usage error that says upload is not supported yet. |

Every subcommand takes `-q`/`--quiet` and accepts Shortcut URLs as keys. A
story-only subcommand given an epic key gets `NotAStory` from
`ShortcutKey::into_story`. `shortcut-cli` turns it into a usage error that
lists `show`, `update` and `comment`.

#### 3. Flow tests

**Files**: `cli/shortcut-cli/tests/flow_{create,update,comment,transition,attach}.rs`
**Changes**: Raise the pinned count in `scenario_inventory.rs`. Cases come
first for each subcommand:
- update sends only the given fields;
- a 403 from a read-only token produces a message that names the scope;
- a create that times out is not resent;
- a failed create leaves the work item file unchanged;
- create with kind `epic` calls `POST /epics`;
- `comment add` on an epic key reaches `/epics/{id}/comments`;
- transition to an ambiguous name lists both states;
- transition on a story whose workflow is missing from the catalogue names
  the workflow and `init discover`;
- `update --state` with a state from another workflow is refused;
- `update --state` and `--estimate` on an epic key give a usage error;
- transition and attach on an epic key give a usage error;
- attach with an http URL is refused;
- `attach --file` gives the "upload not supported yet" usage error.

#### 4. Write skills

**Files**: `skills/integrations/shortcut/{create,update,transition,comment,attach}-shortcut-story/SKILL.md`
**Changes**:
- Each skill is not model-invocable.
- Each previews the resolved intent and confirms before writing.
- The create, update and comment descriptions say they handle epics too.
- Add their subcommands to `tasks/shared/skill_write_gate.py`.
- Raise `EXPECTED_INJECTION_SKILLS` by 5.

### Success Criteria

#### Automated Verification

- [ ] Client tests pass: `cargo nextest run -p shortcut-client`
- [ ] Binary tests pass: `cargo nextest run -p shortcut-cli`
- [ ] Skill lints, including the write gate, pass: `mise run lint:integration-skills:check` and `mise run lint:skill-permissions:check`
- [ ] Full run passes: `mise run`

#### Manual Verification

- [ ] `/accelerator:create-shortcut-story` on a local work item creates the
      story and writes `external_id`.
- [ ] `/accelerator:transition-shortcut-story sc-<id> "In Progress"` moves
      the story.
- [ ] `/accelerator:update-shortcut-story sc-<id> --title "x"` leaves the
      description unchanged.
- [ ] `/accelerator:attach-shortcut-story sc-<id> --url https://example.com`
      adds the link.
- [ ] `/accelerator:comment-shortcut-story` adds a comment.

---

## Phase 7: Documentation and follow-ups

### Overview

Users can find and configure the connector from the docs. The follow-up work
items are recorded.

### Changes Required

- `docs-site/src/content/docs/skills/issue-trackers.mdx`: add a Shortcut
  column and tab.
- A page for the `accelerator shortcut` sub-binary under
  `docs-site/src/content/docs/`, with an `ACCELERATOR_SHORTCUT_BIN` override
  row, and its entry in `docs-site/astro.config.mjs`.
- `README.md`: name Shortcut at `:89`, and add the sub-binary page to the
  Concepts list under `## Documentation`.
- `docs-site/src/content/docs/guides/sync-work-items.mdx`,
  `guides/configuration-cookbook.md`, `guides/which-skill.md` and
  `configuration.md`: add Shortcut where Linear appears.
- `CHANGELOG.md`: add an entry.
- `templates/work-item.md:19`: mention `sc-1234` and `epic-56` in the
  `external_id` comment.
- Create these work items under epic 0181:
  - Shortcut file upload for `attach`;
  - Shortcut parent/child sync, including a story's `epic_id`, as a child of
    0291;
  - Shortcut epics in discovery, blocked by 0290;
  - Shortcut status and priority mapping, as input to 0290;
  - a tracker registry that replaces the string matches on
    `work.integration` with one exhaustive enum;
  - a review of client-crate duplication across the three providers.

### Success Criteria

#### Automated Verification

- [ ] Docs build and link check pass: `mise run docs:check`
- [ ] Full run passes: `mise run`

#### Manual Verification

- [ ] The issue-trackers page renders the Shortcut tab on desktop and phone
      widths.

---

## Testing Strategy

### Unit Tests

- `ShortcutKey` parsing and rendering for both variants: case folding,
  refusal of bare integers and leading zeros, `collection` and `into_story`.
- `NewEntity::for_kind` and `story_type_for_kind`, one case per table row.
- `url_is_allowed`, including lookalike hosts, http, userinfo, ports and
  loopback.
- The transport's `WhileRetryable` and `OnlyWhenThrottled` policies, counted
  on the mock, plus `may_have_applied`, Retry-After, the deadline, redirect
  refusal, body bounds and the `next_page_url` check.
- Each write module's declared resend policy, counted on the mock.
- Classification for every outcome on a read and on each write operation,
  with and without `may_have_applied`.
- Catalogue load, validation, schema versions, unknown-field merging, the
  lock and the refusal to write an incomplete catalogue.
- Team identity: team keys, the null-mention fallback, ambiguity, and
  `resolve_scope` for base-only and broadened scopes.
- Search results kept only for the scoped team uuid.
- `fetch_all` chunking, mixed entities, archived entities, requested-form
  pairing and partial reads.
- Filter lowering, quoting, state validation across teams, and kind names in
  `type`.
- Each Phase 4 match-site decision.

### Integration Tests

- The offline contract suite, for a story subject and an epic subject, in the
  default profile.
- Engine tests in `work-adapters` against a real `ShortcutClient` and a mock
  server.
- Scenario-driven flow tests for every `accelerator shortcut` subcommand,
  with the scenario inventory guard.
- Evidence hygiene over the committed evidence and fixtures.
- The live contract suite against a scratch workspace, outside the default
  run.

### Manual Testing Steps

1. Build the checkout and start Claude Code with it as the plugin, as
   `CONTRIBUTING.md` describes. Set `ACCELERATOR_WORK_BIN` and
   `ACCELERATOR_SHORTCUT_BIN` to the local builds. Disable the installed
   plugin through `--settings`.
2. Create a scratch Shortcut workspace with one team, and an `sct_rw_` token.
3. Put the token in `.accelerator/config.local.md` as `shortcut.token`.
4. Run `/accelerator:init-shortcut`.
5. Create three local work items: one `bug`, one `story` and one `epic`.
6. Run `/accelerator:sync-work-items` and push all three.
7. Check in Shortcut that there is a bug, a feature and an epic.
8. Check that the epic's work item has `external_id: "epic-<id>"`.
9. Edit the story's and the epic's descriptions in Shortcut. Sync again and
   check that both changes are pulled.
10. Replace the token with an `sct_r_` token. Run a push and check that each
    item fails as retryable, and that the message names the read-only scope.

## Performance Considerations

Shortcut allows 200 requests a minute. `fetch_all` asks for 100 ids a request,
so a 1,000-item corpus costs 10 requests. `search` adds one `GET /teams/{uuid}`
for each team in scope. Search pages are capped by
`<tracker>.pull.max_pages`. The client shares one deadline across all chunks
of an operation. Non-init commands make no catalogue requests.

## Migration Notes

No existing data changes. Repositories that do not set
`work.integration: shortcut` see no difference. No migration under
`cli/migrate` is needed, because Shortcut has no legacy scope key. A corpus
synced with another tracker must not switch to Shortcut in place; see What
We're NOT Doing.

## References

- Work item: `meta/work/0371-shortcut-integration.md`
- Parent epic: `meta/work/0181-work-management-additional-integrations.md`
- Review: `meta/reviews/plans/2026-11-13-0371-shortcut-integration-review-1.md`
- Linear and Jira client plan:
  `meta/plans/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`
- Binaries plan:
  `meta/plans/2026-08-19-0211-integration-binaries-and-bash-cluster-retirement.md`
- Decision register: `meta/work/0171-jira-and-linear-integrations.md`
- Identity: `meta/decisions/ADR-0070-tracker-owned-work-item-identity.md`
- Port: `cli/tracker/src/lib.rs:669-900`
- Jira classification: `cli/jira-client/src/classify.rs:50-75`
- Contract suite: `cli/tracker-test-support/src/contract.rs`
- Shortcut v4 reference: https://developer.shortcut.com/api/rest/v4
- Shortcut v4 OpenAPI: https://developer.shortcut.com/api/rest/v4/shortcut.openapi.json
