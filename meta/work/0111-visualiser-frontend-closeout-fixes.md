---
type: work-item
id: "0111"
title: "Visualiser Frontend Fixes for First Milestone Closeout"
date: "2026-06-15T10:26:21+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: story
priority: medium
relates_to: ["work-item:0097"]
tags: ["visualiser", "frontend", "polish", "markdown", "milestone-closeout"]
last_updated: "2026-06-15T16:23:39+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-133
---

# 0111: Visualiser Frontend Fixes for First Milestone Closeout

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a user of the visualiser, I want the remaining markdown-rendering, layout,
and lifecycle fixes surfaced during QA (quality assurance) applied, so that the
first prototype version is polished enough for a general release.

This is a **container story**: a set of small, QA-discovered frontend fixes that
close out the initial visualiser prototype. They are gathered here rather than
tracked as individual stories/bugs because each is too small to justify a full
research → plan → implement cycle on its own, and they ship together as the
milestone-closeout polish pass.

## Context

The visualiser frontend has reached the end of its initial prototype build
(work items 0033–0095 plus tooling 0100/0101/0108/0110, all done). Before a
general release of this first version, a QA pass against the design prototype
surfaced a handful of small fixes — mostly markdown-rendering and layout
parity issues, plus one small feature gap and one small feature removal.

Additional features have since been identified but are deliberately deferred;
the goal here is to release this first version, not to expand it.

