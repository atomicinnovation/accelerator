---
type: codebase-research
id: "2026-06-12-0108-local-docker-visual-regression-baselines"
title: "Research: Local Docker-Based Visual Regression Baseline Generation (0108)"
date: "2026-06-12T16:10:19+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0108"
parent: "work-item:0108"
topic: "Local Docker-Based Visual Regression Baseline Generation"
tags: [research, codebase, visual-regression, playwright, docker, ci, baselines, testing]
revision: "3234a3a815601552d5e3c81a0b41c5d232d87c50"
repository: "miscellaneous"
last_updated: "2026-06-12T16:15:40+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Added follow-up resolving open question #1 — run the Rust server on the host via mise (as CI does) and containerise only Chromium"
schema_version: 1
---

# Research: Local Docker-Based Visual Regression Baseline Generation (0108)

**Date**: 2026-06-12T16:10:19+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 3234a3a815601552d5e3c81a0b41c5d232d87c50
**Branch**: (detached HEAD / jj working copy)
**Repository**: accelerator (miscellaneous jj workspace)

## Research Question

Research the codebase to support work item 0108 — moving visual-regression
baseline generation into a pinned official Playwright Linux Docker image,
collapsing the per-platform (`-darwin`/`-linux`) baseline set into a single
canonical Linux set, isolating visual specs from native macOS runs, extracting
them into a dedicated Linux-only CI job, and removing the CI push-back
workflow. Verify the work item's codebase anchors and surface anything the
plan must account for.

## Summary

The work item's anchors are **accurate** but **incomplete in one decisive
way**: there is already substantial Docker prior art in the repo that the work
item never references, and it changes the shape of the work materially.

Headline findings:

1. **A Docker-friendly carve-out config already exists.**
   `playwright.glyph.config.ts` is a standalone, frontend-only config built
   *specifically* to regenerate baselines inside a vanilla Playwright Linux
   image (`vite preview` of the built `dist/`, no Rust server). The README
   already documents a working `mcr.microsoft.com/playwright:v1.59.1-noble`
   recipe against it. This is not greenfield — it is the seed of the pattern
   0108 wants to generalise.

2. **The central technical obstacle is the Rust server, not Docker plumbing.**
   The main `playwright.config.ts` brings up the real Rust visualiser binary
   via `e2e/start-server.mjs` (which `cargo build`s it if not pre-built).
   Most snapshot-producing specs need that server for `/api/*` data. Getting
   it to run inside a pinned `linux/amd64` Playwright image — which ships **no
   Rust toolchain** — is the load-bearing unsolved problem. The glyph config
   exists precisely because Rust-in-Docker was deemed "awkward". The work
   item's "L" sizing rests on this being solvable; it deserves to be the
   plan's first spike.

3. **"23 specs" overstates the baseline surface.** Only **~8** specs emit PNGs
   (~103 cases → ~206 committed PNGs, ~50/50 darwin/linux). The other ~15
   `*-resolved-*` specs assert computed DOM styles and write **no** snapshots —
   they are unaffected by the baseline collapse and run fine natively.

4. **Spec isolation is partly done but not enough.** The `visual-regression`
   Playwright *project* exists, but the dependent `chromium` project relies on
   it running first (`workers: 1` + `dependencies: ["visual-regression"]`)
   because they share mutable fixtures. Extracting visual-regression to a
   separate job/run breaks that ordering guarantee unless handled.

5. **The version pin is already duplicated.** `@playwright/test` is `^1.59.1`
   in `package.json`, but the README Docker recipe hard-codes `v1.59.1-noble`.
   The committed `package-lock.json` resolves it exactly to `1.59.1`, giving an
   install-independent single source of truth for an image-tag wrapper.

6. **No locale, no Chromium `channel`, no `snapshotPathTemplate` today** — all
   three are net-new, exactly as the work item states.

7. **No ADR covers visual regression / cross-platform baselines / CI testing
   strategy.** This change (single canonical environment as source of truth) is
   an architectural decision worth an ADR.

