---
type: work-item
id: "0176"
title: "Author per-skill-family reference docs under docs/"
date: "2026-06-29T10:28:21+00:00"
author: Toby Clemson
producer: refine-work-item
status: draft
kind: story
priority: medium
parent: "work-item:0145"
tags: []
last_updated: "2026-06-29T10:28:21+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0176: Author per-skill-family reference docs under docs/

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Replace the idea of one monolithic skill-reference section with a per-family
documentation layer: relocate the heavy feature/skill-family sections out of
the README into one focused reference doc per family under `docs/skills/`.

## Context

Child of 0145 — Documentation Improvements. Sibling 0175 slims the README and
moves the narrative sections; this story owns the per-skill-family reference
layer, namespaced under `docs/skills/` to keep it visually separate from the
top-level narrative docs. A "skill family" here is one of the README's existing
per-family groupings (each currently a section with a skills table); the
authoritative set is the seven pages enumerated in Requirements. A single
reference page would be enormous, so each family becomes its own page — making
individual skill families discoverable for users navigating the docs. The
heaviest families are Remote Work Item Management (~143 lines) and Design
Convergence (~128 lines); the source-section mapping is pinned in Technical
Notes. (Requirements is authoritative; figures here are illustrative.)

## Requirements

Author one reference doc per skill family under `docs/skills/`, relocating the
corresponding README content:

- `docs/skills/work-items.md` — Work item management skills and lifecycle.
- `docs/skills/issue-trackers.md` — Remote work item management (Jira & Linear).
- `docs/skills/review-system.md` — The multi-lens review system and lens
  catalogue (the custom-lens subsection links here from
  `docs/configuration.md`).
- `docs/skills/design-convergence.md` — Inventory/gap design-convergence
  workflow.
- `docs/skills/adrs.md` — Architecture decision record skills and lifecycle
  (page title: "Architecture Decision Records (ADRs)").
- `docs/skills/vcs-and-pr.md` — Commit and PR workflow skills.
- `docs/skills/planning.md` — The planning skills (research-codebase,
  research-issue, create-plan, implement-plan, review-plan, stress-test-plan,
  validate-plan, create-note). Draws the skill-catalogue portion of the
  README's Development Loop section; the loop *narrative* stays in 0175's
  `docs/development-loop.md`.

Each page is linked from the root `README.md`, with link text matching the
page title.

## Acceptance Criteria

- [ ] Each skill family in the source→page mapping in Technical Notes has its
      own `docs/skills/` reference page containing that family's relocated
      README content.
- [ ] The README no longer carries the full per-family sections; it links to
      the corresponding `docs/skills/` pages instead.
- [ ] The custom-review-lenses material in `docs/configuration.md` links to
      `docs/skills/review-system.md`.
- [ ] Every per-family page is reachable from the root `README.md`, with link
      text matching the page H1 title in the Technical Notes mapping.
- [ ] No skill described in the original README is dropped: every per-family
      skills table survives in exactly one `docs/skills/` page.

## Open Questions

- _(Resolved)_ Planning skills get a dedicated `docs/skills/planning.md` (a
  seventh family page), consistent with the other families. The loop narrative
  remains in 0175's `docs/development-loop.md`; this page takes the
  skill-catalogue portion of the Development Loop section.

## Dependencies

- Blocked by: 0175 (the `docs/` tree must exist first; this story also edits the
  same root `README.md` and relies on 0175's source→destination mapping to
  define which sections it owns).
- Blocks: 0177 (the documentation site's navigation must expose the
  `docs/skills/` pages this story produces).
- Related: 0145.

## Assumptions

- The README's existing per-family table groupings are the right unit of
  decomposition for the reference layer.

## Technical Notes

Source README section → destination page (the verification baseline for the
relocation criteria and AC5). Line ranges are from the June 2026 README; the
README cross-link text matches the page H1 title.

| Current README section (lines)                    | Destination page                  | Page H1 title                        |
|----------------------------------------------------|-----------------------------------|--------------------------------------|
| Work Item Management (~322–364)                    | `docs/skills/work-items.md`       | Work Items                           |
| Remote Work Item Management (~366–508)             | `docs/skills/issue-trackers.md`   | Issue Trackers (Jira & Linear)       |
| Architecture Decision Records (~510–534)           | `docs/skills/adrs.md`             | Architecture Decision Records (ADRs) |
| VCS and PR Workflow Skills (~536–546)              | `docs/skills/vcs-and-pr.md`       | VCS & PR Workflow                    |
| Review System (~633–671)                           | `docs/skills/review-system.md`    | Review System                        |
| Design Convergence (~673–800)                      | `docs/skills/design-convergence.md` | Design Convergence                 |
| Development Loop skill listings (~88–105)          | `docs/skills/planning.md`         | Planning                             |

The Development Loop section (~64–105) is split: 0175's
`docs/development-loop.md` takes the loop *narrative* (~64–87); this story's
`planning.md` takes the companion + plan-support skill listings (~88–105).

## Drafting Notes

- Family list derives from the current README's section structure; boundaries
  may be adjusted (e.g. merging or splitting a family) during implementation.
- Author inherited from parent 0145.

## References

- Parent: 0145 — Documentation Improvements
- Related: 0175 — Slim the README and split content into a docs/ tree
