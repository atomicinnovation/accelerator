---
id: "0047"
title: "Core Skills Sync Integration"
date: "2026-05-06T17:49:44+00:00"
author: Toby Clemson
kind: story
status: done
priority: high
parent: "work-item:0045"
tags: [work-management, integrations, sync, list-work-items, create-work-item]
type: work-item
schema_version: 1
last_updated: "2026-06-16T07:23:53+00:00"
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
non-empty `external_id`) and **unsynced** (no `external_id`, never pushed). The
three baseline-dependent states (locally modified, remotely modified, conflict)
require the `last-sync.json` baseline produced by `/sync-work-items` (story 0051)
and are realised there; `/list-work-items` is structured here so 0051 can extend
it without rework. On push acceptance, the remote allocates the issue
identifier, which is written as `external_id` in the local file; on decline, the
file is saved without an `external_id`.

The own-identity field `id` (a local work-item number, `(<project-code>-)?\d{4}`)
is unchanged and always allocated locally; `external_id` holds the remote
tracker's identifier and is the per-item local→remote mapping. The two may
coincide (Jira/Linear, where ID schemes align) or be independent (Trello, opaque
card IDs). No work item carries a `work_item_id` remote-key.

## Context

With `work.integration` configured (prerequisite story 0046), the core work
management skills need sync awareness. Throughout this story, "the remote" means
the single remote system declared by `work.integration` (jira, linear, trello,
or github-issues — see 0046); only one integration is active at a time. Two
skills are in scope: `/list-work-items` gains visibility into the sync state of
each work item relative to the remote, and `/create-work-item` gains a post-draft
push offer so newly created items can be pushed immediately.

The convention underpinning both is that the **presence of a non-empty
`external_id`** means the item exists in the remote system (synced), and its
**absence** means the item has never been pushed (unsynced). Without a last-sync
baseline, a present `external_id` is a *presumed synced* signal — the item is
known to exist remotely, but content parity cannot be computed until
`/sync-work-items` (0051) records a baseline. Distinguishing the three
baseline-dependent states (locally modified, remotely modified, conflict) is
therefore out of scope for this story and is delivered by 0051.

## Requirements

- `/list-work-items`: when `work.integration` is configured, display a
  colour-coded sync status label inline with each item's ID. Two net-new
  mechanics are in scope here, both required for the synced/unsynced labels and
  both forming the seams 0051 builds on:
  - extend the frontmatter scan to read the `external_id` value (today the ID is
    derived from the filename and this field is not read); read it through
    `work-item-read-field.sh`
  - render the label as an extensible per-item status slot (a status →
    label+colour lookup), not a hardcoded binary, so 0051 can add the
    baseline-dependent states without refactoring the rendering call site
- The two sync states this story renders, and the rule that classifies them
  (**presence-based**, robust under any `work.id_pattern`):
  - **unsynced** — no `external_id`, or an empty `external_id`; the item has
    never been pushed and does not exist in the remote system
  - **synced** — a non-empty `external_id` (the remote tracker's identifier,
    e.g. `PROJ-0042` for Jira, `BLA-123` for Linear, `AbCd1234` for Trello,
    `atomic-innovation/accelerator#42` for GitHub Issues); the item is known to
    exist in the remote system. Without a `last-sync.json` baseline this is a
    *presumed-synced* signal (existence, not content parity). The `external_id`
    may coincide with `id` (Jira/Linear) or be independent (Trello) — presence,
    not value, is what is tested
  - The three baseline-dependent states (locally modified, remotely modified,
    conflict) are out of scope for this story — see "Deferred to 0051" below.
- `/create-work-item`: when `work.integration` is configured, present an
  interactive confirmation prompt after the work item is drafted (held in
  memory, before any local file is written), offering to push it to the remote
  - On acceptance: the remote creates the issue, returns its identifier, and the
    local file is written with the remote-allocated identifier as `external_id`
    (the locally-allocated `id` is unchanged); on push failure, offer at least
    one retry; if the offered retries are exhausted, fall back to saving locally
    without an `external_id` and inform the user to sync later
  - On decline: the local file is written with `id` only and no `external_id`
  - The local file is not written until the push succeeds, the user declines,
    or push has failed and fallback to local save is confirmed

