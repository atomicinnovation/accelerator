---
type: "pr-description"
id: "261"
title: "[PP-868] Plan the set model, sources parser and library links"
date: "2026-11-24T12:14:41+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-868"
parent: "work-item:PP-868"
relates_to: ["work-item:PP-994", "work-item:PP-995", "work-item:PP-996", "work-item:PP-998", "work-item:PP-1000", "work-item:PP-1002", "work-item:PP-1003"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/261"
pr_number: 261
tags: ["planning", "research", "work-items", "visualiser"]
revision: "0860ba06dd23c841ad0f45de5482c2bfbdb98e36"
repository: "accelerator"
last_updated: "2026-11-24T12:14:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [PP-868] Plan the set model, sources parser and library links

## Summary

This PR researches and plans the first three of PP-868's children: PP-994
(the generic set model), PP-995 (the Sources parser) and PP-996 (library
links by document `id`). Research settled open decisions in five child work
items and raised one new bug, PP-1003. Each plan was reviewed until it
reached COMMENT. Only `meta/` and the Linear sync baseline change; no code,
skill or test is touched.

Stacked on #268, which split PP-868 into its children.

## Changes

- **Research**: one codebase research document each for PP-994, PP-995
  and PP-996, under `meta/research/codebase/`.
- **Plans**: one implementation plan each, under `meta/plans/`, all
  `draft`. PP-994 has 10 phases, PP-995 and PP-996 have 4 each.
- **Reviews**: one plan review each, under `meta/reviews/plans/`. PP-994
  took four passes to reach COMMENT, PP-995 two, and PP-996 was COMMENT
  from the first.
- **Decisions** recorded in the children's work items, at the author's
  direction:
  - PP-994: a set type declares a **shape**, its root file plus member
    slots (`findings/*.md`, `screenshots/*.png`, and so on). Files that fit
    no slot are ignored, which keeps hand-placed `prototype-full/` and
    `assets/` trees out of design inventories. Rows carry the set's `id`
    rather than its slug. Configs with nested doc roots are refused.
  - PP-996: links key on the frontmatter `id`, falling back to the stem,
    because slugs are shared within a doc type. One resolver matches `id`,
    then slug, then stem, and one builder serves all eight link sites.
  - PP-998: library URLs name documents by `id`. A design inventory's URL
    becomes its full directory name, and members sit beneath it by their
    path within the set.
  - PP-1000: the server removes the literal Sources section with
    `SourcesSection::parse` and `LineSpan::remove_from` on the same body,
    so only the parser decides where a line ends. This includes CRLF
    bodies.
  - PP-1002: carries a contract test between `research::topic::layout` and
    PP-994's topic-research shape.
- **New bug, PP-1003**: the related response never reads `relates_to`,
  and only resolves links to work items. In `meta/`, 839 documents carry a
  non-empty `relates_to` and 368 have a `target` naming another doc type.
  PP-994 keeps to the fields read today and leaves this bug to PP-1003.
- **Sync baseline**: `.accelerator/state/integrations/linear/last-sync.json`
  is updated after PP-1003 was pushed to Linear.

## Context

- Parent: `meta/work/PP-868-topic-research-set-detail-page.md`
- Plans: `meta/plans/2026-11-24-PP-994-generic-set-model-indexing-and-api.md`,
  `meta/plans/2026-11-24-PP-995-sources-parser-in-the-research-crate.md`,
  `meta/plans/2026-11-24-PP-996-library-links-by-document-id.md`
- New bug: `meta/work/PP-1003-related-items-ignore-relates-to.md`

## Testing

- [ ] `mise run check`: not run. Only `meta/` and a sync-state JSON file
      change, and no checked component reads either.
- [ ] Plans not yet validated against code. Each plan's success criteria
      run when it is implemented.

## Notes for Reviewers

- ⚠️ **PP-994's plan is XL**, not the L its work item states. Making doc
  types declared data (Phases 3–5) touches about 640 `DocTypeKey`
  references across about 45 files, of which about 90 are production
  `match` arms. The plan recommends a separate enabling work item for
  Phases 3–5, parented to PP-868. That split is not made here, and PP-994's
  size is unchanged.
- ❓ Should Phases 3–5 become their own work item before PP-994 starts?
  Phase 2, which refuses nested doc roots, could ship first under either
  item.
- PP-994's decisions are the ones to check most closely: the shapes table
  and the switch from slug to `id`. PP-996, PP-998 and PP-1000 rely on
  both.
- PP-1003 stays `draft`. Its open question is whether
  `corpus::linkage` allows every `relates_to` pair found in `meta/`.
