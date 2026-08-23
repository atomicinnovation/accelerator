---
type: inventory
id: "0211-divergences"
title: "0211 Behavioural Divergences Ledger"
date: "2026-08-23T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: in-progress
work_item_id: "work-item:0211"
parent: "work-item:0211"
relates_to:
  - "plan:2026-08-19-0211-integration-binaries-and-bash-cluster-retirement"
tags: [jira, linear, integrations, cli, cutover, divergences, exit-codes]
last_updated: "2026-08-23T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0211: Behavioural Divergences Ledger

Every place the `accelerator <provider>` sub-binary deliberately behaves
differently from the retired bash flow. The governing rule transfers verbatim
from 0167: **a divergence nothing can detect is indistinguishable from a defect,
so every row names a real, passing test.**

## Linear track (Phase 2)

| Divergence | Why | Detecting test |
|---|---|---|
| Search codes remapped `70`–`73` → `75`–`78` | The dispatch layer reserves `70`–`74` (`E_DISPATCH_UNCONFIGURED` = 74, Decision 16); a search code in that band would read as a dispatch verdict at the composition root | `cli/linear-cli/tests/exit_codes_parity.rs::no_code_lands_on_the_reserved_dispatch_band` and `::the_allowlist_is_count_pinned_and_moves_off_the_reserved_band` — the count-pinned allowlist asserts the remapped value while `bash-exit-codes.txt` keeps the original |
| Errors route to exit codes + `E_*` stderr, not to a keyword | The keyword discriminant (Decision 11) carries **success** outcomes; error classes have no keyword, so a repointed body handling an error keys on the `E_*` stderr name or a non-zero exit, not a keyword | `cli/linear-cli/tests/flow_errors.rs` (observed exit code per class) + `stderr_diagnostics.rs` (the `E_*` names asserted verbatim); the `for_surface`/`for_client`/`for_failure` maps are wildcard-free so a new variant is a compile error |
| Wire-payload preview dropped; the body previews resolved intent | The client crates expose no mutation-payload composer, so `--print-payload`/`--describe` are not reproduced (Decision 2); the write skills preview title/target/fields and confirm before an atomic send | `tasks/shared/skill_write_gate.py` (confirm precedes the mutation, `test_integration_skills.py::TestWriteGate`) + the stdout-before-mutation seam covered by the flow tests |
| Cleartext-credential subcommand dropped | `linear-auth-cli.sh` is not reproduced; credential validation folds into `init verify`, which resolves and checks the token without printing it (Decision 3) | `cli/linear-cli/tests/flow_init.rs::init_verify_persists_the_viewer_without_leaking_the_token` — the sentinel token appears on neither stdout, stderr nor the written `viewer.json` |
| `init` owns cache production | The repointed `init-linear` skill drops the `Write` grant, so the binary writes `viewer.json` (verify) and `catalogue.json` (discover) via `LinearCache`, scaffolding the state dir and `.gitignore` itself | `cli/linear-cli/tests/flow_init.rs::init_verify_persists_the_viewer_without_leaking_the_token` and `::init_discover_persists_the_catalogue` |
| `resolve_state` is a local catalogue lookup, so `create`/`transition` are single-POST | The plan assumed `transition`'s `resolve_state` was a network round-trip; the implementation resolves from the cached catalogue, so the only genuine linear multi-POST flow is the binary-attach three-step | `cli/linear-cli/tests/flow_attach.rs::attach_file_uploads_then_registers_across_two_posts` (two `/graphql` POSTs + one PUT, asserted per hit); the `http-test-support` per-hit body log is boundary-tested in `cli/http-test-support/tests/server.rs` |
| Search JSON envelope carries `state`/`assignee` | The port `search` returns stamps only; the search subcommand binds an additive read-side projection op (Decision 20) selecting `state { name }` and `assignee { name }` with the title preserved, so the rendered table is not degraded | `cli/linear-cli/tests/stdout_goldens.rs::search_stdout_matches_the_golden` (byte-exact envelope) + `flow_search.rs` |

No residual search-envelope client-vs-bash shape gap was found: the read-side op
carries every column the bash table rendered.

## Jira track (Phase 3 — binary)

| Divergence | Why | Detecting test |
|---|---|---|
| Search codes remapped `70`–`73` → `75`–`78` | The dispatch layer reserves `70`–`74` (`E_DISPATCH_UNCONFIGURED` = 74, Decision 16); a search code in that band would read as a dispatch verdict at the composition root | `cli/jira-cli/tests/exit_codes_parity.rs::no_code_lands_on_the_reserved_dispatch_band` and `::the_allowlist_is_count_pinned_and_moves_off_the_reserved_band` — the count-pinned allowlist asserts the remapped value while `bash-exit-codes.txt` keeps the original |
| Errors route to exit codes + `E_*` stderr, not to a keyword | The keyword discriminant (Decision 11) carries **success** outcomes; error classes have no keyword, so a repointed body handling an error keys on the `E_*` stderr name or a non-zero exit | `cli/jira-cli/tests/flow_errors.rs` (observed exit code per class); the `for_surface`/`for_client`/`for_failure`/`for_credential` maps are wildcard-free so a new variant is a compile error, and `exit_codes_parity.rs::exit_codes_never_parses_a_tracker_error_detail` proves the code is read structurally |
| Cleartext-credential subcommand dropped | `jira-auth-cli.sh` is not reproduced; credential validation folds into `init verify`, which resolves and checks the token without printing it (Decision 3) | `cli/jira-cli/tests/flow_init.rs::verify_caches_the_site_and_stamps_the_outcome`, `::a_verify_failure_never_leaks_the_token`, `::a_missing_token_never_leaks_and_maps_to_no_token` — the sentinel token appears on neither stdout nor stderr on any exit path |
| `init` owns cache production | The repointed `init-jira` skill drops the `Write` grant, so the binary writes `site.json` (verify) and `projects.json`/`fields.json` (discover) via `JiraCache`, scaffolding the state dir itself | `cli/jira-cli/tests/flow_init.rs::verify_caches_the_site_and_stamps_the_outcome` |
| `jira create --emit key` emits the bare key, no keyword | The post-create writeback consumer reads a bare validated key; the keyword discriminant is suppressed in `--emit key` mode, and the created-but-unwritable orphan is signalled by exit 16 (Decision 5/11) | `cli/jira-cli/tests/flow_create.rs::emit_key_prints_only_the_bare_key` and `::a_control_byte_key_fails_closed_at_exit_16` |
| Custom-field composition not reproduced | The binary installs the no-op `FixedResolver` for both account and field resolution (`from_config`), so a `--custom slug=value` mutation and `@me` slug resolution are not composed; the CLI surface carries no custom-field flags | `cli/jira-cli/tests/cli_surface.rs::the_full_cli_surface_matches_the_committed_golden` pins the surface, so re-introducing the flags is a visible golden diff; the 3 `*custom-field*` scenarios are ledgered in `0211-fixture-reconciliation.md` |

## Jira track (Phase 4 — cutover)

_Pending — the write-gate, doc-vs-binary parity and deletion-boundary rows are
recorded at Phase 4's deletion boundary._
