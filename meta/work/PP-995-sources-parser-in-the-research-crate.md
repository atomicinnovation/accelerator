---
type: "work-item"
id: "PP-995"
external_id: "PP-995"
aliases: ["draft-4xrpk0"]
title: "Sources Parser in the Research Crate"
date: "2026-11-23T16:12:42+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "task"
priority: "high"
parent: "work-item:PP-868"
tags: ["research", "visualiser"]
last_updated: "2026-11-23T21:27:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
blocks: ["work-item:PP-1000"]
relates_to: ["work-item:PP-892", "work-item:PP-864", "work-item:PP-1001", "work-item:PP-924", "work-item:PP-891", "work-item:PP-897"]
---
# PP-995: Sources Parser in the Research Crate

**Kind**: Task
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As a maintainer of topic research, I want one reading of the Sources grammar
in `cli/`, so that every Rust consumer — the visualiser first — renders
sources and their tiers the same way.

Two parsers deliver it: the **Sources parser**, which reads a document's
Sources section into structured entries, and the **tier-text parser**, which
it calls for each entry's tier text. No Rust code parses either today: the
grammar is defined in the citing-once and finding-outputter skills and
implemented only in Python eval tooling. Until Sources rendering
(PP-1000) can call these parsers, a document's Sources show as literal
text.

## Context

Child of PP-868 — Topic-Research Set Detail Page, one of its six children,
with no blockers.

A Sources entry reads `- [Title](drafts/url) — tier text — venue`. Tier text is a
base tier (`tier-1`, `tier-2`, `tier-3`) with an optional parenthesised
suffix whose parts are joined by `; ` in a fixed order: `retracted`,
`withdrawn`, then an `also` list of further tiers joined by `, `. The
citing-once rule defines it and gives worked examples.

An entry may be followed by a metadata block, nested `  - key: value`
lines that belong to the entry above. The venue may be absent. A Sources
section with nothing to cite reads `None found.`. The finding outputter
defines the entry and the block.

The research crate has `enum Tier` with `Display` only. The eval tooling's
`parse_sources` and `TierText.parse` implement the whole grammar in Python.

## Requirements

- Read the Sources section of a document body and return its entries in
  written order, each with its title, URL, tier, optional venue, and
  metadata block as key-value pairs in written order.
- Report the section's state with the entries: **absent** when the document
  has no Sources section, **declared empty** when the section holds a
  `None found.` line and no entry, and **present** otherwise.
- Return a source listed twice as two entries.
- Attach each metadata block line to the entry above it.
- Return an entry whose tier text is rejected with that text as written in
  place of a parsed tier, so one bad tier does not lose the entry or the
  section.
- Parse tier text into a base tier, a retracted mark, a withdrawn mark, and
  an ordered list of `also` tiers.
- Check tier text for shape only: a base of `tier-1`, `tier-2`, or
  `tier-3`, then an optional suffix of one space, an opening parenthesis,
  parts joined by `; ` in the order `retracted`, `withdrawn`, `also` list,
  and a closing parenthesis. This covers every form the citing-once rule
  defines and the bare and single-mark forms written before it.
- Accept text of that shape as written, even where it breaks the rule's
  constraints; the rule and its evals police those, not the tier-text parser.
- Skip each unread line, as the line rules below define it, and report the
  unread lines as their own list, in written order, each exactly as
  written.
- Reject text that is not tier text with an error that carries the complete
  input unchanged, so a caller can show it as written.
- Display a parsed tier text as the text it was parsed from.
- Export both parsers from the `research` crate, so other crates call them
  and write none of their own. The two skills remain the grammar's
  definition, and the tier-text parser's test cases include every example
  the citing-once rule gives.
- Leave the crate's existing `Tier` and `Record` types as they are. `Tier`
  is the base tier only; a parsed tier text holds a base tier, the two
  marks, and the `also` tiers.

### Line rules

The Sources parser reads lines as follows. Matches are exact: a line that
does not match a rule exactly is an unread line.

- The section opens at the first line reading exactly `## Sources` and
  closes at the next `## ` heading or the end of the document. A later
  `## Sources` heading starts no second section. A deeper heading inside the
  section is an unread line.
