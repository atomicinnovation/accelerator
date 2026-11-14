---
type: "pr-description"
id: "217"
title: "Check explore-pr's content and draw its figures with diagram skills"
date: "2026-11-14T15:48:25+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/217"
pr_number: 217
tags: ["explore-pr", "diagrams", "skills"]
revision: "c866b44e94688878c8586d51c676487ce3fef00e"
repository: "accelerator"
last_updated: "2026-11-14T15:48:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Check explore-pr's content and draw its figures with diagram skills

## Summary

An `/explore-pr` page could be wrong without anyone knowing: a citation
could point at a line that had moved, and the build passed with warnings.
Its figures were the page's own approximations of UML, drawn by hand-written
layout that only that page could use.

This PR does two things. It makes the build check what the page claims and
fail when a claim does not hold. And it moves every figure onto a new set of
repo-local diagram skills: a shared `diagram-kit` and one `draw-<type>`
skill per notation, each usable alone, with `/explore-pr` as their first
consumer.

It is stacked on #211 and is large: 231 files, about 27,000 lines added.
Most of that is new skills, their tests, and one vendored layout engine.

## Changes

### explore-pr checks its own content

- **Citations and anchors.** Every `where`, test name and chapter or stop
  named in prose is checked against the diff. Each stop and ledger row
  quotes a piece of the line it sits on, so a moved line is caught;
  `build.py --reanchor` moves them when the quote has one new home.
- **The build fails on defects.** It exits 1 while a warning remains. A
  `note:` is for what could not be verified and does not fail it.
- **Verdicts survive a moved head.** Review records are stamped with the
  head they were given at, and lines rewritten since are shown as unread.
- **Three depths.** Quick, Standard and Deep, each stating what it costs in
  subagents and lines read before anything is spawned.
- **A shorter skill.** `SKILL.md` is held under 360 lines by a test, with
  each principle set out once in `reference/content-model.md`.

### A diagram kit and 22 notations

- **`diagram-kit`**: measures text, lays a diagram out with ELK, and draws
  it as themed, interactive SVG. Layout runs in Node when a page is built;
  the renderer runs in Node and in the browser.
- **One skill per notation**, each with a spec reference, an example, tests
  and a `notation.js` the kit loads:
  - Structure: class, object, component, package, deployment, C4,
    domain model, context map, dependency graph.
  - Behaviour: sequence, state machine, activity, flowchart, decision tree.
  - Data: entity-relationship, data flow with trust boundaries, byte-field,
    railroad.
  - Tabular and scaled: matrix, tree, timeline. These draw themselves: a
    matrix and a tree are HTML, so they reflow as tables and text do.
  - Block diagram, for what no notation fits.
- **`draw-diagram`**: chooses the kind by the question the reader has and
  hands over to that kind's skill. Its guide sets each kind against its
  neighbours. `/explore-pr`'s briefs point at it.
- **Checks and remarks.** Each notation reports what is wrong with a spec,
  and remarks on what a reviewer should not have to find by eye: a
  dependency cycle, an unreachable state, a flow across a trust boundary.
- **Legends.** Every notation lists the symbols it can draw and what each
  means. A legend under the figure, closed until opened and headed by the
  kind of diagram, shows the ones the diagram uses, drawn by the kit's own
  renderer, with the rest behind a switch.
- **A browser bundle.** `bin/diagram.js bundle` carries the kit, every
  notation and the layout engine into a page that must draw a diagram it
  could not know when it was built.

### explore-pr on the kit

- **Every figure is drawn by the kit** except `compare` (two figures side
  by side) and raw `svg`. The page's own layout, router and renderers for
  `flow`, `sequence`, `states`, `tree`, `matrix` and `timeline` are gone.
- **Figures appear only in context.** The "In pictures" view is removed.
  The smoke check visits and photographs each view that draws a figure.
- **The Design view's abstraction diagram is a class diagram.** A
  collaborator may say how it is related (`implements`, `extends`, `owns`,
  `holds`, `uses`), and each is drawn as its UML relationship.
- **Custom views can draw any kit diagram** from state they compute, with
  `api.drawInto` or `api.diagram`. The build carries the layout engine only
  in a page whose views ask for one.
- **Layout.** A view uses the width of the screen while prose keeps a
  measure of 72 characters. A drawing wider than half the row takes the
  whole row, and shrinks to fit before it scrolls. A stop's wide figure
  sits across the row above its code.

## Context

There is no work item for this. It continues the explore-pr work in #211,
which is this PR's base.

It was also run for real against PR #188. That run found four defects in
explore-pr, each fixed here in its own commit:

- A file over 4,000 lines kept only five lines around each change, so
  correct citations into the rest of it were reported as not found. The
  limit is now 20,000 lines.
- Backticks in a diagram label were drawn literally.
- The design pass and the chapter passes used the same ids for some
  abstractions and decisions.
- Two lines of the reading-pass brief were stale.

## Testing

- [x] `explore-pr`: 240 pytest tests pass, and `ruff format` and
  `ruff check` are clean.
- [x] `diagram-kit`: 393 `node --test` tests pass.
- [x] The 22 notations: 379 `node --test` tests pass. Each notation's
  example is checked, laid out, and tested for a legend entry for
  everything it draws.
- [x] `mise run check` exits 0.
- [ ] `mise run` (the full local CI mirror) was running when this was
  written.
- [x] A page of all 22 examples, with legends, passes the kit's browser
  check: text inside its box, nothing overlapping, tracing.
- [x] The browser bundle lays out a class, a sequence and an activity
  diagram identically to the build, and was run in headless Chrome.
- [x] A real page for PR #188 builds with no warnings and passes the smoke
  check; seven stops and three ledger rows were checked against the code by
  hand.
- [ ] The evals in `explore-pr/evals/evals.json` were updated and not run.

## Notes for Reviewers

**Existing explore-pr workspaces will not build** until their content is
rewritten. They have no anchors, and may use kinds the page no longer
draws:

- `flow` is now `block-diagram`, or better a notation that fits.
- `sequence` and `states` are `sequence-diagram` and `state-machine`, in a
  different shape.
- A tree's items take `link` where they took `where`.

The build names each of these in a warning.

**None of these skills' tests run in CI.** `mise` does not know about them;
only `ruff` reaches the Python under `.claude/skills`. They are run by hand:
`uv run pytest tests` in `explore-pr`, and `node --test "tests/*.test.js"`
in the kit and in each notation.

**Vendored files.** `diagram-kit/vendor/` holds elkjs 0.12.1 (1.6 MB,
EPL-2.0 OR GPL-3.0-or-later) and four font files (IBM Plex Sans and
JetBrains Mono, OFL-1.1), with versions and checksums in its README. Please
check that this is acceptable and whether the third-party attribution lane
should cover it.

**Known weaknesses.**

- One kit test, the headless-Chrome page check, failed twice in a full run
  and passed on eight reruns. It looks like load from several Chrome
  instances at once.
- 23 new skill descriptions now load in every session in this repository.
- Some ELK layouts wander, and a cyclic diagram can lay out oddly.
- The C4 diagram is recognisable, not conformant.
- The legend's wording for each symbol is new and worth a read by someone
  who knows the notations.
- The switch for unused legend symbols uses the CSS `:has()` selector.
- A page whose custom view draws diagrams carries the layout engine, which
  adds about 1.9 MB.

**Where to start.** `diagram-kit/reference/notations.md` describes what a
notation is, and `draw-diagram/reference/choosing.md` what each is for.
`draw-data-flow-diagram` is a small, typical notation. The explore-pr side
is easiest to follow by commit.