## Acceptance Criteria

- [ ] Given `work.integration` is not configured, when `/list-work-items` is
  invoked, then no sync status label is shown
- [ ] Given `work.integration` is configured and no `last-sync.json` baseline
  exists, when `/list-work-items` is invoked, then each item shows exactly one of
  two labels — **synced** (non-empty `external_id`) or **unsynced** (no
  `external_id`) — and no baseline-dependent state is rendered
- [ ] Given `work.integration` is configured, when `/list-work-items` renders the
  labels, then the synced and unsynced labels differ in both **glyph and text**
  (a markdown-native label, e.g. `🟢 synced` / `⚪ unsynced` — not raw ANSI, since
  the output is a markdown table emitted to the conversation, not a TTY), and the
  status is rendered through an extensible per-item status slot (additional states
  can be added without changing the rendering call site)
- [ ] Given `work.integration` is configured, when `/list-work-items` reads each
  item, then the `external_id` value is read from the item's frontmatter (not
  derived from the filename); the specific reader — folding it into the existing
  single-pass frontmatter scan, or via `work-item-read-field.sh` — is left to the
  implementing plan
- [ ] Given an item's frontmatter, when `/list-work-items` classifies it, then an
  item with no `external_id` (or an empty one) renders as unsynced and an item
  with a non-empty `external_id` renders as synced — independent of `id` shape
  (e.g. `id: "PROJ-0042"` with no `external_id` → unsynced; `external_id:
  "PROJ-0042"`, `"BLA-123"`, `"AbCd1234"`, `"atomic-innovation/accelerator#42"`
  → synced)
- [ ] Given `work.integration` is configured and `/create-work-item` finishes
  drafting, then an interactive confirmation prompt is shown offering to push to
  the remote, before any local file is written
- [ ] Given the user accepts the push offer and the push succeeds, then the
  local file is written exactly once with the remote-allocated identifier as
  `external_id` (and the locally-allocated `id` unchanged)
- [ ] Given the user accepts the push offer and the push fails, then at least one
  retry is offered; if the offered retries are all exhausted, the local file is
  saved with no `external_id` and the user is informed they can push it later via
  `/create-<tracker>-issue` (the standalone per-tracker create skill, which shares
  the same `external_id` contract; `/sync-work-items` is the future batch path,
  owned by 0051 and not yet built)
- [ ] Given the user declines the push offer, then the local file is written
  with `id` only and no `external_id`
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
  states share an identical label+glyph pairing across the full set, and the
  live per-item remote read they require — all depend on the `last-sync.json`
  baseline produced by `/sync-work-items`. When no baseline exists,
  `/list-work-items` shows only synced (presumed) and unsynced.

## Open Questions

- ~~Colour scheme for the synced/unsynced labels~~ — **Resolved by the
  implementing plan**: labels are markdown-native (glyph + distinct text, e.g.
  `🟢 synced` / `⚪ unsynced`), not ANSI, because the output is a markdown table
  rendered in the conversation rather than a TTY.
- ~~Number of push retries offered before fallback in `/create-work-item`~~ —
  **Resolved by the implementing plan**: exactly **one** retry, then local
  fallback (and only for failures provably before the remote create; post-create
  failures are not retried, to avoid duplicate issues).

## Dependencies

- Blocked by: 0046 (`work.integration` configuration)
- Blocks: 0051 — `/sync-work-items` reuses `/create-work-item`'s push-offer /
  confirmation UX (per-item and batch) established here
- External system: both skills couple to the configured remote tracker's API —
  `/create-work-item` writes (create issue) and the 0051-deferred sync-state
  derivation reads. The synced/unsynced subset delivered here needs no remote
  read; only the deferred states do.
- Integration create capability: `/create-work-item`'s push-on-accept dispatches
  to the configured tracker's create operation. **Jira and Linear are both built**
  and are wired here (via a new dispatcher); **Trello (0049) and GitHub Issues
  (0050) are unbuilt** and produce a clean "not available" message + local save.
  0046 alone only declares the config key — it does not provide a create path.
