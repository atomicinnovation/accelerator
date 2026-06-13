import { expect, type Page, test } from "@playwright/test";

// Pin viewport so any rem-derived values resolve at 16px root.
test.use({ viewport: { width: 1280, height: 720 } });

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
//
// Coverage gap (acknowledged in 0075's plan, Testing Strategy section):
// the spec covers one representative selector per outlier file group plus
// the MarkdownRenderer H1 value-transition case. It does not cover every
// migrated font-size site. The vitest categorical ban in
// `src/styles/migration.test.ts` is the authoritative test of compliance
// with ADR-0036; this spec is a runtime regression guard for the most
// load-bearing selectors. Inline-code-in-headings cases from the original
// plan are dropped because the committed e2e markdown fixture
// (`server/tests/fixtures/meta/plans/2026-01-01-first-plan.md`) does not
// contain inline code; the value-transition is exercised in vitest
// (no literal remains in MarkdownRenderer.module.css) and by manual
// visual inspection per the PR description's deliberate-drift screenshots.

type Case = {
  route: string;
  // Optional render precondition (e.g. open a menu) run after `goto` and
  // before the size assertion. Use it for any selector that isn't
  // immediately visible after navigation.
  setup?: (page: Page) => Promise<void>;
  selector: string;
  expected: string;
  name: string;
};

const CASES: Case[] = [
  {
    name: "MarkdownRenderer H1",
    route: "/library/plans/first-plan",
    selector: '[class*="markdown"] h1',
    expected: "28px",
  },
  {
    name: "MarkdownRenderer body p",
    route: "/library/plans/first-plan",
    selector: '[class*="markdown"] p',
    expected: "14.5px",
  },
  {
    name: "Page .eyebrow",
    route: "/lifecycle/first-plan",
    selector: '[data-slot="eyebrow"]',
    expected: "11px",
  },
  {
    name: "Page .subtitle",
    route: "/lifecycle/first-plan",
    selector: '[data-slot="subtitle"]',
    expected: "13px",
  },
  {
    name: "Sidebar .phaseHeading",
    // Sidebar renders phase headings as <h3> elements inside <nav>;
    // LibraryOverviewHub also has a `.phaseHeading` class but renders
    // <h2> elements, so anchoring on `nav h3` disambiguates without
    // depending on CSS-module class-name hashes.
    route: "/library",
    selector: 'nav h3[class*="phaseHeading"]',
    expected: "9.5px",
  },
  {
    name: "Brand .brandSub",
    route: "/library",
    selector: '[class*="brandSub"]',
    expected: "10px",
  },
  {
    name: "SortPill .menuItem",
    route: "/library/plans",
    setup: async (page) => {
      await page.getByTestId("sort-trigger").click();
      await page.getByRole("menuitem").first().waitFor();
    },
    selector: '[role="menuitem"]',
    expected: "12.5px",
  },
  {
    name: "FilterPill .option",
    route: "/library/plans",
    setup: async (page) => {
      await page.getByTestId("filter-trigger").click();
      await page.getByRole("menuitemcheckbox").first().waitFor();
    },
    selector: '[role="menuitemcheckbox"]',
    expected: "12.5px",
  },
  // EmptyState .title was previously exercised at /library/work-item-reviews,
  // which had zero fixtures. 0074 Phase 3 added fixtures for every doc type
  // (so AC #1 can iterate all 12 non-virtual types' detail pages), leaving no
  // doc type with an empty listing — the only e2e-reachable EmptyState path.
  // EmptyState .title font-size compliance remains enforced structurally by
  // the categorical px-literal ban in src/styles/migration.test.ts.
  {
    name: "LibraryTypeView .row",
    // `.row` renders as `<a>` while `.headerRow` is a `<div>`; both
    // carry `role="row"`. Anchoring on the tag disambiguates without
    // depending on CSS-module class-name hashes.
    route: "/library/plans",
    selector: 'a[role="row"]',
    expected: "13px",
  },
  {
    name: "ActivityFeed heading",
    route: "/library",
    selector: "#activity-heading",
    expected: "10.5px",
  },
  {
    name: "ActivityFeed live badge",
    route: "/library",
    selector: '[data-testid="activity-live-badge"]',
    expected: "10.5px",
  },
  // Kanban surfaces — these previously inherited the 16px UA base (no explicit
  // size) or used a token a step larger than the prototype; pinned here to the
  // prototype-matching scale tokens (--size-subtitle 13, --size-3xs-lg 10.5,
  // --size-4xs 9.5).
  {
    name: "Kanban column title",
    route: "/kanban",
    selector: "section[data-column] h2",
    expected: "13px",
  },
  {
    name: "Kanban card title",
    route: "/kanban",
    selector: ".ac-kcard__title",
    expected: "13px",
  },
  {
    name: "Kanban card id (mono meta)",
    route: "/kanban",
    selector: ".ac-kcard__id",
    expected: "10.5px",
  },
  {
    name: "Kanban kind badge",
    // /kanban fixtures carry no `kind`, so the badge only renders where a
    // sample entry has one — the DevDesignSystem Cards section's kanban demo
    // (KANBAN_ENTRY has kind: 'feature'); the first [data-tone] in DOM order.
    route: "/dev#cards",
    selector: "[data-tone]",
    expected: "9.5px",
  },
];

for (const c of CASES) {
  test(`computed font-size: ${c.name}`, async ({ page }) => {
    await page.goto(c.route);
    if (c.setup) await c.setup(page);
    const fs = await page
      .locator(c.selector)
      .first()
      .evaluate((el) => getComputedStyle(el).fontSize);
    expect(fs).toBe(c.expected);
  });
}
