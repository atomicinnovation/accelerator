---
date: "2026-05-05 16:16:48 CEST"
researcher: jonassvalin
git_commit: a2d9c593ca9950596e4a6d53d3089245487372ba
branch: main
repository: accelerator
topic: "Design research for a dedicated debug-issue skill"
tags: [research, skill-design, debugging, production-issues]
status: complete
last_updated: "2026-05-05"
last_updated_by: jonassvalin
---

# Research: Design for a Dedicated Debug-Issue Skill

**Date**: 2026-05-05 16:16:48 CEST
**Researcher**: jonassvalin
**Git Commit**: a2d9c593ca9950596e4a6d53d3089245487372ba
**Branch**: main
**Repository**: accelerator

## Research Question

Should a dedicated debugging/investigation skill be created, separate from `research-codebase`? What would it look like, and how would it handle the spectrum from clear stacktraces to vague behavioral issues?

## Summary

A dedicated `debug-issue` skill provides meaningful benefit over using `research-codebase` for production debugging. The key differentiator is a **hypothesis-driven investigation flow** with adaptive input parsing, compared to `research-codebase`'s exploratory breadth-first approach. The skill should handle a spectrum from structured inputs (stacktraces, logs) to loose behavioral descriptions ("sometimes X causes Y").

## Detailed Findings

### Why a Separate Skill Is Warranted

`research-codebase` is optimised for **understanding code** (architecture, patterns, how things work). Debugging requires **diagnosing failures** (why something broke, what state led to the error, where the fault lies). These have different cognitive shapes:

| Dimension | `research-codebase` | `debug-issue` |
|-----------|---------------------|---------------|
| Input parsing | Treats all input as a "question" | Structured extraction of errors, timestamps, request IDs, affected services |
| Decomposition | "What areas of the codebase are relevant?" | "What is the causal chain from trigger to failure?" |
| Agent orchestration | Locators first, then analysers | Stacktrace→file mapping first, then code path analysis, then "has this broken before?" |
| Approach | Exploratory, breadth-first | Hypothesis-driven: generate → test → eliminate |
| Output | Research document (findings, architecture insights) | Root cause analysis (timeline, causal chain, root cause, fix options, prevention) |
| Temporal awareness | Static code analysis | Correlates timestamps across logs/breadcrumbs |

### Proposed Workflow

#### Step 1: Extract and Classify Input

The skill accepts whatever production context the user provides — from a full Sentry event to a single sentence. It adapts its investigation depth accordingly.

| Input type | Step 1 produces |
|---|---|
| Stacktrace + logs | Exact file:line references, error type, timeline |
| Vague behavior ("sometimes X causes Y") | Affected action, expected vs actual, reproduction conditions, involved systems |

For vague/intermittent issues, the "sometimes" keyword signals the skill to look for:
- Race conditions / timing dependencies
- State that varies between runs (caches, feature flags, session state)
- Non-deterministic code paths (random, time-based, order-dependent)

#### Step 2: Map to Code

- **Structured input**: Map stacktrace frames directly to current source files (accounting for line drift)
- **Vague input**: Find the code path for the described action across all involved repos

#### Step 3: Check Recent Changes

Run `git log` on affected files to identify if something recently changed. This is particularly relevant for "it used to work" scenarios.

#### Step 4: Form Hypotheses

Based on error type + code context, generate 2-3 theories. The less structured the input, the more this step leans on exploratory code analysis before forming hypotheses.

#### Step 5: Investigate in Parallel

Spawn agents to test each hypothesis against the code. Same agent types as `research-codebase` (codebase-locator, codebase-analyser, codebase-pattern-finder).

#### Step 6: Synthesise — Root Cause Analysis

Output a structured RCA document rather than a general research document.

#### Step 7: Suggest Fix

Optionally feed into `create-plan` or propose a direct fix for simple cases.

### Cross-Repository Investigation

When debugging involves multiple services (e.g., this repo + `../my-backend-service`):

- **Use local paths for code investigation** — the codebase agents all operate on local files and provide deep tracing capability
- **Use `gh` CLI as a supplement** for:
  - Recent PRs/commits on the other repo ("was this recently changed?")
  - Checking if there's an open issue about the same behavior
  - Deployment history via releases/tags
- **Fallback**: If the user doesn't have the other repo checked out, the skill can use `gh api` for basic code search but should flag that investigation will be shallower and suggest cloning

The skill prompt should instruct: "If the user references other repositories, use their local paths for code investigation. Optionally use `gh` to check recent changes, deployments, or related issues on those repos."

### What to Keep the Same as `research-codebase`

- Same config infrastructure (`config-read-context.sh`, `config-read-agents.sh`, etc.)
- Same agent types (codebase-locator, codebase-analyser, codebase-pattern-finder)
- Same output location (`meta/research/` — debugging findings are still research)
- Same follow-up pattern (feeds into `create-plan` → `implement-plan`)

### Suggested File Structure

```
skills/research/debug-issue/
├── SKILL.md
└── scripts/
    └── debug-metadata.sh
```

### Output Format: Root Cause Analysis

The output document should flex based on input clarity:
- **Clear error**: Full RCA with timeline, causal chain, root cause, contributing factors, fix options (with risk/effort), prevention recommendations
- **Vague issue**: Investigation notes + most likely cause + suggested next steps

## Architecture Insights

- The skill fits naturally in `skills/research/` alongside `research-codebase` — it's a specialised form of research
- The existing agent infrastructure (locator → analyser pattern) maps well to debugging
- The hypothesis-driven workflow is the core differentiator — it provides cognitive structure that `research-codebase` doesn't enforce
- Cross-repo support via local paths keeps the full agent tooling available; `gh` supplements but doesn't replace

## Open Questions

- Should the skill have a "reproduce" step that attempts to identify or suggest reproduction steps?
- Should it integrate with observability tools (if MCP servers are configured) for pulling logs/metrics directly?
- What template should the RCA output use? A simpler version of the research template, or something purpose-built?
