---
type: plan-validation
id: "2026-06-12-0108-local-docker-visual-regression-baselines-validation"
title: "Validation Report: Local Docker-Based Visual Regression Baseline Generation Implementation Plan"
date: "2026-06-13T17:16:53+00:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: "partial"
parent: "plan:2026-06-12-0108-local-docker-visual-regression-baselines"
target: "plan:2026-06-12-0108-local-docker-visual-regression-baselines"
tags: ["visual-regression", "testing", "ci", "developer-experience", "docker", "playwright"]
last_updated: "2026-06-13T17:16:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Local Docker-Based Visual Regression Baseline Generation Implementation Plan

### Implementation Status

✓ Phase 1: Docker scaffolding (additive, inert) — Fully implemented
✓ Phase 2: Atomic cutover — Fully implemented
✓ Phase 3: Remove legacy, converge, document — Fully implemented

All three phases landed as discrete commits:

- `urvpksqk` — Add Docker visual-regression scaffolding (Phase 1)
- `omnmvqnq` — Collapse visual-regression baselines to a single Docker-rendered set (Phase 2)
- `zvwpsttp` — Remove visual-regression push-back workflow and glyph carve-out (Phase 3)

These commits sit **ahead of `main`** and are **not yet pushed**, so the
keystone end-to-end check — the CI `test-visual-regression` job rendering on
native amd64 and passing zero-diff against the committed (emulation-generated)
set — has not yet been exercised. This is why the result is `partial` rather
than `pass`: the code is complete and every locally-runnable check is green,
but the load-bearing emulated-vs-native parity verification the plan itself
flags (Migration Notes) is outstanding.

### Automated Verification Results

✓ Task unit tests pass: `mise run test:unit:tasks` (182 passed)
  - `test_playwright_shared.py` — version extraction (incl. all 5 malformed-input
    cases + malformed-version rejection) and `mcr.microsoft.com/playwright:v1.59.1-noble`
    tag construction.
  - `test_e2e_docker.py::TestDockerVisualCommand` — token-list assertions for
    `--platform`, `--ipc=host`, `--add-host`, frontend-only mount, node_modules
    mask, env vars, no-drift equality with shared constants, `bash -c` payload
    (no `locale-gen`), `--update-snapshots` gating, `--cache-deps` named-volume switch.
  - `test_e2e_docker.py::TestRunAgainstHostServer` — all four lifecycle paths
    (ready, server-exits-early, port-never-published, terminate-times-out-then-kill
    reaping leader **and** child), plus already-exited-leader no-op teardown.
✓ Python format/lint/types pass: `mise run build-system:check` (76 files formatted, 0 errors)
✓ Frontend typecheck/format/lint pass: `mise run frontend:check` (373 files, clean)
✓ Native visualiser e2e green and runs no screenshot specs: `mise run test:e2e:visualiser`
  (321 passed: 283 resolved-styles + 38 chromium; 0 screenshot cases — confirmed via
  `playwright test --list`, which collects 0 `[visual-regression]` specs in the main config)
✓ `resolved-styles` project collects exactly 14 relocated spec files (no full/partial orphaning)
✓ Mise resolves both new tasks: `mise tasks | grep visualiser:docker`
✓ Exactly one baseline set on disk: 156 PNGs, **0** `*-darwin.png`, **0** `*-linux.png`
✓ `update-visual-baselines.yml` deleted; `playwright.glyph.config.ts` deleted
✓ No dangling references to deleted files in code (`tsconfig.node.json` references
  `playwright.docker.config.ts`; only intentional README explanatory note remains)
⏳ CI `test-visual-regression` job zero-diff on native amd64 — **not run** (commits unpushed)

### Code Review Findings

#### Matches Plan:

- `tasks/shared/playwright.py` — `PLAYWRIGHT_IMAGE_FLAVOUR="noble"`,
  `PLAYWRIGHT_PLATFORM="linux/amd64"`, `BROWSER_LOCALE="en-US"`,
  `E2E_LANG="C.UTF-8"`, `CHROMIUM_CHANNEL="chromium"`, plus `resolved_playwright_version`
  / `playwright_image` exactly as specified — single source of truth, no second
  declaration in TS.
- `playwright.docker.config.ts` — `webServer`-less, no `globalSetup`,
  `snapshotPathTemplate` without `{platform}`, `required()` fail-fast for
  `BASE_URL`/`CHROMIUM_CHANNEL`/`PLAYWRIGHT_LOCALE`, single `visual-regression` project.
