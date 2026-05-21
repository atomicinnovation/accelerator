---
date: "2026-05-21T00:00:00+01:00"
type: plan
skill: create-plan
work-item: "0078"
status: done
---

# Detail-Page Frontmatter Table Implementation Plan

## Overview

Add a `FrontmatterTable` component that renders every key/value pair from a
detail page's parsed YAML frontmatter as a CSS-grid two-column table directly
above the markdown body, with work-item-style ID values auto-linkified. The
existing `FrontmatterChips` strip in the subtitle slot remains untouched —
the table is an additional, full-fidelity surface, not a replacement.

Implementation is decomposed into three TDD-driven, independently testable
phases:

1. **Bare-ID matcher** — pure helpers in the wiki-links module that build a
   bare-token regex from `defaultProjectCode` and split a string into
   text/match segments. Surface a `bareIdPattern` from `useWikiLinkResolver`.
2. **`FrontmatterTable` component** — new component folder with rendering
   tests covering scalar/array/object formatting, empty-value dashes, source
   order, linkification, and styling assertions.
3. **`LibraryDocView` integration** — mount the table inside the article
   grid between the malformed-banner row and the markdown body, sharing the
   body column's width cap.

Each phase ships independently: Phase 1 leaves the matcher and pattern
unused but covered by tests; Phase 2 lands the component without a
consumer; Phase 3 wires it into the page.

## Current State Analysis

- **`FrontmatterChips`** at
  `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`
  is the existing parallel component. It is a discriminated-union prop
  (`'absent' | 'malformed' | 'parsed'`) over
  `Record<string, unknown>`, iterates `Object.entries`, drops `null /
  undefined / ''`, and renders the remainder as a flat `<Chip>` strip in
  the page subtitle slot. It does not linkify and does not import
  `useWikiLinkResolver`.

- **`LibraryDocView`** at
  `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`
  is the detail-page route. It already invokes `useWikiLinkResolver()`
  at line 46, passes the resolver and pattern into `MarkdownRenderer`,
  and renders the chip strip via `FrontmatterChips` in the page
  subtitle slot (lines 79–84). The article grid (`.article`) has two
  columns: `1fr` body + `260px` aside, with a `.malformedBanner` row
  that spans both via `grid-column: 1 / -1` (lines 1–6, 17–25 of the
  module CSS).

- **`useWikiLinkResolver`** at
  `skills/visualisation/visualise/frontend/src/api/use-wiki-link-resolver.ts`
  returns `{ resolver, pattern }`. The pattern is bracket-anchored
  (`[[ADR-NNNN]]` / `[[WORK-ITEM-NNNN]]` / project-prefixed forms) and
  unsuitable for matching bare scalar values like `WORK-ITEM-0041` or
  `PROJ-0041`. The resolver — `(prefix, id) => ResolverResult` — is
  reusable as-is. `defaultProjectCode` is fetched from
  `/api/work-item/config` and feeds `buildWikiLinkPattern` at line 60–63.

- **CSS tokens** in `frontend/src/styles/global.css` and
  `frontend/src/styles/tokens.ts`: `--ac-bg-sunken`, `--ac-stroke`,
  `--ac-fg-muted`, and `--ac-font-mono` (Fira Code) all exist. The
  story's reference to `--ac-text-muted` is wrong — the correct token
  is `--ac-fg-muted`. The `11.5px` and `12px 14px` literals from the
  prototype have no exact token equivalents and will need
  `EXCEPTIONS` entries in `frontend/src/styles/migration.test.ts`.

- **Markdown body width** is a hard-coded `max-width: 720px` literal at
  `MarkdownRenderer.module.css:2`. It is already an `EXCEPTIONS` entry
  ("prose max-width — no token equivalent"). Until 0088 introduces a
  shared width variable, the table mirrors the same literal with its
  own `EXCEPTIONS` entry.

- **Frontmatter shape** is exposed to clients as `Record<string,
  unknown>` on `IndexEntry.frontmatter`, with `frontmatterState:
  'parsed' | 'absent' | 'malformed'` alongside (`api/types.ts:64–80`).
  YAML key insertion order is preserved end-to-end (server YAML parser
  → JSON → `Object.entries`).

## Desired End State

A user opening any detail-page route (e.g. `/library/work-items/0078`)
sees a compact, monospace, sunken-background table immediately above the
markdown body. Every frontmatter key declared in the source file is
present as a row in source order. Empty values show a decorative dimmed
em-dash (`aria-hidden="true"`) so screen readers don't announce
"empty" once per row. Scalar `WORK-ITEM-NNNN` (and project-prefixed)
tokens and `ADR-NNNN` tokens are anchors with a visible link colour
and focus ring that route to the same destination as the markdown
body's wiki-links, resolved through the shared resolver. During the
docs-cache warm-up window, unresolved-yet tokens render with the same
muted-italic pending treatment the markdown body uses, so both
surfaces agree visually. The chip strip in the subtitle continues to
render unchanged.

### Verification:

- Visit `/library/work-items/0078` — the table renders nine rows in the
  order `work_item_id, title, date, author, kind, status, priority,
  parent, tags`. The `parent: ""` row shows a dimmed em-dash whose
  `aria-hidden="true"` keeps it silent for screen readers. The
  `tags` row shows `design, frontend, detail-page, markdown` as
  plain-text comma-joined values. The container's `aria-label` is
  `Document metadata` so the section is announced with user-facing
  wording rather than the developer-jargon "frontmatter".
- Visit a work item whose `parent` value is `"0041"` (or a future work
  item that mentions another by ID) — clicking that value routes to
  the matching work-item page; the link is resolved through the same
  index the markdown body uses.
- `npm run typecheck && npm test` in
  `skills/visualisation/visualise/frontend/` passes, including the
  new `FrontmatterTable.test.tsx`, the new `wiki-links.test.ts`
  cases for `buildBareIdPattern` / segmentation, the extended
  `use-wiki-link-resolver.test.tsx` case for `bareIdPattern`, and the
  migration-test EXCEPTIONS check.

### Key Discoveries:

- The pattern returned by `useWikiLinkResolver` is **bracket-anchored**
  (`\[\[(ADR|WORK-ITEM)-…\]\]` at `wiki-links.ts:28-33`) — it cannot
  match bare scalars. The table needs its own bare-ID regex.
- `resolveWikiLink(prefix, id, idx)` at `wiki-links.ts:110-129` is a
  pure function of `(prefix, id)` and is fully reusable for the bare-ID
  case. The resolver returned by the hook is the right API surface.
