---
type: "work-item"
id: "0269"
title: "Remove Bash Vocabulary And Redesign Exit-Code Classification In Jira And Linear Clients"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "done"
kind: "task"
priority: "medium"
parent: "work-item:0136"
blocks: ["work-item:0271", "work-item:0273"]
relates_to: ["work-item:0264"]
tags: ["jira", "linear", "cleanup", "refactor"]
last_updated: "2026-09-06T09:57:52+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0136 (Migrate Shell Scripts into a Rust CLI): belongs to the shell-to-Rust migration, its shipped cli/ crates, or the launcher runtime-cache cluster."
schema_version: 1
external_id: "PP-799"
---

# 0269: Remove Bash Vocabulary And Redesign Exit-Code Classification In Jira And Linear Clients

**Kind**: Task
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Remove leftover Bash and shell-exit-code vocabulary from the `jira-client`
and `linear-client` crates, in two layers. First, reword every bash-era doc
comment, comment, and message. Second, redesign the exit-code-shaped
classification path so the client crates emit client-specific error types
carrying enough context for each CLI binary to derive an exit code, and move
the numeric exit-code mapping to the CLI boundary. The exit-code values the
`jira-cli` and `linear-cli` binaries emit are a frozen external contract and
must not change.

## Context

The vocabulary is a vestige of the shell-to-Rust migration (parent epic
0136): the client crates still speak in "the bash flow", "bash-era", and
"bash code N", and model classification as a numeric `u16` exit code rather
than a domain outcome.

The exit-code contract is real and load-bearing. `tracker::TrackerError::`
`{Retryable, Terminal}` map to dispatch codes 70 and 71, and the client
crates re-encode granular HTTP-status codes (11–36) that the `jira-cli` and
`linear-cli` binaries map straight to process exit codes. Those values are
pinned by `cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt`
and the `exit_codes_parity` fixtures. Neither client crate is surface-pinned,
so `public-api:check` will not trip, but the bash-named symbols are consumed by
the `jira-cli` and `linear-cli` crates — their only cross-crate consumers.

## Requirements

- Reword every bash-era doc comment, comment, `Cargo.toml` note, and error
  message in the production `src/` of both crates to describe the Rust
  behaviour, with no residual "bash" vocabulary.
- Move the outcome-to-exit-code mapping to the CLI boundary
  (`cli/jira-cli/src/exit_codes.rs`, `cli/linear-cli/src/exit_codes.rs`).
  The client crates stop computing exit codes.
- Redesign the client crates to return client-specific error types that carry
  enough semantic context for each CLI binary to derive the correct exit code,
  without a numeric `u16` intermediary and without embedding numeric codes in
  `detail` strings. The context is the *outcome* (the domain classification of
  the failure) paired with the *operation* (the client method that was invoked,
  e.g. `create_issue`, `search`). Together the outcome/operation pair must
  distinguish every exit code the parity fixtures enumerate for these crates
  (codes 11–36).
- Remove `bash_code` and `classify_bash_code` from the client crates,
  updating the cross-crate consumers and their `exit_codes_parity` /
  `flow_errors` tests to the new error-carried context.
