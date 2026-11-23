---
type: "work-item"
id: "PP-1000"
external_id: "PP-1000"
aliases: ["draft-rzps4n"]
title: "Sources Rendering in the Visualiser"
date: "2026-11-23T16:52:04+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:PP-868"
blocked_by: ["work-item:PP-995", "work-item:PP-994", "work-item:PP-999", "work-item:PP-998", "work-item:PP-1002", "work-item:PP-1001"]
relates_to: ["work-item:PP-924", "work-item:PP-891", "work-item:PP-897", "work-item:PP-892", "work-item:PP-864", "work-item:PP-936"]
tags: ["research", "visualiser"]
last_updated: "2026-11-23T23:43:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-1000: Sources Rendering in the Visualiser

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As someone reading topic research, I want each source in any topic-research
document that lists Sources to show its reputation tier, its venue, and its
marks as distinct elements, so that I can judge how far to trust a
claim without decoding `— tier-3 (retracted) —` by eye.

The visualiser server reads a document's Sources through the Sources parser
and the frontend renders everything it returns, wherever a topic-research
document is shown: tiers, retracted and withdrawn marks, `also` tiers,
venue, metadata, and the lines it could not read.

## Context

Child of PP-868 — Topic-Research Set Detail Page. Split out of the
topic-research set page (PP-1001) after review: its server work and its
component are separable from the page, and the component applies to a
document wherever that document is shown.

It lands after the set page. By then a topic-research document renders in
the set page's member view, with its Sources as literal markdown. This
story replaces that literal text with structured rendering there.

A **topic-research document** is any document of a topic-research set: the
root manifest and every member document, of whatever kind. Doc type names
the set-level type, topic research; kind names a document's kind within the
set, such as `finding` or `brief`.

A Sources entry reads `- [Title](url) — tier text — venue`, optionally
followed by a nested `key: value` metadata block. Tier text is a base tier
with optional `retracted` and `withdrawn` marks and an `also` list of
further tiers a merged source was cited at. The Sources parser
(PP-995) defines the terms used below: a **rejected tier** is tier
text it could not parse, an **unread line** is a Sources line that is
neither an entry nor a metadata line, and the section's **state** is
absent, declared empty, or present. A **source profile** is where a
finding's sources came from: `web`, `openalex`, or `arxiv`.

## Requirements

- Show each source's tier and its venue as separate elements in any
  topic-research document that lists Sources, of whatever kind and source
  profile. An **element** is its own node in the page, which a test can
  select by itself, and its whole text is its label; the written tier text, with its parentheses and semicolons,
  appears nowhere in a parsed entry. A venue is a named venue or a bare domain, and may be
  absent.
- Render what the Sources parser returns. The server calls it and serves
  each source as structured fields: title, URL, either a parsed tier or
  the rejected tier text as written, venue, and metadata, with the unread
  lines and the section state beside them. The frontend parses no entry text.
- Show the base tier, a retracted mark, a withdrawn mark, and each `also`
  tier as its own element with its own text label: the tier's name,
  `retracted`, `withdrawn`, and `also` followed by the tier's name. An
  unmarked source shows no mark.
- Show a rejected tier's text as written, in place of the tier elements,
  under the label "unrecognised tier", with the entry's title, link, and
  venue still shown.
- Show each entry's title as a link to its URL.
- Show the structured Sources in place: where the literal section sat in
  the body, under one Sources heading, with any section that follows it
  still shown after them.
- Render by the parser's section state, by one rule: the Sources heading
  shows whenever the section is not absent, entries show when there are
  any, the notice shows when there are unread lines, and the empty state,
  reading "No sources found.", shows when the section is declared empty or
  has neither entries nor unread lines.

  | Section state  | Entries | Unread lines | Shows                        |
  |----------------|---------|--------------|------------------------------|
  | Absent         | —       | —            | Nothing, no heading          |
  | Declared empty | none    | none         | Heading, empty state         |
  | Declared empty | none    | some         | Heading, empty state, notice |
  | Present        | some    | none         | Heading, entries             |
  | Present        | some    | some         | Heading, entries, notice     |
  | Present        | none    | some         | Heading, notice              |
  | Present        | none    | none         | Heading, empty state         |
