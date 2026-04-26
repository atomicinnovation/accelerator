---
date: "2026-04-26T22:18:09Z"
type: plan
skill: create-plan
ticket: null
status: revised
revised: "2026-04-27T00:00:00Z"
---

# Phase 8 — Kanban write path implementation plan

## Overview

Make Kanban drag-drop write the destination column's status to the
ticket file on disk. Adds the only write path the visualiser exposes
in v1: `PATCH /api/docs/{*path}/frontmatter` with `If-Match`
optimistic-concurrency. Frontend gains an optimistic mutation that
reconciles silently on success, snaps back with a toast on conflict.

Test-driven throughout: every step writes the failing test first,
then the implementation. Failure modes (412 conflict, 400 invalid,
404 missing, 403 path-escape) get tests before behaviour.

## Current State Analysis

Phase 7 left a fully-wired but write-disabled kanban:

- **Server** (`skills/visualisation/visualise/server/`):
  - `FileDriver` trait has only `list` + `read` — no write surface
    (`src/file_driver.rs:39-49`).
  - `Indexer` exposes `rescan()` (full-tree) + `get(path)`. No
    targeted single-entry refresh (`src/indexer.rs:53-130, 149-156`).
  - `frontmatter::parse()` produces `Parsed { state, body }` with no
    line-aware representation; key→line spans are local to the
    parser and not exposed (`src/frontmatter.rs:26-99`).
  - `api/docs.rs::doc_fetch` shows the working `If-None-Match` /
    304 / quoted-and-unquoted-etag pattern Phase 8 mirrors for
    `If-Match` (`src/api/docs.rs:42-95`).
  - `SseHub::broadcast(SsePayload)` is the only emit point;
    watchers drive it today via `watcher::on_path_changed_debounced`
    → `indexer.rescan()` → `compute_clusters()` → broadcast
    (`src/watcher.rs:97-141`).
  - `tests/common/mod.rs::seeded_cfg_with_tickets` already creates
    three ticket fixtures (`0001-todo-fixture`, `0002-done-fixture`,
    `0003-other-fixture` with `status: blocked`) — perfect for
    PATCH tests.
  - `Cargo.toml` already has `tempfile`, `tower`, `http-body-util`,
    `reqwest`, `tokio` dev-deps (`Cargo.toml:36-55`).
  - axum 0.7 / matchit 0.7 — catch-all-then-static (`*path/frontmatter`)
    is **not registrable**; resolved by handling the `/frontmatter`
    suffix inside a single `*path` handler (see Phase 3 below).

- **Frontend** (`skills/visualisation/visualise/frontend/`):
  - `IndexEntry.etag: string` is on the wire and lives in cache;
    mutation can read it directly (`src/api/types.ts:36-49`).
  - `KanbanBoard` mounts `DndContext` with no-op `onDrag*` handlers;
    `closestCorners` collision; `PointerSensor` + `KeyboardSensor`
    sensors (`src/routes/kanban/KanbanBoard.tsx:30-82`).
  - `KanbanColumn` wraps cards in `SortableContext` keyed by
    `entry.relPath`. Empty columns render `<p>No tickets</p>` with
    no droppable, so empty-column drops fail today
    (`src/routes/kanban/KanbanColumn.tsx:34-36`).
  - `TicketCard` gates drag activation behind
    `const PHASE_7_DISABLED = true` and strips
    `aria-roledescription` from `attributes`
    (`src/routes/kanban/TicketCard.tsx:15-28`).
  - `fetch.ts` is GET-only; `FetchError(status, message)` is
    available and Phase 8 reuses it (`src/api/fetch.ts:10-15`).
  - `useDocEvents` always invalidates on every `doc-changed` —
    no self-cause filter (`src/api/use-doc-events.ts:17-30`).
  - `query-client.ts` sets `staleTime: Infinity, retry: 1` — SSE
    is the source of truth.
  - No toast/snackbar component mounted; existing pattern is
    in-place `role="alert"` regions
    (`KanbanBoard.tsx:56`).
  - `vitest@3` + `@testing-library/react@16` + `vi.stubGlobal('fetch')`
    + `vi.spyOn(fetchModule, ...)`. No MSW, no mutation tests yet.

### Key Discoveries

- **matchit 0.7 forbids catch-all + literal suffix.** Verified
  via web research and matchit issue
  [#39](https://github.com/ibraheemdev/matchit/issues/39). axum 0.8
  does not solve this. Workaround: register one `/api/docs/*path`
  route with both `get` and `patch` method handlers; PATCH handler
  strips a trailing `/frontmatter` from the captured path. From the
  client's perspective the spec URL is preserved exactly:
  `PATCH /api/docs/{path}/frontmatter`.
- **`AppState.file_driver` is concrete `Arc<LocalFileDriver>`**,
  not `Arc<dyn FileDriver>` (`src/server.rs:42`). The new write
  method is added to the trait (so the contract is documented) and
  to the inherent impl; no fake driver is needed for tests since
  integration tests run against a real tempdir.
- **Indexer needs a targeted single-path refresh.** Reusing the
  full `rescan()` after every PATCH is correct but wasteful; more
  importantly the broadcast race with the watcher's own debounced
  rescan is hard to reason about. Phase 2 adds
  `Indexer::refresh_one(path)`.
- **dnd item id == ticket relPath** — sortable items are keyed by
  `relPath` already, so `over.id` from `onDragEnd` resolves
  directly to an `IndexEntry` via the cached docs list.
- **The "Other" swimlane is read-only**: drops onto cards in the
  Other column, or PATCHes that target a non-allowlisted value,
  must not be issued. Validation lives in the drag handler (no
  PATCH issued) and in the server (400 if attempted).
- **Self-cause filter design**: a small module-scoped
  `Set<string>` of recently-emitted etags lets `useDocEvents` skip
  invalidation when the inbound `doc-changed` event's etag is one
  the local mutation just produced. Bounded TTL (a few seconds)
  prevents the set from growing unbounded.

## Desired End State

After this phase ships:

1. Drag a ticket card from `todo` to `in-progress` in the kanban
   UI. The card moves immediately (optimistic).
2. The server writes `status: in-progress` to the ticket's
   frontmatter file in place, preserving comments, key order, and
   surrounding whitespace.
3. The server returns `204 No Content` with a fresh `ETag` header.
4. A second browser tab open on the same kanban observes the move
   without a flicker (SSE broadcast → invalidation → refetch).
5. If the underlying ticket file has been edited externally
   between the read and the PATCH, the response is `412 Precondition
   Failed` with a JSON body carrying the current etag. The card
   snaps back to its origin column and an `aria-live` region
   announces the conflict.
6. Disallowed status values (`anything-else`) are rejected
   client-side (no PATCH issued) and, defensively, server-side
   (400 Bad Request if a custom client tried).
7. `<meta/tickets/*.md>` files retain all existing frontmatter
   keys, comments, and trailing newlines after every successful
   PATCH.
8. The `useDocEvents` SSE listener does not trigger any
   refetch for echoes of the local browser's own mutation
   (non-consuming self-cause filter; suppresses every echo
   inside the TTL window, not just the first).
9. **Exactly one** `doc-changed` event is broadcast per
   successful PATCH: the handler emits, and the watcher's
   debounced rescan suppresses its would-be duplicate via the
   server-side `recently_self_written` set. Idempotent same-value
   PATCHes emit zero events because the driver short-circuits.
10. Two concurrent PATCHes against the same ticket with the same
    `If-Match` resolve via the per-path mutex: one returns 204,
    the other 412. No silent last-write-wins.
11. Ticket file Unix permissions (mode, ownership) are
    preserved across PATCH — the new file inherits the
    original's mode, not `NamedTempFile`'s default `0o600`.
12. Crash- and power-loss-safe: file contents and parent dir
    are fsync'd before/after the atomic rename.
13. Missing `If-Match` returns 428 (no `currentEtag` body); only
    stale `If-Match` returns 412 (with `currentEtag`). A buggy
    client that forgets the header does not trigger the
    optimistic-concurrency rollback UI.
14. PATCH from a foreign `Origin` header is rejected with 403
    (defence-in-depth against any future CORS misconfiguration).
15. SSE invalidations from other tabs are *queued* (not applied)
    while a drag is in progress in this tab, then flushed on
    drop — preventing the React reconciler from remounting
    `useSortable` items mid-gesture and aborting the drag.
16. Conflict UI announces via `role="alert"`; non-blocking
    rejections (Other-column drop, keyboard same-column no-op)
    announce via `role="status"`. After conflict, focus is
    restored to the rolled-back card so keyboard users can
    retry without re-navigating.

### Verification

- `curl -X PATCH -H 'If-Match: "<etag>"' -H 'Content-Type: application/json'
  -d '{"patch":{"status":"in-progress"}}'
  http://127.0.0.1:<port>/api/docs/meta/tickets/0001-todo-fixture.md/frontmatter`
  returns `204` with a new `ETag` header. The on-disk file's
  `status:` line shows `in-progress`; comments and other fields
  are unchanged.
- The same `curl` re-issued with the now-stale etag returns
  `412` and `{"currentEtag": "..."}` body.
- `cargo test -p accelerator-visualiser` is green.
- `npm test` (vitest) inside `frontend/` is green.

## What We're NOT Doing

- **Body editing**. Only frontmatter is patchable. Markdown body
  content is untouchable in v1.
- **Other frontmatter keys**. The patch field allowlist is
  `{"status"}` only, on tickets only. Spec defers
  `tags`/`status-on-non-tickets` etc. to v2.
- **Within-column reordering**. Drag is between-column only;
  intra-column reorder is deferred (matches Phase 7 plan §
  "What we are NOT doing").
- **The "Other" swimlane as a drop target**. Drops onto Other
  cards from another column don't dispatch a PATCH (no
  allowlisted value); the user gets a no-op rebound. Other →
  Other reorder is also no-op.
- **A new toast component or animation library**. Conflict UI
  is an `aria-live="polite"` region in `KanbanBoard`, mirroring
  the existing `role="alert"` error pattern.
- **Bumping axum / switching routing libraries**. The spec URL
  is preserved by handling the `/frontmatter` suffix inside the
  PATCH handler.
- **Multi-key atomic patches**. Body shape `{"patch": {...}}` is
  preserved for forward-compat, but Phase 8 only honours one key.
- **Promote-inferred-to-explicit cross-ref affordance**. Out of
  scope (post-v1 roadmap item).

## Implementation Approach

Five inner phases, each TDD:

1. Patcher module (server, pure) — bytes in / bytes out.
2. Write extension on `FileDriver` + targeted `Indexer` refresh.
3. PATCH endpoint, validation, ETag, SSE broadcast.
4. Frontend mutation primitives + self-cause SSE filter.
5. Frontend dnd-kit wiring + UX (column droppables, card flip,
   conflict announcement).

Each step writes its failing test first. Step ordering keeps the
build green except inside the step that introduces a new failing
test.

The Phase 7 plan's per-step "Success criteria with exact
invocations" discipline is preserved.

---

## Phase 1: Patcher module (server, pure)

### Overview

Introduce `src/patcher.rs`: a pure function that takes raw
file bytes plus a `(key, value)` pair and returns the patched
bytes. Targets one allowlisted key (`status`) on tickets;
preserves comments, key order, and surrounding whitespace by
operating on the YAML source line-by-line, not via a YAML
roundtrip.

### Changes Required

#### 1. New module `src/patcher.rs`

```rust
// signature
pub fn patch_status(raw: &[u8], new_value: TicketStatus)
    -> Result<Vec<u8>, PatchError>;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PatchError {
    #[error("frontmatter is absent")]
    FrontmatterAbsent,
    #[error("frontmatter is malformed")]
    FrontmatterMalformed,
    #[error("status key not present in frontmatter")]
    KeyNotFound,
    #[error("status value shape is unsupported: {reason}")]
    UnsupportedValueShape { reason: String },
}
```

The implementation **shares fence detection with the existing
parser** via a new `frontmatter::fence_offsets(raw: &[u8])
-> Result<Option<(usize, usize)>, FenceError>` helper that returns
the byte range of the YAML region (or `None` when frontmatter is
absent). Both `frontmatter::parse` and `patch_status` consume it,
so "where is the frontmatter" has exactly one implementation; CRLF
handling, max-scan window, and unclosed-fence rejection cannot
drift between read and write.

