---
type: inventory
id: "0167-removal-set-references"
title: "0167 Removal-Set Reference Audit"
date: "2026-07-20T00:00:00+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "work-item:0167"
relates_to:
  - "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
tags: [rust, config, cli, migration, removal-set, audit]
last_updated: "2026-07-20T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0167 Removal-Set Reference Audit

Every reference to every removal-set member, in every reference form, with a
disposition per referencing file.

## Why this exists

Three separate review passes each found a different file that references a
removal-set member and survives the migration — `config-common.sh`,
`cli/config/src/catalogue.rs`, and (caught only because it broke the retained
library) `config-defaults.sh`. Each was found by hand, one at a time, and each
time the fix asserted the neighbouring cases were fine without checking.

The cause is that **Grep A cannot see the reference form that keeps breaking.**
It matches removal-set *paths* (`scripts/config-read-value.sh`), while the
references that break are `"$SCRIPT_DIR/config-read-value.sh"`,
`require_script("scripts/config-read-doc-type-paths.sh")` in Rust, and
`source "$scripts/config-dump.sh"` inside a Rust test's bash payload.

This audit is by **basename**, over the whole tree, in every file type.

## Method

```
for each removal-set basename:
    grep -rn <basename> . \
      --exclude-dir={.git,.jj,target,node_modules,.venv,dist}
classify each hit by the referencing file
```

`init.sh` is matched as `config/init/scripts/init\.sh`; the bare basename is too
generic. Re-run the script at `meta/inventories/0167-audit-removal-set.sh`.

## Totals

| Category | Files | References |
|---|---|---|
| SKILL.md | 47 | 267 |
| Shell (non-suite) | 31 | 69 |
| Test suites | 10 | 203 |
| Rust | 4 | 11 |
| Other | 3 | 20 |
| **Distinct referencing files (excluding meta/ prose and the set itself)** | **95** | |

Two counts in the plan are wrong and are corrected here: the shell-consumer
figure is **31**, not 28 (the additions are the retained `config-common.sh`,
`config-defaults.sh` and `accelerator-scaffold.sh`); and the SKILL.md figure is
**47 files**, not 46.

## Disposition by category

### Rust — the least-covered, and where the plan has a live gap

| File | Refs | Disposition |
|---|---|---|
| `cli/config-adapters/tests/parity.rs` | 4 | Covered — Phase 7 §3 |
| `cli/config/src/catalogue.rs` | 4 | **NOT COVERED.** The drift test's `EXTRACT` payload sources `config-dump.sh` (`:301`, sole source of `REVIEW_KEYS`/`AGENT_KEYS`) and invokes `config-read-review.sh` (`:325-326`). Runs under `set -euo pipefail`, so Phase 7 breaks it. The plan currently claims it is "unaffected" — that claim is false |
| `cli/corpus-adapters/tests/common/mod.rs` | 2 | **NOT COVERED.** `doc_type_table()` at `:66` calls `require_script("scripts/config-read-doc-type-paths.sh")` and passes the repo root positionally; imported by `corpus-adapters/tests/parity.rs:16`. A different file from the two the plan names |
| `skills/visualisation/visualise/server/src/config.rs` | 1 | Benign — a comment at `:823`. Goes stale; no functional dependency |

`cli/corpus-adapters/tests/doc_type_single_source.rs` no longer appears: it
references `config-defaults.sh`, which is retained.

### Shell, retained (not on the removal set, not repointed)

| File | Refs | Disposition |
|---|---|---|
| `scripts/config-common.sh` | 5 | **NOT COVERED.** `config_resolve_template` invokes `config-read-value.sh` (`:407`) and `config-read-path.sh` (`:422`). Retained for 0174, so it ships a function referencing two deleted scripts. Every current caller is itself deleted, so nothing breaks at Phase 7 — but the function is silently non-functional until 0174 |
| `scripts/config-defaults.sh` | 9 | Retained (see the plan's removal-set note). References are to sibling catalogue data |
| `scripts/accelerator-scaffold.sh` | 1 | Benign — comment reference to `init.sh` at `:5`. Goes stale |

### Shell, repointed in Phase 5 §4b

28 files: 6 work-item, 4 visualiser, 4 Jira, 2 Linear, 6 migrations, 4 shared
helpers (`doc-type-inference`, `doc-type-table`, `validate-corpus-frontmatter`,
`work-common`), plus `adr-next-number.sh` and the Playwright `run.sh`.

`0003-relocate-accelerator-state.sh`'s single reference is a comment stating it
deliberately does *not* use `config-read-path.sh` — no conversion needed.

### Test suites

| File | Disposition |
|---|---|
| `scripts/test-config.sh` | Repointed Phase 4, deleted Phase 7 |
| `scripts/test-config-read-doc-type-paths.sh` | Repointed Phase 4, deleted Phase 7 |
| `skills/config/init/scripts/test-init.sh` | Characterised Phase 4, deleted Phase 7 |
| `scripts/test-design.sh` | Censuses updated Phase 5 |
| `scripts/test-skill-frontmatter-population.sh` | Censuses updated Phase 5 |
| `scripts/test-validate-corpus-frontmatter.sh` | `exec` stub, Phase 7 §3 |
| `skills/config/migrate/scripts/test-migrate-0007.sh` | `exec` stub, Phase 7 §3 |
| `skills/integrations/jira/scripts/test-jira-paths.sh` | Suite audit |
| `skills/work/scripts/test-work-item-create-remote.sh` | Suite audit |
| `skills/work/scripts/test-work-item-scripts.sh` | Suite audit |

### Other

| File | Refs | Disposition |
|---|---|---|
| `CHANGELOG.md` | 17 | Historical record — no action |
| `hooks/hooks.json` | 1 | Covered — Phase 6 §2 |
| `.claude/settings.local.json` | 2 | Untracked local permissions, pinned to plugin version `1.21.0-pre.38`. No action, but note users accumulate version-pinned rules that silently stop matching |

## Open items this audit adds to the plan

1. `cli/config/src/catalogue.rs` — decide disposition; the "unaffected" claim in
   Phase 7 must be corrected either way.
2. `cli/corpus-adapters/tests/common/mod.rs` — add to the pinned-Rust-tests list;
   its positional root passing interacts with the `paths --doc-types [root]`
   decision.
3. `config_resolve_template` in the retained `config-common.sh` — delete with its
   callers in Phase 7, or repoint, or record as knowingly dormant until 0174.
4. Grep A must match by **basename** as well as path, with a pre-migration
   known-positive floor that includes `config-common.sh:407,422` and
   `catalogue.rs:301,325`.
