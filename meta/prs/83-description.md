---
type: pr-description
id: "83"
title: "Mark the Jira and Linear integrations epic done"
date: "2026-08-27T22:49:42+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
parent: "work-item:0171"
relates_to: ["work-item:0196", "work-item:0211", "work-item:0212"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/83"
pr_number: 83
tags: [work-items, status, jira, linear, integrations]
revision: "0035551b87baf62df5dbbfd4808c0612c46292de"
repository: "accelerator"
last_updated: "2026-08-27T22:49:42+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Mark the Jira and Linear integrations epic done

## Summary

Flips four work items to `done`: the 0171 Jira and Linear integrations epic and the three siblings 0196, 0211 and 0212 that landed with it. Metadata-only under `meta/work/` — no code, skills, hooks or build tooling change.

## Changes

- **0171 (epic) → done.** Closed after verifying its four children's acceptance criteria. 0210, 0211 and 0212 are substantively complete; 0213's Rust deliverables are complete but three of its criteria remain unmet (see Notes for Reviewers). The epic holds no criteria of its own — the children are normative — so it was closed on their aggregate state.
- **0196, 0211, 0212 → done.** Carried by the preceding commit on this branch; each flips both the frontmatter `status` and the body `**Status**` label.
- **Frontmatter re-serialisation on 0171.** `accelerator work update` rewrote the file through its canonical serialiser, so the diff also shows quote normalisation on unrelated keys (`title`, `date`, `parent`, `relates_to`, `external_id`). This is the tool's output, not a hand edit.

## Context

Closes the 0171 epic (`meta/work/0171-jira-and-linear-integrations.md`, remote `PP-192`) under the 0136 Rust CLI migration. The integrations work — provider client crates (0210), the two dispatched binaries and bash-cluster retirement (0211), the work-item script cutover (0212) and the conversational conflict flow (0213) — is in production. 0196 is a 0136 sibling (design-inventory CLI) bundled into the same status sweep.

## Testing

- [x] Acceptance-criteria verification across all four children (0210, 0211, 0212, 0213) by static inspection — locating the named tests, fixtures, greps and registration edits rather than re-running the suite.
- [ ] Full `mise run` end-to-end and the credentialed contract/sync lanes were **not** run in this session; each child gates on `mise run` at its own merge boundary and merged green there.

## Notes for Reviewers

- **0171 is marked done over a known gap.** Child 0213's Phase 5 was deferred: no eval suite at `skills/work/sync-work-items/evals/`, no committed eval evidence, and the automated SKILL.md lint the validation claimed (`tasks/lint/sync_conflict_flow.py`) never landed in the merged branch — the SKILL.md prose is present but unguarded by any check. 0213's own validation record recommended *not* marking it done, and no follow-up work item currently tracks the remainder. This PR accepts that gap deliberately.
- **The branch carries two commits** and marks four items done, not only the integrations epic — 0196 rides along from the preceding commit. Split them if you want the epic closed on its own.
- **The 0171 diff is noisier than the status flip** because of the serialiser re-quoting; the only semantic changes are `status: ready → done` and `**Status**: Ready → Done`.
