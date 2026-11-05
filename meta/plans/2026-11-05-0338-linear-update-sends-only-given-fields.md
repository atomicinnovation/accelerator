---
type: "plan"
id: "2026-11-05-0338-linear-update-sends-only-given-fields"
title: "Linear Update Sends Only the Fields It Is Given Implementation Plan"
date: "2026-11-05T16:51:32+00:00"
author: "Phil Helm"
producer: "create-plan"
status: "draft"
work_item_id: "work-item:0338"
parent: "work-item:0338"
derived_from: ["codebase-research:2026-11-05-0338-linear-update-overwrites-unspecified-fields"]
tags: ["linear", "cli", "update"]
revision: "9a3f77a1c7a3b3d1db560d7848c8cb49ca6f19f2"
repository: "accelerator-git"
last_updated: "2026-11-05T17:17:17+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# Linear Update Sends Only the Fields It Is Given Implementation Plan

## Overview

`accelerator linear update` sends fields the caller never named and drops
fields the caller did. This plan gives `linear-client` one update operation,
`update_op`, over a domain value of optional fields, `UpdateFields`. It sends
one `issueUpdate` whose input holds exactly the fields named, as Jira's
`update_op(&UpdateFields)` already does. The tracker port's `update` and
`retitle` delegate to it with the same input they send today. `linear update`
then builds its `UpdateFields` from exactly the flags given, with `--state`
resolved through the catalogue into the same input.

## Current State Analysis

`run_update` (`cli/linear-cli/src/main.rs:299-342`) has two paths:

- **`--state` given.** It calls `LinearClient::transition`, prints the
  transition's JSON with `"outcome": "updated"`, and returns. `--title`,
  `--description`, `--assignee-id` and `--priority` are silently dropped.
- **No `--state`.** It turns an absent title or description into `""` with
  `unwrap_or_default()` and calls `update_op(id, Some(&title), &body)`. The
  input is therefore always `{"title": …, "description": …}`. `--assignee-id`
  and `--priority` feed only the no-ops guard and are never sent.

`update_op` and `retitle_op` (`cli/linear-client/src/client.rs:716-754`) each
hand-build an `IssueUpdateInput` over the same `UPDATE` document, and back the
port's `update` and `retitle` (`client.rs:937-952`). What the port sends is
pinned exactly for `retitle` and for `update` without a title
(`cli/linear-client/tests/contract_offline.rs:252-298`). `update` with a title
is pinned only field by field (`cli/linear-client/tests/port.rs:212-237`).

## Desired End State

- `linear-client` has one update operation, `update_op(&ExternalId,
  &UpdateFields)`. `retitle_op` is gone. The port's `update` and `retitle`
  delegate to `update_op`, and send byte-for-byte the input they send today.
- `update_op` refuses an `UpdateFields` that names no field before any
  request.
- `linear update <KEY> [flags]` sends exactly one `issueUpdate` mutation.
- That mutation's `input` holds exactly the fields named:
  - `title`, `description` and `assigneeId` as the strings typed, empty ones
    included;
  - `priority` as a JSON integer;
  - `stateId` as the catalogue-resolved UUID.
- An unknown or ambiguous `--state` name is refused before any request. It
  exits 122 or 123 with the existing `E_TRANSITION_STATE_…` stderr line, as
  today.
- Every successful update prints `updated\t<KEY>`, with or without `--state`
  or `--quiet`.
- A failed update, with or without `--state`, prints `linear update failed`
  and exits with the port-failure code for its outcome.
- No mutating flag still refuses with `E_UPDATE_NO_OPS` (exit 111) before any
  request.
- The `update-linear-issue` skill no longer mentions a JSON `outcome`, flags a
  cleared field in its preview, and no longer claims that every failure left
  the issue unwritten.
- `CHANGELOG.md` `[Unreleased]` carries a `Fixed` entry naming the
  erased-description case and a `Breaking` entry for `--state`'s output and
  failure reporting.

Verify with `mise run` (the bare default task) exiting 0, plus Phase 3's
check against real Linear.

### Key Discoveries:

