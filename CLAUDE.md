## What this repo is

Accelerator is a **Claude Code plugin** — not a conventional application. The
shipped product is the set of **skills** (Markdown `SKILL.md` files), **agents**,
**hooks**, **templates**, and **scripts** that Claude Code loads. Alongside them
lives a **visualiser** (a Rust HTTP server + React frontend) distributed as a
pre-compiled binary. Four language toolchains coexist in one repo, each with its
own checks; see Architecture below.

## Build, test, and check

All dev tasks run through **`mise run <task>`** (declared in `mise.toml`,
implemented as [invoke](https://www.pyinvoke.org/) tasks under `tasks/`). Run
`mise tasks` for the full leaf list; `tasks/README.md` documents the *shape* of
the task tree (learn it once).

**"Done" means `mise run` (the bare default task) exits 0 end-to-end.** That is
the full local CI mirror: it builds the frontend + dev server, applies all
formatters and safe lint fixes, runs every lint and type-check, and runs the
entire test suite. It is heavy (reformats in place, compiles Rust several
times). A change is not finished until this is green.

Two faster entry points exist and should be your inner loop:

- `mise run check` — the **exact** read-only set CI runs (format + lint + types
  across all four components). Must exit 0 before pushing.
- `mise run fix` — apply every formatter + safe lint fix (mechanical only; **no
  type-checks**, and shell has no autofixer).
- `mise run <component>:check` — fast single-component loop. Components:
  `frontend`, `server`, `cli` (the `cli/` Rust workspace — workspace-wide
  rustfmt + clippy), `build-system` (the Python `tasks/` toolchain — *not*
  the `build:*` artifact namespace), `scripts` (shell). There is **no
  `<component>:fix`** roll-up — fix a component via its `format:<c>:fix` +
  `lint:<c>:fix` tasks. Rust enforcement beyond `cli:check` (cargo-deny,
  cargo-pup) is documented in `tasks/README.md`.

Enforcement is **CI-only — there are no pre-commit hooks.** Run `mise run fix &&
mise run check` yourself before pushing.

### Running a single test

The aggregate `mise run test:*` tasks have no name filter; drop to the
underlying runner for one test.

## Architecture

### Skills as the product (`skills/`, `agents/`, `templates/`, `hooks/`)

The non-obvious mechanism: a SKILL.md body runs shell via the **`!`
preprocessor** (``!`command` ``) at invocation time to inject live context (VCS
status, config, per-skill context) into the prompt — see
`skills/vcs/commit/SKILL.md`. Scripts are addressed via `${CLAUDE_PLUGIN_ROOT}`
so they resolve from the installed plugin location.

The core design (read the README "Philosophy" section): development is split
into phases (research → plan → implement) that communicate **through the
filesystem**, not the conversation. The `meta/` directory is persistent shared
memory; each skill reads/writes predictable paths within it. Subagents
(`agents/*.md`) do exploratory work in isolated context and return only
summaries. Locator agents (find, no Read) are deliberately separated from
analyser agents (Read) to keep each context bounded.

Gotchas specific to the visualiser (`cli/visualiser/`) and the build system
(`tasks/`) live in their own `CLAUDE.md` files, loaded when you work in those
directories.

### Shell scripts (`scripts/`, `hooks/`)

A large bash library backs the skills (config reading, VCS detection, frontmatter
parsing, migrations). A custom **bashisms** linter
(`scripts/lint-bashisms.sh`) guards a **bash 3.2 floor**
— macOS ships bash 3.2, so bash-4 constructs (associative arrays, `${var,,}`,
etc.) are banned. Suspect the 3.2 floor first for any macOS-only shell failure.
`hooks/` contains `SessionStart`/`PreToolUse` hooks (config detection, VCS
detection + git-guard, migration reminders).

## Conventions and gotchas

- **Line width is 80 everywhere**, set in `.editorconfig` and **duplicated by
  hand** into `pyproject.toml` (ruff) and `server/rustfmt.toml` (rustfmt) because
  those tools don't read `.editorconfig`. Biome and shfmt read it natively. Keep
  the copies in sync — there is no automated check.
- **Shell has no autofixer** — `scripts` is absent from `lint:fix`; ShellCheck
  findings are fixed by hand or with a justified `# shellcheck disable=`.
- **Executable-bit invariant** — a tracked `.sh` is executable (`0755`) iff it
  is *not* a sourced-only library; the `lint:scripts:exec-bits:check` guard
  enforces it. New `.sh` files are entrypoints by default (`chmod +x` + commit);
  only sourced-only libraries go in `SHELL_LIBRARIES`. See the "Executable-bit
  invariant" subsection in `tasks/README.md`.
- Tests deliberately have **no `__init__.py`** (pytest importlib mode) and are
  held to relaxed ruff/pyrefly standards.
- Tooling versions (uv, python, rust, node, jj, shellcheck, shfmt, jq) are
  pinned in `mise.toml`; `mise` provisions them. Minimum supported Claude Code
  for the plugin itself is **v2.1.144** (subagent skill-preload mechanism).
