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
blocked_by: ["work-item:PP-995", "work-item:PP-994", "work-item:PP-999", "work-item:PP-998"]
relates_to: ["work-item:PP-1001", "work-item:PP-924", "work-item:PP-891", "work-item:PP-897"]
tags: ["research", "visualiser"]
last_updated: "2026-11-23T21:04:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-1000: Sources Rendering in the Visualiser

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As someone reading topic research, I want each source under a finding,
level note, synthesis, or report to show its reputation tier, its venue, and
its marks as distinct elements, so that I can judge how far to trust a
claim without decoding `— tier-3 (retracted) —` by eye.

The visualiser server reads a document's Sources through the Sources parser
and the frontend renders everything it returns, wherever a topic-research
document is shown: tiers, retracted and withdrawn marks, `also` tiers,
venue, metadata, and the lines it could not read.

## Context

Child of PP-868 — Topic-Research Set Detail Page. Split out of the
topic-research set page (PP-1001) after review: it does not need the
set page, and it applies to a document wherever that
document is shown.

Once member routes (PP-998) land, a member document renders at its own URL through
the shared `LibraryDocView`, with its Sources as literal markdown. This
story replaces that literal text with structured rendering, in the member
view PP-998 provides and in the set page's member view once that
exists. It can land before or after the set page.

A Sources entry reads `- [Title](drafts/url) — tier text — venue`, optionally
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
  profile. An **element** is separately addressable and its whole text is
  its label; the written tier text, with its parentheses and semicolons,
  appears nowhere in a parsed entry. A venue is a named venue or a bare domain, and may be
  absent.
- Render what the Sources parser returns. The server calls it and serves
  each source as structured fields: title, URL, tier, venue, and metadata,
  with the unread lines beside them. The frontend parses no entry text.
- Show the base tier, a retracted mark, a withdrawn mark, and each `also`
  tier as its own element with its own text label: the tier's name,
  `retracted`, `withdrawn`, and `also` followed by the tier's name. An
  unmarked source shows no mark.
- Show a rejected tier's text as written, in place of the tier elements,
  with the entry's title, link, and venue still shown.
- Show an empty state for a Sources section reading `None found.`, and
  nothing at all for a document with no Sources section.
- Show every entry the parser returns, so a source listed twice shows
  twice.
- Show an entry's metadata within that entry: every key and value, in
  written order.
- Show the lines the parser reports as unread in a collapsed notice below
  the sources, reading "N lines in this section could not be read", which
  expands to show each line as written. Show no notice when there are none.
- Render Sources in one component that every view of a topic-research
  document uses: the member view PP-998 provides, and the set page's
  member view once it exists.
- Decide on the server which documents get structured sources: the server
  attaches them for topic-research documents only, and the frontend renders
  them whenever the response carries them, so no view names a doc type.
- Show each source once: where sources are rendered structurally, the
  literal Sources section is not also shown.
- Take the empty state from the parser's section state: declared empty
  shows it, absent shows nothing.
- Add a visual-regression baseline for a document whose Sources show all
  three tiers, a retracted source, an `also` suffix, and a metadata block,
  checked against the reference design.

## Acceptance Criteria

- [ ] Given a finding, level note, synthesis, or report whose Sources carry
      `— tier-N — <venue>` entries, then each source shows its tier as a
      separate element with its own text label alongside its venue, so the
      three tiers are told apart with colour removed; given an entry with no
      venue, then it shows its tier alone.
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
      `retracted` mark, with no warning.
- [ ] Given three entries whose second reads `— tier-4 —`, then three entries
      show in order, the second showing `tier-4` as written with its title,
      link, and venue, and no tier, mark, or `also` element; the same holds
      for `— tier-3 (unknown) —`, shown whole.
- [ ] Given a Sources section reading `None found.`, then the document shows
      an empty state and no source entries; given a topic-research document
      with no Sources section, then it shows neither, and no Sources
      heading. The empty state's text is the one view 6b of the reference
      design gives.
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
      a `## Sources` section holding `- [T](drafts/url) — tier-1 — venue`, then
      that section renders as a markdown list with the entry text as
      written and no tier elements.
- [ ] A visual-regression baseline exists for a document whose Sources show
      all three tiers, a retracted source, an `also` suffix, and a metadata
      block, a withdrawn source, a source with no venue, a rejected tier,
      and the unread-lines notice; a dated line in this work item's Drafting
      Notes records the author's acceptance of it against view 6 of the
      reference design (PP-999) and the revision compared.
- [ ] Given a finding loaded at its own member URL, then its sources show as
      separate elements and the written tier text appears nowhere in an
      entry; the Sources section is not also rendered as markdown.
- [ ] If the set page (PP-1001) landed first: the same finding opened
      in the set page shows the same elements as at its member URL, and this
      story re-takes view 6's baseline there unmasked. If this story lands
      first, this criterion passes on the member URL alone.
- [ ] Given a Sources section with two unread lines, then the notice reads
      "2 lines in this section could not be read", neither line shows until
      it is expanded, and expanding shows both as written in document
      order.
- [ ] Given a brief with a `## Sources` section, then it renders
      structurally, as a finding's does.

## Open Questions

- How do structured sources reach the frontend (planning-time): on the
  document response, or from a separate request, and does the served body
  keep its raw Sources section? Settled with the set model's member API
  shape, before this story is planned.

## Dependencies

- Blocked by: PP-998, set member routes and view, for member documents
  rendered at their own URLs.
- Blocked by: PP-995, the Sources parser, for a document's sources as
  structured entries, each with a parsed tier or the tier text as written.
- Blocked by: PP-994, the set model, for the member API shape this
  story's sources ride on. That shape is settled before this story is
  planned; within the room it leaves, this story owns how sources are
  served and whether the served body keeps its raw Sources section.
- Blocked by: PP-999, the reference design, whose view 6 this story
  follows for Sources.
- Not blocked by the set page (PP-1001), which can land before or
  after. Whichever lands second wires the shared component into the set
  page's member view: if this story lands second, it does.
- Blocks: the closure of PP-868 and, through it, PP-22, together with the
  set page (PP-1001).
- Related, still draft: PP-891 and PP-897 change which tier a source is
  given, not the grammar. PP-924 adds metadata keys to the block this story
  renders.
- New crate edge: this story or the set page (PP-1001) adds the
  visualiser server's dependency on `research`, whichever lands first.
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
wire, and one new frontend component with its baselines.

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

## References

- Parent: PP-868, under the epic PP-22
- Blocked by: PP-995 (Sources parser), PP-994 (set model),
  PP-998 (member routes and view), PP-999 (reference design)
- Related: PP-1001 (set page), PP-864, PP-892, PP-936, PP-891, PP-897,
  PP-924
- Tier-text grammar: `skills/research/outputters/citing-once/SKILL.md`
- Sources entry format: `skills/research/outputters/finding-outputter/SKILL.md`
