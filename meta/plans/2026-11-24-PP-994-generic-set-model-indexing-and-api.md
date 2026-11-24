---
type: "plan"
id: "2026-11-24-PP-994-generic-set-model-indexing-and-api"
title: "Generic Set Model: Indexing and API Implementation Plan"
date: "2026-11-24T08:39:16+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "draft"
work_item_id: "work-item:PP-994"
parent: "work-item:PP-994"
derived_from: ["codebase-research:2026-11-24-PP-994-generic-set-model-indexing-and-api"]
relates_to: ["plan:2026-11-24-PP-996-library-links-by-document-id"]
tags: ["visualiser", "server", "corpus", "config", "set-model", "topic-research", "design-inventories", "doc-types"]
revision: "93dce5e9dea001b117632d41135f9f1bc88ea6c1"
repository: "accelerator"
last_updated: "2026-11-24T08:39:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Generic Set Model: Indexing and API Implementation Plan

## Overview

Some doc types store each entry as a directory: topic research (root
`manifest.md`) and design inventories (root `inventory.md`). The visualiser
server will index each such directory as a **set**: a root document, plus
the member documents and assets that the doc type's declared **shape**
names. Every index consumer gets a rule for telling a root from a member.
Change events and the activity feed follow members. A **member API** serves
a set whole.

The author chose to make doc types **fully data-driven** (planning decision
Q2, 2026-11-24). A doc type becomes a declared value, with its identity, its
facts and its rules, held in a catalogue. It is no longer a closed enum with
a `match` arm per variant. A set's shape and presentation are fields of that
declaration. Server code therefore branches on declared facts and never on a
named doc type, and test code can declare a doc type of its own through the
same mechanism the built-in types use.

The plan has two stages, as the work item prescribes:

1. **Stage one (Phase 1): decisions.** This stage writes ADR-0077 (the set
   model), ADR-0078 (doc types as declared data) and the member API
   specification. It ends when the author accepts both ADRs through
   `/review-adr` and the dated decisions line exists in PP-994's Drafting
   Notes.
2. **Stage two (Phases 2–10): implementation.** Each phase is test-first
   and mergeable on its own with `mise run` green.

## Sizing consequence and recommended split

The work item is sized **L**. That estimate assumed set declarations as
methods on the `DocTypeKey` enum. Making doc types data-driven adds a
workspace-wide refactor (Phases 3–5):

- the enum, and every per-variant `match` in `corpus` (doc-type facts, slug
  rules, cluster rules, linkage id rules), becomes declared data;
- the built-in variants become declared values, renamed across all 11
  crates that depend on `corpus`;
- the server takes its doc types from an injected catalogue.

The workspace has about 640 references to `DocTypeKey` across about 45
files. Most are in tests and mechanical, but about 90 production `match`
arms carry real rules: 50 in `corpus/src/cluster.rs`, 14 in `slug.rs`,
4 in `linkage.rs` and 12 in `api/library.rs`, plus `doc_type.rs` itself.

**This plan is therefore XL as a whole, and Phases 3–5 alone are about L.**

**Recommendation (not actioned, since the work item is not edited here):**
split Phases 3–5 into their own enabling work item, for example "Doc types
as declared data", parented to PP-868. It would carry no behaviour change,
and PP-994's implementation would depend on it.

- Phase 1's ADR-0078 is that item's decision record.
- Phase 2 (overlapping-root refusal) is independent of both and could ship
  first under either item.
- After the split, PP-994 returns to roughly its original L (Phases 6–10).
- The planning release that the decisions line gives PP-998, PP-1001,
  PP-1000, PP-134 and PP-1002 is unaffected, because it depends only on
  Phase 1.

The work item's Technical Notes size would need updating to match.

## Current State Analysis

- **Doc types are a closed enum.** `DocTypeKey` (`cli/corpus/src/doc_type.rs:9-25`)
  has 15 variants, and each fact is a `const fn` with a `match` arm per
  variant: `config_path_key`, `linkage_type_name`, `label`, `in_lifecycle`,
  `carries_target_frontmatter`, `participates_in_lifecycle`, `in_kanban`,
  `is_virtual`, `nested_manifest_filename` and `wire_str`.
- **Rules elsewhere branch on named variants:**
  - `corpus::slug::derive` (`slug.rs:9-39`);
  - the cluster-key walk, lifecycle rank and completeness marks
    (`cluster.rs:182-222, 344-386, 464-516, 551-565`);
  - linkage id resolution (`linkage.rs:212-251`);
  - the server's `PHASES` (`api/library.rs:78-114`), `facets_for`
    (`indexer.rs:62-68`) and the `WorkItems` drafts listing
    (`file_driver.rs:258-272`).
- **The only set declaration** is `nested_manifest_filename()` (`doc_type.rs:165-171`).
  It is read at `file_driver.rs:258` and `indexer.rs:1514`.
- **The server builds its roots** by iterating `DocTypeKey::all()` over a
  config-keyed path map (`file_driver.rs:112-144`, `compose.rs:139-151`).
  Wire tokens are parsed with `DocTypeKey::from_wire_str`
  (`api/mod.rs:156`, `api/library.rs:149`, `doc_type_serde.rs:17-24`).
- **The `config` crate** keeps its own `DOC_TYPES` table of linkage names
  and path keys (`cli/config/src/catalogue.rs:78-93`). It depends only on
  `kernel`, and `corpus-adapters/tests/doc_type_single_source.rs` keeps the
  two tables aligned. `config::paths::doc_type_dirs` (`paths.rs:32-69`)
  checks only for unsafe paths, and it is the one resolver both the server
  (`compose.rs:144`) and the launcher (`config_command/core/paths.rs:142`)
  use.
- **The index holds one entry per set and no members.** It is a flat map
  (`indexer.rs:247`), and listing emits one `<dir>/<root>` per set
  (`file_driver.rs:275-327`).
- **The watcher drops events** for any path that is not markdown
  (`watcher.rs:118-121`), so directory renames and asset writes trigger
  nothing.
- **The activity feed** is an append-only ring buffer
  (`activity_feed.rs:81-90`).
- **`GET /api/docs/{path}` serves any file under any root**, through an
  unindexed fallback (`api/docs.rs:75-89`).
- **Search rows** are `{docType, title, slug, mtimeMs}` (`api/search.rs:27-35`).
  `classify` matches the title, the slug and the 200-character
  first-paragraph preview (`search.rs:57-81`), and `project` drops entries
  that have no slug (`search.rs:86-94`).
- **The e2e harness and `api_smoke.rs`** remap `research_codebase` to
  `meta/research` (`frontend/e2e/start-server.mjs:82-85`,
  `server/tests/api_smoke.rs:24-27`). That nests the topics and inventory
  roots inside the Research root. The flat research fixtures are
  `server/tests/fixtures/meta/research/2026-01-01-first-research.md` and
  `2026-05-26-ac2-coverage.md`.
- **The set fixtures carry no frontmatter `id`.** All six real inventories
  carry `id: "<directory name>"`.

## Desired End State

- **Doc types are declarations.**
  - `corpus` exports a `DocType` declaration value and a `DocTypeKey`
    handle to a `&'static DocType`, compared by wire token.
  - A `DocTypeCatalogue` holds an ordered list of declarations, with
    lookup by wire token, linkage name and config path key.
    `DocTypeCatalogue::built_in()` holds today's 15 types.
  - Every fact and rule that used to be a `match` arm is a field of the
    declaration.
  - No production module outside the built-in declaration module names a
    built-in doc type.
- **The server takes its catalogue from `Config`.** It defaults to the
  built-in catalogue, and test code replaces it.
- **A set doc type declares its shape and presentation:**
  - `SetDeclaration { shape: SetShape { root, member_slots }, presentation }`;
  - `MemberSlot { pattern: SlotPattern, kind: Document | Asset }`;
  - `Presentation::SetPage | EntryPoint`.
  - Topic research and design inventories declare the shapes in the work
    item's table.
