---
type: "work-item"
id: "0257"
title: "Sync Specific Work Items"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "story"
priority: "medium"
parent: "work-item:0146"
relates_to: ["work-item:0229", "work-item:0213", "work-item:0255"]
external_id: "PP-787"
tags: ["work", "sync"]
last_updated: "2026-09-06T08:27:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0257: Sync Specific Work Items

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As an engineer maintaining work items locally, I want `/sync-work-items` to
reconcile one or more named work items, so that I can push or pull a specific
few without the cost and blast radius of a full sync.

## Context

Captured in the further-ideas backlog. A full sync does a bulk remote read,
classifies every local item, and discovers untracked remote issues to pull.
When only a few known items need reconciling, that is slow and widens the blast
radius unnecessarily. A targeted sync narrows the run to exactly the named
items and skips discovery of items the user did not name. Because targeting
skips discovery, a targeted pull reaches only items already tracked locally;
pulling a brand-new remote-only issue with no local counterpart remains the job
of a full sync.

## Requirements

- The skill accepts one or more repeatable `--target` options, each naming a
  single work item. The skill parses them and passes the resulting target set to
  the `accelerator work sync` engine, which owns the set; the skill stays
  confined to parsing, gating, and rendering.
- A target may be a local work-item id, a remote tracker id (the item's
  `external_id`, e.g. `PP-787`), or a path; the engine resolves the local file
  regardless of which form is given.
- Local-id and path targets resolve through the existing `accelerator work
  resolve` (paths, full IDs, bare and short numbers); remote-id targets resolve
  by matching `external_id` across all local work items in the work directory.
  When a token is both a valid
  local-id shape and a match for some item's `external_id`, the local-id
  interpretation wins and the remote-id match is reported as suppressed.
- Only the named items are reconciled, bidirectionally by default, with per-item
  semantics identical to a full sync: push, pull, conflict dossiers, the
  dirty-overwrite guard, blast-radius bounds, and baseline-last resumability.
  These per-item behaviours are defined by the existing `accelerator work sync`
  engine (see epic 0146) and are reused unchanged, not redefined here.
- For a named unsynced target (no `external_id`), the remote issue is created and
  `external_id` is written back, exactly as the full sync's `pushed-unsynced`
  path. A named target whose local and remote sides both exist but are unlinked
  is handled exactly as a full sync would, with no targeted-run special-casing.
- Untracked-remote discovery does not run when targets are named.
- Targeting composes with the existing flags: `--push-only`/`--pull-only`,
  `--preview`, `--max-pulls`/`--max-pushes`, and `--resolve` (the existing
  per-conflict resolution flag `--resolve <id>=<remote|local|skip>`, which is
  unrelated to target resolution).
- If any target fails to resolve — invalid, or no match across both local ids and
  remote ids — the run aborts before any side effect, naming the offending
  target(s), with zero writes. A token matching both a local id and some
  `external_id` is not a failure: local wins per the precedence above.
- A path target that resolves to a file outside the work directory is rejected:
  the run aborts before any side effect, naming the offending path, with zero
  writes. Sync is confined to managed work items under the work directory.
- The full-sync (whole-set) behaviour remains available when no target is given.

## Acceptance Criteria

For every criterion below whose outcome is stated "as in a full sync", the pass
condition is that the targeted run's per-item report line(s) and write set for
the named item are byte-identical to a full-sync run over the same fixture
(equivalently, the corresponding full-sync acceptance tests pass unchanged when
scoped to that item). The comparison covers per-item lines only; run-level lines
that a targeted run legitimately differs on — notably the discovery line, which
reads "skipped" — are excluded.

- [ ] Given one `--target`, when sync runs, then only that item is reconciled and
      no other local item is written.
- [ ] Given several `--target` options mixing local ids, remote ids, and paths,
      when sync runs, then exactly those items are reconciled.
- [ ] Given a remote tracker id matching an item's `external_id`, when sync runs,
      then that local item is reconciled.
- [ ] Given a token that is both a valid local-id shape and a match for some
      item's `external_id`, when sync runs, then the local-id item is reconciled
      and the report notes the remote-id match as suppressed.
- [ ] Given a target with no `external_id`, when a bidirectional or push run
      executes, then the remote issue is created and `external_id` is written
      back.
- [ ] Given a named target whose local and remote sides both exist but are
      unlinked, when sync runs, then it is reconciled as in a full sync.
- [ ] Given a named target whose `external_id` points to a deleted remote issue,
      when sync runs, then it is handled as in a full sync.
- [ ] Given named targets, when sync runs, then untracked-remote discovery does
      not run and the report's discovery line reads "skipped".
