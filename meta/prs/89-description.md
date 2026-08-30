---
type: "pr-description"
id: "89"
title: "Close work item 0125 as done"
date: "2026-08-30T23:40:08+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0125"
parent: "work-item:0125"
relates_to: ["work-item:0199", "work-item:0188", "work-item:0169", "work-item:0174"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/89"
pr_number: 89
tags: []
revision: "83fd43c5f9f1b925e5256b7f2b2be8746a5edef8"
repository: "accelerator"
last_updated: "2026-08-30T23:40:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Close work item 0125 as done

## Summary

Closes work item 0125 ("Converge legacy lexical VCS detection on the 0058 probe layer") as done. The item's goal was reached indirectly through the epic-0136 shell-retirement chain rather than a dedicated plan, so this records the closure with a note tracing where the work actually landed.

## Changes

- Transitions `meta/work/0125-converge-vcs-detection-on-probe-layer.md` from `draft` to `done` in both frontmatter and the body status line.
- Adds a closing note attributing completion to `0188` (library-backed in-process VCS adapter), `0169` (hooks migration), `0199` (residue retirement), and `0174` (`scripts/` deletion), and records that residual `find_repo_root`/`vcs-common` mentions in `tasks/measure.py` and doc comments are historical only.
- Canonicalises the frontmatter quoting to the current standard: unquotes `date`/`last_updated`/`relates_to` scalars and folds the long `title` as a block scalar.

## Context

Stacks on `0199-retire-vcs-common-residue` (PR #86), the designated successor that retired `find_repo_root`/`vcs_mode`. The two-strategy drift 0125 existed to fix is now structurally impossible — VCS detection is single-sourced through `vcs_adapters::library::InProcessProbe`.

## Testing

Documentation-only change to a single work-item file; no code paths are affected and no automated verification applies.

## Notes for Reviewers

Base is `0199-retire-vcs-common-residue`, not `main` — review the diff against that branch. The closing note is the substantive content; the remaining hunks are mechanical frontmatter-quoting canonicalisation.
