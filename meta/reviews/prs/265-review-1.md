---
type: "pr-review"
id: "265-review-1"
title: "[PP-996] Link each library document by its id so sets and inventories open the right page"
date: "2026-11-24T19:16:43+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:265"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "standards", "compatibility", "performance"]
review_number: 1
pr_number: 265
tags: []
last_updated: "2026-11-24T19:16:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #265 - [PP-996] Link each library document by its id so sets and inventories open the right page

**Verdict:** COMMENT

The PR fixes library linking at the source, not link by link. One address module (`library-document-address.ts`) now builds and resolves every library link, every call site goes through it, and a boundary test keeps it that way. The server only adds fields, and tests cover the precedence order, the round trip and the failure case at every call site. One major correctness gap remains. The resolver checks slugs before filename stems, so a stem link the app builds for a document with no `id` can open another document whose slug is that same text. The other findings are minor: the activity feed's dependency on full listings, search rows missing the older-server fallback, the same document labelled differently on two pages, and a few untested branches.

### Cross-Cutting Themes

- **Activity feed joins full listings to build links** (flagged by: performance, architecture) — The sidebar is mounted on every page, and it now keeps a live `fetchDocs(type)` query for every doc type in its 5 rows. Every `doc-changed` event therefore refetches that type's whole listing from any page. Whether a row is linked also depends on cache-invalidation order. Both lenses suggest putting the `id` (or the ready-made address) on `ActivityEvent` on the server.
- **Search rows skip the older-server fallback** (flagged by: compatibility, standards) — `IndexEntry.id` goes through `WireIndexEntry` and `normaliseEntry`, but `fetchSearch` uses the search JSON as-is. With a server that omits `relPath`, `filenameStemOf(undefined)` throws while rendering.
- **The same document gets two labels** (flagged by: code-quality, standards, architecture) — Lifecycle cards show `workItemId || documentIdOf(entry)`. Related items show `workItemId || descriptiveSlugOf(entry)`, in a local variable named `docId` that holds a slug.
- **"Document id" means three things** (flagged by: code-quality, architecture) — It can mean the raw frontmatter `id`, the link value (`id` or stem), or the route segment, which can be any of `id`, slug or stem. Only the frontend applies the rule for building the link value.
- **The forward-slash `relPath` rule is copied** (flagged by: code-quality, standards) — `search.rs` copies `watcher.rs`'s `rel_of`, while `IndexEntry.rel_path` is sent as a raw `PathBuf`. On Windows the activity feed's exact-path lookup would then match no rows. Windows is not a release target today.

### Tradeoff Analysis

- **Backward compatibility vs correct links**: Checking slugs before stems keeps bookmarked slug URLs working, but breaks the round trip for the stem links the app emits now. Checking stems before slugs protects every link the app emits and leaves old slug bookmarks as a fallback. Recommendation: protect the links the app emits.
- **Scope vs coupling**: Leaving `ActivityEvent` unchanged keeps this PR to one ticket, at the cost of the listing join and a stale-link window after a delete. Recommendation: accept that for this PR and raise a follow-up to put the address on the event.

### Strengths

- ✅ Library addressing lives in one module of small pure functions, and `LibraryDocView` and `useActiveDocRelPath` share one resolver instead of two copies that had already diverged.
- ✅ The resolver runs three separate `find` passes, so an `id` match wins whatever the listing order. The tests pin `id` over slug, slug over stem, and the round trip `resolveLibraryDocument(entries, documentIdOf(e)) === e`.
- ✅ `libraryHref` uses the router's `interpolatePath`, so wiki-link hrefs are encoded the same way as rendered `<Link>`s. A parity test guards this.
- ✅ The type-checked `LibraryLink.to` literal makes any later rename of the route path fail at compile time at every `{...libraryLink(x)}` spread.
- ✅ The boundary test works as an executable fitness function: it tests its own rules and checks that its glob is not empty.
- ✅ The server changes only add fields. `document_id` drops empty ids, `plan_id_from_entry` reuses it, and the Rust test names read as behaviour.
- ✅ The call-site tests and the e2e spec target the actual bug (two sets, two inventories, two reviews sharing a slug), and the e2e spec runs the real server path for activity events.
- ✅ The PR description names its own tradeoffs: a collision between one document's `id` and another's slug or stem, the stale-link window, the limits of a text-matching boundary test, and the hand-offs to PP-998 and PP-1003.

