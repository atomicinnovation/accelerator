---
type: "codebase-research"
id: "2026-09-06-0269-remove-bash-vocabulary-and-redesign-exit-code-classification"
title: "Research: Remove Bash Vocabulary And Redesign Exit-Code Classification In Jira And Linear Clients (0269)"
date: "2026-09-06T12:16:36+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0269"
parent: "work-item:0269"
topic: "Remove bash vocabulary and redesign exit-code classification in jira-client and linear-client"
tags: ["research", "codebase", "jira-client", "linear-client", "exit-codes", "classification", "migration"]
revision: "f54b4ad9741185e8f39d97a3c1fb508a66a2fdd7"
repository: "accelerator"
last_updated: "2026-09-06T14:53:57+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Added follow-up research on oracle re-keying and the semantics of codes 15/17/100-117"
schema_version: 1
---

# Research: Remove Bash Vocabulary And Redesign Exit-Code Classification In Jira And Linear Clients (0269)

**Date**: 2026-09-06T12:16:36+00:00
**Author**: Toby Clemson
**Git Commit**: f54b4ad9741185e8f39d97a3c1fb508a66a2fdd7
**Branch**: working copy (change `nzpqyrrrmuyo`, no bookmark)
**Repository**: accelerator

## Research Question

For work item 0269: how do `jira-client` and `linear-client` currently encode
exit-code classification, where does the "bash" vocabulary live, what is the
frozen exit-code contract, and what must change to (a) strip bash vocabulary
and (b) move the numeric exit-code mapping to the CLI boundary while returning
context-carrying client-specific errors?

## Summary

The exit-code path is **two distinct layers**, and the work item's framing
partly conflates them. Getting the plan right hinges on separating them:

- 🔵 **Granular layer (codes 11–138).** The `jira-cli` / `linear-cli` binaries
  emit these directly as the process exit code. The client crate's
  `bash_code(outcome) -> u16` computes the granular code; the CLI narrows it
  with `u8::try_from(...)`. This is the layer 0269 redesigns.
- 🟣 **Dispatch layer (codes 70/71).** `TrackerError::{Retryable, Terminal}`
  collapse to 70/71 — but that mapping lives in **`work-cli`**
  (`for_tracker_error`), not in `jira-cli`/`linear-cli`, and the `tracker`
  crate owns `TrackerError`. The work item correctly declares 70/71
  out of scope, but the *collapse function that feeds it* —
  `classify_bash_code` — lives inside the client crates and **is** in scope.

The redesign therefore has to split one overloaded pair of functions. Today
`bash_code` + `classify_bash_code` + `build` do triple duty: compute the
granular code, decide the retry class, and format a detail string embedding the
code. The plan must:

1. Move the granular `outcome -> u16` mapping out of the client crates'
   `classify.rs` into `cli/jira-cli/src/exit_codes.rs` and
   `cli/linear-cli/src/exit_codes.rs` (keyed on the new error type).
2. Re-express the `TrackerError` collapse (Retryable/Terminal) directly from
   the outcome/operation pair, with no numeric `classify_bash_code`
   intermediary and no numeric code embedded in the `detail` string.
3. Rewrite the bridge-fixture oracle test, which is keyed by numeric code
   (including codes 100–117 that no `Outcome` produces) — the single largest
   risk in the item, addressed under Open Questions.

⚠️ **The AdfError u16 removal is a third, independent sub-task.**
`AdfError::code() -> u16` returns 40/41/42, unrelated to `classify.rs`, and
must be removed separately with `for_adf` in `jira-cli` inspecting the variant.

## Detailed Findings

### The granular classification path (jira-client)

`jira-client/src/classify.rs` holds two tables and a builder, all keyed by an
`Operation` (`Create` / `Update` / `Read`) enum with `mutates()` / `name()`
helpers (`classify.rs:10-27`). The wire `Outcome` is `Status(u16)`,
`NonJsonBody`, or `Transport` (`classify.rs:32-40`) — essentially status-only.

