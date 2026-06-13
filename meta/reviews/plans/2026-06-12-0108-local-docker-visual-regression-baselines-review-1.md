---
type: plan-review
id: "2026-06-12-0108-local-docker-visual-regression-baselines-review-1"
title: "Plan Review: Local Docker-Based Visual Regression Baseline Generation"
date: "2026-06-12T22:05:05+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
parent: "plan:2026-06-12-0108-local-docker-visual-regression-baselines"
target: "plan:2026-06-12-0108-local-docker-visual-regression-baselines"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "portability", "usability", "documentation", "safety"]
review_number: 1
review_pass: 3
tags: ["visual-regression", "testing", "ci", "docker", "playwright"]
last_updated: "2026-06-13T07:15:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Local Docker-Based Visual Regression Baseline Generation

**Verdict:** REVISE

This is a strong, unusually well-researched plan: the architectural keystone
(containerise only Chromium, run the Rust server on the host over
`host.docker.internal`) is the right seam, the phase decomposition is genuinely
atomic and reversible, and the deterministic logic is factored into pure,
unit-testable helpers. Two issues block approval as-is. First, a **critical**
spec-routing error flagged independently by the architecture and correctness
lenses: moving the entire `visual-regression` project out of the main config
orphans the ~14 `*-resolved-*` computed-style specs that physically live in
`tests/visual-regression/`, so they silently stop running on every native and
matrix CI leg — directly contradicting the plan's "they run natively unchanged"
claim. Second, a cluster of **major** findings around the single-source-of-truth
guarantee (Chromium `channel` and locale are hardcoded in the TS config and
duplicated, not consumed, from the shared module), `pty=True` under headless CI,
the absence of a Docker pre-flight check, and untested server-lifecycle
orchestration. None require rethinking the architecture; they are corrections
and hardening to the Phase 1/2 mechanics.

### Cross-Cutting Themes

- **Resolved specs orphaned by the project move** (flagged by: architecture,
  correctness) — The `*-resolved-*` / non-screenshot specs sit in
  `tests/visual-regression/`. Only the `visual-regression` project (testDir
  `./tests/visual-regression`) runs that directory natively; Phase 2 deletes
  that project, and the remaining `chromium` project's testDir is `./e2e`. The
  new Docker config picks them up instead — so they move from "native, every
  run" to "Docker-only", losing coverage the plan explicitly intends to keep.
  This is the headline issue.

