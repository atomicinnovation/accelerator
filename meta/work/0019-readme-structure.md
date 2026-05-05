---
title: "README structured around philosophy and development loop"
type: adr-creation-task
status: ready
---

# ADR Ticket: README structured around philosophy and development loop

## Summary

In the context of a flat feature catalogue README, we decided to lead with
philosophy, development loop, and `meta/` directory explanation before listing
skills to achieve immediate understanding of architectural rationale, accepting
installation below the fold.

## Context and Forces

- The README was a flat alphabetical catalogue of skills and features
- The plugin's core value proposition (phased, context-efficient development)
  was invisible in this format
- New users need to understand the workflow philosophy before individual skills
  make sense
- The `meta/` directory is central to the plugin but was not explained
  prominently
- Installation instructions are important but secondary to understanding what
  the tool does

## Decision Drivers

- First-time user comprehension: understand the "why" before the "what"
- The phased workflow is the key differentiator, not the skill list
- The `meta/` directory pattern is essential context for using any skill
- README structure should mirror the plugin's conceptual architecture

## Considered Options

1. **Feature catalogue** — Alphabetical list of skills. Standard but obscures
   the workflow.
2. **Installation-first** — Lead with setup instructions. Practical but delays
   understanding.
3. **Philosophy-first** — Lead with philosophy, then development loop, then
   `meta/` directory, then skills. Builds understanding progressively.

## Decision

We will structure the README leading with the plugin's philosophy and
development loop (research → plan → implement → review), followed by the
`meta/` directory explanation, then individual skill documentation.
Installation instructions move below the fold.

## Consequences

### Positive
- Users immediately understand the workflow philosophy
- The `meta/` directory pattern is established before skills reference it
- README structure mirrors the actual user experience

### Negative
- Installation instructions are not immediately visible
- Longer path to "just tell me the commands"

### Neutral
- The structure codifies the plugin's conceptual architecture as its primary
  organizing principle

## Source References

- `meta/plans/2026-03-15-readme-restructure.md` — README restructure plan with
  section ordering rationale
