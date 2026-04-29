export const COLOR_TOKENS = {
  'color-text':              '#0f172a',
  'color-muted-text':        '#4b5563',
  'color-muted-decorative':  '#9ca3af',
  'color-divider':           '#e5e7eb',
  'color-focus-ring':        '#2563eb',
  'color-warning-bg':        '#fff8e6',
  'color-warning-border':    '#d97706',
  'color-warning-text':      '#7c2d12',
} as const
export type ColorToken = keyof typeof COLOR_TOKENS
