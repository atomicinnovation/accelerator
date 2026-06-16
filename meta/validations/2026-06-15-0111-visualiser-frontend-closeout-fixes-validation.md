---
type: plan-validation
id: "2026-06-15-0111-visualiser-frontend-closeout-fixes-validation"
title: "Validation Report: Visualiser Frontend Closeout Fixes Implementation Plan"
date: "2026-06-16T07:18:11+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
target: "plan:2026-06-15-0111-visualiser-frontend-closeout-fixes"
tags: ["visualiser", "frontend", "markdown", "lifecycle", "sidebar", "milestone-closeout"]
last_updated: "2026-06-16T07:18:11+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Visualiser Frontend Closeout Fixes

Validated fresh (post-`/clear`) by reconstructing the implementation from the
seven committed changes (`psmypvyo` … `nykzvrqm`), the clean working copy, the
diff against the pre-implementation parent (`rxxnxzul`), and a full automated
re-run. All six code phases plus the Closeout are present and faithful to the
plan; the plan even adopted its own optional suggestion (the
`workflowStagesComplete` helper). **Result: pass.**

### Implementation Status

- ✓ Phase 1 — Markdown content parity (M1 tables + M3 horizontal rules) — fully implemented
- ✓ Phase 2 — Code-block dark scrollbar (M2) — fully implemented
- ✓ Phase 3 — Detail-page action buttons must not wrap (L1) — fully implemented
- ✓ Phase 4 — Lifecycle overview wording (L3) — fully implemented
- ✓ Phase 5 — Muted META section + Templates link (L4) — fully implemented
- ✓ Phase 6 — Drop decisions from the lifecycle cluster (L2) — fully implemented
- ✓ Closeout — canonical Docker VR baselines regenerated; orphan `-darwin.png` cleanup done; work-item VR note corrected

### Automated Verification Results

- ✓ `mise run frontend:check` (Biome lint + format + `tsc -b`) — clean, isolated
- ✓ `mise run server:check` (rustfmt + clippy) — clean, isolated
- ✓ `mise run types:build-system:check` — 0 errors (isolated)
- ✓ `mise run test:unit:frontend` — **2536 passed** (122 files); matches plan
- ✓ `cargo test` (server) — **537 passed** (21 suites); matches plan, incl. the
  new `has_decision_does_not_push_decisions_into_present` assertion
- ✓ New VR specs + baselines committed:
  `dev-design-system-markdown.spec.ts` with `markdown-{light,dark}-visual-regression.png`
- ⚠️ `mise run check` (aggregate) exits non-zero — **NOT a 0111 defect** (see
  Potential Issues): a pre-existing concurrency race between `deps:install:node`
  and pyrefly's `**/*.py*` glob walking `node_modules` mid-install. Failed twice
  on *different* transient paths (`highlight.js/types`, then `indent-string`);
  passes deterministically in isolation. The plan touched no Python.
- ◻ Docker visual-regression suite (`test:e2e:visualiser:docker[:update]`) — **not
  independently re-run** in this validation (heavy, requires Docker). Relying on
  the committed canonical baselines and the Closeout's documented clean run.

### Code Review Findings

#### Matches Plan:

- **M1** — `table` override added to `MARKDOWN_COMPONENTS`
  (`MarkdownRenderer.tsx`) wrapping `<table>` in `.tableWrap`; CSS replaces the
  bare `table/th/td` rules with a wrapper owning `border` + `--radius-6` +
  `overflow: hidden` + `--ac-bg-card`, a Sora uppercase `--ac-fg-faint` header on
  `--ac-bg-sunken`, `--ac-stroke-soft` row top-borders, `:first-child` no-border,
  no striping/hover; the `td code` override is retained.
- **M2** — `--code-scrollbar-thumb`/`-track` declared in both `global.css`
  (`:root`-only `--code-*` block) and `tokens.ts` `CODE_SURFACE_TOKENS`; a single
  `.markdown pre` rule set adds `scrollbar-width`/`scrollbar-color` + WebKit
  pseudo-elements with `--radius-4`; no redundant `.codeblock pre` block.
