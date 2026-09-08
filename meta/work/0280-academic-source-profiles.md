---
type: "work-item"
id: "0280"
title: "Academic Source Profiles"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocks: ["work-item:0283"]
tags: ["research", "skills", "sources"]
last_updated: "2026-09-08T11:42:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-864"
---

# 0280: Academic Source Profiles

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Research drawing on academic sources — OpenAlex and arXiv — tiered by the same
reputation mechanism Slice 1 established for the web. Purely additive: it adds
source families and the rendering of their tiers, not the tagging itself. This
slice carries the epic's sole output-quality gate.

## Context

Reputation tagging already exists from Slice 1's web profile, so this slice adds
keyless academic profiles and proves the generic `researcher`'s injection seam
handles web vs academic by profile alone. The dedicated final citation pass is
deferred — a safe drop-in later because the immutable tagged findings preserve
attribution.

## Requirements

- OpenAlex + arXiv source profiles, both keyless (Crossref and Semantic Scholar
  are deferred drop-in profiles).
- A `research.contact_email` config injected into the `mailto`/User-Agent polite
  pool — the courtesy identification these APIs use to grant a higher rate limit,
  effectively required for parallel fan-out.
- arXiv Atom-XML parsing (the one non-JSON source).
- Rate-limit-aware fan-out that degrades rather than failing on `429`.
- The brief's `source_profiles` list gains the academic profiles; `conduct`
  assigns one profile per spawned researcher, so a single round may mix web and
  academic researchers.
- Academic profiles emit reputation tiers into the immutable findings exactly as
  the web profile does; `synthesise` carries them forward (no separate citation
  agent).
- Template rendering for reputation tiers, surfacing in the shared
  `LibraryDocView` until the Slice 6 page (0284) supersedes it.

## Acceptance Criteria

- [ ] `brief.md` declares a `source_profiles` list and `conduct` assigns one
      profile per spawned researcher, so a single round may mix web and academic
      researchers.
- [ ] Academic profiles draw on OpenAlex and arXiv with no API key, tagging
      sources with reputation tiers by the Slice 1 mechanism.
- [ ] A `research.contact_email` config key is registered with a documented
      default and injected into the `mailto`/User-Agent of every outbound
      academic request; parallel fan-out is rate-limit aware and degrades rather
      than failing on `429`.
- [ ] The generic `researcher` agent is specialised for web vs academic purely by
      injected profile — no per-source agent required (the Slice 1 seam, proven
      here).
- [ ] A worked reference subject is researched end-to-end and its `synthesis.md`
      reviewed by a human against the brief: every claim traces to a tagged
      source, the reputation tiers are defensible, and the dossier answers the
      brief's questions. This is the epic's only output-quality gate and is
      deliberately human-judged.

## Open Questions

- None specific to this slice.

## Dependencies

- Blocked by: 0277 (the generic `researcher` + web reputation-tier mechanism —
  recorded on 0277's `blocks`).
- Blocks: 0283 (the recursion engine lands only after this output-quality gate
  validates the premise).

## Assumptions

- OpenAlex and arXiv remain keyless and free; no secrets-management scope.
- Semantic Scholar's optional key (deferred) is a rate-limit lift only.

## Technical Notes

- "Reputation tier" is reputation-only — the standing of the publishing venue,
  not any assessment of a claim's correctness.
- `synthesis.md`'s and each report's Sources are derived from the findings'
  tiers, never authored independently.

## Drafting Notes

- `research.contact_email` is a string config; the numeric `depth`/`breadth`
  tunable precedent is the separate child 0282.
- The output-quality gate is human-judged by design, not an automated assertion.
- Extracted from source documents without interactive enrichment. Acceptance
  criteria, dependencies, and kind may need refinement before promoting from
  `draft` to `ready`.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 3)
- Parent epic: 0121
