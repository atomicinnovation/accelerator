---
type: work-item
id: "0107"
title: "Lint Skill-Body Script Invocations Against allowed-tools Rules"
date: "2026-06-11T13:10:03+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: medium
blocked_by: ["work-item:0106"]
relates_to: ["work-item:0098", "work-item:0106"]
source: "issue-research:2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission"
tags: [tooling, linting, static-analysis, ci, skills, allowed-tools, guardrails]
last_updated: "2026-06-11T13:10:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-129
---

# 0107: Lint Skill-Body Script Invocations Against allowed-tools Rules

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add an automated lint/test guardrail that, for every `SKILL.md`, cross-checks each
plugin-script invocation in the body against the skill's `allowed-tools` frontmatter
and fails when an invocation's *shape* is not covered by an allow rule. This converts
the manual, point-in-time grep checks that work item 0106 relies on into a repeatable
gate, so the bare-path invocation convention cannot silently erode as skills are
edited.

## Context

Work item 0106 fixes every current `artifact-*`/`config-*` call site by hand and
verifies the result with a manual grep sweep. Nothing prevents a later skill edit from
reintroducing a `bash`/`sh`/`env`-wrapped invocation, a script path inside a bare
unlabeled code fence, or an invocation whose shape no longer matches any
`allowed-tools` prefix — each of which silently re-breaks the permission match and
reintroduces the unnecessary-prompt symptom the source research diagnosed. The
research's Prevention section explicitly recommends a lint/test that keeps body
invocation shape and rule prefix in lockstep. This item builds that check. It is the
semantic, plugin-specific complement to the generic per-language tooling in 0098
(shellcheck/shfmt etc.): 0098 lints script *contents*; this lints the *relationship*
between a skill's body invocations and its declared permissions.

## Requirements

- Provide a check, runnable both locally (via the existing task runner) and in CI, that
  for each `SKILL.md` under `skills/`:
  - extracts every invocation in the body that references a plugin script (e.g. paths
    containing `${CLAUDE_PLUGIN_ROOT}/scripts/`, `scripts/artifact-*`,
    `scripts/config-*`, or other `scripts/` references the skill invokes);
  - extracts that skill's `allowed-tools` `Bash(...)` rules from frontmatter (expanding
    `${CLAUDE_PLUGIN_ROOT}` consistently with how the plugin loader expands it);
  - asserts each invocation's command shape is covered by at least one rule prefix.
- Flag, as distinct violations with actionable messages (file, line, the offending
  invocation, and why it is uncovered):
  - invocations wrapped in `bash`/`sh`/`env` (or other non-stripped wrappers) that
    escape the path-prefix rule;
  - plugin-script paths sitting inside a bare unlabeled code fence (reuse 0106's AC3
    regression-guard pattern as one assertion);
  - invocation shapes (quoting/path form) not matched by any `allowed-tools` prefix for
    that skill.
- Integrate the check into the repo's lint/test harness established by 0098 so it runs
  as part of the standard `mise`/task-runner lint or test target and fails the build on
  any violation.
- Emit a zero-violation success that is meaningful — i.e. the check must be demonstrated
  to catch a deliberately-seeded violation (known-positive), so a green result proves
  coverage rather than a broken matcher.

## Acceptance Criteria

- [ ] Given a `SKILL.md` whose body invokes a plugin script in a form covered by one of
      its `allowed-tools` `Bash(...)` rules, when the check runs, then it reports no
      violation for that invocation.
- [ ] Given a `SKILL.md` that invokes a plugin script wrapped as `bash <path>` (or
      `sh`/`env <path>`), when the check runs, then it fails and names the file, line,
      and the wrapped invocation.
- [ ] Given a `SKILL.md` with an `artifact-*`/`config-*` script path inside a bare
      unlabeled code fence, when the check runs, then it fails and identifies the fenced
      occurrence.
- [ ] Given a `SKILL.md` whose body invokes a plugin script whose shape is not covered
      by any `allowed-tools` prefix, when the check runs, then it fails with a message
      explaining which invocation is uncovered and why.
- [ ] Given a deliberately-seeded violating fixture, when the check runs against it,
      then it reports a non-zero violation count (known-positive), confirming the
      matcher actually detects the failure modes.
- [ ] Given the check is wired into the 0098 lint/test harness, when the standard lint
      or test target is run, then the check executes and a violation fails the build.

## Open Questions

