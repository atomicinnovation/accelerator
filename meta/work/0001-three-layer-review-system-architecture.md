---
title: "Three-layer review system architecture"
type: adr-creation-task
status: done
---

# ADR Ticket: Three-layer review system architecture

## Summary

In the context of building a multi-lens review system, facing the need for
specialized analysis across quality dimensions, we decided for a three-layer
architecture (agent layer, command/orchestrator layer, convention layer) with
parallel independent execution to achieve unified synthesis from concurrent
specialist agents, accepting the overhead of maintaining consistent conventions
across all layers.

## Context and Forces

- The review system needs to evaluate code and plans across multiple quality
  dimensions (security, architecture, code quality, etc.) simultaneously
- Each dimension requires specialist knowledge that doesn't overlap cleanly
- Sequential review would be too slow; parallel execution is essential
- Agents executing in parallel cannot communicate with each other directly
- The orchestrator must aggregate, deduplicate, and synthesise results from
  independent agents into a coherent review
- Shared conventions (severity tiers, confidence ratings, output formats) are
  needed so that independently-produced findings can be meaningfully compared
  and merged

## Decision Drivers

- Review latency (parallel execution means total time = slowest lens, not sum)
- Independent extensibility (adding a lens should not require changes to other
  lenses)
- Consistent output quality regardless of which or how many lenses run
- Clean separation of concerns between what to evaluate, how to coordinate, and
  what conventions to follow

## Considered Options

1. **Monolithic reviewer** — A single agent evaluates all dimensions in one
   pass. Simple but leads to unfocused analysis and doesn't scale.
2. **Peer-to-peer agents** — Agents communicate with each other to coordinate.
   Complex and fragile.
3. **Three-layer architecture** — Agents are read-only specialists, orchestrator
   commands coordinate and synthesise, shared conventions enable cross-agent
   aggregation. Each layer has a clear responsibility.

## Decision

We will use a three-layer architecture: the agent layer (specialist reviewers
that produce structured findings), the command/orchestrator layer (coordinates
execution, aggregates results, posts to GitHub), and the convention layer
(shared severity tiers, confidence ratings, output schemas). All lenses execute
in parallel as background tasks; the orchestrator waits for the slowest lens.

## Consequences

### Positive
- Parallel execution: adding lenses has near-zero latency cost
- Each lens is independently testable and modifiable
- The orchestrator is lens-agnostic — same aggregation logic regardless of which
  lenses run
- Convention layer ensures consistent quality signalling across all findings

### Negative
- Conventions must be maintained across all lenses — a change to severity tiers
  requires updating every lens
- The orchestrator must handle malformed output from any agent gracefully
- Three layers means three places to understand when debugging

### Neutral
- Each agent runs in its own context window, isolating its token budget from the
  orchestrator and other agents

## Source References

- `meta/research/codebase/2026-02-22-pr-review-agents-design.md` — Original design
  research establishing the three-layer pattern
- `meta/plans/2026-02-23-performance-lens-and-resilience-extension.md` —
  Confirms parallel execution model is preserved when adding lenses