- **Configuration refuses a doc root that equals or contains another,**
  and the error names both path keys. The fixtures run on the default
  paths.
- **Every member of a set is indexed.**
  - Member documents become index records, with their frontmatter state
    and their set-relative path. Assets go in a separate asset index, with
    size and mtime.
  - Dot-prefixed paths inside a set doc root are excluded. Files that fit
    no slot are ignored.
- **The type listing, counts, facets and lifecycle clusters** hold roots
  only.
- **Search** returns member documents, matched on their own title, stem
  and first paragraph. A member row carries the set's slug and `setId` as
  link keys, plus its `setRelativePath`.
- **Related items** attribute a member's declared linkage to that member
  and keep members out of inferred items.
- **Activity** keeps one row per set, keyed on the root path. Its action
  follows the root; its time follows the latest change.
- **The library's latest time per set** counts the root, the members, and
  observed removals.
- **The doc types response** exposes each type's `set` declaration
  (shape and presentation), or `null`.
- **`GET /api/sets/{type}/{setId}`** returns the set, its root and every
  member in one response.
- **`/api/docs/{path}`** returns not-found for an asset, for an unslotted
  file in a set directory, for a member of a set that has no root, and for
  a directory.
- **ADR-0077 and ADR-0078 are accepted.**
  `meta/specs/2026-11-24-set-member-api.md` records the member API.
  PP-994's Drafting Notes carry the dated decisions line.

Verify with `mise run` (the bare default task) exiting 0, and the manual
steps under Testing Strategy.

### Key Discoveries:

- **Rules behind the `match` arms.** The arms reduce to a small set of
  named rules, so "data-driven" does not mean "stringly typed". Each rule
  becomes a domain enum on the declaration:
  - `SlugRule`: numbered prefix, work item, date-and-optional-id, review
    suffix, work-item review, verbatim stem, none;
  - `ClusterKeyRule`: work-item id, parent, target, none;
  - `LinkageIdRule`: work-item id in filename, ADR number, set directory,
    PR number, stem;
  - lifecycle rank;
  - `CompletenessMark`.
- **Test-only declarations.** `DocTypeKey` as a `&'static DocType` handle
  stays `Copy`, so call sites like `kind.label()` keep compiling. Test code
  declares a type with a `static`, which is how the genericity criterion's
  test-only type is declared "through the mechanism topic research uses".
- **The single overlap check.** `config::paths::doc_type_dirs` is the one
  choke point both the server and the launcher go through, so the overlap
  refusal is written once.
- **The router.** matchit 0.7 forbids a suffix after a catch-all
  (`api/mod.rs:33-40`). Keying the member API by `/{type}/{setId}` leaves
  room for `/derived` (PP-1002).
- **Change events for members.** Per-path change events already fall out
  of `rescan_changes` (`watcher.rs:280-317`) once members are indexed. A
  set removed with its root drops every member in the same rescan, because
  enumeration is conditional on the root.
- **Copies to unify.** The cluster recompute is copied at
  `server.rs:116-146`, `watcher.rs:228-255` and `api/docs.rs:276-311`.
  Members are excluded from one shared cluster-input function, which
  Phase 7 extracts.

## What We're NOT Doing

- **A data-driven frontend.** The frontend keeps its static `DocTypeKey`
  union and glyph maps, and changes only the types that describe
  responses. A test-only doc type is exercised against the server only.
- **User-declared doc types** in configuration. The catalogue is built in
  and replaceable only in code. ADR-0078 records this as a deliberate
  boundary.
- **Moving the `config` crate's path-key table onto `corpus`.** ADR-0069
  forbids `config` depending on `corpus`. The existing single-source test
  keeps the two aligned.
- **Member routes, the member view, and any change to what the frontend
  does with a member** (PP-998).
- **Serving asset bytes** (PP-134), **the derived set response** (PP-1002;
  its path is only reserved), and **structured sources** (PP-1000; the
  field is only reserved).
- **Reading `relates_to`** (PP-1003), and **full-body search**.
- **An `id` on activity rows or change events.**
- **Re-ranking search beyond excluding members** from the exact-key bucket
  for the set's slug.
- **Changing what frozen migrations do.** `m0007` and the others keep
  compiling against built-in keys with unchanged behaviour.

## Overlap with PP-996

PP-996's plan (`meta/plans/2026-11-24-PP-996-library-links-by-document-id.md`)
and this plan touch the same wire types and fixtures. Either may land first.

| Touch point | PP-996 | PP-994 | Rule whichever lands second follows |
|---|---|---|---|
| `IndexEntry` | adds `id: Option<String>`, the document's own frontmatter `id` | adds `set_id: Option<String>` and `set_relative_path: Option<String>` | Both fields coexist. A member's `id` is its own `id` (for PP-998); `setId` is the set's. Add the second story's fields to every `IndexEntry` literal (`test_support.rs`, `clusters.rs:242`, `indexer.rs` tests, `tests/parity.rs:184`). |
| `SearchResultRow` | adds `id` and `relPath` | adds `setId` and `setRelativePath`, and sets a member row's `slug` to the set's slug | The row's **link keys** (`slug`, and `id` once PP-996 is in) are the set's for a member row: `slug` is the set's slug and `id` is the set's `setId`. `relPath` stays the member's own. The projection reads these through one function, `link_keys_of(entry)`, which PP-994 introduces in Phase 7. If PP-996 lands second, it fills `id` from `link_keys_of` and not from `entry.id`. If PP-994 lands second, Phase 7 routes PP-996's `id` through `link_keys_of`. Either way PP-996's `libraryLink` opens the set for a member row with no frontend change. |
| Fixture `id`s | adds `id: "<dir>"` to the four topic manifests and two inventories | needs the same `id`s (Phase 2) | Whichever lands first adds them; the second finds them present. |
| Activity | looks a row's entry up in the type listing by `relPath` | keeps a set's row on the root's path | Compatible in both orders, as the work item requires. |
| Related items | links every item through `libraryLink(entry)` | adds member documents as related items | With PP-996 in, a related item naming a member links by the member's own `id` or stem. That link does not open the set until PP-998 extends the builder to members. This is an accepted interim gap, since frontend handling of members is PP-998's. Record it in PP-998 when this plan lands. |
| Fixture paths | `library-links.spec.ts` uses topics, inventories and plan reviews | Phase 2 moves `meta/research/*.md` to `meta/research/codebase/` | Neither spec names a flat research fixture. Re-run both e2e suites after the second lands. |

## Implementation Approach

- **Stage one** produces the decision records and stops for the author.
- **Stage two** runs in this order:
  1. Phase 2: the independent configuration guard.
  2. Phases 3–5: the data-driven refactor, each phase pure refactor
     pinned by characterisation tests.
  3. Phase 6: the set declaration and shape matching, which are pure
     domain code.
  4. Phase 7: the consumer rules, introduced before any member exists on
     disk so they can't regress.
  5. Phase 8: member enumeration.
  6. Phase 9: the member API.
  7. Phase 10: live update, ending with the end-to-end genericity and
     source-boundary acceptance tests.

**TDD.** Every behaviour phase starts with failing tests at the API or
domain boundary. Refactor phases start with characterisation tests written
against the current code; they must pass before and after.

---

## Phase 1: Decision records (stage one)

### Overview

Write the two ADRs and the member API specification, have the author
accept the ADRs, and record the decisions line. No code changes.

### Changes Required:

#### 1. ADR-0077: Generic set model

**File**: `meta/decisions/ADR-0077-generic-set-model-for-directory-doc-types.md`
(through `/create-adr`, status `proposed`, `parent: "work-item:PP-994"`,
`relates_to: ["adr:ADR-0067", "adr:ADR-0078"]`)

The ADR records these decisions:

- **What a set is.** A set is a root document plus members. A member is a
  file fitting a member slot of the set's shape, either a document (any
  markdown file, whatever its frontmatter state) or an asset.
- **The shape.** It is the root filename plus member slots. A slot is a
  `/`-separated, set-relative pattern whose segments are literal names or
  `*`, with an optional extension. `*` matches one visible name, never one
  starting with a dot, and there is no recursive wildcard.
