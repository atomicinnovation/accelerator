export function normaliseValue(value: unknown): string {
  if (typeof value !== 'string') return ''
  return value.trim().toLowerCase().replace(/[\s_\-/]+/g, '')
}
