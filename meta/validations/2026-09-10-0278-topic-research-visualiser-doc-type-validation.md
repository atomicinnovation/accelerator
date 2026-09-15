---
type: "plan-validation"
id: "2026-09-10-0278-topic-research-visualiser-doc-type-validation"
title: "Validation Report: Topic-Research Visualiser Doc Type and Indexer Implementation Plan"
date: "2026-09-15T13:54:36+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
target: "plan:2026-09-10-0278-topic-research-visualiser-doc-type"
tags: ["visualiser", "doc-type", "indexer", "rename", "topic-research"]
last_updated: "2026-09-15T13:54:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Topic-Research Visualiser Doc Type and Indexer

All three phases are implemented and every read-only check exits 0. The
wire-key rename, the `research_status → status` collapse, and the
`TopicResearch` registration (Rust registry, indexer, frontend maps, lifecycle
chips, colour/glyph) are all present and green in the in-loop suite. What
remains is genuinely environment- and human-gated: the 10 topic-research Docker
VR baselines are not generated (no Docker/colima here), the native Playwright
`fixture-coverage.spec.ts` is not run, and the design/status-chip sign-offs are
outstanding. `mise run test` is red on two conditions, neither of them a
0278-visualiser test.

### Implementation Status

- ✅ Phase 1: wire-key rename `research → codebase-research` — `wire_str`,
  `parity.rs`, `cluster.rs`/`clusters.rs`/`pipeline-step-parity`, the full
  `Record<DocTypeKey, …>` sweep, key-derived CSS tokens, the `localStorage`
  one-shot rewrite, and the 10 renamed VR baselines (0 stale `research-*`).
- ✅ Phase 2: `research_status → status` collapse — manifest row `status_vocab`
  carries the five lifecycle states, `research_status` moved to
  `OBSOLETE_LEGACY_KEYS` (`[&str; 3]`), template/skill/fixture all rewritten; no
  `research_status` literal remains in skill, template, or fixtures.
- ✅ Phase 3: `TopicResearch` registration — enum + five method arms, identity
  slug arm, catalogue row, `parity.rs`/`api_types`/`compose_contract` counts,
  four-set indexer fixture (+ dot-dir + empty-set), frontend registry, colour
  tokens, glyph/big-glyph, and the centralised `chipVariantFor` /
  `lifecycleToVariant` dispatch threaded through card, detail, and facet sites.

### Automated Verification Results

- ✅ `mise run cli:check` — exit 0 (registry, slug arm, catalogue, parity).
- ✅ `mise run server:check` — exit 0 (indexer, api_types, compose_contract).
- ✅ `mise run frontend:check` — exit 0 (registry maps, status-chip tests,
  CSS-resolution accessibility test, Rust→TS vocab-drift check, localStorage
  migration/idempotency, VR baseline-coverage guard).
- ❌ `mise run test` — exit 1, on three conditions. Every 0278-visualiser test —
  registry, indexer, chips, dangling/DuplicateId, set-scoped ids — passes; the
  three failures are the pre-existing `work-item:0286` DUPLICATE-ID (which this
  plan recorded as blocking `mise run test`), the 0277 conformance count drift,
  and one stale resolver test this plan's own registration flips (below).
- ⚪ `mise run test:e2e:visualiser:docker` — not run (no Docker/colima); the 10
  topic-research baselines are not generated. Environment-gated per the plan.
- ⚪ `mise run test:frontend:visualiser` (`fixture-coverage.spec.ts`) — not run
  (needs a browser); the server fixture set and its `DETAIL_ROUTE_SLUGS` entry
  are in place.

### Code Review Findings

#### Matches Plan

- `Self::Research => "codebase-research"` (wire) with `config_path_key` still
  `research_codebase`; `Self::TopicResearch` arms all present; Discover order
  places `TopicResearch` adjacent to `Research`. The lifecycle chip ramp
  (`lifecycle-briefed … lifecycle-complete`) is a dedicated non-green ordinal
  scale dispatched by a `Record<DocTypeKey, …>` classifier, exactly as specified.

#### Deviations from Plan

- None material. The plan's `DuplicateId` two-set coverage lands as a tempdir
  construction on the corpus-cli side (noted under 0277) rather than a second
  committed set; the visualiser server fixtures exceed the plan's minimum (four
  real sets, a dot-dir, and an empty-set).

#### Potential Issues

- ⚪ The 10 topic-research VR baselines (8 glyph × 4 sizes × 2 themes, 2
  big-glyph) are absent. The in-loop `vr-baseline-coverage` guard tracks
  `topic-research` as pending and self-clears once the Docker lane commits them
  — no silent drift, but the criterion is not yet met.
- ❌ This plan's registration of `topic-research` invalidates a 0277 resolver
  test: `resolve_goldens::an_unregistered_type_is_a_distinct_unknown_type_code`
  passes `--type topic-research` expecting exit 4 (`E_RESOLVE_UNKNOWN_TYPE`),
  but the now-registered type returns exit 3 (`E_RESOLVE_NOT_FOUND`). The
  behaviour is correct; the co-land should repoint that test at a
  genuinely-unregistered type token. Owned jointly with 0277 (see its report).

### Manual Testing Required

1. Library Discover shows a **Topic research** card (glyph + framed background)
   adjacent to **Codebase research**; clicking opens `manifest.md` via
   `LibraryDocView`.
2. The `synthesised` fixture card's status chip reads "synthesised" (not
   "complete") in a distinct non-grey tone.
3. `/library/codebase-research/<slug>` resolves; `/library/research/...` no
   longer validates; a pre-rename `research` last-seen entry is not re-flagged
   unseen after reload.
4. ❓ Human design sign-off of glyph, big-glyph, and colour against the
   `2026-09-10-174311` prototype (a PR approval or work-item note naming the
   reviewer).
5. ❓ Status-chip lifecycle scale signed off against its own acceptance bar
   (five first-class steps, no green/grey, legible as a lone chip and under CVD
   simulation).

### Recommendations

- Generate and commit the 10 topic-research VR baselines on the pinned
  Docker/Linux harness; run `fixture-coverage.spec.ts` natively — the two
  remaining automated criteria.
- Record the design and status-chip human sign-offs on the PR or work item.
- ⚠️ Resolve the orthogonal aggregate reds before claiming a green
  `mise run` / `mise run check`: the `work-item:0286` DUPLICATE-ID and the
  `RUSTSEC-2026-0285` `rustls` advisory. Neither belongs to this plan.
