---
adr_id: ADR-0005
date: "2026-03-28T20:13:17+00:00"
author: Toby Clemson
status: accepted
tags: [review-system, agents, path-passing, separation-of-concerns, lens-injection]
---

# ADR-0005: Single Generic Reviewer Agent with Runtime Lens Injection

**Date**: 2026-03-28
**Status**: Accepted
**Author**: Toby Clemson

## Context

The accelerator plugin's review system runs 13 specialist lenses in parallel,
each evaluating code changes or plans from a focused quality dimension
(ADR-0002). Each lens runs as a separate agent instance, and findings flow
through orchestrators (`review-pr`, `review-plan`) that aggregate and synthesise
results.

The original implementation used 12 separate agent definition files — one per
lens per review type (6 PR + 6 plan). Each agent file contained near-identical
scaffolding: tool access declarations, output format sections, scoping
conventions, and behavioural instructions. Only the lens-specific domain
expertise varied. Updating format conventions required editing all 12 files.
Adding a new lens required creating 2 agent files (one for PR review, one for
plan review) plus orchestrator updates — an O(N) expansion.

Meanwhile, orchestrators that read lens content into their own prompts to compose
agent instructions consumed significant context — approximately 800–1000 tokens
per lens — reducing the context budget available for user interaction and
synthesis. The main conversation context is the scarce resource; agent contexts
are isolated and disposable.

## Decision Drivers

- **Single-source-of-truth**: each lens's domain expertise defined once, not
  duplicated per review context (PR vs. plan)
- **Minimal orchestrator context consumption**: orchestrators pass a compact
  task prompt (~30–37 lines of paths, context, and framing) rather than
  ~800–1000 lines of injected lens content
- **O(1) agent definitions**: a fixed number of agent definitions regardless of
  how many lenses exist in the catalogue
- **Clear separation of responsibilities**: what to evaluate (lens skill), how
  to format output (output format reference), and what context to apply
  (orchestrator task prompt) are distinct, independently maintainable concerns

## Considered Options

1. **Per-lens agents** — One agent definition per lens per review context. Each
   agent encodes its own domain expertise, output format, and scoping
   conventions. Simple and self-contained, but O(N) duplication: every shared
   concern must be maintained across all agent files. Adding a lens requires 2
   new agent files plus orchestrator updates.

2. **Content injection** — Orchestrator reads lens skill content and injects it
   into a single generic agent's prompt at spawn time. Achieves a single agent
   definition, but the lens content is loaded into the orchestrator's context
   window, consuming ~800–1000 tokens per lens and reducing the budget available
   for user interaction and synthesis.

3. **Path passing** — Orchestrator passes file paths to a single generic agent;
   the agent reads lens skills and output format files in its own isolated
   context window. Minimal orchestrator context consumption; lens content stays
   in the agent's disposable context. Requires the agent to reliably read files
   before starting its review.

## Decision

We will use a single generic `reviewer` agent that receives file paths at spawn
time and reads its lens skill and output format specification in its own isolated
context. Three concerns are separated into distinct file types:

- **Lens skills** (`skills/review/lenses/*/SKILL.md`) — domain expertise
  defining what to evaluate and the perspective-specific criteria (as established
  by ADR-0003)
- **Output format references** (`skills/review/output-formats/*/SKILL.md`) —
  JSON schema and structural conventions defining how to format findings for a
  given review type
- **Orchestrator task prompts** (inline in
  `skills/github/review-pr/SKILL.md`,
  `skills/planning/review-plan/SKILL.md`) — context-specific framing including
  PR artefact paths, analysis strategy, and the instruction to read lens and
  output format files

The orchestrator composes a compact task prompt (~30–37 lines of paths, context,
and framing), then spawns the reviewer agent. The agent reads the referenced
files in its own context window and executes the review. Adding a new lens
requires one new SKILL.md file and a table entry in the orchestrator — no agent
definition changes.

## Consequences

### Positive

- Single-source-of-truth for each lens: defined once in one SKILL.md, used in
  both PR and plan review
- Adding a lens is one file plus minor orchestrator table edits, not two agent
  files
- Orchestrator context stays minimal — paths and framing, not lens content
- Output format changes are a single-file edit per review type, applied to all
  lenses
- Agent count is fixed at 1 regardless of lens count — no agent definition
  proliferation

### Negative

- Three files must be composed at spawn time — understanding how a review works
  requires tracing the path, read, review chain across orchestrator, agent, lens
  skill, and output format
- If a file path were incorrect or a file missing, the review would degrade
  silently rather than fail fast
- Debugging requires understanding three layers of indirection: orchestrator
  prompt composition, agent file reading, and lens-specific evaluation

### Neutral

- The pattern makes orchestrators simpler (pass paths and framing, not content)
  at the cost of agent-side file reading — a shift in where complexity lives
  rather than a net reduction

## References

- `meta/research/codebase/2026-02-22-skills-agents-commands-refactoring.md` — Path
  passing vs. content injection analysis
- `meta/plans/2026-02-22-skills-agents-commands-refactoring.md` —
  Implementation of generic reviewer with three-way separation
- `meta/research/codebase/2026-03-15-review-lens-optimal-structure.md` — Architecture
  insight confirming lenses as passive skills read at runtime
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` — Three-layer
  architecture that this ADR operates within
- `meta/decisions/ADR-0003-pbr-lens-design-with-structural-invariants.md` —
  Lens design principles governing the lens skills this pattern composes
