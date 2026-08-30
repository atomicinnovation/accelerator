---
type: "work-item"
id: "0170"
title: "Work-Item Lifecycle Subdomain"
date: "2026-06-28T17:01:56+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "done"
kind: "story"
priority: "medium"
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0194"]
blocks: ["work-item:0194"]
tags: ["rust", "work-items"]
last_updated: "2026-08-07T23:26:37+00:00"
last_updated_by: "Toby Clemson"
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
`store` crates, absorbing ID allocation and tag-mutation flows from the
legacy bash scripts. These commands are local-only, with no dependency on
the remote tracker: `--push` support for `create`/`update` and the sync
engine itself are both 0194's scope — 0194 wires `--push` onto these
commands once they exist, calling through its own `RemoteTracker` port.

## Context

`skills/work/scripts/` (22 prod scripts) covers create/fetch/update/sync/
normalise/next-number/section-diff/read-field. This story covers the
lifecycle-CRUD half of that surface — create, update, show, resolve, diff —
absorbing the ID-allocation and tag-mutation flows, so plugin maintainers
inherit a typed, bash-3.2-independent, characterization-tested CLI in place
of the current untested shell scripts backing the work-item skills. The sync
engine and the `tracker` crate split off into 0194 on 2026-08-05 (see
Drafting Notes) following a work item review that found the two efforts
independently deliverable and the combined story epic-scale. `--push`
support for `create`/`update` was originally scoped here too, but moved to
0194 on a follow-up pass (see Drafting Notes): 0194 wires the flag onto
these commands once both they and its own `RemoteTracker` port exist, so
this story carries no remote-tracker dependency at all — every command here
is local-only. The coverage gap for the previously-untested lifecycle
scripts is closed via characterize-then-port — write a characterization
test capturing each script's pre-port behaviour before replacing it (see
Acceptance Criteria). 10 lifecycle-side `work-item-*` scripts have no
dedicated test suite today; `work-item-push-decide.sh` moved to 0194
alongside the push-wiring it decides for, and 0194 covers the 4 sync-side
scripts separately.

## Requirements

- Implement `accelerator-work` over the shared `corpus`/`config`/`store`
  crates: lifecycle ops `create`, `show` (read-field/read-status), `update`
  (including tag mutations), `resolve`, `diff` (section-diff), plus the
  internal helpers `next-number`, `normalise`, `pattern`,
  `template-field-hints`, `file-dirty`, and `project-remote`. These
  commands are local-only — no remote calls, no `RemoteTracker` dependency;
  `--push` support is 0194's scope (see Dependencies).
- Preserve the `external_id`-as-remote-key convention and the JSONL/
  atomic-write semantics (via `store`).
- Close the coverage gap: characterize-then-port the 10 previously-untested
  lifecycle scripts — `work-item-file-dirty.sh`, `work-item-next-number.sh`,
  `work-item-normalise.sh`, `work-item-project-remote.sh`,
  `work-item-read-field.sh`, `work-item-read-status.sh`,
  `work-item-resolve-id.sh`, `work-item-section-diff.sh`,
  `work-item-template-field-hints.sh`, `work-item-update-tags.sh` (none has
  a dedicated `test-work-item-*.sh` suite today; `work-item-pattern.sh`
  already does and is out of scope here; `work-item-push-decide.sh` moved
  to 0194 alongside the push-wiring it decides for).

## Acceptance Criteria

- [x] Given a fresh work item directory, when `accelerator work create`
      runs, then it allocates the next ID per the configured pattern and
      writes the local file with fully populated frontmatter (every field
      the item's `kind` requires, per the `create-work-item` template
      schema).
- [x] Given a work item file, when `accelerator work update` runs with
      field or tag mutations, then the local file is rewritten atomically
      via the same whole-file replace contract as `work-item-update-tags.sh`
      for tag mutations, with all other fields left unchanged.
- [x] Given a work item file, when `accelerator work show <path>` runs,
      then it prints the full rendered item; when run with `--field NAME`
      (including the `--field status` shorthand), it prints only that
      field's value, matching `work-item-read-field.sh`/
      `work-item-read-status.sh`'s output.
- [x] Given a path, full ID, or bare number, when `accelerator work resolve
      <input>` runs, then it resolves to the same absolute path (or the
      same exit codes for unrecognised/ambiguous/no-match input) as
      `work-item-resolve-id.sh`.
- [x] Given a local and a remote work item representation, when
      `accelerator work diff <local> <remote>` runs, then it reports the
      same per-section differences as `work-item-section-diff.sh`.
- [x] Given each of the 10 previously-untested lifecycle scripts named in
      Requirements, a characterization test captures its pre-port
      behaviour — covering each documented flag/argument combination and
      at least one error path — before the Rust port replaces it.
- [x] The lifecycle parity suite (`accelerator work create`/`update`/
      `show`/`resolve`/`diff` against the repointed
      `skills/work/scripts/test-work-item-pattern.sh` gate and the new
      characterization suites) passes; this crate makes no network calls
      at all, so no separate contract/integration suite is needed for it
      (0194 carries that gate for the push-wiring it adds on top).
