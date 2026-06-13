---
type: plan-validation
id: "2026-06-12-0108-local-docker-visual-regression-baselines-validation"
title: "Validation Report: Local Docker-Based Visual Regression Baseline Generation Implementation Plan"
date: "2026-06-13T21:00:48+00:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: "pass"
parent: "plan:2026-06-12-0108-local-docker-visual-regression-baselines"
target: "plan:2026-06-12-0108-local-docker-visual-regression-baselines"
tags: ["visual-regression", "testing", "ci", "developer-experience", "docker", "playwright"]
last_updated: "2026-06-13T21:00:48+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Local Docker-Based Visual Regression Baseline Generation Implementation Plan

> **Re-validation (2026-06-13T21:00 UTC).** Supersedes the earlier `partial`
> result. The sole outstanding gate then was the CI `test-visual-regression`
> job rendering on native amd64 — the load-bearing emulated-vs-native parity
> assumption. The implementation is now pushed to `main` (`15b54805`) and that
> job has passed, closing the gate. Result upgraded to `pass`.

### Implementation Status

✓ Phase 1: Docker scaffolding (additive, inert) — Fully implemented
✓ Phase 2: Atomic cutover — Fully implemented
✓ Phase 3: Remove legacy, converge, document — Fully implemented

All three phases landed as discrete commits, now merged to `main`:

- `urvpksqk` — Add Docker visual-regression scaffolding (Phase 1)
- `omnmvqnq` — Collapse visual-regression baselines to a single Docker-rendered set (Phase 2)
- `zvwpsttp` — Remove visual-regression push-back workflow and glyph carve-out (Phase 3)

### Automated Verification Results

**CI (Main workflow, run 27476427238, sha `15b548053` — the pushed main HEAD):**

✓ `Run visual regression tests` (ubuntu-latest Docker) — **completed/success**,
  zero-diff against the committed (emulation-generated) baseline set on native amd64
✓ `Run E2E tests (ubuntu-latest)` — success
✓ `Run E2E tests (macos-latest)` — success (no Docker needed; runs no screenshot specs)
✓ `Run unit tests` (both legs), `Run integration tests` (both legs) — success
✓ `Check Python build system`, `Check frontend`, `Check Rust server`, `Check shell scripts` — success
ℹ The overall run reads "pending" only because the downstream `Create prerelease`
  release step is still running; every *verification* job has passed.

**Local (re-confirmed against these exact commits in the prior validation session):**

✓ `mise run test:unit:tasks` — 182 passed (incl. all `docker_visual_command`
  no-drift token assertions and all four `run_against_host_server` lifecycle paths)
✓ `mise run build-system:check` — clean
✓ `mise run frontend:check` — clean
✓ `mise run test:e2e:visualiser` — 321 passed (283 resolved-styles + 38 chromium;
  0 screenshot cases; main config collects 0 `[visual-regression]` specs)
✓ `resolved-styles` project collects exactly 14 relocated spec files
✓ Exactly one baseline set on disk: 156 PNGs, 0 `*-darwin.png`, 0 `*-linux.png`
✓ `update-visual-baselines.yml` and `playwright.glyph.config.ts` deleted; no dangling refs
✓ Both `test:e2e:visualiser:docker` tasks resolve under `mise tasks`

### Code Review Findings

#### Matches Plan:

- `tasks/shared/playwright.py` — single source of truth: `PLAYWRIGHT_IMAGE_FLAVOUR`,
  `PLAYWRIGHT_PLATFORM`, `BROWSER_LOCALE`, `E2E_LANG`, `CHROMIUM_CHANNEL`,
  `resolved_playwright_version`, `playwright_image` exactly as specified.
- `playwright.docker.config.ts` — `webServer`-less, no `globalSetup`, no `{platform}`
  token, `required()` fail-fast for `BASE_URL`/`CHROMIUM_CHANNEL`/`PLAYWRIGHT_LOCALE`.
- `playwright.config.ts` — `visual-regression` project removed; `resolved-styles`
  added; `dependencies: ["visual-regression"]` dropped; webServer/globalSetup/teardown/
  workers:1 retained; ordering-invariant comments added.
- Directory split executed: 8 screenshot specs in `tests/visual-regression/`,
  14 computed-style specs in `tests/resolved-styles/`, shared helpers hoisted to `tests/lib/`.
- `e2e/start-server.mjs` bind host configurable (`E2E_SERVER_HOST ?? "127.0.0.1"`).
- `tasks/shared/dev/host_server.py` — by-PID reap via `procs.children()` +
  terminate→wait→kill (no `os.killpg`), interleaved `handle.poll()`/port check,
  exited-vs-timeout error distinction.
- `tasks/test/e2e.py` migrated to `tasks.shared.paths`; thin `@task` adapter.
- `mise.toml` tasks depend on `["build:frontend", "build:server:dev"]`, not playwright install.
- `main.yml` — `test-visual-regression` job (ubuntu-latest, timeout 20m) added and
  appended to `prerelease.needs`; `test-e2e` matrix structurally intact.
- `README.md` — leads with the two task names; covers the keystone "why", Docker
  prerequisite, re-baseline flow, the green-but-stale trap (incl. Testing-section
  reconciliation), `--cache-deps`, transient `0.0.0.0` exposure, Colima fallback,
  the `@playwright/test` bump tag-existence check, the debugging path, and the
  glyph spec split. No `jammy` references remain.

#### Deviations from Plan:

- None material. Implementation tracks the plan's prescribed code essentially verbatim.

#### Potential Issues:

- **Emulated-vs-native parity (now resolved).** The committed baselines were
  generated under QEMU amd64 emulation on Apple Silicon; the green CI
  `test-visual-regression` run on native ubuntu-latest confirms byte-equivalence
  within the unchanged `0.05`/`0.01` tolerances. The risk the plan flagged as
  load-bearing is now empirically discharged. Future maintainers should re-run
  the Docker update task (not hand-edit PNGs) and confirm a green CI visual job
  on each re-baseline.

### Manual Testing Required:

Remaining items are optional confidence checks; the gating verifications are done.

1. Documentation:
  - [ ] Skim the README "Visual-Regression Baselines" section end-to-end as a new
        (incl. Linux) contributor would.

2. Inner-loop ergonomics (optional):
  - [ ] Exercise `invoke test.e2e.visualiser-docker --cache-deps` to confirm the
        named-volume speed-up on Apple Silicon.
  - [ ] With Docker stopped, confirm the task fails fast with the
        daemon-not-reachable message and leaves no host server running.

### Recommendations:

- **Plan is complete and verified end-to-end; status moved to `done`.**
- Consider capturing the deferred "single canonical render environment as source
  of truth" ADR (flagged in the plan's References) now that CI has proven the design.
- The full-parallel `mise run test` webServer-contention flake noted in the plan
  is pre-existing (tracked separately); not a regression from this work.
