// Resolved-hex semantic palette. Values are stored as resolved hex (or
// rgba) even where the corresponding global.css declaration uses
// var(--atomic-X) brand-layer indirection — see ADR-0035 §3. The
// CSS↔TS parity comparator resolves var() refs via BRAND_COLOR_TOKENS,
// preserving "TS knows the resolved hex of every semantic token" as a
// load-bearing invariant.
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
  // Per-doc-type glyph foreground (light theme). Light values were eyedroppered
  // from library-view-updated-light.png; values that failed WCAG 1.4.11 against
  // --ac-bg (#fbfcfe) were darkened within the same hue family so all 12 keys
  // clear 3:1 contrast (see global.test.ts contrast block).
  'ac-doc-decisions':           '#ad3437',
  'ac-doc-work-items':          '#af4b2f',
  'ac-doc-plans':               '#3256b6',
  'ac-doc-research':            '#b26f35',
  'ac-doc-plan-reviews':        '#5127b5',
  'ac-doc-pr-reviews':          '#7f2cb6',
  'ac-doc-work-item-reviews':   '#ad3458',
  'ac-doc-validations':         '#2e8b57',
  'ac-doc-notes':               '#8e7b22',
  'ac-doc-pr-descriptions':                 '#4588b8',
  'ac-doc-design-gaps':         '#5c9132',
  'ac-doc-design-inventories':  '#2e7e8a',
  // Per-doc-type glyph BACKGROUND tints (light theme). Each value is a
  // very-light, low-saturation hue matching the corresponding foreground
  // token, used as the framed-glyph background in eyebrows and hub cards.
  'ac-doc-bg-decisions':           '#fbe5e6',
  'ac-doc-bg-work-items':          '#fbe9e2',
  'ac-doc-bg-plans':               '#e3ecf6',
  'ac-doc-bg-research':            '#f7ece0',
  'ac-doc-bg-plan-reviews':        '#ebe3f5',
  'ac-doc-bg-pr-reviews':          '#efe2f6',
  'ac-doc-bg-work-item-reviews':   '#f9e3ec',
  'ac-doc-bg-validations':         '#def0e7',
  'ac-doc-bg-notes':               '#f5f0d6',
  'ac-doc-bg-pr-descriptions':     '#e2eff7',
  'ac-doc-bg-design-gaps':         '#e7f1d8',
  'ac-doc-bg-design-inventories':  '#dceaec',
} as const

// Dark-theme overrides — same resolved-hex invariant as LIGHT_COLOR_TOKENS
// above; the CSS-side var(--atomic-X) indirection is invisible to TS.
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
  'ac-ok':             '#79d9a6',
  'ac-warn':           '#e4b76e',
  'ac-err':            '#e86a6b',
  // Per-doc-type glyph foreground (dark theme). Design intent: monochrome
  // glyphs in dark mode regardless of doc type — all twelve tokens resolve
  // to --ac-fg-strong (#ffffff). Keeping the per-key tokens preserves the
  // light/dark parity contract; the values just collapse to a single hue
  // in dark.
  'ac-doc-decisions':           '#ffffff',
  'ac-doc-work-items':          '#ffffff',
  'ac-doc-plans':               '#ffffff',
  'ac-doc-research':            '#ffffff',
  'ac-doc-plan-reviews':        '#ffffff',
  'ac-doc-pr-reviews':          '#ffffff',
  'ac-doc-work-item-reviews':   '#ffffff',
  'ac-doc-validations':         '#ffffff',
  'ac-doc-notes':               '#ffffff',
  'ac-doc-pr-descriptions':     '#ffffff',
  'ac-doc-design-gaps':         '#ffffff',
  'ac-doc-design-inventories':  '#ffffff',
  // Glyph BACKGROUND tints (dark theme). Design intent: uniform monochrome
  // background regardless of doc type — all twelve collapse to a single
  // lighter-than-bg-sunken neutral grey.
  'ac-doc-bg-decisions':           '#1d2030',
  'ac-doc-bg-work-items':          '#1d2030',
  'ac-doc-bg-plans':               '#1d2030',
  'ac-doc-bg-research':            '#1d2030',
  'ac-doc-bg-plan-reviews':        '#1d2030',
  'ac-doc-bg-pr-reviews':          '#1d2030',
  'ac-doc-bg-work-item-reviews':   '#1d2030',
  'ac-doc-bg-validations':         '#1d2030',
  'ac-doc-bg-notes':               '#1d2030',
  'ac-doc-bg-pr-descriptions':     '#1d2030',
  'ac-doc-bg-design-gaps':         '#1d2030',
  'ac-doc-bg-design-inventories':  '#1d2030',
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
  'size-subtitle':    '13px',
  'size-row':         '12.5px',
  'size-xxs':         '12px',
  'size-xxs-sm':      '11.5px',
  'size-eyebrow':     '11px',
  'size-3xs-lg':      '10.5px',
  'size-3xs':         '10px',
  'size-4xs':         '9.5px',
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
  'ac-content-max-width': '1200px',
  'ac-content-max-width-narrow': '600px',
} as const

