import { describe, it, expect } from 'vitest'
import { buildEditorHref, encodePath, EDITOR_PRESETS } from './editor-link'

const ABS = '/Users/x/a b.md'
const REL = 'sub dir/a.md'

function href(editor: string, opts?: { abs?: string; rel?: string; project?: string }) {
  return buildEditorHref({
    editor,
    editorProject: opts?.project ?? 'myrepo',
    absPath: opts?.abs ?? ABS,
    relPath: opts?.rel ?? REL,
  })
}

describe('encodePath', () => {
  it('percent-encodes segments but preserves / separators', () => {
    expect(encodePath('/a b/c.md')).toBe('/a%20b/c.md')
    expect(encodePath('sub dir/a.md')).toBe('sub%20dir/a.md')
  })
})

describe('buildEditorHref — preset table', () => {
  // The documented scheme (VS Code) or tag (JetBrains) for each preset key.
  const VSCODE = ['vscode', 'vscode-insiders', 'vscodium', 'cursor', 'windsurf']
  const JETBRAINS = [
    'idea', 'web-storm', 'pycharm', 'php-storm', 'goland',
    'rubymine', 'clion', 'rd', 'rustrover',
  ]

  it('covers exactly the documented preset keys (no drift)', () => {
    expect(Object.keys(EDITOR_PRESETS).sort()).toEqual([...VSCODE, ...JETBRAINS].sort())
  })

  it.each(VSCODE)('VS Code family %s → {scheme}://file{abs} (single slash)', (key) => {
    expect(href(key)).toBe(`${key}://file/Users/x/a%20b.md`)
  })

  it.each(JETBRAINS)('JetBrains %s → navigate/reference with tag, project, rel', (key) => {
    expect(href(key)).toBe(
      `jetbrains://${key}/navigate/reference?project=myrepo&path=sub%20dir/a.md`,
    )
  })
})

describe('buildEditorHref — VS Code single-slash regression', () => {
  it('produces a single slash before the absolute path (no file//)', () => {
    const out = href('vscode', { abs: '/Users/x/a b.md' })
    expect(out).toBe('vscode://file/Users/x/a%20b.md')
    expect(out).not.toContain('file//')
  })
})

describe('buildEditorHref — custom templates', () => {
  it('substitutes {abs} verbatim: zed://file{abs}', () => {
    expect(href('zed://file{abs}', { abs: '/a b/c.md' })).toBe('zed://file/a%20b/c.md')
  })

  it('substitutes {rel} and preserves / while encoding spaces', () => {
    expect(href('myeditor://x?path={rel}', { rel: 'sub dir/a.md' })).toBe(
      'myeditor://x?path=sub%20dir/a.md',
    )
  })

  it('substitutes ALL occurrences of a placeholder (replaceAll, not replace-first)', () => {
    expect(href('ed://{abs}?dup={abs}', { abs: '/a b.md' })).toBe(
      'ed:///a%20b.md?dup=/a%20b.md',
    )
  })
})

describe('buildEditorHref — preset vs custom branch', () => {
  it('treats a bare preset key as a preset (no :// in the value)', () => {
    expect(href('cursor')).toBe('cursor://file/Users/x/a%20b.md')
  })

  it('returns null for a non-matching bare value (no placeholder, no scheme)', () => {
    expect(href('notaneditor')).toBeNull()
  })
})

describe('buildEditorHref — unresolvable / unsafe → null', () => {
  it('returns null for a :// template with no {abs}/{rel} placeholder', () => {
    expect(href('myeditor://open')).toBeNull()
  })

  it('returns null for a placeholder template with no scheme', () => {
    expect(href('{rel}')).toBeNull()
    expect(href('./{rel}')).toBeNull()
  })

  it.each(['javascript', 'data', 'vbscript', 'blob', 'file'])(
    'returns null for a dangerous %s: scheme even with a placeholder',
    (scheme) => {
      expect(href(`${scheme}:doEvil({abs})`)).toBeNull()
    },
  )

  it('still resolves a benign editor scheme', () => {
    expect(href('zed://file{abs}', { abs: '/a.md' })).toBe('zed://file/a.md')
  })
})

describe('buildEditorHref — scheme-guard bypass vectors', () => {
  it('rejects a mixed-case dangerous scheme', () => {
    expect(href('JavaScript:doEvil({rel})')).toBeNull()
  })

  it('rejects a leading-whitespace dangerous scheme', () => {
    expect(href('  javascript:doEvil({rel})')).toBeNull()
  })

  it('rejects an embedded TAB/CR/LF in the scheme', () => {
    expect(href('java\tscript:doEvil({rel})')).toBeNull()
    expect(href('java\nscript:doEvil({rel})')).toBeNull()
    expect(href('java\rscript:doEvil({rel})')).toBeNull()
  })
})
