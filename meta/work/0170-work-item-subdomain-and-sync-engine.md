---
type: work-item
id: "0170"
title: "Work-Item Lifecycle Subdomain"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: ready
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0194", "work-item:0171"]
blocked_by: ["work-item:0194"]
tags: [rust, work-items]
last_updated: "2026-08-05T18:18:52+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-191"
---

# 0170: Work-Item Lifecycle Subdomain

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Build the `accelerator-work` subdomain's work-item lifecycle operations —
create, show, update, resolve, diff — over the shared `corpus`/`config`/
`store` crates, absorbing ID allocation, remote create/update, and
tag-mutation flows from the legacy bash scripts. `create`/`update --push`
call through 0194's `RemoteTracker` port (faked in this story's own unit
tests); the sync engine itself is 0194's scope.

## Context

`skills/work/scripts/` (22 prod scripts) covers create/fetch/update/sync/
normalise/next-number/section-diff/read-field. This story covers the
lifecycle-CRUD half of that surface — create, update, show, resolve, diff —
absorbing the ID-allocation and tag-mutation flows, so plugin maintainers
inherit a typed, bash-3.2-independent, characterization-tested CLI in place
of the current untested shell scripts backing the work-item skills. The sync engine and the
`tracker` crate split off into 0194 on 2026-08-05 (see Drafting Notes)
following a work item review that found the two efforts independently
deliverable and the combined story epic-scale. The `RemoteTracker` port and
the sync state machine live in their own `tracker` crate (0194), not inside
`accelerator-work`; this story's `--push` flows call through that port,
faking it in unit tests. The coverage gap for the previously-untested
lifecycle scripts is closed via characterize-then-port — write a
characterization test capturing each script's pre-port behaviour before
replacing it (see Acceptance Criteria). 11 lifecycle-side `work-item-*`
scripts have no dedicated test suite today; 0194 covers the 4 sync-side
ones separately.

## Requirements

- Implement `accelerator-work` over the shared `corpus`/`config`/`store`
  crates: lifecycle ops `create`, `show` (read-field/read-status), `update`
  (including tag mutations), `resolve`, `diff` (section-diff), plus the
  internal helpers `next-number`, `normalise`, `pattern`,
  `template-field-hints`, `file-dirty`, `project-remote`, and
  `push-decide`.
