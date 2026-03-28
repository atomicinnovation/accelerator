---
adr_id: ADR-0001
date: "2026-03-28T12:05:37+00:00"
author: Toby Clemson
status: accepted
tags: [context-engineering, agents, filesystem, token-management, architecture]
---

# ADR-0001: Use Filesystem Communication, Agent Separation, and Token Budgets for Context Isolation

**Date**: 2026-03-28
**Status**: Accepted
**Author**: Toby Clemson

## Context

Claude Code's context window is approximately 200k tokens, but quality degrades
significantly as fill increases, with notable decline above ~120k tokens.
Frontier models follow roughly 150-200 instructions before compliance drops, and
Claude Code's system prompt consumes approximately 50 of those slots. While
recent models offer context windows up to 1M tokens, quality remains highest
when active context is kept well below capacity. The specific thresholds are
tunable but the principle of a soft ceiling persists.

The accelerator plugin runs multi-phase workflows (research -> plan ->
implement) where each phase may execute in a fresh context window. Skills and
agents cannot access another skill's conversation history -- there is no
implicit state sharing between phases or sessions.

A single agent performing both broad codebase search and deep file reading
exhausts its context window: the search results alone can consume significant
token budget before any file content is loaded. This creates a tension between
breadth of investigation and depth of analysis within a single agent invocation.

The plugin is designed for collaborative, cross-team use where different people
may run different phases. Any state that exists only in a conversation is
invisible to other team members and lost across sessions.

## Decision Drivers

- **Consistent quality**: active work should stay below the context degradation
  threshold (~120k tokens) to maintain agent effectiveness
- **Session survivability**: state must persist across fresh context windows and
  session restarts
- **Institutional memory**: past sessions should educate future sessions through
  auditable, codebase-specific artifacts that capture reasoning and context
  beyond what the code itself records
- **Bounded agent context**: no single agent should need to hold both a broad
  search space and deep file content simultaneously
- **Explicit communication**: no hidden dependencies between phases -- all
  inter-phase data must be explicitly serialized
- **Team visibility**: outputs from any workflow phase should be accessible to
  other team members, not locked in a single person's session

## Considered Options

**For inter-phase communication:**

1. **Context continuation** -- keep the conversation going across phases,
   relying on the context window to hold all prior state. Simple but exceeds
   token limits for complex multi-phase work.
2. **External state store** -- use a database or API as the communication
   channel between phases. Durable and queryable but over-engineered for a
   code-focused tool where the filesystem is already the primary medium.
3. **Filesystem via `meta/` directory** -- each phase reads from and writes to
   predictable paths, producing structured artifacts with YAML frontmatter.
   Natural for a code-focused tool; survives sessions; version-controllable;
   visible to the whole team.

**For agent context management:**

1. **Monolithic agents** -- a single agent both searches the codebase and reads
   file contents. Simple to orchestrate but leads to context overflow on broad
   searches, as search results consume budget before any deep analysis begins.
2. **Locator/analyser separation** -- locators (Grep, Glob, LS only) find
   relevant files without reading contents. Analysers (adding Read) examine a
   focused set of findings. Bounds context per agent at the cost of multiple
   agent spawns per investigation.

**For token budget discipline:**

1. **No explicit budget** -- rely on auto-compaction when context fills. Risks
   quality degradation before compaction triggers and gives no predictable
   quality guarantee.
2. **Soft ceiling with instruction cap** -- target ~120k tokens for active work
   and cap CLAUDE.md at ~150 instructions. Provides a predictable quality zone
   while remaining tunable as models evolve.

## Decision

We will use the filesystem (`meta/` directory) as the sole inter-phase
communication channel. Every phase reads from and writes to predictable paths,
producing structured markdown artifacts with machine-parseable metadata (YAML
frontmatter) that serve as institutional memory -- capturing reasoning,
tradeoffs, and context beyond what the code records. These artifacts are
version-controlled and shared across the team.

We will separate agents into locators and analysers. Locators (Grep, Glob,
LS -- no Read tool) perform bounded search across the codebase or document tree.
Analysers (adding Read) examine a focused set of files identified by locators.
This two-phase pattern ensures no single agent needs to hold both a broad search
space and deep file content.

We will maintain a soft token ceiling for active work and an instruction cap for
CLAUDE.md. Current guidelines target ~120k tokens and ~150 instructions
respectively, calibrated against a 200k-token window. These thresholds are
tunable as model capabilities evolve -- the principle is proportional restraint,
not a fixed number.

## Consequences

### Positive

- Clean context boundaries between phases -- each phase starts fresh and reads
  only what it needs from disk
- State survives session restarts, context window resets, and team member
  handoffs
- Institutional memory accumulates naturally as artifacts capture reasoning,
  tradeoffs, and rejected alternatives alongside decisions
- Locator agents never overflow from deep file reads; analyser agents receive
  focused file lists rather than searching broadly
- Token ceiling prevents quality degradation and encourages disciplined context
  usage
- Artifacts are version-controllable, diffable, and auditable through standard
  git workflows

### Negative

- All inter-phase data must be explicitly serialized to disk -- there is no
  implicit state sharing
- Multiple agent spawns per investigation (locator then analyser) adds latency
  compared to a single monolithic agent
- The soft ceiling limits the amount of information that can be loaded at any
  one time, requiring progressive disclosure
- Skills must be designed around filesystem contracts, adding coupling to
  directory structure and artifact formats

### Neutral

- The filesystem approach is natural for a code-focused tool and aligns with
  existing developer workflows
- The ~120k token ceiling and ~150 instruction cap are tunable guidelines, not
  hard limits -- they will evolve with model capabilities
- The locator/analyser separation mirrors established patterns in information
  retrieval (index then fetch)

## References

- `meta/research/2026-03-15-context-management-approaches.md` -- Token budget
  analysis, locator/analyser pattern, filesystem communication design, and
  combined strategy prioritisation
- `meta/research/2026-03-18-meta-management-strategy.md` --
  Filesystem-as-shared-memory principle, meta/ directory as inter-phase
  communication channel, and artifact persistence gaps
- `meta/plans/2026-03-15-readme-restructure.md` -- Documents the filesystem
  communication principle as core to the plugin's architecture
- [Advanced Context Engineering for Coding Agents (AI That Works)](https://github.com/ai-that-works/ai-that-works/tree/main/2025-08-05-advanced-context-engineering-for-coding-agents)
  -- Impact multiplier hierarchy, three-phase workflow, and subagent isolation
  patterns that informed this decision