- Jira's `UpdateFields` and `update_op` (`cli/jira-client/src/mutation.rs:53-73,
  165-260`) are the prior art. The port's `update` and `retitle` both delegate
  there through `edit_summary_and_body` (`cli/jira-client/src/client.rs:638-663,
  713-728`).
- `LinearClient::resolve_state` (`cli/linear-client/src/transition.rs:29-44`)
  is public and reads only the on-disk catalogue. A state can therefore be
  resolved, or refused, with no request.
- `transition.rs` is the precedent for a capability module owning its
  document and an `impl LinearClient` block. `call_op` and
  `refuse_identifier_op` are private to `client.rs` and become `pub(crate)` so
  `update.rs` can own the update operation.
- `SurfaceError::UnknownState` and `AmbiguousState` already map to 122 and 123
  through `exit_codes::for_surface` (`cli/linear-cli/src/exit_codes.rs`).
- A `--state` failure moves from `for_surface` to `for_failure`. The two
  classify some responses differently (for example, a 401 without a JSON
  body), so some `--state` failure exit codes change.
- Flow tests run the real binary against `MockHTTPServer` via
  `support::run`. `issue-update-200.json` answers one `issueUpdate` and
  `bad-request-400.json` answers a 400 that exits 34. Reusing both avoids
  bumping `EXPECTED_SCENARIO_COUNT` in `scenario_inventory.rs`.
- `linear-client` has no cargo-public-api snapshot, so the new public type and
  the changed `update_op` signature need no snapshot update.

## What We're NOT Doing

- Changing what the port's `update` or `retitle` send, so nothing `work sync`,
  `work update --push` or promotion sends changes.
- Changing `linear transition` or its JSON output.
- Improving the `linear update failed` message.
- Adopting the unused `UPDATE_BAD_STATE` (114) or `UPDATE_NO_CATALOGUE` (113)
  codes. State refusals keep 122 and 123.
- Making `--quiet` do anything. It stays a no-op.
- Validating `--priority` or `--assignee-id` locally, or adding a way to
  unassign.
- Changing `update`'s flags or help text, so `cli_surface.golden` is
  unchanged.

## Implementation Approach

Test-first throughout. Phase 1 consolidates the client onto one
`update_op(&UpdateFields)`. It changes no behaviour: the port sends what it
did, and `run_update` mechanically builds the same title-and-description
input, so the phase can merge alone. Phase 2 makes `run_update` send only the
flags given, and updates the skill and CHANGELOG in the same phase, because
the skill reads the output that phase changes. Phase 3 confirms against real
Linear the combined-input assumption that no test double can prove.

## Phase 1: One `update_op` Over `UpdateFields`

### Overview

Add a domain value for the fields an update names. Replace `update_op` and
`retitle_op` with one operation over it, and have the port and `run_update`
delegate to it with the inputs they send today.

### Changes Required:

#### 1. Pin the port's titled update exactly

**File**: `cli/linear-client/tests/contract_offline.rs`
**Changes**: Add `update_with_a_title_sends_the_title_and_description`, beside
`update_without_a_title_sends_only_the_description` and in its shape. It
asserts that `tracker().update(&id, Some("Updated"), "Body\n")` sends exactly
`{"title": "Updated", "description": "Body\n"}`. It passes against today's
code. With the two existing exact pins, it is the safety net for the port
delegation below. No existing test in `contract_offline.rs` or `port.rs` is
edited.

#### 2. Failing client-level tests

**File**: `cli/linear-client/tests/update.rs` (new)
**Changes**: Script `MockHTTPServer` with one `issueUpdate` answer, as
`contract_offline.rs` does with `scripted`/`json`. Build the client with
`support::client::client_with`. Call `update_op`, then assert on
`sent["variables"]["input"]` with `assert_eq!` against an exact `json!`, and
check that `sent["variables"]["id"]` is the identifier. The cases:

- a title alone sends `{"title": "Renamed"}`;
- a description alone sends `{"description": "Body"}`;
- an empty description sends `{"description": ""}`;
- a state id alone sends `{"stateId": "state-ip-uuid"}`;
- an assignee alone sends `{"assigneeId": "user-alice-uuid"}`;
- a priority alone sends `{"priority": 1}`, a JSON number;
- every field together sends all five keys and no others;
- `UpdateFields::default()` returns `LinearFailure::Wire` with outcome
  `Outcome::RequestInvalid` and sends nothing (`server.hits(&key) == 0`);
- an identifier the shared rule refuses (for example `"---ENG-1"`) returns
  `LinearFailure::Wire` with outcome `Outcome::RequestInvalid` and sends
  nothing;
- a 200 response carrying a validation `errors[]` returns
  `LinearFailure::Wire` with outcome
  `Outcome::SuccessWithErrors(GraphQlError::BadRequest)`.

Match failures the way `tests/discriminant.rs` matches outcomes.

```rust
fn sent_input(server: &MockHTTPServer) -> Value {
    let key = RequestKey::post(GRAPHQL);
    assert_eq!(server.hits(&key), 1, "exactly one issueUpdate is sent");
    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("an update"))
            .expect("a JSON request");
    sent["variables"]["input"].clone()
}