- **Single-source-of-truth for channel/locale is not actually realised**
  (flagged by: architecture, correctness, test-coverage, documentation) — Only
  the image tag flows from `tasks/shared/playwright.py`. `CHROMIUM_CHANNEL` is
  defined there but unused (the plan's own note concedes this); the TS config
  hardcodes `channel: "chromium"` and `locale: "en-US"`. The work item's
  no-drift acceptance criteria (0108:132-140) are therefore not met as designed
  — two files must be hand-kept in sync.

- **`pty=True` under a headless CI runner** (flagged by: correctness,
  portability) — The visual job gates `prerelease`; if Invoke's pty path
  mis-propagates the container exit code on a TTY-less runner, a failing
  zero-diff could be reported as green, defeating the gate.

- **No graceful failure when Docker is unavailable** (flagged by: portability,
  usability) — The task spends up to 60s bringing up the host server before
  `docker run` fails with a raw daemon error. For the all-macOS team (Docker is
  developer-provided and often not started), this is the most common first
  failure and it is slow and low-signal.

- **Server-lifecycle orchestration is inline, hard to test, and untested**
  (flagged by: code-quality, test-coverage) — The hand-rolled Popen + poll +
  SIGTERM/kill logic duplicates existing injectable helpers
  (`tasks/shared/polling.py:wait_for_file`, `processes.py:ProcessOps`,
  `dev/lifecycle.py`) that are already unit-tested with fakes, yet the plan
  dismisses this risk-bearing code as "inherently integration/manual-verified".

- **Kanban clean-fixture invariant loses its enforcement** (flagged by:
  architecture, correctness, test-coverage) — Dropping
  `dependencies: ["visual-regression"]` is defensible, but the ordering
  guarantee now rests entirely on global-setup/teardown restore across two
  separately-invoked configs, verified only by a manual happy-path check.

### Tradeoff Analysis

- **Correctness/no-drift vs simplicity (channel & locale)**: Threading
  `channel`/`locale` through env from the shared module satisfies the
  acceptance criteria but adds an env hop; hardcoding in TS is simpler but
  duplicates the value. Recommendation: pass them through env (mirroring how
  `BASE_URL` already flows) so the criteria are genuinely met — the cost is one
  line each.

- **Least-privilege vs convenience (repo mount)**: Mounting `{REPO_ROOT}` is
  simpler (one path, node_modules-mask path is repo-relative) but gives the
  container write access to the whole tree; mounting only `{FRONTEND}` is
  tighter. Given VCS revert is the team's recovery path, this stays a
  suggestion, not a blocker.

### Findings

#### Critical

- 🔴 **Architecture / Correctness**: Removing the `visual-regression` project
  orphans the non-screenshot specs that share its testDir
  **Location**: Phase 2 §1 (move the visual-regression project) + "What We're
  NOT Doing"
  The ~14 `*-resolved-*` specs (plus `fixture-coverage.spec.ts`,
  `non-regression-glyph-consumers.spec.ts`, `eyebrow-unification-resolved.spec.ts`)
  live in `tests/visual-regression/`. After Phase 2 no native-config project
  targets that directory (`chromium` uses testDir `./e2e`), so these
  computed-style specs stop executing natively and in the matrix CI legs, while
  the Docker config (testDir `./tests/visual-regression`, `--project
  visual-regression`) unexpectedly runs them in the container. The plan's claim
  that they "run natively unchanged" is false, and the gap passes every
  green-suite success criterion because absent specs report success.

#### Major

- 🟡 **Architecture / Correctness**: Channel and locale are duplicated, not
  consumed from the single source of truth
  **Location**: Phase 1 §1 (shared source) vs §2 (Docker config) and §4 (task)
  Only the image tag flows from `tasks/shared/playwright.py`. `CHROMIUM_CHANNEL`
  is unused; the TS config hardcodes `channel: "chromium"` and `locale:
  "en-US"`. The work item's no-drift criteria (0108:132-140) are not met — a
  future bump can update one side and silently leave the other stale.

- 🟡 **Architecture / Portability**: Locale value diverges between the shared
  module (`C.UTF-8`) and the config (`en-US`)
  **Location**: Phase 1 §1 (`E2E_LANG`) vs §2 (`locale: "en-US"`)
  The host server and shell get `LANG`/`LC_ALL=C.UTF-8` while Chromium gets
  `locale: "en-US"` — two different locale concepts carrying two different
  values, so "identical locale value from the same shared source" is not
  satisfied. Server-emitted formatting (under C.UTF-8) and browser rendering
  (under en-US) can disagree; relative-time masking only partly shields this.

- 🟡 **Architecture / Correctness / Test-Coverage**: Kanban clean-fixture
  invariant loses its enforcement once the snapshot moves to Docker
  **Location**: Phase 2 §1 (drop `dependencies: ["visual-regression"]`)
  The two halves of what was one ordered, single-worker invariant (clean-fixture
  capture vs mutating drag tests) now live in two independently-invoked configs
  with no shared ordering contract. If a native run is interrupted between
  mutation and teardown-restore, a later Docker rebaseline could capture dirty
  fixtures into the canonical set. Verified only by a manual happy-path bullet.

- 🟡 **Correctness / Portability**: `pty=True` under a no-TTY CI runner may hang
  or mis-report the docker exit code
  **Location**: Phase 1 §4 (`context.run(..., pty=True)`) + Phase 2 §3 (CI job)
  GitHub's headless runner has no controlling TTY; Invoke's pty path differs
  from the piped path and can distort exit-status propagation. The visual job
  gates `prerelease`, so a swallowed non-zero could let a failing zero-diff ship
  as green.

- 🟡 **Portability / Usability**: No Docker-daemon pre-flight; failure is slow
  and cryptic
  **Location**: Phase 1 §4 (orchestrating invoke task)
  The task brings up the host server (up to 60s) before `docker run` fails with
  a raw daemon error, and leaks a started-then-killed server in between. The
  most common first failure for the all-macOS team is opaque and slow.

- 🟡 **Code-Quality / Test-Coverage**: Inline Popen+SIGTERM server lifecycle is
  hard to test and duplicates existing injectable helpers
  **Location**: Phase 1 §4 (orchestrating invoke task) + Implementation Approach
  The hand-rolled spawn / `while not port_file.exists()` poll / SIGTERM-then-kill
  teardown reimplements `wait_for_file`, `ProcessOps`, and the `dev/lifecycle.py`
  pattern that are already unit-tested with `FakeProcs`/`FakeClock`. The
  highest-risk new code (startup races, teardown-on-error, the 60s deadline) has
  no test seam, and the "inherently manual-verified" justification contradicts
  the project's own demonstrated test architecture.

- 🟡 **Code-Quality**: Two path-sourcing idioms coexist in `tasks/test/e2e.py`
  **Location**: Phase 1 §4 (imports) vs existing `tasks/test/e2e.py:3,14-18`
  The new task imports `from tasks.shared.paths import FRONTEND, REPO_ROOT,
  SERVER` (the dominant convention) while the existing `visualiser` task in the
  same file derives paths from `from .helpers import repo_root`. Two ways to
  answer "where is the frontend?" side by side invites drift.

- 🟡 **Portability**: Emulated-amd64 (Apple Silicon) vs native-amd64 (CI)
  rendering parity is asserted, not proven
  **Location**: Phase 2 Manual Verification + Performance Considerations
  The single-baseline promise rests on QEMU-emulated amd64 producing
  byte-identical Chromium/Skia/PNG output to native amd64, checked only by one
  late manual step. QEMU TCG can differ in FP/SIMD codepaths feeding
  antialiasing — the exact sub-pixel delta the plan refuses to absorb with
  tolerance. If they diverge, local `:docker:update` could commit baselines that
  fail native-amd64 CI, reintroducing the mismatch this work eliminates.

- 🟡 **Usability**: Silent skip of visual specs invites a green-but-stale commit
  **Location**: Phase 2/3 (native runs skip visual specs)
  `mise run test`/`test:e2e` run no visual specs and don't depend on the
  `:docker` tasks, so a developer who sees native green gets no signal that
  baselines are stale — they discover it at push time (the pain the work item
  set out to remove, merely relocated for anyone who forgets the opt-in step).

- 🟡 **Safety**: Phase 2 baseline regeneration is validated only tautologically
  **Location**: Phase 2 §2 (regenerate and collapse the baseline set)
  The only correctness check on the new pixels is a zero-diff compare against the
  just-generated set; the outgoing darwin/linux references are deleted in the
  same commit. A silently-wrong regeneration (wrong fixtures, a spec that errored
  and skipped its snapshot, an emulation/font glitch, a partial run) becomes the
  canonical reference undetected. (Per team stance, no confirm/dry-run UX is
  expected — the ask is a count assertion + sample inspection, not a prompt.)

- 🟡 **Documentation**: README rewrite does not capture the "why" of the
  host-server/container split
  **Location**: Phase 3 §3 (contributor documentation)
  With the ADR deferred, the load-bearing rationale (baseline is a function of
  what Chromium renders; only the browser is containerised; `--network=host` is
  avoided locally) lives only in the plan/research — point-in-time artefacts a
  future maintainer editing the Docker task is unlikely to consult. Risk: someone
  "simplifies" it back into a fully-containerised path, reviving the Rust-in-image
  problem.

#### Minor

- 🔵 **Code-Quality / Documentation**: Dead `CHROMIUM_CHANNEL` import retained
  "in case"
  **Location**: Phase 1 §4 (imports + Note callout)
  Unused import → ruff F401 failure under `build-system:check`, and it documents
  a latent split source of truth rather than resolving it. Drop it (keep the
  constant exported) or wire the channel through env.

- 🔵 **Code-Quality**: Long f-string `docker_visual_command` is brittle around
  nested shell quoting
  **Location**: Phase 1 §4 (`docker_visual_command`)
  ~10 concatenated f-string fragments mixing interpolated paths inside
  double-quotes and an inner `bash -c "..."`. Consider assembling args as a list
  and `shlex.join` (as `tasks/github.py:68` already does), keeping only the inner
  payload as a string — also makes the unit tests assert on discrete tokens.

- 🔵 **Code-Quality**: Teardown can mask the underlying failure
  **Location**: Phase 1 §4 (finally block)
  The `RuntimeError("host server did not publish .e2e-port")` drops the server's
  returncode/stderr and collapses "server crashed" vs "timed out" into one
  message. Include `server.poll()` and reference the captured log so the 3am-CI
  root cause is one read away.

- 🔵 **Correctness / Documentation**: `baseURL` fallback `http://host.docker.internal:0`
  is an invalid origin
  **Location**: Phase 1 §2 (Docker config baseURL fallback)
  Port 0 is non-connectable; if BASE_URL is ever unset, every `page.goto` fails
  opaquely instead of failing fast on a clear "BASE_URL not set" error. Drop the
  fallback or make it throw. The inline comment ("set by the docker invoke task")
  also disagrees with the dead `:0` default.

- 🔵 **Correctness**: Anonymous node_modules volume re-creates empty each run;
  host-intact guarantee depends on the dir pre-existing
  **Location**: Phase 1 §4 (node_modules mask)
  Behaviour is correct in steady state (after `build:frontend` installs), but the
  first-ever-run edge case differs from what the manual check assumes. State the
  precondition; no code change needed given the `depends` chain.

- 🔵 **Test-Coverage**: Lockfile-extraction edge cases beyond one malformed case
  are unspecified
  **Location**: Phase 1 §1 + §6
  Missing file / invalid JSON / missing `packages` / missing entry / missing
  `version` each raise differently, and a non-semver-clean resolved version could
  yield a malformed MCR tag (`v1.60.0-beta.1-noble`) failing opaquely at `docker
  run`. Enumerate the inputs; assert a clear error or correct tag encoding.

- 🔵 **Test-Coverage**: node_modules-clobber protection and resolved-spec native
  survival verified only manually
  **Location**: Phase 1 Manual Verification + Phase 2
  Complement the mask-token presence unit assertion with a CI check that no
  Linux-native binary landed in node_modules, and add an automated assertion that
  the resolved specs are still collected by the native `playwright test`
  post-relocation.

- 🔵 **Portability**: `-noble` flavour and MCR tag existence are hardcoded with
  no clear-error fallback
  **Location**: Phase 1 §1 (`playwright_image`) + work item Dependencies (MCR)
  A routine version bump silently changes the required tag; if MCR lacks a
  `-noble` build for that version, everyone breaks with "manifest unknown". The
  risk is accepted, but surface an actionable error and note in
  README/Migration that bumping `@playwright/test` requires confirming the tag
  exists.

- 🔵 **Portability**: Colima `host.docker.internal` DNS-regression fallback is
  documented but unimplemented, with no interim escape hatch
  **Location**: Migration Notes (Colima fallback) + Key Discoveries
  A single Colima point-release regression could block the whole internal team's
  local re-baselining at once. Keep the sidecar unimplemented, but document a
  quick interim workaround (env-overridable BASE_URL host, or pin a known-good
  Colima version).

- 🔵 **Usability / Documentation**: Diff/trace artefact location is promised in
  docs but not surfaced on failure, and the path isn't grounded
  **Location**: Phase 3 §3 (debugging bullet) + Phase 1 §2 (`trace`)
  On non-zero exit, print the absolute host path to `test-results/` diffs/traces
  and the `npx playwright show-trace` hint; confirm `test-results/` is not masked
  by the node_modules volume so the artefacts survive on the host.

- 🔵 **Usability**: Default path (`npm ci` under amd64 emulation) is the slowest,
  every run; the fast path is only a comment
  **Location**: Performance Considerations + Phase 1 §4
  Ship the named-volume node_modules cache as an opt-in flag from the start
  (still defaulting to the anonymous mask for correctness) so iterating on a diff
  isn't gated behind a full emulated `npm ci` each cycle.

- 🔵 **Usability**: README still frames Docker as optional and baselines as
  dual-platform
  **Location**: Phase 3 §3 (Prerequisites / Project Layout updates)
  Verify the rewrite flips Prerequisites to "Docker required for visual work"
  (with a concrete macOS daemon suggestion) and updates the "darwin + linux"
  layout line — and leads with the two task names and when to use each, as the
  onboarding entry path rather than buried prose.

- 🔵 **Safety**: Launcher teardown can orphan the Rust child + bound port
  **Location**: Phase 1 §4 (visualiser_docker teardown)
  The `finally` SIGTERM/kill targets only the node launcher; if it is hard-killed
  after the 10s timeout, its forwarded SIGTERM never runs and the Rust child can
  be orphaned holding the port. Spawn in its own process group and signal the
  group; optionally unlink `.e2e-port` in `finally`.

- 🔵 **Safety**: `E2E_SERVER_HOST=0.0.0.0` exposes the dev server on all
  interfaces during a run
  **Location**: Phase 1 §3 (server bind change)
  Correctly contained (native default stays loopback), but confirm `0.0.0.0` is
  genuinely required vs the bridge gateway address; if so, document the transient
  all-interfaces exposure as a deliberate trade-off.

- 🔵 **Safety**: New CI visual job has no `timeout-minutes`
  **Location**: Phase 2 §3 (test-visual-regression job)
  A hung emulated Chromium run would rely on the 6-hour default before failing.
  Set a `timeout-minutes` proportional to expected runtime (per-test timeout is
  already 30s).

#### Suggestions

- 🔵 **Safety**: Mount only `{FRONTEND}` rather than `{REPO_ROOT}`
  **Location**: Phase 1 §4 (`-v "{REPO_ROOT}:/work"`)
  The run only needs the frontend dir; mounting the whole repo read-write widens
  the blast radius of an in-container mistake. Recoverable via revert, so a
  suggestion — but `-v "{FRONTEND}:/work"` with `-w /work` is tighter. (Note: the
  Docker config has no globalSetup, so the container does not even read
  `.e2e-port`; it only needs frontend specs/config/baselines.)

- 🔵 **Usability**: Verb-less compare task name is less self-evident than its
  `:update` sibling
  **Location**: Phase 1 §5 (mise task naming)
  `...:docker` conveys its compare mode only via the description. Likely fine
  given the `:update` pairing is a recognised convention; consider a leading verb
  in the description.

- 🔵 **Architecture**: Glyph convergence quietly changes the glyph specs'
  rendering substrate (`vite preview` → Rust dev-frontend server)
  **Location**: Phase 3 §2 (remove the glyph carve-out)
  Both serve the same built `dist/`, so pixels should match, but confirm a
  zero-diff for the glyph baselines through the unified server-served flow before
  deleting `playwright.glyph.config.ts`.

- 🔵 **Architecture**: Server-lifecycle/networking is a single point of failure
  with no degradation path wired
  **Location**: Phase 1 §4 + Migration Notes (fallback)
  Add a pre-flight reachability probe distinguishing "server down" from
  "container can't reach host", so the documented sidecar fallback has a concrete
  trigger signal.

### Strengths

- ✅ The host-server / containerised-Chromium split is the right architectural
  seam: it keeps the server build path identical across local and CI, avoids a
  Rust toolchain in the image, and localises nondeterminism to the one component
  (Chromium) that produces the committed pixels.
- ✅ The `host.docker.internal` + `--add-host=host.docker.internal:host-gateway`
  mechanism is validated as the single combination uniform across Colima, Docker
  Desktop, and Linux CI, with the `--network=host` trap (Lima VM namespace)
  explicitly called out and avoided.
- ✅ Phase decomposition respects atomicity: Phase 1 is genuinely inert/additive,
  Phase 2 is the irreducible cutover, and the old push-back workflow is retained
  until the new job is green on `main` — no window where neither path works.
- ✅ Command assembly is factored into a pure `docker_visual_command` helper,
  TDD'd without Docker; version/platform/locale pins are centralised; helpers
  take injectable `frontend: Path = FRONTEND` params mirroring `paths.py`.
- ✅ The node_modules clobber hazard is designed away structurally with an
  anonymous volume mask (not a "remember to re-`npm ci`" footgun), with an
  explicit host-intact verification step.
- ✅ The `:update` suffix is a least-surprise rebaseline convention; a single
  `update` boolean keeps the invoke surface minimal; both tasks carry clear
  descriptions and discoverability is verified.
- ✅ `prerelease.needs` gating correctly fail-safes the release; the back-compat
  server bind change (`?? "127.0.0.1"`) preserves native behaviour; teardown runs
  in all paths via try/finally.

### Recommended Changes

1. **Keep the non-screenshot specs running natively** (addresses: Removing the
   visual-regression project orphans the non-screenshot specs)
   Retain a project in `playwright.config.ts` that targets `tests/visual-regression/`
   scoped via `testMatch`/`grep` to the `*-resolved-*` / non-snapshot specs, and
   scope `playwright.docker.config.ts` to only the 8 screenshot-emitting specs.
   Correct the "What We're NOT Doing" claim and add a success criterion asserting
   the resolved specs still execute natively (assert the collected count, not just
   green).

2. **Make channel and locale genuinely single-sourced** (addresses: Channel and
   locale duplicated; Locale value diverges; Dead CHROMIUM_CHANNEL import;
   no-drift test gap)
   Pass `channel` and `locale` into the container as env vars from the invoke
   task (sourced from `tasks/shared/playwright.py`) and read them via
   `process.env` in the TS config, mirroring `BASE_URL`. Reconcile the
   server-process locale (`C.UTF-8`) and browser locale (`en-US`) to one coherent
   story sourced from one constant, or document explicitly why they differ. Add a
   test that the command carries the same `channel`/`LANG` the server env uses.

3. **Gate `pty` on an interactive environment** (addresses: pty=True under no-TTY
   CI)
   Use `pty=sys.stdout.isatty()` (or gate on `CI`), and verify the CI leg exits
   non-zero on a deliberate diff so the `prerelease` gate cannot be bypassed.

4. **Add a Docker pre-flight and fail fast** (addresses: No Docker-daemon
   pre-flight; server-lifecycle single point of failure)
   Run a cheap `docker info` check at the top of `visualiser_docker`, before
   spawning the host server, exiting with a one-line README-pointing message when
   Docker is unavailable (also avoids leaking a started server).

5. **Reuse the existing lifecycle helpers and make the orchestration testable**
   (addresses: Inline Popen+SIGTERM lifecycle hard to test; orchestration
   untested; teardown masks failure)
   Use `wait_for_file` for the `.e2e-port` poll and `ProcessOps.terminate`/`kill`
   for teardown; factor spawn/poll/teardown into a helper with injected
   process/clock/fs seams and unit-test the timeout, server-died-early,
   port-never-published, and SIGTERM-times-out paths. Spawn in a process group so
   teardown reaps the Rust child. Include returncode/log in the raised error.

6. **Harden the Phase 2 regeneration against silent corruption** (addresses:
   tautological baseline verification)
   Assert the expected regenerated file count (~103) and visually sample a subset
   (or diff against the retained darwin set in the working tree before deleting
   it) so a partial/wrong run is caught before it becomes canonical.

7. **Prove emulated-vs-native parity early** (addresses: rendering parity
   asserted not proven)
   In Phase 1, generate under emulation on Apple Silicon and run the same compare
   on a native-amd64 host / the CI job before Phase 2 commits anything; document
   the residual risk and the per-assertion tolerance as the only absorber.

8. **Unify path idioms and tidy the task module** (addresses: Two path-sourcing
   idioms; brittle f-string command)
   Migrate the existing `visualiser` task to `tasks.shared.paths` constants so
   the file uses one idiom; consider `shlex.join` for the docker args.

9. **Close the green-but-stale gap and strengthen docs** (addresses: silent skip
   of visual specs; README "why"; stale prerequisites; debugging artefacts; MCR
   tag; Colima fallback)
   Add a visible cue when native e2e runs that visual specs are Docker-only;
   capture the host-server/container "why" durably (README or config comment);
   flip Prerequisites to Docker-required and fix the dual-platform layout line;
   name the concrete `test-results/` artefact path + `show-trace` on failure; note
   the MCR-tag-confirmation step on version bumps and an interim Colima workaround.

10. **Minor hardening** (addresses: baseURL `:0` fallback; lockfile edge cases;
    0.0.0.0 exposure; CI `timeout-minutes`; repo-mount scope; glyph substrate)
    Drop/fail-fast the `:0` baseURL fallback; enumerate lockfile-extraction error
    inputs in tests; document the transient 0.0.0.0 exposure; set
    `timeout-minutes` on the visual job; consider mounting only `{FRONTEND}`;
    confirm a zero-diff for glyph baselines through the unified flow before
    deleting the carve-out.

## Per-Lens Results

### Architecture

**Summary**: The plan's central architectural reframing — containerise only
Chromium, run the Rust server on the host over `host.docker.internal` — is
sound, well-justified, and cleanly preserves the existing host server build
path. The single-source-of-truth design is partially realised (image tag is
genuinely derived from one module) but the Chromium channel and locale are
duplicated across the shared module and the TypeScript config rather than
consumed from one place, undermining the no-drift guarantee the work item
demands. The most serious structural gap is the testDir seam: moving the entire
visual-regression project out of the main config orphans the ~15 non-screenshot
`*-resolved-*` specs that physically live in `tests/visual-regression/`,
contradicting the plan's claim that they continue to run natively.

**Strengths**:
- The host-server / containerised-Chromium split is the right architectural
  seam (keeps server build path identical, avoids Rust-in-image, localises
  nondeterminism to Chromium).
- Networking decision (host.docker.internal + --add-host, rejecting
  --network=host under Colima) is researched against all three environments and
  the rationale is captured in the plan.
- Phase decomposition respects atomicity correctly; the old push-back workflow
  is retained until the new Docker job is green on main.
- Command assembly factored into a pure, Docker-free testable helper.

**Findings**:
- 🔴 high — Removing the visual-regression project orphans the non-screenshot
  specs that share its testDir (Phase 2 §1 + "What We're NOT Doing"). The
  `chromium` project inherits testDir `./e2e`; nothing native targets
  `tests/visual-regression/` after the move, so resolved/computed-style specs
  become dead code while green-suite criteria still pass.
- 🟡 high — Channel and locale duplicated, not consumed from the single source
  (Phase 1 §1-2). Only the image tag flows from the shared module; channel and
  locale are hardcoded in the TS config and the `CHROMIUM_CHANNEL` Python
  constant has no consumer.
- 🟡 medium — Locale value diverges between the shared module (C.UTF-8) and the
  config (en-US) (Phase 1 §1). Two locale concepts, two values; the
  "identical locale from one source" criterion isn't met.
- 🟡 medium — Kanban clean-fixture invariant loses enforcement once the snapshot
  moves to Docker (Phase 2 §1 + Key Discoveries). The ordered single-worker
  invariant is split across two independently-invoked configs; an interrupted
  native run could leave dirty fixtures for a later Docker rebaseline.
- 🔵 medium — Server lifecycle and networking are a single point of failure with
  no degradation path wired (Phase 1 §4 / Migration Notes). No retry / no
  diagnostic distinguishing "server down" from "container can't reach host".
- 🔵 low — Glyph convergence quietly changes the glyph specs' rendering substrate
  (Phase 3 §2). `vite preview` → Rust dev-frontend server; confirm byte-identical
  before deleting the carve-out.

### Correctness

**Summary**: The core mechanism (host Rust server + containerised Chromium over
host.docker.internal, single canonical baseline) is logically sound, and the
BASE_URL-via-env path correctly sidesteps the hardcoded 127.0.0.1 in
global-setup.ts. However, there is a critical logic error: the ~14
`*-resolved-*` specs physically live in `./tests/visual-regression/`, so the new
Docker config will run them in the container while Phase 2 removes the only
project that runs that directory natively — the "run natively unchanged" claim
is false. Several single-source-of-truth claims (channel, locale) are only
partially satisfied because the values are duplicated and hardcoded in the TS
config rather than read from the shared module.

**Strengths**:
- The container-to-host networking correctly avoids global-setup.ts's hardcoded
  origin by giving the Docker config no globalSetup and injecting BASE_URL via
  env — the env value is never clobbered.
- The port-file readiness handshake is sound (unlink stale → poll reappearance;
  written only after post-bind server-info.json).
- Server-bind change is correctly back-compatible.
- Teardown runs in all paths via try/finally with SIGTERM then bounded wait then
  kill.

**Findings**:
- 🔴 high — The `*-resolved-*` specs stop running on every leg, not "run natively
  unchanged" (What We're NOT Doing; Phase 1 §2; Phase 2 §1).
- 🟡 high — Single-source-of-truth violated for channel and locale (Phase 1 §1
  vs §2/§4); no-drift acceptance criteria (0108:132-140) not met as designed.
- 🟡 medium — `pty=True` under no-TTY CI may fail or mask the docker exit code
  (Phase 1 §4 + Phase 2 §3); could let a failing zero-diff pass the gate.
- 🔵 medium — Anonymous node_modules volume is re-created empty each run; host
  node_modules state is the real correctness guard, with an edge case on the
  first-ever run (Phase 1 §4).
- 🔵 medium — Drag-test clean-fixture guarantee survives, but only because the
  kanban snapshot moves out; verify the ordering assumption and add a comment
  (Phase 2 §1).
- 🔵 low — `baseURL` fallback `http://host.docker.internal:0` is an invalid
  origin if BASE_URL is ever unset (Phase 1 §2); masks the root cause.

### Code Quality

**Summary**: The plan is unusually thorough and shows real quality discipline:
it factors command assembly into a pure, unit-testable helper, centralises the
pins in one shared module, and TDDs the deterministic logic. However, the new
`visualiser_docker` task introduces several maintainability frictions that cut
against codebase conventions: an inconsistent path-sourcing idiom within a
single file, a hand-rolled inline Popen+SIGTERM lifecycle that duplicates
existing injectable helpers and is hard to test, a deliberately dead
`CHROMIUM_CHANNEL` import, and an f-string Docker command long enough to invite
quoting bugs.

**Strengths**:
- Command construction extracted into a pure `docker_visual_command` helper
  (no I/O), directly unit-testable without Docker.
- Pins centralised in `tasks/shared/playwright.py` with one consumer site.
- Helpers take injectable `frontend: Path = FRONTEND` params, mirroring
  `binary_path(..., bin_dir=BIN_DIR)`.
- TDD scoped honestly to deterministic logic; phasing clean and structural.

**Findings**:
- 🔴 high — Inline Popen+SIGTERM server lifecycle is hard to test and duplicates
  `polling.py:wait_for_file`, `processes.py:ProcessOps`, and `dev/lifecycle.py`
  (Phase 1 §4). [Severity per agent: surfaced as the lens's top finding;
  aggregated here as Major.]
- 🟡 high — Two different path-sourcing idioms coexist in `tasks/test/e2e.py`
  (Phase 1 §4 imports vs existing task).
- 🔵 high — Dead `CHROMIUM_CHANNEL` import retained "in case" — F401 lint failure
  and a latent split source of truth (Phase 1 §4).
- 🔵 medium — Long f-string command assembly is brittle around quoting; prefer
  `shlex.join` (Phase 1 §4).
- 🔵 medium — Teardown can mask the underlying failure and lacks observability
  on the error path (Phase 1 §4 finally block).

> Note: the code-quality agent rated the lifecycle finding 🔴 within its own
> lens; in cross-lens aggregation it is grouped as a Major theme alongside the
> test-coverage lens's matching finding (no critical-severity defect, since the
> code is testable rather than incorrect).

