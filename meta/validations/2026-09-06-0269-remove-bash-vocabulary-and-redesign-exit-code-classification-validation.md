---
type: "plan-validation"
id: "2026-09-06-0269-remove-bash-vocabulary-and-redesign-exit-code-classification-validation"
title: "Validation Report: Remove Bash Vocabulary And Redesign Exit-Code Classification In Jira And Linear Clients"
date: "2026-09-06T22:35:44+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-06-0269-remove-bash-vocabulary-and-redesign-exit-code-classification"
tags: ["jira", "linear", "cleanup", "refactor", "exit-codes", "classification"]
last_updated: "2026-09-06T22:35:44+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Remove Bash Vocabulary And Redesign Exit-Code Classification

All four phases are implemented, committed, and verified; the plan passes. Every
grep gate, symbol gate, numeric-literal gate, and plan-mandated test suite is
green, and `mise run cli:check` exits 0. One low-severity test-structure
deviation and one environmental test-suite caveat are recorded below; neither
affects the emitted-exit-code contract, which is preserved exactly.

### Implementation Status

Each phase landed as its own commit, in order.

| Phase | Scope | Commit | Status |
|-------|-------|--------|--------|
| 1 | Doc-only bash reword, both client crates | `xnuqzlwz` | 🟢 fully implemented |
| 2 | Jira classify redesign, granular map to `jira-cli` | `vyxmyzzz` | 🟢 fully implemented |
| 3 | Linear classify redesign, granular map to `linear-cli` | `moywxswn` | 🟢 fully implemented |
| 4 | `AdfError::code()` removal, inline into `for_adf` | `kvkskpkz` | 🟢 fully implemented |

### Automated Verification Results

- ✅ CI read-only gate: `mise run cli:check` exits 0 (format + lint + types, all four components).
- ✅ Desired-End-State grep gate: `grep -rin bash` over both client crates' `src/` and `Cargo.toml` returns nothing.
- ✅ Symbol gates: no `bash_code`/`classify_bash_code` in either `src/`; no `fn code` in `adf/mod.rs`; no `.code()` caller in `jira-client/tests`.
- ✅ Numeric-literal gate: no `11`–`36` literal in either crate's `classify.rs` or `failure.rs`.
- ✅ Phase-2 and Phase-3 grep gates (client `src`, `Cargo.toml`, swept tests, CLI exit-code source, parity fixture) return nothing.
- ✅ ADF fixture gate: no `bash` in the five `render-abort-*/expected-error.txt` files.
- ✅ Plan-mandated suites pass: `classify`, `discriminant`, `timeouts`, `transport` (both client crates); `exit_codes_parity` and `exit_codes::tests` (both CLIs); `flow_errors` (jira 2, linear 4) under `--features test-loopback`.
- ⚠️ Full `cargo test -p jira-client` reports 2 failures — both live-tenant `contract.rs` tests, not a regression (see Potential Issues).

Suite-by-suite counts (targeted run):

```text
jira-client   classify 3   discriminant 5   timeouts 6   transport 10
linear-client classify 10  discriminant 3   transport 5
jira-cli      exit_codes_parity 5   flow_errors 2
linear-cli    exit_codes_parity 3   flow_errors 4
unit modules  exit_codes::tests (jira, linear) all ok
```

### Code Review Findings

#### Matches Plan

- `jira-client/src/classify.rs` keeps `classify` on an exhaustive `match` over `Outcome`, computes `provably_unapplied` from the status directly, and builds the detail as `"jira {op}: {detail}"` — no `({code})` embed, no `bash_code`/`classify_bash_code`/`build`.
- `linear-client/src/classify.rs` re-expresses `classify` directly on `(Outcome, Operation)`, reproducing the code-34 create/update divergence (`SuccessWithErrors(RateLimited | BadRequest)` and `BadRequest(BadRequest)` retryable on create, terminal on update) and dropping the dead `18|23|25|27|29|110..=114` arms. Exhaustive, no wildcard.
- `jira-cli/src/exit_codes.rs` carries `exit_code_for_outcome` over the named constants with the deliberate `Status(_) => SERVER_ERROR` catch-all; `exit_code_for_status` wraps it; `for_failure`/`for_surface`/`for_client` remain exhaustive with no wildcard.
- `linear-cli/src/exit_codes.rs` carries `exit_code_for_outcome` with the exact OR-pattern grouping the plan specified, exhaustive with no wildcard, plus the reserved-codes structural test asserting the map never yields a Jira-only code `{12,13,14,15,17,19}`.
- `AdfError::code()` is removed; `for_adf` is now an inline `const fn` mapping variants to `BAD_JSON`/`ADF_UNSUPPORTED`/`ADF_BAD_INPUT` (40/41/42), with a `for_adf` unit test and the test-local `expected_adf_exit` helper re-homed in `adf_differential.rs`.
- Client-crate `classify` tests carry exhaustive-`match` completeness guards over `Outcome` (`the_status_table_covers_every_outcome_variant`, `the_table_covers_every_outcome_variant`), so a new variant forces a new `STATUS_TABLE`/`TABLE` row.
- The parity fixtures are renamed `bash-exit-codes.txt → captured-exit-codes.txt` as `jj` renames (`R`), with every `NAME=INT` value row byte-identical — only `#` header comments reworded.

#### Deviations from Plan

- The CLI outcome→code unit tests are fixed `assert_eq!` lists, not the in-test exhaustive `match` the plan specified (Phase 2 change 4, Phase 3 change 4). `linear-cli::every_constructible_outcome_maps_to_its_pinned_code` and the two `jira-cli` unit tests (`every_outcome_maps_to_its_pinned_code`, `every_adf_error_maps_to_its_pinned_code`) would not force a new assertion if an `Outcome`/`AdfError` variant were added. Low severity — the compile-time safety net still holds from two other directions: the production mapping matches are exhaustive with no wildcard (a new variant is a build error at the mapping site), and the client-crate `classify` table guards do use exhaustive matches.

#### Potential Issues

- ⚠️ `cargo test -p jira-client` is red locally: `contract.rs`'s `the_conformance_set_passes_against_a_live_tenant` and `a_failing_read_is_retryable_against_a_live_tenant` panic at `contract.rs:108` demanding `ACCELERATOR_JIRA_SITE`/`EMAIL`/`TOKEN`. These are live-tenant tests that deliberately fail-not-skip without credentials (documented, `contract.rs:10-17`); `contract.rs` sits in no phase's file list and no success criterion. This is an environmental precondition, not a defect introduced by the plan. Phase 4's literal AC (`cargo test -p jira-client ... pass`) is therefore only green in an environment that supplies tracker credentials — as CI does — while the canonical read-only gate `mise run cli:check` is green here.

### Manual Testing Required

None outstanding. The plan's manual criteria were verified during validation:

- [x] Fixture renames are `jj` renames with byte-identical `NAME=INT` rows.
- [x] The re-expressed linear `classify` reproduces the code-34 divergence and the `{11,35,36}`-retryable-on-both verdicts, cross-checked against the fixture rows.
- [x] The CLI mapping matches remain exhaustive with no wildcard beyond the deliberate jira `Status(_)`.
- [x] Reworded comments name the invariant or wire contract they documented; none is a what-comment.

### Recommendations

- Optionally tighten the three CLI unit tests to destructure the value under test through an exhaustive `match`, closing the one deviation so the unit layer also fails-to-compile on a new variant. Not blocking — the guarantee already holds via the production matches and the client-crate table guards.
- No action needed on the `contract.rs` failures; run them with tracker credentials (or the `ACCELERATOR_TRACKER_CONTRACT` gate) where a live check is wanted.
