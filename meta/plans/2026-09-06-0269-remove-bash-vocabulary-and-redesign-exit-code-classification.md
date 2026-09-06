---
type: "plan"
id: "2026-09-06-0269-remove-bash-vocabulary-and-redesign-exit-code-classification"
title: "Remove Bash Vocabulary And Redesign Exit-Code Classification In Jira And Linear Clients Implementation Plan"
date: "2026-09-06T15:33:26+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0269"
parent: "work-item:0269"
derived_from: ["codebase-research:2026-09-06-0269-remove-bash-vocabulary-and-redesign-exit-code-classification"]
tags: ["jira", "linear", "cleanup", "refactor", "exit-codes", "classification"]
revision: "ee2b4e51bb328cf4a905599414bbb5c300972577"
repository: "accelerator"
last_updated: "2026-09-06T19:50:03+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Remove Bash Vocabulary And Redesign Exit-Code Classification In Jira And Linear Clients Implementation Plan

## Overview

Strip every residual "bash" reference from the `jira-client` and `linear-client`
crates, and redesign the exit-code classification path so the numeric map lives
at the CLI boundary rather than inside the client crates. The client crates keep
returning context-carrying failures (`JiraFailure`/`LinearFailure`, each holding
`{ outcome, operation, detail }`); the granular `outcome → u8` map moves into
`jira-cli`/`linear-cli`; and the retry-class collapse to `TrackerError` stays in
the client crates, re-expressed directly on `(Outcome, Operation)` with no `u16`
intermediary and no numeric code embedded in a `detail` string. Every emitted
exit-code value is preserved exactly — the parity fixtures and
`bridge-exit-code-tables.txt` are the oracle.

## Current State Analysis

The exit-code path is two functions doing triple duty inside each client crate's
`classify.rs`:

- **`bash_code(outcome) -> u16`** is the granular map (codes 11–36). Its only
  consumers are the two CLI `exit_codes.rs` (`code_for_outcome`/`code_for_status`,
  which narrow with `u8::try_from`) and outcome-keyed client tests. This is the
  map the work item moves to the CLI.
- **`classify_bash_code(code, op, detail) -> TrackerError`** is the retry-class
  map. Jira never calls it at runtime (`classify.rs:70` goes straight to
  `build`); it is test-only there. Linear calls it once (`classify.rs:155`) but
  only ever with the seven codes `bash_code` emits — its `18|23|25|27|29` and
  `110..=114` arms are dead at runtime.
- **`build(...)`** formats `"jira {op} ({code}): {detail}"`, embedding the
  numeric code in the detail string, then returns `Retryable`/`Terminal`.

The `RemoteTracker` port impl lives **inside the client crates**
(`jira-client/src/client.rs:470-648`, `linear-client/src/client.rs:598+`) —
there is no separate adapter crate. Each port method collapses a client failure
via `.map_err(TrackerError::from)`, relying on `From<Failure> for TrackerError`
(`failure.rs`), whose `Wire` arm delegates to `classify`. `work-cli` reaches
`TrackerError` only through this port impl and maps `Retryable`/`Terminal` to
dispatch codes 70/71 (`work-cli/src/exit_codes.rs`); it never sees
`JiraFailure`/`LinearFailure`. The standalone `jira-cli`/`linear-cli` binaries
never touch `TrackerError` at all — they read the granular `Failure` directly.

`AdfError::code() -> u16` (`adf/mod.rs:38-49`) returns 40/41/42 and is an
independent third thread: unrelated to `classify.rs`, consumed only by
`jira-cli` `for_adf` (`exit_codes.rs:222`).

### Key Discoveries

- **The structured errors already carry the context.** `Failure::Wire` holds
  `{ outcome, operation, detail }` and stores no `u16`; the redesign removes the
  numeric computation, not adds context (`jira-client/src/failure.rs:24-44`,
  `linear-client/src/failure.rs:18-31`).
- **The granular code is a function of `outcome` alone**, in both crates —
  `operation` is not a `bash_code` input (`jira-client/src/classify.rs:44-57`,
  `linear-client/src/classify.rs:122-139`).
- **The linear retry divergence is load-bearing and reproducible on
  `(Outcome, Operation)`.** Code 34 (a `BadRequest`-class body error) is
  retryable on create, terminal on update, because a 200-body error may mean the
  update applied (`linear-client/src/classify.rs:141-148`).
- **The oracle splits cleanly by reachability.**
  `bridge-exit-code-tables.txt` keys on numeric codes including 15/17/22 and
  100–117 that no `Outcome` produces — CLI-emitted argv/credential codes,
  transcriptions of the retired bash mappers. The `tracker-support` self-test
  (`mapper_differential_self_test.rs`) reads the fixture independently and does
  **not** call `classify_bash_code`, so it is untouched.
- **The CLI already forbids detail-string parsing.**
  `exit_codes_never_parses_a_tracker_error_detail`
  (`jira-cli/tests/exit_codes_parity.rs:133`) greps `src/exit_codes.rs` for
  `TrackerError` and `.detail` — the codebase already enforces reading from the
  discriminant.
- **`bash_code` call sites (all move or are re-keyed):** CLI
  `jira-cli/src/exit_codes.rs:226,230`, `linear-cli/src/exit_codes.rs:174`;
  tests `jira-client/tests/classify.rs:99`, `jira-client/tests/discriminant.rs:89,130,146`,
  `linear-client/tests/classify.rs:121`, `linear-client/tests/discriminant.rs:39,78`.
- **`classify_bash_code` call sites (all deleted or re-keyed):** internal
  `linear-client/src/classify.rs:155`; tests `jira-client/tests/classify.rs:203,218`,
  `linear-client/tests/classify.rs:274,288,293,301,305,311,316`.

## Desired End State

