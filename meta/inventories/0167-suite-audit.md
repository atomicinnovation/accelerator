---
type: inventory
id: "0167-suite-audit"
title: "0167 Surviving-Suite Audit"
date: "2026-07-21T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: complete
parent: "work-item:0167"
relates_to:
  - "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
tags: [rust, config, cli, migration, suite-audit]
last_updated: "2026-07-22T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0167: Surviving-Suite Audit

Pinned revision: **`vnorwskwqlrv`**. At this revision `run_shell_suites(context,
"scripts")` discovers **21** executable `test-*.sh` suites (matching
`_EXPECTED_CONFIG_SUITES = 21`). Each is classified:

- **(a)** exercises no removal-set script,
- **(b)** inventoried and ported,
- **(c)** repointed / rewritten against the new invocation shape.

Membership is derived mechanically (grep every discovered suite for every
removal-set basename, then extend to the transitive closure through sourced
helpers), not by hand. The tree-wide reference side is enumerated in
`0167-removal-set-references.md`.

## The `scripts/` config subtree (21 suites)

| Suite | Reaches removal set | Class | Note |
|---|---|---|---|
| `test-config.sh` | 18 readers/templates (direct) | (c) | repointed at the compiled binary via shims (Phase 4) |
| `test-config-read-doc-type-paths.sh` | `config-read-doc-type-paths` (direct) | (c) | repointed (Phase 4); the resolver survives as a full command |
| `test-design.sh` | `config-read-agents/context/path/skill-instructions` (SKILL.md census) | (c) | rewritten against the new invocation shape at the Phase 5 cutover |
| `test-skill-frontmatter-population.sh` | `config-read-template` (SKILL.md census) | (c) | rewritten at the Phase 5 cutover |
| `test-validate-corpus-frontmatter.sh` | `config-read-doc-type-paths` (via an `exec` stub of the resolver) | (c) | keeps its only injection point; the resolver survives as a command |
| `test-doc-type-inference.sh` | `config-read-doc-type-paths` (transitive: `doc-type-inference` → `doc-type-table` → resolver) | (c) | repointed via the `doc-type-table.sh` shell-consumer repoint (Phase 5) |
| `test-atomic-common.sh` | — | (a) | |
| `test-boundary-evals.sh` | — | (a) | |
| `test-evals-structure-self.sh` | — | (a) | |
| `test-evals-structure.sh` | — | (a) | |
| `test-format.sh` | — | (a) | |
| `test-hash-common.sh` | — | (a) | |
| `test-hierarchy-format.sh` | — | (a) | |
| `test-interactive-protocol.sh` | — | (a) | |
| `test-lens-structure.sh` | — | (a) | |
| `test-linkage-parser.sh` | — | (a) | |
| `test-merge-move.sh` | — | (a) | |
| `test-metadata-helpers.sh` | — | (a) | |
| `test-skill-frontmatter-conformance.sh` | — | (a) | |
| `test-skills-index.sh` | — | (a) | |
| `test-template-frontmatter.sh` | — | (a) | |

`test-init.sh` (`skills/config/init/scripts/`) is **not** among the eight
subtrees at this revision; Phase 4 adds a dedicated `run_shell_suites(context,
"skills/config/init")` call site, so it enters the audit corpus then and is
inventoried under member 3 of the behaviour inventory.

## Rust tests pinning the shell surface (break at deletion, not cutover)

- `cli/config-adapters/tests/parity.rs:42-43,113-121` — asserts
  `config-read-value.sh` is a file, then shells out to it. Phase 7 removes the
  differential shell-out helpers and retains the declared-value tests
  (divergences 10-12).
- `cli/corpus-adapters/tests/doc_type_single_source.rs:189-220` — sources
  `config-defaults.sh` (retained for 0174); repointing moves to 0174.

## Suites writing `exec` stubs that hard-code the resolver path

- `scripts/test-validate-corpus-frontmatter.sh:412`
- `skills/config/migrate/scripts/test-migrate-0007.sh:2208`

Both keep working because `DOC_TYPE_PATHS_RESOLVER` survives as a full command.

## Final-state discovery run (Phase 7, 2026-07-22)

After the Phase 7 deletion, `run_shell_suites(context, "scripts")` discovers
**19** executable `test-*.sh` suites (`_EXPECTED_CONFIG_SUITES` lowered
21 → 19). The two-suite difference is attributed to named deletions:

- `test-config.sh` — retired; the config behaviour it gated is covered by the
  `config_read.rs` black-box suite (deletion ledger).
- `test-config-read-doc-type-paths.sh` — retired; `config paths --doc-types` is
  covered by `config_read.rs` and the repointed `corpus-adapters` harness.

The `skills/config/init` subtree now discovers **0** suites, so its dedicated
call site, `test:integration:init` task, and `_EXPECTED_INIT_SUITES` floor were
removed: `test-init.sh` was retired against its recorded Phase 4 green run and
`init.sh`'s contract is covered by `config_read.rs`'s `init_*` tests.

No suite was added. Every surviving `(c)`-class suite above keeps working
against the compiled binary: `test-design.sh` and
`test-skill-frontmatter-population.sh` (SKILL.md censuses, rewritten at the
Phase 5 cutover), `test-validate-corpus-frontmatter.sh` and
`test-doc-type-inference.sh` (resolver reached through the `config paths
--doc-types` command that survives).

The Rust pins listed above are resolved: `parity.rs`'s differential shell-out
helpers are removed (the declared-value divergence tests 10-12 retained);
`corpus-adapters/tests/common/mod.rs` is repointed at the compiled launcher; the
catalogue drift test's extractor no longer sources `config-dump.sh` or invokes
`config-read-review.sh`. `doc_type_single_source.rs` is unchanged (it sources
the retained `config-defaults.sh`; repointing moves to 0174).