- A line is an **entry** when it reads `- [title](drafts/url) — ` followed by
  text. The tier text is the text up to the next ` — ` or the end of the
  line, taken exactly, with no trimming. Anything after that ` — ` is the
  venue, which may itself contain ` — `; nothing after it means no venue.
- A line is a **block line** when it reads two spaces, `- `, a lowercase
  snake_case key, `: `, and a non-empty value. The key ends at the first
  `: `. It attaches to the entry above it when only that entry's own block
  lines and blank lines lie between; otherwise it is an unread line. A key
  given twice yields two pairs.
- Blank and whitespace-only lines are ignored. A line reading exactly
  `None found.` is not an unread line; it marks the section declared empty
  when no entry is present.
- Every other line is an **unread line**: skipped, and reported alongside
  the entries as written. This includes a link with no ` — ` after it, a
  nested line that is not `key: value`, and a block line with no entry
  above it.

## Acceptance Criteria

- [ ] Given `tier-1`, `tier-2`, and `tier-3`, then each parses to that base
      tier with no marks and no `also` tiers.
- [ ] Given `tier-3 (retracted)` and `tier-3 (withdrawn)`, then each parses
      to base `tier-3` with that one mark.
- [ ] Given `tier-3 (also tier-1)` and `tier-3 (also tier-2, tier-1)`, then
      each parses to base `tier-3` with its `also` tiers in the order
      written.
- [ ] Given `tier-3 (retracted; withdrawn; also tier-1)`, then it parses to
      base `tier-3`, both marks, and `also` tier `tier-1`.
- [ ] Given `tier-4`, `tier-3 (unknown)`, `tier-3 (also tier-9)`, and an
      empty string, then each is rejected with an error from which the
      caller recovers the whole input unchanged.
- [ ] Each tier text the citing-once rule gives parses as follows: `tier-1`,
      `tier-2`, and `tier-3` to that base alone; `tier-3 (retracted)` to
      base `tier-3`, retracted; `tier-3 (also tier-1)` and
      `tier-3 (also tier-2)` to base `tier-3` with that one `also` tier;
      `tier-3 (also tier-2, tier-1)` to base `tier-3` with `also` tiers
      `tier-2` then `tier-1`; `tier-3 (retracted; also tier-1)` and
      `tier-3 (retracted; also tier-2, tier-1)` to base `tier-3`, retracted,
      with those `also` tiers; and
      `tier-3 (retracted; withdrawn; also tier-1)` to base `tier-3`, both
      marks, and `also` tier `tier-1`.
- [ ] Given a Sources section holding
      `- [The Notifier Pattern](https://blog.example/notifier/) — tier-2 — blog.example`,
      then one entry is returned with that title, that URL, base `tier-2`,
      and venue `blog.example`.
- [ ] Given an entry with no venue, `- [T](https://a.example/) — tier-1`,
      then the entry is returned with no venue.
- [ ] Given an entry followed by `  - doi: 10.7717/peerj.4375` and
      `  - accessed: 2026-10-24`, then the entry carries those two pairs in
      that order, and the next entry carries none of them.
- [ ] Given a Sources section reading `None found.`, then no entries are
      returned and the section is reported declared empty; given a document
      with no Sources section, then no entries are returned and the section
      is reported absent; given a section with one entry, then it is
      reported present.
- [ ] Given a Sources section listing the same title and URL twice, then two
      entries are returned.
- [ ] Given three entries whose second reads `— tier-4 —`, then three
      entries are returned in order, the second carrying `tier-4` as written
      in place of a parsed tier.
- [ ] Given a Sources section with a blank line after the heading, between
      two entries, and before the next heading, then both entries are
      returned and no unread line is reported.
- [ ] Given a Sources section holding `None found.` and one entry, then the
      entry is returned, the section is reported present, and no unread line
      is reported.
- [ ] Given `- [T](https://a.example/)` with nothing after the link, then no
      entry is returned for it and it is reported as an unread line.
- [ ] Given `- [T](https://a.example/) — blog.example`, then an entry is
      returned carrying `blog.example` as written in place of a parsed tier,
      with no venue.
