---
id: "ADR-0050"
date: "2026-06-27T12:23:42+00:00"
author: Toby Clemson
status: accepted
tags: [architecture, tooling, task-runner, mise, invoke, foundations]
type: adr
title: "ADR-0050: `mise` + invoke Task Runner"
schema_version: 1
last_updated: "2026-06-27T12:23:42+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0045", "adr:ADR-0048", "adr:ADR-0049"]
---

# ADR-0050: `mise` + invoke Task Runner

**Date**: 2026-06-27
**Status**: Accepted
**Author**: Toby Clemson

## Context

Accelerator spans four product toolchains (Rust, Python, shell, TypeScript/React)
plus auxiliary tooling (ADR-0048). Every one needs provisioning at a pinned
version, and contributors need a single, memorable way to run dev tasks — format,
lint, type-check, test, release — across all of them. This repo drives its dev
workflow through **`mise`** for toolchain provisioning and
**[invoke](https://www.pyinvoke.org/)** for task implementation.

Two distinct concerns are in play:

- **Provisioning and pinning.** macOS, Linux, and CI must run identical tool
  versions or the checks diverge. The toolchains and supporting tools (uv, python,
  rust, node, jj, shellcheck, shfmt, jq) are all pinned exactly in `mise.toml`;
  `mise` installs and activates them so a local run mirrors CI.
- **Task definition and logic.** Dev tasks form a dependency graph (e.g. `check`
  → component checks; `default` → fix + lint + types + test) and some carry
  non-trivial logic (release version coherence across `plugin.json`,
  `Cargo.toml`, and `checksums.json`). That logic must be testable.

`mise` and invoke divide along that seam: `mise.toml` declares the task tree and
its `depends` graph, with most leaves delegating to `invoke ...`; invoke holds
the actual task logic in Python under `tasks/`. This ADR records that standing
arrangement.

## Decision Drivers

- One uniform entry point — `mise run <task>` — across four heterogeneous
  toolchains.
- Reproducible, version-pinned provisioning so local environments mirror CI.
- Task logic that is testable, type-checked, and linted — not untestable bash
  (consistent with ADR-0045 and the Python-as-support-language role in ADR-0048).
- A task dependency graph with parallelism, so aggregates compose cleanly.
- Low onboarding friction: provisioning is a single `mise` step.

## Considered Options

1. **`mise` (provision + pin + task interface) + invoke (task logic in Python)**
   — `mise.toml` declares the tree and delegates leaves to `invoke`; Python holds
   the logic.
2. **Make or Just as the task runner, with separate tool provisioning** (asdf,
   Homebrew, or documented manual installs) — task logic in shell recipes.
3. **A version manager (e.g. asdf) plus pure shell scripts** for task logic.
4. **`mise` tasks alone** — inline `run = "…"` shell snippets in `mise.toml`, no
   invoke layer.

## Decision

We will use **`mise` to provision and version-pin all toolchains and to expose
the unified `mise run <task>` interface**, and **invoke to implement the task
logic in Python** under `tasks/`. `mise.toml` stays a thin declaration layer —
the task tree and its `depends` graph — with leaves delegating to `invoke`.

We chose option 1 because it splits cleanly along the two concerns:

- **`mise` owns provisioning and the interface.** Exact version pins in
  `mise.toml` make local runs mirror CI, and a single `mise` install provisions
  every toolchain. `mise run <task>` is the one surface contributors learn,
  regardless of which language a task touches.
- **invoke owns the logic, in a testable language.** Task logic lives in Python —
  type-checked with pyrefly, linted with ruff, and unit-tested under
  `tests/unit/tasks/`. This is exactly the support-and-test role ADR-0048 assigns
  Python, and it keeps non-trivial logic (release coherence) out of the
  untestable bash ADR-0045 exists to avoid.

Option 4 was rejected because pushing real logic into inline `mise.toml` shell
snippets recreates the untestable-shell problem at the task layer. Options 2 and
3 were rejected because they either bolt provisioning on separately from the task
runner (losing the single-step, pinned, local-mirrors-CI guarantee) or put task
logic back in shell. Keeping `mise` and invoke split lets each do what it is best
at — provisioning and interface versus testable logic.

## Consequences

### Positive

- A single, uniform entry point (`mise run <task>`) across all toolchains.
- Reproducible environments: pinned versions mean local runs mirror CI.
- Task logic is testable, typed, and linted Python rather than bash.
- `depends` gives a real dependency graph with parallelism, so aggregates
  (`check`, `fix`, `default`) compose from component tasks.

### Negative

- Two layers to keep in sync: `mise.toml` task declarations mirror invoke task
  names, and drift between them is caught only at run time.
- Contributors must learn `mise`, invoke, and the task-tree shape (documented
  once in `tasks/README.md`).
- `mise` is younger and less ubiquitous than `make`, so some contributors meet it
  here for the first time.

### Neutral

- Node is provisioned through `mise` to underpin the TypeScript/React toolchain
  and auxiliary dev tooling (consistent with ADR-0048).
- Enforcement is CI-only — there are no pre-commit hooks; contributors run
  `mise run fix && mise run check` themselves.
- The tree intentionally offers `<component>:check` roll-ups but no
  `<component>:fix`; that asymmetry is documented in `tasks/README.md`.

## References

- **Ported from luminosity** — original decision (lum ADR-0006):
  https://github.com/atomicinnovation/luminosity/blob/main/meta/decisions/ADR-0006-mise-invoke-task-runner.md
- `meta/decisions/ADR-0048-four-toolchain-split.md` — The four toolchains this
  task runner provisions and pins; assigns Python its support-and-test role.
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md` — Establishes
  keeping deterministic logic out of bash, which this decision applies at the
  task layer.
- `meta/decisions/ADR-0049-bash-3.2-compatibility-floor.md` — Defers ShellCheck
  and shfmt version-pinning to this decision; `mise.toml` is where those pins
  live.
- `tasks/README.md` — Documents the shape of the `mise` + invoke task tree.
</content>
