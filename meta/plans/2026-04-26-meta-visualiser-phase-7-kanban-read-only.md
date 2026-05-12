---
date: "2026-04-26T00:00:00+01:00"
type: plan
skill: create-plan
status: draft
---

# Meta visualiser Phase 7 — Kanban read-only

## Overview

Phase 7 turns the existing `KanbanStub` placeholder into a working
read-only kanban board. By the end of this phase, navigating to
`/kanban` shows three columns (`Todo`, `In progress`, `Done`) plus a
visible-only-when-non-empty `Other` swimlane, with one card per ticket
placed by its `frontmatter.status` value. Cards show the ticket number,
title, type, and last-modified time, and link to the existing library
detail page (`/library/tickets/$fileSlug`). External edits to a ticket's
`status:` field land in the UI within ~250 ms via the SSE invalidation
already wired up in `useDocEvents`.

Phase 7 is a **deliberately read-only** rendering of the data that
phases 1–4 already put on the wire. The board uses `@dnd-kit` from day
one — `DndContext`, `SortableContext`, and per-card `useSortable` —
but with `disabled: true` set on every sortable. This preserves the
ARIA annotations dnd-kit emits (so screen readers see a structured
list) while making drag a no-op. Phase 8 flips the `disabled` flag and
adds the write path; Phase 7 leaves no provider re-threading work for
Phase 8.

The approach is test-driven throughout. Every pure helper has its
unit tests written first; every component has Vitest + React Testing
Library tests written before the implementation; the one server-side
change (adding ticket fixtures and asserting `frontmatter.status`
flows through `/api/docs?type=tickets`) lands as a failing test first.

## Current state

Phases 1–6 are complete. Specifically, for Phase 7 the relevant pieces
are:

- **`server/src/docs.rs:69-71`** — `DocTypeKey::Tickets.in_kanban()`
  returns `true`; this invariant is already pinned by a unit test at
  `docs.rs:148-156`.
- **`server/src/indexer.rs:15-30`** — `IndexEntry` exposes
  `frontmatter: serde_json::Value` verbatim, so `frontmatter.status`
  is already on the wire for tickets via
  `/api/docs?type=tickets`. There is no typed `status` field on
  `IndexEntry` and Phase 7 does not add one.
- **`server/src/indexer.rs:100-104, 165-168`** — the indexer keys
  tickets by their leading filename number into `ticket_by_number`.
  This is used by ADR-style cross-reference resolution but not exposed
  on `IndexEntry`. Phase 7 parses the ticket number client-side from
  `relPath` instead.
- **`server/tests/common/mod.rs:6-70`** — `seeded_cfg` lays down
  fixtures for decisions, plans, and reviews, but the `tickets`
  directory is registered in `doc_paths` without any fixture files
  (and without `mkdir`). Adding ticket fixtures here is the
  one-server-side touch Phase 7 needs.
- **`frontend/src/api/query-keys.ts:12`** — `queryKeys.kanban() =
  ['kanban']` exists but is unused by any view today. The kanban view
  in this phase reads from the already-populated
  `queryKeys.docs('tickets')` cache; the `kanban()` key remains
  reserved for future kanban-specific server data.
- **`frontend/src/api/use-doc-events.ts:17-30`** — already invalidates
  `queryKeys.docs(event.docType)` (and `queryKeys.kanban()`) on every
  `doc-changed` / `doc-invalid` event whose `docType === 'tickets'`.
  No changes needed.
- **`frontend/src/api/use-doc-events.ts:61-64`** — the SSE-disconnect
  handler invalidates the bare `['docs']` prefix, which catches
  `['docs', 'tickets']`. No additional invalidation needed for
  Phase 7.
- **`frontend/src/api/types.ts`** — declares `DocTypeKey`,
  `IndexEntry`, `DocsListResponse`, `SseEvent`, etc., but has no
  `KanbanColumnKey` / `STATUS_COLUMNS` constants.
- **`frontend/src/api/format.ts:1-10`** — `formatMtime(ms, now?)`
  already supports the kanban card's last-modified rendering.
- **`frontend/src/api/path-utils.ts:6-9`** — `fileSlugFromRelPath`
  already provides the round-trippable file slug used for the library
  deep link.
- **`frontend/src/api/test-fixtures.ts:5-19`** — `makeIndexEntry`
  factory exists for use in TS tests.
- **`frontend/src/api/fetch.ts:23-28`** — `fetchDocs(type)` already
  returns `IndexEntry[]`; no new fetch helper is required. **Caveat**:
  it currently throws `new Error(...)` rather than `FetchError`, so the
  kanban error-branching needs the migration in Step 3 to be meaningful.
- **`frontend/src/api/fetch.ts:10-15`** — typed `FetchError` exists but
  is only thrown by the lifecycle helpers; Step 3 migrates the rest of
  the fleet (`fetchTypes`, `fetchDocs`, `fetchDocContent`,
  `fetchTemplates`, `fetchTemplateDetail`) for parity.
- **`frontend/src/components/Sidebar/test-helpers.tsx:1-11`** — existing
  `MemoryRouter` test helper builds a root route whose component
  renders the supplied children, but registers no other routes. Step 4
  relocates it to `frontend/src/test/router-helpers.tsx` and extends it
  with the library doc route so `<Link to="/library/$type/$fileSlug">`
  resolves under tests.
- **`frontend/src/router.ts:98-102`** — `kanbanRoute` exists and
  renders `KanbanStub` as a leaf component (no children).
- **`frontend/src/routes/kanban/KanbanStub.tsx:1-3`** — one-line
  placeholder; will be deleted.
- **`frontend/src/components/Sidebar/Sidebar.tsx:5-8, 53`** — the
  `Kanban` sidebar entry exists; active-state matches strict
  pathname equality, which suffices because Phase 7 introduces no
  child routes.
- **`frontend/src/test/setup.ts:1-26`** — stubs `EventSource` but
  does not stub `ResizeObserver` or `Element.prototype.scrollIntoView`
  (both used by dnd-kit measuring code).
- **`@dnd-kit/*`** — not yet installed (`frontend/package.json:18-26`).
- **`meta/tickets/`** — 29 ticket files, all with
  `type: adr-creation-task` and `status: todo` or `status: done`. No
  `in-progress` and no exotic statuses in real data; the kanban must
  still render correctly when synthetic test fixtures or future
  authoring skills produce other values.

## Desired end state

- Visiting `http://localhost:<port>/kanban` renders a page-level
  `<h1>Kanban</h1>` followed by three labelled columns — `Todo`,
  `In progress`, `Done` — each with an `<h2>` heading and a count
  badge, and containing zero or more ticket cards sorted by `mtimeMs`
  descending with `relPath` as a deterministic tie-break. Below the
  three columns, an `Other` swimlane appears **only when** at least
  one ticket has a status value that isn't `todo` / `in-progress` /
  `done` (or has missing / non-string status / non-parsed
  frontmatter); when no such tickets exist, the swimlane is not
  rendered at all. The `Other` swimlane carries an explanatory line
  ("Tickets whose status is missing or not one of: todo, in-progress,
  done.") so authors understand why their ticket landed there.
- Each ticket card shows: `#NNNN` (ticket number, four digits,
  zero-padded, parsed from `relPath`) — or, if the filename has no
  numeric prefix, the file slug as a defensive fallback identifier;
  the title; the type (from `frontmatter.type`, e.g.
  `adr-creation-task`); and a relative last-modified label
  (`formatMtime(entry.mtimeMs)`). Each card is itself a
  `<Link to="/library/$type/$fileSlug" params={{ type: 'tickets',
  fileSlug }}>` whose `fileSlug` is derived via
  `fileSlugFromRelPath(entry.relPath)` — matching the canonical
  typed-route form used everywhere else in the app.
- Each card is a dnd-kit sortable item (`useSortable({ id: <relPath>,
  disabled: PHASE_7_DISABLED })`) inside a per-column
  `SortableContext` with `verticalListSortingStrategy`, with
  `setNodeRef`, `attributes`, and `listeners` attached to the
  `<Link>` so there is one focus stop per card. The misleading
  `aria-roledescription="sortable"` that dnd-kit emits is
  destructured away while the sortable is disabled — Phase 8 will
  re-include it when drag is actually wired. The whole board is
  wrapped in one `DndContext` with no-op `onDragEnd` / `onDragOver` /
  `onDragStart`, `closestCorners` collision detection, and both
  `PointerSensor` + `KeyboardSensor` (the latter using
  `sortableKeyboardCoordinates`). All `useSortable` hooks set
  `disabled: true`, so no drag interaction is possible; sensors are
  pre-mounted purely so Phase 8 can flip a single flag.
- Loading state renders inside the same `<h1>Kanban</h1>` page
  shell, with a `role="status"` paragraph announcing "Loading…" so
  AT users hear the transition. Error state renders the same shell
  plus a `role="alert"` containing the error message and a
  `Retry` button that calls
  `queryClient.invalidateQueries({ queryKey: queryKeys.docs('tickets') })`
  so users can recover without a full page reload. 5xx responses
  produce "The visualiser server returned an error. Try again in a
  moment."; everything else (including non-FetchError rejections)
  produces "Something went wrong loading the tickets." Internal
  status codes and URLs never leak into user-visible copy.
- Editing a ticket's `status:` on disk (via any external editor)
  causes the corresponding card to move columns within ~250 ms
  through the existing SSE → query-invalidation pipeline. The
  KanbanBoard test suite pins this contract directly by invalidating
  `queryKeys.docs('tickets')` and asserting a card moves between
  columns. No production code change is required to achieve this; it
  falls out of reading from `queryKeys.docs('tickets')`, which
  `dispatchSseEvent` already invalidates on every ticket change.
- The router renders `KanbanBoard` (not `KanbanStub`) at `/kanban`;
  `KanbanStub.tsx` is deleted.
- All `fetch.ts` helpers (`fetchTypes`, `fetchDocs`,
  `fetchDocContent`, `fetchTemplates`, `fetchTemplateDetail`) throw
  `FetchError` on non-2xx responses, bringing them into parity with
  the lifecycle helpers and making `error instanceof FetchError`
  meaningful at every consumer.
- A shared `frontend/src/test/router-helpers.tsx` exposes
  `MemoryRouter` (drop-in replacement for the old Sidebar helper)
  and `renderWithRouterAt(ui)`; the latter registers the library doc
  route so component tests rendering `<Link to="/library/$type/$fileSlug">`
  resolve cleanly.
- Server-side: the `seeded_cfg` test helper creates a
  `meta/tickets/` directory and writes three fixture tickets
  (`todo`, `done`, an exotic status); a new integration test in
  `server/tests/api_docs.rs` confirms `GET /api/docs?type=tickets`
  echoes the `frontmatter.status` field for each fixture verbatim
  AND that `frontmatterState` is `parsed` even for the exotic
  status (narrowing happens client-side, not server-side).
- `CHANGELOG.md`'s `## Unreleased` section gains an `### Added`
  bullet describing the new kanban view, the `Other` swimlane, the
  dnd-kit dependency, and live SSE-driven updates.
- All Rust tests pass (`mise run test:unit:visualiser` and
  `mise run test:integration:visualiser`).
- All frontend tests pass (`mise run test:unit:frontend`).
- `mise run test` is green end-to-end.

### Verification

Verifications cited inline in each step's "Success criteria" block.
At the phase level:

```bash
cd /Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/visualisation-system
mise run test:unit:visualiser   # server cargo --lib (no-default + default features)
mise run test:integration:visualiser   # server cargo --tests + shell suites
mise run test:unit:frontend     # Vitest
mise run test                   # everything
```

Manual:

1. `cd skills/visualisation/visualise/server && cargo run --features dev-frontend -- <path-to-config.json>` (or
   trigger via `/accelerator:visualise` once Phase 12 is shipped — for
   now, `cargo run` against a dev `config.json` is the dogfooding
   path).
2. Open `http://127.0.0.1:<port>/kanban`. Confirm three columns
   render with all 29 real tickets distributed between `Todo` and
   `Done` only (no `In progress` cards exist in the real data).
   Confirm the `Other` swimlane is **not** rendered.
3. In a separate editor, change a ticket's `status:` from `done` to
   `in-progress` and save. The card should hop from `Done` to
   `In progress` within a second.
4. Change the same ticket's `status:` to a synthetic value such as
   `blocked`. The card should appear in a newly-rendered `Other`
   swimlane. Reverting to `todo` makes the swimlane disappear again.
5. Tab through the page; each card should receive a visible focus
   ring from dnd-kit's `attributes` and remain reachable in
   document order.

## What we are NOT doing

- **Drag-and-drop, optimistic mutation, or any write path.** Cards
  are wrapped in `useSortable({ disabled: true })` and the
  `DndContext` handlers are no-ops. Phase 8 will flip the disabled
  flag and wire `PATCH /api/docs/.../frontmatter`, `If-Match` ETag,
  `412` conflict toast, and second-tab reconciliation.
- **A new `/api/kanban` endpoint or a `kanban`-keyed query.** Read-only
  kanban needs only the existing `GET /api/docs?type=tickets` data
  and groups it client-side. Adding a server endpoint or a separate
  `useQuery` keyed off `queryKeys.kanban()` would duplicate cache
  state for no benefit; the reserved `queryKeys.kanban()` key stays
  unused by views in v1. Reason: keep blast radius small; minimise
  Phase 8's surface.
- **Adding a typed `status` field to `IndexEntry`.** The frontmatter
  is already on the wire as a `Record<string, unknown>`; the kanban
  reads `entry.frontmatter.status` with runtime narrowing. A typed
  field would force a server-side schema decision (which enum? what
  about exotic values?) that the v1 spec deliberately defers.
- **Adding a `ticketNumber` field to `IndexEntry`.** Tickets are
  guaranteed to have an `NNNN-` filename prefix by
  `skills/tickets/scripts/ticket-next-number.sh:47-51`. Parsing on
  the client is one regex; a server field would just duplicate
  filename data.
- **Cross-references on cards.** "Related artifacts" / declared-link
  rendering is Phase 9.
- **Ticket-type filters or per-type swimlanes.** Spec explicitly
  defers this: "No type filters or swimlanes in v1 (all existing
  tickets are currently one type)."
