---
id: "ADR-0048"
date: "2026-06-27T12:23:42+00:00"
author: Toby Clemson
status: accepted
tags: [architecture, toolchain, rust, python, shell, typescript, foundations]
type: adr
title: "ADR-0048: Four-Toolchain Split (Python / Shell / Rust / TypeScript)"
schema_version: 1
last_updated: "2026-06-27T12:23:42+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0045", "adr:ADR-0046"]
---

# ADR-0048: Four-Toolchain Split (Python / Shell / Rust / TypeScript)

**Date**: 2026-06-27
**Status**: Accepted
**Author**: Toby Clemson

## Context

Accelerator is a Claude Code plugin. Its runtime substrate today is **shell**
(config reading, VCS detection, frontmatter parsing, migrations, hooks) with
**Python** for build tooling and **Rust** for the visualiser server. ADR-0045
deliberately moves the runtime substrate forward: deterministic procedural logic
belongs in a compiled CLI, not in a body of bash.

Four language toolchains have distinct, deliberately unequal roles:

- **Rust** carries the bulk of the domain — the deterministic product core. The
  CLI is the home for the procedural logic skills delegate to (ADR-0045), and the
  logic behind the plugin's hooks belongs here too. Rust already ships the
  visualiser HTTP server. Distributed as zero-setup static binaries (ADR-0046).
- **Python** is the support language: build, release, and automation tooling
  (invoke tasks under `tasks/`, run through `mise`, type-checked with pyrefly and
  linted with ruff), *and* the general-purpose test language for the non-Rust
  surfaces — testing shell wrappers and writing guardrail tests for components
  that aren't Rust, such as the CI pipeline.
- **Shell (bash)** is the current runtime substrate, with the standing direction
  to confine it to **thin wrappers** — used where the plugin or host environment
  strictly requires a shell entry point and where the wrapper does little more
  than resolve and delegate to the CLI.
- **TypeScript/React** is the visualiser frontend: a React 19 + Vite single-page
  app that renders the `meta/` corpus, linted and formatted with Biome,
  type-checked with `tsc`, and tested with Vitest (unit) and Playwright (E2E). It
  is a genuine GUI product surface — a browser UI no other toolchain can provide —
  not a substitute for the CLI's deterministic core.

Each toolchain carries its own formatters, linters, checks, and pinned versions.
This ADR records the standing decision to keep four toolchains with these roles —
Rust-dominant for the domain, Python-supporting, shell-minimal, and
TypeScript/React for the visualiser frontend — rather than consolidating onto
fewer or reverting to a shell substrate.

## Decision Drivers

- Put the bulk of the domain in one compiled, statically typed, testable,
  distributable language (Rust), per ADR-0045 and ADR-0046.
- Keep a fast-iterating, mature support language for build, release, and
  automation — and, critically, a test language *separate from the product core*
  for exercising the non-Rust surfaces (shell wrappers, CI guardrails).
- Drive shell toward thin wrappers only, avoiding the untestable shell substrate
  ADR-0045 exists to leave behind.
- Provide a real browser UI for the visualiser, which requires a web frontend
  toolchain no other language in the stack supplies.
- Use each language for what it is genuinely best at, and accept the cost of an
  extra toolchain only where it earns its place.

## Considered Options

1. **Four toolchains with deliberately unequal roles** — Rust for the bulk of the
   domain (CLI and hook logic) and the visualiser server, Python as the support
   language (build, release, automation, and testing of non-Rust surfaces), shell
   as thin wrappers only where strictly needed, and TypeScript/React for the
   visualiser frontend.
2. **Collapse the frontend** — drop the TypeScript/React toolchain by rendering
   the visualiser server-side in Rust (templated HTML) or abandoning the browser
   UI, avoiding a dedicated frontend toolchain.
3. **Single runtime language** — drive everything from Rust (including
   build/release automation and the tests for non-Rust surfaces), or the prior
   shell-substrate-plus-Python model with no compiled core.
4. **Drop the support language** — Rust plus shell with Python's support role
   dropped, or Rust plus Python with shell forbidden outright.

## Decision

We will keep **four toolchains in deliberately unequal roles**: **Rust** for the
bulk of the domain (the deterministic CLI core and the logic behind hooks) and
the visualiser server, **Python** as the support language (build, release,
automation, and the test language for non-Rust surfaces), **shell** as thin
wrappers only where strictly required, and **TypeScript/React** for the
visualiser frontend.

We chose option 1 because each language earns a distinct, bounded role:

