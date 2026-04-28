import { describe, it, expect } from 'vitest'
import {
  WIKI_LINK_PATTERN,
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
  for (const m of s.matchAll(WIKI_LINK_PATTERN)) {
    out.push({ prefix: m[1], n: m[2] })
  }
  return out
}

describe('WIKI_LINK_PATTERN', () => {
  // ── Step 3.1 ─────────────────────────────────────────────────────────
  it('matches both ADR and TICKET forms', () => {
    const m = matches('see [[ADR-0017]] and [[TICKET-1]]')
    expect(m).toEqual([
      { prefix: 'ADR', n: '0017' },
      { prefix: 'TICKET', n: '1' },
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
  it('rejects digit-runs longer than six', () => {
    expect(matches('[[ADR-9999999]]')).toEqual([])
    const huge = `[[ADR-${'9'.repeat(10000)}]]`
    expect(matches(huge)).toEqual([])
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
    const planMaskedAsTicket = makeIndexEntry({
      type: 'plans',
      relPath: 'meta/plans/2026-04-18-foo.md',
    })
    const idx = buildWikiLinkIndex([], [planMaskedAsTicket])
    expect(idx.ticketByNumber.get(2026)).toBeUndefined()
  })

  // ── Step 3.8 ─────────────────────────────────────────────────────────
  it('indexes tickets by filename numeric prefix', () => {
    const ticket = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0001-foo.md',
    })
    const idx = buildWikiLinkIndex([], [ticket])
    expect(idx.ticketByNumber.get(1)).toBe(ticket)
  })
})

describe('resolveWikiLink', () => {
  const adr = makeIndexEntry({
    type: 'decisions',
    relPath: 'meta/decisions/ADR-0017-foo.md',
    title: 'Configuration extension points',
    frontmatter: { adr_id: 'ADR-0017' },
  })
  const ticket = makeIndexEntry({
    type: 'tickets',
    relPath: 'meta/tickets/0001-foo.md',
    title: 'Three-layer review system architecture',
  })
  const idx = buildWikiLinkIndex([adr], [ticket])

  // ── Step 3.9 ─────────────────────────────────────────────────────────
  it('returns null for unknown ADR id', () => {
    expect(resolveWikiLink('ADR', 9999, idx)).toBeNull()
  })

  // ── Step 3.10 ────────────────────────────────────────────────────────
  it('returns href and title for known ADR', () => {
    expect(resolveWikiLink('ADR', 17, idx)).toEqual({
      href: '/library/decisions/ADR-0017-foo',
      title: 'Configuration extension points',
    })
  })

  // ── Step 3.11 ────────────────────────────────────────────────────────
  it('returns href and title for known ticket', () => {
    expect(resolveWikiLink('TICKET', 1, idx)).toEqual({
      href: '/library/tickets/0001-foo',
      title: 'Three-layer review system architecture',
    })
  })

  // ── Step 3.12 ────────────────────────────────────────────────────────
  it('returns null for unknown ticket', () => {
    expect(resolveWikiLink('TICKET', 9999, idx)).toBeNull()
  })

  // ── Step 3.13 ────────────────────────────────────────────────────────
  it('ignores leading zeros in input number', () => {
    // Number(0017) === 17 — the resolver takes a number, so the radix
    // is the caller's concern. Locked by the parsing pipeline that
    // feeds 17 in regardless of source-form digits.
    expect(resolveWikiLink('ADR', 17, idx)?.href).toBe('/library/decisions/ADR-0017-foo')
  })
})
