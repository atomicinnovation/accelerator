import { normaliseSelection } from "./query-keys";
import type {
  ActivityEvent,
  ActivityResponse,
  DocType,
  DocTypeKey,
  IndexEntry,
  LibrarySelection,
  LibraryStructureResponse,
  LifecycleCluster,
  RelatedArtifactsResponse,
  TemplateDetail,
  TemplateSummaryListResponse,
} from "./types";

/** Typed error thrown by fetch helpers on non-2xx responses, so
 *  callers can branch on `err instanceof FetchError && err.status === 404`
 *  rather than substring-matching the message. */
export class FetchError extends Error {
  constructor(
    public readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "FetchError";
  }
}

export class ConflictError extends FetchError {
  constructor(
    status: number,
    message: string,
    public readonly currentEtag: string,
  ) {
    super(status, message);
    this.name = "ConflictError";
  }
}

export interface PatchResult {
  etag: string;
}

export async function patchWorkItemFrontmatter(
  relPath: string,
  patch: { status: string },
  etag: string,
): Promise<PatchResult> {
  const encodedPath = relPath.split("/").map(encodeURIComponent).join("/");
  const r = await fetch(`/api/docs/${encodedPath}/frontmatter`, {
    method: "PATCH",
    headers: {
      "If-Match": `"${etag}"`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ patch }),
  });
  if (r.status === 412) {
    const body = (await r.json()) as { currentEtag: string };
    throw new ConflictError(
      412,
      `PATCH /api/docs/${relPath}/frontmatter: 412`,
      body.currentEtag,
    );
  }
  if (!r.ok) {
    throw new FetchError(
      r.status,
      `PATCH /api/docs/${relPath}/frontmatter: ${r.status}`,
    );
  }
  const rawEtag = r.headers.get("etag") ?? "";
  const parsedEtag =
    rawEtag.startsWith('"') && rawEtag.endsWith('"')
      ? rawEtag.slice(1, -1)
      : rawEtag;
  return { etag: parsedEtag };
}

export async function fetchTypes(): Promise<DocType[]> {
  const r = await fetch("/api/types");
  if (!r.ok) throw new FetchError(r.status, `GET /api/types: ${r.status}`);
  const body: { types: DocType[] } = await r.json();
  return body.types;
}

/** Wire-shape variant of `IndexEntry` — older servers may omit the
 *  `completeness` and `linkedCount` fields. The exported `IndexEntry`
 *  type requires both; we accept the broader shape here and narrow at
 *  the boundary. */
type WireIndexEntry = Omit<IndexEntry, "completeness" | "linkedCount"> & {
  completeness?: IndexEntry["completeness"];
  linkedCount?: number;
};

/** Normalises an `IndexEntry` shape from the wire. Servers older than the
 *  per-entry-completeness deployment omit `completeness` entirely (JSON
 *  `undefined`); newer servers emit `null` for orphans. Collapsing both to
 *  `null` here means consumers see a single shape. */
function normaliseEntry(raw: WireIndexEntry): IndexEntry {
  return {
    ...raw,
    completeness: raw.completeness ?? null,
    linkedCount: raw.linkedCount ?? 0,
  };
}

export async function fetchDocs(type: DocTypeKey): Promise<IndexEntry[]> {
  const r = await fetch(`/api/docs?type=${encodeURIComponent(type)}`);
  if (!r.ok)
    throw new FetchError(r.status, `GET /api/docs?type=${type}: ${r.status}`);
  const body: { docs: WireIndexEntry[] } = await r.json();
  return body.docs.map(normaliseEntry);
}

export async function fetchDocContent(
  relPath: string,
): Promise<{ content: string; etag: string }> {
  // Encode per-segment so filenames containing '#', '?', '%', or
  // non-ASCII are transmitted correctly. '/' separators between segments
  // stay literal since the server accepts them as path structure.
  const encodedPath = relPath.split("/").map(encodeURIComponent).join("/");
  const r = await fetch(`/api/docs/${encodedPath}`);
  if (!r.ok)
    throw new FetchError(r.status, `GET /api/docs/${relPath}: ${r.status}`);
  const content = await r.text();
  const etag = r.headers.get("etag") ?? "";
  return { content, etag };
}

