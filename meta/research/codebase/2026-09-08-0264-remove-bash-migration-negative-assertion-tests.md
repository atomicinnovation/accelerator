---
type: "codebase-research"
id: "2026-09-08-0264-remove-bash-migration-negative-assertion-tests"
title: "Research: Remove Bash-Migration Negative-Assertion Tests (0264)"
date: "2026-09-08T21:27:06+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0264"
parent: "work-item:0264"
topic: "Remove Bash-Migration Negative-Assertion Tests (0264)"
tags: ["research", "codebase", "bash-parity", "cargo-pup", "lint", "migration", "cleanup"]
revision: "fe3a860c450fd1cb9d6fe18fef9d46b5e6b35585"
repository: "accelerator"
last_updated: "2026-09-08T21:27:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Remove Bash-Migration Negative-Assertion Tests (0264)

**Date**: 2026-09-08T21:27:06+00:00
**Author**: Toby Clemson
**Git Commit**: fe3a860c450fd1cb9d6fe18fef9d46b5e6b35585
**Branch**: HEAD (detached; jj workspace `visualisation-system`)
**Repository**: accelerator

## Research Question

Verify the removal set, preserve set, and dependencies of work item 0264
against the live codebase, so an implementer can execute the deletion with no
dangling wiring, no accidentally-orphaned build surface, and confidence that
every dependency claim holds.

## Summary

The removal targets all exist as described and the four whole-file/partial
deletions are individually clean, but the work item under-specifies the blast
radius in three places that will each break `mise run check` or leave dead
surface if implemented literally.

- ⚠️ **Missing `mise.toml` wiring.** The work item lists only the three
  `tasks/` wiring lines for `call_site_migration.py`. The guard is also wired
  in `mise.toml`: a dedicated task block at `579-582` and an entry in the
  aggregate `lint:check` `depends` array at `653`. Both must go or the suite
  fails on a missing task.
- ⚠️ **Orphaned build surface in `corpus-adapters`.** Deleting
  `corpus-adapters/tests/zero_spawn.rs` orphans `[[bin]] corpus-adapters-fixture`
  and `tests/fixtures/corpus_adapters_fixture.rs` (both exist solely for it),
  and removes the crate's *only* `bash-parity` consumer — leaving the feature
  dead in that one crate.
- ❌ **The "four `#[cfg(feature = "bash-parity")]` tests" framing is
  inaccurate.** Only two of the four Rust removals are `bash-parity`-gated
  (`corpus-adapters` and `work-adapters` `zero_spawn.rs`). `no_awk.rs` and the
  `start_time.rs` test have no cfg gate and run unconditionally; `no_awk.rs`
  documents itself as a *permanent* regression guard, not scaffolding.

Dependencies are clear: **0269 is `done`** (the work item's one "unconfirmed"
blocker is resolved), all named siblings are `done`, and ADR-0048 is `accepted`
and does establish Python as the guardrail-test language for non-Rust surfaces.
The only genuinely open item is the reviewer sign-off on removing the guard —
which, on the evidence below, should arguably extend to `no_awk.rs` too.

## Detailed Findings

### Removal set — Rust tests (`cli/`)

Four removals, each self-contained (no `[[test]]` registration, no
`tests/common/` helper module). Cfg gating is not uniform, contradicting the
Requirements wording.

| Target | Lines | `bash-parity` gate | Removal note |
|---|---|---|---|
| `cli/corpus-adapters/tests/zero_spawn.rs` | 271 | 🟢 whole-file `#![cfg]` | Orphans fixture bin + feature — see below |
| `cli/work-adapters/tests/zero_spawn.rs` | 85 | 🟢 whole-file `#![cfg]` | Clean; feature stays live via sibling test |
| `cli/migrate-cli/tests/no_awk.rs` | 52 | 🔴 none — unconditional | Self-documented permanent guard |
| `cli/design-adapters/tests/start_time.rs` (one test) | 128 | 🔴 none — unconditional | Excise `the_probe_shells_out_to_nothing` only |

