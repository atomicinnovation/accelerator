---
type: "pr-description"
id: "109"
title: "[0285] Targeted pull of remote-only work items and resolution normalisation"
date: "2026-09-09T10:17:35+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0285"
parent: "work-item:0285"
relates_to: ["work-item:0257"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/109"
pr_number: 109
tags: ["work", "sync", "targeting", "pull", "tracker"]
revision: "f397be3a7254074199a45928abe242f86288fa47"
repository: "accelerator"
last_updated: "2026-09-09T10:17:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0285] Targeted pull of remote-only work items and resolution normalisation

## Summary

`work sync --target <token>` now pulls a work item that exists only on the
remote tracker: a token with no local file is looked up by id and, if present,
created locally and reconciled — where it previously aborted with exit 3. The
same change makes targeted resolution deterministic per token, turning the
genuine local/local collision into a hard usage error and retiring the
suppressed-remote note.

## Changes

- **Remote-only pull.** A no-local-match `--target` token becomes a remote
  candidate rather than an abort. After credentials resolve, one
  `fetch_all(candidates)` gate partitions them: `found` ids are imported through
  the existing `create_from_remote` path, `absent` aborts exit 3 naming every
  absent token, `indeterminate` aborts exit 70.
- **Local/local collision is exit 2.** A token that is one file's local id and a
  *different* file's `external_id` now aborts as a usage error naming both files,
  decided from the local corpus with no remote call. The `Suppressed` machinery
  and its report line are removed; a token equal to a single file's own
  `external_id` reconciles silently.
- **Engine carries pull ids.** `ItemSelection::Targeted` gains a `pull_ids`
  slice, filtered against the corpus's canonical `external_id` set at the
  discovery branch and injected as the untracked set, so preview, pull
  accounting, and the watermark fold a targeted pull in exactly as a discovery
  import. A new `DiscoveryStatus::TargetedPull { attempted }` is emitted only for
  a non-empty set; an empty set keeps `SkippedTargeted`, leaving a reconcile-only
  targeted run's output unchanged.
- **Single-sourced exit precedence.** `absent`, `indeterminate`, and the
  `--push-only` remote-only contradiction join the local failures in one `rank`
  function, so precedence `2 > 6 > 3 > 70` holds from one authority across the
  pre-credential and post-credential gates. Locally-resolvable aborts stay
  credential-free.
- **State-directory fix.** On a never-synced integration the state directory did
  not exist, and the atomic-write containment check canonicalises it as the
  trusted root — so the first baseline write authored the file but failed to
  record its baseline. `run_sync` now creates the directory up-front.
- **Skill and docs.** `sync-work-items` is rewritten for the pull-create, the
  narrowed exit 3, exit 70, the `--push-only` remote-only usage error, and the
  `targeted-pull` discovery line. The work item, plan, and a `pass` validation
  report are included.

## Context

- Work item: `meta/work/0285-targeted-pull-of-remote-only-work-items.md`
- Plan: `meta/plans/2026-09-08-0285-targeted-pull-of-remote-only-work-items.md`
- Validation: `meta/validations/2026-09-08-0285-targeted-pull-of-remote-only-work-items-validation.md`
- Builds on the `--target` targeting introduced by work item 0257 (PR #107).

## Testing

- [x] Read-only aggregate clean: `mise run check` (exit 0)
- [x] Resolution, gate, and skill suite green: `cargo test -p accelerator-work`
- [x] Engine suite green: `cargo test -p work-adapters`
- [ ] Live remote-only pull against Linear creates and reconciles the file, and
      a second run does not re-pull it (manual — no stub for the live tracker)
- [ ] A typo'd `--target` reports exit 3 with zero writes against a live tracker
      (manual)

## Notes for Reviewers

- **Deliberate deviation from the plan.** A whole-call `fetch_all` `Err` maps to
  exit 70 (retryable) rather than the plan's three-way split: `TrackerError`
  carries no discriminant to separate an unembeddable-id from a transient fault,
  and a credential fault is already caught at `registry.resolve` (74). Recorded
  in the plan and covered by
  `a_whole_call_fetch_all_error_folds_to_indeterminate_exit_seventy`.
- **No live-tracker coverage in the suite.** The post-credential gate is
  exercised only through `run_sync` with a stub `TrackerRegistry` — the
  subprocess harness cannot inject one — so the two manual steps above are the
  sole check against real Linear/Jira. Focus there before merge.
- **Non-transactional per-id create loop.** A mid-batch `create_from_remote`
  failure leaves earlier creates written with valid baselines and is recoverable
  idempotently on re-run; the gate itself is all-or-nothing (any `absent` or
  `indeterminate` aborts before a write).