Grepping the production `src/` and `Cargo.toml` of both client crates for `bash`
(case-insensitive) returns nothing. No `bash_code`/`classify_bash_code` symbol
remains; no `u16` exit-code field on any error type; no integer literal in the
11–36 range in either `classify.rs` or `failure.rs`; no numeric code embedded in
a `detail` string. Each CLI's exit-code mapping reads the client error type and
produces the pinned values. `AdfError::code()` is gone, its 40/41/42 mapping
inlined into `jira-cli` `for_adf`. Every emitted exit-code value is unchanged —
`mise run cli:check` and the `exit_codes_parity` / `flow_errors` suites pass.

The split is intentionally asymmetric: the granular numeric map leaves the
client crates, but the retry-class `classify` and the `From<Failure> for
TrackerError` collapse **stay** inside them, so each client crate still serves
two consumers (the structured `Failure` for its CLI, and `tracker::TrackerError`
for `work-cli`) and keeps its dependency on the `tracker` crate. Relocating that
collapse is 0271's job; 0269 leaves it in place, re-expressed on
`(Outcome, Operation)`.

Verify with:

```bash
grep -rin bash cli/jira-client/src cli/jira-client/Cargo.toml \
  cli/linear-client/src cli/linear-client/Cargo.toml   # no output
mise run cli:check
```

## What We're NOT Doing

- **Not touching the transport retry predicates.**
  `is_retryable_status` (`jira-client/src/transport.rs:292-294`) and `retryable`
  (`linear-client/src/transport.rs:229-240`) are HTTP-retry domain terms and
  stay unchanged. Only the `bash code 16` doc mention at linear
  `transport.rs:44` is reworded.
- **Not changing any emitted exit-code value.** The parity fixtures and
  `bridge-exit-code-tables.txt` are a frozen contract; this is a
  structural/vocabulary change only.
- **Not touching `tracker::TrackerError` or `work-cli`'s 70/71 mapping.** Owned
  by the `tracker` crate, out of scope.
- **Not extracting a tracker-adapter crate.** The `RemoteTracker` collapse stays
  inside the client crates; relocating it belongs to item 0271.
- **Not editing `bridge-exit-code-tables.txt` content or the `tracker-support`
  self-test.** The client crates stop reading the fixture; the fixture itself
  and its independent reader are unchanged.
- **Not rewording bash mentions unrelated to exit codes.** Genuine shell-script
  shebangs (e.g. the renamed capture script) and ADF-fidelity fixtures may keep
  a `bash` reference where it names the shell or an unrelated behaviour. These
  sit outside every `grep -i bash` AC path, so they need no reword and no
  justifying comment — a comment there would guard nothing and would violate the
  repo's comment convention.

## Implementation Approach

Deliver in four independently mergeable phases, each leaving `mise run cli:check`
and the full test suite green. Phase 1 is the mechanically safe doc reword.
Phases 2 and 3 are the per-crate classification redesigns (jira, then linear),
each self-contained across its client crate and its CLI. Phase 4 is the
independent `AdfError` u16 removal. The `grep -i bash = 0` acceptance criterion
is met only once all four land — inherent to phased delivery — and is checked in
the final validation.

Follow red-green-refactor throughout: adjust the affected test to express the
new shape (a failing or non-compiling test), make it pass with the minimum
change, then refactor. When relocating a mapping and its coverage, write the new
CLI-side `exit_code_for_outcome` and its unit test **first** (green), and delete
the client-crate assertions only once that test passes, so no interim commit
leaves the granular map unasserted.

