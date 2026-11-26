---
type: "work-item"
id: "PP-994"
external_id: "PP-994"
aliases: ["draft-0wfg5p"]
title: "Generic Set Model: Indexing and API"
date: "2026-11-23T16:12:42+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "ready"
kind: "story"
priority: "high"
parent: "work-item:PP-868"
tags: ["research", "visualiser"]
last_updated: "2026-11-26T00:20:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
blocks: ["work-item:PP-134", "work-item:PP-998", "work-item:PP-1001", "work-item:PP-1000", "work-item:PP-1002"]
relates_to: ["work-item:PP-862", "work-item:PP-995", "work-item:PP-996", "work-item:PP-1003"]
---
# PP-994: Generic Set Model: Indexing and API

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As a maintainer of the visualiser, I want a doc type whose entries are
directories to be indexed as a set — a root document plus the member
documents and assets its declared shape names — so that any multi-document
doc type gets member indexing, search, live update, and a member API from
one model.

Topic research and design inventories move onto the model here. Someone
browsing the visualiser sees three changes: member documents appear in
search, each result opening its set until member routes land; a set's
activity row follows its members' changes; and so does its latest-change
time. The story unblocks member routes and the member view (PP-998),
screenshot rendering for design inventories (PP-134), and, with PP-998, the
set page (PP-1001) and Sources rendering (PP-1000).

## Context

Child of PP-868 — Topic-Research Set Detail Page. It is one of PP-868's
children and has no blockers. Member routes and the member view were split
from this story at the member API and are PP-998.

Two doc types are directories today: topic research (root `manifest.md`) and
design inventories (root `inventory.md`). The visualiser indexes one entry
per directory, keyed on the root, and sees nothing else in it. More
multi-document doc types will follow, work items with attachments among them,
so the model is built as a pattern and not per type.

The index feeds search, activity, related, the library structure, lifecycle
clusters, and the type listing. None distinguishes a root from a member, so
each needs a rule once members are indexed.

Real set directories hold files their skills never wrote: some design
inventories carry hand-placed `prototype-full/` and `assets/` trees beside
`inventory.md` and `screenshots/`. A set's declared shape keeps those out.

## Requirements

This story is server-only. Its outcomes are stated on API responses and
change events, and the frontend changes only the types that describe those
responses. Every change to what the frontend does with a member belongs to
PP-998.

### Model

- Model a set as a root document plus members. A member is a file that fits
  one of the **member slots** of its set's shape: a document (a markdown
  file, whether its frontmatter is parsed, malformed, or absent) or an
  asset, as the slot says. A file that fits no slot is not part of the set.
- Let a doc type make two declarations. Its **set declaration** says it is
  a set and gives its **shape**: the root file's name and its member slots.
  Set handling branches on this and nothing else, never on a named doc
  type. Its **presentation declaration** says how the set presents: a set
  page that lists its members (topic research), or an entry-point document
  that presents them within its own page (design inventories). The
  presentation declaration is recorded here and changes no behaviour until
  the set page and PP-134 read it.
- Write a member slot as a path relative to the set directory whose
  segments are each a literal name or `*`, with an optional file extension.
  `*` matches one visible name, never a dot-prefixed one, and there is no
  recursive wildcard.
- Declare these shapes:

  | Set type | Root | Document slots | Asset slots |
  |---|---|---|---|
  | Topic research | `manifest.md` | `brief.md`, `outline.md`, `synthesis.md`, `findings/*.md`, `findings/*.levels/*.md`, `reports/*.md` | none |
  | Design inventories | `inventory.md` | none | `screenshots/*.png` |

  The topic-research slots follow the research crate's reader rules
  (`is_finding_entry`, `is_report_entry`), not its stricter writer rules:
  any visible markdown file in `findings/` is a member.
- Record the model in an ADR, including set shapes, assets as members, and
  the two presentations, and have it accepted.

### Configuration

