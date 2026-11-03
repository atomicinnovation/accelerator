---
type: "work-item"
id: "0371"
title: "Invocation Validation on accelerator work create --dry-run"
date: "2026-10-28T19:37:23+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0146"
blocked_by: ["work-item:0319"]
external_id: "PP-935"
tags: ["work", "cli", "validation"]
last_updated: "2026-10-28T19:37:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0371: Invocation Validation on accelerator work create --dry-run

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As someone previewing a create, directly or through `/create-work-item`'s
push gate, I want `accelerator work create --dry-run` to refuse every
invocation the real create would refuse, so that a clean preview means the
create will not fail on its arguments or configuration.

## Context

`run_with` in `cli/work-cli/src/create.rs` returns the tracker preview
before `creation_context` runs, so `--dry-run` checks none of the item's
arguments or configuration. A user can pass the push gate and only then
hit `E_PATTERN_MISSING_KEY`, template drift or an unreadable body file.
Without `--push`, `--dry-run` still contacts the tracker and fails outright
when no integration is configured.

## Requirements

- Make `--dry-run` run every invocation check a real create runs, in the
  real create's order, refusing with the same exit code and `E_` message:
  re-key in progress, scheme resolution, `E_PATTERN_KEY_UNUSED`,
  `E_PATTERN_MISSING_KEY`, template resolution and schema drift, author
  resolution, `--body-file` readability, and `E_PATTERN_OVERFLOW`.
- Keep `--dry-run` inert: it writes nothing, creates no remote issue, and
  takes neither the create lock nor the identity-run guard. The re-key and
  overflow checks are advisory, since the real create re-checks both.
- Under `--push --dry-run`, validate before contacting the tracker, and
  send no tracker request when validation refuses.
- Make `--dry-run` without `--push` validate only: it contacts no tracker,
  works with no integration configured, and prints nothing on success.
- Keep a valid `--push --dry-run`'s preview line and exit contract
  unchanged.
- Make `/create-work-item`'s dry-run call carry the same flags as its real
  create.

## Acceptance Criteria

- [ ] Given an unfinished re-key, when `work create --dry-run` runs, then
      it refuses with `E_REKEY_IN_PROGRESS` and exit 4, and the identity-run
      guard is never taken.
- [ ] Given `{key}-{number:04d}` with no `work.key` and no `--project`,
      when `work create --dry-run` runs, then it refuses with
      `E_PATTERN_MISSING_KEY` and writes nothing.
- [ ] Given `--project` under `{number:04d}`, and separately under
      `{tracker}`, when `work create --dry-run` runs, then it refuses with
      `E_PATTERN_KEY_UNUSED`.
- [ ] Given a template whose frontmatter has drifted from the known fields,
      when `work create --dry-run` runs, then it refuses with the real
      create's drift message.
- [ ] Given no `--author` and no VCS identity, when `work create --dry-run`
      runs, then it refuses as the real create does.
- [ ] Given an unreadable `--body-file`, when `work create --dry-run` runs,
      then it refuses with `could not read --body-file`.
- [ ] Given an exhausted number space and another process holding the
      create lock, when `work create --dry-run` runs, then it refuses with
      `E_PATTERN_OVERFLOW` without waiting for the lock.
- [ ] Given each refusing invocation above, when it runs with and without
      `--dry-run`, then both exit with the same code and `E_` prefix.
- [ ] Given an invalid invocation with `--push --dry-run`, when `work
      create` runs, then the tracker receives no request.
- [ ] Given a valid invocation with `--dry-run`, no `--push` and no
      integration configured, when `work create` runs, then it exits 0,
      prints nothing, writes nothing and contacts no tracker.
- [ ] Given a valid invocation with `--push --dry-run`, when `work create`
      runs, then its stdout line and exit code match today's preview.
- [ ] Given `/create-work-item` reaching the push gate, when it runs the
      dry-run, then the call carries every flag its real create carries.

## Open Questions

None.

## Dependencies

- Blocked by: 0319, which lands first and adds its five `--id` checks to
  the dry-run path. This story keeps them in its shared validation, in
  0319's order, and keeps the `--id` 0319 adds to the skill's dry-run
  call.
- Blocks: none.

## Assumptions

- Bare `--dry-run` stops contacting the tracker. Anyone using it for the
  tracker preview must add `--push`; `/create-work-item` already does.

## Technical Notes

- `creation_context` already performs the scheme, `--project`, author and
  template checks, so the dry-run path can run it after `run_with`'s
  early return moves.
- `allocate_id` takes a `CreateLockGuard` token, so the advisory overflow
  check needs an allocation path that holds no lock.
- `identity_run::begin` acquires the guard, so the advisory re-key check
  needs a probe that does not. The `create_dry_run_is_not_guarded` test
  inverts.
- The skill's dry-run call is step 7.1 of
  `skills/work/create-work-item/SKILL.md`.

## Drafting Notes

- "Thorough" means parity with the real create's invocation checks, not
  new validation the real create lacks. `kind`, `priority` and `status`
  stay unvalidated because the real create does not validate them.
- Stakeholders are `/create-work-item` users at the push gate and direct
  callers of `work create --dry-run`.
- Out of scope: the `--id` checks (0319), `create-batch` and
  `extract-work-items`, and unifying with `jira resolve-fields`, which 0171
  decided against.
- Raised while stress-testing 0319. Both change `run_with`'s early return,
  so 0319 lands first and this story builds on its dry-run path.

## References

- `cli/work-cli/src/create.rs`, `skills/work/create-work-item/SKILL.md`
- Related: 0146, 0319, 0230, 0212
