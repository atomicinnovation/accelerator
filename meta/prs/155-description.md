---
type: "pr-description"
id: "155"
title: "[0303] Refine the research concurrency work item through review"
date: "2026-10-10T20:53:32+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0303"
parent: "work-item:0303"
relates_to: ["work-item:0304", "work-item:0307", "work-item:0310"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/155"
pr_number: 155
tags: []
revision: "83bbe57cb208b4796647fa077e36d505165b4413"
repository: "accelerator"
last_updated: "2026-10-10T20:53:32+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0303] Refine the research concurrency work item through review

## Summary

This PR sharpens work item 0303. It is a bug: `research-topic conduct`
defaults to 24 concurrent spawns, Claude Code refuses more than 20, and
4 spawns per full batch are lost. The PR changes only the specification
and its review; the fix itself comes in a later PR. After a stress-test
and four `/review-work-item` passes, the item's verdict went from REVISE
to COMMENT, and it is ready for planning.

## Changes

- **Retry rule**: the item now defines a round (one follow-up message of
  re-issued spawns; the original batch message is not a round). The first
  round always runs, and later rounds run while the latest round accepted
  at least one spawn. This replaces a fixed retry count and closes the case
  where a fully refused batch was never retried.
- **Clamp**: the cap is parsed as the 2.1.296 harness parses it (trimmed,
  `^[+-]?\d+$`, at least 1), after 0282's knob validation. The built-in
  default clamps silently, and a flag or config value warns.
- **Verification**: the item adds four manual runs: knob resolution,
  full load, partial hold and full hold. Hold runs use background
  "holder" subagents to force capacity refusals, with a probe spawn
  checking the setup first. Parser cases are paired with
  `research.topic.concurrency: 24` so a misparse is visible. The full-load
  run only counts if a batch offers exactly 20 spawns.
- **Structural pins**: tests check verbatim sentences rather than loose
  tokens, so prose that says the opposite cannot pass.
- **Dependencies**: the item records the Claude Code harness contract (cap,
  env-var parser, refusal text) as an external dependency. It also records
  overlaps with 0304, 0307 and 0310, and states what it supersedes in 0282
  and 0283. 0307 and 0310 gain the matching link back to 0303.
- **Failure handling**: the item adds a failure-reason table row for the
  exhaustion reason, which now names both causes: held slots, and a harness
  cap below the resolved concurrency.
- **Review record**: the review file
  `meta/reviews/work/0303-…-review-1.md` covers the initial five-lens review
  and three re-review passes.

## Context

- Work item: `meta/work/0303-research-concurrency-exceeds-subagent-limit.md`
- Source issue: https://github.com/atomicinnovation/accelerator/issues/140
- Review: `meta/reviews/work/0303-research-concurrency-exceeds-subagent-limit-review-1.md`
- Orchestration vocabulary: `meta/plans/2026-09-26-0283-recursive-finding-deepening.md`

## Testing

- [x] `accelerator corpus frontmatter validate` passes for all four changed
      files.
- [ ] `mise run check` not run. The PR changes only Markdown under `meta/`,
      which no lint or type-check covers.

## Notes for Reviewers

- **Verdict**: pass 4 ended at COMMENT with 1 major finding: parser cases
  that could not tell a correct parse from a wrong one. The final edits fix
  it, but no further review pass has checked those edits.
- **Known open minor findings**: these are left open on purpose and recorded
  in the review's pass 4 section.
  - Some knob-resolution rules have no pinned sentence.
  - Nothing detects a holder that exits mid-run.
  - There is no fallback if holders prove infeasible.
  - Spawns stopped by a 0310 halting failure would carry the capacity
    reason, which states the wrong cause.
- **Open question**: can a holder stay running until its stop file appears
  without tripping the foreground-`sleep` guard? Both hold runs depend on it.
- **Decisions to sanity-check**:
  - The retry stays bundled with the default/clamp fix.
  - `kind` stays `bug`.
  - The clamp copies the harness parser exactly.
  - 0282 and 0283 are done items and get no back-reference.
- **Verbatim strings**: the implementation PR must use the exhaustion reason
  and the four pinned sentences word for word.