- Refuse a configuration in which one doc root equals or contains another,
  naming both path keys. The default paths nest nothing, and migration
  `m0004` moved earlier layouts off `meta/research/`.
- Run the end-to-end server and the API smoke test on the default paths:
  move the shared fixtures' flat research documents into
  `meta/research/codebase/` and drop their `research_codebase:
  meta/research` overrides.

### Indexing

- Index every member of a set, each with its path relative to the set
  directory; each document with its parsed frontmatter and each asset with
  its size and modification time.
- Index a set's members even when its root's frontmatter is malformed. A
  set directory with no root file is not a set and indexes nothing.
- Index a member document whose frontmatter is missing or malformed, marked
  as such, so a set page can list it as unplaced.
- Exclude dot-prefixed files and directories, within a set directory and at
  the doc root of a set doc type; other doc types are unaffected. The
  research skill writes
  every quarantined file (`.<name>.invalid`), the run ledger
  (`.conduct-run.json`), and temporary set directories (`.<slug>.tmp/`)
  with a leading dot, so this one rule covers them.
- Ignore a file in a set directory that fits no member slot: it is not
  indexed, listed, searched, or served, and emits no change event.

### Index consumers

- Keep the type listing, the library structure's counts and facets, and the
  lifecycle clusters at one item per set: a member has an index record but is never
  counted, listed, or clustered as an item of its own.
- Return a member document as a search result in its own right, titled as
  the member and carrying its set-relative path. Asset members are not
  search results. A member matches a query as any document does, on its
  title, its key, and its first paragraph, where its key is its own
  filename's stem; search reads no further into a body, for any doc type.
  The set's `id` rides on a member's row only as the key the frontend links
  the result by, so a member opens its set until PP-998 switches the link
  to the member's own URL; it is never matched against a query.
- Roll a set's member changes up to the set in the activity feed: one row
  per set, identified by the set's root. The row's action is the root's
  latest own action: created, edited, or deleted. A member's change
  updates the row's time and never its action, so a row reading created or
  deleted stays so, and a member's change to a set whose root was last
  edited leaves the row reading edited.
- Take a set's latest-change time in the library structure from the most
  recent observed change to its root or any member, removals included. For
  an addition or an edit that is the file's modification time; for a
  removal, the time the watcher observed it.
- Attribute a member document's declared linkage to that member, not to its
  root or its set: related items are returned for the document whose
  frontmatter carries the linkage, in the fields the related response reads
  today (`work_item_id`, a `work-item:` `target`, `parent`, and `related`,
  each naming a work item). The linked document returns the member as an
  inbound item. Members take no part in the related response's inferred
  items, which are derived from lifecycle clusters. Reading `relates_to` is
  not this story's: it is PP-1003, for every doc type.
- Serve a member document's body by its path through the document endpoint,
  `/api/docs/{path}`, as for any document. The response stays the file's
  raw markdown with its etag, and carries no set key.
- Expose each doc type's set declaration and presentation declaration to
  the frontend.
- Carry a set's `id`, its root's frontmatter `id`, on every row that names
  a root or a member, so no client derives a link from a root's filename.
  Those rows are the type listing, search results, related items, and the
  member API. An activity row and a change event each carry a path and no
  `id`: the activity feed finds a set's entry from the type listing, as
  PP-996 has it.
- Serve a set's root and members through the **member API**: each document
  with its parsed frontmatter, each asset with its path, size, and
  modification time. The name stands for this capability whichever endpoint
  carries it (see Open Questions for the shape), and its response always
  includes the root. It returns every member in one response, without
  paging, and writes each path as a plain `/`-separated string relative to
  the set directory, never percent-encoded.
- Serve no asset bytes; that belongs to PP-134. A request for an asset's
  path through the document endpoint is not-found, as is a request for a
  file in a set directory that fits no member slot.

### Live update

