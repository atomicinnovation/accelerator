---
id: "ADR-0045"
date: "2026-06-27T12:23:42+00:00"
author: Toby Clemson
status: accepted
tags: [architecture, skills, cli, division-of-labour, foundations]
type: adr
title: "ADR-0045: Skills-vs-CLI Division of Labour"
schema_version: 1
last_updated: "2026-06-27T12:23:42+00:00"
last_updated_by: Toby Clemson
---

# ADR-0045: Skills-vs-CLI Division of Labour

**Date**: 2026-06-27
**Status**: Accepted
**Author**: Toby Clemson

## Context

Accelerator is a Claude Code plugin. Its shipped product is a set of skills,
agents, hooks, templates, and scripts that Claude Code loads, with deterministic
logic expressed in shell scripts and Python build tasks.

Large language models are excellent at probabilistic work — reasoning,
generation, summarisation, judgement — but are, by nature, non-deterministic,
costly per token, and unreliable for exact procedural logic such as parsing,
file manipulation, version-coherence enforcement, or fixed multi-step
transforms. Deterministic logic expressed in skill prose or inline shell is hard
to test, slow, token-expensive, and error-prone, and it accretes ad hoc as the
plugin grows. We know this from our own history: Accelerator's deterministic
logic grew into a large body of bash scripts (~226 `.sh` files) that became hard
to test, slow to change, and error-prone — concrete evidence of the failure mode
this boundary exists to avoid. The plugin is expected to grow further, and it
already runs a backend and a frontend: the visualiser's Rust HTTP server and its
React SPA. Building the visualiser forced us to **duplicate deterministic logic
and data definitions across Bash and Rust** — the same corpus parsing, schema,
and path conventions implemented once for the skills' shell library and again for
the visualiser server. A shared compiled core lets both surfaces reuse one
implementation rather than re-deriving it per surface, removing that duplication.

The plugin therefore needs an explicit, durable boundary that assigns each kind
of work to the medium suited to it, rather than letting probabilistic and
deterministic concerns intermingle in Markdown and bash.

## Decision Drivers

- Reliability and testability of deterministic logic.
- Token cost and latency — the model should not perform mechanical work it
  cannot do reliably or cheaply.
- A clear separation of concerns that holds as the plugin scales.
- Reuse of the deterministic core across surfaces — the skills and the existing
  visualiser server — removing the Bash/Rust duplication the visualiser currently
  requires.
- Using each tool for what it is genuinely best at.

## Considered Options

1. **Skills own probabilistic work; deterministic procedural logic is delegated
   to a compiled CLI** — written in a modern, testable, dependency-managed
   language; a skill decides and orchestrates, the CLI executes the
   deterministic work.
2. **Skills do everything in-prompt** — both judgement and procedural logic live
   in Markdown via the `!` preprocessor and inline bash.
3. **Keep deterministic product logic in bash/Python scripts** (the status quo
   substrate) — no dedicated runtime CLI; skills shell out to scripts.

## Decision

We will divide labour so that **skills own only probabilistic work** (reasoning,
generation, summarisation, judgement) and **delegate all deterministic, standard
procedural logic to a compiled CLI** written in a modern, testable,
dependency-managed language. A skill decides and orchestrates; the CLI executes
the deterministic work, invoked from the skill at runtime.

This ADR records the **division of labour** only. What turns on this decision is
that the deterministic core lives in a compiled, statically typed, testable,
dependency-managed CLI — not the specific language. The concrete language (Rust)
is the direction detailed by the CLI-architecture spike (work item 0158), and the
CLI's internal structure — a thin CLI over a hexagonal ports-and-adapters core —
is a separate decision (ADR-0053), recorded as its own spike-dependent ADR. The
existing Python build tasks and bash scripts remain the **development tooling**
and are out of scope here (recorded as their own decisions).

We chose option 1 because it is the only option that keeps deterministic logic
testable, fast, and reliable while letting skills stay lean and focused on
judgement. Option 2 was rejected: deterministic logic in prose or inline bash is
effectively untestable, token-heavy, and non-deterministic, and it does not
scale as the plugin grows. Option 3 was rejected as the long-term home for
product logic — our own bash body is the cautionary precedent: the bash 3.2 floor
and the absence of static typing limit robustness, the zero-setup distribution
story for scripts is weak, and scripts are not cleanly reusable by the visualiser
server or other surfaces — building the visualiser already forced duplicating
logic and definitions across Bash and Rust.

## Consequences

### Positive

- Deterministic logic becomes unit-testable, fast, and free of token cost and
  model variance.
- Skills stay lean and focused on judgement, improving their clarity and
  evaluability.
- A clear, enforceable boundary scales as the plugin grows rather than accreting
  logic ad hoc.
- The compiled core is reusable across surfaces — the skills and the visualiser
  server — collapsing the duplicated Bash/Rust logic into one implementation, and
  ships as zero-setup static binaries.

### Negative

- Introduces a compiled-language toolchain (Rust) and a binary
  build/distribution pipeline the plugin would not otherwise need.
- Adds a second artifact that must be kept version-coherent with the plugin.
- The skill↔CLI boundary is a new integration surface to design, test, and keep
  stable.
- Classifying work as "probabilistic" vs "deterministic" requires judgement at
  the margins.

### Neutral

- Skills invoke the CLI via the `!` preprocessor / command calls at invocation
  time.
- The existing Python invoke tasks and bash library remain in place for
  development tooling, governed by separate decisions.
- The first concrete proof of the division is the `configure` skill backed by
  the CLI rather than by shell.

## References

- **Ported from luminosity** — original decision (lum ADR-0001):
  https://github.com/atomicinnovation/luminosity/blob/main/meta/decisions/ADR-0001-skills-vs-cli-division-of-labour.md
- `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`
  — CLI architecture spike; basis for the separate hexagonal-core ADR (ADR-0053).
- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
  — Accelerator's existing shell-script surface and Rust-CLI migration research.
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
  — the CLI's internal architecture (a thin CLI over a hexagonal core).