- **Extension case.** A slot's file extension, in its final segment only,
  matches regardless of case, so `screenshots/a.PNG` fits
  `screenshots/*.png`. Directory segments and the rest of the name match
  case-sensitively, so a slot's paths mean exactly what they say. The
  ADR records three reasons:
  - macOS's default APFS is case-insensitive but case-preserving. A
    listing returns the stored name (`a.PNG`), so a case-sensitive match
    would silently skip such files there too.
  - Cameras, phone exports and Windows tools often write uppercase
    extensions.
  - The rule gives the same result on every platform. On a
    case-sensitive volume, `a.png` and `a.PNG` become two separate
    members, which is harmless.
- **The declared shapes:**
  - topic research: `manifest.md` with `brief.md`, `outline.md`,
    `synthesis.md`, `findings/*.md`, `findings/*.levels/*.md` and
    `reports/*.md`. These follow the research crate's reader rules.
  - design inventories: `inventory.md` with the asset slot
    `screenshots/*.png`.
- **Assets are members** with a size and an mtime. They are not search
  results and have no URL yet.
- **The two presentations.** Set page (topic research) and entry point
  (design inventories). The presentation is declared and is
  behaviour-free until PP-1001 and PP-134.
- **Exclusions.** Dot-prefixed paths inside a set directory and at a set
  doc root are excluded, and files that fit no slot are ignored. Non-set
  doc types are unaffected.
- **A directory with no root is not a set.** Deleting the root removes
  every member from the index.
- **Overlapping doc roots are refused,** with both path keys named.
- **The consumer rules:**
  - roots only in listings, counts, facets and clusters;
  - members in search on their own keys;
  - member linkage attributed to the member;
  - members kept out of inferred items;
  - one activity row per set, its action from the root;
  - the latest time includes members and observed removals.
- **The set's `id`** is the root's frontmatter `id`, or else the set
  directory name. It is carried as `setId` on index entries, search rows,
  related items and the member API, and not on activity rows or change
  events.
- **The member API** is `GET /api/sets/{type}/{setId}`. The ADR names
  `meta/specs/2026-11-24-set-member-api.md` for the full shape.
- **The contract with the research crate's topic layout and the
  inventory-design skill's layout.**

#### 2. ADR-0078: Doc types as declared data

**File**: `meta/decisions/ADR-0078-doc-types-as-declared-data.md`
(through `/create-adr`, status `proposed`, `parent: "work-item:PP-994"`,
`relates_to: ["adr:ADR-0067", "adr:ADR-0069", "adr:ADR-0077"]`)

The ADR records these decisions:

- **Declarations.** A doc type is a `DocType` declaration value: its wire
  token, config path key, linkage name, label and capability flags, plus
  declared rules (`SlugRule`, `ClusterKeyRule`, `LinkageIdRule`, lifecycle
  rank, `CompletenessMark`, library phase, facets, drafts subdirectory,
  filename filter), plus an optional `SetDeclaration`.
- **Identity.** `DocTypeKey` is a `Copy` handle to a `&'static DocType`,
  with equality and hashing by wire token.
- **The catalogue.** `DocTypeCatalogue` is ordered. Its built-in instance
  is the single source of today's types. Consumers that need runtime
  lookup (wire token, linkage name) take a catalogue; the built-in one is
  the default.
- **No production code names a built-in doc type** outside the built-in
  declaration module, apart from crates that own one type's domain (the
  research crate's topic-set commands). A boundary test enforces this.
- **Out of scope.** User-declared types and a data-driven frontend.
- **The `config` path-key table stays separate,** under ADR-0069. The
  single-source test is the guard.
- **Rejected alternatives:**
  - per-type `const fn` methods, which can't support a test-only type;
  - an injected server-only `SetDeclarations` table, which splits doc-type
    facts across two places;
  - a feature-gated test variant, which leaks into 11 crates through
    feature unification.

#### 3. Member API specification

**File**: `meta/specs/2026-11-24-set-member-api.md` (`type: spec`,
following `meta/specs/2026-04-17-meta-visualisation-design.md`)

The specification contains:

- **The endpoint.** `GET /api/sets/{type}/{setId}`:
  - `type` is a wire token;
  - `setId` is percent-decoded, matched against `setId` among the type's
    roots, first by path on a duplicate;
  - 404 when the type is not a set type or no set matches; 400 on an
    unknown type token.
- **The response:**

  ```json
  {
    "set": {
      "type": "topic-research",
      "id": "prompt-caching-economics",
      "directory": "meta/research/topics/prompt-caching-economics",
      "presentation": "set-page"
    },
    "root": {
      "kind": "document",
      "path": "manifest.md",
      "title": "Prompt caching economics",
      "frontmatter": {},
      "frontmatterState": "parsed",
      "mtimeMs": 0,
      "etag": "sha256-…"
    },
    "members": [
      {
        "kind": "document",
        "path": "findings/01-cost-model.md",
        "title": "Cost model",
        "frontmatter": {},
        "frontmatterState": "parsed",
        "mtimeMs": 0,
        "etag": "sha256-…"
      },
      {
        "kind": "asset",
        "path": "screenshots/Some File (v2).png",
        "size": 1234,
        "mtimeMs": 0
      }
    ]
  }
  ```

- **Paths.**
  - Each `path` is a plain, set-relative, `/`-separated string, never
    percent-encoded.
  - `set.directory` is project-relative.
  - Members are sorted by path, and every member is in one response with
    no paging. The largest real inventory held 35 screenshots.
  - There is no document body; bodies come from
    `/api/docs/{directory}/{path}`.
- **Extension points:**
  - a document member's structured sources attach as an optional
    `sources` field on a document object (PP-1000), and are absent until
    then;
  - the derived set response is a sibling resource at
    `GET /api/sets/{type}/{setId}/derived` (PP-1002), and is unregistered
    until then.
- **The doc types response field.** `set`:
  `{ root, documentSlots: string[], assetSlots: string[], presentation: "set-page" | "entry-point" } | null`.

#### 4. Acceptance gate

The author runs `/review-adr` on ADR-0077 and ADR-0078 until both are
`accepted`. Then append one dated decisions line to PP-994's Drafting
Notes, through `/update-work-item` or a direct edit by the author:

> - Decisions recorded, YYYY-MM-DD: ADR-0077 (the set model) and ADR-0078
>   (doc types as declared data) are accepted; the member API shape is in
>   `meta/specs/2026-11-24-set-member-api.md`.

**Phases 2–10 do not start until this line exists.**

### Success Criteria:

#### Automated Verification:

- [ ] Each new document's frontmatter is valid:
      `accelerator corpus frontmatter validate --file <path>` for both ADRs
      and the spec.
- [ ] `grep -n '^status: "accepted"' meta/decisions/ADR-0077-*.md meta/decisions/ADR-0078-*.md`
      prints two lines.

#### Manual Verification:

- [ ] The author has accepted both ADRs through `/review-adr`.
- [ ] PP-994's Drafting Notes carry the decisions line, naming ADR-0077,
      ADR-0078 and the spec.

---

## Phase 2: Refuse overlapping doc roots; run fixtures on default paths

### Overview

This phase is independent of the set model and mergeable before
everything else in stage two.

### Changes Required:

#### 1. Failing tests first

**File**: `cli/config/src/paths.rs` (inline tests, using the crate's
`test-support` config double)

- `research_codebase: meta/research` with defaults is refused. The error
  names `paths.research_codebase` and `paths.research_topics`; the
  inventories root also sits under it, and the first pair found is
  enough.
- `notes: meta/plans` (equal roots) is refused, naming `paths.notes` and
  `paths.plans`.
- `notes: meta/plansx` (a shared string prefix but no shared segment) is
  accepted.
- The defaults are accepted.

**File**: `cli/visualiser/server/tests/` (compose test, or the existing
`config` tests in `src/config.rs`)