## Detailed Findings

### 1. Specs and baselines — the real surface area

**Specs**: `skills/visualisation/visualise/frontend/tests/visual-regression/`
— 23 `.spec.ts` files plus 3 helper/lib files (`helpers.ts`,
`lib/detail-route-slugs.ts`, `lib/expected-colours.ts`).

**Critical distinction the work item blurs**: only a subset emit PNG
snapshots. Eight `*-snapshots/` directories exist under `__screenshots__/`:

| Spec | Cases | PNGs (×2 platforms) |
|------|-------|---------------------|
| `glyph-showcase.spec.ts` | 51 | 102 |
| `chip-showcase.spec.ts` | 24 | 48 |
| `big-glyph-showcase.spec.ts` | 13 | 26 |
| `tokens.spec.ts` | 7 | 14 |
| `kanban-card-showcase.spec.ts` | 3 | 6 |
| `code-syntax-showcase.spec.ts` | 2 | 4 |
| `task-list-visual.spec.ts` | 2 | 4 |
| `library-doc-view.spec.ts` | 1 | 2 |

≈103 cases → **≈206 committed PNGs**, split ~50/50 darwin/linux. Collapsing the
set deletes ~half (~103 `-darwin.png` files). The ~15 `*-resolved-*` specs
(e.g. `chip-resolved-colours.spec.ts`, `task-list-resolved-styles.spec.ts`)
assert `getComputedStyle` values, take **no** screenshots, and so carry no
tolerance options and no baselines — they are orthogonal to this work and run
natively without issue.

**Naming**: `<case>-visual-regression-{darwin,linux}.png`. The `-visual-regression`
segment is the Playwright project name; `{darwin,linux}` is Node's
`process.platform` from Playwright's default `snapshotPathTemplate`.

### 2. Playwright config — `playwright.config.ts` (verified)

- **Projects** (`playwright.config.ts:21-37`): `visual-regression`
  (`testDir: ./tests/visual-regression`, `snapshotDir: ./tests/visual-regression/__screenshots__`,
  `browserName: chromium`, lines 24-29) and a dependent `chromium` project
  (`dependencies: ["visual-regression"]`, line 34, inheriting top-level
  `testDir: ./e2e`).
- **`workers: 1`** (line 13) and the `chromium → visual-regression` dependency
  exist *together* so snapshots are captured against clean fixtures before the
  kanban drag tests mutate work-item statuses. **This is the coupling that
  makes "just split visual-regression out" non-trivial.**
- **`snapshotPathTemplate`: absent** — grep confirms no match anywhere. Default
  template `{snapshotDir}/{testFileName}-snapshots/{arg}{-projectName}{-platform}{ext}`
  produces the `-darwin`/`-linux` suffix. Dropping the token means *adding* an
  explicit template, not editing one.
- **Tolerances are per-assertion, no global `expect` block**:
  `maxDiffPixelRatio: 0.05` everywhere except `task-list-visual.spec.ts:19`
  which uses `0.01`. Exact sites: `tokens.spec.ts:22,41,58,76`,
  `chip-showcase.spec.ts:48`, `glyph-showcase.spec.ts:48`,
  `big-glyph-showcase.spec.ts:47`, `kanban-card-showcase.spec.ts:37`,
  `code-syntax-showcase.spec.ts:17`, `library-doc-view.spec.ts:19`. No
  `threshold` or `maxDiffPixels` anywhere (default per-pixel `threshold` 0.2).
- **No `channel`, no `headless`, no `locale`/`timezoneId`, no `LANG`/`LC_ALL`**
  in `use` or webServer env. All net-new, as the work item states.
- **Determinism controls already present**: `animations: "disabled"` per
  assertion; `await document.fonts.ready` before screenshotting; fonts are
  self-hosted woff2 (no Google Fonts network dependency — guarded by
  `src/styles/fonts.test.ts`); relative-time masking (`helpers.ts:7-8`); fixed
  viewports (1440×900 / 1024×768). `deviceScaleFactor` not set (default 1).