- Data relationship with 0051 (deliberately **not** a blocking edge): the three
  deferred sync states consume the `last-sync.json` baseline that 0051 produces.
  This is a forward data dependency for those states only; it is not modelled as
  `blocked_by` because 0051 is itself `blocked_by` this story — adding the
  reverse edge would create a cycle. The deferred states are delivered as part
  of 0051.

## Assumptions

- The presence-of-`external_id` convention is the canonical signal: an absent or
  empty `external_id` means "never pushed" (unsynced); a non-empty `external_id`
  means the item exists remotely (synced). With no baseline, "synced" is presumed
  from `external_id` existence, not computed from content parity. This is
  robust under any `work.id_pattern` (it does not depend on `id` format), and
  `external_id` may equal `id` or be independent of it. (Supersedes the earlier
  ID-format-based framing — see Drafting Notes.)
- The three baseline-dependent states cannot be derived from the local and remote
  update timestamps alone: each is defined as "changed since last sync", which
  requires a persisted last-sync reference. Comparing the two current timestamps
  only reveals which side is newer and misclassifies — e.g. a freshly-synced item
  whose local file was written after the remote's `updated_at` would read as
  locally modified, and a true conflict would masquerade as a one-sided
  modification. The baseline is therefore a hard prerequisite, owned by 0051.
- Items in repos that have never run `/sync-work-items` have no `last-sync.json`
  and therefore show only synced (presumed) or unsynced.
- The sync label is **markdown-native** (a glyph + distinct text, e.g.
  `🟢 synced` / `⚪ unsynced`), **not** terminal ANSI: `/list-work-items` output is
  a markdown table emitted into the conversation by the model, never written to a
  TTY, so ANSI escapes would render as literal text. (Supersedes the earlier
  "terminal ANSI colour output" framing — resolved during plan review; see
  Drafting Notes.)

## Technical Notes

**Size**: L (revised up from M during planning). Beyond the two core SKILL.md
surfaces, the `external_id` decision also requires harmonising the Linear and
Jira create flows onto one contract (content-in → identifier-out; user-facing
skills write `external_id`), a new push dispatcher, and an insert-if-missing
frontmatter helper. `/create-work-item` adds a retry-then-fallback state machine
holding the draft across attempts before a single local write; `/list-work-items`
needs net-new ANSI/colour rendering plus an extended frontmatter scan to read
`external_id`. Bounded because only synced/unsynced are derivable until 0051
delivers `last-sync.json`. See
`meta/plans/2026-06-15-0047-core-skills-sync-integration.md` for the four-phase
breakdown.

- The retry-then-fallback failure path means `/create-work-item` must hold the
  drafted work item in memory across at least two network attempts before
  deciding whether to write the local file.
- The implementing plan should define how many retries are attempted before the
  fallback is offered.
- **Reusable precedent for the push flow**: `/create-work-item`'s push-on-accept
  reuses Linear's create-then-validate-identifier sequence and its loud
  non-idempotent failure stance. See
  `skills/integrations/linear/scripts/linear-create-flow.sh` — it validates the
  returned identifier before any write (lines 197-205) and surfaces a writeback
  failure loudly via `E_CREATE_WRITEBACK_FAILED` (lines 207-213). This story
  refactors that flow to (a) write `external_id` instead of `work_item_id`, and
  (b) expose a no-file create-and-return mode the dispatcher can call (defer-write
  substitutes `external_id` into the in-memory frontmatter before the single
  Write, so the replace-only `config_set_frontmatter_field` in
  `scripts/config-common.sh` is not on the critical path; a new
  `config_upsert_frontmatter_field` insert-if-missing helper covers the
  file-first writeback in `/create-linear-issue` and `/create-jira-issue`).
- **Gating read**: both skills must gate on `work.integration` via
  `config-read-work.sh integration` (validation at `config-read-work.sh:46-58`,
  allowed values in `config-defaults.sh:91-96`). Neither skill reads it today —
  add it alongside the existing `!`-preprocessor config reads in each SKILL.md.