- Composing the server with an overlapping config fails, with an error
  naming both keys.

#### 2. The refusal

**File**: `cli/config/src/paths.rs`

After resolving every `DocTypeDir`, call `refuse_overlapping_roots(&resolved)`:

```rust
fn refuse_overlapping_roots(dirs: &[DocTypeDir]) -> Result<(), ConfigError> {
    for (i, outer) in dirs.iter().enumerate() {
        for inner in &dirs[i + 1..] {
            if let Some((container, contained)) = nesting(outer, inner) {
                return Err(ConfigError::Invalid {
                    detail: format!(
                        "paths.{} ({}) overlaps paths.{} ({}); \
                         doc roots may not equal or contain one another",
                        container.path_key,
                        container.dir,
                        contained.path_key,
                        contained.dir,
                    ),
                });
            }
        }
    }
    Ok(())
}
```

`nesting` compares segment-wise on the normalised `dir` strings. It
returns the containing directory first, or either directory when the two
are equal.

#### 3. Fixtures on default paths

- Move `cli/visualiser/server/tests/fixtures/meta/research/2026-01-01-first-research.md`
  and `2026-05-26-ac2-coverage.md` into `.../meta/research/codebase/`.
- Fix any fixture that references them by path:
  `rg -n "meta/research/20" cli/visualiser`.
- Drop the `research_codebase: meta/research` override from
  `frontend/e2e/start-server.mjs:82-85` (write an empty config or none)
  and from `server/tests/api_smoke.rs:24-27`. Delete their rationale
  comments (`start-server.mjs:68-73` and `api_smoke.rs:11-14`).
- Add a quoted `id: "<directory name>"` to every topic manifest and
  inventory fixture, unless PP-996 has already added it.
- Check the aside-row visual-regression spec, which uses the
  `ac2-coverage` cluster (`frontend/e2e/navigation.spec.ts:23`). If a
  rendered path changes, update its baseline through the Docker VR task.

### Success Criteria:

#### Automated Verification:

- [ ] Config tests pass: `mise run test:unit:cli --filter 'package(config)'`
- [ ] Visualiser tests pass: `mise run test:unit:visualiser` and
      `mise run test:integration:visualiser`, including `api_smoke`.
- [ ] e2e passes: `mise run test:e2e:visualiser`
- [ ] `rg -n "research_codebase: meta/research" cli/visualiser` prints
      nothing.
- [ ] `mise run`

#### Manual Verification:

- [ ] `accelerator config path --doc-types` in a scratch repo with
      `research_codebase: meta/research` prints the refusal naming both
      keys.

---

## Phase 3: Doc types as declared data — identity and facts

### Overview

`DocTypeKey` becomes a handle to a declared `DocType`, and every fact
`doc_type.rs` answers by `match` becomes a field. This phase is a pure
refactor across the workspace.

### Changes Required:

#### 1. Characterisation test first

**File**: `cli/corpus/tests/built_in_doc_type_facts.rs` (new)

Written and committed green against the **current** enum before any
change. It holds a literal table with one row per built-in type: wire
token, config path key, linkage name, label, the five predicates, the
nested root filename, and position in `all()`. It asserts each row
against the current methods. The table stays unchanged through Phases
3–5. Only its accessor calls change mechanically, to the new field
reads.

#### 2. The declaration

**File**: `cli/corpus/src/doc_type.rs`, split into `doc_type/declaration.rs`,
`doc_type/built_in.rs`, `doc_type/catalogue.rs` and `doc_type/infer.rs`

```rust
pub struct DocType {
    pub wire_token: &'static str,
    pub config_path_key: Option<&'static str>,
    pub linkage_type_name: Option<&'static str>,
    pub label: &'static str,
    pub in_lifecycle: bool,
    pub carries_target_frontmatter: bool,
    pub participates_in_lifecycle: bool,
    pub in_kanban: bool,
    pub is_virtual: bool,
    pub nested_root_filename: Option<&'static str>,
}

#[derive(Clone, Copy)]
pub struct DocTypeKey(&'static DocType);

pub struct DocTypeCatalogue { doc_types: Vec<DocTypeKey> }
```

- `DocTypeKey` gets `PartialEq`, `Eq`, `Hash`, `Debug` and `Ord` by
  `wire_token`. It delegates each existing method (`label()`,
  `wire_str()` and so on) to the declaration, so call sites are
  unchanged.
- `DocTypeKey::declared(&'static DocType)` constructs a key.
- `built_in.rs` declares each of the 15 types as a `static` and exposes
  `DocTypeKey::PLANS`, `DocTypeKey::WORK_ITEMS` and so on as associated
  constants.
- `DocTypeCatalogue::built_in()` returns the 15 in today's `all()` order.
  It has `doc_types()`, `from_wire_str`, `from_linkage_type_name` and
  `from_config_path_key`.
- `DocTypeKey::all()`, `from_wire_str` and `from_linkage_type_name` are
  removed. Callers use the built-in catalogue, or an injected one from
  Phase 5.

#### 3. Mechanical migration

- Replace `DocTypeKey::Plans` with `DocTypeKey::PLANS` (and so on) across
  the workspace. Use a scripted rename followed by `mise run fix`.
- Replace `DocTypeKey::all()` with `DocTypeCatalogue::built_in().doc_types()`.
- Rewrite each `match` on a key in production code as `==` comparisons.
  These are transitional and are removed in Phase 4.
  - **Exception:** test code may keep `matches!` on constants only if the
    handle supports structural matching. Otherwise use `==`.
- `corpus-adapters/src/doc_type.rs::table_from_paths` iterates the
  built-in catalogue.

### Success Criteria:

#### Automated Verification:

- [ ] The characterisation test passes before and after:
      `mise run test:unit:cli --filter 'test(built_in_doc_type_facts)'`
- [ ] The whole cli workspace passes: `mise run test:unit:cli`,
      `mise run test:unit:visualiser`, `mise run test:integration:visualiser`
- [ ] `mise run cli:check` and `mise run server:check`
- [ ] Public API drift is reviewed: the cargo-public-api check for
      `corpus` (see `tasks/README.md`).
- [ ] `mise run`

#### Manual Verification:

- [ ] None beyond review: no behaviour changes.

---

## Phase 4: Doc types as declared data — rules

### Overview

Every per-type rule that branches on named types becomes a declared field,
read generically. After this phase no production module outside
`doc_type/built_in.rs` names a built-in type, apart from the allowed
exceptions in the boundary test.

### Changes Required:

#### 1. Characterisation tests first

The existing suites pin the rules: `corpus/src/slug.rs`, `cluster.rs` and
`linkage.rs` tests, and the server's `clusters.rs` tests. Add a
characterisation case wherever a type has no test:

- a slug for every built-in type;
- a lifecycle rank for every type;
- a completeness mark for every type;
- a `resolve_path_target` id for every type;
- facets for every type;
- the `PHASES` membership.

All are written green against Phase 3's code.

#### 2. Declared rules

**File**: `cli/corpus/src/doc_type/declaration.rs`. The new fields:

```rust
pub slug_rule: SlugRule,
pub cluster_key_rule: ClusterKeyRule,
pub lifecycle_rank: Option<u8>,
pub completeness_mark: Option<CompletenessMark>,
pub linkage_id_rule: LinkageIdRule,
pub cluster_anchor: bool,
pub library_phase: Option<LibraryPhase>,
pub facets: &'static [Facet],
pub listing: Listing,
```

- **`SlugRule`:** `NumberedPrefix("ADR-")`, `WorkItem`,
  `DateAndOptionalId`, `DateAndOptionalIdThenReviewSuffix`,
  `WorkItemReview`, `VerbatimStem`, `None`. `slug::derive` dispatches on
  `kind.slug_rule()`.
- **`ClusterKeyRule`:** `WorkItemId`, `Parent`, `Target`, `None`. The
  `cluster::walk` arms become one `match` on the rule.
- **`cluster_anchor`** replaces the `== WorkItems` checks at
  `cluster.rs:360, 371, 464, 488`. The `Templates` checks become
  `is_virtual()`.
