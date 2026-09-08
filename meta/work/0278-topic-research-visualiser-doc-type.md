---
type: "work-item"
id: "0278"
title: "Topic-Research Visualiser Doc Type and Indexer"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocks: ["work-item:0284"]
tags: ["research", "visualiser", "infrastructure"]
last_updated: "2026-09-08T11:42:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-862"
---

# 0278: Topic-Research Visualiser Doc Type and Indexer

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Register the umbrella `topic-research` doc type in the visualiser and index each
research set as one library entry via its `manifest.md`. This is the visualiser
half of epic Slice 1; it must co-land with the engine (0277) so the vertical
demo — build a set, browse it — lands whole.

## Context

The engine (0277) writes contract-conforming artifacts to disk without this
registration, and the VR-baselined registration is separately demoable — but the
two must land together or the end-to-end demo is lost. The six `research_kind`
values are a frontmatter discriminator on one umbrella type, not separate types,
so this registration cost is paid once. The set-level detail page (0284)
supersedes the shared flat view later; this slice renders through the shared
`LibraryDocView`.

## Requirements

- `server/src/docs.rs`: the `config_path_key` tying the doc type to
  `paths.research_topics`; the umbrella `topic-research` `DocTypeKey` variant +
  `all()` + `label()` + `wire_str()`/`from_wire_str()` with exhaustiveness tests.
- Phase membership in `server/src/api/library.rs` (`PHASES`, under "discover").
- TS `DocTypeKey` union + `DOC_TYPE_KEYS` + labels in
  `frontend/src/api/types.ts`.
- Glyph icon component + `ICON_COMPONENTS` in `frontend/src/components/Glyph/`.
- Colour tokens (light/dark) + a framed-background CSS rule.
- Nested-manifest indexing keyed on `manifest.md` (`file_driver.rs` `list`
  branch + `indexer.rs` `build_entry` slug-from-parent-directory), reusing the
  design-inventory `nested_manifest_filename()` precedent.
- VR baselines (~8 PNGs across sizes × themes) passing on darwin and linux.

## Acceptance Criteria

- [ ] The single umbrella `topic-research` doc type appears in the visualiser
      library under the Discover phase with a glyph and framed background and
      passing visual-regression baselines on both darwin and linux.
- [ ] A research set indexes as one library entry per
      `meta/research/topics/<slug>/` directory via the `manifest.md` nested
      manifest; individual findings and reports are not separately indexed.
- [ ] Set sub-documents render through the shared `LibraryDocView` (the
      set-level detail page is 0284).

## Open Questions

- None specific to this slice.

## Dependencies

- Blocked by: 0277 (indexes the artifacts the engine writes; co-land
  constraint — recorded on 0277's `blocks`).
- Blocks: 0284 (the set-level detail page builds on this umbrella type and
  nested indexing).

## Assumptions

- The nested-manifest indexer precedent (design inventories) transfers directly;
  `manifest.md` is a dedicated aggregate root, not the brief or synthesis.

## Technical Notes

- The lister skips dot-prefixed dirs, so the engine (0277) must write a set into
  a temp dir and rename it in — no half-written library entry.
- `list` yields one entry per set directory; the `findings/` and `reports/`
  subdirectories and the root sub-documents are siblings the 0284 detail page
  serves, not index entries.
- The `report` `research_kind` value is registered later by 0281 into this same
  umbrella type.

## Drafting Notes

- Split from Slice 1 at the user's request; carries the doc-type registration and
  nested-manifest indexer, while 0277 carries the engine. The two must co-land.
- Extracted from source documents without interactive enrichment. Acceptance
  criteria, dependencies, and kind may need refinement before promoting from
  `draft` to `ready`.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 1, visualiser half)
- Parent epic: 0121
- Internal patterns: `server/src/docs.rs`, `frontend/src/api/types.ts`
