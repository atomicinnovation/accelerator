---
type: "work-item"
id: "0330"
title: "Rich Document Model for Corpus Documents"
date: "2026-10-10T21:48:33+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
derived_from: ["plan:2026-10-10-0296-sync-round-trip-defects"]
relates_to: ["work-item:0296", "work-item:0290"]
tags: ["corpus", "document", "markdown", "refactor"]
last_updated: "2026-10-10T21:48:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0330: Rich Document Model for Corpus Documents

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Code that reads or writes meta documents works on strings: at least 15
production sites re-implement fence detection and scan frontmatter keys or
body sections by hand. Build out the minimal `corpus::body` model that 0296
introduces into a full document model in `corpus`, parsed through `document`,
and migrate every string-based consumer onto it.

## Context

`corpus` (a kernel-only platform domain) owns the serde-free document model;
`document` (a technical library) owns the file format and confines
`serde-saphyr`; `corpus-adapters` bridges them through the
`FrontmatterParser` port. Frontmatter is structured on both sides, but a
body is a `String` everywhere, and many consumers bypass both crates.

0296 adds the first structured body: `corpus::body` (blocks, inlines,
level-2 sections, canonical form and equivalence), a `document::markdown`
event stream over an exact-pinned `pulldown-cmark`, and a `BodyParser` port
implemented in `corpus-adapters`. It models only what sync equivalence and
dossier section diffs need.

String handling causes drift: each re-implementation of fence detection
differs slightly (`trim_end` or not, CRLF or not), and section or key
extraction by line scanning disagrees with what a parser sees.

## Requirements

- `corpus` exposes a `Document` combining parsed frontmatter
  (`FrontmatterState`) and a parsed `Body`, read through one port.
- The `Body` model covers every CommonMark and GFM construct the corpus
  uses, with source spans, so a consumer can rewrite a section without
  reformatting the rest of the document.
- Every production site that detects fences or extracts frontmatter or body
  sections by hand moves onto the model or onto `document::fence`. Known
  sites: `work/src/work_item_files.rs:54`, `work/src/section_diff.rs:42,70`,
  `work/src/retirement.rs:503`, `work-adapters/src/sync/digest.rs`,
  `work-cli/src/list.rs:80`, `work-cli/src/show.rs:36`,
  `corpus/src/references.rs:55`, `corpus/src/adr.rs:40`,
  `corpus/src/frontmatter_validation/template_shape.rs:217`,
  `config-adapters/src/store.rs:767`, and the migrations `m0002`, `m0006`,
  `m0007`, `m0009` and `m0010`.
- `work`'s frontmatter normalisation (`normalise.rs`) operates on the parsed
  frontmatter model rather than lines.
- No observable behaviour changes, except where a re-implementation
  disagreed with `document`; each such change is listed in the PR.
- Digest recipes are unaffected unless a migration bumps the recipe marker
  on purpose.

## Acceptance Criteria

- [ ] Given the workspace, when production code is searched for hand-rolled
      fence detection (`== "---"`, `is_fence`, `extract_frontmatter`), then
      the only matches are in `document` and in migrations frozen against
      historical formats, each with a stated reason.
- [ ] Given a document with CRLF line endings, a `---` line in its body, or
      no trailing newline, when any migrated consumer reads it, then it sees
      the same frontmatter and body that `document::split` returns.
- [ ] Given a consumer that rewrites one body section, when it writes the
      document, then every byte outside that section is unchanged.
- [ ] Given the baseline corpus and golden-digest fixtures, when the suite
      runs, then every recorded digest still matches.
- [ ] Given `corpus` and `document`, when their public API snapshots are
      checked, then no `pulldown_cmark` or `serde_saphyr` type appears.

## Open Questions

- Does `corpus::Document` replace `ParsedDocument`, or wrap it?
- Should the migrations be migrated, or frozen as written because they
  describe historical formats?

## Dependencies

- Blocked by: 0296, which introduces `corpus::body`, `document::markdown` and
  the `BodyParser` port.
- Related: 0290, whose per-field sync reads frontmatter through this model.

## Assumptions

- `pulldown-cmark` source offsets are precise enough for section-preserving
  rewrites.

## Technical Notes

- Frontmatter port precedent: `cli/corpus-adapters/src/document.rs:20`.
- ADR-0069 rule 1 keeps the parser out of `corpus`; `work` reaches the model
  through `corpus` types and the parse through a port.
- `document` and `corpus` are pinned in `tasks/public_api.py`.

## Drafting Notes

- Split from 0296 during planning so that sync equivalence lands without
  migrating every consumer first.

## References

- Source: `meta/plans/2026-10-10-0296-sync-round-trip-defects.md`
- Related: 0296, 0290
- Codebase research: `meta/research/codebase/2026-10-10-0296-sync-round-trip-defects.md`
