---
title: Meta directory visualiser — design
type: spec
status: draft
date: "2026-04-17"
last_updated: "2026-04-18"
author: Toby Clemson
slug: meta-visualisation
---

# Meta directory visualiser — design

## Purpose

Provide a local, browser-based visualiser for the artifacts the accelerator
plugin writes into a project's `meta/` directory. The primary use case is
running the visualiser alongside an active Claude Code session — a companion
window that reads and lightly interacts with the artifacts Claude produces
(research, plans, reviews, validations, decisions, tickets, notes, PRs). The
secondary use case is pre/post-flight triage between sessions: browsing
decisions, following the lifecycle of a piece of work, and moving tickets on a
kanban board.

The visualiser ships as a new skill in the accelerator plugin, complemented by
a standalone CLI for longer, session-independent usage.

## Scope (v1)

Three views on the ten supported document types:

1. **Library** — a reader for every doc type (decisions, tickets, plans,
   research, plan-reviews, pr-reviews, validations, notes, PRs, templates).
   Markdown rendering with frontmatter-aware chrome and cross-reference links.
   Deep-linkable URLs.
2. **Lifecycle** — timeline view of *work units* formed by slug-clustering
   related documents (e.g. a plan, the research behind it, and the review of
   it, all sharing a slug).
3. **Kanban** — a three-column board (`todo` → `in-progress` → `done`) for
   tickets. Drag-and-drop updates the ticket's frontmatter `status:` field in
   place.

The configured `review_plans` and `review_prs` paths are surfaced as two
distinct doc types — `plan-reviews` and `pr-reviews` — each a flat walk of its
own directory. Clustering across them (lifecycle view) is by slug match
regardless of sub-type.

`templates` is a **virtual doc type** backed by the plugin's three-tier
template resolver (`config_resolve_template()` in `config-common.sh`)
rather than a flat directory walk — the library shows every tier
(config override > user override > plugin default) per template, so
users can preview each template regardless of current configuration.
See "Templates" under Data model for the data shape.

Both a slash command (`/accelerator:visualise`) and a CLI
(`accelerator-visualiser`) launch the same server.

## Non-goals (v1)

- Editing of document bodies or non-status frontmatter fields.
- Templates appearing in lifecycle or kanban (they are authoring aids, shown
  only in the library under a de-emphasised "Meta" heading).
- Authentication, multi-user coordination, or a deployed mode.
- Search, activity feed, knowledge-graph, or review dashboard views.
- Writing back inferred cross-references as explicit frontmatter links.
- A GitHub-backed file driver. The file-access layer is designed as an
  interface so such a driver can be added later without touching the rest of
  the system, but no implementation is produced in v1.

## Architecture

