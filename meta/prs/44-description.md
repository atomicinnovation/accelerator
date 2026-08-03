---
type: pr-description
id: "44"
title: "Fix four load-sensitive test flakes"
date: "2026-08-03T15:19:25+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
pr_url: "https://github.com/atomicinnovation/accelerator/pull/44"
pr_number: 44
tags: [testing, ci, flakiness, build-system, frontend, visualiser]
revision: "05a35fdc7abf372e3830ae5f6b3af334c4406e69"
repository: "accelerator"
last_updated: "2026-08-03T15:19:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Fix four load-sensitive test flakes

## Summary

Four tests fail intermittently on CI and pass in isolation. All four fail for the same underlying reason — a budget or a check that is really measuring *how busy the machine is* — but each needs a different fix, so this is four independent commits. None of them is fixed by retrying, and all four have already cost real CI runs: two of the last twelve `main` runs failed on one of them, PR #42 was re-run for another, and two of them took out PR #43's `Run unit tests (macos-latest)` three times over.

**Based on `main`, with `0186-closeout` (PR #43) stacked on top of this branch.** That ordering is deliberate and is itself part of the fix: #43 is a docs-only change that kept failing `Run unit tests (macos-latest)` on flakes it inherited from `main`, and while these fixes sat *above* it in the stack its CI could never see them. Content-independent of #43 — that branch touches three `meta/` documents and no code.

## Changes

### 1. The vendor-shim marker tests copied 46k files to read three

`vendor_shim_marker_digest` reads `cli/verify/**`, the `minisign-verify` pin in `cli/Cargo.toml`, and `cli/Cargo.lock` — three inputs, about 12 KB. The three tests that feed it a mutated tree copied the whole of `cli/`: 46,391 files including the ~200 MB `cli/visualiser/frontend/node_modules`, with symlinks followed. Any concurrently-running frontend task can move a link target mid-walk, and `shutil.copytree` raises:

```
shutil.Error: [(... /node_modules/@tanstack/router-core/skills/.../SKILL.md,
  "[Errno 2] No such file or directory: ...")]
```

Now they copy the three inputs. Each test still measures against a baseline digest taken over the **real** tree, so an input missed by the seeding shows up as a digest mismatch rather than a false pass — the tests check their own fixture.

`tests/unit/tasks/test_build.py` drops from **11.3s to 0.1s**.

### 2. The frontend watcher was confirmed by poll count, not elapsed time

Accepting `dev up` required two consecutive `"active"` polls inside `frontend_active_timeout`. That measures the wrong thing in both directions:

- On an idle machine the two polls land 100ms apart and prove almost nothing about staying-power — which is what the check exists to establish.
- On a loaded runner, where one status call can take up to `probe_timeout` (5s in the integration driver), a perfectly healthy watcher may never land two *consecutive* polls inside the deadline, and the whole dev stack is torn down for it.

An unreadable poll made it worse. `SupervisorUnreachableError` was folded into an empty status map, which took the `else` branch and reset the counter — so a transient timeout on a busy runner threw away already-confirmed progress. That is the CI failure.

The check is now two distinct questions:

1. **Is it up?** Poll until the watcher reports active, bounded by the deadline. An unreadable poll is retried, not counted against the watcher.
2. **Did it stay up?** Sleep a fixed `frontend_settle` (0.5s) and re-read. This is wall-clock, so it means the same thing regardless of poll cadence.

`None` now means *unknown* rather than *down*, at both steps. At the confirmation it is accepted: being unable to read the status is not evidence the watcher died, and `do_status` already reports a genuinely dead watcher as degraded, which is the safety net. Failing there would just move the flake one step later.

A watcher that really does die in the settle window now reports `became active then stopped` instead of a misleading activation timeout.

This also removes a latent crash. The sleep was `min(0.1, deadline - deps.clock.now())`, and the loop condition was evaluated *before* a status call that can outlast the deadline — so the remainder could go negative, and `sleep` rejects a negative duration.

### 3. `waitFor` had a 1000ms budget that `testTimeout` does not govern

`vite.config.ts` already raises `testTimeout` to 30s, with a comment recording that the number is a hang detector rather than a latency budget, because this suite is scheduled alongside cargo builds and the Python suites. But `waitFor` — and every `findBy*` built on it — polls against its **own** budget, which `testTimeout` does not govern and which still defaulted to 1000ms.

So a mocked-promise render that settles in ~50ms idle can exceed it under load and fail with `Unable to find role="heading" and name "Foo Cluster"`, having only ever rendered `Loading…`. That is exactly the observed failure, in both affected specs.

The existing reasoning is extended to the async utils: `configure({ asyncUtilTimeout: 5_000 })`, still well inside the 30s `testTimeout` so a genuinely stuck query reports as itself rather than being swallowed. `configure` is imported from `@testing-library/react` rather than `@testing-library/dom`, which is not a declared dependency — the same reason every spec reaches `screen` and `waitFor` through that package.

### 4. The watcher tests waited sub-second for a full rescan

Same shape as #3, one language over. Every `watcher.rs` test that expects a filesystem-driven SSE event waited 500ms for it — 800ms in the create/edit/delete chain. That is a latency budget wearing a hang detector's clothes, because delivery runs the whole chain: FSEvents/inotify latency, then the debounce, then `indexer.rescan()` plus `compute_clusters_with_backfill` over the entire snapshot plus `collect_linked_counts`. Cargo runs this module's tests concurrently — one `current_thread` runtime each — while `mise run` schedules the suite alongside cargo builds and the Python and frontend suites.

Instrumented, that chain settles in **~18ms** idle, so 500ms already looked like 28× headroom. A loaded macOS runner still blew through it, failing `new_file_in_watched_dir_produces_doc_changed_event` with a bare `timed out: Elapsed(())`.

The positive waits now route through a shared `EVENT_BUDGET` of 5s. The **expects-no-event** waits deliberately keep their short windows (200–400ms): there the wait *is* the assertion, so lengthening it would only slow the suite without strengthening it. The constant's comment says exactly that, so the two are not helpfully unified later.

Raising the budget weakens no assertion in the affected test. Its `before`/`after` bracketing and the AC6 one-second tolerance both measure *broadcast → receipt*, because the payload's `timestamp` is captured at line 193, after all the rescan work and immediately before the broadcast. Extra fsevents latency lands outside both windows.

One further change, for a sharper reason than budget. `watcher_fires_in_this_env` probes whether the platform watcher fires at all, and its budget was 300ms. A `false` return **silently skips its caller** — so a slow-but-working runner read as "not firing" turns the whole test into a false green rather than a failure. That probe now uses the same `EVENT_BUDGET`.

## Context

Flakes 1–3 were diagnosed while getting PR #42's checks green. #42 changes no file involved in any of those failures, so none of them is caused by it — its description records all three as deserving their own item. This is that item.

Flake 4 was found while diagnosing why PR #43 kept failing `Run unit tests (macos-latest)`. That investigation turned up two things: the watcher flake below, which was unfixed anywhere, and the stack-ordering problem described in the Summary.

Observed failures:

| Test | Where |
| --- | --- |
| `test_dev_integration.py::test_restart_round_trip` | PR #42, `Run integration tests (macos-latest)` |
| `LifecycleClusterView.test.tsx` — "Foo Cluster" | `main` run 30767051461, PR #43 runs 30825942165 + 30831408793, and locally |
| `use-dev-activation.test.tsx` — "restores the LATEST non-/dev path" | locally, under full-suite load |
| `watcher::tests::new_file_in_watched_dir_produces_doc_changed_event` | PR #43 run 30828047240 |

## Testing

- [x] `mise run check` — exit 0, including `types:frontend:check` and `lint:build-system:check`.
- [x] `mise run test:unit:tasks` — exit 0, 625 passed (620 before; +5, all in the new `TestFrontendActivation`).
- [x] `mise run test:integration:tasks` — exit 0, 65 passed.
- [x] `mise run test:unit:frontend` — exit 0, **2536 passed**, 8.85s. No test got slower: nothing relies on an async util timing out, since the negative assertions use the synchronous `queryBy`.
- [x] `mise run test:integration:dev` — exit 0, 17 passed, **three consecutive runs**.
- [x] **The watcher change is covered by tests that did not exist.** The mechanism had none: `FakeSupervisor.start` flipped the status to active, so every case arrived at the loop with the answer already yes. Five cases now cover slow activation, transient unreachability while waiting, transient unreachability at the confirmation, never activating, and dying in the settle window.
- [x] **Each new test is mutation-checked** — every mutation kills exactly the intended test, and every test is killed by at least one mutation:

  | Mutation | Test killed |
  | --- | --- |
  | drop the settle confirmation | `test_a_watcher_that_dies_during_the_settle_window_fails` |
  | an unreadable poll counts as a negative (the old behaviour) | `test_a_transiently_unreachable_supervisor_does_not_fail_it` |
  | an unreadable settle poll counts as death | `test_an_unreadable_settle_poll_does_not_fail_the_watcher` |
  | give up after the first non-active poll | `test_a_watcher_slow_to_report_active_is_accepted`, `test_a_transiently_unreachable_supervisor_does_not_fail_it` |

- [x] **The `asyncUtilTimeout` change is control-tested.** A probe component resolving at 1500ms passes with the `configure` in place and fails with `Unable to find role="heading"` without it — the same signature as the CI failure. The probe was temporary and is not committed.
- [x] `mise run cli:check` — exit 0 (workspace-wide rustfmt + clippy).
- [x] `mise run test:unit:visualiser` — exit 0, 334 + 330 passed. The watcher tests run in 0.84s, unchanged: a raised timeout costs nothing unless it is exceeded.
- [x] `mise run test:unit` — exit 0 with the whole group standing on `main` rather than on #43, confirming none of these fixes depended on that branch.
- [x] **The watcher tests do not silently skip.** Verified `watcher_fires_in_this_env` returns true locally, so all 15 watcher tests genuinely execute rather than passing vacuously.
- [x] **Watcher latency measured, not guessed.** Temporary instrumentation put delivery at 17–20ms across three full-suite runs.
- [ ] **Repeated green CI runs.** By construction these are load-dependent, so the real evidence is the macOS jobs staying green over the next several runs rather than anything reproducible locally.

## Notes for Reviewers

**The watcher change trades a false negative for a narrow false positive, deliberately.** If the supervisor is unreadable at the confirmation poll, the start is now accepted. A watcher that died in that exact window would be reported as started. That is the right trade for a developer-facing dev task: the cost of the old behaviour was tearing down a healthy stack and failing CI, while the cost of the new one is `dev up` reporting success for a stack that `dev status` will immediately show as degraded. The alternative — failing on "could not read" — is precisely the bug being fixed.

**`frontend_settle` sits outside the activation deadline on purpose.** Clamping it to the remaining deadline would silently skip the check for a watcher that became active just before the deadline, which is exactly the slow case it exists for. The cost is a bounded extra 0.5s per `up`.

**The seeding helper in commit 1 is not self-validating in isolation** — it is validated by its callers, which compare against a digest of the real tree. If someone later adds an input to `vendor_shim_marker_digest` without adding it to `_seed_digest_inputs`, the three tests fail with a digest mismatch rather than a helpful message. That is a real (if loud) failure mode, and the helper's docstring says so.

**Flake 4 could not be reproduced locally, and the fix is reasoned rather than demonstrated.** The watcher tests survive 12/12 runs under 12-way CPU load, but they also survived 12/12 at the *old* 500ms budget — this hardware has too much headroom, and loading it did not degrade delivery latency at all (~18ms idle, ~7–9ms under load; the variation is noise). The stretch that broke CI is specific to a 3–4 core shared macOS runner mid-way through a 240s job. So the evidence for #4 is the measured idle latency, the identical failure signature, and the precedent set by #3 in the same CI job — not a local reproduction. Worth weighing if you would rather cap that job's concurrency instead.

**Not fixed here: the underlying parallelism.** All four failures ultimately trace to `mise run test` scheduling every suite at once on a 3-core runner. Raising budgets and shrinking fixtures treats the symptom. If macOS keeps flaking after this, the next move is capping concurrency for that job rather than raising numbers again — and after four separate budget fixes, that point is arguably already here.

**A correction to PR #42's description.** It reports `mise run test:unit:tasks — 619 passed`. A clean re-measurement of that same commit collects **620**. I could not reconstruct where the 619 came from, and the discrepancy is one test either way; #42's committed description and posted body are left as they are rather than force-pushing over a review-in-progress for a ±1 prose count. The number to trust is 620 before this PR, 625 after.