### 3. The Rust-server coupling — the decisive finding

`playwright.config.ts:38-46` `webServer.command: "node e2e/start-server.mjs"`.
`start-server.mjs`:
- Reads `ACCELERATOR_VISUALISER_BIN` (`:42`); **if absent, falls back to
  `cargo build --no-default-features --features dev-frontend`** (`:45-49`).
- Spawns the binary against committed Markdown fixtures under
  `server/tests/fixtures/meta`, inheriting full `process.env` + `FIXTURES_PATH`
  (`:114-117`).
- The server picks its own port, publishes it via `server-info.json`;
  `global-setup.ts:54-62` reads `.e2e-port` and sets `BASE_URL`.

**Implication**: the pinned `mcr.microsoft.com/playwright` image ships Node +
browsers + fonts but **no Rust toolchain**. To run the *full* snapshot set in
Docker, the plan must either (a) build a `linux/amd64` Rust binary outside the
container and mount it in via `ACCELERATOR_VISUALISER_BIN`, (b) install Rust
inside the container and build under emulation (slow on Apple Silicon), or
(c) extend the frontend-only `vite preview` approach (with mocked `/api/*`
data) to cover the server-dependent specs too. This is the single biggest risk
to the "L" estimate and should be the plan's opening spike.

### 4. Existing Docker prior art (NOT in the work item)

**`playwright.glyph.config.ts`** (read in full):
- Standalone config, `testMatch: ["glyph-showcase.spec.ts", "glyph-resolved-fill.spec.ts"]`
  (line 17), shares the same `snapshotDir` (line 18, so linux baselines land
  beside darwin ones), single `visual-regression` chromium project (26-31).
- **`webServer` runs `npx vite preview --host 127.0.0.1 --port ${PORT} --strictPort`**
  (line 33) — no Rust server. The header comment (lines 3-10) explicitly says
  the default config's Rust artefacts are "awkward to produce inside a
  Playwright Docker image", which is why this carve-out exists.

**`README.md` "Visual-Regression Baselines" (lines 50-77)** already documents:
- A generic Docker recipe `mcr.microsoft.com/playwright:v<version>-jammy ...
  --update-snapshots <spec>` with `--ipc=host` (lines 57-63).
- A concrete glyph-only recipe pinned to `v1.59.1-noble` running
  `npm ci && npm run build && npx playwright test --config
  playwright.glyph.config.ts --update-snapshots` (lines 72-74).
- A warning that the container leaves Linux-native `node_modules` binaries
  (esbuild, @rollup/rollup-*) that the host can't load — run `npm ci`
  afterwards (line 77). **This host/container `node_modules` clobbering is a
  real UX hazard the new task must design around** (e.g. mount only the repo,
  use a container-local `node_modules`, or document the re-`npm ci`).

Note the README uses **both** `-jammy` (generic) and `-noble` (glyph) flavours
inconsistently — the work item calls for picking one flavour matching CI.

### 5. Task wiring (verified)

- `mise.toml`: `test:e2e:visualiser` (`:172-175`, depends
  `deps:install:playwright` + `build:frontend` + `build:server:dev`) →
  `invoke test.e2e.visualiser`; `test:e2e` (`:177-179`) and `test` (`:181-183`)
  are pure aggregators; `deps:install:playwright` (`:33-36`) →
  `invoke deps.install-playwright`.
- `tasks/test/e2e.py:6-22`: runs `npm --prefix <frontend> run test:e2e` with
  `env={"ACCELERATOR_VISUALISER_BIN": <debug binary path>}`.
- `tasks/deps.py:28-32`: `npx --prefix {FRONTEND} playwright install --with-deps chromium`.
- `package.json:21`: `"test:e2e": "playwright test"`. No `--update-snapshots`
  or `--project` npm script exists today.
- **Convention to mirror for a Docker task**: one invoke task per mise task;
  all shell-outs via `context.run(<string>, env={...})`; paths from
  `tasks/shared/paths.py` (`FRONTEND`, `REPO_ROOT`). A new
  `test:e2e:visualiser:docker` / `...:rebaseline` mise task → invoke body
  calling `context.run(f"docker run ... {FRONTEND} ...")` fits the existing
  style exactly.

