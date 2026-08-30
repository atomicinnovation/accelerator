---
type: "plan-validation"
id: "2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path-validation"
title: "Validation Report: Remove the Exec Probe from the Bootstrap Warm Path"
date: "2026-08-03T12:23:32+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "work-item:0186"
target: "plan:2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path"
tags: ["shell", "performance", "bootstrap", "bash-3.2", "testing"]
last_updated: "2026-08-03T15:00:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Remove the Exec Probe from the Bootstrap Warm Path

### Implementation Status

✓ Phase 1: Root-Privilege Guard — fully implemented (`oqzoztqs`)
✓ Phase 2: Split the Probe and Relocate It Off the Warm Path — fully
implemented (`rpzsskwo`)
✓ Phase 3: Documentation — fully implemented (`oqkknkry`)
✓ Phase 4: Measurement and Closeout — fully implemented (`vwkmolqw`), with
the cross-lane observation discharged by CI run 30821400291

Every code, test and documentation change the plan specifies is present and
committed, and every success criterion in the plan is now ticked. The
cross-lane CI observation — the one item that could not be discharged locally
when this report was first written — was confirmed on 2026-08-03.

**Amended 2026-08-03** after the CI observation landed. The report below is
otherwise as originally written; the result moves from `partial` to `pass`.

### Automated Verification Results

All run locally on darwin-arm64 at revision `dec6502d`:

