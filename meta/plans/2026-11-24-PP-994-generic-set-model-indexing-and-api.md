---
type: "plan"
id: "2026-11-24-PP-994-generic-set-model-indexing-and-api"
title: "Generic Set Model: Indexing and API Implementation Plan"
date: "2026-11-24T08:39:16+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:PP-994"
parent: "work-item:PP-994"
derived_from: ["codebase-research:2026-11-24-PP-994-generic-set-model-indexing-and-api"]
relates_to: ["plan:2026-11-24-PP-996-library-links-by-document-id"]
tags: ["visualiser", "server", "corpus", "config", "set-model", "topic-research", "design-inventories", "doc-types"]
revision: "93dce5e9dea001b117632d41135f9f1bc88ea6c1"
repository: "accelerator"
last_updated: "2026-11-27T02:11:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Generic Set Model: Indexing and API Implementation Plan

## Overview

Some doc types store each entry as a directory: topic research (root
`manifest.md`) and design inventories (root `inventory.md`). The visualiser
server will index each such directory as a **set**: a root document, plus
the member documents and assets that the doc type's declared **shape**
names. The index's default view holds roots only, and every consumer
that needs members asks for them by name.
Change events and the activity feed follow members. A **member API** serves
a set whole.

The author chose to make doc types **fully data-driven** (planning decision
Q2, 2026-11-24). A doc type becomes a declared value, with its identity, its
facts and its rules, held in a catalogue. It is no longer a closed enum with
a `match` arm per variant. A set listing carries the set's shape and
presentation (ADR-0078). Each doc type is declared once: by its owner in
the owner's **declarations crate** (ADR-0080), or by the corpus while it
has no owner. In this story the two set types' **set declarations**, each
a shape and a presentation, move to their owners: the domain that writes
the set declares both, in `corpus`'s terms, beside its layout constants.
Every other fact and rule of every doc type stays in the corpus's built-in
declarations, where a set type's declaration only marks it as a set. This
is the interim state ADR-0078 and ADR-0080 describe, in which a doc
type's facts stay in the corpus beside a shape its owner declares;
PP-1007 moves whole declarations to their owners. One **declarations
assembly** crate builds the complete catalogue from every declaration,
pairing each corpus-declared set type with its owner's set declaration,
and every composition root that needs a catalogue takes it from there.
Server code therefore branches on declared facts and never on a named doc
type, and test code can declare a doc type and its set declaration
through the same constructor the assembly uses.

The corpus CLI's resolver conforms to the same model in this story:
`corpus resolve`, and the research commands that resolve a topic set
through it, read a set through its shape. They stop treating rootless or
dot-prefixed directories as sets, and resolve a path to a set only when
it is the set directory, its root, a member, or a directory that can
hold members.

The plan has two stages, as the work item prescribes:

1. **Stage one (Phase 1): decisions. Complete.** ADR-0077 (the set
   model), ADR-0078 (doc types as declared data), ADR-0079 (doc roots may
   not overlap) and ADR-0080 (domain declarations crates) are accepted,
   the member API specification is written, and PP-994's Drafting Notes
   carry the decisions line dated 2026-11-26.
2. **Stage two (Phases 2–12): implementation. Next: Phase 11.** Each phase
   is test-first and mergeable on its own with `mise run` green.

## Sizing consequence and the decision not to split

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

The owner-declared shapes grow it further:

- Phase 6: two new crate roles in the crate-rule lint, three new crates
  (`research-declarations`, `design-declarations` and the `declarations`
  assembly), and four composition roots (the visualiser server,
  `corpus-cli`, `research-cli` and `migrate-cli`) taking the assembled
  catalogue, with the adapters they wire taking it by injection;
- Phase 8, the corpus resolver's conformance, a breaking CLI change with
  its own tests, goldens and skill text. It needs only Phase 7, and is
  independent of the visualiser phases (9–12).

**This plan is therefore XL as a whole, and Phases 3–5 alone are about L.**

**Decision: one work item.** A split of Phases 3–6 into their own
enabling work item (for example "Doc types as declared data", parented to
PP-868, carrying no behaviour change) was considered. On 2026-11-26 the
author decided to keep all 12 phases under PP-994. The facts that
recommendation rested on still hold:

- Phase 1's ADR-0078 and ADR-0080 are the decision records Phases 3–6
  implement.
- Phase 2 (overlapping-root refusal) is independent of the refactor and
  can ship first.
- Without Phases 3–6, the rest would be roughly the original L, plus the
  resolver's conformance (Phases 7–12).
- The planning release that the decisions line gives PP-998, PP-1001,
  PP-1000, PP-134 and PP-1002 is unaffected: it depended only on Phase 1,
  and the line has existed since 2026-11-26.

The work item's Technical Notes size (L) understates the whole plan.

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
  It is read at `file_driver.rs:258`, `indexer.rs:1534` and `indexer.rs:1652`,
  and by the corpus resolver (`corpus-adapters/src/resolve.rs:46-50`).
- **The corpus resolver walks set directories by its own rules.**
  - `resolve_document` picks `TypeShape::NestedManifest` when the type
    has a nested manifest filename (`corpus-adapters/src/resolve.rs:46-50`).
    `TypeShape` (`corpus/src/resolve.rs:23-27`) carries no root name or
    slots.
  - Slug input: `TypeDirectoryLister::entries` (`corpus-adapters/src/fs.rs:140-159`)
    keeps every directory, with no dot filter and no root-file check, and
    `nested_candidates` (`corpus/src/resolve.rs:115-130`) matches
    `slug_from_set_dir(name) == slug`. A rootless directory resolves, and
    so does an exact dotted name such as `.x.tmp`.
  - Path input (`corpus-adapters/src/resolve.rs:93-132`): after the
    outside-root check, `set_root_under` (`:134-148`) takes the first
    segment under the type directory. Any depth, any file, dot segments
    and rootless directories resolve, and a stray file directly in the
    type directory (`topics/stray.md`) is returned as a set root. Its
    comment (`:134-138`) names `findings/` and `screenshots/`.
  - Callers: `accelerator corpus resolve` (`corpus-cli/src/resolve.rs:15-24`;
    exit codes in `corpus-cli/src/main.rs:156-201`: not found 3,
    ambiguous 2, outside root 6, invalid 1, unknown type 4) and
    `research-cli`'s `topic_set::resolve_set` (`research-cli/src/topic_set.rs:28-42`),
    which maps any non-resolved outcome to `E_TOPIC_RESEARCH_UNRESOLVED`
    for the topic outstanding, end-run, node, report-path, counts and
    consultable commands.
- **The research crate already names the topic layout.**
  `research::topic::layout` (`cli/research/src/topic/layout.rs:10-15`)
  declares `MANIFEST`, `BRIEF`, `OUTLINE`, `SYNTHESIS`, `FINDINGS` and
  `REPORTS`, with path builders `Stem::finding_path`, `NoteRef::path`
  and `free_report_path`. The `.levels` suffix is a private constant in
  `research-adapters/src/topic.rs:58` and a literal in `research`'s
  `finding_path.rs` and `note_ref.rs`. The research crate depends on
  `network` only, so a shape written in `corpus` would duplicate these
  names. No `design` crate names the inventory layout in production code;
  `inventory.md` and `screenshots/` are named by the skill.
- **The crate role model** (ADR-0069) has seven roles: kernel, domain,
  adapter, technical library, composition root, test support and
  bootstrap verifier. `tasks/lint/crate_dependencies.py` judges each
  crate's normal and build dependencies from its
  `[package.metadata.accelerator]` role, context and kind, and
  `tests/unit/tasks/test_crate_dependencies.py` pins every shipped
  declaration in `_EXPECTED_DECLARATIONS`. No context depends on a product
  context, so a platform crate cannot reach `research` or `design`. The
  launcher may depend only on `kernel`, platform crates and technical
  libraries, and today builds no doc type catalogue.
- **The built-in catalogue's readers.** Besides the server,
  `corpus-adapters` (`doc_type.rs:17`, `resolve.rs:41`),
  `migrate-adapters` (`context.rs:88`) and the frozen `m0007`
  (`m0007/mod.rs:72`) look doc types up at runtime; their roots are
  `corpus-cli`, `research-cli` and `migrate-cli`. `work-cli`,
  `shortcut-cli` and the launcher use `corpus` without a lookup.
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
    `corpus::doc_type::built_in` declares today's 15 types, none of which
    has moved to an owner yet; the `declarations` assembly builds the
    complete catalogue from them and the owners' set declarations.
  - Every fact and rule that used to be a `match` arm is a field of the
    declaration.
  - No production code names a built-in doc type outside the declarations
    themselves (the corpus's built-in declaration module and the
    declarations crates) and the assembly, except code that owns one doc
    type's domain: the research topic set commands and `corpus-cli`'s ADR
    commands. A boundary test enforces this.
- **The server takes its catalogue from `Config`.** It defaults to the
  assembled catalogue, and test code replaces it.
- **Declarations crates and an assembly (ADR-0080).** The crate-rule lint
  knows the declarations and declarations assembly roles.
  `research-declarations` and `design-declarations` hold their contexts'
  layout constants and set declarations (shape and presentation), and
  `research` takes its layout from
  `research-declarations`. The `declarations` assembly exposes
  `catalogue() -> Result<Arc<DocTypeCatalogue>, CatalogueError>`, which
  the visualiser server, `corpus-cli`, `research-cli` and `migrate-cli`
  use; adapters take the catalogue by injection, and tests take the
  assembly as a dev-dependency. The launcher may depend on declarations
  crates and the assembly. Neither `research` nor `design` depends on
  `corpus`.
- **A set listing carries the set's shape and presentation.** In this
  story a set type's declaration is split, the interim state ADR-0078
  and ADR-0080 allow, and the assembly joins the parts:
  - the corpus's built-in declaration of a set type keeps its other facts
    and rules, and its listing, `Listing::Set`, only marks it as a set;
  - `corpus::set` owns the set declaration types:
    `SetDeclaration { shape, presentation }`,
    `Presentation::SetPage | EntryPoint`,
    `SetShape { root, member_slots }`,
    `MemberSlot { pattern: &'static str, kind: MemberKind }`,
    `MemberKind::Document | Asset`, `SlotPattern`, `SetMatcher` and
    `SetPlace`;
  - `research_declarations::topic::SET` and
    `design_declarations::inventory::SET` declare the two sets' shapes, as
    the work item's table gives them, built from each crate's own layout
    constants, and their presentations: a set page for topic research and
    an entry point for design inventories;
  - the assembled catalogue answers both for a set type through
    `catalogue.set_matcher(kind)`, whose `shape()` and `presentation()`
    are the owner's. When PP-1007 moves a set type's whole declaration to
    its owner, the owner declares the type with its set declaration and
    the pairing goes.
- **The catalogue holds the set declarations.**
  `DocTypeCatalogue::new(doc_types, set_declarations)` parses every slot
  pattern and refuses a duplicate declaration (a type declared twice by
  wire token, linkage name or config path key, or given two set
  declarations), a set type with no set declaration, a set declaration
  for a type that is not a set and an unparseable pattern.
  `catalogue.set_matcher(kind)` is the only production source of a
  `SetMatcher`. There is no separate registry.
- **`corpus resolve` conforms to the set model.** A set is a non-dot
  directory holding its root file. A slug resolves only such a set. A
  path resolves to its set when it is the set directory, its root, a
  member, or a directory that may contain members; dot-prefixed segments,
  unslotted files, rootless directories and stray files in the type
  directory are not found. The research commands inherit this through
  `topic_set::resolve_set`.
- **Configuration refuses a doc root that equals or contains another,**
  and the error names both path keys.
  - Every command that resolves the full set of doc roots refuses, and so
    does a single-key lookup of a doc type's root; lookups of other path
    keys and configuration-editing commands are unaffected, so the
    overlap can always be fixed.
  - `migrate` refuses before it applies or lists any migration, and a
    migration that writes an overlap stops the run unrecorded.
  - The visualiser checks its canonical roots again, refusing a symlink
    alias.
  - The fixtures run on the default paths.
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
- **The doc types response** exposes each set type's shape and
  presentation as `set`, or `null`.
- **`GET /api/sets/{type}/{setId}`** returns the set, its root and every
  member in one response.
- **`/api/docs/{path}`** returns not-found for an asset, for an unslotted
  file in a set directory, for a file in a directory that has no root,
  and for a directory.
- **ADR-0077, ADR-0078, ADR-0079 and ADR-0080 are accepted,**
  `meta/specs/2026-11-24-set-member-api.md` records the member API, and
  PP-994's Drafting Notes carry the decisions line dated 2026-11-26.
  Phase 1 achieved this.

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
- **The single overlap check.** `config::paths::doc_type_dirs` is the
  resolver every doc-root consumer goes through: the server
  (`compose.rs:144`), the launcher (`config_command/core/paths.rs:142`),
  `corpus-cli` (`config.rs:96`), `work-cli` (`config.rs:105` and
  `sync.rs:241`) and the migrate adapter (`migrate-adapters/src/context.rs:106`).
  The overlap refusal is written once there, so every caller refuses.
  Single-key lookups go through `resolve_with_fallback`
  (`research-cli/src/context.rs:97`, `corpus-cli/src/config.rs:59`, the
  launcher's `config_command/core/paths.rs:83`). Under ADR-0079 a lookup
  of a doc type's key resolves the full set first, so it refuses too. The
  frozen migrations' direct `paths.*` reads (`m0006`, `m0010`) need no
  change, because the `migrate` runner refuses before any migration
  runs.
  The migrate adapter swallows the resolver's errors today
  (`unwrap_or_default()`), so Phase 2 makes it propagate them.
- **The owner edges and their enforcement.** Set declarations live in
  declarations crates, so `research` and `design` gain no `corpus` edge.
  The new edges are `research-declarations` → `corpus`,
  `design-declarations` → `corpus`, `research` →
  `research-declarations`, the assembly → `corpus` and both declarations
  crates, and four roots → the assembly.
  - The crate-rule lint has no role for either new crate, so it learns
    both (Phase 6), with failing tests in
    `tests/unit/tasks/test_crate_dependencies.py`.
  - `cli/pup.ron` needs an imports rule per new crate, the new crates in
    the command-runner alternation, and `^research_declarations(::|$)`
    in `research_domain_imports_only_permitted`. Each rule gets a probe
    pair in `tests/integration/pup/test_import_rule.py`.
  - `tasks/public_api.py` must classify every member; the three new
    crates are pinned, with snapshots.
  - `cli/deny.toml` needs nothing: the new crates are workspace path
    crates over `kernel` and `corpus`, adding no third-party crate.
  - Adapters may not depend on the assembly, so `corpus-adapters` and
    `migrate-adapters` take the catalogue from their roots.
    `research-adapters` keeps away from `corpus-adapters` in normal
    dependencies (ADR-0069 rule 4); resolver wiring stays in
    `research-cli`.
- **The router.** matchit 0.7 forbids a suffix after a catch-all
  (`api/mod.rs:33-40`). Keying the member API by `/{type}/{setId}` leaves
  room for `/derived` (PP-1002).
- **Change events for members.** Today `rescan_changes`
  (`watcher.rs:280-317`) diffs `indexer.all()`, which Phase 9 makes
  root-only. Phase 12's `rescan_with_diff` snapshots
  `documents_including_members()` and the asset map instead, so member
  and asset events come from the same diff. A set removed with its root
  drops every member in the same rescan, because enumeration is
  conditional on the root.
- **In-app writes bypass the activity feed today.** The document PATCH
  handler broadcasts straight through `sse_hub` (`api/docs.rs:316-324`),
  not through `watcher::emit`, so its writes reach the feed only through
  the frontend's client-built rows. Phase 12 routes it through `emit`.
- **Copies to unify.** The cluster recompute is copied at
  `server.rs:116-146`, `watcher.rs:228-255` and `api/docs.rs:276-311`.
  Phase 9 extracts one `recompute_clusters`, which reads the index's
  default, root-only view.

## What We're NOT Doing

- **A data-driven frontend.** The frontend keeps its static `DocTypeKey`
  union and glyph maps. It changes the types that describe responses,
  and makes two behaviour changes the set model needs: `ActivityFeed`
  refetches `/api/activity` on each `doc-changed` event instead of
  building rows itself (Phase 12), and search rows gain a key that stays
  unique for members (Phase 9). A test-only doc type is exercised
  against the server only.
- **User-declared doc types** in configuration. The catalogue is built in
  and replaceable only in code. ADR-0078 records this as a deliberate
  boundary.
- **Deriving `config`'s doc type table from the declarations.** ADR-0080
  makes that the goal; PP-1009 does it. Until then the single-source test
  keeps the two tables aligned.
- **Member routes, the member view, and any change to what the frontend
  does with a member** (PP-998).
- **Serving asset bytes** (PP-134), **the derived set response** (PP-1002;
  its path is only reserved), and **structured sources** (PP-1000; the
  field is only reserved).
- **Reading `relates_to`** (PP-1003), and **full-body search**.
- **An `id` on activity rows or change events.**
- **Re-ranking search beyond excluding members** from the exact-key bucket
  for the set's slug.
- **The research crates' reads of their own working files.** Quarantine
  `.invalid` markers, `.1.md*.invalid` root markers and the
  `.conduct-run.json` ledger are the owner's dot-prefixed files, inside
  ADR-0077's owner boundary, and the research crates keep reading them as
  they do.
- **Registering working files with the corpus.** Owners declare no
  dot-prefixed working files; every other reader ignores them.
- **Moving whole doc type declarations to their owners.** Only layouts and
  set declarations (shape and presentation) move. Every doc type's other
  facts and rules stay in `corpus`'s built-in declarations, the interim
  state ADR-0078 and ADR-0080 describe, and doc types without an
  owning domain stay wholly in `corpus`. PP-1007 moves the owned doc
  types' whole declarations, and PP-1008 decides homes for the unowned
  ones and for templates.
- **Changing what frozen migrations do.** `m0007` and the others keep
  compiling against built-in keys with unchanged behaviour.

## Overlap with PP-996

PP-996's plan (`meta/plans/2026-11-24-PP-996-library-links-by-document-id.md`)
and this plan touch the same wire types and fixtures. Either may land first.

| Touch point | PP-996 | PP-994 | Rule whichever lands second follows |
|---|---|---|---|
| `IndexEntry` | adds `id: Option<String>`, the document's own frontmatter `id` | adds `set_id: Option<String>` and `set_relative_path: Option<String>` | Both fields coexist. A member's `id` is its own `id` (for PP-998); `setId` is the set's. Add the second story's fields to every `IndexEntry` literal (`test_support.rs`, `clusters.rs:242`, `indexer.rs` tests, `tests/parity.rs:184`). |
| `SearchResultRow` | adds `id` and `relPath` | adds `setId` and `setRelativePath`, and sets a member row's `slug` to the set's slug | The row's **link keys** (`slug`, and `id` once PP-996 is in) are the set's for a member row: `slug` is the set's slug and `id` is the set's `setId`. `relPath` stays the member's own. The projection reads these through one function, `link_keys_of(entry)`, which PP-994 introduces in Phase 9. If PP-996 lands second, it fills `id` from `link_keys_of` and not from `entry.id`. If PP-994 lands second, Phase 9 routes PP-996's `id` through `link_keys_of`. Either way PP-996's `libraryLink` opens the set for a member row with no frontend change. |
| Fixture `id`s | adds `id: "<dir>"` to the four topic manifests and two inventories | needs the same `id`s (Phase 2) | Whichever lands first adds them; the second finds them present. |
| Activity | looks a row's entry up in the type listing by `relPath` | keeps a set's row on the root's path | Compatible in both orders, as the work item requires. |
| Related items | links every item through `libraryLink(entry)` | adds member documents as related items | With PP-996 in, a related item naming a member links by the member's own `id` or stem. That link does not open the set until PP-998 extends the builder to members. This is an accepted interim gap, since frontend handling of members is PP-998's. Record it in PP-998 when this plan lands. |
| Fixture paths | `library-links.spec.ts` uses topics, inventories and plan reviews | Phase 2 moves `meta/research/*.md` to `meta/research/codebase/` | Neither spec names a flat research fixture. Re-run both e2e suites after the second lands. |

## Implementation Approach

- **Stage one** produced the decision records and is complete: all four
  ADRs are accepted and the decisions line is dated 2026-11-26.
- **Stage two** runs in this order:
  1. Phase 2: the independent configuration guard.
  2. Phases 3–5: the data-driven refactor, each phase pure refactor
     pinned by characterisation tests.
  3. Phase 6: the declarations roles in the crate-rule lint, the two
     declarations crates and the assembly, and the composition roots
     taking the assembled catalogue, a pure refactor.
  4. Phase 7: the set declaration types and the catalogue's pairing in
     `corpus`, the owners' set declarations (shape and presentation) in
     their declarations crates, and the doc types response.
  5. Phase 8: the corpus resolver reads sets through their shape. It
     needs only Phase 7 and is independent of Phases 9–12.
  6. Phase 9: the consumer rules, introduced before any member exists on
     disk so they can't regress.
  7. Phase 10: member enumeration.
  8. Phase 11: the member API.
  9. Phase 12: live update, ending with the end-to-end genericity and
     source-boundary acceptance tests.

**TDD.** Every behaviour phase starts with failing tests at the API or
domain boundary. Refactor phases start with characterisation tests written
against the current code; they must pass before and after.

---

## Phase 1: Decision records (stage one)

### Overview

Write the four ADRs and the member API specification, have the author
accept them, and record the decisions line. No code changes.

**Status: complete.** All four ADRs are `accepted`, and the decisions
line dated 2026-11-26 is in PP-994's Drafting Notes. The summaries below
match the accepted ADRs; the code names in them are this plan's, not
the ADRs'.

### Changes Required:

#### 1. ADR-0077: Generic set model

**File**: `meta/decisions/ADR-0077-generic-set-model-for-directory-doc-types.md`
(through `/create-adr`, status `accepted`, `parent: "work-item:PP-994"`,
`relates_to: ["adr:ADR-0067", "adr:ADR-0068", "adr:ADR-0069", "adr:ADR-0078", "adr:ADR-0079", "adr:ADR-0080"]`)

The ADR records the domain decision only. Declaration and catalogue
mechanics are in ADR-0078, where declarations live in ADR-0080, the
overlap refusal in ADR-0079, and the visualiser's behaviour in the
specification.

- **What a set is.** A directory doc type's entry is a document set: a
  root document plus members, where a member is a file fitting a member
  slot of the doc type's declared shape. A slot admits documents (any
  markdown file, whatever its frontmatter state) or assets.