- **`CompletenessMark`** names the `Completeness` field to set. The wire
  struct is unchanged.
- **`LinkageIdRule`:** `WorkItemIdInFilename`, `AdrNumber`,
  `SetDirectory`, `PrNumber`, `Stem`.
- **`LibraryPhase`:** `Define`, `Discover`, `Build`, `Ship`, `Operate`,
  `Remember`. `PHASES` becomes the ordered list of phases with their ids
  and labels. Membership is read from the catalogue in catalogue order.
  Check that this preserves today's within-phase order, and add a
  declared `library_order` only if it does not.
- **`Facet`:** `Status`, `ClusterSlug` and the others used by
  `facets_for`.
- **`Listing`:** `Flat { drafts_subdirectory: Option<&'static str>,
  filename_filter: FilenameFilter }` or `Nested { root }`.
  `nested_root_filename` folds into `Listing::Nested`. Phase 6 replaces
  `Nested { root }` with the set declaration.

#### 3. Consumers read the rules

- **`corpus`:** `slug.rs`, `cluster.rs`, `linkage.rs`.
- **Server:**
  - `facets_for` (`indexer.rs:62-68`);
  - `PHASES` (`api/library.rs`);
  - `file_driver.rs:258-272`, which dispatches on `kind.listing()`;
  - the work-item references in `indexer.rs`, `api/docs.rs` and
    `api/search.rs`, which become declared facts (`in_kanban`,
    `is_virtual`, `cluster_anchor`).
- **Other crates:** `research-cli/src/topic_set.rs` and
  `corpus-cli/src/main.rs` keep their named constant (an allowed
  exception), or read the fact if one fits.

#### 4. Boundary test

**File**: `cli/corpus-adapters/tests/doc_type_declarations_are_single_source.rs`
(new; or extend `doc_type_single_source.rs`)

- Walk every `src/**/*.rs` under `cli/` except `corpus/src/doc_type/built_in.rs`
  and the allow-list (`research-cli/src/topic_set.rs`, `migrate/src/migrations/**`).
- Strip `#[cfg(test)]` modules.
- Assert that no production code contains `DocTypeKey::` followed by an
  upper-case constant.

### Success Criteria:

#### Automated Verification:

- [ ] Characterisation and existing tests pass: `mise run test:unit:cli`,
      `mise run test:unit:visualiser`, `mise run test:integration:visualiser`
- [ ] The boundary test passes, and fails if a named constant is added to
      `cluster.rs`, checked once by hand.
- [ ] `mise run cli:check`, `mise run server:check`
- [ ] `mise run`

#### Manual Verification:

- [ ] The library sidebar shows the same phases, order and counts as
      before against this repo's `meta/`.

---

## Phase 5: The server takes an injected catalogue

### Overview

The server reads every doc type from a catalogue carried on `Config`. A
test can then declare a doc type of its own.

### Changes Required:

#### 1. Failing test first

**File**: `cli/visualiser/server/tests/declared_doc_types.rs` (new)

- A test declares
  `static NOTEBOOKS: DocType = DocType { wire_token: "notebooks", config_path_key: Some("notebooks"), label: "Notebooks", ..DocType::FLAT_DEFAULTS }`
  (a `const` base the declaration module provides).
- It builds a catalogue of the built-ins plus `NOTEBOOKS`, configures
  `doc_paths["notebooks"]` to a tempdir holding one markdown file, and
  builds the router.
- `/api/types` lists `notebooks`. `/api/docs?type=notebooks` returns the
  file. `/api/search` finds it by title.

#### 2. Injection

- `server/src/config.rs`: add
  `#[serde(skip, default = "DocTypeCatalogue::built_in_arc")] pub doc_types: Arc<DocTypeCatalogue>`.
- `LocalFileDriver::new` takes `&DocTypeCatalogue` and iterates it.
- `Indexer` reads the doc-type list from the driver or the catalogue:
  `indexer.rs:334`.
- `describe_types` and `api/library.rs` iterate `cfg.doc_types`.
- Wire-token parsing (`api/mod.rs:156`, `api/library.rs:149`) goes
  through `state.cfg.doc_types.from_wire_str`.
- `doc_type_serde` keeps `serialize`. Remove `deserialize` if nothing
  deserialises a key (`rg "Deserialize" server/src` on the structs using
  it). Otherwise deserialise through the built-in catalogue, and document
  that wire input is resolved at the handler.