// Atomic brand palette — see global.css :root header for the canonical
// rationale. Theme-invariant (ADR-0026 §5). Aliases store their RESOLVED
// hex (not a 'var(...)' string) so this map is a pure name→value lookup
// for the parity comparator; the alias-target equality is asserted in
// global.test.ts. Named by what it contains (brand colours), not by the
// CSS-side `--atomic-` prefix.
export const BRAND_COLOR_TOKENS = {
  'atomic-night':         '#0e0f19',
  'atomic-night-2':       '#0a111b',
  'atomic-night-3':       '#171925',
  'atomic-night-4':       '#1d2131',
  'atomic-ink':           '#202231',
  'atomic-ink-2':         '#2c2e41',
  'atomic-red':           '#cb4647',
  'atomic-red-2':         '#df5758',
  'atomic-red-3':         '#e24e53',
  'atomic-indigo':        '#595fc8',
  'atomic-indigo-2':      '#323062',
  'atomic-indigo-tint':   '#c1c5ff',
  'atomic-medium-purple': '#965dd9',
  'atomic-cream-can':     '#f5c25f',
  'atomic-steel-blue':    '#4295a5',
  'atomic-pastel-green':  '#6be58b',
  'atomic-river-bed':     '#4a545f',
  'atomic-aquamarine':    '#73e4e2',
  'atomic-tradewind':     '#52b0aa',
  'atomic-geyser':        '#d3dbe0',
  'atomic-malibu':        '#72cbf5',
  'atomic-link-water':    '#ddecf4',
  'atomic-marigold':      '#f9de6f',
  'atomic-violet':        '#965dd9', // resolved alias of atomic-medium-purple
  'atomic-teal':          '#52b0aa', // resolved alias of atomic-tradewind
  'atomic-sky':           '#72cbf5', // resolved alias of atomic-malibu
  'atomic-sky-2':         '#72cbf5', // resolved alias of atomic-malibu
  'atomic-white':         '#ffffff',
  'atomic-bone':          '#fbfcfe',
  'atomic-mist':          '#d9d9d9',
  'atomic-ash':           '#d3dbe0',
  'atomic-smoke':         '#c7c9d8',
  'atomic-slate':         '#5f6378',
  'atomic-slate-2':       '#4a545f',
  'atomic-overlay-ink':   'rgba(23, 25, 37, 0.56)',
  'atomic-stroke-light':  'rgba(255, 255, 255, 0.35)',
  'atomic-shadow-soft':   'rgba(0, 0, 0, 0.08)',
} as const

export type BrandColorToken = keyof typeof BRAND_COLOR_TOKENS

// Documented alias pairs — keep in sync with the comments above. The
// alias-target equality test in global.test.ts iterates this list.
export const BRAND_ALIAS_PAIRS: ReadonlyArray<readonly [BrandColorToken, BrandColorToken]> = [
  ['atomic-violet', 'atomic-medium-purple'],
  ['atomic-teal',   'atomic-tradewind'],
  ['atomic-sky',    'atomic-malibu'],
  ['atomic-sky-2',  'atomic-malibu'],
] as const

// Code-block surface tokens — theme-independent palette adopted from
// the design prototype's .ac-codeblock block (meta/research/
// design-inventories/2026-05-21-015231-claude-design-prototype/
// prototype-standalone.html). Same values resolve in both light and
// dark themes (declared only in `:root`). See ADR-0026 §5 and story
// meta/work/0076-code-block-syntax-highlight-palette.md.
export const CODE_SURFACE_TOKENS = {
  'code-bg':        '#0e1320',
  'code-bg-head':   '#161b2c',
  'code-stroke':    'rgba(255, 255, 255, 0.07)',
  'code-fg':        '#d7dcec',
  'code-fg-faint':  '#6f7796',
} as const

// Syntax-highlight tokens. The `tk-` prefix means "syntax token" and
// is preserved from the prototype source so the drift fixture matches
// byte-for-byte. Theme-invariant. Six tokens marked "reserved" below
// ship with the prototype palette but have no current hljs selector
// in code-syntax.global.css — they are kept to preserve the
// prototype-parity contract; do not delete without coordinating with
// prototype-tokens.fixture.test.ts.
export const CODE_SYNTAX_TOKENS = {
  'tk-com':      '#6f7796', // comment
  'tk-str':      '#6be58b', // string literal
  'tk-num':      '#f9de6f', // number literal
  'tk-kw':       '#c1c5ff', // keyword
  'tk-lit':      '#f9a66b', // boolean/null literal
  'tk-typ':      '#73e4e2', // type / class name
  'tk-fn':       '#ffc1a8', // function name
  'tk-attr':     '#c18cf0', // attribute / yaml key
  'tk-deco':     '#c18cf0', // @decorator / hljs-meta
  'tk-macro':    '#df9ce6', // reserved — C/Rust macros
  'tk-var':      '#72cbf5', // variable / template-variable
  'tk-key':      '#c1c5ff', // reserved — key-value key
  'tk-flag':     '#f9de6f', // reserved — CLI/option flags
  'tk-heredoc':  '#c18cf0', // reserved — heredoc bodies
  'tk-pun':      '#8990b0', // punctuation
  'tk-lifet':    '#f9a66b', // reserved — Rust lifetimes
  'tk-header':   '#c18cf0', // markdown section heading
  'tk-anchor':   '#df9ce6', // link / symbol
  'tk-tag':      '#df5758', // HTML/XML tag
  'tk-doctype':  '#c18cf0', // HTML doctype
  'tk-bn':       '#72cbf5', // built-in
  'tk-prop':     '#72cbf5', // object property
  'tk-sel':      '#ffc1a8', // CSS selector
  'tk-atrule':   '#c18cf0', // reserved — CSS @-rule
  'tk-dhdr':     '#c18cf0', // diff file header
  'tk-dhunk':    '#72cbf5', // diff hunk header
  'tk-dadd':     '#6be58b', // diff addition
  'tk-ddel':     '#e56b7e', // diff deletion
} as const

export type CodeSurfaceToken = keyof typeof CODE_SURFACE_TOKENS
export type CodeSyntaxToken = keyof typeof CODE_SYNTAX_TOKENS

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