- **No root, no set.** A directory without its root file is not a set
  and has no members. Removing the root removes the set.
- **Outside the shape is outside the set.** A file that fits no slot is
  not part of any set.
- **Identity.** A set's `id` is its root's frontmatter `id`, or else its
  directory's name.
- **The owner declares the shape.** The domain that writes a set declares
  its shape from its own layout and registers it with the corpus, so the
  shape and the writer cannot drift apart.
- **The shape is the contract.** Every reader other than the owner
  decides membership through the shape. The visualiser and the corpus
  CLI's resolver (ADR-0068) both conform in this story.
- **The dot prefix is the owner's boundary.** Anything dot-prefixed at a
  set doc root or inside a set directory (temporary set directories,
  quarantined files, run ledgers) belongs to the owner, which keeps
  reading it, and is invisible to every other reader, whether or not it
  fits a slot. Owners do not declare these working files.
- **Doc roots may not overlap** (ADR-0079), so every member belongs to
  exactly one set doc type.
- **Consequences recorded:** listings, counts and lifecycle clusters keep
  to roots; the resolver stops treating a rootless or dot-prefixed
  directory as a set, and resolves a path to a set only when it is the
  set directory, its root, a member or a directory that can hold
  members, so some inputs that resolve today stop resolving; and the
  owners, the research and design contexts, gain a dependency on
  `corpus` through their declarations crates (ADR-0080).
- **Standalone.** The ADR names no work item, plan or research document
  in its body or references.
- **Rejected alternatives:** each reader walking the directory its own
  way; a deny-list that admits every file not excluded; and an
  allow-list shape declared in the corpus, kept aligned with each
  owner's layout only by a contract test.

#### 2. ADR-0078: Doc types as declared data

**File**: `meta/decisions/ADR-0078-doc-types-as-declared-data.md`
(through `/create-adr`, status `accepted`, `parent: "work-item:PP-994"`,
`relates_to: ["adr:ADR-0067", "adr:ADR-0069", "adr:ADR-0077", "adr:ADR-0080"]`)

The ADR records these decisions:

- **Declarations.** A doc type is a `DocType` declaration value: its wire
  token, config path key, linkage name, label and capability flags, plus
  declared rules (`SlugRule`, `ClusterKeyRule`, `LinkageIdRule`, lifecycle
  rank, `CompletenessMark`, library phase, facets), plus a `Listing`
  rule: flat (drafts subdirectory, filename filter) or set. A set listing
  carries the set's shape and presentation. Each rule is one of a closed,
  named set of choices, so a declaration is typed data, not strings.
- **Set shapes.**
  - A shape is a root filename plus member slots. A slot is a
    `/`-separated, set-relative pattern whose segments are literal names
    or `*`, with an optional extension. `*` matches one name that does
    not start with a dot, and there is no recursive wildcard.
  - A slot's extension, in its final segment only, matches regardless of
    case. The ADR records three reasons: APFS is case-insensitive but
    case-preserving; cameras, phone exports and Windows tools write
    uppercase extensions; and the rule gives the same result on every
    platform.
  - The shape types live in `corpus`. The domain that writes a set
    declares its shape in them, from its own layout: topic research
    (owner research) is `manifest.md` with `brief.md`, `outline.md`,
    `synthesis.md`, `findings/*.md`, `findings/*.levels/*.md` and
    `reports/*.md`, following the research domain's reader rules; design
    inventories (owner design) are `inventory.md` with the asset slot
    `screenshots/*.png`.
  - The presentation is a set page (topic research) or an entry point
    (design inventories). It is declared data and changes nothing until
    PP-1001 and PP-134 read it.
- **Assembling the catalogue.** The corpus defines what a doc type
  declaration and a shape are. Each doc type is declared once: by its
  owner, in the owner's declarations crate (ADR-0080), or by the corpus
  while it has no owner. A set type's shape is part of its declaration.
  The declarations assembly builds the complete catalogue from every
  declaration. Building it refuses a duplicate declaration, a set type
  with no shape, a shape for a type that is not a set and an unparseable
  slot pattern, so a bad declaration fails at startup. There is no
  separate shape registry. Tests build a catalogue the same way.
- **The interim, in this plan.** Until a doc type moves to its owner, its
  facts stay in the corpus beside a shape its owner declares (the ADR's
  neutral consequence). In PP-994 no whole declaration moves: the owners
  declare each set's shape and presentation together, as the set
  listing's half, and the corpus's declaration of the type marks it as a
  set and keeps its other facts. The assembly pairs the two, and the
  refusals apply to the pairing: a set type with no owner set
  declaration, a set declaration for a type that is not a set, a type
  given two set declarations (a duplicate declaration) and an
  unparseable slot pattern.
- **Names (this plan's).** The declaration is `corpus::DocType`. The
  server's existing doc types response view is renamed `DocTypeView` so
  the two never meet under one name. Each fact keeps one name from
  Phase 3 onward: `wire_token` with a `wire_token()` accessor, replacing
  `wire_str()`.
- **Identity.** A cheap, copyable key points at its declaration and
  compares by wire token: in this plan, `DocTypeKey`, a `Copy` handle to
  a `&'static DocType`, with equality and hashing by wire token and a
  pointer-equality fast path. Existing call sites keep their shape, and
  tests declare a type the way built-in types are declared.
- **The catalogue.** `DocTypeCatalogue` is ordered. The assembled
  catalogue is the single source of today's types. Consumers that need
  runtime lookup (wire token, linkage name) are given a catalogue, and
  tests substitute their own. Building a catalogue also refuses
  duplicate wire tokens, linkage names and config path keys, so a bad
  declaration fails at construction and never at first use.
- **Built-in declarations name every field.** A struct-update base
  (`DocType::FLAT_DEFAULTS`) exists for test-only declarations; the
  built-in module never uses it, so adding a built-in type forces a
  choice for every rule.
- **No production code names a built-in doc type** outside the
  declarations themselves, in the corpus and the declarations crates,
  and the assembly that combines them. The exception is code that owns
  one doc type's domain, such as the research domain's topic set
  commands and `corpus-cli`'s ADR commands. A boundary test enforces
  this.
- **Accepted exception: typed linkage references.** `TypedRef`'s
  `work-item:`, `plan:`, `adr:` and `pr:` forms, and the work-item slug
  and identity helpers, encode the linkage grammar of ADR-0034, not
  per-type facts. They stay as they are; a test-only type takes part in
  linkage through `TypedRef::Path`.
- **Root resolution stays bound to `config`.** Built-in types resolve
  their roots through `config::paths::doc_type_dirs`; a test-only type's
  root is supplied directly on the server's `Config`.
- **Boundaries.** Doc types are declared in code only, never in
  configuration; the frontend keeps its own static list; frozen
  migrations keep working against built-in doc types with unchanged
  behaviour.
- **Configuration's doc type table** is derived from the same
  declarations under ADR-0080 (PP-1009); until a doc type moves,
  `config` keeps its entry in a table of its own and the single-source
  test is the guard.
- **Rejected alternatives:**
  - per-type `const fn` methods, which can't support a test-only type;
  - an injected server-only `SetDeclarations` table, which leaves the
    corpus resolver with its own rules;
  - a feature-gated test variant, which leaks into 11 crates through
    feature unification.

#### 3. ADR-0079: Doc roots may not overlap

**File**: `meta/decisions/ADR-0079-doc-roots-may-not-overlap.md`
(status `accepted`, `parent: "work-item:PP-994"`,
`relates_to: ["adr:ADR-0068", "adr:ADR-0077", "adr:ADR-0078"]`)

The ADR records these decisions:

- **The refusal.** A configuration in which one doc root equals or
  contains another is refused, naming both `paths.*` keys.
- **Where.** In configuration's shared resolver for the full set of doc
  roots (`config::paths::doc_type_dirs`), so every command that resolves
  that set refuses: the visualiser, the configuration commands
  (`accelerator config paths --doc-types`), the corpus and work commands,
  and `migrate`. `migrate` refuses before it applies or lists any
  migration, and a migration that writes an overlapping configuration
  stops the run and is not recorded as applied.
- **Comparison** by path segments after normalisation, with no
  filesystem access: `meta/plans/` and `./meta/plans` equal
  `meta/plans`, and `meta/plansx` does not overlap it.
- **Symlinks.** The visualiser applies the same rule again to the real
  directories it serves from, so a symlink alias is refused too.
- **Single-key lookups check too.** A lookup of one doc type's root
  resolves the full set and refuses the same way; lookups of path keys
  that are not doc roots are unaffected.
- **Correction stays possible.** Commands that edit configuration never
  resolve doc roots, so an overlap can always be fixed.
- **Consequences recorded:** the visualiser's end-to-end and API smoke
  tests move to the default paths; frozen migrations that read one root
  directly need no change; reading the doc roots inside a migration can
  now fail, and migrations pass that failure on.
- **Rejected alternatives:** longest-match attribution, and refusing
  only a set root nested inside another root.

#### 4. ADR-0080: Domain declarations crates

**File**: `meta/decisions/ADR-0080-domain-declarations-crates.md`
(through `/create-adr`, status `accepted`, `parent: "work-item:PP-994"`,
`relates_to: ["adr:ADR-0069", "adr:ADR-0077", "adr:ADR-0078"]`)

The ADR records these decisions:

- **A declarations role.** A context's declarations crate,
  `<context>-declarations`, holds what the context declares to
  cross-cutting platform crates: its doc types, with their layouts, set
  shapes and configuration path keys. It holds declared data and no
  behaviour, and depends only on `kernel` and platform domain crates.
  Any crate in its own context, any composition root and the launcher
  may depend on it.
- **A declarations assembly.** One crate, `declarations`, combines every
  context's declarations into the complete registrations each platform
  crate needs. It depends on platform domain crates and declarations
  crates, and only composition roots, the launcher and tests depend on
  it. It is the single place a new declarations crate is added.
- **One declaration per doc type.** A doc type is declared once, by its
  owner. The assembly derives both the complete corpus catalogue and
  configuration's doc type table (path keys, defaults, linkage names)
  from those declarations; configuration receives its table and gains
  no dependency on the corpus. Deriving configuration's table is
  PP-1009, not this plan.
- **Domains depend on their declarations.** A domain crate takes the
  facts it shares with platform crates, such as its layout, from its own
  context's declarations crate, so the two cannot drift.
- **ADR-0069.** The role table gains both roles and its inward rule
  extends, since a domain crate may also depend on its own context's
  declarations crate. The launcher rule is partially superseded: the
  launcher may also depend on declarations crates and the assembly, and
  its limit to platform crates otherwise stands. ADR-0069 stays
  `accepted` and unedited, after the precedent it set for ADR-0054.
- **Enforcement.** The crate-rule lint learns both roles.
- **First crates.** `research-declarations` and `design-declarations`,
  holding the topic research and design inventory layouts and set
  shapes. Doc types move to their owners' declarations crates
  incrementally; until a doc type moves, its declaration stays in the
  corpus and configuration keeps its entry in its own table. Doc types
  with no owning domain crate stay declared in the corpus.
- **Rejected alternatives:** owners' domain crates holding declarations
  (every root depends on every owning domain, and the launcher cannot);
  platform crates holding every declaration (copies of product facts);
  declarations crates with no assembly (every root must register every
  domain).

#### 5. Member API specification

**File**: `meta/specs/2026-11-24-set-member-api.md`, with the same
frontmatter shape as `meta/specs/2026-04-17-meta-visualisation-design.md`.
`spec` is not a corpus doc type, so its frontmatter is not validated by
`accelerator corpus frontmatter validate`.

The specification contains:

- **The endpoint.** `GET /api/sets/{type}/{setId}`:
  - `type` is a wire token;
  - `setId` is decoded once, by the router's path extractor, and matched
    against `setId` among the type's roots, the lexically first
    directory winning on a duplicate;
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
      "setRelativePath": "manifest.md",
      "title": "Prompt caching economics",
      "frontmatter": {},
      "frontmatterState": "parsed",
      "mtimeMs": 0,
      "etag": "sha256-…"
    },
    "members": [
      {
        "kind": "document",
        "setRelativePath": "findings/01-cost-model.md",
        "title": "Cost model",
        "frontmatter": {},
        "frontmatterState": "parsed",
        "mtimeMs": 0,
        "etag": "sha256-…"
      },
      {
        "kind": "asset",
        "setRelativePath": "screenshots/Some File (v2).png",
        "size": 1234,
        "mtimeMs": 0
      }
    ]
  }
  ```

- **Paths.**
  - Each `setRelativePath` is a plain, `/`-separated string, never
    percent-encoded. It carries the same name as the `setRelativePath`
    field on index entries and search rows.
  - `set.directory` is project-relative.
  - Members are sorted by `setRelativePath`, and every member is in one
    response with no paging. The largest real inventory held 35
    screenshots.
  - There is no document body; bodies come from
    `/api/docs/{directory}/{setRelativePath}`. A consumer building that
    URL percent-encodes each path segment, because file names may hold
    `?`, `#` or `%`.
- **Change events.** A `doc-changed` event may name an asset member.
  Its `etag` is absent for an asset as well as for a deletion, so a
  missing `etag` alone does not mean the path was deleted. Change events
  carry a path and no `id`.
- **Extension points:**
  - a document member's structured sources attach as an optional
    `sources` field on a document object (PP-1000), and are absent until
    then;
  - the derived set response is a sibling resource at
    `GET /api/sets/{type}/{setId}/derived` (PP-1002), and is unregistered
    until then.
- **Members in other responses.** The visualiser rules that follow from
  the set model:
  - roots only in the type listing, counts, facets and clusters;
  - member documents in search on their own title, stem and first
    paragraph, with the set's slug and `setId` as link keys, never
    matched, and their own `setRelativePath`; no member ranks as an
    exact match for its set's slug, and assets are never results;
  - a member's declared linkage attributed to the member, members kept
    out of inferred items, and a related item that is a member carrying
    `setId`;
  - one activity row per set, keyed on the root path, its action from
    the root and its time from the latest change, with no `id`;
  - the latest time includes members, assets and removals observed in
    memory, falling back to mtimes after a restart;
  - `/api/docs/{path}` not-found for assets, unslotted files, files in a
    directory that has no root, and directories;
  - no descent into symlinked directories, and members resolving outside
    the doc roots dropped.
- **The doc types response field.** `set`:
  `{ root, documentSlots: string[], assetSlots: string[], presentation: "set-page" | "entry-point" } | null`.

#### 6. Acceptance gate

The author ran `/review-adr` on ADR-0077, ADR-0078, ADR-0079 and
ADR-0080 until all four were `accepted`, and this dated decisions line
was appended to PP-994's Drafting Notes:

> - Decisions recorded, 2026-11-26: ADR-0077 (the set model), ADR-0078
>   (doc types as declared data), ADR-0079 (doc roots may not overlap)
>   and ADR-0080 (domain declarations crates) are accepted; the member
>   API shape is in `meta/specs/2026-11-24-set-member-api.md`.

Phases 2–12 waited on this line; it exists, so Phase 2 is next.

### Success Criteria:

#### Automated Verification:

- [x] Each ADR's frontmatter is valid:
      `accelerator corpus frontmatter validate --file <path>` for all
      four ADRs.
- [x] `grep -n '^status: "accepted"' meta/decisions/ADR-0077-*.md meta/decisions/ADR-0078-*.md meta/decisions/ADR-0079-*.md meta/decisions/ADR-0080-*.md`
      prints four lines.

#### Manual Verification:

- [x] The author has accepted all four ADRs through `/review-adr`.
- [x] PP-994's Drafting Notes carry the decisions line, naming ADR-0077,
      ADR-0078, ADR-0079, ADR-0080 and the spec.

---

## Phase 2: Refuse overlapping doc roots; run fixtures on default paths

### Overview

This phase implements ADR-0079. It is independent of the set model and
mergeable before everything else in stage two. The refusal lives in the
shared resolver,
so every CLI that reads doc roots refuses an overlapping configuration,
and migrations stop rather than run over no directories.

### Changes Required:

#### 1. Failing tests first

**File**: `cli/config/src/paths.rs` (inline tests, using the crate's
`test-support` config double)

- `research_codebase: meta/research` with defaults is refused as
  `ConfigError::OverlappingDocRoots` with `paths.research_codebase` as
  the container and `paths.research_issues` as the contained root, the
  first pair in `DOC_TYPES` order.
- `notes: meta/plans` (equal roots) is refused, naming `paths.notes` and
  `paths.plans`.
- `notes: meta/plans/` and `notes: ./meta/plans` are refused as equal
  after normalisation.
- `plans: meta/notes/plans` is refused with `paths.notes` as the
  container, although `plans` comes first in `DOC_TYPES` order.
- `notes: meta/plansx` (a shared string prefix but no shared segment) is
  accepted.
- The defaults are accepted.

**File**: `cli/visualiser/server/tests/compose_overlap.rs` (new)

- Composing the server with an overlapping config fails, with an error
  naming both keys.
- Composing the server with a non-overlapping config in which
  `paths.notes` is a symlink to the topics root fails, naming
  `paths.notes` and `paths.research_topics` (Unix only).
- Composing with `paths.notes` naming a directory that does not exist,
  under a symlink into the topics root, fails naming both keys (Unix
  only).
- A test-only doc type's root supplied directly on `Config` is covered
  by the same canonical check.

**File**: `cli/corpus-cli/src/config.rs` (inline tests)

