---
date: "2026-05-23T09:30:00+01:00"
type: plan
producer: create-plan
work-item: "meta/work/0081-status-badge-component.md"
status: done
id: "2026-05-22-0081-status-badge-component"
title: "0081 — StatusBadge / VerdictBadge / ResultBadge Implementation Plan"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-23T09:30:00+01:00"
last_updated_by: Toby Clemson
revision: "0077b04e78a1"
repository: "ticket-management"
relates_to: ["work-item:0081", "codebase-research:2026-05-22-0081-status-badge-component", "adr:ADR-0007", "codebase-research:2026-05-21-0078-detail-page-frontmatter-table"]
---

# 0081 — StatusBadge / VerdictBadge / ResultBadge Implementation Plan

## Overview

Decompose `FrontmatterChips` into a chip-list renderer plus a small
family of per-key chip components: a generic `FrontmatterChip` and
three thin tone-aware wrappers — `StatusBadge`, `VerdictBadge`,
`ResultBadge` — that compose `FrontmatterChip` directly with their
vocabulary-specific mapping. The chip-list renderer dispatches each
frontmatter key to the right wrapper (`status`→`StatusBadge`,
`verdict`→`VerdictBadge`, `result`→`ResultBadge`, everything
else→`FrontmatterChip`). `Chip` is extended to accept and forward a
named `data-testid` prop so each chip carries an observable hook for
tests, aligning with the codebase's existing `data-testid` convention.

Three separate vocabulary mappings (`statusToVariant`,
`verdictToVariant`, `resultToVariant`) instead of one unified mapping
keep vocabularies clean: a stray `status: "approve"` stays neutral
(rather than incidentally resolving to green via shared sets), and
each vocabulary can evolve independently as the corpus changes.

## Current State Analysis

- `components/FrontmatterChips/FrontmatterChips.tsx` (48 lines) bundles
  three concerns: discriminated-state handling, per-key chip rendering
  with formatting + aria-label, and a one-key status-tone branch
  (`FrontmatterChips.tsx:36-44`).
- `api/status-variant.ts` (27 lines) exports `statusToChipVariant` and
  `isStatusKey`. The status mapping covers ~22 normalised values
  across GREEN / INDIGO / AMBER / RED sets, with a `normalise()` step
  that lowercases and strips whitespace / underscores / hyphens /
  slashes (`status-variant.ts:3-13`).
- `statusToChipVariant` has four consumers:
  - `components/FrontmatterChips/FrontmatterChips.tsx:2`
  - `components/FilterPill/FilterPill.tsx:4`
  - `routes/library/LibraryTypeView.tsx:21`
  - `routes/lifecycle/LifecycleClusterView.tsx:14`
- `Chip` primitive (`components/Chip/Chip.tsx`) ships six variants
  (`neutral` / `indigo` / `green` / `amber` / `red` / `violet`) and two
  sizes. Its props destructure spreads `...rest` but only plucks
  `aria-label`; `data-testid` is not forwarded today.
- `data-testid` is already the established observability hook in the
  codebase (used in `Chip.test.tsx`, `ActivityFeed`, `FilterPill`,
  `Popover`); this story extends `Chip` to forward it on the root
  element rather than introducing a new convention.
- All three "surfaces" named by 0081 (validation, plan-review,
  work-item-review) share `routes/library/LibraryDocView.tsx`, routed
  by `library/$type/$fileSlug` (`router.ts:109-112`).
- Verdict / result emission today:
  - Plan review (`skills/planning/review-plan/SKILL.md:417-428`):
    `verdict: APPROVE | REVISE | COMMENT`.
  - Work-item review (`skills/work/review-work-item/SKILL.md:352-363`):
    `verdict: APPROVE | REVISE | COMMENT`.
  - Validation (`skills/planning/validate-plan/SKILL.md:131-142`):
    `result: pass | partial | fail` and `status: complete` — **not**
    `verdict:`.
  - PR review (`skills/github/review-pr/SKILL.md:460`):
    `APPROVE | REQUEST_CHANGES | COMMENT`. PR-review pages aren't a
    surface 0081 touches, but `REQUEST_CHANGES` is retained
    defensively in `verdictToVariant` so future / shared consumers
    get sensible colouring.

## Desired End State

- `FrontmatterChips` is a thin chip-list renderer that walks
  frontmatter in source order and dispatches by key.
- `FrontmatterChip` renders one key/value pair as a `Chip` with
  default `variant="neutral"`, optional variant override,
  `aria-label`, and `data-testid="frontmatter-chip"`. Owns value
  formatting (array join, object stringify, coercion). No domain
  tone logic.
- `StatusBadge` / `VerdictBadge` / `ResultBadge` are one-liner
  wrappers that compose `FrontmatterChip` directly, computing the
  variant from their vocabulary's mapping
  (`variant={statusToVariant(value)}`) and supplying their own
  `testId` string. No shared generic intermediary.
- Three vocabulary mappings live in `api/`:
  - `statusToVariant` — unchanged status sets, renamed from
    `statusToChipVariant`.
  - `verdictToVariant` — handles `APPROVE` / `REVISE` /
    `REQUEST_CHANGES` / `COMMENT`. `APPROVE` / `REVISE` / `COMMENT`
    cover plan-review and work-item-review; `REQUEST_CHANGES`
    additionally covers PR-review (not a 0081 surface) without
    crowding the result vocabulary.
  - `resultToVariant` — `pass` / `partial` / `fail` only. Authoritative
    home for validation result values; `verdictToVariant` does not
    duplicate these tokens.
- A shared `normaliseValue` helper underpins all three mappings so
  case-insensitive separator-insensitive matching is enforced
  consistently.
- `Chip` accepts an explicit `data-testid` prop and forwards it onto
  its root element alongside the component-managed `data-variant` /
  `data-size`.
- Verdict and result chips on plan-review, work-item-review, and
  validation detail pages carry semantic colour; status chips on
  library type and lifecycle cluster pages render with the same
  tones as before.

### Key Discoveries

- All four 0081 "surfaces" share `LibraryDocView` — one render path.
- The live status helper covers RED-set values and
  separator-insensitive normalisation that the work-item AC table
  understates; the full mapping must be preserved.
- Validation pages emit `result:`, not `verdict:`. Recognising
  `result` at the chip-list renderer avoids an upstream skill change
  and corpus migration.
- Per-vocabulary mappings prevent cross-leakage: `status: "approve"`
  stays neutral; `result: "REQUEST_CHANGES"` stays neutral.

## What We're NOT Doing

- Not changing `FrontmatterTable` (the body-side key/value table from
  0078) — its value cells stay neutral until 0084 / a follow-up.
- Not renaming `result` → `verdict` in `skills/planning/validate-plan/SKILL.md`
  or migrating existing validation outputs.
- Not splitting array-valued frontmatter into per-element chips.
- Not wiring PR-review pages (not a surface 0081 covers).
- Not adding new chip variants to `Chip` — the six shipped variants
  suffice.
