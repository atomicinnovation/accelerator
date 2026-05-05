---
title: "VCS abstraction layer"
type: adr-creation-task
status: ready
---

# ADR Ticket: VCS abstraction layer

## Summary

In the context of adding jujutsu support alongside git, we decided for a
layered approach: splitting skills into `vcs/` (backend-agnostic) and `github/`
(platform), a SessionStart hook detecting VCS via directory presence and
injecting persistent context, single VCS-agnostic skills deferring to session
context, a PreToolUse guard hook (block in pure jj, warn in colocated), and
plugin hook registration via `hooks/hooks.json`, to achieve zero-modification
VCS adaptation across all skills, accepting restart on VCS change,
best-effort heuristics, and a jq dependency.

## Context and Forces

- The `skills/git/` directory bundled VCS operations (commit, diff, log) with
  GitHub platform operations (PR creation, review) — distinct concerns
- Jujutsu (jj) is a modern VCS with different concepts: no staging area, working
  copy is always a commit, different command names
- Skills hardcoded git commands in process steps and backtick expressions
- Five possible hook injection approaches existed: SessionStart, PreToolUse,
  skill-level conditionals, UserPromptSubmit, CLAUDE.md
- Ad-hoc git commands in conversation (not just skills) also need to adapt
- The plugin previously had no hook infrastructure

## Decision Drivers

- Zero-modification VCS adaptation: existing skills should work with jj without
  per-skill changes
- Clean concern separation: VCS operations vs platform operations
- Safety: prevent accidental git commands in jj repositories
- Single detection point: detect once, apply everywhere
- Portable hook registration via the plugin system

## Considered Options

For skill organisation:
1. **Keep bundled** — git/ contains both VCS and GitHub. Messy as VCS backends
   multiply.
2. **Split into vcs/ and github/** — Backend-agnostic VCS skills and
   platform-specific GitHub skills.

For VCS detection:
1. **Per-skill conditionals** — Each skill checks the VCS backend. Duplicative.
2. **CLAUDE.md instructions** — Static instructions for the session. No
   auto-detection.
3. **SessionStart hook** — Detect VCS via `.jj/` and `.git/` directory presence;
   inject persistent `additionalContext` with command reference. Single
   detection, applies to everything.

For safety:
1. **No enforcement** — Rely on SessionStart context only. Risk of mistakes.
2. **PreToolUse guard hook** — Block git VCS commands in pure jj; warn in
   colocated. Allow git-specific commands with no jj equivalent (push, fetch)
   and all `gh` commands unconditionally.

## Decision

We will split skills into `vcs/` (backend-agnostic) and `github/` (platform).
A SessionStart hook detects VCS via directory presence (`.jj/` + `.git/` =
colocated; `.jj/` only = pure jj; `.git/` only = git) and injects persistent
context with a VCS command reference. Skills are written in VCS-agnostic
language, deferring to session context for specific commands. A PreToolUse guard
hook blocks git VCS commands in pure jj repos and warns in colocated repos.
Hooks are registered via `hooks/hooks.json` using `${CLAUDE_PLUGIN_ROOT}`.

## Consequences

### Positive
- Existing and future skills work with jj without modification
- Single detection point applies to all skills and ad-hoc interaction
- Guard hook prevents accidental git usage in jj repositories
- Clean separation of VCS and platform concerns
- Hook infrastructure is reusable for future plugin hooks

### Negative
- Session restart required if the repository's VCS setup changes
- Command-splitting heuristic in the guard hook is best-effort
- jq dependency for JSON construction in hook scripts
- More sophisticated prompt engineering needed for VCS-agnostic skills
  (conceptual differences like staging area vs no staging area)

### Neutral
- The detection heuristic (directory presence) matches all community
  integrations and is zero-configuration
- The guard hook is a safety net complementing the primary SessionStart context

## Source References

- `meta/research/2026-03-16-jujutsu-integration-and-vcs-autodetection.md` —
  VCS detection research, hook approach analysis, skill adaptation strategy
- `meta/plans/2026-03-18-vcs-skill-improvements.md` — Implementation plan
  including skill split, hooks, and guard logic