- Show every entry the parser returns, so a source listed twice shows
  twice.
- Show an entry's metadata within that entry: every key and value, in
  written order.
- Show the lines the parser reports as unread in a collapsed notice below
  the sources, reading "1 line in this section could not be read" for one
  line and "N lines in this section could not be read" for more, which
  expands to show each line as written: its exact characters, leading
  spaces kept, with no markdown rendered. Show no notice when there are
  none.
- Render Sources in one new Sources component, used by every view that
  shows a document whose response carries structured sources. Two views
  use it: the set page's member view, where topic-research documents are
  shown, and the shared `LibraryDocView`, which no topic-research document
  reaches once the set page has landed and which uses the component for
  any other doc type whose response carries sources.
- Decide on the server which documents get structured sources: the server
  attaches them for topic-research documents only, and the frontend renders
  them whenever the response carries them, so no view names a doc type.
- Show each source once: where sources are rendered structurally, the
  literal Sources section is not also shown. The server drops it from the
  body it serves, using the line span the Sources parser reports, and
  reads no section boundary of its own.
- Add three visual-regression baselines, one for each of the reference
  design's views 6a, 6b, and 6c. View 6a's document shows all three tiers,
  a retracted source, a withdrawn source, an `also` suffix, a metadata
  block, a source with no venue, a rejected tier, and the collapsed
  unread-lines notice; view 6b's shows the empty state; view 6c's shows
  the notice expanded. Each is taken in the set page, on the rendering
  the design shows. View 6a's replaces the baseline PP-1001 took with the
  Sources region masked. The author accepts each against the approved
  export, and a dated line in Drafting Notes records that.

## Acceptance Criteria

- [ ] Given a finding, a level note, a synthesis, a report, and a brief,
      each of whose Sources carry `— tier-N — <venue>` entries, then in each
      document every source shows its title as a link to the entry's URL,
      its tier as a separate element with its own text label, and its venue
      as text exactly as written, so the three tiers are told apart with
      colour removed; given an entry with no venue, then it shows its tier
      alone.
- [ ] Given `— tier-3 (retracted) —`, then the entry shows `tier-3` and a
      `retracted` mark; given `— tier-3 (withdrawn) —`, then `tier-3` and a
      `withdrawn` mark; given `— tier-2 —`, then `tier-2` and no mark.
- [ ] Given `— tier-3 (also tier-2, tier-1) —`, then the entry shows
      `tier-3`, then `also tier-2` and `also tier-1` as two elements in that
      order, and no mark.
- [ ] Given `— tier-3 (retracted; withdrawn; also tier-1) —`, then the entry
      shows `tier-3`, a `retracted` mark, a `withdrawn` mark, and
      `also tier-1`, as four elements.
- [ ] Given `— tier-1 (retracted) —`, then the entry shows `tier-1` and a
      `retracted` mark, exactly as an entry reading `tier-3 (retracted)`
      does apart from the tier's name, with no sign that it breaks the
      citing-once rule.
- [ ] Given three entries whose second reads `— tier-4 —`, then three entries
      show in order, the second showing `tier-4` as written with its title,
      link, and venue, and no tier, mark, or `also` element; the same holds
      for `— tier-3 (unknown) —`, shown whole. The rejected text carries the
      label "unrecognised tier", so it is told apart from a tier element
      reading the same.
- [ ] Given a finding whose `## Sources` section sits between `## Findings`
      and `## Appendix`, then the served body holds both other sections and
      no line of the Sources span, and the page shows one Sources heading,
      with the structured sources between the other two sections.
- [ ] Given a Sources section reading `None found.`, then the document shows
      an empty state and no source entries; given a topic-research document
      with no Sources section, then it shows neither, and no Sources
      heading. The empty state reads "No sources found.", and view 6b of
      the reference design takes that text from here.
- [ ] Given a Sources section holding only a line of prose, then the
      document shows the Sources heading and the notice, and no empty
      state; given a section holding `None found.` and a line of prose,
      then it shows the heading, the empty state, and the notice; given a
      `## Sources` heading with nothing under it, then it shows the heading
      and the empty state.