- Not capping the chip strip at four chips (0084's concern).
- Not changing `Chip.module.css` or the CSS-module-driven variant
  rendering.

## Implementation Approach

Seven sequential phases, each TDD (tests written before
implementation), each ending with the full test suite green and the
app in a working state.

Dependency shape:

- Phase 1 (Chip `data-testid` forwarding) — depends on nothing.
- Phase 2 (rename `statusToChipVariant` + extract `normaliseValue`)
  — depends on nothing.
- Phase 3 (`FrontmatterChip` component) — depends on Phase 1.
- Phase 4 (`StatusBadge` thin wrapper) — depends on Phases 2, 3.
- Phase 5 (`VerdictBadge` + `verdictToVariant`) — depends on Phases
  2, 3. Independent of Phase 4.
- Phase 6 (`ResultBadge` + `resultToVariant`) — depends on Phases 2,
  3. Independent of Phases 4, 5.
- Phase 7 (refactor `FrontmatterChips` dispatch) — depends on
  Phases 3, 4, 5, 6.

Phases 4, 5, 6 can be parallelised against each other after Phase 3
lands. The visible verdict / result tone behavioural change only
lands at Phase 7.

All test commands target the visualiser frontend workspace:
`skills/visualisation/visualise/frontend/`.

---

## Phase 1: Chip primitive — `data-testid` forwarding

### Overview

Extend `Chip` with a named `data-testid` prop forwarded to its root
`<span>`, aligning with the codebase's existing observability
convention (`data-testid` is used in `Chip.test.tsx`, `ActivityFeed`,
`FilterPill`, and `Popover` today). Pure additive change to the 0038
API; existing call sites keep working unchanged. No index signature,
no reserved-attribute filter, no runtime allowlist — the prop is
explicit and cannot collide with the component-managed `data-variant`
and `data-size`.

### Changes Required

#### 1. Chip tests (written first)

**File**: `skills/visualisation/visualise/frontend/src/components/Chip/Chip.test.tsx`
**Changes**: Add a new `describe('data-testid forwarding', …)` block.

```tsx
describe('data-testid forwarding', () => {
  it('forwards data-testid to the root element', () => {
    const { container } = render(
      <Chip variant="neutral" data-testid="status-badge">x</Chip>,
    )
    expect(container.querySelector('[data-testid="status-badge"]')).not.toBeNull()
  })

  it('omits the data-testid attribute when none is passed', () => {
    const { container } = render(<Chip variant="neutral">x</Chip>)
    expect(container.querySelector('[data-testid]')).toBeNull()
  })
})
```

#### 2. Chip implementation

**File**: `skills/visualisation/visualise/frontend/src/components/Chip/Chip.tsx`
**Changes**: Add `'data-testid'?: string` to `ChipProps` and forward
it onto the root `<span>` alongside the existing `aria-label`.

```tsx
import type { ReactNode } from 'react'
import styles from './Chip.module.css'

export type ChipVariant = 'neutral' | 'indigo' | 'green' | 'amber' | 'red' | 'violet'
export type ChipSize = 'sm' | 'md'

export interface ChipProps {
  variant: ChipVariant
  size?: ChipSize
  leading?: ReactNode
  'aria-label'?: string
  'data-testid'?: string
  children: ReactNode
}

export function Chip({ variant, size = 'sm', leading, children, ...rest }: ChipProps) {
  const hasLeading = leading !== undefined && leading !== null && leading !== false
  return (
    <span
      className={styles.chip}
      data-variant={variant}
      data-size={size}
      aria-label={rest['aria-label']}
      data-testid={rest['data-testid']}
    >
      {hasLeading && (
        <span className={styles.leading} data-slot="leading">{leading}</span>
      )}
      <span className={styles.label}>{children}</span>
    </span>
  )
}
```

### Success Criteria

#### Automated Verification

- [x] New Chip `data-testid` forwarding tests pass: `npm --prefix skills/visualisation/visualise/frontend run test -- Chip`
- [x] Existing Chip variant / size / aria-label / CSS tests still pass
- [x] Type checking passes: `npm --prefix skills/visualisation/visualise/frontend run typecheck`
- [x] Full suite green: `npm --prefix skills/visualisation/visualise/frontend run test`

#### Manual Verification

- [ ] No visual regression on pages that use `<Chip>` directly.

---

## Phase 2: Rename `statusToChipVariant` and extract `normaliseValue`

### Overview

Rename the existing tone helper from `statusToChipVariant` to
`statusToVariant` in `api/status-variant.ts` (sets unchanged — still
status-only). The `Chip` qualifier is dropped to give the three
vocabulary mappings a uniform suffix (`statusToVariant` /
`verdictToVariant` / `resultToVariant`); the return type `ChipVariant`
is already imported at the top of every file, so the function name
doesn't need to repeat the qualifier. Extract the `normalise` helper
into a shared `api/normalise-value.ts` so subsequent vocabulary
mappings can reuse it. Migrate all four importers in one change. Pure
rename + extract; no behaviour change.

### Changes Required

#### 1. Tone-helper tests (rename references first, suite stays green)

**File**: `skills/visualisation/visualise/frontend/src/api/status-variant.test.ts`
**Changes**: Replace all `statusToChipVariant` references with
`statusToVariant` in `import` and assertions. All existing test cases
preserved verbatim except the `internal invariants` block, which
asserts the round-trip property of `normaliseValue` instead of a
hard-coded regex — this is the actual invariant being tested ("every
key is in its normalised form") and is robust to digit-bearing future
values:

```ts
import { normaliseValue } from './normalise-value'

describe('internal invariants', () => {
  it('all Set keys are in normalised form', () => {
    expect(__SETS_FOR_TEST).toBeDefined()
    expect(__SETS_FOR_TEST.length).toBeGreaterThan(0)
    for (const s of __SETS_FOR_TEST) {
      expect(s.size).toBeGreaterThan(0)
      for (const k of s) {
        expect(normaliseValue(k)).toBe(k)
      }
    }
  })
})
```

`isStatusKey` describe block retained (removed in Phase 7).

#### 2. Extract shared `normaliseValue`

**File**: `skills/visualisation/visualise/frontend/src/api/normalise-value.ts`
**Changes**: New file.

```ts
export function normaliseValue(value: unknown): string {
  if (typeof value !== 'string') return ''
  return value.trim().toLowerCase().replace(/[\s_\-/]+/g, '')
}
```

**File**: `skills/visualisation/visualise/frontend/src/api/normalise-value.test.ts`
**Changes**: New file.

```ts
import { describe, expect, it } from 'vitest'
import { normaliseValue } from './normalise-value'

describe('normaliseValue', () => {
  it('lowercases', () => {
    expect(normaliseValue('Accepted')).toBe('accepted')
  })

  it('strips leading/trailing whitespace', () => {
    expect(normaliseValue('  Accepted  ')).toBe('accepted')
  })

  it.each([
    ['in progress', 'inprogress'],
    ['in_progress', 'inprogress'],
    ['in-progress', 'inprogress'],
    ['approve w/ changes', 'approvewchanges'],
    ['REQUEST_CHANGES', 'requestchanges'],
  ])('treats whitespace, underscore, hyphen, slash equivalently (%s)', (input, expected) => {
    expect(normaliseValue(input)).toBe(expected)
  })

  it.each([null, undefined, 42, true, ['a', 'b'], { x: 1 }])(
    'returns empty string for non-string inputs (%s)', (input) => {
      expect(normaliseValue(input as unknown)).toBe('')
    },
  )

  describe('unicode scope (documented limitation)', () => {
    // ASCII whitespace, underscore, hyphen-minus, and slash are collapsed.
    // Unicode-typographic separators (en-dash, em-dash, Unicode hyphen)
    // are NOT collapsed — frontmatter authors must use ASCII separators.
    it.each([
      ['en–dash', 'en–dash'],
      ['em—dash', 'em—dash'],
      ['unicode‐hyphen', 'unicode‐hyphen'],
    ])('does not collapse typographic separator in %s', (input, expected) => {
      expect(normaliseValue(input)).toBe(expected)
    })
  })
})
```

#### 3. Status-helper implementation

**File**: `skills/visualisation/visualise/frontend/src/api/status-variant.ts`
**Changes**: Rename export, import `normaliseValue` from the shared
helper. Sets unchanged. `isStatusKey` retained (removed in Phase 7).

```ts
import type { ChipVariant } from '../components/Chip/Chip'
import { normaliseValue } from './normalise-value'

const GREEN = new Set(['done', 'complete', 'accepted', 'approved', 'implemented', 'final', 'shipped'])
const INDIGO = new Set(['inprogress', 'reviewed', 'ready', 'active', 'proposed', 'live'])
const AMBER = new Set(['approvewithchanges', 'approvewchanges', 'review', 'revised'])
const RED = new Set(['blocked', 'rejected', 'deprecated', 'superseded', 'abandoned'])

export const __SETS_FOR_TEST = [GREEN, INDIGO, AMBER, RED]

export function statusToVariant(value: unknown): ChipVariant {
  const key = normaliseValue(value)
  if (GREEN.has(key)) return 'green'
  if (INDIGO.has(key)) return 'indigo'
  if (AMBER.has(key)) return 'amber'
  if (RED.has(key)) return 'red'
  return 'neutral'
}

export function isStatusKey(key: string): boolean {
  return key.trim().toLowerCase() === 'status'
}
```

#### 4. Importer migration

**File**: `skills/visualisation/visualise/frontend/src/components/FilterPill/FilterPill.tsx`
- Line 4: `import { statusToVariant } from '../../api/status-variant'`
- Line 151: `<Chip variant={statusToVariant(option.id)}>`

**File**: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.tsx`
- Line 14: `import { statusToVariant } from '../../api/status-variant'`
- Line 139: `<Chip variant={statusToVariant(status)}>{status}</Chip>`

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx`
- Line 21: `import { statusToVariant } from '../../api/status-variant'`
- Line 252: `<Chip variant={statusToVariant(statusValue(entry))}>`

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`
- Line 2: `import { statusToVariant, isStatusKey } from '../../api/status-variant'`
- Line 38: `const variant = isStatusKey(key) ? statusToVariant(value) : 'neutral'`

#### 5. Adjust downstream test description

**File**: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx`
- Line 185: update test description from `'…the variant from statusToChipVariant'` to `'…the variant from statusToVariant'`. Assertions unchanged.

### Success Criteria

#### Automated Verification

- [x] `normaliseValue` tests pass: `npm --prefix skills/visualisation/visualise/frontend run test -- normalise-value`
- [x] Renamed status-variant tests pass: `npm --prefix skills/visualisation/visualise/frontend run test -- status-variant`
- [x] LibraryTypeView / LifecycleClusterView / FilterPill / FrontmatterChips tests still pass
- [x] Type checking passes
- [x] Full suite green

#### Manual Verification

- [ ] Library type, lifecycle cluster, filter pill, and frontmatter chip pages render with the same tones as before.

---

## Phase 3: `FrontmatterChip` component (generic per-key chip)

### Overview

Create a `FrontmatterChip` that owns per-key chip rendering: value
formatting, aria-label, default `neutral` variant, optional variant
override, and a `data-testid` observable hook
(default `"frontmatter-chip"`). Net-new files; not yet consumed.

### Changes Required

#### 1. FrontmatterChip tests (written first)

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChip/FrontmatterChip.test.tsx`
**Changes**: New file.

```tsx
import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { FrontmatterChip } from './FrontmatterChip'

describe('FrontmatterChip', () => {
  describe('default rendering', () => {
    it('renders the value text', () => {
      render(<FrontmatterChip name="status" value="Accepted" />)
      expect(screen.getByText('Accepted')).toBeInTheDocument()
    })

    it('does not render the key in visible content', () => {
      render(<FrontmatterChip name="status" value="Accepted" />)
      expect(screen.queryByText(/^status:/i)).toBeNull()
    })

    it('attaches aria-label of "${key}: ${value}"', () => {
      const { container } = render(
        <FrontmatterChip name="status" value="Accepted" />,
      )
      expect(container.querySelector('[aria-label="status: Accepted"]')).not.toBeNull()
    })

    it('defaults to variant="neutral" regardless of key', () => {
      const { container } = render(
        <FrontmatterChip name="status" value="Accepted" />,
      )
      expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
    })

    it('renders with data-testid="frontmatter-chip" by default', () => {
      const { container } = render(
        <FrontmatterChip name="date" value="2026-05-22" />,
      )
      expect(container.querySelector('[data-testid="frontmatter-chip"]')).not.toBeNull()
    })
  })

  describe('variant override', () => {
    it('uses an explicitly-passed variant', () => {
      const { container } = render(
        <FrontmatterChip name="status" value="x" variant="green" />,
      )
      expect(container.querySelector('[data-variant="green"]')).not.toBeNull()
    })
  })

  describe('testId override', () => {
    it('renders a caller-provided data-testid value', () => {
      const { container } = render(
        <FrontmatterChip name="status" value="x" testId="custom-badge" />,
      )
      expect(container.querySelector('[data-testid="custom-badge"]')).not.toBeNull()
    })
  })

  describe('value formatting', () => {
    it('joins array values with ", " (and the join appears in aria-label)', () => {
      const { container } = render(
        <FrontmatterChip name="tags" value={['design', 'frontend']} />,
      )
      expect(screen.getByText('design, frontend')).toBeInTheDocument()
      expect(container.querySelector('[aria-label="tags: design, frontend"]')).not.toBeNull()
    })

    it('JSON-stringifies plain objects (visible text and aria-label parity)', () => {
      const { container } = render(<FrontmatterChip name="meta" value={{ x: 1 }} />)
      expect(screen.getByText('{"x":1}')).toBeInTheDocument()
      expect(container.querySelector('[aria-label="meta: {\\"x\\":1}"]')).not.toBeNull()
    })

    it('coerces booleans to strings', () => {
      render(<FrontmatterChip name="archived" value={false} />)
      expect(screen.getByText('false')).toBeInTheDocument()
    })

    it('coerces numbers to strings', () => {
      render(<FrontmatterChip name="version" value={0} />)
      expect(screen.getByText('0')).toBeInTheDocument()
    })
  })
})
```

#### 2. FrontmatterChip implementation

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChip/FrontmatterChip.tsx`
**Changes**: New file.

```tsx
import { Chip, type ChipVariant } from '../Chip/Chip'

export interface FrontmatterChipProps {
  name: string
  value: unknown
  variant?: ChipVariant
  testId?: string
}

function formatChipValue(value: unknown): string {
  if (Array.isArray(value)) return value.join(', ')
  if (typeof value === 'object' && value !== null) return JSON.stringify(value)
  return String(value)
}

export function FrontmatterChip({
  name,
  value,
  variant = 'neutral',
  testId = 'frontmatter-chip',
}: FrontmatterChipProps) {
  const text = formatChipValue(value)
  return (
    <Chip
      variant={variant}
      aria-label={`${name}: ${text}`}
      data-testid={testId}
    >
      {text}
    </Chip>
  )
}
```

### Success Criteria

#### Automated Verification

- [x] New FrontmatterChip tests pass: `npm --prefix skills/visualisation/visualise/frontend run test -- FrontmatterChip`
- [x] Type checking passes
- [x] Full suite green

#### Manual Verification

- [ ] No UI change (component is not yet consumed).

---

## Phase 4: `StatusBadge` thin wrapper

### Overview

Create `StatusBadge` as a one-liner thin wrapper that composes
`FrontmatterChip` directly, computing the variant from
`statusToVariant(value)`. Owns the status-vocabulary test matrix
end-to-end through the wrapper (the full live mapping, including the
RED set and normalisation reach).

### Changes Required

#### 1. StatusBadge tests (written first)

**File**: `skills/visualisation/visualise/frontend/src/components/StatusBadge/StatusBadge.test.tsx`
**Changes**: New file.

```tsx
import { describe, expect, it } from 'vitest'
import { render } from '@testing-library/react'
import { StatusBadge } from './StatusBadge'

describe('StatusBadge', () => {
  describe('observable hook', () => {
    it('renders with data-testid="status-badge"', () => {
      const { container } = render(
        <StatusBadge value="Accepted" />,
      )
      expect(container.querySelector('[data-testid="status-badge"]')).not.toBeNull()
    })
  })

  describe('aria-label (inherited via composition)', () => {
    it('renders aria-label of "${key}: ${value}"', () => {
      const { container } = render(
        <StatusBadge value="Accepted" />,
      )
      expect(container.querySelector('[aria-label="status: Accepted"]')).not.toBeNull()
    })
  })

  describe('status vocabulary — full live mapping preserved', () => {
    it.each([
      ['Accepted', 'green'], ['Done', 'green'], ['complete', 'green'],
      ['approved', 'green'], ['implemented', 'green'], ['final', 'green'], ['shipped', 'green'],
      ['In progress', 'indigo'], ['Proposed', 'indigo'], ['live', 'indigo'],
      ['active', 'indigo'], ['reviewed', 'indigo'], ['ready', 'indigo'],
      ['Approve w/ changes', 'amber'], ['review', 'amber'], ['revised', 'amber'],
      ['blocked', 'red'], ['rejected', 'red'], ['deprecated', 'red'],
      ['superseded', 'red'], ['abandoned', 'red'],
    ])('status %s → %s', (value, expected) => {
      const { container } = render(<StatusBadge value={value} />)
      expect(container.querySelector(`[data-variant="${expected}"]`)).not.toBeNull()
    })
  })

  describe('normalisation reach (separator insensitivity)', () => {
    it.each([
      ['IN_PROGRESS', 'indigo'], ['in-progress', 'indigo'], ['in_progress', 'indigo'],
    ])('status %s → %s', (value, expected) => {
      const { container } = render(<StatusBadge value={value} />)
      expect(container.querySelector(`[data-variant="${expected}"]`)).not.toBeNull()
    })
  })

  describe('neutral fallback', () => {
    it.each([
      'Todo', 'absent', 'SomeUnknownValue', '2026-05-21', '',
    ])('status %s → neutral', (value) => {
      const { container } = render(<StatusBadge value={value} />)
      expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
    })

    it.each([null, undefined, 42, true, ['a'], { x: 1 }] as const)(
      'non-string value → neutral', (value) => {
        const { container } = render(<StatusBadge value={value} />)
        expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
      },
    )
  })

  describe('vocabulary isolation', () => {
    it.each([
      ['approve', 'neutral'], ['pass', 'neutral'], ['fail', 'neutral'],
      ['REVISE', 'neutral'], ['REQUEST_CHANGES', 'neutral'], ['partial', 'neutral'],
    ])('verdict-shaped value %s under status → %s (cross-leakage prevented)', (value, expected) => {
      const { container } = render(<StatusBadge value={value} />)
      expect(container.querySelector(`[data-variant="${expected}"]`)).not.toBeNull()
    })
  })
})
```

#### 2. StatusBadge implementation

**File**: `skills/visualisation/visualise/frontend/src/components/StatusBadge/StatusBadge.tsx`
**Changes**: New file.

```tsx
import { FrontmatterChip } from '../FrontmatterChip/FrontmatterChip'
import { statusToVariant } from '../../api/status-variant'

export interface StatusBadgeProps {
  value: unknown
}

export function StatusBadge({ value }: StatusBadgeProps) {
  return (
    <FrontmatterChip
      name="status"
      value={value}
      variant={statusToVariant(value)}
      testId="status-badge"
    />
  )
}
```

### Success Criteria

#### Automated Verification

- [x] New StatusBadge tests pass: `npm --prefix skills/visualisation/visualise/frontend run test -- StatusBadge`
- [x] Type checking passes
- [x] Full suite green

#### Manual Verification

- [ ] No UI change (StatusBadge not wired into FrontmatterChips yet).

---

## Phase 5: `VerdictBadge` + `verdictToVariant`

### Overview

Add a new vocabulary mapping `verdictToVariant` that covers the
verdict vocabulary (`APPROVE` / `REVISE` / `REQUEST_CHANGES` /
`COMMENT`). `APPROVE` / `REVISE` / `COMMENT` are emitted by plan-review
and work-item-review; `REQUEST_CHANGES` is emitted by PR-review (not a
0081 surface, but the natural home for the token). Validation result
tokens (`pass` / `fail` / `partial`) deliberately live in
`resultToVariant` only — see Current State Analysis: validation emits
`result:`, not `verdict:`. Create `VerdictBadge` as a thin wrapper
composing `FrontmatterChip` directly. Independent of Phases 4 and 6.

### Changes Required

#### 1. verdictToVariant tests (written first)

**File**: `skills/visualisation/visualise/frontend/src/api/verdict-variant.test.ts`
**Changes**: New file.

```ts
import { describe, expect, it } from 'vitest'
import { verdictToVariant, __SETS_FOR_TEST } from './verdict-variant'
import { normaliseValue } from './normalise-value'

describe('verdictToVariant', () => {
  describe('internal invariants', () => {
    it('all Set keys are in normalised form', () => {
      expect(__SETS_FOR_TEST).toBeDefined()
      expect(__SETS_FOR_TEST.length).toBeGreaterThan(0)
      for (const s of __SETS_FOR_TEST) {
        expect(s.size).toBeGreaterThan(0)
        for (const k of s) {
          expect(normaliseValue(k)).toBe(k)
        }
      }
    })
  })


  describe('plan-review vocabulary', () => {
    it.each([
      ['APPROVE', 'green'],
      ['REVISE', 'amber'],
      ['REQUEST_CHANGES', 'red'],
      ['COMMENT', 'neutral'],
    ])('maps %s → %s', (v, expected) => {
      expect(verdictToVariant(v)).toBe(expected)
    })
  })

  describe('case insensitivity', () => {
    it.each([
      ['approve', 'green'], ['Approve', 'green'], ['APPROVE', 'green'],
      ['revise', 'amber'], ['Revise', 'amber'],
      ['request_changes', 'red'], ['Request_Changes', 'red'],
    ])('maps %s → %s', (v, expected) => {
      expect(verdictToVariant(v)).toBe(expected)
    })
  })

  describe('normalisation reach', () => {
    it.each([
      ['REQUEST_CHANGES', 'red'],
      ['request-changes', 'red'],
      ['request changes', 'red'],
      ['request/changes', 'red'],
    ])('maps %s → %s', (v, expected) => {
      expect(verdictToVariant(v)).toBe(expected)
    })
  })

  describe('neutral fallback', () => {
    it.each(['xyz', '', 'undecided', 'maybe'])(
      'unmapped %s → neutral', (v) => expect(verdictToVariant(v)).toBe('neutral'),
    )

    it.each([null, undefined, 42, true, ['a'], { x: 1 }] as const)(
      'non-string %s → neutral', (v) => expect(verdictToVariant(v as unknown)).toBe('neutral'),
    )
  })

  describe('vocabulary isolation', () => {
    it.each([
      ['done', 'neutral'], ['accepted', 'neutral'], ['blocked', 'neutral'],
      ['in progress', 'neutral'], ['rejected', 'neutral'],
    ])('status-shaped %s under verdict → %s', (v, expected) => {
      expect(verdictToVariant(v)).toBe(expected)
    })

    it.each([
      ['pass', 'neutral'], ['fail', 'neutral'], ['partial', 'neutral'],
    ])('result-shaped %s under verdict → %s (handled by resultToVariant only)', (v, expected) => {
      expect(verdictToVariant(v)).toBe(expected)
    })
  })
})
```

#### 2. verdictToVariant implementation

**File**: `skills/visualisation/visualise/frontend/src/api/verdict-variant.ts`
**Changes**: New file.

```ts
import type { ChipVariant } from '../components/Chip/Chip'
import { normaliseValue } from './normalise-value'

const GREEN = new Set(['approve'])
const AMBER = new Set(['revise'])
const RED = new Set(['requestchanges'])
// COMMENT and unknown values fall through to neutral. Result-vocabulary
// tokens (pass / partial / fail) live in result-variant.ts; validation
// emits `result:`, not `verdict:`, so they do not need to be handled here.

export const __SETS_FOR_TEST = [GREEN, AMBER, RED]

export function verdictToVariant(value: unknown): ChipVariant {
  const key = normaliseValue(value)
  if (GREEN.has(key)) return 'green'
  if (AMBER.has(key)) return 'amber'
  if (RED.has(key)) return 'red'
  return 'neutral'
}
```

#### 3. VerdictBadge tests (written before component)

**File**: `skills/visualisation/visualise/frontend/src/components/VerdictBadge/VerdictBadge.test.tsx`
**Changes**: New file.

```tsx
import { describe, expect, it } from 'vitest'
import { render } from '@testing-library/react'
import { VerdictBadge } from './VerdictBadge'

describe('VerdictBadge', () => {
  describe('observable hook', () => {
    it('renders with data-testid="verdict-badge"', () => {
      const { container } = render(
        <VerdictBadge value="APPROVE" />,
      )
      expect(container.querySelector('[data-testid="verdict-badge"]')).not.toBeNull()
    })
  })

  describe('aria-label (inherited via composition)', () => {
    it('renders aria-label of "${key}: ${value}"', () => {
      const { container } = render(
        <VerdictBadge value="APPROVE" />,
      )
      expect(container.querySelector('[aria-label="verdict: APPROVE"]')).not.toBeNull()
    })
  })

  describe('plan-review verdict vocabulary', () => {
    it.each([
      ['APPROVE', 'green'], ['REVISE', 'amber'],
      ['REQUEST_CHANGES', 'red'], ['COMMENT', 'neutral'],
    ])('verdict %s → %s', (value, expected) => {
      const { container } = render(<VerdictBadge value={value} />)
      expect(container.querySelector(`[data-variant="${expected}"]`)).not.toBeNull()
    })
  })

  describe('case insensitivity', () => {
    it.each([
      ['approve', 'green'], ['Approve', 'green'], ['APPROVE', 'green'],
    ])('verdict %s → green', (value) => {
      const { container } = render(<VerdictBadge value={value} />)
      expect(container.querySelector('[data-variant="green"]')).not.toBeNull()
    })
  })

  describe('neutral fallback', () => {
    it.each(['xyz', '', 'undecided'])('unmapped %s → neutral', (value) => {
      const { container } = render(<VerdictBadge value={value} />)
      expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
    })

    it.each([null, undefined, 42, true] as const)('non-string → neutral', (value) => {
      const { container } = render(<VerdictBadge value={value} />)
      expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
    })
  })

  describe('vocabulary isolation', () => {
    it.each(['done', 'accepted', 'blocked', 'rejected'])(
      'status-shaped %s under verdict → neutral', (value) => {
        const { container } = render(<VerdictBadge value={value} />)
        expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
      },
    )

    it.each(['pass', 'fail', 'partial'])(
      'result-shaped %s under verdict → neutral (handled by ResultBadge only)', (value) => {
        const { container } = render(<VerdictBadge value={value} />)
        expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
      },
    )
  })
})
```

#### 4. VerdictBadge implementation

**File**: `skills/visualisation/visualise/frontend/src/components/VerdictBadge/VerdictBadge.tsx`
**Changes**: New file.

```tsx
import { FrontmatterChip } from '../FrontmatterChip/FrontmatterChip'
import { verdictToVariant } from '../../api/verdict-variant'

export interface VerdictBadgeProps {
  value: unknown
}

export function VerdictBadge({ value }: VerdictBadgeProps) {
  return (
    <FrontmatterChip
      name="verdict"
      value={value}
      variant={verdictToVariant(value)}
      testId="verdict-badge"
    />
  )
}
```

### Success Criteria

#### Automated Verification

- [x] verdictToVariant tests pass: `npm --prefix skills/visualisation/visualise/frontend run test -- verdict-variant`
- [x] VerdictBadge tests pass: `npm --prefix skills/visualisation/visualise/frontend run test -- VerdictBadge`
- [x] Type checking passes
- [x] Full suite green

#### Manual Verification

- [ ] No UI change (not wired yet).

---

## Phase 6: `ResultBadge` + `resultToVariant`

### Overview

Add `resultToVariant` covering the validation result vocabulary
(`pass` / `partial` / `fail`) and a thin `ResultBadge` wrapper
composing `FrontmatterChip` directly. The mapping is intentionally
separate from `verdictToVariant` (despite the current vocabulary
overlap) so the two can diverge as the corpus evolves. Independent
of Phases 4 and 5.

### Changes Required

#### 1. resultToVariant tests (written first)

**File**: `skills/visualisation/visualise/frontend/src/api/result-variant.test.ts`
**Changes**: New file.

```ts
import { describe, expect, it } from 'vitest'
import { resultToVariant, __SETS_FOR_TEST } from './result-variant'
import { normaliseValue } from './normalise-value'

describe('resultToVariant', () => {
  describe('internal invariants', () => {
    it('all Set keys are in normalised form', () => {
      expect(__SETS_FOR_TEST).toBeDefined()
      expect(__SETS_FOR_TEST.length).toBeGreaterThan(0)
      for (const s of __SETS_FOR_TEST) {
        expect(s.size).toBeGreaterThan(0)
        for (const k of s) {
          expect(normaliseValue(k)).toBe(k)
        }
      }
    })
  })

  describe('validation result vocabulary', () => {
    it.each([
      ['pass', 'green'], ['partial', 'amber'], ['fail', 'red'],
    ])('maps %s → %s', (v, expected) => {
      expect(resultToVariant(v)).toBe(expected)
    })
  })

  describe('case insensitivity', () => {
    it.each([
      ['pass', 'green'], ['Pass', 'green'], ['PASS', 'green'],
      ['fail', 'red'], ['FAIL', 'red'],
      ['partial', 'amber'], ['Partial', 'amber'],
    ])('maps %s → %s', (v, expected) => {
      expect(resultToVariant(v)).toBe(expected)
    })
  })

  describe('neutral fallback', () => {
    it.each(['xyz', '', 'undecided', 'unknown'])(
      'unmapped %s → neutral', (v) => expect(resultToVariant(v)).toBe('neutral'),
    )

    it.each([null, undefined, 42, true, ['a'], { x: 1 }] as const)(
      'non-string → neutral', (v) => expect(resultToVariant(v as unknown)).toBe('neutral'),
    )
  })

  describe('vocabulary isolation', () => {
    it.each([
      ['APPROVE', 'neutral'], ['REVISE', 'neutral'], ['REQUEST_CHANGES', 'neutral'],
      ['COMMENT', 'neutral'], ['done', 'neutral'], ['accepted', 'neutral'],
    ])('non-result-vocab %s → %s', (v, expected) => {
      expect(resultToVariant(v)).toBe(expected)
    })
  })
})
```

#### 2. resultToVariant implementation

**File**: `skills/visualisation/visualise/frontend/src/api/result-variant.ts`
**Changes**: New file.

```ts
import type { ChipVariant } from '../components/Chip/Chip'
import { normaliseValue } from './normalise-value'

const GREEN = new Set(['pass'])
const AMBER = new Set(['partial'])
const RED = new Set(['fail'])

export const __SETS_FOR_TEST = [GREEN, AMBER, RED]

export function resultToVariant(value: unknown): ChipVariant {
  const key = normaliseValue(value)
  if (GREEN.has(key)) return 'green'
  if (AMBER.has(key)) return 'amber'
  if (RED.has(key)) return 'red'
  return 'neutral'
}
```

#### 3. ResultBadge tests (written before component)

**File**: `skills/visualisation/visualise/frontend/src/components/ResultBadge/ResultBadge.test.tsx`
**Changes**: New file.

```tsx
import { describe, expect, it } from 'vitest'
import { render } from '@testing-library/react'
import { ResultBadge } from './ResultBadge'

describe('ResultBadge', () => {
  describe('observable hook', () => {
    it('renders with data-testid="result-badge"', () => {
      const { container } = render(
        <ResultBadge value="pass" />,
      )
      expect(container.querySelector('[data-testid="result-badge"]')).not.toBeNull()
    })
  })

  describe('aria-label (inherited via composition)', () => {
    it('renders aria-label of "${key}: ${value}"', () => {
      const { container } = render(
        <ResultBadge value="pass" />,
      )
      expect(container.querySelector('[aria-label="result: pass"]')).not.toBeNull()
    })
  })

  describe('result vocabulary', () => {
    it.each([
      ['pass', 'green'], ['partial', 'amber'], ['fail', 'red'],
    ])('result %s → %s', (value, expected) => {
      const { container } = render(<ResultBadge value={value} />)
      expect(container.querySelector(`[data-variant="${expected}"]`)).not.toBeNull()
    })
  })

  describe('case insensitivity', () => {
    it.each([
      ['pass', 'green'], ['Pass', 'green'], ['PASS', 'green'],
      ['fail', 'red'], ['FAIL', 'red'],
    ])('result %s → %s', (value, expected) => {
      const { container } = render(<ResultBadge value={value} />)
      expect(container.querySelector(`[data-variant="${expected}"]`)).not.toBeNull()
    })
  })

  describe('neutral fallback', () => {
    it.each(['xyz', '', 'undecided'])('unmapped %s → neutral', (value) => {
      const { container } = render(<ResultBadge value={value} />)
      expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
    })

    it.each([null, undefined, 42, true] as const)('non-string → neutral', (value) => {
      const { container } = render(<ResultBadge value={value} />)
      expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
    })
  })

  describe('vocabulary isolation', () => {
    it.each([
      'APPROVE', 'REVISE', 'REQUEST_CHANGES', 'COMMENT',
      'done', 'accepted', 'blocked',
    ])('non-result-vocab %s under result → neutral', (value) => {
      const { container } = render(<ResultBadge value={value} />)
      expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
    })
  })
})
```

#### 4. ResultBadge implementation

**File**: `skills/visualisation/visualise/frontend/src/components/ResultBadge/ResultBadge.tsx`
**Changes**: New file.

```tsx
import { FrontmatterChip } from '../FrontmatterChip/FrontmatterChip'
import { resultToVariant } from '../../api/result-variant'

export interface ResultBadgeProps {
  value: unknown
}

export function ResultBadge({ value }: ResultBadgeProps) {
  return (
    <FrontmatterChip
      name="result"
      value={value}
      variant={resultToVariant(value)}
      testId="result-badge"
    />
  )
}
```

### Success Criteria

#### Automated Verification

- [x] resultToVariant tests pass: `npm --prefix skills/visualisation/visualise/frontend run test -- result-variant`
- [x] ResultBadge tests pass: `npm --prefix skills/visualisation/visualise/frontend run test -- ResultBadge`
- [x] Type checking passes
- [x] Full suite green

#### Manual Verification

- [ ] No UI change (not wired yet).

---

## Phase 7: Refactor `FrontmatterChips` to dispatch by key

### Overview

Rewrite `FrontmatterChips` in place as a thin chip-list renderer:
walks frontmatter in source order, filters absent values, and
dispatches each entry by key to the appropriate wrapper —
`status`→`StatusBadge`, `verdict`→`VerdictBadge`,
`result`→`ResultBadge`, everything else→`FrontmatterChip`. Drop the
now-unused `isStatusKey` export. Reshuffle tests: keep list-level
cases, drop cases covered downstream, add new dispatch + integration
cases. This phase produces the visible behavioural change.

### Changes Required

#### 1. FrontmatterChips tests (rewritten first)

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.test.tsx`
**Changes**: Replace per-chip cases with dispatch cases. Keep
list-level cases.

KEEP (with minor selector adjustments where useful):

- `'skips null and undefined values'`
- `'skips empty-string values'`
- `'renders a chip for each non-null frontmatter value'` (count
  `[data-testid]` rather than `[data-variant]`)
- `'absent state renders nothing'`
- `'malformed state renders the role="alert" banner'`
- `'CSS source assertions'`

REMOVE (covered by FrontmatterChip / StatusBadge / VerdictBadge /
ResultBadge tests):

- `'renders the status field with the colour-coded variant'`
- `'colour-codes status case-insensitively (Status, STATUS)'`
- `'renders non-status fields with variant="neutral"'`
- `'does not render keys (e.g. the literal text "status:") in visible content'`
- `'attaches an aria-label of "${key}: ${value}"'`
- `'renders the value text'`
- `'renders boolean and numeric values as strings'`
- `'joins array values with ", "'`

ADD:

```tsx
describe('dispatch', () => {
  it('dispatches the status key to StatusBadge', () => {
    const { container } = render(
      <FrontmatterChips state="parsed" frontmatter={{ status: 'Accepted' }} />,
    )
    expect(container.querySelector('[data-testid="status-badge"]')).not.toBeNull()
  })

  it('dispatches the verdict key to VerdictBadge', () => {
    const { container } = render(
      <FrontmatterChips state="parsed" frontmatter={{ verdict: 'APPROVE' }} />,
    )
    expect(container.querySelector('[data-testid="verdict-badge"]')).not.toBeNull()
  })

  it('dispatches the result key to ResultBadge', () => {
    const { container } = render(
      <FrontmatterChips state="parsed" frontmatter={{ result: 'pass' }} />,
    )
    expect(container.querySelector('[data-testid="result-badge"]')).not.toBeNull()
  })

  it('dispatches non-tone keys to FrontmatterChip', () => {
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{ priority: 'medium', date: '2026-05-22' }}
      />,
    )
    expect(container.querySelectorAll('[data-testid="frontmatter-chip"]').length).toBe(2)
    expect(container.querySelector('[data-testid="status-badge"]')).toBeNull()
    expect(container.querySelector('[data-testid="verdict-badge"]')).toBeNull()
    expect(container.querySelector('[data-testid="result-badge"]')).toBeNull()
  })

  it.each([
    ['Status', 'status-badge'], ['STATUS', 'status-badge'],
    ['Verdict', 'verdict-badge'], ['VERDICT', 'verdict-badge'],
    ['Result', 'result-badge'], ['RESULT', 'result-badge'],
  ])('dispatches case-folded key "%s" to %s', (key, expectedComponent) => {
    const { container } = render(
      <FrontmatterChips state="parsed" frontmatter={{ [key]: 'pass' }} />,
    )
    expect(container.querySelector(`[data-testid="${expectedComponent}"]`)).not.toBeNull()
  })
})

