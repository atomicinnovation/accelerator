---
title: "GitHub review posting strategy"
type: adr-creation-task
status: todo
---

# ADR Ticket: GitHub review posting strategy

## Summary

In the context of posting review findings to GitHub PRs, facing the constraint
that `gh pr review` doesn't support inline comments, we decided for direct REST
API calls via `gh api` with a single atomic `POST reviews` call (summary +
inline comments together), capped at ~10 inline comments prioritized by severity
with critical always included, to achieve precise line-anchored feedback with
atomicity and focused actionable output, accepting API complexity and that minor
findings are deferred to the summary.

## Context and Forces

- The `gh pr review` CLI command does not support inline diff comments
- The GitHub REST API `POST /repos/{owner}/{repo}/pulls/{number}/reviews`
  supports both a review body and an array of inline comments in a single call
- Posting summary and comments separately risks orphaned summaries if the
  comments call fails
- Excessive inline comments overwhelm the PR author and reduce signal-to-noise
- Critical findings should always be visible inline regardless of count limits
- The `line` parameter must reference a line visible in the diff or the API
  returns 422

## Decision Drivers

- Precise, line-anchored feedback for actionable findings
- Atomicity: either the entire review posts or nothing does
- Focused output: avoid comment spam that reduces review effectiveness
- Critical findings must never be dropped

## Considered Options

1. **`gh pr review`** — Simple CLI but no inline comment support
2. **Separate API calls** — Post summary and comments independently. Risk of
   partial failures and orphaned artifacts.
3. **Single atomic `gh api` call** — POST reviews with body + comments array.
   Atomic, precise, but requires managing commit SHAs and line validation.

For comment limits:
1. **No cap** — All findings become inline comments. Risks spam.
2. **Hard cap at N** — Drop all beyond N regardless of severity.
3. **Severity-prioritized cap with critical override** — Cap at ~10, prioritized
   by critical > major > minor, with critical always included even if exceeding
   the cap.

## Decision

We will use direct GitHub REST API calls via `gh api` to post reviews as a
single atomic operation. Inline comments are capped at approximately 10,
prioritized by severity (critical > major > minor), with all critical findings
always included. Findings beyond the cap are included in the review summary
body.

## Consequences

### Positive
- Atomic posting: no orphaned summaries or partial reviews
- Precise line-anchored feedback for the most important findings
- Critical findings are always visible inline
- Remaining findings are still accessible in the summary

### Negative
- Direct API usage requires managing commit SHAs and line validation
- The summary body in GitHub's UI is collapsed under the review rather than
  appearing as a standalone timeline comment
- The ~10 cap means some actionable findings are less visible

### Neutral
- The cap is configurable and the prioritization logic lives in the orchestrator

## Source References

- `meta/research/2026-02-22-pr-review-inline-comments.md` — API constraints,
  inline comment design, and severity prioritization
- `meta/plans/2026-02-22-pr-review-inline-comments.md` — Implementation of
  atomic posting and comment cap
