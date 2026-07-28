// CSS-free constants for `Icon`. Kept in a separate module so consumers that
// only need the name registry / type (notably the Playwright visual-regression
// specs, whose TS transformer can't parse CSS modules, and the dev-page
// section oracles that assert `ICON_NAMES.length`) can import them without
// pulling `Icon.module.css` into the import graph. Mirrors the
// `Glyph.constants.ts` / `BigGlyph.constants.ts` naming convention.

// The 33 stroke-icon names ported from the prototype `ui.jsx` Icon primitive.
// Ordering mirrors the prototype path map. This tuple is the single source of
// truth for the Icons section count (Overview + Icons section assert
// `ICON_NAMES.length`).
export const ICON_NAMES = [
  "search",
  "library",
  "kanban",
  "lifecycle",
  "activity",
  "clock",
  "link",
  "chevron-right",
  "chevron-down",
  "chevron-left",
  "doc",
  "edit",
  "close",
  "check",
  "dot",
  "plus",
  "minus",
  "git-pr",
  "git-branch",
  "filter",
  "sort",
  "sparkle",
  "hex",
  "shield",
  "moon",
  "sun",
  "settings",
  "terminal",
  "arrow-right",
  "flag",
  "folder",
  "layers",
  "alert",
] as const;

export type IconName = (typeof ICON_NAMES)[number];
