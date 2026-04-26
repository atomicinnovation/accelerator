---
title: "Safety vs security as distinct lenses"
type: adr-creation-task
status: done
---

# ADR Ticket: Safety vs security as distinct lenses

## Summary

In the context of covering ISO 25010's Safety characteristic, we decided for a
separate safety lens (accidental harm: data loss, operational outages, cascading
failures) distinct from the existing security lens (malicious harm: injection,
privilege escalation) to achieve deeper coverage of each concern and clearer
reviewer personas, accepting careful boundary statements.

## Context and Forces

- ISO 25010 distinguishes Safety (freedom from unacceptable risk of harm) as a
  separate quality characteristic from Security
- The existing security lens focuses on malicious threats: injection, XSS,
  authentication bypass, privilege escalation
- Accidental harm (data loss from bugs, cascading failures, operational outages)
  is a distinct failure mode not well covered by the security lens
- Folding safety into security would either dilute the security lens's focus or
  give safety concerns insufficient attention
- PBR theory suggests distinct reviewer personas for distinct failure modes

## Decision Drivers

- ISO 25010 coverage completeness
- Distinct failure modes deserve distinct perspectives (PBR principle)
- Security lens should remain focused on adversarial threats
- Accidental harm needs its own evaluation framework

## Considered Options

1. **Fold into security lens** — Keep one lens for all harm-related concerns.
   Dilutes both perspectives.
2. **Fold into architecture lens** — Safety as a design concern. Architecture
   lens is already broad with resilience added.
3. **Standalone safety lens** — Separate lens for accidental harm with clear
   boundary: safety = accidental harm, security = malicious harm.

## Decision

We will create a separate safety lens focused on accidental harm: data loss
from bugs, operational outages, cascading failures, unsafe defaults, and
resource exhaustion from legitimate use. The security lens retains ownership of
malicious harm: injection, privilege escalation, authentication bypass, and
adversarial exploitation. The boundary is accidental vs malicious threat models.

## Consequences

### Positive
- Deeper coverage of both accidental and malicious harm
- Clearer reviewer personas aligned with PBR principles
- ISO 25010 Safety characteristic properly addressed

### Negative
- Careful boundary statements needed between safety and security
- One additional lens in the catalogue (contributing to the move from 7 to 13)
- Some concerns (e.g., resource exhaustion) could be claimed by either lens

### Neutral
- Both lenses share the same severity tiers and output format

## Source References

- `meta/plans/2026-03-15-new-review-lenses.md` — Safety lens design and
  boundary definitions with security
