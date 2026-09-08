---
type: "pr-description"
id: "108"
title: "[0121] Review and break down the Topic Research Skillset epic"
date: "2026-09-08T17:30:08+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0121"
parent: "work-item:0121"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/108"
pr_number: 108
tags: ["research", "skills", "epic-breakdown"]
revision: "a796d2f32ae099ce512b78366163ecf68429c592"
repository: "accelerator"
last_updated: "2026-09-08T17:30:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0121] Review and break down the Topic Research Skillset epic

## Summary

Brings epic **0121 (Topic Research Skillset)** to `ready` and decomposes it into
eight child work items. The PR records the three review passes that hardened the
epic, marks it ready at high priority, then extracts its six vertical slices into
eight stories (`0277`–`0284`), each synced to Linear under `PP-22`. It carries
meta artifacts only — work items and review records — with no product code.

## Changes

**Epic reviews recorded** (`meta/reviews/work/`):

- Three review passes of `0121`, all lensed on clarity / completeness / scope:
  `review-1` (**REVISE**), `review-2` (**COMMENT**), `review-3` (**APPROVE**).
  Together they drove the epic's revisions — the seven-subcommand workflow, the
  reputation-tier terminology, and the field-name-level artifact contract.

**Epic promoted and rewritten** (`meta/work/0121-topic-research-skillset.md`):

- Status `draft` → `ready`, priority `medium` → `high`.
- Workflow reworked to seven subcommands in two groups — build
  (`brief` → `outline` → `conduct` → `synthesise` → `finalise`) and consume
  (`ask` / `report`) — superseding the earlier `brief`/`conduct`/`expand` model.
- `children` frontmatter wired to the eight new work items.

**Epic decomposed into eight children** (`meta/work/0277`–`0284`):

- Six vertical slices become eight stories, splitting Slice 1 into an engine
  child (`0277`) and a visualiser/indexer child (`0278`), and Slice 5 into a
  tunable-knob child (`0282`) and a recursion-engine child (`0283`), so the
  epic's descope boundary is itself a work-item boundary.
- Dependencies encoded once per edge — canonical `blocks` upstream, with Slice 4
  (`0281`) holding its two edges as `blocked_by` — and every child linked to the
  epic via `parent`.

**Linear sync**:

- The eight children pushed to Linear as `PP-861`–`PP-868`; the sync baseline in
  `.accelerator/state/integrations/linear/last-sync.json` advanced.

## Context

Implements the breakdown of epic `0121` (`PP-22`). The eight children map to the
epic's Slices 1–6:

| Child | Linear | Slice |
|---|---|---|
| `0277` Single-Round Web Research Engine | `PP-861` | 1 (engine) |
| `0278` Topic-Research Visualiser Doc Type and Indexer | `PP-862` | 1 (visualiser) |
| `0279` Iterative Accretion and Finalise | `PP-863` | 2 |
| `0280` Academic Source Profiles | `PP-864` | 3 |
| `0281` Corpus Consumption: Ask and Report | `PP-865` | 4 |
| `0282` Tunable Depth and Breadth | `PP-866` | 5 (knob) |
| `0283` Recursive Finding Deepening | `PP-867` | 5 (recursion) |
| `0284` Topic-Research Set Detail Page | `PP-868` | 6 |

## Testing

- [x] `accelerator corpus frontmatter validate` passes on all eight children and
      the epic.
- [ ] `mise run check` not run — the diff is meta markdown plus one line of
      Linear sync state, touching no frontend / server / cli / build-system /
      scripts code, so no code lane is affected. CI will still run it.

## Notes for Reviewers

- **Bundled scope**: three pre-session review commits ride with this session's
  two extraction commits — all epic-`0121` work, brought to `main` together.
- **Two split decisions to sanity-check**: `0277`/`0278` must co-land (engine +
  library entry), and `0282`/`0283` ship adjacent (knob before recursion).
- **One deliberate dependency relaxation**: `0282` (the knob) waits only on
  `0279`, not on the `0280` output-quality gate — only the recursion engine
  `0283` sits behind `0280`. Flagged in `0282`'s Drafting Notes.
- **Open questions carried forward**: set-document API shape (`0284`) and the
  synthesis-staleness mechanism (`0279`).
- **Linear parenting caveat**: the children exist as `PP-861`–`PP-868`, but the
  plugin's Linear integration exposes no parent field, so they are likely flat
  top-level issues rather than sub-issues nested under `PP-22`. The `parent`
  links are intact locally.