- [ ] Given `- [T](https://a.example/) — tier-2 — Journal — Series B`, then
      the venue is `Journal — Series B`.
- [ ] Given an entry followed by `  - pdf_url: https://a.example/x.pdf`, then
      the entry carries key `pdf_url` with value `https://a.example/x.pdf`.
- [ ] Given an entry, the line `Some prose.`, then `  - doi: 10.1/x`, then
      the entry carries no pair and the unread lines are exactly
      `Some prose.` then `  - doi: 10.1/x`, in that order, leading spaces
      included.
- [ ] Given entry A, then `- [B](https://b.example/)`, then
      `  - doi: 10.1/b`, then A carries no pair and both later lines are
      unread.
- [ ] Given `- [T](https://a.example/) —  tier-1`, with two spaces after the
      dash, then an entry is returned carrying ` tier-1` as written in place
      of a parsed tier.
- [ ] Given an entry followed by `  - DOI: 10.1/x`, `  - Published Date: 2019`,
      and `  - doi: `, then all three are unread lines.
- [ ] Given an entry followed by `  - journal_ref: Phys. Rev.: A 12`, then
      the entry carries key `journal_ref` with value `Phys. Rev.: A 12`.
- [ ] Given an entry followed by two `  - doi: …` lines, then it carries two
      `doi` pairs in written order.
- [ ] Given a document with two `## Sources` sections, then only the first
      is read; given a heading `## sources` or `## Sources and notes`, then
      the section is reported absent.
- [ ] Given `- [T](https://a.example/) — tier-1 — `, with nothing after the
      second dash, then the entry has base `tier-1` and no venue; given
      `- [T](https://a.example/) — `, then it is an unread line.
- [ ] Given an entry followed by `  - not a pair`, then that line is reported
      as an unread line.
- [ ] Given a `## Sources` section with one entry and a `### Notes` line,
      followed by a `## Appendix` section holding a line of the entry shape,
      then one entry is returned and `### Notes` is the only unread line.
- [ ] Given `tier-3 (retracted; withdrawn)` and
      `tier-3 (withdrawn; also tier-1)`, then the first parses to base
      `tier-3` with both marks, and the second to base `tier-3`, withdrawn,
      with `also` tier `tier-1`.
- [ ] A test outside the `research` crate parses a document body into
      entries, unread lines, and a section state, and parses a tier text,
      using only the crate's public exports; the public-API snapshot shows
      both.
- [ ] Given `tier-1 (retracted)`, `tier-3 (also tier-3)`, and
      `tier-3 (also tier-1, tier-2)`, then each is accepted and parses to
      the base, marks, and `also` tiers as written.
- [ ] Given `tier-3 ()`, `tier-3 (withdrawn; retracted)`,
      `tier-3 (also tier-1; retracted)`, `tier-3 (retracted; retracted)`,
      `tier-3(retracted)`, `tier-3 (retracted;withdrawn)`,
      `tier-3 (also tier-2,tier-1)`, `tier-3  (retracted)`, `tier-3 (also)`,
      `tier-3 (retracted) x`, `Tier-1`, and ` tier-1 `, then each is rejected
      with an error from which the caller recovers the whole input
      unchanged, surrounding spaces included.
- [ ] Every accepted tier text in the criteria above renders back from its
      parsed value to the identical text.
- [ ] Given a Sources section holding two entries, a line of prose between
      them, and a block line before the first entry, then both entries are
      returned and the two other lines are reported as unread, each as
      written.

## Open Questions

- None.

## Dependencies

- Blocked by: nothing.
- Blocks: PP-1000, Sources rendering in the visualiser,
  which renders what this parser returns.
- Upstream definitions: the citing-once skill (PP-892) for tier text, and
  the finding-outputter skill for the entry and the metadata block. A
  change to either must change these parsers and the Python eval parser by
  hand; nothing enforces it. The two implementations share the accepted
  tier texts and the entry and block-line shapes, and the cases in
  `tests/unit/tasks/test_citing_once_rule.py` pass on both. They differ on
  purpose in three places: a rejected tier keeps its entry here and fails
  there, unread lines are reported here and fail there, and a block line
  separated from its entry is unread here and attaches there. Where they
  disagree, this item's rules win for the Rust parsers.
