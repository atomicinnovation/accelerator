---
type: "codebase-research"
id: "2026-11-05-0338-linear-update-overwrites-unspecified-fields"
title: "Research: Linear Update Overwrites Fields It Was Not Given"
date: "2026-11-05T16:35:14+00:00"
author: "Phil Helm"
producer: "research-codebase"
status: "complete"
work_item_id: "0338"
parent: "work-item:0338"
relates_to: ["codebase-research:2026-10-03-cli-surface-inventory", "codebase-research:2026-10-10-0296-sync-round-trip-defects", "codebase-research:2026-06-14-0048-linear-integration-apis"]
topic: "Linear Update Overwrites Fields It Was Not Given"
tags: ["research", "codebase", "linear", "linear-cli", "linear-client", "tracker-port", "update"]
revision: "9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2"
repository: "accelerator-git"
last_updated: "2026-11-05T16:35:14+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# Research: Linear Update Overwrites Fields It Was Not Given

**Date**: 2026-11-05T16:35:14+00:00
**Author**: Phil Helm
**Git Commit**: 9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2
**Branch**: main
**Repository**: accelerator-git

## Research Question

How does `accelerator linear update` build and send its `issueUpdate` today?
What does the work item 0338 fix touch? Which constraints, consumers and test
patterns bound it? The work item is at
`meta/work/0338-linear-update-overwrites-fields-it-was-not-given.md`, and its
first review is at
`meta/reviews/work/0338-linear-update-overwrites-fields-it-was-not-given-review-1.md`.

## Summary

