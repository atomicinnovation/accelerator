---
type: "work-item"
id: "0296"
title: "Sync Round-Trip Defects Between Local Work Items and Linear"
date: "2026-09-24T22:48:31+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "high"
parent: "work-item:0146"
relates_to: ["work-item:0213", "work-item:0290"]
external_id: "PP-880"
tags: ["sync", "linear", "bug", "baseline", "projection"]
last_updated: "2026-09-24T22:48:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0296: Sync Round-Trip Defects Between Local Work Items and Linear

**Kind**: Bug
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

`accelerator work sync` reports spurious conflicts for items created by
`work create --push`, and its pull path writes Linear's projected title line into
local files. Together they make a full bidirectional sync unsafe to run: a
preview of this repo's corpus plans 18 pulls, and 4 of its 15 conflicts have no
cause beyond a missing baseline entry.

## Context

Change detection compares each side only against its own baseline digest
(`cli/work/src/sync/classify.rs:110-134`); a missing digest counts as changed.
The Linear port body is deliberately `"{title}\n{description}"`
(`cli/remote-projection/src/lib.rs:118-122`) and the port contract states that a
push followed by a read is not the identity. Three defects follow from how
these interact.

## Requirements

**Defect 1 — `work create --push` records no baseline**

- Reproduction: `work create "<title>" story low --push --body-file <body>`
  (outcome `write-once`), then `work sync --preview`.
- Expected: the new item reports `synced`.
- Actual: `unresolved conflict`, because `execute_push` / `try_run`
  (`cli/work-cli/src/create.rs:470-712`) write the file and `external_id` but no
  baseline entry, so both digests are absent. The sync engine's own
  create-from-local path (`link_and_baseline`, `apply.rs:459-496`) does record
  one. Observed on 0302 and on 0275, 0276, 0293 and 0294 in this corpus.

**Defect 2 — pulls write the projected title line into the local body**

- Reproduction: a `remotely-modified` item is pulled, or a conflict is resolved
  with `--resolve <id>=remote`.
- Expected: the local body is the remote `description` only.
- Actual: `pull_outcome` (`run.rs:651-672`) builds the file with
  `reconstruct_pulled_content` (`run.rs:299-307`) from the projected body, so a
  title line lands above the H1. The next push then sends it back to Linear as
  description content. `create_from_remote` already strips it with
  `split_projected` (`sync_author.rs:104`).

**Defect 3 — conflict dossiers show projection and serialisation noise as diffs**

- Actual: every Linear dossier shows the title line, and Linear's `*` list
  markers against the local `-`, as differing sections. The `*` form is Linear's
  own markdown re-serialisation of `description` (inferred; no code in the
  workspace rewrites markers). A user cannot tell a real edit from noise when
  choosing a resolution.

**Recovery for the existing corpus**

- An item with an `external_id` but no baseline entry, or with a blank
  `local_hash`, whose local body matches the remote `description` after
  stripping the title line, has a baseline recorded and reports `synced` rather
  than `conflict`.

**Out of scope**

- Canonicalising markdown in the digests; whether local files keep `-` list
  markers across a pull (see Open Questions).
- Field mapping beyond the body (0290).

## Acceptance Criteria

- [ ] Given `work create --push` returns `write-once`, when `work sync
      --preview` runs next with no edits on either side, then the item reports
      `synced`.
- [ ] Given that create, when its baseline entry is read, then it holds the
      local digest and a remote digest and `updatedAt` taken from a read-back
      of the created issue.
- [ ] Given a `remotely-modified` item, when it is pulled, then the local body
      begins with the remote `description` and contains no projected title line
      above the H1.
- [ ] Given a conflict resolved with `--resolve <id>=remote`, when the file is
      written, then it contains no projected title line.
- [ ] Given a conflict dossier for a Linear item, when it is rendered, then the
      remote side omits the projected title line, and a section whose only
      difference is list-marker style is not listed as differing.
- [ ] Given an item with an `external_id` and no baseline entry (or a blank
      `local_hash`) whose local body matches the remote `description`, when
      `work sync` runs, then it records a baseline and reports `synced`.
- [ ] Given an item with an `external_id`, no baseline entry, and a remote
      `description` that differs from the local body, when `work sync` runs,
      then it still reports `conflict`.

## Open Questions

- Should a pull preserve the local list-marker style (`-`) rather than adopt
  Linear's `*`? Preserving it needs a markdown-aware merge; adopting it churns
  every pulled file once.
- Should "matches" in the recovery rule mean byte-equal after title stripping
  and `trim_lines`, or equal under a markdown canonicalisation (list markers,
  leading blank line)?

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- The `-` → `*` change is Linear's server-side re-serialisation, not a client
  transform; a fixture capture of a real `issueCreate` read-back would confirm
  it.
- Jira's projection has the same title-line shape; the defects are reported
  against Linear, where they were observed.

## Technical Notes

- Classification: `cli/work/src/sync/classify.rs:66-134`; digests:
  `cli/work-adapters/src/sync/digest.rs:76-94` (`trim_lines` only, leading blank
  lines survive).
- Push read-back already stores Linear's returned form as the baseline
  (`apply.rs:191-225`), so once a baseline exists, `*` markers do not recur as
  changes.
- Unresolved conflicts blank `local_hash` on apply (`run.rs:708-711`), which
  keeps them conflicted until resolved; 0136, 0158, 0169, 0203, 0215, 0216 and
  0217 are in this state.
- Test gaps: the fake trackers return canned `show` bodies, so nothing models
  the projected title line or Linear re-serialisation; no test covers
  `create --push` followed by a sync.

## Drafting Notes

- One bug covers three defects because they share one cause (projection-aware
  round-tripping) and one reproduction session; Defect 2 could be split out if
  it needs to ship first, as it is the only one that corrupts files.
- The 18 planned pulls were not inspected individually; whether they are real
  remote edits or re-serialisation drift is unknown, and was left out of the
  requirements.

## References

- Parent: 0146 — Work Item Synchronisation Enhancements
- Related: 0213 — Conversational Conflict Resolution Flow for Sync
- Related: 0290 — Bidirectional Field Mapping for Work Item Sync
