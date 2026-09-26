---
type: "work-item"
id: "0230"
title: "Tracker-Owned Work Item ID Generation"
date: "2026-08-30T14:35:09+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "in-progress"
kind: "story"
priority: "low"
parent: "work-item:0146"
blocked_by: ["work-item:0228"]
relates_to: ["work-item:0227", "work-item:0229", "work-item:0291"]
blocks: ["work-item:0302"]
tags: ["sync", "tracker", "id-generation", "drafts", "promotion"]
last_updated: "2026-09-26T00:47:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-760"
---
# 0230: Tracker-Owned Work Item ID Generation

**Kind**: Story
**Status**: In Progress
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a developer whose work items live in Jira or Linear, I want the tracker to own
work-item ID generation when configured, so that I refer to each work item by
one identifier — its tracker key — everywhere. Under
`work.id_pattern: "{tracker}"`, `work create --push` creates the remote issue
first and adopts its key as `id`; when the tracker does not confirm an issue,
or the item is created without pushing, it is written as a referenceable
`draft-` item that
`accelerator work sync` later promotes to its tracker ID. Sync pull and
`extract-work-items` adopt tracker keys the same way, and sync follows a key the
tracker changes. An `id` changes only through promotion, a tracker-side key
change, or an explicit re-key (0302); each records the retired ID in `aliases`,
and `work resolve` also accepts aliases and `external_id`. Items created before
`{tracker}` keep their IDs.

## Context

Today the local `id` is minted locally and differs from `external_id`, so every
item carries two identifiers — `0230` and `PP-760` — that developers translate
by hand across conversation, commits, branch names, and tracker links. Making
the tracker the source of truth means obtaining the tracker ID before writing the
local file, so `id` is set once by the tracker and changes only when the
tracker changes the key. That resolves the `id`-immutability boundary epic 0146
sets — provisional before first push, immutable once synced except when the
tracker itself changes the key — for the online path; drafts are the
provisional side.

This repo's corpus shows the gap: 294 items, all synced, all with a local `NNNN`
`id` distinct from their `external_id` (`0230` ↔ `PP-760`). The existing
`work create --push` path already creates the remote issue first and the local
file second, but mints `id` locally and falls back to an unsynced `local-save`
when the tracker is unreachable. Tracker-owned IDs remove local minting from the
online path and replace `local-save` with a draft carrying a provisional ID whose
shape can never equal a tracker key.

`work create --push` ends in one of three outcomes: write-once (the issue was
created and the file written), `local-save` (the tracker was unavailable after
retry; no issue exists), or `loud-terminal`. `loud-terminal` covers two
cases, named here: a tracker-error `loud-terminal` (a terminal or unrecognised
tracker error; an issue may exist, its key unknown), and a write-failed
`loud-terminal` (the issue was created, so its key is known, but the local
write failed). Each create
attempt is recorded in a `pending_push` marker — a gitignored, machine-local
file holding the request fingerprint and, once known, the created key — so a
retry never duplicates an issue. The marker is `attempted` from before the
create until its outcome is known, and `created` once the tracker returns a
key; `local-save` removes it. New issues are created in the tracker's
creation-home entity: the Jira project or Linear team the scope key resolves to
(0228).

## Requirements

**Configuration**

- `work.id_pattern: "{tracker}"` enables tracker-owned IDs.
- `{tracker}` must be the pattern's only token and requires `work.integration` to
  be `jira` or `linear`; any other combination fails configuration validation.

**Online creation**

- When the tracker is reachable, `work create --push` adopts the created issue's
  key as the item's `id`, filename prefix, and H1; `id` equals `external_id`. The
  issue is minted into the tracker's creation-home entity.

**Drafts**

- A creation that does not confirm a tracker issue writes a draft to
  `meta/work/drafts/`: `work create` without `--push`; `local-save` (no issue
  was created); or a tracker-error `loud-terminal` (an `attempted` marker is
  left).
- On a write-failed `loud-terminal` (a `created` marker is left), the write is
  retried once as the tracker-keyed item. If
  the retry also fails, nothing is written locally, and `work sync` adopts the
  marker's issue when it pulls it.
