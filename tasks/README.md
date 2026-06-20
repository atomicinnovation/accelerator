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
| Python tooling | `build-system:check` | format + lint + types (ruff, pyrefly)        |
| Shell          | `scripts:check`      | format + lint (shfmt, ShellCheck + bashisms) |

`build-system` is the repo-root Python automation toolchain (this `tasks/`
package + its tests) — unrelated to the `build:*` artifact namespace. Its task
descriptions name Python/ruff/pyrefly so `mise tasks | grep -i python` finds it.

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
