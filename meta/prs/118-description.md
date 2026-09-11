---
type: "pr-description"
id: "118"
title: "[0278] Topic-Research Visualiser Doc Type and Indexer"
date: "2026-09-11T16:35:27+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0278"
parent: "work-item:0278"
relates_to: ["work-item:0277"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/118"
pr_number: 118
tags: []
revision: "186187a99ac49e0347ba067adc0b15ec54599588"
repository: "accelerator"
last_updated: "2026-09-11T16:35:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0278] Topic-Research Visualiser Doc Type and Indexer

## Summary

The visualiser half of epic Slice 1: topic-research sets produced by the engine
(0277) now appear as cards in the visualiser library that open to their
manifest. This registers the umbrella `topic-research` doc type, indexes each
set as one library entry via its `manifest.md`, and collapses the manifest's
separate `research_status` onto its base `status` so the card reports real
progress. It co-lands with PR #117 (work item 0277) — the engine's artifacts
are reader-observable only once this lands. Stacked on #117.

## Changes

### Visualiser doc type and indexer

- **Register the umbrella `topic-research` doc type.** The six doc `kind`
  values (`manifest`, `brief`, `outline`, `finding`, `synthesis`, `report`)
  are a frontmatter discriminator on one umbrella type, so registration is paid
  once. The nested-manifest indexer keys each set to one library entry by its
  `manifest.md`; sub-documents are not separately indexed (set-level navigation
  is deferred to 0284). Rendering reuses the shared `LibraryDocView`.
- **Collapse `research_status` onto base `status`** in the manifest, so the
  library card and status chip read a single real-progress field.
- **Rename the `research` doc type's wire key to `codebase-research`.** This
  aligns the API/URL token with the `codebase-research` name its typed-linkage
  and `type:` frontmatter already use, and relabels the display name to
  "Codebase research" to disambiguate it from the new type. The
  `research_codebase` config key and `meta/research/codebase/` directory keep
  their own names. A one-shot client-side migration rewrites a stale
  `research` last-seen token so existing users keep their unseen state.
- **Frontend surfacing** — a dedicated ordinal `lifecycle-*` chip ramp for the
  topic-research lifecycle states (`briefed` → `outlined` → `researching` →
  `synthesised` → `complete`), a `TopicResearch` Glyph and BigGlyph, and their
  visual-regression baselines. The generic chip/badge presenters stay
  doc-type-agnostic; a route-level, type-aware resolver routes lifecycle types
  to the ordinal ramp and every other type to the shared semantic lexicon.

### Design research feeding the visualiser

- The `claude-design-prototype` source and its design inventory (sequence 3), a
  current-app runtime design inventory (superseding the 2026-05-21 one), and a
  design-gap analysis comparing the current app against the prototype.
- Three bugs raised from the design/executor work: 0286 (inventory-design does
  not set the screenshot output root), 0287 (executor omits documented
  interaction commands), and 0288 (parallel browser-analyser agents share one
  daemon and tab).

### Planning artifacts

- The 0278 plan, work item, round-1 review, and the 0121/0278 alignment with
  the manifest-status collapse.

### Repository-wide comment cleanup (two commits)

- **Strip work-item, ADR, and plan references from comments** — removes stale
  tracked-number references (0278/0280/0281, ADR-0067) that the comment policy
  bans.
- **Reword comments off the retired bash implementation** — restates ~80 doc
  comments across the `cli/` workspace that documented the byte-for-byte port
  of the removed bash tooling, stating each invariant directly. Live
  identifiers are kept (the `bash-parity` cargo feature, `bash -c`, the Claude
  Code `Bash` tool, and the surviving `capture-adf-oracle.sh` /
  `regenerate.sh` test helpers).

## Context

- Implements work item `work-item:0278`, under epic `work-item:0121`.
- Stacked on PR #117 (`work-item:0277`, the engine). The vertical demo — build
  a set, browse it — lands whole only with both; review and merge #117 first.
- Sub-document navigation within a set is 0284; output-quality judgement is
  0280.

## Testing

Built test-first; the changes carry server indexer tests (one entry per set,
title cascade, sub-document and dot-directory skipping), frontend unit and
visual-regression tests (lifecycle chip contrast, glyph/big-glyph baselines,
the wire-key migration), and the committed `topic-research-set` fixtures.

- [x] `mise run cli:check` — Rust workspace format + clippy across all crates:
  green (run this session, after the comment cleanup).
- [ ] `mise run check` — the full read-only CI mirror (adds frontend, build
  system, and shell). Not run in this session; CI gate.
- [ ] `mise run test` — the full suite, including the Docker/Linux visual-
  regression lane. Not run in this session; CI gate.

## Notes for Reviewers

- **Stacked on #117 — review and merge that first.** This PR's diff is
  computed against the `0277` branch.
- The comment-cleanup commits (the last two) are mechanical and span the whole
  `cli/` workspace; they are best reviewed separately from the 0278 feature
  work, and the reword deliberately keeps the live `bash-parity` / `bash -c` /
  `Bash tool` identifiers.
- Focus areas: the nested-manifest indexer's one-entry-per-set rule, the
  `research` → `codebase-research` wire-key rename and its client-side
  migration, and the lifecycle chip ramp's accessibility gate (HSL saturation
  and contrast bounds).
- The `topic-research` visual-regression baselines render only in the pinned
  Docker/Linux lane; a self-cleaning in-loop guard flags the pending set until
  those baselines are committed.