### Test Coverage

**Summary**: The TDD unit-test plan for the three pure helpers is well-scoped
and mirrors the existing tasks suite, covering the malformed-lockfile path, the
node_modules mask, and the conditional `--update-snapshots`. The most
significant gap is the blanket dismissal of the server-lifecycle orchestration
as "inherently integration/manual-verified" — the codebase already has a
`ProcessOps` protocol and `FakeProcs` fakes precisely so this is unit-tested.
Several acceptance criteria (no-drift alignment, the kanban guarantee) map only
to brittle manual/round-trip steps.

**Strengths**:
- Three helpers correctly isolated as pure functions; tests written first,
  mirroring the existing suite.
- The unit-test plan names the meaningful branches (malformed lockfile, mask,
  env, conditional update).
- Phase criteria separate Automated/Manual and include negative assertions
  (no -darwin/-linux PNGs; no dangling refs).
- The round-trip rebaseline→compare→zero-diff check is the right integration
  shape; Phase 1 baselines correctly discarded.

**Findings**:
- 🟡 high — Server-lifecycle orchestration left untested despite an established
  testable pattern (`ProcessOps` + `FakeProcs`, `dev/lifecycle.py`)
  (Phase 1 §4 + Implementation Approach).
- 🟡 medium — Kanban clean-fixture guarantee after dropping the project
  dependency relies on a manual check (Phase 2 §1 + Manual Verification 3).