- `table_from_config` with an overlapping config returns
  `ConfigError::OverlappingDocRoots`, so `corpus frontmatter validate`
  refuses before walking anything.

**File**: `cli/work-cli/src/config.rs` and `cli/work-cli/src/sync.rs`
(inline tests)

- `resolve_work_dir` with an overlapping config returns an error naming
  both keys.
- `corpus_roots` with an overlapping config returns
  `ConfigError::OverlappingDocRoots`, so a sync refuses before reading
  the tracker.

**File**: `cli/migrate-adapters/src/context.rs` (inline tests)

- `doc_type_dirs()` with an overlapping config returns an error naming
  both keys.
- After a `paths.*` key is rewritten through the context's config,
  `doc_type_dirs()` reflects the new directory.

**File**: `cli/migrate-cli/tests/overlapping_doc_roots.rs` (new)

- A migrate run over an overlapping config applies no migration, writes
  nothing to the ledger, and exits non-zero with the refusal.
- `migrate --list` over the same config exits non-zero with the
  refusal.
- With every migration already applied, a migrate run over an
  overlapping config exits non-zero with the refusal.
- `accelerator migrate --list` exits non-zero with the refusal when no
  interactive migration is pending.

**File**: `cli/migrate/tests/lifecycle.rs`

- `run_pending` with a stub migration that rewrites the config into an
  overlap and returns `Applied` without calling `doc_type_dirs()`, over
  a context double whose `doc_type_dirs()` fails after that rewrite,
  returns the refusal, and the ledger double never receives that
  migration's id.

**File**: `cli/launcher/tests/config_read.rs`

- `config paths --doc-types` over an overlapping config exits non-zero
  and names both keys.
