---
adr_id: ADR-0002
date: "2026-03-28T12:58:11+00:00"
author: Toby Clemson
status: accepted
tags: [review-system, architecture, agents, parallel-execution, orchestration]
---

# ADR-0002: Three-Layer Architecture for Multi-Lens Review System

**Date**: 2026-03-28
**Status**: Accepted
**Author**: Toby Clemson

## Context

The accelerator plugin includes a review system that evaluates code changes and
plans across multiple quality dimensions — architecture, security, code quality,
performance, and others — each implemented as a specialist "lens." Each lens runs
as an independent agent with its own context window and produces structured
findings.

The three-layer separation emerged organically from the plan review system before
PR review was designed. The original design research
(`meta/research/2026-02-22-pr-review-agents-design.md`) identified this as an
existing architectural pattern worth formalising: specialist agents at the
bottom, orchestrating commands in the middle, shared conventions binding them
together. PR review was built to follow the same pattern.

Several forces shape this architecture. Sequential review across all lenses
would make total review time the sum of all lens durations, which becomes
unacceptable as the catalogue grows (currently 13 lenses). Agents executing in
parallel cannot communicate with each other directly, so coordination must happen
at a higher layer. Each agent is read-only — limited to Read, Grep, Glob, and LS
tools — and cannot modify the codebase or influence other agents' output. The
orchestrator prepares shared input data (diff, changed file list, PR description)
once and passes file paths to all agents, rather than each agent fetching
independently. Meanwhile, independently-produced findings need shared conventions
(severity tiers, confidence ratings, output schemas) to be meaningfully compared,
deduplicated, and synthesised into a coherent review.

## Decision Drivers

- **Review latency**: parallel execution means total time equals the slowest
  lens, not the sum
- **Independent extensibility**: adding a lens should not require changes to
  other lenses or the orchestrator
- **Lens-agnostic orchestration**: the orchestrator should work identically
  regardless of which or how many lenses run
- **Consistent output quality**: shared conventions ensure findings from any lens
  are comparable and mergeable
- **Clean separation of concerns**: what to evaluate (specialist), how to
  coordinate (orchestrator), and what conventions to follow (convention layer)
  are distinct responsibilities

## Considered Options

1. **Monolithic reviewer** — A single agent evaluates all quality dimensions in
   one pass. Simplest to implement — no coordination overhead, single context
   window. However, a single agent cannot be expert across all dimensions
   simultaneously, the context window fills quickly when covering many concerns,
   and adding new dimensions means modifying one increasingly complex prompt.
   Does not support parallel execution.

2. **Peer-to-peer agents** — Specialist agents communicate directly with each
   other to share findings and coordinate coverage. Could produce richer
   cross-cutting analysis. However, Claude Code agents have no mechanism for
   direct inter-agent communication — they run in isolated context windows.
   Simulating peer-to-peer via filesystem would add significant complexity and
   ordering dependencies, undermining the latency benefits of parallelism.

3. **Per-lens specialist agents** — A dedicated agent definition for each quality
   dimension (e.g., `security-reviewer`, `architecture-reviewer`). Each agent
   encodes its own domain expertise. Simple conceptually, but creates a
   maintenance burden that grows linearly with the lens catalogue — every shared
   concern (tool access, output format, scoping conventions) must be duplicated
   or kept in sync across all agent definitions.

4. **Three-layer architecture with pluggable lenses** — A single generic
   reviewer agent is specialised at spawn time by injecting a lens skill.
   Orchestrating skills coordinate parallel execution and synthesise results.
   Shared conventions enable cross-agent aggregation. Each layer has a single
   responsibility and can evolve independently.

## Decision

We will use a three-layer architecture for the review system: a specialist
layer, an orchestrator layer, and a convention layer.

The **specialist layer** uses a single generic reviewer agent that is specialised
at spawn time by injecting a lens skill. The lens skill defines what to evaluate
and the domain-specific criteria; the agent definition provides the execution
scaffold — tool access, output structure, and scoping behaviour. Each specialist
is read-only (Read, Grep, Glob, LS only) and runs in its own context window.
Adding a new quality dimension means writing a new lens skill, not a new agent.

The **orchestrator layer** is a coordinating skill (e.g., `review-pr`,
`review-plan`) that prepares shared input data, selects relevant lenses, spawns
all specialists in parallel as background tasks, waits for the slowest to
complete, then aggregates, deduplicates, and synthesises findings into a unified
review. The orchestrator is lens-agnostic — it applies the same aggregation
logic regardless of which or how many lenses run.

The **convention layer** consists of shared output format skills that define
severity tiers, confidence ratings, finding structure, and output schemas. These
conventions allow independently-produced findings from any specialist to be
meaningfully compared and merged by the orchestrator without lens-specific logic.

All three layers are independently extensible: a new lens requires only a new
lens skill; the orchestrator and conventions remain unchanged.

## Consequences

### Positive

- Near-zero latency cost for new lenses — parallel execution means adding a lens
  only affects total time if it becomes the slowest
- Single agent definition scales to any number of lenses — no proliferation of
  agent definitions as the catalogue grows
- Orchestrator is lens-agnostic — the same aggregation and synthesis logic works
  regardless of which or how many lenses run
- Each lens is independently testable and modifiable — changes to one lens cannot
  break another
- Convention layer ensures consistent quality signalling — findings from any
  specialist are comparable regardless of origin

### Negative

- Convention changes have wide blast radius — a change to severity tiers or
  output schemas requires updating every lens and output format skill
- The orchestrator must handle malformed output gracefully — any specialist can
  produce unexpected output, and the orchestrator cannot validate findings
  against lens-specific semantics it doesn't understand
- Three layers means three places to understand when debugging — a review issue
  could originate in the lens skill, the agent scaffold, or the orchestrator's
  synthesis logic

### Neutral

- Each specialist runs in its own context window — isolates token budgets between
  specialists and from the orchestrator, consistent with the context isolation
  principles established in ADR-0001
- The single-agent design couples all lenses to one agent definition — this is a
  deliberate tradeoff; shared concerns are maintained once rather than N times,
  but a breaking change to the reviewer agent affects all lenses simultaneously

## References

- `meta/research/2026-02-22-pr-review-agents-design.md` — Original design
  research that identified the three-layer pattern in the existing plan review
  system
- `meta/research/2026-02-22-skills-agents-commands-refactoring.md` — Detailed
  design of the generic reviewer agent with pluggable lens skills pattern
- `meta/plans/2026-02-22-pr-review-agents.md` — Original implementation plan
  establishing the orchestrator and parallel agent spawning pattern
- `meta/plans/2026-02-23-performance-lens-and-resilience-extension.md` —
  Confirms the lens-agnostic property: adding a lens requires no changes to the
  orchestrator or agent
- `meta/decisions/ADR-0001-context-isolation-principles.md` — Establishes the
  context isolation principles that the specialist layer depends on (agent
  separation, token budgets)