- **`bash_code(outcome) -> u16`** (`classify.rs:44-56`) is the granular map.
  Literals in the 11–36 band: `400→34, 401→11, 403→12, 404→13, 410→14,
  429→19, NonJsonBody→16, Transport→21`, catch-all `Status(_)→20`. Operation
  is not an input here — **the outcome alone determines the granular code.**
- **`classify(outcome, operation, detail) -> TrackerError`**
  (`classify.rs:60-70`) computes `provably_unapplied` from the status, then
  calls `bash_code(outcome)` and `build`.
- **`classify_bash_code(code, operation, detail) -> TrackerError`**
  (`classify.rs:75-88`) is the reverse table, keyed on an already-known
  numeric code. `shared_retryable = matches!(code, 11|12|13|14|15|17|19|22|34)`
  — broader than `bash_code`'s outputs (adds 15, 17, 22). Per-op ranges:
  Create `100..=108`, Update `110..=117`, Read always retryable.
- **`build(...)`** (`classify.rs:91-103`) formats `"jira {op} ({code}):
  {detail}"` — ⚠️ this is where the numeric code is embedded in the detail
  string — then returns `Retryable` if `provably_unapplied || !mutates()`,
  else `Terminal`.

`jira-client/src/failure.rs` is the structured error. `JiraFailure`
(`failure.rs:24-44`) has `Wire { outcome, operation, detail }` plus
`UnwritableIdentifier`, `UnsafeQueryId`, `ComposeRejected`, `ReadFailure`.
✅ **It stores no `u16` and no `bash_code` field** — the code is derived on
demand from the `Wire` variant's `outcome`. `From<JiraFailure> for
TrackerError` (`failure.rs:60-88`) routes `Wire` into `classify(...)`; the
other variants map straight to `Terminal`/`Retryable`.

### The granular classification path (linear-client)

Structurally parallel to jira-client, with the differences driven by Linear
reporting failures in the response body (`errors[]` on a 200) and rate-limiting
as HTTP 400.

- **`classify_errors(body) -> GraphQlError`** (`classify.rs:53-92`) is an
  order-dependent cascade (auth → complexity → rate-limited → bad-request);
  the ordering is documented as load-bearing (`classify.rs:37-38`).
- **`Outcome`** (`classify.rs:103-119`) embeds a parsed `GraphQlError`:
  `SuccessWithErrors`, `NonJsonBody`, `Unauthorised`, `BadRequest`,
  `ServerError`, `Transport`, `Unexpected`.
- **`bash_code(outcome) -> u16`** (`classify.rs:122-139`). Literals in 11–36:
  `Auth→11, Complexity→36, RateLimited(400)→35, BadRequest→34,
  NonJsonBody→16, Transport→21, ServerError|Unexpected→20`. Linear emits none
  of 12/13/14/19 (no 403/404/410/429).
- **`classify_bash_code`** (`classify.rs:162-184`) hard-codes two divergent
  per-op sets rather than sharing one — Create `11|22|34|35|36`, Update
  `11|18|22|23|25|27|29|35|36` plus `110..=114`. ⚠️ The doc comment
  (`classify.rs:141-148`) flags that `34` is retryable-on-create /
  terminal-on-update and `18/23/25/27/29` "run the other way with no rationale
  anywhere". The `format!("linear {} ({code}): {detail}", ...)` embed is at
  `classify.rs:178`.

`LinearFailure` (`failure.rs:18-31`) is `Wire { outcome, operation, detail }`
plus `UnwritableIdentifier` — again ✅ no `u16` / `bash_code` field.
`linear-client/src/error.rs` holds `ClientError` (`error.rs:12-41`), the
*pre-classification* taxonomy (transport/TLS/config/token) using symbolic
`E_*` tokens, no numeric codes; it is what `transport.rs` returns.

### The CLI boundary today

Neither CLI is a blind `u16` pass-through. Both mostly **inspect the structured
error** via exhaustive matches, mapping each variant to a named `u8` constant.
The only genuine pass-through is the *wire* path, which calls the client's
`bash_code` and narrows it.

- `cli/jira-cli/src/exit_codes.rs`: `for_failure(&JiraFailure)`
  (`exit_codes.rs:142`) sends `Wire` to `code_for_outcome` (`:225`, which does
  `u8::try_from(bash_code(outcome))`), and maps the other variants to named
  constants directly. `for_surface`, `for_client`, `for_cache`, `for_adf`
  similarly inspect variants. `bash_code` is imported at `:134`, called at
  `:226` and `:230`.
- `cli/linear-cli/src/exit_codes.rs`: mirror shape — `for_failure`
  (`exit_codes.rs:119`), `code_for_outcome` (`:172`), `bash_code` imported at
  `:104`, called at `:174`; `for_surface` re-derives an `Outcome` via
  `classify_errors` (`:136`, `:181`).

✅ **`classify_bash_code` is NOT consumed by either CLI binary.** Its only
callers are `linear_client::classify::classify` (`classify.rs:155`) and the
client crates' own `tests/classify.rs` bridge-oracle tests. This matters: the
work item lists "cross-crate consumers" of `classify_bash_code`, but the real
consumers are internal (the `From<Failure> for TrackerError` path) and the
oracle tests — not `jira-cli`/`linear-cli`.

### The frozen contract oracle

`cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt` records, per
`(code, provider, operation)` triple, whether the failure is `retryable`
(dispatch 70) or `terminal` (dispatch 71). It is grouped by provider then
operation (Jira create/update, Linear create/update). The granular
*condition* meanings are NOT in the fixture — only the class; the condition
map lives in the `bash_code` functions above.

The fixture drives three suites plus a self-test:

- `cli/jira-client/tests/classify.rs:190-223` — asserts a **hard-coded 43-row
  count** and that `is_retryable(classify_bash_code(code, op, "d"))` matches
  each row.
- `cli/linear-client/tests/classify.rs:261-322` — **hard-coded 31-row count**,
  plus explicit divergence assertions.
- `cli/tracker-support/tests/mapper_differential_self_test.rs` +
  `tests/support/mod.rs` — proves the comparison machinery can actually fail
  (`RETRYABLE_STATUS=70`, `TERMINAL_STATUS=71`).

⚠️ The fixture is keyed by numeric code and includes codes **100–117** (pre/
post-send) that **no `Outcome` maps to** — they are inputs the binary generates
elsewhere. Removing `classify_bash_code` (which currently accepts those codes)
forces a rethink of how these oracle tests express their assertions without a
numeric key.

### The dispatch layer (out of scope, but adjacent)

`cli/tracker/src/lib.rs:144-179` defines `TrackerError` as a closed
two-variant enum (`Retryable { detail }`, `Terminal { detail }`).
`cli/work-cli/src/exit_codes.rs:60-72` maps them to 70/71 via
`for_tracker_error`. The oracle for that collapse is
`cli/tracker/tests/errors.rs`. The client crates reach `TrackerError` through
`From<Failure> for TrackerError` → `classify` → `classify_bash_code` →
`build`. Severing the numeric intermediary means this `From` impl must build
`Retryable`/`Terminal` straight from outcome + operation.

### The `transport.rs` retry predicates (must stay untouched)

Confirmed genuine HTTP-retry domain terms, separate from exit-code
classification:

- jira: `is_retryable_status(status) -> bool` = `429 || 500..600`
  (`transport.rs:292-294`), called only in the retry loop (`:207`). ✅ No
  "bash" vocabulary in the file.
- linear: `retryable(status, body) -> bool` (`transport.rs:229-240`) = 5xx, or
  400 whose body classifies `RATELIMITED`. ⚠️ One "bash" mention at
  `transport.rs:44` (doc: "which is bash code 16") — that comment is in scope
  for the reword even though the predicate itself is not.

### Bash vocabulary inventory

Production `src/` and `Cargo.toml` (the AC-critical set — must reach zero):

| Crate | File | Lines with "bash" |
|-------|------|-------------------|
| jira-client | `classify.rs` | 45, 70, 76 (symbol names) |
| jira-client | `failure.rs` | 4, 5, 25 (doc) |
| jira-client | `read.rs` | 4, 8, 38 (doc) |
| jira-client | `cache.rs` | 35, 142, 162, 170 |
| jira-client | `custom_fields.rs` | 141 |
| jira-client | `principal.rs` | 44 |
| jira-client | `Cargo.toml` | 28 |
| linear-client | `classify.rs` | 102, 121, 123, 155, 163 |
| linear-client | `failure.rs` | 4, 5, 20 |
| linear-client | `transport.rs` | 44 |
| linear-client | `client.rs` | 64, 95, 116, 238, 367, 427, 568 |
| linear-client | `filter.rs` | 60 |
| linear-client | `Cargo.toml` | 24 |

⚠️ The item's "Production files to touch" list omits several confirmed hits:
jira-client `cache.rs` (4 lines) and `custom_fields.rs`/`principal.rs`;
linear-client `client.rs` (7 lines) and `filter.rs`. Any of these left
unedited fails the `grep -i bash` acceptance criterion.

Exit-code-related tests and fixtures carrying "bash" (in scope per the item):

- `cli/jira-client/tests/classify.rs:8,99,101,132,137,203,205,218` and
  `tests/discriminant.rs:2,11,89,130,146`, `tests/timeouts.rs:75` ("bash code
  21"), `tests/transport.rs:134`.
- `cli/linear-client/tests/classify.rs:9,121,274,276,288,293,301,305,311,316`
  and `tests/discriminant.rs:2,11,39,78`, `tests/transport.rs:335`.
- CLI parity: `cli/jira-cli/tests/exit_codes_parity.rs` and
  `cli/linear-cli/tests/exit_codes_parity.rs` use `bash_codes()` helper (`:36`)
  and read `tests/fixtures/bash-exit-codes.txt`.
- jira-only fixtures: `cli/jira-cli/tests/fixtures/bash-exit-codes.txt`,
  `capture-bash-exit-codes.sh`, and `jira-client` `render-abort-*/
  expected-error.txt` files (each has `bash_exit=`).

⚠️ Some "bash" hits are legitimate and out of scope: genuine shell scripts
(`capture-adf-oracle.sh`), and ADF-fidelity fixtures unrelated to exit codes.
The AC permits these if they carry an inline justification comment.

## Code References

- `cli/jira-client/src/classify.rs:44-103` - `bash_code`, `classify`,
  `classify_bash_code`, `build`
- `cli/jira-client/src/failure.rs:24-88` - `JiraFailure` + `From` for
  `TrackerError`
- `cli/jira-client/src/adf/mod.rs:38-49` - `AdfError::code() -> u16` (40/41/42)
- `cli/jira-client/src/transport.rs:292-294` - `is_retryable_status` (untouched)
- `cli/linear-client/src/classify.rs:122-184` - `bash_code`,
  `classify_bash_code` with documented create/update divergence
- `cli/linear-client/src/failure.rs:18-66` - `LinearFailure` + `From`
- `cli/linear-client/src/error.rs:12-41` - `ClientError` (pre-classification)
- `cli/linear-client/src/transport.rs:229-240` - `retryable` (untouched)
- `cli/jira-cli/src/exit_codes.rs:142-230` - variant-inspecting mappers +
  `code_for_outcome`/`code_for_status`
- `cli/linear-cli/src/exit_codes.rs:110-186` - mirror mappers
- `cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt` - the oracle
- `cli/jira-client/tests/classify.rs:190-223` - 43-row oracle guard
- `cli/linear-client/tests/classify.rs:261-322` - 31-row oracle guard
- `cli/tracker/src/lib.rs:144-179` - `TrackerError` (out of scope)
- `cli/work-cli/src/exit_codes.rs:60-72` - `for_tracker_error` → 70/71 (out of
  scope)
- `cli/jira-cli/tests/exit_codes_parity.rs` / `cli/linear-cli/tests/exit_codes_parity.rs`
  - constant-vs-fixture parity, with `exit_codes_never_parses_a_tracker_error_detail`
  grep guard (jira `:133`)
- `cli/jira-cli/tests/flow_errors.rs` / `cli/linear-cli/tests/flow_errors.rs`
  - behavioural exit-code routing (feature `test-loopback`)

## Architecture Insights

- **The structured error already carries the context the item wants.** Both
  `Wire { outcome, operation, detail }` variants hold outcome + operation and
  store no numeric code. The redesign is less "add context" and more "stop
  computing the number inside the crate" — move the `outcome -> u16` table to
  the CLI, and stop embedding `({code})` in `detail`.
- **`bash_code` and `classify_bash_code` are two different functions, differently
  coupled.** `bash_code` (granular map) is consumed by the CLI and moves to the
  CLI. `classify_bash_code` (retry-class map) is internal + oracle-only and its
  logic must be re-expressed on outcome/operation. Conflating them will produce
  a wrong plan.
- **Outcome alone determines the granular code** in both crates (operation is
  not a `bash_code` input). The AC's phrase "outcome/operation pair must
  distinguish every exit code 11–36" is satisfied by outcome alone for the
  granular band; operation only distinguishes the *retry class* and the
  100–117 pre/post-send band.
- **Two oracles, two fixtures, do not confuse them.** `exit_codes_parity`
  reads `bash-exit-codes.txt` (constant-name → value, per CLI). The
  `classify.rs` tests read `bridge-exit-code-tables.txt` (code → retry class).
  Only the latter feeds `classify_bash_code`.
- **The parity suite already forbids detail-string parsing.**
  `exit_codes_never_parses_a_tracker_error_detail` (jira
  `exit_codes_parity.rs:133`) greps `src/exit_codes.rs` for `TrackerError` /
  `.detail` — the codebase already enforces "read from the discriminant, not
  the string", which is the direction 0269 pushes further.

## Historical Context

- `meta/work/0136-migrate-shell-scripts-to-rust-cli.md` - parent epic; the
  bash vocabulary is its vestige.
- `meta/work/0264-remove-bash-migration-negative-assertion-tests.md` -
  CLI-wide sibling cleanup, confirmed distinct (does not absorb 0269).
- `meta/work/0271-unify-surface-remotetracker-client-interfaces.md`,
  `meta/work/0273-domain-crate-for-linear-jira-subcommands.md` - both blocked
  by 0269; they reshape the same `classify`/`failure`/`client` files.
- `meta/notes/2026-06-23-further-ideas-backlog.md` - source note for the
  026x/027x cluster.
- `meta/plans/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`
  and `meta/research/codebase/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`
  - where `jira-client`/`linear-client` originate.
- `meta/plans/2026-08-21-0190-classify-lock-mkdir-failures.md` and its research
  - closest prior art for exit-condition classification design.
- `meta/plans/2026-08-19-0211-integration-binaries-and-bash-cluster-retirement.md`
  - the bash-cluster retirement precedent.
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
  and `ADR-0058-shell-free-cli-to-node-delegation.md` - the architecture the
  clients sit in; nearest anchors (no ADR is dedicated to exit codes).

## Related Research

- `meta/research/codebase/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`
- `meta/research/codebase/2026-08-21-0190-acquire-lock-mkdir-classification.md`
- `meta/research/codebase/2026-08-17-0211-integration-binaries-and-bash-cluster-retirement.md`
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`