- Emit a change event carrying the member's own path when a member document
  or asset enters, changes in, or leaves the index. A member enters or
  leaves the index when its file is added or removed, and also when its
  directory starts or stops being a set.
- When a set arrives whole, as when the research or inventory skill renames
  its dot-prefixed temporary directory into place, emit one created event
  for the root and one for each member.
- When a set's root is deleted and members remain on disk, the directory
  stops being a set: emit a deleted event for the root and for each
  member, and return not-found for those members from the member API and
  the document endpoint.
- Emit nothing for an excluded or ignored file.

### Design inventories

- Move design inventories onto the model: `inventory.md` is the root and
  entry-point document, and the PNG images directly under `screenshots/`
  are its asset members. Nothing else in an inventory's directory is a
  member.
- Leave their library listing and their page showing what they show today.

## Acceptance Criteria

- [ ] Given a topic-research set with a manifest and ten member documents,
      then the member API returns the root and ten members, and the
      topic-research type listing response holds one entry.
- [ ] Given a topic-research set with members at `brief.md`,
      `findings/01-a.md`, `findings/01-a.levels/2-1.md`, and
      `reports/r.md`, then all four are returned by the member API with
      those set-relative paths, and a change to
      `findings/01-a.levels/2-1.md` emits exactly one change event,
      carrying that member's path.
- [ ] Given a topic-research set and a design inventory, each laid out as
      the Model's shapes table gives, then each is indexed with exactly the
      members its row lists, and no other file.
- [ ] Given a set whose root has unclosed frontmatter, then the root is
      marked malformed and the member API still returns its members.
- [ ] Given a set directory with no root file, then nothing in it is
      indexed.
- [ ] Given a member document, when its path is requested from
      `/api/docs/{path}`, then its body is returned.
- [ ] Given the doc types response, then topic research and design
      inventories are each marked as sets with their presentation, and no
      other type is.
- [ ] Given a design inventory holding `screenshots/a.png`,
      `prototype-full/x.html`, `assets/y.svg`, and `notes.txt`, then only
      the screenshot is an asset member; the other three are absent from
      the member API and from search, emit no change event when written,
      and are not-found through `/api/docs/{path}`. A search for
      `a.png` returns no result.
- [ ] Given a library holding only one topic-research set, whose member
      documents carry status and slug frontmatter values the root does not,
      then the library structure's count for topic research is one and its
      status and slug facets hold only the root's values.
- [ ] Given a library holding only one topic-research set, then no lifecycle
      cluster holds a member, and the root is in the same cluster, with the
      same cluster members, as when the set's members are absent from disk.
- [ ] Given a set whose root last changed yesterday, when a member document
      changes now, then the library structure's latest time for that set,
      and for its doc type, is that member's modification time; the same
      holds when a design inventory's screenshot changes.
- [ ] Given a set holding `findings/.01-x-web.md.invalid`,
      `.conduct-run.json`, and a `.scratch/` directory of documents, and a
      `.draft.tmp/` directory beside the set in the doc root, then none is
      indexed; and given a write to each of them followed by a write to a
      member document, then when the member's change event arrives, no
      change event or activity row for any of them has been emitted.
- [ ] Given a member document with unclosed frontmatter, then it is indexed
      with its frontmatter marked malformed; given one with no frontmatter,
      then it is indexed with its frontmatter marked absent; in both cases
      the rest of the set is indexed as normal.
- [ ] Given an asset member, then its index record carries its path relative
      to the set directory, byte size, and modification time.
- [ ] Given a phrase that appears only in the first paragraph of one member
      document, when it is searched for, then the result is that member,
      titled as the member, carrying the member's path relative to its set
      and its set's `id` as the key it links by.
- [ ] Given a topic-research set with ten member documents, when the set's
      slug is searched for, then the set's root is the first result, and no
      member is ranked as an exact key match.
- [ ] Given member changes to set A, then a change to set B, then further
      changes to set A's root and a member document, then the
      activity feed shows exactly one row for A, above B's, carrying the
      time of A's latest change, and no row naming a member.
