import { describe, it, expect } from 'vitest'
import {
  buildWikiLinkPattern,
  buildWikiLinkIndex,
  resolveWikiLink,
} from './wiki-links'
import { makeIndexEntry } from './test-fixtures'

/** Reset the regex's `lastIndex` between matches so global-flag state
 *  doesn't leak across tests. Tests should always create matches via
 *  `[...source.matchAll(regex)]` (which is iterator-based and
 *  state-safe), or call `.exec` once per assertion. */
function matches(s: string): Array<{ prefix: string; n: string }> {
  const out: Array<{ prefix: string; n: string }> = []
  for (const m of s.matchAll(buildWikiLinkPattern(null))) {
    out.push({ prefix: m[1], n: m[2] })
  }
  return out
}

describe('buildWikiLinkPattern', () => {
  // ── Step 3.1 ─────────────────────────────────────────────────────────
  it('matches both ADR and WORK-ITEM forms', () => {
    const m = matches('see [[ADR-0017]] and [[WORK-ITEM-1]]')
    expect(m).toEqual([
      { prefix: 'ADR', n: '0017' },
      { prefix: 'WORK-ITEM', n: '1' },
    ])
  })

  // ── Step 3.2 ─────────────────────────────────────────────────────────
  it('does not match bare numeric form', () => {
    expect(matches('[[0001]]')).toEqual([])
  })

  // ── Step 3.3 ─────────────────────────────────────────────────────────
  it('does not match unknown prefix', () => {
    expect(matches('[[EPIC-0001]]')).toEqual([])
  })

  // ── Step 3.4 ─────────────────────────────────────────────────────────
  it('does not match uppercase-mismatched form', () => {
    expect(matches('[[adr-0001]]')).toEqual([])
    expect(matches('[[Adr-0001]]')).toEqual([])
  })

  // ── Step 3.4b ────────────────────────────────────────────────────────
  // buildWikiLinkPattern uses `\d+` (no upper limit), so long digit
  // runs DO match. The old WIKI_LINK_PATTERN had a {1,6} cap to guard
  // Number.MAX_SAFE_INTEGER — with string-based IDs that concern is gone.
  it('matches digit-runs longer than six (no upper limit)', () => {
    expect(matches('[[ADR-9999999]]')).toEqual([{ prefix: 'ADR', n: '9999999' }])
  })

  // ── Step 3.4c ────────────────────────────────────────────────────────
  it('boundary cases', () => {
    expect(matches('[[ADR-0001]].')).toEqual([{ prefix: 'ADR', n: '0001' }])
    expect(matches('prefix[[ADR-0001]]suffix')).toEqual([
      { prefix: 'ADR', n: '0001' },
    ])
    expect(matches('[[ADR-]]')).toEqual([])
    expect(matches('[[ADR-0001a]]')).toEqual([])
  })

  // ── Project-code tests ────────────────────────────────────────────────
  it('matches project-prefixed work-item ids under a project pattern', () => {
    const pattern = buildWikiLinkPattern('PROJ')
    const text = 'See [[WORK-ITEM-PROJ-0042]] for context'
    const ms = [...text.matchAll(pattern)]
    expect(ms).toHaveLength(1)
    expect(ms[0][2]).toBe('PROJ-0042')
  })

  it('falls back to bare numeric under a project pattern', () => {
    const pattern = buildWikiLinkPattern('PROJ')
    const text = 'See [[WORK-ITEM-0007]] for legacy context'
    const ms = [...text.matchAll(pattern)]
    expect(ms).toHaveLength(1)
    expect(ms[0][2]).toBe('0007')
  })

  it('matches default-pattern work-item ids when no project code is configured', () => {
    const pattern = buildWikiLinkPattern(null)
    const text = 'See [[WORK-ITEM-0042]] and [[ADR-0023]]'
    const ms = [...text.matchAll(pattern)]
    expect(ms).toHaveLength(2)
  })

  it('does not match multi-segment project codes (out of scope)', () => {
    // Pinned negative: the compiler grammar forbids hyphens in project codes.
    // ACME-CORE-0042 is not expected to resolve; ACME pattern matches ACME-CORE
    // as the id (digits portion absent), so no digit match → no result.
    const pattern = buildWikiLinkPattern('ACME')
    const text = 'See [[WORK-ITEM-ACME-CORE-0042]]'
    const ms = [...text.matchAll(pattern)]
    expect(ms).toHaveLength(0)
  })
})

