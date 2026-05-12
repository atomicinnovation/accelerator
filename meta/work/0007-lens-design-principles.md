---
title: "Lens design principles: PBR, boundaries, and structural invariants"
type: adr-creation-task
status: done
---

# ADR Ticket: Lens design principles: PBR, boundaries, and structural invariants

## Summary

In the context of designing AI review lenses for parallel execution, we decided
for Perspective-Based Reading as the theoretical foundation (each lens adopts a
stakeholder perspective and generates derivative artifacts), with mandatory
explicit boundary statements, 3-4 numbered responsibility groups per lens, and
explicit concern ownership transfers when adding new lenses, to achieve high
defect detection, non-overlapping coverage, and a clear sizing heuristic,
accepting maintenance cost of updating boundaries when the lens set changes.

## Context and Forces

- Research on software inspection techniques shows that unfocused review finds
  fewer defects than structured, perspective-based approaches
- Multiple AI lenses running concurrently risk producing duplicate,
  contradictory, or scope-creeping findings
- Lenses need to be sized consistently — too broad and they lose focus, too
  narrow and the lens count explodes
- When new lenses are added, existing lenses may need to relinquish concerns to
  the specialist
- Without explicit boundaries, agents will drift into each other's territory

## Decision Drivers

- Defect detection effectiveness (PBR produces higher detection rates than
  checklist-based approaches)
- Non-overlapping coverage across parallel execution
- Consistent sizing across all lenses
- Maintainable boundary definitions as the lens set evolves
- Each lens must be self-contained and independently understandable

## Considered Options

1. **Checklist-based lenses** — Each lens has a list of things to check. Simple
   but passive — misses emergent issues.
2. **Unstructured expert lenses** — Free-form instructions per lens. Flexible
   but inconsistent sizing and overlap-prone.
3. **PBR-based lenses with structural invariants** — Each lens embodies a
   stakeholder perspective, uses generative questions ("What happens when X
   fails?"), has 3-4 responsibility groups with 4-8 sub-items each, includes
   boundary statements (inline notes + "What NOT to Do" section), and triggers
   ownership transfers when new lenses are added.

## Decision

We will use Perspective-Based Reading as the theoretical foundation for all
lens design. Each lens adopts a focused stakeholder perspective and attempts to
produce a derivative artifact. Lenses must have 3-4 numbered responsibility
groups (any lens needing more should be split). Each lens includes mandatory
boundary statements in two locations: inline boundary notes after
responsibilities, and a "What NOT to Do" section listing all other lenses. When
a new specialized lens is introduced, the previous owner of those concerns must
explicitly relinquish them.

## Consequences

### Positive
- Higher defect detection from generative, perspective-based review
- Non-overlapping coverage through explicit boundary statements
- Consistent sizing via the 3-4 group invariant
- Clean single-ownership of each review concern
- Clear criteria for when a lens is too broad (needs splitting)

### Negative
- Every lens's boundary list must be updated when a new lens is added
- Lens authors must understand PBR principles, not just domain expertise
- Ownership transfers require modifying existing lens files

### Neutral
- The 3-4 group sizing aligns with cognitive chunking research
- Boundary conditions must describe observable code characteristics, not intent

## Source References

- `meta/research/codebase/2026-03-15-review-lens-optimal-structure.md` — PBR foundation,
  boundary statements, structural invariants
- `meta/plans/2026-03-15-new-review-lenses.md` — Concern ownership transfer
  principle and practice
