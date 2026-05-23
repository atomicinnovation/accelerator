import { test, expect, type Page } from '@playwright/test'

// Pin viewport so any rem-derived values resolve at 16px root.
test.use({ viewport: { width: 1280, height: 720 } })

// The `--size-*` scale is theme-invariant in this codebase — no
// `data-theme="dark"` overrides apply to font-size tokens — so this spec
// only exercises the default theme. Contrast with
// `chip-resolved-colours.spec.ts`, which loops both themes because chip
// colours diverge by theme.
//
// Expected px values are derived from the tokens declared in
// `src/styles/global.css` (and mirrored in `src/styles/tokens.ts`). If
// a future legitimate token-scale tweak fires this spec, either re-derive
// expected values from `TYPOGRAPHY_TOKENS` in the spec setup or update the
// hardcoded value with the token change in the same PR.

type Case = {
  route: string
  // Optional render precondition (e.g. open a menu) run after `goto` and
  // before the size assertion. Use it for any selector that isn't
  // immediately visible after navigation.
  setup?: (page: Page) => Promise<void>
  selector: string
  expected: string
  name: string
}

// `/library/plans` is a stable LibraryTypeView route used elsewhere in
// the visual-regression suite (see tokens.spec.ts). Other routes used
// here (`/library`, `/kanban`, `/lifecycle/first-plan`) are similarly
// stable. MarkdownRenderer cases route through a known plan document.
const MD_DOC_ROUTE = '/library/plans/2026-05-23-0075-typography-size-scale-consumption'

const CASES: Case[] = [
  {
    name: 'MarkdownRenderer H1',
    route: MD_DOC_ROUTE,
    selector: '[class*="markdown"] h1',
    expected: '28px',
  },
  {
    name: 'MarkdownRenderer inline code in body',
    route: MD_DOC_ROUTE,
    selector: '[class*="markdown"] p code',
    expected: '14px',
  },
  {
    name: 'MarkdownRenderer inline code inside H2 (deliberate-drift regression)',
    route: MD_DOC_ROUTE,
    selector: '[class*="markdown"] h2 code',
    expected: '14px',
  },
  {
    name: 'Page .eyebrow',
    route: '/lifecycle/first-plan',
    selector: '[data-slot="eyebrow"]',
    expected: '11px',
  },
  {
    name: 'Page .subtitle',
    route: '/lifecycle/first-plan',
    selector: '[data-slot="subtitle"]',
    expected: '13px',
  },
  {
    name: 'Sidebar .phaseHeading',
    route: '/library',
    selector: 'aside [class*="phaseHeading"]',
    expected: '9.5px',
  },
  {
    name: 'Brand .brandSub',
    route: '/library',
    selector: '[class*="brandSub"]',
    expected: '10px',
  },
  {
    name: 'SortPill .menuItem',
    route: '/library/plans',
    setup: async (page) => {
      await page.getByRole('button', { name: /sort/i }).first().click()
      await page.locator('[class*="menuItem"]').first().waitFor()
    },
    selector: '[class*="menuItem"]',
    expected: '12.5px',
  },
  {
    name: 'FilterPill .option',
    route: '/library/plans',
    setup: async (page) => {
      await page.getByRole('button', { name: /filter/i }).first().click()
      await page.locator('[class*="option"]').first().waitFor()
    },
    selector: '[class*="option"]',
    expected: '12.5px',
  },
  {
    name: 'EmptyState .title',
    // `/library/<unknown-type>` renders EmptyState; pick a slug guaranteed
    // not to match a real type.
    route: '/library/__no_such_type__',
    selector: '[class*="title"]',
    expected: '22px',
  },
  {
    name: 'LibraryTypeView .row',
    route: '/library/plans',
    selector: '[class*="row"][role="row"]',
    expected: '13px',
  },
  {
    name: 'ActivityFeed heading',
    route: '/library',
    selector: '#activity-heading',
    expected: '10.5px',
  },
  {
    name: 'ActivityFeed live badge',
    route: '/library',
    selector: '[data-testid="activity-live-badge"]',
    expected: '10.5px',
  },
]

for (const c of CASES) {
  test(`computed font-size: ${c.name}`, async ({ page }) => {
    await page.goto(c.route)
    if (c.setup) await c.setup(page)
    const fs = await page
      .locator(c.selector)
      .first()
      .evaluate((el) => getComputedStyle(el).fontSize)
    expect(fs).toBe(c.expected)
  })
}
