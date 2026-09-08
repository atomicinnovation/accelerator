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
last_updated: "2026-09-08T20:54:29+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Revised via review-work-item (review 1, pass 2): fixed AC7/AC8 verification locations, reworded Blocked-by around 0269, unified negative-assertion terminology, expanded cargo-pup."
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
which assert the absence of behaviour that is no longer relevant, together with
the associated migration-only lint guard `tasks/lint/call_site_migration.py`
and its task wiring.

## Context

These negative-assertion tests were scaffolding for the shell-to-Rust migration
(epic 0136), landed alongside the retirement work in 0174, 0211, 0212, 0245,
and 0269. With the shell surface reduced to the two files named in
`SURVIVING_SHELL_SOURCES`, they now assert the absence of behaviour that is no
longer reachable. Negative assertions carry disproportionate cost: they inflate
test runtime and demand maintenance without guarding a live regression.

## Requirements

- Remove the four Rust `#[cfg(feature = "bash-parity")]` absence tests: three
  whole files and one single test (paths in Technical Notes).
- Remove the call-site guard `tasks/lint/call_site_migration.py` — which guards
  against regressing to the "bash config cluster", the config-reading logic that
  once lived in shell and now lives in the `cli/` config crates — along with its
  test `tests/unit/tasks/test_call_site_migration.py` and its task wiring.
- Keep the bare-invocation lint `tasks/lint/bare_invocation.py` and its test —
  it guards new skills, not just the migration.
- Preserve, explicitly out of scope: the `SURVIVING_SHELL_SOURCES` invariant
  (`TestSurvivingShellSources`), the bash-3.2 floor lint, the cargo-pup "must
  not spawn" architecture rules, and every positive bash-parity/golden test.
- Leave the `bash-parity` cargo feature in place — positive parity tests still
  depend on it.

## Acceptance Criteria

- [ ] Each of these no longer exists: `cli/corpus-adapters/tests/zero_spawn.rs`,
      `cli/work-adapters/tests/zero_spawn.rs`, `cli/migrate-cli/tests/no_awk.rs`,
      `tests/unit/tasks/test_call_site_migration.py`, and the module
      `tasks/lint/call_site_migration.py`.
- [ ] `the_probe_shells_out_to_nothing` is removed from
      `cli/design-adapters/tests/start_time.rs` while that file's positive
      tests remain and pass.
- [ ] `tasks/lint/call_site_migration.py` is no longer referenced from
      `tasks/__init__.py` or `tasks/lint/__init__.py`.
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
- [ ] Removing the call-site guard `tasks/lint/call_site_migration.py` has
      explicit reviewer sign-off, recorded as an approval on the removing pull
      request (see Open Questions).

## Dependencies

- Blocked by: 0269 (pending confirmation it has landed; see Open Questions).
  These negative-assertion tests were introduced alongside the sibling
  retirement items 0174, 0211, 0212, 0245, and 0269, so this cleanup must follow
  them. All are believed landed except 0269, which is unconfirmed; once it is
  confirmed there is no outstanding blocker.
- Blocks: none known.

## Assumptions

- The shell surface is reduced to the two files named in
  `SURVIVING_SHELL_SOURCES` (confirmed), so the removed absence assertions no
  longer guard a reachable regression.

## Open Questions

- Reviewer sign-off to remove the call-site guard
  `tasks/lint/call_site_migration.py`. It is an active anti-regression guard
  against the bash config cluster, not dead scaffolding; a reviewer who wants
  to keep it should say so before this lands.
- Confirm sibling item 0269 has landed, so no test being removed here was
  introduced by work still in flight.

## Technical Notes

Removal set:

- `cli/corpus-adapters/tests/zero_spawn.rs` — whole file (git/jj never spawned).
- `cli/work-adapters/tests/zero_spawn.rs` — whole file (never shells out to
  `diff`/`sh`/`bash`).
- `cli/migrate-cli/tests/no_awk.rs` — whole file (no `awk` shell-out, no `.awk`).
- `cli/design-adapters/tests/start_time.rs` — remove only
  `the_probe_shells_out_to_nothing`; keep the file's positive tests.
- `tests/unit/tasks/test_call_site_migration.py` — whole file.
- `tasks/lint/call_site_migration.py` — whole module, plus its wiring at
  `tasks/__init__.py:129` and `tasks/lint/__init__.py` (lines 4 and 22).

The four Rust removals are gated behind `#[cfg(feature = "bash-parity")]`. The
`bash-parity` feature stays because positive parity/golden tests
(`bash_parity_baseline.rs`, `migration_000N.rs`, `*_exit_codes_parity.rs`)
still depend on it.

## Drafting Notes

- `tasks/lint/call_site_migration.py` is a Python guardrail — per ADR-0048,
  which makes Python the guardrail-test language for non-Rust surfaces — against
  regressing to the bash config cluster. Removing it is per explicit decision,
  gated on the sign-off in Open Questions.
- Kept `tasks/lint/bare_invocation.py` and its test out of scope because the
  guard is forward-looking (it stops new skills invoking a shell), not
  migration scaffolding.
- Positive bash-parity/golden tests and the `SURVIVING_SHELL_SOURCES`
  invariant were treated as out of scope per explicit direction.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
- Related: 0136, 0174, 0211, 0212, 0245, 0269
