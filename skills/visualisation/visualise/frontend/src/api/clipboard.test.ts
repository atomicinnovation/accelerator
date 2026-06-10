import { describe, it, expect, vi, afterEach } from 'vitest'
import { copyText } from './clipboard'

const originalClipboard = Object.getOwnPropertyDescriptor(navigator, 'clipboard')

function setClipboard(value: unknown) {
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value,
  })
}

afterEach(() => {
  if (originalClipboard) {
    Object.defineProperty(navigator, 'clipboard', originalClipboard)
  } else {
    // jsdom has no clipboard by default; remove what the test installed.
    delete (navigator as { clipboard?: unknown }).clipboard
  }
  vi.restoreAllMocks()
})

describe('copyText', () => {
  it('uses navigator.clipboard.writeText when available', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    setClipboard({ writeText })
    const exec = vi.fn().mockReturnValue(true)
    document.execCommand = exec

    const ok = await copyText('meta/work/0080.md')

    expect(ok).toBe(true)
    expect(writeText).toHaveBeenCalledWith('meta/work/0080.md')
    expect(exec).not.toHaveBeenCalled()
  })

  it('falls back to execCommand when navigator.clipboard is absent', async () => {
    setClipboard(undefined)
    const exec = vi.fn().mockReturnValue(true)
    document.execCommand = exec

    const ok = await copyText('a/b.md')

    expect(ok).toBe(true)
    expect(exec).toHaveBeenCalledWith('copy')
  })

  it('falls back to execCommand when writeText rejects', async () => {
    const writeText = vi.fn().mockRejectedValue(new Error('denied'))
    setClipboard({ writeText })
    const exec = vi.fn().mockReturnValue(true)
    document.execCommand = exec

    const ok = await copyText('a/b.md')

    expect(ok).toBe(true)
    expect(writeText).toHaveBeenCalled()
    expect(exec).toHaveBeenCalledWith('copy')
  })

  it('returns false when both the Clipboard API and execCommand fail', async () => {
    setClipboard(undefined)
    const exec = vi.fn().mockReturnValue(false)
    document.execCommand = exec

    const ok = await copyText('a/b.md')

    expect(ok).toBe(false)
  })
})
