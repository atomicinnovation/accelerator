import type { DocTypeKey } from "../../../api/types";

/** Minimum missing-slug length before suggestions are generated. Mirrors the
 *  search hook's gate (`use-search.ts:7-8`) and the server-side intent. */
export const MIN_SUGGESTION_LEN = 2;
export const MAX_SUGGESTIONS = 5;

/** A suggestion candidate sourced from an `IndexEntry`. `slug` is the link-ready
 *  string (`entry.slug ?? fileSlugFromRelPath(entry.relPath)`), never null. */
export interface SlugCandidate {
  type: DocTypeKey;
  slug: string;
  title: string;
  mtimeMs: number;
  relPath: string;
}

/** The single normalisation used everywhere a missing slug is measured/matched:
 *  trim, then lowercase. */
export function normaliseMissingSlug(missingSlug: string): string {
  return missingSlug.trim().toLowerCase();
}

/** The single suggestibility gate. Both the hook's `enabled` flag and
 *  `rankSlugSuggestions` call this so the gate has exactly one definition and
 *  cannot drift (was previously expressed three times with `trim` vs
 *  `trim().toLowerCase()` inconsistency — review finding). */
export function isSuggestible(missingSlug: string): boolean {
  return normaliseMissingSlug(missingSlug).length >= MIN_SUGGESTION_LEN;
}

// 0 = prefix (higher quality), 1 = interior. ExactSlug/Body are intentionally
// absent: an exact slug is a *found* doc, and body matching is out of scope.
const PREFIX = 0;
const INTERIOR = 1;

function bucket(candidateSlug: string, missingLc: string): number | null {
  const s = candidateSlug.toLowerCase();
  if (s === missingLc) return null; // exact ⇒ not a 404 candidate
  if (s.startsWith(missingLc)) return PREFIX;
  if (s.includes(missingLc)) return INTERIOR;
  return null;
}

/** Rank candidates against a missing slug. Returns up to MAX_SUGGESTIONS,
 *  ordered by (bucket asc, mtimeMs desc, relPath asc). Returns [] when the
 *  missing slug is not suggestible (after normalisation). */
export function rankSlugSuggestions(
  missingSlug: string,
  candidates: readonly SlugCandidate[],
): SlugCandidate[] {
  if (!isSuggestible(missingSlug)) return [];
  const missingLc = normaliseMissingSlug(missingSlug);

  const scored: { c: SlugCandidate; b: number }[] = [];
  for (const c of candidates) {
    const b = bucket(c.slug, missingLc);
    if (b !== null) scored.push({ c, b });
  }

  scored.sort(
    (x, y) =>
      x.b - y.b ||
      y.c.mtimeMs - x.c.mtimeMs ||
      x.c.relPath.localeCompare(y.c.relPath),
  );

  return scored.slice(0, MAX_SUGGESTIONS).map((s) => s.c);
}
