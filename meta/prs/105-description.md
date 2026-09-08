---
type: "pr-description"
id: "105"
title: "[0269] Remove Bash Vocabulary And Redesign Exit-Code Classification In Jira And Linear Clients"
date: "2026-09-06T23:16:27+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0269"
parent: "work-item:0269"
relates_to: ["work-item:0264", "work-item:0271", "work-item:0273"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/105"
pr_number: 105
tags: ["jira", "linear", "cleanup", "refactor", "exit-codes", "classification"]
revision: "e8346fad6db334d44a2a8d64fc85734008ec175a"
repository: "accelerator"
last_updated: "2026-09-06T23:16:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0269] Remove Bash Vocabulary And Redesign Exit-Code Classification In Jira And Linear Clients

## Summary

Strips every residual "bash" reference from the `jira-client` and
`linear-client` crates and moves the granular exit-code map from the client
crates to the CLI boundary. The client crates keep returning context-carrying
failures (`JiraFailure`/`LinearFailure`, each holding `{ outcome, operation,
detail }`); the numeric `outcome → u8` map now lives in `jira-cli`/`linear-cli`;
and the retry-class collapse to `TrackerError` stays in the client crates,
re-expressed directly on `(Outcome, Operation)` with no `u16` intermediary and
no numeric code embedded in a `detail` string. Every emitted exit-code value is
preserved exactly — the parity fixtures are the frozen oracle.

## Changes

- **Redesigned `classify` in both client crates.** `bash_code`,
  `classify_bash_code`, and `build` are deleted; `classify` computes the
  retry class directly from `(Outcome, Operation)` over an exhaustive `match`,
  and the `detail` no longer embeds a numeric code. Linear's re-expression
  reproduces the code-34 create/update divergence and drops the dead
  `18|23|25|27|29|110..=114` arms.
- **Moved the granular map to the CLI boundary.** `jira-cli` and `linear-cli`
  each gain `exit_code_for_outcome(Outcome) -> u8` over their named constants,
  with `exit_code_for_status` wrapping it; the mapping matches stay exhaustive
  with no wildcard (bar the deliberate jira `Status(_) => SERVER_ERROR`).
- **Removed `AdfError::code() -> u16`.** The 40/41/42 variant→constant mapping
  is inlined into `jira-cli`'s `for_adf`, now a `const fn`; test call sites are
  re-homed, including a test-local `expected_adf_exit` in `adf_differential`.
- **Reworded bash-era doc vocabulary** across the non-classification production
  files of both crates, and swept the test vocabulary, the ADF render-abort
  fixtures, and the CLI exit-code module docs.
- **Renamed the parity fixtures** `bash-exit-codes.txt → captured-exit-codes.txt`
  (and their capture scripts) as renames, with every `NAME=INT` row
  byte-identical.
- **New unit coverage** for the relocated maps: per-variant `Outcome → code`
  tests in each CLI, a `for_adf` variant test, and a `linear-cli` structural
  test that the map never yields a Jira-only code `{12,13,14,15,17,19}`.

## Context

- Work item: `meta/work/0269-remove-bash-references-from-jira-linear-clients.md`
- Plan: `meta/plans/2026-09-06-0269-remove-bash-vocabulary-and-redesign-exit-code-classification.md`
- Research: `meta/research/codebase/2026-09-06-0269-remove-bash-vocabulary-and-redesign-exit-code-classification.md`
- Validation: `meta/validations/2026-09-06-0269-remove-bash-vocabulary-and-redesign-exit-code-classification-validation.md`

The split is intentionally asymmetric: the granular numeric map leaves the
client crates, but the retry-class `classify` and the `From<Failure> for
TrackerError` collapse **stay** inside them. Relocating that collapse is item
0271's job; extracting a tracker-adapter crate is 0273's. This item leaves the
collapse in place, re-expressed on `(Outcome, Operation)`.

## Testing

- [x] CI read-only gate green: `mise run cli:check` exits 0.
- [x] Client suites pass: `classify`, `discriminant`, `timeouts`, `transport`
      (both crates).
- [x] CLI suites pass: `exit_codes_parity` and `exit_codes::tests` (both CLIs),
      proving no emitted exit-code value changed.
- [x] End-to-end routing unchanged: `flow_errors` under `--features
      test-loopback` (jira 2, linear 4).
- [x] Vocabulary gates clean: `grep -rin bash` over both client crates' `src/`
      and `Cargo.toml` returns nothing; no `bash_code`/`classify_bash_code`
      symbol; no `AdfError::code`; no `11`–`36` literal in the classification
      files.
- [ ] `jira-client`'s live-tenant `contract.rs` tests are not run here — they
      fail-not-skip without `ACCELERATOR_JIRA_*` credentials by design, so they
      need a credentialled environment (or the `ACCELERATOR_TRACKER_CONTRACT`
      gate) to exercise.

## Notes for Reviewers

- **Contract preserved, not changed.** This is a structural/vocabulary change;
  the exit-code values are a frozen external contract. The parity fixtures pin
  every value, and their `NAME=INT` rows are byte-identical across the rename.
- **Focus on the two `classify` rewrites** (`jira-client/src/classify.rs`,
  `linear-client/src/classify.rs`) and the linear code-34 divergence, which is a
  retry-class distinction visible only in the `classify` `TABLE`, not in
  `flow_errors`.
- **One known deviation from the plan:** the CLI outcome→code unit tests are
  fixed `assert_eq!` lists rather than in-test exhaustive matches, so a new
  `Outcome` variant would not force a new assertion there. Low severity — the
  production mapping matches are exhaustive with no wildcard (a new variant is a
  build error), and the client-crate `classify` table guards do use exhaustive
  matches. Recorded in the validation report.
- **Rider commit:** this branch also carries `Mark work item 0240 as done` (a
  two-line status flip on `0240-corpus-update-frontmatter-quote-roundtrip.md`),
  unrelated to 0269. It sits at the base of the chain and was kept in this PR
  deliberately rather than rebased out.
