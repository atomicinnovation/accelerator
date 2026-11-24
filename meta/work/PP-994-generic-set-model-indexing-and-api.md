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
last_updated: "2026-11-24T00:06:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
blocks: ["work-item:PP-134", "work-item:PP-998", "work-item:PP-1001", "work-item:PP-1000", "work-item:PP-1002"]
relates_to: ["work-item:PP-862", "work-item:PP-995", "work-item:PP-996"]
---
# PP-994: Generic Set Model: Indexing and API

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As a maintainer of the visualiser, I want a doc type whose entries are
directories to be indexed as a set — a root document plus its member
documents and assets — so that any multi-document doc type gets member
indexing, search, live update, and a member API from one model.

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

## Requirements

This story is server-only. Its outcomes are stated on API responses and
change events, and the frontend changes only the types that describe those
responses. Every change to what the frontend does with a member belongs to
PP-998.

### Model

- Model a set as a root document plus members. A member is a document (a
  markdown file, whether its frontmatter is parsed, malformed, or absent)
  or an asset (any other file).
- Let a doc type make two declarations. Its **set declaration** says it is
  a set and names its root file; set handling branches on this and nothing
  else, never on a named doc type. Its **presentation declaration** says
  how the set presents: a set page that lists its members (topic research),
  or an entry-point document that presents them within its own page (design
  inventories). The presentation declaration is recorded here and changes
  no behaviour until the set page and PP-134 read it.
- Record the model in an ADR, including assets as members and the two
  presentations, and have it accepted.

### Indexing

- Index every member of a set at any depth, each with its path relative to
  the set directory; each document with its parsed frontmatter and each
  asset with its size and modification time.
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

### Index consumers

- Keep the type listing, the library structure's counts and facets, and the
  lifecycle clusters at one item per set: a member has an index record but is never
  counted, listed, or clustered as an item of its own.
- Return a member document as a search result in its own right, titled as
  the member and carrying its set-relative path. Asset members are not
  search results. A member's search row carries the set's slug in the row's
  existing `slug` field, so the unchanged frontend, which links a result by
  that field, opens the member's set until PP-998 switches the link to the
  member's own URL.
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
- Attribute a member document's typed linkage to that member, not to its
  root or its set: related items are returned for the document whose
  frontmatter carries the linkage, in the typed-linkage fields the related
  response reads today (`parent`, `relates_to`, `target`, and the work-item
  references). The linked document returns the member as an inbound item.
  Members take no part in the related response's inferred items, which are
  derived from lifecycle clusters.
- Serve a member document's body by its path through the document endpoint,
  `/api/docs/{path}`, as for any document.
- Expose each doc type's set declaration and presentation declaration to
  the frontend.
- Carry a set's slug, which is the slug its type-listing entry carries
  today (the directory's name for topic research, and a shortened form of
  it for a design inventory), on every response that names a root or a
  member, so no client derives a link from a root's filename. Those
  responses are the type listing, the document response, search results,
  related items, and the member API. An activity row and a change event
  each carry a path and no slug: the activity feed finds a set's slug from
  the type listing, as PP-996 has it.
- Serve a set's root and members through the **member API**: each document
  with its parsed frontmatter, each asset with its path, size, and
  modification time. The name stands for this capability whichever endpoint
  carries it (see Open Questions for the shape), and its response always
  includes the root.
- Serve no asset bytes; that belongs to PP-134. A request for an asset's
  path through the document endpoint is not-found.

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
- Emit nothing for an excluded file.

### Design inventories

- Move design inventories onto the model: `inventory.md` is the root and
  entry-point document. Its other files are classified by the Model's rule,
  by file type and not by directory: the images under `screenshots/` are
  asset members, as is any other non-markdown file, and a markdown file
  anywhere in the directory is a document member.
- Leave their library listing and their page showing what they show today.

## Acceptance Criteria

- [ ] Given a topic-research set with a manifest and ten member documents,
      then the member API returns the root and ten members, and the
      topic-research type listing response holds one entry.
- [ ] Given a set with members at `brief.md`, `findings/01-a.md`, and
      `findings/round-2/notes/02-b.md`, and an asset at
      `findings/round-2/img/x.png`, then all four are returned by the member
      API with those set-relative paths, and a change to
      `findings/round-2/notes/02-b.md` emits exactly one change event,
      carrying that member's path.
- [ ] Given a set whose root has unclosed frontmatter, then the root is
      marked malformed and the member API still returns its members.
- [ ] Given a set directory with no root file, then nothing in it is
      indexed.
- [ ] Given a member document, when its path is requested from
      `/api/docs/{path}`, then its body is returned.
- [ ] Given the doc types response, then topic research and design
      inventories are each marked as sets with their presentation, and no
      other type is.
