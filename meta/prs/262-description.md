---
type: "pr-description"
id: "262"
title: "[PP-995] Parse a research document's Sources section in the research crate"
date: "2026-11-24T14:08:30+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-995"
parent: "work-item:PP-995"
relates_to: ["work-item:PP-868", "work-item:PP-1000", "work-item:PP-994"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/262"
pr_number: 262
tags: ["research", "visualiser", "sources", "tier-text"]
revision: "9a21593857ee958f3c6240b0e4bd20a25fec1f52"
repository: "accelerator"
last_updated: "2026-11-24T14:08:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [PP-995] Parse a research document's Sources section in the research crate

## Summary

Adds `research::topic::sources_section`, a parser that reads the `## Sources`
section of a finding or level note into structured cited sources, each with
its title, URL, tier text, venue and metadata block. It also reports the lines
it could not read and the span of lines the section occupies, so a caller can
cut the body around the section without counting lines itself. The
visualiser's Sources rendering (PP-1000) is the first consumer; this PR ships
the library only, with no caller yet.

## Changes

- `TierText` (`sources_section/tier_text.rs`) parses tier text such as
  `tier-3 (retracted; withdrawn; also tier-2, tier-1)` into a base `Tier`,
  `retracted` and `withdrawn` marks and the `also` tiers. It checks shape and
  canonical order only, so `parse(s)?.to_string() == s` for every accepted
  input; anything else is refused as `NotTierText`, carrying the whole input.
- `SourcesSection::parse` reads the first exactly named `## Sources` heading
  up to the next level-one or level-two heading, and yields one of three
  states: `Absent`, `DeclaredEmpty` (only `None found.`, no entries) or
  `Present`.
- `CitedSource` reads `- [title](url) — tier — venue`. The title ends at the
  rightmost `](` that still yields an entry, so titles may hold brackets and
  venues may hold links or the `—` separator. A tier that is not tier text
  keeps its entry as `TierReading::Rejected` with the text as written.
- Metadata block lines (`  - key: value`, lower-snake keys) attach to the
  entry above. Blank lines keep that adjacency; prose, an unread link line or
  `None found.` breaks it, and a detached block line is reported unread.
- `UnreadLine` reports every non-blank line the parser could not read, with
  its line number, as written and in order.
- `LineSpan` runs from the heading to the first line after the section,
  trailing blank lines included. `split` returns the text before and after the
  section as written. Line numbering and byte offsets share one rule in
  `sources_section/lines.rs`, so the two cannot disagree; CRLF documents read
  as their LF twins.
- The public-API snapshot gains 118 lines, all under
  `research::topic::sources_section`; nothing existing changes.
- PP-1000's server step now names `LineSpan::split` instead of
  `remove_from`, so the structured Sources render where the literal section
  sat.

## Context

- Work item: `meta/work/PP-995-sources-parser-in-the-research-crate.md`
- Plan: `meta/plans/2026-11-24-PP-995-sources-parser-in-the-research-crate.md`
- Validation:
  `meta/validations/2026-11-24-PP-995-sources-parser-in-the-research-crate-validation.md`
  (result: pass)
- Stacked on #261 (PP-868's child plans), which is stacked on #268.

## Testing

- [x] `mise run test:unit:cli --filter 'binary_id(research::tier_text) |
      binary_id(research::sources_section)'` — 39 of 39 pass
- [x] `mise run public-api:check` — the snapshot is current
- [x] `mise run cli:check` — rustfmt and clippy clean
- [x] `mise run` exits 0 end to end (~267s) — run during validation at the
      code tip; the later commits touch only `meta/`
- [x] Every PP-995 acceptance criterion maps to a named test against public
      exports only, including the citing-once worked example read whole and
      every tier text the citing-once rule produces
- [x] Edge cases: CRLF bodies, a lone or doubled `\r`, sections that run to
      the end of the file with and without a final newline, a heading with
      nothing under it, duplicate sections and near-miss headings, and
      duplicate keys and entries
- [x] Parser run over the eval variants' findings under
      `skills/research/research-topic/evals/variants/*/meta/research/topics/`
      (this workspace has no `meta/research/topics/`)
- [ ] A real finding whose `## Sources` section is followed by another `## `
      section, cut through `split` — all six eval
      files' sections run to the end of the file, so only unit tests cover a
      non-empty after-text today

## Notes for Reviewers

- `TierText` deliberately accepts text of the right shape that breaks the
  citing-once rule, such as `tier-1 (retracted)` or `tier-3 (also tier-3)`.
  Enforcing the rule is the writer's job; the reader's job is to round-trip
  what was written.
- `split` takes any `&str`. Given a body other than the one parsed it returns
  a wrong cut rather than an error; it cannot panic, since every offset lands
  after a `\n` or at the end of the body. The precondition is in its doc.
- The reader does not know about fenced code blocks: a `## ` line or an
  entry-shaped line inside a fence counts as structure. The Sources grammar
  does not define fences.
- Whichever of this PR and PP-1002's lands second regenerates the public-API
  snapshot with `mise run public-api:update`.
