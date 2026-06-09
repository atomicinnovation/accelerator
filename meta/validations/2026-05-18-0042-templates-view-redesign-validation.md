---
date: "2026-05-24T21:40:00+01:00"
type: plan-validation
producer: validate-plan
target: "plan:2026-05-18-0042-templates-view-redesign"
result: pass
status: complete
id: "2026-05-18-0042-templates-view-redesign-validation"
title: "2026-05-18-0042-templates-view-redesign-validation"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-24T21:40:00+01:00"
last_updated_by: Toby Clemson
---

## Validation Report: 0042 Templates View Redesign

### Implementation Status

All six phases are implemented and committed. Implementation commits
(jj log, in order):

- `zklrpxnln` Add sha256 of winning content to template detail. (Phase 1)
- `owtpolqws` Broadcast template-changed SSE events when tier files
  change. (Phase 2)
- `lsqkstzxt` Invalidate template queries on template-changed SSE events.
  (Phase 3)
- `lrnrpxnws` Render three tier-presence chips per templates index row.
  (Phase 4)
- `pvsvwtsms` Render templates detail view as a two-column grid with
  active ring. (Phase 5)
- `styxstvqz` Match templates view to the design prototype. (Phase 6 +
  design fidelity)
- `xusrnusqn` Tighten templates view to design fidelity.
- `nyunsqmtn` Apply round-3 templates view design fidelity feedback.
- `ozpvznnxy` Apply round-4 templates view fidelity feedback.

Status by phase:

- ✓ Phase 1: Backend `TemplateDetail.sha256` — implemented
  (`templates.rs:105-112` for `content_sha256`,
  `templates.rs:189-201` for build-time caching, `templates.rs:240`
  for `detail()` clone).
- ✓ Phase 2: `TemplateChanged` SSE variant + watcher wiring —
  implemented in `sse_hub.rs:159,175` and `watcher.rs:41,119,243-247,
  313,322` (`TemplateChangeHandler`, `TierPathIndex`,
  `canonicalise_path_or_ancestor`).
- ✓ Phase 3: Frontend types + dispatch reducer — implemented in
  `frontend/src/api/types.ts`, `use-doc-events.ts`, plus
  `use-doc-events.test.ts`.
- ✓ Phase 4: Index tier-presence row — implemented in
  `LibraryTemplatesIndex.tsx:74-84` and `template-tier.ts:10,18`.
- ✓ Phase 5: Two-column layout + active ring — implemented in
  `LibraryTemplatesView.tsx` and `LibraryTemplatesView.module.css`.
- ✓ Phase 6: Preview pane + content-hash label — implemented with
  intentional design-fidelity refinements to AC13 (truncated display +
  `title`-attribute tooltip surfacing the full digest on hover);
  work-item AC13 reconciled to match.

### Automated Verification Results

- ✓ Backend tests pass: `cargo test` — 386 passed across 20 suites
  (22.56s).
- ✗ Clippy not clean: `cargo clippy -p accelerator-visualiser --
  -D warnings` fails with one `redundant_closure` warning in
  `src/frontmatter.rs:319`. This is outside the 0042 scope (frontmatter
  module, not touched by this plan), but the plan listed clippy-clean
  as a verification criterion. Flag for separate cleanup.
- ✓ Frontend tests pass: 1873 tests across 89 files (5.77s).
- ✓ Frontend typecheck passes: `tsc --noEmit` clean.

### Code Review Findings

#### Matches Plan

- `content_sha256` helper extracted with the encoding rule in one place,
  matching the plan's pub(crate) helper specification.
- `TemplateResolver` caches the digest on the per-name entry — `detail()`
  reads it via clone, no per-request hashing.
- `sha256` field uses `skip_serializing_if = "Option::is_none"` to omit
  rather than emit `null`.
- `SsePayload::TemplateChanged` variant serialises to
  `"type":"template-changed"` via `#[serde(tag = "type", rename_all =
  "kebab-case")]`.
