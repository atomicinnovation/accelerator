---
type: "work-item"
id: "0285"
title: "Targeted Pull of Remote-Only Work Items and Resolution Normalisation"
date: "2026-09-08T17:43:59+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "done"
kind: "story"
priority: "medium"
parent: "work-item:0146"
relates_to: ["work-item:0257", "work-item:0229", "work-item:0255"]
tags: ["work", "sync", "targeting", "pull"]
last_updated: "2026-09-09T11:52:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0285: Targeted Pull of Remote-Only Work Items and Resolution Normalisation

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As an engineer maintaining work items locally, I want `work sync --target` to
pull a named work item that does not yet exist locally, so that I can import a
specific remote issue without running a full untargeted sync. This closes a gap
0257 explicitly — and wrongly — scoped out, and normalises the targeted
local/remote resolution and collision behaviour that gap forced. The
normalisation is not a separable second deliverable: introducing remote lookup
is exactly what forces the local-vs-remote resolution decision to be redefined,
so both arms share the same `resolve_targets` logic and ship together.

## Context

0257 added `work sync --target <id|external-id|path>` but resolved every target
against the local corpus only — the local corpus being all tracked work items
under the work directory. A remote id with no local counterpart aborts with
`RESOLVE_NOT_FOUND` (exit 3), and untracked discovery is suppressed whenever any
target is named. Pulling a remote-only item was the original point of targeting,
yet 0257 deliberately narrowed away from it and marked it out of scope — a wrong
call this item corrects. A remote-only issue can therefore only be imported by a
full sync today, which defeats the point of targeting a single item.

The same local-only resolution produced a dual-shape "suppressed remote match"
warning whenever a token both resolved as a local id and matched some
`external_id`. That framing is wrong for the ordinary synced item, where the
local file and the remote issue are one logical item carrying the same key — that
case should reconcile normally. A signal is warranted only for a genuine
collision, and that collision is reliably decidable only when both sides are in
the local corpus.

## Requirements

- Extend `work sync --target <remote-id>` so a token with no local file is looked
  up on the remote tracker by id and, if present, pulled into a new local file
  and reconciled — the inverse of today's exit-3 abort. The lookup is a direct
  by-id fetch, scoped to the named target; it never triggers the full
  untracked-discovery search.
- Route a targeted pull through the existing create-from-remote path, so the
  pulled item's local id, filename, and baseline entry are allocated identically
  to a full-sync discovery import (`external_id` set to the remote key). No new
  allocation scheme.
- Count a targeted pull-create as a pull for blast-radius bounds (`--max-pulls`),
  and show it under `--preview` with zero writes, exactly as an untracked import
  is today.
- Resolve each target against the local corpus first: a token that resolves to a
  single local file — by path, by local id, or by that file's own `external_id`
  — wins and selects that local item, except the local/local collision defined
  below, which aborts rather than selecting; consult the remote only for a token
  with no local match of any of those kinds.
- Reconcile the ordinary synced item — a token that resolves to a single local
  file, whether as that file's local id, its own `external_id`, or both (the
  `id == external_id` case) — under standard conflict resolution, with no
  collision warning.
- Detect the genuine local/local collision deterministically from the corpus — a
  token that is one local file's id and simultaneously a *different* local file's
  `external_id` — and abort it as a usage error (exit 2), naming both files and
  how to disambiguate. Never probe the remote for a locally-resolvable token.
- Keep `RESOLVE_NOT_FOUND` (exit 3) for a token that matches neither a local file
  nor a remote issue.
- Update the report lines and the `sync-work-items` skill rendering for the
  targeted pull-create and the revised collision semantics. Preserve full-sync
  behaviour, its report, its write set, and the per-item watermark semantics
  unchanged.

## Acceptance Criteria

- [ ] Given a remote issue `PP-999` with no local counterpart, when I run
      `work sync --target PP-999`, then a new local file is created for PP-999,
      populated from the remote issue, and reconciled, with no exit-3 abort.
- [ ] Given that pulled item, when it is created, then its local id, filename,
      and baseline entry match what a full-sync discovery import produces for the
      same issue.
- [ ] Given `--preview --target PP-999`, when I run it, then the would-create is
      shown, nothing is written, and the create counts against `--max-pulls`.
- [ ] Given a token that resolves to a single local file A — as A's local id, A's
      own `external_id`, or both — with no *different* file claiming the token,
      when I target it, then A reconciles under standard conflict resolution with
      no collision warning.
- [ ] Given a token that is local file A's id and a *different* local file B's
      `external_id`, when I target it, then the run aborts as a usage error
      (exit 2) naming A and B, determined from the local corpus with no remote
      call, and writes nothing.