Inside the frontmatter region, the patcher walks line-by-line for
a top-level `status:` key (rejecting indented occurrences inside
nested mappings). On match, the **supported value shape** is a
simple scalar on the same line, optionally double- or
single-quoted, optionally followed by an inline comment after
whitespace. Any other shape — block scalar (`|`/`>`), YAML anchor
(`&x todo`), alias (`*x`), flow-style mapping (`{status: ...}`),
multi-line value continuation, or value containing `#` outside a
quoted region — returns `Err(UnsupportedValueShape { reason })`
naming the unsupported feature. The patcher does not attempt to
mutate exotic forms; the user can fix by hand.

On a supported match, the patcher replaces the value content only,
preserving the original quote style (none / single / double) and
any trailing inline comment with its leading whitespace, and
preserving the line's original line ending (LF or CRLF) verbatim
even if the rest of the file uses a different ending.

The allowed-status set is enforced *not* by a string allowlist in
the patcher but by the `TicketStatus` enum (Phase 3): the patcher
takes `TicketStatus` directly, so unknown values can never reach
this layer. This eliminates the duplicate allowlist that would
otherwise need to stay in sync between patcher, handler, and
frontend.

#### 2. Wire into `lib.rs`

```rust
pub mod patcher;
```

### TDD Sequence (Phase 1)

For each step, the test is added first; `cargo test patcher` is
expected to fail until the implementation lands. All tests live
inline in `src/patcher.rs` under
`#[cfg(test)] mod tests { use super::*; ... }` to mirror
`src/frontmatter.rs`'s pattern.

1. **Step 1.1** — `replaces_simple_unquoted_status_value`:
   input `"---\nstatus: todo\n---\n# body\n"` → output
   `"---\nstatus: in-progress\n---\n# body\n"` for
   `patch_status(input, "in-progress")`.
2. **Step 1.2** — `preserves_other_frontmatter_keys_and_order`:
   input has `title: Foo\nstatus: todo\nticket: bar\n`; output
   keeps `title:` line, replaces `status:`, keeps `ticket:`.
3. **Step 1.3** — `preserves_quoted_status_values`: input
   `status: "todo"` → output `status: "in-progress"` (preserve
   quote style; only the value content changes).
4. **Step 1.4** — `preserves_inline_comment_after_status`: input
   `status: todo  # current` → output
   `status: in-progress  # current`.
5. **Step 1.5** — `preserves_body_byte_for_byte`: input has body
   markdown including triple-dash separators inside fenced code;
   the body region after the closing `---` is identical in the
   output.
6. **Step 1.6** — `preserves_crlf_line_endings`: input uses
   `\r\n`; output uses `\r\n`.
7. **Step 1.7** — `idempotent_for_same_value`:
   `patch_status(b"---\nstatus: done\n---\n", "done")` returns
   bytes equal to input.
8. **Step 1.8** — `accepts_each_ticket_status_variant`:
    `patch_status(..., TicketStatus::Todo | InProgress | Done)`
    each succeeds, producing the expected serialised value
    (`todo` / `in-progress` / `done`). The patcher cannot be
    called with an unknown value because the enum prohibits it
    at compile time — there is no runtime allowlist to keep in
    sync with the handler.
9. **Step 1.9** — `rejects_when_status_key_missing`: input
   frontmatter without `status:` returns `Err(KeyNotFound)`.
10. **Step 1.10** — `rejects_when_frontmatter_absent`: input
    starts with `# Heading\n` returns
    `Err(FrontmatterAbsent)`.
11. **Step 1.11** — `rejects_when_frontmatter_malformed`: input
    has opening `---` but no closing fence within the scan
    window → `Err(FrontmatterMalformed)`.
12. **Step 1.12** — `does_not_mutate_indented_status_in_nested_mapping`:
    input has `metadata:\n  status: todo\nstatus: todo\n` →
    only the top-level `status:` is changed; the nested one is
    untouched.
13. **Step 1.13** — `does_not_close_frontmatter_at_body_internal_triple_dash`:
    construct a fixture where the body contains a fenced code
    block whose first line is `---` *before* the real
    frontmatter close in some pathological way (e.g. real
    frontmatter close is at line N, with the body containing
    `---` at line N-2 inside a fenced code block embedded in a
    multi-line value? — design the fixture so a naive
    "second-`---`-ends-frontmatter" parser would mis-locate the
    close). Assert the patcher rewrites the real frontmatter's
    `status:` and leaves the body's `---` untouched. Pairs with
    Step 1.5 to pin fence-detection correctness.
14. **Step 1.14** — `rejects_block_scalar_status_value`: input
    `status: |\n  todo\n` returns
    `Err(UnsupportedValueShape { reason })` where `reason`
    names "block scalar".
15. **Step 1.15** — `preserves_single_quoted_status_value`:
    input `status: 'todo'` → output `status: 'in-progress'`
    (single-quote style preserved).
16. **Step 1.16** — `rejects_anchored_status_value`: input
    `status: &s todo` returns
    `Err(UnsupportedValueShape { reason })` where `reason`
    names "anchor".
17. **Step 1.17** — `rejects_flow_style_mapping_status`: input
    `{status: todo}` (top-level flow mapping) → either
    `KeyNotFound` (line-based scanner doesn't see top-level
    `status:`) or `UnsupportedValueShape`. Pin which.
18. **Step 1.18** — `preserves_line_specific_line_ending`:
    file uses LF everywhere except the `status:` line uses
    CRLF; output preserves the CRLF on the rewritten line and
    LF on every other line, byte-for-byte.

### Success Criteria

#### Automated

- [ ] `cd skills/visualisation/visualise/server && cargo test --lib patcher::`
      runs and all 18 tests pass.
- [ ] `cargo test --lib frontmatter::fence_offsets` — the new
      shared helper has its own tests and `frontmatter::parse`
      is refactored to consume it (asserted by all existing
      `frontmatter::tests` continuing to pass).
- [ ] `cargo clippy --all-targets -- -D warnings` clean.
- [ ] `cargo fmt --check` clean.

#### Manual

- [ ] Run the patcher manually on a real ticket file in
      `meta/tickets/0001-*.md`: `cargo run --bin patcher-smoke`
      *(omitted — pure-function module, integration tests cover this)*.

---

## Phase 2: FileDriver write extension + Indexer refresh

### Overview

Add a `write_frontmatter` method to the `FileDriver` trait and
implement on `LocalFileDriver`. The implementation composes
patcher + path-safety guard + atomic rename, then returns the
new ETag. Add `Indexer::refresh_one(path)` for targeted post-write
refresh that does not require a full tree rescan.

### Changes Required

#### 1. Extend `FileDriver` trait — `src/file_driver.rs`

```rust
pub trait FileDriver: Send + Sync {
    fn list(...) -> ...;
    fn read(...) -> ...;
    fn write_frontmatter(
        &self,
        path: &Path,
        patch: FrontmatterPatch<'_>,
        if_match: &str,
    ) -> Pin<Box<dyn Future<Output = Result<FileContent, FileDriverError>> + Send + '_>>;
}

// FrontmatterPatch lives in a domain types module (alongside
// TicketStatus), not co-located with the transport trait. The
// trait knows about "frontmatter patches" structurally; the
// specific keys it can carry are a domain concern.
#[derive(Debug, Clone, Copy)]
pub enum FrontmatterPatch {
    Status(TicketStatus),
}

// extra error variants
pub enum FileDriverError {
    // ... existing variants ...
    EtagMismatch { current: String },
    Patch(PatchError),
    PathNotWritable { path: PathBuf },
    CrossFilesystem { path: PathBuf },
}
```

**Writable-roots, not ticket-roots.** The driver is constructed
with a separate `writable_roots: Vec<PathBuf>` (parallel to the
existing `roots` for reads). The driver verifies the canonical
path is under one of `writable_roots` — it does *not* know that
"tickets" is the only writable doc-type. That product policy
lives in the handler (which decides which root to pass through)
and the index-lookup-then-`r#type`-check (which rejects
non-ticket entries before calling the driver). The error variant
is renamed `PathNotWritable` accordingly. When v2 broadens
writes to non-tickets, only the handler's `r#type` check and the
driver's `writable_roots` configuration change — the trait stays
stable.

For v1, `writable_roots = vec![cfg.tickets_root.clone()]`.

Decision: keep `AppState.file_driver` as concrete
`Arc<LocalFileDriver>` (no fake driver in tests). Tests run
against a real tempdir using the `seeded_cfg_with_tickets`
helper.

#### 2. Implement on `LocalFileDriver`

The driver holds a `path_locks: DashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>`
keyed by canonicalised path. Each write acquires the per-path mutex
before reading and holds it until after the rename — closing the
read→check→rename TOCTOU window so two concurrent writes cannot both
pass the etag check.

To keep `write_frontmatter` legible despite its growing
responsibilities (canonicalise, writable-roots check, lock,
read+etag, patch, idempotent short-circuit, atomic write,
fsync, perms preservation, EXDEV mapping, register self-write),
the implementation is decomposed into three private helpers
plus a coordinator. Each helper has a single reason to change:

```rust
// Returns the canonicalised, writable, locked path. The returned
// guard must be held until the write completes.
async fn acquire_canonical_writable_path(
    &self, path: &Path,
) -> Result<(PathBuf, MutexGuard<'_, ()>), FileDriverError>;

// Reads bytes; if the current etag doesn't match `if_match`,
// returns EtagMismatch{current}. Tolerates quoted/unquoted etags.
async fn read_and_check_etag(
    &self, canonical: &Path, if_match: &str,
) -> Result<Vec<u8>, FileDriverError>;

// fsync(file) → set perms from `original_perms` → persist(canonical)
// → fsync(parent dir). Maps EXDEV to CrossFilesystem.
async fn atomic_write_preserving_perms(
    canonical: &Path,
    new_bytes: &[u8],
    original_perms: std::fs::Permissions,
) -> Result<(), FileDriverError>;
```

The public `write_frontmatter` reads as a six-step coordinator:

```rust
pub async fn write_frontmatter(
    &self,
    path: &Path,
    patch: FrontmatterPatch,
    if_match: &str,
    on_committed: impl FnOnce(&Path) + Send,  // see "register-while-locked"
) -> Result<FileContent, FileDriverError> {
    let (canonical, _guard) = self.acquire_canonical_writable_path(path).await?;
    let original_perms = tokio::fs::metadata(&canonical).await?.permissions();
    let bytes = self.read_and_check_etag(&canonical, if_match).await?;
    let new_bytes = patcher::apply(&bytes, patch)?;

    // Idempotent short-circuit: no rename, no fsync, no register,
    // no broadcast. mtime stays unchanged.
    if new_bytes == bytes {
        let mtime = tokio::fs::metadata(&canonical).await?.modified()?;
        return Ok(FileContent { etag: etag_of(&bytes), mtime, /* ... */ });
    }

    atomic_write_preserving_perms(&canonical, &new_bytes, original_perms).await?;
    on_committed(&canonical);  // INSIDE the lock — closes the watcher race
    let mtime = tokio::fs::metadata(&canonical).await?.modified()?;
    Ok(FileContent { etag: etag_of(&new_bytes), mtime, /* ... */ })
    // _guard dropped here: lock released after on_committed has
    // recorded the self-write.
}
```

**Register-while-locked** is the critical ordering for closing
the watcher race. The `on_committed` callback runs *while the
per-path mutex is still held* and *after* persist + parent-dir
fsync — so the kernel may have already queued the inotify event,
but the watcher's debounced handler has not yet fired (the
debounce window is ~100 ms). The handler passes a closure that
inserts the canonical path into `state.recently_self_written`;
by the time the watcher's debounce expires and it consults the
set, the path is present and the duplicate broadcast is
suppressed.

