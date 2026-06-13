# Visualiser Frontend

The visualiser frontend is the React app served by the [`visualisation/visualise`](../) accelerator skill. It renders a meta-directory inspector (research, plans, decisions, work items, etc.) backed by a Rust server. This README is enough to develop, test, and build the frontend without consulting other docs.

## Prerequisites

- Node.js (version per the `engines` field in `package.json`)
- npm (matches `package-lock.json`)
- **Docker** — required for all visual-regression work (running and
  re-baselining), for internal and public/Linux contributors alike. The
  baselines are rendered inside a pinned Playwright Linux image; macOS needs a
  developer-provided daemon (e.g. Docker Desktop / Colima). See
  [Visual-Regression Baselines](#visual-regression-baselines).

## Project Layout

- `src/components/` — Reusable React components (`Brand`, `Glyph`, `PipelineDots`, etc.). Co-located `<Name>/<Name>.tsx + .module.css + .test.tsx`; named exports; no barrels.
- `src/routes/` — Top-level routes wired through `src/router.ts`. TanStack Router imperative `createRoute` style.
- `src/styles/` — Design-system tokens (`tokens.ts` ↔ `global.css`), parity tests, and migration invariants.
- `src/api/` — Domain types (`DocTypeKey`, `DOC_TYPE_LABELS`, ...) and hooks for backend communication.
- `tests/visual-regression/` — screenshot-emitting Playwright specs and the single canonical PNG baseline set (one render per case, no `-darwin`/`-linux` split). These run **only** inside the pinned Docker image (see [Visual-Regression Baselines](#visual-regression-baselines)).
- `tests/resolved-styles/` — computed-style Playwright specs (`*-resolved-*`). They take no screenshots and run **natively** on every test run.
- `tests/lib/` — shared Playwright helpers imported by both the above trees.

## Development

```sh
npm install
npm run dev
```

The dev server resolves the backend API port from `VISUALISER_API_PORT` or `VISUALISER_INFO_PATH` (see `vite.config.ts`). Without one, API calls fail loudly with ECONNREFUSED — set the env var before `npm run dev`.

## Testing

```sh
npm test                # Vitest unit tests
npm run typecheck       # tsc --noEmit — enforces @ts-expect-error directives
npm run test:e2e        # Playwright E2E + native computed-style (resolved-styles) specs
```

`npm test` does **not** run `@ts-expect-error` directives — those only fail under `tsc`. Run both before pushing.

> **Native runs do not validate the visual baselines.** `npm run test:e2e` (and `mise run test` / `test:e2e`) run the kanban E2E specs and the native `resolved-styles` computed-style specs, but **no screenshot specs** — those run only in Docker. A green native run says nothing about the committed PNG baselines; after any frontend change you must run the Docker compare/update task (see [Visual-Regression Baselines](#visual-regression-baselines)) before committing.

## Building

```sh
npm run build
```

## Developer Routes

The following routes exist solely to preview internal components and are not part of the user-facing navigation:

- `/glyph-showcase` — renders all 12 doc-type Glyphs at all 3 supported sizes (16/24/32 px), viewable under both `data-theme="light"` and `data-theme="dark"` via dev-tools attribute toggling. See [`meta/work/0037-glyph-component.md`](../../../../meta/work/0037-glyph-component.md).

## Visual-Regression Baselines

Two tasks, run from the repo root, are the whole interface:

- **`mise run test:e2e:visualiser:docker:update`** — regenerate the committed
  baseline set. Run this after any frontend change that alters rendered output,
  then commit the regenerated PNGs alongside your change.
- **`mise run test:e2e:visualiser:docker`** — compare against the committed set
  (no writes). Run this before committing to confirm a clean, zero-diff pass —
  the same check CI runs on your pushed commit.

Both require a running Docker daemon (see [Prerequisites](#prerequisites)).

### Why it works this way

The committed baseline is a function of **what Chromium renders**, not of the
server's host architecture. So **only the browser is containerised**: the
visual specs run inside the pinned `mcr.microsoft.com/playwright` Linux image
(amd64), while the Rust dev-frontend server runs on the **host** (exactly as CI
does) and the container reaches it over `host.docker.internal`. This avoids
putting a Rust toolchain in the image or cross-compiling the server.

`--network=host` is deliberately **not** used locally: under Colima it joins the
Lima VM's network namespace, not the macOS host's, so the container could not
reach the host server. Do not "simplify" the flow into a fully-containerised
path — that revives the Rust-in-image problem this design exists to avoid.

A single canonical Linux set is committed (no `-darwin`/`-linux` split): every
developer and CI run renders in the same pinned environment, so one baseline
suffices. On Apple Silicon the amd64 image runs under emulation; the modest
per-assertion tolerances (`maxDiffPixelRatio` 0.05 / 0.01) absorb residual
in-environment flake only — they are **not** a cross-platform bridge, so do not
raise them.

### The native/Docker split (the one footgun)

Native runs (`mise run test`, `mise run test:e2e`, `npm run test:e2e`) execute
the kanban E2E specs and the native `resolved-styles` computed-style specs, but
**no screenshot specs**. A green native suite therefore says **nothing** about
the visual baselines. After any frontend change you must run the Docker
compare/update task before committing — otherwise CI's `test-visual-regression`
job is the first thing to notice the drift.

### Re-baselining workflow

1. Change the frontend.
2. `mise run test:e2e:visualiser:docker:update` — regenerates the canonical set
   under `tests/visual-regression/__screenshots__/`.
3. Commit the regenerated PNGs with your change and push.
4. CI validates the **same commit** (no push-back, no follow-up run).

### Debugging a failing visual test

On a mismatch Playwright writes the actual/expected/diff PNGs and (because
`trace: "on-first-retry"`) a trace under `test-results/` in this frontend
directory — these write through to the mounted host tree, so they are available
after the container exits. Inspect them with:

```sh
npx playwright show-report          # HTML report incl. the diff images
npx playwright show-trace test-results/<...>/trace.zip
```

### Speeding up the inner loop

`npm ci` inside the container under amd64 emulation is the slow part on Apple
Silicon. The power-user escape hatch persists `node_modules` in a version-keyed
named Docker volume across runs:

```sh
invoke test.e2e.visualiser-docker --cache-deps            # compare
invoke test.e2e.visualiser-docker --cache-deps --update   # rebaseline
```

The volume is keyed to the resolved `@playwright/test` version, so a cached
`node_modules` can never outlive a Playwright bump. The default (no
`--cache-deps`) uses an anonymous mask, which is the correctness-safe choice.
Either way the mask keeps the container's Linux-native binaries out of your
host `node_modules`, so no `npm ci` is needed on the host afterwards.

### Operational notes

- **Transient `0.0.0.0` exposure**: during a Docker run the host dev-frontend
  server binds `0.0.0.0` (so the container can reach it over the bridge
  gateway) and the server's loopback-only guards are relaxed via
  `ACCELERATOR_VISUALISER_E2E_INSECURE`, set only by this task. It serves only
  the committed, non-sensitive Markdown fixtures, the binding lasts only for the
  run, and the bypass exists only in the dev-frontend (test) binary — release
  builds and normal `mise run dev` stay strictly loopback-only.
- **Bumping `@playwright/test`**: the image tag derives from the resolved
  lockfile version. Before merging a version bump, confirm the matching
  `mcr.microsoft.com/playwright:v<version>-noble` tag exists on MCR — a missing
  tag breaks everyone with an opaque `manifest unknown`.
- **If `host.docker.internal` breaks on a Colima point-release** (a known
  occasional DNS regression): pin a known-good Colima version as the interim
  workaround before reaching for the documented server-sidecar fallback.

### Glyph showcase specs

The glyph specs are no longer a carve-out — they regenerate through the unified
flow like every other visual spec, driving the `/glyph-showcase` route served by
the Rust dev-frontend server (not `vite preview`). The two former
`playwright.glyph.config.ts` specs now live in different trees by type:
`glyph-showcase.spec.ts` (screenshots) in `tests/visual-regression/` (Docker),
and `glyph-resolved-fill.spec.ts` (computed-style) in `tests/resolved-styles/`
(native).

## Troubleshooting

- **`tsc -b` fails after editing a token**: ensure the new entry appears in BOTH `tokens.ts` and all three theme blocks in `global.css`; run `npx vitest run src/styles/global.test.ts` to surface parity violations.
- **Theme swap not reflected in `getComputedStyle`**: the cascade may not have settled. Prefer `page.waitForFunction` checking a known computed value over a fixed `requestAnimationFrame` wait.
