---
adr_id: ADR-0013
date: "2026-04-17T13:48:51+00:00"
author: Toby Clemson
status: accepted
tags: [review-system, lenses, safety, iso-25010]
---

# ADR-0013: Standalone Safety Lens

**Date**: 2026-04-17
**Status**: Accepted
**Author**: Toby Clemson

## Context

The review system (ADR-0002, ADR-0003) uses specialist lenses selected per
review. A gap analysis against ISO 25010 quality characteristics (ADR-0004)
identified **Safety** as one of the coverage gaps: ISO 25010 treats Safety
(freedom from unacceptable risk of harm from accidental causes) as a quality
characteristic distinct from Security, but the existing catalogue had no lens
owning it.

The existing security lens is anchored on OWASP and STRIDE — adversarial threat
models concerned with malicious harm (injection, authentication bypass,
privilege escalation, disclosure). Accidental harm — data loss from buggy
migrations, cascading outages from missing safeguards, runaway batch jobs
exhausting resources, unsafe configuration defaults — is a different class of
failure. The two share vocabulary ("harm", "damage", "recovery") but are
evaluated through different reviewer mindsets: adversarial threat modelling vs
blast-radius and failure-mode analysis.

The question was whether to fold accidental-harm concerns into an existing
lens (security, or architecture — which had just absorbed resilience per
ADR-0012) or give them a standalone lens with its own reviewer persona.

## Decision Drivers

- **ISO 25010 coverage completeness**: Safety is a distinct quality
  characteristic; leaving it unowned means reviews are blind to a recognised
  class of defects
- **Distinct reviewer personas (PBR principle)**: accidental-harm evaluation
  (blast radius, failure modes, recovery paths) is a different analytical
  stance from adversarial threat modelling; bundling them degrades both
- **Security lens focus**: the security lens is anchored on OWASP/STRIDE and
  should stay focused on adversarial threats rather than broaden to include
  every form of harm
- **Architecture lens already broad**: the architecture lens has just absorbed
  resilience (ADR-0012) as a fourth Core Responsibility and is at its capacity
  for coherent focus

## Considered Options

1. **Fold accidental-harm concerns into the security lens** — Treat all
   harm-related review concerns as a single domain owned by one lens. Avoids
   adding to the catalogue and keeps "harm" evaluation in one place. Conflates
   two distinct reviewer mindsets — adversarial threat modelling and
   failure-mode analysis — which tend to push out each other's findings under
   finite attention per invocation; dilutes both the security lens's
   OWASP/STRIDE focus and the depth of accidental-harm coverage.

2. **Fold accidental-harm concerns into the architecture lens** — Treat
   safety as a design concern (blast radius, graceful degradation, fail-safe
   defaults are architectural choices). Natural adjacency to the resilience
   responsibility the architecture lens just absorbed (ADR-0012). Pushes the
   architecture lens past a coherent scope — it already holds scalability,
   modularity, coupling, and resilience; adding safety makes it a five-way
   grab bag and repeats the pattern ADR-0012 explicitly flagged as risky.

3. **Standalone safety lens** — A dedicated lens covering data safety
   (destructive-operation safeguards, corruption prevention), operational
   safety (blast radius, runaway-process prevention, resource exhaustion from
   legitimate use), and protective mechanisms (fail-safe defaults, recovery
   paths, kill switches). Gives accidental harm its own reviewer persona with
   clean boundaries against security (malicious vs accidental threat model)
   and architecture (strategic fitness of resilience patterns vs whether
   those patterns prevent harm in practice). Requires explicit boundary
   statements and adds one entry to the catalogue.

## Decision

We will add a standalone safety lens that evaluates protection against
accidental harm. Its scope covers three Core Responsibilities: **data safety**
(destructive-operation safeguards, corruption prevention from partial writes,
cascading delete containment, audit trails for irreversible operations);
**operational safety** (deployment safeguards, blast radius containment,
graceful degradation, runaway-process and resource-exhaustion prevention);
and **protective mechanisms and recovery paths** (fail-safe defaults, kill
switches, timeout enforcement, recovery capability).

We draw explicit boundaries with adjacent lenses:

- **Security** retains ownership of malicious harm — OWASP/STRIDE threat
  models, injection, authentication bypass, privilege escalation,
  disclosure, and DoS motivated by adversarial intent. The boundary is the
  *threat model*: accidental causes belong to safety, intentional adversarial
  causes belong to security.
