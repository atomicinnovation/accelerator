import type { DocTypeKey } from "../../../src/api/types";

/**
 * Canonical slug per doc type for the detail-route URL
 * `/library/<docType>/<slug>`.
 *
 * For physical doc types this is the fixture file's slug (filename
 * minus `.md`, after prefix/suffix stripping). For the virtual
 * `templates` key this is a template NAME — `templates` has no on-disk
 * fixture; its detail route is served by the template-summaries
 * endpoint.
 *
 * Slug values mirror the existing fixtures under
 * `server/tests/fixtures/meta/` as of Phase 3 (which adds the three
 * missing key directories). Verify each route renders by running
 * fixture-coverage.spec.ts.
 *
 * Single source of truth consumed by Phases 3, 4, 5.
 */
export const DETAIL_ROUTE_SLUGS: Record<DocTypeKey, string> = {
  decisions: "ADR-0001-example-decision",
  "work-items": "0001-first-work-item",
  plans: "2026-01-01-first-plan",
  research: "2026-01-01-first-research",
  "plan-reviews": "2026-01-01-first-plan-review-1",
  "pr-reviews": "2026-01-15-add-config-layer-review-1",
  "work-item-reviews": "example", // Phase 3 fixture 2026-05-26-example-review-1.md → slug 'example'
  validations: "2026-01-01-first-plan-validation",
  notes: "2026-01-01-first-note",
  "pr-descriptions": "42-add-config-layer",
  "design-gaps": "example-gap", // Phase 3 fixture 2026-05-26-example-gap.md → slug 'example-gap'
  "design-inventories": "example", // Phase 3 nested manifest dir 2026-05-26-example/ → slug 'example'
  templates: "plan", // template NAME (must exist in e2e fixture template set), not a fixture file slug
};

/**
 * Doc types whose detail route renders an `<article>` element. The
 * virtual `templates` detail route is served by LibraryTemplatesView,
 * which renders a tiers/preview layout (no `<article>`), so callers
 * asserting on `<article>` must exclude it.
 */
export const DETAIL_ROUTE_RENDERS_ARTICLE: Record<DocTypeKey, boolean> = {
  decisions: true,
  "work-items": true,
  plans: true,
  research: true,
  "plan-reviews": true,
  "pr-reviews": true,
  "work-item-reviews": true,
  validations: true,
  notes: true,
  "pr-descriptions": true,
  "design-gaps": true,
  "design-inventories": true,
  templates: false,
};
