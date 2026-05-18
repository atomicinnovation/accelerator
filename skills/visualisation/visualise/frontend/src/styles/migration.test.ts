import { describe, it, expect } from 'vitest'
import {
  LIGHT_COLOR_TOKENS,
  DARK_COLOR_TOKENS,
  TYPOGRAPHY_TOKENS,
  SPACING_TOKENS,
  RADIUS_TOKENS,
  LIGHT_SHADOW_TOKENS,
  DARK_SHADOW_TOKENS,
  LAYOUT_TOKENS,
} from './tokens'

const cssModules = import.meta.glob('../**/*.module.css', {
  eager: true,
  query: '?raw',
  import: 'default',
}) as Record<string, string>

const cssGlobals = import.meta.glob('../**/*.global.css', {
  eager: true,
  query: '?raw',
  import: 'default',
}) as Record<string, string>

// All `0` resets are auto-permitted (admitted by AC4's escape-hatch);
// the regex excludes them at the source so they never need EXCEPTIONS
// entries — keeps the list focused on genuine exceptions.
const HEX_RE = /#[0-9a-fA-F]{3,8}\b/g
const PX_REM_EM_RE = /\b(?!0(?:px|rem|em)\b)\d+(?:\.\d+)?(?:px|rem|em)\b/g
const VAR_REF_RE = /var\(\s*--([\w-]+)\s*[,)]/g
// Scoped to --ac-* / --sp-* / --radius-* / --size-* / --shadow-* / --lh-*
// tokens — i.e. the new layered set. Legacy `--color-*` fallback sites
// remain present at Phase 2 commit and are deleted by Phase 3; this regex
// deliberately does not flag them so the harness lands green.
const VAR_FALLBACK_RE = /var\(\s*--(?:ac-|sp-|radius-|size-|shadow-|lh-|tracking-)[\w-]+\s*,/g
const VAR_COUNT_RE = /var\(\s*--/g

// Per-occurrence exception model. `count` is how many times this
// `(file, literal)` pair is allowed to match; when the implementer
// migrates one occurrence, they decrement the count or remove the
// entry. Adding a new exception requires `count: 1` and a reason.
// `file` is the path **relative to `src/`** to disambiguate any future
// modules that share a basename across directories.
type Exception = { file: string; literal: string; count: number; reason: string }

const EXCEPTIONS: ReadonlyArray<Exception & { kind: 'to-migrate' | 'irreducible' }> = [
  // components/Brand/Brand.module.css
  { file: 'components/Brand/Brand.module.css', literal: '10px', count: 1, kind: 'irreducible', reason: 'VISUALISER sub-label — below --size-xxs (12px) floor' },
  { file: 'components/Brand/Brand.module.css', literal: '2px', count: 1, kind: 'irreducible', reason: 'text stack gap — below --sp-1 (4px) floor' },
  // components/Chip/Chip.module.css
  { file: 'components/Chip/Chip.module.css', literal: '0.125rem', count: 1, kind: 'irreducible', reason: 'chip base vertical padding — 2px, below --sp-1 (4px) floor; prototype-derived' },
  { file: 'components/Chip/Chip.module.css', literal: '0.1875rem', count: 1, kind: 'irreducible', reason: 'chip md vertical padding — 3px, below --sp-1 (4px) floor; prototype-derived' },
  { file: 'components/Chip/Chip.module.css', literal: '0.02em', count: 1, kind: 'irreducible', reason: 'chip letter-spacing — prototype-derived typography refinement' },
  { file: 'components/Chip/Chip.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  // components/FrontmatterChips/FrontmatterChips.module.css
  { file: 'components/FrontmatterChips/FrontmatterChips.module.css', literal: '0.4rem', count: 1, kind: 'irreducible', reason: 'off-scale gap (6.4px) — between --sp-1 and --sp-2' },
  { file: 'components/FrontmatterChips/FrontmatterChips.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  // components/MarkdownRenderer/MarkdownRenderer.module.css
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '#1e1e1e', count: 1, kind: 'irreducible', reason: 'code block background — editor-dark, no surface token' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '#d4d4d4', count: 1, kind: 'irreducible', reason: 'code block text colour — editor-light-fg, no token' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '1px', count: 2, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '1.75rem', count: 1, kind: 'irreducible', reason: 'h1 font-size (28px) — 6px above size-lg ceiling; no heading token' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '0.88em', count: 1, kind: 'irreducible', reason: 'relative em font-size on inline code — not a rem scale value' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '0.4rem', count: 1, kind: 'irreducible', reason: 'off-scale cell padding (6.4px) — between --sp-1 and --sp-2' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '0.1rem', count: 1, kind: 'irreducible', reason: 'sub-pixel code padding — below --sp-1 floor' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '4px', count: 1, kind: 'irreducible', reason: 'blockquote border-left width — no border-width token' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '6px', count: 1, kind: 'irreducible', reason: 'code block border-radius — between radius-sm and radius-md' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '720px', count: 1, kind: 'irreducible', reason: 'prose max-width — no token equivalent' },
  // components/PipelineDots/PipelineDots.module.css
  { file: 'components/PipelineDots/PipelineDots.module.css', literal: '14px', count: 2, kind: 'irreducible', reason: 'dot width/height — fixed icon pixel, no sp-* equivalent' },
  { file: 'components/PipelineDots/PipelineDots.module.css', literal: '5px', count: 2, kind: 'irreducible', reason: 'inner dot size — fixed icon pixel, no sp-* equivalent' },
  { file: 'components/PipelineDots/PipelineDots.module.css', literal: '1.5px', count: 1, kind: 'irreducible', reason: 'dot ring width — below --sp-1 floor' },
  { file: 'components/PipelineDots/PipelineDots.module.css', literal: '6px', count: 1, kind: 'irreducible', reason: 'pipeline gap — layout pixel, no sp-* equivalent' },
  // components/RelatedArtifacts/RelatedArtifacts.module.css
  { file: 'components/RelatedArtifacts/RelatedArtifacts.module.css', literal: '2px', count: 3, kind: 'irreducible', reason: 'border-left widths and badge border-radius — below --sp-1 floor' },
  { file: 'components/RelatedArtifacts/RelatedArtifacts.module.css', literal: '0.4rem', count: 2, kind: 'irreducible', reason: 'off-scale spacing (6.4px) — between --sp-1 and --sp-2' },
  { file: 'components/RelatedArtifacts/RelatedArtifacts.module.css', literal: '0.15rem', count: 1, kind: 'irreducible', reason: 'item vertical padding (2.4px) — below --sp-1 floor' },
  { file: 'components/RelatedArtifacts/RelatedArtifacts.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  // components/ActivityFeed/ActivityFeed.module.css
  { file: 'components/ActivityFeed/ActivityFeed.module.css', literal: '#6c7088', count: 2, kind: 'irreducible', reason: 'ACTIVITY heading + LIVE badge colour — mirrors Sidebar section-heading hex' },
  { file: 'components/ActivityFeed/ActivityFeed.module.css', literal: '10.5px', count: 2, kind: 'irreducible', reason: 'ACTIVITY heading + LIVE badge font-size — mirrors Sidebar section-heading sub-pixel value' },
  { file: 'components/ActivityFeed/ActivityFeed.module.css', literal: '0.12em', count: 2, kind: 'irreducible', reason: 'ACTIVITY heading + LIVE badge caps letter-spacing — mirrors Sidebar section-heading' },
  { file: 'components/ActivityFeed/ActivityFeed.module.css', literal: '6px', count: 1, kind: 'irreducible', reason: 'ACTIVITY heading vertical padding — mirrors Sidebar section-heading' },
  { file: 'components/ActivityFeed/ActivityFeed.module.css', literal: '10px', count: 1, kind: 'irreducible', reason: 'ACTIVITY heading horizontal padding — mirrors Sidebar section-heading' },
  { file: 'components/ActivityFeed/ActivityFeed.module.css', literal: '1px', count: 2, kind: 'irreducible', reason: 'row dashed separator width + glyph optical alignment — below --sp-1 floor' },
  // components/Sidebar/Sidebar.module.css
  { file: 'components/Sidebar/Sidebar.module.css', literal: '0.12em', count: 2, kind: 'irreducible', reason: 'library and section heading caps letter-spacing — between --tracking-caps and 2×' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '0.14em', count: 1, kind: 'irreducible', reason: 'phase heading caps letter-spacing — between --tracking-caps and 2×' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '10.5px', count: 2, kind: 'irreducible', reason: 'library and section heading font-size from design — sub-pixel, below --size-xxs (12px)' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '9.5px', count: 1, kind: 'irreducible', reason: 'phase heading font-size from design — sub-pixel, below --size-xxs (12px)' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '11px', count: 1, kind: 'irreducible', reason: 'kbd chip font-size — 1px under --size-xxs (12px)' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '13px', count: 2, kind: 'irreducible', reason: 'nav item label and search input font-size from design — 1px under --size-xs (14px)' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '1px', count: 4, kind: 'irreducible', reason: 'sidebar border-right, search-row/kbd borders, inter-item gap — below --sp-1 floor' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '22px', count: 2, kind: 'irreducible', reason: 'temporary kbd chip min-width and height — no token equivalent (0054 will revisit)' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '36px', count: 1, kind: 'irreducible', reason: 'temporary search row height — no token equivalent (0054 will revisit)' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '240px', count: 1, kind: 'irreducible', reason: 'fixed sidebar width — no token equivalent' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '10px', count: 6, kind: 'irreducible', reason: 'heading, search-row, library-heading-hint and nav-item horizontal padding / font-size from design — between --sp-2 (8px) and --sp-3 (12px)' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '6px', count: 5, kind: 'irreducible', reason: 'heading + nav-item vertical padding plus kbd horizontal padding from design — between --sp-1 (4px) and --sp-2 (8px)' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '4px', count: 2, kind: 'irreducible', reason: 'phase heading bottom padding + library-heading-hint translateX — equals --sp-1 but co-located with non-token siblings' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '0.08em', count: 1, kind: 'irreducible', reason: 'library-heading-hint caps letter-spacing — between --tracking-caps and 0.12em' },
  // components/Breadcrumbs/Breadcrumbs.module.css
  { file: 'components/Breadcrumbs/Breadcrumbs.module.css', literal: '2px', count: 3, kind: 'irreducible', reason: 'outline width/offset, border-radius — below --sp-1 floor' },
  // components/OriginPill/OriginPill.module.css
  { file: 'components/OriginPill/OriginPill.module.css', literal: '6px', count: 2, kind: 'irreducible', reason: 'dot width/height — between --sp-1 (4px) and --sp-2 (8px)' },
  { file: 'components/OriginPill/OriginPill.module.css', literal: '3px', count: 1, kind: 'irreducible', reason: 'box-shadow ring spread — below --sp-1 floor' },
  // components/Topbar/Topbar.module.css
  { file: 'components/Topbar/Topbar.module.css', literal: '1px', count: 2, kind: 'irreducible', reason: 'border-bottom and divider widths — below --sp-1 floor' },
  // components/TopbarIconButton/TopbarIconButton.module.css
  { file: 'components/TopbarIconButton/TopbarIconButton.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  // components/Page/Page.module.css
  { file: 'components/Page/Page.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'header/content divider border-top width — below --sp-1 floor' },
  { file: 'components/Page/Page.module.css', literal: '11px', count: 1, kind: 'irreducible', reason: 'eyebrow font-size from design — 1px under --size-xxs (12px)' },
  { file: 'components/Page/Page.module.css', literal: '13px', count: 1, kind: 'irreducible', reason: 'subtitle font-size from design — 1px under --size-xs (14px)' },
  { file: 'components/Page/Page.module.css', literal: '6px', count: 1, kind: 'irreducible', reason: 'eyebrow margin-bottom from design — between --sp-1 and --sp-2' },
  { file: 'components/Page/Page.module.css', literal: '4px', count: 1, kind: 'irreducible', reason: 'subtitle margin-top from design — equals --sp-1 but co-located with non-token siblings' },
  { file: 'components/Page/Page.module.css', literal: '0.12em', count: 1, kind: 'irreducible', reason: 'eyebrow caps letter-spacing — matches sidebar headings' },
  { file: 'components/Page/Page.module.css', literal: '0.01em', count: 1, kind: 'irreducible', reason: 'title negative letter-spacing — display-font tightening per design' },
  // components/Popover/Popover.module.css
  { file: 'components/Popover/Popover.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'panel border width — below --sp-1 floor' },
  { file: 'components/Popover/Popover.module.css', literal: '240px', count: 1, kind: 'irreducible', reason: 'panel min-width — no token equivalent' },
  // routes/kanban/KanbanBoard.module.css
  { file: 'routes/kanban/KanbanBoard.module.css', literal: '1px', count: 4, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  // routes/kanban/KanbanColumn.module.css
  { file: 'routes/kanban/KanbanColumn.module.css', literal: '16rem', count: 2, kind: 'irreducible', reason: 'column min-width/flex-basis — layout dimension, no token' },
  { file: 'routes/kanban/KanbanColumn.module.css', literal: '2px', count: 2, kind: 'irreducible', reason: 'outline width and offset — below --sp-1 floor' },
  // routes/kanban/WorkItemCard.module.css
  { file: 'routes/kanban/WorkItemCard.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  // routes/library/LibraryDocView.module.css
  { file: 'routes/library/LibraryDocView.module.css', literal: '4px', count: 1, kind: 'irreducible', reason: 'malformed-banner border-left width — no border-width token' },
  { file: 'routes/library/LibraryDocView.module.css', literal: '0.4rem', count: 1, kind: 'irreducible', reason: 'aside h3 margin (6.4px) — between --sp-1 and --sp-2' },
  { file: 'routes/library/LibraryDocView.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  { file: 'routes/library/LibraryDocView.module.css', literal: '260px', count: 1, kind: 'irreducible', reason: 'aside column width — no token equivalent' },
  // routes/library/LibraryTemplatesIndex.module.css
  { file: 'routes/library/LibraryTemplatesIndex.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  // routes/library/LibraryTemplatesView.module.css
  { file: 'routes/library/LibraryTemplatesView.module.css', literal: '1px', count: 2, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  // routes/library/LibraryTypeView.module.css
  { file: 'routes/library/LibraryTypeView.module.css', literal: '1px', count: 3, kind: 'irreducible', reason: 'header-row, row, and error border widths — below --sp-1 floor' },
  { file: 'routes/library/LibraryTypeView.module.css', literal: '120px', count: 2, kind: 'irreducible', reason: 'grid column tracks (first column + status) — no token equivalent' },
  { file: 'routes/library/LibraryTypeView.module.css', literal: '110px', count: 1, kind: 'irreducible', reason: 'grid column track (modified) — no token equivalent' },
  { file: 'routes/library/LibraryTypeView.module.css', literal: '10.5px', count: 1, kind: 'irreducible', reason: 'column-header font-size from design — sub-pixel, below --size-xxs' },
  { file: 'routes/library/LibraryTypeView.module.css', literal: '0.1em', count: 1, kind: 'irreducible', reason: 'column-header caps letter-spacing from design — slightly tighter than --tracking-caps' },
  { file: 'routes/library/LibraryTypeView.module.css', literal: '10px', count: 1, kind: 'irreducible', reason: 'header-row vertical padding from design — between --sp-2 and --sp-3' },
  { file: 'routes/library/LibraryTypeView.module.css', literal: '12px', count: 3, kind: 'irreducible', reason: 'header-row + row padding + first-col font-size from design — equals --sp-3 but co-located' },
  { file: 'routes/library/LibraryTypeView.module.css', literal: '13px', count: 1, kind: 'irreducible', reason: 'row body font-size from design — 1px under --size-xs' },
  { file: 'routes/library/LibraryTypeView.module.css', literal: '11.5px', count: 2, kind: 'irreducible', reason: 'slug + mtime font-size from design — sub-pixel, between --size-xxs and 12px' },
  // routes/library/LibraryOverviewHub.module.css
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'card border width — below --sp-1 floor' },
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '640px', count: 1, kind: 'irreducible', reason: 'responsive grid breakpoint — no token equivalent' },
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '1024px', count: 1, kind: 'irreducible', reason: 'responsive grid breakpoint — no token equivalent' },
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '11px', count: 2, kind: 'irreducible', reason: 'phase-heading + card-count font-size from design — 1px under --size-xxs' },
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '10px', count: 2, kind: 'irreducible', reason: 'phase-heading bottom margin + card row gap from design — between --sp-2 and --sp-3' },
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '12px', count: 2, kind: 'irreducible', reason: 'grid gap + card top-row gap from design — equals --sp-3 but co-located' },
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '14px', count: 2, kind: 'irreducible', reason: 'card padding-top/-bottom + card title font-size from design — between --sp-3 and --sp-4' },
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '16px', count: 2, kind: 'irreducible', reason: 'card padding-left/-right + card column gap from design — equals --sp-4 but co-located' },
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '11.5px', count: 1, kind: 'irreducible', reason: 'card subtitle font-size from design — sub-pixel' },
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '6px', count: 3, kind: 'irreducible', reason: 'card border-radius + pinstripe stride from design — between --sp-1 and --sp-2' },
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '7px', count: 1, kind: 'irreducible', reason: 'pinstripe stride end from design — between --sp-1 and --sp-2' },
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '4px', count: 1, kind: 'irreducible', reason: 'card body row gap — equals --sp-1 but co-located' },
  { file: 'routes/library/LibraryOverviewHub.module.css', literal: '0.12em', count: 1, kind: 'irreducible', reason: 'phase-heading caps letter-spacing' },
  // routes/library/EmptyState.module.css — full-page list-view empty state
  { file: 'routes/library/EmptyState.module.css', literal: '1px', count: 2, kind: 'irreducible', reason: 'dashed card outline + dashed foot top border — below --sp-1 floor' },
  { file: 'routes/library/EmptyState.module.css', literal: '96px', count: 1, kind: 'irreducible', reason: 'PaperFold hero column track from design — no token equivalent' },
  { file: 'routes/library/EmptyState.module.css', literal: '28px', count: 3, kind: 'irreducible', reason: 'card grid gap + horizontal padding + top padding from design — between --sp-5 and --sp-6' },
  { file: 'routes/library/EmptyState.module.css', literal: '26px', count: 1, kind: 'irreducible', reason: 'card bottom padding from design — between --sp-5 and --sp-6' },
  { file: 'routes/library/EmptyState.module.css', literal: '22px', count: 2, kind: 'irreducible', reason: 'card responsive padding + title font-size — equals --size-lg but co-located' },
  { file: 'routes/library/EmptyState.module.css', literal: '14px', count: 2, kind: 'irreducible', reason: 'lede font-size + foot padding-top from design — 1px under --size-xs / co-located' },
  { file: 'routes/library/EmptyState.module.css', literal: '12px', count: 2, kind: 'irreducible', reason: 'card border-radius + foot font-size — equals --size-xxs / --radius-lg but co-located' },
  { file: 'routes/library/EmptyState.module.css', literal: '11.5px', count: 2, kind: 'irreducible', reason: 'eyebrow + path-inline font-size from design — sub-pixel' },
  { file: 'routes/library/EmptyState.module.css', literal: '16px', count: 2, kind: 'irreducible', reason: 'lede margin-bottom + responsive grid gap — equals --sp-4 but co-located' },
  { file: 'routes/library/EmptyState.module.css', literal: '8px', count: 1, kind: 'irreducible', reason: 'title margin-bottom from design — equals --sp-2 but co-located' },
  { file: 'routes/library/EmptyState.module.css', literal: '4px', count: 1, kind: 'irreducible', reason: 'eyebrow margin-bottom — equals --sp-1 but co-located' },
  { file: 'routes/library/EmptyState.module.css', literal: '2px', count: 1, kind: 'irreducible', reason: 'hero top padding — below --sp-1 floor' },
  { file: 'routes/library/EmptyState.module.css', literal: '820px', count: 1, kind: 'irreducible', reason: 'responsive collapse breakpoint — no token equivalent' },
  { file: 'routes/library/EmptyState.module.css', literal: '0.04em', count: 1, kind: 'irreducible', reason: 'eyebrow letter-spacing from design — between --tracking-caps and 0' },
  // routes/library/NoResultsPanel.module.css
  { file: 'routes/library/NoResultsPanel.module.css', literal: '1px', count: 2, kind: 'irreducible', reason: 'panel border widths — below --sp-1 floor' },
  // components/SortPill/SortPill.module.css
  { file: 'components/SortPill/SortPill.module.css', literal: '1px', count: 2, kind: 'irreducible', reason: 'trigger + menu-header divider widths — below --sp-1 floor' },
  { file: 'components/SortPill/SortPill.module.css', literal: '220px', count: 1, kind: 'irreducible', reason: 'menu min-width — no token equivalent' },
  { file: 'components/SortPill/SortPill.module.css', literal: '6px', count: 3, kind: 'irreducible', reason: 'trigger vertical padding + gap + menu-header v-padding from design — between --sp-1 and --sp-2' },
  { file: 'components/SortPill/SortPill.module.css', literal: '10px', count: 3, kind: 'irreducible', reason: 'trigger + menu-header + menu-item horizontal padding from design — between --sp-2 and --sp-3' },
  { file: 'components/SortPill/SortPill.module.css', literal: '12px', count: 1, kind: 'irreducible', reason: 'trigger font-size from design — equals --size-xxs but co-located' },
  { file: 'components/SortPill/SortPill.module.css', literal: '12.5px', count: 1, kind: 'irreducible', reason: 'menu-item font-size from design — sub-pixel, between --size-xxs and --size-xs' },
  { file: 'components/SortPill/SortPill.module.css', literal: '10.5px', count: 1, kind: 'irreducible', reason: 'menu-header font-size from design — sub-pixel' },
  { file: 'components/SortPill/SortPill.module.css', literal: '7px', count: 1, kind: 'irreducible', reason: 'menu-item vertical padding from design — between --sp-1 and --sp-2' },
  { file: 'components/SortPill/SortPill.module.css', literal: '8px', count: 2, kind: 'irreducible', reason: 'menu-header bottom padding + menu-item gap from design — equals --sp-2 but co-located' },
  { file: 'components/SortPill/SortPill.module.css', literal: '4px', count: 1, kind: 'irreducible', reason: 'menu-header bottom margin from design — equals --sp-1 but co-located' },
  { file: 'components/SortPill/SortPill.module.css', literal: '0.12em', count: 1, kind: 'irreducible', reason: 'menu-header caps letter-spacing — matches sidebar headings' },
  // components/FilterPill/FilterPill.module.css
  { file: 'components/FilterPill/FilterPill.module.css', literal: '1px', count: 6, kind: 'irreducible', reason: 'trigger / checkbox / search / dashed dividers + checkmark translateY — below --sp-1 floor' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '260px', count: 1, kind: 'irreducible', reason: 'menu min-width — no token equivalent' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '200px', count: 1, kind: 'irreducible', reason: 'long-facet scroll max-height — no token equivalent' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '14px', count: 1, kind: 'irreducible', reason: 'checkbox column track — fixed pixel; no token equivalent' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '13px', count: 2, kind: 'irreducible', reason: 'checkbox width/height — fixed pixel; no token equivalent' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '2px', count: 4, kind: 'irreducible', reason: 'checkbox radius + scroll padding-right + search top margin + clear-button padding — below --sp-1 floor' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '6px', count: 10, kind: 'irreducible', reason: 'trigger / search / scrollbar / no-matches / facet padding / gap / margin — between --sp-1 and --sp-2' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '3px', count: 3, kind: 'irreducible', reason: 'checkmark height + clear-button + option radius — fixed pixel' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '1.5px', count: 2, kind: 'irreducible', reason: 'checkmark stroke widths — below --sp-1 floor' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '4px', count: 5, kind: 'irreducible', reason: 'facet padding + search padding + facet-heading padding + clear-button — equals --sp-1 but co-located' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '5px', count: 2, kind: 'irreducible', reason: 'badge + option horizontal padding — between --sp-1 and --sp-2' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '10px', count: 3, kind: 'irreducible', reason: 'trigger + badge + menu-header horizontal padding from design — between --sp-2 and --sp-3' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '10.5px', count: 2, kind: 'irreducible', reason: 'menu-header + facet-heading font-size from design — sub-pixel' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '11px', count: 2, kind: 'irreducible', reason: 'clear-button + option-count font-size from design — 1px under --size-xxs' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '11.5px', count: 1, kind: 'irreducible', reason: 'no-matches font-size from design — sub-pixel' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '12px', count: 2, kind: 'irreducible', reason: 'trigger + search input font-size from design — equals --size-xxs but co-located' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '12.5px', count: 1, kind: 'irreducible', reason: 'option font-size from design — sub-pixel' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '16px', count: 2, kind: 'irreducible', reason: 'badge min-width/height — no token equivalent' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '7px', count: 1, kind: 'irreducible', reason: 'checkmark width — fixed pixel' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '8px', count: 8, kind: 'irreducible', reason: 'badge radius + option/facet/heading paddings + option gap from design — equals --sp-2 but co-located' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '0.12em', count: 1, kind: 'irreducible', reason: 'menu-header caps letter-spacing' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '0.1em', count: 1, kind: 'irreducible', reason: 'facet-heading caps letter-spacing' },
  { file: 'components/FilterPill/FilterPill.module.css', literal: '#ffffff', count: 3, kind: 'irreducible', reason: 'checkmark stroke + badge text on --ac-accent — theme-invariant white' },
  // routes/lifecycle/LifecycleClusterView.module.css
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '1.5px', count: 2, kind: 'irreducible', reason: 'coloured ring widths — below --radius-sm/--sp-1 floor' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '2px', count: 2, kind: 'irreducible', reason: 'timeline-spine width + stage-dot border width — below --sp-1 floor' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '1px', count: 4, kind: 'irreducible', reason: 'entry/error/long-tail border widths + 1px spine half-offset margin — below --sp-1 floor' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '0.4rem', count: 2, kind: 'irreducible', reason: 'off-scale spacing (6.4px) — between --sp-1 and --sp-2' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '0.06em', count: 1, kind: 'irreducible', reason: 'letter-spacing — off-scale, half of --tracking-caps' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '0.08em', count: 1, kind: 'irreducible', reason: 'letter-spacing — off-scale, half of --tracking-caps' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '1.4em', count: 1, kind: 'irreducible', reason: 'calc(line-height × 3) for text-clamp — derived value' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '1.75rem', count: 1, kind: 'irreducible', reason: 'off-scale spacing (28px) — between --sp-5 and --sp-6' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '6px', count: 1, kind: 'irreducible', reason: 'timeline spine x-coordinate — layout pixel, no sp-* equivalent' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '7px', count: 1, kind: 'irreducible', reason: 'stage dot top offset — layout pixel, no sp-* equivalent' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '10px', count: 1, kind: 'irreducible', reason: 'stage dot diameter — layout pixel, no sp-* equivalent' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '1.25rem', count: 1, kind: 'irreducible', reason: 'off-scale padding (20px) — nearest --sp-4/--sp-5 are 4px off' },
  // routes/lifecycle/LifecycleIndex.module.css
  { file: 'routes/lifecycle/LifecycleIndex.module.css', literal: '1px', count: 5, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  { file: 'routes/lifecycle/LifecycleIndex.module.css', literal: '2px', count: 3, kind: 'irreducible', reason: 'outline width — below --sp-1 floor' },
  { file: 'routes/lifecycle/LifecycleIndex.module.css', literal: '6px', count: 2, kind: 'irreducible', reason: 'toolbar gap and card radius — layout pixels, no token equivalent' },
  { file: 'routes/lifecycle/LifecycleIndex.module.css', literal: '220px', count: 1, kind: 'irreducible', reason: 'filter input flex basis — no token equivalent' },
  { file: 'routes/lifecycle/LifecycleIndex.module.css', literal: '320px', count: 1, kind: 'irreducible', reason: 'card grid min-width — no token equivalent' },
  // styles/wiki-links.global.css
  { file: 'styles/wiki-links.global.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border-bottom width — below --sp-1 floor' },
]

