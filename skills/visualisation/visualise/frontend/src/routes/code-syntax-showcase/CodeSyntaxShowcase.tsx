// Dev-only Playwright fixture surface for resolved-colour
// assertions in tests/visual-regression/code-block-resolved-colours.spec.ts.
// NOT part of the design-system index — story 0083 owns the
// documented showcase. Do not link from DevDesignSystem.
// See meta/work/0076-code-block-syntax-highlight-palette.md for
// provenance.
import { MarkdownRenderer } from '../../components/MarkdownRenderer/MarkdownRenderer'

// Each fixture's `spans` field names the hljs classes the
// Playwright spec asserts on. Do not remove a class' triggering
// text without updating the spec.
export interface CodeSyntaxFixture {
  lang: string
  code: string
  spans: ReadonlyArray<string>
}

export const FIXTURES: ReadonlyArray<CodeSyntaxFixture> = [
  {
    lang: 'python',
    code:
      'def foo(x: int) -> int:\n    print("hi")\n    return x + 42  # comment\n',
    spans: [
      'hljs-keyword',
      'hljs-string',
      'hljs-number',
      'hljs-comment',
      'hljs-built_in',
      'hljs-function',
      'hljs-title.function_',
    ],
  },
  {
    lang: 'typescript',
    code:
      "const greet = (name: string): string => `Hi ${name}`;\nconst obj = { prop: 1 };\nobj.prop = 2;\n",
    spans: [
      'hljs-keyword',
      'hljs-type',
      'hljs-variable',
      'hljs-template-variable',
      'hljs-property',
      'hljs-punctuation',
    ],
  },
  {
    lang: 'yaml',
    code: 'title: "Example"\ncount: 7\nactive: true\n',
    spans: ['hljs-attr', 'hljs-string', 'hljs-number', 'hljs-literal'],
  },
  {
    lang: 'json',
    code: '{\n  "key": "value",\n  "n": 42\n}\n',
    spans: ['hljs-attr', 'hljs-string', 'hljs-number'],
  },
  {
    lang: 'css',
    code:
      '.cls { color: red; }\n#id { background: blue; }\na:hover { opacity: 0.5; }\n',
    spans: ['hljs-selector-class', 'hljs-selector-id', 'hljs-selector-pseudo'],
  },
  {
    lang: 'html',
    code: '<!DOCTYPE html>\n<div class="x" data-foo="y">hi</div>\n',
    spans: ['hljs-tag', 'hljs-name', 'hljs-attr', 'hljs-meta'],
  },
  {
    lang: 'diff',
    code: 'diff --git a/x b/x\n@@ -1,1 +1,1 @@\n-old\n+new\n',
    spans: ['hljs-meta', 'hljs-comment', 'hljs-addition', 'hljs-deletion'],
  },
  {
    lang: 'markdown',
    code: '# Heading\n\n- item\n[link](http://x)\n',
    spans: ['hljs-section', 'hljs-bullet', 'hljs-link', 'hljs-symbol'],
  },
]

export function CodeSyntaxShowcase() {
  return (
    <main data-testid="code-syntax-showcase">
      {FIXTURES.map(({ lang, code }) => (
        <section key={lang} data-testid={`code-syntax-cell-${lang}`}>
          <h2>{lang}</h2>
          <MarkdownRenderer content={'```' + lang + '\n' + code + '```\n'} />
        </section>
      ))}
    </main>
  )
}
