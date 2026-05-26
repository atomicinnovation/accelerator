---
date: "2026-05-26T16:30:00+00:00"
type: plan
skill: create-plan
work_item_id: "0084"
status: draft
---

# Detail-Page Chip Strip Cap (Status, Date, Author) — Implementation Plan

## Overview

Cap the detail-page subtitle chip strip at three chips drawn from a
fixed whitelist (`status`, `date`, `author`) rendered in canonical
left-to-right order, and preserve the chip-strip container's vertical
height when zero chips qualify so the H1 stays offset from the
divider. The change is confined to one component
(`FrontmatterChips`), its CSS module, and its test file; the cap is
enforced inside the component so callers cannot opt more chips in.

Phases are organised TDD-first (failing tests precede each
implementation change) and structured so each phase is independently
reviewable and shippable: the whitelist + canonical order + input
normalisation (Phase 1), the empty-container height preservation
(Phase 2), and the corpus-wide regression armour (Phase 3) each own
a self-contained AC subset from work item 0084.

## Current State Analysis

The chip strip is implemented at
`skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`
with co-located `.module.css` and `.test.tsx`. It has exactly one
runtime consumer, `LibraryDocView`
(`skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:96-100`),
which is the single detail-page component all twelve doc kinds funnel
through. There is no per-kind variant.

The current component (`FrontmatterChips.tsx:30-57`):

- Iterates **every** non-null/non-undefined/non-empty-string
  frontmatter key in `Object.entries` (YAML insertion) order, with no
  whitelist and no cap.
- Dispatches three keys (`status`/`verdict`/`result`) to badge
  components via `BADGE_FOR_KEY` (`FrontmatterChips.tsx:20-28`) and
  routes everything else through `FrontmatterChip` (neutral variant).
- Returns `null` from both the `'absent'` state
  (`FrontmatterChips.tsx:31`) and the parsed-but-zero-entries branch
  (`FrontmatterChips.tsx:46`). In the latter case the `.subtitle`
  wrapper in `Page` collapses to 0 height because its only child
  rendered nothing — this is the exact "H1 sitting over the divider"
  failure the work item describes.
- Treats whitespace-only strings (`"   "`) as present, not missing.

CSS (`FrontmatterChips.module.css:1-6`):

```css
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  margin: 0 0 var(--sp-4);
}
```

No `min-height`. The container inherits `line-height: 1.5` from
`.subtitle` (`Page.module.css:52-60`).

Tests (`FrontmatterChips.test.tsx`) currently:

- Exercise the dispatch table for `status`/`verdict`/`result` and
  case-folded variants (`:67-112`).
- Pin source-order rendering (`:114-128`) — directly contradicts the
  fixed canonical order this plan introduces.
- Have AC integration fixtures (`:130-174`) that assert four chips
  including `verdict`/`priority`/`tags` — directly contradicts the
  whitelist this plan introduces.

The three upstream dependencies are all in place:

- **0038** — `Chip` primitive with six-variant API
  (`Chip.tsx:4`). Note: `data-*` props on chip instances are silently
  dropped; rely on the existing `data-testid` on wrapper components
  (`status-badge`, `frontmatter-chip`) for test assertions.
- **0078** — `FrontmatterTable` renders every key with no filter
  (`FrontmatterTable.tsx:94-99`); will duplicate `status`/`date`/
  `author` alongside the chip strip after this work lands. The
  prototype intentionally tolerates this duplication
  (`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md:511`)
  — no table change needed.
- **0081** — `StatusBadge` wraps `FrontmatterChip` with
  `statusToVariant` (`StatusBadge.tsx:4-17`). Used by this plan
  unchanged.

The 12 doc kinds enumerate at
`skills/visualisation/visualise/frontend/src/api/types.ts:4-14`
(`DOC_TYPE_KEYS` array). Frontmatter is typed as
`Record<string, unknown>` end-to-end — verification across the 12
kinds happens at the component layer via a parameterised render
test, not via a per-kind type or schema.

## Desired End State

After all three phases land:

- `FrontmatterChips` (parsed state, non-empty whitelist match) renders
  at most three chips drawn only from `{status, date, author}`,
  always in that fixed left-to-right order, regardless of frontmatter
  source order.
- Any frontmatter key outside the whitelist (including `verdict`,
  `result`, `priority`, `tags`, `id`, `type`, `kind`, `slug`, paths,
  hashes, mirrors) is **never** rendered as a chip — including via
  case variants (`Status`/`STATUS` still route correctly; `Verdict`
  no longer does).
- A key is treated as missing when it is absent, `null`, `undefined`,
  the empty string, or a whitespace-only string.
- Both `state === 'parsed'` (with zero qualifying chips) and
  `state === 'absent'` render an empty container element with
  `min-height: 1lh` and `aria-hidden="true"`, keeping the
  `.subtitle` slot at approximately the same vertical height as a
  one-chip strip (the exact match is not load-bearing — see Phase 2
  CSS commentary and manual verification for the visual-tolerance
  threshold). The wrapper carries a stable
  `data-testid="frontmatter-chips"` for test anchoring.
- The `'malformed'` state continues to render its existing
  `role="alert"` banner unchanged.
- A parameterised test iterates the 12 `DOC_TYPE_KEYS` with fixture
  frontmatter containing one non-canonical extra key and asserts that
  key never appears as a chip.

### Verification

Run from `skills/visualisation/visualise/frontend/`:

- `npm run test -- FrontmatterChips` — all unit tests pass.
- `npm run typecheck` — no TypeScript errors.
- `npm run build` — production build succeeds.

