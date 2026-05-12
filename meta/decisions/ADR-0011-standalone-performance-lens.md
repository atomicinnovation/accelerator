---
adr_id: ADR-0011
date: "2026-04-17T12:42:04+00:00"
author: Toby Clemson
status: accepted
tags: [review-system, lenses, performance]
---

# ADR-0011: Standalone Performance Lens

**Date**: 2026-04-17
**Status**: Accepted
**Author**: Toby Clemson

## Context

The review system (ADR-0002, ADR-0003) uses specialist lenses selected per
review. A gap analysis against industry code review frameworks — Google
engineering practices, OWASP, ISO 25010, and a range of comprehensive
checklists — identified **performance** as the most significant coverage gap:
performance is universally listed as a primary review dimension, but no
existing lens evaluated it.

Code-level performance concerns (algorithmic complexity, N+1 queries, caching
strategy, resource management, I/O efficiency, concurrency safety) did not
cleanly belong to any existing lens. The architecture lens covered system-level
scalability ("what happens under 10x load?") but not code-level efficiency.
The code quality lens covered complexity and design principles but not
algorithmic performance. Reviews lacked a perspective dedicated to whether
code would perform well under expected load.

## Decision Drivers

- **Comprehensive quality coverage**: performance is universally cited as a
  primary review dimension; its absence leaves reviews blind to a significant
  class of defects
- **Clean domain boundaries**: code-level efficiency concerns are distinct
  from system-level scalability (architecture) and from complexity and design
  principles (code quality)
- **Perspective focus**: a dedicated lens ensures performance receives
  deliberate evaluation rather than being checked opportunistically as a
  side-effect of other lenses
- **Alignment with the lens model**: the review system is built for specialist
  perspectives; expanding the catalogue to cover a missing dimension is the
  intended extension path

## Considered Options

1. **Fold performance into the code quality lens** — Code quality already
   covers complexity, design principles, and error handling; performance
   arguably sits alongside those as aspects of well-written code. Avoids
   adding a lens but stretches code quality further — it is already one of
   the broadest lenses, and performance concerns (query patterns, concurrency
   safety, caching) are distinct enough to dilute its focus.

2. **Fold performance into the architecture lens** — The architecture lens
   already covers scalability, which shares vocabulary with performance.
   Keeps system-level and code-level efficiency concerns adjacent. Conflates
   two levels of analysis: whether the architecture scales is a different
   question from whether the code is efficient, and bundling them blurs the
   reviewer's focus.

3. **Standalone performance lens** — A dedicated lens covering algorithmic
   complexity, resource efficiency, database and query performance, caching,
   I/O, and concurrency safety. Gives performance deliberate, focused
   evaluation with clean boundaries against architecture (system-level
   strategy) and code quality (design and observability infrastructure).
   Requires clearly drawing those boundaries to prevent overlap.

## Decision

We will add a standalone performance lens that evaluates code-level
performance: algorithmic complexity and data structure fitness, resource
efficiency, database and query performance (including N+1 patterns), caching
strategy, I/O and network efficiency, and concurrency safety. The lens is
used by review orchestrators like any other specialist lens.

We draw explicit boundaries with adjacent lenses:

- **Architecture** retains system-level scalability (horizontal scaling,
  component failure, load behaviour). The performance lens assesses whether
  the *implementation* is efficient; architecture assesses whether the
  *system design* scales.
- **Code quality** retains observability infrastructure (structured logging,
  metrics, tracing design). The performance lens may note *what to measure*
  for performance, but does not own observability design.
- **Security** retains DoS-motivated evaluation ("can a malicious actor
  exhaust this?"). The performance lens handles general efficiency ("is this
  O(n²) when it could be O(n)?").

## Consequences

### Positive

- Performance gets dedicated, focused evaluation matching its importance in
  industry frameworks — reviews now catch a class of defects (N+1 queries,
  algorithmic hotspots, missing caching, concurrency hazards) that were
  previously invisible.
- Clean conceptual boundaries: code-level efficiency (performance),
  system-level scalability (architecture), and observability infrastructure
  (code quality) each have an explicit owner.
- Aligns the review system with widely-referenced code review frameworks,
  making the output more recognisable to reviewers who come from those
  traditions.

### Negative

- The boundary with architecture requires ongoing clarity — concerns can blur
  when system-scaling and code-efficiency questions interact (e.g., "is this
  a code-level query problem or a system-level data-model problem?").
- The security boundary is subtle: DoS-motivated efficiency concerns belong
  in security, general efficiency in performance. Reviewers must distinguish
  the *motivation* for the concern, not just the symptom.
- Observability infrastructure staying in code quality may surprise users who
  mentally associate "observability" with "performance". Needs to be
  signposted in the lens's What NOT to Do section.

### Neutral

- Performance concerns that were previously checked opportunistically across
  multiple lenses now have an explicit owner; existing lenses must update
  their "What NOT to Do" sections to defer those concerns.
- The lens is available to both PR and plan reviews (lens skills are
  context-agnostic per ADR-0005).

## References

- `meta/research/codebase/2026-02-22-review-lens-gap-analysis.md` — Gap analysis
  against Google, OWASP, and ISO 25010 frameworks; identifies performance as
  the primary coverage gap
- `meta/plans/2026-02-23-performance-lens-and-resilience-extension.md` —
  Implementation plan defining the performance lens scope and boundary
  statements
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` — Three-layer
  review architecture the lens plugs into
- `meta/decisions/ADR-0003-pbr-lens-design-with-structural-invariants.md` —
  Lens design principles the performance lens conforms to
- `meta/decisions/ADR-0005-single-generic-reviewer-agent-with-runtime-lens-injection.md` —
  Generic reviewer pattern that makes adding a lens a skill-only change
