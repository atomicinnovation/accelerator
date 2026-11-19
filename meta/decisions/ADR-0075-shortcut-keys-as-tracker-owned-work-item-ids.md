---
type: "adr"
id: "ADR-0075"
title: "Shortcut Keys as Tracker-Owned Work Item IDs"
date: "2026-11-17T14:55:18+00:00"
author: "Tom Scott"
producer: "create-adr"
status: "accepted"
decision_makers: ["Tom Scott"]
parent: "work-item:PP-990"
relates_to: ["adr:ADR-0070"]
tags: ["work-management", "integrations", "identity", "sync", "tracker", "shortcut"]
last_updated: "2026-11-17T14:55:18+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---

# ADR-0075: Shortcut Keys as Tracker-Owned Work Item IDs

**Date**: 2026-11-17
**Status**: Accepted
**Author**: Tom Scott

## Context

ADR-0070 lets the tracker own a work item's `id` when `work.id_pattern` is
`"{tracker}"`, and limits that to `work.integration` of `jira` or `linear`.
Work item 0391 added a Shortcut connector and left `{tracker}` for Shortcut
out of scope, because widening ADR-0070 needed its own decision.

A repository that already used `{tracker}` with Linear found the gap when it
switched to Shortcut. `accelerator work sync` refused to start with
`E_WORK_ID_PATTERN_INVALID`, and the sync skill gave no warning first. The
only way forward was a numbered `id_pattern`, which gives up the one
identifier per item that ADR-0070 exists to provide.

The connector names a Shortcut story `sc-<n>` and an epic `epic-<n>`.
Shortcut assigns the number once, and never changes it: a story that moves
team, workflow or epic keeps its number. The `sc-` and `epic-` prefixes keep
story keys and epic keys apart.
Both keys pass `issue_key_is_safe` under `TrackerKeys::Shortcut`, the key
grammar the client checks each created key against.

## Decision Drivers

- One identifier per work item for teams whose work lives in Shortcut, as
  ADR-0070 gives Jira and Linear teams.
- An `id` that never changes once the tracker confirms it.
- No change to drafts, promotion or ID retirement.

## Considered Options

1. **Keep `{tracker}` to Jira and Linear**, and make the refusal and the sync
   skill say that Shortcut needs a numbered `id_pattern`.
2. **Admit Shortcut to `{tracker}`**, so a story takes its `sc-<n>` key and an
   epic its `epic-<n>` key as `id` (chosen).

## Decision

We will admit `work.integration: shortcut` to `{tracker}`. Every rule in
ADR-0070 applies unchanged: drafts, promotion through ID retirement, and
presence-based sync classification. A pushed story takes `sc-<n>` as its
`id`, filename prefix and H1, and an epic takes `epic-<n>`.

This is option 2. Option 1 keeps Shortcut teams on two identifiers per item.
Shortcut keys never change, so a Shortcut key meets ADR-0070's stability
driver more strictly than a Jira key, which changes when an issue moves
project.

## Consequences

### Positive

- A Shortcut team can name its work items by their Shortcut keys, and a
  repository can switch an existing `{tracker}` setup to Shortcut without
  changing `id_pattern`.
- Promotion and retirement need no Shortcut-specific code. The promotion
  tests pass unchanged with lower-case `sc-` keys.

### Negative

- Shortcut keys are lower case, while Jira and Linear keys are upper case.
  Filenames such as `meta/work/sc-14-add-search.md` sort apart from
  numbered and upper-case items.
- A corpus still must not change tracker in place. Its existing `id`s are
  the old tracker's keys, as ADR-0070 already notes for any switch.

### Neutral

- `TrackerKeys::of_integration` maps `shortcut` to `TrackerKeys::Shortcut`,
  so `owns_ids` holds for it. The `E_WORK_ID_PATTERN_INVALID` message names
  all three trackers.

## References

- `meta/decisions/ADR-0070-tracker-owned-work-item-identity.md`
- `meta/work/PP-990-shortcut-integration.md`
- `meta/plans/2026-11-13-0391-shortcut-integration.md`