## Open Questions

- ❓ **How should the bridge-fixture oracle be re-keyed?** It currently keys on
  numeric codes (including 100–117 that no `Outcome` produces) and calls
  `classify_bash_code`. If that function is removed, either the oracle test is
  rewritten to feed outcome/operation pairs (losing coverage of the 100–117
  band, which has no outcome) or a translation is kept for tests only. This is
  the largest design decision the plan must resolve. Whichever way, the 43-row
  and 31-row count guards must be reconciled with the new keying.
- ❓ **Where do codes 15, 17 (jira) and the 100–117 pre/post-send bands come
  from after the redesign?** No `Outcome` maps to 15/17; they live only in the
  jira `shared_retryable` set and the fixture. Their original semantics sit in
  retired bash sources / an `EXIT_CODES.md` not found under
  `workspaces/build-system`. The plan needs a source of truth for these before
  it can move their retry-class logic.
- ❓ **Does the `From<Failure> for TrackerError` collapse stay in the client
  crate or move?** The work item says client crates "return client-specific
  error types"; the CLI then derives exit codes. But `work-cli` still needs
  `TrackerError` for 70/71. Cleanest: keep `From<Failure> for TrackerError` but
  re-express it on outcome/operation, with the granular `u16` mapping gone from
  the crate. Confirm this is the intended shape.
