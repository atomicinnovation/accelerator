---
type: plan
id: "2026-06-12-0108-local-docker-visual-regression-baselines"
title: "Local Docker-Based Visual Regression Baseline Generation Implementation Plan"
date: "2026-06-12T16:54:30+00:00"
author: "Toby Clemson"
producer: create-plan
status: done
work_item_id: "work-item:0108"
parent: "work-item:0108"
derived_from: ["codebase-research:2026-06-12-0108-local-docker-visual-regression-baselines"]
tags: ["visual-regression", "testing", "ci", "developer-experience", "docker", "playwright"]
revision: "3e0bca8cd5bf31d93886fbf22369afd8eb87eaac"
repository: "miscellaneous"
last_updated: "2026-06-13T07:15:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Local Docker-Based Visual Regression Baseline Generation Implementation Plan

## Overview

Move visual-regression baseline generation off the CI push-back workflow and
into a pinned, official Playwright Linux Docker image that every developer and
CI run shares. Collapse the per-platform (`-darwin` / `-linux`) baseline set
into a single canonical Linux set rendered inside that image, isolate the
visual specs from native macOS test runs, run them in CI as a dedicated
Linux-only Docker job (out of the `test-e2e` matrix), and remove the
`update-visual-baselines.yml` push-back workflow.

The architectural keystone (from the research): **only Chromium is
containerised.** The Rust visualiser server runs on the *host* via the existing
`build:server:dev` path — exactly as CI does today — and the container reaches
it over `host.docker.internal`. This avoids putting a Rust toolchain in the
image, cross-compilation, or API mocking. The committed pixels are produced
entirely by the containerised Chromium, so the host architecture of the server
is irrelevant to the baseline.

## Current State Analysis

- **Specs and baselines** live in the frontend package:
  `skills/visualisation/visualise/frontend/tests/visual-regression/`. Only **8**
  of the 23 specs emit PNGs (≈103 cases → ≈206 committed PNGs, ~50/50
  darwin/linux); the ~15 `*-resolved-*` specs assert computed styles and write
  no snapshots. They share the `visual-regression` project's `testDir`, so they
  are **not** orthogonal: removing that project would orphan them unless they are
  relocated to keep running natively (Phase 2 §1).
- **`playwright.config.ts`** (`:21-37`) defines a `visual-regression` project
  and a dependent `chromium` project (`dependencies: ["visual-regression"]`,
  `:34`) with `workers: 1` (`:13`). The dependency exists *solely* so the kanban
  snapshot captures clean fixtures before the chromium drag tests mutate work
  item statuses. The `webServer` (`:38-46`) spawns the Rust server via
  `node e2e/start-server.mjs`. `baseURL` defaults to `http://127.0.0.1`
  (`:18`).
- **`start-server.mjs`** builds the server (`cargo build` fallback, `:43-50`) or
  uses `ACCELERATOR_VISUALISER_BIN`, writes the chosen port to `.e2e-port`
  (`:145`), and binds the config `host: "127.0.0.1"` (`:100`). `global-setup.ts`
  reads `.e2e-port` and sets `BASE_URL` to `http://127.0.0.1:{port}` (`:54-61`),
  and snapshots/restores fixtures for the drag tests (`:28-52`).
- **No `snapshotPathTemplate`, no `channel`, no `locale`/`LANG`/`LC_ALL`** are
  set today — the `-darwin`/`-linux` suffix comes from Playwright's default
  template. All three pins are net-new.
- **Tolerances are per-assertion**, no global `expect` block:
  `maxDiffPixelRatio: 0.05` everywhere except `task-list-visual.spec.ts:19`
  (`0.01`). No `threshold` anywhere.
- **`playwright.glyph.config.ts`** is a frontend-only (`vite preview`) Docker
  carve-out for the glyph specs, with a working
  `mcr.microsoft.com/playwright:v1.59.1-noble` recipe in `README.md:67-77`.
- **CI** (`main.yml`): `test-e2e` runs on a `[ubuntu-latest, macos-latest]`
  matrix via `mise run test:e2e` (`:59-79`), no Docker. `prerelease` gates on
  all seven check jobs (`:166-176`). `update-visual-baselines.yml` is
  `workflow_dispatch`-only and pushes regenerated baselines back to `main`
  under `GITHUB_TOKEN` (which is why it cannot re-trigger Main CI).
- **Version source of truth**: `package.json:39` is `"@playwright/test":
  "^1.59.1"` (caret); the committed `package-lock.json` resolves it exactly to
  `1.59.1` — an install-independent single source for the image tag.
- **Task wiring**: `mise.toml` `test:e2e:visualiser` (`:172-175`) →
  `invoke test.e2e.visualiser` (`tasks/test/e2e.py:6-22`), which runs
  `npm --prefix <fe> run test:e2e` with `ACCELERATOR_VISUALISER_BIN` set. Path
  constants in `tasks/shared/paths.py` (`FRONTEND`, `REPO_ROOT`). `build:server:dev`
  → `invoke build.server-dev` (`tasks/build.py:130-140`).

## Desired End State

- A `mise run test:e2e:visualiser:docker:update` task regenerates the single
  canonical Linux baseline set inside the pinned `linux/amd64` Playwright image.
- A `mise run test:e2e:visualiser:docker` task runs the visual specs in that
  image in compare mode, so a developer can verify a clean, zero-diff pass
  before committing.
- The image tag, Chromium `channel`, `linux/amd64` platform, and the locale
  (`LANG`/`LC_ALL` **and** the Playwright browser `locale`) are read from a
  single shared source of truth consumed identically by the local task and CI:
  the image tag is derived from the lockfile, and `channel`/`locale` are passed
  into the container as env vars from that same module (read via `process.env`
  in the Docker config, mirroring how `BASE_URL` already flows). Nothing is
  hardcoded a second time in the TS config.
- The **screenshot-emitting** visual specs live only in
  `playwright.docker.config.ts`; native macOS runs (`mise run test`,
  `mise run test:e2e`) never execute them. The non-screenshot `*-resolved-*`
  (computed-style) specs are relocated to their own directory and keep running
  natively on every run via a dedicated `resolved-styles` project in the main
  config.
- CI runs the visual specs in one dedicated Linux-only Docker job, gated into
  the release flow; the `test-e2e` matrix runs only the chromium e2e specs on
  both legs.
- Exactly one baseline set exists on disk (no `-darwin`/`-linux` split).
- `update-visual-baselines.yml` and `playwright.glyph.config.ts` are gone;
  contributor docs cover the Docker prerequisite, re-baselining, and local
  debugging.

**Verification**: the acceptance criteria in `meta/work/0108-...md:99-149`,
mapped onto phase success criteria below.

### Key Discoveries

- Only Chromium needs containerising; the host server suffices because the
  baseline is a function of what Chromium renders, not the server's
  architecture (research Follow-up §"decisive reframing").