- **Architecture** retains resilience *strategy* (ADR-0012) — is retry
  appropriate? is the backoff policy sound? is the circuit breaker placed
  correctly? Safety assesses whether those strategies actually prevent harm
  to users and data in practice (does the fail-safe default actually fail
  safe? is the kill switch reachable from the oncall runbook?).
- **Database** retains migration correctness and schema-design concerns.
  Safety assesses whether destructive data operations have the right
  safeguards regardless of whether the query itself is correct.
- **Performance** (ADR-0011) retains code-level efficiency under expected
  load — is this O(n²) when it could be O(n)? is the query N+1? is the
  cache hit rate acceptable? Safety assesses whether the same operation has
  guardrails against *abnormal* load or failure — if this received 100x
  expected input, would it exhaust memory or run unbounded? The boundary is
  the *motivation*: efficiency under expected conditions belongs to
  performance, runaway-process prevention and resource-exhaustion safeguards
  belong to safety.

## Consequences

### Positive

- Accidental-harm concerns get dedicated, focused evaluation — reviews now
  surface a class of defects (hard deletes without safeguards, missing
  fail-safe defaults, unbounded batch jobs, unreachable kill switches) that
  the security and architecture lenses were not structurally positioned to
  catch.
- The security lens keeps its OWASP/STRIDE focus intact; its reviewer
  persona stays adversarial rather than being pulled toward failure-mode
  analysis.
- The architecture lens is spared a fifth Core Responsibility on top of the
  resilience one it just absorbed (ADR-0012), preserving its coherent scope.
- Reviewer output aligns with ISO 25010's Safety characteristic, making
  findings recognisable to readers coming from that tradition.

### Negative

- The accidental-vs-adversarial boundary with security requires ongoing
  discipline — some concerns sit at the seam (resource exhaustion motivated
  by legitimate load vs adversarial intent, unsafe defaults, operations with
  both safety and security facets) and reviewers must label consistently or
  consumers will see duplicate or dropped findings.
- The strategy-vs-practice boundary with architecture (ADR-0012) adds a
  second subtle seam: whether a retry strategy is *appropriate* belongs to
  architecture, whether it *prevents harm in practice* belongs to safety.
  Easy to confuse under time pressure.
- One additional lens in the catalogue — contributing to the selection-cap
  pressure established in ADR-0004 (the orchestrator must now rank one more
  lens for relevance on every review).
- Every existing lens must add safety to its "What NOT to Do" list — the
  linear boundary-list maintenance cost ADR-0004 already flagged.

### Neutral

- The safety lens shares the same structural invariants as other lenses
  (ADR-0003) — six sections, conditional applicability sub-groups, severity
  tiers, and output format. No new lens-infrastructure patterns are
  introduced.
- Available to both PR and plan reviews via the generic reviewer agent
  (ADR-0005); no orchestrator-level changes beyond selection criteria.
- Auto-detect criteria favour changes involving data deletion/modification,
  deployment config, automated batch processes, infrastructure changes,
  feature flags, and critical system components — read-only and UI-only
  changes will typically skip the lens.

## References

- `meta/research/codebase/2026-02-22-review-lens-gap-analysis.md` — ISO 25010 gap
  analysis identifying Safety as an uncovered quality characteristic
- `meta/plans/2026-03-15-new-review-lenses.md` — Implementation plan (Phase 6)
  defining the safety lens scope, three Core Responsibilities, and boundary
  statements
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` — Three-layer
  review architecture the lens sits within
- `meta/decisions/ADR-0003-pbr-lens-design-with-structural-invariants.md` —
  Lens design principles the safety lens conforms to
- `meta/decisions/ADR-0004-lens-catalogue-expansion-with-bounded-selection-and-core-set.md` —
  Catalogue expansion decision that identified safety as one of six new
  lenses
- `meta/decisions/ADR-0005-single-generic-reviewer-agent-with-runtime-lens-injection.md` —
  Generic reviewer pattern that makes adding a lens a skill-only change
- `meta/decisions/ADR-0011-standalone-performance-lens.md` — Sibling decision;
  precedent for carving out a standalone lens with explicit boundaries
- `meta/decisions/ADR-0012-resilience-as-architecture-responsibility.md` —
  Sibling decision; defines the strategy-vs-practice boundary with this lens