✓ `mise run test:integration:entrypoint` — 54 passed in 32.87s (matches the
plan's expected count; all eight new cases named and passing)
✓ `mise run test:integration:skill-invocation` — 128 passed in 66.07s (matches
the plan's expected count; the shared `run_bootstrap` consumer is unaffected)
✓ `uv run pytest tests/unit/tasks/test_bootstrap_coverage.py
tests/unit/tasks/test_mise.py` — 33 passed, including the new
`test_the_cache_dir_helpers_keep_the_names_the_traces_assert_on`
✓ `mise run build-system:check` — format, lint and types clean (0 errors)
✓ `mise run scripts:check` — shfmt, ShellCheck, bashisms and exec-bits clean
over the edited `bin/accelerator`
✓ `mise run check` — green end to end (all four components)
✓ `mise run test:integration:config` — 58 passed, 0 failed; the corpus
frontmatter validator accepts the amended 0186/0169 items and the three new
follow-ups

Not re-run here: the bare `mise run` default task. Its constituent read-only
gate (`mise run check`) and all three affected test suites are green, and the
plan records it as observed green during Phase 4.

**CI, run
[30821400291](https://github.com/atomicinnovation/accelerator/actions/runs/30821400291)
(2026-08-03, head `f0fdeeb2`) — all 16 jobs green:**

✓ `Run integration tests (macos-latest)` — entrypoint suite 54 passed (139.29s)
✓ `Run integration tests (ubuntu-latest)` — entrypoint suite 53 passed, 1
skipped (72.76s); all eight new cases green
✓ Both unit lanes, both E2E lanes, visual regression, and all seven `Check *`
jobs

The two trace cases are confirmed on both interpreters. The single ubuntu skip
is `test_the_suite_runs_the_bootstrap_on_the_bash_floor`, which is darwin-only
— and its passing on macos while skipping on ubuntu is itself the evidence the
lanes run genuinely different bash versions, so the coverage claim is observed
rather than inferred. `_require_unprivileged` did not fire on either lane, so
no exclusion was needed, as predicted.

An unrelated flake had to be fixed first: both mock HTTP servers installed
their SIGTERM handler *after* writing the url file that `start_mock` treats as
the readiness signal, so a test making no request at all (`--describe`,
`--print-payload`) could kill the process while SIGTERM still had its default
disposition — skipping the `finally` that writes the captured-urls file.
Reproduced 10/10 with the window widened, 0/10 with the ordering reversed.
Fixed in both `mock-jira-server.py` and `mock-linear-server.py`. Unrelated to
this plan, but it blocked the observation.

### Code Review Findings

#### Matches Plan

- **`bin/accelerator:169-191`** — `probe_dir` is split exactly as specified.
  `ensure_dir` carries the `[[ -d ]]` guard; `probe_exec_capable` returns 1 for
  a write/`chmod` failure and 2 for an exec failure, and removes the probe file
  on all three exit paths.
- **`bin/accelerator:193-204`** — `resolve_cache_dir` calls `ensure_dir` at both
  sites; the XDG comment is preserved verbatim.
- **`bin/accelerator:206-215`** — `fail_no_cache_dir` is defined above the
  `resolve_cache_dir` call site, holds the single definition of the asserted
  substring, keeps the `ACCELERATOR_CACHE_DIR` remediation hint, and the call
  site expands the override rather than hard-coding `${plugin_root}/bin`.
- **`bin/accelerator:253-267`** — `require_exec_capable_cache` sits below the
  dev-override block, `probed` is set only on a successful probe, and the three
  causes map to the planned messages.
- **Both gates are in the planned positions**: the staging gate is the first
  statement of the shim-staging `if` body (`:296-298`), the cold-branch gate is
  the first statement of the `else` (`:381-386`).
- **`tests/integration/support/installation.py`** — keyword-only `xtrace: bool
  = False`, `PS4` defaulted alongside it, `bash_flags` spliced into the
  `subprocess.run` argv, and the docstring extended with the three rationales
  the plan names (per-call tracing, the `:-main` default, the `SHELLOPTS`
  rejection). The `False` default preserves the shared consumer's behaviour.
- **All eight regression cases** are present with the planned names, assertions
  and comments, under the `# ── Exec probe: cold-path only ──` divider with the
  helpers directly beneath it.
- **`_require_unprivileged`** is retrofitted onto exactly the three existing
  cases the plan names (`:274`, `:298`, `:1086`) and used by six of the eight
  new ones.
- **Documentation** — `docs/internals.md:205-223` carries both paragraphs with
  the release-base-URL sentence intact and the trust guidance unnarrowed; the
  bootstrap header gains the cold/warm asymmetry clause (`:13-15`); the staging
  comment cites the three test function names instead of line ranges
  (`:280-290`); the CHANGELOG entry sits under `## [Unreleased]` / `### Changed`
  and states no unmeasured figure.
- **Phase 4's record is unusually complete** — both medians with min/p90, both
  instrument floors, host and OS build, launcher provenance confirmed by inode,
  a measured composition table with the unexplained remainder at ~10%, the
  resolved sha256 backend named with its fallback range, and the probe
  attribution re-derived (including the security-agent check that rules out a
  host artefact). All three follow-ups (0189, 0190, 0191) exist and
  cross-reference 0186; 0169's hand-off note is re-confirmed with its threshold
  untouched.

#### Deviations from Plan

All are recorded in the plan and the work item at the point of deviation, with
reasoning — none is silent.

- **`_restricted`'s mode check** — the plan's single `not os.access(path,
  os.W_OK)` assertion is wrong for the two `0o666` cases (which keep the owner
  write bit and clear only search). Implemented as a loop over both owner bits,
  checking each probe only when the mode clears the corresponding bit. This is a
  correction, not a weakening: it still fails loudly on an advisory-permission
  filesystem.
- **The staging comment's byte figure** — the plan instructed correcting `475KB`
  to `465KB`; the implementation established that 465,568 B is the *linux-x64*
  shim and that `475KB` was already right for darwin-arm64 (486,672 B). Written
  as `~475KB`, which is the correct generic form. Verified: the comment at
  `bin/accelerator:283` reads `~475KB`.
- **PS4** — `'+${FUNCNAME[0]:-main}:'` rather than the acceptance criterion's
  literal `'+${FUNCNAME[0]}:'`, which is broken under `set -u`. Anticipated by
  the plan and recorded in the work item.
- **Criteria 1, 3 and 9** — the version assertion became stdout equality against
  a direct launcher run, the positive control got its own traced cold run, and
  the measurement method became 50 interleaved single-process samples. All three
  were specified by the plan and are recorded as deviations in the work item.
- **Work item status** — Phase 4 change 6 says "move `status` to `complete`",
  but `complete` is not in the schema's status vocabulary
  (`draft | ready | in-progress | review | done | blocked | abandoned`), as
  `scripts/validate-corpus-frontmatter.sh` reports. The item was correctly held
  at `in-progress` pending the cross-lane observation, with a note at the top
  explaining why; on 2026-08-03, once run 30821400291 discharged that
  criterion, it moved to **`done`** — the terminal value the plan should have
  named.

#### Potential Issues

- ~~**The linux lane is entirely unobserved.**~~ **Resolved 2026-08-03.** Both
  trace cases pass on `ubuntu-latest` (bash 5.2) and `macos-latest`
  (bash 3.2.57) in run 30821400291. No trace-format divergence exists between
  the interpreters; the `PS4` `:-main` default and the `^\++` matchers behave
  identically on both.
- **The exec-vs-write coverage gap remains open**, as the plan intends. No
  directory-permission combination produces exec-without-write, so
  `probe_exec_capable`'s `return 2` branch — and the `rejected an executable
  file` cause clause it emits — is exercised by no automated case. The exec
  *half* is covered by the positive control's execution assertion; the *cause
  clause* is not. This is recorded in both the plan and the work item and
  explicitly scoped out, but it means one of the three diagnostic causes ships
  unverified by test.
- **`probe_exec_capable` uses unscoped globals** (`probe`, `status`). Checked:
  neither name is used anywhere else in `bin/accelerator`, and the file declares
  no `local` at all, so this matches the established idiom rather than
  introducing a hazard.
- **Cold-path failure ordering changed** — a cold invocation destined to fail
  with the cache-dir diagnostic now hashes first (~7 ms) before the gate fires,
  where `resolve_cache_dir` previously failed before any hashing. Noted in the
  plan's Performance Considerations and in the work item; no test depends on the
  old ordering.

### Manual Testing Required

1. CI observation — **complete**:
  - [x] `test-integration` green on `ubuntu-latest`, specifically on
        `test_warm_path_does_not_enter_the_probe` and
        `test_cold_path_enters_and_executes_the_probe` (bash 5.2 coverage of
        the trace assertions) — run 30821400291, 2026-08-03
  - [x] No lane exclusion needed — `_require_unprivileged` did not fire on
        either runner
  - [ ] Record `command -v sha256sum` on both lanes, so 0169 learns whether the
        Perl `shasum` fallback is reachable in CI at all. **Not done** — it
        needs a deliberate step in the workflow and nothing in this plan
        required it; 0169 carries the backend as a range rather than a per-lane
        fact.

2. Optional, out of scope but worth noting if an opportunity arises:
  - [ ] Point `ACCELERATOR_CACHE_DIR` at a genuine `mount -o noexec` filesystem
        and confirm the cause clause reads `rejected an executable file` rather
        than `is not writable` — the one path no automated case can construct

### Recommendations

- **The plan is complete and 0186 is closed.** All success criteria are ticked,
  both lanes are observed, and PR #41 is green end to end. Nothing blocks merge.
- **Consider whether the `return 2` cause clause deserves a cheap seam** rather
  than staying untested indefinitely — for example an injectable probe command
  in a test-only env var. Not required by this plan, and the trade-off (a new
  seam in a trust-root script) may well favour leaving it; worth a sentence in
  0189, which will face the same gap on the launcher side.
- **Pick up `command -v sha256sum` on the CI lanes when 0169 needs it.** It is
  the one recording step this work left undone, and it is 0169's input rather
  than 0186's.
- **`main` is currently red** on `Run unit tests (macos-latest)`, unrelated to
  this work and predating it. Worth its own look — this branch is green on that
  same job, so whatever it is does not reproduce here.
