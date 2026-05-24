---
date: 2026-05-26T14:59:05+01:00
author: Toby Clemson
git_commit: 0f16c47c1f7cadb49bb69b320d94c7b36565c4d9
branch: HEAD
repository: accelerator
topic: "0088 Markdown Body Width Harmonisation — implementation surface area"
tags: [research, codebase, markdown-renderer, design-tokens, layout]
status: complete
last_updated: 2026-05-26
last_updated_by: Toby Clemson
---

# Research: 0088 Markdown Body Width Harmonisation

**Date**: 2026-05-26T14:59:05+01:00
**Author**: Toby Clemson
**Git Commit**: 0f16c47c1f7cadb49bb69b320d94c7b36565c4d9
**Branch**: HEAD
**Repository**: accelerator

## Research Question

What is the precise codebase surface area for work item
`0088-markdown-body-width-harmonisation` — i.e. introducing
`--ac-content-max-width-prose: 72ch`, replacing the literal
`max-width: 720px` in `MarkdownRenderer.module.css` with
`min(var(--ac-content-max-width-prose), 100%)`, pinning
`.markdown { font-size: var(--size-body); }`, removing the
corresponding `migration.test.ts` exception, and refreshing the
Playwright baselines the story names?

## Summary

The mechanical edits are tight and isolated: three token-system
files (`global.css`, `tokens.ts`, `migration.test.ts`) plus one
CSS module (`MarkdownRenderer.module.css`). Each edit lands on a
known line with clear context, and the `72ch` value sails through
the existing migration-test guards (which whitelist only `px`,
`rem`, `em` literals — `ch` is invisible to them).

Two findings warrant attention before implementation begins:

1. **The story's baseline-refresh acceptance criterion is
   misspecified.** Neither `/library/plans/first-plan`
   (`library-doc-view`) nor `/code-syntax-showcase` has a
   `toHaveScreenshot` baseline in the visual-regression suite —
   both are exercised only by *resolved-style* specs
   (`typography-resolved-sizes.spec.ts`,
   `code-block-resolved-colours.spec.ts`). There are no PNGs to
   refresh for those two specs. The "library-light"/"library-dark"
   PNGs in `tokens.spec.ts` cover the library hub list, not the
   detail page. This needs an AC fix-up (or a deliberate decision
   to add new screenshot specs for those routes).
