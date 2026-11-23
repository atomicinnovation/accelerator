---
type: "work-item"
id: "PP-999"
external_id: "PP-999"
aliases: ["draft-r0jd36"]
title: "Reference Design for the Topic-Research Set Page"
date: "2026-11-23T17:29:32+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "task"
priority: "high"
parent: "work-item:PP-868"
blocks: ["work-item:PP-1001", "work-item:PP-1000"]
tags: ["research", "visualiser"]
last_updated: "2026-11-23T21:14:48+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-999: Reference Design for the Topic-Research Set Page

**Kind**: Task
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Extend the accelerator visualiser artifact with the topic-research set page
and its Sources component, export it standalone into the repository, and
approve the export, so that the set page and Sources rendering are built
against an agreed design. Sources in the plain member view follow view 6
and get no view of their own.

## Context

Child of PP-868 — Topic-Research Set Detail Page. Lifted out of the
topic-research set page (PP-1001) after review: the design gates the
build, needs none of the other stories, and two stories follow it.

The design lives in a Claude artifact, the accelerator visualiser
prototype. It is the visualiser's design reference and does not yet cover
topic research. The original is at
`https://claude.ai/artifact/7zFH3gKWb3CvcWr2XQf9dX`, artboard
`aef294b19b3d`. The author is converting it to Claude's newer artboard
format at `https://claude.ai/artifact/9zroPpcvUtSZ7yS3ZBGERr`, page
`dcb81aee8d83`; the conversion had not finished on 2026-11-23. The
extension is made in the converted artifact.

