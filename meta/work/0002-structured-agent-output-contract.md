---
title: "Structured agent output contract with context-specific schemas"
type: adr-creation-task
status: done
---

# ADR Ticket: Structured agent output contract with context-specific schemas

## Summary

In the context of agent-to-orchestrator communication, facing the need for
reliable aggregation across PR and plan review contexts, we decided for
structured JSON with defined schemas that intentionally diverge between contexts
(PR agents use `path`/`line` with `comments`/`general_findings` split; plan
agents use `location` with a single `findings` array) to achieve both
programmatic reliability and schema fidelity to each context, accepting strict
output contracts and a fallback strategy for malformed output.

## Context and Forces

- Multiple parallel agents produce findings that must be aggregated by the
  orchestrator
- Findings need to be deduplicated, prioritised, and routed (inline comments vs
  summary for PRs; inline text for plans)
- PR findings can be anchored to specific diff lines; plan findings reference
  human-readable section names
- Free-form markdown output would require unreliable parsing for aggregation
- Agents may occasionally produce malformed output that the system must handle
  gracefully
- The temptation to use identical schemas for PR and plan agents would force
  unnecessary fields on one context or the other

## Decision Drivers

- Reliable programmatic aggregation and deduplication
- Schema fidelity: each context's schema should reflect its reality (diffs vs
  documents)
- Graceful degradation when agents produce malformed output
- Consistent enough across contexts that orchestrators can share aggregation
  logic where applicable

## Considered Options

1. **Free-form markdown** — Human-readable but unreliable for programmatic
   processing
2. **Single unified JSON schema** — Simpler but forces unnecessary fields on
   plan agents (no diff lines) or loses precision for PR agents
3. **Context-specific JSON schemas with shared conventions** — PR schema has
   `path`/`line`/`side`/`end_line` and `comments`/`general_findings`; plan
   schema has `location` and `findings`. Both share severity tiers, confidence
   ratings, and verdict logic.

## Decision

We will use structured JSON output with context-specific schemas. PR agents
produce `comments` (line-anchored) and `general_findings` (cross-cutting)
arrays with diff-precise location data. Plan agents produce a single `findings`
array with human-readable `location` references. Both share the same severity
tiers and confidence ratings. A 4-step malformed output extraction strategy
(find fence, extract, parse, fallback to general finding) handles agent errors.

## Consequences

### Positive
- Reliable programmatic aggregation, deduplication, and routing
- Each schema fits its context naturally — no unnecessary fields
- Shared conventions enable common severity/confidence logic
- Malformed output degrades gracefully rather than failing entirely

### Negative
- Agents must follow a strict output contract — any deviation requires the
  fallback strategy
- Orchestrators must handle slightly different schemas for PR vs plan contexts
- Two schemas to maintain and document

### Neutral
- The schema divergence is intentional and documented, not accidental drift

## Source References

- `meta/research/2026-02-22-pr-review-inline-comments.md` — PR output schema
  design and malformed output strategy
- `meta/research/2026-02-22-review-plan-pr-alignment.md` — Plan vs PR schema
  comparison and intentional divergence rationale
- `meta/plans/2026-02-22-review-plan-alignment.md` — Plan schema definition