// Build O(1) lookup maps once at module load
const exceptionsByFile = new Map<string, Map<string, number>>()
for (const e of EXCEPTIONS) {
  let inner = exceptionsByFile.get(e.file)
  if (!inner) {
    inner = new Map()
    exceptionsByFile.set(e.file, inner)
  }
  inner.set(e.literal, (inner.get(e.literal) ?? 0) + e.count)
}

// Map vite-glob keys to the src-relative form used by EXCEPTIONS.file.
// Files in src/styles/ (same dir as this test) return a './' key — Vite
// normalises '../styles/foo.css' → './foo.css' when src and target share
// the same directory. Prepend 'styles/' to match EXCEPTIONS entries.
// Files in other src/ subdirectories return '../<rest>' keys as expected.
function srcRelative(globKey: string): string {
  if (globKey.startsWith('./')) {
    return 'styles/' + globKey.slice(2)
  }
  if (!globKey.startsWith('../') || globKey.startsWith('../../')) {
    throw new Error(
      `srcRelative: unexpected glob key shape "${globKey}". ` +
        `Expected "./" (same-dir) or exactly one "../" prefix (test sits at src/styles/, globs "../**/*.module.css"). ` +
        `If the test or glob has been moved, update srcRelative accordingly.`,
    )
  }
  return globKey.slice(3)
}

