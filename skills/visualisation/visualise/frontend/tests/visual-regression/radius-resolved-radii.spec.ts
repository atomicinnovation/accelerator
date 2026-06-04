import { test, expect, type Page } from '@playwright/test'

// Pin viewport so any box-relative (`50%`) radii resolve at a fixed element
// size, mirroring typography-resolved-sizes.spec.ts.
test.use({ viewport: { width: 1280, height: 720 } })

// AC2 (0090): each migrated `border-radius` selector's computed corner radius
// must equal its pre-migration value — proving the literal→var(--radius-*)
// migration introduced zero visual change. Computed radius is read from the
// `borderTopLeftRadius` corner longhand because Chromium returns the
// `borderRadius` shorthand empty.
//
// Coverage posture (mirrors typography-resolved-sizes.spec.ts): this spec
// covers at least one reliably-mountable selector for each distinct migrated
// radius value (0, 2px, 3px, 6px, 50%) across the chrome and route surfaces.
// It is a runtime regression guard for load-bearing selectors, NOT a complete
// per-site enumeration. The authoritative completeness gate is the categorical
// BORDER_RADIUS_LITERAL_RE ban in src/styles/migration.test.ts (ADR-0039);
// token *values* are additionally guarded by the global.css↔tokens.ts radius
// parity suite in src/styles/global.test.ts. Three values are intentionally
// not asserted here and rely on those backstops:
//   - EmptyState `.card` (12px): not reachable by navigation — 0074 added
//     fixtures for every doc type, so no /library/<type> route yields an empty
//     listing to mount it.
//   - LifecycleClusterView `.timeline::before` (1px spine): a decorative
//     pseudo-element whose selector collides with `.timelineTile`; its 1px
//     value is guarded by the --radius-1 parity assertion + the categorical gate.
//   - FilterPill `.badge` (8px): renders only behind an active-filter
//     selection; 8px equals the renamed --radius-8 token, so it is covered by
//     the var-resolution test, the --radius-8 value parity, and the gate.

type Case = {
  name: string
  route: string
  // Optional render precondition (e.g. open a menu) run after `goto` and
  // before the radius read. Ends with a waitFor on the target so a selector
  // that never mounts fails loudly rather than asserting on the wrong element.
  setup?: (page: Page) => Promise<void>
  selector: string
  expected: string
  // For `50%` radii Chromium's getComputedStyle returns the literal "50%"
  // (not a used px), which directly proves percentage-derivation. The optional
  // box assertion additionally guards that the element's dimensions are
  // unchanged, so a box-size regression is still caught.
  box?: { width: number; height: number }
}

const CASES: Case[] = [
  // --- 6px / 0px: MarkdownRenderer code-block chrome (/library/plans) ---
  {
    name: 'MarkdownRenderer .codeblock wrapper (6px)',
    route: '/library/plans/first-plan',
    selector: '[class*="codeblock"]',
    expected: '6px',
  },
  {
    name: 'MarkdownRenderer .codeblock pre reset (0)',
    route: '/library/plans/first-plan',
    selector: '[class*="codeblock"] pre',
    expected: '0px',
  },
  // --- 3px: MarkdownRenderer inline-code pill (also covered by
  //     inline-code-resolved-styles.spec.ts; kept here for radius-value parity) ---
  {
    name: 'MarkdownRenderer inline-code pill (3px)',
    route: '/library/plans/first-plan',
    selector: '[class*="markdown"] p > code',
    expected: '3px',
  },
  // --- 6px: Pipeline tile + lifecycle index card (/lifecycle) ---
  {
    name: 'LifecycleIndex .card (6px)',
    route: '/lifecycle',
    selector: '[class*="cardList"] > li',
    expected: '6px',
  },
  // --- 6px: LifecycleClusterView panel + timeline tile (/lifecycle/<slug>) ---
  {
    name: 'LifecycleClusterView .pipelinePanel (6px)',
    route: '/lifecycle/first-plan',
    selector: '[class*="pipelinePanel"]',
    expected: '6px',
  },
  {
    name: 'LifecycleClusterView .timelineTile (6px)',
    route: '/lifecycle/first-plan',
    selector: '[class*="timelineTile"]',
    expected: '6px',
  },
  // --- 6px: LibraryOverviewHub card (/library) ---
  {
    name: 'LibraryOverviewHub .card (6px)',
    route: '/library',
    selector: '[class*="LibraryOverviewHub"] [class*="card"], main [class*="card"]',
    expected: '6px',
  },
  // --- 2px: Breadcrumbs focus-ring (keyboard focus needed) ---
  {
    name: 'Breadcrumbs .link:focus-visible (2px)',
    route: '/library/plans/first-plan',
    setup: async (page) => {
      // Tab to the first breadcrumb link so :focus-visible (keyboard heuristic)
      // engages, then confirm it mounted.
      const link = page.locator('nav[aria-label="Breadcrumb"] a').first()
      await link.waitFor()
      await page.keyboard.press('Tab')
    },
    selector: 'nav[aria-label="Breadcrumb"] a',
    expected: '2px',
  },
  // --- 2px / 3px / 8px: FilterPill (open the menu first) ---
  {
    name: 'FilterPill .option (3px)',
    route: '/library/plans',
    setup: async (page) => {
      await page.getByTestId('filter-trigger').first().click()
      await page.getByRole('menuitemcheckbox').first().waitFor()
    },
    selector: '[role="menuitemcheckbox"]',
    expected: '3px',
  },
  {
    name: 'FilterPill .checkbox (2px)',
    route: '/library/plans',
    setup: async (page) => {
      await page.getByTestId('filter-trigger').first().click()
      await page.locator('[class*="checkbox"]').first().waitFor()
    },
    selector: '[class*="checkbox"]',
    expected: '2px',
  },
  // --- 50%: PipelineMini status dot (/kanban) ---
  {
    name: 'PipelineMini .dot (50%)',
    route: '/kanban',
    selector: '.ac-stagedots__dot',
    expected: '50%',
    box: { width: 8, height: 8 },
  },
  // --- 50%: LibraryTemplatesIndex tier-pill bullet (/library/templates) ---
  {
    name: 'LibraryTemplatesIndex .tierPillBullet (50%)',
    route: '/library/templates',
    selector: '[class*="tierPillBullet"]',
    expected: '50%',
    box: { width: 5, height: 5 },
  },
]

for (const c of CASES) {
  test(`computed border-radius: ${c.name}`, async ({ page }) => {
    await page.goto(c.route)
    if (c.setup) await c.setup(page)
    const el = page.locator(c.selector).first()
    await el.waitFor()
    const measured = await el.evaluate((node) => {
      const cs = getComputedStyle(node)
      return {
        radius: cs.borderTopLeftRadius,
        width: (node as HTMLElement).offsetWidth,
        height: (node as HTMLElement).offsetHeight,
      }
    })
    expect(measured.radius).toBe(c.expected)
    if (c.box) {
      expect(measured.width).toBe(c.box.width)
      expect(measured.height).toBe(c.box.height)
    }
  })
}
