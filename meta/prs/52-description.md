---
type: "pr-description"
id: "52"
title: "Split work-item 0173 into 0195/0196/0197, review each, and mark 0197 ready"
date: "2026-08-06T01:34:44+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
parent: "work-item:0136"
relates_to: ["work-item:0173", "work-item:0195", "work-item:0196", "work-item:0197"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/52"
pr_number: 52
tags: []
revision: "da56e1204b087db48d4da7e48193cae002465c1b"
repository: "accelerator"
last_updated: "2026-08-06T01:34:44+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Split work-item 0173 into 0195/0196/0197, review each, and mark 0197 ready

## Summary

Work-item 0173 bundled three functionally independent efforts — `accelerator-corpus`, `accelerator-design`, and `accelerator-collaboration` — into a single oversized story, and its review-1 pass flagged this as a scope violation. This PR abandons 0173 and splits it into three new stories (0195, 0196, 0197) parented directly under the 0136 epic, reviews each of the three through all five work-item lenses, addresses the findings from 0197's review, and marks all three items `ready` for planning. It touches only `meta/work/` and `meta/reviews/work/` — no source code changes.

## Changes

- Abandon work-item:0173 (`status: abandoned`), record the split rationale in its Drafting Notes, and link forward to its three successors via `relates_to` and a new "Superseded by" reference.
- Add work-item:0195 (`accelerator-corpus`: ADR, metadata, frontmatter validation, and linkage CLI), work-item:0196 (`accelerator-design`: design inventory and gap tooling CLI), and work-item:0197 (`accelerator-collaboration`: PR helper CLI), each carrying forward its slice of 0173's Requirements/Acceptance Criteria/Dependencies with that review's gaps fixed, and each set to `status: ready`.
- Review 0173, 0195, 0196, and 0197 through the clarity, completeness, dependency, scope, and testability lenses; record each review under `meta/reviews/work/`. 0173's review-1 (verdict REVISE) is the scope finding that motivated the split. 0195 and 0196 land at verdict APPROVE. 0197 goes through two review passes: pass 1 (verdict COMMENT) surfaces a cross-cutting finding that 0197 inconsistently characterised work-item:0150's completion status across its Summary, Context, and Dependencies sections, plus several precision gaps (glob-shaped scope language, Requirements missing two AC-gated obligations, an ambiguous AC1 verification method); pass 2, after those edits, surfaces one new major finding — 0197 didn't reciprocate a sibling-coordination note that 0196 already carries about concurrent sub-binary registration touching shared state — which is then also addressed, bringing 0197 to verdict APPROVE.
- Update work-item:0136 (the epic)'s phase list and Drafting Notes to reflect the split, and update work-item:0174's `blocked_by` and work-item:0179's `blocks`/cross-references to point at 0195/0196/0197 instead of the now-abandoned 0173.

## Context

Part of the ongoing shell-to-Rust CLI migration epic (work-item:0136). 0173 was originally extracted from `codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture` as a single grouped story for the three remaining sub-binary domains; its review-1 found the grouping risked partial-completion ambiguity and an oversized PR, since the three sub-binaries share no functional relationship beyond the sub-binary registration pattern (work-item:0187).

## Testing

- [x] Confirmed every changed file's YAML frontmatter parses as valid YAML with the expected keys (ran a standalone parse check across all 11 touched files).
- [x] Confirmed 0173/0174/0179's cross-references to the split items are reciprocal (e.g. 0174's `blocked_by` lists 0195/0196/0197; 0196's Dependencies names 0195 and 0197 in its coordination note, which 0197 now reciprocates).
- [ ] No source code changed by this PR, so `mise run check`/`mise run` were not run — there is nothing in `cli/`, `server/`, `frontend/`, or `tasks/` for them to exercise.

## Notes for Reviewers

This is a documentation/work-item-management PR only; it reshapes planning artifacts ahead of implementation, not application behaviour. Actual implementation of 0195/0196/0197 (the three Rust sub-binaries themselves) is follow-up work, not part of this PR.

One known gap, left out of scope deliberately rather than fixed here: five already-completed/merged items (work-item:0166, 0167, 0169, 0172, 0187) still carry a `blocks: [..., "work-item:0173"]` frontmatter edge from before the split. Since 0173 is now abandoned rather than progressing, those edges are stale — they should arguably be retargeted at 0195/0196/0197 (mirroring what this PR already did for 0174 and 0179), but touching five unrelated, already-shipped items felt like scope creep for a PR whose job is the split itself. Flagging for a reviewer call on whether that cleanup belongs here, in a quick follow-up, or is fine left as historical record.