- Wire shape: PP-1000 has not settled how sources reach the frontend.
  These parsers return no section location and no serialisable types; the
  visualiser server maps the result and handles the document body itself.
- Shared file: the set page (PP-1001) also changes the `research`
  crate's public-API snapshot; whichever lands second regenerates it.
- Unread lines: PP-1000 shows them in a collapsed notice, so the
  report has a consumer; it needs each line as written, in order.
- Related, still draft: PP-924 adds metadata keys. The Sources parser needs
  no change provided its keys stay lowercase snake_case with single-line
  values, as the finding-outputter skill requires today.
- Related, still draft: PP-891 and PP-897 change which tier a source is
  given, not the grammar.

## Assumptions

- Three rules were the drafting agent's calls, confirmed by the author on 2026-11-23:
  blank lines are ignored, a link with no tier segment is an unread line,
  and a block line separated from its entry is unread.
- Both parsers live in the `research` crate beside `Tier`. The visualiser
  server does not depend on `research` today; the dependency lint permits
  that edge, and Sources rendering or the set page adds it.
- The Python eval parser stays as it is: eval tooling does not call Rust.

## Technical Notes

**Size**: M — two pure parsers, the section and the tier text, beside an
existing type, with examples already written in the skills and a reference
implementation of both to compare against.

- `enum Tier { One, Two, Three }` with `Display` is in
  `cli/research/src/sources/record.rs:10-24`; `Record` carries separate
  `retracted` and `withdrawn` booleans (`record.rs:56-58`).
- The reference implementation is in `tasks/eval/documents/sources.py`:
  `TierText.parse` at lines 17-49, the entry and metadata-block patterns at
  lines 7-11, and `sources_section` and `parse_sources` at lines 68-100. Its
  tests are in `tests/unit/tasks/test_citing_once_rule.py`.
- The entry and the metadata block are defined in
  `skills/research/outputters/finding-outputter/SKILL.md:80-154`.
- The grammar is in `skills/research/outputters/citing-once/SKILL.md:25-30`
  and `:64-80`, with a worked example at `:158-181`.
- `research` may import only std, `kernel::Error`, `network`, and itself
  (`cli/pup.ron:99-114`), which a pure parser satisfies.
- Exporting the parser changes the public-API snapshot at
  `cli/research/tests/fixtures/public-api.txt`.

## Drafting Notes

- Decomposed from PP-868 on 2026-11-23 after its stress test agreed the
  split.
- The rejection and round-trip criteria were added during decomposition;
  PP-868 required only that the rule's examples pass.
- Widened on 2026-11-23, at the author's direction, from tier text to the
  whole Sources section, so one crate owns the Sources grammar and the set
  page builds none of it. The title and size changed with it, and the file was
  renamed on promotion.
- The tier-text parser checks shape only and the Sources parser is lenient
  on the section, decided on
  2026-11-23: rule-breaking text of the right shape is accepted, so
  `tier-1 (retracted)` reads as a retracted `tier-1` source with no warning.
  Strict shape is what lets round-trip hold for every accepted text.
- Returning a rejected tier as written, not failing the entry, was agreed
  with the author on 2026-11-23; the Python reference fails instead.
- A second review on 2026-11-23 added the line rules. Where the item was
  silent they follow the Python reference: `None found.` is ignored, a
  block line attaches to the most recent entry, and the section closes at
  the next `## ` heading.
- A third review on 2026-11-23 added the section state, the exact-match
  rule, and the adjacency rule for block lines.
- A review on 2026-11-23 enumerated the rule's examples in the criteria,
  narrowed the Summary to Rust consumers, and replaced "no other Rust code
  parses tier text", which was already true, with the export criterion.

## References

- Parent: PP-868
- Related: PP-892 (defines the citing-once rule and the `also` suffix),
  PP-864 (introduced tier text), PP-1000 (the consumer), PP-1001
  (shares the public-API snapshot), PP-924, PP-891, PP-897
- Python reference: `tasks/eval/documents/sources.py`
- `skills/research/outputters/citing-once/SKILL.md`
- `skills/research/outputters/finding-outputter/SKILL.md`