- A draft's `id` is `draft-` followed by 6 lowercase Crockford base32 characters.
  A draw whose suffix is all digits, or which collides with an existing `id` or
  alias in the corpus, is rejected and redrawn. Requiring a letter keeps a
  draft ID out of the `<KEY>-<digits>` shape every tracker key takes, so
  `draft-123456` can never match a key such as `DRAFT-123456` under
  case-insensitive resolution.
- A draft is written to `meta/work/drafts/<draft-id>-<slug>.md` with the H1
  `# <draft-id>: <title>`.
- Drafts are first-class: valid typed-link targets (`work-item:draft-…`),
  resolved by `work resolve`, and listed by `list-work-items` (which renders
  `accelerator work list`) with a draft indicator: `accelerator work list`
  shows `draft` in the Sync column for any item whose `id` has the `draft-`
  prefix, and renders that column whenever the corpus holds a draft.

**Promotion**

- Promotion logic lives in `accelerator work sync`, which promotes drafts by
  default; `--no-promote` skips them. `accelerator work promote <draft-id>` is a
  thin entry point onto the same logic, scoped to one draft.
- Promotion creates the remote issue, moves the file to
  `meta/work/<tracker-id>-<slug>.md`, sets `id` and the H1, and appends the draft
  ID to `aliases`.
- Promotion replaces every occurrence of the draft ID within `meta/` — typed links
  and prose — with the tracker ID, except the promoted item's own `aliases`
  entry. Only whole tokens are replaced: an occurrence adjacent to a letter,
  digit, or hyphen is left alone. Matching ignores case, as `work resolve`
  does; the replacement is written in the new ID's canonical case. Nothing
  outside `meta/` is rewritten.
- Under `{tracker}`, `work create --push` mints a draft ID before the create
  attempt and names the `pending_push` marker after it, not after the slug, so
  retitling a draft or sharing a title with another cannot orphan or clobber
  its marker. The draft ID becomes the item's `id` only when the outcome is a
  draft; a `created` marker whose local write failed keeps the name until
  sync adopts its issue.
- Promotion honours the draft's `pending_push` marker:
  - a `created` marker's key is adopted as `id`, with no new remote issue,
    even if the draft was edited after the marker was written;
  - an `attempted` marker (outcome unknown) stops that draft's promotion,
    naming it and the possible remote duplicate; other drafts still promote.
- Promotion creates the remote issue through the same per-item create logic as
  `work create --push` — its retries, outcomes, and `pending_push` marker
  handling — before changing anything locally. A draft whose promotion does
  not complete is left unchanged (file, `id`, and `meta/` references), and the
  rest of the run continues. The create outcome decides the marker left
  behind: none for `local-save`, an `attempted` marker for a tracker-error
  `loud-terminal`, and a `created` marker when the issue was created but the
  local ID retirement failed, so the next promotion adopts it.
- `work sync` reports each draft whose promotion was attempted but did not
  complete, with its reason, and exits non-zero when there is any. Drafts
  skipped by `--no-promote` do not affect the exit code.
- `work promote <draft-id> --adopt <tracker-id>` promotes a stopped draft onto
  an existing remote issue without creating one; `--create` accepts the
  duplicate risk and creates a new issue. Either clears the marker.

**Identity**

- An `id` changes only through draft promotion, a tracker-side key change, or
  an explicit re-key. Re-keying is delivered by 0302; this story supplies
  `aliases`, alias resolution, and ID retirement for it.
- ID retirement is the one operation that replaces an item's `id`: it moves
  the file to the new ID's filename, sets `id` and the H1, appends the old ID
  to `aliases`, and rewrites the old ID within `meta/` — typed links always,
  and prose, whole tokens only, when the old ID has a distinctive shape (a
  `draft-` token or a `<KEY>-<number>` key). A bare numeric ID has only its
  typed links rewritten, since it is indistinguishable from a number in prose.
  It also rewrites the `work_item_id` field of work-item reviews whose value
  equals the old ID. ID retirement is all-or-nothing: if any step fails, the
  item's file and every `meta/` reference are left as they were before it
  began.
- ID retirement refuses when the new ID is already another item's `id` or
  alias: it exits non-zero naming both items, suggests the reconciliation step
  (remove or retire the duplicate, then retry), and changes nothing. A draft
  so refused keeps its marker; sync reports the collision as the draft's
  reason for staying unpromoted.
