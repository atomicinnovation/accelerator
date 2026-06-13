import { describe, expect, it } from "vitest";
import {
  CODE_SURFACE_TOKENS,
  CODE_SYNTAX_TOKENS,
  DARK_COLOR_TOKENS,
  DARK_SHADOW_TOKENS,
  LAYOUT_TOKENS,
  LIGHT_COLOR_TOKENS,
  LIGHT_SHADOW_TOKENS,
  RADIUS_TOKENS,
  SPACING_TOKENS,
  TYPOGRAPHY_TOKENS,
} from "./tokens";

const cssModules = import.meta.glob("../**/*.module.css", {
  eager: true,
  query: "?raw",
  import: "default",
}) as Record<string, string>;

const cssGlobals = import.meta.glob("../**/*.global.css", {
  eager: true,
  query: "?raw",
  import: "default",
}) as Record<string, string>;

// All `0` resets are auto-permitted (admitted by AC4's escape-hatch);
// the regex excludes them at the source so they never need EXCEPTIONS
// entries — keeps the list focused on genuine exceptions.
const HEX_RE = /#[0-9a-fA-F]{3,8}\b/g;
const PX_REM_EM_RE = /\b(?!0(?:px|rem|em)\b)\d+(?:\.\d+)?(?:px|rem|em)\b/g;
const VAR_REF_RE = /var\(\s*--([\w-]+)\s*[,)]/g;
// Scoped to --ac-* / --sp-* / --radius-* / --size-* / --shadow-* / --lh-*
// tokens — i.e. the new layered set. Legacy `--color-*` fallback sites
// remain present at Phase 2 commit and are deleted by Phase 3; this regex
// deliberately does not flag them so the harness lands green.
const VAR_FALLBACK_RE =
  /var\(\s*--(?:ac-|sp-|radius-|size-|shadow-|lh-|tracking-)[\w-]+\s*,/g;
const VAR_COUNT_RE = /var\(\s*--/g;

// Per-occurrence exception model. `count` is how many times this
// `(file, literal)` pair is allowed to match; when the implementer
// migrates one occurrence, they decrement the count or remove the
// entry. Adding a new exception requires `count: 1` and a reason.
// `file` is the path **relative to `src/`** to disambiguate any future
// modules that share a basename across directories.
type Exception = {
  file: string;
  literal: string;
  count: number;
  reason: string;
};

const EXCEPTIONS: ReadonlyArray<
  Exception & { kind: "to-migrate" | "irreducible" }
> = [
  // components/Brand/Brand.module.css
  {
    file: "components/Brand/Brand.module.css",
    literal: "2px",
    count: 1,
    kind: "irreducible",
    reason: "text stack gap — below --sp-1 (4px) floor",
  },
  // components/DevDesignSystem/DevDesignSystem.module.css — bespoke dev-only
  // reference-page chrome. Spacing/type/radii are tokenised; these are the
  // irreducible off-scale chrome values with no token equivalent.
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "1px",
    count: 44,
    kind: "irreducible",
    reason:
      "hairline borders — chrome (marquee/TOC/section/footer 7) + TOC list hairline gap (1px) + token/type cards + glyph card + glyph-card dashed size divider + mark cards (17, deviations aside removed) + interactive primitives (tier pill, topbar button, search input, search kbd, nav demo = 5) + composites & chrome (13) + the big-glyph cell hover translateY(-1px) lift — below --sp-1 floor, no border-width token",
  },
  // Prototype-fidelity convergence: off-scale dev-page measurements ported
  // verbatim from the prototype `ds-*` chrome (no --sp-* / token equivalent).
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "36px",
    count: 1,
    kind: "irreducible",
    reason: "dev-page content gutter (--ds-gutter) — prototype .ds-root rhythm",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "56px",
    count: 2,
    kind: "irreducible",
    reason:
      "inter-section gap (--ds-section-gap) + colour-swatch chip height — prototype .ds-root / .ds-swatch__chip",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "130px",
    count: 1,
    kind: "irreducible",
    reason: "empty-state-glyph hero height — prototype .ds-bigglyph-cell__hero",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "5px",
    count: 1,
    kind: "irreducible",
    reason:
      "TOC jumplink vertical padding (prototype .ds-toc__item) — off-scale, no token",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "0.06em",
    count: 1,
    kind: "irreducible",
    reason:
      "DEV marquee letter-spacing — prototype .ds-marquee (tighter than --tracking-caps so the ticker reads as one run)",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "3px",
    count: 1,
    kind: "irreducible",
    reason: "toast left accent bar — below --sp-1 floor, no border-width token",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "240px",
    count: 2,
    kind: "irreducible",
    reason:
      "search-input demo max-width + kanban-card demo cell width — no layout-width token",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "420px",
    count: 1,
    kind: "irreducible",
    reason:
      "toast stack max-width (mirrors the live Toaster viewport) — no layout-width token",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "180px",
    count: 3,
    kind: "irreducible",
    reason:
      "swatch + doc-type-hue + big-glyph grid min track widths (minmax floor) — no layout-width token",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "110px",
    count: 1,
    kind: "irreducible",
    reason: "icon grid min track width (minmax floor) — no layout-width token",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "220px",
    count: 2,
    kind: "irreducible",
    reason:
      "sticky TOC column fixed width + doc-type-glyph card grid min track width (minmax floor) — no layout-width token",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "280px",
    count: 1,
    kind: "irreducible",
    reason:
      "sidebar-nav demo max-width (prototype .ds-navdemo, real sidebar width) — no layout-width token",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "1280px",
    count: 1,
    kind: "irreducible",
    reason: "dev-page content max-width — no layout-width token",
  },
  {
    file: "components/DevDesignSystem/DevDesignSystem.module.css",
    literal: "#ffffff",
    count: 2,
    kind: "irreducible",
    reason:
      "DEV marquee tag text on the --ac-accent-2 red badge + filter-badge count text on --ac-accent — theme-invariant white",
  },
  // components/Chip/Chip.module.css
  {
    file: "components/Chip/Chip.module.css",
    literal: "0.125rem",
    count: 1,
    kind: "irreducible",
    reason:
      "chip base vertical padding — 2px, below --sp-1 (4px) floor; prototype-derived",
  },
  {
    file: "components/Chip/Chip.module.css",
    literal: "0.1875rem",
    count: 1,
    kind: "irreducible",
    reason:
      "chip md vertical padding — 3px, below --sp-1 (4px) floor; prototype-derived",
  },
  {
    file: "components/Chip/Chip.module.css",
    literal: "0.02em",
    count: 1,
    kind: "irreducible",
    reason: "chip letter-spacing — prototype-derived typography refinement",
  },
  {
    file: "components/Chip/Chip.module.css",
    literal: "1px",
    count: 1,
    kind: "irreducible",
    reason: "border width — below --sp-1 floor",
  },
  // components/FrontmatterChips/FrontmatterChips.module.css
  {
    file: "components/FrontmatterChips/FrontmatterChips.module.css",
    literal: "0.4rem",
    count: 1,
    kind: "irreducible",
    reason: "off-scale gap (6.4px) — between --sp-1 and --sp-2",
  },
  {
    file: "components/FrontmatterChips/FrontmatterChips.module.css",
    literal: "1px",
    count: 1,
    kind: "irreducible",
    reason: "border width — below --sp-1 floor",
  },
  // components/FrontmatterTable/FrontmatterTable.module.css
  {
    file: "components/FrontmatterTable/FrontmatterTable.module.css",
    literal: "14px",
    count: 1,
    kind: "irreducible",
    reason:
      "container horizontal padding from design — between --sp-3 (12px) and --sp-4 (16px); not aliased to a token",
  },
  {
    file: "components/FrontmatterTable/FrontmatterTable.module.css",
    literal: "6px",
    count: 1,
    kind: "irreducible",
    reason:
      "row-gap from design — between --sp-1 (4px) and --sp-2 (8px); matches prototype .ac-fm gap",
  },
  {
    file: "components/FrontmatterTable/FrontmatterTable.module.css",
    literal: "1px",
    count: 2,
    kind: "irreducible",
    reason:
      "container border + anchor dashed border-bottom — below --sp-1 floor",
  },
  {
    file: "components/FrontmatterTable/FrontmatterTable.module.css",
    literal: "2px",
    count: 2,
    kind: "irreducible",
    reason: "focus-ring outline width + outline-offset — below --sp-1 floor",
  },
  // components/MarkdownRenderer/MarkdownRenderer.module.css
  {
    file: "components/MarkdownRenderer/MarkdownRenderer.module.css",
    literal: "1px",
    count: 7,
    kind: "irreducible",
    reason:
      "hairline borders + sub-px padding: <pre> stroke, h1 underline, table cell, codeblock wrapper, codeblockHead bottom, inline-code pill border, inline-code pill vertical padding — below --sp-1 floor",
  },
  {
    file: "components/MarkdownRenderer/MarkdownRenderer.module.css",
    literal: "0.4rem",
    count: 1,
    kind: "irreducible",
    reason: "off-scale cell padding (6.4px) — between --sp-1 and --sp-2",
  },
  {
    file: "components/MarkdownRenderer/MarkdownRenderer.module.css",
    literal: "5px",
    count: 1,
    kind: "irreducible",
    reason:
      "inline-code pill horizontal padding — off-scale, between --sp-1 (4px) and --sp-2 (8px)",
  },
  {
    file: "components/MarkdownRenderer/MarkdownRenderer.module.css",
    literal: "4px",
    count: 1,
    kind: "irreducible",
    reason: "blockquote border-left width — no border-width token",
  },
  {
    file: "components/MarkdownRenderer/MarkdownRenderer.module.css",
    literal: "1.5px",
    count: 1,
    kind: "irreducible",
    reason: "task box border width — below --sp-1 floor",
  },
  {
    file: "components/MarkdownRenderer/MarkdownRenderer.module.css",
    literal: "2px",
    count: 2,
    kind: "irreducible",
    reason: "task box margin-top + tasklist padding-left — below --sp-1 floor",
  },
  {
    file: "components/MarkdownRenderer/MarkdownRenderer.module.css",
    literal: "6px",
    count: 1,
    kind: "irreducible",
    reason: "task row vertical margin — between --sp-1 and --sp-2",
  },
  {
    file: "components/MarkdownRenderer/MarkdownRenderer.module.css",
    literal: "9px",
    count: 1,
    kind: "irreducible",
    reason: "task box→label gap — between --sp-2 and --sp-3",
  },
  {
    file: "components/MarkdownRenderer/MarkdownRenderer.module.css",
    literal: "17px",
    count: 2,
    kind: "irreducible",
    reason: "task box width + height — fixed component dimension, no token",
  },
  {
    file: "components/MarkdownRenderer/MarkdownRenderer.module.css",
    literal: "#ffffff",
    count: 1,
    kind: "irreducible",
    reason:
      "task tick stroke on --ac-accent — theme-invariant white (mirrors FilterPill checkmark)",
  },
  // routes/lifecycle/LifecycleClusterView.module.css — pipeline panel,
  // timeline spine, stage tile, and tcard literals. Numbers track the
  // prototype's `.ac-tcard` / `.ac-tstep` measurements verbatim so the
  // detail page reads as a port rather than a reinterpretation.
  {
    file: "routes/lifecycle/LifecycleClusterView.module.css",
    literal: "1px",
    count: 4,
    kind: "irreducible",
    reason:
      "pipeline panel + timeline tile + entry card + error-state border-widths — below --sp-1 floor",
  },
  {
    file: "routes/lifecycle/LifecycleClusterView.module.css",
    literal: "2px",
    count: 1,
    kind: "irreducible",
    reason: "timeline spine width — below --sp-1 floor",
  },
  {
    file: "routes/lifecycle/LifecycleClusterView.module.css",
    literal: "6px",
    count: 1,
    kind: "irreducible",
    reason:
      "entry-card head→body row margin-bottom — between --sp-1 (4) and --sp-2 (8)",
  },
  {
    file: "routes/lifecycle/LifecycleClusterView.module.css",
    literal: "10px",
    count: 3,
    kind: "irreducible",
    reason:
      "tcard head gaps + missing-card padding — prototype-spec literal, between --sp-2 (8) and --sp-3 (12)",
  },
  {
    file: "routes/lifecycle/LifecycleClusterView.module.css",
    literal: "14px",
    count: 4,
    kind: "irreducible",
    reason:
      "pipeline eyebrow margin-bottom + tstep padding-top/node-top + tcard padding-y — prototype-spec literal, between --sp-3 (12) and --sp-4 (16)",
  },
  {
    file: "routes/lifecycle/LifecycleClusterView.module.css",
    literal: "16px",
    count: 1,
    kind: "irreducible",
    reason:
      "tcard horizontal padding — prototype-spec literal, equals --sp-4 but kept inline to mirror prototype tcard `14px 16px`",
  },
  {
    file: "routes/lifecycle/LifecycleClusterView.module.css",
    literal: "20px",
    count: 1,
    kind: "irreducible",
    reason:
      "pipeline panel horizontal padding — between --sp-4 (16) and --sp-5 (24)",
  },
  {
    file: "routes/lifecycle/LifecycleClusterView.module.css",
    literal: "22px",
    count: 2,
    kind: "irreducible",
    reason:
      "timeline spine x-coordinate + tstep padding-bottom — layout pixel, no sp-* equivalent",
  },
  {
    file: "routes/lifecycle/LifecycleClusterView.module.css",
    literal: "52px",
    count: 1,
    kind: "irreducible",
    reason:
      "stage tile left offset from card (negative; matches 56px content gutter minus half tile) — layout pixel, no sp-* equivalent",
  },
  {
    file: "routes/lifecycle/LifecycleClusterView.module.css",
    literal: "56px",
    count: 1,
    kind: "irreducible",
    reason:
      "timeline left padding to clear stage tiles — layout pixel, no sp-* equivalent",
  },
  {
    file: "routes/lifecycle/LifecycleClusterView.module.css",
    literal: "0.1em",
    count: 1,
    kind: "irreducible",
    reason: "pipeline eyebrow letter-spacing — sub-pixel rhythm, no token",
  },
  {
    file: "routes/lifecycle/LifecycleClusterView.module.css",
    literal: "0.04em",
    count: 1,
    kind: "irreducible",
    reason: "tcard stage label letter-spacing — sub-pixel rhythm, no token",
  },
  // components/Pipeline/Pipeline.module.css
  {
    file: "components/Pipeline/Pipeline.module.css",
    literal: "1px",
    count: 1,
    kind: "irreducible",
    reason: "tile border-width — below --sp-1 floor",
  },
  {
    file: "components/Pipeline/Pipeline.module.css",
    literal: "2px",
    count: 1,
    kind: "irreducible",
    reason: "connector height — below --sp-1 floor",
  },
  {
    file: "components/Pipeline/Pipeline.module.css",
    literal: "5px",
    count: 1,
    kind: "irreducible",
    reason: "stage column gap (tile→label) — between --sp-1 (4) and --sp-2 (8)",
  },
  {
    file: "components/Pipeline/Pipeline.module.css",
    literal: "13px",
    count: 1,
    kind: "irreducible",
    reason:
      "connector vertical offset (half card-variant tile) — derived from tile size, no token",
  },
  {
    file: "components/Pipeline/Pipeline.module.css",
    literal: "17px",
    count: 1,
    kind: "irreducible",
    reason:
      "connector vertical offset (half panel-variant tile) — derived from tile size, no token",
  },
  {
    file: "components/Pipeline/Pipeline.module.css",
    literal: "26px",
    count: 2,
    kind: "irreducible",
    reason:
      "card-variant tile size — fixed pixel for the chain visual rhythm, no sp-* equivalent",
  },
  {
    file: "components/Pipeline/Pipeline.module.css",
    literal: "34px",
    count: 2,
    kind: "irreducible",
    reason:
      "panel-variant tile size — fixed pixel for the chain visual rhythm, no sp-* equivalent",
  },
  {
    file: "components/Pipeline/Pipeline.module.css",
    literal: "0.04em",
    count: 1,
    kind: "irreducible",
    reason: "letter-spacing — sub-pixel rhythm, no token",
  },
  // components/PipelineMini/PipelineMini.module.css
  {
    file: "components/PipelineMini/PipelineMini.module.css",
    literal: "6px",
    count: 1,
    kind: "irreducible",
    reason: "dot row gap — between --sp-1 and --sp-2",
  },
  {
    file: "components/PipelineMini/PipelineMini.module.css",
    literal: "8px",
    count: 2,
    kind: "irreducible",
    reason:
      "dot diameter — fixed pixel for visual rhythm with kanban card chrome",
  },
  {
    file: "components/PipelineMini/PipelineMini.module.css",
    literal: "1.5px",
    count: 1,
    kind: "irreducible",
    reason: "dot border-width — below --sp-1 floor",
  },
  // components/RelatedArtifacts/RelatedArtifacts.module.css
  // 0079 (Option B): the legend, group border-lefts (2px) and badge border
  // (1px) were removed when the three groups collapsed to a single tagged
  // list, so the former '2px' (×2) and '1px' (×1) exceptions are gone.
  {
    file: "components/RelatedArtifacts/RelatedArtifacts.module.css",
    literal: "0.4rem",
    count: 1,
    kind: "irreducible",
    reason: "updating-hint margin (6.4px) — between --sp-1 and --sp-2",
  },
  {
    file: "components/RelatedArtifacts/RelatedArtifacts.module.css",
    literal: "1px",
    count: 1,
    kind: "irreducible",
    reason: "row hover-card border width — below --sp-1 floor",
  },
  // components/RelatedCluster/RelatedCluster.module.css
  {
    file: "components/RelatedCluster/RelatedCluster.module.css",
    literal: "1px",
    count: 1,
    kind: "irreducible",
    reason: "cluster card border width — below --sp-1 floor",
  },
  // components/ActivityFeed/ActivityFeed.module.css
  {
    file: "components/ActivityFeed/ActivityFeed.module.css",
    literal: "#6c7088",
    count: 2,
    kind: "irreducible",
    reason:
      "ACTIVITY heading + LIVE badge colour — mirrors Sidebar section-heading hex",
  },
  {
    file: "components/ActivityFeed/ActivityFeed.module.css",
    literal: "0.12em",
    count: 2,
    kind: "irreducible",
    reason:
      "ACTIVITY heading + LIVE badge caps letter-spacing — mirrors Sidebar section-heading",
  },
  {
    file: "components/ActivityFeed/ActivityFeed.module.css",
    literal: "6px",
    count: 1,
    kind: "irreducible",
    reason:
      "ACTIVITY heading vertical padding — mirrors Sidebar section-heading",
  },
  {
    file: "components/ActivityFeed/ActivityFeed.module.css",
    literal: "10px",
    count: 1,
    kind: "irreducible",
    reason:
      "ACTIVITY heading horizontal padding — mirrors Sidebar section-heading",
  },
  {
    file: "components/ActivityFeed/ActivityFeed.module.css",
    literal: "1px",
    count: 2,
    kind: "irreducible",
    reason:
      "row dashed separator width + glyph optical alignment — below --sp-1 floor",
  },
  // components/RootLayout/RootLayout.module.css
  {
    file: "components/RootLayout/RootLayout.module.css",
    literal: "12px",
    count: 2,
    kind: "irreducible",
    reason:
      "main-column scrollbar width and height — sized for visibility on touchpads and mice; no token equivalent",
  },
  {
    file: "components/RootLayout/RootLayout.module.css",
    literal: "3px",
    count: 1,
    kind: "irreducible",
    reason:
      "main-column scrollbar thumb inset border (visually narrows the thumb without resizing the gutter) — below --sp-1 floor",
  },
  // components/Sidebar/Sidebar.module.css
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "0.12em",
    count: 2,
    kind: "irreducible",
    reason:
      "library and section heading caps letter-spacing — between --tracking-caps and 2×",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "0.14em",
    count: 1,
    kind: "irreducible",
    reason:
      "phase heading caps letter-spacing — between --tracking-caps and 2×",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "1px",
    count: 10,
    kind: "irreducible",
    reason:
      "sidebar border-right, search-input border, kbd hint border, hint-kbd border, panel border, panel meta-row border-bottom, list-gap, mark padding, row-sub margin-top, animation translateY — below --sp-1 floor",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "2px",
    count: 3,
    kind: "irreducible",
    reason:
      "kbd vertical padding, result list gap, loadbar track height — below --sp-1 floor",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "3px",
    count: 1,
    kind: "irreducible",
    reason: "input focus-ring spread — below --sp-1 floor",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "4px",
    count: 4,
    kind: "irreducible",
    reason:
      "phase heading bottom padding, library-heading-hint translateX, animation translateY negative offset, hint-kbd horizontal padding — equals --sp-1 but co-located with non-token siblings",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "5px",
    count: 1,
    kind: "irreducible",
    reason: "kbd hint horizontal padding — between --sp-1 and --sp-2",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "6px",
    count: 7,
    kind: "irreducible",
    reason:
      "heading + nav-item vertical padding, search-clear right offset, result-row vertical padding, loading row gap, link list — between --sp-1 and --sp-2",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "7px",
    count: 3,
    kind: "irreducible",
    reason:
      "search input vertical padding, meta-row vertical padding — design-pinned half-step between --sp-1 and --sp-2",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "8px",
    count: 3,
    kind: "irreducible",
    reason:
      "kbd hint right offset, scrollbar width, panel margin-top — equals --sp-2 but co-located with non-token siblings",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "10px",
    count: 7,
    kind: "irreducible",
    reason:
      "library heading inline padding, section heading inline padding, phase heading inline padding, nav link inline padding, search icon left offset, input right padding, meta-row inline padding, loading row vertical padding — between --sp-2 (8px) and --sp-3 (12px)",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "12px",
    count: 1,
    kind: "irreducible",
    reason: "search-row chevron column width — fixed pixel column track",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "14px",
    count: 1,
    kind: "irreducible",
    reason:
      "empty-state inline padding from prototype — between --sp-3 and --sp-4",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "16px",
    count: 1,
    kind: "irreducible",
    reason:
      "empty-state bottom padding from prototype — equals --sp-4 but co-located with non-token siblings",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "18px",
    count: 1,
    kind: "irreducible",
    reason:
      "empty-state top padding from prototype — between --sp-4 and --sp-5",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "20px",
    count: 2,
    kind: "irreducible",
    reason:
      "clear-button width and height — no token equivalent at this hit-area size",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "26px",
    count: 1,
    kind: "irreducible",
    reason:
      "result-row Glyph column width — design-pinned tile size between Glyph 24/32",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "28px",
    count: 1,
    kind: "irreducible",
    reason:
      "search input right padding to clear the trailing kbd / clear button — no token equivalent",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "32px",
    count: 1,
    kind: "irreducible",
    reason:
      "search input left padding to clear the leading magnifying-glass icon — no token equivalent",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "256px",
    count: 1,
    kind: "irreducible",
    reason:
      "sidebar base width (256 = prototype 255 + 1px border under border-box) — no token equivalent",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "290px",
    count: 1,
    kind: "irreducible",
    reason: "sidebar widened width above 1400px viewport — no token equivalent",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "1400px",
    count: 1,
    kind: "irreducible",
    reason: "media-query breakpoint for sidebar widen — no token equivalent",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "320px",
    count: 1,
    kind: "irreducible",
    reason: "result list max-height before scroll — no token equivalent",
  },
  {
    file: "components/Sidebar/Sidebar.module.css",
    literal: "0.08em",
    count: 1,
    kind: "irreducible",
    reason:
      "library-heading-hint caps letter-spacing — between --tracking-caps and 0.12em",
  },
  // components/Breadcrumbs/Breadcrumbs.module.css
  {
    file: "components/Breadcrumbs/Breadcrumbs.module.css",
    literal: "2px",
    count: 2,
    kind: "irreducible",
    reason: "focus-ring outline width + outline-offset — below --sp-1 floor",
  },
  // components/OriginPill/OriginPill.module.css
  {
    file: "components/OriginPill/OriginPill.module.css",
    literal: "6px",
    count: 2,
    kind: "irreducible",
    reason: "dot width/height — between --sp-1 (4px) and --sp-2 (8px)",
  },
  {
    file: "components/OriginPill/OriginPill.module.css",
    literal: "3px",
    count: 1,
    kind: "irreducible",
    reason: "box-shadow ring spread — below --sp-1 floor",
  },
  // components/Topbar/Topbar.module.css
  {
    file: "components/Topbar/Topbar.module.css",
    literal: "1px",
    count: 2,
    kind: "irreducible",
    reason: "border-bottom and divider widths — below --sp-1 floor",
  },
  // components/TopbarIconButton/TopbarIconButton.module.css
  {
    file: "components/TopbarIconButton/TopbarIconButton.module.css",
    literal: "1px",
    count: 1,
    kind: "irreducible",
    reason: "border width — below --sp-1 floor",
  },
  // components/Page/Page.module.css
  {
    file: "components/Page/Page.module.css",
    literal: "1px",
    count: 1,
    kind: "irreducible",
    reason: "header/content divider border-top width — below --sp-1 floor",
  },
  {
    file: "components/Page/Page.module.css",
    literal: "6px",
    count: 1,
    kind: "irreducible",
    reason: "eyebrow margin-bottom from design — between --sp-1 and --sp-2",
  },
  {
    file: "components/Page/Page.module.css",
    literal: "4px",
    count: 1,
    kind: "irreducible",
    reason:
      "subtitle margin-top from design — equals --sp-1 but co-located with non-token siblings",
  },
  // 0079: .eyebrow letter-spacing now consumes var(--tracking-caps), so the
  // former '0.12em' literal exception is gone.
  {
    file: "components/Page/Page.module.css",
    literal: "0.01em",
    count: 1,
    kind: "irreducible",
    reason:
      "title negative letter-spacing — display-font tightening per design",
  },
  // components/Popover/Popover.module.css
  {
    file: "components/Popover/Popover.module.css",
    literal: "1px",
    count: 1,
    kind: "irreducible",
    reason: "panel border width — below --sp-1 floor",
  },
  {
    file: "components/Popover/Popover.module.css",
    literal: "240px",
    count: 1,
    kind: "irreducible",
    reason: "panel min-width — no token equivalent",
  },
  // routes/kanban/KanbanBoard.module.css
  {
    file: "routes/kanban/KanbanBoard.module.css",
    literal: "1px",
    count: 3,
    kind: "irreducible",
    reason:
      "border width (load-failure alert + retry button + Other-swimlane divider) — below --sp-1 floor",
  },
  {
    file: "routes/kanban/KanbanBoard.module.css",
    literal: "15rem",
    count: 1,
    kind: "irreducible",
    reason: "minimum kanban column track width — layout dimension, no token",
  },
  // routes/kanban/KanbanColumn.module.css
  {
    file: "routes/kanban/KanbanColumn.module.css",
    literal: "2px",
    count: 2,
    kind: "irreducible",
    reason: "columnOver outline width and offset — below --sp-1 floor",
  },
  {
    file: "routes/kanban/KanbanColumn.module.css",
    literal: "1px",
    count: 4,
    kind: "irreducible",
    reason:
      "column border + count-pill border + count-pill padding-block + empty-panel dashed border — below --sp-1 floor",
  },
  {
    file: "routes/kanban/KanbanColumn.module.css",
    literal: "300px",
    count: 1,
    kind: "irreducible",
    reason:
      "column min-height per prototype .ac-kcol — layout dimension, no token",
  },
  {
    file: "routes/kanban/KanbanColumn.module.css",
    literal: "6px",
    count: 2,
    kind: "irreducible",
    reason:
      "status-dot diameter (width/height) per prototype .ac-kcol__title .dot — below --sp-1 floor",
  },
  {
    file: "routes/kanban/KanbanColumn.module.css",
    literal: "7px",
    count: 1,
    kind: "irreducible",
    reason:
      "count-pill horizontal padding per prototype .ac-kcol__count — between --sp-1 and --sp-2",
  },
  {
    file: "routes/kanban/KanbanColumn.module.css",
    literal: "0.02em",
    count: 1,
    kind: "irreducible",
    reason:
      "column-title caps tracking per prototype .ac-kcol__title — below --tracking-caps",
  },
  // routes/kanban/WorkItemCard.module.css
  {
    file: "routes/kanban/WorkItemCard.module.css",
    literal: "1px",
    count: 3,
    kind: "irreducible",
    reason:
      "card border + foot dashed divider + hover translateY(-1px) — below --sp-1 floor",
  },
  {
    file: "routes/kanban/WorkItemCard.module.css",
    literal: "0.04em",
    count: 1,
    kind: "irreducible",
    reason:
      "card-id mono tracking per prototype .ac-kcard__id — below --tracking-caps",
  },
  // routes/kanban/WorkKindBadge.module.css
  {
    file: "routes/kanban/WorkKindBadge.module.css",
    literal: "1px",
    count: 2,
    kind: "irreducible",
    reason:
      "badge border + vertical padding per prototype .ac-kindbadge — below --sp-1 floor",
  },
  {
    file: "routes/kanban/WorkKindBadge.module.css",
    literal: "6px",
    count: 1,
    kind: "irreducible",
    reason:
      "badge horizontal padding per prototype .ac-kindbadge — between --sp-1 and --sp-2",
  },
  {
    file: "routes/kanban/WorkKindBadge.module.css",
    literal: "0.07em",
    count: 1,
    kind: "irreducible",
    reason:
      "kind-badge caps tracking per prototype .ac-kindbadge — below --tracking-caps",
  },
  // routes/kanban-card-showcase/KanbanCardShowcase.module.css — RETIRED in 0083
  // (the showcase route was deleted; its VR coverage moved to /dev#cards), so the
  // former 16rem exception is gone.
  // routes/library/LibraryDocView.module.css
  {
    file: "routes/library/LibraryDocView.module.css",
    literal: "4px",
    count: 1,
    kind: "irreducible",
    reason: "malformed-banner border-left width — no border-width token",
  },
  {
    file: "routes/library/LibraryDocView.module.css",
    literal: "1px",
    count: 3,
    kind: "irreducible",
    reason:
      "error border + aside border-left + dashed section divider — below --sp-1 floor",
  },
  {
    file: "routes/library/LibraryDocView.module.css",
    literal: "280px",
    count: 1,
    kind: "irreducible",
    reason: "aside column width — no token equivalent",
  },
  // routes/library/LibraryTemplatesIndex.module.css
  {
    file: "routes/library/LibraryTemplatesIndex.module.css",
    literal: "1px",
    count: 4,
    kind: "irreducible",
    reason:
      "connected-table outer border + per-row border-top + tier-pill borders — below --sp-1 floor",
  },
  {
    file: "routes/library/LibraryTemplatesIndex.module.css",
    literal: "0.02em",
    count: 1,
    kind: "irreducible",
    reason:
      "tier-pill letter-spacing — prototype-derived typography refinement (mirrors Chip)",
  },
  {
    file: "routes/library/LibraryTemplatesIndex.module.css",
    literal: "0.1875rem",
    count: 1,
    kind: "irreducible",
    reason:
      "tier-pill vertical padding (3px) — below --sp-1 floor; prototype-derived",
  },
  {
    file: "routes/library/LibraryTemplatesIndex.module.css",
    literal: "0.3125rem",
    count: 2,
    kind: "irreducible",
    reason:
      "tier-pill bullet width/height (5px) — below --sp-1 floor; prototype-derived (.ac-tier-pill .dot)",
  },
  // routes/library/LibraryTemplatesView.module.css
  {
    file: "routes/library/LibraryTemplatesView.module.css",
    literal: "1px",
    count: 4,
    kind: "irreducible",
    reason:
      "tier card + preview-pane + preview-header border-bottom hairlines — below --sp-1 floor",
  },
  {
    file: "routes/library/LibraryTemplatesView.module.css",
    literal: "3px",
    count: 1,
    kind: "irreducible",
    reason:
      "active-tier box-shadow halo spread — prototype-derived; below --sp-1 floor",
  },
  {
    file: "routes/library/LibraryTemplatesView.module.css",
    literal: "19rem",
    count: 1,
    kind: "irreducible",
    reason:
      "tier-card column width in the detail two-column grid — no token equivalent",
  },
  {
    file: "routes/library/LibraryTemplatesView.module.css",
    literal: "0.12em",
    count: 1,
    kind: "irreducible",
    reason:
      "detail heading caps letter-spacing — mirrors Sidebar section-heading",
  },
  {
    file: "routes/library/LibraryTemplatesView.module.css",
    literal: "0.1em",
    count: 1,
    kind: "irreducible",
    reason:
      "tier eyebrow caps letter-spacing — prototype-derived (.ac-tpl-tier__num)",
  },
  {
    file: "routes/library/LibraryTemplatesView.module.css",
    literal: "0.125rem",
    count: 1,
    kind: "irreducible",
    reason:
      "panel-header top margin to align with eyebrow — sub-pixel offset, prototype-derived",
  },
  {
    file: "routes/library/LibraryTemplatesView.module.css",
    literal: "0.375rem",
    count: 2,
    kind: "irreducible",
    reason:
      "tier card path/note top margin (6px) — sub-sp-2, prototype-derived",
  },
  {
    file: "routes/library/LibraryTemplatesView.module.css",
    literal: "1em",
    count: 1,
    kind: "irreducible",
    reason:
      "tpl-line min-height — relative-em line preservation, no token equivalent",
  },
  // routes/library/LibraryTypeView.module.css
  {
    file: "routes/library/LibraryTypeView.module.css",
    literal: "1px",
    count: 3,
    kind: "irreducible",
    reason: "header-row, row, and error border widths — below --sp-1 floor",
  },
  {
    file: "routes/library/LibraryTypeView.module.css",
    literal: "120px",
    count: 2,
    kind: "irreducible",
    reason: "grid column tracks (first column + status) — no token equivalent",
  },
  {
    file: "routes/library/LibraryTypeView.module.css",
    literal: "110px",
    count: 1,
    kind: "irreducible",
    reason: "grid column track (modified) — no token equivalent",
  },
  {
    file: "routes/library/LibraryTypeView.module.css",
    literal: "0.1em",
    count: 1,
    kind: "irreducible",
    reason:
      "column-header caps letter-spacing from design — slightly tighter than --tracking-caps",
  },
  {
    file: "routes/library/LibraryTypeView.module.css",
    literal: "10px",
    count: 1,
    kind: "irreducible",
    reason:
      "header-row vertical padding from design — between --sp-2 and --sp-3",
  },
  {
    file: "routes/library/LibraryTypeView.module.css",
    literal: "12px",
    count: 2,
    kind: "irreducible",
    reason:
      "header-row padding-inline at .headerRow; row padding-inline at .row — equals --sp-3 but co-located",
  },
  // routes/library/LibraryOverviewHub.module.css
  {
    file: "routes/library/LibraryOverviewHub.module.css",
    literal: "1px",
    count: 1,
    kind: "irreducible",
    reason: "card border width — below --sp-1 floor",
  },
  {
    file: "routes/library/LibraryOverviewHub.module.css",
    literal: "640px",
    count: 1,
    kind: "irreducible",
    reason: "responsive grid breakpoint — no token equivalent",
  },
  {
    file: "routes/library/LibraryOverviewHub.module.css",
    literal: "1024px",
    count: 1,
    kind: "irreducible",
    reason: "responsive grid breakpoint — no token equivalent",
  },
  {
    file: "routes/library/LibraryOverviewHub.module.css",
    literal: "10px",
    count: 2,
    kind: "irreducible",
    reason:
      "phase-heading bottom margin + card row gap from design — between --sp-2 and --sp-3",
  },
  {
    file: "routes/library/LibraryOverviewHub.module.css",
    literal: "12px",
    count: 2,
    kind: "irreducible",
    reason:
      "grid gap + card top-row gap from design — equals --sp-3 but co-located",
  },
  {
    file: "routes/library/LibraryOverviewHub.module.css",
    literal: "14px",
    count: 1,
    kind: "irreducible",
    reason: "card padding-block at .card — between --sp-3 and --sp-4",
  },
  {
    file: "routes/library/LibraryOverviewHub.module.css",
    literal: "16px",
    count: 2,
    kind: "irreducible",
    reason:
      "card padding-left/-right + card column gap from design — equals --sp-4 but co-located",
  },
  {
    file: "routes/library/LibraryOverviewHub.module.css",
    literal: "6px",
    count: 2,
    kind: "irreducible",
    reason:
      "card pinstripe stride stops from design — between --sp-1 and --sp-2",
  },
  {
    file: "routes/library/LibraryOverviewHub.module.css",
    literal: "7px",
    count: 1,
    kind: "irreducible",
    reason: "pinstripe stride end from design — between --sp-1 and --sp-2",
  },
  {
    file: "routes/library/LibraryOverviewHub.module.css",
    literal: "4px",
    count: 1,
    kind: "irreducible",
    reason: "card body row gap — equals --sp-1 but co-located",
  },
  {
    file: "routes/library/LibraryOverviewHub.module.css",
    literal: "0.12em",
    count: 1,
    kind: "irreducible",
    reason: "phase-heading caps letter-spacing",
  },
  // routes/library/EmptyState.module.css — full-page list-view empty state
  {
    file: "routes/library/EmptyState.module.css",
    literal: "1px",
    count: 2,
    kind: "irreducible",
    reason: "dashed card outline + dashed foot top border — below --sp-1 floor",
  },
  {
    file: "routes/library/EmptyState.module.css",
    literal: "96px",
    count: 1,
    kind: "irreducible",
    reason: "BigGlyph hero column track from design — no token equivalent",
  },
  {
    file: "routes/library/EmptyState.module.css",
    literal: "28px",
    count: 3,
    kind: "irreducible",
    reason:
      "card grid gap + horizontal padding + top padding from design — between --sp-5 and --sp-6",
  },
  {
    file: "routes/library/EmptyState.module.css",
    literal: "26px",
    count: 1,
    kind: "irreducible",
    reason: "card bottom padding from design — between --sp-5 and --sp-6",
  },
  {
    file: "routes/library/EmptyState.module.css",
    literal: "22px",
    count: 1,
    kind: "irreducible",
    reason:
      "card responsive padding-block at .card — equals --size-lg but co-located",
  },
  {
    file: "routes/library/EmptyState.module.css",
    literal: "14px",
    count: 1,
    kind: "irreducible",
    reason: "foot padding-top at .foot — between --sp-3 and --sp-4",
  },
  {
    file: "routes/library/EmptyState.module.css",
    literal: "16px",
    count: 2,
    kind: "irreducible",
    reason:
      "lede margin-bottom + responsive grid gap — equals --sp-4 but co-located",
  },
  {
    file: "routes/library/EmptyState.module.css",
    literal: "8px",
    count: 1,
    kind: "irreducible",
    reason: "title margin-bottom from design — equals --sp-2 but co-located",
  },
  {
    file: "routes/library/EmptyState.module.css",
    literal: "4px",
    count: 1,
    kind: "irreducible",
    reason: "eyebrow margin-bottom — equals --sp-1 but co-located",
  },
  {
    file: "routes/library/EmptyState.module.css",
    literal: "2px",
    count: 1,
    kind: "irreducible",
    reason: "hero top padding — below --sp-1 floor",
  },
  {
    file: "routes/library/EmptyState.module.css",
    literal: "820px",
    count: 1,
    kind: "irreducible",
    reason: "responsive collapse breakpoint — no token equivalent",
  },
  {
    file: "routes/library/EmptyState.module.css",
    literal: "0.04em",
    count: 1,
    kind: "irreducible",
    reason:
      "eyebrow letter-spacing from design — between --tracking-caps and 0",
  },
  // routes/library/recovery/RecoverySurface.module.css — shared 404 / catch-all
  // / load-error surface; reuses the EmptyState hero+illustration layout.
  {
    file: "routes/library/recovery/RecoverySurface.module.css",
    literal: "96px",
    count: 1,
    kind: "irreducible",
    reason: "BigGlyph hero column track from design — no token equivalent",
  },
  {
    file: "routes/library/recovery/RecoverySurface.module.css",
    literal: "28px",
    count: 3,
    kind: "irreducible",
    reason:
      "card grid gap + horizontal padding + top padding from design — between --sp-5 and --sp-6",
  },
  {
    file: "routes/library/recovery/RecoverySurface.module.css",
    literal: "26px",
    count: 2,
    kind: "irreducible",
    reason:
      "card bottom padding + suggestion-row glyph column track — between --sp-5 and --sp-6",
  },
  {
    file: "routes/library/recovery/RecoverySurface.module.css",
    literal: "22px",
    count: 1,
    kind: "irreducible",
    reason:
      "card responsive padding-block at .card — equals --size-lg but co-located",
  },
  {
    file: "routes/library/recovery/RecoverySurface.module.css",
    literal: "20px",
    count: 1,
    kind: "irreducible",
    reason: "suggestion block bottom margin — between --sp-4 and --sp-5",
  },
  {
    file: "routes/library/recovery/RecoverySurface.module.css",
    literal: "6px",
    count: 1,
    kind: "irreducible",
    reason: "suggestion-row vertical padding — between --sp-1 and --sp-2",
  },
  {
    file: "routes/library/recovery/RecoverySurface.module.css",
    literal: "2px",
    count: 2,
    kind: "irreducible",
    reason: "hero top padding + suggestion-list row gap — below --sp-1 floor",
  },
  {
    file: "routes/library/recovery/RecoverySurface.module.css",
    literal: "1px",
    count: 2,
    kind: "irreducible",
    reason:
      "dashed card outline + suggestion-row sub-label top margin — below --sp-1 floor",
  },
  {
    file: "routes/library/recovery/RecoverySurface.module.css",
    literal: "820px",
    count: 1,
    kind: "irreducible",
    reason: "responsive collapse breakpoint — no token equivalent",
  },
  // routes/library/NoResultsPanel.module.css
  {
    file: "routes/library/NoResultsPanel.module.css",
    literal: "1px",
    count: 2,
    kind: "irreducible",
    reason: "panel border widths — below --sp-1 floor",
  },
  // components/SortPill/SortPill.module.css
  {
    file: "components/SortPill/SortPill.module.css",
    literal: "1px",
    count: 2,
    kind: "irreducible",
    reason: "trigger + menu-header divider widths — below --sp-1 floor",
  },
  {
    file: "components/SortPill/SortPill.module.css",
    literal: "220px",
    count: 1,
    kind: "irreducible",
    reason: "menu min-width — no token equivalent",
  },
  {
    file: "components/SortPill/SortPill.module.css",
    literal: "6px",
    count: 3,
    kind: "irreducible",
    reason:
      "trigger vertical padding + gap + menu-header v-padding from design — between --sp-1 and --sp-2",
  },
  {
    file: "components/SortPill/SortPill.module.css",
    literal: "10px",
    count: 3,
    kind: "irreducible",
    reason:
      "trigger + menu-header + menu-item horizontal padding from design — between --sp-2 and --sp-3",
  },
  {
    file: "components/SortPill/SortPill.module.css",
    literal: "7px",
    count: 1,
    kind: "irreducible",
    reason:
      "menu-item vertical padding from design — between --sp-1 and --sp-2",
  },
  {
    file: "components/SortPill/SortPill.module.css",
    literal: "8px",
    count: 2,
    kind: "irreducible",
    reason:
      "menu-header bottom padding + menu-item gap from design — equals --sp-2 but co-located",
  },
  {
    file: "components/SortPill/SortPill.module.css",
    literal: "4px",
    count: 1,
    kind: "irreducible",
    reason:
      "menu-header bottom margin from design — equals --sp-1 but co-located",
  },
  {
    file: "components/SortPill/SortPill.module.css",
    literal: "0.12em",
    count: 1,
    kind: "irreducible",
    reason: "menu-header caps letter-spacing — matches sidebar headings",
  },
  // components/DetailHeaderActions/HeaderActionButton.module.css — labelled
  // detail-header pill matching the prototype .ac-topbar__btn (same literals
  // the SortPill/FilterPill pill family uses).
  {
    file: "components/DetailHeaderActions/HeaderActionButton.module.css",
    literal: "6px",
    count: 2,
    kind: "irreducible",
    reason:
      "pill vertical padding + icon/label gap from prototype .ac-topbar__btn — between --sp-1 and --sp-2",
  },
  {
    file: "components/DetailHeaderActions/HeaderActionButton.module.css",
    literal: "10px",
    count: 1,
    kind: "irreducible",
    reason:
      "pill horizontal padding from prototype .ac-topbar__btn — between --sp-2 and --sp-3",
  },
  {
    file: "components/DetailHeaderActions/HeaderActionButton.module.css",
    literal: "1px",
    count: 2,
    kind: "irreducible",
    reason:
      "resting transparent border + forced-colors border widths — below --sp-1 floor",
  },
  // components/FilterPill/FilterPill.module.css
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "1px",
    count: 6,
    kind: "irreducible",
    reason:
      "trigger / checkbox / search / dashed dividers + checkmark translateY — below --sp-1 floor",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "260px",
    count: 1,
    kind: "irreducible",
    reason: "menu min-width — no token equivalent",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "200px",
    count: 1,
    kind: "irreducible",
    reason: "long-facet scroll max-height — no token equivalent",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "14px",
    count: 1,
    kind: "irreducible",
    reason: "checkbox column track — fixed pixel; no token equivalent",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "13px",
    count: 2,
    kind: "irreducible",
    reason: "checkbox width/height — fixed pixel; no token equivalent",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "2px",
    count: 3,
    kind: "irreducible",
    reason:
      "clear-button padding + scroll padding-right + search top margin — below --sp-1 floor",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "6px",
    count: 10,
    kind: "irreducible",
    reason:
      "trigger / search / scrollbar / no-matches / facet padding / gap / margin — between --sp-1 and --sp-2",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "1.5px",
    count: 2,
    kind: "irreducible",
    reason: "checkmark stroke widths — below --sp-1 floor",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "4px",
    count: 5,
    kind: "irreducible",
    reason:
      "facet padding + search padding + facet-heading padding + clear-button — equals --sp-1 but co-located",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "5px",
    count: 2,
    kind: "irreducible",
    reason: "badge + option horizontal padding — between --sp-1 and --sp-2",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "10px",
    count: 2,
    kind: "irreducible",
    reason:
      "trigger padding-inline at .trigger; menu-header padding-inline at .menuHeader — between --sp-2 and --sp-3",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "16px",
    count: 2,
    kind: "irreducible",
    reason: "badge min-width/height — no token equivalent",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "7px",
    count: 1,
    kind: "irreducible",
    reason: "checkmark width — fixed pixel",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "8px",
    count: 7,
    kind: "irreducible",
    reason:
      "option/facet/heading paddings + option gaps from design — equals --sp-2 but co-located",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "0.12em",
    count: 1,
    kind: "irreducible",
    reason: "menu-header caps letter-spacing",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "0.1em",
    count: 1,
    kind: "irreducible",
    reason: "facet-heading caps letter-spacing",
  },
  {
    file: "components/FilterPill/FilterPill.module.css",
    literal: "#ffffff",
    count: 3,
    kind: "irreducible",
    reason:
      "checkmark stroke + badge text on --ac-accent — theme-invariant white",
  },
  // routes/lifecycle/LifecycleClusterView.module.css — see top of file
  // for the consolidated entry; this section is intentionally empty
  // (the previous timeline implementation had a different shape and
  // those entries are obsolete).
  // routes/lifecycle/LifecycleIndex.module.css
  {
    file: "routes/lifecycle/LifecycleIndex.module.css",
    literal: "1px",
    count: 4,
    kind: "irreducible",
    reason:
      "card, sort-segment, error-state, and dashed pipe border-width — below --sp-1 floor",
  },
  {
    file: "routes/lifecycle/LifecycleIndex.module.css",
    literal: "2px",
    count: 3,
    kind: "irreducible",
    reason: "cardHeading gap and focus outline — below --sp-1 floor",
  },
  {
    file: "routes/lifecycle/LifecycleIndex.module.css",
    literal: "10px",
    count: 2,
    kind: "irreducible",
    reason:
      "pipeline strip gap and top spacing — prototype-spec literal, between --sp-2 (8) and --sp-3 (12)",
  },
  {
    file: "routes/lifecycle/LifecycleIndex.module.css",
    literal: "18px",
    count: 1,
    kind: "irreducible",
    reason:
      "cardLink vertical padding — prototype-spec literal, between --sp-4 (16) and --sp-5 (24)",
  },
  {
    file: "routes/lifecycle/LifecycleIndex.module.css",
    literal: "20px",
    count: 1,
    kind: "irreducible",
    reason:
      "cardLink horizontal padding — prototype-spec literal, between --sp-4 (16) and --sp-5 (24)",
  },
  // styles/wiki-links.global.css
  {
    file: "styles/wiki-links.global.css",
    literal: "1px",
    count: 1,
    kind: "irreducible",
    reason: "border-bottom width — below --sp-1 floor",
  },
  // components/Toaster/Toaster.module.css
  {
    file: "components/Toaster/Toaster.module.css",
    literal: "320px",
    count: 1,
    kind: "irreducible",
    reason: "viewport min-width per prototype — no token equivalent",
  },
  {
    file: "components/Toaster/Toaster.module.css",
    literal: "420px",
    count: 1,
    kind: "irreducible",
    reason: "viewport max-width per prototype — no token equivalent",
  },
  {
    file: "components/Toaster/Toaster.module.css",
    literal: "1px",
    count: 1,
    kind: "irreducible",
    reason: "toast card border width — below --sp-1 floor",
  },
  {
    file: "components/Toaster/Toaster.module.css",
    literal: "3px",
    count: 1,
    kind: "irreducible",
    reason: "toast border-left accent bar per prototype — below --sp-1 floor",
  },
  {
    file: "components/Toaster/Toaster.module.css",
    literal: "2px",
    count: 3,
    kind: "irreducible",
    reason:
      "icon margin-top optical alignment + close focus outline width + outline-offset — below --sp-1 floor",
  },
  {
    file: "components/Toaster/Toaster.module.css",
    literal: "4px",
    count: 2,
    kind: "irreducible",
    reason:
      "close button negative top/right margin to recover card padding — equals --sp-1 but co-located with sign",
  },
  {
    file: "components/Toaster/Toaster.module.css",
    literal: "0.1rem",
    count: 1,
    kind: "irreducible",
    reason:
      "inline code vertical padding — mirrors MarkdownRenderer `.markdown code:not(pre code)`",
  },
];

