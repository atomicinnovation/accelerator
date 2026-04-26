export function formatMtime(ms: number, now: number = Date.now()): string {
  if (ms <= 0) return '—'
  const diffSec = Math.floor((now - ms) / 1000)
  if (diffSec < 0)          return 'just now'
  if (diffSec < 60)         return `${diffSec}s ago`
  if (diffSec < 3600)       return `${Math.floor(diffSec / 60)}m ago`
  if (diffSec < 86400)      return `${Math.floor(diffSec / 3600)}h ago`
  if (diffSec < 7 * 86400)  return `${Math.floor(diffSec / 86400)}d ago`
  if (diffSec < 30 * 86400) return `${Math.floor(diffSec / (7 * 86400))}w ago`
  return new Date(ms).toLocaleDateString()
}