- Under every `id_pattern`, when `work sync` finds that an item's remote issue
  now has a different key — moved between Linear teams or Jira projects — the
  item's `external_id` takes the new key. Under `{tracker}`, an item whose `id`
  equals its old `external_id` also has the old key retired: `id`, filename
  prefix, and H1 take the new key.
- When the tracker cannot find an item's remote issue under its stored key,
  sync leaves the item unchanged and warns that its remote issue could not be
  found under that key.
- `aliases: []` is a work-item frontmatter field listing retired IDs;
  `work resolve` falls back to it.
- `work resolve` also matches on `external_id`, case-insensitively on every
  field. A token matching different items through different fields is an
  error naming every match and the field it matched; `work resolve` never
  guesses. A token matching one item through several fields resolves to it.

**Legacy corpus**

- Items whose IDs were minted under an earlier pattern keep them. Pushing an
  unsynced legacy item sets only `external_id`.
- `work resolve` and `corpus frontmatter validate` accept legacy ID shapes
  alongside `{tracker}`.

**Sync pull**

- Under `{tracker}`, a remote-only issue pulled into the corpus takes its
  `external_id` as `id`; no local number is allocated.

**Batch extraction**

- `extract-work-items` makes one push offer for the approved batch. Declining
  writes every item as a draft through `work create` without `--push`, each
  child's local `parent` naming its parent's draft ID.
- On accept, it attempts a best-effort remote create per item, reusing the
  per-item `work create --push` outcomes, in topological `parent` order, so each
  child's local `parent` names its parent's tracker ID. No remote parent is set.
- Items whose create confirms no issue — `local-save`, or a tracker-error
  `loud-terminal` — become drafts. Their children are still created remotely,
  and carry `parent: "work-item:<draft-id>"` locally until promotion rewrites
  it.
- An item whose create ends in a write-failed `loud-terminal` is not a draft;
  its children carry `parent: "work-item:<tracker-id>"`, which resolves once
  sync pulls the parent.
- A cycle in the approved batch's `parent` links stops the batch before any
  create, naming the items in the cycle.
- The skill ends with a per-item outcome summary using exactly these outcomes:
  `created <tracker-id>`, `created <tracker-id> — local write failed`,
  `draft — push declined`, `draft — tracker unreachable` (`local-save`), and
  `draft — remote issue may exist` (a tracker-error `loud-terminal`); a
  write-failed `loud-terminal` reports `created <tracker-id> — local write
  failed`.

**`create-work-item`**

- Under `{tracker}`, the push offer's decline option reads "No, save as draft".
  Declining, and every fall-back to saving locally when the push preview
  (`work create --push --dry-run`) fails,
  call `work create` without `--push`, so each yields a draft.

**Out of scope**

- Setting remote parent-child links — at create, on promotion, or on sync
  (0291).
- Pushing `blocks`, `blocked_by`, and `relates_to` to the tracker.
- Trackers other than Jira and Linear.
- `aliases` on artifact types other than work items — no other type's `id`
  changes.
- Showing drafts in the visualiser, which scans `meta/work/` without
  descending into `meta/work/drafts/`.
- Re-keying existing IDs after an `id_pattern` change (0302).

## Acceptance Criteria

- [ ] Given `id_pattern: "{tracker}"`, a reachable tracker, and each of
      `integration: jira` and `integration: linear`, when `work create --push`
      runs, then the written item's `id`, filename prefix, and H1 all equal its
      `external_id`.
- [ ] Given `{tracker}` and a creation-home of Linear team `ENG`, when
      `work create --push` runs, then the issue is created in `ENG`.
- [ ] Given `{tracker}` and a creation-home of Jira project `OPS`, when
      `work create --push` runs, then the issue is created in `OPS`.
- [ ] Given `{tracker}` with `work.integration` of `jira` or `linear`, when
      configuration is validated, then it passes.
- [ ] Given `id_pattern: "{tracker}-{number:04d}"`, when configuration is
      validated, then it fails stating that `{tracker}` must be the only token
      in `work.id_pattern`.
- [ ] Given `id_pattern: "{tracker}-{number:04d}"`, when
      `accelerator config validate` runs, then it fails with the same message
      as load-time validation.
- [ ] Given `{tracker}` with `work.integration` unset, `trello`, or
      `github-issues`, when configuration is validated, then it fails stating
      that `{tracker}` requires `work.integration` to be `jira` or `linear`.