describe('source order', () => {
  it('preserves frontmatter source order in rendered output', () => {
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{ verdict: 'APPROVE', status: 'Accepted', priority: 'medium' }}
      />,
    )
    const labels = Array.from(container.querySelectorAll('[aria-label]'))
      .map((el) => el.getAttribute('aria-label'))
    expect(labels).toEqual([
      'verdict: APPROVE', 'status: Accepted', 'priority: medium',
    ])
  })
})

describe('AC integration fixtures', () => {
  it('plan-review-shaped document: source order, components, variants', () => {
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{
          status: 'Accepted',
          verdict: 'APPROVE',
          priority: 'medium',
          tags: ['design', 'frontend'],
        }}
      />,
    )
    const chips = Array.from(container.querySelectorAll('[data-testid]'))
    expect(chips).toHaveLength(4)
    expect(chips.map((el) => el.getAttribute('data-testid'))).toEqual([
      'status-badge', 'verdict-badge', 'frontmatter-chip', 'frontmatter-chip',
    ])
    expect(chips.map((el) => el.getAttribute('data-variant'))).toEqual([
      'green', 'green', 'neutral', 'neutral',
    ])
  })

  it('validation-shaped document: source order, components, variants', () => {
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{
          status: 'complete',
          result: 'pass',
          priority: 'medium',
          tags: ['validation'],
        }}
      />,
    )
    const chips = Array.from(container.querySelectorAll('[data-testid]'))
    expect(chips).toHaveLength(4)
    expect(chips.map((el) => el.getAttribute('data-testid'))).toEqual([
      'status-badge', 'result-badge', 'frontmatter-chip', 'frontmatter-chip',
    ])
    expect(chips.map((el) => el.getAttribute('data-variant'))).toEqual([
      'green', 'green', 'neutral', 'neutral',
    ])
  })
})
```

#### 2. FrontmatterChips implementation

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`
**Changes**: Rewrite the render path. Drop the `statusToVariant` /
`isStatusKey` imports and the `formatChipValue` helper.