- [ ] Given a member document whose `parent` names work item X, when related
      items are requested for that member's path, then X is returned as an
      outbound item; requested for the set's root or another member, it is
      not; requested for X, the member is returned as an inbound item.
- [ ] Given a member document is added, changed, or removed on disk, then
      after one write followed by quiet longer than the watcher's debounce
      interval exactly one change event is emitted for that
      member's path, with the action created, edited, or deleted, and none
      for the root or any other member; the same holds for an asset
      member.
- [ ] Given a doc type declared as a set only in test code, through the
      mechanism topic research uses, with root `index.md`, a document slot
      `docs/*.md`, and an asset slot `img/*.png`, holding two member
      documents, one asset, and `other/z.md`, which fits no slot: the
      member API returns the root and all three members; a phrase unique to
      one member document's first paragraph returns that member from
      search; the type listing response holds one entry; a change to a
      member emits one change event; the activity feed holds one row for
      the set; the library structure counts one item, with the root's
      facets alone and a latest time that follows a member's change; no
      lifecycle cluster holds a member; a dot-prefixed file in the set and
      `other/z.md` are absent from all of these; the rows carry the set's
      `id`; and the doc types response marks the type as a set.
- [ ] A search of the `cli/` workspace's production sources for the topic
      research and design inventory doc types finds them named only in
      their declarations, in the corpus and the declarations crates; in
      the declarations assembly, which pairs each set type with its
      owner's set declaration; in code that owns one doc type's domain,
      such as the research topic set commands; and in the frozen
      migrations. Test modules are not searched. The visualiser server's
      sources name neither outside test modules, since a doc type's
      library phase is a declared rule.
- [ ] Topic research declares the set-page presentation and design
      inventories the entry-point presentation, each readable from its doc
      type's declaration.
- [ ] Given a set of the test-only doc type above with a root, ten member
      documents (one with unclosed frontmatter, one with none), and six
      assets, when its members are requested through the API, then one
      response holds the root and all sixteen members: each document with
      its parsed frontmatter or its malformed or absent mark, each asset
      with its byte size and modification time, and every member with its
      path relative to the set directory. No document body is in the
      response.
- [ ] Given a design inventory with a `screenshots/` directory of six images,
      then the six are indexed as asset members, and requesting one image's
      path through the document endpoint returns not-found.
- [ ] Given a design inventory screenshot named `Some File (v2).png`, then
      the member API returns its path as `screenshots/Some File (v2).png`,
      with its byte size and modification time.
- [ ] Given the same design inventory, then the design-inventory type
      listing response holds one entry for it, carrying the title, status,
      and date of `inventory.md`'s frontmatter, and the document response
      for `inventory.md` carries that file's body unchanged.
- [ ] Given a set whose root's `id` is `x`, then the type listing entry for
      it, a search result for a member, a related item naming a member, and
      the member API response each carry `x` as the set's `id`; the same
      holds for a design inventory.
- [ ] Given a set whose root was last edited, when one member document is
      added and another removed, then the set's activity row reads edited
      and carries the time of the later change; given a newly created set
      whose row reads created, when a member changes after a further quiet
      period, then the row still reads created, with the new time.
- [ ] Given a directory `.x.tmp/` holding a topic-research root, `brief.md`,
      `outline.md`, and `notes.txt`, when it is renamed to `x/`, then three
      change events are emitted, each reading created and carrying its own
      path, and none for `notes.txt`; the activity feed shows one row for
      the set, reading created; and the member API returns the root and two
      members.
- [ ] Given a set whose root is deleted while two member documents remain
      on disk, then three change events are emitted, each reading deleted;
      the set's activity row reads deleted; the type listing holds no entry
      for the set; and the member API and `/api/docs/{path}` return
      not-found for both members.
- [ ] Given a set whose members were last modified yesterday, when a member
      is removed now, then the library structure's latest time for that set
      is the time the removal was observed.