- [ ] Given `{tracker}`, when `create-work-item` offers to push, then its
      decline option reads "No, save as draft".
- [ ] Given `{tracker}` and a push preview that fails, when `create-work-item`
      saves locally, then the item is a `draft-` item in `meta/work/drafts/`.
- [ ] Given `{tracker}` and a create whose outcome is `local-save`, a
      tracker-error `loud-terminal`, or `work create` without `--push`, when
      the item is written, then it lands at `meta/work/drafts/<id>-<slug>.md`
      with the H1 `# <id>: <title>`, an `id` matching
      `^draft-[0-9a-hjkmnp-tv-z]{6}$`, a suffix that is not all digits, and no
      `external_id`; for the tracker-error `loud-terminal` an `attempted` marker
      named after the draft ID exists, and for
      `local-save` and `work create` without `--push` no marker exists.
- [ ] Given `{tracker}` and a create whose issue `PP-900` is created but whose
      local write and its retry both fail, when the create completes, then no
      file is written, a `created` marker records `PP-900`, and the next
      `work sync` writes `meta/work/PP-900-<slug>.md` with `id` `PP-900` and
      no `pending_push` marker for `PP-900` remains.
- [ ] Given `{tracker}` and a create whose issue `PP-900` is created, whose first
      local write fails, and whose retry succeeds, when the create completes,
      then `meta/work/PP-900-<slug>.md` exists with `id` `PP-900`, no marker
      remains, and the outcome is write-once.
- [ ] Given draws yielding `draft-123456` then `draft-k7mq3x`, when a draft is
      written, then its `id` is `draft-k7mq3x`.
- [ ] Given an existing alias `draft-k7mq3x` and draws yielding `draft-k7mq3x`
      then `draft-p2wq9r`, when a draft is written, then its `id` is
      `draft-p2wq9r`.
- [ ] Given an existing draft `draft-k7mq3x` and draws yielding `draft-k7mq3x`
      then `draft-p2wq9r`, when a draft is written, then its `id` is
      `draft-p2wq9r` and the existing draft is unchanged.
- [ ] Given a draft `draft-k7mq3x`, when another item sets
      `parent: "work-item:draft-k7mq3x"`, then `corpus frontmatter validate`
      passes and `work resolve draft-k7mq3x` returns the draft's path.
- [ ] Given synced items and one draft, when `accelerator work list` runs, then
      a Sync column is shown and reads `draft` for the draft only; with the
      draft removed and no sync baseline, the column is absent.
- [ ] Given a draft referenced by a typed link and in the prose of a `meta/` plan,
      when `work sync` runs without `--no-promote`, then the remote issue is
      created, the file moves to `meta/work/<tracker-id>-<slug>.md` with `id` and
      H1 set to the tracker ID, `aliases` contains the draft ID, the typed link
      reads `work-item:<tracker-id>`, the plan's prose names the tracker ID where
      it named the draft ID, and no whole-token occurrence of the draft ID
      remains in `meta/` apart from the promoted item's `aliases` entry.
- [ ] Given `meta/` prose containing `draft-k7mq3x-notes`, `adraft-k7mq3x`,
      `(draft-k7mq3x)`, and `Draft-K7MQ3X`, when `draft-k7mq3x` is promoted to
      `PP-900`, then the first two are unchanged, the third reads `(PP-900)`,
      and the fourth reads `PP-900`.
- [ ] Given a draft ID named in a file outside `meta/`, when the draft is
      promoted, then that file is byte-identical.
- [ ] Given `{tracker}`, item `PP-76` moved to `ENG-42`, and `meta/` prose
      naming both `PP-76` and `PP-760`, when `work sync` runs, then `PP-76`
      reads `ENG-42` and `PP-760` is unchanged.
- [ ] Given drafts and other pending changes, when `work sync --no-promote` runs,
      then the drafts are unchanged, the other changes sync, and sync exits 0.
- [ ] Given two drafts, when `work promote` runs on one, then only that draft is
      promoted, with the same result as a sync promotion.
- [ ] Given a draft whose `pending_push` marker records a created issue, and
      whose title and body were edited after the marker was written, when it
      is promoted, then no new remote issue is created and the recorded key
      becomes its `id`.
- [ ] Given a tracker-error `loud-terminal` draft with an `attempted` marker
      that is then retitled, and a second draft with its old title, when
      `work sync` runs, then the retitled draft is still stopped by its marker
      and the second draft is promoted.
