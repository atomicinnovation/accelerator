---
type: plan-review
id: "2026-06-06-0101-unified-dev-task-for-visualiser-review-1"
title: "Plan Review: Unified Managed dev Task for Visualiser Server and Frontend"
date: "2026-06-06T19:05:17+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "work-item:0101"
target: "plan:2026-06-06-0101-unified-dev-task-for-visualiser"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, correctness, test-coverage, code-quality, safety, portability, usability]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-06-06T22:39:15+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Unified Managed dev Task for Visualiser Server and Frontend

**Verdict:** REVISE

This is a strong, unusually disciplined plan: it carves the supervisor into a
pure, individually-testable functional core (Phase 2) sitting under a thin
imperative orchestration shell, the per-workspace state-file design gives clean
cross-workspace isolation, and the up-front circus research corrects the work
item's mental model with verified facts (signal-by-PID ⇒ `stop_children`,
30 s default ⇒ `graceful_timeout = 2`, blocking `start()` ⇒ `circusd --daemon`).
The verdict is REVISE — not for structural flaws, but because two **critical**
issues would let core behaviour ship broken (the readiness gate trusts a stale
`server-info.json`, and the macOS/Linux parity the whole feature promises is
never exercised in CI), reinforced by a cluster of **major** findings around
the free-port TOCTOU, the no-orphan fallback path, orchestrator complexity, and
DX discoverability. All are tractable wording/criteria/design additions, not
re-architecture.

### Cross-Cutting Themes

- **Readiness gate trusts file existence, not freshness** (flagged by:
  correctness, safety) — `wait_for_file(_SERVER_INFO_PATH)` returns `True` the
  instant the file exists, but `server-info.json` is server-owned, lives in the
  persistent `dev-server/` dir, and is never deleted before launch. A leftover
  file from any prior run satisfies the gate immediately, starting Vite against
  the old/dead port — silently defeating the ordering guarantee that is the
  feature's entire reason to exist. The prior-art launcher deletes it
  (`launch-server.sh:41`); this plan does not.

- **Free-port allocate-then-bind TOCTOU** (flagged by: architecture,
  correctness, safety) — `free_port()` binds to 0, reads, releases, and circus/
  Vite rebind *later*. Vite has `--strictPort` (fails loud), but the circus
  `endpoint`/`pubsub_endpoint` ports have no equivalent guard or retry, and a
  cross-workspace collision could make one workspace's liveness probe reach
  another workspace's arbiter — the exact isolation failure the design is
  meant to prevent.

- **No-orphan guarantee has gaps on the non-happy path** (flagged by: safety,
  correctness, architecture) — the guarantee rests entirely on circus's `quit`
  reaching children via `stop_children`. The direct-kill fallback reaps only the
  recorded *arbiter* PID, leaving orphaned server/Vite children un-reaped; and a
  partial launch (pidfile never appears) can leave a live arbiter with no
  recoverable dev-state. Both undermine Requirement 9.

- **Cross-platform parity asserted, not enforced** (flagged by: portability) —
  the headline requirement is identical macOS/Linux behaviour, yet CI runs all
  test jobs on `ubuntu-latest` only; macOS is hand-checked once. Compounded by
  `psutil.create_time()` having different precision/units per platform (the ±1 s
  tolerance may be mis-sized), unverified circus detach/`stop_children` parity on
  macOS, and unverified wheel availability under `requires-python >=3.14`.

- **Orchestrator complexity and the test injection seam** (flagged by:
  code-quality) — `up` is a single 9-step `@task` with many failure branches and
  embedded effects, and the plan says tests inject a fake `CircusClient`/launcher
  but invoke `@task` functions only receive `(context)` — no injection seam is
  described, so the branchiest logic has no defined unit-test entry point.

- **DX is behaviourally complete but undiscoverable** (flagged by: usability) —
  success output format, log-file locations, and the 0/3/4 exit-code contract are
  all encoded in behaviour but never surfaced via printed output, docstrings, or
  `mise` descriptions.

### Tradeoff Analysis

- **Identity-gate tolerance — tighter vs looser**: Safety suggests an exact (or
  sub-second) `create_time()` match since capture and comparison share the same
  psutil clock source (the shell launcher's ±1 s was tuned for a /proc-vs-`ps`
  boundary that no longer applies). Portability cautions that psutil's
  resolution differs across macOS and Linux, so any fixed tolerance must be
  validated per-platform. Both point to the same action: **empirically validate
  the tolerance on both OSes and document why the chosen value is correct**,
  rather than inheriting ±1 s by default.

### Findings

#### Critical

- 🔴 **Correctness + Safety**: Readiness gate passes instantly on a stale `server-info.json`, wiring Vite to a dead port
  **Location**: Phase 3, Section 2 (`up` orchestrator, steps 3 & 7)
  `wait_for_file(_SERVER_INFO_PATH, timeout=30)` returns `True` as soon as the
  path exists, but the file is server-owned, persists in `dev-server/`, and is
  never deleted before launch (stale cleanup only removes `dev.json`/`circusd.pid`/
  `circus.ini`). A leftover file satisfies the gate immediately, the frontend
  starts against the old/wrong port, and `/api` proxying breaks — the common case
  across repeated dev sessions and after a crash.

- 🔴 **Portability**: macOS/Linux parity is never verified in CI — all test jobs run on `ubuntu-latest`
  **Location**: Phase 5 (Manual Verification); Desired End State
  CI runs `test-unit`/`test-integration`/`test-e2e`/`check` exclusively on
  `ubuntu-latest`; only prerelease/release jobs use macOS. The core requirement
  (identical exit codes, no orphans, 2 s grace, detach behaviour on both OSes) is
  asserted but never enforced — macOS regressions in circus detach, signal
  escalation, or psutil timing pass CI and surface only on a developer's machine.

#### Major

- 🟡 **Architecture + Correctness + Safety**: Free-port allocate-then-bind TOCTOU with no strict-port guard on circus endpoints
  **Location**: Phase 2 §1 (`free_port`) + Phase 3 §2 (step 4)
  Ports are bound-to-0/read/released, then rebound later by circusd/Vite. Vite
  has `--strictPort`; the circus `endpoint`/`pubsub_endpoint` ZMQ ports do not. A
  lost race surfaces as an opaque arbiter-bind failure (pidfile never appears),
  and a cross-workspace collision could let one workspace's probe/`stop` act on
  another's arbiter. Suggest a bounded re-allocate/re-render retry and treating
  endpoint-bind failure as a first-class outcome.

- 🟡 **Architecture**: No intra-workspace serialisation — TOCTOU in the reuse/allocate/launch sequence
  **Location**: Phase 3 §2 (steps 1–6); Implementation Approach
  The prior-art launcher serialises with flock/mkdir; the unified `dev` drops it.
  Two concurrent `dev` runs in one workspace can both miss the reuse short-circuit
  and both launch arbiters, leaving an unmanageable orphan that `dev:stop` cannot
  reach. Suggest a non-blocking per-workspace lock around the critical section.

- 🟡 **Correctness + Safety**: Pidfile poll has no timeout/failure branch; partial launch leaves an unrecoverable arbiter
  **Location**: Phase 3 §2 (steps 5–6)
  "Poll briefly" for the pidfile is unbounded and has no failure path. If circusd
  fails to bind/start, the pidfile never appears: the poll hangs or reads a
  missing file, and a live-but-undiscoverable arbiter can survive with no
  dev-state to drive teardown. Suggest a bounded poll (reuse `wait_for_file`),
  writing dev-state *before* launch so a partial start is always recoverable, and
  a clean non-zero exit naming the daemon log.

- 🟡 **Correctness**: PARTIAL health conflates the legitimate startup window with a degraded session
  **Location**: Phase 2 §4 (`evaluate_health`) + Phase 3 §2 (step 1)
  The frontend watcher is `autostart = false` and started only after the readiness
  gate, so `server=active, frontend=inactive` is a normal startup state yet maps
  to `PARTIAL`. A concurrent `dev`/`dev:status` in that window tears down an
  in-flight session (or reports exit 3). Suggest distinguishing "frontend not yet
  started" from "frontend died".

- 🟡 **Correctness + Safety + Architecture**: `restart` = `stop` then `up`, but `stop` can raise / leave state half-torn-down
  **Location**: Phase 3 §4 (`restart`) + §3 (`stop`)
  `stop`'s exit/exception contract is undefined (does a refused identity-gate or
  failed SIGKILL raise?). If `stop` raises, `up` never runs and the stack is left
  down; if `stop` returns with a still-live unhealthy arbiter, `up` may double-
  launch. Suggest defining `stop`'s contract, guaranteeing it leaves dev-state
  absent on a clean return, and making `restart` reconcile before launching.

- 🟡 **Safety**: Direct-kill fallback reaps only the arbiter PID, leaving orphaned children
  **Location**: Phase 3 §3 (`stop`, identity-gated fallback)
  When the arbiter is unreachable/dead but its server+Vite children survived
  (the orphan scenario), killing only the recorded arbiter PID does nothing about
  them, and circus's child-reaping never runs (endpoint gone). Suggest reaping
  `psutil` descendants under the same SIGTERM→2 s→SIGKILL escalation, and covering
  the arbiter-dead-but-children-alive case in integration.

