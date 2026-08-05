---
title: Agents
description: Reference for the nine subagents Accelerator skills spawn, and
  the locator/analyser split behind them.
---

Accelerator skills delegate exploratory work to subagents defined in
`agents/*.md`. Each agent runs in an isolated context, does its reading or
searching there, and returns only a summary — keeping the main
conversation's context small. See [Philosophy](../philosophy.md) for why.

## The locator/analyser split

Most agents come in pairs across three domains — codebase, documents, and
browser. **Locators** answer *where*: they enumerate files, documents,
routes, or screens, and deliberately have **no Read tool** (or, for the
browser, take no screenshots), so they cannot be tempted into deep
analysis and their context stays bounded. **Analysers** answer *how*: they
read (or inspect) a focused set of targets in depth and return distilled
findings.

| Domain    | Locator (WHERE)     | Analyser (HOW)       |
| --------- | ------------------- | -------------------- |
| Codebase  | `codebase-locator`  | `codebase-analyser`  |
| Documents | `documents-locator` | `documents-analyser` |
| Browser   | `browser-locator`   | `browser-analyser`   |

Orchestrating skills typically fan out locators first, then spawn
analysers on the narrowed target list.

## Codebase agents

### codebase-locator

Finds WHERE code lives. Searches for files, directories, and components
relevant to a feature or task and organises the results by purpose —
without analysing their contents.

**Tools:** Grep, Glob, LS (no Read)

### codebase-analyser

Explains HOW code works. Reads specific files, traces data flow and
method calls, and reports implementation details with precise
`file:line` references.

**Tools:** Read, Grep, Glob, LS

### codebase-pattern-finder

A hybrid: locates similar implementations, usage examples, and
established patterns, and — unlike the locator — returns concrete code
extracts that can serve as templates for new work.

**Tools:** Grep, Glob, Read, LS

## Documents agents

### documents-locator

Discovers relevant documents in the configured document directories
(`meta/` by default) and categorises them, without deep content
analysis. Preloads the `paths` skill so it searches the directories the
project actually configures.

**Tools:** Grep, Glob, LS (no Read)

### documents-analyser

Extracts high-value insights from specific documents: decisions,
conclusions, actionable recommendations, constraints. Filters
aggressively so only relevant material returns to the caller.

**Tools:** Read, Grep, Glob, LS

## Browser agents

Both browser agents drive a running web application through the
Playwright executor (`run.sh`), whose path is injected via the preloaded
`browser-executor` skill. They are used by the
[design convergence](../skills/design-convergence.md) workflow.

### browser-locator

Enumerates WHERE things appear in the rendered UI: routes, screens, and
DOM-level component presence, using navigation and accessibility-tree
snapshots — no content analysis, state extraction, or screenshots.

**Tools:** Bash (Playwright executor only)

### browser-analyser

Captures HOW a focused set of screens behaves: state variants (loading,
empty, error, success), interactions, screenshots, and computed style
and layout values.

**Tools:** Bash (Playwright executor only)

## Other agents

### reviewer

Generic review agent behind the multi-lens
[Review System](../skills/review-system.md). Review orchestrators
(`review-pr`, `review-plan`, `review-work-item`) spawn one instance per
lens, injecting the lens skill and output-format specification at spawn
time; the agent reads both, explores the artefact, and returns a
structured JSON review.

**Tools:** Read, Grep, Glob, LS

### web-search-researcher

Researches questions on the public web: analyses the query, searches,
fetches promising sources, and returns findings with citations. Used
when a task needs current information beyond the codebase.

**Tools:** WebSearch, WebFetch, TodoWrite, Read, Grep, Glob, LS
