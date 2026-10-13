---
type: "work-item"
id: "0331"
title: "Exact-Match Skill Grants That Never Cover Their Commands"
date: "2026-10-12T02:16:07+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
relates_to: ["work-item:0330", "work-item:0107", "work-item:0167"]
tags: ["permissions", "skills", "lint"]
last_updated: "2026-10-12T02:16:07+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0331: Exact-Match Skill Grants That Never Cover Their Commands

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Two ADR skills declare `allowed-tools` rules with no wildcard, but run the
command with arguments. Claude Code matches such a rule exactly, so the
command prompts even in the turn that invokes the skill. The permission lint
did not catch them, because it treats every rule as a prefix and checks only
`!` preprocessor commands.

## Context

`extract-adrs` grants `Bash(accelerator corpus adr next-number)` and runs
`accelerator corpus adr next-number --count N`. `review-adr` grants
`Bash(accelerator corpus adr read-status)` and runs
`accelerator corpus adr read-status <adr-path>`. Neither rule matches its
command, so both skills prompt for a command they declare.

`covered_by` in `tasks/shared/skill_parsing.py` documents that "a rule not
ending in `*` still matches the command plus trailing arguments". Claude Code
2.1.296 does not behave that way. `skill_permissions.py` applies the matcher
only to `!` preprocessor commands, never to the commands a skill body tells
the model to run.

## Requirements

Reproduction, on Claude Code 2.1.296, headless, with a minimal plugin skill:

1. Grant `Bash(python3 ./probe.py)` and run `python3 ./probe.py`,
   `python3 ./probe.py first` and `python3 ./probe.py --count 2`.
2. Grant `Bash(python3 ./probe.py *)` and run `python3 ./probe.py` and
   `python3 ./probe.py first`.

Expected: every command is pre-approved, matching the lint's model.

Actual:

- Rule without a wildcard: the bare command runs; both commands with
  arguments are denied with "This command requires approval".
- Rule with ` *`: both the bare command and the command with an argument run.

Fix requirements:

- `extract-adrs` and `review-adr` carry rules that cover the commands they
  run.
- The lint's matcher models exact matching for rules without a wildcard, and
  ` *` matching the bare command as well as the command with arguments.
- The lint checks the `accelerator` commands in fenced blocks of a skill body
  against the skill's rules, not only `!` preprocessor commands.

## Acceptance Criteria

- [ ] Given `extract-adrs` invoked in a session with no settings allow rules,
  when it runs `accelerator corpus adr next-number --count 2` in the invoking
  turn, then no permission prompt appears.
- [ ] Given `review-adr` invoked the same way, when it runs
  `accelerator corpus adr read-status <path>` in the invoking turn, then no
  permission prompt appears.
- [ ] Given a skill whose rule `Bash(accelerator foo bar)` lacks a wildcard
  and whose fenced body runs `accelerator foo bar --x`, when
  `mise run check` runs, then the skill-permission lint fails and names the
  skill and command.
- [ ] Given a skill whose rule is `Bash(accelerator foo *)` and whose fenced
  body runs bare `accelerator foo`, when `mise run check` runs, then the
  lint passes.
- [ ] Given the current `skills/` tree after the fix, when `mise run check`
  runs, then the lint passes.

## Open Questions

- Did an earlier Claude Code version match rules without a wildcard as a
  prefix? The `covered_by` docstring calls its model "empirically verified",
  so behaviour may have changed between versions. Adding ` *` is correct
  under either model, down to the 2.1.144 floor.

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- Checking fenced-block commands is enough coverage for the body. Commands
  mentioned inline in prose are left out, since the model runs the fenced
  forms.

## Technical Notes

- A scan of every `skills/**/SKILL.md` under exact-match semantics found only
  these two uncovered commands, plus `accelerator migrate`, which the
  observed ` *` behaviour covers.
- Other rules without a wildcard, such as `accelerator corpus metadata
  derive`, are run with no arguments and are covered.
- Widening a rule with ` *` grants every argument form of that subcommand.
  For `next-number` and `read-status`, both read-only, that is acceptable.

## Drafting Notes

- Set priority to medium: each affected skill prompts once per run, and
  0330's cross-query lapse affects far more skills.
- Treated the lint's matcher and body-command gap as part of this bug rather
  than a separate item, because the gap is what let the two defects ship.

## References

- Claude Code permission rule syntax:
  https://code.claude.com/docs/en/permissions
- Related: 0330, 0107, 0167