- **Editing a non-status frontmatter field.** Out of scope for v1
  per spec.
- **`KanbanLayout` / nested kanban routes.** No child routes are
  introduced; `kanbanRoute` stays as a leaf with `KanbanBoard` as
  its component.
- **Reordering cards within a column by drag.** Even Phase 8 only
  introduces between-column moves (status mutation). Within-column
  ordering is a future feature; sort is fixed at `mtimeMs`
  descending in v1.
- **Sidebar active-state for child routes.** No child routes exist;
  the strict pathname equality at `Sidebar.tsx:53` works as-is.

---

## Implementation approach

1. Server: extend `seeded_cfg` with three ticket fixtures and add an
   integration test pinning `frontmatter.status` pass-through (test
   first).
2. Frontend: install `@dnd-kit/core`, `@dnd-kit/sortable`,
   `@dnd-kit/utilities`; add `ResizeObserver` and `scrollIntoView`
   stubs to `src/test/setup.ts`.
3. Frontend: migrate all fetch helpers (`fetchTypes`, `fetchDocs`,
   `fetchDocContent`, `fetchTemplates`, `fetchTemplateDetail`) to
   throw `FetchError` rather than plain `Error` — bringing them into
   parity with the lifecycle helpers so `error instanceof FetchError`
   is meaningful at every consumer (TDD).
4. Frontend: extend the existing `MemoryRouter` test helper to register
   the library doc route so `<Link to="/library/$type/$fileSlug">`
   resolves under tests, plus a `renderWithRouterAt(ui)` helper that
   renders arbitrary children at `/`.
5. Frontend: declare `STATUS_COLUMNS`, `OTHER_COLUMN_KEY`, and the
   `KanbanColumnKey` type in `src/api/types.ts`.
6. Frontend: `parseTicketNumber(relPath)` helper in
   `src/api/ticket.ts` (TDD).
7. Frontend: `groupTicketsByStatus(entries)` helper in the same file
   (TDD).
8. Frontend: `TicketCard` component (TDD).
9. Frontend: `KanbanColumn` component (TDD).
10. Frontend: `KanbanBoard` view (TDD; covers loading, error, empty,
    column rendering, Other-swimlane toggle, ticket placement,
    deep-link, SSE-driven re-grouping).
11. Frontend: router wiring — replace `KanbanStub` with `KanbanBoard`,
    delete the stub, extend `router.test.tsx`.
12. Append a `## Unreleased` `### Added` entry to `CHANGELOG.md`
    describing the read-only kanban view, the `Other` swimlane
    behaviour, the dnd-kit dependency, and live SSE-driven updates.

Each step is an independently committable unit; tests are written
before code in every step.

---

## Step 1: Server — ticket fixtures and `frontmatter.status` pass-through (TDD)

### Files

- `skills/visualisation/visualise/server/tests/common/mod.rs`
- `skills/visualisation/visualise/server/tests/api_docs.rs`

### 1a. Write the integration test first

In `server/tests/api_docs.rs`, append:

```rust
#[tokio::test]
async fn docs_list_for_tickets_carries_frontmatter_status() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=tickets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let docs = v["docs"].as_array().expect("docs array");

    // Three fixture tickets, statuses todo / done / blocked, distinct
    // filename prefixes so they survive `seeded_cfg` ordering changes.
    let by_status: std::collections::HashMap<&str, &str> =
        std::collections::HashMap::from([
            ("todo", "0001-todo-fixture"),
            ("done", "0002-done-fixture"),
            ("blocked", "0003-other-fixture"),
        ]);
    for (status, slug_prefix) in &by_status {
        let entry = docs
            .iter()
            .find(|d| {
                d["relPath"]
                    .as_str()
                    .map(|p| p.contains(slug_prefix))
                    .unwrap_or(false)
            })
            .unwrap_or_else(|| panic!("expected ticket {slug_prefix}"));
        assert_eq!(
            entry["frontmatter"]["status"].as_str(),
            Some(*status),
            "status pass-through broken for {slug_prefix}: got {entry}",
        );
        // Pin the frontmatterState the kanban relies on. The exotic
        // status `blocked` must still serialise as `parsed`; the kanban
        // narrows to "Other" client-side, not server-side.
        assert_eq!(
            entry["frontmatterState"].as_str(),
            Some("parsed"),
            "frontmatterState should be `parsed` for {slug_prefix}: got {entry}",
        );
    }
}
```

### 1b. Extend `seeded_cfg` to seed the three tickets

In `server/tests/common/mod.rs`, add inside `seeded_cfg`, before the
`Config { … }` return:

```rust
let tickets = meta.join("tickets");
std::fs::create_dir_all(&tickets).unwrap();
std::fs::write(
    tickets.join("0001-todo-fixture.md"),
    "---\ntitle: \"Todo fixture\"\ntype: adr-creation-task\nstatus: todo\n---\n# body\n",
)
.unwrap();
std::fs::write(
    tickets.join("0002-done-fixture.md"),
    "---\ntitle: \"Done fixture\"\ntype: adr-creation-task\nstatus: done\n---\n# body\n",
)
.unwrap();
std::fs::write(
    tickets.join("0003-other-fixture.md"),
    "---\ntitle: \"Blocked fixture\"\ntype: adr-creation-task\nstatus: blocked\n---\n# body\n",
)
.unwrap();
```

The existing `doc_paths.insert("tickets".into(), meta.join("tickets"))`
line at `common/mod.rs:48` already wires the tickets directory into
the config; only the actual files need to exist.

Cross-check: confirm no other integration test (notably
`api_lifecycle.rs`) relies on the tickets directory being empty. The
existing lifecycle test filters to `slug == "foo"`; the three new
ticket fixtures derive slugs `todo-fixture`, `done-fixture`,
`other-fixture` (after the `NNNN-` prefix strip in
`server/src/slug.rs`), so they form three single-entry clusters that
do not collide with the `foo` cluster the lifecycle test asserts on.

**Audit step (run before declaring 1b green):** because `seeded_cfg`
is shared by every integration test, count- or set-based assertions
elsewhere in the suite will see three additional ticket clusters /
docs after this change. Run:

```bash
cd skills/visualisation/visualise/server
rg -n 'len\(\)\s*[,;\)]' tests/ | rg -v 'tests/api_docs.rs'
rg -n 'docs\.len\(\)|clusters\.len\(\)' tests/
rg -n 'assert_eq!\([^,]*\.len\(\)' tests/
```

Triage each hit: assertions that scope by `type == "tickets"` or by a
specific slug (the `foo` cluster filter) are unaffected; assertions
that count *all* docs / *all* clusters need their expected counts
bumped or a more specific filter applied. Document any updates in the
commit message for this step. If the surface is too large to retrofit,
fall back to introducing a separate `seeded_cfg_with_tickets` helper
called only by `api_docs.rs`'s new test — keep `seeded_cfg` itself
unchanged.

### 1c. Confirm `IndexEntry` already JSON-serialises `frontmatter`
field-for-field

No code change. `indexer.rs:79-101` builds the
`serde_json::Value::Object` directly from the parsed YAML map; the
existing `parsed.state == FrontmatterState::Parsed(m)` branch
preserves every key, so `status: todo` lands in the wire body as
`"status": "todo"` without further work.

### Success criteria

```bash
cd skills/visualisation/visualise/server
cargo test --tests --no-default-features --features dev-frontend
# new test `docs_list_for_tickets_carries_frontmatter_status` passes;
# all pre-existing integration tests still pass
```

```bash
cd /Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/visualisation-system
mise run test:integration:visualiser
# green
```

---

## Step 2: Frontend — install `@dnd-kit` and stub jsdom polyfills

### Files