```tsx
import type { ComponentType } from 'react'
import { FrontmatterChip } from '../FrontmatterChip/FrontmatterChip'
import { StatusBadge } from '../StatusBadge/StatusBadge'
import { VerdictBadge } from '../VerdictBadge/VerdictBadge'
import { ResultBadge } from '../ResultBadge/ResultBadge'
import styles from './FrontmatterChips.module.css'

type FrontmatterChipsProps =
  | { state: 'absent' }
  | { state: 'malformed' }
  | { state: 'parsed'; frontmatter: Record<string, unknown> }

interface BadgeProps {
  value: unknown
}

// Keys MUST be lowercase: `badgeFor` lowercases the lookup key so any
// case variant in frontmatter (`status` / `Status` / `STATUS`) routes
// to the same badge. This deliberately tolerates corpus casing drift
// rather than surfacing it as a neutral chip.
const BADGE_FOR_KEY: Record<string, ComponentType<BadgeProps>> = {
  status: StatusBadge,
  verdict: VerdictBadge,
  result: ResultBadge,
}

function badgeFor(key: string): ComponentType<BadgeProps> | null {
  return BADGE_FOR_KEY[key.trim().toLowerCase()] ?? null
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

  const entries = Object.entries(props.frontmatter).filter(([, v]) => {
    if (v === null || v === undefined) return false
    if (typeof v === 'string' && v === '') return false
    return true
  })

  if (entries.length === 0) return null

  return (
    <div className={styles.chips}>
      {entries.map(([key, value]) => {
        const Badge = badgeFor(key)
        if (Badge) return <Badge key={key} value={value} />
        return <FrontmatterChip key={key} name={key} value={value} />
      })}
    </div>
  )
}
```

