---
type: "work-item"
id: "PP-999"
external_id: "PP-999"
aliases: ["draft-r0jd36"]
title: "Reference Design for the Topic-Research Set Page"
date: "2026-11-23T17:29:32+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "ready"
kind: "task"
priority: "high"
parent: "work-item:PP-868"
blocks: ["work-item:PP-1001", "work-item:PP-1000"]
tags: ["research", "visualiser"]
last_updated: "2026-11-24T00:06:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-999: Reference Design for the Topic-Research Set Page

**Kind**: Task
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Extend the accelerator visualiser prototype with the topic-research set page
and its Sources component, export it standalone into the repository, and
approve the export, so that the set page and Sources rendering are built
against an agreed design. The design shows Sources once, in its view of a
finding with tiered sources; the plain member view that PP-998 provides
follows that view and gets no design view of its own.

## Context

Child of PP-868 — Topic-Research Set Detail Page. Lifted out of the
topic-research set page (PP-1001) after review: the design gates the
build, needs none of the other stories, and two stories follow it.

The design lives in a Claude artifact, the accelerator visualiser
prototype, called **the prototype** below. It is the visualiser's design
reference and does not yet cover topic research. It is at
`https://claude.ai/artifact/7zFH3gKWb3CvcWr2XQf9dX`, artboard
`aef294b19b3d`, and the extension is made there.

The prototype exports in two forms, both present in the earlier export at
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

- Extend the prototype with the set page, keeping the views it already
  holds. Those views are carried along unverified: the index of fourteen
  covers this task's additions alone.
- Name each lettered sub-view as the export chooses; the two stories cite
  a rendering by its number, not its name.
- Show nine views. Views 4, 6, and 8 are groupings: each holds lettered
  sub-views and has no rendering of its own. That leaves fourteen
  renderings: 1, 2, 3, 4a, 4b, 4c, 5, 6a, 6b, 6c, 7, 8a, 8b, and 9. Each opens
  on its own from an index in the export, and its title carries its number
  and name, as in "6b. None found". The two stories cite a rendering by
  that number.
- The views, whose terms are defined in the Context of PP-1001 and
  PP-1000:
  1. **Briefed set**: the empty state for each of the outline, findings,
     synthesis, and reports.
  2. **Outlined set**: the outline by round, each outline item with its
     checkbox state, every profile outstanding.
  3. **Researching set, mid-round**: ticked and unticked outline items;
     profiles researched, outstanding, and skipped; each finding's `depth`;
     a finding for a profile that no outline item of its focus area
     assigns; a question asked in two rounds.
  4. **Synthesised set**: (4a) the synthesis open, with the navigation
     listing each report under its `question` with its coverage label,
     `answered` or `partial`, and one report marked stale; (4b) the same
     set with the `partial` report open, its Gaps section rendered in the
     report; (4c) the set of 4a with status `complete`, which differs from
     4a in its status label alone.
  5. **Reopened set**: its synthesis marked stale; its reports marked "may be
     out of date: the set has been reopened"; one report carrying both that
     mark and the stale mark.
  6. **Finding with level notes and tiered sources**: (6a) the finding's
     level notes `1`, `2-1`, `2-2`, and `2-10` linked in that lineage
     order, each with its `level` and `follow_ups`; all three tiers; a retracted and a withdrawn source; an
     `also` suffix naming two tiers; a metadata block of two keys; a source
     with no venue; a source whose tier text is not a recognised tier, such
     as `tier-4`, shown as written under the label "unrecognised tier";
     and the collapsed notice for two lines that could not be read; (6b)
     the empty state, reading "No sources found.", for a section the
     document declares empty with `None found.`; (6c) the notice
     expanded.
  7. **Unplaced and unmatched**: a member in the unplaced group under each
     of its four reasons (unparseable frontmatter, a `kind` that is not a
     member kind, no `kind`, and a `kind` that does not belong in its
     directory), each entry a link; and a finding in the unmatched group.
  8. **Removed document**: (8a) the state shown when the document being read
     is removed, with its link back to the set; (8b) the not-found state
     for a member URL naming no document.
  9. **Set-level error**: an unparseable manifest, the error state naming
     the manifest, members still listed, and the brief as the open
     document.
