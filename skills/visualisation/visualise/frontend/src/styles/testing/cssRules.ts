// Shared CSS-rule structural assertion utilities for design-system
// Vitest specs. Used by `src/styles/code-syntax.test.ts` (Phase 2)
// and `src/routes/library/LibraryTemplatesView.test.tsx` (Phase 4)
// to verify the shared code-syntax layer maps each hljs class to
// the right `--tk-*` token without regex-in-test fragility.
//
// EXACT-MATCH INVARIANT: `assertSelectorColorIs` rejects compound
// suffixes (`.hljs-meta` does not match a `.hljs-meta.doctype`
// rule) and substring siblings (`.hljs-attr` does not match a
// `.hljs-attribute` rule). This is the contract both consumer
// test files rely on — DO NOT weaken to a regex shortcut.
//
// SCOPE: flat global CSS layers (single-class selectors, optional
// compound and descendant selectors, NO nested at-rules). The
// parser throws if the input contains `@media`/`@supports`/etc.
// at top level — the helper is not equipped to recurse into
// nested rule blocks and would silently miss assertions if it
// tried.
//
// Adding a third consumer? Review both existing call sites first
// and consider whether your needs match the flat-layer scope. See
// ADR-0026 §5 for the design-token testing context.

export interface CssRule {
  selectors: string[]
  body: string
  offset: number
}

// Parses a flat CSS source into `{ selectors, body, offset }`
// records. Throws if the input contains nested at-rules — the
// parser does not recurse, so silently parsing them would lose
// assertions. Comments and string literals containing `{` or `}`
// are not handled; the current consumers (`code-syntax.global.css`
// and the templates-preview module after migration) are flat and
// contain neither, but a future consumer producing such content
// must extend the parser before using this helper.
export function parseFlatCssRules(css: string): CssRule[] {
  if (/^\s*@(?:media|supports|container|layer)\b/m.test(css)) {
    throw new Error(
      'parseFlatCssRules: input contains a top-level @-rule (e.g. @media). ' +
        'This helper does not recurse into at-rules; assertions against nested ' +
        'rules would be silently missed. Extend the parser before using.',
    )
  }
  const rules: CssRule[] = []
  let i = 0
  while (i < css.length) {
    const openBrace = css.indexOf('{', i)
    if (openBrace < 0) break
    const closeBrace = css.indexOf('}', openBrace + 1)
    if (closeBrace < 0) break
    const selectorBlock = css
      .slice(i, openBrace)
      .replace(/\/\*[\s\S]*?\*\//g, '')
      .trim()
    const body = css.slice(openBrace + 1, closeBrace)
    if (selectorBlock && !selectorBlock.startsWith('@')) {
      const selectors = selectorBlock.split(',').map((s) => s.trim())
      rules.push({ selectors, body, offset: openBrace })
    }
    i = closeBrace + 1
  }
  return rules
}

// Asserts that some rule in `css` whose comma-separated selector list
// contains EXACTLY `selector` (no compound suffix, no substring of a
// sibling) declares `color: var(--<token>)`. The `color:` match is
// anchored at a property-name boundary so `border-color:`,
// `background-color:`, `outline-color:`, etc. do NOT satisfy it.
// Throws with all matched rule bodies on failure for actionable
// diagnostics.
export function assertSelectorColorIs(
  css: string,
  selector: string,
  token: string,
): void {
  const rules = parseFlatCssRules(css)
  const matchingRules = rules.filter((r) => r.selectors.includes(selector))
  if (matchingRules.length === 0) {
    throw new Error(
      `assertSelectorColorIs: no rule declares selector "${selector}" exactly. ` +
        `Compound selectors like "${selector}.foo" do NOT satisfy this check.`,
    )
  }
  const tokenRef = `var(--${token})`
  const escapedRef = tokenRef.replace(/[()]/g, '\\$&')
  // Anchor on a property-name boundary: start-of-body, `;`, or `{`
  // so the substring `color:` inside `border-color:` does not match.
  const colourRegex = new RegExp(
    `(?:^|[;{\\s])color\\s*:\\s*${escapedRef}`,
  )
  const ok = matchingRules.some((r) => colourRegex.test(r.body))
  if (!ok) {
    throw new Error(
      `assertSelectorColorIs: selector "${selector}" found but no matching rule declares color: ${tokenRef}. ` +
        `Inspected bodies: ${JSON.stringify(matchingRules.map((r) => r.body))}`,
    )
  }
}

// Returns the source-offset of the FIRST rule whose selector list
// includes `selector`. Used by source-order assertions in
// `code-syntax.test.ts`.
export function selectorOffset(css: string, selector: string): number | null {
  const rules = parseFlatCssRules(css)
  const match = rules.find((r) => r.selectors.includes(selector))
  return match?.offset ?? null
}