- [ ] Given two drafts, one with an `attempted` `pending_push` marker, when
      `work sync` runs, then the other draft is promoted, the `attempted` draft
      is unchanged and no remote issue is created for it, the output names it
      as possibly duplicated remotely, and sync exits non-zero.
- [ ] Given a draft with an `attempted` marker and an existing remote issue
      `PP-900`, when `work promote <draft-id> --adopt PP-900` runs, then no
      remote issue is created, `id` is `PP-900`, and the marker is removed;
      when `--create` runs instead, a new remote issue is created, its key
      becomes `id`, and the marker is removed.
- [ ] Given a draft with an `attempted` marker, when
      `work promote <draft-id> --adopt PP-999` runs and no remote issue
      `PP-999` exists, then it exits non-zero and the draft and its marker are
      unchanged.
- [ ] Given two drafts and a tracker unreachable for the first create, when
      `work sync` runs, then the first draft's file, `id`, and `meta/`
      references are unchanged and no marker exists for it, the second is
      promoted, the output names the first as not promoted because the tracker
      was unreachable, and sync exits non-zero.
- [ ] Given a draft whose promotion create returns a tracker-error
      `loud-terminal`, when `work sync` promotes it, then the draft is
      unchanged, carries an `attempted` marker, and sync exits non-zero.
- [ ] Given a draft whose remote issue `PP-900` is created but whose local ID
      retirement fails after moving the file and rewriting some `meta/`
      references, when promotion completes, then the draft's file, `id`, and
      every `meta/` reference match their state before promotion, a `created`
      marker records `PP-900`, and sync exits non-zero naming the draft; when
      it is next promoted, no new remote issue is created and `PP-900` becomes
      its `id`.
- [ ] Given a stopped draft and an existing local item `PP-900`, when
      `work promote <draft-id> --adopt PP-900` runs, then it exits non-zero
      naming both and suggesting the duplicate be removed before retrying, and
      the draft, its marker, and `PP-900` are unchanged.
- [ ] Given `{tracker}`, item `PP-76` whose issue moved to `ENG-42`, and an
      existing item with `ENG-42` in `aliases`, when `work sync` runs, then
      `PP-76` is unchanged, the output names both items, and sync exits
      non-zero.
- [ ] Given a promoted draft, when `work resolve` runs on its former draft ID,
      then it returns the promoted item's path.
- [ ] Given an item promoted from `draft-k7mq3x`, when
      `work resolve DRAFT-K7MQ3X` runs, then it returns the promoted item's
      path.
- [ ] Given ID retirement of `0230` to `PP-760`, when it completes, then
      `work-item:0230` in `meta/` reads `work-item:PP-760`, a prose `0230` is
      unchanged, and a work-item review's `work_item_id: "0230"` reads
      `work_item_id: "PP-760"`.
- [ ] Given legacy item `0230` with `external_id: "PP-760"`, when
      `work resolve PP-760` or `work resolve pp-760` runs, then it returns that
      item's path.
- [ ] Given `{tracker}` and item `PP-900` whose `id` and `external_id` are both
      `PP-900`, when `work resolve PP-900` or `work resolve pp-900` runs, then
      it exits 0 and returns that item's path.
- [ ] Given item A with `external_id: "ENG-42"` and item B with `ENG-42` in
      `aliases`, when `work resolve ENG-42` runs, then it exits non-zero naming
      both items and the fields they matched.
- [ ] Given `{tracker}`, unsynced items `0042` and `ACC-0042`, and synced item
      `PP-760` whose remote key is unchanged, when each of `work sync`,
      `work create --push`, and `extract-work-items` runs, then every one of
      their `id`s is unchanged.
- [ ] Given `{tracker}` and unsynced legacy item `0042`, when it is pushed, then
      its `id` is still `0042` and its `external_id` is set.
- [ ] Given `{tracker}` and item `PP-760` referenced by a typed link and in the
      prose of a `meta/` plan, whose Linear issue has moved to another team as
      `ENG-42`, when `work sync` runs, then the file moves to
      `meta/work/ENG-42-<slug>.md` with `id`, `external_id`, and H1 set to
      `ENG-42`, `aliases` contains `PP-760`, the typed link and prose name
      `ENG-42`, and `work resolve PP-760` returns the item's path.