Every mapping function stays **exhaustive over its enum variants with no
variant-level wildcard**, so a new variant is a compile error at the mapping
site. The one deliberate exception is the jira granular map, whose `Outcome`
wraps `Status(u16)` and so must keep a `Status(_)` catch-all coercing an
unrecognised HTTP status to `SERVER_ERROR`; every other arm is enumerated. Note
that variant-exhaustiveness constrains which inputs are handled, not which exit
codes may be produced — where a codomain property matters (e.g. "linear never
emits a Jira-only code") it is pinned by a test, not left to the match.

## Phase 1: Doc-only vocabulary reword

### Overview

Reword every bash-era doc comment and `Cargo.toml` note in the
non-classification production files of both crates to describe the Rust
behaviour. No symbol, signature, or behaviour changes; the crates compile and
test identically before and after.

The file list below is the current `bash` inventory from the research, verified
against the tree at this revision. The work item's "Production files to touch"
additionally names `mutation.rs` (jira) and `error.rs`, `auth.rs`, `upload.rs`
(linear); a grep confirms none of those carries `bash` at this revision, so they
are deliberately omitted rather than overlooked. The final `grep -i bash = 0`
gate over the whole `src/` (see Desired End State) is the backstop that catches
any file the inventory missed.

### Changes Required

#### 1. jira-client doc comments

**Files**: `cli/jira-client/src/cache.rs` (lines 35, 142, 162, 170),
`cli/jira-client/src/read.rs` (4, 8, 38),
`cli/jira-client/src/custom_fields.rs` (141),
`cli/jira-client/src/principal.rs` (44),
`cli/jira-client/Cargo.toml` (28).

**Changes**: Replace "bash-era" / "the bash flow" / "the bash `<regex>` shape" /
"bash init's holder.pid lock" phrasings with descriptions of the Rust behaviour
or the wire contract. For example, `cache.rs` "Bash-era caches carry" becomes a
description of the unversioned legacy cache layout; `principal.rs` "The bash
`^[A-Za-z0-9:_-]+$` shape" becomes "The `^[A-Za-z0-9:_-]+$` shape — an opaque
Jira accountId". Preserve the invariant each comment documents; only the
provenance vocabulary changes.

#### 2. linear-client doc comments

**Files**: `cli/linear-client/src/client.rs` (lines 64, 95, 116, 238, 367, 427,
568), `cli/linear-client/src/transport.rs` (44),
`cli/linear-client/src/filter.rs` (60), `cli/linear-client/Cargo.toml` (24).

**Changes**: Same treatment. The `transport.rs:44` "which is bash code 16"
mention becomes a plain description of the non-JSON-body case (the predicate
itself is untouched). `client.rs` "the bash `show`/`search` table shows" becomes
a description of the projected shape.

### Success Criteria

#### Automated Verification

- [x] Both crates format clean and lint clean: `mise run cli:check`
- [x] No bash vocabulary in the Phase 1 files:
      `grep -rin bash cli/jira-client/src/cache.rs cli/jira-client/src/read.rs cli/jira-client/src/custom_fields.rs cli/jira-client/src/principal.rs cli/linear-client/src/client.rs cli/linear-client/src/transport.rs cli/linear-client/src/filter.rs cli/jira-client/Cargo.toml cli/linear-client/Cargo.toml`
      returns nothing
- [x] Full client test suites pass unchanged:
      `cargo test -p jira-client -p linear-client` (run from `cli/`)

#### Manual Verification

- [x] Each reworded comment still names the invariant or wire contract it
      documented; none is now a what-comment describing the code

---

## Phase 2: Jira classification redesign and granular map to jira-cli

### Overview

Delete `bash_code`, `classify_bash_code`, and `build` from `jira-client`; keep
`classify` and the `From<JiraFailure>` collapse, re-expressed without the numeric
intermediary and without the `({code})` detail embed. Move the granular map into
`jira-cli` as `exit_code_for_outcome(Outcome) -> u8` over the named constants.
Re-key the client tests off the retired numeric oracle and sweep the jira test
vocabulary. Rename the jira-cli parity fixture, helper, and capture script.

### Changes Required

#### 1. Re-express classify, delete the numeric functions

**File**: `cli/jira-client/src/classify.rs`

`classify` computes `provably_unapplied` from the status directly and builds the
detail without a code. `bash_code`, `classify_bash_code`, and `build` are
removed. The module doc is reworded to describe the two-operation retry policy
without bash provenance.

```rust
#[must_use]
pub fn classify(
    outcome: Outcome,
    operation: Operation,
    detail: &str,
) -> TrackerError {
    let provably_unapplied = match outcome {
        Outcome::Status(400 | 401 | 403 | 404 | 410 | 429) => true,
        Outcome::Status(_) | Outcome::NonJsonBody | Outcome::Transport => false,
    };
    let detail = format!("jira {}: {detail}", operation.name());
    if provably_unapplied || !operation.mutates() {
        TrackerError::Retryable { detail }
    } else {
        TrackerError::Terminal { detail }
    }
}
```

An exhaustive `match` (not `matches!`) is used so a new `Outcome` variant is a
compile error here, matching the current code and the linear rewrite (Phase 3
change 1). `Operation` and `Outcome` (and their derives) are unchanged and stay `pub`;
`Outcome` is consumed by `jira-cli`. No integer literal in the 11–36 range
remains — the only literals are HTTP statuses (400+).

#### 2. Reword the failure module doc

**File**: `cli/jira-client/src/failure.rs`

The module doc and the `Wire` variant doc reference "the granular bash exit
code" and "`bash_code(outcome)` is the integer" (lines 4, 5, 25). Reword them to
state the invariant the failure type itself guarantees — it carries the
`outcome`/`operation` discriminant so no consumer parses a `detail` string —
**without** restating the numeric exit-code mapping, which now lives in
`jira-cli`; a doc narrating a mapping owned by another crate would go stale
silently. The `From<JiraFailure> for TrackerError` impl is unchanged in
behaviour — its `Wire` arm still delegates to `classify`.

#### 3. Move the granular map into jira-cli

**File**: `cli/jira-cli/src/exit_codes.rs`

Replace the `bash_code` import and the two `code_for_outcome`/`code_for_status`
helpers with a local `exit_code_for_outcome` over the named constants, and rename
the status wrapper to `exit_code_for_status` so the whole family reads with one
prefix. `for_failure`'s `Wire` arm calls the outcome helper.

```rust
use jira_client::classify::Outcome;
// ... other imports unchanged (AdfError, CacheError, ClientError, ...)

const fn exit_code_for_outcome(outcome: Outcome) -> u8 {
    match outcome {
        Outcome::Status(400) => REQ_BAD_REQUEST,
        Outcome::Status(401) => UNAUTHORIZED,
        Outcome::Status(403) => FORBIDDEN,
        Outcome::Status(404) => NOT_FOUND,
        Outcome::Status(410) => GONE,
        Outcome::Status(429) => RATELIMITED,
        Outcome::NonJsonBody => REQ_BAD_RESPONSE,
        Outcome::Transport => REQ_CONNECT,
        Outcome::Status(_) => SERVER_ERROR,
    }
}

fn exit_code_for_status(status: u16) -> u8 {
    exit_code_for_outcome(Outcome::Status(status))
}
```

The `for_failure` `Wire` arm changes from `code_for_outcome(*outcome)` to
`exit_code_for_outcome(*outcome)`; the `SurfaceError::Status` arm calls
`exit_code_for_status(status)`.

**On the surviving `u16`:** the name `exit_code_for_status` makes it self-evident
that the input is a wire HTTP status (carried by `SurfaceError::Status`) and the
output is an exit code — the exit-code `u16` intermediary the criterion targets
(`bash_code`/`classify_bash_code`) is gone. With the matched prefix and the
`_for_status` suffix, no explanatory prose is needed to disambiguate the two.

**Reword the CLI-side bash vocabulary in the same file.** Deleting the `bash_code`
symbol leaves stale prose in `jira-cli/src/exit_codes.rs`: the module doc
(lines 3, 5, 16, 26), `for_failure`'s "granular bash code" (line 139), and
`for_credential`'s "mirroring the bash `jira-request.sh`" (line 194). Reword each
to describe the Rust behaviour or the contract without "bash". In particular the
module doc's fixture citation (line 5) names `tests/fixtures/bash-exit-codes.txt`,
which change 7 renames — update it to `captured-exit-codes.txt` so the
document-of-record does not dangle. These lines sit outside the client-crate
`grep -i bash = 0` AC, so they are swept here deliberately rather than left to a
later item.

#### 4. Cover the granular map with a jira-cli unit test

**File**: `cli/jira-cli/src/exit_codes.rs` (inline `#[cfg(test)]` module)

The client-crate `bash_code` assertions move here as a unit test over every
`Outcome` arm. Structure the test so a new `Outcome` variant forces a new
assertion — destructure the value under test through an exhaustive `match` in the
test body rather than trusting the fixed list below — so the completeness the
deleted 43-row guard provided is not silently lost. The module must not mention
`TrackerError` or `.detail` (the parity grep guard scans the whole file).

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use jira_client::classify::Outcome;

    #[test]
    fn every_outcome_maps_to_its_pinned_code() {
        assert_eq!(exit_code_for_outcome(Outcome::Status(400)), REQ_BAD_REQUEST);
        assert_eq!(exit_code_for_outcome(Outcome::Status(401)), UNAUTHORIZED);
        assert_eq!(exit_code_for_outcome(Outcome::Status(403)), FORBIDDEN);
        assert_eq!(exit_code_for_outcome(Outcome::Status(404)), NOT_FOUND);
        assert_eq!(exit_code_for_outcome(Outcome::Status(410)), GONE);
        assert_eq!(exit_code_for_outcome(Outcome::Status(429)), RATELIMITED);
        assert_eq!(exit_code_for_outcome(Outcome::NonJsonBody), REQ_BAD_RESPONSE);
        assert_eq!(exit_code_for_outcome(Outcome::Transport), REQ_CONNECT);
        assert_eq!(exit_code_for_outcome(Outcome::Status(503)), SERVER_ERROR);
    }
}
```

#### 5. Re-key the jira-client classify tests

**File**: `cli/jira-client/tests/classify.rs`

- Drop the `bash_code` and `classify_bash_code` imports.
- Drop the `code` field from `STATUS_TABLE` rows; the outcome→code coverage now
  lives in the jira-cli unit test (change 4). Keep the `create_retryable` /
  `update_retryable` columns and the `classify(...)` retry-class assertions in
  `every_status_row_classifies_per_operation` — this is the live retry oracle.
  Add a completeness guard alongside it: an in-test exhaustive `match` over
  `Outcome` asserting every variant appears in `STATUS_TABLE`, so a new variant
  forces a new row (replacing the coverage the deleted 43-row guard gave).
- Delete `the_status_table_covers_every_condition_the_bash_distinguishes` (it
  iterates over numeric codes), and delete `FixtureRow`, `fixture_rows`,
  `every_jira_row_of_the_committed_fixture_is_asserted` (43-row guard), and
  `a_read_is_retryable_for_every_code_in_the_fixture` — all consumed
  `classify_bash_code` or the bridge fixture.
- **Accepted trade-off — the client crate stops cross-checking the bridge
  fixture.** This is research Option A: the transcription-only fixture rows guard
  no live path, and re-keying the live rows back to the fixture would need a
  numeric outcome↔code helper in the test tree, exactly the numeric-keyed
  vestige 0264 retires. The live verdicts stay triple-pinned — the outcome-keyed
  `STATUS_TABLE`, the jira-cli `exit_codes_parity` suite (constants vs the
  captured fixture), and `flow_errors` end-to-end — so no live coverage is lost.
  The `tracker-support` self-test remains the independent guardian of the bridge
  fixture itself.
- Rename `a_classification_names_the_provider_operation_and_code` to drop the
  code from its name and assertions: it now asserts `detail.contains("jira read")`
  and `detail.contains("ABC-1")` only, and must **not** assert `"(13)"` (the code
  is no longer embedded).

#### 6. Sweep the remaining jira-client test vocabulary

**Files**: `cli/jira-client/tests/discriminant.rs`,
`cli/jira-client/tests/timeouts.rs`, `cli/jira-client/tests/transport.rs`

- `discriminant.rs`: drop the `use ...::bash_code` (line 11), reword the module
  doc (line 2), and replace each `assert_eq!(bash_code(outcome), N, ...)`
  (lines 89, 130, 146) with an assertion on the resulting `Outcome` discriminant
  itself (e.g. `assert_eq!(outcome, Outcome::Status(401))`). The numeric-code
  coverage is the jira-cli unit test (change 4).
- `timeouts.rs`: reword the line-75 message ("bash code 21 covers connect, DNS
  and timeout") to name the transport class without the code.
- `transport.rs`: line 134 ("four attempts, as the bash makes") names retry-count
  parity, not an exit code — out of the exit-code sweep's strict scope. Reword it
  to name the retry count directly and drop "bash"; it is a one-word change, and
  a plain reword is preferred over a justification comment.

#### 7. Rename the jira-cli parity fixture, helper, and capture script

**Files**: `cli/jira-cli/tests/exit_codes_parity.rs`,
`cli/jira-cli/tests/fixtures/bash-exit-codes.txt` →
`cli/jira-cli/tests/fixtures/captured-exit-codes.txt`,
`cli/jira-cli/tests/fixtures/capture-bash-exit-codes.sh` →
`cli/jira-cli/tests/fixtures/capture-exit-codes.sh`

- Rename the fixture file and the `bash_codes()` helper to `captured_codes()`;
  update the `.join(...)` path (line 39) and both call sites (lines 36, 56).
- Reword the parity-suite module doc and the fixture header comment to drop
  "bash" (the fixture's `NAME=INT` rows carry no "bash").
- Rename and reword the capture script; update the header that names the output
  file. The `#!/usr/bin/env bash` shebang is a legitimate interpreter line and
  stays as-is, with no justifying comment: it sits outside every `grep -i bash`
  AC path (the script is in none of the phase grep argument lists), so a comment
  would guard nothing and would be the kind of what-comment the repo's comment
  convention rejects. The script is a manual, uninvoked provenance artefact.