- [x] The migrated lifecycle `work-item-*.sh` scripts (every script named
      in Requirements, plus `work-item-pattern.sh`) and their `test-*.sh`
      suites are removed and the work suite floor is decremented in the
      same change; `work-item-create-remote.sh`, `work-item-update-remote.sh`,
      `work-item-push-decide.sh`, `work-item-fetch-remote.sh`, and the
      sync-stage scripts stay until 0194 removes them (they're 0194's
      porting/removal responsibility now, not this story's).

## Open Questions

- Whether the internal-function boundary for `work-item-pattern.sh`,
  `work-item-template-field-hints.sh`, `work-item-file-dirty.sh`,
  `work-item-project-remote.sh`, and `work-item-normalise.sh` (kept as
  private functions per Technical Notes, not separate subcommands) holds
  once `accelerator-work` scaffolding starts — a bash-era boundary may
  turn out to matter for a reason not visible from a script's header
  comment alone.
- Follow-up: this item's remote counterpart (`external_id: PP-191`) still
  reflects the pre-split title/scope and needs reconciling — via
  `accelerator work update --push` (once 0194 wires it onto this story's
  `update` command) or an equivalent sync — on the next push after this
  split.

## Dependencies

- The pre-split blockers are both done as of 2026-08-05: 0166 (shared
  crates) and 0187 (generalises the sub-binary registration surface).
- No remaining blockers: `--push` support (the one thing that needed
  0194's `RemoteTracker` port) moved to 0194's own scope on 2026-08-05
  (see Drafting Notes), so this story now has zero dependency on 0194 or
  0171 — every command here is local-only and can proceed immediately.
- Blocks: 0194 — 0194 wires `--push` onto this story's `create`/`update`
  commands once they exist, so that one slice of 0194's scope needs this
  story's commands to land first; the rest of 0194 (the `tracker` crate,
  the `sync` command, its characterization tests) doesn't depend on this
  story at all.
- Relates to: 0194 (split sibling — carries the sync engine, the `tracker`
  crate, and now the `--push` wiring for this story's commands too).
- Parent: epic 0136.

## Assumptions

- This story's `create`/`update` CLI signatures (flags and arguments) are
  stable once implemented; 0194 extends them with a `--push` flag
  afterwards without needing this story's further involvement.

## Technical Notes

- Source bash: `skills/work/scripts/work-item-common.sh`,
  `work-item-next-number.sh`, `work-item-normalise.sh`,
  `work-item-pattern.sh`, `work-item-read-field.sh`,
  `work-item-read-status.sh`, `work-item-resolve-id.sh`,
  `work-item-section-diff.sh`, `work-item-update-tags.sh`,
  `work-item-file-dirty.sh`, `work-item-project-remote.sh`,
  `work-item-template-field-hints.sh`. `work-item-create-remote.sh`,
  `work-item-update-remote.sh`, and `work-item-push-decide.sh` moved to
  0194 alongside the `--push` wiring they implement (see Drafting Notes).
- Registration follows the checklist 0187 adds at
  `tasks/README.md#registering-a-dispatched-sub-binary`. The dispatch token
  for the `accelerator work <verb>` subcommand namespace is `work` — a
  single word with no separator, so the constraint that a dispatch token
  may not contain `_` (it derives `ACCELERATOR_<TOKEN>_BIN`) doesn't bite
  here; it would only matter if the token were ever renamed to something
  hyphenated or underscored. (2026-08-01, reconciled 2026-08-05)
- **Subcommand vocabulary (resolved 2026-08-05, revised 2026-08-05)**:
  `accelerator work` exposes six user-facing subcommands; five are this
  story's scope, and `sync` is 0194's. The remaining scripts become
  private functions inside the `accelerator-work`/`tracker` crates rather
  than separate subcommands, since bash needed subprocess boundaries for
  testability that Rust doesn't:
  - `work create` — absorbs `work-item-next-number.sh` (ID allocation);
    local-only in this story. 0194 later adds the `--push` flag, absorbing
    `work-item-create-remote.sh`.
  - `work update` — absorbs `work-item-update-tags.sh` (`--add-tag`/
    `--remove-tag` flags); local-only in this story. 0194 later adds the
    `--push` flag, absorbing `work-item-update-remote.sh`.
  - `work show <path> [--field NAME]` — absorbs `work-item-read-field.sh`
    and `work-item-read-status.sh` (`--field status` shorthand).
  - `work resolve <input>` — direct port of `work-item-resolve-id.sh`.
  - `work diff <local> <remote>` — direct port of
    `work-item-section-diff.sh`.
  - `work sync [--push-only|--pull-only]` — 0194's scope, not this story's.
  - Internal-only (no CLI subcommand), this story's scope:
    `work-item-pattern.sh`, `work-item-template-field-hints.sh`,
    `work-item-file-dirty.sh`, `work-item-project-remote.sh`,
    `work-item-normalise.sh` — each becomes a private function called by
    `create`/`update`, still unit-tested directly at the function level
    (satisfies the characterization-test acceptance criterion without
    needing a CLI surface). `work-item-push-decide.sh` moved to 0194
    alongside the `--push` wiring it decides for.

## Drafting Notes

- Validated 2026-08-07 against `meta/plans/2026-08-06-0170-work-item-lifecycle-subdomain.md`
  (`meta/validations/2026-08-06-0170-work-item-lifecycle-subdomain-validation.md`,
  result: pass) — all 9 plan phases implemented and committed, all 8
  acceptance criteria above confirmed met, status moved to `done`. One
  unrelated `mise run` failure was found at HEAD (a later,
  non-0170-scoped commit) and is tracked separately, not against this
  item.
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
- Revised 2026-08-05, following a review discussion: moved `--push`
  support for `create`/`update` (and the `work-item-create-remote.sh`,
  `work-item-update-remote.sh`, and `work-item-push-decide.sh` scripts
  that implement it) out of this story and into 0194, to remove this
  story's dependency on 0194's `RemoteTracker` port entirely. This story
  is now fully local-only and unblocked; 0194 wires `--push` onto these
  commands once they exist, so the dependency direction flips for that one
  slice of 0194's scope (0194 blocked by 0170), while the rest of 0194
  (the `tracker` crate, the `sync` command) remains independent of this
  story, as before.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Split into: `meta/work/0194-tracker-crate-and-remote-sync-engine.md`
- ADRs: ADR-0045, ADR-0052, ADR-0053