- **Rust owns the domain.** ADR-0045 establishes that deterministic procedural
  logic belongs in a compiled, testable, distributable core, and ADR-0046 ships
  it as static binaries. Hooks are part of this: the hook logic is implemented in
  the CLI, fronted by a thin shell shim only if the hook entry point demands a
  shell command. Rust also already powers the visualiser HTTP server.
- **Python is the support language, not a second product language.** The invoke
  task tree is mature and fast to iterate for build, release, and automation, and
  rewriting it in Rust would slow that work for little gain. Just as important,
  Python is the test language for everything that *isn't* Rust — it exercises the
  shell wrappers and carries guardrail tests for non-Rust components such as the
  CI pipeline. A test harness separate from the product compiler keeps those
  surfaces testable without coupling them to the Rust build.
- **Shell is minimised, not foundational.** Skills' `!` preprocessor can invoke
  the CLI directly, and hook logic lives in the CLI, so shell is not needed as a
  runtime substrate. It remains as thin wrappers where the environment strictly
  requires a shell entry point.
- **TypeScript/React earns the fourth toolchain.** A browser UI for the visualiser
  is a real product surface, and the web platform is where it lives; no other
  toolchain in the stack can render it. The frontend is bounded to the visualiser
  and uses a single, integrated toolchain (Biome for lint+format, `tsc`, Vitest,
  Playwright), so the cost is contained.

Option 2 was rejected: server-side Rust rendering would forfeit the interactive,
component-driven UI the visualiser needs, and abandoning the browser UI removes a
genuine product surface to save a toolchain that is bounded and well-contained.
Option 3 was rejected at both poles: driving build/release/automation and non-Rust
tests from Rust is heavyweight and couples the test harness to the product
compiler, while a shell substrate is precisely the untestable, fragile model
ADR-0045 exists to leave behind. Option 4 was rejected because dropping Python
forfeits the fast-iterating support and the separate test language the non-Rust
surfaces need, while forbidding shell outright forces awkward workarounds at the
few integration points that genuinely require a shell entry point — better to keep
shell and bound it tightly than to ban it.

This ADR records only the **split and the role each toolchain plays**. The
toolchain-specific decisions that hang off it — the bash 3.2 floor and the 
`mise` + invoke task runner — are recorded as their own ADRs (ADR-0049 and ADR-0050).

## Consequences

### Positive

- The domain lives in one compiled, unit-testable, reusable, distributable
  language; reliability, reuse, and zero-setup distribution follow ADR-0045 and
  ADR-0046.
- Python gives fast-iterating build/release/automation and a proper test language
  for the non-Rust surfaces (shell wrappers, CI guardrails) without dragging a
  compile step into that work.
- Shell stays bounded — thin wrappers as the target — so the fragile-shell failure
  mode is contained by design rather than sitting at the foundation.
- The visualiser gets a first-class browser UI from a toolchain bounded to that
  one surface.

### Negative

- Four toolchains mean four sets of formatters, linters, checks, pinned tool
  versions, and CI lanes to maintain.
- Contributors must work across — or context-switch between — four languages.
- Each boundary (skill↔CLI, wrapper↔CLI, hook-shim↔CLI, frontend↔server) is an
  integration surface to design, test, and keep stable.
- Shared conventions must be duplicated by hand across toolchains — notably the
  80-column line width, copied into each tool's config because none reads
  `.editorconfig` uniformly.

### Neutral

- Node is provisioned (e.g. for the frontend build, `actionlint`, and markdown/CI
  tooling); it underpins the TypeScript/React toolchain and auxiliary dev
  infrastructure.
- The bash 3.2 floor (ADR-0049) still governs the thin shell wrappers and the
  Python-driven shell tests, and the `mise` + invoke task runner (ADR-0050)
  provisions and version-pins all four toolchains; both are recorded separately
  and hang off this split.
- The split is about *roles*: shell's footprint is intended to shrink toward thin
  wrappers as logic migrates into the CLI, while the other three hold steady roles.

## References

- **Ported from luminosity** — original decision (lum ADR-0004, which recorded a
  three-toolchain split; this ADR adds the TypeScript/React visualiser frontend as
  a fourth toolchain Accelerator carries):
  https://github.com/atomicinnovation/luminosity/blob/main/meta/decisions/ADR-0004-three-toolchain-split.md
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md` — Establishes the
  Rust deterministic core that carries the domain.
- `meta/decisions/ADR-0046-zero-setup-static-binary-distribution.md` — How the
  Rust core ships.
- `meta/decisions/ADR-0049-bash-3.2-compatibility-floor.md` — Bash floor governing
  the thin shell wrappers.
- `meta/decisions/ADR-0050-mise-invoke-task-runner.md` — Task runner provisioning
  and version-pinning all four toolchains.
</content>