- [ ] Given a token matching neither a local file nor a remote issue, when I
      target it, then the run aborts (exit 3) naming the token, with zero writes.
- [ ] Given several `--target`s mixing local ids, paths, and remote-only ids,
      when I run the sync, then local-id and path tokens reconcile their existing
      local file, remote-only ids create-and-reconcile a new local file, no
      non-targeted item is written, and discovery beyond the named set stays
      suppressed.
- [ ] Given a targeted pull-create of `PP-999` and, separately, a local/local
      collision, when each run finishes, then the report line and the
      `sync-work-items` skill rendering name the pull-create as a created pull and
      the collision as an exit-2 usage error identifying both files.
- [ ] Given a completed targeted pull-create of `PP-999`, when I run sync again,
      then `PP-999` is not re-pulled and its baseline/watermark
      (`local_synced_at`) has advanced.
- [ ] Given more targeted pull-creates than `--max-pulls N` allows, when I run
      the sync, then the whole run is refused with zero creates (fail-safe, no
      partial blast radius) and the bound breach is reported.
- [ ] Given no `--target`, when I run a full sync, then its report and item write
      set are byte-identical to before this change.

## Open Questions

- None outstanding — the drafting questions were resolved into the requirements
  above.

## Dependencies

- Blocked by: none. This item extends 0257's code directly — the exit-3 abort it
  inverts, the `resolve_targets` arm it modifies, the discovery suppression under
  `ItemSelection::Targeted`, and the pull-bound accounting all originate in 0257
  — but 0257 is already done (merged via PR #107), so that prerequisite has
  landed and no open blocker remains.
- Blocks: none identified.
- External: the remote tracker (Linear via `work.integration`). The new by-id
  `tracker.show(external_id)` pull path adds remote contact where today's code
  aborts without it, so a targeted pull now depends on the tracker being
  reachable and on `show` resolving a single issue by id — inherited from the
  full sync, mirroring 0257's external-dependency entry.
- Integration watch-out: 0285 modifies the shared engine pull/discovery path
  (discovery suppression, `resolve_targets`, pull-count accounting in `run.rs`) —
  the same path 0257 flagged for 0229 (pull scope) and 0255 (chunk merge). No
  delivery ordering is required, but whichever lands second should integrate
  against the other's changes rather than assuming disjoint code.

## Assumptions

- `tracker.show(external_id)` resolves a single issue by id, so a targeted pull
  needs no `search`; the by-id fetch keeps remote contact scoped to the named
  targets.
- Standard conflict resolution (conflict dossiers, dirty guard) applies unchanged
  once a targeted item is present locally.

## Technical Notes

- A remote-only pull can reuse `create_from_remote` (`apply.rs:270`), which
  already does `tracker.show(external_id)` then `author.author_from_remote(...)`
  to allocate the local file and baseline entry.
- `resolve_targets` (`cli/work-cli/src/sync.rs`) builds `external_id_index` over
  the local corpus; extend its no-local-match arm to attempt a remote `show`
  (targeted pull) rather than always returning `NoMatch`.
- Discovery stays gated off under `ItemSelection::Targeted`
  (`discovery_suppressed()`); the targeted pull uses by-id `show`, not
  `discover_untracked`.
- The collision signal derives entirely from the local `external_id_index`
  (`item.id != local_match.id`); make it exit 2 rather than a suppressed note.
- Pull-bound accounting already treats imports as pulls
  (`pulls = plan.pull_count() + untracked.len()`, `run.rs:822`); a targeted
  pull-create feeds the same count.
- The `corpus_carries` double-binding guard and the per-item watermark
  (`local_synced_at`) must extend to a newly-created targeted item.

## Drafting Notes

- Interpreted "same ID in both places" as the ordinary synced item (a local file
  whose `external_id` matches the token); confirmed with the requester.
- Chose a hard usage error (exit 2) for the genuine local/local collision over
  0257's local-wins-with-warning, per an explicit decision — safer than silently
  syncing one side.
- Resolved the id-allocation, preview/bounds, and exit-code questions into
  requirements by reusing the existing discovery-import path rather than
  inventing new mechanisms.
- Scoped as a story under epic 0146 (Work Item Synchronisation Enhancements), the
  same parent as 0257, since it directly extends 0257's targeted-sync feature.

## References

- Predecessor: `meta/work/0257-sync-specific-work-items.md`,
  `meta/plans/2026-09-06-0257-sync-specific-work-items.md`
- Parent epic: `meta/work/0146-work-item-sync-enhancements.md`
- Skill: `skills/work/sync-work-items/SKILL.md`