### Key Discoveries

- `LibraryDocView.tsx:95-100` unconditionally passes the
  `<FrontmatterChips>` element to the `Page` subtitle slot — the
  `Page.tsx:32-34` gate is `subtitle !== undefined`, not truthiness
  — so simply ensuring `FrontmatterChips` always renders a DOM
  container is sufficient to occupy the slot. No `Page` or
  `LibraryDocView` change is needed.
- `FrontmatterChips` already routes `status` through `StatusBadge`
  (`FrontmatterChips.tsx:21`), exactly what the work item's Technical
  Notes require. No `StatusBadge` change is needed.
- `Chip` primitive padding (`Chip.module.css:1-15`) plus the
  subtitle's inherited line-height makes `min-height: 1lh` on
  `.chips` an approximation (not exact equality) of a populated
  chip's rendered height. The visualiser is an internal developer
  tool — `package.json` and `vite.config.ts` declare no browserslist
  or build target, and the `1lh` unit (Safari 16.4+ / Chrome 110+ /
  Firefox 120+) is accepted explicitly under a "latest evergreen
  only" posture. If the audience widens, swap to a chip-metric
  `calc(...)` fallback.
- `FrontmatterTable` is the canonical surface for every other
  frontmatter key (`FrontmatterTable.tsx:94-99`); no table change is
  required for `verdict`/`result`/`priority`/`tags` etc. to remain
  visible.
- `DOC_TYPE_KEYS` (`api/types.ts:14`) is a `readonly` runtime array
  and the natural iterator for the parameterised 12-kind test.

## What We're NOT Doing

- **No `Page` component change.** The subtitle slot already renders
  when `subtitle !== undefined`; ensuring `FrontmatterChips` always
  emits a DOM container is sufficient.
- **No `LibraryDocView` runtime change.** The single consumer's wiring
  is already correct. Its co-located dispatch test
  (`LibraryDocView.dispatch.test.tsx`) is updated in Phase 1 — see
  Phase 1's Test-First Changes for details.
- **No `FrontmatterTable` change.** Duplicating
  `status`/`date`/`author` in the table alongside the chip strip is
  intentional per the prototype.
- **No `StatusBadge`, `statusToVariant`, or `Chip` change.**
- **No `VerdictBadge` or `ResultBadge` deletion.** Both components
  remain available for other consumers; only their dispatch entries
  in `FrontmatterChips` are removed.
- **No schema-alignment of templates.** Adding `author` to plans /
  pr-descriptions / review templates is deferred to the
  schema-alignment follow-up story (TBD per work item Open Questions).
- **No new `keys`/`include`/`exclude`/`renderChip` prop.** The cap is
  a property of `FrontmatterChips` itself; callers cannot opt more
  chips in.
- **No work-item frontmatter migration to ADR-0033 base fields.**
  Deferred to epic 0057 per the work item's Open Questions.
- **No browser/Playwright snapshot test.** The 12-kind verification
  is component-layer per the work item's last AC.

## Implementation Approach

Each phase follows strict TDD: write failing tests that lock in the
new contract, then make the minimal code change to flip them green.
Phases are independent in the sense that each owns its own AC
subset, its own test additions, and can be reviewed/merged as a
distinct unit. They touch the same file in places — annotated below
— but the changes layer cleanly:

- **Phase 1** replaces the iteration loop's body (whitelist +
  canonical order + case-fold dedup + whitespace-trim + dispatch
  cleanup) and deletes `LibraryDocView.dispatch.test.tsx`.
- **Phase 2** removes the `entries.length === 0` early return inside
  Phase 1's loop, adds CSS (`min-height: 1lh` + commentary), adds
  `aria-hidden` when empty, and removes the `'absent'` early return
  to use the same container.
- **Phase 3** adds the parameterised 13-kind regression-armour test;
  test-only, no production code.

Phases 1 and 2 will typically be implemented in sequence by the same
implementer (since both touch the `parsed`-branch return), but a
separate implementer could pick up Phase 3 in parallel against the
test file alone.

The existing AC integration fixture tests (`FrontmatterChips.test.tsx:130-174`)
and the source-order test (`:114-128`) contradict the new contract
and must be deleted in Phase 1. The dispatch tests for
`verdict`/`result` (`:75-87`, `:104-106`) must also be deleted in
Phase 1.

The co-located integration test `LibraryDocView.dispatch.test.tsx`
(three cases that assert `verdict-badge` / `result-badge` render
through `FrontmatterChips` for plan-review, work-item-review, and
validation kinds) becomes false after Phase 1 removes those
dispatches. All three cases are deleted in Phase 1. The
verdict→variant and result→variant tone contracts those tests once
guarded are already covered at the unit-test layer by
`VerdictBadge.test.tsx` and `ResultBadge.test.tsx` (both assert
`data-variant` per vocabulary), so deletion is a net simplification,
not a coverage loss.

---

## Phase 1: Whitelist + Canonical Ordering + Input Normalisation + Dispatch Cleanup

### Overview

Replace iteration over all frontmatter entries with iteration over
the fixed canonical list `['status', 'date', 'author']`. Fold
whitespace-trim and null/empty handling into the case-fold dedup
pass so a skipped case-variant cannot block a later valid value.
Inline the status dispatch (single badge-bearing canonical key);
delete `VerdictBadge`/`ResultBadge` imports. Delete
`LibraryDocView.dispatch.test.tsx`, whose three integration cases
became false the moment verdict/result no longer route through
`FrontmatterChips`. This phase enforces ACs 1, 2, 3, 6, and parts of
7, 8, 9, plus the whitespace-trim assumption.