- 🔵 high — No-drift criteria for LANG/channel verified only indirectly, not by
  an end-to-end shared-source assertion (AC 0108:132-140 + Phase 1 tests).
- 🔵 medium — Lockfile-extraction edge cases beyond the single malformed case are
  unspecified (Phase 1 §1 + §6).
- 🔵 medium — node_modules-clobber protection and resolved-spec native survival
  verified only manually (Phase 1 Manual Verification + Phase 2).

### Portability

**Summary**: The central thesis — a single canonical Linux baseline produced by a
pinned, containerised Chromium reachable over host.docker.internal — is the right
portability architecture, and the host↔container networking design is unusually
well-reasoned and uniform across Colima, Docker Desktop, and Linux CI. The
remaining risks cluster around the unproven assumption that QEMU-emulated amd64
renders byte-identically to native amd64, the absence of graceful handling when
Docker or the MCR tag is unavailable, `pty=True` on the headless runner, and a
latent locale mismatch (server C.UTF-8 vs browser en-US).

**Strengths**:
- Host-server / containerised-Chromium split eliminates Rust-in-image,
  cross-compilation, and API-mocking portability problems.
- host.docker.internal + --add-host validated as the single uniform mechanism;
  the --network=host trap is avoided.
- Pins read from one shared module; `--platform=linux/amd64` forces every host
  onto one architecture.
