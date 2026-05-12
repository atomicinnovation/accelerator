---
adr_id: ADR-0012
date: "2026-04-17T12:42:04+00:00"
author: Toby Clemson
status: accepted
tags: [review-system, lenses, architecture, resilience]
---

# ADR-0012: Resilience as Architecture Responsibility

**Date**: 2026-04-17
**Status**: Accepted
**Author**: Toby Clemson

## Context

The review system (ADR-0002, ADR-0003) uses specialist lenses selected per
review. A gap analysis against industry code review frameworks — Google
engineering practices, OWASP, ISO 25010, and a range of comprehensive
checklists — identified **resilience / reliability** as a secondary coverage
gap. The architecture lens mentioned scalability and resilience together in
a single brief question ("what happens when a component fails?"), but
offered no systematic evaluation of retry strategies, circuit breakers,
graceful degradation, timeout handling, idempotency, or health check design.

These concerns are architectural strategy choices — decisions about how a
system behaves under partial failure — rather than a separate analytical
domain. They belong on the architecture lens's territory but had not been
given deliberate structural treatment. The question was whether to elevate
resilience into its own lens or strengthen coverage within the architecture
lens.

## Decision Drivers

- **Conceptual fit**: resilience concerns (retry strategies, circuit
  breakers, graceful degradation, timeouts, idempotency, health checks) are
  decisions about *how the architecture handles partial failure* —
  architectural strategy, not a separate analytical domain
- **Systematic coverage of the gap**: the gap is real — a single bullet
  question is not systematic treatment — so whichever option is chosen must
  produce deliberate, structured evaluation
- **Reversibility**: the decision is not one-way; if resilience reviews
  become a frequent, differentiated need, the responsibility can be promoted
  to a standalone lens later without losing work
- **Boundary clarity with the new performance lens (ADR-0011)**: resilience
  *strategy* (is retry appropriate here?) and performance *implementation*
  (is the retry loop efficient?) need a clear home each so reviewers don't
  duplicate or drop findings

## Considered Options

1. **Status quo — keep the single bundled bullet** — Leave the architecture
   lens's "what happens when a component fails?" question as it stands.
   Rejected because the gap analysis established that a single bullet does
   not constitute systematic treatment of retry strategies, circuit breakers,
   graceful degradation, timeouts, idempotency, and health checks; the
   premise of the decision is that the gap must be closed.

2. **Standalone resilience lens** — A dedicated lens covering retry
   strategies, circuit breakers, graceful degradation, timeout propagation,
   idempotency, error recovery, failover, and health check design. Gives
   resilience the same structural weight as other top-level quality
   concerns and a clean separation of evaluation output. Splits a
   conceptually unified domain (architectural response to failure) across
   two lenses, since load/scalability stays in architecture — reviewers
   would need to reason about which lens owns each failure-related question.

3. **Extend the architecture lens with a resilience responsibility** — Add
   a fourth Core Responsibility to the architecture lens covering the same
   set of concerns, with corresponding evaluation questions. Keeps all
   architectural strategy in one place, where scalability, modularity, and
   resilience are evaluated together as facets of system design. Makes the
   architecture lens broader (four responsibilities rather than three) and
   defers the question of whether resilience deserves independent focus.

## Decision

We will extend the architecture lens with a fourth Core Responsibility —
"Evaluate Resilience and Fault Tolerance" — covering retry strategies and
backoff policies, circuit breaker patterns, graceful degradation, timeout
setting and propagation, idempotency guarantees, error recovery and
compensation, health and readiness check design, and single-point-of-failure
identification. The existing "Scalability & resilience" evaluation question
is split into two separate questions — one for scalability, one for
resilience — each with systematic sub-points.

We draw the boundary with the performance lens (ADR-0011) on *strategy vs
implementation*: the architecture lens assesses whether the resilience
*strategy* is appropriate for the failure modes at hand (is retry the right
response? is the backoff policy sound? should this call have a circuit
breaker?). The performance lens assesses whether the *implementation* of
that strategy is efficient (is the retry loop allocating unnecessarily? is
the circuit breaker's state check hot?).

This decision is deliberately provisional: if experience with reviews shows
resilience findings consistently warrant independent structural focus, the
responsibility can be promoted to a standalone lens in a future ADR without
loss of coverage.

## Consequences

### Positive

- Resilience gets systematic, structured evaluation — a dedicated
  responsibility with its own evaluation questions rather than a single
  bundled bullet, closing the coverage gap.
- Architectural strategy stays together in one lens: scalability,
  modularity, coupling, and resilience are evaluated as facets of the same
  design discipline, which matches how architects typically reason.
- Lower maintenance cost than a new lens: no cascade of "What NOT to Do"
  updates across every other lens, no new auto-detect criteria to define,
  no new output identifier to thread through the output format skills.
- The path forward is preserved — the decision is reversible, so promoting
  resilience to a standalone lens later is straightforward if reviews
  warrant it.

### Negative

- The architecture lens becomes broader (four Core Responsibilities instead
  of three) — risk of the lens feeling like a grab bag and losing the
  focused-perspective quality that lenses are designed to provide.
- Finite reviewer attention within a single agent invocation: adding a
  fourth responsibility may dilute depth on the other three, especially on
  changes where resilience is dominant. When a change is resilience-heavy
  (new external integration, introduction of retries or circuit breakers,
  failover redesign), expect coverage tradeoffs against evolutionary-fitness
  and coupling questions — this is the signal that warrants revisiting the
  provisional decision.
- The strategy-vs-implementation boundary with the performance lens
  (ADR-0011) requires ongoing discipline — the two lenses must keep
  labelling findings consistently or reviewers will see duplicate or
  dropped issues at the seam.
- All resilience findings come tagged as `architecture` in the output, so
  consumers (dashboards, trend analysis) cannot filter resilience separately
  from other architectural concerns without heuristics.

### Neutral

- Decision is explicitly provisional; a future ADR can promote resilience to
  a standalone lens if review patterns justify it.
- Other lenses' "What NOT to Do" sections already defer resilience concerns
  to architecture — verified in the safety lens ("Don't assess architectural
  resilience patterns for fitness — that is the architecture lens's
  responsibility") and the performance lens ("Resilience patterns — retry
  strategies, circuit breakers, timeout policies — are assessed by the
  architecture lens"). No change to existing deferral patterns is needed,
  since resilience stays where it already nominally lived.

## References

- `meta/research/codebase/2026-02-22-review-lens-gap-analysis.md` — Gap analysis
  identifying resilience as a secondary coverage gap and recommending
  extension over a standalone lens
- `meta/plans/2026-02-23-performance-lens-and-resilience-extension.md` —
  Implementation plan defining the fourth Core Responsibility and split of
  the scalability/resilience question
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` — Three-layer
  review architecture the architecture lens sits within
- `meta/decisions/ADR-0003-pbr-lens-design-with-structural-invariants.md` —
  Lens design principles the resilience responsibility conforms to
- `meta/decisions/ADR-0011-standalone-performance-lens.md` — Sibling
  decision; defines the strategy-vs-implementation boundary with this lens