### Test-First Changes

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.test.tsx`

**Deletions** (existing assertions that contradict the new contract):

- Delete the `'dispatches the verdict key to VerdictBadge'` test
  (`:75-80`).
- Delete the `'dispatches the result key to ResultBadge'` test
  (`:82-87`).
- Remove `verdict`/`Verdict`/`VERDICT` and `result`/`Result`/`RESULT`
  rows from the case-folded `it.each` table (`:102-106`); keep only
  the `status`/`Status`/`STATUS` rows.
- Delete the entire `'source order'` describe block (`:114-128`).
- Delete the entire `'AC integration fixtures'` describe block
  (`:130-174`).
- Delete the file
  `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.dispatch.test.tsx`
  in its entirety. Its three cases assert `verdict-badge` /
  `result-badge` render through `FrontmatterChips` for plan-review,
  work-item-review, and validation kinds — false after Phase 1's
  dispatch narrowing. Before deleting, confirm the verdict→variant
  and result→variant tone contracts remain covered at the unit-test
  layer by `VerdictBadge.test.tsx` and `ResultBadge.test.tsx`
  (verified during plan review: both files assert `data-variant`
  values per vocabulary).

**New assertions** (write before implementation):

```tsx
describe('whitelist + canonical order', () => {
  // Note: tests select chips by aria-label (preferred) or by the
  // specific badge/chip testids (`status-badge`, `frontmatter-chip`),
  // not by a generic `[data-testid]` query — the wrapper carries
  // `data-testid="frontmatter-chips"` and would otherwise inflate the
  // count.
  it('renders at most three chips from {status, date, author}', () => {
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{
          status: 'accepted', date: '2026-04-05', author: 'Toby Clemson',
          verdict: 'APPROVE', result: 'pass', priority: 'medium',
          tags: ['x'], id: '0084', kind: 'story',
        }}
      />,
    )
    const chipTestids = ['status-badge', 'frontmatter-chip'] as const
    const selector = chipTestids.map((t) => `[data-testid="${t}"]`).join(',')
    const chips = container.querySelectorAll(selector)
    expect(chips.length).toBe(3)
    expect(Array.from(chips).map((el) => el.getAttribute('data-testid'))).toEqual([
      'status-badge', 'frontmatter-chip', 'frontmatter-chip',
    ])
  })

  it('renders chips in canonical order regardless of frontmatter source order', () => {
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{ author: 'Toby Clemson', status: 'draft', date: '2026-04-05' }}
      />,
    )
    const labels = Array.from(container.querySelectorAll('[aria-label]'))
      .map((el) => el.getAttribute('aria-label'))
    expect(labels).toEqual([
      'status: draft', 'date: 2026-04-05', 'author: Toby Clemson',
    ])
  })

  it('renders only canonical keys when all three are present', () => {
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{ status: 'draft', date: '2026-04-05', author: 'Toby Clemson' }}
      />,
    )
    const labels = Array.from(container.querySelectorAll('[aria-label]'))
      .map((el) => el.getAttribute('aria-label'))
    expect(labels).toEqual([
      'status: draft', 'date: 2026-04-05', 'author: Toby Clemson',
    ])
  })
})

describe('whitelist exclusion', () => {
  it.each([
    ['verdict', 'APPROVE'],
    ['result', 'pass'],
    ['priority', 'medium'],
    ['tags', ['design', 'frontend']],
    ['id', '0084'],
    ['kind', 'story'],
    ['type', 'work-item'],
    ['slug', 'detail-page'],
    ['title', 'A title'],
    ['last_updated', '2026-05-26'],
    ['last_updated_by', 'Someone'],
  ])('does not render a chip for non-canonical key "%s"', (key, value) => {
    const { container } = render(
      <FrontmatterChips state="parsed" frontmatter={{ [key]: value } as Record<string, unknown>} />,
    )
    // Wrapper carries data-testid="frontmatter-chips" — assert no
    // child chip testids, not zero testids overall.
    expect(container.querySelector('[data-testid="status-badge"]')).toBeNull()
    expect(container.querySelector('[data-testid="frontmatter-chip"]')).toBeNull()
  })
})

describe('subset ordering', () => {
  it.each([
    [{ status: 'draft', date: '2026-04-05' }, ['status: draft', 'date: 2026-04-05']],
    [{ status: 'draft', author: 'Toby Clemson' }, ['status: draft', 'author: Toby Clemson']],
    [{ date: '2026-04-05', author: 'Toby Clemson' }, ['date: 2026-04-05', 'author: Toby Clemson']],
    [{ author: 'Toby Clemson', date: '2026-04-05' }, ['date: 2026-04-05', 'author: Toby Clemson']],
    [{ status: 'draft' }, ['status: draft']],
  ])('renders subset %# in canonical order', (frontmatter, expectedLabels) => {
    const { container } = render(
      <FrontmatterChips state="parsed" frontmatter={frontmatter} />,
    )
    const labels = Array.from(container.querySelectorAll('[aria-label]'))
      .map((el) => el.getAttribute('aria-label'))
    expect(labels).toEqual(expectedLabels)
  })
})

describe('case-fold dedup precedence', () => {
  it('skipped (null) case-variant does not block a later valid canonical key', () => {
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{ Status: null, status: 'draft' } as Record<string, unknown>}
      />,
    )
    const labels = Array.from(container.querySelectorAll('[aria-label]'))
      .map((el) => el.getAttribute('aria-label'))
    expect(labels).toEqual(['status: draft'])
  })

  it('first non-skipped value wins among case-variant duplicates', () => {
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{ Status: 'first', status: 'second' } as Record<string, unknown>}
      />,
    )
    const labels = Array.from(container.querySelectorAll('[aria-label]'))
      .map((el) => el.getAttribute('aria-label'))
    expect(labels).toEqual(['status: first'])
  })
})

