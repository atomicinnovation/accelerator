---
type: "note"
id: "2026-11-23-release-the-serving-slot-during-backoff"
title: "Release a start-spaced service's serving slot during backoff"
date: "2026-11-23T15:52:52+00:00"
author: "Toby Clemson"
producer: "respond-to-pr"
status: "captured"
topic: "Whether a Crossref call should give up its serving slot while it sleeps a 5xx or transport-error backoff"
tags: []
revision: "e8be33d75eb1d936c3b841692e2bb90ba0c8b7c6"
repository: "accelerator"
last_updated: "2026-11-23T15:52:52+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Release a start-spaced service's serving slot during backoff

`Attempts::until_settled` in `cli/research/src/sources/serving.rs` sleeps the
3, 6 and 12 s retry backoff inside the serving turn, so the call keeps its
`serving-<i>.lock` slot while nothing is in flight. That suits arXiv, whose
pacing is `PreviousFinish` and allows one connection at a time. Crossref's
pacing is `PreviousStart`, and its slots limit requests in flight. In the
public pool, which has one slot, a DOI call that draws a 5xx or a transport
error therefore stalls every other public caller for up to 21 s. Their own
spacing would let them send every 210 ms. Raised in the PR #258 review.

It was left as is in PP-891 for two reasons:

- Releasing the turn alone frees no one. `Place::is_front` counts every live
  ticket ahead, and a backing-off ticket keeps its place. A change needs a
  queue state in which a backing-off ticket does not contend, and
  re-admission that keeps the place. `Attempts` also borrows one
  `ServingTurn` for its whole life.
- Holding the slot gives backpressure. Backoff follows only a 5xx or a
  transport error, and those usually affect every caller. A 429 already
  defers every caller through the shared `not_before`.

Revisit with data: how often a Crossref 5xx is transient for one request
rather than a failing upstream, and how often a web-profile fan-out in the
public pool ends in `waiting` because of a stall like this. If the stalls
matter, scope the change to `PreviousStart` services and keep arXiv's
behaviour.
