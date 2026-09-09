---
type: "pr-description"
id: "110"
title: "[0264] Remove bash-migration negative-assertion tests"
date: "2026-09-09T20:23:48+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0264"
parent: "work-item:0264"
relates_to: ["work-item:0136"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/110"
pr_number: 110
tags: ["testing", "cleanup", "bash-parity", "migration"]
revision: "85c9df01c5ec4bc79f9c046beb491ac48839d36c"
repository: "accelerator"
last_updated: "2026-09-09T20:23:48+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0264] Remove bash-migration negative-assertion tests

## Summary

Removes the negative-assertion tests left over from the shell-to-Rust migration
(epic 0136) that assert the absence of behaviour no longer reachable, the
migration-only Python call-site guard and its wiring, and the dedicated
zero-spawn CI suite that existed solely to run one of them. The shell surface is
now the two files in `SURVIVING_SHELL_SOURCES`, so these absence assertions no
longer guard a reachable regression. The work is overwhelmingly deletion; the
one additive piece rehomes a kept build-linkage guard so its CI coverage
survives.

## Changes

- **Delete four negative-assertion tests.** `work-adapters/tests/zero_spawn.rs`,
  `migrate-cli/tests/no_awk.rs`, the `the_probe_shells_out_to_nothing` case in
  `design-adapters/tests/start_time.rs` (its three positive tests stay), and the
  `corpus-adapters` zero-spawn test with its fixture bin.
- **Retire the corpus-adapters zero-spawn CI suite.** Removes the crate's
  `bash-parity` feature, fixture `[[bin]]`, and `vcs-test-support` dev-dependency;
  the two `test:integration:zero-spawn*` mise tasks; the backing `integration.py`
  cluster; and the `check-zero-spawn` GitHub Actions job — a ~20-minute
  sudo-gated job removed from every push.
- **Rehome the fixture-size guard (the one additive change).** `build:cli:fixture-size`
  (the gix/jj-lib link-ratio floor) was only invoked by the deleted strong job,
  so it is wired into the `test:integration` roll-up through a new
  `test:integration:fixture-size` leaf, with two `test_mise.py` assertions pinning
  the roll-up → leaf → guard chain.
- **Remove the call-site-migration Python guard.** Deletes
  `tasks/lint/call_site_migration.py` and its test, and unwires it from the lint
  package, task collection, and `mise.toml`. The forward-looking
  `bare_invocation` lint is untouched.
- **Reword two stale `pup.ron` comments** that named the deleted zero-spawn files,
  and **rewrite the retired `tasks/README.md` section**, rehoming the surviving
  link-ratio guard into its own subsection.

## Context

- Work item: `meta/work/0264-remove-bash-migration-negative-assertion-tests.md`
- Plan: `meta/plans/2026-09-08-0264-remove-bash-migration-negative-assertion-tests.md`
- Validation: `meta/validations/2026-09-08-0264-remove-bash-migration-negative-assertion-tests-validation.md` (result: pass)
- Parent epic: 0136 (shell-to-Rust migration)

The plan's "Coverage Retired (Accepted)" section records the regression classes
no surviving guard catches (inline or transitive `std::process` use in the
adapter crates, all zero-spawn coverage in corpus-adapters, subprocess in the
start-time probe, `awk` shell-out in the migrate crates, and config-cluster call
sites). These are accepted losses under the owner's sign-off, not equivalences.

## Testing

- [x] Read-only aggregate green: `mise run check` (exit 0)
- [x] Topology guard green, including the two new chain assertions: `mise run test:unit:tasks` (2781 passed)
- [x] `integration.py` still importable after the import prune: `mise run build-system:check` (exit 0)
- [x] Fixture-size guard reachable in the resolved graph: `test:integration → test:integration:fixture-size → build:cli:fixture-size`
- [x] Positive parity/golden inventory unchanged: 29 files, identical at `@-` vs `@`
- [ ] CI: `test-integration` runs `test:integration:fixture-size` → `build:cli:fixture-size` green on both the ubuntu and macOS legs (requires this PR's CI run)

## Notes for Reviewers

- **The one additive change is the fixture-size rehome** — everything else is
  deletion. Focus review there: the `test:integration:fixture-size` leaf is a
  depends-only shim whose sole purpose is to keep the roll-up's guarded
  homogeneous-namespace invariant satisfied while the guard rejoins CI. Its
  rationale is recorded at the task site and in the `test_mise.py` reason string.
- **This ships all of 0264 as one PR**, not the six standalone PRs the plan
  outlined. The plan's only ordering constraint (4a before 4b) is preserved by
  commit order within the branch, so no intermediate tree drops the guard from CI.
- **Accepted coverage loss** is deliberate and owner-signed-off; see the plan's
  "Coverage Retired" section rather than treating the removed assertions as
  gaps to backfill.
- ⏱️ Net CI time falls: the host-native `build:cli:fixture-size` is cheap against
  the sudo-gated strong suite it replaces. The one local cost is a release-profile
  compile of the gix/jj-lib fixtures now sitting in the `mise run` gate (~27s warm).
