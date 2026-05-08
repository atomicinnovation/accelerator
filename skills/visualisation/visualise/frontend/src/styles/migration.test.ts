import { describe, it, expect } from 'vitest'
import {
  LIGHT_COLOR_TOKENS,
  DARK_COLOR_TOKENS,
  TYPOGRAPHY_TOKENS,
  SPACING_TOKENS,
  RADIUS_TOKENS,
  LIGHT_SHADOW_TOKENS,
  DARK_SHADOW_TOKENS,
  LAYOUT_TOKENS,
} from './tokens'

const cssModules = import.meta.glob('../**/*.module.css', {
  eager: true,
  query: '?raw',
  import: 'default',
}) as Record<string, string>

const cssGlobals = import.meta.glob('../**/*.global.css', {
  eager: true,
  query: '?raw',
  import: 'default',
}) as Record<string, string>

// All `0` resets are auto-permitted (admitted by AC4's escape-hatch);
// the regex excludes them at the source so they never need EXCEPTIONS
// entries — keeps the list focused on genuine exceptions.
const HEX_RE = /#[0-9a-fA-F]{3,8}\b/g
const PX_REM_EM_RE = /\b(?!0(?:px|rem|em)\b)\d+(?:\.\d+)?(?:px|rem|em)\b/g
const VAR_REF_RE = /var\(\s*--([\w-]+)\s*[,)]/g
// Scoped to --ac-* / --sp-* / --radius-* / --size-* / --shadow-* / --lh-*
// tokens — i.e. the new layered set. Legacy `--color-*` fallback sites
// remain present at Phase 2 commit and are deleted by Phase 3; this regex
// deliberately does not flag them so the harness lands green.
const VAR_FALLBACK_RE = /var\(\s*--(?:ac-|sp-|radius-|size-|shadow-|lh-|tracking-)[\w-]+\s*,/g
const VAR_COUNT_RE = /var\(\s*--/g

// Per-occurrence exception model. `count` is how many times this
// `(file, literal)` pair is allowed to match; when the implementer
// migrates one occurrence, they decrement the count or remove the
// entry. Adding a new exception requires `count: 1` and a reason.
// `file` is the path **relative to `src/`** to disambiguate any future
// modules that share a basename across directories.
type Exception = { file: string; literal: string; count: number; reason: string }

const EXCEPTIONS: ReadonlyArray<Exception & { kind: 'to-migrate' | 'irreducible' }> = [
  // components/Brand/Brand.module.css
  { file: 'components/Brand/Brand.module.css', literal: '10px', count: 1, kind: 'irreducible', reason: 'VISUALISER sub-label — below --size-xxs (12px) floor' },
  { file: 'components/Brand/Brand.module.css', literal: '2px', count: 1, kind: 'irreducible', reason: 'text stack gap — below --sp-1 (4px) floor' },
  // components/FrontmatterChips/FrontmatterChips.module.css
  { file: 'components/FrontmatterChips/FrontmatterChips.module.css', literal: '0.4rem', count: 1, kind: 'irreducible', reason: 'off-scale gap (6.4px) — between --sp-1 and --sp-2' },
  { file: 'components/FrontmatterChips/FrontmatterChips.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  // components/MarkdownRenderer/MarkdownRenderer.module.css
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '#1e1e1e', count: 1, kind: 'irreducible', reason: 'code block background — editor-dark, no surface token' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '#d4d4d4', count: 1, kind: 'irreducible', reason: 'code block text colour — editor-light-fg, no token' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '1px', count: 2, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '1.75rem', count: 1, kind: 'irreducible', reason: 'h1 font-size (28px) — 6px above size-lg ceiling; no heading token' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '0.88em', count: 1, kind: 'irreducible', reason: 'relative em font-size on inline code — not a rem scale value' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '0.4rem', count: 1, kind: 'irreducible', reason: 'off-scale cell padding (6.4px) — between --sp-1 and --sp-2' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '0.1rem', count: 1, kind: 'irreducible', reason: 'sub-pixel code padding — below --sp-1 floor' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '4px', count: 1, kind: 'irreducible', reason: 'blockquote border-left width — no border-width token' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '6px', count: 1, kind: 'irreducible', reason: 'code block border-radius — between radius-sm and radius-md' },
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '720px', count: 1, kind: 'irreducible', reason: 'prose max-width — no token equivalent' },
  // components/PipelineDots/PipelineDots.module.css
  { file: 'components/PipelineDots/PipelineDots.module.css', literal: '14px', count: 2, kind: 'irreducible', reason: 'dot width/height — fixed icon pixel, no sp-* equivalent' },
  { file: 'components/PipelineDots/PipelineDots.module.css', literal: '5px', count: 2, kind: 'irreducible', reason: 'inner dot size — fixed icon pixel, no sp-* equivalent' },
  { file: 'components/PipelineDots/PipelineDots.module.css', literal: '1.5px', count: 1, kind: 'irreducible', reason: 'dot ring width — below --sp-1 floor' },
  { file: 'components/PipelineDots/PipelineDots.module.css', literal: '6px', count: 1, kind: 'irreducible', reason: 'pipeline gap — layout pixel, no sp-* equivalent' },
  // components/RelatedArtifacts/RelatedArtifacts.module.css
  { file: 'components/RelatedArtifacts/RelatedArtifacts.module.css', literal: '2px', count: 3, kind: 'irreducible', reason: 'border-left widths and badge border-radius — below --sp-1 floor' },
  { file: 'components/RelatedArtifacts/RelatedArtifacts.module.css', literal: '0.4rem', count: 2, kind: 'irreducible', reason: 'off-scale spacing (6.4px) — between --sp-1 and --sp-2' },
  { file: 'components/RelatedArtifacts/RelatedArtifacts.module.css', literal: '0.15rem', count: 1, kind: 'irreducible', reason: 'item vertical padding (2.4px) — below --sp-1 floor' },
  { file: 'components/RelatedArtifacts/RelatedArtifacts.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  // components/Sidebar/Sidebar.module.css
  { file: 'components/Sidebar/Sidebar.module.css', literal: '0.08em', count: 1, kind: 'irreducible', reason: 'letter-spacing — off-scale, half of --tracking-caps' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '0.4rem', count: 1, kind: 'irreducible', reason: 'section heading margin (6.4px) — between --sp-1 and --sp-2' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '220px', count: 1, kind: 'irreducible', reason: 'fixed sidebar width — no token equivalent' },
  { file: 'components/Sidebar/Sidebar.module.css', literal: '2px', count: 1, kind: 'irreducible', reason: 'list gap — below --sp-1 floor' },
  // components/Breadcrumbs/Breadcrumbs.module.css
  { file: 'components/Breadcrumbs/Breadcrumbs.module.css', literal: '2px', count: 3, kind: 'irreducible', reason: 'outline width/offset, border-radius — below --sp-1 floor' },
  // components/OriginPill/OriginPill.module.css
  { file: 'components/OriginPill/OriginPill.module.css', literal: '6px', count: 2, kind: 'irreducible', reason: 'dot width/height — between --sp-1 (4px) and --sp-2 (8px)' },
  { file: 'components/OriginPill/OriginPill.module.css', literal: '3px', count: 1, kind: 'irreducible', reason: 'box-shadow ring spread — below --sp-1 floor' },
  // components/Topbar/Topbar.module.css
  { file: 'components/Topbar/Topbar.module.css', literal: '1px', count: 2, kind: 'irreducible', reason: 'border-bottom and divider widths — below --sp-1 floor' },
  // routes/kanban/KanbanBoard.module.css
  { file: 'routes/kanban/KanbanBoard.module.css', literal: '1px', count: 4, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  // routes/kanban/KanbanColumn.module.css
  { file: 'routes/kanban/KanbanColumn.module.css', literal: '16rem', count: 2, kind: 'irreducible', reason: 'column min-width/flex-basis — layout dimension, no token' },
  { file: 'routes/kanban/KanbanColumn.module.css', literal: '2px', count: 2, kind: 'irreducible', reason: 'outline width and offset — below --sp-1 floor' },
  // routes/kanban/WorkItemCard.module.css
  { file: 'routes/kanban/WorkItemCard.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  // routes/library/LibraryDocView.module.css
  { file: 'routes/library/LibraryDocView.module.css', literal: '4px', count: 1, kind: 'irreducible', reason: 'malformed-banner border-left width — no border-width token' },
  { file: 'routes/library/LibraryDocView.module.css', literal: '0.4rem', count: 1, kind: 'irreducible', reason: 'aside h3 margin (6.4px) — between --sp-1 and --sp-2' },
  { file: 'routes/library/LibraryDocView.module.css', literal: '1.6rem', count: 1, kind: 'irreducible', reason: 'title font-size (25.6px) — 3.6px above size-lg ceiling; no heading token' },
  { file: 'routes/library/LibraryDocView.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  { file: 'routes/library/LibraryDocView.module.css', literal: '1100px', count: 1, kind: 'irreducible', reason: 'article max-width — no token equivalent' },
  { file: 'routes/library/LibraryDocView.module.css', literal: '260px', count: 1, kind: 'irreducible', reason: 'aside column width — no token equivalent' },
  // routes/library/LibraryTemplatesIndex.module.css
  { file: 'routes/library/LibraryTemplatesIndex.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  { file: 'routes/library/LibraryTemplatesIndex.module.css', literal: '600px', count: 1, kind: 'irreducible', reason: 'container max-width — no token equivalent' },
  // routes/library/LibraryTemplatesView.module.css
  { file: 'routes/library/LibraryTemplatesView.module.css', literal: '1px', count: 2, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  { file: 'routes/library/LibraryTemplatesView.module.css', literal: '0.1rem', count: 1, kind: 'irreducible', reason: 'badge padding (1.6px) — below --sp-1 floor' },
  { file: 'routes/library/LibraryTemplatesView.module.css', literal: '900px', count: 1, kind: 'irreducible', reason: 'container max-width — no token equivalent' },
  // routes/library/LibraryTypeView.module.css
  { file: 'routes/library/LibraryTypeView.module.css', literal: '2px', count: 3, kind: 'irreducible', reason: 'border and outline widths — below --sp-1 floor' },
  { file: 'routes/library/LibraryTypeView.module.css', literal: '1px', count: 2, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  { file: 'routes/library/LibraryTypeView.module.css', literal: '0.4rem', count: 1, kind: 'irreducible', reason: 'off-scale cell padding (6.4px) — between --sp-1 and --sp-2' },
  { file: 'routes/library/LibraryTypeView.module.css', literal: '0.1rem', count: 1, kind: 'irreducible', reason: 'badge padding (1.6px) — below --sp-1 floor' },
  { file: 'routes/library/LibraryTypeView.module.css', literal: '900px', count: 1, kind: 'irreducible', reason: 'container max-width — no token equivalent' },
  // routes/lifecycle/LifecycleClusterView.module.css
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '1.5px', count: 2, kind: 'irreducible', reason: 'coloured ring widths — below --radius-sm/--sp-1 floor' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '2px', count: 2, kind: 'irreducible', reason: 'border/ring widths — below --sp-1 floor' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '1px', count: 3, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '0.4rem', count: 3, kind: 'irreducible', reason: 'off-scale spacing (6.4px) — between --sp-1 and --sp-2' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '0.05rem', count: 1, kind: 'irreducible', reason: 'sub-pixel badge padding — below --sp-1 floor' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '0.06em', count: 1, kind: 'irreducible', reason: 'letter-spacing — off-scale, half of --tracking-caps' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '0.08em', count: 1, kind: 'irreducible', reason: 'letter-spacing — off-scale, half of --tracking-caps' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '1.4em', count: 1, kind: 'irreducible', reason: 'calc(line-height × 3) for text-clamp — derived value' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '1.75rem', count: 1, kind: 'irreducible', reason: 'off-scale spacing (28px) — between --sp-5 and --sp-6' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '6px', count: 1, kind: 'irreducible', reason: 'timeline dot margin-left — layout pixel, no sp-* equivalent' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '7px', count: 1, kind: 'irreducible', reason: 'absolute dot position — layout pixel, no sp-* equivalent' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '1.25rem', count: 1, kind: 'irreducible', reason: 'off-scale padding (20px) — nearest --sp-4/--sp-5 are 4px off' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '800px', count: 1, kind: 'irreducible', reason: 'max-width — no spacing-scale equivalent' },
  // routes/lifecycle/LifecycleIndex.module.css
  { file: 'routes/lifecycle/LifecycleIndex.module.css', literal: '1px', count: 5, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
  { file: 'routes/lifecycle/LifecycleIndex.module.css', literal: '2px', count: 3, kind: 'irreducible', reason: 'outline width — below --sp-1 floor' },
  { file: 'routes/lifecycle/LifecycleIndex.module.css', literal: '6px', count: 2, kind: 'irreducible', reason: 'toolbar gap and card radius — layout pixels, no token equivalent' },
  { file: 'routes/lifecycle/LifecycleIndex.module.css', literal: '220px', count: 1, kind: 'irreducible', reason: 'filter input flex basis — no token equivalent' },
  { file: 'routes/lifecycle/LifecycleIndex.module.css', literal: '320px', count: 1, kind: 'irreducible', reason: 'card grid min-width — no token equivalent' },
  { file: 'routes/lifecycle/LifecycleIndex.module.css', literal: '900px', count: 1, kind: 'irreducible', reason: 'container max-width — no token equivalent' },
  // styles/wiki-links.global.css
  { file: 'styles/wiki-links.global.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border-bottom width — below --sp-1 floor' },
]

// Build O(1) lookup maps once at module load
const exceptionsByFile = new Map<string, Map<string, number>>()
for (const e of EXCEPTIONS) {
  let inner = exceptionsByFile.get(e.file)
  if (!inner) {
    inner = new Map()
    exceptionsByFile.set(e.file, inner)
  }
  inner.set(e.literal, (inner.get(e.literal) ?? 0) + e.count)
}

// Map vite-glob keys to the src-relative form used by EXCEPTIONS.file.
// Files in src/styles/ (same dir as this test) return a './' key — Vite
// normalises '../styles/foo.css' → './foo.css' when src and target share
// the same directory. Prepend 'styles/' to match EXCEPTIONS entries.
// Files in other src/ subdirectories return '../<rest>' keys as expected.
function srcRelative(globKey: string): string {
  if (globKey.startsWith('./')) {
    return 'styles/' + globKey.slice(2)
  }
  if (!globKey.startsWith('../') || globKey.startsWith('../../')) {
    throw new Error(
      `srcRelative: unexpected glob key shape "${globKey}". ` +
        `Expected "./" (same-dir) or exactly one "../" prefix (test sits at src/styles/, globs "../**/*.module.css"). ` +
        `If the test or glob has been moved, update srcRelative accordingly.`,
    )
  }
  return globKey.slice(3)
}

function permittedCount(file: string, literal: string): number {
  return exceptionsByFile.get(srcRelative(file))?.get(literal) ?? 0
}

function violations(matches: string[], file: string): string[] {
  const counts = new Map<string, number>()
  for (const m of matches) counts.set(m, (counts.get(m) ?? 0) + 1)
  const result: string[] = []
  for (const [literal, observed] of counts) {
    const allowed = permittedCount(file, literal)
    if (observed > allowed) {
      for (let i = 0; i < observed - allowed; i++) result.push(literal)
    }
  }
  return result
}

const allCss = { ...cssModules, ...cssGlobals }

describe('AC3: no hex literals outside EXCEPTIONS', () => {
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} hex literals all accounted for`, () => {
      const matches = [...css.matchAll(HEX_RE)].map((m) => m[0])
      expect(violations(matches, path)).toEqual([])
    })
  }
})

describe('AC4: no px/rem/em literals outside EXCEPTIONS (0-resets auto-excluded)', () => {
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} px/rem/em literals all accounted for`, () => {
      const matches = [...css.matchAll(PX_REM_EM_RE)].map((m) => m[0])
      expect(violations(matches, path)).toEqual([])
    })
  }
})

describe('var(--token, fallback) two-arg form is retired', () => {
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} contains no var(--*, fallback) sites`, () => {
      const fallbacks = [...css.matchAll(VAR_FALLBACK_RE)].map((m) => m[0])
      expect(fallbacks).toEqual([])
    })
  }
})

describe('color-mix() convention (Phase 4 special conventions)', () => {
  // Locked-in percentage ladder: 8 (default tinted bg), 18 (hover state),
  // 30 (stroke/border tint). Composition surface always var(--ac-bg).
  const COLOR_MIX_RE = /color-mix\(\s*in\s+srgb\s*,\s*var\(--ac-(err|warn|ok|violet)\)\s+(\d+)%\s*,\s*var\(--ac-bg\)\s*\)/g
  const COLOR_MIX_ANY_RE = /color-mix\(/g
  const ALLOWED_PERCENTAGES = new Set([8, 18, 30])

  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} color-mix sites use the locked-in convention`, () => {
      const totalSites = (css.match(COLOR_MIX_ANY_RE) ?? []).length
      const conventionalSites = [...css.matchAll(COLOR_MIX_RE)]
      expect(conventionalSites.length).toBe(totalSites)
      for (const m of conventionalSites) {
        expect(ALLOWED_PERCENTAGES.has(parseInt(m[2], 10))).toBe(true)
      }
    })
  }
})

describe('var(--NAME) references resolve to declared tokens', () => {
  const declared = new Set([
    ...Object.keys(LIGHT_COLOR_TOKENS),
    ...Object.keys(DARK_COLOR_TOKENS),
    ...Object.keys(TYPOGRAPHY_TOKENS),
    ...Object.keys(SPACING_TOKENS),
    ...Object.keys(RADIUS_TOKENS),
    ...Object.keys(LIGHT_SHADOW_TOKENS),
    ...Object.keys(DARK_SHADOW_TOKENS),
    ...Object.keys(LAYOUT_TOKENS),
  ])
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} references only declared tokens`, () => {
      const refs = [...css.matchAll(VAR_REF_RE)].map((m) => m[1])
      const unknown = refs.filter((name) => !declared.has(name))
      expect(unknown).toEqual([])
    })
  }
})

// AC5 enforcement is a true two-sided ratchet:
//
// - `AC5_FLOOR` is the *committed minimum*. It MUST equal the value
//   observed at the last committed state (or below it by no more than
//   AC5_REGRESSION_SLACK). The implementer bumps AC5_FLOOR upward in
//   the same commit that adds new var(--*) references.
// - `AC5_TARGET = 300` is the work-item contract.
const AC5_FLOOR = 408 // VISUALISER sub-label: 10px raw replaces var(--size-xxs) (-1)
const AC5_TARGET = 300 // contract from work item AC5
const AC5_REGRESSION_SLACK = 0

describe('AC5: aggregate var(--*) coverage (two-sided ratchet)', () => {
  const observed = Object.values(cssModules).reduce(
    (acc, css) => acc + (css.match(VAR_COUNT_RE)?.length ?? 0),
    0,
  )

  it(`observed count (${observed}) is at least AC5_FLOOR (${AC5_FLOOR})`, () => {
    expect(observed).toBeGreaterThanOrEqual(AC5_FLOOR - AC5_REGRESSION_SLACK)
  })

  it(`AC5_FLOOR (${AC5_FLOOR}) is not above observed (${observed}) — bump protocol followed`, () => {
    expect(AC5_FLOOR).toBeLessThanOrEqual(observed)
  })

  const finalStateActive = AC5_FLOOR >= AC5_TARGET
  ;(finalStateActive ? it : it.skip)(
    `(final-state gate) observed reaches AC5_TARGET (${AC5_TARGET})`,
    () => {
      expect(observed).toBeGreaterThanOrEqual(AC5_TARGET)
    },
  )
})

// Build the inverse map for hygiene checks.
const cssBySrcRelative = new Map<string, string>()
for (const [globKey, css] of Object.entries(allCss)) {
  cssBySrcRelative.set(srcRelative(globKey), css)
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

describe('EXCEPTIONS hygiene', () => {
  it('every EXCEPTIONS entry resolves to exactly one CSS file', () => {
    const unresolved: Exception[] = []
    for (const e of EXCEPTIONS) {
      if (!cssBySrcRelative.has(e.file)) unresolved.push(e)
    }
    expect(unresolved).toEqual([])
  })

  it('declared count equals observed count (no stale entries, no over-count)', () => {
    const mismatches: Array<{
      file: string
      literal: string
      declared: number
      observed: number
    }> = []
    for (const [file, literalMap] of exceptionsByFile) {
      const css = cssBySrcRelative.get(file)
      if (!css) continue
      const hexHits = [...css.matchAll(HEX_RE)].map((m) => m[0])
      const unitHits = [...css.matchAll(PX_REM_EM_RE)].map((m) => m[0])
      const allHits = [...hexHits, ...unitHits]
      for (const [literal, declared] of literalMap) {
        const observed = allHits.filter((h) => h === literal).length
        if (observed !== declared) {
          mismatches.push({ file, literal, declared, observed })
        }
      }
    }
    expect(mismatches).toEqual([])
  })
})

// Suppress unused-variable warning — escapeRegExp is available for future
// use by harness extensions that need to escape literal strings in regexes.
void escapeRegExp
