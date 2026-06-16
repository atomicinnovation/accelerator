---
type: adr
id: "ADR-0044"
title: "Remote Work-item Identity in external_id (Presence-based Sync)"
date: "2026-06-16T07:30:58+00:00"
author: Toby Clemson
producer: create-adr
status: accepted
parent: "work-item:0047"
relates_to: ["adr:ADR-0033", "adr:ADR-0040", "adr:ADR-0025", "adr:ADR-0022"]
tags: [work-management, integrations, frontmatter, identity, sync]
last_updated: "2026-06-16T08:09:15+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# ADR-0044: Remote Work-item Identity in external_id (Presence-based Sync)

**Date**: 2026-06-16
**Status**: Accepted
**Author**: Toby Clemson

## Context

Work items carry a local identity and, once pushed to a remote tracker (Jira,
Linear, Trello, GitHub Issues), a remote identity. Historically the field name
`work_item_id` accreted **three distinct roles**:

- **(A) Legacy own-identity** — the pre-`id` schema's identity field (ADR-0022
  named it; migration 0001 rewrote `ticket_id:` → `work_item_id:`). Work items now
  carry `id` as own-identity (ADR-0033); `work-item-read-field.sh` still bridges
  legacy `work_item_id` own-identity on un-migrated files. The own-identity
  field-name canonicalisation (`work_item_id` → `id` across templates and corpus)
  is a separate workstream owned by 0065/0070.
- **(B) Foreign-reference linkage** — a typed cross-reference from one artifact to
  its work item (`work_item_id:`), classified as a foreign reference by ADR-0033
  and ADR-0040. Its spelling was canonicalised by work item 0064 (done), which
  renamed plan frontmatter's `work-item:` → `work_item_id:`.
- **(C) Remote-key writeback** — the Linear create flow wrote the remote tracker's
  identifier back into `work_item_id`.

ADR-0033 has since introduced `id` as every artifact's own-identity and seeded
`external_id` as a work-item cross-system pointer. The sync-awareness work
(work item 0047) forced the question of how to signal "this item exists in the
remote tracker". The original framing classified sync state by the *format* of
the identity field (`^[0-9]+$` ⇒ local, remote-shaped ⇒ synced), which
misclassifies project-coded local IDs such as `PROJ-0042` as synced. With
`id` now general and `external_id` available, the overloading of `work_item_id`
and the format-based signal are both avoidable.

## Decision Drivers

- A sync signal that is robust under any `work.id_pattern` (must not depend on the
  shape of the local `id`).
- One field, one role: reduce the cognitive and correctness hazard of a single
  name carrying three meanings.
- Reuse the schema that already exists (`id`, `external_id` per ADR-0033) rather
  than inventing new fields.
- Forward-compatibility for trackers whose identifiers do not align with the local
  scheme (Trello opaque card IDs, GitHub `owner/repo#42`).
- A single documented home for the work-item identity model, whose field roles
  are otherwise spread across ADR-0033/0040/0025 and several migrations.

## Considered Options

1. **Remote key in `work_item_id`, classify by format** — the original 0047
   framing: keep writing the remote identifier into `work_item_id` and detect
   "synced" by matching a remote-shaped pattern.
2. **Reuse `id` as the remote key when schemes align** — let `id` hold the remote
   identifier for Jira/Linear, renaming files to the remote key.
3. **Dedicated `external_id` for the remote identifier; presence-based sync;
   retire only `work_item_id`'s remote-key role** (chosen).
4. **A separate local→remote mapping store** keyed by `id`, leaving the work-item
   frontmatter free of the remote identifier.

## Decision

We will treat the work-item identity model as three clearly separated concerns:

- **`id`** is the **local own-identity** (`(<project-code>-)?\d{4}`), always
  allocated locally and authoritative for the filename.
- **`external_id`** is the **remote tracker's identifier**. It IS the per-item
  local→remote mapping — no separate mapping store. It may **equal `id`**
  (Jira/Linear, aligned schemes) or be **independent** (Trello, opaque card IDs),
  and is written on push success, even when equal to `id`.
- **Sync classification is presence-based**: a non-empty `external_id` ⇒ *synced*
  (the item is known to exist remotely), an absent/empty one ⇒ *unsynced*. The
  test is presence, not value or format.

`work_item_id`'s **remote-key role (C) is retired** in favour of `external_id`;
the Linear create flow is refactored onto it. This ADR scopes only that
retirement. `work_item_id`'s other roles are untouched here: role (B)'s
foreign-reference spelling was already canonicalised by work item 0064 (done),
and role (A)'s own-identity field-name canonicalisation (`work_item_id` → `id`
across templates and corpus) is a separate workstream (0065/0070).

This is option 3. Option 1 was rejected because it deepens the `work_item_id`
overload and its format signal misclassifies project-coded local IDs. Option 2
conflates local and remote identity, breaks for independent schemes, and makes
file renames invasive. Option 4 is unnecessary for the existence signal —
`external_id` already is the per-item mapping — and the only baseline that *does*
need separate storage (content-parity, `last-sync.json`) is a distinct concern
owned by work item 0051.

## Consequences

### Positive

- The sync signal is robust under any `work.id_pattern`; the `PROJ-0042`
  misclassification is eliminated.
- One field per role removes a standing correctness and comprehension hazard.
- Minimal schema change: `id` and `external_id` already exist (ADR-0033); no new
  fields and no separate mapping store.
- Forward-compatible — independent remote schemes (Trello, GitHub Issues) fit
  without touching `id`.

### Negative

- The field name `work_item_id` is not yet single-purpose system-wide: it remains
  the foreign-reference key (role B, spelling canonical after 0064) and is still
  read as legacy own-identity (role A) on un-migrated files until the 0065/0070
  own-identity canonicalisation lands. This ADR narrows the overload by removing
  the remote-key role, not the others.
- `external_id` is reframed from "foreign reference" (its grouping in ADR-0040)
  to "remote-identity pointer". The omit-when-empty rule still applies unchanged.
  ADR-0040 is accepted and therefore immutable, so its categorisation is left
  unedited; this ADR is the clarifying reference for `external_id`'s role.

### Neutral

- `external_id` may equal `id` or differ from it; presence — not value — is the
  signal.
- The Linear integration is unreleased, so retiring the `work_item_id` remote-key
  writeback needs no data migration: no in-the-wild item carries a remote-key
  `work_item_id`.

## References

- `meta/work/0047-core-skills-sync-integration.md` — work item that forced the decision
- `meta/plans/2026-06-15-0047-core-skills-sync-integration.md` — implementing plan (flagged this ADR)
- `meta/research/codebase/2026-06-15-0047-core-skills-sync-integration.md` — research enumerating the three `work_item_id` uses
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — defines `id`; first introduces `external_id`
- `meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md` — omit-when-empty rule for `external_id`/`work_item_id`
- `meta/decisions/ADR-0025-work-item-cross-ref-aggregation.md` — relies on `work_item_id` roles (A)/(B)
- `meta/decisions/ADR-0022-work-item-terminology.md` — named the `work_item_id` field
- `meta/work/0064-canonicalise-work-item-id-and-author-fields.md` — canonicalised the foreign-reference spelling (done)
- `meta/work/0065-update-artifact-templates-to-unified-schema.md` / `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` — own the work-item own-identity `work_item_id` → `id` canonicalisation
