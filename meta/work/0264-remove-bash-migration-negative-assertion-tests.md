---
type: "work-item"
id: "0264"
title: "Remove Bash-Migration Negative-Assertion Tests"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "task"
priority: "medium"
parent: "work-item:0136"
tags: ["testing", "cleanup"]
last_updated: "2026-09-08T21:27:06+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Enriched from codebase research: full corpus-adapters zero-spawn CI-suite retirement, mise.toml guard wiring, corrected cfg-gating, resolved 0269 dependency and guard sign-off (confirmed removed)."
schema_version: 1
external_id: "PP-794"
---

# 0264: Remove Bash-Migration Negative-Assertion Tests

**Kind**: Task
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Remove the negative-assertion tests left over from the Bash-to-Rust migration,
which assert the absence of behaviour that is no longer reachable, together with
the migration-only lint guard `tasks/lint/call_site_migration.py` and all its
wiring. Removing the `corpus-adapters` zero-spawn test also retires its
dedicated CI suite: the `bash-parity` feature, fixture bin, and dev-dependency
in that crate, the two `test:integration:zero-spawn*` mise tasks, the backing
`tasks/test/integration.py` cluster, and the `check-zero-spawn` job in
`.github/workflows/main.yml`.

## Context

These negative-assertion tests were scaffolding for the shell-to-Rust migration
(epic 0136), landed alongside the retirement work in 0174, 0211, 0212, 0245,
and 0269. With the shell surface reduced to the two files named in
`SURVIVING_SHELL_SOURCES`, they now assert the absence of behaviour that is no
longer reachable. Negative assertions carry disproportionate cost: they inflate
test runtime and demand maintenance without guarding a live regression.

## Requirements

- Remove four Rust absence tests: three whole files and one single test (paths
  in Technical Notes). Note the gating is not uniform — only the two
  `zero_spawn.rs` files carry `#[cfg(feature = "bash-parity")]`; `no_awk.rs` and
  the `start_time.rs` test run unconditionally.
- Retire the `corpus-adapters` zero-spawn suite in full. Removing
  `cli/corpus-adapters/tests/zero_spawn.rs` orphans a dedicated CI surface built
  solely for it: the `bash-parity` feature, the `corpus-adapters-fixture` bin and
  its source, and the `vcs-test-support` dev-dependency in that crate; the
  `test:integration:zero-spawn` and `:strong` mise tasks; the `zero_spawn`,
  `zero_spawn_strong`, and `_compile_zero_spawn_targets` tasks and their helpers
  in `tasks/test/integration.py`; and the `check-zero-spawn` CI job. Remove all
  of it. Do not touch the shared `vcs-adapters-fixture`/`-stub` bins — they back
  `vcs-adapters`'s own tests and `vcs-test-support`.
- Remove the call-site guard `tasks/lint/call_site_migration.py` — which guards
  against regressing to the "bash config cluster", the config-reading logic that
  once lived in shell and now lives in the `cli/` config crates — along with its
  test `tests/unit/tasks/test_call_site_migration.py` and its full wiring,
  including the `mise.toml` task and aggregate entry (paths in Technical Notes).
- Update the two `cli/pup.ron` comments that name the deleted `zero_spawn.rs`
  files so they no longer reference nonexistent paths.
- Keep the bare-invocation lint `tasks/lint/bare_invocation.py` and its test —
  it guards new skills, not just the migration.
- Preserve, explicitly out of scope: the `SURVIVING_SHELL_SOURCES` invariant
  (`TestSurvivingShellSources`), the bash-3.2 floor lint, the cargo-pup "must
  not spawn" architecture rules, and every positive bash-parity/golden test.
- Leave the `bash-parity` cargo feature in place in every crate except
  `corpus-adapters` — positive parity tests still depend on it elsewhere, but
  `corpus-adapters`'s only consumer is the zero-spawn test being removed.

## Acceptance Criteria