// Build O(1) lookup maps once at module load
const exceptionsByFile = new Map<string, Map<string, number>>();
for (const e of EXCEPTIONS) {
  let inner = exceptionsByFile.get(e.file);
  if (!inner) {
    inner = new Map();
    exceptionsByFile.set(e.file, inner);
  }
  inner.set(e.literal, (inner.get(e.literal) ?? 0) + e.count);
}

// Map vite-glob keys to the src-relative form used by EXCEPTIONS.file.
// Files in src/styles/ (same dir as this test) return a './' key — Vite
// normalises '../styles/foo.css' → './foo.css' when src and target share
// the same directory. Prepend 'styles/' to match EXCEPTIONS entries.
// Files in other src/ subdirectories return '../<rest>' keys as expected.
function srcRelative(globKey: string): string {
  if (globKey.startsWith("./")) {
    return `styles/${globKey.slice(2)}`;
  }
  if (!globKey.startsWith("../") || globKey.startsWith("../../")) {
    throw new Error(
      `srcRelative: unexpected glob key shape "${globKey}". ` +
        `Expected "./" (same-dir) or exactly one "../" prefix (test sits at src/styles/, globs "../**/*.module.css"). ` +
        `If the test or glob has been moved, update srcRelative accordingly.`,
    );
  }
  return globKey.slice(3);
}