### 6. Version source of truth (verified + extended)

- `package.json:39`: `"@playwright/test": "^1.59.1"` (caret, not exact).
- **`package-lock.json` IS committed** (lockfileVersion 3) and resolves
  `node_modules/@playwright/test` → `"version": "1.59.1"` at line ~1254. A
  wrapper can read this **without installing** — the cleanest install-
  independent single source for the image tag. Post-`npm ci`,
  `npx playwright --version` is the alternative.

### 7. CI workflows (verified)

- **`main.yml` `test-e2e` job (`:59-79`)**: matrix `[ubuntu-latest, macos-latest]`,
  three steps (checkout, `jdx/mise-action@v4.1.0` with `install/cache/experimental`,
  `mise run test:e2e`). No Docker, no `container:`, no `services:`, no locale
  env anywhere in either workflow. Browsers installed transitively via the mise
  task graph, not a discrete step.
- **Release gating**: seven test/check jobs run in parallel with no inter-job
  `needs:`. `prerelease` (`:166-176`) gates on all seven; `release` gates on
  `prerelease`. **A new visual-regression job must be appended to
  `prerelease.needs` (`:169-176`)** so a release can't ship past a failing
  visual check.
- **`update-visual-baselines.yml`** (to be removed): `workflow_dispatch`-only,
  `ubuntu-latest`, `permissions: contents: write` (`:10-11`), checkout with
  `GITHUB_TOKEN` (`:14-17`), builds explicitly (`:26-30`), runs
  `npx playwright test --project visual-regression --update-snapshots`
  (`:32-36`, working-dir the frontend, `ACCELERATOR_VISUALISER_BIN` env), then
  commits `__screenshots__/` and `git push origin HEAD:${{ github.ref_name }}`
  (`:38-44`). The push under `GITHUB_TOKEN` is exactly why it can't re-trigger
  Main CI (matches project memory).

## Code References

- `skills/visualisation/visualise/frontend/playwright.config.ts:13` — `workers: 1`
- `skills/visualisation/visualise/frontend/playwright.config.ts:21-37` — projects (`visual-regression`, dependent `chromium`)
- `skills/visualisation/visualise/frontend/playwright.config.ts:38-46` — `webServer` → `start-server.mjs`
- `skills/visualisation/visualise/frontend/playwright.glyph.config.ts:1-40` — Docker-friendly `vite preview` carve-out
- `skills/visualisation/visualise/frontend/README.md:50-77` — existing Docker baseline recipes + `node_modules` clobber warning
- `skills/visualisation/visualise/frontend/e2e/start-server.mjs:42-49,114-117` — Rust binary resolution / `cargo build` fallback / env inheritance
- `skills/visualisation/visualise/frontend/e2e/global-setup.ts:54-62` — `.e2e-port` → `BASE_URL`
- `skills/visualisation/visualise/frontend/tests/visual-regression/tokens.spec.ts:22` — `maxDiffPixelRatio: 0.05`
- `skills/visualisation/visualise/frontend/tests/visual-regression/task-list-visual.spec.ts:19` — `maxDiffPixelRatio: 0.01`
- `skills/visualisation/visualise/frontend/tests/visual-regression/__screenshots__/` — 8 `*-snapshots/` dirs, ~206 PNGs
- `skills/visualisation/visualise/frontend/package.json:21,39` — `test:e2e` script; `@playwright/test ^1.59.1`
- `skills/visualisation/visualise/frontend/package-lock.json:~1254` — resolved `@playwright/test` `1.59.1` (committed)
- `mise.toml:33-36,172-183` — task wiring
- `tasks/test/e2e.py:6-22` — `npm --prefix <fe> run test:e2e` + env
- `tasks/deps.py:28-32` — `playwright install --with-deps chromium`
- `tasks/shared/paths.py:3-9` — `FRONTEND`, `REPO_ROOT` path constants
- `.github/workflows/main.yml:59-79` — `test-e2e` matrix job
- `.github/workflows/main.yml:166-176` — `prerelease.needs` (release gate to extend)
- `.github/workflows/update-visual-baselines.yml:1-44` — push-back workflow to remove