The authoritative reference for the intended look and behaviour is the design
prototype at
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`.
Each fix below has been anchored to the prototype's concrete treatment.

## Requirements

Seven fixes, grouped by area. The `--ac-*` and `--size-*` tokens referenced
below are the project's shared design tokens, confirmed present in the frontend;
their definition locations are listed under Technical Notes.

### Markdown rendering

- **M1 — Table rendering parity.** Markdown tables must match the prototype:
  a wrapping element carrying `border-radius` + `overflow: hidden` (the
  border-collapse rounding workaround) and a 1px border; a recessed header-row
  fill (`--ac-bg-sunken`) with uppercase Sora label text in `--ac-fg-faint`;
  rows separated by top borders (`--ac-stroke-soft`); no row striping, no
  hover. Wide tables clip (no horizontal scroll), as in the prototype.
- **M2 — Code-block scrollbar styled dark.** Code blocks are always dark
  regardless of theme; their horizontal scrollbar must be styled dark to match
  (prototype: `::-webkit-scrollbar` height 8px, thumb `rgba(255,255,255,0.10)`
  radius 4px, transparent track). Long lines scroll horizontally; they do not
  wrap. The Firefox-standard `scrollbar-color` should also be set so the
  styling is not WebKit-only (a deliberate improvement over the prototype,
  which styles only the WebKit pseudo-element).
- **M3 — Muted horizontal rules.** Markdown horizontal rules must render as a
  faint, theme-reactive divider (the `--ac-stroke` token), not in the body
  text colour.

### Detail page

- **L1 — Action buttons must not wrap.** On the detail page, the "Open in
  editor" and "Copy path" buttons must not wrap their label text when the
  document title is long; they sit on their own row with ample space.

### Lifecycle

- **L2 — Remove decisions from the lifecycle cluster.** The lifecycle cluster
  visualisation must no longer include decisions (ADRs) as cluster nodes,
  matching the latest prototype, which excludes them (decisions are orthogonal —
  not every lifecycle has them). Decisions remain visible elsewhere on the page
  (related artifacts, etc.).
- **L3 — Lifecycle overview heading/subheading wording.** The lifecycle
  overview page's heading and subheading wording must match the prototype
  (wording only — styling is already correct).

### Navigation

- **L4 — Muted "META" section and "Templates" link.** The sidebar "META"
  section label and "Templates" link must match the prototype's deliberately
  quietest treatment: the faintest foreground token (`--ac-fg-faint`) plus the
  compounded opacity dampening (`0.7` on the whole `.ac-nav__meta` block, an
  extra `0.75` on the META label → ~0.525 net) and the slightly reduced item
  font-size (12.5px vs 13px).

## Acceptance Criteria

Markdown rendering:

- [ ] **M1** — Given a markdown document with a table, when rendered in either
      theme, then: (a) a wrapper element carries `border-radius` + `overflow:
      hidden`, so the outer corners are rounded and a wide table clips with no
      horizontal scrollbar; (b) the wrapper has a 1px border; (c) the header row
      is filled with `--ac-bg-sunken` and shows uppercase Sora label text in
      `--ac-fg-faint`; (d) rows are separated by top borders in `--ac-stroke-soft`;
      (e) there is no row striping and no hover treatment.
- [ ] **M2** — Given a code block with a line wider than the viewport, when
      viewed in light or dark theme, then a dark-styled thin scrollbar appears
      (no light/OS-default scrollbar) and horizontal scrolling works; long
      lines do not wrap.
- [ ] **M3** — Given a markdown horizontal rule, when rendered in either theme,
      then it is a 1px divider whose colour resolves to `--ac-stroke` (faint,
      theme-reactive) — distinct from, and lower-contrast than, the body-text
      token in both themes.

Detail page:

- [ ] **L1** — Given a detail page for a document with a long title, when the
      header renders, then each action button ("Open in editor", "Copy path")
      shows its label on a single line with no intra-button wrapping.

Lifecycle:

- [ ] **L2** — Given a lifecycle page for a work unit that has at least one
      associated decision/ADR, when the cluster renders, then no decision/ADR
      nodes appear in the cluster; and those decisions still appear in the page's
      related-artifacts listing(s).
- [ ] **L3** — Given the lifecycle overview page, when rendered, then the
      eyebrow reads "Lifecycle", the H1 reads "Lifecycle overview", and the
      subheading reads "Every work unit and how far it has progressed. Each row
      groups one unit's artifacts; the pipeline shows which stages it has
      reached."

Navigation:

- [ ] **L4** — Given the sidebar rendered in either theme, when the META section
      is inspected, then: (a) the META block has `opacity: 0.7`; (b) the "META"
      heading carries an additional `0.75` opacity (≈`0.525` net effective);
      (c) the Templates link's font-size is `12.5px` (vs `13px` / `--size-130` on
      other nav links); so that the META label's computed effective opacity
      resolves to ≈`0.525` (`0.7`×`0.75`), lower than the ~`0.75` effective
      opacity of other nav section headings, and the "Templates" link renders at
      a smaller font-size than other nav links.

## Dependencies

- Blocks: General release of the first visualiser version. (Tracked as a prose
  milestone only — there is no dedicated release work item to reference yet; if
  one is created, link it here so closing 0111 visibly unblocks a tracked
  artefact.)
- Requires: regenerating the canonical visual-regression baselines before merge,
  for the visible UI changes. **Correction:** there is no `-darwin`/`-linux`
  baseline split and no dedicated CI regen workflow — the repo renders a single
  canonical baseline set inside a pinned Playwright Chromium-on-Linux Docker
  container, regenerated locally with `mise run test:e2e:visualiser:docker:update`
  (requires Docker) and compared in CI's compare-only job. Because the same pinned
  image runs locally and in CI, locally-regenerated PNGs are authoritative. This
  is a local Docker step, not an out-of-band CI prerequisite (cross-reference work
  item 0108).
- Source of truth: correctness of M1, L2, L3, and L4 is gated on the named design
  prototype snapshot (see References) as the frozen authoritative source for the
  exact wording strings, opacity factors, and font sizes — reviewers diff the
  implementation against it.
- Related: 0097 (Strip redundant doc-type prefixes from artifact titles) — a
  separate polish bug in the same closeout spirit; not folded in here.
- Related: 0112 (Captured Screenshots Section on Design Inventory Pages) — fix
  F1 split out of this story to be tracked as a standalone feature.

## Assumptions

- **M1** mirrors the prototype's `overflow: hidden` on tables, so wide tables
  clip rather than scroll. If horizontal scrolling for wide tables is wanted,
  that changes the requirement.
- **L3** scope is the lifecycle **overview/index** page; the per-cluster detail
  head is data-driven and out of scope.
- **Priority** is medium — milestone-closeout polish with no fixed deadline.

## Open Questions

None outstanding. The one contingent decision — whether wide tables should
scroll or clip (M1) — is resolved under Assumptions (clip, mirroring the
prototype); revisit only if horizontal scrolling for wide tables is later wanted.

## Technical Notes

**Size**: M — seven small, discrete fixes spread across many areas. Four are
net-new CSS-only rules (M2, M3, L1, L4) and one is a text-only edit (L3); what
lifts it above S is M1 (new table wrapper element + `table` override in
`MarkdownRenderer.tsx`, not just CSS) and L2 (cross-file logic: reclassify
`decisions` in `api/types.ts`, fix the hardcoded `/8` completeness count, update
`LifecycleClusterView.test.tsx` + `cluster-via-label.ts`), plus visual-regression
baseline regeneration (darwin + linux) for the six visible UI changes. Of the
seven, **L2 carries the most risk** — it is the only fix with cross-file
behavioural logic (the rest are CSS or text), so it warrants focused review and
visual-regression attention.

- Authoritative visual reference for all seven fixes (the design prototype):
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`
  (`src/app.css`, `src/ui.jsx`, `src/view-library.jsx`, `src/view-lifecycle.jsx`,
  `src/app-shell.jsx`). The prototype uses `.ac-md-*` / `.ac-nav__meta` class
  names; the real frontend uses CSS-module classes, so these are visual targets,
  not literal selectors to copy.