function permittedCount(file: string, literal: string): number {
  return exceptionsByFile.get(srcRelative(file))?.get(literal) ?? 0;
}

function violations(matches: string[], file: string): string[] {
  const counts = new Map<string, number>();
  for (const m of matches) counts.set(m, (counts.get(m) ?? 0) + 1);
  const result: string[] = [];
  for (const [literal, observed] of counts) {
    const allowed = permittedCount(file, literal);
    if (observed > allowed) {
      for (let i = 0; i < observed - allowed; i++) result.push(literal);
    }
  }
  return result;
}

const allCss = { ...cssModules, ...cssGlobals };

describe("AC3: no hex literals outside EXCEPTIONS", () => {
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} hex literals all accounted for`, () => {
      const matches = [...css.matchAll(HEX_RE)].map((m) => m[0]);
      expect(violations(matches, path)).toEqual([]);
    });
  }
});

describe("AC4: no px/rem/em literals outside EXCEPTIONS (0-resets auto-excluded)", () => {
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} px/rem/em literals all accounted for`, () => {
      const matches = [...css.matchAll(PX_REM_EM_RE)].map((m) => m[0]);
      expect(violations(matches, path)).toEqual([]);
    });
  }
});

describe("var(--token, fallback) two-arg form is retired", () => {
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} contains no var(--*, fallback) sites`, () => {
      const fallbacks = [...css.matchAll(VAR_FALLBACK_RE)].map((m) => m[0]);
      expect(fallbacks).toEqual([]);
    });
  }
});

describe("color-mix() convention (Phase 4 special conventions)", () => {
  // Status-tint family: percentage ladder 8 / 18 / 30, composition
  // surface always var(--ac-bg). Used for `err`/`warn`/`ok`/`violet`
  // status surfaces.
  const STATUS_COLOR_MIX_RE =
    /color-mix\(\s*in\s+srgb\s*,\s*var\(--ac-(err|warn|ok|violet)\)\s+(\d+)%\s*,\s*var\(--ac-bg\)\s*\)/g;
  const STATUS_ALLOWED_PERCENTAGES = new Set([8, 18, 30]);
  // Pipeline-tile family: opaque pale (over var(--ac-bg-card)) for
  // light mode and translucent (over transparent) for dark mode.
  // Mixed with `currentColor` so the tint inherits whichever
  // `--ac-stage-*` accent the surrounding context set. Percentages
  // intentionally cover both bg fills (14%) and borders (22% / 30%).
  const PIPELINE_COLOR_MIX_RE =
    /color-mix\(\s*in\s+srgb\s*,\s*currentColor\s+(\d+)%\s*,\s*(var\(--ac-bg-card\)|transparent)\s*\)/g;
  const PIPELINE_ALLOWED_PERCENTAGES = new Set([14, 22, 30]);
  const COLOR_MIX_ANY_RE = /color-mix\(/g;

  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} color-mix sites use the locked-in convention`, () => {
      const totalSites = (css.match(COLOR_MIX_ANY_RE) ?? []).length;
      const statusSites = [...css.matchAll(STATUS_COLOR_MIX_RE)];
      const pipelineSites = [...css.matchAll(PIPELINE_COLOR_MIX_RE)];
      expect(statusSites.length + pipelineSites.length).toBe(totalSites);
      for (const m of statusSites) {
        expect(STATUS_ALLOWED_PERCENTAGES.has(parseInt(m[2], 10))).toBe(true);
      }
      for (const m of pipelineSites) {
        expect(PIPELINE_ALLOWED_PERCENTAGES.has(parseInt(m[1], 10))).toBe(true);
      }
    });
  }
});

