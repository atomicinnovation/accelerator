---
type: "pr-description"
id: "163"
title: "[0310] Recover from an interrupted research conduct run"
date: "2026-10-13T23:16:43+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0310"
parent: "work-item:0310"
relates_to: ["work-item:0304", "work-item:0307", "work-item:0308"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/163"
pr_number: 163
tags: ["research", "conduct", "recovery"]
revision: "02b5ed1df52da0819ce14643e2987ba75db49f89"
repository: "accelerator"
last_updated: "2026-10-13T23:16:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0310] Recover from an interrupted research conduct run

## Summary

When a `research-topic` `conduct` run stops partway, for example because an
agent usage limit, a crash or a closed terminal ends it, the next `conduct`
now reports what the earlier run left of its last batch. It also quarantines
any invalid findings that run wrote. Before this change, recovery worked
only because of how the ledger is designed. The one sign of trouble was a
warning that reads like a concurrency hazard, and stray invalid findings
stayed on disk, counted in the manifest's `finding_count`.

The recovery has to come from the next run. A usage limit applies to the
whole account, so it refuses the orchestrator's next turn as well, and the
interrupted run cannot halt or summarise itself.

## Changes

- **`ReplacedBatch` and `ReplacedRun`** (`cli/research/src/conduct/replaced_run.rs`):
  takes the replaced ledger's last offered batch and puts each spawn in one
  of four classes, judged by the one output file it writes:
  - `valid`
  - `invalid`
  - `unwritten`
  - `unasked`, meaning no outline pair owns the spawn's stem any more

  A pair's finding is `valid` only when it answers its outline pair, not
  just when it passes schema validation. A node is judged against the plan's
  rejections and against the question its tree grew from.
- **`outstanding --start`** emits a `replaced_run` object with the run id,
  the four classes and the absolute `quarantine` paths. It plans as if those
  findings had already been renamed to `.<name>.invalid` markers. The
  replaced-run notice now goes to stderr only. A corrupt ledger still gets a
  `warnings` entry and no report.
- **Index claiming** (`cli/research/src/topic/plan.rs`): a `.levels/` tree
  now outranks a quarantine marker when claiming a question's index, because
  a tree holds research and a marker only reserves an index. A misdirected
  finding that is read as quarantined therefore plans exactly as its marker
  would.
- **Stem ownership** (`OutlineItem::stem_at`, `pair_at`, `pair_ending`): a
  stem is matched back to its outline pair. If the question was reworded to
  the same normalised form, ownership falls back to the profile that ends
  the stem.
- **`SKILL.md` `conduct`**:
  - Step 2 prints the interrupted-run report verbatim before any other tool
    call. It then renames each `quarantine` path, and stops `conduct` if a
    rename fails.
  - Step 3 (Clear) waits until the report has been printed.
  - The summary opens with an **Unfinished run** section.
  - The failure-reason table gains an "unfinished run" row.
- **Docs and CHANGELOG**: a leftover `.conduct-run.json` is no longer
  described as "safe to delete". Re-running `conduct` is the recovery, and
  deleting the ledger skips it.
- **`CONTRIBUTING.md`**: explains how to run local builds of the
  sub-binaries under `--plugin-dir`.
- **Work items**: the halting-failure class moves from 0310 to 0304.
  0307 and 0308 record how they interact with the replaced ledger.

## Context

- Work item: `meta/work/0310-recovering-from-an-interrupted-conduct-run.md`
  (source: #147)
- Plan: `meta/plans/2026-10-13-0310-recovering-from-an-interrupted-conduct-run.md`
- Research: `meta/research/codebase/2026-10-13-0310-recovering-from-an-interrupted-conduct-run.md`
- Validation: `meta/validations/2026-10-13-0310-recovering-from-an-interrupted-conduct-run-validation.md`

## Testing

- [x] `cargo test -p research -p accelerator-research`: 590 passed, 0 failed
- [x] `tests/unit/tasks/test_research_structure.py`: 88 passed
- [x] `mise run cli:check` (rustfmt and clippy) passes
- [x] Full `mise run`: green in the validation run (5366 CLI tests), after
      fixes to the validation findings; not re-run for this description
- [x] Manual interrupted-run walkthrough (the plan's Testing Strategy):
      ticked in the plan
- [x] Edge cases covered by tests:
  - a corrupt ledger
  - an empty last batch
  - a misdirected finding, valid but naming another question
  - a finding for a question removed from the outline
  - an orphaned tree under a root marker
  - a question reworded to the same normalised form under real NFKC

## Notes for Reviewers

- ⚠️ The `outstanding --start` JSON contract changes:
  - `replaced_run.quarantine` paths must be renamed before acting on the
    plan.
  - A readable replaced ledger no longer produces a `warnings` entry; its
    notice is on stderr only.
- The CLI still never renames files. Quarantine stays in the skill, so a
  re-offered pair takes a fresh index rather than its old one.
- Writes that land after the new run starts, from agents of the earlier run
  still in flight, still appear under Unexpected notes. Attributing them to
  the earlier run is 0308's job. Usage-limit wording inside a surviving run
  is 0304's.
- The report prints "(quarantined)" before the rename is attempted. If a
  rename fails, the skill lists the affected paths and says they are not
  quarantined.