- `create --push` and `update --push` call through 0194's `RemoteTracker`
  port (faked in this story's unit tests) to create or whole-content-replace
  the remote issue.
- Preserve the `external_id`-as-remote-key convention and the JSONL/
  atomic-write semantics (via `store`).
- Close the coverage gap: characterize-then-port the 11 previously-untested
  lifecycle scripts — `work-item-file-dirty.sh`, `work-item-next-number.sh`,
  `work-item-normalise.sh`, `work-item-project-remote.sh`,
  `work-item-push-decide.sh`, `work-item-read-field.sh`,
  `work-item-read-status.sh`, `work-item-resolve-id.sh`,
  `work-item-section-diff.sh`, `work-item-template-field-hints.sh`,
  `work-item-update-tags.sh` (none has a dedicated `test-work-item-*.sh`
  suite today; `work-item-pattern.sh` already does and is out of scope
  here).

## Acceptance Criteria

- [ ] Given a fresh work item directory, when `accelerator work create`
      runs, then it allocates the next ID per the configured pattern and
      writes the local file with fully populated frontmatter (every field
      the item's `kind` requires, per the `create-work-item` template
      schema); when invoked with `--push` and the remote create call via
      the wired (or, in unit tests, faked) `RemoteTracker` port succeeds,
      `external_id` is substituted before the single write — no file exists
      until success, decline, or confirmed-local-fallback resolves, per
      `work-item-create-remote.sh`'s existing outcome table; when the
      remote call fails, the file is still written but without
      `external_id` (saved unsynced), with guidance matching that table's
      retryable/terminal rows — the command never silently duplicates a
      create on retry.
- [ ] Given a work item file, when `accelerator work update` runs with
      field or tag mutations, then the local file is rewritten atomically
      and, when `--push` targets a synced item, the remote issue is
      replaced via the same whole-content contract as
      `work-item-update-remote.sh`; when the remote replace call fails, the
      command surfaces that script's existing retryable-vs-terminal exit
      distinction (`E_DISPATCH_RETRYABLE` = provably no mutation, safe to
      retry; `E_DISPATCH_TERMINAL` = mutation state uncertain, never
      auto-retried) and this story defines the corresponding local-file
      outcome for each case as part of its implementation — it must not
      leave the local file silently diverged from a replace that may have
      actually applied.
- [ ] Given a work item file, when `accelerator work show <path>` runs,
      then it prints the full rendered item; when run with `--field NAME`
      (including the `--field status` shorthand), it prints only that
      field's value, matching `work-item-read-field.sh`/
      `work-item-read-status.sh`'s output.
- [ ] Given a path, full ID, or bare number, when `accelerator work resolve
      <input>` runs, then it resolves to the same absolute path (or the
      same exit codes for unrecognised/ambiguous/no-match input) as
      `work-item-resolve-id.sh`.
- [ ] Given a local and a remote work item representation, when
      `accelerator work diff <local> <remote>` runs, then it reports the
      same per-section differences as `work-item-section-diff.sh`.
- [ ] Given each of the 11 previously-untested lifecycle scripts named in
      Requirements, a characterization test captures its pre-port
      behaviour — covering each documented flag/argument combination and
      at least one error path — before the Rust port replaces it.
- [ ] The lifecycle parity suite (`accelerator work create`/`update`/
      `show`/`resolve`/`diff` against the repointed
      `skills/work/scripts/test-work-item-{create-remote,pattern,
      update-remote}.sh` gates and the new characterization suites) passes
      with no live network calls in unit tests — remote calls are
      exercised only by a separate, explicitly-tagged contract/integration
      suite, gated behind a cargo-nextest filter excluded from the default
      `cargo test`/`cargo nextest run` invocation.
- [ ] The migrated lifecycle `work-item-*.sh` scripts (every script named
      in Requirements, plus `work-item-create-remote.sh`,
      `work-item-update-remote.sh`, and `work-item-pattern.sh`) and their
      `test-*.sh` suites are removed and the work suite floor is
      decremented in the same change; `work-item-fetch-remote.sh` and the
      sync-stage scripts stay until 0194 removes them (fetch-remote is a
      dependency of `work-item-sync-apply.sh`, not of any lifecycle
      command).

## Open Questions

- Whether the internal-function boundary for `work-item-pattern.sh`,
  `work-item-template-field-hints.sh`, `work-item-file-dirty.sh`,
  `work-item-project-remote.sh`, `work-item-push-decide.sh`, and
  `work-item-normalise.sh` (kept as private functions per Technical Notes,
  not separate subcommands) holds once `accelerator-work` scaffolding
  starts — a bash-era boundary may turn out to matter for a reason not
  visible from a script's header comment alone.
- Follow-up: this item's remote counterpart (`external_id: PP-191`) still
  reflects the pre-split title/scope and needs reconciling — via
  `accelerator work update --push` or an equivalent sync — on the next
  push after this split.

## Dependencies

- The pre-split blockers are both done as of 2026-08-05: 0166 (shared
  crates) and 0187 (generalises the sub-binary registration surface).
- Blocked by: 0194 (tracker crate and remote sync engine) — the split
  introduced this new blocker: `create`/`update --push` call through the
  `RemoteTracker` port 0194 defines, so only 0194's port (not its full
  `sync` command) needs to land first; this story fakes the port in its
  own unit tests.
- Relates to: 0194 (split sibling — carries the sync engine and `tracker`
  crate that this story was originally bundled with).
- Relates to: 0171 (Jira and Linear Integrations) — this story's own
  Acceptance Criteria only require the faked port, but end-to-end `--push`
  against a real tracker is gated on 0171 landing (via 0194).
- Parent: epic 0136.

## Assumptions

- 0194's `RemoteTracker` port trait is sufficient for this story's
  `--push` behaviour; concrete provider wiring (real Jira/Linear clients)
  happens via 0194/0171, not directly in this story.

