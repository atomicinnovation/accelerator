---
type: inventory
id: "0167-removal-set"
title: "0167 Config-Cluster Removal Set"
date: "2026-07-22T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: complete
parent: "work-item:0167"
relates_to:
  - "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
tags: [rust, config, cli, migration, removal-set, deletion]
last_updated: "2026-07-22T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0167: Config-Cluster Removal Set

The explicit list of scripts deleted at Phase 7 once every call site, shell
consumer, hook, Rust dependant and test binding was cut over to
`accelerator config`. An explicit file list, not a category description, so the
deletion is auditable against `check-inventory.sh`'s pinned-revision floor.

```
scripts/config-read-value.sh          scripts/config-read-template.sh
scripts/config-read-path.sh           scripts/config-list-template.sh
scripts/config-read-all-paths.sh      scripts/config-show-template.sh
scripts/config-read-doc-type-paths.sh scripts/config-eject-template.sh
scripts/config-read-work.sh           scripts/config-diff-template.sh
scripts/config-read-agents.sh         scripts/config-reset-template.sh
scripts/config-read-agent-name.sh     scripts/config-dump.sh
scripts/config-read-context.sh        scripts/config-summary.sh
scripts/config-read-review.sh         skills/config/init/scripts/init.sh
scripts/config-read-skill-context.sh
scripts/config-read-skill-instructions.sh
```

Twenty paths (nineteen `scripts/config-*.sh` plus `init.sh`).

## Also deleted at Phase 7 (not on the set proper)

- `scripts/test-config.sh` and `scripts/test-config-read-doc-type-paths.sh` — the
  superseded parity suites, retired once green against the compiled binary.
- `skills/config/init/scripts/test-init.sh` — the characterised init suite,
  retired against its recorded Phase 4 green run.
- `scripts/test-shims/` — the Phase 4 shim scripts and `rebind-floor.sh`, which
  existed only to point `test-config.sh` at the binary.
- `config_resolve_template` (and its `CONFIG_TEMPLATE_SOURCE_*` constants) in the
  retained `scripts/config-common.sh` — it invoked two removal-set scripts and
  every caller was deleted here.

## Deliberately NOT on the set (retained)

- `scripts/config-common.sh` — 0174 owns its retirement; ~18 non-cluster
  production sourcers survive.
- `scripts/config-read-browser-executor.sh` — 0173 owns its migration.
- `scripts/config-defaults.sh` — retires with `config-common.sh` in 0174; it is
  sourced unconditionally by `config-common.sh` and now also holds the review and
  agent catalogue arrays the drift test reads.