describe('date / author precedence over last_updated mirrors', () => {
  it('uses date (creation-anchored), ignoring last_updated', () => {
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{ date: '2026-04-05', last_updated: '2026-05-26' }}
      />,
    )
    const labels = Array.from(container.querySelectorAll('[aria-label]'))
      .map((el) => el.getAttribute('aria-label'))
    expect(labels).toEqual(['date: 2026-04-05'])
  })

  it('uses author, ignoring last_updated_by', () => {
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{ author: 'Toby Clemson', last_updated_by: 'Someone Else' }}
      />,
    )
    const labels = Array.from(container.querySelectorAll('[aria-label]'))
      .map((el) => el.getAttribute('aria-label'))
    expect(labels).toEqual(['author: Toby Clemson'])
  })
})
```

**Add** to the existing `'parsed state'` describe block (`:7-37`):

```tsx
it('skips whitespace-only string values (treated as missing)', () => {
  const { container } = render(
    <FrontmatterChips
      state="parsed"
      frontmatter={{ status: '   ', date: '\t\n', author: 'Toby Clemson' }}
    />,
  )
  const labels = Array.from(container.querySelectorAll('[aria-label]'))
    .map((el) => el.getAttribute('aria-label'))
  expect(labels).toEqual(['author: Toby Clemson'])
})