- [ ] Given `integration: jira`, `{tracker}`, and item `PP-76` whose issue moved
      to project `OPS` as `OPS-5`, when `work sync` runs, then `id`,
      `external_id`, filename prefix, and H1 are `OPS-5` and `aliases` contains
      `PP-76`.
- [ ] Given `{tracker}` and item `PP-76` whose remote issue the tracker cannot
      find under `PP-76`, when `work sync` runs, then the item is unchanged and
      the output warns that `PP-76` could not be found.
- [ ] Given `id_pattern: "{number:04d}"` and item `0230` with
      `external_id: "PP-760"` whose issue has moved to `ENG-42`, when
      `work sync` runs, then `external_id` is `ENG-42` and `id` is still `0230`.
- [ ] Given `{tracker}` and legacy item `0230` with `external_id: "PP-760"`
      whose issue has moved to `ENG-42`, when `work sync` runs, then
      `external_id` is `ENG-42` and `id` is still `0230`.
- [ ] Given `{tracker}` and a corpus of legacy `NNNN` items, when
      `corpus frontmatter validate` runs over them, then it passes.
- [ ] Given `{tracker}`, creation-home Linear team `ENG`, and a pull scope
      broadened to team `OPS`, when `work sync` pulls remote-only issue
      `OPS-7`, then its local `id` is `OPS-7`.
- [ ] Given `{tracker}` and a remote-only issue, when `work sync` pulls it, then
      the local `id` equals its `external_id`, and `work next-number` returns
      the same value before and after the pull.
- [ ] Given `{tracker}`, a reachable tracker, and `extract-work-items`
      approving an epic and two children with the push offer accepted, when
      the batch is written, then all three exist remotely, each child's local
      `parent` names the epic's tracker ID, and no remote parent is set.
- [ ] Given `{tracker}` and an approved batch whose items A and B name each
      other as `parent`, when the push offer is accepted, then no remote issue
      is created, no file is written, and the output names A and B as a cycle.
- [ ] Given `{tracker}` and `extract-work-items` approving an epic and two
      children with the push offer declined, when the batch is written, then
      all three are drafts, no remote issue is created, each child's local
      `parent` names the epic's draft ID, and the summary gives each
      `draft — push declined`.
- [ ] Given `{tracker}` and `extract-work-items` approving an epic and two
      children with the push offer accepted, and the epic's create returning
      `local-save`, when the batch completes, then the epic is a draft, both
      children exist remotely and carry `parent: "work-item:<epic draft id>"`
      locally, and promoting the epic rewrites both children's local `parent`
      to the epic's tracker ID.
- [ ] Given `{tracker}` and an accepted batch of four whose creates yield
      success (`PP-901`), `local-save`, a tracker error, and a local write
      failure after creating `PP-904`, when the batch completes, then the
      summary reads, in order,
      `created PP-901`, `draft — tracker unreachable`,
      `draft — remote issue may exist`, and
      `created PP-904 — local write failed`.

## Open Questions

None.

## Dependencies

Partial blocks are recorded as `relates_to` in frontmatter, where `blocks`
would gate the whole item; the gated portion is named here.

- Blocked by: 0228 (Layered Configuration Key Model, done) — 0230 builds on the
  scope-key / `work.key` separation it establishes.
- Blocks: the `{tracker}`-validation portion of 0227 (`accelerator config
  validate` Command) — `config validate` must carry the only-token and
  jira/linear rules so command-time and load-time validation agree. This gates
  only that portion of 0227, not the whole. If 0227 has shipped when this
  story is implemented, this story adds the `{tracker}` rules to
  `config validate`; otherwise 0227 includes them.
- Blocks: the draft-deferral portion of 0291 (Parent-Child Relationship
  Synchronisation) — its deferral of edges to `draft-` parents needs the draft
  form and promotion. This gates only that portion of 0291. 0291 owns remote
  parent-child linkage; this story leaves the local `parent` of every item it
  creates or promotes naming a tracker ID or a draft ID, ready for 0291 to
  push. Until 0291 ships, remote hierarchies of those items stay unlinked.
  Whichever of 0230 and 0291 ships first builds the parents-first ordering
  and cycle detection; the other reuses it.
- Relates to: 0229 (Per-Tracker Pull Scope Configuration) — no ordering
  constraint; under a broadened pull scope, remote-only issues from teams or
  projects beyond the creation-home are adopted with their own keys.