- [ ] Given a Sources section holding two entries with a line of prose
      between them, then both entries show, and below them a notice reads
      "1 line in this section could not be read", which expands to show the
      prose as written; given a section with no unread line, then no notice
      shows.
- [ ] Given a Sources section listing the same source twice, then the
      document shows two entries.
- [ ] Given an entry followed by `doi: 10.7717/peerj.4375` and
      `accessed: 2026-10-24`, then that entry shows both keys with their
      values in that order; given a second entry with no block after it,
      then the second shows no metadata.
- [ ] Given a finding whose third source reads
      `— tier-3 (retracted; also tier-2, tier-1) — Example Blog`, then the
      response that carries the finding's sources holds, for that source,
      base `tier-3`, retracted true, withdrawn false, `also` tiers `tier-2`
      then `tier-1`, and venue `Example Blog`, each as its own field,
      alongside its title, URL, and metadata pairs in written order; a
      rejected tier's text and the unread lines are fields too.
- [ ] Given a document of a doc type other than topic research whose body has
      a `## Sources` section holding `- [T](https://a.example/) — tier-1 — venue`, then
      that section renders as a markdown list with the entry text as
      written and no tier elements.
- [ ] Three visual-regression baselines exist, each taken in the set page
      and compared by a spec that runs in the visual-regression suite; view
      6a's replaces PP-1001's masked one. View 6a's document shows all
      three tiers, a retracted source, a withdrawn source, an `also` suffix,
      a metadata block, a source with no venue, a rejected tier, and the
      collapsed unread-lines notice; view 6b's shows the empty state; view
      6c's shows the notice expanded. A dated line in this work item's
      Drafting Notes records the author's acceptance of each against that
      view of the reference design (PP-999) and the commit of the approved
      export it was compared against.
- [ ] Given a finding whose source reads
      `— tier-3 (retracted; also tier-1) —`, loaded at its own URL in the
      set page, then its sources show as separate elements, no tier, mark,
      or `also` element of that entry holds `(`, `;`, or `)`, and the
      string `tier-3 (retracted; also tier-1)` appears nowhere on the page;
      the Sources section is not also rendered as markdown.
- [ ] Given unread lines `### Notes` and `  - doi: 10.1/x`, then the
      expanded notice shows those exact characters, the two leading spaces
      included, and renders no heading and no list.
- [ ] Given `— tier-4 —`, then the response holds that text as written and
      no parsed tier; given a section with one line of prose, then it holds
      that line in its unread lines; given `None found.`, then it reports
      the section declared empty, and given no Sources section, absent.
- [ ] Given a response that carries structured sources for a document of a
      doc type other than topic research, as a test supplies it, then the
      frontend renders them through the same component: it reads the
      payload alone and names no doc type.
- [ ] Given a Sources section with two unread lines, then the notice reads
      "2 lines in this section could not be read", neither line shows until
      it is expanded, and expanding shows both as written in document
      order.

## Open Questions

- How do structured sources reach the frontend (planning-time): on the
  document response, or from a separate request? PP-994 settles the member
  API shape first and leaves room for sources; this story then settles the
  answer at its own planning. Settled already: the served body does not
  keep its raw Sources section.

## Dependencies

- Blocked by: PP-998, set member routes and view, for member documents
  rendered at their own URLs.
- Blocked by: PP-995, the Sources parser, for a document's sources as
  structured entries, each with a parsed tier or the tier text as written,
  and for the section's line span, by which the server drops the literal
  section.
- Closing this story needs the author's acceptance of each baseline
  against the export at the approved commit.
- This story changes the body the document endpoint serves for a
  topic-research document, which the set page (PP-1001) shows.
- Blocked by: PP-994, the set model, for the member API shape this
  story's sources ride on. That shape is settled before this story is
  planned; within the room it leaves, this story owns how sources are
  served.
- Blocked by: PP-999, the reference design, whose view 6 this story
  follows for Sources. The design is approved and its export committed
  before this story is planned; labels and the baseline are taken against
  the export at that approved commit.
- Blocked by: PP-1001, the set page, whose member view this story wires
  the Sources component into and whose rendering its three baselines are
  taken on. Until this story lands, the set page shows a member's Sources
  as literal markdown.