- `frontmatter.rs:352`: use the catalogue.
- Add a `test_support` helper:
  `catalogue_with(extra: &[&'static DocType]) -> Arc<DocTypeCatalogue>`.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test:unit:visualiser`, `mise run test:integration:visualiser`
- [ ] `mise run server:check`
- [ ] `mise run`

#### Manual Verification:

- [ ] None: production serves the built-in catalogue unchanged.

---

## Phase 6: Set declarations and shapes

### Overview

The domain model for sets, in `corpus`: declarations, slot patterns, and
classification of a set-relative path. Topic research and design
inventories declare their shapes. The doc types response exposes the
declarations. Nothing indexes members yet.

### Changes Required:

#### 1. Failing tests first

**File**: `cli/corpus/src/set/shape.rs` (inline tests)

- **`SlotPattern::parse`:**
  - accepts `brief.md`, `findings/*.md`, `findings/*.levels/*.md` and
    `screenshots/*.png`;
  - rejects `**`, an empty segment, a leading `/` and `..`.
- **`*`:**
  - matches `01-a`, `Some File (v2)` and `01-a.levels`;
  - never matches `.x`, `.01-x-web.md.invalid` or `.conduct-run.json`;
  - never spans `/`.
- **Topic-research shape:**
  - `classify("findings/01-a.levels/2-1.md") == Some(Document)`;
  - `classify("findings/sub/x.md") == None`;
  - `classify("notes.txt") == None`;
  - `classify("manifest.md") == Some(Root)`.
- **Inventory shape:**
  - `classify("screenshots/a.png") == Some(Asset)`;
  - `prototype-full/x.html`, `assets/y.svg`, `notes.txt` and
    `screenshots/sub/a.png` are all `None`.
- **Extension case.** Write these red tests first:
  - `screenshots/a.PNG` and `screenshots/a.Png` are `Some(Asset)`;
  - `findings/01-a.MD` is `Some(Document)`;
  - for a test shape with an `img/*.jpg` slot, `img/b.JPG` is
    `Some(Asset)`;
  - a folder whose name differs only in case is not a member:
    `Screenshots/a.png` and `SCREENSHOTS/a.png` are `None`, and so are
    `findings/01-a.LEVELS/2-1.md` and `Findings/01-a.md`;
  - case-insensitivity reaches the extension and no further:
    `screenshots/a.png.bak` is `None`.
- **Characterisation.** Extend the facts test: topic research and design
  inventories are sets with the declared presentation, and no other
  built-in type is.
- **Contract with the research crate.** Every `brief.md`, `outline.md`,
  `synthesis.md`, finding and level path that
  `cli/research/src/topic/layout.rs` writes classifies as a member. Use
  its path builders if `corpus` may see them under ADR-0069. Otherwise
  use literal examples, and leave the cross-crate test to PP-1002 as the
  work item says.

**File**: `cli/visualiser/server/tests/api_types.rs` (or the existing
types test)

- `topic-research` has
  `set: { root: "manifest.md", documentSlots: [...], assetSlots: [], presentation: "set-page" }`.
- `design-inventories` has `assetSlots: ["screenshots/*.png"]` and
  `presentation: "entry-point"`.
- Every other type has `set: null`.

#### 2. The model

**File**: `cli/corpus/src/set.rs` and `set/shape.rs`

```rust
pub struct SetDeclaration {
    pub shape: SetShape,
    pub presentation: Presentation,
}

pub struct SetShape {
    pub root: &'static str,
    pub member_slots: &'static [MemberSlot],
}

pub struct MemberSlot {
    pub pattern: &'static str,
    pub kind: MemberKind,
}

pub enum MemberKind { Document, Asset }

pub enum Presentation { SetPage, EntryPoint }

pub enum SetPlace { Root, Member(MemberKind) }

impl SetShape {
    pub fn classify(&self, set_relative: &str) -> Option<SetPlace>;
}
```

- Slot patterns are parsed once, lazily, behind `classify`.
- A parsed final segment holds its name pattern and its extension
  separately. The extension is compared with `eq_ignore_ascii_case`;
  every other segment, and the name before the extension, is compared
  exactly.
- `Listing::Nested { root }` becomes `Listing::Set(&'static SetDeclaration)`.
- `DocTypeKey::set_declaration() -> Option<&'static SetDeclaration>`.
- `file_driver.rs:258` and `indexer.rs:1514` read `shape.root`.

#### 3. Doc types response

- `server/src/doc_type_view.rs`: add a `set: Option<SetView>` serialised
  as in the spec.
- `frontend/src/api/types.ts:112-123`: add `set: DocTypeSet | null` to
  `DocType`, and normalise a missing field to `null` in `fetch.ts`.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test:unit:cli`, `mise run test:unit:visualiser`,
      `mise run test:integration:visualiser`
- [ ] `mise run frontend:check`, `mise run test:unit:frontend`
- [ ] `mise run`

#### Manual Verification:

- [ ] `curl localhost:<port>/api/types` shows both declarations.

---

## Phase 7: Index consumers tell roots from members

### Overview

Index records gain set membership, and every consumer gets its rule.
Tests use fabricated member records through `test_support`, because
nothing on disk produces members yet. Listings, counts and clusters are
therefore unchanged in production. The only visible change is the new
`setId` and `setRelativePath` fields on roots.

### Changes Required:

#### 1. Failing tests first

Use `test_support` factories that build a root and member records for a
set:

- **Type listing and counts.** `all_by_type` and `counts_by_type` return
  only the root (criteria 1 and 9).
- **Library aggregates:**
  - `count` is 1;
  - `status` and `clusterSlug` facets hold the root's values only, even
    when members carry other status and slug frontmatter;
  - the set's latest time is the newest root or member `mtime_ms`, and
    the doc type's latest follows it (criterion 11, without removals).
- **Clusters.** The shared cluster-input function excludes members. A
  root's cluster and its members are the same with members present as
  absent (criterion 10).
- **Related:**
  - a member whose `parent` names work item X returns X as outbound;
  - the root and another member do not;
  - X returns the member as inbound (criterion 18);
  - no inferred item is a member, including for a member and a plan
    sharing a slug in their filenames (criterion 36).
- **Search:**
  - a phrase in a member's first paragraph returns that member, titled as
    the member, with `setRelativePath`, `setId` and `slug` equal to the
    set's slug (criterion 15);
  - searching the set's slug ranks the root first, and no member is in
    the exact-key bucket (criterion 16);
  - the set's `id` is never matched.
- **Wire fields.** `setId` is the root's `id`, or the directory name when
  the root is malformed or has none. It appears on the type listing
  entry, a member search row, and a related item naming a member
  (criterion 28, minus the member API).

#### 2. Membership on records

**File**: `server/src/indexer.rs`

```rust
pub struct SetMembership {
    pub set_id: String,
    pub set_directory: PathBuf,
    pub set_relative_path: String,
    pub place: SetPlace,
}
```

- Add `pub set: Option<SetMembership>` to `IndexEntry`. Serialise
  `setId` and `setRelativePath` flat, through `#[serde(flatten)]` on a
  wire view or explicit getters, and skip the rest.
- Add `IndexEntry::is_member()` and `is_set_root()`.
- **A member's `slug` is `None`.** This keeps it out of the cluster
  bridge, the facets and the related slug fallback. Its search key is its
  own filename stem, through a `search_key()` method:
  - for a member, the stem;
  - for anything else, `slug`.
- `set_id_of(root_frontmatter, set_directory)` is derived once per set.

#### 3. Consumer rules

- **`all_by_type`, `counts_by_type` and `library_aggregates`** count and
  facet on `!is_member()`. Aggregates' `latest` for a set root uses the
  set's `latest_change_ms`, which this phase computes as the maximum over
  the root and members. Phase 10 adds observed removals.
- **Clusters.** Extract `cluster_input(entries) -> impl Iterator<Item = &IndexEntry>`,
  which filters members, and use it at `server.rs:116-146`,
  `watcher.rs:228-255` and `api/docs.rs:276-311`. Factor the three copies
  into one function if they are identical.
- **`related.rs`:** inferred items skip members, and the slug fallback
  never matches a `None` slug, as before.
- **`api/search.rs`:**
  - `classify` uses `search_key()`;
  - `project` emits a member row via `link_keys_of(entry)`, which returns
    `(slug, id)`: for a member, the set root's slug and the `set_id`; for
    others, their own;
  - add `set_id` and `set_relative_path` to `SearchResultRow`;
  - follow the PP-996 overlap rule for `id`;
  - look up the root's slug from the index snapshot by `set_directory`.
- **Frontend types.** Add `setId: string | null` and
  `setRelativePath: string | null` to `IndexEntry` and `SearchResult`, and
  normalise missing values to `null`.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test:unit:visualiser`, `mise run test:integration:visualiser`
- [ ] `mise run frontend:check`, `mise run test:unit:frontend`
- [ ] `mise run`

#### Manual Verification:

- [ ] The library, lifecycle and search in this repo look unchanged.

---

## Phase 8: Enumerate and index set members

### Overview

The file driver walks each set directory against its shape, and the
indexer records member documents and assets. This also moves design
inventories onto the model and closes the document endpoint for files
that are not members.

### Changes Required:

#### 1. Failing tests first

Rewrite the two pinned tests:

- `topic_research_indexes_one_entry_per_set_from_its_manifest` becomes
  `topic_research_indexes_its_root_and_slotted_members`;
- `design_inventories_indexed_from_nested_directories` becomes
  `design_inventories_index_root_and_screenshots`.

Then add these tests (`indexer.rs` inline tests, tempdir layouts):

- **Real layout.** A topic set, and an inventory holding
  `screenshots/a.png`, `prototype-full/x.html`, `assets/y.svg` and
  `notes.txt`. Each is indexed with exactly the shape's members
  (criteria 3 and 8).
- **Ten members.** `manifest.md` plus ten members gives one root and ten
  member records (criterion 1).
- **Depth.** `brief.md`, `findings/01-a.md`, `findings/01-a.levels/2-1.md`
  and `reports/r.md` are indexed with those set-relative paths
  (criterion 2).
- **Malformed root.** A root with unclosed frontmatter is marked
  malformed, and its members are still indexed (criterion 4).
- **No root.** A directory with no root indexes nothing (criterion 5).
- **Malformed and absent member frontmatter.** Such members are indexed
  with those marks (criterion 13).
- **Asset records.** An asset's record carries its set-relative path,
  size and mtime. Six screenshots give six assets, including
  `Some File (v2).png` (criteria 14, 24 and 25).
- **Dot-prefixed exclusions.** `findings/.01-x-web.md.invalid`,
  `.conduct-run.json`, `.scratch/x.md`, a `.tmp-AbC123` staging file and
  a `.draft.tmp/` beside the set are not indexed (criterion 12, indexing
  half).
- **Non-set doc types.** A dot-prefixed markdown file in a non-set doc
  root is indexed as today (criterion 33).
- **Inventory listing.** It holds one entry with `inventory.md`'s title,
  status and date (criterion 26).

**File**: `server/tests/api_docs.rs`

- A member body is served (criterion 6).
- An asset path, an unslotted file in a set directory, a member of a set
  whose root is gone, and a directory path each return 404 (criteria 8
  and 24).
- `inventory.md` is served unchanged (criterion 26).

**File**: `server/tests/api_search.rs`

- `a.png` returns no result.
- `prototype-full` content is not searchable.

#### 2. Enumeration

**File**: `server/src/file_driver.rs`

- Add `list_sets(kind) -> Vec<SetListing>`, where
  `SetListing { directory, root, members: Vec<(String, MemberKind)> }`.
- For each non-dot directory under a set doc root that has a root file,
  walk it recursively (bounded by the deepest slot's segment count).
  Skip dot-prefixed names at every depth, and classify each file's
  set-relative path through `shape.classify`.
- Build relative paths with `/` separators.
- `list(kind)` for a set type returns roots and member documents, so the
  `rescan` read pipeline stays uniform. Assets come from `list_sets`
  with their metadata.
- Add `FileDriver::stat(path)` for asset size and mtime, so asset bytes
  are never read.

#### 3. Index model

**File**: `server/src/indexer.rs`

- `rescan`: for set types, build entries for the root and member
  documents with `SetMembership`.
  - The set slug comes from the set directory name, not the immediate
    parent; this replaces the `indexer.rs:1512-1521` branch.
  - Members use their own filename for `title_from`'s fallback.
- Add an `assets: HashMap<PathBuf, AssetRecord>` map, swapped with the
  others under the same ordered locks.
  `AssetRecord { doc_type, path, rel_path, set: SetMembership, size, mtime_ms }`.
- `refresh_entry` and `find_entry_for_deleted` handle member paths. If a
  member's single-path refresh is too complex, route member paths
  through a full rescan, as the watcher already does.

#### 4. Document endpoint

**File**: `server/src/api/docs.rs`

Under a set doc root, serve only an indexed document record, and 404
everything else, including directories. Elsewhere the unindexed fallback
stays as today, except that a directory returns 404 instead of 500.

#### 5. e2e

`rg -n "search" cli/visualiser/frontend/e2e` covers the specs that assert
search results. Update any count that the `prompt-caching-economics`
fixture's members now change. Check that the design-inventory listing and
page specs still pass unchanged.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test:unit:visualiser`, `mise run test:integration:visualiser`
- [ ] `mise run test:e2e:visualiser`
- [ ] `mise run`

#### Manual Verification:

- [ ] Against this repo's six real inventories, `/api/docs?type=design-inventories`
      holds six entries. Searching a finding title in a topic fixture
      returns the member and opens the set.
- [ ] Indexing time with all real inventories is not noticeably slower:
      compare `rescan` timing in the debug log before and after.

---

## Phase 9: The member API

### Overview

`GET /api/sets/{type}/{setId}`, as specified in Phase 1.

### Changes Required:

#### 1. Failing tests first

**File**: `server/tests/api_sets.rs` (new)

- **Ten members.** The root plus ten members are returned (criterion 1).
- **Depth.** The four depth paths are returned as set-relative strings
  (criterion 2).
- **Malformed root.** Its members are still returned (criterion 4).
- **Spaces in names.** `screenshots/Some File (v2).png` is returned
  unencoded, with its size and mtime (criterion 25).
- **`setId`.** The response's `set.id` is `x` for a root whose `id` is
  `x`, for a topic set and an inventory (criterion 28, member API half).
- **Malformed directory name.** A set whose root is malformed is
  addressed by its directory name.
- **Not found.** A 404 for an unknown `setId`, and for a non-set type.
  A 400 for an unknown type token.
- **No bodies.** No document object carries a `body`.
- **Deleted root.** After the root is deleted, its members return 404
  from the member API and from `/api/docs` (criterion 32, API half).

#### 2. Handler

**File**: `server/src/api/sets.rs` (new), mounted in `api::mount`
(`api/mod.rs:26-53`)

- Resolve the type through `state.cfg.doc_types`, and 404 unless
  `set_declaration()` exists.
- Percent-decode `setId`. Find the root via `indexer.set_root(type, set_id)`,
  taking the first match by path.
- Collect the documents by `set_directory` with `indexer.set_members(&dir)`,
  and the assets from the asset map. Project to the spec's wire types,
  sorted by path.
- Follow the `related_get` validation and 404 shape
  (`api/related.rs:30-57`).

#### 3. Frontend types

**File**: `frontend/src/api/types.ts`

Add `SetResponse`, `SetDocumentMember` and `SetAssetMember`. No fetcher
is added; PP-1001 adds one.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test:integration:visualiser --filter api_sets`
- [ ] `mise run server:check`, `mise run frontend:check`
- [ ] `mise run`

#### Manual Verification:

- [ ] `curl localhost:<port>/api/sets/design-inventories/2026-05-21-015231-claude-design-prototype`
      on this repo lists only `inventory.md` and its screenshots.

---

## Phase 10: Live update, activity roll-up and acceptance

### Overview

The watcher reacts to assets and directory renames inside set roots and
ignores dot paths there. Activity rolls up per set. The latest time
counts removals. The phase ends with the genericity and source-boundary
acceptance tests.

### Changes Required:

#### 1. Failing tests first

**File**: `server/src/watcher.rs` (the `Scripted` harness, `:779-889`)

- **One event per change.** Adding, changing or removing a member
  document, after one write and quiet past the debounce, emits exactly
  one event for that path, and none for the root or other members. The
  same holds for an asset (criterion 19). Use
  `rapid_writes_coalesce_to_one_event` as the template.
- **Depth.** A change to `findings/01-a.levels/2-1.md` emits one event
  (criterion 2).
- **Case-only rename.** Rename `screenshots/a.png` to `screenshots/a.PNG`
  on disk and inject events for both paths. The asset stays a member:
  - exactly one `edited` event is emitted, carrying `screenshots/a.PNG`,
    with no `deleted` or `created` event;
  - the member API lists `screenshots/a.PNG` once;
  - the set's activity row keeps its action.

  The `Scripted` harness reads the real tempdir but injects its own
  events, so this test runs on every volume. On a case-sensitive volume
  the rename still yields a removed `a.png` and an added `a.PNG` in one
  rescan, which is the case the pairing below handles.
- **Excluded files.** Writes to `.invalid`, `.conduct-run.json`,
  `.scratch/x.md` and `.draft.tmp/manifest.md`, followed by a member
  write: when the member's event arrives, none was emitted for the
  excluded files, and no activity row exists for them (criterion 12).
  Writes to `prototype-full/x.html`, `assets/y.svg` and `notes.txt` emit
  nothing (criterion 8).
- **Rename into place.** A directory event for the rename
  `.x.tmp/ → x/`, holding a root, `brief.md`, `outline.md` and
  `notes.txt`, emits three created events. Activity holds one row
  reading created, and the member API returns the root and two members
  (criterion 30).
- **Root deletion.** Deleting the root while two members remain emits
  three deleted events. The row reads deleted, the listing has no entry,
  and both members return 404 (criterion 32).
- **Non-set doc types.** A dot-prefixed file in a non-set root still
  emits (criterion 33).

**File**: `server/src/activity_feed.rs` (inline tests)

- **Roll-up.** Member changes to A, then B's change, then A's root and a
  member: one row for A, above B, with A's latest time and no member row
  (criterion 17).
- **Root edited.** If A's root was last edited, then adding one member
  and removing another reads edited, with the later time.
- **Created set.** A created set whose member changes later still reads
  created, with the new time (criterion 29).
- **No root action.** A set with no recorded root action reads edited.
- **Non-set rows.** Rows outside sets behave as today; the existing
  three-edits-three-rows test still holds.

**File**: `server/src/indexer.rs`

- **Removals.** A member removed now gives the set's latest time as the
  removal's observed time, even when the other members are older
  (criterion 34). A screenshot change bumps the latest time
  (criterion 11).

**File**: `server/tests/generic_set_model.rs` (new; criteria 20 and 23)

- **Setup.** Declare `static NOTEBOOKS` with
  `SetDeclaration { shape: SetShape { root: "index.md", member_slots: &[docs/*.md Document, img/*.png Asset] }, presentation: SetPage }`.
  Build a tempdir set holding the root, two `docs/*.md`, one
  `img/a.png`, `other/z.md` and `.hidden.md`. Run through
  `InjectedWatcher` (`tests/sse_e2e.rs`).
- **Assertions (criterion 20):**
  - the member API returns the root and three members;
  - a phrase unique to one member's first paragraph finds it;
  - the type listing holds one entry, and a member change emits one
    event;
  - activity holds one row, and the library counts one item, with the
    root's facets and a latest time that follows the member;
  - no cluster holds a member;
  - `other/z.md` and `.hidden.md` are absent everywhere;
  - every row carries `setId`, and `/api/types` marks the type as a set.
- **Sixteen members (criterion 23).** The root, ten documents (one with
  unclosed frontmatter, one with none) and six assets: one response with
  each member's frontmatter or mark, size and mtime, its set-relative
  path, and no body.

**File**: `server/tests/source_boundary.rs` (new; criterion 21)

Scan `server/src/**/*.rs`, excluding `#[cfg(test)]` modules. Assert that
no production code mentions `TOPIC_RESEARCH`, `DESIGN_INVENTORIES`,
`TopicResearch` or `DesignInventories`. After Phase 4, phases are
declared too, so there is no exception.