- node_modules anonymous-volume mask prevents host clobbering.

**Findings**:
- 🔴 medium — Emulated-amd64 vs native-amd64 rendering parity asserted, checked
  only by one late manual step (Phase 2 Manual Verification + Performance).
  [Aggregated as Major.]
- 🟡 high — `pty=True` on the headless CI runner can hang or mis-handle exit code
  (Phase 1 §4).
- 🟡 high — Docker daemon prerequisite with no preflight / graceful failure
  (Phase 1 §4; Desired End State).
- 🔵 medium — Server (C.UTF-8) vs browser (en-US) locale mismatch (Phase 1 §1/§2).
- 🔵 high — `-noble` flavour + MCR tag existence hardcoded with no clear-error
  fallback (Phase 1 §1; work item Dependencies).
- 🔵 medium — Colima host.docker.internal DNS-regression fallback documented but
  unimplemented, no interim escape hatch (Migration Notes).

### Usability

**Summary**: The plan delivers a coherent two-task developer interface with a
clear round-trip workflow and a well-considered node_modules clobber defence,
wired through existing mise/invoke conventions. The biggest DX gaps are
failure-mode handling (no Docker-not-running preflight → up-to-60s wait before a
raw daemon error) and the silent loss of native self-validation (native green
implies nothing about the visual gate). Discoverability of the compare task and
the on-failure artefact path are secondary frictions the Phase 3 docs can partly
mitigate.