- `config path plans` (one doc type's key) over an overlapping config
  exits non-zero and names both keys.
- `config path tmp` (a path key that is not a doc root) over an
  overlapping config still resolves.
- `config set paths.research_codebase meta/research/codebase` over an
  overlapping config succeeds, and a following `config paths --doc-types`
  exits 0.

**File**: `cli/config/src/paths.rs` (inline tests)

- `resolve_with_fallback` for `research_topics` over an overlapping
  config returns `ConfigError::OverlappingDocRoots`.
- With a `level` given, the overlap check reads the full set at that same
  level.
- `resolve_with_fallback` for `tmp` over an overlapping config returns
  the configured path.

**File**: `cli/research-cli/tests/topic_outstanding.rs`

- A `topic outstanding` run over an overlapping config exits non-zero
  with the refusal, before reading any set.

**File**: `cli/corpus-cli/src/config.rs` (inline tests)

- The single-key lookup at `config.rs:59` over an overlapping config
  returns `ConfigError::OverlappingDocRoots`.

#### 2. The refusal

**File**: `cli/config/src/paths.rs`

After resolving every `DocTypeDir`, call `refuse_overlapping_roots(&resolved)`:

```rust
pub struct Overlap {
    pub container: usize,
    pub contained: usize,
}

pub fn first_overlap(roots: &[&Path]) -> Option<Overlap>;

fn refuse_overlapping_roots(dirs: &[DocTypeDir]) -> Result<(), ConfigError> {
    match first_overlap(&paths_of(dirs)) {
        Some(overlap) => Err(ConfigError::OverlappingDocRoots {
            container: dirs[overlap.container].clone(),
            contained: dirs[overlap.contained].clone(),
        }),
        None => Ok(()),
    }
}
```

`first_overlap` compares `Path::components()` after normalisation, in
both directions, and returns the first overlapping pair in input order.
It names the containing root as `container`, or the earlier one in input
order when the two are equal.

`ConfigError::OverlappingDocRoots` is a new variant. `is_refusal`
classifies it as a refusal, and its `Display` renders
`paths.<container> (<dir>) overlaps paths.<contained> (<dir>); doc roots
may not equal or contain one another`.

The resolver compares configured paths, with no filesystem access.
`first_overlap` is public, so `refuse_overlapping_roots` and the
visualiser's compose share the one containment rule. Compose runs it
again over the
canonical roots it builds for `LocalFileDriver` (`file_driver.rs:126-127`)
and returns the same `ConfigError::OverlappingDocRoots`. A symlink alias
therefore cannot place one root inside a set root, where it would undo
the set's serving boundary.

A doc root that does not exist at startup cannot be canonicalised, and
today `LocalFileDriver` keeps its raw path. Compose resolves such a
root by canonicalising its nearest existing ancestor and appending the
rest, and `LocalFileDriver` is built with that same resolved root, so
the overlap check, the set serving decision and the driver's
containment checks all use one representation.

**Single-key lookups.** `resolve_with_fallback` checks whether
`path_key` names a doc type in `DOC_TYPES`. If it does, it resolves the
full set through `doc_type_dirs` at the same `level`, which refuses an
overlap, and returns the requested root from that set. Any other path key
resolves exactly as today.

#### 3. Callers

- **`corpus-cli` and `work-cli`** already propagate the resolver's
  error with `?`; they gain only the tests above.
- **`research-cli` and the launcher's `config path <key>`** already
  propagate `resolve_with_fallback`'s error; they gain only the tests
  above.
- **The migrate adapter.**
  - `MigrationContext::doc_type_dirs` becomes fallible, returning
    `Result<Vec<DocTypeDir>, MigrationError>`. `FileMigrationContext`
    still resolves from live config on every call, because migrations
    such as `m0004` rewrite `paths.*` keys and then read the new
    directories in the same run. It returns the resolver's error in
    place of `unwrap_or_default()`.
  - `FileMigrationContext::linkage_table` (`context.rs:85`), which reads
    `doc_type_dirs`, becomes fallible the same way and propagates the
    error, so `m0007` cannot run over an empty table.
  - `MigrationContext::corpus_index` (`context.rs:118-126`), which
    builds from `linkage_table`, becomes
    `Result<&dyn CorpusIndex, MigrationError>` too, and `m0007`'s call
    (`m0007/mod.rs:212`) adds `?`. Its doubles change with the others
    below, and the `migrate` snapshot diff shows both port changes.
  - The frozen migrations that call either (`m0004`, `m0007`, `m0008`)
    change only mechanically, adding `?`. The inline test doubles in
    `m0001`, `m0003`, `m0008`, `m0009` and `m0011`, the doubles in
    `migrate/tests/engine.rs`, `lifecycle.rs` and
    `list_and_decisions_file.rs`, and the assertion in
    `migrate-adapters/tests/doc_type_dir_parity.rs` change to match.
    What each migration does is unchanged.
  - The runner also resolves the doc-type directories once, before it
    computes the pending migrations and before its no-pending early
    return (`lifecycle.rs:63-66`), so an overlapping configuration stops
    every run, even an up-to-date one, before anything is applied or
    recorded. `run_list` (`migrate-cli/src/main.rs:201-226`) resolves
    them at its start the same way, so `--list` refuses too.
  - `run_pending` (`migrate/src/lifecycle.rs:104-113`) resolves them
    again after each `Applied` outcome and before appending it to the
    ledger, so a migration that writes an overlapping configuration
    stops the run non-zero and is not recorded as applied.

#### 4. CHANGELOG

Add `### Breaking` entries under `[Unreleased]` in `CHANGELOG.md`:

- configurations whose doc roots equal or contain one another are now
  refused, naming both keys, by the visualiser, `accelerator config
  path` (for the doc types and for any one doc type's key), `corpus`,
  `work` and `research` commands;
- `accelerator migrate` and `accelerator migrate --list` now refuse such
  a configuration before applying or listing any migration.

#### 5. Fixtures on default paths

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

- [x] `mise run public-api:update`, with the `config` and `migrate`
      snapshot diffs reviewed.
- [x] Caller tests pass:
      `mise run test:unit:cli --filter 'package(config) | package(accelerator-corpus) | package(accelerator-work) | package(migrate-adapters) | package(migrate) | package(accelerator-migrate) | package(accelerator-research) | package(accelerator)'`
- [x] Visualiser tests pass: `mise run test:unit:visualiser` and
      `mise run test:integration:visualiser`, including `api_smoke`.
- [x] e2e passes: `mise run test:e2e:visualiser`
- [x] `rg -n "research_codebase: meta/research" cli/visualiser` prints
      nothing.
- [x] `mise run`

#### Manual Verification:

- [x] In a scratch repo with `research_codebase: meta/research`,
      `accelerator config paths --doc-types`, `accelerator corpus
      frontmatter validate` and `accelerator migrate` each print the
      refusal naming both keys, and `migrate` records nothing as
      applied.
- [x] In the same repo, `accelerator config path research_topics` and
      `/research-topic` print the refusal, `accelerator config path tmp`
      still prints a path, and `accelerator config set
      paths.research_codebase meta/research/codebase` succeeds and clears
      the refusal.

### Implementation notes

- **Command name.** The doc-types listing is `accelerator config paths
  --doc-types`; `config path` takes one key. This plan said `config
  path --doc-types` in four places, now corrected; the CHANGELOG and the
  launcher tests already used `config paths`.
- **Manual checks, 2026-11-26.** Run in a scratch jj repo with the local
  builds through the dev launcher. Each refusal read `paths.research_codebase
  (meta/research) overlaps paths.research_issues (meta/research/issues);
  doc roots may not equal or contain one another` and exited 1, also with
  `--fail-safe`. `migrate` and `migrate --list` wrote no
  `.accelerator/state/migrations-applied`. `/research-topic`, run headless
  with `--plugin-dir`, failed to load on its `config path research_topics`
  site with that refusal. That needs this checkout's `bin/accelerator`
  first on `PATH`: otherwise the skill's bare `accelerator` resolves to
  the installed plugin's launcher link in `~/.local/bin` and runs the
  released binary. `config set` writes the personal
  `.accelerator/config.local.md`, whose value overrides the team one and
  clears the refusal.
- **Unplanned commits made to get `mise run` green.**
  - The docs site's zod advisory ignore, which expired on 2026-11-25,
    was dropped from `docs-site/audit-ignores.toml` (`uzpklpvk`). The
    override already pins zod 4.6.7, past the advisory's range. Trunk
    has since landed the same removal, so after the rebase onto it
    `uzpklpvk` is empty.
  - The GitHub client is confined to the bundled trust roots
    (`pqmltkww`). octocrab built its TLS connector over the host
    certificate store. On macOS, reading that store under the test lane's
    process-per-test load times out (`errSecIO`), so `OctocrabClient`
    failed to build, intermittently, with octocrab's `Other` error.
    Enabling octocrab's `rustls-webpki-tokio` feature gives it the
    bundled Mozilla roots every reqwest client already trusts. The
    bundled-roots guard (`tests/integration/deny/test_bundled_trust_roots.py`)
    now covers octocrab, and `webpki-roots` joined the third-party
    notices. This is unrelated to PP-994.
- **`mise run`** exited 0 twice in a row after those commits, with
  `test:unit:cli` coverage at lines 94.46%, functions 93.89%, regions
  93.23%. An earlier 90.07% lines reading came from stale instrumented
  artefacts, which the run swept.

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
  `wire_token`, with equality checking `ptr::eq` first. It delegates
  each existing method (`label()` and so on) to the declaration, so call
  sites are unchanged. `wire_str()` is renamed `wire_token()` in the same
  scripted rename.
- `DocTypeKey::declared(&'static DocType)` constructs a key.
- `built_in.rs` declares each of the 15 types as a `static` naming every
  field, and exposes `DocTypeKey::PLANS`, `DocTypeKey::WORK_ITEMS` and
  so on as associated constants.
- `DocTypeCatalogue::built_in()` returns the 15 in today's `all()` order.
  It has `doc_types()`, `from_wire_token`, `from_linkage_type_name` and
  `from_config_path_key`.
- `DocTypeCatalogue::new(keys)` returns a `CatalogueError` naming the
  clash when two declarations share a wire token, a linkage name or a
  config path key. Inline tests pin each clash.
- The server's `doc_type_view::DocType` is renamed `DocTypeView`.
- `DocTypeKey::all()`, `from_wire_str` and `from_linkage_type_name` are
  removed; the catalogue's `from_wire_token` replaces `from_wire_str`.
  Callers use the built-in catalogue, or an injected one from Phase 5.

#### 3. Mechanical migration

- Replace `DocTypeKey::Plans` with `DocTypeKey::PLANS` (and so on) across
  the workspace. Use a scripted rename followed by `mise run fix`.
- Replace `DocTypeKey::all()` with `DocTypeCatalogue::built_in().doc_types()`.
- Rewrite each `match` on a key in production code as `==` comparisons.
  These are transitional and are removed in Phase 4.
  Test code uses `==` too: the hand-written `PartialEq` rules out
  `matches!` on constants.
- `corpus-adapters/src/doc_type.rs::table_from_paths` iterates the
  built-in catalogue.

### Success Criteria:

#### Automated Verification:

- [x] The characterisation test passes before and after:
      `mise run test:unit:cli --filter 'binary_id(corpus::built_in_doc_type_facts)'`
- [x] The whole cli workspace passes: `mise run test:unit:cli`,
      `mise run test:unit:visualiser`, `mise run test:integration:visualiser`
- [x] `mise run cli:check` and `mise run server:check`
- [x] `mise run public-api:update` regenerates
      `cli/corpus/tests/fixtures/public-api.txt`, and the diff is reviewed
      as the intended surface change.
- [x] `mise run`

#### Manual Verification:

- [x] None beyond review: no behaviour changes.

### Implementation notes

- **Method names.** Beyond the plan, `nested_manifest_filename()` is
  renamed `nested_root_filename()` to match its field, alongside
  `wire_str()` → `wire_token()`. `DocTypeKey`'s `Debug` prints the wire
  token; only test assertion messages format a key with `{:?}`.
- **Transitional comparisons.** A `match` over several types became guard
  arms, `kind if [A, B].contains(&kind) =>`, ending in `_`; `canonical_rank`
  became a ranked table and `derive_completeness` one `|=` per mark. Phase
  4 replaces them all.
- **Catalogue errors.** `CatalogueError { identity: Identity, value }`
  names the clashing identity (`WireToken`, `LinkageTypeName`,
  `ConfigPathKey`) and its value.
- **cargo-pup.** The corpus domain rule permits only `crate::` imports, so
  the `doc_type` submodules import each other through
  `crate::doc_type::…`, one item per `use`.
- **Review, 2026-11-26.** The `zxlyztpl`..`qwoklqxs` diff is mechanical
  outside `corpus/src/doc_type/`: renamed constants, catalogue lookups in
  place of `all()` and `from_*`, and the transitional comparisons. Each
  rewritten rule (slug, cluster walk, canonical rank, completeness,
  linkage id, facets) maps every type to its old outcome; the catalogue
  keeps `all()`'s order, and no key is ordered or hashed into an
  observable order. Two doc comments that restated names were dropped.
  The work-item fixture the aborted first run left at `ready` is restored,
  and the working copy is clean.
- **`mise run`** exited 0, with `test:unit:cli` coverage at lines 94.48%,
  functions 93.90%, regions 93.24%.

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
- a cluster key for every type, through `cluster::walk` on one fixture
  document per type, and whether the type anchors a cluster;
- a `resolve_path_target` id for every type;
- facets for every type;
- the listing rule for every type: its drafts subdirectory and filename
  filter, or its nested root;
- the full ordered `PHASES` list: each phase's id and label, and its
  types in order, asserted through `api_library`.

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
pub library_placement: Option<LibraryPlacement>,
pub facets: &'static [Facet],
pub listing: Listing,
```

- **`SlugRule`:** `NumberedPrefix("ADR-")`, `WorkItem`,
  `DateAndOptionalId`, `DateAndOptionalIdThenReviewSuffix`,
  `WorkItemReview`, `VerbatimStem`, `NoSlug`. `slug::derive` dispatches
  on `kind.slug_rule()`.
- **`ClusterKeyRule`:** `WorkItemId`, `Parent`, `Target`, `Unclustered`.
  The `cluster::walk` arms become one `match` on the rule.
- **`cluster_anchor`** replaces the `== WorkItems` checks at
  `cluster.rs:360, 371, 464, 488`. The `Templates` checks become
  `is_virtual()`.
- **`CompletenessMark`** names the `Completeness` field to set. The wire
  struct is unchanged.
- **`LinkageIdRule`:** `WorkItemIdInFilename`, `AdrNumber`,
  `SetDirectory`, `PrNumber`, `Stem`.
- **`LibraryPlacement { phase: LibraryPhase, position: u8 }`**, with
  `LibraryPhase`: `Define`, `Discover`, `Build`, `Ship`, `Operate`,
  `Remember`. `PHASES` becomes the ordered list of phases with their ids
  and labels. A phase's types are ordered by `position`, because today's
  within-phase order differs from catalogue order (Discover lists the
  design and topic types before codebase research; Ship lists PR
  descriptions before PR reviews).
- **`Facet`:** `Status`, `ClusterSlug` and the others used by
  `facets_for`.
- **`Listing`:** `Flat { drafts_subdirectory: Option<&'static str>,
  filename_filter: FilenameFilter }` or `Nested { root }`.
  `nested_root_filename` folds into `Listing::Nested`. Phase 7 replaces
  `Nested { root }` with `Set`, which only marks the type as a set, and
  moves the root filename into the owner's set declaration, beside the
  presentation the owner declares with it.

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
  `corpus-cli/src/main.rs` keep their named constant, as allowed
  exceptions.

#### 4. Boundary test

**File**: `cli/corpus-adapters/tests/doc_type_declarations_are_single_source.rs`
(new), the single source-boundary guard, using a `source_scan` module
added to the existing `cli-test-support` crate

- Walk every `src/**/*.rs` under `cli/` except `corpus/src/doc_type/built_in.rs`
  and the allow-list. The allow-list holds ADR-0078's exceptions for code
  that owns one doc type's domain (`research-cli/src/topic_set.rs`, the
  topic set commands, and `corpus-cli/src/main.rs`, the ADR commands),
  and the frozen migrations (`migrate/src/migrations/**`), which
  ADR-0078's boundaries keep working against built-in doc types
  unchanged. Phase 6 adds the declarations crates and the assembly.
- Strip `#[cfg(test)]` modules and `#[cfg(test)]` items.
- Assert that no production code contains `DocTypeKey::` followed by an
  upper-case constant, a `use` that brings a built-in constant into
  scope, or an import from `doc_type::built_in`.
- Assert that `corpus/src/doc_type/built_in.rs` does not contain
  `FLAT_DEFAULTS`, and that at least 100 files were scanned, so a wrong
  root fails loudly.
- `cli-test-support`'s `source_scan` has its own tests, each seeding one
  violation: a qualified constant, an aliased `DocTypeKey`, a direct
  import from `built_in`, a violation inside a `#[cfg(test)]` item that
  must be ignored, and a violation after a `#[cfg(test)] mod` that must
  be found.

### Success Criteria:

#### Automated Verification:

- [x] Characterisation and existing tests pass: `mise run test:unit:cli`,
      `mise run test:unit:visualiser`, `mise run test:integration:visualiser`
- [x] The boundary test and the scanner's seeded-violation tests pass.
- [x] `mise run public-api:update`, with the `corpus` snapshot diff
      reviewed.
- [x] `mise run cli:check`, `mise run server:check`
- [x] `mise run`

#### Manual Verification:

- [x] None beyond review: the `PHASES` characterisation pins the
      sidebar's phases and order.

### Implementation notes

- **Where the rule types live.** Each rule type sits beside the code that
  reads it: `slug::SlugRule`, `cluster::{ClusterKeyRule, CompletenessMark}`
  and `linkage::LinkageIdRule`. The server-facing `LibraryPlacement`,
  `LibraryPhase`, `Facet`, `Listing` and `FilenameFilter` sit in
  `doc_type`. `nested_root_filename()` stays as an accessor that reads
  `Listing::Nested`. `facets_for` returns the declared facets' ids.
- **The built-in catalogue.** `DocTypeCatalogue::built_in()` moved into
  `built_in.rs` and uses a crate-private constructor, so nothing imports
  from `built_in`. The server's library response takes its templates entry
  from the virtual type, so `templates` is now an `Option`. It still
  serialises as an object.
- **Declared facts in the server.** Work-item checks in the indexer now
  read `cluster_anchor()`: work-item id trust, `remember_work_item`, and
  slug and identity derivation. The PATCH guard in `api/docs.rs` reads
  `in_kanban()`, because the patch only writes a status checked against
  the kanban columns. Templates checks read `is_virtual()`.
- **Beyond the plan: one linkage-address index (author's decision,
  2026-11-26).** `adr_by_id` (keyed `u32`) and `plans_by_id` were the
  last production sites that named a type. One `documents_by_address` map
  from `corpus::LinkageAddress` (linkage name and id) to a path replaces
  them.
  - A document answers to its frontmatter `id:` when it has one.
    Otherwise it answers to the id its path carries by its type's
    `linkage_id_rule`, through `corpus::linkage::linkage_id_of`.
  - `TypedRef::linkage_address` yields the address a prefixed reference
    names.
  - Production resolution through the index stays limited to the `plan:`
    form. A test pins that an `adr:` target still does not resolve.
  - `Indexer::adr_by_id(u32)` had no production caller. It is replaced by
    `document_at(&LinkageAddress)`, and its tests look up `adr:ADR-0001`.
  - **One widening:** a plan without a frontmatter `id:` now resolves by
    its filename stem.
  - **One drop:** the indexer no longer reads the legacy `adr_id`
    frontmatter key or normalises ADR numbers. The corpus schema,
    `corpus-cli`'s ADR commands and `m0007` are untouched.
  - `WorkItemConfig::linkage_ids()` gives the indexer the `LinkageIds`
    reading. Phase 9's per-map tests now name `documents_by_address`.
- **Characterisation.**
  - `corpus/tests/built_in_doc_type_rules.rs` pins, for every built-in
    type: its slug, cluster key, cluster anchoring, completeness mark,
    lifecycle-rank order and path linkage id.
  - The server pins facets (indexer), the listing rule (`file_driver`)
    and the full `PHASES` list (`api_library_structure.rs`).
- **The source scan.** `cli_test_support::source_scan` blanks comments,
  string and character literals, and test-gated items, keeping line
  breaks. A test-gated item ends at its closing brace, or at the `;` or
  `,` that ends it. Its seeded-violation tests also cover a type alias of
  `DocTypeKey`, a test-gated struct field, and a constant inside a comment
  or literal.
- **`mise run`** exited 0, with `test:unit:cli` coverage at lines 94.49%,
  functions 93.90%, regions 93.27%.
- **Review, 2026-11-26.** The `vnyyppzo`..`ulosrulp` diff changes no
  behaviour beyond the two recorded steps.
  - Each rewritten rule maps every type to its old outcome: slug, cluster
    walk, canonical rank, completeness, linkage id, facets, listing and
    phases.
  - `resolve_path_target` still requires a linkage name before deriving an
    id. A `plan:` target looks up the same id as before.
  - Address keys are scoped by linkage name, so the new index holds no
    collision the two old maps could not.
  - The rule-type doc comments say what each variant means and do not
    restate names.
  - A test helper's doc comment restated the code and ran past 80 columns,
    so it is deleted. Two other over-long lines are wrapped, and the
    `plans_by_id_with` test helper is renamed `addresses_with_plan`
    (`nyonrokl`).
  - After the tidy-up, `mise run server:check`, `mise run cli:check`,
    `mise run test:unit:visualiser` and the cli-test-support tests pass.

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
  (a `const` base behind a new `test-support` feature on `corpus`,
  `[features] test-support = []`, following `config`'s pattern and
  enabled only from dev-dependencies: the server, `corpus-adapters`, and
  a `corpus` self dev-dependency for its own tests. `built_in.rs` never
  uses it).
- It builds a catalogue of the built-ins plus `NOTEBOOKS`, configures
  `doc_paths["notebooks"]` to a tempdir holding one markdown file, and
  builds the router.
- `/api/types` lists `notebooks`. `/api/docs?type=notebooks` returns the
  file. `/api/search` finds it by title.

#### 2. Injection

- `server/src/config.rs`: add
  `#[serde(skip, default = "DocTypeCatalogue::built_in_arc")] pub doc_types: Arc<DocTypeCatalogue>`.
- `LocalFileDriver::new` takes the `Arc<DocTypeCatalogue>` that
  `Config` carries, and iterates it. The watcher holds the same `Arc`.
- `Indexer` takes the catalogue from `Config` and reads the doc-type
  list from it (`indexer.rs:334`); the driver holds no list of its own
  beyond its roots.
- `describe_types` and `api/library.rs` iterate `cfg.doc_types`.
- Wire-token parsing (`api/mod.rs:156`, `api/library.rs:149`) goes
  through `state.cfg.doc_types.from_wire_token`.
- `doc_type_serde` keeps `serialize` and loses `deserialize`. Any struct
  that deserialises a key (`rg "Deserialize" server/src` on the structs
  using it) takes the wire token as a `String` instead, and its handler
  resolves it through `state.cfg.doc_types`. Wire input is then always
  resolved against the injected catalogue, never a hidden built-in one.
- `frontmatter.rs:352`: use the catalogue.
- Add a `test_support` helper:
  `catalogue_with(extra: &[&'static DocType]) -> Arc<DocTypeCatalogue>`.
  It builds through `DocTypeCatalogue::new`, so a clashing test
  declaration fails at once.

### Success Criteria:

#### Automated Verification:

- [x] `mise run test:unit:visualiser`, `mise run test:integration:visualiser`
- [x] `mise run server:check`
- [x] `mise run public-api:update`, with the `corpus` snapshot diff
      reviewed.
- [x] `mise run`

#### Manual Verification:

- [x] None: production serves the built-in catalogue unchanged.

### Implementation notes

- **`FLAT_DEFAULTS`.** `DocType::FLAT_DEFAULTS` replaces the corpus tests'
  private `UNREMARKABLE` base. It slugs by the stem (`VerbatimStem`), so
  search, which drops slug-less entries, finds a test-declared type's
  documents. Only the server enables `test-support` from its
  dev-dependencies, alongside `corpus`'s own self dev-dependency;
  `corpus-adapters` gains it when a test there first needs it. The feature
  is off in the public API snapshot, so the `corpus` snapshot is
  unchanged.
- **The default.** `Config.doc_types` defaults through a server-local
  `built_in_doc_types()`, not a new `corpus` function; Phase 6 replaces
  the default with the assembled catalogue.
- **Injection.** `LocalFileDriver::new` and `refuse_overlapping_doc_roots`
  take `&DocTypeCatalogue`; `Indexer::build` takes the
  `Arc<DocTypeCatalogue>` as its second argument. The watcher reads no
  doc type list, so it holds no catalogue yet. `parse_kind` is gone: the
  docs handler resolves the wire token through `state.cfg.doc_types`, and
  `parse_selection_query` takes the catalogue.
- **Test helpers.** `catalogue_with` lives in the integration tests'
  `common` module, because `src/test_support.rs` is compiled for unit
  tests only; unit tests use `test_support::built_in_catalogue()`.
- **`mise run`** exited 0 on the final code, with `test:unit:cli` coverage
  at lines 94.49%, functions 93.90%, regions 93.27%.
- **Production check, 2026-11-26.** Debug builds of `smvsspvt` (before)
  and `onwpqluw` (after), each run twice with `serve` against this
  repository's `meta/`, answered the same:
  - `/api/types`: the 15 built-in types in catalogue order, with the same
    counts;
  - `/api/docs` for every type, `/api/search` for two queries,
    `/api/library/structure` with and without a selection,
    `/api/lifecycle` and `/api/templates`, compared ignoring list order
    and `linkedCount`; an unknown type still answers 400.
  - Two baseline runs already disagree with each other on list order and
    on `linkedCount` for five documents whose slugs collide, such as
    `ADR-0021-template-management-subcommands.md`. The after build gives
    values drawn from the same sets, so the variation predates this phase.
- **Review, 2026-11-26.** The `zstonnsk`..`onwpqluw` diff adds no line
  past 80 columns. Its comments are the test-support feature's
  dev-dependency constraint and `doc_type_serde`'s module doc, trimmed to
  why it has no `deserialize`. The names follow the plan's: `doc_types`,
  `catalogue_with`, `FLAT_DEFAULTS`.

---

## Phase 6: Declarations crates and the assembled catalogue

### Overview

This phase implements ADR-0080 with no change in behaviour. The
crate-rule lint learns the **declarations** and **declarations assembly**
roles. `research-declarations` and `design-declarations` hold their
contexts' layout constants, and `research` takes its topic layout from
its declarations crate. The `declarations` assembly exposes the complete
doc type catalogue, and every composition root that builds a catalogue
takes it from there. Until Phase 7 the assembled catalogue is the
built-in one.

### Changes Required:

#### 1. Failing tests first

**File**: `tests/unit/tasks/test_crate_dependencies.py`

- `test_a_declarations_crate_reaches_only_kernel_and_platform_domains`,
  parametrised: `kernel` and `corpus` are permitted; a product domain, a
  shared domain, an adapter, a technical library, another declarations
  crate and the assembly each break the declarations rule.
- `test_a_declarations_crate_needs_a_context`: a declarations crate with
  no `context` is malformed.
- `test_a_declarations_crate_is_reached_only_from_its_context_roots_and_the_assembly`,
  parametrised: its own context's domain and adapter, the assembly, a
  composition root and the launcher are permitted; another context's
  domain and adapter, a platform domain and a technical library are not.
- `test_the_assembly_reaches_only_platform_domains_and_declarations_crates`:
  `corpus` and `research-declarations` are permitted; a product domain,
  an adapter, a technical library and a composition root are not.
- `test_the_assembly_carries_no_context`: an assembly declaring a
  `context` is malformed.
- `test_only_composition_roots_and_the_launcher_depend_on_the_assembly`:
  a domain, an adapter and a declarations crate depending on it each
  break the rule.
- `test_a_dev_dependency_on_the_assembly_is_not_judged`: an adapter that
  takes the assembly as a dev-dependency only is clean, so tests may
  build the complete catalogue.
- `test_a_domain_may_depend_on_its_own_contexts_declarations_crate`,
  judged from the domain's side: `research` depending on
  `research-declarations` is clean, and on `design-declarations` breaks
  the domain rule.
- `test_the_launcher_may_depend_on_declarations_crates_and_the_assembly`:
  the launcher depending on `research-declarations` and on the assembly
  is clean, and on a product domain crate still breaks `LAUNCHER`.
- `_EXPECTED_DECLARATIONS` gains
  `"research-declarations": ("declarations", "research", None, None)`,
  `"design-declarations": ("declarations", "design", None, None)` and
  `"declarations": ("declarations-assembly", None, None, None)`, so
  `test_the_shipped_declarations_match_the_expected_table` is red until
  the crates exist.

**File**: `tests/integration/pup/test_import_rule.py`

- A probe pair for each new crate: a violation (`research-declarations`
  importing `research`, `design-declarations` importing `design`, the
  assembly importing `corpus_adapters`) and a compliant control importing
  something its permit list admits (`corpus`, and
  `research_declarations` for the assembly).

**File**: `cli/declarations/src/lib.rs` (inline tests)

- `catalogue()` builds, and its doc types equal the corpus's built-in
  declarations in order.

**File**: `cli/research-declarations/src/topic.rs` (inline tests)

- `MANIFEST`, `BRIEF`, `OUTLINE`, `SYNTHESIS`, `FINDINGS`, `REPORTS` and
  `LEVELS_SUFFIX` hold today's values. The research crate's existing
  layout tests keep passing through the re-export, unchanged.

#### 2. The lint

**File**: `tasks/lint/crate_dependencies.py`

- `Role` gains `DECLARATIONS = "declarations"`, which carries a context
  and no kind (the kind is the context domain's), and
  `DECLARATIONS_ASSEMBLY = "declarations-assembly"`, which carries none.
- `broken_rules`:
  - a declarations crate may depend on `kernel` and platform domain
    crates only;
  - the assembly may depend on platform domain crates and declarations
    crates only;
  - a declarations crate may be depended on by its own context's crates,
    the assembly, composition roots and the launcher;
  - the assembly may be depended on by composition roots and the
    launcher only;
  - a domain crate may depend on its own context's declarations crate,
    alongside `kernel` and upstream domains;
  - `_launcher_rules` admits declarations crates and the assembly.
- `Rule` gains `DECLARATIONS`, `DECLARATIONS_REACH`, `ASSEMBLY` and
  `ASSEMBLY_REACH`, each worded as the rule it states, and `LAUNCHER`'s
  wording adds declarations crates and the assembly, recording ADR-0080's
  partial supersession of ADR-0069's launcher rule.

#### 3. The crates

Each follows `tasks/README.md`'s "Registering a library crate":

1. **Workspace membership.** Add `research-declarations`,
   `design-declarations` and `declarations` to `[workspace].members` in
   `cli/Cargo.toml`, and sync the lockfile with `cargo metadata`.
2. **Inherited manifest fields** and `[lints] workspace = true`.
3. **A role declaration:** `role = "declarations"` with `context =
   "research"` or `"design"`, and `role = "declarations-assembly"`; the
   rows above in `_EXPECTED_DECLARATIONS`.
4. **A `cli/pup.ron` rule** for each:
   `research_declarations_imports_only_permitted` and
   `design_declarations_imports_only_permitted` allow `std`/`core`/
   `alloc`, `kernel::Error`, `corpus` and `crate`;
   `declarations_imports_only_permitted` allows those plus
   `research_declarations` and `design_declarations`.
   `research_domain_imports_only_permitted` gains
   `^research_declarations(::|$)`.
5. **The command-runner alternation** in
   `only_the_consent_policy_names_the_command_runner` gains the three
   crate names.
6. **A probe pair** each, above.
7. **A classification in `tasks/public_api.py`.** All three join
   `_PINNED_CRATES`: a declarations crate's surface is the contract its
   context's domain and the assembly build against, and the assembly's
   is the one every composition root builds against. Each gets
   `tests/fixtures/public-api.txt`.

`tasks/README.md`'s "Registering a library crate" names the roles that
carry a `context`; it gains the declarations role, the assembly's lack of
one, the `<context>-declarations` naming, and that a new declarations
crate is also added to the assembly.

**`cli/research-declarations`**, depending on `kernel` and `corpus`:

- `src/topic.rs` takes `MANIFEST`, `BRIEF`, `OUTLINE`, `SYNTHESIS`,
  `FINDINGS` and `REPORTS` from `cli/research/src/topic/layout.rs:10-15`,
  and `LEVELS_SUFFIX` (`".levels"`), today private to
  `research-adapters/src/topic.rs:58` and spelled as a literal in
  `research`'s `finding_path.rs` and `note_ref.rs`. The shape needs the
  suffix, so it moves here; nothing else moves.
- `research` depends on `research-declarations`. `research::topic::layout`
  re-exports the constants, so its path builders and
  `research-adapters`' call sites keep compiling.
  `finding_path.rs`, `note_ref.rs` and `research-adapters`' private
  constant use `LEVELS_SUFFIX`.
- `research` gains no dependency on `corpus`.

**`cli/design-declarations`**, depending on `kernel` and `corpus`:

- `src/inventory.rs` holds `INVENTORY = "inventory.md"` and
  `SCREENSHOTS = "screenshots"`.
- Nothing in `design`, `design-adapters` or `design-cli` names the
  inventory layout in production code today, so `design` does not
  depend on `design-declarations` yet: ADR-0080 gives a domain that edge
  for the facts it shares with platform crates, and `design` shares
  none. It gains the edge when its own code first names the layout.

**`cli/declarations`**, the assembly, depending on `corpus`,
`research-declarations` and `design-declarations`:

```rust
pub fn catalogue() -> Result<Arc<DocTypeCatalogue>, CatalogueError>;
```

It returns a `Result` because building a catalogue is where ADR-0078's
refusals happen, and the workspace lints warn on `expect` and `panic`;
each root surfaces the error at startup as it does a configuration
error. Its own test above proves the shipped declarations build. In this
phase it wraps `DocTypeCatalogue::built_in()`.

#### 4. Composition roots take the assembled catalogue

The built-in catalogue is read today by `corpus-adapters`
(`doc_type.rs:17`, `resolve.rs:41`), `migrate-adapters` (`context.rs:88`),
`m0007` (`m0007/mod.rs:72`) and the visualiser server. Adapters may not
depend on the assembly, so each takes the catalogue from its root:

- **Visualiser server.** Depends on `declarations`. `Config.doc_types`
  becomes `#[serde(skip)] Option<Arc<DocTypeCatalogue>>`; compose takes
  `declarations::catalogue()?` when it is `None`, and hands the one `Arc`
  to `LocalFileDriver`, the indexer, the watcher and `AppState`.
  Handlers read `state.doc_types`. Tests set `Some` through
  `test_support::catalogue_with`.
- **`corpus-cli`.** Depends on `declarations`. `main` builds the
  catalogue once before dispatch; a `CatalogueError` exits 1 with its
  message. `table_from_paths` and `resolve_document` take
  `&DocTypeCatalogue`.
- **`research-cli`.** Depends on `declarations`, and passes the
  catalogue to `resolve_document` from `topic_set::resolve_set`.
  `research-adapters` is unchanged.
- **`migrate-cli`.** Depends on `declarations`, and builds
  `FileMigrationContext` with the catalogue. `MigrationContext` gains
  `doc_types() -> &DocTypeCatalogue`, which `linkage_table` and `m0007`
  read in place of the built-in lookup. `m0007` changes mechanically;
  what it does is unchanged. The test doubles in `migrate` change to
  match.
- `work-cli`, `shortcut-cli`, the launcher and the other roots build no
  catalogue and gain no dependency.
- Tests that need the complete catalogue outside a root
  (`corpus-adapters/tests/common/mod.rs`, `doc_type_single_source.rs`)
  take it from `declarations` as a dev-dependency. The lint judges
  normal and build dependencies only.

#### 5. Boundary test

`doc_type_declarations_are_single_source.rs` allows
`cli/*-declarations/src/**` and `cli/declarations/src/**`: ADR-0078
places the declarations themselves, in the corpus and the declarations
crates, and the assembly that combines them outside its boundary. A
seeded case first proves the scanner skips a built-in name under
`cli/declarations/src` and still reports one under `cli/research/src`.

### Success Criteria:

#### Automated Verification:

- [x] Lint tests pass:
      `mise run test:unit:tasks --filter 'crate_dependencies'`
- [x] `mise run lint:crate-dependencies:check`
- [x] `mise run test:integration:pup --filter 'declarations'` and
      `mise run pup:check`
- [x] `mise run test:unit:cli`, `mise run test:unit:visualiser`,
      `mise run test:integration:visualiser`
- [x] `mise run public-api:update`, with the new `research-declarations`,
      `design-declarations` and `declarations` snapshots and the
      `research` and `migrate` diffs reviewed.
- [x] `mise run`

#### Manual Verification:

- [x] None beyond review: no behaviour changes.

### Implementation notes

- **The lint.** A declarations crate carries a `context` and no `kind`;
  one with a `kind` is malformed. A context made of a declarations crate
  alone, with no domain or adapter crate, is malformed as declaring no
  kind. The assembly may depend on platform domains and declarations
  crates only, not on `kernel`, as the plan words its rule, though its
  pup rule still admits `kernel::Error`. A dependency can break several
  rules at once: `research` on `design-declarations` breaks both the
  inward rule and the declarations-reach rule, and both are reported.
- **The crates.** `research-declarations` and `design-declarations`
  depend on `kernel` and `corpus` as planned, though neither uses them
  yet. The pup probes for the three crates are a dedicated pair each;
  `research`'s domain probe gains `research_declarations` among its
  extra allowances.
- **Roots.** `corpus-adapters`' `table_from_paths`, `infer` and
  `resolve_document` take the catalogue first. `research-cli` maps a
  catalogue that cannot be built to `SetError::UnreadableConfig`, so the
  node query reports it with its project refusal code.
- **Migrations.** `Capabilities` gains `doc_types`, which
  `FileMigrationContext` lends through `MigrationContext::doc_types()`.
  Beyond the plan, the test doubles in `migrate` and `migrate-adapters`'
  test composition use `DocTypeCatalogue::built_in()` rather than the
  assembly; only the two `corpus-adapters` suites the plan names take
  the assembled catalogue as a dev-dependency.
- **Server.** `AppState::build` takes the assembled catalogue when
  `Config.doc_types` is `None`, refusing to start on a `CatalogueError`,
  and holds it as `AppState.doc_types`, which the handlers read.
  `describe_types` and `build_structure` take the catalogue.
- **The boundary test** names a source a declarations crate or the
  assembly owns by its crate directory before `/src/`.
- **`mise run`** exited 0 on the final code, with `test:unit:cli`
  coverage at lines 94.47%, functions 93.79%, regions 93.22%. A first run
  fell below the coverage floor on stale llvm-cov artefacts; after
  `cargo llvm-cov clean --workspace` the rerun passed.
- **Public API.** The `migrate` snapshot gains
  `MigrationContext::doc_types`; `research` re-exports the layout
  constants from `research-declarations`.
- **The server's catalogue test.** It was written alongside the code, so
  it was checked against a mutation: with `AppState::build` taking an
  empty catalogue in place of `declarations::catalogue()` when the config
  carries none, `a_config_carrying_no_catalogue_serves_the_assembled_one`
  fails on the comparison, an empty list against the fifteen assembled
  doc types. With the code restored it passes.
- **Review, 2026-11-26.** The `xruxlvtk`..`typvrzxp` diff names its new
  roles, crates and fields after ADR-0080's terms. Its comments are module
  and item docs, the pup rules' rationale and the probe table's purpose.
  One module doc over 80 columns is rewrapped (`sqzqlltx`); the command
  runner rule's regex stays on one line, because a RON string cannot
  split.
- **Production check, 2026-11-26.** This checkout's launcher, with the
  debug `corpus` and `migrate` binaries, was run in a scratch jj repo
  holding an ADR, this plan, PP-994 and this repo's config and migration
  ledger. `config paths --doc-types`, `corpus adr next-number`, `corpus
  resolve --type plan`, `corpus frontmatter validate`, `corpus linkage
  extract` and `migrate --list` answer exactly as the installed release
  does, and `migrate --list` exits 0 with no pending transformations.

---

## Phase 7: Set declarations on the catalogue

### Overview

The domain model for sets, in `corpus::set`: set declarations, shapes,
slot patterns and classification of a set-relative path. The owners'
declarations crates declare the two sets, each a shape built from their
layout constants and a presentation, and the assembly builds the
catalogue from the corpus's built-in declarations and those set
declarations. This is ADR-0078's interim pairing: both set types' other
facts and rules stay in the corpus, which only marks them as sets, beside
the set declarations their owners make, until PP-1007 moves the whole
declarations. The doc types response exposes shape and presentation
together. Nothing indexes members
yet, and the corpus resolver is unchanged until Phase 8.

### Changes Required:

#### 1. Failing tests first

**File**: `cli/corpus/src/set/shape.rs` (inline tests, against test set
declarations built with `SetMatcher::new`)

- **`SlotPattern::parse`:**
  - accepts `brief.md`, `findings/*.md`, `findings/*.levels/*.md` and
    `screenshots/*.png`;
  - rejects `**`, an empty segment, a leading `/` and `..`.
- **`*`:**
  - matches `01-a`, `Some File (v2)` and `01-a.levels`;
  - never matches `.x`, `.01-x-web.md.invalid` or `.conduct-run.json`;
  - never spans `/`.
- **Classification,** for a test shape with root `index.md`, document
  slots `docs/*.md` and `docs/*.parts/*.md`, and asset slot `img/*.png`:
  - `classify("index.md") == Some(Root)`;
  - `classify("docs/a.parts/1.md") == Some(Member(Document))`;
  - `classify("img/a.png") == Some(Member(Asset))`;
  - `docs/sub/x.md`, `notes.txt`, `img/sub/a.png` and `.hidden.md` are
    `None`.
- **Extension case.** Write these red tests first:
  - `img/a.PNG` and `img/a.Png` are `Some(Member(Asset))`;
  - `docs/a.MD` is `Some(Member(Document))`;
  - for a test shape with an `img/*.jpg` slot, `img/b.JPG` is
    `Some(Member(Asset))`;
  - a folder whose name differs only in case is not a member: `Img/a.png`,
    `IMG/a.png`, `docs/a.PARTS/1.md` and `Docs/a.md` are `None`;
  - case-insensitivity reaches the extension and no further:
    `img/a.png.bak` is `None`.
- **`may_contain_members`:** `docs` and `docs/a.parts` hold;
  `docs/a.parts/x`, `notes` and `.scratch` do not.

**File**: `cli/corpus/src/doc_type/catalogue.rs` (inline tests, over test
declarations of one flat and two set types)

- Giving each set type a set declaration builds, and `set_matcher(kind)`
  is `Some` for each set type, with that declaration's shape and
  presentation, and `None` for the flat type.
- Building refuses a set type with no set declaration, a set declaration
  for the flat type and a type given two set declarations (a duplicate
  declaration), each naming the type.
- Building refuses a set declaration with a `**` slot, naming the type
  and the pattern.
- The duplicate token, linkage name and path key refusals keep passing.

**File**: `cli/research-declarations/src/topic.rs` (inline tests, through
`corpus`'s `test-support` feature)

- `classify(MANIFEST) == Some(Root)`, and
  `classify("findings/01-a.levels/2-1.md") == Some(Member(Document))`.
- `findings/sub/x.md`, `notes.txt`, `findings/.01-x-web.md.invalid` and
  `.conduct-run.json` are `None`.
- `findings/01-a.MD` is `Some(Member(Document))`; `Findings/01-a.md` and
  `findings/01-a.LEVELS/2-1.md` are `None`.
- `SET.presentation` is `Presentation::SetPage`.

**File**: `cli/research/tests/topic_set_shape.rs` (new; the contract
with the layout)

The path builders are behaviour and stay in `research`, so the test
lives in the one crate that sees both them and the shape. It is an
integration test with a `corpus` dev-dependency (`test-support`), so the
`research` library gains no `corpus` edge. Every path the builders
produce classifies as a document member under
`research_declarations::topic::SET`'s shape: `BRIEF`, `OUTLINE`, `SYNTHESIS`,
`Stem::finding_path` for allocated stems, `NoteRef::path` for a root and
a nested lineage, and `ReportSlug::free_report_path` with and without a
taken name.

**File**: `cli/design-declarations/src/inventory.rs` (inline tests)

- `classify(INVENTORY) == Some(Root)`.
- `screenshots/a.png`, `screenshots/a.PNG` and
  `screenshots/Some File (v2).png` are `Some(Member(Asset))`.
- `prototype-full/x.html`, `assets/y.svg`, `notes.txt`,
  `screenshots/sub/a.png` and `Screenshots/a.png` are `None`.
- `may_contain_members("screenshots")` holds, and
  `may_contain_members("prototype-full")` does not.
- `SET.presentation` is `Presentation::EntryPoint`.

**File**: `cli/declarations/src/lib.rs` (inline tests)

- The assembled catalogue's matchers have roots `manifest.md` and the
  set-page presentation for topic research, and `inventory.md` and the
  entry-point presentation for design inventories.

**File**: `cli/corpus/tests/built_in_doc_type_facts.rs`

- **Characterisation.** Topic research and design inventories are
  `Listing::Set`, and no other built-in type is. The table's nested-root
  column moves to the assembly's test above, which also pins each set's
  presentation, because `corpus` cannot see the owners' set
  declarations.

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

pub struct SetDeclaration {
    pub shape: SetShape,
    pub presentation: Presentation,
}

pub enum SetPlace { Root, Member(MemberKind) }

pub struct SetMatcher {
    declaration: &'static SetDeclaration,
    slots: Vec<(SlotPattern, MemberKind)>,
}

impl SetMatcher {
    pub(crate) fn new(
        declaration: &'static SetDeclaration,
    ) -> Result<SetMatcher, SlotPatternError>;
    pub fn shape(&self) -> &'static SetShape;
    pub fn presentation(&self) -> Presentation;
    pub fn root(&self) -> &'static str;
    pub fn classify(&self, set_relative: &str) -> Option<SetPlace>;
    pub fn may_contain_members(&self, set_relative_dir: &str) -> bool;
}
```

**File**: `cli/corpus/src/doc_type/catalogue.rs` and `declaration.rs`

```rust
pub type DeclaredSet = (DocTypeKey, &'static SetDeclaration);

impl DocTypeCatalogue {
    pub fn new(
        doc_types: &[DocTypeKey],
        set_declarations: &[DeclaredSet],
    ) -> Result<DocTypeCatalogue, CatalogueError>;
    pub fn set_matcher(&self, kind: DocTypeKey) -> Option<&SetMatcher>;
}
```

- `CatalogueError` gains `MissingSetDeclaration { kind }`,
  `NotASet { kind }`, `SetDeclaredTwice { kind }` and
  `UnparseableSlot { kind, pattern, cause: SlotPatternError }`, ADR-0078's
  refusals applied to the pairing of a corpus-declared set type with its
  owner's set declaration.
- `DocTypeCatalogue::new` parses every slot pattern, so `classify` never
  meets an unparsed pattern, and a missing or malformed set declaration
  fails when the catalogue is built.
- `DocTypeCatalogue::built_in()` goes. `corpus::doc_type::built_in`
  exposes `doc_types() -> &'static [DocTypeKey]`, the built-in
  declarations in order, which only the assembly and test code build
  from.
- `SetMatcher::new` is crate-private to `corpus`, exposed for tests
  through the `test-support` feature, so `set_matcher` is the only
  production source of a matcher.
- `may_contain_members(dir)` holds when the directory's segments match
  the leading segments of some slot, under the same dot and case rules
  as `classify`. The walk's pruning, the watcher's admission rule and
  the resolver's path rule all call it.
- A parsed final segment holds its name pattern and its extension
  separately. The extension is compared with `eq_ignore_ascii_case`;
  every other segment, and the name before the extension, is compared
  exactly.
- `Listing::Nested { root }` becomes `Listing::Set`, which only marks the
  type as a set, with `DocTypeKey::is_set()`. A set's root and
  presentation are read from its paired set declaration, through
  `catalogue.set_matcher(kind)`.

#### 3. Owners declare their sets

**File**: `cli/research-declarations/src/topic.rs`

```rust
pub static SET: SetDeclaration = SetDeclaration {
    shape: SetShape {
        root: MANIFEST,
        member_slots: &[
            MemberSlot { pattern: BRIEF, kind: MemberKind::Document },
            MemberSlot { pattern: OUTLINE, kind: MemberKind::Document },
            MemberSlot { pattern: SYNTHESIS, kind: MemberKind::Document },
            MemberSlot {
                pattern: "findings/*.md",
                kind: MemberKind::Document,
            },
            MemberSlot {
                pattern: "findings/*.levels/*.md",
                kind: MemberKind::Document,
            },
            MemberSlot {
                pattern: "reports/*.md",
                kind: MemberKind::Document,
            },
        ],
    },
    presentation: Presentation::SetPage,
};
```

Const string concatenation cannot join `FINDINGS`, `LEVELS_SUFFIX` and
`REPORTS` into a pattern without a new dependency, so the wildcard slots
are literals and the contract test binds them to the path builders.

**File**: `cli/design-declarations/src/inventory.rs`

```rust
pub static SET: SetDeclaration = SetDeclaration {
    shape: SetShape {
        root: INVENTORY,
        member_slots: &[MemberSlot {
            pattern: "screenshots/*.png",
            kind: MemberKind::Asset,
        }],
    },
    presentation: Presentation::EntryPoint,
};
```

`research-declarations` and `design-declarations` gain a `corpus`
dev-dependency with the `test-support` feature, and `research` one for
its integration test. No pup rule changes: the declarations crates
already allow `corpus`.

#### 4. The assembly pairs set types with their set declarations

**File**: `cli/declarations/src/lib.rs`

```rust
pub fn set_declarations() -> &'static [DeclaredSet];
pub fn catalogue() -> Result<Arc<DocTypeCatalogue>, CatalogueError>;
```

`set_declarations()` pairs `DocTypeKey::TOPIC_RESEARCH` with
`research_declarations::topic::SET` and
`DocTypeKey::DESIGN_INVENTORIES` with `design_declarations::inventory::SET`.
`catalogue()` builds
`DocTypeCatalogue::new(built_in::doc_types(), set_declarations())`.
Naming the two built-in types here is inside ADR-0078's boundary, since
the assembly combines the declarations. The pairing is the interim form
of one declaration per doc type: once PP-1007 moves a set type's
declaration to its owner, the owner's declarations crate supplies the
type with its set declaration and `set_declarations()` loses that
entry.

#### 5. The visualiser server reads matchers from its catalogue

- `LocalFileDriver`, the watcher and the indexer reach matchers through
  `doc_types.set_matcher(kind)` on the `Arc<DocTypeCatalogue>` they
  hold.
- `file_driver.rs:258` reads the root from `set_matcher(kind)`.
  `indexer.rs:1534` and `:1652`, which only ask whether a type is a set,
  read `kind.is_set()`.
- `test_support::catalogue_with` becomes
  `catalogue_with(extra_types: &[&'static DocType], extra_sets: &[DeclaredSet]) -> Arc<DocTypeCatalogue>`.
  It builds through `DocTypeCatalogue::new` from the built-in
  declarations and `declarations::set_declarations()` plus the extras,
  so a test-only set type's shape and presentation are checked as a
  built-in one's are.

#### 6. Other readers of the old declaration

`corpus-adapters/src/resolve.rs:46-50` reads `kind.is_set()` in place of
the nested manifest filename. Its behaviour is unchanged until Phase 8.

#### 7. Doc types response

- `server/src/doc_type_view.rs`: add a `set: Option<SetView>` to
  `DocTypeView`, built from the paired set declaration,
  `doc_types.set_matcher(kind)`'s `shape()` and `presentation()`,
  serialised as in the spec.
- `frontend/src/api/types.ts:112-123`: add `set: DocTypeSet | null` to
  `DocType`, and normalise a missing field to `null` in `fetch.ts`, with
  a `fetch.test.ts` case for a response that omits `set`.

### Success Criteria:

#### Automated Verification:

- [x] Set declaration, catalogue and owner tests pass:
      `mise run test:unit:cli --filter 'package(corpus) | package(research) | package(research-declarations) | package(design-declarations) | package(declarations)'`
- [x] `mise run test:unit:cli`, `mise run test:unit:visualiser`,
      `mise run test:integration:visualiser`
- [x] `mise run pup:check`
- [x] `mise run public-api:update`, with the `corpus`,
      `research-declarations`, `design-declarations` and `declarations`
      snapshot diffs reviewed.
- [x] `mise run frontend:check`, `mise run test:unit:frontend`
- [x] `mise run`

#### Manual Verification:

- [x] `curl localhost:<port>/api/types` shows both sets' shapes and
      presentations.

### Implementation notes

- **The model.** `corpus::set` holds the declared data (`SetDeclaration`,
  `SetShape`, `MemberSlot`, `MemberKind`, `Presentation`); `set/shape.rs`
  holds `SlotPattern`, `SlotPatternError` (`AbsolutePath`,
  `EmptySegment`, `ParentSegment`, `RecursiveWildcard`), `SetPlace` and
  `SetMatcher`. A `*` never matches a name that starts with a dot. The
  catalogue builds matchers through the crate-private
  `SetMatcher::compiled`; `test-support` exposes it as `SetMatcher::new`,
  which returns the failing pattern with its cause.
- **The catalogue.** `CatalogueError` is now an enum; the identity clash
  is `SharedIdentity { identity, value }`. Beyond the plan, `NotASet`
  also refuses a set declaration for a type outside the catalogue. The
  catalogue has no `Default`: an empty catalogue would serve nothing, and
  only tests wanted one, so an empty one is built with
  `DocTypeCatalogue::new(&[], &[])`.
- **Test doubles.** With `DocTypeCatalogue::built_in()` gone, `migrate`
  and `migrate-adapters` take `declarations` as a dev-dependency. Every
  test helper that takes the assembled catalogue fails loudly when it
  does not build: the `migrate` doubles, `corpus-adapters`' and
  `corpus-cli`'s inline helpers and the server's `assembled_catalogue()`
  `expect` it, and `migrate-adapters`' `Composition::at` propagates the
  error. The corpus's own tests read `built_in::doc_types()`, pairing the
  two set types with a stand-in set declaration where they need a
  catalogue. The server's unit-test helper is `assembled_catalogue()`.
- **Server.** `LocalFileDriver::new` takes `&Arc<DocTypeCatalogue>` and
  holds it, reading a set type's root from `set_matcher`. The watcher and
  indexer need no matcher yet: the indexer reads `kind.is_set()`.
  `DocTypeView.set` is a `SetView` serialised as the spec gives it.
  `declared_doc_types.rs` gains a test-only set type declared through
  `catalogue_with`'s `extra_sets`, listed and described like a built-in.
- **Owners.** `research_declarations::topic::SET` builds its slots with a
  `document(pattern)` `const fn`. The contract test was checked against a
  mutation: with `findings/*.level/*.md` in place of the levels slot, it
  fails on `findings/01-cost-model-web.levels/1.md`.
- **Strict TDD.** Each new test was seen failing before its code, except
  guards that already passed against a stub, which were checked by
  mutation instead:
  - `set::shape`: matching directory names in any case fails
    `a_directory_name_matches_only_in_its_declared_case`; comparing an
    extension by prefix from the first dot fails
    `case_is_ignored_in_the_extension_and_nowhere_else`; dropping the
    depth check fails `a_path_that_fills_no_slot_is_not_in_the_set`;
    refusing `.levels` segments fails the parse test; a fixed
    presentation fails the matcher's accessor test.
  - The owners' negative tests fail when `findings/*/*.md` or
    `prototype-full/*.html` is added to the shape.
  - The server's test-only set type fails when the driver lists
    `index.md` in place of the declared root, or the view drops `set`.
  - `fetchTypes` keeping a set fails when it always returns `set: null`.
- **Review, 2026-11-26.** The `qtvokzqu`..`slvnwxpl` diff adds no source
  line past 80 columns; only `public-api.txt` snapshot lines are longer.
  Doc comments that restated a type's name (`SetPlace`, `DeclaredSet`,
  `CatalogueError`, `SetView`) are dropped, and the indexer's
  `manifest_directory_of` is renamed `set_directory_of` (`lsvtymvo`).
- **Production check, 2026-11-26.** With `mise run dev` serving this
  repository's `meta/` from this checkout, `/api/types` lists
  `design-inventories` with root `inventory.md`, asset slot
  `screenshots/*.png` and `entry-point`, and `topic-research` with root
  `manifest.md`, its six document slots and `set-page`; the other 13
  types carry `set: null`. `/api/docs?type=design-inventories` lists the
  six real inventories, and `/api/library/structure`, `/api/lifecycle`,
  `/api/templates`, `/api/search` and the frontend answer 200.
- **`mise run`** exited 0 on the final code, with `test:unit:cli`
  coverage at lines 94.45%, functions 93.79%, regions 93.20%.

---

## Phase 8: The corpus resolver reads sets through their shape

### Overview

`accelerator corpus resolve`, and the research commands that resolve a
topic set through `topic_set::resolve_set`, conform to ADR-0077. A set
is a non-dot directory holding its root file. A slug resolves only such
a set. A path resolves to its set when it is the set directory, its
root, a member, or a directory inside the set for which
`may_contain_members` holds (`findings/`, `findings/x.levels/`,
`screenshots/`). Dot-prefixed segments anywhere, unslotted files,
rootless directories and stray files directly in the type directory are
not found.

This is a breaking change to the CLI. It needs only Phase 7 and is
independent of Phases 9–12.

### Changes Required:

#### 1. Failing tests first

**File**: `cli/corpus/src/resolve.rs` (inline tests, `FakeLister` and
the test shapes of Phase 7)

- **Slug input.**
  - `a_dot_prefixed_set_directory_does_not_resolve_by_exact_name`: with
    an entry `.x.tmp`, the slug `.x.tmp` is `NotFound`.
  - `a_dot_prefixed_set_directory_is_no_candidate`: entries `x` and
    `.x` with the slug `x` give `Single("x")`, not an ambiguity.
  - The existing date-prefix and ambiguity tests keep passing against
    `TypeShape::Set`.
- **`set_directory_of`,** over a topic-research-like test shape:
  - `s` as a directory, `s/manifest.md`, `s/findings/01-a.md`,
    `s/findings/01-a.levels/2-1.md` and `s/reports/r.md` as files, and
    `s/findings` and `s/findings/01-a.levels` as directories, each give
    `Some("s")`;
  - `s/notes.txt`, `s/.conduct-run.json`,
    `s/findings/.01-x-web.md.invalid` and `s/findings/sub/x.md` as files,
    `s/.scratch` and `s/findings/01-a.levels/x` as directories, `.x.tmp`
    as a directory, `.x.tmp/manifest.md` as a file, and `stray.md` as a
    file directly in the type directory, each give `None`;
  - over an inventory-like shape, `s/screenshots` as a directory gives
    `Some("s")`, and `s/prototype-full/x.html` and `s/prototype-full` give
    `None`.

**File**: `cli/corpus-adapters/src/fs.rs` (inline tests)

- `a_nested_manifest_type_lists_subdirectories_only` becomes
  `a_set_type_lists_directories_holding_their_root`: with `set-one/` and
  its root, a rootless `set-two/`, `.x.tmp/` with a root, and `stray.md`,
  the lister returns `set-one` only.
- `a_directory_named_as_the_root_does_not_make_a_set`: a `set-three/`
  whose root name is a directory is not listed.

**File**: `cli/corpus-cli/tests/resolve_goldens.rs`

The three existing nested tests keep passing unchanged: their sets hold
`inventory.md`, and `screenshots/home.png` fits the inventory shape.
New tests, each asserting exit code 3:

- `a_rootless_set_directory_does_not_resolve_by_slug_or_path`;
- `a_dot_prefixed_directory_does_not_resolve_by_exact_name_or_path`;
- `an_unslotted_file_in_a_set_does_not_resolve`, for `notes.txt`,
  `prototype-full/x.html` and a dot-prefixed working file;
- `a_stray_file_in_the_type_directory_does_not_resolve_as_a_set`.

And one asserting the set directory is printed:

- `a_path_to_a_slot_directory_resolves_to_its_set`, for `screenshots/`.

**File**: `cli/research-cli/tests/topic_outstanding.rs`

- `a_topic_set_without_its_manifest_is_unresolved`: `topic outstanding`
  on a rootless set directory fails with `E_TOPIC_RESEARCH_UNRESOLVED`.
- `a_path_to_a_findings_directory_resolves_to_its_set` succeeds.
- `a_path_to_the_run_ledger_is_unresolved`, for `.conduct-run.json`.

The existing `topic_outstanding`, `topic_counts`, `topic_consultable`,
`topic_report_path` and `topic_node` tests keep passing unchanged; every
`cli/corpus-cli/tests/fixtures/topic-research-*-set/` fixture holds
`manifest.md`.

#### 2. Domain

**File**: `cli/corpus/src/resolve.rs`

```rust
#[derive(Clone, Copy)]
pub enum TypeShape<'m> {
    Flat,
    Set(&'m SetMatcher),
}

pub enum EntryKind { File, Directory }

pub fn set_directory_of<'p>(
    type_relative: &'p str,
    entry: EntryKind,
    matcher: &SetMatcher,
) -> Option<&'p str>;
```

- `TypeShape::NestedManifest` becomes `TypeShape::Set`, carrying the
  type's matcher.
- `nested_candidates` becomes `set_candidates`, which drops dot-prefixed
  names before comparing slugs.
- `set_directory_of` takes a `/`-separated path relative to the type
  directory. Its first segment is the set directory and must not start
  with a dot. With nothing after it, the entry must be a directory. A
  file after it must classify under the shape, as the root or a member;
  a directory after it must satisfy `may_contain_members`.
- `DirectoryLister`'s contract for a set type: the directories holding
  the shape's root file.

#### 3. Adapter

**File**: `cli/corpus-adapters/src/fs.rs`

- `TypeDirectoryLister` for `TypeShape::Set(matcher)` keeps a directory
  only when its name has no leading dot and `matcher.root()` inside it is
  a regular file.

**File**: `cli/corpus-adapters/src/resolve.rs`

- `resolve_document` already takes the root's `&DocTypeCatalogue`
  (Phase 6). The type is a set when `catalogue.set_matcher(key)` is
  `Some`.
- `resolve_path_class` for a set type keeps the canonicalise and
  outside-root steps, then passes the canonical path relative to the type
  directory, `/`-separated, to `set_directory_of` with its entry kind,
  and requires the set directory's root file to exist. Anything else is
  `NotFound`, `no '<type>' set at path '<input>'`.
- `set_root_under` and its comment are deleted.

#### 4. Composition roots

`corpus-cli` and `research-cli` already pass the assembled catalogue
from `declarations` (Phase 6), so they change only in the tests above.
`research-adapters` gains no dependency on `corpus-adapters`.

#### 5. Help text and skills

- `cli/corpus-cli/src/cli.rs:40-49`: the command resolves to "the set
  directory for a set type", and the argument is "the slug, the set
  directory, or a path to its root, a member or a slot directory".
- `skills/design/inventory-design/SKILL.md`:
  - `:318-319`: keep the claim that the directory glob and the resolver
    skip leading-dot directories, which is now true, naming the resolver
    as `accelerator corpus resolve`;
  - `:326` and `:372`: the `sequence` tiebreaker is
    `analyse-design-gaps`'s rule for picking among inventories
    (`skills/design/analyse-design-gaps/SKILL.md:45-59`), not
    `corpus resolve`'s; say so.
- `skills/research/research-topic/SKILL.md`:
  - `:123-124`: "a bare slug, the set directory, or a path to one of its
    documents or to `findings/`, a `.levels/` directory or `reports/`";
  - `:200-201`: the rename keeps a half-written set from the visualiser
    and from `corpus resolve`, both of which skip dot-prefixed
    directories.
- The docs-site skill reference pages
  (`docs-site/src/content/docs/reference/skills/**`) are generated from
  the `SKILL.md` files by `tasks/shared/skill_pages.py` and are
  gitignored; they are not edited by hand.

#### 6. CHANGELOG

Add a `### Breaking` entry under `[Unreleased]`: `accelerator corpus
resolve` resolves a set type only to a non-dot directory holding its
root file, and a path only when it is the set directory, its root, a
member, or a directory that may hold members. A rootless or dot-prefixed
directory, an unslotted file and a stray file in the type directory now
exit 3. The research topic commands resolve sets the same way.

### Success Criteria:

#### Automated Verification:

- [x] Resolver tests pass:
      `mise run test:unit:cli --filter 'package(corpus) | package(corpus-adapters) | package(accelerator-corpus) | package(accelerator-research)'`
- [x] Goldens pass:
      `mise run test:unit:cli --filter 'binary_id(accelerator-corpus::resolve_goldens)'`
- [x] `mise run cli:check`
- [x] `mise run public-api:update`, with the `corpus` snapshot diff
      reviewed.
- [x] `mise run`

#### Manual Verification:

- [x] In this repo, `accelerator corpus resolve --type design-inventory`
      resolves each real inventory by slug, by directory and by a
      screenshot path, and a `prototype-full/` path exits 3.
- [x] `/research-topic outline` on an existing scratch set still
      resolves it by slug and by a findings path.

### Implementation notes

- **Domain.** `TypeShape<'m>` is `Flat | Set(&'m SetMatcher)`; it drops
  `PartialEq`, since a matcher has none. `set_candidates` drops
  dot-prefixed names before comparing slugs. `set_directory_of` is pure,
  so the corpus's inline tests pin every path in the plan's list over a
  topic-like and an inventory-like test shape. Beyond the plan, a test
  pins that a slug two set directories share is still ambiguous.
- **Adapter.** `TypeDirectoryLister` takes the shape's lifetime.
  `resolve_path_class` hands `set_directory_holding` the canonical path;
  a path that is neither a file nor a directory, or not UTF-8, is not
  found. The type is a set when `catalogue.set_matcher(key)` is `Some`, so
  `kind.is_set()` is no longer read here. `set_root_under` is gone.
- **Goldens.** The dot-prefixed test also resolves the hidden set's
  `inventory.md`, and the unslotted-file test uses `.inventory.md.swp` as
  its dot-prefixed working file. The research tests use the multiprofile
  fixture, and the findings-path test asserts that its plan equals the
  slug's.
- **Skills.** `inventory-design`'s tiebreaker lines now name
  `analyse-design-gaps` and say `corpus resolve` does not read
  `sequence`. The indexer's doc comment on `IndexEntry.id` now names a
  set root's set directory, not a nested manifest's.
- **Strict TDD.** Each new test was seen failing before its code, except
  guards that already passed, which were checked by mutation instead:
  - `a_dot_prefixed_set_directory_is_no_candidate` passes against the old
    code, because `.x`'s slug is `.x`; trimming leading dots from the
    candidates fails it.
  - `a_path_the_shape_does_not_hold_names_no_set` passed against a stub
    returning `None`; dropping the dot check, accepting a bare file, or
    accepting any file or directory after the set directory each fail it.
  - `a_path_to_a_slot_directory_resolves_to_its_set` and
    `a_path_to_a_findings_directory_resolves_to_its_set` pass against the
    old walk, which resolved any path; accepting any directory after the
    set directory fails the domain's slot-directory test.
  - The research tests were written after the adapter: the lister
    keeping a rootless directory fails
    `a_topic_set_without_its_manifest_is_unresolved`, and accepting any
    file fails `a_path_to_the_run_ledger_is_unresolved`. Accepting a
    rootless set directory by path fails the rootless golden.
- **Review, 2026-11-27.** The `wrpxyvrw`..`wnnwknym` diff, with its
  `SKILL.md` and CHANGELOG edits, adds no line past 80 columns outside the
  `public-api.txt` snapshot. Doc comments that restated their code
  (`EntryKind`, `set_directory_of`, the adapter's `set_directory_holding`)
  are dropped.
- **Manual checks, 2026-11-27.** Both ran against this checkout's builds
  (`cargo build -p accelerator -p accelerator-corpus -p
  accelerator-research`), through `bin/accelerator` first on `PATH` with
  the `.accelerator-dev-launcher` marker,
  `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER=1`, `ACCELERATOR_LAUNCHER_BIN`,
  `ACCELERATOR_CORPUS_BIN` and `ACCELERATOR_RESEARCH_BIN` naming
  `cli/target/debug/`. Without the overrides the launcher runs the
  cached release `pre.113` corpus binary, which still resolves a
  `prototype-full/` path to its set.
  - `accelerator corpus resolve --type design-inventory <input>` printed
    the set directory and exited 0 for each of the six real inventories
    by slug (e.g. `135214-current-app`), by directory
    (`meta/research/design-inventories/2026-05-06-135214-current-app`) and
    by a screenshot (`…/screenshots/kanban-board.png`). For both
    inventories with a `prototype-full/`, the directory and its HTML file
    (`…/2026-09-10-174311-claude-design-prototype/prototype-full/Accelerator
    Visualiser.html`) exited 3 with `E_RESOLVE_NOT_FOUND: no
    'design-inventory' set at path '…'`.
  - In two scratch jj repos holding a copy of
    `cli/corpus-cli/tests/fixtures/topic-research-set` at
    `meta/research/topics/example-subject` (`status: synthesised`),
    `claude -p "/accelerator:research-topic outline <input> --breadth 2"
    --plugin-dir <checkout> --permission-mode acceptEdits --allowedTools
    'Bash(accelerator *)' Read Write Edit Glob Grep` exited 0 for
    `example-subject` and for
    `meta/research/topics/example-subject/findings`. Each ran
    `accelerator corpus resolve --type topic-research <input>`, appended a
    two-item `## Round 2` to `outline.md` and reopened the manifest to
    `status: researching`. Run directly in the scratch repo, the local
    resolver printed the set for both inputs and exited 3 for
    `…/example-subject/notes.txt`.
- **`mise run`** exited 0 on the final code, with `test:unit:cli`
  coverage at lines 94.45%, functions 93.79%, regions 93.19%.

---

## Phase 9: Index consumers tell roots from members

### Overview

Index records gain set membership. The indexer's default accessors
return only roots and ordinary documents, so a consumer sees members
only when it asks for them by name. Tests use fabricated member records
through `test_support`, because
nothing on disk produces members yet. Listings, counts and clusters are
therefore unchanged in production. The only visible change is the new
`setId` and `setRelativePath` fields on roots.

### Changes Required:

#### 1. Failing tests first

Use `test_support` factories that build a root and member records for a
set. The factories can give a member a `Some` slug, an `id`, a status
and linkage frontmatter that match the root's, so every test below
proves that the member's role, and not a missing field, keeps it out.

- **Default accessors hold no member.** One test per default accessor:
  `all`, `all_by_type`, `counts_by_type`, `library_aggregates`, the
  cluster input of `recompute_clusters`, and the secondary maps built in
  `rescan` (`documents_by_address`, `work_item_by_identity_key`,
  `reviews_by_target`). Each is given a set whose member collides with
  the root on every key that accessor reads, and asserts that no member
  is returned or counted.
- **Explicit accessors.** `set_members(dir)` returns exactly the set's
  members in `setRelativePath` order. `documents_including_members()`
  returns roots, ordinary documents and members. `get(path)` returns a
  member by its own path.
- **Type listing and counts.** `all_by_type` and `counts_by_type` return
  only the root (criteria 1 and 9).
- **Library aggregates:**
  - `count` is 1;
  - `status` and `clusterSlug` facets hold the root's values only, even
    when members carry other status and slug frontmatter;
  - the set's latest time is the newest root or member `mtime_ms`, and
    the doc type's latest follows it (criterion 11, without removals).
- **Clusters.** `recompute_clusters` excludes members. A root's cluster
  and its members are the same with members present as absent, for
  members whose frontmatter names the root's cluster key (criterion 10).
- **Related:**
  - a member whose `parent` names work item X returns X as outbound;
  - the root and another member do not;
  - X returns the member as inbound (criterion 18);
  - no inferred item is a member, for a member that shares the root's
    cluster and a member and a plan sharing a slug in their filenames
    (criterion 36). The fixture gives the member a `Some` slug equal to
    the plan's, so the default accessors, and not a missing slug, decide
    the outcome.
- **Search:**
  - a phrase in a member's first paragraph returns that member, titled as
    the member, with `setRelativePath`, `setId` and `slug` equal to the
    set's slug (criterion 15);
  - searching the set's slug ranks the root first. A member whose stem
    and title both equal the set's slug is outside the exact-key bucket
    and ranks below the root (criterion 16);
  - search never matches a member on its link keys: a member that
    carries the set's `id` only as its `setId` and link `id`, and not in
    its title, stem or first paragraph, is not matched by a search for
    that `id`.
- **Search projection.** Every row's `slug` and `setId` come from
  `link_keys_of`, pinned for a root, a member and a non-set document.
- **Wire fields.** `setId` is the root's `id`, or the directory name when
  the root is malformed or has none. It appears on the type listing
  entry, a member search row, and a related item naming a member
  (criterion 28, minus the member API).

#### 2. Membership on records

**File**: `server/src/indexer.rs`

```rust
pub struct SetContext {
    pub set_id: String,
    pub set_slug: Option<String>,
    pub set_directory: PathBuf,
}

pub struct SetPlacement {
    pub context: Arc<SetContext>,
    pub set_relative_path: String,
}

pub enum SetDocumentRole { Root, Member }

pub struct SetDocument {
    pub placement: SetPlacement,
    pub role: SetDocumentRole,
}

pub struct LinkKeys {
    pub slug: Option<String>,
    pub id: Option<String>,
}
```

- Add `pub set: Option<SetDocument>` to `IndexEntry`. An asset never
  becomes an `IndexEntry`, so the role admits only `Root` and `Member`.
- Serialise `set` with `#[serde(flatten)]` through a `SetWireFields`
  projection that writes only `setId` and `setRelativePath`, both `null`
  outside a set.
- Add `IndexEntry::is_member()` and `is_set_root()`, both read from
  `role`. Only the indexer's accessors call `is_member()`.
- The set's context is built once per set, from the root, and shared by
  the root and every member. `set_slug` is the set directory's slug, so
  no consumer looks the root up to find it.
- `test_support` factories build set records through the same
  `SetDocument` constructor the indexer uses in Phase 10.
- **A member's `slug` is `None`,** because no slug rule applies to a
  member's file name. It is not what keeps a member out of listings,
  clusters or related items; the accessors are. A member's search key
  is its own filename stem, through a `search_key()` method:
  - for a member, the stem;
  - for anything else, `slug`.
- `set_id_of(root_frontmatter, set_directory)` is derived once per set.

#### 3. Accessors

**File**: `server/src/indexer.rs`

- **Default accessors** return roots and ordinary documents only: `all`,
  `all_by_type`, `counts_by_type`, `library_aggregates`, and the
  secondary maps. Each filters on the role in one private iterator,
  `listed_entries()`, which every default accessor reads from.
- **One map builder.** The secondary maps and the `set_directory ->
  latest` map are built by one `IndexMaps::build(records, &ledger)`,
  which `rescan` calls; the removal ledger is empty until Phase 12. A
  test-only `Indexer::from_records(records)` calls the
  same builder, so Phase 9's fabricated members pass through the
  production map construction. Every default-accessor test goes through
  it; Phase 10 replaces none of them.
- **Explicit accessors** reach members:
  - `set_members(dir)`, for the member API;
  - `documents_including_members()`, for search and for declared
    linkage in related items;
  - `get(path)`, for a request that names a document by path.

#### 4. Consumer rules

Consumers read the accessor that matches their rule, and carry no
member filter of their own.

- **`all_by_type`, `counts_by_type` and `library_aggregates`** read
  `listed_entries()` inside the indexer. Aggregates' `latest` for a set
  root reads the
  set's `latest_change_ms` from a `set_directory -> latest` map. The
  indexer computes it during `rescan` as the maximum over the root and
  members, and swaps it with the other maps under the same ordered
  locks, so aggregates read it in O(1) per root. Phase 10 adds assets and
  Phase 12 adds observed removals.
- **Clusters.** The three copies of the recompute (`server.rs:116-146`,
  `watcher.rs:228-255` and `api/docs.rs:276-311`) are identical apart
  from their receiver. Extract one `recompute_clusters(&indexer, &clusters)`
  that reads its entries through the public, root-only `all()`, and
  replace all three call sites with it. It carries no member filter of
  its own.
- **`related.rs`:** declared linkage reads `documents_including_members()`,
  so a member's own `parent` is attributed to the member and the member
  appears as an inbound item. Inbound lookup stays on the precomputed
  `work_item_refs_by_target` map (`indexer.rs:990-1016`), which
  `IndexMaps::build` fills from members as well as roots; it is the one
  secondary map that includes members, because it records declared
  linkage. `resolve_related` makes no scan of its own, so
  `collect_linked_counts` stays linear. Inferred items read the default
  accessors,
  so no member is ever inferred.
- **`api/search.rs`:**
  - candidates come from `documents_including_members()`;
  - `classify` uses `search_key()`;
  - `project` emits a member row via `link_keys_of(entry) -> LinkKeys`, a
    pure function of the entry: for a member, the context's `set_slug`
    and `set_id`; for others, their own;
  - add `set_id` and `set_relative_path` to `SearchResultRow`;
  - follow the PP-996 overlap rule for `id`.
- **Frontend types.** Add `setId: string | null` and
  `setRelativePath: string | null` to `IndexEntry` and `SearchResult`, and
  normalise missing values to `null`, with `fetch.test.ts` cases for
  responses that omit them.
- **Search result keys.** A member row shares the set's `docType` and
  `slug`, so `SearchResultsPanel.tsx:68` adds `setRelativePath` to its
  row key (`${docType}/${slug}/${setRelativePath ?? ""}`). A
  `SearchResultsPanel` test renders a root and two members of one set
  and finds all three rows.

### Success Criteria:

#### Automated Verification:

- [x] `mise run test:unit:visualiser`, `mise run test:integration:visualiser`
- [x] `mise run frontend:check`, `mise run test:unit:frontend`
- [x] `mise run`

#### Manual Verification:

- [x] The library, lifecycle and search in this repo look unchanged.

### Implementation notes

- **Beyond the plan: lifecycle non-determinism (user request,
  2026-11-26).** Two runs of one build against this repository's `meta/`
  gave different `/api/lifecycle` order and different `linkedCount`s for
  five documents whose slugs collide. Two causes, each fixed test-first
  in its own commit:
  - `corpus::cluster::compute` read hash-map and input order. Clusters
    sharing a slug kept `buckets.into_values()` order; every
    orphan-by-design document keeps its own slug as its cluster's, so
    such ties are common. Members tied on rank and mtime kept arrival
    order, which follows `indexer.all()`'s hash-map order. A slug two
    documents' cluster keys claimed went to whichever came last. Each
    tie now falls to the lowest path, as `hold_if_lower` already does
    (`wlyozyql`). `clustering_does_not_depend_on_the_order_entries_arrive_in`
    and `every_build_over_colliding_slugs_serves_the_same_lifecycle`
    pin it.
  - `related::resolve_related` gave an entry without a cluster key the
    first cluster sharing its slug, not the cluster holding it. So
    `ADR-0021-template-management-subcommands.md`, an orphan singleton,
    saw the research, plan and review cluster of the same slug (0 or 3),
    and those saw the ADR (1 or 2); the three
    `current-app-vs-claude-design-prototype` design gaps saw each other
    (0 or 1). An entry without a key now takes the cluster with its slug
    that holds it (`LifecycleCluster::holds`, `pxvmoonm`), as the
    composite-key plan intended: orphan types never merge with
    slug-mates, and `/api/related` agrees with `/api/lifecycle`.
    `an_entry_infers_only_the_cluster_it_is_in_when_slugs_collide` pins
    it.
  - Four runs of the old debug build differed on exactly those documents
    and on `/api/lifecycle`; four runs of the fixed build gave identical
    responses, with ADR-0021 at 0, the research document at 2 and the
    design gaps at 0.
- **Membership.** `IndexEntry.set: Option<SetDocument>` serialises as
  `setId` and `setRelativePath`. `build_entry` gives a set root its
  `SetContext`, with `set_id` its frontmatter `id` or else its
  directory's name, and its root filename as its set-relative path.
  `search_key()` and `link_keys_of` sit beside it. `test_support` builds
  set records through `SetDocument::root` and `::member`, over a
  test-only `dossiers` set type that carries every key the maps and
  facets read; `set_member_for_test` copies its root's keys.
- **Accessors.** A private `listed()` filter backs `all`, `all_by_type`,
  `counts_by_type`, `library_aggregates`, `release` and the root-only
  maps. `IndexMaps::build` builds every map for `rescan` and for the
  test-only `Indexer::from_records`; `install` swaps them under the
  ordered locks. `latest_change_by_set` is swapped with them, and
  `refresh_one` recomputes the affected set's entry. Beyond the plan,
  `publish_document_ids` counts only listed documents, so a member
  sharing its root's `id` leaves the root its `id`; a member's own `id`
  is its frontmatter `id`.
- **Departures.**
  - `IndexMaps::build` takes no removal ledger; Phase 12 adds it.
  - `SearchResultsPanel`'s row key is unchanged. PP-996's key already
    includes `relPath`, which is unique per member; the new test fails
    against a slug-only key.
  - Search's third test bullet is reworded (user's decision,
    2026-11-27). As first written it contradicted criterion 15: search
    matches the first paragraph, so a member whose first paragraph holds
    the set's `id` is found by it. The test now pins that search never
    matches a member on its link keys.
- **Strict TDD.** The wire-field, refresh, `link_keys_of`,
  `search_key()`, three search, fetch and non-determinism tests were
  seen failing first. Two slips: the accessor and map tests in
  `set_membership_tests`, and the first search tests, were written in
  the same edit as their code. Guards that passed against existing
  code, and both slips, were checked by mutation; each mutation failed
  at least one test:
  - reading every entry in `all`, `all_by_type`, `counts_by_type`,
    either aggregates pass, the identity and address maps or
    `reviews_by_target`;
  - `work_item_refs_by_target` dropping members;
  - a set root's latest ignoring its members;
  - `set_members` in reverse order;
  - `recompute_clusters` reading members;
  - `publish_document_ids` counting members;
  - search candidates leaving out members, keying a member on its slug,
    letting a member take the exact-key bucket, or matching a member's
    link `id`;
  - a non-set entry or search row serialising a `setId`;
  - a slug-only search row key.
- **IDE diagnostics.** rust-analyzer in the IDE flagged E0107 at ten
  `indexer.rs` lines. None of those lines carries a generic argument
  list, `cargo` and clippy are clean, `rust-analyzer diagnostics` reports
  no E0107, and the IDE's inspection of the current file reports no
  errors, so it was stale analysis.
- **`mise run`** exited 0 on the final code, with `test:unit:cli`
  coverage at lines 94.46%, functions 93.80%, regions 93.20%.
- **Manual check, 2026-11-27.** Debug `dev-frontend` builds of
  `rkpnpmwu` (Phase 8, from a temporary jj workspace with its own
  `CARGO_TARGET_DIR`) and `nttuyyxl` (Phase 9) each ran `serve` against
  this repository's `meta/`. Two Phase 8 runs and four Phase 9 runs
  fetched `/api/types`, `/api/library/structure`, `/api/lifecycle`,
  `/api/templates`, `/api/docs` for every type, and `/api/search` for
  `current-app`, `135214-current-app` (an inventory's slug),
  `template`, `visualiser`, `inventory`, `crawler` and `react 19 spa`
  (from an inventory's first paragraph).
  - Every Phase 9 run's responses were byte-identical. The two Phase 8
    runs differed on `/api/lifecycle` and on `design-gaps`
    `linkedCount`s.
  - `/api/types`, `/api/library/structure` and `/api/templates` are
    identical across builds.
  - `/api/docs` differs only in the new `setId` and `setRelativePath`
    fields (an inventory root's set id and `inventory.md`, null
    elsewhere) and in the `linkedCount`s the fix settles: the
    template-management research document 1 to 2, and the three
    `current-app-vs-claude-design-prototype` design gaps 0 or 1 to 0.
    ADR-0021 read 0 in both Phase 8 runs here, and 3 in earlier ones.
  - `/api/lifecycle` holds the same clusters with the same entries;
    only the order of clusters sharing a slug, and of entries tied on
    rank and mtime, differs, and is now fixed.
  - Search returns the same rows in the same order, with the two new
    fields. No member appears, since nothing enumerates members until
    Phase 10, and this repository holds no topic research sets.
  - Through the Vite dev frontend (`npm run dev`, proxying to the
    Phase 9 server) driven by Playwright, the
    library, a design inventory listing, the lifecycle index, ADR-0021's
    page (no related items; a one-artifact cluster) and a search for
    `current-app` (9 rows) render. The lifecycle index logs React
    duplicate-key warnings for `current-app-vs-claude-design-prototype`
    and `template-management-subcommands`, the same against the Phase 8
    server: `LifecycleIndex.tsx` keys its rows by cluster slug, which
    orphan clusters share. This predates Phase 9 and is left as a
    finding.
- **Review, 2026-11-27.** The `wlyozyql`..`nttuyyxl` diff adds no source
  line past 80 columns apart from three Biome-formatted test titles,
  which Biome does not wrap. Two comments that restated their field
  names (the frontend `setId`/`setRelativePath` pair and
  `latest_change_by_set`) are trimmed (`nvsyzrzq`); `mise run` exited 0
  again after the trim, with unchanged coverage.
- **Build trap.** Two jj workspaces sharing one `CARGO_TARGET_DIR` reuse
  each other's workspace-crate artifacts: a build in one can link a
  crate compiled from the other's sources. Give a comparison build its
  own target directory.

---

## Phase 10: Enumerate and index set members

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
- A path through a symlinked ancestor to an unslotted file returns 404
  on every platform.
- A case-variant path (`meta/Research/Topics/<set>/.conduct-run.json`)
  returns 404. The test probes the tempdir's case sensitivity, as
  Phase 12's real-OS step does, and skips with a log line on a
  case-sensitive volume, where the path cannot resolve.
- A directory path under a non-set root returns 404, not 500.
- The document endpoint refuses writes to anything but work items
  (`OnlyWorkItemsAreWritable`, `api/docs.rs:189-191`), so no set member
  or root is writable through it; a PATCH to a member returns that
  refusal.

**File**: `server/src/indexer.rs` (inline tests, single-path refresh)

- `refresh_one` on a member document leaves an entry equal to the one a
  full rescan builds, and `all_by_type` and `counts_by_type` unchanged.
- `refresh_one` on an unslotted file and on a dot-prefixed file in a set
  directory leaves both unindexed.
- Deleting a member and calling `refresh_one` removes only that member.
- Changing the root's frontmatter `id` on disk and calling
  `refresh_one` on the root moves every member's `setId` to the new id.
- After an external write to a member's frontmatter and `refresh_one`,
  listing the type still shows one entry, and the member keeps its
  `setId`.

**File**: `server/src/file_driver.rs` (inline tests, Unix only)

- A slot-matching asset symlink pointing outside the doc roots, and a
  symlinked `findings/` directory, are both absent from `list_sets`,
  mirroring `read_rejects_symlink_escape`.

**File**: `server/tests/api_related.rs` (on an enumerated fixture set)

- A member with `parent: X` has X as outbound; the root and a sibling do
  not.
- X has the member as inbound, and the related item carries `setId`.
- No inferred item is a member (criteria 18, 28 and 36 on real records).

**File**: `server/tests/api_search.rs`

- `a.png` returns no result.
- `prototype-full` content is not searchable.

#### 2. Enumeration

**File**: `server/src/file_driver.rs`

- Add `list_sets(kind) -> Vec<SetListing>`, the single walk of a set
  type's directories, where
  `SetListing { directory, root, documents: Vec<String>, assets: Vec<AssetListing> }`
  and `AssetListing { set_relative_path, size, mtime_ms }`.
- For each non-dot directory under a set doc root that has a root file,
  walk it recursively (bounded by the deepest slot's segment count).
  Skip dot-prefixed names at every depth, descend only directories for
  which `matcher.may_contain_members(dir)` holds, and classify each
  file's set-relative path through `matcher.classify`, where `matcher`
  is `doc_types.set_matcher(kind)` on the driver's `Arc<DocTypeCatalogue>`.
- The walk reads entries with `symlink_metadata` and does not descend a
  symlinked directory. It canonicalises each slotted file and drops any
  that fall outside the allowed roots, through the same
  `path_is_allowed` check `read` uses.
- Asset size and mtime come from the walk's own metadata, so asset bytes
  are never read and no second stat pass is needed.
- Build relative paths with `/` separators.
- `list_sets` joins the `FileDriver` trait (`file_driver.rs:61`), so
  `rescan` stays port-driven and the indexer's test double can supply
  sets. `list(kind)` serves flat types only; on a set type it returns
  `FileDriverError::SetType`, pinned by a test.

#### 3. Index model

**File**: `server/src/indexer.rs`

- `rescan`: for set types, read each `SetListing` as a group. Read the
  root first and build its `SetContext` (`set_id`, `set_slug`,
  `set_directory`), then build the member documents against that
  context, so every record of one set carries the same `setId`. Member
  reads go through the same `buffered` stream as other reads.
  - The set slug comes from the set directory name, not the immediate
    parent; this replaces the `indexer.rs:1512-1521` branch.
  - Members use their own filename for `title_from`'s fallback.
- Add an `assets: HashMap<PathBuf, AssetRecord>` map, swapped with the
  others under the same ordered locks.
  `AssetRecord { doc_type, path, rel_path, placement: SetPlacement, size, mtime_ms }`.
  It reuses Phase 9's `SetPlacement`, so anything a member carries is
  added in one place.
- A set's `latest_change_ms` is the maximum over its root, its member
  documents and its assets.
- `refresh_one` on any path inside a set directory runs a full rescan,
  as the watcher already does, because a member's record depends on its
  root. `refresh_entry` and `find_entry_for_deleted` keep their
  single-path behaviour for flat types only. Phase 12 routes every full
  rescan, this one included, through `rescan_with_diff`.

#### 4. Document endpoint

**File**: `server/src/api/docs.rs`

Canonicalise the requested path first, and decide whether it sits under
a set doc root on that canonical path, against the canonical roots. A
case variant or a symlinked ancestor therefore resolves to the same
decision as the stored path. The decision fails closed: a path counts
as under a set root when it matches either the raw or the resolved form
of that root. A test creates a set root after startup and asserts an
unslotted file under it returns 404. Under a set doc root, serve only an indexed
document record, and 404 everything else, including directories.
Elsewhere the unindexed fallback stays as today, except that a directory
returns 404 instead of 500.

#### 5. CHANGELOG

Add a `### Changed` entry under `[Unreleased]`: `/api/docs` returns 404
for files in a set directory that are not member documents, and for
directories.

#### 6. e2e

`rg -n "search" cli/visualiser/frontend/e2e` covers the specs that assert
search results. Update any count that the `prompt-caching-economics`
fixture's members now change. Check that the design-inventory listing and
page specs still pass unchanged.

### Success Criteria:

#### Automated Verification:

- [x] `mise run test:unit:visualiser`, `mise run test:integration:visualiser`
- [x] `mise run test:e2e:visualiser`
- [x] `mise run`

#### Manual Verification:

- [ ] Against this repo's six real inventories, `/api/docs?type=design-inventories`
      holds six entries. Searching a finding title in a topic fixture
      returns the member and opens the set.
- [ ] Indexing time with all real inventories is not noticeably slower:
      compare `rescan` timing in the debug log before and after.

### Implementation notes

- **Enumeration.** `FileDriver::list_sets(kind)` walks each non-dot
  directory holding the set's root, skipping dot-prefixed names at every
  depth, descending only directories `may_contain_members` admits, and
  classifying each file through the catalogue's `SetMatcher`. Entries are
  read with `symlink_metadata`, so a symlinked directory is never walked;
  a slotted file is canonicalised and dropped outside the allowed roots.
  Beyond the plan, a symlink is kept only when its target is a file, so a
  symlinked directory named like a member (`findings/02-b.md`) is not
  listed as a document whose read would fail the rescan. Documents and
  assets are sorted by set-relative path, and sets by directory. `list`
  refuses a set type with `FileDriverError::SetType`, and the per-type
  listing table test now covers flat types only.
- **Index model.** `rescan` reads a set type through `list_sets`: each
  set's root is read first and builds the `SetContext`; its member
  documents follow it in the same `buffered` stream and are built against
  that context, so a member whose root is gone is skipped. `build_entry`
  takes a `Placement` (`Flat`, `SetRoot(dir)`, `SetMember(context, path)`):
  a root's slug comes from its set directory, a member has no slug and
  takes its title fallback from its own stem. Assets become `AssetRecord`s
  in an `assets` map swapped in `install` after `latest_change_by_set`,
  read through `set_assets(dir)` in set-relative order, and counted in
  the set's latest change.
- **Refresh.** `refresh_one` on any path whose canonical form, or nearest
  existing ancestor's, lies in a set type's doc root runs a full rescan
  and returns the path's entry. `refresh_latest_change` became
  unreachable, since only flat documents take the single-path route, and
  is gone.
- **Document endpoint.** The driver records each set type's doc root as
  configured and as resolved, and `lies_in_a_set_doc_root(canonical)`
  matches either. `doc_fetch` returns 404 for a path whose canonical form
  lies there and that the index does not hold. `read` reports a directory
  as not found, so a directory is 404 under every root. The write
  refusal needed no new code: a member's PATCH is refused by the kanban
  guard, and the driver's writable-root check would refuse it too.
- **Fixtures.** The topic fixtures' prose that said a finding and a
  report "must not be separately indexed" now says they are indexed as
  member documents. No e2e count changed: neither member's text matches an
  e2e search.
- **Strict TDD.** Each test below was seen failing before its code, by a
  compile error and then, after a minimal stub (`list_sets` returning no
  sets, `set_assets` returning none), by its assertion:
  - every `file_driver` `list_sets` test, the symlinked-directory test
    (seen failing against the first implementation), and
    `list_refuses_a_set_type`;
  - the two rewritten pinned tests and every `set_enumeration_tests` test
    but three guards;
  - the `api_docs` asset, unslotted, working-file, rootless, directory
    (set and flat), symlinked-ancestor, case-variant (run, not skipped, on
    this volume) and created-after-startup tests.
  Guards that passed against existing code were checked by mutation; each
  mutation failed at least one test:
  - `list_sets_of_a_missing_doc_root_is_empty`: erroring on a missing
    directory;
  - the escape and symlinked-slot tests: dropping the allowed-roots check,
    or reading entries with `metadata`;
  - `a_dot_prefixed_document_of_a_flat_type_is_indexed`: skipping dot
    names in flat listings;
  - `an_inventory_lists_as_its_inventory_document`: indexing a root as a
    member;
  - `a_directory_without_its_root_indexes_nothing` is a second guard:
    listing rootless directories fails the driver's listing test, while
    the indexer still skips their members because no root is read;
  - `a_member_document_is_served`, `an_inventory_root_is_served_unchanged`:
    refusing every path in a set doc root;
  - `a_set_member_cannot_be_written`: survives bypassing the kanban guard
    alone, since the writable-root check also refuses; fails with both
    bypassed.
  One slip: the `api_related` and `api_search` tests were written after
  enumeration landed, so they never ran red against the old code. Hiding
  every member from the listing fails all three related tests; listing
  every file as a member document fails both search tests. Members of
  the built-in set types can never be inferred items, since topic
  research is unclustered and outside the lifecycle, so
  `no_inferred_item_is_a_member` survives `recompute_clusters` reading
  members; Phase 9's `no_member_is_ever_an_inferred_item`, over a
  clustered test-only type, is the test that fails on it.
- **`mise run`** exited 0 on the final code, with `test:unit:cli`
  coverage at lines 94.46%, functions 93.80%, regions 93.20%, and the
  e2e suite at 360 passed, 1 skipped.

---

## Phase 11: The member API

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
- **Duplicate `setId`.** Two sets whose roots share a frontmatter `id`:
  the lexically first directory is returned.
- **Encoded `setId`.** A set whose directory name holds a space and a
  literal `%` is found by its percent-encoded id, decoded exactly once.
- **Order.** `members` is in `setRelativePath` order, written to disk in
  a different order.
- **Inventory clutter.** For a design inventory holding
  `prototype-full/x.html`, `assets/y.svg` and `notes.txt`, none of them
  is a member (criterion 8).

#### 2. Handler

**File**: `server/src/api/sets.rs` (new), mounted in `api::mount`
(`api/mod.rs:26-53`)

- Resolve the type through `state.doc_types`, and 404 unless
  `kind.is_set()`. The presentation comes from the paired set
  declaration, `state.doc_types.set_matcher(kind)`'s `presentation()`.
- Register the route as `/api/sets/:type/:set_id`, in matchit 0.7's
  syntax like its neighbours. The `Path` extractor's decoded `set_id` is
  used as is, with no second decode.
- Find the root via `indexer.set_root(type, set_id)`, taking the
  lexically first directory on a duplicate.
- Collect the documents by `set_directory` with `indexer.set_members(&dir)`,
  and the assets from the asset map. Project to the spec's wire types,
  sorted by `setRelativePath`. Both lookups scan the maps; at today's
  scale no extra index is added.
- Follow the `related_get` validation and 404 shape
  (`api/related.rs:30-57`).

#### 3. Frontend types

**File**: `frontend/src/api/types.ts`

Add `SetResponse`, `SetDocumentMember` and `SetAssetMember`. No fetcher
is added; PP-1001 adds one.

#### 4. CHANGELOG

Add an `### Added` entry under `[Unreleased]` for
`GET /api/sets/{type}/{setId}`.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test:integration:visualiser --filter set_` (every
      `api_sets.rs` test name starts with `set_`)
- [ ] `mise run server:check`, `mise run frontend:check`
- [ ] `mise run`

#### Manual Verification:

- [ ] `curl localhost:<port>/api/sets/design-inventories/2026-05-21-015231-claude-design-prototype`
      on this repo lists only `inventory.md` and its screenshots.

---

## Phase 12: Live update, activity roll-up and acceptance

### Overview

The watcher reacts to assets and directory renames inside set roots and
ignores dot paths there. Activity rolls up per set on the server, and the
live feed shows the server's rows. The latest time
counts removals. The phase ends with the genericity and source-boundary
acceptance tests.

### Changes Required:

#### 1. Failing tests first

**File**: `server/src/watcher.rs` (the `Scripted` harness, `:779-889`,
extended with helpers that write under a set doc root)

These tests assert events, actions and activity rows only. Assertions
on the member API and `/api/docs` for the same scenarios live in
`server/tests/sse_e2e.rs`, through `InjectedWatcher`.

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
  `notes.txt`, with events injected for the directory and all four
  children at once, emits exactly three created events. Activity holds
  one row reading created (criterion 30).
- **Root deletion.** Deleting the root while two members remain emits
  three deleted events. The row reads deleted (criterion 32).
- **Concurrent rescans.** Events for two paths of one change, released
  together past the debounce, emit each change once.
- **Set-level debounce,** counted through the recording `ChangeSink`'s
  number of `rescan_with_diff` calls:
  - ten member writes inside one set, within the debounce window, cause
    one rescan;
  - one write in each of two sets within the window causes two rescans
    and an event for each path;
  - a write under a flat root is debounced on its own path, as today.
- **Ordered emission.** Create a member, and while its rescan's sink
  call is held by a gated sink, delete the member. The events arrive as
  created then deleted, and the set's activity row ends as the deletion.
- **Trust flip.** Flipping trust while a member has changed on disk
  emits one event for it and updates the set's latest time.
- **Startup.** Building the indexer over a populated tempdir emits
  nothing and leaves the activity feed empty.
- **Non-set doc types.** A dot-prefixed file in a non-set root still
  emits (criterion 33).

**File**: `server/tests/sse_e2e.rs`

- After the case-only rename, the member API lists `screenshots/a.PNG`
  once.
- After the rename into place, the member API returns the root and two
  members (criterion 30).
- After the root deletion, the listing has no entry and both members
  return 404 from the member API and `/api/docs` (criterion 32).

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
  (criterion 34). Removing an asset does the same. A screenshot change
  bumps the latest time (criterion 11). Observed times are bounded by
  `Utc::now()` captures taken before and after the rescan.
- **Recreated set.** A set deleted and then recreated has a latest time
  that ignores the earlier removal.
- **Case-only rename.** Renaming `a.png` to `a.PNG` leaves the latest
  time at the file's mtime, with no removal observed.

**File**: `server/tests/generic_set_model.rs` (new; criteria 20 and 23)

- **Setup.** Declare `static NOTEBOOKS` with `listing: Listing::Set`,
  and its set declaration, shape and presentation together, as an owner
  declares one:
  `static NOTEBOOKS_SET: SetDeclaration = SetDeclaration { shape: SetShape { root: "index.md", member_slots: &[docs/*.md Document, img/*.png Asset] }, presentation: Presentation::SetPage }`.
  Configure the server with
  `catalogue_with(&[&NOTEBOOKS], &[(notebooks, &NOTEBOOKS_SET)])`,
  which builds through the constructor the assembly uses. Build a
  tempdir set holding the root, two `docs/*.md`, one
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

**File**: `cli/corpus-adapters/tests/doc_type_declarations_are_single_source.rs`
(criterion 21)

Criterion 21 allows the two set types to be named only in their
declarations (corpus and declarations crates), the assembly, domain-owning
code and the frozen migrations, with test modules unsearched. That is
exactly Phase 4's guard with Phase 6's allow-list, which walks every
`src/**/*.rs` under `cli/`. `server/src` has no allow-list entry: set
declarations are made in the declarations crates and paired with their
types in the assembly, never in a root, and after Phase 4 a doc type's
library phase is a declared rule, so the library's phase table needs no
exception. Add a named case, `visualiser_server_names_no_set_doc_type`,
that seeds `DocTypeKey::TOPIC_RESEARCH` into a copy of `api/search.rs`'s
source and asserts the scanner reports it, and asserts that the real
`server/src` scan found files and no violation.

#### 2. Watcher

**File**: `server/src/watcher.rs`

Replace `is_markdown` with `should_rescan(path, &roots)`, checked in
this order:

- under a set doc root, a path with a dot-prefixed component below the
  root is rejected, whatever its extension;
- under a set doc root, a path is admitted when it is a set directory, a
  file `matcher.classify` places, or a directory for which
  `matcher.may_contain_members` holds, where `matcher` is
  `doc_types.set_matcher(kind)` on the watcher's
  `Arc<DocTypeCatalogue>`;
- elsewhere, markdown, as today.

Directory events under a set root trigger a rescan, which makes renames
independent of how the OS backend reports children. Unslotted trees such
as `prototype-full/` admit nothing. `rescan_changes` also diffs the asset
map on `(size, mtime_ms)` and emits `doc-changed` for assets, with no
`etag`.

**One diff per rescan.** Today each debounced path task reads its
before-snapshot outside `rescan_lock`, so tasks released together can
diff against the same before-snapshot and emit one change twice. Add
`Indexer::rescan_with_diff(&dyn ChangeSink)`. Under the one
`rescan_lock` permit, in this order, it:

1. snapshots `documents_including_members()` and the asset map;
2. rescans and snapshots again;
3. builds the paired `IndexDiff` of changed documents and assets;
4. updates and clears removal ledger entries from that diff;
5. builds every map once through `IndexMaps::build(records, &ledger)`,
   which takes the ledger as an input, so the `set_directory -> latest`
   map has one builder and the maps are swapped once;
6. hands the diff to the `ChangeSink` it was given, still under the
   permit, so events from successive rescans leave in the order their
   diffs were taken.

Steps 1 and 2 read the index's state, and step 2's rescan builds records
without swapping; the swap happens only in step 5.

The sink is passed per call, never stored on the `Indexer`: the sink's
recompute reads the `Indexer`, so storing it would make a cycle, and
`AppState::build` (`server.rs:94-151`) creates the `Indexer` before the
hub, the ring buffer, the clusters and the `WriteCoordinator` exist.
The watcher and the PATCH handler both hold those collaborators and
build the sink at the call. `refresh_one(path, &dyn ChangeSink)` takes
one too, because a set path or a trust flip runs `rescan_with_diff`;
its existing test call sites pass a recording sink.

Today's `pub async fn rescan` (`indexer.rs:316`) becomes a private,
non-swapping `build_records`, used by the constructor and by
`rescan_with_diff`, so `rescan_with_diff` is the only public full-rescan
method and no caller can swap maps without recording removals or
calling a sink. The watcher's direct call (`watcher.rs:223`) and test
callers move to `rescan_with_diff`.

The indexer owns the snapshot, diff, ledger and map steps, and knows
nothing of SSE, the activity feed or clusters. The server's
`ChangeSink`, wired in `server.rs`, holds the SSE hub, the ring buffer,
the `WriteCoordinator` and the clusters. For each diff it:

1. runs `recompute_clusters` and its completeness, cluster-key and
   linked-count backfills when the diff is non-empty, so a client that
   refetches on the event sees fresh clusters;
2. drops every change whose path `write_coordinator.should_suppress`
   matches, as `on_path_changed` does today (`watcher.rs:260-263`);
3. calls `emit` for each remaining change.

Indexer tests pass a recording sink. The `Scripted` watcher tests pass
a `TestSink` that wraps the production sink, delegating to it while it
counts calls and, when asked, holds a gate, so one sink drives both the
rescan counts and the activity and SSE assertions. A test makes an
external edit that changes a work item's cluster key, reads
`/api/lifecycle` as soon as its event arrives, and sees the new
cluster; a member-only diff leaves clusters unchanged.

`rescan_with_diff` is the only full-rescan entry point after start-up.
The watcher's path tasks, `refresh_one` for set paths, and the
trust-flip branch (`indexer.rs:486`) all call it, so a change another
tool makes during an in-app write is emitted and recorded by whichever
rescan sees it first. The constructor's initial build (`indexer.rs:308`)
is the one exception: it builds the first index privately, with no diff
and no sink, because nothing has been observed before it.

**In-app writes.** Only work items are writable in-app
(`OnlyWorkItemsAreWritable`, `api/docs.rs:189-191`), and they are flat,
so an in-app write never touches a set. The PATCH handler keeps today's
order: `refresh_one(path)`, a single-path refresh (or `rescan_with_diff`
on a trust flip), then its own event with the `FileContent` etag. The
only change is that the event goes through `emit`, so the ring buffer
records it, and the handler calls `recompute_clusters` instead of its
own copy. The watcher's task for the written path, and the sink for any
rescan that sees it, suppress it through `write_coordinator`, as the
watcher does today. The handler's event is for a different path from
any event a concurrent rescan emits, so leaving it outside the rescan
permit does not reorder events for one path.

Tests:
- a work-item PATCH adds one activity row and one SSE event, carrying
  the `FileContent` etag;
- an external member edit, followed by a work-item PATCH inside the
  member's debounce window, emits each change once and the PATCH's with
  the `FileContent` etag.

**Set-level debounce.** A path admitted under a set doc root is
debounced on its set directory rather than its own path, so a burst
inside one set (a copy, or a rename into place) collapses to one
rescan. The set-keyed task keeps the file paths it admitted and checks
`write_coordinator.should_suppress` against each, so a burst made only
of the application's own writes skips the rescan.

**Case-only renames.** `should_rescan` already covers these: both the old
and new paths sit under a set root and contain no dot-prefixed
component, so either event triggers a rescan.
- One domain function, `corpus::set::pair_case_only_renames(removed,
  added)`, pairs each removed member path with an added member path in
  the same set directory that is equal to it under ASCII case folding.
- `rescan_with_diff` applies it, so the diff it returns already holds
  each pair as one renamed member. `rescan_changes` emits a pair as one
  `edited` event for the new path, and the removal ledger, which reads
  the same diff, does not record it.
- The pairing applies to member documents and assets alike. It runs only
  when the old path is gone from the after-snapshot, so on a
  case-sensitive volume two coexisting files `a.png` and `a.PNG` stay two
  members.

#### 3. Activity roll-up

**File**: `server/src/activity_feed.rs`, and `watcher::emit` (`:328-333`)

- `emit` passes each event's `Option<SetEventContext>`, taken from the
  before or after record, where
  `SetEventContext { root_rel_path, role: SetDocumentRole }` reuses the
  role from the membership model, and an asset event carries
  `SetDocumentRole::Member`.
- `ActivityRingBuffer::record(event, set: Option<SetEventContext>)`:
  - **non-set events** push as today;
  - **set events** take out any row whose path is the root path. The new
    row's action is the event's own action for a root event, and
    otherwise the removed row's action, or `Edited` when there was none.
    Then push `ActivityEvent { action, path: root, doc_type, timestamp }`.
- The root's action lives on its row, so the buffer holds no state
  beyond its 50 rows. A set whose row has been evicted reads `Edited` on
  its next member change.
- The work stays O(n) over 50 rows under the lock, within the module's
  mutex discipline: no allocation beyond the row, and no I/O.

#### 4. Live activity feed

**File**: `frontend/src/components/ActivityFeed/ActivityFeed.tsx`

The server's ring buffer is the single owner of the roll-up rule. Today
`ActivityFeed` adds one live row per `doc-changed` event
(`ActivityFeed.tsx:69-99`), which would show a row for every member and
asset. Instead:

- its `subscribe` listener invalidates `queryKeys.activity(LIMIT)` on
  each `doc-changed` event, so the feed refetches `/api/activity`.
  TanStack Query collapses a burst of invalidations into one in-flight
  refetch;
- the `live` state and the merge with `initial` are removed, and rows
  are the query's data as the server returns them;
- `subscribe` still fires before the self-cause check, so the
  application's own writes still refresh the feed.

**File**: `server/src/api/docs.rs`

Today the document PATCH handler broadcasts through `sse_hub` directly
(`api/docs.rs:316-324`), so its writes never reach the ring buffer and
appear in the feed only as client-built rows. With those rows gone, the
handler's event goes through `emit`, as described under "In-app
writes". `emit` moves out of `watcher.rs` into a module that the sink
and the handler both call.

**File**: `frontend/src/components/ActivityFeed/ActivityFeed.test.tsx`

Write these red first:

- a `doc-changed` event causes one refetch of `/api/activity`, and the
  feed shows the refetched rows;
- three `doc-changed` events for members of one set, with the mocked
  server returning one rolled-up row, leave exactly one row for the set;
- a self-caused `doc-changed` event still refetches, and the refetched
  feed shows the row the server recorded for that write.

Replace the existing tests that assert client-built live rows.

#### 5. Removal ledger

**File**: `server/src/indexer.rs`

- `rescan_with_diff` reads the paired diff's removed member documents
  and assets for each set directory. For each removal it records
  `last_removal_observed_ms[set_directory] = now`.
- `latest_change_ms` is the maximum of the root, the member documents,
  the assets and that value.
- When a set disappears, its ledger entry is cleared.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test:unit:visualiser`, `mise run test:integration:visualiser`
- [ ] `mise run public-api:update`, with the `corpus` snapshot diff
      reviewed.
- [ ] Real-OS rename check: extend
      `the_os_watcher_reports_a_write_under_a_registered_directory` with
      a directory-rename case, and pass
      `mise run test:integration:visualiser-os-watcher` on macOS and in
      CI (Linux).
- [ ] Real-OS case-only rename: the lane runs exactly one test by name
      (`tasks/test/integration.py:71-74`), so the case is folded into
      the same test function as a third step, not added as a second
      test. It renames `screenshots/a.png` to `screenshots/a.PNG`
      through the OS watcher and expects one `edited` event and no
      `deleted` event.
  - The step first probes the tempdir: it creates `probe` and checks
    whether `PROBE` resolves to it.
  - On a case-sensitive volume (Linux CI) it skips the step with a log
    line, because the `Scripted` test above already covers the pairing
    there.
  - On macOS's default APFS the lane runs the test with
    `--include-ignored`, and the OS reports the rename with the stored
    name.
- [ ] `mise run test:unit:frontend`, `mise run frontend:check`
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
  - the catalogue's shape refusals;
  - the resolver's slug candidates and `set_directory_of`;
  - the boundary test for named built-in types.
- **Declarations.** Each declared shape's classification, in
  `research-declarations` and `design-declarations`; the research
  layout's path builders classifying as members (`research`'s
  integration test); and the assembled catalogue building.
- **Build system.** The crate-rule lint's declarations and assembly
  roles, a domain's edge to its own declarations crate, the launcher's
  widened rule, and a pup probe pair per new crate.
- **Config.** Overlap refusal for equal, nested and prefix-but-not-segment
  roots, and the defaults; single-key lookups of a doc root refusing,
  other path keys unaffected.
- **Migrate.** The runner refusing before applying or listing, and a
  migration that writes an overlap stopping the run unrecorded.
- **Server:**
  - root-only default accessors and explicit member accessors, on
    fabricated members;
  - enumeration on tempdir layouts, including real-inventory clutter;
  - asset records;
  - the activity roll-up's state machine;
  - the removal ledger;
  - the watcher's admission rule.

### Integration Tests:

- `api_types`, `api_docs`, `api_search`, `api_related`, `api_sets` and
  `api_library` assert every wire criterion.
- `generic_set_model.rs` holds the test-only doc type end to end.
- `resolve_goldens.rs` and the research CLI's topic tests cover the
  resolver's conformance.
- `sse_e2e.rs` covers renames and root deletion through `InjectedWatcher`.
- The real-OS watcher lane's single test covers a directory rename and a
  case-only rename.
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
- **Watcher admission.** The watcher now also admits slotted asset
  paths and slot-ancestor directories in set roots, and nothing in
  unslotted trees such as `prototype-full/`. Admitted paths under a set
  root are debounced per set directory, so copying an inventory or
  renaming a topic set into place costs one full rescan, not one per
  file. An empty diff skips the cluster and linked-count recompute.
- **Client fan-out.** Each `doc-changed` event invalidates several
  queries. TanStack Query keeps one refetch in flight per query, so a
  burst of member events costs a handful of refetches, not one per
  event per query.
- **Set facts are precomputed.** A set's slug, `setId` and latest time
  are computed once per rescan, so search projection and library
  aggregates read them without scanning the index.
- **Activity roll-up.** It scans at most 50 rows per event.

## Migration Notes

- **Configuration.** The defaults and `m0004`'s layout nest nothing.
  Users whose configuration nests doc roots today (for example
  `review_plans: meta/reviews` beside the default
  `review_prs: meta/reviews/prs`, or a flat `research_codebase:
  meta/research`) will see every command that reads doc roots refuse,
  naming both keys:
  - move the contained directory's files to a sibling path, or point
    the containing key at a subdirectory (for example
    `research_codebase: meta/research/codebase`), and update the config;
  - `accelerator config set` never resolves doc roots, so it works
    while the overlap stands;
  - then run `accelerator migrate` for any pending migrations; it
    refuses until the overlap is gone.

  Both CHANGELOG Breaking entries carry these steps.
- **`corpus resolve`.** An input naming a rootless or dot-prefixed set
  directory, an unslotted file or a stray file in a set type's directory
  now exits 3. A set missing its root file needs the file restored to
  resolve again. The CHANGELOG Breaking entry says so.
- **No data migration.**
- **Bookmarked URLs** are unaffected. Member routes are PP-998's.

## References

- Work item: `meta/work/PP-994-generic-set-model-indexing-and-api.md`
- Research: `meta/research/codebase/2026-11-24-PP-994-generic-set-model-indexing-and-api.md`
- Overlapping plan: `meta/plans/2026-11-24-PP-996-library-links-by-document-id.md`
- Precedent ADRs: `meta/decisions/ADR-0067-kind-discriminated-corpus-schema-and-templates.md`,
  `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
- Decisions (accepted):
  - `meta/decisions/ADR-0077-generic-set-model-for-directory-doc-types.md`
  - `meta/decisions/ADR-0078-doc-types-as-declared-data.md`
  - `meta/decisions/ADR-0079-doc-roots-may-not-overlap.md`
  - `meta/decisions/ADR-0080-domain-declarations-crates.md`
- Member API: `meta/specs/2026-11-24-set-member-api.md`
- Follow-ups out of scope here: PP-1006 (domain-owned doc type
  declarations), PP-1007 (whole declarations to their owners), PP-1008
  (homes for unowned doc types), PP-1009 (configuration's doc type table
  derived from the declarations)
- Crate registration: `tasks/README.md`, "Registering a library crate"
- Set layouts: `skills/research/research-topic/SKILL.md`,
  `skills/design/inventory-design/SKILL.md`, `cli/research/src/topic/layout.rs`
- Patterns:
  - the handler shape in `cli/visualiser/server/src/api/related.rs:30-57`;
  - the watcher harness in `cli/visualiser/server/src/watcher.rs:779-889`;
  - the end-to-end SSE harness in `cli/visualiser/server/tests/sse_e2e.rs`.