it('renders a chip for a non-string canonical value (smoke test)', () => {
  // YAML parsers may emit Date objects for ISO dates; co-authored
  // documents may use array `author`. These are realistic shapes for
  // `Record<string, unknown>` frontmatter — pin the current
  // `FrontmatterChip.formatChipValue` rules (JSON.stringify for
  // non-array objects including Date, `, ` join for arrays) so
  // future changes there don't regress chip rendering silently.
  const { container } = render(
    <FrontmatterChips
      state="parsed"
      frontmatter={{
        date: new Date('2026-04-05T00:00:00Z'),
        author: ['Alice', 'Bob'],
      } as Record<string, unknown>}
    />,
  )
  const labels = Array.from(container.querySelectorAll('[aria-label]'))
    .map((el) => el.getAttribute('aria-label'))
  expect(labels).toEqual([
    // JSON.stringify(Date) yields a quoted ISO string — the literal
    // quotes are part of the rendered chip text. This is surprising
    // but accurate; if FrontmatterChip ever special-cases Date, this
    // assertion catches the change.
    'date: "2026-04-05T00:00:00.000Z"',
    'author: Alice, Bob',
  ])
})
```

**Retained assertions** (require small selector adjustments because
the wrapper now carries `data-testid="frontmatter-chips"` — the
existing `[data-testid]` count queries would include the wrapper):

- `'renders a chip for each non-null frontmatter value'` (`:8-17`)
  — change `container.querySelectorAll('[data-testid]')` to
  `container.querySelectorAll('[data-testid="status-badge"],
  [data-testid="frontmatter-chip"]')`. The fixture
  `{ status, date, author }` is already canonical so the assertion
  still expects length 3.
- `'skips null and undefined values'` (`:19-28`) — same selector
  change; expected count remains 1.
- `'skips empty-string values'` (`:30-36`) — same selector change;
  expected count remains 1.
- `'malformed state'` describe (`:46-56`) — unchanged (the
  malformed branch renders the banner, not the chip-strip wrapper).
- `'CSS source assertions'` describe (`:58-65`) — unchanged.
- `'dispatches the status key to StatusBadge'` (`:68-73`) —
  unchanged (queries `[data-testid="status-badge"]` directly).
- `'dispatches non-tone keys to FrontmatterChip'` — keep but narrow
  the fixture to `{ date: '2026-05-22', author: 'X' }` (both
  canonical keys go to `FrontmatterChip`, asserting two
  `frontmatter-chip` testids and no badges).
- Case-folded dispatch test, narrowed to `Status`/`STATUS`.

Run the test suite. All new tests in the `whitelist + canonical
order`, `whitelist exclusion`, `subset ordering`, and `date / author
precedence` blocks should **fail** because the component still
iterates source order.

### Implementation Changes

#### 1. Rewrite the whitelist + iteration in `FrontmatterChips`

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`

**Changes**:

- Add a `CANONICAL_KEYS` constant defining the whitelist and its
  fixed order, with an ADR-0033 cross-reference comment.
- Replace the `Object.entries(...).filter(...)` body in the parsed
  branch with iteration over `CANONICAL_KEYS`, picking up each
  canonical key (case-folded) from a Map populated only with
  non-null / non-undefined / non-whitespace values — so a skipped
  case-variant cannot block a later valid value (regression-tested
  in `case-fold dedup precedence`).
- Inline the status dispatch directly in the render loop; delete
  `BADGE_FOR_KEY`, `badgeFor`, and `BadgeProps` (the abstraction is
  not justified with `status` as the only badge-bearing key).
- Drop the now-unused `VerdictBadge`, `ResultBadge`, and
  `ComponentType` imports.
- Add `data-testid="frontmatter-chips"` to the rendered wrapper
  `<div>` so tests can anchor without relying on CSS-module hashing.

```tsx
import { FrontmatterChip } from '../FrontmatterChip/FrontmatterChip'
import { StatusBadge } from '../StatusBadge/StatusBadge'
import styles from './FrontmatterChips.module.css'

type FrontmatterChipsProps =
  | { state: 'absent' }
  | { state: 'malformed' }
  | { state: 'parsed'; frontmatter: Record<string, unknown> }

// The chip whitelist. Drawn from ADR-0033's unified base frontmatter
// schema (status / date / author are base fields shared across all
// doc kinds). If the base schema gains or loses a chip-worthy field,
// update this list and the ADR together.
const CANONICAL_KEYS = ['status', 'date', 'author'] as const

function pickCanonical(
  frontmatter: Record<string, unknown>,
): Array<[string, unknown]> {
  // Build a case-folded view, ignoring null / undefined / empty /
  // whitespace-only values during the fold. A skipped value never
  // claims the canonical slot, so `{ Status: null, status: 'draft' }`
  // correctly resolves to `'draft'` rather than being silently dropped
  // by a first-match-wins collision.
  const folded = new Map<string, unknown>()
  for (const [k, v] of Object.entries(frontmatter)) {
    if (v === null || v === undefined) continue
    if (typeof v === 'string' && v.trim() === '') continue
    const lk = k.trim().toLowerCase()
    if (!folded.has(lk)) folded.set(lk, v)
  }
  const picked: Array<[string, unknown]> = []
  for (const key of CANONICAL_KEYS) {
    if (folded.has(key)) picked.push([key, folded.get(key)])
  }
  return picked
}

export function FrontmatterChips(props: FrontmatterChipsProps) {
  if (props.state === 'absent') return null
  if (props.state === 'malformed') {
    return (
      <div role="alert" className={styles.banner}>
        Frontmatter unparseable — showing raw content.
      </div>
    )
  }

  const entries = pickCanonical(props.frontmatter)

  if (entries.length === 0) return null

  return (
    <div className={styles.chips} data-testid="frontmatter-chips">
      {entries.map(([key, value]) =>
        key === 'status'
          ? <StatusBadge key={key} value={value} />
          : <FrontmatterChip key={key} name={key} value={value} />
      )}
    </div>
  )
}
```

Notes on the implementation:

- The Phase 3 whitespace-trim predicate is included from Phase 1
  because folding it into the dedup pass (rather than re-checking
  values post-fold) is structurally simpler — see Phase 3 for the
  parameterised whitespace-only tests that pin this behaviour.
- The two `return null` paths (`'absent'` and zero-entries-parsed)
  are retained here; both flip to render-container in Phase 2.
- `pickCanonical` returns canonical lowercase keys, so chip
  `aria-label`s read `status: …`, `date: …`, `author: …` even when
  the original frontmatter uses `Status`/`Date`/`Author`. This is a
  deliberate user-visible change to the accessible name for
  case-variant frontmatter input.
- The dispatch is inlined — with `status` as the only badge-bearing
  canonical key, a `BADGE_FOR_KEY` map of one entry would signal an
  extensibility surface the whitelist actively forecloses. If a
  future canonical key needs a badge, reintroduce the map.
- A stable `data-testid="frontmatter-chips"` on the wrapper anchors
  Phase 2's empty-container tests without relying on CSS-module
  class hashing.

### Success Criteria

#### Automated Verification

- [x] All `FrontmatterChips.test.tsx` tests pass:
  `cd skills/visualisation/visualise/frontend && npm run test -- FrontmatterChips`
- [x] TypeScript compiles:
  `cd skills/visualisation/visualise/frontend && npm run typecheck`
- [x] Production build succeeds:
  `cd skills/visualisation/visualise/frontend && npm run build`
- [x] Full frontend test suite still passes:
  `cd skills/visualisation/visualise/frontend && npm run test`

#### Manual Verification

- [ ] Open the dev server (`npm run dev`) and visit a detail page
  for each of: a work item (has `status`, `date`, `author`), a
  research doc (has `date`, `author`; no `status`), and a
  plan-review (has `status`, `verdict`, `priority`, `tags`).
  Confirm the chip strip shows only canonical chips in canonical
  order and never includes `verdict`/`priority`/`tags`/`id`/`kind`.
- [ ] On a design-inventory page (which has `last_updated` and
  `last_updated_by` alongside `date`/`author`), confirm the chips
  read the `date` and `author` values, not the `last_updated*`
  mirrors.
- [ ] Verdict/result still appears in the frontmatter table on
  review and validation detail pages.
- [ ] **Verdict scannability**: open a plan-review detail page. The
  verdict is now a plain row in `FrontmatterTable`, no longer a
  coloured chip in the header. Confirm you can still find the
  verdict at a glance without scrolling. If the scannability loss
  feels material, capture a follow-up story to teach
  `FrontmatterTable` to render `verdict`/`result` with tone
  colouring (out of scope for this work item).
- [ ] **2-chip vs 3-chip asymmetry**: open a work-item detail page
  (renders 3 chips) and one of its plans in an adjacent tab (renders
  2 chips — plans templates currently lack `author`). Confirm the
  header width difference is visually acceptable. If jarring, the
  schema-alignment follow-up story (Open Questions on the work item)
  should be captured with an ID before this work merges.

---

## Phase 2: Empty-Container Height Preservation

### Overview

Ensure the chip-strip container is always present in the DOM on
detail pages (in both `'parsed'` and `'absent'` states) and has a
`min-height` matching one chip-line, so the H1 stays vertically
offset from the divider regardless of chip count. This phase
enforces ACs 4 and 5.

### Test-First Changes

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.test.tsx`

**Replace** the existing `'absent state'` describe block (`:39-44`):

```tsx
import styles from './FrontmatterChips.module.css'

describe('absent state', () => {
  it('renders the empty chip-strip container so the subtitle slot keeps its height', () => {
    const { container } = render(<FrontmatterChips state="absent" />)
    const strip = container.querySelector('[data-testid="frontmatter-chips"]')
    expect(strip).not.toBeNull()
    expect(strip).toBeInstanceOf(HTMLElement)
    expect((strip as HTMLElement).children.length).toBe(0)
    expect(strip).toHaveClass(styles.chips)
    expect(strip).toHaveAttribute('aria-hidden', 'true')
  })
})
```

The `data-testid="frontmatter-chips"` anchor (introduced in Phase 1)
keeps the assertion independent of CSS-module class hashing. The
`toHaveClass(styles.chips)` assertion wires the test-id and the
styling class together so a future refactor that drops one of them
fails the test rather than silently rendering an unstyled empty div.

**Add** a new describe block:

```tsx
describe('zero qualifying keys (parsed state)', () => {
  it('renders the empty chip-strip container when no canonical keys qualify', () => {
    const { container } = render(
      <FrontmatterChips state="parsed" frontmatter={{ priority: 'medium', tags: ['x'] }} />,
    )
    expect(container.querySelectorAll('[data-testid]').length).toBe(1)
    const strip = container.querySelector('[data-testid="frontmatter-chips"]')
    expect(strip).not.toBeNull()
    expect((strip as HTMLElement).children.length).toBe(0)
    expect(strip).toHaveAttribute('aria-hidden', 'true')
  })

  it('renders the empty chip-strip container when frontmatter is entirely empty', () => {
    const { container } = render(
      <FrontmatterChips state="parsed" frontmatter={{}} />,
    )
    const strip = container.querySelector('[data-testid="frontmatter-chips"]')
    expect(strip).not.toBeNull()
    expect((strip as HTMLElement).children.length).toBe(0)
    expect(strip).toHaveAttribute('aria-hidden', 'true')
  })

  it('omits aria-hidden when at least one chip qualifies', () => {
    const { container } = render(
      <FrontmatterChips state="parsed" frontmatter={{ status: 'draft' }} />,
    )
    const strip = container.querySelector('[data-testid="frontmatter-chips"]')
    expect(strip).not.toBeNull()
    expect(strip).not.toHaveAttribute('aria-hidden')
  })
})
```

**Extend** the existing `'CSS source assertions'` describe block
(`:58-65`):

```tsx
it('declares a min-height on .chips so the empty container preserves one-chip height', () => {
  expect(css).toMatch(/\.chips\s*\{[^}]*min-height:\s*1lh/)
})
```

Run the suite. The replaced `'absent state'` test, both `'zero
qualifying keys'` tests, and the new CSS source assertion should
**fail**.

### Implementation Changes

#### 1. Always render the container in `'absent'` and zero-entries-parsed

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`

**Changes** (relative to Phase 1):

- Drop the `'absent'` early-return; render an empty `.chips`
  container instead.
- Drop the `entries.length === 0` early-return; always render the
  `.chips` container in the parsed branch, possibly with zero
  children.

```tsx
export function FrontmatterChips(props: FrontmatterChipsProps) {
  if (props.state === 'malformed') {
    return (
      <div role="alert" className={styles.banner}>
        Frontmatter unparseable — showing raw content.
      </div>
    )
  }

  const entries = props.state === 'parsed'
    ? pickCanonical(props.frontmatter)
    : []

  // The empty container is a deliberate spacer reserving subtitle
  // height when no canonical chips qualify (see .chips min-height in
  // the module CSS). Mark it aria-hidden so screen readers don't
  // announce an undifferentiated landmark inside the subtitle slot.
  const isEmpty = entries.length === 0

  return (
    <div
      className={styles.chips}
      data-testid="frontmatter-chips"
      aria-hidden={isEmpty ? true : undefined}
    >
      {entries.map(([key, value]) =>
        key === 'status'
          ? <StatusBadge key={key} value={value} />
          : <FrontmatterChip key={key} name={key} value={value} />
      )}
    </div>
  )
}
```

#### 2. Add `min-height: 1lh` to `.chips`

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.module.css`

**Changes**:

```css
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  margin: 0 0 var(--sp-4);
  /* Reserve one inherited line-height so the subtitle slot keeps
     its height when zero canonical chips qualify (line-height is
     inherited from Page.module.css `.subtitle`: 1.5). This is an
     approximation of a populated chip's height, not an exact match:
     a chip's box is governed by its own font-size + padding + border,
     which differ from the subtitle cascade. Phase 2 manual
     verification compares H1-to-divider distance side-by-side to
     confirm the approximation is visually acceptable. */
  min-height: 1lh;
}
```

**Browser-target assumption**: `1lh` is supported in Safari 16.4+
(March 2023), Chrome 110+ (Feb 2023), and Firefox 120+ (Nov 2023).
The visualiser is an internal developer tool and we accept this
floor explicitly — `package.json` and `vite.config.ts` declare no
browserslist or build target, so absence of override is treated as
"latest evergreen only". If the tool's audience widens later, swap
to a `calc(...)` fallback derived from the chip's font-size +
padding + border.

### Success Criteria

#### Automated Verification

- [x] All `FrontmatterChips.test.tsx` tests pass:
  `cd skills/visualisation/visualise/frontend && npm run test -- FrontmatterChips`
- [x] TypeScript compiles:
  `cd skills/visualisation/visualise/frontend && npm run typecheck`
- [x] Production build succeeds:
  `cd skills/visualisation/visualise/frontend && npm run build`

#### Manual Verification

- [ ] Open a notes file with frontmatter omitting `status`/`date`/
  `author` entirely (or with all three missing) and confirm the H1
  no longer sits flush against the divider — the subtitle slot
  retains visible vertical space.
- [ ] Open a detail page for a doc with all three chips and confirm
  the H1-to-divider distance is unchanged from before this work
  (i.e. the `min-height` is a floor, not a ceiling).
- [ ] **Side-by-side height comparison**: open a zero-chip page and a
  one-chip page in adjacent browser tabs. In each tab's devtools,
  capture `document.querySelector('[data-testid="frontmatter-chips"]')
  .getBoundingClientRect().height`. The values should be visually
  close (the empty container's `1lh` is an approximation of a
  populated chip's height, not an exact match). If the discrepancy is
  visually obvious (>4px difference is the suggested threshold), file
  a follow-up to switch to a chip-metric-derived `calc(...)`.
- [ ] On a zero-chip page, confirm the empty container carries
  `aria-hidden="true"` (devtools accessibility inspector) and that
  screen-reader navigation skips it.

---

## Phase 3: 12-Doc-Kind Parameterised Regression Armour

### Overview

Add a parameterised test that iterates `DOC_TYPE_KEYS` with fixture
frontmatter containing one non-canonical extra key per kind,
asserting that key never appears as a chip. This phase enforces
the last AC (corpus-wide verification). The whitespace-trim
behaviour was pulled forward into Phase 1 (folded into
`pickCanonical`'s dedup pass for structural simplicity), so this
phase is test-only and adds no production code.

This phase is regression armour, not a failing-first TDD step:
once Phase 1 lands, the parameterised assertions already pass.
Their value is locking in the contract so any future relaxation
of the whitelist fails the suite. If Phase 3 is implemented before
Phase 1, the assertions correctly fail as a TDD red — both
orderings are valid.

### Test-First Changes

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.test.tsx`

**Add** a new describe block at the end of the file:

```tsx
import { DOC_TYPE_KEYS, type DocTypeKey } from '../../api/types'

describe('12-doc-kind corpus verification', () => {
  // One extra non-canonical key per kind, plus the three canonical
  // keys, ensures the whitelist excludes anything outside the set
  // regardless of which kind's fixture shape is used. Per-kind
  // extras are deliberately illustrative — they need not match
  // ADR-0033's per-artifact-type extras list exactly; the contract
  // being verified is "any non-canonical key is excluded", not "this
  // specific key shape per kind". Typed as Record<DocTypeKey, ...>
  // so a future doc-kind addition fails typecheck rather than
  // runtime-destructuring undefined.
  const NON_CANONICAL_PER_KIND: Record<DocTypeKey, [string, unknown]> = {
    'decisions':           ['adr', '0033'],
    'work-items':          ['priority', 'medium'],
    'plans':               ['type', 'plan'],
    'research':            ['git_commit', 'abc123def456'],
    'plan-reviews':        ['verdict', 'APPROVE'],
    'pr-reviews':          ['verdict', 'APPROVE'],
    'work-item-reviews':   ['verdict', 'APPROVE'],
    'validations':         ['result', 'pass'],
    'notes':               ['tags', ['note']],
    'pr-descriptions':     ['pr_number', 123],
    'design-gaps':         ['file_path', 'some/path.md'],
    'design-inventories':  ['last_updated_by', 'Someone'],
    'templates':           ['template_for', 'work-items'],
  }

  it.each(DOC_TYPE_KEYS)('kind "%s" never renders the extra non-canonical key as a chip', (kind) => {
    const [extraKey, extraValue] = NON_CANONICAL_PER_KIND[kind]
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{
          status: 'draft',
          date: '2026-04-05',
          author: 'Toby Clemson',
          [extraKey]: extraValue,
        } as Record<string, unknown>}
      />,
    )
    const labels = Array.from(container.querySelectorAll('[aria-label]'))
      .map((el) => el.getAttribute('aria-label'))
    // Exactly three canonical chips, no chip carrying the extra key.
    expect(labels).toEqual([
      'status: draft', 'date: 2026-04-05', 'author: Toby Clemson',
    ])
    expect(labels.some((l) => l?.startsWith(`${extraKey}:`))).toBe(false)
  })
})
```

The `DOC_TYPE_KEYS` import is a `readonly` 13-element array
(includes `'templates'` per `api/types.ts:30`'s
`VIRTUAL_DOC_TYPE_KEYS`). The work item's AC enumerates 12 real
kinds; including `'templates'` in the parameterised test is harmless
(`FrontmatterChips` has no per-kind branching, so all rows exercise
the same code path) and guarantees future kind additions are
auto-covered by the typed `Record<DocTypeKey, ...>` fixture.

Run the suite. If Phase 1 is already in place, all 13 parameterised
assertions **pass** immediately — their job is regression armour,
not a TDD red. If Phase 3 is implemented standalone before Phase 1,
all 13 assertions correctly fail because the whitelist isn't yet
enforced. Either ordering is valid.

### Implementation Changes

None. Phase 3 is test-only.

### Success Criteria

#### Automated Verification

- [x] All `FrontmatterChips.test.tsx` tests pass including the
  parameterised 13-kind matrix:
  `cd skills/visualisation/visualise/frontend && npm run test -- FrontmatterChips`
- [x] TypeScript compiles:
  `cd skills/visualisation/visualise/frontend && npm run typecheck`
- [x] Production build succeeds:
  `cd skills/visualisation/visualise/frontend && npm run build`
- [x] Full frontend test suite still passes:
  `cd skills/visualisation/visualise/frontend && npm run test`

#### Manual Verification

- [ ] On a real doc with `author: "  "` (whitespace-only),
  manually edit the file to set such a value, reload the page, and
  confirm the `author` chip is omitted (not rendered as a chip with
  visible blank value). The whitespace-trim itself shipped in
  Phase 1; this is a corpus-level confirmation.
- [ ] Sample three of the twelve kinds in the running app and
  visually confirm only canonical chips appear (already covered by
  Phase 1's manual checks).

---

## Testing Strategy

### Unit Tests

`FrontmatterChips.test.tsx` is the primary test file affected. Its
final composition after all three phases:

- **`parsed state`** describe — null/undefined/empty-string filter
  (existing), whitespace-only filter (Phase 1 add),
  non-string smoke test (Phase 1 add).
- **`absent state`** describe — empty container present with
  `aria-hidden="true"` (Phase 2 replace).
- **`malformed state`** describe — unchanged.
- **`CSS source assertions`** describe — `.chip` absent + `.banner`
  present (existing), `min-height: 1lh` on `.chips` (Phase 2 add).
- **`dispatch`** describe — narrowed to `status` only (Phase 1
  delete `verdict`/`result`).
- **`whitelist + canonical order`** describe — Phase 1 add.
- **`whitelist exclusion`** describe — Phase 1 add.
- **`subset ordering`** describe — Phase 1 add.
- **`case-fold dedup precedence`** describe — Phase 1 add
  (null-collision regression + first-non-skipped-wins).
- **`date / author precedence over last_updated mirrors`** describe
  — Phase 1 add.
- **`zero qualifying keys (parsed state)`** describe — Phase 2 add.
- **`12-doc-kind corpus verification`** describe — Phase 3 add.

Deleted from `FrontmatterChips.test.tsx`: source-order describe
(`:114-128`) and AC integration fixtures describe (`:130-174`) —
both contradict the new contract
and have no salvageable assertions.

Deleted entirely: `LibraryDocView.dispatch.test.tsx` — its three
cases assert `verdict-badge` / `result-badge` render through
`FrontmatterChips` for plan-review, work-item-review, and validation
kinds; all false after Phase 1's dispatch narrowing. The tone
contracts those cases pinned (verdict→variant, result→variant)
remain covered at the unit-test layer by `VerdictBadge.test.tsx`
and `ResultBadge.test.tsx`.

### Integration Tests

No new integration tests required. `LibraryDocView.dispatch.test.tsx`
is reduced in Phase 1: its three cases assert verdict/result badge
rendering through `FrontmatterChips`, which Phase 1 removes. The
tone contracts those cases pinned are already covered at the unit
layer by `VerdictBadge.test.tsx` and `ResultBadge.test.tsx`, so the
file is deleted in Phase 1 rather than reworked.

### Manual Testing Steps

Pre-condition: dev server running via
`cd skills/visualisation/visualise/frontend && npm run dev`.

1. Open a work-item detail page (e.g. `0084`); confirm three chips
   in order status → date → author. Inspect to confirm no
   `priority`/`tags`/`id`/`kind` chips.
2. Open a research detail page (e.g. this plan's research doc);
   confirm date + author chips only (no status); inspect to confirm
   no `git_commit`/`branch`/`change_id` chips.
3. Open a plan-review detail page; confirm only the canonical
   chips appear and `verdict` is shown in the frontmatter table
   (not the chip strip).
4. Open a design-inventory detail page; confirm `date`/`author`
   chips show creation-anchored values, not `last_updated`/
   `last_updated_by` values.
5. Open a notes file with no `status`/`date`/`author` (or
   manually create one); confirm the H1 has visible vertical space
   below it before the divider.
6. Use the browser inspector on the zero-chip page: confirm the
   `.chips` element is present, has zero children, and reports a
   non-zero `getBoundingClientRect().height`.
7. Edit a doc's frontmatter to set `author: "   "` (whitespace
   only); reload; confirm the `author` chip is omitted.

## Performance Considerations

None. `pickCanonical` iterates a fixed three-element canonical list
plus the input frontmatter (typically <20 keys); this is strictly
less work than the current `Object.entries(...).filter(...)` plus
unbounded iteration. No memoisation needed.

## Migration Notes

No data migration. No backwards-compatibility shim. The change is
purely a render-layer narrowing; existing frontmatter files do not
need editing. After the change, keys like `verdict`/`priority`/
`tags` continue to live in YAML and continue to render in the
`FrontmatterTable` — they just stop appearing as chips in the
subtitle slot.

## References

- Work item: `meta/work/0084-detail-page-chip-strip-cap.md`
- Research: `meta/research/codebase/2026-05-26-0084-detail-page-chip-strip-cap.md`
- Work item review: `meta/reviews/work/0084-detail-page-chip-strip-cap-review-1.md`
- Schema authority: `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`
- Source design-gap: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Prototype inventory: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md`
- Implementation surface: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`
- Co-located CSS: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.module.css`
- Co-located test: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.test.tsx`
- Sole consumer: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:96-100`
- Subtitle slot: `skills/visualisation/visualise/frontend/src/components/Page/Page.tsx:32-34`
- Subtitle styles (line-height inheritance): `skills/visualisation/visualise/frontend/src/components/Page/Page.module.css:52-60`
- Doc-kind enumeration: `skills/visualisation/visualise/frontend/src/api/types.ts:4-14`