### Success Criteria

#### Automated Verification

- [ ] Format, lint, and type-check clean: `mise run cli:check`
- [ ] Jira classify + discriminant + timeouts + transport suites pass:
      `cargo test -p jira-client` (run from `cli/`)
- [ ] The jira-cli parity and map-unit suites pass:
      `cargo test -p jira-cli --test exit_codes_parity` and
      `cargo test -p jira-cli exit_codes::tests`
- [ ] Every jira-cli test target still compiles (catches a stale consumer of a
      removed symbol, since the crate is not surface-pinned):
      `cargo test -p jira-cli --all-targets --no-run`
- [ ] Behavioural routing unchanged:
      `cargo test -p jira-cli --features test-loopback --test flow_errors`
- [ ] No `bash_code`/`classify_bash_code` symbol in jira-client:
      `grep -rn "bash_code\|classify_bash_code" cli/jira-client/src` returns
      nothing
- [ ] No 11–36 literal in the classification files:
      inspection of `cli/jira-client/src/classify.rs` and `failure.rs`
- [ ] No bash vocabulary in jira-client src, Cargo.toml, the swept jira tests,
      or the jira-cli exit-code source and parity suite:
      `grep -rin bash cli/jira-client/src cli/jira-client/Cargo.toml cli/jira-client/tests/classify.rs cli/jira-client/tests/discriminant.rs cli/jira-client/tests/timeouts.rs cli/jira-client/tests/transport.rs cli/jira-cli/src/exit_codes.rs cli/jira-cli/tests/exit_codes_parity.rs cli/jira-cli/tests/fixtures/captured-exit-codes.txt`
      returns nothing (the renamed capture script is not in this list; its
      shebang is out of scope)

