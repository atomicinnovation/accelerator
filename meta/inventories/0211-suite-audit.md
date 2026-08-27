---
type: inventory
id: "0211-suite-audit"
title: "0211 Integration Suite Audit"
date: "2026-08-23T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: in-progress
work_item_id: "work-item:0211"
parent: "work-item:0211"
relates_to:
  - "plan:2026-08-19-0211-integration-binaries-and-bash-cluster-retirement"
tags: [jira, linear, integrations, cli, cutover, suite-audit]
last_updated: "2026-08-23T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0211: Integration Suite Audit

Where the coverage of each retired bash suite now lives. The governing rule
(0167): every retired suite names the Rust test that carries its behaviour, so
no coverage silently vanishes with the shell.

## Linear track (Phase 2) — 12 suites retired

| Retired suite | Coverage now in |
|---|---|
| `test-linear-create.sh` | `cli/linear-cli/tests/flow_create.rs` (created + writeback-failed) + `cli/linear-client/tests/port.rs` (`create`) |
| `test-linear-update.sh` | `cli/linear-cli/tests/flow_update.rs` + `cli/linear-client/tests/port.rs` (`update`) |
| `test-linear-show.sh` | `cli/linear-cli/tests/flow_show.rs` + `stdout_goldens.rs` + `cli/linear-client/tests/port.rs` (`show`) |
| `test-linear-search.sh` | `cli/linear-cli/tests/flow_search.rs` + `stdout_goldens.rs` + `cli/linear-client/tests/search_projection.rs` |
| `test-linear-comment.sh` | `cli/linear-cli/tests/flow_comment.rs` + `cli/linear-client/tests/comment.rs` |
| `test-linear-transition.sh` | `cli/linear-cli/tests/flow_transition.rs` + `cli/linear-client/tests/transition.rs` |
| `test-linear-attach.sh` | `cli/linear-cli/tests/flow_attach.rs` (link + binary multi-POST) + `cli/linear-client/tests/attach.rs` (every upload-failure variant) |
| `test-linear-init-flow.sh` | `cli/linear-cli/tests/flow_init.rs` (verify/list-teams/discover + cache writes) + `cli/linear-client/tests/discovery.rs` |
| `test-linear-auth.sh` | `cli/linear-cli/tests/flow_seam.rs` (credential resolution + no-token) + `cli/linear-client/tests/auth.rs` |
| `test-linear-graphql.sh` | `cli/linear-client/tests/transport.rs` + `classify.rs` (transport, retry, rate-limit, error classification) |
| `test-linear-common.sh` | subsumed — the shared shell helpers are gone; their behaviour lives in `linear-client`'s typed surface |
| `test-linear-paths.sh` | `cli/linear-cli/tests/exit_codes_parity.rs` (the machine-pinned exit-code contract, formerly `test-linear-paths.sh:71-103`) |

The exit-code oracle formerly pinned by `test-linear-paths.sh` is now the
committed `bash-exit-codes.txt` fixture, and the parity test is independent of
the constants it guards (Decision 6).

## Jira track (Phase 4) — 21 suites retired

| Retired suite | Coverage now in |
|---|---|
| `test-jira-create.sh` | `cli/jira-cli/tests/flow_create.rs` (created, `--emit key`, the rich field set) + `cli/jira-client/tests/{port,mutation}.rs` (`create`) |
| `test-jira-update.sh` | `cli/jira-cli/tests/flow_update.rs` + `cli/jira-client/tests/{port,mutation}.rs` (`update`, set + incremental channels) |
| `test-jira-show.sh` | `cli/jira-cli/tests/flow_show.rs` + `stdout_goldens.rs` + `cli/jira-client/tests/port.rs` (`show`) |
| `test-jira-search.sh` | `cli/jira-cli/tests/flow_search.rs` + `stdout_goldens.rs` + `cli/jira-client/tests/{port,jql}.rs` |
| `test-jira-comment.sh` | `cli/jira-cli/tests/flow_comment.rs` + `cli/jira-client/tests/comment.rs` |
| `test-jira-transition.sh` | `cli/jira-cli/tests/flow_transition.rs` + `cli/jira-client/tests/transition.rs` |
| `test-jira-attach.sh` | `cli/jira-cli/tests/flow_attach.rs` + `cli/jira-client/tests/{attach,multipart}.rs` |
| `test-jira-init-flow.sh` | `cli/jira-cli/tests/flow_init.rs` (verify/discover/prompt-default + cache writes, TTY refusal) + `cli/jira-client/tests/discovery.rs` |
| `test-jira-auth.sh` | `cli/jira-cli/tests/flow_seam.rs` (credential resolution + no-token) + `cli/jira-client/tests/auth.rs` |
| `test-jira-request.sh` | `cli/jira-client/tests/{transport,classify}.rs` (transport, retry, rate-limit, status classification) |
| `test-jira-jql.sh` | `cli/jira-client/tests/jql.rs` (JQL composition, quoting, `@me`) |
| `test-jira-fields.sh` | `cli/jira-cli/tests/flow_fields.rs` (refresh/resolve/list + the cache-version marker) |
| `test-jira-custom-fields.sh` | `cli/jira-client/tests/custom_fields.rs` (slug resolution + schema-typed coercion + `@json:`) |
| `test-jira-adf-to-md.sh` | `cli/jira-client/tests/{adf,adf_differential}.rs` (ADF → Markdown, against the frozen oracle) |
| `test-jira-md-to-adf.sh` | `cli/jira-client/tests/{adf,adf_differential}.rs` (Markdown → ADF, against the frozen oracle) |
| `test-jira-adf-roundtrip.sh` | `cli/jira-client/tests/adf_differential.rs` + `adf_differential_self_test.rs` (the differential and its can-reject proof) |
| `test-jira-render-adf-fields.sh` | `cli/jira-cli/tests/{flow_show,flow_search}.rs` (`--render-adf`) over `cli/jira-cli/src/render.rs` |
| `test-jira-body-input.sh` | subsumed — the interactive `$EDITOR`/stdin body resolution stays out of the crate; `--body`/`--body-file` are the binary's, covered by `flow_create.rs`/`flow_update.rs` |
| `test-jira-common.sh` | subsumed — the shared shell helpers are gone; their behaviour lives in `jira-client`'s typed surface |
| `test-jira-paths.sh` | `cli/jira-cli/tests/exit_codes_parity.rs` (the exit-code contract pinned against `bash-exit-codes.txt`, independent of the constants it guards) |
| `test-jira-scripts.sh` | none needed — the umbrella runner only re-invoked the individual suites (excluded from discovery via `EXCLUDED_HELPER_NAMES`) |