- `skills/visualisation/visualise/frontend/package.json`
- `skills/visualisation/visualise/frontend/package-lock.json`
- `skills/visualisation/visualise/frontend/src/test/setup.ts`

### 2a. Add dependencies

Run from `skills/visualisation/visualise/frontend/`:

```bash
npm install --save-exact \
  @dnd-kit/core@^6.3.1 \
  @dnd-kit/sortable@^10.0.0 \
  @dnd-kit/utilities@^3.2.2
```

These are the latest stable versions (verified for April 2026). Pin
with `--save-exact` so the lockfile and `package.json` agree
deterministically; `package.json:18-26` records them under
`dependencies` (not `devDependencies` — the bundle uses them at
runtime).

### 2b. Stub `ResizeObserver` and `scrollIntoView` in jsdom

dnd-kit calls `ResizeObserver` from its measuring loop and the
KeyboardSensor calls `Element.prototype.scrollIntoView` when focus
shifts. Neither exists in jsdom; missing them produces a
`ResizeObserver is not defined` ReferenceError on first render in any
test that mounts the board.

Extend `src/test/setup.ts` (after the existing `MockEventSource`
stub):

```ts
class MockResizeObserver {
  observe = vi.fn()
  unobserve = vi.fn()
  disconnect = vi.fn()
}

beforeAll(() => {
  vi.stubGlobal('EventSource', MockEventSource)
  vi.stubGlobal('ResizeObserver', MockResizeObserver)
  if (!Element.prototype.scrollIntoView) {
    Element.prototype.scrollIntoView = vi.fn()
  }
})
```

Merge the new global stub into the existing `beforeAll` rather than
adding a second `beforeAll` block — keeps the setup file
self-consistent.

### Success criteria

```bash
cd skills/visualisation/visualise/frontend
npm test
# pre-existing test count unchanged; no test failures introduced;
# no `ReferenceError: ResizeObserver is not defined` in any test output
```

```bash
cd skills/visualisation/visualise/frontend
npm run build
# tsc -b succeeds (the new dnd-kit types are picked up); vite build
# completes; bundle still produced under dist/
```

---

## Step 3: Frontend — migrate fetch helpers to `FetchError` (TDD)

### Files

- `skills/visualisation/visualise/frontend/src/api/fetch.ts`
- `skills/visualisation/visualise/frontend/src/api/fetch.test.ts` (new)

### Rationale

`FetchError` already exists at `fetch.ts:10-15`, but only the
lifecycle helpers throw it. `fetchTypes`, `fetchDocs`,
`fetchDocContent`, `fetchTemplates`, and `fetchTemplateDetail` still
throw plain `Error`. Phase 7's `KanbanBoard.errorMessageFor` needs
`error instanceof FetchError` to discriminate user-visible copy by
HTTP status; rather than upgrade only `fetchDocs` (leaving an
inconsistent fleet), we migrate every helper in one step. Phase 6
established the pattern; Phase 7 finishes the parity.

### 3a. Write a unit test pinning the new contract first

Create `frontend/src/api/fetch.test.ts`:

```ts
import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import {
  FetchError, fetchTypes, fetchDocs, fetchDocContent,
  fetchTemplates, fetchTemplateDetail,
} from './fetch'

const ORIGINAL_FETCH = globalThis.fetch

afterEach(() => {
  globalThis.fetch = ORIGINAL_FETCH
})

function mockFetch(status: number) {
  globalThis.fetch = vi.fn(async () => ({
    ok: false,
    status,
    statusText: 'err',
    headers: new Headers(),
    text: async () => '',
    json: async () => ({}),
  })) as unknown as typeof fetch
}

describe('fetch helpers throw FetchError on non-2xx responses', () => {
  it.each([
    ['fetchTypes',          () => fetchTypes()],
    ['fetchDocs',           () => fetchDocs('tickets')],
    ['fetchDocContent',     () => fetchDocContent('meta/tickets/0001-x.md')],
    ['fetchTemplates',      () => fetchTemplates()],
    ['fetchTemplateDetail', () => fetchTemplateDetail('foo')],
  ])('%s rejects with FetchError carrying the status', async (_name, call) => {
    mockFetch(503)
    await expect(call()).rejects.toBeInstanceOf(FetchError)
    try {
      await call()
    } catch (err) {
      expect(err).toBeInstanceOf(FetchError)
      expect((err as FetchError).status).toBe(503)
    }
  })

  it.each([
    ['fetchTypes',          () => fetchTypes(),                 404],
    ['fetchDocs',           () => fetchDocs('tickets'),         404],
    ['fetchDocContent',     () => fetchDocContent('foo.md'),    404],
    ['fetchTemplates',      () => fetchTemplates(),             404],
    ['fetchTemplateDetail', () => fetchTemplateDetail('foo'),   404],
  ])('%s rejects with FetchError(404)', async (_name, call, status) => {
    mockFetch(status)
    await expect(call()).rejects.toMatchObject({
      name: 'FetchError',
      status,
    })
  })
})
```

### 3b. Update each helper

In `fetch.ts`, replace each `throw new Error(...)` with
`throw new FetchError(r.status, ...)`:

```ts
export async function fetchTypes(): Promise<DocType[]> {
  const r = await fetch('/api/types')
  if (!r.ok) throw new FetchError(r.status, `GET /api/types: ${r.status}`)
  return r.json()
}

export async function fetchDocs(type: DocTypeKey): Promise<IndexEntry[]> {
  const r = await fetch(`/api/docs?type=${encodeURIComponent(type)}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/docs?type=${type}: ${r.status}`)
  const body: DocsListResponse = await r.json()
  return body.docs
}

export async function fetchDocContent(relPath: string): Promise<{ content: string; etag: string }> {
  const encodedPath = relPath.split('/').map(encodeURIComponent).join('/')
  const r = await fetch(`/api/docs/${encodedPath}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/docs/${relPath}: ${r.status}`)
  const content = await r.text()
  const etag = r.headers.get('etag') ?? ''
  return { content, etag }
}

export async function fetchTemplates(): Promise<TemplateSummaryListResponse> {
  const r = await fetch('/api/templates')
  if (!r.ok) throw new FetchError(r.status, `GET /api/templates: ${r.status}`)
  return r.json()
}

export async function fetchTemplateDetail(name: string): Promise<TemplateDetail> {
  const r = await fetch(`/api/templates/${encodeURIComponent(name)}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/templates/${name}: ${r.status}`)
  return r.json()
}
```

### 3c. Audit existing consumers for `instanceof Error` narrowing

The migration is contract-compatible (`FetchError extends Error`), so
`error instanceof Error` checks elsewhere keep working. Grep for
`fetchTypes\|fetchDocs\|fetchDocContent\|fetchTemplates` callers:

```bash
cd skills/visualisation/visualise/frontend
rg -n "fetch(Types|Docs|DocContent|Templates|TemplateDetail)\\b" src/
```

For each call site that destructures `error.message` or asserts on
substrings of the message, sanity-check that the error message
text is unchanged (it is — the migration only changes the class,
not the message string).

### Success criteria

```bash
cd skills/visualisation/visualise/frontend
npm test -- fetch
# 10 tests pass (5 helpers × 2 cases each)
```

```bash
cd skills/visualisation/visualise/frontend
npm test
# the full pre-existing suite remains green; no consumer broke
# because of the class change
```

```bash
cd skills/visualisation/visualise/frontend
npm run build
# `tsc -b` succeeds
```

---

## Step 4: Frontend — extend test router helpers to register the library doc route

### Files

- `skills/visualisation/visualise/frontend/src/components/Sidebar/test-helpers.tsx` (extend, then move) → `skills/visualisation/visualise/frontend/src/test/router-helpers.tsx` (relocated)
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.test.tsx` (update import)

### Rationale

The existing `MemoryRouter` helper at
`Sidebar/test-helpers.tsx:1-11` builds a root route whose component
renders the supplied children, but it registers no other routes —
so any `<Link to="/library/$type/$fileSlug">` rendered inside the
children will not resolve. Phase 7 introduces three components
(`TicketCard`, `KanbanColumn`, `KanbanBoard`) whose tests need to
render real `<Link>` elements that resolve to library doc URLs.

Rather than duplicate test plumbing in every kanban test file, we
relocate the helper to a shared `test/router-helpers.tsx` module
and extend it to register the library doc route, plus a small
`renderWithRouterAt(ui)` convenience wrapper.

### 4a. Write a smoke test for the helper first

Create `frontend/src/test/router-helpers.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest'
import { screen } from '@testing-library/react'
import { Link } from '@tanstack/react-router'
import { renderWithRouterAt } from './router-helpers'

describe('renderWithRouterAt', () => {
  it('renders the supplied children at /', () => {
    renderWithRouterAt(<p>Hello kanban</p>)
    expect(screen.getByText('Hello kanban')).toBeInTheDocument()
  })

  it('resolves <Link> targets at the library doc route', () => {
    renderWithRouterAt(
      <Link to="/library/$type/$fileSlug" params={{ type: 'tickets', fileSlug: '0001-x' }}>
        ticket link
      </Link>,
    )
    const link = screen.getByRole('link', { name: /ticket link/i })
    expect(link.getAttribute('href')).toBe('/library/tickets/0001-x')
  })
})
```

### 4b. Implement the helper

Create `frontend/src/test/router-helpers.tsx`:

```tsx
import React from 'react'
import {
  createRouter, createRootRoute, createRoute,
  createMemoryHistory, RouterProvider, Outlet,
} from '@tanstack/react-router'
import { render } from '@testing-library/react'

/** Build a minimal router test tree:
 *    /                          → renders the supplied children
 *    /library/$type/$fileSlug   → no-op leaf so <Link> can resolve
 *  Use this in component tests when the unit under test renders
 *  TanStack <Link> elements that target the library doc route. */
function buildTestRouter(ui: React.ReactNode, atUrl = '/') {
  const root = createRootRoute({ component: () => <Outlet /> })
  const indexRoute = createRoute({
    getParentRoute: () => root,
    path: '/',
    component: () => <>{ui}</>,
  })
  const libraryDocRoute = createRoute({
    getParentRoute: () => root,
    path: '/library/$type/$fileSlug',
    component: () => null,
  })
  const tree = root.addChildren([indexRoute, libraryDocRoute])
  return createRouter({
    routeTree: tree,
    history: createMemoryHistory({ initialEntries: [atUrl] }),
  })
}

/** Render `ui` inside a memory-history RouterProvider whose route
 *  tree includes the library doc route, so `<Link>` resolution
 *  works. Re-export for tests that need finer control over the
 *  router instance. */
export function renderWithRouterAt(ui: React.ReactNode, atUrl = '/') {
  const router = buildTestRouter(ui, atUrl)
  return render(<RouterProvider router={router} />)
}