If the write fails before `on_committed`, no register happens,
so the watcher (correctly) treats the would-be event as
external. If the write succeeds and `on_committed` registers,
but the consumer (handler) fails to broadcast for some reason,
the watcher still sees the suppressed-self entry and skips its
broadcast — which is wrong (no event reaches anyone). The
handler's broadcast must therefore be infallible (or its
failure must `recently_self_written.remove(canonical)` to let
the watcher take over).

**Cross-filesystem failure mode**: `persist` on a tempfile in a
different filesystem from the target falls back to copy-then-delete,
which is **not** atomic. Map `tempfile::PersistError` whose source
is `EXDEV` to a distinct `FileDriverError::CrossFilesystem` variant
rather than a generic IO error. Same-filesystem placement is
guaranteed because `new_in` uses the canonicalised parent.

#### 3. Add `Indexer::refresh_one` — `src/indexer.rs`

```rust
pub async fn refresh_one(&self, path: &Path)
    -> Result<Option<IndexEntry>, FileDriverError>;
```

- **Acquires the same `rescan_lock` semaphore that the watcher uses
  for full rescans.** This serialises `refresh_one` against the
  watcher's debounced `rescan()` so the two cannot interleave;
  without this, a watcher rescan that started before `refresh_one`
  could swap its freshly-built map in *after* `refresh_one`
  completes, silently overwriting the targeted update.
- If the file no longer exists, remove from `entries` (and from
  `adr_by_id` / `ticket_by_number` if applicable) and return
  `Ok(None)`.
- Otherwise re-read via the driver, re-derive the same shape
  `rescan()` produces, replace the entry under
  `entries.write().await`, return `Ok(Some(entry))`.

The implementation factors out the per-file body of `rescan()`
into a private `build_entry(kind, path, content) -> IndexEntry`
helper that both `rescan` and `refresh_one` use, eliminating
duplication.

#### 4. Suppress watcher broadcasts for handler-driven writes — `src/watcher.rs` + `src/write_coordinator.rs` (new)

A new `WriteCoordinator` module (small struct, sibling to
`SseHub`) owns the suppression set instead of leaving it as a raw
field on `AppState`:

```rust
pub struct WriteCoordinator {
    recent: Mutex<HashMap<PathBuf, Instant>>,
    ttl: Duration,
    max_entries: usize,        // FIFO cap, default 256
    now: Box<dyn Fn() -> Instant + Send + Sync>,  // injectable
}

impl WriteCoordinator {
    pub fn mark_self_write(&self, canonical: &Path);
    pub fn should_suppress(&self, canonical: &Path) -> bool;  // consumes on hit
}
```

`AppState` holds `Arc<WriteCoordinator>`. The driver's
`on_committed` callback (see §2 above) calls `mark_self_write`
*while still holding the per-path mutex*. The watcher's
`on_path_changed_debounced` calls `should_suppress` after
canonicalising the event path (see below); if true, it skips
both rescan and broadcast.

**Canonicalise the watcher event path before consulting the
coordinator.** The handler/driver insert canonical paths; the
watcher receives paths shaped by the notifier, which on macOS
(`/var ↔ /private/var`), Linux bind mounts, and Windows symlinks
may differ from the canonical form. Inside
`on_path_changed_debounced`, call `tokio::fs::canonicalize` on
the event path and use the result as the lookup key. If
canonicalisation fails (file deleted between event and
debounce), fall back to the original path — the lookup will
miss, the watcher rescans, the indexer will pick up the
deletion. A test creates a symlinked tickets directory (the
canonical path differs from the watcher-registered path) and
confirms dedup still works.

**FIFO cap (256, mirroring frontend).** `mark_self_write` evicts
the oldest entry when the map is at capacity, in addition to
lazy TTL pruning. Bounds memory under sustained PATCH bursts.

**Injectable `now`.** Tests construct the coordinator with a
spy clock so TTL behaviour is deterministic, mirroring the
frontend `SelfCauseRegistry` design.

External edits are unaffected because their paths are never
inserted via `mark_self_write`.

### TDD Sequence (Phase 2)

#### `LocalFileDriver::write_frontmatter` — integration tests in `src/file_driver.rs` `#[cfg(test)] mod write_tests`

For each step the test goes first.

1. **Step 2.1** — `writes_status_to_disk_atomically`: tempdir
   with a ticket file `tickets/0001-foo.md`,
   `LocalFileDriver::new(...)`, call `write_frontmatter` with the
   correct etag. Assert the file on disk now contains
   `status: in-progress`.
2. **Step 2.2** — `returns_new_etag_and_mtime_in_filecontent`:
   the returned `FileContent.etag` differs from the input
   etag and matches `etag_of(new_bytes)`.
3. **Step 2.3** — `preserves_other_frontmatter_and_body`:
   re-read the file via `tokio::fs::read_to_string` after the
   write; verify all non-status keys, body markdown, and
   trailing newline are byte-for-byte identical (modulo the
   single status line).
4. **Step 2.4** — `rejects_etag_mismatch_with_current_etag`:
   pre-edit the file out-of-band (writing a different
   `status: done` directly), then call `write_frontmatter` with
   the original etag. Assert
   `Err(EtagMismatch { current })` where `current` matches the
   etag of the on-disk content. The on-disk file is unchanged.
5. **Step 2.5** — `rejects_path_outside_tickets_root`: tempdir
   has both `tickets/foo.md` and `plans/foo.md`; calling
   `write_frontmatter` on the plans path returns
   `Err(OnlyTicketsAreWritable)`. The plans file is unchanged.
6. **Step 2.6** — `rejects_path_escape_via_symlink` (`#[cfg(unix)]`):
   create a symlink inside `tickets/` pointing at a sibling
   `plans/sneaky.md`; canonicalisation must reject it as
   `PathNotWritable`. Gated on Unix because Windows symlinks
   require elevated privileges. The Windows path-escape story
   is covered by the canonicalize+writable_roots check itself
   (which has its own non-symlink test); a comment in the test
   names the gating rationale.
7. **Step 2.7** — `propagates_patcher_error_for_disallowed_value`:
   `write_frontmatter` with `FrontmatterPatch::Status("blocked")`
   returns `Err(Patch(ValueNotAllowed("blocked")))`. The file
   is unchanged.
8. **Step 2.8** — `propagates_patcher_error_when_frontmatter_absent`:
   ticket fixture with no frontmatter → `Err(Patch(FrontmatterAbsent))`.
9. **Step 2.9** — `concurrent_writes_with_same_if_match_one_returns_etag_mismatch`:
   use a `tokio::sync::Barrier::new(2)` so both tasks start their
   `write_frontmatter` call simultaneously after passing the barrier.
   Both observe the same starting etag from the index. Assert
   exactly one returns `Ok(_)` and exactly one returns
   `Err(EtagMismatch{..})`. The on-disk content equals the
   payload of the winning task. The per-path mutex serialises the
   read-through-rename window so the second task observes the
   first task's write before its own etag check.
10. **Step 2.10** — `tolerates_quoted_etag_in_if_match`: the
    `if_match` parameter is accepted as `"sha256-..."` (with
    surrounding quotes) or `sha256-...` (bare); both succeed.
11. **Step 2.11** — `idempotent_same_value_short_circuits`: file
    has `status: in-progress`. PATCH with `Status("in-progress")`
    and the current etag returns `Ok(FileContent)` whose etag
    equals the input etag and whose mtime equals the on-disk
    mtime *before* the call (no rename happened). Verify by
    capturing mtime before and after — it must be unchanged.
12. **Step 2.12** — `preserves_unix_file_permissions`: on
    `#[cfg(unix)]`, set the original file to mode `0o644` via
    `tokio::fs::set_permissions`. After `write_frontmatter`,
    re-read metadata and assert mode is still `0o644` (not
    `NamedTempFile`'s default `0o600`).
13. **Step 2.13** — `cross_filesystem_persist_returns_cross_filesystem_error`:
    skip on platforms where simulating a separate filesystem is
    impractical; on Linux, mount a tmpfs into the test layout and
    place the tempfile parent on a different fs than the target.
    Assert `Err(CrossFilesystem { .. })` rather than a generic IO
    error. Marked `#[cfg(target_os = "linux")]` and
    `#[ignore]` by default — opt-in for environments where the
    setup is available; documents the failure mode.

#### `Indexer::refresh_one` — tests in `src/indexer.rs` `#[cfg(test)] mod refresh_tests`

14. **Step 2.14** — `refresh_one_picks_up_external_edit`:
    tempdir + indexer; write a new ticket file, call
    `refresh_one(path)`, assert the entry now appears in
    `all_by_type(Tickets)` with the expected etag and slug.
15. **Step 2.15** — `refresh_one_updates_etag_on_change`: build
    indexer, edit a ticket file out-of-band, call
    `refresh_one(path)`. Assert the entry's etag matches the new
    content; old etag is gone.
16. **Step 2.16** — `refresh_one_removes_deleted_file`: build
    indexer with a ticket fixture, delete the file from disk,
    call `refresh_one(path)` → returns `Ok(None)`; the entry is
    gone from `entries`; if a ticket-number index existed, that
    is also cleaned up.
17. **Step 2.17** — `refresh_one_does_not_disturb_unrelated_entries`:
    build indexer with 3 tickets; `refresh_one` on ticket #1.
    Tickets #2 and #3 are byte-identical in `entries` (same
    etag, same mtime, same path).
18. **Step 2.18** — `refresh_one_rebuilds_secondary_indexes_for_tickets_and_decisions`:
    refreshing a ticket file updates `ticket_by_number`;
    refreshing an ADR file updates `adr_by_id`.
19. **Step 2.19** — `refresh_one_serialises_with_concurrent_rescan`:
    build indexer; spawn `rescan()` and `refresh_one(path)`
    concurrently using a `tokio::sync::Barrier::new(2)`. After
    both complete, the entry produced by `refresh_one` is the one
    present in `entries` (the rescan's wholesale-replacement path
    must not silently overwrite the targeted update). This pins
    the `rescan_lock`-sharing contract.

### Success Criteria

#### Automated

- [ ] `cargo test --lib file_driver::write_tests::` — all 13 tests pass
      (steps 2.13 marked `#[ignore]`).
- [ ] `cargo test --lib indexer::refresh_tests::` — all 6 tests pass.
- [ ] `cargo test --lib` — full library tests still green.
- [ ] `cargo clippy --all-targets -- -D warnings` clean.

#### Manual

- [ ] Inspect a real ticket file before and after a manual
      `write_frontmatter` call: comments and key ordering preserved.

---

## Phase 3: PATCH endpoint + SSE broadcast

### Overview

Wire `PATCH /api/docs/{*path}/frontmatter` into the axum router
by sharing the `/api/docs/*path` route between `get` and
`patch` method handlers. The PATCH handler strips the trailing
`/frontmatter` suffix from the captured path; if absent, returns
400. On success, recomputes the index entry and broadcasts a
`doc-changed` SSE event so other browsers reconcile.

### Changes Required

#### 1. Add PATCH method to existing route — `src/api/mod.rs`

```rust
.route(
    // PATCH URL exposed to clients is /api/docs/{path}/frontmatter,
    // but matchit 0.7 forbids catch-all + literal suffix
    // (https://github.com/ibraheemdev/matchit/issues/39). The PATCH
    // handler strips the trailing `/frontmatter` from the captured
    // *path. GET requests continue to treat *path as the doc path.
    "/api/docs/*path",
    get(docs::doc_fetch).patch(docs::doc_patch_frontmatter),
)
```

Both `doc_fetch` and `doc_patch_frontmatter` carry a doc-comment
naming the matchit limitation and pointing at this route
registration so a future maintainer doesn't try to "fix" the
shared-route shape during an axum bump.

#### 2. New handler — `src/api/docs.rs`

```rust
// In a shared types module (consumed by patcher, driver, handler):
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TicketStatus {
    Todo,
    InProgress,
    Done,
}

impl TicketStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in-progress",
            Self::Done => "done",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchFrontmatterBody {
    patch: PatchFields,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchFields {
    // Only `status` is allowed in v1. Adding new fields is a single
    // line each — no separate handler-level allowlist to keep in sync.
    status: Option<TicketStatus>,
}

pub(crate) async fn doc_patch_frontmatter(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
    headers: HeaderMap,
    Json(body): Json<PatchFrontmatterBody>,
) -> Result<Response, ApiError>;
```

