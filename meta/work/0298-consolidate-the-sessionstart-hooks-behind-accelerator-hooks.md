---
type: "work-item"
id: "0298"
title: "Consolidate the SessionStart Hooks Behind accelerator hooks session-start"
date: "2026-09-25T17:53:43+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "low"
blocked_by: ["work-item:0226"]
derived_from: ["plan-review:2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys-review-1"]
relates_to: ["work-item:0172"]
tags: ["hooks", "session-start", "launcher"]
last_updated: "2026-09-25T17:53:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0298: Consolidate the SessionStart Hooks Behind accelerator hooks session-start

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Replace the three `accelerator` SessionStart hook entries with one
`accelerator hooks session-start` command. It runs VCS detection, the config
summary and the migrate advisory in a fixed order and emits one merged hook
envelope. Session start then costs one launcher process instead of three,
the contributions arrive in a deterministic order, and a later contribution
can rely on an earlier one having run.

## Context

`hooks/hooks.json` registers four SessionStart entries:

- `accelerator vcs detect --format=hook --fail-safe --descriptive`;
- `accelerator config summary --format=hook --fail-safe`;
- `accelerator migrate --discoverability-hook --format=hook --fail-safe`;
- `hooks/launcher-link-refresh.sh`.

Claude Code runs the hooks matched for an event without an ordering
guarantee across entries, so no entry can depend on another having run. One
hook invocation may print only one JSON object, so two of these commands
cannot be chained in one entry either.

0226's Phase 6 hit this. `config summary` asks the dispatched `vcs` sub-binary
whether `config.local.md` is tracked, and in the first session after an
upgrade, `vcs detect` may be fetching the same binary at the same moment.
0226 accepted the duplicate fetch, which is correct because the binary cache
writes atomically, and deferred sequencing to this item.

The envelope pieces already exist. `kernel::hooks::session_start(context,
Some(message))` emits both `additionalContext` and `systemMessage`, and 0172
made `systemMessage` the user-facing channel.

## Requirements

1. Add an `accelerator hooks` subcommand namespace, with `session-start` as
   its first command.
2. `accelerator hooks session-start` runs VCS detection, then the config
   summary, then the migrate advisory, in that order, each with its current
   behaviour and flags.
3. It emits exactly one hook JSON object. `additionalContext` concatenates
   each contribution's context in run order. `systemMessage` joins each
   contribution's user message, and is omitted when there is none.
4. A contribution that fails under today's `--fail-safe` semantics is
   dropped without suppressing the others. The command exits 0 unless a
   contribution fails in a way that fails its hook today, such as the config
   store refusing an insecure `config.local.md`.
5. `hooks/hooks.json` registers `accelerator hooks session-start` plus
   `hooks/launcher-link-refresh.sh` for SessionStart. The three replaced
   entries are removed.
6. For the same repository state, the merged envelope carries the same
   context and messages as the three separate hooks carry today.

## Acceptance Criteria

- [ ] Given a repository where each of the three contributions produces
      output, when `accelerator hooks session-start` runs, then it prints one
      JSON object whose `additionalContext` holds the VCS, config and migrate
      contexts in that order.
- [ ] Given contributions that produce user messages, when the command runs,
      then `systemMessage` holds all of them, and given none, then
      `systemMessage` is absent.
- [ ] Given the VCS contribution fails under `--fail-safe`, when the command
      runs, then it exits 0 and the envelope carries the config and migrate
      contributions.
- [ ] Given an insecure `config.local.md`, when the command runs, then it
      fails as the config summary hook does today.
- [ ] Given `hooks/hooks.json`, when it is read, then SessionStart registers
      exactly `accelerator hooks session-start` and
      `hooks/launcher-link-refresh.sh`.
- [ ] Given the command runs in a fresh session, when its wall-time is
      measured, then it is no greater than the slowest of the three hooks it
      replaces run separately today.

## Open Questions

- Does the launcher gather each dispatched contribution by capturing its
  JSON and merging it, or do `vcs-cli` and `migrate-cli` expose a contract
  that returns their contribution structurally? Capturing JSON makes the hook
  envelope a contract between binaries.
- Should the `PreToolUse` hooks (`vcs guard`, `research guard`) move under
  `accelerator hooks` as well?
- Do the existing per-command `--format=hook` modes stay for direct use, or
  are they retired once nothing registers them?

## Dependencies

- Blocked by: 0226, whose Phase 6 adds the consent warnings to the config
  summary contribution and the captured-dispatch `CaptureBinary` port this
  command would reuse.
- Blocks: none.

## Assumptions

- Claude Code runs the hooks matched for an event without ordering them
  across entries. This has not been checked against current documentation.
- Merging `systemMessage` contributions into one message is acceptable to
  users. Today they arrive as separate messages.

## Technical Notes

- `cli/launcher/src/launch/core.rs`: dispatch execs through `ExecBinary`.
  0226 Phase 6's `CaptureBinary` port runs a sub-binary as a captured child
  with a deadline, which is the mechanism this command needs.
- `cli/kernel/src/hooks.rs:9-23`: `session_start` builds the envelope.
- `cli/migrate-cli/src/discoverability.rs`: the migrate advisory's
  `systemMessage` precedent (0172).
- `config summary` runs in-process in the launcher, so of the three
  contributions only the config one needs no dispatch.

## Drafting Notes

- Raised from 0226's plan review, where sequencing the SessionStart hooks was
  judged out of scope for the consent-key work.
- Scoped to SessionStart. Moving `PreToolUse` is left as an open question.
- Priority is low because 0226 is correct without it. The gains are fewer
  processes at session start and deterministic ordering.

## References

- Source: `meta/reviews/plans/2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys-review-1.md`
- Plan: `meta/plans/2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys.md` (Phase 6)
- Related: 0226, 0172