```
┌───────────────────────────── User's machine ─────────────────────────────┐
│                                                                          │
│   Claude Code session                                                    │
│        │                                                                 │
│        │  /accelerator:visualise  (slash command)                        │
│        ▼                                                                 │
│   skills/visualisation/visualise/SKILL.md                                │
│        │                                                                 │
│        ▼                                                                 │
│   launch-server.sh  ──► (first run per plugin version)                   │
│        │                     │                                           │
│        │                     ▼                                           │
│        │            GitHub Releases ──► fetch accelerator-visualiser-<os>-<arch>      │
│        │             (curl + SHA-256     → <plugin-root>/.../bin/        │
│        │              verify against     checksums.json gate             │
│        │              committed manifest)                                │
│        ▼                                                                 │
│   Rust server (axum) ──► HTTP /api/* · SSE /api/events · static /*       │
│     │                                                                    │
│     ├─ file_driver (tokio fs + path canonicalize/prefix guard)           │
│     ├─ indexer     (scan · slug derive · SHA-256 ETag cache)             │
│     ├─ watcher     (notify crate · 100ms per-path debounce)              │
│     ├─ sse_hub     (broadcast doc-changed to subscribers)                │
│     └─ patcher     (YAML-aware line patcher · atomic rename)             │
│             │                                                            │
│             └─────► meta/**/*.md                                         │
│                                                                          │
│   accelerator-visualiser CLI  ──► same launch-server.sh                  │
│                                                                          │
│   Browser at http://localhost:<dynamic-port>                             │
│        │                                                                 │
│        ▼                                                                 │
│   React SPA (Vite-built, embedded into the server binary via            │
│   rust-embed at compile time — no committed dist/)                       │
│   ├─ TanStack Router  (URLs as state, deep-linking)                      │
│   ├─ TanStack Query   (doc cache, ETag-aware, SSE invalidation)          │
│   ├─ dnd-kit          (kanban drag-and-drop)                             │
│   └─ Views: Library · Lifecycle · Kanban                                 │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Runtime

- **Server**: Rust (stable toolchain), `axum` for HTTP and SSE, `tokio` async
  runtime, `notify` for file watching, `gray_matter` + `serde_yml` for
  frontmatter, `sha2` for ETag hashing, `rust-embed` for bundling the
  frontend into the binary, and a small YAML-aware line patcher for
  writes. Ships as a single static binary per target arch (the frontend
  bundle is embedded — one artefact, not two). End users never need Rust
  installed — binaries are fetched from GitHub Releases on first use
  (see Distribution).
- **Frontend**: React + TypeScript + Vite. TanStack Router, TanStack Query,
  dnd-kit, CSS Modules. `frontend/dist/` is built by maintainers as part
  of the release pipeline and **embedded into the server binary via
  `rust-embed`** at `cargo build` time; it is **not** committed to the
  repo. End users never run a build.

### Launch and lifecycle

- Two entry points, one implementation:
  - `/accelerator:visualise` — slash command; launches the server, prints the
    URL, backgrounds the process.
  - `accelerator-visualiser` — standalone CLI shell wrapper; same
    `launch-server.sh` code path; runs in a terminal the user keeps open.
- **Binary acquisition**: on first invocation for a given plugin version,
  `launch-server.sh` detects the host platform via `uname -s` / `uname -m`,
  downloads the matching binary from the plugin's GitHub Release, verifies
  its SHA-256 against a committed `checksums.json` manifest, marks it
  executable, and caches it under
  `${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/bin/`. Subsequent
  invocations within the same plugin version skip the download.
- **Port**: dynamically allocated. The server binds to `127.0.0.1` only.
- **Instance management**: one server per repo, scoped to that repo's
  `meta/`. Re-invocation detects a live instance via a PID file and reuses
  it rather than starting a duplicate.
- **Idle timeout**: 30 minutes without any HTTP or SSE activity, then clean
  shutdown. Also subject to an **owner-PID watch** — the server
  self-terminates if the Claude Code harness process exits.

### Preprocessor responsibilities

All configuration resolution lives in a bash preprocessor
(`skills/visualisation/visualise/scripts/launch-server.sh`) invoked by both
the slash command and the CLI wrapper — the Rust server itself does no
shell-outs. The preprocessor:

1. Reads `tmp` path via `scripts/config-read-path.sh tmp`.
2. Reads the path for each of the ten doc types via `config-read-path.sh`
   (`decisions`, `tickets`, `plans`, `research`, `review_plans`,
   `review_prs`, `validations`, `notes`, `prs`, `templates`).
3. Ensures `<tmp>/visualiser/` exists.
4. If `<tmp>/visualiser/server-info.json` exists and its PID is alive,
   prints the URL and exits (reuse).
5. Detects host platform via `uname -s` / `uname -m`, resolving to one of
   `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`.
6. Computes expected binary path at
   `${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/bin/accelerator-visualiser-<os>-<arch>`.
   If absent or its SHA-256 does not match the committed
   `skills/visualisation/visualise/bin/checksums.json` entry, downloads
   the binary from the GitHub Release tagged with the current plugin
   version (`curl -fsSL`), verifies SHA-256 against the committed
   manifest, and makes it executable. Errors out cleanly on network or
   checksum failure.
7. Writes `<tmp>/visualiser/config.json` with resolved paths and runtime
   config (owner PID, host, logging path, plugin version), execs the
   binary passing the config path, waits for `server-info.json` to
   appear, prints the URL.

**Dev override**: if environment variable `ACCELERATOR_VISUALISER_BIN` is
set to a path, the preprocessor uses that binary directly and skips the
download and checksum flow. Supports local `cargo build` workflows without
touching the release pipeline.

All visualiser runtime state — `config.json`, `server-info.json`,
`server-stopped.json`, `server.pid`, `server.log` — lives under
`<meta/tmp>/visualiser/`, which is already gitignored by
`/accelerator:init`.

## Components

### Server (Rust)

The server is a single `axum`-based binary composed of focused modules. All
module names below refer to Rust modules (`src/<name>.rs` or
`src/<name>/mod.rs`). Behaviour interfaces are Rust traits; swappable
implementations (e.g. a future `GithubFileDriver`) plug in as alternative
trait impls.

- **`main` / `bootstrap`** — reads `config.json`, builds the shared
  application state (`Arc<AppState>`), initialises file driver, indexer,
  watcher, SSE hub, attaches routes, writes `server-info.json` when the
  listener is ready.
- **`file_driver`** — the only abstraction over disk access. Trait
  `FileDriver` with methods `list(type)`, `read(path)`,
  `write_frontmatter(path, patch, if_etag)`, `watch(callback)`. Body writes
  are intentionally absent. A `LocalFileDriver` impl is provided in v1; the
  trait exists so a future `GithubFileDriver` can be dropped in without
  touching callers.
- **`indexer`** — in-memory index of every doc:
  `{ type, path, rel_path, slug, frontmatter, title, mtime, size, etag }`.
  Kept in an `RwLock<HashMap<PathBuf, IndexEntry>>` plus precomputed
  cluster / ID lookup indices. Updated on watcher events with a 100ms
  per-path debounce. Frontmatter parsed via `gray_matter` + `serde_yml`;
  tolerates absent frontmatter (distinct state — not an error) and
  malformed frontmatter (indexed raw-only; emits `doc-invalid`).
- **`watcher`** — wraps the `notify` crate with per-path debouncing.
  Subscribes to each configured doc-type directory (flat walk per type).
- **`sse_hub`** — broadcasts `{ type: "doc-changed", docType, path, etag }`
  to subscribers on every index update. Built on `tokio::sync::broadcast`;
  back-pressures slow consumers by dropping older messages.
- **`patcher`** — the YAML-aware line patcher for frontmatter writes.
  Targets one allowlisted key (`status` on tickets), preserves comments,
  key order, surrounding whitespace; refuses unknown keys and values.
- **`routes`** — axum `Router`. All document paths in URLs are the
  document's repo-root-relative path; `{*path}` wildcards capture the full
  relative path:
  - `GET /api/types` — configured doc types and their paths. `templates`
    returns with `dirPath: null` and `virtual: true`.
  - `GET /api/docs?type=…` — for the nine ordinary types, index
    entries (`IndexEntry[]`). For `type=templates`, a
    `TemplateSummary[]` carrying per-tier presence info (see
    Templates subsection under Data model).
  - `GET /api/docs/{*path}` — one doc (frontmatter + raw markdown). Sends
    `ETag` header; supports `If-None-Match` → `304`.
  - `GET /api/docs/templates/{name}` — a `TemplateDetail` containing
    all three tiers (plugin default always present). Separate from
    `GET /api/docs/{*path}` because templates aren't addressed by a
    single disk path.
  - `GET /api/lifecycle` — slug clusters.
  - `GET /api/lifecycle/{slug}` — one cluster, ordered.
  - `PATCH /api/docs/{*path}/frontmatter` — requires `If-Match: <etag>`.
    Only write endpoint in v1.
  - `GET /api/events` — SSE stream.
  - `GET /{*path}` — serve the frontend bundle embedded into the binary
    via `rust-embed` (default `embed-dist` feature). The `dev-frontend`
    Cargo feature swaps in `tower-http::services::ServeDir` reading
    `frontend/dist/` from disk, for fast local iteration. In both
    modes, unknown paths fall back to `index.html` for SPA routing.

Wire-format JSON for every API endpoint matches the TypeScript types shown
under Data model; server-side types derive `serde::Serialize` /
`serde::Deserialize` with `#[serde(rename_all = "camelCase")]` to match.

### Frontend

- **Router** — TanStack Router with a typed route tree:
  - `/` → redirect to `/library`.
  - `/library`, `/library/:type`, `/library/:type/:filename` (where
    `:filename` is the file's name without the `.md` extension, preserving
    any date or ID prefix; round-trippable with disk).
  - `/lifecycle`, `/lifecycle/:slug`.
  - `/kanban`.
- **Query layer** — TanStack Query client with an `EventSource` listener that
  dispatches invalidations based on the incoming `doc-changed` event. Self-
  caused changes — where the event's ETag matches what Query already has —
  skip the refetch.
- **Shell** — the sidebar layout: doc types grouped on top, views (Kanban,
  Lifecycle) below, Templates grouped under "Meta" and visually
  de-emphasised.
- **Views**:
  - `library/*` — tree of docs per type; markdown renderer; frontmatter
    chips; "Related artifacts" aside for cluster + declared links.
  - `lifecycle/*` — cluster index cards and a vertical timeline per cluster.
  - `kanban` — three columns with dnd-kit sortable lists and
    optimistic mutations.
- **Markdown** — CommonMark + GFM, syntax highlighting, Mermaid in fenced
  ```mermaid``` blocks. Wiki-link resolution in v1 covers `[[ADR-NNNN]]` and
  `[[TICKET-NNNN]]` forms (resolved via the index's ID lookups); other forms
  render as plain text and are left for a later iteration.

## Data model

TypeScript types below describe the **wire format** the frontend sees. The
server mirrors these with `serde`-annotated Rust structs that serialise to
the same shape.

### DocType

```ts
type DocTypeKey =
  | "decisions" | "tickets" | "plans" | "research"
  | "plan-reviews" | "pr-reviews"
  | "validations" | "notes" | "prs" | "templates";

interface DocType {
  key: DocTypeKey;
  label: string;          // "Decisions", "Plan reviews", …
  dirPath: string | null; // resolved absolute path, from preprocessor config.
                          // null for the virtual `templates` type (backed by
                          // the three-tier resolver, not a single directory).
  inLifecycle: boolean;   // false for templates
  inKanban: boolean;      // true only for tickets
  virtual?: boolean;      // true for `templates`; omitted otherwise
}
```

`plan-reviews` reads from the `review_plans` configured path, `pr-reviews`
from `review_prs`. Both are flat walks of their respective directories —
no recursion is required because the two-type split absorbs the physical
nesting of `meta/reviews/plans/` and `meta/reviews/prs/`.

`templates` is backed by three-tier resolution rather than a directory
walk. See the "Templates" subsection below for the data shape the
server exposes.

### IndexEntry

```ts
interface IndexEntry {
  type: DocTypeKey;
  path: string;                   // absolute
  relPath: string;                // relative to repo root
  slug: string;                   // see "Slug derivation"
  frontmatter: Record<string, unknown>;
  title: string;                  // frontmatter.title || first H1 || filename
  mtime: number;                  // display/sort only
  size: number;
  etag: string;                   // "sha256-<hex>"
}
```

### Slug derivation

Deterministic, per type:

- **tickets** — strip leading `NNNN-`. `0001-three-layer-review-system-architecture` → `three-layer-review-system-architecture`.
- **decisions** — strip leading `ADR-NNNN-`. `ADR-0002-three-layer-review-architecture` → `three-layer-review-architecture`.
- **plans / research / validations / notes / prs** — strip leading `YYYY-MM-DD-`. `2026-02-22-pr-review-agents` → `pr-review-agents`.
- **plan-reviews / pr-reviews** — strip leading `YYYY-MM-DD-` **and** trailing `-review-N` suffix. `2026-03-27-remaining-configuration-features-review-1` → `remaining-configuration-features`. Without the suffix strip, reviews do not cluster with their target plan or PR.
- **templates** — no slug; excluded from lifecycle.

Docs with the same intent cluster by slug regardless of where they live.
Slug alignment across doc types is an authoring discipline; the visualiser's
lifecycle view makes gaps visible and is the forcing function for improving
that discipline. Explicit frontmatter-declared links are recognised when
present and rendered in v1 (see "Cross-references").

### Templates (virtual doc type)

Templates don't occupy a single directory — they resolve across three
tiers, with the highest-tier hit winning in the plugin's existing
`config_resolve_template()` logic (`scripts/config-common.sh`). The
visualiser exposes **all three tiers** per template so users can
preview what each template looks like regardless of current
configuration.

Tiers, highest-priority first:

| Tier | Source         | Path                                                                    | Presence |
|------|----------------|-------------------------------------------------------------------------|----------|
| 1    | Config override | value of `templates.<name>` in `.claude/accelerator.md` / `.local.md` | optional |
| 2    | User override   | `<paths.templates>/<name>.md` (default `meta/templates/<name>.md`)    | optional |
| 3    | Plugin default  | `<plugin-root>/templates/<name>.md`                                    | always present |

The authoritative list of template names is the set of files matching
`<plugin-root>/templates/*.md` (current names: `adr`, `plan`,
`research`, `validation`, `pr-description`). Files placed in tier 2 or
tier 1 that don't match a tier-3 name are "orphans" and surface under
a separate "unregistered" badge — they don't appear in the canonical
template index.

Wire-format types:

```ts
type TemplateTierSource = "config-override" | "user-override" | "plugin-default";

interface TemplateTier {
  source: TemplateTierSource;
  path: string;             // absolute; present even when `present: false`
  present: boolean;
  active: boolean;          // true for exactly one tier when `present`
  content?: string;         // omitted unless the detail endpoint requests it
  etag?: string;            // "sha256-<hex>"; present when `present: true`
}

interface TemplateSummary {
  name: string;             // e.g. "adr"
  tiers: TemplateTier[];    // always 3 entries in priority order; content omitted
  activeTier: TemplateTierSource;  // the winning (present, highest-priority) tier
}

interface TemplateDetail {
  name: string;
  tiers: TemplateTier[];    // always 3 entries; `content` populated on present tiers
  activeTier: TemplateTierSource;
}
```

Endpoints:

- `GET /api/docs?type=templates` returns `TemplateSummary[]` — the
  five template names with per-tier presence and the active tier,
  content omitted.
- `GET /api/docs/templates/{name}` returns a `TemplateDetail` — full
  content for every present tier, in priority order; the plugin-default
  tier is always present.

Watch behaviour: tier 3 (plugin default) is not watched — it only
changes on plugin upgrade, which restarts the server. Tiers 1 and 2
are watched when present so live edits reflect immediately in the
UI.

### LifecycleCluster

```ts
interface LifecycleCluster {
  slug: string;
  title: string;                      // derived from the richest member
  entries: IndexEntry[];              // ordered by doc-type role, then mtime
  completeness: {
    hasTicket: boolean;
    hasResearch: boolean;
    hasPlan: boolean;
    hasPlanReview: boolean;
    hasValidation: boolean;
    hasPr: boolean;
    hasPrReview: boolean;
    hasDecision: boolean;
  };
}
```

Timeline ordering: ticket → research → plan → plan-review → validation →
PR → pr-review → decision → notes (grouped). Within a doc type, order by
mtime ascending.

### Kanban state

- Ticket status lives in `frontmatter.status`.
- Allowed values in v1: `todo`, `in-progress`, `done`.
- Unknown values render in a visible read-only "Other" swimlane so nothing is
  lost silently.

### Cross-references

v1 renders three kinds of links:

1. **Same-cluster links** — inferred from slug match. Rendered as a
   "Related artifacts" aside, tagged `(inferred)`.
2. **Explicit frontmatter links** — when a doc declares any recognised
   cross-reference field, rendered as first-class links, tagged
   `(declared)`. The only cross-ref field populated in the wild today is
   `target:` on plan-reviews (a full repo-relative path to the reviewed
   plan). The visualiser renders this **bidirectionally**: the review's
   library page links to the target plan, and the target plan's library
   page lists all reviews pointing at it. Other recognised fields
   (`ticket:`, `supersedes:`, etc.) activate automatically the moment
   authoring skills populate them — no parser change needed.
3. **Wiki-style body refs** — `[[ADR-NNNN]]` and `[[TICKET-NNNN]]` in
   markdown resolve via the index's ID lookups (`adr_id` / filename
   prefix for ADRs; filename numeric prefix for tickets — the `TICKET-`
   prefix is stripped at resolution time). Bare `[[NNNN]]` is
   intentionally unsupported so the prefix namespace stays free for
   future ID kinds (e.g. `[[EPIC-NNNN]]`).

The visible distinction between inferred and declared links is the seed
for a future "promote inferred to explicit" affordance.

## Views

### Library

- `/library/:type` — index page with a sortable table: title, status/date
  badge, slug, last-modified.
- `/library/:type/:fileSlug` — doc page with a header (title, frontmatter
  chips, breadcrumb), body (rendered markdown), and an aside containing
  "Related artifacts" and file metadata including an "Open in editor" action.

### Lifecycle

- `/lifecycle` — an index of clusters. Each cluster is a card with a
  horizontal pipeline of dots (ticket · research · plan · plan-review ·
  validation · PR · pr-review · decision) filled or empty per
  `completeness`. Sortable by most recently changed, age, or completeness.
- `/lifecycle/:slug` — a single cluster rendered as a vertical timeline. Each
  artifact is a card (type, date, title, body preview, link to library).
  Missing stages appear as faded placeholders labelled "no plan yet", "no
  review yet", etc.

### Kanban

- `/kanban` — three columns: `todo`, `in-progress`, `done`.
- Each card shows ticket number, title, type, and last-modified.
- Drag-drop flow:
  1. Drop fires an optimistic mutation: the card moves immediately.
  2. `PATCH /api/docs/tickets/:path/frontmatter` with `If-Match: <etag>`.
  3. On `204`, the inbound SSE `doc-changed` event reconciles silently.
  4. On `412 Precondition Failed`, the card snaps back, a toast explains the
     external change, and the query is invalidated.
- Visible "Other" swimlane for non-standard status values, preserved
  read-only.
- No type filters or swimlanes in v1 (all existing tickets are currently one
  type).

### Cross-cutting UX

- Deep links everywhere. All three views are addressable via URL.
- Inferred vs declared links are always visually distinct.
- Live-update indicator: a subtle pulse on the sidebar badge of any doc type
  with unseen changes.
- Keyboard: `/` focuses a search-box stub that filters the current list only
  in v1. Proper search is v2.

## Writes and conflict handling

### The only write path: kanban status

`PATCH /api/docs/tickets/:path/frontmatter`

- Request headers: `If-Match: <etag>`.
- Request body: `{ "patch": { "status": "in-progress" } }`.
- Validation:
  - `status` is the only patchable field in v1.
  - Value must be one of `todo`, `in-progress`, `done`.
  - Path must resolve (via `std::fs::canonicalize`) inside the configured
    tickets dir.
- Server steps inside `LocalFileDriver::write_frontmatter`:
  1. Look up cached ETag; if `If-Match` mismatches → `412` with current
     ETag.
  2. Read the file; run the YAML-aware line patcher (in the `patcher`
     module) that replaces only the `status:` line in the frontmatter
     block, preserving comments, key order, and surrounding whitespace.
  3. Write atomically via `tempfile::NamedTempFile::persist` — write to a
     sibling `<name>.tmp` then rename onto the target path.
  4. Recompute ETag; update the indexer; broadcast `doc-changed` through
     the SSE hub.
- Response: `204 No Content` + fresh `ETag` header.

### ETag definition

- Strong validator (`"sha256-<hex>"`, no `W/` prefix).
- Content hash of the full file bytes, computed via the `sha2` crate
  (`Sha256`). Fast enough for markdown; can swap for a faster hash later
  if profiling demands.
- Computed by the indexer on initial scan and on every watcher change;
  cached per file. Read handlers pull from cache.

### Live updates

- The `notify` crate watches each configured doc-type directory
  (non-recursive — the ten per-type roots are flat). On macOS this is
  backed by FSEvents; on Linux, inotify.
- On `add | change | unlink`, the indexer re-reads (or removes) and
  broadcasts `doc-changed` with the new ETag.
- Events are debounced 100ms per path to coalesce editor-save chatter.
- The frontend's SSE listener maps each event to query invalidations:
  - Ticket change → `["doc", path]`, `["tickets"]`, `["kanban"]`.
  - Any doc change → `["lifecycle", slug]` if the slug is known.

### Failure modes

| Case | Server | Client |
|---|---|---|
| File removed between read and write | `404` | Toast "file deleted externally"; remove card |
| Stale `If-Match` on PATCH | `412 { currentEtag }` | Snap card back; toast; invalidate query |
| Disallowed field or value in PATCH | `400` | Toast "invalid change"; no mutation retained |
| Malformed frontmatter on read | Index with raw-only; emit `doc-invalid` | Doc page banner: "Frontmatter unparseable; showing raw content" |
| Disk error on write | `500 { message }` | Toast with retry; card returns to origin |
| SSE disconnect | — | Auto-reconnect with exponential backoff; invalidate-all on reconnect |
| Two servers racing on same repo | Preprocessor detects live PID; reuses | n/a |
| Port already in use | Server tries next free port | Preprocessor reads `server-info.json` for actual port |

## Testing strategy

### Unit (cargo test, server side)

Rust unit tests colocated with each module (`#[cfg(test)] mod tests`):

- `file_driver` — read; surgical frontmatter patch preserves comments,
  order, trailing whitespace; atomic rename write; symlink-escape
  rejection via `canonicalize` + prefix check.
- `slug` — one table-driven test per doc type covering prefix and
  (for plan-reviews / pr-reviews) suffix patterns.
- `indexer` — constructs correct clusters from a synthetic tree; updates
  correctly on add / change / unlink; debounces noisy event streams;
  distinguishes absent vs malformed frontmatter.
- `sse_hub` — subscribers receive broadcasts; drops oldest on slow
  consumers (`tokio::sync::broadcast` channel overflow path).
- `patcher` — targets only the requested field, rejects disallowed
  keys/values, idempotent on same-value input, preserves quoted /
  unquoted / commented frontmatter layouts.

### Integration (cargo test + axum test client)

- Use `tower::ServiceExt::oneshot` to drive the full router against a
  tmp directory seeded with fixture docs.
- `GET /api/docs/{*path}` returns a correct ETag; `If-None-Match` →
  `304`.
- `PATCH /api/docs/{*path}/frontmatter`: success returns `204` + new
  ETag; stale `If-Match` returns `412`; unknown status value returns
  `400`.
- SSE: subscribe to `/api/events`, mutate a file, assert exactly one
  `doc-changed` event with the fresh ETag.
- Startup: missing `config.json` → clean error; missing doc-type dir →
  indexed as empty, no crash.

### End-to-end (Playwright, frontend)

- Kanban golden path: drag card → column changes → disk `status:`
  updates; a second browser tab receives the move via SSE.
- Conflict path: hand-edit a ticket's `status:` on disk while the UI
  has a stale ETag → PATCH returns `412` → toast → card snaps back.
- Library → Lifecycle → ticket deep-link round trip.
- Markdown rendering smoke: Mermaid renders, wiki-link resolves.
- Binary acquisition smoke: with no cached binary present, a fresh
  invocation downloads, verifies, and launches successfully.

### Fixtures

- `tests/fixtures/meta/`: 3–5 docs per type including deliberately
  absent-frontmatter, malformed-frontmatter, and `-review-N` suffix
  cases. Committed. Used by both integration and Playwright suites.

## Non-functional

- **Performance**: initial scan of up to ~2000 files under 1s; live
  events land in the UI within 250ms of the write. Larger repos out of
  scope for v1.
- **Security**: server binds to `127.0.0.1` only. No auth (localhost
  trust). Same-origin CORS. Path-escape guard via
  `std::fs::canonicalize` + prefix check against the resolved doc-type
  directories. PATCH allowlist restricts writable fields to `status`
  on tickets. No shell execution from within the Rust process. Binary
  downloads verified against a committed SHA-256 manifest (see
  Distribution).
- **Accessibility**: keyboard-navigable kanban (dnd-kit supports this),
  visible focus rings, WCAG AA contrast on the default theme. Full
  audit deferred.
- **Observability**: JSON-line logs to `<tmp>/visualiser/server.log` —
  startup, config snapshot, request summary (method, path, status,
  duration), SSE subscriber count, FS events. Rotated at 5MB.
  Structured via `tracing` + `tracing-subscriber` with a JSON layer.
- **Distribution**: the server is distributed as **per-arch static
  binaries published on GitHub Releases**. Each binary is a single
  artefact containing both the compiled server and the embedded
  frontend bundle (via `rust-embed`). The plugin repo ships:
  - Rust sources under `skills/visualisation/visualise/server/src/`.
  - Frontend sources under `skills/visualisation/visualise/frontend/src/`.
  - A committed `skills/visualisation/visualise/bin/checksums.json`
    manifest mapping `<os>-<arch>` → SHA-256 of the binary for the
    current plugin version.
  No built artefacts are committed: `frontend/dist/` and the per-arch
  binaries are both gitignored.
  Release process, in strict order: on version bump, maintainers run
  (1) `npm ci && npm run build` in `frontend/` to produce a fresh
  `dist/`; (2) `cargo zigbuild --release --target …` for the four
  targets (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
  `aarch64-unknown-linux-musl`, `x86_64-unknown-linux-musl`) — the
  default `embed-dist` Cargo feature embeds `../frontend/dist/` into
  each binary, and a `build.rs` check fails the build if `dist/` is
  missing; (3) compute checksums, update `checksums.json`, commit,
  tag the release, and attach the four binaries as release assets
  named `accelerator-visualiser-<os>-<arch>`. End users trigger a one-time
  download per plugin version via `launch-server.sh` (see
  Preprocessor responsibilities). No Rust, Node, or npm required on
  the end-user machine. Cached binaries live at
  `${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/bin/accelerator-visualiser-<os>-<arch>`
  and are covered by the plugin's own `.gitignore` for dev checkouts.
  Network is required only on the first run per plugin version;
  air-gapped environments can pre-stage binaries by env-var override
  (`ACCELERATOR_VISUALISER_BIN`).
- **Supported platforms**: macOS (arm64, x64) and Linux (arm64, x64).
  Windows is not supported in v1 — accelerator as a whole currently
  targets macOS/unix + Claude Code.
- **Versioning**: server serves `X-Accelerator-Visualiser: <semver>`
  from a `const VERSION: &str = env!("CARGO_PKG_VERSION")`; the UI
  shows the version in a sidebar footer. Client/server mismatch logs
  a warning but does not hard-fail. Binary version is bound to plugin
  version via release tags.
- **Failure isolation**: one bad file produces a warning and an
  "invalid" badge in the UI but does not break the view.
- **Shutdown**: `SIGTERM` / `SIGINT` flush SSE subscribers, stop the
  `notify` watcher, remove `server-info.json`, write
  `server-stopped.json`, and exit. Idle timeout (30 min, configurable)
  and owner-PID death both trigger the same path.

## Key decisions

| Decision | Choice | Reason |
|---|---|---|
| File-access model | Local HTTP server spawned by skill or CLI | Writes are trivial; live watching matches the companion use case; mirrors the `superpowers:brainstorming` precedent; future GitHub driver is a swap of one trait impl. |
| Server implementation language | Rust (axum + tokio + notify + gray_matter + serde_yml + sha2) | Small static binaries (~6-10 MB per arch); zero runtime dependency on the end-user machine; strong cross-platform file-watching via `notify`; excellent perf and correctness guarantees for the YAML-aware patcher and path-safety code. |
| Server framework | `axum` | Tokio-native, trait-based routing composes cleanly with the file-driver trait; excellent SSE support; industry-standard Rust HTTP. |
| Static file serving | `rust-embed` (release) / `tower-http::services::ServeDir` (dev) | Default `embed-dist` Cargo feature embeds `frontend/dist/` into each release binary — one artefact carries server + frontend, and `dist/` stays uncommitted. `dev-frontend` feature opts in to disk-based `ServeDir` for fast local iteration without Rust rebuilds. |
| Binary distribution | GitHub Releases + download-on-first-run | Keeps the plugin repo small (no ~30-40 MB of committed binaries); release assets are versioned by tag, pinned by plugin version; SHA-256 checksums committed in the plugin tree gate verification even if the release is tampered with. |
| Cross-compilation | `cargo zigbuild` on a single macOS host | One-command compile for all four targets; musl for Linux gives truly static binaries (no glibc version coupling). |
| Frontend stack | React + Vite + TanStack Router + TanStack Query + dnd-kit, TypeScript | User preference for React; TanStack Query is a natural fit for server state + SSE invalidation + optimistic mutations; dnd-kit is the current best-in-class DnD lib. |
| Launch | Slash command + standalone CLI, one `launch-server.sh` implementation | Covers both the companion and pre/post-flight use cases without a second codebase. |
| Port | Dynamic | Avoids collisions across multiple accelerator repos. |
| Doc-type list | Hardcoded to the ten known types | Simplicity for v1; auto-discovery deferred to v2+. |
| Path resolution | Done by bash preprocessor, handed to the Rust binary via `config.json` | Keeps the server pure; avoids shelling out from Rust; uses the existing `scripts/config-read-path.sh`. |
| State location | `<meta/tmp>/visualiser/` | Reuses the existing gitignored `tmp` convention from `/accelerator:init`. |
| Lifecycle anchor | Slug cluster | Works today without retrofitting frontmatter; the visualiser makes gaps visible and nudges authoring toward cleaner slugs. |
| Kanban columns | `todo`, `in-progress`, `done` | Matches and minimally extends the statuses in use today. |
| Writes in v1 | Only `status:` on tickets | Scopes one surface to test and get right; avoids body-edit conflicts entirely. |
| Conflict detection | HTTP ETags (SHA-256 content hash via `sha2`) with `If-Match` | Standard HTTP semantics; resolution-free; enables conditional GETs and cheap SSE reconciliation. |
| Live updates | Server-Sent Events | One-way is sufficient; simpler than WebSockets; axum's SSE support is first-class. |
| Reviews type modelling | Two distinct DocTypes (`plan-reviews`, `pr-reviews`) | The config system splits the paths (`review_plans` / `review_prs`); modelling as two types keeps each walk flat and avoids recursive-watch complexity. |
| Templates modelling | Virtual doc type backed by the three-tier resolver; the library shows every tier per template | Mirrors the plugin's `config_resolve_template()` semantics (config override > user override > plugin default); users preview what a template renders to regardless of their current configuration, and drift between "what I configured" and "what I'd get" is always visible. |

## Roadmap (post-v1)

- Search and filter across all docs.
- Activity feed surfacing recent changes.
- Knowledge-graph view across declared + inferred links.
- Review dashboard: plans and PRs with their lens verdicts.
- "Promote inferred link to explicit" affordance — one-click writes a
  frontmatter link inferred from a slug cluster.
- Inline frontmatter editing beyond kanban (tags, status on non-ticket docs).
- Type filters / swimlanes on the kanban once more ticket types exist.
- Multiple kanban workflows declared per type.
- GitHub-backed `FileDriver` for a deployed mode.
- Authentication (only meaningful for deployed mode).