The artifact exports in two forms, both present in the earlier export at
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`.
The **standalone** form is one HTML file, about 3 MB, with every script,
stylesheet, and font bundled into it; it names no external host. The
**source** form is an entry page with `src/*.jsx` view sources that loads
React and Babel from `unpkg.com` and must be served over HTTP. This task
commits the standalone form, with the source form beside it for reading.

The set page is persistent set navigation beside one member shown at a time.
A set's manifest moves through five statuses (`briefed`, `outlined`,
`researching`, `synthesised`, `complete`); a reopened set reads
`status: researching` with `primary: synthesis.md`.

## Requirements

- Extend the accelerator visualiser artifact with the set page.
- Show nine views. Views 4, 6, and 8 hold sub-views, lettered below. Each
  view and sub-view opens on its own from an index in the export, and its
  title carries its number and name, as in "6b. None found".
- The views, whose terms are defined in the Context of PP-1001 and
  PP-1000:
  1. **Briefed set**: the empty state for each of the outline, findings,
     synthesis, and reports.
  2. **Outlined set**: the outline by round, every profile outstanding.
  3. **Researching set, mid-round**: profiles researched, outstanding, and
     skipped; a finding for a profile its outline item does not assign; a
     question asked in two rounds.
  4. **Synthesised set**: (4a) the synthesis open, a report marked stale,
     and a `partial` report with its Gaps; (4b) the same set with status
     `complete`, which differs in its status label alone.
  5. **Reopened set**: its synthesis marked stale; its reports marked "may be
     out of date: the set has been reopened"; one report carrying both that
     mark and the stale mark.
  6. **Finding with level notes and tiered sources**: (6a) the finding's
     level notes linked in lineage order, each with its `level` and
     `follow_ups`; all three tiers; a retracted and a withdrawn source; an
     `also` suffix naming two tiers; a metadata block of two keys; a source
     with no venue; a source whose tier text is not a recognised tier, such
     as `tier-4`, shown as written; and the collapsed notice for lines that
     could not be read; (6b) the `None found.` empty state; (6c) the notice
     expanded.
  7. **Unreadable and unmatched**: a member under each of the four unreadable
     reasons (unparseable frontmatter, a `kind` that is not a member kind,
     no `kind`, and a `kind` that does not belong in its directory), and a
     finding in the unmatched group.
  8. **Removed document**: (8a) the state shown when the document being read
     is removed; (8b) the not-found state for a member URL naming no
     document.
  9. **Set-level error**: an unparseable manifest, with members still
     listed.
- Give every mark and state a text label. The three tiers differ in their
  label text, not in colour alone. Where a story already states a label's
  text, the export uses that text.
- Export the artifact in its standalone form, one self-contained HTML file,
  and commit it under `meta/prototypes/<date-slug>-claude-design-prototype`,
  with the source form beside it.
- Record the author's approval of the export on this work item.

Views 1 to 5 and 7 to 9 belong to the set page (PP-1001). View 6 is
shared: the finding and its level notes belong to the set page, its Sources
to Sources rendering (PP-1000).

## Acceptance Criteria

- [ ] The standalone HTML file is committed under
      `meta/prototypes/<date-slug>-claude-design-prototype`; opened from
      disk with networking disabled, it makes no request to any host and
      every view and sub-view renders.
- [ ] The export has an index from which each of the nine views and the
      seven sub-views (4a, 4b, 6a, 6b, 6c, 8a, 8b) opens on its own, each
      titled with its number and name.
- [ ] Each of these shows a visible text label in one or more views: the
      five statuses; stale; the reopened mark; `partial`; researched,
      outstanding, skipped; retracted, withdrawn, `also`; the four
      unreadable reasons; unmatched; removed; not-found; and the set-level
      error.
- [ ] Views 1 to 3 each show every element Requirements lists for them.
- [ ] Views 4 and 5 each show every element Requirements lists for them.
- [ ] Views 6a, 6b, and 6c each show every element Requirements lists for
      them.
- [ ] Views 7 to 9 each show every element Requirements lists for them.
- [ ] In view 6a, every source with a recognised tier shows it as visible
      text, and the text
      differs between `tier-1`, `tier-2`, and `tier-3`, so the tiers are
      told apart in a greyscale rendering.
- [ ] This work item's Drafting Notes carry a dated line recording the
      author's approval of the committed export, its path, and the commit
      that holds the approved export.

## Open Questions

- None.

## Dependencies

- Blocked by: no work item. It can start alongside the set model and the
  Sources parser, once the artifact's conversion to the artboard format has
  finished.
- Blocks: PP-1001, the topic-research set page, and PP-1000,
  Sources rendering in the visualiser. Both are planned only after approval
  is recorded here; the design can be drafted while their requirements are
  still being settled, and is approved once they are.
- External: the artifact is authored in Claude artifacts, outside the
  repository, and the approval is the author's. The committed standalone
  file depends on nothing outside the repository.
- Content: the views draw on the requirements of PP-1001 and
  PP-1000 as they stand at approval. A later change to either that
  alters a view is a follow-up to this task, not a reopening of it. No
  design question in either story is open: unread lines are shown, in
  view 6.
- Contract: the two stories cite the views by number and take label wording
  from the export, so numbering and wording are fixed once approved. Their
  baselines are accepted against the export at the approved commit.
- Known sources of later follow-ups: PP-923 (finding states) and PP-924
  (metadata keys).

## Assumptions

- The author both produces and approves the design, so approval is a
  deliberate sign-off and not an independent review.
- The upgraded artboard-format artifact still offers the standalone export
  the earlier one did.

## Technical Notes

**Size**: M — nine views, several of them dense, in an artifact that does
not yet cover topic research; no production code.

- The two stories that follow the design state the behaviour each view must
  show: PP-1001 for the page, PP-1000 for sources.

## Drafting Notes

- Lifted from the set page on 2026-11-23 at the author's direction.
- The export's form and path were supplied by the author on 2026-11-23. The
  standalone form was confirmed self-contained by inspecting the May export:
  no external `src` or `href`, and no host named but the SVG namespace. It
  was not opened in a browser.
- Both artifact links are recorded without their share keys.
- A review on 2026-11-23 grew the design from seven views plus two states
  to nine numbered views, and folded in the states the two stories render
  that no view showed. "The author" throughout is the work item's author,
  who also owns the artifact; "my reading" is the drafting agent's.

## References

- Parent: PP-868, under the epic PP-22
- Related: PP-1001 (set page), PP-1000 (Sources rendering)
- Artifact, converted: `https://claude.ai/artifact/9zroPpcvUtSZ7yS3ZBGERr`
- Artifact, original: `https://claude.ai/artifact/7zFH3gKWb3CvcWr2XQf9dX`
- Earlier export, as a model of both forms:
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`
