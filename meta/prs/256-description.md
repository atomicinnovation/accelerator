---
type: "pr-description"
id: "256"
title: "Open explore-pr links in place, keep the scroll on Back and mark verdicts"
date: "2026-11-22T22:00:20+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/256"
pr_number: 256
tags: ["explore-pr", "skills", "review", "visualisation"]
revision: "684d416e02934adcb523c1344d09fd95692bcde5"
repository: "accelerator"
last_updated: "2026-11-22T22:00:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Open explore-pr links in place, keep the scroll on Back and mark verdicts

## Summary

Reviewing with the pages `explore-pr` builds raised three problems:

- Following a link to a decision or invariant from the design or the tour
  lost the reader's place: Back always landed at the top of the page.
- On a phone, the sticky filter panel on each catalogue sat under the top
  bar, and an item opened from a link had its top hidden behind both.
- No link said whether the item it names still wanted the reviewer's
  verdict.

This PR opens those items in place, remembers where each view was
scrolled, measures the sticky bars instead of assuming their height, and
marks every link to an item that takes a verdict.

## Changes

- **Items open in place**:
  - A link to a decision, abstraction, invariant, risk or finding opens its
    card without leaving the page. On a screen wider than 760 px the card is
    a popover beside the link; on a phone it unfolds below the link's
    paragraph.
  - The card carries its review control and an "Open in Decisions →" (or
    Findings, etc.) link to its catalogue. Esc, Close, the link again or,
    for a popover, a click elsewhere closes it.
  - Links to stops and chapters still navigate, and links that open a
    catalogue already filtered still do.
- **Every item has an address** (`assets/item-addresses.js`):
  `#<view>/<item id>`, such as `#decisions/dec-cap-depth` or
  `#tour-claims/stop-claims-s1`. Opening one lands the item just below the
  top bar and any filters, outlined for a moment. Every link to an item now
  carries its address, which replaces the `data-scroll-*` attributes and
  their click handler.
- **Back returns to where you were**:
  - Each history entry remembers its scroll position, and Back or Forward
    restores it. A new view opens at its top.
  - The restored position is held for up to 1.5 s while code slices and
    diagrams finish laying out, and released as soon as the reader scrolls,
    taps or types.
- **Sticky bars are measured**:
  - The top bar's real height drives every sticky offset (`--top-h`, set
    from a `ResizeObserver`), and a catalogue's filter panel is measured
    too (`--filters-h`). Items scroll to just below both.
  - On a phone the top bar keeps to one line, with the title shortened, and
    a catalogue's filters fold into one row naming how many are set and how
    many items are shown.
- **Verdict marks** (`assets/verdict-marks.js`):
  - Every link to a stop, chapter, decision, invariant, finding or ledger
    row starts with a mark: an empty ring until the reviewer gives a
    verdict, then a filled dot in that verdict's tone carrying ✓, ?, ✕, ↻
    or –, so it reads without colour. A stop whose lines were all read
    elsewhere is marked read.
  - Marks appear in prose, in a stop's evidence strip, in the chapter
    strips, the side rail (replacing its status dots), the chapter pager,
    the readiness conditions, the code notes in All files and the Tests
    catalogue. The readiness gates' outstanding items are now links, with
    marks.
  - Marks follow every verdict as it is given, wherever it is given,
    without redrawing the view.
  - The module also holds the verdict vocabulary of every review control,
    which moves out of `page.html`.
- **Peeked cards fit a phone**:
  - A peek drops an abstraction's coloured status edge, which had sat over
    the first letter of every line. The edge rules now use `:where()` so a
    context can override them.
  - An invariant's control flows beneath its text, in a peek and in the
    invariants catalogue at phone width. It had left the text a fifth of
    the width.
  - A peeked risk shows as a card, not as one row of the catalogue's table.
  - Pinning tests are listed one per line. A test name is a button, which
    lays out as a block, so the comma after a long name could wrap onto a
    line of its own.
- **Custom views**: `api.md` output gets the new links and marks, and
  `api.refreshMarks(element)` paints the marks in HTML a view adds after
  `mount`. Until then they stay hidden, rather than showing a verdict that
  may be wrong.

## Context

- Follows PRs #252, #254 and #255, which restructured the review deck and
  added its file navigator. The three problems were found reviewing pages
  built with that deck.
- `reference/page.md` gains a "Links and where they land" section and rows
  for the two new modules; `reference/custom-views.md` documents the link
  and mark behaviour of `api.md`.

## Testing

- [x] Written test first: node suites for `item-addresses` and
  `verdict-marks`, the `prose-references` suite updated for the new link
  shapes, and smoke probes that failed before each page change.
- [x] Skill suites pass: 347 pytest and 228 node tests
  (`uv run --with pytest python -m pytest -q .claude/skills/explore-pr/tests`,
  `node --test .claude/skills/explore-pr/tests/*.test.js`).
- [x] `uvx ruff format --check` is clean. `uvx ruff check` reports the same
  40 findings as on `main`, none in touched lines.
- [x] `scripts/smoke.js` passes on pages built from PR #250 and from PR
  #234 (the latter from its published deck's content), with no errors and
  no sideways scroll at 400 px. Its new probes check:
  - a decision named in the design opening in a popover that stays in view,
    a verdict given there marking every link to it, and Esc and a click
    elsewhere closing it;
  - Back returning a view to within 2 px of where it was scrolled, and a new
    view opening at its top;
  - every link to an item that takes a verdict carrying a painted mark, the
    readiness gates linking each outstanding item, and the rail marking each
    chapter;
  - at phone width: the top bar on one line; the filters folding, unfolding
    and sticking below it; an item opened by its address landing below both
    bars; a peek unfolding below its paragraph and closing again; no card in
    a peek drawing its own edge; an item's text keeping at least 75% of the
    width in every kind of peek and in the invariants catalogue; and a comma
    in a list staying on the line of the item before it.
  The probes for a stop's evidence and for a chapter's findings now expect a
  peek, and a probe that throws reports its error instead of an empty
  result.
- [x] A hosted page built from PR #234 was reviewed on desktop and on a
  phone; the phone-layout fixes in the last commit came from that review.
- [ ] `mise run` was not run: none of its tasks covers `.claude/skills/`, and
  it reformats in place.
- [ ] Real iOS Safari: the checks ran in headless Chrome only, so the notch
  insets and scroll memory under Safari's elastic scrolling are untested.
- [ ] A screen reader on the marks and peeks.

## Notes for Reviewers

- Most of the change is in `assets/page.html`. The pure rules (addresses,
  the verdict vocabulary, what a mark says) live in the two new modules,
  where their tests are.
- ⚠️ A link to a finding now opens it in place. It used to open Findings
  filtered to it; the peek's "Open in Findings →" link still does.
- ⚠️ A custom view that adds linked HTML after `mount` shows no marks on it
  until it calls `api.refreshMarks(element)` or a verdict changes.
- Scroll positions are held in memory, so they survive Back and Forward but
  not a reload, which lands on the address's item or at the top. Each
  history entry is keyed through `history.replaceState`; where a host
  refuses history state, the page lands at the top.
- The popover is positioned in page coordinates, so it scrolls with the
  page, and closes when the window is resized.
- `ExplorePR.registerKind` still adds to the same verdict vocabulary, now
  held by `verdict-marks.js`.