- ❓ **Naming for the replacement of `bash_code`.** The granular map moves to
  the CLI; what domain name replaces it (e.g. `exit_code_for_outcome`)? The
  item mandates no residual "bash" but does not name the successor.

## Follow-up Research 2026-09-06T14:53:57+00:00

Two questions from the first pass, now resolved with evidence: how the
bridge-fixture oracle should be re-keyed once `classify_bash_code` is deleted,
and what codes 15/17/22/100–117 mean and where they live.

### The oracle is two oracles wearing one function

`classify_bash_code` is asked to do two unrelated jobs, and the fixture rows
split cleanly along that seam:

- 🟢 **Live retry-class of a wire outcome.** For the codes a real `Outcome`
  produces, the fixture pins the Retryable/Terminal verdict of the live path.
- ⚪ **Transcription of the retired bash mappers.** For codes no `Outcome` can
  produce, the fixture pins how the *deleted* bash `_wicr_map_*` / `_wiur_map_*`
  functions classified them — pure historical fidelity, no live code path.

The reachability boundary is exact. `bash_code` emits only
`{11,12,13,14,16,19,20,21,34}` (jira, `jira-client/src/classify.rs:45-57`) and
`{11,16,20,21,34,35,36}` (linear, `linear-client/src/classify.rs:123-139`).
Every other fixture code reaches `classify_bash_code` **only from a test**:

