import { fileSlugFromRelPath } from "./path-utils";
import type { IndexEntry } from "./types";

export interface WikiLinkIndex {
  adrById: Map<number, IndexEntry>;
  workItemById: Map<string, IndexEntry>;
}

export interface ResolvedWikiLink {
  href: string;
  title: string;
}

type WikiLinkKind = "ADR" | "WORK-ITEM";

/** Build the wiki-link match pattern: `[[ADR-NNNN]]`, and a work item
 *  through the server's `idToken`, which matches every ID the configured
 *  pattern writes, such as `PP_0042`, `v001` or a tracker key, or a bare
 *  number before one is served. Read a match with `wikiLinkOf`. */
export function buildWikiLinkPattern(idToken?: string | null): RegExp {
  return new RegExp(`\\[\\[${linkGrammar(idToken)}\\]\\]`, "g");
}

function linkGrammar(idToken?: string | null): string {
  return `(?:(ADR)-(\\d+)|(WORK-ITEM)-(${idToken ?? "\\d+"}))`;
}

export interface WikiLinkMatch {
  prefix: WikiLinkKind;
  id: string;
}

/** The prefix and ID of a match of `buildWikiLinkPattern` or
 *  `buildBareIdPattern`. */
export function wikiLinkOf(match: RegExpMatchArray): WikiLinkMatch {
  return match[1] === undefined
    ? { prefix: "WORK-ITEM", id: match[4] }
    : { prefix: "ADR", id: match[2] };
}

export type BareIdSegment =
  | { kind: "text"; text: string }
  | {
      kind: "match";
      text: string;
      prefix: "ADR" | "WORK-ITEM";
      id: string;
    };

/** Build a bare-token regex that matches the wiki-link grammar without
 *  the surrounding `[[ … ]]`. Used by surfaces that linkify scalar
 *  values (e.g. frontmatter cells) rather than markdown text.
 *
 *  Word-boundary anchored, so embedded matches inside hyphen-joined or
 *  path-shaped strings (`MY-ADR-0017`, `notes/WORK-ITEM-0042.md`) do
 *  match — the surrounding `-`, `/`, `.`, and whitespace are non-word.
 *  Always returned with the `g` flag set so callers can iterate
 *  matches; `splitByBareIds` clones to avoid `lastIndex` bleed across
 *  calls. */
export function buildBareIdPattern(idToken?: string | null): RegExp {
  return new RegExp(`\\b${linkGrammar(idToken)}\\b`, "g");
}

/** Split a string into ordered segments alternating between plain text
 *  and bare-ID matches. Empty leading/trailing text segments around
 *  matches are elided. Returns a single empty-text segment for an
 *  empty input.
 *
 *  Forces the `g` flag on the cloned regex so a caller that hands in
 *  a non-global pattern does not hang the loop. */
export function splitByBareIds(text: string, pattern: RegExp): BareIdSegment[] {
  const flags = pattern.flags.includes("g")
    ? pattern.flags
    : `${pattern.flags}g`;
  const re = new RegExp(pattern.source, flags);
  const segments: BareIdSegment[] = [];
  let lastIndex = 0;
  let m = re.exec(text);
  while (m !== null) {
    if (m.index > lastIndex) {
      segments.push({ kind: "text", text: text.slice(lastIndex, m.index) });
    }
    segments.push({ kind: "match", text: m[0], ...wikiLinkOf(m) });
    lastIndex = m.index + m[0].length;
    m = re.exec(text);
  }
  if (lastIndex < text.length) {
    segments.push({ kind: "text", text: text.slice(lastIndex) });
  }
  if (segments.length === 0) {
    segments.push({ kind: "text", text });
  }
  return segments;
}

/** Parse an ADR id that may live on `frontmatter.adr_id` (e.g. `"ADR-0017"`)
 *  or in the filename prefix (`ADR-0017-foo.md`). Frontmatter wins over
 *  filename — mirrors the server's `parse_adr_id` precedence. */
function adrIdOf(entry: IndexEntry): number | null {
  const fmId = entry.frontmatter.adr_id;
  if (typeof fmId === "string") {
    const rest = fmId.startsWith("ADR-") ? fmId.slice(4) : fmId;
    const n = parsePositiveInt(rest);
    if (n !== null) return n;
  }
  const filename = entry.relPath.split("/").at(-1) ?? "";
  if (!filename.startsWith("ADR-")) return null;
  const rest = filename.slice(4);
  const dash = rest.indexOf("-");
  if (dash < 0) return null;
  return parsePositiveInt(rest.slice(0, dash));
}

/** Single helper for all numeric extraction in this module. Explicit
 *  radix 10 (no octal/hex inference); rejects leading-`+`, leading-`-`,
 *  empty, and trailing-non-digits. */
function parsePositiveInt(s: string): number | null {
  if (!/^\d+$/.test(s)) return null;
  const n = parseInt(s, 10);
  return Number.isFinite(n) && n >= 0 ? n : null;
}

/** Build the resolver maps from the docs caches. ADRs are keyed by
 *  `frontmatter.adr_id` *or* the filename prefix. Work items are keyed by
 *  `entry.workItemId` (filename-derived, supplied by the server). Defensively
 *  filters by `entry.type` so a misuse — accidentally passing plans as work
 *  items — cannot route `[[WORK-ITEM-N]]` to a non-work-item. On duplicate
 *  keys the entry with the lexically-earliest `relPath` wins (deterministic
 *  across reloads). */
export function buildWikiLinkIndex(
  adrEntries: IndexEntry[],
  workItemEntries: IndexEntry[],
): WikiLinkIndex {
  const adrById = new Map<number, IndexEntry>();
  for (const entry of adrEntries) {
    if (entry.type !== "decisions") continue;
    const id = adrIdOf(entry);
    if (id === null) continue;
    insertWithEarliestRelPath(adrById, id, entry);
  }
  const workItemById = new Map<string, IndexEntry>();
  for (const entry of workItemEntries) {
    if (entry.type !== "work-items") continue;
    const id = entry.workItemId;
    if (id === null) continue;
    insertWithEarliestRelPath(workItemById, id, entry);
  }
  return { adrById, workItemById };
}

function insertWithEarliestRelPath<K>(
  map: Map<K, IndexEntry>,
  key: K,
  candidate: IndexEntry,
): void {
  const existing = map.get(key);
  if (!existing || candidate.relPath < existing.relPath) {
    map.set(key, candidate);
  }
}

/** Resolve one wiki-link target. Returns `{ href, title }` on hit,
 *  `null` on miss. The href follows the existing
 *  `/library/:type/:fileSlug` shape; `fileSlug` is derived from the
 *  entry's `relPath` so the filename's date or numeric prefix is
 *  preserved exactly.
 *
 *  `id` is the raw string captured from the wiki-link pattern (group 2):
 *  e.g. `"0017"` for `[[ADR-0017]]`, `"PROJ-0042"` for
 *  `[[WORK-ITEM-PROJ-0042]]`, `"0042"` for `[[WORK-ITEM-0042]]`. */
export function resolveWikiLink(
  prefix: WikiLinkKind,
  id: string,
  idx: WikiLinkIndex,
): ResolvedWikiLink | null {
  let entry: IndexEntry | undefined;
  if (prefix === "ADR") {
    const n = parsePositiveInt(id);
    if (n === null) return null;
    entry = idx.adrById.get(n);
  } else {
    entry = idx.workItemById.get(id);
  }
  if (!entry) return null;
  const fileSlug = fileSlugFromRelPath(entry.relPath);
  return {
    href: `/library/${entry.type}/${fileSlug}`,
    title: entry.title,
  };
}