`deny_unknown_fields` on both structs means a body like
`{"patch":{"status":"todo","title":"foo"}}` is rejected by the
JSON extractor before the handler runs (axum's `Json<T>` returns
400 with a parse error). `TicketStatus` deserialisation rejects
unknown values automatically — the handler does not need a
runtime allowlist. Adding `Blocked` in v2 is one enum variant
plus its serde rename.

Sequence inside the handler:

1. `let doc_rel = path.strip_suffix("/frontmatter")
      .ok_or(ApiError::PatchEndpointMismatch)?;`
   (returns 400 with body `{"error": "patch URL must end with /frontmatter"}`).
2. **Per-segment path validation** (replaces the brittle
   `contains("..")` substring check). Split `doc_rel` on `/`. If
   any segment equals `..`, equals `.`, is empty, contains `\`,
   or contains a NUL byte → `ApiError::PathEscape`. Reject
   absolute paths (`doc_rel.starts_with('/')`) likewise. The
   substring approach was load-bearing only by accident; the real
   defence is the canonicalize+prefix check inside the driver.
   Per-segment validation is precise (allows legitimate
   filenames like `0001..todo.md`) while still rejecting `..`
   directory traversal.
3. `let abs = state.cfg.project_root.join(doc_rel);`
4. Look up the entry in the index. If not present →
   `ApiError::NotFound`.
5. Reject if `entry.r#type != DocTypeKey::Tickets` →
   `ApiError::OnlyTicketsAreWritable` (400).
6. Validate body: `body.patch.status` must be `Some(_)` (a patch
   that touches no fields is meaningless). If `None` →
   `ApiError::InvalidPatch("patch object is empty")` (400). The
   value-allowlist check is **not needed at this layer** because
   the `TicketStatus` enum has already validated the value during
   deserialisation; an unknown value would have been rejected by
   axum's `Json<T>` extractor with 400 before the handler ran.
7. Read `If-Match` header — required.
   `headers.get(http::header::IF_MATCH)` → strip surrounding `"`.
   If absent → `ApiError::IfMatchRequired` → **428 Precondition
   Required**, with body `{"error":"if-match-required"}`. No
   `currentEtag` is included because the client demonstrated no
   prior knowledge of any etag — leaking the current hash here
   would turn the endpoint into an oracle. Per RFC 7232 §6 and
   RFC 6585 §3, 428 is the correct status for a missing required
   precondition; 412 is reserved strictly for *failed*
   preconditions (i.e. stale `If-Match`). Distinguishing the two
   matters because a client bug that omits the header should not
   trigger the optimistic-concurrency rollback UI.

   Reject any `If-Match` value that contains commas, starts with
   `W/`, or equals `*` → 400 with a clear message naming the
   unsupported feature (weak etags, etag lists, wildcard). The
   driver only handles strong, single, quoted-or-unquoted etags.
8. Call `state.file_driver.write_frontmatter(&abs,
      FrontmatterPatch::Status(value), if_match)`.
9. On `Err(EtagMismatch { current })` → 412 with body
   `{"currentEtag": current}` and `ETag` header set to that
   etag.
10. On other errors map to `ApiError` (Patch errors → 400 with
    a clear message; IO → 500; OnlyTicketsAreWritable already
    handled above; NotFound → 404).
11. Call the driver, passing an `on_committed` closure that
    captures `state.write_coordinator.clone()` and calls
    `coordinator.mark_self_write(&canonical)`. The driver invokes
    this closure **inside its per-path mutex, after persist+fsync,
    before releasing the lock** — closing the race where an
    inotify event could otherwise reach the watcher's debounce
    window before the suppression entry exists.

    On success the driver returns `FileContent { etag: new_etag,
    canonical: PathBuf, .. }`. The handler then performs the
    following actions in this **specific order**:

    1. **Refresh the index first**:
       `state.indexer.refresh_one(&canonical).await` (which
       acquires `rescan_lock`). This way subscribers that refetch
       on the broadcast hit a fresh index.
    2. **Then broadcast**:
       `state.sse_hub.broadcast(SsePayload::DocChanged {
           doc_type: entry.r#type, path: doc_rel.to_string(),
           etag: Some(new_etag.clone()) })`.
       Use `new_etag` from the driver's `FileContent` — never
       read it from the indexer after refresh, because between
       persist and refresh a third party could edit the file,
       producing a different etag and breaking the self-cause
       filter for this tab.

    **Idempotent short-circuit**: if the driver short-circuited
    (returned bytes equal to input), the `on_committed` callback
    was *not* invoked (driver skipped that path), so nothing was
    inserted into the coordinator. The handler skips refresh
    (no change to pick up) and skips broadcast (no event). Still
    respond `204` with the unchanged `ETag` header.

    **Broadcast-failure recovery**: if `sse_hub.broadcast` fails
    after `mark_self_write` registered the path, the handler
    must call `state.write_coordinator.unmark(&canonical)` to
    let the watcher take over (otherwise no consumer ever sees
    the event). The watcher's broadcast is correct, just
    delayed by the debounce window.
12. Respond `204 No Content` with `ETag: "<new_etag>"` header.

#### 3. Extend `ApiError` — `src/api/mod.rs`

```rust
pub(crate) enum ApiError {
    // existing variants
    PatchEndpointMismatch,            // 400
    OnlyTicketsAreWritable,           // 400
    InvalidPatch(String),             // 400
    UnsupportedIfMatch(String),       // 400 (weak/list/wildcard)
    IfMatchRequired,                  // 428 + body { error }
    EtagMismatch { current: String }, // 412 + body { currentEtag }
}
```

`IntoResponse`:
- `IfMatchRequired` returns **428 Precondition Required** with
  body `{"error":"if-match-required"}`. No `currentEtag`, no
  `ETag` header — the client gets no information about the
  resource's current state until it sends a real precondition.
- `EtagMismatch` returns **412 Precondition Failed** with body
  `{"currentEtag":"<value>"}` and the `ETag` header set to that
  value. Etag is already public via GET; echoing it here is fine
  because the client demonstrated prior knowledge by sending an
  `If-Match`.

#### 4. `api_from_fd` — map new `FileDriverError` variants

```rust
F::EtagMismatch { current } => ApiError::EtagMismatch { current },
F::Patch(p) => ApiError::InvalidPatch(p.to_string()),
F::PathNotWritable { .. } => ApiError::OnlyTicketsAreWritable,
F::CrossFilesystem { .. } => ApiError::Internal,
```

#### 5. Origin pin for state-changing methods — `src/server.rs` middleware

Extend the existing `host_header_guard` (or add a sibling
`origin_guard`) to reject state-changing requests (`PATCH`,
`POST`, `PUT`, `DELETE`) whose `Origin` header is set to anything
other than the bound listen address (`http://127.0.0.1:<port>` /
`http://localhost:<port>`). Requests with no `Origin` header (e.g.
`curl`, server-to-server) are allowed through, matching browser
semantics — the browser only omits `Origin` for safe top-level
GETs.

This is **defence-in-depth against a future CORS misconfiguration**.
Today, browsers block cross-origin PATCHes with custom headers
(`If-Match`) at the preflight step because no `Access-Control-Allow-Origin`
is granted. If a maintainer ever adds a permissive CORS layer
(e.g. for a separate-port dev frontend), this middleware ensures
write CSRF still fails closed.

The check applies to all state-changing methods, not just PATCH,
so any future POST/PUT/DELETE inherits the protection automatically.

### TDD Sequence (Phase 3)

All tests live in a new file
`server/tests/api_docs_patch.rs` (so failures are isolated from
the existing read tests). Each test boots an `AppState` from
`common::seeded_cfg_with_tickets` and drives the router via
`tower::ServiceExt::oneshot`. Assertions on SSE use
`state.sse_hub.subscribe()` before issuing the PATCH.

1. **Step 3.1** — `patch_succeeds_with_correct_if_match_returns_204_and_new_etag`:
   - GET first to learn the current etag.
   - PATCH with `If-Match: "<etag>"`, body
     `{"patch":{"status":"in-progress"}}`.
   - Assert `204`, `ETag` header present and different from the
     `If-Match`, on-disk file has `status: in-progress`.
2. **Step 3.2** — `patch_broadcasts_doc_changed_with_new_etag`:
   subscribe to SSE before the call. After 204, assert
   `tokio::time::timeout(Duration::from_millis(100), rx.recv()).await`
   yields `Ok(Ok(SsePayload::DocChanged { doc_type: Tickets, path:
   "meta/tickets/0001-todo-fixture.md", etag: Some(new_etag) }))`.
   The timeout (rather than `try_recv`) avoids a flake when the
   broadcast is even slightly async, while still failing fast if
   the event never arrives. Assert `new_etag` equals the response
   `ETag` header — both come from the driver's `FileContent`.
3. **Step 3.3a** — `patch_with_stale_if_match_returns_412_when_indexer_not_refreshed`:
   pre-edit the file out-of-band via `tokio::fs::write` (do
   NOT call `refresh_one` — the index entry's etag is now
   stale relative to disk). Issue PATCH with the original
   index etag → `412`, body `{"currentEtag":"<actual disk
   etag>"}`, `ETag` header echoes the disk etag (not the
   index's stale view — the driver re-reads bytes from disk
   for the etag check). The on-disk file is unchanged.
3. **Step 3.3b** — `patch_with_stale_if_match_returns_412_when_indexer_refreshed`:
   same out-of-band edit, but also call
   `state.indexer.refresh_one(&abs)` so the index reflects the
   new etag. Issue PATCH with the *original* etag → still
   `412` with the new disk etag. Pinned scenario: a second
   client raced ahead and the index has caught up; the etag
   precondition still trips because the client's `If-Match`
   was based on a pre-edit read. The two steps prove that the
   driver's etag check is sourced from disk, not from the
   indexer cache, in both stale-cache and fresh-cache states.
4. **Step 3.4** — `patch_without_if_match_returns_428`: same
   setup as 3.3 but `If-Match` header omitted → **428**, body
   `{"error":"if-match-required"}`, **no** `currentEtag`, **no**
   `ETag` header. On-disk file unchanged.
4. **Step 3.4b** — `patch_with_unsupported_if_match_returns_400`:
   `If-Match: *` → 400 with message naming wildcard.
   `If-Match: W/"sha256-..."` → 400 with message naming weak
   etag. `If-Match: "a", "b"` → 400 with message naming etag
   list. Each variant has its own assertion.
5. **Step 3.5** — `patch_with_unknown_status_value_returns_400`:
   body `{"patch":{"status":"blocked"}}` → `400`. On-disk file
   unchanged. No SSE event.
6. **Step 3.6** — `patch_with_disallowed_field_returns_400`: body
   `{"patch":{"title":"foo"}}` → `400`. On-disk file unchanged.
7. **Step 3.7** — `patch_with_empty_patch_object_returns_400`:
   body `{"patch":{}}` → `400`.
8. **Step 3.8** — `patch_to_non_ticket_path_returns_400`:
   PATCH against `/api/docs/meta/plans/2026-04-18-foo.md/frontmatter`
   → `400`. The plan file is unchanged. (The seeded plan fixture
   exists in `seeded_cfg_with_tickets` via the underlying
   `seeded_cfg`.)
9. **Step 3.9** — `patch_to_missing_path_returns_404`: PATCH
   against `meta/tickets/9999-ghost.md/frontmatter` → `404`.
10. **Step 3.10** — `patch_with_path_escape_returns_403`: two
    sub-cases pin the layered defence.
    - **3.10a** `path_with_dotdot_segment_rejected_at_handler`:
      `PATCH /api/docs/meta/tickets/../plans/foo.md/frontmatter`
      → `403`. The per-segment validator catches `..` before
      canonicalisation.
    - **3.10b** `path_passing_handler_check_but_resolving_outside_writable_roots_rejected_at_driver`:
      construct a path that the per-segment validator accepts
      (e.g. a literal filename without `..` or `.`) but which
      canonicalises (via a symlink under `meta/tickets/`) to a
      sibling outside the writable root. → `400`
      (`PathNotWritable`). Skipped on Windows
      (`#[cfg(unix)]`). Confirms the canonicalize+writable_roots
      check is the actual security boundary, not the
      per-segment validator.
    - **3.10c** `legitimate_filename_with_dots_accepted`:
      `PATCH /api/docs/meta/tickets/0001..todo.md/frontmatter`
      with a real fixture file at that path → 404 if the
      fixture doesn't exist or 204 if it does, but **not**
      403. Pins that the per-segment validator no longer
      false-positives on filenames containing `..` substrings.
