---
type: "pr-description"
id: "187"
title: "List each key abstraction once on the explore-pr design page"
date: "2026-11-09T22:57:39+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/187"
pr_number: 187
tags: ["explore-pr", "skills"]
revision: "4af66cd8e6ef775c250068c3f08c3ff9c9a6ab35"
repository: "accelerator"
last_updated: "2026-11-09T22:57:39+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# List each key abstraction once on the explore-pr design page

## Summary

The Design view of an `/explore-pr` page showed the same abstraction several
times under "Key abstractions". The design pass and the chapter passes write
in parallel and cannot see each other, so one abstraction is often declared
by the design and by one or more chapters, and the view gave every
declaration its own card. On PR #178's page that was 70 cards for 63
abstractions. The build now merges them by name, so each appears once.

## Changes

- **`scripts/build.py`**: a new `key_abstractions` merges the abstractions
  in `design.json` and every chapter by name, ignoring backticks, into the
  content model's `keyAbstractions`. The first declaration describes the
  abstraction (the design pass's, when it declares one), its collaborators
  are the union of every declaration's, and it lists every chapter that
  declares it, starting with the chapter a design entry names.
- **`assets/page.html`**: the Design view's cards and its collaboration
  diagram render `keyAbstractions`. A card links to every chapter that
  declares its abstraction instead of one. A chapter's own "What it adds to
  the design" section is unchanged.
- **`reference/content-model.md`**: says the Design view merges
  abstractions by name, so a pass may safely declare one that another pass
  also declares.
- **`tests/test_design.py`**: seven tests for the merge: one card for the
  design and a chapter, every declaring chapter listed, names that differ
  only in markup, the first declaration's description, the union of
  collaborators, a design entry's own chapter first, and the design-then-tour
  order of distinct abstractions.

## Context

Found reviewing PR #178 with `/explore-pr`; its page is the one that showed
the duplicates. Decisions are not merged: each carries its own review
control, keyed by its id.

## Testing

- [x] The skill's Python tests pass (94), seven of them new and seen failing
  before the change; the coverage tests pass (29).
- [x] `mise run check` exits 0.
- [x] PR #178's page, rebuilt with this change, renders 63 abstraction cards
  in its Design view, all distinct, and passes the skill's smoke check with
  no errors or overflow.
- [ ] The full `mise run` was not run. None of its tasks reach
  `.claude/skills/`, so it does not exercise this change.

## Notes for Reviewers

- **Merge rule**: the first declaration wins on everything but
  collaborators and chapters, including `status`. PR #178's design pass
  called the scheme type `changed` and its `scheme` chapter called it `new`,
  so the card says `changed`.
- **Not in CI**: the skill's suites run in no `mise` task, so CI does not
  run them. Worth wiring in separately.
- **Diagram limit**: the collaboration diagram still draws only when there
  are 2 to 16 abstractions, so a page as large as PR #178's shows the cards
  without it.