- [ ] Given a CSS asset member holding a phrase found nowhere else, when
      that phrase and the asset's filename are each searched for, then no
      result is returned, and the member API still lists the asset.
- [ ] Given a library holding only one topic-research set, whose member
      documents carry status and slug frontmatter values the root does not,
      then the library structure's count for topic research is one and its
      status and slug facets hold only the root's values.
- [ ] Given a library holding only one topic-research set, then no lifecycle
      cluster holds a member, and the root is in the same cluster, with the
      same cluster members, as when the set's members are absent from disk.
- [ ] Given a set whose root last changed yesterday, when a member document
      changes now, then the library structure's latest time for that set,
      and for topic research, is that member's modification time; the same
      holds when an asset member changes.
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
- [ ] Given a phrase that appears only in one member document, when it is
      searched for, then the result is that member, titled as the member,
      carrying the member's path relative to its set, and its `slug` field
      holds the set's slug.
- [ ] Given member changes to set A, then a change to set B, then further
      changes to set A's root, a member document, and an asset, then the
      activity feed shows exactly one row for A, above B's, carrying the
      time of A's latest change, and no row naming a member.
- [ ] Given a member document whose `relates_to` names a plan outside the
      set, when related items are requested for that member's path, then the
      plan is returned; requested for the set's root or another member, it
      is not; requested for the plan, the member is returned as an inbound
      item.
- [ ] Given a member document is added, changed, or removed on disk, then
      after one write followed by quiet longer than the watcher's debounce
      interval exactly one change event is emitted for that
      member's path, with the action created, edited, or deleted, and none
      for the root or any other member; the same holds for an asset
      member.
- [ ] Given a doc type declared as a set only in test code, through the
      mechanism topic research uses, with root `index.md`, two member
      documents, and one asset: the member API returns the root and all three
      members; a phrase unique to one member document returns that member
      from search; the type listing response holds one entry; a change to
      a member emits one change event; the activity feed holds one row for
      the set; the library structure counts one item, with the root's
      facets alone and a latest time that follows a member's change; no
      lifecycle cluster holds a member; a dot-prefixed file in the set is
      absent from all of these; the responses carry the set's slug; and
      the doc types response marks the type as a set.
- [ ] A search of the visualiser server's sources for `TopicResearch` and
      `DesignInventories` finds matches only in the doc-type declarations
      and in test modules.
- [ ] Topic research declares the set-page presentation and design
      inventories the entry-point presentation, each readable from its doc
      type's declaration.
- [ ] Given a set with a root, ten member documents (one with unclosed
      frontmatter, one with none), and six assets, when its members are
      requested through the API, then the response holds the root and all
      sixteen members: each document with its parsed frontmatter or its
      malformed or absent mark, each asset with its byte size and
      modification time, and every member with its path relative to the set
      directory. No document body is in the response.
- [ ] Given a design inventory with a `screenshots/` directory of six images,
      then the six are indexed as asset members, and requesting one image's
      path through the document endpoint returns not-found.
- [ ] Given the same design inventory, then the design-inventory type
      listing response holds one entry for it, carrying the title, status,
      and date of `inventory.md`'s frontmatter, and the document response
      for `inventory.md` carries that file's body unchanged.
- [ ] Given a set with slug `x`, then the type listing entry for it, the
      document response for its root and for a member document, a search
      result for a member, a related item naming a member, and the member
      API response each carry `x` as the set's slug; the same holds for a
      design inventory and the slug on its type-listing entry.
- [ ] Given a set whose root was last edited, when one member document is
      added and another removed, then the set's activity row reads edited
      and carries the time of the later change; given a newly created set
      whose row reads created, when a member changes after a further quiet
      period, then the row still reads created, with the new time.
- [ ] Given a directory `.x.tmp/` holding a root, two member documents, and
      an asset, when it is renamed to `x/`, then four change events are
      emitted, each reading created and carrying its own path; the activity
      feed shows one row for the set, reading created; and the member API
      returns the root and three members.
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
- [ ] Given a design inventory with a `notes/` directory holding a CSS file
      and a markdown file, then the CSS file is indexed as an asset member
      and the markdown file as a document member.
- [ ] Given a member document and a plan outside its set that carry the
      same slug in their filenames, when related items are requested for
      either, then the other is not among the inferred items, and no
      inferred item of any response is a member.
- [ ] An ADR records the set model, including assets as members and the two
      presentations, and the author has accepted it through `/review-adr`,
      so its status is `accepted`.
- [ ] The accepted ADR, or a document it names, records the member API's
      endpoint and response shape, covering asset enumeration, where a
      document's structured sources attach, and where the derived set
      response sits beside the member listing.
