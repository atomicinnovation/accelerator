---
title: "Plan review verdict semantics"
type: adr-creation-task
status: done
---

# ADR Ticket: Plan review verdict semantics

## Summary

In the context of plan review verdict logic, facing the difference that plan
edits are cheap compared to code changes, we decided for
`APPROVE`/`REVISE`/`COMMENT` with a lower threshold for revision (any critical
OR 3+ major findings) to achieve earlier rework, accepting divergent verdict
semantics from PR review's `APPROVE`/`REQUEST_CHANGES`/`COMMENT`.

## Context and Forces

- PR reviews result in GitHub change requests that block merging — a high-cost
  action for the author
- Plan reviews result in in-conversation edits — a low-cost action
- The asymmetry in rework cost means different thresholds are appropriate
- Using the same thresholds for both would either over-trigger PR change
  requests or under-trigger plan revisions
- Plan quality benefits from aggressive revision since catching issues early
  (before code) is cheaper than catching them later

## Decision Drivers

- Rework cost asymmetry between plans (cheap to revise) and PRs (expensive to
  rework)
- Early defect detection: plans should be revised more aggressively
- Consistency where possible: same verdict categories, different thresholds
- Clear, deterministic verdict logic

## Considered Options

1. **Identical verdicts** — Same `APPROVE`/`REQUEST_CHANGES`/`COMMENT` with
   same thresholds. Simple but inappropriate cost model.
2. **Divergent verdicts** — `APPROVE`/`REVISE`/`COMMENT` for plans with a lower
   bar. `REVISE` triggers on any critical OR 3+ major findings. PR reviews only
   escalate on critical findings.
3. **No verdicts for plans** — Plans always get comments, never formal verdicts.
   Loses the signal of structured quality gates.

## Decision

We will use `APPROVE`/`REVISE`/`COMMENT` for plan reviews with `REVISE`
triggered by any critical finding OR 3+ major findings. PR reviews use
`APPROVE`/`REQUEST_CHANGES`/`COMMENT` with a higher threshold (critical
findings only trigger change requests). The deliberate asymmetry reflects that
plan edits are low-cost and should be encouraged.

## Consequences

### Positive
- Plans are revised more aggressively, catching issues before code is written
- The lower threshold is appropriate to the low cost of plan edits
- Consistent verdict categories make the system conceptually coherent

### Negative
- Divergent semantics between plan and PR verdicts require clear documentation
- Orchestrator logic must handle both verdict sets

### Neutral
- The threshold values can be tuned independently for each context

## Source References

- `meta/plans/2026-02-22-review-plan-alignment.md` — Verdict logic definition
  and threshold rationale
