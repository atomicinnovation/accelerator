---
type: work-item
id: "0183"
title: "SessionStart hook advisories written to stderr at exit 0 reach nobody"
date: "2026-07-28T09:14:02+00:00"
author: Toby Clemson
producer: implement-plan
status: abandoned
kind: bug
priority: low
relates_to: ["work-item:0182", "work-item:0172"]
tags: [bug, hooks, session-start, diagnostics]
last_updated: "2026-08-08T00:00:00+00:00"
last_updated_by: Toby Clemson
last_updated_note: "Abandoned: work-item:0172 Phase 7 absorbed this item's
  contract directly. The migrate-discoverability advisory now goes through
  kernel::hooks::session_start's systemMessage field
  (cli/migrate-cli/src/discoverability.rs), the same mechanism this item's
  own Dependencies amendment already pointed to for vcs-detect/config-summary.
  No separate audit-and-fix pass is needed for this one remaining site."
schema_version: 1
external_id: PP-713
---

# 0183: SessionStart hook advisories written to stderr at exit 0 reach nobody

**Kind**: Bug
**Status**: Abandoned
**Priority**: Low
**Author**: Toby Clemson

## Summary

[`hooks/migrate-discoverability.sh:66-72`](../../hooks/migrate-discoverability.sh#L66)
writes its "migration state is behind the plugin, run `/accelerator:migrate`"
advisory to **stderr** and then `exit 0`. The Claude Code hooks reference
specifies stdout handling on exit 0 (for `SessionStart`, it becomes context
Claude can see) and stderr handling on exit 2 and on other non-zero codes (a
`<hook name> hook error` notice in the transcript), but assigns **no channel to
stderr at exit 0** — it is discarded. So the one advisory whose entire purpose
is to tell the *user* to run a skill is emitted on the one channel that reaches
neither the user nor Claude.

The documented mechanism for putting a line in front of the user is the
universal top-level `systemMessage` JSON output field, which
`hooks/vcs-detect.sh:14` already used before 0169 (2026-08-06) retired it in
favour of `accelerator vcs detect` — see the Dependencies amendment below for
where that pattern lives now.

## Context

Surfaced while recording the hook output-channel determinations for 0182
(Phase 0), which fixed that plan's own two-channel split. 0182 does not touch
`migrate-discoverability.sh`, so the pre-existing case is raised separately
rather than folded in.

Two constraints make this more than a one-line channel swap:

- **At most one JSON object may reach stdout**, so a hook that emits both an
  `additionalContext` envelope and a `systemMessage` must merge them into a
  single object rather than printing twice.
- **For `SessionStart`, plain stdout is Claude's context, not user output.** A
  diagnostic written there would be spliced into the prompt, so switching the
  advisory from stderr to bare stdout would be a regression, not a fix.

The same question applies to any other `SessionStart` advisory in `hooks/` that
uses stderr at exit 0; the audit is part of the work rather than assumed to
return only this one site.

## Requirements

- Audit every `SessionStart` hook in `hooks/` for user-facing text written to
  stderr at exit 0.
- Move each such advisory onto a top-level `systemMessage` field, merged into
  whatever single JSON object the hook already emits on stdout.
- Keep genuinely internal diagnostics (tracing, "nothing to do" notes) where
  they are — the change is scoped to text intended for the user.
- Extend the relevant hook suites so a regression to stderr-at-exit-0 is caught.

## Acceptance Criteria

- [ ] Running `hooks/migrate-discoverability.sh` against a repo whose migration
      state is behind the plugin emits the advisory inside a single JSON object
      on stdout carrying a top-level `systemMessage`, and writes nothing to
      stderr.
- [ ] When that hook has both an advisory and any other stdout output to emit,
      exactly one JSON object reaches stdout.
- [ ] The advisory text still names `/accelerator:migrate` and both the highest
      applied and highest available migration ids.
- [ ] A hook suite asserts the advisory is absent from stderr and present in the
      parsed stdout JSON, so a revert to `>&2` turns it red.
- [ ] The audit's result is recorded — every `SessionStart` hook is either
      converted or explicitly listed as having no user-facing stderr output.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- Related: 0182 (recorded the channel determination that surfaced this).
- **Amendment 2026-08-06 — `accelerator vcs detect` is a new SessionStart audit
  site, outside this item's literal `hooks/` scope.** 0169 replaced
  `hooks/vcs-detect.sh`'s SessionStart registration with
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs detect --format=hook --fail-safe
  --descriptive` — a hooks.json-registered command that is no longer a file
  under `hooks/` at all, so the Requirements' "audit every `SessionStart` hook
  in `hooks/`" wording would silently miss it. On inspection it is already
  compliant: `cli/vcs-cli/src/detect.rs`'s `run` renders an adapter failure via
  `kernel::hooks::adapter_failure` (a bare top-level `systemMessage`, no
  `hookSpecificOutput`, nothing written to stderr for user-facing text), and
  the normal-path envelope goes through `kernel::hooks::session_start`, which
  merges `systemMessage` into the same `additionalContext` object rather than
  emitting two JSON values — satisfying this item's own "at most one JSON
  object on stdout" constraint natively. `hooks/config-detect.sh` was also
  retired the same way, onto `accelerator config summary --format=hook`
  (`cli/launcher/src/config_command`), likewise systemMessage-based. When this
  item's audit runs, broaden its scope statement to cover hooks.json-registered
  binary commands, not just files physically under `hooks/`.

## Assumptions

- The hooks reference's silence on stderr-at-exit-0 means discarded rather than
  undocumented-but-shown. Worth a five-minute live check before the fix, since
  the whole item rests on it.

## References

- Claude Code hooks reference — "Exit code" and "JSON output" sections
- `meta/work/0182-cli-derives-plugin-root-from-own-location.md`
