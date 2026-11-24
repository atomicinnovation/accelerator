---
type: "plan-validation"
id: "2026-11-24-PP-995-sources-parser-in-the-research-crate-validation"
title: "Validation Report: Sources Parser in the Research Crate Implementation Plan"
date: "2026-11-24T13:29:23+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-11-24-PP-995-sources-parser-in-the-research-crate"
tags: ["research", "visualiser", "sources", "tier-text"]
last_updated: "2026-11-24T13:29:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Sources Parser in the Research Crate Implementation Plan

### Implementation Status

✓ Phase 1: The Tier-Text Parser - Fully implemented (`vuuqvrrv`)
✓ Phase 2: Reading Entries, State, Unread Lines and Span - Fully implemented (`zsyzytmv`)
✓ Phase 3: Metadata Blocks - Fully implemented (`qnrnxsnp`)
✓ Phase 4: Splitting and Removing the Section by Its Span, and the Whole-Section Read - Fully implemented (`tytknnvu`)

The change since base `kqsmnqzr` touches eight files: the three new source
files under `cli/research/src/topic/sources_section`, the `topic.rs` module
wiring and doc, two new test files, the public-API snapshot and the plan.

### Automated Verification Results

✓ Targeted tests pass: `mise run test:unit:cli --filter 'binary_id(research::tier_text) | binary_id(research::sources_section)'` (39 of 39)
✓ Snapshot current: `mise run public-api:check`
✓ Snapshot additions only: `jj diff --git --from kqsmnqzr --to @ cli/research/tests/fixtures/public-api.txt | grep '^-[^-]'` prints nothing
✓ No manifest change: `jj diff --stat --from kqsmnqzr --to @ cli/research/Cargo.toml` is empty
✓ Import fence holds: `mise run pup:check`
✓ Full suite green: `mise run` exits 0 (~267s) and leaves the working copy unchanged

### Code Review Findings

#### Matches Plan:

- `TierText::parse` is the plan's code verbatim: shape-only, canonical-order
  checks, so `Display` reproduces every accepted input; `NotTierText` carries
  the whole input and renders ``not tier text: `{text}` ``.
- `lines::numbered` and `lines::offset_of` share one `split_inclusive('\n')`
  line rule, so the parser's line numbers and the span's byte offsets cannot
  drift apart; every offset lands after a `\n` or at a body end, so `split`
  cannot slice off a character boundary.
- `SourcesSection::parse` closes at the first `## ` line, keeps trailing
  blank lines inside the span, and derives `DeclaredEmpty` only when there are
  no entries and `None found.` was seen.
- `CitedSource::parse` tries `](` from the right; `url_and_rest` and
  `tier_and_venue` enforce the token rules; `TierReading::of` is private, so
  `TierText::parse` stays the one public way to read tier text.
- Phase 3's `SectionLine` and `BlockAdjacency` state each line kind's effect
  on adjacency in one `match`; `leave_unread` always detaches, so prose, an
  unread link, `None found.` and an unattached block line all break adjacency.
- Every PP-995 acceptance criterion maps to a named test in
  `cli/research/tests/tier_text.rs` or `cli/research/tests/sources_section.rs`,
  all against public exports only.
- New types derive only `Debug, Clone, PartialEq, Eq` (plus `Copy` on
  `LineSpan`); the snapshot adds 118 lines, all under
  `research::topic::sources_section`.

#### Deviations from Plan:

- `LineSpan::remove_from` uses `format!("{before}{after}")` in place of
  `[before, after].concat()`, which clippy's `tuple_array_conversions` lint
  rejected. Behaviour is identical.
- A `URL_CLOSE: char` constant joins the plan's constants
  (`cli/research/src/topic/sources_section.rs:16`).
- The plan's state test is split in two:
  `the_section_state_is_absent_declared_empty_or_present` and
  `a_heading_with_nothing_under_it_spans_only_its_own_line`.
- `a_metadata_block_belongs_to_the_entry_above_in_written_order` uses
  `10.1/a` and `2026-11-24` where the criterion names
  `10.7717/peerj.4375` and `2026-10-24`; the behaviour checked is the same.
- The module doc in `sources_section.rs` was widened to name metadata and
  the span (an improvement).

#### Potential Issues:

- `split` and `remove_from` take any `&str`; given a body other than the one
  parsed they return a wrong cut rather than an error. They cannot panic, and
  the precondition is documented, so this is acceptable for the internal
  consumer PP-1000 will be.
- The section reader does not know about fenced code blocks: a `## ` line or
  an entry-shaped line inside a fence counts as structure. The grammar does
  not define fences, and the plan scopes them out implicitly.

### Manual Testing Required:

All plan manual checks were run during implementation against the files
under `skills/research/research-topic/evals/variants/*/meta/research/topics/`,
since this workspace has no `meta/research/topics/`. One gap remains:

1. Real-document coverage:
  - [ ] When a real finding whose `## Sources` section is followed by another
        `## ` section exists, feed it through `split` and `remove_from` and
        confirm a non-empty after-text round-trips byte for byte; all six eval
        files' sections run to the end of the file, so only unit tests cover
        that case today.

### Recommendations:

- Update PP-1000's server step to name `LineSpan::split` rather than
  `remove_from`, as the plan's migration note asks.
- Whichever of PP-995 and PP-1002 lands second regenerates the public-API
  snapshot with `mise run public-api:update`.
