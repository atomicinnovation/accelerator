---
type: work-item
id: "0145"
title: "Documentation Improvements"
date: "2026-06-22T23:41:03+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: epic
priority: medium
source: "note:2026-06-22-ideas-backlog"
tags: []
last_updated: "2026-06-22T23:41:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-166
---

# 0145: Documentation Improvements

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Improve the project's documentation, including breaking up the monolithic
README into smaller focused docs and building a dedicated documentation site.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Documentation currently centres on a
single 886-line README. At that size it is hard to navigate, raises the
onboarding cost for new users, and buries the per-skill-family reference
material so it is poorly discoverable. Splitting it into focused docs and
publishing them addresses navigation, onboarding, and discoverability.

## Requirements

Three strands of work, each owned by a child work item below:

- Slim `README.md` to its essentials and relocate the narrative sections into
  a focused `docs/` tree (0175).
- Author the per-skill-family reference docs under `docs/`, replacing a single
  monolithic skill-reference page (0176).
- Build a documentation site that publishes the `docs/` tree (0177).

Acceptance criteria for each strand are defined on its child work item; the
epic's criteria below are the roll-up done-definition.

### Child work items

- 0175 — Slim the README and split content into a docs/ tree
- 0176 — Author per-skill-family reference docs under docs/
- 0177 — Stand up a documentation site for the docs/ tree

## Acceptance Criteria

- [ ] `README.md` is reduced to §1 (What it is) and §2 (Install + run first
      skill) plus links into `docs/`, with the narrative sections relocated to
      the `docs/` files named in Technical Notes (0175).
- [ ] The per-skill-family reference docs named in Technical Notes
      (`work-items`, `issue-trackers`, `review-system`, `design-convergence`,
      `adrs`, `vcs-and-pr`, `planning`) exist under `docs/skills/`, each linked
      from the root README, with the corresponding README sections removed
      (0176).
- [ ] The "load from a local checkout" content lives only in the root
      `CONTRIBUTING.md` and is linked from the README; no `docs/contributing.md`
      is created.
- [ ] A documentation site is reachable at a documented URL (or CI artefact)
      and renders every file in the `docs/` tree (0177).

## Open Questions

- What documentation-site tooling / generator should be used?
- What is the target information architecture once the README is split?

## Dependencies

Internal child ordering (a pipeline — content first, then publish):

- 0175 establishes the `docs/` tree and index; it has no blockers.
- 0176 is blocked by 0175 (its `docs/skills/` reference pages and cross-links
  need the `docs/` tree to exist).
- 0177 is blocked by 0175 and 0176 (it publishes the tree they produce;
  scheduling it earlier would ship an empty site).

Other couplings:

- 0175 requires an edit to the root `CONTRIBUTING.md` (the "load from a local
  checkout" fold) so the README link target exists.
- Relates to prior decision 0019 and `meta/plans/2026-03-15-readme-restructure.md`
  — the split should align with (or explicitly supersede) those.

- Blocked by: None (external).
- Blocks: None (external).

## Assumptions

- The three child items are complementary and sequential (split content,
  author the reference layer, then publish it).
- A static-site generator over the `docs/` markdown tree is sufficient; no
  dynamic docs platform is required.

## Technical Notes

### Target information architecture (agreed 2026-06-29)

The README split follows a narrative spine in the README plus a `docs/` tree.

**Stays in `README.md` (above the fold):**

- §1 What it is — tagline, logo, screenshot, one-paragraph pitch.
- §2 Install + run first skill — marketplace add, stable install, `/init`,
  and the research → plan → implement quickstart (the former standalone
  "first time usage" is collapsed into install).

**`docs/` narrative layer (owned by 0175):**

- `docs/how-it-works.md` — philosophy / phase model (refs `meta/`) + VCS
  detection. (README ~41–62, ~172–192)
- `docs/development-loop.md` — R→P→I loop + work items (refs `meta/`).
  (README ~64–105)
- `docs/configuration.md` — config, templates, per-skill, custom lenses.
  (README ~194–320)
- `docs/visualiser.md` — visualiser. (README ~548–631)
- `docs/internals.md` — "lifting the lid": `meta/` deep-dive + agents.
  (README ~107–140, ~802–827)

There is no `docs/` index file; the slimmed root `README.md` is the entry point
and links directly into every docs page.

**`docs/skills/` per-skill-family reference layer (owned by 0176) — replaces a
single monolithic "skill reference" page:**

- `docs/skills/work-items.md` (README ~322–364)
- `docs/skills/issue-trackers.md` — Jira & Linear (README ~366–508)
- `docs/skills/review-system.md` (README ~633–671); custom-lens text in
  `configuration.md` links here.
- `docs/skills/design-convergence.md` (README ~673–800)
- `docs/skills/adrs.md` (README ~510–534)
- `docs/skills/vcs-and-pr.md` (README ~536–546)
- `docs/skills/planning.md` — planning skills (README Development Loop
  skill listings ~88–105; the loop narrative stays in `development-loop.md`)

**Cross-cutting decisions:**

- The "load from a local checkout" snippet (README `### Development`,
  ~865–871) folds into the existing root `CONTRIBUTING.md`; README links to
  it. No new `docs/contributing.md`.
- `meta/` is referenced early (how-it-works, workflow) with the full deep-dive
  deferred to `docs/internals.md`.
- Documentation-site tooling is greenfield (no mkdocs/Docusaurus/VitePress
  today); owned by 0177.

Current `README.md` is 886 lines; heaviest sections to relocate are Remote
Integrations (~143), Design Convergence (~128), Configuration (~127),
Visualiser (~84). Relates to prior README-structure decisions (0019,
`meta/plans/2026-03-15-readme-restructure.md`).

## Drafting Notes

- Treated as a single epic per the user's instruction, with the two source
  sub-bullets captured as child candidates under Requirements rather than as
  separate work items.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
