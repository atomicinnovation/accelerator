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
revision: "92d7a3a271004f1002465d624f452e801d890c53"
repository: "accelerator"
last_updated: "2026-10-15T00:40:11+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0310] Recover from an interrupted research conduct run

## Summary

When a `research-topic` `conduct` run stops partway, for example because an
agent usage limit, a crash or a closed terminal ends it, the next `conduct`
now reports what the earlier run left of its last batch. It also quarantines
any invalid findings that run wrote, and keeps owing those renames until they
are done, so a second interruption cannot lose the recovery. Before this
change, recovery worked only because of how the ledger is designed. The one
sign of trouble was a warning that read like a concurrency hazard, and stray
invalid findings stayed on disk, counted in the manifest's `finding_count`.

The recovery has to come from the next run. A usage limit applies to the
whole account, so it refuses the orchestrator's next turn as well, and the
run that did not end cannot halt or summarise itself.

## Changes

- **`Recovery::of`** (`cli/research/src/conduct/replaced_run.rs`): one call
  that classes the replaced ledger's last offered batch, plans as if its
  invalid findings were already quarantined, and reports. It returns the
  quarantined inputs, the plan, the `ReplacedRun` report and the renames the
  new run owes. The steps behind it (`ReplacedBatch`) are private, so they
  cannot run out of order. Each spawn lands in one of four classes, judged by
  the one output file it writes:
  - `valid`
  - `invalid`
  - `unwritten`
  - `unasked`, meaning no outline pair owns the spawn's stem any more

  A pair's finding is `valid` only when it answers its outline pair, not
  just when it passes schema validation. A node is judged against the plan's
  rejections and against the question its tree grew from.
- **Owed quarantine** (`RunMemory::owed_quarantine`,
  `RunLedger::owing_quarantine` and `settling_quarantine`): the new run's
  ledger records
  each quarantine rename whose path its plan does not offer again. While an
  owed finding is still on disk, `--run` exits 1 with
  `E_TOPIC_RESEARCH_QUARANTINE_OWED` naming each path, and a later `--start`
  quarantines it again and lists it in `replaced_run.owed`. The ledger file
  gains an optional `owed_quarantine` key; a ledger without it owes nothing.
- **`outstanding --start`** emits `replaced_run`: the run id, the four
  classes, the absolute `quarantine` paths and the `owed` stems. Entry
  `question` and `profile` are `null` when no pair owns the stem and the
  entry is not an unasked finding naming one. The replaced-run notice goes
  to stderr only, and asks its reader for the renames only when there are
  some. A corrupt ledger still gets a `warnings` entry and no report.
- **Index claiming** (`cli/research/src/topic/plan.rs`): a `.levels/` tree
  now outranks a quarantine marker when claiming a question's index, because
  a tree holds research and a marker only reserves an index. A finding read
  as quarantined claims its index for the question it names, whether it
  answers another pair or fails validation
  (`Finding::still_naming`), so the first plan matches the plan drawn once
  the renames are done, with or without the run's own claims.
- **Stem ownership** (`OutlineItem::stem_at`, `pair_at`, `pair_ending`,
  `Stem::ends_with_profile`): a stem is matched back to its outline pair. If
  the question was reworded to the same normalised form, ownership falls
  back to the longest profile that ends the stem after a non-empty slug, so
  `deep-web` is not mistaken for `web`.
- **`SKILL.md` `conduct`**:
  - Step 2 prints the report of a run that did not end verbatim before any
    other tool call, including a "still to quarantine from an earlier start"
    group. It then renames each `quarantine` path, and stops `conduct` if a
    rename fails.
  - Step 3 (Clear) waits until the report has been printed.
  - The summary opens with a **Run that did not end** section, and the
    failure-reason table gains a "run that did not end" row.
- **Docs and CHANGELOG**: a leftover `.conduct-run.json` is no longer
  described as "safe to delete". Re-running `conduct` is the recovery, and
  deleting the ledger skips it. `research.md` documents `owed`, the new
  error code, `null` pair fields, and the window left by agents of a run
  that is still going. The CHANGELOG records the dropped `warnings` entry,
  the index-claim precedence and the renames other `--start` callers owe.
- **`CONTRIBUTING.md`**: explains how to run local builds of the
  sub-binaries under `--plugin-dir`, and which of those launches the
  unverified log records.
- **Work items**: the halting-failure class moves from 0310 to 0304.
  0307 and 0308 record how they interact with the replaced ledger.

## Context

- Work item: `meta/work/0310-recovering-from-an-interrupted-conduct-run.md`
  (source: #147)
- Plan: `meta/plans/2026-10-13-0310-recovering-from-an-interrupted-conduct-run.md`
- Research: `meta/research/codebase/2026-10-13-0310-recovering-from-an-interrupted-conduct-run.md`
- Validation: `meta/validations/2026-10-13-0310-recovering-from-an-interrupted-conduct-run-validation.md`
- Review: `meta/reviews/prs/163-review-1.md`; every finding is addressed on
  the PR

## Testing

- [x] `cargo test -p research -p research-adapters -p accelerator-research`:
      755 passed, 0 failed
- [x] `tests/unit/tasks/test_research_structure.py`: 89 passed
- [x] Full `mise run` on the rebased tip: exits 0
- [x] Manual interrupted-run walkthrough (the plan's Testing Strategy):
      ticked in the plan; not repeated after the review fixes
- [x] Edge cases covered by tests:
  - a corrupt ledger, and an empty last batch
  - a misdirected finding, and one that fails validation but names its
    question, each planned as its marker would be
  - a second `--start` before the renames, and `--run` while a rename is
    owed
  - an owed stem the recovering plan offers again, which is not owed
  - a question reworded to the same normalised form, including under
    overlapping profiles (`web`, `deep-web`)
  - a finding for a question removed from the outline, and an orphaned tree
    under a root marker

## Notes for Reviewers

- ⚠️ The `outstanding --start` JSON changes, within the additive-only
  contract:
  - A consumer that ignores `replaced_run.quarantine` is stopped by
    `E_TOPIC_RESEARCH_QUARANTINE_OWED` on its next `--run` rather than
    acting on a false plan.
  - A readable replaced ledger no longer produces a `warnings` entry; its
    notice is on stderr only.
- The CLI still never renames files; quarantine stays in the skill. A
  re-offered pair reuses its old index when the invalid finding at that
  stem names the pair's question, and takes a fresh one otherwise.
- An agent of a run that is still going can finish a finding after `--start`
  judged it, and the rename then quarantines it anyway. A write after the
  rename to an owed path stops the next `--run`. Nothing is deleted either
  way. Attributing such writes to the earlier run is 0308's job, and
  usage-limit wording inside a surviving run is 0304's.
- The report prints "(quarantined)" before the rename is attempted. If a
  rename fails, the skill lists the affected paths and says they are not
  quarantined.