#### Manual Verification

- [ ] `git`/`jj` shows the fixture rename as a rename, not a delete-plus-add,
      and the fixture content (the `NAME=INT` rows) is byte-identical
- [ ] The jira-cli `for_failure`/`for_surface`/`for_client` matches remain
      exhaustive with no wildcard

---

## Phase 3: Linear classification redesign and granular map to linear-cli

### Overview

Mirror Phase 2 for linear, with the added subtlety that linear's `classify`
currently routes through `classify_bash_code`. Re-express it directly on
`(Outcome, Operation)`, reproducing every reachable retry verdict including the
code-34 divergence, and drop the dead `18|23|25|27|29|110..=114` arms.

### Changes Required

#### 1. Re-express classify, delete the numeric functions

**File**: `cli/linear-client/src/classify.rs`

`classify` maps `(Outcome, Operation)` to `provably_unapplied` directly.
`bash_code` and `classify_bash_code` are removed. The divergence doc comment is
kept (reworded to drop "bash") because it records a genuine non-obvious
invariant: a 200-body error may mean the update applied.

```rust
#[must_use]
pub fn classify(
    outcome: Outcome,
    operation: Operation,
    detail: &str,
) -> TrackerError {
    let provably_unapplied = match outcome {
        Outcome::SuccessWithErrors(GraphQlError::Auth)
        | Outcome::Unauthorised
        | Outcome::BadRequest(GraphQlError::Auth)
        | Outcome::SuccessWithErrors(GraphQlError::Complexity)
        | Outcome::BadRequest(GraphQlError::Complexity)
        | Outcome::BadRequest(GraphQlError::RateLimited) => true,
        Outcome::SuccessWithErrors(
            GraphQlError::RateLimited | GraphQlError::BadRequest,
        )
        | Outcome::BadRequest(GraphQlError::BadRequest) => {
            matches!(operation, Operation::Create)
        }
        Outcome::NonJsonBody
        | Outcome::Transport
        | Outcome::ServerError
        | Outcome::Unexpected => false,
    };
    let detail = format!("linear {}: {detail}", operation.name());
    if provably_unapplied || !operation.mutates() {
        TrackerError::Retryable { detail }
    } else {
        TrackerError::Terminal { detail }
    }
}
```

This reproduces the fixture exactly for every reachable outcome: `{11,35,36}`
retryable on both operations, code-34 (`SuccessWithErrors(RateLimited|BadRequest)`
and `BadRequest(BadRequest)`) retryable on create and terminal on update, and
`{16,20,21}` terminal on both. `classify_errors`, `carries_errors`,
`COMPLEXITY_PATTERN`, `GraphQlError`, `Outcome`, and `Operation` are unchanged.

#### 2. Reword the failure module doc

**File**: `cli/linear-client/src/failure.rs`

Same as jira Phase 2 change 2 (lines 4, 5, 20): state the invariant
`LinearFailure` itself guarantees — it carries the `outcome`/`operation`
discriminant so no consumer parses a `detail` string — without restating the
numeric mapping, which now lives in `linear-cli`.

#### 3. Move the granular map into linear-cli

**File**: `cli/linear-cli/src/exit_codes.rs`

Replace the `bash_code` import and `code_for_outcome` with `exit_code_for_outcome`
over the named constants, and rename the status wrapper to `exit_code_for_status`
so the family reads with one prefix. `for_failure`, `for_surface`, and
`exit_code_for_status` call the outcome helper.

```rust
use linear_client::classify::{classify_errors, Outcome};
use linear_client::{ClientError, GraphQlError, LinearFailure, SurfaceError};

const fn exit_code_for_outcome(outcome: Outcome) -> u8 {
    match outcome {
        Outcome::SuccessWithErrors(GraphQlError::Auth)
        | Outcome::Unauthorised
        | Outcome::BadRequest(GraphQlError::Auth) => UNAUTHORIZED,
        Outcome::SuccessWithErrors(GraphQlError::Complexity)
        | Outcome::BadRequest(GraphQlError::Complexity) => COMPLEXITY,
        Outcome::BadRequest(GraphQlError::RateLimited) => RATELIMITED,
        Outcome::SuccessWithErrors(
            GraphQlError::RateLimited | GraphQlError::BadRequest,
        )
        | Outcome::BadRequest(GraphQlError::BadRequest) => BAD_REQUEST,
        Outcome::NonJsonBody => BAD_RESPONSE,
        Outcome::Transport => CONNECT,
        Outcome::ServerError | Outcome::Unexpected => SERVER_ERROR,
    }
}
```

