/**
 * Short-form elapsed-time ladder shared by `formatMtime` and `formatRelative`.
 * **Precondition**: caller must pass `diffSec >= 0`. Returns `null` only when
 * `diffSec >= 7 * 86400` (caller decides the long-form fallback). Otherwise
 * returns one of `<n>s ago` / `<n>m ago` / `<n>h ago` / `<n>d ago`.
 *
 * The negative-elapsed clamp is intentionally NOT in this helper — callers
 * have divergent semantics (`formatMtime` → 'just now', `formatRelative` →
 * '0s ago') and the divergence belongs at the call site.
 */
function formatElapsedShort(diffSec: number): string | null {
  if (diffSec < 60) return `${diffSec}s ago`
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`
  if (diffSec < 7 * 86400) return `${Math.floor(diffSec / 86400)}d ago`
  return null
}

export function formatMtime(ms: number, now: number = Date.now()): string {
  if (ms <= 0) return '—'
  const diffSec = Math.floor((now - ms) / 1000)
  if (diffSec < 0) return 'just now'
  const short = formatElapsedShort(diffSec)
  if (short !== null) return short
  if (diffSec < 30 * 86400) return `${Math.floor(diffSec / (7 * 86400))}w ago`
  return new Date(ms).toLocaleDateString()
}

/**
 * Activity-feed-flavoured relative formatter. Diverges from `formatMtime`
 * at both ends: clamps negative elapsed to '0s ago' (vs 'just now') and
 * keeps `<n>d ago` indefinitely (vs weeks/locale flip at 7d).
 */
/** Renders a frontmatter `date:` string in `YYYY-MM-DD` form when valid,
 *  or returns the raw input untouched when not parseable. */
export function formatDate(raw: string): string {
  const parsed = Date.parse(raw)
  if (Number.isNaN(parsed)) return raw
  const d = new Date(parsed)
  const y = d.getUTCFullYear()
  const m = String(d.getUTCMonth() + 1).padStart(2, '0')
  const day = String(d.getUTCDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

export function formatRelative(ms: number, now: number = Date.now()): string {
  const diffSec = Math.floor((now - ms) / 1000)
  if (diffSec < 0) return '0s ago'
  return formatElapsedShort(diffSec) ?? `${Math.floor(diffSec / 86400)}d ago`
}

/** Renders a frontmatter `date:` value as short-form relative time
 *  (`3h ago` / `2d ago`), flipping to a locale date past 30 days via
 *  `formatMtime`. Accepts the string or `Date` shapes a YAML parser may
 *  emit; returns the raw text untouched when the value is not a parseable
 *  date so a malformed `date:` still surfaces rather than vanishing. */
export function formatChipDate(value: unknown, now: number = Date.now()): string {
  const ms = value instanceof Date
    ? value.getTime()
    : typeof value === 'string'
      ? Date.parse(value)
      : NaN
  if (Number.isNaN(ms)) return typeof value === 'string' ? value : String(value)
  return formatMtime(ms, now)
}
