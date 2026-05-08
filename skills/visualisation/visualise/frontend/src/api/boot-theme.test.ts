import { describe, it, expect, beforeEach } from 'vitest'
import { applyBootAttributes, BOOT_SCRIPT_SOURCE } from './boot-theme'

function makeDoc(): Document {
  document.documentElement.removeAttribute('data-theme')
  document.documentElement.removeAttribute('data-font')
  return document
}

function fakeStorage(items: Record<string, string>): Storage {
  return {
    getItem: (k) => (k in items ? items[k] : null),
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    length: 0,
  }
}

const throwingStorage: Storage = {
  getItem: () => { throw new DOMException('private mode', 'SecurityError') },
  setItem: () => {},
  removeItem: () => {},
  clear: () => {},
  key: () => null,
  length: 0,
}

describe('applyBootAttributes', () => {
  beforeEach(() => {
    document.documentElement.removeAttribute('data-theme')
    document.documentElement.removeAttribute('data-font')
  })

  it('writes data-theme when storage has a valid theme entry', () => {
    applyBootAttributes({
      doc: makeDoc(),
      storage: fakeStorage({ 'ac-theme': 'dark' }),
      matchPrefersDark: () => false,
    })
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })

  it('writes data-font when storage has a valid font-mode entry', () => {
    applyBootAttributes({
      doc: makeDoc(),
      storage: fakeStorage({ 'ac-font-mode': 'mono' }),
      matchPrefersDark: () => false,
    })
    expect(document.documentElement.getAttribute('data-font')).toBe('mono')
  })

  it('leaves data-theme unset when storage has no theme entry', () => {
    applyBootAttributes({
      doc: makeDoc(),
      storage: fakeStorage({}),
      matchPrefersDark: () => true,
    })
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
  })

  it('leaves data-font unset when storage has no font-mode entry', () => {
    applyBootAttributes({
      doc: makeDoc(),
      storage: fakeStorage({}),
      matchPrefersDark: () => false,
    })
    expect(document.documentElement.hasAttribute('data-font')).toBe(false)
  })

  it('rejects invalid stored theme values and leaves attribute unset', () => {
    applyBootAttributes({
      doc: makeDoc(),
      storage: fakeStorage({ 'ac-theme': 'midnight' }),
      matchPrefersDark: () => false,
    })
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
  })

  it('does not throw when storage.getItem throws (private mode)', () => {
    expect(() => applyBootAttributes({
      doc: makeDoc(),
      storage: throwingStorage,
      matchPrefersDark: () => false,
    })).not.toThrow()
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
    expect(document.documentElement.hasAttribute('data-font')).toBe(false)
  })

  it('font-mode failure does not overwrite a successfully-applied data-theme', () => {
    const partialFailureStorage: Storage = {
      getItem: (k) => {
        if (k === 'ac-theme') return 'dark'
        throw new DOMException('partial failure', 'SecurityError')
      },
      setItem: () => {}, removeItem: () => {}, clear: () => {},
      key: () => null, length: 0,
    }
    applyBootAttributes({
      doc: makeDoc(),
      storage: partialFailureStorage,
      matchPrefersDark: () => false,
    })
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    expect(document.documentElement.hasAttribute('data-font')).toBe(false)
  })
})

// PARITY SUITE — guarantees BOOT_SCRIPT_SOURCE (the string that ships)
// behaves identically to applyBootAttributes (the function that's
// directly tested above).
function runBootScript(opts: {
  storage: Storage | null
  resetDoc?: boolean
}): void {
  if (opts.resetDoc !== false) {
    document.documentElement.removeAttribute('data-theme')
    document.documentElement.removeAttribute('data-font')
  }
  // eslint-disable-next-line @typescript-eslint/no-implied-eval
  const fn = new Function('document', 'localStorage', BOOT_SCRIPT_SOURCE) as
    (doc: Document, storage: Storage | null) => void
  fn(document, opts.storage)
}

describe('BOOT_SCRIPT_SOURCE parity with applyBootAttributes', () => {
  beforeEach(() => {
    document.documentElement.removeAttribute('data-theme')
    document.documentElement.removeAttribute('data-font')
  })

  it('writes data-theme=dark when storage has ac-theme=dark', () => {
    runBootScript({ storage: fakeStorage({ 'ac-theme': 'dark' }) })
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })

  it('writes data-font=mono when storage has ac-font-mode=mono', () => {
    runBootScript({ storage: fakeStorage({ 'ac-font-mode': 'mono' }) })
    expect(document.documentElement.getAttribute('data-font')).toBe('mono')
  })

  it('leaves data-theme unset when storage is empty', () => {
    runBootScript({ storage: fakeStorage({}) })
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
  })

  it('rejects invalid stored theme values', () => {
    runBootScript({ storage: fakeStorage({ 'ac-theme': 'midnight' }) })
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
  })

  it('does not throw when storage.getItem throws (private mode)', () => {
    expect(() => runBootScript({ storage: throwingStorage })).not.toThrow()
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
    expect(document.documentElement.hasAttribute('data-font')).toBe(false)
  })

  it('font-mode failure does not overwrite a successfully-applied data-theme', () => {
    const partialFailureStorage: Storage = {
      getItem: (k) => {
        if (k === 'ac-theme') return 'dark'
        throw new DOMException('partial failure', 'SecurityError')
      },
      setItem: () => {}, removeItem: () => {}, clear: () => {},
      key: () => null, length: 0,
    }
    runBootScript({ storage: partialFailureStorage })
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    expect(document.documentElement.hasAttribute('data-font')).toBe(false)
  })
})
