# Contributing

## Local development

To load the plugin from a local checkout:

```bash
claude --plugin-dir /path/to/accelerator
```

That loads the checkout's skills, but every `accelerator` command still
downloads and runs the released, verified binaries. To run your local
builds instead, build them, mark the checkout as a dev launcher, and name
each sub-binary you changed:

```bash
A=/path/to/accelerator
(cd "$A/cli" && cargo build --workspace --exclude accelerator-visualiser)
touch "$A/.accelerator-dev-launcher"
cd /path/to/project-under-test
ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER=1 \
ACCELERATOR_LAUNCHER_BIN="$A/cli/target/debug/accelerator" \
ACCELERATOR_RESEARCH_BIN="$A/cli/target/debug/accelerator-research" \
claude --plugin-dir "$A"
```

The marker and the first two variables swap in your launcher only; a dev
launcher still fetches each sub-binary for its own version. Each
`ACCELERATOR_<SUB>_BIN` (the subcommand upper-cased, `-` as `_`) points one
subcommand at its local build, so set one per sub-binary you changed, such
as `ACCELERATOR_CORPUS_BIN` or `ACCELERATOR_VCS_BIN`. Each dev-launcher exec
is logged to `.accelerator-unverified.log` in the cache directory (`bin/`
unless `ACCELERATOR_CACHE_DIR` is set); a sub-binary override is not logged
there, and shows only as an `info` trace under `ACCELERATOR_LOG=info`.

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
