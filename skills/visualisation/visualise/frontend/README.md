# Visualiser Frontend

The visualiser frontend is the React app served by the [`visualisation/visualise`](../) accelerator skill. It renders a meta-directory inspector (research, plans, decisions, work items, etc.) backed by a Rust server. This README is enough to develop, test, and build the frontend without consulting other docs.

## Prerequisites

- Node.js (version per the `engines` field in `package.json`)
- npm (matches `package-lock.json`)
- For Linux Playwright baselines from macOS: Docker (optional; CI can produce them otherwise)

## Project Layout

- `src/components/` — Reusable React components (`Brand`, `Glyph`, `PipelineDots`, etc.). Co-located `<Name>/<Name>.tsx + .module.css + .test.tsx`; named exports; no barrels.
- `src/routes/` — Top-level routes wired through `src/router.ts`. TanStack Router imperative `createRoute` style.
- `src/styles/` — Design-system tokens (`tokens.ts` ↔ `global.css`), parity tests, and migration invariants.
- `src/api/` — Domain types (`DocTypeKey`, `DOC_TYPE_LABELS`, ...) and hooks for backend communication.
- `tests/visual-regression/` — Playwright specs and committed PNG baselines (darwin + linux per spec).

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
npm run test:e2e        # Playwright E2E + visual regression
```

`npm test` does **not** run `@ts-expect-error` directives — those only fail under `tsc`. Run both before pushing.

## Building

```sh
npm run build
```

## Developer Routes

The following routes exist solely to preview internal components and are not part of the user-facing navigation:

- `/glyph-showcase` — renders all 12 doc-type Glyphs at all 3 supported sizes (16/24/32 px), viewable under both `data-theme="light"` and `data-theme="dark"` via dev-tools attribute toggling. See [`meta/work/0037-glyph-component.md`](../../../../meta/work/0037-glyph-component.md).

## Visual-Regression Baselines

Each visual-regression spec commits two PNG baselines per case: `<name>-<project>-darwin.png` and `<name>-<project>-linux.png`. Capture both before merging.

- **macOS**: `npx playwright test --update-snapshots <spec>` produces darwin baselines.
- **Linux (from macOS)**: Use the Playwright Docker image matching `package.json`:

  ```sh
  docker run --rm -v "$(pwd):/work" -w /work --ipc=host \
    mcr.microsoft.com/playwright:v<version>-jammy \
    npx playwright test --update-snapshots <spec>
  ```

  Replace `<version>` with the pinned `@playwright/test` version.

If a Playwright test fails locally with a baseline mismatch on the opposite platform, do not regenerate that platform's baseline locally — let CI regenerate it under a known environment.

### Glyph showcase specs

`tests/visual-regression/glyph-showcase.spec.ts` and `glyph-resolved-fill.spec.ts` are frontend-only — they don't need the Rust visualiser server. A standalone config at `playwright.glyph.config.ts` serves the built `dist/` via `vite preview`, so they can be regenerated inside a vanilla Playwright Linux image without any Rust toolchain:

```sh
docker run --rm -v "$(pwd):/work" -w /work --ipc=host -e CI=1 \
  mcr.microsoft.com/playwright:v1.59.1-noble \
  bash -c "npm ci && npm run build && npx playwright test --config playwright.glyph.config.ts --update-snapshots"
```

Restore your host `node_modules` with `npm ci` afterwards — the container build leaves Linux-native binaries (esbuild, @rollup/rollup-*) in place that the host platform can't load.

## Troubleshooting

- **`tsc -b` fails after editing a token**: ensure the new entry appears in BOTH `tokens.ts` and all three theme blocks in `global.css`; run `npx vitest run src/styles/global.test.ts` to surface parity violations.
- **Theme swap not reflected in `getComputedStyle`**: the cascade may not have settled. Prefer `page.waitForFunction` checking a known computed value over a fixed `requestAnimationFrame` wait.