- [ ] Each of these no longer exists: `cli/corpus-adapters/tests/zero_spawn.rs`,
      `cli/work-adapters/tests/zero_spawn.rs`, `cli/migrate-cli/tests/no_awk.rs`,
      `tests/unit/tasks/test_call_site_migration.py`, and the module
      `tasks/lint/call_site_migration.py`.
- [ ] `the_probe_shells_out_to_nothing` is removed from
      `cli/design-adapters/tests/start_time.rs` while that file's positive
      tests remain and pass.
- [ ] `tasks/lint/call_site_migration.py` is no longer referenced from
      `tasks/__init__.py`, `tasks/lint/__init__.py`, or `mise.toml` (neither the
      `lint:call-site-migration:check` task nor the `lint:check` `depends` entry
      remains).
- [ ] The `corpus-adapters` zero-spawn suite is gone: the `bash-parity` feature,
      the `[[bin]] corpus-adapters-fixture` and `tests/fixtures/corpus_adapters_fixture.rs`,
      and the `vcs-test-support` dev-dependency are removed from
      `cli/corpus-adapters/`; the `test:integration:zero-spawn` and `:strong`
      tasks are gone from `mise.toml`; the `zero_spawn`/`zero_spawn_strong`/
      `_compile_zero_spawn_targets` tasks and their helpers are gone from
      `tasks/test/integration.py`; and the `check-zero-spawn` job (and its
      `needs:` reference) is gone from `.github/workflows/main.yml`.
- [ ] The shared `vcs-adapters-fixture` and `vcs-adapters-fixture-stub` bins and
      their `tasks/build.py` build entries still exist and pass.
- [ ] `cli/pup.ron` no longer names either deleted `zero_spawn.rs` file.
- [ ] `mise run check` and the full test suite pass after the removals.
- [ ] The bare-invocation lint (`tasks/lint/bare_invocation.py`) and its test
      still exist and pass.
- [ ] `TestSurvivingShellSources` still exists and passes.
- [ ] The positive `bash-parity`/golden test-file inventory — `bash_parity_baseline.rs`,
      the `migration_000N.rs` set, and every `*_exit_codes_parity.rs` — is
      unchanged by the removals: no positive parity or golden test file has been
      deleted or renamed. Verify by diffing the listing from
      `git ls-files 'cli/**/*parity*.rs' 'cli/**/migration_*.rs'` at the parent
      commit against the same command post-change, not by a green suite alone.

## Dependencies

- Blocked by: none. These negative-assertion tests were introduced alongside the
  sibling retirement items 0174, 0211, 0212, 0245, and 0269, so this cleanup must
  follow them; all five are confirmed `done` (0269 verified — see the research
  document in References), so there is no outstanding blocker.
- Blocks: none known.

## Assumptions

- The shell surface is reduced to the two files named in
  `SURVIVING_SHELL_SOURCES` (confirmed), so the removed absence assertions no
  longer guard a reachable regression.

## Technical Notes

Line references below were verified against commit
`fe3a860c450fd1cb9d6fe18fef9d46b5e6b35585`; treat them as a guide, not gospel.

Rust absence tests:

- `cli/migrate-cli/tests/no_awk.rs` — whole file. No cfg gate; runs
  unconditionally. Static source scan asserting no `awk` shell-out and no `.awk`
  file across the `migrate*` crates.
- `cli/design-adapters/tests/start_time.rs` — remove only
  `the_probe_shells_out_to_nothing` (lines 91-117, test plus its doc-comment).
  No cfg gate. Shares no helper with the file's three positive tests, which stay.
- `cli/work-adapters/tests/zero_spawn.rs` — whole file, gated
  `#[cfg(feature = "bash-parity")]`. Self-contained; the crate's `bash-parity`
  feature stays (its `sync_working_copy_status.rs` still uses it).

`corpus-adapters` zero-spawn suite — a coupled cluster, all removed:

- `cli/corpus-adapters/tests/zero_spawn.rs` — whole file, gated `bash-parity`.
- `cli/corpus-adapters/tests/fixtures/corpus_adapters_fixture.rs` — the fixture
  bin's source; only `zero_spawn.rs:211` consumes it.
- `cli/corpus-adapters/Cargo.toml` — remove the `[features] bash-parity`
  definition, the `[[bin]] corpus-adapters-fixture` block, and the
  `[dev-dependencies] vcs-test-support` edge (all three comments included); the
  dev-dep's only consumer is `zero_spawn.rs`.
- `tasks/test/integration.py` — remove the `zero_spawn` (101-120),
  `zero_spawn_strong` (123-188), and `_compile_zero_spawn_targets` (191-206)
  tasks, their helpers `_build_fixture_matrix`/`_build_status_log_states`/
  `_resolve_vcs_binaries`/`_restore_vcs_binaries`, the `_SHADOW_OPT_IN`/
  `_MATRIX_ROOT`/`_STATUS_LOG_ROOT` constants, and any imports left unused. The
  `-p vcs-adapters --bin vcs-adapters-fixture` builds here go with the tasks, but
  the bins themselves stay — they are shared (`tasks/build.py:60-61`,
  `vcs-adapters/tests/scrub.rs`, `user_name.rs`, `vcs-test-support/src/stubs.rs`).
- `mise.toml:349-357` — the `test:integration:zero-spawn` and `:strong` tasks.
- `.github/workflows/main.yml` — the `check-zero-spawn` job (355-399) and its
  entry in the aggregate `needs:` list (line 617).

Python call-site guard:

- `tasks/lint/call_site_migration.py` — whole module.
- `tests/unit/tasks/test_call_site_migration.py` — whole file.
- Wiring: `tasks/lint/__init__.py:4` (import) and `:22` (`__all__`);
  `tasks/__init__.py:128-130` (the `Collection.from_module(...)` call spans all
  three lines); `mise.toml:579-582` (the `lint:call-site-migration:check` task
  block) and `mise.toml:653` (its entry in the `lint:check` `depends` array).

`cli/pup.ron:286` and `:327` are comments naming the two deleted `zero_spawn.rs`
files; the pup rules themselves stay. Update the comments.

Leave the `bash-parity` feature defined in the other five crates — positive
parity/golden tests (`bash_parity_baseline.rs`, `migration_000N.rs`,
`*_exit_codes_parity.rs`) still depend on it.

## Drafting Notes

- `tasks/lint/call_site_migration.py` and `cli/migrate-cli/tests/no_awk.rs` are
  both active anti-regression guards, not dead scaffolding. `call_site_migration.py`
  is a Python guardrail (per ADR-0048, which makes Python the guardrail-test
  language for non-Rust surfaces) against regressing to the bash config cluster;
  `no_awk.rs` guards against reintroducing an `awk` shell-out in the now-pure-Rust
  `migrate*` crates. Both are removed with the owner's explicit confirmation that
  they are no longer needed — no separate reviewer sign-off is required.
- Kept `tasks/lint/bare_invocation.py` and its test out of scope because the
  guard is forward-looking (it stops new skills invoking a shell), not
  migration scaffolding.
- Positive bash-parity/golden tests and the `SURVIVING_SHELL_SOURCES`
  invariant were treated as out of scope per explicit direction.
- The `corpus-adapters` removal is larger than a one-file delete: the zero-spawn
  test anchors a dedicated CI suite (feature, fixture bin, dev-dep, two mise
  tasks, the `integration.py` cluster, and the `check-zero-spawn` job), all of
  which exist only to run it and all of which are retired here. Verified against
  the live tree; see the research document in References.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
- Research: `meta/research/codebase/2026-09-08-0264-remove-bash-migration-negative-assertion-tests.md`
- Related: 0136, 0174, 0211, 0212, 0245, 0269
