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

## Jira track (Phase 4) — 21 suites

_Pending — recorded at Phase 4's deletion boundary._