export async function fetchLibraryStructure(
  selection?: LibrarySelection,
): Promise<LibraryStructureResponse> {
  const normalised = normaliseSelection(selection);
  const params = new URLSearchParams();
  for (const [docType, perType] of Object.entries(normalised)) {
    for (const [facetId, options] of Object.entries(perType)) {
      for (const option of options) {
        params.append(`selection[${docType}][${facetId}]`, option);
      }
    }
  }
  const qs = params.toString();
  const url = qs ? `/api/library/structure?${qs}` : "/api/library/structure";
  const r = await fetch(url);
  if (!r.ok) throw new FetchError(r.status, `GET ${url}: ${r.status}`);
  return r.json();
}

export async function fetchTemplates(): Promise<TemplateSummaryListResponse> {
  const r = await fetch("/api/templates");
  if (!r.ok) throw new FetchError(r.status, `GET /api/templates: ${r.status}`);
  return r.json();
}

export async function fetchTemplateDetail(
  name: string,
): Promise<TemplateDetail> {
  const r = await fetch(`/api/templates/${encodeURIComponent(name)}`);
  if (!r.ok)
    throw new FetchError(r.status, `GET /api/templates/${name}: ${r.status}`);
  return r.json();
}

type WireLifecycleCluster = Omit<LifecycleCluster, "entries"> & {
  entries: WireIndexEntry[];
};

function normaliseCluster(raw: WireLifecycleCluster): LifecycleCluster {
  return {
    ...raw,
    entries: raw.entries.map(normaliseEntry),
  };
}

export async function fetchLifecycleClusters(): Promise<LifecycleCluster[]> {
  const r = await fetch("/api/lifecycle");
  if (!r.ok) throw new FetchError(r.status, `GET /api/lifecycle: ${r.status}`);
  const body: { clusters: WireLifecycleCluster[] } = await r.json();
  return body.clusters.map(normaliseCluster);
}

export async function fetchLifecycleCluster(
  slug: string,
): Promise<LifecycleCluster> {
  const r = await fetch(`/api/lifecycle/${encodeURIComponent(slug)}`);
  if (!r.ok)
    throw new FetchError(r.status, `GET /api/lifecycle/${slug}: ${r.status}`);
  const body: WireLifecycleCluster = await r.json();
  return normaliseCluster(body);
}

export async function fetchActivity(limit: number): Promise<ActivityEvent[]> {
  const r = await fetch(
    `/api/activity?limit=${encodeURIComponent(String(limit))}`,
  );
  if (!r.ok)
    throw new FetchError(
      r.status,
      `GET /api/activity?limit=${limit}: ${r.status}`,
    );
  const body: ActivityResponse = await r.json();
  return body.events;
}

export interface SearchResult {
  docType: DocTypeKey;
  title: string;
  slug: string;
  mtimeMs: number;
}

export async function fetchSearch(
  q: string,
  signal?: AbortSignal,
): Promise<SearchResult[]> {
  try {
    const r = await fetch(`/api/search?q=${encodeURIComponent(q)}`, { signal });
    if (!r.ok) throw new FetchError(r.status, `GET /api/search: ${r.status}`);
    const body: { results: SearchResult[] } = await r.json();
    return body.results;
  } catch (err) {
    if (err instanceof DOMException && err.name === "AbortError") throw err;
    console.error(err);
    throw err;
  }
}

export async function fetchRelated(
  relPath: string,
): Promise<RelatedArtifactsResponse> {
  const encodedPath = relPath.split("/").map(encodeURIComponent).join("/");
  const r = await fetch(`/api/related/${encodedPath}`);
  if (!r.ok)
    throw new FetchError(r.status, `GET /api/related/${relPath}: ${r.status}`);
  return r.json();
}
