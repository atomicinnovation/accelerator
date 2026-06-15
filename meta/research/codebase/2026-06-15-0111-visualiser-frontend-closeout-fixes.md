---
type: codebase-research
id: "2026-06-15-0111-visualiser-frontend-closeout-fixes"
title: "Research: Visualiser Frontend Closeout Fixes (work item 0111)"
date: "2026-06-15T19:23:00+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0111"
parent: "work-item:0111"
topic: "Visualiser Frontend Closeout Fixes (work item 0111)"
tags: [research, codebase, visualiser, frontend, markdown, lifecycle, sidebar, design-tokens, visual-regression]
revision: "656a1635e693f969f7d082b22204eafeb1756c75"
repository: "visualisation-system"
last_updated: "2026-06-15T19:23:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Visualiser Frontend Closeout Fixes (work item 0111)

**Date**: 2026-06-15T19:23:00+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 656a1635e693f969f7d082b22204eafeb1756c75 (working copy; parent `dd56f3e1` — "Record review approving visualiser closeout work item 0111")
**Branch**: anonymous jj change (not pushed; `main` bookmark is behind)
**Repository**: visualisation-system (jj workspace)

## Research Question

For the story at `meta/work/0111-visualiser-frontend-closeout-fixes.md`: research the
seven QA-discovered frontend fixes (M1 table parity, M2 code-block scrollbar, M3
horizontal rules, L1 detail-page button wrap, L2 drop decisions from the lifecycle
cluster, L3 lifecycle overview wording, L4 muted META/Templates nav) — confirm the
current code state, anchor each fix to the design prototype, and surface anything
that changes how the work should be planned.

## Summary

All seven fixes are well-scoped and the work item's per-fix technical notes are
**largely accurate** (file paths and line numbers verified). The prototype is a
faithful, frozen source of truth — L3's exact wording, M1/M2/M3 CSS, and L4's
opacity/font factors all match the work item verbatim. Three findings, however,
**materially change the plan** and should be resolved before implementation:

1. **The visual-regression (VR) model in the work item is wrong.** There is **no
   `-darwin`/`-linux` baseline split and no CI workflow to regenerate Linux
   baselines.** The repo renders a **single canonical baseline set inside a pinned
   Playwright Docker (Chromium-on-Linux) container**, regenerated locally with
   `mise run test:e2e:visualiser:docker:update`. The work item's "Requires: a
   Linux VR baseline regeneration via the dedicated CI workflow" dependency does
   not reflect the codebase as it stands — there is no such workflow. This removes
   the out-of-band prerequisite entirely (it becomes a local Docker step) but adds
   a **Docker requirement** for whoever regenerates.

2. **L2 ("drop decisions") has two very different implementation strategies, and
   the prototype uses the *lighter* one the work item did not propose.** The work
   item's technical note proposes editing `LIFECYCLE_PIPELINE_STEPS` in
   `api/types.ts` — a shared data-model change that ripples into the Rust server
   (`STAGE_PUSH_ORDER`), a frontend↔server parity test, Pipeline aria-labels, the
   `/dev` page, and Kanban. **The prototype instead leaves the shared `STAGES`
   model intact and filters only at the lifecycle view** via a
   `LIFECYCLE_OMIT = ["decisions", "root-cause-analyses"]` constant. The view-layer
   approach is smaller, lower-risk, and matches the prototype's stated rationale.

3. **The prototype omits *root-cause-analyses* (RCA) from the lifecycle cluster
   alongside decisions** — the work item (L2) only mentions decisions. RCAs were
   only just surfaced in the visualiser (work item 0110). This is an **open
   question**: should L2 also drop RCA to match the prototype, or is the work item
   deliberately scoping to decisions only?

Secondary findings: M1 (tables) and M3 (horizontal rules) currently have **no VR
coverage at all**, and the lifecycle **overview/index** page (relevant to L3) has
no VR coverage either — so "regenerate baselines" is not enough for those; new
specs would be needed to guard them. The `--size-125` (12.5px) token already
exists, so L4 needs no literal. Code-block colours are theme-invariant `--code-*`
tokens, confirming M2's "always dark" premise.

---

## Detailed Findings

### M1 — Markdown table rendering parity

