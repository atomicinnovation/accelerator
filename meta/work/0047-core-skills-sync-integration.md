---
id: "0047"
title: "Core Skills Sync Integration"
date: "2026-05-06T17:49:44+00:00"
author: Toby Clemson
kind: story
status: ready
priority: high
parent: "work-item:0045"
tags: [work-management, integrations, sync, list-work-items, create-work-item]
type: work-item
schema_version: 1
last_updated: "2026-06-15T21:12:34+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0046"]
blocks: ["work-item:0051"]
---

# 0047: Core Skills Sync Integration

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

When `work.integration` is configured, extend `/list-work-items` to show a
colour-coded sync status label per item, and extend `/create-work-item` to
offer an interactive push to the remote after the item is drafted. This story
delivers the two locally-derivable sync states — **synced** (the item carries a
remote-format `work_item_id`) and **unsynced** (numeric `work_item_id`, never
pushed). The three baseline-dependent states (locally modified, remotely
modified, conflict) require the `last-sync.json` baseline produced by
`/sync-work-items` (story 0051) and are realised there; `/list-work-items` is
structured here so 0051 can extend it without rework. On push acceptance, the
remote allocates the issue key, which is written as `work_item_id` in the local
file; on decline, a local numeric ID is used instead.

## Context

With `work.integration` configured (prerequisite story 0046), the core work
management skills need sync awareness. Throughout this story, "the remote" means
the single remote system declared by `work.integration` (jira, linear, trello,
or github-issues — see 0046); only one integration is active at a time. Two
skills are in scope: `/list-work-items` gains visibility into the sync state of
each work item relative to the remote, and `/create-work-item` gains a post-draft
push offer so newly created items can be pushed immediately.

The convention underpinning both is that a numeric `work_item_id` means "never
pushed" (unsynced), and a remote-format `work_item_id` means the item exists in
the remote system. Without a last-sync baseline, a remote-format ID is *presumed
synced* — the item is known to exist remotely, but content parity cannot be
computed until `/sync-work-items` (0051) records a baseline. Distinguishing the
three baseline-dependent states (locally modified, remotely modified, conflict)
is therefore out of scope for this story and is delivered by 0051.

## Requirements

- `/list-work-items`: when `work.integration` is configured, display a
  colour-coded sync status label inline with each item's ID. Two net-new
  mechanics are in scope here, both required for the synced/unsynced labels and
  both forming the seams 0051 builds on:
  - extend the frontmatter scan to read the `work_item_id`/`id` value (today the
    ID is derived from the filename and this field is not read); read it through
    `work-item-read-field.sh`, which bridges both keys
  - render the label as an extensible per-item status slot (a status →
    label+colour lookup), not a hardcoded binary, so 0051 can add the
    baseline-dependent states without refactoring the rendering call site
- The two sync states this story renders, and the rule that classifies them:
  - **unsynced** — `work_item_id` matches the pure-numeric pattern `^[0-9]+$`;
    the item has never been pushed and does not exist in the remote system
  - **synced** — `work_item_id` is any other non-empty value (a remote-format
    key, e.g. `PROJ-0042` for Jira, `BLA-123` for Linear, `AbCd1234` for Trello,
    `atomic-innovation/accelerator#42` for GitHub Issues); the item is known to
    exist in the remote system. Without a `last-sync.json` baseline this is a
    *presumed-synced* signal (existence, not content parity)
  - The three baseline-dependent states (locally modified, remotely modified,
    conflict) are out of scope for this story — see "Deferred to 0051" below.
- `/create-work-item`: when `work.integration` is configured, present an
  interactive confirmation prompt after the work item is drafted (held in
  memory, before any local file is written), offering to push it to the remote
  - On acceptance: the remote creates the issue, returns its key, and the local
    file is written with the remote-allocated key as `work_item_id`; on push
    failure, offer at least one retry; if the offered retries are exhausted, fall
    back to saving locally with a numeric ID and inform the user to sync later
  - On decline: the local file is written with a local numeric ID as
    `work_item_id`
  - The local file is not written until the push succeeds, the user declines,
    or push has failed and fallback to local save is confirmed

## Acceptance Criteria

- [ ] Given `work.integration` is not configured, when `/list-work-items` is
  invoked, then no sync status label is shown
- [ ] Given `work.integration` is configured and no `last-sync.json` baseline
  exists, when `/list-work-items` is invoked, then each item shows exactly one of
  two labels — **synced** (remote-format `work_item_id`) or **unsynced** (numeric
  `work_item_id`) — and no baseline-dependent state is rendered
