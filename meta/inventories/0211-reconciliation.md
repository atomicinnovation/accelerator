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

_Pending — recorded at Phase 4's deletion boundary._