- `ArcSwap<TemplateResolver>` plumbed through AppState with `load()` at
  the API call sites.
- `TierPathIndex` precomputed at startup with the
  `canonicalise_path_or_ancestor` walk-up fallback.
- `TemplateChangeHandler` owns a `tokio::sync::Notify`-driven consumer
  with panic isolation via inner `tokio::spawn`.
- Per-template sha256 diffing in the consumer suppresses no-op
  broadcasts.
- Tier-parent directories watched recursively; existing `is_markdown`
  filter + canonical-path index keep scoping intact.
- Frontend `dispatchSseEvent` and `queryKeysForEvent` both route
  `template-changed` through a shared `templateKeysForEvent` helper.
- Index row renders three chips in `plugin-default → user-override →
  config-override` order using `TIER_SHORT_LABELS` and
  `chipVariantForTier`.
- Detail screen uses a two-column grid with `data-active="true"`
  outline on the winning tier.

#### Intentional Refinements During Design Fidelity

- **AC13 reinterpreted** — content-hash label now truncates to
  `sha256-<5 hex>…` via `truncateSha256`
  (`LibraryTemplatesView.tsx:136-139`) and surfaces the full digest
  on hover via `title={data.sha256}`
  (`LibraryTemplatesView.tsx:160`), with `data-full-sha` for test
  recovery (`LibraryTemplatesView.tsx:161`). Work item AC13 has been
  reconciled to match the shipped behaviour; the browser-native
  tooltip is the sole hover affordance. All other non-interactive
  contract bits (no `role`, no `tabindex`, no click handler, no
  hover styling, no cursor change) are preserved.
- **Markdown rendering swapped.** Phase 6 specified
  `<MarkdownRenderer content={winning.content} />`. Implementation
  uses `<TemplateHighlight content={winning.content} />`
  (`LibraryTemplatesView.tsx:169`), introduced during the design
  fidelity rounds to give the preview pane template-aware syntax
  highlighting.
- **Empty winning content path** — plan said the preview pane
  returns `null` when there is no winning tier. The implementation
  renders an explicit "No winning tier resolved." paragraph
  (`LibraryTemplatesView.tsx:144-149`) and a "tier not present"
  note when content is absent (`LibraryTemplatesView.tsx:170`).
- **AC7 / AC8 editorial follow-ups** flagged in the plan's
  Migration Notes have been applied to the work item (AC7:
  `/api/library/templates/{name}` → `/api/templates/{name}`; AC8:
  raw hex → `sha256-<64-char lowercase hex>` shape).

#### Potential Issues

- **Clippy regression** in `frontmatter.rs:319`
  (`redundant_closure`) is unrelated to 0042 scope but trips the
  plan's `cargo clippy -- -D warnings` verification line. Worth a
  separate cleanup.

### Manual Testing Required

The plan lists the following manual verifications. Code paths are in
place; visual confirmation is still required against a running server.

1. UI behaviour:
   - [ ] At ≥1024px, `/library/templates` shows three tier chips per
     row in fixed `default → user → config` order with neutral /
     indigo / green variants.
   - [ ] At ≥1024px, `/library/templates/{name}` shows a two-column
     grid with stacked tier cards on the left and the preview pane
     on the right.
   - [ ] Winning tier card has an accent-coloured outline ring;
     non-winning cards do not.
   - [ ] Preview header shows winning path on the left and a
     truncated content-hash label on the right (note: truncated +
     `title` tooltip, not the full literal the plan/AC13 specify).

2. Live SSE:
   - [ ] Editing the winning tier file on disk updates the
     content-hash label within ~1s without a page reload.
   - [ ] Emptying the winning tier file makes the content-hash
     label disappear (AC10 live path).
   - [ ] Editing a file that is both a tier and a doc produces both
     `template-changed` and `doc-changed` events.

### Recommendations

- Fix the unrelated clippy regression in `frontmatter.rs:319` so
  the plan's "clippy clean" verification line passes truthfully on
  future runs (out of 0042 scope).