`for_surface`'s `GraphQlErrors` arm and `exit_code_for_status` still build an
`Outcome` (via `classify_errors`) and pass it to `exit_code_for_outcome`, as they
do today with `code_for_outcome`. As in Phase 2 change 3, the renamed
`exit_code_for_status(status: u16, body: &str)` makes plain that its `u16` is a
wire HTTP status, not the removed exit-code intermediary.

**Reword the CLI-side bash vocabulary in the same file.** Deleting `bash_code`
leaves stale prose in `linear-cli/src/exit_codes.rs`: the module doc (lines 5,
16, 25) and the inline note at line 173 (`bash_code never exceeds that band`),
whose helper body is being replaced. Reword each without "bash", and update the
module doc's fixture citation (line 5) from `tests/fixtures/bash-exit-codes.txt`
to `captured-exit-codes.txt` (renamed in change 7) so the document-of-record does
not dangle. These sit outside the client-crate AC and are swept here.

#### 4. Cover the granular map with a linear-cli unit test

**File**: `cli/linear-cli/src/exit_codes.rs` (inline `#[cfg(test)]` module)

Assert every **constructible `Outcome`**, not one representative per code value —
`exit_code_for_outcome` bundles several `GraphQlError` variants per arm via
OR-patterns, so a per-code test would miss a variant drifting between arms (e.g.
`BadRequest(Complexity)` slipping 36→34, or `BadRequest(RateLimited)` 35→34).
Cover each `SuccessWithErrors(g)` and `BadRequest(g)` for every `GraphQlError`
`g`, plus `Unauthorised`, `NonJsonBody`, `Transport`, `ServerError`, and
`Unexpected` — and structure the test through an exhaustive `match` so a new
`GraphQlError` or `Outcome` variant forces a new assertion. Watch the two arms
that split on the same wrapper: `BadRequest(RateLimited) => RATELIMITED` (35)
while `SuccessWithErrors(RateLimited) => BAD_REQUEST` (34). No
`TrackerError`/`.detail` tokens.

Add the reserved-codes structural test in this same module (relocated from
`linear-client` change 5, since `exit_code_for_outcome` lives here): assert the
map over every linear `Outcome` never yields a Jira-only code
`{12,13,14,15,17,19}` — the codomain property match-exhaustiveness cannot
guarantee.

#### 5. Re-key the linear-client classify tests

**File**: `cli/linear-client/tests/classify.rs`

- Drop the `bash_code` and `classify_bash_code` imports.
- Drop the `code` field from `TABLE` rows; keep the `create_retryable` /
  `update_retryable` columns and the `classify(...)` assertions in
  `every_row_classifies_per_operation`. Add the row the re-expressed `classify`
  newly distinguishes: `SuccessWithErrors(RateLimited)` (retryable-create,
  terminal-update, code 34) — the current `TABLE` covers only
  `BadRequest(RateLimited)`, so without this row the rewrite's
  `SuccessWithErrors(RateLimited | BadRequest)` arm is asserted only for
  `BadRequest`. Add a completeness guard: an in-test exhaustive `match` over
  `Outcome` asserting every variant appears in `TABLE`, replacing the deleted
  31-row guard.
- Re-express `both_directions_of_the_divergence_are_reproduced` on outcomes:
  assert `classify(Outcome::BadRequest(GraphQlError::BadRequest), Create)` and
  `classify(Outcome::SuccessWithErrors(GraphQlError::BadRequest), Create)` are
  retryable while their `Update` forms are terminal, and that the auth /
  complexity / rate-limited outcomes are retryable on both. Drop the
  `18|23|25|27|29` assertions — no `Outcome` produces those codes.
- Delete `FixtureRow`, `fixture_rows`, and
  `every_linear_row_of_the_committed_fixture_is_asserted` (31-row guard). As in
  Phase 2 change 5, this is research Option A: the client crate stops
  cross-checking the bridge fixture (the transcription-only rows guard no live
  path, and re-keying them would need a numeric outcome↔code helper 0264
  retires); live verdicts stay triple-pinned by the outcome-keyed `TABLE`, the
  linear-cli `exit_codes_parity` suite, and `flow_errors`.
- Delete `linear_emits_no_403_404_410_or_429_so_those_codes_are_reserved` here
  (it lives in `linear-client` and iterates numeric codes). Its replacement — a
  **structural test, not a comment** — is added in `linear-cli` alongside the
  granular map (change 4), because `exit_code_for_outcome` now lives in
  `linear-cli` and `linear-client` cannot call it without the
  `linear-cli → linear-client` dependency becoming a cargo cycle (see Phase 4
  change 3). Match-exhaustiveness constrains the handled inputs, not the codomain
  — a future arm could map to a literal `13` and stay exhaustive — so the live
  test is required; do not reintroduce numeric keys on the input side.
- Keep the `classify_errors` ordering, complexity-word, and `carries_errors`
  tests unchanged; rename `a_classification_names_the_provider_operation_and_code`
  and drop its `"(21)"` assertion.

#### 6. Sweep the remaining linear-client test vocabulary

**Files**: `cli/linear-client/tests/discriminant.rs`,
`cli/linear-client/tests/transport.rs`

- `discriminant.rs`: drop the `use ...::bash_code` (line 11), reword the module
  doc (line 2), and replace the `bash_code(outcome)` assertions (lines 39, 78)
  with `Outcome` discriminant assertions.
