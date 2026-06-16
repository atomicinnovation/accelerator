---
type: plan
id: "2026-06-15-0111-visualiser-frontend-closeout-fixes"
title: "Visualiser Frontend Closeout Fixes Implementation Plan"
date: "2026-06-15T20:36:00+00:00"
author: Toby Clemson
producer: create-plan
status: done
work_item_id: "work-item:0111"
parent: "work-item:0111"
derived_from: ["codebase-research:2026-06-15-0111-visualiser-frontend-closeout-fixes"]
tags: ["visualiser", "frontend", "markdown", "lifecycle", "sidebar", "milestone-closeout"]
revision: "6bd95b32314656c07698ca08afda2eaed4415826"
repository: "visualisation-system"
last_updated: "2026-06-15T20:57:16+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Visualiser Frontend Closeout Fixes Implementation Plan

## Overview

Seven small, QA-discovered frontend fixes that close out the initial visualiser
prototype before its first general release. All live under
`skills/visualisation/visualise/frontend/` except L2, which also touches the Rust
server's stage-order constant. The design prototype at
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`
is the frozen source of truth for wording, opacity factors, and sizes.

## Current State Analysis

The frontend is at the end of its prototype build. A QA pass against the design
prototype surfaced parity gaps in markdown rendering (tables, code-block
scrollbar, horizontal rules), one detail-page layout bug (action buttons wrap),
one lifecycle-model change (decisions appear as a pipeline stage), one wording
edit (lifecycle overview), and one navigation-styling gap (META/Templates not
dampened). Research (`meta/research/codebase/2026-06-15-0111-...`) verified every
file/line reference and corrected three work-item assumptions (see Key
Discoveries).

## Desired End State

All seven acceptance criteria in work item 0111 pass; `mise run check` and the
full test suite are green; and the Docker visual-regression baselines are
regenerated for every affected surface so CI's compare-only job passes. The
markdown renderer matches the prototype's table/scrollbar/hr treatment, detail
action buttons never wrap, the lifecycle cluster shows no decision node, the
lifecycle overview wording matches the prototype verbatim, and the sidebar
META/Templates block carries the compounded opacity dampening + reduced font size
+ faint recolour.

### Key Discoveries

- **The work item's VR model is wrong.** There is no `-darwin`/`-linux` baseline
  split and no CI regen workflow. The repo renders a **single canonical baseline
  set inside a pinned Playwright Chromium-on-Linux Docker container**, regenerated
  locally with `mise run test:e2e:visualiser:docker:update` and compared in CI's
  compare-only job (`.github/workflows/main.yml:81-97`). Because the same pinned
  image runs locally and in CI, locally-regenerated PNGs are authoritative on CI.
  This removes the "out-of-band Linux CI regen" prerequisite but adds a **Docker
  requirement** for whoever regenerates.
- **"Drop RCA too" (L2) is already satisfied — zero code change.**
  `RootCauseAnalyses` is **not** in `LIFECYCLE_PIPELINE_STEPS` nor in the Rust
  `STAGE_PUSH_ORDER`; it maps to `u8::MAX` in `canonical_rank`
  (`server/src/clusters.rs:370`) and is pinned as an "out-of-lifecycle peer type"
  by `server/src/docs.rs:313`. `buildTimeline()`
  (`LifecycleClusterView.tsx:136-158`) iterates only `WORKFLOW_PIPELINE_STEPS` +
  `LONG_TAIL_PIPELINE_STEPS`, neither of which contains RCA, so RCA never renders
  as a cluster node today. L2 therefore reduces to **removing `decisions` only**.
- **The `.codeblock` wrapper is the precedent for M1.**
  `MarkdownRenderer.module.css:33-44` already uses a wrapper carrying
  `border` + `border-radius` + `overflow: hidden` with the inner element resetting
  its own border/radius — `border-collapse: collapse` makes that wrapper mandatory
  for visibly rounded table corners.
- **Code-block colours are intentionally theme-invariant** (`--code-*` declared
  only under `:root`, `global.css:294-324`, pinned by a fixture test), confirming
  M2's "always dark" premise.
- **`--size-125` (12.5px) already exists** (`global.css:190`), so L4 needs no new
  literal — ADR-0043 bans px literals.
- **The stage model has a cross-language parity contract**: frontend
  `LIFECYCLE_PIPELINE_STEPS` ↔ Rust `STAGE_PUSH_ORDER`, asserted by
  `pipeline-step-parity.test.ts` against a hardcoded `CANONICAL_PRESENT_ORDER`.
  The Rust order is the canonical superset; the frontend renders it **minus the
  backend-only `decisions` key**, and the parity test pins exactly that
  relationship.

## What We're NOT Doing

- **Not** touching the backend clustering / `present` model for L2. `decisions`
  stays in the Rust `STAGE_PUSH_ORDER` and `present`; the omission is frontend-only
  (dropped from `LIFECYCLE_PIPELINE_STEPS`), keeping the Rust side as the canonical
  present superset.
- **Not** excising the `hasDecision`/`has_decision` completeness boolean. Decisions
  must still cluster and surface in related-artifacts; we remove only its
  *frontend pipeline-stage membership*, retaining the recorded boolean to avoid
  churning
  10+ test fixtures (`api/types.ts:244`, `test-fixtures.ts:17`, `fetch.test.ts`,
  `router.test.tsx`, `DevDesignSystem.tsx`, etc.).
- **Not** dropping `root-cause-analyses` from the lifecycle cluster as a code change
  — it is already excluded (see Key Discoveries). We only confirm via test.
- **Not** touching the per-cluster lifecycle detail head (L3 is the overview/index
  page only).
- **Not** implementing F1 (captured screenshots) — split to work item 0112.
- **Not** adding horizontal scrolling for wide tables (M1 clips per the prototype).

## Implementation Approach

Six independently mergeable phases, ordered cheapest/lowest-risk first and the
cross-language L2 last. TDD where it applies: VR specs and unit assertions are
written or adjusted alongside each change. Every phase that alters visible UI
regenerates its affected Docker VR baselines and ends green on `mise run check` +
the test suite, so each can merge on its own. Visual-regression baseline
regeneration requires **Docker**.

---

## Phase 1: Markdown content parity (M1 tables + M3 horizontal rules)

### Overview

Add the rounded table wrapper + reworked header/row CSS (M1) and the muted `hr`
rule (M3). Both are unguarded by VR today and share a new `/dev#markdown` clip
spec, so they ship together. TDD: extend `MARKDOWN_SAMPLE` with a table (already
present) + a new horizontal rule, add the VR spec, then implement until the
captured baseline matches the prototype.