- Blocks: 0302 (Work Item Re-Key Command) — builds on `aliases`, alias
  resolution, the draft form, ID retirement, and tracker-side key-change
  detection.

## Assumptions

- Jira and Linear issue identifiers always end in `-<issue number>`; the draft
  ID's collision guarantee rests only on this suffix, not on the key's
  character set. Linear team keys may contain or start with digits, and Linear
  publishes no key grammar, but the suffix is always the team's positive issue
  number.
- `pending_push` markers are gitignored and machine-local. A tracker-error
  `loud-terminal` draft promoted from another checkout sees no marker and
  creates a new issue, duplicating any the original attempt created.
- Both trackers still resolve a moved issue by its old key, which is how sync
  finds the issue to detect the new key. Linear redirects old identifiers and
  exposes `previousIdentifiers`. Jira Cloud's get-issue endpoint answers an old
  key with the moved issue directly — a 200 carrying the new `key`, not a
  redirect — after both project moves and project-key renames. Verified
  against Linear's docs (linear.app/docs/editing-issues) and Atlassian's Cloud
  REST docs (developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issues/);
  0302's `external_id` refresh relies on the same behaviour.
- Six base32 characters (~10⁹ values) with redraw on collision suffice for any
  realistic corpus.

## Technical Notes

- The online path extends `execute_push` in `cli/work-cli/src/create.rs`; the
  draft path replaces its unsynced write on `LocalSave` / `LoudTerminal`.
- Sync pull allocates a local ID in `cli/work-adapters/src/sync/create.rs`; under
  `{tracker}` it adopts `external_id` instead.
- `extract-work-items` allocates through `work next-number --count N`; under
  `{tracker}` that is replaced by per-item creates.
- A tracker-side key change is detected by comparing the key an issue is
  fetched by with the key the tracker returns.
- Markers live at
  `<paths.integrations>/<integration>/pending-push/<marker-name>.json`; today
  the marker name is the slug, which suits a marker living for one create call
  but not a draft's marker, which lives until promotion.
- Promotion, a tracker-side key change, and 0302's re-key all use ID
  retirement, differing only in where the new ID comes from.
- The corpus readers that must handle `meta/work/drafts/` and changing IDs are
  `work resolve`, `accelerator work list`, `corpus frontmatter validate`, and
  `work sync`. The visualiser also reads `meta/work/`, but scans it flat.
- Under tracker-owned IDs `work.key` is unused: synced items take their key from
  the tracker and drafts never carry its prefix.

## Drafting Notes

- Promotion defaults on in `work sync`: an unpromoted draft is the exception, so
  opting out (`--no-promote`) is safer than opting in.
- Sync follows tracker-side key changes rather than deferring them to the
  re-key command, so `id` equal to `external_id` holds for every tracker-owned
  item without manual steps. If renaming a Linear team key rewrites its issues'
  identifiers, the next sync retires the key of every synced item of that
  team; that is intended, not a risk to guard against.
- Following `external_id` across tracker-side key changes, resolving by
  `external_id` and `aliases`, and ID retirement apply under every
  `id_pattern`, yet stay in this story rather than a prerequisite: they are
  the identity foundations tracker-owned IDs rest on, and are kept with them
  by decision.
- Rewriting prose inside `meta/` replaces whole tokens only, since a retired
  tracker key such as `PP-76` is a prefix of others such as `PP-760`. Prose
  that names the old key historically is rewritten too; the old key still
  resolves through `aliases`.
- Children of a failed parent are still created, to persist as much of a batch
  with the tracker as possible rather than drafting a whole subtree; 0291 links
  them remotely once the parent is promoted.

## References

- Parent: 0146 — Work Item Synchronisation Enhancements
- Blocked by: 0228 — Layered Configuration Key Model
- Blocks (portion): 0227 — `accelerator config validate` Command
- Blocks (portion): 0291 — Parent-Child Relationship Synchronisation
- Related: 0229 — Per-Tracker Pull Scope Configuration
- Blocks: 0302 — Work Item Re-Key Command
- Code: `cli/work-cli/src/create.rs`, `cli/tracker/src/lib.rs`,
  `cli/work-adapters/src/sync/create.rs`,
  `cli/work-adapters/src/sync/pending_push.rs`