- `transport.rs`: reword the line-335 message ("a 2xx non-JSON body is bash code
  16") to name the non-JSON-body outcome without the code.

#### 7. Rename the linear-cli parity fixture, helper, and capture script

**Files**: `cli/linear-cli/tests/exit_codes_parity.rs`,
`cli/linear-cli/tests/fixtures/bash-exit-codes.txt` → `captured-exit-codes.txt`,
`cli/linear-cli/tests/fixtures/capture-bash-exit-codes.sh` →
`capture-exit-codes.sh`

Same treatment as Phase 2 change 7.

### Success Criteria

#### Automated Verification

- [ ] Format, lint, and type-check clean: `mise run cli:check`
- [ ] Linear classify + discriminant + transport suites pass:
      `cargo test -p linear-client` (run from `cli/`)
- [ ] The linear-cli parity and map-unit suites pass:
      `cargo test -p linear-cli --test exit_codes_parity` and
      `cargo test -p linear-cli exit_codes::tests`
- [ ] Every linear-cli test target still compiles (catches a stale consumer of a
      removed symbol; the crate is not surface-pinned):
      `cargo test -p linear-cli --all-targets --no-run`
- [ ] Behavioural routing unchanged:
      `cargo test -p linear-cli --features test-loopback --test flow_errors`
      (the code-34 create/update divergence is a retry-class 70/71 distinction
      collapsed only in `work-cli`; `linear-cli` emits 34 for both operations, so
      the divergence is pinned by the `classify` `TABLE`, not by `flow_errors`)
- [ ] No `bash_code`/`classify_bash_code` symbol in linear-client:
      `grep -rn "bash_code\|classify_bash_code" cli/linear-client/src` returns
      nothing
- [ ] No 11–36 literal in the classification files: inspection of
      `cli/linear-client/src/classify.rs` and `failure.rs`
- [ ] No bash vocabulary in linear-client src, Cargo.toml, swept tests, or the
      linear-cli exit-code source and parity suite:
      `grep -rin bash cli/linear-client/src cli/linear-client/Cargo.toml cli/linear-client/tests/classify.rs cli/linear-client/tests/discriminant.rs cli/linear-client/tests/transport.rs cli/linear-cli/src/exit_codes.rs cli/linear-cli/tests/exit_codes_parity.rs cli/linear-cli/tests/fixtures/captured-exit-codes.txt`
      returns nothing (the renamed capture script is not in this list; its
      shebang is out of scope)

#### Manual Verification

- [ ] The re-expressed `classify` reproduces the code-34 divergence
      (retryable-create / terminal-update) and the `{11,35,36}`-retryable-on-both
      verdicts, cross-checked against `bridge-exit-code-tables.txt` linear rows
- [ ] The two crate-split views of the taxonomy stay coherent: the set of
      outcomes `linear-cli::exit_code_for_outcome` maps to `BAD_REQUEST` (34) is
      exactly the set whose `linear-client::classify` verdict flips between
      `Create` and `Update`. These now live in different crates as independent
      matches; confirm no future regroup on one side desyncs from the other
- [ ] The fixture rename is a rename with byte-identical content

---

## Phase 4: AdfError u16 removal

### Overview

Remove `AdfError::code() -> u16` (40/41/42) and inline the variant→constant
mapping into `jira-cli` `for_adf`. `AdfError::code()` has four live test call
sites that must be re-homed with it — `adf.rs:84`, `adf_inventory.rs:173`, and
`adf_differential.rs:134,138` (the ADF exit-code differential oracle) — so the
removal does not break compilation or silently drop the 40/41/42 contract guard.
Sweep the `bash_exit=` provenance from the five ADF render-abort fixtures.
Independent of Phases 2–3 and jira-only.

### Changes Required

#### 1. Remove the code method

**File**: `cli/jira-client/src/adf/mod.rs`

Delete the `impl AdfError { pub const fn code(&self) -> u16 { ... } }` block
(lines 36-50). The `Display` impl, the variants, and the two public conversion
functions are unchanged.

#### 2. Inline the mapping at the CLI boundary

**File**: `cli/jira-cli/src/exit_codes.rs`

`for_adf` inspects the variant directly and becomes `const`:

```rust
const fn for_adf(error: &AdfError) -> u8 {
    match error {
        AdfError::RootNotDoc { .. }
        | AdfError::HeadingWithoutLevel
        | AdfError::ListWithoutContent { .. } => BAD_JSON,
        AdfError::UnsupportedBlockquote
        | AdfError::UnsupportedTable
        | AdfError::UnsupportedNestedList => ADF_UNSUPPORTED,
        AdfError::BadInput => ADF_BAD_INPUT,
    }
}
```

`BAD_JSON` (40), `ADF_UNSUPPORTED` (41), `ADF_BAD_INPUT` (42) are the existing
constants. The `u8::try_from(error.code())` fallback disappears; a new variant is
now a compile error at this match.

Add a `#[cfg(test)]` unit test in this file asserting every `AdfError` variant
maps to its pinned constant, through an exhaustive `match` in the test body so a
new variant forces a new assertion. This is the new home for the variant→code
coverage the deleted `AdfError::code()` assertions in `adf.rs` and
`adf_inventory.rs` provided (change 3).

#### 3. Re-home the `AdfError::code()` call sites

**Files**: `cli/jira-client/tests/adf.rs` (84), `cli/jira-client/tests/adf_inventory.rs` (173),
`cli/jira-client/tests/adf_differential.rs` (134, 138).

`jira-client` cannot import `for_adf` from `jira-cli` — `jira-cli` depends on
`jira-client`, so the reverse (even as a dev-dependency) is a cargo cycle. So the
call sites split by concern:

- `adf.rs` and `adf_inventory.rs`: drop the `error.code()` numeric assertions;
  keep the `to_string()` message, `starts_with(name)`, variant, and count
  assertions — the crate's render/refusal behaviour stays a `jira-client`
  concern. The variant→code coverage these lines gave now lives in the `for_adf`
  unit test (change 2).
- `adf_differential.rs`: this oracle compares the crate's refusal against the
  captured oracle's exit status, so it needs a variant→code mapping in place.
  Replace `error.code()` with a **test-local** `expected_adf_exit(&AdfError) -> u16`
  helper (a small exhaustive `match` in the test), preserving the crate-vs-oracle
  exit-code parity. The helper's `40/41/42` literals are ADF codes outside the
  `11–36` band and live only in the test tree, so no AC is affected. (The purer
  alternative — relocate the whole differential harness to `jira-cli` and key it
  on `for_adf` — is heavier because the harness drives the external oracle
  script, and belongs with that oracle's eventual retirement under 0211/0264, not
  this item.) Note the seam: `expected_adf_exit` does not cross-check production
  `for_adf` directly (the cycle forbids sharing the symbol). Instead each side is
  anchored to a frozen artefact — `for_adf` to the captured parity fixture via its
  unit test (change 2) and `exit_codes_parity`, `expected_adf_exit` to the
  captured oracle status — so a mistake on either side surfaces as a failing test
  rather than silent drift.

#### 4. Sweep the render-abort fixtures

**Files**: `cli/jira-client/tests/fixtures/adf/render-abort-root-not-doc/expected-error.txt`
and the four sibling `render-abort-*/expected-error.txt` files.

Remove the `bash_exit=` line (no test parses that fixture line — `adf_differential.rs`
reads only the `class=` prefix and the `E_` message line; the exit-code parity
comes from `expected_adf_exit`, change 3) and reword the "The bash aborts here"
doc prose to describe the Rust render abort.

### Success Criteria

#### Automated Verification

- [ ] Format, lint, and type-check clean: `mise run cli:check`
- [ ] The whole jira-client and jira-cli suites pass (not just the ADF tests —
      this catches the re-homed call sites and any stale consumer, since the
      crates are not surface-pinned):
      `cargo test -p jira-client` and `cargo test -p jira-cli`
- [ ] The `for_adf` unit test passes:
      `cargo test -p jira-cli exit_codes`
- [ ] No `code()` method on `AdfError` and no residual caller:
      `grep -rn "fn code" cli/jira-client/src/adf/mod.rs` returns nothing, and
      `grep -rn "\.code()" cli/jira-client/tests` returns nothing
- [ ] No `bash` in the ADF render-abort fixtures:
      `grep -rin bash cli/jira-client/tests/fixtures/adf/render-abort-*/expected-error.txt`
      returns nothing

#### Manual Verification

- [ ] `for_adf` produces 40/41/42 for the same variants the removed `code()`
      did, confirmed against the ADF exit codes in `jira-cli/src/exit_codes.rs`
- [ ] The 40/41/42 contract stays guarded from two independent directions:
      production `for_adf` by its unit test (change 2) plus `exit_codes_parity`,
      and the crate-vs-oracle render parity by `adf_differential` via the
      test-local `expected_adf_exit` — so neither can drift silently even though
      the two are not cross-checked against each other directly
- [ ] `for_adf` and the test-local `expected_adf_exit` stay coherent through their
      shared anchoring to the frozen ADF exit codes (symmetric to the linear
      taxonomy coherence check in Phase 3)

---

## Testing Strategy

### Unit Tests

- The granular `Outcome → u8` map, per crate, as an inline `#[cfg(test)]` module
  in each CLI's `exit_codes.rs` — every constructible `Outcome` (per-variant, not
  per-code value) asserted against its named constant, through an in-test
  exhaustive `match` so a new variant forces a new assertion. This replaces the
  client crates' `bash_code` assertions.