- 🟡 **Code Quality**: `up` concentrates 9 steps and many failure branches in one task function
  **Location**: Phase 3 §2 (`up`)
  Nine sequential steps + three reuse/stale branches + a readiness-failure branch
  + inline kill/cleanup, all with embedded effects. Suggest decomposing into named
  helpers (`reuse_or_clean`, `launch_arbiter`, shared `teardown`) with the `@task`
  body kept to a short guard-claused sequence.

- 🟡 **Code Quality**: Orchestrator injection seam for `CircusClient`/launcher is unspecified
  **Location**: Phase 3 (Overview) + Testing Strategy
  Tests "inject a fake CircusClient and fake launcher," but `@task` functions get
  only `(context)`. With no seam, the branchiest logic is testable only via the
  heavier real-circusd suite. Suggest a plain `bring_up(*, client_factory,
  launcher, clock, state_path)` in `dev_supervisor.py` with the task as a one-line
  adapter.

- 🟡 **Code Quality**: Teardown error handling on partial failures / mid-teardown exceptions is unspecified
  **Location**: Phase 3 §3 (`stop`) + `up` readiness-failure teardown
  No stated behaviour when `quit` raises an unexpected `CallError`, the kill
  succeeds but an unlink fails, or removal partially completes. Suggest specifying
  the tolerated-vs-fatal contract, `missing_ok` best-effort removal of all three
  artifacts, and a mid-teardown-exception unit test.

- 🟡 **Test Coverage**: Wall-clock-dependent integration assertions risk CI flakiness
  **Location**: Phase 5 (readiness-timeout ~30 s + 2 s-grace tests)
  Hard timing bounds on a loaded CI runner (where this repo's integration suites
  already flake under parallel load) are a classic flake source. Suggest
  parametrising the readiness timeout small (~2 s) and asserting direction not
  magnitude; assert SIGKILL *happened* with a generous margin; consider serialising
  the timing-sensitive cases.

- 🟡 **Test Coverage**: Fake `CircusClient` contract is unspecified — real circus behaviour can drift undetected
  **Location**: Phase 3 (fake CircusClient)
  Real circus has behaviour-sensitive shapes (`{"statuses": {...}}` vs
  `{"status": ...}`; `CallError` on unreachable vs *timeout* on a slow live
  endpoint). An over-friendly mock keeps unit tests green while production breaks.
  Suggest pinning the fake to the documented contract and cross-checking each
  branch against real circusd in Phase 5.

- 🟡 **Test Coverage**: Several acceptance criteria are covered only by manual verification
  **Location**: AC mapping (work item) vs Testing Strategy
  Prerequisite auto-run (AC 2), real `/api` proxying with no port-0 fallback
  (AC 1/6), and two-workspace coexistence are manual-only; the integration harness
  fakes both processes so the real Vite ordering gate is never asserted
  automatically. Suggest an automated cross-workspace/concurrency case (needs no
  Rust/Vite) and either a thin env-wiring smoke check or an explicit documented
  manual-only gap.

- 🟡 **Portability**: circus/pyzmq/tornado/psutil wheel availability unverified against `requires-python >=3.14`
  **Location**: Phase 1 (dependency declaration); `pyproject.toml`
  "importable" is checked but not that prebuilt wheels exist for CPython 3.14 on
  both arm64 macOS and x86_64 Linux; pyzmq/psutil frequently fall back to source
  builds on a very new interpreter. Suggest recording verified resolution on both
  platforms (or documenting required toolchain/libraries).

- 🟡 **Portability**: `psutil.create_time()` resolution/units differ across platforms; ±1 s tolerance may be mis-sized
  **Location**: Phase 2 §7 (PID identity check)
  Linux derives start-time from /proc jiffies+btime at CLK_TCK granularity; macOS
  reads kinfo_proc at microsecond granularity. The ±1 s tolerance was tuned for
  the shell helper's /proc-vs-`ps` mismatch, not psutil's normalised value.
  Suggest validating recycle/drift behaviour on both OSes and documenting the
  chosen tolerance.

- 🟡 **Portability**: circusd `--daemon` detach + `stop_children` tree-walk parity asserted, not demonstrated, on macOS
  **Location**: Current State Analysis (circus verification); Implementation Approach
  Daemonisation and recursive child discovery can behave subtly differently under
  launchd vs Linux. Suggest proving "arbiter detached and survives the shell" and
  "no children of recorded PIDs survive" on a macOS CI runner rather than from
  docs.

- 🟡 **Usability**: Success output format for `dev` is unspecified
  **Location**: Phase 3 §2 (step 9) + Phase 4 (status)
  After a detached command returns, the printed lines are the only feedback. The
  plan says "print URL/ports" without format, labelling, or which URL to open.
  Suggest a concrete labelled "ready" block (`Frontend:`/`API:` on their own
  lines) asserted in manual verification.

- 🟡 **Usability**: Log files are discoverable only by convention; nothing on screen points to them
  **Location**: What We're NOT Doing (no `dev:logs`) + Phase 3/4
  The log path appears only in the *failure* readiness-timeout message; on success
  and on `PARTIAL`/`DOWN` status there is no pointer. With detached processes, logs
  are the primary debugging surface. Suggest printing the two log paths on `dev`
  success and in non-HEALTHY `dev:status`.

- 🟡 **Usability**: Exit-code contract (0/3/4) is undiscoverable
  **Location**: Phase 4 §1 (status) + Desired End State
  The scriptable codes appear in no docstring, `--help`, `mise` description, or
  printed legend, and 3/4 are non-obvious. Suggest stating the legend in the
  `dev:status` docstring and `mise.toml` description and correlating the printed
  wording with each state.

#### Minor

- 🔵 **Architecture**: Detached arbiter is a single point of failure between launch and dev-state write
  **Location**: Phase 3 §2 (steps 5–7)
  Interrupt/crash after circusd detaches but before dev-state is written strands a
  fully-detached arbiter with no recorded handle. Suggest a provisional dev-state
  before launch, or having stale-cleanup scan the known pidfile path independently.

- 🔵 **Architecture**: `restart` couples to `stop` fully completing; invariant not stated
  **Location**: Phase 3 §4 (`restart`)
  Suggest documenting and unit-testing the invariant that `stop` always leaves
  dev-state absent on return.

- 🔵 **Architecture**: Two-directory split (`dev-server/` vs `dev/`) leaves the discovery boundary implicit
  **Location**: Phase 3 §1 (constants); Migration Notes
  Suggest a one-line note/comment that `dev-server/` is server-owned, `dev/` is
  orchestration-owned, and `server-info.json` is the sole cross-directory contract.

- 🔵 **Correctness**: Quit-poll vs identity-gated direct-kill ordering can orphan children if the wait is too short
  **Location**: Phase 3 §3 (`stop`)
  An under-sized post-`quit` wait can SIGKILL the arbiter mid-teardown before
  circus reaps children. Suggest a wait generous enough to cover the 2 s
  `graceful_timeout` + child-reaping margin, and verifying children are gone.

- 🔵 **Correctness**: Per-workspace isolation does not cover the shared `dev-server/` config/info paths
  **Location**: Overview / Phase 2 (dev-state)
  Concurrent unified `dev` + legacy `dev:server` in one workspace share
  `config.json`/`server-info.json` with no lock. Suggest moving the dev-path info
  under `dev/` or guarding against concurrent `dev:server`.

- 🔵 **Correctness**: `wait_for_file` ~300 poll-count assertion implies a boundary off-by-one
  **Location**: Phase 2 §6
  Suggest defining the loop precisely (check at t=0, sleep `min(interval,
  deadline-now)`, final check) and asserting an exact poll count.

- 🔵 **Test Coverage**: No test that the three allocated ports are distinct / race handling
  **Location**: Phase 2 `free_port` + Phase 3 step 4
  Suggest a unit test asserting the three ports differ, and noting the
  release-then-bind window explicitly if the race is accepted.

- 🔵 **Test Coverage**: Ambiguity about which file the fake-driven tests live in (mise registration)
  **Location**: Phase 2 Success Criteria + `mise.toml` `test:unit:tasks`
  A pytest file absent from every explicit mise list runs locally but is skipped
  in CI. Suggest stating all Phase 2/3 fake-driven tests live in
  `tests/tasks/test_dev.py` and confirming the run includes them.

- 🔵 **Test Coverage**: Recycled-PID safety gate exercised only against mocked psutil
  **Location**: Phase 2 §7 + Phase 5
  Suggest an integration assertion that records a real arbiter PID+create_time,
  lets it exit, and confirms `pid_identity_matches` returns `False` on both OSes.

- 🔵 **Code Quality**: Teardown logic is duplicated across the `up` stale gate, `up` readiness failure, and `stop`
  **Location**: Phase 3 §2 (step 1) + §3
  Three copies of identity-gate + kill + removal will drift. Suggest one shared
  `teardown(state, …)` helper all three delegate to.

- 🔵 **Code Quality**: Only the readiness-timeout path specifies a diagnostic message
  **Location**: Phase 3 §2 (error messaging)
  Other failures (no pidfile, daemon non-zero, `start frontend` error) get no
  equivalent diagnostic. Suggest each branch raises `invoke.Exit` naming the
  relevant artifact.

