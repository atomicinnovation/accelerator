---
type: work-item
id: "0175"
title: "Slim the README and split content into a docs/ tree"
date: "2026-06-29T10:28:21+00:00"
author: Phil Helm
producer: refine-work-item
status: done
kind: story
priority: medium
parent: "work-item:0145"
tags: []
last_updated: "2026-06-29T14:46:59+00:00"
last_updated_by: Phil Helm
schema_version: 1
external_id: PP-697
---

# 0175: Slim the README and split content into a docs/ tree

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Phil Helm

## Summary

Reduce the root `README.md` (currently ~886 lines, as of June 2026) to its
essentials — what it is, and how to install and run a first skill — and
relocate the narrative sections into focused `docs/*.md` pages. The slimmed
README links directly into each page; there is no separate `docs/` index.

## Context

Child of 0145 — Documentation Improvements. The README is a single ~886-line
document that is hard for **new users** to navigate, raising onboarding cost
and burying the material they need first. This story implements the agreed
information architecture: the README keeps only "what it is" and "install +
run your first skill"; the conceptual/narrative material moves into a new
`docs/` directory (which does not exist today). The per-skill-family reference
pages are handled separately by sibling 0176; the documentation site is
sibling 0177.

The "load from a local checkout" snippet (README `### Development`, lines
865–871) is folded into the existing root `CONTRIBUTING.md` rather than a new
docs page, and the README links to it.

## Requirements

Keep in the README (above the fold):

- §1 What it is — tagline, logo, visualiser screenshot, one-paragraph pitch.
- §2 Install + run first skill — marketplace add, stable install, `/init`,
  and the research → plan → implement quickstart (collapses the former
  standalone "first time usage" into install).

Relocate into a new `docs/` tree (narrative layer):

- `docs/how-it-works.md` — Philosophy / phase model, with a brief reference
  to the `meta/` directory, plus VCS detection.
- `docs/development-loop.md` — The research → plan → implement loop and work
  items, referencing the `meta/` directory.
- `docs/configuration.md` — Configuration, template management, per-skill
  customisation, custom review lenses.
- `docs/visualiser.md` — The visualiser section in full.
- `docs/internals.md` — "Lifting the lid": the `meta/` directory deep-dive
  and the agents roster.

There is no `docs/` index file — the slimmed root `README.md` is the entry
point and links directly into every docs page.

Cross-cutting:

- Fold the local-checkout instructions into the existing `CONTRIBUTING.md`;
  the README links to it instead of carrying the snippet.
- Add "see the docs" cross-links from the slimmed README to each relocated
  page; the link text matches each page's title.

## Acceptance Criteria

- [ ] The root `README.md` retains only §1 (What it is) and §2 (Install + run
      first skill) — plus, optionally, a short one-paragraph teaser — and the
      links into `docs/`; every other current section has been removed
      (target: under ~150 lines).
- [ ] A `docs/` directory exists containing `how-it-works.md`,
      `development-loop.md`, `configuration.md`, `visualiser.md`, and
      `internals.md` (no `docs/` index file).
- [ ] `docs/how-it-works.md` and `docs/development-loop.md` each reference the
      `meta/` directory.
- [ ] The "load from a local checkout" content lives in `CONTRIBUTING.md` and
      is linked from the README; it no longer appears as a README subsection.
- [ ] No relocated content is lost: every README section in the source→page
      mapping in Technical Notes is present in exactly one destination
      (`docs/` page or `CONTRIBUTING.md`), with none dropped or duplicated.
- [ ] Every `docs/` page is reachable from the root `README.md`, with link text
      matching the page H1 title defined in the Technical Notes mapping.

## Open Questions

- A short one-paragraph "how it works" teaser in the slimmed README is
  permitted at the author's discretion (AC1 allows it); the default is to link
  straight out to `docs/how-it-works.md`. _(Resolved — no longer blocks AC1.)_
- **Two current README sections have no destination in the agreed IA and need a
  home before this story can satisfy "no content lost":** Migrations (~142–170)
  and the Installation section's Prerelease + Compatibility material
  (~841–881; the Development subsection already folds into `CONTRIBUTING.md`).
  Candidate homes: a new `docs/installation.md`, or folding Migrations into
  `docs/configuration.md` / `docs/internals.md`. See the Technical Notes
  mapping.

## Dependencies

- Blocks: 0176 (its `docs/skills/` reference pages and cross-links need the
  `docs/` tree to exist); 0177 (documentation site needs the `docs/` tree to
  publish).
- Related: 0019 — this work supersedes the single-file 9-section structure that
  decision and the 2026-03-15 restructure plan describe.

## Assumptions

- Per-skill-family reference pages are out of scope here and owned by 0176.
- The existing `CONTRIBUTING.md` is the right home for the local-checkout
  instructions.

## Technical Notes

Source README section → destination page (the verification baseline for AC5).
Line ranges are from the June 2026 README (~886 lines) and supersede the
99-line structure in the 2026-03-15 restructure plan. The README cross-link
text matches the page H1 title.

| Current README section (lines)                          | Destination              | Page H1 title          |
|---------------------------------------------------------|--------------------------|------------------------|
| Getting Started (~19–39)                                | stays in README §2       | —                      |
| Philosophy (~41–62) + VCS Detection (~172–192)          | `docs/how-it-works.md`   | How It Works           |
| The Development Loop — narrative (~64–87)               | `docs/development-loop.md` | The Development Loop |
| Configuration (~194–320)                                | `docs/configuration.md`  | Configuration          |
| Visualiser (~548–631)                                   | `docs/visualiser.md`     | Visualiser             |
| The `meta/` Directory (~107–140) + Agents (~802–827)    | `docs/internals.md`      | Internals              |
| Installation → Development (~865–871)                   | `CONTRIBUTING.md`        | (existing)             |
| License (~883–886)                                      | stays in README (footer) | —                      |

Owned by sibling 0176 (out of scope here), relocating to `docs/skills/`: Work
Item Management (~322–364), Remote Work Item Management (~366–508), ADRs
(~510–534), VCS and PR Workflow Skills (~536–546), Review System (~633–671),
Design Convergence (~673–800), and the Development Loop skill listings
(~88–105 → `docs/skills/planning.md`).

**Unassigned (see Open Questions):** Migrations (~142–170) and Installation →
Prerelease + Compatibility (~841–881). These have no destination in the agreed
IA yet; AC5 cannot pass until they do.

## Drafting Notes

- Section-to-page mapping derives from a structural analysis of the current
  README; exact page boundaries may shift during implementation.
- Author inherited from parent 0145.

## References

- Parent: 0145 — Documentation Improvements
- Related: 0019 — README structured around philosophy and development loop
- Prior plan: `meta/plans/2026-03-15-readme-restructure.md`
