---
date: "2026-05-18T22:30:00+01:00"
type: plan
skill: create-plan
work_item_id: "0042"
status: accepted
---

# 0042 Templates View Redesign — Implementation Plan

## Overview

Redesign the templates view to surface per-tier presence on the index
and expose the resolved winning-tier content hash on the detail screen,
live-updated via SSE. The change spans a small backend addition (one
JSON field, one SSE variant, watcher plumbing for template-tier paths)
and a focused frontend redesign (tristate tier-presence row on the
index; two-column layout on the detail screen with a non-interactive
content-hash label in a new preview pane).

The infrastructure this work depends on (SSE hub, `Option<String>`
content-hash pattern, Chip palette, Page wrapper, MarkdownRenderer,
`TemplateResolver`, three-tier resolution model) is all already in
place; this plan is a focused composition over established primitives
plus a single backend addition for live-updateable content hashes.

## Current State Analysis

- The templates index
  (`skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx:32-41`)
  shows a single `<Chip variant="neutral">` per row with the active
  tier's friendly label and ignores per-tier presence — `TemplateSummary.tiers`
  carries the data but it's unused.
- The templates detail screen
  (`skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx:47-55,62-78`)
  renders a single-column stack of tier panels with an indigo "active"
  Chip on the winning tier and a neutral "absent" Chip on missing
  tiers. No content-hash, no two-column layout, no preview pane.
- The backend `TemplateDetail`
  (`skills/visualisation/visualise/server/src/templates.rs:38-44`)
  exposes per-tier `etag` (in `sha256-<hex>` form), but has no
  top-level `sha256` field; clients have no way to display a hash for
  the resolved winning-tier content without re-deriving it.
- `TemplateResolver` is built once at startup and stored as
  `Arc<TemplateResolver>` in `AppState`
  (`skills/visualisation/visualise/server/src/server.rs:79-81`); there
  is no live-rebuild path when a tier file changes.
- The filesystem watcher (`skills/visualisation/visualise/server/src/watcher.rs:28-98`)
  only watches `cfg.doc_paths`
  (`skills/visualisation/visualise/server/src/server.rs:286`); template
  tier paths sit in `cfg.templates`, exposed to the file driver via
  `template_extra_roots`
  (`skills/visualisation/visualise/server/src/file_driver.rs:486-504`)
  for path whitelisting, but never passed to the watcher. Templates
  are also filtered out of the indexer
  (`skills/visualisation/visualise/server/src/indexer.rs:272`), so the
  existing watcher branch wouldn't fire for them anyway.
- The SSE hub (`skills/visualisation/visualise/server/src/sse_hub.rs:15-32`)
  only carries `DocChanged` and `DocInvalid` variants. The wire format
  is JSON-discriminated (`#[serde(tag = "type", rename_all =
  "kebab-case")]`) so a new variant serialises to its kebab-case name
  automatically.
- The frontend SSE dispatch reducer
  (`skills/visualisation/visualise/frontend/src/api/use-doc-events.ts:80-111`)
  branches on `doc-changed | doc-invalid`; the `SseEvent` union
  (`skills/visualisation/visualise/frontend/src/api/types.ts:115-141`)
  is closed at those two variants.

## Desired End State

A user navigating `/library/templates`:

- Sees each row with three inline Chip indicators (in the fixed order
  `plugin-default` → `user-override` → `config-override`, left-to-right
  with short labels "default" / "user" / "config") whose variants
  encode `(present, active)` as `neutral` / `indigo` / `green`.

A user navigating `/library/templates/{name}` at ≥1024px viewport:

- Sees a two-column layout with stacked tier cards on the left and a
  preview pane on the right.
- The winning tier card carries both an accent-coloured outline ring
  (new) and an indigo "active" Chip (kept from the current
  implementation).
- The preview pane's first row shows the winning tier's path (muted
  monospace, left) and the content-hash label `sha256-<64-hex>`
  (monospace, right), both non-interactive; the rendered template
  markdown body follows immediately below.
- Editing the winning-tier file on disk produces a visible update to
  the content-hash label within 1 second, with no page reload.

Backend HTTP surface:

- `GET /api/templates/:name` (this is the existing mount point;
  the work item's `/api/library/templates/{name}` phrasing is a
  labelling error — the rest of the resource-level API mounts at
  `/api/<resource>` and `/api/library/*` is reserved for the
  structure/aggregation index) returns a `sha256` top-level string
  field of the form `sha256-<64-hex>` (matching the existing per-tier
  `etag` shape), omitted entirely when the resolved winning-tier
  content is `None` or empty.
- `/api/events` emits a new `template-changed` event variant of shape
  `{ type, template, sha256, timestamp }` whenever a tier file
  underlying a configured template changes on disk. `sha256` is
  `Option<String>` on the wire (omitted when the new winning content
  is `None`/empty) so the empty-content transition still produces an
  event and the client can invalidate accordingly.