- 🔵 **Code Quality**: `DevState` mixes endpoint strings and structured fields without typing/validation guidance
  **Location**: Phase 2 §3
  Suggest specifying field types, deciding whether endpoints are stored as ports
  or full strings, and treating a schema-mismatched file as malformed (`None`).

- 🔵 **Safety**: Removing dev-state before confirming the arbiter is dead can strand a survivor
  **Location**: Phase 3 §3 (state-file removal ordering)
  Suggest removing `_DEV_STATE`/`_PIDFILE`/`_INI` only after a post-SIGKILL
  liveness check passes (mirroring `stop_server_stop`); on identity mismatch, keep
  state and surface an error.

- 🔵 **Safety**: Confirm the ±1 s start-time tolerance is applied symmetrically to the arbiter PID
  **Location**: Phase 2 §7 + Phase 3
  Since capture and comparison share the psutil clock, the boundary-crossing
  rationale for ±1 s may not apply; consider an exact/sub-second match.
  *(See Tradeoff Analysis — pairs with the portability tolerance finding.)*

- 🔵 **Portability**: npm/node toolchain assumed on PATH inside the detached arbiter environment
  **Location**: Phase 2 §2 (frontend watcher cmd)
  A detached daemon may run with a reduced environment where mise/asdf/nvm-shimmed
  `npm` is not resolvable. Suggest confirming PATH inheritance (or an explicit
  env/cwd in the INI) and a manual-verification note for version-manager toolchains.

#### Suggestions

- 🔵 **Portability**: Vendor coupling to circus has no substitution boundary
  **Location**: Decision / Phase 1 ADR-0041
  Acceptable lock-in for a dev-only tool, already being recorded in ADR-0041.
  Suggest the ADR explicitly note the absence of an abstraction boundary and the
  migration cost, and keep the pure helpers circus-agnostic where feasible.

- 🔵 **Usability**: Reuse messaging is unspecified — silent reuse can read as a no-op
  **Location**: Phase 3 §2 (reuse short-circuit)
  Suggest an explicit line ("Reusing running dev stack — use `dev:restart` to pick
  up changes").

- 🔵 **Usability**: Bare `mise run dev` mental model and the invoke/mise name split deserve help text
  **Location**: `tasks/__init__.py` wiring (Phase 3 §5)
  Suggest a one-line mental model in the `up` docstring and `mise.toml` `dev`
  description (detached background; `dev:stop` to tear down; `dev:server`/
  `dev:frontend` for the manual flow).

- 🔵 **Usability**: `dev:stop` partial-failure feedback under stale/recycled/timeout conditions is unspecified
  **Location**: Phase 3 §3 (`stop`) + Phase 5 (stale cleanup)
  Suggest human-readable messages for the stale/recycled/timeout branches so every
  teardown outcome is legible.

### Strengths

- ✅ Exemplary functional-core / imperative-shell separation: Phase 2 isolates all
  pure logic (`render_circus_ini`, `evaluate_health`, `status_exit_code`,
  `wait_for_file` with injected `sleep`/`now`, `pid_identity_matches`) into
  deterministic, individually-testable units before any I/O wiring.
- ✅ Per-workspace state under `.accelerator/tmp/dev/` leverages jj's per-workspace
  working copy for natural cross-workspace isolation with no shared global state —
  the right boundary for the multi-instance requirement.
- ✅ The up-front circus research corrects the work item's mental model with
  verified facts and bakes the fixes into frozen INI invariants (`stop_children =
  true`, `graceful_timeout = 2`, `respawn = false`, `autostart = false` on
  frontend) that unit tests assert.
- ✅ Liveness decided by probing the recorded endpoint with a short timeout, with
  `(pid, start_time)` identity used *only* as a safety gate before destructive
  direct kills — a clean separation of the normal decision path from the
  destructive fallback.
- ✅ Replacing the shell launcher's macOS `LANG=C` start-time parsing with
  `psutil.create_time()` removes a known cross-platform fragility (an explicitly
  justified, well-reasoned divergence).
- ✅ Phase decomposition keeps the repo green and independently mergeable at each
  step, each behavioural phase is written test-first, and the generated-INI
  approach correctly keeps `FileStream`/`stop_children`/`graceful_timeout` out of
  the ZMQ marshalling path.
- ✅ Retaining `dev:server`/`dev:frontend` unchanged preserves the existing
  single-process mental model — progressive disclosure done well — and the
  readiness-timeout failure message already names both the missing file and the
  server log.
- ✅ The teardown-without-orphans integration test asserts against children of
  recorded PIDs (not a global grep), matching the work item's acceptance-criterion
  wording.

### Recommended Changes

1. **Make the readiness gate freshness-aware** (addresses: stale `server-info.json`
   critical). Delete `_SERVER_INFO_PATH` (and any dev-path server pidfile) in
   `up` step 3 before launching the arbiter — as `launch-server.sh:41` does — or
   gate on mtime-after-launch / a changed-port content check rather than mere
   existence. Add a unit/integration test that a pre-existing stale info file does
   *not* satisfy the gate.

2. **Add a macOS CI job for the new suites** (addresses: parity-not-enforced
   critical; psutil tolerance; circus detach parity; wheel availability). Extend
   the integration (and ideally unit) job with `strategy.matrix.os:
   [ubuntu-latest, macos-latest]` so `test:unit:tasks` + `test:integration:dev`
   run on both, turning the parity claim and the dependency-resolution check into
   enforced CI rather than a one-off manual check.

3. **Close the free-port TOCTOU and partial-launch gaps** (addresses: free-port
   TOCTOU; pidfile poll; partial-launch orphan; intra-workspace race). Specify a
   bounded pidfile poll with an explicit failure path; treat circusd endpoint-bind
   failure as first-class with a bounded re-allocate/re-render retry; write
   dev-state *before* launch so a partial start is recoverable; and add a
   non-blocking per-workspace lock around the reuse/allocate/launch critical
   section.

4. **Harden the teardown / no-orphan fallback** (addresses: arbiter-only reap;
   state-removal ordering; quit-poll timing; teardown error handling). In the
   direct-kill fallback, reap `psutil` descendants under the same SIGTERM→2 s→
   SIGKILL escalation; remove state only after a post-kill liveness check; size
   the post-`quit` wait to cover the 2 s grace + reaping; and define the
   tolerated-vs-fatal exception contract with best-effort artifact removal. Cover
   the arbiter-dead-but-children-alive case in integration.

5. **Define the orchestrator decomposition and test seam** (addresses: 9-step
   `up`; injection seam; duplicated teardown). Extract `up` into a plain
   `bring_up(*, client_factory, launcher, clock, state_path)` with a one-line
   `@task` adapter, factor a single shared `teardown(...)` helper used by all
   three call sites, and state the seam in the plan so Phase 3 fake-driven tests
   have a defined entry point.

6. **Resolve the PARTIAL/startup-window and `restart` contract** (addresses:
   PARTIAL conflation; `restart` = stop+up coupling). Distinguish "frontend not
   yet started" from "frontend died" in the reuse/status path, and define `stop`'s
   exit/exception contract plus how `restart` reconciles a failed stop before
   launching (with an `up` invariant of starting from clean state).

7. **Make the DX discoverable** (addresses: success output; log paths; exit-code
   legend; reuse/stop messaging; mental model). Specify a concrete labelled
   success block, print log paths on success and non-HEALTHY status, document the
   0/3/4 legend in the `dev:status` docstring + `mise.toml` description, add an
   explicit reuse line, and give `up`/`dev` a one-line mental-model description.

8. **Tighten the test strategy** (addresses: wall-clock flakiness; mock fidelity;
   manual-only ACs; port-distinctness; mise registration; real recycled-PID).
   Parametrise the readiness timeout small for integration and assert direction
   not magnitude; pin the fake `CircusClient` to circus's real response/exception
   shapes and cross-check in Phase 5; add an automated cross-workspace case and a
   port-distinctness unit test; state which file the fake-driven tests live in;
   and add a real recycled-PID integration assertion.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally sound and unusually disciplined: it
cleanly separates a pure functional core (Phase 2 side-effect-isolated helpers
with injected clocks) from the imperative orchestration shell (Phase 3/4 tasks),
and the per-workspace state-file design gives natural cross-workspace isolation
with no shared global state. The detach mechanism (`circusd --daemon` over a
generated INI driven by `CircusClient`) is well-reasoned and explicitly corrects
the work item's mental model with verified facts. The main structural gaps are
the absence of any intra-workspace concurrency serialisation (the prior-art
launcher's flock/mkdir lock is dropped, exposing a TOCTOU race on the reuse/
allocate/launch sequence) and a free-port allocation pattern that carries an
inherent allocate-then-rebind race the plan does not acknowledge.

**Strengths**:
- Exemplary functional core / imperative shell separation (Phase 2 pure helpers
  before any I/O wiring; Phase 3/4 thin shell over them).
- Per-workspace state file leverages jj's per-workspace working copy for
  cross-workspace isolation, avoiding shared global state.
- Liveness by endpoint probe; `(pid, start_time)` identity used only as a safety
  gate before direct kills — clean separation of normal vs destructive paths.
- Justified divergence to `psutil.create_time()` removing the macOS LANG=C
  fragility.
- Phase decomposition keeps the repo green and independently mergeable; generated
  INI keeps FileStream/stop_children/graceful_timeout out of the ZMQ path.

**Findings**:
- 🟡 major (confidence: high) — **No intra-workspace serialisation — TOCTOU race
  in the reuse/allocate/launch sequence** (Phase 3 §2 steps 1–6): the launcher's
  flock/mkdir lock is dropped, so two concurrent `dev` runs can both launch
  arbiters, leaving an unmanageable orphan. Suggest a non-blocking per-workspace
  lock around the critical section.
- 🟡 major (confidence: medium) — **Free-port allocation has an unacknowledged
  allocate-then-rebind race** (Phase 2 §1 / Phase 3 §2 step 4): a lost race on the
  circus endpoint/pubsub port surfaces as an opaque bind failure with no
  degradation. Suggest a bounded retry/reallocate around arbiter startup.
- 🔵 minor (confidence: medium) — **Detached arbiter is a single point of failure
  between launch and dev-state write** (Phase 3 §2 steps 5–7): interrupt after
  detach but before dev-state write strands an unrecoverable arbiter. Suggest a
  provisional dev-state before launch.
- 🔵 minor (confidence: high) — **`restart` composes tasks by direct call rather
  than through the supervised lifecycle seam** (Phase 3 §4 / Phase 4 §1):
  restart's correctness depends on stop fully completing; the invariant is
  implicit. Suggest documenting and unit-testing that `stop` always leaves
  dev-state absent on return.
- 🔵 minor (confidence: medium) — **Two-directory split (`dev-server/` vs `dev/`)
  leaves the server-info discovery boundary implicit** (Phase 3 §1 / Migration
  Notes): a future cleanup could break the readiness gate. Suggest a note that
  `dev-server/` is server-owned, `dev/` orchestration-owned, `server-info.json`
  the sole cross-directory contract.

### Correctness

**Summary**: The plan's decomposition into pure, unit-testable helpers (health
mapping, exit codes, INI generation, injected-clock polling) is sound and the
identity-gated kill correctly mirrors the production launcher's recycled-PID
defence. However, several concurrency and state-transition concerns are
unaddressed: the readiness gate polls a shared, server-owned `server-info.json`
that is never cleaned up, so a stale file from a prior run satisfies the gate
instantly and wires Vite to a dead port; the free-port allocate-then-later-bind
sequence is a textbook TOCTOU with no strictPort/retry on the circus endpoints;
and the reuse/health evaluation has an ambiguous window where the just-launched
frontend watcher is legitimately inactive.

