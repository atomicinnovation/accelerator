---
type: work-item
id: "0176"
title: "Author per-skill-family reference docs under docs/"
date: "2026-06-29T10:28:21+00:00"
author: Phil Helm
producer: refine-work-item
status: draft
kind: story
priority: medium
parent: "work-item:0145"
tags: []
last_updated: "2026-06-29T15:01:34+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# 0176: Author per-skill-family reference docs under docs/

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Phil Helm

## Summary

Establish a per-skill-family documentation layer under `docs/skills/`: one
focused reference page per family, relocated from the README. Beyond the
relocation, this story adds (a) a **master index** listing every user-invokable
skill, and (b) a consistent **per-skill subsection** within each family page
(what it does / how to use it / advice & guidelines). The grouping keeps the
existing **skill family** framing — pages are *not* renamed to "workflows"
(that term is reserved for the phase process and the existing "VCS & PR
Workflow" page) — and there is **no** standalone page per skill; per-skill
detail lives within the family pages.

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

**Current state (as of the docs branch):** the seven family pages already exist
on disk under `docs/skills/`, created alongside the 0175 split, and the README
already links to them under its **Skills** heading. The relocation criteria
(AC1–AC5) are therefore largely met; this story's active work is the two
additions in Requirements — the master index and the per-skill subsections —
plus a verification pass over the relocation.

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

Beyond the relocation, this story also delivers:

- **Master skill index** — a `docs/skills/index.md` page (H1 "All Skills")
  listing **every user-invokable skill** (46 at the time of writing), grouped by
  the same skill families, each entry linking to its family page (and, where
  practical, to the per-skill subsection anchor). This is the skills landing
  page the 0177 documentation site will navigate to. "User-invokable" means a
  skill not carrying `user-invocable: false` — this excludes the 18 review
  lenses, the 3 review output-formats, and the `browser-executor` / `paths`
  internal config skills.
- **Per-skill subsections** — within each family page, present each skill with a
  consistent, anchored subsection covering: *what it does*, *how to use it*
  (including its `argument-hint`), and *advice & guidelines* specific to that
  skill. Skill name, description, and argument-hint must stay faithful to the
  skill's `SKILL.md` frontmatter (the canonical source) to avoid drift. **No
  standalone per-skill page is created** — the heaviest families already have
  their own family page (e.g. issue-trackers, design-convergence), so per-skill
  detail stays inside the family pages.

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
- [ ] A master index page `docs/skills/index.md` (H1 "All Skills") lists every
      user-invokable skill grouped by family, each linking to its family page;
      it is reachable from the README's **Skills** section.
- [ ] No user-invokable skill is missing from the master index, and no
      non-invokable skill (review lenses, output-formats, `browser-executor`,
      `paths`) is listed in it.
- [ ] Each family page presents its skills with a consistent, anchored
      per-skill subsection (what it does / how to use it / advice & guidelines),
      with name + description + argument-hint matching the skill's `SKILL.md`.
- [ ] No standalone per-skill page exists under `docs/skills/`; the only pages
      are the seven family pages plus `index.md`.
- [ ] Pages retain the "skill family" framing and are not renamed to
      "workflows".

## Open Questions

- _(Resolved)_ Planning skills get a dedicated `docs/skills/planning.md` (a
  seventh family page), consistent with the other families. The loop narrative
  remains in 0175's `docs/development-loop.md`; this page takes the
  skill-catalogue portion of the Development Loop section.
- _(Resolved)_ Keep the **skill family** framing; do not rename the pages to
  "workflows" ("workflow" is already the process term in the docs).
- _(Resolved)_ Provide a single master index of every user-invokable skill at
  `docs/skills/index.md`, rather than scattering the catalogue or omitting it.
- _(Resolved)_ Do **not** create a page per skill; per-skill detail lives as
  anchored subsections within the family pages.
- Should the master index (and per-skill subsections) be **generated** from
  `SKILL.md` frontmatter by a `tasks/` invoke task with a CI consistency check,
  rather than hand-maintained? There is no automated drift check today. (Defer
  to 0177 if it standardises a site build.)

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