- [ ] Given a target that resolves to nothing across both local and remote id
      spaces, or is otherwise invalid, when sync runs, then it aborts before any
      remote or local write and names the offending target(s).
- [ ] Given a `--target` path that resolves to a file outside the work directory,
      when sync runs, then it aborts before any remote or local write and names
      the offending path.
- [ ] Given a target with `--preview`, when sync runs, then the plan for that
      item is shown with no writes.
- [ ] Given a target with `--push-only` or `--pull-only`, when sync runs, then
      only that direction is applied to the named item.
- [ ] Given a named item that is remotely modified and locally dirty, when sync
      runs, then the dirty-overwrite guard skips it and reports it, as in a full
      sync.
- [ ] Given a named target in conflict (bidirectional, remotely modified and
      locally changed), when sync runs, then a conflict dossier is produced and
      the item is reported unresolved, as in a full sync.
- [ ] Given more named targets to push than `--max-pushes N` allows, when sync
      runs, then at most N pushes occur and the remainder are reported as bounded,
      as in a full sync.
- [ ] Given more named targets to pull than `--max-pulls N` allows, when sync
      runs, then at most N pulls occur and the remainder are reported as bounded,
      as in a full sync.
- [ ] Given a named target and `--resolve <id>=<remote|local|skip>`, when sync
      runs, then that conflict resolution is applied to the named item as in a
      full sync.
- [ ] Given a completed targeted run, when sync next runs, then the sync baseline
      has advanced for the reconciled items, as in a full sync.
- [ ] Given a targeted run interrupted mid-way, when sync runs again, then it
      resumes without re-pushing already-synced items, as in a full sync.
- [ ] Given no target, when sync runs, then its report and write set match a
      captured golden run of the current full sync over the same fixture (or an
      existing full-sync acceptance test passes unchanged).

## Open Questions

- None — all resolved during review.

## Dependencies

- Blocked by: none. The `accelerator work sync` full-sync engine that owns the
  per-item semantics reused here is already shipped under epic 0146; this story
  only adds an engine-level target set and skill-side parsing, so 0146 is a
  completed prerequisite, not an open blocker. 0213 (id-targeted resolve) is not
  a blocker either — local-id and path resolution reuses the existing
  `accelerator work resolve`, and remote-id resolution is self-contained here.
- External: the Linear tracker (via `work.integration`). Every push, pull, and
  `external_id` lookup depends on the Linear API being reachable and on the
  full-sync engine's existing rate-limit and error handling; this coupling is
  inherited from the full sync, not added here.
- Relates to (no ordering constraint, each concurrent): 0146 (parent epic), 0229
  (pull scope), 0213 (id-targeted resolve), 0255 (chunk merge).
- Integration watch-out: 0229 (pull scope) and 0255 (chunk merge) touch the same
  engine pull/discovery path as this story's discovery suppression. No delivery
  ordering is required, but whichever lands second should expect to integrate
  against the other's changes to that path rather than assuming disjoint code.

## Assumptions

- Naming targets suppresses untracked-remote discovery.
- Per-item semantics are identical to the full-sync semantics (conflict, dirty
  guard, bounds, baseline advance).
- Local-id and path resolution reuses `accelerator work resolve` rather than a
  new parser; remote-id resolution is a new lookup over `external_id`.
- On a token matching both a local-id shape and some `external_id`, the local-id
  interpretation wins; the remote-id match is reported as suppressed, not as
  ambiguity.
- A named target whose local and remote sides both exist but are unlinked, and a
  remote-absent target (`external_id` set, issue gone), are handled by the
  existing full-sync semantics with no targeted-run special-casing.

## Technical Notes

The safety-critical orchestration lives in the `accelerator work sync` engine,
not the skill; targeting needs an engine-level target set plus skill-side
argument parsing, with the skill confined to gating, parsing, and rendering.
`--max-pulls`/`--max-pushes` still apply but are naturally small for a targeted
run. The `#\tdiscovery\t…` report line should say "skipped" for a targeted run
so a full run that found nothing is never mistaken for a skip.

## Drafting Notes

- Retitled from "Sync A Single Work Item" to reflect the multi-target ask.
- Proposed parent 0146, inferred from the sync-enhancements family.
- Read "user" as an engineer maintaining work items locally.
- Treated discovery-suppression as in scope.
- Interpreted "remote tracking system ID" as the `external_id` frontmatter
  field.
- Review 1 decisions: target surface is a repeatable `--target` flag; on a
  dual-shape token the local-id interpretation wins; unlinked and remote-absent
  targets follow existing full-sync behaviour; 0213 confirmed a peer, not a
  blocker; a path target outside the work directory is rejected (abort, no
  writes).

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
- Related: 0051, 0146, 0229, 0213, 0255, 0230
