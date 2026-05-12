---
title: "Diff data pipeline: shared storage, hybrid access, and validation"
type: adr-creation-task
status: done
---

# ADR Ticket: Diff data pipeline: shared storage, hybrid access, and validation

## Summary

In the context of delivering PR data to multiple parallel agents and ensuring
valid API submissions, we decided for a shared temp directory with full diff and
changed-files list, where agents use judgment to prioritize files and the
orchestrator validates line numbers against parsed hunk headers, to achieve
single-fetch efficiency, practical scalability for large PRs, and reliable
GitHub API submissions, accepting file-system coupling and diff parsing
complexity.

## Context and Forces

- Multiple agents need access to the same PR diff data simultaneously
- Embedding the full diff in each agent's prompt would duplicate large content
  across parallel context windows
- Large PRs may contain more diff content than a single agent can practically
  process
- The GitHub Reviews API requires line numbers that reference lines visible in
  the diff — invalid line references return 422 errors
- Agents may reference lines outside diff hunks (hallucination or imprecision)
- A single invalid line reference in a review payload causes the entire review
  submission to fail

## Decision Drivers

- Single-fetch efficiency: the orchestrator fetches PR data once
- Consistent data across all agents (no stale or divergent copies)
- Practical scalability for large PRs without agent context overflow
- Reliable GitHub API submissions (zero 422 errors from invalid line references)

## Considered Options

1. **Inline in prompt** — Embed full diff in each agent's prompt. Simple but
   duplicates large content and doesn't scale.
2. **Shared temp directory** — Write diff artifacts to
   `/tmp/pr-review-{number}-XXXXXXXX` containing `diff.patch`,
   `changed-files.txt`, `pr-description.md`, and `commits.txt`. Agents read
   from shared location.
3. **API re-fetch per agent** — Each agent fetches its own data from GitHub.
   Consistent but slow and rate-limit-sensitive.

For validation:
1. **Agent-only validation** — Trust agents to provide correct line numbers.
   Simple but unreliable.
2. **Orchestrator-only validation** — Don't ask agents for line numbers. Loses
   inline precision.
3. **Hybrid validation** — Agents try to anchor to precise diff lines;
   orchestrator validates against parsed hunk headers and moves invalid
   references to general findings.

## Decision

We will use a shared temp directory for PR data delivery. Agents receive both
the full diff and a changed-files list and use their judgment to prioritize
files relevant to their lens (hybrid access). For validation, agents provide
their best-effort line references and the orchestrator validates all references
against parsed diff hunk headers, moving invalid references to general findings
rather than failing.

## Consequences

### Positive
- Single fetch, single write, multiple readers — efficient and consistent
- Agents can scale to large PRs by prioritizing relevant files
- Hybrid validation provides defense-in-depth: agent imprecision is caught
  before API submission
- No 422 errors from invalid line references

### Negative
- File-system coupling between orchestrator and agents
- Diff parsing in the orchestrator adds complexity
- Non-deterministic file prioritization means agents may skip relevant files in
  very large PRs

### Neutral
- The temp directory is ephemeral and cleaned up after review completion

## Source References

- `meta/research/codebase/2026-02-22-pr-review-agents-design.md` — Shared temp directory
  design and hybrid diff access strategy
- `meta/research/codebase/2026-02-22-pr-review-inline-comments.md` — Diff-line
  validation problem and hybrid validation approach
- `meta/plans/2026-02-22-pr-review-inline-comments.md` — Validation
  implementation details
