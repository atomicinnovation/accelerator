---
type: plan-validation
id: "2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee-validation"
title: "Validation Report: At-Most-Once Cache-Root Probe Guarantee Implementation Plan"
date: "2026-08-12T13:43:58+00:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: "pass"
parent: "work-item:0189"
target: "plan:2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee"
tags: [cli, launcher, bootstrap]
last_updated: "2026-08-12T13:43:58+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: At-Most-Once Cache-Root Probe Guarantee

All four phases are implemented as specified and every automated gate is green,
including the bare `mise run`. One defect was found, in the plan's own recorded
evidence rather than in the code: the Mutation A cell for the warm-hit test is
wrong, and the observation the work item's criterion 6 demands was never taken.
It has now been taken, and it is green.

### Implementation Status

✓ Phase 1: The Probe Counter Seam — fully implemented (`c9e7181d1be1`)
✓ Phase 2: The At-Most-Once Invariant Tests — fully implemented (`c96ad43af161`)
✓ Phase 3: Remove the Second Probe Entry Point — fully implemented
  (`0e5365fbfbc1`)
✓ Phase 4: Correct the Stale `docs/internals.md` References — fully implemented
  (`b44c7e429ac7`)

One commit per phase, in plan order. Working copy clean at `b20d0507`.

### Automated Verification Results

| Gate | Command | Status |
| --- | --- | --- |
| Default task | `mise run` | 🟢 exit 0 |
| Aggregate check | `mise run check` | 🟢 exit 0 |
| Docs | `mise run docs:check` | 🟢 exit 0 |
| CLI unit + integration | `ACCELERATOR_COVERAGE=off mise run test:unit:cli` | 🟢 exit 0 |
| Resolution binary | `cargo nextest … -E 'binary(resolution)'` | 🟢 25/25, 0 skipped |
| Anchor suite | `uv run pytest tests/unit/tasks/test_docs_anchors.py` | 🟢 3 passed |
| Config integration | `mise run test:integration:config` | 🟢 on clean re-run |
| Hooks integration | `mise run test:integration:hooks` | 🟢 exit 0 |

✅ The eight delta-bearing tests were confirmed to execute rather than
skip — `minisign` 0.12 resolves via mise, and the 25-test run reports `0
skipped`, so the suite is not a `skip_if_no_minisign!` false negative.

⚠️ The first `test:integration:config` run failed with `rm: Directory not
empty` under the playwright temp dir while a cargo run held the machine
concurrently; a clean re-run passed. This matches the known flaky shell-suite
pattern and is unrelated to the plan.

### Code Review Findings

#### Matches Plan:

- `PROBE_ATTEMPTS` is a file-level `thread_local!` with the
  `const { Cell::new(0) }` initialiser the Performance section calls
  load-bearing, incremented as the first statement of `verify_writable`
  (`cli/launcher/src/launch/outbound/resolve/cache_root.rs:74-102`).
- `SEQUENCE` is untouched — still declared inside
  `probe_writable_and_executable`'s body and still incremented after the
  `create_dir_all` guard (`cache_root.rs:121-126`).
- `verify_writable` is `pub(super)` with one production call site,
  `fetch_verify_store` (`resolve/mod.rs:141`); `cache_root::resolve` is gone and
  the test-module import no longer names it. `probe_attempts` stays `pub`.
- Phase 1 left `verify_writable` `pub`; the narrowing lands only in the Phase 3
  commit, exactly as the phase boundary required.
- All eight delta assertions present: six new tests plus retrofits on
  `a_signature_read_io_error_propagates_the_refetch_error_verbatim` (with the
  load-bearing findability assertion) and
  `an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss`.
- `an_unwritable_cache_root_…` captures its delta around `resolve` and asserts
  **after** restoring `0o755`, never inside `probes_during`
  (`resolution.rs:687-701`) — the permissions-window rule is honoured.
- `seed_cache` returns `CachedBinary` and reuses the retained `sha`/`asset_sig`;
  no test re-queries `cache::find` for a value the helper returned. `#[track_caller]`
  on `seed_cache`, `clear_cache` and `probes_during`.
- `resolve_offline` replaces all three prior verbatim copies of the
  unreachable-server construction.
