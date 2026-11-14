---
type: "pr-description"
id: "211"
title: "Draw explore-pr graphs at right angles and trace them by press"
date: "2026-11-12T18:54:50+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/211"
pr_number: 211
tags: ["explore-pr", "skills"]
revision: "a34ae5acc9404c37d1f8c403f18c907f90eb857a"
repository: "accelerator"
last_updated: "2026-11-12T18:54:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Draw explore-pr graphs at right angles and trace them by press

## Summary

The diagrams on an `/explore-pr` page were hard to follow once they got busy.
Curved arrows overlapped, ran behind boxes, and could not be followed to their
ends. This PR routes every arrow as horizontal and vertical runs with rounded
corners, around boxes rather than behind them. Pressing boxes now highlights
their arrows, and each box in the abstraction diagram names its language
construct.

## Changes

- **`assets/graph-routes.js`** (new): a pure router.
  - `plan` spreads each box's arrows along its bottom (out) and top (in) edges,
    ordered by where the other end sits.
  - Each horizontal run gets its own track in the gap between rows, ordered to
    keep crossings down.
  - An arrow that skips a row drops through a free gap between the boxes it
    passes.
  - An arrow to a box in its own row goes down, across and up.
  - An arrow pointing back up the diagram climbs a lane to the left of every
    box.
  - Ports within 48px of lining up are snapped straight, in order and 8px
    apart.
  - `draw` places the tracks in gaps the caller sizes with `gapHeight`, then
    rounds the corners.
  - `orderRows` reorders a diagram's rows to untangle crossings, for the
    diagrams the page builds itself.
- **`assets/graph-focus.js`** (new): highlights the arrows of every box pressed.
  - Arrows leaving a traced box are blue and arrows arriving are amber. Their
    other ends stay lit and everything else dims.
  - Pressing a box again (click, tap, or Enter/Space when it has focus) drops
    it; Esc clears the lot.
  - There is no hover tracing, so it works the same on a touch screen.
- **`assets/page.html`**:
  - **Flow figures:** each one plans its routes before it lays out rows, so
    each gap grows to fit its tracks. The module map and sequence-diagram
    self-loops use the same router.
  - **Arrowheads:** a fixed size, starting where the line stops. Edge labels
    are drawn after every arrow, so they sit on top.
  - **Boxes:** each box is a focusable trace toggle. A box with a `where` or
    `chapter` opens it from its own ↗ control, no longer from the box. That
    control's hover background starts at its divider and sits under the box's
    border.
  - **Abstraction diagram:** it arranges its rows and gives every box its kind.
    Unchanged boxes get a grey border, which the legend labels "unchanged".
- **`scripts/build.py`**:
  - Inlines both new models beside the coverage model, through a slot renamed
    from `explore-pr:coverage` to `explore-pr:models`.
  - Collects each collaborator's kind into `collaboratorKinds`.
  - Warns about a collaborator that is neither an abstraction nor given a kind.
- **`scripts/smoke.js`**: new browser checks.
  - Pointing at or focusing a box traces nothing. Pressing boxes adds to and
    removes from the trace, and Esc clears it.
  - The ↗ control navigates.
  - Labels sit above arrows, and borders sit above link backgrounds.
  - Every arrow is right-angled, straight into its head, clear of every box,
    and shares no run with another.
  - It reports the number of crossings.
- **`reference/content-model.md`** and **`SKILL.md`** describe the routing,
  pressing to trace, the ↗ control, `{ "name", "kind" }` collaborators, and
  the new assets and checks.
- **Tests**:
  - `tests/graph-routes.test.js` (13) and `tests/graph-focus.test.js` (12) are
    node suites, run from pytest through `test_graph_routes.py` and
    `test_graph_focus.py`.
  - New build tests cover collaborator kinds, the warning, and inlining both
    models in both page layouts.
  - New page-shell tests check that both graph renderers use the router and
    the ↗ control, that only the abstraction diagram rearranges its rows, and
    the "unchanged" legend entry.

## Context

Found while reviewing PR #181 with `/explore-pr`. Its abstraction diagram had
arrows crossing behind boxes and several boxes labelled "existing" where their
kind belonged. Every change here was checked on that page as it was made.

## Testing

- [x] The skill's Python tests pass (114) and its node suites pass (54). Each
  new test was seen failing before the change it covers.
- [x] `mise run check` exits 0.
- [x] PR #181's page, rebuilt with this change, builds with no warnings and
  passes the smoke check. On that page:
  - every arrow is right-angled, straight into its head, clear of every box
    and shares no run;
  - pressing traces, Esc clears and ↗ navigates.
- [x] Each new probe was seen failing before its fix. The arrow probe was also
  run against a build with the old curved arrows, and it flagged them.
- [x] Checked by eye at 3× zoom: the overview's before/after figure, the whole
  abstraction diagram with two boxes traced, and the ↗ control under a real
  mouse hover.
- [ ] The full `mise run` was not run. None of its tasks reach
  `.claude/skills/`, so it does not exercise this change.

## Notes for Reviewers

- **Interaction change**: pressing a box used to open its code or chapter. It
  now toggles its trace, and the ↗ strip on the box's right opens the link.
  Pressing empty space no longer clears a trace, so a stray press doesn't lose
  a selection. Esc does, and on a touch screen you press each box again.
- **Crossings are reduced, not eliminated**:
  - Track order is a greedy insertion. Only the abstraction diagram, which the
    page builds, reorders its rows.
  - Authored figures keep their nodes' order, because captions can refer to
    positions. The content model now asks authors to order rows to avoid
    crossings.
  - On PR #181 the overview's before/after figure has no crossings and the
    abstraction diagram has 20.
- **New build warning**: a workspace whose collaborators are bare names that
  aren't abstractions now warns until they are given a kind. On PR #181 two of
  the eleven warnings were duplicates: `update_op` drawn beside
  `LinearClient::update_op`, and the same for `resolve_state`.
- **Renamed slot**: a workspace copy of `assets/page.html` taken before this
  change still has `<!-- explore-pr:coverage -->`. Its build would carry none
  of the models, the coverage model included, so it needs the slot renamed.
- **Unverified**: module-map labels are not measured, so a long one could run
  under its ↗ strip. PR #181 has no module map.
- **Not in CI**: as before, the skill's suites run in no `mise` task.