**Current state** (`src/components/MarkdownRenderer/`):
- `MARKDOWN_COMPONENTS` (`MarkdownRenderer.tsx:110-172`) overrides only `pre`,
  `input`, `ul`, `li`. **`table`, `code`, and `hr` have no JS override** — they
  emit native elements styled by CSS.
- The existing **`.codeblock` wrapper pattern** is the exact precedent to mirror:
  the `pre` override (`:111-124`) wraps the native `<pre>` in
  `<div className={styles.codeblock}>`, and `.codeblock`
  (`MarkdownRenderer.module.css:33-39`) owns `border` + `border-radius` +
  `overflow: hidden`; `.codeblock pre` (`:40-44`) resets the inner element's
  border/radius to zero.
- Today's table CSS (`MarkdownRenderer.module.css:65-70`) styles bare
  `.markdown table/th/td` with `border-collapse: collapse` and per-cell
  `1px solid var(--ac-stroke-soft)` borders; `th` gets `var(--ac-bg-sunken)`.
  There is **no wrapper element** and no rounding.

**Why a wrapper is required:** `border-collapse: collapse` means a `border-radius`
on the `<table>` will not visibly clip the cell borders. The prototype solves this
with a wrapper carrying `overflow: hidden` — the same trick as `.codeblock`.

**Prototype target** (`prototype-full/src/app.css:923-928`, JSX `src/ui.jsx:190-212`):
```css
.ac-md-tablewrap { margin: 14px 0; border: 1px solid var(--ac-stroke);
  border-radius: 6px; overflow: hidden; background: var(--ac-bg-card); }
.ac-md-table { width: 100%; border-collapse: collapse; font-size: 13.5px; line-height: 1.55; }
.ac-md-table thead th { text-align: left; font-family: var(--ac-font-display); /* Sora */
  font-weight: 600; font-size: 11.5px; letter-spacing: 0.06em; text-transform: uppercase;
  color: var(--ac-fg-faint); padding: 9px 14px; background: var(--ac-bg-sunken);
  border-bottom: 1px solid var(--ac-stroke); }
.ac-md-table tbody td { padding: 9px 14px; border-top: 1px solid var(--ac-stroke-soft);
  color: var(--ac-fg); vertical-align: top; }
.ac-md-table tbody tr:first-child td { border-top: none; }
```
Note: header border-bottom is the **stronger `--ac-stroke`**; row separators are
the **softer `--ac-stroke-soft`**; no `:nth-child` striping and no `:hover` rule
exist (confirmed absent). The JSX is `div.ac-md-tablewrap > table > thead/tbody`.

**Plan implication:** add a `table` entry to `MARKDOWN_COMPONENTS` returning
`<div className={styles.tableWrap}><table {...rest}>…</table></div>`, add a
`.tableWrap` rule (border + radius + `overflow: hidden`), and rework the existing
`th/td` rules into header-fill + top-border-separator form. The outer cell borders
should be dropped so the wrapper border isn't doubled.

### M2 — Code-block scrollbar styled dark

**Current state:** `.markdown pre` (`MarkdownRenderer.module.css:19-27`) sets
`overflow-x: auto` (`:25`) — the real horizontal-scroll surface — but **no
`::-webkit-scrollbar` or `scrollbar-color` rule exists anywhere** in the module
(confirmed by grep; scrollbar styling lives only in `RootLayout`, `FilterPill`,
`Sidebar`, `DevDesignSystem`). The inner `.codeblock pre` (`:40-44`) does **not**
reset `overflow-x`, so it inherits the scroll surface.

