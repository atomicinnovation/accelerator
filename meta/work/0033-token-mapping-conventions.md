# 0033 Token Mapping Conventions

Locked-in during Wave 4a from observation of three worst-offender modules
(`LifecycleClusterView`, `LifecycleIndex`, `KanbanBoard`); applied unchanged
to Wave 4b.

## Colour

- `#111827` → `var(--ac-fg-strong)`
- `#374151` → `var(--ac-fg)`
- `#4b5563` → `var(--ac-fg-muted)`
- `#6b7280` → `var(--ac-fg-muted)`
- `#9ca3af` → `var(--ac-fg-faint)`
- `#d1d5db` → `var(--ac-stroke)`
- `#e5e7eb` → `var(--ac-stroke-soft)`
- `#f3f4f6` → `var(--ac-bg-sunken)`
- `#ffffff` → `var(--ac-bg-card)`
- `#1d4ed8`, `#2563eb` → `var(--ac-accent)` (indigo collapse; AC6 drift
  documented in PR — two slightly different blues unified to one token)
- `#dbeafe` → `var(--ac-accent-tint)`
- `#991b1b` → `var(--ac-err)`
- `#fef2f2` → `color-mix(in srgb, var(--ac-err) 8%, var(--ac-bg))` (bg tint)
- `#fecaca` → `color-mix(in srgb, var(--ac-err) 30%, var(--ac-bg))` (border tint)
- `#fee2e2` → `color-mix(in srgb, var(--ac-err) 18%, var(--ac-bg))` (hover state)

color-mix() convention: always `in srgb`, surface always `var(--ac-bg)`,
locked percentages: `8%` = bg tint, `18%` = hover state, `30%` = border tint.

## Spacing

- `0.25rem` (4px) → `var(--sp-1)`
- `0.5rem` (8px) → `var(--sp-2)`
- `0.6rem`, `0.55rem` (≈8px, ≤2px drift) → `var(--sp-2)`
- `0.75rem` (12px) → `var(--sp-3)`
- `0.7rem`, `0.8rem` (≈12px, ≤2px drift) → `var(--sp-3)`
- `1rem` (16px) → `var(--sp-4)`
- `1.5rem` (24px) → `var(--sp-5)`
- `2rem` (32px) → `var(--sp-6)`

Off-scale (→ EXCEPTIONS as irreducible):
- `0.4rem` (6.4px) — between sp-1 and sp-2; kept as-is
- `0.05rem` — sub-pixel badge padding; kept as-is
- `1.25rem` (20px) — between sp-4 and sp-5; 4px off nearest token; kept as-is
- `1.75rem` (28px) — between sp-5 and sp-6; kept as-is

## Radius

- `4px`, `0.25rem` → `var(--radius-sm)`
- `8px` → `var(--radius-md)`
- `9999px` → `var(--radius-pill)`
- `6px` — no token equivalent; kept as EXCEPTIONS irreducible

## Typography (font-size)

- `0.75rem` (12px) → `var(--size-xxs)`
- `0.85rem` (14px) → `var(--size-xs)` (±0.6px drift accepted)
- `0.875rem` (14px) → `var(--size-xs)` (exact match)
- `1rem` (16px) → `var(--size-sm)` (body-copy font-size)
- `1.25rem` (20px) → `var(--size-body)` (only when used as body font-size)
- `1.4rem`, `1.5rem` (≈22px) → `var(--size-lg)` (display; ≤2px drift accepted)

## Typography (font-family)

- `monospace` → `var(--ac-font-mono)`
- Display headings → `var(--ac-font-display)` (Sora)

## Letter-spacing

- `0.06em`, `0.08em` — off-scale; kept as EXCEPTIONS irreducible
- Standard caps-tracking → `var(--tracking-caps)` (0.12em) only if exact match

## Box-shadow

- `rgba(29, 78, 216, 0.12)` accent-tinted elevation → `var(--ac-shadow-soft)`
  (loses accent tint; documented as AC6 drift)
- Coloured rings (`0 0 0 1.5px <colour>`) — replace colour with token, keep
  `1.5px` width as EXCEPTIONS irreducible

## Irreducible by category

These literals always land in EXCEPTIONS as `irreducible`:

- `1px`, `2px` — border/outline widths below `--sp-1` floor
- `1.5px` — coloured ring width
- `6px`, `7px` — layout pixel values with no token equivalent
- `0.4rem`, `0.05rem` — off-scale sub-token spacings
- `0.06em`, `0.08em` — off-scale letter-spacing
- `1.4em` — `calc(line-height × 3)` derived value in text-clamp
- Layout max-widths (`800px`, `900px`, `1100px`) — no token equivalent
- Component min-widths (`220px`, `320px`) — grid/flex layout, no token