### Changes Required

#### 1. Table wrapper override (M1)

**File**: `src/components/MarkdownRenderer/MarkdownRenderer.tsx`
**Changes**: Add a `table` entry to `MARKDOWN_COMPONENTS` (`:110-172`) wrapping the
native `<table>` in a rounding wrapper, mirroring the existing `pre`/`.codeblock`
pattern.

```tsx
table({ children, node: _node, ...rest }) {
  return (
    <div className={styles.tableWrap}>
      <table {...rest}>{children}</table>
    </div>
  );
},
```

#### 2. Table + hr CSS (M1, M3)

**File**: `src/components/MarkdownRenderer/MarkdownRenderer.module.css`
**Changes**: Replace the bare `.markdown table/th/td` rules (`:65-67`) with a
wrapper that owns the border + radius + `overflow: hidden`, a recessed Sora
uppercase header fill, top-border row separators, and no striping/hover. Add a new
`.markdown hr` rule. Keep the existing `.markdown td code` override (`:70`).

```css
.tableWrap {
  margin: var(--sp-4) 0;
  border: 1px solid var(--ac-stroke);
  border-radius: var(--radius-6);
  overflow: hidden;            /* clips wide tables; rounds outer corners */
  background: var(--ac-bg-card);
}
.markdown table { width: 100%; border-collapse: collapse; }
.markdown thead th {
  text-align: left;
  font-family: var(--ac-font-display);   /* Sora */
  font-weight: 600;
  font-size: var(--size-115);             /* 11.5px */
  letter-spacing: var(--tracking-caps);
  text-transform: uppercase;
  color: var(--ac-fg-faint);
  padding: var(--sp-2) var(--sp-3);       /* 8px 12px — tokenised */
  background: var(--ac-bg-sunken);
  border-bottom: 1px solid var(--ac-stroke);
}
.markdown tbody td {
  padding: var(--sp-2) var(--sp-3);
  border-top: 1px solid var(--ac-stroke-soft);
  color: var(--ac-fg);
  vertical-align: top;
}
.markdown tbody tr:first-child td { border-top: none; }

/* M3 — muted, theme-reactive divider (prototype .ac-md-hr). */
.markdown hr {
  border: 0;
  height: 1px;
  background: var(--ac-stroke);
  margin: var(--sp-6) 0;
}
```

Notes: header `border-bottom` uses the stronger `--ac-stroke`; row separators use
the softer `--ac-stroke-soft`; per-cell outer borders are dropped so the wrapper
border isn't doubled. Cell padding is tokenised to `var(--sp-2) var(--sp-3)`
(8px×12px) — 8px is within ADR-0026 §2's ±2px tolerance band of the prototype's
9px and avoids adding new px literals to the file's `EXCEPTIONS` ledger.

This CSS adds **four `1px` borders** (the `.tableWrap` border, the `thead th`
`border-bottom`, the `tbody td` `border-top`, and the `hr` `height`) but also
**removes one** existing `1px` border: the rewritten `.markdown th, .markdown td`
rule (`MarkdownRenderer.module.css:66`) currently carries `border: 1px solid
var(--ac-stroke-soft)`. Net `1px` delta is therefore **+3** (7 − 1 + 4 = **10**),
not +4. That same line-66 rule also holds the file's only `0.4rem` padding, which
the tokenised `var(--sp-2)` removes. See the ledger step (§4) for the exact
count/entry changes both facts require. (`--radius-6` is already a declared token,
so it needs no ledger change.)

#### 3. VR fixture + spec (guards M1 + M3)

