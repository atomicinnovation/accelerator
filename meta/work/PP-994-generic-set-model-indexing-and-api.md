---
type: "work-item"
id: "PP-994"
external_id: "PP-994"
aliases: ["draft-0wfg5p"]
title: "Generic Set Model: Indexing and API"
date: "2026-11-23T16:12:42+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:PP-868"
tags: ["research", "visualiser"]
last_updated: "2026-11-23T21:27:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
blocks: ["work-item:PP-134", "work-item:PP-998", "work-item:PP-1001", "work-item:PP-1000"]
relates_to: ["work-item:PP-862", "work-item:PP-995"]
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

Topic research and design inventories move onto the model here. A reader
sees three changes: member documents appear in search, each result opening
its set until member routes land; a set's activity row and latest-change
time now follow its members' changes. The story unblocks member routes and the
member view (PP-998) and screenshot rendering for design inventories (PP-134).

## Context

Child of PP-868 — Topic-Research Set Detail Page. It is one of PP-868's
children and has no blockers. Member routes and the member view were split
from it at the member API and are PP-998.

Two doc types are directories today: topic research (root `manifest.md`) and
design inventories (root `inventory.md`). The visualiser indexes one entry
per directory, keyed on the root, and sees nothing else in it. More
multi-document doc types will follow, work items with attachments among them,
so the model is built as a pattern and not per type.

The index feeds search, activity, related, the library structure, lifecycle
clusters, and the type listing. None distinguishes a root from a member, so
each needs a rule once members are indexed.

## Requirements

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
  as such, so a set page can list it as unreadable.
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
  search results. A member result's link opens the member's set until PP-998
  switches it to the member's own URL.
- Roll a set's member changes up to the set in the activity feed: one row
  per set, identified by the set's root, reading as an edit to the set
  whatever the member's own action. The row reads created or deleted only
  when the root itself is.
- Take a set's latest-change time in the library structure from the newest
  change to its root or any member.
- Return related items for the document whose frontmatter carries the
  linkage, in the typed-linkage fields the related response reads today
  (`parent`, `relates_to`, `target`, and the work-item references). The
  linked document returns the member as an inbound item. Members take no
  part in inferred, cluster-derived relations.
- Serve a member document's body by its path through the document endpoint,
  `/api/docs/{path}`, as for any document.
- Expose each doc type's set declaration and presentation declaration to
  the frontend, and carry a set's slug on every response that names a root
  or a member, so no client derives a link from a root's filename.
- Serve a set's root and members through the API: each document with its
  parsed frontmatter, each asset with its path, size, and modification time
  (see Open Questions for the shape).
- Serve no asset bytes; that belongs to PP-134. A request for an asset's
  path through the document endpoint is not-found.

### Live update

- Emit a change event carrying the member's own path when a member document
  or asset is added, changed, or removed.
- Emit nothing for an excluded file.

### Design inventories

- Move design inventories onto the model: `inventory.md` is the root and
  entry-point document, and the files under `screenshots/` are asset members.
- Leave their library listing and their page showing what they show today.

## Acceptance Criteria

- [ ] Given a topic-research set with a manifest and ten member documents,
      then the member API returns the root and ten members, and the
      topic-research type listing shows one entry.
- [ ] Given a set with members at `brief.md`, `findings/01-a.md`, and
      `findings/round-2/notes/02-b.md`, and an asset at
      `findings/round-2/img/x.png`, then all four are returned by the member
      API with those set-relative paths, and a change to the deepest emits
      one change event.
- [ ] Given a set whose root has unclosed frontmatter, then the root is
      marked malformed and the member API still returns its members.
- [ ] Given a set directory with no root file, then nothing in it is
      indexed.
- [ ] Given a member document, when its path is requested from
      `/api/docs/{path}`, then its body is returned.
- [ ] Given the doc types response, then topic research and design
      inventories are each marked as sets with their presentation, and no
      other type is.
