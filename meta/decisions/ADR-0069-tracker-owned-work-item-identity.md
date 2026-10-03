---
type: "adr"
id: "ADR-0069"
title: "Tracker-Owned Work Item Identity"
date: "2026-09-27T14:40:00+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "accepted"
decision_makers: ["Toby Clemson"]
parent: "work-item:0230"
supersedes: ["adr:ADR-0044"]
relates_to: ["adr:ADR-0034", "adr:ADR-0033"]
tags: ["work-management", "integrations", "identity", "sync", "tracker", "drafts"]
last_updated: "2026-09-27T14:40:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0069: Tracker-Owned Work Item Identity

**Date**: 2026-09-27
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0044 separated a work item's local own-identity (`id`, always minted
locally) from its remote tracker identifier (`external_id`), and classified
sync state by the presence of `external_id`. It considered and rejected
reusing `id` as the remote key when the schemes align, on three grounds: it
conflates local and remote identity, it breaks for trackers with independent
identifier schemes (Trello card IDs, GitHub `owner/repo#42`), and it makes
file renames invasive.

In practice every synced item now carries two identifiers. This repository's
corpus holds 294 work items, all synced, each with a local `NNNN` `id`
distinct from its `external_id` (`0230` and `PP-760`). Developers translate
between them by hand across conversation, commits, branch names and tracker
links.

The forces behind ADR-0044's rejection have shifted:

- **Opt-in scope.** The behaviour can be enabled per repository through
  `work.id_pattern` rather than imposed on every corpus, so repositories
  whose trackers do not align keep ADR-0044's model unchanged.
- **Aligned trackers only.** Restricting it to Jira and Linear, whose issue
  keys always take the shape `<KEY>-<number>`, removes the independent-scheme
  objection.
- **Retirement over renames.** File renames become a single, all-or-nothing
  operation — ID retirement — that records the old identifier in `aliases`
  and rewrites references within `meta/`, rather than ad hoc renames.

A tracker-owned `id` needs the tracker's key before the local file is
written. Creations that cannot obtain a key — no push requested, tracker
unreachable, or an outcome that leaves the remote state unknown — still need
a referenceable identity.

## Decision Drivers

- One identifier per work item, used everywhere, for teams whose work items
  live in Jira or Linear.
- No change for existing corpora or for trackers with independent schemes.
- An `id` that is stable once the tracker has confirmed it, changing only
  for explicit, recorded reasons.
- A provisional identity that can never be mistaken for a tracker key.
- Sync classification that stays robust under every `id_pattern`.

## Considered Options

1. **Keep ADR-0044's model** — `id` always minted locally, remote key only in
   `external_id`.
2. **Tracker-owned `id` under an opt-in `{tracker}` pattern, with drafts for
   unconfirmed creations** (chosen).
3. **Local `id` with a display alias** — keep minting locally but render the
   tracker key in tooling output, leaving files and references on the local
   `id`.

## Decision

We will let the tracker own a work item's `id` when `work.id_pattern` is
`"{tracker}"` and `work.integration` is `jira` or `linear`:

- Under `{tracker}`, `id` equals `external_id` and is set by the tracker.
  `work create --push` creates the remote issue first and adopts its key as
  `id`, filename prefix and H1.
- Creations that do not confirm a tracker issue are **drafts**: items whose
  provisional `id` is `draft-` followed by six lowercase Crockford base32
  characters, at least one of them a letter, living in `meta/work/drafts/`.
  Drafts are first-class typed-link targets and are promoted to their tracker
  key by `work sync` or `work promote`.
- An `id` changes only through draft promotion, a tracker-side key change, or
  an explicit re-key. Each goes through **ID retirement**, which moves the
  file, sets `id` and the H1, appends the old identifier to `aliases`, and
  rewrites references to it within `meta/`, all-or-nothing.
- Sync classification stays presence-based, as ADR-0044 decided: a non-empty
  `external_id` means the item exists remotely.

This is option 2. Option 1 keeps the two-identifier cost this decision exists
to remove. Option 3 removes it only from output: files, typed links and prose
still carry the local `id`, so developers still meet both.

## Consequences

### Positive

- Under `{tracker}`, a work item has one identifier across files, typed links,
  prose, commits and the tracker.
- ID retirement gives promotion, tracker-side key changes and re-keying one
  shared, recorded mechanism; `aliases` keeps retired identifiers resolvable.
- Repositories that do not opt in are unaffected.

### Negative

- ID retirement renames files and rewrites references across `meta/`, so a
  promotion or key change touches many files in one change.
- Typed links inside accepted ADRs are rewritten on retirement. A typed link
  is a reference to another artifact, not decision content, so rewriting it
  does not breach ADR immutability; prose inside accepted ADRs is rewritten
  under the same whole-token rules as any other `meta/` file.
- Drafts add a second state an item can be in locally, which every corpus
  reader must discover under `meta/work/drafts/`.

### Neutral

- Items created before `{tracker}` keep their legacy identifiers; pushing an
  unsynced legacy item sets only `external_id`.
- ADR-0044's separation of `id` and `external_id` still holds under every
  other `id_pattern`.

## References

- `meta/work/0230-tracker-owned-work-item-id-generation.md` — work item that
  forced the decision
- `meta/plans/2026-09-26-0230-tracker-owned-work-item-id-generation.md` —
  implementing plan
- `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md` —
  superseded decision
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — defines `id`
  and `external_id`
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — typed links rewritten
  by ID retirement