- **`/list-work-items` gaps (both net-new)**: it has no ANSI/colour convention
  anywhere in `skills/work/`, and its frontmatter scan extracts only
  `title, kind, status, priority, tags, parent` — it does NOT read `external_id`,
  deriving the displayed ID from the filename instead (`list-work-items/SKILL.md`
  Step 2). The sync label needs `external_id` added to that scan (read via
  `work-item-read-field.sh`); the filename remains the authoritative displayed ID.
- **`id` / `external_id` / `work_item_id` clarification**: `id` (quoted string)
  is the local own-identity and stays so. `external_id` (an existing
  omit-by-default field) holds the remote tracker's identifier. The legacy
  `work_item_id` own-identity key (bridged by `work-item-read-field.sh`) and the
  foreign-reference `work_item_id` in plan/research/PR templates are unrelated and
  untouched; only Linear's remote-key writeback (the third use of the name) is
  retired in favour of `external_id`.
- **`last-sync.json` is not read by this story.** Its location (story 0051 places
  it under `meta/integrations/<system>/`, while the live default for
  `paths.integrations` is `.accelerator/state/integrations`,
  `config-defaults.sh:59`) and its schema are 0051's to define. 0047 only ensures
  `/list-work-items`' status rendering is an extensible per-item slot so 0051 can
  plug the baseline-dependent states in without refactoring.
- **Two surfaces, one story (deliberate).** `/list-work-items` and
  `/create-work-item` share the `external_id`-presence convention and the same
  `work.integration` gate. They have no implementation dependency on each other,
  so if one stalls the other can land independently. The implementing plan splits
  the work into four independently mergeable phases (Linear convention refactor,
  Jira contract harmonisation, list-work-items labels, create-work-item push) —
  the shared convention and single config gate keep them one coherent unit of
  "core skills become sync-aware".

## Drafting Notes

- **Post-review reconciliation (2026-06-16):** after the implementing plan was
  reviewed (3 passes → APPROVE) and marked ready, this work item was reconciled to
  its decisions: (1) the sync label is markdown-native glyph + text, not terminal
  ANSI (AC #3, Assumptions, Open Questions, and the 0051 deferred-set invariant all
  updated from "colour" to "glyph"); (2) AC #4 no longer mandates a specific
  `external_id` reader — the plan prefers folding it into the single-pass scan,
  with `work-item-read-field.sh` as the alternative; (3) the retry-exhausted
  guidance points at `/create-<tracker>-issue` rather than the unbuilt
  `/sync-work-items` (AC #7); (4) both Open Questions (colour scheme, retry count)
  are now resolved by the plan. See
  `meta/reviews/plans/2026-06-15-0047-core-skills-sync-integration-review-1.md`.
- **Planning decision (2026-06-15, supersedes the original ID-format framing):**
  the remote tracker's identifier is stored in **`external_id`**, not
  `work_item_id`. `id` stays the local own-identity; sync classification is the
  **presence of a non-empty `external_id`** (not the `^[0-9]+$` format of `id`,
  which misclassifies project-coded local IDs like `PROJ-0042` as synced). This
  also retires Linear's `work_item_id` remote-key writeback and harmonises the
  Linear/Jira create flows onto one `external_id`-writing contract. Captured in
  the implementing plan; the AC, Requirements, Summary, and Assumptions above were
  reconciled to match.
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
  anyway: (1) `/list-work-items` reads `external_id` from frontmatter; (2) the
  status label is an extensible per-item slot so 0051 adds states without
  refactoring the rendering.
- Why timestamps alone are insufficient for the deferred states: the three
  "since last sync" states need a persisted baseline, not just the two current
  update times — direct timestamp comparison misclassifies freshly-synced items
  and silently hides conflicts. This is why those states are owned by 0051, which
  records the baseline.
- Push failure UX: retry-first, then fall back to local save with no
  `external_id` (the item simply reads as unsynced until the next push/sync).
- Colour scheme for sync status labels left to implementing plan.

## References

- Source: `meta/work/0045-work-management-integration.md`
- Research: `meta/research/codebase/2026-06-15-0047-core-skills-sync-integration.md`
- Implementing plan: `meta/plans/2026-06-15-0047-core-skills-sync-integration.md`
  (four-phase breakdown; records the `external_id` / presence-based-classification
  decision this work item was reconciled to)
