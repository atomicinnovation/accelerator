---
type: inventory
id: "0167-config-behaviour-inventory"
title: "0167 Config Behaviour Inventory (Non-Repointable Remainder)"
date: "2026-07-21T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: in-progress
parent: "work-item:0167"
relates_to:
  - "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
tags: [rust, config, cli, migration, inventory, parity]
last_updated: "2026-07-21T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0167: Config Behaviour Inventory

The repointed shell suites (Phase 4) are the parity gate for the bulk of the
config behaviour. This inventory covers only the **four non-repointable
members** — the assertions that cannot survive a mechanical repoint at the
compiled binary — so nothing is silently lost when the removal set is deleted.

Pinned revision (removal set whole): **`vnorwskwqlrv`**.

Cardinality is recorded, not reconciled against the 337 `test-config.sh`
`assert_` sites — that figure counts the *repointable* suite and bears no
relation to the remainder's size.

## Member 1 — `test-config.sh` regions that grep consumer source text

These greps count a removal-set script name *inside* SKILL.md / shell / Rust
source. They assert the invocation contract this story rewrites, so they break
by design at the Phase 5 cutover (most match the *old* name and would pass
vacuously afterwards). Each region is dispositioned: **ported** to a Rust test,
**rewritten** against the new invocation shape in a surviving gate, or
**dropped with a reason**. The regions are enumerated by pattern (grep for every
removal-set basename in the suite), not by hand.

| Region | Kind | Disposition |
|---|---|---|
| call-site greps (`:1095-1101`, `:1139-1194`, `:2995-3032`, `:4985-5027`, `:5029+`) | invocation-contract census | rewritten into `check-call-site-migration.sh` (Grep A/B) + `check-skill-permissions.sh` (Phase 5) |
| key-registration census (`:2543-2577`) | every jira/linear/visualiser `config-read-value.sh` key is catalogue-registered | rewritten into `check-call-site-migration.sh` (Phase 5); the Rust catalogue drift test (`config/src/catalogue.rs`) also pins registration |
| inline-default census (`:3287-3316`, `:4056-4083`) | no consumer passes a hardcoded inline default | rewritten into `check-call-site-migration.sh` (Phase 5) |
| work-key reader census (`:4085-4114`) | no non-`config-*` file reads `work.*` via `config-read-value.sh` | rewritten into `check-call-site-migration.sh` (Phase 5) |
| allowed-tools coverage census (`:4116-4143`, `:4145-4164`) | every SKILL.md invoking `config-read-work.sh` has a covering `allowed-tools` entry | rewritten into `check-skill-permissions.sh` (Phase 5) |

Status: **pending Phase 5** — the surviving gates (`check-call-site-migration.sh`,
`check-skill-permissions.sh`) are Phase 5 deliverables. This member is recorded
now so the dispositions are pinned before the cutover rewrites the regions.

## Member 2 — `config-defaults.sh` file assertions

`test-config.sh:2441`, `:2525-2532` treat the defaults as a *shell file*. The
Rust catalogue has no such file.

| Assertion | Disposition |
|---|---|
| `DEFAULTS_FILE` existence / content (`:2441`, `:2525-2532`) | dropped: superseded by the Rust catalogue (`config/src/catalogue.rs`) and its bash-parity drift test (`the_rust_catalogue_matches_the_bash_catalogue`), which pins every key/default against the retained `config-defaults.sh` |

## Member 3 — all of `test-init.sh`

Not repointed (the suite has never run in CI, so there is no trustworthy
baseline). Depth floor is **per-assertion**, not per-branch: its most valuable
checks belong to no single `init.sh` branch.

Status: **pending Phase 4** — `test-init.sh` is characterised (wired into
`run_shell_suites`, observed green at a recorded commit) in Phase 4, then its
~25 assertions are ported to Rust `config init` tests in Phase 3/7. Cross-cutting
assertions to port per-assertion: `tree_hash` idempotency (`:91-94`), gitignore
rule non-duplication (`:101-102`), legacy `.claude/accelerator.local.md` rule
preservation (`:104-122`), root discovery from a deep subdirectory (`:124-131`),
`paths.tmp` override (`:133-145`).

## Member 4 — removal-set scripts with no covering suite

**Computed set: empty.** Every removal-set script is named by a surviving
config suite (`test-config.sh` covers the 18 readers/templates/dump/summary;
`test-config-read-doc-type-paths.sh` covers `config-read-doc-type-paths.sh`;
`test-init.sh` covers `init.sh` — member 3). So there is no script whose only
record is a hand-inventory row, which is the likeliest silent-loss path.

`scripts/check-inventory.sh` asserts this set **equality** (removal set minus
suite-covered = ∅) rather than a count floor, so a mis-scoped extractor that
dropped a script would fail against the computed set.

## What the read subcommands already pin

The Phase 2 read subcommands are byte-exact-goldened against the bash readers in
`cli/launcher/tests/config_read.rs` (68 tests) and the reused, cmp-verified
`scripts/test-fixtures/config-read-review/*.txt` goldens. Those goldens are the
durable byte-exact gate for the read surface; this inventory covers only what
they and the repointed suites cannot.
