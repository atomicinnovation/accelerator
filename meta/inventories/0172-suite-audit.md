---
type: inventory
id: "0172-suite-audit"
title: "0172 Migrate-Suite Assertion Inventory"
date: "2026-08-08T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: complete
parent: "work-item:0172"
relates_to:
  - "plan:2026-08-07-0172-migration-engine-subdomain"
tags: [rust, migrate, cli, migration, suite-audit]
last_updated: "2026-08-08T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0172: Migrate-Suite Assertion Inventory

Pinned revision: **`34d15280adaf`**. `tasks/lint/migrate_suite_inventory.py`
(`inventory()`) discovers, at this revision, **1,010** `assert_*` call sites
across the six retiring suites named in the plan's Technical Notes. No
duplicate site and no raw/inventoried count mismatch
(`tests/unit/tasks/test_migrate_suite_inventory.py::test_the_real_suites_have_no_duplicate_or_gap`).

## Threshold decision

**1,010 > 400.** Per the plan's Phase 9 point 2, the exhaustive
repointable/not-repointable mapping narrows to the three suites named in the
AC — `test-migrate-0007.sh`, `test-migrate-interactive.sh`,
`scripts/test-interactive-protocol.sh` — with 0167's remainder-only pattern
for the rest (`test-migrate.sh`, `test-migrate-snapshot.sh`,
`hooks/test-migrate-discoverability.sh`: repointed at the compiled binary in
place, per Phase 9 point 4, without an exhaustive per-assertion table).

## Per-suite counts

| Suite | Total | Repointable | Not-repointable |
|---|---|---|---|
| `skills/config/migrate/scripts/test-migrate.sh` | 522 | 522 | 0 |
| `skills/config/migrate/scripts/test-migrate-snapshot.sh` | 2 | 2 | 0 |
| `skills/config/migrate/scripts/test-migrate-interactive.sh` | 265 | 265 | 0 |
| `skills/config/migrate/scripts/test-migrate-0007.sh` | 195 | 195 | 0 |
| `scripts/test-interactive-protocol.sh` | 12 | 0 | 12 |
| `hooks/test-migrate-discoverability.sh` | 14 | 14 | 0 |
| **Total** | **1,010** | **998** | **12** |

## Classification method, and its known limitation

`migrate_suite_inventory.py` classifies each assertion by scanning its own
logical statement (backslash-continuation lines joined) for a marker naming
a wire-protocol internal (`$FRAME_TYPE`, `harness_*`, `mkfifo`, ...);
`scripts/test-interactive-protocol.sh` is forced `not-repointable`
categorically, since it drives `interactive-protocol.sh` in-process and has
no CLI surface at all.

**This finds zero not-repointable assertions in `test-migrate-interactive.sh`,
which is a real limitation of the heuristic, not evidence the suite is
trivially repointable.** Reading the suite directly: it is not organised
into `test_*` functions but into sequential `echo "=== Phase N: ... ==="`
sections, each of which typically heredocs a **synthetic bash migration
script** to disk (using `harness_emit_transformation`/`migration_apply_decision`/
`harness_reject` — the author-facing callback contract, not a real numbered
migration) and then drives it through the real `run-migrations.sh`, asserting
on the captured stdout/stderr/session-log file. The *assertions themselves*
mostly reference only captured-output variables (hence "repointable" by this
tool's own literal text-marker rule), but the *test as a whole* has no Rust
equivalent to repoint at: the compiled `accelerator-migrate` registry is
compiled-in (Phase 2), so there is no way to inject a synthetic migration
into it the way bash injects a heredoc'd script. Every one of these tests'
real Rust equivalent already exists as a `FixtureMigration`-driven test in
`cli/migrate/tests/engine.rs`/`list_and_decisions_file.rs` (the established
pattern since Phase 5) — not as a black-box rewrite of the bash assertion
itself.

**Practical implication for Phase 9 point 3 (black-box rewrites):**
`test-migrate-interactive.sh`'s ~265 assertions do not decompose into a
short "not-repointable" list this tool can hand off as a black-box-rewrite
work list. Confirming its behaviour is already covered requires reading each
of its ~13 phase sections against the equivalent `FixtureMigration`
coverage already landed in Phase 5, not running this extractor again with a
smarter marker list — a materially larger, judgement-driven task than the
mechanical classification this tool performs for
`scripts/test-interactive-protocol.sh` (whose 12 sites this tool's
file-level rule classifies correctly and completely). This gap is
disclosed here rather than papered over with a wider marker list that would
only produce a false sense of completeness.
