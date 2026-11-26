---
type: "work-item"
id: "PP-1005"
external_id: "PP-1005"
aliases: ["draft-a36xw0"]
title: "Audit Linear issues for rewrites left unmarked"
date: "2026-11-24T22:29:18+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
relates_to: ["work-item:PP-906"]
tags: []
last_updated: "2026-11-24T22:29:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-1005: Audit Linear issues for rewrites left unmarked

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As someone who syncs work items to Linear, I want one command that finds
every issue whose stored description no longer means what my local file
says, so that I can repair it with `accelerator work update <path> --push`
instead of auditing the team by hand.

PP-906 marks an entry `rewritten_on_push` only when a push by a release
that includes PP-906 judges its own read-back. Rewrites that release never
saw stay invisible.

## Context

Two gaps came out of the review of PR #264 (PP-906):

- **Lost marks.** Any baseline write by a release before PP-906 (sync,
  `work update --push`, `work create --push`, promotion, rename)
  re-renders the committed `last-sync.json` and drops every mark. Nothing
  rebuilds them.
- **Damage done before the upgrade.** Issues Linear rewrote before the
  upgrade were never marked. `work list` does not flag them and no
  `still rewritten on push` notice appears, so a user cannot tell which
  `<path>` needs repairing.

A passive rebuild inside every `work sync` cannot reach either case. The
two-tier read rule (`needs_body_read`, `cli/work/src/sync/plan.rs`) fetches
only items whose `updatedAt` stamp no longer proves them unchanged, and in
both cases the remote has not changed since the baseline.

## Requirements

- An opt-in mode, `accelerator work sync --audit-rewrites`, that reads
  every linked item's remote with `show` and judges the stored description
  against the local body. It uses `PushVerdict::judged(PushOrigin::Pushed,
  ..)`, the same equivalence a push uses.
- The audit sets the `rewritten_on_push` mark on each item whose remote
  differs in meaning, and clears it on each item whose remote matches.
- It reports each marked item with the existing `rewritten on push`
  warning block, then a count of the items audited, marked and cleared.
- It writes nothing to the tracker and changes no local work item.
  Nothing but the baseline's marks changes.
- `--dry-run` reports what the audit would mark and clear, and writes
  nothing.
- `/sync-work-items` and `/list-work-items` relay the audit's output
  verbatim.
- The CHANGELOG names the audit as the way to find items to repair after
  upgrading, in place of the manual audit in Phase 12 of the PP-906 plan.

## Acceptance Criteria

- [ ] Given an item whose baseline entry lost its mark and whose Linear
  description still holds a rewrite, when `work sync --audit-rewrites`
  runs, then the entry is marked and the item is reported as
  `rewritten on push`.
- [ ] Given an item Linear rewrote before the upgrade, with no mark, when
  the audit runs, then the item is marked and `work list` flags it with
  ` · rewritten on push`.
- [ ] Given a marked item whose Linear description now matches the local
  body, when the audit runs, then the mark is cleared.
- [ ] Given any audit run, then the tracker receives no `issueUpdate` or
  `issueCreate`, and no local work item file changes.
- [ ] Given `--dry-run`, then the same report is printed and
  `last-sync.json` is byte-identical before and after.
- [ ] Given an item whose remote cannot be read, then the audit leaves its
  mark unchanged and names it in the report.
- [ ] Given a stderr block carrying an audit warning, `/sync-work-items`
  relays the `warning:` and `note:` lines verbatim and suggests no pull.

## Open Questions

- Should the audit also run on unlinked drafts? Probably not, since they
  have no remote.
- Should `/sync-work-items` offer the audit once after an upgrade, or only
  on request?

## Dependencies

- Blocked by: PP-906 (`PushVerdict`, the `rewritten_on_push` mark).

## Assumptions

- The audit judges against the local file as it stands, not against the
  body last pushed. A local edit made since the last push may therefore be
  reported as a rewrite. The user is expected to push in that case anyway.

## Technical Notes

- ⏱️ One `show` per linked item, roughly 100+ calls on the `PP` corpus.
  That cost is why the audit is opt-in rather than part of every sync.
- Reuse `RewriteNotice::on_push` and `BaselineStore::record` with
  `PushVerdict::mark_change()`. Write the mark-only change through
  `MarkChange`, so fingerprints and stamps stay unchanged.

## Drafting Notes

- Placed under `work sync` as a flag rather than as a new subcommand,
  because sync already owns the baseline, the tracker session and the
  rewrite report.
- Clearing a mark on a match is in scope. It is the audit's inverse case
  and costs nothing extra.

## References

- Related: PP-906
- PR #264 review threads on `cli/work-adapters/src/sync/rewrite_mark.rs:68`
  and `CHANGELOG.md:1125`
- `meta/reviews/prs/264-review-1.md`