- Sweep exit-code-related bash vocabulary from the client crates' own tests
  and fixtures (test field names, "bash code N" messages, and provenance
  comments — comments that record a value's bash-era origin) where it concerns
  exit codes.
- Leave `transport.rs` `retryable` / `is_retryable_status` untouched — these
  are HTTP-retry domain terms, not exit-code framing.
- Preserve every emitted exit-code value exactly; the parity fixtures and
  `bridge-exit-code-tables.txt` are the oracle.

## Acceptance Criteria

- [ ] Given the production `src/` and the `Cargo.toml` of `jira-client` and
  `linear-client`, when grepped case-insensitively for `bash`, then there are
  no matches.
- [ ] Given the client crates, when inspected, then no symbol named
  `bash_code` or `classify_bash_code` remains, the error type exposes no `u16`
  exit-code field, neither `classify.rs` nor `failure.rs` contains an integer
  literal in the 11–36 range, and no numeric code is embedded in a `detail`
  string; classification yields a client-specific error whose outcome/operation
  pair distinguishes every exit code the parity fixtures enumerate for these
  crates (codes 11–36).
- [ ] Given `cli/jira-cli/src/exit_codes.rs` and `cli/linear-cli/src/`
  `exit_codes.rs`, when the exit-code mapping is inspected, then it accepts
  only the client error type (no `u16` parameter) and produces the values
  pinned by the parity fixtures.
- [ ] Given `is_retryable_status` / `retryable` in either `transport.rs`,
  when the change is complete, then they are unchanged.
- [ ] Given the exit-code-related tests and fixtures of both crates (the
  `exit_codes_parity` and `flow_errors` suites), when grepped
  case-insensitively for `bash`, then the `bash_*` test field names, "bash code
  N" messages, and exit-code provenance comments are gone, and any remaining
  match sits in a fixture unrelated to exit codes and carries an inline comment
  justifying it.
- [ ] Given `mise run cli:check` and the `exit_codes_parity` / `flow_errors`
  suites, when run, then all pass — proving emitted exit-code values are
  unchanged.

## Open Questions

- None. The single-versus-split question is resolved: this lands as one work
  item. The vocabulary reword and the classification redesign touch the same
  `classify.rs` / `failure.rs` files and the same symbols (`bash_code`,
  `classify_bash_code`), so splitting them would force the reword to either
  skip those symbols or rename them twice, and a standalone reword could not
  satisfy the "no numeric exit code computed" criterion without the redesign.
  Delivery is phased within the single item: land the pure-doc reword of the
  non-classification files first (mechanically safe), then the classification
  redesign and its cross-crate ripple in a second commit, so review can isolate
  the risky layer.

## Dependencies

- Blocked by: none.
- Blocks: 0271 (unify client interfaces), 0273 (domain crate for Linear/Jira
  subcommands). Both refactor the same `classify` / `failure` / `client` files,
  so this item lands first: they inherit the renamed symbols and the
  context-carrying error rather than reshaping a redesign a second time.
- Parent: 0136 (Migrate Shell Scripts into a Rust CLI — in-progress epic).
- Relates to: 0264 (remove Bash-migration negative-assertion tests —
  CLI-wide sibling cleanup).

## Assumptions

- The numeric exit-code values are a frozen external contract; only
  vocabulary and internal structure may change, never the values.
- `transport.rs` `retryable` / `is_retryable_status` are genuine HTTP-retry
  domain terms and stay untouched.

## Technical Notes

Production files to touch:

- `jira-client`: `classify.rs`, `failure.rs`, `adf/mod.rs` (`AdfError::code ->
  u16`, whose numeric return is in scope for the u16 removal, not a doc-only
  edit), plus doc-only edits in `read.rs`, `cache.rs`,
  `custom_fields.rs`, `principal.rs`, `mutation.rs`, `client.rs`,
  `transport.rs`, `Cargo.toml`.
- `linear-client`: `classify.rs`, `failure.rs`, plus doc-only edits in
  `transport.rs`, `client.rs`, `filter.rs`, `error.rs`, `auth.rs`,
  `upload.rs`, `Cargo.toml`.
- CLI boundary (gains the mapping): `cli/jira-cli/src/exit_codes.rs`,
  `cli/linear-cli/src/exit_codes.rs`, plus their `exit_codes_parity` and
  `flow_errors` tests.
- Contract oracle (read-only reference):
  `cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt`.

`tracker::TrackerError::{Retryable, Terminal}` and the 70/71 dispatch codes
are owned by the `tracker` crate and are out of scope.

## Drafting Notes

- Scope confirmed with the author as cosmetic reword plus classification
  redesign, with exit-code mapping moved to the CLI boundary and the client
  crates returning context-carrying custom errors.
- Test and fixture rework is in scope where it concerns exit codes; 0264 was
  confirmed to be CLI-wide and distinct, so it does not absorb this.
- Interpreted "no exit-code values may change" as a hard constraint from the
  parity fixtures; the redesign is a structural/vocabulary change only.
- `transport.rs` retry predicates were separated out as legitimate HTTP
  domain vocabulary, not migration cruft.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
- Related: 0136, 0264, 0271, 0273
- Contract fixture: `cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt`
