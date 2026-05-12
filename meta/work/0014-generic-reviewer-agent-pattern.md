---
title: "Generic reviewer agent with path passing and three-way separation"
type: adr-creation-task
status: done
---

# ADR Ticket: Generic reviewer agent with path passing and three-way separation

## Summary

In the context of 12 duplicated review agents, we decided for 1 generic
reviewer agent that reads its lens skill at runtime via path passing, with a
three-way separation of concerns (lens skills hold domain expertise, output
format references hold JSON schema, orchestrator prompts hold context-specific
framing) to achieve single-source-of-truth for each lens, minimal orchestrator
context, and zero agent changes when adding lenses, accepting indirection of
three files composed at spawn time.

## Context and Forces

- The original system had 12 separate agent files (6 PR + 6 plan) with
  near-identical output format sections
- Updating format conventions required editing all 12 files
- Each lens read into the orchestrator's context consumed tokens that could be
  used for user interaction
- The main conversation context is the scarce resource; agent contexts are
  isolated and disposable
- Adding a new lens required creating 2 agent files (PR + plan) plus
  orchestrator updates

## Decision Drivers

- Single-source-of-truth: each lens defined once, not duplicated per context
- Minimal orchestrator context consumption (~12 lines vs ~800-1000)
- O(1) agent definitions regardless of lens count
- Clear separation of what to evaluate, how to format, and what context to apply

## Considered Options

1. **Per-lens agents** — One agent per lens per context. Simple but O(N)
   duplication.
2. **Content injection** — Orchestrator reads lens content and injects into
   agent prompt. Single agent but consumes orchestrator context.
3. **Path passing** — Orchestrator passes file paths to a generic agent; agent
   reads files in its own isolated context. Minimal orchestrator context; lens
   content stays in agent context.

## Decision

We will use 1 generic `reviewer` agent that receives file paths at spawn time
and reads them in its own context. Three concerns are separated into distinct
files: lens skills (domain expertise — what to evaluate), output format
references (JSON schema — how to structure output), and orchestrator task
prompts (context-specific framing — PR diff vs plan file). Adding a new lens
requires one new SKILL.md file plus minor orchestrator table updates — no agent
changes.

## Consequences

### Positive
- Single-source-of-truth for each lens: defined once, used in PR and plan
  review
- Adding a lens is one file + minor edits, not two agent files
- Orchestrator context stays minimal (paths, not content)
- Output format changes are a single-file edit per review type
- Agent count is fixed at 1 regardless of lens count

### Negative
- Three files must be composed at spawn time — indirection
- The agent must reliably read skill files before starting its review
- Debugging requires understanding the path → read → review chain

### Neutral
- The pattern makes the orchestrator simpler (pass paths and framing, not
  content) at the cost of agent-side file reading

## Source References

- `meta/research/codebase/2026-02-22-skills-agents-commands-refactoring.md` — Path
  passing vs content injection analysis
- `meta/plans/2026-02-22-skills-agents-commands-refactoring.md` —
  Implementation of generic reviewer
- `meta/research/codebase/2026-03-15-review-lens-optimal-structure.md` — Architecture
  insight confirming lenses as passive skills
