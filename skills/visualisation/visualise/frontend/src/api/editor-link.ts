interface VscodePreset { family: 'vscode'; scheme: string }
interface JetBrainsPreset { family: 'jetbrains'; tag: string }
type Preset = VscodePreset | JetBrainsPreset

// VS Code family: {scheme}://file{abs} (single slash; {abs} carries the leading /);
// scheme == preset key.
// JetBrains: jetbrains://{tag}/navigate/reference?project={project}&path={rel};
// tag == preset key (per the story's documented contract).
export const EDITOR_PRESETS: Record<string, Preset> = {
  vscode: { family: 'vscode', scheme: 'vscode' },
  'vscode-insiders': { family: 'vscode', scheme: 'vscode-insiders' },
  vscodium: { family: 'vscode', scheme: 'vscodium' },
  cursor: { family: 'vscode', scheme: 'cursor' },
  windsurf: { family: 'vscode', scheme: 'windsurf' },
  idea: { family: 'jetbrains', tag: 'idea' },
  'web-storm': { family: 'jetbrains', tag: 'web-storm' },
  pycharm: { family: 'jetbrains', tag: 'pycharm' },
  'php-storm': { family: 'jetbrains', tag: 'php-storm' },
  goland: { family: 'jetbrains', tag: 'goland' },
  rubymine: { family: 'jetbrains', tag: 'rubymine' },
  clion: { family: 'jetbrains', tag: 'clion' },
  rd: { family: 'jetbrains', tag: 'rd' },
  rustrover: { family: 'jetbrains', tag: 'rustrover' },
}

/** Percent-encode a path while preserving `/` separators:
 *  `/a b/c.md` → `/a%20b/c.md`. */
export function encodePath(p: string): string {
  return p.split('/').map(encodeURIComponent).join('/')
}

export interface EditorLinkInputs {
  editor: string            // preset key or custom template (non-empty)
  editorProject: string     // resolved JetBrains project name
  absPath: string           // entry.path
  relPath: string           // entry.relPath
}

/** Dangerous schemes rejected even via a custom template. This is a DENY-LIST (not an
 *  allow-list) so arbitrary *editor* protocols (zed://, subl://, txmt://, …) keep
 *  working through the escape hatch — a deliberate divergence from the MarkdownRenderer,
 *  which delegates to react-markdown's allow-list `urlTransform`. A deny-list is
 *  best-effort: extend it if a new dangerous scheme emerges. The bypass-resistant
 *  normalisation in `schemeOf` (it rejects embedded TAB/CR/LF and strips leading
 *  whitespace before matching the scheme) is what makes it sound. */
const BLOCKED_SCHEMES = new Set(['javascript', 'data', 'vbscript', 'blob', 'file'])

/** Strict, lowercased scheme of a URL, or null if there is no syntactically valid one.
 *  Leading ASCII whitespace is stripped first (browsers strip it before parsing the
 *  scheme), and the scheme is matched per RFC 3986 (`ALPHA *( ALPHA / DIGIT / + - . )`)
 *  with a `:` lookahead — so a token containing whitespace or control chars does NOT
 *  match, closing the classic ` javascript:` / `java\tscript:` deny-list bypass. */
function schemeOf(url: string): string | null {
  // Reject embedded TAB/CR/LF: browsers strip these from a URL before parsing its
  // scheme, so `java\t script:` would execute as `javascript:` while slipping past
  // a naive check. Treat their presence as unresolvable.
  if (/[\t\n\r]/.test(url)) return null
  const m = /^[a-z][a-z0-9+.-]*(?=:)/i.exec(url.trimStart())
  return m ? m[0].toLowerCase() : null
}

/** Resolve the deep-link href, or null if `editor` is empty/unresolvable/unsafe. */
export function buildEditorHref(inputs: EditorLinkInputs): string | null {
  const { editor, editorProject, absPath, relPath } = inputs
  const abs = encodePath(absPath)
  const rel = encodePath(relPath)

  const preset = EDITOR_PRESETS[editor]
  if (preset) {
    if (preset.family === 'vscode') {
      // `abs` already begins with `/`, so concatenate WITHOUT an extra slash —
      // `file` + `/Users/…` == `file/Users/…`, not `file//Users/…`.
      return `${preset.scheme}://file${abs}`
    }
    return `jetbrains://${preset.tag}/navigate/reference` +
      `?project=${encodeURIComponent(editorProject)}&path=${rel}`
  }

  // Custom template: MUST carry an {abs}/{rel} placeholder (a value that cannot
  // reference the file cannot open it). A `://`-but-placeholder-free value (e.g.
  // `myeditor://open`) resolves to null → disabled, not a path-less link.
  const hasPlaceholder = editor.includes('{abs}') || editor.includes('{rel}')
  if (!hasPlaceholder) {
    return null
  }
  const href = editor.replaceAll('{abs}', abs).replaceAll('{rel}', rel)

  // Scheme guard: require a syntactically valid scheme that is NOT on the deny-list.
  // A null scheme (relative / protocol-relative href, e.g. a bare `{rel}` template)
  // is unresolvable → disabled, never emitted as an in-app navigation link.
  const scheme = schemeOf(href)
  if (scheme === null || BLOCKED_SCHEMES.has(scheme)) {
    return null
  }

  return href
}