- [ ] This work item's Drafting Notes carry one dated decisions line
      recording three things: the outcome of the nested-doc-root check,
      made before the ADR was accepted; the ADR's number and its accepted
      status; and the document that holds the member API shape.

## Open Questions

- Set-member API shape (planning-time): does a set page get its members
  from a new set endpoint, or from an augmented `/api/docs/{path}` detail
  response that includes siblings? The set page (PP-1001), Sources
  rendering (PP-1000), and PP-134 consume the answer. It covers asset
  enumeration, which settles PP-134's own question on listing screenshots.
  It must leave room for two things other stories add: a document's
  structured sources (PP-1000), and the derived set response
  (PP-1002) served beside the member listing (the outline's rounds
  and items, focus areas, pair states, the set's status, and the stale and
  reopened marks).
- Awaiting the author's confirmation, and stated as settled in
  Requirements until then: an activity row and a change event carry no
  set slug; any markdown file in a design inventory is a document member.
- Can a set doc root sit under another configured doc root (planning-time
  check)? The story assumes it cannot; a contrary finding reopens the
  indexing requirements.

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
  first stage is three decisions, in this order: the nested-doc-root
  check, then the member API shape, then the accepted ADR. The second is
  the implementation. The first stage is complete when the dated
  **decisions line** in Drafting Notes exists, recording the check's
  outcome, the ADR's number, and where the API shape is written down.
  PP-998, PP-1001, PP-1000, PP-134, and PP-1002 are planned only once
  that line exists; it is their release, and they need not wait for this
  story to land. PP-134's planning also waits on PP-998's route shape. The
  Sources parser's entry shape (PP-995) informs the API shape without
  blocking it; if that entry shape is not settled in time, the API shape
  carries sources as an extension point it does not define.
- Independent of PP-996, the bug whose fix builds set links from the slug:
  either may land first. PP-996's activity feed looks a set's slug up in
  the type listing by the row's path, in both orders, so this story keeps
  a set's activity row identified by its root's path.
- On-disk conventions: the dot-prefix exclusion relies on what the
  research-topic skill and the research CLI write (the CLI writes the run
  ledger), and the design-inventory fixtures on the layout the
  inventory-design skill writes: `inventory.md` with `screenshots/`, built
  in a dot-prefixed temporary directory and renamed. A change to either
  layout must be matched here.
- Shared crate: the set declaration extends `DocTypeKey` in the `corpus`
  crate, which every `cli/` crate that reads doc types depends on; ADR-0069
  governs that edge.
- Existing consumers of change events: the frontend's query invalidation
  starts receiving member events when this story lands. No route shows a
  member yet, so the external-edit toast cannot fire for one.
- Existing consumers of the search and activity responses: the frontend's
  search results panel and activity feed. Both keep working unchanged: a
  member's search row is an ordinary row whose `slug` is its set's, and a
  set's activity row is identified by its root, as today.

## Assumptions

- A set doc root does not sit under another configured doc root. The
  planning-time check in Open Questions verifies it; a contrary finding
  reopens the indexing requirements.
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
search, activity, and related); a member API is added,
and two pinned tests and their fixtures are rewritten.

- The index is a map of canonical path to `IndexEntry` with no notion of
  root and member (`cli/visualiser/server/src/indexer.rs:247`).
  `LocalFileDriver::list` emits one `<dir>/<root>` per set, one level deep
  (`file_driver.rs:275-327`).
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
- Activity is a 50-slot ring buffer filled in `watcher::emit`
  (`watcher.rs:328-333`, `activity_feed.rs:43-60`); the roll-up belongs
  there, while the event itself keeps the member's path.
- Two tests pin one entry per set and must change:
  `topic_research_indexes_one_entry_per_set_from_its_manifest`
  (`indexer.rs:2945`) and `design_inventories_indexed_from_nested_directories`
  (`indexer.rs:2885`). Only one fixture set has members, and none has assets.
- A member's doc type is resolved by path prefix in
  `kind_for_canonical_path` (`file_driver.rs:506-511`); see the open
  question on nested doc roots.
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

## References

- Parent: PP-868
- Related: PP-134 (consumes asset members), PP-862 (pinned one entry per
  set, which this story changes), PP-998 (member routes and view),
  PP-1001 (the set page), PP-1000 (Sources rendering), PP-1002 (the
  derived set response), PP-995 (the Sources
  parser, whose entry shape informs the member API shape), PP-996 (set
  links built from the slug)
- Set contract: `skills/research/research-topic/SKILL.md`
- Design-inventory layout: `skills/design/inventory-design/SKILL.md`
- `meta/decisions/ADR-0067-kind-discriminated-corpus-schema-and-templates.md`
  (nearest precedent for the ADR this story writes)
- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
  (governs the `corpus` crate the set declaration lives in)