- **M3** — `.markdown hr` renders a 1px `--ac-stroke` divider.
- **L1** — `white-space: nowrap` on `.btn`; `flex-shrink: 0` on `.actions`.
- **L3** — title `"Lifecycle overview"` and the prototype subtitle verbatim
  (ASCII apostrophe, "artifacts", semicolon); asserted verbatim in the test.
- **L4** — `.metaSection`/`.metaHeading`/`.link.metaLink` rules (0.7 × 0.75 net
  ≈ 0.525; `--size-125` + `--ac-fg-faint`) with the compound `.link.metaLink`
  specificity the plan flagged; class hooks added in `Sidebar.tsx`.
- **L2** — `decisions` removed from `LIFECYCLE_PIPELINE_STEPS` and `"hasDecision"`
  from `PipelineStepKey`; `Completeness.hasDecision`/`has_decision` retained with
  explanatory comments on both sides; Rust `STAGE_PUSH_ORDER` tuple removed;
  `CANONICAL_PRESENT_ORDER` down to 10 entries; denominators switched to
  `WORKFLOW_PIPELINE_STEPS.length`; numerators routed through the new
  `docType`-keyed `workflowStagesComplete` helper. Test sites updated 8→7;
  `data-stage="decisions"` assertions converted to negative (`toBeNull`); RCA
  absence regression test added; non-vacuous Rust test targets
  `derive_completeness` directly.

#### Deviations from Plan:

- **Token ledger (M2), cosmetic only** — the plan said *delete* the `0.4rem`
  `EXCEPTIONS` entry and *add* a separate `8px` entry. The implementation instead
  *edited the same entry in place* (literal `0.4rem` → `8px`, reason rewritten).
  Net ledger state is identical and the hygiene gate passes; functionally
  equivalent.
- **Helper extraction (improvement)** — the plan listed `workflowStagesComplete`
  as a "consider extracting" optional. It was extracted into `api/types.ts` and
  consumed by `Pipeline`, `PipelineMini`, and `LifecycleIndex`, removing the
  duplicated numerator derivation. Improvement over the minimum.

#### Potential Issues:

- **Pre-existing tooling flake (not introduced by 0111, but worth fixing):**
  `mise run check` is non-deterministic because the parallel `deps:install:node`
  task can be mid-install while `types:build-system:check` (pyrefly) globs
  `**/*.py*` into `frontend/node_modules`, hitting transiently-absent paths and
  failing the whole aggregate. This also masks behind the `mise … | tail` exit-code
  gotcha. It will intermittently red CI's check job. Consider scoping pyrefly's
  glob to exclude `node_modules`, or ordering `deps:install:node` before the type
  check. Independent of this work item.

### Manual Testing Required:

The plan's manual-verification items are largely covered by committed VR
baselines + resolved-styles specs; the following are worth an eyeball before
release (none independently re-run here):

1. Markdown (M1/M3):
  - [ ] Wide table clips with rounded corners, no horizontal scrollbar, both themes
  - [ ] `hr` renders as a faint `--ac-stroke` divider, both themes
2. Code block (M2):
  - [ ] Over-wide line shows a thin dark scrollbar (not OS-default), scrolls, no wrap
  - [ ] Confirm whether pinned Chromium captures the standard `scrollbar-width`
        thumb vs the 8px WebKit thumb (plan flagged the divergence)
3. Detail page (L1):
  - [ ] Long-title page: action labels stay single-line, buttons keep space
4. Sidebar (L4):
  - [ ] META block ≈0.525 effective opacity; Templates link 12.5px + faint —
        note the recorded, deliberate WCAG 1.4.3 departure (prototype-faithful)
5. Lifecycle (L2):
  - [ ] Cluster with an ADR shows no decision node; decisions still in sidebar +
        related-artifacts; counters read `N/7`

### Recommendations:

- **Ship** — implementation is complete and faithful; all runnable automated
  gates are green.
- File a separate tooling ticket for the `mise run check` pyrefly/node_modules
  concurrency race so it stops intermittently failing CI.
- Before merge, run the Docker VR suite once on the merge base
  (`mise run test:e2e:visualiser:docker`) to independently confirm the committed
  baselines compare clean — this validation trusted the Closeout's documented run
  rather than re-executing Docker.
