---
title: "Skills and agents as complementary abstractions"
type: adr-creation-task
status: ready
---

# ADR Ticket: Skills and agents as complementary abstractions

## Summary

In the context of Claude Code deprecating commands in v2.1.3 and skills gaining
`context: fork` and `allowed-tools`, we decided for migrating all commands to
skills while retaining agents as a distinct concept (agents provide system
prompt behavioural framing and capabilities like parallel spawning, permission
modes, worktree isolation; skills define knowledge and workflows) to achieve
future-proofing and clear separation of concerns, accepting two related
abstraction layers.

## Context and Forces

- Claude Code v2.1.3 deprecated commands in favour of skills
- Skills support directory-based structure, bundled files, auto-discovery,
  progressive disclosure, and dynamic context injection via preprocessor
- Commands were single-file, always-loaded, and had no supporting files
- Skills gained `context: fork` and `allowed-tools` in Claude Code 2.1, raising
  the question of whether agents are now redundant
- Agents provide capabilities that skills cannot: system prompt behavioural
  framing, disallowedTools, permissionMode, maxTurns, mcpServers, memory access,
  and worktree isolation
- An agent's body IS its system prompt (persistent behavioural framing); a
  forked skill's body becomes the task prompt (what to do, not how to behave)

## Decision Drivers

- Commands are deprecated: migration to skills is mandatory
- Skills and agents serve fundamentally different purposes (knowledge/workflow
  vs execution environment/behaviour)
- Agent-only capabilities cannot be replicated with skills
- The cognitive overhead of two abstractions is justified by their distinct roles

## Considered Options

1. **Skills only** — Migrate everything to skills, remove agents. Loses
   behavioural framing and agent-only capabilities.
2. **Agents only** — Keep agents, ignore skills. Against platform direction;
   loses progressive disclosure and bundled files.
3. **Skills + agents as complementary** — Skills define knowledge and workflows
   (user-invocable); agents define execution environments and behavioural roles
   (spawned by skills/orchestrators). Each has a distinct role.

## Decision

We will migrate all 8 commands to skills and retain all 7 agents as a
complementary concept. Skills are the primary user-facing abstraction (invoked
via `/accelerator:skill-name`). Agents are the execution-environment abstraction
(spawned by skills and orchestrators with specific behavioural framing). The
critical distinction: an agent's body is its system prompt; a forked skill's
body is its task prompt.

## Consequences

### Positive
- Future-proofed against command deprecation
- Skills provide progressive disclosure, bundled files, and preprocessor
- Agents retain behavioural framing and exclusive capabilities
- Clear separation: skills = knowledge/workflow, agents = behaviour/environment

### Negative
- Two related abstraction layers to understand and maintain
- Contributors must know when to create a skill vs an agent
- Potential confusion between `context: fork` skills and agents

### Neutral
- The distinction becomes clearer with usage: "Is this user-invocable knowledge?
  → Skill. Is this a spawnable execution environment? → Agent."

## Source References

- `meta/research/2026-02-22-skills-agents-commands-refactoring.md` — Full
  analysis of commands, skills, and agents