#[test]
fn a_title_alone_is_the_whole_input() {
    let (server, client) = scripted_update();

    client
        .update_op(
            &ExternalId::new("ENG-1".to_owned()),
            &UpdateFields {
                title: Some("Renamed"),
                ..UpdateFields::default()
            },
        )
        .expect("the update succeeds");

    assert_eq!(sent_input(&server), json!({"title": "Renamed"}));
}
```

#### 3. The domain value and its operation

**File**: `cli/linear-client/src/update.rs` (new), registered in
`cli/linear-client/src/lib.rs` as `pub mod update;` with
`pub use crate::update::UpdateFields;`
**Changes**: Move the `UPDATE` document here from `client.rs`, and replace
`update_op` and `retitle_op` with:

```rust
//! An update names only the fields it changes; Linear leaves every field its
//! input omits as it is.

use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use tracker::ExternalId;

use crate::classify::Operation;
use crate::classify::Outcome;
use crate::client::refuse_identifier_op;
use crate::client::LinearClient;
use crate::failure::LinearFailure;

const UPDATE: &str = "mutation($id: String!, $input: IssueUpdateInput!) {
    issueUpdate(id: $id, input: $input) { success issue { id identifier } }
  }";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UpdateFields<'a> {
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    /// A UUID from [`LinearClient::resolve_state`], never a state name.
    pub state_id: Option<&'a str>,
    pub assignee_id: Option<&'a str>,
    pub priority: Option<i64>,
}

impl UpdateFields<'_> {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.state_id.is_none()
            && self.assignee_id.is_none()
            && self.priority.is_none()
    }

    fn input(&self) -> Value {
        let named = [
            ("title", self.title.map(Value::from)),
            ("description", self.description.map(Value::from)),
            ("stateId", self.state_id.map(Value::from)),
            ("assigneeId", self.assignee_id.map(Value::from)),
            ("priority", self.priority.map(Value::from)),
        ];
        Value::Object(
            named
                .into_iter()
                .filter_map(|(key, value)| {
                    value.map(|value| (key.to_owned(), value))
                })
                .collect::<Map<_, _>>(),
        )
    }
}

impl LinearClient {
    /// `update`, sending only the fields `fields` names, surfacing the
    /// structured discriminant. The port's `update` and `retitle` delegate
    /// here.
    ///
    /// # Errors
    ///
    /// [`LinearFailure`] for a refused identifier, an update naming no field,
    /// or a wire failure.
    pub fn update_op(
        &self,
        id: &ExternalId,
        fields: &UpdateFields<'_>,
    ) -> Result<(), LinearFailure> {
        refuse_identifier_op(id, Operation::Update)?;
        if fields.is_empty() {
            return Err(LinearFailure::wire(
                Outcome::RequestInvalid,
                Operation::Update,
                "an update must name at least one field".to_owned(),
            ));
        }
        let variables = json!({"id": id.as_str(), "input": fields.input()});
        self.call_op(UPDATE, &variables, Operation::Update, id.as_str())?;
        Ok(())
    }
}
```

**File**: `cli/linear-client/src/client.rs`
**Changes**: Delete `UPDATE`, `update_op` and `retitle_op`. Make `call_op` and
`refuse_identifier_op` `pub(crate)`. The port delegates:

```rust
fn update(
    &self,
    id: &ExternalId,
    title: Option<&str>,
    body: &str,
) -> Result<(), TrackerError> {
    self.update_op(
        id,
        &UpdateFields {
            title,
            description: Some(body),
            ..UpdateFields::default()
        },
    )
    .map_err(TrackerError::from)
}