The code confirms every symptom in the work item. All of them come from one
function, `run_update`
([main.rs:299-342](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/src/main.rs#L299-L342)),
which works on two separate paths:

- **`--state` given.** It calls `LinearClient::transition` and returns early.
  `--title`, `--description`, `--assignee-id` and `--priority` are ignored
  without a warning. It prints the transition JSON with `"outcome": "updated"`.
- **No `--state`.** It falls back to `""` for an absent title or description
  (`unwrap_or_default()`) and calls `update_op(id, Some(&title), &body)`. So
  the request always sends `{"title": …, "description": …}`, even for flags
  that were never given. `--assignee-id` and `--priority` are used only by the
  no-ops guard and are never sent. No code in `cli/` builds a Linear input
  containing `assigneeId` or `priority`.

The history shows this is a regression, not a decision:

- The original 0048 design merged only the flags that were given into one
  `issueUpdate`, with `--state` resolved inside `update`. One of its
  acceptance criteria was "update setting title `T` + state `S` produces a
  payload with the catalogue-resolved `stateId` and title `T`"
  ([0048 plan:750-751](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/meta/plans/2026-06-15-0048-linear-integration.md#L750-L751)).
- No later plan records the decision to send `--state` through `transition`.
- The 2026-10-03 CLI surface inventory already logged the whole defect as
  discrepancy #7.

The fix has four constraints:

1. **`update_op` must keep its current input.** `update_op` *is* the tracker
   port's `RemoteTracker::update`. Three production callers depend on it:
   `work sync`, `work update --push` and promotion's retitle stage. The work
   item names only `work sync`. Tests in `contract_offline.rs` pin its input
   exactly.
2. **State resolution needs no network call.** It reads `catalogue.json` from
   disk when the client is built. So the only network request in the whole
   command is the `issueUpdate` mutation itself.
3. **Only one consumer reads the `--state` JSON:** the skill's Step 4
   ([SKILL.md:63-65](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/skills/integrations/linear/update-linear-issue/SKILL.md#L63-L65)).
4. **Exit `34` is `BAD_REQUEST` (`E_GQL_BAD_REQUEST`).** It covers any GraphQL
   validation error Linear returns, so an empty title rejected by Linear lands
   there.

## Detailed Findings

### CLI: `UpdateArgs` and `run_update`

`UpdateArgs` is defined at
[cli.rs:58-73](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/src/cli.rs#L58-L73):

| Flag | Type | Use today |
|---|---|---|
| `id` (positional) | `String` | Target identifier |
| `--title` | `Option<String>` | Guard; `unwrap_or_default()` → always sent |
| `--description` | `Option<String>` | Guard; `unwrap_or_default()` → always sent |
| `--state` | `Option<String>` | Guard; diverts to `transition` |
| `--assignee-id` | `Option<String>` | Guard only, never sent |
| `--priority` | `Option<i64>` | Guard only, never sent. A non-integer is a clap error (exit 2). |
| `-q`/`--quiet` | `bool` | Never read |

The steps of `run_update`
([main.rs:299-342](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/src/main.rs#L299-L342)):

1. **No-ops guard (`:300-308`).** If all five mutating flags are `None`, it
   prints `E_UPDATE_NO_OPS: at least one field to update is required` and
   exits with `UPDATE_NO_OPS` = 111
   ([exit_codes.rs:88](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/src/exit_codes.rs#L88)).
   This runs before any config, credential or client is touched. `--title ""`
   counts as given (`Some("")`), so it passes the guard. `--quiet` on its own
   still exits 111.
2. **Client build (`:309-312`).** `client_or_report(Intent::Write)` builds the
   client. As part of that it loads the catalogue
   ([client.rs:267](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-client/src/client.rs#L267)).
3. **The `--state` branch (`:313-329`).**
   - It calls `client.transition(&args.id, state)`.
   - On success it prints `print_json(with_outcome(response, "updated"))` and
     returns.
   - On failure it prints the `SurfaceError` message and exits through
     `exit_codes::for_surface`.
   - The comment at `:313-314` states the split as intended: "A state change
     routes through the transition mutation; a plain field update through the
     port's whole-item update."
4. **The plain branch (`:330-341`).**
   - It sets `title = unwrap_or_default()` and `body = unwrap_or_default()`,
     then calls `update_op(&id_of(&args.id), Some(&title), &body)`.
   - On success it prints `updated\t<id>` via `keywords::text_line`
     ([keywords.rs:19-21](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/src/keywords.rs#L19-L21)).
     The `<id>` is the argument as the user typed it, not an identifier read
     back from the response.
   - On failure it prints the fixed string `linear update failed` and exits
     through `exit_codes::for_failure`.

What each kind of invocation sends today:

| Invocation | `issueUpdate` input sent | Likely result |
|---|---|---|
| `--description "x"` | `{"title": "", "description": "x"}` | Linear rejects the empty title → exit 34 (observed on PP-714) |
| `--title "x"` | `{"title": "x", "description": ""}` | Exit 0, description erased |
| `--priority 1` / `--assignee-id X` | `{"title": "", "description": ""}` | Exit 34; the flag is never sent |
| `--state S --title "x"` | `{"stateId": "<uuid>"}` via `transition` | Exit 0, the title change is silently dropped |

The two error paths report differently:

- A plain failure prints `linear update failed` and hides the cause.
- A `--state` failure prints the surface error itself. For example, an unknown
  state prints `E_TRANSITION_STATE_NOT_IN_CATALOGUE: state "…" is not in the
  catalogue — run /init-linear to refresh it` and exits 122. An ambiguous one
  prints `E_TRANSITION_STATE_AMBIGUOUS` and exits 123
  ([exit_codes.rs:95-96, 161-162](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/src/exit_codes.rs#L161-L162)).
- Update has its own `UPDATE_NO_CATALOGUE` (113) and `UPDATE_BAD_STATE` (114)
  codes, along with `UPDATE_NO_KEY` (110) and `UPDATE_BAD_FLAG` (112). All four
  are declared at `exit_codes.rs:87-91` and **none is used**.

### Client: `update_op`, `retitle_op` and the tracker port

`update_op` is at
[client.rs:721-735](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-client/src/client.rs#L721-L735):

- It first runs `refuse_identifier_op(id, Operation::Update)`, which wraps
  `auth::check_identifier`. A refused identifier becomes `RequestInvalid`
  (exit 34), and nothing is sent.
- It builds the input with untyped `json!`: `None` gives `{"description":
  body}`, and `Some(t)` gives `{"title": t, "description": body}`. The
  description is always present. The input has no other fields at all.
- It sends through `call_op(UPDATE, …)`, which retries retryable failures and
  discards the response body.
- The `UPDATE` document
  ([client.rs:100-102](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-client/src/client.rs#L100-L102))
  selects `success issue { id identifier }`.

`retitle_op`
([client.rs:742-754](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-client/src/client.rs#L742-L754))
runs the same identifier check, sends the same document with input
`{"title": title}`, and has no other effect.

The port trait is `RemoteTracker`, in crate `tracker`
([lib.rs:669](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/tracker/src/lib.rs#L669)):

- `update(&self, id, title: Option<&str>, body: &str)` is documented as
  replacing the description, and the title only when one is given (`:699-722`).
- `retitle(&self, id, title)` is documented as leaving the description
  untouched (`:732-733`).
- Linear implements both at
  [client.rs:937-952](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-client/src/client.rs#L937-L952):
  `update` delegates to `update_op` and `retitle` delegates to `retitle_op`.

`update_op` has exactly two callers: the CLI's `run_update`, and the port's
`update`. The port's `update` has three production callers:

1. **`work sync`.** The sync code is in the `work-adapters` crate.
   - `Payload::of` maps each sync action to what gets sent
     ([sync/run.rs:1056-1074](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/work-adapters/src/sync/run.rs#L1056-L1074)):
     - `Push(Send)` and `Overwrite` send both title and body.
     - `Push(Withhold)` sends `title: None` with the body.
     - `Retitle` sends the title and no body.
   - Title-only actions go to `retitle`, which calls the port at
     `sync/apply.rs:851`. Actions with a body go to `push`, which calls the
     port's `update` at
     [sync/apply.rs:821](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/work-adapters/src/sync/apply.rs#L812-L833).
2. **`work update --push`.** `push_update` calls
   `tracker.update(&external_id, Some(&title), &body)`
   ([work-cli/src/update.rs:240](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/work-cli/src/update.rs#L240)).
3. **Promotion's retitle stage**, used by `work create --push` and
   `work promote`
   ([work-adapters/src/promotion.rs:730-737](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/work-adapters/src/promotion.rs#L730-L737)).

The work item says "`work sync` is unaffected". The constraint is wider: none
of these three port callers may see its input change.

### `transition` and the workflow-state catalogue

`transition` is in
[transition.rs:16-64](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-client/src/transition.rs#L16-L64):

- It defines its own `ISSUE_UPDATE` document, which also selects
  `state { name }`.
- `resolve_state(name)` (`:29-44`) resolves the name through
  `self.resolvers().team_states()`.
- It sends `{"id": identifier, "input": {"stateId": state_id}}` (`:58-61`).
  This is the same `issueUpdate` mutation, so `stateId` in this mutation is
  already proven against Linear.
- It does **not** run `check_identifier`, which `update_op` does.
- It returns the parsed response through `surface::interpret`, and the CLI
  prints that as JSON.

The catalogue is a file read from disk. There is no live lookup:

- `Catalogue::load(integrations_root)` reads
  `<integrations>/linear/catalogue.json` once, when the client is built
  ([catalogue/mod.rs:34-82](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-client/src/catalogue/mod.rs#L53-L82)).
- `TeamStates`
  ([catalogue/team_states.rs:25-67](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-client/src/catalogue/team_states.rs#L25-L67))
  holds the base team's non-archived states. It compares names with
  `trim().to_lowercase()`.
- Lookups return `NotFound`, `Resolved(id)` or `Ambiguous { count }`. A
  missing or damaged catalogue gives `NotCatalogued` and is reported as an
  unknown state (exit 122).
- `init discover` writes the file
  ([main.rs:466-533](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/src/main.rs#L466-L533)).

So "refuses before any request" holds for every request, not just the
`issueUpdate` mutation: an unknown or ambiguous state is refused before
anything is sent.

### Exit 34 and "linear update failed"

`BAD_REQUEST: u8 = 34`
([exit_codes.rs:47](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/src/exit_codes.rs#L47))
is in the shared transport/auth band (11-53). In the original design it is
`E_GQL_BAD_REQUEST`: "HTTP 400 GraphQL error that is *neither* rate limit,
complexity, **nor** authentication (validation / bad query)". It is never
retried
([0048 plan:223-228, 358-361](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/meta/plans/2026-06-15-0048-linear-integration.md#L213-L241)).

These outcomes map to 34
([exit_codes.rs:219-236](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/src/exit_codes.rs#L219-L236)):

- a 2xx response that carries `errors[]` classified as `BadRequest`
  ([classify.rs:54-92](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-client/src/classify.rs#L54-L92)
  orders the checks auth → complexity → ratelimited → otherwise bad request);
- an HTTP 400;
- `RequestInvalid` (a refused identifier, or a payload that would not
  serialise).

The plain path prints only `linear update failed`, so the reason Linear gave
is lost. Improving that is explicitly out of scope for 0338.

### Linear schema evidence for the combined input

- The repo has no copy of Linear's GraphQL schema: no `*.graphql` and no
  schema JSON.
- The 0048 API research records `issueUpdate` as updating "`title`,
  `description`, `stateId`, `assigneeId`, `priority`", sourced from
  linear.app/developers
  ([2026-06-14 research:408-410](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/meta/research/codebase/2026-06-14-0048-linear-integration-apis.md#L408-L410)).
  It gives no field types.
- Nothing in the repo states whether `priority` is a GraphQL `Int` or a
  `Float`. The CLI parses it as `i64`, and `json!` would serialise that as a
  JSON number, so `{"priority": 1}` rather than `"1"`.

### Prior art: an input built only from the fields given

- **Jira.** `jira-cli` `run_update`
  ([jira-cli/src/main.rs:372-458](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/jira-cli/src/main.rs#L372-L458))
  builds an `UpdateFields` struct of `Option`s, with its own `update_has_ops`
  guard (`:331-346`). `update_payload`
  ([jira-client/src/mutation.rs:204-259](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/jira-client/src/mutation.rs#L204-L259))
  adds each field only when it is `Some`. It also models explicit clearing as
  `FieldEdit::Clear`, which 0338 scopes out for Linear.
- **Linear.** `SearchArgs::filter_pairs`
  ([cli.rs:99-114](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/src/cli.rs#L99-L114))
  uses `filter_map` to drop flags that were not given. That is for search
  filters, not a mutation input.

### Consumers of `linear update` output

- **The only caller.** No skill, agent, script, hook or docs page runs
  `linear update` except `update-linear-issue`.
  - [SKILL.md:9](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/skills/integrations/linear/update-linear-issue/SKILL.md#L9)
    lists all six flags, including `--quiet`.
  - Step 4 (`:63-65`) treats either the `updated\t<identifier>` line or "the
    `outcome` field when a `--state` change renders JSON" as success. This
    sentence is the only reader of the JSON.
  - Step 4 also says "On any non-zero exit … no write was made". That stays
    true with a single `issueUpdate`.
- **Lint checks.**
  - `tasks/shared/skill_keyword_parity.py:22-99` checks that the outcome
    keywords a skill branches on match `cli/linear-cli/src/keywords.rs`.
    `outcome` is on its list of tokens that are never keywords.
  - `tasks/shared/skill_write_gate.py:19-20` requires a confirm step before
    `update`.
  - Both are unit-tested in `tests/unit/tasks/test_integration_skills.py`.
- **The transition skill is unaffected.** `transition-linear-issue` parses the
  JSON `outcome` from `linear transition`
  ([SKILL.md:58-62](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/skills/integrations/linear/transition-linear-issue/SKILL.md#L58-L62)),
  and `linear transition` does not change.
- **Docs site.** `docs-site/.../issue-trackers.mdx:28,30` links to the
  reference pages that are generated from `SKILL.md`. No prose page documents
  the `linear update` flags.
- **`CHANGELOG.md`.** `[Unreleased]` starts at :3, with `Changed` at :448 and
  `Fixed` at :702. The only Linear entry under `Fixed` is :727, about
  `work sync` comparing bodies by meaning. Nothing covers `linear update` yet.
- **Help text.** The frozen `update --help` text is
  `cli/linear-cli/tests/fixtures/cli_surface.golden:32-46`. It only needs to
  change if the flags or their help strings change.
- **Evals.** No Linear skill has any. The closest analogue is Jira's
  `update-jira-issue/evals/`.

### Test landscape and patterns

**Current coverage.**
[flow_update.rs](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/tests/flow_update.rs)
has two tests:

- `--title` with `--description`. It checks the body only by substring
  (`contains("issueUpdate")`), and checks that stdout is `updated\tBLA-1\n`.
- The no-ops refusal, which checks exit 111 and zero hits.

No flow test covers a single-field update, `update --state`,
`--assignee-id`, `--priority` or `--quiet`. Unknown and ambiguous state names
are tested only at client level
([linear-client/tests/transition.rs:105-203](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-client/tests/transition.rs#L105-L149)).

**Pins on `update_op`'s input**, i.e. what the port sends for sync:

- [contract_offline.rs:253-298](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-client/tests/contract_offline.rs#L253-L298)
  holds the **exact** pins: `retitle_sends_only_the_title` asserts
  `== json!({"title": "Retitled"})`, and
  `update_without_a_title_sends_only_the_description` asserts
  `== json!({"description": "Body\n"})`.
- [port.rs:210-238](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-client/tests/port.rs#L210-L238),
  which the work item cites, asserts `input.title` and `input.description`
  one key at a time. Extra keys would pass it, so it is a weaker pin than the
  work item suggests.
- `tracker-test-support/src/contract.rs:283-307` checks the round trip.

**Patterns to copy.**

- **Mock server.** `graphql_test_support::MockGraphQLServer`
  ([lib.rs:63-169](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/graphql-test-support/src/lib.rs#L63-L169))
  routes by operation name. Linear's mutations are anonymous, so the key is
  their first root field, `"issueUpdate"`.
  - `answer(op, Route)` stubs a response.
  - `hits(op)`, `bodies(op)` and `last_body(op)` inspect what was sent.
  - `http()` gives `MockHTTPServer`-wide hit counts.
  - The mock panics on drop if an operation arrived that it had no answer for.
- **Exact-input assertion.**
  `assert_eq!(sent["variables"]["input"], json!({...}))`, as in
  `contract_offline.rs:264-273` and, at flow level, `flow_init.rs:210-215`.
- **No request sent.**
  `assert_eq!(server.http().hits(&RequestKey::post("/graphql")), 0)`
  (`flow_init.rs:316`).
- **Refusal shape.** `flow_search.rs:210-223` asserts the exit code, a
  substring of stderr, and zero hits.
- **Catalogue seeding.**
  - `support::seed_catalogue` and `seed_legacy_catalogue`
    ([tests/support/mod.rs:35-73](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/tests/support/mod.rs#L35-L73))
    seed `BLA` with `{"id": "state-ip-uuid", "name": "In Progress"}`.
  - For the ambiguous case, write a catalogue with two base-team states that
    share a name. Copy `flow_init.rs:39-44`, because `write_catalogue` is
    private.
  - `flow_transition.rs:21-26` runs over both catalogue shapes.
- **How the binary is run.** Tests spawn the real binary in a tempdir through
  `support::run(dir, server.http(), args)`, which wires up the
  `ACCELERATOR_LINEAR_API_URL` loopback seam. Flow tests are behind
  `#![cfg(feature = "test-loopback")]`.
- **Gotcha: scenario count.**
  [tests/scenario_inventory.rs:13](https://github.com/atomicinnovation/accelerator/blob/9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2/cli/linear-cli/tests/scenario_inventory.rs#L13)
  pins `EXPECTED_SCENARIO_COUNT = 16`. A new scenario JSON file must bump it
  and be referenced from a test. Inline `MockGraphQLServer::answer` stubs
  avoid this.

## Code References

- `cli/linear-cli/src/main.rs:299-342`: `run_update`, the guard, the `--state`
  divert and the `unwrap_or_default()` calls.
- `cli/linear-cli/src/main.rs:239-261`: `run_transition`, which shares the
  `--state` branch's shape.
- `cli/linear-cli/src/cli.rs:58-73`: `UpdateArgs`.
- `cli/linear-cli/src/exit_codes.rs:47, 87-96, 134-190, 219-236`:
  `BAD_REQUEST` 34, the update and transition codes, `for_failure`,
  `for_surface` and the outcome mapping.
- `cli/linear-cli/src/keywords.rs:19-31, 85-97`: `text_line`, `with_outcome`
  and `Update::Updated`.
- `cli/linear-client/src/client.rs:100-102`: the `UPDATE` document.
- `cli/linear-client/src/client.rs:267`: catalogue loading.
- `cli/linear-client/src/client.rs:721-754`: `update_op` and `retitle_op`.
- `cli/linear-client/src/client.rs:900-911`: `refuse_identifier_op`.
- `cli/linear-client/src/client.rs:926-952`: `impl RemoteTracker for
  LinearClient`.
- `cli/linear-client/src/transition.rs:16-64`: the `ISSUE_UPDATE` document,
  `resolve_state` and `transition`.
- `cli/linear-client/src/catalogue/team_states.rs:25-67`: state resolution.
- `cli/linear-client/src/surface.rs:59-69, 125-156`: the unknown and ambiguous
  state messages, and `interpret`.
- `cli/tracker/src/lib.rs:669-733`: the `RemoteTracker` `update` and
  `retitle` contract.
- `cli/work-adapters/src/sync/run.rs:1056-1074, 1485-1505`: how sync builds
  its payload and dispatches it.
- `cli/work-adapters/src/sync/apply.rs:812-858`: the port calls for push and
  retitle.
- `cli/work-cli/src/update.rs:240` and
  `cli/work-adapters/src/promotion.rs:730-737`: the other port `update`
  callers.
- `cli/jira-client/src/mutation.rs:204-259`: Jira's payload built from only
  the fields given.
- `cli/linear-cli/tests/flow_update.rs`: the current update flow tests.
- `cli/linear-client/tests/contract_offline.rs:253-298`: the exact pins on the
  port's input.
- `cli/linear-client/tests/port.rs:210-238`: the per-key pins on the port's
  `update`.
- `skills/integrations/linear/update-linear-issue/SKILL.md:63-65`: the only
  reader of the `--state` JSON `outcome`.

## Architecture Insights

- **`update_op` and the port's `update` are one function.** The CLI
  subcommand reuses the port adapter's whole-item update. Its semantics
  (always replace the description, replace the title when given) suit sync and
  are wrong for an edit command where any field may be absent. The narrow fix
  is to give the CLI its own client operation over a domain value of optional
  edits (title, description, state, assignee, priority) and leave `update_op`
  and the port untouched. This matches how `retitle_op` was added alongside
  `update_op` in 0296 rather than changing it.
- **`--state` depends on `transition`, but needs only its resolver.** Folding
  the state into the edit needs `resolve_state` (catalogue-only), not the
  `transition` method. Its response shape (`state { name }`) and its JSON
  output are what disappear.
- **Where the identifier check stands after the fix.** `update_op` and
  `retitle_op` refuse an unsafe identifier before sending. `transition` does
  not. A single edit operation built like `update_op` would apply the check to
  state changes too.
- **Two exit-code vocabularies for the same refusal.** `update --state`
  reports the transition codes 122 and 123 today. Update's own codes 113 and
  114 exist and are unused. Folding the state into `update` makes it natural to
  choose between them deliberately (see Open Questions).
- **Error reporting changes on the `--state` path.** A wire failure during a
  `--state` update prints the surface error today. Once that update goes
  through the same path as a plain update, it will print
  `linear update failed`. The catalogue refusals happen before the request, so
  they can keep their specific `E_…` stderr lines.

## Historical Context

- `meta/plans/2026-06-15-0048-linear-integration.md:699-705, 750-751, 761`:
  the original `linear-update-flow.sh` design.
  - It did an "incremental `jq -n` merge" over title, description, state,
    assignee and priority, with the state resolved inside `update`.
  - Acceptance criterion: title `T` with state `S` → one payload carrying both.
  - Exit codes 110-117 belong to update, 120-126 to transition, and 34 is
    `E_GQL_BAD_REQUEST` (:213-241).
  - State resolution needs no live lookup (:144-145, :778-789).
- `meta/plans/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`:
  Phase 6b (:2508-2617) and Phase 9 (:3172-3188, :628-634) introduced
  `StateResolver`/`CatalogueStates` and the
  `E_TRANSITION_STATE_NOT_IN_CATALOGUE` and `_AMBIGUOUS` errors. Neither phase
  discusses how `update --state` should work alongside the other flags.
- `meta/plans/2026-08-19-0211-integration-binaries-and-bash-cluster-retirement.md:860-864`:
  maps `linear-update-flow.sh` to `linear update`. No plan records the switch
  to routing through `transition`. Since the bash design merged state with the
  other fields, the current behaviour is an unrecorded regression, not a
  faithful port of the bash script.
- `meta/plans/2026-10-10-0296-sync-round-trip-defects.md`:
  - added `RemoteTracker::retitle` (:871-884): "a tracker that changed the
    description to change the title would defeat it";
  - added the tests `retitle_sends_only_the_title` (:743-744) and
    `update_without_a_title_sends_only_the_description` (:2119-2121);
  - set out the retitle/push decision table (:1967-1999).
  These pin the port's input.
- `meta/research/codebase/2026-10-03-cli-surface-inventory.md:84-85, 696`:
  logged the whole defect as discrepancy #7, without prescribing a fix.
- `meta/research/codebase/2026-06-14-0048-linear-integration-apis.md:395, 408-410`:
  records the `IssueUpdateInput` fields and the catalogue resolving states
  with no live lookup.
- No ADR governs Linear CLI output or exit-code contracts. The nearest are
  `ADR-0045` (how work is divided between skills and the CLI) and `ADR-0053`
  (thin CLI over a hexagonal core).

## Related Research

- `meta/research/codebase/2026-10-03-cli-surface-inventory.md`
- `meta/research/codebase/2026-10-10-0296-sync-round-trip-defects.md`
- `meta/research/codebase/2026-06-14-0048-linear-integration-apis.md`
- `meta/research/codebase/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`
- `meta/research/codebase/2026-08-17-0211-integration-binaries-and-bash-cluster-retirement.md`

## Open Questions

- **Which codes should state refusals use?** Should `update` keep the
  transition codes 122 and 123 and their `E_TRANSITION_…` stderr names for
  unknown and ambiguous states? Or should it adopt the unused
  `UPDATE_BAD_STATE` (114) and `UPDATE_NO_CATALOGUE` (113)? Today's behaviour
  is 122/123. The work item's "refuses before any request" criterion names no
  code.
- **What is `priority`'s GraphQL type?** Nothing in the repo records whether
  `IssueUpdateInput.priority` is an `Int` or a `Float`, or what range it
  accepts. Sending an `i64` as a JSON number is consistent with either.
  Neither `assigneeId` nor `priority` has ever been sent by this codebase, so
  the combined-input assumption is backed only by the 0048 API research's
  field list. Verifying it needs a run against real Linear, for example a
  merge-time check on a scratch issue.
- **What should `--quiet` do?** It is declared on `update` (and `create` and
  `transition`) and advertised by the skill, but never read. Once
  `updated\t<KEY>` is the only success output, should `--quiet` suppress it,
  or stay a no-op? Today it is a no-op.
- **Should the work item's wording change?** It says "`work sync` is
  unaffected" and cites `port.rs:224-237` as the pin. The constraint covers all
  three port callers (`work sync`, `work update --push`, promotion). The exact
  pins are in `contract_offline.rs:253-298`.
