---
type: "work-item"
id: "PP-1001"
external_id: "PP-1001"
aliases: ["draft-tjp8ry"]
title: "Topic-Research Set Page"
date: "2026-11-23T16:12:42+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:PP-868"
blocked_by: ["work-item:PP-994", "work-item:PP-999", "work-item:PP-998"]
tags: ["research", "visualiser"]
last_updated: "2026-11-23T21:18:04+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
relates_to: ["work-item:PP-1000", "work-item:PP-923", "work-item:PP-997"]
---
# PP-1001: Topic-Research Set Page

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As someone reading topic research, I want to open a research set in the
visualiser and read every document in it, so that I can follow a subject from
its brief to its reports without leaving the visualiser.

A set-level page represents a whole research set — the manifest, the brief,
the outline, its findings and their level notes, the synthesis, and any
reports — and navigates into each, replacing the shared `LibraryDocView` for
topic-research entries. It is the visible work that closes PP-22.

## Context

Child of PP-868 — Topic-Research Set Detail Page. It is one of PP-868's
six children, built on the set model (PP-994) and the member routes (PP-998). Rendering each
document's Sources is a separate story, PP-1000.

Today the visualiser renders a set's `manifest.md` alone, through the shared
`LibraryDocView`, and no other document in the set can be reached.

A set holds seven document kinds (`manifest`, `brief`, `outline`, `finding`,
`level-note`, `synthesis`, `report`) and its manifest moves through five
statuses (`briefed`, `outlined`, `researching`, `synthesised`, `complete`).
A **reopened** set has no status of its own: it reads `status: researching`
with `primary: synthesis.md`. This is the one definition of reopened used
below.

Member routes (PP-998) leave a member rendering as a plain document at its own URL
through `LibraryDocView`, with a breadcrumb to its set. This story replaces
that for topic research with the set page: persistent set navigation beside
one member shown at a time, at that member's URL. The set's own URL shows
the member that `primary` names. Opening a finding means navigating to it.

Terms used below. The **root** is the manifest, and a **member** is any
other document in the set; the manifest is readable like a member, and the
rules for "the document being read" cover it too. An **outline item** is
one checkbox line under one `## Round N` heading: it owns a question, a
checkbox state, and assigned profiles. A **focus area** is a question,
together with every outline item that asks it; one asked in two rounds is
one focus area with two outline items. A **pair** is one outline item with
one of its assigned profiles. A finding answers every pair of its focus
area that names its profile, in whichever round, so a round-1 `web` finding
shows `web` as researched on a round-3 item asking the same question. A level note's **lineage** is its
filename stem, dash-separated numbers such as `2-1`; lineage order compares
those numbers segment by segment, a parent before its children, so `2-2`
precedes `2-10`. A level note's `follow_ups` is the list of questions it
raised.

The page is built against the reference design that PP-999 produces
and the author approves. Its views are listed in that task's Requirements.

## Requirements

### Navigation

- A topic-research library entry opens the set page, whose navigation links
  to `manifest.md`, `brief.md`, `outline.md`, each finding, `synthesis.md`,
  and each report. A finding's level notes are linked from that finding
  once it, or one of its level notes, is the open document, not from the
  set-level navigation.
- Default to the member named by `manifest.md`'s `primary` (`brief.md`
  before a synthesis exists, `synthesis.md` after). When `primary` is absent
  or names a missing file, default to `brief.md`; when `brief.md` is missing
  too, default to the manifest.
- Show any document of the set when its URL is loaded directly, with the set
  navigation present.
- Show a not-found state, with the set navigation present, for a member URL
  that names no document.

### Set status

- Show the set's status.
- Render a set in every status: a `briefed` set has no outline, an `outlined`
  set has no findings, and a set may have no synthesis and no reports.
- Mark the synthesis stale when its `rounds_covered` is below the manifest's
  `round_count`, or when the set is reopened.
- Show a set-level error state when `manifest.md` has unparseable
  frontmatter or a `status` outside the five, with the set's members still
  listed. At the set's own URL the error state overrides `primary`: the
  brief is the open document, or none when there is no brief. A member URL
  loaded directly still shows that member, with the error state beside it.
- Give every mark and state a text label, worded as in the approved export.

### Outline and focus areas

- Show the outline by round (`## Round N`), each outline item with its
  checkbox state.
- Read an outline item's `— profiles: …` suffix (absent means `web`) as that
  item's assigned profiles, accepting `—`, `–`, and `--` alike before
  `profiles:`.
