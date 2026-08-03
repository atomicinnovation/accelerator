---
type: plan-validation
id: "2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path-validation"
title: "Validation Report: Remove the Exec Probe from the Bootstrap Warm Path"
date: "2026-08-03T12:23:32+00:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: "partial"
parent: "work-item:0186"
target: "plan:2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path"
tags: [shell, performance, bootstrap, bash-3.2, testing]
last_updated: "2026-08-03T12:23:32+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Remove the Exec Probe from the Bootstrap Warm Path

### Implementation Status

✓ Phase 1: Root-Privilege Guard — fully implemented (`oqzoztqs`)
✓ Phase 2: Split the Probe and Relocate It Off the Warm Path — fully
implemented (`rpzsskwo`)
✓ Phase 3: Documentation — fully implemented (`oqkknkry`)
⚠️ Phase 4: Measurement and Closeout — implemented (`vwkmolqw`); one
criterion outstanding by construction (see below)

Every code, test and documentation change the plan specifies is present in the
working tree and committed. `jj status` is clean. The single unresolved item is
the cross-lane CI observation, which cannot be discharged locally: the change
sits on an unnamed change with no bookmark and no pushed branch, so CI has not
run against it at all.

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
- **Work item status** — Phase 4 change 6 says "move `status` to `complete`";
  it is `in-progress`, with an explicit note at the top of the item explaining
  that the item's own Drafting Notes make the cross-lane observation "a genuine
  closure condition, not a formality". This is the correct call, not a miss.

#### Potential Issues

- **The linux lane is entirely unobserved.** The two trace cases
  (`test_warm_path_does_not_enter_the_probe`,
  `test_cold_path_enters_and_executes_the_probe`) are the most
  environment-sensitive in the suite, and the plan's own reasoning is that the
  harness's pinned `/bin/bash` is 3.2.57 on darwin but 5.2 on `ubuntu-latest` —
  so a trace-format divergence would surface only in CI. The change is not on a
  bookmark and has not been pushed, so nothing has run there.
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

1. CI observation (the only genuinely blocking item):
  - [ ] Push the change and confirm the `test-integration` matrix is green on
        `ubuntu-latest`, specifically on `test_warm_path_does_not_enter_the_probe`
        and `test_cold_path_enters_and_executes_the_probe` (bash 5.2 coverage of
        the trace assertions)
  - [ ] Confirm no lane exclusion was needed — `_require_unprivileged` should
        not fire on either runner
  - [ ] Record `command -v sha256sum` on both lanes, so 0169 learns whether the
        Perl `shasum` fallback is reachable in CI at all

2. Optional, out of scope but worth noting if an opportunity arises:
  - [ ] Point `ACCELERATOR_CACHE_DIR` at a genuine `mount -o noexec` filesystem
        and confirm the cause clause reads `rejected an executable file` rather
        than `is not writable` — the one path no automated case can construct

### Recommendations

- **Push the branch and let CI discharge the last criterion**, then tick the
  two remaining boxes (Phase 2 manual item 4, Phase 4 manual item 5), fill the
  "Lanes observed green" entry in the work item's Validation Results, and move
  0186 to `complete`. Nothing else stands between the change and closure.
- **Leave the work item at `in-progress` until then.** The deliberate hold is
  correct and the note explaining it should stay until CI replaces it with an
  observation.
- **Consider whether the `return 2` cause clause deserves a cheap seam** rather
  than staying untested indefinitely — for example an injectable probe command
  in a test-only env var. Not required by this plan, and the trade-off (a new
  seam in a trust-root script) may well favour leaving it; worth a sentence in
  0189, which will face the same gap on the launcher side.
