---
title: "Performance as dedicated lens, resilience into architecture"
type: adr-creation-task
status: todo
---

# ADR Ticket: Performance as dedicated lens, resilience into architecture

## Summary

In the context of the gap analysis revealing no lens covered code-level
performance and only superficial resilience coverage, we decided for a
standalone performance lens (algorithmic complexity, resource efficiency,
concurrency) and extending the architecture lens with a resilience section
rather than a separate lens, to achieve focused performance evaluation without
lens proliferation, accepting boundary clarification between performance
(code-level implementation) and architecture (system-level strategy).

## Context and Forces

- Gap analysis against industry frameworks (OWASP, ISO 25010) revealed
  performance as universally recommended but absent from the existing 6 lenses
- Resilience/reliability was only superficially covered by the architecture lens
- Adding both as standalone lenses would increase the count from 6 to 8
- Performance concerns (algorithms, N+1 queries, caching, concurrency) are
  code-level and distinct from architecture's system-level scalability concerns
- Resilience concerns (retry strategies, circuit breakers, graceful degradation)
  are architectural strategy choices, not a separate domain

## Decision Drivers

- Industry-standard review coverage (performance is universally recommended)
- Keeping lens count manageable (restraint principle)
- Clean domain boundaries between code-level and system-level concerns
- Extensibility: the architecture supports adding lenses cheaply

## Considered Options

For performance:
1. **Fold into code quality** — Loses focus; code quality is already broad
2. **Fold into architecture** — Conflates code-level and system-level concerns
3. **Standalone performance lens** — Focused evaluation of code-level
   performance

For resilience:
1. **Standalone resilience lens** — Adds another lens for a concern that is
   fundamentally architectural
2. **Extend architecture lens** — Add a 4th Core Responsibility covering
   resilience and fault tolerance. Can be promoted to standalone later if needed.

## Decision

We will add a standalone performance lens covering algorithmic complexity,
resource efficiency, database query performance, caching, I/O efficiency, and
concurrency/thread safety. Resilience will be added as a 4th Core
Responsibility of the architecture lens covering retry strategies, circuit
breakers, graceful degradation, timeouts, idempotency, and health checks. The
boundary: architecture assesses whether the resilience *strategy* is
appropriate; performance assesses whether the *implementation* is efficient.

## Consequences

### Positive
- Performance gets focused, dedicated evaluation
- Resilience is covered without lens proliferation (7 lenses not 8)
- Clear boundary between strategy (architecture) and implementation
  (performance)
- Resilience can be promoted to standalone later if reviews warrant it

### Negative
- Architecture lens becomes broader (4 responsibilities instead of 3)
- Boundary between performance and architecture requires ongoing clarity
- Observability infrastructure (logging, metrics) stays in code quality, which
  may surprise some users

### Neutral
- The decision to keep resilience in architecture is explicitly a restraint
  decision, not a permanent architectural constraint

## Source References

- `meta/research/2026-02-22-review-lens-gap-analysis.md` — Gap analysis
  identifying performance as primary gap and resilience as secondary
- `meta/plans/2026-02-23-performance-lens-and-resilience-extension.md` —
  Implementation plan with boundary definitions
