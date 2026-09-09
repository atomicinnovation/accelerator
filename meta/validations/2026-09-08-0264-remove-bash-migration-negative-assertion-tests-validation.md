---
type: "plan-validation"
id: "2026-09-08-0264-remove-bash-migration-negative-assertion-tests-validation"
title: "Validation Report: Remove Bash-Migration Negative-Assertion Tests Implementation Plan"
date: "2026-09-09T16:41:20+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-08-0264-remove-bash-migration-negative-assertion-tests"
tags: ["testing", "cleanup", "bash-parity", "migration", "validation"]
last_updated: "2026-09-09T16:41:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Remove Bash-Migration Negative-Assertion Tests

All six phases landed as discrete commits, every delete target is gone, every
rewire is present, and the read-only aggregate plus the two lanes that actually
exercise the changed guards are green. The result is a **pass**. Two manual
CI-log checks remain — both deferred by the plan itself as they require the PR
CI run, not a local machine.

### Implementation Status

Each phase is a named commit in the working-copy ancestry (`pymtnnov` →
`xqvyqkon`), and the working copy is clean.

✓ Phase 1: Remove the work-adapters zero-spawn test — fully implemented
✓ Phase 2: Remove the migrate-cli no_awk guard — fully implemented
✓ Phase 3: Excise `the_probe_shells_out_to_nothing` — fully implemented
✓ Phase 4a: Rewire the fixture-size guard into the roll-up — fully implemented
✓ Phase 4b: Retire the corpus-adapters zero-spawn cluster — fully implemented
✓ Phase 5: Remove the call-site-migration Python guard — fully implemented

### Automated Verification Results

✓ `mise run check` (read-only aggregate CI mirror): exit 0
✓ `mise run test:unit:tasks` (exercises `test_mise.py`): 2781 passed, exit 0
✓ `mise run build-system:check` (integration.py importable after import prune): exit 0
✓ Fixture-size chain resolves: `test:integration → test:integration:fixture-size → build:cli:fixture-size` present in `mise tasks deps`
✓ Both chain assertions ran green: `test_fixture_size_leaf_reaches_the_guard`, `test_fixture_size_guard_runs_in_the_integration_rollup`

Delete targets, all confirmed gone:

| Target | Phase | State |
| --- | --- | --- |
| `cli/work-adapters/tests/zero_spawn.rs` | 1 | 🟢 gone |
| `cli/migrate-cli/tests/no_awk.rs` | 2 | 🟢 gone |
| `the_probe_shells_out_to_nothing` in `start_time.rs` | 3 | 🟢 gone |
| `cli/corpus-adapters/tests/zero_spawn.rs` + fixture | 4b | 🟢 gone |
| `test:integration:zero-spawn*` mise tasks | 4b | 🟢 gone |
| `check-zero-spawn` CI job + `needs:` entry | 4b | 🟢 gone |
| `tasks/lint/call_site_migration.py` + test + wiring | 5 | 🟢 gone |

### Code Review Findings

#### Matches Plan:

- **Single-test excision is clean.** `start_time.rs` keeps exactly its three
  positive tests (`the_probe_agrees_across_locales`,
  `the_probe_agrees_across_timezones`, `the_probe_is_stable_for_a_live_process`)
  and the absence test is gone.
- **Fixture-size guard is rehomed, not retired.** The
  `test:integration:fixture-size` leaf exists (`mise.toml:387`), is a member of
  the `test:integration` roll-up (`:402`), and its sole `depends` edge reaches
  `build:cli:fixture-size` — proven both in the resolved graph and by the two
  pinning assertions in `test_mise.py`.
- **pup.ron comments name no deleted file.** Both
  `work_adapters_is_zero_spawn` (`cli/pup.ron:283-285`) and
  `vcs_adapters_is_zero_spawn` (`:323-325`) now describe the crate-wide
  use-path deny alone, with the deleted-file clause dropped as the plan specified.
- **corpus-adapters manifest is section-clean.** `bash-parity`,
  `corpus-adapters-fixture`, and `vcs-test-support` are all absent, and the
  crate compiles under the all-features CI configuration (proven by the `check`
  compile).
- **README rehome landed.** `tasks/README.md` now carries a standalone
  `### The gix/jj-lib link-ratio guard` subsection (`:218`); the old
  `### Zero-spawn strong form` machinery — `ACCELERATOR_ZERO_SPAWN_*`, the sudo
  shadow/restore, the `check-zero-spawn` table row — is gone.
- **call-site-migration wiring fully removed.** No `call_site_migration` /
  `call-site-migration` reference survives in `tasks/` or `mise.toml`.

#### Deviations from Plan:

- **Topology-guard test count is 2781, not the 2787/2789 the plan's Phase 4a/4b
  criteria recorded.** The delta is unrelated churn in other test lanes between
  the plan's authoring and now; the lane is green and both classification
  invariants hold, so this is drift in an incidental count, not a regression.

#### Potential Issues:

- **Coverage retired as designed, not by accident.** The plan's "Coverage
  Retired (Accepted)" section enumerates five regression classes no surviving
  guard catches (inline/transitive `std::process` in the three adapter crates,
  all zero-spawn coverage in corpus-adapters, subprocess in the start-time
  probe, `awk` shell-out in the migrate crates, config-cluster call sites).
  These are sign-off losses, reachable at runtime. No new guard was added to
  offset them — consistent with the plan, worth restating so the loss stays
  explicit rather than implied.

### Manual Testing Required:

1. CI job coverage (deferred — needs the PR CI run, not verifiable locally):
  - [ ] Phase 4a PR: `test-integration` runs `test:integration:fixture-size` →
    `build:cli:fixture-size` green on both the `ubuntu-latest` and
    `macos-latest` legs.
  - [ ] Phase 4b PR: after `check-zero-spawn` deletion, the same step still runs
    green on both legs — the guard's CI home survived the old lane's removal.

### Recommendations:

- **Land Phase 4a before Phase 4b as separate PRs.** The plan's one ordering
  constraint (4a establishes the guard's CI home before 4b deletes its only
  prior invocation). The commits are stacked in the correct order; keep that
  order across the PR boundary — 4b applied alone would leave
  `build:cli:fixture-size` a live task with no CI invocation, a silent gap the
  topology guard passes vacuously over.
- **On each PR, confirm the two deferred CI-log checks before merge.** They are
  the only verification a local run cannot cover.

### What was not checked

- **Full test suite / bare `mise run`.** I ran `check`, `test:unit:tasks`, and
  `build-system:check` — not the whole suite or the docs lane. The changed
  guards are covered by the lanes I did run.
- **Local re-execution of `test:integration:fixture-size`.** Confirmed graph
  reachability and the plan's recorded 32.6× ratio; did not re-run the
  release-profile build of the gix/jj-lib fixtures locally.