- The `AdfError → u8` map, via the `for_adf` unit test in `jira-cli`'s
  `exit_codes.rs` — every variant asserted against 40/41/42. This replaces the
  `AdfError::code()` assertions removed from `jira-client`'s ADF tests.
- The retry-class map, per crate, via `classify(outcome, operation)` in the
  client crates' `tests/classify.rs` — the outcome-keyed `STATUS_TABLE`/`TABLE`
  is the live oracle, including the linear code-34 create/update divergence,
  which is a retry-class distinction visible only here (not in `flow_errors`).
- A structural test in `linear-cli` (where `exit_code_for_outcome` lives) that the
  map never yields a Jira-only code `{12,13,14,15,17,19}`, replacing the deleted
  numeric-iterating `linear_emits_no_403_404_410_or_429_...` test that lived in
  `linear-client`.

### Integration Tests

- `exit_codes_parity` (both CLIs): constants pinned against the renamed
  `captured-exit-codes.txt`, proving no emitted value changed.
- `flow_errors` (both CLIs, `--features test-loopback`): the binary is driven
  into each error class and the observed exit code asserted, proving the routing
  is unchanged end-to-end. It does not exercise the code-34 create/update
  divergence — `linear-cli` emits 34 for both operations; that divergence is a
  70/71 distinction owned by `work-cli` and pinned here by the `classify` `TABLE`.
- `adf_differential` (`jira-client`): the crate's ADF exit code (via the
  test-local `expected_adf_exit`) is compared against the captured oracle status,
  preserving the 40/41/42 contract guard after `AdfError::code()` is removed.

### Manual Testing Steps

1. Run the final full grep gate (see Desired End State) and confirm zero
   matches across both client crates' `src/` and `Cargo.toml`.
2. Diff the renamed fixtures to confirm content is byte-identical to the
   originals.
3. Cross-check the linear divergence verdicts against
   `bridge-exit-code-tables.txt` linear rows by hand.

## Migration Notes

None. No persisted data or on-disk format changes; the exit-code values are the
external contract and are preserved exactly.

## References

- Original work item: `meta/work/0269-remove-bash-references-from-jira-linear-clients.md`
- Research: `meta/research/codebase/2026-09-06-0269-remove-bash-vocabulary-and-redesign-exit-code-classification.md`
- Contract oracle: `cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt`
- Granular map today: `cli/jira-client/src/classify.rs:44-103`,
  `cli/linear-client/src/classify.rs:122-184`
- CLI boundary today: `cli/jira-cli/src/exit_codes.rs:134-231`,
  `cli/linear-cli/src/exit_codes.rs:104-188`
- Port impls (collapse stays here): `cli/jira-client/src/client.rs:470-648`,
  `cli/linear-client/src/client.rs:598+`
- AdfError: `cli/jira-client/src/adf/mod.rs:36-50`, consumer
  `cli/jira-cli/src/exit_codes.rs:221-223`
- Blocked items inheriting this shape: 0271 (unify client interfaces), 0273
  (domain crate for Linear/Jira subcommands)
