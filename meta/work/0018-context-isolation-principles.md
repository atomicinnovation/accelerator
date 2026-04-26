---
title: "Context isolation: filesystem communication, agent separation, and token budgets"
type: adr-creation-task
status: done
---

# ADR Ticket: Context isolation: filesystem communication, agent separation, and token budgets

## Summary

In the context of a 200k-token context window with quality degradation at high
fill, we decided for the filesystem (`meta/` directory) as the sole inter-phase
communication channel, a two-phase agent pattern (locators without Read,
analysers with Read) for bounded context per agent, and a soft ceiling of ~120k
tokens for active work, to achieve clean context boundaries, session-survivable
state, and consistent agent quality, accepting explicit serialization, multiple
agent spawns, and limits on loaded information.

## Context and Forces

- Claude Code context window is approximately 200k tokens, but quality degrades
  significantly as context fills
- Multi-phase workflows (research → plan → implement) run across fresh context
  windows — no implicit state sharing
- A single agent doing both broad search and deep file reading exhausts its
  context window
- The CLAUDE.md system prompt consumes roughly 50 instruction slots
- Skills and agents cannot access another skill's conversation history
- All inter-phase data must be explicitly serialized to survive session
  boundaries

## Decision Drivers

- Consistent quality: stay below degradation threshold (~120k tokens)
- Session survivability: state must persist across fresh context windows
- Bounded agent context: no single agent should hold both broad search space
  and deep file content
- Explicit communication: no hidden dependencies between phases

## Considered Options

For inter-phase communication:
1. **Context continuation** — Keep conversation going across phases. Exceeds
   token limits for complex work.
2. **Database/API** — External state store. Over-engineered for the use case.
3. **Filesystem** — `meta/` directory with structured subdirectories. Natural
   for a code-focused tool; survives sessions; version-controllable.

For agent design:
1. **Monolithic agents** — One agent searches and reads. Context overflow on
   broad searches.
2. **Locator/analyser separation** — Locators (Grep, Glob, LS only) find
   relevant files. Analysers (adding Read) examine findings. Bounded context
   per agent.

## Decision

We will use the filesystem (`meta/` directory) as the sole inter-phase
communication channel: every phase reads from and writes to predictable paths,
producing machine-parseable artifacts (YAML frontmatter, JSON schemas). We will
separate agents into locators (no Read tool — bounded search) and analysers
(with Read — bounded examination). We will target ~120k tokens for active work
and cap CLAUDE.md at ~150 instructions.

## Consequences

### Positive
- Clean context boundaries between phases
- State survives session restarts and context window resets
- Locator agents never overflow from deep file reads
- Analyser agents receive focused file lists rather than searching broadly
- Token ceiling prevents quality degradation

### Negative
- All inter-phase data must be explicitly serialized to disk
- Multiple agent spawns per investigation (locator + analyser)
- Limits on information loaded at any one time
- Progressive disclosure adds latency (skills load on demand)

### Neutral
- The filesystem approach is natural for a code-focused tool and artifacts are
  version-controllable
- The 120k ceiling and 150-instruction cap are tunable guidelines, not hard
  limits

## Source References

- `meta/research/2026-03-15-context-management-approaches.md` — Token budget
  analysis, locator/analyser pattern, and filesystem communication design
- `meta/plans/2026-03-15-readme-restructure.md` — Documents the filesystem
  communication principle as core to the plugin's architecture