- The resolver returns a **tri-state** result (`'resolved' | 'pending'
  | 'unresolved'`). `'pending'` is emitted while either docs query is
  warming the cache; the markdown body renders this case with a
  `wiki-link-pending` span at `wiki-link-plugin.ts:99`. The table
  mirrors this treatment with its own `.pending` class so both
  surfaces agree visually during the cache-warming window — diverging
  would produce a flash-of-content-change between adjacent surfaces
  for the same `(prefix, id)`.
- The `WORK-ITEM` prefix is canonical in this codebase, not `WORK`. The
  story's references to `WORK-####` should be read as `WORK-ITEM-####`.
- `--ac-text-muted` does not exist; use `--ac-fg-muted` (light `#5f6378`
  / dark `#a0a5b8`).
- `--size-chip-md` (11.5px) already exists in `tokens.ts` and is
  consumed by `Chip.module.css`. Reusing it for the table's font-size
  avoids a redundant EXCEPTIONS entry and keeps the design-token
  surface coherent.
- The migration test (`src/styles/migration.test.ts`) enforces that
  every hex / `px|rem|em` literal in a `.module.css` is either a token
  reference or an `EXCEPTIONS` entry. After tokenising 11.5px and the
  12px vertical padding, only `14px`, `1px`, `2px`, `720px`, and
  `12rem` literals remain to be allowlisted.
- `Object.entries` over a JSON-parsed YAML object preserves insertion
  order for string keys, so "render in source order" is the JS default;
  no sort needed. Caveat: integer-string keys (`'0'`, `'1'`, …) would
  be reordered numerically before string keys — unlikely for
  YAML-authored frontmatter but worth noting.
- `\b`-anchored regex matching admits embedded matches inside
  hyphen-joined and path-shaped strings (`MY-ADR-0017`,
  `notes/WORK-ITEM-0042.md`, `WORK-ITEM-0042-suffix`). The chosen
  behaviour is to allow these matches — they are pinned by tests so
  any future tightening is an explicit, tested change.

## What We're NOT Doing

- **Not** modifying or replacing `FrontmatterChips` — that work
  belongs to 0084.
- **Not** changing `MarkdownRenderer`, the wiki-link remark plugin,
  or the resolver's index/build logic.
- **Not** introducing a collapse/expand affordance — the table is
  always expanded per AC.
- **Not** harmonising the markdown body's width with the new table —
  that work belongs to 0088. The table mirrors `MarkdownRenderer`'s
  `720px` literal as an interim measure.
- **Not** rendering a partial-parse view for `malformed` frontmatter.
  The malformed banner already communicates the error; adding the
  table on top would either duplicate the signal (if the parser
  returned `{}`) or imply a successful partial parse the server
  does not actually expose. Authors fix YAML errors by editing the
  source file, not by interrogating the rendered page. If the
  server later exposes a partial-parse surface, revisit.
- **Not** unifying the `FrontmatterChips` and `FrontmatterTable`
  state-handling shapes. `FrontmatterChips` accepts a discriminated
  union (`'absent' | 'malformed' | 'parsed'`) for historical reasons;
  `FrontmatterTable` accepts only the parsed shape and is gated at
  the call site. The asymmetry is acknowledged and intentional —
  consolidating both surfaces is out of scope until 0084.
- **Not** adding a status-coloured row treatment — the table is
  visually flat; status colouring continues to live on the chip strip.
- **Not** exposing the server-side `work.id_pattern` string to the
  client — the table reads `defaultProjectCode` only and builds the
  bare-ID regex from that, matching the existing resolver's input
  surface.