#### 3. Drop the `isStatusKey` export

**File**: `skills/visualisation/visualise/frontend/src/api/status-variant.ts`
**Changes**: Delete the `isStatusKey` function.

**File**: `skills/visualisation/visualise/frontend/src/api/status-variant.test.ts`
**Changes**: Delete the `describe('isStatusKey', …)` block.

#### 4. Route-level integration tests (surface ACs)

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.dispatch.test.tsx`
**Changes**: New file. Reuses the `Wrapper` / `MemoryRouter` /
`QueryClientProvider` pattern and the `mockEntry` shape established in
`LibraryDocView.test.tsx`. Mocks `fetchModule.fetchDocs`,
`fetchModule.fetchDocContent`, and `fetchModule.fetchRelated` per
existing test conventions.

The three canonical `DocTypeKey` values for the review surfaces are
`'plan-reviews'`, `'work-item-reviews'`, and `'validations'`
(`src/api/types.ts:4-9`). Each test mounts `LibraryDocView` with the
matching `type` and a frontmatter fixture carrying the relevant key,
then asserts the chip's `data-testid` and `data-variant` reach the DOM
end-to-end through the loader → route → `FrontmatterChips` integration.

```tsx
import { describe, it, expect, vi } from 'vitest'
import { render, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryDocView } from './LibraryDocView'
import * as fetchModule from '../../api/fetch'
import type { IndexEntry } from '../../api/types'
import { MemoryRouter } from '../../test/router-helpers'