/** Backwards-compatible wrapper used by the Sidebar test suite. */
export function MemoryRouter({ children }: { children: React.ReactNode }) {
  return <RouterProvider router={buildTestRouter(children)} />
}
```

### 4c. Migrate the Sidebar test suite to the relocated helper

Update `Sidebar.test.tsx` to import `MemoryRouter` from
`../../test/router-helpers` instead of `./test-helpers`. Then delete
`components/Sidebar/test-helpers.tsx`.

### Success criteria

```bash
cd skills/visualisation/visualise/frontend
npm test -- router-helpers Sidebar
# pre-existing Sidebar tests still pass (unchanged behaviour);
# new router-helpers tests pass
```

```bash
ls skills/visualisation/visualise/frontend/src/test/
# setup.ts, router-helpers.tsx, router-helpers.test.tsx
# (no test-helpers.tsx under Sidebar/)
```

---

## Step 5: Frontend — `STATUS_COLUMNS` and column-key types

### File: `skills/visualisation/visualise/frontend/src/api/types.ts`

### 5a. Add the column constants

Append below the existing `LIFECYCLE_PIPELINE_STEPS` block:

```ts
/** Canonical kanban statuses defined by the spec. The "Other" key is
 *  rendered as a separate swimlane and is *not* a valid persisted
 *  status — it's a UI-only catch-all for tickets whose
 *  `frontmatter.status` is missing, non-string, or outside this set. */
export type KanbanColumnKey = 'todo' | 'in-progress' | 'done'

export const OTHER_COLUMN_KEY = 'other' as const
export type KanbanGroupKey = KanbanColumnKey | typeof OTHER_COLUMN_KEY

export const STATUS_COLUMNS: ReadonlyArray<{
  key: KanbanColumnKey
  label: string
}> = [
  { key: 'todo',        label: 'Todo' },
  { key: 'in-progress', label: 'In progress' },
  { key: 'done',        label: 'Done' },
] as const

export const OTHER_COLUMN: { key: typeof OTHER_COLUMN_KEY; label: string } = {
  key: OTHER_COLUMN_KEY,
  label: 'Other',
}
```

No test for this step — it's a pure type and constant declaration.
The `STATUS_COLUMNS` shape is exercised by `KanbanColumn` and
`KanbanBoard` tests in later steps.

### Success criteria

```bash
cd skills/visualisation/visualise/frontend
npm run build
# `tsc -b` succeeds: the new types are syntactically valid and
# downstream files can import them without error
```

---

## Step 6: Frontend — `parseTicketNumber(relPath)` helper (TDD)

### Files

- `skills/visualisation/visualise/frontend/src/api/ticket.ts` (new)
- `skills/visualisation/visualise/frontend/src/api/ticket.test.ts` (new)

### 6a. Write the test first

```ts
// src/api/ticket.test.ts
import { describe, it, expect } from 'vitest'
import { parseTicketNumber } from './ticket'

describe('parseTicketNumber', () => {
  it('returns the integer parsed from a four-digit prefix', () => {
    expect(parseTicketNumber('meta/tickets/0001-foo.md')).toBe(1)
    expect(parseTicketNumber('meta/tickets/0029-bar-baz.md')).toBe(29)
  })

  it('returns the integer when the path has no directory component', () => {
    expect(parseTicketNumber('0042-bare.md')).toBe(42)
  })

  it('returns null when the leading segment is non-numeric', () => {
    expect(parseTicketNumber('meta/tickets/foo-bar.md')).toBeNull()
    expect(parseTicketNumber('meta/tickets/ADR-0001-foo.md')).toBeNull()
  })

  it('returns null when there is no leading digit run', () => {
    expect(parseTicketNumber('meta/tickets/-foo.md')).toBeNull()
    expect(parseTicketNumber('')).toBeNull()
  })

  it('returns null when the dash separator is missing', () => {
    expect(parseTicketNumber('meta/tickets/0001.md')).toBeNull()
  })

  it('parses ticket numbers with arbitrary digit count (no upper bound)', () => {
    expect(parseTicketNumber('meta/tickets/12345-foo.md')).toBe(12345)
  })
})
```

### 6b. Implement

```ts
// src/api/ticket.ts
/** Parse the leading numeric prefix from a ticket filename.
 *  Tickets are always created with a four-digit prefix by
 *  `skills/tickets/scripts/ticket-next-number.sh`, but this helper
 *  accepts any positive-length digit run followed by `-`. Returns
 *  `null` when the filename does not start with `<digits>-`. */
export function parseTicketNumber(relPath: string): number | null {
  const filename = relPath.split('/').at(-1) ?? ''
  const match = filename.match(/^(\d+)-/)
  if (!match) return null
  const n = Number.parseInt(match[1], 10)
  return Number.isFinite(n) ? n : null
}
```

### Success criteria

```bash
cd skills/visualisation/visualise/frontend
npm test -- ticket
# 6 tests pass (one per `it` block above)
```

---

## Step 7: Frontend — `groupTicketsByStatus(entries)` helper (TDD)

### Files

- `skills/visualisation/visualise/frontend/src/api/ticket.ts` (extend)
- `skills/visualisation/visualise/frontend/src/api/ticket.test.ts` (extend)

### 7a. Write the test first

Append to `ticket.test.ts`:

```ts
import { groupTicketsByStatus } from './ticket'
import { makeIndexEntry } from './test-fixtures'
import { OTHER_COLUMN_KEY } from './types'

describe('groupTicketsByStatus', () => {
  it('groups by canonical status values', () => {
    const a = makeIndexEntry({ relPath: 'meta/tickets/0001-a.md', frontmatter: { status: 'todo' } })
    const b = makeIndexEntry({ relPath: 'meta/tickets/0002-b.md', frontmatter: { status: 'in-progress' } })
    const c = makeIndexEntry({ relPath: 'meta/tickets/0003-c.md', frontmatter: { status: 'done' } })

    const groups = groupTicketsByStatus([a, b, c])
    expect(groups.get('todo')).toEqual([a])
    expect(groups.get('in-progress')).toEqual([b])
    expect(groups.get('done')).toEqual([c])
    expect(groups.get(OTHER_COLUMN_KEY) ?? []).toEqual([])
  })

  it('places exotic status values in the "other" group', () => {
    const blocked = makeIndexEntry({
      relPath: 'meta/tickets/0001-x.md',
      frontmatter: { status: 'blocked' },
    })
    const groups = groupTicketsByStatus([blocked])
    expect(groups.get(OTHER_COLUMN_KEY)).toEqual([blocked])
    expect(groups.get('todo') ?? []).toEqual([])
  })

  it('places tickets with missing status in "other"', () => {
    const noStatus = makeIndexEntry({ frontmatter: {} })
    const groups = groupTicketsByStatus([noStatus])
    expect(groups.get(OTHER_COLUMN_KEY)).toEqual([noStatus])
  })

  it('places tickets with non-string status in "other"', () => {
    const numeric = makeIndexEntry({ frontmatter: { status: 42 } })
    const groups = groupTicketsByStatus([numeric])
    expect(groups.get(OTHER_COLUMN_KEY)).toEqual([numeric])
  })

  it('places tickets with absent or malformed frontmatter in "other"', () => {
    const absent = makeIndexEntry({ frontmatterState: 'absent', frontmatter: {} })
    const malformed = makeIndexEntry({ frontmatterState: 'malformed', frontmatter: {} })
    const groups = groupTicketsByStatus([absent, malformed])
    expect(groups.get(OTHER_COLUMN_KEY)).toEqual([absent, malformed])
  })

  it('sorts each group by mtimeMs descending', () => {
    const old = makeIndexEntry({ relPath: 'meta/tickets/0001-old.md', frontmatter: { status: 'todo' }, mtimeMs: 100 })
    const mid = makeIndexEntry({ relPath: 'meta/tickets/0002-mid.md', frontmatter: { status: 'todo' }, mtimeMs: 200 })
    const newest = makeIndexEntry({ relPath: 'meta/tickets/0003-new.md', frontmatter: { status: 'todo' }, mtimeMs: 300 })
    const groups = groupTicketsByStatus([old, newest, mid])
    expect(groups.get('todo')).toEqual([newest, mid, old])
  })

  it('breaks mtime ties deterministically by relPath ascending', () => {
    // Two tickets edited in the same second must not flicker on
    // re-render. The tie-break is `relPath` ascending so the order is
    // independent of the upstream `fetchDocs` array order.
    const beta = makeIndexEntry({
      relPath: 'meta/tickets/0002-beta.md',
      frontmatter: { status: 'todo' }, mtimeMs: 500,
    })
    const alpha = makeIndexEntry({
      relPath: 'meta/tickets/0001-alpha.md',
      frontmatter: { status: 'todo' }, mtimeMs: 500,
    })
    // Pass beta-then-alpha; expect alpha first because of relPath order.
    const groups = groupTicketsByStatus([beta, alpha])
    expect(groups.get('todo')).toEqual([alpha, beta])
  })

  it('omits the "other" key entirely when no exotic tickets exist', () => {
    // Callers rely on `groups.has(OTHER_COLUMN_KEY)` (or
    // `groups.get(...)?.length > 0`) to decide whether to render the
    // Other swimlane. Pin the omission contract so a future change to
    // pre-initialise the key would be caught.
    const todo = makeIndexEntry({
      relPath: 'meta/tickets/0001-x.md',
      frontmatter: { status: 'todo' },
    })
    const groups = groupTicketsByStatus([todo])
    expect(groups.has(OTHER_COLUMN_KEY)).toBe(false)
  })

  it('returns empty arrays for known columns when no tickets match', () => {
    const groups = groupTicketsByStatus([])
    expect(groups.get('todo')).toEqual([])
    expect(groups.get('in-progress')).toEqual([])
    expect(groups.get('done')).toEqual([])
    // The "other" key is omitted on empty input — callers branch on
    // `groups.get('other')?.length` to hide the swimlane.
    expect(groups.has(OTHER_COLUMN_KEY)).toBe(false)
  })
})
```

### 7b. Implement

Append to `ticket.ts`:

```ts
import type { IndexEntry } from './types'
import {
  STATUS_COLUMNS,
  OTHER_COLUMN_KEY,
  type KanbanColumnKey,
  type KanbanGroupKey,
} from './types'

const KNOWN_KEYS: ReadonlySet<KanbanColumnKey> = new Set(
  STATUS_COLUMNS.map(c => c.key),
)

function statusGroupOf(entry: IndexEntry): KanbanGroupKey {
  if (entry.frontmatterState !== 'parsed') return OTHER_COLUMN_KEY
  const raw = entry.frontmatter['status']
  if (typeof raw !== 'string') return OTHER_COLUMN_KEY
  return KNOWN_KEYS.has(raw as KanbanColumnKey)
    ? (raw as KanbanColumnKey)
    : OTHER_COLUMN_KEY
}

/** Group tickets into kanban columns by `frontmatter.status`,
 *  catching all non-canonical / missing / non-string / malformed
 *  cases in the "other" group. The three known columns are always
 *  present in the returned map (with empty arrays when no tickets
 *  match); the "other" key is added lazily on first hit so callers
 *  can branch on `groups.has('other')` to decide swimlane visibility.
 *  Within each group, entries are sorted by `mtimeMs` descending and
 *  ties are broken by `relPath` ascending so the order is
 *  deterministic across re-renders. */