describe("var(--NAME) references resolve to declared tokens", () => {
  const declared = new Set([
    ...Object.keys(LIGHT_COLOR_TOKENS),
    ...Object.keys(DARK_COLOR_TOKENS),
    ...Object.keys(TYPOGRAPHY_TOKENS),
    ...Object.keys(SPACING_TOKENS),
    ...Object.keys(RADIUS_TOKENS),
    ...Object.keys(LIGHT_SHADOW_TOKENS),
    ...Object.keys(DARK_SHADOW_TOKENS),
    ...Object.keys(LAYOUT_TOKENS),
    ...Object.keys(CODE_SURFACE_TOKENS),
    ...Object.keys(CODE_SYNTAX_TOKENS),
  ]);
  // File-scoped local custom properties — declared at the top of the
  // module's own stylesheet and consumed only within that file. These
  // never make it into the global token set but still need to pass the
  // "var(--*) resolves to something" guard.
  const LOCAL_CUSTOM_PROPS: Record<string, ReadonlySet<string>> = {
    "components/DevDesignSystem/DevDesignSystem.module.css": new Set([
      "ds-gutter",
      "ds-section-gap",
      "ds-marquee-h",
      // per-cell empty-state-glyph hue, set inline on each big-glyph hero
      "bg-hue",
    ]),
    "routes/library/EmptyState.module.css": new Set(["ac-empty-page-hue"]),
    "routes/library/recovery/RecoverySurface.module.css": new Set([
      "ac-empty-page-hue",
    ]),
    "routes/lifecycle/LifecycleClusterView.module.css": new Set([
      "spine-x",
      "dot-size",
    ]),
    "components/Pipeline/Pipeline.module.css": new Set(["next-accent"]),
    "components/Toaster/Toaster.module.css": new Set(["toast-accent"]),
    // --kanban-cols is set inline (React style) on the .columns grid from the
    // configured column count; consumed only here as the grid track repeat.
    "routes/kanban/KanbanBoard.module.css": new Set(["kanban-cols"]),
  };
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} references only declared tokens`, () => {
      const refs = [...css.matchAll(VAR_REF_RE)].map((m) => m[1]);
      const local = LOCAL_CUSTOM_PROPS[srcRelative(path)] ?? new Set<string>();
      const unknown = refs.filter(
        (name) => !declared.has(name) && !local.has(name),
      );
      expect(unknown).toEqual([]);
    });
  }
});

// AC5 enforcement is a true two-sided ratchet:
//
// - `AC5_FLOOR` is the *committed minimum*. It MUST equal the value
//   observed at the last committed state (or below it by no more than
//   AC5_REGRESSION_SLACK). The implementer bumps AC5_FLOOR upward in
//   the same commit that adds new var(--*) references.
// - `AC5_TARGET = 300` is the work-item contract.
const AC5_FLOOR = 1532; // 0083: P11 retired the 5 showcase modules (1622→1546); prototype-fidelity passes removed the deviations panel + dead .tocActions (1546→1529) then re-added the shared .row helper + TOC line-height (1529→1532). Coverage in surviving files is unchanged.
const AC5_TARGET = 300; // contract from work item AC5
const AC5_REGRESSION_SLACK = 0;

describe("AC5: aggregate var(--*) coverage (two-sided ratchet)", () => {
  const observed = Object.values(cssModules).reduce(
    (acc, css) => acc + (css.match(VAR_COUNT_RE)?.length ?? 0),
    0,
  );

  it(`observed count (${observed}) is at least AC5_FLOOR (${AC5_FLOOR})`, () => {
    expect(observed).toBeGreaterThanOrEqual(AC5_FLOOR - AC5_REGRESSION_SLACK);
  });

  it(`AC5_FLOOR (${AC5_FLOOR}) is not above observed (${observed}) — bump protocol followed`, () => {
    expect(AC5_FLOOR).toBeLessThanOrEqual(observed);
  });

  const finalStateActive = AC5_FLOOR >= AC5_TARGET;
  (finalStateActive ? it : it.skip)(
    `(final-state gate) observed reaches AC5_TARGET (${AC5_TARGET})`,
    () => {
      expect(observed).toBeGreaterThanOrEqual(AC5_TARGET);
    },
  );
});

// Build the inverse map for hygiene checks.
const cssBySrcRelative = new Map<string, string>();
for (const [globKey, css] of Object.entries(allCss)) {
  cssBySrcRelative.set(srcRelative(globKey), css);
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

describe("EXCEPTIONS hygiene", () => {
  it("every EXCEPTIONS entry resolves to exactly one CSS file", () => {
    const unresolved: Exception[] = [];
    for (const e of EXCEPTIONS) {
      if (!cssBySrcRelative.has(e.file)) unresolved.push(e);
    }
    expect(unresolved).toEqual([]);
  });

  it("declared count equals observed count (no stale entries, no over-count)", () => {
    const mismatches: Array<{
      file: string;
      literal: string;
      declared: number;
      observed: number;
    }> = [];
    for (const [file, literalMap] of exceptionsByFile) {
      const css = cssBySrcRelative.get(file);
      if (!css) continue;
      const hexHits = [...css.matchAll(HEX_RE)].map((m) => m[0]);
      const unitHits = [...css.matchAll(PX_REM_EM_RE)].map((m) => m[0]);
      const allHits = [...hexHits, ...unitHits];
      for (const [literal, declared] of literalMap) {
        const observed = allHits.filter((h) => h === literal).length;
        if (observed !== declared) {
          mismatches.push({ file, literal, declared, observed });
        }
      }
    }
    expect(mismatches).toEqual([]);
  });
});

describe("MarkdownRenderer .markdown rule consumes prose-width and body-size tokens", () => {
  const path = "components/MarkdownRenderer/MarkdownRenderer.module.css";
  const css = cssBySrcRelative.get(path);
  const itIfPresent = css ? it : it.skip;

  it("the file is discoverable", () => {
    expect(css, `expected ${path} to be globbed by cssModules`).toBeDefined();
  });

  itIfPresent(
    "references var(--ac-content-max-width-prose) for the prose cap",
    () => {
      expect(css!).toContain("var(--ac-content-max-width-prose)");
    },
  );

  itIfPresent("references var(--size-prose) for the body font-size", () => {
    expect(css!).toContain("var(--size-prose)");
  });
});

describe("MarkdownRenderer inline-code rule (0094)", () => {
  const path = "components/MarkdownRenderer/MarkdownRenderer.module.css";
  const css = cssBySrcRelative.get(path);
  const itIfPresent = css ? it : it.skip;
  itIfPresent("inline code uses the monospace face", () => {
    expect(css!).toContain("font-family: var(--ac-font-mono)");
  });
  itIfPresent("inline code uses the 11.5px token, not --size-xs", () => {
    expect(css!).toContain("var(--size-xxs-sm)");
  });
  itIfPresent("inline code has the soft pill border", () => {
    expect(css!).toContain("1px solid var(--ac-stroke-soft)");
  });
  itIfPresent("inline code retains the :not(pre code) scoping", () => {
    expect(css!).toContain("code:not(pre code)");
  });
  itIfPresent("inline code no longer sizes off --size-xs", () => {
    // indexOf finds the base rule `.markdown code:not(pre code)` first (the
    // td override shares the substring); scope the negative check to its body
    // so it does not trip on `.markdown pre`'s legitimate var(--size-xs).
    const i = css!.indexOf("code:not(pre code)");
    const body = i >= 0 ? extractBlockBody(css!, i) : null;
    expect(body).not.toBeNull();
    expect(body!).not.toContain("var(--size-xs)");
  });
  itIfPresent(
    "table-body inline code uses the 11px token, out-specifying the base rule",
    () => {
      expect(css!).toContain("td code:not(pre code)");
      expect(css!).toContain("var(--size-eyebrow)");
    },
  );
});

describe("MarkdownRenderer task-list rule (0095)", () => {
  const path = "components/MarkdownRenderer/MarkdownRenderer.module.css";
  const css = cssBySrcRelative.get(path);
  const itIfPresent = css ? it : it.skip;
  itIfPresent("tasklist removes the list marker", () => {
    expect(css!).toContain("list-style: none");
  });
  itIfPresent(
    "unchecked box borders off --ac-stroke-strong (single-arg var)",
    () => {
      expect(css!).toContain("border: 1.5px solid var(--ac-stroke-strong)");
    },
  );
  itIfPresent("box fills off --ac-bg-card", () => {
    expect(css!).toContain("background: var(--ac-bg-card)");
  });
  itIfPresent("checked box fills + borders off --ac-accent", () => {
    expect(css!).toContain("background: var(--ac-accent)");
    expect(css!).toContain("border-color: var(--ac-accent)");
  });
  itIfPresent(
    "done label is muted + struck through off --ac-stroke-strong",
    () => {
      expect(css!).toContain("color: var(--ac-fg-muted)");
      expect(css!).toContain("text-decoration: line-through");
      expect(css!).toContain("text-decoration-color: var(--ac-stroke-strong)");
    },
  );
});

// Suppress unused-variable warning — escapeRegExp is available for future
// use by harness extensions that need to escape literal strings in regexes.
void escapeRegExp;

// Brace-balanced extraction of a `{ ... }` block body. Retained as a
// utility for future selector-body assertions (matches the convention
// of `escapeRegExp` above); suppressed from the unused-locals check.
function extractBlockBody(css: string, startIdx: number): string | null {
  const open = css.indexOf("{", startIdx);
  if (open === -1) return null;
  let depth = 1;
  let i = open + 1;
  while (i < css.length && depth > 0) {
    const ch = css[i];
    if (ch === "{") depth++;
    else if (ch === "}") depth--;
    i++;
  }
  return depth === 0 ? css.slice(open + 1, i - 1) : null;
}
void extractBlockBody;

describe("0038: --radius-pill is reserved for non-status surfaces", () => {
  // Files that legitimately use --radius-pill. Every other consumer in
  // src/ would be an open-coded status pill regression and must be
  // migrated to <Chip>. Adding a new entry to this list requires a
  // brief reason recorded alongside the path.
  const PILL_RADIUS_ALLOW_LIST = new Set([
    "components/Chip/Chip.module.css",
    "components/OriginPill/OriginPill.module.css",
    "components/Sidebar/Sidebar.module.css",
    "components/FilterPill/FilterPill.module.css",
    "routes/kanban/KanbanColumn.module.css",
    "routes/lifecycle/LifecycleIndex.module.css",
  ]);

  it("no module outside the allow-list defines a pill-radius element", () => {
    const offenders: string[] = [];
    for (const [file, css] of cssBySrcRelative) {
      if (PILL_RADIUS_ALLOW_LIST.has(file)) continue;
      if (/border-radius:\s*var\(--radius-pill\)/.test(css)) {
        offenders.push(file);
      }
    }
    expect(offenders).toEqual([]);
  });
});

// Phase 1 (0034)'s per-route `.title { color: var(--ac-fg-strong) }`
// guard has been retired: every consumer route now renders its title
// through the shared `<Page>` wrapper, which owns the title styling
// centrally. The Page test suite asserts the colour binding.

// AUTHORITATIVE IMPLEMENTATION of ADR-0036's font-size consumption
// rule. The FONT_SIZE_LITERAL_RE and FONT_SHORTHAND_RE regexes here
// are the load-bearing CI guard. The three AC2 ripgrep sweeps below
// are coarser approximations used at the review-time grep gate; the
// regexes above are the authoritative test of compliance. The AC2
// sweeps may flag additional candidates (e.g. unit-bearing
// line-heights inside `font:` shorthand) that this test correctly
// excludes via the negated character class on the shorthand regex;
// any such grep hit must be inspected and dismissed only if confirmed
// to be the line-height slot. Any structural edit to the regexes here
// requires re-deriving the AC2 sweep guidance in the same commit.
//
// AC2 sweeps (review-time grep gate):
//   rg --glob '**/*.module.css' 'font-size:\s*[.0-9]' src
//   rg --glob '**/*.css' --glob '!**/global.css' 'font-size:\s*[.0-9]' src
//   rg --glob '**/*.module.css' 'font:\s*[^;]*\s[.0-9]+(px|rem|em)' src

const globalCssModules = import.meta.glob("../styles/global.css", {
  eager: true,
  query: "?raw",
  import: "default",
}) as Record<string, string>;

if (Object.keys(globalCssModules).length !== 1) {
  throw new Error(
    `Expected exactly one match for global.css glob, got ${Object.keys(globalCssModules).length}: ` +
      `${Object.keys(globalCssModules).join(", ")}. ` +
      `Update Phase 8.1's glob path if styles/global.css was moved.`,
  );
}

