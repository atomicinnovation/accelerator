---
adr_id: ADR-0008
date: "2026-03-30T00:00:00+00:00"
author: Toby Clemson
status: accepted
tags: [pr-review, diff-pipeline, data-delivery, orchestration]
---

# ADR-0008: Shared Temp Directory for PR Diff Delivery

**Date**: 2026-03-30
**Status**: Accepted
**Author**: Toby Clemson

## Context

The PR review system (ADR-0002) spawns multiple specialist agents in parallel,
each evaluating the same pull request through a different quality lens. These
agents need access to the PR diff, changed file list, PR description, and commit
messages.

Several forces shape the data delivery design. Embedding the full diff in each
agent's prompt would duplicate potentially large content across parallel context
windows. Large PRs may contain more diff content than a single agent can
practically process within its context budget. The orchestrator already fetches
PR data as part of its coordination role, so the delivery mechanism should
leverage that single fetch rather than having agents fetch independently.

## Decision Drivers

- **Single-fetch efficiency**: the orchestrator fetches PR data once and writes
  it once; all agents read from the same source
- **Consistent data across agents**: no stale or divergent copies from
  independent fetches
- **Practical scalability for large PRs**: agents can prioritise relevant files
  without context overflow
- **Minimal prompt bloat**: diff content stays out of agent prompts, preserving
  context budget for analysis

## Considered Options

1. **Inline in prompt** — Embed the full diff and metadata in each agent's spawn
   prompt. Simplest approach with no external dependencies. However, it
   duplicates large content across parallel context windows and doesn't scale —
   large PRs can exceed an agent's context budget before analysis begins.

2. **Shared temp directory** — The orchestrator writes diff artifacts to a temp
   directory and passes the path to each agent. Agents read what they need.
   Single write, multiple readers. Agents receive both the full diff and a
   changed-files list and use judgment to prioritise files relevant to their lens
   for large PRs (hybrid access).

3. **Independent API fetch per agent** — Each agent fetches its own PR data from
   GitHub. Produces consistent data per agent but multiplies API calls by the
   number of lenses, is rate-limit-sensitive, and wastes time re-fetching
   identical data.

## Decision

We will use a shared temp directory for PR data delivery. The orchestrator
fetches PR data once and writes it to a randomised temp directory
(`/tmp/pr-review-{number}-XXXXXXXX`) containing:

- `diff.patch` — full PR diff
- `changed-files.txt` — list of changed file paths
- `pr-description.md` — the PR description body
- `commits.txt` — commit message headlines

The temp directory path is passed to each agent in its spawn prompt. Agents use
`Read` to access the files and `Read`, `Grep`, `Glob` to explore the actual
codebase for context beyond the diff.

Additional orchestrator-specific files (e.g., `head-sha.txt`, `repo-info.txt`)
may be written to the same directory for downstream integration needs such as
the GitHub Reviews API. These are outside the scope of this decision, which
covers the shared review input data only.

For small PRs, agents read the full diff. For large PRs, agents use the
changed-files list as a table of contents and prioritise reading files most
relevant to their lens directly from the codebase, rather than consuming the
entire diff. Agent prompts include guidance for this prioritisation.

The temp directory is ephemeral — created at review start and cleaned up after
the review completes.

## Consequences

### Positive

- Single fetch, single write, multiple readers — efficient and consistent across
  all agents
- Agents can scale to large PRs by prioritising files relevant to their lens
  rather than consuming the entire diff
- Diff content stays out of agent prompts, preserving context budget for
  analysis
- Adding new lenses requires no changes to the data delivery mechanism — new
  agents read from the same shared directory

### Negative

- File-system coupling between orchestrator and agents — agents must know the
  temp directory structure and file naming conventions
- Non-deterministic file prioritisation means agents may skip relevant files in
  very large PRs, potentially missing findings

### Neutral

- The temp directory is ephemeral and cleaned up after review completion — no
  persistent storage concerns

## References

- `meta/research/codebase/2026-02-22-pr-review-agents-design.md` — Shared temp directory
  design and hybrid diff access strategy
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` — Three-layer
  architecture that this pipeline serves
- `meta/decisions/ADR-0005-single-generic-reviewer-agent-with-runtime-lens-injection.md`
  — Path-passing pattern: orchestrator passes file paths, agents read in their
  own context