2. **`typography-resolved-sizes.spec.ts` is the at-risk test, not
   `migration.test.ts`.** Its `MarkdownRenderer H1` case asserts an
   exact `28px` resolved size on `[class*="markdown"] h1`, and the
   story's change adds an explicit `font-size: var(--size-body)`
   to the `.markdown` parent. H1 has its own explicit
   `var(--size-h3)` (28px) override, so the case should still
   pass — but no other case in the spec covers plain markdown
   `<p>` text, so the new body size is *unobserved* by the suite.
   That is a coverage gap the story already implicitly accepts
   (the "visual delta" is bound by Playwright PNG baselines that
   don't exist).

## Detailed Findings

### 1. MarkdownRenderer.module.css — the surface to mutate

File: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css`

Current `.markdown` rule (lines 1–5):

```css
.markdown {
  max-width: 720px;
  line-height: 1.6;
  color: var(--ac-fg-strong);
}
```

- Line 2: literal `max-width: 720px` — the only literal width in
  the file. Target of the replacement.
- **No `font-size` declaration on `.markdown` currently.** Body
  text inherits from the document root. The story's
  `font-size: var(--size-body)` is therefore a *new* declaration,
  not a replacement.
- Every other `font-size` in the file already uses a `--size-*`
  token (`MarkdownRenderer.module.css:9–53`): h1 → `--size-h3`,
  h2 → `--size-lg`, h3 → `--size-sm`, pre → `--size-xs`,
  codeblockLang → `--size-3xs-lg`, inline code → `--size-xs`.
  → The "no literal px font-size" AC currently passes; adding
  `var(--size-body)` keeps it passing.
- Other pixel literals exist (1px borders, 4px/6px radii) but
  they are non-width / non-font-size and out of this story's
  scope.

Outer element wired to `.markdown` at `MarkdownRenderer.tsx:77`
(`<div className={styles.markdown}>`). `1ch` is the advance of
the `0` glyph in *this* element's own font — confirms the
story's note that the cap must live on `.markdown` itself.

### 2. Design tokens — where to add `--ac-content-max-width-prose`

File: `skills/visualisation/visualise/frontend/src/styles/global.css`

The Layout block lives in the single `:root` (lines 195–198):

```css
/* Layout */
--ac-topbar-h: 48px;
--ac-content-max-width:        1200px;
--ac-content-max-width-narrow: 600px;
```

→ Insert `--ac-content-max-width-prose: 72ch;` immediately after
line 198 (or between `-narrow` and the closing block at line 199)
to keep the `--ac-content-max-width{,-narrow,-prose}` family
contiguous. The block is theme-invariant (no dark-mode mirror).

Adjacent context:

- `--size-body: 20px;` declared at `global.css:151` inside the
  size scale (lines 145–162). The scale's block comment
  (lines 133–144) enumerates `--size-body` as part of the
  integer-px tier-name scale; the "prose tier" framing in the
  story is the inline rationale the author has chosen — there is
  no existing comment naming it "prose", so no comment edit is
  required.

File: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`

`LAYOUT_TOKENS` at lines 189–193:

```ts
export const LAYOUT_TOKENS = {
  'ac-topbar-h': '48px',
  'ac-content-max-width': '1200px',
  'ac-content-max-width-narrow': '600px',
} as const
```

→ Add `'ac-content-max-width-prose': '72ch',` between line 192
and line 193 to mirror `global.css`. The `LayoutToken` alias at
`tokens.ts:321` derives from `keyof typeof LAYOUT_TOKENS` so the
new key picks up type coverage automatically.

### 3. migration.test.ts — the exception to remove

File: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`

The `EXCEPTIONS` array (lines 48–226) holds entries shaped per
the `Exception` type at line 46:

```ts
type Exception = { file: string; literal: string; count: number; reason: string }
```

The MarkdownRenderer group banner is at line 64. The target entry
is `migration.test.ts:70`, verbatim:

```ts
{ file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '720px', count: 1, kind: 'irreducible', reason: 'prose max-width — no token equivalent' },
```

→ Delete this line. The `EXCEPTIONS hygiene` block (lines
401–432) will fail the build if a declared exception no longer
appears in the file, so removing the literal *and* the exception
together is the only consistent move.

`72ch` is invisible to the unit guards:

- `PX_REM_EM_RE` at `migration.test.ts:31` matches only
  `\d+(?:\.\d+)?(?:px|rem|em)\b` — `ch` is not in the
  alternation, so `72ch` neither raises an "unmigrated literal"
  alarm nor needs a new exception entry.
- `HEX_RE` at line 30 is colour-only.
- The "var(--NAME) references resolve" assertion (lines 323–355)
  builds its declared-set from `Object.keys(LAYOUT_TOKENS)` at
  line 332 → adding the key to `LAYOUT_TOKENS` is sufficient for
  `var(--ac-content-max-width-prose)` references in CSS to
  resolve.

No other MarkdownRenderer exceptions touch widths or font-sizes
(`migration.test.ts:65–69` cover 1px borders, 0.4rem/0.1rem
off-scale paddings, 4px blockquote, 6px radii — all unaffected).

### 4. Consumers of MarkdownRenderer — width budget at each surface

`MarkdownRenderer` has exactly two consumers in production
routes plus one indirect reference:

- **`LibraryDocView`** at
  `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:144–146` —
  renders `<div className={styles.body}>` inside the article
  grid. The grid lives in `LibraryDocView.module.css:1–6`:

  ```css
  .article {
    display: grid;
    grid-template-areas: "body aside";
    grid-template-columns: 1fr 260px;
    gap: var(--sp-5) var(--sp-6);
  }
  .bodyColumn { grid-area: body; min-width: 0; }
  .body { margin-top: var(--sp-4); }
  ```

  - No media query collapses the grid at narrow viewports — it
    stays `1fr 260px` at any width. `min-width: 0` on
    `.bodyColumn` is what lets the `1fr` track shrink below its
    intrinsic size at narrow viewports.
  - Page wrapper: `Page.module.css` enforces
    `max-width: var(--ac-content-max-width)` (1200px) with
    `padding: 0 var(--sp-7)` (40px each side), per the page test
    "1120px content at 1200px max-width" (`Page.test.tsx:60`).
  - Body-column horizontal budget:
    - At 1440px viewport: page = 1200px; inner = 1200 − 80 (h-pad)
      = 1120; body = 1120 − 260 (aside) − 32 (`--sp-6` gap) =
      ~828px. The new `72ch ≈ 720px` cap (at the explicit 20px
      body) is *narrower* than 828px → cap applies, prose does
      not bleed into the aside. ✅ Matches story AC.
    - At 800px viewport: page = 800px (fluid below 1200); inner =
      800 − 80 = 720; body = 720 − 260 − 32 = ~428px. `72ch ≈
      720px` > 428px → the `100%` branch of `min()` applies, body
      = parent column width (~428px). ✅ Matches story AC.

- **`CodeSyntaxShowcase`** at
  `skills/visualisation/visualise/frontend/src/routes/code-syntax-showcase/CodeSyntaxShowcase.tsx:79–90` —
  dev-only Playwright fixture. The `<main>` and per-language
  `<section>` wrappers carry no `className` and no width
  constraint, so the `.markdown` rule's `max-width` is the *only*
  width cap active here. Replacing 720px with `min(72ch, 100%)`
  changes the cap from a px literal to ~720px at 20px body — a
  near-zero visible delta, modulo the new explicit body size.

- **`FrontmatterTable`** at
  `skills/visualisation/visualise/frontend/src/components/FrontmatterTable/FrontmatterTable.tsx`
  also imports `MarkdownRenderer` for cell-level rendering. The
  story does not name it as in-scope, but the change applies
  uniformly. Worth confirming during implementation that the
  72ch cap inside table cells does not produce surprising
  layouts (cells are typically much narrower than 720px, so the
  `100%` branch will dominate — likely no visible change, but
  verify).

The story's listed-out-of-scope monospace surface is at
`skills/visualisation/visualise/frontend/src/routes/library/template-highlight.tsx`,
used by `LibraryTemplatesView.tsx`. Confirmed: it does not import
`MarkdownRenderer` and is unaffected.

### 5. Playwright visual-regression — what actually exists

Directory: `skills/visualisation/visualise/frontend/tests/visual-regression/`

Specs that touch the markdown-renderer surface:

- `typography-resolved-sizes.spec.ts` — uses `viewport: 1280×720`
  (line 4). The `MarkdownRenderer H1` case at lines 43–48
  navigates to `/library/plans/first-plan` and asserts
  `[class*="markdown"] h1` resolves to `'28px'` via exact-string
  equality at line 139. **This case is unaffected by the story's
  change**: H1 keeps its explicit `var(--size-h3)` (28px)
  override; the new `.markdown { font-size: var(--size-body) }`
  only changes the cascading default for elements without their
  own size declaration (`<p>`, `<li>`, `<td>`, `<blockquote>`).
- `code-block-resolved-colours.spec.ts` — navigates to
  `/code-syntax-showcase`, asserts colours only
  (`getComputedStyle(el).color` / `.backgroundColor` /
  `.borderTopColor`, lines 79–200). No `font-size` and no
  screenshot baseline.

Specs with actual `toHaveScreenshot` PNGs (under
`tests/visual-regression/__screenshots__/*-snapshots/`):

- `tokens.spec.ts` — viewport 1440×900 (line 18). The library
  routes baselined here are the hub (`/library`) and the plans
  list (`/library/plans`); they appear as `library-light.png` /
  `library-dark.png`. **Neither baseline renders the markdown
  body**, so they will not change.
- `glyph-showcase.spec.ts` — viewport 1024×768; no markdown.
- `chip-showcase.spec.ts` — viewport 1024×768; no markdown.

**Conclusion:** Neither `/library/plans/first-plan` nor
`/code-syntax-showcase` has a screenshot baseline anywhere. The
story's AC "the commit includes refreshed baseline images for at
least the library-doc-view and code-syntax-showcase Playwright
specs" cannot be satisfied as written, because no such baselines
exist. Options for the plan to pick from:

1. Soften the AC to "all existing Playwright baselines and
   resolved-style assertions continue to pass".
2. Add new screenshot specs that render
   `/library/plans/first-plan` and `/code-syntax-showcase` (which
   *would* change visibly under the new body size) and ship the
   baselines as part of this story. The PNGs would canonicalise
   the 20px body.

Refresh command (no dedicated npm script):
`npm run test:e2e -- --project=visual-regression --update-snapshots`
(`playwright.config.ts:24–29`; `package.json:17` has only
`"test:e2e": "playwright test"`).

### 6. Confirmation of story-cited line numbers

- `MarkdownRenderer.module.css:2` — `max-width: 720px` ✅
- `migration.test.ts:70` — the `'720px'` exception entry ✅
- `--ac-content-max-width{,-narrow}` declared in the same `:root`
  Layout block in `global.css` (lines 197–198) ✅
- Same family exported from `LAYOUT_TOKENS` in `tokens.ts`
  (lines 191–192) ✅
- `--size-body: 20px` at `global.css:151` ✅
- 1200px page wrapper, `1fr | 260px` grid at desktop ✅
  (Note: the grid does **not** collapse at narrow viewports,
  contrary to the casual "and fluid below" phrasing in the
  story's Technical Notes — the columns stay 1fr/260px, only
  the page padding/gap behave fluidly.)

## Code References

- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:1-5` — `.markdown` rule (literal `720px` on line 2)
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx:77` — outer `<div className={styles.markdown}>`
- `skills/visualisation/visualise/frontend/src/styles/global.css:151` — `--size-body: 20px`
- `skills/visualisation/visualise/frontend/src/styles/global.css:195-198` — Layout `:root` block (insertion point for `--ac-content-max-width-prose`)
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:189-193` — `LAYOUT_TOKENS` (mirror insertion point)
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:321` — `LayoutToken` alias (no-op update — auto-derives)
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:46-48` — `Exception` type and `EXCEPTIONS` typing
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:64-70` — MarkdownRenderer exception group; delete line 70
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:30-31` — unit-style regexes (confirm `ch` is unaffected)
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:323-355` — `var(--NAME) references resolve` (auto-pickup of new key via `Object.keys(LAYOUT_TOKENS)` at line 332)
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:401-432` — `EXCEPTIONS hygiene` (forces removal of exception when literal is gone)
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:144-146` — `MarkdownRenderer` invocation
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.module.css:1-9` — article grid (`1fr 260px`)
- `skills/visualisation/visualise/frontend/src/routes/code-syntax-showcase/CodeSyntaxShowcase.tsx:79-90` — dev-only fixture
- `skills/visualisation/visualise/frontend/src/components/FrontmatterTable/FrontmatterTable.tsx` — third consumer (out of story scope but inherits the change)
- `skills/visualisation/visualise/frontend/src/components/Page/Page.module.css:1-9` — `max-width: var(--ac-content-max-width)` page wrapper
- `skills/visualisation/visualise/frontend/tests/visual-regression/typography-resolved-sizes.spec.ts:43-48` — H1 resolved-size assertion (`/library/plans/first-plan`)
- `skills/visualisation/visualise/frontend/tests/visual-regression/code-block-resolved-colours.spec.ts:62-206` — `/code-syntax-showcase` colour assertions (no font-size)
- `skills/visualisation/visualise/frontend/tests/visual-regression/tokens.spec.ts:9-18` — ROUTES list and 1440px viewport (no detail-page baseline)
- `skills/visualisation/visualise/frontend/playwright.config.ts:24-29` — `visual-regression` project and `snapshotDir`
- `skills/visualisation/visualise/frontend/package.json:17` — `"test:e2e": "playwright test"`

## Architecture Insights

- **Token surface is a strict two-mirror pattern.** Every CSS
  custom property in `global.css` has a literal mirror entry in
  `tokens.ts` under a topic-named `*_TOKENS` const, and the
  migration test verifies both surfaces. Adding a token = two
  one-line edits in the same topic block in each file.
- **Migration test is unit-aware but only for `px|rem|em`.** The
  `ch` unit slides through cleanly. This is consistent with
  ADR-0036 (named in the size-scale comment at `global.css:133`)
  and means no new exception entries are needed for `72ch`.
- **`72ch` lives on `.markdown` for a reason.** The CSS `ch`
  unit measures against the element's *own* computed font, so
  the cap and the body `font-size` must be on the same element
  to be meaningful. Splitting them across cascade layers would
  silently couple the measure to the inherited document-root
  size — which is exactly the bug the current literal-720px
  approach has. The story's "ship them together" decision is
  the only correct sequencing here.
- **Width tokens have semantic suffixes, not numeric ones.** The
  existing family is `*{,-narrow}` — semantic role, not size.
  The new `*-prose` continues that convention (rather than a
  numeric `*-720` / `*-72ch`), so future width caps slot into
  the same naming axis.
- **Visual-regression coverage is gappy on the markdown-body
  surface.** No PNG baselines target the long-form prose layout
  on `/library/plans/first-plan` or `/code-syntax-showcase`.
  This is an observable, structural gap — the story can either
  accept it or fill it.

## Historical Context

- `meta/work/0033-design-token-system.md` (status: done) —
  established the `--size-*` scale, the `--ac-content-max-width{,-narrow}`
  family, and the two-mirror token discipline this story builds
  on.
- `meta/work/0075-typography-size-scale-consumption.md` (status:
  done) — added the vitest font-size enforcement layer in
  `migration.test.ts` that the story's "no literal px font-size"
  AC depends on. Confirmed alive in the current commit (most
  recent merged: `77dcc312c Land vitest font-size enforcement
  and mark plan/ACs complete`).
- `meta/work/0076-code-block-syntax-highlight-palette.md`
  (status: done) — recent work on the same
  `MarkdownRenderer.module.css` file. Inspection of the current
  file shows token-based colour rules (`var(--ac-fg-strong)`,
  `var(--tk-*)`) already landed; no rebase risk against 0076 in
  the current commit.
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — original source identifying the markdown-width drift and
  feeding into this story.

## Related Research

- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — the design-gap analysis that originated the story.

## Open Questions

- **AC re-spec for Playwright baselines.** The story's baseline-
  refresh AC names two specs that have no screenshot baselines.
  Should the plan (a) soften the AC to "existing visual and
  resolved-style suites continue to pass", or (b) introduce new
  screenshot specs for `/library/plans/first-plan` and
  `/code-syntax-showcase` to lock in the new body size?
- **`FrontmatterTable` cell-level markdown.** The story is
  silent on this third consumer. Confirm it is intentionally
  out of scope or should be re-tested as part of the change
  (the cap will almost certainly resolve to the `100%` branch
  inside narrow cells, so the change is most likely a no-op
  there).
- **Body-text coverage in `typography-resolved-sizes.spec.ts`.**
  Should the spec gain a `MarkdownRenderer body p` case at 20px
  to make the new `var(--size-body)` declaration observable in
  the regression suite? Without it, the body-size change is
  silent to CI.
- **Narrow-viewport grid behaviour.** `LibraryDocView` keeps
  `1fr 260px` at any viewport (no media-query collapse). The
  story's 800px AC implicitly assumes this; it works, but a
  future stacking change to the grid would shift the budget the
  `min()` resolves against.
