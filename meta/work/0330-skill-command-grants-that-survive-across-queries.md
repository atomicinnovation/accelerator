---
type: "work-item"
id: "0330"
title: "Skill Command Grants That Survive Across Queries"
date: "2026-10-12T02:11:25+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "high"
relates_to: ["work-item:0169", "work-item:0239", "work-item:0245", "work-item:0303"]
tags: ["permissions", "skills"]
last_updated: "2026-10-12T02:11:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-916"
---
# 0330: Skill Command Grants That Survive Across Queries

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As an accelerator user, I want a skill to run the `accelerator` commands it
declares without prompting me, however many turns or subagent completions it
spans, so that long-running skills such as `research-topic conduct` finish
unattended and interactive skills don't interrupt me for their own commands.

Today a skill's `allowed-tools` grant lasts only for the query that invokes
the skill. A user reply or a background subagent's completion opens a new
query and clears the grant. Every declared command after that prompts, or is
denied outright when headless.

## Context

Claude Code documents the skill grant as single-turn: "The grant clears when
you send your next message … To pre-approve tools for the whole session rather
than a single turn, add allow rules to those permission settings instead."
Background subagent completions are not mentioned, yet they clear the grant
too: each `task_notification` that arrives after the orchestrator has ended
its turn starts a new query.

Every Agent call is asynchronous in current Claude Code, so any skill that
spawns subagents and then runs a declared command is exposed. In the 0303
Phase 5 full-load run of `research-topic conduct`, every
`accelerator research topic outstanding` call after the first batch was denied
with "This command requires approval". The interactive multi-turn skills
(`create-plan`, `stress-test-*`, `conduct-spike`) lose their grant at the
user's first reply whether or not they spawn agents.

A classification of the skills that spawn agents or hold Bash grants found 16
of 20 affected, most often at the final `accelerator corpus metadata derive` or
`accelerator corpus frontmatter validate`. `stress-test-work-item` is affected
only on its optional spawn path. `list-work-items`, `configure` and the
non-`conduct` verbs of `research-topic` are not affected.

Plugins cannot ship `permissions.allow`: a plugin's `settings` honours only
`agent` and `subagentStatusLine`. 0169 deliberately limited the plugin's
`PreToolUse` hooks to deny or warn, rejecting `"allow"` as a privilege
widening.

## Requirements

- An `accelerator` command a skill declares in `allowed-tools` runs without a
  permission prompt after a user reply.
- It also runs without a prompt after a background subagent's completion
  opens a new query.
- Headless `claude -p` runs of `research-topic conduct` complete with no
  "This command requires approval" denials.
- The user's own `deny` and `ask` permission rules still take precedence over
  anything the plugin does.
- Anything the plugin pre-approves is limited to commands some skill already
  declares; it does not grant broader Bash access.

Out of scope:

- The `extract-adrs` and `review-adr` grants that lack a wildcard and may
  never match their commands, which get a separate work item.
- Subagents not inheriting the parent's grants (upstream #14714, #37730).
- The visualiser's dynamic-args prompt (0239).

## Acceptance Criteria

- [ ] Given a `conduct` run whose plan needs at least 2 batches, when it runs
  headless, then every `accelerator research topic outstanding`, `end-run`
  and `accelerator corpus frontmatter validate` call after the first batch
  runs without a denial.
- [ ] Given an interactive `create-plan` session, when the user replies once
  and the skill then runs `accelerator corpus metadata derive`, then no
  permission prompt appears.
- [ ] Given a user settings rule `ask: Bash(accelerator corpus *)`, when a
  skill runs `accelerator corpus metadata derive`, then the user is still
  prompted.
- [ ] Given `conduct`, `create-plan` and `review-pr`, when each runs a
  declared command after spawning subagents or after a user reply, then none
  prompts for that command.

## Open Questions

- Does pre-approval through a hook return to the plugin's permission surface
  after 0169 rejected `"allow"`? That needs an ADR before implementation.
- Is setup through `configure` that writes allow rules into project or user
  settings acceptable, given that plugins cannot ship `permissions.allow`?

## Dependencies

- Blocked by: none
- Blocks: unattended use of `research-topic conduct`

## Assumptions

- Users should not have to add allow rules by hand for this to count as done.
  If documented allow rules alone are acceptable, the scope shrinks to
  documentation and a `configure` helper.

## Technical Notes

- Root cause, in Claude Code 2.1.296: `alwaysAllowRules.command` is reset at
  the start of every query (`M0n`, `skipSkillPermissionReset`), which takes
  the new query's own slash-command allowed tools, or none. Only poll-event
  queries skip the reset, in both the headless and REPL paths.
- A minimal reproduction plugin granting `Bash(python3 ./probe.py *)` showed:
  - passing when both commands run in the invoking turn, or when a subagent
    finishes inside that turn;
  - denied after the turn ends and a `task_notification` arrives, and after
    `--resume` with a user reply.
- Candidate approaches:
  - Allow-rule setup, the docs' own remedy: `configure` offers to write
    `Bash(accelerator research topic *)`, `Bash(accelerator corpus *)` and
    `Bash(accelerator config *)` into settings. It needs an opt-in from each
    user.
  - A `PreToolUse` hook returning `permissionDecision: "allow"` for declared
    `accelerator` subcommands. The docs say settings `deny` and `ask` rules
    are "still evaluated regardless of what the hook returns", which needs
    confirming by a run. It reverses 0169's stance, so it needs an ADR.
  - Re-invoking the skill each turn re-applies the grant but reloads the whole
    skill prompt every turn. Rejected on cost.
  - An upstream report that a task notification clears the grant. No existing
    issue covers it; #99353 and #14956 are adjacent. A fix there would not
    cover the user-reply case.

## Drafting Notes

- Treated the headless and unattended user as the primary persona, because
  there the failure is a hard denial rather than a prompt.
- The 16-of-20 figure comes from reading each skill's code path, not from
  running each one.
- The interactive-session behaviour rests on the REPL code path and the
  docs; no interactive session was run.
- Kept the solution choice open as the user asked; the approaches live in
  Technical Notes.

## References

- Claude Code skills docs, pre-approving tools:
  https://code.claude.com/docs/en/skills#pre-approve-tools-for-a-skill
- Claude Code permissions docs, extending permissions with hooks:
  https://code.claude.com/docs/en/permissions#extend-permissions-with-hooks
- Plugin manifest reference, `settings`:
  https://code.claude.com/docs/en/plugins-reference
- Upstream: anthropics/claude-code #99353, #14956, #60515
- Related: 0169, 0239, 0245, 0303
- Plan: `meta/plans/2026-10-10-0303-research-concurrency-subagent-limit.md`
