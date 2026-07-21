---
type: inventory
id: "0167-deletion-ledger"
title: "0167 Removal-Set Deletion Ledger"
date: "2026-07-21T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: in-progress
parent: "work-item:0167"
relates_to:
  - "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
tags: [rust, config, cli, migration, removal-set, deletion]
last_updated: "2026-07-21T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0167: Removal-Set Deletion Ledger

One row per removal-set path. Each row records the gate that replaces the
script's behaviour, the commit at which that gate went green, and — filled at
Phase 7 — the final-state gate that must be present and passing after the
deleting commit. Authored forwards (in the commit that turns each gate green)
so Phase 7's replay verifies a record built forwards, never one read off the
history it certifies.

`covering gate` values name the replacing Rust subcommand + its black-box tests
in `cli/launcher/tests/config_read.rs` unless stated otherwise. The
`commit` column holds the change that turned that gate green;
`final-state gate` is filled at Phase 7 deletion.

| Removal-set path | Covering gate | Commit where green | Final-state gate |
|---|---|---|---|
| `scripts/config-read-value.sh` | `config get` (`get_*` tests) | zywwkxum | |
| `scripts/config-read-path.sh` | `config path` (`path_*` tests) | zywwkxum | |
| `scripts/config-read-agent-name.sh` | `config agent` (`agent_*` tests) | zywwkxum | |
| `scripts/config-read-agents.sh` | `config agents` (`agents_*` tests) | nuzkvtso | |
| `scripts/config-read-work.sh` | `config work` (`work_*` tests) | nuzkvtso | |
| `scripts/config-read-context.sh` | `config context` (`context_*` tests) | tqpkksum | |
| `scripts/config-read-skill-context.sh` | `config context --skill` (`context_skill_*` tests) | tqpkksum | |
| `scripts/config-read-skill-instructions.sh` | `config instructions` (`instructions_*` tests) | tqpkksum | |
| `scripts/config-read-all-paths.sh` | `config paths` (`paths_*` tests) | lkvlpovs | |
| `scripts/config-read-doc-type-paths.sh` | `config paths --doc-types` (`paths_doc_types_*` tests) | lkvlpovs | |
| `scripts/config-dump.sh` | `config dump` (`dump_*` tests) | zuklzyku | |
| `scripts/config-read-review.sh` | `config review` (`review_*` tests) | kmzquzwr | |
| `scripts/config-summary.sh` | `config summary` (`summary_*` tests) | xuyskuzq | |
| `scripts/config-read-template.sh` | `config template` (`template_*` tests) | qryqkuox | |
| `scripts/config-list-template.sh` | `config templates list` (`templates_list_*` tests) | qryqkuox | |
| `scripts/config-show-template.sh` | `config templates show` (`templates_show_*` tests) | qryqkuox | |
| `scripts/config-eject-template.sh` | `config templates eject` | (Phase 3) | |
| `scripts/config-diff-template.sh` | `config templates diff` | (Phase 3) | |
| `scripts/config-reset-template.sh` | `config templates reset` | (Phase 3) | |
| `skills/config/init/scripts/init.sh` | `config init` | (Phase 3) | |

Not on the removal set (retained): `scripts/config-common.sh` (0174),
`scripts/config-read-browser-executor.sh` (0173), `scripts/config-defaults.sh`
(retires with `config-common.sh` in 0174).
