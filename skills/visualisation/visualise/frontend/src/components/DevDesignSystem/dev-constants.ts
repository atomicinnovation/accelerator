// Cross-cutting constants for the DevDesignSystem (`/dev`) reference page and
// its activation bridge. Kept CSS-free so the bridge hook, the keychord handler,
// the README-mirrored hint, and the section oracles can all import one source.

/**
 * Matches ONLY the activation-alias hash forms (`#dev`, `#dev/<section>`),
 * never the canonical bare `#<section>` hash — so the bridge ignores the
 * scroll-spy's own hash writes.
 *
 * INVARIANT: no `DEV_SECTIONS` id may begin with "dev" at a word boundary, or
 * its canonical `#dev…` hash would be mis-classified as an alias (asserted by a
 * unit test — see Phase 4 success criteria).
 */
export const DEV_ALIAS_RE = /^#\/?dev(\b|\/|$)/;

/** Section slug carried by an alias hash: "" for `#dev`, "colors" for `#dev/colors`. */
export const aliasSection = (hash: string): string =>
  hash.replace(/^#\/?dev\/?/, "");

/**
 * Classify a `hashchange` for the activation bridge. Pure + total so it can be
 * unit-tested in isolation. Returns whether to activate `/dev` and, if so, the
 * target section (undefined → overview). A hash the bridge itself just wrote
 * (`lastProgrammaticHash`) is ignored so a future switch to `location.hash = …`
 * cannot reintroduce the activation loop; a bare `#<section>` hash (the
 * scroll-spy's canonical write) is not an alias and never activates.
 */
export function devAliasTarget(
  hash: string,
  lastProgrammaticHash: string | null,
): { activate: boolean; section?: string } {
  if (hash !== "" && hash === lastProgrammaticHash) return { activate: false };
  if (!DEV_ALIAS_RE.test(hash)) return { activate: false };
  const section = aliasSection(hash);
  return { activate: true, section: section || undefined };
}

function isMac(): boolean {
  if (typeof navigator === "undefined") return false;
  return /Mac|iPhone|iPad|iPod/i.test(navigator.userAgent ?? "");
}

// The activation chord. Matches on `event.code` (physical key position,
// layout-independent) rather than `event.key` (the layout/Shift-sensitive
// "l"/"L"), so it is reachable on non-US keyboard layouts. The handler binds
// `meta || ctrl` (mac ⌘ or Win/Linux Ctrl), so the hint follows suit.
//
// NB: if the cross-browser matrix (Manual Testing step 5) forces the fallback,
// the ONLY change is DEV_CHORD + DEV_CHORD_HINT here — re-run the matrix on any
// edit, and re-verify the README chord wording against the final hint.
export const DEV_CHORD = {
  code: "KeyL",
  shift: true,
  meta: true,
  ctrl: true,
} as const;

/** Platform-resolved chord hint — the marquee, footer, the DEV console hint,
 *  and the README all bind to this one value (never a hardcoded `⌘⇧D`). */
export const DEV_CHORD_HINT = isMac() ? "⌘⇧L" : "Ctrl+⇧+L";

export interface DevSection {
  id: string;
  label: string;
}

// ids = prototype `DEV_SECTIONS` slugs (view-dev.jsx:15-39); they differ from
// the display labels in many cases (slug `colors` vs label "Colours").
export const DEV_SECTIONS = [
  { id: "overview", label: "Overview" },
  { id: "colors", label: "Colours" },
  { id: "type", label: "Type" },
  { id: "spacing", label: "Spacing" },
  { id: "radii", label: "Radii & shadows" },
  { id: "icons", label: "Icons" },
  { id: "glyphs", label: "Doc-type glyphs" },
  { id: "bigglyphs", label: "Empty-state glyphs" },
  { id: "mark", label: "Atomic mark" },
  { id: "chips", label: "Chips" },
  { id: "badges", label: "Status badges" },
  { id: "stagedots", label: "Stage dots" },
  { id: "tierpills", label: "Tier pills" },
  { id: "buttons", label: "Buttons" },
  { id: "form", label: "Inputs & form" },
  { id: "nav", label: "Sidebar nav" },
  { id: "cards", label: "Cards" },
  { id: "table", label: "Tables" },
  { id: "markdown", label: "Markdown" },
  { id: "code", label: "Code blocks" },
  { id: "frontmatter", label: "Frontmatter" },
  { id: "empty", label: "Empty & banners" },
  { id: "toast", label: "Toasts" },
  { id: "topbar", label: "Topbar" },
] as const satisfies readonly DevSection[];