function Wrapper({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

const baseEntry: IndexEntry = {
  type: 'plan-reviews',
  path: '/p/meta/reviews/plans/2026-01-01-foo-review-1.md',
  relPath: 'meta/reviews/plans/2026-01-01-foo-review-1.md',
  slug: '2026-01-01-foo-review-1',
  workItemId: null,
  title: 'Foo review',
  frontmatter: {},
  frontmatterState: 'parsed',
  workItemRefs: [],
  mtimeMs: 1_700_000_000_000,
  size: 100,
  etag: 'sha256-a',
  bodyPreview: '',
}

function mockFetches(entry: IndexEntry) {
  vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([entry])
  vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
    content: 'Body.',
    etag: '"sha256-a"',
  })
  vi.spyOn(fetchModule, 'fetchRelated').mockResolvedValue({
    inferredCluster: [],
    declaredOutbound: [],
    declaredInbound: [],
  })
}

describe('LibraryDocView chip dispatch (surface ACs for 0081)', () => {
  it('plan-review document: verdict chip is coloured per plan-review vocabulary', async () => {
    mockFetches({
      ...baseEntry,
      type: 'plan-reviews',
      frontmatter: { verdict: 'APPROVE' },
    })
    const { container } = render(
      <LibraryDocView type="plan-reviews" fileSlug="2026-01-01-foo-review-1" />,
      { wrapper: Wrapper },
    )
    await waitFor(() => {
      const chip = container.querySelector('[data-testid="verdict-badge"]')
      expect(chip).not.toBeNull()
      expect(chip?.getAttribute('data-variant')).toBe('green')
    })
  })

  it('work-item-review document: verdict chip is coloured per plan-review vocabulary', async () => {
    mockFetches({
      ...baseEntry,
      type: 'work-item-reviews',
      relPath: 'meta/reviews/work-items/0042-review-1.md',
      slug: '0042-review-1',
      frontmatter: { verdict: 'REVISE' },
    })
    const { container } = render(
      <LibraryDocView type="work-item-reviews" fileSlug="0042-review-1" />,
      { wrapper: Wrapper },
    )
    await waitFor(() => {
      const chip = container.querySelector('[data-testid="verdict-badge"]')
      expect(chip).not.toBeNull()
      expect(chip?.getAttribute('data-variant')).toBe('amber')
    })
  })

  it('validation document: result chip is coloured per validation vocabulary', async () => {
    mockFetches({
      ...baseEntry,
      type: 'validations',
      relPath: 'meta/validations/2026-01-01-foo-validation-1.md',
      slug: '2026-01-01-foo-validation-1',
      frontmatter: { result: 'pass' },
    })
    const { container } = render(
      <LibraryDocView type="validations" fileSlug="2026-01-01-foo-validation-1" />,
      { wrapper: Wrapper },
    )
    await waitFor(() => {
      const chip = container.querySelector('[data-testid="result-badge"]')
      expect(chip).not.toBeNull()
      expect(chip?.getAttribute('data-variant')).toBe('green')
    })
  })
})
```

These three tests close the gap between the unit-level dispatch tests
(which assert `FrontmatterChips` in isolation) and the surface ACs.
A regression in the loader, doc-content fetch, or `LibraryDocView`
that drops `verdict` / `result` from frontmatter before reaching
`FrontmatterChips` will fail one or more of these tests.

### Success Criteria

#### Automated Verification

- [x] Rewritten FrontmatterChips tests pass: `npm --prefix skills/visualisation/visualise/frontend run test -- FrontmatterChips`
- [x] FrontmatterChip / StatusBadge / VerdictBadge / ResultBadge tests still pass
- [x] status-variant / verdict-variant / result-variant / normalise-value tests still pass
- [x] LibraryTypeView / LifecycleClusterView / FilterPill tests still pass
- [x] New route-level integration tests pass: `npm --prefix skills/visualisation/visualise/frontend run test -- LibraryDocView.dispatch`
- [x] Existing LibraryDocView tests still pass: `npm --prefix skills/visualisation/visualise/frontend run test -- LibraryDocView`
- [x] Type checking passes
- [x] Full suite green: `npm --prefix skills/visualisation/visualise/frontend run test`
- [x] Production build succeeds: `npm --prefix skills/visualisation/visualise/frontend run build`

#### Manual Verification

- [ ] Plan-review detail page: the `verdict` chip is coloured per the
  plan-review vocabulary (APPROVE → green, REVISE → amber,
  COMMENT → neutral) instead of neutral.
- [ ] Work-item-review detail page: `verdict` chip coloured per
  plan-review vocabulary.
- [ ] Validation detail page: the `result` chip is coloured
  (pass → green, fail → red, partial → amber).
- [ ] Library type pages: status chips render with the same tones as
  before.
- [ ] Lifecycle cluster pages: status chips render with the same
  tones as before.
- [ ] Frontmatter chips appear in document source order — open a doc
  whose frontmatter starts with `verdict:` before `status:` and
  confirm the verdict chip renders first.
- [ ] Malformed-frontmatter banner still appears on intentionally
  broken fixtures.
- [ ] Filter pill on the library page renders option chips with the
  same tones as before.

---

## Testing Strategy

### Unit tests

Each phase authors its tests before its implementation (red → green
TDD). Coverage per unit:

- **`Chip`** (Phase 1): `data-testid` forwarding (presence and
  omission). Existing tests preserved.
- **`normaliseValue`** (Phase 2): lowercase, trim, separator
  collapse, non-string handling.
- **`statusToVariant`** (Phase 2): full live status vocabulary
  (GREEN / INDIGO / AMBER / RED), normalisation reach, neutral
  fallback, non-string inputs.
- **`FrontmatterChip`** (Phase 3): default rendering, value
  formatting, neutral default, variant override, testId
  override.
- **`StatusBadge`** (Phase 4): full status vocabulary,
  normalisation, neutral fallback, vocabulary isolation
  (verdict-shaped values stay neutral).
- **`verdictToVariant`** (Phase 5): verdict vocabulary (`APPROVE` /
  `REVISE` / `REQUEST_CHANGES` / `COMMENT`), case insensitivity,
  normalisation, neutral fallback, vocabulary isolation (both
  status-shaped and result-shaped values stay neutral).
- **`VerdictBadge`** (Phase 5): mirrors `verdictToVariant` cases
  through the component.
- **`resultToVariant`** (Phase 6): result vocabulary, case
  insensitivity, neutral fallback, vocabulary isolation.
- **`ResultBadge`** (Phase 6): mirrors `resultToVariant` cases
  through the component.
- **`FrontmatterChips`** (Phase 7): list filtering, dispatch
  (status / verdict / result / other, case-insensitive), source-order
  preservation, absent / malformed states, CSS source assertions, AC
  integration fixture.

### Integration tests

Two integration layers in Phase 7:

- The component-level AC fixture in `FrontmatterChips.test.tsx`
  exercises the full chip-list decomposition and asserts both dispatch
  (correct component per key) and tone (correct variant per value)
  from a literal `frontmatter` prop.
- The route-level dispatch tests in `LibraryDocView.dispatch.test.tsx`
  mount `LibraryDocView` with mocked loader fixtures for each of the
  three review surfaces (`plan-reviews`, `work-item-reviews`,
  `validations`) and assert the chip's `data-testid` and
  `data-variant` reach the DOM through the loader → route →
  `FrontmatterChips` integration. These close the gap between
  component-level dispatch correctness and the work item's
  surface-level ACs.

### Manual testing steps

After Phase 7 lands:

1. `npm --prefix skills/visualisation/visualise/frontend run dev`
2. Navigate to a plan-review document and verify the `verdict` chip
   is coloured.
3. Navigate to a validation document and verify the `result` chip is
   coloured.
4. Navigate to a work-item-review document and verify the `verdict`
   chip is coloured.
5. Navigate to a library type page and verify status chips are
   unchanged.
6. Navigate to a lifecycle cluster view and verify status chips are
   unchanged.
7. Open a document with `verdict:` ordered before `status:` and
   verify the verdict chip renders first.
8. Open a document with intentionally broken frontmatter and verify
   the malformed-frontmatter banner still appears.

## Performance Considerations

None. The refactor is a pure render-path change with constant-time
per-chip dispatch via a three-entry record lookup.

## Migration Notes

- **No data migration.** Validation pages keep emitting `result:`;
  the chip-list renderer dispatches it to `ResultBadge`. No upstream
  skill change or corpus rewrite is required.
- **Single-commit rename in Phase 2.** All four importers of
  `statusToChipVariant` are migrated to `statusToVariant` in the
  same change, so there is no broken window between names.
- **Vocabulary mappings live side-by-side.** Three small files in
  `api/`, all consuming a shared `normaliseValue`. Each vocabulary
  can evolve independently — e.g. validation can grow new result
  states without affecting plan-review verdict semantics.
- **`isStatusKey` deferred deletion.** Retained in Phase 2 because
  `FrontmatterChips` still uses it; deleted in Phase 7 after the
  dispatch refactor.
- **No rollback shim needed.** Each phase ends with the full test
  suite and production build green.

## References

- Work item: `meta/work/0081-status-badge-component.md`
- Research: `meta/research/codebase/2026-05-22-0081-status-badge-component.md`
- Related ADR: `meta/decisions/ADR-0007-divergent-verdict-semantics-for-plan-and-pr-reviews.md`
- Prior research (Chip primitive): `meta/research/codebase/2026-05-15-0038-generic-chip-component.md`
- Prior research (FrontmatterTable): `meta/research/codebase/2026-05-21-0078-detail-page-frontmatter-table.md`
- Component being decomposed: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`
- Canonical status tone helper: `skills/visualisation/visualise/frontend/src/api/status-variant.ts`
- Chip primitive: `skills/visualisation/visualise/frontend/src/components/Chip/Chip.tsx`
- Shared detail-page route: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`
- Plan-review verdict emission: `skills/planning/review-plan/SKILL.md:417-428`
- Work-item-review verdict emission: `skills/work/review-work-item/SKILL.md:352-363`
- Validation `result:` emission: `skills/planning/validate-plan/SKILL.md:131-142`
