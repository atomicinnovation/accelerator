import { test, expect, type Page } from '@playwright/test'
import { CODE_SYNTAX_TOKENS } from '../../src/styles/tokens'
import { hexToRgbString } from '../../src/styles/contrast'

// Story 0076 AC2/AC3 — live-cascade verification that the shared
// `code-syntax.global.css` layer resolves every required mapping to
// the prototype palette in both light and dark themes (the palette
// is theme-invariant by design). Empirically pinned to classes that
// rehype-highlight actually emits for the showcase fixtures; see the
// `FIXTURES` array in `src/routes/code-syntax-showcase/CodeSyntaxShowcase.tsx`.

interface ThemeCase {
  readonly name: 'light' | 'dark'
  readonly setup: (page: Page) => Promise<void>
}

const THEMES: ReadonlyArray<ThemeCase> = [
  { name: 'light', setup: async () => {} },
  {
    name: 'dark',
    setup: async (page: Page) => {
      await page.evaluate(() => {
        document.documentElement.dataset.theme = 'dark'
      })
      await page.waitForFunction(
        () => document.documentElement.dataset.theme === 'dark',
      )
    },
  },
]

// (selector, language-cell, token) — each row is confirmed to be
// emitted by rehype-highlight against the showcase fixture text. The
// theme loop below executes the same assertions in both themes.
const AC2_CASES: ReadonlyArray<{
  selector: string
  cell: string
  token: keyof typeof CODE_SYNTAX_TOKENS
}> = [
  { selector: '.hljs-keyword',         cell: 'python',     token: 'tk-kw' },
  { selector: '.hljs-string',          cell: 'python',     token: 'tk-str' },
  { selector: '.hljs-number',          cell: 'python',     token: 'tk-num' },
  { selector: '.hljs-literal',         cell: 'yaml',       token: 'tk-lit' },
  { selector: '.hljs-function',        cell: 'typescript', token: 'tk-fn' },
  { selector: '.hljs-attr',            cell: 'yaml',       token: 'tk-attr' },
  { selector: '.hljs-meta',            cell: 'html',       token: 'tk-deco' },
  { selector: '.hljs-built_in',        cell: 'python',     token: 'tk-bn' },
  { selector: '.hljs-property',        cell: 'typescript', token: 'tk-prop' },
  { selector: '.hljs-selector-class',  cell: 'css',        token: 'tk-sel' },
  { selector: '.hljs-selector-id',     cell: 'css',        token: 'tk-sel' },
  { selector: '.hljs-selector-pseudo', cell: 'css',        token: 'tk-sel' },
  { selector: '.hljs-tag',             cell: 'html',       token: 'tk-tag' },
  { selector: '.hljs-name',            cell: 'html',       token: 'tk-tag' },
  { selector: '.hljs-section',         cell: 'markdown',   token: 'tk-header' },
  { selector: '.hljs-link',            cell: 'markdown',   token: 'tk-anchor' },
  { selector: '.hljs-punctuation',     cell: 'json',       token: 'tk-pun' },
  { selector: '.hljs-comment',         cell: 'python',     token: 'tk-com' },
  { selector: '.hljs-bullet',          cell: 'markdown',   token: 'tk-com' },
  { selector: '.hljs-attribute',       cell: 'css',        token: 'tk-attr' },
]

for (const theme of THEMES) {
  test.describe(`AC2 — resolved colours (${theme.name} theme)`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto('/code-syntax-showcase')
      await theme.setup(page)
    })

    for (const { selector, cell, token } of AC2_CASES) {
      test(`${selector} in ${cell} cell resolves to --${token}`, async ({
        page,
      }) => {
        const locator = page
          .locator(`[data-testid="code-syntax-cell-${cell}"] ${selector}`)
          .first()
        // Fail loud if the class is not emitted at all — silent
        // "colour is undefined" is the failure mode we want to avoid.
        await expect(locator).toBeVisible()
        const colour = await locator.evaluate(
          (el) => getComputedStyle(el).color,
        )
        expect(colour).toBe(hexToRgbString(CODE_SYNTAX_TOKENS[token]))
      })
    }
  })

  test.describe(`AC3 — diff overrides (${theme.name} theme)`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto('/code-syntax-showcase')
      await theme.setup(page)
    })

    test('.hljs-meta inside .language-diff resolves to --tk-dhdr', async ({
      page,
    }) => {
      const locator = page
        .locator('[data-testid="code-syntax-cell-diff"] .hljs-meta')
        .first()
      await expect(locator).toBeVisible()
      const colour = await locator.evaluate(
        (el) => getComputedStyle(el).color,
      )
      expect(colour).toBe(hexToRgbString(CODE_SYNTAX_TOKENS['tk-dhdr']))
    })

    test('.hljs-comment inside .language-diff resolves to --tk-dhunk', async ({
      page,
    }) => {
      const locator = page
        .locator('[data-testid="code-syntax-cell-diff"] .hljs-comment')
        .first()
      await expect(locator).toBeVisible()
      const colour = await locator.evaluate(
        (el) => getComputedStyle(el).color,
      )
      expect(colour).toBe(hexToRgbString(CODE_SYNTAX_TOKENS['tk-dhunk']))
    })

    test('.hljs-addition resolves to --tk-dadd', async ({ page }) => {
      const locator = page
        .locator('[data-testid="code-syntax-cell-diff"] .hljs-addition')
        .first()
      await expect(locator).toBeVisible()
      const colour = await locator.evaluate(
        (el) => getComputedStyle(el).color,
      )
      expect(colour).toBe(hexToRgbString(CODE_SYNTAX_TOKENS['tk-dadd']))
    })

    test('.hljs-deletion resolves to --tk-ddel', async ({ page }) => {
      const locator = page
        .locator('[data-testid="code-syntax-cell-diff"] .hljs-deletion')
        .first()
      await expect(locator).toBeVisible()
      const colour = await locator.evaluate(
        (el) => getComputedStyle(el).color,
      )
      expect(colour).toBe(hexToRgbString(CODE_SYNTAX_TOKENS['tk-ddel']))
    })

    test('.hljs-meta OUTSIDE .language-diff still resolves to --tk-deco', async ({
      page,
    }) => {
      // Paired with the override test: proves the diff override is
      // scoped to .language-diff and hasn't accidentally become global.
      const locator = page
        .locator('[data-testid="code-syntax-cell-html"] .hljs-meta')
        .first()
      await expect(locator).toBeVisible()
      const colour = await locator.evaluate(
        (el) => getComputedStyle(el).color,
      )
      expect(colour).toBe(hexToRgbString(CODE_SYNTAX_TOKENS['tk-deco']))
    })
  })
}