**Strengths**:
- Pure functions make correctness-critical logic directly unit-testable, with
  boundary cases enumerated.
- Identity-gated kill carries forward the recycled-PID defence and confines it to
  the safety path.
- `psutil.create_time()` side-steps the macOS LANG=C pitfall.
- Correcting the signal-by-PID model (stop_children, graceful_timeout=2) fixes a
  real orphaned-children defect before it ships.

**Findings**:
- 🔴 critical (confidence: high) — **Readiness gate passes instantly on a stale
  server-info.json, wiring Vite to a dead port** (Phase 3 §2 step 7): the
  server-owned file in `dev-server/` is never deleted before launch, so a leftover
  satisfies the gate immediately. Suggest deleting it before launch or gating on
  freshness.
- 🟡 major (confidence: high) — **Allocate-then-later-bind free-port is a TOCTOU
  with no strictPort/retry on the circus endpoints** (Phase 2 §1 / Phase 3 §2 step
  4): a taken endpoint port makes circusd fail to bind, surfacing only as a missing
  pidfile. Suggest first-class bind-failure detection + bounded retry.
- 🟡 major (confidence: high) — **Pidfile-appearance poll has no specified timeout
  or failure branch** (Phase 3 §2 steps 5–6): a circusd startup failure hangs or
  reads a missing file, and dev-state may record a wrong/dead PID. Suggest a
  bounded poll with an explicit failure path, writing dev-state only after the PID
  is confirmed live.
- 🟡 major (confidence: medium) — **PARTIAL health conflates the legitimate
  startup window with a degraded session** (Phase 2 §4 / Phase 3 §2 step 1): the
  autostart=false frontend yields a normal `server=active, frontend=inactive`
  window mis-read as degraded. Suggest distinguishing not-yet-started from died.
- 🟡 major (confidence: medium) — **restart composes stop+up but stop/up raise
  invoke.Exit on non-success paths** (Phase 3 §4 / §3): if stop raises, up never
  runs. Suggest defining stop's contract and having restart proceed/reconcile
  appropriately.
- 🔵 minor (confidence: medium) — **Ordering of quit-poll vs identity-gated direct
  kill leaves a gap when quit is slow** (Phase 3 §3): a too-short wait SIGKILLs the
  arbiter mid-teardown, orphaning children. Suggest a wait covering the 2 s grace +
  reaping, verifying children gone.
- 🔵 minor (confidence: medium) — **Per-workspace isolation does not cover the
  shared dev-server config/info paths** (Overview / Phase 2): concurrent unified
  `dev` + legacy `dev:server` share files with no lock. Suggest moving dev-path
  info under `dev/` or guarding.
- 🔵 minor (confidence: low) — **Asserted poll count ~300 implies an off-by-one at
  the deadline boundary** (Phase 2 §6): suggest a precise loop definition and an
  exact poll-count assertion.

### Test Coverage

**Summary**: The plan is unusually strong on test-first discipline: it isolates
the orchestrator into pure, injected-clock helpers (Phase 2), drives the
orchestration seam with a fake CircusClient and fake launcher (Phase 3), and
backs it with a real-circusd integration harness (Phase 5). The pyramid balance
is sound and most acceptance criteria map to a named test. The principal risks
are wall-clock-dependent integration assertions under parallel CI load (where
this repo already flakes), an under-specified fake-CircusClient contract that
could let real circus behaviour drift undetected, and a few acceptance criteria
(prerequisite auto-run, frontend /api proxy, two-workspace coexistence) covered
only by manual verification.

**Strengths**:
- Phase 2 pure functions are independently unit-testable with deterministic
  assertions.
- `wait_for_file` injected clock tests the 30 s logic without real waiting.
- The orchestration seam covers the genuinely risky branches (reuse, unhealthy
  teardown, stale cleanup, readiness timeout, identity-gated kill).
- Phase 5 uses lightweight Python fake processes against real circusd, exercising
  the real lifecycle cross-platform without Rust/Vite.
- No-orphan teardown asserted via children-of-recorded-PIDs (not a global grep).
- INI invariants (stop_children, graceful_timeout=2) asserted as frozen tests.

**Findings**:
- 🟡 major (confidence: high) — **Wall-clock-dependent integration assertions**
  (Phase 5): hard timing bounds on a loaded CI runner are a classic flake source.
  Suggest a small parametrised readiness timeout, assert direction not magnitude,
  assert SIGKILL happened with a generous margin, and consider serialising the
  timing-sensitive cases.
- 🟡 major (confidence: medium) — **Fake CircusClient contract unspecified** (Phase
  3): an over-friendly mock hides reuse/stale breakages. Suggest pinning the fake
  to circus's documented response/exception shapes and cross-checking each branch
  against real circusd.
- 🟡 major (confidence: high) — **Several acceptance criteria covered only by
  manual verification** (AC mapping vs Testing Strategy): prerequisite auto-run,
  real /api proxy, two-workspace coexistence. Suggest an automated cross-workspace
  case and either an env-wiring smoke check or a documented manual-only gap.
- 🔵 minor (confidence: medium) — **No test that the three allocated ports are
  distinct / race handling** (Phase 2 / Phase 3 step 4): suggest a port-distinctness
  unit test and noting the release-then-bind window.
- 🔵 minor (confidence: high) — **Ambiguity about which file the fake-driven tests
  live in (mise registration)** (Phase 2 Success Criteria / mise.toml): a file
  absent from every explicit list is skipped in CI. Suggest stating all fake-driven
  tests live in `tests/tasks/test_dev.py` and confirming the run includes them.
- 🔵 minor (confidence: medium) — **Recycled-PID safety gate exercised only against
  mocked psutil** (Phase 2 §7 / Phase 5): suggest a real recycled-PID integration
  assertion on both platforms.

### Code Quality

**Summary**: The plan is unusually well-structured for code quality: Phase 2
deliberately isolates pure, individually-testable helpers with named,
parametrised tests, and the naming/module conventions are respected. The
principal weaknesses are concentrated in the Phase 3 `up` orchestrator and the
teardown paths: `up` is a single 9-step `@task` function carrying many failure
branches and effectful collaborators that the plan claims are "injected" without
specifying any injection seam — invoke `@task` functions only receive
`(context)`. Teardown error handling (partial failures, mid-teardown exceptions)
and the duplication of cleanup logic between `up` and `stop` are under-specified
and will be the main maintenance pain points.

**Strengths**:
- Clean pure/effect separation; deterministic, trivially unit-testable helpers.
- `wait_for_file` clock injection for fast deterministic tests.
- Naming conventions respected (flat `dev_supervisor.py`, no underscore-prefixed
  module; `_UPPER_SNAKE` constants).