describe('buildWikiLinkIndex', () => {
  // ── Step 3.5 ─────────────────────────────────────────────────────────
  it('indexes ADRs by adr_id when present', () => {
    const adr = makeIndexEntry({
      type: 'decisions',
      relPath: 'meta/decisions/ADR-0017-foo.md',
      frontmatter: { adr_id: 'ADR-0017' },
    })
    const idx = buildWikiLinkIndex([adr], [])
    expect(idx.adrById.get(17)).toBe(adr)
  })

  // ── Step 3.6 ─────────────────────────────────────────────────────────
  it('falls back to filename prefix for ADRs missing adr_id', () => {
    const adr = makeIndexEntry({
      type: 'decisions',
      relPath: 'meta/decisions/ADR-0042-foo.md',
      frontmatter: {},
    })
    const idx = buildWikiLinkIndex([adr], [])
    expect(idx.adrById.get(42)).toBe(adr)
  })

  // ── Step 3.7 ─────────────────────────────────────────────────────────
  it('prefers adr_id over filename when both are present and disagree', () => {
    const adr = makeIndexEntry({
      type: 'decisions',
      relPath: 'meta/decisions/ADR-0042-foo.md',
      frontmatter: { adr_id: 'ADR-0099' },
    })
    const idx = buildWikiLinkIndex([adr], [])
    expect(idx.adrById.get(99)).toBe(adr)
    expect(idx.adrById.get(42)).toBeUndefined()
  })

  // ── Step 3.7b ────────────────────────────────────────────────────────
  it('picks earliest-relPath on duplicate IDs', () => {
    const earlier = makeIndexEntry({
      type: 'decisions',
      relPath: 'meta/decisions/ADR-0017-aaa.md',
      frontmatter: { adr_id: 'ADR-0017' },
    })
    const later = makeIndexEntry({
      type: 'decisions',
      relPath: 'meta/decisions/ADR-0017-zzz.md',
      frontmatter: { adr_id: 'ADR-0017' },
    })
    // Insert in non-lexical order to verify the tie-break is independent of input order.
    const idx = buildWikiLinkIndex([later, earlier], [])
    expect(idx.adrById.get(17)).toBe(earlier)
  })

  // ── Step 3.7c ────────────────────────────────────────────────────────
  it('defensively filters by entry type', () => {
    // A plan mistakenly passed as a work item must not appear in workItemById,
    // even if it has a workItemId set.
    const planMaskedAsWorkItem = makeIndexEntry({
      type: 'plans',
      relPath: 'meta/plans/2026-04-18-foo.md',
      workItemId: '2026',
    })
    const idx = buildWikiLinkIndex([], [planMaskedAsWorkItem])
    expect(idx.workItemById.get('2026')).toBeUndefined()
  })

  // ── Step 3.8 ─────────────────────────────────────────────────────────
  it('indexes work items by workItemId (string key)', () => {
    const workItem = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-foo.md',
      workItemId: '0001',
    })
    const idx = buildWikiLinkIndex([], [workItem])
    expect(idx.workItemById.get('0001')).toBe(workItem)
  })

  it('indexes project-prefixed work items by full string ID', () => {
    const workItem = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/PROJ-0042-foo.md',
      workItemId: 'PROJ-0042',
    })
    const idx = buildWikiLinkIndex([], [workItem])
    expect(idx.workItemById.get('PROJ-0042')).toBe(workItem)
  })
})

describe('resolveWikiLink', () => {
  const adr = makeIndexEntry({
    type: 'decisions',
    relPath: 'meta/decisions/ADR-0017-foo.md',
    title: 'Configuration extension points',
    frontmatter: { adr_id: 'ADR-0017' },
  })
  const workItem = makeIndexEntry({
    type: 'work-items',
    relPath: 'meta/work/0001-foo.md',
    title: 'Three-layer review system architecture',
    workItemId: '0001',
  })
  const idx = buildWikiLinkIndex([adr], [workItem])

  // ── Step 3.9 ─────────────────────────────────────────────────────────
  it('returns null for unknown ADR id', () => {
    expect(resolveWikiLink('ADR', '9999', idx)).toBeNull()
  })

  // ── Step 3.10 ────────────────────────────────────────────────────────
  it('returns href and title for known ADR', () => {
    expect(resolveWikiLink('ADR', '17', idx)).toEqual({
      href: '/library/decisions/ADR-0017-foo',
      title: 'Configuration extension points',
    })
  })

  // ── Step 3.11 ────────────────────────────────────────────────────────
  it('returns href and title for known work item', () => {
    expect(resolveWikiLink('WORK-ITEM', '0001', idx)).toEqual({
      href: '/library/work-items/0001-foo',
      title: 'Three-layer review system architecture',
    })
  })

  // ── Step 3.12 ────────────────────────────────────────────────────────
  it('returns null for unknown work item', () => {
    expect(resolveWikiLink('WORK-ITEM', '9999', idx)).toBeNull()
  })

  // ── Step 3.13 ────────────────────────────────────────────────────────
  it('ADR id string is parsed to integer for lookup (leading zeros transparent)', () => {
    // resolveWikiLink takes a raw string from the regex capture group.
    // ADR lookups parse the string to int internally, so '0017' and '17'
    // both hit the same entry keyed on 17.
    expect(resolveWikiLink('ADR', '0017', idx)?.href).toBe('/library/decisions/ADR-0017-foo')
    expect(resolveWikiLink('ADR', '17', idx)?.href).toBe('/library/decisions/ADR-0017-foo')
  })

  it('resolves a project-prefixed work item via full string id', () => {
    const projItem = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/PROJ-0042-foo.md',
      title: 'Foo work item',
      workItemId: 'PROJ-0042',
    })
    const projIdx = buildWikiLinkIndex([], [projItem])
    expect(resolveWikiLink('WORK-ITEM', 'PROJ-0042', projIdx)).toEqual({
      href: '/library/work-items/PROJ-0042-foo',
      title: 'Foo work item',
    })
  })
})