export function groupTicketsByStatus(
  entries: IndexEntry[],
): Map<KanbanGroupKey, IndexEntry[]> {
  const groups = new Map<KanbanGroupKey, IndexEntry[]>()
  for (const c of STATUS_COLUMNS) groups.set(c.key, [])
  for (const entry of entries) {
    const key = statusGroupOf(entry)
    let list = groups.get(key)
    if (!list) {
      list = []
      groups.set(key, list)
    }
    list.push(entry)
  }
  for (const list of groups.values()) {
    list.sort((a, b) => {
      if (b.mtimeMs !== a.mtimeMs) return b.mtimeMs - a.mtimeMs
      return a.relPath.localeCompare(b.relPath)
    })
  }
  return groups
}
```

### Success criteria

```bash
cd skills/visualisation/visualise/frontend
npm test -- ticket
# 15 tests pass total (6 from Step 6 + 9 from this step)
```

---

## Step 8: Frontend — `TicketCard` component (TDD)

### Files

- `skills/visualisation/visualise/frontend/src/routes/kanban/TicketCard.tsx` (new)
- `skills/visualisation/visualise/frontend/src/routes/kanban/TicketCard.module.css` (new)
- `skills/visualisation/visualise/frontend/src/routes/kanban/TicketCard.test.tsx` (new)

The card lives alongside `KanbanColumn` and `KanbanBoard` because it is
kanban-specific (Phase 8 will repurpose `useSortable` for drag
mutation). This mirrors `LifecycleClusterView`'s inline `EntryCard`
cohesion rather than the speculative "shared `components/`" placement
that would invite cross-route reuse not actually planned.

### 8a. Write the test first

```tsx
// TicketCard.test.tsx
import { describe, it, expect } from 'vitest'
import { screen } from '@testing-library/react'
import { DndContext } from '@dnd-kit/core'
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { TicketCard } from './TicketCard'
import { makeIndexEntry } from '../../api/test-fixtures'
import { renderWithRouterAt } from '../../test/router-helpers'

const FROZEN_NOW = 1_700_000_000_000 // arbitrary deterministic instant

function renderCard(entry: ReturnType<typeof makeIndexEntry>, now = FROZEN_NOW) {
  return renderWithRouterAt(
    <DndContext>
      <SortableContext items={[entry.relPath]} strategy={verticalListSortingStrategy}>
        <TicketCard entry={entry} now={now} />
      </SortableContext>
    </DndContext>,
  )
}

describe('TicketCard', () => {
  it('renders the ticket number with four-digit zero-padding', () => {
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0001-three-layer.md',
      title: 'Three-layer review system architecture',
      frontmatter: { type: 'adr-creation-task', status: 'done' },
      // Exactly 90s before FROZEN_NOW so formatMtime returns "1m ago"
      // deterministically (well inside the 60s..3600s bucket).
      mtimeMs: FROZEN_NOW - 90_000,
    })
    renderCard(entry)
    // Pin the four-digit padding: removing `padStart(4, '0')` would
    // render `#1` and fail this assertion.
    expect(screen.getByText('#0001')).toBeInTheDocument()
    expect(screen.getByText('Three-layer review system architecture')).toBeInTheDocument()
    expect(screen.getByText('adr-creation-task')).toBeInTheDocument()
    expect(screen.getByText('1m ago')).toBeInTheDocument()
  })

  it('renders larger ticket numbers verbatim (no truncation)', () => {
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0029-template-management.md',
      title: 'Template management',
      frontmatter: { type: 'adr-creation-task', status: 'done' },
      mtimeMs: FROZEN_NOW - 90_000,
    })
    renderCard(entry)
    expect(screen.getByText('#0029')).toBeInTheDocument()
  })

  it('links to the library detail page using the canonical typed-route form', () => {
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0001-three-layer-review-system-architecture.md',
      title: 'Three-layer review system architecture',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderCard(entry)
    const link = screen.getByRole('link', {
      name: /three-layer review system architecture/i,
    })
    expect(link.getAttribute('href')).toBe(
      '/library/tickets/0001-three-layer-review-system-architecture',
    )
  })

  it('renders gracefully when frontmatter.type is missing', () => {
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0042-no-type.md',
      title: 'No type',
      frontmatter: { status: 'todo' },
    })
    renderCard(entry)
    expect(screen.getByText('#0042')).toBeInTheDocument()
    expect(screen.getByText('No type')).toBeInTheDocument()
    expect(screen.queryByText(/undefined/)).toBeNull()
  })

  it('falls back to the file slug when the relPath has no numeric prefix', () => {
    // Defensive guard against tickets created outside `ticket-next-number.sh`
    // (which guarantees an `NNNN-` prefix). Cards still need a visible
    // identifier in this case so users can refer to the ticket.
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/foo-without-number.md',
      title: 'No number',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderCard(entry)
    expect(screen.getByText('No number')).toBeInTheDocument()
    // No `#NNNN` chip rendered…
    expect(screen.queryByText(/^#\d/)).toBeNull()
    // …but a slug fallback chip is rendered so the card has an identifier.
    expect(screen.getByText('foo-without-number')).toBeInTheDocument()
  })

  it('does not announce a misleading "sortable" role-description while disabled', () => {
    // Phase 7 invariant: drag is a no-op. The sortable wiring is
    // pre-mounted for Phase 8 reuse (no provider re-threading), but
    // ARIA must reflect actual capability in Phase 7. The card must
    // not announce role-description "sortable" while drag is disabled.
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0001-a.md',
      title: 'Some ticket',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderCard(entry)
    const link = screen.getByRole('link', { name: /some ticket/i })
    expect(link.getAttribute('aria-roledescription')).toBeNull()
  })

  it('does not respond to drag interaction while disabled', () => {
    // Behavioural pin on `disabled: true`: a future change that flips
    // the flag to `false` would change `transform` after a pointer
    // drag. We assert the inline `style.transform` stays empty after
    // a synthetic pointerDown + pointerMove on the sortable node.
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0001-drag.md',
      title: 'Draggy',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderCard(entry)
    const link = screen.getByRole('link', { name: /draggy/i })
    const before = link.getAttribute('style') ?? ''
    // Fire pointer events that would normally start a drag.
    link.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
    link.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientX: 50, clientY: 50 }))
    const after = link.getAttribute('style') ?? ''
    // No transform mutation occurred — the sortable did not engage.
    expect(after).toBe(before)
  })
})
```

### 8b. Implement

```tsx
// TicketCard.tsx
import { Link } from '@tanstack/react-router'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { formatMtime } from '../../api/format'
import { fileSlugFromRelPath } from '../../api/path-utils'
import { parseTicketNumber } from '../../api/ticket'
import type { IndexEntry } from '../../api/types'
import styles from './TicketCard.module.css'

export interface TicketCardProps {
  entry: IndexEntry
  /** Injected for deterministic tests. Production callers omit it
   *  and `formatMtime` defaults to `Date.now()`. */
  now?: number
}

const PHASE_7_DISABLED = true