- Reuse of `atomic_write_text` and extraction of `_write_server_config` follow DRY
  and existing patterns.
- Health modelled as an enum, avoiding primitive obsession.

**Findings**:
- 🟡 major (confidence: high) — **`up` concentrates 9 steps and many failure
  branches in one task function** (Phase 3 §2): suggest decomposing into named
  helpers with a short guard-claused `@task` body.
- 🟡 major (confidence: high) — **Orchestrator injection seam for CircusClient and
  subprocess launcher is unspecified** (Phase 3 Overview / Testing Strategy):
  `@task` gets only `(context)`. Suggest a plain `bring_up(*, client_factory,
  launcher, clock, state_path)` with a one-line task adapter.
- 🟡 major (confidence: medium) — **Teardown error handling on partial failures and
  mid-teardown exceptions is unspecified** (Phase 3 §3 + up readiness-failure):
  suggest specifying the tolerated-vs-fatal contract, missing_ok best-effort
  removal, and a mid-teardown-exception test.
- 🔵 minor (confidence: medium) — **Cleanup/teardown logic is duplicated across the
  up stale gate, up readiness failure, and stop** (Phase 3 §2 step 1 / §3): suggest
  one shared `teardown(...)` helper.
- 🔵 minor (confidence: medium) — **Only the readiness-timeout path specifies a
  diagnostic error message** (Phase 3 §2): suggest each failure branch raises
  invoke.Exit naming the relevant artifact.
- 🔵 minor (confidence: low) — **DevState mixes endpoint strings and structured
  fields without typing/validation guidance** (Phase 2 §3): suggest specifying
  field types, endpoint representation, and treating schema-mismatched files as
  malformed.

### Safety

**Summary**: The plan correctly inherits the proven launcher safety contract —
SIGTERM→2 s grace→SIGKILL, (pid, start_time) identity gating before any direct
kill, per-workspace state isolation, and stop_children=true to reap Vite's
node/esbuild tree. As a low-criticality dev tool with no user data, the blast
radius is small (orphaned processes, wrong-port proxying), and VCS is the
recovery path for any committed state. However, several operational-safety gaps
undermine the no-orphan guarantee and the readiness gate: a stale server-info.json
is never cleared before launch (so the gate can pass against a dead server), the
direct-kill fallback reaps only the arbiter PID and not its orphaned children, and
the free-port allocation has a TOCTOU window with no strict-port guard on the
circus endpoints.

**Strengths**:
- Preserves the (pid, start_time) identity gate as a hard precondition for any
  direct kill — the central recycled-PID safety property.
- `psutil.create_time()` sidesteps the macOS LANG=C parsing pitfall.
- Mandates stop_children=true and graceful_timeout=2 on both watchers with INI
  invariant tests, addressing the orphaned-worker risk.
- Per-workspace state with atomic writes; no shared global mutable state.
- respawn=false avoids runaway resurrection, matching the no-auto-restart contract.
- Integration suite verifies no-orphan teardown via children-of-recorded-PIDs and
  SIGKILL escalation against a SIGTERM-ignoring fake.

**Findings**:
- 🟡 major (confidence: high) — **Stale server-info.json is never cleared before
  launch** (Phase 3 §2 steps 3 & 7): *(merged into the Correctness critical above)*
  the gate can pass against a dead server. Suggest deleting it before launch.
- 🟡 major (confidence: high) — **Direct-kill fallback reaps only the arbiter PID,
  leaving orphaned server/frontend children** (Phase 3 §3): when the arbiter is
  dead but children survive, killing the arbiter PID does nothing about them.
  Suggest reaping psutil descendants under the same escalation and covering the
  case in integration.
- 🟡 major (confidence: medium) — **Free-port TOCTOU with no strict-port guard on
  circus endpoints risks cross-workspace arbiter collision** (Implementation
  Approach / Phase 2 §1 / Phase 3 §2 steps 4–5): a collision could let one
  workspace's stop quit another's arbiter. Suggest cross-checking the recorded
  (arbiter_pid, start_time) against the pidfile and handling endpoint-bind failure
  explicitly.
- 🟡 major (confidence: medium) — **Partial-launch failure can leave an orphaned
  arbiter with no recoverable dev-state** (Phase 3 §2 steps 5–6): suggest defining
  the pidfile-poll timeout/failure path and writing dev-state before launch.
- 🔵 minor (confidence: medium) — **restart proceeds to up even if stop fails**
  (Phase 3 §4): suggest aborting on an unconfirmed-clean stop or making up's stale
  gate authoritative.
- 🔵 minor (confidence: medium) — **Removing dev-state before confirming the
  arbiter is dead can strand a survivor** (Phase 3 §3): suggest removing artifacts
  only after a post-SIGKILL liveness check; keep state on identity mismatch.
- 🔵 minor (confidence: low) — **1 s start-time tolerance — verify symmetric
  application to the arbiter PID** (Phase 2 §7 / Phase 3): since capture and
  comparison share the psutil source, consider an exact/sub-second match.

### Portability

**Summary**: The plan targets two OSes (macOS + Linux) across developer machines
and CI and is admirably explicit about the POSIX-only ceiling and about
delegating signalling/detach to circus. However, several load-bearing
cross-platform claims rest on assertions rather than verified parity: circus's
--daemon detach and stop_children tree-walking behave differently in nuance on
macOS vs Linux, psutil.create_time() has materially different precision and clock
semantics across the two platforms (which the ±1 s tolerance may not absorb), and
the circus/psutil dependency tree (pyzmq, tornado) must produce installable
wheels under an unusual requires-python >=3.14 + uv prerelease=allow constraint.
Most importantly, the CI matrix runs all test jobs on ubuntu-latest only, so the
headline "identical on macOS and Linux" requirement is never actually exercised in
CI — macOS parity is verified by hand.

**Strengths**:
- POSIX-only / no-Windows boundary stated explicitly as a conscious decision.
- Per-workspace state avoids hardcoded global paths and shared state.
- Free-port allocation avoids hardcoded port assumptions.
- Recognises psutil.create_time() as the cross-platform replacement for the LANG=C
  ps parsing.
- ADR-0041 mandated to record the circus dependency and its non-Windows
  consequence.

**Findings**:
- 🔴 critical (confidence: high) — **macOS/Linux parity is never verified in CI —
  all test jobs run on ubuntu-latest** (Phase 5 Manual Verification / Desired End
  State): the parity guarantee is asserted but never enforced. Suggest a macOS
  matrix job running at least test:unit:tasks + test:integration:dev.
- 🟡 major (confidence: medium) — **circus/pyzmq/tornado/psutil wheel availability
  unverified against requires-python >=3.14** (Phase 1 / pyproject.toml): "importable"
  is checked, not wheel availability on both platforms for CPython 3.14. Suggest
  recording verified resolution or documenting required toolchain.
- 🟡 major (confidence: medium) — **psutil.create_time() resolution/units differ
  across macOS and Linux; ±1 s tolerance may be mis-sized** (Phase 2 §7): suggest
  validating recycle/drift on both OSes and documenting the chosen tolerance.
- 🟡 major (confidence: low) — **circusd --daemon detach and stop_children tree-walk
  parity asserted, not demonstrated, on macOS** (Current State Analysis /
  Implementation Approach): suggest proving detach + no-orphans on a macOS CI
  runner rather than from docs.
- 🔵 minor (confidence: high) — **Vendor coupling to circus runs throughout
  orchestration without a substitution boundary** (Decision / Phase 1 ADR-0041):
  acceptable lock-in for a dev tool; suggest the ADR note the absent abstraction
  boundary and migration cost, and keep pure helpers circus-agnostic.
- 🔵 minor (confidence: medium) — **npm/node toolchain assumed on PATH inside the
  detached arbiter environment** (Phase 2 §2): a detached daemon may lack
  mise/asdf/nvm shims on PATH. Suggest confirming PATH inheritance (or explicit
  env/cwd) and a manual-verification note.

### Usability

**Summary**: The plan delivers a strong DX win: one `mise run dev` collapses a
two-terminal dance into a single managed stack with reuse, status, and clean
teardown, and it sensibly retains the existing `dev:server`/`dev:frontend` tasks.
From a usability standpoint the orchestration, readiness-gate error (names the
missing file + log path), and exit-code contract are well specified. The main
gaps are discoverability: the 0/3/4 exit-code semantics, the log-file locations,
and the detached-arbiter mental model are encoded in behaviour but never surfaced
to the developer through task descriptions, success output, or `--help` text.

**Strengths**:
- Collapsing the two-terminal split into one prompt-returning command with reuse
  is a large time-to-first-success improvement.
- Retaining `dev:server`/`dev:frontend` preserves the single-process mental model
  (progressive disclosure).
- Readiness-timeout failure names both the missing file and the server log.
- Exit-code contract is scriptable and consistent across OSes; status surfaces the
  fields developers need.
- Per-workspace state gives zero-config concurrent workspace isolation.

**Findings**:
- 🟡 major (confidence: high) — **Success output format for dev is unspecified**
  (Phase 3 §2 step 9 / Phase 4): the printed lines are the only feedback after a
  detached return. Suggest a concrete labelled "ready" block asserted in manual
  verification.
