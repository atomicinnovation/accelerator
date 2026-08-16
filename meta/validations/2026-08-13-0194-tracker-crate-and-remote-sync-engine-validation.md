---
type: plan-validation
id: "2026-08-13-0194-tracker-crate-and-remote-sync-engine-validation"
title: "Validation Report: Tracker Crate and Remote Sync Engine Implementation Plan"
date: "2026-08-14T11:15:19+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: partial
target: "plan:2026-08-13-0194-tracker-crate-and-remote-sync-engine"
relates_to: ["plan-validation:2026-08-11-0204-remote-tracker-port-validation"]
tags: [rust, tracker, sync, work-items, bash-parity, nextest]
last_updated: "2026-08-14T11:15:19+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Tracker Crate and Remote Sync Engine Implementation Plan

### Implementation Status

✓ Phase 1: Boundaries and shared test apparatus - Fully implemented
✓ Phase 2: The pure state machine in `work` - Fully implemented
✓ Phase 3: Persistence and apply in `work-adapters` - Fully implemented
⚠️ Phase 4: `accelerator work sync` - Shipped, named test deliverables absent
⚠️ Phase 5: `--push` wiring onto `create` and `update` - Same

All five phases ship working code, and every box in the plan is ticked. The
shipped *behaviour* is complete; what is incomplete is the test matrix
Phases 4 and 5 specify. The plan records that as a deviation and attributes
it to `work-cli` being bin-only. That attribution is wrong for the layer
where most of those tests belong — see Finding 1, the material result of
this validation.

`jj log` shows eleven commits on `main..0194-remote-sync-engine`: research,
plan, two work-item corrections, five implementation phases, a lockfile
sync and a comment-policy sweep. Working copy clean and empty on top.

### Automated Verification Results

✓ `mise run check` — exits 0 across all four components
✓ `mise run test:unit:cli` — 1634 pass, exit 0
✓ `mise run test:integration:tracker-contract` — 3 pass with the gate open
✓ `mise run test:integration:work` — 21 pass
✓ `mise run test:integration:pup` — 45 pass
✓ `mise run test:integration:skill-invocation` — 128 pass
✓ `mise run test:unit:tasks` — 764 pass
✓ `mise run deny:check` — advisories, bans, licenses, sources ok
✓ `mise run public-api:check` — passes (runs inside `check`)
✓ `cargo nextest run --all-features -p work -p work-adapters` — 159 pass,
  including `sync_baseline_shellout_parity::every_case_matches_live_bash_right_now`
✗ `mise run` (bare default) — **exits 1**. See Finding 4; not branch-caused.

### Code Review Findings

#### Matches Plan:

Each of these was reproduced independently rather than taken from the
plan's own tick.

- **Contract filtering works exactly as specified.** `mise run test:unit:cli`
  runs 1634 tests, zero of them from a `contract` binary, while 5
  `tracker-test-support` lib tests do run — the by-binary-not-by-crate
  distinction the plan argues for, confirmed by counting.
- **The baseline corpus regenerates byte for byte.** Running
  `regenerate.sh` on the committed checkout left every `expected.json`
  hash-identical.
- **The corpus has teeth.** Mutating `digest::local`'s separator by one
  byte reddened both `sync_baseline_corpus` (`case-all-blank-body`) and
  `sync_baseline_shellout_parity`; restoring it returned both to green.
- **The bash golden loops report failures rather than swallowing them.**
  Breaking one `[LABEL]` row produced `FAIL: label synced` and `Failed: 1`,
  confirming the read-by-redirect over a pipe-to-`while` subshell.
- **Provider selection behaves as described.** With `work.integration:
  linear` the binary exits 72 naming the tracker as recognised but unwired;
  in a scratch repo with the key unset it exits 73 naming the key, the four
  recognised trackers and `/accelerator:configure`.
- **The write-bounds refusal precedes the preview early-return**, so
  `--preview` refuses an over-threshold plan as the plan requires, and
  returns with `finalised: false` before any write.
