---
type: "work-item"
id: "0300"
title: "Sync Pulls of Tracked Work Items Fail Silently"
date: "2026-10-05T20:03:25+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "high"
parent: "work-item:0146"
relates_to: ["work-item:0257", "work-item:0285"]
external_id: "PP-884"
tags: ["bug", "sync", "work-cli"]
last_updated: "2026-10-05T20:03:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0300: Sync Pulls of Tracked Work Items Fail Silently

**Kind**: Bug
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

`accelerator work sync` fails every pull into an already-tracked local work
item, and reports the failure as a bare `failed … -` row with exit 0. Remote
edits to synced items therefore never reach the working copy, and a
`--resolve <id>=remote` conflict choice cannot take effect.

## Context

The sync applier writes pulled content through `SyncPorts.writer`. The CLI
wires that port to the same `FileCorpusStore` it builds for the baseline,
rooted at the integration state directory
(`.accelerator/state/integrations/<tracker>/`). The store's containment check
refuses any path outside that root, so the write to
`meta/work/<id>-<slug>.md` is rejected. Creating items from remote is
unaffected because it writes through the separate local-author port.

The refusal surfaces as an I/O-class apply error, which has no failure class.
The report renders an unclassified failure with detail `-`, discards its
message, and the exit-code calculation treats it as clean.

Observed on 2026-10-05 against Linear: ten items (0183, 0184, 0197, 0204,
0207, 0209, 0227, 0229, 0230, 0257) reported `failed remotely-modified -`.
The full run exited 4 only because conflicts were also pending.

## Requirements

Reproduction:

1. Have a synced work item whose remote issue was edited after the last sync,
   with a clean local file.
2. Run `accelerator work sync --target <id>`.

Expected: the local file is rewritten from remote, the baseline advances, and
the row reads `pull`.

Actual: the row reads `<id>\tfailed\tremotely-modified\t-`, the file and
baseline are untouched, nothing is printed to stderr, and the exit code is 0.
`--preview` plans the same item as a `pull`; `--per-item-reads` makes no
difference.

The fix must:

- Let a pull write to a work-item file inside the work directory, while
  baseline writes stay bounded to the integration state directory.
- Classify every apply failure so that none renders as `-` or exits 0.
- Surface each failure's message on stderr (or in the report detail), naming
  the item and the failed operation.

## Acceptance Criteria

- [ ] Given a tracked item that was modified remotely and is clean locally,
      when `work sync` runs in apply mode, then the local file holds the remote
      body under its existing frontmatter, the baseline entry advances, and the
      row's action is `pull`.
- [ ] Given a conflicted item, when `work sync --resolve <id>=remote` runs,
      then the local file is overwritten from remote and the row reports the
      pull as applied.
- [ ] Given a pull whose local write fails for any reason, when `work sync`
      runs, then the row's detail is a failure class (not `-`), stderr names
      the item, the operation, and the cause, and the exit code is non-zero.
- [ ] Given a pull aimed at a path outside the work directory, when the
      applier writes it, then the write is refused.
- [ ] A regression test drives a pull of an existing item through the real
      `FileCorpusStore` wiring the CLI uses, not a fake writer.

## Open Questions

- Which failure class should a local I/O failure take: retryable (exit 70) or
  terminal (exit 71)? A failed local write is safe to retry because the
  baseline is written last, which argues for retryable.
- A `--target` run deletes the conflict dossiers of items it did not target.
  Is that intended? If not, does it belong in this item or a separate one?

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- Only the writer wiring is wrong. The pull reconstruction and baseline logic
  are correct once the write succeeds (the preview plans these pulls
  correctly).

## Technical Notes

- `work-cli/src/sync.rs` (`FileCorpusStore::new(baseline_dir)` passed as both
  the baseline store and `SyncPorts.writer`).
- `work-adapters/src/sync/apply.rs` (`ItemApplier::pull` writes through
  `self.writer`; `ApplyError::class` returns `None` for `Io`).
- `work-cli/src/sync.rs` (`render_report` maps `None` to `-`;
  `exit_code_for_report` has no branch for unclassified failures).
- Existing applier tests use spy or fake `AtomicWrite`s, which is why the
  containment refusal was never exercised.

## Drafting Notes

- Kind is `bug` and priority `high`, as specified. Pulls are one half of
  bidirectional sync, and every one of them is broken.
- Scoped to the pull wiring and failure reporting. The dossier pruning is
  parked as an open question rather than merged in, because its cause is
  unconfirmed.
- Choosing between a separate work-directory-bounded writer and a widened
  store is left to the implementer; either satisfies the criteria.

## References

- Related: 0146, 0257, 0285