- Show the persistent set navigation in every view, the removed,
  not-found, and error states included.
- Give every mark and state a text label. The three tiers differ in their
  label text, not in colour alone.
- Word the labels by one rule: where PP-1001 or PP-1000 already states a
  label's text, the export uses that text; the export decides every other
  label, and the two stories then take it from the export.
- Export the prototype in its standalone form, one self-contained HTML file,
  and commit it in the directory
  `meta/prototypes/<date-slug>-claude-design-prototype`, with the source
  form in the same directory. The slug carries the export's date and time
  in the form of the earlier export's, `2026-05-21-015231`.
- Record the author's approval of the export on this work item.

Views 1 to 5 and 7 to 9 belong to the set page (PP-1001). View 6 is
shared: the finding and its level notes belong to the set page, its Sources
to Sources rendering (PP-1000).

## Acceptance Criteria

- [ ] The standalone HTML file is committed under
      `meta/prototypes/<date-slug>-claude-design-prototype`; opened from
      disk with networking disabled, it makes no request to any host and
      every view and sub-view renders: it shows the content listed for it,
      with no error in the browser console.
- [ ] The same directory, at the same commit, holds the source form: its
      entry page and its `src/` view sources, exported from the same
      revision of the prototype as the standalone file.
- [ ] The export has an index of fourteen entries (1, 2, 3, 4a, 4b, 4c, 5,
      6a, 6b, 6c, 7, 8a, 8b, 9), from which each opens on its own, titled with
      its number and name.
- [ ] Each of these shows a visible text label in one or more views: the
      five statuses; stale; the reopened mark; `answered` and `partial`;
      researched, outstanding, skipped; retracted, withdrawn, `also`; the
      unplaced and its four reasons; unmatched; removed; not-found; the set-level
      error; the four empty states of view 1; the Sources empty state; the
      unrecognised tier; and the unread-lines notice. This list is the
      full extent of "every mark and state".
- [ ] These labels read as the two stories state them: view 5 shows "may be
      out of date: the set has been reopened"; view 6a's collapsed notice
      reads "2 lines in this section could not be read"; its unrecognised
      tier carries the label "unrecognised tier";
      view 6b's empty state reads "No sources found.";
      and view 6a shows `retracted`, `withdrawn`, and `also` followed by a
      tier's name, each as its own label.
- [ ] Every view and sub-view shows the set navigation.
- [ ] Views 1 to 3 each show every element Requirements lists for them.
- [ ] Views 4a, 4b, 4c, and 5 each show every element Requirements lists
      for them.
- [ ] Views 6a, 6b, and 6c each show every element Requirements lists for
      them.
- [ ] Views 7, 8a, 8b, and 9 each show every element Requirements lists for
      them.
- [ ] In view 6a, every source with a recognised tier shows it as visible
      text, and the text
      differs between `tier-1`, `tier-2`, and `tier-3`, so the tiers are
      told apart in a greyscale rendering.
- [ ] This work item's Drafting Notes carry a dated line that records four
      things: the author's approval of the committed export, the export's
      path, the commit that holds it, and the commits of PP-1001 and
      PP-1000 that it was approved against. At that date the latest review
      of each of those two stories has the verdict APPROVE or COMMENT.

## Open Questions

- Awaiting the author's confirmation, and stated as settled in
  Requirements until then: the views show the set navigation, checkbox
  states, `depth`, an `answered` report, and each report's `question`;
  and the export directory's slug takes the earlier export's date form.

## Dependencies

- Blocked by: nothing. It can start alongside the set model and the
  Sources parser.
- Blocks: PP-1001, the topic-research set page, and PP-1000,
  Sources rendering in the visualiser. Both are planned only after approval
  is recorded here; the design can be drafted while their requirements are
  still being settled, and is approved once they are. A story's
  requirements are settled when its latest `/review-work-item` verdict is
  APPROVE or COMMENT; its status does not change, since it cannot be
  planned until the design is approved. Open questions the stories settle
  at planning do not hold approval: the member API shape, the member route
  shape, and how sources reach the frontend. The order is therefore: the
  stories' requirements settle, the design is approved against them, and
  the stories are then planned.