### General Findings

- 🔵 **Correctness**: `templates/design-inventory.md:3` (outside the diff) still says `id: "{filename-stem}" # filename without .md`. For `inventory.md` that gives `inventory`, so every inventory written from the template shares one `id` and links to the first one by path, which brings back PP-996's bug. Change the comment to the directory name.
- 🔵 **Compatibility**: Documents whose `id` differs from their stem now have a different canonical URL (`ADR-0001`, `265`, `0001`). An unquoted numeric `id` is parsed as a YAML number, dropped, and falls back to the stem without any warning.
- 🔵 **Code Quality**: The not-found recovery module (`useNearbySlugSuggestions`, `rankSlugSuggestions`, `SlugCandidate`) and its copy still say "slug" for what is now a document id.

### Additional Findings

- 🔵 `cli/visualiser/frontend/src/components/ActivityFeed/use-activity-row-documents.ts:13` — Activity rows for templates (a virtual type) are now always unlinked, and no test pins this (test-coverage)
- 🔵 `cli/visualiser/frontend/src/routes/library/LibraryTypeView.test.tsx:533` — A test asserts something is absent after a `setTimeout(0)`, so it can pass before the page renders (test-coverage)
- 🔵 `cli/visualiser/frontend/src/api/library-document-address.ts:15` — "Document id" names three different concepts (code-quality)
- 🔵 `cli/visualiser/frontend/src/api/library-document-address.ts:15` — The address rule is computed only in the browser; consider a server-side `documentId` (architecture)
- 🔵 `cli/visualiser/frontend/src/api/library-document-address.ts:37` — Checking `id` first can silently open another document from an old slug or stem URL (compatibility)
- 🔵 `cli/visualiser/frontend/src/api/library-document-address.test.tsx:175` — No test opens the page for an id containing reserved characters (test-coverage)
- 🔵 `cli/visualiser/server/src/api/search.rs:95` — The forward-slash `relPath` conversion is copied from `watcher.rs` (code-quality, standards)
- 🔵 `cli/visualiser/frontend/src/components/Sidebar/SearchResultsPanel.tsx:70` — A search row is converted to `LibraryDocument` inline in the view (code-quality)
- 🔵 `cli/visualiser/server/src/indexer.rs:151` — The bare `id` field has no doc comment, though it sits beside 3 other identity fields (architecture)
- 🔵 `cli/visualiser/frontend/src/components/RelatedArtifacts/RelatedArtifacts.test.tsx:13` — A local async `render` shadows Testing Library's `render` (standards)
- 🔵 `cli/visualiser/frontend/e2e/library-links.spec.ts:115` — `FIXTURE_META_DIR` is re-derived instead of imported from `global-setup.ts` (standards)

---
*Review generated by /review-pr*

## Inline Comments

### `cli/visualiser/frontend/src/api/library-document-address.ts:42-44` — Stem links for documents with no `id` can open another document by slug
**Severity**: major | **Confidence**: medium | **Lens**: correctness

🟡 **Correctness**

For a document with no `id`, `libraryLink` builds a link from the filename stem. `resolveLibraryDocument` checks slugs before stems, so if that stem equals another document's slug, the app's own link opens the wrong document. Example: `foo.md` and `2026-01-01-foo.md` have no `id`, and the second has slug `foo`. The link to `foo.md` is `/library/<type>/foo`, the slug pass returns `2026-01-01-foo.md`, and the click opens the wrong document. This is the bug class the PR sets out to fix. The test "a slug beats a stem" locks in this order, and the reviewer notes only cover a collision between one document's `id` and another's slug or stem.

**Suggestion**: Make every link the app emits round-trip by checking `id`, then stem, then slug. Slugs stay as a fallback for old bookmarked URLs. Add a test that round-trips `documentIdOf` for every entry in a listing whose slugs and stems collide.

---

### `cli/visualiser/frontend/e2e/global-setup.ts:62` — A snapshot from before this change restores to the wrong paths
**Severity**: minor | **Confidence**: high | **Lens**: correctness

🔵 **Correctness**