11. **Step 3.11** — `patch_url_without_frontmatter_suffix_returns_400`:
    PATCH `/api/docs/meta/tickets/0001-todo-fixture.md`
    (forgot `/frontmatter`) → `400` with a message naming
    the missing suffix. (Confirms our suffix-strip workaround
    rejects degenerate URLs cleanly.)
12. **Step 3.12** — `patch_with_invalid_json_body_returns_400`:
    body `not json at all` → axum's built-in JSON extractor
    returns 400. (Sanity test that `Json<PatchFrontmatterBody>`
    handles bad input gracefully.)
13. **Step 3.13** — `get_request_unaffected_by_patch_method_being_added`:
    same `/api/docs/.../meta/tickets/0001-todo-fixture.md`
    GET still works exactly as before (regression test for the
    method-router merge).
14. **Step 3.14** — `idempotent_patch_with_same_value_returns_204_with_unchanged_etag_and_no_broadcast`:
    target file already has `status: in-progress`; PATCH with
    `{"status":"in-progress"}` and the current etag returns
    `204` with the same etag (idempotent — patcher returns
    same bytes, driver short-circuits, etag is identical).
    On-disk file is byte-identical AND mtime is unchanged (no
    rename happened). Subscribe to SSE before the call and assert
    `tokio::time::timeout(Duration::from_millis(50), rx.recv()).await`
    returns `Err(Elapsed)` — no broadcast was emitted. The
    watcher cannot fire either because no filesystem event was
    produced.
14b. **Step 3.14b** — `patch_with_unknown_field_in_body_returns_400`:
    body `{"patch":{"status":"todo","title":"hijack"}}` → 400.
    `serde(deny_unknown_fields)` on `PatchFields` rejects mixed
    bodies before the handler sees them. Pins the smuggle-fields
    defence.
14c. **Step 3.14c** — `patch_from_disallowed_origin_returns_403`:
    issue PATCH with `Origin: https://evil.example` → 403.
    Issue same PATCH with no `Origin` header (curl-style) →
    succeeds. Issue with `Origin: http://127.0.0.1:<port>`
    matching the bound port → succeeds. Pins the CSRF
    defence-in-depth middleware.
15. **Step 3.15** — `patch_emits_exactly_one_doc_changed_event`:
    subscribe to SSE. Synthesise a watcher event for the
    canonical path *directly* (calling
    `on_path_changed_debounced` on the registered watcher
    handler, bypassing the real notifier and its debounce
    timer) immediately after the PATCH returns 204. Assert
    exactly one `DocChanged` event reached the subscriber: the
    handler-driven broadcast. The synthesised watcher event
    finds the path in `WriteCoordinator` and short-circuits.
    Avoids sleep-based timing; pins the dedup contract
    deterministically.
16. **Step 3.16** — `patch_dedup_works_when_watcher_event_path_is_non_canonical`
    (`#[cfg(unix)]`): set up the test fixture with a symlinked
    tickets directory (e.g. `tickets_link → tickets_real`).
    The handler/driver canonicalise to `tickets_real/...`; the
    watcher receives `tickets_link/...`. Issue a PATCH;
    synthesise a watcher event with the symlinked path. Assert
    the watcher's `should_suppress` lookup canonicalises the
    event path, finds the entry, and returns true — exactly one
    `DocChanged` event reaches the subscriber. Without
    canonicalisation in the watcher path, this test would fail
    with two events.
17. **Step 3.17** — `patch_does_not_register_self_write_on_idempotent`:
    after an idempotent PATCH (Step 3.14), assert
    `WriteCoordinator::should_suppress(canonical)` returns
    `false` — no entry was ever inserted because the driver
    short-circuited before invoking `on_committed`.

### Success Criteria

#### Automated

- [ ] `cargo test --test api_docs_patch` — all 23 tests pass
      (Steps 3.3a/b, 3.10a/b/c, 3.14b, 3.14c, 3.15, 3.16, 3.17 added).
- [ ] `cargo test --test api_docs` — Phase 7 GET tests still
      green (regression).
- [ ] `cargo test` (full suite) green.
- [ ] `cargo clippy --all-targets -- -D warnings` clean.

#### Manual

- [ ] Run the server (`cargo run --features dev-frontend`),
      open two tabs on the kanban URL, drag a card in one — the
      other reflects the change without manual refresh.
- [ ] Repeat after editing the file out-of-band to confirm the
      conflict path shows the expected 412 in DevTools network
      tab.

---

## Phase 4: Frontend mutation primitives + self-cause SSE filter

### Overview

Add a `patchTicketFrontmatter` HTTP helper, a `useMoveTicket`
mutation hook with optimistic update + rollback, and a
self-cause filter that suppresses redundant invalidations from
SSE when the inbound event was caused by the local mutation.

### Changes Required

#### 1. New helper — `src/api/fetch.ts`

```ts
export interface PatchResult {
  etag: string
}

export async function patchTicketFrontmatter(
  relPath: string,
  patch: { status: KanbanColumnKey },
  etag: string,
): Promise<PatchResult>
```

- URL: `/api/docs/${encodeRelPath(relPath)}/frontmatter`
  (`encodeRelPath` splits on `/` and `encodeURIComponent`s
  each segment, mirroring the existing `fetchDocContent` path
  encoding).
- Method: `PATCH`.
- Headers: `If-Match: "<etag>"`,
  `Content-Type: application/json`.
- Body: `JSON.stringify({ patch })`.
- On `204`: read `ETag` header, return `{ etag: parsed }`
  (strip surrounding quotes).
- On `412`: parse `{ currentEtag }` from body, throw
  `FetchError(412, message)` with a `cause` of `{ currentEtag }`
  on a subclass `ConflictError extends FetchError` so callers
  can distinguish.
- On `4xx/5xx`: throw `FetchError(status, message)` like every
  other helper.

#### 2. Self-cause module — `src/api/self-cause.ts` (new)

```ts
export interface SelfCauseRegistry {
  register(etag: string): void
  has(etag: string | undefined): boolean
  // testing seam: clear all entries
  reset(): void
}

export interface SelfCauseOptions {
  ttlMs?: number       // default 5_000
  maxEntries?: number  // default 256, FIFO eviction
  now?: () => number   // default () => Date.now() — injectable for tests
}

export function createSelfCauseRegistry(
  opts?: SelfCauseOptions,
): SelfCauseRegistry

// Default app-wide instance, wired into the main QueryClient
// provider tree alongside makeUseDocEvents.
export const defaultSelfCauseRegistry: SelfCauseRegistry = createSelfCauseRegistry()
```

Design changes from the original sketch:

- **Factory pattern, not module-scoped global.** Mirrors the
  existing `makeUseDocEvents(factory)` DI convention. Tests
  instantiate fresh registries; production wires
  `defaultSelfCauseRegistry` through React context (a small
  `SelfCauseProvider` alongside `QueryClientProvider`).
- **Inject `now()`.** Tests use `vi.useFakeTimers()` and pass a
  spy `now`, so TTL behaviour is testable without depending on
  the module's internal time source.
- **`has(etag)`, not `consumeSelfEtag(etag)`.** Membership
  check **does not consume**. Entries are evicted only by TTL
  expiry (or FIFO cap). This fixes the duplicate-broadcast
  problem: when the same write produces multiple SSE events
  (e.g. handler-side broadcast plus a watcher event that escaped
  suppression), `has()` returns `true` for both — every
  echo of our own write is suppressed for the TTL window.
  Etag is SHA-256 of full content; collision with a foreign edit
  inside 5s is effectively impossible, so non-consuming
  membership is safe.
- **FIFO size cap.** `maxEntries` (default 256) prevents
  unbounded growth if SSE drops while the user keeps mutating.
  Lazy prune on register: drop expired entries first; if still
  over cap, drop the oldest by insertion order.
- **Drop-all on SSE reconnect.** `useDocEvents` calls
  `registry.reset()` when the EventSource transitions from
  disconnected to connected — pending self-etags from the
  outage window may correspond to broadcasts the client missed,
  so refetching is the safe choice.

#### 3. Mutation hook — `src/api/use-move-ticket.ts` (new)

```ts
export interface MoveTicketVars {
  entry: IndexEntry
  toStatus: KanbanColumnKey
}
```

- `useMutation<PatchResult, FetchError, MoveTicketVars,
       { previous?: IndexEntry[] }>`:
  - `mutationFn`: calls `patchTicketFrontmatter`.
  - `onMutate({ entry, toStatus })`:
    1. `await qc.cancelQueries({ queryKey:
       queryKeys.docs('tickets') })`.
    2. Snapshot
       `previous = qc.getQueryData<IndexEntry[]>(...)`.
    3. `qc.setQueryData(queryKeys.docs('tickets'),
       (old) => old.map(e => e.relPath === entry.relPath
         ? { ...e, frontmatter: { ...e.frontmatter,
             status: toStatus }} : e))`.
    4. Return `{ previous }`.
  - `onSuccess(result)`: `registry.register(result.etag)`
    (registry comes via `useSelfCauseRegistry()` context hook,
    defaulting to `defaultSelfCauseRegistry`).
  - `onError(_err, _vars, ctx)`: roll back —
    `qc.setQueryData(queryKeys.docs('tickets'), ctx?.previous)`.
  - `onSettled()`: invalidate
    `queryKeys.docs('tickets')` so the next refetch picks up
    the server's authoritative entry (etag, mtime, etc).

#### 4. Self-cause guard + drag-suppress — `src/api/use-doc-events.ts`

`makeUseDocEvents` accepts an additional dependency: a
`SelfCauseRegistry` (defaults to `defaultSelfCauseRegistry`). In
`dispatchSseEvent`:

```ts
if (event.type === 'doc-changed' && registry.has(event.etag)) {
  return // local mutation already updated the cache; non-consuming
}
```

`has` does not consume, so duplicate broadcasts for the same
write are all suppressed.

**Drag-suppress invalidation.** A new module-level (per-instance)
ref `isDragging` is exposed via `setDragInProgress(boolean)`.
While `isDragging` is `true`, `dispatchSseEvent` queues incoming
`doc-changed` events into a `pendingInvalidations: Set<string>`
keyed by query key, *without* triggering the invalidation. When
`setDragInProgress(false)` is called, queued invalidations flush
in one `qc.invalidateQueries({ queryKey })` per unique key.

This prevents the React reconciler from remounting `useSortable`
items mid-gesture (which would abort dnd-kit's PointerSensor and
silently cancel the user's drag in any tab that received an
unrelated `doc-changed` while a drag was in progress).

**SSE reconnect handling.** When the EventSource transitions
from `disconnected → connected`, call `registry.reset()` and
flush any `pendingInvalidations` — broadcasts the client missed
during the outage may correspond to mutations whose etags we
registered but whose echoes will never arrive.

#### 5. Conflict-message helper — extend `errorMessageFor` in
`KanbanBoard.tsx` (referenced from Phase 5)

```ts
if (error instanceof ConflictError) {
  // The SSE listener will (or already did) refetch the tickets
  // query in response to the same write that caused the
  // conflict. Use future-tense copy because the invalidation
  // may not have settled when the user reads the message.
  return 'This ticket was changed elsewhere — the board will update shortly. Try the move again if it still applies.'
}
```

The 412 response body carries `currentEtag` — when the spec
expands to include `currentStatus` in the body (a v2 nicety, not
required here), the message can name the new status directly.

Bespoke messages for non-conflict failure modes:

```ts
if (error instanceof FetchError) {
  switch (error.status) {
    case 428:
      return 'Could not save: the request was missing required precondition data. This is a bug; please reload the page.'
    case 403:
      return 'This move was refused. The ticket may be outside the writable area.'
    case 400:
      return 'Could not save the move; the request was rejected. The board will refresh — try again.'
    case 500:
    case 502:
    case 503:
      return 'Could not save the move (server error). The board will refresh — try again.'
  }
}
return 'Could not save the move. The board will refresh — try again.'
```

This means the user never sees a raw `if-match-required` or
`patch URL must end with /frontmatter` string in the alert
region.

### TDD Sequence (Phase 4)

#### `patchTicketFrontmatter` — `src/api/fetch.test.ts`

1. **Step 4.1** — `sends_patch_with_if_match_and_json_body`:
   `vi.stubGlobal('fetch')`, mock `204` response with
   `ETag: "sha256-NEW"`. Assert
   `mockFetch` was called with the correct URL, method
   `PATCH`, `If-Match: "sha256-OLD"`, body
   `{"patch":{"status":"in-progress"}}`. Returned
   `{ etag: 'sha256-NEW' }`.
2. **Step 4.2** — `unwraps_quoted_etag_from_response`: response
   header `ETag` is `"sha256-NEW"` — returned `etag` is
   `sha256-NEW` (quotes stripped).
3. **Step 4.3** — `throws_conflict_error_on_412_with_current_etag`:
   mock `412` with body `{"currentEtag":"sha256-LATEST"}`.
   Assert thrown `ConflictError`, its `status === 412`, its
   `currentEtag === 'sha256-LATEST'`.
4. **Step 4.4** — `throws_fetch_error_on_other_4xx`: mock 400 →
   throws `FetchError` (not `ConflictError`).
5. **Step 4.5** — `encodes_rel_path_segments`: relPath
   `meta/tickets/0001 weird path.md` results in URL with
   each segment percent-encoded.

#### `self-cause` module — `src/api/self-cause.test.ts`

Each test constructs a fresh registry via
`createSelfCauseRegistry({ now: () => clock })` so time and
state are fully controlled — no shared module state, no global
fake-timer setup.

6. **Step 4.6** — `has_returns_true_for_registered_etag_repeatedly`:
   `r.register('sha256-X')`; `r.has('sha256-X')` → `true`;
   second call **also** → `true` (non-consuming — the same
   write may produce multiple SSE echoes).
7. **Step 4.7** — `has_returns_false_for_unknown_etag`:
   no register; `r.has('sha256-Y')` → `false`.
8. **Step 4.8** — `has_returns_false_for_undefined_etag`:
   `r.has(undefined)` → `false`.
9. **Step 4.9** — `expired_etag_is_no_longer_present`:
   create registry with `ttlMs: 5000` and a controllable
   `now`. Register at `clock=0`; advance to `clock=5001`;
   `r.has(...)` → `false`. Pruning is verified by registering
   a second etag after expiry and asserting the first is gone
   from the internal map (size === 1).
10. **Step 4.10** — `fifo_eviction_drops_oldest_when_over_cap`:
    create registry with `maxEntries: 3`. Register four etags
    A, B, C, D in order. `has('A')` → `false` (evicted);
    `has('B'..'D')` → `true`.
11. **Step 4.11** — `reset_drops_all_entries`: register a few
    etags; `r.reset()`; all `has(...)` → `false`.

#### `useMoveTicket` — `src/api/use-move-ticket.test.tsx`

Pattern: render a tiny test component with a `QueryClient`,
seed `queryKeys.docs('tickets')` with two entries, fire the
mutation, assert cache state at each lifecycle hook. No router
needed.

10. **Step 4.10** — `optimistically_updates_status_in_cache`:
    seed cache with `[{ ..., frontmatter: { status: 'todo' }
    }]`. Spy `patchTicketFrontmatter` returning a never-settling
    promise; trigger mutation; immediately assert cache has
    `status: 'in-progress'` for that entry.
11. **Step 4.11** — `rolls_back_on_error`: seed cache; spy
    rejects with `new ConflictError(412, '...', { currentEtag })`;
    trigger; await; assert cache reverted to original.
12. **Step 4.12** — `registers_self_etag_on_success`: spy
    resolves with `{ etag: 'sha256-NEW' }`; trigger; await;
    assert `consumeSelfEtag('sha256-NEW') === true`.
13. **Step 4.13** — `invalidates_tickets_query_on_settle`: same
    setup as 4.12 but spy on `qc.invalidateQueries`. After
    settle: called with `{ queryKey: queryKeys.docs('tickets') }`.
14. **Step 4.14** — `does_not_modify_other_entries_in_cache`:
    cache has two tickets; mutate ticket A; assert ticket B is
    `===` to its original reference (cache identity preserved
    for unrelated rows).

#### `useDocEvents` self-cause skip — extend `use-doc-events.test.ts`

15. **Step 4.15** — `skips_invalidation_when_etag_was_self_caused`:
    pass an injected `SelfCauseRegistry` to `makeUseDocEvents`;
    spy on `qc.invalidateQueries`. `registry.register('sha256-X')`;
    dispatch a `doc-changed` event with that etag; assert
    `invalidateQueries` was NOT called.
16. **Step 4.16** — `still_invalidates_for_unknown_etags`:
    dispatch a `doc-changed` event with a different etag;
    assert `invalidateQueries` IS called (regression).
17. **Step 4.17** — `suppresses_duplicate_self_caused_events`:
    register `'sha256-X'`; dispatch *two* `doc-changed` events
    with that etag in succession; assert `invalidateQueries`
    was NOT called either time. Pins the non-consuming
    contract.
18. **Step 4.18** — `queues_invalidation_during_drag_and_flushes_on_drop`:
    call `setDragInProgress(true)`. Dispatch a `doc-changed`
    for an unknown etag. Assert `invalidateQueries` was NOT
    called yet. Call `setDragInProgress(false)`. Assert
    `invalidateQueries` is now called once with the queued
    query key.
19. **Step 4.19** — `coalesces_multiple_invalidations_during_drag`:
    `setDragInProgress(true)`; dispatch three
    `doc-changed` events for the same `doc_type`. On
    `setDragInProgress(false)`, assert `invalidateQueries`
    is called exactly once for that key (deduped).
20. **Step 4.20** — `resets_registry_and_flushes_on_sse_reconnect`:
    `register('sha256-X')`; simulate the EventSource going
    `connected → disconnected → connected`. After reconnect
    assert `r.has('sha256-X') === false` (registry reset) AND
    any pending invalidations were flushed.

### Success Criteria

#### Automated

- [ ] `cd skills/visualisation/visualise/frontend &&
       npm test -- fetch` — all 4.1–4.5 pass plus existing.
- [ ] `npm test -- self-cause` — 4.6–4.9 pass.
- [ ] `npm test -- use-move-ticket` — 4.10–4.14 pass.
- [ ] `npm test -- use-doc-events` — 4.15–4.16 pass plus existing.
- [ ] `npm test` (whole suite) green.
- [ ] `npm run lint` clean.
- [ ] `npm run typecheck` clean (TS type-check).

#### Manual

- [ ] None for this phase — purely network/cache plumbing.

---

## Phase 5: Frontend dnd-kit wiring + UX

### Overview

Engage the dnd-kit wiring already mounted in Phase 7: enable
`useSortable` on cards, register column droppables so empty
columns accept drops, replace the `onDrag*` no-ops with a real
`onDragEnd` that calls `useMoveTicket`, and surface conflicts
through an `aria-live` region.

### Changes Required

#### 1. Make columns droppable — `src/routes/kanban/KanbanColumn.tsx`

```ts
const isOtherColumn = columnKey === OTHER_COLUMN_KEY
const { setNodeRef, isOver } = useDroppable({
  id: `column:${columnKey}`,
  disabled: isOtherColumn,  // Other column does not accept drops
})
```

Attach `setNodeRef` to the outer column container. The column
id `column:<key>` lets `onDragEnd` distinguish "dropped on
empty column" from "dropped on a card" (whose sortable id is
the relPath). The Other column is registered as a *disabled*
droppable so dnd-kit's collision detection skips it entirely —
the dragged card snaps back from the gutter without ever
showing a positive `isOver` affordance over Other.

When `isOver` is true on a non-Other column, apply a visual
dragOver outline (e.g. `border-2 border-dashed border-primary`)
so the user can see the column is a valid target. For empty
columns, also reveal the "Drop here" copy (replacing the
`aria-hidden="true"` "No tickets" placeholder when a drag is
in progress) — discoverability for the case where the user has
not yet learned that empty columns accept drops.

#### 2. Flip drag activation — `src/routes/kanban/TicketCard.tsx`

- Remove `const PHASE_7_DISABLED = true`.
- Pass `disabled: false` (default) — or simply omit `disabled`.
- Restore `aria-roledescription`: stop destructuring it out of
  `attributes`. Spread the full `attributes` plus the existing
  `listeners` onto the inner `<Link>` element.
- Update the existing test
  `does_not_announce_a_misleading_sortable_role_description_while_disabled`
  → renamed
  `announces_sortable_role_description_when_enabled`.
- Update the test
  `does_not_respond_to_drag_interaction_while_disabled` → renamed
  `responds_to_keyboard_drag_interaction` and asserts that a
  Space-press initiates dnd-kit's keyboard sensor (style
  attribute updates after Space + Arrow keys).

#### 3. Sensors — `src/routes/kanban/KanbanBoard.tsx`

```ts
const sensors = useSensors(
  useSensor(PointerSensor, {
    activationConstraint: { distance: 5 },
  }),
  useSensor(KeyboardSensor),
)
```

The `distance: 5` activation threshold prevents accidental drags
from quick clicks: a click that moves less than 5 pixels still
fires the underlying `<Link>` navigation. Without this, the
ergonomics break — the card is both a navigation target and a
drag handle, and small mouse jitter on click would otherwise
start a drag.

#### 4. Real onDragEnd — `src/routes/kanban/KanbanBoard.tsx`

```ts
type DropOutcome =
  | { kind: 'move'; toStatus: KanbanColumnKey }
  | { kind: 'no-op-same-column' }
  | { kind: 'no-op-other-rejected' }
  | { kind: 'no-op-unknown' }

function resolveDropOutcome(
  active: Active,
  over: Over | null,
  entriesByRelPath: Map<string, IndexEntry>,
): DropOutcome
```

Logic:

- If `over` is null, or `entriesByRelPath` does not contain
  `active.id`, return `no-op-unknown`.
- If `over.id` starts with `column:` → strip prefix; if the
  column is `OTHER_COLUMN_KEY` return `no-op-other-rejected`;
  if not a known status column return `no-op-unknown`;
  otherwise compare to the source card's current status: equal
  → `no-op-same-column`, else `move`.
- Else `over.id` is a card relPath. Verify the relPath is a
  known card (`entriesByRelPath.has(over.id)`); if not →
  `no-op-unknown`. Look up the target card's column. If Other
  → `no-op-other-rejected`. If equal to source card's column
  → `no-op-same-column`. Else `move`.

The discriminated outcome type (rather than a nullable column)
lets the caller distinguish *why* a no-op happened, which is
needed for the keyboard accessibility announcements below.

`onDragEnd` flow:

```ts
docEvents.setDragInProgress(false)  // resume queued invalidations
const outcome = resolveDropOutcome(active, over, entriesByRelPath)
const source = entriesByRelPath.get(active.id as string)
const cardId = active.id as string

switch (outcome.kind) {
  case 'move':
    move.mutate(
      { entry: source!, toStatus: outcome.toStatus },
      {
        onError: (err) => {
          setConflict(messageFor(err))
          // restore focus to the rolled-back card after React
          // reconciles cache reversion
          requestAnimationFrame(() =>
            document.querySelector<HTMLElement>(
              `[data-relpath="${CSS.escape(cardId)}"]`,
            )?.focus()
          )
        },
        onSuccess: () => setConflict(null),  // clear stale
      },
    )
    return
  case 'no-op-other-rejected':
    setAnnouncement('The Other column is read-only; drops are ignored.')
    return
  case 'no-op-same-column': {
    // Always populate the polite announcement so:
    //   - keyboard/screen-reader users (who have no visual
    //     spring-back) hear that the drop landed,
    //   - users with `prefers-reduced-motion: reduce` (whose
    //     dnd-kit snap-back is suppressed) get explicit
    //     feedback that the system saw the drop,
    //   - mouse users get a redundant but non-disruptive
    //     confirmation. role="status" + aria-live="polite"
    //     does not interrupt anything, so the cost is zero.
    setAnnouncement(`Card returned to ${columnLabel(outcome)}.`)
    return
  }
  case 'no-op-unknown':
    return
}
```