const FONT_SIZE_LITERAL_EXCEPTIONS: ReadonlyArray<{
  file: string;
  literal: string;
  count: number;
  reason: string;
  reference: string;
}> = [];

// Shared by both literal-ban gates (font-size + border-radius). Hoisted to
// module scope so the AC4 / 0090 radius gate reuses one definition rather than
// re-inlining the comment-strip regex.
const stripComments = (css: string) => css.replace(/\/\*[\s\S]*?\*\//g, "");

describe("AC2 / 0075: no font-size literals in module or global CSS", () => {
  const FONT_SIZE_LITERAL_RE =
    /(?<![\w-])font-size:\s*(\d+(?:\.\d+)?|\.\d+)\s*(px|rem|em)\b/g;
  const FONT_SHORTHAND_RE =
    /(?<![\w-])font:[^;/]*?(\d+(?:\.\d+)?|\.\d+)(px|rem|em)\b/g;

  const allCssWithRoot = {
    ...allCss,
    ...Object.fromEntries(
      Object.entries(globalCssModules).map(([, css]) => [
        "styles/global.css",
        css,
      ]),
    ),
  };

  for (const [path, css] of Object.entries(allCssWithRoot)) {
    it(`${path}: no font-size literal or shorthand with embedded size`, () => {
      const cssNoComments = stripComments(css);
      const fontSizeHits = [
        ...cssNoComments.matchAll(FONT_SIZE_LITERAL_RE),
      ].map((m) => `${m[1]}${m[2]}`);
      const shorthandHits = [...cssNoComments.matchAll(FONT_SHORTHAND_RE)].map(
        (m) => `${m[1]}${m[2]}`,
      );

      const exemptForFile = new Map<string, number>();
      for (const e of FONT_SIZE_LITERAL_EXCEPTIONS) {
        if (e.file === path) {
          exemptForFile.set(
            e.literal,
            (exemptForFile.get(e.literal) ?? 0) + e.count,
          );
        }
      }

      const consumed = new Map<string, number>();
      function subtractExemptions(hits: string[]): string[] {
        const remaining: string[] = [];
        for (const hit of hits) {
          const budget = exemptForFile.get(hit) ?? 0;
          const used = consumed.get(hit) ?? 0;
          if (used < budget) {
            consumed.set(hit, used + 1);
          } else {
            remaining.push(hit);
          }
        }
        return remaining;
      }

      expect({
        fontSize: subtractExemptions(fontSizeHits),
        shorthand: subtractExemptions(shorthandHits),
      }).toEqual({ fontSize: [], shorthand: [] });
    });
  }
});

describe("AC2 / 0075: font-size literal regex fixtures", () => {
  const FONT_SIZE_LITERAL_RE =
    /(?<![\w-])font-size:\s*(\d+(?:\.\d+)?|\.\d+)\s*(px|rem|em)\b/g;
  const FONT_SHORTHAND_RE =
    /(?<![\w-])font:[^;/]*?(\d+(?:\.\d+)?|\.\d+)(px|rem|em)\b/g;

  const POSITIVE_LITERAL = [
    "font-size: 0.88em;",
    "font-size: .5px;",
    "font-size:14px;",
  ];
  const POSITIVE_SHORTHAND = [
    "font: 400 12px/1.5 var(--ac-font-body);",
    "font:12px/1 sans;",
    "font: italic 400 .9em/1 sans;",
    "font:.5px/1 sans;",
    "font: bold 14px sans;",
  ];
  const NEGATIVE_LITERAL = [
    "--size-foo: 11px;",
    "--my-font-size: 12px;",
    "--ac-font-size-base: 14px;",
    "border-radius: 12px;",
    "transform: scale(1.2em);",
  ];
  const NEGATIVE_SHORTHAND = [
    "--ac-font: bold 14px sans;",
    "--my-line-height: 1.5rem;",
    'font-family: "Inter", sans-serif;',
    "font: 400 var(--size-xxs)/1 var(--ac-font-mono);",
    "font: 400 var(--size-xxs)/1.5rem var(--ac-font-body);",
    "font: 14pxsans;",
  ];

  for (const css of POSITIVE_LITERAL) {
    it(`literal regex flags: ${css}`, () => {
      expect([...css.matchAll(FONT_SIZE_LITERAL_RE)].length).toBeGreaterThan(0);
    });
  }
  for (const css of POSITIVE_SHORTHAND) {
    it(`shorthand regex flags: ${css}`, () => {
      expect([...css.matchAll(FONT_SHORTHAND_RE)].length).toBeGreaterThan(0);
    });
  }
  for (const css of NEGATIVE_LITERAL) {
    it(`literal regex skips: ${css}`, () => {
      expect([...css.matchAll(FONT_SIZE_LITERAL_RE)].length).toBe(0);
    });
  }
  for (const css of NEGATIVE_SHORTHAND) {
    it(`shorthand regex skips: ${css}`, () => {
      expect([...css.matchAll(FONT_SHORTHAND_RE)].length).toBe(0);
    });
  }
});

// AUTHORITATIVE IMPLEMENTATION of ADR-0039's border-radius consumption
// rule. BORDER_RADIUS_LITERAL_RE is the load-bearing CI guard; the three
// AC3 ripgrep sweeps below are coarser review-time approximations.
//
// AC3 sweeps (review-time grep gate, run from frontend/src):
//   rg --glob '**/*.module.css' 'border-radius:\s*[.0-9]' src
//   rg --glob '**/*.css' --glob '!**/global.css' 'border-radius:\s*[.0-9]' src
//   rg --glob '**/*.css' --glob '!**/global.css' 'border-(top|bottom)-(left|right)-radius:\s*[.0-9]' src

describe("AC4 / 0090: no border-radius literals in module or global CSS", () => {
  // The (?<![\w-]) lookbehind exists to avoid matching custom-property
  // *names* like `--my-radius:` (the `-` before `radius` would otherwise let
  // the property-name fragment match). A side effect is that vendor-prefixed
  // forms (`-webkit-border-radius: 6px`) are NOT matched — see the negative
  // fixture below. Current-app CSS uses no vendor-prefixed radius, so this is
  // an accepted, recorded limitation; the AC3 sweeps share it in practice.
  //
  // CSS logical-property longhands (`border-start-start-radius`, etc.) are
  // intentionally out of scope — current-app CSS uses none, and the regex
  // matches only the physical shorthand + four physical corners. If a logical
  // form is ever introduced it must be added to both the regex and a fixture.
  const BORDER_RADIUS_LITERAL_RE =
    /(?<![\w-])border-(?:(?:top|bottom)-(?:left|right)-)?radius:\s*[.0-9]/g;
  // allCss = component module CSS + any *.global.css files. The root
  // styles/global.css token file is NOT in allCss (it is not named
  // *.global.css) and is intentionally out of scope — matching the AC3
  // `!**/global.css` exclusion; its `--radius-*:` defs would not match this
  // border-radius property regex anyway.
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path}: no literal border-radius value`, () => {
      const stripped = stripComments(css);
      const hits = [...stripped.matchAll(BORDER_RADIUS_LITERAL_RE)].map(
        (m) => m[0],
      );
      expect(
        hits,
        `${path}: border-radius must use a var(--radius-*) token per ADR-0039; ` +
          `off-scale values need a new ladder step (with sign-off), not a literal`,
      ).toEqual([]);
    });
  }
});

describe("AC4 / 0090: border-radius literal regex fixtures", () => {
  const BORDER_RADIUS_LITERAL_RE =
    /(?<![\w-])border-(?:(?:top|bottom)-(?:left|right)-)?radius:\s*[.0-9]/g;

  const POSITIVE = [
    "border-radius: 6px;",
    "border-top-left-radius: 2px;",
    "border-bottom-right-radius:.5rem;",
    "border-radius: 0;",
    "border-radius: 50%;",
  ];
  const NEGATIVE = [
    "border-radius: var(--radius-6);",
    "--radius-6: 6px;",
    "--radius-0: 0;", // the bare-0 token definition must not trip the gate
    "--my-radius: 0;",
    // Vendor-prefixed *literal*: NOT matched (the lookbehind admits prefixed
    // forms). This proves the documented limitation — a var() form would be
    // skipped by the [.0-9] class regardless and so would prove nothing.
    "-webkit-border-radius: 6px;",
  ];

  for (const css of POSITIVE) {
    it(`gate regex flags: ${css}`, () => {
      expect(
        [...css.matchAll(BORDER_RADIUS_LITERAL_RE)].length,
      ).toBeGreaterThan(0);
    });
  }
  for (const css of NEGATIVE) {
    it(`gate regex skips: ${css}`, () => {
      expect([...css.matchAll(BORDER_RADIUS_LITERAL_RE)].length).toBe(0);
    });
  }
});

describe("AC5 / 0090: no irreducible reason references a migrated radius", () => {
  // The EXCEPTIONS `reason` strings are human-maintained prose the hygiene
  // gate never validates, so a decrement that left a stale radius mention
  // would pass silently. Match the bare word `radius` (not just
  // `border-radius`): the original reasons phrase radius uses both ways
  // ("scrollbar thumb radius", "mark border-radius, loadbar … radius"). After
  // 0090 migrated every border-radius to a token, no surviving irreducible
  // reason should mention radius at all.
  const RADIUS_REASON_RE = /border-radius|\bradius\b/i;
  it("no surviving irreducible reason mentions a radius", () => {
    const offenders = EXCEPTIONS.filter(
      (e) => e.kind === "irreducible" && RADIUS_REASON_RE.test(e.reason),
    ).map((e) => `${e.file} '${e.literal}': ${e.reason}`);
    expect(offenders).toEqual([]);
  });
});
