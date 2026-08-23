---
type: inventory
id: "0211-reconciliation"
title: "0211 Executable Reconciliation"
date: "2026-08-23T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: in-progress
work_item_id: "work-item:0211"
parent: "work-item:0211"
relates_to:
  - "plan:2026-08-19-0211-integration-binaries-and-bash-cluster-retirement"
tags: [jira, linear, integrations, cli, cutover, reconciliation]
last_updated: "2026-08-23T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0211: Executable Reconciliation

Every retired cluster executable and library maps to its subsuming
`accelerator <provider>` subcommand or is named as dropped, so no product
surface silently disappears.

## Linear track (Phase 2) — 10 executables + 2 libraries

| Bash executable | Disposition |
|---|---|
| `linear-create-flow.sh` | `linear create` |
| `linear-update-flow.sh` | `linear update` |
| `linear-show-flow.sh` | `linear show` |
| `linear-search-flow.sh` | `linear search` (`filter::compose` + the read-side projection op, Decision 20) |
| `linear-comment-flow.sh` | `linear comment add` |
| `linear-transition-flow.sh` | `linear transition` (`resolve_state` + `transition`) |
| `linear-attach-flow.sh` | `linear attach --url \| --file` |
| `linear-init-flow.sh` | `linear init verify \| list-teams \| discover` |
| `linear-auth-cli.sh` | dropped — subsumed by `init verify` (Decision 3) |
| `linear-graphql.sh` | dropped — internal transport, subsumed by `linear-client` |
| `linear-common.sh`, `linear-auth.sh` (libraries) | subsumed by the crate |

**`SKILL.md`-reachable entrypoints: 9, not 10.** `linear-graphql.sh` is named by
no `SKILL.md` (not even in prose) and was reachable only through the read/init
skills' wildcard `scripts/*` glob grant, which is now gone. **Dispatch-mode
count: 6, not "roughly 15"** — nine of the ten executables are flag-and-
positional only; only `init` has named modes (verify/list-teams/discover).

## Jira track (Phase 4) — 17 executables + 5 libraries + 3 data assets

| Bash executable | Disposition |
|---|---|
| `jira-create-flow.sh` | `jira create` |
| `jira-update-flow.sh` | `jira update` |
| `jira-show-flow.sh` | `jira show` |
| `jira-search-flow.sh` | `jira search` (`jql::compose` + the read-side projection op, Decision 20) |
| `jira-comment-flow.sh` | `jira comment add \| list \| edit \| delete` |
| `jira-transition-flow.sh` | `jira transition` |
| `jira-attach-flow.sh` | `jira attach` |
| `jira-init-flow.sh` | `jira init verify \| discover \| prompt-default \| refresh-fields \| list-projects \| list-fields` |
| `jira-resolve-fields.sh` | `jira resolve-fields` (Decision 4) |
| `jira-emit-key.sh` | `jira create --emit key` (Decision 5) |
| `jira-fields.sh` | `jira fields refresh \| resolve \| list` (the dual-use script) |
| `jira-render-adf-fields.sh` | subsumed by `--render-adf` (`jira-cli` `render.rs` over `jira-client`'s ADF) |
| `jira-adf-to-md.sh` | subsumed by `jira-client`'s `adf::document_to_markdown` |
| `jira-md-to-adf.sh` | subsumed by `jira-client`'s `adf::markdown_to_document` |
| `jira-request.sh` | subsumed — the bounded transport is `jira-client`'s `transport.rs` |
| `jira-auth-cli.sh` | dropped — subsumed by `init verify` (Decision 3) |
| `jira-jql-cli.sh` | dropped — orphan, invoked only by its own test (Decision 5) |
| `jira-common.sh`, `jira-auth.sh`, `jira-jql.sh`, `jira-body-input.sh`, `jira-custom-fields.sh` (libraries) | subsumed by the crate (`auth.rs`, `jql.rs`, `custom_fields.rs`; interactive body resolution stays out of the crate) |
| `jira-adf-render.jq`, `jira-md-tokenise.awk`, `jira-md-assemble.jq` (data assets) | subsumed by `jira-client`'s `adf/` module |

**`SKILL.md`-reachable entrypoints: 11, not 8** — the eight flows plus
`jira-auth-cli.sh`, `jira-resolve-fields.sh` and `jira-emit-key.sh` (named by
`create-jira-issue`/`init-jira`). **Dispatch-mode count: 21** — `comment`'s four
subcommands, `init`'s six, `fields`' three and `resolve-fields` bring the 17
executables to 21 named modes.