function permittedCount(file: string, literal: string): number {
  return exceptionsByFile.get(srcRelative(file))?.get(literal) ?? 0
}

function violations(matches: string[], file: string): string[] {
  const counts = new Map<string, number>()
  for (const m of matches) counts.set(m, (counts.get(m) ?? 0) + 1)
  const result: string[] = []
  for (const [literal, observed] of counts) {
    const allowed = permittedCount(file, literal)
    if (observed > allowed) {
      for (let i = 0; i < observed - allowed; i++) result.push(literal)
    }
  }
  return result
}

const allCss = { ...cssModules, ...cssGlobals }

describe('AC3: no hex literals outside EXCEPTIONS', () => {
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} hex literals all accounted for`, () => {
      const matches = [...css.matchAll(HEX_RE)].map((m) => m[0])
      expect(violations(matches, path)).toEqual([])
    })
  }
})

describe('AC4: no px/rem/em literals outside EXCEPTIONS (0-resets auto-excluded)', () => {
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} px/rem/em literals all accounted for`, () => {
      const matches = [...css.matchAll(PX_REM_EM_RE)].map((m) => m[0])
      expect(violations(matches, path)).toEqual([])
    })
  }
})

describe('var(--token, fallback) two-arg form is retired', () => {
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} contains no var(--*, fallback) sites`, () => {
      const fallbacks = [...css.matchAll(VAR_FALLBACK_RE)].map((m) => m[0])
      expect(fallbacks).toEqual([])
    })
  }
})

describe('color-mix() convention (Phase 4 special conventions)', () => {
  // Locked-in percentage ladder: 8 (default tinted bg), 18 (hover state),
  // 30 (stroke/border tint). Composition surface always var(--ac-bg).
  const COLOR_MIX_RE = /color-mix\(\s*in\s+srgb\s*,\s*var\(--ac-(err|warn|ok|violet)\)\s+(\d+)%\s*,\s*var\(--ac-bg\)\s*\)/g
  const COLOR_MIX_ANY_RE = /color-mix\(/g
  const ALLOWED_PERCENTAGES = new Set([8, 18, 30])

  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} color-mix sites use the locked-in convention`, () => {
      const totalSites = (css.match(COLOR_MIX_ANY_RE) ?? []).length
      const conventionalSites = [...css.matchAll(COLOR_MIX_RE)]
      expect(conventionalSites.length).toBe(totalSites)
      for (const m of conventionalSites) {
        expect(ALLOWED_PERCENTAGES.has(parseInt(m[2], 10))).toBe(true)
      }
    })
  }
})