**Strengths**:
- The `:update` suffix is an excellent least-surprise rebaseline convention; a
  single boolean keeps the surface minimal.
- Both tasks carry clear descriptions; discoverability verified via `mise tasks`.
- node_modules clobber designed away structurally, with a host-intact check.
- The round-trip workflow is exercised by Phase 2 manual verification on the same
  commit.
- Server lifecycle handled robustly; `pty=True` preserves progress output during
  the slow emulated run.

**Findings**:
- 🟡 high — No Docker-not-running pre-flight check; failure is slow and cryptic
  (Phase 1 §4).
- 🟡 medium — Silent skip of visual specs invites a green-but-stale commit
  (Phase 2/3).
- 🔵 medium — Diff/trace artefact location promised in docs but not surfaced on
  failure (Phase 3 §3).
- 🔵 medium — Default slow path (npm ci under emulation) every run; fast path only
  a comment (Performance + Phase 1 §4).
- 🔵 high — README still frames Docker as optional and baselines as dual-platform
  (Phase 3 §3).
- 🔵 low (suggestion) — Verb-less compare task name less self-evident than its
  `:update` sibling (Phase 1 §5).

### Documentation

**Summary**: The contributor-documentation scope (Phase 3 §3) is strong and maps
cleanly onto the work item's final documentation acceptance criterion, and goes
beyond it (native skip, audience split). Inline comments are appropriately
rationale-focused. The main gaps are durability of the architectural "why" given
the deferred ADR, one stale inline comment in the new Docker config, and an
under-specified debugging section that asserts artefact locations without
grounding them.

**Strengths**:
- Phase 3 §3 enumerates every element the doc acceptance criterion demands and
  covers both internal macOS and public/Linux contributors.
- Inline comments are rationale-led (locale, channel/v1.49, node_modules-mask,
  snapshotPathTemplate), matching project style.
- The grep guard protects against dangling refs; the "-jammy / two tags" manual
  criterion targets the specific stale README content.
- Overview/Key Discoveries capture the single-canonical-environment rationale in
  durable plan prose; References records the deferred ADR.

**Findings**:
- 🟡 medium — README rewrite does not capture the "why" of the
  host-server/container split for future maintainers (Phase 3 §3).
- 🔵 high — Inline comment claims BASE_URL is set by the invoke task, but the `:0`
  fallback default could mislead (Phase 1 §2).
- 🔵 medium — Debugging guidance asserts artefact locations without grounding them
  in the container's mount/output behaviour (Phase 3 §3).
- 🔵 medium — The CHROMIUM_CHANNEL "imported but unused" note documents a latent
  inconsistency rather than resolving it (Phase 1 §4).

### Safety

**Summary**: A developer-tooling change with a low blast radius — the worst-case
failure is wrong reference pixels, recoverable via VCS revert (the team's
canonical recovery path). The phasing is genuinely careful. The main residual
risk is silent baseline corruption: the Phase 2 regeneration is validated only
by a tautological zero-diff against the just-generated set, with no visual check
against the outgoing baselines, so a subtly-wrong render becomes the canonical
reference undetected. Per the team's stance, missing confirmation/dry-run UX is
explicitly out of scope and not flagged.

**Strengths**:
- Three-phase sequencing keeps each step mergeable and green; the old push-back
  workflow is retained until the new job is green on main.
- Phase 1 is purely additive and inert; trial baselines explicitly discarded.
- node_modules masked so the container never clobbers the host tree, with an
  integrity check.
- The atomic cutover avoids divergent-tag/shape mid-transition states.
- Full regeneration/deletion is trivially reversible via VCS revert.

