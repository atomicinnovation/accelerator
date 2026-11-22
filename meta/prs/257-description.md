---
type: "pr-description"
id: "257"
title: "Show everything an explore-pr stop shows in one strip, and list chapter items one line each"
date: "2026-11-22T23:38:04+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/257"
pr_number: 257
tags: ["explore-pr", "skills", "review", "visualisation"]
revision: "b7c376b5a4008da42b37671ce5a92a283aaf4582"
repository: "accelerator"
last_updated: "2026-11-22T23:38:04+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Show everything an explore-pr stop shows in one strip, and list chapter items one line each

## Summary

PR #256 let every decision, invariant and finding open in place. Reviewing
pages built with it raised the next layer of problems in the guided tour:

- A stop's "Introduced here" cards repeated in full what the strip above it
  already opened in place, so stops were long.
- A chapter's sections were drawn four ways: invariants as cards with inline
  controls, risks and tests as tables, findings as one-line links.
- Domain terms lived only in a glossary at the top of each chapter, out of
  sight at the stops that use them.
- A stop's notes looked inconsistent: three sat in panels, "Why it is this
  way" did not.
- Verdict marks sat off the line: filled ones about 4 px below it.

This PR gathers everything a stop shows into one strip, terms included,
lists every chapter item one line each, and seats marks on the text.

## Changes

- **One strip per stop**, now titled "What it shows":
  - It names the decisions, abstractions, invariants and risks the stop
    carries, then the domain terms it uses as compact chips that wrap under
    a "Terms" label.
  - Each item opens in place. A "new here" badge marks the stop where the
    tour shows it first.
  - The "Introduced here" cards are gone. A decision's verdict is given in
    its popover or in the Decisions catalogue; the stop shows its mark.
- **Domain terms found at stops** (`assets/stop-terms.js`, new):
  - A stop shows a term when its code names the term's `code` identifier as
    a whole word, or its text (title, role, why, risk, checks and focus
    notes) uses the term, in any case or the plural.
  - A term an abstraction in the same strip already names is left out.
  - Each term knows the first stop in tour order that shows it, and has an
    address in the Domain terms catalogue.
  - Code is read from the diff, which a hosted page loads after its first
    draw, so the strips repaint in place once it arrives.
- **Chapter pages**:
  - The opening strip names the chapter's terms beside what it introduces
    and decides; the "Terms this chapter introduces" glossary is removed.
  - Invariants, Risks, Findings and Tests are each a list, one line per
    item, each opening in place. Invariants and findings keep their verdict
    marks, and an invariant's verdict buttons move into its popover. Tests
    show what each proves.
  - Tests and terms get popovers of their own, with "Open in Tests" and
    "Open in Domain terms". Their catalogue rows gain ids, so those links
    land on them.
  - A risk without an `id` gets a key from its place in the catalogue, so
    it too can open in place.
- **One table of where items live** (`ITEM_HOMES` in `page.html`): each
  kind's label, catalogue route, element id and catalogue name. It replaces
  three overlapping tables (`CARRIED`, `PEEK_HOMES`, `PEEK_FINDING`).
- **Stop notes**: "Role in the design", "Why it is this way", "If this is
  wrong" and "Check" share one panel style, told apart by their edge
  colour.
- **Marks and badges**:
  - A mark is 1 em, and its glyph is drawn by `::after` from a
    `data-glyph` attribute, so the mark has no text of its own to align by.
    As an inline grid, a filled mark had aligned by its glyph and sunk below
    the line.
  - Marks and "new here" badges are centred on the text's capital letters,
    through a new `--cap-centre` token (0.35 em for IBM Plex Sans).
  - The strip labels are plain text again, which restores the ellipsis on
    long labels.
- **Docs**: `reference/content-model.md` replaces the principle that each
  decision is set out in full at its first stop. `reference/reading-pass-brief.md`
  tells reading passes a stop need not restate what it carries nor define
  a term it uses. `reference/page.md` describes the new tour and the new
  module.

## Context

- Follows PR #256, which opened items in place, remembered scroll on Back
  and added verdict marks. Every change here came from reviewing a hosted
  page built with it.

## Testing

- [x] Written test first: a node suite of 14 tests for `stop-terms`, and
  smoke probes that failed before each page change. Eight shell-source tests
  that described the old layout now describe the new one.
- [x] Skill suites pass: 348 pytest and 242 node tests
  (`uv run --with pytest python -m pytest -q .claude/skills/explore-pr/tests`,
  `node --test .claude/skills/explore-pr/tests/*.test.js`).
- [x] `uvx ruff format --check` is clean. `uvx ruff check` reports the same
  40 findings as on `main`, none in touched lines.
- [x] `scripts/smoke.js` passes on pages built from PR #250 and PR #234,
  with no errors and no sideways scroll at 400 px. Its new probes check:
  - a stop's notes sharing one panel style, and its terms wrapping side by
    side;
  - a stop holding no cards or controls but its own, and badges exactly
    where the tour first shows each item;
  - every term a stop's text uses appearing in its strip, with none
    twinned with an abstraction there;
  - a chapter's invariants, risks, findings and tests listed one line each,
    each opening in place, its terms in its opening strip and no glossary;
  - test and term popovers keeping their text at full width on a phone;
  - every mark as tall as its text, and every mark and badge centred on
    the capitals within 1.5 px, with marks both open and filled (the probe
    records a verdict on every marked item, then restores them), measured
    against the font's real capital height;
  - a verdict given in the Decisions catalogue marking the stop that first
    carries the decision.
- [x] A hosted page built from PR #234 was reviewed after each change; the
  mark alignment took three rounds of that review.
- [ ] `mise run` was not run: none of its tasks covers `.claude/skills/`, and
  it reformats in place.
- [ ] Real iOS Safari and a screen reader: the checks ran in headless
  Chrome only.

## Notes for Reviewers

- ⚠️ A decision's verdict control is no longer drawn in the tour itself,
  only in its popover and in the Decisions catalogue. This changes a stated
  principle of the content model.
- ⚠️ Finding terms is heuristic. A deck that defines common words as terms
  (PR #234 defines "preview" and "run") shows them at many stops, and a stop
  that uses a synonym shows nothing. Checked by eye on two decks only.
- `--cap-centre` is tuned to IBM Plex Sans. A different body font needs it
  changed; the smoke check measures the real capital height, so a mismatch
  fails it.
- A risk without an `id` is keyed by its place in the catalogue, so
  reordering risks moves its address. Risks take no verdict, so nothing
  saved depends on it.
- `.finding-list` is renamed `.item-list`, now that it lists every kind of
  chapter item.