- Blocks: the closure of PP-868 and, through it, PP-22.
- Related, still draft: PP-891 and PP-897 change which tier a source is
  given, not the grammar. PP-924 adds metadata keys to the block this story
  renders.
- Blocked by: PP-1002, the derived set response, which adds the
  visualiser server's dependency on `research`. This story calls the
  Sources parser across that edge and adds no edge of its own.
- Upstream format: the entry and tier grammar come from the citing-once and
  finding-outputter skills by way of the parser; a format change reaches
  this story's fixtures.

## Assumptions

- Sources are rendered structurally only for topic-research documents. Other
  doc types with a "Sources" heading keep their markdown rendering.
- Text of the right shape that breaks the citing-once rule, such as
  `tier-1 (retracted)`, renders through the normal tier elements, with no
  sign that it breaks the rule, because the parser accepts it. "As written"
  is kept for rejected tiers and unread lines.

## Technical Notes

**Size**: M — one server call into the parser, structured sources on the
wire, and one new frontend component, wired into the set page and the
shared `LibraryDocView`, with three baselines taken in the set page.

- `MarkdownRenderer` renders a document body today
  (`cli/visualiser/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx`).
- The visualiser server does not depend on `research` today
  (`cli/visualiser/server/Cargo.toml:24-33`); the dependency lint permits
  the edge (`tasks/lint/crate_dependencies.py:392-397`).
- Chip and badge components exist for labelled elements
  (`components/Chip/`, `components/StatusBadge/`).
- The reference design for the set page shows a finding with tiered sources;
  this story follows it.

## Drafting Notes

- Split from the set page on 2026-11-23 at the author's direction.
- Structured rendering applies to topic-research documents only, decided on
  the server, confirmed by the author on 2026-11-23. A later doc type that
  adopts the grammar opts in on the server with no frontend change; a
  declaration on the doc type is left until a second type needs it.
- Unread lines are shown in a collapsed notice, decided on 2026-11-23: a
  source written in the wrong shape stays visible, not silently dropped.
- A review on 2026-11-23 gave each mark and `also` tier its own labelled
  element, stated per-input outcomes, and put the rendering in one shared
  component. Showing nothing for a document with no Sources section, and
  every metadata key and value in written order, were the drafting agent's
  calls, confirmed by the author on 2026-11-23.
- Tier rendering reached PP-868 from PP-864 on 2026-09-22.
- A third review on 2026-11-23 defined a topic-research document, replaced
  `drafts/url`, a path-rewrite artefact, with `url`, aligned the baseline
  requirement with its criterion, gave the second story to land the set
  page's unmasked baseline, and added criteria for unread lines shown
  literally, the response's rejected tier, unread lines and section state,
  and a frontend that reads the payload alone. The author then confirmed
  what each section state renders, a bare heading showing the empty state,
  and the empty state's text, "No sources found."; that the server drops
  the literal section by the parser's line span; that this story takes
  three baselines, for views 6a, 6b, and 6c; and that the crate edge
  belongs to the derived set response, PP-1002.
- The landing order was fixed on 2026-11-23, at the author's direction:
  this story is blocked by the set page (PP-1001). Its component is wired
  into the set page and its three baselines are taken there, so no
  requirement or criterion depends on which story lands first.
- A re-review on 2026-11-23 added the unrecognised-tier label, a criterion
  on the served body, and the title link. Showing the structured Sources
  in place, and the label's text, are the drafting agent's calls, not yet
  confirmed by the author.

## References

- Parent: PP-868, under the epic PP-22
- Blocked by: PP-995 (Sources parser), PP-994 (set model),
  PP-998 (member routes and view), PP-999 (reference design),
  PP-1002 (derived set response, for the crate edge), PP-1001 (set
  page)
- Related: PP-864 (introduced tier text), PP-892
  (defines the citing-once rule and the `also` suffix), PP-936 (added
  the metadata block this story renders), PP-891, PP-897, PP-924
- Tier-text grammar: `skills/research/outputters/citing-once/SKILL.md`
- Sources entry format: `skills/research/outputters/finding-outputter/SKILL.md`