**Findings**:
- 🟡 medium — Phase 2 baseline regeneration is validated only tautologically;
  outgoing references deleted in the same commit (Phase 2 §2).
- 🔵 medium — Launcher teardown can orphan the Rust child + bound port; `.e2e-port`
  not cleaned on exit (Phase 1 §4).
- 🔵 high — `E2E_SERVER_HOST=0.0.0.0` exposes the dev server on all interfaces
  during a run (Phase 1 §3).
- 🔵 high — New CI visual job has no `timeout-minutes`; a hung run relies on the
  6-hour default (Phase 2 §3).
- 🔵 medium (suggestion) — Container gets a read-write mount of the entire repo
  when only the frontend dir is needed (Phase 1 §4).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-06-12

**Verdict:** REVISE (near-approval — remaining items are spec-precision
refinements to the illustrative code, not architectural rework)

The revision resolved the Critical and most Major findings cleanly. The
directory split (resolved specs → `tests/resolved-styles/` under a native
project) genuinely converts spec-routing from a fragile convention into a
structural boundary; channel/locale are now single-sourced via env with a
`required()` fail-fast; `pty=sys.stdout.isatty()`, the `docker info` preflight,
the emulated-vs-native parity gate, the path-idiom unification, the green-but-
stale warning, and the anti-corruption count/clean-tree guards all landed. The
re-review's findings are a **new cluster the revision introduced**, concentrated
in the illustrative `run_against_host_server` helper and the in-container locale
bootstrap — flagged independently across multiple lenses, so they are real, but
each is a precise-the-spec fix rather than a rethink. Three-plus genuine majors
keep the verdict at REVISE per the configured threshold.

### Previously Identified Issues

- 🔴 **Architecture/Correctness**: Resolved specs orphaned by the project move —
  **Resolved**. Directory split + native `resolved-styles` project; both lenses
  confirm the routing seam is now structural. (Introduced a follow-on: shared
  `lib/` coupling — see new issues.)
- 🟡 **Architecture/Correctness/TestCov/Doc**: Channel/locale not single-sourced —
  **Resolved**. Passed via env from `tasks/shared/playwright.py`, read via
  `process.env`, with no-drift unit assertions.
- 🟡 **Architecture/Portability**: Locale value divergence (C.UTF-8 vs en-US) —
  **Partially resolved**. Aligned to one `en_US.UTF-8`/`en-US` source, but the
  in-container `locale-gen` mechanism introduced a new portability risk (below).
- 🟡 **Correctness/Portability**: `pty=True` under no-TTY CI — **Resolved**
  (`pty=sys.stdout.isatty()`); one lens suggests keying off `CI` as more
  authoritative (minor).
- 🟡 **Portability/Usability**: No Docker pre-flight — **Resolved** (`docker info`
  guard with actionable message, before server bringup).
- 🟡 **CodeQuality/TestCov**: Inline lifecycle hard to test — **Partially
  resolved**. Extracted into `run_against_host_server`, but the described seams
  are inconsistent/incomplete (below) — the strongest new theme.
- 🟡 **CodeQuality**: Two path idioms — **Resolved** (existing task migrated to
  `tasks.shared.paths`).
- 🟡 **Portability**: Emulated-vs-native parity unproven — **Resolved** (early
  Phase 1 gate before any commit).
- 🟡 **Usability**: Green-but-stale native runs — **Resolved** in plan; docs gap
  remains (README Testing/Prereq lines outside rewrite scope — below).
- 🟡 **Safety/TestCov**: Tautological baseline verification — **Partially
  resolved**. Count + clean-tree + sample guard added; test-coverage asks for a
  structural content check (non-blank/dimension floor, exact case-name match).
- 🟡 **Documentation**: README "why" not durable — **Resolved** (rationale in
  Phase 3 §3 scope; one lens asks which code site owns the mirrored comment).
- 🟡 **Architecture/Correctness/TestCov**: Kanban clean-fixture invariant —
  **Partially resolved**. Comment + clean-tree guard added; still no automated
  regression that a drag test observes pristine fixtures (test-coverage major).

### New Issues Introduced

- 🟡 **Architecture/CodeQuality/Correctness/Safety**: **Process-group teardown
  vs PID-only `ProcessOps`.** The plan says `run_against_host_server` "tears the
  process group down via `ProcessOps`", but `tasks/shared/processes.py:ProcessOps`
  signals a single PID (`os.kill(pid, …)`) — it has no `killpg`. Since
  `start-server.mjs` spawns the Rust binary as a child, a SIGKILL escalation to
  the node leader can orphan the Rust server holding the port (and the transient
  0.0.0.0 binding). Flagged by 4 lenses. Fix: spawn with `start_new_session=True`
  and `os.killpg`, **or** enumerate `ProcessOps.children()` and reap each
  (mirroring `lifecycle.py:_reap_handle`); name the exact path.
- 🟡 **CodeQuality/TestCov**: **Spawn seam unspecified; `FakeProcs` can't model
  spawn/exit.** `ProcessOps`/`FakeProcs` only inspect-and-signal; the
  server-died-early path needs an injected launcher returning a handle with
  `poll()`/returncode (the existing `FakeLauncher`/`FakeHandle` seam), which the
  plan omits. Also `FakeProcs` lives privately in `test_lifecycle.py`, not the
  shared `doubles.py` — "reuse" implies hoisting it first.
- 🟡 **Correctness**: **`wait_for_file` cannot detect early server death**, so the
  promised exited-vs-timed-out distinction can't be a fast path — a crashed
  server stalls the full 60s. Interleave a `poll()` liveness check, or document
  detection as post-deadline (and match the unit test to whichever).
- 🟡 **Portability** (×2): **`locale-gen`/`update-locale` bootstrap is fragile.**
  Both are Ubuntu/`locales`-package-specific and may be absent in the slim
  image; the `^en_US` grep over-matches a non-UTF-8 locale and `update-locale`
  does not affect already-exported `LANG`/`LC_ALL`. Risk: silent `C`-locale
  fallback → wrong-but-self-consistent baseline the parity gate can't catch.
  Confirm `locale-gen` exists in the pinned image; assert the locale actually
  resolves post-bootstrap (hard error, not `||` swallow); consider `C.UTF-8`
  everywhere if generation is unreliable.
- 🟡 **Usability**: **`--cache-deps` has no mise entry point.** The inner-loop
  accelerator is defined only on the invoke task; the two mise tasks don't pass
  it through, so the documented `mise` surface can't reach it. Add a third task
  or document the `invoke … --cache-deps` escape hatch (or defer the flag).
