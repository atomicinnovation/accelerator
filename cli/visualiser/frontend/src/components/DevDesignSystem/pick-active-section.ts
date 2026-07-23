/** A section's top edge, in scroll-root-relative pixels (rect.top − rootTop). */
export interface SectionTop {
  id: string;
  top: number;
}

/**
 * Pure, total-order active-section picker for the scroll-spy. Recomputed from
 * ALL sections' current tops on every IntersectionObserver dispatch (never the
 * prototype's single-dispatch highest-ratio, which pinned to Colours).
 *
 * Rule: **the last section whose top edge is at or above the active-band top.**
 * This is well-defined for every scroll position — a gap between two short
 * sections keeps the upper one active (no clearing), a tall section spanning the
 * whole band stays active, scrolled-above-all defaults to the first section, and
 * scrolled-below-all resolves to the last — so the highlight never clears,
 * flickers, or pins.
 *
 * @param sections section tops in document order
 * @param bandTop  the active-band top offset (matches the observer's rootMargin
 *                 top, e.g. 80 for `-80px 0px -55% 0px`)
 */
export function pickActiveSection(
  sections: ReadonlyArray<SectionTop>,
  bandTop: number,
): string | null {
  if (sections.length === 0) return null;
  let active = sections[0].id; // default: scrolled above all → first
  for (const s of sections) {
    if (s.top <= bandTop) active = s.id;
  }
  return active;
}