export function TicketCard({ entry, now }: TicketCardProps) {
  // The sortable wiring is pre-mounted so Phase 8 can flip
  // `PHASE_7_DISABLED` to false (and pass real onDrag handlers in the
  // parent DndContext) without re-threading providers. While disabled,
  // we deliberately strip dnd-kit's `aria-roledescription="sortable"`
  // so screen readers don't announce a drag affordance that does not
  // exist — keeping ARIA truthful in Phase 7.
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({
    id: entry.relPath,
    disabled: PHASE_7_DISABLED,
  })

  const { 'aria-roledescription': _ariaRoleDescription, ...phase7Attributes } = attributes

  const number = parseTicketNumber(entry.relPath)
  const fmType = entry.frontmatter['type']
  const typeLabel = typeof fmType === 'string' && fmType.length > 0 ? fmType : null
  const fileSlug = fileSlugFromRelPath(entry.relPath)
  // When the filename has no numeric prefix (extremely rare; defensive
  // against external authoring tools), fall back to the slug so the
  // card still has a visible identifier.
  const idChip = number !== null
    ? `#${String(number).padStart(4, '0')}`
    : fileSlug

  return (
    <li className={styles.card}>
      <Link
        ref={setNodeRef}
        to="/library/$type/$fileSlug"
        params={{ type: 'tickets', fileSlug }}
        className={styles.cardLink}
        style={{
          transform: CSS.Transform.toString(transform),
          transition,
        }}
        {...phase7Attributes}
        {...listeners}
      >
        <div className={styles.cardHeader}>
          <span
            className={number !== null ? styles.cardNumber : styles.cardSlug}
          >
            {idChip}
          </span>
          <span className={styles.cardMtime}>{formatMtime(entry.mtimeMs, now)}</span>
        </div>
        <p className={styles.cardTitle}>{entry.title}</p>
        {typeLabel !== null && <p className={styles.cardType}>{typeLabel}</p>}
      </Link>
    </li>
  )
}
```

Notes on the dnd-kit wiring:

- `setNodeRef`, `attributes`, `listeners`, `transform`, and `transition`
  are all attached to the `<Link>` rather than the wrapper `<li>`. This
  produces a single focus stop (the link) rather than the
  link-inside-button double tab stop that emerges if `attributes`
  (which include `tabIndex={0}` and `role="button"`) land on the `<li>`.
- `aria-roledescription` is stripped from `phase7Attributes` so AT
  users are not told the card is "sortable" when drag is disabled.
  Phase 8 should re-add it (drop the destructure) when flipping
  `PHASE_7_DISABLED` to `false`.

CSS module mirrors the literal palette already used by
`LifecycleClusterView.module.css`:

```css
/* TicketCard.module.css */
.card {
  list-style: none;
  margin: 0 0 0.5rem 0;
  border: 1px solid #e5e7eb;
  border-radius: 0.25rem;
  background: #ffffff;
}
.cardLink {
  display: block;
  padding: 0.5rem 0.75rem;
  color: inherit;
  text-decoration: none;
}
.cardLink:hover { background: #f9fafb; }
.cardLink:hover .cardTitle { color: #1d4ed8; }
.cardHeader {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 0.5rem;
  font-size: 0.85rem;
}
.cardNumber { font-family: monospace; color: #4b5563; }
.cardSlug   { font-family: monospace; color: #9ca3af; font-size: 0.75rem; }
.cardMtime  { color: #6b7280; }
.cardTitle  { margin: 0.25rem 0; font-weight: 600; color: #111827; }
.cardType   { margin: 0; font-size: 0.85rem; color: #6b7280; }
```

### Success criteria

```bash
cd skills/visualisation/visualise/frontend
npm test -- TicketCard
# 7 tests pass (one per `it` block above)
```

---

## Step 9: Frontend — `KanbanColumn` component (TDD)

### Files

- `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanColumn.tsx` (new)
- `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanColumn.module.css` (new)
- `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanColumn.test.tsx` (new)

### 9a. Write the test first

```tsx
// KanbanColumn.test.tsx
import { describe, it, expect } from 'vitest'
import { screen, within } from '@testing-library/react'
import { DndContext } from '@dnd-kit/core'
import { KanbanColumn } from './KanbanColumn'
import { makeIndexEntry } from '../../api/test-fixtures'
import { renderWithRouterAt } from '../../test/router-helpers'

function renderColumn(ui: React.ReactNode) {
  return renderWithRouterAt(<DndContext>{ui}</DndContext>)
}

describe('KanbanColumn', () => {
  it('renders the column heading and one card per entry', () => {
    const a = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0001-a.md', title: 'Alpha',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    const b = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0002-b.md', title: 'Beta',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderColumn(<KanbanColumn columnKey="todo" label="Todo" entries={[a, b]} />)
    const region = screen.getByRole('region', { name: /todo/i })
    expect(within(region).getByRole('heading', { name: /todo/i, level: 2 })).toBeInTheDocument()
    expect(within(region).getByText('Alpha')).toBeInTheDocument()
    expect(within(region).getByText('Beta')).toBeInTheDocument()
  })

  it('renders an empty-state message when entries is empty, marked aria-hidden', () => {
    renderColumn(<KanbanColumn columnKey="in-progress" label="In progress" entries={[]} />)
    const region = screen.getByRole('region', { name: /in progress/i })
    const empty = within(region).getByText(/no tickets/i)
    expect(empty).toBeInTheDocument()
    // The empty-state paragraph is marked aria-hidden so AT users
    // hear the count chip's "0 tickets" once, not twice.
    expect(empty.getAttribute('aria-hidden')).toBe('true')
  })

  it('exposes the count via aria-label without duplicating the column name', () => {
    const a = makeIndexEntry({ type: 'tickets', relPath: 'meta/tickets/0001-a.md', frontmatter: { status: 'done' } })
    const b = makeIndexEntry({ type: 'tickets', relPath: 'meta/tickets/0002-b.md', frontmatter: { status: 'done' } })
    renderColumn(<KanbanColumn columnKey="done" label="Done" entries={[a, b]} />)
    // Region label provides the column name; the badge's aria-label
    // adds only the count so AT users don't hear "Done … 2 tickets in
    // Done" (duplicate). Pluralisation handles 1 vs 2+.
    expect(screen.getByLabelText(/^2 tickets$/i)).toBeInTheDocument()
  })

  it('uses singular wording for one ticket and plural for zero or many', () => {
    const a = makeIndexEntry({ type: 'tickets', relPath: 'meta/tickets/0001-a.md', frontmatter: { status: 'todo' } })
    const { unmount } = renderColumn(<KanbanColumn columnKey="todo" label="Todo" entries={[a]} />)
    expect(screen.getByLabelText(/^1 ticket$/i)).toBeInTheDocument()
    unmount()
    renderColumn(<KanbanColumn columnKey="todo" label="Todo" entries={[]} />)
    expect(screen.getByLabelText(/^0 tickets$/i)).toBeInTheDocument()
  })

  it('renders the "Other" column variant with a distinct heading and explanation', () => {
    const x = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0007-x.md', title: 'Exotic',
      frontmatter: { type: 'adr-creation-task', status: 'blocked' },
    })
    renderColumn(<KanbanColumn columnKey="other" label="Other" entries={[x]} description="Tickets whose status is missing or not one of: todo, in-progress, done." />)
    expect(screen.getByRole('heading', { name: /other/i, level: 2 })).toBeInTheDocument()
    expect(screen.getByText('Exotic')).toBeInTheDocument()
    // The Other column carries an explanation so authors understand
    // why their ticket landed there.
    expect(screen.getByText(/missing or not one of/i)).toBeInTheDocument()
  })
})
```

### 9b. Implement

```tsx
// KanbanColumn.tsx
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { TicketCard } from './TicketCard'
import type { IndexEntry, KanbanGroupKey } from '../../api/types'
import styles from './KanbanColumn.module.css'

export interface KanbanColumnProps {
  columnKey: KanbanGroupKey
  label: string
  entries: IndexEntry[]
  /** Optional explanation rendered under the heading. The Other
   *  swimlane uses this to tell authors why their ticket landed there. */
  description?: string
}

export function KanbanColumn({ columnKey, label, entries, description }: KanbanColumnProps) {
  const ids = entries.map(e => e.relPath)
  const count = entries.length
  const headingId = `kanban-col-${columnKey}-heading`
  const ticketWord = count === 1 ? 'ticket' : 'tickets'

  return (
    <section
      className={styles.column}
      aria-labelledby={headingId}
      data-column={columnKey}
    >
      <header className={styles.columnHeader}>
        <h2 id={headingId} className={styles.columnHeading}>
          {label}
        </h2>
        <span className={styles.columnCount} aria-label={`${count} ${ticketWord}`}>
          {count}
        </span>
      </header>
      {description && <p className={styles.columnDescription}>{description}</p>}
      <SortableContext items={ids} strategy={verticalListSortingStrategy}>
        {entries.length === 0 ? (
          <p className={styles.empty} aria-hidden="true">No tickets</p>
        ) : (
          <ul className={styles.cardList}>
            {entries.map(entry => (
              <TicketCard key={entry.relPath} entry={entry} />
            ))}
          </ul>
        )}
      </SortableContext>
    </section>
  )
}
```

```css
/* KanbanColumn.module.css */
.column {
  display: flex;
  flex-direction: column;
  min-width: 16rem;
  flex: 1 1 16rem;
  background: #f3f4f6;
  border-radius: 0.25rem;
  padding: 0.75rem;
}
.columnHeader {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 0.5rem;
}
.columnHeading { margin: 0; font-size: 1rem; color: #111827; }
.columnCount {
  background: #ffffff;
  border-radius: 999px;
  padding: 0 0.5rem;
  font-size: 0.85rem;
  color: #6b7280;
}
.columnDescription {
  margin: 0 0 0.5rem 0;
  font-size: 0.85rem;
  color: #6b7280;
}
.cardList {
  list-style: none;
  margin: 0;
  padding: 0;
}
.empty {
  margin: 0.5rem 0;
  color: #9ca3af;
  font-style: italic;
}
```

### Success criteria

```bash
cd skills/visualisation/visualise/frontend
npm test -- KanbanColumn
# 5 tests pass
```

---

## Step 10: Frontend — `KanbanBoard` view (TDD)

### Files

- `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.tsx` (new)
- `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.module.css` (new)
- `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.test.tsx` (new)

### 10a. Write the test first

```tsx
// KanbanBoard.test.tsx
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryHistory, createRouter, RouterProvider } from '@tanstack/react-router'
import { routeTree } from '../../router'
import * as fetchModule from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { makeIndexEntry } from '../../api/test-fixtures'

function renderKanbanAt(qc: QueryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })) {
  const router = createRouter({
    routeTree,
    history: createMemoryHistory({ initialEntries: ['/kanban'] }),
  })
  return {
    ...render(
      <QueryClientProvider client={qc}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    ),
    queryClient: qc,
  }
}

describe('KanbanBoard', () => {
  beforeEach(() => {
    // Stub every root-level fetch RootLayout / Sidebar may issue —
    // mirrors `router.test.tsx`'s setup so KanbanBoard tests don't hit
    // the real network through unrelated views.
    vi.spyOn(fetchModule, 'fetchTypes').mockResolvedValue([])
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: [] })
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue({} as never)
  })

  it('renders the page-level heading at the top of the board', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    renderKanbanAt()
    expect(await screen.findByRole('heading', { level: 1, name: /^kanban$/i })).toBeInTheDocument()
  })

  it('shows a loading state while the tickets list is pending', () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation(() => new Promise(() => {}))
    renderKanbanAt()
    const loading = screen.getByText(/loading/i)
    expect(loading).toBeInTheDocument()
    // Loading copy is wrapped in role="status" so AT users hear the
    // transition.
    expect(loading.closest('[role="status"]')).not.toBeNull()
  })

  it('renders three labelled columns when there are no tickets, no Other swimlane', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    renderKanbanAt()
    expect(await screen.findByRole('region', { name: /todo/i })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: /in progress/i })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: /done/i })).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: /other/i })).toBeNull()
  })

  it('places tickets in the column matching their frontmatter.status', async () => {
    const todo = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0001-todo.md', title: 'Todo ticket',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    const inProgress = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0002-wip.md', title: 'WIP ticket',
      frontmatter: { type: 'adr-creation-task', status: 'in-progress' },
    })
    const done = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0003-done.md', title: 'Done ticket',
      frontmatter: { type: 'adr-creation-task', status: 'done' },
    })
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([todo, inProgress, done])

    renderKanbanAt()
    const todoCol = await screen.findByRole('region', { name: /todo/i })
    expect(within(todoCol).getByText('Todo ticket')).toBeInTheDocument()
    expect(within(screen.getByRole('region', { name: /in progress/i })).getByText('WIP ticket')).toBeInTheDocument()
    expect(within(screen.getByRole('region', { name: /done/i })).getByText('Done ticket')).toBeInTheDocument()
  })

  it('renders the Other swimlane with non-canonical statuses', async () => {
    const blocked = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0007-blocked.md', title: 'Blocked ticket',
      frontmatter: { type: 'adr-creation-task', status: 'blocked' },
    })
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([blocked])
    renderKanbanAt()
    const other = await screen.findByRole('region', { name: /other/i })
    expect(within(other).getByText('Blocked ticket')).toBeInTheDocument()
  })

  it('sorts cards within a column by mtimeMs descending', async () => {
    const old = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0001-old.md', title: 'Old',
      frontmatter: { type: 'adr-creation-task', status: 'todo' }, mtimeMs: 100,
    })
    const newest = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0002-new.md', title: 'Newest',
      frontmatter: { type: 'adr-creation-task', status: 'todo' }, mtimeMs: 300,
    })
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([old, newest])
    renderKanbanAt()
    const todoCol = await screen.findByRole('region', { name: /todo/i })
    const titles = within(todoCol).getAllByRole('link').map(l => l.textContent)
    // First link is the newer ticket; "Old" comes after.
    const newestIdx = titles.findIndex(t => t?.includes('Newest'))
    const oldIdx = titles.findIndex(t => t?.includes('Old'))
    expect(newestIdx).toBeGreaterThanOrEqual(0)
    expect(oldIdx).toBeGreaterThan(newestIdx)
  })

  it('renders a typed-aware error message on FetchError(5xx)', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockRejectedValue(
      new fetchModule.FetchError(500, 'GET /api/docs?type=tickets: 500'),
    )
    renderKanbanAt()
    const alert = await screen.findByRole('alert')
    expect(alert.textContent).toMatch(/server returned an error/i)
    // Internal status / URL must not leak into user-visible copy.
    expect(alert.textContent).not.toMatch(/500/)
    expect(alert.textContent).not.toMatch(/\/api\//)
  })

  it('renders a generic error message on non-FetchError rejection', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockRejectedValue(new Error('boom'))
    renderKanbanAt()
    const alert = await screen.findByRole('alert')
    // Tightened to match only the generic copy — must not also match
    // the 5xx-branch copy ("server returned an error").
    expect(alert.textContent).toMatch(/something went wrong loading the tickets/i)
    expect(alert.textContent).not.toMatch(/server returned an error/i)
    expect(alert.textContent).not.toMatch(/boom/)
  })

  it('renders a Retry button inside the error alert that invalidates the query', async () => {
    const fetchSpy = vi.spyOn(fetchModule, 'fetchDocs')
      .mockRejectedValueOnce(new fetchModule.FetchError(500, 'fail'))
      .mockResolvedValue([])
    const { queryClient } = renderKanbanAt()
    const alert = await screen.findByRole('alert')
    const retry = within(alert).getByRole('button', { name: /retry|try again/i })
    fireEvent.click(retry)
    // After retry, the alert should disappear and the columns appear.
    expect(await screen.findByRole('region', { name: /todo/i })).toBeInTheDocument()
    expect(fetchSpy).toHaveBeenCalledTimes(2)
    expect(queryClient.getQueryState(queryKeys.docs('tickets'))?.status).toBe('success')
  })

  it('links cards to their library detail pages via the canonical typed-route form', async () => {
    const ticket = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0029-template-management-subcommand-surface.md',
      title: 'Template management',
      frontmatter: { type: 'adr-creation-task', status: 'done' },
    })
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([ticket])
    renderKanbanAt()
    const link = await screen.findByRole('link', { name: /template management/i })
    expect(link.getAttribute('href')).toBe(
      '/library/tickets/0029-template-management-subcommand-surface',
    )
  })

  it('moves a card between columns when the tickets query is invalidated (SSE-driven update)', async () => {
    // Pin the phase's user-facing promise: external `status:` edits land
    // within ~250ms via SSE invalidating `queryKeys.docs("tickets")`.
    // We don't fire an SSE event directly — we invalidate the query
    // and assert KanbanBoard re-reads from the same key.
    const before = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0001-x.md', title: 'Movable',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    const after = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0001-x.md', title: 'Movable',
      frontmatter: { type: 'adr-creation-task', status: 'done' },
    })
    const fetchSpy = vi.spyOn(fetchModule, 'fetchDocs')
      .mockResolvedValueOnce([before])
      .mockResolvedValueOnce([after])

    const { queryClient } = renderKanbanAt()
    const todoCol = await screen.findByRole('region', { name: /todo/i })
    expect(within(todoCol).getByText('Movable')).toBeInTheDocument()

    await act(async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.docs('tickets') })
    })

    const doneCol = await screen.findByRole('region', { name: /done/i })
    expect(within(doneCol).getByText('Movable')).toBeInTheDocument()
    expect(within(todoCol).queryByText('Movable')).toBeNull()
    expect(fetchSpy).toHaveBeenCalledTimes(2)
  })
})
```

### 10b. Implement

```tsx
// KanbanBoard.tsx
import { useMemo } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  DndContext, PointerSensor, KeyboardSensor,
  useSensor, useSensors, closestCorners,
} from '@dnd-kit/core'
import { sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { fetchDocs, FetchError } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { groupTicketsByStatus } from '../../api/ticket'
import {
  STATUS_COLUMNS, OTHER_COLUMN, OTHER_COLUMN_KEY,
} from '../../api/types'
import { KanbanColumn } from './KanbanColumn'
import styles from './KanbanBoard.module.css'

const OTHER_DESCRIPTION =
  'Tickets whose status is missing or not one of: todo, in-progress, done.'

function errorMessageFor(error: unknown): string {
  if (error instanceof FetchError && error.status >= 500) {
    return 'The visualiser server returned an error. Try again in a moment.'
  }
  return 'Something went wrong loading the tickets.'
}

export function KanbanBoard() {
  const queryClient = useQueryClient()

  // Sensors are pre-mounted so Phase 8 can flip TicketCard's
  // PHASE_7_DISABLED to false without re-threading the DndContext.
  // While every sortable is disabled in Phase 7, the sensors attach
  // listeners but never engage.
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )

  const { data: entries = [], isPending, isError, error } = useQuery({
    queryKey: queryKeys.docs('tickets'),
    queryFn: () => fetchDocs('tickets'),
  })

  const groups = useMemo(() => groupTicketsByStatus(entries), [entries])
  const otherEntries = groups.get(OTHER_COLUMN_KEY) ?? []

  if (isPending) {
    return (
      <div className={styles.board}>
        <h1 className={styles.title}>Kanban</h1>
        <p role="status" className={styles.status}>Loading…</p>
      </div>
    )
  }
  if (isError) {
    return (
      <div className={styles.board}>
        <h1 className={styles.title}>Kanban</h1>
        <div role="alert" className={styles.alert}>
          <p className={styles.alertMessage}>{errorMessageFor(error)}</p>
          <button
            type="button"
            className={styles.retry}
            onClick={() => {
              queryClient.invalidateQueries({ queryKey: queryKeys.docs('tickets') })
            }}
          >
            Retry
          </button>
        </div>
      </div>
    )
  }

  return (
    // Drag handlers are deliberately no-ops in Phase 7. Phase 8 wires
    // them to the PATCH mutation; the structural providers stay
    // identical — only TicketCard's PHASE_7_DISABLED flag flips.
    <DndContext
      sensors={sensors}
      collisionDetection={closestCorners}
      onDragStart={() => {}}
      onDragOver={() => {}}
      onDragEnd={() => {}}
    >
      <div className={styles.board}>
        <h1 className={styles.title}>Kanban</h1>
        <div className={styles.columns}>
          {STATUS_COLUMNS.map(col => (
            <KanbanColumn
              key={col.key}
              columnKey={col.key}
              label={col.label}
              entries={groups.get(col.key) ?? []}
            />
          ))}
        </div>
        {otherEntries.length > 0 && (
          <div className={styles.otherSwimlane}>
            <KanbanColumn
              columnKey={OTHER_COLUMN.key}
              label={OTHER_COLUMN.label}
              entries={otherEntries}
              description={OTHER_DESCRIPTION}
            />
          </div>
        )}
      </div>
    </DndContext>
  )
}
```

```css
/* KanbanBoard.module.css */
.board {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem;
}
.title {
  margin: 0;
  font-size: 1.5rem;
  color: #111827;
}
.columns {
  display: flex;
  gap: 1rem;
  align-items: flex-start;
}
.otherSwimlane {
  border-top: 1px dashed #e5e7eb;
  padding-top: 1rem;
}
.status {
  margin: 0;
  color: #6b7280;
  font-style: italic;
}
.alert {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.75rem 1rem;
  border: 1px solid #fecaca;
  background: #fef2f2;
  border-radius: 0.25rem;
  color: #991b1b;
}
.alertMessage { margin: 0; flex: 1; }
.retry {
  background: #ffffff;
  border: 1px solid #fecaca;
  border-radius: 0.25rem;
  padding: 0.25rem 0.75rem;
  cursor: pointer;
  color: #991b1b;
}
.retry:hover { background: #fee2e2; }
```

### Success criteria

```bash
cd skills/visualisation/visualise/frontend
npm test -- KanbanBoard
# 9 tests pass
```

---

## Step 11: Frontend — router wiring and `KanbanStub` removal

### Files

- `skills/visualisation/visualise/frontend/src/router.ts`
- `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanStub.tsx` (delete)
- `skills/visualisation/visualise/frontend/src/router.test.tsx` (extend)

### 11a. Update the router test first

Append to `router.test.tsx` (using the existing `renderAt` /
`waitForPath` helpers):

```tsx
it('routes /kanban to the kanban board with three columns', async () => {
  vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
  const router = renderAt('/kanban')
  await waitForPath(router, '/kanban')
  // The board renders three labelled columns immediately on mount.
  expect(await screen.findByRole('region', { name: /todo/i })).toBeInTheDocument()
  expect(screen.getByRole('region', { name: /in progress/i })).toBeInTheDocument()
  expect(screen.getByRole('region', { name: /done/i })).toBeInTheDocument()
  // No Other swimlane when there are no tickets.
  expect(screen.queryByRole('region', { name: /other/i })).toBeNull()
})

it('does not render the legacy "coming in Phase 7" stub copy at /kanban', async () => {
  vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
  const router = renderAt('/kanban')
  await waitForPath(router, '/kanban')
  expect(screen.queryByText(/coming in phase 7/i)).toBeNull()
})
```

### 11b. Replace the stub in `router.ts`

```ts
// at top of router.ts, replace:
import { KanbanStub } from './routes/kanban/KanbanStub'
// with:
import { KanbanBoard } from './routes/kanban/KanbanBoard'

// and at line 98–102, replace `KanbanStub` with `KanbanBoard`:
const kanbanRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/kanban',
  component: KanbanBoard,
})
```

### 11c. Delete the stub file

```bash
git rm skills/visualisation/visualise/frontend/src/routes/kanban/KanbanStub.tsx
# (or `jj` equivalent: simply delete the file; `jj status` picks up
# the removal automatically)
```

The `KanbanStub` had no companion CSS module or test file, so no
sibling cleanup is required.

### Success criteria

```bash
cd skills/visualisation/visualise/frontend
npm test -- router
# pre-existing router tests pass; two new tests pass
```

```bash
cd skills/visualisation/visualise/frontend
npm run build
# `tsc -b` passes (no dangling reference to KanbanStub); vite build
# completes
```

```bash
ls skills/visualisation/visualise/frontend/src/routes/kanban/
# KanbanBoard.tsx, KanbanBoard.module.css, KanbanBoard.test.tsx
# KanbanColumn.tsx, KanbanColumn.module.css, KanbanColumn.test.tsx
# TicketCard.tsx, TicketCard.module.css, TicketCard.test.tsx
# (no KanbanStub.tsx)
```

---

## Step 12: Append a `CHANGELOG.md` entry

### File

- `CHANGELOG.md`

Under `## Unreleased`, add an `### Added` (creating it if absent)
bullet describing the user-visible feature. Keep tone and detail
level consistent with existing entries (declarative, what changed
from a user's perspective, not implementation detail).

```markdown
### Added

- **Visualiser kanban view** — `/kanban` now renders a read-only
  kanban board with `Todo`, `In progress`, and `Done` columns
  populated from ticket frontmatter. Tickets with non-canonical or
  missing statuses surface in an `Other` swimlane. Cards show the
  ticket number, title, type, and last-modified time, and link
  through to the existing library detail page. External edits to a
  ticket's `status:` field land in the UI within ~250 ms via the
  existing SSE-driven query invalidation. Drag-and-drop is wired
  for Phase 8 but disabled in this release. Adds `@dnd-kit/core`,
  `@dnd-kit/sortable`, and `@dnd-kit/utilities` as runtime
  dependencies. *Behind no flag — visible to all users on first
  navigation to `/kanban`.*
```

If the `## Unreleased` section already has an `### Added` heading,
append the bullet there rather than creating a duplicate heading.

### Success criteria

- `## Unreleased` section in `CHANGELOG.md` contains the new bullet.
- No other section is reordered or modified.

```bash
ls skills/visualisation/visualise/frontend/src/routes/kanban/
# KanbanBoard.tsx, KanbanBoard.module.css, KanbanBoard.test.tsx,
# KanbanColumn.tsx, KanbanColumn.module.css, KanbanColumn.test.tsx
# (no KanbanStub.tsx)
```

---

## Full success criteria

### Automated verification

- [ ] `mise run test:unit:visualiser` passes (cargo `--no-default-features --features dev-frontend` and default-features `--lib`).
- [ ] `mise run test:integration:visualiser` passes (cargo `--tests --no-default-features --features dev-frontend` plus shell suites).
- [ ] `mise run test:unit:frontend` passes (Vitest).
- [ ] `mise run test` passes end-to-end.
- [ ] `cd skills/visualisation/visualise/frontend && npm run build` exits 0 — covers TypeScript typecheck (`tsc -b`) + vite build.
- [ ] `cd skills/visualisation/visualise/server && cargo fmt -- --check` exits 0.
- [ ] `cd skills/visualisation/visualise/server && cargo clippy --all-targets -- -D warnings` exits 0.

Specific suites with expected counts (so a future plan author can verify the delta):

- [ ] `cargo test --tests --no-default-features --features dev-frontend` — pre-existing integration tests + 1 new (`docs_list_for_tickets_carries_frontmatter_status`).
- [ ] `npm test -- fetch` — 10 new tests (5 helpers × 2 cases each).
- [ ] `npm test -- router-helpers` — 2 new tests.
- [ ] `npm test -- ticket` — 15 tests (6 from Step 6 + 9 from Step 7).
- [ ] `npm test -- TicketCard` — 7 tests.
- [ ] `npm test -- KanbanColumn` — 5 tests.
- [ ] `npm test -- KanbanBoard` — 9 tests.
- [ ] `npm test -- router` — pre-existing N + 2 new tests.
- [ ] `npm test -- Sidebar` — pre-existing tests pass after `MemoryRouter` import path change.

### Manual verification

Run a dev server (`cargo run --features dev-frontend -- <config.json>` while
`vite build --watch` produces fresh `dist/` output) and:

- [ ] `http://127.0.0.1:<port>/kanban` renders with a page-level `Kanban` heading at the top, then three labelled columns: `Todo`, `In progress`, `Done`.
- [ ] All 29 real tickets are distributed between `Todo` and `Done` only; the `In progress` column appears empty; the `Other` swimlane is **not** rendered (no real ticket has a non-canonical status).
- [ ] Each card shows a four-digit ticket number prefixed by `#` (or, defensively, the file slug if the filename has no numeric prefix), the ticket title, the type chip (`adr-creation-task`), and a relative last-modified label.
- [ ] Clicking a card navigates to `/library/tickets/<NNNN-slug>` and renders the existing library detail view.
- [ ] In a separate editor, change one ticket's `status:` from `done` to `in-progress` and save — the card moves from `Done` to `In progress` within ~1 second without a page reload.
- [ ] Change the same ticket's `status:` to `blocked`. The `Other` swimlane appears below the three columns, carries the explanation "Tickets whose status is missing or not one of: todo, in-progress, done.", and contains exactly that ticket. Reverting to `todo` makes the swimlane disappear again.
- [ ] Stop the visualiser server and refresh `/kanban`; an error alert appears with the message "The visualiser server returned an error. Try again in a moment." and a `Retry` button. Restart the server, click `Retry`, and the columns appear without a full page reload.
- [ ] Tab through the page from the sidebar; cards receive a visible focus ring; each card is exactly one tab stop (no double tab); tab order is left-to-right within a column, top-to-bottom across columns.
- [ ] Pressing Enter on a focused card navigates to the same library detail page.
- [ ] With a screen reader (e.g. VoiceOver), navigating to a card announces its title (and ticket number) as a "link", not as "sortable button".
- [ ] Sidebar `Kanban` entry stays highlighted while on `/kanban`.

---

## Implementation sequence

Tick off as you go. Stop after each step and verify the named tests
pass before proceeding.

- [ ] **Step 1.** Add ticket fixtures to `server/tests/common/mod.rs`; run the audit grep for count-/set-based assertions in `tests/`.
- [ ] **Step 1.** Append `docs_list_for_tickets_carries_frontmatter_status` to `server/tests/api_docs.rs` (asserts `frontmatter.status` AND `frontmatterState`).
- [ ] **Step 1.** Run `cargo test --tests --no-default-features --features dev-frontend`; confirm new test passes and pre-existing tests still pass (or fix the assertions the audit flagged).
- [ ] **Step 2.** From `frontend/`, run `npm install --save-exact @dnd-kit/core@^6.3.1 @dnd-kit/sortable@^10.0.0 @dnd-kit/utilities@^3.2.2`.
- [ ] **Step 2.** Add `MockResizeObserver` and `Element.prototype.scrollIntoView` stub to `src/test/setup.ts`.
- [ ] **Step 2.** Run `npm test`; confirm pre-existing test count unchanged.
- [ ] **Step 3.** Create `src/api/fetch.test.ts` with the FetchError-contract tests for all 5 helpers; confirm 10 fail (red).
- [ ] **Step 3.** Replace `throw new Error(...)` with `throw new FetchError(r.status, ...)` in `fetchTypes`, `fetchDocs`, `fetchDocContent`, `fetchTemplates`, `fetchTemplateDetail`; confirm all 10 tests pass (green).
- [ ] **Step 3.** Run the full Vitest suite; confirm no consumer broke from the class change.
- [ ] **Step 4.** Create `src/test/router-helpers.tsx` (with `MemoryRouter` and `renderWithRouterAt`) and `src/test/router-helpers.test.tsx` (2 tests); confirm tests pass.
- [ ] **Step 4.** Update `Sidebar.test.tsx` to import `MemoryRouter` from `../../test/router-helpers`; delete `components/Sidebar/test-helpers.tsx`; confirm Sidebar tests still green.
- [ ] **Step 5.** Add `KanbanColumnKey`, `KanbanGroupKey`, `STATUS_COLUMNS`, `OTHER_COLUMN`, `OTHER_COLUMN_KEY` to `src/api/types.ts`.
- [ ] **Step 5.** Run `npm run build`; confirm `tsc -b` succeeds.
- [ ] **Step 6.** Create `src/api/ticket.test.ts` with `parseTicketNumber` tests; confirm all 6 fail (red).
- [ ] **Step 6.** Create `src/api/ticket.ts` with `parseTicketNumber`; confirm all 6 tests pass (green).
- [ ] **Step 7.** Append `groupTicketsByStatus` tests (9 cases) to `ticket.test.ts`; confirm all 9 fail.
- [ ] **Step 7.** Implement `groupTicketsByStatus` in `ticket.ts` (with mtime-then-relPath sort and Other-key omission); confirm all 15 tests pass.
- [ ] **Step 8.** Create `src/routes/kanban/TicketCard.test.tsx` (7 tests); confirm all fail.
- [ ] **Step 8.** Create `routes/kanban/TicketCard.tsx` and `TicketCard.module.css` (canonical Link form, dnd-kit attrs on `<Link>`, suppressed `aria-roledescription`, slug fallback chip); confirm all 7 tests pass.
- [ ] **Step 9.** Create `src/routes/kanban/KanbanColumn.test.tsx` (5 tests); confirm all fail.
- [ ] **Step 9.** Create `KanbanColumn.tsx` and `KanbanColumn.module.css` (`<h2>` heading, count-only aria-label, optional description, aria-hidden empty state); confirm all 5 tests pass.
- [ ] **Step 10.** Create `src/routes/kanban/KanbanBoard.test.tsx` (9 tests); confirm all fail.
- [ ] **Step 10.** Create `KanbanBoard.tsx` and `KanbanBoard.module.css` (`<h1>Kanban</h1>`, `role="status"` loading, retry button in error alert, sensors pre-mounted); confirm all 9 tests pass.
- [ ] **Step 11.** Append two router tests to `router.test.tsx`; confirm both fail.
- [ ] **Step 11.** Swap `KanbanStub` import for `KanbanBoard` in `router.ts`; confirm both new tests pass.
- [ ] **Step 11.** Delete `src/routes/kanban/KanbanStub.tsx`.
- [ ] **Step 11.** Run `npm run build`; confirm no dangling references.
- [ ] **Step 12.** Append the `### Added` bullet to `## Unreleased` in `CHANGELOG.md`.
- [ ] **Phase gate.** Run `mise run test`; confirm all suites green.
- [ ] **Phase gate.** Run `cargo fmt -- --check` and `cargo clippy --all-targets -- -D warnings` from `server/`; confirm both pass.
- [ ] **Phase gate.** Manual verification per the checklist above.

---

## References

- Spec: `meta/specs/2026-04-17-meta-visualisation-design.md` (Kanban section, lines 492–505; Writes and conflict handling, lines 516–540 — Phase 7 deliberately defers the write half).
- Research: `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md` (Phase 7 entry at lines 1069–1080; phasing rationale at lines 1196–1214).
- Phase 6 plan (template for structure, TDD discipline, `FetchError`, and `makeIndexEntry` patterns): `meta/plans/2026-04-25-meta-visualiser-phase-6-lifecycle-clusters-and-view.md`.
- Phase 5 plan (frontend testing patterns origin, `MemoryRouter` helper, CSS module conventions): `meta/plans/2026-04-22-meta-visualiser-phase-5-frontend-scaffold-and-library-view.md`.
- dnd-kit `useSortable` `disabled` argument: <https://docs.dndkit.com/presets/sortable/usesortable>.
- dnd-kit collision detection (`closestCorners` for kanban-style stacked containers): <https://docs.dndkit.com/api-documentation/context-provider/collision-detection-algorithms>.
- dnd-kit Keyboard sensor + `sortableKeyboardCoordinates`: <https://docs.dndkit.com/api-documentation/sensors/keyboard>.
- Existing kanban stub being replaced: `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanStub.tsx:1-3`.
- Existing kanban-relevant infrastructure already in place: `frontend/src/api/query-keys.ts:12` (`queryKeys.kanban()`), `frontend/src/api/use-doc-events.ts:26-28` (ticket-change SSE invalidation).
- Sidebar kanban entry: `frontend/src/components/Sidebar/Sidebar.tsx:5-8, 53`.
- Indexer ticket keying: `server/src/indexer.rs:100-104, 165-168`; `parse_ticket_number` at `server/src/indexer.rs:184-187`.
- `DocTypeKey::Tickets.in_kanban()` invariant: `server/src/docs.rs:69-71` (test: `docs.rs:148-156`).
- Ticket filename pattern enforced by: `skills/tickets/scripts/ticket-next-number.sh:47-67`.
