---
type: "work-item"
id: "draft-01raev"
title: "Keep conduct's Grants Through a Headless Session"
date: "2026-11-26T16:32:37+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
relates_to: ["work-item:PP-893"]
tags: ["research", "permissions"]
last_updated: "2026-11-26T16:32:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# draft-01raev: Keep conduct's Grants Through a Headless Session

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As someone running `/accelerator:research-topic conduct` in a headless
`claude -p` session, I want the researchers and the orchestrator to keep
`research-topic`'s grants until the run ends, so that a run does not fail
every node of a batch, or stall on an approval no one gives, after the
orchestrator's turn ends.

## Context

Claude Code can drop a skill's `allowed-tools` once the orchestrator's turn
ends while background subagents are still running. PP-893 (PR #250) moved a
node researcher's context and template behind two commands, the node query
and `accelerator config template`, which it runs before it writes anything.
Before PP-893 a lost grant cost a web researcher only optional metadata; now
it fails every node of the batch. The orchestrator's own later commands lose
their grants too: its next plan, `end-run`, `counts` and the corpus
commands.

PR #250 documents the behaviour and the project allow rules that work around
it, under "Granting the commands" in `docs-site/src/content/docs/research.md`.
Those rules apply to every session, not only to confined researchers, which
is the exposure the docs' caution names.

## Requirements

- Find out when and why Claude Code drops a skill's grants in a headless
  session, on the minimum supported release and the latest.
- Choose a way for `conduct` to keep working through it without asking users
  to widen their project allow rules, or record why none exists.
- If the project allow rules remain the answer, keep the documented list in
  step with `research-topic`'s `allowed-tools`.

## Acceptance Criteria

- [ ] Given a headless `conduct` run at depth 2 whose orchestrator's turn
      ends while researchers run, when no project allow rule covers the
      research commands, then every node writes its note and the run ends
      with `end-run`, or the docs record why that cannot hold.
- [ ] The research docs say which releases drop the grants and what a user
      must do about it.

## Open Questions

- Is the grant loss a Claude Code defect to report upstream, or intended
  behaviour for background subagents?
- Would waiting on every spawn in the foreground, as step 4 of `conduct`
  already requires, keep the turn open long enough?

## Dependencies

- Builds on PP-893.

## References

- PR #250 and its first review, `meta/reviews/prs/250-review-1.md`, finding
  R1-05.
- `docs-site/src/content/docs/research.md`, "Granting the commands".