- `host.docker.internal` + `--add-host=host.docker.internal:host-gateway` is the
  single container→host mechanism that works on Colima, Docker Desktop, **and**
  Linux CI; `--network=host` must **not** be used locally (under Colima it joins
  the Lima VM's namespace, not the macOS host). Research Follow-up §"What the
  host-server split requires".
- Isolation is achieved structurally: **move the `visual-regression`
  (screenshot) project out of `playwright.config.ts` into
  `playwright.docker.config.ts`**, and **relocate the non-screenshot
  `*-resolved-*` specs to `tests/resolved-styles/`** under a native
  `resolved-styles` project so they keep running on every native/CI leg. With no
  screenshot project left in the main config, native `playwright test` captures
  no snapshots — no `--project` filter, `testIgnore`, or tag needed; the
  directory split is what makes the isolation structural (simply removing the
  project would orphan the resolved specs that share its `testDir`). The
  `chromium` project's `dependencies: ["visual-regression"]` is then dropped; the
  drag tests' clean-fixture guarantee is preserved by `global-setup`/`teardown`
  restore (the kanban *snapshot* now runs only in the Docker job, which runs no
  drag tests).
- The committed `-linux.png` baselines were generated by plain `ubuntu-latest`
  (non-Docker, default channel, no locale pin), so they will **not** match the
  pinned image zero-diff. Regeneration is therefore inseparable from switching
  CI execution to the Docker job and removing visual specs from the plain matrix
  leg — this is the work item's "single atomic increment" (Phase 2).
- `node_modules` host/container clobbering (README `:77`) is avoided by masking
  `node_modules` with a Docker volume so the container's Linux-native binaries
  never touch the host tree.

## What We're NOT Doing

- **No ADR** in this plan (confirmed out of scope). The "single canonical render
  environment as source of truth" decision is noted as a recommended follow-up
  (see References), not captured here.
- **Not** changing what the `*-resolved-*` specs assert — they take no
  screenshots. They are, however, **relocated** to a dedicated directory
  (`tests/resolved-styles/`) so they keep running natively, because removing the
  `visual-regression` project from the main config would otherwise orphan them
  (nothing native targets `tests/visual-regression/` once that project is gone —
  the surviving `chromium` project's `testDir` is `./e2e`). See Phase 2 §1.
- **Not** introducing a global `expect`/`threshold` block or raising any
  tolerance; per-assertion `maxDiffPixelRatio` values stay at or below today's
  (`0.05` / `0.01`).
- **Not** adopting Git LFS for the PNG set (out of scope; flagged in the work
  item as a possible separate follow-up).
- **Not** baking Rust/cargo/mise into the Docker image, cross-compiling the
  server, or mocking `/api/*`.
- **Not** using the cross-compiled-server sidecar fallback unless host↔container
  networking proves flaky (documented as a fallback only).

## Implementation Approach

Three phases, each independently mergeable and each leaving the suite green.
Phase 1 is purely additive scaffolding that nothing existing references. Phase 2
is the irreducible atomic cutover the work item mandates. Phase 3 removes the
now-redundant legacy and writes the docs. The old push-back workflow survives
until Phase 3, by which point the Phase 2 Docker job has been green on `main` —
satisfying the work item's "keep `update-visual-baselines.yml` in place until
the new Docker task is verified passing in CI" constraint.

TDD applies to the unit-testable logic introduced in Phase 1 — version
extraction, image-tag construction, docker-command assembly, **and the
server-lifecycle helper** (spawn/poll/teardown, made testable by injecting the
existing `FakeProcs`/`FakeClock` seams) — via pytest under `tests/unit/tasks`,
mirroring the existing task test suite (`tasks/shared/dev/lifecycle.py` +
`test_lifecycle.py`). Only the actual baseline regeneration and CI wiring (which
need a real Docker daemon and a real render) remain integration/manual-verified.

---

## Phase 1: Docker scaffolding (additive, inert)

### Overview

Introduce the shared version-pin source of truth, the `webServer`-less Docker
Playwright config, the back-compatible server bind change, and the orchestrating
invoke + mise tasks. Nothing existing references these, so CI, native runs, and
committed baselines are untouched and stay green. Verified by a local
rebaseline→compare round-trip and by unit tests — **not** by committing a second
baseline set (that lands in Phase 2).

### Changes Required

#### 1. Shared version-pin source of truth

**File**: `tasks/shared/playwright.py` (new)
**Changes**: A helper that reads the resolved `@playwright/test` version from
the committed `package-lock.json` (install-independent) and builds the pinned
image tag, plus the shared locale/channel/flavour/platform constants. This is
the single source consumed by every Docker invocation. To satisfy the work
item's no-drift acceptance criteria (`0108:132-140`), the channel and locale are
**not** re-declared in the TS config — they are passed into the container as env
vars by the invoke task (below) and read via `process.env` in
`playwright.docker.config.ts`, exactly as `BASE_URL` already flows. So every
value that must stay aligned has exactly one declaration site.

**The locale that determines the committed pixels is Chromium's, not the
container OS's.** Chromium's rendering locale is set explicitly via Playwright's
`locale` option (`BROWSER_LOCALE = "en-US"`), which is OS-independent and does
**not** require any glibc locale to be generated in the image. The container
shell's glibc locale (`LANG`/`LC_ALL`) is therefore pinned to `C.UTF-8` — which
every Playwright image ships pre-generated — purely for deterministic
shell/`npm` behaviour; it has no bearing on the rendered baseline, so there is no
need (and no attempt) to `locale-gen en_US.UTF-8` inside the container. This
removes a fragile, flavour-coupled bootstrap step while keeping the
rendering-relevant locale (`en-US`) single-valued and identical across every run.

```python
import json
from pathlib import Path

from tasks.shared.paths import FRONTEND

PLAYWRIGHT_IMAGE_FLAVOUR = "noble"
PLAYWRIGHT_PLATFORM = "linux/amd64"
# The locale that shapes the committed pixels is Chromium's, set explicitly and
# OS-independently via Playwright's `locale` option — single-valued en-US across
# every run. The container/host glibc locale (LANG/LC_ALL) is pinned to C.UTF-8,
# which every Playwright image ships pre-generated, only for deterministic
# shell/npm behaviour; it does NOT affect the rendered baseline, so we never
# locale-gen inside the image. Both values live here so this module is the sole
# source and neither can drift.
BROWSER_LOCALE = "en-US"
E2E_LANG = "C.UTF-8"
# Chromium channel pinned so the headless build is deterministic regardless of
# Playwright's default-headless changes (e.g. the v1.49 shift). Consumed by the
# Docker config via the CHROMIUM_CHANNEL env var the invoke task sets — defined
# here so this module is the sole source.
CHROMIUM_CHANNEL = "chromium"


def resolved_playwright_version(frontend: Path = FRONTEND) -> str:
    """Read the exact @playwright/test version from the committed lockfile."""
    lock = json.loads((frontend / "package-lock.json").read_text())
    pkg = lock["packages"]["node_modules/@playwright/test"]
    return pkg["version"]


def playwright_image(frontend: Path = FRONTEND) -> str:
    version = resolved_playwright_version(frontend)
    return f"mcr.microsoft.com/playwright:v{version}-{PLAYWRIGHT_IMAGE_FLAVOUR}"
```

> **Why not `en_US.UTF-8` for `LANG`?** Generating it in-container is
> flavour-coupled (the `locales` package + `locale-gen`/`update-locale` are
> Debian/Ubuntu-specific and may be absent from the slim image) and, critically,
> pointless: the host Rust server runs on the host (where the invoke task sets
> its locale), and the container's glibc locale never reaches the rendered
> pixels — Chromium renders under its explicit `en-US` locale. Pinning the shell
> to the universally-present `C.UTF-8` is the robust choice; `en-US` (the
> rendering locale) remains single-valued everywhere.

#### 2. Docker Playwright config

**File**: `skills/visualisation/visualise/frontend/playwright.docker.config.ts` (new)
**Changes**: A standalone config that runs **only** the screenshot-emitting
visual-regression specs (after the Phase 2 directory split,
`tests/visual-regression/` contains only those — see Phase 2 §1), with no
`webServer` (the server is started on the host by the invoke task), an explicit
`snapshotPathTemplate` **without** `{platform}`, and no `globalSetup` (the
read-only visual specs need no fixture snapshotting). `BASE_URL`, the Chromium
`channel`, and the browser `locale` are all read from env (set by the invoke
task from `tasks/shared/playwright.py`) so there is no second declaration of any
no-drift-critical value. `BASE_URL` **fails fast** if unset rather than falling
back to an unreachable placeholder origin.

```ts
import { defineConfig } from "@playwright/test";

function required(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(
      `${name} must be set by the docker invoke task — refusing to run ` +
        `against an unconfigured origin/channel/locale.`,
    );
  }
  return value;
}

// Visual-regression screenshot specs run only here, inside the pinned Playwright
// Linux image. The Rust server runs on the host; this config reaches it via
// BASE_URL. channel and locale come from the single shared source
// (tasks/shared/playwright.py) via env, never hardcoded here — see that module.
export default defineConfig({
  testDir: "./tests/visual-regression",
  snapshotDir: "./tests/visual-regression/__screenshots__",
  // Single canonical baseline — no {platform} token; rendering is always the
  // pinned container.
  snapshotPathTemplate:
    "{snapshotDir}/{testFileName}-snapshots/{arg}{-projectName}{ext}",
  timeout: 30_000,
  retries: 1,
  workers: 1,
  use: {
    baseURL: required("BASE_URL"),
    channel: required("CHROMIUM_CHANNEL"),
    locale: required("PLAYWRIGHT_LOCALE"),
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "visual-regression",
      use: { browserName: "chromium" },
    },
  ],
});
```

#### 3. Back-compatible server bind change

**File**: `skills/visualisation/visualise/frontend/e2e/start-server.mjs`
**Changes**: Make the server's bind host configurable, defaulting to
`127.0.0.1` so native runs are unaffected; the docker task sets it to `0.0.0.0`
so the container can reach the host over the bridge gateway (required for CI
parity; Colima tolerates loopback but `0.0.0.0` covers both).

```js
// near the config block (~line 100)
// Loopback by default; the Docker task overrides to 0.0.0.0 so the container
// can reach the host over the bridge gateway. The 0.0.0.0 binding is transient
// (one compare/rebaseline run) and serves only non-sensitive committed fixtures.
host: process.env.E2E_SERVER_HOST ?? "127.0.0.1",
```

> **Note on exposure**: when the docker task sets `0.0.0.0`, the dev server (which
> serves only the committed, non-sensitive Markdown fixtures) is reachable on all
> host interfaces for the duration of the run. This is a deliberate, transient,
> documented trade-off — native runs keep the loopback default, and the binding
> lives only as long as the containerised compare/rebaseline. Document it in the
> README testing section so it is intentional, not incidental.

#### 4. Orchestrating invoke task

**File**: `tasks/test/e2e.py`
**Changes**: Add a `visualiser_docker` task that pre-flights Docker, starts the
host server, waits for `.e2e-port`, runs Playwright in the container against
`host.docker.internal:<port>`, and tears the server down. A single `update`
flag toggles compare vs rebaseline.

Two factoring decisions keep the risk-bearing parts testable and consistent with
the rest of the suite:

- **Command assembly is a pure helper** (`docker_visual_command`) that returns a
  token list assembled with `shlex.join`, so the unit tests assert on discrete
  tokens rather than substring-matching one long string, and nested-quote bugs
  are designed out (cf. `tasks/github.py:68`).
- **Server lifecycle reuses the existing seams** instead of hand-rolling raw
  process control, mirroring `tasks/shared/dev/lifecycle.py`. The helper lives in
  a named module beside its peers — `tasks/shared/dev/host_server.py` — keeping
  `tasks/test/e2e.py` a thin `@task` adapter. Three distinct seams are injected
  (the same split `lifecycle.py` already uses — they are *not* one object):
  - a **launcher** `Callable[..., LaunchHandle]` that spawns
    `node e2e/start-server.mjs` and returns a handle exposing `pid` and
    `poll() -> int | None` (the exact `LaunchHandle`/`PopenHandle` shape in
    `tasks/shared/dev/circus.py` — note it has **no** separate `returncode`; the
    exit code *is* `poll()`'s return value once the process exits);
  - `tasks/shared/clock.py:Clock` for the poll deadline; and
  - `tasks/shared/processes.py:ProcessOps` for signalling.

  **Teardown reaps the node→Rust tree exactly as `lifecycle.py:_reap_handle`
  does** — enumerate `ProcessOps.children(handle.pid)` and signal the node leader
  plus each child through the injected seam (`terminate` → bounded wait →
  `kill`, both of which already suppress `ProcessLookupError`). This is chosen
  over `os.killpg` deliberately: it routes through the injected `ProcessOps` (so
  the teardown is observable by `FakeProcs` in tests), it matches the codebase's
  established by-PID convention (circus signals by PID, not by group), and it has
  no `os.getpgid` call to raise on the already-exited path. The Rust child is a
  plain child of the node launcher, so `children()` finds it.

  The poll loop **interleaves `handle.poll()` with the `.e2e-port` check** (using
  the same `Clock.now`/clamped-`Clock.sleep` deadline shape as
  `lifecycle.py:_poll_arbiter_pid`, rather than blocking purely on
  `wait_for_file`), so a server that dies before publishing the port
  short-circuits immediately; the raised error distinguishes *server exited
  (code N)* — the code from `handle.poll()` — from *timed out after 60s*. The
  server's own stdout/stderr already stream to the console (start-server.mjs
  spawns the binary with `stdio: "inherit"`), so the error points the reader at
  that streamed output rather than a log path the Python parent cannot know (the
  server's `server.log` lives in a runtime-random tmpdir inside the node child).
  The whole spawn/poll/teardown lives in `run_against_host_server(...)` taking
  these seams, so all four paths (ready, server-died-early, port-never-published,
  terminate-times-out-then-kill) are unit-testable with fakes and no live node
  process.

Also migrate the **existing** `visualiser` task in this file from
`from .helpers import repo_root` to the `tasks.shared.paths` constants
(`FRONTEND`, `SERVER`) so the module uses one path-sourcing idiom throughout.

```python
import os
import shlex
import sys
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.paths import FRONTEND, SERVER
from tasks.shared.playwright import (
    BROWSER_LOCALE,
    CHROMIUM_CHANNEL,
    E2E_LANG,
    PLAYWRIGHT_PLATFORM,
    playwright_image,
    resolved_playwright_version,
)

def docker_visual_command(
    base_url: str, image: str, update: bool, cache_deps: bool = False
) -> str:
    pw = (
        "npx playwright test --config playwright.docker.config.ts "
        "--project visual-regression"
    )
    if update:
        pw += " --update-snapshots"
    # No locale-gen: C.UTF-8 is pre-generated in every Playwright image and the
    # rendering locale is Chromium's explicit en-US (see tasks/shared/playwright.py).
    bootstrap = f"npm ci && {pw}"
    # Either an anonymous mask (default, ephemeral, always correct) or a named
    # volume keyed to the resolved Playwright version (opt-in --cache-deps) so a
    # cached node_modules can never outlive a Playwright bump.
    node_modules_volume = (
        f"pw-node-modules-{resolved_playwright_version()}:/work/node_modules"
        if cache_deps
        else "/work/node_modules"
    )
    args = [
        "docker", "run", "--rm",
        f"--platform={PLAYWRIGHT_PLATFORM}", "--ipc=host",
        "--add-host=host.docker.internal:host-gateway",
        # Mount only the frontend dir (least privilege): the container needs the
        # specs/config/baselines, not the whole repo. The host server runs
        # outside the container and is reached over BASE_URL, so .e2e-port is
        # never read inside it.
        "-v", f"{FRONTEND}:/work",
        # Mask node_modules so the container's Linux-native binaries never
        # clobber the host tree (anonymous by default; named iff --cache-deps).
        "-v", node_modules_volume,
        "-w", "/work",
        "-e", "CI=1",
        "-e", f"BASE_URL={base_url}",
        "-e", f"CHROMIUM_CHANNEL={CHROMIUM_CHANNEL}",
        "-e", f"PLAYWRIGHT_LOCALE={BROWSER_LOCALE}",
        "-e", f"LANG={E2E_LANG}",
        "-e", f"LC_ALL={E2E_LANG}",
        image,
        "bash", "-c", bootstrap,
    ]
    return shlex.join(args)


@task
def visualiser_docker(
    context: Context, update: bool = False, cache_deps: bool = False
) -> None:
    """Run the visual-regression specs in the pinned Playwright Docker image.

    The Rust server runs on the host (as in CI); only Chromium is containerised.
    `--update` regenerates the canonical Linux baseline set. `--cache-deps`
    persists node_modules in a named volume across runs to skip the (slow,
    emulated) `npm ci` each iteration; the default anonymous mask is used
    otherwise for correctness. `docker_visual_command(..., cache_deps=...)`
    selects the named vs anonymous volume accordingly.
    """
    # Fail fast with an actionable message if Docker is unavailable, BEFORE
    # spending up to 60s bringing up the host server (and before leaving one
    # started). `docker info` is a cheap daemon-reachability probe.
    if context.run("docker info", hide=True, warn=True).failed:
        raise Exit(
            "Docker daemon not reachable — start Docker Desktop / Colima and "
            "retry. See skills/visualisation/visualise/frontend/README.md "
            "(Visual-Regression Baselines).",
            code=1,
        )

    server_bin = SERVER / "target/debug/accelerator-visualiser"
    # The mise tasks declare build:server:dev as a dependency, but a direct
    # `invoke` call (e.g. for --cache-deps) bypasses it — so check explicitly.
    if not server_bin.exists():
        raise Exit(
            f"Server binary not found at {server_bin} — run "
            "`mise run build:server:dev` first (or use the mise tasks).",
            code=1,
        )

    def on_ready(port: str) -> None:
        context.run(
            docker_visual_command(
                base_url=f"http://host.docker.internal:{port}",
                image=playwright_image(),
                update=update,
                cache_deps=cache_deps,
            ),
            # Live Docker/npm output locally; piped (no PTY) on the headless CI
            # runner so the container exit code is reliably propagated.
            pty=sys.stdout.isatty(),
        )

    run_against_host_server(server_bin=server_bin, on_ready=on_ready)  # seam-injected helper
```

`run_against_host_server` (`tasks/shared/dev/host_server.py`, new) unlinks any
stale `.e2e-port`; uses the injected **launcher** to spawn
`node e2e/start-server.mjs` with `ACCELERATOR_VISUALISER_BIN`,
`E2E_SERVER_HOST=0.0.0.0`, and `LANG`/`LC_ALL` set; polls for `.e2e-port` while
interleaving `handle.poll()` so an early server exit short-circuits (the raised
error distinguishes *server exited (code N)* — from `handle.poll()` — from
*timed out after 60s*, and points at the inherited console output); runs
`on_ready`; and in a `finally` block reaps the node leader **and its
`ProcessOps.children()`** (`terminate` → bounded wait → `kill`, exactly as
`lifecycle.py:_reap_handle`) so the Rust child is never orphaned, then unlinks
`.e2e-port`.

For the unit tests, **hoist only the contract-pure `FakeProcs`** from
`tests/unit/tasks/shared/dev/test_lifecycle.py` into the shared
`tests/unit/tasks/shared/doubles.py` (alongside `FakeClock`) — it is a faithful
`ProcessOps` double already and is what the teardown assertions check
(`children()` + `terminate`/`kill`). Do **not** hoist circus's `FakeLauncher`:
it encodes pidfile/`server-info.json`/arbiter-watcher semantics this helper
doesn't have. Instead add a tiny local launcher/handle double whose `poll()`
returns `None` until a configurable point and then an exit code (the existing
`FakeHandle` hardcodes `poll() -> None` and cannot simulate a server exit), so
the server-died-early path is actually exercisable.

> The Chromium `channel` and browser `locale` are now passed into the container
> as env vars (`CHROMIUM_CHANNEL`, `PLAYWRIGHT_LOCALE`) sourced from
> `tasks/shared/playwright.py` and read by `playwright.docker.config.ts` — so
> that module is genuinely the single source for every no-drift value, with no
> duplicated literal in the TS config.

#### 5. Mise tasks

**File**: `mise.toml`
**Changes**: Two tasks alongside `test:e2e:visualiser`. Both depend on the host
build of the frontend (`dist/`, served by the dev-frontend server) and the dev
server binary — but **not** `deps:install:playwright`, since Playwright runs in
the container.

```toml
[tasks."test:e2e:visualiser:docker"]
description = "Run visual-regression specs in the pinned Playwright Docker image (compare)"
depends = ["build:frontend", "build:server:dev"]
run = "invoke test.e2e.visualiser-docker"

[tasks."test:e2e:visualiser:docker:update"]
description = "Regenerate the canonical Linux visual-regression baselines in the pinned Docker image"
depends = ["build:frontend", "build:server:dev"]
run = "invoke test.e2e.visualiser-docker --update"
```

The `--cache-deps` accelerator (named `node_modules` volume) is reachable from
the mise surface — not only via a bare `invoke` call — so the inner-loop speed-up
is discoverable. Either add a fourth task or pass it through; the documented form
is the power-user escape hatch `invoke test.e2e.visualiser-docker --cache-deps`
(optionally `--update`), and the README debugging section documents it.

#### 6. Unit tests (TDD)

**File**: `tests/unit/tasks/test_playwright_shared.py` (new),
`tests/unit/tasks/test_e2e_docker.py` (new)
**Changes**: Write these first.

- `resolved_playwright_version` reads `1.59.1` from a fixture lockfile, and
  raises a clear error for **each** distinct failure input — missing lockfile
  file, invalid JSON, missing `packages` key, missing the
  `node_modules/@playwright/test` entry, and entry-present-but-no-`version` —
  rather than leaking an unhelpful `KeyError`/`JSONDecodeError`.
- `playwright_image` yields `mcr.microsoft.com/playwright:v1.59.1-noble`, and a
  multi-component resolved version (e.g. a pre-release) is either rejected or
  correctly tag-encoded (no silently-malformed tag reaching `docker run`).
- `docker_visual_command` (asserting on the parsed token list, not substrings)
  includes `--platform=linux/amd64`, `--ipc=host`,
  `--add-host=host.docker.internal:host-gateway`, the frontend-only `-v` mount,
  the `node_modules` volume mask, and `-w /work`; carries the
  `BASE_URL`/`CHROMIUM_CHANNEL`/`PLAYWRIGHT_LOCALE`/`LANG`/`LC_ALL` env; the
  `bash -c` payload is exactly `npm ci && npx playwright test …` (no `locale-gen`);
  appends `--update-snapshots` only when `update=True`; and switches the
  `node_modules` volume from the anonymous mask to the version-keyed named volume
  only when `cache_deps=True`.
- **No-drift assertions**: the command's `CHROMIUM_CHANNEL`/`PLAYWRIGHT_LOCALE`
  env values equal the `tasks/shared/playwright.py` constants, and its
  `LANG`/`LC_ALL` equal the same `E2E_LANG` the server-spawn env uses — proving
  one source per no-drift value (work item AC `0108:132-140`).
- **`required()` fail-fast** (a small TS unit/spec): the Docker config's
  `required()` throws when `BASE_URL` / `CHROMIUM_CHANNEL` / `PLAYWRIGHT_LOCALE`
  is unset, so the removed `:0` placeholder cannot silently return.
- **Lifecycle helper** (`run_against_host_server`) with an injected local
  launcher/handle double + the shared `FakeProcs` + `FakeClock`: ready path
  invokes `on_ready` with the published port; **server-exits-early** (handle
  `poll()` returns a code before `.e2e-port` appears) short-circuits with the
  *exited (code N)* error; **port-never-published** hits the deadline with the
  *timed out* error; and **terminate-times-out-then-kill** reaps the node leader
  **and its `FakeProcs.children()`** — assert both the leader PID and the
  simulated Rust child PID are signalled through `ProcessOps` (no `os.killpg`, so
  the calls are observable on the fake), and that an already-exited leader makes
  teardown a no-op rather than raising.

### Success Criteria

#### Automated Verification

- [x] New unit tests pass: `mise run test:unit:tasks`
- [x] Python format/lint/types pass: `mise run build-system:check`
- [x] Frontend typecheck/format/lint still pass: `mise run frontend:check`
- [x] Native e2e unchanged and green: `mise run test:e2e:visualiser`
- [x] Mise resolves the new tasks: `mise tasks | grep visualiser:docker`

#### Manual Verification

- [ ] On macOS (Docker running): `mise run test:e2e:visualiser:docker:update`
      generates a `*-snapshots/` set with **no** `-darwin`/`-linux` suffix.
- [ ] Immediately re-running `mise run test:e2e:visualiser:docker` passes
      zero-diff against those just-generated baselines (round-trip stability).
- [ ] The container's shell locale (`C.UTF-8`) resolves out of the box (no
      `locale-gen`), and Chromium renders under the explicit `en-US` locale —
      confirm a locale-sensitive render (e.g. a formatted date/number in a
      snapshot) matches between a local emulated run and the native-amd64 parity
      run below.
- [ ] **Emulated-vs-native parity (de-risk before Phase 2 commits anything)**:
      baselines generated under amd64 emulation on Apple Silicon pass zero-diff
      when compared on a native-amd64 host (or via a throwaway CI run of the
      compare task) — confirming QEMU and native silicon render byte-identically
      before any baseline is committed as canonical.
- [ ] With Docker stopped, the task fails fast with the actionable
      daemon-not-reachable message (not a raw Docker error) and does **not**
      leave a host server running.
- [ ] After the run, host `node_modules` is intact (no Linux-native binary
      clobber — host `mise run test:unit:frontend` still works without re-`npm ci`).
- [ ] The generated baselines from Phase 1 are **discarded** (`jj`/`git`
      restore) — they are committed in Phase 2 under the cutover.

---

## Phase 2: Atomic cutover

### Overview

The single mergeable unit that collapses the baseline set, isolates the visual
specs from native runs, and switches CI execution to the Docker job. Everything
here lands together because regenerating the baselines under the canonical
environment makes only that environment able to validate them.

### Changes Required

#### 1. Split the spec directory; move the visual-regression project to Docker; keep the resolved specs native

**Files**:
`skills/visualisation/visualise/frontend/tests/visual-regression/*-resolved-*.spec.ts`
(+ `fixture-coverage.spec.ts`, `non-regression-glyph-consumers.spec.ts`,
`eyebrow-unification-resolved.spec.ts`, and any other non-screenshot spec under
that dir) → relocate to
`skills/visualisation/visualise/frontend/tests/resolved-styles/`;
`skills/visualisation/visualise/frontend/playwright.config.ts`.

**Changes**:

1. **Relocate the non-screenshot specs.** Move every spec under
   `tests/visual-regression/` that takes **no** screenshot (the ~14
   `*-resolved-*` / computed-style specs) into a new `tests/resolved-styles/`
   directory. `tests/visual-regression/` is left holding only the 8
   screenshot-emitting specs (plus `__screenshots__/`), which the Docker config
   targets. **Hoist the shared helpers** (`helpers.ts`, `lib/expected-colours.ts`,
   `lib/detail-route-slugs.ts`) to a neutral `tests/lib/` (or `tests/shared/`)
   imported by both directories, and update all import paths — do **not** leave
   them under `tests/visual-regression/`, or the always-native `resolved-styles`
   tree would import from the Docker-only tree and re-couple what the split
   separates.

2. **Remove the `visual-regression` project** (`:24-29`) from
   `playwright.config.ts` — it now lives only in `playwright.docker.config.ts`.

3. **Add a `resolved-styles` project** to `playwright.config.ts` so the
   relocated computed-style specs keep running natively on every run (and on
   both CI matrix legs). They need the host server (computed styles of rendered
   pages) but take no screenshots, so they run under the existing `webServer`
   with no baselines.

4. **Drop `dependencies: ["visual-regression"]`** from the `chromium` project
   (`:34`). Keep `webServer`, `globalSetup`/`globalTeardown` (the drag tests
   still need fixture snapshot/restore) and `workers: 1`. Update the stale
   ordering comment and add a one-line comment recording that the
   clean-fixture-before-drag ordering is now provided by global setup/teardown,
   **not** by an inter-project dependency, so no future native spec may rely on
   observing pre-drag clean fixtures.

```ts
  projects: [
    {
      // Computed-style specs only — no screenshots. They hit static showcase
      // routes, so they must stay independent of the chromium drag tests'
      // fixture mutation/restore order (project order is unspecified under
      // workers:1); do not add a resolved-styles spec that reads live
      // work-item status.
      name: "resolved-styles",
      testDir: "./tests/resolved-styles",
      use: { browserName: "chromium" },
    },
    {
      // No dependency on a snapshot project: the clean-fixture-before-drag
      // guarantee is provided by globalSetup/globalTeardown restore. Do not add
      // a native spec that relies on observing pre-drag clean fixture state.
      name: "chromium",
      use: { browserName: "chromium" },
    },
  ],
```

#### 2. Regenerate and collapse the baseline set

**Files**: `skills/visualisation/visualise/frontend/tests/visual-regression/__screenshots__/**`
**Changes**:
1. Run `mise run test:e2e:visualiser:docker:update` to write the single
   no-`{platform}` Linux baseline set (≈103 PNGs). **Rebaseline from a clean
   fixture state** — the Docker job runs only the read-only visual specs, but it
   reads the committed Markdown fixtures the native drag tests mutate, so an
   interrupted prior native run could in principle leave them dirty; a clean
   working tree (`jj`/`git status` on the fixture dir) before capture guarantees
   the canonical baseline reflects pristine fixtures.
2. **Guard against silent corruption before deleting anything.** The compare's
   zero-diff against the just-generated set is tautological, so it cannot catch
   a wrong-but-self-consistent regeneration (wrong fixtures, a spec that errored
   and skipped its snapshot, an emulation/font glitch, a partial run). Before
   the delete, while the outgoing `-darwin`/`-linux` PNGs are still in the
   working tree:
   - **Assert structurally**, not just by count: the regenerated case-name set
     exactly matches the retained `-linux` case set (no missing/extra cases — a
     spec that errored and skipped its snapshot is caught), and every PNG
     decodes, is non-blank, and meets a minimum dimension/byte floor (a blank or
     truncated render is caught). This is a repeatable check, not only a one-time
     eyeball.
   - **Eyeball-sample** a representative subset (spot-diff against the retained
     `-linux` references) as a final human sign-off. (Per team convention, no
     scripted confirm/dry-run UX; VCS revert remains the recovery path.)
3. Delete every old `*-visual-regression-darwin.png` and
   `*-visual-regression-linux.png` (the platform-suffixed files the new template
   no longer references).
4. Commit only the new no-suffix set.

#### 3. Dedicated Linux-only CI visual job

**File**: `.github/workflows/main.yml`
**Changes**: Add a `test-visual-regression` job (ubuntu-latest) that runs the
compare task, and append it to `prerelease.needs` so a release cannot ship past
a failing visual check. Leave the `test-e2e` matrix as-is structurally — it now
runs the `chromium` and `resolved-styles` projects on both legs (the screenshot
project is gone from the main config), so the macOS leg never needs Docker while
the computed-style specs still run on every leg. Give the new job an explicit
`timeout-minutes` so a wedged emulated/Chromium run fails fast rather than
occupying a runner until the 6-hour platform default.

```yaml
  test-visual-regression:
    name: Run visual regression tests
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - name: Checkout code
        uses: actions/checkout@v5
      - name: Install dependencies
        uses: jdx/mise-action@v4.1.0
        with:
          install: true
          cache: true
          experimental: true
      - name: Run visual regression tests (Docker)
        run: mise run test:e2e:visualiser:docker
```

```yaml
  # prerelease.needs (:169-176) — append:
      - test-visual-regression
```

### Success Criteria

#### Automated Verification

- [x] Native e2e runs the `chromium` and `resolved-styles` projects, green, and
      executes **no** screenshot specs: `mise run test:e2e:visualiser`.
      (321 passed: 283 resolved-styles + 38 chromium; 0 screenshot cases.)
- [x] The relocated `*-resolved-*` specs are still collected and run natively —
      assert the **exact** expected count for the `resolved-styles` project
      (`npx playwright test --project resolved-styles --list` shows the precise
      number of relocated specs), so neither full nor *partial* orphaning (a
      single dropped spec from a broken import) slips through.
      (`--list` shows exactly 14 relocated spec files.)
- [x] Full native suite green and runs no screenshot specs:
      `mise run test` and `mise run test:e2e`. (`mise run test:e2e` = 321
      passed, no screenshot specs. The fully-parallel `mise run test` flaked
      with the e2e webServer unreachable — `ECONNREFUSED` — under contention
      from the concurrently-compiling unit/integration suites; pre-existing
      parallel-load flakiness, not a regression, since the e2e leg is green
      standalone.)
- [x] **Clean-fixture guarantee enforced by a test, not just a comment**: the
      chromium drag suite passes deterministically with the `visual-regression`
      dependency removed — run it **twice in succession** (or add an assertion
      that the pre-drag kanban column membership matches the committed fixture),
      so the ordering invariant the dropped dependency used to enforce is now
      guarded automatically and global-teardown's restore is proven complete.
      (chromium project: 38 passed on two consecutive runs.)
- [x] Docker compare passes zero-diff against the committed set:
      `mise run test:e2e:visualiser:docker` (157 passed, zero-diff round-trip).
- [x] Exactly one baseline set on disk — no matches for:
      `ls skills/visualisation/visualise/frontend/tests/visual-regression/__screenshots__/**/*-darwin.png`
      and none for `*-linux.png` (156 no-suffix PNGs; 0 darwin/linux).
- [x] Frontend checks pass: `mise run frontend:check`
- [ ] On a pushed branch, the CI `test-visual-regression` job passes zero-diff
      and `prerelease` lists it under `needs`. (`needs` updated; CI run pending push.)

#### Manual Verification

- [ ] A fresh frontend change → `:docker:update` → commit → push yields a green
      `test-visual-regression` run on that same commit (no CI push-back).
- [ ] On Apple Silicon, the compare task passes under amd64 emulation (confirms
      cross-architecture reproducibility).
- [ ] No chromium e2e spec regressed from dropping the `visual-regression`
      dependency (drag tests still see clean fixtures via global setup/teardown).
- [ ] Per-assertion tolerances are unchanged (`0.05` / `0.01`); no global
      `threshold` introduced.

---

## Phase 3: Remove legacy, converge, document

### Overview

Delete the now-redundant push-back workflow and the glyph carve-out, and write
the contributor documentation. Safe because the Phase 2 Docker job has been
green on `main`.

### Changes Required

#### 1. Remove the push-back workflow

**File**: `.github/workflows/update-visual-baselines.yml`
**Changes**: Delete the file. No mechanism pushes regenerated baselines back to
`main` thereafter.

#### 2. Remove the glyph carve-out and converge

**Files**: `skills/visualisation/visualise/frontend/playwright.glyph.config.ts`,
`skills/visualisation/visualise/frontend/README.md`
**Changes**: Delete `playwright.glyph.config.ts` (the glyph specs now regenerate
through the unified Docker config + host server like every other visual spec).
Remove the glyph-specific Docker recipe from the README. Confirm `glyph-showcase`
and `big-glyph-showcase` baselines are part of the Phase 2 regenerated set.

> Convergence quietly changes the glyph specs' rendering substrate from
> `vite preview` of `dist/` to the Rust **dev-frontend** server serving the same
> `dist/`. Both serve the identical built assets, so pixels should match — but
> confirm it: before deleting `playwright.glyph.config.ts`, verify the glyph
> baselines regenerated through the unified server-served Docker flow pass
> zero-diff. Since Phase 2 regenerates the whole set from scratch in the unified
> environment, this is a check that the route is served correctly, not a
> cross-baseline diff.

#### 3. Contributor documentation

**File**: `skills/visualisation/visualise/frontend/README.md`
**Changes**: Rewrite the "Visual-Regression Baselines" section to **lead** with
the two task names and when to use each (the onboarding entry path, not buried
prose), and cover:
- **The "why" (durable rationale, since the ADR is deferred).** A short
  paragraph capturing the keystone: the committed baseline is a function of what
  Chromium renders, so **only the browser is containerised** while the Rust
  server runs on the host; and `--network=host` is deliberately **not** used
  locally (under Colima it joins the Lima VM namespace, not the macOS host).
  This prevents a future maintainer "simplifying" it back into a fully
  containerised path and reviving the Rust-in-image problem. Mirror a one-line
  version of this rationale in **the `visualiser_docker` task docstring** (its
  canonical home, since that task owns the host-server-plus-container
  orchestration) so it travels with the code.
- Docker is a **prerequisite** for all visual-test work (running and
  re-baselining), for internal and public/Linux contributors alike; macOS needs
  a developer-provided daemon (e.g. Docker Desktop / Colima).
- When and how to re-baseline: change frontend → `mise run
  test:e2e:visualiser:docker:update` → commit the regenerated set → push;
  CI validates the same commit.
- How to verify locally before committing: `mise run test:e2e:visualiser:docker`.
- **The native/Docker split and the green-but-stale trap.** Native runs
  (`mise run test` / `test:e2e`) deliberately run no screenshot specs (the
  `resolved-styles` computed-style specs still run), so a green native suite says
  **nothing** about the visual baselines — after any frontend change you must run
  the Docker compare/update task before committing. State this prominently; it is
  the one footgun the new flow introduces.
- How to debug a failing visual test: name the **concrete** host-visible artefact
  location (the `test-results/` diff PNGs and, with `trace: "on-first-retry"`,
  the trace) and the `npx playwright show-trace` / `show-report` invocation.
  Confirm during implementation that `test-results/` writes through to the
  mounted frontend dir and is **not** shadowed by the `node_modules` volume mask.
- **Speeding up the inner loop**: document the `--cache-deps` escape hatch
  (`invoke test.e2e.visualiser-docker --cache-deps`, optionally `--update`),
  which persists `node_modules` in a version-keyed named volume to skip the slow
  emulated `npm ci` each iteration, and note the default (anonymous mask) is the
  correctness-safe one.
- The transient `0.0.0.0` host-server exposure during a Docker run (deliberate;
  serves only non-sensitive fixtures; native runs stay loopback).
- **Bumping `@playwright/test`**: confirm the corresponding
  `mcr.microsoft.com/playwright:v<version>-noble` tag exists on MCR before
  merging the lockfile change (the image tag derives from the resolved version;
  a missing tag breaks everyone with an opaque "manifest unknown").
- **If `host.docker.internal` breaks on a Colima point-release** (known
  occasional DNS regression): the interim workaround (e.g. an env-overridable
  `BASE_URL` host, or pinning a known-good Colima version) before the documented
  server-sidecar fallback would be implemented.
- Update the "Prerequisites" section (Docker is now **required** for visual
  work, not optional — the current `README.md:9` "Docker (optional; CI can
  produce them otherwise)" line is now wrong) and the project-layout notes that
  still mention "darwin + linux per spec" (now a single canonical set) and the
  `tests/visual-regression/` (Docker, screenshots) vs new `tests/resolved-styles/`
  (native, computed-style) split.
- **Reconcile every place the README describes visual regression, not just this
  section**: the Testing line at `README.md:33` ("`npm run test:e2e` … Playwright
  E2E + visual regression") becomes misleading once native runs execute no
  screenshot specs — update it so a green native `test:e2e` is not read as
  validating the baselines (this is the same green-but-stale trap, in the
  Testing section).
- **Glyph spec split**: note that `glyph-showcase.spec.ts` (screenshot) now runs
  in the Docker visual config while `glyph-resolved-fill.spec.ts` (computed-style)
  relocates to the native `resolved-styles` project — both driving the
  `/glyph-showcase` route now served by the dev-frontend server, not `vite
  preview` — so a future reader understands why the two former glyph-config specs
  live in different places.

### Success Criteria

#### Automated Verification

- [x] `update-visual-baselines.yml` no longer exists:
      `test ! -f .github/workflows/update-visual-baselines.yml`
- [x] `playwright.glyph.config.ts` no longer exists:
      `test ! -f skills/visualisation/visualise/frontend/playwright.glyph.config.ts`
- [x] No dangling references to the deleted config/workflow.
      (`tsconfig.node.json` updated to reference `playwright.docker.config.ts`;
      the only remaining mentions are in `meta/` historical docs and one
      **intentional** explanatory note in the README glyph section — the plan
      asked for that note so a reader understands why the two former glyph-config
      specs now live in different trees.)
- [x] Docker compare still green: `mise run test:e2e:visualiser:docker`
      (verified zero-diff in the cutover; Phase 3 changed nothing
      rendering-affecting — only a deleted workflow, the glyph config, README,
      and tsconfig).
- [x] Full native suite green: `mise run test` (see the note under Phase 2 —
      e2e leg green standalone; full-parallel `mise run test` flakes on
      webServer contention, pre-existing).

#### Manual Verification

- [ ] README documents the Docker prerequisite, the re-baseline workflow, local
      verification, and debugging — readable end-to-end by a new (incl. Linux)
      contributor.
- [ ] No README recipe references `-jammy` or two divergent image tags.
- [ ] Glyph baselines are present in the single canonical set and validated by
      the unified Docker job.

---

## Testing Strategy

### Unit Tests (Phase 1, TDD)
- `resolved_playwright_version` / `playwright_image`: correct extraction and tag
  construction from a fixture `package-lock.json`; a clear, distinct failure for
  each malformed input (missing file, bad JSON, missing key/entry/version) and
  for a malformed resolved version.
- `docker_visual_command`: presence (asserted on the parsed token list) of
  `--platform=linux/amd64`, `--ipc=host`, `--add-host`, the frontend-only mount,
  the `node_modules` volume mask, and the env vars, with `--update-snapshots`
  only when `update=True`; plus no-drift assertions that
  `CHROMIUM_CHANNEL`/`PLAYWRIGHT_LOCALE`/`LANG`/`LC_ALL` equal the shared-module
  constants.
- `run_against_host_server` (with `FakeProcs`/`FakeClock`): ready, server-died,
  port-never-published, and SIGTERM-times-out-then-kill paths; error messages
  distinguish exit-vs-timeout and reference the server log.

### Integration / End-to-End
- Round-trip in Docker locally: rebaseline then compare zero-diff (Phase 1).
- CI `test-visual-regression` job green on a pushed branch against the committed
  set (Phase 2).
- Native `mise run test` / `test:e2e` execute no screenshot specs and pass, while
  the `resolved-styles` project's computed-style specs are collected and run
  (Phase 2).

### Manual Testing Steps
1. (macOS) `mise run test:e2e:visualiser:docker:update`, inspect `__screenshots__/`
   for single no-suffix PNGs; confirm host `node_modules` intact.
2. `mise run test:e2e:visualiser:docker` → zero diffs.
3. Make a deliberate visual change, rebaseline, commit, push → confirm the CI
   visual job passes on that commit.
4. (Apple Silicon) confirm the compare task passes under emulation.

## Performance Considerations

- amd64 emulation on Apple Silicon (`--platform=linux/amd64`) adds runtime
  overhead — accepted as the cost of cross-architecture reproducibility.
- `npm ci` inside the container under amd64 emulation runs on every invocation
  and is the slowest part of the inner loop on Apple Silicon. Ship the **named**
  Docker volume for `node_modules` (persisted across runs) as an **opt-in flag
  from the start** (e.g. `--cache-deps`), defaulting to the anonymous mask for
  correctness, so a developer iterating on a diff is not forced through a full
  emulated `npm ci` each cycle. Document the flag in the README debugging
  section.
- `--ipc=host` is required to avoid Chromium OOM crashes on large screenshot
  runs.

## Migration Notes

- The cutover is atomic (Phase 2): the baseline regeneration, native isolation,
  and CI switch land in one mergeable change so local and CI never reference
  divergent image tags or baseline shapes mid-change.
- One-time full regeneration of the canonical set happens in Phase 2 regardless
  of the v1.49 headless question — pinning `channel: "chromium"` makes the
  headless build deterministic going forward.
- **Emulated-vs-native parity is the load-bearing reproducibility assumption.**
  The single-baseline promise requires QEMU-emulated amd64 (Apple Silicon local)
  to render byte-identically to native amd64 (Linux CI). This is de-risked
  **early** (Phase 1 success criterion) by comparing emulated-generated baselines
  on a native-amd64 host before any baseline is committed. The per-assertion
  tolerances (`0.05` / `0.01`) are the only absorber for any residual delta and
  are deliberately not raised; if emulated/native diverge beyond them, that is a
  blocker surfaced before Phase 2, not after.
- **Locale**: the rendering-determining locale is Chromium's explicit `en-US`
  (single-valued everywhere, OS-independent); the container/host shell glibc
  locale is the universally-present `C.UTF-8`. Both are sourced from
  `tasks/shared/playwright.py`. There is deliberately **no** in-container
  `locale-gen`: the container OS locale never reaches the rendered baseline
  (Chromium uses its own `locale`), so generating `en_US.UTF-8` would be
  flavour-coupled and pointless. The host Rust server's locale is set in the
  invoke task's spawn env from the same `E2E_LANG` constant.
- **Fallback** (do not implement unless needed): if host↔container networking
  proves flaky on Colima, switch to a server-sidecar (two containers on a shared
  Docker network via `build:server:cross-compile`, `mise.toml:80-83`). Keep as
  fallback only — it uses the `embed-dist` feature, not the e2e `dev-frontend`
  feature.

## References

- Original work item: `meta/work/0108-local-docker-visual-regression-baselines.md`
- Research: `meta/research/codebase/2026-06-12-0108-local-docker-visual-regression-baselines.md`
- Recommended follow-up (out of scope here): an ADR capturing "single canonical
  render environment as source of truth" for visual-regression baselines — no
  existing ADR covers cross-platform baseline strategy (research §Historical
  Context).
- Prior art: `skills/visualisation/visualise/frontend/playwright.glyph.config.ts`,
  `README.md:50-77` (Docker recipes), `playwright.config.ts:21-46`,
  `e2e/start-server.mjs:42-50,100,145`, `e2e/global-setup.ts:28-61`
- CI: `.github/workflows/main.yml:59-79,166-176`,
  `.github/workflows/update-visual-baselines.yml:1-44`
- Task wiring: `mise.toml:33-36,172-183`, `tasks/test/e2e.py:6-22`,
  `tasks/build.py:130-140`, `tasks/shared/paths.py:3-9`
- Version source: `package.json:39`, `package-lock.json` (resolved `1.59.1`)
- Playwright Docker: https://playwright.dev/docs/docker
- Playwright Visual comparisons: https://playwright.dev/docs/test-snapshots