| Code(s) | Reachable from an `Outcome`? | What it actually is |
|---------|------------------------------|---------------------|
| 11,12,13,14,16,19,20,21,34 (jira) | Yes — live wire path | Status/transport codes |
| 11,16,20,21,34,35,36 (linear) | Yes — live wire path | Body-classified codes |
| 15,17,22 (jira) | No | `BAD_SITE`, `REQ_BAD_PATH`, `REQ_NO_CREDS` — CLI client/cred errors |
| 100–108,109,110–117 (jira) | No | `CREATE_*`/`RESOLVE_*`/`UPDATE_*` argv validation |
| 18,22,23,25,27,29 (linear) | No | test-hook / cred codes, some with no linear taxonomy entry |
| 110–114 (linear) | No | `UPDATE_*` argv validation |

⚠️ The runtime routing differs per crate, which sharpens the point:

- jira's `classify` calls `build` directly (`classify.rs:70`), never
  `classify_bash_code`. So jira's `classify_bash_code` is **entirely
  test-only** — its sole callers are `jira-client/tests/classify.rs:203,218`.
- linear's `classify` calls `classify_bash_code(bash_code(outcome), …)`
  (`classify.rs:155`), so it is reached at runtime — but only ever with the
  seven codes `bash_code` emits. The `18|23|25|27|29` and `110..=114` arms
  (`classify.rs:171-175`) are dead at runtime.