- Show each of an outline item's assigned profiles in one of three states:
  researched (a finding answers the pair), outstanding (no finding yet), or
  skipped (not in the brief's `source_profiles`). A finding wins: a profile
  the brief omits but that has a finding is researched. A brief with no
  `source_profiles` skips nothing.
- Place findings through the planner's rule in `cli/research`, not a rule of
  the visualiser's own, at two levels. A finding **belongs to a focus area**
  when its question equals the focus area's after Unicode compatibility
  folding and whitespace collapsing. A finding **answers a pair** when it
  belongs to the focus area and its `source_profile` is the pair's profile.
  Both read frontmatter, not filenames, so legacy `<nn>-<slug>.md` findings
  are placed too.
- Show every finding that belongs to a focus area under it, labelled by its
  profile, including one whose profile no outline item of that focus area
  assigns, and all of them when several answer one pair.
- List a finding that belongs to no focus area in an unmatched group. When
  `outline.md` is missing or unreadable, every finding is unmatched.
- Show each finding's `depth` (an unstamped `depth` reads as 1).
- Show a finding whose frontmatter carries a field or value the page does
  not know, other than `kind`, as readable.

### Level notes

- Reach a finding's level notes from that finding, ordered by lineage, each
  showing its `level` and `follow_ups`.
- Request only the open document's body. No level note's body is requested
  until that note is opened; a note's frontmatter may arrive with the set's
  listing.

### Reports

- Show each report's `question`, and mark a report stale when its
  `rounds_covered` is below the manifest's `round_count`. For a report,
  stale means only this.
- While the set is reopened, mark every report "may be out of date: the set
  has been reopened". A report that is also stale shows both marks.
- Show each report's `coverage` (`answered` or `partial`) and render a
  `partial` report's Gaps section.

### Visual regression

- Add a visual-regression baseline for each of the reference design's views
  1 to 5 and 7 to 9, and for view 6 with its Sources region masked. The
  tiered-sources baseline belongs to PP-1000.

### Sources

- If PP-1000 has landed first, render the Sources of every member that
  lists them through its shared component.
- If this page lands first, a member's Sources read as literal markdown, and
  PP-1000 adds the component to this page's member view and its
  criterion for it.

### Server

- Derive on the server, through the `research` crate, what the page shows
  beyond members: the outline's rounds and items, focus areas with the
  findings that belong to them, each pair's state, the unmatched findings,
  the set's status, and the stale and reopened marks. Serve it to the page
  alongside the member listing.
- Add the `research` exports this needs: the planner's question comparison,
  the outline grammar, `SetStatus`, and freshness.
- Choose the set page for a doc type by reading its presentation
  declaration, never its name.

### Unreadable members

- List a member the page cannot place in an unreadable group with its path
  and the reason, one of four: unparseable frontmatter; a `kind` that is not
  one of the six member kinds, which includes `kind: manifest` on a document
  that is not the root; no `kind`, which includes a document with no
  frontmatter at all; or a document whose `kind` does not belong in its
  directory, by the layout in the set contract (a `kind: finding` under
  `reports/`, for example).
- Never fail the page for an unreadable member.

### Live change

- Update the navigation without a reload as members are added or removed,
  raising no toast.
- Update what the page derives from the set without a reload: the status
  and stale marks when the manifest changes, checkbox states when the
  outline changes, and a profile's state when its finding arrives.
- When the document being read changes on disk, show the new content and
  raise the visualiser's external-edit toast, without moving the reader's
  position.
- When the document being read is removed, stay on its URL and show a removed
  state linking back to the set.

## Acceptance Criteria

- [ ] Given a set with a brief, an outline, findings, level notes, a synthesis,
      and reports, when its library entry is opened, then the set navigation
      links to the manifest, the brief, the outline, each finding, the
      synthesis, and each report, and to no level note; when a finding is
      opened, then its level notes are linked.
- [ ] Given `primary: synthesis.md`, then the page opens on the synthesis;
      given `primary: brief.md`, on the brief; given a `primary` naming a
      missing file, or no `primary`, on the brief.
- [ ] Given a `briefed` set, then the page shows its status and its brief, and
      shows for the outline, findings, synthesis, and reports the empty state
      the reference design gives each, without error; given an `outlined`
      set, then it also shows the outline.
- [ ] Given a set in each of the five statuses, then the page shows that
      status's label.
- [ ] Given a set with `status: researching` and `primary: synthesis.md`, then
      the synthesis is marked stale; given a synthesis with `rounds_covered: 2`
      and a manifest with `round_count: 3`, then it is marked stale; given
      `status: synthesised` and equal counts, then it is not.
- [ ] Given a set whose `manifest.md` has unclosed frontmatter, or a `status`
      outside the five, when its library entry is opened, then the page shows
      a set-level error state naming the manifest, still lists the set's
      members, and has the brief as the open document.
- [ ] Given an outline with `## Round 1` and `## Round 2`, then the page shows
      each round's outline items under its round, each with its own checkbox
      state.
- [ ] Given a focus area with `web` and `openalex` findings and an outline
      item assigning `web`, `openalex`, and `arxiv`, in a set whose brief
      lists all three profiles, then the page shows one focus area with both
      findings labelled by profile and `arxiv` as outstanding.
- [ ] Given the same outline item in a set whose brief's `source_profiles`
      omits `arxiv`, then `arxiv` shows as skipped, not outstanding; given
      that set also holds an `arxiv` finding for the question, then `arxiv`
      shows as researched.
- [ ] Given a finding for `openalex` whose question matches an outline item
      assigning only `web`, then the finding shows under that focus area,
      labelled `openalex`.
- [ ] Given outline items suffixed `— profiles: web, arxiv`,
      `– profiles: web, arxiv`, and `-- profiles: web, arxiv`, then each is
      read as assigning `web` and `arxiv`; given an item with no suffix, then
      it is read as assigning `web`.
- [ ] Given an outline item asking a question, and two findings with that
      `question` frontmatter but differing `<nn>` prefixes, then the page shows them under one focus area; given a
      legacy `<nn>-<slug>.md` finding, then it shows under its question's
      focus area.
- [ ] Given a finding whose `question` differs from an outline item's only by
      whitespace or Unicode compatibility form, then it shows under that
      item's focus area; given one that differs by case or punctuation, then
      it does not.
- [ ] Given the same question asked in rounds 1 and 3, ticked in round 1 and
      unticked in round 3, then the page shows one focus area under both
      rounds, with the same findings and each round's own checkbox state.
- [ ] Given a finding whose question no outline item asks, then the page
      lists it in an unmatched group, and the finding stays readable.
- [ ] Given two findings answering one question for one profile, then the
      page shows both under that focus area.
- [ ] This story adds a table of question pairs beside the planner's tests,
      holding at least a whitespace-only difference, a compatibility-form
      difference, a case difference, a punctuation difference, and a
      compatibility-form punctuation mark. The planner's test and the
      visualiser's test both read that one file, and the page places a
      finding under a focus area exactly when the planner reports the pair
      equal.
- [ ] Given a finding with `depth: 3`, then the page shows depth 3; given a
      finding with no `depth`, then it shows depth 1.
- [ ] Given a finding with level notes `1`, `2-1`, `2-2`, and `2-10`, then the
      finding links to all four in that order, each showing its `level` and
      its `follow_ups`; given a finding with no `.levels/` directory, then it
      shows no level notes.
- [ ] Given a report with `rounds_covered: 2` in a set whose manifest has
      `round_count: 3`, then the page lists the report under its question
      and marks it stale; at `rounds_covered: 3` it is not marked.
- [ ] Given a set with `status: researching`, `primary: synthesis.md`, and one
      or more reports, then each report shows
      "may be out of date: the set has been reopened"; given
      `status: synthesised` or `complete`, then no report shows it; given a
      reopened set and a report whose `rounds_covered` is below
      `round_count`, then that report shows both that mark and the stale
      mark.
- [ ] Given a report with `coverage: partial` and a Gaps section, then the
      page shows its coverage and its Gaps; given `coverage: answered`, it
      shows the coverage and no Gaps.
- [ ] Given a member with unparseable frontmatter, then the page lists it in
      an unreadable group with its path and that reason, and every other
      document in the set stays readable.
- [ ] Given a member whose `kind` is not one of the six member kinds,
      `kind: manifest` among them, then it is listed in the unreadable group
      with that reason; given a member with no `kind`, or with no
      frontmatter at all, with the no-`kind` reason; given a `kind: finding` document under `reports/`,
      with the reason that its kind does not belong there. The four reasons
      read differently.
- [ ] Given the document being read changes on disk, then the new content
      appears without a reload, the "External edit detected" toast appears
      naming that document's path, and, the reader having scrolled away
      from the top, the content that was in view stays in view.
- [ ] Given the document being read is removed from disk, then the page
      stays on its URL and shows a removed state linking back to the set.
- [ ] Given documents are added to an open set, then the navigation shows
      them without a reload and raises no toast; given a member that is not
      being read is removed, then it leaves the navigation without a reload
      and raises no toast.
- [ ] Given the URL of the manifest, the brief, the outline, a finding, a
      level note, the synthesis, or a report, when it is loaded directly,
      then the set page shows that document with the set navigation present;
      given a member URL that names no document, then the page shows a
      not-found state with the set navigation present.
- [ ] Given an open set with `arxiv` outstanding for an outline item, when an
      `arxiv` finding for that question is added, then `arxiv` shows as
      researched without a reload; given the manifest's `status` changes on
      disk, then the shown status and stale marks change without a reload;
      given an outline item is ticked on disk, then its checkbox state
      changes without a reload.
- [ ] Given a brief with no `source_profiles`, then no profile shows as
      skipped; given a missing `outline.md` in a set with findings, then
      every finding is listed in the unmatched group; given no usable
      `primary` and no `brief.md`, then the page opens on the manifest.
- [ ] Given a finding with an extra field `state: superseded`, and one with
      an unrecognised `source_profile` value, then each shows under its
      focus area and its body is readable.
- [ ] A visual-regression baseline exists for each of the reference design's
      views 1 to 5 and 7 to 9, and for view 6 with its Sources region
      masked; the author has accepted each against the export, recorded as a
      dated line in this work item's Drafting Notes.
- [ ] If PP-1000 landed first: a finding open in the set page shows
      each source's tier and venue as separate elements, as at its plain
      member URL. If this page lands first: a finding's Sources render as
      literal markdown, and this criterion passes on that.
- [ ] Given a set with one round, two outline items, and three findings,
      then the server's response for the set holds the round, both items
      with their checkbox states and profile states, each finding under its
      focus area, the set's status, and whether the synthesis is stale.
- [ ] The page's marks and states each carry a text label equal to the
      approved export's: the five statuses, stale, the reopened mark,
      researched, outstanding, skipped, the four unreadable reasons,
      unmatched, removed, not-found, and the set-level error.
- [ ] Given a set of 60 findings, 600 level notes, and 10 reports, when the
      set page opens, then it requests no level note's body until that
      note is opened, and it makes the same number of requests as it does
      for a set of one finding and one level note.

## Open Questions

- None of this story's own. It inherits two, each owned elsewhere: the
  member API shape (PP-994) and the member route shape
  (PP-998). The reference design's export is committed under
  `meta/prototypes/`, per PP-999.
- The shape of the derived set response is settled with the member API
  shape, at planning.

## Dependencies

- Blocked by: PP-998, set member routes and view, for a URL per
  member, the member view this page replaces for topic research, and the
  lookup of the member being read.
- Blocked by: PP-994, the set model, for the member API (members with
  parsed frontmatter, a malformed or absent mark, and set-relative paths,
  even under a malformed root), the presentation declaration, and change
  events carrying a member's path.
- Not blocked by Sources rendering (PP-1000), which can land before or
  after this page. Whichever lands second wires the shared Sources
  component into the set page's member view; until then a member's Sources
  read as literal text.
- Two upstream open questions are settled before this story is planned:
  the member API shape (PP-994), which must leave room for the
  derived set response above, and the member route shape (PP-998).
- Blocked by: PP-999, the reference design, approved before this page is
  planned.
- Blocks: the closure of PP-868 and, through it, PP-22, together with
  PP-1000.
- Existing code this page calls across a new crate edge: the planner's
  join, the outline grammar, `SetStatus`, and freshness derivation in the
  `research` crate. This story or PP-1000 adds the visualiser's
  dependency on `research`, whichever lands first; this story adds the
  exports those items need. The Sources parser (PP-995) changes the
  same public-API snapshot, so whichever lands later regenerates it.
- Related, still draft: PP-923 may add finding states. This page shows a
  finding in a state it does not know as readable; rendering new states
  distinctly is a follow-up to raise when PP-923 settles them, and PP-923
  does not yet record it.
- Follow-up, not a blocker:
  `drafts/../PP-997-stable-focus-area-identifiers-in-topic-research-sets.md`
  replaces the question-text join with an explicit identifier.

## Assumptions

- Quarantined files and the run ledger are not shown, so an invalid finding
  and an interrupted `conduct` run are invisible on the page.
- A reworded outline question shows its earlier findings as unmatched, as
  `conduct` sees them, until focus areas carry identifiers.
- A report cannot record same-round additions, so every report on a reopened
  set is marked as possibly out of date, including one written after the
  reopen.
- The fixture size of 60 findings, 600 level notes, and 10 reports is a
  guess, not a measurement of a real set. It is there to show that the
  number of requests on open does not grow with the set.
- "Skipped" is a profile the brief's `source_profiles` omits. The planner
  also skips a profile that is not installed; the page cannot see that.
- The manifest is readable as a document as well as driving the set's
  status and default.

## Technical Notes

**Size**: L — a new page with eight of the design's nine views and part of the ninth, a first dependency from the visualiser on the `research` crate, and
visual-regression baselines for each view.

- Delivery order: the `research` crate edge and the server response first;
  then the page shell (navigation, default member, status, staleness,
  reports, level notes, unreadable members, removed and error states, live
  update); then the outline join (rounds, focus areas, profile states, the
  unmatched group). The story stays whole.

- The visualiser server does not depend on `research` today
  (`cli/visualiser/server/Cargo.toml:24-33`). The dependency lint permits a
  composition root to depend on a domain crate
  (`tasks/lint/crate_dependencies.py:392-397`).
- The planner's join: `NormalisedQuestion::of` in
  `cli/research/src/topic/question.rs`, `Finding::answers` in
  `evidence.rs:102`, and `Warning::UnmatchedFinding` in `plan.rs:111`.
- The outline grammar is parsed in `cli/research/src/topic/outline.rs`; set
  status is `SetStatus` in `lifecycle.rs`; freshness is derived in
  `counts.rs:154-180`.
- `LibraryDocView` resolves a document from the type listing by `type` and
  `fileSlug` (`frontend/src/routes/library/LibraryDocView.tsx:62-85`); the
  set page replaces it for topic research.
- The external-edit toast fires when a change event's path equals the active
  document's path (`frontend/src/api/use-external-edit-toast.ts:32-55`),
  which depends on `useActiveDocRelPath` resolving a member route.
- The status chip ramp for the five statuses already exists
  (`frontend/src/api/status-variant.ts:54-99`).
- No outline, tree, or table-of-contents component exists; `Sidebar`,
  `RelatedArtifacts`, and `LifecycleClusterView` are the nearest.
- Visual-regression specs live in `frontend/tests/visual-regression/`, and
  `frontend/tests/lib/detail-route-slugs.ts` lists the detail routes they
  cover. A full sample set with level notes and reports is at
  `skills/research/research-topic/evals/project/meta/research/topics/llm-agent-planning/`.

## Drafting Notes

- Decomposed from PP-868 on 2026-11-23 after its stress test agreed the
  split.
- Level notes are reached from their finding rather than listed at set level,
  to keep the set page to one row per finding.
- The external-edit criterion names the toast, tightened during
  decomposition. The tier criteria moved to PP-1000.
- A second review on 2026-11-23 defined outline item and focus area,
  separated belonging to a focus area from answering a pair, settled on one
  definition of reopened, and added live update of derived state. The drafting agent's
  calls in it: the manifest is the last fallback when no brief exists, a missing
  outline makes every finding unmatched, and a document with no frontmatter
  is listed as having no `kind`. All of the drafting agent's calls on this
  item were confirmed by the author on 2026-11-23.
- A review on 2026-11-23 added the terms in Context and several small rules
  that were the drafting agent's calls: a finding outranks "skipped", a finding for an
  unassigned profile shows under its focus area, checkbox state is per
  outline item, a stale report on a reopened set shows both marks, an
  unparseable manifest still lists members, and the manifest is navigable.
  It also restored the parent's criterion that changed content appears
  without a reload.
- Sources rendering was split into its own story on 2026-11-23, at the
  author's direction: it needs only the parser, and applies to a document
  wherever it is shown. The reference design still shows tiered sources, so
  the two are designed together.
- The reference design was lifted into its own task, PP-999, on 2026-11-23
  at the author's direction; its approval is recorded there.
- Kept whole on 2026-11-23, at the author's direction, against a review
  suggestion to split the outline join from a flat-findings shell: the
  design is approved as one, and a flat shell is interim UI nobody designed.

## References

- Parent: PP-868, under the epic PP-22 (the topic-research skillset)
- Blocked by: PP-994, PP-998, PP-999
- Follow-up: PP-997 (stable focus-area identifiers)
- Related: PP-1000 (Sources rendering), PP-995 (Sources
  parser), PP-862, PP-863, PP-864, PP-865, PP-867, PP-892, PP-936, PP-923
- Set contract: `skills/research/research-topic/SKILL.md`
