## What this repo is

Accelerator is a **Claude Code plugin** — not a conventional application. The
shipped product is the set of **skills** (Markdown `SKILL.md` files),
**agents**, **hooks**, **templates**, and **scripts** that Claude Code loads.
Alongside them lives a **visualiser** (a Rust HTTP server + React frontend)
distributed as a pre-compiled binary. Four language toolchains coexist in one
repo, each with its own checks; see Architecture below.

## Build, test, and check

All dev tasks run through **`mise run <task>`** (declared in `mise.toml`,
implemented as [invoke](https://www.pyinvoke.org/) tasks under `tasks/`). Run
`mise tasks` for the full leaf list; `tasks/README.md` documents the *shape* of
the task tree (learn it once) and carries the thirteen-point checklist for
registering a dispatched sub-binary, plus a shorter one for registering a
plain library crate.

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
  cargo-pup, cargo-public-api) is documented in `tasks/README.md`. `docs:check`
  is deliberately **absent from the aggregate `check`** — it writes gitignored
  artefacts and needs network + a Chromium install, so it would blunt the fast
  read-only loop — but the bare `default` task **does** run it, so a full local
  run covers the docs CI lane. Run `docs:check` directly when touching
  `docs-site/` and you don't want the full default run.

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

### Shell scripts (`bin/`, `hooks/`)

The shell surface is two thin wrappers — the launcher bootstrap
`bin/accelerator` and the hook wrapper `hooks/launcher-link-refresh.sh`; the
config reading, VCS detection, frontmatter parsing and migration logic they
once backed now live in the `cli/` Rust workspace. Both stay under a **bash 3.2
floor** — macOS ships bash 3.2, so bash-4 constructs (associative arrays,
`${var,,}`, etc.) are banned. A Python **bashisms** task
(`tasks/lint/scripts.py`) plus shfmt and ShellCheck guard exactly these two;
suspect the 3.2 floor first for any macOS-only shell failure. `hooks/hooks.json`
registers the `SessionStart`/`PreToolUse` hooks (VCS detection + git-guard,
config summary, migration reminders), each dispatched through `bin/accelerator`
into the `cli/` sub-binaries.

## How we write code

These are non-negotiable. They override convenience.

- **Test-driven development, in its purest form.** Follow the red-green-refactor
  loop: write a failing test first (red), write the minimum code to pass it
  (green), then refactor with the safety net in place. Never write production
  code without a failing test demanding it. Test behaviour, not implementation.
- **Strict domain-driven design.** Model the domain explicitly. Prefer code that
  is clear, readable, and expresses intent through rich domain language — names
  and abstractions that mirror how the domain is spoken about, not technical
  incidental detail.
- **Comments are a last resort.** A comment is a signal that the code failed to
  express its own intent. Before writing one, do the work to make the code
  itself clear — rename, extract, restructure. Only keep a comment when it
  captures something genuinely non-obvious to a skilled developer that no amount
  of refactoring could convey (e.g. *why* an unusual choice was made, an
  external constraint, a subtle invariant). Never include comments that
  describe what code could otherwise express — we have a *very* low tolerance
  for comments. Actively remove them from plans you create. References to
  ADRs, work items, acceptance criteria, plan phases etc. in comments can go 
  stale fast, so don't include them.

## Conventions and gotchas

- **Line width is 80 everywhere**, set in `.editorconfig` and **duplicated by
  hand** into `pyproject.toml` (ruff) and `server/rustfmt.toml` (rustfmt)
  because those tools don't read `.editorconfig`. Biome and shfmt read it
  natively. Keep the copies in sync — there is no automated check.
- **Shell has no autofixer** — `scripts` is absent from `lint:fix`; ShellCheck
  findings are fixed by hand or with a justified `# shellcheck disable=`.
- **Surviving thin shell** — the shell surface is exactly the two files in
  `SURVIVING_SHELL_SOURCES` (`tasks/shared/sources.py`): `bin/accelerator` and
  `hooks/launcher-link-refresh.sh`, both tracked-executable (`0755`) and
  bash-3.2-safe. See the "Surviving thin shell" subsection in `tasks/README.md`.
- Tests deliberately have **no `__init__.py`** (pytest importlib mode) and are
  held to relaxed ruff/pyrefly standards.
- Tooling versions (uv, python, rust, node, jj, shellcheck, shfmt, jq) are
  pinned in `mise.toml`; `mise` provisions them. Minimum supported Claude Code
  for the plugin itself is **v2.1.144** (subagent skill-preload mechanism).
