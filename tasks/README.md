# Task tree

The repo's dev tasks are declared in `mise.toml` (run them with
`mise run <task>`) and implemented as [invoke](https://www.pyinvoke.org/) tasks
in this package. `mise tasks` lists every leaf with its description; this file
documents the *shape* of the tree so it only has to be learned once.

## Per-component checks

Each component has a `<component>:check` roll-up that folds that component's
format + lint (+ type-check where applicable):

| Component      | Roll-up              | Folds                                        |
| -------------- | -------------------- | -------------------------------------------- |
| Frontend       | `frontend:check`     | format + lint + types (Biome, tsc)           |
| Rust server    | `server:check`       | format + lint (rustfmt, clippy)              |
| Rust cli       | `cli:check`          | format + lint (rustfmt, workspace-wide clippy) |
| Python tooling | `build-system:check` | format + lint + types (ruff, pyrefly)        |
| Shell          | `scripts:check`      | format + lint (shfmt, ShellCheck + bashisms) |

`build-system` is the repo-root Python automation toolchain (this `tasks/`
package + its tests) — unrelated to the `build:*` artifact namespace. Its task
descriptions name Python/ruff/pyrefly so `mise tasks | grep -i python` finds it.

`cli:check` runs **one workspace-wide** `cargo clippy --workspace` pass that
covers every member declared in `cli/Cargo.toml` `[workspace].members`, so new
members join enforcement with no per-member wiring. `format:cli:*` and
`lint:cli:*` each depend on `deps:install:rust-components` — mise's `[tools]`
rust `components` field is silently skipped for an already-present toolchain, so
rustfmt/clippy are provisioned explicitly. `lint:cli:fix` applies only clippy's
machine-rewritable subset (lints such as `unwrap_used` cannot be auto-fixed), so
`cli:check` must still be run for the remainder. Beyond `cli:check`, Rust
enforcement also spans standalone entity tasks wired directly into the top-level
`check` (they sit outside the `cli:` roll-up, mirroring `version:*` /
`github:*`): `deny:check` (cargo-deny supply-chain) and `pup:check` (cargo-pup
architecture, on an isolated nightly lane).

## Family aggregates

`format:check`, `lint:check`, and `types:check` run the corresponding family
across every component; `check` runs all of them (this is what CI runs). `fix`
applies `format:fix` + `lint:fix` (mechanical changes only).

## Conventions (learn once)

- A component name **leads** its roll-up (`server:check`, entity-first, like
  `version:*` / `github:*`) but **trails** in the families
  (`format:server:check`).
- `scripts` and `server` have no `types:*` — only `frontend` and `build-system`
  type-check. `lint:scripts:check` nests one level deeper (shellcheck +
  bashisms) because shell has two linters.
- There are `<component>:check` roll-ups but **no** `<component>:fix`. Fix one
  component via `format:<component>:fix` + `lint:<component>:fix`, or run the
  top-level `fix`. Shell has no autofixer, so `scripts` is absent from
  `lint:fix` — run `mise run scripts:check` for remaining shell findings.

The `lint:<language>` task naming requested by work item 0098 is satisfied by
these `<component>:check` roll-ups, and its aggregate `lint` / `format`
acceptance criteria by the family aggregates above.

### Executable-bit invariant

A tracked `.sh` is executable (`0755`) **iff it is _not_ a sourced-only
library**. The `lint:scripts:exec-bits:check` guard
(`exec_bits` in `tasks/lint/scripts.py`) enforces this over every shell source
and fails — naming each offending file with the exact `chmod` to run — when an
off-list entrypoint lacks `+x`, a library carries `+x`, or a library-list path
is no longer enumerated.

- **Default: new `.sh` files are entrypoints.** `chmod +x` and commit them. You
  only touch `SHELL_LIBRARIES` (the manifest in `tasks/lint/scripts.py`) for a
  sourced-only library.
- **The classification rule is two-part: sourced AND never invoked by path.**
  "Sourced" alone is not enough. `jira-fields.sh` is `source`d by
  `jira-init-flow.sh` *and* invoked `bash …/jira-fields.sh refresh` in
  production, so it is an **entrypoint** that stays OFF the list at `0755`.
  Dual-use ⇒ entrypoint.
- **Maintenance:** a new sourced-only library must be **added** to
  `SHELL_LIBRARIES` (or the guard demands `+x`); a removed/renamed library must
  be **deleted/updated** there (or the stale-entry check fails).
- **Runner vs helper:** `test-interactive-protocol.sh` is a test *runner* →
  entrypoint → `0755`; `test-helpers.sh` is a sourced *helper* → on the list →
  `0644`.
- **Fixtures are a third category.** Scripts under `test-fixtures/**` are
  bash-run migration fixtures (executed via `bash "$f"`, never sourced, never
  path-invoked): the guard exempts them in both directions — they need neither
  `+x` nor a list entry.
- **Working-copy mode.** The guard reads the *working-copy* mode (matching
  `tasks/test/helpers.py`), so the `chmod` must be **committed** to satisfy CI
  on a fresh checkout. It intentionally enforces working-copy (not VCS-recorded)
  mode and assumes an exec-bit-preserving filesystem — acceptable given the
  macOS + Linux target matrix (CI runs `check-scripts` on `ubuntu-latest`; local
  dev is macOS via jj workspaces).

### Rust nightly lane (cargo-pup)

Architecture enforcement (the ADR-0053 inward-dependency rule) runs via
**cargo-pup**, a compiler plugin that needs a **second, pinned nightly**
toolchain (`PUP_NIGHTLY` / `PUP_VERSION` in `tasks/shared/rust.py`, a matched
pair). Everything else — the product build and every other check — stays on the
mise-pinned stable `1.90.0`.

- **Isolated by construction.** The nightly is provisioned only by
  `deps:install:pup` (rustup-managed, deliberately *not* a mise `[tool]`: mise
  cannot pin two rust toolchains, and a `cargo:` backend would build cargo-pup
  against stable and fail to load). Only `pup:check` and `test:integration:pup`
  consume it, and only the `check-architecture` CI job runs them. A nightly
  break (or a GC'd pinned nightly) therefore reddens `check-architecture` alone;
  every stable-lane check and the product build stay green. `pup:check` is
  wired into the top-level `check`, so a local `mise run check` still exercises
  it. The isolation is guarded by `tests/unit/tasks/test_workflows.py`.
- **First run is slow.** `deps:install:pup` builds cargo-pup from source
  (multi-minute) the first time; a presence probe skips the rebuild in steady
  state, so subsequent `pup:check` runs are fast.
- **Bumping the pin.** `PUP_NIGHTLY` and `PUP_VERSION` are a matched pair —
  cargo-pup's `rustc_private` driver only loads under the nightly it was built
  against — so bump them **together**. Dated nightlies are GC'd from
  `static.rust-lang.org` after a window; when the pinned one disappears,
  `deps:install:pup` fails with an actionable message naming the pin. Before
  committing a bump, verify the upstream release's published
  checksum/attestation (mirroring the SHA-256/SLSA discipline the visualiser
  binary gets via `checksums.json`).
- **`mise.lock` refresh.** The committed `mise.lock` hash-pins the aqua-backed
  tools. On **any** `[tools]` edit (or aqua pin bump), regenerate it — `mise
  lock --platform linux-x64,macos-arm64,macos-x64` (all matrix platforms) — and
  commit the result, so a lock authored on one arch does not force a fetch or
  dirty the tree on another. It does **not** cover the from-source cargo-pup
  build or the rustup nightly (an accepted unverified surface for the isolated
  lane).

### Contributor environment variables

Local-only toolchain escape hatches. **CI ignores both** (it runs the
fail-closed defaults), so the fix for a red job is the underlying finding, not
the env var.

| Variable                 | Default | Effect                                    |
| ------------------------ | ------- | ----------------------------------------- |
| `ACCELERATOR_PUP_MODE`   | `deny`  | `warn` downgrades a cargo-pup findings failure to advisory (log only). Unrecognised values fail closed to `deny`. |
| `ACCELERATOR_COVERAGE`   | `on`    | `off`/`false`/`0`/`no` drops `test:unit:cli` from instrumented `cargo llvm-cov nextest` to plain `cargo nextest run` (faster inner loop). |

## CI job → local command

Each CI check job mirrors a single `mise run` task, so a red job is reproducible
locally with the mapped command:

| CI job (`.github/workflows/main.yml`) | Local command                             |
| ------------------------------------- | ----------------------------------------- |
| `check-cli`                           | `mise run cli:check`                      |
| `check-supply-chain`                  | `mise run deny:check`                     |
| `check-architecture`                  | `mise run pup:check` (+ `test:integration:pup`) |