**Code blocks are always dark — confirmed.** `--code-*` and `--tk-*` tokens are
declared **only under `:root`** in `global.css:294-324` (explicit comment: "Same
values resolve in both light and dark themes; declared only under `:root` by
design"). Values are dark navy: `--code-bg: #0e1320`, `--code-stroke:
rgba(255,255,255,0.07)`, etc. A `prototype-tokens.json` fixture pins these
byte-for-byte (`src/styles/fixtures/prototype-tokens.json:2-6`).

**Prototype target** (`prototype-full/src/app.css:913-916`, WebKit-only):
```css
.ac-codeblock .ac-md-pre::-webkit-scrollbar { height: 8px; }
.ac-codeblock .ac-md-pre::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.10); border-radius: 4px; }
.ac-codeblock .ac-md-pre::-webkit-scrollbar-track { background: transparent; }
```

**Plan implication:** add fresh rules targeting the `.markdown pre` /
`.codeblock pre` scroll surface. The work item deliberately extends the prototype
with the Firefox-standard `scrollbar-color` + `scrollbar-width` (the dual-declaration
precedent is `RootLayout.module.css:33-56` and `FilterPill.module.css:154-165`).
**Token-rule caveat:** the thumb colour `rgba(255,255,255,0.10)` and `4px`/`8px`
literals may collide with the CSS token-application conventions (ADR-0026 for
colour, ADR-0039 for radius). Either derive from the dark `--code-*` palette / add
theme-invariant tokens, or confirm scrollbar pseudo-elements are exempt before
hardcoding.

### M3 — Muted horizontal rules

**Current state:** **no `.markdown hr` rule exists** — markdown `---` renders as a
UA-default `<hr>`. (The detail page already strips YAML frontmatter before render —
`LibraryDocView.tsx:34-37` — so the closing `---` is not mistaken for an `<hr>`.)

**Prototype target** (`prototype-full/src/app.css:777`):
```css
.ac-md-hr { border: 0; height: 1px; background: var(--ac-stroke); margin: 28px 0; }
```

**Plan implication:** one new `.markdown hr` rule; no JS override needed. `hr`
already falls through to a native element.

### L1 — Detail-page action buttons must not wrap

**Layout hierarchy** (the header is rendered by the shared `Page` component, not
`LibraryDocView` directly):
- `LibraryDocView.tsx:222-229` supplies the `actions` slot:
  `<OpenInEditorButton/>` + `<CopyPathButton/>`, both → `HeaderActionButton`.
- `Page.tsx:25-46` lays them out: `.headerTopRow` (`Page.module.css:23-28`) is a
  non-wrapping flex **row** (`justify-content: space-between`) holding an
  **unclassed** title `<div>` and `.actions` (`Page.module.css:64-68`).
- **No guards exist:** `.headerTopRow` has no `flex-wrap`; `.actions` has no
  `flex-shrink: 0` / `min-width` / `white-space`; `HeaderActionButton .btn`
  (`HeaderActionButton.module.css:5-18`) has no `white-space: nowrap` and its label
  is a plain unclassed `<span>{label}</span>` (`HeaderActionButton.tsx:44`). A long
  title therefore shrinks `.actions`, compressing each button until its label wraps.

**Plan implication (three layered options):**
1. `white-space: nowrap` on `.btn` (prevents intra-button label wrapping — the
   literal AC).
2. `flex-shrink: 0` on `.actions` (stops the title from compressing the block).
3. To make actions "sit on their own row" the title block needs a class hook
   (it is currently unclassed at `Page.tsx:32`) so it can take `flex: 1`/
   `min-width`, plus `flex-wrap: wrap` on `.headerTopRow`. **Caveat:** `.actions`,
   `.headerTopRow`, and `HeaderActionButton` are **shared by every page** using
   `Page`, so changes are global, not LibraryDocView-only. `data-slot="actions"`
   (`Page.tsx:41`) is available as a more targeted selector hook.

### L2 — Remove decisions from the lifecycle cluster (highest-risk fix)

**Current state — decisions is a workflow pipeline step in the shared model:**
- `LIFECYCLE_PIPELINE_STEPS` (`api/types.ts:288-364`) has 11 entries; the decisions
  step (`:337-342`, `key: "hasDecision"`, `docType: "decisions"`) has **no
  `longTail: true`**, so it lands in `WORKFLOW_PIPELINE_STEPS` (`:366-372`) as the
  8th workflow step.
- `buildTimeline()` (`LifecycleClusterView.tsx:136-158`) iterates
  `WORKFLOW_PIPELINE_STEPS`, emitting a decision node when present or a "No decision
  yet" placeholder when absent.
- Hardcoded `8` appears in: `LifecycleIndex.tsx:127` (`{stagesComplete}/8`),
  `Pipeline.tsx:29` and `PipelineMini.tsx:14` (aria-label `…of 8 stages`), and
  `DevDesignSystem.tsx:908`. (`stagesComplete` itself is derived from
  `WORKFLOW_PIPELINE_STEPS` and auto-updates; the `/8` literal does not.)
- **Cross-boundary parity contract:** `pipeline-step-parity.test.ts:8-23` pins
  `CANONICAL_PRESENT_ORDER` (decisions at index 7) and asserts it equals
  `LIFECYCLE_PIPELINE_STEPS.map(s => s.docType)` **and** must match
  `STAGE_PUSH_ORDER` in `server/src/clusters.rs`. So editing the shared model
  forces a **coordinated Rust server change** — this is not frontend-only.
- Tests asserting decisions: `LifecycleClusterView.test.tsx:86` (`toHaveLength(8)`),
  `:91-95` (`data-stage="decisions"`), `:119` (`ADR Foo` node);
  `LifecycleIndex.test.tsx:104,112-123` (`N/8` copy).
- `cluster-via-label.ts` has **no** decisions-specific case (falls through to
  default) — no change needed there.
- Decisions remain visible via `RelatedArtifacts.tsx:42-139` (type-agnostic), the
  `/library/decisions` routes, and `RelatedCluster` — so the AC's "decisions still
  appear elsewhere" holds regardless of approach.

**Two strategies:**

| | A. Shared-model edit (work item's note) | B. View-layer omit (prototype's approach) |
|---|---|---|
| Change | Remove/flag decisions in `LIFECYCLE_PIPELINE_STEPS` | Add `LIFECYCLE_OMIT` constant, filter in the lifecycle view only |
| Touches | `api/types.ts`, **Rust `clusters.rs`**, parity test, Pipeline/PipelineMini aria-labels, `/dev`, all `/8`→`/7` | Lifecycle view + its count denominator only; shared `STAGES` intact (kanban, `/dev` unaffected) |
| Risk | High — cross-language, cross-surface | Low — localised, matches prototype intent |

The **prototype uses B** (`prototype-full/src/view-lifecycle.jsx:5-11`):
```js
// Decisions and root-cause analyses don't form part of a linear lifecycle …
// for the interim we drop them from the lifecycle pipeline/timeline.
// (window.STAGES stays intact for kanban + the design-system view.)
const LIFECYCLE_OMIT = ["decisions", "root-cause-analyses"];
const LIFECYCLE_STAGES = window.STAGES.filter(s => !LIFECYCLE_OMIT.includes(s.key));
```
and re-filters the completeness denominator the same way
(`view-lifecycle.jsx:95-97`). **Recommendation: adopt strategy B** — it is smaller,
avoids the server parity change, and is what the frozen reference actually does.
This should be confirmed with the planner, since the work item's technical note
assumed strategy A.

### L3 — Lifecycle overview heading/subheading wording (text-only)

**Current state** (`LifecycleIndex.tsx:139-153`): eyebrow "Lifecycle", title
**"Work units, from idea to shipped"**, subtitle "Each row is a slug-clustered work
unit. Filled tiles mark the stages present on disk. Missing stages are where the
workflow has gaps."

**Prototype target** (`prototype-full/src/view-lifecycle.jsx:66-71`, verbatim) —
matches the work item's AC exactly:
- Eyebrow: `Lifecycle` (with framed lifecycle glyph)
- H1: `Lifecycle overview`
- Subheading: `Every work unit and how far it has progressed. Each row groups one
  unit's artifacts; the pipeline shows which stages it has reached.`
  (ASCII apostrophe in "unit's", US spelling "artifacts", semicolon after
  "artifacts".)

