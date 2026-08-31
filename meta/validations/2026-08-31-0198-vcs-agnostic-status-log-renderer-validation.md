---
type: "plan-validation"
id: "2026-08-31-0198-vcs-agnostic-status-log-renderer-validation"
title: "Validation Report: VCS-agnostic status/log renderer"
date: "2026-08-31T22:54:24+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-08-31-0198-vcs-agnostic-status-log-renderer"
tags: ["rust", "vcs", "cli", "gix", "jj-lib", "status", "log"]
last_updated: "2026-08-31T22:54:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: VCS-agnostic status/log renderer

Result: **pass**. All three phases are fully implemented across commits
`mxsyssqw` (phase 1), `rqlpyonu` (phase 2), and `kuvktpup` (phase 3). Every
automated success criterion is green locally; the sole unchecked item is a
Linux-only CI lane that cannot run under macOS.

### Implementation Status

- ✓ Phase 1 — renderer, both backends, conflict: fully implemented
- ✓ Phase 2 — cross-backend parity and content goldens: fully implemented
- ✓ Phase 3 — delete subprocess, extend zero-spawn: fully implemented

`cli/vcs-adapters/src/subprocess.rs` is deleted (369 lines removed). The neutral
model + pure renderer (`cli/vcs/src/status.rs`, `log.rs`), the `VcsReporter`
port returning `kernel::Error` (`cli/vcs/src/lib.rs:90`), both library adapters
(`cli/vcs-adapters/src/library/status_log.rs`, `snapshot.rs`), the never-fail
boundary helper (`cli/vcs-cli/src/report.rs`), the shared state builder
(`cli/vcs-test-support/src/status_log.rs`), and the two golden/parity harnesses
all exist and match the plan.

### Automated Verification Results

| Check | Command | Status |
|---|---|---|
| Renderer unit tests | `nextest -p vcs` (52) | 🟢 pass |
| Full cli workspace (all-features) | `nextest --workspace` (2783) | 🟢 pass |
| Architecture rules | `pup:check` | 🟢 pass |
| Pup probe pairs | `test:integration:pup` (65) | 🟢 pass |
| Feature graph | `test:integration:deny` (111) | 🟢 pass |
| Licence closure | `deny:check` | 🟢 pass |
| Public-API baseline | `public-api:check` | 🟢 pass |
| Local zero-spawn | `test:integration:zero-spawn` (3) | 🟢 pass |
| jj-settings guard | `lint:vcs-settings:check` | 🟢 pass |
| CLI component | `cli:check` (rustfmt + clippy) | 🟢 pass |
| Build-system component | `build-system:check` | 🟢 pass |

✅ Verified by running each command. The full workspace run (752s) exercised the
goldens, the git-vs-jj parity harness with its unmasked control, the never-fail
`Err`/panic/malformed-`ACCELERATOR_LOG` boundary tests, the release-profile
`abort` manifest guard, and the AC6 `D2`-forced-failure token test.

⚠️ The plan's documented per-package command
(`nextest -p vcs-cli --features bash-parity`) does not run: cargo rejects
per-package features against the workspace manifest ("cannot specify features
for packages outside of workspace"). The runnable path is the whole-workspace
`--all-features` invocation that `tasks/test/cli.py` and CI use. This is a stale
command string in the plan's success-criteria prose, not an implementation
defect — the underlying suites pass under the correct invocation.

`test:unit:tasks` reports 2738 passed, 3 failed — but all three failures are in
`tests/unit/tasks/test_vendor_assemble.py` (a `node --version` smoke check
timing out after 30s in the sandbox), unrelated to this plan. The vcs_settings
paired-exemption tests are among the 2738 passed.

### Code Review Findings

#### Matches Plan

- The `VcsReporter` port returns `kernel::Error` via the existing `From` seam
  (`.map_err(Into::into)`); no new public error type.
- `adapter_token` returns `jj-lib` for `Jj`, else `gix` (`report.rs:30`); the
  `warn!` tags it on an `adapter =` field, distinct from the library adapter's
  `vcs =` field.
- `catch_unwind(AssertUnwindSafe(…))` folds cleanly-unwinding panics to the
  `(status|log unavailable)` fallback; the release-profile manifest guard
  (`report.rs:253-270`) pins `panic != "abort"`.
- Conflict is read correctly per backend: git counts the `Summary::Conflict`
  status item in `N`; jj unions `MergedTree::conflicts()` in, so `conflict-jj`
  renders `1 changed, 1 conflicted` with no other change lines.
- The jj snapshot path is extracted to `snapshot.rs` as `working_copy_diff`;
  `dirty_paths.rs` is re-pointed at it, the `_EXEMPT` entry moved to
  `snapshot.rs`, and `Error::JjDirtyPaths` renamed to `JjWorkingCopyDiff`.
- Phase 3 widens the `std::process` deny crate-wide via the new
  `vcs_adapters_is_zero_spawn` rule (`pup.ron:330`) with its probe pair in
  `test_import_rule.py`.
- The Migration-Notes in-scope follow-up landed: `skills/vcs/commit/SKILL.md`
  now frames the injected status/log block in a `<repository-vcs-context>`
  delimiter with an untrusted-data warning.

#### Deviations from Plan

- None affecting behaviour. The one documentation deviation is the stale
  `--features bash-parity` command form noted above.

#### Potential Issues

- ⚠️ The accepted fault-isolation regression stands as designed: an in-process
  wall-clock hang or unbounded read on a hostile/pathological repo is not
  caught (only cleanly-unwinding panics are). This is a recorded, priced
  acceptance for single-shot `/commit` callers, not a defect — reopened only by
  a future hang complaint.
- ⚠️ Status now honours the user's global git config (`core.excludesFile`,
  `status.showUntrackedFiles`, `core.ignorecase`), a deliberate behaviour change
  pinned by the extended `scrub.rs` characterisation test.

### Manual Testing Required

1. Orientation text:
   - [x] `clean-git`, `dirty-git`, `conflict-{git,jj}`, `rename-git`,
         `deleted-git`, `unborn-git`, `sha256-git`, `no-repo` goldens spot-read
         as valid ADR-0066 output.
   - [ ] Run `accelerator vcs status`/`log` in a live dirty git and jj checkout
         and confirm useful `/commit` orientation (spot-checked via goldens; a
         live run in a real working copy is still worth a glance).

2. CI-only lane:
   - [ ] 🔴 Strong-form zero-spawn on the `check-zero-spawn` CI job (Linux).
         Cannot run under macOS SIP; the local path-only lane passes. This is
         the plan's own single remaining unchecked box and gates final sign-off
         on the CI run, not on local work.

### Recommendations

- Merge-ready locally. Push and confirm the `check-zero-spawn` CI job is green
  on Linux before final sign-off — it is the only verification the local macOS
  environment cannot cover.
- Correct the stale `-p vcs-cli --features bash-parity` command in the plan's
  success-criteria prose (or leave it, as the plan is now `done`) — future
  readers copying it will hit the cargo rejection.
