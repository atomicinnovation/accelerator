This file provides guidance to Claude Code (claude.ai/code) when working with 
code in this repository.

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
  `frontend`, `server`, `build-system` (the Python `tasks/` toolchain — *not*
  the `build:*` artifact namespace), `scripts` (shell). There is **no
  `<component>:fix`** roll-up — fix a component via its `format:<c>:fix` +
  `lint:<c>:fix` tasks.

Enforcement is **CI-only — there are no pre-commit hooks.** Run `mise run fix &&
mise run check` yourself before pushing.

### Running a single test

The aggregate `mise run test:*` tasks have no name filter; drop to the
underlying runner for one test:

- **Python (tasks/):** `uv run pytest tests/unit/tasks/test_x.py::test_y -v`
- **Rust (server):** `cd skills/visualisation/visualise/server && cargo test <name>`
- **Frontend (Vitest):** `cd skills/visualisation/visualise/frontend && npx vitest run -t "<name>"`
- **Shell:** the suites are standalone scripts — run e.g.
  `bash scripts/test-config.sh` or `bash hooks/test-vcs-detect.sh` directly.

## Architecture

### Skills as the product (`skills/`, `agents/`, `templates/`, `hooks/`)

Skills are grouped by category (`planning/`, `research/`, `work/`, `review/`,
`decisions/`, `design/`, `vcs/`, `github/`, `integrations/`, `config/`, …) and
registered in `.claude-plugin/plugin.json`. Each skill is a `SKILL.md` with YAML
frontmatter (`name`, `description`, `argument-hint`, `allowed-tools`). The
non-obvious mechanism: a SKILL.md body runs shell via the **`!` preprocessor**
(``!`command` ``) at invocation time to inject live context (VCS status, config,
per-skill context) into the prompt — see `skills/vcs/commit/SKILL.md`. Scripts
are addressed via `${CLAUDE_PLUGIN_ROOT}` so they resolve from the installed
plugin location.

The core design (read the README "Philosophy" section): development is split
into phases (research → plan → implement) that communicate **through the
filesystem**, not the conversation. The `meta/` directory is persistent shared
memory; each skill reads/writes predictable paths within it. Subagents
(`agents/*.md`) do exploratory work in isolated context and return only
summaries. Locator agents (find, no Read) are deliberately separated from
analyser agents (Read) to keep each context bounded.

### Visualiser (`skills/visualisation/visualise/`)

- `server/` — Rust (axum) HTTP server. Cargo features: `embed-dist` (default,
  bundles the built SPA via `rust-embed`) and `dev-frontend` (serves from disk).
  `build:server:dev` builds the dev binary; release builds embed the frontend.
- `frontend/` — React 19 + TypeScript + Vite SPA. **Biome** (not ESLint/Prettier)
  for lint + format; `tsc -b` for types; Vitest for unit, Playwright for E2E.
- The binary is distributed via GitHub Releases and downloaded on first use,
  verified against `bin/checksums.json` (SHA-256, optional SLSA provenance).

### Build system (`tasks/`)

Python invoke tasks, type-checked with **pyrefly (strict preset)** and linted
with **ruff (`select = ["ALL"]`)** — both version-pinned exactly in
`pyproject.toml` because their rule sets are version-sensitive. Shared helpers
live in `tasks/shared/`. Release/version logic enforces **version coherence**:
`plugin.json`, the server's `Cargo.toml`, and `checksums.json` must agree.

### Shell scripts (`scripts/`, `hooks/`)

A large bash library backs the skills (config reading, VCS detection, frontmatter
parsing, migrations). Checked with shfmt + ShellCheck, plus a custom
**bashisms** linter (`scripts/lint-bashisms.sh`) that guards a **bash 3.2 floor**
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
- Tests deliberately have **no `__init__.py`** (pytest importlib mode) and are
  held to relaxed ruff/pyrefly standards.
- Tooling versions (uv, python, rust, node, jj, shellcheck, shfmt, jq) are
  pinned in `mise.toml`; `mise` provisions them. Minimum supported Claude Code
  for the plugin itself is **v2.1.144** (subagent skill-preload mechanism).