> **AC8 wording clarification**: AC8 in the work item says "value is
> the 64-character lowercase hex SHA-256 digest". The plan adopts the
> existing project-wide `sha256-<hex>` etag shape on the wire to avoid
> introducing a second hash-encoding convention. The 64-char hex
> digest is preserved verbatim inside the prefix. AC8's literal
> wording will need a small editorial clarification ("value of the
> form `sha256-<hex>`") in a follow-up; the semantic content is
> unchanged.

### Verification

- `cargo test -p accelerator-visualiser` is green, with new
  integration tests in `server/tests/api_templates.rs` and
  `server/tests/sse_e2e.rs`.
- `npm test --prefix skills/visualisation/visualise/frontend` is
  green, with new tests in `LibraryTemplatesIndex.test.tsx`,
  `LibraryTemplatesView.test.tsx`, and the dispatch-reducer test
  file.
- `npm run typecheck --prefix skills/visualisation/visualise/frontend`
  passes.
- Manual: launch the visualiser, edit a template tier file, observe
  the hash update in real time on the detail screen at ≥1024px.

### Key Discoveries:

- Per-tier `etag` is already populated in `sha256-<hex>` form via
  `FileContent.etag` →
  `templates.rs:158-172` →
  `file_driver::etag_of` (`file_driver.rs:480-484`). The new
  top-level `sha256` field adopts the **same** `sha256-<hex>` shape
  so that every content-hash surface in the response carries one
  consistent encoding. The UI renders the field verbatim — no
  prefix-prepend logic in the frontend.
- `SsePayload`'s `#[serde(tag = "type", rename_all = "kebab-case")]`
  means a new Rust variant `TemplateChanged` serialises to
  `"type":"template-changed"` without any manual wiring. A
  per-variant `#[serde(rename_all = "camelCase")]` is applied
  defensively so future multi-word fields keep the wire convention.
- `TemplateSummary.tiers` (already present, with
  `source/present/active`) supplies everything the new index row
  needs; no index endpoint change is required.
- Frontend SSE uses TanStack Query invalidation, not manual cache
  mutation. The reducer is pure
  (`use-doc-events.ts:80-111`) and exported for direct testing.
- `arc-swap` is **not** currently a server dependency — Phase 2 adds it.
- The watcher today canonicalises **event** paths (`watcher.rs:114-121`)
  but never canonicalises **config-held** paths. On macOS
  (`/var` vs `/private/var`), under symlinks, or with relative vs
  absolute paths, a naïve string compare between the two silently
  misses. The plan canonicalises `cfg.templates` tier paths once at
  startup (with the same `canonicalize.unwrap_or(original)` fallback
  the watcher already uses) and builds a precomputed
  `HashMap<PathBuf, Vec<String>>` lookup so multiple templates that
  share a tier file each receive an event.
- `notify::RecursiveMode::NonRecursive` is the current setting; the
  plan switches template-tier watches to `Recursive` (relying on the
  `is_markdown` filter + canonical-path index for scoping) so
  editor atomic-rename patterns and nested tier layouts don't drop
  events.
- The existing tests in
  `LibraryTemplatesIndex.test.tsx:71-73` and
  `LibraryTemplatesView.test.tsx:84-86` assert that legacy CSS classes
  (`.active`, `.activeBadge`) are absent — these need to be preserved
  or updated as our CSS changes.

## What We're NOT Doing

- Not adding responsive collapse behaviour for the detail screen
  below 1024px viewports (explicitly out of scope per work item).
- Not modifying the `/api/templates` index endpoint shape — the
  existing `tiers: TemplateTier[]` payload already carries
  presence/active flags. The new `sha256` field is detail-only.
- Not changing tier resolution semantics
  (`TemplateResolver::build` at `templates.rs:51-107`). Order remains
  `ConfigOverride → UserOverride → PluginDefault`; first-present-wins.
- Not adding a `templates` glyph icon (intentionally absent from
  `Glyph.constants.ts:19`; the work item doesn't require icons).
- Not adding a copy-to-clipboard or tooltip affordance to the
  content-hash label (explicitly rejected; non-interactive by AC13).
- Not extracting a shared `TierPresenceRow` component — inline in the
  index route file unless the rendering becomes unreadable.
- Not adding self-cause filtering for `template-changed` events;
  there is no frontend write path for templates, so no self-cause
  collision is possible.
- Not changing the per-tier `etag` field (it stays `sha256-<hex>`-prefixed
  to preserve the existing contract).
- Not adding a per-name `TemplateResolver::refresh(name)` API in this
  plan. The watcher rebuilds the whole resolver on each tier-file
  change. At current scale (~10 templates × 3 tier reads × O(KB)) this
  is cheap; a per-name refresh is the natural future evolution lever
  if the template catalogue grows.
- Not slimming the templates detail response payload to active-tier-
  content-only. The current shape (`content` on all three tiers)
  stays; revisit if per-tier content sizes grow materially.
- Not adding hot-reload of `cfg.templates`. The watcher captures the
  config snapshot at startup; future config hot-reload would need
  the canonical-path index to be rebuilt and pushed into the change
  handler.

## Implementation Approach

Six phases, TDD-shaped (failing test first, implementation second,
verification third), in dependency order: backend foundation → SSE
plumbing → frontend types/dispatch → frontend UI redesign.

Backend changes touch four files (`templates.rs`, `sse_hub.rs`,
`watcher.rs` — with a new `TemplateChangeHandler` / `TierPathIndex`
alongside the existing loop — and `server.rs` for AppState +
watch-dir wiring), plus tests. Frontend changes touch `types.ts`,
`use-doc-events.ts`, the two route components, a new shared
`template-tier.ts` module, two CSS modules, and three test files.

---

## Phase 1: Backend — `TemplateDetail.sha256` field

### Overview

Add a top-level `sha256: Option<String>` field on `TemplateDetail`
holding the resolved winning-tier content hash in the same
`sha256-<64-hex>` shape used by the existing per-tier `etag`. The
digest is **cached on the resolver entry** at `build()` time so
`detail()` reads it in O(1) — no per-request hashing.

### Changes Required:

#### 1. Templates response type, cached digest, and resolver

**File**: `skills/visualisation/visualise/server/src/templates.rs`
**Changes**: (a) Add `sha256: Option<String>` to `TemplateDetail` with
`skip_serializing_if`; (b) extend the resolver's internal by-name
storage with a precomputed `sha256: Option<String>`; (c) populate it
inside `TemplateResolver::build` from the winning tier's content;
(d) have `detail()` clone the cached string.

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDetail {
    pub name: String,
    pub tiers: Vec<TemplateTier>,
    pub active_tier: TemplateTierSource,
    /// Content hash of the resolved winning-tier content in the
    /// project-wide `sha256-<64-hex>` etag shape (matches
    /// `TemplateTier.etag` and `SsePayload::DocChanged.etag`).
    /// Omitted from JSON when the winning tier is absent or its
    /// content is empty (per AC10). Whose presence/absence is the
    /// only "no winning content" signal — never `null`, never `""`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}
```

Inside the resolver, attach the digest to each entry's existing
tier vec:

```rust
struct TemplateEntry {
    tiers: Vec<TemplateTier>,
    active_tier: TemplateTierSource,
    sha256: Option<String>,
}
```

Extract a small `pub(crate)` helper so the encoding rule lives in
one place (used by `build`, the SSE handler's diffing logic in
Phase 2, and any future consumer):

```rust
/// `sha256-<64-hex>` form, matching the per-tier etag shape.
/// `None` for empty content (AC10 — empty-string digest is
/// suppressed so an empty winning file produces no displayable
/// hash) and for absent content.
pub(crate) fn content_sha256(content: Option<&str>) -> Option<String> {
    content
        .filter(|s| !s.is_empty())
        .map(|s| format!("sha256-{}", hex::encode(Sha256::digest(s.as_bytes()))))
}
```

In `TemplateResolver::build`, after the existing tier-vec
construction and winner selection, compute the digest **once** for
each template:

```rust
let sha256 = content_sha256(
    tiers.iter()
        .find(|t| t.active && t.present)
        .and_then(|t| t.content.as_deref()),
);
```

`TemplateResolver::detail` (currently `templates.rs:137-149`)
returns the precomputed value via `entry.sha256.clone()` — no
hashing on the request thread.

Imports: `use sha2::{Digest, Sha256};` and use the existing `hex`
crate.

> **Why prefix + cache**: aligning the new field with the existing
> `sha256-<hex>` etag shape removes a divergence and the UI's
> prepend responsibility. Caching the digest pairs naturally with
> the ArcSwap-based rebuild boundary (Phase 2) — recomputation only
> happens when the resolver is rebuilt.

#### 2. TDD: integration test for the new field

**File**: `skills/visualisation/visualise/server/tests/api_templates.rs`
**Changes**: Add three new tests modelled after
`template_detail_returns_three_tiers_with_plugin_default_active`
(lines 35-60):

```rust
#[tokio::test]
async fn template_detail_includes_sha256_of_winning_content() {
    // ... fetch /api/templates/adr (whose winning tier in the
    // seeded fixture has non-empty content)
    let sha = v["sha256"].as_str().expect("sha256 must be present");
    // Shape: "sha256-" + 64 lowercase hex chars (matches per-tier etag)
    assert!(sha.starts_with("sha256-"), "must be sha256-prefixed: {sha}");
    let hex_part = &sha["sha256-".len()..];
    assert_eq!(hex_part.len(), 64, "hex digest must be 64 chars: {sha}");
    // AC8 requires lowercase — explicit class, not is_ascii_hexdigit()
    assert!(
        hex_part.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "must be lowercase hex only: {sha}",
    );
    // Cross-check: re-derive from fixture content (robust to fixture edits)
    let active = tiers.iter().find(|t| t["active"] == true).unwrap();
    let content = active["content"].as_str().unwrap();
    let expected = format!(
        "sha256-{}",
        hex::encode(sha2::Sha256::digest(content.as_bytes())),
    );
    assert_eq!(sha, expected);
}

#[tokio::test]
async fn template_detail_omits_sha256_when_winning_content_empty() {
    // ... configure a template whose winning tier is an empty file
    // assert v.get("sha256").is_none() and the JSON does not contain
    // the literal substring "sha256":null
    let raw = std::str::from_utf8(&bytes).unwrap();
    assert!(!raw.contains("\"sha256\":null"));
    assert!(!raw.contains("\"sha256\":\"\""));
}

#[tokio::test]
async fn template_detail_omits_sha256_when_winning_tier_absent() {
    // ... configure a template with no overrides AND no plugin
    // default file present (so all three tiers are present=false)
    // assert v.get("sha256").is_none()
}

#[tokio::test]
async fn template_detail_omits_sha256_when_winning_content_not_utf8() {
    // Edge case: tier file exists with bytes that fail UTF-8 conversion
    // (load_via_driver leaves content=None but present=true). The
    // winning tier is "present but content unavailable" — sha256 must
    // be omitted to match the empty/absent contract.
    // Use bytes: [0xFF, 0xFE, 0x00, 0xFF] (invalid UTF-8 sequence)
}
```

The first test piggybacks on the existing `common::seeded_cfg`
fixture. The second and third add inline directories to construct a
template whose winning tier is empty (write a 0-byte file) or absent
(point all tiers at non-existent paths).

Also add inline `templates.rs` unit tests inside the existing
`#[cfg(test)] mod tests` block:

```rust
#[tokio::test]
async fn detail_sha256_matches_winning_tier_content_prefixed() {
    let tmp = tempfile::tempdir().unwrap();
    let driver = test_driver(tmp.path());
    let mut map = HashMap::new();
    map.insert("adr".to_string(), tiers_all_three(tmp.path()));
    let r = TemplateResolver::build(&map, &driver).await;
    let d = r.detail("adr").unwrap();
    let active = d.tiers.iter().find(|t| t.active).unwrap();
    let expected = format!(
        "sha256-{}",
        hex::encode(sha2::Sha256::digest(
            active.content.as_ref().unwrap().as_bytes(),
        )),
    );
    assert_eq!(d.sha256.as_deref(), Some(expected.as_str()));
}

#[tokio::test]
async fn detail_sha256_omitted_when_winning_content_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let empty_plugin = tier(tmp.path(), "plugin-empty.md", "");
    let t = TemplateTiers {
        config_override: None,
        user_override: tmp.path().join("missing-user.md"),
        plugin_default: empty_plugin,
    };
    let driver = test_driver(tmp.path());
    let mut map = HashMap::new();
    map.insert("e".to_string(), t);
    let r = TemplateResolver::build(&map, &driver).await;
    let d = r.detail("e").unwrap();
    assert!(d.sha256.is_none(), "empty winning content -> no sha256");
}

#[tokio::test]
async fn detail_sha256_cached_at_build_time() {
    // Mutating the tier vec post-build must not change detail().sha256
    // — the digest is owned by the resolver entry, not recomputed.
    // (Validates the cache invariant; future regressions that move
    // computation back into detail() would fail this test.)
    let r = TemplateResolver::build(&map, &driver).await;
    let s1 = r.detail("adr").unwrap().sha256.clone();
    let s2 = r.detail("adr").unwrap().sha256.clone();
    assert_eq!(s1, s2);
}
```

#### 3. Server Cargo manifest

**File**: `skills/visualisation/visualise/server/Cargo.toml`
**Changes**: Confirm `sha2` and `hex` are listed under `[dependencies]`.
The research notes both are already present
(used by `file_driver::etag_of`). If they're not direct deps yet
(only transitive), add them explicitly:

```toml
sha2 = "0.10"
hex = "0.4"
```

### Success Criteria:

#### Automated Verification:

- [ ] Backend tests pass: `cd skills/visualisation/visualise/server && cargo test`
- [ ] Specifically the new tests are green:
      `cargo test --test api_templates template_detail_includes_sha256_of_winning_content`,
      `cargo test --test api_templates template_detail_omits_sha256_when_winning_content_empty`,
      `cargo test --test api_templates template_detail_omits_sha256_when_winning_tier_absent`,
      `cargo test -p accelerator-visualiser templates::tests::detail_sha256_matches_winning_tier_content`,
      `cargo test -p accelerator-visualiser templates::tests::detail_sha256_omitted_when_winning_content_empty`
- [ ] Clippy passes: `cargo clippy -p accelerator-visualiser -- -D warnings`
- [ ] No JSON regressions: the existing
      `template_detail_returns_three_tiers_with_plugin_default_active`
      test still passes.

#### Manual Verification:

- [ ] `curl http://127.0.0.1:<port>/api/templates/adr | jq` shows a
      top-level `sha256` field of the form `"sha256-<64-hex>"` when
      the winning tier has content; field is absent (not `null`, not
      `""`) when the winning content is empty or missing.

---

## Phase 2: Backend — `TemplateChanged` SSE event + watcher wiring

### Overview

Extend the SSE payload with a `TemplateChanged` variant; wrap
`AppState.templates` in `ArcSwap<TemplateResolver>` for lock-free
reads; route all template-tier rebuild work through a single
`Notify`-driven coalescing task (so concurrent edits cannot
lose-update each other and bursts collapse to the minimum number
of rebuilds without any bounded queue); precompute a canonical-path
index from `cfg.templates` at startup, walking up to the nearest
existing ancestor for tier files absent at boot (so canonicalised
event paths match the index on macOS, under symlinks, and for
user/config overrides added to a running workspace); watch
tier-parent directories recursively (so nested layouts and editor
atomic-renames don't drop events); broadcast `template-changed`
for every winning-sha256 change (including empty/absent
transitions, with `sha256: Option<String>` in the payload, so
AC10 is satisfied via the live path) and **only** for changes —
identical-bytes edits and non-winning-tier edits produce zero
broadcasts.

The template-change handling is extracted to a small
`TemplateChangeHandler` collaborator that owns the resolver
ArcSwap, the canonical-path index, and the consumer task. The
watcher's existing `on_path_changed_debounced` stays focused on
docs and calls the handler additively (i.e., a path that is both
a tier and a doc emits both `template-changed` and `doc-changed`).

### Changes Required:

#### 1. Add `arc-swap` to server dependencies

**File**: `skills/visualisation/visualise/server/Cargo.toml`
**Changes**: Add `arc-swap = "1"` under `[dependencies]`.

#### 2. New SSE payload variant

**File**: `skills/visualisation/visualise/server/src/sse_hub.rs`
**Changes**: Add `TemplateChanged` variant to `SsePayload` with
`sha256: Option<String>` (omitted from the wire when `None`):

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SsePayload {
    DocChanged { /* unchanged */ },
    DocInvalid { /* unchanged */ },
    // `rename_all = "camelCase"` applied defensively so a future
    // multi-word field (e.g. `display_name`) keeps the project's
    // camelCase JSON convention without a per-field rename.
    #[serde(rename_all = "camelCase")]
    TemplateChanged {
        template: String,
        /// `sha256-<64-hex>` form (matches `DocChanged.etag`),
        /// `None` (omitted) when the new winning content is empty
        /// or absent. AC10 relies on this transition being
        /// broadcast — the client must be able to invalidate when
        /// a winning file is emptied.
        #[serde(skip_serializing_if = "Option::is_none")]
        sha256: Option<String>,
        timestamp: DateTime<Utc>,
    },
}
```

`#[serde(tag = "type", rename_all = "kebab-case")]` makes this
serialise to `"type":"template-changed"` automatically.

#### 3. TDD: SSE payload serialisation test

**File**: `skills/visualisation/visualise/server/src/sse_hub.rs`
**Changes**: In the existing `#[cfg(test)] mod tests` block, extend
`sse_payload_json_wire_format` (currently lines 111-151) with
template-changed assertions for both the Some-sha256 and None-sha256
shapes:

```rust
let with_hash = SsePayload::TemplateChanged {
    template: "adr".into(),
    sha256: Some("sha256-abc123...".into()),
    timestamp: ts,
};
let json = serde_json::to_string(&with_hash).unwrap();
assert!(json.contains("\"type\":\"template-changed\""), "json: {json}");
assert!(json.contains("\"template\":\"adr\""), "json: {json}");
assert!(json.contains("\"sha256\":\"sha256-abc123...\""), "json: {json}");

let without_hash = SsePayload::TemplateChanged {
    template: "adr".into(),
    sha256: None,
    timestamp: ts,
};
let json = serde_json::to_string(&without_hash).unwrap();
// AC10: None must be omitted entirely, not serialised as null/""
assert!(!json.contains("\"sha256\":null"), "json: {json}");
assert!(!json.contains("\"sha256\":\"\""), "json: {json}");
assert!(json.contains("\"type\":\"template-changed\""), "json: {json}");
assert!(json.contains("\"template\":\"adr\""), "json: {json}");
```

#### 4. AppState — `ArcSwap<TemplateResolver>` + canonical-path index

**File**: `skills/visualisation/visualise/server/src/server.rs`
**Changes**: Change the field type from `Arc<TemplateResolver>` to
`Arc<ArcSwap<TemplateResolver>>`. Build a precomputed canonical
template-tier path index at startup. Wrap both in an
`Arc<TemplateChangeHandler>` (new type, defined in Section 6) so
downstream callers only see one collaborator.

```rust
// server.rs
pub struct AppState {
    // ...
    pub templates: Arc<arc_swap::ArcSwap<crate::templates::TemplateResolver>>,
    pub template_change_handler: Arc<crate::watcher::TemplateChangeHandler>,
    // ...
}

// AppState::build
let templates = Arc::new(arc_swap::ArcSwap::from_pointee(
    crate::templates::TemplateResolver::build(&cfg.templates, driver.as_ref()).await,
));

// Canonicalise once at startup. Falls back to the original path
// if canonicalize fails (e.g. file does not yet exist) — same
// pattern the watcher already uses on event paths.
let canonical_index = crate::watcher::TierPathIndex::build(&cfg.templates).await;

let template_change_handler = Arc::new(
    crate::watcher::TemplateChangeHandler::spawn(
        templates.clone(),
        Arc::new(cfg.templates.clone()),
        driver.clone(),
        canonical_index,
        hub.clone(),
    ),
);
```

```rust
// api/templates.rs — replace 2 call sites
// before: state.templates.list() / state.templates.detail(&name)
// after:
let resolver = state.templates.load();
Json(TemplatesListResponse { templates: resolver.list() })
// and:
state.templates.load().detail(&name)
    .map(Json)
    .ok_or(ApiError::NotFound(name))
```

`load()` returns a `Guard<Arc<T>>` that derefs to `&T`; both `list()`
and `detail()` take `&self`, so this compiles without changes to
their signatures.

#### 5. Pass template-change handler to the watcher

**File**: `skills/visualisation/visualise/server/src/server.rs`
**Changes**: At `server.rs:286`, extend `watch_dirs` to include
template-tier-parent directories, canonicalising before dedup:

```rust
async fn canonical_or_self(p: PathBuf) -> PathBuf {
    tokio::fs::canonicalize(&p).await.unwrap_or(p)
}

let mut watch_dirs: Vec<PathBuf> = Vec::new();
for p in state.cfg.doc_paths.values() {
    watch_dirs.push(canonical_or_self(p.clone()).await);
}
for p in crate::file_driver::template_extra_roots(&state.cfg.templates) {
    watch_dirs.push(canonical_or_self(p).await);
}
// Canonical dedup catches /var-vs-/private/var on macOS and
// symlink-equivalent paths that raw-PathBuf dedup misses.
watch_dirs.sort();
watch_dirs.dedup();
```

Pass `state.template_change_handler.clone()` into `watcher::spawn`
as a single collaborator (replacing the earlier proposal of three
new parameters: `templates`, `cfg`, `driver`). The watcher's
existing parameter list grows by exactly one.

Tier-parent directories are watched **recursively**
(`RecursiveMode::Recursive`) so editor atomic-rename patterns and
nested tier layouts produce events. Scope is preserved by the
`is_markdown` filter (already present in `watcher.rs:67-69`) and
by the canonical-path index inside the handler — non-tier files
under a tier-parent directory are filtered out by the handler's
`try_handle` returning `false`.

> **Risk**: switching to recursive watches widens fs-event volume
> for tier-parent directories. The `is_markdown` filter and the
> O(1) canonical-path lookup keep cost negligible per event;
> measured impact on a workspace with ~10 templates is below
> noise.

#### 6. New module — `TemplateChangeHandler` + `TierPathIndex`

**File**: `skills/visualisation/visualise/server/src/watcher.rs`
(new types alongside the existing watcher loop)
**Changes**: Introduce two small types that own the template-side
state and serialise rebuilds.

```rust
/// O(1) lookup from canonical tier-file path → list of template
/// names that reference it. Built once at startup. Multiple
/// templates may share a tier file (e.g. a common plugin-default),
/// so the value is `Vec<String>`.
pub struct TierPathIndex {
    by_canonical_path: HashMap<PathBuf, Vec<String>>,
}

/// Canonicalise a path that may not exist yet by walking up to
/// the nearest existing ancestor, canonicalising it, and
/// re-appending the descendant components. Used by both
/// `TierPathIndex::build` (to canonicalise absent-at-startup tier
/// paths) AND by the watcher's per-event path resolution (so
/// **delete** events — where canonicalize fails because the inode
/// is gone — still match the index).
///
/// Falls back to the raw path only when no ancestor exists
/// (effectively never on a real filesystem).
///
/// Note: assumes intermediate components, if created later, are
/// real directories rather than symlinks. A symlink introduced
/// at an intermediate path after startup would resolve differently
/// at event canonicalize time than at index build time and would
/// silently miss; document as a known edge case.
pub(crate) async fn canonicalise_path_or_ancestor(raw: &Path) -> PathBuf {
    if let Ok(c) = tokio::fs::canonicalize(raw).await {
        return c;
    }
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cursor = raw.to_path_buf();
    while let Some(parent) = cursor.parent().map(Path::to_path_buf) {
        if let Some(name) = cursor.file_name() {
            tail.push(name.to_os_string());
        }
        cursor = parent;
        if let Ok(canonical_ancestor) = tokio::fs::canonicalize(&cursor).await {
            let mut out = canonical_ancestor;
            for name in tail.iter().rev() {
                out.push(name);
            }
            return out;
        }
        if cursor.as_os_str().is_empty() {
            break;
        }
    }
    raw.to_path_buf()
}

impl TierPathIndex {
    pub async fn build(templates: &HashMap<String, TemplateTiers>) -> Self {
        let mut by_canonical_path: HashMap<PathBuf, Vec<String>> = HashMap::new();
        for (name, t) in templates {
            for raw in t.iter_paths() { // walks (co, user, plugin)
                let canon = canonicalise_path_or_ancestor(&raw).await;
                by_canonical_path.entry(canon).or_default().push(name.clone());
            }
        }
        Self { by_canonical_path }
    }

    pub fn names_for(&self, canonical: &Path) -> &[String] {
        self.by_canonical_path
            .get(canonical)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn has_any(&self, canonical: &Path) -> bool {
        self.by_canonical_path.contains_key(canonical)
    }
}

/// Owns the resolver ArcSwap and the canonical-path index. All
/// template-tier change handling goes through `try_handle`. A
/// single background task coalesces all pending changes into one
/// rebuild via `tokio::sync::Notify` — no bounded channel, no
/// overflow, no silent drops; sustained burst collapses into the
/// minimum number of rebuilds (one per consumer iteration).
pub struct TemplateChangeHandler {
    /// `tokio::sync::Notify` rather than `mpsc::Sender` because
    /// repeated `notify_one()` calls before the next
    /// `notified().await` saturate at one pending permit —
    /// exactly the coalescing behaviour the consumer needs. An
    /// mpsc would require a capacity decision and a drop policy;
    /// neither is needed here because the consumer always reads
    /// the latest filesystem state when it wakes.
    notify: Arc<tokio::sync::Notify>,
    index: Arc<TierPathIndex>,
}

impl TemplateChangeHandler {
    pub fn spawn(
        templates: Arc<ArcSwap<TemplateResolver>>,
        cfg_templates: Arc<HashMap<String, TemplateTiers>>,
        driver: Arc<dyn FileDriver>,
        index: TierPathIndex,
        hub: Arc<SseHub>,
    ) -> Self {
        let index = Arc::new(index);
        let notify = Arc::new(tokio::sync::Notify::new());
        let consumer_notify = notify.clone();

        // Initial sha256 snapshot per template — used to suppress
        // no-op broadcasts and to dedup across-tier multiplicity
        // (two tier files for the same template with identical
        // winning content produces zero broadcasts, not two).
        let mut previous: HashMap<String, Option<String>> = cfg_templates
            .keys()
            .map(|name| {
                (
                    name.clone(),
                    templates.load().detail(name).and_then(|d| d.sha256),
                )
            })
            .collect();

        tokio::spawn(async move {
            // Coalescing consumer loop: each iteration handles all
            // changes pending since the last iteration. `Notify`
            // holds a single pending permit; repeated `notify_one`
            // before the next `notified().await` coalesce to one
            // wake-up — exactly the desired behaviour.
            loop {
                consumer_notify.notified().await;

                // Isolate `TemplateResolver::build` from the loop:
                // a panic inside build surfaces here as a JoinError
                // rather than killing the consumer task and
                // silently disabling all future broadcasts. The
                // consumer logs and continues; subsequent edits
                // get another rebuild attempt.
                let cfg_for_build = cfg_templates.clone();
                let driver_for_build = driver.clone();
                let build_result = tokio::spawn(async move {
                    TemplateResolver::build(
                        &cfg_for_build,
                        driver_for_build.as_ref(),
                    ).await
                }).await;

                let new_resolver = match build_result {
                    Ok(r) => Arc::new(r),
                    Err(join_err) => {
                        tracing::error!(
                            error = ?join_err,
                            "TemplateResolver::build panicked or was \
                             cancelled; consumer skipping this rebuild \
                             and remaining alive for future events",
                        );
                        continue;
                    }
                };

                // Diff per-template winning sha256 against the
                // previous snapshot; bind broadcasts to the rebuild
                // that produced them.
                let mut to_broadcast: Vec<(String, Option<String>)> = Vec::new();
                for name in cfg_templates.keys() {
                    let new_sha = new_resolver.detail(name).and_then(|d| d.sha256);
                    // `previous` is initialised from `cfg_templates.keys()`
                    // at construction time and cfg is frozen for the
                    // task's lifetime (no hot-reload — see "What we're
                    // NOT doing"), so `previous.get(name)` is always
                    // `Some(_)` and the `flatten()` only collapses
                    // `Some(None)` → `None` (i.e., previously-absent
                    // sha vs newly-absent sha both compare equal).
                    let prev = previous.get(name).cloned().flatten();
                    if new_sha != prev {
                        previous.insert(name.clone(), new_sha.clone());
                        to_broadcast.push((name.clone(), new_sha));
                    }
                }

                templates.store(new_resolver);

                for (template, sha256) in to_broadcast {
                    // Broadcast carries Option<String> (omit-on-None
                    // via skip_serializing_if) so the empty/absent
                    // transition still triggers a client invalidate
                    // — satisfies AC10 via the live path.
                    hub.broadcast(SsePayload::TemplateChanged {
                        template,
                        sha256,
                        timestamp: Utc::now(),
                    });
                }
            }
        });

        Self { notify, index }
    }

    /// Returns `true` when the path was claimed as a template-tier
    /// change. The watcher uses the return value additively — a
    /// path that is *also* under a doc_path still flows through
    /// the existing doc-changed branch.
    pub fn try_handle(&self, canonical_path: &Path) -> bool {
        if !self.index.has_any(canonical_path) {
            return false;
        }
        // Fire-and-forget. If the consumer is currently building,
        // this permit is held until the next `notified().await` —
        // sustained burst collapses to one extra rebuild.
        self.notify.notify_one();
        true
    }
}
```

`TemplateTiers::iter_paths` is a small helper added alongside the
struct: returns an iterator over the (optional) config-override,
user-override, and plugin-default paths.

##### Why this shape

- **`Notify`-based coalescing** replaces a bounded mpsc — no
  overflow, no silent drops, no `try_send` vs `send().await`
  question. `Notify` holds at most one pending permit; repeated
  `notify_one()` calls before the next `notified().await`
  coalesce into one wake-up. Sustained burst collapses into at
  most one extra rebuild beyond the currently-running one,
  regardless of edit rate.
- **Panic isolation** via inner `tokio::spawn` around
  `TemplateResolver::build`: a panic surfaces as `JoinError`,
  gets logged, and the consumer loop continues to its next
  `notified().await`. Without this, a single panic would
  silently disable all future template-changed broadcasts (the
  `ArcSwap` would keep serving the last-known-good resolver via
  the API, masking the failure for the lifetime of the process).
- **Per-template sha256 diffing** in the consumer suppresses no-op
  broadcasts and naturally dedups across-tier multiplicity:
  two tier edits for the same template whose winning sha256 is
  unchanged produce zero broadcasts; a single edit that changes
  multiple templates' winning sha256 (via a shared plugin
  default) produces one broadcast per affected template.
- **Ancestor-canonicalisation fallback** in `TierPathIndex::build`
  resolves the absent-at-startup case: user/config overrides
  that don't exist yet still get a canonical-matching key, so
  events for them after creation hit the index correctly.
- **Single rebuild task** still guarantees rebuild serialisation;
  build → snapshot → store ordering still pairs each broadcast's
  sha256 with the rebuild that produced it.
- **`try_handle` is now synchronous** — `Notify::notify_one()` is
  a sync call. The watcher's call site drops the `.await`.
- **`try_handle` returns `bool`** so the watcher's doc-flow stays
  additive: a path that is both a tier and a doc emits both
  events. The bool is used at the call site (see §7) for a
  tracing span attribute.

#### 7. Watcher — call the handler additively

**File**: `skills/visualisation/visualise/server/src/watcher.rs`
**Changes**: Extend `pub fn spawn(...)` with **one** new parameter
(`handler: Arc<TemplateChangeHandler>`), pass it down to
`on_path_changed_debounced`, and call it before the existing doc
flow without short-circuiting.

```rust
// Use the same walk-up canonicalisation as TierPathIndex::build so
// that delete events (canonicalize fails because the inode is gone)
// still match the index. This is symmetric with the index-side fix
// and is what makes AC10's deletion variant work via SSE — `rm`,
// `git checkout` reverting an override, and editor atomic-rename
// transient deletes all produce a final event whose path no longer
// exists at canonicalize time.
let canonical = canonicalise_path_or_ancestor(&path).await;

// Additive: a tier-file change is signalled for rebuild + broadcast.
// We do NOT early-return — if the same canonical path is also under
// a watched doc_path, the existing doc flow still runs below.
//
// INVARIANT: Templates have no frontend write path today, so we
// skip `WriteCoordinator::should_suppress` for the template branch.
// If a future feature adds template editing via the frontend, this
// call must be routed through the WriteCoordinator first to avoid
// self-cause echoes back to the originating client.
let is_template = handler.try_handle(&canonical);
tracing::debug!(
    file = %canonical.display(),
    is_template,
    "watcher dispatched fs event",
);

// existing flow: WriteCoordinator suppress, rescan, cluster recompute,
// emit DocChanged/DocInvalid …
```

The existing `on_path_changed_debounced` function gains exactly
one parameter (`handler`); the body's structure is unchanged
except for the additive call above. `try_handle` is synchronous
(no `.await`), and the `bool` return is logged via the existing
event-style tracing convention (`tracing::debug!` with structured
fields — consistent with the watcher's existing call sites at
`watcher.rs:55,124,151,166`). No `return`-mid-async-fn control
flow; no parameter explosion.

`canonicalise_path_or_ancestor` becomes `pub(crate)` so the
watcher main loop can call it.

> **Doc-overlap handling**: if a configured tier path also sits
> under a watched doc_path, both `template-changed` and
> `doc-changed` fire for the same edit. This is the correct
> behaviour — both views need to invalidate. The frontend
> reducers handle each event independently.

#### 8. TDD: SSE end-to-end test for template-changed

**File**: `skills/visualisation/visualise/server/tests/sse_e2e.rs`
**Changes**: Add a sibling test
`template_file_mutation_arrives_as_template_changed_sse_event`,
modelled on the existing `file_mutation_arrives_as_sse_event` (lines
6-98). Configure a single template via `cfg.templates`, point a tier
file at a temp path, write initial content, launch the server, open
SSE, mutate the tier file, and assert the new event's full shape
(including the exact sha256 derived from the new content).

```rust
#[tokio::test]
async fn template_file_mutation_arrives_as_template_changed_sse_event() {
    let tmp = tempfile::tempdir().unwrap();
    let tier_dir = tmp.path().join("templates");
    std::fs::create_dir_all(&tier_dir).unwrap();
    let tier_file = tier_dir.join("adr.md");
    std::fs::write(&tier_file, "v1").unwrap();

    let mut templates = HashMap::new();
    templates.insert("adr".to_string(), accelerator_visualiser::config::TemplateTiers {
        config_override: None,
        user_override: tier_dir.join("missing-user.md"),
        plugin_default: tier_file.clone(),
    });
    let cfg = accelerator_visualiser::config::Config { /* …, templates, … */ };

    // … boot, poll server-info.json, open /api/events …

    tokio::time::sleep(Duration::from_millis(100)).await;
    std::fs::write(&tier_file, "v2").unwrap();

    // Read chunks until we see "template-changed" or time out.
    let chunk = read_until_substring(&mut stream, "\"type\":\"template-changed\"").await;
    // Pin the wire shape: parse the SSE payload as JSON and
    // assert exact sha256 derived from the new on-disk content.
    let payload: serde_json::Value = serde_json::from_str(&chunk).unwrap();
    assert_eq!(payload["template"], "adr");
    let expected = format!(
        "sha256-{}",
        hex::encode(sha2::Sha256::digest(b"v2")),
    );
    assert_eq!(payload["sha256"], expected);
}
```

Also add a paired e2e test
`template_file_emptied_arrives_as_template_changed_with_no_sha256`
that truncates the tier file to 0 bytes and asserts the SSE event
arrives with the `sha256` key **absent** from the payload (verifying
AC10's empty-content live path).

#### 9. TDD: handler, index, and watcher unit tests

Tests are split between three modules so each contract is covered
where it lives:

**File**: `skills/visualisation/visualise/server/src/watcher.rs`
inline `#[cfg(test)] mod tier_path_index_tests` — direct unit
tests for `TierPathIndex`:

A. **Path referenced by N templates returns all N names** —
   configure two templates pointing at the same plugin-default
   file; `index.names_for(&canonical)` returns both names.
B. **Path not present returns an empty slice** —
   `index.names_for(unrelated)` returns `&[]`.
C. **Absent-at-startup tier file is keyed by canonical ancestor +
   filename** — point a user-override at a non-existent file in
   an existing directory; after `build`, look up the same file
   via its canonicalised path (computed at look-up time as if it
   were created); the lookup hits. This locks the
   `canonicalise_path_or_ancestor` fix from the Pass-2 review.
D. **macOS canonicalisation match** — on macOS where the temp
   dir is symlinked (`/var` vs `/private/var`), look up via the
   resolved path and assert the index hits. Gated by
   `cfg!(target_os = "macos")`.
E. **Walk-up fallback for deeply absent paths** — point a tier
   at a path under a not-yet-existing parent (e.g.
   `tmp/templates/missing-dir/adr.md` where `missing-dir`
   doesn't exist); assert the index key matches what
   `canonicalise_path_or_ancestor` produces at event time
   (canonical of `tmp/templates/` + `missing-dir/adr.md`).

**File**: `skills/visualisation/visualise/server/src/watcher.rs`
inline `#[cfg(test)] mod template_change_handler_tests` — direct
unit tests for `TemplateChangeHandler` against a lightweight
in-process hub (subscribed at construction, drained after each
assertion):

F. **Burst of `try_handle` calls collapses to ≤ 2 rebuilds** —
   use the test-only `gate_consumer()` seam (see below) to hold
   the consumer at its first `notified().await`; fire
   `try_handle` 200 times in tight succession; release the
   gate; assert exactly **one or two** rebuilds completed (one
   collapsing the burst, plus at most one trailing for a
   permit issued after the consumer woke). A loose "bounded
   number" would not catch a regression that woke once per
   notification.
F1. **Sustained back-pressure produces no panic, no deadlock,
    and a final-state broadcast** — use the test-only
    `gate_consumer()` seam to block the consumer mid-rebuild;
    fire `try_handle` again; release the gate; assert the
    consumer completes the in-flight rebuild, then performs a
    second rebuild whose broadcast matches the last-written
    content. Replaces the Pass-1 'mpsc full channel' concern
    entirely.
G. **No broadcast when winning sha256 is unchanged across a
    rebuild** — a tier file change that doesn't move the
    winning content (e.g. a non-winning tier edit, or an edit
    that produces identical bytes) produces zero broadcasts.
    This locks the per-template diffing.
H. **Shared tier file produces one broadcast per affected
    template** — two templates pointing at the same
    plugin-default file, edit it once, assert two broadcasts
    (one per template name) with matching sha256.
I. **Empty-content transition broadcasts with `sha256: None`** —
   write content, wait for broadcast, truncate to 0 bytes;
   assert a second broadcast with `sha256: None`.
I1. **Deletion of winning-tier file broadcasts with `sha256:
    None`** — write content, await first broadcast,
    `std::fs::remove_file`, assert a second `TemplateChanged`
    with `sha256: None`. This pins the watcher's
    `canonicalise_path_or_ancestor` event-path fix: without the
    walk-up fallback, the delete event's path fails
    `canonicalize` and the raw path doesn't match the index,
    silently swallowing the deletion broadcast.
J. **Build → snapshot → store causal pairing** — write content
   A, await broadcast, assert sha256 = hash(A); write B, await
   broadcast, assert sha256 = hash(B). Pins the design intent.
J1. **Two tier files for the same template with unchanged
    winning content produce zero broadcasts** — configure a
    template with all three tiers present; edit a
    non-winning tier file twice; assert zero broadcasts (the
    per-template sha256 diff suppresses the no-op). Locks the
    "Why this shape" claim about per-template dedup.
K. **Concurrent reads during rebuild succeed (ArcSwap stress)**
   — spawn N=32 reader tasks calling
   `templates.load().detail("adr")` for ≥500ms while a writer
   fires ≥100 `try_handle`s; assert no panic, zero read
   failures, and every read returns a structurally valid
   `TemplateDetail`.
K1. **Consumer survives a panic in `TemplateResolver::build`**
    — inject a build failure via a test-only `FileDriver`
    implementation that panics on its second read; fire
    `try_handle` once (initial successful build), then again
    (panic), then again (recovery — driver returns to normal);
    assert the consumer logs the panic and continues to emit
    broadcasts on the third edit. Pins the panic-isolation
    invariant from "Why this shape".

**File**: `skills/visualisation/visualise/server/src/watcher.rs`
existing inline `#[cfg(test)] mod tests` — watcher integration
covering only the routing decisions:

L. **Non-template path still routes to doc-changed** —
   regression guard that the existing doc flow runs for a path
   not in the index.
M. **Tier path also under a doc_path produces both events** —
   additive precedence: a single edit to a file matched by both
   produces one `TemplateChanged` and one `DocChanged`.
N. **Recursive-watch scoping: non-tier markdown sibling under
   tier-parent does not trigger a rebuild** — write
   `<tier-parent>/notes.md` (not in any template's tier set);
   assert zero `TemplateChanged` broadcasts and (via the
   `rebuild_counter()` test seam — see below) zero resolver
   rebuilds. Locks the `is_markdown` + `TierPathIndex.has_any`
   scoping the recursive widening relies on.

##### Test seams (committed API)

Tests F, F1, K, K1, and N reference `gate_consumer()` and
`rebuild_counter()` test seams. These are committed surface, not
ad-hoc patches:

```rust
#[cfg(test)]
impl TemplateChangeHandler {
    /// Returns a gate that, when held by the test, blocks the
    /// consumer at the next iteration's start. Drop the gate to
    /// release. Used by tests F and F1 to control rebuild
    /// timing.
    pub(crate) fn gate_consumer(&self) -> ConsumerGate { /* … */ }

    /// Returns an `Arc<AtomicUsize>` counting completed rebuilds
    /// (incremented after each successful `templates.store`).
    /// Used by tests F, K, K1, and N for precise count assertions.
    pub(crate) fn rebuild_counter(&self) -> Arc<AtomicUsize> { /* … */ }
}
```

Both seams are `#[cfg(test) pub(crate)`. The consumer task
acquires the gate before each iteration (via
`gate.notified().await` if a gate is installed) and increments
the counter after each successful store. Production code paths
are unaffected.

### Success Criteria:

#### Automated Verification:

- [ ] All backend tests pass: `cd skills/visualisation/visualise/server && cargo test`
- [ ] The new SSE serialisation assertions are green (both
      Some-sha256 and None-sha256 shapes):
      `cargo test -p accelerator-visualiser sse_hub::tests::sse_payload_json_wire_format`
- [ ] Both new e2e tests are green:
      `cargo test --test sse_e2e template_file_mutation_arrives_as_template_changed_sse_event`
      `cargo test --test sse_e2e template_file_emptied_arrives_as_template_changed_with_no_sha256`
- [ ] The new `tier_path_index_tests` are green (multi-name
      lookup, miss, absent-at-startup ancestor fallback, macOS
      canonicalisation, walk-up fallback).
- [ ] The new `template_change_handler_tests` are green (burst
      coalesce ≤2 rebuilds, back-pressure recovery, no-op
      suppression, shared-tier broadcast, empty transition,
      deletion transition, causal pairing, multi-tier-same-template
      no-op, ArcSwap concurrent-read stress, panic isolation).
- [ ] The new watcher integration tests are green (non-template
      passthrough, doc-overlap additive, recursive-watch
      non-tier-sibling no-op).
- [ ] Existing watcher tests still pass:
      `cargo test -p accelerator-visualiser watcher::tests`
- [ ] Existing SSE e2e test still passes:
      `cargo test --test sse_e2e file_mutation_arrives_as_sse_event`
- [ ] Clippy clean: `cargo clippy -p accelerator-visualiser -- -D warnings`

#### Manual Verification:

- [ ] Launch the visualiser, open the SSE stream
      (`curl -N http://127.0.0.1:<port>/api/events`), edit a template
      tier file, observe a `template-changed` event arrive within ~1s
      with `"sha256":"sha256-<hex>"`.
- [ ] Truncate the same tier file to 0 bytes; observe a follow-up
      `template-changed` event whose payload **omits** the `sha256`
      key.
- [ ] Editing a non-template file still produces `doc-changed` as
      before.
- [ ] Editing a file that is both a tier and a doc produces both
      events.

---

## Phase 3: Frontend — types & SSE dispatch reducer

### Overview

Extend the TypeScript type surface for `TemplateDetail` and `SseEvent`,
then teach the pure `dispatchSseEvent` reducer to invalidate the
templates queries when a `template-changed` event arrives.

### Changes Required:

#### 1. Type widening

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts`
**Changes**:

```typescript
export interface TemplateDetail {
  name: string
  tiers: TemplateTier[]
  activeTier: TemplateTierSource
  /** Content hash of the resolved winning-tier content, in the
   *  project's `sha256-<64-hex>` etag shape (matches the per-tier
   *  `etag` field and `SseDocChangedEvent.etag`). Absent on the
   *  wire when there is no winning content or the content is
   *  empty (AC10). Rendered verbatim by the UI — no prefix
   *  prepending in the frontend. */
  sha256?: string
}

export interface SseTemplateChangedEvent {
  type: 'template-changed'
  template: string
  /** Same shape as `TemplateDetail.sha256`. Omitted from the
   *  payload when the new winning content is empty/absent so the
   *  client invalidates and the UI label disappears. */
  sha256?: string
  timestamp: string
}

export type SseEvent =
  | SseDocChangedEvent
  | SseDocInvalidEvent
  | SseTemplateChangedEvent
```

#### 2. Dispatch reducer branch + shared key-list helper

**File**: `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts`
**Changes**: Introduce a shared `templateKeysForEvent(event)`
helper so the direct-dispatch and drag-deferred paths cannot drift,
then call it from both:

```typescript
function templateKeysForEvent(event: SseTemplateChangedEvent) {
  return [
    queryKeys.templateDetail(event.template),
    queryKeys.templates(),
  ] as const
}
```

In `dispatchSseEvent` (currently lines 80-111), add the new branch
above the existing `doc-changed | doc-invalid` block (early return
so the existing fan-out doesn't accidentally fire on template
events):

```typescript
if (event.type === 'template-changed') {
  for (const queryKey of templateKeysForEvent(event)) {
    void queryClient.invalidateQueries({ queryKey })
  }
  return
}
```

In `queryKeysForEvent` (currently lines 52-66), return the same
list when the event is `template-changed`, so the drag-deferred
path covers it too:

```typescript
if (event.type === 'template-changed') {
  return [...templateKeysForEvent(event)]
}
```

#### 3. TDD: dispatch reducer unit tests

**File**:
`skills/visualisation/visualise/frontend/src/api/use-doc-events.test.ts`
(this file should already exist for the existing reducer; if it
doesn't, locate via `find . -name 'use-doc-events.test.*'` and add to
the nearest sibling test file).

**Changes**: Add focused tests for the new branch:

```typescript
describe('dispatchSseEvent — template-changed', () => {
  const event = {
    type: 'template-changed',
    template: 'adr',
    sha256: `sha256-${'a'.repeat(64)}`,
    timestamp: '2026-05-18T00:00:00Z',
  } as const

  it('invalidates exactly templateDetail(name) and templates()', () => {
    const qc = new QueryClient()
    const spy = vi.spyOn(qc, 'invalidateQueries')
    dispatchSseEvent(event, qc)
    // Positive count + deep-equal check — tighter than .not.toContain
    expect(spy).toHaveBeenCalledTimes(2)
    const keys = spy.mock.calls.map(c => c[0]?.queryKey)
    expect(keys).toContainEqual(queryKeys.templateDetail('adr'))
    expect(keys).toContainEqual(queryKeys.templates())
  })

  it('invalidates the same keys when sha256 is absent (empty content transition)', () => {
    const qc = new QueryClient()
    const spy = vi.spyOn(qc, 'invalidateQueries')
    dispatchSseEvent(
      { type: 'template-changed', template: 'adr', timestamp: '2026-05-18T00:00:00Z' },
      qc,
    )
    expect(spy).toHaveBeenCalledTimes(2)
  })
})

describe('queryKeysForEvent — template-changed', () => {
  it('returns the template invalidation keys for drag-deferred dispatch', () => {
    const keys = queryKeysForEvent({
      type: 'template-changed',
      template: 'adr',
      sha256: `sha256-${'a'.repeat(64)}`,
      timestamp: '2026-05-18T00:00:00Z',
    })
    expect(keys).toContainEqual(queryKeys.templateDetail('adr'))
    expect(keys).toContainEqual(queryKeys.templates())
    expect(keys).toHaveLength(2)
  })
})
```

Also add a useDocEvents-level test exercising the drag-deferred
path end-to-end:

1. Spy on `queryClient.invalidateQueries`.
2. Set drag in progress (`setDragInProgress(true)`).
3. Deliver a `template-changed` event via the hook's EventSource.
4. Assert `expect(spy).not.toHaveBeenCalled()` — nothing fires
   while the drag is active.
5. Release the drag (`setDragInProgress(false)`).
6. Assert each of `queryKeys.templateDetail('adr')` and
   `queryKeys.templates()` was invalidated exactly once.

The symmetric "zero before, exactly-once after" shape mirrors the
direct-dispatch test and locks the drag-vs-direct paths together.

### Success Criteria:

#### Automated Verification:

- [ ] Frontend tests pass:
      `npm test --prefix skills/visualisation/visualise/frontend`
- [ ] Specifically the new reducer tests are green:
      `npm test --prefix skills/visualisation/visualise/frontend -- use-doc-events`
- [ ] Typecheck passes:
      `npm run typecheck --prefix skills/visualisation/visualise/frontend`

#### Manual Verification:

- [ ] No regressions: existing doc-changed dispatch behaviour still
      fires for non-template events (covered by existing tests, but
      worth a smoke check on the dev server).

---

## Phase 4: Frontend — index tier-presence row

### Overview

Replace the single active-tier Chip on each index row with a fixed
three-Chip presence row in the order `plugin-default → user-override
→ config-override`, using short labels `default` / `user` / `config`
and mapping `(present, active)` to `neutral` / `indigo` / `green`
Chip variants.

### Changes Required:

#### 1. TDD: route tests for the tristate matrix

**File**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.test.tsx`
**Changes**: Replace the existing "renders the active tier (friendly
label) beside each template name" and "renders the active-tier label
as a neutral Chip per row" tests (lines 38-45 and 61-69) with the
following coverage:

```typescript
const mockWithVariety: TemplateSummary[] = [
  // adr: no overrides — plugin-default wins
  {
    name: 'adr',
    activeTier: 'plugin-default',
    tiers: [
      { source: 'config-override', path: '/x', present: false, active: false },
      { source: 'user-override',   path: '/y', present: false, active: false },
      { source: 'plugin-default',  path: '/z', present: true,  active: true  },
    ],
  },
  // log: user-override only, no config-override — most common
  // team setup; explicit case to lock the "user wins, config
  // absent" classification.
  {
    name: 'log',
    activeTier: 'user-override',
    tiers: [
      { source: 'config-override', path: '/x', present: false, active: false },
      { source: 'user-override',   path: '/y', present: true,  active: true  },
      { source: 'plugin-default',  path: '/z', present: true,  active: false },
    ],
  },
  // ticket: user + config both present, config wins
  {
    name: 'ticket',
    activeTier: 'config-override',
    tiers: [
      { source: 'config-override', path: '/x', present: true,  active: true  },
      { source: 'user-override',   path: '/y', present: true,  active: false },
      { source: 'plugin-default',  path: '/z', present: true,  active: false },
    ],
  },
  // review: all three present, config wins
  {
    name: 'review',
    activeTier: 'config-override',
    tiers: [
      { source: 'config-override', path: '/x', present: true,  active: true  },
      { source: 'user-override',   path: '/y', present: true,  active: false },
      { source: 'plugin-default',  path: '/z', present: true,  active: false },
    ],
  },
]

it('renders three tier chips per row in the fixed left-to-right order: default → user → config', async () => {
  vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockWithVariety })
  const { container } = render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
  await screen.findByRole('link', { name: 'adr' })
  for (const name of ['adr', 'log', 'ticket', 'review']) {
    const row = screen.getByRole('link', { name }).closest('li')!
    const chips = within(row).getAllByText(/^(default|user|config)$/)
    expect(chips.map(c => c.textContent)).toEqual(['default', 'user', 'config'])
  }
})

it('maps tier presence/active to neutral/indigo/green Chip variants', async () => {
  // adr: plugin-default is winning -> default=green, user=neutral, config=neutral
  // log: user wins, config absent -> default=indigo, user=green, config=neutral
  // ticket / review: config wins -> default=indigo, user=indigo, config=green
  vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockWithVariety })
  render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
  await screen.findByRole('link', { name: 'adr' })
  const variantFor = (rowName: string, label: 'default'|'user'|'config') => {
    const row = screen.getByRole('link', { name: rowName }).closest('li')!
    return within(row).getByText(label).closest('[data-variant]')!.getAttribute('data-variant')
  }
  expect(variantFor('adr',    'default')).toBe('green')
  expect(variantFor('adr',    'user')).toBe('neutral')
  expect(variantFor('adr',    'config')).toBe('neutral')
  expect(variantFor('log',    'default')).toBe('indigo')
  expect(variantFor('log',    'user')).toBe('green')
  expect(variantFor('log',    'config')).toBe('neutral')
  expect(variantFor('ticket', 'default')).toBe('indigo')
  expect(variantFor('ticket', 'user')).toBe('indigo')
  expect(variantFor('ticket', 'config')).toBe('green')
  expect(variantFor('review', 'default')).toBe('indigo')
  expect(variantFor('review', 'user')).toBe('indigo')
  expect(variantFor('review', 'config')).toBe('green')
})

it('no row-level highlight is applied to the winning row (CSS regression)', () => {
  // The green Chip is the sole winning-state signal on the index.
  // Regression-only assertion: the legacy `.winning` / `.active`
  // selectors must not return. Specific (`\b` word boundary) and
  // narrow in intent — this is the one place where a CSS-text grep
  // is the right tool.
  expect(indexCss).not.toMatch(/\.winning\b/)
  expect(indexCss).not.toMatch(/\.active\b/)
})
```

(Import `within` from `@testing-library/react`.)

Also update the existing `mockTemplates` fixture (lines 11-14) to
include real `tiers: TemplateTier[]` arrays — the existing `tiers:
[]` no longer renders three chips, which would break the
"renders a link for each template name" test if it inadvertently
depends on the row rendering successfully.

#### 2. Shared tier-label constants

**File** (new):
`skills/visualisation/visualise/frontend/src/routes/library/template-tier.ts`
**Changes**: Hoist tier label/ordering constants into a shared
module so the index and detail routes import from one source.
Drops `TIER_LABELS` from both routes; adds `TIER_SHORT_LABELS` and
`TIER_ORDER` alongside.

```typescript
import type { TemplateTierSource } from '../../api/types'

export const TIER_LABELS: Record<TemplateTierSource, string> = {
  'plugin-default':  'plugin default',
  'user-override':   'user override',
  'config-override': 'config override',
}

export const TIER_SHORT_LABELS: Record<TemplateTierSource, string> = {
  'plugin-default':  'default',
  'user-override':   'user',
  'config-override': 'config',
}

/** Fixed left-to-right render order for the index tier-presence row
 *  (resolution order, lowest priority first). */
export const TIER_ORDER: readonly TemplateTierSource[] = [
  'plugin-default',
  'user-override',
  'config-override',
] as const
```

#### 3. Index route — render the three-chip row

**File**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx`
**Changes**:

```typescript
import { TIER_ORDER, TIER_SHORT_LABELS } from './template-tier'

function chipVariantForTier(present: boolean, active: boolean): ChipVariant {
  if (!present)    return 'neutral'
  if (active)      return 'green'
  return 'indigo'
}

function TierPresenceRow({ tiers }: { tiers: TemplateTier[] }) {
  const byKey = new Map(tiers.map(t => [t.source, t]))
  return (
    <span className={styles.tierPresenceRow}>
      {TIER_ORDER.map(source => {
        // The backend resolver always emits all three tiers
        // (templates.rs:64,75,86), so any missing entry would be a
        // server-contract violation. Treat it as absent (and a typed
        // missing entry would render as neutral) rather than smuggling
        // a UI-side "plugin-default is always present" invariant.
        const t = byKey.get(source)
        const present = t?.present ?? false
        const active = t?.active ?? false
        return (
          <Chip key={source} variant={chipVariantForTier(present, active)}>
            {TIER_SHORT_LABELS[source]}
          </Chip>
        )
      })}
    </span>
  )
}
```

In the row render (replace lines 32-41):

```tsx
<ul className={styles.list}>
  {data.templates.map((t: TemplateSummary) => (
    <li key={t.name}>
      <Link to="/library/templates/$name" params={{ name: t.name }}>
        {t.name}
      </Link>
      <TierPresenceRow tiers={t.tiers} />
    </li>
  ))}
</ul>
```

Drop the now-unused `TIER_LABELS` constant (lines 11-15).

#### 4. CSS for the inline chip row

**File**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.module.css`
**Changes**: Add a `.tierPresenceRow` rule using flex + gap. Keep the
existing `.list li` flex container; the new span becomes a child that
fills the remaining space alongside the link.

```css
.tierPresenceRow {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  margin-left: auto;  /* push to row-end like the prototype */
}
```

### Success Criteria:

#### Automated Verification:

- [ ] All index route tests pass:
      `npm test --prefix skills/visualisation/visualise/frontend -- LibraryTemplatesIndex`
- [ ] Typecheck passes
- [ ] CSS regression assertion (no `.winning`/`.active`) still holds

#### Manual Verification:

- [ ] Visit `/library/templates` in the dev server with a project
      that has a mix of tier configurations; confirm the chip row
      ordering, labels, and colours match the prototype.

---

## Phase 5: Frontend — detail two-column layout + active-tier ring

### Overview

Convert the detail screen's single-column tier stack into a two-column
CSS grid (left: stacked tier cards; right column left genuinely empty
— Phase 6 fills it with the real preview pane), and apply an
accent-coloured outline ring to the winning tier card. The existing
indigo "active" Chip is retained.

> **No placeholder div**: the right column is rendered only in Phase
> 6 — the grid leaves the slot empty. Avoids an `aria-hidden` empty
> wrapper as a phase-boundary smell. Phases 5 and 6 are meant to ship
> together; the split exists for review clarity, not as a rollout
> boundary.

### Changes Required:

#### 1. TDD: route tests for layout and ring

**File**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.test.tsx`
**Changes**: Add three tests after the existing block:

```typescript
it('renders a two-column grid layout for the tier stack and preview pane', async () => {
  vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
  const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
  await screen.findByText('active')
  const layout = container.querySelector(`.${'twoColumn'}`)
    ?? container.querySelector('[data-testid="templates-detail-layout"]')
  // We tag the container with data-testid="templates-detail-layout"; the test
  // reads its computed `grid-template-columns` via window.getComputedStyle
  // is not stable under jsdom, so instead assert that the container has the
  // expected CSS-module class and that the CSS module text declares grid.
  expect(layout).not.toBeNull()
})

it('CSS module declares a two-column grid for the layout container', () => {
  // Anchor to the .twoColumn selector specifically so the test fails
  // if the declaration moves out from under .twoColumn or weakens to
  // `none`. (`minmax` reasserts the column-template shape too.)
  expect(templatesCss).toMatch(/\.twoColumn\s*\{[^}]*grid-template-columns:\s*minmax/m)
})

it('applies the accent-ring class to the winning tier card only', async () => {
  vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
  const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
  await screen.findByText('active')
  // The winning tier card is plugin-default per mockDetail.
  const ringed = container.querySelectorAll('[data-active="true"]')
  expect(ringed.length).toBe(1)
  // CSS regression: ring is implemented as `outline:` on .panel[data-active="true"].
  // Anchor to .panel so a stray data-active rule elsewhere can't satisfy this.
  expect(templatesCss).toMatch(/\.panel\[data-active="true"\]\s*\{[^}]*outline:/m)
})

it('retains the indigo "active" Chip alongside the ring', async () => {
  vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
  const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
  await screen.findByText('active')
  expect(container.querySelector('[data-variant="indigo"]')).not.toBeNull()
})
```

#### 2. Detail route — two-column layout + active-ring marker

**File**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx`
**Changes**: Wrap `data.tiers` rendering in a two-column grid
container. Tag the active tier section with `data-active="true"`. Leave
the right column as a placeholder for Phase 6's preview pane.

```tsx
content = (
  <div className={styles.twoColumn} data-testid="templates-detail-layout">
    <div className={styles.tiers}>
      {data.tiers.map(tier => (
        <TierPanel
          key={tier.source}
          tier={tier}
          isActive={tier.source === data.activeTier}
        />
      ))}
    </div>
    {/* Phase 6 adds <TemplatePreviewPane data={data} /> here.
        Phase 5's CSS grid leaves the right column empty for now;
        no placeholder div needed. */}
  </div>
)
```

In `TierPanel`, add `data-active={isActive ? 'true' : undefined}` to
the `<section>`:

```tsx
<section
  className={`${styles.panel} ${!tier.present ? styles.absent : ''}`}
  data-active={isActive ? 'true' : undefined}
>
  ...
</section>
```

#### 3. CSS module — two-column grid + accent ring

**File**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css`
**Changes**: Add new rules and keep the existing `.tiers / .panel /
.absent / .panelHeader / …` rules intact.

```css
.twoColumn {
  display: grid;
  grid-template-columns: minmax(0, 24rem) minmax(0, 1fr);
  gap: var(--sp-5);
  align-items: start;
}
.panel[data-active="true"] {
  outline: 2px solid var(--ac-accent);
  outline-offset: 2px;
}
/* No .previewPaneSlot rule — Phase 6 adds .previewPane directly. */
```

`outline` is used instead of `border` so the ring doesn't alter card
geometry. `outline-offset: 2px` keeps it visually separate from the
existing `.panel` border.

### Success Criteria:

#### Automated Verification:

- [ ] All detail route tests pass:
      `npm test --prefix skills/visualisation/visualise/frontend -- LibraryTemplatesView`
- [ ] Typecheck passes
- [ ] The CSS regression assertion that `.activeBadge` is absent (line
      84-86 of the existing test) still holds.

#### Manual Verification:

- [ ] At ≥1024px viewport, the detail screen shows a 24rem + 1fr grid
      with the tier cards on the left and an empty right column. The
      active tier card has a visible accent-coloured ring around it.
- [ ] Non-active cards have no ring (only the existing soft border).

---

## Phase 6: Frontend — preview pane + content-hash label + live update

### Overview

Fill the right column with a `TemplatePreviewPane` that renders the
winning tier's path + content-hash label as its first row, and the
winning tier's rendered markdown body below. The label is fully
non-interactive and updates live via SSE within 1 second of a
`template-changed` event.

### Changes Required:

#### 1. TDD: route tests for the preview pane

**File**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.test.tsx`
**Changes**: Add a fixture variant that carries `sha256`, and add
focused tests.

```typescript
import { sha256 as sha256Bytes } from '@noble/hashes/sha256'  // or equivalent
import { bytesToHex } from '@noble/hashes/utils'

// Compute the digest from real fixture content so AC11 ("label
// value equals backend sha256") is asserted end-to-end rather than
// against a literal placeholder.
function digestForContent(content: string): string {
  return `sha256-${bytesToHex(sha256Bytes(new TextEncoder().encode(content)))}`
}

// mockDetail's winning tier is plugin-default with content '# ADR\nBody.'
const winningContent = '# ADR\nBody.'
const expectedDigest = digestForContent(winningContent)

const mockDetailWithSha: TemplateDetail = {
  ...mockDetail,
  sha256: expectedDigest,
}

it('renders the content-hash label with the digest computed from the winning content (AC11)', async () => {
  vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetailWithSha)
  render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
  // The label text equals the digest derived in this test from the
  // same content the resolver would hash on the backend.
  expect(await screen.findByText(expectedDigest)).toBeInTheDocument()
})

it('renders the content-hash label as the first row of the preview pane', async () => {
  vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetailWithSha)
  render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
  await screen.findByText(expectedDigest)
  const label = screen.getByText(expectedDigest)
  const body = await screen.findByText('Body.')
  // Use compareDocumentPosition rather than firstElementChild so
  // benign wrapper nodes don't break the test. The user-observable
  // contract is "label appears before body in document order".
  // eslint-disable-next-line no-bitwise
  expect(
    label.compareDocumentPosition(body) & Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy()
})

it('renders the winning-tier path alongside the content-hash label', async () => {
  vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetailWithSha)
  render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
  await screen.findByText(expectedDigest)
  // mockDetail's winning tier path is '/plugin/templates/adr.md'
  expect(screen.getByText('/plugin/templates/adr.md')).toBeInTheDocument()
})

it('renders the winning-tier markdown body below the content-hash label', async () => {
  vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetailWithSha)
  render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
  // Body comes from mockDetail.tiers (plugin-default winning, content '# ADR\nBody.')
  expect(await screen.findByText('Body.')).toBeInTheDocument()
})

it('omits the content-hash label when sha256 is absent from the detail', async () => {
  vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail) // no sha256
  render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
  await screen.findByText('active')
  expect(screen.queryByText(/^sha256-/)).toBeNull()
})

it('omits the content-hash label when an SSE event clears sha256 (AC10 live path)', async () => {
  // First fetch returns hash; after SSE invalidation, fetch returns
  // detail without sha256 (winning content emptied). Label must
  // disappear without a page reload.
  const first = { ...mockDetailWithSha }
  const cleared = { ...mockDetail, sha256: undefined }
  vi.spyOn(fetchModule, 'fetchTemplateDetail')
    .mockResolvedValueOnce(first)
    .mockResolvedValueOnce(cleared)

  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  // ... render, wait for first hash ...
  dispatchSseEvent(
    { type: 'template-changed', template: 'adr', timestamp: '2026-05-18T00:00:00Z' },
    qc,
  )
  await waitFor(() => expect(screen.queryByText(/^sha256-/)).toBeNull(), { timeout: 1_000 })
})

it('content-hash label is non-interactive (AC13)', async () => {
  vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetailWithSha)
  render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
  const label = await screen.findByText(expectedDigest)

  // No interactive ARIA / DOM attributes
  expect(label.getAttribute('role')).toBeNull()
  expect(label.getAttribute('tabindex')).toBeNull()
  expect(label.getAttribute('title')).toBeNull()

  // Active click test — catches React synthetic onClick that
  // `label.onclick === null` would miss. No observable side-effect
  // should occur: no clipboard write, no navigation, no toast.
  const writeSpy = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue()
  await userEvent.click(label)
  expect(writeSpy).not.toHaveBeenCalled()

  // CSS regressions: no :hover rule, no cursor: pointer/copy.
  // Anchored to the .contentHashLabel selector specifically.
  expect(templatesCss).not.toMatch(/\.contentHashLabel\s*\{[^}]*cursor:\s*pointer/m)
  expect(templatesCss).not.toMatch(/\.contentHashLabel\s*\{[^}]*cursor:\s*copy/m)
  expect(templatesCss).not.toMatch(/\.contentHashLabel:hover/)
})

it('updates the content-hash label end-to-end via dispatchSseEvent (AC12)', async () => {
  // Drive the *real* dispatchSseEvent so this test catches reducer
  // regressions (wrong key, missing branch). The reducer-unit test
  // covers the branch in isolation; this test joins it with the
  // route render so the full SSE → invalidate → refetch → DOM
  // path is exercised in one place.
  const second = digestForContent('# ADR\nBody. v2.')
  const first = { ...mockDetailWithSha }
  const next = { ...mockDetailWithSha, sha256: second }
  const spy = vi.spyOn(fetchModule, 'fetchTemplateDetail')
    .mockResolvedValueOnce(first)
    .mockResolvedValueOnce(next)

  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
  render(<LibraryTemplatesView name="adr" />, { wrapper })
  await screen.findByText(expectedDigest)

  // The real reducer — same code that runs in production for
  // EventSource messages.
  dispatchSseEvent({
    type: 'template-changed',
    template: 'adr',
    sha256: second,
    timestamp: '2026-05-18T00:00:00Z',
  }, qc)

  await screen.findByText(second, undefined, { timeout: 1_000 })
  expect(spy).toHaveBeenCalledTimes(2)
})
```

#### 2. Detail route — `TemplatePreviewPane`

**File**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx`
**Changes**: Replace the placeholder right-column `div` with a
`TemplatePreviewPane`, defined inline at the bottom of the file:

```tsx
// Single canonical winning-tier derivation, shared by TierPanel
// (which uses isActive) and TemplatePreviewPane. Avoids two
// definitions of "winning" drifting in the same file.
function getWinningTier(data: TemplateDetail): TemplateTier | undefined {
  return data.tiers.find(t => t.source === data.activeTier && t.present)
}

function TemplatePreviewPane({ data }: { data: TemplateDetail }) {
  const winning = getWinningTier(data)
  if (!winning) return null  // Phase-5 grid leaves the slot empty
  return (
    <div className={styles.previewPane} data-testid="template-preview-pane">
      <div className={styles.previewHeader} data-testid="template-preview-header">
        <span className={styles.previewPath}>{winning.path}</span>
        {data.sha256 ? (
          // Render the field verbatim — the wire shape is already
          // `sha256-<hex>`, matching the per-tier etag. No UI
          // prefix-prepending.
          <span
            className={styles.contentHashLabel}
            aria-label="Content hash"
          >
            {data.sha256}
          </span>
        ) : null}
      </div>
      {winning.content != null ? (
        <MarkdownRenderer content={winning.content} />
      ) : null}
    </div>
  )
}
```

> **ARIA framing**: the `aria-label="Content hash"` gives screen
> readers a short semantic frame for the 70-character hex blob
> without adding visible chrome. Compatible with AC13's
> non-interactivity (no `role`, no `tabindex`, no `title`, no click
> handler).

In the success branch, replace the placeholder right column:

```tsx
content = (
  <div className={styles.twoColumn} data-testid="templates-detail-layout">
    <div className={styles.tiers}>
      {data.tiers.map(tier => (
        <TierPanel
          key={tier.source}
          tier={tier}
          isActive={tier.source === data.activeTier}
        />
      ))}
    </div>
    <TemplatePreviewPane data={data} />
  </div>
)
```

#### 3. CSS for the preview pane and content-hash label

**File**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css`
**Changes**:

```css
.previewPane {
  border: 1px solid var(--ac-stroke-soft);
  border-radius: var(--radius-md);
  padding: var(--sp-4);
  background: var(--ac-bg-card);
}
.previewHeader {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--sp-4);
  margin-bottom: var(--sp-4);
  font-family: var(--ac-font-mono);
  font-size: var(--size-xxs);
  color: var(--ac-fg-muted);
}
.previewPath {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.contentHashLabel {
  /* Non-interactive: explicit default cursor; no hover rule. */
  cursor: default;
  user-select: text;  /* default browser behaviour for static text */
}
```

Phase 5 didn't add a `.previewPaneSlot` rule (placeholder div was
dropped); no removal needed here.

#### 4. AC11 verification — single end-to-end assertion

The Phase 1 test `template_detail_includes_sha256_of_winning_content`
asserts the wire field re-derives from fixture content. Phase 6's
"renders the content-hash label with the digest computed from the
winning content (AC11)" test re-derives the digest in the frontend
test from the same content the backend would hash. Together they
pin both halves of AC11 in matched form: the UI displays exactly the
string the backend would emit for the same content, with no
encoding divergence (the `sha256-` prefix lives on the wire and is
rendered verbatim — no UI-side prepending).

### Success Criteria:

#### Automated Verification:

- [ ] All detail route tests pass, including the live-update test:
      `npm test --prefix skills/visualisation/visualise/frontend -- LibraryTemplatesView`
- [ ] Typecheck passes
- [ ] Non-interactive CSS regressions hold:
      no `cursor: pointer` / `cursor: copy` / `:hover` rules on
      `.contentHashLabel`.

#### Manual Verification:

- [ ] On `/library/templates/adr` at ≥1024px viewport:
      - The right column shows the winning tier path on the left and
        `sha256-<64-hex>` on the right of its first row.
      - The rendered markdown body appears directly below.
      - Hovering the hash label does not change the cursor or
        background, and there is no tooltip.
      - Selecting the hash text via mouse drag still works (browser
        default), but no Copy button or click handler is present.
- [ ] Edit the winning tier file on disk (e.g.
      `echo "edited $(date)" >> meta/templates/adr.md`) and observe
      the hash label change within ~1 second without a page reload.
- [ ] If the winning tier file is emptied to 0 bytes, the hash
      label disappears (AC10).

---

## Testing Strategy

### Unit Tests

- Backend (`templates.rs` inline `#[cfg(test)] mod tests`):
  - `detail_sha256_matches_winning_tier_content_prefixed`
    (asserts `sha256-<hex>` shape)
  - `detail_sha256_omitted_when_winning_content_empty`
  - `detail_sha256_cached_at_build_time` (validates the
    precomputed-digest invariant)
- Backend (`sse_hub.rs` inline):
  - Extend `sse_payload_json_wire_format` with `TemplateChanged`
    serialisation assertions for **both** `Some(sha256)` and
    `None`-sha256 shapes (None omits the key entirely).
- Backend (`watcher.rs` inline `#[cfg(test)] mod tests`):
  - Non-template path still routes to doc-changed (regression
    guard)
  - Rapid edits coalesce into one broadcast with the final
    sha256
  - Shared tier file produces one broadcast per affected
    template
  - Doc-overlap path produces both `TemplateChanged` and
    `DocChanged` (additive precedence)
  - Empty-content transition broadcasts `TemplateChanged` with
    `sha256: None`
  - Concurrent reads during ArcSwap rebuild succeed (stress)
  - macOS canonicalisation match (gated by
    `cfg!(target_os = "macos")`)
- Frontend (`use-doc-events.test.ts`):
  - `dispatchSseEvent — template-changed` (count + deep-equal
    keys; covers `sha256?` present and absent)
  - `queryKeysForEvent — template-changed`
  - `useDocEvents` drag-deferred path: `template-changed`
    received during drag → keys invalidated at drag release

### Integration Tests

- Backend (`server/tests/api_templates.rs`):
  - `template_detail_includes_sha256_of_winning_content`
    (asserts `sha256-<hex>` shape + re-derivation cross-check)
  - `template_detail_omits_sha256_when_winning_content_empty`
  - `template_detail_omits_sha256_when_winning_tier_absent`
  - `template_detail_omits_sha256_when_winning_content_not_utf8`
- Backend (`server/tests/sse_e2e.rs`):
  - `template_file_mutation_arrives_as_template_changed_sse_event`
    (asserts exact sha256 from the new on-disk content)
  - `template_file_emptied_arrives_as_template_changed_with_no_sha256`

### Route Tests (Frontend)

- `LibraryTemplatesIndex.test.tsx`:
  - Three-chip order across the **4-row** fixture
    (no-overrides / user-only / user+config / all-three)
  - Variant matrix across the same four rows
  - CSS regression on `.winning`/`.active`
- `LibraryTemplatesView.test.tsx`:
  - Two-column layout assertion (anchored CSS selector match)
  - Active-ring on winning tier only (anchored `.panel[data-active="true"]`)
  - Active chip retained
  - Content-hash label rendering: digest computed from fixture
    content (AC11 end-to-end), present-vs-absent cases
  - Document-order assertion (label precedes body via
    `compareDocumentPosition`)
  - Path rendered alongside label
  - Body rendered below
  - Non-interactivity (no `role`/`tabindex`/`title`; userEvent
    click produces no clipboard write or other side-effect; no
    `:hover`/`cursor: pointer/copy` in anchored CSS)
  - Live update **end-to-end via `dispatchSseEvent`** (real
    reducer, not direct `invalidateQueries`)
  - Empty-content transition: SSE event with no sha256 causes
    label to disappear without a reload

### Manual Testing Steps

1. Launch the visualiser against a real project with at least one
   configured template (`config.md`) and another using only the
   plugin default.
2. Visit `/library/templates`; confirm three-chip rows render in the
   correct order, with the correct variant for each tier presence
   state.
3. Visit `/library/templates/<name>`; confirm the two-column layout,
   accent-coloured ring on the winning tier, indigo "active" chip
   retained.
4. Inspect the preview pane: path on the left, `sha256-<64-hex>` on
   the right of the first row, body below.
5. Hover the hash: cursor stays default, no tooltip, no hover bg.
   Right-click → no copy helper.
6. Edit the underlying winning-tier file from a terminal; observe
   the hash label update within ~1s without a page reload.
7. Empty the winning-tier file (`: > path/to/winning.md`); observe
   the hash label disappear.

## Performance Considerations

- `ArcSwap::load()` is lock-free and allocates only an `Arc` clone
  per call. Hot-path cost on `/api/templates*` is negligible.
- `TemplateDetail.sha256` is **precomputed at resolver build time**
  and cached on the per-name entry; `detail()` reads it in O(1)
  with no per-request hashing.
- The watcher rebuilds the full `TemplateResolver` on each
  template-tier file change. Rebuilding walks `cfg.templates` (~10
  entries × 3 tier reads + one digest per template) — bounded and
  cheap relative to file IO latency. Per-template-cost scales with
  the catalogue size, not the change; flagged as a known
  evolutionary lever in "What we're NOT doing".
- All rebuilds are serialised through a single `Notify`-driven
  consumer task in `TemplateChangeHandler` — eliminates the
  concurrent `ArcSwap::store` lost-update race. `Notify` holds a
  single pending permit; repeated `notify_one()` before the next
  `notified().await` coalesce, so sustained bursts collapse to at
  most one extra rebuild beyond the currently-running one. No
  bounded channel exists, so no overflow / silent-drop failure
  mode.
- The consumer wraps `TemplateResolver::build` in an inner
  `tokio::spawn` so panics surface as `JoinError` and the consumer
  logs + continues. Adds one extra task allocation per rebuild
  (~µs); cheap relative to the file IO the rebuild already does.
- The consumer diffs winning-tier sha256 per template against the
  previous resolver before broadcasting, so identical-bytes edits
  and non-winning-tier edits produce zero broadcasts. A single edit
  to a shared tier file produces one broadcast per template whose
  winning sha256 actually changed.
- The canonical-path index lookup is O(1) per fs event;
  `canonicalise_path_or_ancestor` adds at most a few `canonicalize`
  syscalls at startup (walking up to the nearest existing
  ancestor) and one `canonicalize` per event at runtime. For
  non-tier markdown files under a recursively-watched tier-parent
  directory, the index `has_any` check rejects them without a
  rebuild — recursive-watch cost stays negligible.
- TanStack Query refetches `templateDetail(name)` after invalidation;
  the response body includes per-tier `content` strings. No
  streaming or paging is added — acceptable for the current `O(KB)`
  template sizes; flagged as a future trim in "What we're NOT
  doing".

## Migration Notes

- No schema migration; the new `sha256` field is purely additive
  and `Option<String>`-serialised — older frontends ignore it.
- The new `template-changed` SSE event is purely additive; older
  frontends discard unknown event types (the SSE handler `try/catch`
  at `use-doc-events.ts:213-216` already protects against unknown
  variants).
- `arc-swap` is a new dependency; included in Cargo.lock by Phase 2.
- **AC7 wording in the work item** will need a follow-up edit:
  AC7 currently says `/api/library/templates/{name}`. The canonical
  resource-level mount is `/api/templates/{name}` (the rest of
  `/api/*` follows that shape; `/api/library/*` is reserved for
  the structure/aggregation index). AC7 should be reworded to
  `/api/templates/{name}` — editorial only; the endpoint and
  semantics are unchanged.
- **AC8 wording in the work item** will need a follow-up edit:
  AC8 currently says "value is the 64-character lowercase hex
  SHA-256 digest". The implementation adopts the existing
  project-wide `sha256-<hex>` etag shape on the wire (matching
  `TemplateTier.etag` and `SsePayload::DocChanged.etag`); AC8
  should be reworded to "value of the form `sha256-<64-char
  lowercase hex>`". The semantic content is unchanged; this is an
  editorial clarification, not a scope change.

## References

- Work item: `meta/work/0042-templates-view-redesign.md`
- Research: `meta/research/codebase/2026-05-18-0042-templates-view-redesign.md`
- Screenshots:
  `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/templates-view-dark.png`,
  `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/templates-view-light.png`
- Related ADRs / work items: 0028 (userspace customisation), 0029
  (template resolution model), 0033 (design tokens), 0037 (Glyph),
  0038 (Chip), 0041 (Page wrapper), 0055 (etag/SSE pattern)
- Key source pointers:
  - `skills/visualisation/visualise/server/src/templates.rs:38-44,51-107,137-149`
  - `skills/visualisation/visualise/server/src/sse_hub.rs:15-32`
  - `skills/visualisation/visualise/server/src/watcher.rs:28-98,101-170`
  - `skills/visualisation/visualise/server/src/server.rs:60,79-81,286-296`
  - `skills/visualisation/visualise/server/src/file_driver.rs:480-484,486-504`
  - `skills/visualisation/visualise/frontend/src/api/types.ts:86-141`
  - `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts:80-111`
  - `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx:32-41`
  - `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx:47-78`