- 🟡 major (confidence: high) — **Log files are discoverable only by convention;
  nothing on screen points to them** (What We're NOT Doing / Phase 3/4): the path
  appears only in the failure message. Suggest printing log paths on success and in
  non-HEALTHY status.
- 🟡 major (confidence: high) — **Exit-code contract (0/3/4) is undiscoverable**
  (Phase 4 §1 / Desired End State): no docstring/help/legend. Suggest stating the
  legend in the docstring + mise description.
- 🔵 minor (confidence: medium) — **Reuse messaging is unspecified** (Phase 3 §2):
  silent reuse reads as a no-op. Suggest an explicit reuse line.
- 🔵 minor (confidence: medium) — **Bare `mise run dev` mental model and the
  invoke/mise name split deserve help text** (tasks/__init__.py wiring): suggest a
  one-line mental model in the docstring + mise description.
- 🔵 minor (confidence: medium) — **dev:stop / partial-failure feedback under
  stale/recycled/timeout conditions is unspecified** (Phase 3 §3 / Phase 5):
  suggest human-readable messages for each teardown outcome.

## Re-Review (Pass 2) — 2026-06-06

**Verdict:** REVISE

Re-ran all 7 lenses against the revised plan. **Every Pass-1 critical, major, and
minor finding is resolved** by the revision: the stale-`server-info.json` gate is
deleted-before-launch, the macOS CI matrix and py3.14 wheel check enforce parity,
a per-workspace lock + bounded port retry + provisional-state close the
concurrency gaps, the orchestration is decomposed behind an injection seam, the
DX (success block, log paths, exit-code legend, reuse/stop messaging) is now
legible, and the test strategy is de-flaked with automated cross-workspace and
real-recycled-PID coverage. However — exactly as in the work-item re-review — two
of the *fixes themselves* introduced **new critical-severity gaps**: the
descendant-reaping added to guarantee no orphans signals child PIDs with no
identity gate (can kill a recycled process) and queries a dead arbiter's children
(which no longer exist after reparenting), and the cross-workspace endpoint guard
as worded does not actually verify the *responding* arbiter. Hence REVISE on the
pass. These are tractable and concentrated in Phase 3 §2–§3.

### Previously Identified Issues

- 🔴 **Correctness+Safety**: Stale `server-info.json` readiness gate — **Resolved**
  (Phase 3 §2 step 2 deletes the stale file + server pidfile before launch; new
  unit + integration + manual tests assert a stale file no longer satisfies the
  gate).
- 🔴 **Portability**: macOS/Linux parity never enforced in CI — **Resolved**
  (Phase 5 §2b adds the `[ubuntu-latest, macos-latest]` matrix; both legs are a
  success criterion) — *but see new majors: the matrix shape is left as an
  either/or, and the load-bearing npm-PATH behaviour isn't exercised on it.*
- 🟡 **Architecture/Correctness/Safety**: Free-port TOCTOU + no strict-port guard —
  **Resolved** (bounded re-allocate/retry + first-class bind-failure handling) —
  *new edge cases around the retry's partial-launch orphan remain (below).*
- 🟡 **Architecture**: No intra-workspace serialisation — **Resolved**
  (non-blocking per-workspace lock around the critical section).
- 🟡 **Correctness/Safety**: Unbounded pidfile poll / unrecoverable partial launch —
  **Resolved** (bounded `wait_for_file` poll + provisional-state-before-launch) —
  *residual: empty/partially-written pidfile parse; null-PID teardown (below).*
- 🟡 **Correctness**: PARTIAL conflates startup window — **Resolved** (lock makes
  the window unobservable externally; `do_status` labels it "(starting)").
- 🟡 **Correctness/Safety/Architecture**: `restart` = stop+up coupling — **Resolved**
  (explicit `clean`/`refused`/`survivor` contract; `up` invariant documented) —
  *new minor: `refused` semantics diverge between `do_stop` and `restart` (below).*
- 🟡 **Safety**: Fallback reaped only the arbiter PID — **Addressed but regressed**
  (descendant reaping added — but introduced the two new criticals/majors below).
- 🟡 **Code Quality**: `up` 9-step monolith / no injection seam — **Resolved for
  the seam** (injectable `bring_up`/`do_stop`/`do_status` + one-line adapters);
  **partially for complexity** (`bring_up` still carries the full launch sequence
  incl. the retry loop — below).
- 🟡 **Code Quality**: Teardown error handling unspecified — **Resolved** (tolerated-
  vs-fatal contract + best-effort removal) — *new minor: undefined logging sink.*
- 🟡 **Test Coverage**: Wall-clock-flaky tests — **Resolved** (exact poll counts in
  units; direction-only + small parametrised timeout in integration) — *residual:
  macOS-runner timing margin; the "avoid pytest-xdist" framing targets a
  non-existent mechanism (below).*
- 🟡 **Test Coverage**: Fake CircusClient fidelity — **Resolved** (wire shapes
  pinned + cross-checked against real circusd).
- 🟡 **Test Coverage**: Manual-only ACs — **Mostly resolved** (cross-workspace now
  automated) — *prerequisite-auto-run still manual; new `test:integration:dev`
  task omits `deps:install:python` (below).*
- 🟡 **Portability**: py3.14 wheel availability — **Resolved** (Phase 1 both-platform
  check + enforced on the matrix).
- 🟡 **Portability**: psutil tolerance cross-platform — **Resolved** (explicit
  argument, validated per-platform via the real-PID integration assertion).
- 🟡 **Portability**: circus detach/`stop_children` parity on macOS — **Resolved**
  (proven on the macOS leg) — *residual minor: CI-step vs real-interactive-shell
  detach.*
- 🟡 **Usability**: Success output / log paths / exit-code legend — **Resolved**
  (labelled success block; legend in docstrings) — *residual minors: `dev:status`
  mise description omits the legend; HEALTHY status hides log paths (below).*
- 🔵 All Pass-1 minors/suggestions (two-dir boundary, DevState typing, state-removal
  ordering, vendor lock-in ADR note, npm PATH, `wait_for_file` precision, etc.) —
  **Resolved** (each folded into the revision).

### New Issues Introduced

- 🔴 **Safety** (high): **Descendant reap has no identity gate** (Phase 3 §3) — the
  new orphan-reaping SIGKILLs snapshotted child PIDs without re-checking
  `(pid, create_time)`, so a child that exits during the grace window and has its
  PID recycled gets an unrelated process killed — reintroducing, on the
  highest-volume signalling path, the exact recycled-PID hazard the arbiter gate
  guards against. **Fix**: capture `(pid, create_time)` per descendant at snapshot
  and re-verify each before signalling (or hold live `psutil.Process` handles).
- 🔴 **Correctness** (high): **Cross-workspace endpoint guard doesn't verify the
  responder** (Implementation Approach / Phase 3 §2 step 1) — it checks the local
  recorded PID against the local pidfile, which only confirms *this workspace's*
  arbiter is alive; it does not establish that the arbiter answering on a
  (possibly collided) endpoint port is the same process. The stated guarantee
  ("a collision can never cause one workspace's `dev:stop` to quit another's") is
  not delivered. **Fix (recommended)**: bind the circus `endpoint`/`pubsub` to
  per-workspace **`ipc://` Unix-domain sockets** under `.accelerator/tmp/dev/`
  instead of TCP ports — the port space then cannot collide across workspaces at
  all, which also removes the endpoint free-port TOCTOU entirely (only the
  frontend port still needs allocation). Alternatively, have the arbiter
  self-identify (workspace UUID surfaced over the wire).
- 🟡 **Safety** (high): **Arbiter-already-dead reap queries a dead PID's children**
  (Phase 3 §3 + Phase 5 "arbiter already dead" test) — once the arbiter dies its
  children are reparented to init/launchd and are no longer `children()` of the
  recorded PID, so the snapshot finds nothing and the orphans leak; the
  integration test as written could only pass if the snapshot were taken while
  the arbiter was alive. **Fix**: persist the server/frontend watcher PIDs (and
  trees) into dev-state at readiness time, or snapshot descendants *before*
  `quit`, and reap reparented orphans by recorded PID+identity.
- 🟡 **Correctness/Safety** (medium): **Teardown not guarded for null/dead arbiter
  PID** (Phase 3 §3) — `psutil.Process(arbiter_pid).children()` is called "before
  any signalling" and before the identity gate; on the provisional null-PID state
  or a dead/recycled PID it raises (breaking the best-effort removal contract) or
  enumerates an unrelated process's children. **Fix**: run the identity gate
  first; only snapshot/reap when the PID is non-null and identity-confirmed.
- 🟡 **Correctness** (medium): **Bind-failure retry can orphan a half-started
  `circusd`** (Phase 3 §2 step 3) — a daemon that started but never wrote its
  pidfile leaves `arbiter_pid = null`, yet the only teardown is identity-gated
  direct kill (needs a PID). **Fix**: reap each failed attempt by the in-hand
  `circusd` subprocess handle / a final late pidfile read before the next attempt.
- 🟡 **Test Coverage** (high): **`test:integration:dev` mise task omits
  `depends = ["deps:install:python"]`** (Phase 5 §2) — every other pytest task
  declares it; without it the circus/psutil import fails on a clean runner
  (notably the new macOS leg). **Fix**: add the dependency.
- 🟡 **Portability/Test Coverage** (high): **macOS matrix is an either/or, not a
  concrete diff** (Phase 5 §2b) — "matrix the `test-unit`/`test-integration` jobs
  **or** a dedicated job" are materially different (the former drags every
  unrelated suite onto a 10×-priced macOS runner). **Fix**: pin one shape — a
  dedicated `test-cross-platform` job running exactly `test:unit:tasks` +
  `test:integration:dev` across both OSes — and name it in the success criteria.