Snapshot keys changed from bare work-item filenames (`0001-x.md`) to paths under `meta/` (`work/0001-x.md`). `globalSetup` restores any leftover `.e2e-fixture-snapshot.json` without checking it. An old snapshot writes stray `meta/0001-x.md` files and leaves the edited `meta/work/*.md` files as they are, and they are then snapshotted again as the new baseline. Deleting the file by hand, as the PR description advises, is the only safeguard.

**Suggestion**: Restore only keys that appear in `editableFixtures()`, and fail loudly on any other key.

---

### `cli/visualiser/frontend/src/components/RelatedArtifacts/RelatedArtifacts.tsx:84` — The same document gets two labels; `docId` holds a slug
**Severity**: minor | **Confidence**: high | **Lens**: code-quality, standards, architecture

🔵 **Code Quality / Standards / Architecture**

`const docId = entry.workItemId || descriptiveSlugOf(entry)` holds a slug, but this PR made `documentId` mean the `id` or the stem. `LifecycleClusterView.tsx:210` labels the same entry `workItemId || documentIdOf(entry)`, so a plan reads `2026-04-18-PP-1-foo` on its lifecycle card and `foo` in a related list. The rule for which label to show now exists in two copies that can drift apart.

**Suggestion**: Choose one label rule, expose it from the address module (for example `displayedIdentifierOf(entry)`), and use it in both views. If the difference is intentional, rename `docId` to say what it holds.

---

### `cli/visualiser/server/src/indexer.rs:1590` — Frontmatter `id` is read twice in `build_entry`
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

The work-item branch already reads `raw_id` out of `parsed.state`, and `document_id(&parsed.state)` reads it again. The two copies already disagree on empty strings: `""` reaches `work_item_identity`, but `document_id` drops it. Any change to how ids are parsed, such as accepting numeric YAML ids, must be made twice.

**Suggestion**: Read the raw `id` once (for example `raw_frontmatter_id(state) -> Option<&str>`) and build both values from it.

---

### `cli/visualiser/frontend/src/routes/library/recovery/CatchAllNotFound.tsx:3` — Doc comment exceeds the 80-column limit
**Severity**: minor | **Confidence**: high | **Lens**: standards

🔵 **Standards**

Renaming the prop to `missingDocumentId` pushed this doc-comment line to about 85 columns. Biome does not reflow comments, so `mise run check` will not catch it.

**Suggestion**: Wrap ``H1 `Page not found` `` onto the next line.

---

### `cli/visualiser/frontend/src/components/ActivityFeed/use-activity-row-documents.ts:7-21` — The always-mounted activity feed keeps full listings live to build 5 links
**Severity**: minor | **Confidence**: medium | **Lens**: performance, architecture

🔵 **Performance / Architecture**

The sidebar is mounted on every page, and this hook subscribes to the full `fetchDocs(type)` listing, including frontmatter and body previews, for every doc type in its 5 rows. The type just edited is always among them, so every `doc-changed` event refetches that type's whole listing from any page. A burst of agent writes refetches once per event, and an SSE reconnect refetches them all. Whether a row is linked also depends on when `dispatchSseEvent` invalidates the listing, and that ordering is what causes the stale-link window after a delete.

**Suggestion**: Leave this for the current PR, but raise a follow-up to have the server add the `id` (or the ready-made address) to `ActivityEvent` and the `/api/activity` rows. This hook then becomes `libraryLink(row)`, with no listings involved.

---

### `cli/visualiser/frontend/src/api/fetch.ts:248-249` — Search rows skip the older-server fallback that `IndexEntry` gets
**Severity**: minor | **Confidence**: medium | **Lens**: compatibility, standards

🔵 **Compatibility / Standards**

`SearchResult` declares `id` and `relPath` as always present, but `fetchSearch` uses `r.json()` as-is. This PR handles the same gap for `IndexEntry.id` through `WireIndexEntry` and `?? null`. With a server that omits these fields (for example a Vite dev frontend proxied to an installed release), `filenameStemOf(undefined)` throws while rendering, and every row gets the React key `undefined`.

**Suggestion**: Add a `WireSearchResult` with optional `id`/`relPath` and a normaliser for it. Fall back or drop rows that have no `relPath`, and add a test with a search response from a server that predates ids.

