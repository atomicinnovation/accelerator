---
type: "work-item"
id: "0277"
title: "Single-Round Web Research Engine"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocks: ["work-item:0278", "work-item:0279", "work-item:0280"]
tags: ["research", "skills", "deep-research"]
last_updated: "2026-09-08T11:42:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-861"
---

# 0277: Single-Round Web Research Engine

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

The walking-skeleton engine: run the full build loop once over web sources —
`brief` → `outline` → `conduct` (single round) → `synthesise` — producing a
contract-conforming set on disk at `meta/research/topics/<slug>/`. This is the
engine half of epic Slice 1; the visualiser registration and nested-manifest
indexer are the sibling child (0278), which must co-land so the vertical demo
lands whole.

## Context

The epic's central design risk is whether the loop produces a dossier worth
reading, and no single verb puts that question in front of a user — so the loop
must be observable end to end. This child writes conforming artifacts to disk
(judged later by the Slice 3 output-quality gate in 0280); 0278 makes them
browsable. It is built on the reusable-infrastructure seam: a generic
`researcher` agent specialised at spawn time by an injected
`(source_profile, question)` pair, mirroring the `reviewer` agent.

## Requirements

- `paths.research_topics` config key + default, so the skill locates the set
  directory (the server-side `config_path_key` wiring belongs to 0278).
- Manifest / brief / outline / finding / synthesis templates (web-only)
  resolving through the 3-tier override (config path → user override → plugin
  default), plus per-skill `instructions.md`/`context.md` wiring.
- A dedicated `manifest.md` aggregate root written at `brief` time, carrying the
  `primary` pointer (`brief.md` before synthesis, `synthesis.md` after).
- The generic `researcher` agent granted WebFetch of arbitrary URLs, plus a web
  source profile that captures each source URL inline in findings with its
  reputation tier — so the corpus conforms from the first file written.
- The `research-topic` skill dispatching `brief` + `outline` + `conduct`
  (single round) + `synthesise` on the `configure` pattern, with set-handle
  resolution (set directory, bare slug, or any set document) on every verb but
  `brief`.
- An `outline` prompt carrying the effort-scaling rubric (1 focus area for a
  simple subject, 2–4 for comparisons, 10+ for broad subjects), sizing the round
  at or beneath `breadth` as a ceiling.
- `conduct` writing one immutable finding per focus area into `findings/`;
  `synthesise` writing `synthesis.md` inline from the findings.
- Hardcoded defaults `breadth: 8`, `depth: 1` (made tunable in 0282).

## Acceptance Criteria

- [ ] Given a subject, `brief` interactively scopes it and produces
      `manifest.md` + `brief.md` conforming to the artifact contract, with
      `research_status: briefed` and `primary` pointing at `brief.md`.
- [ ] Given a set handle, `outline` writes `outline.md` — a `## Round 1`
      checklist of focus areas sized at or beneath `breadth` by the rubric — and
      sets `research_status: outlined`; the user may hand-edit it.
- [ ] Given a set handle, `conduct` runs one round over the outline's
      outstanding focus areas, spawns parallel `researcher` agents, writes one
      immutable finding per focus area into `findings/`, flips the corresponding
      checkboxes, and sets `research_status: researching`, `round_count`,
      `finding_count`.
- [ ] Given a set handle, `synthesise` writes `synthesis.md` from the findings,
      sets `research_status: synthesised`, and flips `primary` to
      `synthesis.md`.
- [ ] Focus areas commissioned per round never exceed `breadth`; the rubric may
      only reduce the count beneath that ceiling, never raise it.
- [ ] Every source entry in every finding carries an inline reputation tier from
      the first round onwards, web-profile sources included, and those tiers are
      carried into `synthesis.md`.
- [ ] Templates resolve through the 3-tier override; users can override
      templates and add per-skill `instructions.md`/`context.md`.
- [ ] The generic `researcher` agent is specialised for web purely by injected
      profile — no per-source agent — the seam Slice 3 (0280) later proves for
      academic sources.

## Open Questions

- None specific to this slice — the epic resolved its design questions; the one
  still-open question (set-document API shape) is scoped to 0284.

## Dependencies

- Blocks: 0278 (co-land), 0279, 0280; also gates 0281 (recorded on 0281's
  `blocked_by`).
- Co-land constraint: must merge together with 0278 so the vertical demo
  (engine + library entry) is not lost.

## Assumptions

- Sources are external web only in this slice; academic sources arrive in 0280.
- The artifact contract's field names are stable; findings are immutable once
  written, so any later additions must be backward-compatible.

## Technical Notes

- Set-handle resolution mirrors the `work resolve` precedent (paths, IDs, bare
  numbers), resolving three forms to the set root; surface it in each
  subcommand's `argument-hint`.
- The skill writes a set into a temp dir and renames it in, because the indexer's
  lister skips dot-prefixed dirs and must never see a half-written set.

## Drafting Notes

- Slice 1 was split into this engine child and the visualiser/indexer child
  (0278) at the user's request; the partition puts skill/agent/template/artifact
  here and doc-type registration + nested-manifest indexing in 0278. The two
  must co-land.
- `paths.research_topics` default lives here (skill-side path resolution); the
  server `config_path_key` moved to 0278.
- Kept `breadth: 8` / `depth: 1` hardcoded; tunability is 0282.
- Extracted from source documents without interactive enrichment. Acceptance
  criteria, dependencies, and kind may need refinement before promoting from
  `draft` to `ready`.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 1, engine half)
- Parent epic: 0121
