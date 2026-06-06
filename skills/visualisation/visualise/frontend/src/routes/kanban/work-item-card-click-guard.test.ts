import { describe, it, expect } from 'vitest'
import { shouldSuppressClick } from './WorkItemCard'

// Supplementary decision check for the A2 click guard. The authoritative oracle
// (drag-then-release does not navigate; a plain click navigates) is the E2E
// test — jsdom cannot reproduce the PointerSensor 5px activation or the
// synthetic post-drag click, so the ref-toggle/clear timing is verified there.
describe('shouldSuppressClick', () => {
  it('suppresses while a drag is active', () => {
    expect(shouldSuppressClick(true, false)).toBe(true)
  })

  it('suppresses the synthetic click just after a drag ended', () => {
    expect(shouldSuppressClick(false, true)).toBe(true)
  })

  it('passes a genuine click through when no drag is/was in progress', () => {
    expect(shouldSuppressClick(false, false)).toBe(false)
  })
})
