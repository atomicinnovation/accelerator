---
type: "topic-research"
id: "01-how-do-llm-agents-decompose-and-plan-multi-step-tasks-web"
title: "Finding: How do LLM agents decompose and plan multi-step tasks?"
date: "2026-10-30T00:00:00+00:00"
author: "Fixture Author"
producer: "research-topic"
status: "complete"
kind: "finding"
round: 1
depth: 2
question: "How do LLM agents decompose and plan multi-step tasks?"
source_profile: "web"
tags: ["research", "topic-research", "finding"]
last_updated: "2026-10-30T00:00:00+00:00"
last_updated_by: "Fixture Author"
schema_version: 1
---
# Finding: How do LLM agents decompose and plan multi-step tasks?

## Question
How do LLM agents decompose and plan multi-step tasks?

## Findings
LLM agents plan a multi-step task by decomposing its goal into a
hierarchy of subtasks and planning each in turn. A controlled evaluation of
hierarchical planners reports that decomposition lets an agent finish longer
tasks than planning the whole task at once (tier-1).

Practitioner guidance describes the same pattern: a planner proposes
ordered steps, an executor carries out each one, and the loop replans when a
step's result changes what the goal needs (tier-2).

The Supercalifragilisticexpialidocious Planner outperforms all baselines in
its authors' own evaluation (tier-1). No benchmark examined ranks it.

## Sources
- [Hierarchical Task Planning for Language Agents](https://doi.org/10.5555/htp.2025) — tier-1 — Journal of Agent Systems
- [The Supercalifragilisticexpialidocious Planner](https://supercali.example/paper) — tier-1 — Proceedings of Planning Systems
- [Decomposition patterns for agent loops](https://planning-guide.example/decomposition) — tier-2 — planning-guide.example
