---
type: pr-description
id: "24"
title: "[0168/0169] Refine work items"
date: "2026-07-20T15:22:29+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
parent: "work-item:0136"
relates_to: ["work-item:0166", "work-item:0168", "work-item:0169", "work-item:0180", "work-item:0172"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/24"
pr_number: 24
tags: [rust, vcs, visualiser, work-items, planning]
revision: "6a8227762496309959238736e78eb15607105dc5"
repository: "accelerator"
last_updated: "2026-07-20T15:49:34+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0168/0169] Refine work items

## Summary

Grooms the next tranche of the Rust CLI migration epic (0136): refines work items **0168** (fold the visualiser into the `cli/` workspace) and **0169** (VCS subdomain + hooks migration) to *ready* through a review pass, and marks **0166** and **0180** *done*. This is a documentation-only PR — no code changes.

## Changes

- **0169 — VCS subdomain & hooks migration**: reviewed (five lenses, REVISE → APPROVE) and refined to *ready*. Key refinement: VCS access is now specified as **library-based** — `gix` (gitoxide) for git and `jj-lib` for jujutsu, read in-process behind the outbound VCS port, rather than spawning `jj`/`git` subprocesses (a per-query shell fallback stays behind the same port). Flags `jj-lib`'s unstable public API as an early-validation risk.
- **0168 — fold visualiser into `cli/` workspace**: reviewed (REVISE → APPROVE) and refined to *ready*; relocates `accelerator-visualiser` onto the shared `config`/`corpus` crates and moves start/stop/status under `accelerator visualiser …` as the first on-demand sub-binary (ADR-0054).
- **Status transitions**: 0166 (shared config/corpus/store crates) moved draft → *done*, and 0180 (atomic-store primitives) moved ready → *done*.
- Adds the work-item review artifacts for 0168 and 0169.

## Context

- Epic **0136** — Rust CLI migration (`parent`).
- Refines **0168** (`PP-189`) and **0169** (`PP-190`); marks **0166** and **0180** (`PP-704`) done.
- Relates to **0172** (migration engine subdomain) — 0169 blocks it, and the SessionStart migration-discoverability reminder is deferred to it.
- Stacked on `0180-atomic-store-primitives` (PR #23), which carries the corpus/atomic-store code this grooming refers to.

## Testing

- [x] No executable surface — the changes are work-item and review markdown; verification is limited to frontmatter well-formedness and internal linkage consistency, both confirmed by inspection.

## Notes for Reviewers

- The load-bearing decision to confirm is **0169's library-based VCS access** (`gix` + `jj-lib` in-process vs subprocess shell-out) — it departs from the earlier `CommandProbe` shell-out assumption and hinges on `jj-lib`'s unstable API, which the work item calls out for early validation.
- Both work-item reviews resolved from an initial **REVISE** (driven by breadth, not weakness) to **APPROVE**; the review docs are included for the audit trail.
