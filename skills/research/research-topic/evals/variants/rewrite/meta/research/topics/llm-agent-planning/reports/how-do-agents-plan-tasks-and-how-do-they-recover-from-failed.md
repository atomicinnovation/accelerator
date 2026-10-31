---
type: "topic-research"
id: "llm-agent-planning-report-how-do-agents-plan-tasks-and-how-do-they-recover-from-failed"
title: "How do agents plan tasks, and how do they recover from failed steps?"
date: "2026-10-30T00:00:00+00:00"
author: "Fixture Author"
producer: "research-topic"
status: "complete"
kind: "report"
question: "How do agents plan tasks, and how do they recover from failed steps?"
rounds_covered: 3
coverage: "partial"
parent: "work-item:0900"
relates_to: ["topic-research:llm-agent-planning-synthesis"]
tags: ["research", "topic-research", "report"]
last_updated: "2026-10-30T00:00:00+00:00"
last_updated_by: "Fixture Author"
schema_version: 1
---
# How do agents plan tasks, and how do they recover from failed steps?

## Answer
LLM agents plan a multi-step task by decomposing its goal into a hierarchy
of subtasks and planning each in turn, or by writing a plan of steps before
acting and carrying it out step by step. A planner proposes ordered steps,
an executor carries out each one, and the loop replans when a step's result
changes what the goal needs.

Agents recover from a failed step by retrying it, often with a
revised prompt, or by replanning from the failed step.

## Coverage
Partial.

- Planning multi-step tasks: Hierarchical Task Planning for Language Agents
  (tier-1), Plan-and-Solve Prompting for Language Agents (tier-1),
  Decomposition patterns for agent loops (tier-2)

## Gaps
- Recovering from failed steps: Retry loops in agent pipelines (tier-3),
  Recovering from tool failures (tier-3)

## Sources
- [Hierarchical Task Planning for Language Agents](https://doi.org/10.5555/htp.2025) — tier-1 — Journal of Agent Systems
- [Plan-and-Solve Prompting for Language Agents](https://openalex.org/W3001) — tier-1 — Association for Computational Linguistics
  - doi: 10.5555/plan-and-solve.2024
  - openalex_id: W3001
  - published: 2024-05-01
  - authors: Ada Planner; Ben Solver
- [Decomposition patterns for agent loops](https://planning-guide.example/decomposition) — tier-2 — planning-guide.example
- [Recovering from tool failures](https://openalex.org/W3003) — tier-3 — Workshop on Agent Failures
- [Self-Correcting Planners](https://openalex.org/W3002) — tier-3 (retracted) — Journal of Agent Systems
- [Retry loops in agent pipelines](https://agent-blog.example/retries) — tier-3 — agent-blog.example