## Architecture Insights

- **Single canonical render environment as source of truth.** The strategic
  move is replacing "two host environments, two baselines" with "one
  containerised environment, one baseline". The architectural risk is anything
  that leaks host nondeterminism into the container: CPU arch (hence
  `--platform=linux/amd64`), Ubuntu flavour (`-jammy` vs `-noble` — pick one),
  Chromium build/`channel`, locale, and fonts. Fonts are already neutralised
  (self-hosted woff2, network-font test guard); the others are net-new pins.
- **The Rust server is the architectural seam.** The clean split is
  *frontend-only specs* (glyph, already carved out) vs *server-dependent specs*
  (tokens, library-doc-view, kanban, etc., needing `/api/*`). The plan's design
  hinges on which side of that seam each snapshot-producing spec sits, and how
  the server side is satisfied in a Rust-free image. `vite preview` + mocked
  API, or a mounted prebuilt `linux/amd64` binary, are the two coherent
  directions.
- **Isolation has two distinct meanings here** that must not be conflated:
  (a) *native macOS runs must skip the visual project* — solvable by a project
  filter / `testIgnore` / tag so `mise run test`/`test:e2e` don't execute it
  natively; (b) *the `chromium` project's ordering dependency on
  `visual-regression`* — extracting the snapshot project from the native e2e
  run removes the clean-fixture capture step, so the kanban-card snapshot's
  source of truth has to move into the Docker job too, or the dependency be
  restructured.
- **Version-pin wrapper**: derive the image tag from the committed
  `package-lock.json` resolved version (install-independent), consumed
  identically by the mise task and the CI job — the work item's no-drift
  requirement maps cleanly onto this.

## Historical Context

- `meta/work/0037-glyph-component.md` + `meta/plans/2026-05-12-0037-glyph-component.md`
  — established the dual-platform baseline convention and `maxDiffPixelRatio: 0.05`;
  the plan already contains an `mcr.microsoft.com/playwright:v<version>-jammy`
  invocation — the closest prior art and likely origin of the glyph carve-out.
- `meta/work/0033-design-token-system.md` (+ plan/research) — origin of the
  `tokens.spec.ts` VR convention and the 0.05 threshold.
- `meta/work/0095-...checkboxes....md` — the item whose context records that
  Linux baselines lag darwin (the pain 0108 fixes); uses the tighter 0.01.
- `meta/work/0082-big-glyph-hero-illustrations.md` — the only outstanding
  (non-`done`) item that captures new visual baselines; 0108 declares a soft
  sequencing edge over it.
- `meta/reviews/work/0108-local-docker-visual-regression-baselines-review-1.md`
  — flags MCR/Docker dependency, the baseline-collapse coupling with in-flight
  items, and the `maxDiffPixelRatio` bound criterion.
- **No ADR** covers visual regression, snapshot testing, Playwright, or
  cross-platform baseline strategy (closest: `ADR-0026-css-design-token-application-conventions.md`,
  which the VR specs guard but which says nothing about baselines). A genuine
  gap this work could fill.

## Related Research

- `meta/research/codebase/2026-05-12-0037-glyph-component.md` — snapshot naming
  scheme, dual-platform commit, cross-platform drift / flake risk.
- `meta/research/codebase/2026-06-09-0082-big-glyph-hero-illustrations.md` —
  `snapshotPathTemplate` `-visual-regression-<platform>` suffix, per-cell clips.
- `meta/research/codebase/2026-05-06-0033-design-token-system.md` — original
  VR convention and threshold rationale.
- `meta/research/codebase/2026-06-06-0101-unified-dev-task-for-visualiser.md` —
  Playwright/e2e execution environment.

## Open Questions