`onDragStart` calls `docEvents.setDragInProgress(true)` so SSE
invalidations from other tabs queue while the gesture is in
progress (preventing dnd-kit's sortable items from remounting
mid-drag).

#### 5. Conflict and announcement regions — `KanbanBoard.tsx`

Two distinct live regions, mounted near the existing error banner:

```tsx
{conflict && (
  <div role="alert" aria-atomic="true" className="conflict-banner">
    <span>{conflict}</span>
    <button
      type="button"
      aria-label="Dismiss conflict notice"
      onClick={() => setConflict(null)}
    >
      ×
    </button>
  </div>
)}
<div role="status" aria-live="polite">{announcement}</div>
```

- **`role="alert"`** for `conflict`: implies
  `aria-live="assertive"` and interrupts in-progress speech.
  Conflicts block the user's intended action and must be
  announced immediately.
- **`role="status"`** (polite) for `announcement`:
  non-blocking acknowledgements (no-op rejections, drop
  confirmations) — must not interrupt.

**Dismissal lifecycle**:

- Both regions are cleared on the next `onDragStart` so a fresh
  gesture starts with a clean slate.
- `conflict` is also cleared on a successful mutation.
- `conflict` auto-clears after **30 seconds** of inactivity via
  a `setTimeout` (longer than the announcement's 15s — see
  below — because alerts are higher-stakes copy that the user
  may want to reread). The visible × dismiss button gives
  keyboard users an explicit clear without starting a new
  drag.
- `announcement` auto-clears after **15 seconds** (raised from
  the initial 8s draft to avoid truncating slow screen-reader
  speech mid-utterance).

Both timers are `useRef`-backed so they reset cleanly on
component unmount and on each new message.

The `<TicketCard>` outer element gains `data-relpath={relPath}`
so the focus-restore selector can find it after rollback (a
test seam *and* a stable focus target — using the relPath as a
DOM hook is justified as the same identifier dnd-kit uses).
The selector uses `CSS.escape(relPath)` (standard DOM API), not
a non-existent `cssEscape` helper — relPaths contain `/` and
`.` which are CSS-special and must be escaped.

#### 6. CHANGELOG entry — `CHANGELOG.md`

Append under `## Unreleased > ### Added`:
`Kanban drag-and-drop now writes ticket status to disk via
PATCH /api/docs/{path}/frontmatter; conflicts roll the card
back and announce via aria-live; concurrent edits are detected
via If-Match optimistic concurrency.`

Append under `## Unreleased > ### Notes`:
`Tickets are written in place. If a write produces unexpected
output, recover with `git checkout meta/tickets/<file>`.`

### TDD Sequence (Phase 5)

#### KanbanColumn droppable — `src/routes/kanban/KanbanColumn.test.tsx`

Tests render the column wrapped in a real `DndContext` (no
spying on dnd-kit internals; no test-only DOM attributes). The
behavioural surface is exercised via dnd-kit's public APIs:
`useDndContext().droppableContainers` exposes the registered
droppables, which the test can introspect through a tiny
`<DroppableProbe />` component that consumes the context and
renders the count or ids as data — used only inside the test
file, not in production markup.

1. **Step 5.1** — `non_other_column_is_registered_as_droppable_with_column_key_id`:
   render a `todo` column inside a probed `DndContext`. Assert
   the probe sees a registered droppable with id
   `column:todo`.
2. **Step 5.2** — `empty_non_other_column_is_still_droppable`:
   render with `entries=[]`. The droppable is still registered.
3. **Step 5.3** — `other_column_is_not_droppable`: render the
   Other column. The probe sees the droppable registered with
   `disabled: true` (or absent from `droppableContainers`,
   depending on dnd-kit's behaviour with `disabled`). Pin which
   it is and assert.
4. **Step 5.4** — `column_shows_drop_outline_when_isOver`:
   render with a fake `DndMonitor` event putting the column
   into the over state; assert the column container has the
   dragOver outline class.

#### TicketCard flip — update `TicketCard.test.tsx`

5. **Step 5.5** — refactor existing test
   `does_not_announce_a_misleading_sortable_role_description_while_disabled`
   → new positive test
   `announces_sortable_role_description_when_enabled`:
   render the card; assert
   `link.getAttribute('aria-roledescription') === 'sortable'`.
6. **Step 5.6** — `card_carries_data_relpath_for_focus_restore`:
   render the card; assert the outer element has
   `data-relpath` equal to the entry's relPath. Pins the
   focus-restore selector contract used by the rollback path.
7. **Step 5.7** — `quick_click_navigates_without_starting_drag`:
   render the card inside the board's `DndContext` configured
   with `PointerSensor({ distance: 5 })`. Simulate a synthetic
   pointerdown / pointerup that travels < 5 px. Assert the
   `<Link>`'s default navigation is *not* prevented (no
   `data-dragging` attribute appeared; the click handler ran).
   This pins the activation-distance contract; without it,
   click-to-navigate is unreliable.
8. **Step 5.8** — `keyboard_drag_initiated_by_space_then_arrow`:
   focus the card, press Space (KeyboardSensor activator), then
   arrow Right. Assert the dnd-kit `aria-describedby` /
   `data-dragging` state machine reflects an in-progress drag.
   **No fallback assertion path** — if jsdom + KeyboardSensor
   proves unreliable in CI, this is moved to the manual smoke
   checklist and the test is removed; the hybrid "test the
   attributes if drag flakes" approach is rejected because it
   reduces to a vacuous attribute-presence check.

#### resolveDropOutcome pure — `src/routes/kanban/resolve-drop-outcome.ts` + test

9. **Step 5.9** — extract `resolveDropOutcome` into its own
   module (pure function, no React). Tests:
   - `column_id_returns_move_outcome`: `over.id =
     'column:done'`, source in `todo` → `{kind:'move',toStatus:'done'}`.
   - `card_id_returns_target_cards_column_as_move`: dropped on
     a card in `done` → `move` to `done`.
   - `other_column_id_returns_no_op_other_rejected`.
   - `card_id_in_other_returns_no_op_other_rejected`.
   - `same_column_returns_no_op_same_column`.
   - `card_on_card_in_same_column_returns_no_op_same_column`.
   - `unknown_column_id_returns_no_op_unknown`.
   - `unknown_card_id_returns_no_op_unknown`.
   - `null_over_returns_no_op_unknown`.
   - `unknown_active_returns_no_op_unknown`.

#### KanbanBoard mutation wiring — extend `KanbanBoard.test.tsx`

These tests render the **whole `KanbanBoard`** inside a real
`DndContext`-bearing `RouterProvider` and drive interactions via
testing-library and dnd-kit's `KeyboardSensor` (Space + Arrow +
Space). The previous plan offered a "test the hook directly
seam"; that is rejected — we test the user-visible behaviour, and
when the keyboard-sensor path is unreliable in jsdom we move to
manual smoke instead of papering over with attribute assertions.

Each test seeds `qc.setQueryData(queryKeys.docs('tickets'), [...])`
with the relevant fixture cards. `patchTicketFrontmatter` is
spied via `vi.spyOn(fetchModule, 'patchTicketFrontmatter')`.

10. **Step 5.10** — `successful_drag_to_other_column_optimistically_moves_card_then_PATCHes`:
    render with one todo card. Drive a keyboard drag from todo
    to done. Assert (a) the card appears under done in the DOM
    *before* the spy resolves (optimistic), (b) the spy was
    called with the correct relPath and `toStatus: 'done'`,
    (c) after the spy resolves, the card stays in done.
11. **Step 5.11** — `conflict_rolls_card_back_and_announces_via_role_alert_and_restores_focus`:
    spy rejects with `ConflictError(412, ..., { currentEtag })`.
    Drag from todo to done. Assert: card returns to todo column;
    `role="alert"` element contains the conflict copy
    (specifically *not* the old "refresh and try again" text);
    after `requestAnimationFrame` settles, the rolled-back card
    has document focus.
12. **Step 5.12** — `drop_onto_other_column_announces_read_only_via_role_status_no_PATCH`:
    drag a todo card onto Other. Assert spy NOT called;
    `role="status"` element contains the read-only message.
13. **Step 5.13** — `drop_onto_same_column_announces_no_op_status_for_both_keyboard_and_pointer`:
    keyboard-drag a todo card and drop on the same column.
    Assert spy NOT called; `role="status"` element contains
    the "Card returned to {column}." copy. Repeat with a
    pointer-initiated drag → same assertions: spy NOT called
    AND announcement IS populated. (Always-announce policy
    decided for accessibility — see onDragEnd comment.)
14. **Step 5.14** — `drop_onto_other_card_in_same_column_no_PATCH`:
    drop a todo card onto another todo card → no PATCH (v1
    has no intra-column reorder).
15. **Step 5.15** — `live_regions_clear_on_next_drag_start`:
    after a conflict populates `role="alert"`, start a new
    drag — assert the alert region is empty.
16. **Step 5.16** — `announcement_auto_clears_after_15_seconds`:
    use `vi.useFakeTimers()`; trigger a no-op announcement;
    advance 15000ms; assert the region is empty.
16b. **Step 5.16b** — `conflict_auto_clears_after_30_seconds`:
    same setup; trigger a conflict; advance 30000ms; assert
    the alert region is empty.
16c. **Step 5.16c** — `conflict_dismissable_via_close_button`:
    trigger a conflict; click the dismiss × button; assert the
    alert region is empty without advancing timers.
17. **Step 5.17** — `dragstart_calls_setDragInProgress_true_dragend_calls_false`:
    spy on the injected `useDocEvents` instance's
    `setDragInProgress`. Start and end a drag. Assert the spy
    was called with `true` then `false`.
18. **Step 5.18** — `incoming_sse_during_drag_does_not_remount_sortable`:
    start a drag; while in progress, dispatch an unrelated
    `doc-changed` SSE event for a different ticket. Assert
    `qc.invalidateQueries` was NOT called yet (queued); the
    sortable items in flight retain their identity (no remount
    — assert via `key`-stable refs or by spying on
    `useSortable` cleanup). After the drag ends, the queued
    invalidation flushes.
19. **Step 5.19** — `pointer_quick_click_navigates_does_not_drag`:
    pointerdown + pointerup with < 5px movement → navigation
    fired (assert spy on router navigate or on the link's
    default click), no dnd-kit dragging state attached.

#### Cross-tab smoke (manual; covered server-side in Phase 3.2)

No new automated test — Phase 3.2's SSE assertion plus
self-cause-filter test in 4.15 already pin the cross-tab
behaviour.

### Success Criteria

#### Automated

- [ ] `cd skills/visualisation/visualise/frontend &&
       npm test -- KanbanColumn` — 5.1–5.4 pass plus existing.
- [ ] `npm test -- TicketCard` — 5.5–5.8 pass; old
      Phase-7-disabled tests removed.
- [ ] `npm test -- resolve-drop-outcome` — 10 sub-tests pass.
- [ ] `npm test -- KanbanBoard` — 5.10–5.19 pass plus existing.
- [ ] `npm test` whole suite green.
- [ ] `npm run lint` clean.
- [ ] `npm run typecheck` clean.
- [ ] `npm run build` produces `dist/` (smoke that nothing
      regressed at the bundling layer).

#### Manual

- [ ] Start the server (`cargo run --features dev-frontend`)
      with the frontend in `--watch` build. Open the kanban
      URL in two browser windows.
- [ ] Drag a `todo` card to `done` in window A. Window A's
      card animates and settles in `done`. Window B's card
      moves silently.
