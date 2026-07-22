---
type: inventory
id: "0167-deletion-ledger-replay"
title: "0167 Deletion-Ledger Replay Output"
date: "2026-07-22T16:32:47+00:00"
author: Toby Clemson
producer: implement-plan
status: complete
parent: "work-item:0167"
relates_to:
  - "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
  - "inventory:0167-deletion-ledger"
tags: [rust, config, cli, migration, removal-set, deletion, replay]
last_updated: "2026-07-22T16:32:47+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0167: Deletion-Ledger Replay Output

A captured resolution snapshot of the deletion-ledger replay against the
final-state tree. The replay is enforced by the Python gate
`tasks/lint/deletion_ledger_replay.py` (`mise run lint:deletion-ledger-replay:check`,
per ADR-0048), which supersedes the original `scripts/replay-deletion-ledger.sh`.
For every removal-set row, the named covering-gate test prefix resolves to at
least one surviving `#[test]` in the final-state gate file — so each deleted
script's behaviour is pinned by a test in a file the Phase 7 deletion did
**not** remove, never solely by the now-deleted `test-config.sh`.

The replay carries its own known-positive floor: a built-in self-test proves a
bogus prefix is rejected, and a mis-named gate row (verified out-of-band) makes
the replay fail rather than pass vacuously.

```
== 0167 deletion-ledger replay ==
replay: self-test ok (bogus prefix rejected, known-good prefix resolves)
  scripts/config-read-value.sh             get_*                -> cli/launcher/tests/config_read.rs
  scripts/config-read-path.sh              path_*               -> cli/launcher/tests/config_read.rs
  scripts/config-read-agent-name.sh        agent_*              -> cli/launcher/tests/config_read.rs
  scripts/config-read-agents.sh            agents_*             -> cli/launcher/tests/config_read.rs
  scripts/config-read-work.sh              work_*               -> cli/launcher/tests/config_read.rs
  scripts/config-read-context.sh           context_*            -> cli/launcher/tests/config_read.rs
  scripts/config-read-skill-context.sh     context_skill_*      -> cli/launcher/tests/config_read.rs
  scripts/config-read-skill-instructions.sh instructions_*       -> cli/launcher/tests/config_read.rs
  scripts/config-read-all-paths.sh         paths_*              -> cli/launcher/tests/config_read.rs
  scripts/config-read-doc-type-paths.sh    paths_doc_types_*    -> cli/launcher/tests/config_read.rs
  scripts/config-dump.sh                   dump_*               -> cli/launcher/tests/config_read.rs
  scripts/config-read-review.sh            review_*             -> cli/launcher/tests/config_read.rs
  scripts/config-summary.sh                summary_*            -> cli/launcher/tests/config_read.rs
  scripts/config-read-template.sh          template_*           -> cli/launcher/tests/config_read.rs
  scripts/config-list-template.sh          templates_list_*     -> cli/launcher/tests/config_read.rs
  scripts/config-show-template.sh          templates_show_*     -> cli/launcher/tests/config_read.rs
  scripts/config-eject-template.sh         eject_*              -> cli/launcher/tests/config_read.rs
  scripts/config-diff-template.sh          diff_*               -> cli/launcher/tests/config_read.rs
  scripts/config-reset-template.sh         reset_*              -> cli/launcher/tests/config_read.rs
  skills/config/init/scripts/init.sh       init_*               -> cli/launcher/tests/config_read.rs
replay: 20 ledger row(s) resolve to a surviving test
replay: running the surviving config-read suite to assert it passes...
replay: config-read suite passed (present AND passing)
replay-deletion-ledger: OK
```