- [ ] Given a dot-prefixed markdown file in the doc root of a doc type that
      is not a set, then it is indexed, listed, and emits change events as
      it does without this story.
- [ ] Given a configuration in which one doc root equals another, or
      contains another, then it is refused with an error naming both path
      keys; given the default paths, then nothing is refused.
- [ ] The end-to-end server and the API smoke test run with no path
      override, and the shared fixtures' research documents sit under
      `meta/research/codebase/`.
- [ ] Given a member document and a plan outside its set that carry the
      same slug in their filenames, when related items are requested for
      either, then the other is not among the inferred items, and no
      inferred item of any response is a member.
- [ ] Given a set directory without its root file, when `corpus resolve`
      is given its slug or its path, then it does not resolve.
- [ ] Given a dot-prefixed directory under a set type's doc root, when
      `corpus resolve` is given its exact name or its path, then it does
      not resolve.
- [ ] Given a path to a file in a set directory that fits no member slot,
      when it is given to `corpus resolve`, then it does not resolve to the
      set; given a path to the set directory, its root, a member, or a
      slot directory, then it resolves to the set.
- [ ] Given a stray file directly in a set type's doc root, when its path
      is given to `corpus resolve`, then it does not resolve as a set.
- [ ] Each set type's shape is declared in its owning domain's
      declarations crate, topic research in `research-declarations` and
      design inventories in `design-declarations`; every composition root
      that builds a doc type catalogue gets it from the declarations
      assembly; and given a catalogue that lacks a set type's shape, then
      building it fails.
- [ ] Accepted ADRs record the set model (ADR-0077), doc types as declared
      data with the catalogue assembled from owners' set shapes
      (ADR-0078), the refusal of overlapping doc roots (ADR-0079), and
      domain declarations crates with their assembly (ADR-0080), and the
      author has accepted each through `/review-adr`, so each status is
      `accepted`.
- [ ] An accepted ADR, or a document it names, records the member API's
      endpoint and response shape, covering asset enumeration without
      paging (the largest real inventory held 35 screenshots), plain
      set-relative paths, where a document's structured sources attach,
      and where the derived set response sits beside the member listing.
- [ ] This work item's Drafting Notes carry one dated decisions line
      recording two things: the ADRs' numbers and their accepted status;
      and the document that holds the member API shape.

## Open Questions