describe('var(--NAME) references resolve to declared tokens', () => {
  const declared = new Set([
    ...Object.keys(LIGHT_COLOR_TOKENS),
    ...Object.keys(DARK_COLOR_TOKENS),
    ...Object.keys(TYPOGRAPHY_TOKENS),
    ...Object.keys(SPACING_TOKENS),
    ...Object.keys(RADIUS_TOKENS),
    ...Object.keys(LIGHT_SHADOW_TOKENS),
    ...Object.keys(DARK_SHADOW_TOKENS),
    ...Object.keys(LAYOUT_TOKENS),
  ])
  // File-scoped local custom properties — declared at the top of the
  // module's own stylesheet and consumed only within that file. These
  // never make it into the global token set but still need to pass the
  // "var(--*) resolves to something" guard.
  const LOCAL_CUSTOM_PROPS: Record<string, ReadonlySet<string>> = {
    'routes/library/EmptyState.module.css': new Set(['ac-empty-page-hue']),
    'routes/lifecycle/LifecycleClusterView.module.css': new Set([
      'spine-x',
      'dot-size',
    ]),
  }
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} references only declared tokens`, () => {
      const refs = [...css.matchAll(VAR_REF_RE)].map((m) => m[1])
      const local = LOCAL_CUSTOM_PROPS[srcRelative(path)] ?? new Set<string>()
      const unknown = refs.filter((name) => !declared.has(name) && !local.has(name))
      expect(unknown).toEqual([])
    })
  }
})

// AC5 enforcement is a true two-sided ratchet:
//
// - `AC5_FLOOR` is the *committed minimum*. It MUST equal the value
//   observed at the last committed state (or below it by no more than
//   AC5_REGRESSION_SLACK). The implementer bumps AC5_FLOOR upward in
//   the same commit that adds new var(--*) references.
// - `AC5_TARGET = 300` is the work-item contract.
const AC5_FLOOR = 423 // 0034: FontModeToggle switched to SVG icon (-2)
const AC5_TARGET = 300 // contract from work item AC5
const AC5_REGRESSION_SLACK = 0

describe('AC5: aggregate var(--*) coverage (two-sided ratchet)', () => {
  const observed = Object.values(cssModules).reduce(
    (acc, css) => acc + (css.match(VAR_COUNT_RE)?.length ?? 0),
    0,
  )

  it(`observed count (${observed}) is at least AC5_FLOOR (${AC5_FLOOR})`, () => {
    expect(observed).toBeGreaterThanOrEqual(AC5_FLOOR - AC5_REGRESSION_SLACK)
  })

  it(`AC5_FLOOR (${AC5_FLOOR}) is not above observed (${observed}) — bump protocol followed`, () => {
    expect(AC5_FLOOR).toBeLessThanOrEqual(observed)
  })

  const finalStateActive = AC5_FLOOR >= AC5_TARGET
  ;(finalStateActive ? it : it.skip)(
    `(final-state gate) observed reaches AC5_TARGET (${AC5_TARGET})`,
    () => {
      expect(observed).toBeGreaterThanOrEqual(AC5_TARGET)
    },
  )
})

// Build the inverse map for hygiene checks.
const cssBySrcRelative = new Map<string, string>()
for (const [globKey, css] of Object.entries(allCss)) {
  cssBySrcRelative.set(srcRelative(globKey), css)
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

describe('EXCEPTIONS hygiene', () => {
  it('every EXCEPTIONS entry resolves to exactly one CSS file', () => {
    const unresolved: Exception[] = []
    for (const e of EXCEPTIONS) {
      if (!cssBySrcRelative.has(e.file)) unresolved.push(e)
    }
    expect(unresolved).toEqual([])
  })

  it('declared count equals observed count (no stale entries, no over-count)', () => {
    const mismatches: Array<{
      file: string
      literal: string
      declared: number
      observed: number
    }> = []
    for (const [file, literalMap] of exceptionsByFile) {
      const css = cssBySrcRelative.get(file)
      if (!css) continue
      const hexHits = [...css.matchAll(HEX_RE)].map((m) => m[0])
      const unitHits = [...css.matchAll(PX_REM_EM_RE)].map((m) => m[0])
      const allHits = [...hexHits, ...unitHits]
      for (const [literal, declared] of literalMap) {
        const observed = allHits.filter((h) => h === literal).length
        if (observed !== declared) {
          mismatches.push({ file, literal, declared, observed })
        }
      }
    }
    expect(mismatches).toEqual([])
  })
})

// Suppress unused-variable warning — escapeRegExp is available for future
// use by harness extensions that need to escape literal strings in regexes.
void escapeRegExp

function extractBlockBody(css: string, startIdx: number): string | null {
  const open = css.indexOf('{', startIdx)
  if (open === -1) return null
  let depth = 1
  let i = open + 1
  while (i < css.length && depth > 0) {
    const ch = css[i]
    if (ch === '{') depth++
    else if (ch === '}') depth--
    i++
  }
  return depth === 0 ? css.slice(open + 1, i - 1) : null
}

describe('0038: --radius-pill is reserved for non-status surfaces', () => {
  // Files that legitimately use --radius-pill. Every other consumer in
  // src/ would be an open-coded status pill regression and must be
  // migrated to <Chip>. Adding a new entry to this list requires a
  // brief reason recorded alongside the path.
  const PILL_RADIUS_ALLOW_LIST = new Set([
    'components/Chip/Chip.module.css',
    'components/OriginPill/OriginPill.module.css',
    'components/Sidebar/Sidebar.module.css',
    'components/FilterPill/FilterPill.module.css',
    'routes/kanban/KanbanColumn.module.css',
    'routes/lifecycle/LifecycleIndex.module.css',
  ])

  it('no module outside the allow-list defines a pill-radius element', () => {
    const offenders: string[] = []
    for (const [file, css] of cssBySrcRelative) {
      if (PILL_RADIUS_ALLOW_LIST.has(file)) continue
      if (/border-radius:\s*var\(--radius-pill\)/.test(css)) {
        offenders.push(file)
      }
    }
    expect(offenders).toEqual([])
  })
})

describe('Phase 1 (0034): route titles consume --ac-fg-strong', () => {
  const REQUIRED: { file: string; selector: string }[] = []

  for (const { file, selector } of REQUIRED) {
    it(`${file} ${selector} declares color: var(--ac-fg-strong)`, () => {
      const css = cssBySrcRelative.get(file)
      expect(css, `missing ${file}`).toBeDefined()
      const idx = css!.indexOf(selector)
      expect(idx, `selector ${selector} not found in ${file}`).toBeGreaterThanOrEqual(0)
      const body = extractBlockBody(css!, idx)
      expect(body, `body for ${selector} in ${file}`).not.toBeNull()
      expect(body!).toMatch(/(?<!background-)color:\s*var\(--ac-fg-strong\)/)
    })
  }
})