**File**: `src/components/DevDesignSystem/DevDesignSystem.tsx`
**Changes**: Add a horizontal rule (`---`) to `MARKDOWN_SAMPLE` (it already
contains a table) so both M1 and M3 appear in the `/dev#markdown` section. The
existing sample table is a narrow 2×2; **widen it** (more columns or long cell
text that exceeds the container) so the M1 clip-vs-scroll boundary — the literal
`overflow: hidden` AC — is actually exercised by the VR snapshot and the
resolved-styles check (assert `scrollWidth > clientWidth` with no scrollbar).
Otherwise a regression dropping `overflow: hidden` passes both gates because no
rendered table is wide enough to reveal it.

**File**: `tests/visual-regression/dev-design-system-markdown.spec.ts` (new)
**Changes**: New clip spec capturing the `#markdown` section in light + dark,
modelled on `dev-design-system-code-syntax.spec.ts`.

#### 4. Token-ledger update (mechanical, required to stay green)

**File**: `src/styles/migration.test.ts`
**Changes**: Two edits, both required for the exact-equality hygiene gate
(`observed === declared`):
1. Bump the `MarkdownRenderer.module.css` `1px` `EXCEPTIONS` entry (`:296-303`)
   from `count: 7` to `count: 10` (7 − 1 removed per-cell border + 4 new
   hairlines). Rewrite the `reason` to **drop** the obsolete "table cell" phrase
   and name the surviving + new uses (tableWrap border, thead bottom, tbody top,
   hr height).
2. **Delete** the `0.4rem` `EXCEPTIONS` entry for `MarkdownRenderer.module.css`
   (`:304-310`) — tokenising the cell padding to `--sp-2` removes the file's only
   `0.4rem` occurrence, so a `count: 1` declaration against 0 observed would fail.

No new entry is needed for padding (tokenised) or radius (`--radius-6` is
declared).

#### 5. Structural assertion (complements the VR snapshot)

