---
type: "topic-research"
id: "llm-agent-planning-synthesis"
title: "Synthesis: LLM Agent Planning"
date: "2026-10-30T00:00:00+00:00"
author: "Fixture Author"
producer: "research-topic"
status: "complete"
kind: "synthesis"
rounds_covered: 4
parent: "work-item:0900"
tags: ["research", "topic-research", "synthesis"]
last_updated: "2026-10-30T00:00:00+00:00"
last_updated_by: "Fixture Author"
schema_version: 1
---
# Synthesis: LLM Agent Planning

## Overview
LLM agents plan multi-step tasks by decomposition, and the
evidence for how they plan is stronger than the evidence for how they
recover when a step fails.

## Findings
LLM agents plan multi-step tasks by decomposing a goal into subtasks and
planning each in turn, or by writing a plan of steps before acting and
carrying it out step by step (tier-1). Practitioner guidance describes the
same planner and executor loop (tier-2). Orchestra and Relay document a
planning API (tier-1). Context limits degrade an agent's plan as its
context grows towards the model's window (tier-1). The Agent Planner
Leaderboard ranks hierarchical planners first, a ranking that rests on a
source cited at tier-3 (also tier-1).

Agents recover from a failed step by reflecting on its error, revising
their plan and retrying from that step (tier-1). Retries and replanning
are also described at tier-3.

## Open Threads
No source examined compares the hardware that trains agent models.
The benchmark ranking of planners rests on a source cited at tier-3
(also tier-1).

## Sources
- [Hierarchical Task Planning for Language Agents](https://doi.org/10.5555/htp.2025) — tier-1 — Journal of Agent Systems
- [The Supercalifragilisticexpialidocious Planner](https://supercali.example/paper) — tier-1 — Proceedings of Planning Systems
- [Plan-and-Solve Prompting for Language Agents](https://openalex.org/W3001) — tier-1 — Association for Computational Linguistics
  - doi: 10.5555/plan-and-solve.2024
  - openalex_id: W3001
  - published: 2024-05-01
  - authors: Ada Planner; Ben Solver
- [Reflexive recovery for language agents](https://openalex.org/W3007) — tier-1 — Journal of Agent Systems
- [Long-context degradation in language agents](https://openalex.org/W3005) — tier-1 — Transactions on Machine Learning Research
- [Orchestra planning API](https://orchestra.example/docs/planning) — tier-1 — orchestra.example
- [Relay planner hooks](https://relay.example/docs/hooks) — tier-1 — relay.example
- [Context limits in agent loops](https://context-study.example/limits) — tier-1 — context-study.example
- [Decomposition patterns for agent loops](https://planning-guide.example/decomposition) — tier-2 — planning-guide.example
- [Agent planner leaderboard](https://openalex.org/W3004) — tier-3 (also tier-1) — Journal of Agent Benchmarks
  - doi: 10.5555/leaderboard.2026
  - openalex_id: W3004
- [Recovering from tool failures](https://openalex.org/W3003) — tier-3 — Workshop on Agent Failures
- [Self-Correcting Planners](https://openalex.org/W3002) — tier-3 (retracted) — Journal of Agent Systems
- [Retry loops in agent pipelines](https://agent-blog.example/retries) — tier-3 — agent-blog.example