---

### `cli/visualiser/frontend/src/api/library-document-address.ts:15-17` — Duplicate frontmatter ids make every document but the first unreachable by link
**Severity**: minor | **Confidence**: medium | **Lens**: correctness

🔵 **Correctness**

`documentIdOf` uses the raw `id` without checking that it is unique within the doc type. Two documents can share an `id`: a copied file whose `id` was never updated, or inventories that followed the template's `{filename-stem}` comment and so are all `inventory`. Both then get one URL, and the `id` pass always opens the first by path. No link ever points at the second document's stem.

**Suggestion**: Fall back to the stem when an `id` is not unique within its listing, or have the server publish `id` only when it is unique per type.

---

### `cli/visualiser/frontend/src/api/testing/library-addressing-boundary.ts:26-29` — The `.slug ===` boundary rule bans more than addressing
**Severity**: minor | **Confidence**: medium | **Lens**: architecture

🔵 **Architecture**

The rule fires on any `.slug ===` in production code. Lifecycle clusters and not-found ranking also compare slugs, legitimately. As those grow, the rule will either push unrelated logic into the address module or teach people to get round it (`x === e.slug`).

**Suggestion**: Narrow the rule to comparisons with the route segment (`documentId`, which the other two rules already cover) or with values read from `useParams`.

---

### `cli/visualiser/server/src/indexer.rs:1613-1622` — Non-string and unparsed frontmatter ids are untested
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

`document_id_tests` covers a quoted id, a missing id and an empty id. Nothing covers an unquoted `id: 42` or `id: 2026-01-01` (dropped by `as_str()`), or the `_ => None` arm for frontmatter that failed to parse. Hand-edited documents easily end up with an unquoted id, and that silently switches their URL to the stem.

**Suggestion**: Add `"---\nid: 42\n---\n"` with an assertion of today's `None`, and a malformed-frontmatter case.

---

## Per-Lens Results

### Architecture

**Summary**: Fixes library linking at the source: one address module, every call site delegating to it, a boundary test enforcing that, and server changes that only add fields. The remaining concerns are the address rule living only in the frontend, the activity feed's join over full listings, an over-broad `.slug ===` rule, and two different label rules.

**Strengths**: One module with domain names; the split resolvers removed; the boundary test as a fitness function that also keeps doc-type knowledge out of the module; the type-checked `LibraryLink.to`; `interpolatePath` without an import cycle; additive server fields normalised for older servers; tradeoffs written down.

**Comments**:
- `library-document-address.ts:15-17` (suggestion, medium) — The address rule is derived only on the client; consider a server-side `documentId`.
- `use-activity-row-documents.ts:7-18` (minor, medium) — The activity feed depends on full listings and on invalidation order.
- `library-addressing-boundary.ts:26-29` (minor, medium) — The `.slug ===` rule bans more than addressing.
- `LifecycleClusterView.tsx:210` (suggestion, medium) — The two views label the same document differently.
- `indexer.rs:151` (suggestion, low) — The bare `id` field sits beside other identity fields with no doc comment; consider `document_id` with `#[serde(rename = "id")]`.

**General Findings**: none.

### Code Quality

**Summary**: The address module of pure functions clearly improves maintainability. The concerns are naming ("document id" means three things), two label rules, a second read of `id` on the server, and the copied path conversion.

**Strengths**: A cohesive module of pure functions; one shared resolver; `path-utils.ts` deleted; `ActivityRowContent` extracted; `plan_id_from_entry` simplified.

**Comments**:
- `library-document-address.ts:15-17` (minor, medium) — "Document id" names three concepts.
- `LifecycleClusterView.tsx:210` (minor, medium) — The label falls back differently from `RelatedArtifacts`.
- `RelatedArtifacts.tsx:84` (suggestion, medium) — `docId` holds a slug.
- `SearchResultsPanel.tsx:70-74` (suggestion, medium) — Inline conversion from a search row to `LibraryDocument`.
- `indexer.rs:1590` (minor, high) — Frontmatter `id` is read twice.
- `search.rs:95` (suggestion, high) — Forward-slash conversion copied from `watcher.rs`.

**General Findings**:
- (suggestion) Not-found recovery still uses "slug" names.

### Test Coverage