**File**: new `tests/resolved-styles/markdown-table-resolved-styles.spec.ts`
(the resolved-styles suite lives under `tests/resolved-styles/`, e.g.
`dev-design-system-code-block-resolved-colours.spec.ts` — model the new spec on
those).
**Changes**: Add a cheap DOM/computed-style assertion that the `.tableWrap`
wrapper exists carrying `overflow: hidden` (the literal M1 AC and the
border-collapse rounding workaround), that a wide table's `scrollWidth >
clientWidth` with no scrollbar (clip, not scroll), and that the rendered `hr`'s
computed background resolves to the `--ac-stroke` value — so a structural
regression that slips under the VR `maxDiffPixelRatio` budget is still caught.

### Success Criteria

#### Automated Verification

- [x] Frontend check passes: `mise run frontend:check`
- [x] Unit tests pass (incl. the bumped `1px` ledger count): `mise run test:unit:frontend`
- [x] Resolved-styles assertion passes (`.tableWrap` has `overflow: hidden`; `hr`
      resolves to `--ac-stroke`)
- [ ] New VR baselines render and compare clean: `mise run test:e2e:visualiser:docker`
      — **deferred to Closeout** (full canonical regen; markdown specs verified
      passing in the interim regen)
- [ ] Baselines regenerated: `mise run test:e2e:visualiser:docker:update` produces
      `dev-design-system-markdown` snapshots — **deferred to Closeout**

#### Manual Verification

- [x] Table renders with rounded outer corners, 1px border, `--ac-bg-sunken`
      uppercase Sora header in `--ac-fg-faint`, top-border row separators, no
      striping, no hover (verified on the regenerated `markdown` baselines, both themes)
- [x] Horizontal rule renders as a faint `--ac-stroke` 1px divider, lower-contrast
      than body text in both themes (verified on the regenerated baselines)

---

## Phase 2: Code-block dark scrollbar (M2)

### Overview

Add dark horizontal-scrollbar styling to the always-dark code blocks, routed
through tokens (ADR-0026/0039), plus the Firefox-standard `scrollbar-color` /
`scrollbar-width` (a deliberate cross-browser improvement over the WebKit-only
prototype). Long lines scroll, never wrap.

### Changes Required

#### 1. Theme-invariant scrollbar tokens

**File**: `src/styles/global.css`
**Changes**: Add scrollbar tokens alongside the existing `:root`-only `--code-*`
block (`:294-324`) so they resolve identically in both themes, satisfying
ADR-0026.

```css
--code-scrollbar-thumb: rgba(255, 255, 255, 0.10);
--code-scrollbar-track: transparent;
```

#### 1b. Declare the new tokens in the token registry (required to stay green)

**File**: `src/styles/tokens.ts`
**Changes**: Add `--code-scrollbar-thumb` and `--code-scrollbar-track` to
`CODE_SURFACE_TOKENS` (`:329`). The "var(--NAME) references resolve to declared
tokens" gate in `migration.test.ts` builds its `declared` set from these exports,
so any `var(--code-scrollbar-*)` reference in step 2 is an undeclared-token
failure until they are listed here. The gate the two new tokens must satisfy is
the `tokens.ts ↔ global.css :root` parity test (`global.test.ts:113`), which
iterates `CODE_SURFACE_TOKENS` and asserts each matches `global.css` — declaring
in both files (step 1 + 1b) satisfies it. (No `prototype-tokens.fixture.test.ts`
change is needed: that test iterates the committed prototype fixture, not
`CODE_SURFACE_TOKENS`, so extra non-prototype tokens are ignored.)

#### 2. Scrollbar rules on the code scroll surface

**File**: `src/components/MarkdownRenderer/MarkdownRenderer.module.css`
**Changes**: Target the `.markdown pre` / `.codeblock pre` scroll surface
(`overflow-x: auto` is at `:25`). Use `--radius-4` for the 4px thumb radius
(ADR-0039) and the dual WebKit + Firefox declaration precedent
(`RootLayout.module.css:33-56`).

```css
.markdown pre { scrollbar-width: thin; scrollbar-color: var(--code-scrollbar-thumb) var(--code-scrollbar-track); }
.markdown pre::-webkit-scrollbar { height: 8px; }
.markdown pre::-webkit-scrollbar-thumb { background: var(--code-scrollbar-thumb); border-radius: var(--radius-4); }
.markdown pre::-webkit-scrollbar-track { background: var(--code-scrollbar-track); }
```

Selector resolution: `.codeblock pre` resets `border`/`border-radius`/`margin`
but **not** `overflow-x`, so it inherits the `.markdown pre` scroll surface — a
single `.markdown pre` rule set covers both the bare and language-labelled code
blocks. Do **not** add a redundant `.codeblock pre` scrollbar block.

#### 3. Token-ledger update (mechanical, required to stay green)

**File**: `src/styles/migration.test.ts`
**Changes**: The `8px` scrollbar height is a new px literal in
`MarkdownRenderer.module.css`, which has no existing `8px` `EXCEPTIONS` entry. Add
one (`count: 1`, `kind: "irreducible"`, reason: WebKit scrollbar track height —
no token) or the AC4 px-literal gate fails. (`--radius-4` is a declared token and
needs no entry.)

### Success Criteria

#### Automated Verification

- [x] Frontend check passes: `mise run frontend:check`
- [x] Unit tests pass (incl. the new `8px` ledger entry + declared scrollbar
      tokens): `mise run test:unit:frontend`
- [ ] Resolved-styles + code-block colour tests pass: `mise run test:e2e:visualiser:docker`
      (recompare `dev-design-system-code-syntax` baselines; regenerate if cell render shifts)
      — **deferred to Closeout** (full canonical regen)

#### Manual Verification

- [ ] A code block with a line wider than the viewport shows a thin dark scrollbar
      (no light/OS-default chrome) in both light and dark themes; horizontal scroll
      works; long lines do not wrap
- [ ] Scrollbar appears styled in Firefox (via `scrollbar-color`), not only WebKit
- [ ] When regenerating `dev-design-system-code-syntax` baselines, confirm which
      rendering the pinned Chromium actually captures: modern Chromium honours the
      standard `scrollbar-width`/`scrollbar-color` and ignores `::-webkit-scrollbar`
      when both are present, so the baseline may show a thin standard scrollbar
      rather than the 8px WebKit thumb — align this verification wording with what
      the runner produces

---

## Phase 3: Detail-page action buttons must not wrap (L1)

### Overview

Stop "Open in editor" / "Copy path" labels wrapping when the document title is
long. Minimal, targeted guards on the shared button + actions row.

### Changes Required

#### 1. No intra-button wrapping

**File**: `src/components/DetailHeaderActions/HeaderActionButton.module.css`
**Changes**: Add `white-space: nowrap;` to `.btn` (`:5-18`) — the literal AC.

#### 2. Stop the title compressing the actions block

**File**: `src/components/Page/Page.module.css`
**Changes**: Add `flex-shrink: 0;` to `.actions` (`:64-68`) so a long title cannot
squeeze the buttons.

**Decision (recorded, not incidental)**: `.btn` and `.actions` are shared by
every page using `Page`, so these guards are global. We **intentionally** change
the shared primitive's contract rather than scoping via the `data-slot="actions"`
hook (`Page.tsx:41`), because `nowrap` + no-shrink on action pills is universally
desirable; the `data-slot` hook remains available if a LibraryDocView-only scope
is ever needed later.

**Fixture note**: L1's failure mode is specifically a *long title* compressing
`.actions`. Before regenerating the `library-doc-view` baselines, confirm the
fixture renders a title long enough to reproduce the original wrap; if not, add a
long-title fixture so the no-wrap guard is exercised at the boundary that actually
triggered the bug (otherwise the baseline can regenerate clean without testing the
fix).

### Success Criteria

#### Automated Verification

- [x] Frontend check passes: `mise run frontend:check`
- [x] The L1 guards are pinned deterministically by a resolved-styles spec
      (`detail-actions-nowrap.spec.ts`: `.btn` computes `white-space: nowrap`,
      `[data-slot="actions"]` computes `flex-shrink: 0`) — chosen over a
      long-title server fixture (which would churn server fixture-count tests)
      because the computed-style assertion guards the fix independent of title
      length
- [ ] `library-doc-view` VR baselines compare clean / regenerate:
      `mise run test:e2e:visualiser:docker:update` — **deferred to Closeout**

#### Manual Verification

- [ ] On a detail page with a long title, each action button shows its label on a
      single line with no intra-button wrapping; buttons keep ample space

---

## Phase 4: Lifecycle overview wording (L3)

### Overview

Text-only edit of the lifecycle overview/index `<Page>` props to match the
prototype verbatim.

### Changes Required

#### 1. Eyebrow / title / subtitle strings

**File**: `src/routes/lifecycle/LifecycleIndex.tsx`
**Changes**: At `:143-147`, keep eyebrow "Lifecycle"; change `title` to
`"Lifecycle overview"` and `subtitle` to the prototype string (ASCII apostrophe,
US "artifacts", semicolon after "artifacts"):

```tsx
title="Lifecycle overview"
subtitle="Every work unit and how far it has progressed. Each row groups one unit's artifacts; the pipeline shows which stages it has reached."
```

### Success Criteria

#### Automated Verification

- [x] Frontend check passes: `mise run frontend:check`
- [x] `LifecycleIndex.test.tsx:82-95` updated to assert the new title and the
      full subtitle **verbatim** (exact-string match, not substring regex):
      `cd skills/visualisation/visualise/frontend && npx vitest run -t "LifecycleIndex"`

#### Manual Verification

- [ ] `/lifecycle` shows eyebrow "Lifecycle", H1 "Lifecycle overview", and the new
      subheading verbatim

---

## Phase 5: Muted META section + Templates link (L4)

### Overview

Add the prototype's compounded opacity dampening (0.7 on the block × 0.75 on the
META heading → ~0.525 net), the reduced 12.5px Templates font-size, and the faint
recolour — scoped to the META section only, isolated from the full-opacity VIEWS
heading.

### Changes Required

#### 1. Scoped class hooks

**File**: `src/components/Sidebar/Sidebar.tsx`
**Changes**: On the META `<section>` (`:152-174`) add `metaSection` to the
`section`, `metaHeading` to the `<h2>`, and `metaLink` to the Templates `<Link>`
(composed with existing `section`/`sectionHeading`/`link` classes).

#### 2. Dampening rules

**File**: `src/components/Sidebar/Sidebar.module.css`
**Changes**: Add three rules (the multiplying-opacity precedent is `.phaseHeading`,
`:239-249`):

```css
.metaSection { opacity: 0.7; }
.metaHeading { opacity: 0.75; }                 /* compounds → ~0.525 net */
.metaLink { font-size: var(--size-125); color: var(--ac-fg-faint); }
```

### Success Criteria

#### Automated Verification

- [x] Frontend check passes: `mise run frontend:check`
- [x] L4 values pinned by a resolved-styles spec (`sidebar-meta-muted.spec.ts`:
      `.metaSection` opacity 0.7, `.metaHeading` opacity 0.75, Templates link
      12.5px + `--ac-fg-faint`). Note: `.link.metaLink` needed compound
      specificity to beat the base `.link` size/colour — caught by the spec.
- [ ] All full-page VR baselines (sidebar renders in `RootLayout`) regenerate and
      compare clean: `mise run test:e2e:visualiser:docker:update` — **deferred to
      Closeout**

#### Manual Verification

- [ ] META block computes `opacity: 0.7`; META heading effective opacity ~0.525
      (lower than the ~0.75 of other section headings); Templates link is 12.5px
      (vs 13px elsewhere) and recoloured `--ac-fg-faint` — in both themes
- [x] Contrast check: the Templates `<Link>` composited colour (~0.525 opacity
      over the sidebar bg) falls below WCAG 1.4.3 4.5:1, especially in dark theme.
      **Recorded as a deliberate, prototype-faithful departure**: the META block
      is intentionally the sidebar's quietest treatment per the frozen prototype
      (the source of truth for opacity factors); the faint value is not raised so
      the implementation matches the prototype exactly

---

## Phase 6: Drop decisions from the lifecycle cluster (L2)

### Overview

Remove `decisions` from the **frontend** pipeline-stage model
(`LIFECYCLE_PIPELINE_STEPS`), switch the hardcoded `8` denominators to the
self-deriving `WORKFLOW_PIPELINE_STEPS.length`, and update tests. The **backend
clustering is left unchanged** — `decisions` stays in the Rust `STAGE_PUSH_ORDER`
and is still pushed into `completeness.present`; the SPA simply does not render or
count it. Decisions remain clustered and visible in related-artifacts (the
`hasDecision`/`has_decision` boolean is retained). RCA needs no change (already
excluded) — add a regression test asserting it. TDD: update the parity test +
cluster assertions first, then make them green.

**Cross-surface scope (intentional).** `WORKFLOW_PIPELINE_STEPS` is consumed not
only by the lifecycle cluster but also by `Pipeline` (lifecycle-index cards) and
`PipelineMini` (kanban work-item cards), and all three gate rendering on
`present.has(step.docType)`. Dropping `decisions` from `WORKFLOW_PIPELINE_STEPS`
therefore removes the decision tile/dot — and the workflow-stage numerator no
longer counts it — on the lifecycle **index** and the **kanban board** too, even
though `present` still carries `"decisions"` from the backend. This is deliberate:
"decisions is not a linear pipeline stage" holds on every surface, matching the
prototype's intent. Because these surfaces change, their VR baselines are in scope
for the closeout
(see the updated Closeout + Testing Strategy sections): the kanban
work-item-card baseline and the lifecycle-index card baseline must regenerate
alongside the lifecycle-cluster `tokens.spec` baselines.

### Changes Required

#### 1. Frontend stage list + parity anchor

**File**: `src/api/types.ts`
**Changes**: Remove the `decisions` entry from `LIFECYCLE_PIPELINE_STEPS`
(`:337-342`) and the `"hasDecision"` member from the `PipelineStepKey` union
(`:283`). **Retain** `Completeness.hasDecision` (`:244`).

**File**: `src/api/pipeline-step-parity.test.ts`
**Changes**: **Keep** `"decisions"` in `CANONICAL_PRESENT_ORDER` (11 entries) so
it still mirrors the unchanged Rust `STAGE_PUSH_ORDER`. Assert that
`LIFECYCLE_PIPELINE_STEPS` equals that order with `"decisions"` filtered out —
the frontend renders the backend present order minus the backend-only decisions
key.

#### 2. Rust stage-push order — left unchanged

**File**: `server/src/clusters.rs`
**Changes**: **None to the clustering behaviour.** `decisions` is the canonical
`present` model and the backend stays as-is: the `(|c| c.has_decision, "decisions")`
tuple remains in `STAGE_PUSH_ORDER`, decisions still cluster and are still pushed
into `present` server-side. This is the chosen scope (see What We're NOT Doing):
the lifecycle view is a **frontend** decision, so the Rust side is the canonical
superset and the SPA simply omits `decisions` from `LIFECYCLE_PIPELINE_STEPS`.
The only edit here is a clarifying comment on `STAGE_PUSH_ORDER` documenting that
`decisions` is intentionally a backend-only `present` key with no rendered
lifecycle stage, and that the frontend order must match this literal **minus**
`decisions` (pinned by the parity test). No new Rust assertion is added — the
behaviour being asserted (present *includes* decisions) is the pre-existing,
already-covered server behaviour.

#### 3. Self-deriving stage-count denominators

**Files**: `src/components/Pipeline/Pipeline.tsx:29`,
`src/components/PipelineMini/PipelineMini.tsx:14`,
`src/routes/lifecycle/LifecycleIndex.tsx:127`.
**Changes**: Replace the hardcoded `8` / `of 8` / `/8` with
`WORKFLOW_PIPELINE_STEPS.length` (or interpolate it) so the denominator becomes 7
automatically and never drifts again. **Note**: `DevDesignSystem.tsx` is *not* in
this list — verification showed it already uses `WORKFLOW_PIPELINE_STEPS.length`
throughout (no hardcoded `8` near `:908`), so no edit is needed there.

Also fix the **numerator domain** in the two aria-labels. `present.size` counts
the full server `present` set, which includes the three long-tail keys (notes,
design-inventories, design-gaps), so against a 7-stage denominator a fully
populated cluster reads a nonsensical "10 of 7 stages complete". Count only
workflow-present stages, matching how `stagesComplete` is already derived in
`LifecycleIndex.tsx:93-95`. **Key on `docType`, not `key`**: `present` is a `Set`
of kebab-case `DocTypeKey` strings (the render loops use `present.has(step.docType)`,
`Pipeline.tsx:32`), whereas `step.key` is the camelCase `PipelineStepKey`
(`"hasWorkItem"`); filtering on `s.key` would match nothing and render "0 of 7".

```tsx
// Pipeline.tsx / PipelineMini.tsx — numerator filtered to workflow stages
const workflowComplete = WORKFLOW_PIPELINE_STEPS.filter((s) =>
  present.has(s.docType),
).length;
aria-label={`Lifecycle pipeline, ${workflowComplete} of ${WORKFLOW_PIPELINE_STEPS.length} stages complete`}
// LifecycleIndex.tsx
{stagesComplete}/{WORKFLOW_PIPELINE_STEPS.length}
```

Consider extracting a single `workflowStagesComplete(present)` helper alongside
`WORKFLOW_PIPELINE_STEPS` in `api/types.ts` so this numerator domain is defined
once rather than duplicated across `Pipeline`, `PipelineMini`, and the existing
`LifecycleIndex.tsx:93-95` derivation.

#### 4. Test updates

Dropping `decisions` lowers the rendered stage count from 8 to 7, so **every**
`toHaveLength(8)` / `N/8` assertion driven by `WORKFLOW_PIPELINE_STEPS` must move
to 7. The full set of sites (all verified against the current tree) is:

**`src/components/Pipeline/Pipeline.tsx` → `Pipeline.test.tsx`**: `:13`
`toHaveLength(8)` → `7`, and retitle the test at `:8` ("renders exactly **eight**
stage tiles…" → "…**seven**…").

**`src/components/PipelineMini/PipelineMini.tsx` → `PipelineMini.test.tsx`**:
`:15` `toHaveLength(8)` → `7`, and retitle the test at `:8` ("renders **eight**
`<li>` dots…" → "…**seven**…") so the title matches its assertion.

**`src/routes/lifecycle/LifecycleClusterView.test.tsx`**: timeline length assertion
(`:86`, `toHaveLength(8)` → `7`); remove the `data-stage="decisions"` assertions
(`:91-95`) and the `ADR Foo` decision-node assertion (`:119`). This view has **no
related-artifacts surface** (`RelatedArtifacts` is rendered by `LibraryDocView`,
not here), so assert only the **negative** here: no `data-stage="decisions"` node
renders for a cluster that has a decision entry.

**`src/routes/lifecycle/LifecycleIndex.test.tsx`**: update the title at `:104`
("renders a Pipeline + **N/8** counter…" → `N/7`) and the copy assertions at
`:112-123`. The older cluster's `1/8` (`:114`) → `1/7`. The newer cluster's `4/8`
(`:123`) → **`3/7`**: its fixture `present` (`:58-64`) is
`["work-items", "plans", "plan-reviews", "decisions"]` — four entries, all four
currently workflow stages, so dropping `decisions` lowers the workflow-present
count to 3 against a denominator of 7. Critically, **remove/replace** the
`:124-128` block that does `querySelector('[data-stage="decisions"]')!.getAttribute(...)`
— after removal that query returns `null` and the non-null assertion throws. Also
update the separate test "renders 8 pipeline tiles per card" (`:198-209`,
`toHaveLength(8)` at `:207` → `7`).

**`src/components/DevDesignSystem/DevDesignSystem.test.tsx`**: **no stage-count
change needed** — it already derives counts from `WORKFLOW_PIPELINE_STEPS.length`
(`:288-302`). Do **not** touch `:267` (`toHaveLength(8)`): that asserts the **8
statuses**, unrelated to pipeline stages.

**New regression tests** (TDD — write first):
- Assert a decision still appears in the **related-artifacts** listing for a work
  unit that has one — the half of AC L2 that `LifecycleClusterView.test.tsx`
  cannot cover. Anchor this at the **`LibraryDocView` integration level** (where a
  cluster's decision entry actually flows into the related-artifacts lists after
  the `present`/`hasDecision` contract changes), not in an isolated
  `RelatedArtifacts` unit test — `RelatedArtifacts` is purely prop-driven and
  already proves a decision renders (`RelatedArtifacts.test.tsx`), so it cannot
  regress from the L2 stage-model change and would be a vacuous guard. The
  `tokens.spec` lifecycle VR is an acceptable alternative anchor.
- An RCA-bearing cluster renders **no** RCA node (locks in the already-true
  behaviour).
- Wording-only: when the `ADR Foo` decision-node assertion is dropped, reword the
  `LifecycleClusterView.test.tsx:115` test title ("renders one timeline step per
  present entry") — post-L2 the fixture's `present` still contains `"decisions"`
  but that entry no longer renders a timeline step, so the current title
  misdescribes the behaviour (e.g. "…per present workflow/long-tail entry").

### Success Criteria

#### Automated Verification

- [x] Frontend check passes: `mise run frontend:check`
- [x] Server check passes: `mise run server:check`
- [x] Parity test passes (`CANONICAL_PRESENT_ORDER` mirrors the unchanged Rust
      `STAGE_PUSH_ORDER` = 11 entries; the frontend renders that order minus the
      backend-only `decisions` key; part of the frontend unit suite)
- [x] Full unit suites pass — all `8 → 7` sites updated (`Pipeline`,
      `PipelineMini`, `LifecycleClusterView`, `LifecycleIndex`), `data-stage="decisions"`
      query removed: `mise run test:unit:frontend` (2536) and
      `cargo test` (537)
- [x] Pipeline aria-label numerator + denominator pinned by a **partial** fixture
      (`Pipeline.test.tsx`: present `["work-items","plans","notes"]` ⇒ "2 of 7
      stages complete"), guarding the `docType`-filtered numerator vs the old
      `present.size` over-count
- [x] Backend clustering left unchanged: `decisions` is still pushed into
      `present` by `STAGE_PUSH_ORDER`; the frontend simply omits it from
      `LIFECYCLE_PIPELINE_STEPS` so it never renders or counts as a stage
- [ ] Lifecycle-cluster (`tokens.spec`), lifecycle-index card, and kanban
      work-item-card VR baselines regenerate and compare clean: **deferred to
      Closeout** (full canonical regen)

#### Manual Verification

- [x] A lifecycle cluster shows **no** decision node in the cluster (verified on
      the regenerated `/lifecycle/first-plan` baseline: 7-stage pipeline, no
      decision tile), and decisions still surface (sidebar nav + related-artifacts,
      `RelatedArtifacts.test.tsx`)
- [x] Pipeline counters read `N/7`; aria-labels say "of 7 stages" (unit-asserted
      in `Pipeline.test.tsx`/`LifecycleIndex.test.tsx`)
- [x] An RCA-bearing work unit shows no RCA node in the cluster (regression test
      in `LifecycleClusterView.test.tsx`)

---

## Closeout (not a code phase): VR baselines + work-item correction

- [x] Regenerated the canonical Docker baseline set after all phases landed
  (`mise run test:e2e:visualiser:docker:update`) and verified with
  `mise run test:e2e:visualiser:docker`. **Note:** Playwright's
  `--update-snapshots` only rewrites baselines that exceed the `maxDiffPixelRatio`
  budget, so the L2 (decision-tile removal) and L4 (muted sidebar) changes — being
  within tolerance on the full-page shots — were silently *not* rewritten by a
  plain update. To produce a truly clean canonical set, the baselines were
  **deleted and fully regenerated**; the genuinely-changed set then correctly
  included `tokens.spec` lifecycle-cluster + kanban (7-tile pipeline), the
  `kanban-card` cells (7 PipelineMini dots), all full-page `library-*` shots
  (muted META sidebar), `code-syntax` (M2 scrollbar gutter), and the new
  `dev-design-system-markdown` baselines. Visually verified: the lifecycle
  pipeline shows 7 stages with no decision tile, and decisions still appear in the
  sidebar nav.
- [x] Deleted the four stray orphan `-darwin.png` files under
  `tests/visual-regression/__screenshots__/library-doc-view.spec.ts-snapshots/`.
- [x] Corrected work item 0111's VR-dependency note — there is no
  `-darwin`/`-linux` split or dedicated CI regen workflow; it is a local Docker
  step (cross-referenced work item 0108).

(Operational note: a leftover E2E host-server process from a prior compare run
held resources and made one regen attempt fail fast with "host server exited
(code 1) before publishing its port" — clearing the orphaned process resolved it.)

## Testing Strategy

### Unit Tests

- L2: parity test (`CANONICAL_PRESENT_ORDER` = 11 entries, mirroring the unchanged
  Rust `STAGE_PUSH_ORDER`; `LIFECYCLE_PIPELINE_STEPS` = that order minus the
  backend-only `decisions` key); the full set of `8 → 7` count assertions
  (`Pipeline`, `PipelineMini`, `LifecycleClusterView`, `LifecycleIndex` incl. the
  `:198-209` per-card test); removal of the `data-stage="decisions"` query in
  `LifecycleIndex.test.tsx`; no-decision-node regression (negative, in the cluster
  view); decision-still-in-related-artifacts regression (in a
  `LibraryDocView`/`RelatedArtifacts` test or VR); RCA-absence regression;
  aria-label "N of 7" assertion. The backend is unchanged, so no new Rust
  assertion is needed.
- L3: `LifecycleIndex` wording assertions — exact verbatim subtitle string.

### Integration / Visual Regression

- New `dev-design-system-markdown` spec (M1/M3); `dev-design-system-code-syntax`
  (M2); `library-doc-view` with a long-title fixture (L1); all full-page baselines
  (L4 sidebar); and for L2 **three** surfaces that all lose the decision tile/dot:
  the `tokens.spec` lifecycle-cluster, the lifecycle-**index** card, and the
  **kanban** work-item-card baselines. All via the Docker VR runner.

### Manual Testing Steps

1. Render a markdown doc with a wide table + an `hr` in both themes (M1, M3).
2. Render a code block with an over-wide line in both themes; confirm dark thin
   scrollbar + no wrap (M2).
3. Open a detail page with a long title; confirm buttons don't wrap (L1).
4. Visit `/lifecycle`; confirm wording (L3).
5. Inspect the sidebar META/Templates block opacity + font-size (L4).
6. Open a lifecycle cluster with an ADR; confirm no decision node, decision in
   related-artifacts, counter `N/7` (L2).

## Performance Considerations

None — CSS rules, a wrapper element, string edits, and a constant removal.

## Migration Notes

No data migration. The L2 stage-order parity contract (frontend
`LIFECYCLE_PIPELINE_STEPS` ↔ Rust `STAGE_PUSH_ORDER` ↔ `CANONICAL_PRESENT_ORDER`)
is the cross-surface coupling; the Rust order is unchanged and the frontend +
`CANONICAL_PRESENT_ORDER` are kept consistent with it (frontend = Rust order minus
`decisions`) in Phase 6.

Note that the serialised `present: string[]` field is **unchanged** — a cluster
with a decision still emits `hasDecision: true` and still contains `"decisions"`
in `present`. The embedded SPA simply does not render or count it, because all its
readers (`Pipeline`, `PipelineMini`, `LifecycleIndex`, `DevDesignSystem`) gate on
`WORKFLOW_PIPELINE_STEPS`, edited in
the same change; the SPA is the sole consumer of `/api/lifecycle` (bundled via
`rust-embed`), so no out-of-tree reader of `present` is affected.

## References

- Work item: `meta/work/0111-visualiser-frontend-closeout-fixes.md`
- Research: `meta/research/codebase/2026-06-15-0111-visualiser-frontend-closeout-fixes.md`
- Prototype source of truth:
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`
  (`src/app.css:923-928` M1, `:913-916` M2, `:777` M3, `:577-579` L4;
  `src/view-lifecycle.jsx:5-11,66-71,95-97` L2/L3)
- VR model: `meta/work/0108-local-docker-visual-regression-baselines.md`
- ADRs: `ADR-0026` (colour tokens), `ADR-0039` (border-radius), `ADR-0043`
  (pure-numeric `--size-*`)
