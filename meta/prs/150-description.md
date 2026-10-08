---
type: "pr-description"
id: "150"
title: "Sync work item statuses and inventory the CLI surface"
date: "2026-10-08T11:48:28+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0121", "work-item:0146", "work-item:0207", "work-item:0215", "work-item:0217", "work-item:0226", "work-item:0230", "work-item:0241", "work-item:0277", "work-item:0278", "work-item:0280", "work-item:0283", "work-item:0286", "work-item:0295"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/150"
pr_number: 150
tags: ["meta", "work-items", "housekeeping", "research", "cli"]
revision: "70c81d06ee45b991840bbf86ac41427f2974b0b0"
repository: "accelerator"
last_updated: "2026-10-08T21:01:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Sync work item statuses and inventory the CLI surface

## Summary

This PR brings `meta/work/` up to date with the work that has merged or
started since the last status sweep. It also adds a codebase research
document that inventories the full CLI surface. Every change is under `meta/`;
no code, skill, or build file is touched.

## Changes

- **Done**: 0207, 0217, 0226, 0241, 0277, 0278, 0280, 0283 and 0286 are
  marked done, each now that its PR has merged (#122, #138, #135, #132, #117,
  #118/#123/#124, #134, #136, #131).
- **Abandoned**: 0215 is abandoned as superseded by 0216. 0217 measured the
  musl warm-path digest at 3047 MB/s, so the cache-hit sha256 is no longer
  worth removing. A dated note records why, and the acceptance criteria are
  deliberately left unticked.
- **In progress**: 0230 (PR #137) and 0295 (PR #148) move to in-progress,
  and so do the 0121 and 0146 epics now that their children are underway.
- **0121 epic**: 0295 is added to its children, because the arXiv batch cap
  belongs to the topic research skillset.
- **0146 epic**: the requirements now point to the children that deliver
  them: 0290 for field mapping, 0291 for parent–child sync, 0229 for pull
  scope, and 0228 and 0230 for keys and ids. 0257, 0285, 0290 and 0291 are
  added to the story list, and the "TBD (existing candidates)" placeholder is
  removed.
- **Ideas backlog**: adds a task to remove `make` references from skills.
- **Consistency**: the body `**Status**:` line now matches the frontmatter
  `status:` on all 14 touched work items. 0146's `last_updated` placeholder
  is replaced with the reconcile commit's real time, and the ideas backlog
  gains its trailing newline.
- **CLI surface inventory**: adds
  `meta/research/codebase/2026-10-03-cli-surface-inventory.md`, which covers:
  - every launcher built-in and dispatched sub-binary, with its subcommands
    and switches
  - the 65-key config catalogue, consent keys included
  - about 60 environment variables, grouped into five families
  - each binary's exit-code vocabulary
  - 18 places where code, docs and the catalogue disagree

## Context

There is no single owning work item; this is a status sweep across the
`0121` (topic research) and `0146` (work item sync) epics and the `0136`
children, plus a stand-alone research document. One commit subject names
0229, but 0229 was already `done` on `main`, so it has no net diff here.

## Testing

- [x] `accelerator corpus frontmatter validate --file <abs path>` exits 0 for
      all 16 changed files. A deliberately broken copy of one file was
      rejected with exit 1, which confirms the validator inspects the files
      rather than passing vacuously.
- [x] Each "done" transition was checked against a merged PR in
      `gh pr list --state all`.
- [ ] `mise run check` was not run, because no file it covers has changed.

## Notes for Reviewers

- **Discrepancies list**: the defects in the research document's
  "Discrepancies and defects" table are not fixed here. Most of them come from
  reading the code; only item 1 (`corpus frontmatter validate --dir
  <relative>` passes vacuously) was reproduced by running it. Each defect is a
  candidate work item.