- [ ] Given a search for an asset's filename, then no asset is returned.
- [ ] Given a library holding only that set, whose members carry status and
      slug values the root does not, then the library structure's count for
      topic research is one and its status and slug facets hold only the
      root's values.
- [ ] Given a library holding only that set, then no lifecycle cluster holds
      a member, and the set's root clusters as it did before.
- [ ] Given a set whose root last changed yesterday, when a member document
      changes now, then the library structure's latest time for that set,
      and for topic research, is now.
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
      carrying the member's path relative to its set, and its link opens the
      member's set.
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
      once the write settles exactly one change event is emitted for that
      member's path, with the action created, edited, or deleted, and none
      for the root or any other member; the same holds for an asset
      member.
- [ ] Given a doc type declared as a set only in test code, through the
      mechanism topic research uses, with root `index.md`, two member
      documents, and one asset: the member API returns all three members; a
      phrase unique to one member document returns that member from search;
      the type listing shows one entry; a change to a member emits one
      change event.
- [ ] Outside the doc-type declarations and tests, the indexer, file driver,
      and watcher name neither `TopicResearch` nor `DesignInventories` in
      code that handles sets.
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
- [ ] Given the same design inventory, then the design-inventory listing
      shows one row for it with the title, status, and date it showed
      before, and its page renders the same body, with screenshot paths as
      inline code.
- [ ] An ADR records the set model, including assets as members and the two
      presentations, and the author has accepted it through `/review-adr`,
      so its status is `accepted`.

## Open Questions

- Set-member API shape (planning-time): does a set page get its members
  from a new set endpoint, or from an augmented `/api/docs/{path}` detail
  response that includes siblings? The set page (PP-1001), Sources
  rendering (PP-1000), and PP-134 consume the answer. It covers asset
  enumeration, which settles PP-134's own question on listing screenshots,
  and it must leave room for a document's structured sources.
- Can a set doc root sit under another configured doc root (planning-time
  check)? The story assumes it cannot; a contrary finding reopens the
  indexing requirements.

## Dependencies

- Blocked by: nothing.
- Blocks: PP-1001 and PP-1000 directly, for the member API, and:
- Blocks: PP-998, member routes and the member view, which read the member
  index, the member API, and change events; and PP-134, which serves and
  renders the asset members this story indexes. Through PP-998 it also gates
  the set page (PP-1001) and Sources rendering (PP-1000), which
  consume the member API directly.
- Planning order: the API shape and the accepted ADR are this story's
  first deliverables and release downstream planning on their own. The
  Sources parser's entry shape (PP-995) informs the API shape without
  blocking it. This story's API shape is settled, and the ADR accepted,
  before PP-998, PP-1001, PP-1000, and PP-134 are planned.
- On-disk conventions: the dot-prefix exclusion relies on what the
  research-topic skill writes, and the asset rule on the layout the
  design-inventory skill writes: `inventory.md` with `screenshots/` and
  `assets/`, every file under which is an asset member. A change to
  either layout must be matched here.
- Shared crate: the set declaration extends `DocTypeKey` in the `corpus`
  crate, which every `cli/` crate that reads doc types depends on; ADR-0069
  governs that edge.
- Existing consumers of change events: the frontend's query invalidation
  starts receiving member events when this story lands. No route shows a
  member yet, so the external-edit toast cannot fire for one.

## Assumptions

- Assets are indexed and listed but have no URL and are not served, so the
  asset half of the model has no end-to-end consumer until PP-134 and may
  need changes then.
- One activity row per set is read as "the latest change replaces the set's
  earlier row"; a count of changes on the row is not required.
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
  routes, the member view, and the corrected set links moved to PP-998.
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
  PP-1001 (the set page), PP-1000 (Sources rendering)
- Set contract: `skills/research/research-topic/SKILL.md`
- `meta/decisions/ADR-0067-kind-discriminated-corpus-schema-and-templates.md`
  (nearest precedent for the ADR this story writes)
- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
  (governs the `corpus` crate the set declaration lives in)
