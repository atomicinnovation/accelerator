---
adr_id: ADR-0003
date: "2026-03-28T14:16:02+00:00"
author: Toby Clemson
status: accepted
tags: [review-lenses, perspective-based-reading, pbr, lens-design, structural-invariants, boundaries]
---

# ADR-0003: Perspective-Based Reading (PBR) with Structural Invariants for Lens Design

**Date**: 2026-03-28
**Status**: Accepted
**Author**: Toby Clemson

## Context

The accelerator plugin's review system runs multiple specialist lenses in
parallel, each evaluating code changes or plans from a focused quality
dimension. ADR-0002 established the three-layer architecture (specialist,
orchestrator, convention) that enables this parallel execution. With 13 lenses
now in the catalogue and the expectation of further growth, the system needs
a principled foundation for how individual lenses are designed — what each lens
contains, how large it should be, and how lenses avoid duplicating or
contradicting each other's findings.

Research into software inspection techniques shows that unfocused review finds
fewer defects than structured, perspective-based approaches. Perspective-Based
Reading (PBR), developed by Basili and Shull at the University of Maryland,
demonstrates that reviewers who adopt a specific stakeholder perspective and
attempt to produce a derivative artifact (attack scenarios, failure modes, test
cases) achieve significantly higher defect detection rates than those using
checklists or unstructured approaches. Different perspectives find different
defect types — the value is in the combination.

Several forces shape how this principle applies to AI review lenses. Multiple
lenses running concurrently risk producing duplicate, contradictory, or
scope-creeping findings unless boundaries are explicit. Lenses need consistent
sizing — too broad and they lose focus, too narrow and the lens count explodes.
When new lenses are added, they often take ownership of concerns previously
handled by existing lenses (as occurred when the documentation lens absorbed
documentation concerns from the standards lens, and the database lens absorbed
query performance from the performance lens). Without a defined protocol for
these ownership transfers, boundary drift accumulates silently.

## Decision Drivers

- **Detection effectiveness**: PBR produces higher detection rates for defects,
  weak assumptions, and implementation gaps than checklist-based or unstructured
  approaches — each perspective surfaces issues that others miss
- **Non-overlapping coverage**: lenses running in parallel must not produce
  duplicate or contradictory findings
- **Consistent sizing**: every lens should be comparable in scope — broad enough
  to be worthwhile, narrow enough to maintain focus
- **Maintainable boundaries**: boundary definitions must be practical to update
  as the lens catalogue evolves
- **Independent comprehensibility**: each lens must be self-contained and
  understandable without reading other lenses

## Considered Options

1. **Checklist-based lenses** — Each lens carries a flat list of items to check.
   Simple to author and easy to understand. However, checklists encourage
   passive scanning rather than active probing, miss emergent issues that don't
   map to a predefined item, and provide no natural sizing heuristic — lists
   tend to grow unboundedly.

2. **Unstructured expert lenses** — Each lens contains free-form instructions
   written by a domain expert. Maximally flexible and can encode deep domain
   nuance. However, lenses end up inconsistently sized, overlap-prone (no
   structural mechanism to prevent it), and difficult to maintain as the
   catalogue grows — each lens is a snowflake.

3. **PBR-based lenses with structural invariants** — Each lens embodies a
   stakeholder perspective and uses generative questions ("What happens when X
   fails?", "Can an attacker Y?") to actively probe rather than passively scan.
   Structural invariants enforce consistency: 3–4 numbered responsibility
   groups per lens (a sizing heuristic — more means split, fewer means too
   narrow), mandatory boundary statements in two locations (inline notes and a
   "What NOT to Do" section), and explicit concern ownership transfers when new
   lenses are added. More prescriptive than the alternatives, requiring lens
   authors to understand PBR principles and maintain boundary lists across the
   catalogue.

## Decision

We will use Perspective-Based Reading (PBR) as the theoretical foundation for
all lens design. Each lens adopts a focused stakeholder perspective and attempts
to produce a derivative artifact rather than passively scanning against a
checklist. For example, the security lens constructs attack scenarios, the
safety lens enumerates failure modes, the performance lens identifies bottleneck
analyses, and the test-coverage lens derives test cases.

Each lens must have 3–4 numbered responsibility groups, each containing 4–8
sub-items. This invariant serves as a sizing heuristic: a lens needing more
than 4 groups should be split; fewer than 3 indicates the lens is too narrow or
under-specified.

Every lens must include explicit boundary statements in two locations: inline
boundary notes after responsibilities that border another lens's domain, and a
"What NOT to Do" section that names all other lenses and any domain-specific
anti-patterns. Where boundaries are ambiguous, a concrete clarifying example
must be included (e.g., security-motivated DoS evaluation stays in the security
lens while general algorithmic efficiency belongs to the performance lens).

When a new lens is introduced that takes ownership of concerns previously
handled by an existing lens, the previous owner must explicitly relinquish
those concerns — updating its responsibilities, boundary notes, and "What NOT
to Do" section to reflect the transfer. This was practised when the
documentation lens absorbed documentation concerns from the standards lens and
the database lens absorbed query performance from the performance lens.

## Consequences

### Positive

- Higher detection effectiveness from generative, perspective-based review —
  each lens actively probes from a stakeholder viewpoint rather than passively
  scanning
- Non-overlapping coverage through explicit boundary statements in two
  locations, reducing duplicate and contradictory findings across parallel
  lenses
- Consistent sizing via the 3–4 responsibility group invariant, with a clear
  signal for when a lens should be split or is too narrow
- Clean single-ownership of each review concern, with a defined protocol for
  transferring ownership when new lenses are added

### Negative

- Every lens's boundary list must be updated when a new lens is added —
  maintenance cost scales linearly with the catalogue size
- Lens authors must understand PBR principles (perspective adoption, generative
  questions), not just domain expertise
- Ownership transfers require modifying existing lens files, creating a
  coordination cost when the catalogue changes

### Neutral

- The 3–4 group sizing aligns with cognitive chunking research — it is a
  pragmatic heuristic, not a theoretically derived limit
- Boundary statements must describe observable code characteristics rather than
  intent, keeping them verifiable by both humans and AI agents

## References

- `meta/research/2026-03-15-review-lens-optimal-structure.md` — PBR foundation,
  boundary statement design, structural invariants, and optimal lens template
- `meta/plans/2026-03-15-new-review-lenses.md` — Concern ownership transfers in
  practice (documentation from standards, query performance from performance,
  breaking changes from usability)
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` — Three-layer
  architecture that this ADR's lens design principles operate within
- `meta/decisions/ADR-0001-context-isolation-principles.md` — Context isolation
  principles underpinning the independent, parallel execution model that
  motivates boundary enforcement
- [How Perspective-Based Reading Can Improve Requirements Inspections](https://ieeexplore.ieee.org/document/869376/)
  (Basili et al., IEEE) — Original PBR methodology demonstrating that
  structured perspective adoption improves inspection effectiveness
- [The Empirical Investigation of Perspective-Based Reading](https://link.springer.com/article/10.1007/BF00368702)
  (Basili & Shull, Springer) — Empirical validation that different perspectives
  find different defect types, with significantly better coverage than
  unfocused review