- Docs: `internals.md`, `CHANGELOG.md`, both module headers and both runtime
  pointers match the plan's verbatim text. The anchor guard is
  `tests/unit/tasks/test_docs_anchors.py`, Python per ADR-0048, collected by
  `test:unit:tasks`; no by-name `_REQUIRED_CONFIG_SUITES` entry applies, since
  that guards a bash suite renamed off the `test-*.sh` glob.
  `_EXPECTED_CONFIG_SUITES = 16` matches the real discovered count (16
  executable `test-*.sh` under `scripts/`, with the sourced-only
  `test-helpers.sh` correctly non-executable).
- Work item amendments all present, in the dated-retraction form the item uses.

#### Deviations from Plan:

- ❌ **The recorded Mutation A cell for the warm-hit test is wrong.** The
  Validation Results table marks `a_warm_hit_never_probes_the_cache_root` as ✗
  under Mutation A. That contradicts the plan's own prediction (green, delta 0)
  *and* its own footnote, which states the warm-hit test "did not yet exist when
  A was in force; it was authored under B" — 6 failed / 18 passed over a
  24-test binary. The prose claim "every predicted cell was observed as
  predicted" is therefore not supported for that cell. Independently rerun
  during this validation: under Mutation A the warm-hit test **passes**, and the
  binary reports 6 failed / 19 passed over 25 tests.
- The re-homed override test uses a path beneath a temp dir rather than the
  literal `/some-override-dir` the plan specified. The plan records this
  deviation and its reason — the literal path left the assertion unfalsifiable
  on a non-root host — and the change is an improvement.
- `probe_attempts`' doc comment carries an intra-doc link `[`verify_writable`]`
  to a now-`pub(super)` item (`cache_root.rs:78`). The plan's own spec text used
  a plain code span, and Phase 3 item 1 demoted precisely this shape at
  `cache_root.rs:41` "unconditionally, not if rustdoc objects". Nothing fires —
  no rustdoc runs in CI — but it is the same stale-public-doc-link class the
  phase set out to remove.

#### Potential Issues:

- Work item criterion 6 requires that under a duplicated `verify_writable` in
  `fetch_verify_store`, the cold-miss, both refetch and two-resolution criteria
  go red with the cold-miss delta observed as exactly 2, **while the warm-hit
  criterion stays green**. Because the warm-hit test was authored under Mutation
  B, that last clause was never observed. It is discharged here: cold-miss
  failed with `left: 2, right: 1` at `resolution.rs:594` — `#[track_caller]`
  naming the test's own line, not the helper's — and warm-hit passed.
- `candidate_performs_no_filesystem_write_or_process_spawn` still uses the
  literal `/nonexistent-acc-parent-dir` and carries the unfalsifiability the
  plan flagged for the override test. The plan declares it out of scope.
- The guard's stated blind spot is real and unexercised: a probe *added* on
  another thread is invisible to all eight assertions. The plan records this
  rather than implying coverage.
- `mise run docs:check` prints a moderate `dompurify` npm advisory. Pre-existing,
  non-failing, unrelated to this plan.

### Manual Testing Required:

1. Root-privileged fixture check (partially discharged):
  - [ ] Run the assembled `cache_root` test binary as root on linux. The plan
        reproduced the temp-dir arrangement as uid 0 in `rust:1.90-slim` and
        recorded the deviation honestly, but the test binary itself was never
        run as root.

2. Deferred to the sibling plan:
  - [ ] Warm-call latency G and shell baseline B from one darwin host, with
        `G ≤ 1.1 × B`. This plan explicitly excludes it;
        `meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md` owns
        it, so work item 0189 cannot close on this plan alone.

### Recommendations:

- Correct the Mutation A / warm-hit cell in the plan's Validation Results to
  green, with the observation recorded above. Leaving a known-false cell in the
  evidence record is worse than the gap it was papering over, and the
  observation now exists.
- Demote the `[`verify_writable`]` intra-doc link on `probe_attempts` to a plain
  code span, matching the plan's own spec text and Phase 3's treatment of
  `cache_root.rs:41`.
- Close work item 0189 only after the sibling measurement plan lands; this plan
  discharges every criterion except the latency pair.