- **Not** humanising the H1 (0085's scope) or building the aside region
  (0079's scope).

## Implementation Approach

Three TDD phases. Each lands a green test suite plus the code needed to
satisfy it, and is independently verifiable.

Tests are written **before** implementation in every phase. Existing
tests demonstrate the codebase's style:
`FrontmatterChips.test.tsx` for component-level RTL assertions,
`use-wiki-link-resolver.test.tsx` for hook tests via `renderHook` with a
`QueryClient` wrapper, `wiki-links.test.ts` for pure-module assertions.

---

## Phase 1: Bare-ID Matcher Utility

### Overview

Add two pure helpers to the wiki-links module and surface a
`bareIdPattern` from `useWikiLinkResolver`, so any consumer (the
forthcoming `FrontmatterTable` is the first, but the API stands alone)
can match bare `ADR-NNNN` / `WORK-ITEM-NNNN` / `WORK-ITEM-PROJ-NNNN`
tokens inside arbitrary strings.

### Changes Required:

#### 1. `wiki-links.ts` — `buildBareIdPattern` + `splitByBareIds`

**File**:
`skills/visualisation/visualise/frontend/src/api/wiki-links.ts`

**Changes**: Add two exported helpers parallel to `buildWikiLinkPattern`
and a small companion type. Mirror the project-code grammar restriction
called out at lines 26–27 (no hyphens inside project codes).

```ts
// Public exports added near buildWikiLinkPattern:

export type BareIdSegment =
  | { kind: 'text'; text: string }
  | {
      kind: 'match'
      text: string
      prefix: 'ADR' | 'WORK-ITEM'
      id: string
    }

/** Build a bare-token regex that matches the wiki-link grammar without
 *  the surrounding `[[ … ]]`. Used by surfaces that linkify scalar
 *  values (e.g. frontmatter cells) rather than markdown text.
 *
 *  Word-boundary anchored so embedded matches inside free text are
 *  handled the same way the bracketed pattern handles them inside
 *  text nodes. Always returned with the `g` flag set so callers can
 *  iterate matches; `splitByBareIds` clones to avoid `lastIndex`
 *  bleed across calls. */
export function buildBareIdPattern(projectCode: string | null): RegExp {
  const innerWorkItem = projectCode
    ? `${escapeRegExp(projectCode)}-\\d+|\\d+`
    : `\\d+`
  return new RegExp(`\\b(ADR|WORK-ITEM)-(${innerWorkItem})\\b`, 'g')
}

/** Split a string into ordered segments alternating between plain text
 *  and bare-ID matches. Empty leading/trailing text segments around
 *  matches are elided. Returns a single empty-text segment for an
 *  empty input.
 *
 *  Forces the `g` flag on the cloned regex so a caller that hands in
 *  a non-global pattern does not hang the loop. */
export function splitByBareIds(
  text: string,
  pattern: RegExp,
): BareIdSegment[] {
  const flags = pattern.flags.includes('g')
    ? pattern.flags
    : pattern.flags + 'g'
  const re = new RegExp(pattern.source, flags)
  const segments: BareIdSegment[] = []
  let lastIndex = 0
  let m: RegExpExecArray | null
  while ((m = re.exec(text)) !== null) {
    if (m.index > lastIndex) {
      segments.push({ kind: 'text', text: text.slice(lastIndex, m.index) })
    }
    segments.push({
      kind: 'match',
      text: m[0],
      prefix: m[1] as 'ADR' | 'WORK-ITEM',
      id: m[2],
    })
    lastIndex = m.index + m[0].length
  }
  if (lastIndex < text.length) {
    segments.push({ kind: 'text', text: text.slice(lastIndex) })
  }
  if (segments.length === 0) {
    segments.push({ kind: 'text', text })
  }
  return segments
}
```

#### 2. `wiki-links.test.ts` — pattern + segmentation tests (TDD: written first)

**File**:
`skills/visualisation/visualise/frontend/src/api/wiki-links.test.ts`

**Changes**: Add a `describe('buildBareIdPattern')` block and a
`describe('splitByBareIds')` block alongside the existing
`buildWikiLinkPattern` coverage.

```ts
describe('buildBareIdPattern', () => {
  it('matches bare ADR and WORK-ITEM tokens when projectCode is null', () => {
    const re = buildBareIdPattern(null)
    expect('ADR-0001'.match(re)).not.toBeNull()
    expect('WORK-ITEM-0042'.match(re)).not.toBeNull()
  })

  it('captures the inner token even when wrapped in brackets', () => {
    // Bracket characters are non-word, so the bare-ID regex's \b
    // anchors still match at the inner edges of `[[ … ]]`. The
    // bracketed-text plugin owns markdown text; the table owns scalar
    // cells, which are not expected to contain brackets. Pinning the
    // behaviour here documents that the two patterns are NOT mutually
    // exclusive — they are scoped by consumer, not by grammar.
    const re = buildBareIdPattern(null)
    const matches = [...'[[WORK-ITEM-0042]]'.matchAll(re)]
    expect(matches.length).toBe(1)
    expect(matches[0][1]).toBe('WORK-ITEM')
    expect(matches[0][2]).toBe('0042')
  })

  it('matches project-prefixed forms when projectCode is set', () => {
    const re = buildBareIdPattern('PROJ')
    expect('WORK-ITEM-PROJ-0042'.match(re)).not.toBeNull()
    expect('WORK-ITEM-0042'.match(re)).not.toBeNull() // bare-numeric fallback
  })

  it('does not match non-token text', () => {
    const re = buildBareIdPattern(null)
    expect('see issue 0042 for context'.match(re)).toBeNull()
    expect('WORKBOOK-0042'.match(re)).toBeNull()
  })

  describe('word-boundary collision cases', () => {
    // The `\b` anchors mean the pattern can match inside larger
    // strings where the surrounding character is non-word (typically
    // `-`, `/`, `.`, or whitespace). These tests pin the chosen
    // behaviour so future tightening (e.g. negative lookbehinds) is
    // an explicit, tested change rather than a silent regression.
    const re = buildBareIdPattern(null)

    it('matches ADR-NNNN embedded inside a longer hyphen-joined token', () => {
      // `MY-ADR-0017` — \b between `Y` and `-` is a boundary, \b
      // between `-` and `A` is a boundary, so the inner ADR-0017
      // matches. Document the behaviour; the visualiser surface
      // does not currently expose strings of this shape, so
      // matching is acceptable.
      const matches = [...'MY-ADR-0017'.matchAll(re)]
      expect(matches.length).toBe(1)
      expect(matches[0][0]).toBe('ADR-0017')
    })

    it('matches WORK-ITEM-NNNN even when followed by an extra hyphenated suffix', () => {
      // `WORK-ITEM-0042-suffix` — the digit run ends at `-`, which
      // is a word boundary, so `WORK-ITEM-0042` is captured and the
      // suffix is rendered as plain trailing text.
      const matches = [...'WORK-ITEM-0042-suffix'.matchAll(re)]
      expect(matches.length).toBe(1)
      expect(matches[0][0]).toBe('WORK-ITEM-0042')
    })

    it('matches a token inside a path-shaped string', () => {
      // `notes/WORK-ITEM-0042.md` — `/`, `.`, and end-of-string are
      // non-word, so the inner token matches.
      const matches = [...'notes/WORK-ITEM-0042.md'.matchAll(re)]
      expect(matches.length).toBe(1)
      expect(matches[0][0]).toBe('WORK-ITEM-0042')
    })

    it('does not match a token immediately followed by a word character', () => {
      // `WORK-ITEM-0042a` — `\b` requires a transition; `2` and `a`
      // are both word chars, so no boundary, no match.
      expect('WORK-ITEM-0042a'.match(re)).toBeNull()
    })
  })
})

describe('splitByBareIds', () => {
  it('returns a single text segment when no matches are present', () => {
    const segs = splitByBareIds('plain value', buildBareIdPattern(null))
    expect(segs).toEqual([{ kind: 'text', text: 'plain value' }])
  })

  it('returns a single match segment when the whole string is one token', () => {
    const segs = splitByBareIds('WORK-ITEM-0042', buildBareIdPattern(null))
    expect(segs).toEqual([
      { kind: 'match', text: 'WORK-ITEM-0042', prefix: 'WORK-ITEM', id: '0042' },
    ])
  })

  it('interleaves text and matches in source order', () => {
    const segs = splitByBareIds('see WORK-ITEM-0041 for context', buildBareIdPattern(null))
    expect(segs).toEqual([
      { kind: 'text', text: 'see ' },
      { kind: 'match', text: 'WORK-ITEM-0041', prefix: 'WORK-ITEM', id: '0041' },
      { kind: 'text', text: ' for context' },
    ])
  })

  it('handles consecutive matches with no separator text', () => {
    const segs = splitByBareIds('WORK-ITEM-0001 WORK-ITEM-0002', buildBareIdPattern(null))
    expect(segs.filter((s) => s.kind === 'match').length).toBe(2)
  })

  it('returns one empty text segment for empty input', () => {
    const segs = splitByBareIds('', buildBareIdPattern(null))
    expect(segs).toEqual([{ kind: 'text', text: '' }])
  })

  it('captures project-prefixed IDs when the pattern includes them', () => {
    const segs = splitByBareIds('WORK-ITEM-PROJ-0099', buildBareIdPattern('PROJ'))
    expect(segs).toEqual([
      { kind: 'match', text: 'WORK-ITEM-PROJ-0099', prefix: 'WORK-ITEM', id: 'PROJ-0099' },
    ])
  })
})
```

#### 3. `use-wiki-link-resolver.ts` — surface `bareIdPattern`

**File**:
`skills/visualisation/visualise/frontend/src/api/use-wiki-link-resolver.ts`

**Changes**: Extend the hook to compute and return `bareIdPattern`
alongside `pattern` and `resolver`. The bare pattern is memoised on the
same `defaultProjectCode` key as the bracketed pattern.

```ts
import { buildBareIdPattern, /* … */ } from './wiki-links'

export interface UseWikiLinkResolverResult {
  resolver: Resolver
  pattern: RegExp
  bareIdPattern: RegExp
}

// inside useWikiLinkResolver():
const bareIdPattern = useMemo<RegExp>(
  () => buildBareIdPattern(workItemConfig.data?.defaultProjectCode ?? null),
  [workItemConfig.data?.defaultProjectCode],
)

return { resolver, pattern, bareIdPattern }
```

#### 4. `use-wiki-link-resolver.test.tsx` — one extra case

**File**:
`skills/visualisation/visualise/frontend/src/api/use-wiki-link-resolver.test.tsx`

**Changes**: Add an assertion alongside the existing cases.

```ts
it('exposes a bareIdPattern that matches bare WORK-ITEM tokens', async () => {
  const qc = new QueryClient()
  vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])

  const { result } = renderHook(() => useWikiLinkResolver(), {
    wrapper: makeWrapper(qc),
  })

  await waitFor(() => {
    // bareIdPattern is built synchronously from the work-item-config
    // query, which resolves immediately with {} in the beforeEach stub.
    expect('WORK-ITEM-0042'.match(result.current.bareIdPattern)).not.toBeNull()
  })
})
```

### Success Criteria:

#### Automated Verification:

- [x] All new `wiki-links.test.ts` cases pass:
      `cd skills/visualisation/visualise/frontend && npm test -- wiki-links.test`
- [x] The new `use-wiki-link-resolver.test.tsx` case passes:
      `npm test -- use-wiki-link-resolver.test`
- [x] Existing wiki-link-related tests are untouched and still green:
      `npm test -- wiki-link`
- [x] Type checking passes: `npm run typecheck`
- [x] Full test suite remains green: `npm test`

#### Manual Verification:

- [ ] Reading the new helpers and the `bareIdPattern` value in
      DevTools console (via a temporary log in `LibraryDocView`)
      confirms the regex compiles with the expected source.

---

## Phase 2: `FrontmatterTable` Component

### Overview

Build a self-contained `FrontmatterTable` component that takes a
parsed-frontmatter object plus the resolver and bare-ID pattern from
Phase 1, and renders a CSS-grid two-column table per the story's
acceptance criteria. The component is unmounted at the end of this
phase — Phase 3 mounts it.

### Changes Required:

#### 1. Component module file

**File**:
`skills/visualisation/visualise/frontend/src/components/FrontmatterTable/FrontmatterTable.tsx`
(new)

**Changes**: Implement the component. Prop shape parallels
`FrontmatterChips` but is intentionally narrower — the malformed and
absent banners stay with the chip strip and `LibraryDocView`, so the
table accepts only the parsed shape.

```tsx
import { Fragment, type ReactNode } from 'react'
import type { Resolver } from '../MarkdownRenderer/wiki-link-plugin'
import { splitByBareIds } from '../../api/wiki-links'
import styles from './FrontmatterTable.module.css'

export interface FrontmatterTableProps {
  frontmatter: Record<string, unknown>
  resolveWikiLink: Resolver
  bareIdPattern: RegExp
}

function isEmpty(value: unknown): boolean {
  if (value === null || value === undefined) return true
  if (typeof value === 'string' && value === '') return true
  if (Array.isArray(value) && value.length === 0) return true
  if (
    typeof value === 'object' &&
    value !== null &&
    !Array.isArray(value) &&
    Object.keys(value as object).length === 0
  ) {
    return true
  }
  return false
}

function safeStringify(value: unknown): string {
  try {
    return JSON.stringify(value)
  } catch {
    return String(value)
  }
}

function renderScalar(
  text: string,
  resolveWikiLink: Resolver,
  bareIdPattern: RegExp,
): ReactNode {
  // Index keys are intentional: the segment list is derived purely
  // from `text` and is regenerated on every render with no
  // reordering or insertion, so reconciliation is stable.
  const segments = splitByBareIds(text, bareIdPattern)
  return segments.map((seg, i) => {
    if (seg.kind === 'text') {
      return <Fragment key={i}>{seg.text}</Fragment>
    }
    const result = resolveWikiLink(seg.prefix, seg.id)
    if (result.kind === 'resolved') {
      return (
        <a key={i} href={result.href} title={result.title}>
          {seg.text}
        </a>
      )
    }
    if (result.kind === 'pending') {
      // Mirror the markdown body's pending treatment so both surfaces
      // agree visually during the cache-warming window.
      return (
        <span key={i} className={styles.pending}>
          {seg.text}
        </span>
      )
    }
    return <Fragment key={i}>{seg.text}</Fragment>
  })
}

function renderValue(
  value: unknown,
  resolveWikiLink: Resolver,
  bareIdPattern: RegExp,
): ReactNode {
  if (Array.isArray(value)) {
    return value.map((el, i) => (
      <Fragment key={i}>
        {i > 0 ? ', ' : ''}
        {renderScalar(
          typeof el === 'object' && el !== null
            ? safeStringify(el)
            : String(el),
          resolveWikiLink,
          bareIdPattern,
        )}
      </Fragment>
    ))
  }
  if (typeof value === 'object' && value !== null) {
    // Route the JSON-serialised form through `renderScalar` so any
    // embedded WORK-ITEM/ADR tokens inside the object linkify
    // consistently with scalar cells. `safeStringify` falls back to
    // `String(value)` if serialisation throws (e.g. circular refs).
    return renderScalar(safeStringify(value), resolveWikiLink, bareIdPattern)
  }
  return renderScalar(String(value), resolveWikiLink, bareIdPattern)
}

export function FrontmatterTable({
  frontmatter,
  resolveWikiLink,
  bareIdPattern,
}: FrontmatterTableProps) {
  const entries = Object.entries(frontmatter)
  if (entries.length === 0) return null

  return (
    <dl className={styles.table} aria-label="Document metadata">
      {entries.map(([key, value]) => (
        <div key={key} className={styles.row}>
          <dt className={styles.key}>{key}</dt>
          <dd className={styles.value} data-empty={isEmpty(value) || undefined}>
            {isEmpty(value) ? (
              <span className={styles.empty} aria-hidden="true">—</span>
            ) : (
              renderValue(value, resolveWikiLink, bareIdPattern)
            )}
          </dd>
        </div>
      ))}
    </dl>
  )
}
```

#### 2. Component CSS module

**File**:
`skills/visualisation/visualise/frontend/src/components/FrontmatterTable/FrontmatterTable.module.css`
(new)

**Changes**: CSS grid two-column layout, Fira Code 11.5px, sunken bg,
stroke border, 12px 14px padding. Width capped to the same 720px the
markdown body uses today.

```css
.table {
  display: block;
  margin: 0 0 var(--sp-4);
  padding: var(--sp-3) 14px;
  background: var(--ac-bg-sunken);
  border: 1px solid var(--ac-stroke);
  border-radius: var(--radius-sm);
  font-family: var(--ac-font-mono);
  font-size: var(--size-chip-md);
  max-width: 720px;
}

.row {
  display: grid;
  grid-template-columns: minmax(auto, 12rem) 1fr;
  column-gap: var(--sp-3);
  align-items: baseline;
}

.row + .row {
  margin-top: var(--sp-1);
}

.key {
  color: var(--ac-fg-muted);
  margin: 0;
  white-space: nowrap;
}

.value {
  color: var(--ac-fg-strong);
  margin: 0;
  overflow-wrap: break-word;
}

.value a {
  color: var(--ac-link);
  text-decoration: underline;
  text-underline-offset: 2px;
}

.value a:hover {
  color: var(--ac-link-hover);
}

.value a:focus-visible {
  outline: 2px solid var(--ac-focus);
  outline-offset: 2px;
  border-radius: var(--radius-xs);
}

.empty {
  color: var(--ac-fg-muted);
}

.pending {
  color: var(--ac-fg-muted);
  font-style: italic;
}
```

The anchor styles reference `--ac-link`, `--ac-link-hover`,
`--ac-focus`, and `--radius-xs`. **Pre-flight check during
implementation**: verify each of these tokens exists in
`frontend/src/styles/tokens.ts` (or `global.css`). If any are absent,
either (a) add them to `tokens.ts` as part of this phase with both
light- and dark-mode values matching the existing palette
conventions, or (b) substitute the closest extant token (e.g.
`--ac-fg-strong` for `--ac-link` if no dedicated link token has
shipped yet) and note the substitution in the migration-test
EXCEPTIONS as appropriate. The CSS source assertions below pin the
chosen token names, so confirm before writing the tests.

#### 3. Component test file (TDD: written first)

**File**:
`skills/visualisation/visualise/frontend/src/components/FrontmatterTable/FrontmatterTable.test.tsx`
(new)

**Changes**: Cover row count + ordering, empty-value dash, scalar
linkification, array rendering with embedded links, object
JSON-serialisation, numeric/boolean rendering, and CSS source
assertions for grid template and tokens.

```tsx
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { FrontmatterTable } from './FrontmatterTable'
import css from './FrontmatterTable.module.css?raw'
import type { Resolver } from '../MarkdownRenderer/wiki-link-plugin'

const passThroughResolver: Resolver = (prefix, id) => ({
  kind: 'resolved',
  href: `/library/${prefix === 'ADR' ? 'decisions' : 'work-items'}/${id}`,
  title: `${prefix}-${id}`,
})

const bareIdPattern = /\b(ADR|WORK-ITEM)-(\d+)\b/g

const baseProps = {
  resolveWikiLink: passThroughResolver,
  bareIdPattern,
}

describe('FrontmatterTable', () => {
  it('renders one row per frontmatter key in source order', () => {
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{
          work_item_id: '0078',
          title: 'Detail-Page Frontmatter Table',
          date: '2026-05-21',
          author: 'Toby Clemson',
          kind: 'story',
          status: 'ready',
          priority: 'medium',
          parent: '',
          tags: ['design', 'frontend'],
        }}
      />,
    )
    const rows = container.querySelectorAll('dt')
    expect(rows.length).toBe(9)
    const keys = Array.from(rows).map((dt) => dt.textContent)
    expect(keys).toEqual([
      'work_item_id', 'title', 'date', 'author', 'kind',
      'status', 'priority', 'parent', 'tags',
    ])
  })

  it('renders a dimmed em-dash for null, undefined, empty string, empty array, and empty object', () => {
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{
          a: null, b: undefined, c: '', d: [], e: {},
        } as Record<string, unknown>}
      />,
    )
    const empties = container.querySelectorAll('[data-empty]')
    expect(empties.length).toBe(5)
    empties.forEach((dd) => {
      const dash = dd.querySelector('span')
      expect(dash?.textContent).toBe('—')
      // Dash is decorative; screen readers must not announce "empty"
      // five times per page.
      expect(dash?.getAttribute('aria-hidden')).toBe('true')
    })
  })

  it('renders 0 and false as scalars, not as the empty dash', () => {
    render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ archived: false, version: 0 }}
      />,
    )
    expect(screen.getByText('false')).toBeInTheDocument()
    expect(screen.getByText('0')).toBeInTheDocument()
  })

  it('renders comma-joined array values with comma separators between elements', () => {
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ tags: ['design', 'frontend', 'detail-page'] }}
      />,
    )
    // Pin the full joined output, not just the labels — guards
    // against a regression that drops the `, ` separator.
    const dd = container.querySelector('dd')
    expect(dd?.textContent).toBe('design, frontend, detail-page')
  })

  it('renders object values as JSON-serialised strings', () => {
    render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ meta: { a: 1, b: 'two' } }}
      />,
    )
    expect(screen.getByText('{"a":1,"b":"two"}')).toBeInTheDocument()
  })

  it('linkifies WORK-ITEM tokens embedded inside an object value', () => {
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ refs: { related: 'WORK-ITEM-0041' } }}
      />,
    )
    // Object values are JSON-serialised and re-scanned for bare IDs,
    // so embedded tokens linkify just like scalar cells do.
    const link = container.querySelector('a[href="/library/work-items/0041"]')
    expect(link).not.toBeNull()
    expect(link?.textContent).toBe('WORK-ITEM-0041')
  })

  it('linkifies a scalar value that is exactly a WORK-ITEM token', () => {
    render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ parent: 'WORK-ITEM-0041' }}
      />,
    )
    const link = screen.getByRole('link', { name: 'WORK-ITEM-0041' })
    expect(link).toHaveAttribute('href', '/library/work-items/0041')
  })

  it('linkifies a WORK-ITEM token embedded in free text, leaving the rest plain', () => {
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ note: 'see WORK-ITEM-0041 for context' }}
      />,
    )
    const link = container.querySelector('a[href="/library/work-items/0041"]')
    expect(link).not.toBeNull()
    expect(link?.textContent).toBe('WORK-ITEM-0041')
    // surrounding text is plain
    expect(container.textContent).toContain('see ')
    expect(container.textContent).toContain(' for context')
  })

  it('linkifies array elements that match a token, leaving non-matching elements as text', () => {
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ refs: ['WORK-ITEM-0001', 'misc', 'ADR-0017'] }}
      />,
    )
    expect(container.querySelector('a[href="/library/work-items/0001"]')).not.toBeNull()
    expect(container.querySelector('a[href="/library/decisions/0017"]')).not.toBeNull()
    expect(container.textContent).toContain('misc')
  })

  it('does not linkify when the resolver returns unresolved', () => {
    const unresolved: Resolver = () => ({ kind: 'unresolved' })
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        resolveWikiLink={unresolved}
        frontmatter={{ parent: 'WORK-ITEM-9999' }}
      />,
    )
    expect(container.querySelector('a')).toBeNull()
    expect(screen.getByText('WORK-ITEM-9999')).toBeInTheDocument()
  })

  it('renders a pending span (matching the markdown body) when the resolver returns pending', () => {
    const pending: Resolver = () => ({ kind: 'pending' })
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        resolveWikiLink={pending}
        frontmatter={{ parent: 'WORK-ITEM-0041' }}
      />,
    )
    // No anchor during cache warm-up — same as the markdown body's
    // wiki-link-pending treatment.
    expect(container.querySelector('a')).toBeNull()
    // The pending token is still rendered as visible text inside a
    // dedicated span, so a refetch can swap it to a link without
    // remounting unrelated DOM.
    const pendingSpan = container.querySelector(
      'span[class*="pending"]',
    )
    expect(pendingSpan).not.toBeNull()
    expect(pendingSpan?.textContent).toBe('WORK-ITEM-0041')
  })

  it('returns null when the frontmatter object is empty', () => {
    const { container } = render(
      <FrontmatterTable {...baseProps} frontmatter={{}} />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('labels the container as "Document metadata" to match the malformed-banner wording', () => {
    render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ kind: 'story' }}
      />,
    )
    expect(screen.getByLabelText('Document metadata')).toBeInTheDocument()
  })

  describe('CSS token contract', () => {
    // Pin the design-system token *references* (these are
    // load-bearing for dark mode and the migration test). Pixel
    // values, paddings, and grid templates are policed by
    // `migration.test.ts` and are intentionally not asserted here —
    // those are cosmetic choices that should not break the component
    // test on a formatting tweak.
    it('uses --ac-bg-sunken for container background', () => {
      expect(css).toMatch(/background:\s*var\(--ac-bg-sunken\)/)
    })
    it('uses --ac-stroke for container border', () => {
      expect(css).toMatch(/border:\s*1px\s+solid\s+var\(--ac-stroke\)/)
    })
    it('uses --ac-font-mono (Fira Code) for table text', () => {
      expect(css).toMatch(/font-family:\s*var\(--ac-font-mono\)/)
    })
    it('uses --size-chip-md for table font-size (not a literal)', () => {
      expect(css).toMatch(/font-size:\s*var\(--size-chip-md\)/)
    })
    it('uses --ac-fg-muted for keys and the empty dash', () => {
      expect(css).toMatch(/color:\s*var\(--ac-fg-muted\)/)
    })
    it('styles anchor children with a link colour (not the inherited body colour)', () => {
      expect(css).toMatch(/\.value a\s*\{[^}]*color:\s*var\(--ac-link\)/)
    })
  })
})
```

#### 4. Migration test EXCEPTIONS

**File**:
`skills/visualisation/visualise/frontend/src/styles/migration.test.ts`

**Changes**: Add EXCEPTIONS entries for the new module's remaining
irreducible literals. The 11.5px font-size and the 12px vertical
padding are tokenised via `var(--size-chip-md)` and `var(--sp-3)`
respectively, so they do not need EXCEPTIONS entries; only `14px`
(horizontal padding, genuinely off-scale), `1px` (border width below
the `--sp-1` floor), `2px` (focus-ring outline / underline offset),
and `720px` (interim mirror of MarkdownRenderer; tied to 0088) need
to be allowlisted.

```ts
// components/FrontmatterTable/FrontmatterTable.module.css
{ file: 'components/FrontmatterTable/FrontmatterTable.module.css', literal: '14px', count: 1, kind: 'irreducible', reason: 'container horizontal padding from design — between --sp-3 (12px) and --sp-4; not aliased to a token' },
{ file: 'components/FrontmatterTable/FrontmatterTable.module.css', literal: '1px', count: 1, kind: 'irreducible', reason: 'border width — below --sp-1 floor' },
{ file: 'components/FrontmatterTable/FrontmatterTable.module.css', literal: '2px', count: 2, kind: 'irreducible', reason: 'focus-ring outline width / text-underline-offset — below --sp-1 floor' },
{ file: 'components/FrontmatterTable/FrontmatterTable.module.css', literal: '720px', count: 1, kind: 'irreducible', reason: 'table max-width mirroring MarkdownRenderer 720px until 0088 introduces shared width variable' },
{ file: 'components/FrontmatterTable/FrontmatterTable.module.css', literal: '12rem', count: 1, kind: 'irreducible', reason: 'key-column max-width cap to prevent unusually long YAML keys squeezing the value column on narrow viewports' },
```

**Pre-flight check**: confirm that `--size-chip-md` (11.5px),
`--sp-3` (12px), `--ac-link`, `--ac-link-hover`, `--ac-focus`, and
`--radius-xs` are present in `tokens.ts` / `global.css`. If
`--size-chip-md` is absent (unlikely — it is consumed by
`Chip.module.css` today), either substitute `--size-xxs` and add an
11.5px EXCEPTIONS entry with the chosen rationale, or introduce a
`--size-meta` alias.

### Success Criteria:

#### Automated Verification:

- [x] `FrontmatterTable.test.tsx` passes:
      `cd skills/visualisation/visualise/frontend && npm test -- FrontmatterTable.test`
- [x] `migration.test.ts` passes after EXCEPTIONS additions:
      `npm test -- migration.test`
- [x] Phase 1's tests remain green: `npm test -- wiki-link`
- [x] Type checking passes: `npm run typecheck`
- [x] Full test suite remains green: `npm test`

#### Manual Verification:

- [ ] No new ESLint or Vite warnings appear when running `npm run dev`
      after the component lands.

---

## Phase 3: `LibraryDocView` Integration

### Overview

Mount `FrontmatterTable` between the existing `.malformedBanner` row
and the markdown body's `.body` row inside the article grid. The table
takes its own grid-area in the body column so it sits above the
markdown but does not push the aside down. The chip strip and the page
header are unchanged.

### Changes Required:

#### 1. `LibraryDocView.tsx` — mount the table

**File**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`

**Changes**:

- Import `FrontmatterTable` from its new folder.
- Destructure `bareIdPattern` from the existing `useWikiLinkResolver()`
  invocation at line 46.
- Insert the table rendering between lines 117 (closing the malformed
  banner `</div>`) and 119 (opening of `<div className={styles.body}>`).
- Guard on `entry.frontmatterState === 'parsed'` so the table is
  suppressed for `absent` and `malformed` frontmatter — the malformed
  banner already communicates the error condition.

```tsx
import { FrontmatterTable } from '../../components/FrontmatterTable/FrontmatterTable'

// inside the render branch where entry && content.data:
const {
  resolver: resolveWikiLink,
  pattern: wikiLinkPattern,
  bareIdPattern,
} = useWikiLinkResolver()

// inside the article grid, between the malformed banner and .body:
{entry.frontmatterState === 'parsed' && (
  <div className={styles.frontmatter}>
    <FrontmatterTable
      frontmatter={entry.frontmatter as Record<string, unknown>}
      resolveWikiLink={resolveWikiLink}
      bareIdPattern={bareIdPattern}
    />
  </div>
)}
```

#### 2. `LibraryDocView.module.css` — grid-area for the table

**File**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.module.css`

**Changes**: Add a `.frontmatter` rule that pins the table to the body
column so the aside continues to align with the markdown content
below.

```css
/* Both .frontmatter and .body claim grid-area: body; CSS grid's
   auto-row placement stacks them vertically inside the body column
   while the aside column continues to align with the markdown body.
   If a future change adds a second named row to the article grid,
   expand grid-template-areas to "frontmatter aside" / "body aside"
   and rebind the rule below to grid-area: frontmatter. */
.frontmatter { grid-area: body; }
```

The article grid's `grid-template-areas: "body aside"` is reused as-is:
both `.frontmatter` and `.body` land in the `body` area; CSS grid
auto-rows handle the vertical stacking. No new EXCEPTIONS entries are
needed.

#### 3. Integration test (TDD: written first)

**File**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.test.tsx`
(extends the existing file)

**Changes**: `LibraryDocView.test.tsx` already exists with established
conventions: a `Wrapper` that mounts `QueryClientProvider` plus
`MemoryRouter`, a shared `mockEntry`, and `vi.spyOn` of
`fetchDocContent` / `fetchDocs` / `fetchRelated` at the
`fetchModule` boundary. Reuse those — do not introduce a bespoke
`vi.stubGlobal('fetch', …)` scaffold, and use the real API name
`fetchDocContent` (not `fetchDoc`). `makeIndexEntry` (or the existing
`mockEntry` factory) already supports `frontmatterState` via the
`Partial<IndexEntry>` override surface; no fixture extension is
needed.

Add three new test cases inside the existing `describe` block:

```tsx
// Inside the existing describe('LibraryDocView', () => { … })

it('renders the FrontmatterTable when frontmatterState is parsed', async () => {
  const entry = {
    ...mockEntry,
    frontmatter: { kind: 'story', status: 'ready', parent: '' },
    frontmatterState: 'parsed' as const,
  }
  vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([entry])
  vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
    content: '# Body',
  })

  render(<LibraryDocView /* same args as existing tests */ />, {
    wrapper: Wrapper,
  })

  expect(
    await screen.findByLabelText('Document metadata'),
  ).toBeInTheDocument()
})