**Plan implication:** pure prop/string edit on the existing `<Page>` props. Scope
is the **overview/index** page only (per the work item's L3 assumption); the
per-cluster detail head is data-driven and out of scope.

### L4 — Muted META section + Templates link

**Current state** (`Sidebar.module.css` + `Sidebar.tsx`): **no `.ac-nav__meta`
class and no opacity dampening exist** (confirmed). META and VIEWS section labels
**share** `.sectionHeading` (`:222-231`, `color: var(--ac-fg-faint)`,
`font-size: var(--size-105)`, full opacity); nav links share `.link` (`:260-275`,
`var(--ac-fg-muted)`, `var(--size-130)` = 13px). The only existing opacity is
`.phaseHeading { opacity: 0.75 }` (`:239-249`) on LIBRARY phase labels. The META
`<section>` (`Sidebar.tsx:152-174`) is gated on `templates &&`, carries a stable
`aria-labelledby="meta-heading"`, and the Templates link is its sole `<li>`.

**The `--size-125` (12.5px) token already exists** (`global.css:190`), so the
12.5px requirement needs no literal — use `var(--size-125)` (a literal would also
violate ADR-0043's no-px-literals rule).

**Prototype target** (`prototype-full/src/app.css:577-579`):
```css
.ac-nav__meta { opacity: 0.7; }
.ac-nav__meta .ac-nav__label { opacity: 0.75; }            /* compounds → 0.525 net */
.ac-nav__meta .ac-nav__item { color: var(--ac-fg-faint); font-size: 12.5px; }
```
Note the prototype also **recolours the item to `--ac-fg-faint`** (the work item's
L4 focuses on opacity + font-size but doesn't call out the recolour; the current
`.link` is `--ac-fg-muted`).

**Plan implication:** add three classes in `Sidebar.tsx` — `metaSection` on the
`<section>`, `metaHeading` on the `<h2>`, `metaLink` on the Templates `<Link>` — and
three CSS rules: `.metaSection { opacity: 0.7 }`, `.metaHeading { opacity: 0.75 }`
(compounds to 0.525 via the multiplying opacity chain — same precedent as
`.phaseHeading`), `.metaLink { font-size: var(--size-125) }` (and optionally
`color: var(--ac-fg-faint)` to match the prototype). This cleanly isolates META
from the full-opacity VIEWS heading.

### Design tokens — all confirmed present (`src/styles/global.css`)

Single source file (no `tokens*.css`). All six `--ac-*` tokens defined in light
`:root` and mirrored in both dark blocks (`[data-theme="dark"]` canonical +
`@media (prefers-color-scheme: dark)` fallback):

| Token | Light | Dark |
|---|---|---|
| `--ac-stroke` | `rgba(32,34,49,0.10)` | `rgba(255,255,255,0.08)` |
| `--ac-stroke-soft` | `rgba(32,34,49,0.06)` | `rgba(255,255,255,0.04)` |
| `--ac-bg-sunken` | `#f4f6fa` | `#070b12` |
| `--ac-fg-faint` | `#8b90a3` | `#6c7088` |
| `--ac-fg-muted` | `rgb(95,99,120)` | `#a0a5b8` |
| `--ac-fg-strong` | `rgb(10,17,27)` | `#ffffff` |

`--size-*` scale (`global.css:178-196`) is "pure-numeric px×10": `--size-105` =
10.5px, `--size-130` = 13px, `--size-125` = 12.5px. Theming is a `data-theme`
attribute selector with a `prefers-color-scheme` fallback (mirror parity guarded by
`global.test.ts`).

### Visual regression — the work item's model is incorrect

**Actual setup:**
- Screenshot specs run **only** via `playwright.docker.config.ts` inside a pinned
  Playwright Chromium-on-Linux Docker image. `snapshotPathTemplate`
  (`:23-24`) has **no `{platform}` token** — a single canonical baseline set, named
  `<arg>-visual-regression.png` under
  `tests/visual-regression/__screenshots__/<spec>-snapshots/`.
- `playwright.config.ts` (native) runs only `resolved-styles` (computed-style) and
  a kanban E2E project — **no screenshots**.
- **Regenerate:** `mise run test:e2e:visualiser:docker:update` (local, Docker
  required). **Verify:** `mise run test:e2e:visualiser:docker`. CI
  (`.github/workflows/main.yml:81-97`) runs the **compare-only** job; there is **no
  `workflow_dispatch` baseline-regen workflow**. Because the same pinned image runs
  locally and in CI, locally-regenerated PNGs are authoritative on CI too.
- Four stray orphan `-darwin.png` files exist under
  `library-doc-view.spec.ts-snapshots/` (no live template emits `-darwin`); they are
  not part of the canonical set and can be deleted.

**Coverage impact per fix:**

| Fix | VR baselines affected | Notes |
|---|---|---|
| M1 tables | **None today** | `/dev#markdown` not screenshotted; `MARKDOWN_SAMPLE` has a table but isn't captured. New spec needed to guard. |
| M2 code scrollbar | `dev-design-system-code-syntax.spec.ts-snapshots/*` (18, if cell render changes) | hits `/dev#code` |
| M3 hr | **None today** | no HR in any VR fixture; `MARKDOWN_SAMPLE` lacks one |
| L1 buttons | `library-doc-view.spec.ts-snapshots/library-doc-view{,-rca}-{light,dark}` | full-page doc capture |
| L2 cluster | `tokens.spec.ts-snapshots/lifecycle-cluster{,-after-click}-{light,dark}` (4) | route `/lifecycle/first-plan` |
| L2/L3 overview | **None today** | `/lifecycle` index has only resolved-styles coverage |
| L4 sidebar | **Every full-page baseline** | Sidebar renders in `RootLayout`, captured by all `tokens.spec.ts` full-page shots + all `library-doc-view` shots |

**Plan implication:** "regenerate the six baselines" understates it in two
directions — L4 touches *every* full-page baseline (broad), while M1/M3 and the
L2/L3 overview page have *no* baselines at all (so new specs are needed to guard
them, or they ship unguarded). Add an HR to `MARKDOWN_SAMPLE` and a `#markdown`
clip spec if M1/M3 should be VR-guarded.

## Code References

- `src/components/MarkdownRenderer/MarkdownRenderer.tsx:110-172` — `MARKDOWN_COMPONENTS`; `pre`/`.codeblock` wrapper precedent for M1
- `src/components/MarkdownRenderer/MarkdownRenderer.module.css:19-70` — `pre` (`overflow-x:auto`), `.codeblock`, table CSS; no scrollbar/hr rules
- `src/styles/code-syntax.global.css:27-81` + `src/styles/global.css:294-324` — theme-invariant dark `--code-*`/`--tk-*` tokens (M2 "always dark")
- `src/components/Page/Page.tsx:25-46` + `Page.module.css:23-28,64-68` — header layout; `.headerTopRow`/`.actions` lack wrap/shrink guards (L1)
- `src/components/DetailHeaderActions/HeaderActionButton.module.css:5-18` — `.btn` lacks `white-space:nowrap` (L1)
- `src/api/types.ts:288-372` — `LIFECYCLE_PIPELINE_STEPS` / `WORKFLOW_PIPELINE_STEPS`; decisions step `:337-342` (L2)
- `src/api/pipeline-step-parity.test.ts:8-23` — frontend↔Rust `STAGE_PUSH_ORDER` parity contract (L2 strategy-A blocker)
- `src/routes/lifecycle/LifecycleClusterView.tsx:136-158` — `buildTimeline()` emits decision node/placeholder (L2)
- `src/routes/lifecycle/LifecycleIndex.tsx:93-95,127,139-153` — `stagesComplete`/`/8` count + eyebrow/title/subtitle (L2, L3)
- `src/components/Sidebar/Sidebar.tsx:152-174` + `Sidebar.module.css:222-275` — META section + `.sectionHeading`/`.link`; no dampening (L4)
- `src/styles/global.css:189-190` — `--size-130`=13px, `--size-125`=12.5px (L4)
- `playwright.docker.config.ts:19-39` — single-baseline Docker VR config (no platform split)
- `tasks/test/e2e.py:34-133`, `mise.toml:177-194`, `.github/workflows/main.yml:81-97` — VR regen/compare wiring

**Prototype source of truth** (`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/`):
- `src/app.css:923-928` (M1 tables), `:913-916` (M2 scrollbar), `:777` (M3 hr), `:577-579` (L4 META)
- `src/ui.jsx:185,190-212` (M3/M1 JSX)
- `src/view-lifecycle.jsx:5-11,66-71,95-97` (L2 omit list + RCA, L3 wording, count denominator)
- `src/data.jsx:39-41,45-55` (META/Templates + `STAGES` definition)

## Architecture Insights

- **Wrapper-for-rounding is an established pattern** (`.codeblock`); M1 should reuse
  it rather than invent a new approach. `border-collapse: collapse` makes the
  `overflow: hidden` wrapper mandatory for visible rounded corners.
- **The shared pipeline-step model has a cross-language parity contract** (frontend
  `LIFECYCLE_PIPELINE_STEPS` ↔ Rust `STAGE_PUSH_ORDER`, guarded by a test). The
  prototype deliberately avoided touching it by filtering at the view layer —
  reflecting a "lifecycle is a linear stage list; decisions/RCAs are a graph
  relationship, not a stage" design stance (prototype comment). This is the safer
  altitude for L2.
- **Code-block colours are intentionally theme-invariant** (`:root`-only `--code-*`),
  pinned by a fixture test — M2 colours must respect that (derive from `--code-*` or
  hardcode like the prototype, mindful of ADR-0026/0039 token rules).
- **Opacity compounds down the DOM** — L4's 0.525 net is `0.7 × 0.75` across the
  block and label, matching the `.phaseHeading` precedent.
- **VR is single-baseline-in-Docker**, not platform-split — a recent change (work
  item 0108) that the 0111 work item's dependencies section predates.

## Historical Context

- `meta/decisions/ADR-0043-pure-numeric-typography-size-token-naming.md` — pure-numeric `--size-*` scheme (supersedes ADR-0036); governs L4's `var(--size-125)`
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` + `ADR-0035` (brand-layer indirection) — colour token rules relevant to M1/M2/M3 colours
- `meta/decisions/ADR-0039-border-radius-consumption-rule.md` — radius-token rule relevant to M1 wrapper + M2 scrollbar `4px`
- `meta/research/codebase/2026-05-31-0040-pipeline-visualisation-overhaul.md` + `meta/plans/2026-05-31-0040-pipeline-visualisation-overhaul.md` — the lifecycle pipeline-step model L2 touches
- `meta/work/0108-local-docker-visual-regression-baselines.md` + `meta/research/codebase/2026-06-12-0108-local-docker-visual-regression-baselines.md` — the single-baseline Docker VR model (corrects the work item's VR dependency)
- `meta/work/0110-surface-root-cause-analyses-in-visualiser.md` — RCAs were only just surfaced; relevant to the L2/RCA open question
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md` — prior gap analysis vs the same prototype snapshot
- `meta/work/0076-code-block-syntax-highlight-palette.md`, `0094` (inline code), `0088` (markdown body width), `0095` (markdown checkboxes) — prior MarkdownRenderer work

## Related Research

- `meta/research/codebase/2026-05-12-0053-sidebar-nav-and-unseen-tracker.md` — sidebar nav structure (L4)
- `meta/research/codebase/2026-06-13-0099-remap-typography-size-scale-to-pure-numeric-tokens.md` — `--size-*` remap (L4 token)
- `meta/research/codebase/2026-05-06-0033-design-token-system.md` — `--ac-*` token foundation

## Open Questions

1. **L2 strategy (A vs B):** the work item's technical note proposes editing the
   shared `LIFECYCLE_PIPELINE_STEPS` (strategy A — pulls in a Rust server change +
   parity test). The prototype uses a view-layer omit list (strategy B — no server
   change). Which does the planner want? (Recommendation: B, matching the
   prototype.)
2. **RCA in L2:** the prototype's `LIFECYCLE_OMIT` drops **both** `decisions` *and*
   `root-cause-analyses`. The work item L2 only names decisions. Should L2 also drop
   RCA from the lifecycle cluster to match the prototype, given RCA was just added
   (0110)? This needs an explicit decision.
3. **M1/M3 VR coverage:** these have no baselines today. Should the work add a
   `/dev#markdown` VR spec (and an HR to `MARKDOWN_SAMPLE`) to guard them, or ship
   them unguarded?
4. **M2 colour-token compliance:** does the dark scrollbar thumb hardcode
   `rgba(255,255,255,0.10)` (prototype-faithful) or route through a token to satisfy
   ADR-0026/0039? Confirm scrollbar pseudo-elements are exempt before hardcoding.
5. **L4 recolour:** the prototype recolours the Templates item to `--ac-fg-faint`;
   the work item L4 emphasises opacity + font-size. Include the recolour to match
   the prototype, or leave the link at `--ac-fg-muted`?