- All seven fixes are frontend-only (React 19 + Vite SPA under
  `skills/visualisation/visualise/frontend/`). The one item needing server work
  (F1 — the captured-screenshots feature, originally the eighth closeout fix) was
  split out to 0112 and is not in scope here.
- **M1** (tables) — `src/components/MarkdownRenderer/MarkdownRenderer.module.css:65`
  styles bare `.markdown table/th/td` today; there is no wrapper element/class.
  A rounded-corner wrap needs a new wrapper (cf. the existing `.codeblock`
  wrapper pattern, `:33-56`) plus a custom `table` override in
  `MarkdownRenderer.tsx` (`MARKDOWN_COMPONENTS`, `:110-172`).
- **M2** (code scrollbar) — `.markdown pre` sets `overflow-x: auto`
  (`MarkdownRenderer.module.css:25`) but no `::-webkit-scrollbar` /
  `scrollbar-color` rules exist anywhere for code blocks. Both rules are net-new
  here. Code colours: `src/styles/code-syntax.global.css`.
- **M3** (`hr`) — no `.markdown hr` rule exists; markdown `---` renders as the
  UA-default `<hr>`. New rule, filled with `var(--ac-stroke)`. (Note: the detail
  page already strips YAML frontmatter before render — `LibraryDocView.tsx:34-37`
  — so the closing `---` is not mistaken for an `<hr>`.)
- **L1** (button wrap) — buttons are siblings in `Page`'s `.actions` row
  (`LibraryDocView.tsx:222-229`). `.headerTopRow` and `.actions` in
  `Page.module.css:23-28`/`:64-68` have no `flex-wrap` / `white-space` guards;
  the fix likely lands on the button label (`HeaderActionButton.module.css`) or
  the `.actions`/`.headerTopRow` container.
- **L2** (drop decisions) — `decisions` is a non-long-tail **workflow** step in
  `LIFECYCLE_PIPELINE_STEPS` (`src/api/types.ts:337-342`), so `buildTimeline()`
  (`LifecycleClusterView.tsx:136-158`) always emits it as a node or "No decision
  yet" placeholder. Removing it from `WORKFLOW_PIPELINE_STEPS` is the core change;
  it also touches the hardcoded `stagesComplete`/`/8` completeness count
  (`LifecycleIndex.tsx:22-27`, `:93-95`, `:127` → becomes `/7`), the
  `data-stage="decisions"` test assertions (`LifecycleClusterView.test.tsx`), and
  `cluster-via-label.ts` decision handling. Decisions must still surface in
  related-artifacts.
- **L3** (overview wording) — `LifecycleIndex.tsx:139-153` currently sets eyebrow
  "Lifecycle", title "Work units, from idea to shipped", subtitle "Each row is a
  slug-clustered work unit…". The criteria specify new title "Lifecycle overview"
  and a new subheading — a text-only edit on these props.
- **L4** (muted META/Templates) — there is **no `.ac-nav__meta` class and no
  opacity dampening** in the code: META is styled by the shared `.sectionHeading`
  (`Sidebar.module.css:222-231`, already `color: var(--ac-fg-faint)`,
  `font-size: var(--size-105)`) and the Templates link by `.link`
  (`:260-275`, `var(--ac-fg-muted)`, `var(--size-130)`). The only opacity in the
  sidebar is `.phaseHeading` (`opacity: 0.75`, `:239-249`). The prototype's
  compounded 0.7×0.75 dampening + 12.5px sizing would be net-new rules scoped to
  the META section/Templates link.
- Relevant shared tokens (all confirmed present in the frontend):
  `--ac-stroke`, `--ac-stroke-soft`, `--ac-bg-sunken`, `--ac-fg-faint`,
  `--ac-fg-muted`, `--ac-fg-strong`; code-block colours via `--code-*`
  (`code-syntax.global.css`).

## Drafting Notes

- Captured as a single **story** (kind) per the explicit request for one
  container item, rather than an epic with child work items — the fixes are too
  small to warrant decomposition and ship together. Epic was considered and
  rejected on those grounds.
- Scope boundary: existing open items 0097 (doc-type prefix stripping) and 0102
  (legacy linkage fallback removal, backend) are deliberately excluded — 0097 is
  noted as related, 0102 is out of scope.
- **M2** adds the Firefox-standard `scrollbar-color` beyond what the prototype
  does (WebKit-only). Flagged as an intentional cross-browser improvement.
- **F1** was extracted to its own story (0112 — Captured Screenshots Section on
  Design Inventory Pages) during refinement: it is the one closeout item that is
  both a new feature and outsized (net-new Rust server work plus SPA UI), so it
  warrants its own research → plan → implement cycle. The other seven fixes
  remain inline here; the fix count dropped from eight to seven accordingly.

## References

- Source: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`
- Related: 0097, 0112