- External: the prototype is authored in Claude artifacts, outside the
  repository, and the approval is the author's. The committed standalone
  file depends on nothing outside the repository.
- Content: the views draw on the requirements of PP-1001 and
  PP-1000 as they stand at approval. A later change to either that
  alters a view is a follow-up to this task, not a reopening of it. The
  last open design question in either story, whether unread lines are
  shown, is settled: they are shown, in view 6.
- Contract: the two stories cite the views by number and take label wording
  from the export, so numbering and wording are fixed once approved. Their
  baselines are accepted against the export at the approved commit.
- Known sources of later follow-ups: PP-923 (finding states), PP-924
  (metadata keys), and PP-997 (focus-area identifiers, which replace the
  join behind views 3 and 7). Views 6 to 8 also show states the two
  stories take from other draft items: the rejected tier and unread lines
  from the Sources parser (PP-995), the unplaced reasons, and
  every other derived state, from the derived set response
  (PP-1002), and the not-found member URL from member routes
  (PP-998). A change there that reaches a view is a follow-up too.

## Assumptions

- The author both produces and approves the design, so approval is a
  deliberate sign-off and not an independent review.
- The prototype still offers the two export forms it gave in May 2026.

## Technical Notes

**Size**: M — nine views, several of them dense, in a prototype that does
not yet cover topic research; no production code.

- The two stories that follow the design state the behaviour each view must
  show: PP-1001 for the page, PP-1000 for sources.

## Drafting Notes

- Lifted from the set page on 2026-11-23 at the author's direction.
- The export's form and path were supplied by the author on 2026-11-23. The
  standalone form was confirmed self-contained by inspecting the May export:
  no external `src` or `href`, and no host named but the SVG namespace. It
  was not opened in a browser.
- The prototype's link is recorded without its share key.
- A review on 2026-11-23 grew the design from seven views plus two states
  to nine numbered views, and folded in the states the two stories render
  that no view showed. "The author" throughout is the work item's author,
  who also owns the prototype.
- A second review on 2026-11-23 stated one rule for label wording and a
  criterion for the labels the stories fix; added a criterion for the
  source form; and tightened views 6a, 8a, and 9. It also added to the
  views what PP-1001 renders and no view listed: the set navigation,
  checkbox states, `depth`, an `answered` report, and each report's
  `question`. Adding those, and the slug's date form, are the drafting
  agent's calls, not yet confirmed by the author.
- The conversion of the prototype to Claude's artboard format was dropped
  on 2026-11-23, at the author's direction: the design is made in the
  original prototype, so the task waits on nothing.
- Views 4, 6, and 8 were settled as groupings on 2026-11-23, at the
  author's direction. A report's Gaps were settled the same day as part
  of the report, shown when it is the open member, which split view 4 into
  three: the synthesis open, the `partial` report open, and the `complete`
  set. That leaves fourteen renderings. PP-1001 takes baselines for twelve
  of them and PP-1000 for three, view 6a being shared.
- The approval trigger was settled on 2026-11-23, at the author's
  direction: a review verdict of APPROVE or COMMENT on each of the two
  stories, with their commits recorded on the approval line.
- A re-review on 2026-11-23 found that PP-1000 takes an unrecognised-tier
  label from view 6a that this task did not require. The label is now
  required, with its text, "unrecognised tier", fixed in PP-1000. A later
  change that alters a view is raised as a new task under PP-868, which
  commits a new dated export; the affected story re-accepts its baselines
  against it.

## References

- Parent: PP-868, under the epic PP-22
- Related: PP-1001 (set page), PP-1000 (Sources rendering)
- The prototype: `https://claude.ai/artifact/7zFH3gKWb3CvcWr2XQf9dX`
- Earlier export, as a model of both forms:
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`
