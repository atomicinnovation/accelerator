---
adr_id: ADR-0010
date: "2026-03-30T00:19:49+00:00"
author: Toby Clemson
status: accepted
tags: [review-system, github-api, inline-comments, posting-strategy, orchestration]
---

# ADR-0010: Atomic Review Posting via GitHub REST API

**Date**: 2026-03-30
**Status**: Accepted
**Author**: Toby Clemson

## Context

The review system's orchestrator (ADR-0002) aggregates findings from multiple
specialist agents into structured output (ADR-0006) and must post the results
to GitHub as a PR review. The `gh pr review` CLI command supports submitting a
review verdict and body but does not support inline diff comments — only
overall review text. Inline comments require the GitHub REST API endpoint
`POST /repos/{owner}/{repo}/pulls/{number}/reviews`, which accepts a review
body and an array of line-anchored comments in a single call.

Several forces shape the posting strategy. Posting the summary and inline
comments as separate API calls risks partial failure — an orphaned summary
with no inline comments, or comments without the contextual summary. The
`line` parameter in the API must reference a line visible in the diff; lines
outside any hunk cause a 422 validation error, requiring the orchestrator to
validate line references before posting. Agents typically produce more
findings than are useful as inline comments — excessive inline comments
overwhelm the PR author and reduce signal-to-noise. Critical findings must
always be visible inline regardless of volume constraints.

## Decision Drivers

- **Line-anchored precision**: Actionable findings should appear as inline
  comments on the specific diff lines they reference
- **Posting atomicity**: The entire review — summary and inline comments —
  should post as a single operation, avoiding orphaned or partial artifacts
- **Signal-to-noise ratio**: Excessive inline comments overwhelm the PR author
  and reduce review effectiveness
- **Critical finding visibility**: Critical findings must always appear as
  inline comments regardless of volume constraints
- **Graceful degradation**: Findings that fail line validation or exceed the
  comment cap should remain accessible in the review summary rather than being
  silently dropped

## Considered Options

For the posting mechanism:

1. **`gh pr review` CLI** — Simple invocation but no support for inline diff
   comments; only posts a review body and verdict
2. **Separate API calls** — Post the summary as a PR comment via
   `gh pr comment` and inline comments as a review via `gh api`. Allows the
   summary to appear as a standalone timeline comment but risks partial failure
   and orphaned artifacts.
3. **Single atomic `gh api` call** — Post the review body and inline comments
   together via `POST /repos/{owner}/{repo}/pulls/{number}/reviews`. Atomic,
   but requires managing commit SHAs and line validation, and the summary is
   collapsed under the review in GitHub's UI rather than appearing as a
   standalone timeline comment.

For comment volume management:

1. **No cap** — All findings become inline comments regardless of count. Risks
   overwhelming the PR author.
2. **Hard cap at N** — Drop all findings beyond N regardless of severity.
   Simple but may silently discard critical findings.
3. **Severity-prioritised cap with critical override** — Cap at ~10 inline
   comments, prioritised by severity (critical > major > minor > suggestion),
   with all critical findings always included even if that pushes the count
   beyond the cap. Overflow findings are included in the review summary body.

## Decision

We will use the GitHub REST API via `gh api` to post reviews as a single
atomic operation using `POST /repos/{owner}/{repo}/pulls/{number}/reviews`.
The review body carries the overall summary — cross-cutting themes, strengths,
general findings, and any overflow — while the `comments` array carries
line-anchored inline comments. The `commit_id` required by the API is obtained
by querying the PR's HEAD SHA via
`gh api repos/{owner}/{repo}/pulls/{number} --jq '.head.sha'` during the
orchestrator's data-fetching step and stored alongside the diff for use at
posting time. If the SHA becomes stale (the PR's HEAD advances between review
start and posting), the orchestrator re-fetches the SHA and offers to retry.

Inline comments are capped at ~10, prioritised by severity
(critical > major > minor > suggestion) and then by confidence
(high > medium > low). When severity and confidence are both equal, ordering
among those findings is undefined — the cap value and not the tie-break
determines which findings appear inline. All critical findings are always
included as inline comments even if that pushes the count beyond the cap.
Findings beyond the cap, and findings whose line references fail diff-hunk
validation, are included in the review summary body rather than silently
dropped.

## Consequences

### Positive

- Atomic posting eliminates the risk of orphaned summaries or partial reviews
- Inline comments appear on the exact diff lines they reference, matching human
  reviewer behaviour
- Critical findings are always visible inline regardless of volume
- Overflow and invalid findings remain accessible in the review summary rather
  than being lost

### Negative

- Direct REST API usage requires the orchestrator to manage commit SHAs and
  validate line references against diff hunks
- The review summary appears collapsed under the review in GitHub's UI rather
  than as a standalone timeline comment, reducing its visibility
- The ~10 comment cap means some actionable findings are less prominent,
  appearing only in the summary body

### Neutral

- The cap value and prioritisation logic live in the orchestrator and are
  adjustable without changing the posting mechanism
- The `event` field (APPROVE, COMMENT, REQUEST_CHANGES) is determined by the
  orchestrator based on finding severities but can be overridden by the user
  before posting

## References

- `meta/research/2026-02-22-pr-review-inline-comments.md` — API constraints,
  inline comment design, and severity prioritisation
- `meta/plans/2026-02-22-pr-review-inline-comments.md` — Implementation plan
  for atomic posting and comment capping
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` — Three-layer
  review system that this posting strategy serves
- `meta/decisions/ADR-0006-structured-agent-output-contract-with-context-specific-schemas.md`
  — Structured agent output consumed by the posting mechanism
