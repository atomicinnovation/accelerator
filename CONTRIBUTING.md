# Contributing

## Linting, formatting, and type-checking

Enforcement is **CI-only** — there are no pre-commit hooks. So before you push:

```sh
mise run fix     # apply every formatter + safe lint fix (mechanical only)
mise run check   # the exact set of checks CI runs — must exit 0
```

`mise run check` runs the four per-component aggregates, each folding that
component's format + lint (+ type-check where applicable):

| Component      | Aggregate             | Tools                          |
| -------------- | --------------------- | ------------------------------ |
| `frontend`     | `frontend:check`      | Biome (format + lint), tsc     |
| `server`       | `server:check`        | rustfmt, clippy                |
| `build-system` | `build-system:check`  | ruff (format + lint), pyrefly  |
| `scripts`      | `scripts:check`       | shfmt, ShellCheck, bashisms    |

(`build-system` is the repo-root Python automation toolchain — the `tasks/`
invoke package and its tests — not the `build:*` artifact namespace.)

### Fixing one component

There is **no `<component>:fix` roll-up**. To clean up a single component, run
its family fix tasks directly:

```sh
mise run format:frontend:fix && mise run lint:frontend:fix
```

…and likewise `format:server:fix`, `format:build-system:fix`,
`format:scripts:fix` and their `lint:…:fix` siblings. Shell has **no
autofixer** — `shfmt` reformats, but ShellCheck findings are fixed by hand or
with a justified `# shellcheck disable=`/`source=` directive, so `scripts` is
absent from the top-level `lint:fix`; run `mise run scripts:check` to see what
remains.

Type-checks (`types:check`, `tsc -b --noEmit` + `pyrefly`) are **not** auto-fixable
and are not part of `fix` — `mise run check` (or `mise run default`) runs them.

Run `mise tasks` for the full per-component leaf list.
