---
title: Set member API
type: spec
status: draft
date: "2026-11-24"
last_updated: "2026-11-25"
author: Toby Clemson
slug: set-member-api
---

# Set member API

## Purpose

Serve a set's root and every member in one response, so the set page,
member routes, Sources rendering and screenshot rendering read a set
without walking its directory, and state how members appear in the
visualiser's other responses. The set model is ADR-0077, and set shapes
and presentations are declared under ADR-0078. This document is the
visualiser's contract.

## Endpoint

`GET /api/sets/{type}/{setId}`

- `type` is a doc type's wire token.
- `setId` is decoded once, by the router's path extractor, and matched
  against the `setId` of the type's roots. A set's `id` is its root's
  frontmatter `id`, or else its directory's name. When two sets share an
  `id`, the lexically first directory wins.
- `400` when `type` is not a known wire token.
- `404` when the type is not a set type, or no set matches `setId`.

## Response

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

- `set.presentation` is `set-page` or `entry-point`.
- A document's `frontmatterState` is `parsed`, `malformed` or `absent`.
  The root is served whatever its state, and so are its members.
- An asset carries its byte `size` and `mtimeMs`, and no `etag`.

## Paths

- Each `setRelativePath` is a plain, `/`-separated string relative to the
  set directory, never percent-encoded. It carries the same name as the
  `setRelativePath` field on index entries and search rows.
- `set.directory` is relative to the project root.
- Members are sorted by `setRelativePath`.
- Every member is in one response, with no paging. The largest real
  inventory held 35 screenshots.
- The response holds no document body. A body comes from
  `/api/docs/{directory}/{setRelativePath}`. A consumer building that URL
  percent-encodes each path segment, because file names may hold `?`, `#`
  or `%`.

## Members in other responses

- **Type listing, counts, facets and lifecycle clusters** hold roots only.
  A member has an index record but is never counted, listed or clustered
  as an item.
- **Search** returns member documents, matched on their own title,
  filename stem and first paragraph. A member's row carries the set's
  slug and `setId` as its link keys, and its own `setRelativePath`. The
  link keys are never matched against a query, and no member ranks as an
  exact match for its set's slug. Assets are never search results.
- **Related items.** A member document's declared linkage belongs to that
  member, not to its root or its set. Members take no part in inferred
  related items. A related item that is a member carries `setId`.
- **Activity** keeps one row per set, identified by the root's path. The
  row's action is the root's latest own action, and its time is the
  latest change to the root or any member. Activity rows carry no `id`.
- **The library's latest time** for a set counts its root, its members,
  its assets and the removals the server has observed. Removals are
  remembered in memory only, so after a restart the latest time falls
  back to the files' modification times, and can move backwards.
- **`/api/docs/{path}`** returns not-found for an asset, for a file in a
  set directory that fits no slot, for a file in a directory that has no
  root, and for a directory.
- **Symlinks.** Members are not found through a symlinked directory, and a
  member that resolves outside the configured doc roots is dropped.

## Change events

A `doc-changed` event may name an asset member. Its `etag` is absent for
an asset as well as for a deletion, so a missing `etag` alone does not
mean the path was deleted. Change events carry a path and no `id`.

## Extension points

- **Structured sources.** A document member's structured sources attach
  as an optional `sources` field on a document object. The field is
  absent until Sources rendering (PP-1000) defines it.
- **The derived set response.** It is a sibling resource at
  `GET /api/sets/{type}/{setId}/derived` (PP-1002), unregistered until
  then.

## Doc types response

Each entry of the doc types response gains a `set` field:

```ts
set: {
  root: string;
  documentSlots: string[];
  assetSlots: string[];
  presentation: "set-page" | "entry-point";
} | null
```

It is `null` for every doc type that is not a set.