it('does NOT render the FrontmatterTable when frontmatterState is malformed', async () => {
  const entry = {
    ...mockEntry,
    frontmatter: {},
    frontmatterState: 'malformed' as const,
  }
  vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([entry])
  vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
    content: '# Body',
  })

  render(<LibraryDocView /* same args */ />, { wrapper: Wrapper })

  await screen.findByRole('alert') // existing malformed banner
  expect(screen.queryByLabelText('Document metadata')).toBeNull()
})

it('does NOT render the FrontmatterTable when frontmatterState is absent', async () => {
  const entry = {
    ...mockEntry,
    frontmatter: {},
    frontmatterState: 'absent' as const,
  }
  vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([entry])
  vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
    content: '# Body',
  })

  render(<LibraryDocView /* same args */ />, { wrapper: Wrapper })

  await screen.findByText('# Body')
  expect(screen.queryByLabelText('Document metadata')).toBeNull()
})

it('linkifies a WORK-ITEM scalar value via the shared resolver (end-to-end)', async () => {
  // Two real entries: the doc whose frontmatter references another
  // work item by ID, and the referenced work item that the resolver
  // must look up. This pins the headline AC end-to-end: the table
  // and the markdown body resolve through the *same* hook and the
  // anchor's href matches what the body would produce.
  const referenced = {
    ...mockEntry,
    relPath: 'meta/work/0041-page-wrapper.md',
    workItemId: '0041',
    title: 'Page wrapper',
  }
  const subject = {
    ...mockEntry,
    relPath: 'meta/work/0078-detail-page-frontmatter-table.md',
    workItemId: '0078',
    frontmatter: { parent: 'WORK-ITEM-0041' },
    frontmatterState: 'parsed' as const,
  }
  vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([subject, referenced])
  vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
    content: '# Body',
  })

  const { container } = render(
    <LibraryDocView /* same args, slug for subject */ />,
    { wrapper: Wrapper },
  )

  await screen.findByLabelText('Document metadata')
  const link = container.querySelector(
    'a[href$="/library/work-items/0041-page-wrapper"]',
  )
  expect(link).not.toBeNull()
  expect(link?.textContent).toBe('WORK-ITEM-0041')
})
```

The exact `LibraryDocView` invocation (`type` / `fileSlug` props vs
router-driven path) should follow whatever the existing tests use —
mirror, do not invent. The `href` substring in the end-to-end test
asserts the resolver-built path (which includes the slug), not just
the work-item ID, so the test pins both the resolver lookup and the
final href shape.

### Success Criteria:

#### Automated Verification:

- [x] Integration test passes:
      `cd skills/visualisation/visualise/frontend && npm test -- LibraryDocView.test`
- [x] Phase 1 and Phase 2 tests remain green:
      `npm test -- 'wiki-link|FrontmatterTable|migration|use-wiki-link-resolver'`
- [x] Type checking passes: `npm run typecheck`
- [x] Full test suite remains green: `npm test`
- [x] `npm run build` succeeds (Vite + TS compile):
      `npm run build`

#### Manual Verification:

- [ ] Run the dev server (`npm run dev` in
      `skills/visualisation/visualise/frontend`), navigate to
      `/library/work-items/0078-detail-page-frontmatter-table`, and
      confirm:
      - Table renders above the markdown body, below the page header.
      - Nine rows are visible in source order, including `parent: —`.
      - `tags` row shows the comma-joined list.
- [ ] Visit a work item whose `parent` value references another work
      item by `WORK-ITEM-NNNN` form; clicking the link routes correctly.
- [ ] Visit a route whose source file has been deliberately mangled to
      produce `frontmatterState: 'malformed'` (or use a fixture in dev)
      — the table is absent, the malformed banner is present, and
      the markdown body renders as-is.
- [ ] Toggle dark mode — the sunken background, stroke border,
      muted-key colour, link colour, focus-ring outline, and pending-span
      colour all shift to their dark-token values; contrast remains
      acceptable. Confirm `--ac-link`, `--ac-link-hover`, `--ac-focus`,
      and `--radius-xs` exist in the design tokens before this check.
- [ ] Resize the viewport across the markdown-body breakpoint — the
      table's `max-width: 720px` matches the body cap visually; the
      aside does not overlap.
- [ ] Narrow the viewport to ~360px — long keys do not push the value
      column to single-character wraps; the `minmax(auto, 12rem)` key
      cap engages; values with `WORK-ITEM-NNNN` tokens wrap on
      whitespace (`break-word`), not mid-token.
- [ ] With a cold cache (hard refresh or first-load of an
      unvisited route), confirm `WORK-ITEM-NNNN` cells render with
      the pending span treatment, then transition to live links
      once the docs queries settle — same behaviour as bracketed
      wiki-links in the markdown body.
- [ ] Use a screen reader (VoiceOver/NVDA) to traverse a row with
      `parent: ""` — the `<dd>` should be announced cleanly (e.g.
      "parent, empty" or punctuated silence), not "parent empty"
      repeated for every empty row on the page.

---

## Testing Strategy

### Unit Tests:

- **Phase 1 (`wiki-links.test.ts`)**: bare-ID pattern behaviour
  (default vs project-prefixed; positive/negative cases; word-boundary
  collision cases including `MY-ADR-0017`, suffix-collision,
  path-shaped strings, and word-char trailing rejection) and
  `splitByBareIds` segmentation (single text, single match,
  interleaved, consecutive matches, empty input, project-prefixed IDs).
- **Phase 1 (`use-wiki-link-resolver.test.tsx`)**: one extra case
  asserting `bareIdPattern` is returned and matches a bare token.
- **Phase 2 (`FrontmatterTable.test.tsx`)**: source-order rendering,
  empty-value dash for null/undefined/''/`[]`/`{}` with
  `aria-hidden="true"` and `[data-empty]` selector,
  scalar/array/object formatting (including object linkification via
  recursive `renderScalar`), numeric/boolean rendering,
  comma-separator pinning for arrays, linkification (whole scalar,
  embedded substring, array elements, unresolved case, **pending
  case**), empty-object null return, `Document metadata` aria-label,
  and CSS token-reference assertions (background, border, font,
  font-size token, fg-muted, anchor colour). Literal-value CSS
  assertions are intentionally omitted — `migration.test.ts` polices
  those.

### Integration Tests:

- **Phase 3 (`LibraryDocView.test.tsx`)**: extending the existing
  file, asserts that (a) the table is present for `parsed`
  frontmatter under the `Document metadata` aria-label, (b) the
  table is absent and the malformed banner is present for
  `malformed` frontmatter, (c) the table is absent for `absent`
  frontmatter, and (d) — the headline end-to-end case — a
  `parent: 'WORK-ITEM-0041'` scalar resolves via the *real*
  `useWikiLinkResolver` to an anchor whose href points at the
  referenced entry's slug.

### Manual Testing Steps:

1. `cd skills/visualisation/visualise/frontend && npm run dev`
2. Navigate to `/library/work-items/0078-detail-page-frontmatter-table`
3. Confirm: nine rows in source order; `parent: —` renders with a
   dimmed em-dash; `tags` row shows the comma list.
4. Click any work-item-style value (if linkified in a fixture) and
   verify it routes to the corresponding entry.
5. Toggle dark mode (theme toggle in the header) and confirm tokens
   re-bind correctly.
6. Visit a malformed-frontmatter document (mangling the YAML
   delimiters locally is the fastest way) and confirm the table is
   suppressed.

## Performance Considerations

- `Object.entries(frontmatter)` is O(n) over a typically tiny key set
  (~10 keys); no memoisation needed.
- `splitByBareIds` instantiates a fresh `RegExp` from the cached
  pattern's `source/flags` each call to avoid `lastIndex` cross-call
  bleed and to force the `g` flag defensively; cost is negligible.
- `resolveWikiLink` is already memoised inside `useWikiLinkResolver`
  and is reused as-is. No extra fetches are introduced.
- Object values are JSON-serialised via a `safeStringify` wrapper
  that catches circular-reference errors and falls back to
  `String(value)` — frontmatter is YAML-sourced so cycles are
  unreachable in practice, but the wrapper keeps the failure mode
  contained if a future codepath feeds in a non-YAML object.
- The `'pending'` resolver branch re-renders the same DOM shape as
  the `'resolved'` branch (a `<span>` swapped for an `<a>` once the
  cache warms), so React can reuse text nodes and avoid full
  remounts during cache warm-up.

## Migration Notes

None — this is a purely additive frontend change. The chip strip and
all server-side surfaces are unchanged. Any existing detail-page route
gains a table; any frontmatter shape continues to render.

## References

- Original work item: `meta/work/0078-detail-page-frontmatter-table.md`
- Codebase research: `meta/research/codebase/2026-05-21-0078-detail-page-frontmatter-table.md`
- Story review: `meta/reviews/work/0078-detail-page-frontmatter-table-review-1.md`
- Parallel component (chips): `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`
- Mount point: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:46,79-84,117-121`
- Resolver hook: `skills/visualisation/visualise/frontend/src/api/use-wiki-link-resolver.ts:43-80`
- Wiki-link primitives: `skills/visualisation/visualise/frontend/src/api/wiki-links.ts:28-129`
- Migration test allowlist: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
- Related work items: 0041 (page wrapper, shipped), 0084 (chip cap),
  0085 (H1 humanisation), 0088 (markdown width harmonisation).