fn retitle(
    &self,
    id: &ExternalId,
    title: &str,
) -> Result<(), TrackerError> {
    self.update_op(
        id,
        &UpdateFields {
            title: Some(title),
            ..UpdateFields::default()
        },
    )
    .map_err(TrackerError::from)
}
```

#### 4. `run_update` keeps today's input

**File**: `cli/linear-cli/src/main.rs`
**Changes**: Only the call changes, so behaviour is unchanged and
`flow_update.rs` passes unedited:

```rust
let fields = UpdateFields {
    title: Some(title.as_str()),
    description: Some(body.as_str()),
    ..UpdateFields::default()
};
match client.update_op(&id_of(&args.id), &fields) {
```

Add `UpdateFields` to the existing `use linear_client::{…}` import.

### Success Criteria:

#### Automated Verification:

- [ ] The new suite passes:
      `cargo test --manifest-path cli/Cargo.toml -p linear-client --test update`
- [ ] The port pins pass, with no existing test edited:
      `cargo test --manifest-path cli/Cargo.toml -p linear-client --test contract_offline --test port`
- [ ] The update flow passes unedited:
      `cargo test --manifest-path cli/Cargo.toml -p linear-cli --features test-loopback --test flow_update`
- [ ] Format, lint and types pass: `mise run cli:check`
- [ ] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [ ] `UpdateFields`'s field names read as the domain speaks of an update, and
      `update.rs` carries no comment beyond its module doc, the `state_id`
      field doc and `update_op`'s rustdoc

---

## Phase 2: `linear update` Sends Only the Flags Given

### Overview

`run_update` builds `UpdateFields` from exactly the flags given. It resolves
`--state` into the same fields, refusing before any request, and reports every
success as `updated\t<KEY>`. The skill and CHANGELOG are updated to match.

### Changes Required:

#### 1. Failing flow tests first

**File**: `cli/linear-cli/tests/flow_update.rs`
**Changes**: Replace the substring-only title-and-description test with
exact-input tests. Keep the no-ops refusal test. Add a helper that parses the
one captured body:

```rust
fn sent_input(server: &MockHTTPServer) -> Value {
    let key = RequestKey::post("/graphql");
    assert_eq!(server.hits(&key), 1, "exactly one issueUpdate is sent");
    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("a JSON request");
    assert!(sent["query"].as_str().expect("a document").contains("issueUpdate"));
    sent["variables"]["input"].clone()
}
```

Each test installs `issue-update-200.json` and seeds the catalogue where
`--state` is used. It then asserts exit 0, stdout `b"updated\tBLA-1\n"`, and
the exact input:

| Test | Args after `update BLA-1` | Exact input |
|---|---|---|
| description alone | `--description x` | `{"description": "x"}` |
| title alone | `--title x` | `{"title": "x"}` |
| priority alone | `--priority 1` | `{"priority": 1}` |
| assignee alone | `--assignee-id user-alice-uuid` | `{"assigneeId": "user-alice-uuid"}` |
| empty description | `--description ""` | `{"description": ""}` |
| title and description | `--title Renamed --description "Rewritten body"` | `{"title": "Renamed", "description": "Rewritten body"}` |
| state with title, over both catalogue shapes | `--state "In Progress" --title x` | `{"stateId": "state-ip-uuid", "title": "x"}` |
| state alone with `--quiet` | `--state "In Progress" --quiet` | `{"stateId": "state-ip-uuid"}` |

Run the state-with-title test over `support::seed_catalogue` and
`support::seed_legacy_catalogue`, as `flow_transition.rs` does.

Add the refusals. Each asserts the exit code, a stderr substring, and
`server.hits(&RequestKey::post("/graphql")) == 0`:

- `--state Nowhere --title x` over `support::seed_catalogue` exits 122, and
  stderr contains `E_TRANSITION_STATE_NOT_IN_CATALOGUE`;
- `--state Nowhere --title x` with no catalogue seeded exits 122, and stderr
  contains `E_TRANSITION_STATE_NOT_IN_CATALOGUE`;
- `--state Done --title x` over a catalogue with two base-team states named
  `Done` exits 123, and stderr contains `E_TRANSITION_STATE_AMBIGUOUS`.

Add the failure: with `bad-request-400.json` installed and
`support::seed_catalogue`, `--state "In Progress" --title x` exits 34, stdout
is empty, stderr contains `linear update failed`, and exactly one request is
sent.

**File**: `cli/linear-cli/tests/support/mod.rs`
**Changes**: Add `seed_ambiguous_catalogue(dir)` beside `seed_catalogue`. It
uses the private `write_catalogue` to seed base team `BLA` with two states
named `Done` (`state-done-a-uuid`, `state-done-b-uuid`).

#### 2. The no-ops guard reads as the domain

**File**: `cli/linear-cli/src/cli.rs`
**Changes**: Give `UpdateArgs` a `names_no_field` method, replacing the
five-way `is_none()` chain in `run_update`. It refuses with exit 111 before
any credential or catalogue work. `update_op`'s own refusal of an empty
`UpdateFields` backs it up.

```rust
impl UpdateArgs {
    pub const fn names_no_field(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.state.is_none()
            && self.assignee_id.is_none()
            && self.priority.is_none()
    }
}
```

#### 3. `run_update` sends only the flags given

**File**: `cli/linear-cli/src/main.rs`
**Changes**: Remove the `transition` branch and its comment, and the
`unwrap_or_default()` calls. `run_update` now only borrows its arguments, so
it takes `&UpdateArgs` and the dispatch becomes `run_update(&args)`. Resolve
`--state` before building the fields, so a refusal happens before any request:

```rust
fn run_update(args: &UpdateArgs) -> ExitCode {
    if args.names_no_field() {
        eprintln!("E_UPDATE_NO_OPS: at least one field to update is required");
        return ExitCode::from(exit_codes::UPDATE_NO_OPS);
    }
    let client = match client_or_report(Intent::Write) {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    let state_id =
        match resolve_requested_state(&client, args.state.as_deref()) {
            Ok(state_id) => state_id,
            Err(code) => return code,
        };
    let fields = UpdateFields {
        title: args.title.as_deref(),
        description: args.description.as_deref(),
        state_id: state_id.as_deref(),
        assignee_id: args.assignee_id.as_deref(),
        priority: args.priority,
    };
    match client.update_op(&id_of(&args.id), &fields) {
        Ok(()) => {
            keywords::text_line(keywords::Update::Updated.keyword(), &args.id);
            ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("linear update failed");
            ExitCode::from(exit_codes::for_failure(&failure))
        }
    }
}

fn resolve_requested_state(
    client: &LinearClient,
    name: Option<&str>,
) -> Result<Option<String>, ExitCode> {
    name.map(|name| client.resolve_state(name))
        .transpose()
        .map_err(|error| {
            eprintln!("{error}");
            ExitCode::from(exit_codes::for_surface(&error))
        })
}
```

#### 4. The skill previews cleared fields and reads one success shape

**File**: `skills/integrations/linear/update-linear-issue/SKILL.md`
**Changes**: In Step 2, after the paragraph listing the fields being set,
add:

```markdown
A field given an empty value is cleared, not left alone. Show it as cleared,
for example `description: (cleared — the existing description is erased)`.
```

In Step 4, replace the paragraph after the command block with:

```markdown
The subcommand reports the `updated` keyword on success: a trailing
`updated\t<identifier>` line. Confirm the updated fields and suggest
`/show-linear-issue <IDENTIFIER>` to verify.

Exits 111, 122 and 123 are refusals before any request, so no write was made.
On any other non-zero exit, report the error and suggest `/show-linear-issue
<IDENTIFIER>` to check whether the update landed: a timeout or server failure
can follow a write Linear applied.
```

Steps 1 and 3 are unchanged.

#### 5. CHANGELOG

**File**: `CHANGELOG.md`
**Changes**: Under `[Unreleased]`, append to the end of `### Breaking`:

```markdown
- **`linear update --state` reports and fails like every other
  `linear update`.** It prints `updated\t<KEY>` instead of the issue as JSON,
  so the resulting state name is no longer echoed; read it with `linear show`.
  A failed `--state` update prints `linear update failed` and exits with the
  code a plain `linear update` would, which differs from before for some
  failures, such as a 401 without a JSON body. An identifier the shared rule
  refuses is now refused before sending. Scripts that parse
  `linear update --state` output or exit codes need updating.
```

Append to the end of `### Fixed`:

```markdown
- **`linear update` changes only the fields it is given.** Without `--title`
  it sent an empty title, which Linear refused; without `--description` it
  sent an empty description, which erased the issue's body. `--assignee-id`
  and `--priority` are now sent, and `--state` joins the same update instead
  of discarding the other flags.
```

### Success Criteria:

#### Automated Verification:

- [ ] The update flow passes:
      `cargo test --manifest-path cli/Cargo.toml -p linear-cli --features test-loopback --test flow_update`
- [ ] The surrounding linear-cli suites pass unedited (`flow_transition`,
      `flow_errors`, `stderr_diagnostics`, `keyword_surface`, `cli_surface`,
      `scenario_inventory`, `exit_codes_parity`):
      `cargo test --manifest-path cli/Cargo.toml -p linear-cli --features test-loopback`
- [ ] Skill lints pass: `mise run test:unit:tasks`
- [ ] Format, lint and types pass: `mise run check`
- [ ] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [ ] `/update-linear-issue` previews and confirms a title-only change, and
      the skill's text no longer mentions a JSON `outcome`
- [ ] `/update-linear-issue <KEY> --description ""` previews the description
      as cleared
- [ ] The generated reference page for `update-linear-issue` reflects the new
      Steps 2 and 4 (`mise run docs:check`)

---

## Phase 3: Merge-Time Check Against Real Linear

### Overview

The flow tests prove what is sent, not that Linear accepts it.
`stateId`-in-`issueUpdate` is proven by `transition`, but this codebase has
never sent `assigneeId` or `priority`. Linear's `IssueUpdateInput` documents
`assigneeId: String` and `priority: Int` (0–4), and this phase confirms that
against the live API.

### Changes Required:

No code. Run in a repo configured for Linear with a fresh catalogue
(`/init-linear`), using the release binary built from the Phase 2 branch.
Create the scratch issue first with
`accelerator linear create --title "0338 scratch"`, and use the key it prints
as `<KEY>` for every check below.

### Success Criteria:

#### Automated Verification:

- [ ] None. Phases 1 and 2 cover everything a test double can.

#### Manual Verification:

- [ ] `accelerator linear update <KEY> --description "x"` exits 0, and
      `linear show <KEY>` shows the title unchanged and the description `x`
- [ ] `accelerator linear update <KEY> --title "y"` exits 0, and the
      description is still `x`
- [ ] `accelerator linear update <KEY> --priority 1` exits 0, and the
      priority is Urgent with title and description unchanged
- [ ] `accelerator linear update <KEY> --state "<a catalogued state>"
      --priority 2 --assignee-id <your member id from the catalogue>` exits 0,
      and the state, priority and assignee all change in one update
- [ ] `accelerator linear update <KEY> --state "No Such State" --title "z"`
      exits 122, and the issue is unchanged
- [ ] `accelerator linear update <KEY> --assignee-id ""` behaves as recorded:
      note its exit code and whether the assignee changed, in the PR
      description
- [ ] The scratch issue is archived in Linear once the checks are done

---

## Testing Strategy

### Unit Tests:

- `cli/linear-client/tests/update.rs`: each field alone, all together, an
  empty description, an empty update, a refused identifier, and a GraphQL
  error. Exact-input assertions catch both extra and missing keys, and the
  helper asserts exactly one request.
- `contract_offline.rs` and `port.rs` keep every existing test unedited. With
  the new exact pin for a titled `update`, they guard that the port sends what
  it did before the delegation.

### Integration Tests:

- `cli/linear-cli/tests/flow_update.rs`: the real binary against a mock, one
  test per acceptance criterion, each asserting the exact input, the hit count
  and stdout. Refusals assert the exit code, stderr and zero hits. The failure
  test asserts exit 34, `linear update failed` and one hit.

### Manual Testing Steps:

1. Phase 3's scratch-issue commands against real Linear.
2. `/update-linear-issue <KEY> --title "…"` end to end, then
   `/show-linear-issue <KEY>` to confirm the description survived.

## Performance Considerations

None. The command still sends one request, and state resolution reads the
catalogue already loaded when the client is built.

## Migration Notes

None for stored data. A caller that parsed `linear update --state`'s JSON
must read `updated\t<KEY>` instead, and one that branched on its failure exit
codes must expect the plain update's codes. The only such caller in the repo
is the skill, which Phase 2 updates. The CHANGELOG `Breaking` entry records
both changes.

## References

- Original work item:
  `meta/work/0338-linear-update-overwrites-fields-it-was-not-given.md`
- Work item review:
  `meta/reviews/work/0338-linear-update-overwrites-fields-it-was-not-given-review-1.md`
- Plan review:
  `meta/reviews/plans/2026-11-05-0338-linear-update-sends-only-given-fields-review-1.md`
- Research:
  `meta/research/codebase/2026-11-05-0338-linear-update-overwrites-unspecified-fields.md`
- One-update-operation precedent: `cli/jira-client/src/mutation.rs:53-73,
  165-260` and `cli/jira-client/src/client.rs:638-663, 713-728`
- Capability-module precedent: `cli/linear-client/src/transition.rs`
- Original design (state merged into `update`):
  `meta/plans/2026-06-15-0048-linear-integration.md:699-705, 750-751`
