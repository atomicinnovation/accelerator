---
type: "pr-description"
id: "124"
title: "[0278] Validate the topic-research plans and fix co-land test regressions"
date: "2026-09-15T15:27:00+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0277", "work-item:0278"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/124"
pr_number: 124
tags: ["research", "topic-research", "validation", "testing"]
revision: "749fbd78e8a9574da338145e5bc686f7d41a629d"
repository: "accelerator"
last_updated: "2026-09-15T15:27:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0278] Validate the topic-research plans and fix co-land test regressions

## Summary

Validates the three co-landed topic-research plans — the single-round web
research engine (0277) and the two 0278 plans (visualiser doc-type + indexer,
and the codebase-research rename completion) — recording each outcome as a
plan-validation report and recording validation progress on the work items.
The CI regressions the validation surfaced are fixed at their origin layers in
the lower PRs of this stack (#117, #118), so every PR passes independently, not
only the tip. Stacked on #123.

## Changes

- Add three plan-validation reports under `meta/validations/`, one per plan.
  Each is `partial`: implementation and merge complete, read-only checks green,
  with live-web, Docker VR, and human sign-offs still outstanding.
- Move work items 0277 and 0278 to `in-progress` and record the outcome.
  Reconcile 0277's acceptance criteria and artifact-shape notes from
  `research_status` to the manifest's base `status`, per the mandated 0278
  co-land collapse, so the published criteria match the shipped behaviour.
- The co-land CI fixes land in the lower PRs, at the layer each regression
  originates: **#117** — conformance producing-skill count `19 → 18`,
  `research_agent_contract` repointed to the `topic-research-finding` template,
  and `rustls` pinned `=0.23.45` to clear `RUSTSEC-2026-0285`; **#118** —
  `resolve_goldens` exit-4 case repointed to an unregistered type, the duplicate
  `work-item:0286` renumbered to `0289`, a topic-research `ac2-coverage` E2E
  fixture, and the 10 regenerated topic-research VR baselines.

## Context

- Plans validated:
  `meta/plans/2026-09-09-0277-single-round-web-research-engine.md`,
  `meta/plans/2026-09-10-0278-topic-research-visualiser-doc-type.md`,
  `meta/plans/2026-09-13-0278-complete-codebase-research-rename.md`.
- Work items: 0277 (single-round web research engine), 0278 (topic-research
  visualiser doc type and indexer).
- Reports:
  `meta/validations/2026-09-09-0277-single-round-web-research-engine-validation.md`
  and the two `2026-09-{10,13}-0278-*-validation.md` siblings.

## Testing

- [x] Validation-report frontmatter validates clean
      (`accelerator corpus frontmatter validate --file`).
- [x] The stack-wide remediation drives every CI check green. Verified per
      layer: #117 — conformance 27 passed, `research_agent_contract` 2/2,
      `deny:check` clean; #118 — `resolve_goldens` 9/9, `this_repositorys_own_
      corpus_is_clean` green, visualiser server tests green (indexer now 4 sets),
      Docker VR 225 passed with the 10 new topic-research baselines.
- [x] The fixes for those checks live in #117/#118, not this PR — see the
      Changes above and those PRs.

## Notes for Reviewers

- This PR is validation reports + work-item bookkeeping. The co-land CI fixes
  were applied at their origin layers (#117/#118) so every PR in the stack goes
  green independently, not only the tip — each corrects a pre-co-land assumption
  that 0278's registration or the outputter refactor invalidated, not a
  behaviour change; the CLI, discovery, and rendering behaviour were already
  correct.
- The validation reports are `partial` by design — the live-web `brief →
  outline → conduct → synthesise` loop (0277), the Docker VR baselines and
  design/status-chip sign-offs (0278), and a few visual confirmations (rename)
  remain.
- The `research_status → status` reconciliation is applied to 0277; the sibling
  reconciliations on 0279 and epic 0121 are still outstanding follow-ups.