- **`corpus-adapters/tests/zero_spawn.rs`** asserts a git/jj-scoped zero-spawn
  property (three tests, each pairing "no stub marker written" with "values
  unchanged"). Its third case drives `[[bin]] corpus-adapters-fixture`
  (`Cargo.toml:23-25`), whose source is `tests/fixtures/corpus_adapters_fixture.rs`.
  Nothing else references that bin. It is the crate's only `bash-parity`
  consumer, so the feature (`Cargo.toml:18`) and the `vcs-test-support` dev-dep
  go dead here on removal.
- **`work-adapters/tests/zero_spawn.rs`** writes tripwire `diff`/`sh`/`bash`
  scripts onto `PATH` and drives `render`/`render_dossier` in-process, asserting
  no marker is written. The crate's `bash-parity` feature also gates
  `sync_working_copy_status.rs`, so leave the feature in place here.
- **`migrate-cli/tests/no_awk.rs`** is a static source scan (no runtime spawn):
  for crates `migrate`, `migrate-adapters`, `migrate-cli` it asserts no `.awk`
  file exists and no source contains the substring `awk`. Its own docs call it
  a permanent regression guard. Fully self-contained.
- **`design-adapters/tests/start_time.rs`** — remove only
  `the_probe_shells_out_to_nothing` (a static `include_str!` scan of
  `process-probe/src/lib.rs` for `getconf`/`Command::new`/`process::Command`).
  Deleting lines `91-117` (test plus its doc-comment) leaves the three positive
  tests, the `own_pid`/`observe`/`with_env` helpers, all imports, and the
  `TestError` alias intact and compiling. It shares no helper with the positive
  tests.

⚠️ **Stale references after removal.** `cli/pup.ron:286` and `:327` are
comments naming the two `zero_spawn.rs` files as the runtime complement to the
still-active `work_adapters_is_zero_spawn` / `vcs_adapters_is_zero_spawn` pup
rules. The rules keep working; the comments become stale and should be updated.

### Removal set — Python guard (`tasks/`) and its full wiring

`tasks/lint/call_site_migration.py` (83 lines) is an active anti-regression
gate: `grep_b_hits` flags any `skills/**/SKILL.md` line containing
`scripts/config-`; `stray_legacy_flag` flags `--allow-legacy-layout` outside an
allowlist; the `@task check` raises `invoke.Exit(code=1)` on any hit. Its
docstring cites ADR-0048. Its test `tests/unit/tasks/test_call_site_migration.py`
(67 lines, six tests) covers both guards plus a real-tree clean assertion.

Complete wiring — the three named lines plus **two the work item omits**:

| Reference | Location | Named in work item? |
|---|---|---|
| `from . import (... call_site_migration ...)` | `tasks/lint/__init__.py:4` | 🟢 yes |
| `__all__` entry | `tasks/lint/__init__.py:22` | 🟢 yes |
| `Collection.from_module(...)` | `tasks/__init__.py:128-130` | 🟡 partial (`:129` cited; the call spans 128-130) |
| Dedicated task block `[tasks."lint:call-site-migration:check"]` | `mise.toml:579-582` | 🔴 **no** |
| `lint:check` aggregate `depends` entry | `mise.toml:653` | 🔴 **no** |

⚠️ Miss the `mise.toml` entries and `mise run lint:check` (hence
`mise run check`) fails on a missing task. All other `call_site_migration`
matches are in `meta/` prose — no further executable wiring. `check-call-site-migration.sh`
is only a defensive allowlist string; no such file exists.

### Preserve set — survival inventory and out-of-scope guards

The positive-parity survival inventory (untruncated), none touched by the
removals:

- **`bash_parity_baseline.rs`** — `cli/work-adapters/tests/`.
- **`migration_000N.rs`** — `cli/migrate-cli/tests/migration_0001.rs` … `0008.rs`
  (contiguous, 8 files).
- **`*_exit_codes_parity.rs`** — `cli/{work-cli,jira-cli,linear-cli}/tests/exit_codes_parity.rs`.

The **`bash-parity` feature** is defined in six crates (`migrate-adapters`,
`corpus-adapters`, `work-adapters`, `vcs-adapters`, `migrate-cli`, `vcs-cli`)
and consumed by 17 gated test files. After removal, only **`corpus-adapters`**
loses its sole consumer; every other defining crate retains at least one gated
test, so the work item's "leave the feature in place" holds everywhere except
there.

Out-of-scope guards, all confirmed present and untouched:

- `SURVIVING_SHELL_SOURCES` — `tasks/shared/sources.py:41`; test
  `TestSurvivingShellSources` in `tests/unit/tasks/shared/test_sources.py:99`.
- Bash-3.2 floor / bashisms lint — `tasks/lint/scripts.py`.
- cargo-pup "must not spawn" rules — `cli/pup.ron` (`work_adapters_is_zero_spawn:289`,
  `vcs_adapters_is_zero_spawn:330`, migrate-adapters no-spawn ~`402`); runner
  `tasks/pup.py`.
- Bare-invocation lint (kept) — `tasks/lint/bare_invocation.py` + test
  `tests/unit/tasks/test_bare_invocation.py`.

### Dependencies and rationale

- 🟢 **0269 is `done`.** Title: "Remove Bash Vocabulary And Redesign Exit-Code
  Classification In Jira And Linear Clients". Its "lands first" wording is
  intra-item delivery sequencing, not an unfinished state; it `relates_to` 0264
  and records that 0264 is a distinct CLI-wide cleanup. The work item's one
  unconfirmed blocker is resolved — Open Question 2 can close.
- 🟢 **All named siblings `done`**: 0174, 0211, 0212, 0245, 0269. Parent epic
  **0136 is `in-progress`** (frontmatter) though its body header reads "Ready" —
  a cosmetic discrepancy in 0136, not a blocker for 0264.
- 🟢 **ADR-0048 `accepted`** (`meta/decisions/ADR-0048-four-toolchain-split.md`).
  It states Python "is the test language for everything that *isn't* Rust … and
  carries guardrail tests for non-Rust components", confirming the guard's
  rationale and the language choice.

## Code References

- `cli/corpus-adapters/tests/zero_spawn.rs:15` — whole-file `#![cfg(feature = "bash-parity")]`; git/jj zero-spawn property.
- `cli/corpus-adapters/Cargo.toml:23-25` — `[[bin]] corpus-adapters-fixture`, orphaned on removal.
- `cli/work-adapters/tests/zero_spawn.rs:52-84` — `PATH` tripwire, in-process render.
- `cli/migrate-cli/tests/no_awk.rs:32-51` — unconditional static `awk` scan.
- `cli/design-adapters/tests/start_time.rs:91-117` — `the_probe_shells_out_to_nothing`; the only lines to delete.
- `cli/pup.ron:286,327` — comments naming the deleted `zero_spawn.rs` files (stale after removal).
- `tasks/lint/call_site_migration.py:1` — docstring citing ADR-0048.
- `tasks/lint/__init__.py:4,22` — import + `__all__` wiring.
- `tasks/__init__.py:128-130` — `Collection.from_module(lint.call_site_migration)`.
- `mise.toml:579-582` — dedicated task block (unlisted in work item).
- `mise.toml:653` — `lint:check` `depends` entry (unlisted in work item).
- `tasks/shared/sources.py:41` — `SURVIVING_SHELL_SOURCES` (preserved).

## Architecture Insights

- **Cfg gating is not a reliable proxy for "migration scaffolding".** Two of the
  four Rust absence tests run unconditionally, and `no_awk.rs` is self-described
  as permanent. The work item treats all four as dead `bash-parity` scaffolding;
  the code says two are standing guards. The reachability argument (shell surface
  reduced to `SURVIVING_SHELL_SOURCES`) still supports removing them, but the
  framing overstates how dead they are.
- **The guard removal debate applies twice.** `call_site_migration.py` and
  `no_awk.rs` are both active anti-regression guards, not scaffolding. The work
  item gates the former on reviewer sign-off (AC8, Open Question 1) but treats
  the latter as an obvious deletion. Consistency argues for extending the
  sign-off gate to `no_awk.rs`.
- **AC7's diff command is a valid superset check but misses goldens.**
  `git ls-files 'cli/**/*parity*.rs' 'cli/**/migration_*.rs'` captures the named
  survival set plus nine other `*parity*.rs` files — so any accidental parity or
  migration deletion surfaces. It does not cover `*_goldens.rs` files, which sit
  outside the glob; a golden deletion would not be caught by AC7 alone.

## Historical Context

- `meta/decisions/ADR-0048-four-toolchain-split.md` — accepted; establishes the
  Python-as-guardrail-language role the removed guard depends on.
- `meta/work/0136-migrate-shell-scripts-to-rust-cli.md` — parent epic
  (`in-progress`); the migration these tests scaffolded.
- `meta/work/0269-remove-bash-references-from-jira-linear-clients.md` — `done`;
  the sibling the work item flagged as unconfirmed.
- `meta/reviews/work/0264-remove-bash-migration-negative-assertion-tests-review-1.md`
  — the approving review (pass 2, verdict now APPROVE).

## Related Research

None found specific to this cleanup; this is the first codebase-research
document scoped to 0264.

## Open Questions

- ❓ **Extend the sign-off gate to `no_awk.rs`?** It is a permanent guard by its
  own docs, not scaffolding — the same objection AC8 raises for
  `call_site_migration.py`. Decide whether its removal needs the same reviewer
  sign-off.
- ❓ **Scope of the `corpus-adapters` cleanup.** Should removal also delete the
  orphaned `[[bin]] corpus-adapters-fixture` + `corpus_adapters_fixture.rs` and
  drop the now-dead `bash-parity` feature from `corpus-adapters/Cargo.toml`, or
  leave the feature defined-but-unused? The work item's "leave `bash-parity` in
  place" was written as a global instruction and does not address this crate.
- ❓ **Update `cli/pup.ron:286,327` comments** as part of this task, or leave the
  stale references?