- `playwright.config.ts` — `visual-regression` project removed; `resolved-styles`
  project added (`testDir: ./tests/resolved-styles`); `dependencies: ["visual-regression"]`
  dropped from `chromium`; `webServer`/`globalSetup`/`globalTeardown`/`workers:1`
  retained; ordering-invariant comments added as specified.
- Spec directory split executed: 8 screenshot specs remain in `tests/visual-regression/`;
  14 `*-resolved-*`/computed-style specs relocated to `tests/resolved-styles/`;
  shared helpers hoisted to neutral `tests/lib/` (`helpers.ts`,
  `expected-colours.ts`, `detail-route-slugs.ts`).
- `e2e/start-server.mjs` — bind host now `process.env.E2E_SERVER_HOST ?? "127.0.0.1"`.
- `tasks/shared/dev/host_server.py` — `run_against_host_server` reaps by-PID via
  `procs.children(handle.pid)` + `terminate`→bounded-wait→`kill` (no `os.killpg`),
  interleaves `handle.poll()` with the `.e2e-port` check, distinguishes
  *exited (code N)* from *timed out*; binds `0.0.0.0` with `# noqa: S104`.
- `tasks/test/e2e.py` — migrated to `tasks.shared.paths` (`FRONTEND`, `SERVER`),
  thin `@task` adapter over `docker_visual_command` + `run_against_host_server`.
- `mise.toml` — both tasks depend on `["build:frontend", "build:server:dev"]`,
  not `deps:install:playwright`.
- `main.yml` — `test-visual-regression` job (ubuntu-latest, `timeout-minutes: 20`)
  added and appended to `prerelease.needs`; `test-e2e` matrix left structurally intact.
- `README.md` — leads with the two task names; covers the keystone "why" (only
  Chromium containerised, no `--network=host`), Docker prerequisite, re-baseline
  workflow, the native/Docker green-but-stale trap (incl. the Testing-section
  reconciliation), `--cache-deps`, transient `0.0.0.0` exposure, `host.docker.internal`
  Colima fallback, the `@playwright/test` bump tag-existence check, the
  `test-results/`/`show-trace`/`show-report` debugging path, and the glyph spec split.
  No `jammy` references remain.

#### Deviations from Plan:

- None material. Implementation tracks the plan's prescribed code essentially
  verbatim, including comments and the by-PID reap rationale.

#### Potential Issues:

- **Baselines are emulation-generated and unvalidated on native amd64.** The
  committed 156-PNG set was produced under QEMU amd64 emulation on Apple Silicon.
  The plan's own Migration Notes name emulated-vs-native parity as "the
  load-bearing reproducibility assumption," absorbed only by the per-assertion
  `0.05`/`0.01` tolerances. Until the CI job renders on native ubuntu-latest and
  passes zero-diff, a divergence beyond tolerance is an unsurfaced risk.
- **Plan status not auto-closed.** Because the cutover's end-to-end CI validation
  is pending, the plan is left at its current status rather than marked `done`.

### Manual Testing Required:

1. CI / cross-architecture parity (the gating items):
  - [ ] Push the three commits; confirm the CI `test-visual-regression` job
        passes zero-diff against the committed set on the **same commit** (no push-back).
  - [ ] Confirm `prerelease` gates on `test-visual-regression` in a real run.
  - [ ] On Apple Silicon, `mise run test:e2e:visualiser:docker` passes under amd64 emulation.

2. Local Docker round-trip (Phase 1, requires a running daemon):
  - [ ] `mise run test:e2e:visualiser:docker:update` writes a no-suffix `*-snapshots/` set.
  - [ ] Immediate `mise run test:e2e:visualiser:docker` passes zero-diff (round-trip stability).
  - [ ] With Docker stopped, the task fails fast with the daemon-not-reachable
        message and leaves no host server running.
  - [ ] Host `node_modules` intact after a run (no Linux-native binary clobber).

3. Documentation:
  - [ ] README readable end-to-end by a new (incl. Linux) contributor.
  - [ ] Glyph baselines present in the single canonical set and validated by the Docker job.

### Recommendations:

- **Push and let CI render before treating the baselines as canonical.** This is
  the single outstanding gate; the whole single-baseline promise rests on it.
  Once the `test-visual-regression` job is green on the pushed commit, re-run
  this validation to upgrade the result to `pass` and close the plan.
- Consider capturing the deferred "single canonical render environment as source
  of truth" ADR (flagged in the plan's References) now that the implementation
  has concretised the decision.
- The full-parallel `mise run test` webServer-contention flake noted in the plan
  is pre-existing (tracked separately); it is not a regression from this work.