- **The public-api delta is exactly what was predicted** — `work::sync`,
  `CreateInputs::external_id`, and 18 lines of `tracker::` types the
  widened rule now exposes. No incidental surface.
- **Classify coverage grew rather than shrank.** The pre-branch bash
  classify block asserted 12 named scenarios; the shared table now carries
  14 bash-applicable rows plus 3 Rust-only ones, the latter being exactly
  the two unknown-stamp distinctions and the unreadable-mtime case bash
  cannot express.
- **Housekeeping criteria all hold**: `_EXPECTED_WORK_SUITES` is still 5,
  all 13 `work-item-*.sh` scripts are present, the pending-push marker is
  gitignored, and `grep -rn "accelerator work sync\|work create --push"
  skills/` returns nothing, so no SKILL touches the new binary.

#### Deviations from Plan:

The plan itself records four deviations (reduced Phase 3 corpus, no `[lib]`
target, simplified item reconstruction, `WorkingCopyStatus` shelling VCS
directly, `update --push` bypassing `ItemApplier::push`). Those are
accurately described and are not repeated here. Three further deviations
were found that the plan does not record:

- **A ticked criterion asserts behaviour the code does not have.** Phase 1
  Manual criterion 2 and Manual Testing Step 2 both say that with
  `ACCELERATOR_TRACKER_CONTRACT` unset the harness "skips rather than
  running". It does not skip — it fails. Running the contract profile with
  the variable unset yields `Error: NotOptedIn` and a red test. The
  implementation is the better choice and both the source and
  `tasks/README.md` say so explicitly ("errors, rather than skips"); the
  plan's criterion was simply never updated to match, and was then ticked.
- **`for_tracker_error` is dead and duplicated.**
  `work_cli::exit_codes::for_tracker_error` carries `#[allow(dead_code)]`
  while `create.rs:346` hand-rolls an identical
  `dispatch_code_for_tracker_error`. The taxonomy the module exists to
  centralise is therefore split in two.
- **`update.rs` duplicates `ItemApplier::push`'s tail with nothing pinning
  the two together.** Both do show → `remote_body`-or-`NotRead` →
  `digest::local` → `baseline.set`. The plan justifies the duplication
  convincingly (ordering), but no test asserts the two copies agree, so a
  future change to the baseline-writing recipe can silently diverge. They
  also hash different sources — `apply.rs` re-reads the file from disk,
  `update.rs` hashes the in-memory `rendered` string.

#### Potential Issues:

- **Finding 1 (material): `work_adapters::sync::run` has no test coverage
  at all, and the plan's deviation note states the opposite.** Phase 4's
  recorded deviation reassures that "`work_adapters::sync::run`/`fetch`
  themselves are tested directly in `work-adapters` (`sync_fetch.rs`,
  `sync_apply.rs`) … those tests hold". They do not test `run`.
  `sync_fetch.rs` exercises `fetch::gather`; `sync_apply.rs` exercises
  `ItemApplier`. Grepping the whole tree, the only caller of
  `work_adapters::sync::run::run` is production code in
  `work-cli/src/sync.rs:412`, and `run.rs` has no inline `#[cfg(test)]`
  module. Untested as a consequence: the `--max-pulls`/`--max-pushes`
  refusal (a safety feature with a documented blast-radius rationale, and
  `max_pulls` appears in no test file anywhere), the preview early-return
  that guarantees preview writes nothing, the blank-local-hash bookkeeping
  before `finalise`, and the per-item apply dispatch.

  The stated blocker does not apply here. `run` is `pub` in a library
  crate; `work-adapters` already carries `tracker-test-support` as a
  dev-dependency and `sync_apply.rs` already imports `work_adapters::sync::*`.
  The write-bounds boundary tests, the preview observables, the conflict
  loop and classification stability could be written today at the
  `work-adapters` boundary with `RecordingTracker`, needing no `[lib]`
  target on `work-cli`. The bin-only constraint is real for `run_sync` —
  arg parsing, exit-code mapping, report rendering — but not for the
  orchestration those deferred scenarios actually target.

