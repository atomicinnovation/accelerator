---
id: "ADR-0051"
date: "2026-06-27T12:23:42+00:00"
author: Toby Clemson
status: accepted
tags: [architecture, skills, plugin, product, foundations]
type: adr
title: "ADR-0051: Skills as the Product"
schema_version: 1
last_updated: "2026-06-27T12:23:42+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0045"]
---

# ADR-0051: Skills as the Product

**Date**: 2026-06-27
**Status**: Accepted
**Author**: Toby Clemson

## Context

Accelerator delivers behaviour to its users. That behaviour could be packaged
many ways: as a standalone application with its own agent loop wrapping an LLM
API, as an MCP server or library any host integrates, or as a Claude Code plugin
whose skills the Claude Code host loads and runs.

This project commits to the last shape. Its artefacts are **skills** (`SKILL.md`
Markdown files with YAML frontmatter), together with the **agents**, **hooks**,
and **templates** that support them, registered in `.claude-plugin/plugin.json`
and loaded by Claude Code at runtime. A skill's body is natural-language prose; it
injects live context through the `!` preprocessor and orchestrates work,
delegating deterministic procedural logic to a compiled CLI (ADR-0045). There is
no bespoke agent loop, no standalone host, and no conventional application
wrapping the model — the Claude Code host provides the runtime, tool use, context
management, and permissioning.

The host today is the interactive Claude Code CLI, but it need not be the only
one: the Claude Agent SDK can drive the same plugin and skills programmatically
and headlessly, so a future frontend or backend could trigger the same workflows
and model interactions without reimplementing them. The product therefore targets
the *Claude Code runtime*, not specifically its interactive CLI.

This is the shape Accelerator already runs on — a substantial body of behaviour
shipped entirely as a Claude Code plugin. Recording it as an ADR makes the product
boundary explicit: the product is *skills running on the Claude Code runtime*, not
software that merely uses a model.

## Decision Drivers

- Leverage the Claude Code host (agent loop, tool dispatch, context management,
  permissioning) rather than building and maintaining our own harness.
- Express behaviour in natural-language prose, close to how the model reasons,
  so skills are fast to author and iterate.
- Distribution and discoverability through the Claude Code plugin/marketplace
  mechanism, which users already have installed.
- Composition with Claude Code's built-in tools and other plugins for free.
- Stay on the proven plugin shape rather than reinventing a runtime.
- A programmatic path (the Claude Agent SDK) that lets a future frontend or
  backend drive the same skills, so the product is not locked to interactive use.
- Focus engineering effort on domain behaviour, not harness plumbing.

## Considered Options

1. **Skills as the product** — ship behaviour as a Claude Code plugin (skills
   plus supporting agents, hooks, templates) that the Claude Code runtime loads
   and executes, whether via the interactive CLI or the Claude Agent SDK.
2. **Standalone application** — build a conventional CLI/app/service with its
   own agent loop calling an LLM API directly, independent of Claude Code.
3. **Library or MCP server only** — ship capability as an MCP server or library
   that any host integrates, with no skill layer of its own.

## Decision

We will ship the product **as a Claude Code plugin whose skills are the primary
deliverable**, supported by agents, hooks, and templates, registered in
`.claude-plugin/plugin.json` and loaded by the Claude Code runtime. Skills are
authored as `SKILL.md` Markdown with frontmatter; they orchestrate in prose and
delegate deterministic work to the CLI (ADR-0045). We rely on Claude Code for
the agent loop, tool use, context management, and permissioning rather than
building our own, and treat the Claude Agent SDK as the programmatic entry point
should a frontend or backend later need to drive the same skills headlessly.

We chose option 1 because it lets us deliver behaviour without building a host,
express that behaviour in the medium the model reasons in, and distribute
through a mechanism users already have — with our existing skill corpus as proof
the shape scales. Option 2 was rejected: building and maintaining an agent loop,
tool dispatch, and context management is a large, ongoing cost that duplicates
what Claude Code already provides and diverts effort from domain behaviour.
Option 3 was rejected as the *whole* product: an MCP server or library exposes
capability but provides no orchestration, prompts, or user-facing workflow, so
something would still have to drive it — which is precisely the skill layer this
decision keeps. (Deterministic capability may still be exposed as a CLI per
ADR-0045; that is a complement, not a replacement, for the skill layer.)

## Consequences

### Positive

- No agent loop, tool-dispatch layer, or context manager to build and maintain
  — the runtime provides them.
- Behaviour lives in natural-language prose, so it is quick to author, read, and
  iterate.
- The plugin/marketplace mechanism gives distribution and discoverability to
  users who already run Claude Code.
- Skills compose with Claude Code's built-in tools and other installed plugins
  at no extra cost.
- The same skills can be driven headlessly through the Claude Agent SDK, so a
  future frontend or backend can trigger the workflows and model interactions
  without reimplementing them.

### Negative

- The product is bound to the Claude Code runtime and ecosystem: it runs only
  where that runtime runs (the CLI or the SDK), not on an arbitrary host.
- It is coupled to Claude Code's capabilities, lifecycle, and versioning — it
  sets a minimum supported version (currently v2.1.144) and must track host
  changes.
- Skill behaviour is probabilistic and must be evaluated rather than unit-tested,
  motivating a dedicated skill-evaluation approach (the skill-evaluation spike,
  work item 0159).
- Limited control over host internals; we adapt to Claude Code rather than shape
  it.

### Neutral

- Deterministic procedural logic is delegated to the compiled CLI (ADR-0045);
  this decision governs the product *shape*, while a separate decision governs
  the *division of labour*.
- Skills are grouped by category and registered in `.claude-plugin/plugin.json`;
  supporting agents, hooks, and templates ship alongside them.
- Skills communicate across phases through the filesystem rather than the
  conversation (recorded as a separate ADR — ADR-0052).
- The interactive CLI is the host today; an SDK-driven frontend or backend is a
  possible future, not a current commitment.

## References

- **Ported from luminosity** — original decision (lum ADR-0007):
  https://github.com/atomicinnovation/luminosity/blob/main/meta/decisions/ADR-0007-skills-as-the-product.md
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md` — Companion
  decision on what work lives in skills vs. the CLI.
- `meta/decisions/ADR-0052-filesystem-as-message-bus-and-knowledge-corpus.md` —
  The filesystem-as-message-bus mechanism this decision defers to.
- `meta/work/0159-skill-evaluation-framework-selection.md` — The skill-evaluation
  spike motivated by skills being the product.
</content>
