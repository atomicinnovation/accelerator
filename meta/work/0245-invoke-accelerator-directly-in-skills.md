---
type: "work-item"
id: "0245"
title: "Invoke Accelerator Directly In Skills"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "task"
priority: "medium"
parent: "work-item:0136"
relates_to: ["work-item:0106", "work-item:0167", "work-item:0212", "work-item:0107", "work-item:0182"]
tags: ["skills", "cli"]
last_updated: "2026-09-05T23:48:43+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0136 (Migrate Shell Scripts into a Rust CLI): belongs to the shell-to-Rust migration, its shipped cli/ crates, or the launcher runtime-cache cluster."
schema_version: 1
external_id: "PP-775"
---

# 0245: Invoke Accelerator Directly In Skills

**Kind**: Task
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Standardise every skill so it invokes the `accelerator` CLI as a bare command
(`accelerator …`) rather than through the `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`
path prefix. That `bin` directory is already on `PATH`, so the prefix is
redundant. A single uniform convention is easier to allowlist for permissions,
read, and maintain.

## Context

Part of the shell-to-Rust CLI migration (epic 0136). 0106 moved skill bodies to
bash-free path invocation (dropping the `bash` prefix while retaining the
`${CLAUDE_PLUGIN_ROOT}/bin/` path); 0167 and 0212 cut the config and work
clusters to no-path `accelerator …`. Skills remain inconsistent — many
still carry the `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` prefix, and some render
as absolute paths. This item completes the convergence to a single call form
across all skills. The launcher bootstrap (`bin/accelerator`) is retained; only
the invocation form in skill bodies changes.

## Requirements

- Every SKILL.md body that shells out to Accelerator invokes it as bare
  `accelerator …` — no `${CLAUDE_PLUGIN_ROOT}/bin/` prefix, no absolute path,
  no `bash` wrapper.
- The bare form is unconditional. `${CLAUDE_PLUGIN_ROOT}/bin` being on `PATH`
  in every skill execution context is a precondition; the PATH Open Question
  must resolve affirmatively before conversion proceeds. No call site retains
  the explicit path.
- The launcher bootstrap `bin/accelerator` is retained; only the call form in
  skills changes.
- No behavioural change: each converted call resolves to the same binary and
  produces identical output.
- Scope is SKILL.md bodies. Hooks are out of scope — they dispatch via
  `hooks.json` through `bin/accelerator` and may not have `bin` on `PATH`.
- A lint rule — owned by this item, superseding 0107 — guards the convention
  after conversion, so a reintroduced `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`
  prefix, absolute path, or `bash` wrapper fails the check.

## Acceptance Criteria

- [ ] Given an allowlist lint over `skills/**/SKILL.md`, when run, then every
  `accelerator` invocation matches the bare form and none carries a path
  prefix, absolute path, or `bash` wrapper.
- [ ] Given greps across `skills/` for each forbidden form — the literal
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`, the `/bin/accelerator` path suffix
  (catching fully expanded absolute-path renderings), and `bash .*accelerator`
  — when run, then each returns zero matches.
- [ ] Given a SKILL.md that reintroduces any forbidden form (path prefix,
  absolute path, or `bash` wrapper), when the lint runs, then the lint fails
  and names the offending file.
- [ ] Given each skill execution context named in the Open Question (main
  session and subagents), when a bare `accelerator` call runs via the `!`
  preprocessor, then it resolves and exits successfully; because it resolves to
  the same on-`PATH` binary as the former path-prefixed call, its output is
  identical by construction.
- [ ] Given the full suite after conversion, when `mise run check` and the tests
  run, then they pass with no regressions.

## Open Questions

- Is `${CLAUDE_PLUGIN_ROOT}/bin` reliably on `PATH` (and `CLAUDE_PLUGIN_ROOT`
  set) in every context a SKILL.md `!` preprocessor runs — main session and
  subagents? Asserted yes. This is a gating precondition: it must be verified
  affirmatively before bulk conversion, since the bare form is now
  unconditional and no call site retains the explicit path as a fallback.

## Dependencies

- Blocked by: none for the conversion itself (the precedent items 0167 and
  0212 are done).
- Gated by: the PATH / `CLAUDE_PLUGIN_ROOT` precondition (see Open Questions),
  which must resolve affirmatively before bulk conversion begins.
- Owns / supersedes: 0107 — this item owns the bare-invocation lint rule;
  0107's lint-enforcement scope is subsumed here, so coordinate closure of
  0107 rather than building a duplicate rule.
- Blocks: the epic-0136 single-call-form convergence. An anticipated (not yet
  tracked as a work item) follow-on permissions/allowlist simplification also
  depends on one uniform call form across skills; link its id here once raised.
- Informed by: 0182 (documents that bare invocation still requires
  `CLAUDE_PLUGIN_ROOT` in the environment — the source of this item's central
  Assumption).
- Relates to: 0106 (bash-free path invocation convention), 0167 and 0212
  (config and work clusters already migrated).

## Assumptions

- `${CLAUDE_PLUGIN_ROOT}/bin` is on `PATH` and `CLAUDE_PLUGIN_ROOT` is set in
  all skill execution contexts, so bare `accelerator` resolves. This is treated
  as a gating precondition to verify, not a per-call-site fallback: if it does
  not hold for a context, the conversion is blocked and the assumption must be
  revisited rather than that call site keeping the explicit path.
- Hooks are excluded; only skills change.

## Technical Notes

- Mechanical replace of `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` — and any
  absolute-path or `bash`-prefixed variants — with `accelerator` across
  `skills/**/SKILL.md`.
- 0182 documents that `bin/accelerator` needs `CLAUDE_PLUGIN_ROOT` in the
  environment; bare invocation still depends on that being set.

## Drafting Notes

- Interpreted "invoke directly / remove the wrapper" as dropping the
  `${CLAUDE_PLUGIN_ROOT}/bin/` path prefix in favour of bare `accelerator`
  (confirmed with the author), not dropping the launcher bootstrap.
- Scoped to skills only; hooks excluded because they dispatch through
  `bin/accelerator` and may lack `bin` on `PATH`. Flag if hooks should be in
  scope.
- Lint enforcement is in scope here and this item owns the rule, superseding
  0107 ("Lint Skill-Body Script Invocations"). 0107's lint scope is subsumed;
  close or re-point it rather than building a duplicate lint.
- Relates-to links drawn from a work-directory scan. Initially left unparented
  by choice; subsequently reparented under epic 0136 (the shell-to-Rust CLI
  migration), which the `parent` field now records.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
- Related: 0106, 0167, 0212, 0107, 0182
- Epic: 0136
