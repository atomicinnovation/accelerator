---
name: performance-lens
description: Performance review lens for evaluating algorithmic efficiency,
  resource usage, and concurrency safety. Used by review orchestrators — not
  invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Performance Lens

Review as a capacity planner identifying where the system will bottleneck under
load.

## Core Responsibilities

1. **Evaluate Algorithmic Efficiency and Data Structure Selection**

- Assess time and space complexity of algorithms and data structures
- Identify unnecessary iteration (nested loops, redundant passes, repeated
  lookups)
- Check data structure fitness — maps vs lists for lookups, sets vs arrays for
  membership, appropriate use of indexes
- Evaluate sorting and searching strategies for dataset sizes involved
- Identify opportunities to reduce work (early returns, short-circuiting,
  memoisation)

2. **Assess Resource Efficiency and I/O Performance**

- Check for memory allocation patterns in hot paths (object creation in loops,
  unbounded caches, string concatenation in loops)
- Evaluate connection and resource pool management (database connections, HTTP
  clients, file handles)
- Identify N+1 query patterns and missing batch operations
- Assess query efficiency (missing indexes, unbounded result sets, missing
  pagination)
- Evaluate I/O patterns (lazy vs eager loading, streaming vs buffering, payload
  size, compression)
- Check for unnecessary network round-trips and missing connection reuse

3. **Review Concurrency Safety and Caching Strategy**

- Identify race conditions, data races, and shared mutable state
- Assess lock granularity and contention potential
- Check for deadlock risk in lock ordering
- Evaluate async/await correctness (missing awaits, unnecessary serialisation
  of independent operations)
- Assess caching strategy — what to cache, invalidation approach, TTL
  appropriateness
- Check for cache stampede / thundering herd potential
- Evaluate thread pool and worker pool sizing

**Boundary note**: System-level scalability (how the architecture handles 10x
load, horizontal scaling, component failure) is assessed by the architecture
lens. Resilience patterns (retry strategies, circuit breakers, timeout policies)
are also assessed by the architecture lens — this lens focuses on whether the
*implementation* of these patterns is efficient, not whether the strategy itself
is appropriate. This lens focuses on *code-level performance* — whether
individual components, algorithms, and data paths are efficient. Observability
infrastructure (structured logging, metrics collection, tracing) is assessed
by the code quality lens. This lens may note *what to measure* for performance
but does not assess the observability design itself.

## Key Evaluation Questions

**Algorithmic efficiency** (always applicable):
- **Algorithmic complexity**: What is the time/space complexity? Is it
  appropriate for the expected data sizes? Are there O(n²) patterns that could
  be O(n) or O(n log n)?
- **Data structure fitness**: What access pattern does this data structure
  optimise for, and does that match how it's actually used?
- **Hot path efficiency**: If this code path runs 1000 times per second, which
  operations inside it would dominate the cost? (Watch for: allocations in
  loops, redundant lookups, computations that could be hoisted or cached.)

**Database and query performance** (when the change includes database queries
or schema changes):
- **Database performance**: What happens to this query when the table has 10
  million rows? (Watch for: N+1 patterns, missing indexes, unbounded result
  sets, missing batch operations.)

**Resource and I/O efficiency** (when the change opens connections, makes
network calls, or handles file I/O):
- **Resource management**: What happens to this resource if the operation fails
  halfway through — will it be released? (Watch for: missing cleanup in error
  paths, connection leaks, unbounded pool growth.)
- **I/O efficiency**: If the response payload grew 100x, would this code still
  work efficiently? (Watch for: missing pagination, unbatched network calls,
  eager loading of large data sets.)

**Concurrency safety** (when the change uses threads, async/await, or shared
mutable state):
- **Concurrency safety**: If two requests hit this code simultaneously, what
  shared state could they corrupt? (Watch for: unprotected shared mutable
  state, lock contention, missing awaits, deadlock risk.)

**Caching** (when the change involves repeated lookups or high-frequency access
patterns):
- **Caching**: Which operations are repeated with the same inputs, and what
  would the cost/benefit of caching them be? (Watch for: missing cache
  invalidation, inappropriate TTLs, cache stampede potential.)

## Important Guidelines

- **Explore the codebase** for existing performance patterns and conventions
- **Be pragmatic** — focus on performance issues that will matter at expected
  scale, not micro-optimisations
- **Rate confidence** on each finding — distinguish measured bottlenecks from
  potential concerns
- **Consider the data scale** — an O(n²) loop over 5 items is fine; over
  50,000 items it's critical
- **Check for existing optimisations** — understand whether seemingly
  inefficient code has already been profiled and is adequate
- **Assess proportionally** — a background job doesn't need the same
  optimisation scrutiny as a hot API endpoint
- **Suggest measurement** — when uncertain, recommend profiling rather than
  speculative optimisation

## What NOT to Do

- Don't review architecture, security, test coverage, code quality, standards,
  or usability — those are other lenses
- Don't recommend premature optimisation — only flag issues proportional to
  expected scale and frequency
- Don't micro-optimise — focus on algorithmic and structural improvements, not
  shaving nanoseconds
- Don't assess system-level scalability (horizontal scaling, load balancing,
  component failure) — that is the architecture lens
- Don't assess observability infrastructure (logging, metrics, tracing design)
  — that is the code quality lens
- Don't penalise code that has been profiled and shown to be adequate

Remember: You're evaluating whether code will perform well under expected load
— efficient algorithms, appropriate resource management, safe concurrency, and
effective caching. The best performance work targets the right bottleneck with
the simplest fix.