✅ Confirmed by grep: the only non-test caller of `classify_bash_code` anywhere
is `linear-client/src/classify.rs:155`. The only callers of `bash_code` are the
two `classify` functions and the four CLI `code_for_outcome`/`code_for_status`
helpers — all fed exclusively from an `Outcome`.

### Where the non-wire codes actually come from

The non-wire codes are emitted **directly by the CLI**, never through
classification:

- 15/17/22 (jira) come from `for_client` / `for_credential` inspecting
  `ClientError` / `CredentialError` variants (`jira-cli/src/exit_codes.rs:173-207`).
- 100–117 come from argv validation in `main.rs` as `ExitCode::from(...)` —
  e.g. `CREATE_NO_SUMMARY` (102) at `jira-cli/src/main.rs:222-223`, `UPDATE_NO_OPS`
  (112) at `:357-358`, `RESOLVE_ALREADY_SYNCED` (109) at `:965-968`.

The naming source of record is the two `exit_codes.rs` module docs plus
`cli/{jira,linear}-cli/tests/fixtures/bash-exit-codes.txt`, both citing the
retired cluster's `EXIT_CODES.md` (`jira-cli/src/exit_codes.rs:1-9`,
`linear-cli/src/exit_codes.rs:1-9`). `EXIT_CODES.md` itself is gone with the
bash cluster; nothing under `workspaces/build-system` still holds it.

