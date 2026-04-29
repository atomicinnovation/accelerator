# Top-level `.accelerator/` reorganisation

## Problem

Accelerator-specific files are currently scattered across the repository:

- `.claude/accelerator.md` and `.claude/accelerator.local.md` — config
- `meta/.migrations-applied`, `meta/.migrations-skipped` — migration state
- `meta/tmp/` — ephemeral working data
- `meta/integrations/<tool>/` — incoming with the Jira integration
  (research dated 2026-04-29)

Everything else under `meta/` (decisions, plans, research, prs, reviews,
specs, validations, work, notes, templates) is **content** that belongs to
the codebase and would persist even if Accelerator were uninstalled. The
items above are **Accelerator's own state** and don't belong intermixed
with the codebase content.

## Proposed structure

Move everything Accelerator-owned into a single top-level `.accelerator/`
tree:

```
.accelerator/
  config.md           # was .claude/accelerator.md
  config.local.md     # was .claude/accelerator.local.md
  state/
    migrations-applied
    migrations-skipped
    integrations/
      jira/           # was meta/integrations/jira/
      linear/
      …
  tmp/                # was meta/tmp/
```

Benefits:

- All Accelerator-owned files in one place, easy to gitignore selectively
  (e.g. `tmp/`) or share (e.g. `state/`).
- `meta/` becomes purely human/agent-authored content, agent-agnostic in
  shape — useful if Accelerator ever supports non-Claude agents.
- Single root makes the "remove Accelerator from this codebase" path
  trivial: delete `.accelerator/` plus the plugin registration.
- Aligns config and state under one prefix, so one mental model covers
  both.

## Why not now

This change is structural and cross-cutting:

- Touches every `config-*` script that reads `.claude/accelerator*.md`.
- Touches every SKILL.md preamble line that calls those scripts.
- Touches the migration framework (`scripts/run-migrations.sh` and the
  state file paths it manages).
- Needs its own migration to relocate user files in existing repos.
- Touches documentation (README, configure skill, init skill, the migrate
  skill).

It is too large to bundle with the Jira integration work. Doing it
*before* multiple integrations land is preferable, though, so the
`meta/integrations/` location doesn't get baked deeply into skill prose.

## Mitigation in the meantime

The Jira integration research (2026-04-29) places state at
`meta/integrations/jira/` under the existing convention. When the reorg
happens, the migration moves it to `.accelerator/state/integrations/jira/`
and updates references. Skills should reach the path through a
`config-read-path.sh integrations` helper rather than hard-coding
`meta/integrations/`, so the future move is a single point of change.

(Adding a path-config key for `integrations` is a pre-requisite worth
folding into Phase 1 of the Jira work — it's a one-line addition and
removes ~5 hard-coded paths from later skill prose.)

## Scope of the eventual change

A standalone research + plan should cover:

- Path-config additions: `integrations`, plus updating `tmp` to point
  inside `.accelerator/`.
- `config-*` script updates to read from `.accelerator/config*.md`.
- Migration `0003-relocate-accelerator-state.sh` that moves files in an
  existing repo.
- Backwards compatibility window: read from old paths if new paths don't
  exist, log a one-time deprecation warning, with a removal target one
  minor version later.
- Plugin extraction implications (the 2026-03-14 plugin-extraction work
  may interact with this).
