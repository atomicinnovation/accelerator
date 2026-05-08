export const LIGHT_COLOR_TOKENS = {
  'ac-bg':             '#fbfcfe',
  'ac-bg-raised':      '#ffffff',
  'ac-bg-sunken':      '#f4f6fa',
  'ac-bg-chrome':      '#ffffff',
  'ac-bg-sidebar':     '#f7f8fb',
  'ac-bg-card':        '#ffffff',
  'ac-bg-hover':       'rgba(32, 34, 49, 0.04)',
  'ac-bg-active':      'rgba(89, 95, 200, 0.09)',
  'ac-fg':             '#14161f',
  'ac-fg-strong':      '#0a111b',
  'ac-fg-muted':       '#5f6378',
  'ac-fg-faint':       '#8b90a3',
  'ac-stroke':         'rgba(32, 34, 49, 0.10)',
  'ac-stroke-soft':    'rgba(32, 34, 49, 0.06)',
  'ac-stroke-strong':  'rgba(32, 34, 49, 0.18)',
  'ac-accent':         '#595fc8',
  'ac-accent-2':       '#cb4647',
  'ac-accent-tint':    'rgba(89, 95, 200, 0.12)',
  'ac-accent-faint':   'rgba(89, 95, 200, 0.06)',
  'ac-ok':             '#2e8b57',
  'ac-warn':           '#d98f2e',
  'ac-err':            '#cb4647',
  'ac-violet':         '#7b5cd9',
} as const

export const DARK_COLOR_TOKENS = {
  'ac-bg':             '#0a111b',
  'ac-bg-raised':      '#0e0f19',
  'ac-bg-sunken':      '#070b12',
  'ac-bg-chrome':      '#0e0f19',
  'ac-bg-sidebar':     '#0b121c',
  'ac-bg-card':        '#131524',
  'ac-bg-hover':       'rgba(255, 255, 255, 0.04)',
  'ac-bg-active':      'rgba(89, 95, 200, 0.22)',
  'ac-fg':             '#e7e9f2',
  'ac-fg-strong':      '#ffffff',
  'ac-fg-muted':       '#a0a5b8',
  'ac-fg-faint':       '#6c7088',
  'ac-stroke':         'rgba(255, 255, 255, 0.08)',
  'ac-stroke-soft':    'rgba(255, 255, 255, 0.04)',
  'ac-stroke-strong':  'rgba(255, 255, 255, 0.16)',
  'ac-accent':         '#8a90e8',
  'ac-accent-2':       '#e86a6b',
  'ac-accent-tint':    'rgba(138, 144, 232, 0.18)',
  'ac-accent-faint':   'rgba(138, 144, 232, 0.08)',
} as const

export const TYPOGRAPHY_TOKENS = {
  'ac-font-display':  '"Sora", system-ui, sans-serif',
  'ac-font-body':     '"Inter", system-ui, sans-serif',
  'ac-font-mono':     '"Fira Code", ui-monospace, monospace',
  'size-hero':        '68px',
  'size-h1':          '48px',
  'size-h2':          '36px',
  'size-h3':          '28px',
  'size-h4':          '26px',
  'size-lg':          '22px',
  'size-body':        '20px',
  'size-md':          '18px',
  'size-sm':          '16px',
  'size-xs':          '14px',
  'size-xxs':         '12px',
  'lh-tight':         '1.05',
  'lh-snug':          '1.2',
  'lh-normal':        '1.5',
  'lh-loose':         '1.6',
  'tracking-caps':    '0.12em',
} as const

export const SPACING_TOKENS = {
  'sp-1':   '4px',
  'sp-2':   '8px',
  'sp-3':   '12px',
  'sp-4':   '16px',
  'sp-5':   '24px',
  'sp-6':   '32px',
  'sp-7':   '40px',
  'sp-8':   '48px',
  'sp-9':   '64px',
  'sp-10':  '80px',
  'sp-11':  '124px',
} as const

export const RADIUS_TOKENS = {
  'radius-sm':   '4px',
  'radius-md':   '8px',
  'radius-lg':   '12px',
  'radius-pill': '999px',
} as const

// Theme-invariant shadows (--shadow-card / --shadow-card-lg / --shadow-crisp)
// plus light-theme values for theme-variant shadows (--ac-shadow-soft / --ac-shadow-lift).
export const LIGHT_SHADOW_TOKENS = {
  'shadow-card':    '6px 12px 85px 0px rgba(0, 0, 0, 0.08)',
  'shadow-card-lg': '12px 24px 120px 0px rgba(0, 0, 0, 0.12)',
  'shadow-crisp':   '0 1px 2px rgba(10, 17, 27, 0.06), 0 4px 12px rgba(10, 17, 27, 0.04)',
  'ac-shadow-soft': '0 1px 2px rgba(10,17,27,0.04), 0 8px 28px rgba(10,17,27,0.06)',
  'ac-shadow-lift': '0 2px 4px rgba(10,17,27,0.06), 0 20px 60px rgba(10,17,27,0.10)',
} as const

// Dark-theme overrides — only the theme-variant shadows redefine.
export const DARK_SHADOW_TOKENS = {
  'ac-shadow-soft': '0 1px 2px rgba(0,0,0,0.3), 0 8px 28px rgba(0,0,0,0.4)',
  'ac-shadow-lift': '0 2px 4px rgba(0,0,0,0.4), 0 20px 60px rgba(0,0,0,0.55)',
} as const

export const LAYOUT_TOKENS = {
  'ac-topbar-h': '48px',
} as const

export const MONO_FONT_TOKENS = {
  'ac-font-display': 'var(--ac-font-mono)',
  'ac-font-body':    'var(--ac-font-mono)',
} as const

export type MonoFontToken = keyof typeof MONO_FONT_TOKENS

export type ColorTokenLight = keyof typeof LIGHT_COLOR_TOKENS
export type ColorTokenDark = keyof typeof DARK_COLOR_TOKENS
export type TypographyToken = keyof typeof TYPOGRAPHY_TOKENS
export type SpacingToken = keyof typeof SPACING_TOKENS
export type RadiusToken = keyof typeof RADIUS_TOKENS
export type LightShadowToken = keyof typeof LIGHT_SHADOW_TOKENS
export type DarkShadowToken = keyof typeof DARK_SHADOW_TOKENS
export type LayoutToken = keyof typeof LAYOUT_TOKENS