1. ~~**How does the Rust server get into the Rust-free Playwright image?**~~
   **RESOLVED** — see Follow-up Research below. The server does not go into the
   image at all: run it on the host via `mise run build:server:dev` (as CI does
   today) and containerise only Chromium. The remaining sub-questions are
   networking (`host.docker.internal`) and `webServer` decoupling, detailed below.
2. **Does v1.59.1 straddle the Playwright v1.49 Chromium headless change?**
   (Work item's own open question.) If the existing committed linux baselines
   predate it, a one-time full regeneration is needed regardless of strategy.
3. **`-jammy` or `-noble`?** The README uses both; CI currently uses neither
   (plain `ubuntu-latest`). Pick one flavour and pin it as the single tag base.
4. **How is the `chromium` project's clean-fixture dependency preserved** once
   `visual-regression` leaves the native e2e run? Does the kanban-card snapshot
   move wholly into the Docker job?
5. **`node_modules` host/container clobbering** — how does the new compare/
   rebaseline task avoid leaving Linux-native binaries in the host tree (the
   README's manual workaround is "re-`npm ci` afterwards")? A container-local
   `node_modules` or a build-in-container-then-discard pattern may be cleaner.
6. **Glyph config convergence** — does `playwright.glyph.config.ts` get folded
   into the new single Docker flow, or remain a separate frontend-only path?
   Leaving two configs risks the very drift 0108 sets out to eliminate.

## Follow-up Research [2026-06-12T16:15:40+00:00]

**Question**: Regarding the Rust server, could we use the existing mise support,
just as we do in CI — rather than putting Rust into the Docker image?

**Answer: yes, and it is the cleanest path — provided the server runs on the
*host* and only Chromium is containerised.**

### The decisive reframing

The committed baseline is determined by **what Chromium renders**, not by the
server's architecture. The Rust visualiser only emits HTML/CSS (the built
frontend it serves) and JSON (`/api/*`). Those bytes are identical whether the
server binary is darwin/arm64 (local macOS) or linux/amd64 (CI); pixels are
produced entirely by the containerised Chromium. Therefore **only the browser
needs to live in the pinned `linux/amd64` image** — the server can run anywhere
the browser can reach it.

This means the existing mise path is reused unchanged:
`mise run build:server:dev` (`mise.toml:71-73` → `invoke build.server-dev` →
`cargo build`) builds the binary on the host exactly as it does today and in CI;
`start-server.mjs` runs it against committed fixtures as today. Only the
Playwright/Chromium runner is wrapped in `docker run`. The server build/run path
stays identical across local and CI — local gets a darwin binary, CI a linux
binary, and both are fine.

This **supersedes** the three options previously listed under finding §3 and
open question #1 (mount a prebuilt linux binary / install Rust in-container /
mock `/api/*`). All three are avoided: no Rust toolchain in the image, no
cross-compilation, no API mocking.

### Why the alternative reading is worse

Running mise *inside* the container (so it `cargo build`s the server there)
works in CI (linux host → linux binary) but requires Rust + cargo + mise baked
into the Playwright image and a full server build under QEMU emulation on Apple
Silicon — heavy image, slow local runs. Not recommended.

### What the host-server split requires

The current wiring has Playwright *spawn* the server via
`webServer.command: "node e2e/start-server.mjs"` (`playwright.config.ts:38-46`).
If Playwright runs in the container, its `webServer` would try to spawn the
server *inside* the container — back to the Rust problem. So three changes:

1. **Move `webServer` ownership to the host.** Start the server on the host
   *before* `docker run` (a mise task: `build:server:dev` → launch
   `start-server.mjs` on host → `docker run` the Playwright image with no
   `webServer` and `reuseExistingServer`/external `BASE_URL`). A dedicated
   `playwright.docker.config.ts` with no `webServer` block is the natural shape,
   mirroring the existing `playwright.glyph.config.ts` carve-out.
