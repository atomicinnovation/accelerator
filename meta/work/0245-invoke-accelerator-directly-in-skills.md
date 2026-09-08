---
type: "work-item"
id: "0245"
title: "Invoke Accelerator Directly In Skills"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "done"
kind: "task"
priority: "medium"
parent: "work-item:0136"
relates_to: ["work-item:0106", "work-item:0167", "work-item:0212", "work-item:0107", "work-item:0182"]
tags: ["skills", "cli"]
last_updated: "2026-09-06T21:26:30+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Traced the version floor: the plugin bin/ PATH addition landed in Claude Code v2.1.91, below the declared minimum v2.1.144, so bin-on-PATH is guaranteed at the supported floor. Replaced the earlier untraced-floor wording in the Open Questions and Dependencies."
schema_version: 1
external_id: "PP-775"
---

# 0245: Invoke Accelerator Directly In Skills

**Kind**: Task
**Status**: Done
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

  **Resolved affirmatively (2026-09-06).** A throwaway probe skill invoking
  `!`accelerator config path plans --fail-safe`` resolved and emitted the plans
  path (`meta/plans`) on both the `!`-preprocessor and Bash-tool surfaces, in a
  main session and in a subagent, with no permission prompt against the bare
  `Bash(accelerator config *)` grant. Bare `accelerator` resolves to
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` — the **identical** binary the pathed
  form named — because Claude Code places the plugin `bin/` on the execution
  `PATH`. On the developer's machine the plugin `bin/` is present but appended
  last, and a personal `~/.local/bin/accelerator` symlink (→
  `${CLAUDE_PLUGIN_DATA}/bin`, which `hooks/launcher-link-refresh.sh` keeps
  pointing at the current version) resolves earlier; both entries `realpath` to
  the same file, so which one wins does not change what runs. That personal
  symlink is a per-developer convenience, not a plugin-shipped guarantee — the
  general mechanism is the on-`PATH` plugin `bin/`.

  **Version floor traced (2026-09-06).** The Claude Code changelog records the
  plugin `bin/` PATH addition at **v2.1.91**
  (`https://code.claude.com/docs/en/changelog#2-1-91`), below the declared
  minimum **v2.1.144** — so bin-on-`PATH` is guaranteed at the supported floor,
  not merely observed on the developer's installed version. The changelog names
  the Bash-tool `PATH`; it does not name the `!`-preprocessor surface, but the
  empirical gate above confirmed that surface resolves too, so both are covered.

  **Accepted trade-off / revert path.** The bare form is unconditional and the
  new lint forbids the pathed form, so there is no per-call-site fallback. A
  future Claude Code regression that stopped putting the plugin `bin/` on the
  execution `PATH` would break every injection skill at load; reverting is a
  coordinated lint change plus a reconversion, accepted here in exchange for one
  uniform call form. Under a `PATH`-shadowing compromise (an unrelated
  `accelerator` earlier on `PATH`) the bare grant authorises whatever resolves
  first, with no re-pin to the explicit path — a low residual risk for a
  developer-tooling surface, recorded rather than mitigated.

## Dependencies

- Blocked by: none for the conversion itself (the precedent items 0167 and
  0212 are done).
- Gated by: the PATH / `CLAUDE_PLUGIN_ROOT` precondition (see Open Questions),
  which must resolve affirmatively before bulk conversion begins.
- Owns / supersedes: 0107 — this item owns the bare-invocation lint rule, now
  built as `tasks/lint/bare_invocation.py` and wired into `build-system:check`
  and `lint:check`. 0107 is closed `done` with a `relates_to` link back here;
  its coverage half is discharged by the pre-existing
  `tasks/lint/skill_permissions.py`.
- Blocks: the epic-0136 single-call-form convergence. An anticipated (not yet
  tracked as a work item) follow-on permissions/allowlist simplification also
  depends on one uniform call form across skills; link its id here once raised.
- Informed by: 0182 (documents that bare invocation still requires
  `CLAUDE_PLUGIN_ROOT` in the environment — the source of this item's central
  Assumption).
- External dependency (tracked, alongside 0182's documented behaviours): Claude
  Code placing the plugin `bin/` on the `!`-preprocessor / Bash-tool `PATH`, so
  bare `accelerator` resolves to `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`. See the
  Open Questions resolution for the traced version floor (v2.1.91) and the
  accepted revert path if a Claude Code change removes it.
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