- 🟡 **Portability** (high): **npm/node-PATH behaviour isn't exercised on the
  matrix** (Phase 5) — the load-bearing detached-daemon PATH resolution is left as
  a macOS *manual* note because the fakes avoid npm, so the enforced matrix never
  proves the one thing most likely to diverge. **Fix**: one integration case
  resolving a shim `npm` from a stripped PATH under the detached daemon.
- 🟡 **Code Quality** (medium): **`bring_up` still carries the whole launch
  sequence** incl. the bounded retry loop (Phase 3 §2) — the decomposition stopped
  at the task boundary. **Fix**: extract `reuse_or_teardown` and
  `allocate_and_launch` so `bring_up` reads as a short named sequence.
- 🟡 **Code Quality** (medium): **`teardown` responsibility/return contract is split
  from `do_stop`** (Phase 3) — unclear whether `teardown` returns the
  `clean`/`refused`/`survivor` result and whether the `bring_up` reuse-gate
  callers honour `survivor`/`refused`. **Fix**: have `teardown` return the result;
  all three callers consume it; reuse-gate must fail-fast on survivor.
- 🔵 Minors (new): `refused` keep-vs-clear divergence between `do_stop` and
  `restart`; mkdir-fallback stale lock has no documented reclaim/recovery; empty/
  partially-written pidfile parse; `Result` type referenced but unspecified; wide
  keyword-injection signatures (consider a `DevDeps` dataclass); `_write_server_config`
  is underscore-private yet called cross-module from `dev_supervisor`; undefined
  logging sink for teardown diagnostics; "avoid pytest-xdist" framing targets a
  mechanism the project doesn't configure (real contention is the mise-level
  parallel aggregate); missing automated cases for the lock-held-reuse branch, the
  "(starting)" status label, the prerequisite-auto-run AC, and a split
  retry-succeeds-vs-exhausts assertion; `dev:status` mise description omits the
  exit-code legend; HEALTHY `dev:status` hides log paths; lock-held and survivor
  messages give thin recovery guidance; reuse line should state "changes are NOT
  live" explicitly; macOS-runner timing margins; CI-step vs interactive-shell
  detach; circus-agnostic boundary aspirational (consider a `Supervisor` protocol);
  `server.log` filename ambiguity across the ownership boundary; single-stack-per-
  workspace as an unstated design limit.

### Assessment

The revision cleared **100% of the Pass-1 findings** — the plan is materially
stronger and the previously-broken core behaviours (stale-gate, parity
enforcement, concurrency, orchestration testability, DX) are now sound. The
re-review verdict is REVISE only because two of the new mitigations
(descendant-reaping and the cross-workspace guard) introduced fresh
critical-severity gaps, plus a small cluster of majors around the retry/teardown
edge cases and the CI wiring concreteness. All are tractable, localised to
Phase 3 §2–§3 and Phase 5 §2, and the recommended `ipc://` endpoint switch would
simultaneously resolve the cross-workspace-guard critical and shrink the entire
free-port-TOCTOU surface. One more focused edit pass should land the plan at
APPROVE.

## Re-Review (Pass 3) — 2026-06-06

**Verdict:** REVISE

Re-ran all 7 lenses against the twice-revised plan. **Both Pass-2 criticals and
every Pass-2 major/minor are resolved**: the `ipc://` socket switch made
cross-workspace isolation structural (and dissolved the endpoint free-port TOCTOU
+ retry machinery), per-descendant identity gating + watcher-PID-based orphan
reaping fixed the unsafe reap, and the CI/test-wiring/decomposition/DX majors all
landed. The plan's core is now genuinely implementation-ready. But the pattern
from Passes 1–2 repeated once more: the new mechanisms surfaced a **thinner,
narrower layer** of findings — one correctness critical (circusd's double-fork
defeats the reap-by-subprocess-handle) plus a cluster of majors concentrated in
the `ipc://`/`$TMPDIR` introduction and two real test/factual gaps. These are
mechanical and local, not structural. **Convergence is clear** — each pass'
findings are smaller and more isolated than the last — but it is not yet APPROVE.

### Previously Identified Issues (Pass 2)

- 🔴 **Safety**: Descendant reap had no identity gate — **Resolved**
  (per-descendant `(pid, create_time)` re-verification before signalling).
- 🔴 **Correctness**: Cross-workspace endpoint guard didn't verify the responder —
  **Resolved** (per-workspace `ipc://` sockets; collision structurally impossible).
- 🟡 **Safety**: Arbiter-already-dead reap queried a dead PID's children —
  **Resolved** (reap via persisted `server_pid`/`frontend_pid`) — *but a new
  window remains before those PIDs are recorded; see below.*
- 🟡 **Correctness/Safety**: Teardown unguarded for null/dead PID — **Resolved**
  (identity established first, `NoSuchProcess`-wrapped).
- 🟡 **Correctness**: Bind-failure retry could orphan a half-started daemon —
  **Resolved at the design level** (no retry; reap-by-handle) — *but the reap
  mechanism is itself flawed by double-fork; see new critical.*
- 🟡 **Test/Portability**: `deps:install:python` missing; macOS matrix an
  either/or; npm-PATH not on the matrix — **all Resolved** (dependency added;
  dedicated `test-cross-platform` job pinned; PATH integration case added).
- 🟡 **Code Quality**: `bring_up` monolith / `teardown` return contract —
  **Resolved** (decomposed into named helpers; `teardown` returns `StopResult`
  consumed by all three call sites; `Supervisor` protocol + `DevDeps` added).
- 🔵 All Pass-2 minors (refused consistency, `Result` types, dev:status legend,
  HEALTHY log paths, reuse wording, empty-pidfile parse, config-helper ownership,
  diagnostic sink, de-flaking framing, single-stack note, mkdir reclaim,
  interactive-shell detach) — **Resolved**.

### New Issues Introduced / Surfaced

- 🔴 **Correctness** (medium): **`circusd --daemon` double-forks, so the retained
  subprocess handle is not the real daemon** (Phase 3 §2 step 3c–3e). On the
  startup-failure path the handle reaps the short-lived intermediate, not the
  detached `circusd` written to the pidfile — if circusd detaches then crashes
  before a usable pidfile, an orphaned arbiter survives, undiscoverable.
  **Fix**: launch without `--daemon` and self-detach (handle = real PID), or on
  failure read/scan the pidfile + recorded `ipc://` endpoint to reap; assert
  no-orphan in the Phase 5 daemon-startup-failure case.
- 🟡 **Safety** (high): **Deleting the `ipc://` socket on a `survivor` severs the
  only clean control channel** (Phase 3 §3 error contract). "Best-effort removal
  of all artifacts" conflicts with "keep state on survivor": unlinking a live-but-
  wedged arbiter's sockets makes every later `dev:stop` fall through to a direct
  SIGKILL, orphaning children. **Fix**: skip socket+pidfile removal on
  `survivor`/`refused`; only unlink sockets once the arbiter is confirmed dead.
- 🟡 **Correctness/Safety** (high/medium): **Arbiter death before watcher PIDs are
  recorded leaves an unreachable orphan tree** (Phase 3 §2 step 5). PIDs are
  persisted only once *both* watchers are active; a crash in the launch→record
  window leaves `server_pid`/`frontend_pid` null and the (dead) arbiter
  un-enumerable. **Fix**: record each watcher PID incrementally as it goes active
  (server PID right after the readiness gate), not only once both are up.
- 🟡 **Correctness** (medium): **Pre-quit descendant snapshot misses grandchildren
  spawned during the grace window** (Phase 3 §3). Vite spawns esbuild/node workers
  lazily; reaping only the pre-quit snapshot can miss late workers. **Fix**: after
  the grace window, re-enumerate live descendants of the recorded watcher PIDs and
  identity-gate-reap those, using the snapshot only as the recycled-PID baseline.
- 🟡 **Architecture** (medium): **dev-state is a single point of failure for
  discovery, and the `ipc://` paths aren't reconstructible without it** (Impl
  Approach; Phase 2 §3). A truncated/partial state file orphans a live stack since
  the `$TMPDIR`-hashed sockets can't be found. **Fix**: make `ipc_socket_paths`
  deterministic from the workspace root so `dev:stop`/`dev:status` can recompute +
  probe them when dev-state is missing — treat the file as a cache, not the sole
  truth.