2. **Container→host networking + configurable `BASE_URL` host.** Both
   `playwright.config.ts:18` and `global-setup.ts:61` hardcode `127.0.0.1`,
   and the server binds `host: "127.0.0.1"` (`start-server.mjs:100`). Two
   changes make this work uniformly across Colima, Docker Desktop, and Linux CI:
   - **Bind the server to `0.0.0.0`** (not `127.0.0.1`). Driven by **CI
     parity**, not Colima: on the Linux CI runner the container reaches the
     host over the bridge gateway (`172.17.0.x`), where a loopback-only server
     is unreachable. Colima would tolerate `127.0.0.1` (its gvisor user-net
     NATs `192.168.5.2` onto the macOS host loopback), but `0.0.0.0` covers
     both. Set the server's bind host and the health host accordingly.
   - **Address the host as `host.docker.internal` with
     `docker run --add-host=host.docker.internal:host-gateway`**, and
     `BASE_URL=http://host.docker.internal:${port}`. This single combination
     is correct on Colima (≥ 0.6.x, where `host.docker.internal` resolves by
     default — Colima aliases it to Lima's `host.lima.internal` and sets
     `--host-gateway-ip` to the host loopback via PR #574), on Docker Desktop,
     and on Linux CI (where `host-gateway` resolves to the bridge gateway that
     reaches the runner). The server-chosen port from `.e2e-port` is unchanged;
     only the host part varies.
   - **Do NOT use `docker run --network=host` as the local mechanism.** Under
     Colima it shares the *Lima VM's* network namespace, so `127.0.0.1` inside
     the container hits the VM, not the macOS host running the server.
     (`--network=host` is fine in Linux CI but would diverge the code path.)
3. **Mount the repo** (`-v "$(pwd):/work"`, already in the README recipe) so the
   container sees `.e2e-port` (read by `global-setup.ts:54-60`) and the fixture
   `.md` files.

### What gets lighter

The fixture snapshot/restore machinery (`global-setup.ts:28-52`,
`global-teardown`) exists for the **kanban drag tests in the `chromium`
project** — the visual-regression specs are read-only. The Docker job runs only
`--project visual-regression` (as `update-visual-baselines.yml:36` already
does), so it never mutates fixtures and the cross-boundary file-write concern is
moot for the visual job.

### Residual caveat

Pin the server's locale/timezone too (already in scope), since the server emits
dates — though the visual specs already mask relative timestamps via
`relativeTimeMask` (`helpers.ts:7-8`), limiting exposure.

### Fallback if host↔container networking proves flaky (Colima)

Colima has had occasional point-release DNS regressions for
`host.docker.internal` (e.g. abiosoft/colima #902, #1159). If that bites, the
repo already has `build:server:cross-compile` (`mise.toml:80-83` →
`tasks/build.py:153-167`, `cargo zigbuild` for all release targets incl.
linux/amd64). That enables a fully-containerised **server-sidecar** option:
server and Playwright in two containers on a shared Docker network, reaching
each other by service name — no `host.docker.internal`, no host binding. More
setup, and the cross-compile uses the `embed-dist` feature rather than the e2e
`dev-frontend` (filesystem-served) feature, so keep it as the fallback, not the
default.

### Follow-up code references

- `mise.toml:71-73` — `build:server:dev` → `invoke build.server-dev`
- `tasks/build.py` — `server-dev` task (`cargo build`)
- `skills/visualisation/visualise/frontend/e2e/start-server.mjs:42-50` — binary resolution (`ACCELERATOR_VISUALISER_BIN` or `cargo build` fallback)
- `skills/visualisation/visualise/frontend/e2e/start-server.mjs:145` — writes `.e2e-port`
- `skills/visualisation/visualise/frontend/e2e/global-setup.ts:28-52` — fixture snapshot (chromium-project concern, not visual)
- `skills/visualisation/visualise/frontend/e2e/global-setup.ts:54-61` — reads `.e2e-port`, sets `BASE_URL` (hardcoded `127.0.0.1`)
- `skills/visualisation/visualise/frontend/playwright.config.ts:18,38-46` — `BASE_URL` default + `webServer` to relocate