⚠️ Linear codes **18 and 23 have no entry in `linear-cli/src/exit_codes.rs`**
at all (linear defines 11,16,20,21,22,24,25,27,29,34,35,36,53). In jira they
are `TEST_OVERRIDE_REJECTED` (18) and `TEST_HOOK_REJECTED` (23) — bash-cluster-
wide test-hook codes. Their presence in the linear fixture is a transcription
of the bash mapper's case arm, not a live linear condition. These are exactly
the kind of test-hook vestige work item 0264 targets.

### Blast radius: three independent fixture parsers, only two affected

The fixture `bridge-exit-code-tables.txt` is parsed by three independent
readers:

- `jira-client/tests/classify.rs:160-223` — its own `fixture_rows()` →
  `classify_bash_code`. **Affected.**
- `linear-client/tests/classify.rs:231-322` — its own `fixture_rows()` →
  `classify_bash_code`. **Affected.**
- `cli/tracker-support/tests/support/mod.rs` + `mapper_differential_self_test.rs`
  — re-implements the classification rule ("listed code → its class, unlisted →
  terminal") independently; does **not** call `classify_bash_code`. ✅
  Unaffected by the redesign.

So deleting `classify_bash_code` breaks exactly the two client-crate
`classify.rs` suites. The tracker-support self-test that proves the comparison
machinery can fail is self-contained and survives.

Note also the jira suite already carries the live-path oracle keyed on
`Outcome`: `STATUS_TABLE` (`jira-client/tests/classify.rs:25-129`) asserts
`bash_code(outcome) == code` and the retry class via `classify(outcome,
operation)` for every outcome-reachable code. **Linear has no equivalent
outcome-keyed table** — its only outcome→class assertions today run through
`classify_bash_code(bash_code(outcome))`.

### Re-keying options (recommendation first)

🔵 **Option A — split by reachability (recommended).** Re-key the live rows to
`(Outcome, Operation)` and assert them through the retained `From<Failure> for
TrackerError` collapse plus the new CLI mapping; build linear an outcome-keyed
table mirroring jira's `STATUS_TABLE`. Drop the transcription-only rows
(15/17/22/100–117 jira; 18/22/23/25/27/29/110–114 linear) and the fixture rows
that back them, since the bash mappers they transcribe are being retired
(0136/0211) and the test-hook codes fall to 0264. The 43-row and 31-row count
guards drop to the outcome-reachable counts. Cleanest and aligned with the
epic; loses the retired-mapper fidelity guard, which no longer guards live code.

🟣 **Option B — keep the fixture as documentation.** Retain
`bridge-exit-code-tables.txt` as a record of the retired mappers but stop
asserting against it from the client crates once `classify_bash_code` is gone;
move all live retry-class assertions to outcome-keyed tables. Preserves the
historical artefact without a dead public function.

🟠 **Option C — test-only numeric shim.** Move the numeric-code classification
into a private helper inside each test module so the transcription oracle
survives without the crate's public API carrying bash vocabulary. Preserves
fidelity but keeps a numeric-keyed classifier alive in the test tree, arguably
against the spirit of 0264.

The decision is the author's, but the reachability analysis removes the risk
the first pass flagged: the transcription-only rows guard no live behaviour, so
dropping them (Option A) costs no runtime coverage. The live retry-class
verdicts remain fully pinned by outcome-keyed tables and the CLI parity suite.

### Answers to the two prior open questions

- **Oracle re-keying:** re-key live rows on `(Outcome, Operation)`; the
  transcription-only rows have no `Outcome` and no live path, so they are
  dropped (or demoted to documentation), not translated. Only two test suites
  are affected; the tracker-support self-test is independent.
- **Codes 15/17/22/100–117:** all are CLI-emitted (client/credential errors and
  argv validation), fully named in the two `exit_codes.rs` documents of record
  and `bash-exit-codes.txt`; none is produced by any `Outcome`, so none needs a
  home in the redesigned `classify.rs`. Linear 18/23 have no linear taxonomy
  entry and are bash test-hook vestiges (0264 territory).