- 🟡 **Portability** (high): **`$TMPDIR`-hashed socket base may still breach the
  macOS `sun_path` ~104-byte limit** (Overview note; Phase 2 §1). macOS `$TMPDIR`
  is a deep `/var/folders/...` path; the plan never budgets the worst-case length.
  **Fix**: cap the hash width (e.g. 12 hex), compute the worst-case path against
  the macOS limit in the design, and hard-error (not silently truncate) if exceeded.
- 🟡 **Portability** (medium): **Unset `$TMPDIR` has no fallback** (Phase 2 §1).
  Common on Linux (cron/systemd/containers/some CI). **Fix**: resolve via
  `tempfile.gettempdir()` (honours `$TMPDIR` then falls back to `/tmp`); unit-test
  the unset case.
- 🟡 **Test Coverage** (high): **`dev:restart` AC has no integration case** (Phase
  5). The most race-prone seam (stop→start round-trip) is only unit-tested.
  **Fix**: add a real-circusd restart case (assert new arbiter PID, both active,
  fresh frontend port).
- 🟡 **Test Coverage** (medium): **No test exercises the real Vite-reads-
  `server-info.json` `/api` proxy** that AC1/AC6 promise (Phase 5 uses fakes for
  both processes). **Fix**: add one thin real-Vite proxy-target assertion, or
  narrow the AC wording to "the ordering gate fires + the watcher receives the
  resolved port" (what is actually tested).
- 🟡 **Usability** (high) / **Test** (medium): **Factual error — the plan claims
  today's `dev.server` "passes no `--log-file`", but `tasks/dev.py:34` does**, and
  two `server.log` files (`dev-server/` vs `dev/`) share a basename. The
  single-writer/ownership note rests on a false premise. **Fix**: correct the
  claim and either confirm the unified path genuinely launches the server without
  `--log-file` (then `dev/server.log` capture is fine) or disambiguate the names;
  add a unit assertion that the unified launch omits `--log-file`.
- 🔵 Minors (new): `test-cross-platform` not in the prerelease/release `needs` (so
  a red macOS leg doesn't gate — flagged by test **and** portability); the
  `(starting)` status label persists when the frontend has actually died (gate it
  on "watcher PIDs never recorded"/within-`up`-window, not just "port populated");
  reuse-line wording inconsistent across three places (pin one verbatim string);
  `dev.log` has no discovery route in printed output (name it in survivor/refused
  recovery messages); the teardown grace/escalation wait should reuse the injected
  clock for deterministic unit tests; `teardown` is still a long multi-branch
  function (decompose like `bring_up`); `dev_supervisor.py` is a wide module
  (consider `tasks/shared/` for the pure helpers); config-render/mkdir ownership
  still spans the `dev.py`↔`dev_supervisor.py` seam; hashed socket-base
  collision/cleanup semantics; resolve npm/node to absolute paths to survive PATH
  drift; prerequisite-auto-run AC behavioural half still manual-only.

### Assessment

Three passes in, the plan has cleared **100% of every prior finding** and its
architecture is sound and well-tested by design. The recurring dynamic — each
revision resolving the last layer and exposing a thinner one — is now clearly
**converging**: Pass 3's findings are markedly narrower (a double-fork mechanic,
a socket-path length budget, two test gaps, a factual log-file error) than Pass
1's structural gaps. The one item that genuinely must be fixed before
implementation is the **double-fork reap** (it can orphan an arbiter on the
failure path); the `survivor`-socket-deletion and watcher-PID-window safety
majors and the `$TMPDIR`/`sun_path` portability majors are real but mechanical;
the test gaps (restart, real `/api` proxy) and the `--log-file` factual error
should be corrected for accuracy. A fourth focused edit pass — or simply handing
these to the implementer as a tracked checklist — would close them. This is the
point of **diminishing returns on further full review cycles**: the remaining
findings are implementation-detail-level and individually small.

## Re-Review (Pass 4) — 2026-06-06

**Verdict:** APPROVE

Re-ran all 7 lenses against the thrice-revised plan, with reviewers explicitly
instructed to be calibrated (three thorough passes already; raise only material,
not-already-addressed findings). The result confirms **convergence**: **no
criticals, no structural majors, a single medium-confidence correctness major,
and the rest minor/mechanical** — and notably, four lenses independently flagged
the *same* stale doc line (the ADR brief still naming the rejected
`circusd --daemon`), the clearest signal that the substantive design space is
exhausted and what remains is wording/consistency. Under the configured
thresholds (REVISE requires a critical or ≥3 majors) this pass scored **COMMENT**;
all Pass-4 findings were then **addressed by follow-up edits** (below), so the
verdict is updated to **APPROVE**.

### Previously Identified Issues (Pass 3)

- 🔴 **Correctness**: circusd double-fork defeats reap-by-handle — **Resolved**
  (launch plain `circusd` without `--daemon` via `Popen(start_new_session=True)`;
  handle = real arbiter; verified by all lenses this pass).
- 🟡 **Safety**: deleting the `ipc://` socket on `survivor` — **Resolved**
  (sockets+pidfile retained unless the arbiter is confirmed dead).
- 🟡 **Correctness/Safety**: arbiter death before watcher PIDs recorded —
  **Resolved** (incremental per-watcher recording: server PID after the readiness
  gate, frontend PID after `start_frontend`).
- 🟡 **Correctness**: pre-quit snapshot misses grandchildren — **Resolved**
  (post-grace re-enumeration of recorded watcher descendants).
- 🟡 **Architecture**: dev-state single point of failure — **Resolved**
  (deterministic recomputable `ipc_socket_paths`; state file is a cache).
- 🟡 **Portability**: `sun_path` budget / unset `$TMPDIR` — **Resolved**
  (`tempfile.gettempdir()`, 12-hex hash, worst-case computed + hard-error).
- 🟡 **Test**: no restart / lost-state / real-proxy coverage — **Resolved**
  (restart round-trip + lost-state-file discovery + port/info wiring cases added;
  real `/api` proxy explicitly scoped to manual with AC wording narrowed).
- 🟡 **Usability/Test**: `--log-file` factual error — **Resolved** (corrected;
  `server.log` single-writer disambiguated; unit assertion added).
- 🔵 All Pass-3 minors (test-cross-platform gating, `(starting)` gate, reuse-string
  pinning, `dev.log` discovery, teardown decomposition, `DevDeps` paths, etc.) —
  **Resolved**.

### New Issues Introduced / Surfaced (all addressed)

- 🟡 **Correctness** (medium): **`start_frontend` had an unbounded "reports
  active" wait with undefined behaviour on a frontend that starts then dies**
  (`respawn = false`; `send_message("start")` only confirms acceptance). The one
  genuine major. **Addressed**: step 5 now bounds the transition (injected-clock
  poll), routes a timeout / observed `active`→`stopped` to `teardown` +
  `invoke.Exit` naming `frontend.log`, and records the frontend PID only once
  `active` is confirmed.
- 🔵 **Doc consistency** (flagged by architecture + correctness + code-quality +
  usability): the **ADR brief still named `circusd --daemon`**. **Addressed**:
  Phase 1 §2 now records self-detach (`Popen(start_new_session=True)`, no
  `--daemon`) and notes *why* `--daemon` was rejected.
- 🔵 **Test/Usability**: the **Phase 4 success-criteria bullet still described the
  old "frontend port populated" `(starting)` gate**. **Addressed**: reworded to
  the frontend-PID-never-recorded gate + the dead-frontend-renders-degraded case.
- 🔵 **Safety** (medium): null-arbiter-PID death confirmation (interrupted-launch
  provisional state) was under-specified. **Addressed**: null-PID fallback confirms
  death by re-probing the endpoint (channel-gone == dead).
- 🔵 **Safety** (medium): lock-free `dev:stop` probe-then-unlink sub-window vs a
  concurrent same-workspace launch. **Addressed**: stale-socket unlink now
  opportunistically takes the non-blocking lock and skips if it can't acquire it.
- 🔵 **Portability**: release-gate wording (release `needs: prerelease` only);
  aarch64-Linux wheels unverified. **Addressed**: corrected to gate on
  `prerelease.needs`; aarch64-Linux noted as a known-unverified target.
- 🔵 **Architecture/Code Quality**: pure helpers' placement left deferred.
  **Addressed**: committed to placing the circus-agnostic pure helpers under
  `tasks/shared/` so the core/shell split is a file boundary.
- 🔵 **Test**: AC3 per-process log routing only checked non-empty. **Addressed**:
  the detach case now asserts each process's distinguishable marker appears only
  in its own log.

### Assessment

Four passes have driven the plan from genuinely risky to **approved for
implementation**. Pass 4 is the convergence inflection: zero criticals, one
medium major (now fixed), and a tail of consistency/wording items that multiple
lenses redundantly surfaced — the signature of a search that has bottomed out.
The remaining open items recorded by reviewers but intentionally *not* changed are
accepted, non-blocking residuals appropriate to a local dev tool: the irreducible
sub-millisecond read→kill (and probe→unlink) TOCTOUs, and the real Vite `/api`
proxy being a manual rather than automated check (the fake-process harness cannot
boot Vite). The plan is internally consistent, the safety/teardown story is sound
on every path analysed, cross-platform parity is enforced in CI, and every
acceptance criterion maps to coverage. **Verdict: APPROVE — ready for
`/implement-plan`.**
