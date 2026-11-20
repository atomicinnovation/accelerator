---
type: "pr-description"
id: "252"
title: "Restructure the explore-pr review deck"
date: "2026-11-20T14:56:52+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/252"
pr_number: 252
tags: ["explore-pr", "skills", "review", "visualisation"]
revision: "82f3405112e24783d84b808eb59fe75bf2cac21c"
repository: "accelerator"
last_updated: "2026-11-20T14:56:52+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Restructure the explore-pr review deck

## Summary

Reviewers using the pages `explore-pr` builds asked for six changes, and this
PR makes all of them. The Design view reads as an article. Decisions and
abstractions are met in the tour, at the code that embodies them. Interactive
tools get their own section, and catalogues move into Reference. Decide
becomes a readiness check. The reading passes' observations join the findings
without repeats, and Your notes takes freeform text. The skill was then run
end to end on PR #250.

## Changes

- **Rail in five sections** (`assets/review-sections.js`):
  - The rail reads Start, Read, Interact, Decide, Reference. A section with
    nothing in it is hidden, so Interact appears only with custom views.
  - A custom view without a known `group` now goes in Interact.
  - An old `#criteria` link opens Tests.
- **Reference catalogues** (`assets/catalogue.js`):
  - Decisions, Abstractions, Invariants & risks, and Tests, each filterable.
  - Tests lists every test named by a chapter, an invariant, a ledger row or
    a criterion, de-duplicated, with what names it.
  - Any link can open a catalogue already filtered, through `data-jump` and
    `data-filter`.
- **Editable notes** (`assets/review-notes.js`):
  - Your notes opens with an autosaving freeform box, saved as the
    `notes:general` record.
  - Every recorded item then gets a card whose note can be edited in place,
    and the Markdown export follows.
  - A change of state refreshes the view in place, so typing keeps its focus.
  - The export drops no record whose kind it does not know; those go under
    Other.
- **The tour carries the design** (`assets/stop-evidence.js`,
  `assets/prose-references.js`):
  - A chapter's wall of design cards gives way to a strip naming what it
    introduces.
  - Each decision is set out in full, with its control, at the first stop
    that carries it.
  - A decision or abstraction may name its carrying stops in `carriedBy`.
  - The build now warns about any decision or abstraction no stop carries,
    in a chapter or in `design.json`.
  - Prose links decision, abstraction and invariant ids to their entries in
    Reference.
- **The Design view is an article** (`design.json` `article`):
  - Headed sections hold prose, figure, code and aside blocks.
  - Code excerpts are anchored and re-anchored like stops, kept to 15 lines,
    and claim no lines as read.
  - The design pass now runs **after** the chapter passes, reuses their ids,
    and writes the article.
- **One findings set** (`findings_register` in `scripts/build.py`,
  `assets/target-renames.js`):
  - Observations from the reading passes and the design pass become findings
    with a source, merged by `sameAs`.
  - Their links (`sameAs`, `extends`, `distinctFrom`) are checked, and the
    build warns about unlinked findings at overlapping lines.
  - Old `ownNotes` still work as `N-…` findings. Verdicts on their positional
    `own:` targets are copied once, head and all, when the page loads.
- **Readiness** (`assets/readiness.js`, `content/readiness.json`):
  - Decide opens with the skill's recommendation and the conditions that
    would change it.
  - Eleven gates follow, worked out from the content and the reviewer's
    records: met, needs you, or not met.
  - Then comes the reviewer's verdict, moved from the Overview, which keeps a
    readiness strip.
- **Skill, briefs and docs**:
  - Every pass is handed the whole prior-findings list.
  - `SKILL.md` stays at its 360-line cap.
  - The example design article is the one the design pass wrote for PR #250.
- **Smoke check**:
  - Six new probes: catalogues, notes, design, findings, readiness, and the
    tour's decisions met once.
  - The commits probe now tests unread-run links in a file that has a run of
    three changed lines, instead of failing on a one-line file.

### Migration for existing workspaces

Breaking for workspace content, not for the plugin's users.

- A rebuilt older workspace now warns until it has a `design.json`
  `article` and a `content/readiness.json`. It also warns for any decision
  or abstraction no stop carries.
- `ownNotes` keep working, so stored verdicts keep their targets.
- `reference/re-running.md` says how to bring a workspace up to date.

## Context

- Plan: `meta/plans/2026-11-19-explore-pr-review-deck-restructure.md`. Its
  six phases match the commits.
- No work item. The proposal was agreed with the user before planning.
- First end-to-end run: PR #250, at standard depth, delivered standalone and
  hosted.

## Testing

- [x] `mise run check` passes on the final tip. A full `mise run` passed after
      phase 6; the three commits after it touch only the plan, `smoke.js` and
      the example, and `mise run check` was re-run on them.
- [x] The skill's suites pass: 342 pytest tests (up from 273) and 139 node
      tests (up from 74). Each page model came with failing tests first:
      `review-sections`, `catalogue`, `review-notes`, `target-renames`,
      `readiness`, and the extended `stop-evidence` and `prose-references`.
- [x] `uvx ruff format --check` is clean, and `uvx ruff check` still reports
      the 40 findings it reported before.
- [x] The smoke check reports `"ok": true` against PR #250's page and a
      synthetic workspace. The synthetic workspace covers legacy `ownNotes`,
      a `sameAs` merge and an observation that `extends` a prior finding.
- [x] Verdict carry-over checked in Chrome: an old `own:` verdict landed on its
      `N-…` finding with its original head, and Your notes listed it once.
- [x] End to end on PR #250 (manual):
  - the catalogues filter and share verdicts with the tour;
  - all 58 chip-strip links open each item's first carrying stop;
  - the Design view reads as an article on the chapters' ids;
  - no finding appears twice;
  - the readiness gates match their model.
- [x] The PR #250 stepper and guard-explorer views agree with their pass
      specifications. The guard model also agrees with all 77 rows of the
      guard's own boundary fixture, after both prefixes.
- [ ] Manual, needs a person in the hosted page:
  - type into the freeform box and confirm it survives a reload;
  - read `notes:general` back with `ArtifactData`;
  - edit a card's note and see it in the tour.

## Notes for Reviewers

- **Where to look hardest:**
  - `assets/page.html`: `Store.put`'s new `preserveHead` option,
    `refreshNotes` (when it re-renders and when it keeps focus), and the
    generalised `data-jump` handler.
  - `scripts/build.py`: `findings_register` (merging, link errors,
    overlaps) and `well_formed_design` (the article's lenient shape).
  - `assets/stop-evidence.js`: how `carriedBy` merges with `carries`, and
    which stop introduces an item.
- **Deviations from the plan:**
  - The node suites run from one parametrised test in `tests/test_build.py`.
    The register and readiness build tests sit in `tests/test_references.py`.
    The plan named a new file for each, but every new Python file adds a
    `CPY001` finding, which would break its "no more than 40" ruff rule.
  - The example article was hand-written at phase 4. The last commit replaces
    it with the real one from PR #250.
- **Known limitation:** a readiness gate's "Open the outstanding items" link
  opens its catalogue with the nearest filter it has, not always exactly the
  outstanding items. The invariants gate opens the "not yet" list; the
  high-risk files gate opens All files unfiltered. The plan's check for this
  stays unticked.
- **Unchanged on purpose:** the diagram kit, `review-pr`,
  `parse_review.py`'s output, and the triage vocabulary mismatch
  (`drop`/`reject`). The skill's suites are still not part of `mise run`.