- Set-member API shape (planning-time): does a set page get its members
  from a new set endpoint, or from an augmented `/api/docs/{path}` detail
  response that includes siblings? The set page (PP-1001), Sources
  rendering (PP-1000), and PP-134 consume the answer. It must leave room
  for two things other stories add: a document's structured sources
  (PP-1000), and the derived set response (PP-1002) served beside the
  member listing (the outline's rounds and items, focus areas, pair
  states, the set's status, and the stale and reopened marks). Settled on
  2026-11-24: asset enumeration lists every asset in one response without
  paging, and paths are plain set-relative strings, which settles PP-134's
  own question on listing screenshots.
- Awaiting the author's confirmation, and stated as settled in
  Requirements until then: an activity row and a change event carry no
  set `id`.
- Settled on 2026-11-24: a doc root may not equal or sit under another
  configured doc root, and configuration refuses one that does.

## Dependencies

- Blocked by: nothing.
- Blocks, each directly:
  - PP-998, member routes and the member view, which read the member index,
    the member API, and change events.
  - PP-1001, the set page, which consumes the member API and the
    presentation declaration.
  - PP-1000, Sources rendering, whose sources ride on the member API shape.
  - PP-134, which serves and renders the asset members this story indexes.
  - PP-1002, the derived set response, which reads the member index
    and sits beside the member API.

  PP-1001 and PP-1000 are also blocked by PP-998, so they wait on this
  story twice over.
- Planning order: this is a two-stage story, by the author's choice. The
  first stage is two decisions, in this order: the member API shape, then
  the accepted ADRs; the nested-doc-root check that preceded them was
  settled on 2026-11-24. The second is the implementation. The first stage
  is complete when the dated **decisions line** in Drafting Notes exists,
  recording the ADRs' numbers and where the API shape is written down.
  PP-998, PP-1001, PP-1000, PP-134, and PP-1002 are planned only once
  that line exists; it is their release, and they need not wait for this
  story to land. PP-134's planning also waits on PP-998's route shape. The
  Sources parser's entry shape (PP-995) informs the API shape without
  blocking it; if that entry shape is not settled in time, the API shape
  carries sources as an extension point it does not define.
- Independent of PP-996, the bug whose fix builds every library link from
  the document's `id`: either may land first. PP-996's activity feed looks
  a set's entry up in the type listing by the row's path, in both orders,
  so this story keeps a set's activity row identified by its root's path.
- Follow-up: PP-1003 reads `relates_to` for every doc type, members
  included; this story does not wait on it.
- Contract with the research crate: the topic-research shape is declared
  in `research-declarations` from the layout constants the research
  domain also uses, and a test in the research crate checks that every
  path its layout builders can produce fits a slot.
- On-disk conventions: the dot-prefix exclusion relies on what the
  research-topic skill and the research CLI write (the CLI writes the run
  ledger), and the shapes on the layouts the research-topic and
  inventory-design skills write: `inventory.md` with `screenshots/`, built
  in a dot-prefixed temporary directory and renamed. A change to either
  layout must be matched here.
- Shared crates: the `corpus` crate defines what a doc type declaration
  and a set declaration are (ADR-0078). Each set's owner declares its
  shape and presentation in its declarations crate, `research-declarations`
  or `design-declarations`, and the `declarations` assembly builds the
  complete catalogue every composition root takes (ADR-0080); ADR-0069,
  as ADR-0080 extends it, governs those edges.
- Existing consumers of change events: the frontend's query invalidation
  starts receiving member events when this story lands. No route shows a
  member yet, so the external-edit toast cannot fire for one.
- Existing consumers of the search and activity responses: the frontend's
  search results panel and activity feed. Both keep working unchanged: a
  member's search row is an ordinary row whose link key is its set's
  `id`, and a set's activity row is identified by its root, as today.

## Assumptions

- No doc root equals or sits under another; configuration enforces it, so
  a member's doc type comes from the one root that contains it.
- Assets are indexed and listed but have no URL and are not served, so the
  asset half of the model has no end-to-end consumer until PP-134 and may
  need changes then.
- One activity row per set means the latest change replaces the row's time
  and position; the row's action follows the root alone. A count of
  changes on the row is not required.
- Until PP-998 lands, a member is findable in search but its result opens
  the set, and the set's own URL renders its root as it does today.

## Technical Notes

**Size**: L — the indexer, file driver, and watcher change shape; six
index consumers each gain a root-versus-member rule (the type listing, the
library structure's counts, facets and latest time, lifecycle clusters,
search, activity, and related); a member API is added, configuration
refuses overlapping doc roots, and two pinned tests and their fixtures are
rewritten.

- The index is a map of canonical path to `IndexEntry` with no notion of
  root and member (`cli/visualiser/server/src/indexer.rs:247`).
  `LocalFileDriver::list` emits one `<dir>/<root>` per set, one level deep
  (`file_driver.rs:275-327`).
- The only set declaration today is `DocTypeKey::nested_manifest_filename()`
  (`cli/corpus/src/doc_type.rs:165-171`); the shape extends it. The
  research crate's topic layout is `cli/research/src/topic/layout.rs` and
  its `entry` and `finding_path` modules.
- The watcher watches each doc root recursively but drops non-markdown files
  (`watcher.rs:119`) and emits only for entries whose etag or frontmatter
  state changed (`watcher.rs:280-317`), so a member's change produces a full
  rescan and no event today.
- Consumers to give a root-versus-member rule: `library_aggregates`
  (`indexer.rs:725-787`), `counts_by_type` (`indexer.rs:707-713`),
  `all_by_type` behind `GET /api/docs?type=` (`indexer.rs:697-705`), cluster
  computation at its three call sites (`watcher.rs:228-255`,
  `api/docs.rs:276-311`, startup in `server.rs`), and the inferred-cluster
  path in `related.rs:32-51`.
- A search row is `{docType, title, slug, mtimeMs}` with no path
  (`api/search.rs:29-35`), so it cannot address a member as it stands.
  `classify` (`api/search.rs:57-81`) matches title, slug, and the
  200-character first-paragraph preview; a member's index entry keeps its
  own stem to match on apart from the set's `id` it links by.
- The related response's declared links come from `read_ref_keys`
  (`frontmatter.rs:178-228`), which reads `work_item_id`, a `work-item:`
  `target`, `parent`, and `related`, and never `relates_to`.
- `GET /api/docs/{path}` serves any file under any root through an
  unindexed fallback (`api/docs.rs:75-89`); the not-found criteria close
  that for set directories.
- Activity is a 50-slot ring buffer filled in `watcher::emit`
  (`watcher.rs:328-333`, `activity_feed.rs:43-60`); the roll-up belongs
  there, while the event itself keeps the member's path.
- Two tests pin one entry per set and must change:
  `topic_research_indexes_one_entry_per_set_from_its_manifest`
  (`indexer.rs:2945`) and `design_inventories_indexed_from_nested_directories`
  (`indexer.rs:2885`). Only one fixture set has members, and none has assets.
- Overlapping roots: `config::paths::doc_type_dirs`
  (`cli/config/src/paths.rs:32-69`) checks only for unsafe paths today. The
  overrides to drop are in `frontend/e2e/start-server.mjs:82-85` and
  `server/tests/api_smoke.rs:24-27`, and the documents to move are
  `server/tests/fixtures/meta/research/*.md`. With no overlap,
  `kind_for_canonical_path` (`file_driver.rs:506-511`) has at most one
  matching root, so its first-match order no longer matters.
- `PHASES` (`api/library.rs:78-114`) assigns every doc type to a library
  phase and stays as it is.
- The next ADR number was 0077 at the time of writing. ADR-0067 (kind-discriminated corpus schema)
  is the nearest precedent.

## Drafting Notes

- Decomposed from PP-868 on 2026-11-23 after its stress test agreed the
  split.
- The criteria on library counts, lifecycle clusters, malformed members, and
  the corrected set links were added during decomposition from codebase
  analysis; PP-868 stated only the type listing.
- Asset indexing and the design-inventory migration could ship separately
  and have no end-to-end consumer here. They stay, at the author's
  direction, to prove the model on two doc types.
- Split at the member API on 2026-11-23, at the author's direction: member
  routes and the member view moved to PP-998. The corrected set links moved
  on from there to their own bug, PP-996.
- A fourth review on 2026-11-23 added criteria for the set slug, the
  activity row's action, a non-set doc type's dot-prefixed files, `assets/`,
  inferred relations, and the recorded API shape; stated, at the author's
  direction, that the story is server-only, with its outcomes asserted on
  responses; named the responses that
  carry the slug; stated the asset rule by file type; and rewrote the
  Blocks entries. It dropped `assets/` from the design-inventory layout:
  the inventory-design skill writes no such directory. Leaving the slug
  off change events, and classing any markdown file in an inventory as a
  document member, are the drafting agent's calls, not yet confirmed by
  the author.
- A re-review on 2026-11-23 found the activity row's action dependent on
  event order. The author settled four rules: the action is the root's
  own; a set arriving by rename emits one created event per file; deleting
  a root removes its members from the index, with a deleted event each;
  and the latest-change time counts removals. The activity row no longer
  carries the set's slug, since PP-996 looks it up from the type
  listing.
- The decision deliverables stay in this story, at the author's direction
  on 2026-11-23, against a review suggestion to lift them into a spike. A
  dated decisions line in these notes marks the first stage complete and
  releases the planning of the five blocked items.
- No work item is tracked yet for work-item attachments, the future consumer
  Context names.
- A member's change bumps its set's latest time, decided on 2026-11-23.
- A third review on 2026-11-23 adopted what the set page and member routes
  need from this story (members under a malformed root, set-relative paths
  for documents, a member body by path, declarations and the set slug
  exposed to the frontend), and added depth and genericity fixtures. The
  activity row reading as an edit, and members taking no part in inferred
  relations, were the drafting agent's calls,
  confirmed by the author on 2026-11-23.
- A second review on 2026-11-23 added the API criterion, a behavioural
  genericity criterion using a test-only doc type, and the split between
  the set declaration and the presentation declaration.
- A review on 2026-11-23 separated "index record" from "listed item", gave
  the count criteria a baseline, removed the time window from the activity
  criterion, and added criteria for asset records, missing frontmatter, and
  excluded files emitting nothing.
- Decisions, 2026-11-24, by the author after codebase research
  (`meta/research/codebase/2026-11-24-PP-994-generic-set-model-indexing-and-api.md`):
  configuration refuses equal or nested doc roots, and the legacy test
  overrides go; members match search on their own title, stem, and first
  paragraph, with no full-body search, and the set's `id` on a member's row
  is never matched; related items keep today's fields, and `relates_to`
  moves to PP-1003; the document response
  stays raw markdown with no set key; the source-search criterion admits
  the library's phase table; and a set's shape, its root and member slots,
  is part of its declaration, so files outside it are ignored, the member
  API needs no paging, and its paths are plain set-relative strings. Links
  carry the set's `id` in place of its slug, following PP-996's decision
  to link every document by its `id`. These settle the nested-doc-root
  check and supersede the earlier rule that any file in an inventory is a
  member.
- Decisions, 2026-11-25, by the author: the corpus CLI's resolver conforms
  to the set model in this story; each set type's shape is declared by
  the domain that writes it and registered by every composition root that
  reads set members; the dot prefix is the owner's boundary, so its
  working files are never registered and every other reader ignores them;
  and a path still resolves to its set when it is the set directory, its
  root, a member, or a slot directory.
- Decisions, 2026-11-25, by the author (ADR-0080), superseding the
  registration above: each context that declares to the corpus gets a
  declarations crate, and one declarations assembly builds the complete
  catalogue that every composition root takes; the declarations crates
  hold layouts and set shapes only, and doc type facts stay in the
  corpus; and shapes are part of the catalogue, with no separate
  registry.
- Decisions recorded, 2026-11-26: ADR-0077 (the set model), ADR-0078
  (doc types as declared data), ADR-0079 (doc roots may not overlap)
  and ADR-0080 (domain declarations crates) are accepted; the member
  API shape is in `meta/specs/2026-11-24-set-member-api.md`.

## References

- Parent: PP-868
- Related: PP-134 (consumes asset members), PP-862 (pinned one entry per
  set, which this story changes), PP-998 (member routes and view),
  PP-1001 (the set page), PP-1000 (Sources rendering), PP-1002 (the
  derived set response), PP-995 (the Sources
  parser, whose entry shape informs the member API shape), PP-996 (library
  links built from the document's `id`), PP-1003 (the follow-up
  for `relates_to`)
- Research: `meta/research/codebase/2026-11-24-PP-994-generic-set-model-indexing-and-api.md`
- Set contract: `skills/research/research-topic/SKILL.md`
- Design-inventory layout: `skills/design/inventory-design/SKILL.md`
- `meta/decisions/ADR-0067-kind-discriminated-corpus-schema-and-templates.md`
  (nearest precedent for the ADR this story writes)
- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
  (governs the `corpus` crate the set declaration lives in)