#### 2. Watcher

**File**: `server/src/watcher.rs`

Replace `is_markdown` with `should_rescan(path, &roots)`:

- markdown anywhere, as today;
- under a set doc root, any path whose components below the root contain
  no dot-prefixed name, including directories;
- otherwise false.

Directory events under a set root trigger a rescan, which makes renames
independent of how the OS backend reports children. `rescan_changes`
also diffs the asset map on `(size, mtime_ms)` and emits `doc-changed`
for assets, with no `etag`.

**Case-only renames.** `should_rescan` already covers these: both the old
and new paths sit under a set root and contain no dot-prefixed component,
so either event triggers a rescan.
- In `rescan_changes`, before emitting, pair each removed member path
  with an added member path in the same set directory that is equal to
  it under ASCII case folding. Emit each pair as one `edited` event for
  the new path, and drop the pair's `deleted` and `created` events.
- The pairing applies to member documents and assets alike. It runs only
  when the old path is gone from the after-snapshot, so on a
  case-sensitive volume two coexisting files `a.png` and `a.PNG` stay two
  members.
- Per-path debounce means the second path's rescan finds nothing to
  change and emits nothing.
- The removal ledger skips paired paths, because the member was renamed,
  not removed.

#### 3. Activity roll-up

**File**: `server/src/activity_feed.rs`, and `watcher::emit` (`:328-333`)

