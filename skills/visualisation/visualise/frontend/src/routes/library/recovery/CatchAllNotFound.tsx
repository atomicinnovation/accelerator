import { NotFoundSurface } from "./NotFoundSurface";

/** Router-level catch-all: no missingSlug, no knownType ⇒ H1 `Page not found`,
 *  DefaultBigGlyph hero (hue 215), no eyebrow, `Back to library` only, no
 *  suggestions (no slug to match). Lives in a `.tsx` file so `router.ts` (a
 *  JSX-free `.ts` module) can wire it by reference. */
export function CatchAllNotFound() {
  return <NotFoundSurface />;
}