- [ ] Reload window A. The card stayed in `done`.
- [ ] Hand-edit `meta/tickets/0001-todo-fixture.md` to
      `status: done`. In window A (still showing `todo`),
      drag it to `in-progress`. The card snaps back; a
      polite announcement explains the conflict; the on-disk
      file is unchanged.
- [ ] Drag a card sitting in the "Other" swimlane (the
      `0003-other-fixture.md` ticket whose status is
      `blocked`) → no PATCH attempted; card stays put.
- [ ] Tab through cards with keyboard; press Space, arrow,
      Space — the keyboard sensor moves the card; status
      updates on disk.

---

## Testing Strategy

### Rust unit tests

Inline `#[cfg(test)] mod tests` modules per the project's
existing convention (see `frontmatter.rs:215-272` and
`sse_hub.rs:42-90`). 12 patcher tests, 10
`LocalFileDriver::write_frontmatter` tests, 5 `Indexer::refresh_one`
tests.

### Rust integration tests

`server/tests/api_docs_patch.rs` — 14 tests using
`tower::ServiceExt::oneshot` against the full router built from
`AppState::build` + `seeded_cfg_with_tickets`. Pattern mirrors
`server/tests/api_docs.rs`. Each test that asserts SSE behaviour
subscribes via `state.sse_hub.subscribe()` before issuing the
request.

### Frontend unit tests

vitest + `@testing-library/react`. New test files:
`fetch.test.ts` (extended), `self-cause.test.ts`,
`use-move-ticket.test.tsx`, `use-doc-events.test.ts`
(extended), `KanbanColumn.test.tsx` (extended),
`TicketCard.test.tsx` (refactored), `resolve-target.test.ts`,
`KanbanBoard.test.tsx` (extended). All use the existing
`vi.stubGlobal('fetch')` + `vi.spyOn(fetchModule, ...)`
patterns; no MSW.

### Manual smoke checklist

Documented under each phase's "Manual Verification" section.
The cross-tab smoke + conflict path are the two scenarios that
cannot be cheaply automated in jsdom and live as manual checks.

### Fixtures

Reuse `seeded_cfg_with_tickets` from
`server/tests/common/mod.rs`. No new fixtures needed — the
three existing ticket files cover todo / done / other
swimlane.

## References

- Original ticket: none (driven by spec).
- Spec: `meta/specs/2026-04-17-meta-visualisation-design.md`
  — § "Writes and conflict handling", § "Kanban".
- Research: `meta/research/2026-04-17-meta-visualiser-implementation-context.md`
  — Phase 8 outline at line 1082.
- Phase 7 plan (for TDD step style and read-only kanban
  context):
  `meta/plans/2026-04-26-meta-visualiser-phase-7-kanban-read-only.md`.
- ETag round-trip pattern: `server/src/api/docs.rs:42-95`.
- SSE broadcast pattern: `server/src/watcher.rs:97-141`.
- Atomic write idiom: `tempfile::NamedTempFile::persist`.
- matchit catch-all-then-static limitation:
  https://github.com/ibraheemdev/matchit/issues/39 — drives
  the in-handler suffix-strip workaround in Phase 3.

## Implementation sequence

Tick each step as it lands. A step is done when its named test
file passes and `cargo clippy` / `npm run lint` are clean.

- [ ] Step 1.1 — patcher: simple unquoted replace
- [ ] Step 1.2 — patcher: preserves other keys + order
- [ ] Step 1.3 — patcher: preserves quoted values
- [ ] Step 1.4 — patcher: preserves inline comment
- [ ] Step 1.5 — patcher: preserves body byte-for-byte
- [ ] Step 1.6 — patcher: preserves CRLF
- [ ] Step 1.7 — patcher: idempotent on same value
- [ ] Step 1.8 — patcher: accepts each TicketStatus variant
- [ ] Step 1.9 — patcher: rejects when key missing
- [ ] Step 1.10 — patcher: rejects when fm absent
- [ ] Step 1.11 — patcher: rejects when fm malformed
- [ ] Step 1.12 — patcher: ignores nested `status:`
- [ ] Step 1.13 — patcher: doesn't close fm at body-internal `---`
- [ ] Step 1.14 — patcher: rejects block-scalar value
- [ ] Step 1.15 — patcher: preserves single-quoted value
- [ ] Step 1.16 — patcher: rejects anchored value
- [ ] Step 1.17 — patcher: handles flow-style mapping (pin which error)
- [ ] Step 1.18 — patcher: preserves line-specific line ending
- [ ] frontmatter::fence_offsets helper extracted; parse refactored to use it
- [ ] Step 2.1 — driver: writes status atomically
- [ ] Step 2.2 — driver: returns new etag/mtime
- [ ] Step 2.3 — driver: preserves rest of file
- [ ] Step 2.4 — driver: rejects etag mismatch
- [ ] Step 2.5 — driver: rejects non-tickets path
- [ ] Step 2.6 — driver: rejects symlink escape
- [ ] Step 2.7 — driver: propagates ValueNotAllowed
- [ ] Step 2.8 — driver: propagates FrontmatterAbsent
- [ ] Step 2.9 — driver: concurrent matching writes — one EtagMismatch
- [ ] Step 2.10 — driver: tolerates quoted If-Match
- [ ] Step 2.11 — driver: idempotent same-value short-circuits (no rename)
- [ ] Step 2.12 — driver: preserves Unix file permissions (0o644)
- [ ] Step 2.13 — driver: cross-filesystem persist returns CrossFilesystem (linux, ignored)
- [ ] Step 2.14 — indexer.refresh_one: picks up new entry
- [ ] Step 2.15 — indexer.refresh_one: updates etag
- [ ] Step 2.16 — indexer.refresh_one: removes deleted
- [ ] Step 2.17 — indexer.refresh_one: leaves others alone
- [ ] Step 2.18 — indexer.refresh_one: rebuilds id maps
- [ ] Step 2.19 — indexer.refresh_one: serialises with watcher's rescan_lock
- [ ] Step 3.1 — PATCH 204 + new etag
- [ ] Step 3.2 — PATCH broadcasts doc-changed
- [ ] Step 3.3a — PATCH 412 stale If-Match (indexer not refreshed)
- [ ] Step 3.3b — PATCH 412 stale If-Match (indexer refreshed)
- [ ] Step 3.4 — PATCH 428 missing If-Match (no currentEtag)
- [ ] Step 3.4b — PATCH 400 unsupported If-Match (weak/list/wildcard)
- [ ] Step 3.5 — PATCH 400 unknown status value
- [ ] Step 3.6 — PATCH 400 disallowed field
- [ ] Step 3.7 — PATCH 400 empty patch object
- [ ] Step 3.8 — PATCH 400 non-ticket path
- [ ] Step 3.9 — PATCH 404 missing path
- [ ] Step 3.10a — PATCH 403 path escape (dotdot at handler)
- [ ] Step 3.10b — PATCH 400 path resolves outside writable_roots (driver)
- [ ] Step 3.10c — PATCH does NOT false-reject filenames containing `..`
- [ ] Step 3.11 — PATCH 400 missing /frontmatter suffix
- [ ] Step 3.12 — PATCH 400 invalid JSON
- [ ] Step 3.13 — GET regression after PATCH route added
- [ ] Step 3.14 — PATCH idempotent: 204, unchanged etag, unchanged mtime, no SSE
- [ ] Step 3.14b — PATCH 400 unknown field in body (deny_unknown_fields)
- [ ] Step 3.14c — PATCH Origin pin (3 sub-cases: foreign 403, no-origin OK, same-origin OK)
- [ ] Step 3.15 — PATCH emits exactly one doc-changed event (synthesised watcher event, deterministic)
- [ ] Step 3.16 — PATCH dedup with non-canonical watcher path (symlinked tickets dir, unix-only)
- [ ] Step 3.17 — Idempotent PATCH does NOT register self-write
- [ ] Step 4.1 — fetch: PATCH wire format
- [ ] Step 4.2 — fetch: unwraps quoted etag
- [ ] Step 4.3 — fetch: ConflictError on 412
- [ ] Step 4.4 — fetch: FetchError on other 4xx
- [ ] Step 4.5 — fetch: encodes path segments
- [ ] Step 4.6 — self-cause: has true after register (non-consuming)
- [ ] Step 4.7 — self-cause: false for unknown
- [ ] Step 4.8 — self-cause: false for undefined
- [ ] Step 4.9 — self-cause: TTL expiry with injected now
- [ ] Step 4.10 — self-cause: FIFO eviction at maxEntries
- [ ] Step 4.11 — self-cause: reset drops all
- [ ] Step 4.12 — useMoveTicket: optimistic update
- [ ] Step 4.13 — useMoveTicket: rollback on error
- [ ] Step 4.14 — useMoveTicket: registers self etag via registry
- [ ] Step 4.15 — useDocEvents: skip on self etag
- [ ] Step 4.16 — useDocEvents: invalidate other etag
- [ ] Step 4.17 — useDocEvents: suppresses duplicate self-caused events
- [ ] Step 4.18 — useDocEvents: queues invalidation during drag
- [ ] Step 4.19 — useDocEvents: coalesces invalidations during drag
- [ ] Step 4.20 — useDocEvents: resets registry on SSE reconnect
- [ ] Step 5.1 — KanbanColumn: non-Other registered as droppable
- [ ] Step 5.2 — KanbanColumn: empty non-Other still droppable
- [ ] Step 5.3 — KanbanColumn: Other column not droppable (disabled)
- [ ] Step 5.4 — KanbanColumn: shows drop outline when isOver
- [ ] Step 5.5 — TicketCard: aria-roledescription enabled
- [ ] Step 5.6 — TicketCard: data-relpath for focus restore
- [ ] Step 5.7 — TicketCard: quick click navigates (5px threshold)
- [ ] Step 5.8 — TicketCard: keyboard drag (Space + Arrow)
- [ ] Step 5.9 — resolveDropOutcome: 10 sub-tests
- [ ] Step 5.10 — KanbanBoard: optimistic move + PATCH
- [ ] Step 5.11 — KanbanBoard: conflict rollback + role=alert + focus restore
- [ ] Step 5.12 — KanbanBoard: drop on Other → role=status + no PATCH
- [ ] Step 5.13 — KanbanBoard: same-col keyboard announces; mouse silent
- [ ] Step 5.14 — KanbanBoard: card-on-card same col → no PATCH
- [ ] Step 5.15 — KanbanBoard: live regions clear on next dragstart
- [ ] Step 5.16 — KanbanBoard: announcement auto-clears after 15s
- [ ] Step 5.16b — KanbanBoard: conflict auto-clears after 30s
- [ ] Step 5.16c — KanbanBoard: conflict dismissable via × button
- [ ] Step 5.17 — KanbanBoard: dragstart/dragend toggle setDragInProgress
- [ ] Step 5.18 — KanbanBoard: SSE during drag is queued, no remount
- [ ] Step 5.19 — KanbanBoard: pointer quick click navigates, no drag
- [ ] CHANGELOG entry added under `## Unreleased > ### Added`
- [ ] CHANGELOG note: tickets are written in place; recover via `git checkout`
- [ ] WriteCoordinator module landed (mark_self_write/should_suppress/unmark, FIFO cap, injectable now)
- [ ] Watcher canonicalises event path before consulting WriteCoordinator
- [ ] Driver `on_committed` callback invoked while holding per-path mutex (closes race)
- [ ] Origin-pin middleware on PATCH/POST/PUT/DELETE
- [ ] `serde(deny_unknown_fields)` on PatchFrontmatterBody + PatchFields
- [ ] TicketStatus enum lands in shared types module
- [ ] FrontmatterPatch moved to domain types module (not file_driver)
- [ ] FileDriver `writable_roots` config replaces hard-coded tickets-root
- [ ] Suffix-strip doc-comments on doc_fetch + doc_patch_frontmatter + route registration
- [ ] Manual cross-tab smoke completed
- [ ] Manual conflict-path smoke completed (verify message text matches new copy)
- [ ] Manual keyboard-sensor smoke completed (Space + Arrow + Space)
- [ ] Manual permissions smoke: ticket file mode preserved across PATCH
- [ ] Manual recovery smoke: corrupt a ticket, recover via git checkout