- Does the Claude Code `allowed-tools` matcher allow `*` to span `/` path separators?
  The coverage decision (whether a given rule prefix actually authorises a given
  invocation) depends on this, and it affects the check's false-positive/false-negative
  rate. Confirming the matcher's glob semantics is a prerequisite for a precise
  coverage test. (Carried from the source research's Open Questions.)
- Scope of "plugin script invocation": start with the `artifact-*`/`config-*` family and
  any `${CLAUDE_PLUGIN_ROOT}/scripts/` reference, or cover every `scripts/` invocation a
  skill body makes? Proposed default: all `${CLAUDE_PLUGIN_ROOT}/scripts/` and
  `skills/**/scripts/` references, with the `artifact-*`/`config-*` family as the
  must-cover core.

## Dependencies

- Blocked by: 0106 — the CI gate cannot be enabled (green) until 0106's edits make the
  existing call sites compliant; running this check beforehand fails on known
  pre-existing violations.
- Blocks: none.
- Relates to: 0098 (repo-wide linting/static-analysis harness this check plugs into);
  0106 (the manual fix this guardrail automates and protects from regression).

## Assumptions

- The plugin loader expands `${CLAUDE_PLUGIN_ROOT}` in `allowed-tools` rules before
  matching (established empirically in the source research), so the check can expand the
  variable the same way when computing coverage.
- The set of process wrappers the matcher strips before matching
  (`timeout`/`time`/`nice`/`nohup`/`stdbuf`, not `bash`/`sh`/`env`) is stable enough to
  encode in the check; if Claude Code changes that set, this check is a place to update.
- 0098's lint/test harness exposes an extension point (a task target or test directory)
  this check can attach to rather than requiring a parallel runner.

## Technical Notes

- 0106's AC3 already provides a ready-made first assertion for the bare-fence check:
  `rg -U '```\n(?:(?!```)[\s\S])*scripts/(?:artifact|config)-[\s\S]*?```' skills/`.
- The repo already lints shell via shellcheck/shfmt (0098) and has a Python task layer
  (`tasks/lint`, `tasks/format`) plus shell test helpers — the check could be authored
  in either layer; the deciding factor is where `SKILL.md` frontmatter parsing is
  cleanest.
- Coverage logic is the hard part: parse each `Bash(...)` rule into an expanded prefix,
  normalise each body invocation to the command string the model would emit, and decide
  prefix coverage under the matcher's glob semantics (see Open Questions).

## Drafting Notes

- Framed as the semantic complement to 0098 rather than a sub-task of it: 0098 lints
  script contents per-language; this lints the body-invocation-to-permission
  relationship, a plugin-specific concern. Related, not parented — flag if you'd rather
  make it a child of 0098.
- Set `blocked_by: 0106` because committing this as a failing CI gate is only viable
  once 0106's compliance edits land; if you'd prefer it developed in parallel (and used
  to *drive* 0106's cleanup), downgrade to a plain `relates_to`.
- Kept scope to detection/gating only — auto-fixing non-compliant invocations is
  deliberately excluded.
- Priority medium: it prevents regression of an already-low-severity, now-fixed issue;
  raise to high if the convention is expected to be edited frequently soon.
- The committed guard must **not** reuse 0106's original
  `rg -U '```\n…--pcre2'` regex (it is defective per research §5 — errors without
  `--pcre2`, over-matches via inter-fence false positives) and should build on the
  fence-state `awk` parser approach instead. Additional constraints for the committed
  guard:
  - **Avoid `rg --pcre2`** — PCRE2 is not a guaranteed ripgrep build feature, and a CI
    guard must run on the team's macOS BSD and Linux GNU environments alike; prefer the
    POSIX `awk` parser.
  - **Encode the general invariant**, not just the wrapper case — assert that each
    model-issued invocation's first token is the bare braced path
    `${CLAUDE_PLUGIN_ROOT}/scripts/<name>.sh`, so it also catches assignment-prefix
    (`VAR=$(…)`) and quoted/unbraced escapes, not only `bash`/`sh`/`env` prefixes.
  - **Mind the fence-syntax assumption** — the plan's `awk` guard recognises only plain
    triple-backtick fences (the only form in the current corpus); a committed guard
    should match the opening-fence backtick run length or document that 4+-backtick and
    tilde (`~~~`) fences are out of scope.

## References

- Source: `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`
- Related: 0106, 0098
