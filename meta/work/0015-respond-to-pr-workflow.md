---
title: "Respond-to-PR as sequential workflow with verify-before-implement"
type: adr-creation-task
status: todo
---

# ADR Ticket: Respond-to-PR as sequential workflow with verify-before-implement

## Summary

In the context of designing a skill for responding to PR review feedback, we
decided for a sequential interactive workflow (no sub-agents) with a mandatory
verify-before-implement pattern (READ, UNDERSTAND, VERIFY, EVALUATE, RESPOND,
IMPLEMENT) and user confirmation at each step, using GraphQL's
`pullRequest.reviewThreads` as the primary data source, to achieve per-item
verification and technical correctness over social compliance, accepting slower
throughput and REST as supplementary for top-level reviews.

## Context and Forces

- Review feedback arrives as inline thread comments and top-level review bodies
  on GitHub PRs
- Blindly implementing reviewer suggestions risks introducing bugs when
  feedback is technically incorrect
- Feedback items are interdependent (one change may affect another) making
  parallel processing risky
- The user should retain final judgment on whether to accept or push back on
  each feedback item
- GraphQL's `pullRequest.reviewThreads` returns thread IDs directly usable in
  the `resolveReviewThread` mutation, avoiding REST/GraphQL ID correlation
- REST is still needed for top-level review bodies and issue comments outside
  the review thread model

## Decision Drivers

- Technical correctness over social compliance ("performative agreement")
- User control at each step (verify, confirm, implement)
- Sequential processing due to item interdependence
- Reliable thread resolution via GraphQL

## Considered Options

1. **Parallel sub-agents per feedback item** — Fast but items are
   interdependent and user loses per-item control
2. **Batch implementation** — Process all feedback at once. Fast but bypasses
   verification.
3. **Sequential verify-before-implement** — Per-item: read feedback, understand
   intent, verify against codebase, evaluate correctness, confirm with user,
   implement if appropriate, respond on GitHub, resolve thread.

## Decision

We will use a sequential workflow skill with no sub-agents. Each feedback item
follows: READ → UNDERSTAND → VERIFY (against codebase) → EVALUATE (is the
feedback technically correct?) → RESPOND (draft reply) → IMPLEMENT (if
confirmed). The user confirms at each step. GraphQL is the primary data source
for review threads; REST supplements for top-level reviews. The AI should push
back when feedback is technically incorrect.

## Consequences

### Positive
- Technical correctness is verified before implementation
- User retains control at every step
- No risk of interdependent changes conflicting
- Thread resolution uses correct GraphQL IDs without correlation

### Negative
- Slower throughput — items processed one at a time
- More interactive: requires user attention at each step
- Two data sources (GraphQL primary, REST supplementary)

### Neutral
- Establishes that the orchestrator-with-sub-agents pattern is specifically for
  independent parallel evaluation, not universal
- The sequential pattern combines `implement-plan`'s code change approach with
  `review-pr`'s GitHub API patterns

## Source References

- `meta/research/2026-02-23-respond-to-pr-feedback-skill.md` —
  Verify-before-implement pattern and philosophical rationale
- `meta/plans/2026-02-23-respond-to-pr-skill.md` — GraphQL data source design
  and workflow implementation