## Technical Notes

- Source bash: `skills/work/scripts/work-item-common.sh`,
  `work-item-create-remote.sh`,
  `work-item-next-number.sh`, `work-item-normalise.sh`,
  `work-item-pattern.sh`, `work-item-read-field.sh`,
  `work-item-read-status.sh`, `work-item-resolve-id.sh`,
  `work-item-section-diff.sh`, `work-item-update-remote.sh`,
  `work-item-update-tags.sh`, `work-item-file-dirty.sh`,
  `work-item-project-remote.sh`, `work-item-push-decide.sh`,
  `work-item-template-field-hints.sh`.
- Registration follows the checklist 0187 adds at
  `tasks/README.md#registering-a-dispatched-sub-binary`. The dispatch token
  for the `accelerator work <verb>` subcommand namespace is `work` — a
  single word with no separator, so the constraint that a dispatch token
  may not contain `_` (it derives `ACCELERATOR_<TOKEN>_BIN`) doesn't bite
  here; it would only matter if the token were ever renamed to something
  hyphenated or underscored. (2026-08-01, reconciled 2026-08-05)
- **Subcommand vocabulary (resolved 2026-08-05)**: `accelerator work`
  exposes six user-facing subcommands; five are this story's scope, and
  `sync` is 0194's. The remaining scripts become private functions inside
  the `accelerator-work`/`tracker` crates rather than separate
  subcommands, since bash needed subprocess boundaries for testability
  that Rust doesn't:
  - `work create` — absorbs `work-item-next-number.sh` (ID allocation) and
    `work-item-create-remote.sh` (`--push` flag triggers the remote
    create).
  - `work update` — absorbs `work-item-update-tags.sh` (`--add-tag`/
    `--remove-tag` flags) and `work-item-update-remote.sh` (`--push`).
  - `work show <path> [--field NAME]` — absorbs `work-item-read-field.sh`
    and `work-item-read-status.sh` (`--field status` shorthand).
  - `work resolve <input>` — direct port of `work-item-resolve-id.sh`.
  - `work diff <local> <remote>` — direct port of
    `work-item-section-diff.sh`.
  - `work sync [--push-only|--pull-only]` — 0194's scope, not this story's.
  - Internal-only (no CLI subcommand), this story's scope:
    `work-item-pattern.sh`, `work-item-template-field-hints.sh`,
    `work-item-file-dirty.sh`, `work-item-project-remote.sh`,
    `work-item-push-decide.sh`, `work-item-normalise.sh` — each becomes a
    private function called by `create`/`update`, still unit-tested
    directly at the function level (satisfies the characterization-test
    acceptance criterion without needing a CLI surface).

## Drafting Notes

- Split on 2026-08-05 into this item (lifecycle CRUD) and 0194 (tracker
  crate and remote sync engine) following work item review 1
  (`meta/reviews/work/0170-work-item-subdomain-and-sync-engine-review-1.md`),
  which found the two efforts independently deliverable and the combined
  story epic-scale for a `kind: story` item. This item keeps the original
  title's scope narrowed to lifecycle ops; 0194 carries the `tracker` crate
  and `sync` command. This item's remote counterpart (`external_id:
  PP-191`) still reflects the pre-split title/scope until the next push.
- Refined interactively on 2026-08-05, prior to the split: resolved the
  subcommand-vocabulary open question (six user-facing verbs; internal
  helpers stay as private functions rather than 1:1 script ports — see
  Technical Notes), tightened Acceptance Criteria to Given/When/Then, and
  cleared both blockers (0166, 0187) from Dependencies/frontmatter.
- The internal-vs-subcommand grouping is a judgment call, not confirmed
  against an actual implementation spike — see Open Questions.
- Note: 0187's own frontmatter still shows `status: ready`, not `done`, as
  of this edit — Toby confirmed it's actually done; someone should update
  0187's status field separately (out of scope for this edit).

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Split into: `meta/work/0194-tracker-crate-and-remote-sync-engine.md`
- ADRs: ADR-0045, ADR-0052, ADR-0053