- [ ] Given `work.integration` is configured, when `/list-work-items` renders the
  labels, then the synced and unsynced labels differ in both text and colour, and
  the status is rendered through an extensible per-item status slot (additional
  states can be added without changing the rendering call site)
- [ ] Given `work.integration` is configured, when `/list-work-items` reads each
  item, then the `work_item_id`/`id` value is read from frontmatter (via
  `work-item-read-field.sh`), not derived from the filename
- [ ] Given a `work_item_id` value, when `/list-work-items` classifies it, then a
  value matching `^[0-9]+$` renders as unsynced and any other non-empty value
  renders as synced (e.g. `42` → unsynced; `PROJ-0042`, `BLA-123`, `AbCd1234`,
  `atomic-innovation/accelerator#42` → synced)
- [ ] Given `work.integration` is configured and `/create-work-item` finishes
  drafting, then an interactive confirmation prompt is shown offering to push to
  the remote, before any local file is written
- [ ] Given the user accepts the push offer and the push succeeds, then the
  local file is written exactly once with the remote-allocated key as
  `work_item_id`
- [ ] Given the user accepts the push offer and the push fails, then at least one
  retry is offered; if the offered retries are all exhausted, the local file is
  saved with a numeric `work_item_id` and the user is informed to run
  `/sync-work-items` later
- [ ] Given the user declines the push offer, then the local file is written
  with a numeric `work_item_id`
- [ ] Given a push attempt is in progress and has neither succeeded, been
  declined, nor reached a confirmed local fallback, then no file for the work
  item exists in the work directory

## Deferred to 0051

Specified here for continuity; owned and realised by `/sync-work-items` (story
0051), which produces the `last-sync.json` baseline these depend on. None are
verifiable within this story's deliverable.

- **locally modified** — local content has changed since last sync; remote has
  not
- **remotely modified** — remote content has changed since last sync; local has
  not
- **conflict** — both local and remote content have changed since last sync
- Rendering of the three labels above, the invariant that no two of the five
  states share an identical label+colour pairing across the full set, and the
  live per-item remote read they require — all depend on the `last-sync.json`
  baseline produced by `/sync-work-items`. When no baseline exists,
  `/list-work-items` shows only synced (presumed) and unsynced.

## Open Questions

- Colour scheme for the synced/unsynced labels — deferred to the implementing
  plan; must satisfy the distinct-text-and-colour constraint.
- Number of push retries offered before fallback in `/create-work-item` —
  deferred to the implementing plan; the acceptance criterion is written to be
  independent of the exact count.

## Dependencies

- Blocked by: 0046 (`work.integration` configuration)
- Blocks: 0051 — `/sync-work-items` reuses `/create-work-item`'s push-offer /
  confirmation UX (per-item and batch) established here
- External system: both skills couple to the configured remote tracker's API —
  `/create-work-item` writes (create issue) and the 0051-deferred sync-state
  derivation reads. The synced/unsynced subset delivered here needs no remote
  read; only the deferred states do.
- Integration create capability: `/create-work-item`'s push-on-accept requires
  at least one integration's create operation to be available (Jira is complete;
  Linear/Trello/GitHub are sibling stories 0048-0050). 0046 alone only declares
  the config key — it does not provide a create path.
- Data relationship with 0051 (deliberately **not** a blocking edge): the three
  deferred sync states consume the `last-sync.json` baseline that 0051 produces.
  This is a forward data dependency for those states only; it is not modelled as
  `blocked_by` because 0051 is itself `blocked_by` this story — adding the
  reverse edge would create a cycle. The deferred states are delivered as part
  of 0051.

## Assumptions