- 🟡 **Documentation**: **README Testing/Prereq lines are outside the rewrite
  scope.** `README.md:33` ("`npm run test:e2e` … visual regression") and `:9`
  ("Docker (optional)") will contradict the new flow; add them to the Phase 3 §3
  edit list.
- 🔵 **Correctness**: `_FRONTEND_REL` is now computed but unused (the mount went
  frontend-only) → ruff F401-class failure under `build-system:check`. Drop it.
- 🔵 **Architecture/Correctness**: Shared `lib/`/`helpers.ts` left as a "keep or
  hoist" open choice — if kept, the native `resolved-styles` tree imports from
  the Docker-only `tests/visual-regression/` tree, re-coupling what the split
  separates. Commit to hoisting to a neutral `tests/lib/`.
- 🔵 **TestCov**: resolved-styles collected-count assertion uses "~14" — pin the
  exact count so partial orphaning (a dropped spec) fails.
- 🔵 **Safety/Doc/Usability**: minor follow-ons — fixture-dir clean-tree as a
  hard precondition (not just sample inspection); MCR/Docker as an unguarded
  release-gate external dep (accepted, lean on the bump-check doc); spell the
  full README path in the preflight message; inline `0.0.0.0` comment at the
  bind site; document the glyph showcase(Docker)/resolved-fill(native) split.

### Assessment

The plan's architecture and approach are sound and the prior blocking issues are
genuinely fixed. The remaining majors collapse into three clusters: (1) tighten
the `run_against_host_server` spec — spawn seam, group-aware teardown, early-death
detection (resolves ~5 findings at once); (2) harden the locale bootstrap (or fall
back to `C.UTF-8` everywhere); (3) wire `--cache-deps` into mise and extend the
Phase 3 doc edit list. None require rework. Once those are addressed the plan is
ready to implement; a further full re-review is likely unnecessary — a focused
check of the lifecycle-helper and locale sections would suffice.

## Re-Review (Pass 3, focused) — 2026-06-13

**Verdict:** REVISE → (findings addressed in a follow-up edit; see Assessment)

Scope: architecture, correctness, code-quality, portability — restricted to the
two areas revised after Pass 2: the `run_against_host_server` lifecycle helper
and the locale decision. All four lenses verified claims against the actual
`tasks/shared/*` code.

### Previously Identified Issues

- 🟡 **Locale portability/correctness** (Pass 2): the fragile in-container
  `locale-gen`/Ubuntu coupling — **Resolved**. Portability confirms the coupling
  is "genuinely gone"; correctness confirms the reasoning is "sound" (pixels are
  a function of Chromium's explicit `en-US`; the host/container glibc `LANG`
  reaches no rendered output — verified that the only locale-sensitive renders
  run client-side in Chromium, and Rust's default sort is codepoint-based, not
  `LC_COLLATE`-sensitive).
- 🟡 **Single-source-of-truth, pty, preflight, path-idiom, green-but-stale,
  emulation parity** (Pass 2) — confirmed **Resolved** incidentally; not
  re-litigated.

### New Issues Introduced (by the Pass-2 fixes) — all now addressed

The Pass-2 fix chose `start_new_session=True` + `os.killpg` for teardown and
attributed a `returncode` to the launch handle; all four lenses caught that this
diverged from the real codebase:

- 🟡 **`LaunchHandle`/`PopenHandle` have no `returncode`** (architecture,
  code-quality) — verified: `circus.py` exposes only `pid` + `poll()`. **Fixed**:
  the exit code is now read from `handle.poll()`; all `returncode` references
  dropped.
- 🟡 **`os.killpg` bypasses the injected `ProcessOps` seam** (architecture,
  correctness, code-quality, portability) — `ProcessOps` has no group-signal
  method, so `FakeProcs` could not observe the teardown the §6 tests assert.
  **Fixed**: teardown now uses the established `lifecycle.py:_reap_handle`
  pattern — enumerate `ProcessOps.children(pid)` and `terminate`/`kill` each
  through the seam (observable by `FakeProcs`, matches the codebase's by-PID
  convention).
- 🟡 **`os.getpgid` raises `ProcessLookupError` on the early-exit path**
  (correctness, portability) — the `finally` block would mask the real error.
  **Fixed**: dissolved by dropping `getpgid`/`killpg`; `terminate`/`kill` already
  suppress `ProcessLookupError`, and the test asserts an already-exited leader is
  a no-op.
- 🟡 **Hoisting circus's `FakeLauncher` is a misfit** (code-quality,
  architecture) — it encodes pidfile/arbiter/watcher semantics this helper lacks,
  and `FakeHandle.poll()` is hardcoded to `None` (can't simulate exit). **Fixed**:
  hoist only the contract-pure `FakeProcs`; add a tiny local launcher/handle
  double with a configurable `poll()`.
- 🔵 **Server-log path unsurfaceable** (correctness) — it lives in a
  runtime-random tmpdir inside the node child. **Fixed**: the error now points at
  the inherited console stream (`stdio: "inherit"`) instead of a path the parent
  cannot know.
- 🔵 **Helper module home unspecified / poll-loop duplicates `wait_for_file`**
  (architecture, code-quality) — **Fixed**: named `tasks/shared/dev/host_server.py`
  and aligned the interleaved poll loop with `lifecycle.py:_poll_arbiter_pid`'s
  clamped-deadline shape.
- 🔵 **`on_ready` reads `sys.stdout.isatty()` directly** (code-quality, medium) —
  **Consciously deferred**: thin orchestration glue; the load-bearing
  command-assembly it calls (`docker_visual_command`) is already pure-tested.
  Noted, not changed.

### Assessment

The locale rework is fully resolved and architecturally endorsed by both the
correctness and portability lenses. The lifecycle-helper cluster the Pass-2 fix
introduced was real (verified against `circus.py`/`processes.py`), but it
collapsed to a single root choice — `os.killpg` vs the established by-PID
`_reap_handle` pattern — and switching to the latter (plus dropping the invented
`returncode`, scoping the fake hoist, and fixing the log-path claim) dissolves the
entire cluster while making the teardown both seam-faithful and unit-testable.
Those edits were applied immediately after this pass. The plan is now ready to
implement; no further review pass is warranted — the remaining open item is a
single consciously-deferred code-quality minor (`on_ready` isatty injection).

**Verdict updated to APPROVE (2026-06-13)** by the reviewer after confirming the
Pass-3 lifecycle-helper findings were addressed (teardown switched to the
`_reap_handle` children-enumeration pattern, `returncode` dropped in favour of
`handle.poll()`, fake hoist scoped to `FakeProcs`, log-path claim corrected). The
plan is approved for implementation; the one remaining `on_ready` minor is
accepted as a deliberate trade-off, not a blocker.
