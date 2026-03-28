---
adr_id: ADR-0007
date: "2026-03-29T21:09:03+00:00"
author: Toby Clemson
status: accepted
tags: [review-system, verdicts, plan-review, pr-review, thresholds]
---

# ADR-0007: Divergent Verdict Semantics for Plan and PR Reviews

**Date**: 2026-03-29
**Status**: Accepted
**Author**: Toby Clemson

## Context

The review system (ADR-0002) evaluates both implementation plans and pull
requests across multiple quality lenses, with each specialist agent producing
structured findings that include a severity tier and confidence rating
(ADR-0006). The orchestrator for each review context must map aggregated
findings to a verdict — a summary judgement that signals whether the reviewed
artefact is ready to proceed, needs revision, or warrants only advisory
commentary.

PR reviews and plan reviews operate in fundamentally different cost
environments. A PR review verdict of `REQUEST_CHANGES` creates a GitHub change
request that blocks merging — the author must revise code, update the PR, and
re-request review, a high-cost cycle. A plan review verdict requesting revision
results in an in-conversation edit to a plan document — a low-cost action that
can happen immediately within the same session.

This cost asymmetry creates tension when defining verdict thresholds. A
threshold calibrated for the high cost of PR rework (escalating only on
critical findings) would under-trigger plan revisions, allowing plans with
multiple major concerns to proceed to implementation where rework is far more
expensive. A threshold calibrated for the low cost of plan edits (escalating on
any critical or several major findings) would over-trigger PR change requests,
creating unnecessary friction for code authors.

## Decision Drivers

- **Rework cost asymmetry**: plan edits are near-zero cost (in-conversation),
  while PR rework involves code changes, CI cycles, and re-review — verdict
  thresholds should reflect this difference
- **Early defect detection**: plans are the cheapest point to catch design
  issues — aggressive revision at this stage prevents expensive rework later
- **Categorical consistency**: using the same verdict categories
  (`APPROVE`/`X`/`COMMENT`) across both contexts keeps the system conceptually
  coherent and allows shared orchestrator logic where applicable
- **Deterministic verdict logic**: verdict determination should be mechanical
  (based on finding counts and severities), not a subjective judgement by the
  orchestrator

## Considered Options

1. **Identical verdicts** — Use `APPROVE`/`REQUEST_CHANGES`/`COMMENT` with the
   same thresholds for both plan and PR reviews. Simplest approach — one verdict
   system to document and implement. However, thresholds calibrated for the high
   cost of PR rework would under-trigger plan revisions, allowing plans with
   multiple major concerns to pass through to implementation unchallenged.

2. **Divergent verdicts** — Use `APPROVE`/`REVISE`/`COMMENT` for plan reviews
   with a lower escalation threshold, while PR reviews retain
   `APPROVE`/`REQUEST_CHANGES`/`COMMENT` with a higher threshold. `REVISE`
   triggers on any critical finding or 3+ major findings; `REQUEST_CHANGES`
   triggers only on critical findings. Reflects the cost asymmetry directly in
   both the vocabulary and the thresholds.

3. **No verdicts for plans** — Plan reviews produce only advisory commentary
   with no formal verdict. Avoids the threshold question entirely. However, this
   loses the structured quality gate signal — there is no machine-readable
   indication of whether a plan needs rework, making it harder for orchestrators
   to drive automated iteration workflows.

## Decision

We will use divergent verdict semantics for plan and PR reviews, with both the
vocabulary and thresholds tailored to each context's rework cost.

Plan reviews use `APPROVE`/`REVISE`/`COMMENT`. The `REVISE` verdict triggers
when findings include any critical severity or 3+ major severity findings. The
`REVISE` label was chosen over `REQUEST_CHANGES` to reflect that plan revision
is a lightweight, in-conversation action rather than a formal GitHub change
request.

PR reviews use `APPROVE`/`REQUEST_CHANGES`/`COMMENT`. The `REQUEST_CHANGES`
verdict triggers only on critical findings. The higher threshold reflects the
greater cost of PR rework — code changes, CI cycles, and re-review.

The `APPROVE` and `COMMENT` verdicts share the same semantics across both
contexts: `APPROVE` applies when there are no findings at all (only
strengths); `COMMENT` applies when findings exist but fall below the
escalation threshold. Only the escalation boundary diverges — `REVISE` for
plans, `REQUEST_CHANGES` for PRs — with different thresholds as described
above.

The deliberate asymmetry means plans are revised more aggressively, catching
design issues at the cheapest possible point. The three verdict categories
remain consistent across both contexts — only the escalation labels and
thresholds diverge.

## Consequences

### Positive

- Plans are revised more aggressively, catching design issues before code is
  written when rework is cheapest
- The lower plan threshold is proportionate to the near-zero cost of
  in-conversation plan edits
- Consistent verdict categories across both contexts keep orchestrator
  aggregation logic conceptually aligned

### Negative

- Divergent verdict labels and thresholds require clear documentation —
  contributors must understand that `REVISE` and `REQUEST_CHANGES` are
  context-specific rather than interchangeable
- Orchestrator logic must handle both verdict sets, branching on review context
  when determining and acting on the verdict
- The PR threshold deliberately does not escalate on major findings — a PR with
  many major findings receives `COMMENT`, not `REQUEST_CHANGES`. This is a
  conscious acceptance that some significant concerns will be advisory-only in
  the PR context, reflecting the high cost of formal change requests

### Neutral

- The threshold values (critical-only for PR, critical-or-3+-major for plan)
  can be tuned independently for each context without affecting the other

## References

- `meta/plans/2026-02-22-review-plan-alignment.md` — Verdict logic definition
  and threshold rationale in the "What We're NOT Doing" section
- `meta/research/2026-02-22-review-plan-pr-alignment.md` — Research comparing
  PR and plan review patterns, including output format and verdict divergence
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` — Three-layer
  architecture that both review contexts operate within
- `meta/decisions/ADR-0006-structured-agent-output-contract-with-context-specific-schemas.md`
  — Output schemas that carry verdict fields and shared severity/confidence
  conventions