- The numeric-ID-as-unsynced convention (a numeric `work_item_id` means "never
  pushed") is the canonical signal for unsynced; a remote-format `work_item_id`
  is the canonical signal for synced. With no baseline, "synced" is presumed from
  ID format (existence), not computed from content parity.
- The three baseline-dependent states cannot be derived from the local and remote
  update timestamps alone: each is defined as "changed since last sync", which
  requires a persisted last-sync reference. Comparing the two current timestamps
  only reveals which side is newer and misclassifies — e.g. a freshly-synced item
  whose local file was written after the remote's `updated_at` would read as
  locally modified, and a true conflict would masquerade as a one-sided
  modification. The baseline is therefore a hard prerequisite, owned by 0051.
- Items in repos that have never run `/sync-work-items` have no `last-sync.json`
  and therefore show only synced (presumed) or unsynced.
- "Colour-coded" means terminal ANSI colour output; the specific colour scheme
  is left to the implementing plan.

## Technical Notes

**Size**: M — Two SKILL.md surfaces change. `/create-work-item` reuses the proven
`linear-create-flow.sh` write-back pattern but adds a retry-then-fallback state
machine holding the draft across attempts before a single local write;
`/list-work-items` needs net-new ANSI/colour rendering plus an extended
frontmatter scan to read the `work_item_id`/`id` shape. Bounded because only
synced/unsynced are derivable until 0051 delivers `last-sync.json`. Not L: no new
scripts or remote adapter work.

- The retry-then-fallback failure path means `/create-work-item` must hold the
  drafted work item in memory across at least two network attempts before
  deciding whether to write the local file.
- The implementing plan should define how many retries are attempted before the
  fallback is offered.
- **Reusable precedent for the push flow**: `/create-work-item`'s push-on-accept
  is mechanically the same as Linear's create-then-write-back. See
  `skills/integrations/linear/scripts/linear-create-flow.sh` — it rejects
  already-synced (remote-format) IDs (`LINEAR_IDENTIFIER_RE`, line 45), validates
  the returned identifier before any write (lines 198-205), then persists it with
  `config_set_frontmatter_field "$file" work_item_id "$identifier"` (line 209)
  behind a loud non-idempotent failure path `E_CREATE_WRITEBACK_FAILED`
  (lines 207-213). That write primitive lives in `scripts/config-common.sh`.
- **Gating read**: both skills must gate on `work.integration` via
  `config-read-work.sh integration` (validation at `config-read-work.sh:46-58`,
  allowed values in `config-defaults.sh:91-96`). Neither skill reads it today —
  add it alongside the existing `!`-preprocessor config reads in each SKILL.md.
- **`/list-work-items` gaps (both net-new)**: it has no ANSI/colour convention
  anywhere in `skills/work/`, and its frontmatter scan extracts only
  `title, kind, status, priority, tags, parent` — it does NOT read the
  `work_item_id`/`id` value, deriving the ID from the filename instead
  (`list-work-items/SKILL.md` Step 2). The sync label needs the frontmatter ID
  shape added to that scan.
- **Schema-key tension**: new work items use `id` (quoted string) as own-identity;
  this story and the Linear precedent write `work_item_id`. `work-item-read-field.sh`
  already bridges both keys transparently — read through it, don't assume one key.
- **`last-sync.json` is not read by this story.** Its location (story 0051 places
  it under `meta/integrations/<system>/`, while the live default for
  `paths.integrations` is `.accelerator/state/integrations`,
  `config-defaults.sh:59`) and its schema are 0051's to define. 0047 only ensures
  `/list-work-items`' status rendering is an extensible per-item slot so 0051 can
  plug the baseline-dependent states in without refactoring.
- **Two surfaces, one story (deliberate).** `/list-work-items` and
  `/create-work-item` share the numeric-vs-remote-format `work_item_id` convention
  and the same `work.integration` gate, and ship as one M-sized PR. They have no
  implementation dependency on each other, so if one stalls the other can land
  independently — but the shared convention and single config gate make them one
  coherent unit of "core skills become sync-aware".

## Drafting Notes

- Sync status is conceptually about content parity with the remote, not ID
  format. This was clarified during extraction; the epic's language was ID-centric
  but intent is content-centric. Note the scope decision below: the content-parity
  states are delivered by 0051, while this story renders only the two states that
  are derivable without a baseline.
- Five sync states defined: synced, unsynced, locally modified, remotely
  modified, conflict. The epic named only three (synced, unsynced, conflict);
  locally modified and remotely modified were added to cover the asymmetric
  cases, confirmed during extraction.
- **Scope decision (this review): the committed deliverable is synced/unsynced
  only** — the two states derivable without a baseline. With no baseline, a
  remote-format ID is *presumed synced* (existence, not content parity). The
  three baseline-dependent states are specified for continuity but delivered by
  0051, which owns `last-sync.json`. This keeps the dependency graph acyclic
  (0051 is `blocked_by` 0047, so 0047 must not depend on 0051) and keeps
  `/list-work-items` local-only and fast for the states it renders here.
- **Two forward-compat seams retained in 0047**, both needed for synced/unsynced
  anyway: (1) `/list-work-items` reads the `work_item_id`/`id` frontmatter shape;
  (2) the status label is an extensible per-item slot so 0051 adds states without
  refactoring the rendering.
- Why timestamps alone are insufficient for the deferred states: the three
  "since last sync" states need a persisted baseline, not just the two current
  update times — direct timestamp comparison misclassifies freshly-synced items
  and silently hides conflicts. This is why those states are owned by 0051, which
  records the baseline.
- Push failure UX: retry-first, then fall back to local save with numeric ID.
- Colour scheme for sync status labels left to implementing plan.

## References

- Source: `meta/work/0045-work-management-integration.md`