- **Finding 2: the environment gate guards only `run_all`, so two of three
  contract tests run with it closed.** `tests/contract.rs` calls
  `unaccounted_id_is_indeterminate_not_absent` and
  `a_failing_read_is_retryable` directly rather than through `run_all`,
  which is where the `ACCELERATOR_TRACKER_CONTRACT` check lives. Running
  the contract profile with the variable unset executes both to completion
  — 2 passed, 1 failed. Harmless today, since the only subject is an
  in-process fake. From 0171 it is not: a real client implementing
  `ContractSubject` in the same shape would issue live `fetch_all` and
  `show` calls with the gate closed. The nextest binary filter still keeps
  all of it out of the default run, so this is defence-in-depth that is
  thinner than `tasks/README.md` claims ("a second, independent gate"),
  not an open hole.

- **Finding 3: `serde_json` numeric divergence remains uncovered**, as the
  plan's own Phase 3 note flags. Worth carrying into 0171 as an explicit
  criterion rather than a note, since that is where a live Jira payload
  first meets the recipe.

- **Finding 4: the bare `mise run` gate does not currently pass.** It exits
  1 on `test:unit:frontend`, reproducibly, three runs out of three
  including standalone. All 2536 frontend tests across 122 files pass; the
  failure is a single unhandled `Error: [vitest-worker]: Timeout calling
  "onTaskUpdate"`, an RPC timeout in the runner rather than an assertion.
  This branch cannot be the cause: it changes no file under
  `cli/visualiser/frontend`, no `package.json`, no vitest config and no
  frontend task (its only `mise.toml` edit adds the tracker-contract task).
  I did not run the suite at `main` to confirm the condition predates the
  branch, so attribution rests on that reasoning rather than on a
  comparison. Either way the plan's repeatedly-ticked "Whole tree green:
  `mise run`" does not hold on this machine today.

### Manual Testing Required:

1. Frontend gate:
  - [ ] Reproduce `test:unit:frontend` on `main` to confirm Finding 4
        predates this branch, or bisect if it does not
  - [ ] Confirm whether CI reproduces the vitest worker timeout or whether
        it is local to this machine
2. Remote behaviour (blocked until 0171 wires a client):
  - [ ] `work sync` against a live tracker — every path through
        `tracker.create`/`update`/`show` is unexercised end to end
  - [ ] `create --push` and `update --push` against a live tracker,
        including the terminal-failure-that-succeeded shape
  - [ ] The pending-push marker's crash-recovery path against a real
        interrupted create
3. Cutover rehearsal:
  - [ ] `/sync-work-items` still drives bash end to end (grep confirms no
        SKILL references the binary, but the flow was not exercised)

### Recommendations:

- **Close Finding 1 before merge, or record it as an accepted risk with an
  owner.** A `sync_run.rs` binary in `work-adapters/tests/` driving `run`
  over `RecordingTracker` would cover the write bounds, the preview
  guarantee and the apply dispatch without any structural change. The
  refusal is the one feature whose whole purpose is to prevent a
  large-blast-radius mistake, and it is currently asserted nowhere.
- **Move the `ACCELERATOR_TRACKER_CONTRACT` check out of `run_all`** into
  the two shape-specific functions as well, or into a shared guard they all
  call, so the gate matches its documented "independent" description before
  0171 attaches real clients to it.
- **Correct the plan's Phase 1 criterion and Manual Testing Step 2** to say
  the harness errors rather than skips, so the record matches the code.
- **Amend Phase 4's deviation note**, which currently reassures a reader
  that `run` is covered.
- **Resolve the `for_tracker_error` duplication** in either direction —
  point `create.rs` at the shared constant or delete the unused function.
- Consider carrying Finding 3 into 0171 as an acceptance criterion.