**Summary**: Strong coverage for the risk: precedence order, round trip, encoding parity, failure cases at every call site, Rust id rules, and an e2e spec over the real server path. The gaps are small.

**Strengths**: Precedence tests; round-trip invariant; `libraryHref`/`<Link>` parity; behaviour-level tests at every call site; ActivityFeed pending, deleted and refetch states with the real `dispatchSseEvent`; server id rules; a test for servers that predate ids; real e2e fixtures and snapshotting; the boundary test tests itself.

**Comments**:
- `indexer.rs:1613-1622` (minor, medium) — Non-string and unparsed ids untested.
- `use-activity-row-documents.ts:13-15` (minor, medium) — Template activity rows lose their link, and no test pins it.
- `LibraryTypeView.test.tsx:533-540` (minor, medium) — A test asserts absence after a timer.
- `library-document-address.test.tsx:175-187` (suggestion, low) — No navigation test for an id with reserved characters.

**General Findings**: none.

### Correctness

**Summary**: Every call site now builds its link through one address module, which fixes sets and inventories that carry an `id`. For documents with no `id`, checking slugs before stems means a stem link can open another document. The e2e restore also trusts an old-format snapshot.

**Strengths**: Three separate passes guarantee that `id` wins; `document_id` drops empty and non-string ids; the activity feed finds entries by exact `relPath` and leaves rows unlinked rather than wrong; `libraryHref` encodes the same way as the router.

**Comments**:
- `library-document-address.ts:42-44` (major, medium) — Stem links can resolve to another document by slug.
- `library-document-address.ts:15-17` (minor, low) — Duplicate ids make every document but the first unreachable.
- `global-setup.ts:62` (minor, high) — An old snapshot restores to the wrong paths.

**General Findings**:
- (minor) The design-inventory template's `id` comment contradicts the directory-name convention.

### Standards

**Summary**: The rename to `documentId` is applied consistently, and the new modules follow the existing layout. Gaps: `docId` holds a slug, search rows skip the normalisation convention, and a doc comment exceeds 80 columns.

**Strengths**: Rename complete across routes, props and fixtures; the address module's names say what each function does; `api/testing/` and the `import.meta.glob` convention followed; `WireIndexEntry` pattern followed; `RelatedArtifacts` uses `<Link>`; Rust test names describe behaviour.

**Comments**:
- `RelatedArtifacts.tsx:84` (minor, high) — `docId` holds a slug.
- `fetch.ts:248-249` (minor, medium) — Search rows skip the normalisation convention.
- `CatchAllNotFound.tsx:3` (minor, high) — The doc comment exceeds 80 columns.
- `RelatedArtifacts.test.tsx:13-19` (suggestion, medium) — A local async `render` shadows Testing Library's.
- `library-links.spec.ts:115-118` (suggestion, low) — `FIXTURE_META_DIR` re-derived.

**General Findings**:
- (suggestion) The forward-slash `relPath` rule is spelled 3 ways across the server modules.

### Compatibility

**Summary**: The wire changes only add fields, and old slug and stem URLs still resolve, apart from the noted collision with another document's `id`. The gap is that search rows get no fallback for servers that predate the new fields.

**Strengths**: Fields added in camelCase only; `normaliseEntry` handles servers without `id`, with a test; the slug and stem passes keep old URLs working; plan identity unchanged; `relPath` uses forward slashes on the wire.

**Comments**:
- `fetch.ts:248-249` (minor, medium) — Search fields have no fallback for older servers.
- `library-document-address.ts:37-46` (suggestion, low) — Checking `id` first can silently open another document from an old URL.

**General Findings**:
- (suggestion) The canonical URL changes for documents whose `id` differs from their stem; unquoted numeric ids fall back to the stem.

### Performance

**Summary**: The cost is negligible apart from the activity feed, which now keeps full listings live from the always-mounted sidebar, so every edit on disk refetches that type's listing.

**Strengths**: The resolver's `find` passes stop early over a cached listing; the server reads `id` once; the activity lookup uses a Map; the shared query keys reuse cached listings.

**Comments**:
- `use-activity-row-documents.ts:13-21` (minor, medium) — The always-mounted feed loads and keeps live up to 5 full listings.

**General Findings**: none.
