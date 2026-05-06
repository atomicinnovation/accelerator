export function parseHex(hex: string): { r: number; g: number; b: number } {
  const h = hex.replace('#', '')
  if (h.length === 3) {
    return {
      r: parseInt(h[0] + h[0], 16),
      g: parseInt(h[1] + h[1], 16),
      b: parseInt(h[2] + h[2], 16),
    }
  }
  return {
    r: parseInt(h.slice(0, 2), 16),
    g: parseInt(h.slice(2, 4), 16),
    b: parseInt(h.slice(4, 6), 16),
  }
}

export function parseRgba(value: string): { r: number; g: number; b: number; a: number } {
  const m = value.match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)(?:\s*,\s*([\d.]+))?\s*\)/)
  if (!m) throw new Error(`parseRgba: cannot parse "${value}"`)
  return {
    r: parseInt(m[1], 10),
    g: parseInt(m[2], 10),
    b: parseInt(m[3], 10),
    a: m[4] !== undefined ? parseFloat(m[4]) : 1,
  }
}

export function composeOverSurface(fg: string, surfaceHex: string): string {
  const s = parseHex(surfaceHex)
  let fr: number, fg_g: number, fb: number, fa: number
  if (fg.startsWith('#')) {
    const p = parseHex(fg)
    fr = p.r; fg_g = p.g; fb = p.b; fa = 1
  } else {
    const p = parseRgba(fg)
    fr = p.r; fg_g = p.g; fb = p.b; fa = p.a
  }
  const r = Math.round(fa * fr + (1 - fa) * s.r)
  const g = Math.round(fa * fg_g + (1 - fa) * s.g)
  const b = Math.round(fa * fb + (1 - fa) * s.b)
  return '#' + [r, g, b].map((v) => v.toString(16).padStart(2, '0')).join('')
}

function linearize(c: number): number {
  const n = c / 255
  return n <= 0.04045 ? n / 12.92 : Math.pow((n + 0.055) / 1.055, 2.4)
}

function relativeLuminance({ r, g, b }: { r: number; g: number; b: number }): number {
  return 0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
}

export function contrastRatio(fg: string, bg: string): number {
  const l1 = relativeLuminance(parseHex(fg))
  const l2 = relativeLuminance(parseHex(bg))
  const lighter = Math.max(l1, l2)
  const darker = Math.min(l1, l2)
  return (lighter + 0.05) / (darker + 0.05)
}

export function contrastRatioComposed(
  fgRgbaOrHex: string,
  bgHex: string,
  surfaceHex: string,
): number {
  return contrastRatio(composeOverSurface(fgRgbaOrHex, surfaceHex), bgHex)
}
