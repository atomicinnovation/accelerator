---
adr_id: ADR-0006
date: "2026-03-29T13:34:55+00:00"
author: Toby Clemson
status: accepted
tags: [review-system, agent-output, json-schema, structured-output, orchestration]
---

# ADR-0006: Structured Agent Output Contract with Context-Specific Schemas

**Date**: 2026-03-29
**Status**: Accepted
**Author**: Toby Clemson

## Context

The review system's three-layer architecture (ADR-0002) runs multiple specialist
agents in parallel, each producing findings that the orchestrator must aggregate,
deduplicate, prioritise, and utilise in different ways.

Several forces shape the output format decision. The orchestrator cannot perform
reliable aggregation over free-form markdown — it needs machine-parseable
structure with explicit fields for severity, confidence, location, and lens
attribution. PR findings anchor to specific diff lines and must split between
line-anchored comments (posted as inline GitHub review comments) and
cross-cutting observations (included in the review summary). Plan findings
reference human-readable section names and have no inline-vs-summary
distinction. A single unified schema would force unnecessary fields on one
context — `path`/`line`/`side` are meaningless for plans; a single `findings`
array loses the inline-vs-general separation that PR review requires. Meanwhile,
agents may occasionally produce malformed output, and the system must degrade
gracefully rather than fail entirely.

## Decision Drivers

- **Reliable programmatic aggregation**: the orchestrator must parse,
  deduplicate, and prioritise findings from multiple parallel agents without
  heuristic text extraction
- **Schema fidelity to context**: each review context has different location
  semantics (diff lines vs plan sections) and output routing (inline comments +
  summary vs single presentation) — the schema should reflect these realities
  rather than forcing a lowest-common-denominator structure
- **Graceful degradation**: agents may produce malformed output; the system must
  extract what it can and fall back rather than fail entirely
- **Shared conventions where meaningful**: severity tiers, confidence ratings,
  and verdict logic should be consistent across contexts so orchestrators can
  share aggregation logic where applicable

## Considered Options

1. **Free-form markdown** — Agents produce human-readable prose with severity
   headings and informal location references. Natural for LLM output and easy to
   read directly. However, extracting structured fields (severity, location,
   lens) requires unreliable regex or heuristic parsing, making programmatic
   aggregation and deduplication fragile.

2. **Single unified JSON schema** — One schema used by both PR and plan agents.
   Simpler to document and maintain. However, it either forces unnecessary
   fields on plan agents (`path`, `line`, `side`, `end_line`) or loses precision
   for PR agents (no inline-vs-general split). The schema would reflect neither
   context faithfully.

3. **Context-specific JSON schemas with shared conventions** — PR agents produce
   `comments` (line-anchored with `path`/`line`/`side`/`end_line`) and
   `general_findings` (cross-cutting) arrays. Plan agents produce a single
   `findings` array with human-readable `location` references. Both share
   severity tiers, confidence ratings, lens attribution, and verdict logic.

## Decision

We will use structured JSON output with context-specific schemas for
agent-to-orchestrator communication.

PR review agents produce two arrays: `comments` for line-anchored findings (with
`path`, `line`, `side`, and optional `end_line` for precise diff location) and
`general_findings` for cross-cutting observations that cannot be anchored to
specific lines. This split maps to the GitHub Reviews API, which accepts inline
comments alongside a review summary body — the orchestrator handles the
schema-to-API mapping (e.g., translating the agent's `line`/`end_line` to the
API's `start_line`/`line` convention).

Plan review agents produce a single `findings` array with a human-readable
`location` field referencing plan sections. There is no inline-vs-general
distinction because plan reviews are presented in-conversation, not posted as
inline comments.

Both schemas share the same severity tiers (`critical`, `major`, `minor`,
`suggestion`), confidence ratings (`high`, `medium`, `low`), verdict logic, and
a common finding structure: top-level `lens` identifier, `summary`, and
`strengths` fields, plus per-finding `severity`, `confidence`, `lens`
attribution, `title`, and a self-contained `body` following a shared format
(emoji prefix + lens name + issue + impact + suggestion). This enables
orchestrators to share aggregation, deduplication, and prioritisation logic
across contexts while each schema reflects its context naturally.

A four-step malformed output extraction strategy, implemented by the
orchestrator, handles agent errors: find a JSON code fence in the output,
extract the fenced content, attempt to parse it as JSON, and if all else fails,
wrap the raw output as a single general finding. This ensures the orchestrator
always has something to work with.

## Consequences

### Positive

- Reliable programmatic aggregation, deduplication, and prioritisation — no
  heuristic text parsing required
- Each schema fits its context naturally — PR schema maps directly to the GitHub
  Reviews API; plan schema reflects section-based plan structure
- Shared severity, confidence, and verdict conventions enable common aggregation
  logic across orchestrators
- Malformed output degrades gracefully to a single general finding rather than
  failing the review

### Negative

- Agents must follow a strict output contract — any deviation triggers the
  fallback strategy, which loses structural detail (severity, location, lens
  attribution)
- Orchestrators must handle two slightly different schemas — parsing logic
  branches on review context
- Two schemas to maintain and keep in sync where conventions are shared

### Neutral

- The schema divergence between PR and plan contexts is intentional and
  documented, not accidental drift — future maintainers should resist the
  temptation to unify them without revisiting the forces that drove the split

## References

- `meta/research/codebase/2026-02-22-pr-review-inline-comments.md` — PR output schema
  design, JSON format definition, and malformed output extraction strategy
- `meta/research/codebase/2026-02-22-review-plan-pr-alignment.md` — Plan vs PR schema
  comparison and intentional divergence rationale
- `meta/plans/2026-02-22-review-plan-alignment.md` — Plan schema definition
  with `location` field and single `findings` array
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` — Three-layer
  architecture that this output contract operates within
- `meta/decisions/ADR-0005-single-generic-reviewer-agent-with-runtime-lens-injection.md`
  — Generic reviewer agent pattern; output format references are one of the
  three composed file types
