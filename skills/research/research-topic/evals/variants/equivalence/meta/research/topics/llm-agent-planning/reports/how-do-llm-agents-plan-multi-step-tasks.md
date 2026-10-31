---
type: "topic-research"
id: "llm-agent-planning-report-how-do-llm-agents-plan-multi-step-tasks"
title: "How do LLM agents plan multi-step tasks?"
date: "2026-10-30T00:00:00+00:00"
author: "Fixture Author"
producer: "research-topic"
status: "complete"
kind: "report"
question: "How do LLM agents plan multi-step tasks?"
rounds_covered: 3
coverage: "answered"
parent: "work-item:0900"
relates_to: ["topic-research:llm-agent-planning-synthesis"]
tags: ["research", "topic-research", "report"]
last_updated: "2026-10-30T00:00:00+00:00"
last_updated_by: "Fixture Author"
schema_version: 1
---
# How do LLM agents plan multi-step tasks?

## Answer
LLM agents plan a multi-step task by decomposing its goal into a hierarchy
of subtasks and planning each in turn, or by writing a plan of steps before
acting and carrying it out step by step. A planner proposes ordered steps,
an executor carries out each one, and the loop replans when a step's result
changes what the goal needs.

## Coverage
Answered.

- Planning multi-step tasks: Hierarchical Task Planning for Language Agents
  (tier-1), Plan-and-Solve Prompting for Language Agents (tier-1),
  Decomposition patterns for agent loops (tier-2)

## Sources
- [Hierarchical Task Planning for Language Agents](https://doi.org/10.5555/htp.2025) — tier-1 — Journal of Agent Systems
- [Plan-and-Solve Prompting for Language Agents](https://openalex.org/W3001) — tier-1 — Association for Computational Linguistics
  - doi: 10.5555/plan-and-solve.2024
  - openalex_id: W3001
  - published: 2024-05-01
  - authors: Ada Planner; Ben Solver
- [Decomposition patterns for agent loops](https://planning-guide.example/decomposition) — tier-2 — planning-guide.example
- [Self-Correcting Planners](https://openalex.org/W3002) — tier-3 (retracted) — Journal of Agent Systems