- `emit` passes each event's set context, `Option<(root_rel_path, is_root)>`,
  taken from the before or after record.
- `ActivityRingBuffer::record(event, set: Option<SetContext>)`:
  - **non-set events** push as today;
  - **set events** remove any row whose path is the root path. A root
    event updates `root_actions[root]`. Then push
    `ActivityEvent { action: root_actions.get(root).unwrap_or(Edited), path: root, doc_type, timestamp }`.
- The work stays O(n) over 50 rows under the lock, within the module's
  mutex discipline: no allocation beyond the row, and no I/O.

#### 4. Removal ledger

**File**: `server/src/indexer.rs`

- `rescan` compares the before and after member paths for each set
  directory. For each removal it records
  `last_removal_observed_ms[set_directory] = now`.
- `latest_change_ms` is the maximum of the root, the members and that
  value.
- When a set disappears, its ledger entry is cleared.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test:unit:visualiser`, `mise run test:integration:visualiser`
- [ ] Real-OS rename check: extend
      `the_os_watcher_reports_a_write_under_a_registered_directory` with
      a directory-rename case, and pass
      `mise run test:integration:visualiser-os-watcher` on macOS and in
      CI (Linux).
- [ ] Real-OS case-only rename: in the same lane, add
      `a_case_only_rename_keeps_an_asset_a_member`, which renames
      `screenshots/a.png` to `screenshots/a.PNG` through the OS watcher
      and expects one `edited` event and no `deleted` event.
  - The test first probes the tempdir: it creates `probe` and checks
    whether `PROBE` resolves to it.
  - On a case-sensitive volume (Linux CI) it returns early with a logged
    skip, because the `Scripted` test above already covers the pairing
    there.
  - It runs on macOS's default APFS, where the OS reports the rename
    with the stored name.
- [ ] `mise run test:e2e:visualiser`
- [ ] `mise run`

#### Manual Verification:

- [ ] With `mise run dev:up`, running `/research-topic brief` in a scratch
      repo shows one created activity row for the new set.
- [ ] Editing a finding moves that row to the top, still reading created.
- [ ] Copying an inventory directory into place shows one row.
      Deleting its `inventory.md` makes the row read deleted, and the
      set leaves the listing.

---

## Testing Strategy

### Unit Tests:

- **Corpus:**
  - the characterisation table of every built-in fact and rule;
  - slot-pattern parsing and matching (dot names, `*`, an extension that
    ignores case while folder names stay case-sensitive, no recursion);
  - shape classification for both declared shapes;
  - the boundary test for named built-in types.
- **Config.** Overlap refusal for equal, nested and prefix-but-not-segment
  roots, and the defaults.
- **Server:**
  - consumer rules on fabricated members;
  - enumeration on tempdir layouts, including real-inventory clutter;
  - asset records;
  - the activity roll-up's state machine;
  - the removal ledger;
  - the watcher's admission rule.

### Integration Tests:

- `api_types`, `api_docs`, `api_search`, `api_related`, `api_sets` and
  `api_library` assert every wire criterion.
- `generic_set_model.rs` holds the test-only doc type end to end.
- `sse_e2e.rs` covers renames and root deletion through `InjectedWatcher`.
- The real-OS watcher lane covers a directory rename.
- `api_smoke` and e2e run on the default paths.

### Manual Testing Steps:

1. Run `mise run dev:up` against this repo, with its six real inventories,
   and a scratch topic set.
2. Search for a finding's title. The member appears and opens the set.
3. Edit a finding. One activity row, for the set, moves to the top.
4. `curl /api/sets/topic-research/<id>` lists the root and members with
   plain paths.
5. `curl /api/docs/<inventory>/screenshots/<png>` returns 404.
6. Set `paths.research_codebase: meta/research` in a scratch repo. The
   visualiser refuses to start, naming both keys.

## Performance Considerations

- **Walk cost.** Set enumeration walks only set directories, bounded by
  the deepest slot. Real inventories' `prototype-full/` trees are
  descended at most to that depth, and are never stat'ed beyond
  classification. Prune any directory segment that no slot's
  corresponding segment can match.
- **Assets are stat'ed, not read,** so large PNGs cost nothing to index.
- **Watcher admission.** The watcher now also admits asset and directory
  events in set roots. Debounce is per path, so a renamed set triggers
  one rescan per event. The first rescan emits everything, and the rest
  are no-ops. This is acceptable at today's scale, so no coalescing is
  added.
- **Activity roll-up.** It scans at most 50 rows per event.

## Migration Notes

- **Configuration.** Users whose configuration nests doc roots will see a
  refusal naming both keys. The defaults and `m0004`'s layout nest
  nothing. Note it in the CHANGELOG.
- **No data migration.**
- **Bookmarked URLs** are unaffected. Member routes are PP-998's.

## References

- Work item: `meta/work/PP-994-generic-set-model-indexing-and-api.md`
- Research: `meta/research/codebase/2026-11-24-PP-994-generic-set-model-indexing-and-api.md`
- Overlapping plan: `meta/plans/2026-11-24-PP-996-library-links-by-document-id.md`
- Precedent ADRs: `meta/decisions/ADR-0067-kind-discriminated-corpus-schema-and-templates.md`,
  `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
- Set layouts: `skills/research/research-topic/SKILL.md`,
  `skills/design/inventory-design/SKILL.md`, `cli/research/src/topic/layout.rs`
- Patterns:
  - the handler shape in `cli/visualiser/server/src/api/related.rs:30-57`;
  - the watcher harness in `cli/visualiser/server/src/watcher.rs:779-889`;
  - the end-to-end SSE harness in `cli/visualiser/server/tests/sse_e2e.rs`.
